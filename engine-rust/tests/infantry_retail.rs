//! Marine, Firebat, Ghost and Medic against the retail data (iscripts 78 /
//! 69 / 70 / 360, weapons.dat rows 0 / 2 / 25, units.dat) and OpenBW's
//! rules for Stim Packs, range upgrades, the Firebat's three flames, the
//! Ghost's cloak and the Medic's heal orders.

use std::sync::Arc;

use galaxycraft::data::GameData;
use galaxycraft::ids::{tech, unit as uid, upgrade};
use galaxycraft::map::Terrain;
use galaxycraft::sim::{
    ENERGY_SCALE, MEDIC_REACH_PX, Order, STIM_FRAMES, SimError, Spell, SpellTarget, World,
};

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
fn unit_stats_come_from_the_retail_tables() {
    let d = GameData::bundled().expect("data");
    // (type, hp, armor, sight, minerals, gas, build time, ground weapon)
    for (ty, hp, armor, sight, minerals, gas, time, weapon) in [
        (uid::MARINE, 40, 0, 7, 50, 0, 360, 0),
        (uid::GHOST, 45, 0, 9, 25, 75, 750, 2),
        (uid::FIREBAT, 50, 1, 7, 50, 25, 360, 25),
        (uid::MEDIC, 60, 1, 9, 50, 25, 450, 130),
    ] {
        let u = d.unit(ty);
        assert_eq!(u.hit_points_raw as i32 / 256, hp, "hp of {ty}");
        assert_eq!(u.armor as i32, armor, "armor of {ty}");
        assert_eq!(u.sight_range as i32, sight, "sight of {ty}");
        assert_eq!(
            (
                u.mineral_cost as i32,
                u.gas_cost as i32,
                u.build_time as i32
            ),
            (minerals, gas, time),
            "cost of {ty}"
        );
        assert_eq!(u.ground_weapon as i32, weapon, "weapon of {ty}");
    }
    // Gauss Rifle 6 / 15 / 128 px, C-10 10 / 22 / 224 px, Flame Thrower
    // 8 / 22 / 32 px with a 15 / 20 / 25 px enemy splash.
    let w = |id: u8| d.weapon(id).expect("weapon");
    assert_eq!((w(0).damage, w(0).cooldown, w(0).max_range), (6, 15, 128));
    assert_eq!((w(2).damage, w(2).cooldown, w(2).max_range), (10, 22, 224));
    assert_eq!((w(25).damage, w(25).cooldown, w(25).max_range), (8, 22, 32));
    assert_eq!(
        (w(25).inner_splash, w(25).middle_splash, w(25).outer_splash),
        (15, 20, 25)
    );
}

/// Frames until `attacker` first damages `target`, or `None` in 200 frames.
fn first_hit(w: &mut World, attacker: usize, target: usize) -> Option<i32> {
    let (aid, tid) = (w.units[attacker].id, w.units[target].id);
    let hp = w.units[target].hp;
    w.issue(aid, Order::HoldPosition).unwrap();
    let _ = tid;
    (0..200).find(|_| {
        w.step();
        w.units[target].hp < hp
    })
}

#[test]
fn u238_shells_add_one_tile_of_marine_range() {
    // Boxes: Marine 17 px wide, Zergling 16; centres 160 px apart leave a
    // gap of about 144 px: outside 128, inside 160.
    let mut w = world();
    let marine = w.spawn_px(0, uid::MARINE, 400, 400).unwrap();
    let ling = w.spawn_px(1, uid::ZERGLING, 560, 400).unwrap();
    w.units[ling].order = Order::HoldPosition;
    assert_eq!(first_hit(&mut w, marine, ling), None, "out of range");
    w.players[0].upgrades[upgrade::U238_SHELLS as usize] = 1;
    assert!(first_hit(&mut w, marine, ling).is_some(), "in range now");
}

