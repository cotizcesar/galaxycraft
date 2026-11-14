use super::*;
mod cloak;

#[test]
fn the_psi_field_is_wide_rounded_and_only_the_owners() {
    // 16 x 10 tiles: 8 tiles to each side, 5 up and down (rounded).
    assert!(psi_field_covers(0, 0));
    assert!(psi_field_covers(7 * 32 + 31, 0), "eight tiles to the right");
    assert!(!psi_field_covers(8 * 32 + 31, 0));
    assert!(psi_field_covers(0, 4 * 32 + 31), "five tiles down");
    assert!(!psi_field_covers(0, 5 * 32 + 31));
    assert!(!psi_field_covers(7 * 32, 4 * 32), "the corners are cut");
    let mut w = World::new(
        Arc::new(GameData::bundled().unwrap()),
        crate::map::Terrain::open(64, 64),
        2,
    );
    let pylon = w.spawn_px(1, uid::PYLON, 640, 640).unwrap();
    w.units[pylon].completed = true;
    assert!(w.has_power_px(1, 700, 640));
    assert!(
        !w.has_power_px(0, 700, 640),
        "an enemy Pylon powers nothing of ours"
    );
}

fn open_world() -> World {
    World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(64, 64),
        2,
    )
}

#[test]
fn facing_follows_the_screen_compass_while_walking() {
    // A script walker (Marine) and a flingy (SCV, turn rate 40).
    for type_id in [uid::MARINE, uid::SCV] {
        for ((tx, ty), want) in [
            ((960, 640), 64),  // east
            ((640, 960), 128), // south
            ((320, 640), 192), // west
            ((640, 320), 0),   // north
        ] {
            let mut w = open_world();
            let i = w.spawn_px(0, type_id, 640, 640).unwrap();
            let id = w.units[i].id;
            w.issue(id, Order::Move { pos: (tx, ty) }).unwrap();
            for _ in 0..60 {
                w.step();
                let f = w.units[i].facing as i32;
                let diff = (f - want + 128).rem_euclid(256) - 128;
                if diff.abs() <= 2 && w.units[i].x != 640 * 256 {
                    break;
                }
            }
            let f = w.units[i].facing as i32;
            let diff = (f - want + 128).rem_euclid(256) - 128;
            assert!(
                diff.abs() <= 2,
                "type {type_id} target {tx},{ty}: facing {f}, want {want}"
            );
        }
    }
}

#[test]
fn cardinal_direction_vectors_point_north_east_south_west() {
    assert_eq!(direction_vector(0), (0, -256));
    assert_eq!(direction_vector(64), (256, 0));
    assert_eq!(direction_vector(128), (0, 256));
    assert_eq!(direction_vector(192), (-256, 0));
}

#[test]
fn rescuable_units_are_nobodys_target_and_join_the_first_comer() {
    let mut w = World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(64, 64),
        3,
    );
    w.players[2].rescuable = true;
    w.players[1].is_computer = true;
    // Positions in pixels: Raynor 20 px from the Marine, the other
    // rescuable unit ~2500 px away.
    let marine = w.spawn(0, uid::MARINE, 100, 100).unwrap();
    // units.dat 20: Jim Raynor (Marine), as placed by `terran04`.
    let raynor = w.spawn(2, 20, 120, 100).unwrap();
    let far = w.spawn(2, uid::MARINE, 1900, 1900).unwrap();
    // A computer enemy next to a rescuable unit does not take it.
    let held = w.spawn(2, uid::MARINE, 1000, 1000).unwrap();
    w.spawn(1, uid::MARINE, 1020, 1000).unwrap();
    let hp = w.units[raynor].hp;
    assert!(!w.hostile(0, 2) && !w.hostile(2, 1) && w.hostile(0, 1));
    // The retail handler polls on the first tick (Backwater Station's
    // Raynor line is on the first game frame), then every 15 ticks.
    w.step();
    assert_eq!(w.units[raynor].owner, 0, "rescued by the adjacent Marine");
    assert_eq!(w.units[marine].target, None);
    // One placed next to the Marine after the first poll waits for the
    // next one (tick 16) without being shot.
    let late = w.spawn(2, uid::MARINE, 100, 120).unwrap();
    for _ in 0..14 {
        w.step();
    }
    assert_eq!(w.units[late].owner, 2, "tick 15: not polled yet");
    w.step();
    assert_eq!(w.units[late].owner, 0, "tick 16: rescued");
    assert_eq!(w.units[raynor].hp, hp, "never attacked");
    assert_eq!(w.units[far].owner, 2, "nobody within reach");
    assert_eq!(w.units[held].owner, 2, "computers never rescue");
    assert_eq!(
        w.rescues,
        vec![(w.units[raynor].id, 0), (w.units[late].id, 0)]
    );
}

#[test]
fn speed_clamp_matches_the_recovered_status_rule() {
    // FUN_005b6900: +50% with a 0x355 floor, hero types 0x46/0x50/0x58
    // pinned at 0x6ab, ensnare halves, and both statuses cancel.
    assert_eq!(World::modified_speed(uid::MARINE, 1024, true, false), 1536);
    assert_eq!(World::modified_speed(uid::MARINE, 100, true, false), 0x355);
    assert_eq!(World::modified_speed(0x46, 100, true, false), 0x6ab);
    assert_eq!(World::modified_speed(0x50, 100, true, false), 0x6ab);
    assert_eq!(World::modified_speed(0x58, 100, true, false), 0x6ab);
    assert_eq!(World::modified_speed(uid::MARINE, 1024, false, true), 512);
    assert_eq!(World::modified_speed(uid::MARINE, 1024, true, true), 1024);
    assert_eq!(World::modified_speed(uid::MARINE, 1024, false, false), 1024);
}

#[test]
fn ticks_are_42_ms_of_real_time() {
    // Retail Desperate Alliance: game start 127.25 s, victory dialog
    // 1347.37 s, score "Elapsed Time: 20:20" (1220 s).
    let ticks = ((1347.37 - 127.25) * 1000.0 / TICK_MS as f64) as u64;
    assert_eq!(real_seconds(ticks), 1220);
    assert_eq!(real_seconds(28_800), 1209, "the 30-minute countdown");
}

#[test]
fn score_tallies_follow_production_spending_and_kills() {
    let mut w = open_world();
    w.players[0].minerals = 1000;
    let cc = w.spawn(0, uid::COMMAND_CENTER, 300, 300).unwrap();
    let before = w.players[0].clone();
    w.train(w.units[cc].id, uid::SCV).expect("train SCV");
    for _ in 0..24 * 30 {
        w.step();
    }
    let p = &w.players[0];
    let scv = w.data.unit(uid::SCV).clone();
    assert_eq!(p.units_produced, 1, "one SCV produced");
    assert_eq!(
        p.scores[1], scv.build_score as i32,
        "units score = build score"
    );
    assert_eq!(
        p.minerals_spent - before.minerals_spent,
        scv.mineral_cost as i32
    );
    // Placed and starting units count too (OpenBW `add_completed_unit`;
    // a fresh retail melee start shows Structures 400).
    assert_eq!(p.structures_built, 1, "the placed CC counts");
    // A kill credits the destroy score and the loss to the victim.
    let enemy = w.spawn(1, uid::MARINE, 1500, 1500).unwrap();
    let marine_destroy = w.data.unit(uid::MARINE).destroy_score as i32;
    let shooter = w.spawn(0, uid::MARINE, 1800, 1800).unwrap();
    w.units[enemy].last_damage_source = w.units[shooter].id;
    let id = w.units[enemy].id;
    w.damage_unit(id, 100_000).expect("damage");
    w.step();
    assert_eq!(w.players[0].scores[4], marine_destroy);
    assert_eq!(w.players[1].units_lost, 1);
}

#[test]
fn mission_worlds_leave_victory_to_the_triggers() {
    for trigger_victory in [false, true] {
        let mut w = open_world();
        w.trigger_victory = trigger_victory;
        w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let enemy = w.spawn(1, uid::MARINE, 50, 50).unwrap();
        w.units[enemy].alive = false;
        w.step();
        let want = if trigger_victory { None } else { Some(0) };
        assert_eq!(w.winner, want, "trigger_victory = {trigger_victory}");
    }
}

#[test]
fn miss_chances_are_exact_fractions_of_256() {
    // 255/256 under Dark Swarm, 119/256 on higher terrain, never 256/256.
    assert!(dodge_roll(0, 255));
    assert!(dodge_roll(254, 255));
    assert!(!dodge_roll(255, 255));
    assert!(dodge_roll(0, 119));
    assert!(dodge_roll(118, 119));
    assert!(!dodge_roll(119, 119));
    assert!(!dodge_roll(0, 0));
    // Over a full 256-value sweep exactly `chance` rolls miss.
    let misses = (0u32..256).filter(|r| dodge_roll(*r, 119)).count();
    assert_eq!(misses, 119);
    let misses = (0u32..256).filter(|r| dodge_roll(*r, 255)).count();
    assert_eq!(misses, 255);
}

#[test]
fn dark_swarm_only_affects_ground_targets() {
    let mut w = open_world();
    let shooter = w.spawn(0, uid::MARINE, 10, 10).unwrap();
    let ground = w.spawn(1, uid::MARINE, 20, 10).unwrap();
    let air = w.spawn(1, uid::WRAITH, 20, 11).unwrap();
    assert_eq!(w.dodge_chance(shooter, ground), 0);
    // A target standing one elevation class higher misses 119/256.
    w.terrain.set_elevation_cell(20 / 8, 10 / 8, 2);
    assert_eq!(w.dodge_chance(shooter, ground), 119);
    w.terrain.set_elevation_cell(20 / 8, 10 / 8, 0);
    // A Dark Swarm cloud at the target's position.
    w.spawn(0, uid::DARK_SWARM, 20, 10).unwrap();
    assert_eq!(w.dodge_chance(shooter, ground), 255);
    assert_eq!(w.dodge_chance(shooter, air), 0);
}

#[test]
fn marines_fight_and_damage_matches_the_pipeline() {
    let mut w = open_world();
    let a = w.spawn(0, uid::MARINE, 10, 10).unwrap();
    let b = w.spawn(1, uid::MARINE, 12, 10).unwrap();
    let start_hp = w.units[b].hp;
    for _ in 0..16 {
        w.step();
        if w.units[b].hp < start_hp {
            break;
        }
    }
    assert!(w.units[b].hp < start_hp);
    // Cooldown 15 (+ -1..=+2) starts with the animation; the hit lands 5 frames in.
    let cd = w.units[a].cooldown;
    assert!((9..=12).contains(&cd), "{cd}");
}

#[test]
fn attack_animation_supplies_the_first_shot_timing() {
    let mut w = open_world();
    let _a = w.spawn(0, uid::MARINE, 10, 10).unwrap();
    let b = w.spawn(1, uid::MARINE, 12, 10).unwrap();
    let start = w.units[b].hp;
    let mut fired = None;
    for t in 1..=10 {
        w.step();
        if w.units[b].hp < start {
            fired = Some(t);
            break;
        }
    }
    assert_eq!(
        fired,
        Some(6),
        "the Marine attackwith opcode is reached on the fourth logical tick \
         and its bullet's domissiledmg two frames later"
    );
}

#[test]
fn worker_mines_and_returns() {
    let mut w = open_world();
    let cc = w.spawn(0, uid::COMMAND_CENTER, 4, 4).unwrap();
    let mf = w.spawn_px(0, uid::MINERAL_FIELD, 12 * 32, 12 * 32).unwrap();
    w.units[mf].resource_amount = 1500;
    let scv = w.spawn(0, uid::SCV, 8, 8).unwrap();
    w.issue(
        w.units[scv].id,
        Order::Gather {
            resource: w.units[mf].id,
        },
    )
    .unwrap();
    for _ in 0..2000 {
        w.step();
        if w.players[0].minerals > 50 {
            break;
        }
    }
    assert!(w.players[0].minerals > 50, "worker should deposit minerals");
    let _ = cc;
}

#[test]
fn production_requires_the_right_producer_and_supply() {
    let mut w = open_world();
    let rax = w.spawn(0, uid::BARRACKS, 10, 10).unwrap();
    // A Barracks has no supply of its own without a Command Center, so a
    // Marine cannot be queued even though the producer matches.
    assert_eq!(
        w.train(w.units[rax].id, uid::MARINE),
        Err(SimError::NotEnoughSupply)
    );
    let cc = w.spawn(0, uid::COMMAND_CENTER, 4, 4).unwrap();
    let _ = cc;
    assert_eq!(
        w.train(w.units[rax].id, uid::SCV),
        Err(SimError::MissingRequirements)
    );
}

#[test]
fn barracks_trains_a_marine() {
    let mut w = open_world();
    let cc = w.spawn(0, uid::COMMAND_CENTER, 4, 4).unwrap();
    let depot = w.spawn(0, uid::SUPPLY_DEPOT, 8, 4).unwrap();
    let rax = w.spawn(0, uid::BARRACKS, 12, 4).unwrap();
    w.players[0].minerals = 1000;
    w.train(w.units[rax].id, uid::MARINE).unwrap();
    for _ in 0..1000 {
        w.step();
        if w.units.iter().any(|u| u.alive && u.type_id == uid::MARINE) {
            break;
        }
    }
    assert!(w.units.iter().any(|u| u.alive && u.type_id == uid::MARINE));
    let _ = (cc, depot);
}

#[test]
fn tech_and_upgrade_are_tracked_per_player() {
    let mut w = open_world();
    let academy = w.spawn(0, uid::ACADEMY, 8, 8).unwrap();
    w.players[0].minerals = 1000;
    w.players[0].gas = 1000;
    w.research(w.units[academy].id, tech_id::STIM_PACKS)
        .unwrap();
    for _ in 0..3000 {
        w.step();
        if w.players[0].has_tech(tech_id::STIM_PACKS) {
            break;
        }
    }
    assert!(w.players[0].has_tech(tech_id::STIM_PACKS));
}

#[test]
fn upgrade_costs_and_times_use_the_current_level() {
    let mut w = open_world();
    let bay = w.spawn(0, uid::ENGINEERING_BAY, 8, 8).unwrap();
    let bay_id = w.units[bay].id;
    w.players[0].minerals = 1000;
    w.players[0].gas = 1000;

    w.upgrade(bay_id, upgrade::TERRAN_INFANTRY_WEAPONS).unwrap();
    assert_eq!((w.players[0].minerals, w.players[0].gas), (900, 900));
    assert_eq!(w.units[bay].build_remaining, 4000);
    assert_eq!(w.units[bay].build_total, 4000);

    w.units[bay].build_remaining = 1;
    w.step();
    assert_eq!(
        w.players[0].upgrade_level(upgrade::TERRAN_INFANTRY_WEAPONS),
        1
    );

    w.spawn(0, uid::SCIENCE_FACILITY, 16, 8).unwrap();
    w.upgrade(bay_id, upgrade::TERRAN_INFANTRY_WEAPONS).unwrap();
    assert_eq!((w.players[0].minerals, w.players[0].gas), (725, 725));
    assert_eq!(w.units[bay].build_remaining, 4480);
    assert_eq!(w.units[bay].build_total, 4480);
}

#[test]
fn energy_caps_follow_the_retail_reactor_table_and_hero_flag() {
    const BASE: i32 = MAX_ENERGY * ENERGY_SCALE;
    const CAP: i32 = MAX_ENERGY_UPGRADED * ENERGY_SCALE;
    let reactors = [
        (uid::GHOST, upgrade::MOEBIUS_REACTOR),
        (uid::WRAITH, upgrade::APOLLO_REACTOR),
        (uid::SCIENCE_VESSEL, upgrade::TITAN_REACTOR),
        (uid::BATTLECRUISER, upgrade::COLOSSUS_REACTOR),
        (uid::MEDIC, upgrade::CADUCEUS_REACTOR),
        (uid::QUEEN, upgrade::GAMETE_MEIOSIS),
        (uid::DEFILER, upgrade::METASYNAPTIC_NODE),
        (uid::CORSAIR, upgrade::ARGUS_JEWEL),
        (uid::DARK_ARCHON, upgrade::ARGUS_TALISMAN),
        (uid::HIGH_TEMPLAR, upgrade::KHAYDARIN_AMULET),
        (uid::ARBITER, upgrade::KHAYDARIN_CORE),
    ];
    for (unit, upgrade_id) in reactors {
        let mut w = open_world();
        let i = w.spawn(0, unit, 10, 10).unwrap();
        w.units[i].set_energy_fixed(CAP);
        assert_eq!(w.units[i].energy_fixed(), BASE, "unit {unit}");
        w.players[0].upgrades[upgrade_id as usize] = 1;
        w.refresh_energy_caps();
        w.units[i].set_energy_fixed(CAP);
        assert_eq!(w.units[i].energy_fixed(), CAP, "unit {unit}");
    }
    let mut w = open_world();
    let hero = w
        .data
        .units
        .iter()
        .position(|u| u.is_hero() && u.flags & 0x0020_0000 != 0)
        .expect("a spellcasting hero") as u16;
    let i = w.spawn(0, hero, 10, 10).unwrap();
    w.units[i].set_energy_fixed(CAP);
    assert_eq!(w.units[i].energy_fixed(), CAP, "hero energy cap");
}

#[test]
fn siege_splash_skips_air_and_spares_its_source() {
    let mut w = open_world();
    let siege = w.spawn(0, uid::SIEGE_MODE_TURRET, 320, 320).unwrap();
    let target = w.spawn(1, uid::MARINE, 420, 320).unwrap();
    let wraith = w.spawn(1, uid::WRAITH, 440, 320).unwrap();
    let friendly = w.spawn(0, uid::MARINE, 430, 320).unwrap();
    let weapon = w.data.weapon(wid::ARCLITE_SHOCK_CANNON).cloned().unwrap();
    let (siege_hp, friendly_hp, wraith_hp) =
        (w.units[siege].hp, w.units[friendly].hp, w.units[wraith].hp);
    let pos = (w.units[target].x, w.units[target].y);
    w.resolve_hit_type(
        0,
        Some(w.units[target].id),
        siege,
        wid::ARCLITE_SHOCK_CANNON,
        &weapon,
        pos,
        1,
        1,
    );
    assert_eq!(w.units[siege].hp, siege_hp, "splash never hits its source");
    assert_eq!(
        w.units[wraith].hp, wraith_hp,
        "ground-only splash skips air"
    );
    assert!(
        w.units[friendly].hp < friendly_hp,
        "same-owner units still take radial splash"
    );
}

