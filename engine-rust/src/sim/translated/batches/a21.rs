#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent21.rs
// ===========================================================================
// Translation batch 21.  Only this file is edited by the batch.
//
// The batch covers the `decompiled_uncited` ledger rows in
// `0x005a0000..=0x005a1fff`.  The window is the retail order-handler cluster:
// the Computer AI handler (row 152), Teleport (row 135), Sap Location
// (row 134), Nothing (row 22) and the attack range/facing helper at
// `0x005a07b0`.  Most ledger rows in the window are not function entries:
// Ghidra split the handlers at call and jump targets, and several rows fall
// inside an instruction or a jump-table datum.  Each method below names its
// exact ledger address and says which recovered branch it reproduces.  Rows
// that are pure data or have no recoverable behavior at all are reported as
// skipped; a row whose only recovered effect is a fatal halt is kept as an
// explicit fatal hook rather than silently ignored.
//
// Retail state that [`SimUnit`] does not model (`CUnit+0x4e` phase,
// `CUnit+0x4f` end flag, the `CUnit+0x54` wait-byte array, `CUnit+0x20`
// flag bit 3, the `CUnit+0x134` link and the sprite destination
// `0x28/0x2a`) is supplied by the caller or returned.  No new persistent
// fields are added.

/// Control-flow result of the Teleport order handler (row 135) for one
/// `CUnit+0x4e` phase byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A21TeleportStep {
    /// The handler returns without changing the phase.
    Return,
    /// Phase 0 accepted the destination and stores phase 1.
    ToPhase1,
    /// Phase 1 accepted the approach check, sets the order target to self,
    /// emits cue `0x0d`, sets the hidden bit and stores phase 2.
    ToPhase2,
    /// Phase 2 with the `CUnit+0x4f` bit-0 end flag runs the cleanup path.
    FinishOrder,
}

/// Control-flow result of the Nothing handler's click-order match
/// (`0x005a04cb`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A21NothingTail {
    /// The lookup did not match, or the linked unit is a completed Lurker:
    /// retail plays cue 4.
    Cue4,
    /// The lookup matched but the linked unit is a flyer: retail returns
    /// without a cue and without the range fallback.
    QuietReturn,
    /// The lookup matched and the linked unit is not a flyer: retail runs
    /// the order-5 range fallback.
    RangeFallback,
}

/// Wrapped integer distance between two byte directions (0..=128).
fn a21_angle_distance(a: u8, b: u8) -> u8 {
    let raw = (i32::from(a) - i32::from(b)).rem_euclid(0x100);
    if raw <= 0x80 {
        raw as u8
    } else {
        (0x100 - raw) as u8
    }
}

impl World {
    /// `0x005a0058` (`FUN_005a0058`, ledger size 36).  The ledger row is the
    /// tail of the Computer AI handler: the handler reads the unit's
    /// click-order row from the per-unit-type table at `0x00f8f3b0`, inserts
    /// that row at the head of the order list with the current target
    /// (`CUnit+0x7c`) and list head (`CUnit+0x74`), clears `CUnit+0x7c`,
    /// then clears hidden bit 12, sets `CUnit+0x93` bit 0 when an order is
    /// present, and sets `CUnit+0xdc` bit 27 on the unit and its
    /// `CUnit+0x70` link.
    ///
    /// The per-type table row is not part of [`crate::data::GameData`], so the caller
    /// supplies it.  [`World::issue_order_id`] reproduces the row dispatch;
    /// the clear of `CUnit+0x7c`, the hidden-bit clear and the `0x93` bit-0
    /// marker are reproduced here.  `SimUnit` has no bit-27 field, so that
    /// last write has no Rust equivalent and is documented rather than
    /// stored.
    pub fn a21_computer_ai_click_order(
        &mut self,
        unit_id: u32,
        order_row: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        self.issue_order_id(unit_id, order_row, target, pos)?;
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        let unit = &mut self.units[i];
        unit.flags_93 |= 1;
        unit.target = None;
        // The reveal tail clears `CUnit+0xdc` bit 12, the hidden/loaded bit.
        unit.in_transport = None;
        Ok(())
    }
}

