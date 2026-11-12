//! Wraith, Science Vessel, Dropship, Battlecruiser and Valkyrie against the
//! retail data and the OpenBW source (see docs/UNIT-FIDELITY-METHOD.md).

use std::sync::Arc;

use galaxycraft::data::GameData;
use galaxycraft::ids::{tech, unit as uid};
use galaxycraft::map::Terrain;
use galaxycraft::sim::{ENERGY_SCALE, Order, Spell, SpellTarget, World};

fn world() -> World {
    World::new(
        Arc::new(GameData::bundled().expect("data")),
        Terrain::open(64, 64),
        2,
    )
}

fn fp(v: i32) -> i32 {
    v * 256
}

#[test]
fn yamato_charges_then_fires_a_blast_that_travels() {
    let mut w = world();
    w.players[0].techs[tech::YAMATO_GUN as usize] = true;
    let bc = w.spawn_px(0, uid::BATTLECRUISER, 400, 400).unwrap();
    // 700 hit points, large: 260 explosive in full.
    let target = w.spawn_px(1, uid::COMMAND_CENTER, 650, 400).unwrap();
    w.units[target].completed = true;
    let (bid, tid) = (w.units[bc].id, w.units[target].id);
    w.units[bc].set_energy_fixed(200 * ENERGY_SCALE);
    let hp = w.units[target].hp;
    w.issue(
        bid,
        Order::Cast {
            spell: Spell::YamatoGun,
            target: SpellTarget::Unit(tid),
        },
    )
    .unwrap();
    let (mut fired_at, mut hit_at) = (None, None);
    for t in 0..200 {
        w.step();
        if fired_at.is_none() && !w.projectiles.is_empty() {
            fired_at = Some(t);
            assert_eq!(w.units[target].hp, hp, "nothing lands at launch");
        }
        // (Its lasers may open up once the order is done; wait for the
        // blast itself.)
        if w.units[target].hp <= hp - fp(259) {
            hit_at = Some(t);
            break;
        }
    }
    // The charge overlay (iscript 314) reaches `castspell` after 44 frames.
    let fired = fired_at.expect("the blast was fired");
    assert!((40..=56).contains(&fired), "charge time: {fired}");
    assert!(hit_at.expect("it hit") > fired, "the blast travels");
    // 260 explosive, minus the Command Center's armor (1), plus whatever
    // laser fire followed.
    assert!((hp - w.units[target].hp) / 256 >= 259);
    assert_eq!(w.units[bc].energy_fixed() / ENERGY_SCALE, 50, "150 energy");
}

#[test]
fn emp_is_a_missile_that_drains_a_square_and_spares_the_caster() {
    let mut w = world();
    w.players[0].techs[tech::EMP_SHOCKWAVE as usize] = true;
    let vessel = w.spawn_px(0, uid::SCIENCE_VESSEL, 400, 400).unwrap();
    w.units[vessel].set_energy_fixed(200 * ENERGY_SCALE);
    // A caster and a shielded unit inside the 64 px square, in its corner
    // (a circle of 64 px would miss a unit at 60 / 60), and one outside.
    let templar = w.spawn_px(1, uid::HIGH_TEMPLAR, 600, 400).unwrap();
    let zealot = w.spawn_px(1, uid::ZEALOT, 660, 460).unwrap();
    let far = w.spawn_px(1, uid::ZEALOT, 700, 400).unwrap();
    for u in [templar, zealot, far] {
        w.units[u].order = Order::HoldPosition;
    }
    w.units[templar].set_energy_fixed(150 * ENERGY_SCALE);
    let vid = w.units[vessel].id;
    w.cast(vid, Spell::EmpShockwave, SpellTarget::Point(600, 400))
        .unwrap();
    assert_eq!(
        w.units[templar].energy_fixed() / ENERGY_SCALE,
        150,
        "in flight"
    );
    for _ in 0..60 {
        w.step();
    }
    // (All of it gone; a few frames of regeneration since.)
    assert!(w.units[templar].energy_fixed() / ENERGY_SCALE <= 2);
    // (Zero, plus the few frames of shield regeneration since.)
    assert!(w.units[zealot].shields < fp(3), "the square's corner");
    assert!(w.units[far].shields > 0, "outside");
    assert!(
        w.units[vessel].energy_fixed() > 0,
        "the caster keeps its energy"
    );
}

