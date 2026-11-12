#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent16.rs
// ===========================================================================
// Translation batch 16.  Only this file is edited by the batch.
//
// Routines recovered from `0x005c0000..=0x005c3fff`, the retail
// command-cursor / target-selection module.  Every item below names the exact
// decompiled entry address it reconstructs.
//
// The predicates in this range read four `CUnit` bytes that the Rust model
// does not store (`CUnit+0xdc` bits 1, 20 and 23, and the internal order byte
// at `CUnit+0x4d`).  They are supplied through [`A16UnitState`] rather than
// adding fields to [`SimUnit`]; every modelled field is read through the
// existing accessors.
//
// Several ledger rows in this range are not routine entries.  The decompiler
// split single routines at internal jump targets (`0x005c05f8` and
// `0x005c0631` inside `0x005c05d0`, `0x005c12e0` inside `0x005c12c0`,
// `0x005c23c9` inside `0x005c23a0`) or picked up mid-instruction addresses
// (`FUN_005c0229`, `FUN_005c0969`, `FUN_005c0d81`, `FUN_005c149c`, `FUN_005c315c`,
// `FUN_005c3cf4`, `FUN_005c3e11`).  The remaining rows (`FUN_005c03a4`,
// `FUN_005c0498`, `FUN_005c0b7f`, `FUN_005c1109`, `FUN_005c1148`) sit inside
// routines that are larger than this batch's size filter.  The routines
// translated here cover the internal chunk rows.

/// `CUnit` state the batch-16 target predicates read that [`SimUnit`] does
/// not model.
///
/// All three flags live in the retail `CUnit+0xdc` bit word and the order byte
/// is the retail "current order id" byte at `CUnit+0x4d`.  [`SimUnit`] keeps
/// `CUnit+0xdc` bits 19, 21, 22 and 26 and the numeric `orders.dat` row
/// ([`SimUnit::order_row`]); the values below are the remaining ones, passed
/// in by the caller so no persistent state is added.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A16UnitState {
    /// `CUnit+0xdc` bit 1.  Gates the Larva candidate branch and
    /// short-circuits the pair-filter tail.
    pub bit_1: bool,
    /// `CUnit+0xdc` bit 20.  The candidate-side terminal condition of the
    /// near-cursor searches, and the entry gate of the gated variant.
    pub bit_20: bool,
    /// `CUnit+0xdc` bit 23.  One half of the shared pair-filter tail.
    pub bit_23: bool,
    /// `CUnit+0x4d`, the retail internal order byte.  The filters compare it
    /// with `0x52` (`'R'`), `0x54` (`'T'`) and `0x42` (`'B'`).
    pub order_byte: u8,
}

/// Result of the `CUnit+0xe2` target-retention step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A16TargetStep {
    /// Next value of the `CUnit+0xe2` state byte.
    pub state: u8,
    /// Sound cue the retail routine hands to its cue helper, if any.
    pub cue: Option<u16>,
}

fn a16_state(extras: &[A16UnitState], slot: usize) -> A16UnitState {
    extras.get(slot).copied().unwrap_or_default()
}

impl World {
    /// `0x005c12c0`: `units.dat` bounding-box intersection test.
    ///
    /// The routine takes the first unit's box from its own record (the four
    /// `units.dat` box extents read by [`World::unit_box_left`],
    /// [`World::unit_box_top`], [`World::unit_box_right`] and
    /// [`World::unit_box_bottom`]) and the second unit's box from an explicit
    /// type id.  With boxes `[x - left, x + right]` and
    /// `[y - top, y + bottom]` it returns true exactly when the two boxes
    /// overlap:
    ///
    /// * `x2 >= x1 - self.left - other.right`
    /// * `x2 <= x1 + self.right + other.left`
    /// * `y2 >= y1 - self.top - other.bottom`
    /// * `y2 <= y1 + self.bottom + other.top`
    ///
    /// Retail compares the four bounds as signed 16-bit values; world
    /// coordinates fit in `i16`, so the `i32` arithmetic here is exact.
    /// Also covers the interior ledger row `0x005c12e0`, a split of this
    /// routine's tail.
    pub fn a16_unit_boxes_overlap(
        &self,
        self_type: u16,
        self_at: (i32, i32),
        other_type: u16,
        other_at: (i32, i32),
    ) -> bool {
        let (x1, y1) = self_at;
        let (x2, y2) = other_at;
        x2 >= x1 - self.unit_box_left(self_type) - self.unit_box_right(other_type)
            && x2 <= x1 + self.unit_box_right(self_type) + self.unit_box_left(other_type)
            && y2 >= y1 - self.unit_box_top(self_type) - self.unit_box_bottom(other_type)
            && y2 <= y1 + self.unit_box_bottom(self_type) + self.unit_box_top(other_type)
    }