#[test]
fn siege_mode_switches_unit_type() {
    let mut w = open_world();
    let tank = w.spawn(0, uid::SIEGE_TANK, 8, 8).unwrap();
    w.players[0].techs[tech_id::TANK_SIEGE_MODE as usize] = true;
    w.issue(w.units[tank].id, Order::Siege).unwrap();
    // It first turns to heading 96 (13 per frame from 0: 8 frames).
    for _ in 0..8 {
        w.step();
        assert_eq!(w.units[tank].type_id, uid::SIEGE_TANK);
    }
    w.step();
    assert_eq!(w.units[tank].type_id, uid::SIEGE_MODE);
    // Busy for the deploy animation, then idle again.
    assert!(w.is_transforming(tank));
    for _ in 0..=SIEGE_FRAMES {
        w.step();
    }
    assert!(!w.is_transforming(tank));
    assert_eq!(w.units[tank].order, Order::Idle);
}

#[test]
fn travelling_weapons_create_a_bullet_and_hit_on_arrival() {
    let mut w = open_world();
    // Missile Turret air weapons are BulletType 1 (follow target), unlike
    // the Marine's instant appear-at-target rifle.
    let a = w.spawn(0, uid::MISSILE_TURRET, 320, 320).unwrap();
    let b = w.spawn(1, uid::DROPSHIP, 480, 320).unwrap();
    let start = w.units[b].hp;
    for _ in 0..16 {
        w.step();
        if !w.projectiles.is_empty() {
            break;
        }
    }
    assert!(!w.projectiles.is_empty(), "the turret missile must travel");
    assert_eq!(w.units[b].hp, start, "damage is not applied at fire time");
    for _ in 0..120 {
        w.step();
        if w.units[b].hp < start {
            break;
        }
    }
    assert!(
        w.units[b].hp < start,
        "the bullet must deal damage on impact"
    );
    let _ = a;
}

#[test]
fn instant_weapons_still_hit_on_the_firing_tick() {
    let mut w = open_world();
    let a = w.spawn(0, uid::MARINE, 10, 10).unwrap();
    let b = w.spawn(1, uid::MARINE, 12, 10).unwrap();
    let start = w.units[b].hp;
    for _ in 0..16 {
        w.step();
        if w.units[b].hp < start {
            break;
        }
    }
    assert!(w.projectiles.is_empty(), "Gauss Rifle is instant");
    assert!(w.units[b].hp < start);
    let _ = a;
}

#[test]
fn cloak_requires_a_detector() {
    let mut w = open_world();
    let ghost = w.spawn(0, uid::GHOST, 10, 10).unwrap();
    w.players[0].techs[tech_id::PERSONNEL_CLOAKING as usize] = true;
    w.set_cloak(w.units[ghost].id, true).unwrap();
    let marine = w.spawn(1, uid::MARINE, 12, 10).unwrap();
    for _ in 0..20 {
        w.step();
    }
    assert_eq!(
        w.units[ghost].hp,
        w.data.unit(uid::GHOST).hit_points_raw as i32
    );
    // A Missile Turret is a detector and makes the Ghost targetable.
    let turret = w.spawn(1, uid::MISSILE_TURRET, 14, 10).unwrap();
    let start = w.units[ghost].hp;
    for _ in 0..50 {
        w.step();
        if w.units[ghost].hp < start {
            break;
        }
    }
    assert!(
        w.units[ghost].hp < start,
        "detector should reveal the Ghost"
    );
    let _ = (marine, turret);
}

#[test]
fn detector_radius_is_seven_tiles_in_fixed_point_space() {
    // Unit positions are 1/256 px, the detector radius px (comparing them
    // raw made it under a pixel).  The recovered building radius is 7 tiles.
    let mut w = open_world();
    let ghost = w.spawn(0, uid::GHOST, 320, 320).unwrap();
    w.players[0].techs[tech_id::PERSONNEL_CLOAKING as usize] = true;
    w.set_cloak(w.units[ghost].id, true).unwrap();
    let turret = w.spawn(1, uid::MISSILE_TURRET, 320 + 6 * 32, 320).unwrap();
    assert!(w.can_see_unit(1, ghost), "six tiles is inside the radius");
    w.units[turret].x = (320 + 7 * 32) * fp(1); // exactly seven tiles
    assert!(w.can_see_unit(1, ghost));
    w.units[turret].x = (320 + 8 * 32) * fp(1); // eight tiles
    assert!(!w.can_see_unit(1, ghost));
}

#[test]
fn energy_regenerates_at_eight_256ths_per_tick() {
    // `CUnit+0xa2` is fixed point and the update routine adds 8 each
    // logical tick, so one whole energy point takes 32 ticks.
    let mut w = open_world();
    let ghost = w.spawn(0, uid::GHOST, 10, 10).unwrap();
    w.units[ghost].set_energy_fixed(0);
    for _ in 0..32 {
        w.step();
    }
    assert_eq!(w.units[ghost].energy_fixed(), ENERGY_SCALE);
}

#[test]
fn personal_cloak_drains_its_256ths_and_suppresses_regen() {
    let mut w = open_world();
    let ghost = w.spawn(0, uid::GHOST, 10, 10).unwrap();
    w.players[0].techs[tech_id::PERSONNEL_CLOAKING as usize] = true;
    w.units[ghost].set_energy_fixed(MAX_ENERGY * ENERGY_SCALE);
    w.set_cloak(w.units[ghost].id, true).unwrap();
    for _ in 0..25 {
        w.step();
    }
    // 25 ticks * 10/256 drained, no +8 regen while cloaked.
    assert_eq!(
        w.units[ghost].energy_fixed(),
        // 25 energy to cloak (techdata), then the drain.
        MAX_ENERGY * ENERGY_SCALE - 25 * ENERGY_SCALE - 25 * GHOST_CLOAK_COST_PER_TICK
    );
    assert!(w.units[ghost].status.cloaked);

    let wraith = w.spawn(0, uid::WRAITH, 20, 20).unwrap();
    w.players[0].techs[tech_id::CLOAKING_FIELD as usize] = true;
    // Under the 25-energy cost the cloak is refused.
    w.units[wraith].set_energy_fixed(1000);
    assert_eq!(
        w.set_cloak(w.units[wraith].id, true),
        Err(SimError::NotEnoughEnergy)
    );
    w.units[wraith].set_energy_fixed(1000 + 25 * ENERGY_SCALE);
    w.set_cloak(w.units[wraith].id, true).unwrap();
    w.step();
    assert_eq!(
        w.units[wraith].energy_fixed(),
        1000 - WRAITH_CLOAK_COST_PER_TICK
    );
}

#[test]
fn medic_heals_200_256ths_for_100_256ths_energy() {
    let mut w = open_world();
    let medic = w.spawn(0, uid::MEDIC, 320, 320).unwrap();
    let marine = w.spawn(0, uid::MARINE, 340, 320).unwrap();
    w.units[marine].hp = fp(10);
    w.units[medic].set_energy_fixed(ENERGY_SCALE);
    let before = w.units[marine].hp;
    w.step();
    assert_eq!(w.units[marine].hp - before, MEDIC_HEAL_PER_TICK);
    // +8 regen then -100 heal cost.
    assert_eq!(
        w.units[medic].energy_fixed(),
        ENERGY_SCALE + ENERGY_REGEN_PER_TICK - MEDIC_HEAL_COST_PER_TICK
    );
}

#[test]
fn acid_spores_stack_nine_independent_150_update_timers() {
    // `FUN_005dc360` increments `CUnit+0x126` up to nine and writes
    // `0x96` into one of the nine `CUnit+0x127..+0x12f` slots;
    // `FUN_005dc940` expires each slot independently every 8th tick.
    let mut status = Status::default();
    for _ in 0..12 {
        status.add_acid_spore();
    }
    assert_eq!(status.acid_spores, 9);
    assert!(status.acid_spore_ticks.iter().all(|&t| t > 0));
    for _ in 0..ACID_SPORE_TICKS - 1 {
        status.tick_acid_spores();
    }
    assert_eq!(status.acid_spores, 9, "stacks last 150 updates");
    status.tick_acid_spores();
    assert_eq!(status.acid_spores, 0);
    assert_eq!(status.acid_spore_ticks, [0; 9]);
}

#[test]
fn acid_spore_refreshes_the_stack_closest_to_expiring_when_full() {
    // With all nine slots busy the apply routine scans for the smallest
    // remaining timer and refreshes that slot, not the first one.
    let mut status = Status {
        acid_spore_ticks: [150, 10, 90, 120, 60, 30, 140, 80, 20],
        acid_spores: 9,
        ..Status::default()
    };
    status.add_acid_spore();
    assert_eq!(status.acid_spores, 9);
    assert_eq!(status.acid_spore_ticks[1], ACID_SPORE_TICKS);
    assert_eq!(status.acid_spore_ticks[0], 150);
}

#[test]
fn acid_spore_timers_drop_one_stack_per_expiry() {
    let mut status = Status {
        acid_spore_ticks: [1, 2, 0, 0, 0, 0, 0, 0, 0],
        acid_spores: 2,
        ..Status::default()
    };
    status.tick_acid_spores();
    assert_eq!(status.acid_spores, 1);
    assert_eq!(status.acid_spore_ticks[0], 0);
    assert_eq!(status.acid_spore_ticks[1], 1);
    status.tick_acid_spores();
    assert_eq!(status.acid_spores, 0);
}

#[test]
fn corrosive_acid_spores_the_128_px_box_but_damages_one_target() {
    // `FUN_005dc1c0` passes a 0x40 half-extent to `FUN_006c1ee0`, which
    // calls `FUN_005dc360` for every unit in the resulting 128 x 128 box.
    // Weapon 104's damage splash radii are all zero, so only the target
    // takes the 25 damage while nearby air units only gain a stack.
    let mut w = open_world();
    let queen = w.spawn(0, uid::QUEEN, 320, 320).unwrap();
    let a = w.spawn(1, uid::WRAITH, 360, 320).unwrap();
    let b = w.spawn(1, uid::WRAITH, 380, 320).unwrap();
    let friendly = w.spawn(0, uid::WRAITH, 380, 340).unwrap();
    let far = w.spawn(1, uid::WRAITH, 460, 320).unwrap();
    let weapon = w.data.weapon(104).cloned().expect("corrosive acid");
    assert_eq!(weapon.explosion, crate::projectile::hit::CORROSIVE_ACID);
    assert_eq!(
        (
            weapon.inner_splash,
            weapon.middle_splash,
            weapon.outer_splash
        ),
        (0, 0, 0)
    );
    let (hp_a, hp_b) = (w.units[a].hp, w.units[b].hp);
    let pos = (w.units[a].x, w.units[a].y);
    w.resolve_hit_type(0, Some(w.units[a].id), queen, 104, &weapon, pos, 1, 1);
    assert!(w.units[a].hp < hp_a, "the struck unit takes damage");
    assert_eq!(w.units[b].hp, hp_b, "no splash damage");
    assert_eq!(w.units[a].status.acid_spores, 1);
    assert_eq!(w.units[b].status.acid_spores, 1);
    assert_eq!(
        w.units[friendly].status.acid_spores, 0,
        "same owner as the attacker"
    );
    assert_eq!(w.units[far].status.acid_spores, 0);
}

#[test]
fn acid_spore_stacks_expire_in_the_world_tick() {
    let mut w = open_world();
    let queen = w.spawn(0, uid::QUEEN, 320, 320).unwrap();
    let bc = w.spawn(1, uid::BATTLECRUISER, 360, 320).unwrap();
    let weapon = w.data.weapon(104).cloned().expect("corrosive acid");
    let pos = (w.units[bc].x, w.units[bc].y);
    w.resolve_hit_type(0, Some(w.units[bc].id), queen, 104, &weapon, pos, 1, 1);
    assert_eq!(w.units[bc].status.acid_spores, 1);
    let expire_ticks = u64::from(ACID_SPORE_TICKS) * ACID_SPORE_UPDATE_INTERVAL;
    for _ in 0..expire_ticks - 1 {
        w.step();
    }
    assert_eq!(w.units[bc].status.acid_spores, 1);
    w.step();
    assert_eq!(w.units[bc].status.acid_spores, 0);
}

#[test]
fn restoration_clears_acid_spores_and_parasite() {
    let mut w = open_world();
    let medic = w.spawn(0, uid::MEDIC, 320, 320).unwrap();
    let bc = w.spawn(0, uid::BATTLECRUISER, 340, 320).unwrap();
    w.units[bc].status.add_acid_spore();
    w.units[bc].status.add_acid_spore();
    w.units[bc].status.parasite = true;
    assert_eq!(w.units[bc].status.acid_spores, 2);
    w.apply_spell(medic, Spell::Restoration, SpellTarget::Unit(w.units[bc].id));
    // Its bullet (iscript 292) lands seven frames after the cast.
    assert_eq!(w.units[bc].status.acid_spores, 2);
    for _ in 0..8 {
        w.step();
    }
    assert_eq!(w.units[bc].status.acid_spores, 0);
    assert_eq!(w.units[bc].status.acid_spore_ticks, [0; 9]);
    assert!(!w.units[bc].status.parasite);
}

#[test]
fn restoration_worthwhile_matches_fun_005c7460() {
    let mut w = open_world();
    let medic = w.spawn(0, uid::MEDIC, 320, 320).unwrap();
    let bc = w.spawn(0, uid::BATTLECRUISER, 340, 320).unwrap();
    let (medic_id, bc_id) = (w.units[medic].id, w.units[bc].id);
    assert!(
        !w.restoration_worthwhile(medic_id, bc_id),
        "nothing to cure"
    );
    w.units[bc].status.plague_ticks = 100;
    assert!(w.restoration_worthwhile(medic_id, bc_id));

    // 40 HP + 30 shields = 70 whole points passes the 0x3d threshold;
    // 40 + 20 = 60 fails it.
    let zealot = w.spawn(0, uid::ZEALOT, 360, 320).unwrap();
    w.units[zealot].status.ensnare_ticks = 100;
    w.units[zealot].hp = fp(40);
    w.units[zealot].shields = fp(30);
    assert!(w.restoration_worthwhile(medic_id, w.units[zealot].id));
    w.units[zealot].shields = fp(20);
    assert!(!w.restoration_worthwhile(medic_id, w.units[zealot].id));

    let enemy = w.spawn(1, uid::ZEALOT, 380, 320).unwrap();
    w.units[enemy].status.plague_ticks = 100;
    assert!(!w.restoration_worthwhile(medic_id, w.units[enemy].id));

    let cc = w.spawn(0, uid::COMMAND_CENTER, 400, 320).unwrap();
    w.units[cc].status.plague_ticks = 100;
    assert!(!w.restoration_worthwhile(medic_id, w.units[cc].id));
}

#[test]
fn acid_spores_reveal_a_cloaked_unit() {
    let mut w = open_world();
    let ghost = w.spawn(1, uid::GHOST, 320, 320).unwrap();
    w.units[ghost].status.cloaked = true;
    w.units[ghost].status.revealed_ticks = 0;
    assert!(!w.can_see_unit(0, ghost), "no detector");
    w.units[ghost].status.add_acid_spore();
    assert!(w.can_see_unit(0, ghost), "spores reveal cloaking");
}

#[test]
fn air_units_do_not_block_ground_paths() {
    let mut w = open_world();
    let wraith = w.spawn(0, uid::WRAITH, 320, 320).unwrap();
    w.units[wraith].completed = true;
    w.build_obstacles();
    assert!(w.obstacles.is_empty(), "flyers are not ground obstacles");
}

#[test]
fn door_units_are_not_targets_or_colliders() {
    // `FUN_005b5b00` returns true for the four door types 0xcd..0xd0.
    let mut w = open_world();
    for ty in 0xcd..=0xd0 {
        assert!(super::is_door_type(ty));
    }
    assert!(!super::is_door_type(uid::MARINE));
    let door = w.spawn(1, 0xcd, 340, 320).unwrap();
    w.units[door].completed = true;
    assert!(w.collision_extents(door).is_none());
    let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert!(
        w.acquire_target(marine).is_none(),
        "doors are excluded from automatic targeting"
    );
}

#[test]
fn closed_door_is_a_path_obstacle_and_open_door_is_not() {
    let mut w = open_world();
    let door = w.spawn(0, 0xcd, 340, 320).unwrap();
    w.units[door].completed = true;
    w.units[door].doodad_enabled = Some(true);
    w.build_obstacles();
    assert!(w.collision_extents(door).is_none());
    assert!(w.obstacles.blocks_point(340, 320));

    w.units[door].doodad_enabled = Some(false);
    w.build_obstacles();
    assert!(w.obstacles.is_empty());
    assert!(!w.obstacles.blocks_point(340, 320));
}

#[test]
fn path_obstacles_use_the_asymmetric_units_dat_box() {
    // Barracks extents are left 48, top 40, right 56, bottom 32.  The
    // old symmetric `max` made it block 56/40 px on every side; the
    // recovered path context keeps the four directions separate.
    let mut w = open_world();
    let barracks = w.spawn(0, uid::BARRACKS, 320, 320).unwrap();
    w.units[barracks].completed = true;
    w.build_obstacles();
    assert_eq!(w.obstacles.len(), 1);
    assert!(w.obstacles.blocks_point(272, 320));
    assert!(!w.obstacles.blocks_point(271, 320));
    assert!(w.obstacles.blocks_point(375, 320));
    assert!(!w.obstacles.blocks_point(376, 320));
    assert!(w.obstacles.blocks_point(320, 280));
    assert!(!w.obstacles.blocks_point(320, 279));
    assert!(w.obstacles.blocks_point(320, 351));
    assert!(!w.obstacles.blocks_point(320, 352));
}

#[test]
fn feedback_deals_the_full_drained_energy_through_matrix() {
    let mut w = open_world();
    w.players[0].techs[tech_id::FEEDBACK as usize] = true;
    let archon = w.spawn(0, uid::DARK_ARCHON, 320, 320).unwrap();
    let bc = w.spawn(1, uid::BATTLECRUISER, 340, 320).unwrap();
    w.units[bc].set_energy_fixed(200 * ENERGY_SCALE);
    let before = w.units[bc].hp;
    w.cast(
        w.units[archon].id,
        Spell::Feedback,
        SpellTarget::Unit(w.units[bc].id),
    )
    .unwrap();
    assert_eq!(w.units[bc].hp, before - 200 * fp(1));
    assert_eq!(w.units[bc].energy_fixed(), 0);

    // Defensive Matrix absorbs the feedback damage first.
    let bc2 = w.spawn(1, uid::BATTLECRUISER, 360, 320).unwrap();
    w.units[bc2].set_energy_fixed(100 * ENERGY_SCALE);
    w.units[bc2].status.apply_matrix(fp(40));
    let before = w.units[bc2].hp;
    w.cast(
        w.units[archon].id,
        Spell::Feedback,
        SpellTarget::Unit(w.units[bc2].id),
    )
    .unwrap();
    assert_eq!(w.units[bc2].status.matrix, 0);
    assert_eq!(w.units[bc2].hp, before - 60 * fp(1));
}

