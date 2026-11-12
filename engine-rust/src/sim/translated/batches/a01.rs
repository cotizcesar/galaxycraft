#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent01.rs
// ===========================================================================
// Translation batch 01.  Only this file is edited by the batch.
//
// Every function below cites the decompiled address it reconstructs.
// The retail data arrays were identified from the loader descriptor table
// at `0x00b01ad0`: `0x00f94110` is the `weapons.dat` damage column,
// `0x00f94218` the damage-bonus column, `0x00f94320` the cooldown column
// and `0x00f943a8` the damage-factor/bullet-count column.  The units.dat
// ground-weapon column used here is `0x00f8f580`, with `0x82` (130) as the
// "no weapon" sentinel.

impl SimUnit {
    /// `0x005a2d20`: special-unit order-id dispatch.  Carrier/Gantrithor
    /// produce `0x39`, Reaver/Warbringer `0x3e`, Queen/Matriarch `0x6c`,
    /// Scourge/Infested Terran `0x88`, Medic `0xb2`; every other unit falls
    /// back to `0x6b` (Hold Position).
    pub fn a01_special_order_id(&self) -> u8 {
        if is_carrier_type(self.type_id) {
            0x39
        } else if matches!(self.type_id, 0x53 | 0x51) {
            0x3e
        } else if is_queen_type(self.type_id) {
            0x6c
        } else if is_suicide_unit_type(self.type_id) {
            0x88
        } else if is_medic_type(self.type_id) {
            0xb2
        } else {
            0x6b
        }
    }

    /// `0x005a2d55`: the short form of the same dispatch.  A nonzero input
    /// selects `0x88` (suicide order); otherwise a Medic gets `0xb2` and
    /// every other unit gets `0x6b`.
    pub fn a01_special_order_id_with_flag(&self, flag: bool) -> u8 {
        if flag {
            0x88
        } else if is_medic_type(self.type_id) {
            0xb2
        } else {
            0x6b
        }
    }

    /// `0x005a2dd0`: adjusted `weapons.dat` cooldown.  Starts from the raw
    /// cooldown byte (`0x00f94320[weapon]`), adds
    /// `max(3, cooldown >> 3)` per acid spore (`FUN_005b3ba0` reads
    /// `CUnit+0x126`), applies the `FUN_005b49a0` modifier (`-1` adds 25%,
    /// `+1` halves it) and finally clamps the result to `5..=250`.
    pub fn a01_adjusted_weapon_cooldown(&self, base_cooldown: u8, modifier: i32) -> u32 {
        let mut value = u32::from(base_cooldown);
        let spores = u32::from(self.status.acid_spores);
        if spores != 0 {
            let per_spore = 3.max(value >> 3);
            value += per_spore * spores;
        }
        if modifier < 0 {
            value += value >> 2;
        } else if modifier > 0 {
            value >>= 1;
        }
        value.clamp(5, 250)
    }

    /// `0x005a2e40` (identical body at `0x005a2e90`): a Lurker
    /// (`units.dat` row `0x67`) that is still incomplete yields order `0x82`;
    /// every other unit or a completed Lurker yields nothing.
    pub fn a01_lurker_order_id(&self) -> Option<u8> {
        (self.type_id == 0x67 && !self.completed).then_some(0x82)
    }

    /// `0x005a651c`: the unit-type dispatch that runs the Pylon-only
    /// `FUN_005c4b00` update.  True exactly for the Pylon row `0x9c`.
    pub fn a01_pylon_update_required(&self) -> bool {
        self.type_id == 0x9c
    }
}

impl World {
    /// `0x005a3e92`: the type's `units.dat` ground weapon (`0x00f8f580`),
    /// or `None` for the `0x82` sentinel (no weapon).
    pub fn a01_ground_weapon_or_none(&self, type_id: u16) -> Option<u8> {
        let weapon = self.data.unit(type_id).ground_weapon;
        (weapon != 0x82).then_some(weapon)
    }
}

/// `0x005a0720` / `0x005a077a`: the effect dispatcher runs the weapon action
/// twice exactly when the `weapons.dat` damage-factor byte
/// (`0x00f943a8[weapon]`) equals 2.
pub fn a01_weapon_effect_repetitions(weapon: &WeaponType) -> u32 {
    if weapon.bullet_count == 2 { 2 } else { 1 }
}

/// `0x005a3220`: true when the weapon's minimum range
/// (`0x00f93740[weapon]`) is zero or no larger than `distance`.
pub fn a01_min_range_satisfied(weapon: &WeaponType, distance: u32) -> bool {
    weapon.min_range == 0 || weapon.min_range <= distance
}