    /// `0x005c13d0`: "the point lies on the perimeter of the unit's click
    /// box".
    ///
    /// The unit's box comes from its own record (`left/top/right/bottom`
    /// half-extents around the pixel position) and is widened in place:
    ///
    /// * left edge  -= `right + 1`
    /// * right edge += `left + 1`
    /// * top edge   -= `bottom + 1`
    /// * the right edge is then grown again by `top + 1` (the retail code adds
    ///   the *top* extent to the right edge; that quirk is reproduced
    ///   verbatim, the bottom edge itself is left at `y + bottom`).
    ///
    /// A point passes when it sits on one of the two horizontal edges and
    /// within the other axis' span, or on one of the two vertical edges and
    /// within that span.  An interior point (on neither edge) fails.
    pub fn a16_point_on_unit_perimeter(&self, unit: usize, point: (i32, i32)) -> bool {
        let u = &self.units[unit];
        let left = self.unit_box_left(u.type_id);
        let right = self.unit_box_right(u.type_id);
        let top = self.unit_box_top(u.type_id);
        let bottom = self.unit_box_bottom(u.type_id);
        let x = u.x / fp(1);
        let y = u.y / fp(1);
        let lo_x = x - left - right - 1;
        let hi_x = x + right + left + top + 2;
        let lo_y = y - top - bottom - 1;
        let hi_y = y + bottom;
        let (px, py) = point;
        if px == lo_x || px == hi_x {
            return py >= lo_y && py <= hi_y;
        }
        if py == lo_y || py == hi_y {
            return px >= lo_x && px <= hi_x;
        }
        false
    }

    /// `0x005c1c00`: the shared pair filter used by the cursor searches.
    /// `a` is the candidate, `b` the unit that owns the search.
    ///
    /// 1. `a` flagged `CUnit+0xdc` bit 22 and `b` on the opposing side fails
    ///    (retail reads the alliance table through the `CUnit+0x4c` owners;
    ///    [`World::is_enemy`] is the model).
    /// 2. Larva (`units.dat` row `0x23`) and non-Larva never pass each other;
    ///    a Larva candidate additionally requires `a`'s `CUnit+0xdc` bit 1.
    /// 3. With `b`'s bit 23 clear the pair passes, and with `a`'s bit 1 set it
    ///    passes as well.
    /// 4. Otherwise `a` needs bit 23, `b`'s order byte must not be `0x54`
    ///    (`'T'`) and the result is `a`'s order byte being `0x52` (`'R'`).
    pub fn a16_pair_filter(
        &self,
        a: &SimUnit,
        a_state: A16UnitState,
        b: &SimUnit,
        b_state: A16UnitState,
    ) -> bool {
        if a.flag_22() && self.is_enemy(a.owner, b.owner) {
            return false;
        }
        if b.type_id == uid::LARVA && !a_state.bit_1 {
            return false;
        }
        if b.type_id != uid::LARVA && a.type_id == uid::LARVA {
            return false;
        }
        if !b_state.bit_23 {
            return true;
        }
        if a_state.bit_1 {
            return true;
        }
        if !a_state.bit_23 {
            return false;
        }
        if b_state.order_byte == 0x54 {
            return false;
        }
        a_state.order_byte == 0x52
    }

