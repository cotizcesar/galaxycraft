#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent39.rs
// ===========================================================================
// Translation batch 39.  Only this file is edited by the batch.
//
// The `decompiled_uncited` rows in `0x005e0000..=0x005e1fff` fall into two
// groups.  The `0x005e00xx`/`0x005e04xx`/`0x005e08xx` rows are fragments of
// the Zerg hatchery's larva spawn-side scan (the four per-side scores that
// [`SimUnit::larva_spawn_side_values`] already stores); the `0x005e0bxx` rows
// are the scenario countdown-text arithmetic.  Everything else in the page is
// MSVC exception-unwinding funclets, tail-call thunks or jump-table targets
// that the decompiler split out with bad control flow (several decode into
// `halt_baddata`); those are recorded as skipped rather than guessed.
//
// Only record-level behaviour the existing [`World`]/[`SimUnit`] state can
// hold is translated here.  Retail's failure branches call a noreturn abort
// helper; they are reported as `false`/`None` so the caller can tell the
// accepted path.

/// Slot order of [`SimUnit::larva_spawn_side_values`]: `[left, top, right,
/// bottom]`.  The four 16-bit spawn scores the retail record keeps after
/// `CUnit+0xd0` are addressed in this order (`+0xd0` left, `+0xd2` top,
/// `+0xd4` right, `+0xd6` bottom).
const A39_SIDE_LEFT: usize = 0;
const A39_SIDE_RIGHT: usize = 2;
const A39_SIDE_BOTTOM: usize = 3;

/// A spawn-side score saturates at 100 and every accepted placement adds 25.
const A39_SIDE_SCORE_CAP: i32 = 100;
const A39_SIDE_SCORE_STEP: i32 = 25;

impl SimUnit {
    /// `0x005e0250`: thunk onto the record accessor that returns the unit type
    /// id.  The retail body is a single tail jump into the `CUnit+0x64`
    /// getter, so the recovered behaviour is just the type id of the record.
    pub fn a39_record_type_id(&self) -> u16 {
        self.type_id
    }

    /// `0x005e0280`: record-handle comparison.  Retail compares the two
    /// supplied integers and returns 1 only when they are identical, so the
    /// modelled equivalent is "does this handle name this record".
    pub fn a39_id_matches(&self, id: u32) -> bool {
        self.id == id
    }

    /// `0x005e02c8`: a leaf predicate whose whole body is `return 1`.  It
    /// accepts every candidate; the spawn-side scan therefore relies on the
    /// remaining tests to reject a side.
    pub fn a39_accepts_any(&self) -> bool {
        true
    }

    /// `0x005e047f`: raise the **bottom** spawn-side score (`CUnit+0xd6`) by
    /// 25, saturating at 100.  Retail first requires `index > bound` and
    /// aborts otherwise; that guard is reported as `false`.  Returns true when
    /// the score changed.
    pub fn a39_raise_bottom_side_score(&mut self, index: usize, bound: usize) -> bool {
        if index <= bound {
            return false;
        }
        Self::a39_bump_side_score(&mut self.larva_spawn_side_values, A39_SIDE_BOTTOM)
    }

    /// `0x005e04ad`: `index < bound` selects the **left** side score
    /// (`CUnit+0xd0`), otherwise the **right** side (`CUnit+0xd4`); the chosen
    /// score is raised by 25 up to 100.  Returns the side that changed, or
    /// `None` when it was already capped.
    pub fn a39_raise_side_score_for_index(&mut self, index: usize, bound: usize) -> Option<usize> {
        let side = if index < bound {
            A39_SIDE_LEFT
        } else {
            A39_SIDE_RIGHT
        };
        Self::a39_bump_side_score(&mut self.larva_spawn_side_values, side).then_some(side)
    }

    fn a39_bump_side_score(scores: &mut [i32; 4], side: usize) -> bool {
        let score = &mut scores[side];
        if *score >= A39_SIDE_SCORE_CAP {
            return false;
        }
        *score = (*score + A39_SIDE_SCORE_STEP).min(A39_SIDE_SCORE_CAP);
        true
    }

    /// `0x005e08e0`: the "keep the better candidate" rule of the spawn
    /// selection loop.  The retail body compares the **second** word of the
    /// candidate pair first (`<=`), then the **first** word (`<`), so an equal
    /// pair keeps the incumbent.  Modelled as the pure pair fold the scan
    /// performs.
    pub fn a39_keep_smaller_pair(
        &self,
        incumbent: (u32, u32),
        candidate: (u32, u32),
    ) -> (u32, u32) {
        if candidate.1 <= incumbent.1 && (candidate.1 < incumbent.1 || candidate.0 < incumbent.0) {
            candidate
        } else {
            incumbent
        }
    }

    /// `0x005e08fa`: the single-key variant of the same fold.  Retail replaces
    /// the incumbent when the candidate's first word is strictly smaller and
    /// copies both words across.
    pub fn a39_keep_lower_first(&self, incumbent: (u32, u32), candidate: (u32, u32)) -> (u32, u32) {
        if candidate.0 < incumbent.0 {
            candidate
        } else {
            incumbent
        }
    }