#[test]
fn defensive_matrix_absorbs_and_breaks_like_fun_005d2300() {
    let mut status = Status::default();
    status.apply_matrix(1000);
    assert!(status.matrix_active());
    assert_eq!(status.matrix_ticks, DEFENSIVE_MATRIX_UPDATES);
    assert_eq!(status.absorb_matrix(400), (400, false));
    assert_eq!(status.matrix, 600);
    assert!(status.matrix_active());
    assert_eq!(status.matrix_ticks, DEFENSIVE_MATRIX_UPDATES);
    assert_eq!(status.absorb_matrix(600), (600, true));
    assert_eq!(status.matrix, 0);
    assert!(!status.matrix_active());
    assert_eq!(status.matrix_ticks, 0);
    // Overkill consumes only what is left, even if the hit is larger.
    status.apply_matrix(250);
    assert_eq!(status.absorb_matrix(10_000), (250, true));
    assert_eq!(status.matrix, 0);
    assert!(!status.matrix_active());
}

#[test]
fn defensive_matrix_expires_after_168_status_updates() {
    let mut status = Status::default();
    status.apply_matrix(64000);
    for _ in 0..DEFENSIVE_MATRIX_UPDATES - 1 {
        assert_eq!(status.tick_matrix(), None);
    }
    assert_eq!(status.matrix, 64000);
    assert_eq!(status.tick_matrix(), Some(64000));
    assert_eq!(status.matrix, 0);
    assert!(!status.matrix_active());
    assert_eq!(status.tick_matrix(), None);
}

#[test]
fn defensive_matrix_expires_in_the_world_tick() {
    let mut w = open_world();
    let bc = w.spawn(0, uid::BATTLECRUISER, 320, 320).unwrap();
    w.units[bc].status.apply_matrix(fp(250));
    let expire_ticks = u64::from(DEFENSIVE_MATRIX_UPDATES) * ACID_SPORE_UPDATE_INTERVAL;
    for _ in 0..expire_ticks - 1 {
        w.step();
    }
    assert!(w.units[bc].status.matrix_active());
    w.step();
    assert!(!w.units[bc].status.matrix_active());
    assert_eq!(w.units[bc].status.matrix, 0);
    assert_eq!(w.matrix_cues.len(), 1);
    assert!(w.matrix_cues[0].broke);
    assert_eq!(w.matrix_cues[0].absorbed, fp(250));
}

#[test]
fn matrix_cues_record_partial_hits_and_the_break() {
    let mut w = open_world();
    let vessel = w.spawn(0, uid::SCIENCE_VESSEL, 320, 320).unwrap();
    let bc = w.spawn(0, uid::BATTLECRUISER, 340, 320).unwrap();
    w.apply_spell(
        vessel,
        Spell::DefensiveMatrix,
        SpellTarget::Unit(w.units[bc].id),
    );
    assert_eq!(w.units[bc].status.matrix, fp(250));
    assert!(w.units[bc].status.matrix_active());
    assert_eq!(w.units[bc].status.matrix_ticks, DEFENSIVE_MATRIX_UPDATES);

    let marine = w.spawn(1, uid::MARINE, 360, 320).unwrap();
    let rifle = w.data.weapon(wid::GAUSS_RIFLE).cloned().unwrap();
    let pos = (w.units[bc].x, w.units[bc].y);
    w.resolve_hit_type(
        1,
        Some(w.units[bc].id),
        marine,
        wid::GAUSS_RIFLE,
        &rifle,
        pos,
        1,
        1,
    );
    assert_eq!(w.matrix_cues.len(), 1);
    assert_eq!(w.matrix_cues[0].absorbed, fp(6));
    assert!(!w.matrix_cues[0].broke, "the matrix survives one rifle hit");
    assert_eq!(w.units[bc].status.matrix, fp(244));
    assert!(w.units[bc].status.matrix_active());

    // A hit larger than the remaining pool consumes the pool, clears the
    // overlay flag and emits the break cue.
    let mut heavy = rifle.clone();
    heavy.damage = 300;
    w.resolve_hit_type(
        1,
        Some(w.units[bc].id),
        marine,
        wid::GAUSS_RIFLE,
        &heavy,
        pos,
        1,
        1,
    );
    assert_eq!(w.matrix_cues.len(), 2);
    assert_eq!(w.matrix_cues[1].absorbed, fp(244));
    assert!(w.matrix_cues[1].broke);
    assert_eq!(w.units[bc].status.matrix, 0);
    assert!(!w.units[bc].status.matrix_active());

    // Without an active matrix no further cue is produced.
    w.resolve_hit_type(
        1,
        Some(w.units[bc].id),
        marine,
        wid::GAUSS_RIFLE,
        &rifle,
        pos,
        1,
        1,
    );
    assert_eq!(w.matrix_cues.len(), 2);

    // Cues belong to the tick that produced them.
    w.step();
    assert!(w.matrix_cues.is_empty());
}

#[test]
fn status_timers_use_the_recovered_eight_tick_updates() {
    let mut w = open_world();
    let marine = w.spawn(1, uid::MARINE, 320, 320).unwrap();
    {
        let s = &mut w.units[marine].status;
        s.lockdown_ticks = 2;
        s.ensnare_ticks = 2;
        s.maelstrom_ticks = 2;
    }
    for _ in 0..7 {
        w.step();
    }
    assert_eq!(w.units[marine].status.lockdown_ticks, 2);
    assert_eq!(w.units[marine].status.ensnare_ticks, 2);
    assert_eq!(w.units[marine].status.maelstrom_ticks, 2);
    w.step();
    assert_eq!(w.units[marine].status.lockdown_ticks, 1);
    assert_eq!(w.units[marine].status.ensnare_ticks, 1);
    assert_eq!(w.units[marine].status.maelstrom_ticks, 1);
}

#[test]
fn ensnare_affects_air_but_not_buildings_burrowed_or_the_caster() {
    let mut w = open_world();
    let caster = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let air = w.spawn(1, uid::MUTALISK, 350, 320).unwrap();
    let building = w.spawn(1, uid::HATCHERY, 340, 340).unwrap();
    w.units[building].completed = true;
    let burrowed = w.spawn(1, uid::ZERGLING, 330, 320).unwrap();
    w.units[burrowed].status.burrowed = true;
    let ground = w.spawn(1, uid::ZERGLING, 360, 320).unwrap();
    w.apply_spell(caster, Spell::Ensnare, SpellTarget::Point(340, 320));
    assert_eq!(w.units[air].status.ensnare_ticks, ENSNARE_UPDATES);
    assert_eq!(w.units[ground].status.ensnare_ticks, ENSNARE_UPDATES);
    assert_eq!(w.units[building].status.ensnare_ticks, 0);
    assert_eq!(w.units[burrowed].status.ensnare_ticks, 0);
    assert_eq!(w.units[caster].status.ensnare_ticks, 0);
}

#[test]
fn area_status_spells_skip_burrowed_and_stasis_maelstrom_skip_buildings() {
    let mut w = open_world();
    let caster = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let buried = w.spawn(1, uid::ZERGLING, 360, 320).unwrap();
    w.units[buried].status.burrowed = true;
    let building = w.spawn(1, uid::HATCHERY, 380, 320).unwrap();
    w.units[building].completed = true;
    let normal = w.spawn(1, uid::ZERGLING, 340, 320).unwrap();
    let point = SpellTarget::Point(360, 320);

    w.apply_spell(caster, Spell::Ensnare, point);
    assert_eq!(w.units[buried].status.ensnare_ticks, 0);
    assert_eq!(w.units[normal].status.ensnare_ticks, ENSNARE_UPDATES);

    w.apply_spell(caster, Spell::Plague, point);
    assert_eq!(w.units[buried].status.plague_ticks, 0);
    assert_eq!(w.units[normal].status.plague_ticks, PLAGUE_UPDATES);

    w.apply_spell(caster, Spell::StasisField, point);
    assert_eq!(w.units[buried].status.stasis_ticks, 0);
    assert_eq!(w.units[building].status.stasis_ticks, 0);
    assert_eq!(w.units[normal].status.stasis_ticks, STASIS_UPDATES);

    w.apply_spell(caster, Spell::Maelstrom, point);
    assert_eq!(w.units[buried].status.maelstrom_ticks, 0);
    assert_eq!(w.units[building].status.maelstrom_ticks, 0);
    assert_eq!(w.units[normal].status.maelstrom_ticks, MAELSTROM_UPDATES);
}

#[test]
fn stasis_expires_on_the_eighth_tick_and_freezes_the_unit() {
    let mut w = open_world();
    let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let target = (400, 320);
    w.issue(w.units[marine].id, Order::Move { pos: target })
        .unwrap();
    w.units[marine].status.stasis_ticks = 2;
    let start = (w.units[marine].x, w.units[marine].y);
    for _ in 0..7 {
        w.step();
    }
    assert_eq!(w.units[marine].status.stasis_ticks, 2);
    assert_eq!((w.units[marine].x, w.units[marine].y), start);
    w.step();
    assert_eq!(w.units[marine].status.stasis_ticks, 1);
    for _ in 0..8 {
        w.step();
    }
    assert_eq!(w.units[marine].status.stasis_ticks, 0);
    let frozen = (w.units[marine].x, w.units[marine].y);
    for _ in 0..8 {
        w.step();
    }
    assert_ne!((w.units[marine].x, w.units[marine].y), frozen);
}

#[test]
fn irradiate_deals_weapon_34_damage_every_eight_ticks() {
    let mut w = open_world();
    let marine = w.spawn(1, uid::MARINE, 320, 320).unwrap();
    let before = w.units[marine].hp;
    w.units[marine].status.irradiate_ticks = 1;
    w.step();
    assert_eq!(w.units[marine].hp, before);
    for _ in 0..7 {
        w.step();
    }
    // 250 * 256 / 75 = 853 in 1/256 points.
    assert_eq!(w.units[marine].hp, before - 853);
    assert_eq!(w.units[marine].status.irradiate_ticks, 0);
    let after = w.units[marine].hp;
    for _ in 0..8 {
        w.step();
    }
    assert_eq!(w.units[marine].hp, after, "expired Irradiate stops");
}

#[test]
fn irradiate_mechanical_source_splashes_only_biological_units_in_32_px() {
    let mut w = open_world();
    let source = w.spawn(0, uid::VULTURE, 320, 320).unwrap();
    let near = w.spawn(0, uid::MARINE, 340, 320).unwrap();
    let burrowed = w.spawn(0, uid::MARINE, 330, 320).unwrap();
    w.units[burrowed].status.burrowed = true;
    let building = w.spawn(0, uid::HATCHERY, 340, 340).unwrap();
    w.units[building].completed = true;
    let far = w.spawn(0, uid::MARINE, 500, 320).unwrap();
    w.units[source].status.irradiate_ticks = 1;
    let (source_hp, near_hp, burrowed_hp, building_hp, far_hp) = (
        w.units[source].hp,
        w.units[near].hp,
        w.units[burrowed].hp,
        w.units[building].hp,
        w.units[far].hp,
    );
    for _ in 0..8 {
        w.step();
    }
    assert_eq!(w.units[source].hp, source_hp, "mechanical source is immune");
    assert_eq!(w.units[near].hp, near_hp - 853);
    assert_eq!(
        w.units[burrowed].hp, burrowed_hp,
        "splash skips burrowed units"
    );
    assert_eq!(w.units[building].hp, building_hp);
    assert_eq!(w.units[far].hp, far_hp);
}

#[test]
fn state_hash_covers_future_deciding_unit_fields() {
    use crate::replay::state_hash;
    let data = Arc::new(GameData::bundled().unwrap());
    let base = || {
        let mut w = World::new(data.clone(), Terrain::open(8, 8), 2);
        w.spawn(0, uid::MARINE, 100, 100).unwrap();
        w
    };
    let h0 = state_hash(&base());
    let mut w = base();
    w.units[0].energy_frac = 1;
    assert_ne!(h0, state_hash(&w), "energy fraction");
    let mut w = base();
    w.units[0].larva_timer = 1;
    assert_ne!(h0, state_hash(&w), "larva timer");
    let mut w = base();
    w.units[0].expires_at = 100;
    assert_ne!(h0, state_hash(&w), "temporary-unit expiry");
    let mut w = base();
    w.units[0].order_carry = 3;
    assert_ne!(h0, state_hash(&w), "order carry");
    let mut w = base();
    w.units[0].path.push_back((7, 9));
    assert_ne!(h0, state_hash(&w), "path queue");
}

#[test]
fn transports_reject_enemy_and_unfinished_carriers() {
    let mut w = open_world();
    let dropship = w.spawn(0, uid::DROPSHIP, 320, 320).unwrap();
    let dropship_id = w.units[dropship].id;
    let enemy = w.spawn(1, uid::MARINE, 330, 320).unwrap();
    let enemy_id = w.units[enemy].id;
    w.issue(
        enemy_id,
        Order::EnterTransport {
            transport: dropship_id,
        },
    )
    .unwrap();
    for _ in 0..12 {
        w.step();
    }
    assert!(w.units[enemy].alive);
    assert_eq!(w.units[enemy].in_transport, None);

    let marine = w.spawn(0, uid::MARINE, 330, 320).unwrap();
    let marine_id = w.units[marine].id;
    w.units[dropship].completed = false;
    w.issue(
        marine_id,
        Order::EnterTransport {
            transport: dropship_id,
        },
    )
    .unwrap();
    for _ in 0..12 {
        w.step();
    }
    assert_eq!(w.units[marine].in_transport, None);
    w.units[dropship].completed = true;
    w.issue(
        marine_id,
        Order::EnterTransport {
            transport: dropship_id,
        },
    )
    .unwrap();
    for _ in 0..12 {
        w.step();
    }
    assert_eq!(w.units[marine].in_transport, Some(dropship_id));
}

#[test]
fn temporary_units_expire_on_the_recovered_lifetime_timer() {
    let mut w = open_world();
    let id = w
        .spawn_temporary(0, uid::DISRUPTION_WEB, (320, 320), 8)
        .unwrap();
    for _ in 0..7 {
        w.step();
    }
    assert!(w.units[id].alive);
    w.step();
    assert!(!w.units[id].alive, "CUnit+0x110 lifetime reached zero");
}

#[test]
fn plague_deals_weapon_60_damage_and_stops_before_killing() {
    let mut w = open_world();
    let marine = w.spawn(1, uid::MARINE, 320, 320).unwrap();
    w.units[marine].hp = 1500;
    w.units[marine].status.plague_ticks = 10;
    for _ in 0..8 {
        w.step();
    }
    // 300 * 256 / 0x4c = 1010 in 1/256 points.
    assert_eq!(w.units[marine].hp, 1500 - 1010);
    for _ in 0..8 {
        w.step();
    }
    assert_eq!(
        w.units[marine].hp, 490,
        "the next 1010-point tick would reach 0, so it is skipped"
    );
    assert_eq!(w.units[marine].status.plague_ticks, 8);
}

#[test]
fn spell_handlers_set_the_recovered_status_durations() {
    let mut w = open_world();
    let caster = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let target = w.spawn(1, uid::MARINE, 360, 320).unwrap();
    let point = (w.units[target].x / fp(1), w.units[target].y / fp(1));
    let id = w.units[target].id;
    let idx = w.index_of(id).unwrap();
    for (spell, field) in [
        (Spell::Ensnare, 0),
        (Spell::Plague, 1),
        (Spell::StasisField, 2),
        (Spell::Maelstrom, 3),
        (Spell::Irradiate, 4),
    ] {
        let target = match spell {
            Spell::Irradiate => SpellTarget::Unit(id),
            _ => SpellTarget::Point(point.0, point.1),
        };
        w.apply_spell(caster, spell, target);
        if spell == Spell::Irradiate {
            // It lands with its bullet, a few frames after the cast.
            assert_eq!(w.units[idx].status.irradiate_ticks, 0);
            for _ in 0..10 {
                w.step();
                if w.units[idx].status.irradiate_ticks > 0 {
                    break;
                }
            }
        }
        let s = &w.units[idx].status;
        let value = match field {
            0 => s.ensnare_ticks,
            1 => s.plague_ticks,
            2 => s.stasis_ticks,
            3 => s.maelstrom_ticks,
            _ => s.irradiate_ticks,
        };
        let expected = match field {
            0 => ENSNARE_UPDATES,
            1 => PLAGUE_UPDATES,
            2 => STASIS_UPDATES,
            3 => MAELSTROM_UPDATES,
            _ => IRRADIATE_UPDATES,
        };
        assert_eq!(value, expected, "{spell:?}");
    }

    let vulture = w.spawn(1, uid::VULTURE, 400, 320).unwrap();
    let vulture_id = w.units[vulture].id;
    w.apply_spell(caster, Spell::Lockdown, SpellTarget::Unit(vulture_id));
    // The Lockdown missile has to reach the Vulture first.
    assert_eq!(w.units[vulture].status.lockdown_ticks, 0);
    let mut landed = None;
    for _ in 0..60 {
        w.step();
        if w.units[vulture].status.lockdown_ticks > 0 {
            landed = Some(w.units[vulture].status.lockdown_ticks);
            break;
        }
    }
    assert_eq!(landed, Some(LOCKDOWN_UPDATES));
}

#[test]
fn fog_of_war_hides_distant_units_from_targeting() {
    let mut w = open_world();
    w.enable_fog();
    let a = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let b = w.spawn(1, uid::MARINE, 600, 600).unwrap();
    w.step();
    assert!(w.is_visible_to(0, w.units[a].x / 256, w.units[a].y / 256));
    assert!(!w.is_visible_to(0, w.units[b].x / 256, w.units[b].y / 256));
    assert!(w.acquire_target(a).is_none());
    let _ = b;
}

#[test]
fn spider_mines_arm_and_detonate() {
    let mut w = open_world();
    let vulture = w.spawn(0, uid::VULTURE, 10, 10).unwrap();
    w.players[0].techs[tech_id::SPIDER_MINES as usize] = true;
    w.units[vulture].energy = 200;
    w.cast(
        w.units[vulture].id,
        Spell::SpiderMines,
        SpellTarget::Point(14, 10),
    )
    .unwrap();
    assert!(
        w.units
            .iter()
            .filter(|u| u.type_id == uid::SPIDER_MINE)
            .count()
            == 1,
        "one mine per order (the Vulture carries three)"
    );
    // Let the mines arm, then walk an enemy onto them.
    for _ in 0..SPIDER_MINE_ARM_TICKS as usize + 2 {
        w.step();
    }
    let enemy = w.spawn(1, uid::MARINE, 14, 10).unwrap();
    let start = w.units[enemy].hp;
    for _ in 0..20 {
        w.step();
    }
    assert!(w.units[enemy].hp < start, "spider mine should detonate");
}

#[test]
fn scv_repairs_a_mechanical_unit() {
    let mut w = open_world();
    let scv = w.spawn(0, uid::SCV, 320, 320).unwrap();
    let tank = w.spawn(0, uid::SIEGE_TANK, 340, 320).unwrap();
    w.units[tank].hp = fp(50);
    w.players[0].minerals = 100;
    w.issue(
        w.units[scv].id,
        Order::Repair {
            target: w.units[tank].id,
        },
    )
    .unwrap();
    for _ in 0..30 {
        w.step();
    }
    assert!(w.units[tank].hp > fp(50));
}

