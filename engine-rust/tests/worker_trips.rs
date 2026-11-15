//! Worker trips measured against the reference replay (`docs/FIDELITY.md`
//! item 4): the builder of a Refinery harvests it, and a worker walks to the
//! side of the mineral field that faces it.

use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::Terrain;
use galaxycraft::sim::{Order, World};
use std::sync::Arc;

fn world() -> World {
    World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(64, 64),
        2,
    )
}

#[test]
fn the_builder_of_a_refinery_starts_harvesting_it() {
    // OpenBW `order_ConstructingBuilding`: `unit_can_gather_gas` -> Harvest1.
    let mut w = world();
    let cc = w.spawn_px(0, uid::COMMAND_CENTER, 600, 200).unwrap();
    w.units[cc].completed = true;
    w.spawn_px(11, uid::VESPENE_GEYSER, 600, 480).unwrap();
    let scv = w.spawn_px(0, uid::SCV, 600, 330).unwrap();
    let id = w.units[scv].id;
    w.players[0].minerals = 100;
    let spot = w.find_build_spot(uid::REFINERY, (600, 480)).unwrap();
    w.issue(
        id,
        Order::Build {
            unit_type: uid::REFINERY,
            pos: spot,
        },
    )
    .unwrap();
    let mut gathering = false;
    for _ in 0..3000 {
        w.step();
        if matches!(w.units[scv].order, Order::Gather { .. }) {
            gathering = true;
            break;
        }
    }
    assert!(gathering, "builder ended with {:?}", w.units[scv].order);
    let mut gas = 0;
    for _ in 0..600 {
        w.step();
        gas = w.players[0].gas;
    }
    assert!(gas > 0, "the builder brings gas");
}

#[test]
fn a_worker_mines_from_the_side_of_the_field_that_faces_it() {
    // Challenger replay: an SCV sent from the Command Center to a field of
    // the second row walked round the whole line and mined from behind it.
    let mut w = world();
    let cc = w.spawn_px(0, uid::COMMAND_CENTER, 640, 300).unwrap();
    w.units[cc].completed = true;
    let mut fields = Vec::new();
    for (x, y) in [(576, 560), (640, 560), (704, 560), (608, 592), (672, 592)] {
        let f = w.spawn_px(11, uid::MINERAL_FIELD, x, y).unwrap();
        w.units[f].resource_amount = 1500;
        fields.push(f);
    }
    // Second row, right below the gap between two front fields.
    let target = fields[3];
    let fid = w.units[target].id;
    let scv = w.spawn_px(0, uid::SCV, 600, 380).unwrap();
    let id = w.units[scv].id;
    w.issue(id, Order::Gather { resource: fid }).unwrap();
    let mut started = None;
    for t in 0..400 {
        w.step();
        if w.units[scv].gather_timer > 0 {
            started = Some(t);
            break;
        }
    }
    let t = started.expect("the SCV starts mining");
    let (sy, fy) = (w.units[scv].y / 256, w.units[target].y / 256);
    assert!(sy < fy, "mines from the front (scv y {sy}, field y {fy})");
    assert!(t < 120, "a short trip, not a detour: {t} frames");
}
