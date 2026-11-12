#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent35.rs
// ===========================================================================
// Translation batch 35.  Only this file is edited by the batch.
//
// The `decompiled_uncited` entries of `0x005d8000..=0x005d9fff` are a mixed
// cluster: CRT stack/page helpers, the per-unit order-dispatch tail, the
// teleport/placement guard used by the scenario and order code, one order
// reset tail, a diagnostic hook and a small queue-latch helper.  Most bodies
// were recovered with `Control flow encountered bad instruction data`, so
// only the readable blocks are translated.  The retail record offsets that
// [`SimUnit`] does not store (`CUnit+0x20` status byte, `+0x4c` secondary
// order, `+0x70` attached unit, `+0x74` weapon/sprite slot, `+0xe3` retry
// counter, `+0x134` link) are either passed in by the caller or reported in
// the return value; no state is added to [`World`] or [`SimUnit`].

/// Outcome of the direct-position guard (`0x005d8f70`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A35RepositionGuard {
    /// The unit already stands on the requested pixel.
    AlreadyThere,
    /// A link exists and the unit is in the one order that permits the move,
    /// or it is airborne/a subunit; retail returns 0.
    Refused,
    /// No link, or a linked ground unit in any other order.  Retail calls the
    /// non-returning record-error helper on this path, so the translation
    /// reports it instead of aborting.
    Invalid,
}

// ---------------------------------------------------------------------
// Free helpers (no World/SimUnit state involved).
// ---------------------------------------------------------------------

/// `0x005d8000`: page count of a stack probe.
///
/// The recovered body stores `0x296`, adds `0xd6a` and reads the sum back
/// (`0x296 + 0xd6a = 0x1000`), then takes the timestamp and tail-calls the
/// non-returning stack helper with that value and zero.  The constant pair is
/// the CRT page stride: one page is touched per `0x1000` bytes of the
/// allocation.  The translated routine reports how many 4 KiB pages a request
/// of `len` bytes covers.
pub fn a35_stack_probe_pages(len: usize) -> usize {
    let pages = len / 0x1000;
    if len.is_multiple_of(0x1000) {
        pages
    } else {
        pages + 1
    }
}

/// `0x005d8027`: drain the intrusive pending list.
///
/// The readable epilogue of this control-flow-corrupt body walks the global
/// head while it is non-null, saves the `+4` next link, runs the per-node
/// routine and clears both the head and tail globals before returning 0.  The
/// list nodes are engine-owned memory that [`World`] does not model, so the
/// same walk is expressed over a caller-supplied queue of ids.  Returns the
/// ids in the order they were processed.
pub fn a35_drain_pending_ids(pending: &mut VecDeque<u32>) -> Vec<u32> {
    let mut processed = Vec::with_capacity(pending.len());
    while let Some(id) = pending.pop_front() {
        processed.push(id);
    }
    processed
}

/// `0x005d8a19`: set every bit except `0x1a`.
///
/// The nine-byte body is a single `or dword [stack_slot], 0xffffffe5`; the
/// stack slot is far below the frame pointer and has no symbol.  `0xffffffe5`
/// is the complement of `0x1a` (bits 1, 3 and 4), which is the recovered
/// effect: the source bits are preserved and every other bit is raised.
pub fn a35_or_reserved_bits(value: u32) -> u32 {
    value | 0xffff_ffe5
}

/// `0x005d8a39`: empty body.
///
/// The one-byte routine is a bare `ret`: it is a complete, separately
/// dispatched hook with no arguments, no reads and no writes.  Translating it
/// as a no-op keeps the ledger row accounted for without inventing behaviour.
pub fn a35_empty_body() {}

/// `0x005d9cd3`: empty `stdcall` stub.
///
/// The three-byte body is `ret 0` (`c2 00 00`), the stdcall spelling of an
/// empty routine: no arguments are popped and no register or memory is
/// written.  It is a separately dispatched entry, but it carries no
/// behaviour, so the translation is a no-op.
pub fn a35_empty_stdcall_stub() {}

