#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent18.rs
// ===========================================================================
// Translation batch 18.  Only this file is edited by the batch.
//
// Ten `decompiled_uncited` rows from `0x005c8000..=0x005cbfff` whose
// decompiled body is at most 100 bytes and whose observable state is already
// exposed by [`crate::sim::World`] / [`crate::sim::SimUnit`].
//
// The block is part of the retail order-handler preamble.  Several routines
// end their failure paths in the non-returning assertion helper; those are
// reported as `Result::Err(())` instead of terminating the host process, so
// the recovered guard stays testable.  Helper routines that were already
// recovered in `sim.rs` are called through their public methods; helpers that
// were not recovered are described by role only and never addressed here.

/// Recovered "unit carries a disabling status" test used by the order-handler
/// assertions: `CUnit+0xdc` bit 10, or a running Lockdown (`CUnit+0x117`),
/// Stasis Field (`CUnit+0x119`) or Maelstrom (`CUnit+0x124`) timer.
fn a18_status_disabled(unit: &SimUnit) -> bool {
    unit.flag_10
        || unit.status.lockdown_ticks > 0
        || unit.status.stasis_ticks > 0
        || unit.status.maelstrom_ticks > 0
}

/// Recovered "target is cloaked and invisible to `other_owner`" test: the
/// unit's cloak state (`CUnit+0xdc` bits 8-9) is non-zero and the `CUnit+0xe4`
/// visibility mask lacks the other owner's bit.
fn a18_cloaked_from(unit: &SimUnit, other_owner: u8) -> bool {
    unit.cloak_state() != 0 && unit.visibility_mask_and(1u32 << (other_owner & 31)) == 0
}

impl World {
    /// `0x005c8070`: Shield Battery target pre-check.
    ///
    /// The recovered body returns zero while the battery already carries the
    /// shield-recharge flag or the dying bit of `CUnit+0xdc` (bit 30, which
    /// the Rust model does not retain); otherwise it forwards the candidate
    /// target to the Shield Battery target test, which requires a different
    /// living unit on the same side with a plasma-shield pool.
    pub fn a18_shield_battery_target_ok(&self, battery: usize, target: usize) -> bool {
        let (Some(b), Some(t)) = (self.units.get(battery), self.units.get(target)) else {
            return false;
        };
        if b.shield_recharge_flag() {
            return false;
        }
        self.can_shield_battery_target(b.id, t.id)
    }

    /// `0x005c8130`: guard for the `0x20` target filter.
    ///
    /// When the relationship gate (`allied`) accepts the pair and the retail
    /// target filter accepts the unit, the routine aborts unless the unit is
    /// already carrying a disabling status.  The normal retail path returns
    /// zero unconditionally; here the abort path is `Err(())`.
    pub fn a18_disabled_target_guard(
        &self,
        unit: usize,
        allied: bool,
        targetable: bool,
    ) -> Result<(), ()> {
        let Some(u) = self.units.get(unit) else {
            return Err(());
        };
        if allied && targetable && !a18_status_disabled(u) {
            Err(())
        } else {
            Ok(())
        }
    }

    /// `0x005c81c0`: per-unit accumulator step of the area enumeration.
    ///
    /// A unit contributes the whole-point energy byte (`CUnit+0xa3`) when its
    /// `CUnit+0xdc` bit 26 is clear, the recovered relationship test accepts
    /// the pair (`hostile`), and the type carries the `units.dat` energy flag.
    /// Retail adds the byte to a module accumulator and always returns zero,
    /// so [`Option`] reports whether a contribution was made.
    pub fn a18_energy_accumulator_step(&self, unit: usize, hostile: bool) -> Option<u32> {
        let u = self.units.get(unit)?;
        if u.flag_26() || !hostile || !self.unit_has_energy(unit) {
            return None;
        }
        Some(((u.energy_fixed() >> 8) & 0xff) as u32)
    }

