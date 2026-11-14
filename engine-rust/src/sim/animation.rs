//! `impl World`: animation (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

impl World {
    // ------------------------------------------------------------------
    // iscript animation VM integration
    // ------------------------------------------------------------------

    /// Start an animation and execute the script's first tick immediately.
    /// The retail engine does the same from `FUN_7a1148f0`, which is why a
    /// Walking script's first `move` takes effect on the tick the order is
    /// issued.
    pub(super) fn start_animation(&mut self, i: usize, anim: usize) {
        let ty = self.data.unit(self.units[i].type_id).clone();
        let Some(iscript) = self.data.unit_iscript_id(&ty) else {
            return;
        };
        let direction = self.units[i].facing;
        if let Some(st) = self.data.iscript.start_animation(iscript, anim, direction) {
            self.units[i].anim = Some(st);
            self.tick_animation(i);
        }
    }

    /// Advance the current animation by one logical tick and apply gameplay
    /// events (weapon release, spell release, order signals, turns).
    pub(super) fn tick_animation(&mut self, i: usize) {
        let Some(mut st) = self.units[i].anim.take() else {
            return;
        };
        let direction = self.units[i].facing;
        let (target_distance, target_angle) = match self.units[i].target {
            Some(id) => match self.index_of(id) {
                Some(tidx) => {
                    let dx = self.units[tidx].x - self.units[i].x;
                    let dy = self.units[tidx].y - self.units[i].y;
                    let d = crate::sim::isqrt((dx as i64) * (dx as i64) + (dy as i64) * (dy as i64))
                        as i32
                        / fp(1);
                    let a = direction_byte(dx, dy) as i32 * 8;
                    (Some(d), Some(a))
                }
                None => (None, None),
            },
            None => (None, None),
        };
        let mut ctx = AnimContext {
            direction,
            tileset: 0,
            rng: self.rng as u32,
            target_distance,
            target_angle,
            lifted: false,
            powerup_picked: false,
            hallucination: self.units[i].status.hallucination,
        };
        let events = self.data.iscript.tick_anim(&mut st, &mut ctx);
        self.rng = (self.rng & 0xffff_ffff_0000_0000) | ctx.rng as u64;
        let finished = st.ended || st.ignore_rest;
        if finished {
            self.units[i].anim = None;
        } else {
            self.units[i].anim = Some(st);
        }
        for ev in events {
            self.handle_anim_event(i, ev);
        }
    }

    pub(super) fn handle_anim_event(&mut self, i: usize, ev: AnimEvent) {
        match ev {
            AnimEvent::Attack {
                weapon,
                melee,
                ground,
            } => self.animation_attack(i, weapon, melee, ground),
            AnimEvent::AttackShift(distance) => self.animation_attack_shift(i, distance),
            AnimEvent::CastSpell => self.animation_cast(i),
            // `orderdone <flag>` only clears bits of the unit's order
            // signal (the counterpart of `sigorder`); it does not finish the
            // order.  Treating it as completion made the Medic, whose
            // Walking script starts with `orderdone 1`, drop every order
            // the moment it started to walk.
            AnimEvent::OrderDone(_) => {}
            AnimEvent::SetDirection(d) => {
                self.units[i].facing = d.clamp(0, 255) as u8;
            }
            AnimEvent::Turn(d) => {
                self.units[i].facing = (self.units[i].facing as i32 + d).rem_euclid(256) as u8;
            }
            AnimEvent::Flip(_)
            | AnimEvent::Frame { .. }
            | AnimEvent::Sound(_)
            | AnimEvent::RandomSound(_)
            | AnimEvent::Move(_)
            | AnimEvent::SetFlingSpeed(_)
            | AnimEvent::Signal(_)
            | AnimEvent::Visible(_)
            | AnimEvent::Overlay { .. }
            | AnimEvent::End
            | AnimEvent::IgnoreRest => {}
        }
    }

    pub(super) fn animation_attack(
        &mut self,
        i: usize,
        weapon: Option<u8>,
        melee: bool,
        ground: bool,
    ) {
        let Some(target) = self.units[i].target else {
            return;
        };
        let Some(tidx) = self.index_of(target) else {
            return;
        };
        if !self.units[tidx].alive {
            return;
        }
        let ty = self.data.unit(self.units[i].type_id).clone();
        let selected = if let Some(id) = weapon {
            self.data.weapon(id).cloned().map(|w| (id, w))
        } else if ground {
            self.data
                .ground_weapon(&ty)
                .cloned()
                .map(|w| (self.data.ground_weapon_id(&ty), w))
        } else {
            self.weapon_for(i, tidx)
        };
        let Some((weapon_id, w)) = selected else {
            self.special_attack(i, tidx);
            return;
        };
        if melee {
            self.fire_melee(i, tidx, weapon_id, &w);
        } else {
            self.fire(i, tidx, weapon_id, &w);
        }
        self.units[i].attacks_fired = self.units[i].attacks_fired.saturating_add(1);
    }