impl SimUnit {
    /// `0x005a0144` (`FUN_005a0144`, ledger size 110).  The ledger row lies
    /// inside the Teleport order handler (row 135); the decompiler started a
    /// second function in the middle of the phase dispatch.  Retail switches
    /// on `CUnit+0x4e`:
    ///
    /// * phase 0 validates the destination and stores phase 1;
    /// * phase 1 queries the approach state; a zero result returns, a result
    ///   of 2 re-checks the destination and a failure returns, every other
    ///   success stores `CUnit+0x5c = self`, plays cue `0x0d`, sets the
    ///   hidden bit and stores phase 2;
    /// * phase 2 tests `CUnit+0x4f` bit 0 and runs the end-of-order cleanup
    ///   when it is set.
    ///
    /// The phase byte, the destination check, the approach result and the end
    /// flag are not [`SimUnit`] fields, so they are explicit inputs and the
    /// recovered control flow is returned as [`A21TeleportStep`].
    pub fn a21_teleport_step(
        phase: u8,
        destination_clear: bool,
        approach_status: u8,
        end_flag: bool,
    ) -> A21TeleportStep {
        match phase {
            0 if destination_clear => A21TeleportStep::ToPhase1,
            0 => A21TeleportStep::Return,
            1 => match approach_status {
                0 => A21TeleportStep::Return,
                2 if !destination_clear => A21TeleportStep::Return,
                _ => A21TeleportStep::ToPhase2,
            },
            2 if end_flag => A21TeleportStep::FinishOrder,
            2 => A21TeleportStep::Return,
            _ => A21TeleportStep::Return,
        }
    }

    /// `0x005a0212` (`FUN_005a0212`, ledger size 3).  The whole decompiled
    /// body is a bare `return`: it is the default arm of the Sap Location
    /// handler's `CUnit+0x4e` dispatch (row 134), taken when the phase byte
    /// is not 0, 1 or 2.  The handler has no case for any other phase and
    /// returns without touching the target.
    pub fn a21_sap_location_phase_known(phase: u8) -> bool {
        phase <= 2
    }

    /// `0x005a046a` (`FUN_005a046a`, ledger size 97).  The ledger row is a
    /// branch inside the Nothing handler (row 22).  The handler tests the
    /// order target (`CUnit+0x5c`) with the critter predicate for rows
    /// `0x59`, `0x5a` and `0x5d..=0x60`.  A non-critter target takes the
    /// acknowledgement block: cue `0x17` and the per-type units.dat sound on
    /// the unit, then cue `0x62` and the per-type sound on the attached
    /// subunit (`CUnit+0x70`); a critter skips that block and falls through
    /// to target validation.
    ///
    /// The per-type sound bytes are not part of [`crate::data::GameData`], so only the two
    /// recovered cue ids are returned.  `None` means the critter path (no
    /// acknowledgement block).
    pub fn a21_nothing_ack_for_non_critter(target_type: u16) -> Option<(u8, u8)> {
        (!is_critter_type(target_type)).then_some((0x17, 0x62))
    }

    /// `0x005a04cb` (`FUN_005a04cb`, ledger size 51).  The ledger row is the
    /// tail of the Nothing handler (row 22).  Retail compares the order byte
    /// `CUnit+0x4d` of the attached subunit (`CUnit+0x70`) with the unit's
    /// per-type click-order row from the table at `0x00f8f3b0`, then runs the
    /// completed-Lurker query on the link, which reports 3 only for a
    /// completed Lurker (type `0x67` plus `CUnit+0xdc` bit 4).  On a match
    /// that is not 3, it syncs the sprite position, and if the link's
    /// `CUnit+0x20` bit 1 is clear it runs the order-5 range fallback;
    /// otherwise the handler returns quietly.  A failed match (or a
    /// completed Lurker) plays cue 4.
    ///
    /// The link's order byte, click row, Lurker/completed state and flyer bit
    /// are caller inputs because [`SimUnit`] does not own the subunit link.
    pub fn a21_nothing_click_tail(
        linked_order_byte: u8,
        click_row: u8,
        linked_is_completed_lurker: bool,
        linked_is_flyer: bool,
    ) -> A21NothingTail {
        if linked_order_byte != click_row || linked_is_completed_lurker {
            A21NothingTail::Cue4
        } else if linked_is_flyer {
            A21NothingTail::QuietReturn
        } else {
            A21NothingTail::RangeFallback
        }
    }

