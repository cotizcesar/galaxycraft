#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent19.rs
// ===========================================================================
// Translation batch 19.  Only this file is edited by the batch.
//
// Batch 19 covers `decompiled_uncited` routines in
// `0x005cc000..=0x005cffff`, the retail status-effect / derived-stat cluster.
// Every routine below either reads the `CUnit+0x110` status block or the
// derived maximum-energy value for a unit type.  Retail reports its failure
// paths with the non-returning assertion helper; batch 08's convention is
// kept here (`Err(())`), so a failed assertion stays testable instead of
// aborting the test runner.

/// One per-status update pass of `0x005cc5c0`, listed in the exact order the
/// recovered routine probes the status bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A19StatusPhase {
    /// `CUnit+0x114`: remaining Defensive Matrix updates.
    Matrix,
    /// `CUnit+0x117`: Lockdown.
    Lockdown,
    /// `CUnit+0x124`: Maelstrom.
    Maelstrom,
    /// `CUnit+0x118`: Irradiate.
    Irradiate,
    /// `CUnit+0x116`: Ensnare.
    Ensnare,
    /// `CUnit+0x119`: Stasis Field.
    Stasis,
    /// `CUnit+0x11a`: Plague.
    Plague,
    /// `CUnit+0x126`: Acid Spore stacks.
    AcidSpores,
}

impl SimUnit {
    /// `0x005cc840`: true when the unit still carries one of the status
    /// effects the routine probes.
    ///
    /// The recovered code returns zero only when the Defensive Matrix pool
    /// (`CUnit+0x112`) is empty **and** each of the seven status bytes it
    /// reads is zero.  The probes, in recovered order, are the lockdown
    /// timer (`CUnit+0x117`), the Maelstrom timer (`CUnit+0x124`), the
    /// irradiate timer (`CUnit+0x118`), the ensnare timer (`CUnit+0x116`),
    /// the stasis timer (`CUnit+0x119`), the plague timer (`CUnit+0x11a`)
    /// and the Acid Spore count (`CUnit+0x126`).
    ///
    /// Retail reads each timer as a single byte, so the wider timers this
    /// crate stores are truncated to their low byte before the test;
    /// [`SimUnit::status`] fields keep the retail units, so the comparison is
    /// exact for every value the simulators write.
    pub fn a19_has_blocking_status(&self) -> bool {
        let s = &self.status;
        s.matrix != 0
            || s.lockdown_ticks as u8 != 0
            || s.maelstrom_ticks as u8 != 0
            || s.irradiate_ticks as u8 != 0
            || s.ensnare_ticks as u8 != 0
            || s.stasis_ticks as u8 != 0
            || s.plague_ticks as u8 != 0
            || s.acid_spores != 0
    }

    /// `0x005cc5c0` (leading status-block copy): mirror the donor unit's
    /// `CUnit+0x110..=0x12f` status block into `self`.
    ///
    /// The routine runs before the per-status passes because the copied
    /// values decide which passes run.  Retail copies the 16-bit slot at
    /// `CUnit+0x110` (the corpse/death timer, [`SimUnit::death_timer`]), the
    /// 16-bit matrix pool, every effect byte from `CUnit+0x114` through
    /// `CUnit+0x126`, and the nine spore timers at `CUnit+0x127..=0x12f`.
    ///
    /// The block also contains five bytes/dwords (`CUnit+0x115`,
    /// `+0x11c`, `+0x120`, `+0x122`, `+0x125`) that this crate does not
    /// model; they have no destination field and are therefore not copied.
    pub fn a19_copy_status_block_from(&mut self, donor: &SimUnit) {
        self.death_timer = donor.death_timer;
        self.status.matrix = donor.status.matrix;
        self.status.matrix_ticks = donor.status.matrix_ticks;
        self.status.ensnare_ticks = donor.status.ensnare_ticks;
        self.status.lockdown_ticks = donor.status.lockdown_ticks;
        self.status.irradiate_ticks = donor.status.irradiate_ticks;
        self.status.stasis_ticks = donor.status.stasis_ticks;
        self.status.plague_ticks = donor.status.plague_ticks;
        self.status.psi_storm_guard = donor.status.psi_storm_guard;
        self.status.status_mask_121 = donor.status.status_mask_121;
        self.status.status_mask_123 = donor.status.status_mask_123;
        self.status.maelstrom_ticks = donor.status.maelstrom_ticks;
        self.status.acid_spores = donor.status.acid_spores;
        self.status.acid_spore_ticks = donor.status.acid_spore_ticks;
    }