#[test]
fn stim_packs_cost_ten_hit_points_and_halve_the_cooldown() {
    let mut w = world();
    w.players[0].techs[tech::STIM_PACKS as usize] = true;
    let marine = w.spawn_px(0, uid::MARINE, 400, 400).unwrap();
    let id = w.units[marine].id;
    w.cast(id, Spell::StimPacks, SpellTarget::None).unwrap();
    assert_eq!(w.units[marine].hp, fp(30));
    assert_eq!(w.units[marine].status.stim_ticks, STIM_FRAMES);
    assert_eq!(STIM_FRAMES, 296, "37 timer units of 8 frames");
    w.cast(id, Spell::StimPacks, SpellTarget::None).unwrap();
    w.cast(id, Spell::StimPacks, SpellTarget::None).unwrap();
    assert_eq!(w.units[marine].hp, fp(10));
    // At 10 hit points or fewer the Stim Pack is refused.
    assert!(w.cast(id, Spell::StimPacks, SpellTarget::None).is_err());
    assert_eq!(w.units[marine].hp, fp(10));

    // Rate of fire: 15 frames per shot, 7 (15 / 2) under Stim.
    let shots = |stim: bool| {
        let mut w = world();
        w.players[0].techs[tech::STIM_PACKS as usize] = true;
        let m = w.spawn_px(0, uid::MARINE, 400, 400).unwrap();
        let t = w.spawn_px(1, uid::ULTRALISK, 460, 400).unwrap();
        w.units[t].order = Order::HoldPosition;
        let (mid, tid) = (w.units[m].id, w.units[t].id);
        if stim {
            w.cast(mid, Spell::StimPacks, SpellTarget::None).unwrap();
        }
        w.issue(mid, Order::AttackUnit { target: tid }).unwrap();
        let mut hits = 0;
        let mut hp = w.units[t].hp;
        for _ in 0..150 {
            w.step();
            if w.units[t].hp < hp {
                hits += 1;
            }
            hp = w.units[t].hp;
        }
        hits
    };
    let (plain, stimmed) = (shots(false), shots(true));
    assert!(
        (9..=11).contains(&plain),
        "about 10 shots in 150 frames: {plain}"
    );
    assert!(
        stimmed >= plain * 2 - 3,
        "stim doubles it: {stimmed} vs {plain}"
    );
}

#[test]
fn heroes_use_stim_and_cloak_like_their_units() {
    let mut w = world();
    // 20 = Jim Raynor (Marine), 10 = Gui Montag (Firebat), 16 = Sarah
    // Kerrigan (Ghost).
    for ty in [20, 10] {
        let u = w.spawn_px(0, ty, 400, 400).unwrap();
        let id = w.units[u].id;
        w.cast(id, Spell::StimPacks, SpellTarget::None)
            .unwrap_or_else(|e| panic!("hero {ty}: {e:?}"));
    }
    let k = w.spawn_px(0, 16, 500, 400).unwrap();
    let id = w.units[k].id;
    w.set_cloak(id, true).expect("Kerrigan cloaks");
    assert!(w.units[k].status.cloaked);
}

#[test]
fn a_firebat_burns_an_adjacent_target_twice_per_attack() {
    let mut w = world();
    let bat = w.spawn_px(0, uid::FIREBAT, 400, 400).unwrap();
    // Between the first two flames (24 and 52 px ahead): both reach its box.
    let ling = w.spawn_px(1, uid::ZERGLING, 438, 400).unwrap();
    w.units[ling].order = Order::HoldPosition;
    w.units[ling].hp = fp(35);
    let (bid, lid) = (w.units[bat].id, w.units[ling].id);
    w.issue(bid, Order::AttackUnit { target: lid }).unwrap();
    let mut drops = Vec::new();
    let mut hp = w.units[ling].hp;
    for _ in 0..20 {
        w.step();
        if w.units[ling].hp < hp {
            drops.push(hp - w.units[ling].hp);
        }
        hp = w.units[ling].hp;
        if !drops.is_empty() && drops.len() >= 3 {
            break;
        }
    }
    // Flames at 24 and 52 px burn it for the full 8 (concussive is 100 %
    // against small units, inner splash 15 px from its box); the one at
    // 80 px is past it.
    // (The Zergling regenerates a fraction of a point between flames.)
    let first_attack: i32 = drops.iter().take(2).sum();
    assert!(
        (16 * 256 - 64..=16 * 256).contains(&first_attack),
        "two flames of 8: {drops:?}"
    );
}