#[test]
fn irradiate_lands_after_its_bullet_and_burns_organic_units() {
    let mut w = world();
    w.players[0].techs[tech::IRRADIATE as usize] = true;
    let vessel = w.spawn_px(0, uid::SCIENCE_VESSEL, 400, 400).unwrap();
    w.units[vessel].set_energy_fixed(200 * ENERGY_SCALE);
    let ling = w.spawn_px(1, uid::ULTRALISK, 560, 400).unwrap();
    w.units[ling].order = Order::HoldPosition;
    let (vid, lid) = (w.units[vessel].id, w.units[ling].id);
    w.cast(vid, Spell::Irradiate, SpellTarget::Unit(lid))
        .unwrap();
    assert_eq!(w.units[ling].status.irradiate_ticks, 0, "not yet");
    for _ in 0..10 {
        w.step();
    }
    assert!(w.units[ling].status.irradiate_ticks > 0, "landed");
    let hp = w.units[ling].hp;
    for _ in 0..100 {
        w.step();
    }
    assert!(w.units[ling].hp < hp - fp(20), "it burns");
}

#[test]
fn a_dropship_unloads_one_unit_every_15_frames_with_weapons_on_cooldown() {
    let mut w = world();
    let ship = w.spawn_px(0, uid::DROPSHIP, 400, 400).unwrap();
    let sid = w.units[ship].id;
    let mut marines = Vec::new();
    for k in 0..4 {
        let m = w.spawn_px(0, uid::MARINE, 380 + 12 * k, 440).unwrap();
        let mid = w.units[m].id;
        w.issue(mid, Order::EnterTransport { transport: sid })
            .unwrap();
        marines.push(m);
    }
    for _ in 0..200 {
        w.step();
    }
    assert_eq!(w.units[ship].cargo.len(), 4, "all aboard");
    w.issue(sid, Order::Unload).unwrap();
    let mut out_at = Vec::new();
    for t in 0..100 {
        w.step();
        let out = marines.iter().filter(|&&m| w.units[m].alive).count();
        if out > out_at.len() {
            out_at.push(t);
            // Fresh off the ship: the weapon is on its cooldown (15).
            let newest = marines
                .iter()
                .copied()
                .filter(|&m| w.units[m].alive)
                .map(|m| w.units[m].cooldown)
                .max()
                .unwrap();
            assert!(newest >= 13, "cooldown {newest}");
        }
    }
    assert_eq!(out_at.len(), 4);
    for pair in out_at.windows(2) {
        assert_eq!(pair[1] - pair[0], 16, "15 frames apart: {out_at:?}");
    }
}

#[test]
fn halo_rockets_come_in_eight_and_splash_other_air_units() {
    let mut w = world();
    let valk = w.spawn_px(0, uid::VALKYRIE, 400, 400).unwrap();
    // Two Overlords close together and a friendly (unarmed) Dropship
    // between them.
    let a = w.spawn_px(1, uid::OVERLORD, 540, 400).unwrap();
    let b = w.spawn_px(1, uid::OVERLORD, 570, 430).unwrap();
    let friend = w.spawn_px(0, uid::DROPSHIP, 555, 380).unwrap();
    w.units[friend].order = Order::HoldPosition;
    let (hp_a, hp_b, hp_f) = (w.units[a].hp, w.units[b].hp, w.units[friend].hp);
    let (vid, aid) = (w.units[valk].id, w.units[a].id);
    w.issue(vid, Order::AttackUnit { target: aid }).unwrap();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..60 {
        w.step();
        seen.extend(w.projectiles.iter().map(|p| p.id));
    }
    // iscript: four `attackwith 2`, ten frames apart, two rockets each
    // (cooldown 64, so one volley in 60 frames).
    assert_eq!(seen.len(), 8, "one volley");
    assert!(w.units[a].hp < hp_a, "the target is hit");
    assert!(w.units[b].hp < hp_b, "its neighbour takes splash");
    assert_eq!(
        w.units[friend].hp, hp_f,
        "the Valkyrie's own side is spared"
    );
}

#[test]
fn hero_cloaks_drain_energy_like_their_units() {
    let mut w = world();
    // 21 = Tom Kazansky (Wraith), 16 = Sarah Kerrigan (Ghost).
    for (ty, per_frame) in [(21u16, 13), (16, 10)] {
        let u = w.spawn_px(0, ty, 400, 400 + ty as i32).unwrap();
        let id = w.units[u].id;
        w.units[u].set_energy_fixed(200 * ENERGY_SCALE);
        w.set_cloak(id, true).unwrap();
        let before = w.units[u].energy_fixed();
        for _ in 0..10 {
            w.step();
        }
        assert_eq!(
            before - w.units[u].energy_fixed(),
            10 * per_frame,
            "unit {ty}"
        );
    }
}