    /// `0x005cc660` and its byte-identical sibling `0x005cc6a0`: the pair
    /// asserts that the unit belongs to the Ghost family and then returns the
    /// constant `10`.
    ///
    /// Both addresses decompile to the same three operations: query the
    /// Ghost-family predicate, abort on the non-returning assertion when it
    /// is false, otherwise return `0xa`.  The abort is reported as `Err(())`
    /// so the predicate stays testable.
    pub fn a19_ghost_family_constant(&self) -> Result<u8, ()> {
        if is_ghost_family_type(self.type_id) {
            Ok(10)
        } else {
            Err(())
        }
    }

    /// `0x005cff85`: the recovered body is a bare `return`.  The routine is a
    /// deliberate no-op tail (its one-byte size matches the single `ret`), so
    /// the translation takes no action and returns no value.
    pub fn a19_bare_return(&self) {}
}

impl World {
    /// `0x005cc5c0` (per-status dispatch): the ordered list of status update
    /// passes retail runs after copying the status block.
    ///
    /// The routine tests each status slot in a fixed sequence and invokes
    /// that status's update handler only when the slot is nonzero.  The
    /// recovered order is the Defensive Matrix update count
    /// (`CUnit+0x114`), lockdown (`0x117`), Maelstrom (`0x124`),
    /// irradiate (`0x118`), ensnare (`0x116`), stasis (`0x119`), plague
    /// (`0x11a`) and Acid Spore stacks (`0x126`).  The routine ends in a
    /// non-returning tail, which this helper does not model; the returned
    /// list is the observable dispatch order.
    ///
    /// The matrix pass is keyed on the update counter, not on the matrix
    /// pool, so a live pool with `CUnit+0x114 == 0` selects no pass.  As in
    /// [`SimUnit::a19_has_blocking_status`], each timer is read as the retail
    /// byte it is stored in.
    pub fn a19_status_update_phases(&self, i: usize) -> Vec<A19StatusPhase> {
        let s = &self.units[i].status;
        let mut phases = Vec::new();
        if s.matrix_ticks != 0 {
            phases.push(A19StatusPhase::Matrix);
        }
        if s.lockdown_ticks as u8 != 0 {
            phases.push(A19StatusPhase::Lockdown);
        }
        if s.maelstrom_ticks as u8 != 0 {
            phases.push(A19StatusPhase::Maelstrom);
        }
        if s.irradiate_ticks as u8 != 0 {
            phases.push(A19StatusPhase::Irradiate);
        }
        if s.ensnare_ticks as u8 != 0 {
            phases.push(A19StatusPhase::Ensnare);
        }
        if s.stasis_ticks as u8 != 0 {
            phases.push(A19StatusPhase::Stasis);
        }
        if s.plague_ticks as u8 != 0 {
            phases.push(A19StatusPhase::Plague);
        }
        if s.acid_spores != 0 {
            phases.push(A19StatusPhase::AcidSpores);
        }
        phases
    }

    /// `0x005cc7d8`: maximum energy in 1/256 points for the queried unit
    /// type, with the recovered Dark Archon override.
    ///
    /// The routine takes the queried type in `AX` and the unit in `ECX`.  For
    /// type `0x3f` (Dark Archon) whose order byte is the idle hold order
    /// (`CUnit+0x4d == 0x6a` with order state `CUnit+0x4e == 0`) it returns
    /// the fixed constant `0x3200` (50 whole points on the 1/256 scale);
    /// every other case falls through to [`World::max_energy_fixed`].
    ///
    /// The override is keyed on the queried type, not on the type of the unit
    /// in `ECX`: a Dark Archon is the unit whose idle order is inspected, but
    /// the returned value still describes `type_id`.
    pub fn a19_max_energy_fixed_for(&self, i: usize, type_id: u16) -> i32 {
        let unit = &self.units[i];
        if type_id == uid::DARK_ARCHON && matches!(unit.order, Order::HoldPosition) {
            return 0x3200;
        }
        self.max_energy_fixed(type_id, unit.owner)
    }
}

