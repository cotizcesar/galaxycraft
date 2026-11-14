//! `impl World`: damage (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

impl World {
    /// Apply a weapon's `hit_type` at `pos`.  `target` is the intended unit
    /// (used by normal hits and unit-targeted spells); `divisor` is the splash
    /// ring divisor.
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_hit_type(
        &mut self,
        owner: u8,
        target: Option<u32>,
        source: usize,
        weapon_id: u8,
        weapon: &WeaponType,
        pos: (i32, i32),
        divisor: u32,
        bullet_count: u32,
    ) {
        let _ = bullet_count;
        let target_idx = target.and_then(|id| self.index_of(id));
        match weapon.explosion {
            hit::NONE => {}
            hit::NORMAL | hit::YAMATO | hit::UNUSED_23 => {
                if let Some(t) = target_idx
                    && self.can_see_unit(owner, t)
                {
                    self.apply_hit_from(owner, source, t, weapon, divisor);
                }
            }
            hit::RADIAL_SPLASH | hit::ENEMY_SPLASH | hit::AIR_SPLASH => {
                self.apply_splash(
                    owner,
                    source,
                    weapon_id,
                    weapon,
                    target,
                    pos,
                    matches!(weapon.explosion, hit::ENEMY_SPLASH | hit::AIR_SPLASH),
                    weapon.explosion == hit::AIR_SPLASH,
                );
            }
            hit::NUCLEAR => {
                self.apply_nuke(owner, source, pos);
            }
            hit::LOCKDOWN => {
                if let Some(t) = target_idx {
                    self.units[t].status.apply_lockdown(LOCKDOWN_UPDATES);
                    let id = self.units[t].id;
                    self.push_spell_cue(SpellCueKind::LockdownHit, id);
                }
            }
            hit::PARASITE => {
                if let Some(t) = target_idx {
                    self.units[t].status.parasite = true;
                    self.units[t].status.revealed_ticks = 1200;
                }
            }
            hit::BROODLINGS => {
                if let Some(t) = target_idx
                    && self.data.unit(self.units[t].type_id).is_organic()
                {
                    let (x, y) = (self.units[t].x / fp(1), self.units[t].y / fp(1));
                    self.kill_unit(t);
                    self.spawn_broodling(owner as usize, x, y);
                    self.spawn_broodling(owner as usize, x + 16, y);
                }
            }
            hit::EMP => {
                // OpenBW `emp_shockwave`: every unit whose box touches the
                // square of the weapon's inner splash radius (64 px) around
                // the impact loses its energy and shields, except the
                // caster and units in stasis; hallucinations die.
                let half = weapon.inner_splash as i32;
                let (ix, iy) = (pos.0 / fp(1), pos.1 / fp(1));
                for j in 0..self.units.len() {
                    if !self.units[j].alive || j == source {
                        continue;
                    }
                    let rect = self
                        .data
                        .unit(self.units[j].type_id)
                        .dimensions
                        .rect_at((self.units[j].x / fp(1), self.units[j].y / fp(1)));
                    if rect.2 < ix - half
                        || rect.0 > ix + half
                        || rect.3 < iy - half
                        || rect.1 > iy + half
                    {
                        continue;
                    }
                    if self.units[j].status.hallucination {
                        self.kill_unit(j);
                        continue;
                    }
                    if self.units[j].status.stasis_ticks > 0 {
                        continue;
                    }
                    self.units[j].shields = 0;
                    self.units[j].set_energy_fixed(0);
                }
            }
            hit::IRRADIATE => {
                if let Some(t) = target_idx
                    && !self.data.unit(self.units[t].type_id).is_building()
                {
                    // Any non-building unit can carry the aura; only
                    // biological units in range take damage.
                    self.units[t].status.apply_irradiate(IRRADIATE_UPDATES);
                }
            }
            hit::ENSNARE => {
                for j in 0..self.units.len() {
                    if j == source
                        || !self.units[j].alive
                        || self.units[j].in_transport.is_some()
                        || self.units[j].status.burrowed
                        || self.data.unit(self.units[j].type_id).is_building()
                        || self.distance_to_point(j, pos) > fp(64)
                    {
                        continue;
                    }
                    self.units[j].status.apply_ensnare();
                }
            }
            hit::PLAGUE => {
                for j in 0..self.units.len() {
                    if self.units[j].alive
                        && !self.units[j].status.burrowed
                        && self.distance_to_point(j, pos) <= fp(64)
                    {
                        self.units[j].status.apply_plague();
                    }
                }
            }
            hit::STASIS => {
                for j in 0..self.units.len() {
                    if self.units[j].alive
                        && !self.units[j].status.burrowed
                        && !self.data.unit(self.units[j].type_id).is_building()
                        && self.distance_to_point(j, pos) <= fp(64)
                    {
                        self.units[j].status.apply_stasis(STASIS_UPDATES);
                    }
                }
            }
            hit::DARK_SWARM => {
                self.spawn_dark_swarm(owner, pos);
            }
            hit::CONSUME => {
                if let Some(t) = target_idx
                    && t != source
                {
                    let gained =
                        self.data.unit(self.units[t].type_id).hit_points_raw as i32 / fp(1) * 2;
                    if source < self.units.len() {
                        self.units[source].add_energy_points(gained);
                    }
                    self.kill_unit(t);
                }
            }
            hit::RESTORATION => {
                if let Some(t) = target_idx {
                    self.restore_unit(t);
                }
            }
            hit::DISRUPTION_WEB => {
                let _ = self.spawn_temporary(owner, uid::DISRUPTION_WEB, pos, 360);
            }
            hit::CORROSIVE_ACID => {
                if let Some(t) = target_idx {
                    self.apply_hit_from(owner, source, t, weapon, divisor);
                }
                // `FUN_005dc1c0` asks `FUN_006c1ee0` for every unit in a
                // 128 x 128 px box (`0x40` half-extent) and calls
                // `FUN_005dc360` on it: the damage is single-target, but the
                // spore debuff is an area effect.  The callback skips
                // buildings, loaded units and cloaked units the attacker
                // cannot see.
                let mut spored = Vec::new();
                for j in 0..self.units.len() {
                    let u = &self.units[j];
                    if !u.alive || u.owner == owner || u.in_transport.is_some() {
                        continue;
                    }
                    if self.data.unit(u.type_id).is_building() {
                        continue;
                    }
                    if (u.x - pos.0).abs() > fp(64) || (u.y - pos.1).abs() > fp(64) {
                        continue;
                    }
                    if u.status.cloaked && !self.can_see_unit(owner, j) {
                        continue;
                    }
                    spored.push(j);
                }
                for j in spored {
                    self.units[j].status.add_acid_spore();
                }
            }
            hit::MIND_CONTROL => {
                if let Some(t) = target_idx {
                    self.units[t].owner = owner;
                }
            }
            hit::FEEDBACK => {
                if let Some(t) = target_idx {
                    self.feedback_damage(t, source as u32);
                }
            }
            hit::OPTICAL_FLARE => {
                if let Some(t) = target_idx {
                    self.blind_unit(t);
                }
            }
            hit::MAELSTROM => {
                for j in 0..self.units.len() {
                    if self.units[j].alive
                        && !self.units[j].status.burrowed
                        && self.distance_to_point(j, pos) <= fp(64)
                        && self.data.unit(self.units[j].type_id).is_organic()
                        && !self.data.unit(self.units[j].type_id).is_building()
                    {
                        self.units[j].status.apply_maelstrom(MAELSTROM_UPDATES);
                    }
                }
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_splash(
        &mut self,
        owner: u8,
        source: usize,
        weapon_id: u8,
        weapon: &WeaponType,
        target: Option<u32>,
        impact: (i32, i32),
        enemy_only: bool,
        air_only: bool,
    ) {
        let inner = weapon.inner_splash as i64 * fp(1) as i64;
        let middle = weapon.middle_splash as i64 * fp(1) as i64;
        let outer = weapon.outer_splash as i64 * fp(1) as i64;
        if air_only {
            self.apply_air_splash(owner, source, weapon, target, impact);
            return;
        }
        for j in 0..self.units.len() {
            if !self.units[j].alive {
                continue;
            }
            // `0x00567aa0`: the bullet's source is never a splash victim,
            // except weapon 0x54 (Psionic Storm), which may hit its caster.
            if j == source && weapon_id != wid::PSIONIC_STORM {
                continue;
            }
            // `0x005b4c20`/`0x005b67d0`: a Psionic Storm victim can only be
            // hit once per damage cadence.
            if weapon_id == wid::PSIONIC_STORM && self.units[j].status.psi_storm_guard != 0 {
                continue;
            }
            let air = self.is_air_unit(j);
            if air_only && !air {
                continue;
            }
            if enemy_only && self.units[j].owner == owner && Some(self.units[j].id) != target {
                continue;
            }
            // Target flags apply to every splash victim, not just the
            // acquired target: ground-only Shock Cannon splash must not hit
            // air and Psionic Storm's 0x10 bit must not hit buildings.
            let victim_ty = self.data.unit(self.units[j].type_id).clone();
            if !weapon_can_target_unit(weapon, &victim_ty, air) {
                continue;
            }
            // Distance from the impact point to the victim's box, not to its
            // centre (OpenBW `unit_distance_to`): a large unit is hit when
            // any part of it is inside the ring.
            let rect = victim_ty
                .dimensions
                .rect_at((self.units[j].x / fp(1), self.units[j].y / fp(1)));
            let (ix, iy) = (impact.0 / fp(1), impact.1 / fp(1));
            let d = {
                let dx = (rect.0 - ix).max(ix - rect.2).max(0) as i64 * fp(1) as i64;
                let dy = (rect.1 - iy).max(iy - rect.3).max(0) as i64 * fp(1) as i64;
                isqrt(dx * dx + dy * dy)
            };
            if d > outer {
                continue;
            }
            // `FUN_00576ce0` (hit types 2/3/24): burrowed units only take
            // the immediate/inner ring; middle and outer miss them.
            if self.units[j].status.burrowed && d > inner {
                continue;
            }
            let divisor = if d <= inner {
                1
            } else if d <= middle {
                2
            } else {
                4
            };
            if weapon_id == wid::PSIONIC_STORM {
                self.units[j].status.psi_storm_guard = 1;
            }
            self.apply_hit_from(owner, source, j, weapon, divisor);
        }
    }

    /// Air splash (Halo Rockets; OpenBW `bullet_deal_splash_damage` with
    /// `is_air_splash`).  One unit takes the full hit: the bullet's target
    /// if the rocket landed within the inner radius of it, else one valid
    /// air unit in the outer radius picked at random.  Every other valid
    /// unit in reach, except the bullet's target and Interceptors, takes
    /// half within the middle radius and a quarter within the outer one.
    /// The shooter's own units are spared unless they are the target.
    pub(super) fn apply_air_splash(
        &mut self,
        owner: u8,
        source: usize,
        weapon: &WeaponType,
        target: Option<u32>,
        impact: (i32, i32),
    ) {
        let (inner, middle, outer) = (
            weapon.inner_splash as i32,
            weapon.middle_splash as i32,
            weapon.outer_splash as i32,
        );
        let (ix, iy) = (impact.0 / fp(1), impact.1 / fp(1));
        // (unit, distance from the impact to its box)
        let mut reach: Vec<(usize, i32)> = Vec::new();
        for j in 0..self.units.len() {
            let u = &self.units[j];
            if !u.alive || j == source || !self.is_air_unit(j) {
                continue;
            }
            if u.owner == owner && Some(u.id) != target {
                continue;
            }
            let ty = self.data.unit(u.type_id);
            if !weapon_can_target_unit(weapon, ty, true) {
                continue;
            }
            let rect = ty.dimensions.rect_at((u.x / fp(1), u.y / fp(1)));
            let dx = (rect.0 - ix).max(ix - rect.2).max(0) as i64;
            let dy = (rect.1 - iy).max(iy - rect.3).max(0) as i64;
            let d = isqrt(dx * dx + dy * dy) as i32;
            if d <= outer {
                reach.push((j, d));
            }
        }
        if reach.is_empty() {
            return;
        }
        let on_target = reach
            .iter()
            .find(|(j, d)| Some(self.units[*j].id) == target && *d <= inner)
            .map(|(j, _)| *j);
        let main = on_target.unwrap_or_else(|| {
            if reach.len() == 1 {
                reach[0].0
            } else {
                reach[self.world_rand() as usize % reach.len()].0
            }
        });
        self.apply_hit_from(owner, source, main, weapon, 1);
        for (j, d) in reach {
            let u = &self.units[j];
            if !u.alive
                || Some(u.id) == target
                || u.type_id == uid::INTERCEPTOR
                || u.status.burrowed
            {
                continue;
            }
            let divisor = if d <= middle { 2 } else { 4 };
            self.apply_hit_from(owner, source, j, weapon, divisor);
        }
    }

    /// The retail nuclear missile does `max(500, 2/3 * max_hp_plus_shields)`
    /// damage to every unit in its splash radius, ignoring the weapon's stored
    /// per-shot damage.
    pub(super) fn apply_nuke(&mut self, owner: u8, source: usize, impact: (i32, i32)) {
        let Some(mut nuke) = self.data.weapon(wid::NUCLEAR_STRIKE).cloned() else {
            return;
        };
        // weapons.dat row 31: full damage within 128 px of the victim's
        // box, half within 192, a quarter within 256 (the splash rings of
        // every radial weapon).
        let (inner, middle, outer) = (
            nuke.inner_splash as i32,
            nuke.middle_splash as i32,
            nuke.outer_splash as i32,
        );
        let (ix, iy) = (impact.0 / fp(1), impact.1 / fp(1));
        let mut targets = Vec::new();
        for j in 0..self.units.len() {
            if !self.units[j].alive {
                continue;
            }
            let rect = self
                .data
                .unit(self.units[j].type_id)
                .dimensions
                .rect_at((self.units[j].x / fp(1), self.units[j].y / fp(1)));
            let dx = (rect.0 - ix).max(ix - rect.2).max(0) as i64;
            let dy = (rect.1 - iy).max(iy - rect.3).max(0) as i64;
            let d = isqrt(dx * dx + dy * dy) as i32;
            let divisor = if d <= inner {
                1
            } else if d <= middle {
                2
            } else if d <= outer {
                4
            } else {
                continue;
            };
            targets.push((j, divisor));
        }
        nuke.damage_bonus = 0;
        for (j, divisor) in targets {
            let ty = self.data.unit(self.units[j].type_id);
            let max = ty.hit_points_raw as i32
                + if ty.has_shields {
                    fp(ty.shield_points as i32)
                } else {
                    0
                };
            let damage = (max * 2 / 3).max(fp(500));
            nuke.damage = (damage / fp(1)).max(1) as u16;
            self.apply_hit_from(owner, source, j, &nuke, divisor);
        }
    }

    pub(super) fn apply_hit(
        &mut self,
        attacker: usize,
        target: usize,
        weapon: &WeaponType,
        divisor: u32,
    ) {
        let owner = self.units.get(attacker).map(|u| u.owner).unwrap_or(0);
        self.apply_hit_from(owner, attacker, target, weapon, divisor);
    }

    pub(super) fn apply_hit_from(
        &mut self,
        owner: u8,
        _source: usize,
        target: usize,
        weapon: &WeaponType,
        divisor: u32,
    ) {
        if owner as usize >= self.players.len() || !self.units.get(target).is_some_and(|u| u.alive)
        {
            return;
        }
        let t_owner = self.units[target].owner;
        if t_owner as usize >= self.players.len() {
            return;
        }
        let t_ty = self.data.unit(self.units[target].type_id).clone();
        let a_up = self.players[owner as usize].upgrade_level(weapon.damage_upgrade);
        let t_armor_up = self.players[t_owner as usize].upgrade_level(t_ty.armor_upgrade);
        let plating = crate::combat::chitinous_plating_bonus(
            self.units[target].type_id,
            t_ty.flags,
            self.players[t_owner as usize].upgrade_level(upgrade::CHITINOUS_PLATING) > 0,
        );
        let defense = Defense {
            size: t_ty.size,
            armor_level: t_ty.armor as u32 + t_armor_up as u32 + plating,
            has_shields: t_ty.has_shields,
            shield_points: self.units[target].shields,
            shield_upgrade: self.players[t_owner as usize]
                .upgrade_level(upgrade::PROTOSS_PLASMA_SHIELDS),
        };
        let status = StatusDamage {
            matrix: self.units[target].status.matrix,
            acid_spores: self.units[target].status.acid_spores,
            hallucination: self.units[target].status.hallucination,
        };
        let result = resolve_damage_status(weapon, a_up, defense, divisor, status);
        let matrix_was_active = self.units[target].status.matrix_active();
        if matrix_was_active {
            let (absorbed, broke) = self.units[target]
                .status
                .absorb_matrix(result.matrix_absorbed);
            let cue = MatrixCue {
                unit: self.units[target].id,
                x: self.units[target].x / fp(1),
                y: self.units[target].y / fp(1),
                absorbed,
                broke,
            };
            self.matrix_cues.push(cue);
        }
        if result.damage.shield_damage > 0 {
            self.units[target].shields =
                (self.units[target].shields - result.damage.shield_damage).max(0);
        }
        if result.damage.hp_damage > 0 {
            // `_source` is the attacker's index (`usize::MAX` once it died);
            // `last_damage_source` stores unit ids, which is what
            // `kill_unit` resolves to credit the kill.  Passing the index
            // credited kills to whichever unit had that id.
            let source_id = self.units.get(_source).map(|u| u.id).unwrap_or(0);
            self.damage_hp(target, result.damage.hp_damage, source_id);
        }
    }

    /// Apply raw hit-point damage to a unit (scripting/test entry point).
    pub fn damage_unit(&mut self, id: u32, amount: i32) -> Result<(), SimError> {
        let target = self.index_of(id).ok_or(SimError::BadUnit)?;
        self.damage_hp(target, amount, 0);
        Ok(())
    }

    /// Set a unit's plasma shields in whole points (scripting/test entry
    /// point for damage and Shield Battery scenarios).
    pub fn set_unit_shields(&mut self, id: u32, points: i32) -> Result<(), SimError> {
        let target = self.index_of(id).ok_or(SimError::BadUnit)?;
        self.units[target].shields = fp(points.max(0));
        Ok(())
    }

    /// Feedback drains every 1/256 energy point and deals exactly that many
    /// 1/256 HP of damage.  Defensive Matrix absorbs the damage first; shields
    /// and armour are bypassed, as in the retail spell.
    pub(super) fn feedback_damage(&mut self, target: usize, source: u32) {
        let energy = self.units[target].energy_fixed();
        if energy <= 0 {
            return;
        }
        self.units[target].set_energy_fixed(0);
        let matrix_was_active = self.units[target].status.matrix_active();
        let absorbed = if matrix_was_active {
            let (absorbed, broke) = self.units[target].status.absorb_matrix(energy);
            self.matrix_cues.push(MatrixCue {
                unit: self.units[target].id,
                x: self.units[target].x / fp(1),
                y: self.units[target].y / fp(1),
                absorbed,
                broke,
            });
            absorbed
        } else {
            0
        };
        let hp_damage = energy - absorbed;
        if hp_damage > 0 {
            self.damage_hp(target, hp_damage, source);
        }
    }

    pub(super) fn damage_hp(&mut self, target: usize, amount: i32, source: u32) {
        if !self.units[target].alive || self.units[target].invincible {
            return;
        }
        let target_owner = self.units[target].owner as usize;
        if self
            .players
            .get(target_owner)
            .is_some_and(|p| p.cheats.invincible)
        {
            return;
        }
        if source != 0 {
            self.units[target].last_damage_source = source;
        }
        self.units[target].hp -= amount;
        self.units[target].last_damaged = self.tick;
        if self.units[target].hp <= 0 {
            self.kill_unit(target);
        } else if source != 0 {
            self.retaliate(target, source);
        }
    }

    /// `0x005b2b70`: clear references to a dying unit from every other
    /// unit's target/order fields, plus the nuke link.  Rust stores the
    /// same links in typed orders instead of the retail `CUnit` offsets.
    pub(super) fn remove_target_references(&mut self, dead_id: u32) {
        let mut cleared_orders = Vec::new();
        for (i, u) in self.units.iter_mut().enumerate() {
            if u.target == Some(dead_id) {
                u.target = None;
            }
            if u.order_unit_target() == Some(dead_id) {
                cleared_orders.push(i);
            }
            if u.connected_unit == Some(dead_id) {
                u.connected_unit = None;
            }
            if u.rally.is_some() {
                // Rally points store only the ground position in this
                // engine; no unit id remains to clear.
            }
        }
        // `0x005a3cd0`: the order target is gone; finish the temporary
        // attack so `resume_order` (attack-move/patrol) can take over.
        for i in cleared_orders {
            self.order_done(i);
        }
        // Once the bomb is falling nothing calls it off.
        self.nuke_strikes
            .retain(|s| s.timer <= -NUKE_WARP_FRAMES || (s.ghost != dead_id && s.silo != dead_id));
    }

    /// `0x005b083a`: retail unit-death cleanup.  The Rust path marks the
    /// unit dead, clears target/nuke references, tears down add-ons/cargo,
    /// credits the kill and refreshes creep exactly once.
    pub(super) fn kill_unit(&mut self, i: usize) {
        if !self.units[i].alive {
            return;
        }
        self.units[i].alive = false;
        self.units[i].hp = 0;
        // Translation batch 10 (`0x005d22bf`): removal wipes acid-spore
        // stacks; the rest of the death status cleanup follows.
        self.units[i].a10_clear_acid_spores_if_present();
        self.units[i].status.clear_on_death();
        self.units[i].clear_production_queue();
        // `0x005b06aa`: a destroyed Nuclear Silo drops its armed missile
        // and the parent link (the generic add-on cleanup below removes the
        // parent's `addon` slot).
        if self.units[i].type_id == uid::NUCLEAR_SILO {
            self.units[i].nuke_ready = false;
        }
        // `0x005b6810` stores a 0x546 removal timer in CUnit+0x110.
        self.units[i].set_death_timer_min(HALLUCINATION_LIFETIME_TICKS as u16);
        let owner = self.units[i].owner as usize;
        // `0x005b6160`/`0x005da2e0`: a producer destroyed while upgrading
        // refunds 75% of the current level's mineral and gas cost.  An
        // active technology is only cancelled (`0x005da390`).
        if let Order::Upgrade { upgrade_id, .. } = self.units[i].order
            && let Some(up) = self.data.upgrade(upgrade_id)
            && owner < self.players.len()
        {
            let level = self.players[owner].upgrade_level(upgrade_id);
            let minerals = up.mineral_cost(level) as i32 * 3 / 4;
            let gas = up.gas_cost(level) as i32 * 3 / 4;
            self.players[owner].minerals += minerals;
            self.players[owner].gas += gas;
        }
        self.units[i].order = Order::Die;
        let type_id = self.units[i].type_id;
        let dead_id = self.units[i].id;
        self.cancel_nuke_for(dead_id);
        self.remove_target_references(dead_id);
        // Translation batch 29 (`0x005b05a0`): carrier/reaver hangars empty
        // on removal; the ghost-hero arm has no extra Rust state.
        let _ = self.units[i].a29_apply_removal_route();
        let source_id = self.units[i].last_damage_source;
        self.units[i].last_damage_source = 0;
        let source_owner = self
            .index_of(source_id)
            .map(|j| self.units[j].owner as usize)
            .filter(|&s| s != owner && s < self.players.len());
        if is_creep_source(type_id) {
            self.creep_dirty = true;
        }
        // An add-on dies with its parent, and a destroyed add-on clears the
        // parent's slot.
        if let Some(addon_id) = self.units[i].addon.take()
            && let Some(j) = self.index_of(addon_id)
            && self.units[j].alive
        {
            self.kill_unit(j);
        }
        let dead_id = self.units[i].id;
        for u in &mut self.units {
            if u.addon == Some(dead_id) {
                u.addon = None;
            }
        }
        let building = self.data.unit(type_id).is_building();
        if self.sudden_death
            && owner < self.players.len()
            && matches!(
                type_id,
                uid::COMMAND_CENTER | uid::NEXUS | uid::HATCHERY | uid::LAIR | uid::HIVE
            )
        {
            self.players[owner].defeated = true;
        }
        if owner < self.players.len() {
            self.players[owner].deaths += 1;
            if building {
                self.players[owner].buildings_lost += 1;
            } else {
                self.players[owner].units_lost += 1;
            }
            if (type_id as usize) < self.players[owner].death_counts.len() {
                self.players[owner].death_counts[type_id as usize] += 1;
            }
        }
        if let Some(killer) = source_owner {
            if let Some(j) = self.index_of(source_id) {
                self.units[j].kills += 1;
                self.units[j].credit_rank_kill();
            }
            self.players[killer].kills += 1;
            if (type_id as usize) < self.players[killer].kill_counts.len() {
                self.players[killer].kill_counts[type_id as usize] += 1;
            }
            if building {
                self.players[killer].razings += 1;
            }
            // Kill score (Men) and razing score (Building): the victim's
            // `units.dat` destroy score.
            let ty = self.data.unit(type_id);
            let destroy = ty.destroy_score as i32;
            if ty.group_flags & GROUP_MEN != 0 {
                self.players[killer].scores[4] += destroy;
            } else if ty.group_flags & GROUP_BUILDING != 0 {
                self.players[killer].scores[5] += destroy;
            }
        }
        // Loaded units die with their transport/bunker (retail behaviour);
        // they are not ejected alive.
        let cargo = std::mem::take(&mut self.units[i].cargo);
        for id in cargo {
            if let Some(j) = self.index_of(id) {
                // Loaded units are kept with `alive == false` and
                // `in_transport = Some(transport)`, so `kill_unit(j)` would
                // early-return without touching them.  Mark the cargo dead
                // directly and clear the link.
                self.units[j].hp = 0;
                self.units[j].alive = false;
                self.units[j].clear_transport_link();
                self.units[j].set_in_bunker(false);
                self.units[j].order = Order::Die;
                self.units[j].path.clear();
                self.units[j].path_goal = None;
            }
        }
    }

    pub(super) fn resolve_deaths(&mut self) {
        // Dead units stay in the vector (ids are stable) but are skipped by
        // every system.  Transports drop their cargo on death in kill_unit.
    }
}