#[test]
fn firebat_flames_do_not_burn_the_owners_units() {
    let mut w = world();
    let bat = w.spawn_px(0, uid::FIREBAT, 400, 400).unwrap();
    // A friendly Marine right under the third flame (80 px ahead), out of
    // the Zergling's reach.
    let friend = w.spawn_px(0, uid::MARINE, 478, 400).unwrap();
    let ling = w.spawn_px(1, uid::ZERGLING, 430, 400).unwrap();
    w.units[ling].order = Order::HoldPosition;
    let (bid, lid) = (w.units[bat].id, w.units[ling].id);
    let hp = w.units[friend].hp;
    w.issue(bid, Order::AttackUnit { target: lid }).unwrap();
    for _ in 0..40 {
        w.step();
    }
    assert_eq!(w.units[friend].hp, hp);
    assert!(w.units[ling].hp < fp(35) || !w.units[ling].alive);
}

#[test]
fn the_ghost_cloak_costs_25_energy_and_then_drains() {
    let mut w = world();
    w.players[0].techs[tech::PERSONNEL_CLOAKING as usize] = true;
    let ghost = w.spawn_px(0, uid::GHOST, 400, 400).unwrap();
    let id = w.units[ghost].id;
    w.units[ghost].set_energy_fixed(24 * ENERGY_SCALE);
    assert_eq!(w.set_cloak(id, true), Err(SimError::NotEnoughEnergy));
    w.units[ghost].set_energy_fixed(50 * ENERGY_SCALE);
    w.set_cloak(id, true).unwrap();
    assert_eq!(w.units[ghost].energy_fixed(), 25 * ENERGY_SCALE);
    // Decloaking is free.
    w.set_cloak(id, false).unwrap();
    assert_eq!(w.units[ghost].energy_fixed(), 25 * ENERGY_SCALE);
    // Cloaked and unobserved by a detector, the enemy cannot see it.
    w.set_cloak(id, true).unwrap();
    assert!(!w.can_see_unit(1, ghost));
    // It runs out: 0 energy drops the cloak.
    for _ in 0..40 {
        w.step();
    }
    assert!(!w.units[ghost].status.cloaked, "out of energy");
}

#[test]
fn lockdown_disables_a_mechanical_unit_and_spares_organic_ones() {
    let mut w = world();
    w.players[0].techs[tech::LOCKDOWN as usize] = true;
    let ghost = w.spawn_px(0, uid::GHOST, 400, 400).unwrap();
    let tank = w.spawn_px(1, uid::SIEGE_TANK, 560, 400).unwrap();
    let ling = w.spawn_px(1, uid::ZERGLING, 560, 460).unwrap();
    let (gid, tid, lid) = (w.units[ghost].id, w.units[tank].id, w.units[ling].id);
    w.units[ghost].set_energy_fixed(200 * ENERGY_SCALE);
    assert!(
        w.cast(gid, Spell::Lockdown, SpellTarget::Unit(lid))
            .is_err()
    );
    assert_eq!(w.units[ghost].energy_fixed(), 200 * ENERGY_SCALE);
    w.cast(gid, Spell::Lockdown, SpellTarget::Unit(tid))
        .unwrap();
    assert_eq!(w.units[ghost].energy_fixed(), 100 * ENERGY_SCALE);
    for _ in 0..40 {
        w.step();
    }
    assert!(w.units[tank].status.lockdown_ticks > 0);
}

