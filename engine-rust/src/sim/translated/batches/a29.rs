#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent29.rs
// ===========================================================================
// Translation batch 29.  Only this file is edited by the batch.
//
// Fifteen `decompiled_uncited` entry points in `0x005b0000..=0x005b1fff`,
// the `CUnit` record/destructor and unit-pool block of the retail engine.
// Those routines splice the intrusive unit lists, free the AI slot, return a
// pool slot to the free list, and reset record fields.  Only the
// record-level effects that the Rust model already stores are reproduced
// here: the unit type, order/queue/target state, the `field_090`,
// `field_091`, `field_096`, `field_100`, `overlay_flags`, `visibility_mask`
// and `hangar` fields, and the cargo list.  The list pointers, sprite flags,
// voice/sound cues and pool bookkeeping live outside [`SimUnit`] and are not
// modelled.  Retail failure branches call a noreturn abort helper; they are
// translated as `false`/`None` so the caller can tell the accepted path.

impl SimUnit {
    /// `0x005b0086`: removal finalizer for the map-trap units.
    ///
    /// The recovered body first picks one of three intrusive-list exits:
    /// temporary rows `0x21`/`0x65` (Scanner Sweep and Map Revealer) unlink
    /// from both the sprite-hidden and temporary lists, the hidden-sprite
    /// condition re-inserts the record, and everything else takes the normal
    /// unlink.  All of that is list bookkeeping that [`World::units`] does
    /// not store.
    ///
    /// The record-level writes happen after that dispatch: rows `0xcb` and
    /// `0xd1..=0xd5` (the Floor/Wall Missile and Flame Traps) receive the
    /// visibility mask `0x80000000` at `CUnit+0xe4` and have `CUnit+0x91`
    /// cleared.  Returns whether that trap branch ran.
    pub fn a29_finalize_trap_removal(&mut self) -> bool {
        if matches!(self.type_id, 0xcb | 0xd1..=0xd5) {
            self.visibility_mask = 0x8000_0000;
            self.field_091 = 0;
            true
        } else {
            false
        }
    }

    /// `0x005b0342`: initial facing for a newly placed unit.
    ///
    /// The routine reads the per-unit byte table at `0x00f8e718`.  The value
    /// `0x20` is the random sentinel: the caller's `roll` draw is masked to
    /// five bits instead.  The chosen direction is then shifted left three
    /// times (retail's 0..31 direction unit becomes a 0..255 sprite angle)
    /// and stored through the sprite direction setter.  Trap rows
    /// `0xcb..=0xd5` additionally take the hidden-to-visible re-insertion
    /// branch, which is reported as `true`.
    ///
    /// The per-type table is a constant in the shipped executable, not part
    /// of the bundled `GameData`, so the caller supplies the row value.
    pub fn a29_initial_facing(&mut self, table_value: u8, roll: u8) -> bool {
        let direction = if table_value == 0x20 {
            roll & 0x1f
        } else {
            table_value
        };
        self.facing = direction.wrapping_shl(3);
        (0xcb..=0xd5).contains(&self.type_id)
    }

    /// `0x005b05a0`: removal routing by unit type.
    ///
    /// Three outcomes are recovered.  The hangar rows — Carrier `0x48`,
    /// Warbringer `0x51`, Gantrithor `0x52` and Reaver `0x53` — walk and
    /// empty the interceptor/scarab list; the model keeps that list as the
    /// [`SimUnit::hangar`] count, so it is zeroed.  The ghost-hero rows —
    /// Ghost `0x01`, Sarah Kerrigan `0x10`, Samir Duran `0x63`, Alexei
    /// Stukov `0x64` and Infested Duran `0x68` — clear the nuke/order slot
    /// at `CUnit+0xd0`, which has no Rust field.  Every other row falls
    /// through.  Returns `1` for the hangar route, `2` for the ghost-hero
    /// route and `0` otherwise.
    pub fn a29_apply_removal_route(&mut self) -> u8 {
        if matches!(self.type_id, 0x48 | 0x51 | 0x52 | 0x53) {
            self.hangar = 0;
            1
        } else if matches!(self.type_id, 0x01 | 0x10 | 0x63 | 0x64 | 0x68) {
            2
        } else {
            0
        }
    }