    /// `0x005c8200`: guard asserting the target carries the hero flag.
    ///
    /// The recovered body aborts when the relationship gate accepts the pair
    /// (`allied`) and the `units.dat` hero flag (0x40) is clear, and returns
    /// zero on every other path.  The abort path is `Err(())`.
    pub fn a18_hero_target_guard(&self, unit: usize, allied: bool) -> Result<(), ()> {
        let Some(u) = self.units.get(unit) else {
            return Err(());
        };
        if allied && !self.data.unit(u.type_id).is_hero() {
            Err(())
        } else {
            Ok(())
        }
    }

    /// `0x005c8470`: guard asserting a running Maelstrom on an organic target.
    ///
    /// Retail walks the relationship gate (`allied`), the `0x6c` target
    /// filter (`targetable`) and the `units.dat` organic flag before aborting
    /// when the Maelstrom timer (`CUnit+0x124`) is zero.  The abort path is
    /// `Err(())`; the normal path returns zero unconditionally.
    pub fn a18_organic_maelstrom_guard(
        &self,
        unit: usize,
        allied: bool,
        targetable: bool,
    ) -> Result<(), ()> {
        let Some(u) = self.units.get(unit) else {
            return Err(());
        };
        if allied
            && targetable
            && self.data.unit(u.type_id).is_organic()
            && u.maelstrom_ticks() == 0
        {
            Err(())
        } else {
            Ok(())
        }
    }

    /// `0x005c8990`: shielded-unit HP accumulator step of the area
    /// enumeration.
    ///
    /// A unit contributes its whole-point HP (`CUnit+0x60 >> 8`) when its
    /// `CUnit+0xdc` bit 26 is clear, the recovered relationship test accepts
    /// the pair (`hostile`), and the type carries the `units.dat` shield flag.
    /// Retail adds the value to a module accumulator and returns zero, so
    /// [`Option`] reports whether a contribution was made.
    pub fn a18_shielded_hp_accumulator_step(&self, unit: usize, hostile: bool) -> Option<i32> {
        let u = self.units.get(unit)?;
        if u.flag_26() || !hostile || !self.unit_has_shields(u.type_id) {
            return None;
        }
        Some(u.hp >> 8)
    }

    /// `0x005c8ad0`: relationship gate shared by the order handlers.
    ///
    /// Three recovered checks decide whether a handler may apply its effect
    /// to another unit: the acting unit's owner bit must be present in the
    /// player mask (`player_enabled`, kept as a caller input because the mask
    /// is shell/player state), the acting unit must not be cloaked-and-hidden
    /// from the other owner, and the two owners must be hostile in the
    /// alliance table.  Retail aborts through its assertion helper on the
    /// non-hostile path, so only the hostile path continues.
    pub fn a18_hostile_target_gate(
        &self,
        actor: usize,
        other: usize,
        player_enabled: bool,
    ) -> bool {
        let (Some(a), Some(o)) = (self.units.get(actor), self.units.get(other)) else {
            return false;
        };
        player_enabled && !a18_cloaked_from(a, o.owner) && self.is_enemy(a.owner, o.owner)
    }

    /// `0x005cbe10`: HUD-plate refresh epilogue for an ordered unit.
    ///
    /// Retail looks the unit up in the plate list, asserts that the unit still
    /// carries a disabling status (clearing `CUnit+0xdc` bit 10 first), does
    /// sprite work for workers, and finally writes `0xf` to `CUnit+0x85`, the
    /// per-unit order carry that restarts the carry-gated order handlers.  The
    /// plate/sprite work is presentation-only and not modelled.  Returns
    /// `false` on the recovered assertion path (unit not disabled), after the
    /// same bit-10 clear the retail code performs before aborting.
    pub fn a18_plate_refresh_reset_carry(&mut self, unit: usize) -> bool {
        if unit >= self.units.len() {
            return false;
        }
        if !a18_status_disabled(&self.units[unit]) {
            self.units[unit].clear_flag_10();
            return false;
        }
        self.units[unit].order_carry = 0xf;
        true
    }
}

