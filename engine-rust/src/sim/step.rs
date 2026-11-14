//! `impl World`: step (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

impl World {
    // ------------------------------------------------------------------
    // Tick
    // ------------------------------------------------------------------

    pub fn step(&mut self) {
        // The world keeps running after Victory / Defeat: retail units move
        // behind the end dialog (Wasteland recording 302.5-307.5 s) until
        // the player leaves for the score screen.
        self.tick += 1;
        self.routes = Default::default();
        for u in &mut self.units {
            if !u.alive && u.death_timer > 0 {
                u.death_timer -= 1;
            }
        }
        self.matrix_cues.clear();
        self.nuke_warnings.clear();
        let n = self.units.len();
        for i in 0..n {
            if !self.units[i].alive {
                continue;
            }
            self.update_timers(i);
            self.update_construction(i);
            self.update_production(i);
        }
        for i in 0..n {
            if !self.units[i].alive {
                continue;
            }
            self.tick_animation(i);
            if !self.units[i].alive {
                continue;
            }
            let order = self.units[i].order.clone();
            // Recovered `0x7a178340` dispatch cadence: the carry counter is
            // decremented every tick and reset to 8 when it reaches zero, so
            // the carry-gated handlers run on ticks 0, 8, 16, ... per unit.
            let carry = self.units[i].order_carry;
            // Translation batch 35 (`0x005d834c`): decrement/reset carry.
            self.units[i].a35_advance_order_carry();
            let think = order_runs_this_tick(order.order_id(), carry);
            if think {
                // Enter the recovered handler routine for this row.  The
                // typed order state machine below remains the gameplay
                // authority; this is the recovered dispatcher boundary and
                // records the hit in `World::handler_hits`.
                crate::handlers::tick_row(self, self.units[i].id, order.order_id());
            }
            if !matches!(order, Order::Die) {
                self.execute_order(i, order, think);
            }
        }
        self.resolve_collisions();
        self.apply_air_repulse();
        self.turn_turrets();
        if self.tick.is_multiple_of(CREEP_GROW_INTERVAL as u64) {
            self.grow_creep();
        }
        self.update_specials();
        self.update_pending_hits();
        self.update_projectiles();
        self.advance_nuke_strikes();
        self.update_larva();
        self.update_vision();
        self.resolve_deaths();
        // The first check runs on the first tick (the order's timer starts
        // at 0), before the triggers: Backwater Station's "Glad to see you,
        // boys" (needs Raynor rescued) shows on the first game frame of the
        // retail recording.  Then every 15 ticks.
        if (self.tick - 1).is_multiple_of(RESCUE_INTERVAL) {
            self.update_rescues();
        }
        self.credit_scores();
        self.update_ctf();
        self.check_victory();
        if self.triggers.is_some() {
            let mut triggers = self.triggers.take();
            if let Some(t) = triggers.as_mut() {
                t.process(self);
            }
            self.triggers = triggers;
        }
    }