#[test]
fn medics_automatically_heal_nearby_organic_units() {
    let mut w = open_world();
    let medic = w.spawn(0, uid::MEDIC, 320, 320).unwrap();
    let marine = w.spawn(0, uid::MARINE, 340, 320).unwrap();
    w.units[marine].hp = fp(10);
    w.units[medic].energy = 200;
    for _ in 0..20 {
        w.step();
    }
    assert!(w.units[marine].hp > fp(10));
    let _ = medic;
}

#[test]
fn point_target_spells_hit_the_ground_location() {
    let mut w = open_world();
    let templar = w.spawn(0, uid::HIGH_TEMPLAR, 320, 320).unwrap();
    w.players[0].techs[tech_id::PSIONIC_STORM as usize] = true;
    w.units[templar].energy = 200;
    let a = w.spawn(1, uid::DROPSHIP, 480, 320).unwrap();
    let b = w.spawn(1, uid::DROPSHIP, 528, 320).unwrap();
    // Psionic Storm reaches 48 px from the victim's box (OpenBW
    // `bullet_deal_splash_damage`: `unit_distance_to`).  Dropship box:
    // 24 px up from its centre, so 73 px below the storm is just out.
    let c = w.spawn(1, uid::DROPSHIP, 480, 320 + 48 + 25).unwrap();
    for i in [a, b, c] {
        w.units[i].hp = fp(500);
    }
    let (ha, hb, hc) = (w.units[a].hp, w.units[b].hp, w.units[c].hp);
    w.cast(
        w.units[templar].id,
        Spell::PsionicStorm,
        SpellTarget::Point(480, 320),
    )
    .unwrap();
    assert_eq!(w.units[a].hp, ha, "the cast tick does not deal damage");
    let bullet = w.projectiles.last().expect("persistent storm field");
    assert_eq!(bullet.weapon, wid::PSIONIC_STORM);
    assert_eq!(bullet.state, BulletState::Persist);
    for _ in 0..60 {
        w.step();
    }
    assert_eq!(w.units[a].hp, ha - fp(14) * 8);
    assert_eq!(w.units[b].hp, hb - fp(14) * 8, "48 px is inside");
    assert_eq!(w.units[c].hp, hc, "49 px from its box is outside");
    assert!(w.projectiles.is_empty());
}

#[test]
fn order_queue_uses_the_recovered_queueable_metadata() {
    let mut w = open_world();
    let idx = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let id = w.units[idx].id;
    w.issue(id, Order::Move { pos: (400, 400) }).unwrap();
    for i in 0..MAX_ORDER_QUEUE {
        w.queue_order(
            id,
            Order::Move {
                pos: (400 + i as i32, 400),
            },
        )
        .unwrap();
    }
    assert_eq!(
        w.units[w.index_of(id).unwrap()].queue.len(),
        MAX_ORDER_QUEUE
    );
    assert_eq!(
        w.queue_order(id, Order::Move { pos: (1, 1) }),
        Err(SimError::QueueFull)
    );
    // Build is not queueable in the shipped table; it replaces the
    // current order and clears the queue.
    w.queue_order(
        id,
        Order::Build {
            unit_type: uid::BARRACKS,
            pos: (400, 400),
        },
    )
    .unwrap();
    let u = &w.units[w.index_of(id).unwrap()];
    assert!(matches!(u.order, Order::Build { .. }));
    assert!(u.queue.is_empty());
}

#[test]
fn order_done_uses_the_batch_06_queue_pop_and_refreshes_the_row() {
    let mut w = open_world();
    let idx = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let id = w.units[idx].id;
    w.issue(id, Order::Move { pos: (400, 400) }).unwrap();
    w.queue_order(id, Order::Move { pos: (500, 500) }).unwrap();
    w.order_done(idx);
    let queued = Order::Move { pos: (500, 500) };
    let u = &w.units[idx];
    assert_eq!(u.order, queued);
    assert_eq!(u.order_row, queued.order_id());
    assert!(u.queue.is_empty());
}

#[test]
fn status_modifier_count_drives_the_speed_clamp() {
    let mut w = open_world();
    let idx = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert_eq!(w.units[idx].a04_status_modifier_count(), 0);
    w.units[idx].set_flag_29();
    let mods = w.units[idx].a04_status_modifier_count();
    assert_eq!(mods, 1);
    assert_eq!(
        World::modified_speed_for_mods(uid::MARINE, 1024, mods),
        1536
    );
}

#[test]
fn queue_order_uses_can_be_queued_not_highlight() {
    let mut w = open_world();
    let idx = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let id = w.units[idx].id;
    // Row 115 Burrow has highlight 240 but `can_be_queued = false`; it
    // must replace the current order and clear the queue.
    w.issue(id, Order::Move { pos: (400, 400) }).unwrap();
    assert!(w.data.order(115).is_some_and(|r| !r.can_be_queued));
    assert!(w.data.order(115).is_some_and(|r| r.highlight != -1));
    w.queue_order(id, Order::Burrow).unwrap();
    let u = &w.units[w.index_of(id).unwrap()];
    assert!(matches!(u.order, Order::Burrow));
    assert!(u.queue.is_empty());
    // Row 21 Turret Attack is the opposite: queueable but highlight -1.
    assert!(
        w.data
            .order(21)
            .is_some_and(|r| r.can_be_queued && r.highlight == -1)
    );
}

#[test]
fn carrier_attack_waits_for_the_60_tick_launch_phase() {
    let mut w = open_world();
    let carrier = w.spawn(0, uid::CARRIER, 320, 320).unwrap();
    w.players[0].minerals = 1000;
    w.build_hangar_unit(w.units[carrier].id).unwrap();
    let enemy = w.spawn(1, uid::MARINE, 480, 320).unwrap();
    let start = w.units[enemy].hp;
    w.issue_order_id(w.units[carrier].id, 20, Some(w.units[enemy].id), None)
        .unwrap();
    for _ in 0..59 {
        w.step();
    }
    assert_eq!(w.units[enemy].hp, start, "launch phase is still running");
    for _ in 0..15 {
        w.step();
    }
    assert!(
        w.units[enemy].hp < start,
        "the interceptor launches after CUnit+0x55 reaches zero"
    );
}

#[test]
fn reaver_scarab_damages_and_consumes_one_hangar() {
    let mut w = open_world();
    let reaver = w.spawn(0, uid::REAVER, 320, 320).unwrap();
    w.units[reaver].hangar = 1;
    let enemy = w.spawn(1, uid::BARRACKS, 480, 320).unwrap();
    w.units[enemy].completed = true;
    let start = w.units[enemy].hp;
    w.issue(
        w.units[reaver].id,
        Order::AttackUnit {
            target: w.units[enemy].id,
        },
    )
    .unwrap();
    for _ in 0..300 {
        w.step();
        if w.units[enemy].hp < start {
            break;
        }
    }
    assert!(
        w.units[enemy].hp < start,
        "the scarab projectile must damage the target (projectiles={}, reaver hangar={})",
        w.projectiles.len(),
        w.units[reaver].hangar
    );
    assert_eq!(w.units[reaver].hangar, 0);
}

#[test]
fn carrier_launches_interceptors_from_its_hangar() {
    let mut w = open_world();
    let carrier = w.spawn(0, uid::CARRIER, 320, 320).unwrap();
    w.players[0].minerals = 1000;
    w.build_hangar_unit(w.units[carrier].id).unwrap();
    assert_eq!(w.units[carrier].hangar, 1);
    let enemy = w.spawn(1, uid::MARINE, 480, 320).unwrap();
    let start = w.units[enemy].hp;
    for _ in 0..120 {
        w.step();
        if w.units[enemy].hp < start {
            break;
        }
    }
    assert!(
        w.units[enemy].hp < start,
        "interceptor should damage the target"
    );
}

#[test]
fn nuclear_strike_uses_the_nuclear_damage_floor() {
    let mut w = open_world();
    let ghost = w.spawn(0, uid::GHOST, 320, 320).unwrap();
    let silo = w.spawn(0, uid::NUCLEAR_SILO, 128, 128).unwrap();
    w.units[silo].addon_attached = true;
    let target = w.spawn(1, uid::MARINE, 700, 320).unwrap();
    w.players[0].minerals = 1000;
    w.players[0].gas = 1000;
    w.arm_nuke(w.units[silo].id).unwrap();
    let start = w.units[target].hp;
    w.launch_nuke(w.units[ghost].id, (700, 320)).unwrap();
    // Walk into range, 90 frames of countdown, 250 off the map, the
    // fall.
    for _ in 0..700 {
        w.step();
        if w.units[target].hp < start {
            break;
        }
    }
    assert!(
        w.units[target].hp < start,
        "nuke should hit the target area"
    );
}

#[test]
fn nuke_waits_90_ticks_warns_at_45_and_consumes_the_silo() {
    let mut w = open_world();
    // Within the Ghost's sight (9 tiles) of the target point.
    let ghost = w.spawn(0, uid::GHOST, 450, 320).unwrap();
    let silo = w.spawn(0, uid::NUCLEAR_SILO, 128, 128).unwrap();
    w.units[silo].addon_attached = true;
    let target = w.spawn(1, uid::MARINE, 700, 320).unwrap();
    w.players[0].minerals = 1000;
    w.players[0].gas = 1000;
    w.arm_nuke(w.units[silo].id).unwrap();
    assert_eq!((w.players[0].minerals, w.players[0].gas), (800, 800));
    assert!(w.units[silo].nuke_ready);
    let hp = w.units[target].hp;
    w.launch_nuke(w.units[ghost].id, (700, 320)).unwrap();
    assert!(!w.units[silo].nuke_ready);
    assert_eq!(w.nuke_strikes.len(), 1);
    assert!(w.nuke_strikes[0].dot);
    assert!(w.projectiles.is_empty());
    for _ in 0..44 {
        w.step();
    }
    assert!(w.nuke_warnings.is_empty(), "warning is at 45 ticks left");
    w.step();
    assert_eq!(w.nuke_warnings, vec![(700, 320)]);
    for _ in 0..45 {
        w.step();
    }
    assert!(
        w.nuke_strikes.iter().all(|s| s.timer <= 0),
        "countdown over; the strike stays until the bomb lands"
    );
    // The missile is off the map for its WarpIn animation (250
    // frames) before the bomb appears above the target.
    assert!(w.projectiles.is_empty(), "nothing falls yet");
    for _ in 0..NUKE_WARP_FRAMES {
        w.step();
    }
    assert_eq!(w.projectiles.len(), 1, "the bomb is falling");
    assert_eq!(w.units[target].hp, hp, "impact is still travelling");
}

#[test]
fn nuke_aborts_on_ghost_death_and_on_a_new_order() {
    let mut w = open_world();
    let ghost = w.spawn(0, uid::GHOST, 320, 320).unwrap();
    let silo = w.spawn(0, uid::NUCLEAR_SILO, 128, 128).unwrap();
    w.units[silo].addon_attached = true;
    let target = w.spawn(1, uid::MARINE, 700, 320).unwrap();
    w.players[0].minerals = 2000;
    w.players[0].gas = 2000;
    let ghost_id = w.units[ghost].id;
    w.arm_nuke(w.units[silo].id).unwrap();
    w.launch_nuke(ghost_id, (700, 320)).unwrap();
    for _ in 0..10 {
        w.step();
    }
    w.damage_unit(ghost_id, 1_000_000).unwrap();
    assert!(w.nuke_strikes.is_empty(), "ghost death cancels the strike");
    let hp = w.units[target].hp;
    for _ in 0..200 {
        w.step();
    }
    assert_eq!(w.units[target].hp, hp, "canceled strike deals no damage");

    let ghost2 = w.spawn(0, uid::GHOST, 320, 320).unwrap();
    let ghost2_id = w.units[ghost2].id;
    w.arm_nuke(w.units[silo].id).unwrap();
    w.launch_nuke(ghost2_id, (700, 320)).unwrap();
    assert_eq!(w.nuke_strikes.len(), 1);
    w.issue(ghost2_id, Order::Move { pos: (400, 400) }).unwrap();
    assert!(w.nuke_strikes.is_empty(), "a new order cancels the strike");
}

#[test]
fn spawn_larva_adds_one_and_stops_at_three() {
    let mut w = open_world();
    let hatch = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
    let id = w.units[hatch].id;
    let larvae = |w: &World| {
        w.units
            .iter()
            .filter(|u| u.alive && u.owner == 0 && u.type_id == uid::LARVA)
            .count()
    };
    w.spawn_larva(id).unwrap();
    assert_eq!(larvae(&w), 1);
    w.spawn_larva(id).unwrap();
    w.spawn_larva(id).unwrap();
    assert_eq!(larvae(&w), 3);
    assert_eq!(w.spawn_larva(id), Err(SimError::QueueFull));
}

#[test]
fn larva_spawn_uses_the_lowest_side_score() {
    let mut w = open_world();
    let hatch = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
    let id = w.units[hatch].id;
    w.mature_creep();
    let (hx, hy) = (w.units[hatch].x / fp(1), w.units[hatch].y / fp(1));
    let (left, _, _, bottom) = w.data.unit(uid::HATCHERY).dimensions.rect_at((hx, hy));

    w.units[hatch].larva_spawn_side_values = [40, 40, 40, 7];
    w.spawn_larva(id).unwrap();
    let larva = w
        .units
        .iter()
        .find(|u| u.type_id == uid::LARVA)
        .expect("first larva");
    assert_eq!((larva.x / fp(1), larva.y / fp(1)), (hx, bottom + 10));

    w.units[hatch].larva_spawn_side_values = [1, 40, 40, 40];
    w.spawn_larva(id).unwrap();
    let larva = w
        .units
        .iter()
        .rfind(|u| u.type_id == uid::LARVA)
        .expect("second larva");
    assert_eq!((larva.x / fp(1), larva.y / fp(1)), (left - 10, hy));
}

#[test]
fn killing_a_unit_clears_every_reference_to_it() {
    let mut w = open_world();
    let attacker = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let victim = w.spawn(1, uid::MARINE, 360, 320).unwrap();
    let (a_id, v_id) = (w.units[attacker].id, w.units[victim].id);
    w.issue(a_id, Order::AttackUnit { target: v_id }).unwrap();
    w.step();
    assert_eq!(w.units[attacker].target, Some(v_id));
    w.damage_unit(v_id, 1_000_000).unwrap();
    assert_eq!(w.units[attacker].target, None);
}

#[test]
fn larva_cap_is_per_connected_hatchery() {
    let mut w = open_world();
    let a = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
    let b = w.spawn(0, uid::HATCHERY, 400, 320).unwrap();
    let (aid, bid) = (w.units[a].id, w.units[b].id);
    for k in 0..3 {
        let l = w.spawn(0, uid::LARVA, 320 + k, 320).unwrap();
        w.units[l].connected_unit = Some(aid);
    }
    assert_eq!(w.spawn_larva(aid), Err(SimError::QueueFull));
    w.spawn_larva(bid).unwrap();
    assert_eq!(
        w.units
            .iter()
            .filter(|u| u.alive && u.type_id == uid::LARVA && u.connected_unit == Some(bid))
            .count(),
        1
    );
}

#[test]
fn hatchery_larva_interval_is_37_status_updates() {
    let mut w = open_world();
    let hatch = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
    w.units[hatch].completed = true;
    w.units[hatch].larva_timer = LARVA_SPAWN_TICKS;
    let larvae = |w: &World| {
        w.units
            .iter()
            .filter(|u| u.alive && u.owner == 0 && u.type_id == uid::LARVA)
            .count()
    };
    for _ in 0..LARVA_SPAWN_TICKS - 1 {
        w.step();
    }
    assert_eq!(larvae(&w), 0);
    w.step();
    assert_eq!(larvae(&w), 1);
    assert_eq!(w.units[hatch].larva_timer, LARVA_SPAWN_TICKS);
}

#[test]
fn cancel_morph_returns_an_egg_to_its_larva() {
    let mut w = open_world();
    let hatch = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
    w.units[hatch].completed = true;
    let larva = w.spawn(0, uid::LARVA, 320, 320).unwrap();
    let id = w.units[larva].id;
    w.spawn(0, uid::SPAWNING_POOL, 512, 512).unwrap();
    w.players[0].minerals = 1000;
    w.morph(id, uid::ZERGLING).unwrap();
    assert_eq!(w.units[larva].type_id, uid::EGG);
    w.cancel_morph(id).unwrap();
    assert_eq!(w.units[larva].type_id, uid::LARVA);
    assert!(w.units[larva].alive);
}

#[test]
fn cancel_addon_refunds_and_unlinks_the_addon() {
    let mut w = open_world();
    let cc = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
    w.units[cc].completed = true;
    w.spawn(0, uid::ACADEMY, 300, 320).unwrap();
    w.players[0].minerals = 1000;
    w.players[0].gas = 1000;
    w.build_addon(w.units[cc].id, uid::COMSAT_STATION).unwrap();
    let before = w.players[0].minerals;
    w.cancel_addon(w.units[cc].id).unwrap();
    assert!(w.units[cc].addon.is_none());
    assert!(w.players[0].minerals > before, "75 % refund");
}

#[test]
fn addons_require_academy_and_covert_ops() {
    let mut w = open_world();
    w.players[0].minerals = 2000;
    w.players[0].gas = 2000;
    let cc = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
    let cc_id = w.units[cc].id;
    let before = w.players[0].minerals;
    assert_eq!(
        w.build_addon(cc_id, uid::COMSAT_STATION),
        Err(SimError::MissingRequirements)
    );
    assert_eq!(w.players[0].minerals, before);
    w.spawn(0, uid::ACADEMY, 300, 320).unwrap();
    w.build_addon(cc_id, uid::COMSAT_STATION).unwrap();
}

#[test]
fn cancel_research_and_upgrade_refunds_the_full_price() {
    let mut w = open_world();
    let academy = w.spawn(0, uid::ACADEMY, 8, 8).unwrap();
    let id = w.units[academy].id;
    w.players[0].minerals = 1000;
    w.players[0].gas = 1000;

    let tech = tech_id::STIM_PACKS;
    let t = w.data.tech(tech).unwrap().clone();
    let before = (w.players[0].minerals, w.players[0].gas);
    w.research(id, tech).unwrap();
    w.step();
    assert_eq!(
        (w.players[0].minerals, w.players[0].gas),
        (
            before.0 - t.mineral_cost as i32,
            before.1 - t.gas_cost as i32
        )
    );
    w.cancel_production(id).unwrap();
    assert_eq!((w.players[0].minerals, w.players[0].gas), before);
    assert_eq!(w.units[academy].build_remaining, 0);
    assert_eq!(w.units[academy].build_total, 0);
    assert!(!w.players[0].has_tech(tech));

    let up_id = upgrade::U238_SHELLS;
    let up = w.data.upgrade(up_id).unwrap().clone();
    let before = (w.players[0].minerals, w.players[0].gas);
    w.upgrade(id, up_id).unwrap();
    w.step();
    assert_eq!(
        (w.players[0].minerals, w.players[0].gas),
        (
            before.0 - up.mineral_cost(0) as i32,
            before.1 - up.gas_cost(0) as i32
        )
    );
    w.cancel_production(id).unwrap();
    assert_eq!((w.players[0].minerals, w.players[0].gas), before);
    assert_eq!(w.units[academy].build_remaining, 0);
    assert_eq!(w.units[academy].build_total, 0);
    assert_eq!(w.players[0].upgrade_level(up_id), 0);
}

