#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent12.rs
// ===========================================================================
// Translation batch 12.  Only this file is edited by the batch.
//
// The routines reconstructed here are the retail unit-search helpers that
// live in `0x005a4000..=0x005a7fff`.  Retail keeps a temporary "search
// grid" of per-column and per-row unit lists (`CUnit+0x13c..+0x148` are the
// unit's tile bounds), plus a scratch mark array and a small stack of saved
// result cursors.  [`crate::sim::World`] already exposes the unit records,
// their `units.dat` boxes and the terrain dimensions, so every query below
// is answered directly from [`World::units`]; nothing here adds persistent
// state.  The routines that only shuffled the retail scratch buffers are
// translated as free helpers that operate on caller-owned slices/vectors.

/// Retail stores unit positions in 1/256-pixel units.
const A12_FP: i32 = 256;

impl World {
    /// `0x005a4a70` (4-byte entry) / `0x005a4a74` (body): target switch gate.
    ///
    /// The routine takes the prospective target and the player's current target
    /// and returns 1 when the current target may be replaced.  It is a no when
    /// both pointers are the same and when the new unit is not hostile.  When a
    /// current target exists, is alive and can still be attacked, the switch is
    /// allowed only if the new unit is visible while the old one is not; this is
    /// the "keep shooting what you can see" rule of the auto-acquire path.
    ///
    /// Visibility is read from the cached per-player `CUnit+0xe4` mask
    /// ([`SimUnit::visibility_mask`]); attackability mirrors the retail guards
    /// (no lockdown/stasis/maelstrom lock, no disabled capability bit, not
    /// loaded into a transport or Bunker).
    pub fn a12_may_switch_target(&self, owner: u8, current: Option<u32>, candidate: u32) -> bool {
        if current == Some(candidate) {
            return false;
        }
        let Some(candidate_idx) = self.index_of(candidate) else {
            return false;
        };
        let new = &self.units[candidate_idx];
        if !new.alive || !self.is_enemy(owner, new.owner) {
            return false;
        }
        if let Some(old_id) = current
            && let Some(old_idx) = self.index_of(old_id)
        {
            let old = &self.units[old_idx];
            if old.alive && self.is_enemy(owner, old.owner) && a12_attackable(old) {
                let bit = 1u32 << (owner & 31);
                return new.visibility_mask & bit != 0 && old.visibility_mask & bit == 0;
            }
        }
        true
    }
}

/// `0x005a4b90`: resizes one of the retail unit-search buffers to
/// `param + 1` entries.  The routine is the inlined `std::vector<CUnit*>`
/// `resize`: the shrink half erases the tail, the growth half appends
/// null pointers.  The translated helper performs the equivalent
/// [`Vec::resize`] with the same null (zero) fill value.
pub fn a12_resize_search_buffer(buffer: &mut Vec<u32>, depth: usize) {
    buffer.resize(depth + 1, 0);
}

/// `0x005a5000`: clears the search scratch array and resets the result
/// cursor.  Retail zero-fills the scratch mark array and writes zero to the
/// result count.  The caller owns the mark array in this model, so only the
/// fill is reproduced; the reset count is `0`.
pub fn a12_clear_search_marks(marks: &mut [u32]) {
    marks.fill(0);
}

