#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent30.rs
// ===========================================================================
// Translation batch 30.  Only this file is edited by the batch.
//
// The `decompiled_uncited` rows in `0x005b2000..=0x005b3fff` are a mixed
// `CUnit` / command-card / container-helper cluster.  Most of the 2xxx
// rows are retail dialog callbacks that only touch sprite and window
// objects, and several 3xxx rows are one-byte flag getters already covered
// by `sim.rs`.  The methods below keep the record-level behaviour that the
// existing [`World`] and [`SimUnit`] fields can represent; presentation
// work (sound cues, sprite vertices, command-card refreshes) is not stored
// in the simulation and is called out in each doc comment instead of being
// invented as new state.

/// Number of per-owner unit lists addressed by the 12-bit mask in the
/// reference-cleanup routines (retail supports twelve players).
const A30_OWNER_LISTS: u8 = 12;

/// The unit target carried by an order, if any.  This is the order-level
/// view of `CUnit+0x5c`; the helper is used to scrub queue entries that
/// still point at a removed unit.
fn a30_order_target(order: &Order) -> Option<u32> {
    match order {
        Order::AttackUnit { target }
        | Order::Gather { resource: target }
        | Order::ReturnCargo { depot: target }
        | Order::Repair { target }
        | Order::Construct { building: target }
        | Order::EnterTransport { transport: target } => Some(*target),
        Order::Cast {
            target: crate::sim::SpellTarget::Unit(id),
            ..
        } => Some(*id),
        _ => None,
    }
}

/// Clear every modelled pointer in `unit` that names `dead`.  Returns true
/// when at least one field changed, matching the per-record effect of the
/// retail reference sweep.  The retail record also owns order-list nodes at
/// `CUnit+0x74`; the typed `queue`/`production` deques stand in for those
/// nodes, so entries that target the removed unit are dropped in place.
fn a30_clear_unit_reference(unit: &mut SimUnit, dead: u32) -> bool {
    let mut changed = false;
    if unit.target == Some(dead) {
        unit.target = None;
        changed = true;
    }
    if unit.order_unit_target() == Some(dead) {
        unit.clear_order_target(dead);
        changed = true;
    }
    if unit.resume_order.as_ref().and_then(a30_order_target) == Some(dead) {
        unit.resume_order = None;
        changed = true;
    }
    let queue_len = unit.queue.len();
    unit.queue
        .retain(|order| a30_order_target(order) != Some(dead));
    if unit.queue.len() != queue_len {
        changed = true;
    }
    let production_len = unit.production.len();
    unit.production
        .retain(|order| a30_order_target(order) != Some(dead));
    if unit.production.len() != production_len {
        changed = true;
    }
    if unit.connected_unit == Some(dead) {
        unit.connected_unit = None;
        changed = true;
    }
    if unit.addon == Some(dead) {
        unit.addon = None;
        changed = true;
    }
    if unit.builder == dead {
        unit.builder = 0;
        changed = true;
    }
    if unit.resource_target == Some(dead) {
        unit.resource_target = None;
        changed = true;
    }
    if unit.depot == Some(dead) {
        unit.depot = None;
        changed = true;
    }
    if unit.in_transport == Some(dead) {
        unit.in_transport = None;
        changed = true;
    }
    if unit.nydus_exit == Some(dead) {
        unit.nydus_exit = None;
        changed = true;
    }
    if unit.last_damage_source == dead {
        unit.last_damage_source = 0;
        changed = true;
    }
    let cargo_len = unit.cargo.len();
    unit.cargo.retain(|&id| id != dead);
    if unit.cargo.len() != cargo_len {
        changed = true;
    }
    changed
}

impl World {
    /// `0x005b242b`: set the hit points of the linked unit to the raw
    /// maximum of a given type.
    ///
    /// The recovered routine writes the `units.dat` maximum-HP entry of the
    /// low 16-bit argument into `CUnit+0x8` of the record at `CUnit+0x70`.
    /// `CUnit+0x8` is the current hit-point pool in 1/256 points (the same
    /// field that starts construction at ten percent in the build-time
    /// initialiser), so the linked record becomes full-health for that type.
    /// The `CUnit+0x70` subunit pointer is not stored by [`SimUnit`]; the
    /// linked record is therefore identified by id.  Returns `false` when
    /// the id is not a live record of this world.
    pub fn a30_set_linked_unit_hp(&mut self, linked_id: u32, type_id: u16) -> bool {
        let Some(i) = self.index_of(linked_id) else {
            return false;
        };
        self.units[i].hp = self.data.unit(type_id).hit_points_raw as i32;
        true
    }

