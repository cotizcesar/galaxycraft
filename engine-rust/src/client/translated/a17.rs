#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent17.rs
// ===========================================================================
// Translation batch 17.  Only this file is edited by the batch.
//
// Ten recovered routines from the `0x005c4000..=0x005c7fff` client/command
// helper region.  The area is dominated by GUI registration wrappers and
// debug-only accumulator code; only entries whose observable behaviour can
// be expressed with the existing [`crate::sim::World`] / [`crate::sim::SimUnit`]
// fields are translated here, and each item names the exact retail address
// it was recovered from.
//
// Helpers that agent09 already modelled as an alliance precondition
// (`FUN_005c8ad0`) are reported through an explicit `same_side` argument
// instead of being re-derived.  Unmodelled `CUnit` bytes are likewise
// supplied as caller arguments and called out in the item docs.

impl SimUnit {
    /// `0x005c78e0`: true when this unit's `CUnit+0x64` row is `0x7d` and the
    /// other unit's `CUnit+0x4c` owner byte is the same.  The recovered body
    /// reads the row through the type getter and compares the owner bytes
    /// directly, so no `units.dat` lookup is needed beyond the row constant.
    pub fn a17_type_7d_same_owner(&self, other: &SimUnit) -> bool {
        self.type_id == 0x7d && self.owner == other.owner
    }

    /// `0x005c7c20`: precondition for registering the `0x005c78e0`
    /// callback.  The recovered body requires the unit's row (`CUnit+0x64`)
    /// to be `0x7d` and `FUN_00695c10(0x7d, owner)` to return nonzero; only
    /// then does it insert the callback into the GUI list.  The GUI
    /// insertion is not modelled, so the data-table answer is supplied as
    /// `available`.
    pub fn a17_type_7d_available(&self, available: bool) -> bool {
        self.type_id == 0x7d && available
    }
}

impl World {
    /// `0x005c4e03`: debug guard that walks the unit list and fails when an
    /// entry's owner byte (`CUnit+0x4c`) equals `owner`.  The recovered body
    /// first compares the absolute coordinate delta with the map width
    /// (`DAT_00fa3728`) and only then scans the list; only the list predicate
    /// is observable in this model.
    ///
    /// Returns true when no entry of [`World::units`] belongs to `owner`
    /// (the retail success path).
    pub fn a17_owner_absent_from_units(&self, owner: u8) -> bool {
        !self.units.iter().any(|unit| unit.owner == owner)
    }

    /// `0x005c58b0`: walk the visible-unit list from its head
    /// (`DAT_00fa36a4`) through the `CUnit+0x04` next links and invoke a
    /// callback for each entry.  The retail pointer/functor object is
    /// represented by the Rust closure; list membership uses the same
    /// hidden predicate as the visible-list accessors (alive and not loaded
    /// into a transport or Bunker).
    pub fn a17_visit_visible_units<F: FnMut(u32)>(&self, mut visit: F) {
        for unit in self.units.iter().rev() {
            if unit.alive && unit.in_transport.is_none() && !unit.in_bunker {
                visit(unit.id);
            }
        }
    }

    /// `0x005c60b0`: worker/flag-target gate.  The recovered body applies
    /// every flag helper to its first unit argument: the worker row
    /// (`units.dat` flag `0x8`), `CUnit+0xdc` bit 0, the completed bit 4,
    /// bit 30 and the worker-only `CUnit+0xc0` linked target must all be in
    /// the idle-worker state.  It then rejects the case where the second
    /// argument is the owned `0xd7` Flag row, whose type comes from
    /// `CUnit+0x64`.
    ///
    /// `CUnit+0xdc` bits 0 and 30 are not separate [`SimUnit`] fields, so
    /// they are supplied by the caller.  Returns true on the retail success
    /// path.
    pub fn a17_worker_flag_target_gate(
        &self,
        worker: usize,
        other: usize,
        status_bit0: bool,
        status_bit30: bool,
    ) -> bool {
        let worker_unit = &self.units[worker];
        let other_unit = &self.units[other];
        self.data.unit(worker_unit.type_id).is_worker()
            && status_bit0
            && !worker_unit.completed
            && !status_bit30
            && worker_unit.linked_target().is_none()
            && !(other_unit.type_id == 0xd7 && other_unit.owner == worker_unit.owner)
    }