    /// `0x005c1b40`: search-side wrapper around [`World::a16_pair_filter`].
    ///
    /// The unit must carry `CUnit+0x105` bit 0 and must not have
    /// `CUnit+0xdc` bit 21.  A null `other` passes, an `other` equal to the
    /// unit itself fails, and otherwise the pair filter decides.
    pub fn a16_target_filter(
        &self,
        unit: usize,
        other: Option<usize>,
        extras: &[A16UnitState],
    ) -> bool {
        let a = &self.units[unit];
        if !a.flag_105_bit0() || a.flag_21() {
            return false;
        }
        let Some(other) = other else {
            return true;
        };
        if other == unit {
            return false;
        }
        let b = &self.units[other];
        self.a16_pair_filter(a, a16_state(extras, unit), b, a16_state(extras, other))
    }

    /// `0x005c1380`: the command cursor's "this unit may be the target of the
    /// pending command" gate.
    ///
    /// The routine runs only while the unit has arrived at its move target
    /// (the retail `CUnit+0x10`/`+0x12` pixel pair matches the unit position,
    /// modelled by [`SimUnit::path_goal`] pointing at the unit's own
    /// position), or the unit is the pending command target (the
    /// pending-target global), or the caller forces the check (the forcing
    /// global).
    /// It then rejects the four door types (`0xcd..=0xd0`) and applies
    /// [`World::a16_target_filter`].
    pub fn a16_command_target_allowed(
        &self,
        unit: usize,
        other: Option<usize>,
        pending: Option<usize>,
        force: bool,
        extras: &[A16UnitState],
    ) -> bool {
        let u = &self.units[unit];
        let at_target = u.path_goal == Some((u.x, u.y));
        if !(at_target || pending == Some(unit) || force) {
            return false;
        }
        if is_door_type(u.type_id) {
            return false;
        }
        self.a16_target_filter(unit, other, extras)
    }

    /// `0x005c05d0`: find a unit near the cursor that the owning unit may act
    /// on.
    ///
    /// The retail routine asks the shared spatial-grid cursor (the retail
    /// grid query) for the units in the grid line between the unit and
    /// the cursor, keeps the candidates that carry `CUnit+0x105` bit 0, do
    /// not have `CUnit+0xdc` bit 21, are not the owning unit itself and pass
    /// [`World::a16_pair_filter`], and returns the first one whose
    /// `CUnit+0xdc` bit 20 is set.  The grid cursor and its scratch list are
    /// global state the Rust world does not keep, so this translation scans
    /// the rectangle spanned by the unit and the cursor in slot order; the
    /// caller only distinguishes "found" from "not found".  Also covers the
    /// interior ledger rows `0x005c05f8` and `0x005c0631`, splits of this
    /// routine's filter loop.
    pub fn a16_find_target_near_cursor(
        &self,
        unit: usize,
        cursor: (i32, i32),
        extras: &[A16UnitState],
    ) -> Option<u32> {
        let owner = &self.units[unit];
        let from = (owner.x / fp(1), owner.y / fp(1));
        let (x0, x1) = if from.0 <= cursor.0 {
            (from.0, cursor.0)
        } else {
            (cursor.0, from.0)
        };
        let (y0, y1) = if from.1 <= cursor.1 {
            (from.1, cursor.1)
        } else {
            (cursor.1, from.1)
        };
        for (slot, cand) in self.units.iter().enumerate() {
            if !cand.alive || slot == unit {
                continue;
            }
            let at = (cand.x / fp(1), cand.y / fp(1));
            if at.0 < x0 || at.0 > x1 || at.1 < y0 || at.1 > y1 {
                continue;
            }
            if !cand.flag_105_bit0() || cand.flag_21() {
                continue;
            }
            let cand_state = a16_state(extras, slot);
            if !self.a16_pair_filter(cand, cand_state, owner, a16_state(extras, unit)) {
                continue;
            }
            if cand_state.bit_20 {
                return Some(cand.id);
            }
        }
        None
    }

