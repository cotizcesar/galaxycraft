#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent04.rs
// ===========================================================================
// Translation batch 04.  Only this file is edited by the batch.
//
// The routines below were recovered from the `0x005b3000..=0x005b4fff`
// CUnit/CSprite helper cluster.  Every method name is prefixed with `a04_`
// so the batch cannot collide with the main `SimUnit`/`World` impls or with
// the other concurrent batches.

/// `FUN_005b4d2d`/`0x005b4db0`/`0x005b4e00`: width of the slot field in a
/// packed unit reference when the pool holds at most `0x6a4` (1700, the
/// retail unit-pool size) entries.
const A04_SLOT_BITS_NARROW: u32 = 11;

/// `FUN_005b4d2d`/`0x005b4db0`/`0x005b4e00`: width of the slot field once the
/// pool grows past `0x6a4`; the remaining high bits keep the generation byte
/// (`CUnit+0xa5`).
const A04_SLOT_BITS_WIDE: u32 = 13;

/// `FUN_005b4d2d`/`0x005b4db0`/`0x005b4e00`: the pool size that selects the
/// wide slot field (`0x6a4 < FUN_0060b560()`).
const A04_WIDE_POOL_THRESHOLD: usize = 0x6a4;

impl SimUnit {
    /// `0x005b49a0`: status-modifier count shared by the speed and cooldown
    /// helpers.
    ///
    /// The recovered body is a pure integer combination of three status
    /// sources:
    ///
    /// ```text
    /// base = (CUnit+0x115 != 0)
    /// n    = (CUnit+0xdc & 0x20000000) != 0 ? base + 1 : base
    /// mods = (CUnit+0x116 != 0)            ? n - 1    : n
    /// ```
    ///
    /// `CUnit+0x115` is the Stim Pack timer, `CUnit+0x116` is the Ensnare
    /// timer and `CUnit+0xdc` bit 29 is [`SimUnit::flag_29`].  The
    /// speed/cooldown helpers `FUN_005b4f20`, `FUN_005b5640`, `FUN_005b68b0` and
    /// `FUN_005b6900` spell out the same shape but test bit 28
    /// (`0x10000000`) instead of bit 29, so the two groups are kept separate.
    pub fn a04_status_modifier_count(&self) -> i32 {
        let mut mods = i32::from(self.status.stim_ticks > 0);
        if self.flag_29 {
            mods += 1;
        }
        if self.status.ensnare_ticks > 0 {
            mods -= 1;
        }
        mods
    }

    /// `0x005b3bb0`: wrapper around `FUN_005d48f0(1)`.
    ///
    /// `FUN_005d48f0` only acts when `CUnit+0xdc` bit 6 (Bunker) is set.  It
    /// early-returns when the passed flag is zero or when the unit still has
    /// a `CUnit+0x80` link, then clears the Bunker bit (`FUN_005b6840(0)`) and
    /// the transport bit 5 (`FUN_005b48e0`).  The second `FUN_005b48e0` call
    /// covers the attached subunit at `CUnit+0x70`, which the sim does not
    /// link from the parent; the caller's own two flags are the observable
    /// effect.
    pub fn a04_exit_bunker(&mut self) {
        if !self.in_bunker {
            return;
        }
        // `FUN_005d48f0`: `if (param_1 == 0 || this+0x80 != 0) return;`.
        if self.connected_unit.is_some() {
            return;
        }
        self.in_bunker = false;
        self.in_transport = None;
    }

    /// `0x005b3b57`: the recovered body is a bare `return` (size 1).  Kept as
    /// an explicit hook so callers that dispatch the recovered routine have a
    /// named, testable no-op.
    pub fn a04_empty_body(&self) {}
}

impl World {
    /// `0x005b3620`: the unit's linked target is a Flag Beacon.
    ///
    /// The recovered body first asks `FUN_005b64c0` for the worker flag and, on
    /// success, returns `FUN_005b55d0`: type id below `0xe4`, `units.dat` flag
    /// `0x8` set, a nonzero `CUnit+0xc0` link and a linked target whose type
    /// is `0xd7`.  The fallback branch calls `FUN_005d5c80`, whose only
    /// non-aborting path returns zero, so the predicate above is the whole
    /// result.
    pub fn a04_worker_flag_target(&self, i: usize) -> bool {
        let Some(u) = self.units.get(i) else {
            return false;
        };
        let ty = u.type_id;
        if ty >= 0xe4 || !self.data.unit(ty).is_worker() {
            return false;
        }
        match u.linked_target() {
            Some(id) => self.unit(id).is_some_and(|t| t.type_id == 0xd7),
            None => false,
        }
    }

