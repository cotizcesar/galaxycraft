#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent09.rs
// ===========================================================================
// Translation batch 09.  Only this file is edited by the batch.
//
// Ten `decompiled_uncited` routines from `0x005c8000..=0x005cffff`.  Every
// item below names its exact decompiled address.  Helper routines whose
// meaning was already recovered in `sim.rs` are called instead of being
// re-derived: `FUN_005b5820` (building flag), `FUN_005b4b70` (`CUnit+0x123`),
// `FUN_005b4bc0` (`CUnit+0x121`), `FUN_005b6630` (Maelstrom timer),
// `FUN_005b6b20` (energy flag), `FUN_005b5d30` (hero flag), `FUN_005b4aa0`
// (HP + shields in whole points), `FUN_005b4a20` (`CUnit+0x60 >> 8`),
// `FUN_005b5890` (Carrier/Reaver family) and `FUN_005692b0` (byte facing).

impl World {
    /// `0x005c8300`: target-value gate.  The retail routine first calls the
    /// `FUN_005c8ad0` assertion helper, which only returns on the normal path
    /// when the two units are on the same side (every other path is fatal),
    /// so the alliance check is modelled as the `allied` precondition.
    ///
    /// It then returns false for buildings (`FUN_005b5820`) and for units with
    /// a non-empty `CUnit+0x123` mask (`FUN_005b4b70`).  Otherwise it asks
    /// `FUN_005b5a50` whether the unit is a detector; a non-detector whose
    /// whole-point HP plus shields (`FUN_005b4aa0`) is below `0x51` (81) fails
    /// the gate, everything else passes.  The data-table detector bit is
    /// [`crate::data::UnitType::is_detector`].
    pub fn a09_value_target_gate(&self, i: usize, allied: bool) -> bool {
        if !allied {
            return false;
        }
        let u = &self.units[i];
        if self.unit_is_building_flagged(u.type_id) || u.mask_123() != 0 {
            return false;
        }
        let detector = self.data.unit(u.type_id).is_detector();
        detector || self.unit_current_total_points(i) >= 0x51
    }

    /// `0x005c88f0`: whether a caster's energy covers the target's whole-point
    /// HP plus shields, the low-health recharge/heal pre-check.
    ///
    /// `FUN_005c8ad0` is the same-side assertion as in
    /// [`World::a09_value_target_gate`] (modelled by `allied`).  The routine
    /// then requires the energy flag (`FUN_005b6b20`), the clear state of
    /// `CUnit+0xdc` bit 1 (`FUN_005b5810`, not separately modelled here and
    /// passed as `status_bit1`) and a non-hero row (`FUN_005b5d30`).  Finally
    /// the whole-point target total (`FUN_005b4aa0`) is compared with the low
    /// byte of `CUnit+0xa3`, i.e. the fixed-point energy `>> 8`
    /// (`FUN_005b4af0`).
    pub fn a09_energy_covers_heal(&self, i: usize, allied: bool, status_bit1: bool) -> bool {
        if !allied || status_bit1 {
            return false;
        }
        if !self.unit_has_energy(i) {
            return false;
        }
        let u = &self.units[i];
        if self.data.unit(u.type_id).is_hero() {
            return false;
        }
        let energy_whole = (u.energy_fixed() >> 8) & 0xff;
        self.unit_current_total_points(i) <= energy_whole
    }

    /// `0x005c8950`: true when `i` and the unit owned by `other_owner` share
    /// an owner, `i` carries an order unit target in `CUnit+0x5c`
    /// ([`SimUnit::order_unit_target`]), `i` is a hangar-family carrier or
    /// reaver (`FUN_005b5890`) and its whole-point HP (`FUN_005b4a20`,
    /// `CUnit+0x60 >> 8`) is below 11.
    pub fn a09_carrier_has_low_hp_target(&self, i: usize, other_owner: u8) -> bool {
        let u = &self.units[i];
        u.owner == other_owner
            && u.order_unit_target().is_some()
            && is_carrier_reaver_family(u.type_id)
            && (u.hp >> 8) < 0xb
    }
}