    /// `0x005c0730`: [`World::a16_find_target_near_cursor`] behind the
    /// `CUnit+0xdc` bit-20 entry gate.
    ///
    /// The only difference from the ungated search is the first check: the
    /// searching unit must itself have bit 20 set, otherwise the routine
    /// returns null before it touches the grid.
    pub fn a16_find_target_near_cursor_gated(
        &self,
        unit: usize,
        cursor: (i32, i32),
        extras: &[A16UnitState],
    ) -> Option<u32> {
        if !a16_state(extras, unit).bit_20 {
            return None;
        }
        self.a16_find_target_near_cursor(unit, cursor, extras)
    }
}

/// `0x005c1820`: select entry `index` from a variable-length word table.
///
/// The retail table starts with a `u16` count, then one reserved word, then
/// the entries; the routine stores the requested index, rejects the request
/// when `index > count`, and stores the selected entry when it is nonzero.
/// It returns whether an entry was selected and the selected value, so the
/// index and selection globals it would write stay with the caller.
pub fn a16_select_indexed_entry(count: u16, entries: &[u16], index: u8) -> Option<u16> {
    let entry = *entries.get(usize::from(index))?;
    (u16::from(index) <= count && entry != 0).then_some(entry)
}

impl SimUnit {
    /// `0x005c23a0`: three-state target-retention step for `CUnit+0xe2`.
    ///
    /// State 0 with a live `CUnit+0x5c` target ([`SimUnit::order_unit_target`])
    /// plays cue `0x13` and moves to state 1.  State 1 keeps the target while
    /// the target's `CUnit+0x4d` order byte is `0x42` (`'B'`); otherwise it
    /// drops the target and moves to state 2.  State 2 plays cue `0x14` and
    /// returns to state 0 when there is no target, and goes back to state 1
    /// while a target is present.  Any other state value is left alone.
    ///
    /// Dropping the target clears the order, matching the retail store of zero
    /// into `CUnit+0x5c`; the state byte itself is returned instead of being
    /// stored because [`SimUnit`] has no `CUnit+0xe2` field.  Also covers the
    /// interior ledger row `0x005c23c9`, a split inside this routine.
    pub fn a16_target_state_step(&mut self, state: u8, target_order_byte: u8) -> A16TargetStep {
        let has_target = self.order_unit_target().is_some();
        let (next, cue) = match state {
            0 => {
                if has_target {
                    (1, Some(0x13))
                } else {
                    (0, None)
                }
            }
            1 => {
                if !has_target {
                    (2, None)
                } else if target_order_byte == 0x42 {
                    (1, None)
                } else {
                    self.order = Order::Idle;
                    (2, None)
                }
            }
            2 => {
                if has_target {
                    (1, None)
                } else {
                    (0, Some(0x14))
                }
            }
            other => (other, None),
        };
        A16TargetStep { state: next, cue }
    }
}