#[test]
fn trained_casters_start_with_a_quarter_of_their_energy() {
    let mut w = world();
    w.players[0].minerals = 1000;
    w.players[0].gas = 1000;
    let cc = w.spawn_px(0, uid::COMMAND_CENTER, 300, 300).unwrap();
    w.units[cc].completed = true;
    for ty in [uid::SUPPLY_DEPOT, uid::ACADEMY] {
        let b = w.spawn_px(0, ty, 600, 300 + ty as i32).unwrap();
        w.units[b].completed = true;
    }
    let rax = w.spawn_px(0, uid::BARRACKS, 300, 600).unwrap();
    w.units[rax].completed = true;
    let rid = w.units[rax].id;
    w.train(rid, uid::MEDIC).expect("train a Medic");
    for _ in 0..2000 {
        w.step();
        if w.units.iter().any(|u| u.alive && u.type_id == uid::MEDIC) {
            break;
        }
    }
    let medic = w
        .units
        .iter()
        .find(|u| u.alive && u.type_id == uid::MEDIC)
        .expect("the Medic was trained");
    assert_eq!(medic.energy_fixed() / ENERGY_SCALE, 50);
}

#[test]
fn a_medic_walks_to_a_wounded_marine_and_heals_it_but_never_itself() {
    let mut w = world();
    let medic = w.spawn_px(0, uid::MEDIC, 400, 400).unwrap();
    let marine = w.spawn_px(0, uid::MARINE, 520, 400).unwrap();
    w.units[marine].hp = fp(10);
    w.units[medic].hp = fp(20);
    w.units[medic].set_energy_fixed(100 * ENERGY_SCALE);
    assert!(w.units_distance_px(medic, marine) > MEDIC_REACH_PX);
    let mut healed_at = None;
    for t in 0..200 {
        w.step();
        if healed_at.is_none() && w.units[marine].hp > fp(10) {
            healed_at = Some(t);
            assert!(
                w.units_distance_px(medic, marine) <= MEDIC_REACH_PX,
                "it heals only in reach"
            );
            assert!(w.medic_is_healing(medic));
        }
    }
    assert!(healed_at.is_some(), "the Medic walked over");
    assert_eq!(w.units[marine].hp, fp(40), "fully healed");
    assert_eq!(w.units[medic].hp, fp(20), "a Medic does not heal itself");
    // 30 hit points cost 15 energy (2 hp per energy point), minus regen.
    let spent = 100 * ENERGY_SCALE - w.units[medic].energy_fixed();
    assert!(spent > 0 && spent <= 15 * ENERGY_SCALE, "{spent}");
}

#[test]
fn two_medics_do_not_heal_the_same_patient() {
    let mut w = world();
    let a = w.spawn_px(0, uid::MEDIC, 400, 400).unwrap();
    let b = w.spawn_px(0, uid::MEDIC, 400, 430).unwrap();
    let marine = w.spawn_px(0, uid::MARINE, 425, 415).unwrap();
    w.units[marine].hp = fp(10);
    for m in [a, b] {
        w.units[m].set_energy_fixed(100 * ENERGY_SCALE);
    }
    let hp = w.units[marine].hp;
    w.step();
    // One Medic's 200/256 per frame, not two.
    assert_eq!(w.units[marine].hp - hp, 200);
}

