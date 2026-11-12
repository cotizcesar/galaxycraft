//! SCV behaviour against the retail data (iscript 84, weapons.dat row 14,
//! images 397 / 403) and OpenBW's worker rules: mining and welding loop the
//! AlmostBuilt animation with the Fusion Cutter spark, a loaded worker shows
//! its cargo, a worker inside a Refinery is hidden, and gathering workers do
//! not collide with other units.

use std::sync::Arc;

use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::Terrain;
use galaxycraft::sim::{CarryKind, Order, World};
use galaxycraft::unit_anim::{Animator, carry_image};

fn world() -> World {
    World::new(
        Arc::new(GameData::bundled().expect("data")),
        Terrain::open(64, 64),
        2,
    )
}

/// A Command Center, a mineral field to its right and one SCV between them.
fn mining_base(w: &mut World) -> (usize, usize) {
    let cc = w.spawn_px(0, uid::COMMAND_CENTER, 400, 400).unwrap();
    w.units[cc].completed = true;
    let field = w.spawn_px(0, uid::MINERAL_FIELD, 700, 400).unwrap();
    w.units[field].resource_amount = 1500;
    let scv = w.spawn_px(0, uid::SCV, 560, 400).unwrap();
    (scv, field)
}

#[test]
fn a_mining_scv_loops_the_cutter_animation_with_sparks_and_sound() {
    let mut w = world();
    let (scv, field) = mining_base(&mut w);
    let (sid, fid) = (w.units[scv].id, w.units[field].id);
    w.issue(sid, Order::Gather { resource: fid }).unwrap();
    let mut anim = Animator::default();
    let (mut frames, mut sparks, mut sounds) = (Vec::new(), 0, Vec::new());
    for _ in 0..400 {
        w.step();
        anim.advance(&w);
        if w.units[scv].gather_timer > 0 {
            frames.extend(anim.unit_frame(sid));
            sparks += anim.effects().filter(|e| e.image == 526).count();
            // The field is to its right (direction byte 64 = east).
            assert!(
                (w.units[scv].facing as i32 - 64).abs() < 32,
                "the SCV faces its field: {}",
                w.units[scv].facing
            );
        }
        sounds.extend(anim.take_sounds().into_iter().map(|s| s.0));
        if w.units[scv].carry > 0 {
            break;
        }
    }
    // iscript 84 AlmostBuilt: `playfram 34`, `useweapon 14`, `playfram 17`.
    assert!(frames.contains(&34) && frames.contains(&17), "{frames:?}");
    assert!(sparks > 0, "the scvspark bullet (image 526) appears");
    // scvspark Init: `playsndbtwn 35 39`.
    assert!(
        sounds.iter().any(|s| (35..=39).contains(s)),
        "cutter sound: {sounds:?}"
    );
}

#[test]
fn a_loaded_scv_shows_its_mineral_chunk_until_it_delivers() {
    let mut w = world();
    let (scv, field) = mining_base(&mut w);
    let (sid, fid) = (w.units[scv].id, w.units[field].id);
    w.issue(sid, Order::Gather { resource: fid }).unwrap();
    let anim = Animator::default();
    assert!(anim.carry_overlays(&w).is_empty(), "empty hands at first");
    let mut carried = false;
    for _ in 0..800 {
        w.step();
        let over = anim.carry_overlays(&w);
        if w.units[scv].carry > 0 {
            carried = true;
            assert_eq!(over.len(), 1);
            assert_eq!(over[0].image, 397, "OreChunk type 1");
            assert_eq!(
                (over[0].x, over[0].y),
                (w.units[scv].x / 256, w.units[scv].y / 256)
            );
        } else {
            assert!(over.is_empty());
        }
        if w.players[0].minerals_gathered > 0 {
            break;
        }
    }
    assert!(carried && w.players[0].minerals_gathered == 8);
}

#[test]
fn cargo_images_follow_the_race_and_the_load_size() {
    assert_eq!(carry_image(uid::SCV, CarryKind::None, 0), None);
    assert_eq!(carry_image(uid::SCV, CarryKind::Minerals, 8), Some(397));
    assert_eq!(carry_image(uid::SCV, CarryKind::Minerals, 2), Some(398));
    assert_eq!(carry_image(uid::SCV, CarryKind::Gas, 8), Some(403));
    assert_eq!(carry_image(uid::SCV, CarryKind::Gas, 2), Some(404));
    assert_eq!(carry_image(uid::PROBE, CarryKind::Gas, 8), Some(399));
    assert_eq!(carry_image(uid::DRONE, CarryKind::Gas, 8), Some(401));
}

