//! Buildings, resources and spell effects for `docs/UNIT-COMPARISON.md`
//! (included by `unit_comparison.rs`).

use galaxycraft::data::GameData;
use galaxycraft::ids::{unit as uid, weapon as wid};
use galaxycraft::map::Terrain;
use galaxycraft::sim::{ENERGY_SCALE, Order, Spell, SpellTarget, World};
use galaxycraft::tech::build_info;
use std::fmt::Write as _;
use std::sync::Arc;

fn fresh(data: &Arc<GameData>) -> World {
    let mut w = World::new(data.clone(), Terrain::open(96, 96), 2);
    for p in 0..2 {
        w.players[p].minerals = 100_000;
        w.players[p].gas = 100_000;
        for t in 0..44 {
            w.players[p].techs[t] = true;
        }
    }
    w
}

fn ok(good: bool) -> &'static str {
    if good { "ok" } else { "**DIFF**" }
}

/// Frames from placement until a worker-built building completes.
fn construction(data: &Arc<GameData>, ty: u16, worker: u16) -> Option<u64> {
    let mut w = fresh(data);
    w.players[0].cheats.build_anything = true;
    let info = build_info(ty)?;
    for (k, (req, _)) in info.required_units.iter().enumerate() {
        if let Ok(r) = w.spawn_px(0, *req, 300 + 160 * k as i32, 1300) {
            w.units[r].completed = true;
        }
    }
    let site = (1000, 800);
    if worker == uid::PROBE {
        let p = w.spawn_px(0, uid::PYLON, 1000, 900).ok()?;
        w.units[p].completed = true;
    }
    if worker == uid::DRONE {
        let h = w.spawn_px(0, uid::HATCHERY, 1000, 1000).ok()?;
        w.units[h].completed = true;
        for _ in 0..1200 {
            w.step();
        }
    }
    let target = if ty == uid::EXTRACTOR || ty == uid::REFINERY || ty == uid::ASSIMILATOR {
        let g = w.spawn_px(0, uid::VESPENE_GEYSER, site.0, site.1).ok()?;
        w.units[g].resource_amount = 5000;
        (w.units[g].x / 256, w.units[g].y / 256)
    } else {
        site
    };
    let old: Vec<u32> = w.units.iter().map(|u| u.id).collect();
    let k = w.spawn_px(0, worker, 900, 700).ok()?;
    w.issue(
        w.units[k].id,
        Order::Build {
            unit_type: ty,
            pos: target,
        },
    )
    .ok()?;
    let mut start = None;
    for _ in 0..6000 {
        w.step();
        let b = w
            .units
            .iter()
            .find(|u| u.alive && u.owner == 0 && u.type_id == ty && !old.contains(&u.id));
        match (b, start) {
            (Some(_), None) => start = Some(w.tick),
            (Some(b), Some(s)) if b.completed => return Some(w.tick - s),
            _ => {}
        }
    }
    None
}

fn supply_given(data: &Arc<GameData>, ty: u16) -> Option<i32> {
    let mut w = fresh(data);
    let before = w.supply_cap(0);
    let b = w.spawn_px(0, ty, 1000, 1000).ok()?;
    w.units[b].completed = true;
    Some(w.supply_cap(0) - before)
}

pub fn buildings(data: &Arc<GameData>) -> String {
    let mut s = String::from(
        "## Buildings\n\nEach worker-built building is placed by a fresh worker on open \
ground (Zerg on creep, Protoss beside a Pylon); construction time is measured from \
placement to completion and compared with `units.dat`.  Supply is the cap change \
when a completed building appears (half-points, as `units.dat`).\n\n\
| id | building | construction measured/units.dat | supply measured/units.dat | detector | sight |\n|---|---|---|---|---|---|\n",
    );
    let (mut n, mut good) = (0, 0);
    for ty in 106..=202u16 {
        let u = data.unit(ty);
        let Some(info) = build_info(ty) else { continue };
        if !u.is_building() || !matches!(info.producer, uid::SCV | uid::PROBE | uid::DRONE) {
            continue;
        }
        let t = construction(data, ty, info.producer);
        let sup = supply_given(data, ty);
        // Zerg morphs consume the Drone, whose own supply comes back.
        let sup_ok = sup == Some(u.supply_provided as i32);
        let t_ok = t.is_some_and(|t| t.abs_diff(u.build_time as u64) <= 2);
        n += 1;
        good += (t_ok && sup_ok) as u32;
        let _ = writeln!(
            s,
            "| {ty} | {} | {}/{} {} | {}/{} {} | {} | {} |",
            data.unit_name(ty),
            t.map_or("-".into(), |t| t.to_string()),
            u.build_time,
            ok(t_ok),
            sup.map_or("-".into(), |v| v.to_string()),
            u.supply_provided,
            ok(sup_ok),
            if u.is_detector() { "yes" } else { "-" },
            u.sight_range,
        );
    }
    let _ = writeln!(
        s,
        "\nBuildings matching: {good} / {n}.  Buildings trained by other \
buildings (add-ons) and units trained in buildings are timed in \
`tests/build_times_retail.rs`.\n"
    );
    s
}