    /// `0x005e0be7`: one step of the scenario clock builder.  Given the
    /// countdown length in seconds (`esi` in retail) and the elapsed seconds,
    /// retail stores, in this order:
    ///
    /// 1. `limit - elapsed` — seconds still on the clock,
    /// 2. `(elapsed + 5) / 60` — elapsed minutes, rounded up on five seconds,
    /// 3. `elapsed % 60` — the elapsed second inside the minute,
    /// 4. `(limit / 60) - ((elapsed + 5) / 60)` — minutes still on the clock,
    /// 5. `(limit - elapsed) % 60` — the remaining second inside the minute.
    ///
    /// C truncation is reproduced with `i32` arithmetic (Rust divides and
    /// takes remainders the same way).
    pub fn a39_clock_parts(&self, limit_seconds: i32, elapsed_seconds: i32) -> [i32; 5] {
        let minutes_elapsed = (elapsed_seconds + 5) / 0x3c;
        [
            limit_seconds - elapsed_seconds,
            minutes_elapsed,
            elapsed_seconds % 0x3c,
            limit_seconds / 0x3c - minutes_elapsed,
            (limit_seconds - elapsed_seconds) % 0x3c,
        ]
    }

    /// `0x005e0b61`: the same clock builder with the countdown length fixed at
    /// `0xb4` (180 seconds, three minutes).  Retail writes the remaining
    /// seconds, the rounded elapsed minutes, the elapsed second, `3 -` the
    /// rounded minutes and `(0xb4 - elapsed) % 60`.
    pub fn a39_three_minute_clock(&self, elapsed_seconds: i32) -> [i32; 5] {
        self.a39_clock_parts(0xb4, elapsed_seconds)
    }

    /// `0x005e0a39`: the formatted-entry version of `0x005e0b61`.  Before
    /// recomputing the three-minute clock retail clears the destination text
    /// buffer (it zeroes the payload, writes the size 0 and restores the
    /// 16-byte small-string capacity), then runs the same arithmetic.  The
    /// buffer reset is modelled by clearing `text`; the values handed to the
    /// text formatter are returned.
    pub fn a39_refresh_three_minute_clock(
        &self,
        elapsed_seconds: i32,
        text: &mut String,
    ) -> [i32; 5] {
        text.clear();
        self.a39_three_minute_clock(elapsed_seconds)
    }
}

impl World {
    /// `0x005e134a`: the hit test that gates drawing the 460x35 dialog at
    /// `(0, 5)`.  Retail accepts when `y <= bottom` and either `y < bottom` or
    /// `x <= right`, i.e. the bottom edge is inclusive and the x check only
    /// applies on that edge.  The dialog command itself (command `0x1d` with
    /// the `(0, 5, 460, 35)` rectangle) is presentation and carries no
    /// simulation state, so only the recovered predicate is reproduced.
    pub fn a39_dialog_hit_test(&self, x: i32, y: i32, right: i32, bottom: i32) -> bool {
        y <= bottom && (y < bottom || x <= right)
    }
}

#[cfg(test)]
mod tests_agent39 {
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

