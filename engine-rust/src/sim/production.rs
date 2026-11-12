//! `impl World`: production (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

impl World {
    pub fn supply_cap(&self, owner: u8) -> i32 {
        if self
            .players
            .get(owner as usize)
            .is_some_and(|p| p.cheats.no_supply)
        {
            return 400;
        }
        // Supply is stored in half-points, so the retail 200 cap is 400 raw.
        let cap: i32 = self
            .units
            .iter()
            .filter(|u| u.alive && u.owner == owner && u.completed)
            .map(|u| self.data.unit(u.type_id).supply_provided as i32)
            .sum();
        cap.min(400)
    }

    pub fn supply_used(&self, owner: u8) -> i32 {
        let mut used: i32 = self
            .units
            .iter()
            // Units inside a transport/bunker are hidden (`alive == false`)
            // but still consume supply.
            .filter(|u| (u.alive || u.in_transport.is_some()) && u.owner == owner)
            .map(|u| self.data.unit(u.type_id).supply_required as i32)
            .sum();
        // Only the unit in training counts; queued ones take their supply
        // when they start (FACT, Backwater Station recording 83.5-121.5 s:
        // four SCVs queued at once, 200 minerals paid at once, supply 6 -> 7
        // at once and +1 at each start, every 12.5-13.0 s).
        for u in &self.units {
            if u.alive
                && u.owner == owner
                && let Order::Train { unit_type } = u.order
            {
                used += self.data.unit(unit_type).supply_required as i32;
            }
            // A missile waiting in its silo is a unit too (8 supply).
            if u.alive && u.owner == owner && u.type_id == uid::NUCLEAR_SILO && u.nuke_ready {
                used += self.data.unit(uid::NUCLEAR_MISSILE).supply_required as i32;
            }
        }
        used
    }

    pub fn can_afford(&self, owner: u8, type_id: u16) -> bool {
        let p = &self.players[owner as usize];
        let ty = self.data.unit(type_id);
        p.minerals >= ty.mineral_cost as i32 && p.gas >= ty.gas_cost as i32
    }

    pub fn has_requirements(&self, owner: u8, type_id: u16) -> bool {
        let Some(info) = tree::build_info(type_id) else {
            return false;
        };
        let p = &self.players[owner as usize];
        if p.cheats.build_anything {
            return true;
        }
        if let Some(t) = info.required_tech
            && !p.has_tech(t)
        {
            return false;
        }
        let attached_addons: std::collections::HashSet<u32> =
            self.units.iter().filter_map(|u| u.addon).collect();
        info.required_units.iter().all(|&(required, count)| {
            self.units
                .iter()
                .filter(|u| {
                    u.alive
                        && u.owner == owner
                        && u.completed
                        && requirement_equivalent(required, u.type_id)
                        && (addon_parent(u.type_id).is_none() || attached_addons.contains(&u.id))
                })
                .count()
                >= count as usize
        })
    }