impl SimUnit {
    /// `0x005c8620`: recharge/recast status gate.  False when the owners in
    /// `CUnit+0x4c` differ.  Otherwise the implicit unit must have either a
    /// non-empty `CUnit+0x121` mask (`FUN_005b4bc0`) or a running Maelstrom
    /// timer (`FUN_005b6630`), and the target predicate
    /// `FUN_0059ca70(0x66, unit)` must accept the unit; that data-table
    /// predicate is supplied by the caller as `targetable`.
    pub fn a09_recharge_status_gate(&self, other: &SimUnit, targetable: bool) -> bool {
        self.owner == other.owner
            && (self.mask_121() != 0 || self.maelstrom_ticks() != 0)
            && targetable
    }

    /// `0x005cf1e5`: true when the unit's current byte facing is within five
    /// steps (of 256) of `wanted`.  `FUN_005692b0` computes the current byte
    /// facing from the unit's position; the result is folded into
    /// `0..=128` and compared with 6, so 0..=5 passes and 6..=128 fails.
    pub fn a09_facing_near(&self, wanted: u8) -> bool {
        let delta = wanted.wrapping_sub(self.facing) as u16;
        delta.min(0x100 - delta) < 6
    }
}

/// `0x005c8a54`: accumulator gate.  `FUN_006c1ee0` is called first for its
/// side effect; the routine then returns 0 while the force flag
/// `DAT_00fa3900` is clear and the accumulator `DAT_00fa38f8` is below
/// `0xfa` (250).  Once either condition holds it stores zero in
/// `DAT_00fa38fc` and tail-calls `FUN_0058b650`, i.e. it reports success and
/// restarts the accumulator.  Returns `(ready, next_counter_value)`.
pub fn a09_accumulator_gate(flag: bool, counter: u32) -> (bool, u32) {
    let ready = flag || counter >= 0xfa;
    (ready, if ready { 0 } else { counter })
}

/// `0x005c8cd0`: checked lookup of the value slot at node offset `0x14`.
/// Retail calls `FUN_00597430` with the key and aborts through `FUN_0093ec70`
/// when the lookup reports no node, so there is no default value; here a
/// miss is `None`.  `0x005c9350` stores a `u16` through this slot, which
/// fixes the value type.
pub fn a09_map_value_u16(map: &BTreeMap<i32, u16>, key: i32) -> Option<u16> {
    map.get(&key).copied()
}

/// `0x005c9350`: write a `u16` through the value slot returned by
/// [`a09_map_value_u16`] (`0x005c8cd0`).  Because the lookup aborts on a
/// missing key, this only updates an existing entry; `false` reports the
/// retail abort path and leaves the map unchanged.
pub fn a09_map_store_u16(map: &mut BTreeMap<i32, u16>, key: i32, value: u16) -> bool {
    match map.get_mut(&key) {
        Some(slot) => {
            *slot = value;
            true
        }
        None => false,
    }
}

/// `0x005c8ee0`: exact lookup in an ordered tree whose key is the unsigned
/// dword at node offset `0x10` and whose value is at `0x14`.  The walk is a
/// lower bound, and the caller keeps the node only when its key is also
/// `<= query`, so only an exact match is returned.  `keys` must be in
/// ascending order, as they are in the tree.
pub fn a09_unsigned_key_index(keys: &[u32], key: u32) -> Option<usize> {
    let index = keys.partition_point(|&k| k < key);
    (keys.get(index) == Some(&key)).then_some(index)
}

/// `0x005c8f20`: signed-key twin of [`a09_unsigned_key_index`].  The tree
/// walk and the final `key <= query` check are the same; only the key
/// comparisons are signed.  `keys` must be in ascending order.
pub fn a09_signed_key_index(keys: &[i32], key: i32) -> Option<usize> {
    let index = keys.partition_point(|&k| k < key);
    (keys.get(index) == Some(&key)).then_some(index)
}