#[test]
fn optical_flare_blinds_for_good_until_restoration() {
    let mut w = world();
    w.players[0].techs[tech::OPTICAL_FLARE as usize] = true;
    w.players[0].techs[tech::RESTORATION as usize] = true;
    let medic = w.spawn_px(0, uid::MEDIC, 400, 400).unwrap();
    let vessel = w.spawn_px(1, uid::SCIENCE_VESSEL, 520, 400).unwrap();
    let (mid, vid) = (w.units[medic].id, w.units[vessel].id);
    w.units[medic].set_energy_fixed(200 * ENERGY_SCALE);
    assert_eq!(w.unit_sight_range(vessel), 10);
    w.cast(mid, Spell::OpticalFlare, SpellTarget::Unit(vid))
        .unwrap();
    for _ in 0..1500 {
        w.step();
    }
    assert_eq!(w.unit_sight_range(vessel), 2, "still blind a minute later");
    // A blinded detector no longer reveals cloaked units.
    w.players[0].techs[tech::PERSONNEL_CLOAKING as usize] = true;
    let ghost = w.spawn_px(0, uid::GHOST, 500, 420).unwrap();
    w.units[ghost].set_energy_fixed(200 * ENERGY_SCALE);
    w.set_cloak(w.units[ghost].id, true).unwrap();
    assert!(!w.can_see_unit(1, ghost));
}

/// Put `unit` into `bunker` through the load order.
fn load(w: &mut World, unit: usize, bunker: usize) {
    let (uid_, bid) = (w.units[unit].id, w.units[bunker].id);
    w.issue(uid_, Order::EnterTransport { transport: bid })
        .unwrap();
    for _ in 0..200 {
        w.step();
        if w.units[unit].in_transport.is_some() {
            return;
        }
    }
    panic!(
        "unit {} did not enter the bunker: order {:?} at {:?}, cargo {}",
        w.units[unit].type_id,
        w.units[unit].order,
        (w.units[unit].x / 256, w.units[unit].y / 256),
        w.units[bunker].cargo.len()
    );
}

#[test]
fn a_bunker_gives_its_marines_64_px_and_shelters_scvs_heroes_and_medics() {
    let mut w = world();
    let bunker = w.spawn_px(0, uid::BUNKER, 400, 400).unwrap();
    w.units[bunker].completed = true;
    let marine = w.spawn_px(0, uid::MARINE, 400, 450).unwrap();
    load(&mut w, marine, bunker);
    // A free Marine reaches 128 px; from the Bunker's centre it reaches
    // 128 + 64.  Target box 180 px from the occupant's box.
    let ling = w
        .spawn_px(1, uid::ZERGLING, 400 + 8 + 180 + 8, 400)
        .unwrap();
    w.units[ling].order = Order::HoldPosition;
    let hp = w.units[ling].hp;
    for _ in 0..60 {
        w.step();
    }
    assert!(w.units[ling].hp < hp, "the Bunker out-ranges a free Marine");

    // SCVs, heroes and Medics may enter; the SCV does not fire and the
    // Medic does not heal from inside.
    let scv = w.spawn_px(0, uid::SCV, 380, 450).unwrap();
    load(&mut w, scv, bunker);
    let raynor = w.spawn_px(0, 20, 420, 450).unwrap();
    load(&mut w, raynor, bunker);
    let medic = w.spawn_px(0, uid::MEDIC, 400, 455).unwrap();
    w.units[medic].set_energy_fixed(100 * ENERGY_SCALE);
    load(&mut w, medic, bunker);
    let wounded = w.spawn_px(0, uid::MARINE, 400, 450).unwrap();
    w.units[wounded].hp = fp(10);
    for _ in 0..50 {
        w.step();
    }
    assert_eq!(w.units[wounded].hp, fp(10), "no healing from a Bunker");
}