    /// Slot-field width used by the packed unit-reference helpers
    /// (`FUN_005b4d2d`, `0x005b4db0`, `0x005b4e00`): 11 bits for the classic
    /// 1700-entry pool, 13 bits once the pool grows past `0x6a4`.  The live
    /// `units` vector stands in for `FUN_0060b560`'s pool size.
    pub fn a04_reference_slot_bits(&self) -> u32 {
        if self.units.len() > A04_WIDE_POOL_THRESHOLD {
            A04_SLOT_BITS_WIDE
        } else {
            A04_SLOT_BITS_NARROW
        }
    }

    /// `0x005b4db0`: resolve the 1-based slot stored in the low bits of a
    /// packed unit reference.
    ///
    /// Retail returns `base + (slot - 1) * 0x150`, i.e. the `CUnit*` for a
    /// one-based pool slot, after masking the slot with `(1 << s) - 1`.
    /// `SimUnit` ids are opaque and the pool is a dense vector, so the result
    /// is the id occupying that one-based slot.  A zero reference yields
    /// `None` (retail would index one element before the pool) and an
    /// out-of-range slot yields `None` instead of a stale pointer.
    pub fn a04_unit_id_at_reference(&self, reference: u32) -> Option<u32> {
        if reference == 0 {
            return None;
        }
        let mask = (1u32 << self.a04_reference_slot_bits()) - 1;
        let slot = reference & mask;
        if slot == 0 {
            return None;
        }
        self.units.get(slot as usize - 1).map(|u| u.id)
    }

    /// `0x005b4e00`: pack a one-based slot number and the generation byte
    /// (`CUnit+0xa5`) into the reference used by `0x005b4db0`/`FUN_005b4d2d`.
    ///
    /// Retail derives the slot from the pointer distance to the unit array
    /// (`(ptr - base) / 0x150 + 1`), rejects slots beyond `FUN_0060b560()`,
    /// then returns `(byte@0xa5 << s) | slot`.  `s` is 11 or 13 as selected
    /// by the pool size.  The sim has no per-unit generation byte, so the
    /// caller supplies it; the arithmetic is otherwise unchanged.
    pub fn a04_unit_reference(&self, i: usize, generation: u8) -> u32 {
        if i >= self.units.len() {
            return 0;
        }
        let slot = i as u32 + 1;
        let bits = self.a04_reference_slot_bits();
        (u32::from(generation) << bits) | slot
    }
}

#[cfg(test)]
mod tests_agent04 {
    use super::*;
    use crate::ids::unit as uid;
    use crate::sim::Order;

    fn world() -> World {
        World::new(
            std::sync::Arc::new(crate::data::GameData::bundled().unwrap()),
            crate::map::Terrain::open(64, 64),
            2,
        )
    }

    /// `0x005b49a0`: the modifier count is +1 for Stim, +1 for `CUnit+0xdc`
    /// bit 29 and -1 for Ensnare; the three sources combine additively.
    #[test]
    fn status_modifier_count_combines_stim_ensnare_and_flag29() {
        let mut w = world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();

        assert_eq!(w.units[u].a04_status_modifier_count(), 0);

        w.units[u].status.stim_ticks = 336;
        assert_eq!(w.units[u].a04_status_modifier_count(), 1);

        w.units[u].status.ensnare_ticks = 75;
        assert_eq!(w.units[u].a04_status_modifier_count(), 0, "both cancel");

        w.units[u].status.stim_ticks = 0;
        assert_eq!(w.units[u].a04_status_modifier_count(), -1);

        // Bit 29 is the second positive source (the units.dat spellcaster
        // flag mirrored into `CUnit+0xdc`).
        w.units[u].flag_29 = true;
        assert_eq!(w.units[u].a04_status_modifier_count(), 0);
        w.units[u].status.ensnare_ticks = 0;
        assert_eq!(w.units[u].a04_status_modifier_count(), 1);
        w.units[u].status.stim_ticks = 336;
        assert_eq!(w.units[u].a04_status_modifier_count(), 2);
        w.units[u].status.ensnare_ticks = 75;
        assert_eq!(w.units[u].a04_status_modifier_count(), 1);
    }

