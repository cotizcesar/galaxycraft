#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent13.rs
// ===========================================================================
// Translation batch 13.  Only this file is edited by the batch.
//
// Recovered `CUnit` helpers in `0x005b0000..=0x005b3fff` whose persistent
// effect lands on fields the Rust model already stores.  Retail keeps the
// death list and the sprite lists out of the unit record, so the routines
// that only splice list pointers have no separate Rust state; the methods
// below model the record-level effects that survive in [`World`] and
// [`SimUnit`].

impl World {
    /// `0x005b0d20`: head of the removed-unit (death) list (`0x00fa3700`).
    ///
    /// Death cleanup and the subunit removal path splice a record into this
    /// list with the insert-behind-head helper, so the head stays on the
    /// first record that reached the list.  [`World::units`] keeps dead
    /// records in spawn order and never removes them, so the first unit with
    /// [`SimUnit::alive`] clear is that head.
    pub fn a13_first_dead_unit(&self) -> Option<u32> {
        self.units
            .iter()
            .find(|unit| !unit.alive)
            .map(|unit| unit.id)
    }
}

impl SimUnit {
    /// `0x005b0b4b`: clear the `CUnit+0xdc` bit-0 "ready/completed" flag.
    ///
    /// The routine is the removal counterpart of the construction finisher:
    /// it clears bit 0 of the state dword and returns the unit type id.  In
    /// the Rust model that bit is [`SimUnit::completed`].  Retail also asks
    /// for a command-card refresh and clears a presentation-only shield
    /// flag, neither of which has a persistent model field.
    pub fn a13_clear_completed_flag(&mut self) {
        self.completed = false;
    }

    /// `0x005b3910`: recovery-side construction hit-point rate at
    /// `CUnit+0xa8`.
    ///
    /// Retail divides the type's maximum raw hit points less the ten-percent
    /// construction discount, minus one, plus the build time by the build
    /// time (integer division, both operands positive):
    ///
    /// `rate = (hp - hp / 10 - 1 + build_time) / build_time`
    ///
    /// and clamps the result to one.  The engine recomputes this gain inside
    /// `update_construction` instead of caching it, so the method returns the
    /// value rather than storing `CUnit+0xa8`.
    pub fn a13_construction_hp_rate(&self, data: &GameData) -> i16 {
        let ty = data.unit(self.type_id);
        let hp = ty.hit_points_raw as i32;
        let build = i32::from(ty.build_time);
        if build != 0 {
            let rate = (hp - hp / 10 - 1 + build) / build;
            if rate != 0 {
                return rate.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
        }
        1
    }

    /// `0x005b0380`: copy the unit's status-effect block from another
    /// record.
    ///
    /// Retail copies the words and bytes at `CUnit+0x110..+0x12f` verbatim:
    /// the death timer, the Defensive Matrix counter, the psi-storm guard,
    /// the per-unit masks, the Maelstrom timer, the Acid Spore count and the
    /// nine Acid Spore lifetime slots.  Those are exactly the fields held by
    /// [`SimUnit::death_timer`] and [`SimUnit::status`], so the modelled
    /// subset is cloned in one step.  The raw block also covers a handful of
    /// bytes with no Rust field; they are not represented here.
    pub fn a13_copy_status_block_from(&mut self, other: &SimUnit) {
        self.death_timer = other.death_timer;
        self.status = other.status.clone();
    }

    /// `0x005b3220`: drop the linked order target while no overlay is
    /// active.
    ///
    /// If `CUnit+0xe0` is non-zero the routine returns immediately.  Otherwise
    /// it clears that overlay byte, and when the linked target at `CUnit+0xc0`
    /// is non-zero it sets bit 2 of the order/state byte `CUnit+0x93` before
    /// dropping the link.  The Rust model derives the linked target from the
    /// current order's unit slot, so the link is cleared with
    /// [`SimUnit::clear_order_target`]; the return value says whether a link
    /// was present.  Retail additionally refreshes a command-card lookup,
    /// which is presentation-only.
    pub fn a13_clear_idle_link(&mut self) -> bool {
        if self.overlay_flags != 0 {
            return false;
        }
        let linked = self.order_unit_target();
        if let Some(id) = linked {
            self.flags_93 |= 4;
            self.clear_order_target(id);
        }
        self.overlay_flags = 0;
        linked.is_some()
    }

    /// `0x005b1239`: restore the unit's all-visible mask (`CUnit+0xe4`).
    ///
    /// The handler writes `0xffffffff` to the per-player visibility mask and
    /// reports success.  Retail also releases the sprite image and gates a
    /// voice line on the current order being row `0x23`; the mask is the only
    /// persistent state the model stores, and the sprite is owned by the
    /// renderer rather than [`SimUnit`].
    pub fn a13_restore_visibility(&mut self) -> bool {
        self.visibility_mask = u32::MAX;
        true
    }
}

#[cfg(test)]
mod tests_agent13 {
    use super::*;
    use crate::ids::unit as uid;
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

    /// `0x005b0d20`: the death-list head is the oldest dead record, and live
    /// units are never part of that view.
    #[test]
    fn dead_list_head_is_the_oldest_dead_unit() {
        let mut w = world();
        let first = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let second = w.spawn(0, uid::MARINE, 20, 10).unwrap();
        let third = w.spawn(1, uid::SCV, 30, 10).unwrap();

        assert_eq!(w.a13_first_dead_unit(), None);

        w.units[second].alive = false;
        assert_eq!(w.a13_first_dead_unit(), Some(w.units[second].id));

        w.units[first].alive = false;
        assert_eq!(w.a13_first_dead_unit(), Some(w.units[first].id));

        w.units[third].alive = false;
        assert_eq!(w.a13_first_dead_unit(), Some(w.units[first].id));
    }

