#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent23.rs
// ===========================================================================
// Translation batch 23.  Only this file is edited by the batch.
//
// Recovered routines in `0x005a4000..=0x005a5fff`, the retail `CUnit`
// attack/selection block that sits between the weapon-volley helpers and the
// unit-search grid.  The grid keeps one list per pixel column and per pixel
// row; each list is sorted by the unit's `units.dat` box edge, and the
// searches below are expressed directly against [`World::units`] and the
// existing `units.dat` dimensions.  Nothing here adds persistent state.
//
// The ledger rows in this window are a mixture of real routine entries and
// fragments Ghidra split at internal jump targets.  Rows that only contain a
// tail call into a helper outside the batch (`FUN_005a414b`, `FUN_005a4920`,
// `FUN_005a4ae0`, `FUN_005a5c50`, `FUN_005a5ce0`, `FUN_005a5e60`, `FUN_005a5f90`),
// mid-instruction data (`FUN_005a430c`, `FUN_005a492d`, `FUN_005a51c4`), the
// compiler-generated container loop at `FUN_005a4524` and the mid-routine
// entry `FUN_005a5a26` whose prefix is missing are not translated.  The chunks
// `FUN_005a4c5f`/`FUN_005a4e59` are continuation pieces of `0x005a4bc0`, and
// `FUN_005a5aff`/`FUN_005a5b92` continue `FUN_005a5a26`; only the represented
// routine entry is translated.

/// Retail stores unit positions in 1/256-pixel units.
const A23_FP: i32 = 256;

/// The retail sprite-hidden flag, mapped to the two states the sim models:
/// a unit loaded into a transport or a Bunker has no drawn sprite.
fn a23_sprite_hidden(unit: &SimUnit) -> bool {
    unit.in_transport.is_some() || unit.in_bunker
}

/// The `units.dat` bounding box of a unit in whole world pixels.
///
/// The four columns are the same extents [`crate::data::Box16::rect_at`]
/// exposes to the rest of the sim.
fn a23_pixel_box(world: &World, unit: &SimUnit) -> (i32, i32, i32, i32) {
    world
        .data
        .unit(unit.type_id)
        .dimensions
        .rect_at((unit.x.div_euclid(A23_FP), unit.y.div_euclid(A23_FP)))
}

/// Wrapped absolute distance between two byte direction values, in the
/// retail 0..=128 range (0 = same facing, 128 = opposite).
fn a23_facing_distance(a: u8, b: u8) -> i32 {
    let delta = (i32::from(b) - i32::from(a)).rem_euclid(0x100);
    delta.min(0x100 - delta)
}

/// True when the `weapons.dat` row for `weapon` is clear of its minimum
/// range at `distance`.
///
/// Retail lets a zero minimum-range row through without a distance test; the
/// caller's gate only runs for nonzero rows.
fn a23_weapon_clears_min_range(world: &World, weapon: u8, distance: i32) -> bool {
    match world.data.weapon(weapon) {
        Some(stats) => stats.min_range == 0 || distance >= stats.min_range as i32,
        None => false,
    }
}

impl World {
    /// `0x005a48b0`: horizontal fire-origin offset for one shot.
    ///
    /// The routine defaults a null target to the unit's order target
    /// (`CUnit+0x5c`, [`SimUnit::order_unit_target`]).  A unit that is not in
    /// the air always uses the plain origin.  For an air unit attacking
    /// another air unit whose facing is within `0x20` steps of its own, the
    /// shot starts at the plain origin as well; every other air shot is
    /// shifted right by the per-unit spread amount.
    ///
    /// The spread itself comes from the unmodelled iscript image geometry, so
    /// the caller passes the already-computed value.
    pub fn a23_attack_origin_x(&self, unit: &SimUnit, base_x: i32, spread: i32) -> i32 {
        let airborne = unit.lifted || self.data.unit(unit.type_id).is_flyer();
        if !airborne {
            return base_x;
        }
        let Some(target) = unit.order_unit_target().and_then(|id| self.unit(id)) else {
            return base_x;
        };
        let target_airborne = target.lifted || self.data.unit(target.type_id).is_flyer();
        if target_airborne && a23_facing_distance(unit.facing, target.facing) < 0x21 {
            return base_x;
        }
        base_x.saturating_add(spread)
    }