    /// `0x005b3620`: only a worker with a linked Flag Beacon (`0xd7`) passes;
    /// a non-worker with the same link, or a worker without one, fails.
    #[test]
    fn worker_flag_target_requires_a_worker_and_a_flag_beacon() {
        let mut w = world();
        let scv = w.spawn(0, uid::SCV, 320, 320).unwrap();
        let flag = w.spawn(0, 0xd7, 360, 320).unwrap();
        let flag_id = w.units[flag].id;

        assert!(!w.a04_worker_flag_target(scv), "no linked target yet");

        w.units[scv].order = Order::Gather { resource: flag_id };
        assert!(w.a04_worker_flag_target(scv));

        // An ordinary unit carrying the same unit-target order is rejected by
        // the `FUN_005b64c0` worker-flag test.
        let marine = w.spawn(0, uid::MARINE, 320, 360).unwrap();
        w.units[marine].order = Order::AttackUnit { target: flag_id };
        assert!(!w.a04_worker_flag_target(marine));

        // A worker pointing at a non-flag unit is rejected too.
        let depot = w.spawn(0, uid::COMMAND_CENTER, 400, 320).unwrap();
        w.units[scv].order = Order::Repair {
            target: w.units[depot].id,
        };
        assert!(!w.a04_worker_flag_target(scv));

        // Out-of-range indices never panic.
        assert!(!w.a04_worker_flag_target(w.units.len()));
    }

    /// `0x005b3bb0` -> `FUN_005d48f0(1)`: a Bunkered unit drops both links,
    /// but the helper early-returns while a `CUnit+0x80` link is present.
    #[test]
    fn exit_bunker_clears_bunker_and_transport_bits() {
        let mut w = world();
        let bunker = w.spawn(0, uid::BUNKER, 320, 320).unwrap();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let bunker_id = w.units[bunker].id;

        w.units[u].in_bunker = true;
        w.units[u].in_transport = Some(bunker_id);
        w.units[u].a04_exit_bunker();
        assert!(!w.units[u].in_bunker);
        assert_eq!(w.units[u].in_transport, None);

        // `FUN_005d48f0` returns early while `CUnit+0x80` is set.
        w.units[u].in_bunker = true;
        w.units[u].in_transport = Some(bunker_id);
        w.units[u].connected_unit = Some(bunker_id);
        w.units[u].a04_exit_bunker();
        assert!(w.units[u].in_bunker);
        assert_eq!(w.units[u].in_transport, Some(bunker_id));

        // A unit that is not in a Bunker is untouched (bit 6 gate).
        w.units[u].in_bunker = false;
        w.units[u].connected_unit = None;
        w.units[u].a04_exit_bunker();
        assert_eq!(w.units[u].in_transport, Some(bunker_id));
    }

    /// `0x005b4e00` / `0x005b4db0`: one-based slots pack into the low field
    /// and resolve back to the same unit; the high generation bits are
    /// ignored by the resolver, as in the retail body.
    #[test]
    fn packed_unit_references_round_trip_through_one_based_slots() {
        let mut w = world();
        let a = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let b = w.spawn(0, uid::MARINE, 340, 320).unwrap();
        let (id_a, id_b) = (w.units[a].id, w.units[b].id);

        assert_eq!(w.a04_reference_slot_bits(), 11);
        assert_eq!(w.a04_unit_reference(a, 0), 1);
        assert_eq!(w.a04_unit_reference(b, 0), 2);
        assert_eq!(w.a04_unit_reference(a, 0x2a), (0x2a << 11) | 1);

        assert_eq!(w.a04_unit_id_at_reference(1), Some(id_a));
        assert_eq!(w.a04_unit_id_at_reference(2), Some(id_b));
        assert_eq!(w.a04_unit_id_at_reference((7 << 11) | 2), Some(id_b));
        assert_eq!(w.a04_unit_id_at_reference(0), None);
        assert_eq!(w.a04_unit_id_at_reference(3), None);
        assert_eq!(w.a04_unit_reference(w.units.len(), 0), 0);

        // Past 0x6a4 entries the slot field widens to 13 bits and the
        // generation byte starts at bit 13.
        let template = w.units[a].clone();
        while w.units.len() <= A04_WIDE_POOL_THRESHOLD {
            w.units.push(template.clone());
        }
        assert_eq!(w.a04_reference_slot_bits(), 13);
        assert_eq!(w.a04_unit_reference(0, 1), (1 << 13) | 1);
        assert_eq!(w.a04_unit_id_at_reference((3 << 13) | 1), Some(id_a));
    }

    /// `0x005b3b57`: the recovered routine is empty, so calling it leaves the
    /// unit untouched.
    #[test]
    fn empty_body_changes_nothing() {
        let mut w = world();
        let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        w.units[u].hp = 1234;
        w.units[u].order = Order::HoldPosition;
        let before = format!("{:?}", w.units[u]);
        w.units[u].a04_empty_body();
        assert_eq!(before, format!("{:?}", w.units[u]));
    }
}