    /// `0x005c6210`: rectangle computation for a centred screen box.  The
    /// recovered body reads two 16-bit table extents, computes
    /// `-32 - extent / 2 + position` for each axis and then adds the extent
    /// plus `0x40` for the opposite corner.  Retail stores the four results
    /// as 16-bit words and hands them to the GUI list builder; only the
    /// arithmetic is translated here.
    ///
    /// `centre` is the unit position, `width`/`height` are the two extents
    /// read by the retail getters.
    pub fn a17_selection_rect(centre: (i32, i32), width: i32, height: i32) -> [i32; 4] {
        // The retail code uses an arithmetic shift, i.e. floor division.
        let x0 = centre.0 - 32 - (width >> 1);
        let y0 = centre.1 - 32 - (height >> 1);
        [x0, y0, x0 + width + 0x40, y0 + height + 0x40]
    }

    /// `0x005c72c0`: target gate for the fixed weapon row `0x39`.  The
    /// recovered body first calls the same-side helper `FUN_005c8ad0`
    /// (modelled as `same_side`), then requires `FUN_0059ca70(0x39, target)`
    /// to accept the first argument with the weapon belonging to the second.
    /// It rejects target heroes (`units.dat` flag `0x40`), and finally
    /// accepts when the force flag is set, when the target's whole-point HP
    /// plus shields is at least 100, when its row is a worker, when its row
    /// is one of `0x1e`, `5`, `0x22`, or when its current order byte lies in
    /// `0x7b..=0x81`.
    pub fn a17_ability_39_gate(
        &self,
        target: usize,
        caster: usize,
        same_side: bool,
        force_flag: bool,
    ) -> bool {
        if !same_side {
            return false;
        }
        let target_unit = &self.units[target];
        let caster_unit = &self.units[caster];
        let info = crate::data::WeaponTarget {
            unit: self.data.unit(target_unit.type_id),
            owner: target_unit.owner,
            flying: self.is_air_unit(target),
            untargetable: !target_unit.alive
                || target_unit.invincible
                || target_unit.status.stasis_ticks > 0,
        };
        if !self
            .data
            .weapon_target_ok(0x39, caster_unit.owner, Some(info))
        {
            return false;
        }
        if self.data.unit(target_unit.type_id).is_hero() {
            return false;
        }
        if force_flag || self.unit_current_total_points(target) >= 100 {
            return true;
        }
        if self.data.unit(target_unit.type_id).is_worker() {
            return true;
        }
        if matches!(target_unit.type_id, 0x1e | 5 | 0x22) {
            return true;
        }
        matches!(target_unit.order.order_id(), 0x7b..=0x81)
    }

    /// `0x005c7b60`: same-owner worker gate.  The recovered body requires
    /// equal `CUnit+0x4c` owners, the worker row (`units.dat` flag `0x8`),
    /// a clear Bunker bit (`CUnit+0xdc` bit 6), a non-null `CUnit+0x134`
    /// link, and a resolver call that returns the same unit.  Both
    /// `CUnit+0x134` and the resolver are unmodelled and are supplied by the
    /// caller as `has_link` / `resolves_to_self`.
    pub fn a17_worker_link_gate(
        &self,
        worker: usize,
        other: usize,
        has_link: bool,
        resolves_to_self: bool,
    ) -> bool {
        let worker_unit = &self.units[worker];
        worker_unit.owner == self.units[other].owner
            && self.data.unit(worker_unit.type_id).is_worker()
            && !worker_unit.in_bunker
            && has_link
            && resolves_to_self
    }

    /// `0x005c7a70`: conditional `0x400` registration wrapper.  A nonzero
    /// second argument short-circuits to zero; otherwise the recovered body
    /// calls the GUI registry and returns its pointer.  The registry call is
    /// represented by the `registered` argument.
    pub fn a17_register_400_when_unflagged(flag: bool, registered: Option<u32>) -> Option<u32> {
        if flag { None } else { registered }
    }

    /// `0x005c7f30`: registration class selection.  A nonzero second
    /// argument selects `0x120`; otherwise the per-player query
    /// `FUN_00635720(owner)` picks `0x800` when it is nonzero and `0x400`
    /// when it is zero.  The subsequent GUI list lookup and its
    /// `CUnit+0x5c` target unwrap are registry state and are not modelled.
    pub fn a17_registration_class(flag: bool, player_flag: bool) -> u16 {
        if flag {
            0x120
        } else if player_flag {
            0x800
        } else {
            0x400
        }
    }
}