/// `0x005d9cf0`: unconditional rejection.
///
/// The eight-byte body is a tail call to the non-returning record-error
/// helper with no arguments: every caller of this entry is rejected.  The
/// translation returns the corresponding [`SimError`] instead of aborting.
pub fn a35_reject_unconditionally() -> Result<(), SimError> {
    Err(SimError::InvalidTarget)
}

// ---------------------------------------------------------------------
// SimUnit state transitions.
// ---------------------------------------------------------------------

impl SimUnit {
    /// `0x005d834c`: advance the per-unit order carry counter.
    ///
    /// The recovered tail (reached after the per-order dispatch switch)
    /// reads `CUnit+0x85`, decrements it and, when the old value was zero,
    /// resets it to 8, clears `CUnit+0x92` and runs the second-stage handler
    /// switch keyed by `CUnit+0x4d`.  [`SimUnit::order_carry`] is the modelled
    /// `+0x85` byte; `+0x92` and the switch itself belong to the typed order
    /// state machine in `sim.rs`, so only the counter step is reproduced here.
    /// Returns whether the second-stage arm runs.
    pub fn a35_advance_order_carry(&mut self) -> bool {
        let previous = self.order_carry;
        self.order_carry = previous.wrapping_sub(1);
        if previous == 0 {
            self.order_carry = 8;
            true
        } else {
            false
        }
    }

    /// `0x005d90a9`: reset the order retry counter and visual latch.
    ///
    /// After the control-flow-corrupt prologue the body writes `0xf` to
    /// `CUnit+0xe3` (the order/AI retry counter), clears `CUnit+0xdc` bit 19
    /// and then inspects the byte at `CUnit+0x74` (the weapon/sprite slot):
    /// when the slot exists and the per-slot table entry is zero, bit `0x20`
    /// of the `CUnit+0x20` status byte is raised, otherwise it is cleared.
    /// Both arms return 1.  [`SimUnit::set_flag_19`] is the modelled write;
    /// `+0xe3` and `+0x20` have no Rust field.  The returned `bool` is the
    /// value the `+0x20` bit would take.
    pub fn a35_reset_retry_visual(&mut self, weapon_table_zero: bool) -> bool {
        self.set_flag_19(false);
        weapon_table_zero
    }

