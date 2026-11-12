#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent08.rs
// ===========================================================================
// Translation batch 08.  Only this file is edited by the batch.
//
// Ten `decompiled_uncited` routines from the `0x005c0000..=0x005c7fff`
// client helper region (trigger/ability and unit-order predicates).  Every
// item below names the exact retail address it was recovered from.
//
// The recovered code expresses several failures with the non-returning
// assertion helpers `FUN_005b4cd0` / `FUN_0058b640`.  Those paths are
// reported as `Result::Err(())` instead of aborting the host process, so
// the observable predicate remains testable without killing the test
// runner.

impl SimUnit {
    /// `0x005c4da0`: true when `CUnit+0x4d` (the one-byte `orders.dat` row
    /// id) is `0x6a` and `CUnit+0x4e` (the order state byte) is zero.
    ///
    /// `0x6a` is orders.dat row 106, Hold Position (`Order::order_id`
    /// returns 106 for [`Order::HoldPosition`]).  The Rust order model has
    /// no separate hold sub-state, so the state-zero branch is the plain
    /// hold order.
    pub fn a08_is_hold_position_idle(&self) -> bool {
        matches!(self.order, Order::HoldPosition)
    }

    /// `0x005c7331`: the 16-bit selector in `AX` matches this unit when
    /// `AX == 0x22` (orders.dat row 34, Place Add-On) or when the unit's
    /// `CUnit+0x4d` order byte lies in `0x7b..=0x81` (Nuke Train through
    /// Initializing Arbiter, rows 123..=129).
    pub fn a08_order_matches_addon_or_nuke(&self, selector: u16) -> bool {
        a08_order_code_matches_addon_or_nuke(selector, self.order.order_id())
    }
}

impl World {
    /// `0x005c2350`: tail of the Shield Battery recharge step whose main
    /// body is `FUN_005c2210`.
    ///
    /// `recharged_id` is the unit carrying the recharge order (`CUnit+0x60`
    /// shields, `CUnit+0x4e` order state); `battery_id` is the Shield
    /// Battery it points back to (`CUnit+0xa2` fixed-point energy).  Retail
    /// stops when the recharged pool reached `units.dat` maximum shields
    /// (`max << 8`) or the battery has no energy left.  On that path it
    /// clears the battery's `CUnit+0x5c` back-link when it still names the
    /// recharged unit and returns to the order dispatcher.
    ///
    /// Returns true when the pair finished (the retail non-tail path) and
    /// the back-link is now clear.
    pub fn a08_shield_battery_finish_tick(&mut self, recharged_id: u32, battery_id: u32) -> bool {
        let (Some(recharged), Some(battery)) =
            (self.index_of(recharged_id), self.index_of(battery_id))
        else {
            return false;
        };
        let max_shields = fp(self.data.unit(self.units[recharged].type_id).shield_points as i32);
        let pool_full = self.units[recharged].shields >= max_shields;
        let battery_empty = self.units[battery].energy_fixed() <= 0;
        if !(pool_full || battery_empty) {
            return false;
        }
        // `0x005c2350`: clear the back-link only when it still names the
        // recharged unit (`CUnit+0x5c == ESI`).
        if self.units[battery].target == Some(recharged_id) {
            self.units[battery].target = None;
        }
        true
    }

    /// `0x005c4dc0`: validation gate for a unit type's `units.dat` flag
    /// `0x80000`.  Retail reads the flag word through `DAT_00f8fa08`; a
    /// type without the flag is accepted (`Ok(true)`).
    ///
    /// With the flag set, retail walks the unit list (`DAT_00fa3724`) for
    /// any unit whose `CUnit+0x4c` owner equals the argument and asserts
    /// (`FUN_0058b640`) when one exists; this translation reports that as
    /// `Err(())`.  With no such owner it reports `Ok(false)`.
    pub fn a08_unit_type_owner_query(&self, type_id: u16, owner: u8) -> Result<bool, ()> {
        let Some(ty) = self.data.units.get(type_id as usize) else {
            return Ok(false);
        };
        if ty.flags & 0x0008_0000 == 0 {
            return Ok(true);
        }
        if self.units.iter().any(|u| u.owner == owner) {
            Err(())
        } else {
            Ok(false)
        }
    }