#[test]
fn stacked_air_units_drift_apart_and_a_lone_one_stays_put() {
    let mut w = world();
    let lone = w.spawn_px(0, uid::WRAITH, 1500, 1500).unwrap();
    let at = (w.units[lone].x, w.units[lone].y);
    // Four Wraiths on the same spot (the same 48 px repulse cell).
    let stack: Vec<usize> = (0..4)
        .map(|_| w.spawn_px(0, uid::WRAITH, 600, 600).unwrap())
        .collect();
    for _ in 0..200 {
        w.step();
    }
    assert_eq!((w.units[lone].x, w.units[lone].y), at, "alone: no drift");
    let mut cells = std::collections::HashSet::new();
    for &u in &stack {
        cells.insert((w.units[u].x / 256 / 48, w.units[u].y / 256 / 48));
    }
    assert!(cells.len() >= 3, "the stack spread over cells: {cells:?}");
    // Ground units are not part of it.
    let a = w.spawn_px(0, uid::DROPSHIP, 1000, 1000).unwrap();
    let marine = w.spawn_px(0, uid::MARINE, 1000, 1000).unwrap();
    let _ = marine;
    let at = (w.units[a].x, w.units[a].y);
    for _ in 0..50 {
        w.step();
    }
    assert_eq!((w.units[a].x, w.units[a].y), at);
}

#[test]
fn a_dropship_flies_to_a_point_and_unloads_there() {
    let mut w = world();
    let ship = w.spawn_px(0, uid::DROPSHIP, 400, 400).unwrap();
    let sid = w.units[ship].id;
    let marine = w.spawn_px(0, uid::MARINE, 400, 440).unwrap();
    let mid = w.units[marine].id;
    w.issue(mid, Order::EnterTransport { transport: sid })
        .unwrap();
    for _ in 0..100 {
        w.step();
    }
    assert_eq!(w.units[ship].cargo.len(), 1);
    // OpenBW `order_MoveUnload`: unload once within 16 px of the point.
    w.issue(sid, Order::MoveUnload { pos: (900, 500) }).unwrap();
    let mut out_at = None;
    for _ in 0..400 {
        w.step();
        if w.units[marine].alive {
            out_at = Some((w.units[ship].x / 256, w.units[ship].y / 256));
            break;
        }
    }
    let (x, y) = out_at.expect("the Marine got out");
    assert!(
        (x - 900).abs() <= 20 && (y - 500).abs() <= 20,
        "at the point: {x},{y}"
    );
    assert!((w.units[marine].x / 256 - 900).abs() < 80);
}

#[test]
fn matrix_and_irradiate_show_their_overlays_on_the_target() {
    use galaxycraft::unit_anim::{Animator, IRRADIATE_IMAGE, MATRIX_FRONT_IMAGE};
    let mut w = world();
    for t in [tech::DEFENSIVE_MATRIX, tech::IRRADIATE] {
        w.players[0].techs[t as usize] = true;
    }
    let vessel = w.spawn_px(0, uid::SCIENCE_VESSEL, 400, 400).unwrap();
    w.units[vessel].set_energy_fixed(200 * ENERGY_SCALE);
    let marine = w.spawn_px(0, uid::MARINE, 460, 400).unwrap();
    let ultra = w.spawn_px(1, uid::ULTRALISK, 560, 460).unwrap();
    w.units[ultra].order = Order::HoldPosition;
    let (vid, mid, uid_) = (w.units[vessel].id, w.units[marine].id, w.units[ultra].id);
    w.cast(vid, Spell::DefensiveMatrix, SpellTarget::Unit(mid))
        .unwrap();
    w.cast(vid, Spell::Irradiate, SpellTarget::Unit(uid_))
        .unwrap();
    let mut anim = Animator::default();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..30 {
        w.step();
        anim.advance(&w);
        seen.extend(anim.status_overlays(&w).iter().map(|o| o.image));
    }
    // Marine: small (+0); Ultralisk: large (+2).
    assert!(seen.contains(&MATRIX_FRONT_IMAGE), "{seen:?}");
    // The back shell is drawn behind the Marine.
    assert!(
        anim.status_underlays(mid)
            .iter()
            .any(|(img, _)| *img == galaxycraft::unit_anim::MATRIX_BACK_IMAGE)
    );
    assert!(seen.contains(&(IRRADIATE_IMAGE + 2)), "{seen:?}");
}