/// Resources banked by one worker's first two trips.
fn trips(data: &Arc<GameData>, worker: u16, depot: u16, gas: Option<(u16, i32)>) -> Vec<i32> {
    trips_from(data, worker, depot, gas, uid::MINERAL_FIELD)
}

fn trips_from(
    data: &Arc<GameData>,
    worker: u16,
    depot: u16,
    gas: Option<(u16, i32)>,
    field: u16,
) -> Vec<i32> {
    let mut w = fresh(data);
    w.players[0].minerals = 0;
    w.players[0].gas = 0;
    let d = w.spawn_px(0, depot, 400, 400).unwrap();
    w.units[d].completed = true;
    let res = match gas {
        Some((refinery, left)) => {
            let r = w.spawn_px(0, refinery, 400, 640).unwrap();
            w.units[r].completed = true;
            w.units[r].resource_amount = left;
            r
        }
        None => {
            let f = w.spawn_px(2, field, 400, 560).unwrap();
            w.units[f].resource_amount = 1500;
            f
        }
    };
    let k = w.spawn_px(0, worker, 400, 500).unwrap();
    let rid = w.units[res].id;
    let _ = w.issue(w.units[k].id, Order::Gather { resource: rid });
    let mut got = Vec::new();
    let mut last = 0;
    for _ in 0..3000 {
        w.step();
        let now = if gas.is_some() {
            w.players[0].gas
        } else {
            w.players[0].minerals
        };
        if now > last {
            got.push(now - last);
            last = now;
        }
        if got.len() >= 2 {
            break;
        }
    }
    got
}

pub fn resources(data: &Arc<GameData>) -> String {
    let mut s = String::from(
        "## Resources\n\nOne worker of each race mines a Mineral Field (unit 176) and a refinery, and an SCV the other two field types (177, 178); the \
amount banked on each of the first two trips is compared with OpenBW \
`gather_resource` (8 minerals, 8 gas, 2 gas once the geyser is depleted).\n\n\
| worker | minerals/trip | gas/trip | depleted gas/trip |\n|---|---|---|---|\n",
    );
    let (mut n, mut good) = (0, 0);
    for (worker, depot, refinery) in [
        (uid::SCV, uid::COMMAND_CENTER, uid::REFINERY),
        (uid::PROBE, uid::NEXUS, uid::ASSIMILATOR),
        (uid::DRONE, uid::HATCHERY, uid::EXTRACTOR),
    ] {
        let m = trips(data, worker, depot, None);
        let g = trips(data, worker, depot, Some((refinery, 5000)));
        let e = trips(data, worker, depot, Some((refinery, 0)));
        let cell = |v: &Vec<i32>, want: i32, n: &mut u32, good: &mut u32| {
            let o = v.len() == 2 && v.iter().all(|&x| x == want);
            *n += 1;
            *good += o as u32;
            format!("{v:?}/{want} {}", ok(o))
        };
        let row = [
            cell(&m, 8, &mut n, &mut good),
            cell(&g, 8, &mut n, &mut good),
            cell(&e, 2, &mut n, &mut good),
        ];
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} |",
            data.unit_name(worker),
            row[0],
            row[1],
            row[2]
        );
    }
    for f in [uid::MINERAL_FIELD_2, uid::MINERAL_FIELD_3] {
        let v = trips_from(data, uid::SCV, uid::COMMAND_CENTER, None, f);
        let o = v.len() == 2 && v.iter().all(|&x| x == 8);
        n += 1;
        good += o as u32;
        let _ = writeln!(
            s,
            "| {} {f} (SCV) | {v:?}/8 {} | | |",
            data.unit_name(f),
            ok(o)
        );
    }
    let field = {
        let mut w = fresh(data);
        let f = w.spawn_px(2, uid::MINERAL_FIELD, 400, 400).unwrap();
        w.units[f].resource_amount
    };
    let _ = writeln!(
        s,
        "\nResource trips matching: {good} / {n}.  A Mineral Field placed without a map value \
holds {field} minerals (maps set their own amounts in the `UNIT` section).\n"
    );
    s
}