#[test]
fn capture_the_flag_pickup_and_capture_eliminate_the_owner() {
    let mut w = open_world();
    let cc0 = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
    let cc1 = w.spawn(1, uid::COMMAND_CENTER, 1200, 320).unwrap();
    w.units[cc0].completed = true;
    w.units[cc1].completed = true;
    w.start_ctf();
    assert!(w.ctf.is_some());
    // Force the flags out without simulating three game minutes.
    {
        let c = w.ctf.as_mut().unwrap();
        c.release_at = 0;
        for f in &mut c.flags {
            f.released = true;
        }
    }
    let enemy_flag = w.ctf.as_ref().unwrap().flags[1].pos;
    let worker = w.spawn_px(0, uid::SCV, enemy_flag.0, enemy_flag.1).unwrap();
    w.update_ctf();
    assert_eq!(
        w.ctf.as_ref().unwrap().flags[1].carrier,
        Some(w.units[worker].id)
    );
    // Carry it to our own beacon: the enemy is eliminated.
    let home = w.ctf.as_ref().unwrap().flags[0].home;
    w.units[worker].x = home.0 * fp(1);
    w.units[worker].y = home.1 * fp(1);
    w.update_ctf();
    assert!(w.players[1].defeated);
}

#[test]
fn a_beacon_can_move_once_before_the_deadline() {
    let mut w = open_world();
    let cc = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
    w.units[cc].completed = true;
    w.start_ctf();
    w.move_beacon(0, (700, 700)).unwrap();
    assert_eq!(w.ctf.as_ref().unwrap().flags[0].home, (700, 700));
    assert_eq!(w.move_beacon(0, (800, 800)), Err(SimError::Unsupported));
}

#[test]
fn cheat_codes_apply_and_are_recorded() {
    let mut w = open_world();
    w.recorder = Some(crate::replay::Recorder::new());
    let before = w.players[0].minerals;
    w.apply_cheat(0, cheat::SHOW_ME_THE_MONEY).unwrap();
    assert_eq!(w.players[0].minerals, before + 10_000);
    assert!(matches!(
        w.recorder.as_ref().unwrap().cmds.last().map(|c| &c.act),
        Some(crate::replay::RecAct::Cheat { .. })
    ));
    w.apply_cheat(0, cheat::POWER_OVERWHELMING).unwrap();
    let marine = w.spawn(0, uid::MARINE, 100, 100).unwrap();
    let hp = w.units[marine].hp;
    w.damage_hp(marine, 1000, 0);
    assert_eq!(w.units[marine].hp, hp);
    w.apply_cheat(0, cheat::MEDIEVAL_MAN).unwrap();
    assert!(w.players[0].has_tech(tech_id::STIM_PACKS));
    assert!(w.players[0].cheats.free_upgrades);
    w.apply_cheat(0, cheat::FOOD_FOR_THOUGHT).unwrap();
    assert_eq!(w.supply_cap(0), 400);
    w.apply_cheat(0, cheat::THERE_IS_NO_COW_LEVEL).unwrap();
    assert_eq!(w.winner, Some(0));
}

#[test]
fn sudden_death_ends_the_owner_of_a_destroyed_town_hall() {
    let mut w = open_world();
    let cc = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
    let nexus = w.spawn(1, uid::NEXUS, 800, 800).unwrap();
    w.units[cc].completed = true;
    w.units[nexus].completed = true;
    w.sudden_death = true;
    let id = w.units[cc].id;
    w.units[cc].hp = 0;
    w.kill_unit(w.index_of(id).unwrap());
    w.check_victory();
    assert!(w.players[0].defeated);
    assert_eq!(w.winner, Some(1));
}

#[test]
fn greed_wins_at_the_gathered_resource_goal() {
    let mut w = open_world();
    w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
    w.spawn(1, uid::NEXUS, 800, 800).unwrap();
    w.greed_goal = Some(1000);
    w.players[0].minerals_gathered = 1000;
    w.players[0].gas_gathered = 999;
    w.check_victory();
    assert_eq!(w.winner, None, "both resources are required");
    w.players[0].gas_gathered = 1000;
    w.check_victory();
    assert_eq!(w.winner, Some(0));
}

#[test]
fn slaughter_wins_by_kills_at_the_deadline() {
    let mut w = open_world();
    w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
    w.spawn(1, uid::NEXUS, 800, 800).unwrap();
    w.players[0].kills = 3;
    w.players[1].kills = 7;
    w.players[1].razings = 2;
    w.slaughter_deadline = Some(10);
    w.tick = 10;
    w.check_victory();
    assert_eq!(w.winner, Some(1));
}

