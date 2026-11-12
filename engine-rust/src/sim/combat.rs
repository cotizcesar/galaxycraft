//! `impl World`: combat (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

impl World {
    /// Advance toward a hostile without starting a new attack.  Used on the
    /// ticks between carry passes: retail keeps moving the flingy every tick
    /// while the order handler only re-validates the target every 8th tick.
    pub(super) fn advance_attack(&mut self, i: usize, target: u32) {
        if let Some(tidx) = self.index_of(target)
            && self.units[tidx].alive
        {
            // Already in weapon reach: stand still (walking on into the
            // target shoved both fighters across the map).
            if let Some((_, w)) = self.weapon_for(i, tidx)
                && self.units_distance_px(i, tidx) <= self.weapon_max_range_px(i, tidx, &w)
            {
                self.units[i].path.clear();
                self.units[i].path_goal = None;
                return;
            }
            // Game pixels (see `attack_target`).
            let p = (self.units[tidx].x / fp(1), self.units[tidx].y / fp(1));
            self.move_towards(i, p, fp(2));
        }
    }

    pub(super) fn attack_target(&mut self, i: usize, target: u32) {
        let Some(tidx) = self.index_of(target) else {
            self.order_done(i);
            return;
        };
        if !self.units[tidx].alive {
            self.order_done(i);
            return;
        }
        self.units[i].target = Some(target);
        if self.attack_target_no_move(i, target) {
            // In range: the unit stands and fires.  A path left over from
            // the approach made the renderer play the walk animation over
            // the attack (the Firebat's flame never showed).
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            return;
        }
        // `move_towards` takes game pixels: passing the fixed-point position
        // sent every chasing unit toward a point 256x farther away, off the
        // map (Wasteland: Marines at x 3700-4200 on a 2048 px map).
        let (tx, ty) = (self.units[tidx].x, self.units[tidx].y);
        self.move_towards(i, (tx / fp(1), ty / fp(1)), fp(2));
    }

    /// Fire if in range.  Returns true when the unit is in range (whether or
    /// not the cooldown allowed a shot this tick).
    pub(super) fn attack_target_no_move(&mut self, i: usize, target: u32) -> bool {
        let Some(tidx) = self.index_of(target) else {
            return false;
        };
        if !self.units[tidx].alive {
            return false;
        }
        self.units[i].target = Some(target);
        let Some((weapon_id, weapon)) = self.weapon_for(i, tidx) else {
            return self.special_attack(i, tidx);
        };
        // Weapon range is measured between the units' boxes, not their
        // centres (OpenBW `units_distance`): a melee weapon (SCV 10 px,
        // Zergling / Zealot 15 px) reaches a target whose box it touches.
        // Translation batch 21 (`0x005a0836`): maximum range plus the
        // `CUnit+0x105` bit-1 gate, in whole pixels.
        let distance = self.units_distance_px(i, tidx);
        // Translation batch 23 (`0x005a4835` / `0x005a485f`): the matching
        // ground/air weapon must clear its `weapons.dat` minimum range.
        let min_ok = if self.is_air_unit(tidx) {
            self.a23_air_weapon_min_range_ok(&self.units[i], distance)
        } else {
            self.a23_ground_weapon_min_range_ok(&self.units[i], distance)
        };
        let max_range = self.weapon_max_range_px(i, tidx, &weapon);
        if !self.units[i].a21_attack_range_max_gate(distance, max_range) || !min_ok {
            return false;
        }
        if self.units[i].status.disruption_ticks > 0 {
            return false;
        }
        // The attack animation supplies the release timing: the weapon fires
        // on its `attack`/`attackwith`/`attackmelee` opcode, not on the tick
        // the order is issued.
        // `gotorepeatattk` ends the swing: scripts that go on to an idle
        // loop instead of `ignorerest` (SCV, Zergling) may attack again.
        let attacking = self.units[i].anim.as_ref().is_some_and(|a| {
            !a.repeat_attack
                && matches!(
                    a.anim as usize,
                    ANIM_GND_ATTACK_INIT
                        | ANIM_AIR_ATTACK_INIT
                        | ANIM_GND_ATTACK_REPEAT
                        | ANIM_AIR_ATTACK_REPEAT
                )
        });
        if attacking {
            return true;
        }
        // A turret fires only once it points at the target, within the
        // weapon's attack angle (OpenBW `unit_target_in_attack_angle` on
        // the turret's heading).
        if self
            .data
            .turret(self.data.unit(self.units[i].type_id))
            .is_some()
        {
            let (dx, dy) = (
                self.units[tidx].x - self.units[i].x,
                self.units[tidx].y - self.units[i].y,
            );
            let want = direction_byte(dx, dy) as i32;
            let diff = (want - self.units[i].turret_facing as i32).rem_euclid(256);
            if diff.min(256 - diff) > weapon.attack_angle as i32 {
                return true;
            }
        }
        if self.units[i].cooldown == 0 {
            let flying = self.is_air_unit(tidx);
            if !self.start_attack_animation(i, flying) {
                self.fire(i, tidx, weapon_id, &weapon);
            }
        }
        true
    }