/// `0x005c81fc`: empty routine.  The recovered body is a bare `return` with
/// no reads or writes; it is kept as an explicit no-op.
pub fn a18_empty_routine() {}

/// `0x005c83d0`: unconditional assertion failure.  The recovered body has no
/// normal return and hands control straight to the non-returning assertion
/// helper.  Modelled as the failed guard so callers and tests can observe it.
pub fn a18_unconditional_failure() -> Result<(), ()> {
    Err(())
}

#[cfg(test)]
mod tests_agent18 {
    use super::*;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(crate::data::GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    #[test]
    fn shield_battery_precheck_requires_a_shielded_same_owner_target() {
        let mut w = world();
        let battery = w.spawn(0, uid::SHIELD_BATTERY, 320, 320).unwrap();
        let zealot = w.spawn(0, uid::ZEALOT, 360, 320).unwrap();
        let marine = w.spawn(0, uid::MARINE, 380, 320).unwrap();
        let enemy = w.spawn(1, uid::ZEALOT, 420, 320).unwrap();

        assert!(w.a18_shield_battery_target_ok(battery, zealot));
        assert!(!w.a18_shield_battery_target_ok(battery, marine));
        assert!(!w.a18_shield_battery_target_ok(battery, enemy));
        assert!(!w.a18_shield_battery_target_ok(battery, battery));

        w.units[battery].shield_recharged = true;
        assert!(!w.a18_shield_battery_target_ok(battery, zealot));
        assert!(w.units[battery].shield_recharge_flag());
    }

    #[test]
    fn disabled_target_guard_aborts_only_on_active_allied_targets() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();

        assert_eq!(w.a18_disabled_target_guard(marine, true, true), Err(()));
        assert_eq!(w.a18_disabled_target_guard(marine, false, true), Ok(()));
        assert_eq!(w.a18_disabled_target_guard(marine, true, false), Ok(()));

        w.units[marine].set_flag_10();
        assert_eq!(w.a18_disabled_target_guard(marine, true, true), Ok(()));
        w.units[marine].clear_flag_10();

        w.units[marine].status.apply_lockdown(0x83);
        assert_eq!(w.a18_disabled_target_guard(marine, true, true), Ok(()));
        w.units[marine].status.clear_lockdown();

        w.units[marine].status.apply_stasis(3);
        assert_eq!(w.a18_disabled_target_guard(marine, true, true), Ok(()));
        w.units[marine].status.clear_stasis();

        w.units[marine].status.apply_maelstrom(5);
        assert_eq!(w.a18_disabled_target_guard(marine, true, true), Ok(()));
    }

    #[test]
    fn energy_accumulator_step_gates_flag_relationship_and_energy_flag() {
        let mut w = world();
        let vessel = w.spawn(0, uid::SCIENCE_VESSEL, 320, 320).unwrap();
        let marine = w.spawn(0, uid::MARINE, 360, 320).unwrap();
        assert!(w.unit_has_energy(vessel));

        w.units[vessel].set_energy_points(75);
        assert_eq!(w.units[vessel].energy_fixed() >> 8, 75);
        assert_eq!(w.a18_energy_accumulator_step(vessel, true), Some(75));
        assert_eq!(w.a18_energy_accumulator_step(vessel, false), None);
        assert_eq!(w.a18_energy_accumulator_step(marine, true), None);

        w.units[vessel].flag_26 = true;
        assert_eq!(w.a18_energy_accumulator_step(vessel, true), None);
        w.clear_unsupported_flag_26(vessel);

        w.units[vessel].energy = 300;
        w.units[vessel].energy_frac = 0;
        assert_eq!(w.a18_energy_accumulator_step(vessel, true), Some(44));
    }

    #[test]
    fn hero_target_guard_aborts_on_non_heroes() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert_eq!(w.a18_hero_target_guard(marine, true), Err(()));
        assert_eq!(w.a18_hero_target_guard(marine, false), Ok(()));

