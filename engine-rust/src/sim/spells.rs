//! `impl World`: spells (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

/// Scanner Sweep lifetime: its iscript's waits before `sigorder 4`.
const SCANNER_FRAMES: i64 = 156;
/// Frames between a Recall cast and the units' arrival.
const RECALL_DELAY: u64 = 22;

impl World {
    // ------------------------------------------------------------------
    // Spells and modes
    // ------------------------------------------------------------------

    /// `0x005a3c90` / `0x005b7370`: unburrow clears the hidden/burrow state
    /// and the sprite visibility bit; the Rust model clears
    /// `status.burrowed`.
    pub(super) fn order_burrow(&mut self, i: usize, burrow: bool) {
        if burrow {
            if !self.players[self.units[i].owner as usize].has_tech(tech_id::BURROWING) {
                self.order_done(i);
                return;
            }
            self.units[i].status.burrowed = true;
        } else {
            self.units[i].status.burrowed = false;
        }
        self.order_done(i);
    }

    /// Siege Mode and back (OpenBW `order_Sieging` / `order_Unsieging`).
    /// Sieging: the tank stops, becomes the sieged type at once and is busy
    /// for the 64 frames of its deploy animation (iscript 92 Init: `wait 1`,
    /// five frames of 5, `wait 38`, `sigorder 1`).  Unsieging: it stays
    /// sieged for the 63 frames of SpecialState2 (`wait 38`, five frames of
    /// 5, `sigorder 1`) and only then becomes the tank again.  It cannot
    /// fire or take another order meanwhile (`order_wait` counts down; 1
    /// marks the last frame).
    pub(super) fn order_siege(&mut self, i: usize, siege: bool) {
        if !self.players[self.units[i].owner as usize].has_tech(tech_id::TANK_SIEGE_MODE) {
            self.units[i].order_wait = 0;
            self.order_done(i);
            return;
        }
        let ty = self.units[i].type_id;
        let wait = self.units[i].order_wait;
        if siege {
            if ty == uid::SIEGE_TANK {
                self.units[i].path.clear();
                self.units[i].path_goal = None;
                self.units[i].target = None;
                // OpenBW `order_Sieging`: hull and turret first turn to the
                // type's default direction (`units.dat` 12, heading 96).
                let rate = self
                    .data
                    .flingy(self.data.unit(ty).flingy)
                    .map_or(255, |f| f.turn_rate as i32)
                    .max(1);
                let cur = self.units[i].facing as i32;
                let mut diff = (SIEGE_HEADING - cur).rem_euclid(256);
                if diff > 128 {
                    diff -= 256;
                }
                self.units[i].facing = (cur + diff.clamp(-rate, rate)).rem_euclid(256) as u8;
                if self.units[i].facing as i32 != SIEGE_HEADING
                    || self.units[i].turret_facing as i32 != SIEGE_HEADING
                {
                    return;
                }
                // A tank that deploys on top of a building is destroyed.
                if self.overlaps_grounded_building(i) {
                    self.kill_unit(i);
                    return;
                }
                self.units[i].status.sieged = true;
                self.units[i].type_id = uid::SIEGE_MODE;
                self.units[i].order_wait = SIEGE_FRAMES + 1;
                return;
            }
            if ty == uid::SIEGE_MODE && wait > 1 {
                self.units[i].order_wait = wait - 1;
                return;
            }
        } else if ty == uid::SIEGE_MODE {
            if wait == 0 {
                self.units[i].target = None;
                // The cannon swings back to its rest heading first.
                if self.units[i].turret_facing as i32 != SIEGED_TURRET_HEADING {
                    return;
                }
                self.units[i].order_wait = UNSIEGE_FRAMES + 1;
                return;
            }
            if wait > 1 {
                self.units[i].order_wait = wait - 1;
                return;
            }
            self.units[i].status.sieged = false;
            self.units[i].type_id = uid::SIEGE_TANK;
        }
        self.units[i].order_wait = 0;
        self.order_done(i);
    }

    /// Whether unit `i`'s box overlaps a building on the ground.
    pub(super) fn overlaps_grounded_building(&self, i: usize) -> bool {
        let r = self
            .data
            .unit(self.units[i].type_id)
            .dimensions
            .rect_at((self.units[i].x / fp(1), self.units[i].y / fp(1)));
        self.units.iter().enumerate().any(|(k, b)| {
            k != i && b.alive && !b.lifted && self.data.unit(b.type_id).is_building() && {
                let o = self
                    .data
                    .unit(b.type_id)
                    .dimensions
                    .rect_at((b.x / fp(1), b.y / fp(1)));
                r.0 <= o.2 && o.0 <= r.2 && r.1 <= o.3 && o.1 <= r.3
            }
        })
    }

    /// True while a tank is in the middle of its siege / unsiege animation.
    pub fn is_transforming(&self, i: usize) -> bool {
        self.units.get(i).is_some_and(|u| {
            u.alive
                && ((matches!(u.order, Order::Siege | Order::Unsiege) && u.order_wait > 0)
                    || u.lift_phase.is_some())
        })
    }

    /// `0x005d4640`: Shield Battery target pre-check (different unit, same
    /// side and shielded).  A full pool is handled by the caller, which
    /// returns zero restored.
    pub fn can_shield_battery_target(&self, battery_id: u32, target_id: u32) -> bool {
        let Some(b) = self.index_of(battery_id) else {
            return false;
        };
        let Some(t) = self.index_of(target_id) else {
            return false;
        };
        if b == t || !self.units[b].alive || !self.units[t].alive {
            return false;
        }
        if self.units[b].owner != self.units[t].owner {
            return false;
        }
        self.data.unit(self.units[t].type_id).has_shields
    }

