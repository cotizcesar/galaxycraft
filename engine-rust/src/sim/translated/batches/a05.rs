#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent05.rs
// ===========================================================================
// Translation batch 05.  Only this file is edited by the batch.
//
// Nine `decompiled_uncited` routines from the `0x005b5000..=0x005b6fff`
// slice of the retail `CUnit` method table.  Every item names the exact
// retail address it was recovered from.
//
// The routines in this slice are mostly tiny `CUnit` accessors.  The port
// does not keep a `CSprite`, a raw `CUnit+0xdc` status word, a raw
// `CUnit+0x70/0xc0/0xd0/0xfc` link, or the global sound mixer, so the
// accessors that only poke those unmodelled slots are listed as skipped in
// the batch report rather than approximated here.

impl SimUnit {
    /// `0x005b5640`: true when the recovered status count is positive.
    ///
    /// Retail builds the count from three inputs: `CUnit+0x115` (the Stim
    /// Pack timer, +1), `CUnit+0xdc` bit 28 (+1) and `CUnit+0x116` (the
    /// Ensnare timer, -1), then returns `count > 0`.  This engine keeps the
    /// two timers but not bit 28 -- the same simplification already used by
    /// `World::modified_speed` for the twin routine `FUN_005b6900` -- so a
    /// positive count is "stimmed and not ensnared".
    pub fn a05_status_boost_active(&self) -> bool {
        self.a05_status_modifier() > 0
    }

    /// `0x005b68b0`: apply the recovered attack-damage scaling.
    ///
    /// The routine is `FUN_005b6900` with damage instead of speed: a positive
    /// status count doubles the value, a negative count returns
    /// `value - value/4`, and a zero count leaves it unchanged.  The count
    /// is built exactly as in [`SimUnit::a05_status_boost_active`] (Stim
    /// `CUnit+0x115` +1, Ensnare `CUnit+0x116` -1, `CUnit+0xdc` bit 28 is
    /// not retained).
    pub fn a05_status_damage_scale(&self, value: u32) -> u32 {
        let count = self.a05_status_modifier();
        if count > 0 {
            value.wrapping_mul(2)
        } else if count < 0 {
            value - (value >> 2)
        } else {
            value
        }
    }

    /// Shared status count for `0x005b5640` and `0x005b68b0` (and the speed
    /// routine `FUN_005b6900`): `+1` for Stim Pack, `-1` for Ensnare.
    fn a05_status_modifier(&self) -> i32 {
        let mut count = 0;
        if self.status.stim_ticks > 0 {
            count += 1;
        }
        if self.status.ensnare_ticks > 0 {
            count -= 1;
        }
        count
    }

    /// `0x005b5920`: `CUnit+0xdc` bit 11 set while status bit 4 (the
    /// completed flag, see `FUN_005b5840`) is clear.
    ///
    /// Bit 4 is [`SimUnit::completed`].  Bit 11 is not retained by this
    /// engine, so it is supplied by the caller.
    pub fn a05_bit11_set_and_unfinished(&self, bit_11: bool) -> bool {
        bit_11 && !self.completed
    }

    /// `0x005b61ca`: empty virtual slot.  The recovered body is a bare
    /// `return`; the method exists so vtable dispatch still resolves.
    pub fn a05_noop(&self) {}

    /// `0x005b62f0`: true when the queued row in a Zerg queue slot is below
    /// `0x6a`, i.e. it is a mobile unit rather than a building (row `0x6a`
    /// is the Command Center, the first building row).
    ///
    /// The slot lookup is `FUN_0058deb0`, which reads the ring of five u16
    /// type ids at `CUnit+0x98`, indexed by `(CUnit+0xa4 + slot) % 5`.  This
    /// engine's Zerg queue is [`SimUnit::production`], whose front is the
    /// head slot, so `slot` indexes it from the front.
    pub fn a05_queue_slot_below_buildings(&self, slot: u8) -> bool {
        self.a05_queue_slot_type(slot) < 0x6a
    }

    /// `FUN_0058deb0`: type id stored in the Zerg five-slot queue
    /// (`CUnit+0x98`), or zero for an empty/non-training slot.
    fn a05_queue_slot_type(&self, slot: u8) -> u16 {
        match self.production.get(slot as usize) {
            Some(Order::Train { unit_type }) => *unit_type,
            _ => 0,
        }
    }

    /// `0x005b63bd`: raw `CUnit*` identity test (`this == other`).  Retail
    /// compares the two `this` pointers, so two distinct units are never
    /// equal even when their fields coincide.
    pub fn a05_is_same_unit(&self, other: &SimUnit) -> bool {
        std::ptr::eq(self, other)
    }

    /// `0x005b66f0`: set `CUnit+0xdc` bit 26 (`0x04000000`), the capability
    /// bit retained only for types with `units.dat` flag `0x20000000`
    /// (cleared again by `FUN_005b66c0`).
    pub fn a05_set_flag_26(&mut self) {
        self.flag_26 = true;
    }

    /// `0x005b6890`: play fixed line `0xf` for the unit's owner
    /// (`FUN_00773180(CUnit+0x4c, 0xf)`).
    ///
    /// `FUN_00773180` resolves a per-player sound-table entry; this engine
    /// has no global mixer, so the request is returned as
    /// `(owner, sound id)` for the caller to forward.
    pub fn a05_alert_sound(&self) -> (u8, u8) {
        (self.owner, 0x0f)
    }

    /// `0x005b6af0`: play the caller's sound id for the unit's owner
    /// (`FUN_00773180(CUnit+0x4c, sound)`).  Like [`SimUnit::a05_alert_sound`]
    /// the request is returned rather than played.
    pub fn a05_owner_sound(&self, sound: u8) -> (u8, u8) {
        (self.owner, sound)
    }
}