    /// `0x005b07aa`: subunit-aware teardown, first entry point.
    ///
    /// A subunit (`units.dat` flag `0x10`) only leaves its intrusive lists,
    /// which changes no modelled field, so the method reports `false` and
    /// leaves the record untouched.  Every other row runs the shared record
    /// cleanup: non-buildings clear `CUnit+0x96`, a non-zero `CUnit+0x100`
    /// AI slot is freed, the current target at `CUnit+0xec` is dropped and
    /// the order row at `CUnit+0xa6` is reset.  Returns whether a modelled
    /// field changed.
    pub fn a29_teardown_subunit_aware(&mut self, data: &GameData) -> bool {
        if data.unit(self.type_id).is_subunit() {
            return false;
        }
        self.a29_teardown_state(data, true)
    }

    /// `0x005b07ad`: subunit-aware teardown, companion entry point.
    ///
    /// Same modelled effects as [`SimUnit::a29_teardown_subunit_aware`].  The
    /// two bodies differ only in their prologues and in the already-unlinked
    /// branch zeroing the `CUnit+0x70` pointer, neither of which the Rust
    /// record stores.
    pub fn a29_teardown_subunit_aware_alt(&mut self, data: &GameData) -> bool {
        if data.unit(self.type_id).is_subunit() {
            return false;
        }
        self.a29_teardown_state(data, true)
    }

    /// `0x005b0927`: teardown without the subunit early-out.
    ///
    /// Every row reaches the record cleanup, including subunits: the
    /// non-building `CUnit+0x96` clear, the AI-slot free, the target drop and
    /// the order-row reset all run.  Returns whether a modelled field
    /// changed.
    pub fn a29_teardown_full(&mut self, data: &GameData) -> bool {
        self.a29_teardown_state(data, true)
    }

    /// `0x005b0978`: tail of the teardown chain.
    ///
    /// This entry point begins after the non-building block, so `CUnit+0x96`
    /// is left untouched; the AI slot, current target and order row are
    /// still cleared.  Returns whether a modelled field changed.
    pub fn a29_teardown_tail(&mut self, data: &GameData) -> bool {
        self.a29_teardown_state(data, false)
    }

    /// Shared modelled body of the `0x005b07aa`/`0x005b07ad`/`0x005b0927`/
    /// `0x005b0978` teardown entries.  `clear_nonbuilding` is `false` for
    /// the tail entry, which starts after the `CUnit+0x96` block.
    fn a29_teardown_state(&mut self, data: &GameData, clear_nonbuilding: bool) -> bool {
        let mut changed = false;
        if clear_nonbuilding && !data.unit(self.type_id).is_building() && self.field_096 != 0 {
            self.field_096 = 0;
            changed = true;
        }
        if self.field_100 != 0 {
            self.field_100 = 0;
            changed = true;
        }
        if self.target.is_some() {
            self.target = None;
            changed = true;
        }
        if self.order != Order::Idle || self.order_row != 0 {
            self.order = Order::Idle;
            self.order_row = 0;
            changed = true;
        }
        changed
    }

    /// `0x005b11e8`: recreate/respawn state reset.
    ///
    /// Vulture rows `0x02` and `0x13` get their three Spider Mines restored;
    /// retail stores that counter in the low byte of `CUnit+0xc0`, the same
    /// slot the Rust model uses for [`SimUnit::hangar`].  Every row then
    /// receives the all-visible mask `0xffffffff` at `CUnit+0xe4`.  The
    /// sprite-hidden/visible bits and the type `0x23` special case are
    /// presentation-only.  Returns `true` for the Vulture branch.
    pub fn a29_restore_spawned_state(&mut self) -> bool {
        let vulture = matches!(self.type_id, 0x02 | 0x13);
        if vulture {
            self.hangar = 3;
        }
        self.visibility_mask = u32::MAX;
        vulture
    }