    /// `0x005e0250`, `0x005e0280` and `0x005e02c8`: the record accessors
    /// report the unit's own type and handle and accept every candidate.
    #[test]
    fn record_accessors_read_their_own_record() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let u = &w.units[marine];
        assert_eq!(u.a39_record_type_id(), uid::MARINE);
        assert!(u.a39_id_matches(u.id));
        assert!(!u.a39_id_matches(u.id.wrapping_add(1)));
        assert!(u.a39_accepts_any());
    }

    /// `0x005e047f`: the bottom score gains 25 per accepted placement, stops
    /// at 100, and the `index > bound` guard rejects the other branch.
    #[test]
    fn bottom_side_score_ramps_by_25_to_the_cap() {
        let mut w = world();
        let hatch = w.spawn(0, uid::HATCHERY, 8, 8).unwrap();
        let unit = &mut w.units[hatch];
        unit.larva_spawn_side_values = [0, 0, 0, 90];

        assert!(unit.a39_raise_bottom_side_score(4, 3));
        assert_eq!(unit.larva_spawn_side_values[A39_SIDE_BOTTOM], 100);

        // Already at the cap: no change.
        assert!(!unit.a39_raise_bottom_side_score(4, 3));
        assert_eq!(unit.larva_spawn_side_values[A39_SIDE_BOTTOM], 100);

        // Guard: the index must be strictly greater than the bound.
        unit.larva_spawn_side_values[A39_SIDE_BOTTOM] = 10;
        assert!(!unit.a39_raise_bottom_side_score(3, 3));
        assert_eq!(unit.larva_spawn_side_values[A39_SIDE_BOTTOM], 10);

        // A step below the cap adds exactly 25.
        assert!(unit.a39_raise_bottom_side_score(5, 3));
        assert_eq!(unit.larva_spawn_side_values[A39_SIDE_BOTTOM], 35);
        assert_eq!(unit.larva_spawn_side_values[A39_SIDE_LEFT], 0);
    }

    /// `0x005e04ad`: the index/bound comparison picks the left score below the
    /// bound and the right score at or above it.
    #[test]
    fn index_selects_left_or_right_side_score() {
        let mut w = world();
        let hatch = w.spawn(0, uid::HATCHERY, 20, 20).unwrap();
        let unit = &mut w.units[hatch];
        unit.larva_spawn_side_values = [0, 0, 0, 0];

        assert_eq!(
            unit.a39_raise_side_score_for_index(0, 2),
            Some(A39_SIDE_LEFT)
        );
        assert_eq!(unit.larva_spawn_side_values[A39_SIDE_LEFT], 25);
        assert_eq!(unit.larva_spawn_side_values[A39_SIDE_RIGHT], 0);

        assert_eq!(
            unit.a39_raise_side_score_for_index(2, 2),
            Some(A39_SIDE_RIGHT)
        );
        assert_eq!(unit.larva_spawn_side_values[A39_SIDE_RIGHT], 25);
        assert_eq!(
            unit.a39_raise_side_score_for_index(9, 2),
            Some(A39_SIDE_RIGHT)
        );
        assert_eq!(unit.larva_spawn_side_values[A39_SIDE_RIGHT], 50);

        unit.larva_spawn_side_values[A39_SIDE_LEFT] = 100;
        assert_eq!(unit.a39_raise_side_score_for_index(1, 2), None);
        assert_eq!(unit.larva_spawn_side_values[A39_SIDE_LEFT], 100);
    }

    /// `0x005e08e0` and `0x005e08fa`: the two "keep the better candidate"
    /// folds used by the spawn-side scan.
    #[test]
    fn pair_folds_keep_the_smaller_candidate() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 12, 12).unwrap();
        let u = &w.units[marine];

        // Second word first, then the first word as tie-break.
        let best = (7, 40);
        assert_eq!(u.a39_keep_smaller_pair(best, (9, 39)), (9, 39));
        assert_eq!(u.a39_keep_smaller_pair(best, (5, 40)), (5, 40));
        assert_eq!(u.a39_keep_smaller_pair(best, (8, 40)), best);
        assert_eq!(u.a39_keep_smaller_pair(best, (1, 41)), best);

        // Single key: only a strictly smaller first word replaces.
        assert_eq!(u.a39_keep_lower_first(best, (6, 99)), (6, 99));
        assert_eq!(u.a39_keep_lower_first(best, (7, 1)), best);
        assert_eq!(u.a39_keep_lower_first(best, (8, 0)), best);
    }

    /// `0x005e0be7`: the five clock values for a caller-supplied countdown.
    #[test]
    fn clock_parts_split_elapsed_and_remaining() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 14, 14).unwrap();
        let u = &w.units[marine];

        assert_eq!(u.a39_clock_parts(180, 0), [180, 0, 0, 3, 0]);
        assert_eq!(u.a39_clock_parts(180, 59), [121, 1, 59, 2, 1]);
        // The five-second rounding is what pushes the minute over.
        assert_eq!(u.a39_clock_parts(180, 55), [125, 1, 55, 2, 5]);
        assert_eq!(u.a39_clock_parts(180, 180), [0, 3, 0, 0, 0]);
        // C remainder truncates toward zero, so an over-run past the limit
        // stays negative rather than wrapping into the previous minute.
        assert_eq!(u.a39_clock_parts(600, 601), [-1, 10, 1, 0, -1]);
    }

    /// `0x005e0b61`: the same builder with the three-minute limit hard-coded.
    #[test]
    fn three_minute_clock_matches_the_180_second_limit() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 16, 16).unwrap();
        let u = &w.units[marine];

        assert_eq!(u.a39_three_minute_clock(0), [180, 0, 0, 3, 0]);
        assert_eq!(u.a39_three_minute_clock(175), [5, 3, 55, 0, 5]);
        assert_eq!(u.a39_three_minute_clock(90), [90, 1, 30, 2, 30]);
        assert_eq!(u.a39_three_minute_clock(180), u.a39_clock_parts(180, 180));
    }

    /// `0x005e0a39`: the formatted entry clears the destination text before
    /// recomputing the three-minute values.
    #[test]
    fn refresh_clears_the_display_before_filling() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 18, 18).unwrap();
        let u = &w.units[marine];
        let mut text = String::from("stale countdown");

        let parts = u.a39_refresh_three_minute_clock(175, &mut text);
        assert!(text.is_empty());
        assert_eq!(parts, [5, 3, 55, 0, 5]);
        assert_eq!(parts, u.a39_three_minute_clock(175));
    }

    /// `0x005e134a`: the bottom edge of the dialog rectangle is inclusive and
    /// the x bound only applies on that edge.
    #[test]
    fn dialog_hit_test_includes_the_boundary() {
        let w = world();

        assert!(w.a39_dialog_hit_test(10, 5, 460, 35));
        assert!(w.a39_dialog_hit_test(500, 34, 460, 35));
        assert!(w.a39_dialog_hit_test(460, 35, 460, 35));
        assert!(!w.a39_dialog_hit_test(461, 35, 460, 35));
        assert!(!w.a39_dialog_hit_test(10, 36, 460, 35));
    }
}
