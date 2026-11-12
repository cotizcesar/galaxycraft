#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent38.rs
// ===========================================================================
// Translation batch 38.  Only this file is edited by the batch.
//
// Coverage: the `decompiled_uncited` rows of `0x005de000..=0x005dffff`.
// Most of that window is the client's runtime-integrity / obfuscation
// island: the recovered listings there land inside encrypted blocks,
// overlapping instructions, inline jump-table arms and libc++ exception
// cleanup thunks.  The rows translated here are the ones with recoverable
// behaviour: the Hatchery larva-placement arms and their four side probes,
// the placement fallback's half-width probe, plus the small
// placement-callback echo block.
//
// The placement arms write a freshly created Larva's unmodelled `+0x4e`
// order-state byte (the spawn side) and read the Hatchery's four
// `+0xd0..+0xd6` side scores.  The sim keeps the scores in
// [`SimUnit::larva_spawn_side_values`] as `[left, top, right, bottom]`; the
// state byte is returned to the caller instead of being added as a field.

impl World {
    /// Bounding box `(left, top, right, bottom)` of a unit in whole pixels,
    /// built from that unit type's `units.dat` extents.  This is the box the
    /// placement arms compare the newly created Larva against.
    fn a38_hatch_box(&self, hatch_id: u32) -> Option<(i32, i32, i32, i32)> {
        let hatch = self.unit(hatch_id)?;
        let dims = self.data.unit(hatch.type_id).dimensions;
        Some(dims.rect_at((hatch.x / fp(1), hatch.y / fp(1))))
    }

    /// One of the Hatchery's four side scores, in the retail order
    /// `[left, top, right, bottom]`.  The scores are 16-bit values in retail
    /// and the initial "no side" screen is 101 (`0x65`).
    fn a38_side_score(&self, hatch_id: u32, slot: usize) -> u16 {
        self.unit(hatch_id)
            .map(|u| u.larva_spawn_side_values[slot] as u16)
            .unwrap_or(0x65)
    }

    /// The larva-placement clearance test shared by the four side arms: the
    /// candidate must be on the map and on creep, and its neighbour `(dx,
    /// dy)` away must be on the map and walkable or crepuscular.  Retail
    /// probes the neighbour 22 px away along the side's outward axis.
    fn a38_spot_clear(&mut self, pos: (i32, i32), dx: i32, dy: i32) -> bool {
        let max_x = self.terrain.width as i32 * 32;
        let max_y = self.terrain.height as i32 * 32;
        let inside = |x: i32, y: i32| (0..max_x).contains(&x) && (0..max_y).contains(&y);
        if !inside(pos.0, pos.1) {
            return false;
        }
        let neighbour = (pos.0 + dx, pos.1 + dy);
        if !inside(neighbour.0, neighbour.1) {
            return false;
        }
        self.has_creep_px(pos.0, pos.1)
            && (self.terrain.walkable_px(neighbour.0, neighbour.1)
                || self.has_creep_px(neighbour.0, neighbour.1))
    }

    /// `0x005dfabc`: first arm of the spawn-side chain.  A newly created
    /// Larva whose X lies left of the Hatchery's box records order-state byte
    /// `0`; otherwise control falls through to the next arm.
    ///
    /// `None` means "no write" (the retail code jumps to the Y test), not an
    /// error; a missing unit also yields `None`.
    pub fn a38_hatch_state_left(&self, hatch_id: u32, x: i32) -> Option<u8> {
        let (left, _, _, _) = self.a38_hatch_box(hatch_id)?;
        (x < left).then_some(0)
    }

    /// `0x005dfad4`: second arm of the spawn-side chain.  A Larva whose Y
    /// lies above the Hatchery's box records order-state byte `1`; otherwise
    /// control falls through to the final arm.
    pub fn a38_hatch_state_top(&self, hatch_id: u32, y: i32) -> Option<u8> {
        let (_, top, _, _) = self.a38_hatch_box(hatch_id)?;
        (y < top).then_some(1)
    }