    /// `0x005c5f9a`: column counterpart of
    /// [`Self::a08_placement_tile_row_matches`].  Both coordinates are
    /// converted with the recovered pixel-to-tile expression
    /// `(v + ((v >> 31) & 31)) >> 5` (see [`a08_tile_index`]); the unit-side
    /// coordinate first subtracts the type's `units.dat` placement X
    /// (`DAT_00f90cb8`, stride 4).
    ///
    /// Retail returns zero when the two columns differ and calls the
    /// assertion `FUN_005b4cd0` when they coincide.  Coincidence is
    /// therefore `Err(())` here.
    pub fn a08_placement_columns_differ(
        &self,
        point_x: i32,
        unit_x: i32,
        type_id: u16,
    ) -> Result<(), ()> {
        let Some(ty) = self.data.units.get(type_id as usize) else {
            return Ok(());
        };
        let offset = ty.placement.x as i32;
        if a08_tile_index(point_x) == a08_tile_index(unit_x.wrapping_sub(offset)) {
            Err(())
        } else {
            Ok(())
        }
    }

    /// `0x005c5fcd`: true when the caller's Y coordinate and a unit-side Y
    /// coordinate, after subtracting the type's `units.dat` placement Y
    /// (`DAT_00f90cba`, stride 4), land on the same 32-pixel row.  This is
    /// the same signed rounding as `0x005c5f9a`; the routine returns 1 for
    /// a match and 0 otherwise.
    pub fn a08_placement_tile_row_matches(&self, point_y: i32, unit_y: i32, type_id: u16) -> bool {
        let Some(ty) = self.data.units.get(type_id as usize) else {
            return false;
        };
        let offset = ty.placement.y as i32;
        a08_tile_index(point_y) == a08_tile_index(unit_y.wrapping_sub(offset))
    }
}

/// `0x005c5060`: write bit 7 of the byte at `object+0xe` from the
/// parameter, preserving bits 0..=6 (`v = (v & 0x7f) | (param << 7)`).
/// The client keeps this flag on its sprite/iScript object; the helper is
/// kept as a pure byte operation because that object is not part of the
/// simulation world.
pub fn a08_set_sprite_flag_high_bit(flags: u8, set: bool) -> u8 {
    (flags & 0x7f) | ((set as u8) << 7)
}

/// `0x005c6329`: choose an entry from the first non-empty group of the six
/// 16-entry arrays based at `&DAT_00fa3740`.
///
/// `counts` is the parallel `DAT_00fa38c0` count array and `rng` is the
/// value returned by the retail RNG helper `FUN_0073f3b0(0x13)`.  A group
/// with count one returns slot zero; otherwise the slot is `rng % count`.
/// Retail returns the stored value at that slot, so the translation returns
/// `(group, slot)` for the caller's parallel value rows.  `None` means
/// every one of the six counts was zero (the retail loop then returns the
/// last zero count).
pub fn a08_first_nonempty_group_pick(counts: &[u32; 6], rng: u32) -> Option<(usize, u32)> {
    for (group, &count) in counts.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let slot = if count == 1 { 0 } else { rng % count };
        return Some((group, slot));
    }
    None
}

/// `0x005c65d0`: scan a candidate row and keep the strictly smallest cost
/// (retail calls the distance helper `FUN_00604060` for each candidate).
/// The comparison is `<`, so an equal later cost does not replace the
/// earlier one.  Retail returns the candidate array element; the index into
/// the caller's parallel cost row is returned here so the caller keeps
/// ownership of the values.
pub fn a08_cheapest_candidate(costs: &[u32]) -> Option<usize> {
    let mut best: Option<(u32, usize)> = None;
    for (index, &cost) in costs.iter().enumerate() {
        if best.is_none_or(|(previous, _)| cost < previous) {
            best = Some((cost, index));
        }
    }
    best.map(|(_, index)| index)
}

/// `0x005c7331` body: `AX` is 0x22 (orders.dat row 34, Place Add-On) or the
/// unit's `CUnit+0x4d` order id is in `0x7b..=0x81` (rows 123..=129, Nuke
/// Train through Initializing Arbiter).  The unit byte is passed raw so
/// callers can feed a recovered order id that has no typed [`Order`] row.
pub fn a08_order_code_matches_addon_or_nuke(selector: u16, order_id: u8) -> bool {
    selector == 0x22 || (0x7b..=0x81).contains(&order_id)
}

