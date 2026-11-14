//! Rate of fire of every armed ground unit, against OpenBW: a unit attacking
//! a held target fires every `weapons.dat` cooldown plus a random -1..=+2
//! frames (`attack_unit_fire_weapon`), the order timer is cut to the cooldown
//! and a long swing does not delay the next shot (`iscript_run_anim`).

use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::Terrain;
use galaxycraft::sim::{Order, World};
use std::sync::Arc;

/// Mean frames between attacks against a completed Supply Depot whose health
/// is restored after each hit (hits under 4 frames apart are one attack: the
/// Zealot's two blows, the Firebat's flames), and the number of attacks.
fn period(data: &Arc<GameData>, ty: u16, target: u16) -> Option<(f64, u32)> {
    let mut w = World::new(data.clone(), Terrain::open(64, 64), 2);
    let m = w.spawn_px(0, ty, 400, 400).ok()?;
    let t = w.spawn_px(1, target, 480, 400).ok()?;
    w.units[t].completed = true;
    let max_hp = w.units[t].hp;
    let (mid, tid) = (w.units[m].id, w.units[t].id);
    w.issue(mid, Order::AttackUnit { target: tid }).ok()?;
    let (mut hits, mut first, mut last) = (0u32, 0u64, 0u64);
    for _ in 0..2000 {
        w.step();
        if w.units[t].hp < max_hp {
            if hits == 0 {
                first = w.tick;
                hits = 1;
            } else if w.tick - last >= 4 {
                hits += 1;
            }
            last = w.tick;
            w.units[t].hp = max_hp;
        }
    }
    (hits > 10).then(|| ((last - first) as f64 / (hits - 1) as f64, hits))
}

#[test]
fn every_armed_ground_unit_fires_at_its_weapons_dat_cooldown() {
    let data = Arc::new(GameData::bundled().unwrap());
    let mut checked = 0;
    let mut bad = Vec::new();
    for ty in 0..228u16 {
        let ut = data.unit(ty);
        if ut.is_building() || ut.ground_weapon == 130 {
            continue;
        }
        let Some(w) = data.ground_weapon(ut) else {
            continue;
        };
        // Units whose damage is not one weapon swing per cooldown: hangar
        // and self-destruct units, burrowed Lurkers, Spider Mines, Archons'
        // splash pair and two-hit weapons are measured per attack below.
        // Turret subunits (4, 6, 18, 24) fire through their base unit.
        if matches!(
            ty,
            4 | 6 | 18 | 24 | 13 | 14 | 46 | 50 | 83 | 84 | 103 | 85 | 72 | 73
        ) {
            continue;
        }
        let Some((p, hits)) = period(&data, ty, uid::SUPPLY_DEPOT) else {
            continue;
        };
        let per_attack = p;
        // The Devouring One (54) always has Adrenal Glands: half cooldown,
        // clamped to 5 (`get_modified_weapon_cooldown`).
        let cooldown = if ty == 54 {
            (w.cooldown as u32 / 2).max(5)
        } else {
            w.cooldown as u32
        };
        let want = cooldown as f64 + 0.5;
        checked += 1;
        if (per_attack - want).abs() > 1.5 {
            bad.push(format!(
                "type {ty}: {per_attack:.2} frames per attack, want ~{want} ({hits} hits)"
            ));
        }
    }
    assert!(checked > 40, "only {checked} units measured");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn every_armed_air_attacker_fires_at_its_weapons_dat_cooldown() {
    let data = Arc::new(GameData::bundled().unwrap());
    let mut checked = 0;
    let mut bad = Vec::new();
    for ty in 0..228u16 {
        let ut = data.unit(ty);
        if ut.is_building()
            || matches!(
                ty,
                4 | 6 | 18 | 24 | 13 | 14 | 46 | 50 | 83 | 84 | 85 | 72 | 73 | 103
            )
        {
            continue;
        }
        let Some(w) = data.air_weapon(ut) else {
            continue;
        };
        let Some((p, hits)) = period(&data, ty, uid::OVERLORD) else {
            continue;
        };
        let want = w.cooldown as f64 + 0.5;
        // The Valkyrie's volley lands four times (16 frames apart) per
        // cooldown: its iscript repeats `attack` inside one swing.
        let p = if ty == 58 { p * 4.0 } else { p };
        checked += 1;
        if (p - want).abs() > 1.5 {
            bad.push(format!(
                "type {ty}: {p:.2} frames per attack, want ~{want} ({hits} attacks)"
            ));
        }
    }
    assert!(checked > 15, "only {checked} units measured");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// hp damage of one hit from the executable's damage matrix
/// (`fixtures/oracle_damage.tsv`): upgrade 0, large target, armor 1, no shields.
fn oracle_hit(weapon: u8) -> Option<i32> {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/oracle_damage.tsv"
    ))
    .ok()?;
    text.lines().skip(1).find_map(|l| {
        let f: Vec<i32> = l.split('\t').filter_map(|x| x.parse().ok()).collect();
        (f.len() == 8
            && f[0] == weapon as i32
            && f[1] == 0
            && f[2] == 3
            && f[3] == 1
            && f[4] == 0
            && f[5] == 0)
            .then_some(f[7])
    })
}