fn cast(w: &mut World, caster: u16, spell: Spell, target: SpellTarget) -> usize {
    let c = w.spawn_px(0, caster, 400, 400).unwrap();
    w.units[c].set_energy_fixed(250 * ENERGY_SCALE);
    let target = match target {
        SpellTarget::Unit(i) => SpellTarget::Unit(w.units[i as usize].id),
        t => t,
    };
    w.cast(w.units[c].id, spell, target).unwrap();
    c
}

fn peak(w: &mut World, read: impl Fn(&World) -> i32) -> i32 {
    let mut best = read(w);
    for _ in 0..300 {
        w.step();
        best = best.max(read(w));
    }
    best
}

pub fn effects(data: &Arc<GameData>) -> String {
    let mut rows: Vec<(&str, i32, i32, &str)> = Vec::new();
    let pt = SpellTarget::Point(470, 400);
    let on = |victim: u16, owner: usize| {
        let mut w = fresh(data);
        let t = w.spawn_px(owner, victim, 470, 400).unwrap();
        (w, t)
    };
    let (mut w, t) = on(uid::SIEGE_TANK, 1);
    cast(
        &mut w,
        uid::GHOST,
        Spell::Lockdown,
        SpellTarget::Unit(t as u32),
    );
    rows.push((
        "Lockdown timer",
        peak(&mut w, |w| w.units[t].status.lockdown_ticks),
        131,
        "lockdown_timer",
    ));
    let (mut w, t) = on(uid::MARINE, 1);
    cast(&mut w, uid::ARBITER, Spell::StasisField, pt);
    rows.push((
        "Stasis Field timer",
        peak(&mut w, |w| w.units[t].status.stasis_ticks),
        131,
        "stasis_timer",
    ));
    let (mut w, t) = on(uid::MARINE, 1);
    cast(&mut w, uid::QUEEN, Spell::Ensnare, pt);
    rows.push((
        "Ensnare timer",
        peak(&mut w, |w| w.units[t].status.ensnare_ticks),
        75,
        "ensnare_timer",
    ));
    let (mut w, t) = on(uid::SIEGE_TANK, 1);
    let hp0 = w.units[t].hp;
    cast(&mut w, uid::DEFILER, Spell::Plague, pt);
    rows.push((
        "Plague timer",
        peak(&mut w, |w| w.units[t].status.plague_ticks),
        75,
        "plague_timer",
    ));
    let per = data.weapon(wid::PLAGUE).unwrap().damage as i32 * 256 / 76;
    let _ = hp0;
    let (mut w, t) = on(uid::SIEGE_TANK, 1);
    cast(&mut w, uid::DEFILER, Spell::Plague, pt);
    let mut hp = w.units[t].hp;
    let mut first = 0;
    for _ in 0..40 {
        w.step();
        if w.units[t].hp < hp && first == 0 {
            first = hp - w.units[t].hp;
        }
        hp = w.units[t].hp;
    }
    rows.push(("Plague damage per 8 frames (x256)", first, per, "damage/76"));
    let (mut w, t) = on(uid::ZERGLING, 1);
    cast(
        &mut w,
        uid::DARK_ARCHON,
        Spell::Maelstrom,
        SpellTarget::Unit(t as u32),
    );
    rows.push((
        "Maelstrom timer",
        peak(&mut w, |w| w.units[t].status.maelstrom_ticks),
        22,
        "maelstrom_timer",
    ));
    let (mut w, t) = on(uid::MARINE, 0);
    cast(
        &mut w,
        uid::SCIENCE_VESSEL,
        Spell::DefensiveMatrix,
        SpellTarget::Unit(t as u32),
    );
    rows.push((
        "Defensive Matrix hp",
        peak(&mut w, |w| w.units[t].status.matrix) >> 8,
        250,
        "defensive_matrix_hp",
    ));
    let (mut w, t) = on(uid::MARINE, 0);
    cast(
        &mut w,
        uid::SCIENCE_VESSEL,
        Spell::DefensiveMatrix,
        SpellTarget::Unit(t as u32),
    );
    rows.push((
        "Defensive Matrix timer",
        peak(&mut w, |w| w.units[t].status.matrix_ticks as i32),
        168,
        "defensive_matrix_timer",
    ));
    let (mut w, t) = on(uid::ZERGLING, 1);
    cast(
        &mut w,
        uid::SCIENCE_VESSEL,
        Spell::Irradiate,
        SpellTarget::Unit(t as u32),
    );
    let want = data.weapon(wid::IRRADIATE).unwrap().cooldown as i32;
    rows.push((
        "Irradiate timer",
        peak(&mut w, |w| w.units[t].status.irradiate_ticks),
        want,
        "weapon cooldown",
    ));
    let (mut w, t) = on(uid::SIEGE_TANK, 1);
    let hp = w.units[t].hp;
    cast(&mut w, uid::HIGH_TEMPLAR, Spell::PsionicStorm, pt);
    for _ in 0..300 {
        w.step();
    }
    let per = data.weapon(wid::PSIONIC_STORM).unwrap().damage as i32;
    rows.push((
        "Psionic Storm total hp",
        (hp - w.units[t].hp) >> 8,
        8 * per,
        "8 hits",
    ));
    let (mut w, t) = on(uid::HIGH_TEMPLAR, 1);
    w.units[t].set_energy_fixed(200 * ENERGY_SCALE);
    cast(&mut w, uid::SCIENCE_VESSEL, Spell::EmpShockwave, pt);
    let (mut e, mut sh) = (i32::MAX, i32::MAX);
    for _ in 0..200 {
        w.step();
        e = e.min(w.units[t].energy_fixed());
        sh = sh.min(w.units[t].shields);
    }
    rows.push(("EMP: energy left", e, 0, "emp_shockwave"));
    rows.push(("EMP: shields left", sh, 0, "emp_shockwave"));
    let (mut w, t) = on(uid::SUPPLY_DEPOT, 1);
    w.units[t].completed = true;
    let hp = w.units[t].hp;
    let c = w.spawn_px(0, uid::BATTLECRUISER, 270, 400).unwrap();
    w.units[c].set_energy_fixed(250 * ENERGY_SCALE);
    let tid = w.units[t].id;
    w.issue(
        w.units[c].id,
        Order::Cast {
            spell: Spell::YamatoGun,
            target: SpellTarget::Unit(tid),
        },
    )
    .unwrap();
    let (mut big, mut last) = (0, hp);
    for _ in 0..300 {
        w.step();
        big = big.max((last - w.units[t].hp) / 256);
        last = w.units[t].hp;
    }
    rows.push(("Yamato Gun hp on a Supply Depot", big, 259, "damage matrix"));
    let (mut n, mut good) = (0, 0);
    let mut s = String::from(
        "## Spell effects\n\nEach spell is cast in the simulation on a fresh target and the \
value it leaves is compared with the number OpenBW (`bwgame.h`) applies; status \
timers count 8-frame updates in both engines.  Hallucination, Broodling, Swarm, \
Web, Feedback, Consume, Restoration, Mind Control, Optical Flare, Parasite, Recall, \
Scanner Sweep and Spider Mines follow under *Other spells*; all are also asserted in \
`tests/effects_openbw.rs`.\n\n| effect | measured | OpenBW | rule |\n|---|---|---|---|\n",
    );
    for (name, got, want, rule) in rows {
        n += 1;
        good += (got == want) as u32;
        let _ = writeln!(
            s,
            "| {name} | {got} | {want} {} | `{rule}` |",
            ok(got == want)
        );
    }
    let _ = writeln!(s, "\nSpell effects matching: {good} / {n}.\n");
    s
}