    /// `0x005b2800`: removal-time sweep that clears every reference to a
    /// gone unit.
    ///
    /// The retail body walks the unit lists and the order-list nodes of
    /// every record, clearing the link slots that compare equal to `this`
    /// (order target, unit target, hatchery link, research/building links,
    /// queue nodes) before the record is freed.  The typed model keeps the
    /// same references in [`SimUnit::target`], [`SimUnit::order`] and the
    /// other optional id fields, so the sweep translates directly to
    /// [`a30_clear_unit_reference`].  Global links that name the unit (the
    /// staged nuke strike, CTF flag carrier/beacon slots and rescue list)
    /// are cleaned here as well, mirroring the caller-side cleanup in the
    /// death path.
    ///
    /// Returns the number of unit records whose links changed.
    pub fn a30_purge_unit_references(&mut self, dead_id: u32) -> usize {
        let mut changed = 0;
        for unit in &mut self.units {
            if unit.id == dead_id {
                continue;
            }
            if a30_clear_unit_reference(unit, dead_id) {
                changed += 1;
            }
        }
        self.nuke_strikes
            .retain(|strike| strike.ghost != dead_id && strike.silo != dead_id);
        if let Some(ctf) = &mut self.ctf {
            for flag in &mut ctf.flags {
                if flag.carrier == Some(dead_id) {
                    flag.carrier = None;
                    flag.released = true;
                }
            }
            for beacon in &mut ctf.beacons {
                if *beacon == Some(dead_id) {
                    *beacon = None;
                }
            }
        }
        self.rescues.retain(|(id, _)| *id != dead_id);
        changed
    }

    /// `0x005b29c0`: scoped reference sweep driven by a 12-bit owner mask.
    ///
    /// Retail keeps one unit list per player and processes only the lists
    /// whose bit is set in `param_1 & 0xfff`.  Each visited record runs the
    /// same link cleanup as the full sweep, including its recursively linked
    /// subunit chain.  The Rust pool is a single vector, so the mask is
    /// applied to [`SimUnit::owner`] and each matching record receives the
    /// modelled cleanup.  Nuke strikes are filtered by their launching
    /// player's bit.
    ///
    /// Returns the number of unit records whose links changed.
    pub fn a30_purge_unit_references_for_owners(&mut self, dead_id: u32, owner_mask: u16) -> usize {
        let mask = owner_mask & 0x0fff;
        let mut changed = 0;
        for unit in &mut self.units {
            if unit.id == dead_id || unit.owner >= A30_OWNER_LISTS {
                continue;
            }
            if mask & (1u16 << unit.owner) == 0 {
                continue;
            }
            if a30_clear_unit_reference(unit, dead_id) {
                changed += 1;
            }
        }
        self.nuke_strikes.retain(|strike| {
            strike.owner >= A30_OWNER_LISTS
                || mask & (1u16 << strike.owner) == 0
                || (strike.ghost != dead_id && strike.silo != dead_id)
        });
        changed
    }

    /// `0x005b2f20`: resize an eight-byte-element buffer in place.
    ///
    /// The recovered helper is the MSVC `std::vector` resize path used for a
    /// buffer of eight-byte records: a smaller length erases the tail (the
    /// routine calls the move/destroy helper), a larger length zero-fills the
    /// new slots up to the (possibly grown) capacity, and the new length is
    /// stored.  [`World::nuke_warnings`] is the model's public eight-byte
    /// pair buffer, so the same truncate/zero-fill rule is applied there.
    /// Returns the resulting length.
    pub fn a30_resize_pair_buffer(&mut self, len: usize) -> usize {
        self.nuke_warnings.resize(len, (0, 0));
        self.nuke_warnings.len()
    }