/// `0x005c740a`: the 16-bit selector is one of `0x7c`, `0x7d`, `0x90` or
/// `0xa2` (orders.dat rows Nuke Launch 124, Nuke Paint 125, cConsume 144
/// and Computer Return 162).  Every other value returns zero.
pub fn a08_is_nuke_or_special_row(selector: u16) -> bool {
    matches!(selector, 0x7c | 0x7d | 0x90 | 0xa2)
}

/// The signed 32-pixel row/column index used by `0x005c5f9a` and
/// `0x005c5fcd`.  Retail emits `(v + ((v >> 31) & 31)) >> 5`: for
/// non-negative pixels that is a plain floor division by 32, while a
/// negative pixel adds 31 first, so `-1..=-31` map to tile 0 and `-32`
/// maps to tile -1.  This is the retail conversion, not Euclidean division.
fn a08_tile_index(v: i32) -> i32 {
    v.wrapping_add((v >> 31) & 31) >> 5
}

#[cfg(test)]
mod tests_agent08 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn test_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    #[test]
    fn a08_hold_position_predicate_reads_the_order() {
        // `0x005c4da0`: order byte 0x6a with state zero.
        let mut w = test_world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        assert!(!w.units[marine].a08_is_hold_position_idle());
        w.issue(w.units[marine].id, Order::HoldPosition).unwrap();
        assert!(w.units[marine].a08_is_hold_position_idle());
        w.issue(w.units[marine].id, Order::Stop).unwrap();
        assert!(!w.units[marine].a08_is_hold_position_idle());
    }

    #[test]
    fn a08_shield_battery_tail_clears_the_back_link() {
        // `0x005c2350`: stop at a full pool or an empty battery.
        let mut w = test_world();
        let battery = w.spawn(0, uid::SHIELD_BATTERY, 320, 320).unwrap();
        let target = w.spawn(0, uid::ZEALOT, 340, 320).unwrap();
        let battery_id = w.units[battery].id;
        let target_id = w.units[target].id;
        let max = fp(w.data.unit(uid::ZEALOT).shield_points as i32);

        // Full pool, back-link set: finished, link cleared.
        w.units[target].shields = max;
        w.units[battery].target = Some(target_id);
        assert!(w.a08_shield_battery_finish_tick(target_id, battery_id));
        assert_eq!(w.units[battery].target, None);

        // Partial pool but an empty battery also finishes the pair.
        w.units[target].shields = max / 2;
        w.units[battery].set_energy_fixed(0);
        w.units[battery].target = Some(target_id);
        assert!(w.a08_shield_battery_finish_tick(target_id, battery_id));
        assert_eq!(w.units[battery].target, None);

        // Partial pool and energy left: the recharge continues, link stays.
        w.units[target].shields = max / 2;
        w.units[battery].set_energy_points(50);
        w.units[battery].target = Some(target_id);
        assert!(!w.a08_shield_battery_finish_tick(target_id, battery_id));
        assert_eq!(w.units[battery].target, Some(target_id));
    }

    #[test]
    fn a08_unit_type_owner_gate_follows_units_dat_flag_80000() {
        // `0x005c4dc0`.
        let mut w = test_world();
        let with_flag = w
            .data
            .units
            .iter()
            .position(|ty| ty.flags & 0x0008_0000 != 0)
            .unwrap() as u16;
        let without_flag = w
            .data
            .units
            .iter()
            .position(|ty| ty.flags & 0x0008_0000 == 0)
            .unwrap() as u16;

        assert_eq!(w.a08_unit_type_owner_query(without_flag, 1), Ok(true));
        assert_eq!(w.a08_unit_type_owner_query(with_flag, 1), Ok(false));

        w.spawn(1, uid::MARINE, 320, 320).unwrap();
        assert_eq!(w.a08_unit_type_owner_query(with_flag, 1), Err(()));
        // The scan keys on the owner byte only, not on the queried type.
        assert_eq!(w.a08_unit_type_owner_query(with_flag, 0), Ok(false));
    }

    #[test]
    fn a08_tile_rounding_matches_the_recovered_floor_form() {
        // `0x005c5f9a`/`0x005c5fcd` share this rounding.
        assert_eq!(a08_tile_index(0), 0);
        assert_eq!(a08_tile_index(31), 0);
        assert_eq!(a08_tile_index(32), 1);
        assert_eq!(a08_tile_index(-1), 0, "negative pixels add 31 first");
        assert_eq!(a08_tile_index(-31), 0);
        assert_eq!(a08_tile_index(-32), -1);
        assert_eq!(a08_tile_index(-33), -1);
        assert_eq!(a08_tile_index(-64), -2);
    }

    #[test]
    fn a08_placement_tile_row_and_column_helpers_use_the_units_dat_box() {
        // `0x005c5f9a` (columns) and `0x005c5fcd` (rows).
        let w = test_world();
        let offset_x = w.unit_placement_x(uid::MARINE);
        let offset_y = w.unit_placement_y(uid::MARINE);

        assert!(w.a08_placement_tile_row_matches(96, 96 + offset_y, uid::MARINE));
        assert!(w.a08_placement_tile_row_matches(96, 96 + offset_y + 31, uid::MARINE));
        assert!(!w.a08_placement_tile_row_matches(96, 96 + offset_y + 32, uid::MARINE));

        assert_eq!(
            w.a08_placement_columns_differ(96, 96 + offset_x, uid::MARINE),
            Err(())
        );
        assert_eq!(
            w.a08_placement_columns_differ(96, 96 + offset_x + 32, uid::MARINE),
            Ok(())
        );
    }

    #[test]
    fn a08_sprite_flag_high_bit_preserves_the_low_bits() {
        // `0x005c5060`.
        assert_eq!(a08_set_sprite_flag_high_bit(0x00, true), 0x80);
        assert_eq!(a08_set_sprite_flag_high_bit(0x7f, true), 0xff);
        assert_eq!(a08_set_sprite_flag_high_bit(0xff, false), 0x7f);
        assert_eq!(a08_set_sprite_flag_high_bit(0x92, false), 0x12);
    }

    #[test]
    fn a08_first_nonempty_group_pick_skips_empty_counts() {
        // `0x005c6329`.
        assert_eq!(
            a08_first_nonempty_group_pick(&[0, 0, 3, 0, 0, 0], 7),
            Some((2, 1))
        );
        assert_eq!(
            a08_first_nonempty_group_pick(&[0, 1, 4, 0, 0, 0], 3),
            Some((1, 0)),
            "a single entry ignores the RNG"
        );
        assert_eq!(
            a08_first_nonempty_group_pick(&[0, 0, 0, 0, 0, 2], 0),
            Some((5, 0))
        );
        assert_eq!(a08_first_nonempty_group_pick(&[0; 6], 9), None);
    }

    #[test]
    fn a08_cheapest_candidate_keeps_the_first_minimum() {
        // `0x005c65d0`.
        assert_eq!(a08_cheapest_candidate(&[9, 3, 3, 7]), Some(1));
        assert_eq!(a08_cheapest_candidate(&[5]), Some(0));
        assert_eq!(a08_cheapest_candidate(&[]), None);
    }

    #[test]
    fn a08_order_selector_matches_add_on_and_the_nuke_rows() {
        // `0x005c7331`.
        let mut w = test_world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        // AX == 0x22 (Place Add-On) matches regardless of the unit order.
        assert!(w.units[marine].a08_order_matches_addon_or_nuke(0x22));
        assert!(!w.units[marine].a08_order_matches_addon_or_nuke(0x99));
        w.issue(w.units[marine].id, Order::HoldPosition).unwrap();
        // Hold Position is row 0x6a, neither 0x22 nor 0x7b..=0x81.
        assert!(!w.units[marine].a08_order_matches_addon_or_nuke(0x99));

        // Row 34 (Place Add-On) is selected directly by AX.
        assert!(a08_order_code_matches_addon_or_nuke(0x22, 0));
        assert!(a08_order_code_matches_addon_or_nuke(0x99, 0x7b));
        assert!(a08_order_code_matches_addon_or_nuke(0x99, 0x81));
        assert!(!a08_order_code_matches_addon_or_nuke(0x99, 0x7a));
        assert!(!a08_order_code_matches_addon_or_nuke(0x99, 0x82));
    }

    #[test]
    fn a08_nuke_and_special_row_selector() {
        // `0x005c740a`.
        for row in [0x7c, 0x7d, 0x90, 0xa2] {
            assert!(a08_is_nuke_or_special_row(row), "row {row:#x}");
        }
        for row in [0x00, 0x7b, 0x7e, 0x8f, 0x91, 0xa1, 0xa3, 0xffff] {
            assert!(!a08_is_nuke_or_special_row(row), "row {row:#x}");
        }
    }
}