    pub fn train(&mut self, building_id: u32, unit_type: u16) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner;
        let building_type = self.units[i].type_id;
        if !self.units[i].completed || self.units[i].lifted {
            return Err(SimError::MissingRequirements);
        }
        if !self.building_powered(i) {
            return Err(SimError::Unpowered);
        }
        let info = tree::build_info(unit_type).ok_or(SimError::MissingRequirements)?;
        if info.producer != building_type {
            return Err(SimError::MissingRequirements);
        }
        if !self.has_requirements(owner, unit_type) {
            return Err(SimError::MissingRequirements);
        }
        if !self.unit_available(owner, unit_type) {
            return Err(SimError::MissingRequirements);
        }
        let ty = self.unit_stats(unit_type);
        {
            let p = &self.players[owner as usize];
            if p.minerals < ty.mineral_cost as i32 {
                return Err(SimError::NotEnoughMinerals);
            }
            if p.gas < ty.gas_cost as i32 {
                return Err(SimError::NotEnoughGas);
            }
        }
        let starts_now = self.units[i].production.is_empty() && self.units[i].order == Order::Idle;
        if starts_now
            && self.supply_used(owner) + ty.supply_required as i32 > self.supply_cap(owner)
        {
            return Err(SimError::NotEnoughSupply);
        }
        if self.units[i].production.len() >= MAX_PRODUCTION_QUEUE {
            return Err(SimError::QueueFull);
        }
        {
            let p = &mut self.players[owner as usize];
            p.pay_minerals(ty.mineral_cost as i32);
            p.pay_gas(ty.gas_cost as i32);
        }
        let order = Order::Train { unit_type };
        self.units[i].production.push_back(order);
        if self.units[i].order == Order::Idle {
            self.start_next_production(i);
        }
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::Train {
                    building: building_id,
                    unit: unit_type,
                },
            );
        }
        Ok(())
    }

    /// Morph a Zerg larva into a unit.  The larva becomes an egg and hatches
    /// after the target's build time.  Supply is checked and the cost is paid
    /// immediately, matching the retail build queue.
    /// Container unit shown while a Zerg morph is in progress.  Retail uses
    /// the generic Egg for a larva, the Lurker Egg for Hydralisk -> Lurker,
    /// and the Cocoon for Mutalisk -> Guardian/Devourer.
    pub(super) fn morph_container(source: u16, target: u16) -> u16 {
        match (source, target) {
            (uid::HYDRALISK, uid::LURKER) => uid::LURKER_EGG,
            (uid::MUTALISK, uid::GUARDIAN | uid::DEVOURER) => uid::COCOON,
            _ => uid::EGG,
        }
    }

    pub fn morph(&mut self, larva_id: u32, unit_type: u16) -> Result<(), SimError> {
        let i = self.index_of(larva_id).ok_or(SimError::BadUnit)?;
        if self.units[i].type_id != uid::LARVA {
            return Err(SimError::MissingRequirements);
        }
        let info = tree::build_info(unit_type).ok_or(SimError::MissingRequirements)?;
        if info.producer != uid::LARVA {
            return Err(SimError::MissingRequirements);
        }
        let owner = self.units[i].owner;
        if !self.has_requirements(owner, unit_type) {
            return Err(SimError::MissingRequirements);
        }
        if !self.unit_available(owner, unit_type) {
            return Err(SimError::MissingRequirements);
        }
        let ty = self.unit_stats(unit_type);
        // Supply first, then the cost: a refused morph used to keep the
        // money, and an Overlord (no supply needed) was refused while supply
        // blocked, so a blocked Zerg could never recover (2026-11-06).
        if ty.supply_required > 0
            && self.supply_used(owner) + ty.supply_required as i32 > self.supply_cap(owner)
        {
            return Err(SimError::NotEnoughSupply);
        }
        {
            let p = &mut self.players[owner as usize];
            if p.minerals < ty.mineral_cost as i32 {
                return Err(SimError::NotEnoughMinerals);
            }
            if p.gas < ty.gas_cost as i32 {
                return Err(SimError::NotEnoughGas);
            }
            p.pay_minerals(ty.mineral_cost as i32);
            p.pay_gas(ty.gas_cost as i32);
        }
        let container = Self::morph_container(uid::LARVA, unit_type);
        self.units[i].type_id = container;
        self.units[i].morph_to = Some(unit_type);
        // Translation batch 13 (`0x005b0b4b`): a morph starts uncompleted.
        self.units[i].a13_clear_completed_flag();
        self.units[i].credited = false;
        self.units[i].build_total = build_ticks(ty.build_time);
        self.units[i].build_remaining = self.units[i].build_total;
        self.units[i].hp = self.data.unit(container).hit_points_raw as i32;
        self.units[i].order = Order::Idle;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::Morph {
                    larva: larva_id,
                    unit: unit_type,
                },
            );
        }
        Ok(())
    }

    /// A player's placement: the building goes exactly where the cursor
    /// puts it or nowhere.  The units.dat placement box (`placement`, px) is
    /// snapped to the 32 px tile grid around `cursor` (retail placement
    /// preview), then checked like [`Self::find_build_spot`] without
    /// searching further.  Gas buildings need the cursor on a free geyser.
    /// FACT (2026-11-06): the client used `find_build_spot` for the player
    /// too, which moved a depot ~96 px away from the click.
    pub fn placement_spot(&mut self, type_id: u16, cursor: (i32, i32)) -> Option<(i32, i32)> {
        if is_gas_building(type_id) {
            let on_geyser = self.units.iter().any(|u| {
                u.alive
                    && u.type_id == uid::VESPENE_GEYSER
                    && (u.x / fp(1) - cursor.0).abs() <= 64
                    && (u.y / fp(1) - cursor.1).abs() <= 32
            });
            return on_geyser
                .then(|| self.find_build_spot(type_id, cursor))
                .flatten()
                .filter(|g| (g.0 - cursor.0).abs() <= 64 && (g.1 - cursor.1).abs() <= 32);
        }
        let ty = self.data.unit(type_id);
        let (pw, ph) = (ty.placement.x.max(32) as i32, ty.placement.y.max(32) as i32);
        let left = ((cursor.0 - pw / 2) as f32 / 32.0).round() as i32 * 32;
        let top = ((cursor.1 - ph / 2) as f32 / 32.0).round() as i32 * 32;
        let centre = (left + pw / 2, top + ph / 2);
        self.build_spot_search(type_id, centre, 1)
    }

    /// Placement preview at `cursor`: (snapped centre, placement box size
    /// in px, valid).  Same snapping as [`Self::placement_spot`].
    pub fn placement_preview(
        &mut self,
        type_id: u16,
        cursor: (i32, i32),
    ) -> ((i32, i32), (i32, i32), bool) {
        let ty = self.data.unit(type_id);
        let (pw, ph) = (ty.placement.x.max(32) as i32, ty.placement.y.max(32) as i32);
        let left = ((cursor.0 - pw / 2) as f32 / 32.0).round() as i32 * 32;
        let top = ((cursor.1 - ph / 2) as f32 / 32.0).round() as i32 * 32;
        let centre = (left + pw / 2, top + ph / 2);
        let spot = self.placement_spot(type_id, cursor);
        (spot.unwrap_or(centre), (pw, ph), spot.is_some())
    }

    /// Per-tile validity of a placement box at `cursor` (the build and Land
    /// previews): (snapped centre, box size px, row-major tiles, true =
    /// free).  A tile is free when its terrain is buildable and no structure
    /// or resource other than `ignore` overlaps it (FACT, Desperate Alliance
    /// recording 64 s: landing a Barracks, only the blocked tiles of the box
    /// turn red).
    pub fn placement_tiles(
        &self,
        type_id: u16,
        cursor: (i32, i32),
        ignore: Option<u32>,
    ) -> ((i32, i32), (i32, i32), Vec<bool>) {
        let ty = self.data.unit(type_id);
        let (pw, ph) = (ty.placement.x.max(32) as i32, ty.placement.y.max(32) as i32);
        let left = ((cursor.0 - pw / 2) as f32 / 32.0).round() as i32 * 32;
        let top = ((cursor.1 - ph / 2) as f32 / 32.0).round() as i32 * 32;
        let blockers: Vec<Rect> = self
            .units
            .iter()
            .filter(|u| u.alive && !u.lifted && Some(u.id) != ignore)
            .filter_map(|u| {
                let ut = self.data.unit(u.type_id);
                if !(ut.is_building() || is_resource_type(u.type_id)) {
                    return None;
                }
                let (ux, uy) = (u.x / fp(1), u.y / fp(1));
                Some(unit_box_rect(ut, (ux, uy)))
            })
            .collect();
        let mut tiles = Vec::new();
        for ty_ in 0..ph / 32 {
            for tx in 0..pw / 32 {
                let (x0, y0) = (left + tx * 32, top + ty_ * 32);
                let free = x0 >= 0
                    && y0 >= 0
                    && self.terrain.footprint_buildable(x0, y0, x0 + 32, y0 + 32)
                    && !blockers
                        .iter()
                        .any(|r| Rect::new(x0 + 1, y0 + 1, x0 + 31, y0 + 31).intersects(r));
                tiles.push(free);
            }
        }
        ((left + pw / 2, top + ph / 2), (pw, ph), tiles)
    }

    /// Find a buildable spot near `near` (pixel coordinates).
    pub fn find_build_spot(&mut self, type_id: u16, near: (i32, i32)) -> Option<(i32, i32)> {
        self.rebuild_creep();
        // Gas buildings are not free-standing: they are placed on a geyser
        // and occupy the geyser tile.
        if is_gas_building(type_id) {
            // The Extractor is the one Zerg structure exempt from creep
            // (it lives off the geyser itself).
            let zerg = is_zerg_building(type_id) && type_id != uid::EXTRACTOR;
            // The free geyser nearest to the requested spot (an under-construction
            // refinery already occupies its geyser).
            let geyser = self
                .units
                .iter()
                .filter(|u| u.alive && u.type_id == uid::VESPENE_GEYSER)
                .filter(|u| {
                    !self.units.iter().any(|b| {
                        b.alive
                            && is_gas_building(b.type_id)
                            && (b.x - u.x).abs() < fp(64)
                            && (b.y - u.y).abs() < fp(64)
                    })
                })
                .map(|u| (u.x / fp(1), u.y / fp(1)))
                .min_by_key(|g| (g.0 - near.0).pow(2) as i64 + (g.1 - near.1).pow(2) as i64);
            if let Some((gx, gy)) = geyser {
                if zerg && !self.has_creep_px(gx, gy) {
                    return None;
                }
                return Some((gx, gy));
            }
            return None;
        }
        self.build_spot_search(type_id, near, 20)
    }

    /// `0x00750b20`: search the candidate rectangle around `near` in 8 px
    /// steps for a free footprint, clamped to the retail map margins.
    /// `rings` is the Rust search radius in 32 px tiles.
    pub(super) fn build_spot_search(
        &mut self,
        type_id: u16,
        near: (i32, i32),
        rings: i32,
    ) -> Option<(i32, i32)> {
        self.rebuild_creep();
        let ty = self.data.unit(type_id).clone();
        let half_w = ty.dimensions.right.max(0).max(ty.dimensions.left.max(0)) as i32;
        let half_h = ty.dimensions.bottom.max(0).max(ty.dimensions.top.max(0)) as i32;
        let need_creep = is_zerg_building(type_id) && type_id != uid::HATCHERY;
        let need_power =
            is_protoss_building(type_id) && !matches!(type_id, uid::NEXUS | uid::PYLON);
        // Cache the Pylon field and the blocking footprint list once instead
        // of rescanning every unit for each candidate position.
        let pylons: Vec<(i32, i32)> = if need_power {
            self.units
                .iter()
                .filter(|u| u.alive && u.completed && is_pylon_type(u.type_id))
                .map(|u| (u.x / fp(1), u.y / fp(1)))
                .collect()
        } else {
            Vec::new()
        };
        let blockers: Vec<Rect> = self
            .units
            .iter()
            // Unfinished structures block too (two depots could overlap).
            .filter(|u| u.alive && !u.lifted)
            .filter_map(|u| {
                let ut = self.data.unit(u.type_id);
                if !(ut.is_building() || is_resource_type(u.type_id)) {
                    return None;
                }
                let ux = u.x / fp(1);
                let uy = u.y / fp(1);
                Some(unit_box_rect(ut, (ux, uy)))
            })
            .collect();
        let step = 16;
        for ring in 0..rings {
            for dy in -ring..=ring {
                for dx in -ring..=ring {
                    if ring > 0 && dx.abs() != ring && dy.abs() != ring {
                        continue;
                    }
                    let x = near.0 + dx * step * 2;
                    let y = near.1 + dy * step * 2;
                    let left = x - half_w;
                    let top = y - half_h;
                    let right = x + half_w;
                    let bottom = y + half_h;
                    if left < 0 || top < 0 {
                        continue;
                    }
                    if !self.terrain.footprint_buildable(left, top, right, bottom) {
                        continue;
                    }
                    let rect = Rect::new(left, top, right, bottom);
                    if blockers.iter().any(|r| rect.intersects(r)) {
                        continue;
                    }
                    if need_creep && !self.has_creep_px(x, y) {
                        continue;
                    }
                    if need_power
                        && !pylons
                            .iter()
                            .any(|&(px, py)| (px - x).abs() <= 6 * 32 && (py - y).abs() <= 6 * 32)
                    {
                        continue;
                    }
                    return Some((x, y));
                }
            }
        }
        None
    }

    /// Build a Terran add-on on the right edge of its parent building.  The
    /// recovered build tree names the parent as the add-on's producer
    /// (Command Center → Comsat/Nuclear Silo, Factory → Machine Shop,
    /// Starport → Control Tower, Science Facility → Covert Ops/Physics Lab).
    /// Recorded mutator (`RecAct`): saves, replays and LAN replay it.
    pub fn build_addon(&mut self, parent_id: u32, addon_type: u16) -> Result<(), SimError> {
        self.build_addon_unrecorded(parent_id, addon_type)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::Addon {
                    parent: parent_id,
                    unit: addon_type,
                },
            );
        }
        Ok(())
    }

    pub(super) fn build_addon_unrecorded(
        &mut self,
        parent_id: u32,
        addon_type: u16,
    ) -> Result<(), SimError> {
        let p = self.index_of(parent_id).ok_or(SimError::BadUnit)?;
        if !self.units[p].alive || !self.units[p].completed || self.units[p].lifted {
            return Err(SimError::MissingRequirements);
        }
        if addon_parent(addon_type) != Some(self.units[p].type_id) {
            return Err(SimError::InvalidTarget);
        }
        if self.units[p].addon.is_some() {
            return Err(SimError::ProducerBusy);
        }
        let addon_owner = self.units[p].owner;
        if !self.has_requirements(addon_owner, addon_type) {
            return Err(SimError::MissingRequirements);
        }
        let owner = self.units[p].owner;
        let ty = self.unit_stats(addon_type);
        if self.players[owner as usize].minerals < ty.mineral_cost as i32 {
            return Err(SimError::NotEnoughMinerals);
        }
        if self.players[owner as usize].gas < ty.gas_cost as i32 {
            return Err(SimError::NotEnoughGas);
        }
        let parent_ty = self.data.unit(self.units[p].type_id).clone();
        let addon_hw = ty.dimensions.right.max(0).max(ty.dimensions.left.max(0)) as i32;
        let parent_hw = parent_ty
            .dimensions
            .right
            .max(0)
            .max(parent_ty.dimensions.left.max(0)) as i32;
        let px = self.units[p].x + (parent_hw + addon_hw).max(8) * fp(1);
        let py = self.units[p].y;
        if !self.terrain.footprint_buildable(
            px / fp(1) - addon_hw,
            py / fp(1) - addon_hw,
            px / fp(1) + addon_hw,
            py / fp(1) + addon_hw,
        ) {
            return Err(SimError::InvalidTarget);
        }
        {
            let pl = &mut self.players[owner as usize];
            pl.pay_minerals(ty.mineral_cost as i32);
            pl.pay_gas(ty.gas_cost as i32);
        }
        let idx = self.spawn_px(owner as usize, addon_type, px / fp(1), py / fp(1))?;
        self.units[idx].a13_clear_completed_flag();
        self.units[idx].credited = false;
        self.units[idx].build_total = build_ticks(ty.build_time);
        self.units[idx].build_remaining = self.units[idx].build_total;
        self.units[idx].hp = (ty.hit_points_raw as i32 / 10).max(1);
        let addon_id = self.units[idx].id;
        // Translation batch 14 (`0x005b66a0`): install the parent link.
        self.units[p].a14_set_addon_link(addon_id);
        self.units[idx].addon_attached = true;
        Ok(())
    }

    /// Morph one Zerg unit into another (Hydralisk → Lurker, Mutalisk →
    /// Guardian/Devourer).  The unit becomes an egg until the morph time
    /// elapses, exactly like the larva morph path.
    pub fn morph_unit(&mut self, unit_id: u32, target_type: u16) -> Result<(), SimError> {
        self.morph_unit_unrecorded(unit_id, target_type)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::MorphUnit {
                    unit: unit_id,
                    unit_type: target_type,
                },
            );
        }
        Ok(())
    }

    pub(super) fn morph_unit_unrecorded(
        &mut self,
        unit_id: u32,
        target_type: u16,
    ) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive || !self.units[i].completed {
            return Err(SimError::MissingRequirements);
        }
        let source_type = self.units[i].type_id;
        let valid = matches!(
            (source_type, target_type),
            (uid::HYDRALISK, uid::LURKER)
                | (uid::MUTALISK, uid::GUARDIAN)
                | (uid::MUTALISK, uid::DEVOURER)
        );
        if !valid {
            return Err(SimError::InvalidTarget);
        }
        let container = Self::morph_container(source_type, target_type);
        let owner = self.units[i].owner;
        if !self.has_requirements(owner, target_type) {
            return Err(SimError::MissingRequirements);
        }
        let ty = self.unit_stats(target_type);
        {
            let p = &mut self.players[owner as usize];
            if p.minerals < ty.mineral_cost as i32 {
                return Err(SimError::NotEnoughMinerals);
            }
            if p.gas < ty.gas_cost as i32 {
                return Err(SimError::NotEnoughGas);
            }
            p.pay_minerals(ty.mineral_cost as i32);
            p.pay_gas(ty.gas_cost as i32);
        }
        self.units[i].type_id = container;
        self.units[i].morph_to = Some(target_type);
        self.units[i].a13_clear_completed_flag();
        self.units[i].credited = false;
        self.units[i].build_total = build_ticks(ty.build_time).max(1);
        self.units[i].build_remaining = self.units[i].build_total;
        self.units[i].hp = self.data.unit(container).hit_points_raw as i32;
        self.units[i].order = Order::Idle;
        Ok(())
    }

    /// Lift a Terran building off the ground.  Add-ons are left behind and
    /// destroyed, as in retail.
    pub fn lift_building(&mut self, building_id: u32) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive || !self.units[i].completed {
            return Err(SimError::MissingRequirements);
        }
        if !can_lift_building(self.units[i].type_id) {
            return Err(SimError::InvalidTarget);
        }
        if self.units[i].lifted {
            return Err(SimError::ProducerBusy);
        }
        if let Some(addon_id) = self.units[i].addon.take()
            && let Some(a) = self.index_of(addon_id)
        {
            // Add-ons stay grounded and become inactive, not destroyed.
            self.units[a].addon_attached = false;
        }
        self.units[i].lifted = true;
        self.units[i].order = Order::Idle;
        self.units[i].path.clear();
        self.units[i].path_goal = None;
        Ok(())
    }

    /// Land a lifted building at `pos` (pixels).  The footprint must be
    /// buildable and free of other structures/units.
    pub fn land_building(&mut self, building_id: u32, pos: (i32, i32)) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive || !self.units[i].lifted {
            return Err(SimError::InvalidTarget);
        }
        let ty = self.data.unit(self.units[i].type_id).clone();
        let hw = ty.dimensions.right.max(0).max(ty.dimensions.left.max(0)) as i32;
        let hh = ty.dimensions.bottom.max(0).max(ty.dimensions.top.max(0)) as i32;
        let left = pos.0 - hw;
        let top = pos.1 - hh;
        if left < 0
            || top < 0
            || !self
                .terrain
                .footprint_buildable(left, top, pos.0 + hw, pos.1 + hh)
        {
            return Err(SimError::InvalidTarget);
        }
        let rect = Rect::new(left, top, pos.0 + hw, pos.1 + hh);
        let blocked = self.units.iter().any(|u| {
            if !u.alive || u.id == building_id || !u.completed || u.lifted {
                return false;
            }
            let ut = self.data.unit(u.type_id);
            if !(ut.is_building() || is_resource_type(u.type_id)) {
                return false;
            }
            let ux = u.x / fp(1);
            let uy = u.y / fp(1);
            rect.intersects(&unit_box_rect(ut, (ux, uy)))
        });
        if blocked {
            return Err(SimError::InvalidTarget);
        }
        self.units[i].x = pos.0 * fp(1);
        self.units[i].y = pos.1 * fp(1);
        self.units[i].lifted = false;
        self.units[i].order = Order::Idle;
        self.units[i].path.clear();
        self.units[i].path_goal = None;
        // Landing on/near a grounded add-on of the right type re-attaches it.
        let parent_type = self.units[i].type_id;
        let (px, py) = (pos.0 * fp(1), pos.1 * fp(1));
        let found = self.units.iter().position(|u| {
            u.alive
                && u.completed
                && !u.addon_attached
                && addon_parent(u.type_id) == Some(parent_type)
                && (u.x - px).abs() <= fp(96)
                && (u.y - py).abs() <= fp(96)
        });
        if let Some(a) = found {
            self.units[a].addon_attached = true;
            let addon_id = self.units[a].id;
            self.units[i].a14_set_addon_link(addon_id);
        }
        Ok(())
    }

    pub fn addon_parent_unit(&self, owner: u8, addon: u16) -> Option<u32> {
        let parent = addon_parent(addon)?;
        self.units
            .iter()
            .find(|u| {
                u.alive
                    && u.completed
                    && u.owner == owner
                    && u.type_id == parent
                    && u.addon.is_none()
            })
            .map(|u| u.id)
    }

    /// Find the owned completed building that can morph into `target`.
    pub fn morph_source_unit(&self, owner: u8, target: u16) -> Option<u32> {
        let source = building_morph_source(target)?;
        self.units
            .iter()
            .find(|u| u.alive && u.completed && u.owner == owner && u.type_id == source)
            .map(|u| u.id)
    }

    /// Morph an existing Zerg building into its upgrade (Hatchery → Lair,
    /// Lair → Hive, Spire → Greater Spire, Creep Colony → Sunken/Spore
    /// Colony).  The building keeps its position and identity and becomes a
    /// construction site for the target type.
    pub fn morph_building(&mut self, building_id: u32, target_type: u16) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive || !self.units[i].completed {
            return Err(SimError::MissingRequirements);
        }
        if building_morph_source(target_type) != Some(self.units[i].type_id) {
            return Err(SimError::InvalidTarget);
        }
        let owner = self.units[i].owner;
        if !self.has_requirements(owner, target_type) {
            return Err(SimError::MissingRequirements);
        }
        let ty = self.unit_stats(target_type);
        {
            let p = &mut self.players[owner as usize];
            if p.minerals < ty.mineral_cost as i32 {
                return Err(SimError::NotEnoughMinerals);
            }
            if p.gas < ty.gas_cost as i32 {
                return Err(SimError::NotEnoughGas);
            }
            p.pay_minerals(ty.mineral_cost as i32);
            p.pay_gas(ty.gas_cost as i32);
        }
        let old_max = self.data.unit(self.units[i].type_id).hit_points_raw.max(1) as i64;
        let old_hp = self.units[i].hp.max(1) as i64;
        let new_max = ty.hit_points_raw.max(1) as i64;
        self.units[i].type_id = target_type;
        self.units[i].a13_clear_completed_flag();
        self.units[i].credited = false;
        self.units[i].build_total = build_ticks(ty.build_time);
        self.units[i].build_remaining = self.units[i].build_total;
        // Preserve the current health fraction across the morph.
        self.units[i].hp = ((new_max * old_hp) / old_max).clamp(1, new_max) as i32;
        self.units[i].order = Order::Idle;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::MorphBuilding {
                    source: building_id,
                    unit: target_type,
                },
            );
        }
        Ok(())
    }

    /// Recorded mutator (`RecAct`): saves, replays and LAN replay it.
    pub fn cancel_production(&mut self, building_id: u32) -> Result<(), SimError> {
        self.cancel_production_unrecorded(building_id)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::Cancel {
                    building: building_id,
                },
            );
        }
        Ok(())
    }

    /// Cancel an unfinished building (the Cancel button of a structure
    /// under construction, `0x74d440`).  Recorded (`RecAct::CancelBuild`).
    /// Retail refunds 75 % of the cost (FACT: `stat_txt` / OpenBW
    /// `cancel_building_unit`), removes the structure without counting a
    /// loss, frees the Terran SCV and gives a Zerg Drone back.
    pub fn cancel_construction(&mut self, building_id: u32) -> Result<(), SimError> {
        self.cancel_construction_unrecorded(building_id)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::CancelBuild {
                    building: building_id,
                },
            );
        }
        Ok(())
    }

    /// Cancel the add-on being built on `building_id` (the Cancel button of
    /// a Command Center / Factory / Starport / Science Facility).  Recorded
    /// (`RecAct::CancelAddon`, save tag 18).
    pub fn cancel_addon(&mut self, building_id: u32) -> Result<(), SimError> {
        self.cancel_addon_unrecorded(building_id)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::CancelAddon {
                    building: building_id,
                },
            );
        }
        Ok(())
    }

    pub(super) fn cancel_addon_unrecorded(&mut self, building_id: u32) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let addon = self.units[i].addon.ok_or(SimError::InvalidTarget)?;
        self.cancel_construction_unrecorded(addon)?;
        self.units[i].addon = None;
        Ok(())
    }

    /// Cancel an in-progress Zerg morph: an egg returns to its larva or the
    /// original unit, and an upgrading building returns to its source type.
    /// Recorded (`RecAct::CancelMorph`, save tag 19).
    pub fn cancel_morph(&mut self, building_id: u32) -> Result<(), SimError> {
        self.cancel_morph_unrecorded(building_id)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::CancelMorph {
                    building: building_id,
                },
            );
        }
        Ok(())
    }

    pub(super) fn cancel_morph_unrecorded(&mut self, unit_id: u32) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive {
            return Err(SimError::BadUnit);
        }
        let target = self.units[i].morph_to.ok_or(SimError::Unsupported)?;
        let source = match target {
            uid::LURKER => uid::HYDRALISK,
            uid::GUARDIAN | uid::DEVOURER => uid::MUTALISK,
            _ => building_morph_source(target).unwrap_or(uid::LARVA),
        };
        let ty = self.data.unit(target).clone();
        let source_ty = self.data.unit(source).clone();
        let owner = self.units[i].owner as usize;
        if owner < self.players.len() {
            self.players[owner].minerals += ty.mineral_cost as i32 * 3 / 4;
            self.players[owner].gas += ty.gas_cost as i32 * 3 / 4;
        }
        self.units[i].type_id = source;
        self.units[i].completed = true;
        self.units[i].credited = true;
        self.units[i].morph_to = None;
        self.units[i].build_remaining = 0;
        self.units[i].build_total = 0;
        self.units[i].hp = source_ty.hit_points_raw as i32;
        self.units[i].shields = if source_ty.has_shields {
            fp(source_ty.shield_points as i32)
        } else {
            0
        };
        self.units[i].order = Order::Idle;
        self.units[i].order_row = 0;
        Ok(())
    }

    pub(super) fn cancel_construction_unrecorded(
        &mut self,
        building_id: u32,
    ) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let u = &self.units[i];
        if !u.alive || u.completed || u.morph_to.is_some() {
            return Err(SimError::Unsupported);
        }
        let type_id = u.type_id;
        let ty = self.data.unit(type_id).clone();
        if !ty.is_building() {
            return Err(SimError::Unsupported);
        }
        let owner = u.owner as usize;
        let (x, y) = (u.x / fp(1), u.y / fp(1));
        let builder = u.builder;
        if owner < self.players.len() {
            self.players[owner].minerals += ty.mineral_cost as i32 * 3 / 4;
            self.players[owner].gas += ty.gas_cost as i32 * 3 / 4;
        }
        self.units[i].alive = false;
        self.units[i].order = Order::Die;
        // `0x005b02df`: cancel clears both per-unit masks before the
        // construction is removed.
        self.units[i].clear_mask_121();
        self.units[i].clear_mask_123();
        if let Some(b) = self.index_of(builder)
            && matches!(self.units[b].order, Order::Construct { building } if building == building_id)
        {
            self.units[b].order = Order::Idle;
            self.units[b].order_row = 0;
        }
        if is_zerg_building(type_id) {
            let _ = self.spawn_px(owner, uid::DRONE, x, y);
        }
        Ok(())
    }

    /// `0x0058dcb9`: clear the active build unit and queue state.  The Rust
    /// path also refunds the current block (train/research/upgrade).
    pub(super) fn cancel_production_unrecorded(
        &mut self,
        building_id: u32,
    ) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner as usize;
        // `0x5d9f80` (research) and `0x5da010` (upgrade) refund the full
        // current-level price.  Train keeps the pre-existing full refund.
        let (minerals, gas, resume_queue) = match self.units[i].order.clone() {
            Order::Train { unit_type } => {
                let ty = self.data.unit(unit_type);
                (ty.mineral_cost as i32, ty.gas_cost as i32, true)
            }
            Order::Research { tech_id } => {
                let t = self.data.tech(tech_id).ok_or(SimError::BadUnit)?;
                (t.mineral_cost as i32, t.gas_cost as i32, false)
            }
            Order::Upgrade { upgrade_id, .. } => {
                let up = self.data.upgrade(upgrade_id).ok_or(SimError::BadUnit)?;
                let level = self.players[owner].upgrade_level(upgrade_id);
                (
                    up.mineral_cost(level) as i32,
                    up.gas_cost(level) as i32,
                    false,
                )
            }
            Order::Idle => return Ok(()),
            _ => return Err(SimError::Unsupported),
        };
        self.units[i].order = Order::Idle;
        self.units[i].build_remaining = 0;
        self.units[i].build_total = 0;
        self.players[owner].minerals += minerals;
        self.players[owner].gas += gas;
        if resume_queue {
            self.start_next_production(i);
        }
        Ok(())
    }

    /// Recorded mutator (`RecAct::Dequeue`): refund and remove the newest
    /// queued production item.  Returns false when the queue is empty; the
    /// item currently being built is untouched.
    pub fn dequeue_production(&mut self, building_id: u32) -> Result<bool, SimError> {
        let removed = self.dequeue_production_unrecorded(building_id)?;
        if removed {
            let tick = self.tick;
            if let Some(r) = self.recorder.as_mut() {
                r.record_econ(
                    tick,
                    crate::replay::RecAct::Dequeue {
                        building: building_id,
                    },
                );
            }
        }
        Ok(removed)
    }

    pub(super) fn dequeue_production_unrecorded(
        &mut self,
        building_id: u32,
    ) -> Result<bool, SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner as usize;
        let Some(order) = self.units[i].production.pop_back() else {
            return Ok(false);
        };
        let (minerals, gas) = match order {
            Order::Train { unit_type } => {
                let ty = self.data.unit(unit_type);
                (ty.mineral_cost as i32, ty.gas_cost as i32)
            }
            _ => (0, 0),
        };
        self.players[owner].minerals += minerals;
        self.players[owner].gas += gas;
        Ok(true)
    }

    /// Recorded mutator (`RecAct`): saves, replays and LAN replay it.
    pub fn research(&mut self, building_id: u32, tech_id: u8) -> Result<(), SimError> {
        self.research_unrecorded(building_id, tech_id)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::Research {
                    building: building_id,
                    tech: tech_id,
                },
            );
        }
        Ok(())
    }

    pub(super) fn research_unrecorded(
        &mut self,
        building_id: u32,
        tech_id: u8,
    ) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner;
        if self.players[owner as usize].cheats.free_tech {
            self.players[owner as usize].techs[tech_id as usize] = true;
            return Ok(());
        }
        if !self.units[i].completed || self.units[i].lifted {
            return Err(SimError::MissingRequirements);
        }
        if !self.building_powered(i) {
            return Err(SimError::Unpowered);
        }
        if !tree::tech_buildings(tech_id)
            .iter()
            .any(|&b| requirement_equivalent(b, self.units[i].type_id))
        {
            return Err(SimError::MissingRequirements);
        }
        if !self.tech_available(owner, tech_id) {
            return Err(SimError::MissingTech);
        }
        if self.players[owner as usize].has_tech(tech_id) {
            return Err(SimError::MissingTech);
        }
        let t = self.tech_stats(tech_id).ok_or(SimError::BadUnit)?;
        {
            let p = &mut self.players[owner as usize];
            if p.minerals < t.mineral_cost as i32 {
                return Err(SimError::NotEnoughMinerals);
            }
            if p.gas < t.gas_cost as i32 {
                return Err(SimError::NotEnoughGas);
            }
            p.pay_minerals(t.mineral_cost as i32);
            p.pay_gas(t.gas_cost as i32);
        }
        if self.units[i].order != Order::Idle {
            return Err(SimError::ProducerBusy);
        }
        self.units[i].build_remaining = research_ticks(t.research_time);
        self.units[i].build_total = self.units[i].build_remaining;
        self.units[i].order = Order::Research { tech_id };
        Ok(())
    }

    /// Recorded mutator (`RecAct`): saves, replays and LAN replay it.
    pub fn upgrade(&mut self, building_id: u32, upgrade_id: u8) -> Result<(), SimError> {
        self.upgrade_unrecorded(building_id, upgrade_id)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::Upgrade {
                    building: building_id,
                    upgrade: upgrade_id,
                },
            );
        }
        Ok(())
    }

    /// `0x005da550`: begin an upgrade.  Retail checks the producer, its
    /// requirements and the player's resources, writes the upgrade id to
    /// `CUnit+0xc8` and the duration to `CUnit+0xc6`; the Rust order carries
    /// the same target level.
    pub(super) fn upgrade_unrecorded(
        &mut self,
        building_id: u32,
        upgrade_id: u8,
    ) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner;
        if self.players[owner as usize].cheats.free_upgrades {
            let up = self
                .data
                .upgrade(upgrade_id)
                .ok_or(SimError::BadUnit)?
                .clone();
            let level = &mut self.players[owner as usize].upgrades[upgrade_id as usize];
            if *level < up.max_level {
                *level += 1;
            }
            return Ok(());
        }
        if !self.units[i].completed || self.units[i].lifted {
            return Err(SimError::MissingRequirements);
        }
        if !self.building_powered(i) {
            return Err(SimError::Unpowered);
        }
        if !tree::upgrade_buildings(upgrade_id)
            .iter()
            .any(|&b| requirement_equivalent(b, self.units[i].type_id))
        {
            return Err(SimError::MissingRequirements);
        }
        let up = self.upgrade_stats(upgrade_id).ok_or(SimError::BadUnit)?;
        let level = self.players[owner as usize].upgrade_level(upgrade_id);
        if level >= self.upgrade_max_level(owner, upgrade_id) {
            return Err(SimError::MissingTech);
        }
        let next = level + 1;
        let extra = tree::upgrade_level_requirement(upgrade_id, level);
        for required in extra {
            if !self
                .units
                .iter()
                .any(|u| u.alive && u.owner == owner && u.completed && u.type_id == *required)
            {
                return Err(SimError::MissingRequirements);
            }
        }
        if self.units[i].order != Order::Idle {
            return Err(SimError::ProducerBusy);
        }
        {
            let p = &mut self.players[owner as usize];
            // `0x5da5d0` charges base + factor * current_level; the
            // target level is stored separately in the order.
            let m = up.mineral_cost(level);
            let g = up.gas_cost(level);
            if p.minerals < m as i32 {
                return Err(SimError::NotEnoughMinerals);
            }
            if p.gas < g as i32 {
                return Err(SimError::NotEnoughGas);
            }
            p.pay_minerals(m as i32);
            p.pay_gas(g as i32);
        }
        self.units[i].build_remaining = upgrade_ticks(up.time_cost(level));
        self.units[i].build_total = self.units[i].build_remaining;
        self.units[i].order = Order::Upgrade {
            upgrade_id,
            level: next,
        };
        Ok(())
    }

    pub(super) fn start_next_production(&mut self, i: usize) {
        // A queued unit starts only when its supply is free; otherwise it
        // waits at the front of the queue (retried every frame).
        if let Some(Order::Train { unit_type }) = self.units[i].production.front() {
            let owner = self.units[i].owner;
            let need = self.unit_stats(*unit_type).supply_required as i32;
            if need > 0 && self.supply_used(owner) + need > self.supply_cap(owner) {
                return;
            }
        }
        let next = self.units[i].production.pop_front();
        if let Some(Order::Train { unit_type }) = next {
            let ty = self.unit_stats(unit_type);
            self.units[i].build_remaining = build_ticks(ty.build_time);
            self.units[i].build_total = self.units[i].build_remaining;
            self.units[i].order = Order::Train { unit_type };
        }
    }

    pub(super) fn update_construction(&mut self, i: usize) {
        if self.units[i].completed {
            return;
        }
        // A Terran building that was started by an SCV only advances while
        // an SCV is constructing it (stopped SCV = paused construction), and
        // its hit points grow with the progress like retail.
        if self.units[i].builder != 0 {
            let bid = self.units[i].id;
            let busy = self.index_of(self.units[i].builder).is_some_and(|j| {
                self.units[j].alive
                    && matches!(self.units[j].order, Order::Construct { building } if building == bid)
            });
            if !busy {
                return;
            }
        }
        if self.units[i].build_remaining > 0 {
            if self.units[i].builder != 0 && self.units[i].build_total > 0 {
                let max = self.data.unit(self.units[i].type_id).hit_points_raw as i32;
                let gain = (max - max / 10) / self.units[i].build_total;
                self.units[i].hp = (self.units[i].hp + gain.max(1)).min(max);
            }
            self.units[i].build_remaining -= self.build_step(self.units[i].owner);
            if self.units[i].build_remaining <= 0 {
                if let Some(target) = self.units[i].morph_to.take() {
                    self.units[i].type_id = target;
                }
                // Translation batch 03 (`0x005b0ba9`): mark the structure
                // complete and restore its full raw HP.
                let type_id = self.units[i].type_id;
                self.units[i].a03_finish_construction(&self.data, type_id);
                let ty = self.data.unit(self.units[i].type_id).clone();
                if ty.has_shields {
                    self.units[i].shields = fp(ty.shield_points as i32);
                }
                if is_creep_source(self.units[i].type_id) {
                    self.creep_dirty = true;
                }
                // A completed Refinery/Assimilator/Extractor takes over the
                // geyser underneath it: the gas pool moves to the building
                // and the geyser is hidden.
                if is_gas_building(self.units[i].type_id) {
                    let (bx, by) = (self.units[i].x, self.units[i].y);
                    let geyser = self.units.iter().position(|u| {
                        u.alive
                            && u.type_id == uid::VESPENE_GEYSER
                            && (u.x - bx).abs() < fp(64)
                            && (u.y - by).abs() < fp(64)
                    });
                    if let Some(g) = geyser {
                        let pool = self.units[g].resource_amount;
                        self.units[i].resource_amount = if pool > 0 { pool } else { GAS_CAPACITY };
                        self.units[g].resource_amount = 0;
                        self.units[g].alive = false;
                    }
                }
            }
        }
    }

    pub(super) fn update_production(&mut self, i: usize) {
        if !self.units[i].alive || !self.units[i].completed || self.units[i].lifted {
            return;
        }
        // Production and research pause while a Protoss structure is outside
        // a Pylon field; it resumes when power returns.
        if !self.building_powered(i) {
            return;
        }
        if self.units[i].order == Order::Idle && !self.units[i].production.is_empty() {
            self.start_next_production(i);
        }
        if self.units[i].build_remaining > 0 {
            self.units[i].build_remaining -= self.build_step(self.units[i].owner);
            if self.units[i].build_remaining > 0 {
                return;
            }
        }
        let order = self.units[i].order.clone();
        match order {
            // A Nuclear Silo "trains" its missile: it stays inside, armed.
            Order::Train { unit_type }
                if unit_type == uid::NUCLEAR_MISSILE
                    && self.units[i].type_id == uid::NUCLEAR_SILO =>
            {
                self.units[i].nuke_ready = true;
                self.units[i].order = Order::Idle;
                self.start_next_production(i);
            }
            Order::Train { unit_type } => {
                let owner = self.units[i].owner;
                let spawn_pos = self.exit_position(i);
                let rally = self.units[i].rally;
                if let Ok(new_idx) = self.spawn_px(
                    owner as usize,
                    unit_type,
                    spawn_pos.0 / fp(1),
                    spawn_pos.1 / fp(1),
                ) {
                    // A trained caster starts with a quarter of its maximum
                    // energy: 50, or 62.5 with its reactor upgrade (retail
                    // `finish_building_unit`: `energy = max_energy / 4`).
                    if self.units[new_idx].energy_cap > 0 {
                        let quarter = self.units[new_idx].energy_cap / 4;
                        self.units[new_idx].set_energy_fixed(quarter);
                    }
                    if let Some(rally) = rally {
                        self.units[new_idx].order = Order::Move { pos: rally };
                    }
                }
                self.units[i].order = Order::Idle;
                self.start_next_production(i);
            }
            Order::Research { tech_id } => {
                let owner = self.units[i].owner as usize;
                if let Some(t) = self.data.tech(tech_id) {
                    let _ = t;
                }
                self.players[owner].techs[tech_id as usize] = true;
                self.units[i].order = Order::Idle;
            }
            Order::Upgrade { upgrade_id, level } => {
                let owner = self.units[i].owner as usize;
                self.players[owner].upgrades[upgrade_id as usize] = level;
                self.units[i].order = Order::Idle;
                self.refresh_energy_caps();
            }
            _ => {}
        }
    }

    pub(super) fn update_larva(&mut self) {
        let mut spawns: Vec<(usize, (i32, i32), u32)> = Vec::new();
        for i in 0..self.units.len() {
            let (alive, completed, hatch) = {
                let u = &self.units[i];
                (
                    u.alive,
                    u.completed,
                    matches!(u.type_id, uid::HATCHERY | uid::LAIR | uid::HIVE),
                )
            };
            if !alive || !completed || !hatch {
                continue;
            }
            if self.larva_count_for(i) >= 3 {
                // Retail returns before touching CUnit+0xca while capped.
                continue;
            }
            if self.units[i].larva_timer <= 1 {
                self.units[i].larva_timer = LARVA_SPAWN_TICKS;
                if let Some(pos) = self.larva_spawn_point(i) {
                    spawns.push((self.units[i].owner as usize, pos, self.units[i].id));
                }
            } else {
                self.units[i].larva_timer -= 1;
            }
        }
        for (owner, (x, y), parent) in spawns {
            if let Ok(l) = self.spawn_px(owner, uid::LARVA, x, y) {
                self.units[l].set_connected_unit(Some(parent));
            }
        }
    }

    /// `0x005dfe20`: choose the next larva spot from the bottom/left/right/
    /// top side scores, requiring creep and a walkable/creep neighbour.
    pub(super) fn larva_spawn_point(&mut self, hatch_idx: usize) -> Option<(i32, i32)> {
        let (hx, hy, type_id, values) = {
            let h = &self.units[hatch_idx];
            (
                h.x / fp(1),
                h.y / fp(1),
                h.type_id,
                h.larva_spawn_side_values,
            )
        };
        let d = self.data.unit(type_id).dimensions;
        let candidates = [
            (3usize, (hx, hy + d.bottom as i32 + 10), (0, 22)),
            (0, (hx - d.left as i32 - 10, hy), (-22, 0)),
            (2, (hx + d.right as i32 + 10, hy), (22, 0)),
            (1, (hx, hy - d.top as i32 - 10), (0, -22)),
        ];
        let mut best = None;
        let mut best_score = 101;
        for (side, pos, off) in candidates {
            let score = values[side];
            if score < best_score && self.larva_spawn_clear(pos, (pos.0 + off.0, pos.1 + off.1)) {
                best_score = score;
                best = Some(pos);
            }
        }
        best.or(Some((hx, hy + d.bottom as i32 + 10)))
    }

    pub(super) fn larva_spawn_clear(&mut self, pos: (i32, i32), neighbour: (i32, i32)) -> bool {
        let max_x = self.terrain.width as i32 * 32;
        let max_y = self.terrain.height as i32 * 32;
        if pos.0 < 0 || pos.1 < 0 || pos.0 >= max_x || pos.1 >= max_y {
            return false;
        }
        if neighbour.0 < 0 || neighbour.1 < 0 || neighbour.0 >= max_x || neighbour.1 >= max_y {
            return false;
        }
        self.has_creep_px(pos.0, pos.1)
            && (self.terrain.walkable_px(neighbour.0, neighbour.1)
                || self.has_creep_px(neighbour.0, neighbour.1))
    }

    /// Retail `0x5e0290`: larvae are counted per linked hatchery inside a
    /// +/-256 px square, not per owner.
    pub(super) fn larva_count_for(&self, hatch_idx: usize) -> usize {
        let h = &self.units[hatch_idx];
        let (hx, hy) = (h.x, h.y);
        self.units
            .iter()
            .filter(|l| {
                l.alive
                    && l.type_id == uid::LARVA
                    && l.connected_unit == Some(h.id)
                    && (l.x - hx).abs() <= fp(256)
                    && (l.y - hy).abs() <= fp(256)
            })
            .count()
    }

    pub(super) fn cancel_nuke_for(&mut self, unit_id: u32) {
        self.nuke_strikes
            .retain(|s| s.timer <= -NUKE_WARP_FRAMES || (s.ghost != unit_id && s.silo != unit_id));
    }

    // ------------------------------------------------------------------
    // Carriers, Reavers, Spider Mines and other special units
    // ------------------------------------------------------------------

    /// Add one interceptor/scarab to a carrier/reaver hangar.
    pub fn build_hangar_unit(&mut self, unit_id: u32) -> Result<(), SimError> {
        self.build_hangar_unit_unrecorded(unit_id)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(self.tick, crate::replay::RecAct::Hangar { unit: unit_id });
        }
        Ok(())
    }

    pub(super) fn build_hangar_unit_unrecorded(&mut self, unit_id: u32) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner;
        let type_id = self.units[i].type_id;
        let (child, cost) = match type_id {
            uid::CARRIER => (uid::INTERCEPTOR, 15),
            uid::REAVER => (uid::SCARAB, 15),
            _ => return Err(SimError::Unsupported),
        };
        let max = self.hangar_capacity(i);
        if self.units[i].hangar >= max {
            return Err(SimError::QueueFull);
        }
        if self.players[owner as usize].minerals < cost {
            return Err(SimError::NotEnoughMinerals);
        }
        self.players[owner as usize].pay_minerals(cost);
        self.units[i].hangar += 1;
        let _ = child;
        Ok(())
    }

    pub fn hangar_capacity(&self, i: usize) -> u8 {
        let u = &self.units[i];
        let owner = u.owner as usize;
        match u.type_id {
            uid::CARRIER => {
                if self.players[owner].upgrade_level(upgrade::CARRIER_CAPACITY) > 0 {
                    8
                } else {
                    CARRIER_HANGAR
                }
            }
            uid::REAVER => {
                if self.players[owner].upgrade_level(upgrade::REAVER_CAPACITY) > 0 {
                    10
                } else {
                    REAVER_HANGAR
                }
            }
            _ => 0,
        }
    }
}
