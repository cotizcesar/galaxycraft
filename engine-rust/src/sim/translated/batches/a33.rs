#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent33.rs
// ===========================================================================
// Translation batch 33.  Only this file is edited by the batch.
//
// The `decompiled_uncited` rows of `0x005d4000..=0x005d5fff` are a mixture of
// complete routines and labels that the decompiler split out of longer
// routines.  Several rows land in the middle of an instruction or of a
// jump-table arm, so their C bodies have no recoverable behaviour.  The
// methods below translate the rows whose observable effect is backed by the
// state already kept in [`SimUnit`]/[`World`]:
//
// * the cargo/reference array at `CUnit+0xb0` (the eight-slot list that
//   [`SimUnit::cargo`] models), its per-slot type lookups, resident scans,
//   and the `units.dat` space accounting that gates loading;
// * the transport/Bunker detach path (`CUnit+0x80` parent link and the
//   `CUnit+0xdc` bit 6/5 flags, mapped to
//   [`SimUnit::connected_unit`], [`SimUnit::in_bunker`] and
//   [`SimUnit::in_transport`]);
// * the command-button unit-id remap, the empty state-0 epilogue, the
//   target-resolution state selector and the position reply store;
// * the `units.dat` selection-box extents read by the status window.
//
// Retail bytes that have no Rust field are handled the same way earlier
// batches handle them: the per-type byte at `CUnit+0x4d` is passed in by the
// caller, and the order state byte at `CUnit+0x4e` is returned by the one
// arm that only writes it.  Presentation calls (sound cues, sprite box
// updates, window refreshes) are intentionally not reproduced.

impl SimUnit {
    /// `0x005d40e3`: remap the incoming command-button unit id before the
    /// selection panel refreshes.
    ///
    /// The arm rewrites `5 -> 0x1e` and `0x17 -> 0x19` and passes every
    /// other id through unchanged, then stores order state `3`.  This is the
    /// inverse of the remap used when the button is built, so a unit id
    /// written into the panel and read back out survives the round trip.
    ///
    /// Only the remap is modelled; the state-byte write and the refresh
    /// calls that follow it in the same arm have no unit-level counterpart.
    pub fn a33_remap_selection_unit_id(id: u16) -> u16 {
        match id {
            5 => 0x1e,
            0x17 => 0x19,
            other => other,
        }
    }

    /// `0x005d41b3`: the state-0 arm falls through to the shared routine
    /// epilogue when its first predicate is false.
    ///
    /// The instructions at the recorded address are the tail
    /// (`mov ecx, edi; pop; pop; jmp <epilogue>`) of a branch that has no
    /// remaining `CUnit` reads or writes.  The translation is an explicit,
    /// testable hook, matching the empty vtable slots translated in earlier
    /// batches.
    pub fn a33_state_zero_epilogue_hook(&self) {}

    /// `0x005d436c`: choose the next order state after the target-resolution
    /// arm.
    ///
    /// The arm first refreshes the command icon from the unit type and clears
    /// `CUnit+0xdc` bit 21.  It then tests the order target at `CUnit+0x5c`:
    /// a target keeps the arm in state `5`, while an empty target drops it
    /// back to state `1`.  [`SimUnit::target`] and the order-level target
    /// view together stand in for the retail slot, and the caller keeps the
    /// returned state byte.
    pub fn a33_target_arm_next_state(&self) -> u8 {
        if self.target.is_some() || self.order_unit_target().is_some() {
            5
        } else {
            1
        }
    }

    /// `0x005d4403`: store the position reply of the target arm and move to
    /// state `6`.
    ///
    /// This address is the second half of the reply store: the x half is
    /// written immediately before it and the y half at this instruction,
    /// both from the unit's position getters.  The same block plays cue
    /// `0xd` and, when `CUnit+0x4f` bit 0 is set, tail-calls the effect
    /// refresh.  The cue and refresh are presentation; the modelled effect is
    /// the position write, reported together with the state the caller must
    /// install.  `(x, y)` are the low 16-bit pixel coordinates retail stores
    /// in `CUnit+0x58`/`CUnit+0x5a`, which are this model's `x`/`y`.
    pub fn a33_store_reply_position(&mut self, x: i32, y: i32) -> u8 {
        self.x = x;
        self.y = y;
        6
    }