    /// `0x005dfaec`: final, unconditional arm of the spawn-side chain.  It
    /// records `2` when X is at or left of the Hatchery's right edge (the
    /// right side) and `3` when X is beyond it (the bottom side).
    pub fn a38_hatch_state_right_or_bottom(&self, hatch_id: u32, x: i32) -> Option<u8> {
        let (_, _, right, _) = self.a38_hatch_box(hatch_id)?;
        Some(if x <= right { 2 } else { 3 })
    }

    /// `0x005dfe56`: bottom-side larva spot.  When the bottom score
    /// (`larva_spawn_side_values[3]`) is below both the running best and the
    /// initial 101 screen, probe `(hatch_x, box_bottom + 10)` with the
    /// neighbour 22 px further down.  Returns the accepted point and the
    /// score it replaced the running best with.
    pub fn a38_larva_spot_bottom(
        &mut self,
        hatch_id: u32,
        best_score: u16,
    ) -> Option<(i32, i32, u16)> {
        let (_, _, _, bottom) = self.a38_hatch_box(hatch_id)?;
        let hx = self.unit(hatch_id)?.x / fp(1);
        let score = self.a38_side_score(hatch_id, 3);
        if score >= 0x65 || score >= best_score {
            return None;
        }
        let pos = (hx, bottom + 10);
        self.a38_spot_clear(pos, 0, 22)
            .then_some((pos.0, pos.1, score))
    }

    /// `0x005dfeb5`: left-side larva spot.  When the left score
    /// (`larva_spawn_side_values[0]`) beats the running best, probe
    /// `(box_left - 10, hatch_y)` with the neighbour 22 px further left.
    pub fn a38_larva_spot_left(
        &mut self,
        hatch_id: u32,
        best_score: u16,
    ) -> Option<(i32, i32, u16)> {
        let (left, _, _, _) = self.a38_hatch_box(hatch_id)?;
        let hy = self.unit(hatch_id)?.y / fp(1);
        let score = self.a38_side_score(hatch_id, 0);
        if score >= best_score {
            return None;
        }
        let pos = (left - 10, hy);
        self.a38_spot_clear(pos, -22, 0)
            .then_some((pos.0, pos.1, score))
    }

    /// `0x005dff07`: right-side larva spot.  When the right score
    /// (`larva_spawn_side_values[2]`) beats the running best, probe
    /// `(box_right + 10, hatch_y)` with the neighbour 22 px further right.
    pub fn a38_larva_spot_right(
        &mut self,
        hatch_id: u32,
        best_score: u16,
    ) -> Option<(i32, i32, u16)> {
        let (_, _, right, _) = self.a38_hatch_box(hatch_id)?;
        let hy = self.unit(hatch_id)?.y / fp(1);
        let score = self.a38_side_score(hatch_id, 2);
        if score >= best_score {
            return None;
        }
        let pos = (right + 10, hy);
        self.a38_spot_clear(pos, 22, 0)
            .then_some((pos.0, pos.1, score))
    }

    /// `0x005dff4c`: top-side larva spot and the final return of the four
    /// side arms.  When the top score (`larva_spawn_side_values[1]`) beats
    /// the running best, probe `(hatch_x, box_top - 10)` with the neighbour
    /// 22 px further up.  A rejected top probe returns `None`, which is the
    /// retail `best` flag being clear.
    pub fn a38_larva_spot_top(
        &mut self,
        hatch_id: u32,
        best_score: u16,
    ) -> Option<(i32, i32, u16)> {
        let (_, top, _, _) = self.a38_hatch_box(hatch_id)?;
        let hx = self.unit(hatch_id)?.x / fp(1);
        let score = self.a38_side_score(hatch_id, 1);
        if score >= best_score {
            return None;
        }
        let pos = (hx, top - 10);
        self.a38_spot_clear(pos, 0, -22)
            .then_some((pos.0, pos.1, score))
    }