    /// `0x005a4835`: is this unit's `units.dat` air weapon clear of its
    /// minimum range at `distance`?
    ///
    /// The retail body reads the air-weapon column into a weapon id, rejects
    /// the `0x82` "no weapon" sentinel (anything above `0x81`), and then runs
    /// the weapon's minimum-range gate before starting the volley.  A
    /// zero-minimum row passes; otherwise the target must be at least
    /// `weapons.dat` minimum range away.
    pub fn a23_air_weapon_min_range_ok(&self, unit: &SimUnit, distance: i32) -> bool {
        let weapon = self.data.air_weapon_id(self.data.unit(unit.type_id));
        if weapon > 0x81 {
            return false;
        }
        a23_weapon_clears_min_range(self, weapon, distance)
    }

    /// `0x005a485f`: the ground-weapon twin of
    /// [`World::a23_air_weapon_min_range_ok`].
    ///
    /// The routine is the same gate over the `units.dat` ground-weapon column
    /// (`units.dat` ground weapon, `0x82` sentinel, `weapons.dat` minimum
    /// range).
    pub fn a23_ground_weapon_min_range_ok(&self, unit: &SimUnit, distance: i32) -> bool {
        let weapon = self.data.ground_weapon_id(self.data.unit(unit.type_id));
        if weapon > 0x81 {
            return false;
        }
        a23_weapon_clears_min_range(self, weapon, distance)
    }

    /// `0x005a4807`: the no-weapon override for an unburrowed Lurker.
    ///
    /// The recoverable branch writes the `0x82` sentinel when the unit is a
    /// Lurker (`0x67`) whose burrow state is clear; a burrowed Lurker and
    /// every other type keep the normal weapon selection.  The surrounding
    /// blocks in the decompiled row are the compiler's copies of the sibling
    /// weapon selectors and are not modelled.
    pub fn a23_lurker_weapon_override(&self, unit: &SimUnit, burrowed: bool) -> Option<u8> {
        (unit.type_id == uid::LURKER && !burrowed).then_some(0x82)
    }

    /// `0x005a49a8`: clamp a pixel pair to the open map rectangle.
    ///
    /// The routine clamps the horizontal coordinate to `0..width-1` and the
    /// vertical coordinate to `0..height-1` in the retail pixel space (the
    /// map dimensions are the pixel extents used by the unit-search grid),
    /// then returns the pair packed as two signed shorts.  The retail version
    /// first adds the caller's step to the vertical coordinate; callers that
    /// already stepped pass the stepped value here.
    pub fn a23_clamp_map_pixel_pair(&self, x: i32, y: i32) -> (i16, i16) {
        let max_x = (self.terrain.width as i32 * TILE_PX - 1).max(0);
        let max_y = (self.terrain.height as i32 * TILE_PX - 1).max(0);
        (x.clamp(0, max_x) as i16, y.clamp(0, max_y) as i16)
    }

    /// `0x005a49dd`: clamp a pixel pair after stepping the vertical coordinate
    /// back by one.
    ///
    /// The body overwrites the vertical coordinate with `arg - 1` when the
    /// subtraction does not overflow, then falls into the same map clamp as
    /// [`World::a23_clamp_map_pixel_pair`].  The overflow guard is reproduced
    /// with `i32::MIN`.
    pub fn a23_clamp_stepped_pixel_pair(&self, x: i32, y: i32) -> (i16, i16) {
        let stepped = if y == i32::MIN { y } else { y - 1 };
        self.a23_clamp_map_pixel_pair(x, stepped)
    }