    /// `0x005d4bb0`: early gate of the direct-transport targeting routine.
    ///
    /// The routine answers three ways before the full interaction sweep:
    /// a unit whose `CUnit+0x4d` byte is `0x60` is accepted immediately;
    /// a dynamic air unit (natively flying or a lifted building) is
    /// rejected; and a `units.dat` subunit is rejected.  Everything else
    /// continues into the position and cargo checks, which depend on the
    /// clicked point and are not part of this arm.
    ///
    /// `byte_4d` is the unmodelled per-type byte supplied by the caller.
    /// `None` means "not decided here".
    pub fn a33_direct_transport_gate(&self, data: &GameData, byte_4d: u8) -> Option<bool> {
        if byte_4d == 0x60 {
            return Some(true);
        }
        let ty = data.unit(self.type_id);
        if self.lifted || ty.is_flyer() {
            return Some(false);
        }
        if ty.is_subunit() {
            return Some(false);
        }
        None
    }

    /// `0x005d4dc9`: the transport candidate must provide cargo space.
    ///
    /// The arm looks up the candidate's type and requires the `units.dat`
    /// space-provided byte to be nonzero before the selected unit is moved
    /// to state `1`.  Types without a cargo hold therefore fail the
    /// boarding check.
    pub fn a33_has_cargo_capacity(&self, data: &GameData) -> bool {
        data.unit(self.type_id).space_provided != 0
    }

    /// `0x005d5be0`: resolve cargo slot `slot` to a unit.
    ///
    /// Retail packs the 16-bit slot reference with two per-slot state bits
    /// from `CUnit+0x132` in the high half on Brood War builds, then resolves
    /// the reference to a unit record.  The state bits are not part of
    /// [`SimUnit::cargo`], so the translated accessor returns the resident
    /// unit id and `None` for an empty or out-of-range slot.  The model's
    /// cargo list is compact, so `slot` is the resident index rather than a
    /// fixed hardware slot number.
    pub fn a33_cargo_unit_slot(&self, slot: usize) -> Option<u32> {
        self.cargo.get(slot).copied().filter(|&id| id != 0)
    }

    /// `0x005d5af0`: first cargo slot that resolves to a unit.
    ///
    /// The routine walks the type's cargo slots in order and returns the
    /// first reference that resolves.  The compact model list preserves that
    /// order; a zero entry is an empty slot and is skipped.
    pub fn a33_first_cargo_unit(&self) -> Option<u32> {
        self.cargo.iter().copied().find(|&id| id != 0)
    }

    /// `0x005d5fd0`: selection-box half extents read for the status window.
    ///
    /// The routine loads the two extent words of the unit's `units.dat`
    /// dimensions entry (left at `+0`, top at `+2`) and forwards them to the
    /// sprite status-box setter.  The setter is presentation, so the
    /// translation returns the pair the routine reads.  Marine is `(8, 9)`.
    pub fn a33_status_box_extents(&self, data: &GameData) -> (i32, i32) {
        let dimensions = data.unit(self.type_id).dimensions;
        (i32::from(dimensions.left), i32::from(dimensions.top))
    }
}

impl World {
    /// `0x005d47d0`: first visible unit whose linked target is the receiver.
    ///
    /// The retail body walks the visible-unit list and compares each record's
    /// `CUnit+0xc0` link against the receiver, returning the first node that
    /// matches.  [`SimUnit::linked_target`] exposes the same link in the
    /// model; the walk is [`World::units`] order (creation order) with dead
    /// records skipped, mirroring their removal from the retail list.
    pub fn a33_first_unit_linked_to(&self, target_id: u32) -> Option<u32> {
        self.units
            .iter()
            .find(|unit| unit.alive && unit.linked_target() == Some(target_id))
            .map(|unit| unit.id)
    }