    /// `0x005b2710`: first zero slot in a twelve-dword table.
    ///
    /// The decompiled body unrolls the scan in blocks of six and returns the
    /// index of the first zero entry, or twelve when every slot is occupied.
    /// The table itself is a file-static array, which the simulation does not
    /// mirror, so the recovered slot search is exposed as a pure helper over
    /// the caller's twelve-slot buffer.  `None` is the all-occupied result.
    pub fn a30_first_free_slot(&self, slots: &[u32; A30_OWNER_LISTS as usize]) -> Option<usize> {
        slots.iter().position(|&slot| slot == 0)
    }

    /// `0x005b27b1`: erase one slot from a twelve-entry table.
    ///
    /// The recovered body is a `memmove` of `(len - 1 - index)` dwords from
    /// `index + 1` down to `index`; the old last entry is deliberately left
    /// untouched and the caller owns the length decrement.  With the table
    /// not held by [`World`], the same tail shift is applied to the supplied
    /// buffer.  Out-of-range indexes and lengths are ignored.
    pub fn a30_shift_slot_left(
        &self,
        slots: &mut [u32; A30_OWNER_LISTS as usize],
        len: usize,
        index: usize,
    ) {
        let len = len.min(A30_OWNER_LISTS as usize);
        if index >= len {
            return;
        }
        slots.copy_within(index + 1..len, index);
    }

    /// `0x005b36c0`: deposit the carried amount into the owner's economy and
    /// score.
    ///
    /// The routine reads the amount from `CUnit+0xcf` (the companion byte
    /// written by the overlay setter) and the carried kind from the two
    /// `CUnit+0xe0` overlay bits: bit 1 means gas and bit 0 means minerals.
    /// A set bit credits the owner's current stock and its gathered total
    /// through the two-argument per-player score helper; the always-run tail
    /// adds the same amount to score column 3 (resources) before clearing
    /// both carrying bytes.  With neither bit set only the score column is
    /// updated, exactly as the recovered control flow falls through.
    ///
    /// Returns whether a mineral or gas branch was taken.
    pub fn a30_credit_gathered_resources(&mut self, i: usize) -> bool {
        if i >= self.units.len() {
            return false;
        }
        let owner = self.units[i].owner as usize;
        let amount = i32::from(self.units[i].field_0cf);
        let gas = self.units[i].overlay_flags & 2 != 0;
        let minerals = !gas && self.units[i].overlay_flags & 1 != 0;
        if owner < self.players.len() {
            if gas {
                self.players[owner].gas += amount;
                self.players[owner].gas_gathered += amount;
            } else if minerals {
                self.players[owner].minerals += amount;
                self.players[owner].minerals_gathered += amount;
            }
            self.players[owner].scores[3] += amount;
        }
        self.units[i].overlay_flags = 0;
        self.units[i].field_0cf = 0;
        gas || minerals
    }

    /// `0x005b3b20`: drop the unit's `CUnit+0x5c` order target.
    ///
    /// The recovered routine unconditionally zeroes the order's unit-target
    /// slot, then follows `CUnit+0x70`: when that link exists and carries
    /// the `units.dat` subunit flag `0x10`, the linked record's target slot
    /// is cleared too.  `CUnit+0x70` is not stored by [`SimUnit`], so the
    /// subunit record is supplied by id (the same convention used by the
    /// other recovered subunit helpers).  Only the unit-target slot is
    /// cleared; the independent `CUnit+0x7c` attack target is left alone.
    ///
    /// Returns true when the calling unit itself had a unit-target link.
    pub fn a30_clear_order_link(&mut self, unit_id: u32, subunit_id: Option<u32>) -> bool {
        let Some(i) = self.index_of(unit_id) else {
            return false;
        };
        let linked = self.units[i].order_unit_target();
        if let Some(target) = linked {
            self.units[i].clear_order_target(target);
        }
        if let Some(sid) = subunit_id
            && let Some(j) = self.index_of(sid)
        {
            let is_subunit = self.data.unit(self.units[j].type_id).is_subunit();
            if is_subunit && let Some(target) = self.units[j].order_unit_target() {
                self.units[j].clear_order_target(target);
            }
        }
        linked.is_some()
    }
}

