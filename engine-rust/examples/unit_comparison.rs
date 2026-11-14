//! Writes `docs/UNIT-COMPARISON.md`: every unit and building of the game with
//! its stats beside the executable's own tables (`fixtures/oracle_units.tsv`,
//! `fixtures/oracle_weapons.tsv`), and the rate of fire and damage per hit
//! measured in the simulation against `weapons.dat` cooldown + 0.5 (OpenBW
//! `attack_unit_fire_weapon`) and the damage matrix (`oracle_damage.tsv`).
//!
//! `cargo run --release --example unit_comparison`

use galaxycraft::data::{GameData, UnitType, WeaponType};
use galaxycraft::ids::unit as uid;
use galaxycraft::map::Terrain;
use galaxycraft::sim::{Order, World};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn table(name: &str) -> (Vec<String>, HashMap<i64, Vec<i64>>) {
    let text = std::fs::read_to_string(format!("{ROOT}/fixtures/{name}")).unwrap();
    let mut lines = text.lines();
    let head = lines
        .next()
        .unwrap()
        .split('\t')
        .map(String::from)
        .collect();
    let rows = lines
        .map(|l| {
            l.split('\t')
                .map(|x| x.parse().unwrap_or(-1))
                .collect::<Vec<i64>>()
        })
        .map(|r| (r[0], r))
        .collect();
    (head, rows)
}

fn col(head: &[String], row: &[i64], name: &str) -> i64 {
    row[head.iter().position(|h| h == name).unwrap()]
}

/// Fields that differ between our `units.dat` row and the oracle.
fn unit_diffs(u: &UnitType, head: &[String], row: &[i64]) -> Vec<&'static str> {
    let ours: [(&str, i64); 15] = [
        ("hp", u.hit_points_raw as i64),
        ("shield", u.shield_points as i64),
        ("has_shield", u.has_shields as i64),
        ("armor", u.armor as i64),
        ("size", u.size as u8 as i64),
        ("ground_weapon", u.ground_weapon as i64),
        ("air_weapon", u.air_weapon as i64),
        ("sight", u.sight_range as i64),
        ("mineral", u.mineral_cost as i64),
        ("gas", u.gas_cost as i64),
        ("build_time", u.build_time as i64),
        ("supply_required", u.supply_required as i64),
        ("supply_provided", u.supply_provided as i64),
        ("flags", u.flags as i64),
        ("target_acq", u.target_acquisition_range as i64),
    ];
    ours.iter()
        .filter(|(n, v)| col(head, row, n) != *v)
        .map(|(n, _)| *n)
        .collect()
}

fn weapon_diffs(w: &WeaponType, head: &[String], row: &[i64]) -> Vec<&'static str> {
    let ours: [(&str, i64); 7] = [
        ("damage", w.damage as i64),
        ("damage_bonus", w.damage_bonus as i64),
        ("cooldown", w.cooldown as i64),
        ("max_range", w.max_range as i64),
        ("min_range", w.min_range as i64),
        ("damage_type", w.damage_type as u8 as i64),
        ("bullets", w.bullet_count as i64),
    ];
    ours.iter()
        .filter(|(n, v)| col(head, row, n) != *v)
        .map(|(n, _)| *n)
        .collect()
}

/// Mean frames between attacks on a held target whose health is restored
/// after each hit, and the hp lost on the first hit.
fn measure(data: &Arc<GameData>, ty: u16, target: u16) -> Option<(f64, i32)> {
    let mut w = World::new(data.clone(), Terrain::open(64, 64), 2);
    let m = w.spawn_px(0, ty, 400, 400).ok()?;
    let t = w.spawn_px(1, target, 480, 400).ok()?;
    w.units[m].completed = true;
    w.units[t].completed = true;
    let max_hp = w.units[t].hp;
    let (mid, tid) = (w.units[m].id, w.units[t].id);
    w.issue(mid, Order::AttackUnit { target: tid }).ok()?;
    let (mut hits, mut first, mut last, mut first_loss) = (0u32, 0u64, 0u64, 0);
    for _ in 0..2000 {
        w.step();
        if w.units[t].hp < max_hp {
            if hits == 0 {
                first = w.tick;
                first_loss = max_hp - w.units[t].hp;
                hits = 1;
            } else if w.tick - last >= 4 {
                hits += 1;
            }
            last = w.tick;
            w.units[t].hp = max_hp;
        }
    }
    (hits > 10).then(|| ((last - first) as f64 / (hits - 1) as f64, first_loss))
}