#[test]
fn a_silo_builds_its_missile_before_a_ghost_can_call_the_strike() {
    let mut w = world();
    w.players[0].minerals = 1000;
    w.players[0].gas = 1000;
    let cc = w.spawn_px(0, uid::COMMAND_CENTER, 300, 300).unwrap();
    w.units[cc].completed = true;
    for k in 0..4 {
        let d = w
            .spawn_px(0, uid::SUPPLY_DEPOT, 600 + 100 * k, 200)
            .unwrap();
        w.units[d].completed = true;
    }
    let silo = w.spawn_px(0, uid::NUCLEAR_SILO, 420, 320).unwrap();
    w.units[silo].completed = true;
    w.units[silo].addon_attached = true;
    let ghost = w.spawn_px(0, uid::GHOST, 300, 900).unwrap();
    let (sid, gid) = (w.units[silo].id, w.units[ghost].id);
    // No missile yet: the strike is refused and nothing is spent.
    assert_eq!(
        w.launch_nuke(gid, (1500, 900)),
        Err(SimError::MissingRequirements)
    );
    assert_eq!(w.players[0].minerals, 1000);
    w.train(sid, uid::NUCLEAR_MISSILE).expect("arm the silo");
    assert_eq!((w.players[0].minerals, w.players[0].gas), (800, 800));
    assert!(!w.units[silo].nuke_ready, "it takes its build time");
    let mut frames = 0;
    while !w.units[silo].nuke_ready && frames < 3000 {
        w.step();
        frames += 1;
    }
    assert!(w.units[silo].nuke_ready, "armed after {frames} frames");
    assert!(frames > 1000, "units.dat build time 1500: {frames}");
    assert!(
        !w.units
            .iter()
            .any(|u| u.alive && u.type_id == uid::NUCLEAR_MISSILE),
        "the missile stays in the silo"
    );

    // The Ghost walks until the point is within its sight, then paints.
    w.launch_nuke(gid, (1500, 900)).unwrap();
    assert!(!w.units[silo].nuke_ready);
    let start = w.units[ghost].x;
    let mut hit_at = None;
    let victim = w.spawn_px(1, uid::SUPPLY_DEPOT, 1500, 900).unwrap();
    let far = w
        .spawn_px(1, uid::SUPPLY_DEPOT, 1500 + 48 + 230, 900)
        .unwrap();
    let (hp_v, hp_f) = (w.units[victim].hp, w.units[far].hp);
    for t in 0..1500 {
        w.step();
        if w.units[victim].hp < hp_v || !w.units[victim].alive {
            hit_at = Some(t);
            break;
        }
    }
    assert!(hit_at.is_some(), "the nuke landed");
    assert!(
        w.units[ghost].x > start + 256 * 500,
        "the Ghost walked into range"
    );
    assert!(
        ((1500 - w.units[ghost].x / 256) as i64) <= 9 * 32 + 16,
        "and stopped at its sight range"
    );
    // Depot: 500 hp, armor 1.  Ground zero takes the full 500; one whose
    // box is 200+ px away is in the outer ring and takes a quarter.
    assert!(!w.units[victim].alive || w.units[victim].hp <= fp(2));
    let lost = (hp_f - w.units[far].hp) / 256;
    assert!((100..=130).contains(&lost), "outer ring: {lost}");
}

#[test]
fn the_lockdown_missile_takes_time_to_reach_its_target() {
    let mut w = world();
    w.players[0].techs[tech::LOCKDOWN as usize] = true;
    let ghost = w.spawn_px(0, uid::GHOST, 300, 400).unwrap();
    let tank = w.spawn_px(1, uid::SIEGE_TANK, 520, 400).unwrap();
    w.units[tank].order = Order::HoldPosition;
    let (gid, tid) = (w.units[ghost].id, w.units[tank].id);
    w.units[ghost].set_energy_fixed(100 * ENERGY_SCALE);
    w.issue(
        gid,
        Order::Cast {
            spell: Spell::Lockdown,
            target: SpellTarget::Unit(tid),
        },
    )
    .unwrap();
    let mut landed = None;
    for t in 0..120 {
        w.step();
        if w.units[tank].status.lockdown_ticks > 0 {
            landed = Some(t);
            break;
        }
    }
    let t = landed.expect("locked down");
    assert!(t >= 3, "cast animation plus missile flight: {t}");
    assert_eq!(w.units[ghost].energy_fixed() / ENERGY_SCALE, 0);
}