    /// `0x005d48f0`: detach a loaded unit from its Bunker/transport parent.
    ///
    /// The routine only runs when `CUnit+0xdc` bit 6 is set
    /// ([`SimUnit::in_bunker`]).  With a nonzero `direct` flag and no
    /// `CUnit+0x80` parent, it simply clears the Bunker and loaded bits.  In
    /// every other case it locates the unit's slot in the parent's reference
    /// array, clears that slot (and its `CUnit+0x132` state pair), clears the
    /// parent link, and clears both containment bits.
    ///
    /// The model keeps passengers in [`SimUnit::cargo`] and the parent in
    /// [`SimUnit::connected_unit`], so the same sweep removes the id from the
    /// parent's cargo.  Returns whether any link was cleared; `false` when
    /// the unit is not Bunkered or a parent was required but missing.
    pub fn a33_detach_from_parent(&mut self, unit_id: u32, direct: bool) -> bool {
        let Some(i) = self.index_of(unit_id) else {
            return false;
        };
        if !self.units[i].in_bunker {
            return false;
        }
        if direct && self.units[i].connected_unit.is_none() {
            self.units[i].in_bunker = false;
            self.units[i].in_transport = None;
            return true;
        }
        let Some(parent_id) = self.units[i].connected_unit else {
            return false;
        };
        if let Some(p) = self.index_of(parent_id) {
            self.units[p].cargo.retain(|&id| id != unit_id);
        }
        self.units[i].connected_unit = None;
        self.units[i].in_bunker = false;
        self.units[i].in_transport = None;
        true
    }

    /// `0x005d5770`: run a predicate over the parent's cargo residents.
    ///
    /// The routine walks the capacity-counted reference slots, resolves each
    /// slot, and calls the supplied callback until one returns nonzero,
    /// reporting `true`.  Empty slots and references that do not resolve are
    /// skipped.  The callback is supplied by the caller, exactly like the
    /// indirect `call [ebp+0x8]` in the retail body.
    pub fn a33_any_cargo_unit_satisfies<F>(&self, unit_id: u32, predicate: F) -> bool
    where
        F: FnMut(&SimUnit) -> bool,
    {
        let Some(carrier) = self.unit(unit_id) else {
            return false;
        };
        carrier
            .cargo
            .iter()
            .filter_map(|id| self.unit(*id))
            .any(predicate)
    }

    /// `0x005d5d10`: true when at least one cargo slot resolves to a unit.
    ///
    /// The body walks the type's cargo slots and returns `1` on the first
    /// successful resolution, otherwise `0`.  The compact model list plus a
    /// live-record check reproduces the same answer.
    pub fn a33_has_cargo_unit(&self, unit_id: u32) -> bool {
        let Some(carrier) = self.unit(unit_id) else {
            return false;
        };
        carrier.cargo.iter().any(|id| self.unit(*id).is_some())
    }

    /// `0x005d5d80`: cargo sweep for a worker carrying a Flag Beacon.
    ///
    /// Every resolved slot is tested with the worker-flag-target predicate
    /// (the `CUnit+0xc0` link must name a `0xd7` Flag row).  Returns `true`
    /// when any resident passes.
    pub fn a33_cargo_has_flag_worker(&self, unit_id: u32) -> bool {
        let Some(carrier) = self.unit(unit_id) else {
            return false;
        };
        carrier.cargo.iter().any(|id| {
            self.index_of(*id)
                .is_some_and(|slot| self.a04_worker_flag_target(slot))
        })
    }

    /// `0x005d5e00`: true while the carrier has cargo space left.
    ///
    /// The routine sums the `units.dat` space-required byte of every resolved
    /// resident and compares the total against the carrier's space-provided
    /// byte, returning `1` when the residents fit with room to spare.  A type
    /// with no cargo hold therefore reports `false`, and unresolved references
    /// contribute nothing, matching the retail accumulation loop.
    pub fn a33_cargo_has_room(&self, unit_id: u32) -> bool {
        let Some(carrier) = self.unit(unit_id) else {
            return false;
        };
        let capacity = i32::from(self.data.unit(carrier.type_id).space_provided);
        let used: i32 = carrier
            .cargo
            .iter()
            .filter_map(|id| self.unit(*id))
            .map(|resident| i32::from(self.data.unit(resident.type_id).space_required))
            .sum();
        used < capacity
    }

    /// `0x005d5ea0`: total space required by the carrier's residents.
    ///
    /// The body walks the reference slots, resolves each resident, and sums
    /// its `units.dat` space-required byte.  Unresolved slots add nothing.
    pub fn a33_cargo_used_space(&self, unit_id: u32) -> i32 {
        let Some(carrier) = self.unit(unit_id) else {
            return 0;
        };
        carrier
            .cargo
            .iter()
            .filter_map(|id| self.unit(*id))
            .map(|resident| i32::from(self.data.unit(resident.type_id).space_required))
            .sum()
    }
}

