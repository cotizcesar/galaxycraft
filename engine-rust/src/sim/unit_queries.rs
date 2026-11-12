//! `impl World`: unit queries (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

impl World {
    /// `0x005b4fa0`: display name from the `UNIS`/`UNIx` string id, with
    /// `stat_txt.tbl` as the fallback.
    pub fn unit_name(&self, type_id: u16) -> &str {
        if let Some(settings) = &self.rules.unit_settings
            && settings.use_default.get(type_id as usize).copied() == Some(0)
            && let Some(&id) = settings.string_id.get(type_id as usize)
            && id != 0
        {
            return self.data.strings.text(id);
        }
        self.data.unit_name(type_id)
    }

    /// `0x005b3a50`: nonzero when the type carries `units.dat` flag 0x80.
    pub fn unit_regenerates(&self, type_id: u16) -> bool {
        self.data.unit(type_id).regenerates()
    }

    /// `0x005b57b0`: nonzero when the type carries `units.dat` flag 0x2.
    pub fn unit_is_addon(&self, type_id: u16) -> bool {
        self.data.unit(type_id).is_addon()
    }

    /// `0x005b5950`: nonzero when the type carries `units.dat` flag 0x200.
    pub fn unit_can_cloak(&self, type_id: u16) -> bool {
        self.data.unit(type_id).can_cloak()
    }

    /// `0x005b61d0`: nonzero when the type carries `units.dat` flag 0x2000.
    pub fn unit_is_resource_container(&self, type_id: u16) -> bool {
        self.data.unit(type_id).is_resource_container()
    }

    /// `0x005b53a0`: `CUnit+0xd4` resource amount for types with the
    /// `units.dat` powerup/resource flag 0x800; all other types report 0.
    pub fn unit_resource_amount(&self, i: usize) -> i32 {
        let u = &self.units[i];
        if self.data.unit(u.type_id).is_powerup_flag() {
            u.resource_amount
        } else {
            0
        }
    }

    /// `0x005c1880`: true when the terrain at a placement point accepts a
    /// building (retail checks the tile's build/walk flag word).
    pub fn terrain_accepts_placement(&self, x: i32, y: i32) -> bool {
        self.terrain.buildable_px(x, y)
    }

    /// `0x005c1d60`: retail's placement pre-check.  The point must satisfy
    /// the type's `units.dat` box against the map bounds and the terrain
    /// check above; the pathfinder occupancy test (`0x00761210`) follows in
    /// [`Self::placement_spot`].
    pub fn placement_corner_valid(&self, type_id: u16, x: i32, y: i32) -> bool {
        let d = self.data.unit(type_id).dimensions;
        let width = self.terrain.width as i32 * 32;
        let height = self.terrain.height as i32 * 32;
        d.left as i32 <= x
            && d.right as i32 + x < width
            && d.top as i32 <= y
            && d.bottom as i32 + y < height
            && self.terrain_accepts_placement(x, y)
    }

