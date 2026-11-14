//! Production time of every unit trained in a building, against
//! `units.dat` (OpenBW: the queued unit's `remaining_build_time` starts at
//! the build time and drops by one per frame).

use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::Terrain;
use galaxycraft::sim::World;
use galaxycraft::tech::build_info;
use std::sync::Arc;

#[test]
fn every_building_trained_unit_takes_its_units_dat_build_time() {
    let data = Arc::new(GameData::bundled().unwrap());
    let mut checked = 0;
    let mut bad = Vec::new();
    for ty in 0..228u16 {
        let Some(info) = build_info(ty) else {
            continue;
        };
        let producer = data.unit(info.producer);
        if !producer.is_building() || data.unit(ty).is_building() {
            continue;
        }
        let mut w = World::new(data.clone(), Terrain::open(64, 64), 2);
        w.players[0].minerals = 100_000;
        w.players[0].gas = 100_000;
        for t in 0..44 {
            w.players[0].techs[t] = true;
        }
        // Add-on prerequisites (Machine Shop, Control Tower) are not what is
        // measured here.
        w.players[0].cheats.build_anything = true;
        // Supply, power and the producer's requirements, all complete.
        for (k, depot) in [uid::SUPPLY_DEPOT, uid::PYLON, uid::OVERLORD]
            .iter()
            .enumerate()
        {
            for j in 0..6 {
                if let Ok(d) = w.spawn_px(0, *depot, 1200 + 80 * j, 200 + 120 * k as i32) {
                    w.units[d].completed = true;
                }
            }
        }
        let p = w.spawn_px(0, info.producer, 400, 400).unwrap();
        // Power for Protoss producers.
        if let Ok(py) = w.spawn_px(0, uid::PYLON, 400, 560) {
            w.units[py].completed = true;
        }
        w.units[p].completed = true;
        for (k, (req, _)) in info.required_units.iter().enumerate() {
            if let Ok(r) = w.spawn_px(0, *req, 700 + 120 * k as i32, 700) {
                w.units[r].completed = true;
            }
        }
        let before = w
            .units
            .iter()
            .filter(|u| u.alive && u.type_id == ty)
            .count();
        if let Err(e) = w.train(w.units[p].id, ty) {
            eprintln!("skip {ty}: {e:?}");
            continue;
        }
        let start = w.tick;
        let mut done = None;
        for _ in 0..3000 {
            w.step();
            if w.units
                .iter()
                .filter(|u| u.alive && u.type_id == ty)
                .count()
                > before
            {
                done = Some(w.tick - start);
                break;
            }
        }
        let want = data.unit(ty).build_time as u64;
        checked += 1;
        match done {
            Some(t) if t.abs_diff(want) <= 2 => {}
            other => bad.push(format!("type {ty}: {other:?} frames, units.dat {want}")),
        }
    }
    assert!(checked > 25, "only {checked} units trained");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