    /// `0x005a4bc0`: units in the box around a unit's `units.dat` footprint.
    ///
    /// The routine saves the result cursor, then scans the grid columns to the
    /// right (`dx > 0`) or left (`dx < 0`) and the grid rows below/above,
    /// collecting every unit whose box overlaps the scan band but is not the
    /// caller and not another record sitting inside the caller's own
    /// footprint.  `dx`/`dy` are pixel distances; only their magnitude is the
    /// reach, while the sign selects the retail scan direction.  A hidden
    /// sprite (loaded into a transport or Bunker) clears both distances and
    /// therefore returns an empty list.
    ///
    /// The translated query returns one id per unit in column-major order.
    pub fn a23_units_around_unit(&self, index: usize, dx: i32, dy: i32) -> Vec<u32> {
        let Some(origin) = self.units.get(index) else {
            return Vec::new();
        };
        if a23_sprite_hidden(origin) {
            return Vec::new();
        }
        let (left, top, right, bottom) = a23_pixel_box(self, origin);
        let reach_x = dx.saturating_abs();
        let reach_y = dy.saturating_abs();
        let (edge_left, edge_top) = (left - reach_x, top - reach_y);
        let (edge_right, edge_bottom) = (right + reach_x, bottom + reach_y);

        let mut hits: Vec<(i32, i32, usize, u32)> = Vec::new();
        for (slot, unit) in self.units.iter().enumerate() {
            if !unit.alive || unit.id == origin.id {
                continue;
            }
            let (l, t, r, b) = a23_pixel_box(self, unit);
            if r < edge_left || l > edge_right || b < edge_top || t > edge_bottom {
                continue;
            }
            // The retail column scan starts beyond the caller's edge run, so
            // records that fit entirely in the caller's footprint are skipped.
            if l >= left && r <= right && t >= top && b <= bottom {
                continue;
            }
            // A positive delta walks columns to the right and rows below; a
            // negative one walks the opposite sides.  The column pass checks
            // the row span expanded by `dy`, the row pass the column span
            // expanded by `dx`.
            let column_side = if reach_x == 0 {
                false
            } else if dx > 0 {
                l > right && l <= right + reach_x
            } else {
                r < left && r >= left - reach_x
            };
            let column_ok = column_side && t <= bottom + reach_y && b >= top - reach_y;
            let row_side = if reach_y == 0 {
                false
            } else if dy > 0 {
                t > bottom && t <= bottom + reach_y
            } else {
                b < top && b >= top - reach_y
            };
            let row_ok = row_side && l <= right + reach_x && r >= left - reach_x;
            if !column_ok && !row_ok {
                continue;
            }
            hits.push((l, t, slot, unit.id));
        }
        hits.sort_by_key(|&(l, t, slot, _)| (l, t, slot));
        hits.into_iter().map(|(_, _, _, id)| id).collect()
    }

    /// `0x005a556e`: units in the row band below `start_row` whose horizontal
    /// span overlaps `[min_x, max_x)`.
    ///
    /// Retail walks the row lists from `start_row` down to `row_limit` and
    /// keeps a unit when its right edge reaches `min_x` and its left edge is
    /// before `max_x`.  The caller's own record is skipped.  A unit that
    /// spans several visited rows is reached once per row in retail; the
    /// translated query keeps its first occurrence.
    pub fn a23_units_in_row_band_excluding(
        &self,
        index: usize,
        start_row: i32,
        row_limit: i32,
        min_x: i32,
        max_x: i32,
    ) -> Vec<u32> {
        if start_row < row_limit {
            return Vec::new();
        }
        let skip = self.units.get(index).map(|unit| unit.id);
        let mut result = Vec::new();
        for unit in &self.units {
            if !unit.alive || Some(unit.id) == skip {
                continue;
            }
            let (l, t, r, b) = a23_pixel_box(self, unit);
            if t <= start_row && b >= row_limit && r >= min_x && l < max_x {
                result.push(unit.id);
            }
        }
        result
    }

