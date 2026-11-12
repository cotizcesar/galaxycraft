#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent14.rs
// ===========================================================================
// Translation batch 14.  Only this file is edited by the batch.
//
// Ten `decompiled_uncited` routines from the `0x005b4000..=0x005b7fff`
// `CUnit` helper cluster.  Every method names the retail address it was
// recovered from, and the `a14_` prefix keeps this batch clear of the other
// concurrent batches.

impl SimUnit {
    /// `0x005b66a0`: write the raw `CUnit+0x70` attached-unit slot.
    ///
    /// Retail stores a `CUnit*` and clears the link with zero.  The Rust
    /// model keeps the same attachment as [`SimUnit::addon`], so a nonzero
    /// argument installs that id and zero clears it.  The recovered body is
    /// a single store and does not touch the other side of the link.
    pub fn a14_set_addon_link(&mut self, addon_id: u32) {
        self.addon = (addon_id != 0).then_some(addon_id);
    }

    /// `0x005b6b60`: voice-line request for the unit's owner.
    ///
    /// The recovered body reads the owner byte (`CUnit+0x4c`) and indexes a
    /// fixed per-line sound table, then hands both to the audio mixer.  This
    /// engine has no mixer, so the request is returned as `(owner, line)` for
    /// the caller to resolve against the table.
    pub fn a14_voice_line(&self, line: u8) -> (u8, u8) {
        (self.owner, line)
    }
}

impl World {
    /// `0x005b4bb0`: next record in the owner's unit list (`CUnit+0x6c`).
    ///
    /// New units are linked at the head of their owner's list, so the next
    /// pointer walks toward older units.  The unit vector's append order
    /// stands in for the list; scanning backwards from the unit finds the
    /// same neighbour.  Dead units are skipped because the death cleanup
    /// unlinks them.
    pub fn a14_next_unit_of_owner(&self, unit_id: u32) -> Option<u32> {
        let i = self.index_of(unit_id)?;
        let owner = self.units[i].owner;
        self.units[..i]
            .iter()
            .rev()
            .find(|u| u.alive && u.owner == owner)
            .map(|u| u.id)
    }

    /// `0x005b4c70`: missile linked to a silo (`CUnit+0xd0`).
    ///
    /// Retail returns the low 16 bits of the link word.  The Rust model
    /// records the live link in [`World::nuke_strikes`] (the strike's
    /// `silo`/`ghost` pair), so the full 32-bit missile id is returned here;
    /// `None` is the zero link.
    pub fn a14_linked_missile(&self, silo_id: u32) -> Option<u32> {
        self.nuke_strikes
            .iter()
            .find(|s| s.silo == silo_id)
            .map(|s| s.ghost)
    }

    /// `0x005b67b0`: write the silo's missile link (`CUnit+0xd0`).
    ///
    /// A zero argument clears the link, exactly like the raw dword store.
    /// The link lives in [`World::nuke_strikes`], so clearing drops the
    /// strike for that silo and a nonzero id re-attaches the strike whose
    /// `ghost` is that missile.  Returns whether a stored link changed.
    pub fn a14_set_linked_missile(&mut self, silo_id: u32, missile_id: u32) -> bool {
        if missile_id == 0 {
            let before = self.nuke_strikes.len();
            self.nuke_strikes.retain(|s| s.silo != silo_id);
            return before != self.nuke_strikes.len();
        }
        if let Some(strike) = self.nuke_strikes.iter_mut().find(|s| s.ghost == missile_id) {
            strike.silo = silo_id;
            return true;
        }
        false
    }