    /// `0x005a07b0` (`FUN_005a07b0`, ledger size 134).  Entry of the attack
    /// range/facing helper.  Weapon/order id `0x82` returns immediately.
    /// Otherwise the byte at `CUnit+0x54+slot` is considered: a nonzero value
    /// clamps `CUnit+0x85` to `value - 1` when that is smaller.  When the
    /// slot byte is zero and `CUnit+0x20` bit 3 is clear, the target or point
    /// distance is compared with `weapons.dat` minimum range
    /// (`0x00f93740[weapon]`); a zero minimum, or a distance at least the
    /// minimum, lets retail continue to the maximum-range lookup.
    ///
    /// Returns true exactly on that continuation path.  The slot byte, the
    /// minimum-range value and the `CUnit+0x20` bit are caller inputs because
    /// the first two live outside [`SimUnit`] and the bit is not modelled.
    pub fn a21_attack_range_precheck(
        &mut self,
        slot_value: u8,
        weapon: u8,
        min_range: u32,
        distance: i32,
        unit_flag_20_bit3: bool,
    ) -> bool {
        if weapon == 0x82 {
            return false;
        }
        if slot_value != 0 {
            let ceiling = slot_value - 1;
            if self.order_carry > ceiling {
                self.order_carry = ceiling;
            }
            return false;
        }
        if unit_flag_20_bit3 {
            return false;
        }
        min_range == 0 || distance >= min_range as i32
    }

    /// `0x005a0836` (`FUN_005a0836`, ledger size 49).  Continuation of the
    /// helper above after the maximum-range lookup has returned: reject when
    /// the distance exceeds that maximum, then reject when `CUnit+0x105`
    /// bit 1 is set.  The retail gate helper returns the complement of that
    /// bit, and the caller returns when the complement is zero.
    pub fn a21_attack_range_max_gate(&self, distance: i32, max_range: i32) -> bool {
        distance <= max_range && self.flags_105 & 2 == 0
    }

    /// `0x005a0871` (`FUN_005a0871`, ledger size 168).  Continuation of the
    /// same helper: the direction routine computes the byte direction from
    /// the sprite position (`CUnit+0x1c/0x1e`) to the unit position
    /// (`CUnit+0x58/0x5a`).  When the sprite destination
    /// (`CUnit+0x28/0x2a`) differs, a Lurker (type `0x67`) compares the
    /// wrapped difference between its facing (`CUnit+0x21`) and that
    /// direction with the byte table at `0x00f94430[weapon]`; out of arc
    /// clears `CUnit+0x85` and returns.  Every other unit stores the
    /// direction in `CUnit+0x21`.  At the destination retail falls into an
    /// unrecovered block.
    ///
    /// `direction` is the recovered direction byte, `at_destination` is the
    /// `CUnit+0x28/0x2a` comparison and `attack_angle` is the table byte.
    pub fn a21_attack_face_or_lurker_arc(
        &mut self,
        direction: u8,
        at_destination: bool,
        attack_angle: u8,
    ) -> bool {
        if at_destination {
            return true;
        }
        if self.type_id == uid::LURKER {
            if a21_angle_distance(self.facing, direction) > attack_angle {
                self.order_carry = 0;
                return false;
            }
            return true;
        }
        self.facing = direction;
        true
    }

    /// `0x005a0a08` (`FUN_005a0a08`, ledger size 22).  The leading
    /// conditional in the decompiled body is unrecoverable, but its coherent
    /// tail plays a cue and then runs the reveal helper.  The first step
    /// sets `CUnit+0x93` bit 0; the second clears hidden bit 12
    /// (`CUnit+0xdc`) and, when the order-list head `CUnit+0x74` is null,
    /// either plays cue `0x9c` (when `CUnit+0x134` is non-null) or the
    /// per-type units.dat sound from `0x00f8f2c8`.
    ///
    /// Returns the sound id the null-list branch would play, or `None` when
    /// a current order exists.  The per-type sound is caller-supplied because
    /// the units.dat sound column is not in [`crate::data::GameData`].
    pub fn a21_cue_and_reveal(&mut self, has_extra_link: bool, per_type_sound: u8) -> Option<u8> {
        self.flags_93 |= 1;
        self.in_transport = None;
        if !matches!(self.order, Order::Idle) {
            return None;
        }
        if has_extra_link {
            Some(0x9c)
        } else {
            Some(per_type_sound)
        }
    }