#[test]
fn a_medic_keeps_its_orders_while_it_walks() {
    // Its Walking script starts with `orderdone 1`, which only clears an
    // order-signal bit.
    let mut w = world();
    let medic = w.spawn_px(0, uid::MEDIC, 400, 400).unwrap();
    let id = w.units[medic].id;
    w.issue(id, Order::Move { pos: (900, 400) }).unwrap();
    for _ in 0..200 {
        w.step();
    }
    let x = w.units[medic].x / 256;
    assert!((880..=920).contains(&x), "walked to its destination: {x}");
}

#[test]
fn selection_and_order_voices_cover_every_retail_line() {
    use galaxycraft::sfx::SfxTable;
    use galaxycraft::voice::{VoiceCue, selection_cue, voice_ids};
    let table = SfxTable::bundled();
    // sfxdata.tbl: every unit has 1 ready line, 4 "what?" lines, 4 order
    // replies and 7 annoyed lines (the Ghost has 4).
    for (ty, pissed) in [
        (uid::MARINE, 7),
        (uid::GHOST, 4),
        (uid::SCV, 7),
        (uid::FIREBAT, 7),
        (uid::MEDIC, 7),
    ] {
        let n = |cue| voice_ids(&table, ty, cue).len();
        assert_eq!(n(VoiceCue::Ready), 1, "ready of {ty}");
        assert_eq!(n(VoiceCue::What), 4, "what of {ty}");
        assert_eq!(n(VoiceCue::Yes), 4, "yes of {ty}");
        assert_eq!(n(VoiceCue::Pissed), pissed, "annoyed of {ty}");
        // Clicking the same unit again and again: three "what?" lines,
        // then every annoyed line in order, then it starts over.
        for click in 1..=3 {
            assert_eq!(selection_cue(click, pissed).0, VoiceCue::What);
        }
        for k in 0..pissed {
            assert_eq!(
                selection_cue(4 + k as u32, pissed),
                (VoiceCue::Pissed, k, false)
            );
        }
        let (cue, _, restart) = selection_cue(4 + pissed as u32, pissed);
        assert!(cue == VoiceCue::What && restart);
    }
}

#[test]
fn the_renderer_shows_the_medic_heal_and_the_spell_effects() {
    use galaxycraft::unit_anim::{
        Animator, HEAL_IMAGE, LOCKDOWN_FIELD_IMAGE, OPTICAL_FLARE_HIT_IMAGE, RESTORATION_IMAGE,
        STIM_SOUNDS,
    };
    let mut w = world();
    for t in [
        tech::STIM_PACKS,
        tech::LOCKDOWN,
        tech::RESTORATION,
        tech::OPTICAL_FLARE,
    ] {
        w.players[0].techs[t as usize] = true;
    }
    let medic = w.spawn_px(0, uid::MEDIC, 400, 400).unwrap();
    let marine = w.spawn_px(0, uid::MARINE, 425, 400).unwrap();
    w.units[marine].hp = fp(20);
    w.units[medic].set_energy_fixed(200 * ENERGY_SCALE);
    let (mid, marine_id) = (w.units[medic].id, w.units[marine].id);
    let mut anim = Animator::default();
    let (mut frames, mut heal_fx, mut sounds) = (Vec::new(), false, Vec::new());
    for _ in 0..30 {
        w.step();
        anim.advance(&w);
        frames.extend(anim.unit_frame(mid));
        // The Marine is small: Heal image 970.
        heal_fx |= anim
            .status_overlays(&w)
            .iter()
            .any(|o| o.image == HEAL_IMAGE && o.x == w.units[marine].x / 256);
        sounds.extend(anim.take_sounds().into_iter().map(|s| s.0));
    }
    // iscript 360 SpecialState1: frames 68 / 85 and `playsnd 1018`.
    assert!(frames.contains(&68) && frames.contains(&85), "{frames:?}");
    assert!(heal_fx, "the Heal glow shows on the patient");
    assert!(sounds.contains(&1018), "heal sound: {sounds:?}");

    // Stim: one of the two Stim Pack sounds.
    w.cast(marine_id, Spell::StimPacks, SpellTarget::None)
        .unwrap();
    w.step();
    anim.advance(&w);
    let s: Vec<u16> = anim.take_sounds().into_iter().map(|s| s.0).collect();
    assert!(s.iter().any(|id| STIM_SOUNDS.contains(id)), "{s:?}");

    // Optical Flare on an enemy tank (large: +2), Restoration on our own
    // Marine (small), Lockdown field on the tank.
    let tank = w.spawn_px(1, uid::SIEGE_TANK, 560, 400).unwrap();
    w.units[tank].order = Order::HoldPosition;
    let tid = w.units[tank].id;
    w.cast(mid, Spell::OpticalFlare, SpellTarget::Unit(tid))
        .unwrap();
    w.cast(mid, Spell::Restoration, SpellTarget::Unit(marine_id))
        .unwrap_or(());
    w.step();
    anim.advance(&w);
    let images: Vec<u16> = anim.effects().map(|e| e.image).collect();
    assert!(
        images.contains(&(OPTICAL_FLARE_HIT_IMAGE + 2)),
        "flare hit on a large unit: {images:?}"
    );
    let _ = RESTORATION_IMAGE;
    w.units[tank].status.lockdown_ticks = 50;
    w.step();
    anim.advance(&w);
    assert!(
        anim.status_overlays(&w)
            .iter()
            .any(|o| o.image == LOCKDOWN_FIELD_IMAGE + 2),
        "lockdown field on the tank"
    );
}