    pub(super) fn update_timers(&mut self, i: usize) {
        let ty = self.data.unit(self.units[i].type_id).clone();
        if self.units[i].status.stasis_ticks > 0 {
            // Stasis freezes the unit; its own timer still runs on the
            // recovered eighth-tick status cadence.
            if self.tick.is_multiple_of(ACID_SPORE_UPDATE_INTERVAL) {
                self.units[i].status.stasis_ticks -= 1;
            }
            return;
        }
        self.units[i].anim_tick = self.units[i].anim_tick.wrapping_add(1);
        if self.units[i].cooldown > 0 {
            self.units[i].cooldown -= 1;
        }
        self.units[i].tick_speech();
        self.units[i].tick_timer_087();
        // `0x005d91e0`: the recharge flag only survives the tick that set it.
        self.units[i].shield_recharged = false;
        if ty.has_shields {
            let max = fp(ty.shield_points as i32);
            if self.units[i].shields < max {
                self.units[i].shields = (self.units[i].shields + SHIELD_REGEN_PER_TICK).min(max);
            }
        }
        if ty.regenerates() {
            let max = ty.hit_points_raw as i32;
            if self.units[i].hp < max && self.units[i].hp > 0 {
                self.units[i].hp = (self.units[i].hp + ZERG_REGEN_PER_TICK).min(max);
            }
        }
        // Terran structures below one third HP burn at the recovered rate of
        // 20/256 HP per frame (1.875 HP/s at 24 Hz).
        if self.units[i].completed && is_terran_building(self.units[i].type_id) {
            let max = ty.hit_points_raw as i32;
            if self.units[i].hp > 0 && self.units[i].hp * 3 < max {
                self.units[i].hp -= 20;
                if self.units[i].hp <= 0 {
                    self.kill_unit(i);
                    return;
                }
            }
        }
        if self.units[i].special_cooldown > 0 {
            self.units[i].special_cooldown -= 1;
        }
        // `0x005d93a0` decrements CUnit+0x110 and kills the temporary unit
        // when it reaches zero (Dark Swarm, Disruption Web, Scanner Sweep,
        // Nuclear Missile).
        if self.units[i].expires_at > 0 && self.tick as i64 >= self.units[i].expires_at {
            self.kill_unit(i);
            return;
        }
        if ty.flags & 0x0020_0000 != 0 {
            // Personal cloaking drains 10 (Ghost) or 13 (Wraith) energy in
            // 1/256 points per tick and suppresses the +8 regeneration, for
            // the documented net 18/256 and 21/256.  Every other energy user
            // regenerates 8/256 per tick up to its maximum.
            let cloak_drain = if self.units[i].status.cloaked && !self.units[i].status.hallucination
            {
                match self.units[i].type_id {
                    // OpenBW `unit_cloak_energy_cost`: the Ghost heroes
                    // (and Infested Kerrigan, 51) pay 10, Tom Kazansky's
                    // Wraith (21) pays 13.
                    t if is_ghost_type(t) || t == 51 => GHOST_CLOAK_COST_PER_TICK,
                    uid::WRAITH | 21 => WRAITH_CLOAK_COST_PER_TICK,
                    _ => 0,
                }
            } else {
                0
            };
            if cloak_drain > 0 {
                let owner = self.units[i].owner as usize;
                if !self.players[owner].cheats.infinite_energy
                    && !self.units[i].spend_energy_fixed(cloak_drain)
                {
                    self.units[i].status.cloaked = false;
                }
            } else {
                self.units[i].add_energy_fixed(ENERGY_REGEN_PER_TICK);
            }
        }
        let status_update = self.tick.is_multiple_of(ACID_SPORE_UPDATE_INTERVAL);
        let (irradiate, plague, matrix_expired) = {
            let s = &mut self.units[i].status;
            // `FUN_005cf290` runs every eighth tick.  Its byte timers are
            // update counts, not logical ticks: Ensnare/Plague/Irradiate 75,
            // Lockdown/Stasis 131, Maelstrom 22, Matrix 168.
            let matrix_expired = if status_update { s.tick_matrix() } else { None };
            // Damage is applied on the same update that decrements the last
            // point of the timer (`FUN_005cf290` calls the damage routine
            // before testing for zero).
            let irradiate_due = s.irradiate_ticks;
            let plague_due = s.plague_ticks;
            if status_update {
                s.tick_acid_spores();
                s.lockdown_ticks = (s.lockdown_ticks - 1).max(0);
                s.irradiate_ticks = (s.irradiate_ticks - 1).max(0);
                s.plague_ticks = (s.plague_ticks - 1).max(0);
                s.ensnare_ticks = (s.ensnare_ticks - 1).max(0);
                s.maelstrom_ticks = (s.maelstrom_ticks - 1).max(0);
            }
            s.stim_ticks = (s.stim_ticks - 1).max(0);
            s.psi_storm_guard = s.psi_storm_guard.saturating_sub(1);
            s.disruption_ticks = (s.disruption_ticks - 1).max(0);
            s.revealed_ticks = (s.revealed_ticks - 1).max(0);
            (irradiate_due, plague_due, matrix_expired)
        };
        if let Some(absorbed) = matrix_expired {
            self.matrix_cues.push(MatrixCue {
                unit: self.units[i].id,
                x: self.units[i].x / fp(1),
                y: self.units[i].y / fp(1),
                absorbed,
                broke: true,
            });
        }
        if status_update && irradiate > 0 {
            // `FUN_005d25e3`: (Irradiate damage << 8) / cooldown, weapon 34
            // (250 * 256 / 75 = 853) every eighth tick.  The aura damages
            // every biological unit within 32 px of the irradiated unit's
            // collision box, including itself; burrowed units only take the
            // direct damage when they are the irradiated unit.
            let damage = self
                .data
                .weapon(wid::IRRADIATE)
                .map(|w| (w.damage as i32) * fp(1) / (w.cooldown.max(1) as i32))
                .unwrap_or(0);
            if damage > 0 {
                let eligible = |j: usize| {
                    let u = &self.units[j];
                    let ty = self.data.unit(u.type_id);
                    u.alive
                        && ty.is_organic()
                        && !ty.is_building()
                        && !matches!(
                            u.type_id,
                            uid::LARVA | uid::EGG | uid::COCOON | uid::LURKER_EGG
                        )
                };
                let mut targets = Vec::new();
                // OpenBW `deal_irradiate_damage`: a burrowed unit only
                // hurts itself; a loaded one only its transport; otherwise
                // the aura (loaded units are out of reach).
                if self.units[i].status.burrowed {
                    if eligible(i) {
                        targets.push(i);
                    }
                } else if let Some(carrier) = self.units[i].in_transport {
                    if let Some(c) = self.index_of(carrier)
                        && eligible(c)
                    {
                        targets.push(c);
                    }
                } else {
                    for j in 0..self.units.len() {
                        let u = &self.units[j];
                        if u.in_transport.is_some() || !eligible(j) {
                            continue;
                        }
                        if j != i && u.status.burrowed {
                            continue;
                        }
                        if self.box_gap_px(i, j) <= IRRADIATE_RADIUS_PX {
                            targets.push(j);
                        }
                    }
                }
                for j in targets {
                    self.damage_hp(j, damage, 0);
                }
            }
        }
        if status_update && plague > 0 {
            // `FUN_005dc2c0`: plague damage is weapon 60's 300 * 256 / 0x4c
            // per update, and is skipped when it would reach the current HP
            // (so the spell cannot kill by itself).
            let damage = self
                .data
                .weapon(wid::PLAGUE)
                .map(|w| (w.damage as i32) * fp(1) / 0x4c)
                .unwrap_or(0);
            let hp = self.units[i].hp;
            if damage > 0 && damage < hp {
                self.damage_hp(i, damage, 0);
            }
        }
    }

