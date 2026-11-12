#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent10.rs
// ===========================================================================
// Translation batch 10.
//
// Only this file is edited by the batch.  Every method below is an
// inherent helper on [`SimUnit`] (prefixed `a10_`) whose observable
// behaviour is recovered from a `decompiled_uncited` routine in
// `0x005d0000..=0x005dffff`.  The routines in that range are the retail
// status-panel / status-effect helpers, and the comments document the
// `CUnit` offsets each one touches.  None of them needs new persistent
// state: the existing [`crate::sim::Status`], [`SimUnit::production`] and
// [`SimUnit::connected_unit`] fields already model the retail storage.

impl SimUnit {
    /// `0x005d21a0` (`FUN_005d21a0`, size 13): reports whether
    /// `CUnit+0x08` is greater than `0xa00`.  Retail stores hit points in
    /// 1/256 points, so `0xa00` is ten whole points; the predicate is used
    /// by the status panel to decide whether the unit is above that
    /// threshold.
    ///
    /// [`SimUnit::hp`] uses the same 1/256 fixed-point scale as the retail
    /// field, so the comparison is exact.
    pub fn a10_hp_above_ten(&self) -> bool {
        self.hp > 0xa00
    }

    /// `0x005d26a0` (`FUN_005d26a0`, size 18): the routine aborts when
    /// `CUnit+0x114` is nonzero.  That byte is the remaining Defensive
    /// Matrix status-update count, so the translated predicate reports
    /// whether a matrix is currently running on the unit.
    pub fn a10_defensive_matrix_active(&self) -> bool {
        self.status.matrix_ticks != 0
    }

    /// `0x005d27da` (`FUN_005d27da`, size 18): compiled conditional store
    /// (`cmovbe`-style) into the lockdown timer `CUnit+0x117`.  The value
    /// already in the timer is kept unless the comparison flags say
    /// below-or-equal (`CF || ZF`), in which case `candidate` is written.
    /// Returns the byte that was stored.
    ///
    /// The retail byte store truncates the current value to its low eight
    /// bits; that is reproduced here before the conditional selection.
    pub fn a10_store_lockdown_ticks_if(&mut self, candidate: u8, below_or_equal: bool) -> u8 {
        let existing = self.status.lockdown_ticks as u8;
        let stored = if below_or_equal { candidate } else { existing };
        self.status.lockdown_ticks = i32::from(stored);
        stored
    }

    /// `0x005d22bf` (`FUN_005d22bf`, size 56): after the death/removal
    /// sound is queued, the routine tests `CUnit+0x126` and, when it is
    /// nonzero, calls `FUN_005dc4a0` to wipe every acid-spore stack.
    /// Reproduces exactly that observable effect and reports whether any
    /// stack was present.
    pub fn a10_clear_acid_spores_if_present(&mut self) -> bool {
        if self.status.acid_spores == 0 {
            return false;
        }
        self.status.clear_acid_spores();
        true
    }

    /// `0x005dc5e0` (`FUN_005dc5e0`, size 65): plague status cue.  When
    /// `CUnit+0x11a` is nonzero the routine temporarily zeroes the timer,
    /// plays cue `0x184 + (local_player & 0xff)` for a unit that has not
    /// finished construction (`FUN_005b5840` is the `CUnit+0xdc` bit-4
    /// "completed" flag), and restores the timer.  The timer therefore
    /// never changes; the observable result is the cue id, which this
    /// helper returns.
    pub fn a10_plague_sound_cue(&self, player: u8) -> Option<u16> {
        if self.status.plague_ticks != 0 && !self.completed {
            Some(0x184 + u16::from(player))
        } else {
            None
        }
    }

    /// `0x005dc630` (`FUN_005dc630`, size 68): acid-spore status cue.
    /// When `CUnit+0x126` is nonzero the routine derives a variant
    /// `min(count / 2, 3)` and plays cue
    /// `0x3da + (local_player & 0xff) * 4 + variant`.  The helper returns
    /// the same cue id without playing it.
    pub fn a10_acid_spore_sound_cue(&self, player: u8) -> Option<u16> {
        if self.status.acid_spores == 0 {
            return None;
        }
        let variant = (u16::from(self.status.acid_spores) >> 1).min(3);
        Some(0x3da + u16::from(player) * 4 + variant)
    }