    /// `0x005dffcc`: half the Hatchery's pixel width, `(left + right + 1) /
    /// 2`.  This is the offset the bottom fallback probes use when all four
    /// box sides are rejected.
    pub fn a38_hatch_half_width(&self, hatch_id: u32) -> Option<i32> {
        let hatch = self.unit(hatch_id)?;
        let (left, _, right, _) = self.data.unit(hatch.type_id).dimensions.extents();
        Some((left + right + 1) / 2)
    }

    /// `0x005dffe0`: bottom fallback larva spot at the Hatchery centre plus
    /// its half width.  The probe is the same bottom-side clearance test;
    /// the bottom score is recorded without another comparison because this
    /// arm only runs after every side probe failed.
    pub fn a38_larva_spot_bottom_right_half(&mut self, hatch_id: u32) -> Option<(i32, i32, u16)> {
        let (_, _, _, bottom) = self.a38_hatch_box(hatch_id)?;
        let hx = self.unit(hatch_id)?.x / fp(1);
        let half = self.a38_hatch_half_width(hatch_id)?;
        let score = self.a38_side_score(hatch_id, 3);
        let pos = (hx + half, bottom + 10);
        self.a38_spot_clear(pos, 0, 22)
            .then_some((pos.0, pos.1, score))
    }

    /// `0x005df950`: placement-callback echo.  The recovered block invokes
    /// the caller's placement callback with request code `-1` and the record
    /// id; when the callback reports success (a non-negative result) it
    /// copies the 28-byte placement record to the output before running the
    /// follow-up.  The follow-up itself is in the obfuscated tail and is not
    /// reconstructed, so only the successful record copy is returned.
    pub fn a38_placement_echo<F>(
        &self,
        request: u32,
        record: [u8; 28],
        callback: F,
    ) -> Option<[u8; 28]>
    where
        F: FnOnce(i32, u32) -> i32,
    {
        (callback(-1, request) >= 0).then_some(record)
    }
}

#[cfg(test)]
mod tests_agent38 {
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

    fn spawn(world: &mut World, type_id: u16) -> usize {
        world.spawn(0, type_id, 320, 320).expect("spawn")
    }

    fn hatch_box(w: &World, slot: usize) -> (i32, i32, i32, i32) {
        let u = &w.units[slot];
        w.data
            .unit(u.type_id)
            .dimensions
            .rect_at((u.x / fp(1), u.y / fp(1)))
    }

    #[test]
    fn hatch_state_chain_uses_the_parent_box() {
        let mut w = world();
        let hatch = spawn(&mut w, uid::HATCHERY);
        let (left, top, right, _) = hatch_box(&w, hatch);
        let id = w.units[hatch].id;

        assert_eq!(w.a38_hatch_state_left(id, left - 1), Some(0));
        assert_eq!(w.a38_hatch_state_left(id, left), None);
        assert_eq!(w.a38_hatch_state_top(id, top - 1), Some(1));
        assert_eq!(w.a38_hatch_state_top(id, top), None);
        assert_eq!(w.a38_hatch_state_right_or_bottom(id, right), Some(2));
        assert_eq!(w.a38_hatch_state_right_or_bottom(id, right - 50), Some(2));
        assert_eq!(w.a38_hatch_state_right_or_bottom(id, right + 1), Some(3));
    }

    #[test]
    fn hatch_state_chain_returns_none_for_an_unknown_unit() {
        let w = world();
        assert_eq!(w.a38_hatch_state_left(0xdead_beef, 0), None);
        assert_eq!(w.a38_hatch_state_top(0xdead_beef, 0), None);
        assert_eq!(w.a38_hatch_state_right_or_bottom(0xdead_beef, 0), None);
    }

    #[test]
    fn bottom_spot_is_taken_below_the_box() {
        let mut w = world();
        let hatch = spawn(&mut w, uid::HATCHERY);
        let (_, _, _, bottom) = hatch_box(&w, hatch);
        let hx = w.units[hatch].x / fp(1);
        let id = w.units[hatch].id;
        w.mature_creep();
        w.units[hatch].larva_spawn_side_values = [40, 40, 40, 7];

        assert_eq!(
            w.a38_larva_spot_bottom(id, 0x65),
            Some((hx, bottom + 10, 7))
        );
    }