    /// `0x005b11fd`: detach-path visibility reset.
    ///
    /// The routine restores the all-visible mask at `CUnit+0xe4` and then
    /// dispatches on the subunit flag: subunits set a sprite bit, every other
    /// row runs the normal visibility refresh.  The mask is the only stored
    /// field; the subunit dispatch is reported as the return value.
    pub fn a29_reset_visibility_mask(&mut self, data: &GameData) -> bool {
        self.visibility_mask = u32::MAX;
        data.unit(self.type_id).is_subunit()
    }

    /// `0x005b1400`: blank a unit record for a new type.
    ///
    /// The recovered initializer constructs the type's image, then clears the
    /// link/count words at `CUnit+0xc0..+0xcc`, the overlay byte at
    /// `CUnit+0xe0` and `CUnit+0x91`, writes the type at `CUnit+0x64` and
    /// the constant `8` at `CUnit+0x90`, and returns the type.  In the model
    /// the `CUnit+0xc0`/`+0xec` links are the current target and order queue,
    /// so those are cleared together with the current order row.  Returns
    /// the stored type id.
    pub fn a29_reinitialize_record(&mut self, type_id: u16) -> u16 {
        self.target = None;
        self.queue.clear();
        self.overlay_flags = 0;
        self.field_091 = 0;
        self.type_id = type_id;
        self.field_090 = 8;
        self.order = Order::Idle;
        self.order_row = 0;
        type_id
    }
}

impl World {
    /// `0x005b071f`: release the eight-slot unit-reference array.
    ///
    /// Retail runs this only when the state dword's bit 30 is set (not part
    /// of the Rust model).  It resolves each of the eight references at
    /// `CUnit+0xb0` and requires every resolved unit to be idle or hidden
    /// before clearing the array and its high-bit mask at `CUnit+0x132`.
    /// The modelled equivalent is the transport/Bunker cargo list: every
    /// passenger must satisfy [`SimUnit::is_idle_or_hidden`], after which the
    /// cargo is emptied and each passenger's transport/Bunker link cleared.
    /// Returns `false` on the aborting path (unknown carrier or a live,
    /// busy occupant).
    pub fn a29_release_cargo_slots(&mut self, id: u32) -> bool {
        let Some(i) = self.index_of(id) else {
            return false;
        };
        let all_idle = self.units[i]
            .cargo
            .iter()
            .all(|cid| self.unit(*cid).is_some_and(|u| u.is_idle_or_hidden()));
        if !all_idle {
            return false;
        }
        let cargo = std::mem::take(&mut self.units[i].cargo);
        for cid in cargo {
            if let Some(j) = self.index_of(cid) {
                self.units[j].in_transport = None;
                self.units[j].in_bunker = false;
            }
        }
        true
    }

    /// `0x005b0768`: Bunker removal entry.
    ///
    /// Only Bunker rows (`0x7d`) enter the slot-verification path; every
    /// other type takes the ordinary removal route, reported as `false`.
    /// For a Bunker the routine runs the unload pass, verifies slots `1..7`
    /// hold only idle or hidden units, then clears the reference array.  The
    /// model applies the recovered cargo rule from
    /// [`World::a29_release_cargo_slots`]; the skipped slot `0` has no Rust
    /// counterpart.
    pub fn a29_release_bunker_slots(&mut self, id: u32) -> bool {
        match self.unit(id) {
            Some(unit) if unit.type_id == crate::ids::unit::BUNKER => {}
            _ => return false,
        }
        self.a29_release_cargo_slots(id)
    }