    /// `0x005b6ac0`: `units.dat` placement X (pixels).
    pub fn unit_placement_x(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).placement.x as i32
    }

    /// `0x005b6ab0`: `units.dat` placement Y (pixels).
    pub fn unit_placement_y(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).placement.y as i32
    }

    /// `0x005a55e0`: living unit ids within one 32 px search half-extent
    /// of a world-pixel point.
    pub fn units_near_point(&self, point: (i32, i32)) -> Vec<u32> {
        self.units_in_rect(point, 32)
    }

    /// `0x005c9620`: living unit ids whose centres lie inside the
    /// axis-aligned box `centre ± half` (world pixels).
    pub fn units_in_rect(&self, centre: (i32, i32), half: i32) -> Vec<u32> {
        self.units
            .iter()
            .filter(|u| {
                u.alive
                    && (u.x / fp(1) - centre.0).abs() <= half
                    && (u.y / fp(1) - centre.1).abs() <= half
            })
            .map(|u| u.id)
            .collect()
    }

    /// `0x005b0cf0`: 1-based CUnit slot number (retail derives it from the
    /// pointer distance to the unit array base).
    pub fn unit_slot_number(&self, i: usize) -> u32 {
        i as u32 + 1
    }

    /// `0x005b4be0`: worker-only `CUnit+0xc0` linked target.
    pub fn unit_worker_target(&self, i: usize) -> Option<u32> {
        let u = &self.units[i];
        if self.data.unit(u.type_id).is_worker() {
            u.linked_target()
        } else {
            None
        }
    }

    /// `0x005b55d0`: the worker's linked target is the Flag unit row 0xd7.
    pub fn unit_worker_target_is_flag(&self, i: usize) -> bool {
        self.unit_worker_target(i)
            .and_then(|id| self.index_of(id))
            .is_some_and(|j| self.units[j].type_id == 0xd7)
    }

    /// `0x005b4b00`: `units.dat` destroy score used by the score screen.
    pub fn unit_destroy_score(&self, type_id: u16) -> u16 {
        self.data.unit(type_id).destroy_score
    }

    /// `0x005b4b20`: `units.dat` maximum shield points.
    pub fn unit_shield_points(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).shield_points as i32
    }

    /// `0x005b5250`: score-screen race class from `group_flags`
    /// (Zerg = 0, Terran = 1, Protoss = 2, other = 4).
    pub fn unit_race_class(&self, type_id: u16) -> u8 {
        let group = self.data.unit(type_id).group_flags;
        if group & 1 != 0 {
            0
        } else if group & 4 != 0 {
            2
        } else if group & 2 != 0 {
            1
        } else {
            4
        }
    }

    /// `0x005b59f0`: true while current HP is below the type's raw maximum.
    pub fn unit_is_damaged(&self, i: usize) -> bool {
        let u = &self.units[i];
        (u.hp as u32) < self.data.unit(u.type_id).hit_points_raw
    }

    /// `0x005a3df0`: true when the id is below 0xe4 and `units.dat` flag
    /// 0x20000000 is clear.
    pub fn unit_flag_29_clear(&self, type_id: u16) -> bool {
        type_id < 0xe4 && self.data.unit(type_id).flags & 0x2000_0000 == 0
    }

    /// `0x005b6b20`: energy flag 0x200000 on a live unit (retail also
    /// requires `CUnit+0xdc` bit 30, the dying flag, to be clear).
    pub fn unit_has_energy(&self, i: usize) -> bool {
        let u = &self.units[i];
        u.alive && self.data.unit(u.type_id).has_energy()
    }

    /// `0x005b68a0`: nonzero when `units.dat` marks the type as shielded.
    pub fn unit_has_shields(&self, type_id: u16) -> bool {
        self.data.unit(type_id).has_shields
    }

    /// `0x005b4aa0`: current HP in whole points plus the shield pool in
    /// whole points for shielded types (`FUN_00588d60` + `CUnit+0x60 >> 8`).
    pub fn unit_current_total_points(&self, i: usize) -> i32 {
        let u = &self.units[i];
        let hp = (u.hp + 0xff) >> 8;
        if self.data.unit(u.type_id).has_shields {
            hp + (u.shields >> 8)
        } else {
            hp
        }
    }

    /// `0x005b6710`: set the shield pool from whole points, clamped to the
    /// type's `units.dat` maximum and stored in 1/256 points.
    pub fn set_unit_shields_points(&mut self, i: usize, points: i32) {
        let max = self.data.unit(self.units[i].type_id).shield_points as i32;
        self.units[i].shields = points.min(max).max(0) << 8;
    }

    /// `0x005b4a50`: one third of the raw maximum-HP table value.
    pub fn unit_max_hp_third(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).hit_points_raw as i32 / 3
    }

    /// `0x005b4b30`: maximum HP in whole points (`raw >> 8`).  Types whose
    /// raw HP is below one point fall back to the current HP rounded up, or
    /// one when that is also zero.
    pub fn unit_max_hp_points(&self, i: usize) -> i32 {
        let u = &self.units[i];
        let points = self.data.unit(u.type_id).hit_points_raw as i32 >> 8;
        if points != 0 {
            points
        } else {
            ((u.hp + 0xff) >> 8).max(1)
        }
    }

    /// `0x005b6560`: whole-point HP plus shields for shielded types.
    pub fn unit_total_hit_points(&self, i: usize) -> i32 {
        let u = &self.units[i];
        let ty = self.data.unit(u.type_id);
        let shields = if ty.has_shields {
            ty.shield_points as i32
        } else {
            0
        };
        self.unit_max_hp_points(i) + shields
    }

    /// `0x005b5820`: `units.dat` building flag 0x1, but only for ids below
    /// 0xe4; higher ids are not buildings.
    pub fn unit_is_building_flagged(&self, type_id: u16) -> bool {
        type_id < 0xe4 && self.data.unit(type_id).is_building()
    }

    /// `0x005b58e0`: inverse of `0x005b5820`, treating every id >= 0xe4 as
    /// non-building.
    pub fn unit_is_not_building_flagged(&self, type_id: u16) -> bool {
        type_id >= 0xe4 || !self.data.unit(type_id).is_building()
    }

    /// `0x005b5ea0`: `units.dat` flying-building flag 0x20.
    pub fn unit_is_flying_building(&self, type_id: u16) -> bool {
        self.data.unit(type_id).is_flying_building()
    }

    /// `0x005b6a80`: units.dat box left extent.    /// `0x005b6a80`: units.dat box left extent.
    pub fn unit_box_left(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).dimensions.left as i32
    }

    /// `0x005b6a90`: units.dat box right extent.
    pub fn unit_box_right(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).dimensions.right as i32
    }

    /// `0x005b6aa0`: units.dat box top extent.
    pub fn unit_box_top(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).dimensions.top as i32
    }

    /// `0x005b6a60`: units.dat box bottom extent.
    pub fn unit_box_bottom(&self, type_id: u16) -> i32 {
        self.data.unit(type_id).dimensions.bottom as i32
    }

    /// `0x005b6ad0`: total box width including the centre column
    /// (`right + 1 + left`).
    pub fn unit_box_width(&self, type_id: u16) -> i32 {
        let d = self.data.unit(type_id).dimensions;
        d.right as i32 + 1 + d.left as i32
    }

    /// `0x005b6a40`: total box height including the centre row
    /// (`top + 1 + bottom`).
    pub fn unit_box_height(&self, type_id: u16) -> i32 {
        let d = self.data.unit(type_id).dimensions;
        d.top as i32 + 1 + d.bottom as i32
    }

    /// `0x007601c0`: pathfinder box extents.  Types below 0x8000 read
    /// `units.dat` dimensions; types with bit 15 set use the low 15 bits as
    /// an all-round radius.  The stored order is
    /// `[top, -right, -bottom, left, top+1, ~right, ~bottom, left+1]`.
    pub fn unit_path_extents(&self, type_id: u16) -> [i32; 8] {
        let (top, right, bottom, left) = if type_id < 0x8000 {
            let d = self.data.unit(type_id).dimensions;
            (d.top as i32, d.right as i32, d.bottom as i32, d.left as i32)
        } else {
            let r = (type_id & 0x7fff) as i32;
            (r, r, r, r)
        };
        [
            top,
            -right,
            -bottom,
            left,
            top + 1,
            !right,
            !bottom,
            left + 1,
        ]
    }

    /// `0x005b6670`: supply provided by a unit type.
    pub fn unit_supply_provided(&self, type_id: u16) -> u8 {
        self.data.unit(type_id).supply_provided
    }

    /// `0x005b4a40`: maximum HP table lookup for a unit type.
    pub fn unit_max_hp_raw(&self, type_id: u16) -> i32 {
        self.unit_stats(type_id).hit_points_raw as i32
    }

    /// Unit stats with the scenario's `UNIS`/`UNIx` overrides applied.
    pub fn unit_stats(&self, type_id: u16) -> UnitType {
        let mut ty = self.data.unit(type_id).clone();
        if let Some(settings) = &self.rules.unit_settings
            && settings.use_default.get(type_id as usize).copied() == Some(0)
            && let Some(&hp) = settings.hit_points.get(type_id as usize)
        {
            ty.hit_points_raw = hp;
            if let Some(&shields) = settings.shields.get(type_id as usize) {
                ty.shield_points = shields;
                ty.has_shields = shields > 0;
            }
            if let Some(&armor) = settings.armor.get(type_id as usize) {
                ty.armor = armor;
            }
            if let Some(&build) = settings.build_time.get(type_id as usize) {
                ty.build_time = build;
            }
            if let Some(&minerals) = settings.mineral_cost.get(type_id as usize) {
                ty.mineral_cost = minerals;
            }
            if let Some(&gas) = settings.gas_cost.get(type_id as usize) {
                ty.gas_cost = gas;
            }
        }
        ty
    }

    /// Technology stats with `TECS`/`TECx` overrides.
    pub fn tech_stats(&self, tech_id: u8) -> Option<TechType> {
        let mut tech = self.data.tech(tech_id)?.clone();
        if let Some(settings) = &self.rules.tech_settings
            && settings.use_default.get(tech_id as usize).copied() == Some(0)
            && let Some(&minerals) = settings.mineral_cost.get(tech_id as usize)
        {
            tech.mineral_cost = minerals;
            if let Some(&gas) = settings.gas_cost.get(tech_id as usize) {
                tech.gas_cost = gas;
            }
            if let Some(&time) = settings.research_time.get(tech_id as usize) {
                tech.research_time = time;
            }
            if let Some(&energy) = settings.energy_cost.get(tech_id as usize) {
                tech.energy_cost = energy;
            }
        }
        Some(tech)
    }

    /// Upgrade stats with `UPGS`/`UPGx` overrides.
    pub fn upgrade_stats(&self, upgrade_id: u8) -> Option<UpgradeType> {
        let mut upgrade = self.data.upgrade(upgrade_id)?.clone();
        if let Some(settings) = &self.rules.upgrade_settings
            && settings.use_default.get(upgrade_id as usize).copied() == Some(0)
            && let Some(&base) = settings.mineral_base.get(upgrade_id as usize)
        {
            upgrade.mineral_cost_base = base;
            if let Some(&v) = settings.mineral_factor.get(upgrade_id as usize) {
                upgrade.mineral_cost_factor = v;
            }
            if let Some(&v) = settings.gas_base.get(upgrade_id as usize) {
                upgrade.gas_cost_base = v;
            }
            if let Some(&v) = settings.gas_factor.get(upgrade_id as usize) {
                upgrade.gas_cost_factor = v;
            }
            if let Some(&v) = settings.time_base.get(upgrade_id as usize) {
                upgrade.time_cost_base = v;
            }
            if let Some(&v) = settings.time_factor.get(upgrade_id as usize) {
                upgrade.time_cost_factor = v;
            }
        }
        Some(upgrade)
    }

    /// `PUNI` unit availability for a player.
    pub fn unit_available(&self, owner: u8, type_id: u16) -> bool {
        self.rules
            .unit_availability
            .as_ref()
            .is_none_or(|a| a.available(owner as usize, type_id))
    }

    /// `PTEC`/`PTEx` technology availability.
    pub fn tech_available(&self, owner: u8, tech_id: u8) -> bool {
        self.rules
            .tech_restrictions
            .as_ref()
            .is_none_or(|r| r.available(owner as usize, tech_id))
    }

    /// `PTEC`/`PTEx` already-researched bit.
    pub fn tech_researched(&self, owner: u8, tech_id: u8) -> bool {
        self.rules
            .tech_restrictions
            .as_ref()
            .is_some_and(|r| r.researched(owner as usize, tech_id))
    }

    /// `UPGR`/`PUPx` maximum level, falling back to `upgrades.dat`.
    pub fn upgrade_max_level(&self, owner: u8, upgrade_id: u8) -> u8 {
        self.rules.upgrade_restrictions.as_ref().map_or_else(
            || {
                self.data
                    .upgrade(upgrade_id)
                    .map(|u| u.max_level)
                    .unwrap_or(0)
            },
            |r| r.max_level(owner as usize, upgrade_id),
        )
    }

    /// `UPGR`/`PUPx` starting level for a player.
    pub fn upgrade_start_level(&self, owner: u8, upgrade_id: u8) -> u8 {
        self.rules
            .upgrade_restrictions
            .as_ref()
            .map_or(0, |r| r.start_level(owner as usize, upgrade_id))
    }

    /// `FUN_005cc6d0` maximum energy in 1/256 points for a unit type and
    /// owner (hero or researched reactor upgrade).
    pub fn max_energy_fixed(&self, type_id: u16, owner: u8) -> i32 {
        self.players
            .get(owner as usize)
            .map_or(MAX_ENERGY * ENERGY_SCALE, |p| {
                unit_max_energy_fixed(&self.data, &p.upgrades, type_id)
            })
    }

    /// Retail sight getter (`0x5a6670`).  Ocular Implants, Antennae,
    /// Sensor Array and Apial Sensors *replace* the unit's sight with 11
    /// tiles; they do not add two.  The Scout's `units.dat` sight is 8, so
    /// the retail switch jumps it straight to 11.
    /// Sight of one unit in tiles: [`Self::sight_range`], or 2 while it is
    /// blinded by Optical Flare.
    pub fn unit_sight_range(&self, i: usize) -> i32 {
        let u = &self.units[i];
        if u.status.optical_flare_ticks > 0 {
            2
        } else {
            self.sight_range(u.owner, u.type_id)
        }
    }

    pub fn sight_range(&self, owner: u8, type_id: u16) -> i32 {
        let base = self.data.unit(type_id).sight_range as i32;
        let upgrade_id = match type_id {
            uid::GHOST => upgrade::OCULAR_IMPLANTS,
            uid::OVERLORD => upgrade::ANTENNAE,
            uid::SCOUT => upgrade::APIAL_SENSORS,
            uid::OBSERVER => upgrade::SENSOR_ARRAY,
            _ => return base,
        };
        if self
            .players
            .get(owner as usize)
            .is_some_and(|p| p.upgrade_level(upgrade_id) > 0)
        {
            11
        } else {
            base
        }
    }

    pub fn index_of(&self, id: u32) -> Option<usize> {
        self.index.get(&id).copied()
    }

    pub fn unit(&self, id: u32) -> Option<&SimUnit> {
        self.index_of(id).map(|i| &self.units[i])
    }

    pub fn unit_mut(&mut self, id: u32) -> Option<&mut SimUnit> {
        self.index_of(id).map(|i| &mut self.units[i])
    }

    /// Minimap dots for one observer: own units always, enemies only when the
    /// rules-level fog can currently see them.  Each unit is tested by its own
    /// index; using one shared index would leak every enemy once any unit was
    /// visible.
    pub fn minimap_units(&self, owner: u8, fog: bool) -> Vec<crate::render::MinimapDot> {
        self.units
            .iter()
            .enumerate()
            .filter(|(i, u)| u.alive && (u.owner == owner || !fog || self.can_see_unit(owner, *i)))
            .map(|(_, u)| {
                let ty = self.data.unit(u.type_id);
                crate::render::MinimapDot {
                    x: u.x / 256,
                    y: u.y / 256,
                    owner: u.owner,
                    placement: (ty.placement.x as i32, ty.placement.y as i32),
                    building: ty.is_building(),
                    resource: is_resource_type(u.type_id),
                    color: crate::render::player_rgb(self, u.owner),
                }
            })
            .collect()
    }

    /// All living unit ids owned by `owner`.
    pub fn unit_ids(&self, owner: u8) -> Vec<u32> {
        self.units
            .iter()
            .filter(|u| u.alive && u.owner == owner)
            .map(|u| u.id)
            .collect()
    }

    /// Find an owned completed parent building with a free add-on slot.
    /// `0x005b5c70`: dynamic air bit (`CUnit+0xdc` bit 2).  True when the
    /// unit is in the air, either natively or as a lifted Terran building.
    pub fn is_air_unit(&self, i: usize) -> bool {
        self.units[i].lifted || self.data.unit(self.units[i].type_id).is_flyer()
    }

    /// Whether units of `a` and `b` fight: enemies ([`World::is_enemy`],
    /// allies never) and neither side a rescuable player, whose units
    /// nobody targets (they wait to be rescued).
    pub fn hostile(&self, a: u8, b: u8) -> bool {
        let rescuable = |p: u8| self.players.get(p as usize).is_some_and(|p| p.rescuable);
        self.is_enemy(a, b) && !rescuable(a) && !rescuable(b)
    }

    /// `0x005b5ba3`: allied lookup in the retail alliance table.  Players
    /// are on the same side when identical or equal `team`, which the lobby
    /// and the "Set Alliance Status" trigger both write.
    pub fn same_side(&self, a: u8, b: u8) -> bool {
        a == b
            || matches!(
                (self.players.get(a as usize), self.players.get(b as usize)),
                (Some(pa), Some(pb)) if pa.team == pb.team
            )
    }

    /// `0x005b5b90`: retail alliance-table lookup.  Two owners are enemies
    /// when neither the same player nor an ally.
    pub fn is_enemy(&self, a: u8, b: u8) -> bool {
        !self.same_side(a, b)
    }

    /// Whether `p` won: the game has a winner and `p` is on its side.
    pub fn has_won(&self, p: u8) -> bool {
        self.winner.is_some_and(|w| {
            w == p
                || (self.same_side(w, p)
                    && self
                        .players
                        .get(w as usize)
                        .is_some_and(|winner| winner.allied_victory))
        })
    }

    /// Retail hides a worker while it is inside a Refinery / Assimilator /
    /// Extractor (OpenBW `order_HarvestGas` calls `hide_unit` for the 37
    /// frames): it is not drawn, cannot be selected or targeted and takes no
    /// part in collision.
    pub fn inside_gas_building(&self, i: usize) -> bool {
        let Some(u) = self.units.get(i) else {
            return false;
        };
        if !u.alive || u.gather_timer == 0 {
            return false;
        }
        let Order::Gather { resource } = u.order else {
            return false;
        };
        self.index_of(resource)
            .is_some_and(|r| is_gas_building(self.units[r].type_id))
    }

    /// Retail `status_flag_gathering`: set while a worker walks to a
    /// resource, mines it or carries cargo back.  Such a worker collides
    /// only with buildings (OpenBW `unit_can_collide_with`), which is why
    /// mineral lines do not jam.
    pub fn is_gathering(&self, i: usize) -> bool {
        self.units.get(i).is_some_and(|u| {
            u.alive && matches!(u.order, Order::Gather { .. } | Order::ReturnCargo { .. })
        })
    }
}