    /// `0x005b5578`: issue order row `0xa2` with the flags-93 bit set.
    ///
    /// The recovered wrapper forwards `0xa2` to the shared order issuer,
    /// which first sets bit 0 of `CUnit+0x93` and then calls a follow-up
    /// display/refresh hook.  The hook is not part of the decompilation, so
    /// only the flag write and the row dispatch are modelled.
    pub fn a14_issue_order_a2(&mut self, unit_id: u32) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        self.units[i].flags_93 |= 1;
        self.issue_order_id(unit_id, 0xa2, None, None)
    }

    /// `0x005b6c10`: issue an order whose target is a packed `(x, y)` pair.
    ///
    /// The recovered wrapper packs the two `u16` arguments into one dword
    /// (low half `x`, high half `y`) and forwards the row with no unit
    /// target.  The typed dispatcher takes the position directly; the raw
    /// list-head flag is not retained by [`World::issue_order_id`].
    pub fn a14_issue_order_packed_target(
        &mut self,
        unit_id: u32,
        order_id: u8,
        x: u16,
        y: u16,
    ) -> Result<(), SimError> {
        self.issue_order_id(unit_id, order_id, None, Some((x as i32, y as i32)))
    }

    /// `0x005b6c50`: issue an order with no target or position.
    ///
    /// The recovered body aborts when its second argument (the target) is
    /// nonzero, then forwards the row with a zero target and zero position.
    /// The Rust signature enforces the empty target.  The raw head-insertion
    /// flag has no typed equivalent and is not modelled.
    pub fn a14_issue_order_no_target(
        &mut self,
        unit_id: u32,
        order_id: u8,
    ) -> Result<(), SimError> {
        self.issue_order_id(unit_id, order_id, None, None)
    }

    /// `0x005b6f3c`: Medic heal-target gate.
    ///
    /// The recovered body accepts a target that is organic, currently below
    /// its maximum hit points, not a flyer, and without the raw status bit 1
    /// (`CUnit+0xdc` bit 1).  It also first checks the Medic requirement
    /// table, which the caller supplies here.  On rejection it writes error
    /// code `0x52d` to its out-parameter; `None` below means accepted.
    pub fn a14_heal_target_error(
        &self,
        unit_id: u32,
        requirements_met: bool,
        bit_1: bool,
    ) -> Option<u16> {
        let Some(i) = self.index_of(unit_id) else {
            return Some(0x52d);
        };
        let organic = self.data.unit(self.units[i].type_id).is_organic();
        let accepted = requirements_met
            && !self.is_air_unit(i)
            && !bit_1
            && organic
            && self.unit_is_damaged(i);
        (!accepted).then_some(0x52d)
    }

    /// `0x005b6fb2`: SCV repair-target gate.
    ///
    /// Accepts a damaged Terran mechanical unit with the raw status bit 0
    /// (`CUnit+0xdc` bit 0) set.  Rejection writes `0x373` to the
    /// out-parameter in retail; `None` below means accepted.
    pub fn a14_repair_target_error(&self, unit_id: u32, bit_0: bool) -> Option<u16> {
        let Some(i) = self.index_of(unit_id) else {
            return Some(0x373);
        };
        let u = &self.units[i];
        let accepted = self.unit_race_class(u.type_id) == 1
            && self.data.unit(u.type_id).is_mechanical()
            && bit_0
            && self.unit_is_damaged(i);
        (!accepted).then_some(0x373)
    }
}

#[cfg(test)]
mod tests_agent14 {
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::{NukeStrike, Order, World};
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    #[test]
    fn next_unit_walks_the_owners_insertion_list() {
        // 0x005b4bb0: the list is head-inserted, so the next pointer from the
        // newest unit is the unit spawned just before it.
        let mut w = world();
        let a = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let b = w.spawn(0, uid::MARINE, 20, 10).unwrap();
        let c = w.spawn(0, uid::MARINE, 30, 10).unwrap();
        let other = w.spawn(1, uid::MARINE, 40, 10).unwrap();
        let (aid, bid, cid) = (w.units[a].id, w.units[b].id, w.units[c].id);
        let oid = w.units[other].id;

        assert_eq!(w.a14_next_unit_of_owner(cid), Some(bid));
        assert_eq!(w.a14_next_unit_of_owner(bid), Some(aid));
        assert_eq!(w.a14_next_unit_of_owner(aid), None);
        assert_eq!(w.a14_next_unit_of_owner(oid), None);
    }