    /// `0x005d94e0`: report the secondary order via the diagnostic helper.
    ///
    /// The body tests the engine-wide diagnostic latch and, when it is set,
    /// calls the non-returning reporting helper with the byte at `CUnit+0x4c`
    /// (the secondary order id; the main order sits at `+0x4d`).  That byte
    /// is not stored by [`SimUnit`], so the caller supplies it.  `Some(code)`
    /// is the reported id; `None` is the quiet path.
    pub fn a35_diagnostic_secondary_order(
        &self,
        diagnostic_latch: bool,
        secondary_order: u8,
    ) -> Option<u8> {
        if diagnostic_latch {
            Some(secondary_order)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------
// World-level placement and queue helpers.
// ---------------------------------------------------------------------

impl World {
    /// `0x005d8f70`: guard a direct position change.
    ///
    /// The body compares the unit's `CUnit+0x10`/`+0x12` pixel pair with the
    /// requested one and returns 1 on a match.  Otherwise, when the record
    /// carries a `CUnit+0x134` link, it consults a helper that returns 1 for
    /// order `0x60` and 0 for airborne or subunit records, aborting for every
    /// other combination; a 0 result makes the guard return 0.  With no link
    /// the body aborts immediately.  The translated guard reports the three
    /// outcomes instead of aborting.  `has_order_link` stands in for
    /// `CUnit+0x134`; the air bit and the subunit flag come from the bundled
    /// unit table.
    pub fn a35_reposition_guard(
        &self,
        i: usize,
        pos: (i32, i32),
        has_order_link: bool,
    ) -> A35RepositionGuard {
        let Some(unit) = self.units.get(i) else {
            return A35RepositionGuard::Invalid;
        };
        if unit.x / fp(1) == pos.0 && unit.y / fp(1) == pos.1 {
            return A35RepositionGuard::AlreadyThere;
        }
        let order_60 = unit.order_row == 0x60 || unit.order.order_id() == 0x60;
        let may_move = order_60 || self.is_air_unit(i) || self.data.unit(unit.type_id).is_subunit();
        if has_order_link && may_move {
            A35RepositionGuard::Refused
        } else {
            A35RepositionGuard::Invalid
        }
    }

    /// `0x005d8fc5`: clamp, place and reset the order retry state.
    ///
    /// The recovered body first clamps the requested point with the
    /// per-unit-type box table: `x` below the left extent becomes the extent,
    /// `x` at or past `map_width - right` becomes `map_width - right - 1`, and
    /// `y` follows the same rule with an extra `0x20`-pixel bottom margin
    /// (`map_height - bottom - 0x21`).  It then stores the point through the
    /// position setter, writes `0xf` to `CUnit+0xe3`, clears `CUnit+0xdc` bit
    /// 19 and raises or clears bit `0x20` of the `CUnit+0x20` status byte from
    /// the weapon-slot table.  Both arms return 1.
    ///
    /// The extents come from [`World::unit_box_left`] and companions, the map
    /// size from the walk grid (eight pixels per cell), and
    /// [`SimUnit::set_flag_19`] models the flag-19 clear.  `+0xe3` and `+0x20`
    /// have no Rust field.  The returned `bool` is the value the `+0x20` bit
    /// would take (`weapon_table_zero`).  Out-of-range indexes are ignored.
    pub fn a35_place_unit_clamped(
        &mut self,
        i: usize,
        pos: (i32, i32),
        weapon_table_zero: bool,
    ) -> bool {
        let Some(unit) = self.units.get(i) else {
            return weapon_table_zero;
        };
        let type_id = unit.type_id;
        let left = self.unit_box_left(type_id);
        let top = self.unit_box_top(type_id);
        let right = self.unit_box_right(type_id);
        let bottom = self.unit_box_bottom(type_id);
        let map_w = self.terrain.walk_width as i32 * 8;
        let map_h = self.terrain.walk_height as i32 * 8;

        let mut x = pos.0;
        let mut y = pos.1;
        if x < left {
            x = left;
        } else if map_w - right <= x {
            x = map_w - right - 1;
        }
        if y < top {
            y = top;
        } else if map_h - bottom - 0x20 <= y {
            y = map_h - bottom - 0x21;
        }

        let unit = &mut self.units[i];
        if unit.x / fp(1) != x || unit.y / fp(1) != y {
            unit.x = x.saturating_mul(fp(1));
            unit.y = y.saturating_mul(fp(1));
        }
        unit.set_flag_19(false);
        weapon_table_zero
    }

    /// `0x005d9c90`: latch the queue/flag word on a grounded unit.
    ///
    /// The body reads `CUnit+0xdc` bit 2 (the dynamic air bit) and returns 0
    /// for airborne records.  Otherwise it raises `CUnit+0xdc` bit 15
    /// (`set_flag_15`), then, when bit 30 of the same dword is set, walks the
    /// eight entries at `CUnit+0xb0..=+0xbf` and raises bit 15 once for every
    /// occupied queue slot.  With bit 30 clear it raises bit 15 once when the
    /// `CUnit+0x70` attached-unit link is non-null.  The routine always
    /// returns 0.
    ///
    /// Bit 2 is modelled by [`World::is_air_unit`], the queue by
    /// [`SimUnit::production`], and bit 15 by [`SimUnit::set_flag_15`].  Bit
    /// 30 and the `+0x70` link have no Rust field, so the caller supplies
    /// them.  The returned value counts the latch writes (retail reports 0).
    pub fn a35_latch_queue_flag(
        &mut self,
        i: usize,
        queue_flag_bit30: bool,
        has_attached_unit: bool,
    ) -> u8 {
        if i >= self.units.len() || self.is_air_unit(i) {
            return 0;
        }
        let mut latches = 0u8;
        self.units[i].set_flag_15();
        latches += 1;
        if queue_flag_bit30 {
            let slots = self.units[i].production.len().min(8);
            for _ in 0..slots {
                self.units[i].set_flag_15();
                latches = latches.saturating_add(1);
            }
        } else if has_attached_unit {
            self.units[i].set_flag_15();
            latches += 1;
        }
        latches
    }
}

#[cfg(test)]
mod tests_agent35 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::Order;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    /// `0x005d8000`: the recovered constant pair is the 0x1000-byte page
    /// stride, so a request covers one page per started 4 KiB block.
    #[test]
    fn stack_probe_pages_round_up_per_page() {
        assert_eq!(a35_stack_probe_pages(0), 0);
        assert_eq!(a35_stack_probe_pages(1), 1);
        assert_eq!(a35_stack_probe_pages(0xfff), 1);
        assert_eq!(a35_stack_probe_pages(0x1000), 1);
        assert_eq!(a35_stack_probe_pages(0x1001), 2);
    }

    /// `0x005d8027`: the shared epilogue processes every node in link order
    /// and clears the list.
    #[test]
    fn pending_ids_drain_in_order() {
        let mut pending = VecDeque::from([7, 9, 11]);
        assert_eq!(a35_drain_pending_ids(&mut pending), vec![7, 9, 11]);
        assert!(pending.is_empty());
        assert!(a35_drain_pending_ids(&mut pending).is_empty());
    }

    /// `0x005d8a19`: the mask keeps the source bits and raises every bit
    /// except `0x1a`.
    #[test]
    fn reserved_bits_mask_is_complement_of_1a() {
        assert_eq!(a35_or_reserved_bits(0), 0xffff_ffe5);
        assert_eq!(a35_or_reserved_bits(0x1a), 0xffff_ffff);
        assert_eq!(a35_or_reserved_bits(0x1a), a35_or_reserved_bits(0) | 0x1a);
        assert_eq!(a35_or_reserved_bits(0xffff_ffff), 0xffff_ffff);
    }

    /// `0x005d8a39`: the body is a bare return.
    #[test]
    fn empty_body_is_a_no_op() {
        a35_empty_body();
    }

    /// `0x005d9cd3`: the body is a bare `ret 0`.
    #[test]
    fn empty_stdcall_stub_is_a_no_op() {
        a35_empty_stdcall_stub();
    }

    /// `0x005d9cf0`: the whole body is the non-returning rejection path.
    #[test]
    fn unconditional_reject_returns_error() {
        assert_eq!(a35_reject_unconditionally(), Err(SimError::InvalidTarget));
    }

    /// `0x005d834c`: the carry wraps from zero to eight and reports the
    /// second-stage arm only on that reset.
    #[test]
    fn order_carry_resets_at_zero() {
        let mut w = world();
        let u = w.spawn(0, uid::MARINE, 20, 20).unwrap();

        assert_eq!(w.units[u].order_carry, 0);
        assert!(w.units[u].a35_advance_order_carry());
        assert_eq!(w.units[u].order_carry, 8);

        assert!(!w.units[u].a35_advance_order_carry());
        assert_eq!(w.units[u].order_carry, 7);

        w.units[u].order_carry = 3;
        assert!(!w.units[u].a35_advance_order_carry());
        assert_eq!(w.units[u].order_carry, 2);

        w.units[u].order_carry = 0;
        assert!(w.units[u].a35_advance_order_carry());
        assert_eq!(w.units[u].order_carry, 8);
    }

    /// `0x005d8f70`: the same pixel is accepted, a linked unit in order 0x60
    /// (or airborne) is refused, and every other linked ground case is the
    /// retail abort path.
    #[test]
    fn reposition_guard_matches_order_and_air_rules() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 20, 20).unwrap();
        assert_eq!(
            w.a35_reposition_guard(marine, (20, 20), false),
            A35RepositionGuard::AlreadyThere
        );

        assert_eq!(
            w.a35_reposition_guard(marine, (40, 40), true),
            A35RepositionGuard::Invalid
        );
        assert_eq!(
            w.a35_reposition_guard(marine, (40, 40), false),
            A35RepositionGuard::Invalid
        );

        w.units[marine].order_row = 0x60;
        assert_eq!(
            w.a35_reposition_guard(marine, (40, 40), true),
            A35RepositionGuard::Refused
        );

        let wraith = w.spawn(0, uid::WRAITH, 60, 20).unwrap();
        assert_eq!(
            w.a35_reposition_guard(wraith, (80, 80), true),
            A35RepositionGuard::Refused
        );
        assert_eq!(
            w.a35_reposition_guard(wraith, (80, 80), false),
            A35RepositionGuard::Invalid
        );

        assert_eq!(
            w.a35_reposition_guard(w.units.len(), (0, 0), true),
            A35RepositionGuard::Invalid
        );
    }

