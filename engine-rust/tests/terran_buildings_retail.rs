//! Terran buildings against the retail data (units.dat add-on positions,
//! iscript LiftOff / Landing) and OpenBW's rules (`order_LiftingOff`,
//! `order_BuildingLand`, `building_abandon_addon`, `find_connecting_addon`,
//! `unit_can_build`).

use std::sync::Arc;

use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::Terrain;
use galaxycraft::sim::{Order, World};
use galaxycraft::unit_anim::{LIFT_HEIGHT, building_lift_frame, lift_offset, lift_phase_ticks};

fn world() -> World {
    let mut w = World::new(
        Arc::new(GameData::bundled().expect("data")),
        Terrain::open(64, 64),
        2,
    );
    w.players[0].minerals = 10_000;
    w.players[0].gas = 10_000;
    w
}

fn fp(v: i32) -> i32 {
    v * 256
}

/// A finished building of `ty` at the pixel centre `at`.
fn building(w: &mut World, ty: u16, at: (i32, i32)) -> u32 {
    let i = w.spawn_px(0, ty, at.0, at.1).expect("spawn");
    w.units[i].id
}

fn unit(w: &World, id: u32) -> &galaxycraft::sim::SimUnit {
    w.unit(id).expect("unit")
}

fn finish_addon(w: &mut World, parent: u32, addon: u16) -> u32 {
    w.build_addon(parent, addon).expect("addon");
    let id = unit(w, parent).addon.expect("linked");
    for _ in 0..2000 {
        if unit(w, id).completed {
            break;
        }
        w.step();
    }
    assert!(unit(w, id).completed, "add-on finished");
    id
}

#[test]
fn addons_sit_at_the_units_dat_addon_position() {
    // units.dat: every add-on is placed 128 px right and 32 px down from
    // its parent's placement box (128 x 96); add-ons are 64 x 64.
    let d = GameData::bundled().expect("data");
    for addon in [
        uid::COMSAT_STATION,
        uid::NUCLEAR_SILO,
        uid::MACHINE_SHOP,
        uid::CONTROL_TOWER,
        uid::COVERT_OPS,
        uid::PHYSICS_LAB,
    ] {
        let a = d.unit(addon);
        assert_eq!(
            (a.addon_position.x, a.addon_position.y),
            (128, 32),
            "{addon}"
        );
        assert_eq!((a.placement.x, a.placement.y), (64, 64), "{addon}");
    }
    let mut w = world();
    let factory = building(&mut w, uid::FACTORY, (560, 560));
    let shop = finish_addon(&mut w, factory, uid::MACHINE_SHOP);
    // Parent box top-left (496, 512) + (128, 32) + half the add-on (32, 32).
    assert_eq!((unit(&w, shop).x, unit(&w, shop).y), (fp(656), fp(576)));
}

#[test]
fn a_building_rises_42_px_and_takes_no_orders_while_it_lifts() {
    let mut w = world();
    let rax = building(&mut w, uid::BARRACKS, (576, 560));
    w.issue(rax, Order::Lift).unwrap();
    let total = lift_phase_ticks(&w.data, uid::BARRACKS, false);
    // Barracks LiftOff: four frames 8 ticks apart, then `sigorder 16`;
    // the 42 px climb at 1 px per tick is the longer of the two.
    assert_eq!(total, LIFT_HEIGHT as u32);
    w.step();
    assert!(unit(&w, rax).lifted);
    assert_eq!(unit(&w, rax).y, fp(559));
    assert!(
        w.issue(rax, Order::Move { pos: (900, 900) }).is_err(),
        "not interruptible"
    );
    // The first LiftOff frame is 5 (legs folding).
    assert_eq!(building_lift_frame(&w.data, unit(&w, rax)), Some(5));
    for _ in 0..60 {
        w.step();
    }
    let u = unit(&w, rax);
    assert_eq!(u.y, fp(560 - LIFT_HEIGHT));
    assert_eq!(u.lift_phase, None);
    assert_eq!(u.order, Order::Idle);
    assert_eq!(lift_offset(u), LIFT_HEIGHT);
    // In the air it holds the last LiftOff frame (4).
    assert_eq!(building_lift_frame(&w.data, u), Some(4));
}

#[test]
fn a_lifted_building_flies_at_one_pixel_per_tick() {
    // OpenBW `order_LiftingOff` leaves `flingy_top_speed` at 1 for buildings.
    let mut w = world();
    let rax = building(&mut w, uid::BARRACKS, (576, 560));
    w.issue(rax, Order::Lift).unwrap();
    for _ in 0..60 {
        w.step();
    }
    w.issue(rax, Order::Move { pos: (900, 518) }).unwrap();
    for _ in 0..40 {
        w.step();
    }
    let dx = unit(&w, rax).x / 256 - 576;
    assert!((30..=40).contains(&dx), "moved {dx} px in 40 ticks");
}