    /// `0x005b0f40`: return a unit record to the pool free list.
    ///
    /// The routine decrements the live-unit counter `0x00fa36ec`, walks and
    /// clears the `CUnit+0x74` list, nulls the sprite pointer at
    /// `CUnit+0x0c`, unlinks the death list, and hands the record's one-based
    /// pool slot (`(record - pool) / 0x150 + 1`) to the free-list tree.
    /// [`World::units`] keeps a dense spawn-order vector and never removes a
    /// row, so the modelled release clears [`SimUnit::alive`] and returns
    /// that one-based slot.  `None` means the id is not in the world.
    pub fn a29_release_unit_slot(&mut self, id: u32) -> Option<u32> {
        let i = self.index_of(id)?;
        self.units[i].alive = false;
        Some(self.unit_slot_number(i))
    }

    /// `0x005b1db7` and `0x005b1dc1`: clamp a unit centre inside the map.
    ///
    /// The two addresses are overlapping decompilation entry points of one
    /// body; both perform the same arithmetic.  Rows at or above `0xe4`
    /// return the input unchanged.  Otherwise the `units.dat` box around the
    /// centre is kept on the map:
    ///
    /// `x in [left, map_width - 1 - right]`,
    /// `y in [top, map_height - 1 - bottom]`.
    ///
    /// Retail only reaches the clamp for rows whose binary placement table
    /// has bit 1 set; that table is not part of the bundled `GameData`, so
    /// the caller chooses the rows to clamp.
    pub fn a29_clamp_position_to_map(&self, type_id: u16, x: i32, y: i32) -> (i32, i32) {
        if type_id >= 0xe4 {
            return (x, y);
        }
        let d = self.data.unit(type_id).dimensions;
        let width = self.terrain.width as i32 * 32;
        let height = self.terrain.height as i32 * 32;
        let min_x = d.left as i32;
        let max_x = width - 1 - d.right as i32;
        let min_y = d.top as i32;
        let max_y = height - 1 - d.bottom as i32;
        (
            x.clamp(min_x, max_x.max(min_x)),
            y.clamp(min_y, max_y.max(min_y)),
        )
    }
}

#[cfg(test)]
mod tests_agent29 {
    use super::*;
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

    /// `0x005b0086`: the trap rows lose their normal visibility mask and the
    /// `CUnit+0x91` byte, while ordinary rows keep both.
    #[test]
    fn trap_removal_clears_trap_fields_only() {
        let mut w = world();
        let trap = w.spawn(0, 0xcb, 20, 20).unwrap();
        let marine = w.spawn(1, uid::MARINE, 40, 20).unwrap();
        for i in [trap, marine] {
            w.units[i].visibility_mask = u32::MAX;
            w.units[i].field_091 = 7;
        }

        assert!(w.units[trap].a29_finalize_trap_removal());
        assert_eq!(w.units[trap].visibility_mask, 0x8000_0000);
        assert_eq!(w.units[trap].field_091, 0);

        assert!(!w.units[marine].a29_finalize_trap_removal());
        assert_eq!(w.units[marine].visibility_mask, u32::MAX);
        assert_eq!(w.units[marine].field_091, 7);

        for type_id in [0xd1, 0xd2, 0xd3, 0xd4, 0xd5] {
            let u = w.spawn(0, type_id, 60, 20).unwrap();
            assert!(w.units[u].a29_finalize_trap_removal(), "{type_id:#x}");
        }
    }

