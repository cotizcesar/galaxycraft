#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent07.rs
// ===========================================================================
// Translation batch 07.  Only this file is edited by the batch.
//
// The batch covers the `decompiled_uncited` rows in `0x005ba000..=0x005bffff`
// whose decompiled body is at most 100 bytes and whose observable state is
// already exposed by [`crate::sim::World`] / [`crate::sim::SimUnit`].  Most
// rows in that block are fragments of the retail unit-order switch (they read
// caller stack slots through `unaff_EBP`, or write the opaque order-descriptor
// pointer at `CUnit+0x100`); those are documented as skipped in the batch
// report instead of being guessed at.

/// `FUN_006c1ee0` half-extent used by the `0x005bac70` area callback: `0x30`
/// world pixels on each axis.
pub const A07_STASIS_HALF_EXTENT: i32 = 0x30;

/// `FUN_005bb640` stasis refresh timer applied by the `0x005bad20` callback.
pub const A07_STASIS_TICKS: i32 = crate::sim::STASIS_UPDATES;

impl World {
    /// `0x005ba160`: wrapper that calls `FUN_005ba190(1, ...)`.
    ///
    /// The kind-1 branch of `FUN_005ba190` calls `FUN_005b6c50(order_id, 0, 0)`
    /// and then `FUN_005b6cd0(order_id, 0, 0, 0xe4, 0)`, i.e. it issues an
    /// order row without a point.  The retail wrapper's third argument is the
    /// null-point sentinel (`FUN_005b6c50` aborts unless it is zero), so the
    /// Rust front door forwards `None`.  The preliminary race/queue checks and
    /// error sounds in `FUN_005ba190` are presentation checks with no state
    /// effect and are not modelled.
    pub fn a07_issue_order_without_point(
        &mut self,
        unit_id: u32,
        order_id: u8,
    ) -> Result<(), SimError> {
        self.issue_order_id(unit_id, order_id, None, None)
    }

    /// `0x005ba130`: wrapper that calls `FUN_005ba190(2, ...)`.
    ///
    /// The kind-2 branch of `FUN_005ba190` calls
    /// `FUN_005b6cd0(order_id, point(x, y), 0, flags, 0)`; the point is built
    /// from the wrapper's two coordinate words.  The `flags` word selects the
    /// retail order descriptor and has no Rust equivalent because
    /// [`World::issue_order_id`] derives the descriptor from `orders.dat`.
    /// The preliminary race/queue checks and error sounds are not modelled.
    pub fn a07_issue_order_at_point(
        &mut self,
        unit_id: u32,
        order_id: u8,
        pos: (i32, i32),
    ) -> Result<(), SimError> {
        self.issue_order_id(unit_id, order_id, None, Some(pos))
    }

    /// `0x005bac70`: apply the stasis area effect at `pos`.
    ///
    /// Retail creates the effect object with `FUN_00588410(0x169, x, y, 0)`,
    /// marks the caster's order (`FUN_0058b430(0x11)`) and then asks the
    /// `FUN_005c9620` box enumerator `FUN_006c1ee0(x, y, 0x30, callback, ctx)`
    /// to visit every unit in the box.  The callback is [`Self::a07_area_stasis_accepts`],
    /// which applies the recovered `0x83` stasis timer through `FUN_005bb640`.
    /// Object creation, the order marker and the callback's return value are
    /// outside the Rust model; the returned ids are the units that received
    /// the timer.
    pub fn a07_area_stasis(&mut self, caster: u32, pos: (i32, i32)) -> Vec<u32> {
        let mut hit = Vec::new();
        for id in self.units_in_rect(pos, A07_STASIS_HALF_EXTENT) {
            if !self.a07_area_stasis_accepts(caster, id) {
                continue;
            }
            if let Some(i) = self.index_of(id) {
                self.units[i].status.apply_stasis(A07_STASIS_TICKS);
                hit.push(id);
            }
        }
        hit
    }

    /// `0x005bad20`: the `FUN_006c1ee0` box callback used by
    /// [`Self::a07_area_stasis`].
    ///
    /// The decompiled body returns immediately when the visited unit is the
    /// caster, then rejects the unit when any of the three predicates is
    /// nonzero:
    ///
    /// * `FUN_005b3ac0` — `CUnit+0xdc` bit 26 ([`SimUnit::flag_26`]);
    /// * `FUN_005b5820` — `units.dat` building flag (`CUnit+0x64 < 0xe4` and
    ///   flag 0x1), exposed as [`Self::unit_is_building_flagged`];
    /// * `FUN_005b5840` — `CUnit+0xdc` bit 4, exposed as
    ///   [`SimUnit::is_completed_flag`].
    ///
    /// Survival is added because the enumerator behind `FUN_006c1ee0` only
    /// visits living units (`FUN_005c9620`, see [`Self::units_in_rect`]).
    pub fn a07_area_stasis_accepts(&self, caster: u32, target: u32) -> bool {
        let Some(unit) = self.unit(target) else {
            return false;
        };
        unit.alive
            && unit.id != caster
            && !unit.flag_26()
            && !self.unit_is_building_flagged(unit.type_id)
            && !unit.is_completed_flag()
    }