impl World {
    /// `0x005a5210`: units in the tile ring around a unit's box.
    ///
    /// Starting from `index`'s tile bounds (`CUnit+0x13c..+0x148`), retail scans
    /// the grid columns to the right, then to the left, then the rows below and
    /// above.  Each of the four scans is grown by `dx`/`dy` tiles and collects a
    /// unit when its perpendicular span overlaps the grown band and it is not
    /// the caller itself.  The returned ids follow that scan order (right, left,
    /// down, up) with a unit emitted once even when two side scans both match a
    /// diagonal neighbour.
    pub fn a12_units_around_box(&self, index: usize, dx: i32, dy: i32) -> Vec<u32> {
        let Some(origin) = self.units.get(index) else {
            return Vec::new();
        };
        let (left, top, right, bottom) = a12_tile_bounds(self, origin);
        let (left_edge, right_edge) = (left - dx, right + dx);
        let (top_edge, bottom_edge) = (top - dy, bottom + dy);

        let mut result = Vec::new();
        let push = |unit: &SimUnit, result: &mut Vec<u32>| {
            if !result.contains(&unit.id) {
                result.push(unit.id);
            }
        };
        for unit in &self.units {
            if !unit.alive || unit.id == origin.id {
                continue;
            }
            let (l, t, _, b) = a12_tile_bounds(self, unit);
            // Columns to the right of the origin box.
            if l > right && l <= right_edge && t <= bottom_edge && b >= top_edge {
                push(unit, &mut result);
            }
        }
        for unit in &self.units {
            if !unit.alive || unit.id == origin.id {
                continue;
            }
            let (_, t, r, b) = a12_tile_bounds(self, unit);
            // Columns to the left of the origin box.
            if r < left && r >= left_edge && t <= bottom_edge && b >= top_edge {
                push(unit, &mut result);
            }
        }
        for unit in &self.units {
            if !unit.alive || unit.id == origin.id {
                continue;
            }
            let (l, t, r, _) = a12_tile_bounds(self, unit);
            // Rows below the origin box.
            if t > bottom && t <= bottom_edge && l <= right_edge && r >= left_edge {
                push(unit, &mut result);
            }
        }
        for unit in &self.units {
            if !unit.alive || unit.id == origin.id {
                continue;
            }
            let (l, _, r, b) = a12_tile_bounds(self, unit);
            // Rows above the origin box.
            if b < top && b >= top_edge && l <= right_edge && r >= left_edge {
                push(unit, &mut result);
            }
        }
        result
    }

    /// `0x005a55b2`: collects units while walking grid rows downward.
    ///
    /// The routine starts at `start_row` and steps towards smaller row indices
    /// while the row edge coordinate stays at or above `row_limit`.  Every unit
    /// in each visited row is kept when its tile box horizontally overlaps the
    /// inclusive-to-exclusive tile range `[min_x, max_x)`, which is the same
    /// overlap test the retail row lists use.  A unit that spans several
    /// visited rows would be reached once per row in retail; the translated
    /// helper keeps only its first occurrence.
    pub fn a12_units_in_row_band(
        &self,
        start_row: i32,
        row_limit: i32,
        min_x: i32,
        max_x: i32,
    ) -> Vec<u32> {
        if start_row < row_limit {
            return Vec::new();
        }
        let mut result = Vec::new();
        for row in (row_limit..=start_row).rev() {
            for unit in &self.units {
                if !unit.alive {
                    continue;
                }
                let (l, t, r, b) = a12_tile_bounds(self, unit);
                if t <= row && b >= row && r >= min_x && l < max_x && !result.contains(&unit.id) {
                    result.push(unit.id);
                }
            }
        }
        result
    }

    /// `0x005a574f`: units whose tile box contains the given tile.
    ///
    /// The retail routine walks the column's unit list, keeps the units whose
    /// column span covers `tile_x`, and verifies the row span against `tile_y`
    /// before appending them to the result buffer.  The translated query scans
    /// every live unit in spawn order.
    pub fn a12_units_at_tile(&self, tile_x: i32, tile_y: i32) -> Vec<u32> {
        self.units
            .iter()
            .filter(|unit| {
                if !unit.alive {
                    return false;
                }
                let (l, t, r, b) = a12_tile_bounds(self, unit);
                l <= tile_x && tile_x <= r && t <= tile_y && tile_y <= b
            })
            .map(|unit| unit.id)
            .collect()
    }
}

/// `0x005a5770`: pops one saved search cursor.
///
/// Retail decrements the saved-depth counter and restores the result cursor
/// from the depth stack.  The helper reports the popped cursor (`None` when
/// the stack is already empty) and updates the caller's depth, so no
/// persistent global is needed.
pub fn a12_pop_search_cursor(stack: &[u32], depth: &mut usize) -> Option<u32> {
    if *depth == 0 {
        return None;
    }
    *depth -= 1;
    Some(stack.get(*depth).copied().unwrap_or(0))
}

