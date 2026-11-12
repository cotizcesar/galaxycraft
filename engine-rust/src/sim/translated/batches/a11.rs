#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent11.rs
// ===========================================================================
// Translation batch 11.  Only this file is edited by the batch.
//
// The batch covers the `decompiled_uncited` entries of the
// `0x005a0000..=0x005a3fff` range whose ledger size is at most 100 bytes.
// That window is retail unit-order/attack helper code; most of the ledger
// rows in it are not function entries (Ghidra split single routines at
// call sites and jump targets), so only rows that are real function starts
// are reconstructed here.
//
// The recovered semantics use the existing [`SimUnit`] order/target
// storage (`CUnit+0x5c`, `CUnit+0x64`), [`SimUnit::facing`] and the
// `units.dat`/`weapons.dat` columns exposed through
// [`crate::data::GameData`].  Two retail links are not modelled by the sim
// and are therefore passed in explicitly instead of stored: the
// `CUnit+0x70` subunit pointer and the `CUnit+0x4e` order-phase byte.

/// Byte direction from `(x1, y1)` towards `(x2, y2)` in the retail compass
/// convention: 0 = north, 64 = east, 128 = south, 192 = west.
///
/// The retail direction routine divides the deltas and feeds an integer
/// arctangent table; this uses the same convention via `atan2`, which can
/// differ from the table entry by one byte step.  The axis-exact cases are
/// reproduced exactly.
fn a11_direction_byte(dx: i32, dy: i32) -> u8 {
    if dx == 0 {
        return if dy < 1 { 0 } else { 0x80 };
    }
    let angle = (dx as f32).atan2(-(dy as f32));
    let mut a = (angle * 256.0 / std::f32::consts::TAU).round() as i32;
    a = (a + 256) % 256;
    a as u8
}

/// Retail octagonal distance estimate (the integer routine the attack
/// helpers use instead of a square root): `dx`/`dy` are absolute deltas.
///
/// The large axis and the small axis are ordered first; for diagonal
/// deltas the result is `large + small * 3 / 8` with the small correction
/// terms the retail routine applies.
fn a11_octagonal_distance(dx: i32, dy: i32) -> i32 {
    let mut large = dx.abs();
    let mut small = dy.abs();
    if large < small {
        std::mem::swap(&mut large, &mut small);
    }
    if large >> 2 < small {
        let adjusted = (small >> 1) + small;
        return (adjusted >> 2) + (((adjusted >> 7) - (large >> 6)) - (large >> 4)) + large;
    }
    large
}

impl World {
    /// `0x005a27d0`: pick the attack weapon for the current order target
    /// and report how many attack cues the attack-cue emission routine
    /// produces for it.
    ///
    /// The routine returns immediately when the order's unit-target slot
    /// (`CUnit+0x5c`) is empty.  `air` selects the retail air branch: the
    /// selector value 1 takes the ground weapon, and any other value
    /// (normally 2) takes the air weapon.  In the ground case an
    /// incomplete Lurker (`unit type 0x67` with `CUnit+0xdc` bit 4 clear)
    /// yields the `0x82` "no weapon" sentinel.  The selected weapon is cued
    /// once, and a second time when the `weapons.dat` damage-factor byte
    /// (exposed as [`crate::data::WeaponType::bullet_count`]) equals 2.
    ///
    /// Returns `None` when the unit has no order unit-target, otherwise
    /// `Some((weapon_id, cue_count))`.
    pub fn a11_attack_weapon_cues(&self, unit: &SimUnit, air: bool) -> Option<(u8, u8)> {
        unit.order_unit_target()?;
        let stats = self.data.unit(unit.type_id);
        let weapon = if air {
            stats.air_weapon
        } else if unit.type_id == uid::LURKER && !unit.completed {
            0x82
        } else {
            stats.ground_weapon
        };
        let cues = if self
            .data
            .weapon(weapon)
            .is_some_and(|w| w.bullet_count == 2)
        {
            2
        } else {
            1
        };
        Some((weapon, cues))
    }