    /// `0x005a5690`: first unit whose `units.dat` box contains a pixel point.
    ///
    /// The retail routine runs a small rectangle query around the point and
    /// then tests `left <= x < right` and `top <= y < bottom` on each
    /// candidate, so the right and bottom edges are exclusive.  Candidates
    /// are visited in column-major order: leftmost box first, then topmost,
    /// then spawn order.
    pub fn a23_first_unit_at_point(&self, x: i32, y: i32) -> Option<u32> {
        let mut hits: Vec<(i32, i32, usize, u32)> = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, unit)| unit.alive)
            .filter_map(|(slot, unit)| {
                let (l, t, r, b) = a23_pixel_box(self, unit);
                (l <= x && x < r && t <= y && y < b).then_some((l, t, slot, unit.id))
            })
            .collect();
        hits.sort_by_key(|&(l, t, slot, _)| (l, t, slot));
        hits.first().map(|&(_, _, _, id)| id)
    }

    /// `0x005a57f0`: units intersecting a half-open pixel rectangle.
    ///
    /// The routine lower-bounds the sorted column and row lists at
    /// `(left, top)`, walks the column list while the column edge stays before
    /// `right`, and keeps a unit when `top <= box.bottom` and
    /// `box.top < bottom`.  The mark array makes each unit appear once.  The
    /// translated query intersects each `units.dat` box (inclusive extents)
    /// with `[left, right) x [top, bottom)` and returns column-major order.
    pub fn a23_units_in_rect_half_open(
        &self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Vec<u32> {
        let mut hits: Vec<(i32, i32, usize, u32)> = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, unit)| unit.alive)
            .filter_map(|(slot, unit)| {
                let (l, t, r, b) = a23_pixel_box(self, unit);
                (l < right && r >= left && t < bottom && b >= top).then_some((l, t, slot, unit.id))
            })
            .collect();
        hits.sort_by_key(|&(l, t, slot, _)| (l, t, slot));
        hits.into_iter().map(|(_, _, _, id)| id).collect()
    }

    /// `0x005a5ee0`: is a sound id already queued on the unit?
    ///
    /// The routine looks the id up in the unit's queued-sound list and, when
    /// the unit has an attached subunit, retries the lookup on the subunit.
    /// [`SimUnit::speech_ticks`]/[`SimUnit::speech_id`] are the modelled
    /// single-slot queue; the subunit link itself is passed in because the
    /// sim does not store the retail `CUnit+0x70` pointer.
    pub fn a23_sound_queued(
        &self,
        unit: &SimUnit,
        sound_id: u16,
        subunit: Option<&SimUnit>,
    ) -> bool {
        let matches = |candidate: &SimUnit| {
            candidate.speech_ticks > 0 && u16::from(candidate.speech_id) == sound_id
        };
        matches(unit) || subunit.is_some_and(matches)
    }

    /// `0x005a5f61`: does the hide path have a live subunit to hide alongside
    /// the unit?
    ///
    /// The body inserts the unit into the sprite-hidden list and then repeats
    /// the visibility update for `CUnit+0x70` when that link exists and
    /// carries the `units.dat` subunit flag.  The list splice itself is not
    /// modelled; the translated predicate reports the recoverable gate.
    pub fn a23_hide_subunit_alongside(&self, subunit: Option<&SimUnit>) -> bool {
        subunit.is_some_and(|unit| unit.alive && self.data.unit(unit.type_id).is_subunit())
    }
}

#[cfg(test)]
mod tests_agent23 {
    use super::*;
    use crate::data::GameData;
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

    fn spawn(world: &mut World, owner: usize, type_id: u16, x: i32, y: i32) -> u32 {
        let slot = world.spawn(owner, type_id, x, y).expect("spawn");
        world.units[slot].id
    }

    fn slot_of(world: &World, id: u32) -> usize {
        world.index_of(id).expect("unit present")
    }

    #[test]
    fn attack_origin_only_offsets_unaligned_air_shots() {
        let mut w = world();
        let attacker = spawn(&mut w, 0, uid::WRAITH, 320, 320);
        let target = spawn(&mut w, 1, uid::WRAITH, 640, 320);
        assert!(w.data.unit(uid::WRAITH).is_flyer());
        w.issue(attacker, Order::AttackUnit { target }).unwrap();

        let attacker_slot = slot_of(&w, attacker);
        assert_eq!(w.a23_attack_origin_x(&w.units[attacker_slot], 100, 7), 100);

        let target_slot = slot_of(&w, target);
        w.units[target_slot].facing = 0x80;
        assert_eq!(w.a23_attack_origin_x(&w.units[attacker_slot], 100, 7), 107);

        // An air unit firing at a ground unit always takes the spread.
        let ground = spawn(&mut w, 1, uid::MARINE, 640, 640);
        w.units[target_slot].facing = 0;
        w.issue(attacker, Order::AttackUnit { target: ground })
            .unwrap();
        assert_eq!(w.a23_attack_origin_x(&w.units[attacker_slot], 100, 7), 107);

        // A ground attacker never offsets.
        let marine = spawn(&mut w, 0, uid::MARINE, 320, 640);
        w.issue(marine, Order::AttackUnit { target }).unwrap();
        let marine_slot = slot_of(&w, marine);
        assert_eq!(w.a23_attack_origin_x(&w.units[marine_slot], 100, 7), 100);
    }