/// `0x005a4570`: upgraded bonus damage for one weapon volley.  The high byte
/// of the retail concatenated value is the owner's upgrade level (read by
/// `FUN_005b6b60`), the low byte is `0x00f943a8[weapon]`.  Firebat/Gui Montag
/// and Zealot/Fenix force two hits; Valkyrie forces one.  The table product
/// is truncated to 16 bits before the hit count is applied.
pub fn a01_volley_bonus_damage(unit_type: u16, weapon: &WeaponType, upgrade_level: u8) -> i32 {
    let hits = if is_firebat_family_type(unit_type) || is_zealot_type(unit_type) {
        2
    } else if is_valkyrie_type(unit_type) {
        1
    } else {
        weapon.bullet_count
    };
    let bonus = (u32::from(weapon.damage_bonus) * u32::from(upgrade_level)) & 0xffff;
    (bonus * u32::from(hits)) as i32
}

/// `0x005a45ae`: the parameterised tail of `0x005a4570`.
/// `damage_bonus * upgrade_level` is truncated to 16 bits and multiplied by
/// `hits`; `force_single_hit` selects the Valkyrie branch (one hit).
pub fn a01_scaled_bonus_damage(
    damage_bonus: u16,
    upgrade_level: u8,
    hits: u8,
    force_single_hit: bool,
) -> i32 {
    let hits = if force_single_hit { 1 } else { hits };
    let bonus = (u32::from(damage_bonus) * u32::from(upgrade_level)) & 0xffff;
    (bonus * u32::from(hits)) as i32
}

/// `0x005a45e0`: one volley's damage.  Starts from
/// `0x00f94110[weapon]` (the `weapons.dat` damage column) and multiplies by
/// the damage-factor byte, except that Firebat/Gui Montag and Zealot/Fenix
/// force two hits and the Valkyrie forces one.
pub fn a01_volley_damage(unit_type: u16, weapon: &WeaponType) -> i32 {
    let hits = if is_firebat_family_type(unit_type) || is_zealot_type(unit_type) {
        2
    } else if is_valkyrie_type(unit_type) {
        1
    } else {
        weapon.bullet_count
    };
    i32::from(weapon.damage) * i32::from(hits)
}