    /// `0x005a2d90`: air weapon of the unit, falling back to the air weapon
    /// of its attached subunit.
    ///
    /// The routine reads the `units.dat` air weapon of the unit.  When that
    /// is the `0x82` sentinel it follows `CUnit+0x70`; a null link returns
    /// `0x82`, otherwise the air weapon of the linked unit is read.  The
    /// `CUnit+0x70` subunit pointer is not part of [`SimUnit`], so the
    /// linked unit is supplied by the caller.  (Ghidra split this routine
    /// into two ledger rows; the body after the split is part of the same
    /// method.)
    pub fn a11_air_weapon_with_subunit(&self, unit: &SimUnit, subunit: Option<&SimUnit>) -> u8 {
        let own = self.data.unit(unit.type_id).air_weapon;
        if own != 0x82 {
            return own;
        }
        match subunit {
            Some(link) => self.data.unit(link.type_id).air_weapon,
            None => 0x82,
        }
    }

    /// `0x005a3ed0`: minimum-range test used by the attack-target
    /// validation path.
    ///
    /// The effective target is the `CUnit+0x70` link when that link carries
    /// the `units.dat` subunit flag (`0x10`), otherwise the unit itself.
    /// The weapon is then selected exactly like `0x005a27d0`: the air weapon
    /// when the *source* unit is airborne (dynamic `CUnit+0xdc` bit 2),
    /// otherwise the ground weapon, with an incomplete Lurker collapsing to
    /// `0x82`.  A `0x82` weapon or a zero `weapons.dat` minimum range
    /// returns `false`; otherwise the routine reports whether the gap
    /// between the two unit rectangles is at most the weapon's minimum
    /// range.
    ///
    /// `target_subunit` stands in for the unmodelled `CUnit+0x70` link.
    pub fn a11_min_range_reached(
        &self,
        target: &SimUnit,
        target_subunit: Option<&SimUnit>,
        source: &SimUnit,
    ) -> bool {
        let effective = match target_subunit {
            Some(link) if self.data.unit(link.type_id).is_subunit() => link,
            _ => target,
        };
        let source_airborne = source.lifted || self.data.unit(source.type_id).is_flyer();
        let weapon = if source_airborne {
            self.data.unit(effective.type_id).air_weapon
        } else if effective.type_id == uid::LURKER && !effective.completed {
            0x82
        } else {
            self.data.unit(effective.type_id).ground_weapon
        };
        if weapon == 0x82 {
            return false;
        }
        let Some(stats) = self.data.weapon(weapon) else {
            return false;
        };
        if stats.min_range == 0 {
            return false;
        }
        let target_rect = self.a11_unit_rect(effective);
        let source_rect = self.a11_unit_rect(source);
        let gap_x = (target_rect.0 - source_rect.2)
            .max(source_rect.0 - target_rect.2)
            .max(0);
        let gap_y = (target_rect.1 - source_rect.3)
            .max(source_rect.1 - target_rect.3)
            .max(0);
        a11_octagonal_distance(gap_x, gap_y) <= stats.min_range as i32
    }

    /// Helper for `0x005a3ed0`: the `units.dat` rectangle of `unit` with
    /// the right/bottom edges advanced by one to the exclusive form the
    /// retail gap calculation uses.
    fn a11_unit_rect(&self, unit: &SimUnit) -> (i32, i32, i32, i32) {
        let (left, top, right, bottom) = self
            .data
            .unit(unit.type_id)
            .dimensions
            .rect_at((unit.x / fp(1), unit.y / fp(1)));
        (left, top, right + 1, bottom + 1)
    }
}

impl SimUnit {
    /// `0x005a31a0`: is the target inside the weapon's firing arc?
    ///
    /// The routine computes the direction byte from the unit's position to
    /// the target and, for a Lurker (`unit type 0x67`), writes that byte to
    /// `CUnit+0x21` and accepts immediately.  Every other unit measures the
    /// wrapped absolute difference between its facing (`CUnit+0x21`) and
    /// the direction, and compares it with the `weapons.dat` attack-angle
    /// byte (exposed as [`crate::data::WeaponType::attack_angle`]): the
    /// target is in the arc when `min(diff, 256 - diff) <= attack_angle`.
    ///
    /// `attack_angle` is passed in because the caller has the weapon row;
    /// only [`SimUnit::facing`] is written here.  `target` is in whole
    /// pixels, while [`SimUnit::x`]/[`SimUnit::y`] are 1/256-pixel fixed
    /// point and are converted first.
    pub fn a11_attack_angle_ok(&mut self, attack_angle: u8, target: (i32, i32)) -> bool {
        let dx = target.0 - self.x / fp(1);
        let dy = target.1 - self.y / fp(1);
        let angle = a11_direction_byte(dx, dy);
        if self.type_id == uid::LURKER {
            self.facing = angle;
            return true;
        }
        let diff = (i32::from(self.facing) - i32::from(angle)).rem_euclid(0x100);
        let delta = diff.min(0x100 - diff);
        delta <= i32::from(attack_angle)
    }