#[test]
fn a_landing_hovers_over_the_spot_and_sets_down_42_px() {
    let mut w = world();
    let rax = building(&mut w, uid::BARRACKS, (576, 560));
    w.issue(rax, Order::Lift).unwrap();
    for _ in 0..60 {
        w.step();
    }
    w.issue(rax, Order::Land { pos: (576, 560) }).unwrap();
    let mut seen_phase = false;
    let mut ticks = 0;
    while unit(&w, rax).lifted && ticks < 400 {
        w.step();
        ticks += 1;
        seen_phase |= unit(&w, rax).lift_phase.is_some();
    }
    assert!(seen_phase, "the set-down phase ran");
    let u = unit(&w, rax);
    assert!(!u.lifted);
    assert_eq!((u.x, u.y), (fp(576), fp(560)));
    // Barracks Landing: wait 15, four frames 8 ticks apart, then sigorder,
    // counted from the tick it starts: 48 ticks, longer than the 42 px
    // descent.
    assert_eq!(lift_phase_ticks(&w.data, uid::BARRACKS, true), 48);
    assert_eq!(building_lift_frame(&w.data, u), None, "grounded frame");
}

#[test]
fn lifting_abandons_the_addon_and_landing_on_its_spot_takes_it_back() {
    let mut w = world();
    let factory = building(&mut w, uid::FACTORY, (576, 560));
    let shop = finish_addon(&mut w, factory, uid::MACHINE_SHOP);
    w.issue(factory, Order::Lift).unwrap();
    for _ in 0..60 {
        w.step();
    }
    assert!(unit(&w, shop).is_detached_addon());
    assert_eq!(unit(&w, factory).addon, None);
    // An abandoned add-on researches nothing.
    assert!(
        w.research(shop, galaxycraft::ids::tech::TANK_SIEGE_MODE)
            .is_err()
    );
    // Landing one tile off does not reconnect it.
    w.issue(factory, Order::Land { pos: (544, 560) }).unwrap();
    for _ in 0..300 {
        w.step();
    }
    assert!(!unit(&w, factory).lifted);
    assert!(unit(&w, shop).is_detached_addon());
    // Back on the original spot it reconnects.
    w.issue(factory, Order::Lift).unwrap();
    for _ in 0..60 {
        w.step();
    }
    w.issue(factory, Order::Land { pos: (576, 560) }).unwrap();
    for _ in 0..300 {
        w.step();
    }
    assert!(!unit(&w, factory).lifted);
    assert_eq!(unit(&w, factory).addon, Some(shop));
    assert!(!unit(&w, shop).is_detached_addon());
}

#[test]
fn another_players_building_takes_over_an_abandoned_addon() {
    let mut w = world();
    let factory = building(&mut w, uid::FACTORY, (576, 560));
    let shop = finish_addon(&mut w, factory, uid::MACHINE_SHOP);
    w.issue(factory, Order::Lift).unwrap();
    for _ in 0..60 {
        w.step();
    }
    w.issue(factory, Order::Move { pos: (1500, 1500) }).unwrap();
    for _ in 0..1200 {
        w.step();
    }
    let i = w.spawn_px(1, uid::FACTORY, 300, 300).unwrap();
    let theirs = w.units[i].id;
    w.lift_building(theirs).unwrap();
    w.land_building(theirs, (576, 560)).unwrap();
    assert_eq!(unit(&w, theirs).addon, Some(shop));
    assert_eq!(unit(&w, shop).owner, 1);
}

#[test]
fn tanks_and_starport_ships_need_the_addon_on_their_own_producer() {
    let mut w = world();
    building(&mut w, uid::COMMAND_CENTER, (1500, 300));
    building(&mut w, uid::ARMORY, (900, 900));
    building(&mut w, uid::SCIENCE_FACILITY, (900, 1100));
    let f1 = building(&mut w, uid::FACTORY, (300, 300));
    let f2 = building(&mut w, uid::FACTORY, (300, 600));
    finish_addon(&mut w, f2, uid::MACHINE_SHOP);
    // A Machine Shop on another Factory does not count.
    assert!(w.train(f1, uid::SIEGE_TANK).is_err());
    assert!(w.train(f2, uid::SIEGE_TANK).is_ok());
    let s1 = building(&mut w, uid::STARPORT, (300, 900));
    let s2 = building(&mut w, uid::STARPORT, (300, 1200));
    finish_addon(&mut w, s2, uid::CONTROL_TOWER);
    for ship in [uid::DROPSHIP, uid::SCIENCE_VESSEL, uid::VALKYRIE] {
        assert!(w.train(s1, ship).is_err(), "{ship} without a tower");
    }
    assert!(w.train(s1, uid::WRAITH).is_ok());
    assert!(w.train(s2, uid::DROPSHIP).is_ok());
    // Nothing trains while the building constructs its add-on.
    let f3 = building(&mut w, uid::FACTORY, (300, 1500));
    w.build_addon(f3, uid::MACHINE_SHOP).unwrap();
    assert!(w.train(f3, uid::VULTURE).is_err());
}
