//! `impl World`: spawn (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

impl World {
    pub fn new(data: Arc<GameData>, terrain: Terrain, player_count: usize) -> Self {
        let regions = RegionMap::from_terrain(&terrain);
        let region_count = regions.count() as usize;
        let creep_tiles = terrain.width * terrain.height;
        let creep = vec![false; creep_tiles];
        let creep_strength = vec![0u8; creep_tiles];
        let creep_supported = vec![false; creep_tiles];
        Self {
            data,
            terrain,
            regions,
            ai_region_states: vec![vec![0; region_count]; player_count],
            players: (0..player_count)
                .map(|i| Player::new(Race::Terran, i as u8, 50, 0))
                .collect(),
            units: Vec::new(),
            next_id: 1,
            tick: 0,
            winner: None,
            sudden_death: false,
            greed_goal: None,
            slaughter_deadline: None,
            cheat_no_enemy_spells: false,
            ctf: None,
            rules: crate::map::ScenarioRules::default(),
            rescues: Vec::new(),
            matrix_cues: Vec::new(),
            spell_cues: std::collections::VecDeque::new(),
            spell_cue_seq: 0,
            pending_hits: Vec::new(),
            nuke_strikes: Vec::new(),
            nuke_warnings: Vec::new(),
            trigger_victory: false,
            triggers: None,
            projectiles: Vec::new(),
            vision: None,
            fog_of_war: false,
            creep,
            creep_strength,
            creep_supported,
            creep_dirty: true,
            next_projectile_id: 1,
            rng: 0x1234_5678_9abc_def0,
            obstacles: Obstacles::new(),
            index: HashMap::new(),
            handler_hits: [0; crate::handlers::HANDLER_COUNT],
            recorder: None,
        }
    }

    pub fn from_scenario(
        data: Arc<GameData>,
        scenario: &Scenario,
        tiles: &crate::map::TileData,
        races: &[Race],
    ) -> Self {
        Self::from_scenario_occupied(data, scenario, tiles, races, None)
    }