#[cfg(test)]
mod tests_agent09 {
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

    #[test]
    fn value_gate_rejects_non_allied_buildings_and_flagged_units() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let cruiser = w.spawn(0, uid::BATTLECRUISER, 900, 320).unwrap();
        assert!(w.a09_value_target_gate(cruiser, true));
        assert!(!w.a09_value_target_gate(cruiser, false));

        let centre = w.spawn(0, uid::COMMAND_CENTER, 1400, 900).unwrap();
        assert!(!w.a09_value_target_gate(centre, true));

        assert!(!w.a09_value_target_gate(marine, true));
        w.units[marine].set_mask_123_bit(0);
        assert!(!w.a09_value_target_gate(marine, true));
        w.units[cruiser].set_mask_123_bit(0);
        assert!(!w.a09_value_target_gate(cruiser, true));
    }

    #[test]
    fn value_gate_lets_detectors_and_high_hp_units_through() {
        let mut w = world();
        let vessel = w.spawn(0, uid::SCIENCE_VESSEL, 320, 320).unwrap();
        w.units[vessel].hp = 10 << 8;
        assert!(w.data.unit(uid::SCIENCE_VESSEL).is_detector());
        assert!(w.a09_value_target_gate(vessel, true));

        let marine = w.spawn(0, uid::MARINE, 640, 640).unwrap();
        w.units[marine].hp = 10 << 8;
        assert!(!w.a09_value_target_gate(marine, true));
        w.units[marine].hp = 0x51 << 8;
        assert!(w.a09_value_target_gate(marine, true));
    }

    #[test]
    fn value_gate_uses_inclusive_81_point_threshold() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        w.units[marine].hp = 80 << 8;
        assert_eq!(w.unit_current_total_points(marine), 80);
        assert!(!w.a09_value_target_gate(marine, true));
        w.units[marine].hp = 81 << 8;
        assert_eq!(w.unit_current_total_points(marine), 81);
        assert!(w.a09_value_target_gate(marine, true));
    }

    #[test]
    fn energy_covers_heal_compares_whole_points() {
        let mut w = world();
        let ghost = w.spawn(0, uid::GHOST, 320, 320).unwrap();
        assert!(w.data.unit(uid::GHOST).has_energy());
        w.units[ghost].energy = 45;
        assert_eq!(w.unit_current_total_points(ghost), 45);
        assert!(w.a09_energy_covers_heal(ghost, true, false));

        w.units[ghost].energy = 44;
        assert!(!w.a09_energy_covers_heal(ghost, true, false));
    }

    #[test]
    fn energy_covers_heal_rejects_allied_flag_status_and_heroes() {
        let mut w = world();
        let ghost = w.spawn(0, uid::GHOST, 320, 320).unwrap();
        w.units[ghost].energy = 200;
        assert!(!w.a09_energy_covers_heal(ghost, false, false));
        assert!(!w.a09_energy_covers_heal(ghost, true, true));
        assert!(w.a09_energy_covers_heal(ghost, true, false));

        let hero = (0..w.data.units.len())
            .find(|&t| {
                let ty = w.data.unit(t as u16);
                ty.is_hero() && ty.has_energy()
            })
            .expect("bundled data has an energy hero") as u16;
        let hero = w.spawn(0, hero, 900, 900).unwrap();
        w.units[hero].energy = 250;
        assert!(!w.a09_energy_covers_heal(hero, true, false));
    }

    #[test]
    fn carrier_low_hp_target_requires_family_order_target_and_owner() {
        let mut w = world();
        let carrier = w.spawn(0, uid::CARRIER, 320, 320).unwrap();
        w.units[carrier].hp = 10 << 8;
        assert!(!w.a09_carrier_has_low_hp_target(carrier, 0));
        w.units[carrier].order = Order::AttackUnit { target: 7 };
        assert!(w.a09_carrier_has_low_hp_target(carrier, 0));

        assert!(!w.a09_carrier_has_low_hp_target(carrier, 1));
        w.units[carrier].hp = 11 << 8;
        assert!(!w.a09_carrier_has_low_hp_target(carrier, 0));

        let marine = w.spawn(0, uid::MARINE, 640, 640).unwrap();
        w.units[marine].hp = 5 << 8;
        w.units[marine].order = Order::AttackUnit { target: 7 };
        assert!(!w.a09_carrier_has_low_hp_target(marine, 0));
    }

    #[test]
    fn recharge_status_gate_needs_status_same_owner_and_target_predicate() {
        let mut w = world();
        let a = w.spawn(0, uid::MEDIC, 320, 320).unwrap();
        let b = w.spawn(0, uid::MARINE, 640, 640).unwrap();
        let mut ua = w.units[a].clone();
        let mut ub = w.units[b].clone();
        assert!(!ua.a09_recharge_status_gate(&ub, true));

        ua.set_mask_121_bit(0);
        assert!(ua.a09_recharge_status_gate(&ub, true));
        assert!(!ua.a09_recharge_status_gate(&ub, false));
        ub.owner = 1;
        assert!(!ua.a09_recharge_status_gate(&ub, true));

        let mut uc = w.units[a].clone();
        uc.clear_mask_121();
        uc.status.maelstrom_ticks = 3;
        assert!(uc.a09_recharge_status_gate(&w.units[b], true));
    }

    #[test]
    fn accumulator_gate_threshold_and_reset() {
        assert_eq!(a09_accumulator_gate(false, 0), (false, 0));
        assert_eq!(a09_accumulator_gate(false, 249), (false, 249));
        assert_eq!(a09_accumulator_gate(false, 250), (true, 0));
        assert_eq!(a09_accumulator_gate(true, 3), (true, 0));
    }

    #[test]
    fn map_slot_get_matches_existing_keys_only() {
        let mut map = BTreeMap::new();
        map.insert(7, 0x36e);
        assert_eq!(a09_map_value_u16(&map, 7), Some(0x36e));
        assert_eq!(a09_map_value_u16(&map, 8), None);
    }

    #[test]
    fn map_slot_store_only_overwrites_existing_entries() {
        let mut map = BTreeMap::new();
        map.insert(7, 0x36e);
        assert!(a09_map_store_u16(&mut map, 7, 0x36c));
        assert_eq!(map[&7], 0x36c);
        assert!(!a09_map_store_u16(&mut map, 9, 0x386));
        assert!(!map.contains_key(&9));
    }

    #[test]
    fn unsigned_tree_lookup_is_exact_match() {
        let keys = [1u32, 3, 5, 7];
        assert_eq!(a09_unsigned_key_index(&keys, 1), Some(0));
        assert_eq!(a09_unsigned_key_index(&keys, 5), Some(2));
        assert_eq!(a09_unsigned_key_index(&keys, 4), None);
        assert_eq!(a09_unsigned_key_index(&keys, 9), None);
        assert_eq!(a09_unsigned_key_index(&[], 1), None);
    }

    #[test]
    fn signed_tree_lookup_is_exact_match() {
        let keys = [-5i32, -1, 3];
        assert_eq!(a09_signed_key_index(&keys, -5), Some(0));
        assert_eq!(a09_signed_key_index(&keys, -1), Some(1));
        assert_eq!(a09_signed_key_index(&keys, 0), None);
        assert_eq!(a09_signed_key_index(&keys, 4), None);
    }

    #[test]
    fn facing_near_uses_five_step_window() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
        let mut u = w.units[marine].clone();
        u.facing = 250;
        assert!(u.a09_facing_near(250));
        assert!(u.a09_facing_near(254));
        assert!(u.a09_facing_near(255));
        assert!(!u.a09_facing_near(0));
        assert!(!u.a09_facing_near(1));

        u.facing = 0;
        assert!(u.a09_facing_near(5));
        assert!(!u.a09_facing_near(6));
        assert!(u.a09_facing_near(251));
        assert!(!u.a09_facing_near(250));
    }
}