/// hp damage (x256) of one hit from the executable's damage matrix.
fn oracle(weapon: u8, size: i64, armor: i64) -> Option<i32> {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/oracle_damage.tsv"
    ))
    .ok()?;
    text.lines().skip(1).find_map(|l| {
        let f: Vec<i64> = l.split('\t').filter_map(|x| x.parse().ok()).collect();
        (f.len() == 8 && f[0] == weapon as i64 && f[1..6] == [0, size, armor, 0, 0])
            .then_some(f[7] as i32)
    })
}

/// First hp loss of a `target` attacked by `ty` from `dist` px away.
fn first_hit(
    data: &Arc<GameData>,
    ty: u16,
    target: u16,
    dist: i32,
    prep: impl Fn(&mut World, usize),
) -> Option<i32> {
    let mut w = fresh(data);
    let m = w.spawn_px(0, ty, 400, 400).ok()?;
    let t = w.spawn_px(1, target, 400 + dist, 400).ok()?;
    w.units[m].completed = true;
    w.units[t].completed = true;
    prep(&mut w, m);
    let hp = w.units[t].hp;
    let (mid, tid) = (w.units[m].id, w.units[t].id);
    let _ = w.issue(mid, Order::AttackUnit { target: tid });
    for _ in 0..1500 {
        w.step();
        if !w.units[t].alive {
            return Some(hp);
        }
        if w.units[t].hp < hp {
            return Some(hp - w.units[t].hp);
        }
    }
    None
}