    #[test]
    fn weapon_min_range_gates_use_the_units_dat_rows() {
        let mut w = world();
        let marine = spawn(&mut w, 0, uid::MARINE, 320, 320);
        let wraith = spawn(&mut w, 0, uid::WRAITH, 640, 320);
        let scv = spawn(&mut w, 0, uid::SCV, 960, 320);
        let siege = spawn(&mut w, 0, uid::SIEGE_MODE_TURRET, 1280, 320);
        let wraith_slot = slot_of(&w, wraith);
        let scv_slot = slot_of(&w, scv);
        let siege_slot = slot_of(&w, siege);
        let marine_slot = slot_of(&w, marine);

        assert!(w.data.unit(uid::SCV).air_weapon > 0x81);
        assert!(!w.a23_air_weapon_min_range_ok(&w.units[scv_slot], 0));

        let air_weapon = w.data.unit(uid::WRAITH).air_weapon;
        assert!(air_weapon <= 0x81);
        assert_eq!(w.data.weapon(air_weapon).unwrap().min_range, 0);
        assert!(w.a23_air_weapon_min_range_ok(&w.units[wraith_slot], 0));
        assert!(w.a23_air_weapon_min_range_ok(&w.units[wraith_slot], 1000));

        let siege_weapon = w.data.unit(uid::SIEGE_MODE_TURRET).ground_weapon;
        assert!(siege_weapon <= 0x81);
        let minimum = w.data.weapon(siege_weapon).unwrap().min_range as i32;
        assert!(minimum > 0);
        assert!(!w.a23_ground_weapon_min_range_ok(&w.units[siege_slot], minimum - 1));
        assert!(w.a23_ground_weapon_min_range_ok(&w.units[siege_slot], minimum));
        assert!(!w.a23_ground_weapon_min_range_ok(&w.units[siege_slot], -1));

        let ground_weapon = w.data.unit(uid::MARINE).ground_weapon;
        assert!(ground_weapon <= 0x81);
        assert_eq!(w.data.weapon(ground_weapon).unwrap().min_range, 0);
        assert!(w.a23_ground_weapon_min_range_ok(&w.units[marine_slot], 0));
    }

    #[test]
    fn unburrowed_lurker_reports_the_no_weapon_sentinel() {
        let mut w = world();
        let lurker = spawn(&mut w, 1, uid::LURKER, 320, 320);
        let hydralisk = spawn(&mut w, 1, uid::HYDRALISK, 640, 320);
        let lurker_slot = slot_of(&w, lurker);
        let hydralisk_slot = slot_of(&w, hydralisk);

        assert_eq!(
            w.a23_lurker_weapon_override(&w.units[lurker_slot], false),
            Some(0x82)
        );
        assert_eq!(
            w.a23_lurker_weapon_override(&w.units[lurker_slot], true),
            None
        );
        assert_eq!(
            w.a23_lurker_weapon_override(&w.units[hydralisk_slot], false),
            None
        );
    }

    #[test]
    fn pixel_pair_clamps_stay_on_the_open_map() {
        let w = world();
        assert_eq!(w.a23_clamp_map_pixel_pair(-3, 2100), (0, 2047));
        assert_eq!(w.a23_clamp_map_pixel_pair(10, 20), (10, 20));
        assert_eq!(w.a23_clamp_map_pixel_pair(5000, -5000), (2047, 0));

        assert_eq!(w.a23_clamp_stepped_pixel_pair(10, 0), (10, 0));
        assert_eq!(w.a23_clamp_stepped_pixel_pair(10, 20), (10, 19));
        assert_eq!(w.a23_clamp_stepped_pixel_pair(10, 2100), (10, 2047));
        assert_eq!(w.a23_clamp_stepped_pixel_pair(10, i32::MIN), (10, 0));
    }