impl SimUnit {
    /// `0x005b24b6`: restore a resource-container record to its full amount.
    ///
    /// The only accepted non-aborting path is the Vespene Geyser row (type
    /// `0xbc`, [`uid::VESPENE_GEYSER`]): the routine stores the default gas
    /// amount `5000` into the container's `CUnit+0xd0` word.  The other
    /// decompiled branches call the fatal helper, so any other type reports
    /// `false` without changing the record.  The existing
    /// [`SimUnit::resource_amount`] field holds the same amount.
    pub fn a30_reset_resource_amount(&mut self) -> bool {
        if self.type_id != uid::VESPENE_GEYSER {
            return false;
        }
        self.resource_amount = GAS_CAPACITY;
        true
    }

    /// `0x005b2570`: cached whole-point hit points for the status display.
    ///
    /// The routine copies the type id from `CUnit+0x64` to the cached
    /// `CUnit+0x88`, then converts the current fixed-point HP at `CUnit+0x8`
    /// to whole points with an arithmetic right shift by eight and stores
    /// the result in `CUnit+0xae`.  A zero result is clamped to one; a
    /// negative value is preserved as the signed short the retail store
    /// performs.  [`SimUnit`] already keeps the authoritative
    /// [`SimUnit::type_id`] and [`SimUnit::hp`], so the method exposes the
    /// recovered cached value instead of adding a second copy.
    pub fn a30_cached_whole_hp(&self) -> i16 {
        let whole = (self.hp >> 8) as i16;
        if whole == 0 { 1 } else { whole }
    }
}

#[cfg(test)]
mod tests_agent30 {
    use super::*;
    use crate::data::GameData;
    use crate::map::Terrain;
    use crate::sim::NukeStrike;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    /// `0x005b242b`: the linked record receives the raw maximum HP of the
    /// supplied type, and an unknown id is rejected.
    #[test]
    fn linked_unit_gets_full_hit_points() {
        let mut w = world();
        let linked = w.spawn(0, uid::MARINE, 12, 12).unwrap();
        let linked_id = w.units[linked].id;
        w.units[linked].hp = 1;

        let expected = w.data.unit(uid::BARRACKS).hit_points_raw as i32;
        assert!(w.a30_set_linked_unit_hp(linked_id, uid::BARRACKS));
        assert_eq!(w.units[linked].hp, expected);

        assert!(!w.a30_set_linked_unit_hp(0xdead_beef, uid::MARINE));
    }

    /// `0x005b24b6`: only the geyser row takes the full-amount branch; other
    /// types are the fatal path and are left untouched.
    #[test]
    fn geyser_resource_amount_is_restored_to_five_thousand() {
        let mut w = world();
        let geyser = w.spawn(0, uid::VESPENE_GEYSER, 20, 20).unwrap();
        w.units[geyser].resource_amount = 17;

        assert!(w.units[geyser].a30_reset_resource_amount());
        assert_eq!(w.units[geyser].resource_amount, 5000);

        let marine = w.spawn(0, uid::MARINE, 24, 24).unwrap();
        w.units[marine].resource_amount = 9;
        assert!(!w.units[marine].a30_reset_resource_amount());
        assert_eq!(w.units[marine].resource_amount, 9);
    }

    /// `0x005b2570`: whole HP is `hp >> 8` with the zero result clamped to
    /// one; the signed conversion is preserved.
    #[test]
    fn cached_whole_hp_matches_shift_and_clamp() {
        let mut w = world();
        let u = w.spawn(0, uid::MARINE, 10, 10).unwrap();

        w.units[u].hp = 0;
        assert_eq!(w.units[u].a30_cached_whole_hp(), 1);

        w.units[u].hp = 0xff;
        assert_eq!(w.units[u].a30_cached_whole_hp(), 1);

        w.units[u].hp = 0x100;
        assert_eq!(w.units[u].a30_cached_whole_hp(), 1);

        w.units[u].hp = 400 << 8;
        assert_eq!(w.units[u].a30_cached_whole_hp(), 400);

        // The retail store is a signed short, so a negative fixed-point
        // value stays negative instead of being clamped.
        w.units[u].hp = -0x100;
        assert_eq!(w.units[u].a30_cached_whole_hp(), -1);
    }