    /// Shield Battery recharge (orders 66/67, `0x005d4700`).  The classic ability converts
    /// one energy point into two plasma-shield points and can be repeated
    /// until the battery is empty.  Returns the number of shield points
    /// restored.
    pub fn shield_battery(&mut self, battery_id: u32, target_id: u32) -> Result<i32, SimError> {
        let restored = self.shield_battery_unrecorded(battery_id, target_id)?;
        if restored > 0
            && let Some(r) = self.recorder.as_mut()
        {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::ShieldBattery {
                    battery: battery_id,
                    target: target_id,
                },
            );
        }
        Ok(restored)
    }

    pub(super) fn shield_battery_unrecorded(
        &mut self,
        battery_id: u32,
        target_id: u32,
    ) -> Result<i32, SimError> {
        let b = self.index_of(battery_id).ok_or(SimError::BadUnit)?;
        let t = self.index_of(target_id).ok_or(SimError::BadUnit)?;
        if self.units[b].type_id != uid::SHIELD_BATTERY || !self.units[b].completed {
            return Err(SimError::InvalidTarget);
        }
        if !self.units[b].alive || !self.units[t].alive {
            return Err(SimError::BadUnit);
        }
        if self.units[b].owner != self.units[t].owner {
            return Err(SimError::InvalidTarget);
        }
        if !self.can_shield_battery_target(battery_id, target_id) {
            return Err(SimError::InvalidTarget);
        }
        let ty = self.data.unit(self.units[t].type_id).clone();
        let max = fp(ty.shield_points as i32);
        if self.units[t].shields >= max {
            return Ok(0);
        }
        if self.units[b].energy <= 0 {
            return Err(SimError::NotEnoughEnergy);
        }
        let need = max - self.units[t].shields;
        let restore_fp = need.min(self.units[b].energy * fp(2));
        let energy_used = (restore_fp + fp(2) - 1) / fp(2);
        self.units[t].shields += restore_fp;
        self.units[b].energy -= energy_used;
        // `0x005d4700`: mark the target as recharged for this tick.
        self.units[t].shield_recharged = true;
        Ok(restore_fp / fp(1))
    }

    /// Recorded mutator (`RecAct::Cast`).
    pub fn cast(
        &mut self,
        caster_id: u32,
        spell: Spell,
        target: SpellTarget,
    ) -> Result<(), SimError> {
        self.cast_unrecorded(caster_id, spell, target)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::Cast {
                    caster: caster_id,
                    spell,
                    target,
                },
            );
        }
        Ok(())
    }

    /// Energy (fixed point) the Yamato charge takes; none with the
    /// infinite-energy cheat.
    pub(super) fn yamato_charge_cost(&self, i: usize) -> i32 {
        if self.players[self.units[i].owner as usize]
            .cheats
            .infinite_energy
        {
            return 0;
        }
        spell_energy_cost(&self.data, Spell::YamatoGun) * ENERGY_SCALE
    }

    pub(super) fn cast_unrecorded(
        &mut self,
        caster_id: u32,
        spell: Spell,
        target: SpellTarget,
    ) -> Result<(), SimError> {
        let i = self.index_of(caster_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner as usize;
        if self.cheat_no_enemy_spells && self.players[owner].is_computer {
            return Err(SimError::MissingTech);
        }
        // A detached Comsat/Nuclear Silo is inactive.
        if matches!(
            self.units[i].type_id,
            uid::COMSAT_STATION | uid::NUCLEAR_SILO
        ) && !self.units[i].addon_attached
        {
            return Err(SimError::InvalidTarget);
        }
        let tech_id = spell.tech_id();
        // Heroes (units.dat flag 0x40, Jim Raynor) use their abilities
        // without the research (INFERENCE from BW; falsifier: a hero card
        // without the button before the research).
        let hero = self.data.unit(self.units[i].type_id).is_hero();
        // Scanner Sweep, Archon Warp and Dark Archon Meld have no research
        // building; they are implicit abilities of their caster.
        let implicit = matches!(
            spell,
            Spell::Healing | Spell::ScannerSweep | Spell::ArchonWarp | Spell::DarkArchonMeld
        );
        if !self.players[owner].has_tech(tech_id) && !implicit && !hero {
            return Err(SimError::MissingTech);
        }
        if spell == Spell::SpiderMines && self.units[i].hangar == 0 {
            return Err(SimError::QueueFull);
        }
        if !caster_can_cast(self.units[i].type_id, spell) {
            return Err(SimError::Unsupported);
        }
        // Stim Packs cost 10 hit points and are refused at 10 or fewer
        // (OpenBW `Stimpacks`: `if (u->hp <= 10_fp8) return`).
        if spell == Spell::StimPacks && self.units[i].hp <= fp(10) {
            return Err(SimError::InvalidTarget);
        }
        // `0x005cbea0` validates the target through `FUN_0059ca70` before
        // reading and spending the techdata energy.  A rejected target must
        // therefore leave the energy pool untouched.
        if spell.takes_unit_target() {
            let SpellTarget::Unit(id) = target else {
                return Err(SimError::InvalidTarget);
            };
            let Some(t) = self.index_of(id) else {
                return Err(SimError::InvalidTarget);
            };
            if matches!(spell, Spell::SpawnBroodlings | Spell::Consume) && id == caster_id {
                return Err(SimError::InvalidTarget);
            }
            // OpenBW `unit_tech_target_valid`: Restoration takes any unit but
            // a building.  `FUN_005c7460` (same owner, 61+ hit points and
            // shields, a removable effect) is the computer's choice of
            // target, so it only gates computer casters.
            if spell == Spell::Restoration
                && (self.data.unit(self.units[t].type_id).is_building()
                    || (self.players[owner].is_computer
                        && !self.restoration_worthwhile(caster_id, id)))
            {
                return Err(SimError::InvalidTarget);
            }
            if spell == Spell::Healing {
                let ty = self.data.unit(self.units[t].type_id);
                if self.units[t].owner != self.units[i].owner
                    || ty.is_building()
                    || !ty.is_organic()
                    || self.units[t].hp >= ty.hit_points_raw as i32
                {
                    return Err(SimError::InvalidTarget);
                }
            }
            // Optical Flare blinds any unit, air included (its weapons.dat
            // row says ground only, but the tech is not gated by it: the
            // classic use is on Observers and Overlords); only buildings
            // are refused.
            if spell == Spell::OpticalFlare && self.data.unit(self.units[t].type_id).is_building() {
                return Err(SimError::InvalidTarget);
            }
            // OpenBW `unit_tech_target_valid`: Consume takes any of the
            // caster's own units that is not a building, Terran or a Larva
            // (a mind-controlled Protoss unit too), not the weapon's
            // organic-only flags.
            if spell == Spell::Consume {
                let ty = self.data.unit(self.units[t].type_id);
                if self.units[t].owner != self.units[i].owner
                    || ty.is_building()
                    || self.unit_race_class(self.units[t].type_id) == 1
                    || self.units[t].type_id == uid::LARVA
                    || self.units[t].status.stasis_ticks > 0
                {
                    return Err(SimError::InvalidTarget);
                }
            }
            if let Some(weapon) = spell.weapon_id()
                && spell != Spell::OpticalFlare
                && spell != Spell::Consume
            {
                let info = WeaponTarget {
                    unit: self.data.unit(self.units[t].type_id),
                    owner: self.units[t].owner,
                    flying: self.is_air_unit(t),
                    untargetable: !self.units[t].alive
                        || self.units[t].invincible
                        || self.units[t].status.stasis_ticks > 0,
                };
                if !self
                    .data
                    .weapon_target_ok(weapon, self.units[i].owner, Some(info))
                {
                    return Err(SimError::InvalidTarget);
                }
            }
        }
        if let Some(range) = self.spell_range_px(spell) {
            let pos = match target {
                SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
                SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
                SpellTarget::None => None,
            };
            if let Some(p) = pos
                && self.distance_to_point(i, p) > range
            {
                return Err(SimError::InvalidTarget);
            }
        }
        // Lurker Aspect is a unit morph, not an energy ability; the techdata
        // energy column is unused for it.
        // Switching a cloak off is free; only cloaking costs energy.
        let decloak = matches!(spell, Spell::CloakingField | Spell::PersonnelCloaking)
            && self.units[i].status.cloaked;
        let cost = if spell == Spell::LurkerAspect || decloak {
            0
        } else {
            spell_energy_cost(&self.data, spell)
        };
        if !self.players[owner].cheats.infinite_energy
            && !self.units[i].spend_energy_fixed(cost * ENERGY_SCALE)
        {
            return Err(SimError::NotEnoughEnergy);
        }
        self.apply_spell(i, spell, target);
        Ok(())
    }

    /// Order-weapon maximum range in 1/256 pixels.  Spells whose order row
    /// has weapon 130 (Scanner Sweep, Recall, Meld, ...) have no weapon
    /// range and fall back to [`spell_range`].
    pub(super) fn spell_range_px(&self, spell: Spell) -> Option<i32> {
        // OpenBW `order_PlaceMine`: the Vulture lays the mine once it is
        // within 20 px of the point.
        if spell == Spell::SpiderMines {
            return Some(20 * fp(1));
        }
        spell
            .weapon_id()
            .and_then(|id| self.data.weapon(id))
            .map(|w| w.max_range as i32 * fp(1))
    }

    pub(super) fn order_cast(&mut self, i: usize, spell: Spell, target: SpellTarget) {
        let current = self.units[i].anim.as_ref().map(|a| a.anim as usize);
        if current == Some(ANIM_CAST_SPELL) {
            return;
        }
        // Spells are released on the iscript `castspell` opcode.  The order
        // waits at casting range until the animation reaches that frame.
        let range = self
            .spell_range_px(spell)
            .unwrap_or_else(|| spell_range(spell));
        let pos = match target {
            SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
            SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
            SpellTarget::None => Some((self.units[i].x, self.units[i].y)),
        };
        if let Some(p) = pos
            && self.distance_to_point(i, p) > range
        {
            self.move_towards(i, (p.0 / fp(1), p.1 / fp(1)), range / fp(1));
            return;
        }
        if spell == Spell::YamatoGun {
            // Stop and charge; the Battlecruiser's CastSpell animation
            // shows the charge overlay.
            let charge = self
                .data
                .image(YAMATO_CHARGE_IMAGE)
                .and_then(|im| {
                    self.data
                        .iscript
                        .frames_until(im.iscript_id as u16, 0, 0x27)
                })
                .unwrap_or(0);
            if charge > 0 {
                // OpenBW `order_CastSpell`: the energy is spent when the
                // cast animation (the charge) starts, not when it fires.
                let cost = self.yamato_charge_cost(i);
                if !self.units[i].spend_energy_fixed(cost) {
                    self.order_done(i);
                    return;
                }
                self.units[i].path.clear();
                self.units[i].path_goal = None;
                self.units[i].order_wait = charge.min(255) as u8;
                self.start_animation(i, ANIM_CAST_SPELL);
                return;
            }
        }
        let has_anim = self
            .data
            .unit_iscript_id(self.data.unit(self.units[i].type_id))
            .is_some_and(|id| self.data.iscript.animation_has_cast(id, ANIM_CAST_SPELL));
        if has_anim {
            self.start_animation(i, ANIM_CAST_SPELL);
        } else {
            // Unrecorded: the order itself is the logged command.
            let _ = self.cast_unrecorded(self.units[i].id, spell, target);
            self.order_done(i);
        }
    }

    pub(super) fn apply_spell(&mut self, caster: usize, spell: Spell, target: SpellTarget) {
        let c_owner = self.units[caster].owner;
        let (cx, cy) = (self.units[caster].x, self.units[caster].y);
        let target_idx = match target {
            SpellTarget::Unit(id) => self.index_of(id),
            _ => None,
        };
        let point = match target {
            SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
            SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
            SpellTarget::None => Some((cx, cy)),
        };
        match spell {
            Spell::StimPacks => {
                let hp = self.units[caster].hp;
                self.units[caster].hp = (hp - fp(10)).max(1);
                self.units[caster].status.stim_ticks = STIM_FRAMES;
                let id = self.units[caster].id;
                self.push_spell_cue(SpellCueKind::Stim, id);
            }
            Spell::TankSiegeMode => {
                // The spell only starts the transformation order.
                if !self.is_transforming(caster) {
                    self.units[caster].order = if self.units[caster].type_id == uid::SIEGE_TANK {
                        Order::Siege
                    } else {
                        Order::Unsiege
                    };
                }
            }
            Spell::CloakingField | Spell::PersonnelCloaking => {
                self.units[caster].status.cloaked = !self.units[caster].status.cloaked;
            }
            Spell::Burrowing => {
                self.units[caster].status.burrowed = true;
            }
            Spell::YamatoGun => {
                // The blast travels (weapons.dat row 30, a flying bullet)
                // and deals its 260 on arrival.
                if let Some(t) = target_idx
                    && let Some(w) = self.data.weapon(wid::YAMATO_GUN).cloned()
                {
                    if Projectile::weapon_is_travelling(&w) {
                        self.spawn_projectile(caster, t, wid::YAMATO_GUN, &w);
                    } else {
                        self.apply_hit(caster, t, &w, 1);
                    }
                }
            }
            Spell::DefensiveMatrix => {
                if let Some(t) = target_idx {
                    self.units[t].status.apply_matrix(fp(250));
                }
            }
            Spell::EmpShockwave => {
                // The EMP missile flies to the point (weapons.dat row 33)
                // and goes off there (`hit::EMP`).
                if let Some(p) = point
                    && let Some(w) = self.data.weapon(wid::EMP_SHOCKWAVE).cloned()
                {
                    self.spawn_point_bullet(caster, wid::EMP_SHOCKWAVE, &w, p);
                }
            }
            Spell::Irradiate => {
                // Irradiate lands with its bullet (weapons.dat row 34,
                // "appear on target"), a few frames after the cast.
                if let Some(t) = target_idx
                    && !self.data.unit(self.units[t].type_id).is_building()
                {
                    let tid = self.units[t].id;
                    let pos = (self.units[t].x, self.units[t].y);
                    match self.data.weapon(wid::IRRADIATE).cloned() {
                        Some(w) if w.explosion == hit::IRRADIATE => {
                            self.instant_hit(c_owner, Some(tid), caster, wid::IRRADIATE, &w, pos);
                        }
                        _ => self.units[t].status.apply_irradiate(IRRADIATE_UPDATES),
                    }
                }
            }
            Spell::PsionicStorm => {
                if let Some((tx, ty)) = point
                    && let Some(w) = self.data.weapon(wid::PSIONIC_STORM).cloned()
                {
                    self.spawn_persistent_field(caster, wid::PSIONIC_STORM, &w, (tx, ty));
                }
            }
            Spell::Hallucination => {
                // OpenBW `order_CastHallucination`: two copies, owned by the
                // caster, each living 1350 frames (`default_remove_timer`,
                // retail `0x005cc810` returns 0x546 for a hallucination).
                if let Some(t) = target_idx {
                    let ty = self.units[t].type_id;
                    let owner = self.units[caster].owner;
                    let (x, y) = (self.units[t].x / fp(1), self.units[t].y / fp(1));
                    for _ in 0..2 {
                        let Ok(new) = self.spawn_px(owner as usize, ty, x, y) else {
                            break;
                        };
                        self.units[new].status.hallucination = true;
                        self.units[new].status.revealed_ticks = 600;
                        self.units[new].expires_at =
                            self.tick as i64 + HALLUCINATION_LIFETIME_TICKS;
                    }
                }
            }
            Spell::StasisField => {
                if let Some((tx, ty)) = point {
                    for j in 0..self.units.len() {
                        if self.units[j].alive
                            && !self.units[j].status.burrowed
                            && !self.data.unit(self.units[j].type_id).is_building()
                            && self.distance_to_point(j, (tx, ty)) <= fp(64)
                        {
                            self.units[j].status.apply_stasis(STASIS_UPDATES);
                        }
                    }
                }
            }
            Spell::ArchonWarp | Spell::DarkArchonMeld => {
                let (a, b) = (uid::HIGH_TEMPLAR, uid::DARK_TEMPLAR);
                let (wanted, result) = if spell == Spell::ArchonWarp {
                    (a, uid::ARCHON)
                } else {
                    (b, uid::DARK_ARCHON)
                };
                let count = self
                    .units
                    .iter()
                    .filter(|u| u.alive && u.owner == c_owner && u.type_id == wanted)
                    .count();
                if count >= 2 {
                    let mut removed = 0;
                    let mut merged = Vec::new();
                    for j in 0..self.units.len() {
                        if removed == 2 {
                            break;
                        }
                        if self.units[j].alive
                            && self.units[j].owner == c_owner
                            && self.units[j].type_id == wanted
                        {
                            merged.push(self.units[j].status.clone());
                            self.units[j].alive = false;
                            removed += 1;
                        }
                    }
                    if let Ok(new) = self.spawn_px(c_owner as usize, result, cx / fp(1), cy / fp(1))
                    {
                        // `0x005c1fe0`/`0x005c206d`: the merged unit inherits
                        // the longer timers from every consumed unit.
                        for status in &merged {
                            self.units[new].status.merge_from(status);
                        }
                    }
                }
            }
            Spell::Feedback => {
                if let Some(t) = target_idx {
                    self.feedback_damage(t, caster as u32);
                }
            }
            Spell::Maelstrom => {
                if let Some((tx, ty)) = point {
                    for j in 0..self.units.len() {
                        if self.units[j].alive
                            && !self.units[j].status.burrowed
                            && self.distance_to_point(j, (tx, ty)) <= fp(64)
                            && self.data.unit(self.units[j].type_id).is_organic()
                            && !self.data.unit(self.units[j].type_id).is_building()
                        {
                            self.units[j].status.apply_maelstrom(MAELSTROM_UPDATES);
                        }
                    }
                }
            }
            Spell::MindControl => {
                // OpenBW `order_MindControl`: a hallucination dies, a Dark
                // Archon target loses its energy, and the caster's shields
                // drop to 0.
                if let Some(t) = target_idx {
                    if self.units[t].status.hallucination {
                        self.kill_unit(t);
                    } else {
                        self.units[t].owner = c_owner;
                        if self.units[t].type_id == uid::DARK_ARCHON {
                            self.units[t].set_energy_fixed(0);
                        }
                    }
                    self.units[caster].shields = 0;
                }
            }
            Spell::SpawnBroodlings => {
                if let Some(t) = target_idx {
                    let tx = self.units[t].x / fp(1);
                    let ty = self.units[t].y / fp(1);
                    if self.data.unit(self.units[t].type_id).is_organic() {
                        self.kill_unit(t);
                        self.spawn_broodling(c_owner as usize, tx, ty);
                        self.spawn_broodling(c_owner as usize, tx + 16, ty);
                    }
                }
            }
            Spell::Parasite => {
                if let Some(t) = target_idx {
                    self.units[t].status.parasite = true;
                    self.units[t].status.revealed_ticks = 1200;
                }
            }
            Spell::Infestation => {
                if let Some(t) = target_idx {
                    let max = self.data.unit(uid::COMMAND_CENTER).hit_points_raw as i32;
                    if self.units[t].type_id == uid::COMMAND_CENTER && self.units[t].hp * 2 < max {
                        if let Some(addon) = self.units[t].addon.take()
                            && let Some(a) = self.index_of(addon)
                        {
                            self.kill_unit(a);
                        }
                        self.units[t].type_id = uid::INFESTED_COMMAND_CENTER;
                        self.units[t].owner = c_owner;
                        self.units[t].shields = 0;
                        self.units[t].status.parasite = false;
                    }
                }
            }
            Spell::Plague => {
                if let Some((tx, ty)) = point {
                    for j in 0..self.units.len() {
                        if self.units[j].alive
                            && !self.units[j].status.burrowed
                            && self.distance_to_point(j, (tx, ty)) <= fp(64)
                        {
                            self.units[j].status.apply_plague();
                        }
                    }
                }
            }
            Spell::Consume => {
                if let Some(t) = target_idx
                    && t != caster
                {
                    // OpenBW `consume_unit`: a flat 50 energy (none from a
                    // hallucination), up to the caster's maximum.
                    if !self.units[t].status.hallucination {
                        self.units[caster].add_energy_points(50);
                    }
                    self.kill_unit(t);
                }
            }
            Spell::Ensnare => {
                if let Some((tx, ty)) = point {
                    for j in 0..self.units.len() {
                        if j == caster
                            || !self.units[j].alive
                            || self.units[j].in_transport.is_some()
                            || self.units[j].status.burrowed
                            || self.data.unit(self.units[j].type_id).is_building()
                            || self.distance_to_point(j, (tx, ty)) > fp(64)
                        {
                            continue;
                        }
                        self.units[j].status.apply_ensnare();
                    }
                }
            }
            Spell::Recall => {
                // OpenBW `order_Recall`: the units go 22 frames after the
                // cast (`main_order_timer = 22`), see `advance_recalls`.
                if let Some(p) = point {
                    let id = self.units[caster].id;
                    self.pending_recalls.push((self.tick + RECALL_DELAY, id, p));
                }
            }
            Spell::Restoration => {
                // Restoration lands when its bullet (weapons.dat row 102,
                // "appear on target") reaches `domissiledmg`.
                if let Some(t) = target_idx {
                    let tid = self.units[t].id;
                    let pos = (self.units[t].x, self.units[t].y);
                    match self.data.weapon(wid::RESTORATION).cloned() {
                        Some(w) if w.explosion == hit::RESTORATION => {
                            self.instant_hit(c_owner, Some(tid), caster, wid::RESTORATION, &w, pos);
                        }
                        _ => self.restore_unit(t),
                    }
                }
            }
            Spell::DisruptionWeb => {
                if let Some((tx, ty)) = point {
                    for j in 0..self.units.len() {
                        if self.units[j].alive && self.distance_to_point(j, (tx, ty)) <= fp(64) {
                            self.units[j].status.disruption_ticks = 240;
                        }
                    }
                    self.spawn_temporary(c_owner, uid::DISRUPTION_WEB, (tx, ty), 360);
                }
            }
            Spell::OpticalFlare => {
                // The Medic throws the flare (weapons.dat row 107, a bullet
                // that follows its target); the unit is blinded when it
                // lands.
                if let Some(t) = target_idx {
                    let cid = self.units[caster].id;
                    self.push_spell_cue(SpellCueKind::OpticalFlareCast, cid);
                    match self.data.weapon(wid::OPTICAL_FLARE).cloned() {
                        Some(w) if Projectile::weapon_is_travelling(&w) => {
                            self.spawn_projectile(caster, t, wid::OPTICAL_FLARE, &w);
                        }
                        _ => self.blind_unit(t),
                    }
                }
            }
            Spell::Healing => {
                // The Heal command only names the patient: the Medic then
                // walks to it and heals it at the normal rate.
                if let Some(t) = target_idx {
                    self.units[caster].target = Some(self.units[t].id);
                }
            }
            Spell::Lockdown => {
                // The Ghost fires the Lockdown missile (weapons.dat row 32,
                // a bullet that follows its target); the unit is disabled
                // when it lands (`hit::LOCKDOWN`).
                if let Some(t) = target_idx
                    && self.data.unit(self.units[t].type_id).is_mechanical()
                {
                    match self.data.weapon(wid::LOCKDOWN).cloned() {
                        Some(w) if Projectile::weapon_is_travelling(&w) => {
                            self.spawn_projectile(caster, t, wid::LOCKDOWN, &w);
                        }
                        _ => self.units[t].status.apply_lockdown(LOCKDOWN_UPDATES),
                    }
                }
            }
            Spell::DarkSwarm => {
                let point = match target {
                    SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
                    SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
                    SpellTarget::None => None,
                };
                if let Some((tx, ty)) = point {
                    self.spawn_dark_swarm(c_owner, (tx, ty));
                }
            }
            Spell::ScannerSweep => {
                let point = match target {
                    SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
                    SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
                    SpellTarget::None => None,
                };
                // OpenBW `order_CastScannerSweep`: a Scanner Sweep unit (33,
                // sight 10, detector) appears at the point; its iscript
                // waits 156 frames and signals `order_Scanner`, which kills
                // it.  It revealed a 96 px circle for 400 frames instead.
                if let Some((tx, ty)) = point
                    && let Some(scan) =
                        self.spawn_temporary(c_owner, uid::SCANNER_SWEEP, (tx, ty), SCANNER_FRAMES)
                {
                    self.units[scan].invincible = true;
                    self.reveal_point(c_owner, tx / fp(1), ty / fp(1), 10);
                }
            }
            Spell::SpiderMines => {
                let point = match target {
                    SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
                    SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
                    SpellTarget::None => None,
                };
                // One mine per order from the Vulture's three (`hangar`),
                // as `place_spider_mine`; the old version laid all three.
                if let Some((tx, ty)) = point
                    && self.units[caster].hangar > 0
                    && let Some(idx) = self.spawn_temporary(c_owner, uid::SPIDER_MINE, (tx, ty), 0)
                {
                    // The mine takes the Vulture's heading and starts in
                    // the open (`update_specials` runs its state machine).
                    self.units[idx].facing = self.units[caster].facing;
                    self.units[idx].mine_armed_at = self.tick as i64 + SPIDER_MINE_WAIT_FRAMES;
                    self.units[caster].hangar -= 1;
                }
            }
            Spell::LurkerAspect => {
                if let Some(t) = target_idx {
                    let _ = self.morph_unit_unrecorded(self.units[t].id, uid::LURKER);
                }
            }
        }
    }

    /// `FUN_005c7460`: whether a Medic's Restoration is worthwhile on
    /// `target` for `caster`.  The recovered routine requires the same owner,
    /// at least 61 whole HP+shields and a removable status.
    pub fn restoration_worthwhile(&self, caster: u32, target: u32) -> bool {
        let (Some(c), Some(t)) = (self.index_of(caster), self.index_of(target)) else {
            return false;
        };
        let (cu, tu) = (&self.units[c], &self.units[t]);
        if !tu.alive || tu.in_transport.is_some() || self.data.unit(tu.type_id).is_building() {
            return false;
        }
        if cu.owner != tu.owner {
            return false;
        }
        if tu.hp / fp(1) + tu.shields / fp(1) < 0x3d {
            return false;
        }
        tu.status.has_removable_effect()
    }

    /// Toggle a Ghost's/Wraith's personal cloak.  The tech gate is the same
    /// `Cloaking Field` / `Personnel Cloaking` research used by the ability.
    pub fn set_cloak(&mut self, unit_id: u32, cloaked: bool) -> Result<(), SimError> {
        self.set_cloak_unrecorded(unit_id, cloaked)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::Cloak {
                    unit: unit_id,
                    cloaked,
                },
            );
        }
        Ok(())
    }

    pub(super) fn set_cloak_unrecorded(
        &mut self,
        unit_id: u32,
        cloaked: bool,
    ) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        let owner = self.units[i].owner as usize;
        let (tech_id, spell) = match self.units[i].type_id {
            t if is_ghost_type(t) => (tech_id::PERSONNEL_CLOAKING, Spell::PersonnelCloaking),
            uid::WRAITH | 21 => (tech_id::CLOAKING_FIELD, Spell::CloakingField),
            _ => return Err(SimError::Unsupported),
        };
        let hero = self.data.unit(self.units[i].type_id).flags & 0x40 != 0;
        if cloaked && !self.players[owner].has_tech(tech_id) && !hero {
            return Err(SimError::MissingTech);
        }
        // Cloaking costs the techdata energy (25); decloaking is free.
        if cloaked
            && !self.units[i].status.cloaked
            && !self.players[owner].cheats.infinite_energy
            && !self.units[i]
                .spend_energy_fixed(spell_energy_cost(&self.data, spell) * ENERGY_SCALE)
        {
            return Err(SimError::NotEnoughEnergy);
        }
        self.units[i].status.cloaked = cloaked;
        Ok(())
    }

    /// OpenBW `restore_unit`: a hallucination dies; anything else loses
    /// parasite, blindness, ensnare, plague, irradiate, lockdown, maelstrom
    /// and acid spores (`0x005d2620` / `0x005dc300` / `0x005d2540` /
    /// `0x005d2660` clear the timers).
    pub(super) fn restore_unit(&mut self, t: usize) {
        if self.units[t].status.hallucination {
            self.kill_unit(t);
            return;
        }
        let s = &mut self.units[t].status;
        s.clear_lockdown();
        s.clear_plague();
        s.clear_irradiate();
        s.clear_maelstrom();
        s.clear_ensnare();
        s.optical_flare_ticks = 0;
        s.parasite = false;
        s.clear_acid_spores();
        let id = self.units[t].id;
        self.push_spell_cue(SpellCueKind::Restoration, id);
    }

    /// OpenBW `blind_unit`: Optical Flare lands.  A hallucination dies;
    /// anything else is blinded until Restoration (any non-zero value of
    /// the timer means blind; it does not count down).
    pub(super) fn blind_unit(&mut self, t: usize) {
        if self.units[t].status.hallucination {
            self.kill_unit(t);
            return;
        }
        self.units[t].status.optical_flare_ticks = 600;
        let id = self.units[t].id;
        self.push_spell_cue(SpellCueKind::OpticalFlareHit, id);
    }

    pub(super) fn push_spell_cue(&mut self, kind: SpellCueKind, unit: u32) {
        if self.spell_cues.len() >= SPELL_CUE_KEEP {
            self.spell_cues.pop_front();
        }
        self.spell_cues.push_back(SpellCue { kind, unit });
        self.spell_cue_seq += 1;
    }

    /// Whether `medic` may heal `j` (OpenBW `medic_can_heal_target`): a
    /// living, wounded, organic, non-building ally that is not the Medic
    /// itself and is not loaded in a transport or Bunker.
    pub(super) fn medic_can_heal(&self, medic: usize, j: usize) -> bool {
        let t = &self.units[j];
        let ty = self.data.unit(t.type_id);
        j != medic
            && t.alive
            && t.in_transport.is_none()
            && !t.in_bunker
            && !t.invincible
            && !self.is_enemy(self.units[medic].owner, t.owner)
            && ty.is_organic()
            && !ty.is_building()
            // OpenBW `medic_can_heal_target`: not a flyer, not a
            // hallucination, not disabled (lockdown, stasis, maelstrom).
            && !self.is_air_unit(j)
            && !t.status.hallucination
            && t.status.lockdown_ticks == 0
            && t.status.stasis_ticks == 0
            && t.status.maelstrom_ticks == 0
            && t.hp < ty.hit_points_raw as i32
    }

    /// The unit a Medic is (or will be) healing: its current target while
    /// that stays healable, else the nearest healable unit whose box is
    /// within [`MEDIC_SEARCH_PX`] and that no other Medic has taken.
    pub(super) fn medic_pick_target(&self, i: usize, taken: &[usize]) -> Option<usize> {
        if let Some(t) = self.units[i].target.and_then(|t| self.index_of(t))
            && self.medic_can_heal(i, t)
            && !taken.contains(&t)
        {
            return Some(t);
        }
        // OpenBW `find_medic_target`: the nearest healable unit inside the
        // square of 160 px around the Medic.
        let (mx, my) = (self.units[i].x, self.units[i].y);
        let half = MEDIC_SEARCH_PX * fp(1);
        (0..self.units.len())
            .filter(|&j| self.medic_can_heal(i, j) && !taken.contains(&j))
            .filter(|&j| {
                (self.units[j].x - mx).abs() <= half && (self.units[j].y - my).abs() <= half
            })
            .map(|j| (self.distance(i, j), j))
            .min()
            .map(|(_, j)| j)
    }

    /// True while Medic `i` stands in reach of its heal target with the
    /// energy to heal it (the renderer plays the heal animation then).
    pub fn medic_is_healing(&self, i: usize) -> bool {
        let Some(u) = self.units.get(i) else {
            return false;
        };
        u.alive
            && u.type_id == uid::MEDIC
            && u.energy_fixed() >= MEDIC_HEAL_COST_PER_TICK
            && u.target.and_then(|t| self.index_of(t)).is_some_and(|t| {
                self.medic_can_heal(i, t) && self.units_distance_px(i, t) <= MEDIC_REACH_PX
            })
    }

    /// Medics heal wounded organic allies: 200/256 hit points for 100/256
    /// energy per frame, one Medic per patient.  An idle Medic (or one on
    /// attack-move, retail `HealMove`) walks to a patient within
    /// [`MEDIC_SEARCH_PX`]; on hold position it only heals what it can
    /// reach.  A Medic inside a transport or Bunker does nothing.
    pub(super) fn medic_healing(&mut self) {
        let mut taken: Vec<usize> = Vec::new();
        for i in 0..self.units.len() {
            if !self.units[i].alive
                || self.units[i].type_id != uid::MEDIC
                || self.units[i].in_transport.is_some()
                || self.units[i].in_bunker
                || self.units[i].status.lockdown_ticks > 0
                || self.units[i].status.stasis_ticks > 0
                || self.units[i].status.maelstrom_ticks > 0
            {
                continue;
            }
            let hold = self.units[i].a08_is_hold_position_idle();
            let roaming = matches!(self.units[i].order, Order::Idle | Order::AttackMove { .. });
            if !hold && !roaming {
                continue;
            }
            if self.units[i].energy_fixed() < MEDIC_HEAL_COST_PER_TICK {
                self.units[i].target = None;
                continue;
            }
            let Some(target) = self.medic_pick_target(i, &taken) else {
                self.units[i].target = None;
                continue;
            };
            let dist = self.units_distance_px(i, target);
            if dist > MEDIC_REACH_PX {
                if hold {
                    self.units[i].target = None;
                    continue;
                }
                // Walk up to the patient.
                self.units[i].target = Some(self.units[target].id);
                taken.push(target);
                let (tx, ty) = (self.units[target].x / fp(1), self.units[target].y / fp(1));
                self.move_towards(i, (tx, ty), fp(8));
                continue;
            }
            self.units[i].target = Some(self.units[target].id);
            taken.push(target);
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            let (dx, dy) = (
                self.units[target].x - self.units[i].x,
                self.units[target].y - self.units[i].y,
            );
            if dx != 0 || dy != 0 {
                self.units[i].facing = direction_byte(dx, dy);
            }
            let max = self.data.unit(self.units[target].type_id).hit_points_raw as i32;
            // OpenBW `medic_try_heal`: up to 200/256 hp for half as much
            // energy (the last, smaller step costs less).
            let healed = MEDIC_HEAL_PER_TICK.min(max - self.units[target].hp);
            if healed > 0 && self.units[i].spend_energy_fixed(healed / 2) {
                self.units[target].hp += healed;
            }
        }
    }

    /// OpenBW `order_Recall`: once due, the Arbiter's own units inside a
    /// 64 px square of the point (not buildings, larvae, eggs,
    /// hallucinations, burrowed or invincible units) come to it.  It sent the
    /// units around the Arbiter to the point instead.
    pub(super) fn advance_recalls(&mut self) {
        let tick = self.tick;
        let (due, wait): (Vec<_>, Vec<_>) = std::mem::take(&mut self.pending_recalls)
            .into_iter()
            .partition(|r| r.0 <= tick);
        self.pending_recalls = wait;
        for (_, arbiter, (tx, ty)) in due {
            let Some(a) = self.index_of(arbiter).filter(|&a| self.units[a].alive) else {
                continue;
            };
            let (owner, cx, cy) = (self.units[a].owner, self.units[a].x, self.units[a].y);
            for j in 0..self.units.len() {
                let u = &self.units[j];
                if u.alive
                    && u.owner == owner
                    && j != a
                    && (u.x - tx).abs() <= fp(64)
                    && (u.y - ty).abs() <= fp(64)
                    && !self.data.unit(u.type_id).is_building()
                    && !u.status.hallucination
                    && !u.status.burrowed
                    && !u.invincible
                    && !matches!(u.type_id, uid::LARVA | uid::EGG | uid::LURKER_EGG)
                    && u.in_transport.is_none()
                {
                    // `set_unit_order(target, return_to_idle)` and
                    // `remove_target_references`: it arrives idle and nobody
                    // keeps it as a target.
                    let id = self.units[j].id;
                    let u = &mut self.units[j];
                    u.x = cx;
                    u.y = cy;
                    u.order = Order::Idle;
                    u.target = None;
                    u.path.clear();
                    u.path_goal = None;
                    for k in 0..self.units.len() {
                        if self.units[k].target == Some(id) {
                            self.units[k].target = None;
                            if matches!(self.units[k].order, Order::AttackUnit { target } if target == id)
                            {
                                self.units[k].order = Order::Idle;
                            }
                        }
                    }
                }
            }
        }
    }
}