    /// `attkshiftproj`: one bullet of the ground weapon `distance` px ahead
    /// of the attacker, along the line to its target.  The Firebat's attack
    /// runs it three times (24, 52 and 80 px): each flame is an enemy-only
    /// splash of the full weapon damage, so a target next to the Firebat is
    /// burnt by the first two.
    pub(super) fn animation_attack_shift(&mut self, i: usize, distance: u8) {
        let Some(tidx) = self.units[i].target.and_then(|t| self.index_of(t)) else {
            return;
        };
        if !self.units[tidx].alive {
            return;
        }
        let ty = self.data.unit(self.units[i].type_id).clone();
        let Some(w) = self.data.ground_weapon(&ty).cloned() else {
            return;
        };
        let weapon_id = self.data.ground_weapon_id(&ty);
        // The cooldown started with the attack animation (OpenBW
        // `attack_unit_fire_weapon`); each flame only releases damage.
        if self.units[i].cooldown == 0 {
            self.units[i].cooldown = self.fire_cooldown(i, &w);
        }
        self.units[i].attacks_fired = self.units[i].attacks_fired.saturating_add(1);
        if self.units[i].status.hallucination {
            return;
        }
        let (ax, ay) = (self.units[i].x, self.units[i].y);
        let (dx, dy) = (
            (self.units[tidx].x - ax) as i64,
            (self.units[tidx].y - ay) as i64,
        );
        let len = isqrt(dx * dx + dy * dy).max(1);
        let reach = distance as i64 * fp(1) as i64;
        let impact = (
            ax + (dx * reach / len) as i32,
            ay + (dy * reach / len) as i32,
        );
        let owner = self.units[i].owner;
        let target_id = self.units[tidx].id;
        self.instant_hit(owner, Some(target_id), i, weapon_id, &w, impact);
    }

    pub(super) fn animation_cast(&mut self, i: usize) {
        let order = self.units[i].order.clone();
        if let Order::Cast { spell, target } = order {
            let id = self.units[i].id;
            // Unrecorded: the `Order::Cast` that led here is the logged act.
            let _ = self.cast_unrecorded(id, spell, target);
            self.order_done(i);
        }
    }

    pub(super) fn ensure_walking_animation(&mut self, i: usize) {
        // Any resting state starts the walk.  Only Init and the *ToIdle
        // states used to qualify, so a unit whose script sat in Init (map
        // placed units) or IsWorking (an SCV back from mining) kept
        // `move_speed` 0 and crawled on the walk-cycle fallback (Wasteland,
        // 2026-11-06: an SCV ordered to build moved ~0.5 px/tick instead of
        // 5; a Marine group stood still for 1,500 ticks).
        let current = self.units[i].anim.as_ref().map(|a| a.anim as usize);
        let busy = matches!(
            current,
            Some(
                crate::iscript::ANIM_WALKING
                    | crate::iscript::ANIM_DEATH
                    | crate::iscript::ANIM_GND_ATTACK_INIT
                    | crate::iscript::ANIM_AIR_ATTACK_INIT
                    | crate::iscript::ANIM_GND_ATTACK_REPEAT
                    | crate::iscript::ANIM_AIR_ATTACK_REPEAT
                    | crate::iscript::ANIM_CAST_SPELL
                    | crate::iscript::ANIM_BURROW
                    | crate::iscript::ANIM_UNBURROW
                    | crate::iscript::ANIM_LANDING
                    | crate::iscript::ANIM_LIFT_OFF
                    | crate::iscript::ANIM_WARP_IN
            )
        );
        if !busy {
            self.start_animation(i, ANIM_WALKING);
        }
    }

    pub(super) fn start_attack_animation(&mut self, i: usize, flying_target: bool) -> bool {
        let repeat = self.units[i].attacks_fired > 0;
        let anim = match (flying_target, repeat) {
            (true, false) => ANIM_AIR_ATTACK_INIT,
            (true, true) => ANIM_AIR_ATTACK_REPEAT,
            (false, false) => ANIM_GND_ATTACK_INIT,
            (false, true) => ANIM_GND_ATTACK_REPEAT,
        };
        let ty = self.data.unit(self.units[i].type_id);
        let Some(iscript) = self.data.unit_iscript_id(ty) else {
            return false;
        };
        if !self.data.iscript.animation_has_attack(iscript, anim) {
            return false;
        }
        self.start_animation(i, anim);
        true
    }
}