    /// `0x005d8fc5`: both ends clamp with the unit box, the position is
    /// stored in pixels, flag 19 is cleared and the weapon-table value is the
    /// reported `+0x20` bit.
    #[test]
    fn place_unit_clamps_to_box_and_map_bounds() {
        let mut w = world();
        let u = w.spawn(0, uid::MARINE, 20, 20).unwrap();
        let type_id = w.units[u].type_id;
        let left = w.unit_box_left(type_id);
        let top = w.unit_box_top(type_id);
        let right = w.unit_box_right(type_id);
        let bottom = w.unit_box_bottom(type_id);
        let map_w = w.terrain.walk_width as i32 * 8;
        let map_h = w.terrain.walk_height as i32 * 8;

        w.units[u].flag_19 = true;
        assert!(w.a35_place_unit_clamped(u, (-50, -50), true));
        assert_eq!(w.units[u].x / fp(1), left);
        assert_eq!(w.units[u].y / fp(1), top);
        assert!(!w.units[u].flag_19);

        w.units[u].flag_19 = true;
        assert!(!w.a35_place_unit_clamped(u, (map_w + 50, map_h + 50), false));
        assert_eq!(w.units[u].x / fp(1), map_w - right - 1);
        assert_eq!(w.units[u].y / fp(1), map_h - bottom - 0x21);
        assert!(!w.units[u].flag_19);

        // Inside the box the requested pixel survives unchanged.
        assert!(w.a35_place_unit_clamped(u, (100, 120), true));
        assert_eq!(w.units[u].x / fp(1), 100);
        assert_eq!(w.units[u].y / fp(1), 120);
    }