#[cfg(test)]
mod tests_agent19 {
    use super::A19StatusPhase;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::{Order, World};
    use std::sync::Arc;

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World) -> usize {
        world.spawn_px(0, uid::MARINE, 640, 640).unwrap()
    }

    #[test]
    fn blocking_status_probes_every_recovered_slot() {
        let mut w = open_world();
        let i = spawn(&mut w);
        assert!(!w.units[i].a19_has_blocking_status(), "clean unit");

        w.units[i].status.matrix = 0xfa00;
        assert!(w.units[i].a19_has_blocking_status(), "CUnit+0x112 pool");
        w.units[i].status.matrix = 0;

        w.units[i].status.lockdown_ticks = 0x83;
        assert!(w.units[i].a19_has_blocking_status(), "CUnit+0x117");
        w.units[i].status.lockdown_ticks = 0;

        w.units[i].status.maelstrom_ticks = 0x16;
        assert!(w.units[i].a19_has_blocking_status(), "CUnit+0x124");
        w.units[i].status.maelstrom_ticks = 0;

        w.units[i].status.irradiate_ticks = 0x20;
        assert!(w.units[i].a19_has_blocking_status(), "CUnit+0x118");
        w.units[i].status.irradiate_ticks = 0;

        w.units[i].status.ensnare_ticks = 0x4b;
        assert!(w.units[i].a19_has_blocking_status(), "CUnit+0x116");
        w.units[i].status.ensnare_ticks = 0;

        w.units[i].status.stasis_ticks = 0x83;
        assert!(w.units[i].a19_has_blocking_status(), "CUnit+0x119");
        w.units[i].status.stasis_ticks = 0;

        w.units[i].status.plague_ticks = 0x4b;
        assert!(w.units[i].a19_has_blocking_status(), "CUnit+0x11a");
        w.units[i].status.plague_ticks = 0;

        w.units[i].status.acid_spores = 1;
        assert!(w.units[i].a19_has_blocking_status(), "CUnit+0x126");
        w.units[i].status.acid_spores = 0;

        assert!(!w.units[i].a19_has_blocking_status(), "cleared again");
    }

    #[test]
    fn blocking_status_keeps_the_retail_byte_reads() {
        let mut w = open_world();
        let i = spawn(&mut w);
        // The timer slots are single bytes in retail: 0x100 truncates to 0.
        w.units[i].status.lockdown_ticks = 0x100;
        w.units[i].status.plague_ticks = 0x200;
        assert!(!w.units[i].a19_has_blocking_status());
        w.units[i].status.plague_ticks = 0x201;
        assert!(w.units[i].a19_has_blocking_status());
        // The matrix pool is a 16-bit slot, not a byte.
        w.units[i].status.plague_ticks = 0;
        w.units[i].status.matrix = 0x1_0000;
        assert!(w.units[i].a19_has_blocking_status());
    }

    #[test]
    fn status_update_phases_follow_the_recovered_order() {
        let mut w = open_world();
        let i = spawn(&mut w);
        assert!(w.a19_status_update_phases(i).is_empty());

        w.units[i].status.maelstrom_ticks = 0x16;
        w.units[i].status.acid_spores = 2;
        w.units[i].status.plague_ticks = 0x4b;
        w.units[i].status.matrix_ticks = 0xa8;
        w.units[i].status.stasis_ticks = 0x83;
        w.units[i].status.irradiate_ticks = 0x20;
        w.units[i].status.ensnare_ticks = 0x4b;
        w.units[i].status.lockdown_ticks = 0x83;

        assert_eq!(
            w.a19_status_update_phases(i),
            vec![
                A19StatusPhase::Matrix,
                A19StatusPhase::Lockdown,
                A19StatusPhase::Maelstrom,
                A19StatusPhase::Irradiate,
                A19StatusPhase::Ensnare,
                A19StatusPhase::Stasis,
                A19StatusPhase::Plague,
                A19StatusPhase::AcidSpores,
            ]
        );
    }

    #[test]
    fn status_update_phases_skip_inactive_and_truncated_slots() {
        let mut w = open_world();
        let i = spawn(&mut w);
        // A live matrix pool alone does not select the matrix pass; only the
        // CUnit+0x114 update counter does.
        w.units[i].status.matrix = 64_000;
        w.units[i].status.ensnare_ticks = 0x100;
        w.units[i].status.acid_spore_ticks[3] = 40;
        assert!(w.a19_status_update_phases(i).is_empty());

        w.units[i].status.ensnare_ticks = 1;
        w.units[i].status.stasis_ticks = 2;
        assert_eq!(
            w.a19_status_update_phases(i),
            vec![A19StatusPhase::Ensnare, A19StatusPhase::Stasis]
        );

        w.units[i].status.matrix_ticks = 1;
        assert_eq!(
            w.a19_status_update_phases(i),
            vec![
                A19StatusPhase::Matrix,
                A19StatusPhase::Ensnare,
                A19StatusPhase::Stasis,
            ]
        );
    }

    #[test]
    fn status_block_copy_mirrors_the_modelled_slots() {
        let mut w = open_world();
        let target = spawn(&mut w);
        let donor = w.spawn_px(1, uid::MARINE, 800, 800).unwrap();
        let target_hp = w.units[target].hp;
        let target_x = w.units[target].x;

        {
            let s = &mut w.units[donor].status;
            s.matrix = 64_000;
            s.matrix_ticks = 0xa8;
            s.ensnare_ticks = 0x4b;
            s.lockdown_ticks = 0x83;
            s.irradiate_ticks = 0x20;
            s.stasis_ticks = 0x83;
            s.plague_ticks = 0x4b;
            s.maelstrom_ticks = 0x16;
            s.psi_storm_guard = 1;
            s.status_mask_121 = 0x40;
            s.status_mask_123 = 0x08;
            s.acid_spores = 1;
            s.acid_spore_ticks[4] = 77;
        }
        w.units[donor].death_timer = 0x1234;

        let donor_unit = w.units[donor].clone();
        w.units[target].a19_copy_status_block_from(&donor_unit);

        let s = &w.units[target].status;
        assert_eq!(s.matrix, 64_000);
        assert_eq!(s.matrix_ticks, 0xa8);
        assert_eq!(s.ensnare_ticks, 0x4b);
        assert_eq!(s.lockdown_ticks, 0x83);
        assert_eq!(s.irradiate_ticks, 0x20);
        assert_eq!(s.stasis_ticks, 0x83);
        assert_eq!(s.plague_ticks, 0x4b);
        assert_eq!(s.maelstrom_ticks, 0x16);
        assert_eq!(s.psi_storm_guard, 1);
        assert_eq!(s.status_mask_121, 0x40);
        assert_eq!(s.status_mask_123, 0x08);
        assert_eq!(s.acid_spores, 1);
        assert_eq!(s.acid_spore_ticks[4], 77);
        assert_eq!(w.units[target].death_timer, 0x1234);
        // State outside the copied block is untouched, and the donor keeps
        // its own copy.
        assert_eq!(w.units[target].hp, target_hp);
        assert_eq!(w.units[target].x, target_x);
        assert_eq!(w.units[donor].status.acid_spore_ticks[4], 77);
        assert_eq!(w.units[donor].hp, target_hp);
    }

    #[test]
    fn dark_archon_hold_order_uses_the_recovered_override() {
        let mut w = open_world();
        let i = w.spawn_px(0, uid::DARK_ARCHON, 640, 640).unwrap();
        let archon_cap = w.max_energy_fixed(uid::DARK_ARCHON, 0);
        let marine_cap = w.max_energy_fixed(uid::MARINE, 0);

        w.units[i].order = Order::Idle;
        assert_eq!(w.a19_max_energy_fixed_for(i, uid::DARK_ARCHON), archon_cap);

        w.units[i].order = Order::HoldPosition;
        assert_eq!(w.a19_max_energy_fixed_for(i, uid::DARK_ARCHON), 0x3200);
        // The override is keyed on the queried type, not the unit's own type.
        assert_eq!(w.a19_max_energy_fixed_for(i, uid::MARINE), marine_cap);

        // Any other order falls through to the derived maximum.
        w.units[i].order = Order::Stop;
        assert_eq!(w.a19_max_energy_fixed_for(i, uid::DARK_ARCHON), archon_cap);
    }

    #[test]
    fn ghost_family_constant_requires_the_ghost_rows() {
        let mut w = open_world();
        let ghost = w.spawn_px(0, uid::GHOST, 640, 640).unwrap();
        let marine = spawn(&mut w);
        assert_eq!(w.units[ghost].a19_ghost_family_constant(), Ok(10));
        assert_eq!(w.units[marine].a19_ghost_family_constant(), Err(()));
    }

    #[test]
    fn bare_return_tail_leaves_the_unit_untouched() {
        let mut w = open_world();
        let i = spawn(&mut w);
        w.units[i].status.lockdown_ticks = 0x83;
        let hp = w.units[i].hp;
        let position = (w.units[i].x, w.units[i].y);
        let order = format!("{:?}", w.units[i].order);
        let status = format!("{:?}", w.units[i].status);
        w.units[i].a19_bare_return();
        assert_eq!(w.units[i].hp, hp);
        assert_eq!((w.units[i].x, w.units[i].y), position);
        assert_eq!(format!("{:?}", w.units[i].order), order);
        assert_eq!(format!("{:?}", w.units[i].status), status);
    }
}