    #[test]
    fn side_scores_gate_the_probes() {
        let mut w = world();
        let hatch = spawn(&mut w, uid::HATCHERY);
        let id = w.units[hatch].id;
        let (left, _, _, _) = hatch_box(&w, hatch);
        let hy = w.units[hatch].y / fp(1);
        w.mature_creep();

        w.units[hatch].larva_spawn_side_values = [101, 101, 101, 101];
        assert_eq!(w.a38_larva_spot_bottom(id, 0x65), None);
        assert_eq!(w.a38_larva_spot_left(id, 0x65), None);
        assert_eq!(w.a38_larva_spot_right(id, 0x65), None);
        assert_eq!(w.a38_larva_spot_top(id, 0x65), None);

        w.units[hatch].larva_spawn_side_values = [4, 3, 2, 1];
        assert_eq!(w.a38_larva_spot_left(id, 4), None);
        assert_eq!(w.a38_larva_spot_left(id, 5), Some((left - 10, hy, 4)));
    }

    #[test]
    fn left_right_and_top_spots_sit_outside_the_box() {
        let mut w = world();
        let hatch = spawn(&mut w, uid::HATCHERY);
        let (left, top, right, _) = hatch_box(&w, hatch);
        let hy = w.units[hatch].y / fp(1);
        let hx = w.units[hatch].x / fp(1);
        let id = w.units[hatch].id;
        w.mature_creep();
        w.units[hatch].larva_spawn_side_values = [1, 2, 3, 4];

        assert_eq!(w.a38_larva_spot_left(id, 0x65), Some((left - 10, hy, 1)));
        assert_eq!(w.a38_larva_spot_right(id, 0x65), Some((right + 10, hy, 3)));
        assert_eq!(w.a38_larva_spot_top(id, 0x65), Some((hx, top - 10, 2)));
    }

    #[test]
    fn a_non_creep_source_does_not_produce_a_spot() {
        let mut w = world();
        let marine = spawn(&mut w, uid::MARINE);
        let id = w.units[marine].id;
        assert_eq!(w.a38_larva_spot_bottom(id, 0x65), None);
        assert_eq!(w.a38_larva_spot_left(id, 0x65), None);
        assert_eq!(w.a38_larva_spot_right(id, 0x65), None);
        assert_eq!(w.a38_larva_spot_top(id, 0x65), None);
    }

    #[test]
    fn half_width_is_the_retail_span() {
        let mut w = world();
        let hatch = spawn(&mut w, uid::HATCHERY);
        let (left, _, right, _) = hatch_box(&w, hatch);
        let id = w.units[hatch].id;
        // The box halves are the stored extents; the recovered helper adds
        // them, adds one and halves the sum.
        let extents = w.data.unit(uid::HATCHERY).dimensions.extents();
        assert_eq!(
            w.a38_hatch_half_width(id),
            Some((extents.0 + extents.2 + 1) / 2)
        );
        assert!(right > left);
    }

    #[test]
    fn bottom_right_half_spot_uses_the_half_width() {
        let mut w = world();
        let hatch = spawn(&mut w, uid::HATCHERY);
        let (_, _, _, bottom) = hatch_box(&w, hatch);
        let hx = w.units[hatch].x / fp(1);
        let id = w.units[hatch].id;
        w.mature_creep();
        w.units[hatch].larva_spawn_side_values = [0, 0, 0, 9];
        let half = w.a38_hatch_half_width(id).unwrap();

        assert_eq!(
            w.a38_larva_spot_bottom_right_half(id),
            Some((hx + half, bottom + 10, 9))
        );
    }

    #[test]
    fn placement_echo_copies_only_on_success() {
        let w = world();
        let record = [0x5au8; 28];
        let mut seen = None;
        let echoed = w.a38_placement_echo(7, record, |code, request| {
            seen = Some((code, request));
            0
        });
        assert_eq!(seen, Some((-1, 7)));
        assert_eq!(echoed, Some(record));

        let rejected = w.a38_placement_echo(9, record, |_, _| -1);
        assert_eq!(rejected, None);
    }
}