    /// `0x005dc680` (`FUN_005dc680`, size 79): ensnare status cue.  When
    /// `CUnit+0x116` is nonzero the routine temporarily saves the timer,
    /// zeroes it, queues cue `0x180 + (local_player & 0xff)` for a unit
    /// that is not complete, writes `0x4b`, calls the noreturn
    /// `FUN_005da480`, and restores the saved timer.  The timer is
    /// unchanged; the observable result is the cue id.
    pub fn a10_ensnare_sound_cue(&self, player: u8) -> Option<u16> {
        if self.status.ensnare_ticks != 0 && !self.completed {
            Some(0x180 + u16::from(player))
        } else {
            None
        }
    }

    /// `0x005dcb50` (`FUN_005dcb50`, size 40): returns 1 when
    /// `CUnit+0x80` (the Hatchery/Lair/Hive owning this larva) is null and
    /// otherwise reaches a noreturn error.  Translated as the ownership
    /// predicate.
    pub fn a10_larva_orphaned(&self) -> bool {
        self.connected_unit.is_none()
    }

    /// `0x005d5c30` (`FUN_005d5c30`, size 68): reads the 16-bit unit type
    /// from production-queue slot `param_1` at `CUnit+0xb0 + param_1 * 2`.
    /// The retail routine also folds the two per-slot state bits from
    /// `CUnit+0x132` into the high half when `FUN_0060b560()` reports a
    /// Brood War build; those state bits are not modeled by
    /// [`SimUnit::production`], so the translated getter returns the
    /// queued unit type and `None` for an empty or non-training slot.
    pub fn a10_production_slot_type(&self, slot: usize) -> Option<u16> {
        match self.production.get(slot) {
            Some(Order::Train { unit_type }) => Some(*unit_type),
            _ => None,
        }
    }

    /// `0x005d423a` (`FUN_005d423a`, size 52): remaps the incoming
    /// 16-bit unit/button id before `FUN_005afa00` receives it:
    /// `0x19 -> 0x17` and `0x1e -> 5`; every other id is passed through.
    pub fn a10_remap_button_unit_id(id: u16) -> u16 {
        match id {
            0x19 => 0x17,
            0x1e => 5,
            other => other,
        }
    }
}