    /// `0x005b2800`: every modelled link to the removed record is cleared,
    /// including queue entries and the staged nuke strike.
    #[test]
    fn full_reference_sweep_clears_all_links() {
        let mut w = world();
        let victim = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let victim_id = w.units[victim].id;
        let a = w.spawn(0, uid::MARINE, 12, 10).unwrap();
        let b = w.spawn(1, uid::SCV, 14, 10).unwrap();

        for i in [a, b] {
            w.units[i].target = Some(victim_id);
            w.units[i].order = Order::AttackUnit { target: victim_id };
            w.units[i]
                .queue
                .push_back(Order::Repair { target: victim_id });
            w.units[i].connected_unit = Some(victim_id);
            w.units[i].addon = Some(victim_id);
            w.units[i].builder = victim_id;
            w.units[i].resource_target = Some(victim_id);
            w.units[i].depot = Some(victim_id);
            w.units[i].cargo.push(victim_id);
            w.units[i].last_damage_source = victim_id;
        }
        w.units[b].cargo.push(123);
        w.nuke_strikes.push(NukeStrike {
            owner: 0,
            ghost: victim_id,
            silo: 0,
            target: (10, 10),
            timer: 5,
            dot: true,
            warned: false,
            missile: 0,
        });

        assert_eq!(w.a30_purge_unit_references(victim_id), 2);

        for i in [a, b] {
            assert!(w.units[i].target.is_none());
            assert!(matches!(w.units[i].order, Order::Idle));
            assert!(w.units[i].queue.is_empty());
            assert!(w.units[i].connected_unit.is_none());
            assert!(w.units[i].addon.is_none());
            assert_eq!(w.units[i].builder, 0);
            assert!(w.units[i].resource_target.is_none());
            assert!(w.units[i].depot.is_none());
            assert!(w.units[i].last_damage_source == 0);
        }
        assert!(w.units[a].cargo.is_empty());
        assert_eq!(w.units[b].cargo, vec![123]);
        assert!(w.nuke_strikes.is_empty());
    }

    /// `0x005b29c0`: the twelve-bit mask limits the sweep to the selected
    /// owners' lists.
    #[test]
    fn masked_reference_sweep_honours_owner_bits() {
        let mut w = world();
        let victim = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let victim_id = w.units[victim].id;
        let own = w.spawn(0, uid::MARINE, 12, 10).unwrap();
        let other = w.spawn(1, uid::SCV, 14, 10).unwrap();

        w.units[own].target = Some(victim_id);
        w.units[other].target = Some(victim_id);
        w.nuke_strikes.push(NukeStrike {
            owner: 1,
            ghost: victim_id,
            silo: 0,
            target: (10, 10),
            timer: 5,
            dot: true,
            warned: false,
            missile: 0,
        });

        // Only owner 0's list: the strike belongs to owner 1 and survives.
        assert_eq!(w.a30_purge_unit_references_for_owners(victim_id, 0b01), 1);
        assert!(w.units[own].target.is_none());
        assert_eq!(w.units[other].target, Some(victim_id));
        assert_eq!(w.nuke_strikes.len(), 1);

        // Owner 1's list plus the strike.
        assert_eq!(w.a30_purge_unit_references_for_owners(victim_id, 0b10), 1);
        assert!(w.units[other].target.is_none());
        assert!(w.nuke_strikes.is_empty());
    }

    /// `0x005b2f20`: growing zero-fills, shrinking keeps the prefix.
    #[test]
    fn pair_buffer_resize_truncates_and_zero_fills() {
        let mut w = world();
        assert_eq!(w.a30_resize_pair_buffer(3), 3);
        assert_eq!(w.nuke_warnings, vec![(0, 0), (0, 0), (0, 0)]);

        w.nuke_warnings[0] = (7, 9);
        w.nuke_warnings[1] = (1, 2);
        assert_eq!(w.a30_resize_pair_buffer(1), 1);
        assert_eq!(w.nuke_warnings, vec![(7, 9)]);

        assert_eq!(w.a30_resize_pair_buffer(2), 2);
        assert_eq!(w.nuke_warnings, vec![(7, 9), (0, 0)]);
    }

    /// `0x005b2710`: the first zero slot wins; an all-occupied table has no
    /// free slot.
    #[test]
    fn first_free_slot_scans_twelve_entries() {
        let w = world();
        let mut slots = [3u32; 12];
        slots[0] = 0;
        assert_eq!(w.a30_first_free_slot(&slots), Some(0));

        slots[0] = 3;
        slots[7] = 0;
        assert_eq!(w.a30_first_free_slot(&slots), Some(7));

        slots[7] = 3;
        assert_eq!(w.a30_first_free_slot(&slots), None);
    }