    #[test]
    fn expanded_box_collects_neighbours_but_not_the_caller() {
        let mut w = world();
        let origin = spawn(&mut w, 0, uid::MARINE, 336, 336);
        let origin_slot = slot_of(&w, origin);
        let (left, _top, right, bottom) = a23_pixel_box(&w, &w.units[origin_slot]);

        let right_neighbour = spawn(&mut w, 1, uid::MARINE, right + 20, 336);
        let below_neighbour = spawn(&mut w, 1, uid::MARINE, 336, bottom + 20);
        let left_neighbour = spawn(&mut w, 1, uid::MARINE, left - 20, 336);
        let overlapping = spawn(&mut w, 1, uid::MARINE, 336, 336);
        let far = spawn(&mut w, 1, uid::MARINE, 3000, 3000);

        let (nl, _, _, _) = a23_pixel_box(&w, &w.units[slot_of(&w, right_neighbour)]);
        let (_, nt, _, _) = a23_pixel_box(&w, &w.units[slot_of(&w, below_neighbour)]);
        let (_, _, lr, _) = a23_pixel_box(&w, &w.units[slot_of(&w, left_neighbour)]);
        let dx = nl - right;
        let dy = nt - bottom;
        let left_reach = left - lr;
        assert!(dx > 0 && dy > 0 && left_reach > 0);

        // The row pass widens its column test by `dx`, so a below-and-left
        // unit still inside that widened span is collected.
        let diagonal = spawn(&mut w, 1, uid::MARINE, left - dx, bottom + 20);

        let found = w.a23_units_around_unit(origin_slot, dx, dy);
        assert!(found.contains(&right_neighbour), "{found:?}");
        assert!(found.contains(&below_neighbour), "{found:?}");
        assert!(
            found.contains(&diagonal),
            "row pass reaches diagonals: {found:?}"
        );
        assert!(
            !found.contains(&left_neighbour),
            "the positive column pass walks right only"
        );
        assert!(!found.contains(&origin), "caller must be excluded");
        assert!(
            !found.contains(&overlapping),
            "footprint overlap is skipped"
        );
        assert!(!found.contains(&far), "far unit must be excluded");

        let leftward = w.a23_units_around_unit(origin_slot, -left_reach, 0);
        assert!(leftward.contains(&left_neighbour), "{leftward:?}");

        let narrow = w.a23_units_around_unit(origin_slot, dx - 1, dy - 1);
        assert!(!narrow.contains(&right_neighbour));
        assert!(!narrow.contains(&below_neighbour));

        // A unit whose sprite is hidden has no search reach at all.
        w.units[origin_slot].in_transport = Some(u32::MAX);
        assert!(w.a23_units_around_unit(origin_slot, dx, dy).is_empty());
        assert!(w.a23_units_around_unit(usize::MAX, 4, 4).is_empty());
    }

    #[test]
    fn row_band_walk_skips_the_caller_and_tight_x_spans() {
        let mut w = world();
        let origin = spawn(&mut w, 0, uid::MARINE, 336, 336);
        let origin_slot = slot_of(&w, origin);
        let (_, top, _, bottom) = a23_pixel_box(&w, &w.units[origin_slot]);

        let row_mate = spawn(&mut w, 1, uid::MARINE, 400, 336);
        let below = spawn(&mut w, 1, uid::MARINE, 336, bottom + 40);
        let far = spawn(&mut w, 1, uid::MARINE, 3000, 336);

        let start = bottom + 80;
        let band = w.a23_units_in_row_band_excluding(origin_slot, start, top, 0, 2000);
        assert!(band.contains(&row_mate), "{band:?}");
        assert!(band.contains(&below), "{band:?}");
        assert!(!band.contains(&origin), "caller must be excluded");
        assert!(!band.contains(&far), "far unit must be excluded");

        let (mate_left, _, _, _) = a23_pixel_box(&w, &w.units[slot_of(&w, row_mate)]);
        let tight = w.a23_units_in_row_band_excluding(origin_slot, start, top, 0, mate_left);
        assert!(!tight.contains(&row_mate), "left edge is not before max_x");
        assert!(tight.contains(&below), "{tight:?}");

        assert!(
            w.a23_units_in_row_band_excluding(origin_slot, top - 1, bottom + 1, 0, 2000)
                .is_empty()
        );
    }