    /// `0x005bacff`: apply the `0x16` Maelstrom timer when the unit's
    /// `units.dat` type is organic.
    ///
    /// The body increments a module counter, calls `FUN_005b5c20` (organic
    /// flag 0x10000, [`crate::data::UnitType::is_organic`]) and on a nonzero
    /// result calls `FUN_005d2a80(0x16)`, the Maelstrom refresh.  The counter
    /// has no recovered reader and is omitted; the retail function returns
    /// zero unconditionally, while the returned `bool` here reports whether
    /// the organic branch applied the timer.
    pub fn a07_maelstrom_if_organic(&mut self, unit_id: u32) -> bool {
        let Some(i) = self.index_of(unit_id) else {
            return false;
        };
        if !self.data.unit(self.units[i].type_id).is_organic() {
            return false;
        }
        self.units[i]
            .status
            .apply_maelstrom(crate::sim::MAELSTROM_UPDATES);
        true
    }

    /// `0x005bb6a0`: clear the `CUnit+0x119` stasis timer, verify that the
    /// unit still carries another disabling flag, then clear the unsupported
    /// status bit 26.
    ///
    /// The body writes `CUnit+0x119 = 0`, calls `FUN_005b5aa0` (true when
    /// `CUnit+0xdc` bit 10, or the `+0x117`/`+0x119`/`+0x124` timers are
    /// nonzero), aborts to the runtime error handler when nothing remains,
    /// calls `FUN_005b66c0` (clear flag 26 unless the type carries `units.dat`
    /// flag 0x20000000) and finally refreshes a HUD plate
    /// (`FUN_005cbe10(0x16d, 0x16f)`).  The HUD refresh is not modelled; a
    /// `false` return is the recovered error path instead of terminating.
    pub fn a07_end_stasis(&mut self, unit_id: u32) -> bool {
        let Some(i) = self.index_of(unit_id) else {
            return false;
        };
        self.units[i].status.clear_stasis();
        let unit = &self.units[i];
        let still_disabled = unit.flag_10
            || unit.status.lockdown_ticks != 0
            || unit.status.stasis_ticks != 0
            || unit.status.maelstrom_ticks != 0;
        if !still_disabled {
            return false;
        }
        self.clear_unsupported_flag_26(i);
        true
    }

    /// `0x005bf9b0`: pick one element from a temporary list.
    ///
    /// The retail body takes the list length (`FUN_0075d720`), copies the
    /// list (`FUN_0075d6e0`), draws an inclusive index with
    /// `FUN_007415f0(0, len - 1, 0x32)` and returns `list[index]`
    /// (`FUN_0075d660`).  The inclusive draw is replaced by the crate's deterministic LCG ([`World::world_rand`]) so replays stay
    /// reproducible.  An empty list yields `None`; the retail routine would
    /// index `-1` and fault.
    pub fn a07_rand_pick<T: Copy>(&mut self, items: &[T]) -> Option<T> {
        if items.is_empty() {
            return None;
        }
        let index = self.world_rand() as usize % items.len();
        Some(items[index])
    }
}

impl SimUnit {
    /// `0x005bf210`: release the `CUnit+0x100` slot.
    ///
    /// When `FUN_005bb920` reports the slot nonzero, retail calls
    /// `FUN_0076bd90(slot)` (release the linked object) and writes zero to
    /// `CUnit+0x100`.  The release call is outside the Rust model, whose slot
    /// is the opaque [`SimUnit::field_100`] id; the observable write is
    /// reproduced here.  Returns whether a live slot was cleared.
    pub fn a07_release_field_100(&mut self) -> bool {
        if !self.field_100_nonzero() {
            return false;
        }
        self.set_field_100(0);
        true
    }

    /// `0x005bf8ca`: true when the unit's facing, snapped down to the start
    /// of its 64-step quadrant, is more than `0x40` angular steps away from
    /// `other_facing`.
    ///
    /// The body reads the facing byte at `CUnit+0x23` and snaps it with
    /// `0x20`/`0x60`/`0xa0`/`0xe0` boundaries to `0`/`0x40`/`0x80`/`0xc0`
    /// (back to `0` at and above `0xe0`).  `FUN_00569170` then returns the
    /// circular distance `min(d, 0x100 - d)`, and the predicate is
    /// `distance > 0x40`.
    pub fn a07_facing_quadrant_distant(&self, other_facing: u8) -> bool {
        let snapped = if self.facing < 0x20 {
            0
        } else if self.facing < 0x60 {
            0x40
        } else if self.facing < 0xa0 {
            0x80
        } else if self.facing < 0xe0 {
            0xc0
        } else {
            0
        };
        a07_facing_distance(snapped, other_facing) > 0x40
    }
}