#[cfg(test)]
mod tests_agent05 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    #[test]
    fn status_boost_and_damage_scale_follow_the_recovered_counts() {
        // 0x005b5640 / 0x005b68b0: +1 for Stim (CUnit+0x115), -1 for
        // Ensnare (CUnit+0x116), damage doubled when positive and reduced
        // by a quarter when negative.
        let mut w = world();
        let m = w.spawn(0, uid::MARINE, 100, 100).unwrap();
        assert!(!w.units[m].a05_status_boost_active());
        assert_eq!(w.units[m].a05_status_damage_scale(100), 100);

        w.units[m].status.stim_ticks = 5;
        assert!(w.units[m].a05_status_boost_active());
        assert_eq!(w.units[m].a05_status_damage_scale(100), 200);

        w.units[m].status.ensnare_ticks = 5;
        assert!(!w.units[m].a05_status_boost_active(), "the counts cancel");
        assert_eq!(w.units[m].a05_status_damage_scale(100), 100);

        w.units[m].status.stim_ticks = 0;
        assert!(!w.units[m].a05_status_boost_active());
        assert_eq!(w.units[m].a05_status_damage_scale(100), 75);
        assert_eq!(w.units[m].a05_status_damage_scale(101), 76);
    }

    #[test]
    fn bit11_and_unfinished_needs_both_inputs() {
        // 0x005b5920: bit 11 set at CUnit+0xdc while bit 4 (completed) is
        // clear.  Bit 4 maps to SimUnit::completed.
        let mut w = world();
        let m = w.spawn(0, uid::MARINE, 100, 100).unwrap();
        assert!(!w.units[m].a05_bit11_set_and_unfinished(false));
        assert!(!w.units[m].a05_bit11_set_and_unfinished(true));
        w.units[m].completed = false;
        assert!(w.units[m].a05_bit11_set_and_unfinished(true));
        assert!(!w.units[m].a05_bit11_set_and_unfinished(false));
    }

    #[test]
    fn empty_virtual_slot_does_nothing() {
        // 0x005b61ca: the recovered body is an empty return.
        let mut w = world();
        let m = w.spawn(0, uid::MARINE, 100, 100).unwrap();
        let before = (
            w.units[m].x,
            w.units[m].y,
            w.units[m].hp,
            w.units[m].order_row,
        );
        w.units[m].a05_noop();
        assert_eq!(
            (
                w.units[m].x,
                w.units[m].y,
                w.units[m].hp,
                w.units[m].order_row
            ),
            before
        );
    }

    #[test]
    fn queue_slot_predicate_reads_production_rows() {
        // 0x005b62f0 with the FUN_0058deb0 slot lookup: rows below the
        // building range 0x6a are mobile units.
        let mut w = world();
        let h = w.spawn(0, uid::HATCHERY, 200, 200).unwrap();
        assert!(w.units[h].a05_queue_slot_below_buildings(0), "empty slot");
        w.units[h].production.push_back(Order::Train {
            unit_type: uid::ZERGLING,
        });
        w.units[h].production.push_back(Order::Train {
            unit_type: uid::COMMAND_CENTER,
        });
        assert_eq!(w.units[h].a05_queue_slot_type(0), uid::ZERGLING);
        assert!(w.units[h].a05_queue_slot_below_buildings(0));
        assert!(!w.units[h].a05_queue_slot_below_buildings(1));
        assert_eq!(w.units[h].a05_queue_slot_type(1), uid::COMMAND_CENTER);
        assert!(
            w.units[h].a05_queue_slot_below_buildings(2),
            "past the queue"
        );
    }

    #[test]
    fn identity_test_matches_the_this_pointer() {
        // 0x005b63bd: `this == other` on the CUnit pointer.
        let mut w = world();
        let a = w.spawn(0, uid::MARINE, 100, 100).unwrap();
        let b = w.spawn(0, uid::MARINE, 120, 100).unwrap();
        let same = w.units[a].clone();
        assert!(w.units[a].a05_is_same_unit(&w.units[a]));
        assert!(!w.units[a].a05_is_same_unit(&w.units[b]));
        assert!(!w.units[a].a05_is_same_unit(&same), "a clone is not this");
    }

    #[test]
    fn flag_26_setter_sets_the_status_bit() {
        // 0x005b66f0: OR 0x04000000 into CUnit+0xdc.
        let mut w = world();
        let m = w.spawn(0, uid::MARINE, 100, 100).unwrap();
        assert!(!w.units[m].flag_26);
        w.units[m].a05_set_flag_26();
        assert!(w.units[m].flag_26);
        w.units[m].a05_set_flag_26();
        assert!(w.units[m].flag_26, "idempotent");
    }

    #[test]
    fn sound_requests_carry_the_owner_and_the_recovered_id() {
        // 0x005b6890: FUN_00773180(CUnit+0x4c, 0xf).
        // 0x005b6af0: FUN_00773180(CUnit+0x4c, sound).
        let mut w = world();
        let m = w.spawn(0, uid::MARINE, 100, 100).unwrap();
        let e = w.spawn(1, uid::MARINE, 300, 100).unwrap();
        assert_eq!(w.units[m].a05_alert_sound(), (0, 0x0f));
        assert_eq!(w.units[e].a05_alert_sound(), (1, 0x0f));
        assert_eq!(w.units[m].a05_owner_sound(0x2a), (0, 0x2a));
        assert_eq!(w.units[e].a05_owner_sound(0x2a), (1, 0x2a));
    }
}