#[test]
fn direct_abilities_enter_the_command_log() {
    let mut w = open_world();
    w.recorder = Some(crate::replay::Recorder::new());
    let hatch = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
    let id = w.units[hatch].id;
    w.spawn_larva(id).unwrap();
    let cloak = w.spawn(0, uid::GHOST, 380, 380).unwrap();
    let cid = w.units[cloak].id;
    w.players[0].techs[crate::ids::tech::PERSONNEL_CLOAKING as usize] = true;
    w.set_cloak(cid, true).unwrap();
    let acts: Vec<_> = w
        .recorder
        .as_ref()
        .unwrap()
        .cmds
        .iter()
        .map(|c| c.act.clone())
        .collect();
    assert!(matches!(
        acts.first(),
        Some(crate::replay::RecAct::SpawnLarva { .. })
    ));
    assert!(matches!(
        acts.last(),
        Some(crate::replay::RecAct::Cloak { cloaked: true, .. })
    ));
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn scenario_rules_override_unit_tech_upgrade_and_availability() {
    use crate::map::{TechSettings, UnitAvailability, UnitSettings, UpgradeSettings};
    let mut w = open_world();

    let mut units = UnitSettings::default();
    units.use_default = vec![1; 228];
    units.use_default[0] = 0;
    units.hit_points = vec![0; 228];
    units.hit_points[0] = 5120; // 20 HP
    units.shields = vec![0; 228];
    units.armor = vec![0; 228];
    units.armor[0] = 2;
    units.build_time = vec![0; 228];
    units.build_time[0] = 300;
    units.mineral_cost = vec![0; 228];
    units.mineral_cost[0] = 25;
    units.gas_cost = vec![0; 228];
    units.string_id = vec![0; 228];
    units.weapon_damage = vec![0; 130];
    units.weapon_bonus = vec![0; 130];
    w.rules.unit_settings = Some(units);

    let mut avail = UnitAvailability::default();
    avail.player = vec![1; 12 * 228];
    avail.use_global = vec![0; 12 * 228];
    avail.global = vec![1; 228];
    avail.player[37] = 0; // player 0 cannot train Zergling
    w.rules.unit_availability = Some(avail);

    let mut techs = TechSettings::default();
    techs.use_default = vec![1; 44];
    techs.use_default[0] = 0;
    techs.mineral_cost = vec![0; 44];
    techs.mineral_cost[0] = 150;
    techs.gas_cost = vec![0; 44];
    techs.research_time = vec![0; 44];
    techs.research_time[0] = 100;
    techs.energy_cost = vec![0; 44];
    w.rules.tech_settings = Some(techs);

    let mut upgrades = UpgradeSettings::default();
    upgrades.use_default = vec![1; 61];
    upgrades.use_default[7] = 0;
    upgrades.mineral_base = vec![0; 61];
    upgrades.mineral_base[7] = 100;
    upgrades.mineral_factor = vec![0; 61];
    upgrades.mineral_factor[7] = 10;
    upgrades.gas_base = vec![0; 61];
    upgrades.gas_factor = vec![0; 61];
    upgrades.time_base = vec![0; 61];
    upgrades.time_base[7] = 4000;
    upgrades.time_factor = vec![0; 61];
    w.rules.upgrade_settings = Some(upgrades);

    let marine = w.spawn(0, uid::MARINE, 10, 10).unwrap();
    assert_eq!(w.units[marine].hp, 5120);
    assert_eq!(w.units[marine].shields, 0);
    assert_eq!(w.unit_stats(uid::MARINE).armor, 2);
    assert_eq!(w.unit_stats(uid::MARINE).mineral_cost, 25);
    assert!(!w.unit_available(0, uid::ZERGLING));
    assert!(w.unit_available(1, uid::ZERGLING));
    assert_eq!(w.tech_stats(0).unwrap().mineral_cost, 150);
    assert_eq!(w.tech_stats(0).unwrap().research_time, 100);
    assert_eq!(w.upgrade_stats(7).unwrap().mineral_cost(0), 100);
    assert_eq!(w.upgrade_stats(7).unwrap().time_cost(0), 4000);
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn property_slot_applies_retail_limits_and_ignores_owner() {
    use crate::map::UnitProperties;
    let mut w = open_world();
    let props = UnitProperties {
        state_flags: 0x14,
        field_flags: 0x3f,
        owner: 1,
        hit_points_percent: 50,
        shields_percent: 25,
        energy_percent: 10,
        resource_amount: 0x1_2345,
        hangar: 2,
        states: 0x14,
    };
    let i = w
        .spawn_with_properties(0, uid::HIGH_TEMPLAR, 10, 10, &props)
        .unwrap();
    let ty = w.data.unit(uid::HIGH_TEMPLAR).clone();
    assert_eq!(w.units[i].hp, (ty.hit_points_raw as i32 * 50 / 100).max(1));
    assert_eq!(w.units[i].shields, fp(ty.shield_points as i32 * 25 / 100));
    assert_eq!(
        w.units[i].energy_fixed(),
        w.max_energy_fixed(uid::HIGH_TEMPLAR, 0) * 10 / 100
    );
    assert_eq!(w.units[i].owner, 0, "owner byte is unused");
    assert_eq!(w.units[i].resource_amount, 0);
    assert!(w.units[i].invincible);

    let mut mineral = UnitProperties::default();
    mineral.field_flags = 0x10;
    mineral.resource_amount = 1234;
    let m = w
        .spawn_with_properties(0, uid::MINERAL_FIELD, 40, 40, &mineral)
        .unwrap();
    assert_eq!(w.units[m].resource_amount, 1234);

    let mut hangar = UnitProperties::default();
    hangar.field_flags = 0x20;
    hangar.hangar = 8;
    let c = w
        .spawn_with_properties(0, uid::CARRIER, 60, 60, &hangar)
        .unwrap();
    assert_eq!(w.units[c].hangar, 4, "clamped to the Carrier's capacity");

    let mut lift = UnitProperties::default();
    lift.state_flags = 0x04;
    lift.states = 0x04;
    let cc = w
        .spawn_with_properties(0, uid::COMMAND_CENTER, 80, 80, &lift)
        .unwrap();
    assert!(w.units[cc].lifted);
}

#[test]
fn burrow_and_unburrow_toggle_the_hidden_state() {
    let mut w = open_world();
    let ling = w.spawn(0, uid::ZERGLING, 320, 320).unwrap();
    w.players[0].techs[tech_id::BURROWING as usize] = true;
    let id = w.units[ling].id;
    w.issue(id, Order::Burrow).unwrap();
    w.step();
    assert!(w.units[ling].status.burrowed);
    w.issue(id, Order::Unburrow).unwrap();
    for _ in 0..10 {
        w.step();
    }
    assert!(!w.units[ling].status.burrowed);
}

#[test]
fn exposed_predicate_rejects_hidden_and_burrowed_units() {
    let mut w = open_world();
    let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert!(w.units[marine].is_exposed());
    w.units[marine].status.burrowed = true;
    assert!(!w.units[marine].is_exposed());
    w.units[marine].status.burrowed = false;
    w.units[marine].in_transport = Some(1);
    assert!(!w.units[marine].is_exposed());
}

#[test]
fn only_production_buildings_accept_a_rally_point() {
    let mut w = open_world();
    let pylon = w.spawn(0, uid::PYLON, 320, 320).unwrap();
    assert_eq!(
        w.set_rally(w.units[pylon].id, (400, 400)),
        Err(SimError::MissingRequirements)
    );
    let barracks = w.spawn(0, uid::BARRACKS, 420, 320).unwrap();
    w.set_rally(w.units[barracks].id, (400, 400)).unwrap();
    assert_eq!(w.units[barracks].rally, Some((400, 400)));
}

#[test]
fn worker_flag_matches_the_recovered_predicate() {
    let w = open_world();
    for (id, expected) in [
        (uid::SCV, true),
        (uid::DRONE, true),
        (uid::PROBE, true),
        (uid::MARINE, false),
        (uid::ZERGLING, false),
    ] {
        assert_eq!(w.data.unit(id).is_worker(), expected, "unit {id}");
    }
}

#[test]
fn arbiter_predicate_matches_the_recovered_rows() {
    assert!(is_arbiter_type(uid::ARBITER));
    assert!(is_arbiter_type(0x56));
    assert!(!is_arbiter_type(uid::CARRIER));
}

#[test]
fn cloak_state_matches_the_recovered_bits() {
    let mut w = open_world();
    let ghost = w.spawn(0, uid::GHOST, 320, 320).unwrap();
    assert_eq!(w.units[ghost].cloak_state(), 0);
    w.units[ghost].status.cloaked = true;
    assert_eq!(w.units[ghost].cloak_state(), 2);
}

#[test]
fn unit_box_rect_uses_each_asymmetric_extent() {
    let w = open_world();
    let ty = w.data.unit(uid::BARRACKS);
    let rect = unit_box_rect(ty, (100, 100));
    assert_eq!(rect.left, 100 - ty.dimensions.left as i32);
    assert_eq!(rect.top, 100 - ty.dimensions.top as i32);
    assert_eq!(rect.right, 100 + ty.dimensions.right as i32);
    assert_eq!(rect.bottom, 100 + ty.dimensions.bottom as i32);
}

#[test]
fn unit_box_width_matches_left_plus_right_plus_one() {
    let w = open_world();
    for id in [uid::MARINE, uid::BARRACKS, uid::COMMAND_CENTER] {
        let ty = w.data.unit(id);
        assert_eq!(
            unit_box_width(ty),
            ty.dimensions.left as i32 + ty.dimensions.right as i32 + 1,
            "unit {id}"
        );
    }
}

#[test]
fn current_build_unit_reads_the_queue_head() {
    let mut w = open_world();
    let barracks = w.spawn(0, uid::BARRACKS, 320, 320).unwrap();
    w.units[barracks].completed = true;
    let cc = w.spawn(0, uid::COMMAND_CENTER, 240, 320).unwrap();
    w.units[cc].completed = true;
    w.players[0].minerals = 500;
    assert_eq!(w.units[barracks].current_build_unit(), uid::BARRACKS);
    w.train(w.units[barracks].id, uid::MARINE).unwrap();
    assert_eq!(w.units[barracks].current_build_unit(), uid::MARINE);
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn unit_name_uses_the_scenario_string_override() {
    let mut w = open_world();
    let mut units = crate::map::UnitSettings::default();
    units.use_default = vec![1; 228];
    units.use_default[0] = 0;
    units.string_id = vec![0; 228];
    units.string_id[0] = 5;
    w.rules.unit_settings = Some(units);
    assert_eq!(w.unit_name(uid::MARINE), w.data.strings.text(5));
    assert_eq!(w.unit_name(uid::ZERGLING), w.data.unit_name(uid::ZERGLING));
}

#[test]
fn doodad_range_matches_the_recovered_predicate() {
    for ty in 0xcb..=0xd5 {
        assert!(is_doodad_type(ty), "type {ty:#x}");
    }
    assert!(!is_doodad_type(0xca));
    assert!(!is_doodad_type(0xd6));
}

#[test]
fn organic_predicate_matches_the_recovered_flag() {
    let w = open_world();
    assert!(w.data.unit(uid::MARINE).is_organic());
    assert!(!w.data.unit(uid::VULTURE).is_organic());
    assert!(!w.data.unit(uid::BARRACKS).is_organic());
}

#[test]
fn robotic_and_hero_flags_match_the_recovered_predicates() {
    let w = open_world();
    assert!(w.data.unit(uid::PROBE).is_robotic());
    assert!(!w.data.unit(uid::MARINE).is_robotic());
    assert!(!w.data.unit(uid::MARINE).is_hero());
    assert!(
        w.data.units.iter().any(|u| u.is_hero()),
        "the table has at least one hero"
    );
}

#[test]
fn air_state_matches_the_dynamic_flag() {
    let mut w = open_world();
    let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let wraith = w.spawn(0, uid::WRAITH, 360, 320).unwrap();
    let cc = w.spawn(0, uid::COMMAND_CENTER, 400, 320).unwrap();
    assert!(!w.is_air_unit(marine));
    assert!(w.is_air_unit(wraith));
    assert!(!w.is_air_unit(cc));
    w.units[cc].lifted = true;
    assert!(w.is_air_unit(cc), "lifted building is dynamically air");
}

#[test]
fn pylon_predicate_matches_type_0x9c() {
    assert!(is_pylon_type(uid::PYLON));
    assert!(!is_pylon_type(uid::NEXUS));
}

#[test]
fn broodlings_expire_after_1350_frames() {
    assert!(is_broodling_or_dying(uid::BROODLING));
    assert!(!is_broodling_or_dying(uid::ZERGLING));
    let mut w = open_world();
    w.spawn_broodling(0, 320, 320);
    let i = w
        .units
        .iter()
        .position(|u| u.type_id == uid::BROODLING)
        .unwrap();
    assert!(w.units[i].alive);
    for _ in 0..BROODLING_LIFETIME_TICKS {
        w.step();
    }
    assert!(!w.units[i].alive);
}

#[test]
fn floor_trap_predicate_matches_0xcb_and_0xd1() {
    assert!(is_floor_trap_type(0xcb));
    assert!(is_floor_trap_type(0xd1));
    assert!(!is_floor_trap_type(0xcc));
    assert!(!is_floor_trap_type(0xd2));
}

#[test]
fn trap_trigger_predicate_excludes_doors_and_0xcc() {
    for ty in [0xcb, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5] {
        assert!(is_trap_trigger_type(ty), "type {ty:#x}");
    }
    assert!(!is_trap_trigger_type(0xcc));
    assert!(!is_trap_trigger_type(0xcd));
    assert!(!is_trap_trigger_type(0xd0));
}

#[test]
fn goliath_turret_predicate_matches_both_rows() {
    assert!(is_goliath_turret_type(uid::GOLIATH_TURRET));
    assert!(is_goliath_turret_type(0x12));
    assert!(!is_goliath_turret_type(uid::GOLIATH));
}

#[test]
fn non_building_predicate_inverts_the_units_dat_flag() {
    let w = open_world();
    assert!(is_non_building_type(&w.data, uid::MARINE));
    assert!(!is_non_building_type(&w.data, uid::BARRACKS));
    assert!(is_non_building_type(&w.data, 0xff));
}

#[test]
fn special_family_predicates_match_the_recovered_rows() {
    for ty in [1, 16, 99, 100, 104] {
        assert!(is_ghost_family_type(ty), "ghost {ty}");
    }
    assert!(!is_ghost_family_type(0));
    assert!(is_hangar_unit_type(0x49));
    assert!(is_hangar_unit_type(0x55));
    assert!(!is_hangar_unit_type(uid::CARRIER));
    assert!(is_firebat_family_type(uid::FIREBAT));
    assert!(is_firebat_family_type(10));
    assert!(!is_firebat_family_type(uid::MARINE));
}

#[test]
fn zerg_flag_predicate_matches_the_units_dat_bit() {
    let w = open_world();
    assert!(w.data.unit(uid::ZERGLING).is_zerg_flag());
    assert!(w.data.unit(uid::DRONE).is_zerg_flag());
    assert!(!w.data.unit(uid::MARINE).is_zerg_flag());
}

#[test]
fn carrier_reaver_family_predicate_matches_all_four_rows() {
    for ty in [0x48, 0x52, 0x53, 0x51] {
        assert!(is_carrier_reaver_family(ty), "type {ty:#x}");
    }
    assert!(!is_carrier_reaver_family(uid::SCOUT));
}

#[test]
fn carrier_defiler_and_medic_predicates_match_their_rows() {
    assert!(is_carrier_type(0x48));
    assert!(is_carrier_type(0x52));
    assert!(!is_carrier_type(0x53));
    assert!(is_defiler_family_type(0x2e));
    assert!(is_defiler_family_type(0x34));
    assert!(!is_defiler_family_type(uid::HYDRALISK));
    assert!(is_medic_type(0x22));
    assert!(!is_medic_type(uid::MARINE));
}

#[test]
fn valkyrie_predicate_matches_row_0x3a() {
    assert!(is_valkyrie_type(0x3a));
    assert!(!is_valkyrie_type(uid::WRAITH));
}

#[test]
fn nydus_canal_predicate_matches_row_0x86() {
    assert!(is_nydus_canal_type(uid::NYDUS_CANAL));
    assert!(!is_nydus_canal_type(uid::HATCHERY));
}

#[test]
fn powerup_flag_predicate_matches_the_units_dat_bit() {
    let w = open_world();
    assert!(w.data.unit(128).is_powerup_flag()); // Uraj Crystal
    assert!(w.data.unit(215).is_powerup_flag()); // Flag
    assert!(w.data.unit(220).is_powerup_flag()); // Mineral Chunk
    assert!(!w.data.unit(uid::MARINE).is_powerup_flag());
}

#[test]
fn detector_flag_predicate_matches_units_dat() {
    let w = open_world();
    assert!(w.data.unit(uid::SCIENCE_VESSEL).is_detector());
    assert!(w.data.unit(uid::OBSERVER).is_detector());
    assert!(!w.data.unit(uid::MARINE).is_detector());
}

#[test]
fn enemy_predicate_uses_the_alliance_table() {
    let mut w = open_world();
    assert!(w.is_enemy(0, 1));
    assert!(!w.is_enemy(0, 0));
    w.players[1].team = w.players[0].team;
    assert!(!w.is_enemy(0, 1));
}

#[test]
fn same_side_predicate_matches_the_alliance_table() {
    let mut w = open_world();
    assert!(w.same_side(0, 0));
    assert!(!w.same_side(0, 1));
    w.players[1].team = w.players[0].team;
    assert!(w.same_side(0, 1));
}

#[test]
fn connected_unit_and_nuke_ready_getters_match_their_fields() {
    let mut w = open_world();
    let hatch = w.spawn(0, uid::HATCHERY, 320, 320).unwrap();
    let larva = w.spawn(0, uid::LARVA, 320, 340).unwrap();
    w.units[larva].connected_unit = Some(w.units[hatch].id);
    assert_eq!(w.units[larva].connected_unit_id(), Some(w.units[hatch].id));
    let silo = w.spawn(0, uid::NUCLEAR_SILO, 400, 320).unwrap();
    assert!(!w.units[silo].nuke_ready_flag());
    w.units[silo].nuke_ready = true;
    assert!(w.units[silo].nuke_ready_flag());
}

#[test]
fn pending_tech_and_upgrade_getters_read_the_active_order() {
    let mut w = open_world();
    let building = w.spawn(0, uid::ACADEMY, 320, 320).unwrap();
    assert_eq!(w.units[building].pending_tech(), None);
    w.units[building].order = Order::Research {
        tech_id: tech_id::STIM_PACKS,
    };
    assert_eq!(w.units[building].pending_tech(), Some(tech_id::STIM_PACKS));
    w.units[building].order = Order::Upgrade {
        upgrade_id: upgrade::U238_SHELLS,
        level: 1,
    };
    assert_eq!(
        w.units[building].pending_upgrade(),
        Some(upgrade::U238_SHELLS)
    );
}

#[test]
fn acid_spore_getter_matches_the_status_counter() {
    let mut w = open_world();
    let z = w.spawn(0, uid::ZERGLING, 320, 320).unwrap();
    assert_eq!(w.units[z].acid_spore_count(), 0);
    w.units[z].status.acid_spores = 3;
    assert_eq!(w.units[z].acid_spore_count(), 3);
}

#[test]
fn max_hp_lookup_matches_the_units_dat_table() {
    let w = open_world();
    assert_eq!(
        w.unit_max_hp_raw(uid::MARINE),
        w.data.unit(uid::MARINE).hit_points_raw as i32
    );
    assert_eq!(
        w.unit_max_hp_raw(uid::BARRACKS),
        w.data.unit(uid::BARRACKS).hit_points_raw as i32
    );
}

#[test]
fn special_type_and_state_getters_match_their_rows() {
    assert!(is_map_revealer_type(0x65));
    assert!(is_scanner_sweep_type(0x21));
    assert!(is_nuclear_silo_type(0x6c));
    assert!(!is_nuclear_silo_type(uid::COMMAND_CENTER));

    let mut w = open_world();
    let b = w.spawn(0, uid::ACADEMY, 320, 320).unwrap();
    assert!(!w.units[b].has_pending_upgrade());
    w.units[b].order = Order::Upgrade {
        upgrade_id: upgrade::U238_SHELLS,
        level: 1,
    };
    assert!(w.units[b].has_pending_upgrade());
    assert_eq!(w.units[b].lockdown_ticks(), 0);
    w.units[b].status.lockdown_ticks = 7;
    assert_eq!(w.units[b].lockdown_ticks(), 7);
    let hatch = w.spawn(0, uid::HATCHERY, 400, 320).unwrap();
    let hatch_id = w.units[hatch].id;
    w.units[b].set_connected_unit(Some(hatch_id));
    assert_eq!(w.units[b].connected_unit_id(), Some(hatch_id));
}

#[test]
fn target_maelstrom_and_supply_getters_match_their_fields() {
    let mut w = open_world();
    let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert!(!w.units[marine].has_target());
    w.units[marine].target = Some(7);
    assert!(w.units[marine].has_target());
    assert_eq!(w.units[marine].maelstrom_ticks(), 0);
    w.units[marine].status.maelstrom_ticks = 5;
    assert_eq!(w.units[marine].maelstrom_ticks(), 5);
    assert_eq!(
        w.unit_supply_provided(uid::OVERLORD),
        w.data.unit(uid::OVERLORD).supply_provided
    );
}

#[test]
fn unit_box_extent_getters_match_units_dat() {
    let w = open_world();
    for id in [uid::MARINE, uid::BARRACKS, uid::COMMAND_CENTER] {
        let d = w.data.unit(id).dimensions;
        assert_eq!(w.unit_box_left(id), d.left as i32, "left {id}");
        assert_eq!(w.unit_box_right(id), d.right as i32, "right {id}");
        assert_eq!(w.unit_box_top(id), d.top as i32, "top {id}");
    }
}

#[test]
fn box_bottom_height_and_path_extents_follow_units_dat() {
    let w = open_world();
    for id in [uid::MARINE, uid::BARRACKS, uid::COMMAND_CENTER] {
        let d = w.data.unit(id).dimensions;
        assert_eq!(w.unit_box_bottom(id), d.bottom as i32, "bottom {id}");
        assert_eq!(
            w.unit_box_height(id),
            d.top as i32 + 1 + d.bottom as i32,
            "height {id}"
        );
        assert_eq!(
            w.unit_path_extents(id),
            [
                d.top as i32,
                -(d.right as i32),
                -(d.bottom as i32),
                d.left as i32,
                d.top as i32 + 1,
                !(d.right as i32),
                !(d.bottom as i32),
                d.left as i32 + 1,
            ],
            "path extents {id}"
        );
    }
    // Bit 15 selects the all-round radius form.
    assert_eq!(w.unit_path_extents(0x8005), [5, -5, -5, 5, 6, -6, -6, 6]);
}

#[test]
fn target_and_link_accessors_reproduce_the_cunit_setters() {
    let mut w = open_world();
    let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert!(!w.units[marine].has_target());
    w.units[marine].set_target(42);
    assert_eq!(w.units[marine].target, Some(42));
    w.units[marine].set_target(0);
    assert!(!w.units[marine].has_target());
    w.units[marine].set_target(42);
    w.units[marine].clear_target();
    assert!(!w.units[marine].has_target());

    let hatch = w.spawn(0, uid::HATCHERY, 400, 320).unwrap();
    let hatch_id = w.units[hatch].id;
    w.units[marine].set_connected_unit(Some(hatch_id));
    w.units[marine].clear_connected_unit();
    assert_eq!(w.units[marine].connected_unit_id(), None);

    let academy = w.spawn(0, uid::ACADEMY, 480, 320).unwrap();
    w.units[academy].order = Order::Research {
        tech_id: tech_id::STIM_PACKS,
    };
    assert_eq!(w.units[academy].pending_tech(), Some(tech_id::STIM_PACKS));
    w.units[academy].clear_pending_tech();
    assert_eq!(w.units[academy].pending_tech(), None);
    assert!(matches!(w.units[academy].order, Order::Idle));
    // A non-research order is not disturbed.
    w.units[academy].order = Order::Train {
        unit_type: uid::MARINE,
    };
    w.units[academy].clear_pending_tech();
    assert!(matches!(
        w.units[academy].order,
        Order::Train { unit_type } if unit_type == uid::MARINE
    ));

    assert!(is_infested_terran_type(0x32));
    assert!(!is_infested_terran_type(uid::MARINE));
}

#[test]
fn units_dat_flags_and_resource_amount_match_their_rows() {
    let mut w = open_world();
    // 0x005b3a50 / 0x005b57b0 / 0x005b5950 / 0x005b61d0.
    assert!(w.unit_regenerates(uid::ZERGLING));
    assert!(!w.unit_regenerates(uid::MARINE));
    assert!(w.unit_is_addon(107)); // Comsat Station.
    assert!(!w.unit_is_addon(uid::COMMAND_CENTER));
    assert!(w.unit_can_cloak(uid::GHOST));
    assert!(w.unit_can_cloak(uid::WRAITH));
    assert!(!w.unit_can_cloak(uid::MARINE));
    assert!(w.unit_is_resource_container(176)); // Mineral Field.
    assert!(!w.unit_is_resource_container(uid::BARRACKS));

    // 0x005b6010: Queen / Matriarch only.
    assert!(is_queen_type(uid::QUEEN));
    assert!(is_queen_type(0x31));
    assert!(!is_queen_type(uid::DEFILER));

    // 0x005b53a0: +0xd4 exposed only for flag-0x800 powerup rows.
    let chunk = w.spawn(0, 220, 300, 300).unwrap(); // Mineral Chunk.
    w.units[chunk].resource_amount = 1500;
    assert_eq!(w.unit_resource_amount(chunk), 1500);
    // A mineral field carries 0x2000, not the powerup bit 0x800.
    let field = w.spawn(0, 176, 340, 300).unwrap();
    w.units[field].resource_amount = 1500;
    assert_eq!(w.unit_resource_amount(field), 0);
    let marine = w.spawn(0, uid::MARINE, 400, 300).unwrap();
    w.units[marine].resource_amount = 99;
    assert_eq!(w.unit_resource_amount(marine), 0);
}

#[test]
fn energy_gate_and_reactor_upgrades_match_the_retail_switch() {
    assert_eq!(energy_upgrade(uid::GHOST), Some(upgrade::MOEBIUS_REACTOR));
    assert_eq!(energy_upgrade(uid::WRAITH), Some(upgrade::APOLLO_REACTOR));
    assert_eq!(
        energy_upgrade(uid::SCIENCE_VESSEL),
        Some(upgrade::TITAN_REACTOR)
    );
    assert_eq!(
        energy_upgrade(uid::BATTLECRUISER),
        Some(upgrade::COLOSSUS_REACTOR)
    );
    assert_eq!(energy_upgrade(uid::MEDIC), Some(upgrade::CADUCEUS_REACTOR));
    assert_eq!(energy_upgrade(uid::QUEEN), Some(upgrade::GAMETE_MEIOSIS));
    assert_eq!(
        energy_upgrade(uid::DEFILER),
        Some(upgrade::METASYNAPTIC_NODE)
    );
    assert_eq!(energy_upgrade(uid::CORSAIR), Some(upgrade::ARGUS_JEWEL));
    assert_eq!(
        energy_upgrade(uid::DARK_ARCHON),
        Some(upgrade::ARGUS_TALISMAN)
    );
    assert_eq!(
        energy_upgrade(uid::HIGH_TEMPLAR),
        Some(upgrade::KHAYDARIN_AMULET)
    );
    assert_eq!(energy_upgrade(uid::ARBITER), Some(upgrade::KHAYDARIN_CORE));
    assert_eq!(energy_upgrade(uid::MARINE), None);

    let w = open_world();
    let mut upgrades = [0u8; 61];
    assert_eq!(
        unit_max_energy_fixed(&w.data, &upgrades, uid::GHOST),
        MAX_ENERGY * ENERGY_SCALE
    );
    upgrades[upgrade::MOEBIUS_REACTOR as usize] = 1;
    assert_eq!(
        unit_max_energy_fixed(&w.data, &upgrades, uid::GHOST),
        MAX_ENERGY_UPGRADED * ENERGY_SCALE
    );

    assert_eq!(
        unit_max_energy_points(&w.data, &upgrades, uid::GHOST),
        MAX_ENERGY_UPGRADED
    );
    assert_eq!(
        unit_max_energy_points(&w.data, &[0u8; 61], uid::MARINE),
        MAX_ENERGY
    );

    let mut w = open_world();
    let ghost = w.spawn(0, uid::GHOST, 320, 320).unwrap();
    w.units[ghost].set_energy_points(50);
    assert_eq!(w.units[ghost].energy_fixed(), 50 * ENERGY_SCALE);
    w.units[ghost].set_energy_points(999);
    assert_eq!(
        w.units[ghost].energy_fixed(),
        unit_max_energy_fixed(&w.data, &w.players[0].upgrades, uid::GHOST)
    );
    w.units[ghost].set_energy_fixed(100);
    assert!(w.units[ghost].energy_below(200));
    assert!(!w.units[ghost].energy_below(100));
    assert!(!w.units[ghost].spend_energy_fixed(200));
    assert_eq!(w.units[ghost].energy_fixed(), 100);
    assert!(w.units[ghost].spend_energy_fixed(100));
    assert_eq!(w.units[ghost].energy_fixed(), 0);
}

#[test]
fn building_and_flying_building_flag_getters_follow_the_retail_guards() {
    let w = open_world();
    // 0x005b5820 / 0x005b58e0.
    assert!(w.unit_is_building_flagged(uid::BARRACKS));
    assert!(!w.unit_is_not_building_flagged(uid::BARRACKS));
    assert!(!w.unit_is_building_flagged(uid::MARINE));
    assert!(w.unit_is_not_building_flagged(uid::MARINE));
    // Ids >= 0xe4 are outside the per-type table and count as non-buildings.
    assert!(!w.unit_is_building_flagged(0xe4));
    assert!(w.unit_is_not_building_flagged(0xe4));
    // 0x005b5ea0.
    assert!(w.unit_is_flying_building(uid::COMMAND_CENTER));
    assert!(!w.unit_is_flying_building(uid::HATCHERY));
    // 0x005b5e70.
    assert!(w.data.unit(uid::SCV).is_mechanical());
    assert!(!w.data.unit(uid::ZERGLING).is_mechanical());
}

#[test]
fn hp_point_conversions_match_the_retail_table_helpers() {
    let mut w = open_world();
    assert_eq!(
        w.unit_max_hp_third(uid::MARINE),
        w.data.unit(uid::MARINE).hit_points_raw as i32 / 3
    );
    let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert_eq!(w.unit_max_hp_points(marine), 40);
    assert_eq!(w.unit_total_hit_points(marine), 40);

    let zealot = w.spawn(1, uid::ZEALOT, 400, 320).unwrap();
    let z = w.data.unit(uid::ZEALOT);
    assert_eq!(
        w.unit_total_hit_points(zealot),
        (z.hit_points_raw as i32 >> 8) + z.shield_points as i32
    );

    // A subunit row with raw HP 1 has zero whole max HP; the helper
    // rounds the current pool up instead (FUN_00588d60).
    let turret = w.spawn(0, uid::GOLIATH_TURRET, 480, 320).unwrap();
    w.units[turret].hp = 300;
    assert_eq!(w.unit_max_hp_points(turret), 2);
    w.units[turret].hp = 1;
    assert_eq!(w.unit_max_hp_points(turret), 1);
}

#[test]
fn psi_storm_guard_blocks_a_second_hit_in_the_same_cadence() {
    let mut w = open_world();
    w.fog_of_war = false;
    let caster = w.spawn(0, uid::HIGH_TEMPLAR, 320, 320).unwrap();
    let victim = w.spawn(1, uid::MARINE, 340, 320).unwrap();
    assert_eq!(w.units[victim].storm_guard(), 0);
    let weapon = w.data.weapon(wid::PSIONIC_STORM).cloned().unwrap();
    let before = w.units[victim].hp;
    w.apply_splash(
        0,
        caster,
        wid::PSIONIC_STORM,
        &weapon,
        None,
        (340 * fp(1), 320 * fp(1)),
        false,
        false,
    );
    let after_first = w.units[victim].hp;
    assert!(after_first < before, "first storm hit must damage");
    assert_eq!(w.units[victim].storm_guard(), 1);
    w.apply_splash(
        0,
        caster,
        wid::PSIONIC_STORM,
        &weapon,
        None,
        (340 * fp(1), 320 * fp(1)),
        false,
        false,
    );
    assert_eq!(
        w.units[victim].hp, after_first,
        "guard must block the same-cadence repeat"
    );
    // Next logical tick clears the guard, so the following cadence hits.
    w.step();
    assert_eq!(w.units[victim].storm_guard(), 0);
    w.apply_splash(
        0,
        caster,
        wid::PSIONIC_STORM,
        &weapon,
        None,
        (340 * fp(1), 320 * fp(1)),
        false,
        false,
    );
    assert!(w.units[victim].hp < after_first);
}

#[test]
fn placement_and_destroy_score_getters_match_units_dat() {
    let w = open_world();
    for id in [uid::BARRACKS, uid::COMMAND_CENTER, uid::HATCHERY] {
        let ty = w.data.unit(id);
        assert_eq!(w.unit_placement_x(id), ty.placement.x as i32, "px {id}");
        assert_eq!(w.unit_placement_y(id), ty.placement.y as i32, "py {id}");
        assert_eq!(
            w.unit_destroy_score(id),
            ty.destroy_score,
            "destroy score {id}"
        );
    }
    assert!(w.unit_destroy_score(uid::MARINE) > 0);
}

#[test]
fn shield_flag_and_current_total_follow_the_retail_helpers() {
    let mut w = open_world();
    assert!(w.unit_has_shields(uid::ZEALOT));
    assert!(!w.unit_has_shields(uid::MARINE));

    let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    w.units[marine].hp = 1024; // 4 whole points.
    w.units[marine].shields = 5000; // must be ignored: no shield flag.
    assert_eq!(w.unit_current_total_points(marine), 4);

    let zealot = w.spawn(1, uid::ZEALOT, 400, 320).unwrap();
    let max_shields = w.data.unit(uid::ZEALOT).shield_points as i32;
    w.units[zealot].hp = 2560; // 10 whole points.
    w.set_unit_shields_points(zealot, max_shields + 25);
    assert_eq!(w.units[zealot].shields, max_shields << 8);
    assert_eq!(w.unit_current_total_points(zealot), 10 + max_shields);
    w.set_unit_shields_points(zealot, 7);
    assert_eq!(w.units[zealot].shields, 7 << 8);
    assert_eq!(w.unit_current_total_points(zealot), 17);
}

#[test]
fn destroying_an_upgrading_producer_refunds_seventy_five_percent() {
    let mut w = open_world();
    let academy = w.spawn(0, uid::ACADEMY, 320, 320).unwrap();
    w.units[academy].completed = true;
    w.players[0].minerals = 1000;
    w.players[0].gas = 1000;
    w.upgrade(w.units[academy].id, upgrade::U238_SHELLS)
        .unwrap();
    assert!(w.units[academy].has_pending_upgrade());
    assert!(!w.units[academy].has_pending_tech());
    let up = w.data.upgrade(upgrade::U238_SHELLS).cloned().unwrap();
    let level = w.players[0].upgrade_level(upgrade::U238_SHELLS);
    let refund_m = up.mineral_cost(level) as i32 * 3 / 4;
    let refund_g = up.gas_cost(level) as i32 * 3 / 4;
    let before_m = w.players[0].minerals;
    let before_g = w.players[0].gas;
    w.kill_unit(academy);
    assert_eq!(w.players[0].minerals, before_m + refund_m);
    assert_eq!(w.players[0].gas, before_g + refund_g);
}

#[test]
fn pending_tech_and_upgrade_presence_helpers_match_their_orders() {
    let mut w = open_world();
    let b = w.spawn(0, uid::ACADEMY, 320, 320).unwrap();
    assert!(!w.units[b].has_pending_tech());
    assert!(!w.units[b].has_pending_upgrade());
    w.units[b].order = Order::Research {
        tech_id: tech_id::STIM_PACKS,
    };
    assert!(w.units[b].has_pending_tech());
    assert!(!w.units[b].has_pending_upgrade());
    w.units[b].order = Order::Upgrade {
        upgrade_id: upgrade::U238_SHELLS,
        level: 1,
    };
    assert!(w.units[b].has_pending_upgrade());
    assert!(!w.units[b].has_pending_tech());
}

#[test]
fn shield_race_damage_energy_and_box_width_helpers_match_retail() {
    let mut w = open_world();
    assert_eq!(
        w.unit_shield_points(uid::ZEALOT),
        w.data.unit(uid::ZEALOT).shield_points as i32
    );
    assert_eq!(w.unit_race_class(uid::ZERGLING), 0);
    assert_eq!(w.unit_race_class(uid::MARINE), 1);
    assert_eq!(w.unit_race_class(uid::ZEALOT), 2);
    let ghost = w.spawn(0, uid::GHOST, 300, 300).unwrap();
    let marine = w.spawn(0, uid::MARINE, 360, 300).unwrap();
    assert!(w.unit_has_energy(ghost));
    assert!(!w.unit_has_energy(marine));

    w.units[marine].hp = 100;
    assert!(w.unit_is_damaged(marine));
    w.units[marine].hp = w.data.unit(uid::MARINE).hit_points_raw as i32;
    assert!(!w.unit_is_damaged(marine));

    for id in [uid::MARINE, uid::BARRACKS, uid::HATCHERY] {
        let d = w.data.unit(id).dimensions;
        assert_eq!(
            w.unit_box_width(id),
            d.right as i32 + 1 + d.left as i32,
            "width {id}"
        );
    }
}

#[test]
fn unit_type_predicates_match_the_retail_rows() {
    assert!(is_suicide_unit_type(0x2f));
    assert!(is_suicide_unit_type(0x32));
    assert!(!is_suicide_unit_type(uid::MARINE));
    assert!(is_beacon_type(0xc2));
    assert!(is_beacon_type(0xc7));
    assert!(is_beacon_type(0xcc));
    assert!(!is_beacon_type(uid::MARINE));
    assert!(is_ultralisk_type(0x27));
    assert!(is_ultralisk_type(0x30));
    assert!(!is_ultralisk_type(0x28));
    assert!(is_vulture_type(0x02));
    assert!(is_vulture_type(0x13));
    assert!(!is_vulture_type(uid::MARINE));
    assert!(is_zealot_type(0x41));
    assert!(is_zealot_type(0x4d));
    assert!(!is_zealot_type(uid::DRAGOON));
    assert!(is_zergling_type(0x25));
    assert!(is_zergling_type(0x36));
    assert!(!is_zergling_type(uid::HYDRALISK));
    assert!(is_critter_type(0x59));
    assert!(is_critter_type(0x5a));
    assert!(is_critter_type(0x5d));
    assert!(is_critter_type(0x60));
    assert!(!is_critter_type(0x61));
    assert!(is_gas_building(uid::REFINERY));
    assert!(is_gas_building(uid::EXTRACTOR));
    assert!(is_gas_building(uid::ASSIMILATOR));
    assert!(!is_gas_building(uid::COMMAND_CENTER));
}

#[test]
fn status_refresh_helpers_keep_the_longest_timer_and_clear_on_expiry() {
    let mut s = Status::default();
    s.apply_lockdown(3);
    s.apply_lockdown(1);
    assert_eq!(s.lockdown_ticks, 3);
    s.apply_lockdown(7);
    assert_eq!(s.lockdown_ticks, 7);
    s.clear_lockdown();
    assert_eq!(s.lockdown_ticks, 0);

    s.apply_stasis(2);
    s.apply_stasis(9);
    assert_eq!(s.stasis_ticks, 9);
    s.clear_stasis();
    assert_eq!(s.stasis_ticks, 0);

    s.apply_maelstrom(4);
    s.apply_maelstrom(1);
    assert_eq!(s.maelstrom_ticks, 4);
    s.clear_maelstrom();
    assert_eq!(s.maelstrom_ticks, 0);

    s.apply_plague();
    assert_eq!(s.plague_ticks, PLAGUE_UPDATES);
    s.clear_plague();
    assert_eq!(s.plague_ticks, 0);
    s.apply_lockdown(2);
    s.clear_lockdown();
    assert_eq!(s.lockdown_ticks, 0);
    s.ensnare_ticks = 5;
    s.clear_ensnare();
    assert_eq!(s.ensnare_ticks, 0);
    s.apply_ensnare();
    assert_eq!(s.ensnare_ticks, ENSNARE_UPDATES);
    s.apply_irradiate(IRRADIATE_UPDATES);
    assert_eq!(s.irradiate_ticks, IRRADIATE_UPDATES);
    s.clear_irradiate();
    assert_eq!(s.irradiate_ticks, 0);
}

#[test]
fn death_clears_removable_status_timers_and_acid_spores() {
    let mut w = open_world();
    let z = w.spawn(0, uid::ZERGLING, 320, 320).unwrap();
    let st = &mut w.units[z].status;
    st.ensnare_ticks = 5;
    st.plague_ticks = 5;
    st.irradiate_ticks = 5;
    st.lockdown_ticks = 5;
    st.maelstrom_ticks = 5;
    st.add_acid_spore();
    assert!(st.acid_spores > 0);
    w.kill_unit(z);
    let st = &w.units[z].status;
    assert_eq!(st.ensnare_ticks, 0);
    assert_eq!(st.plague_ticks, 0);
    assert_eq!(st.irradiate_ticks, 0);
    assert_eq!(st.lockdown_ticks, 0);
    assert_eq!(st.maelstrom_ticks, 0);
    assert_eq!(st.acid_spores, 0);
}

#[test]
fn unit_voice_timer_and_sound_id_follow_the_cunit_fields() {
    let mut w = open_world();
    let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert_eq!(w.units[marine].speech_sound(), 0);
    assert_eq!(w.units[marine].tick_speech(), None);
    w.units[marine].say(2, 0x2a);
    assert_eq!(w.units[marine].speech_sound(), 0x2a);
    assert_eq!(w.units[marine].speech_ticks, 2);
    w.step();
    assert_eq!(w.units[marine].speech_ticks, 1);
    w.step();
    assert_eq!(w.units[marine].speech_ticks, 0);
    // The getter is an atomic read/decrement.
    w.units[marine].say(3, 1);
    assert_eq!(w.units[marine].tick_speech(), Some(3));
    assert_eq!(w.units[marine].tick_speech(), Some(2));
    assert_eq!(w.units[marine].tick_speech(), Some(1));
    assert_eq!(w.units[marine].tick_speech(), None);
}

#[test]
fn completed_loaded_and_cloak_flags_follow_the_unit_state() {
    let mut w = open_world();
    let barracks = w.spawn(0, uid::BARRACKS, 320, 320).unwrap();
    w.units[barracks].completed = false;
    assert!(!w.units[barracks].is_completed_flag());
    w.units[barracks].completed = true;
    assert!(w.units[barracks].is_completed_flag());

    let marine = w.spawn(0, uid::MARINE, 400, 320).unwrap();
    assert!(!w.units[marine].is_loaded());
    w.units[marine].in_transport = Some(w.units[barracks].id);
    assert!(w.units[marine].is_loaded());

    assert!(!w.units[marine].is_burrowed_flags());
    w.units[marine].status.burrowed = true;
    assert!(w.units[marine].is_burrowed_flags());

    let ghost = w.spawn(0, uid::GHOST, 480, 320).unwrap();
    assert!(!w.units[ghost].cloak_bit());
    w.units[ghost].status.cloaked = true;
    assert_eq!(w.units[ghost].cloak_state(), 2);
    assert!(w.units[ghost].cloak_bit());
}

#[test]
fn research_timer_and_death_timer_are_readable_and_tick_down() {
    let mut w = open_world();
    let academy = w.spawn(0, uid::ACADEMY, 320, 320).unwrap();
    w.units[academy].completed = true;
    w.players[0].minerals = 1000;
    w.players[0].gas = 1000;
    w.upgrade(w.units[academy].id, upgrade::U238_SHELLS)
        .unwrap();
    let remaining = w.units[academy].research_remaining();
    assert!(remaining > 0);
    assert_eq!(remaining as i32, w.units[academy].build_remaining);

    let z = w.spawn(0, uid::ZERGLING, 420, 320).unwrap();
    assert_eq!(w.units[z].death_timer(), 0);
    w.kill_unit(z);
    assert_eq!(w.units[z].death_timer(), 1350);
    w.step();
    assert_eq!(w.units[z].death_timer(), 1349);
    // `0x005b6810` keeps the smaller value once it is set.
    w.units[z].set_death_timer_min(2000);
    assert_eq!(w.units[z].death_timer(), 1349);
    w.units[z].set_death_timer_min(10);
    assert_eq!(w.units[z].death_timer(), 10);
}

#[test]
fn rank_kill_counter_increments_and_caps_at_255() {
    let mut w = open_world();
    let a = w.spawn(0, uid::MARINE, 300, 300).unwrap();
    let b = w.spawn(1, uid::ZERGLING, 340, 300).unwrap();
    assert_eq!(w.units[a].rank_kills(), 0);
    w.units[b].last_damage_source = w.units[a].id;
    w.kill_unit(b);
    assert_eq!(w.units[a].rank_kills(), 1);
    assert_eq!(w.units[a].kills, 1);
    w.units[a].rank_kills = 254;
    w.units[a].credit_rank_kill();
    assert_eq!(w.units[a].rank_kills(), 255);
    w.units[a].credit_rank_kill();
    assert_eq!(w.units[a].rank_kills(), 255);
}

#[test]
fn pending_research_raw_sum_and_queue_count_match_the_cunit_fields() {
    let mut w = open_world();
    let academy = w.spawn(0, uid::ACADEMY, 320, 320).unwrap();
    assert_eq!(w.units[academy].pending_tech_raw(), 0);
    assert_eq!(w.units[academy].pending_research_sum(), 0);
    assert_eq!(w.units[academy].queued_count(), 0);
    w.units[academy].order = Order::Research {
        tech_id: tech_id::STIM_PACKS,
    };
    assert_eq!(
        w.units[academy].pending_tech_raw(),
        u32::from(tech_id::STIM_PACKS)
    );
    assert_eq!(w.units[academy].pending_research_sum(), tech_id::STIM_PACKS);
    w.units[academy].order = Order::Upgrade {
        upgrade_id: upgrade::U238_SHELLS,
        level: 1,
    };
    assert_eq!(
        w.units[academy].pending_research_sum(),
        upgrade::U238_SHELLS
    );
    w.units[academy].production.push_back(Order::Train {
        unit_type: uid::MARINE,
    });
    assert_eq!(w.units[academy].queued_count(), 1);
}

#[test]
fn placement_precheck_uses_box_extents_and_terrain_flags() {
    let w = open_world();
    let (wpx, hpx) = (w.terrain.width as i32 * 32, w.terrain.height as i32 * 32);
    // Find a buildable point away from the map border.
    let mut probe = None;
    'outer: for y in (64..hpx - 64).step_by(32) {
        for x in (64..wpx - 64).step_by(32) {
            if w.terrain_accepts_placement(x, y) {
                probe = Some((x, y));
                break 'outer;
            }
        }
    }
    let (px, py) = probe.expect("test map needs one buildable point");
    assert!(w.placement_corner_valid(uid::BARRACKS, px, py));
    assert!(!w.placement_corner_valid(uid::BARRACKS, -64, py));
    assert!(!w.placement_corner_valid(uid::BARRACKS, wpx, py));
    assert!(!w.placement_corner_valid(uid::BARRACKS, px, -64));
    assert!(!w.placement_corner_valid(uid::BARRACKS, px, hpx));
}

#[test]
fn status_merge_keeps_the_longest_timers_and_largest_spores() {
    let mut a = Status::default();
    let mut b = Status::default();
    a.lockdown_ticks = 5;
    b.lockdown_ticks = 9;
    a.stasis_ticks = 12;
    b.stasis_ticks = 3;
    a.matrix_ticks = 2;
    a.matrix = 100;
    b.matrix_ticks = 7;
    b.matrix = 500;
    a.acid_spores = 2;
    a.acid_spore_ticks[0] = 4;
    b.acid_spores = 4;
    b.acid_spore_ticks[0] = 9;
    b.parasite = true;
    a.merge_from(&b);
    assert_eq!(a.lockdown_ticks, 9);
    assert_eq!(a.stasis_ticks, 12);
    assert_eq!(a.matrix_ticks, 7);
    assert_eq!(a.matrix, 500);
    assert_eq!(a.acid_spores, 4);
    assert_eq!(a.acid_spore_ticks[0], 9);
    assert!(a.parasite);
}

#[test]
fn archon_merge_inherits_source_statuses() {
    let mut w = open_world();
    let a = w.spawn(0, uid::HIGH_TEMPLAR, 320, 320).unwrap();
    let b = w.spawn(0, uid::HIGH_TEMPLAR, 340, 320).unwrap();
    w.units[a].status.plague_ticks = 40;
    w.units[b].status.ensnare_ticks = 70;
    w.cast(w.units[a].id, Spell::ArchonWarp, SpellTarget::None)
        .unwrap();
    let archon = w
        .units
        .iter()
        .find(|u| u.alive && u.type_id == uid::ARCHON)
        .expect("archon spawned");
    assert_eq!(archon.status.plague_ticks, 40);
    assert_eq!(archon.status.ensnare_ticks, 70);
}

#[test]
fn linked_nuke_missile_state_tracks_launches() {
    let mut w = open_world();
    let cc = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
    let silo = w.spawn(0, uid::NUCLEAR_SILO, 352, 320).unwrap();
    w.units[silo].connected_unit = Some(w.units[cc].id);
    let silo_id = w.units[silo].id;
    assert!(w.has_no_linked_missile(silo_id));
    assert!(w.link_nuke_missile(silo_id));
    w.nuke_strikes.push(NukeStrike {
        owner: 0,
        ghost: 0,
        silo: silo_id,
        target: (320, 320),
        timer: 100,
        dot: true,
        warned: false,
        missile: 0,
    });
    assert!(!w.has_no_linked_missile(silo_id));
    let other = w.spawn(0, uid::NUCLEAR_SILO, 420, 320).unwrap();
    assert!(w.has_no_linked_missile(w.units[other].id));
}

#[test]
fn per_unit_masks_set_clear_and_merge_bitwise() {
    let mut w = open_world();
    let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert_eq!(w.units[u].mask_121(), 0);
    assert_eq!(w.units[u].mask_123(), 0);
    w.units[u].set_mask_121_bit(0);
    w.units[u].set_mask_121_bit(3);
    w.units[u].set_mask_123_bit(2);
    assert_eq!(w.units[u].mask_121(), 0b0000_1001);
    assert_eq!(w.units[u].mask_123(), 0b0000_0100);
    let other = Status {
        status_mask_121: 0b0000_0010,
        status_mask_123: 0b0000_0001,
        ..Status::default()
    };
    w.units[u].status.merge_from(&other);
    assert_eq!(w.units[u].mask_121(), 0b0000_1011);
    assert_eq!(w.units[u].mask_123(), 0b0000_0101);
    w.units[u].clear_mask_121();
    w.units[u].clear_mask_123();
    assert_eq!(w.units[u].mask_121(), 0);
    assert_eq!(w.units[u].mask_123(), 0);
}

#[test]
fn zerg_queue_helpers_expose_the_front_unit_or_the_type_itself() {
    assert!(is_zerg_queue_building(uid::LAIR));
    assert!(is_zerg_queue_building(uid::HIVE));
    assert!(is_zerg_queue_building(uid::GREATER_SPIRE));
    assert!(is_zerg_queue_building(uid::SPORE_COLONY));
    assert!(is_zerg_queue_building(uid::SUNKEN_COLONY));
    assert!(!is_zerg_queue_building(uid::HATCHERY));

    let mut w = open_world();
    let lair = w.spawn(0, uid::LAIR, 320, 320).unwrap();
    assert_eq!(w.units[lair].current_queue_or_self(), None);
    w.units[lair].production.push_back(Order::Train {
        unit_type: uid::MUTALISK,
    });
    assert_eq!(w.units[lair].current_queue_or_self(), Some(uid::MUTALISK));
    let marine = w.spawn(0, uid::MARINE, 400, 320).unwrap();
    assert_eq!(w.units[marine].current_queue_or_self(), Some(uid::MARINE));
}

#[test]
fn production_queue_clears_on_demand_and_on_death() {
    let mut w = open_world();
    let barracks = w.spawn(0, uid::BARRACKS, 320, 320).unwrap();
    w.units[barracks].production.push_back(Order::Train {
        unit_type: uid::MARINE,
    });
    w.units[barracks].production.push_back(Order::Train {
        unit_type: uid::FIREBAT,
    });
    assert_eq!(w.units[barracks].queued_count(), 2);
    w.units[barracks].clear_production_queue();
    assert_eq!(w.units[barracks].queued_count(), 0);
    w.units[barracks].production.push_back(Order::Train {
        unit_type: uid::MARINE,
    });
    w.kill_unit(barracks);
    assert_eq!(w.units[barracks].queued_count(), 0);
}

#[test]
fn dequeue_removes_and_refunds_the_newest_queued_item() {
    let mut w = open_world();
    let barracks = w.spawn(0, uid::BARRACKS, 320, 320).unwrap();
    let id = w.units[barracks].id;
    w.units[barracks].production.push_back(Order::Train {
        unit_type: uid::MARINE,
    });
    w.units[barracks].production.push_back(Order::Train {
        unit_type: uid::FIREBAT,
    });
    w.players[0].minerals = 0;
    w.players[0].gas = 0;
    let firebat = w.data.unit(uid::FIREBAT).clone();
    assert!(w.dequeue_production(id).unwrap());
    assert_eq!(w.units[barracks].queued_count(), 1);
    assert_eq!(w.players[0].minerals, firebat.mineral_cost as i32);
    assert_eq!(w.players[0].gas, firebat.gas_cost as i32);
    assert!(w.dequeue_production(id).unwrap());
    assert_eq!(w.units[barracks].queued_count(), 0);
    assert!(!w.dequeue_production(id).unwrap());
}

#[test]
fn cancelling_construction_clears_per_unit_masks() {
    let mut w = open_world();
    let b = w.spawn(0, uid::BARRACKS, 320, 320).unwrap();
    w.units[b].completed = false;
    w.units[b].set_mask_121_bit(1);
    w.units[b].set_mask_123_bit(2);
    assert_ne!(w.units[b].mask_121(), 0);
    assert_ne!(w.units[b].mask_123(), 0);
    w.cancel_construction(w.units[b].id).unwrap();
    assert_eq!(w.units[b].mask_121(), 0);
    assert_eq!(w.units[b].mask_123(), 0);
    assert!(!w.units[b].alive);
}

#[test]
fn generic_087_timer_sets_ticks_and_reports_zero() {
    let mut w = open_world();
    let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert!(w.units[u].timer_087_is_zero());
    w.units[u].set_timer_087(3);
    assert!(!w.units[u].timer_087_is_zero());
    w.units[u].tick_timer_087();
    assert_eq!(w.units[u].timer_087, 2);
    w.step();
    assert_eq!(w.units[u].timer_087, 1);
    w.step();
    assert!(w.units[u].timer_087_is_zero());
    w.units[u].tick_timer_087();
    assert_eq!(w.units[u].timer_087, 0);
}

#[test]
fn visibility_masks_track_fog_and_the_masked_query() {
    let mut w = open_world();
    let own = w.spawn(0, uid::MARINE, 32, 32).unwrap();
    let far = w.spawn(1, uid::ZERGLING, 2000, 2000).unwrap();
    assert_eq!(w.units[own].visibility_mask(), u32::MAX);
    assert_eq!(w.units[own].visibility_mask_and(1), 1);
    w.enable_fog();
    assert_ne!(w.units[own].visibility_mask() & 1, 0);
    assert_eq!(w.units[far].visibility_mask() & 1, 0);
    assert_ne!(w.units[far].visibility_mask() & 2, 0);
    assert_eq!(w.units[own].visibility_mask_and(1), 1);
    assert_eq!(w.units[far].visibility_mask_and(1), 0);
}

#[test]
fn raw_cunit_getters_return_their_stored_fields() {
    let mut w = open_world();
    let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert_eq!(w.units[u].field_090(), 8);
    assert_eq!(w.units[u].field_094(), 0);
    assert_eq!(w.units[u].field_0ac(), 0);
    assert_eq!(w.units[u].field_0e1(), 0);
    w.units[u].field_090 = 3;
    w.units[u].field_094 = 0x1234;
    w.units[u].field_0ac = 0xbeef;
    w.units[u].field_0e1 = 7;
    assert_eq!(w.units[u].field_090(), 3);
    assert_eq!(w.units[u].field_094(), 0x1234);
    assert_eq!(w.units[u].field_0ac(), 0xbeef);
    assert_eq!(w.units[u].field_0e1(), 7);
}

#[test]
fn killing_a_units_order_target_clears_the_order() {
    let mut w = open_world();
    let attacker = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let victim = w.spawn(1, uid::ZERGLING, 360, 320).unwrap();
    let victim_id = w.units[victim].id;
    w.units[attacker].order = Order::AttackUnit { target: victim_id };
    assert_eq!(w.units[attacker].order_unit_target(), Some(victim_id));
    w.kill_unit(victim);
    assert_eq!(w.units[attacker].order_unit_target(), None);
    assert!(matches!(w.units[attacker].order, Order::Idle));
}

#[test]
fn worker_linked_target_helpers_use_the_order_target() {
    let mut w = open_world();
    let scv = w.spawn(0, uid::SCV, 320, 320).unwrap();
    let flag = w.spawn(0, 0xd7, 360, 320).unwrap();
    let flag_id = w.units[flag].id;
    w.units[scv].order = Order::Construct { building: flag_id };
    assert_eq!(w.units[scv].linked_target(), Some(flag_id));
    assert_eq!(w.unit_worker_target(scv), Some(flag_id));
    assert!(w.unit_worker_target_is_flag(scv));

    let marine = w.spawn(0, uid::MARINE, 400, 320).unwrap();
    w.units[marine].order = Order::Construct { building: flag_id };
    assert_eq!(w.units[marine].linked_target(), Some(flag_id));
    assert_eq!(w.unit_worker_target(marine), None);
    assert!(!w.unit_worker_target_is_flag(marine));
}

#[test]
fn bunker_bit_tracks_loading_and_unloading() {
    let mut w = open_world();
    let bunker = w.spawn(0, uid::BUNKER, 320, 320).unwrap();
    let marine = w.spawn(0, uid::MARINE, 340, 320).unwrap();
    assert!(!w.units[marine].is_in_bunker());
    let bunker_id = w.units[bunker].id;
    let marine_id = w.units[marine].id;
    w.order_enter_transport(marine, bunker_id);
    for _ in 0..200 {
        if w.units[marine].in_transport.is_some() {
            break;
        }
        w.step();
    }
    assert!(w.units[marine].is_loaded());
    assert!(w.units[marine].is_in_bunker());
    w.unload_unit(bunker_id, marine_id).unwrap();
    assert!(!w.units[marine].is_loaded());
    assert!(!w.units[marine].is_in_bunker());
}

#[test]
fn status_bit_26_is_cleared_for_unsupported_rows_only() {
    let mut w = open_world();
    let marine = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let field = w.spawn(0, 176, 420, 320).unwrap();
    assert!(!w.units[marine].flag_26());
    w.units[marine].flag_26 = true;
    w.units[field].flag_26 = true;
    w.clear_unsupported_flag_26(marine);
    assert!(!w.units[marine].flag_26());
    w.clear_unsupported_flag_26(field);
    assert!(w.units[field].flag_26());
}

#[test]
fn overlay_state_bits_follow_the_two_mode_or() {
    let mut w = open_world();
    let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert!(!w.units[u].overlay_bit0());
    assert!(!w.units[u].overlay_bit1());
    w.units[u].set_overlay_state(false, 9);
    assert!(w.units[u].overlay_bit0());
    assert!(!w.units[u].overlay_bit1());
    assert_eq!(w.units[u].field_0cf, 9);
    w.units[u].set_overlay_state(true, 4);
    assert!(w.units[u].overlay_bit0());
    assert!(w.units[u].overlay_bit1());
    assert_eq!(w.units[u].overlay_flags, 3);
    assert_eq!(w.units[u].field_0cf, 4);
}

#[test]
fn raw_status_bits_10_14_15_and_29_toggle_exactly() {
    let mut w = open_world();
    let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert!(!w.units[u].flag_10);
    assert!(!w.units[u].flag_14());
    assert!(!w.units[u].flag_15());
    assert!(!w.units[u].flag_29);
    w.units[u].set_flag_10();
    w.units[u].set_flag_15();
    w.units[u].set_flag_29();
    assert!(w.units[u].flag_10);
    assert!(w.units[u].flag_15());
    assert!(w.units[u].flag_29);
    w.units[u].clear_flag_10();
    w.units[u].clear_flag_15();
    assert!(!w.units[u].flag_10);
    assert!(!w.units[u].flag_15());
    w.units[u].flag_14 = true;
    assert!(w.units[u].flag_14());
}

#[test]
fn status_bits_21_22_and_105_read_their_storage() {
    let mut w = open_world();
    let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert!(!w.units[u].flag_21());
    assert!(!w.units[u].flag_22());
    assert!(!w.units[u].flag_105_bit0());
    assert!(w.units[u].flag_105_bit1_clear());
    w.units[u].flag_21 = true;
    w.units[u].flag_22 = true;
    w.units[u].flags_105 = 0b11;
    assert!(w.units[u].flag_21());
    assert!(w.units[u].flag_22());
    assert!(w.units[u].flag_105_bit0());
    assert!(!w.units[u].flag_105_bit1_clear());
    w.units[u].flags_105 = 0b01;
    assert!(w.units[u].flag_105_bit0());
    assert!(w.units[u].flag_105_bit1_clear());
}

#[test]
fn order_flags_93_96_and_transport_link_helpers_are_exact() {
    let mut w = open_world();
    let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    w.units[u].set_flag_93_bit1(true);
    assert_eq!(w.units[u].flags_93, 2);
    w.units[u].set_flag_93_bit1(false);
    assert_eq!(w.units[u].flags_93, 0);
    w.units[u].field_096 = 7;
    w.units[u].clear_field_096();
    assert_eq!(w.units[u].field_096, 0);

    assert!(!w.units[u].is_loaded());
    w.units[u].mark_in_transport(42);
    assert!(w.units[u].is_loaded());
    assert_eq!(w.units[u].in_transport, Some(42));
    w.units[u].clear_transport_link();
    assert!(!w.units[u].is_loaded());
}

#[test]
fn status_bits_18_19_and_pointer_100_are_exact() {
    let mut w = open_world();
    let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert!(!w.units[u].flag_18);
    assert!(!w.units[u].flag_19);
    assert_eq!(w.units[u].field_100(), 0);
    assert!(!w.units[u].field_100_nonzero());
    w.units[u].set_flag_18(true);
    w.units[u].set_flag_19(true);
    w.units[u].set_field_100(0x1234);
    assert!(w.units[u].flag_18);
    assert!(w.units[u].flag_19);
    assert_eq!(w.units[u].field_100(), 0x1234);
    assert!(w.units[u].field_100_nonzero());
    w.units[u].set_flag_18(false);
    w.units[u].set_flag_19(false);
    w.units[u].set_field_100(0);
    assert!(!w.units[u].flag_18);
    assert!(!w.units[u].flag_19);
    assert!(!w.units[u].field_100_nonzero());
}

#[test]
fn shield_battery_marks_the_target_for_one_tick() {
    let mut w = open_world();
    let battery = w.spawn(0, uid::SHIELD_BATTERY, 320, 320).unwrap();
    let zealot = w.spawn(0, uid::ZEALOT, 360, 320).unwrap();
    w.units[battery].completed = true;
    w.units[battery].energy = 100;
    w.units[zealot].shields = 0;
    assert!(!w.units[zealot].shield_recharge_flag());
    assert!(w.can_shield_battery_target(w.units[battery].id, w.units[zealot].id));
    assert!(!w.can_shield_battery_target(w.units[battery].id, w.units[battery].id));
    w.shield_battery(w.units[battery].id, w.units[zealot].id)
        .unwrap();
    assert!(w.units[zealot].shield_recharge_flag());
    assert!(w.units[zealot].shields > 0);
    w.step();
    assert!(!w.units[zealot].shield_recharge_flag());
}

#[test]
fn subunit_state_sets_bits_16_and_17() {
    let mut w = open_world();
    let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    w.units[u].set_subunit_state(false);
    assert!(w.units[u].flag_16);
    assert!(w.units[u].flag_17);
    w.units[u].set_subunit_state(true);
    assert!(w.units[u].flag_16);
    assert!(!w.units[u].flag_17);
}

#[test]
fn unit_slot_numbers_are_one_based_indices() {
    let mut w = open_world();
    let a = w.spawn(0, uid::MARINE, 300, 300).unwrap();
    let b = w.spawn(0, uid::MARINE, 340, 300).unwrap();
    assert_eq!(w.unit_slot_number(a), a as u32 + 1);
    assert_eq!(w.unit_slot_number(b), b as u32 + 1);
    assert_eq!(w.unit_slot_number(b), w.unit_slot_number(a) + 1);
}

#[test]
fn unit_flag_29_predicate_matches_units_dat() {
    assert!(w_flag29(uid::MARINE));
    assert!(!w_flag29(176)); // Mineral Field carries 0x20000000.
    assert!(!w_flag29(0xe4));
}

fn w_flag29(type_id: u16) -> bool {
    let w = open_world();
    w.unit_flag_29_clear(type_id)
}

#[test]
fn rotate_byte_6_matches_the_shift_or_expression() {
    assert_eq!(rotate_byte_6(0b0000_0001), 0b0000_0100);
    assert_eq!(rotate_byte_6(0b0100_0000), 0b0000_0001);
    assert_eq!(rotate_byte_6(0), 0);
    for v in 0u8..=255 {
        assert_eq!(rotate_byte_6(v), v.rotate_left(2));
    }
}

#[test]
fn order_carry_setter_matches_the_retail_minus_one() {
    let mut w = open_world();
    let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert!(w.units[u].set_order_carry_from(4));
    assert_eq!(w.units[u].order_carry, 3);
    assert!(w.units[u].set_order_carry_from(0));
    assert_eq!(w.units[u].order_carry, 255);
}

#[test]
fn order_carry_advances_through_the_batch_35_step() {
    let mut w = open_world();
    let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    w.units[u].order_carry = 0;
    assert!(w.units[u].a35_advance_order_carry());
    assert_eq!(w.units[u].order_carry, 8);
    assert!(!w.units[u].a35_advance_order_carry());
    assert_eq!(w.units[u].order_carry, 7);
}

#[test]
fn idle_or_hidden_predicate_matches_order_zero_or_no_sprite() {
    let mut w = open_world();
    let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    assert!(w.units[u].is_idle_or_hidden());
    w.units[u].order = Order::Move { pos: (400, 320) };
    assert!(!w.units[u].is_idle_or_hidden());
    w.units[u].alive = false;
    assert!(w.units[u].is_idle_or_hidden());
}

#[test]
fn units_in_rect_returns_only_centres_inside_the_box() {
    let mut w = open_world();
    let a = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let b = w.spawn(1, uid::ZERGLING, 380, 320).unwrap();
    let c = w.spawn(0, uid::MARINE, 700, 700).unwrap();
    let mut ids = w.units_in_rect((350, 320), 64);
    ids.sort_unstable();
    let mut want = vec![w.units[a].id, w.units[b].id];
    want.sort_unstable();
    assert_eq!(ids, want);
    assert!(!ids.contains(&w.units[c].id));
    let ids = w.units_in_rect((700, 700), 8);
    assert_eq!(ids, vec![w.units[c].id]);
}

#[test]
fn units_near_point_uses_the_32px_half_extent() {
    let mut w = open_world();
    let a = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    let b = w.spawn(0, uid::MARINE, 360, 320).unwrap();
    let c = w.spawn(0, uid::MARINE, 380, 320).unwrap();
    let mut ids = w.units_near_point((331, 320));
    ids.sort_unstable();
    let mut want = vec![w.units[a].id, w.units[b].id];
    want.sort_unstable();
    assert_eq!(ids, want);
    assert!(!ids.contains(&w.units[c].id));
}

#[test]
fn destroyed_nuclear_silo_drops_its_missile_and_parent_link() {
    let mut w = open_world();
    let cc = w.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
    let silo = w.spawn(0, uid::NUCLEAR_SILO, 352, 320).unwrap();
    w.units[cc].addon = Some(w.units[silo].id);
    w.units[silo].addon_attached = true;
    w.units[silo].nuke_ready = true;
    w.kill_unit(silo);
    assert!(!w.units[silo].nuke_ready);
    assert_eq!(w.units[cc].addon, None);
}

#[test]
fn field_091_clears_like_the_retail_initializer() {
    let mut w = open_world();
    let u = w.spawn(0, uid::MARINE, 320, 320).unwrap();
    w.units[u].field_091 = 9;
    w.units[u].clear_field_091();
    assert_eq!(w.units[u].field_091, 0);
}

#[test]
fn owner_predicate_matches_the_retail_assertion() {
    let mut w = open_world();
    let u = w.spawn(1, uid::MARINE, 320, 320).unwrap();
    assert!(w.units[u].owner_is(1));
    assert!(!w.units[u].owner_is(0));
}