impl World {
    /// `0x005a5790`: removes matching units from a search result list.
    ///
    /// The retail routine walks the result list, asks the per-unit getter for
    /// its short record id and compacts the list when that id equals the
    /// selector passed in.  In this model the per-unit row is
    /// [`SimUnit::type_id`], so the helper filters that row out of a caller-owned
    /// result list while preserving the order of the survivors.  Unknown ids are
    /// left untouched, matching the retail pointer-list behaviour.
    pub fn a12_remove_units_of_type(&self, ids: &mut Vec<u32>, type_id: u16) {
        ids.retain(|id| self.unit(*id).is_none_or(|unit| unit.type_id != type_id));
    }

    /// `0x005a5990`: units intersecting a tile rectangle.
    ///
    /// Retail intersects the column list of every column in `left..=right` with
    /// the row list of every row in `top..=bottom`.  A unit is therefore kept
    /// when both its tile column span and its tile row span intersect the
    /// inclusive rectangle.  Iterating the unit array directly yields one entry
    /// per unit; results are ordered column-major, then by row, which is the
    /// order the column lists are walked.
    pub fn a12_units_in_tile_rect(&self, left: i32, top: i32, right: i32, bottom: i32) -> Vec<u32> {
        let mut hits: Vec<(i32, i32, usize, u32)> = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, unit)| unit.alive)
            .filter_map(|(slot, unit)| {
                let (l, t, r, b) = a12_tile_bounds(self, unit);
                (l <= right && r >= left && t <= bottom && b >= top)
                    .then_some((l, t, slot, unit.id))
            })
            .collect();
        hits.sort_by_key(|&(l, t, slot, _)| (l, t, slot));
        hits.into_iter().map(|(_, _, _, id)| id).collect()
    }

    /// `0x005a5fb0`: map-margin guard for a tile row and pixel column.
    ///
    /// The retail check divides the 16-bit `AX` pixel coordinate by 32 to get a
    /// tile column and aborts when either `y + 2` reaches the map height or
    /// `x / 32 + 2` reaches the map width.  The translated predicate returns
    /// true when both coordinates stay inside that two-tile margin, which is the
    /// success path of the original routine.
    pub fn a12_tile_margin_ok(&self, x_px: i32, y_tile: i32) -> bool {
        let ax = i32::from(x_px as i16);
        let tile_x = (((ax >> 31) & 0x1f) + ax) >> 5;
        y_tile + 2 < self.terrain.height as i32 && tile_x + 2 < self.terrain.width as i32
    }
}

/// Retail attackability guards in front of target acquisition (the
/// `CUnit+0xdc` capability bit, the lock status timers and the hidden-sprite
/// states).  A unit that fails any guard cannot be retained as a target.
fn a12_attackable(unit: &SimUnit) -> bool {
    unit.alive
        && !unit.flag_26
        && unit.in_transport.is_none()
        && !unit.in_bunker
        && unit.status.lockdown_ticks == 0
        && unit.status.stasis_ticks == 0
        && unit.status.maelstrom_ticks == 0
}

/// Retail per-unit tile bounds (`CUnit+0x13c`, `+0x140`, `+0x144`, `+0x148`)
/// rebuilt from the `units.dat` box around the fixed-point centre.
fn a12_tile_bounds(world: &World, unit: &SimUnit) -> (i32, i32, i32, i32) {
    let dims = &world.data.unit(unit.type_id).dimensions;
    let px = unit.x.div_euclid(A12_FP);
    let py = unit.y.div_euclid(A12_FP);
    (
        (px - i32::from(dims.left)) >> 5,
        (py - i32::from(dims.top)) >> 5,
        (px + i32::from(dims.right)) >> 5,
        (py + i32::from(dims.bottom)) >> 5,
    )
}