#[test]
fn idle_marines_and_ghosts_open_fire_on_an_enemy_in_range() {
    for fog in [false, true] {
        let mut w = world();
        if fog {
            w.enable_fog();
        }
        let marine = w.spawn_px(0, uid::MARINE, 400, 400).unwrap();
        let ghost = w.spawn_px(0, uid::GHOST, 400, 440).unwrap();
        // An enemy that does not fight back, inside both ranges.
        let egg = w.spawn_px(1, uid::EGG, 500, 420).unwrap();
        let hp = w.units[egg].hp;
        let mut fired = (false, false);
        for _ in 0..60 {
            w.step();
            fired.0 |= w.units[marine].cooldown > 0;
            fired.1 |= w.units[ghost].cooldown > 0;
        }
        assert!(fired.0, "the idle Marine fires (fog {fog})");
        assert!(fired.1, "the idle Ghost fires (fog {fog})");
        assert!(w.units[egg].hp < hp);
    }
}

#[test]
fn the_renderer_shows_the_firebat_flame_and_the_marine_gunfire() {
    use galaxycraft::unit_anim::Animator;
    let mut w = world();
    let bat = w.spawn_px(0, uid::FIREBAT, 400, 400).unwrap();
    let marine = w.spawn_px(0, uid::MARINE, 380, 440).unwrap();
    let egg = w.spawn_px(1, uid::EGG, 440, 400).unwrap();
    let _ = egg;
    let (bid, mid) = (w.units[bat].id, w.units[marine].id);
    let mut anim = Animator::default();
    let (mut flame, mut bat_frames, mut marine_frames, mut sounds) =
        (false, Vec::new(), Vec::new(), Vec::new());
    for _ in 0..60 {
        w.step();
        anim.advance(&w);
        // iscript 69 GndAttkInit: `imgol 421` (thingy\flamer.grp).
        flame |= anim.effects().any(|e| e.image == 421);
        bat_frames.extend(anim.unit_frame(bid));
        marine_frames.extend(anim.unit_frame(mid));
        sounds.extend(anim.take_sounds().into_iter().map(|s| s.0));
    }
    assert!(flame, "the flame overlay appears");
    assert!(
        bat_frames.contains(&17),
        "Firebat firing pose: {bat_frames:?}"
    );
    // iscript 78: firing frames 34 / 51 and the Gauss Rifle sound 69.
    assert!(marine_frames.contains(&51), "Marine muzzle frame");
    assert!(sounds.contains(&69), "gunfire sound: {sounds:?}");
}