    /// `0x005b0b4b`: only the completion bit is dropped; hit points are left
    /// for the removal path.
    #[test]
    fn clearing_completed_bit_leaves_hit_points_alone() {
        let mut w = world();
        let u = w.spawn(0, uid::BARRACKS, 24, 24).unwrap();
        w.units[u].completed = true;
        w.units[u].hp = 333;

        w.units[u].a13_clear_completed_flag();

        assert!(!w.units[u].completed);
        assert_eq!(w.units[u].hp, 333);
    }

    /// `0x005b3910`: the stored construction rate matches the retail
    /// integer formula and is never zero.
    #[test]
    fn construction_hp_rate_matches_retail_formula() {
        let mut w = world();
        for type_id in [uid::MARINE, uid::BARRACKS, uid::COMMAND_CENTER] {
            let u = w.spawn(0, type_id, 32, 32).unwrap();
            let ty = w.data.unit(type_id);
            let hp = ty.hit_points_raw as i32;
            let build = i32::from(ty.build_time);
            let expected = if build == 0 {
                1
            } else {
                let rate = (hp - hp / 10 - 1 + build) / build;
                rate.max(1) as i16
            };

            let rate = w.units[u].a13_construction_hp_rate(&w.data);
            assert_eq!(rate, expected, "type {type_id:#x}");
            assert!(rate >= 1, "type {type_id:#x}");
        }
    }

    /// `0x005b0380`: the death timer and the whole Status block move to the
    /// destination while the source keeps its values.
    #[test]
    fn status_block_copy_moves_modelled_effect_state() {
        let mut w = world();
        let source = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let dest = w.spawn(1, uid::MARINE, 20, 10).unwrap();

        w.units[dest].death_timer = 1;
        w.units[dest].status.matrix = 0;
        w.units[dest].status.acid_spores = 0;

        w.units[source].death_timer = 37;
        w.units[source].status.matrix = 512;
        w.units[source].status.matrix_ticks = 0xa8;
        w.units[source].status.acid_spores = 2;
        w.units[source].status.acid_spore_ticks[0] = 0x96;
        w.units[source].status.psi_storm_guard = 1;
        w.units[source].status.status_mask_121 = 0x12;
        w.units[source].status.status_mask_123 = 0x34;
        w.units[source].status.maelstrom_ticks = 0x16;
        w.units[source].status.parasite = true;
        w.units[source].status.cloaked = true;

        let donor = w.units[source].clone();
        w.units[dest].a13_copy_status_block_from(&donor);

        assert_eq!(w.units[dest].death_timer, 37);
        assert_eq!(w.units[dest].status.matrix, 512);
        assert_eq!(w.units[dest].status.matrix_ticks, 0xa8);
        assert_eq!(w.units[dest].status.acid_spores, 2);
        assert_eq!(w.units[dest].status.acid_spore_ticks[0], 0x96);
        assert_eq!(w.units[dest].status.psi_storm_guard, 1);
        assert_eq!(w.units[dest].status.status_mask_121, 0x12);
        assert_eq!(w.units[dest].status.status_mask_123, 0x34);
        assert_eq!(w.units[dest].status.maelstrom_ticks, 0x16);
        assert!(w.units[dest].status.parasite);
        assert!(w.units[dest].status.cloaked);

        // The source is untouched.
        assert_eq!(w.units[source].death_timer, 37);
        assert_eq!(w.units[source].status.matrix, 512);
    }

    /// `0x005b3220`: an active overlay blocks the cleanup; otherwise the
    /// linked target is dropped and bit 2 of `CUnit+0x93` is recorded.
    #[test]
    fn idle_link_cleanup_drops_target_only_without_overlay() {
        let mut w = world();
        let scv = w.spawn(0, uid::SCV, 10, 10).unwrap();
        let mineral = w.spawn(0, uid::MINERAL_FIELD, 20, 10).unwrap();
        let target = w.units[mineral].id;

        w.units[scv].overlay_flags = 1;
        w.units[scv].order = Order::Gather { resource: target };
        assert!(!w.units[scv].a13_clear_idle_link());
        assert_eq!(w.units[scv].order_unit_target(), Some(target));
        assert_eq!(w.units[scv].overlay_flags, 1);
        assert_eq!(w.units[scv].flags_93 & 4, 0);

        w.units[scv].overlay_flags = 0;
        assert!(w.units[scv].a13_clear_idle_link());
        assert_eq!(w.units[scv].order_unit_target(), None);
        assert!(matches!(w.units[scv].order, Order::Idle));
        assert_eq!(w.units[scv].flags_93 & 4, 4);

        // Nothing left to clear.
        assert!(!w.units[scv].a13_clear_idle_link());
    }

    /// `0x005b1239`: the visibility mask is restored to all ones and the
    /// handler reports success.
    #[test]
    fn visibility_restore_sets_every_player_bit() {
        let mut w = world();
        let u = w.spawn(1, uid::MARINE, 12, 12).unwrap();
        w.units[u].visibility_mask = 0;

        assert!(w.units[u].a13_restore_visibility());
        assert_eq!(w.units[u].visibility_mask, u32::MAX);
        assert!(w.units[u].a13_restore_visibility());
    }
}