/// `0x005bb721`: compare a 32-bit value's low byte against `expected`.
///
/// The decompiled body is `(unaff_EBX & 0xff) == in_EAX`; the function is a
/// leaf predicate with no unit access, so it is kept as a free helper.
pub fn a07_low_byte_matches(value: u32, expected: u32) -> bool {
    value & 0xff == expected
}

/// `FUN_00569170` as used by `0x005bf8ca`: circular 8-bit angular distance.
fn a07_facing_distance(a: u8, b: u8) -> u8 {
    let d = b.wrapping_sub(a);
    if d < 0x81 { d } else { 0u8.wrapping_sub(d) }
}

#[cfg(test)]
mod tests_agent07 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::{MAELSTROM_UPDATES, Order, STASIS_UPDATES};
    use std::sync::Arc;

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    #[test]
    fn wrapper_005ba130_issues_the_order_at_the_given_point() {
        let mut w = open_world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let id = w.units[marine].id;

        w.a07_issue_order_at_point(id, 6, (400, 400)).unwrap();

        assert_eq!(w.units[marine].order_row, 6);
        assert_eq!(w.units[marine].order, Order::Move { pos: (400, 400) });
    }

    #[test]
    fn wrapper_005ba160_issues_the_order_without_a_point() {
        let mut w = open_world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let id = w.units[marine].id;

        w.a07_issue_order_without_point(id, 1).unwrap();

        assert_eq!(w.units[marine].order_row, 1);
        assert!(matches!(
            w.units[marine].order,
            Order::HoldPosition | Order::Stop
        ));
    }

    #[test]
    fn area_stasis_005bac70_uses_the_callback_filters() {
        let mut w = open_world();
        let caster = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let caster_id = w.units[caster].id;

        let accepted = w.spawn(1, uid::ZERGLING, 330, 320).unwrap();
        w.units[accepted].completed = false;

        let completed = w.spawn(1, uid::ZERGLING, 340, 320).unwrap();
        assert!(w.units[completed].completed);

        let building = w.spawn(1, uid::HATCHERY, 350, 320).unwrap();
        w.units[building].completed = false;

        let flagged = w.spawn(1, uid::ZERGLING, 300, 320).unwrap();
        w.units[flagged].completed = false;
        w.units[flagged].flag_26 = true;

        let out_of_box = w.spawn(1, uid::ZERGLING, 500, 320).unwrap();
        w.units[out_of_box].completed = false;

        let hit = w.a07_area_stasis(caster_id, (330, 320));

        assert_eq!(hit, vec![w.units[accepted].id]);
        assert_eq!(w.units[accepted].status.stasis_ticks, STASIS_UPDATES);
        assert_eq!(w.units[completed].status.stasis_ticks, 0);
        assert_eq!(w.units[building].status.stasis_ticks, 0);
        assert_eq!(w.units[flagged].status.stasis_ticks, 0);
        assert_eq!(w.units[out_of_box].status.stasis_ticks, 0);
        assert_eq!(w.units[caster].status.stasis_ticks, 0);
    }

    #[test]
    fn callback_005bad20_rejects_the_caster_buildings_and_flag_26() {
        let mut w = open_world();
        let caster = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let caster_id = w.units[caster].id;
        let zergling = w.spawn(1, uid::ZERGLING, 330, 320).unwrap();
        let zergling_id = w.units[zergling].id;
        w.units[zergling].completed = false;

        assert!(!w.a07_area_stasis_accepts(caster_id, caster_id));
        assert!(w.a07_area_stasis_accepts(caster_id, zergling_id));

        w.units[zergling].flag_26 = true;
        assert!(!w.a07_area_stasis_accepts(caster_id, zergling_id));

        w.units[zergling].flag_26 = false;
        w.units[zergling].completed = true;
        assert!(!w.a07_area_stasis_accepts(caster_id, zergling_id));

        let building = w.spawn(1, uid::HATCHERY, 350, 320).unwrap();
        let building_id = w.units[building].id;
        w.units[building].completed = false;
        assert!(!w.a07_area_stasis_accepts(caster_id, building_id));

        assert!(!w.a07_area_stasis_accepts(caster_id, 0xdead_beef));
    }

    #[test]
    fn maelstrom_005bacff_only_applies_to_organic_types() {
        let mut w = open_world();
        let zergling = w.spawn(0, uid::ZERGLING, 320, 320).unwrap();
        let goliath = w.spawn(0, uid::GOLIATH, 340, 320).unwrap();

        assert!(w.a07_maelstrom_if_organic(w.units[zergling].id));
        assert_eq!(w.units[zergling].status.maelstrom_ticks, MAELSTROM_UPDATES);

        assert!(!w.a07_maelstrom_if_organic(w.units[goliath].id));
        assert_eq!(w.units[goliath].status.maelstrom_ticks, 0);

        assert!(!w.a07_maelstrom_if_organic(0xdead_beef));
    }

    #[test]
    fn end_stasis_005bb6a0_needs_another_disabling_flag() {
        let mut w = open_world();
        let zergling = w.spawn(0, uid::ZERGLING, 320, 320).unwrap();
        let id = w.units[zergling].id;
        w.units[zergling].flag_26 = true;
        w.units[zergling].status.apply_stasis(STASIS_UPDATES);

        // Only stasis is active: retail takes the runtime-error path and does
        // not reach FUN_005b66c0, so flag 26 must survive.
        assert!(!w.a07_end_stasis(id));
        assert_eq!(w.units[zergling].status.stasis_ticks, 0);
        assert!(w.units[zergling].flag_26);

        w.units[zergling].flag_26 = true;
        w.units[zergling].status.apply_stasis(STASIS_UPDATES);
        w.units[zergling].status.apply_lockdown(3);

        assert!(w.a07_end_stasis(id));
        assert_eq!(w.units[zergling].status.stasis_ticks, 0);
        assert_eq!(w.units[zergling].status.lockdown_ticks, 3);
        assert!(!w.units[zergling].flag_26);

        assert!(!w.a07_end_stasis(0xdead_beef));
    }

    #[test]
    fn low_byte_predicate_005bb721_matches_the_decompiled_compare() {
        assert!(a07_low_byte_matches(0x1234_56ff, 0xff));
        assert!(a07_low_byte_matches(0x00, 0));
        assert!(a07_low_byte_matches(0x100, 0));
        assert!(!a07_low_byte_matches(0x100, 0x100));
        assert!(!a07_low_byte_matches(0xfe, 0xff));
    }

    #[test]
    fn release_field_100_005bf210_clears_a_live_slot() {
        let mut w = open_world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let unit = &mut w.units[marine];

        assert!(!unit.a07_release_field_100());
        unit.set_field_100(0x1234);
        assert!(unit.a07_release_field_100());
        assert_eq!(unit.field_100(), 0);
        assert!(!unit.a07_release_field_100());
    }

    #[test]
    fn facing_quadrant_005bf8ca_uses_snapped_quadrant_starts() {
        let mut w = open_world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let unit = &mut w.units[marine];

        unit.facing = 0x10; // snaps to 0x00
        assert!(unit.a07_facing_quadrant_distant(0x50)); // 0x50 = 80 > 64
        assert!(!unit.a07_facing_quadrant_distant(0x30)); // 0x30 = 48
        assert!(!unit.a07_facing_quadrant_distant(0x40)); // 64 is not > 64

        unit.facing = 0x90; // snaps to 0x80
        assert!(unit.a07_facing_quadrant_distant(0x10)); // 112
        assert!(!unit.a07_facing_quadrant_distant(0xb0)); // 48

        unit.facing = 0xe5; // snaps back to 0x00
        assert!(!unit.a07_facing_quadrant_distant(0xff)); // 1
        assert!(unit.a07_facing_quadrant_distant(0x80)); // 128

        // Exhaustive: the helper always stays in 0..=0x80.
        for a in 0..=255u8 {
            for b in 0..=255u8 {
                assert!(a07_facing_distance(a, b) <= 0x80);
            }
        }
    }

    #[test]
    fn rand_pick_005bf9b0_is_in_bounds_and_deterministic() {
        let mut w = open_world();
        assert_eq!(w.a07_rand_pick::<u8>(&[]), None);
        assert_eq!(w.a07_rand_pick(&[7u8]), Some(7));

        let items = [10u8, 20, 30, 40];
        let mut seen = [false; 4];
        for _ in 0..64 {
            let picked = w.a07_rand_pick(&items).expect("non-empty list");
            let pos = items.iter().position(|v| *v == picked).unwrap();
            seen[pos] = true;
        }
        assert!(seen.iter().all(|v| *v), "every element should be reachable");

        let mut a = open_world();
        let mut b = open_world();
        for _ in 0..8 {
            assert_eq!(a.a07_rand_pick(&items), b.a07_rand_pick(&items));
        }
    }
}