#[cfg(test)]
mod tests_agent17 {
    use super::*;
    use crate::ids::unit as uid;
    use crate::sim::Order;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(crate::data::GameData::bundled().unwrap()),
            crate::map::Terrain::open(64, 64),
            2,
        )
    }

    /// `0x005c4e03`: the guard scans every unit entry, so a single unit of
    /// the queried owner makes the predicate false.
    #[test]
    fn owner_absence_follows_the_unit_list() {
        let mut w = world();
        assert!(w.a17_owner_absent_from_units(0));
        assert!(w.a17_owner_absent_from_units(1));

        w.spawn(0, uid::MARINE, 100, 100).unwrap();
        assert!(!w.a17_owner_absent_from_units(0));
        assert!(w.a17_owner_absent_from_units(1));

        w.spawn(1, uid::SCV, 200, 100).unwrap();
        assert!(!w.a17_owner_absent_from_units(0));
        assert!(!w.a17_owner_absent_from_units(1));
    }

    /// `0x005c58b0`: the visitor follows the visible list from head (newest
    /// live unit) to tail and skips loaded units.
    #[test]
    fn visible_visitor_orders_newest_first_and_skips_loaded_units() {
        let mut w = world();
        let first = w.spawn(0, uid::MARINE, 100, 100).unwrap();
        let transport = w.spawn(0, uid::DROPSHIP, 200, 100).unwrap();
        let second = w.spawn(0, uid::MARINE, 300, 100).unwrap();

        let mut seen = Vec::new();
        w.a17_visit_visible_units(|id| seen.push(id));
        assert_eq!(
            seen,
            vec![w.units[second].id, w.units[transport].id, w.units[first].id]
        );

        w.units[second].in_transport = Some(w.units[transport].id);
        let mut seen = Vec::new();
        w.a17_visit_visible_units(|id| seen.push(id));
        assert_eq!(seen, vec![w.units[transport].id, w.units[first].id]);

        w.units[first].in_bunker = true;
        let mut seen = Vec::new();
        w.a17_visit_visible_units(|id| seen.push(id));
        assert_eq!(seen, vec![w.units[transport].id]);
    }

    /// `0x005c60b0`: every guard must hold, and the owned Flag row is the
    /// one target that is rejected on the success arm.
    #[test]
    fn worker_flag_target_gate_requires_the_idle_worker_state() {
        let mut w = world();
        let scv = w.spawn(0, uid::SCV, 100, 100).unwrap();
        let marine = w.spawn(0, uid::MARINE, 200, 100).unwrap();
        let enemy_marine = w.spawn(1, uid::MARINE, 300, 100).unwrap();
        let flag = w.spawn(0, 0xd7, 400, 100).unwrap();
        let enemy_flag = w.spawn(1, 0xd7, 500, 100).unwrap();

        w.units[scv].completed = false;
        assert!(w.a17_worker_flag_target_gate(scv, marine, true, false));
        assert!(!w.a17_worker_flag_target_gate(scv, flag, true, false));
        assert!(w.a17_worker_flag_target_gate(scv, enemy_flag, true, false));
        assert!(!w.a17_worker_flag_target_gate(scv, marine, false, false));
        assert!(!w.a17_worker_flag_target_gate(scv, marine, true, true));

        w.units[scv].completed = true;
        assert!(!w.a17_worker_flag_target_gate(scv, marine, true, false));
        w.units[scv].completed = false;

        w.units[scv].order = Order::Gather {
            resource: w.units[marine].id,
        };
        assert!(!w.a17_worker_flag_target_gate(scv, marine, true, false));

        assert!(!w.a17_worker_flag_target_gate(marine, enemy_marine, true, false));
    }

    /// `0x005c6210`: the origin is centre minus 32 minus half the extent and
    /// the opposite corner adds the extent plus 0x40.
    #[test]
    fn selection_rect_matches_the_recovered_arithmetic() {
        assert_eq!(
            World::a17_selection_rect((1000, 2000), 16, 20),
            [960, 1958, 1040, 2042]
        );
        assert_eq!(World::a17_selection_rect((0, 0), 15, 1), [-39, -32, 40, 33]);
        assert_eq!(
            World::a17_selection_rect((-100, -200), 18, 18),
            [-141, -241, -59, -159]
        );
    }

    /// `0x005c72c0`: same-side and the weapon predicate are hard gates;
    /// heroes are rejected, while the force flag / health / worker / row
    /// exceptions accept.
    #[test]
    fn ability_39_gate_follows_all_branches() {
        let mut w = world();
        let target = w.spawn(0, uid::MARINE, 100, 100).unwrap();
        let caster = w.spawn(0, uid::MARINE, 200, 100).unwrap();

        assert!(!w.a17_ability_39_gate(target, caster, false, true));
        assert!(!w.a17_ability_39_gate(target, caster, true, false));
        assert!(w.a17_ability_39_gate(target, caster, true, true));

        w.units[target].hp = 100 << 8;
        assert!(w.a17_ability_39_gate(target, caster, true, false));

        w.units[target].hp = 40 << 8;
        let worker = w.spawn(0, uid::SCV, 300, 100).unwrap();
        assert!(w.a17_ability_39_gate(worker, caster, true, false));

        let hero_type = w
            .data
            .units
            .iter()
            .position(|unit| unit.is_hero())
            .expect("a hero row") as u16;
        let hero = w.spawn(0, hero_type, 400, 100).unwrap();
        assert!(!w.a17_ability_39_gate(hero, caster, true, true));

        let special = w.spawn(0, 0x1e, 500, 100).unwrap();
        assert!(!w.data.unit(0x1e).is_hero());
        assert!(w.a17_ability_39_gate(special, caster, true, false));
    }

    /// `0x005c78e0`: both the row check and the owner comparison are
    /// required.
    #[test]
    fn type_7d_gate_needs_the_row_and_the_owner() {
        let mut w = world();
        let a = w.spawn(0, 0x7d, 100, 100).unwrap();
        let b = w.spawn(0, 0x7d, 200, 100).unwrap();
        let c = w.spawn(1, 0x7d, 300, 100).unwrap();
        let marine = w.spawn(0, uid::MARINE, 400, 100).unwrap();

        assert!(w.units[a].a17_type_7d_same_owner(&w.units[b]));
        assert!(w.units[a].a17_type_7d_same_owner(&w.units[marine]));
        assert!(!w.units[a].a17_type_7d_same_owner(&w.units[c]));
        assert!(!w.units[marine].a17_type_7d_same_owner(&w.units[b]));
    }

    /// `0x005c7b60`: owner, worker row, Bunker bit and the two supplied
    /// link answers all participate.
    #[test]
    fn worker_link_gate_checks_owner_bunker_and_link() {
        let mut w = world();
        let scv = w.spawn(0, uid::SCV, 100, 100).unwrap();
        let marine = w.spawn(0, uid::MARINE, 200, 100).unwrap();
        let enemy = w.spawn(1, uid::MARINE, 300, 100).unwrap();

        assert!(!w.a17_worker_link_gate(scv, marine, false, true));
        assert!(!w.a17_worker_link_gate(scv, marine, true, false));
        assert!(w.a17_worker_link_gate(scv, marine, true, true));
        assert!(!w.a17_worker_link_gate(scv, enemy, true, true));
        assert!(!w.a17_worker_link_gate(marine, marine, true, true));

        w.units[scv].in_bunker = true;
        assert!(!w.a17_worker_link_gate(scv, marine, true, true));
    }

    /// `0x005c7f30`: the flag wins, otherwise the per-player query selects
    /// between the `0x400` and `0x800` classes.
    #[test]
    fn registration_class_follows_the_two_branch_flags() {
        assert_eq!(World::a17_registration_class(true, false), 0x120);
        assert_eq!(World::a17_registration_class(true, true), 0x120);
        assert_eq!(World::a17_registration_class(false, true), 0x800);
        assert_eq!(World::a17_registration_class(false, false), 0x400);
    }

    /// `0x005c7c20`: only the row `0x7d` and the data-table answer are
    /// visible in this model.
    #[test]
    fn type_7d_availability_gate_needs_the_row_and_the_table_answer() {
        let mut w = world();
        let unit = w.spawn(0, 0x7d, 100, 100).unwrap();
        let marine = w.spawn(0, uid::MARINE, 200, 100).unwrap();

        assert!(w.units[unit].a17_type_7d_available(true));
        assert!(!w.units[unit].a17_type_7d_available(false));
        assert!(!w.units[marine].a17_type_7d_available(true));
    }

    /// `0x005c7a70`: a set flag suppresses the registration result.
    #[test]
    fn registration_wrapper_short_circuits_on_the_flag() {
        assert_eq!(
            World::a17_register_400_when_unflagged(false, Some(7)),
            Some(7)
        );
        assert_eq!(World::a17_register_400_when_unflagged(true, Some(7)), None);
        assert_eq!(World::a17_register_400_when_unflagged(false, None), None);
    }
}