#[test]
fn every_single_hit_weapon_deals_the_executable_damage_per_hit() {
    let data = Arc::new(GameData::bundled().unwrap());
    let mut checked = 0;
    let mut bad = Vec::new();
    for ty in 0..228u16 {
        let ut = data.unit(ty);
        if ut.is_building()
            || matches!(
                ty,
                4 | 6 | 18 | 24 | 13 | 14 | 46 | 50 | 83 | 84 | 85 | 72 | 73 | 103
            )
        {
            continue;
        }
        let Some(w) = data.ground_weapon(ut) else {
            continue;
        };
        let weapon = data.ground_weapon_id(ut);
        // Flame weapons (Firebat 25, Gui Montag 26, flame traps 98) release
        // three splash flames at 24 / 52 / 80 px (`attkshiftproj`): against
        // the Depot the first lands 18 px from its box, in the 50 % ring
        // (192), the others inside it (448).  Not one hit per attack.
        if matches!(weapon, 25 | 26 | 98) {
            continue;
        }
        let Some(want) = oracle_hit(weapon) else {
            continue;
        };
        let mut world = World::new(data.clone(), Terrain::open(64, 64), 2);
        let Ok(m) = world.spawn_px(0, ty, 400, 400) else {
            continue;
        };
        let t = world.spawn_px(1, uid::SUPPLY_DEPOT, 480, 400).unwrap();
        world.units[t].completed = true;
        let max_hp = world.units[t].hp;
        let (mid, tid) = (world.units[m].id, world.units[t].id);
        if world.issue(mid, Order::AttackUnit { target: tid }).is_err() {
            continue;
        }
        let mut losses = Vec::new();
        for _ in 0..400 {
            world.step();
            if world.units[t].hp < max_hp {
                losses.push(max_hp - world.units[t].hp);
                world.units[t].hp = max_hp;
            }
        }
        if losses.is_empty() {
            continue;
        }
        checked += 1;
        // One damage event is one bullet; a weapon with two bullets may land
        // both on the same frame.
        let per_bullet: Vec<i32> = losses
            .iter()
            .map(|&l| {
                if l == want * w.bullet_count.max(1) as i32 {
                    want
                } else {
                    l
                }
            })
            .collect();
        if per_bullet.iter().any(|&l| l != want) {
            let mut seen: Vec<i32> = per_bullet.clone();
            seen.sort();
            seen.dedup();
            bad.push(format!(
                "type {ty} weapon {weapon}: hits {seen:?}, executable {want}"
            ));
        }
    }
    assert!(checked > 40, "only {checked} units measured");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