#[cfg(test)]
mod tests_agent33 {
    use super::*;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::Order;
    use std::sync::Arc;

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, owner: usize, type_id: u16) -> usize {
        world.spawn_px(owner, type_id, 640, 640).unwrap()
    }

    #[test]
    fn selection_remap_is_the_inverse_of_the_button_remap() {
        // 0x005d40e3: 5 -> 0x1e and 0x17 -> 0x19, everything else unchanged.
        assert_eq!(SimUnit::a33_remap_selection_unit_id(5), 0x1e);
        assert_eq!(SimUnit::a33_remap_selection_unit_id(0x17), 0x19);
        assert_eq!(SimUnit::a33_remap_selection_unit_id(0x19), 0x19);
        assert_eq!(SimUnit::a33_remap_selection_unit_id(0x1e), 0x1e);
        assert_eq!(SimUnit::a33_remap_selection_unit_id(uid::MARINE), 0);
    }

    #[test]
    fn state_zero_epilogue_touches_no_links() {
        // 0x005d41b3: the branch contains only the routine epilogue.
        let mut w = open_world();
        let target = spawn(&mut w, 0, uid::SCV);
        let i = spawn(&mut w, 0, uid::MARINE);
        w.units[i].target = Some(w.units[target].id);
        w.units[i].order = Order::AttackUnit {
            target: w.units[target].id,
        };
        let before_target = w.units[i].target;
        let before_order = w.units[i].order.clone();
        w.units[i].a33_state_zero_epilogue_hook();
        assert_eq!(w.units[i].target, before_target);
        assert_eq!(w.units[i].order, before_order);
    }

    #[test]
    fn target_arm_chooses_state_five_or_one() {
        // 0x005d436c: target present -> 5, no target -> 1.
        let mut w = open_world();
        let other = spawn(&mut w, 0, uid::SCV);
        let i = spawn(&mut w, 0, uid::MARINE);
        assert_eq!(w.units[i].a33_target_arm_next_state(), 1);
        w.units[i].target = Some(w.units[other].id);
        assert_eq!(w.units[i].a33_target_arm_next_state(), 5);
        // The order-level target view feeds the same decision.
        w.units[i].target = None;
        w.units[i].order = Order::Gather {
            resource: w.units[other].id,
        };
        assert_eq!(w.units[i].a33_target_arm_next_state(), 5);
    }

    #[test]
    fn reply_position_store_writes_pixels_and_state_six() {
        // 0x005d4403: the reply arm stores x/y and enters state 6.
        let mut w = open_world();
        let i = spawn(&mut w, 0, uid::MARINE);
        assert_eq!(w.units[i].a33_store_reply_position(0x123, 0x456), 6);
        assert_eq!(w.units[i].x, 0x123);
        assert_eq!(w.units[i].y, 0x456);
    }

    #[test]
    fn early_transport_gate_checks_special_byte_air_and_subunit() {
        // 0x005d4bb0: 0x4d == 0x60 wins, air and subunit are rejected, the
        // remaining units continue into the position checks.
        let mut w = open_world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        assert_eq!(
            w.units[marine].a33_direct_transport_gate(&w.data, 0x60),
            Some(true)
        );
        assert_eq!(
            w.units[marine].a33_direct_transport_gate(&w.data, 0x00),
            None
        );

        let wraith = spawn(&mut w, 0, uid::WRAITH);
        assert_eq!(
            w.units[wraith].a33_direct_transport_gate(&w.data, 0x00),
            Some(false)
        );
        w.units[marine].lifted = true;
        assert_eq!(
            w.units[marine].a33_direct_transport_gate(&w.data, 0x00),
            Some(false)
        );
        w.units[marine].lifted = false;

        let turret = spawn(&mut w, 0, uid::GOLIATH_TURRET);
        assert!(w.data.unit(uid::GOLIATH_TURRET).is_subunit());
        assert_eq!(
            w.units[turret].a33_direct_transport_gate(&w.data, 0x00),
            Some(false)
        );
    }

    #[test]
    fn cargo_capacity_gate_reads_space_provided() {
        // 0x005d4dc9: only types with a nonzero space-provided byte accept.
        let mut w = open_world();
        let bunker = spawn(&mut w, 0, uid::BUNKER);
        let marine = spawn(&mut w, 0, uid::MARINE);
        assert!(w.data.unit(uid::BUNKER).space_provided > 0);
        assert_eq!(w.data.unit(uid::MARINE).space_provided, 0);
        assert!(w.units[bunker].a33_has_cargo_capacity(&w.data));
        assert!(!w.units[marine].a33_has_cargo_capacity(&w.data));
    }

    #[test]
    fn cargo_slot_helpers_follow_the_resident_order() {
        // 0x005d5be0 and 0x005d5af0: slot lookup and first resident.
        let mut w = open_world();
        let bunker = spawn(&mut w, 0, uid::BUNKER);
        let marine = spawn(&mut w, 0, uid::MARINE);
        let scv = spawn(&mut w, 0, uid::SCV);
        let bunker_id = w.units[bunker].id;
        let marine_id = w.units[marine].id;
        let scv_id = w.units[scv].id;

        assert_eq!(w.units[bunker].a33_cargo_unit_slot(0), None);
        assert_eq!(w.units[bunker].a33_first_cargo_unit(), None);

        w.units[bunker].cargo.push(0);
        w.units[bunker].cargo.push(marine_id);
        assert_eq!(w.units[bunker].a33_cargo_unit_slot(0), None);
        assert_eq!(w.units[bunker].a33_cargo_unit_slot(1), Some(marine_id));
        assert_eq!(w.units[bunker].a33_cargo_unit_slot(2), None);
        assert_eq!(w.units[bunker].a33_first_cargo_unit(), Some(marine_id));

        w.units[bunker].cargo.push(scv_id);
        assert_eq!(w.units[bunker].a33_cargo_unit_slot(2), Some(scv_id));
        assert!(w.a33_has_cargo_unit(bunker_id));
    }

    #[test]
    fn cargo_scans_skip_missing_records() {
        // 0x005d5d10 and 0x005d5770: unresolved references do not match.
        let mut w = open_world();
        let bunker = spawn(&mut w, 0, uid::BUNKER);
        let marine = spawn(&mut w, 0, uid::MARINE);
        let bunker_id = w.units[bunker].id;
        let marine_id = w.units[marine].id;
        w.units[bunker].cargo.push(u32::MAX);
        assert!(!w.a33_has_cargo_unit(bunker_id));
        assert!(!w.a33_any_cargo_unit_satisfies(bunker_id, |_| true));

        w.units[bunker].cargo.push(marine_id);
        assert!(w.a33_has_cargo_unit(bunker_id));
        assert!(w.a33_any_cargo_unit_satisfies(bunker_id, |unit| { unit.type_id == uid::MARINE }));
        assert!(!w.a33_any_cargo_unit_satisfies(bunker_id, |unit| { unit.type_id == uid::SCV }));
        assert!(!w.a33_any_cargo_unit_satisfies(u32::MAX, |_| true));
    }

    #[test]
    fn cargo_space_accounting_uses_units_dat_sizes() {
        // 0x005d5e00 and 0x005d5ea0: sum space required, compare to provided.
        let mut w = open_world();
        let bunker = spawn(&mut w, 0, uid::BUNKER);
        let bunker_id = w.units[bunker].id;
        let capacity = i32::from(w.data.unit(uid::BUNKER).space_provided);
        let required = i32::from(w.data.unit(uid::MARINE).space_required);
        assert!(capacity > 0);
        assert!(required > 0 && required <= capacity);
        assert_eq!(w.a33_cargo_used_space(bunker_id), 0);
        assert!(w.a33_cargo_has_room(bunker_id));

        let mut used = 0i32;
        while used + required <= capacity {
            let marine = spawn(&mut w, 0, uid::MARINE);
            let marine_id = w.units[marine].id;
            w.units[bunker].cargo.push(marine_id);
            used += required;
        }
        assert_eq!(w.a33_cargo_used_space(bunker_id), used);
        assert_eq!(w.a33_cargo_has_room(bunker_id), used < capacity);
        assert_eq!(w.a33_cargo_used_space(u32::MAX), 0);
        assert!(!w.a33_cargo_has_room(u32::MAX));
    }

    #[test]
    fn cargo_flag_worker_scan_uses_the_worker_predicate() {
        // 0x005d5d80: a resident worker whose link names a 0xd7 Flag row.
        let mut w = open_world();
        let bunker = spawn(&mut w, 0, uid::BUNKER);
        let bunker_id = w.units[bunker].id;
        let flag = spawn(&mut w, 0, 0xd7);
        let scv = spawn(&mut w, 0, uid::SCV);
        let marine = spawn(&mut w, 0, uid::MARINE);
        let marine_id = w.units[marine].id;
        let scv_id = w.units[scv].id;
        w.units[scv].order = Order::Gather {
            resource: w.units[flag].id,
        };

        w.units[bunker].cargo.push(marine_id);
        assert!(!w.a33_cargo_has_flag_worker(bunker_id));
        w.units[bunker].cargo.push(scv_id);
        assert!(w.a33_cargo_has_flag_worker(bunker_id));
        assert!(!w.a33_cargo_has_flag_worker(u32::MAX));
    }

    #[test]
    fn detaching_removes_the_parent_cargo_slot() {
        // 0x005d48f0: parent path clears the slot and both containment bits.
        let mut w = open_world();
        let bunker = spawn(&mut w, 0, uid::BUNKER);
        let marine = spawn(&mut w, 0, uid::MARINE);
        let bunker_id = w.units[bunker].id;
        let marine_id = w.units[marine].id;
        w.units[bunker].cargo.push(marine_id);
        w.units[marine].in_bunker = true;
        w.units[marine].in_transport = Some(bunker_id);
        w.units[marine].connected_unit = Some(bunker_id);

        assert!(w.a33_detach_from_parent(marine_id, true));
        assert!(w.units[bunker].cargo.is_empty());
        assert_eq!(w.units[marine].connected_unit, None);
        assert!(!w.units[marine].in_bunker);
        assert_eq!(w.units[marine].in_transport, None);
        // Not Bunkered any more: the bit-6 gate rejects a second call.
        assert!(!w.a33_detach_from_parent(marine_id, true));

        // Nonzero flag with no parent only clears the containment bits.
        w.units[marine].in_bunker = true;
        w.units[marine].in_transport = Some(bunker_id);
        w.units[marine].connected_unit = None;
        assert!(w.a33_detach_from_parent(marine_id, true));
        assert!(!w.units[marine].in_bunker);
        assert_eq!(w.units[marine].in_transport, None);

        // The parent-required form reports a missing parent.
        w.units[marine].in_bunker = true;
        assert!(!w.a33_detach_from_parent(marine_id, false));
        assert!(w.units[marine].in_bunker);
        assert!(!w.a33_detach_from_parent(u32::MAX, true));
    }

    #[test]
    fn linked_target_scan_returns_the_first_live_match() {
        // 0x005d47d0: walk records and compare the linked target.
        let mut w = open_world();
        let receiver = spawn(&mut w, 0, uid::MARINE);
        let first = spawn(&mut w, 0, uid::SCV);
        let second = spawn(&mut w, 0, uid::SCV);
        let receiver_id = w.units[receiver].id;
        assert_eq!(w.a33_first_unit_linked_to(receiver_id), None);
        w.units[first].order = Order::Repair {
            target: receiver_id,
        };
        w.units[second].order = Order::Repair {
            target: receiver_id,
        };
        assert_eq!(
            w.a33_first_unit_linked_to(receiver_id),
            Some(w.units[first].id)
        );
        w.units[first].alive = false;
        assert_eq!(
            w.a33_first_unit_linked_to(receiver_id),
            Some(w.units[second].id)
        );
    }

    #[test]
    fn status_box_extents_read_the_dimensions_table() {
        // 0x005d5fd0: left and top words of the units.dat dimensions entry.
        let mut w = open_world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        let scv = spawn(&mut w, 0, uid::SCV);
        assert_eq!(w.units[marine].a33_status_box_extents(&w.data), (8, 9));
        assert_eq!(w.units[scv].a33_status_box_extents(&w.data), (11, 11));
        let dimensions = w.data.unit(uid::MARINE).dimensions;
        assert_eq!(
            w.units[marine].a33_status_box_extents(&w.data),
            (i32::from(dimensions.left), i32::from(dimensions.top))
        );
    }
}