    /// `0x005b27b1`: erasing a slot shifts the tail down and leaves the
    /// stale last element in place for the caller's length update.
    #[test]
    fn slot_removal_shifts_tail_left() {
        let w = world();
        let mut slots = [0u32; 12];
        for (i, slot) in slots.iter_mut().enumerate() {
            *slot = (i + 1) as u32;
        }

        w.a30_shift_slot_left(&mut slots, 5, 2);
        assert_eq!(slots[..5], [1, 2, 4, 5, 5]);
        assert_eq!(slots[5], 6);

        // Out-of-range operations leave the buffer alone.
        let before = slots;
        w.a30_shift_slot_left(&mut slots, 5, 5);
        w.a30_shift_slot_left(&mut slots, 99, 20);
        assert_eq!(slots, before);
    }

    /// `0x005b36c0`: the overlay bits select minerals versus gas, and the
    /// score column is always credited.
    #[test]
    fn carried_resources_credit_stock_totals_and_score() {
        let mut w = world();
        let scv = w.spawn(0, uid::SCV, 10, 10).unwrap();

        let minerals = w.players[0].minerals;
        let gathered = w.players[0].minerals_gathered;
        w.units[scv].set_overlay_state(false, 8);
        assert!(w.a30_credit_gathered_resources(scv));
        assert_eq!(w.players[0].minerals, minerals + 8);
        assert_eq!(w.players[0].minerals_gathered, gathered + 8);
        assert_eq!(w.players[0].scores[3], 8);
        assert_eq!(w.units[scv].overlay_flags, 0);
        assert_eq!(w.units[scv].field_0cf, 0);

        let gas = w.players[0].gas;
        let gas_gathered = w.players[0].gas_gathered;
        w.units[scv].set_overlay_state(true, 5);
        assert!(w.a30_credit_gathered_resources(scv));
        assert_eq!(w.players[0].gas, gas + 5);
        assert_eq!(w.players[0].gas_gathered, gas_gathered + 5);
        assert_eq!(w.players[0].scores[3], 13);

        // No kind bit: only the score column moves.
        w.units[scv].overlay_flags = 0;
        w.units[scv].field_0cf = 4;
        assert!(!w.a30_credit_gathered_resources(scv));
        assert_eq!(w.players[0].scores[3], 17);
        assert_eq!(w.units[scv].field_0cf, 0);
    }

    /// `0x005b3b20`: the caller's order target is dropped, and the separate
    /// subunit link is only cleared when its type carries the subunit flag.
    #[test]
    fn order_link_cleanup_follows_subunit_flag() {
        let mut w = world();
        let scv = w.spawn(0, uid::SCV, 10, 10).unwrap();
        let mineral = w.spawn(0, uid::MINERAL_FIELD, 20, 10).unwrap();
        let target = w.units[mineral].id;
        w.units[scv].order = Order::Gather { resource: target };

        let turret = w.spawn(0, uid::GOLIATH_TURRET, 12, 12).unwrap();
        let turret_id = w.units[turret].id;
        w.units[turret].order = Order::AttackUnit {
            target: w.units[mineral].id,
        };
        assert!(w.data.unit(uid::GOLIATH_TURRET).is_subunit());

        assert!(w.a30_clear_order_link(w.units[scv].id, Some(turret_id)));
        assert!(matches!(w.units[scv].order, Order::Idle));
        assert!(matches!(w.units[turret].order, Order::Idle));

        // A non-subunit linked record is left alone.
        let marine = w.spawn(0, uid::MARINE, 14, 12).unwrap();
        let marine_id = w.units[marine].id;
        w.units[marine].order = Order::AttackUnit {
            target: w.units[mineral].id,
        };
        w.units[scv].order = Order::Gather { resource: target };
        assert!(w.a30_clear_order_link(w.units[scv].id, Some(marine_id)));
        assert!(matches!(w.units[marine].order, Order::AttackUnit { .. }));

        // No link left on the caller.
        assert!(!w.a30_clear_order_link(w.units[scv].id, None));
    }
}