#[test]
fn an_scv_inside_a_refinery_is_hidden_and_comes_out_with_a_gas_tank() {
    let mut w = world();
    let cc = w.spawn_px(0, uid::COMMAND_CENTER, 400, 400).unwrap();
    w.units[cc].completed = true;
    let refinery = w.spawn_px(0, uid::REFINERY, 720, 400).unwrap();
    w.units[refinery].completed = true;
    w.units[refinery].resource_amount = 5000;
    let scv = w.spawn_px(0, uid::SCV, 560, 400).unwrap();
    let (sid, rid) = (w.units[scv].id, w.units[refinery].id);
    w.issue(sid, Order::Gather { resource: rid }).unwrap();
    let mut anim = Animator::default();
    let (mut inside, mut tank, mut exit_checked) = (0, false, false);
    for _ in 0..800 {
        w.step();
        anim.advance(&w);
        if w.inside_gas_building(scv) {
            inside += 1;
            assert!(!w.can_see_unit(1, scv), "hidden from the enemy");
            assert!(anim.carry_overlays(&w).is_empty());
        }
        if w.units[scv].carry_kind == CarryKind::Gas {
            assert!(!w.inside_gas_building(scv), "out again with its load");
            if !exit_checked {
                exit_checked = true;
                // The depot is to the left: the worker reappears on the
                // refinery's left edge, level with it, facing west (192).
                let (r, u) = (&w.units[refinery], &w.units[scv]);
                assert!(
                    u.x < r.x && (u.y - r.y).abs() < 4 * 256,
                    "{:?}",
                    (u.x / 256, u.y / 256)
                );
                assert_eq!(u.facing, 192);
            }
            tank |= anim.carry_overlays(&w).iter().any(|o| o.image == 403);
        }
        if w.players[0].gas_gathered > 0 {
            break;
        }
    }
    // OpenBW `order_HarvestGas`: 37 frames inside.
    assert!((36..=38).contains(&inside), "frames inside: {inside}");
    assert!(tank, "the GasTank overlay shows on the way back");
    assert_eq!(w.players[0].gas_gathered, 8);
}

#[test]
fn gathering_workers_pass_through_units_but_idle_ones_collide() {
    let mut w = world();
    let (scv, field) = mining_base(&mut w);
    // A second SCV on the very same spot.
    let other = w.spawn_px(0, uid::SCV, 560, 400).unwrap();
    for _ in 0..4 {
        w.step();
    }
    assert_ne!(
        (w.units[scv].x, w.units[scv].y),
        (w.units[other].x, w.units[other].y),
        "idle units are pushed apart"
    );
    // A Marine standing in the mining path is not shoved by the worker.
    let marine = w.spawn_px(0, uid::MARINE, 620, 400).unwrap();
    let before = (w.units[marine].x, w.units[marine].y);
    let (sid, fid) = (w.units[scv].id, w.units[field].id);
    w.issue(sid, Order::Gather { resource: fid }).unwrap();
    assert!(w.is_gathering(scv));
    for _ in 0..600 {
        w.step();
        if w.players[0].minerals_gathered > 0 {
            break;
        }
    }
    assert_eq!(w.players[0].minerals_gathered, 8, "the worker got through");
    assert_eq!((w.units[marine].x, w.units[marine].y), before);
}

#[test]
fn an_scv_has_the_retail_speed_sight_and_cutter() {
    let d = GameData::bundled().expect("data");
    let ty = d.unit(uid::SCV);
    // flingy.dat: top speed 1280 (5 px / frame), acceleration 67, turn 40.
    let f = d.flingy(ty.flingy).expect("flingy");
    assert_eq!((f.top_speed, f.acceleration, f.turn_rate), (1280, 67, 40));
    assert_eq!(ty.sight_range, 7);
    // weapons.dat row 13: Fusion Cutter, 5 damage, cooldown 15, range 10 px.
    let wpn = d.weapon(ty.ground_weapon).expect("weapon");
    assert_eq!(
        (ty.ground_weapon, wpn.damage, wpn.cooldown, wpn.max_range),
        (13, 5, 15, 10)
    );
}

#[test]
fn an_scv_welding_its_building_loops_the_cutter_animation_facing_it() {
    let mut w = world();
    w.players[0].minerals = 500;
    let cc = w.spawn_px(0, uid::COMMAND_CENTER, 400, 400).unwrap();
    w.units[cc].completed = true;
    let scv = w.spawn_px(0, uid::SCV, 560, 400).unwrap();
    let sid = w.units[scv].id;
    w.issue(
        sid,
        Order::Build {
            unit_type: uid::SUPPLY_DEPOT,
            pos: (720, 400),
        },
    )
    .unwrap();
    let mut anim = Animator::default();
    let (mut frames, mut sparks, mut welding_ticks) = (Vec::new(), 0, 0);
    for _ in 0..400 {
        w.step();
        anim.advance(&w);
        if let Order::Construct { building } = w.units[scv].order
            && w.units[scv].path.is_empty()
        {
            welding_ticks += 1;
            frames.extend(anim.unit_frame(sid));
            sparks += anim.effects().filter(|e| e.image == 526).count();
            let b = w.unit(building).expect("building");
            let (dx, dy) = (b.x - w.units[scv].x, b.y - w.units[scv].y);
            // Towards the structure: within a quarter turn of the vector.
            let want = (dx as f64).atan2(-(dy as f64)) / std::f64::consts::TAU * 256.0;
            let diff = (w.units[scv].facing as f64 - want).rem_euclid(256.0);
            assert!(diff.min(256.0 - diff) < 64.0, "facing the building");
        }
    }
    assert!(welding_ticks > 50, "welded for {welding_ticks} ticks");
    assert!(frames.contains(&34) && frames.contains(&17), "{frames:?}");
    assert!(sparks > 0);
}