pub fn special_attackers(data: &Arc<GameData>) -> String {
    let mut s = String::from(
        "## Special attackers\n\nUnits whose damage does not come from one swing of their own \
weapon (turret bases, Siege Mode, hangars, suicide units, burrowed Lurkers): the first \
hit they land is compared with the executable's damage matrix for the weapon that \
actually fires.  Turret subunits (4, 6, 18, 24) fire through these rows.\n\n\
| id | unit | target | weapon | first hit measured/matrix (x256) |\n|---|---|---|---|---|\n",
    );
    let siege = |w: &mut World, m: usize| {
        let id = w.units[m].id;
        let _ = w.issue(id, Order::Siege);
        for _ in 0..120 {
            w.step();
        }
    };
    let burrow = |w: &mut World, m: usize| {
        let id = w.units[m].id;
        let _ = w.issue(id, Order::Burrow);
        for _ in 0..120 {
            w.step();
        }
    };
    let none = |_: &mut World, _: usize| {};
    let hangar = |w: &mut World, m: usize| {
        w.units[m].hangar = 4;
    };
    let depot = (uid::SUPPLY_DEPOT, 3, 1);
    let lord = (uid::OVERLORD, 3, 0);
    // (unit, target, distance, weapon id, prep)
    type Prep<'a> = &'a dyn Fn(&mut World, usize);
    let g = |t: u16| data.unit(t).ground_weapon;
    let a = |t: u16| data.unit(t).air_weapon;
    type Case<'a> = (u16, (u16, i64, i64), i32, u8, Prep<'a>);
    let cases: [Case; 7] = [
        (uid::SIEGE_TANK, depot, 250, g(data.unit(30).turret), &siege),
        (uid::GOLIATH, lord, 120, a(4), &none),
        (uid::CARRIER, depot, 200, g(73), &hangar),
        (uid::REAVER, depot, 150, g(85), &hangar),
        (uid::LURKER, depot, 120, g(uid::LURKER), &burrow),
        (uid::SCOURGE, lord, 80, a(uid::SCOURGE), &none),
        (
            uid::INFESTED_TERRAN,
            depot,
            80,
            g(uid::INFESTED_TERRAN),
            &none,
        ),
    ];
    let (mut n, mut good) = (0, 0);
    for (ty, (target, size, armor), dist, weapon, prep) in cases {
        let got = first_hit(data, ty, target, dist, prep);
        let want = oracle(weapon, size, armor);
        // A target that dies on the first hit reports its whole health.
        let full = data.unit(target).hit_points_raw as i32;
        let o = match (got, want) {
            (Some(g), Some(wv)) => g == wv || g == 2 * wv || (g == full && wv >= full),
            _ => false,
        };
        n += 1;
        good += o as u32;
        let _ = writeln!(
            s,
            "| {ty} | {} | {} | {weapon} | {}/{} {} |",
            data.unit_name(ty),
            data.unit_name(target),
            got.map_or("-".into(), |v| v.to_string()),
            want.map_or("-".into(), |v| v.to_string()),
            ok(o),
        );
    }
    let _ = writeln!(
        s,
        "\nSpecial attackers matching: {good} / {n}.  Flame weapons \
(Firebat, Gui Montag, flame traps) release three splash flames per attack and are \
checked against OpenBW `attkshiftproj` in `tests/rate_of_fire_retail.rs`.\n"
    );
    s
}

