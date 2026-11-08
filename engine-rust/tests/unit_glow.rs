//! Engine glows of units (user report 2026-11-07: the SCV's thruster glow).

use std::sync::Arc;

use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::iscript::{AnimContext, AnimEvent};
use galaxycraft::map::{Scenario, TileData};
use galaxycraft::sim::{Order, Race, World};
use galaxycraft::unit_anim::{Animator, ENGINE_GLOW_ISCRIPT};

#[test]
fn the_scv_walking_animation_attaches_its_thruster_glow() {
    // FACT (iscript probe): the SCV's walking animation (iscript 84, anim
    // 11) runs `imgol 249`, the `tscGlow` image driven by the engine-glow
    // iscript.
    let d = GameData::bundled().expect("data");
    let mut st = d.iscript.start_animation(84, 11, 0).expect("walking");
    let mut overlays = Vec::new();
    for _ in 0..8 {
        let mut ctx = AnimContext::default();
        for e in d.iscript.tick_anim(&mut st, &mut ctx) {
            if let AnimEvent::Overlay { id, below, .. } = e {
                overlays.push((id, below));
            }
        }
    }
    assert!(overlays.contains(&(249, false)), "{overlays:?}");
    assert_eq!(
        d.image(249).map(|i| i.iscript_id),
        Some(ENGINE_GLOW_ISCRIPT)
    );
}

#[test]
fn a_moving_scv_shows_its_glow_and_a_stopped_one_does_not() {
    let raw = std::fs::read("assets/maps/Boxer.chk").expect("bundled Boxer");
    let vf4 = std::fs::read("assets/TileSet/platform.vf4").expect("vf4");
    let cv5 = std::fs::read("assets/TileSet/platform.cv5").expect("cv5");
    let sc = Scenario::parse(&raw).expect("parse");
    let mut w = World::from_scenario_occupied(
        Arc::new(GameData::bundled().expect("data")),
        &sc,
        &TileData::new(&cv5, &vf4),
        &[Race::Terran, Race::Terran],
        Some(&[true, true]),
    );
    w.step();
    let scv = w
        .units
        .iter()
        .find(|u| u.owner == 0 && u.type_id == uid::SCV)
        .map(|u| (u.id, u.x / 256, u.y / 256))
        .expect("an SCV");
    let mut anim = Animator::default();
    anim.advance(&w);
    assert_eq!(anim.unit_glow(scv.0), None, "idle: no glow");
    w.issue(
        scv.0,
        Order::Move {
            pos: (scv.1 + 300, scv.2),
        },
    )
    .expect("move");
    let mut seen = false;
    for _ in 0..12 {
        w.step();
        anim.advance(&w);
        seen |= anim.unit_glow(scv.0) == Some(249);
    }
    assert!(seen, "walking: the thruster glow is attached");
    w.issue(scv.0, Order::Stop).expect("stop");
    for _ in 0..12 {
        w.step();
        anim.advance(&w);
    }
    assert_eq!(anim.unit_glow(scv.0), None, "stopped: the glow is gone");
}
