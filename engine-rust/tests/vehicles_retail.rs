//! Vulture, Siege Tank, Goliath and the Terran air units against the retail
//! data and the OpenBW source (see docs/UNIT-FIDELITY-METHOD.md).

use std::sync::Arc;

use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::Terrain;
use galaxycraft::sim::World;

fn world() -> World {
    World::new(
        Arc::new(GameData::bundled().expect("data")),
        Terrain::open(64, 64),
        2,
    )
}

/// Hit points an idle `attacker` takes off a passive target in `frames`.
fn damage_done(attacker: u16, target: u16, gap: i32, frames: usize) -> i32 {
    let mut w = world();
    let _a = w.spawn_px(0, attacker, 400, 400).unwrap();
    let t = w.spawn_px(1, target, 400 + gap, 400).unwrap();
    let hp = w.units[t].hp;
    for _ in 0..frames {
        w.step();
    }
    (hp - w.units[t].hp) / 256
}

#[test]
fn tanks_and_goliaths_fire_their_turret_weapons() {
    // Passive targets: a Zerg Egg on the ground, an Overlord in the air.
    assert!(damage_done(uid::SIEGE_TANK, uid::EGG, 150, 120) > 0, "tank");
    assert!(
        damage_done(uid::SIEGE_MODE, uid::EGG, 250, 200) > 0,
        "siege"
    );
    assert!(
        damage_done(uid::GOLIATH, uid::EGG, 120, 120) > 0,
        "goliath ground"
    );
    assert!(
        damage_done(uid::GOLIATH, uid::OVERLORD, 120, 120) > 0,
        "goliath air"
    );
}

/// Pixels a unit covers in `frames` at cruise (after `warmup` frames).
fn cruise(w: &mut World, unit: usize, warmup: usize, frames: usize) -> i32 {
    let id = w.units[unit].id;
    let x = w.units[unit].x / 256;
    w.issue(
        id,
        galaxycraft::sim::Order::Move {
            pos: (x + 1500, w.units[unit].y / 256),
        },
    )
    .unwrap();
    for _ in 0..warmup {
        w.step();
    }
    let x0 = w.units[unit].x;
    for _ in 0..frames {
        w.step();
    }
    (w.units[unit].x - x0) / 256
}

#[test]
fn ion_thrusters_make_the_vulture_half_again_as_fast() {
    use galaxycraft::ids::upgrade;
    // flingy.dat row 88: top speed 1707 (6.67 px per frame).
    let mut w = world();
    let v = w.spawn_px(0, uid::VULTURE, 200, 1000).unwrap();
    let plain = cruise(&mut w, v, 40, 30);
    assert!((198..=202).contains(&plain), "30 frames at 6.67: {plain}");
    let mut w = world();
    w.players[0].upgrades[upgrade::ION_THRUSTERS as usize] = 1;
    let v = w.spawn_px(0, uid::VULTURE, 200, 1000).unwrap();
    assert!(w.has_speed_upgrade(v));
    let fast = cruise(&mut w, v, 40, 30);
    // 1707 * 1.5 = 2560 (10 px per frame).
    assert!((298..=302).contains(&fast), "30 frames at 10: {fast}");
    // The same rule: Metabolic Boost for Zerglings, not for Drones.
    let mut w = world();
    w.players[0].upgrades[upgrade::METABOLIC_BOOST as usize] = 1;
    let ling = w.spawn_px(0, uid::ZERGLING, 200, 1000).unwrap();
    let drone = w.spawn_px(0, uid::DRONE, 200, 1100).unwrap();
    assert!(w.has_speed_upgrade(ling) && !w.has_speed_upgrade(drone));
}

#[test]
fn siege_mode_takes_its_animation_time_both_ways() {
    use galaxycraft::ids::tech;
    use galaxycraft::sim::{Order, SIEGE_FRAMES, UNSIEGE_FRAMES};
    let mut w = world();
    w.players[0].techs[tech::TANK_SIEGE_MODE as usize] = true;
    let tank = w.spawn_px(0, uid::SIEGE_TANK, 400, 400).unwrap();
    // In the Arclite cannon's range (64..384 px) but not the tank cannon's.
    let egg = w.spawn_px(1, uid::EGG, 700, 400).unwrap();
    let id = w.units[tank].id;
    let hp = w.units[egg].hp;
    w.issue(id, Order::Siege).unwrap();
    w.step();
    assert_eq!(w.units[tank].type_id, uid::SIEGE_MODE, "it deploys at once");
    // Busy for the 64 frames of the deploy animation: no orders, no fire.
    assert!(w.issue(id, Order::Stop).is_err());
    for _ in 0..SIEGE_FRAMES - 2 {
        w.step();
    }
    assert_eq!(w.units[egg].hp, hp, "no shot while deploying");
    let mut fired_at = None;
    for t in 0..80 {
        w.step();
        if w.units[egg].hp < hp {
            fired_at = Some(t);
            break;
        }
    }
    assert!(fired_at.is_some(), "the sieged tank shells the Egg");
    // Arclite Shock Cannon: 70 explosive, 75 % against a medium unit.
    // Back to tank mode: sieged for the 63 frames of the pack-up animation.
    w.issue(id, Order::Unsiege).unwrap();
    for _ in 0..UNSIEGE_FRAMES {
        w.step();
        assert_eq!(w.units[tank].type_id, uid::SIEGE_MODE);
    }
    w.step();
    w.step();
    assert_eq!(w.units[tank].type_id, uid::SIEGE_TANK);
    assert_eq!(w.units[tank].order, Order::Idle);
}