    /// `0x005b0342`: table directions are shifted to sprite angles, the
    /// `0x20` sentinel consumes the random draw, and only trap rows report
    /// the re-insertion branch.
    #[test]
    fn initial_facing_uses_table_and_random_sentinel() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 20, 20).unwrap();
        assert!(!w.units[marine].a29_initial_facing(5, 0));
        assert_eq!(w.units[marine].facing, 40);

        let trap = w.spawn(0, 0xcb, 40, 20).unwrap();
        assert!(w.units[trap].a29_initial_facing(0x20, 0x25));
        assert_eq!(w.units[trap].facing, (0x25 & 0x1f) << 3);

        assert!(w.units[trap].a29_initial_facing(0x20, 0xff));
        assert_eq!(w.units[trap].facing, 0xf8);
    }

    /// `0x005b05a0`: hangar rows lose their interceptor/scarab count, the
    /// ghost heroes take the nuke route, and other rows take neither.
    #[test]
    fn removal_route_classifies_hangar_and_ghost_rows() {
        let mut w = world();
        let carrier = w.spawn(0, uid::CARRIER, 20, 20).unwrap();
        w.units[carrier].hangar = 4;
        assert_eq!(w.units[carrier].a29_apply_removal_route(), 1);
        assert_eq!(w.units[carrier].hangar, 0);

        let reaver = w.spawn(0, uid::REAVER, 40, 20).unwrap();
        w.units[reaver].hangar = 2;
        assert_eq!(w.units[reaver].a29_apply_removal_route(), 1);
        assert_eq!(w.units[reaver].hangar, 0);

        for type_id in [0x01, 0x10, 0x63, 0x64, 0x68] {
            let u = w.spawn(0, type_id, 60, 20).unwrap();
            assert_eq!(w.units[u].a29_apply_removal_route(), 2, "{type_id:#x}");
        }

        let marine = w.spawn(0, uid::MARINE, 80, 20).unwrap();
        assert_eq!(w.units[marine].a29_apply_removal_route(), 0);
    }

    /// `0x005b071f`: every cargo passenger must be idle or hidden before the
    /// slot array is cleared, and a busy passenger blocks the release.
    #[test]
    fn cargo_slots_release_only_idle_occupants() {
        let mut w = world();
        let dropship = w.spawn(0, uid::DROPSHIP, 20, 20).unwrap();
        let marine = w.spawn(0, uid::MARINE, 40, 20).unwrap();
        let transport_id = w.units[dropship].id;
        let marine_id = w.units[marine].id;
        w.units[dropship].cargo.push(marine_id);
        w.units[marine].in_transport = Some(transport_id);
        w.units[marine].alive = false;
        w.units[marine].order = Order::Idle;

        assert!(w.a29_release_cargo_slots(transport_id));
        assert!(w.units[dropship].cargo.is_empty());
        assert_eq!(w.units[marine].in_transport, None);
        assert!(!w.units[marine].in_bunker);

        let second = w.spawn(0, uid::DROPSHIP, 60, 20).unwrap();
        let busy = w.spawn(0, uid::MARINE, 80, 20).unwrap();
        let second_id = w.units[second].id;
        let busy_id = w.units[busy].id;
        w.units[second].cargo.push(busy_id);
        w.units[busy].order = Order::Move { pos: (100, 100) };
        assert!(!w.a29_release_cargo_slots(second_id));
        assert_eq!(w.units[second].cargo.len(), 1);

        assert!(!w.a29_release_cargo_slots(9999));
    }

    /// `0x005b0768`: only a Bunker takes the slot-release path.
    #[test]
    fn bunker_slots_require_the_bunker_type() {
        let mut w = world();
        let bunker = w.spawn(0, uid::BUNKER, 40, 40).unwrap();
        let marine = w.spawn(0, uid::MARINE, 60, 40).unwrap();
        let bunker_id = w.units[bunker].id;
        let marine_id = w.units[marine].id;
        w.units[bunker].cargo.push(marine_id);
        w.units[marine].in_bunker = true;
        w.units[marine].alive = false;
        w.units[marine].order = Order::Idle;

        assert!(w.a29_release_bunker_slots(bunker_id));
        assert!(w.units[bunker].cargo.is_empty());
        assert!(!w.units[marine].in_bunker);

        let marine_hull = w.spawn(0, uid::MARINE, 80, 40).unwrap();
        let other = w.spawn(0, uid::MARINE, 100, 40).unwrap();
        let hull_id = w.units[marine_hull].id;
        let other_id = w.units[other].id;
        w.units[marine_hull].cargo.push(other_id);
        w.units[other].alive = false;
        assert!(!w.a29_release_bunker_slots(hull_id));
        assert_eq!(w.units[marine_hull].cargo.len(), 1);
    }

    /// `0x005b07aa`/`0x005b07ad`: subunits only unlink, other rows clear the
    /// non-building `CUnit+0x96`, the AI slot, the target and the order row.
    #[test]
    fn subunit_aware_teardown_skips_subunits() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 20, 20).unwrap();
        w.units[marine].field_096 = 9;
        w.units[marine].field_100 = 0x1234;
        w.units[marine].target = Some(77);
        w.units[marine].order = Order::Move { pos: (1, 2) };
        w.units[marine].order_row = 0x0d;

        assert!(w.units[marine].a29_teardown_subunit_aware(&w.data));
        assert_eq!(w.units[marine].field_096, 0);
        assert_eq!(w.units[marine].field_100, 0);
        assert_eq!(w.units[marine].target, None);
        assert_eq!(w.units[marine].order, Order::Idle);
        assert_eq!(w.units[marine].order_row, 0);

        let turret = w.spawn(0, 0x18, 40, 20).unwrap();
        assert!(w.data.unit(0x18).is_subunit());
        w.units[turret].field_096 = 5;
        w.units[turret].field_100 = 9;
        w.units[turret].target = Some(88);
        w.units[turret].order = Order::Move { pos: (3, 4) };
        assert!(!w.units[turret].a29_teardown_subunit_aware_alt(&w.data));
        assert_eq!(w.units[turret].field_096, 5);
        assert_eq!(w.units[turret].field_100, 9);
        assert_eq!(w.units[turret].target, Some(88));
        assert_eq!(w.units[turret].order, Order::Move { pos: (3, 4) });
    }

    /// `0x005b0927`: the full entry reaches subunit rows, and the
    /// non-building field block still applies.
    #[test]
    fn full_teardown_reaches_subunits() {
        let mut w = world();
        let turret = w.spawn(0, 0x18, 20, 20).unwrap();
        w.units[turret].field_096 = 5;
        w.units[turret].field_100 = 9;
        w.units[turret].target = Some(88);
        w.units[turret].order = Order::Move { pos: (3, 4) };

        assert!(w.units[turret].a29_teardown_full(&w.data));
        assert_eq!(w.units[turret].field_096, 0);
        assert_eq!(w.units[turret].field_100, 0);
        assert_eq!(w.units[turret].target, None);
        assert_eq!(w.units[turret].order, Order::Idle);
    }

    /// `0x005b0978`: the tail entry clears the AI slot, target and order but
    /// leaves the non-building `CUnit+0x96` byte alone.
    #[test]
    fn tail_teardown_keeps_field_096() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 20, 20).unwrap();
        w.units[marine].field_096 = 7;
        w.units[marine].field_100 = 0x55;
        w.units[marine].target = Some(4);
        w.units[marine].order = Order::Stop;

        assert!(w.units[marine].a29_teardown_tail(&w.data));
        assert_eq!(w.units[marine].field_096, 7);
        assert_eq!(w.units[marine].field_100, 0);
        assert_eq!(w.units[marine].target, None);
        assert_eq!(w.units[marine].order, Order::Idle);
    }

    /// `0x005b0f40`: the released record leaves the live view and reports
    /// its one-based pool slot.
    #[test]
    fn release_unit_slot_marks_the_record_dead() {
        let mut w = world();
        let first = w.spawn(0, uid::MARINE, 20, 20).unwrap();
        let second = w.spawn(1, uid::MARINE, 40, 20).unwrap();
        let second_id = w.units[second].id;

        assert_eq!(w.a29_release_unit_slot(second_id), Some(2));
        assert!(!w.units[second].alive);
        assert!(w.units[first].alive);
        assert_eq!(w.a29_release_unit_slot(9999), None);
        assert_eq!(w.a29_release_unit_slot(second_id), Some(2));
    }

    /// `0x005b11e8`: recreation restores three Spider Mines for the Vulture
    /// rows and gives every row an all-visible mask.
    #[test]
    fn respawned_vulture_gets_three_mines() {
        let mut w = world();
        let vulture = w.spawn(0, uid::VULTURE, 20, 20).unwrap();
        w.units[vulture].hangar = 0;
        w.units[vulture].visibility_mask = 0;
        assert!(w.units[vulture].a29_restore_spawned_state());
        assert_eq!(w.units[vulture].hangar, 3);
        assert_eq!(w.units[vulture].visibility_mask, u32::MAX);

        let hero = w.spawn(0, 0x13, 40, 20).unwrap();
        assert!(w.units[hero].a29_restore_spawned_state());
        assert_eq!(w.units[hero].hangar, 3);

        let marine = w.spawn(0, uid::MARINE, 60, 20).unwrap();
        w.units[marine].hangar = 0;
        w.units[marine].visibility_mask = 0;
        assert!(!w.units[marine].a29_restore_spawned_state());
        assert_eq!(w.units[marine].hangar, 0);
        assert_eq!(w.units[marine].visibility_mask, u32::MAX);
    }

    /// `0x005b11fd`: the all-visible reset reports the subunit dispatch.
    #[test]
    fn visibility_reset_reports_subunit_rows() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 20, 20).unwrap();
        w.units[marine].visibility_mask = 0;
        assert!(!w.units[marine].a29_reset_visibility_mask(&w.data));
        assert_eq!(w.units[marine].visibility_mask, u32::MAX);

        let turret = w.spawn(0, 0x18, 40, 20).unwrap();
        w.units[turret].visibility_mask = 0;
        assert!(w.units[turret].a29_reset_visibility_mask(&w.data));
        assert_eq!(w.units[turret].visibility_mask, u32::MAX);
    }

    /// `0x005b1400`: a blanked record gets the new type, the constant
    /// `field_090 = 8`, and no order/target/queue state.
    #[test]
    fn reinitialize_record_blanks_the_record() {
        let mut w = world();
        let barracks = w.spawn(0, uid::BARRACKS, 40, 40).unwrap();
        w.units[barracks].target = Some(3);
        w.units[barracks].order = Order::Train {
            unit_type: uid::MARINE,
        };
        w.units[barracks].queue.push_back(Order::Stop);
        w.units[barracks].overlay_flags = 0x12;
        w.units[barracks].field_091 = 5;
        w.units[barracks].field_090 = 0;

        let returned = w.units[barracks].a29_reinitialize_record(uid::MARINE);
        assert_eq!(returned, uid::MARINE);
        assert_eq!(w.units[barracks].type_id, uid::MARINE);
        assert_eq!(w.units[barracks].field_090, 8);
        assert_eq!(w.units[barracks].field_091, 0);
        assert_eq!(w.units[barracks].overlay_flags, 0);
        assert_eq!(w.units[barracks].target, None);
        assert_eq!(w.units[barracks].order, Order::Idle);
        assert!(w.units[barracks].queue.is_empty());
    }

    /// `0x005b1db7`/`0x005b1dc1`: the unit box is clamped to the map, rows
    /// above `0xe3` are returned unchanged.
    #[test]
    fn position_clamp_uses_the_unit_box() {
        let w = world();
        let d = w.data.unit(uid::BARRACKS).dimensions;
        let width = w.terrain.width as i32 * 32;
        let height = w.terrain.height as i32 * 32;

        assert_eq!(
            w.a29_clamp_position_to_map(uid::BARRACKS, -100, -100),
            (d.left as i32, d.top as i32)
        );
        assert_eq!(
            w.a29_clamp_position_to_map(uid::BARRACKS, 5000, 5000),
            (width - 1 - d.right as i32, height - 1 - d.bottom as i32)
        );
        assert_eq!(
            w.a29_clamp_position_to_map(uid::BARRACKS, 1000, 900),
            (1000, 900)
        );
        assert_eq!(w.a29_clamp_position_to_map(0xe4, -100, -100), (-100, -100));
    }
}