/// Frames an add-on or a Zerg building morph takes, started with `train`
/// on a completed producer (add-on requirements unlocked by the cheat).
fn morph_time(data: &Arc<GameData>, ty: u16) -> Option<u64> {
    let info = build_info(ty)?;
    let mut w = fresh(data);
    w.players[0].cheats.build_anything = true;
    for (k, (req, _)) in info.required_units.iter().enumerate() {
        if let Ok(r) = w.spawn_px(0, *req, 300 + 160 * k as i32, 1500) {
            w.units[r].completed = true;
        }
    }
    if matches!(
        info.producer,
        uid::HATCHERY | 132 | 133 | uid::CREEP_COLONY | 141
    ) {
        let h = w.spawn_px(0, uid::HATCHERY, 1500, 1500).ok()?;
        w.units[h].completed = true;
        for _ in 0..600 {
            w.step();
        }
    }
    let p = w.spawn_px(0, info.producer, 1000, 1000).ok()?;
    w.units[p].completed = true;
    let old: Vec<u32> = w
        .units
        .iter()
        .filter(|u| u.type_id == ty)
        .map(|u| u.id)
        .collect();
    let pid = w.units[p].id;
    w.train(pid, ty).ok()?;
    let start = w.tick;
    for _ in 0..4000 {
        w.step();
        if w.units
            .iter()
            .any(|u| u.alive && u.type_id == ty && u.completed && !old.contains(&u.id))
        {
            return Some(w.tick - start);
        }
    }
    None
}

pub fn addons_and_morphs(data: &Arc<GameData>) -> String {
    let mut s = String::from(
        "## Add-ons and Zerg building morphs\n\nBuilt by a completed producer \
(Command Center, Factory, Starport, Science Facility, Hatchery, Lair, Spire, Creep \
Colony): frames from the order to the finished building, against `units.dat`.\n\n\
| id | building | producer | time measured/units.dat |\n|---|---|---|---|\n",
    );
    let (mut n, mut good) = (0, 0);
    for ty in [107u16, 108, 115, 117, 118, 120, 132, 133, 137, 144, 146] {
        let Some(info) = build_info(ty) else { continue };
        let u = data.unit(ty);
        let t = morph_time(data, ty);
        let o = t.is_some_and(|t| t.abs_diff(u.build_time as u64) <= 2);
        n += 1;
        good += o as u32;
        let _ = writeln!(
            s,
            "| {ty} | {} | {} | {}/{} {} |",
            data.unit_name(ty),
            data.unit_name(info.producer),
            t.map_or("-".into(), |t| t.to_string()),
            u.build_time,
            ok(o)
        );
    }
    let _ = writeln!(s, "\nAdd-ons and morphs matching: {good} / {n}.\n");
    s
}

/// Frames left on the first unit of `ty` the moment it appears.
fn spawned_lifetime(w: &mut World, ty: u16) -> i32 {
    for _ in 0..300 {
        w.step();
        if let Some(u) = w.units.iter().find(|u| u.alive && u.type_id == ty) {
            return (u.expires_at - w.tick as i64) as i32 + 1;
        }
    }
    -1
}