    #[test]
    fn addon_link_and_voice_request_follow_the_unit_fields() {
        // 0x005b66a0: raw CUnit+0x70 write, zero clears.
        // 0x005b6b60: owner byte plus the caller's voice-line index.
        let mut w = world();
        let i = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let u = &mut w.units[i];

        u.a14_set_addon_link(9);
        assert_eq!(u.addon, Some(9));
        u.a14_set_addon_link(0);
        assert_eq!(u.addon, None);

        assert_eq!(u.a14_voice_line(0x0f), (0, 0x0f));
        u.owner = 1;
        assert_eq!(u.a14_voice_line(3), (1, 3));
    }

    #[test]
    fn nuke_link_getter_and_setter_use_the_strike_table() {
        // 0x005b4c70 reads CUnit+0xd0; 0x005b67b0 writes it.  The Rust model
        // keeps the pair in World::nuke_strikes.
        let mut w = world();
        let i = w.spawn(0, uid::NUCLEAR_SILO, 10, 10).unwrap();
        let silo = w.units[i].id;
        w.nuke_strikes.push(NukeStrike {
            owner: 0,
            ghost: 0x77,
            silo: 0,
            target: (0, 0),
            timer: 90,
            dot: true,
            warned: false,
            missile: 0,
        });

        assert_eq!(w.a14_linked_missile(silo), None);
        assert!(w.a14_set_linked_missile(silo, 0x77));
        assert_eq!(w.a14_linked_missile(silo), Some(0x77));
        // No strike carries this missile id, so the raw link cannot change.
        assert!(!w.a14_set_linked_missile(silo, 0x99));
        // A zero write clears the link.
        assert!(w.a14_set_linked_missile(silo, 0));
        assert_eq!(w.a14_linked_missile(silo), None);
    }

    #[test]
    fn order_wrappers_dispatch_rows_through_the_typed_order() {
        // 0x005b5578 sets flags-93 bit 0 and issues row 0xa2.
        // 0x005b6c10 packs a point target; 0x005b6c50 issues an empty order.
        let mut w = world();
        let i = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let id = w.units[i].id;

        w.a14_issue_order_a2(id).unwrap();
        assert_ne!(w.units[i].flags_93 & 1, 0);

        w.a14_issue_order_packed_target(id, 6, 12, 34).unwrap();
        assert_eq!(w.units[i].order, Order::Move { pos: (12, 34) });

        w.a14_issue_order_no_target(id, 1).unwrap();
        assert_eq!(w.units[i].order, Order::HoldPosition);
    }

    #[test]
    fn heal_and_repair_target_gates_use_the_recovered_errors() {
        // 0x005b6f3c: organic, damaged, grounded, bit 1 clear.
        let mut w = world();
        let z = w.spawn(0, uid::ZERGLING, 10, 10).unwrap();
        w.units[z].hp = 1;
        let zid = w.units[z].id;
        assert_eq!(w.a14_heal_target_error(zid, true, false), None);
        assert_eq!(w.a14_heal_target_error(zid, false, false), Some(0x52d));
        assert_eq!(w.a14_heal_target_error(zid, true, true), Some(0x52d));

        // Organic flyers are rejected.
        let m = w.spawn(0, uid::MUTALISK, 20, 20).unwrap();
        w.units[m].hp = 1;
        let mid = w.units[m].id;
        assert_eq!(w.a14_heal_target_error(mid, true, false), Some(0x52d));

        // 0x005b6fb2: damaged Terran mechanical with bit 0 set.
        let s = w.spawn(0, uid::SCV, 30, 30).unwrap();
        w.units[s].hp = 1;
        let sid = w.units[s].id;
        assert_eq!(w.a14_repair_target_error(sid, true), None);
        assert_eq!(w.a14_repair_target_error(sid, false), Some(0x373));

        let marine = w.spawn(0, uid::MARINE, 40, 40).unwrap();
        w.units[marine].hp = 1;
        let marine_id = w.units[marine].id;
        assert_eq!(w.a14_repair_target_error(marine_id, true), Some(0x373));
    }
}