/// One hit's hp damage from the executable's matrix: upgrade 0, large,
/// armor 1, no shields (a Supply Depot).
fn oracle_hit(dmg: &str, weapon: u8) -> Option<i64> {
    dmg.lines().skip(1).find_map(|l| {
        let f: Vec<i64> = l.split('\t').filter_map(|x| x.parse().ok()).collect();
        (f.len() == 8 && f[0] == weapon as i64 && f[1..6] == [0, 3, 1, 0, 0]).then_some(f[7])
    })
}

fn size_name(u: &UnitType) -> &'static str {
    ["-", "small", "medium", "large"][u.size as usize]
}

fn main() {
    let data = Arc::new(GameData::bundled().unwrap());
    let (uh, urows) = table("oracle_units.tsv");
    let (wh, wrows) = table("oracle_weapons.tsv");
    let dmg = std::fs::read_to_string(format!("{ROOT}/fixtures/oracle_damage.tsv")).unwrap();
    // Units whose damage is not one swing per cooldown (hangars, suicide
    // units, Spider Mines, turret subunits, flame splash): stats only.
    let multi = |ty: u16| {
        matches!(
            ty,
            4 | 6 | 18 | 24 | 13 | 14 | 46 | 50 | 83 | 84 | 85 | 72 | 73 | 103
        )
    };
    let mut out = String::new();
    let (mut n, mut data_ok, mut rof_ok, mut rof_n, mut hit_ok, mut hit_n) = (0, 0, 0, 0, 0, 0);
    let mut rows = String::new();
    for ty in 0..228u16 {
        let u = data.unit(ty);
        let name = data.unit_name(ty).replace('|', "/");
        let Some(orow) = urows.get(&(ty as i64)) else {
            continue;
        };
        n += 1;
        let mut diffs: Vec<String> = unit_diffs(u, &uh, orow)
            .iter()
            .map(|s| s.to_string())
            .collect();
        for (wid, label) in [(u.ground_weapon, "g"), (u.air_weapon, "a")] {
            if wid == 130 {
                continue;
            }
            if let (Some(w), Some(r)) = (data.weapon(wid), wrows.get(&(wid as i64))) {
                diffs.extend(
                    weapon_diffs(w, &wh, r)
                        .iter()
                        .map(|d| format!("{label}.{d}")),
                );
            }
        }
        if diffs.is_empty() {
            data_ok += 1;
        }
        let gw = data.ground_weapon(u);
        let aw = data.air_weapon(u);
        let wdesc = |w: Option<&WeaponType>| {
            w.map(|w| {
                format!(
                    "{}+{} cd{} r{}{}",
                    w.damage,
                    w.damage_bonus,
                    w.cooldown,
                    w.max_range,
                    if w.bullet_count > 1 {
                        format!(" x{}", w.bullet_count)
                    } else {
                        String::new()
                    }
                )
            })
            .unwrap_or_else(|| "-".into())
        };
        let (mut rof, mut hit) = ("-".to_string(), "-".to_string());
        if !multi(ty) && !u.is_resource_container() {
            if let Some(w) = gw
                && let Some((p, loss)) = measure(&data, ty, uid::SUPPLY_DEPOT)
            {
                let cd = if ty == 54 {
                    (w.cooldown as u32 / 2).max(5)
                } else {
                    w.cooldown as u32
                };
                let want = cd as f64 + 0.5;
                let ok = (p - want).abs() <= 1.5;
                rof_n += 1;
                rof_ok += ok as u32;
                rof = format!("{p:.1}/{want} {}", if ok { "ok" } else { "**DIFF**" });
                let wid = data.ground_weapon_id(u);
                if !matches!(wid, 25 | 26 | 98)
                    && let Some(o) = oracle_hit(&dmg, wid)
                {
                    let bc = w.bullet_count.max(1) as i64;
                    let ok = loss as i64 == o || loss as i64 == o * bc;
                    hit_n += 1;
                    hit_ok += ok as u32;
                    hit = format!("{loss}/{o} {}", if ok { "ok" } else { "**DIFF**" });
                }
            }
            if gw.is_none()
                && let Some(w) = aw
                && let Some((p, _)) = measure(&data, ty, uid::OVERLORD)
            {
                let p = if ty == 58 { p * 4.0 } else { p };
                let want = w.cooldown as f64 + 0.5;
                let ok = (p - want).abs() <= 1.5;
                rof_n += 1;
                rof_ok += ok as u32;
                rof = format!("{p:.1}/{want} (air) {}", if ok { "ok" } else { "**DIFF**" });
            }
        }
        let shields = if u.has_shields {
            u.shield_points.to_string()
        } else {
            "-".into()
        };
        let supply = if u.supply_provided > 0 {
            format!("+{}", u.supply_provided as f32 / 2.0)
        } else {
            format!("{}", u.supply())
        };
        let mut tags = Vec::new();
        if u.is_building() {
            tags.push("building");
        }
        if u.is_detector() {
            tags.push("detector");
        }
        if u.is_resource_container() {
            tags.push("resource");
        }
        if u.is_hero() {
            tags.push("hero");
        }
        let _ = writeln!(
            rows,
            "| {ty} | {name} | {} | {shields} | {} {} | {}/{} | {} | {supply} | {} | {} | {} | {rof} | {hit} | {} | {} |",
            u.hit_points_raw / 256,
            u.armor,
            size_name(u),
            u.mineral_cost,
            u.gas_cost,
            u.build_time,
            u.sight_range,
            wdesc(gw),
            wdesc(aw),
            tags.join(" "),
            if diffs.is_empty() {
                "ok".to_string()
            } else {
                format!("**{}**", diffs.join(","))
            },
        );
    }
    let _ = writeln!(
        out,
        "# Unit comparison\n\n\
Generated by `cargo run --release --example unit_comparison`; do not edit by hand.\n\n\
Every unit, building and resource of `units.dat` (228 rows) beside the\n\
executable's own tables, extracted independently into `fixtures/oracle_*.tsv`\n\
(`tests/original_parity.rs` asserts the same).  Rate of fire is measured in the\n\
simulation (attacker holding on a Supply Depot, or an Overlord for air-only\n\
attackers, health restored after each hit) against `weapons.dat` cooldown + 0.5\n\
(OpenBW `attack_unit_fire_weapon`: cooldown + rand(-1..=2)).  Damage per hit is\n\
the first hit on the Depot (large, armor 1) against the executable's damage\n\
matrix (`oracle_damage.tsv`).  Spell effects, durations and special weapons are\n\
checked in `tests/effects_openbw.rs`; training, research and upgrade times in\n\
`tests/build_times_retail.rs`.\n\n\
## Summary\n\n\
| check | matching |\n|---|---|\n\
| unit + weapon data vs executable | {data_ok} / {n} |\n\
| measured rate of fire vs cooldown | {rof_ok} / {rof_n} |\n\
| measured damage per hit vs matrix | {hit_ok} / {hit_n} |\n\n\
Weapons read `damage+bonus cd<cooldown> r<range px> xN bullets`; rate of fire\n\
`measured/expected frames`; hit `measured/expected hp (x256)`; `-` means not\n\
measured (unarmed, turret subunit, hangar, suicide or splash-flame weapon:\n\
those are covered per case in the tests above).\n\n\
## Units\n\n\
| id | name | hp | shields | armor size | cost | time | supply | sight | ground | air | rate of fire | hit | tags | data |\n\
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    out.push_str(&rows);
    std::fs::write(format!("{ROOT}/docs/UNIT-COMPARISON.md"), out).unwrap();
    println!("data {data_ok}/{n} rof {rof_ok}/{rof_n} hit {hit_ok}/{hit_n}");
}