    /// `0x005a3150`: predicate used to finish/validate an order step.
    ///
    /// The routine returns true when the unit has no sprite
    /// (`CUnit+0x0c == 0`) or when its primary order id (`CUnit+0x4d`) is
    /// zero and its order-phase byte (`CUnit+0x4e`) is 1.  The sim maps the
    /// missing-sprite and zero-order cases onto [`SimUnit::alive`] and
    /// [`Order::Idle`] (the same mapping the already-translated
    /// sprite/order predicate uses); the order-phase byte is not stored, so
    /// the caller supplies it.
    pub fn a11_sprite_missing_or_order_phase(&self, order_phase: u8) -> bool {
        !self.alive || (matches!(self.order, Order::Idle) && order_phase == 1)
    }
}

#[cfg(test)]
mod tests_agent11 {
    use super::*;
    use crate::data::GameData;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, type_id: u16, x: i32, y: i32) -> u32 {
        let i = world.spawn_px(0, type_id, x, y).unwrap();
        world.units[i].id
    }

    fn attach_target(world: &mut World, unit: u32, target: u32) {
        let i = world.index_of(unit).unwrap();
        world.units[i].order = Order::AttackUnit { target };
    }

    #[test]
    fn attack_weapon_cues_need_a_unit_target() {
        let mut w = open_world();
        let marine = spawn(&mut w, uid::MARINE, 640, 640);
        let enemy = spawn(&mut w, uid::MARINE, 700, 640);
        let i = w.index_of(marine).unwrap();
        assert_eq!(w.a11_attack_weapon_cues(&w.units[i], false), None);
        assert_eq!(w.a11_attack_weapon_cues(&w.units[i], true), None);

        attach_target(&mut w, marine, enemy);
        let (weapon, cues) = w.a11_attack_weapon_cues(&w.units[i], false).unwrap();
        assert_eq!(weapon, w.data.unit(uid::MARINE).ground_weapon);
        let expected_cues = if w.data.weapon(weapon).unwrap().bullet_count == 2 {
            2
        } else {
            1
        };
        assert_eq!(cues, expected_cues);
        assert!(w.units[i].order_unit_target().is_some());
    }

    #[test]
    fn attack_weapon_cues_select_ground_air_and_repeat_count() {
        let mut w = open_world();
        let marine = spawn(&mut w, uid::MARINE, 640, 640);
        let enemy = spawn(&mut w, uid::MARINE, 700, 640);
        let i = w.index_of(marine).unwrap();
        attach_target(&mut w, marine, enemy);

        let (ground, ground_cues) = w.a11_attack_weapon_cues(&w.units[i], false).unwrap();
        let (air, air_cues) = w.a11_attack_weapon_cues(&w.units[i], true).unwrap();
        assert_eq!(ground, w.data.unit(uid::MARINE).ground_weapon);
        assert_eq!(air, w.data.unit(uid::MARINE).air_weapon);
        let damage_factor = w.data.weapon(ground).unwrap().bullet_count;
        assert_eq!(ground_cues, if damage_factor == 2 { 2 } else { 1 });
        assert_eq!(
            air_cues,
            if w.data.weapon(air).unwrap().bullet_count == 2 {
                2
            } else {
                1
            }
        );

        // A weapon whose damage-factor byte is 2 is cued twice.  There is at
        // least one such ground weapon in the shipped table.
        let repeat_type = (0..w.data.units.len() as u16)
            .find(|&t| {
                w.data
                    .weapon(w.data.unit(t).ground_weapon)
                    .is_some_and(|s| s.bullet_count == 2)
            })
            .expect("a ground weapon with a damage factor of 2");
        let repeater = spawn(&mut w, repeat_type, 600, 600);
        attach_target(&mut w, repeater, enemy);
        let j = w.index_of(repeater).unwrap();
        assert_eq!(
            w.a11_attack_weapon_cues(&w.units[j], false),
            Some((w.data.unit(repeat_type).ground_weapon, 2))
        );
    }

    #[test]
    fn attack_weapon_cues_incomplete_lurker_has_no_weapon() {
        let mut w = open_world();
        let lurker = spawn(&mut w, uid::LURKER, 640, 640);
        let enemy = spawn(&mut w, uid::MARINE, 700, 640);
        let i = w.index_of(lurker).unwrap();
        attach_target(&mut w, lurker, enemy);
        w.units[i].completed = false;
        assert_eq!(
            w.a11_attack_weapon_cues(&w.units[i], false),
            Some((0x82, 1))
        );

        // The air selector bypasses the Lurker special case entirely, and the
        // Lurker row has the 0x82 air-weapon sentinel.
        assert_eq!(w.data.unit(uid::LURKER).air_weapon, 0x82);
        assert_eq!(w.a11_attack_weapon_cues(&w.units[i], true), Some((0x82, 1)));

        // A completed Lurker uses its ground weapon again.
        w.units[i].completed = true;
        assert_eq!(
            w.a11_attack_weapon_cues(&w.units[i], false),
            Some((w.data.unit(uid::LURKER).ground_weapon, 1))
        );
    }

    #[test]
    fn attack_angle_accepts_exact_facing_and_rejects_opposite() {
        let mut w = open_world();
        let marine = spawn(&mut w, uid::MARINE, 640, 640);
        let i = w.index_of(marine).unwrap();
        w.units[i].facing = 64; // east
        assert!(w.units[i].a11_attack_angle_ok(0, (740, 640)));
        // Due west is 128 byte steps away: 0x7f is outside, 0x80 is not.
        assert!(!w.units[i].a11_attack_angle_ok(0x7f, (540, 640)));
        assert!(w.units[i].a11_attack_angle_ok(0x80, (540, 640)));
        // A target slightly off the current facing is inside angle 0.
        w.units[i].facing = a11_direction_byte(100, 1);
        assert!(w.units[i].a11_attack_angle_ok(0, (740, 641)));
    }

    #[test]
    fn attack_angle_snaps_lurker_facing() {
        let mut w = open_world();
        let lurker = spawn(&mut w, uid::LURKER, 640, 640);
        let i = w.index_of(lurker).unwrap();
        w.units[i].facing = 0;
        assert!(w.units[i].a11_attack_angle_ok(0, (740, 640)));
        assert_eq!(w.units[i].facing, 64);
    }

    #[test]
    fn air_weapon_falls_back_to_the_subunit() {
        let mut w = open_world();
        let firebat = spawn(&mut w, uid::FIREBAT, 640, 640);
        let marine = spawn(&mut w, uid::MARINE, 660, 640);
        let i = w.index_of(firebat).unwrap();
        let j = w.index_of(marine).unwrap();
        assert_eq!(w.data.unit(uid::FIREBAT).air_weapon, 0x82);
        assert_eq!(
            w.a11_air_weapon_with_subunit(&w.units[i], Some(&w.units[j])),
            w.data.unit(uid::MARINE).air_weapon
        );
        assert_eq!(w.a11_air_weapon_with_subunit(&w.units[i], None), 0x82);
        // A unit with its own air weapon never consults the link.
        assert_ne!(w.data.unit(uid::MARINE).air_weapon, 0x82);
        assert_eq!(
            w.a11_air_weapon_with_subunit(&w.units[j], Some(&w.units[i])),
            w.data.unit(uid::MARINE).air_weapon
        );
    }

    #[test]
    fn min_range_reached_uses_rectangle_gap() {
        let mut w = open_world();
        let (target_type, min_range) = (0..w.data.units.len() as u16)
            .find_map(|t| {
                let stats = w.data.weapon(w.data.unit(t).ground_weapon)?;
                (stats.min_range > 0).then_some((t, stats.min_range))
            })
            .expect("a ground weapon with a minimum range");
        let target = spawn(&mut w, target_type, 640, 640);
        let source = spawn(&mut w, uid::MARINE, 646, 640);
        let ti = w.index_of(target).unwrap();
        let si = w.index_of(source).unwrap();
        w.units[ti].completed = true;
        assert!(w.a11_min_range_reached(&w.units[ti], None, &w.units[si]));

        // Push the source beyond the minimum range plus both boxes.
        let far = min_range as i32 + 200;
        w.units[si].x = w.units[ti].x + far * fp(1);
        assert!(!w.a11_min_range_reached(&w.units[ti], None, &w.units[si]));
    }

    #[test]
    fn min_range_reached_ignores_no_weapon_and_zero_min_range() {
        let mut w = open_world();
        let source = spawn(&mut w, uid::MARINE, 646, 640);
        let si = w.index_of(source).unwrap();

        let unarmed_type = (0..w.data.units.len() as u16)
            .find(|&t| w.data.unit(t).ground_weapon == 0x82)
            .expect("a unit type without a ground weapon");
        let unarmed = spawn(&mut w, unarmed_type, 640, 640);
        let ui = w.index_of(unarmed).unwrap();
        assert!(!w.a11_min_range_reached(&w.units[ui], None, &w.units[si]));

        let zero_type = (0..w.data.units.len() as u16)
            .find(|&t| {
                w.data
                    .weapon(w.data.unit(t).ground_weapon)
                    .is_some_and(|s| s.min_range == 0 && s.max_range > 0)
            })
            .expect("a ground weapon with zero minimum range");
        let zero = spawn(&mut w, zero_type, 800, 800);
        let zi = w.index_of(zero).unwrap();
        w.units[zi].completed = true;
        assert!(!w.a11_min_range_reached(&w.units[zi], None, &w.units[si]));
    }

    #[test]
    fn min_range_reached_prefers_a_subunit_target() {
        let mut w = open_world();
        // A marine's ground weapon has no minimum range.
        let target = spawn(&mut w, uid::MARINE, 640, 640);
        let source = spawn(&mut w, uid::MARINE, 646, 640);
        let ti = w.index_of(target).unwrap();
        let si = w.index_of(source).unwrap();
        assert!(!w.a11_min_range_reached(&w.units[ti], None, &w.units[si]));

        // A `units.dat` subunit row with a minimum-range ground weapon.
        let subunit_type = (0..w.data.units.len() as u16)
            .find(|&t| {
                w.data.unit(t).is_subunit()
                    && w.data
                        .weapon(w.data.unit(t).ground_weapon)
                        .is_some_and(|s| s.min_range > 0)
            })
            .expect("a subunit with a minimum-range ground weapon");
        let subunit = spawn(&mut w, subunit_type, 660, 640);
        let ci = w.index_of(subunit).unwrap();
        assert!(w.a11_min_range_reached(&w.units[ti], Some(&w.units[ci]), &w.units[si]));
        // A non-subunit link is ignored, leaving the marine's zero min range.
        let plain = spawn(&mut w, uid::FIREBAT, 620, 640);
        let fi = w.index_of(plain).unwrap();
        assert!(!w.a11_min_range_reached(&w.units[ti], Some(&w.units[fi]), &w.units[si]));
    }

    #[test]
    fn sprite_missing_or_order_phase_predicate() {
        let mut w = open_world();
        let marine = spawn(&mut w, uid::MARINE, 640, 640);
        let i = w.index_of(marine).unwrap();
        assert!(matches!(w.units[i].order, Order::Idle));
        assert!(w.units[i].a11_sprite_missing_or_order_phase(1));
        assert!(!w.units[i].a11_sprite_missing_or_order_phase(0));
        assert!(!w.units[i].a11_sprite_missing_or_order_phase(2));

        w.units[i].order = Order::Move { pos: (700, 700) };
        assert!(!w.units[i].a11_sprite_missing_or_order_phase(1));

        // No sprite is modelled as a dead unit; then the phase is irrelevant.
        w.units[i].alive = false;
        assert!(w.units[i].a11_sprite_missing_or_order_phase(0));
        assert!(w.units[i].a11_sprite_missing_or_order_phase(2));
    }
}