    #[test]
    fn first_unit_at_point_uses_half_open_boxes() {
        let mut w = world();
        let point = (400, 400);
        let first = spawn(&mut w, 0, uid::MARINE, point.0, point.1);
        let second = spawn(&mut w, 1, uid::MARINE, point.0, point.1);
        let first_slot = slot_of(&w, first);
        let (left, top, right, bottom) = a23_pixel_box(&w, &w.units[first_slot]);

        assert_eq!(w.a23_first_unit_at_point(point.0, point.1), Some(first));
        assert_eq!(w.a23_first_unit_at_point(left, top), Some(first));
        assert_eq!(w.a23_first_unit_at_point(right, point.1), None);
        assert_eq!(w.a23_first_unit_at_point(point.0, bottom), None);
        assert_eq!(w.a23_first_unit_at_point(3000, 3000), None);
        assert_ne!(w.a23_first_unit_at_point(point.0, point.1), Some(second));

        // A box that starts further left wins the column-major order.
        let mut w2 = world();
        let marine = spawn(&mut w2, 0, uid::MARINE, point.0, point.1);
        let depot = spawn(&mut w2, 1, uid::SUPPLY_DEPOT, point.0, point.1);
        let (marine_left, _, _, _) = a23_pixel_box(&w2, &w2.units[slot_of(&w2, marine)]);
        let (depot_left, _, _, _) = a23_pixel_box(&w2, &w2.units[slot_of(&w2, depot)]);
        let expected = if depot_left < marine_left {
            depot
        } else {
            marine
        };
        assert_eq!(w2.a23_first_unit_at_point(point.0, point.1), Some(expected));
    }

    #[test]
    fn half_open_rect_query_excludes_the_far_edge() {
        let mut w = world();
        let marine = spawn(&mut w, 0, uid::MARINE, 400, 400);
        let marine_slot = slot_of(&w, marine);
        let (left, top, right, bottom) = a23_pixel_box(&w, &w.units[marine_slot]);

        assert!(
            w.a23_units_in_rect_half_open(left, top, right + 1, bottom + 1)
                .contains(&marine)
        );
        assert!(
            !w.a23_units_in_rect_half_open(right + 1, top, right + 2, bottom + 1)
                .contains(&marine)
        );
        assert!(
            w.a23_units_in_rect_half_open(right, top, right + 1, bottom + 1)
                .contains(&marine),
            "the inclusive right edge still touches the half-open span"
        );
        assert!(
            w.a23_units_in_rect_half_open(left, top, left, bottom + 1)
                .is_empty(),
            "an empty width matches nothing"
        );
        w.units[marine_slot].alive = false;
        assert!(
            w.a23_units_in_rect_half_open(left, top, right + 1, bottom + 1)
                .is_empty()
        );
    }

    #[test]
    fn queued_sound_lookup_falls_back_to_a_subunit() {
        let mut w = world();
        let unit = spawn(&mut w, 0, uid::MARINE, 320, 320);
        let subunit = spawn(&mut w, 0, uid::INTERCEPTOR, 352, 320);
        let unit_slot = slot_of(&w, unit);
        let subunit_slot = slot_of(&w, subunit);

        w.units[unit_slot].say(5, 7);
        w.units[subunit_slot].say(5, 9);

        assert!(w.a23_sound_queued(&w.units[unit_slot], 7, None));
        assert!(!w.a23_sound_queued(&w.units[unit_slot], 8, None));
        assert!(!w.a23_sound_queued(&w.units[unit_slot], 9, None));
        assert!(w.a23_sound_queued(&w.units[unit_slot], 9, Some(&w.units[subunit_slot])));

        w.units[subunit_slot].speech_ticks = 0;
        assert!(!w.a23_sound_queued(&w.units[unit_slot], 9, Some(&w.units[subunit_slot])));
    }

    #[test]
    fn hide_path_extends_to_a_live_subunit_only() {
        let mut w = world();
        let marine = spawn(&mut w, 0, uid::MARINE, 320, 320);
        let turret = spawn(&mut w, 0, uid::GOLIATH_TURRET, 352, 320);
        let marine_slot = slot_of(&w, marine);
        let turret_slot = slot_of(&w, turret);

        assert!(w.data.unit(uid::GOLIATH_TURRET).is_subunit());
        assert!(!w.data.unit(uid::MARINE).is_subunit());

        assert!(!w.a23_hide_subunit_alongside(None));
        assert!(w.a23_hide_subunit_alongside(Some(&w.units[turret_slot])));
        assert!(!w.a23_hide_subunit_alongside(Some(&w.units[marine_slot])));

        w.units[turret_slot].alive = false;
        assert!(!w.a23_hide_subunit_alongside(Some(&w.units[turret_slot])));
    }
}