#[test]
fn an_scv_builds_a_refinery_on_a_geyser_and_then_gathers_from_it() {
    let mut w = world();
    w.players[0].minerals = 500;
    let cc = w.spawn_px(0, uid::COMMAND_CENTER, 400, 400).unwrap();
    w.units[cc].completed = true;
    let geyser = w.spawn_px(0, uid::VESPENE_GEYSER, 720, 400).unwrap();
    w.units[geyser].resource_amount = 5000;
    let scv = w.spawn_px(0, uid::SCV, 560, 400).unwrap();
    let sid = w.units[scv].id;
    let (gx, gy) = (w.units[geyser].x / 256, w.units[geyser].y / 256);
    w.issue(
        sid,
        Order::Build {
            unit_type: uid::REFINERY,
            pos: (gx, gy),
        },
    )
    .unwrap();
    for _ in 0..3000 {
        w.step();
        if w.units
            .iter()
            .any(|u| u.alive && u.type_id == uid::REFINERY && u.completed)
        {
            break;
        }
    }
    let refinery = w
        .units
        .iter()
        .find(|u| u.alive && u.type_id == uid::REFINERY && u.completed)
        .map(|u| u.id)
        .expect("the refinery is built on the geyser");
    w.issue(sid, Order::Gather { resource: refinery }).unwrap();
    for _ in 0..800 {
        w.step();
        if w.players[0].gas_gathered > 0 {
            break;
        }
    }
    assert_eq!(w.players[0].gas_gathered, 8);
}

#[test]
fn an_scv_ordered_to_attack_closes_in_and_hits_with_its_cutter() {
    let mut w = world();
    let scv = w.spawn_px(0, uid::SCV, 400, 400).unwrap();
    let ling = w.spawn_px(1, uid::ZERGLING, 480, 330).unwrap();
    let (sid, lid) = (w.units[scv].id, w.units[ling].id);
    let hp0 = w.units[ling].hp;
    w.issue(sid, Order::AttackUnit { target: lid }).unwrap();
    let mut trace = Vec::new();
    for t in 0..120 {
        w.step();
        if t % 10 == 0 {
            trace.push((
                w.units[scv].x / 256,
                w.units[scv].y / 256,
                w.units[ling].x / 256,
                w.units[ling].y / 256,
                w.units[ling].hp / 256,
                w.units[scv].hp / 256,
                w.units[scv].cooldown,
            ));
        }
    }
    // Fusion Cutter: 5 damage every 15 frames once in reach (about 30
    // frames of approach), so at least four hits in 120 frames.
    assert!(
        w.units[ling].hp <= hp0 - 4 * 5 * 256 || !w.units[ling].alive,
        "the cutter keeps hitting: {trace:?}"
    );
    // Once in reach the fighters stand their ground instead of shoving each
    // other across the map.
    let (a, b) = (trace[6], trace[trace.len() - 1]);
    assert!(
        (a.0 - b.0).abs() <= 8 && (a.1 - b.1).abs() <= 8,
        "the SCV holds its position while it fights: {trace:?}"
    );
}

#[test]
fn an_scv_cruises_at_five_pixels_per_frame_and_sees_seven_tiles() {
    let mut w = world();
    w.enable_fog();
    let scv = w.spawn_px(0, uid::SCV, 200, 1000).unwrap();
    let sid = w.units[scv].id;
    w.issue(sid, Order::Move { pos: (1800, 1000) }).unwrap();
    // Let it accelerate (67 / 256 px per frame each frame up to 1280 / 256).
    for _ in 0..40 {
        w.step();
    }
    let x0 = w.units[scv].x;
    for _ in 0..20 {
        w.step();
    }
    assert_eq!(
        (w.units[scv].x - x0) / 256,
        100,
        "20 frames at 5 px / frame"
    );
    // Sight: 7 tiles (224 px) around it, not 9.
    let (x, y) = (w.units[scv].x / 256, w.units[scv].y / 256);
    assert!(w.is_visible_to(0, x + 6 * 32, y));
    assert!(!w.is_visible_to(0, x + 9 * 32, y));
}