    /// `0x005a0de8` (`FUN_005a0de8`, ledger size 6).  The decompiled body is
    /// only `halt_baddata()`: the bytes at this ledger address are not a
    /// valid instruction stream.  The row is preserved as an explicit fatal
    /// hook so a dispatcher that reaches it fails loudly instead of silently
    /// continuing.
    pub fn a21_invalid_instruction_halt(&self) -> ! {
        panic!("0x005a0de8: invalid instruction stream")
    }
}

#[cfg(test)]
mod tests_agent21 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::Order;
    use std::sync::Arc;

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, owner: usize, type_id: u16) -> u32 {
        let i = world.spawn(owner, type_id, 320, 320).unwrap();
        world.units[i].id
    }

    #[test]
    fn computer_ai_click_order_installs_the_row_and_clears_the_target() {
        let mut w = open_world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        let zergling = spawn(&mut w, 1, uid::ZERGLING);

        let i = w.index_of(marine).unwrap();
        w.units[i].target = Some(zergling);
        w.units[i].in_transport = Some(zergling);
        w.units[i].flags_93 = 0;

        w.a21_computer_ai_click_order(marine, 9, Some(zergling), None)
            .unwrap();

        let i = w.index_of(marine).unwrap();
        assert_eq!(w.units[i].order, Order::AttackUnit { target: zergling });
        assert_eq!(w.units[i].order_row, 9);
        assert_eq!(w.units[i].target, None, "CUnit+0x7c is cleared");
        assert_eq!(w.units[i].in_transport, None, "hidden bit 12 is cleared");
        assert_eq!(w.units[i].flags_93 & 1, 1, "CUnit+0x93 bit 0");
    }

    #[test]
    fn teleport_step_models_the_phase_table() {
        use A21TeleportStep::*;

        assert_eq!(SimUnit::a21_teleport_step(0, false, 0, false), Return);
        assert_eq!(SimUnit::a21_teleport_step(0, true, 0, false), ToPhase1);
        assert_eq!(SimUnit::a21_teleport_step(1, true, 0, false), Return);
        assert_eq!(SimUnit::a21_teleport_step(1, false, 2, false), Return);
        assert_eq!(SimUnit::a21_teleport_step(1, true, 2, false), ToPhase2);
        assert_eq!(SimUnit::a21_teleport_step(1, true, 1, false), ToPhase2);
        assert_eq!(SimUnit::a21_teleport_step(2, true, 1, false), Return);
        assert_eq!(SimUnit::a21_teleport_step(2, true, 1, true), FinishOrder);
        assert_eq!(SimUnit::a21_teleport_step(3, true, 1, true), Return);
    }

    #[test]
    fn sap_location_phase_known_accepts_only_the_three_cases() {
        assert!(SimUnit::a21_sap_location_phase_known(0));
        assert!(SimUnit::a21_sap_location_phase_known(1));
        assert!(SimUnit::a21_sap_location_phase_known(2));
        assert!(!SimUnit::a21_sap_location_phase_known(3));
        assert!(!SimUnit::a21_sap_location_phase_known(0xff));
    }

    #[test]
    fn nothing_ack_is_only_for_non_critters() {
        assert_eq!(
            SimUnit::a21_nothing_ack_for_non_critter(uid::MARINE),
            Some((0x17, 0x62))
        );
        assert_eq!(SimUnit::a21_nothing_ack_for_non_critter(0x59), None);
        assert_eq!(SimUnit::a21_nothing_ack_for_non_critter(0x5d), None);
        assert_eq!(SimUnit::a21_nothing_ack_for_non_critter(0x60), None);
    }

    #[test]
    fn nothing_click_tail_handles_cue_quiet_and_fallback() {
        use A21NothingTail::*;

        assert_eq!(SimUnit::a21_nothing_click_tail(6, 7, false, false), Cue4);
        assert_eq!(SimUnit::a21_nothing_click_tail(7, 7, true, false), Cue4);
        assert_eq!(
            SimUnit::a21_nothing_click_tail(7, 7, false, true),
            QuietReturn
        );
        assert_eq!(
            SimUnit::a21_nothing_click_tail(7, 7, false, false),
            RangeFallback
        );
    }

    #[test]
    fn attack_range_precheck_covers_weapon_slot_flag_and_minimum_range() {
        let mut w = open_world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        let i = w.index_of(marine).unwrap();

        assert!(!w.units[i].a21_attack_range_precheck(0, 0x82, 0, 100, false));

        w.units[i].order_carry = 8;
        assert!(!w.units[i].a21_attack_range_precheck(4, 6, 0, 100, false));
        assert_eq!(w.units[i].order_carry, 3);

        w.units[i].order_carry = 2;
        assert!(!w.units[i].a21_attack_range_precheck(4, 6, 0, 100, false));
        assert_eq!(w.units[i].order_carry, 2);

        assert!(!w.units[i].a21_attack_range_precheck(0, 6, 0, 100, true));
        assert!(w.units[i].a21_attack_range_precheck(0, 6, 0, 0, false));
        assert!(w.units[i].a21_attack_range_precheck(0, 6, 100, 100, false));
        assert!(!w.units[i].a21_attack_range_precheck(0, 6, 100, 99, false));
    }

    #[test]
    fn attack_range_max_gate_checks_range_and_flags_105_bit1() {
        let mut w = open_world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        let i = w.index_of(marine).unwrap();

        w.units[i].flags_105 = 0;
        assert!(w.units[i].a21_attack_range_max_gate(100, 100));
        assert!(!w.units[i].a21_attack_range_max_gate(101, 100));

        w.units[i].flags_105 = 2;
        assert!(!w.units[i].a21_attack_range_max_gate(100, 100));
    }

    #[test]
    fn attack_face_or_lurker_arc_sets_facing_or_clears_the_carry() {
        let mut w = open_world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        let i = w.index_of(marine).unwrap();
        w.units[i].facing = 0;
        w.units[i].order_carry = 7;
        assert!(w.units[i].a21_attack_face_or_lurker_arc(64, false, 0));
        assert_eq!(w.units[i].facing, 64);
        assert_eq!(w.units[i].order_carry, 7);

        let lurker = spawn(&mut w, 0, uid::LURKER);
        let j = w.index_of(lurker).unwrap();
        w.units[j].facing = 250;
        w.units[j].order_carry = 5;
        assert!(w.units[j].a21_attack_face_or_lurker_arc(5, false, 20));
        assert_eq!(w.units[j].facing, 250);
        assert_eq!(w.units[j].order_carry, 5);

        assert!(!w.units[j].a21_attack_face_or_lurker_arc(100, false, 20));
        assert_eq!(w.units[j].order_carry, 0);

        w.units[j].order_carry = 9;
        assert!(w.units[j].a21_attack_face_or_lurker_arc(100, true, 0));
        assert_eq!(w.units[j].order_carry, 9);
    }

    #[test]
    fn cue_and_reveal_reports_the_recovered_sound_branch() {
        let mut w = open_world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        let i = w.index_of(marine).unwrap();

        w.units[i].order = Order::Move { pos: (400, 400) };
        w.units[i].in_transport = Some(marine);
        w.units[i].flags_93 = 0;
        assert_eq!(w.units[i].a21_cue_and_reveal(true, 0x10), None);
        assert_eq!(w.units[i].flags_93 & 1, 1);
        assert_eq!(w.units[i].in_transport, None);

        w.units[i].order = Order::Idle;
        w.units[i].flags_93 = 0;
        w.units[i].in_transport = Some(marine);
        assert_eq!(w.units[i].a21_cue_and_reveal(true, 0x10), Some(0x9c));
        assert_eq!(w.units[i].flags_93 & 1, 1);
        assert_eq!(w.units[i].in_transport, None);

        w.units[i].flags_93 = 0;
        assert_eq!(w.units[i].a21_cue_and_reveal(false, 0x10), Some(0x10));
    }

    #[test]
    #[should_panic(expected = "0x005a0de8")]
    fn invalid_instruction_row_is_an_explicit_fatal_hook() {
        let mut w = open_world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        let i = w.index_of(marine).unwrap();
        w.units[i].a21_invalid_instruction_halt();
    }
}
