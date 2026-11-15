//! Retail skirmish computers run their race's melee script (`TMCx`, `ZMCx`,
//! `PMCx`).  Jump operands in `aiscript.bin` are absolute file offsets; the
//! scripts used to stop after `start_town` because every jump missed.

use galaxycraft::aiscript::{melee_ai, melee_script_id};
use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::Terrain;
use galaxycraft::sim::{Race, World};
use std::sync::Arc;

fn town(race: Race, hall: u16, worker: u16, building: u16, ticks: u64) {
    let mut w = World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(128, 128),
        2,
    );
    w.players[0].race = race;
    w.players[0].minerals = 2000;
    w.players[0].gas = 1000;
    w.players[0].is_computer = true;
    w.spawn_px(0, hall, 640, 640).unwrap();
    if race == Race::Zerg {
        w.spawn_px(0, uid::OVERLORD, 700, 600).unwrap();
    }
    for k in 0..4 {
        w.spawn_px(0, worker, 600 + k * 24, 720).unwrap();
    }
    for k in 0..6 {
        let m = w
            .spawn_px(0, uid::MINERAL_FIELD, 520 + k * 40, 820)
            .unwrap();
        w.units[m].resource_amount = 1500;
    }
    let g = w.spawn_px(0, uid::VESPENE_GEYSER, 800, 520).unwrap();
    w.units[g].resource_amount = 5000;
    w.spawn_px(1, uid::COMMAND_CENTER, 3600, 3600).unwrap();
    let mut ai = melee_ai(&w, 0).expect("melee script");
    assert_eq!(Some(ai.id), melee_script_id(race));
    for _ in 0..ticks {
        ai.update(&mut w, Some((3600, 3600)));
        w.step();
    }
    assert!(
        w.units
            .iter()
            .any(|u| u.alive && u.owner == 0 && u.type_id == building),
        "{race:?} melee script builds unit {building} (pc {} of {})",
        ai.pc,
        ai.instructions.len()
    );
}

#[test]
fn terran_melee_script_builds_a_barracks() {
    town(
        Race::Terran,
        uid::COMMAND_CENTER,
        uid::SCV,
        uid::BARRACKS,
        6000,
    );
}

#[test]
fn zerg_melee_script_builds_a_spawning_pool() {
    town(
        Race::Zerg,
        uid::HATCHERY,
        uid::DRONE,
        uid::SPAWNING_POOL,
        6000,
    );
}

#[test]
fn protoss_melee_script_builds_an_assimilator() {
    town(
        Race::Protoss,
        uid::NEXUS,
        uid::PROBE,
        uid::ASSIMILATOR,
        9000,
    );
}