    /// `0x005d90a9`: the reset clears flag 19 on both arms and reports the
    /// weapon-slot table bit.
    #[test]
    fn retry_reset_clears_flag_19_and_reports_visual_bit() {
        let mut w = world();
        let u = w.spawn(0, uid::MARINE, 20, 20).unwrap();
        w.units[u].flag_19 = true;
        assert!(w.units[u].a35_reset_retry_visual(true));
        assert!(!w.units[u].flag_19);

        w.units[u].flag_19 = true;
        assert!(!w.units[u].a35_reset_retry_visual(false));
        assert!(!w.units[u].flag_19);
    }

    /// `0x005d94e0`: the diagnostic code is only produced while the latch is
    /// set.
    #[test]
    fn diagnostic_secondary_order_needs_latch() {
        let mut w = world();
        let u = w.spawn(0, uid::MARINE, 20, 20).unwrap();
        assert_eq!(w.units[u].a35_diagnostic_secondary_order(false, 0x2a), None);
        assert_eq!(
            w.units[u].a35_diagnostic_secondary_order(true, 0x2a),
            Some(0x2a)
        );
    }

    /// `0x005d9c90`: airborne units are skipped, the first latch always
    /// happens on the ground, bit 30 scans queue slots and the attached-unit
    /// arm only runs when bit 30 is clear.
    #[test]
    fn latch_queue_flag_counts_ground_and_queue_latches() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 20, 20).unwrap();
        assert_eq!(w.a35_latch_queue_flag(marine, false, false), 1);
        assert!(w.units[marine].flag_15);

        w.units[marine].flag_15 = false;
        w.units[marine].production.push_back(Order::Train {
            unit_type: uid::MARINE,
        });
        w.units[marine].production.push_back(Order::Train {
            unit_type: uid::MARINE,
        });
        w.units[marine].production.push_back(Order::Train {
            unit_type: uid::MARINE,
        });
        assert_eq!(w.a35_latch_queue_flag(marine, true, false), 4);
        assert!(w.units[marine].flag_15);

        w.units[marine].flag_15 = false;
        assert_eq!(w.a35_latch_queue_flag(marine, false, true), 2);
        assert!(w.units[marine].flag_15);

        let wraith = w.spawn(0, uid::WRAITH, 60, 20).unwrap();
        assert_eq!(w.a35_latch_queue_flag(wraith, true, true), 0);
        assert!(!w.units[wraith].flag_15);
    }
}
