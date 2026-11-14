//! `impl World`: workers (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

/// Gas a worker brings from a depleted geyser (OpenBW `gather_resource`).
const DEPLETED_GAS_PER_TRIP: i32 = 2;

impl World {
    /// Place one Spider Mine from a Vulture.  Rows 18/19/131 use this path;
    /// the Vulture carries three mines (`hangar`).
    pub fn place_spider_mine(&mut self, unit_id: u32, pos: (i32, i32)) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        if !matches!(self.units[i].type_id, uid::VULTURE | 19) {
            return Err(SimError::Unsupported);
        }
        let owner = self.units[i].owner;
        if !self.players[owner as usize].has_tech(tech_id::SPIDER_MINES) {
            return Err(SimError::MissingTech);
        }
        if self.units[i].hangar == 0 {
            return Err(SimError::QueueFull);
        }
        // The Vulture drives to the point and lays the mine there
        // (`order_PlaceMine`), like the Spider Mines button.
        self.issue(
            unit_id,
            Order::Cast {
                spell: Spell::SpiderMines,
                target: SpellTarget::Point(pos.0, pos.1),
            },
        )
    }

    // ------------------------------------------------------------------
    // Economy
    // ------------------------------------------------------------------

    /// Another worker is mining `resource` right now.
    pub(super) fn resource_busy(&self, i: usize, resource: u32) -> bool {
        self.units.iter().enumerate().any(|(j, u)| {
            j != i && u.alive && u.gather_timer > 0 && u.order == Order::Gather { resource }
        })
    }

    /// Nearest mineral field within 8 tiles of the worker that nobody is
    /// mining (OpenBW `find_mineral_field(.., 32 * 8, true)`).
    pub(super) fn free_mineral_near(&self, i: usize, current: usize) -> Option<u32> {
        let (x, y) = (self.units[i].x, self.units[i].y);
        self.units
            .iter()
            .enumerate()
            .filter(|(j, u)| {
                *j != current
                    && u.alive
                    && is_resource_type(u.type_id)
                    && !is_gas_resource(u.type_id)
                    && u.type_id != uid::VESPENE_GEYSER
                    && u.resource_amount > 0
                    && (u.x - x).abs() <= fp(256)
                    && (u.y - y).abs() <= fp(256)
                    && !self.resource_busy(i, u.id)
            })
            .min_by_key(|(j, u)| {
                let (dx, dy) = ((u.x - x) as i64 / 256, (u.y - y) as i64 / 256);
                (dx * dx + dy * dy, *j)
            })
            .map(|(_, u)| u.id)
    }

    /// Nearest live mineral field within 8 tiles of worker `i` other than
    /// `current`, busy or not.
    pub(super) fn nearest_mineral_near(&self, i: usize, current: usize) -> Option<u32> {
        let (x, y) = (self.units[i].x, self.units[i].y);
        self.units
            .iter()
            .enumerate()
            .filter(|(j, u)| {
                *j != current
                    && u.alive
                    && is_resource_type(u.type_id)
                    && !is_gas_resource(u.type_id)
                    && u.type_id != uid::VESPENE_GEYSER
                    && u.resource_amount > 0
                    && (u.x - x).abs() <= fp(256)
                    && (u.y - y).abs() <= fp(256)
            })
            .min_by_key(|(j, u)| {
                let (dx, dy) = ((u.x - x) as i64 / 256, (u.y - y) as i64 / 256);
                (dx * dx + dy * dy, *j)
            })
            .map(|(_, u)| u.id)
    }

    pub(super) fn order_gather(&mut self, i: usize, resource: u32) {
        let Some(ridx) = self.index_of(resource) else {
            self.order_done(i);
            return;
        };
        // A geyser at 0 still yields 2 per trip (OpenBW `gather_resource`);
        // only a mineral field runs out.
        let gas_source = is_gas_resource(self.units[ridx].type_id)
            || is_gas_building(self.units[ridx].type_id)
            || self.units[ridx].type_id == uid::VESPENE_GEYSER;
        let mined_out =
            !self.units[ridx].alive || (self.units[ridx].resource_amount <= 0 && !gas_source);
        if mined_out {
            // Retail `order_MoveToMinerals`: a worker whose field is gone
            // takes the nearest other field within 8 tiles (FACT, Backwater
            // Station 2026-11-06: without it every SCV went idle once the
            // small base fields ran out, freezing income at tick ~3,500).
            let was_mineral = !is_gas_resource(self.units[ridx].type_id)
                && self.units[ridx].type_id != uid::VESPENE_GEYSER
                && !is_gas_building(self.units[ridx].type_id);
            match was_mineral
                .then(|| self.nearest_mineral_near(i, ridx))
                .flatten()
            {
                Some(next) => {
                    self.units[i].order = Order::Gather { resource: next };
                    self.units[i].resource_target = Some(next);
                }
                None => {
                    self.units[i].resource_target = None;
                    self.order_done(i);
                }
            }
            return;
        }
        // Raw geysers require a completed Refinery/Assimilator/Extractor.
        // The worker then gathers from that building, exactly as in the
        // retail game.
        if self.units[ridx].type_id == uid::VESPENE_GEYSER {
            let (gx, gy) = (self.units[ridx].x, self.units[ridx].y);
            let refinery = self.units.iter().position(|u| {
                u.alive
                    && u.completed
                    && is_gas_building(u.type_id)
                    && (u.x - gx).abs() < fp(64)
                    && (u.y - gy).abs() < fp(64)
            });
            if let Some(r) = refinery {
                self.issue(
                    self.units[i].id,
                    Order::Gather {
                        resource: self.units[r].id,
                    },
                )
                .ok();
            } else {
                self.order_done(i);
            }
            return;
        }
        // Mining starts on contact: the units.dat boxes within
        // `HARVEST_CONTACT_PX` (the 80 px centre distance shortened every
        // trip; Wasteland recording: 5 SCVs bring 3.98 minerals/s, ours 4.6).
        // Refineries too: an 80 px centre range left a queued SCV standing
        // 81.6 px away, held off by the footprint, idle until nudged (Boot
        // Camp: deliveries every 35 / 71 frames instead of the recording's
        // three-SCV rhythm).
        // Once mining has started it runs to the end: a worker nudged off
        // contact by another one (it is inside the Refinery, or busy at its
        // field) restarted its 37 / 75 frames and the queue stalled.
        if self.units[i].gather_timer == 0 && self.box_gap_px(i, ridx) > HARVEST_CONTACT_PX {
            let (rx, ry) = (self.units[ridx].x, self.units[ridx].y);
            // Walk on until the boxes touch (a 64 px stop left workers
            // short of contact).
            self.move_towards(i, (rx / fp(1), ry / fp(1)), fp(4));
            return;
        }
        // One worker at a time per patch / refinery, and mining takes time
        // (OpenBW `try_gather_resource`, `order_MiningMinerals`: 75 frames;
        // `order_HarvestGas`: 37).  Without this every worker harvested on
        // arrival and income ran ~2.5x retail (session log 2026-11-04 (86)).
        let gas = is_gas_resource(self.units[ridx].type_id);
        if self.units[i].gather_timer == 0 {
            if self.resource_busy(i, resource) {
                // `order_MoveToMinerals`: try a free field within 8 tiles,
                // else wait for this one.
                if !gas && let Some(free) = self.free_mineral_near(i, ridx) {
                    self.units[i].order = Order::Gather { resource: free };
                }
                return;
            }
            // `0x005abe7f` writes 75 to CUnit+0x54 at MiningMinerals start;
            // this contact-based model uses the calibrated 93-frame dwell.
            self.units[i].gather_timer = if gas {
                GAS_MINING_FRAMES
            } else {
                MINING_FRAMES
            };
            self.units[i].resource_target = Some(resource);
            // The worker faces what it mines (its Fusion Cutter spark appears
            // 10 px ahead of it, weapons.dat row 14).
            let (dx, dy) = (
                self.units[ridx].x - self.units[i].x,
                self.units[ridx].y - self.units[i].y,
            );
            if dx != 0 || dy != 0 {
                self.units[i].facing = direction_byte(dx, dy);
            }
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            return;
        }
        if self.units[i].gather_timer > 1 {
            self.units[i].gather_timer -= 1;
            return;
        }
        self.units[i].gather_timer = 0;
        // Harvest.
        let kind = if gas {
            CarryKind::Gas
        } else {
            CarryKind::Minerals
        };
        let amount = if kind == CarryKind::Gas {
            GAS_PER_TRIP
        } else {
            MINERALS_PER_TRIP
        };
        // OpenBW `gather_resource`: under 8 left, minerals give the rest
        // (and the field dies below); a depleted geyser gives 2 per trip
        // from then on (it gave nothing).
        let left = self.units[ridx].resource_amount;
        let taken = if gas && left < amount {
            DEPLETED_GAS_PER_TRIP
        } else {
            amount.min(left)
        };
        self.units[ridx].resource_amount = (left - amount).max(0);
        // A mineral field mined out disappears (retail removes the sprite and
        // its footprint); geysers / refineries stay and yield less gas.
        if !gas && self.units[ridx].resource_amount <= 0 {
            self.units[ridx].alive = false;
            self.units[ridx].order = Order::Die;
        }
        self.units[i].carry = taken;
        self.units[i].carry_kind = kind;
        self.units[i].resource_target = Some(resource);
        self.units[i].order = Order::ReturnCargo { depot: 0 };
        if gas {
            self.place_at_gas_exit(i, ridx);
        }
    }

    /// Retail `order_HarvestGas`: the worker that was hidden inside the gas
    /// building reappears on the building's edge, on the side of the depot it
    /// will return to, already facing it.
    pub(super) fn place_at_gas_exit(&mut self, i: usize, ridx: usize) {
        let Some(d) = self.nearest_depot(i) else {
            return;
        };
        let (rx, ry) = (self.units[ridx].x / fp(1), self.units[ridx].y / fp(1));
        let (dx, dy) = (
            (self.units[d].x / fp(1) - rx) as i64,
            (self.units[d].y / fp(1) - ry) as i64,
        );
        if dx == 0 && dy == 0 {
            return;
        }
        let rd = self.data.unit(self.units[ridx].type_id).dimensions;
        let wd = self.data.unit(self.units[i].type_id).dimensions;
        // Half extents of the building's box grown by the worker's own.
        let hw = (if dx >= 0 {
            rd.right + wd.left
        } else {
            rd.left + wd.right
        }) as i64
            + 1;
        let hh = (if dy >= 0 {
            rd.bottom + wd.top
        } else {
            rd.top + wd.bottom
        }) as i64
            + 1;
        // Where the line to the depot leaves that box.
        let (ex, ey) = if dx.abs() * hh >= dy.abs() * hw {
            (hw * dx.signum(), dy * hw / dx.abs())
        } else {
            (dx * hh / dy.abs(), hh * dy.signum())
        };
        let (nx, ny) = (rx + ex as i32, ry + ey as i32);
        if self.terrain.walkable_px(nx, ny) {
            self.units[i].x = nx * fp(1);
            self.units[i].y = ny * fp(1);
            self.units[i].path.clear();
            self.units[i].path_goal = None;
        }
        self.units[i].facing = direction_byte(dx as i32, dy as i32);
    }

    pub(super) fn order_return(&mut self, i: usize, depot: u32) {
        let kind = self.units[i].carry_kind;
        if kind == CarryKind::None {
            if let Some(res) = self.units[i].resource_target {
                self.units[i].order = Order::Gather { resource: res };
            } else {
                self.order_done(i);
            }
            return;
        }
        let depot_idx = if depot != 0 {
            self.index_of(depot)
        } else {
            self.nearest_depot(i)
        };
        let Some(didx) = depot_idx else {
            self.order_done(i);
            return;
        };
        // Cargo is handed in on contact with the depot's box (it was a
        // 96 px centre distance; see `HARVEST_CONTACT_PX`).
        if self.box_gap_px(i, didx) > HARVEST_CONTACT_PX {
            let (dx, dy) = (self.units[didx].x, self.units[didx].y);
            self.move_towards(i, (dx / fp(1), dy / fp(1)), fp(4));
            return;
        }
        let owner = self.units[i].owner as usize;
        match kind {
            CarryKind::Minerals => {
                self.players[owner].minerals += self.units[i].carry;
                self.players[owner].minerals_gathered += self.units[i].carry;
            }
            CarryKind::Gas => {
                self.players[owner].gas += self.units[i].carry;
                self.players[owner].gas_gathered += self.units[i].carry;
            }
            CarryKind::None => {}
        }
        self.units[i].carry = 0;
        self.units[i].carry_kind = CarryKind::None;
        if let Some(res) = self.units[i].resource_target {
            self.units[i].order = Order::Gather { resource: res };
        } else {
            self.order_done(i);
        }
    }

    pub(super) fn nearest_depot(&self, i: usize) -> Option<usize> {
        let owner = self.units[i].owner;
        self.units
            .iter()
            .enumerate()
            .filter(|(_, u)| {
                u.alive
                    && u.owner == owner
                    && u.completed
                    && (u.type_id == uid::COMMAND_CENTER
                        || u.type_id == uid::NEXUS
                        || u.type_id == uid::HATCHERY
                        || u.type_id == uid::LAIR
                        || u.type_id == uid::HIVE)
            })
            .min_by_key(|(_, u)| {
                let dx = (u.x - self.units[i].x) as i64;
                let dy = (u.y - self.units[i].y) as i64;
                dx * dx + dy * dy
            })
            .map(|(idx, _)| idx)
    }

    pub(super) fn order_build(&mut self, i: usize, unit_type: u16, pos: (i32, i32)) {
        // Add-ons are parent-built, never worker-built; building morphs keep
        // the source building and never create a second structure.
        if addon_parent(unit_type).is_some() || building_morph_source(unit_type).is_some() {
            self.order_done(i);
            return;
        }
        // Zerg structures (except the Hatchery, which creates creep) must be
        // placed on creep; Protoss structures must be inside a Pylon field.
        if is_zerg_building(unit_type)
            && !matches!(unit_type, uid::HATCHERY | uid::EXTRACTOR)
            && !self.has_creep_px(pos.0, pos.1)
        {
            self.order_done(i);
            return;
        }
        if is_protoss_building(unit_type)
            && !matches!(unit_type, uid::NEXUS | uid::PYLON)
            && !self.has_power_px(self.units[i].owner, pos.0, pos.1)
        {
            self.order_done(i);
            return;
        }
        let ty = self.data.unit(unit_type).clone();
        let pos_fp = (pos.0 * fp(1), pos.1 * fp(1));
        if self.distance_to_point(i, pos_fp) > fp(64) {
            self.move_towards(i, pos, fp(32));
            return;
        }
        let owner = self.units[i].owner as usize;
        if !self.has_requirements(owner as u8, unit_type) {
            self.order_done(i);
            return;
        }
        if !self.can_afford(owner as u8, unit_type) {
            self.order_done(i);
            return;
        }
        {
            let p = &mut self.players[owner];
            p.pay_minerals(ty.mineral_cost as i32);
            p.pay_gas(ty.gas_cost as i32);
        }
        let worker_is_drone = self.units[i].type_id == uid::DRONE;
        let Ok(bidx) = self.spawn_px(owner, unit_type, pos.0, pos.1) else {
            self.order_done(i);
            return;
        };
        self.units[bidx].a13_clear_completed_flag();
        self.units[bidx].credited = false;
        self.units[bidx].build_total = build_ticks(ty.build_time);
        self.units[bidx].build_remaining = self.units[bidx].build_total;
        self.units[bidx].hp = (ty.hit_points_raw as i32 / 10).max(1);
        if worker_is_drone && is_zerg_building(unit_type) {
            // The Drone is consumed by the structure it starts.
            self.kill_unit(i);
        } else if self.units[i].type_id == uid::SCV && !is_protoss_building(unit_type) {
            // Retail Terran construction: the SCV stays on the building
            // (`Order::Construct`) until it completes.
            let bid = self.units[bidx].id;
            self.units[bidx].builder = self.units[i].id;
            self.units[i].order = Order::Construct { building: bid };
            self.units[i].order_row = 33;
        } else {
            self.order_done(i);
        }
    }

    /// SCV on a building it constructs: hold the order until the building
    /// completes or dies, then go idle (resume it by right-clicking the
    /// unfinished building).
    pub(super) fn order_construct(&mut self, i: usize, building: u32) {
        let Some(b) = self.index_of(building) else {
            self.order_done(i);
            return;
        };
        if !self.units[b].alive || self.units[b].completed {
            if self.units[b].builder == self.units[i].id {
                self.units[b].builder = 0;
            }
            self.order_done(i);
            return;
        }
        // The SCV works its way around the structure: a spot on the edge of
        // the building's box, changing every `CONSTRUCT_HOP_TICKS` (Boot Camp
        // recording 175-199 s: the SCV welds the depot from its left, front
        // and right sides, never from its centre).  Derived from the tick and
        // the unit id, so no extra state and no RNG draw.
        let ty = self.data.unit(self.units[b].type_id);
        let (hw, hh) = (
            ty.dimensions.left.max(ty.dimensions.right).max(8) as i64,
            ty.dimensions.top.max(ty.dimensions.bottom).max(8) as i64,
        );
        let hop = self.tick / CONSTRUCT_HOP_TICKS;
        let h = (hop as i64)
            .wrapping_mul(2654435761)
            .wrapping_add(self.units[i].id as i64 * 40503)
            .rem_euclid(1 << 20);
        let (ex, ey) = match h % 3 {
            0 => (-hw, (h / 3).rem_euclid(2 * hh) - hh),
            1 => (hw, (h / 3).rem_euclid(2 * hh) - hh),
            _ => ((h / 3).rem_euclid(2 * hw) - hw, hh),
        };
        let (bx, by) = (self.units[b].x / fp(1), self.units[b].y / fp(1));
        let spot = (bx + ex as i32, by + ey as i32);
        if self.distance(i, b) > fp(96) {
            self.move_towards(i, spot, fp(8));
            return;
        }
        self.units[b].builder = self.units[i].id;
        if self.distance_to_point(i, (spot.0 * fp(1), spot.1 * fp(1))) > fp(6) {
            self.move_towards(i, spot, fp(4));
        } else {
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            // Welding: the SCV faces the structure.
            let (dx, dy) = (
                self.units[b].x - self.units[i].x,
                self.units[b].y - self.units[i].y,
            );
            if dx != 0 || dy != 0 {
                self.units[i].facing = direction_byte(dx, dy);
            }
        }
    }

    /// SCV repair.  Rate is the documented `0.9 * maxHP / build_time` per
    /// second; cost is about one third of the unit's mineral cost over the
    /// full hit-point pool, accumulated in fractional minerals per SCV.
    pub(super) fn order_repair(&mut self, i: usize, target: u32) {
        let Some(t) = self.index_of(target) else {
            self.order_done(i);
            return;
        };
        if !self.units[t].alive {
            self.order_done(i);
            return;
        }
        let ty = self.data.unit(self.units[t].type_id).clone();
        if !ty.is_mechanical() && !ty.is_building() {
            self.order_done(i);
            return;
        }
        let max = ty.hit_points_raw as i32;
        if self.units[t].hp >= max {
            self.order_done(i);
            return;
        }
        if t == i {
            self.order_done(i);
            return;
        }
        let dist = self.distance(i, t);
        if dist > fp(48) {
            let (tx, ty) = (self.units[t].x, self.units[t].y);
            self.move_towards(i, (tx / fp(1), ty / fp(1)), fp(32));
            return;
        }
        let owner = self.units[i].owner as usize;
        // Retail repair rate: 0.9 * maxHP / build_time_seconds per SCV.
        // `build_time` is in 1/15 s and the logical rate is 24 Hz, so the
        // per-tick raw HP is maxHP * 27 / (build_time * 48).
        let build_time = ty.build_time.max(1) as i64;
        let max_raw = max.max(1) as i64;
        let mut heal = (max_raw * 27 / (build_time * 48)).max(1) as i32;
        heal = heal.min(max - self.units[t].hp);
        if heal <= 0 {
            self.order_done(i);
            return;
        }
        // Repair costs about one third of the unit's mineral cost over its
        // whole hit-point pool.  Fractional minerals are kept per SCV.
        let debt = (ty.mineral_cost as i64 * heal as i64 * 256) / (3 * max_raw);
        self.units[i].repair_debt = self.units[i].repair_debt.saturating_add(debt as i32);
        let payable = self.units[i].repair_debt / 256;
        if payable > self.players[owner].minerals {
            self.order_done(i);
            return;
        }
        if payable > 0 {
            self.players[owner].pay_minerals(payable);
            self.units[i].repair_debt %= 256;
        }
        self.units[t].hp += heal;
    }

    // ------------------------------------------------------------------
    // Transports
    // ------------------------------------------------------------------

    /// Gap in px between the units.dat boxes of units `a` and `b` (0 when
    /// they touch or overlap): the larger of the x and y separations.
    pub(super) fn box_gap_px(&self, a: usize, b: usize) -> i32 {
        let bx = |i: usize| {
            let u = &self.units[i];
            let (x, y) = (u.x / fp(1), u.y / fp(1));
            self.data.unit(u.type_id).dimensions.rect_at((x, y))
        };
        let (a0, b0) = (bx(a), bx(b));
        let dx = (b0.0 - a0.2).max(a0.0 - b0.2).max(0);
        let dy = (b0.1 - a0.3).max(a0.1 - b0.3).max(0);
        dx.max(dy)
    }
}
