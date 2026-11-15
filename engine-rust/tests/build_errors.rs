//! A worker that reaches its build site without the money drops the order
//! and reports the error line (OpenBW `order_PlaceBuilding`:
//! `player_has_supply_and_resources_for(.., true)` then `order_done`).

use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::Terrain;
use galaxycraft::sim::{Order, SimError, World};
use std::sync::Arc;

#[test]
fn build_refused_on_arrival_reports_not_enough_minerals() {
    let mut w = World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(64, 64),
        2,
    );
    let scv = w.spawn_px(0, uid::SCV, 200, 200).unwrap();
    w.units[scv].completed = true;
    let id = w.units[scv].id;
    w.players[0].minerals = 100;
    let spot = w.find_build_spot(uid::SUPPLY_DEPOT, (800, 800)).unwrap();
    w.issue(
        id,
        Order::Build {
            unit_type: uid::SUPPLY_DEPOT,
            pos: spot,
        },
    )
    .unwrap();
    // The money is spent elsewhere while the SCV walks.
    w.players[0].minerals = 40;
    for _ in 0..600 {
        w.step();
    }
    assert!(
        !w.units
            .iter()
            .any(|u| u.alive && u.type_id == uid::SUPPLY_DEPOT),
        "no depot without the money"
    );
    assert!(w.units[scv].alive, "the SCV is kept");
    assert_eq!(w.order_error_seq, 1);
    assert_eq!(
        w.order_errors.back(),
        Some(&(0, SimError::NotEnoughMinerals))
    );
}