pub fn effects_more(data: &Arc<GameData>) -> String {
    let mut rows: Vec<(&str, i32, i32, &str)> = Vec::new();
    let pt = SpellTarget::Point(470, 400);
    let on = |victim: u16, owner: usize| {
        let mut w = fresh(data);
        let t = w.spawn_px(owner, victim, 470, 400).unwrap();
        (w, t)
    };
    // Hallucination: two copies owned by the caster, 1350 frames.
    let (mut w, t) = on(uid::ZERGLING, 1);
    cast(
        &mut w,
        uid::HIGH_TEMPLAR,
        Spell::Hallucination,
        SpellTarget::Unit(t as u32),
    );
    let copies: Vec<i64> = w
        .units
        .iter()
        .filter(|u| u.alive && u.status.hallucination && u.owner == 0)
        .map(|u| u.expires_at - w.tick as i64)
        .collect();
    rows.push((
        "Hallucination copies",
        copies.len() as i32,
        2,
        "order_CastHallucination",
    ));
    rows.push((
        "Hallucination lifetime",
        copies.first().copied().unwrap_or(-1) as i32,
        1350,
        "default_remove_timer",
    ));
    let (mut w, t) = on(uid::MARINE, 1);
    cast(
        &mut w,
        uid::QUEEN,
        Spell::SpawnBroodlings,
        SpellTarget::Unit(t as u32),
    );
    rows.push((
        "Broodling lifetime",
        spawned_lifetime(&mut w, uid::BROODLING),
        1800,
        "default_remove_timer",
    ));
    let mut w = fresh(data);
    cast(&mut w, uid::DEFILER, Spell::DarkSwarm, pt);
    rows.push((
        "Dark Swarm lifetime",
        spawned_lifetime(&mut w, uid::DARK_SWARM),
        900,
        "dark_swarm",
    ));
    let mut w = fresh(data);
    cast(&mut w, uid::CORSAIR, Spell::DisruptionWeb, pt);
    rows.push((
        "Disruption Web lifetime",
        spawned_lifetime(&mut w, uid::DISRUPTION_WEB),
        360,
        "disruption_web",
    ));
    let mut w = fresh(data);
    let cc = w.spawn_px(0, uid::COMMAND_CENTER, 300, 300).unwrap();
    w.units[cc].completed = true;
    let cs = w.spawn_px(0, uid::COMSAT_STATION, 380, 320).unwrap();
    w.units[cs].completed = true;
    w.units[cs].addon_attached = true;
    w.units[cs].set_energy_fixed(200 * ENERGY_SCALE);
    w.cast(
        w.units[cs].id,
        Spell::ScannerSweep,
        SpellTarget::Point(1500, 1500),
    )
    .unwrap();
    rows.push((
        "Scanner Sweep lifetime",
        spawned_lifetime(&mut w, uid::SCANNER_SWEEP),
        156,
        "iscript",
    ));
    // Feedback: damage equal to the target's energy.
    let (mut w, t) = on(uid::SCIENCE_VESSEL, 1);
    w.units[t].set_energy_fixed(100 * ENERGY_SCALE);
    let hp = w.units[t].hp;
    cast(
        &mut w,
        uid::DARK_ARCHON,
        Spell::Feedback,
        SpellTarget::Unit(t as u32),
    );
    for _ in 0..100 {
        w.step();
    }
    rows.push((
        "Feedback damage (100 energy)",
        (hp - w.units[t].hp) >> 8,
        100,
        "order_Feedback",
    ));
    // Consume: +50 energy.
    let mut w = fresh(data);
    let c = w.spawn_px(0, uid::DEFILER, 400, 400).unwrap();
    let z = w.spawn_px(0, uid::ZERGLING, 410, 400).unwrap();
    w.units[c].set_energy_fixed(100 * ENERGY_SCALE);
    w.cast(
        w.units[c].id,
        Spell::Consume,
        SpellTarget::Unit(w.units[z].id),
    )
    .unwrap();
    // Read before regeneration adds to it: the first frame it jumps.
    let e0 = 100 * ENERGY_SCALE;
    let mut gain = (w.units[c].energy_fixed() - e0) / ENERGY_SCALE;
    for _ in 0..100 {
        if gain >= 40 {
            break;
        }
        w.step();
        gain = (w.units[c].energy_fixed() - e0) / ENERGY_SCALE;
    }
    rows.push(("Consume energy gained", gain, 50, "consume"));
    // Restoration clears every status.
    let (mut w, t) = on(uid::MARINE, 0);
    {
        let st = &mut w.units[t].status;
        st.ensnare_ticks = 50;
        st.plague_ticks = 50;
        st.lockdown_ticks = 50;
        st.maelstrom_ticks = 10;
        st.optical_flare_ticks = 100;
    }
    cast(
        &mut w,
        uid::MEDIC,
        Spell::Restoration,
        SpellTarget::Unit(t as u32),
    );
    for _ in 0..60 {
        w.step();
    }
    let st = &w.units[t].status;
    let left = [
        st.ensnare_ticks,
        st.plague_ticks,
        st.lockdown_ticks,
        { st.maelstrom_ticks },
        { st.optical_flare_ticks },
    ]
    .iter()
    .filter(|&&v| v > 0)
    .count() as i32;
    rows.push(("Restoration: effects left of 5", left, 0, "restore_unit"));
    // Mind Control: owner changes, caster shields to 0.
    let (mut w, t) = on(uid::MARINE, 1);
    let c = cast(
        &mut w,
        uid::DARK_ARCHON,
        Spell::MindControl,
        SpellTarget::Unit(t as u32),
    );
    let mut shields = i32::MAX;
    for _ in 0..60 {
        w.step();
        shields = shields.min(w.units[c].shields);
    }
    rows.push((
        "Mind Control: new owner",
        w.units[t].owner as i32,
        0,
        "order_MindControl",
    ));
    rows.push((
        "Mind Control: caster shields (lowest)",
        shields >> 8,
        0,
        "order_MindControl",
    ));
    let (mut w, t) = on(uid::MARINE, 1);
    cast(
        &mut w,
        uid::MEDIC,
        Spell::OpticalFlare,
        SpellTarget::Unit(t as u32),
    );
    for _ in 0..60 {
        w.step();
    }
    rows.push((
        "Optical Flare blinds (1 = yes)",
        (w.units[t].status.optical_flare_ticks > 0) as i32,
        1,
        "blind_unit",
    ));
    let (mut w, t) = on(uid::MARINE, 1);
    cast(
        &mut w,
        uid::QUEEN,
        Spell::Parasite,
        SpellTarget::Unit(t as u32),
    );
    for _ in 0..120 {
        w.step();
    }
    rows.push((
        "Parasite marks (1 = yes)",
        w.units[t].status.parasite as i32,
        1,
        "parasite_flags",
    ));
    // Recall: own unit in the 64 px square comes to the Arbiter.
    let mut w = fresh(data);
    let a = w.spawn_px(0, uid::ARBITER, 300, 300).unwrap();
    let near = w.spawn_px(0, uid::MARINE, 1200, 1200).unwrap();
    w.units[a].set_energy_fixed(250 * ENERGY_SCALE);
    w.cast(w.units[a].id, Spell::Recall, SpellTarget::Point(1200, 1200))
        .unwrap();
    for _ in 0..60 {
        w.step();
    }
    let d = ((w.units[near].x - w.units[a].x).abs() + (w.units[near].y - w.units[a].y).abs()) / 256;
    rows.push((
        "Recall brings the unit (1 = yes)",
        (d < 120) as i32,
        1,
        "order_Recall",
    ));
    // Spider Mines: three per Vulture.
    let mut w = fresh(data);
    let v = w.spawn_px(0, uid::VULTURE, 400, 400).unwrap();
    rows.push((
        "Spider Mines per Vulture",
        w.units[v].hangar as i32,
        3,
        "hangar",
    ));
    let mut s = String::from(
        "### Other spells\n\n| effect | measured | OpenBW | rule |\n|---|---|---|---|\n",
    );
    let (mut n, mut good) = (0, 0);
    for (name, got, want, rule) in rows {
        // Spawned effects appear a few frames after the cast.
        let o = if name.ends_with("lifetime") {
            (want - 10..=want).contains(&got)
        } else {
            got == want
        };
        n += 1;
        good += o as u32;
        let _ = writeln!(s, "| {name} | {got} | {want} {} | `{rule}` |", ok(o));
    }
    let _ = writeln!(
        s,
        "\nOther spells matching: {good} / {n}.  Lifetimes of spawned effects are read \
the frame they appear (up to 10 frames after the cast, the casting swing).\n"
    );
    s
}
