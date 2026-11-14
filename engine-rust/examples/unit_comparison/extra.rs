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
            let f = w.spawn_px(2, uid::MINERAL_FIELD, 400, 560).unwrap();
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
        "## Resources\n\nOne worker of each race mines a Mineral Field and a refinery; the \
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
    let field = {
        let mut w = fresh(data);
        let f = w.spawn_px(2, uid::MINERAL_FIELD, 400, 400).unwrap();
        w.units[f].resource_amount
    };
    let _ = writeln!(
        s,
        "\nTrips matching: {good} / {n}.  A Mineral Field placed without a map value \
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
Scanner Sweep and Spider Mines are asserted the same way in \
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
    let _ = writeln!(s, "\nEffects matching: {good} / {n}.\n");
    s
}