#[cfg(test)]
mod tests_agent16 {
    use super::*;
    use crate::data::GameData;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, owner: usize, type_id: u16, x: i32, y: i32) -> usize {
        world.spawn(owner, type_id, x, y).expect("spawn")
    }

    fn state(bit_1: bool, bit_20: bool, bit_23: bool, order_byte: u8) -> A16UnitState {
        A16UnitState {
            bit_1,
            bit_20,
            bit_23,
            order_byte,
        }
    }

    #[test]
    fn boxes_overlap_uses_the_units_dat_extents() {
        let w = world();
        let t = uid::MARINE;
        let l = w.unit_box_left(t);
        let r = w.unit_box_right(t);
        let top = w.unit_box_top(t);
        let bottom = w.unit_box_bottom(t);
        assert!(l > 0 && r > 0 && top > 0 && bottom > 0);

        assert!(w.a16_unit_boxes_overlap(t, (100, 100), t, (100, 100)));
        // Exactly touching on the right/left and bottom/top edges.
        assert!(w.a16_unit_boxes_overlap(t, (100, 100), t, (100 + r + l, 100)));
        assert!(w.a16_unit_boxes_overlap(t, (100, 100), t, (100 - r - l, 100)));
        assert!(w.a16_unit_boxes_overlap(t, (100, 100), t, (100, 100 + bottom + top)));
        assert!(w.a16_unit_boxes_overlap(t, (100, 100), t, (100, 100 - bottom - top)));
        // One pixel of separation on each axis.
        assert!(!w.a16_unit_boxes_overlap(t, (100, 100), t, (100 + r + l + 1, 100)));
        assert!(!w.a16_unit_boxes_overlap(t, (100, 100), t, (100 - r - l - 1, 100)));
        assert!(!w.a16_unit_boxes_overlap(t, (100, 100), t, (100, 100 - bottom - top - 1)));
        // A large building's box reaches a marine further away.
        let cc = uid::COMMAND_CENTER;
        let gap = 100 + w.unit_box_right(cc) + l;
        assert!(w.a16_unit_boxes_overlap(cc, (100, 100), t, (gap, 100)));
        assert!(!w.a16_unit_boxes_overlap(cc, (100, 100), t, (gap + 1, 100)));
    }

    #[test]
    fn perimeter_test_accepts_edge_points_only() {
        let mut w = world();
        let slot = spawn(&mut w, 0, uid::MARINE, 100, 100);
        let t = w.units[slot].type_id;
        let l = w.unit_box_left(t);
        let r = w.unit_box_right(t);
        let top = w.unit_box_top(t);
        let bottom = w.unit_box_bottom(t);
        let lo_x = 100 - l - r - 1;
        let hi_x = 100 + r + l + top + 2;
        let lo_y = 100 - top - bottom - 1;
        let hi_y = 100 + bottom;
        assert!(lo_x < 100 && 100 < hi_x && lo_y < 100 && 100 < hi_y);

        assert!(w.a16_point_on_unit_perimeter(slot, (lo_x, 100)));
        assert!(w.a16_point_on_unit_perimeter(slot, (hi_x, 100)));
        assert!(w.a16_point_on_unit_perimeter(slot, (100, lo_y)));
        assert!(w.a16_point_on_unit_perimeter(slot, (100, hi_y)));
        // Interior points are not on the perimeter.
        assert!(!w.a16_point_on_unit_perimeter(slot, (100, 100)));
        // Corner combinations outside the orthogonal span fail.
        assert!(!w.a16_point_on_unit_perimeter(slot, (lo_x, lo_y - 1)));
        assert!(!w.a16_point_on_unit_perimeter(slot, (hi_x + 1, 100)));
        assert!(!w.a16_point_on_unit_perimeter(slot, (100 + bottom + 1, 100)));
    }

    #[test]
    fn pair_filter_follows_the_retail_chain() {
        let mut w = world();
        let a = spawn(&mut w, 0, uid::MARINE, 100, 100);
        let b = spawn(&mut w, 0, uid::MARINE, 200, 200);
        let d = A16UnitState::default();
        assert!(w.a16_pair_filter(&w.units[a], d, &w.units[b], d));

        // bit 22 plus an enemy owner fails; the same side continues.
        let mut held = w.units[a].clone();
        held.flag_22 = true;
        let mut enemy = w.units[b].clone();
        enemy.owner = 1;
        assert!(!w.a16_pair_filter(&held, d, &enemy, d));
        assert!(w.a16_pair_filter(&held, d, &w.units[b], d));
    }

    #[test]
    fn pair_filter_treats_larva_as_its_own_class() {
        let mut w = world();
        let a = spawn(&mut w, 0, uid::MARINE, 100, 100);
        let larva = spawn(&mut w, 0, uid::LARVA, 120, 120);
        let d = A16UnitState::default();
        // A Larva candidate needs bit 1 on the searching unit.
        assert!(!w.a16_pair_filter(&w.units[a], d, &w.units[larva], d));
        assert!(w.a16_pair_filter(
            &w.units[a],
            state(true, false, false, 0),
            &w.units[larva],
            d
        ));
        // A Larva search unit never matches a non-Larva candidate.
        assert!(!w.a16_pair_filter(
            &w.units[larva],
            state(true, false, false, 0),
            &w.units[a],
            d
        ));
        // Larva against Larva is decided by the tail only.
        assert!(w.a16_pair_filter(
            &w.units[larva],
            state(true, false, false, 0),
            &w.units[larva],
            d
        ));
    }

    #[test]
    fn pair_filter_tail_uses_bits_23_and_the_order_byte() {
        let mut w = world();
        let a = spawn(&mut w, 0, uid::MARINE, 100, 100);
        let b = spawn(&mut w, 0, uid::MARINE, 200, 200);
        let none = A16UnitState::default();
        // Candidate bit 23 clear: always passes.
        assert!(w.a16_pair_filter(&w.units[a], none, &w.units[b], none));
        // Searching unit bit 1 short-circuits the tail.
        assert!(w.a16_pair_filter(
            &w.units[a],
            state(true, false, false, 0),
            &w.units[b],
            state(false, false, true, 0)
        ));
        // Candidate bit 23 set, unit bit 1 clear, unit bit 23 clear: fails.
        assert!(!w.a16_pair_filter(&w.units[a], none, &w.units[b], state(false, false, true, 0)));
        // Unit bit 23 set, candidate order byte 'T': fails.
        assert!(!w.a16_pair_filter(
            &w.units[a],
            state(false, false, true, 0x52),
            &w.units[b],
            state(false, false, true, 0x54)
        ));
        // Unit order byte 'R' passes, anything else fails.
        assert!(w.a16_pair_filter(
            &w.units[a],
            state(false, false, true, 0x52),
            &w.units[b],
            state(false, false, true, 0)
        ));
        assert!(!w.a16_pair_filter(
            &w.units[a],
            state(false, false, true, 0x51),
            &w.units[b],
            state(false, false, true, 0)
        ));
    }

    #[test]
    fn target_filter_checks_the_unit_flags_first() {
        let mut w = world();
        let unit = spawn(&mut w, 0, uid::MARINE, 100, 100);
        let other = spawn(&mut w, 0, uid::MARINE, 120, 100);
        assert!(!w.a16_target_filter(unit, Some(other), &[]));
        w.units[unit].flags_105 = 1;
        assert!(w.a16_target_filter(unit, Some(other), &[]));
        assert!(w.a16_target_filter(unit, None, &[]));
        assert!(!w.a16_target_filter(unit, Some(unit), &[]));
        w.units[unit].flag_21 = true;
        assert!(!w.a16_target_filter(unit, Some(other), &[]));
        assert!(!w.a16_target_filter(unit, None, &[]));
    }

    #[test]
    fn command_gate_requires_arrival_pending_or_force() {
        let mut w = world();
        let unit = spawn(&mut w, 0, uid::MARINE, 100, 100);
        let other = spawn(&mut w, 0, uid::MARINE, 120, 100);
        w.units[unit].flags_105 = 1;
        assert!(!w.a16_command_target_allowed(unit, Some(other), None, false, &[]));
        assert!(w.a16_command_target_allowed(unit, Some(other), Some(unit), false, &[]));
        assert!(w.a16_command_target_allowed(unit, Some(other), None, true, &[]));
        // Arriving at the stored move target opens the gate as well.
        let pos = (w.units[unit].x, w.units[unit].y);
        w.units[unit].path_goal = Some(pos);
        assert!(w.a16_command_target_allowed(unit, Some(other), None, false, &[]));
        // A door unit is never a valid command target.
        let door = spawn(&mut w, 0, 0xcd, 140, 100);
        w.units[door].flags_105 = 1;
        w.units[door].path_goal = Some((w.units[door].x, w.units[door].y));
        assert!(!w.a16_command_target_allowed(door, Some(other), None, false, &[]));
    }

    #[test]
    fn near_cursor_search_returns_the_first_armed_candidate() {
        let mut w = world();
        let unit = spawn(&mut w, 0, uid::MARINE, 100, 100);
        let cand = spawn(&mut w, 0, uid::MARINE, 120, 100);
        w.units[cand].flags_105 = 1;
        let extras = [A16UnitState::default(), state(false, true, false, 0)];
        assert_eq!(
            w.a16_find_target_near_cursor(unit, (130, 100), &extras),
            Some(w.units[cand].id)
        );
        // Candidate bit 20 clear: no result.
        assert_eq!(w.a16_find_target_near_cursor(unit, (130, 100), &[]), None);
        // Candidate outside the unit/cursor rectangle: no result.
        assert_eq!(
            w.a16_find_target_near_cursor(unit, (100, 200), &extras),
            None
        );
        // Candidate flag 21 blocks the search.
        w.units[cand].flag_21 = true;
        assert_eq!(
            w.a16_find_target_near_cursor(unit, (130, 100), &extras),
            None
        );
    }

    #[test]
    fn gated_near_cursor_search_needs_bit_20_on_the_searching_unit() {
        let mut w = world();
        let unit = spawn(&mut w, 0, uid::MARINE, 100, 100);
        let cand = spawn(&mut w, 0, uid::MARINE, 120, 100);
        w.units[cand].flags_105 = 1;
        let inert = [A16UnitState::default(), state(false, true, false, 0)];
        assert_eq!(
            w.a16_find_target_near_cursor_gated(unit, (130, 100), &inert),
            None
        );
        let active = [state(false, true, false, 0), state(false, true, false, 0)];
        assert_eq!(
            w.a16_find_target_near_cursor_gated(unit, (130, 100), &active),
            Some(w.units[cand].id)
        );
    }

    #[test]
    fn indexed_table_selection_rejects_out_of_range_and_zero_entries() {
        let entries = [5u16, 0, 7, 9];
        assert_eq!(a16_select_indexed_entry(3, &entries, 0), Some(5));
        assert_eq!(a16_select_indexed_entry(3, &entries, 1), None);
        assert_eq!(a16_select_indexed_entry(3, &entries, 2), Some(7));
        assert_eq!(a16_select_indexed_entry(3, &entries, 3), Some(9));
        assert_eq!(a16_select_indexed_entry(2, &entries, 3), None);
        assert_eq!(a16_select_indexed_entry(9, &entries, 4), None);
    }

    #[test]
    fn target_state_machine_plays_cues_and_drops_stale_targets() {
        let mut w = world();
        let unit = spawn(&mut w, 0, uid::MARINE, 100, 100);
        let victim = spawn(&mut w, 1, uid::MARINE, 120, 100);
        let victim_id = w.units[victim].id;

        // State 0 without a target does nothing.
        assert_eq!(
            w.units[unit].a16_target_state_step(0, 0),
            A16TargetStep {
                state: 0,
                cue: None
            }
        );
        // State 0 with a target arms the cue and advances.
        w.units[unit].order = Order::AttackUnit { target: victim_id };
        assert_eq!(
            w.units[unit].a16_target_state_step(0, 0),
            A16TargetStep {
                state: 1,
                cue: Some(0x13)
            }
        );
        // State 1 keeps a target whose order byte is 'B'.
        assert_eq!(
            w.units[unit].a16_target_state_step(1, 0x42),
            A16TargetStep {
                state: 1,
                cue: None
            }
        );
        assert!(w.units[unit].order_unit_target().is_some());
        // Any other order byte drops the target and moves on.
        assert_eq!(
            w.units[unit].a16_target_state_step(1, 0x00),
            A16TargetStep {
                state: 2,
                cue: None
            }
        );
        assert_eq!(w.units[unit].order_unit_target(), None);
        // State 2 without a target fires the second cue and resets.
        assert_eq!(
            w.units[unit].a16_target_state_step(2, 0),
            A16TargetStep {
                state: 0,
                cue: Some(0x14)
            }
        );
        // State 2 with a target returns to state 1.
        w.units[unit].order = Order::AttackUnit { target: victim_id };
        assert_eq!(
            w.units[unit].a16_target_state_step(2, 0),
            A16TargetStep {
                state: 1,
                cue: None
            }
        );
        // Unknown states are ignored.
        assert_eq!(
            w.units[unit].a16_target_state_step(7, 0x42),
            A16TargetStep {
                state: 7,
                cue: None
            }
        );
    }
}