    /// `Rescue Passive` (order 160): every [`RESCUE_INTERVAL`] ticks a unit
    /// of a rescuable player joins the first human (not computer, not
    /// rescuable) player with a unit within [`RESCUE_RANGE_PX`] of its box.  The
    /// cadence is the retail handler's (`+0x54 = 0xf`, FACT); range and
    /// rescuer rule are INFERENCE (the Wasteland enemy sits near Raynor and
    /// must not take him; falsifier: a retail recording of that rescue).
    pub(super) fn update_rescues(&mut self) {
        if !self.players.iter().any(|p| p.rescuable) {
            return;
        }
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if !u.alive
                || !self
                    .players
                    .get(u.owner as usize)
                    .is_some_and(|p| p.rescuable)
            {
                continue;
            }
            let rescuer = self.units.iter().enumerate().find(|&(j, o)| {
                o.alive
                    && (o.owner as usize) < 8
                    && !self
                        .players
                        .get(o.owner as usize)
                        .is_none_or(|p| p.rescuable || p.defeated || p.is_computer)
                    && !is_resource_type(o.type_id)
                    && o.type_id != uid::START_LOCATION
                    && self.box_gap_px(i, j) <= RESCUE_RANGE_PX
            });
            if let Some(owner) = rescuer.map(|(_, o)| o.owner) {
                self.units[i].owner = owner;
                self.rescues.push((self.units[i].id, owner));
            }
        }
    }

    pub(super) fn check_victory(&mut self) {
        if self.players.iter().any(|p| p.cheats.stay_alive) {
            return;
        }
        // Special melee rules run before the trigger-driven victory gate:
        // they are lobby rules, not map objectives.
        if let Some(goal) = self.greed_goal
            && let Some(p) = self
                .players
                .iter()
                .position(|p| !p.defeated && p.minerals_gathered >= goal && p.gas_gathered >= goal)
        {
            self.winner = Some(p as u8);
            return;
        }
        if let Some(deadline) = self.slaughter_deadline
            && self.tick >= deadline
        {
            let best = self
                .players
                .iter()
                .enumerate()
                .filter(|(_, p)| !p.defeated)
                .max_by_key(|(i, p)| (p.kills, std::cmp::Reverse(*i)))
                .map(|(i, _)| i as u8);
            self.winner = best;
            return;
        }
        if self.trigger_victory {
            return;
        }
        // A sandbox with a single owner (unit tests, spell labs) has no
        // opposing side and therefore no victory condition.
        let owners: std::collections::HashSet<u8> = self.units.iter().map(|u| u.owner).collect();
        if owners.len() < 2 {
            return;
        }
        let alive_players: Vec<u8> = (0..self.players.len() as u8)
            .filter(|&p| {
                !self.players[p as usize].defeated
                    && self.units.iter().any(|u| {
                        u.owner == p
                            && !is_resource_type(u.type_id)
                            && (u.alive || u.in_transport.is_some())
                    })
            })
            .collect();
        // The game ends when everyone still standing is on one side (a lone
        // survivor, or one team in a team game); `winner` names the side's
        // first player and `has_won` answers for the whole side.
        let one_side = alive_players
            .iter()
            .all(|&p| alive_players.first().is_none_or(|&f| self.same_side(p, f)));
        if one_side {
            self.winner = alive_players.first().copied();
        }
    }

    /// Town hall an AI of `player` attacks: `prefer`'s (the local player in
    /// the client, as before teams existed) when `prefer` is an enemy, else
    /// the first enemy player's by player order.  Allies are never chosen.
    pub fn enemy_town_hall(&self, player: u8, prefer: u8) -> Option<(i32, i32)> {
        let hall = |p: u8| {
            self.units
                .iter()
                .find(|u| {
                    u.alive
                        && u.owner == p
                        && matches!(
                            u.type_id,
                            uid::COMMAND_CENTER
                                | uid::NEXUS
                                | uid::HATCHERY
                                | uid::LAIR
                                | uid::HIVE
                        )
                })
                .map(|u| (u.x / 256, u.y / 256))
        };
        if self.is_enemy(player, prefer)
            && let Some(at) = hall(prefer)
        {
            return Some(at);
        }
        (0..self.players.len() as u8)
            .filter(|&p| self.is_enemy(player, p) && !self.players[p as usize].defeated)
            .find_map(hall)
    }

    /// Engine score bookkeeping (OpenBW `add_completed_unit` and the kill
    /// path): the first tick a unit is complete, its `units.dat` build score
    /// goes to its owner's unit score (Men) or building score (Building).
    /// Starting and pre-placed units count, which is why a fresh Protoss
    /// melee start already shows Units 200 / Structures 400 (retail score
    /// screen, 2026-11-04 21:14:04).  Hallucinations do not (OpenBW also
    /// skips turret subunits, which this engine does not create as units).
    pub fn credit_scores(&mut self) {
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if !u.alive || !u.completed || u.credited {
                continue;
            }
            let owner = u.owner as usize;
            let resource = is_resource_type(u.type_id);
            let hallucination = u.status.hallucination;
            self.units[i].credited = true;
            if resource || hallucination || owner >= self.players.len() {
                continue;
            }
            let ty = self.data.unit(self.units[i].type_id);
            let score = ty.build_score as i32;
            if ty.group_flags & GROUP_MEN != 0 {
                self.players[owner].scores[1] += score;
                self.players[owner].units_produced += 1;
            } else if ty.group_flags & GROUP_BUILDING != 0 {
                self.players[owner].scores[2] += score;
                self.players[owner].structures_built += 1;
            }
        }
    }

    /// Apply one single-player cheat for `player` (see [`cheat`]).  Recorded
    /// (`RecAct::Cheat`) so a save or replay keeps the resources, upgrades
    /// and toggles it changed.
    pub fn apply_cheat(&mut self, player: u8, code: u8) -> Result<(), SimError> {
        self.apply_cheat_unrecorded(player, code)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(self.tick, crate::replay::RecAct::Cheat { player, code });
        }
        Ok(())
    }

    pub(super) fn apply_cheat_unrecorded(&mut self, player: u8, code: u8) -> Result<(), SimError> {
        let idx = player as usize;
        if idx >= self.players.len() {
            return Err(SimError::BadOwner);
        }
        match code {
            cheat::SHOW_ME_THE_MONEY => {
                self.players[idx].minerals += 10_000;
                self.players[idx].gas += 10_000;
            }
            cheat::WHATS_MINE_IS_MINE => self.players[idx].minerals += 500,
            cheat::BREATHE_DEEP => self.players[idx].gas += 500,
            cheat::POWER_OVERWHELMING => {
                self.players[idx].cheats.invincible = !self.players[idx].cheats.invincible
            }
            cheat::THE_GATHERING => {
                self.players[idx].cheats.infinite_energy = !self.players[idx].cheats.infinite_energy
            }
            cheat::FOOD_FOR_THOUGHT => {
                self.players[idx].cheats.no_supply = !self.players[idx].cheats.no_supply
            }
            cheat::BLACK_SHEEP_WALL | cheat::WAR_AINT_WHAT_IT_USED_TO_BE => {
                self.players[idx].cheats.reveal = !self.players[idx].cheats.reveal
            }
            cheat::OPERATION_CWAL => {
                self.players[idx].cheats.fast_build = !self.players[idx].cheats.fast_build
            }
            cheat::SOMETHING_FOR_NOTHING => {
                for level in &mut self.players[idx].upgrades {
                    *level = (*level + 1).min(3);
                }
            }
            cheat::MEDIEVAL_MAN => {
                let on = !self.players[idx].cheats.free_tech;
                self.players[idx].cheats.free_tech = on;
                self.players[idx].cheats.free_upgrades = on;
            }
            cheat::MODIFY_THE_PHASE_VARIANCE => {
                self.players[idx].cheats.build_anything = !self.players[idx].cheats.build_anything
            }
            cheat::THERE_IS_NO_COW_LEVEL => self.winner = Some(player),
            cheat::GAME_OVER_MAN => self.players[idx].defeated = true,
            cheat::STAYING_ALIVE => {
                self.players[idx].cheats.stay_alive = !self.players[idx].cheats.stay_alive
            }
            cheat::NOGLUES => self.cheat_no_enemy_spells = !self.cheat_no_enemy_spells,
            _ => return Err(SimError::Unsupported),
        }
        Ok(())
    }

    /// Start Capture the Flag: one beacon per active player near its town
    /// hall, flags hidden for three game minutes, beacons movable for ten.
    pub fn start_ctf(&mut self) {
        if self.ctf.is_some() {
            return;
        }
        let mut beacons = [None; 8];
        let mut flags = Vec::new();
        let active = self.players.len().min(8);
        for (p, beacon_slot) in beacons.iter_mut().enumerate().take(active) {
            if self.players[p].defeated {
                continue;
            }
            let hall = self.units.iter().find(|u| {
                u.alive
                    && u.owner as usize == p
                    && u.completed
                    && matches!(
                        u.type_id,
                        uid::COMMAND_CENTER | uid::NEXUS | uid::HATCHERY | uid::LAIR | uid::HIVE
                    )
            });
            let (x, y) = hall
                .map(|u| (u.x / fp(1) + 64, u.y / fp(1) + 48))
                .or_else(|| {
                    self.units
                        .iter()
                        .find(|u| u.alive && u.owner as usize == p)
                        .map(|u| (u.x / fp(1) + 48, u.y / fp(1) + 48))
                })
                .unwrap_or((16 + p as i32 * 48, 16));
            let beacon_type = match self.players[p].race {
                Race::Terran => uid::TERRAN_FLAG_BEACON,
                Race::Zerg => uid::ZERG_FLAG_BEACON,
                Race::Protoss => uid::PROTOSS_FLAG_BEACON,
                Race::Neutral => uid::TERRAN_FLAG_BEACON,
            };
            if let Ok(i) = self.spawn_px(p, beacon_type, x, y) {
                self.units[i].invincible = true;
                self.units[i].completed = true;
                *beacon_slot = Some(self.units[i].id);
            }
            flags.push(CtfFlag {
                owner: p as u8,
                home: (x, y),
                pos: (x, y),
                carrier: None,
                return_at: None,
                released: false,
            });
        }
        let secs = TICKS_PER_SECOND as u64;
        self.ctf = Some(CtfState {
            flags,
            beacons,
            release_at: self.tick + 3 * 60 * secs,
            move_until: self.tick + 10 * 60 * secs,
            beacon_moved: [false; 8],
        });
    }

    /// Move a flag beacon once during the first ten game minutes.
    pub fn move_beacon(&mut self, player: u8, pos: (i32, i32)) -> Result<(), SimError> {
        let idx = player as usize;
        let Some(beacon) = self
            .ctf
            .as_ref()
            .and_then(|c| c.beacons.get(idx).copied().flatten())
        else {
            return Err(SimError::Unsupported);
        };
        if self
            .ctf
            .as_ref()
            .is_none_or(|c| self.tick > c.move_until || c.beacon_moved[idx])
        {
            return Err(SimError::Unsupported);
        }
        let Some(i) = self.index_of(beacon) else {
            return Err(SimError::BadUnit);
        };
        self.units[i].x = pos.0 * fp(1);
        self.units[i].y = pos.1 * fp(1);
        if let Some(c) = self.ctf.as_mut() {
            c.beacon_moved[idx] = true;
            if let Some(flag) = c.flags.iter_mut().find(|f| f.owner == player) {
                flag.home = pos;
                if flag.carrier.is_none() {
                    flag.pos = pos;
                }
            }
        }
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(self.tick, crate::replay::RecAct::Beacon { player, pos });
        }
        Ok(())
    }

    pub(super) fn update_ctf(&mut self) {
        let Some(ctf) = self.ctf.as_ref() else {
            return;
        };
        let now = self.tick;
        let release_at = ctf.release_at;
        let mut drops: Vec<(usize, Option<u32>, (i32, i32))> = Vec::new();
        let mut captures: Vec<u8> = Vec::new();
        let mut returns: Vec<usize> = Vec::new();
        let mut pickups: Vec<(usize, u32, (i32, i32))> = Vec::new();
        let mut return_home: Vec<usize> = Vec::new();
        for (fi, flag) in ctf.flags.iter().enumerate() {
            if now < release_at || !flag.released {
                continue;
            }
            if let Some(carrier) = flag.carrier {
                let Some(ci) = self.index_of(carrier) else {
                    drops.push((fi, None, flag.pos));
                    continue;
                };
                let u = &self.units[ci];
                if !u.alive || u.in_transport.is_some() {
                    drops.push((fi, None, (u.x / fp(1), u.y / fp(1))));
                    continue;
                }
                if u.owner != flag.owner {
                    let home = ctf
                        .flags
                        .iter()
                        .find(|f| f.owner == u.owner)
                        .map(|f| f.home)
                        .unwrap_or(flag.pos);
                    if (u.x / fp(1) - home.0).abs() < 64 && (u.y / fp(1) - home.1).abs() < 64 {
                        captures.push(flag.owner);
                    }
                }
            } else if let Some(at) = flag.return_at {
                if now >= at {
                    returns.push(fi);
                }
            } else {
                for (i, u) in self.units.iter().enumerate() {
                    if !u.alive
                        || u.in_transport.is_some()
                        || !self.data.unit(u.type_id).is_worker()
                    {
                        continue;
                    }
                    let (ux, uy) = (u.x / fp(1), u.y / fp(1));
                    if (ux - flag.pos.0).abs() >= 40 || (uy - flag.pos.1).abs() >= 40 {
                        continue;
                    }
                    if u.owner == flag.owner {
                        if flag.pos != flag.home {
                            return_home.push(fi);
                        }
                    } else {
                        pickups.push((fi, u.id, (ux, uy)));
                        let _ = i;
                        break;
                    }
                }
            }
        }
        for (fi, carrier, pos) in drops {
            if let Some(c) = self.ctf.as_mut() {
                c.flags[fi].carrier = carrier;
                c.flags[fi].pos = pos;
                c.flags[fi].return_at = Some(now + 30 * TICKS_PER_SECOND as u64);
            }
        }
        for fi in returns {
            if let Some(c) = self.ctf.as_mut() {
                c.flags[fi].pos = c.flags[fi].home;
                c.flags[fi].return_at = None;
                c.flags[fi].released = true;
            }
        }
        for fi in return_home {
            if let Some(c) = self.ctf.as_mut() {
                c.flags[fi].pos = c.flags[fi].home;
                c.flags[fi].return_at = None;
            }
        }
        for (fi, unit, pos) in pickups {
            if let Some(c) = self.ctf.as_mut() {
                c.flags[fi].carrier = Some(unit);
                c.flags[fi].return_at = None;
                c.flags[fi].pos = pos;
            }
        }
        for owner in captures {
            self.defeat_player(owner);
        }
        // Release and carrier-follow updates are computed against the
        // immutable world, then applied to the flag state.
        let updates: Vec<(usize, (i32, i32), bool)> = {
            let Some(c) = self.ctf.as_ref() else {
                return;
            };
            c.flags
                .iter()
                .enumerate()
                .map(|(fi, flag)| {
                    let pos = flag
                        .carrier
                        .and_then(|id| self.unit(id))
                        .map(|u| (u.x / fp(1), u.y / fp(1)))
                        .unwrap_or(flag.pos);
                    (fi, pos, flag.released || now >= release_at)
                })
                .collect()
        };
        if let Some(c) = self.ctf.as_mut() {
            for (fi, pos, released) in updates {
                c.flags[fi].pos = pos;
                c.flags[fi].released = released;
            }
        }
    }

    /// Remove a player from a Capture the Flag match (the flag owner is
    /// "destroyed" when an enemy flag reaches the capturer's beacon).
    pub(super) fn defeat_player(&mut self, player: u8) {
        let idx = player as usize;
        if idx >= self.players.len() {
            return;
        }
        self.players[idx].defeated = true;
        for u in &mut self.units {
            if u.owner == player && !is_resource_type(u.type_id) {
                u.alive = false;
                u.order = Order::Die;
            }
        }
    }

    /// Spider Mines (OpenBW `order_SpiderMine`).  A new mine sits in the
    /// open for 60 frames, takes 4 to burrow, and is then hidden and armed.
    /// It wakes for the nearest enemy inside the 96 px square around it
    /// (3 tiles of target acquisition) that is not invincible, a grounded
    /// building, a flyer or a hovering unit; unburrows (4 frames), chases
    /// it at 16 px per frame while it stays within 576 px, and explodes
    /// when their boxes are within 30 px (weapons.dat row 6: 125 explosive,
    /// radial splash 50 / 75 / 100).  A mine that loses its target digs in
    /// again.  `mine_armed_at` is the tick of the next state change.
    pub(super) fn update_specials(&mut self) {
        let now = self.tick as i64;
        let mut detonations: Vec<usize> = Vec::new();
        for i in 0..self.units.len() {
            if !self.units[i].alive || self.units[i].type_id != uid::SPIDER_MINE {
                continue;
            }
            if self.units[i].status.lockdown_ticks > 0 || self.units[i].status.stasis_ticks > 0 {
                continue;
            }
            // Fresh mine: start the 60-frame wait.
            if self.units[i].mine_armed_at == 0 {
                self.units[i].mine_armed_at = now + SPIDER_MINE_WAIT_FRAMES;
            }
            let armed_at = self.units[i].mine_armed_at;
            let target = self.units[i].target.and_then(|t| self.index_of(t));
            if self.units[i].status.burrowed {
                // Armed: look for a victim.
                let (mx, my, owner) = (self.units[i].x, self.units[i].y, self.units[i].owner);
                let half = SPIDER_MINE_SEEK_PX * fp(1);
                let victim = (0..self.units.len())
                    .filter(|&n| {
                        let v = &self.units[n];
                        n != i
                            && v.alive
                            && self.hostile(owner, v.owner)
                            && !v.invincible
                            && !self.data.unit(v.type_id).is_building()
                            && !self.is_air_unit(n)
                            && self.data.unit(v.type_id).movement_flags != 0xc1
                            && (v.x - mx).abs() <= half
                            && (v.y - my).abs() <= half
                    })
                    .map(|n| (self.distance(i, n), n))
                    .min()
                    .map(|(_, n)| n);
                if let Some(v) = victim {
                    self.units[i].status.burrowed = false;
                    self.units[i].target = Some(self.units[v].id);
                    self.units[i].mine_armed_at = now + SPIDER_MINE_BURROW_FRAMES;
                }
                continue;
            }
            match target {
                // Waiting in the open, then burrowing.
                None => {
                    if now >= armed_at + SPIDER_MINE_BURROW_FRAMES {
                        self.units[i].status.burrowed = true;
                        self.units[i].path.clear();
                        self.units[i].path_goal = None;
                    }
                }
                // Unburrowing, then the chase.
                Some(_) if now < armed_at => {}
                Some(v) => {
                    let alive = self.units[v].alive && !self.is_air_unit(v);
                    let dist = self.units_distance_px(i, v);
                    if !alive || dist > SPIDER_MINE_CHASE_PX {
                        self.units[i].target = None;
                        self.units[i].path.clear();
                        self.units[i].path_goal = None;
                        self.units[i].mine_armed_at = now + SPIDER_MINE_WAIT_FRAMES;
                    } else if dist <= SPIDER_MINE_BLAST_PX {
                        detonations.push(i);
                    } else {
                        let (vx, vy) = (self.units[v].x / fp(1), self.units[v].y / fp(1));
                        self.move_towards(i, (vx, vy), fp(4));
                    }
                }
            }
        }
        for i in detonations {
            let pos = (self.units[i].x, self.units[i].y);
            if let Some(w) = self.data.weapon(wid::SPIDER_MINES).cloned() {
                let owner = self.units[i].owner;
                self.resolve_hit_type(owner, None, i, wid::SPIDER_MINES, &w, pos, 1, 1);
            }
            self.kill_unit(i);
        }
        self.bunker_fire();
        self.medic_healing();
    }
}