        let hero = (0..w.data.units.len())
            .find(|&t| w.data.unit(t as u16).is_hero())
            .expect("bundled data has a hero") as u16;
        let hero = w.spawn(0, hero, 640, 640).unwrap();
        assert_eq!(w.a18_hero_target_guard(hero, true), Ok(()));
    }

    #[test]
    fn organic_maelstrom_guard_requires_a_running_timer() {
        let mut w = world();
        let zergling = w.spawn(0, uid::ZERGLING, 320, 320).unwrap();
        let battlecruiser = w.spawn(0, uid::BATTLECRUISER, 360, 320).unwrap();
        assert!(w.data.unit(uid::ZERGLING).is_organic());
        assert!(!w.data.unit(uid::BATTLECRUISER).is_organic());

        assert_eq!(w.a18_organic_maelstrom_guard(zergling, true, true), Err(()));
        assert_eq!(w.a18_organic_maelstrom_guard(zergling, true, false), Ok(()));
        assert_eq!(w.a18_organic_maelstrom_guard(zergling, false, true), Ok(()));
        assert_eq!(
            w.a18_organic_maelstrom_guard(battlecruiser, true, true),
            Ok(())
        );

        w.units[zergling].status.apply_maelstrom(5);
        assert_eq!(w.a18_organic_maelstrom_guard(zergling, true, true), Ok(()));
    }

    #[test]
    fn shielded_hp_accumulator_step_gates_flag_relationship_and_shields() {
        let mut w = world();
        let zealot = w.spawn(0, uid::ZEALOT, 320, 320).unwrap();
        let marine = w.spawn(0, uid::MARINE, 360, 320).unwrap();
        assert!(w.unit_has_shields(uid::ZEALOT));
        assert!(!w.unit_has_shields(uid::MARINE));

        w.units[zealot].hp = 42 << 8;
        assert_eq!(w.a18_shielded_hp_accumulator_step(zealot, true), Some(42));
        assert_eq!(w.a18_shielded_hp_accumulator_step(zealot, false), None);
        assert_eq!(w.a18_shielded_hp_accumulator_step(marine, true), None);

        w.units[zealot].flag_26 = true;
        assert_eq!(w.a18_shielded_hp_accumulator_step(zealot, true), None);
    }

    #[test]
    fn hostile_target_gate_requires_enemy_and_visibility() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let enemy = w.spawn(1, uid::MARINE, 360, 320).unwrap();
        let ally = w.spawn(0, uid::MARINE, 400, 320).unwrap();

        assert!(w.a18_hostile_target_gate(marine, enemy, true));
        assert!(!w.a18_hostile_target_gate(marine, ally, true));
        assert!(!w.a18_hostile_target_gate(marine, enemy, false));

        w.units[marine].status.cloaked = true;
        w.units[marine].visibility_mask = 0;
        assert!(!w.a18_hostile_target_gate(marine, enemy, true));

        w.units[marine].visibility_mask = 2;
        assert!(w.a18_hostile_target_gate(marine, enemy, true));
    }

    #[test]
    fn plate_refresh_resets_carry_only_when_the_unit_is_disabled() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        w.units[marine].order_carry = 3;
        assert!(!w.a18_plate_refresh_reset_carry(marine));
        assert_eq!(w.units[marine].order_carry, 3);

        w.units[marine].set_flag_10();
        assert!(w.a18_plate_refresh_reset_carry(marine));
        assert_eq!(w.units[marine].order_carry, 0xf);
    }

    #[test]
    fn empty_routine_changes_nothing() {
        let mut w = world();
        w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let before = w.units.len();
        a18_empty_routine();
        assert_eq!(w.units.len(), before);
        assert_eq!(w.tick, 0);
    }

    #[test]
    fn unconditional_failure_reports_the_assertion_path() {
        assert_eq!(a18_unconditional_failure(), Err(()));
    }
}