    /// [`Self::from_scenario`] for a lobby game: `occupied[p]` false (an
    /// Open or Closed slot) gets no melee starting force and is defeated
    /// from the first tick, as retail leaves empty start locations empty.
    pub fn from_scenario_occupied(
        data: Arc<GameData>,
        scenario: &Scenario,
        tiles: &crate::map::TileData,
        races: &[Race],
        occupied: Option<&[bool]>,
    ) -> Self {
        let terrain = Terrain::from_scenario(scenario, tiles);
        let mut world = Self::new(data, terrain, races.len());
        world.vision = Some(Vision::from_mask(
            scenario.width,
            scenario.height,
            &scenario.mask,
        ));
        world.rules = scenario.rules();
        for (i, race) in races.iter().enumerate() {
            world.players[i].race = *race;
        }
        let rules = world.rules.clone();
        for p in 0..world.players.len() {
            for tech in 0..44u8 {
                if rules
                    .tech_restrictions
                    .as_ref()
                    .is_some_and(|r| r.researched(p, tech))
                {
                    world.players[p].techs[tech as usize] = true;
                }
            }
            for upgrade in 0..61u8 {
                if let Some(restrictions) = &rules.upgrade_restrictions {
                    let level = restrictions.start_level(p, upgrade);
                    if level > 0 {
                        world.players[p].upgrades[upgrade as usize] = level;
                    }
                }
            }
        }
        for (i, p) in world.players.iter_mut().enumerate() {
            p.rescuable = scenario.owners.get(i) == Some(&OWNR_RESCUE_PASSIVE);
            p.neutral = scenario.owners.get(i) == Some(&7);
        }
        world.trigger_victory = scenario.has_placed_force() && !scenario.triggers.is_empty();
        if world.trigger_victory {
            world.apply_ums_alliances(scenario);
        }
        let mut serial_to_index: HashMap<u32, usize> = HashMap::new();
        for mu in &scenario.units {
            // Start-location markers are metadata, not units.
            if mu.unit_id == uid::START_LOCATION {
                continue;
            }
            let resource = is_resource_type(mu.unit_id);
            let owner = mu.owner as usize;
            if !resource && owner >= world.players.len() {
                continue;
            }
            // Neutral resources are placed under player 0 for ownership
            // bookkeeping; every combat/economy path treats them as neutral.
            let owner = if resource { 0 } else { owner };
            let idx = world.spawn(owner, mu.unit_id, mu.x, mu.y);
            if let Ok(i) = idx {
                serial_to_index.insert(mu.serial, i);
                let ty = world.unit_stats(mu.unit_id);
                let u = &mut world.units[i];
                if mu.fields_set & 0x02 != 0 {
                    let pct = (mu.hit_point_percent as i32).min(100);
                    // 0% leaves the retail minimum of 1/256 HP.
                    u.hp = ((ty.hit_points_raw as i64 * pct as i64 / 100) as i32).max(1);
                }
                if mu.fields_set & 0x04 != 0 && ty.has_shields {
                    let pct = (mu.shield_percent as i32).min(100);
                    u.shields = (fp(ty.shield_points as i32) * pct / 100).max(0);
                }
                if mu.fields_set & 0x08 != 0 {
                    let pct = (mu.energy_percent as i32).min(100);
                    u.set_energy_fixed(MAX_ENERGY * ENERGY_SCALE * pct / 100);
                }
                if mu.fields_set & 0x20 != 0 {
                    u.hangar = mu.hangar.min(u.hangar_capacity as u16) as u8;
                }
                if is_resource_type(u.type_id) {
                    let capacity = if u.type_id == uid::VESPENE_GEYSER {
                        GAS_CAPACITY
                    } else {
                        MINERAL_CAPACITY
                    };
                    u.resource_amount = if mu.fields_set & 0x10 != 0 || mu.resource_amount > 0 {
                        (mu.resource_amount as u16) as i32
                    } else {
                        capacity
                    };
                }
                let states = mu.states & mu.states_set;
                if states & 0x01 != 0 {
                    u.status.cloaked = true;
                }
                if states & 0x02 != 0 {
                    u.status.burrowed = true;
                }
                if states & 0x04 != 0 && can_lift_building(u.type_id) {
                    u.lifted = true;
                }
                if states & 0x08 != 0 {
                    u.status.hallucination = true;
                }
                if states & 0x10 != 0 {
                    u.invincible = true;
                }
            }
        }
        // `0x70fc40`: THG2 unit records become doodad CUnits.  Static
        // sprites (`0x1000`) are never simulated.
        for th in &scenario.thingies {
            if th.flags & 0x1000 != 0 || !is_doodad_type(th.unit_id) {
                continue;
            }
            let owner = if is_door_type(th.unit_id) {
                0
            } else if (th.player as usize) < world.players.len() {
                th.player as usize
            } else {
                0
            };
            // `0x00711730`: realize the THG2 record as a unit, then apply
            // the disabled bit from `0x005b56a0`.
            if let Ok(i) = world.spawn(owner, th.unit_id, th.x, th.y) {
                world.units[i].doodad_enabled = Some(th.flags & 0x8000 == 0);
            }
        }

        // Link add-ons and Nydus pairs by the serial numbers StarEdit writes.
        // The link flags at offset 10 are ignored by the retail engine; only
        // the two records naming each other matter.
        for mu in &scenario.units {
            if mu.link_serial == 0 {
                continue;
            }
            let (Some(&i), Some(&j)) = (
                serial_to_index.get(&mu.serial),
                serial_to_index.get(&mu.link_serial),
            ) else {
                continue;
            };
            if i == j || !world.units[i].alive || !world.units[j].alive {
                continue;
            }
            if is_nydus_canal_type(world.units[i].type_id)
                && is_nydus_canal_type(world.units[j].type_id)
            {
                world.units[i].nydus_exit = Some(world.units[j].id);
                world.units[j].nydus_exit = Some(world.units[i].id);
            }
            // Add-on -> parent, or parent -> add-on.
            if let Some(parent_type) = addon_parent(world.units[i].type_id) {
                if world.units[j].type_id == parent_type && world.units[j].addon.is_none() {
                    let addon_id = world.units[i].id;
                    world.units[j].a14_set_addon_link(addon_id);
                    world.units[i].addon_attached = true;
                }
            } else if let Some(child_parent) = addon_parent(world.units[j].type_id)
                && world.units[i].type_id == child_parent
                && world.units[i].addon.is_none()
            {
                let addon_id = world.units[j].id;
                world.units[i].a14_set_addon_link(addon_id);
                world.units[j].addon_attached = true;
            }
        }
        // Legacy/protected maps with no usable serial link: attach an add-on
        // that sits beside its parent.
        let placed_addons: Vec<(usize, u16, (i32, i32))> = world
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| {
                addon_parent(u.type_id).is_some() && u.alive && u.completed && !u.addon_attached
            })
            .map(|(i, u)| (i, u.type_id, (u.x, u.y)))
            .collect();
        for (ai, addon_type, (ax, ay)) in placed_addons {
            let Some(parent_type) = addon_parent(addon_type) else {
                continue;
            };
            let parent = world.units.iter().position(|u| {
                u.alive
                    && u.completed
                    && u.type_id == parent_type
                    && u.addon.is_none()
                    && (u.x - ax).abs() <= fp(96)
                    && (u.y - ay).abs() <= fp(96)
            });
            if let Some(p) = parent {
                world.units[p].addon = Some(world.units[ai].id);
                world.units[ai].addon_attached = true;
            }
        }
        // Melee maps contain only start locations and resources; create the
        // stock starting town hall and four workers for every player.  A
        // scenario that already places a force for any active player is a
        // campaign/use-map map and must not receive free bases.
        let has_placed_force = scenario.units.iter().any(|mu| {
            mu.unit_id != uid::START_LOCATION
                && !is_resource_type(mu.unit_id)
                && (mu.owner as usize) < world.players.len()
        });
        let starts = scenario.start_locations();
        for p in 0..world.players.len() {
            if has_placed_force {
                break;
            }
            if occupied.is_some_and(|o| !o.get(p).copied().unwrap_or(false)) {
                world.players[p].defeated = true;
                continue;
            }
            let race = world.players[p].race;
            let start = scenario
                .start_location_of(p as u8)
                .or_else(|| starts.get(p).copied())
                .or_else(|| starts.first().copied())
                .unwrap_or((16 + p as i32 * 16, 16));
            let (sx, sy) = (start.0, start.1);
            let has_base = world.units.iter().any(|u| {
                u.alive
                    && u.owner == p as u8
                    && u.completed
                    && matches!(u.type_id, uid::COMMAND_CENTER | uid::NEXUS | uid::HATCHERY)
            });
            if has_base {
                continue;
            }
            let (town, worker) = match race {
                Race::Terran => (uid::COMMAND_CENTER, uid::SCV),
                Race::Protoss => (uid::NEXUS, uid::PROBE),
                Race::Zerg => (uid::HATCHERY, uid::DRONE),
                Race::Neutral => (uid::COMMAND_CENTER, uid::SCV),
            };
            let town_idx = world.spawn_px(p, town, sx, sy).ok();
            // Start-unit offsets measured on the retail captures of
            // (8)The Hunters (135051: probes in a row under the Nexus;
            // 134905: drones around the Hatchery, whose larvae take the
            // spots under it, and the Overlord to the lower left).
            // INFERENCE: retail runs its placement search; these are its
            // results for an unobstructed start location.
            let workers: [(i32, i32); 4] = if race == Race::Zerg {
                [(-49, 45), (11, 45), (39, 45), (54, 0)]
            } else {
                [(-48, 53), (-24, 53), (0, 53), (24, 53)]
            };
            for (dx, dy) in workers {
                let _ = world.spawn_px(p, worker, sx + dx, sy + dy);
            }
            // A melee Zerg start also includes one Overlord; it is the
            // starting supply provider (the Hatchery alone provides 1).
            // Retail `create_starting_units` (OpenBW): the Overlord sits the
            // town hall's full extent + 1 px from the start location, toward
            // the map centre on each axis; the Hatchery spawns two larvae at
            // its first larva spot, 10 px below its box.
            if race == Race::Zerg {
                let d = world.data.unit(town).dimensions;
                let add_x = d.left as i32 + d.right as i32 + 1;
                let add_y = d.top as i32 + d.bottom as i32 + 1;
                let (mw, mh) = (
                    world.terrain.width as i32 * 32,
                    world.terrain.height as i32 * 32,
                );
                let ox = if sx >= mw / 2 { sx - add_x } else { sx + add_x };
                let oy = if sy >= mh / 2 { sy - add_y } else { sy + add_y };
                let _ = world.spawn_px(p, uid::OVERLORD, ox, oy);
                let town_id = town_idx.map(|i| world.units[i].id);
                for _ in 0..2 {
                    if let (Some(parent), Ok(l)) = (
                        town_id,
                        world.spawn_px(p, uid::LARVA, sx, sy + d.bottom as i32 + 10),
                    ) {
                        world.units[l].connected_unit = Some(parent);
                    }
                }
            }
            world.players[p].minerals = 50;
        }
        if !has_placed_force {
            world.mature_creep();
        }
        if !scenario.triggers.is_empty() {
            // `MRGN` rectangles are already in pixels (Boot Camp's location
            // 1 is 0..1280 x 0..2048 on a 64x64-tile = 2048 px map); the
            // old x32 made every location cover the whole map.
            let locations: Vec<(i32, i32, i32, i32)> = scenario
                .locations
                .iter()
                .map(|l| (l.left, l.top, l.right, l.bottom))
                .collect();
            world.triggers = Some(crate::trigger::TriggerSystem::parse(
                &scenario.triggers,
                &locations,
            ));
            if let Some(t) = world.triggers.as_mut() {
                t.unit_properties = scenario.unit_properties.clone();
                t.forces = scenario.forces;
                for (p, present) in t.present.iter_mut().enumerate() {
                    *present = scenario.owners[p] != 0;
                }
                t.mission_objectives = vec![None; world.players.len()];
                t.preserved = vec![vec![false; t.triggers.len()]; world.players.len()];
            }
        }
        world
    }

    /// Recompute the cached caps after an upgrade changes (including every
    /// existing unit of that type) and clamp current energy down if needed.
    pub fn refresh_energy_caps(&mut self) {
        for i in 0..self.units.len() {
            let type_id = self.units[i].type_id;
            let owner = self.units[i].owner;
            let cap = self.max_energy_fixed(type_id, owner);
            self.units[i].energy_cap = cap;
            if self.units[i].energy_fixed() > cap {
                self.units[i].set_energy_fixed(cap);
            }
        }
    }

    /// Turn on rules-level fog of war.  A scenario gets a vision grid when it
    /// is loaded; for a synthetic `World::new` the grid is created lazily from
    /// the terrain extent.
    pub fn enable_fog(&mut self) {
        if self.vision.is_none() {
            self.vision = Some(Vision::new(self.terrain.width, self.terrain.height));
        }
        self.fog_of_war = true;
        self.update_vision();
    }

    /// `0x005b0fe0`: retail unit constructor.  Its `0x00585350` sub-init
    /// (owner, order/target zeroing) is mirrored by the `SimUnit`
    /// initializer.  This path clears the same runtime state (target, order
    /// timers, energy, hp) and, because the engine spawns a completed unit,
    /// takes the finished-HP branch.
    pub fn spawn(&mut self, owner: usize, type_id: u16, x: i32, y: i32) -> Result<usize, SimError> {
        if owner >= self.players.len() {
            return Err(SimError::BadOwner);
        }
        let ty = self.unit_stats(type_id);
        let shields = if ty.has_shields {
            fp(ty.shield_points as i32)
        } else {
            0
        };
        let energy_cap = self.max_energy_fixed(type_id, owner as u8);
        let energy = if ty.flags & 0x0020_0000 != 0 {
            energy_cap / ENERGY_SCALE
        } else {
            0
        };
        let id = self.next_id;
        self.next_id += 1;
        let unit = SimUnit {
            id,
            kills: 0,
            builder: 0,
            owner: owner as u8,
            type_id,
            hp: ty.hit_points_raw as i32,
            shields,
            energy,
            energy_frac: 0,
            energy_cap,
            x: x.saturating_mul(fp(1)),
            y: y.saturating_mul(fp(1)),
            facing: 0,
            alive: true,
            completed: true,
            credited: false,
            lifted: false,
            lift_phase: None,
            near_target_index: 0,
            addon_attached: false,
            build_remaining: 0,
            build_total: 0,
            cooldown: 0,
            order: Order::Idle,
            resume_order: None,
            queue: VecDeque::new(),
            target: None,
            path: VecDeque::new(),
            path_goal: None,
            path_stall: 0,
            speed: 0,
            moved_tick: 0,
            path_last_dist: 0,
            rally: None,
            carry: 0,
            carry_kind: CarryKind::None,
            gather_timer: 0,
            resource_target: None,
            depot: None,
            production: VecDeque::new(),
            addon: None,
            cargo: Vec::new(),
            in_transport: None,
            status: Status::default(),
            larva_timer: 0,
            resource_amount: 0,
            morph_to: None,
            last_damaged: 0,
            invincible: false,
            last_damage_source: 0,
            doodad_enabled: None,
            nuke_ready: false,
            nydus_exit: None,
            expires_at: 0,
            mine_armed_at: 0,
            repulse_dir: 0,
            repulse_flags: 0,
            turret_facing: 0,
            repair_debt: 0,
            hangar: default_hangar(type_id),
            hangar_capacity: default_hangar_capacity(type_id),
            special_cooldown: 0,
            order_carry: 0,
            speech_ticks: 0,
            speech_id: 0,
            death_timer: 0,
            rank_kills: 0,
            visibility_mask: u32::MAX,
            in_bunker: false,
            flag_26: false,
            overlay_flags: 0,
            flag_10: false,
            flag_14: false,
            flag_15: false,
            flag_29: false,
            flag_21: false,
            flag_18: false,
            shield_recharged: false,
            flag_16: false,
            flag_17: false,
            flag_19: false,
            field_100: 0,
            flag_22: false,
            flags_105: 0,
            flags_93: 0,
            field_091: 0,
            field_096: 0,
            field_0cf: 0,
            field_090: 8,
            field_094: 0,
            field_0ac: 0,
            field_0e1: 0,
            timer_087: 0,
            order_wait: 0,
            connected_unit: None,
            larva_spawn_side_values: [0; 4],
            order_row: 0,
            anim_tick: 0,
            anim: None,
            attacks_fired: 0,
        };
        let idx = self.units.len();
        self.index.insert(id, idx);
        self.units.push(unit);
        if is_creep_source(type_id) {
            self.creep_dirty = true;
        }
        self.start_animation(idx, ANIM_INIT);
        Ok(idx)
    }

    /// Spawn using pixel coordinates rather than tiles.
    pub fn spawn_px(
        &mut self,
        owner: usize,
        type_id: u16,
        x: i32,
        y: i32,
    ) -> Result<usize, SimError> {
        let ty = self.unit_stats(type_id);
        let shields = if ty.has_shields {
            fp(ty.shield_points as i32)
        } else {
            0
        };
        let energy_cap = self.max_energy_fixed(type_id, owner as u8);
        let energy = if ty.flags & 0x0020_0000 != 0 {
            energy_cap / ENERGY_SCALE
        } else {
            0
        };
        let id = self.next_id;
        self.next_id += 1;
        let unit = SimUnit {
            id,
            kills: 0,
            builder: 0,
            owner: owner as u8,
            type_id,
            hp: ty.hit_points_raw as i32,
            shields,
            energy,
            energy_frac: 0,
            energy_cap,
            x: x.saturating_mul(fp(1)),
            y: y.saturating_mul(fp(1)),
            facing: 0,
            alive: true,
            completed: true,
            credited: false,
            lifted: false,
            lift_phase: None,
            near_target_index: 0,
            addon_attached: false,
            build_remaining: 0,
            build_total: 0,
            cooldown: 0,
            order: Order::Idle,
            resume_order: None,
            queue: VecDeque::new(),
            target: None,
            path: VecDeque::new(),
            path_goal: None,
            path_stall: 0,
            speed: 0,
            moved_tick: 0,
            path_last_dist: 0,
            rally: None,
            carry: 0,
            carry_kind: CarryKind::None,
            gather_timer: 0,
            resource_target: None,
            depot: None,
            production: VecDeque::new(),
            addon: None,
            cargo: Vec::new(),
            in_transport: None,
            status: Status::default(),
            larva_timer: 0,
            resource_amount: 0,
            morph_to: None,
            last_damaged: 0,
            invincible: false,
            last_damage_source: 0,
            doodad_enabled: None,
            nuke_ready: false,
            nydus_exit: None,
            expires_at: 0,
            mine_armed_at: 0,
            repulse_dir: 0,
            repulse_flags: 0,
            turret_facing: 0,
            repair_debt: 0,
            hangar: default_hangar(type_id),
            hangar_capacity: default_hangar_capacity(type_id),
            special_cooldown: 0,
            order_carry: 0,
            speech_ticks: 0,
            speech_id: 0,
            death_timer: 0,
            rank_kills: 0,
            visibility_mask: u32::MAX,
            in_bunker: false,
            flag_26: false,
            overlay_flags: 0,
            flag_10: false,
            flag_14: false,
            flag_15: false,
            flag_29: false,
            flag_21: false,
            flag_18: false,
            shield_recharged: false,
            flag_16: false,
            flag_17: false,
            flag_19: false,
            field_100: 0,
            flag_22: false,
            flags_105: 0,
            flags_93: 0,
            field_091: 0,
            field_096: 0,
            field_0cf: 0,
            field_090: 8,
            field_094: 0,
            field_0ac: 0,
            field_0e1: 0,
            timer_087: 0,
            order_wait: 0,
            connected_unit: None,
            larva_spawn_side_values: [0; 4],
            order_row: 0,
            anim_tick: 0,
            anim: None,
            attacks_fired: 0,
        };
        let idx = self.units.len();
        self.index.insert(id, idx);
        self.units.push(unit);
        if is_creep_source(type_id) {
            self.creep_dirty = true;
        }
        self.start_animation(idx, ANIM_INIT);
        Ok(idx)
    }

    /// Spawn a unit and apply one `UPRP` property slot (trigger actions 11 and
    /// 44).  Only fields whose flag bit is set are applied; percentages clamp
    /// to 1..=100 and resources/hangar keep the low 16/8 bits, matching the
    /// retail loader.
    /// `0x00712ce0`: apply a `UPRP`/UNIT property slot.  The owner byte
    /// (field flag 0x01) is not read by retail; HP is raw fixed-point,
    /// shields/energy are percentages of the cap, resources only apply to
    /// `units.dat` flag 0x2000, and hangar is clamped to the unit's capacity.
    pub fn spawn_with_properties(
        &mut self,
        owner: usize,
        type_id: u16,
        x: i32,
        y: i32,
        props: &UnitProperties,
    ) -> Result<usize, SimError> {
        let idx = self.spawn_px(owner, type_id, x, y)?;
        let ty = self.unit_stats(type_id);
        let energy_cap = self.max_energy_fixed(type_id, self.units[idx].owner);
        let u = &mut self.units[idx];
        if props.field_flags & 0x02 != 0 {
            let pct = props.hit_points_percent.min(100) as i32;
            u.hp = ((ty.hit_points_raw as i64 * pct as i64 / 100) as i32).max(1);
        }
        if props.field_flags & 0x04 != 0 && ty.has_shields {
            let pct = props.shields_percent.min(100) as i32;
            let max = ty.shield_points as i32;
            u.shields = fp((max * pct / 100).min(max));
        }
        if props.field_flags & 0x08 != 0 {
            let pct = props.energy_percent.min(100) as i32;
            u.set_energy_fixed(energy_cap * pct / 100);
        }
        if props.field_flags & 0x10 != 0 && ty.flags & 0x2000 != 0 {
            u.resource_amount = (props.resource_amount as u16) as i32;
        }
        if props.field_flags & 0x20 != 0 {
            let capacity = u.hangar_capacity as u16;
            let count = props.hangar.min(capacity);
            if capacity != 0 && count != 0 {
                u.hangar = count as u8;
            }
        }
        let states = props.states & props.state_flags;
        if states & 0x01 != 0 {
            u.status.cloaked = true;
        }
        if states & 0x02 != 0 {
            u.status.burrowed = true;
        }
        if states & 0x04 != 0 && can_lift_building(u.type_id) {
            u.lifted = true;
        }
        if states & 0x08 != 0 {
            u.status.hallucination = true;
        }
        if states & 0x10 != 0 {
            u.invincible = true;
        }
        Ok(idx)
    }

    /// Ticks of progress per simulation tick (Operation CWAL runs at 20x).
    pub(super) fn build_step(&self, owner: u8) -> i32 {
        if self
            .players
            .get(owner as usize)
            .is_some_and(|p| p.cheats.fast_build)
        {
            20
        } else {
            1
        }
    }

    /// The Hatchery / Lair / Hive "Spawn Larva" button.  Recorded
    /// (`RecAct::SpawnLarva`, save tag 17).
    /// Spawn a Broodling with the `0x005b5e20`/`0x005cc810` 1350-frame
    /// lifetime.
    pub(super) fn spawn_broodling(&mut self, owner: usize, x: i32, y: i32) {
        if let Ok(i) = self.spawn_px(owner, uid::BROODLING, x, y) {
            self.units[i].expires_at = self.tick as i64 + BROODLING_LIFETIME_TICKS;
        }
    }

    pub fn spawn_larva(&mut self, hatchery_id: u32) -> Result<(), SimError> {
        self.spawn_larva_unrecorded(hatchery_id)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::SpawnLarva {
                    hatchery: hatchery_id,
                },
            );
        }
        Ok(())
    }

    pub(super) fn spawn_larva_unrecorded(&mut self, hatchery_id: u32) -> Result<(), SimError> {
        let i = self.index_of(hatchery_id).ok_or(SimError::BadUnit)?;
        let u = &self.units[i];
        if !u.alive || !u.completed || !matches!(u.type_id, uid::HATCHERY | uid::LAIR | uid::HIVE) {
            return Err(SimError::MissingRequirements);
        }
        let owner = u.owner;
        if self.larva_count_for(i) >= 3 {
            return Err(SimError::QueueFull);
        }
        let parent = self.units[i].id;
        let (sx, sy) = self.larva_spawn_spot(i);
        if let Ok(l) = self.spawn_px(owner as usize, uid::LARVA, sx, sy) {
            self.units[l].connected_unit = Some(parent);
        }
        Ok(())
    }

    /// Retail larva placement (`0x005dfabc`..`0x005dffe0`): probe the four
    /// Hatchery sides (translation batch 38), keep the lowest side score with
    /// the batch-39 pair fold, then try the bottom-right half fallback.  A
    /// map without creep falls back to the historical bottom point.
    pub(super) fn larva_spawn_spot(&mut self, i: usize) -> (i32, i32) {
        let id = self.units[i].id;
        let hx = self.units[i].x / fp(1);
        let hy = self.units[i].y / fp(1);
        let bottom = self.data.unit(self.units[i].type_id).dimensions.bottom as i32;
        let mut candidates: Vec<(u32, i32, i32, u32)> = Vec::with_capacity(4);
        if let Some((x, y, score)) = self.a38_larva_spot_bottom(id, 0x65) {
            candidates.push((0, x, y, score as u32));
        }
        if let Some((x, y, score)) = self.a38_larva_spot_left(id, 0x65) {
            candidates.push((1, x, y, score as u32));
        }
        if let Some((x, y, score)) = self.a38_larva_spot_right(id, 0x65) {
            candidates.push((2, x, y, score as u32));
        }
        if let Some((x, y, score)) = self.a38_larva_spot_top(id, 0x65) {
            candidates.push((3, x, y, score as u32));
        }
        let mut incumbent = (0u32, 0x65u32);
        let mut chosen = None;
        for (order, x, y, score) in candidates {
            let next = self.units[i].a39_keep_smaller_pair(incumbent, (order, score));
            if next != incumbent {
                chosen = Some((x, y));
                incumbent = next;
            }
        }
        if let Some(pos) = chosen {
            return pos;
        }
        if let Some((x, y, _)) = self.a38_larva_spot_bottom_right_half(id) {
            return (x, y);
        }
        (hx, hy + bottom + 10)
    }

    pub(super) fn is_under_dark_swarm(&self, target: usize) -> bool {
        let t = &self.units[target];
        if self.is_air_unit(target) || self.data.unit(t.type_id).is_building() {
            return false;
        }
        self.units.iter().any(|u| {
            u.alive
                && u.type_id == uid::DARK_SWARM
                && (u.x - t.x).abs() < fp(96)
                && (u.y - t.y).abs() < fp(96)
        })
    }

    pub(super) fn spawn_dark_swarm(&mut self, owner: u8, pos: (i32, i32)) {
        // Translation batch 37 (`0x005dc1e0`): clamp with the `0xca` margin
        // row, set the hidden status bit and the removal timer, then keep the
        // sim's 360-tick temporary-unit expiry.
        if let Ok(id) = self.a37_spawn_dark_swarm(owner as usize, (pos.0 / fp(1), pos.1 / fp(1)))
            && let Some(idx) = self.index_of(id)
        {
            let expiry = self.tick as i64 + 360;
            self.units[idx].expires_at = if self.units[idx].expires_at > 0 {
                self.units[idx].expires_at.min(expiry)
            } else {
                expiry
            };
        }
    }

    pub(super) fn spawn_temporary(
        &mut self,
        owner: u8,
        type_id: u16,
        pos: (i32, i32),
        lifetime: i64,
    ) -> Option<usize> {
        let idx = self
            .spawn_px(owner as usize, type_id, pos.0 / fp(1), pos.1 / fp(1))
            .ok()?;
        if lifetime > 0 {
            // `CUnit+0x110` is the temporary-unit lifetime (`FUN_005b6810`
            // keeps the smaller of the current and new value).
            let expiry = self.tick as i64 + lifetime;
            self.units[idx].expires_at = if self.units[idx].expires_at > 0 {
                self.units[idx].expires_at.min(expiry)
            } else {
                expiry
            };
        }
        self.units[idx].completed = true;
        Some(idx)
    }
}