#[cfg(test)]
mod tests_agent01 {
    use super::*;
    use crate::data::GameData;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, type_id: u16) -> usize {
        world.spawn(0, type_id, 320, 320).expect("spawn")
    }

    #[test]
    fn special_order_dispatch_matches_the_retail_chain() {
        let mut w = world();
        let cases = [
            (0x48, 0x39), // Carrier
            (0x52, 0x39), // Gantrithor
            (0x53, 0x3e), // Reaver
            (0x51, 0x3e), // Warbringer
            (0x2d, 0x6c), // Queen
            (0x31, 0x6c), // Matriarch
            (0x2f, 0x88), // Scourge
            (0x32, 0x88), // Infested Terran
            (0x22, 0xb2), // Medic
            (0x00, 0x6b), // Marine fallback
        ];
        for (type_id, expected) in cases {
            let slot = spawn(&mut w, type_id);
            assert_eq!(
                w.units[slot].a01_special_order_id(),
                expected,
                "type {type_id:#x}"
            );
        }
    }

    #[test]
    fn special_order_short_form_uses_flag_then_medic_then_hold() {
        let mut w = world();
        let medic = spawn(&mut w, 0x22);
        let marine = spawn(&mut w, 0x00);
        assert_eq!(w.units[medic].a01_special_order_id_with_flag(true), 0x88);
        assert_eq!(w.units[medic].a01_special_order_id_with_flag(false), 0xb2);
        assert_eq!(w.units[marine].a01_special_order_id_with_flag(false), 0x6b);
    }

    #[test]
    fn adjusted_cooldown_adds_acid_spores_and_modifiers() {
        let mut w = world();
        let slot = spawn(&mut w, 0x00);
        let unit = &mut w.units[slot];
        assert_eq!(unit.a01_adjusted_weapon_cooldown(15, 0), 15);
        unit.status.acid_spores = 3;
        assert_eq!(unit.a01_adjusted_weapon_cooldown(15, 0), 24);
        assert_eq!(unit.a01_adjusted_weapon_cooldown(15, -1), 30);
        assert_eq!(unit.a01_adjusted_weapon_cooldown(15, 1), 12);
    }

    #[test]
    fn adjusted_cooldown_clamps_to_five_through_two_hundred_fifty() {
        let mut w = world();
        let slot = spawn(&mut w, 0x00);
        let unit = &w.units[slot];
        assert_eq!(unit.a01_adjusted_weapon_cooldown(1, 0), 5);
        assert_eq!(unit.a01_adjusted_weapon_cooldown(250, 0), 250);
        assert_eq!(unit.a01_adjusted_weapon_cooldown(250, -1), 250);
        assert_eq!(unit.a01_adjusted_weapon_cooldown(255, 0), 250);
    }

    #[test]
    fn incomplete_lurker_returns_order_eighty_two() {
        let mut w = world();
        let lurker = spawn(&mut w, 0x67);
        let marine = spawn(&mut w, 0x00);
        assert_eq!(w.units[lurker].a01_lurker_order_id(), None);
        assert_eq!(w.units[marine].a01_lurker_order_id(), None);
        w.units[lurker].completed = false;
        assert_eq!(w.units[lurker].a01_lurker_order_id(), Some(0x82));
    }

    #[test]
    fn pylon_dispatch_is_limited_to_the_pylon_row() {
        let mut w = world();
        let pylon = spawn(&mut w, 0x9c);
        let marine = spawn(&mut w, 0x00);
        assert!(w.units[pylon].a01_pylon_update_required());
        assert!(!w.units[marine].a01_pylon_update_required());
    }

    #[test]
    fn ground_weapon_or_none_uses_the_units_dat_column() {
        let w = world();
        assert_eq!(w.a01_ground_weapon_or_none(0x00), Some(0)); // Marine
        assert_eq!(w.a01_ground_weapon_or_none(0x41), Some(64)); // Zealot
        assert_eq!(w.a01_ground_weapon_or_none(0x22), None); // Medic
        assert_eq!(w.a01_ground_weapon_or_none(0x48), None); // Carrier
    }

    #[test]
    fn weapon_effect_runs_twice_only_for_a_factor_of_two() {
        let w = world();
        let one = w.data.weapon(0).unwrap();
        let two = w.data.weapon(103).unwrap();
        assert_eq!(one.bullet_count, 1);
        assert_eq!(two.bullet_count, 2);
        assert_eq!(a01_weapon_effect_repetitions(one), 1);
        assert_eq!(a01_weapon_effect_repetitions(two), 2);
    }

    #[test]
    fn minimum_range_predicate_matches_the_weapon_row() {
        let w = world();
        let no_minimum = w.data.weapon(0).unwrap();
        let minimum = w.data.weapon(27).unwrap();
        assert_eq!(no_minimum.min_range, 0);
        assert_eq!(minimum.min_range, 64);
        assert!(a01_min_range_satisfied(no_minimum, 0));
        assert!(!a01_min_range_satisfied(minimum, 63));
        assert!(a01_min_range_satisfied(minimum, 64));
        assert!(a01_min_range_satisfied(minimum, 384));
    }

    #[test]
    fn volley_damage_applies_the_type_hit_overrides() {
        let w = world();
        let marine_weapon = w.data.weapon(0).unwrap(); // Gauss Rifle
        let firebat_weapon = w.data.weapon(25).unwrap(); // Flame Thrower
        let zealot_weapon = w.data.weapon(64).unwrap(); // Psi Blades
        let valkyrie_weapon = w.data.weapon(103).unwrap();

        assert_eq!(
            a01_volley_damage(0x00, marine_weapon),
            i32::from(marine_weapon.damage) * i32::from(marine_weapon.bullet_count)
        );
        assert_eq!(
            a01_volley_damage(0x20, firebat_weapon),
            i32::from(firebat_weapon.damage) * 2
        );
        assert_eq!(
            a01_volley_damage(0x41, zealot_weapon),
            i32::from(zealot_weapon.damage) * 2
        );
        assert_eq!(
            a01_volley_damage(0x3a, valkyrie_weapon),
            i32::from(valkyrie_weapon.damage)
        );
    }

    #[test]
    fn volley_bonus_damage_multiplies_bonus_upgrade_and_hits() {
        let w = world();
        let valkyrie_weapon = w.data.weapon(103).unwrap();
        let marine_weapon = w.data.weapon(0).unwrap();
        assert_eq!(valkyrie_weapon.damage_bonus, 1);
        assert_eq!(valkyrie_weapon.bullet_count, 2);
        // Valkyrie forces one hit even though the table says two.
        assert_eq!(a01_volley_bonus_damage(0x3a, valkyrie_weapon, 3), 3);
        assert_eq!(
            a01_volley_bonus_damage(0x00, marine_weapon, 3),
            i32::from(marine_weapon.damage_bonus) * 3 * i32::from(marine_weapon.bullet_count)
        );
    }

    #[test]
    fn scaled_bonus_damage_truncates_and_honours_forced_single_hit() {
        assert_eq!(a01_scaled_bonus_damage(2, 3, 4, false), 24);
        assert_eq!(a01_scaled_bonus_damage(2, 3, 4, true), 6);
        // 0xffff * 255 is truncated to 0xff01 by the 16-bit table product.
        assert_eq!(a01_scaled_bonus_damage(0xffff, 255, 2, false), 130562);
    }
}