#[cfg(test)]
mod tests_agent10 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::World;
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
    fn hp_threshold_is_ten_whole_points() {
        let mut w = open_world();
        let i = spawn(&mut w);
        w.units[i].hp = 0xa00;
        assert!(!w.units[i].a10_hp_above_ten(), "exactly ten points");
        w.units[i].hp = 0xa01;
        assert!(w.units[i].a10_hp_above_ten(), "just above ten points");
        w.units[i].hp = 0;
        assert!(!w.units[i].a10_hp_above_ten());
    }

    #[test]
    fn defensive_matrix_predicate_reads_the_status_timer() {
        let mut w = open_world();
        let i = spawn(&mut w);
        w.units[i].status.matrix_ticks = 0;
        assert!(!w.units[i].a10_defensive_matrix_active());
        w.units[i].status.matrix_ticks = 0xa8;
        assert!(w.units[i].a10_defensive_matrix_active());
    }

    #[test]
    fn lockdown_store_keeps_or_replaces_the_byte() {
        let mut w = open_world();
        let i = spawn(&mut w);
        w.units[i].status.lockdown_ticks = 5;
        // above-or-equal comparison: the existing timer stays.
        assert_eq!(w.units[i].a10_store_lockdown_ticks_if(9, false), 5);
        assert_eq!(w.units[i].status.lockdown_ticks, 5);
        // below-or-equal: the candidate is written.
        assert_eq!(w.units[i].a10_store_lockdown_ticks_if(9, true), 9);
        assert_eq!(w.units[i].status.lockdown_ticks, 9);
        // The retail store is a byte write: keep only the low eight bits.
        w.units[i].status.lockdown_ticks = 0x1_0105;
        assert_eq!(w.units[i].a10_store_lockdown_ticks_if(0, false), 5);
    }

    #[test]
    fn acid_spore_cleanup_reports_and_clears_every_stack() {
        let mut w = open_world();
        let i = spawn(&mut w);
        w.units[i].status.acid_spores = 0;
        assert!(!w.units[i].a10_clear_acid_spores_if_present());
        w.units[i].status.acid_spores = 3;
        w.units[i].status.acid_spore_ticks[0] = 40;
        assert!(w.units[i].a10_clear_acid_spores_if_present());
        assert_eq!(w.units[i].status.acid_spores, 0);
        assert_eq!(w.units[i].status.acid_spore_ticks, [0; 9]);
        assert!(!w.units[i].a10_clear_acid_spores_if_present());
    }

    #[test]
    fn plague_cue_only_for_active_unfinished_units() {
        let mut w = open_world();
        let i = spawn(&mut w);
        w.units[i].status.plague_ticks = 0;
        assert_eq!(w.units[i].a10_plague_sound_cue(2), None);
        w.units[i].status.plague_ticks = 100;
        w.units[i].completed = false;
        assert_eq!(w.units[i].a10_plague_sound_cue(2), Some(0x184 + 2));
        w.units[i].completed = true;
        assert_eq!(w.units[i].a10_plague_sound_cue(2), None);
    }

    #[test]
    fn acid_spore_cue_scales_with_count_and_player() {
        let mut w = open_world();
        let i = spawn(&mut w);
        w.units[i].status.acid_spores = 0;
        assert_eq!(w.units[i].a10_acid_spore_sound_cue(1), None);
        w.units[i].status.acid_spores = 1;
        assert_eq!(w.units[i].a10_acid_spore_sound_cue(1), Some(0x3da + 4));
        w.units[i].status.acid_spores = 7;
        assert_eq!(w.units[i].a10_acid_spore_sound_cue(1), Some(0x3da + 4 + 3));
        w.units[i].status.acid_spores = 9;
        assert_eq!(w.units[i].a10_acid_spore_sound_cue(0), Some(0x3da + 3));
        w.units[i].status.acid_spores = 4;
        assert_eq!(w.units[i].a10_acid_spore_sound_cue(3), Some(0x3da + 12 + 2));
    }

    #[test]
    fn ensnare_cue_only_for_active_unfinished_units() {
        let mut w = open_world();
        let i = spawn(&mut w);
        w.units[i].status.ensnare_ticks = 0;
        assert_eq!(w.units[i].a10_ensnare_sound_cue(0), None);
        w.units[i].status.ensnare_ticks = 75;
        w.units[i].completed = false;
        assert_eq!(w.units[i].a10_ensnare_sound_cue(0), Some(0x180));
        w.units[i].completed = true;
        assert_eq!(w.units[i].a10_ensnare_sound_cue(1), None);
    }

    #[test]
    fn larva_is_orphaned_without_a_hatchery_link() {
        let mut w = open_world();
        let i = spawn(&mut w);
        w.units[i].connected_unit = None;
        assert!(w.units[i].a10_larva_orphaned());
        w.units[i].connected_unit = Some(42);
        assert!(!w.units[i].a10_larva_orphaned());
    }

    #[test]
    fn production_slots_expose_only_queued_training_types() {
        let mut w = open_world();
        let i = spawn(&mut w);
        assert_eq!(w.units[i].a10_production_slot_type(0), None);
        w.units[i].production.push_back(Order::Train {
            unit_type: uid::MARINE,
        });
        w.units[i].production.push_back(Order::Train {
            unit_type: uid::SCV,
        });
        assert_eq!(w.units[i].a10_production_slot_type(0), Some(uid::MARINE));
        assert_eq!(w.units[i].a10_production_slot_type(1), Some(uid::SCV));
        assert_eq!(w.units[i].a10_production_slot_type(2), None);
        assert_eq!(w.units[i].a10_production_slot_type(usize::MAX), None);
    }

    #[test]
    fn button_id_remap_matches_the_retail_table() {
        assert_eq!(SimUnit::a10_remap_button_unit_id(0x19), 0x17);
        assert_eq!(SimUnit::a10_remap_button_unit_id(0x1e), 5);
        assert_eq!(SimUnit::a10_remap_button_unit_id(0x18), 0x18);
        assert_eq!(SimUnit::a10_remap_button_unit_id(0), 0);
    }
}