#[test]
fn the_arclite_cannon_has_a_minimum_range_and_hurts_its_own_side() {
    use galaxycraft::ids::tech;
    use galaxycraft::sim::Order;
    let mut w = world();
    w.players[0].techs[tech::TANK_SIEGE_MODE as usize] = true;
    // Too close: box gap under 64 px.
    let tank = w.spawn_px(0, uid::SIEGE_MODE, 400, 400).unwrap();
    w.units[tank].status.sieged = true;
    let near = w.spawn_px(1, uid::EGG, 460, 400).unwrap();
    let hp = w.units[near].hp;
    for _ in 0..120 {
        w.step();
    }
    assert_eq!(w.units[near].hp, hp, "inside the minimum range");
    // A far target with a friendly Marine next to it: splash 10 / 25 / 40
    // is radial and burns the tank's own side.
    let far = w.spawn_px(1, uid::EGG, 700, 400).unwrap();
    let friend = w.spawn_px(0, uid::MARINE, 700, 425).unwrap();
    w.units[friend].order = Order::HoldPosition;
    let (hp_far, hp_friend) = (w.units[far].hp, w.units[friend].hp);
    for _ in 0..120 {
        w.step();
    }
    assert!(w.units[far].hp < hp_far);
    assert!(w.units[friend].hp < hp_friend || !w.units[friend].alive);
}

#[test]
fn a_spider_mine_waits_burrows_ignores_hover_units_and_chases_walkers() {
    use galaxycraft::ids::tech;
    use galaxycraft::sim::{Order, SPIDER_MINE_ARM_TICKS, Spell, SpellTarget};
    let mut w = world();
    w.players[0].techs[tech::SPIDER_MINES as usize] = true;
    let vulture = w.spawn_px(0, uid::VULTURE, 400, 400).unwrap();
    let vid = w.units[vulture].id;
    // The Vulture drives to within 20 px of the point before it lays it.
    w.issue(
        vid,
        Order::Cast {
            spell: Spell::SpiderMines,
            target: SpellTarget::Point(600, 400),
        },
    )
    .unwrap();
    let mut mine = None;
    for _ in 0..200 {
        w.step();
        mine = w
            .units
            .iter()
            .position(|u| u.alive && u.type_id == uid::SPIDER_MINE);
        if mine.is_some() {
            break;
        }
    }
    let mine = mine.expect("a mine was laid");
    assert_eq!(w.units[vulture].hangar, 2);
    assert!(
        (w.units[vulture].x / 256 - 600).abs() <= 24,
        "the Vulture drove there"
    );
    // At the point (the Vulture next to it may nudge it a pixel or two).
    assert!((w.units[mine].x / 256 - 600).abs() <= 4 && (w.units[mine].y / 256 - 400).abs() <= 4);
    // In the open for 60 frames plus the 4 of the burrow animation.
    assert!(!w.units[mine].status.burrowed);
    for _ in 0..SPIDER_MINE_ARM_TICKS - 3 {
        w.step();
    }
    assert!(!w.units[mine].status.burrowed, "still visible");
    assert!(w.can_see_unit(1, mine));
    for _ in 0..6 {
        w.step();
    }
    assert!(w.units[mine].status.burrowed, "armed");
    assert!(!w.can_see_unit(1, mine), "hidden without a detector");

    // Hovering units and buildings do not wake it.
    let scv = w.spawn_px(1, uid::SCV, 630, 400).unwrap();
    let enemy_vulture = w.spawn_px(1, uid::VULTURE, 600, 440).unwrap();
    for u in [scv, enemy_vulture] {
        w.units[u].order = Order::HoldPosition;
    }
    for _ in 0..40 {
        w.step();
    }
    assert!(w.units[mine].alive && w.units[mine].status.burrowed);
    // (Take them away: they would shoot the mine the moment it surfaces.)
    for u in [scv, enemy_vulture] {
        w.units[u].alive = false;
    }

    // A Marine 80 px away (inside the 96 px square) does: the mine
    // unburrows, runs at it and explodes for 125 explosive (50 % on small).
    let marine = w.spawn_px(1, uid::MARINE, 680, 400).unwrap();
    w.units[marine].order = Order::HoldPosition;
    let mut ran = false;
    for _ in 0..60 {
        w.step();
        ran |= w.units[mine].alive && w.units[mine].x / 256 > 610;
        if !w.units[mine].alive {
            break;
        }
    }
    assert!(ran, "the mine left its hole and chased");
    assert!(!w.units[mine].alive, "and blew up");
    assert!(!w.units[marine].alive, "62 damage kills a 40 hp Marine");
}