    pub(super) fn weapon_for(&self, attacker: usize, target: usize) -> Option<(u8, WeaponType)> {
        let a_ty = self.data.unit(self.units[attacker].type_id);
        if self.is_air_unit(target) {
            let id = self.data.air_weapon_id(a_ty);
            self.data.air_weapon(a_ty).cloned().map(|w| (id, w))
        } else {
            let id = self.data.ground_weapon_id(a_ty);
            self.data.ground_weapon(a_ty).cloned().map(|w| (id, w))
        }
    }

    pub(super) fn acquire_target(&self, i: usize) -> Option<u32> {
        let u = &self.units[i];
        let ty = self.data.unit(u.type_id);
        let mut acq_range = self.data.target_acquisition_px(ty);
        if is_carrier_reaver_family(u.type_id) {
            acq_range = acq_range.max(if u.type_id == uid::CARRIER {
                8 * 32
            } else {
                10 * 32
            });
        }
        if acq_range == 0 {
            return None;
        }
        // OpenBW `unit_target_acquisition_range`: a cloaked Ghost does not
        // pick targets on its own unless it is holding position.
        if is_ghost_type(u.type_id) && u.status.cloaked && !matches!(u.order, Order::HoldPosition) {
            return None;
        }
        let max_fp = acq_range * fp(1);
        let mut best: Option<(i32, u32)> = None;
        for (other_idx, other) in self.units.iter().enumerate() {
            if !other.alive || !self.hostile(u.owner, other.owner) {
                continue;
            }
            if u.in_transport == Some(other.id) {
                continue;
            }
            let t_ty = self.data.unit(other.type_id);
            if is_resource_type(other.type_id)
                || other.type_id == uid::START_LOCATION
                || is_door_type(other.type_id)
            {
                continue;
            }
            if !self.can_see_unit(u.owner, other_idx) {
                continue;
            }
            let Some(w) = (if self.is_air_unit(other_idx) {
                self.data.air_weapon(ty)
            } else {
                self.data.ground_weapon(ty)
            }) else {
                if !is_carrier_reaver_family(u.type_id) {
                    continue;
                }
                // Carriers and Reavers attack through their hangar units.
                if u.hangar == 0 {
                    continue;
                }
                let d = self.distance_ids(i, other.id);
                if d <= max_fp && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, other.id));
                }
                continue;
            };
            if !weapon_can_target(w, t_ty) {
                continue;
            }
            // Box-to-box distance, and the weapon's range with its upgrade
            // (U-238 Shells, ...), as the attack itself measures it.
            let d = self.units_distance_px(i, other_idx) * fp(1);
            let limit = max_fp.max(self.weapon_max_range_px(i, other_idx, w) * fp(1));
            // A target inside the weapon's minimum range (the sieged tank's
            // 64 px) is not a candidate.
            if d < w.min_range as i32 * fp(1) {
                continue;
            }
            if d <= limit && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, other.id));
            }
        }
        best.map(|(_, id)| id)
    }

    pub(super) fn distance_ids(&self, a: usize, b: u32) -> i32 {
        let ui = &self.units[a];
        let Some(bi) = self.index_of(b) else {
            return i32::MAX;
        };
        let uj = &self.units[bi];
        let dx = (ui.x - uj.x) as i64;
        let dy = (ui.y - uj.y) as i64;
        isqrt(dx * dx + dy * dy) as i32
    }

    /// Range upgrades (OpenBW `weapon_max_range`): U-238 Shells +32 for
    /// Marines, Grooved Spines +32 for Hydralisks, Singularity Charge +64
    /// for Dragoons, Charon Boosters +96 for the Goliath's air weapon.  The
    /// hero versions (Fenix Dragoon, Alan Schezar) always have theirs.
    pub fn weapon_range_bonus_px(&self, i: usize, target: usize) -> i32 {
        let u = &self.units[i];
        let has = |id: u8| {
            self.players
                .get(u.owner as usize)
                .is_some_and(|p| p.upgrade_level(id) > 0)
        };
        match u.type_id {
            uid::MARINE if has(upgrade::U238_SHELLS) => 32,
            uid::HYDRALISK if has(upgrade::GROOVED_SPINES) => 32,
            uid::DRAGOON if has(upgrade::SINGULARITY_CHARGE) => 64,
            // 75 = Fenix (Dragoon).
            75 => 64,
            uid::GOLIATH | uid::GOLIATH_TURRET
                if self.is_air_unit(target) && has(upgrade::CHARON_BOOSTERS) =>
            {
                96
            }
            // 17 / 18 = Alan Schezar (Goliath) and his turret.
            17 | 18 if self.is_air_unit(target) => 96,
            _ => 0,
        }
    }

    /// Maximum range in pixels of `weapon` fired by unit `i` at `target`.
    pub fn weapon_max_range_px(&self, i: usize, target: usize, weapon: &WeaponType) -> i32 {
        weapon.max_range as i32 + self.weapon_range_bonus_px(i, target)
    }

    /// Box-to-box distance in pixels from a Bunker occupant (standing at
    /// the Bunker's centre `(bx, by)`, 1/256 px) to `target`.
    pub(super) fn bunker_distance_px(&self, o: usize, target: usize, bx: i32, by: i32) -> i32 {
        let a0 = self
            .data
            .unit(self.units[o].type_id)
            .dimensions
            .rect_at((bx / fp(1), by / fp(1)));
        let t = &self.units[target];
        let b0 = self
            .data
            .unit(t.type_id)
            .dimensions
            .rect_at((t.x / fp(1), t.y / fp(1)));
        let dx = (b0.0 - a0.2).max(a0.0 - b0.2).max(0) as i64;
        let dy = (b0.1 - a0.3).max(a0.1 - b0.3).max(0) as i64;
        isqrt(dx * dx + dy * dy) as i32
    }

    /// Distance in pixels between the `units.dat` boxes of two units (0 when
    /// they touch or overlap), as retail measures weapon range.
    pub fn units_distance_px(&self, a: usize, b: usize) -> i32 {
        let bx = |i: usize| {
            let u = &self.units[i];
            self.data
                .unit(u.type_id)
                .dimensions
                .rect_at((u.x / fp(1), u.y / fp(1)))
        };
        let (a0, b0) = (bx(a), bx(b));
        let dx = (b0.0 - a0.2).max(a0.0 - b0.2).max(0) as i64;
        let dy = (b0.1 - a0.3).max(a0.1 - b0.3).max(0) as i64;
        isqrt(dx * dx + dy * dy) as i32
    }

    pub(super) fn distance(&self, a: usize, b: usize) -> i32 {
        let dx = (self.units[a].x - self.units[b].x) as i64;
        let dy = (self.units[a].y - self.units[b].y) as i64;
        isqrt(dx * dx + dy * dy) as i32
    }

    pub(super) fn fire(
        &mut self,
        attacker: usize,
        target: usize,
        weapon_id: u8,
        weapon: &WeaponType,
    ) {
        // Translation batch 01 (`0x005a0720` / `0x005a077a`): a damage
        // factor of 2 runs the effect dispatcher twice.
        let shots = a01_weapon_effect_repetitions(weapon);
        let owner = self.units[attacker].owner;
        if self.units[attacker].status.hallucination {
            // Hallucinations play the attack animation and cooldown, but do
            // no damage.
            self.units[attacker].cooldown = self.modified_cooldown(attacker, weapon);
            return;
        }
        if weapon.bullet_type == behavior::SELF_DESTRUCT {
            self.kill_unit(attacker);
            return;
        }
        if Projectile::weapon_is_travelling(weapon) {
            for _ in 0..shots {
                self.spawn_projectile(attacker, target, weapon_id, weapon);
            }
        } else {
            let missed = weapon.bullet_type == behavior::APPEAR_AT_TARGET_UNIT
                && self.target_dodges(attacker, target);
            if !missed {
                let target_id = self.units[target].id;
                let pos = (self.units[target].x, self.units[target].y);
                for _ in 0..shots {
                    self.instant_hit(owner, Some(target_id), attacker, weapon_id, weapon, pos);
                }
            }
        }
        self.units[attacker].cooldown = self.modified_cooldown(attacker, weapon);
    }

    /// Frames between an instant weapon firing and its damage: the bullet
    /// image (weapon -> flingy -> sprite -> image) runs its Init script and
    /// deals the damage on `domissiledmg`.  The Gauss Rifle, C-10 and Fusion
    /// Cutter sparks take 2 frames, the Flame Thrower's bullet 7.
    pub fn instant_hit_delay(&self, weapon: &WeaponType) -> u32 {
        u16::try_from(weapon.flingy)
            .ok()
            .and_then(|f| self.data.flingy(f))
            .and_then(|f| self.data.sprite(f.sprite))
            .and_then(|s| self.data.image(s.image))
            .and_then(|i| self.data.iscript.frames_until_damage(i.iscript_id as u16))
            .unwrap_or(0)
    }

    /// Deal an instant weapon's hit when its bullet script reaches
    /// `domissiledmg`: now when there is no delay, else queued.
    pub(super) fn instant_hit(
        &mut self,
        owner: u8,
        target: Option<u32>,
        source: usize,
        weapon_id: u8,
        weapon: &WeaponType,
        pos: (i32, i32),
    ) {
        let delay = self.instant_hit_delay(weapon);
        if delay == 0 {
            self.resolve_hit_type(owner, target, source, weapon_id, weapon, pos, 1, 1);
            return;
        }
        self.pending_hits.push(PendingHit {
            due: self.tick + delay as u64,
            owner,
            source: self.units[source].id,
            target,
            weapon: weapon_id,
            pos,
        });
    }

    /// Land the queued instant hits whose time has come.
    pub(super) fn update_pending_hits(&mut self) {
        if self.pending_hits.is_empty() {
            return;
        }
        let tick = self.tick;
        let (due, later): (Vec<PendingHit>, Vec<PendingHit>) =
            std::mem::take(&mut self.pending_hits)
                .into_iter()
                .partition(|h| h.due <= tick);
        self.pending_hits = later;
        for h in due {
            // The shooter may have died since; its bullet still lands.
            let Some(source) = self.index_of(h.source) else {
                continue;
            };
            let Some(weapon) = self.data.weapon(h.weapon).cloned() else {
                continue;
            };
            // An "appear on target" bullet follows its target; the others
            // (the flame) stay where they appeared.
            let pos = match h.target.and_then(|t| self.index_of(t)) {
                Some(t) if weapon.bullet_type == behavior::APPEAR_AT_TARGET_UNIT => {
                    (self.units[t].x, self.units[t].y)
                }
                _ => h.pos,
            };
            self.resolve_hit_type(h.owner, h.target, source, h.weapon, &weapon, pos, 1, 1);
        }
    }

    /// Cooldown modifiers recovered from the retail `get_modified_weapon_cooldown`
    /// helper: acid spores lengthen it, Stim/cooldown upgrades halve it and
    /// Ensnare adds 25%.  The result is clamped to `[5, 250]`.
    pub(super) fn modified_cooldown(&self, attacker: usize, weapon: &WeaponType) -> u32 {
        let u = &self.units[attacker];
        let mut cooldown = weapon.cooldown as i32;
        if u.status.acid_spores > 0 {
            cooldown += (cooldown / 8).max(3) * u.status.acid_spores as i32;
        }
        let mut mods = 0;
        if u.status.stim_ticks > 0 {
            mods += 1;
        }
        // OpenBW `update_unit_speed_upgrades`: Adrenal Glands for the
        // Zergling; the Devouring One (54) always has it.
        if self.data.unit(u.type_id).flags & 0x2000_0000 != 0
            || u.type_id == 54
            || (u.type_id == uid::ZERGLING
                && self
                    .players
                    .get(u.owner as usize)
                    .is_some_and(|p| p.upgrade_level(upgrade::ADRENAL_GLANDS) > 0))
        {
            mods += 1;
        }
        if u.status.ensnare_ticks > 0 {
            mods -= 1;
        }
        if mods > 0 {
            cooldown /= 2;
        } else if mods < 0 {
            cooldown += cooldown / 4;
        }
        cooldown = cooldown.clamp(5, 250);
        cooldown as u32
    }

    pub fn world_rand(&mut self) -> u32 {
        // 32-bit LCG used by the retail engine for miss rolls and scatter.
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.rng >> 32) as u32
    }

    /// Miss chance in 1/256 units for one ranged attack.  Retail uses
    /// 255/256 against ground units inside Dark Swarm and 119/256 when the
    /// target stands on higher terrain.  Melee attacks (`fire_melee`) never
    /// consult this, and air units ignore both rules.
    pub(super) fn dodge_chance(&self, attacker: usize, target: usize) -> u32 {
        let a = &self.units[attacker];
        let t = &self.units[target];
        if !self.is_air_unit(target) && self.is_under_dark_swarm(target) {
            255
        } else if !self.is_air_unit(attacker) && !self.is_air_unit(target) {
            let ah = self.terrain.elevation_px(a.x / fp(1), a.y / fp(1));
            let th = self.terrain.elevation_px(t.x / fp(1), t.y / fp(1));
            if th > ah { 119 } else { 0 }
        } else {
            0
        }
    }

    pub(super) fn target_dodges(&mut self, attacker: usize, target: usize) -> bool {
        let chance = self.dodge_chance(attacker, target);
        dodge_roll(self.world_rand(), chance)
    }

    /// An idle unit that takes damage attacks its attacker when one of its
    /// weapons can hit it; idle units of a computer player near the victim
    /// join in.  FACT (Wasteland, 2026-11-05): without this, Zerglings shot
    /// by Marines (4-tile range against their 3-tile acquisition) stayed idle
    /// until dead.  INFERENCE: the helper radius (8 tiles) and that only
    /// computer players call for help; falsifier: a retail recording of a
    /// human player's idle unit next to an attacked one.
    pub(super) fn retaliate(&mut self, victim: usize, source: u32) {
        let Some(a) = self.index_of(source) else {
            return;
        };
        let (owner, attacker_owner) = (self.units[victim].owner, self.units[a].owner);
        if !self.units[a].alive || !self.hostile(owner, attacker_owner) {
            return;
        }
        let air = self.is_air_unit(a);
        let attacker_ty = self.data.unit(self.units[a].type_id).clone();
        let can_hit = |w: &World, i: usize| {
            let ty = w.data.unit(w.units[i].type_id);
            // (Spider Mines run their own state machine.)
            if ty.is_worker()
                || w.units[i].type_id == uid::SPIDER_MINE
                || !matches!(w.units[i].order, Order::Idle | Order::Stop)
            {
                return false;
            }
            let weapon = if air {
                w.data.air_weapon(ty)
            } else {
                w.data.ground_weapon(ty)
            };
            weapon.is_some_and(|wt| weapon_can_target(wt, &attacker_ty))
        };
        let mut responders = Vec::new();
        if can_hit(self, victim) {
            responders.push(victim);
        }
        if self
            .players
            .get(owner as usize)
            .is_some_and(|p| p.is_computer)
        {
            let r = (8 * 32 * fp(1)) as i64;
            let (vx, vy) = (self.units[victim].x as i64, self.units[victim].y as i64);
            for j in 0..self.units.len() {
                if j != victim
                    && self.units[j].alive
                    && self.units[j].owner == owner
                    && (self.units[j].x as i64 - vx).pow(2) + (self.units[j].y as i64 - vy).pow(2)
                        <= r * r
                    && !self.data.unit(self.units[j].type_id).is_building()
                    && can_hit(self, j)
                {
                    responders.push(j);
                }
            }
        }
        for j in responders {
            self.units[j].order = Order::AttackUnit { target: source };
            self.units[j].order_row = self.units[j].order.order_id();
        }
    }

    pub(super) fn fire_melee(
        &mut self,
        attacker: usize,
        target: usize,
        weapon_id: u8,
        weapon: &WeaponType,
    ) {
        if self.units[attacker].status.hallucination {
            self.units[attacker].cooldown = self.modified_cooldown(attacker, weapon);
            return;
        }
        let owner = self.units[attacker].owner;
        let pos = (self.units[target].x, self.units[target].y);
        self.resolve_hit_type(
            owner,
            Some(self.units[target].id),
            attacker,
            weapon_id,
            weapon,
            pos,
            1,
            1,
        );
        self.units[attacker].cooldown = self.modified_cooldown(attacker, weapon);
    }

    pub(super) fn special_attack(&mut self, attacker: usize, target: usize) -> bool {
        let ty = self.units[attacker].type_id;
        let dist = self.distance(attacker, target);
        match ty {
            uid::CARRIER => {
                if self.units[attacker].special_cooldown > 0 {
                    return true;
                }
                if self.units[attacker].hangar == 0 || dist > 8 * 32 * fp(1) {
                    return false;
                }
                if self.units[attacker].cooldown == 0 {
                    let w = self.data.weapon(wid::PULSE_CANNON).cloned();
                    if let Some(w) = w {
                        self.spawn_projectile(attacker, target, wid::PULSE_CANNON, &w);
                        self.units[attacker].cooldown = 9;
                    }
                }
                true
            }
            uid::REAVER => {
                if self.units[attacker].special_cooldown > 0 {
                    return true;
                }
                if self.units[attacker].hangar == 0 || dist > 10 * 32 * fp(1) {
                    return false;
                }
                if self.units[attacker].cooldown == 0 {
                    let w = self.data.weapon(wid::SCARAB).cloned();
                    if let Some(w) = w {
                        let before = self.projectiles.len();
                        self.spawn_projectile(attacker, target, wid::SCARAB, &w);
                        if self.projectiles.len() > before {
                            self.units[attacker].hangar -= 1;
                            self.units[attacker].special_cooldown = 0x3c;
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// Infantry inside a Bunker fire from the building's position.  Each
    /// occupant keeps its own weapon, cooldown and upgrades; the bunker only
    /// supplies the location and protection.
    pub(super) fn bunker_fire(&mut self) {
        for b in 0..self.units.len() {
            if !self.units[b].alive
                || !self.units[b].completed
                || self.units[b].type_id != uid::BUNKER
                || self.units[b].cargo.is_empty()
            {
                continue;
            }
            let (bx, by) = (self.units[b].x, self.units[b].y);
            let cargo = self.units[b].cargo.clone();
            for id in cargo {
                let Some(o) = self.index_of(id) else {
                    continue;
                };
                if self.units[o].hp <= 0 {
                    continue;
                }
                // An SCV sheltering in the Bunker does not fire.
                if self.data.unit(self.units[o].type_id).is_worker() {
                    continue;
                }
                // `0x5b5aa0`: disabled occupants do not enter Bunker Guard.
                if self.units[o].status.lockdown_ticks > 0
                    || self.units[o].status.stasis_ticks > 0
                    || self.units[o].status.maelstrom_ticks > 0
                {
                    continue;
                }
                if self.units[o].order_wait > 0 {
                    self.units[o].order_wait -= 1;
                }
                // `CUnit+0x7c`: keep the current target while it is legal.
                if let Some(t) = self.units[o].target
                    && !self.bunker_target_valid(o, t)
                {
                    self.units[o].target = None;
                }
                if let Some(t) = self.units[o].target {
                    self.bunker_fire_at(o, t, bx, by);
                    continue;
                }
                // `0x5a0581`: still waiting out the 15-tick retry.
                if self.units[o].order_wait != 0 {
                    continue;
                }
                self.units[o].order_wait = BUNKER_GUARD_RETRY_TICKS;
                let owner = self.units[o].owner;
                let mut best: Option<(i32, u32)> = None;
                for j in 0..self.units.len() {
                    if !self.units[j].alive
                        || !self.hostile(owner, self.units[j].owner)
                        || self.units[j].invincible
                        || self.units[j].status.stasis_ticks > 0
                    {
                        continue;
                    }
                    let ty = self.data.unit(self.units[j].type_id);
                    let Some((_, w)) = self.weapon_for(o, j) else {
                        continue;
                    };
                    if !weapon_can_target(&w, ty) {
                        continue;
                    }
                    let dist = self.bunker_distance_px(o, j, bx, by);
                    let max_range =
                        self.weapon_max_range_px(o, j, &w) + BUNKER_GUARD_RANGE_BONUS_PX;
                    // The nearest legal target, not the last one scanned.
                    if dist <= max_range
                        && dist >= w.min_range as i32
                        && best.is_none_or(|(bd, _)| dist < bd)
                    {
                        best = Some((dist, self.units[j].id));
                    }
                }
                if let Some((_, t)) = best {
                    self.units[o].target = Some(t);
                    self.bunker_fire_at(o, t, bx, by);
                }
            }
        }
    }

    pub(super) fn bunker_target_valid(&self, o: usize, target: u32) -> bool {
        let owner = self.units[o].owner;
        self.index_of(target).is_some_and(|ti| {
            let t = &self.units[ti];
            t.alive
                && t.hp > 0
                && !t.invincible
                && t.status.stasis_ticks == 0
                && self.hostile(owner, t.owner)
                && self.can_see_unit(owner, ti)
                && self
                    .weapon_for(o, ti)
                    .is_some_and(|(_, w)| weapon_can_target(&w, self.data.unit(t.type_id)))
        })
    }

    pub(super) fn bunker_fire_at(&mut self, o: usize, target: u32, bx: i32, by: i32) {
        if self.units[o].cooldown > 0 || self.units[o].status.disruption_ticks > 0 {
            return;
        }
        let Some(ti) = self.index_of(target) else {
            return;
        };
        if !self.units[ti].alive {
            return;
        }
        let Some((wid, w)) = self.weapon_for(o, ti) else {
            return;
        };
        if !weapon_can_target(&w, self.data.unit(self.units[ti].type_id)) {
            return;
        }
        let dist = self.bunker_distance_px(o, ti, bx, by);
        let max_range = self.weapon_max_range_px(o, ti, &w) + BUNKER_GUARD_RANGE_BONUS_PX;
        if dist > max_range || dist < w.min_range as i32 {
            return;
        }
        let (dx, dy) = (self.units[ti].x - bx, self.units[ti].y - by);
        self.units[o].facing = direction_byte(dx, dy);
        self.fire(o, ti, wid, &w);
    }
}