#[cfg(test)]
mod tests_agent12 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, owner: usize, type_id: u16, x: i32, y: i32) -> u32 {
        let slot = world.spawn(owner, type_id, x, y).expect("spawn");
        world.units[slot].id
    }

    fn slot_of(world: &World, id: u32) -> usize {
        world.index_of(id).expect("unit present")
    }

    #[test]
    fn switch_target_rejects_same_ally_and_dead_candidates() {
        let mut w = world();
        let viewer = spawn(&mut w, 0, uid::MARINE, 320, 320);
        let enemy = spawn(&mut w, 1, uid::MARINE, 640, 320);
        let friend = spawn(&mut w, 0, uid::MARINE, 960, 320);

        assert!(!w.a12_may_switch_target(0, Some(enemy), enemy));
        assert!(!w.a12_may_switch_target(0, Some(viewer), friend));
        assert!(!w.a12_may_switch_target(0, Some(viewer), 0xdead_beef));
        // Undiscovered candidate: no current target means acquiring is fine.
        assert!(w.a12_may_switch_target(0, Some(viewer), enemy));

        let enemy_idx = slot_of(&w, enemy);
        w.units[enemy_idx].alive = false;
        assert!(!w.a12_may_switch_target(0, Some(viewer), enemy));
    }

    #[test]
    fn switch_prefers_only_a_visible_replacement_for_a_hidden_target() {
        let mut w = world();
        let old = spawn(&mut w, 1, uid::MARINE, 640, 320);
        let new = spawn(&mut w, 1, uid::MARINE, 640, 640);
        let old_idx = slot_of(&w, old);
        let new_idx = slot_of(&w, new);

        let bit = 1u32;
        w.units[new_idx].visibility_mask = u32::MAX;
        w.units[old_idx].visibility_mask = u32::MAX;
        assert!(!w.a12_may_switch_target(0, Some(old), new));

        w.units[old_idx].visibility_mask &= !bit;
        assert!(w.a12_may_switch_target(0, Some(old), new));

        w.units[new_idx].visibility_mask &= !bit;
        assert!(!w.a12_may_switch_target(0, Some(old), new));

        // A locked old target cannot be retained, so the gate opens.
        w.units[old_idx].visibility_mask = u32::MAX;
        w.units[new_idx].visibility_mask = u32::MAX;
        w.units[old_idx].status.lockdown_ticks = 10;
        assert!(w.a12_may_switch_target(0, Some(old), new));
        w.units[old_idx].status.lockdown_ticks = 0;
        assert!(!w.a12_may_switch_target(0, Some(old), new));
    }

    #[test]
    fn search_buffer_resize_matches_vector_resize() {
        let mut buffer = vec![7, 8, 9];
        a12_resize_search_buffer(&mut buffer, 1);
        assert_eq!(buffer, vec![7, 8]);
        a12_resize_search_buffer(&mut buffer, 4);
        assert_eq!(buffer, vec![7, 8, 0, 0, 0]);
    }

    #[test]
    fn clearing_the_search_marks_zeros_the_scratch_array() {
        let mut marks = [1u32, 2, 3, 4];
        a12_clear_search_marks(&mut marks);
        assert_eq!(marks, [0; 4]);
    }

    #[test]
    fn around_box_finds_adjacent_units_and_skips_overlapping_ones() {
        let mut w = world();
        let self_id = spawn(&mut w, 0, uid::MARINE, 320, 320);
        let same = spawn(&mut w, 0, uid::MARINE, 320, 320);
        let right = spawn(&mut w, 1, uid::MARINE, 448, 320);
        let below = spawn(&mut w, 1, uid::MARINE, 320, 448);
        let far = spawn(&mut w, 1, uid::MARINE, 992, 992);
        let self_idx = slot_of(&w, self_id);

        let near = w.a12_units_around_box(self_idx, 4, 4);
        assert!(near.contains(&right), "right neighbour: {near:?}");
        assert!(near.contains(&below), "lower neighbour: {near:?}");
        assert!(!near.contains(&self_id), "origin must be excluded");
        assert!(!near.contains(&same), "overlapping unit is not in the ring");
        assert!(!near.contains(&far), "far unit must be excluded: {near:?}");

        let narrow = w.a12_units_around_box(self_idx, 2, 2);
        assert!(
            !narrow.contains(&right),
            "2-tile margin is too small: {narrow:?}"
        );
        assert!(
            !narrow.contains(&below),
            "2-tile margin is too small: {narrow:?}"
        );
    }

    #[test]
    fn row_band_walks_downward_and_tests_horizontal_overlap() {
        let mut w = world();
        let top = spawn(&mut w, 0, uid::MARINE, 320, 320);
        let bottom = spawn(&mut w, 1, uid::MARINE, 320, 448);
        let side = spawn(&mut w, 1, uid::MARINE, 700, 448);

        let (_, top_t, _, _) = a12_tile_bounds(&w, &w.units[slot_of(&w, top)]);
        let (_, _, _, bottom_b) = a12_tile_bounds(&w, &w.units[slot_of(&w, bottom)]);
        let band = w.a12_units_in_row_band(bottom_b, top_t, 0, 16);
        assert!(band.contains(&top), "top unit: {band:?}");
        assert!(band.contains(&bottom), "bottom unit: {band:?}");
        assert!(!band.contains(&side), "unit outside the x band: {band:?}");

        let empty = w.a12_units_in_row_band(top_t - 1, bottom_b + 1, 0, 16);
        assert!(empty.is_empty(), "inverted range must be empty");
    }

    #[test]
    fn units_at_tile_uses_the_units_dat_box() {
        let mut w = world();
        let here = spawn(&mut w, 0, uid::MARINE, 320, 320);
        let far = spawn(&mut w, 1, uid::MARINE, 992, 992);

        let at = w.a12_units_at_tile(10, 10);
        assert!(at.contains(&here), "unit at tile 10,10: {at:?}");
        assert!(!at.contains(&far));
        assert!(w.a12_units_at_tile(31, 31).iter().all(|id| *id != here));
        assert!(w.a12_units_at_tile(31, 31).contains(&far));
    }

    #[test]
    fn search_cursor_pop_reports_and_restores_saved_depth() {
        let stack = [11u32, 22, 33];
        let mut depth = 3;
        assert_eq!(a12_pop_search_cursor(&stack, &mut depth), Some(33));
        assert_eq!(depth, 2);
        assert_eq!(a12_pop_search_cursor(&stack, &mut depth), Some(22));
        assert_eq!(depth, 1);
        assert_eq!(a12_pop_search_cursor(&stack, &mut depth), Some(11));
        assert_eq!(depth, 0);
        assert_eq!(a12_pop_search_cursor(&stack, &mut depth), None);
        assert_eq!(depth, 0);
    }

    #[test]
    fn result_list_filter_removes_only_the_requested_type() {
        let mut w = world();
        let marine = spawn(&mut w, 0, uid::MARINE, 320, 320);
        let zealot = spawn(&mut w, 0, uid::ZEALOT, 640, 640);
        let mut ids = vec![marine, zealot, 0xdead_beef];
        w.a12_remove_units_of_type(&mut ids, uid::MARINE);
        assert_eq!(ids, vec![zealot, 0xdead_beef]);

        w.a12_remove_units_of_type(&mut ids, uid::ZEALOT);
        assert_eq!(ids, vec![0xdead_beef]);
    }

    #[test]
    fn tile_rect_query_returns_column_major_intersections() {
        let mut w = world();
        let inside = spawn(&mut w, 0, uid::MARINE, 320, 320);
        let lower = spawn(&mut w, 1, uid::MARINE, 320, 448);
        let outside = spawn(&mut w, 1, uid::MARINE, 992, 992);

        let hits = w.a12_units_in_tile_rect(9, 9, 11, 11);
        assert!(hits.contains(&inside), "inside rectangle: {hits:?}");
        assert!(!hits.contains(&lower), "row is outside the rectangle");
        assert!(!hits.contains(&outside), "far unit is outside");

        let both = w.a12_units_in_tile_rect(9, 9, 14, 14);
        assert!(both.contains(&inside));
        assert!(both.contains(&lower));
        assert!(!both.contains(&outside));
        assert_eq!(both.len(), 2);
    }

    #[test]
    fn tile_margin_guard_requires_both_coordinates_inside() {
        let w = world();
        assert!(w.a12_tile_margin_ok(0, 0));
        assert!(w.a12_tile_margin_ok(61 * 32, 61));
        assert!(!w.a12_tile_margin_ok(62 * 32, 0));
        assert!(!w.a12_tile_margin_ok(0, 62));
        assert!(!w.a12_tile_margin_ok(62 * 32, 62));
    }
}