#[test]
fn an_idle_dropship_flies_over_to_pick_up_a_unit_that_asks_to_board() {
    let mut w = world();
    let ship = w.spawn_px(0, uid::DROPSHIP, 400, 400).unwrap();
    // Far apart: the Marine walks, the Dropship comes to meet it (OpenBW
    // `order_PickupTransport`), and it boards when their boxes touch.
    let marine = w.spawn_px(0, uid::MARINE, 900, 400).unwrap();
    let (sid, mid) = (w.units[ship].id, w.units[marine].id);
    let ship_x = w.units[ship].x;
    w.issue(mid, Order::EnterTransport { transport: sid })
        .unwrap();
    let mut aboard_at = None;
    for t in 0..300 {
        w.step();
        if w.units[marine].in_transport.is_some() {
            aboard_at = Some(t);
            break;
        }
    }
    assert!(aboard_at.is_some(), "boarded");
    assert!(
        w.units[ship].x > ship_x + 256 * 60,
        "the Dropship came to meet it"
    );
    assert_eq!(w.units[ship].cargo.len(), 1);
    // Its pickup is over: it goes back to idle.
    for _ in 0..3 {
        w.step();
    }
    assert_eq!(w.units[ship].order, Order::Idle);
}

#[test]
fn boarding_a_transport_does_not_play_a_death_animation() {
    use galaxycraft::unit_anim::Animator;
    let mut w = world();
    let ship = w.spawn_px(0, uid::DROPSHIP, 400, 400).unwrap();
    let marine = w.spawn_px(0, uid::MARINE, 420, 430).unwrap();
    let (sid, mid) = (w.units[ship].id, w.units[marine].id);
    let marine_image = galaxycraft::unit_anim::unit_image(&w.data, uid::MARINE).unwrap();
    let mut anim = Animator::default();
    w.step();
    anim.advance(&w);
    w.issue(mid, Order::EnterTransport { transport: sid })
        .unwrap();
    for _ in 0..60 {
        w.step();
        anim.advance(&w);
        assert!(
            !anim.effects().any(|e| e.image == marine_image),
            "no Marine death on boarding"
        );
    }
    assert!(w.units[marine].in_transport.is_some());
}

#[test]
fn unloaded_units_land_on_separate_free_spots_under_the_transport() {
    let mut w = world();
    let ship = w.spawn_px(0, uid::DROPSHIP, 600, 600).unwrap();
    let sid = w.units[ship].id;
    let marines: Vec<usize> = (0..4)
        .map(|k| w.spawn_px(0, uid::MARINE, 590 + 8 * k, 620).unwrap())
        .collect();
    for &m in &marines {
        let mid = w.units[m].id;
        w.issue(mid, Order::EnterTransport { transport: sid })
            .unwrap();
    }
    for _ in 0..200 {
        w.step();
    }
    assert_eq!(w.units[ship].cargo.len(), 4);
    w.issue(sid, Order::Unload).unwrap();
    for _ in 0..80 {
        w.step();
    }
    let (sx, sy) = (w.units[ship].x / 256, w.units[ship].y / 256);
    let spots: Vec<(i32, i32)> = marines
        .iter()
        .map(|&m| (w.units[m].x / 256, w.units[m].y / 256))
        .collect();
    for (k, &(x, y)) in spots.iter().enumerate() {
        assert!(
            (x - sx).abs() <= 40 && (y - sy).abs() <= 40,
            "near the ship: {spots:?}"
        );
        for &(x2, y2) in &spots[k + 1..] {
            // Marine box 17 x 20: no two overlap.
            assert!((x - x2).abs() >= 16 || (y - y2).abs() >= 19, "{spots:?}");
        }
    }
}

#[test]
fn idle_flyers_bob_up_and_down() {
    use galaxycraft::unit_anim::Animator;
    let mut w = world();
    let wraith = w.spawn_px(0, uid::WRAITH, 400, 400).unwrap();
    let id = w.units[wraith].id;
    let mut anim = Animator::default();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..600 {
        w.step();
        anim.advance(&w);
        seen.insert(anim.unit_vert(id));
    }
    // iscript 0x79a2: setvertpos 1, 2, 1, 0 with random waits.
    assert!(seen.contains(&1) && seen.contains(&2), "{seen:?}");
}
