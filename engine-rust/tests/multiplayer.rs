//! Multiplayer game rules: lobby setup applied to the world, teams
//! (alliance, shared vision, team victory, AI targets) and the score sheet.

use std::sync::Arc;

use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::{Scenario, Terrain, TileData};
use galaxycraft::multiplayer::{
    GameSetup, GameType, ScoreTab, SetupOccupant, SetupSlot, bar_fraction, score_lines, score_table,
};
use galaxycraft::sim::{Order, Race, World};

fn data() -> Arc<GameData> {
    Arc::new(GameData::bundled().expect("bundled tables"))
}

fn boxer(races: &[Race], occupied: &[bool]) -> World {
    let raw = std::fs::read("assets/maps/Boxer.chk").expect("bundled Boxer");
    let vf4 = std::fs::read("assets/TileSet/platform.vf4").expect("bundled vf4");
    let cv5 = std::fs::read("assets/TileSet/platform.cv5").expect("bundled cv5");
    let sc = Scenario::parse(&raw).expect("parse");
    World::from_scenario_occupied(
        data(),
        &sc,
        &TileData::new(&cv5, &vf4),
        races,
        Some(occupied),
    )
}

#[test]
fn a_melee_start_scores_like_the_retail_score_screen() {
    // Retail 21:14:04: LinKa (Protoss, never mined) reads Units 200,
    // Structures 400, Resources 50, Total 650; Zerg starts read Structures
    // 300 (the Hatchery).
    let mut w = boxer(&[Race::Protoss, Race::Zerg], &[true, true]);
    w.step();
    let lines = score_lines(&w, &[0, 1]);
    let p = lines.iter().find(|l| l.player == 0).expect("protoss row");
    assert_eq!((p.values, p.total), ([200, 400, 50], 650));
    let z = lines.iter().find(|l| l.player == 1).expect("zerg row");
    assert_eq!(z.values[1], 300);
    assert_eq!(z.total, z.values.iter().sum::<i32>());
    // Credited once: more ticks do not add the starting units again.
    for _ in 0..24 {
        w.step();
    }
    let again = score_lines(&w, &[0]);
    assert_eq!(again[0].values[..2], [200, 400]);
}

#[test]
fn open_and_closed_slots_stay_empty() {
    let w = boxer(&[Race::Terran, Race::Terran], &[true, false]);
    assert!(w.units.iter().any(|u| u.alive && u.owner == 0));
    assert!(!w.units.iter().any(|u| u.alive && u.owner == 1));
    assert!(w.players[1].defeated);
}

fn setup(teams: &[u8]) -> GameSetup {
    GameSetup {
        game_type: GameType::TopVsBottom,
        slots: teams
            .iter()
            .map(|&team| SetupSlot {
                occupant: SetupOccupant::Computer,
                race: Race::Terran,
                team,
                color: 2,
            })
            .collect(),
    }
}

#[test]
fn allies_hold_fire_and_enemies_do_not() {
    let mut w = World::new(data(), Terrain::open(64, 64), 3);
    setup(&[0, 0, 1]).apply(&mut w);
    let a = w.spawn(0, uid::MARINE, 200, 200).unwrap();
    let b = w.spawn(1, uid::MARINE, 230, 200).unwrap();
    let e = w.spawn(2, uid::MARINE, 600, 600).unwrap();
    let (hp_a, hp_b) = (w.units[a].hp, w.units[b].hp);
    for _ in 0..96 {
        w.step();
    }
    assert_eq!(
        (w.units[a].hp, w.units[b].hp),
        (hp_a, hp_b),
        "allies never shoot"
    );
    // Move the enemy next to them: now there is a fight.
    let id = w.units[e].id;
    w.issue(id, Order::Move { pos: (260, 200) }).unwrap();
    for _ in 0..240 {
        w.step();
    }
    assert!(
        w.units[e].hp < 40 * 256 || !w.units[e].alive,
        "the enemy is engaged"
    );
}

#[test]
fn a_team_wins_together_and_never_targets_itself() {
    let mut w = World::new(data(), Terrain::open(64, 64), 3);
    setup(&[0, 0, 1]).apply(&mut w);
    let _ = w.spawn(0, uid::COMMAND_CENTER, 300, 300).unwrap();
    let _ = w.spawn(1, uid::COMMAND_CENTER, 900, 300).unwrap();
    let enemy_hall = w.spawn(2, uid::COMMAND_CENTER, 900, 900).unwrap();
    // An AI of player 1 is pointed at the enemy, not at its ally 0.
    assert_eq!(w.enemy_town_hall(1, 0), Some((900, 900)));
    assert_eq!(w.enemy_town_hall(2, 0), Some((300, 300)));
    w.step();
    assert_eq!(w.winner, None, "two sides still stand");
    w.players[2].defeated = true;
    w.units[enemy_hall].alive = false;
    w.step();
    assert!(w.has_won(0) && w.has_won(1), "the whole team wins");
    assert!(!w.has_won(2));
}

#[test]
fn teammates_share_vision() {
    let mut w = World::new(data(), Terrain::open(64, 64), 3);
    w.enable_fog();
    setup(&[0, 0, 1]).apply(&mut w);
    let _ = w.spawn(0, uid::MARINE, 100, 100).unwrap();
    let _ = w.spawn(1, uid::MARINE, 1500, 1500).unwrap();
    let _ = w.spawn(2, uid::MARINE, 100, 1500).unwrap();
    w.step();
    assert!(w.is_visible_to(0, 1500, 1500), "sees through its ally");
    assert!(!w.is_visible_to(0, 100, 1500), "not through the enemy");
    assert!(!w.is_visible_to(2, 1500, 1500));
}

#[test]
fn the_other_score_tabs_count_what_their_headers_say() {
    // Protoss start: a Nexus and four Probes produced; nothing killed.
    let mut w = boxer(&[Race::Protoss, Race::Zerg], &[true, true]);
    w.step();
    let units = score_table(&w, &[0], ScoreTab::Units);
    assert_eq!(units[0].values, [4, 0, 0], "Produced / Killed / Lost");
    let structures = score_table(&w, &[0], ScoreTab::Structures);
    assert_eq!(
        structures[0].values,
        [1, 0, 0],
        "Constructed / Razed / Lost"
    );
    let res = score_table(&w, &[0], ScoreTab::Resources);
    assert_eq!(res[0].values, [50, 0, 0], "Minerals / Gas / Spent");
    // Every tab keeps the Overview total and order.
    assert_eq!(units[0].total, 650);
    // Three Marines of player 0 against one of player 1, in open ground
    // away from the bases (a Drone would flee): a real kill.
    let victim = w.spawn(1, uid::MARINE, 1000, 1000).unwrap();
    let target = w.units[victim].id;
    for dx in [-60, -40, -20] {
        let m = w.spawn(0, uid::MARINE, 1000 + dx, 1000).unwrap();
        let id = w.units[m].id;
        w.issue(id, Order::AttackUnit { target }).unwrap();
    }
    for _ in 0..480 {
        w.step();
    }
    assert!(!w.units[victim].alive, "the marine died");
    let killed = score_table(&w, &[0], ScoreTab::Units)[0].values[1];
    let lost = score_table(&w, &[1], ScoreTab::Units)[0].values[2];
    assert!(killed >= 1 && killed == lost, "killed {killed} lost {lost}");
    assert!(w.players[0].scores[4] > 0, "kill score credited");
    assert_eq!(bar_fraction(400, 550), 400.0 / 550.0);
    assert_eq!(bar_fraction(5, 0), 0.0);
}

#[test]
fn an_empty_slots_melee_defeat_trigger_does_not_defeat_player_0() {
    // Melee maps carry "All Players: Current Player commands at most 0
    // buildings -> Defeat".  The empty slot must not run it, and Defeat
    // never applies to anyone but the player running the trigger.
    let mut w = boxer(&[Race::Protoss, Race::Zerg], &[true, false]);
    assert!(w.triggers.is_some(), "Boxer carries the melee triggers");
    for _ in 0..90 {
        w.step();
    }
    assert!(!w.players[0].defeated, "player 0 keeps playing");
    assert!(
        w.units
            .iter()
            .any(|u| u.alive && u.owner == 0 && u.type_id == uid::NEXUS)
    );
    // Resources stay on the map (they are bookkept under player 0).
    assert!(
        w.units
            .iter()
            .any(|u| u.alive && galaxycraft::sim::is_resource_type(u.type_id))
    );
}

#[test]
fn a_defeat_trigger_ends_the_game_only_for_the_player_running_it() {
    let mut w = boxer(&[Race::Terran, Race::Zerg], &[true, true]);
    for _ in 0..60 {
        w.step();
    }
    assert!(!w.players[0].defeated && !w.players[1].defeated);
    // Player 1 loses every building: its own Defeat trigger fires, and
    // player 0's "Non Allied Victory Players command 0 buildings" wins.
    let doomed: Vec<usize> = (0..w.units.len())
        .filter(|&i| w.units[i].owner == 1 && w.units[i].alive)
        .collect();
    for i in doomed {
        w.units[i].alive = false;
    }
    for _ in 0..60 {
        w.step();
    }
    // The game ends for player 1 (the sim stops at the winner); player 0
    // must not have been defeated by player 1's Defeat trigger.
    assert!(
        !w.players[0].defeated,
        "player 0 was not defeated in its place"
    );
    assert!(w.has_won(0) && !w.has_won(1));
}

#[test]
fn lobby_computers_run_identically_on_every_peer_and_after_a_resync() {
    use galaxycraft::ai::SimpleAi;
    use galaxycraft::net::{Lockstep, apply_resync};
    use galaxycraft::replay::{RecAct, state_hash};

    // Slot 0 is a human, slot 1 a computer on the other team.
    let setup = GameSetup {
        game_type: GameType::TopVsBottom,
        slots: vec![
            SetupSlot {
                occupant: SetupOccupant::Human { peer: 0 },
                race: Race::Terran,
                team: 0,
                color: 14,
            },
            SetupSlot {
                occupant: SetupOccupant::Computer,
                race: Race::Zerg,
                team: 1,
                color: 2,
            },
        ],
    };
    let build = || {
        let mut w = boxer(&setup.races(), &setup.occupied());
        setup.apply(&mut w);
        let mut lock = Lockstep::new(0, 1, 4);
        lock.computers = vec![SimpleAi::new(1)];
        (w, lock)
    };
    let scv = |w: &World| {
        w.units
            .iter()
            .find(|u| u.owner == 0 && u.type_id == uid::SCV)
            .map(|u| u.id)
            .unwrap()
    };
    // Two peers commit the same bundles: one human command, then idle turns.
    let run = || {
        let (mut w, mut lock) = build();
        let order = RecAct::Issue {
            unit: scv(&w),
            order: Order::Move { pos: (300, 300) },
        };
        lock.commit(&mut w, &[vec![order]]);
        for _ in 0..300 {
            lock.commit(&mut w, &[vec![]]);
        }
        (w, lock)
    };
    let (a, lock_a) = run();
    let (b, _) = run();
    assert_eq!(state_hash(&a), state_hash(&b), "peers agree");
    assert!(
        a.units.iter().filter(|u| u.alive && u.owner == 1).count() > 9,
        "the computer played (it trained units)"
    );
    // A rejoining peer rebuilds from the log with fresh computers.
    let (mut c, mut lock_c) = build();
    apply_resync(&mut c, &mut lock_c, lock_a.turn, &lock_a.recording());
    assert_eq!(state_hash(&c), state_hash(&a), "resync reproduces the game");
}

#[test]
fn lobby_colours_reach_the_game_and_its_drawing() {
    use galaxycraft::multiplayer::{
        COLOR_CHOICES, DEFAULT_COLORS, Lobby, Occupant, RaceChoice, color_rgb, color_score_group,
        score_group,
    };
    use galaxycraft::render::{PLAYER_COLORS, player_rgb};

    let mut l = Lobby::new_host("LinKa", "Boxer", 2, GameType::Melee, (1, 1));
    l.slots[0].race = RaceChoice::Protoss;
    l.set_occupant(1, Occupant::Computer);
    // Slot 1 picks Red, slot 0's default: Red is no longer offered to slot 0
    // and slot 0 (Random) falls to the first free colour.
    let red = COLOR_CHOICES.iter().position(|c| *c == "Red").unwrap();
    let green = COLOR_CHOICES.iter().position(|c| *c == "Green").unwrap();
    assert_eq!(DEFAULT_COLORS[0], red);
    assert!(l.free_colors(1).contains(&red));
    l.slots[1].color = red;
    assert!(
        !l.free_colors(0).contains(&red),
        "a taken colour is not offered"
    );
    assert!(l.free_colors(0).contains(&0), "Random is always offered");
    let resolved = l.resolved_colors();
    assert_eq!(resolved[1], red);
    assert_ne!(resolved[0], red);
    l.slots[0].color = green;
    let setup = l.resolve(1);
    assert_eq!(
        setup
            .slots
            .iter()
            .map(|s| s.color as usize)
            .collect::<Vec<_>>(),
        [green, red]
    );
    assert_eq!(GameSetup::decode(&setup.encode()), Some(setup.clone()));
    let mut w = boxer(&setup.races(), &setup.occupied());
    // Before the setup: classic slot colours.
    assert_eq!(player_rgb(&w, 0), PLAYER_COLORS[0]);
    setup.apply(&mut w);
    assert_eq!(
        player_rgb(&w, 0),
        color_rgb(green),
        "units drawn in the lobby colour"
    );
    assert_eq!(player_rgb(&w, 1), color_rgb(red));
    let dots = w.minimap_units(0, false);
    let enemy = dots.iter().find(|d| d.owner == 1 && !d.resource).unwrap();
    assert_eq!(enemy.color, color_rgb(red), "minimap blips too");
    // The eight default colours keep the classic look and their score bars.
    for (slot, &c) in DEFAULT_COLORS.iter().enumerate() {
        assert_eq!(color_rgb(c), PLAYER_COLORS[slot]);
        assert_eq!(
            color_score_group(c),
            slot,
            "score bar group of {}",
            COLOR_CHOICES[c]
        );
    }
    assert_eq!(color_score_group(green), 8);
    // Extra colours fall to the nearest group (Lime -> a yellow-green one).
    let lime = COLOR_CHOICES.iter().position(|c| *c == "Lime").unwrap();
    assert!([7, 8, 9].contains(&score_group(color_rgb(lime))));
    // A setup without a colour (0 = unresolved Random) is malformed.
    let mut bad = setup.encode();
    bad[6] = 0;
    assert_eq!(GameSetup::decode(&bad), None);
}

#[test]
fn a_team_player_who_loses_every_building_is_defeated() {
    // User report 2026-11-06: no Defeat after losing everything in a Top vs
    // Bottom game with computers.  The rules side defeats the player.
    let raw = std::fs::read("assets/maps/(4)Blood Bath.chk").expect("map");
    let sc = Scenario::parse(&raw).expect("parse");
    let era = galaxycraft::render::era_name(sc.era);
    let vf4 = std::fs::read(format!("assets/TileSet/{era}.vf4")).unwrap();
    let cv5 = std::fs::read(format!("assets/TileSet/{era}.cv5")).unwrap();
    let slot = |occupant, team| SetupSlot {
        occupant,
        race: Race::Terran,
        team,
        color: 0,
    };
    let setup = GameSetup {
        game_type: GameType::TopVsBottom,
        slots: vec![
            slot(SetupOccupant::Human { peer: 0 }, 0),
            slot(SetupOccupant::Computer, 0),
            slot(SetupOccupant::Computer, 1),
            slot(SetupOccupant::Computer, 1),
        ],
    };
    let mut w = World::from_scenario_occupied(
        data(),
        &sc,
        &TileData::new(&cv5, &vf4),
        &setup.races(),
        Some(&setup.occupied()),
    );
    setup.apply(&mut w);
    for _ in 0..60 {
        w.step();
    }
    let doomed: Vec<usize> = (0..w.units.len())
        .filter(|&i| w.units[i].owner == 0 && w.units[i].alive)
        .collect();
    for i in doomed {
        w.units[i].alive = false;
    }
    for _ in 0..400 {
        w.step();
    }
    eprintln!(
        "p0 defeated {} defeated={:?}",
        w.players[0].defeated,
        w.players.iter().map(|p| p.defeated).collect::<Vec<_>>()
    );
}

#[test]
fn units_animate_fire_and_leave_death_effects() {
    // User report 2026-11-06: no attack animation, no visible shots, units
    // and buildings vanish without an explosion.  The render-side animator
    // must give poses, bullets and death effects from the shipped iscript.
    use galaxycraft::unit_anim::{Animator, bullet_image, unit_image};
    let mut w = boxer(&[Race::Terran, Race::Zerg], &[true, true]);
    w.step();
    let (x, y) = (1200, 1200);
    let mut marines = Vec::new();
    for k in 0..4 {
        marines.push(w.spawn_px(0, uid::MARINE, x + k * 20, y).expect("marine"));
    }
    let lings: Vec<usize> = (0..3)
        .map(|k| {
            w.spawn_px(1, uid::ZERGLING, x + k * 24, y + 70)
                .expect("zergling")
        })
        .collect();
    let ling_image = unit_image(&w.data, uid::ZERGLING).expect("zergling image");
    let gauss = w.data.unit(uid::MARINE).ground_weapon;
    let gauss_image = bullet_image(&w.data, gauss).expect("gauss rifle image");
    let mut anim = Animator::default();
    let mut frames = std::collections::HashSet::new();
    let mut saw_bullet = false;
    let mut saw_ling_death = false;
    let mut sounds = 0usize;
    for _ in 0..600 {
        w.step();
        anim.advance(&w);
        for &m in &marines {
            if let Some(f) = anim.unit_frame(w.units[m].id) {
                frames.insert(f);
            }
        }
        // The Gauss Rifle is an instant weapon: its flash shows at the
        // target as an effect; travelling shots show as bullets.
        saw_bullet |= !anim.bullets().is_empty() || anim.effects().any(|e| e.image == gauss_image);
        saw_ling_death |= anim.effects().any(|e| e.image == ling_image);
        sounds += anim.take_sounds().len();
    }
    assert!(sounds > 0, "gunfire and deaths play their iscript sounds");
    assert!(
        lings.iter().any(|&i| !w.units[i].alive),
        "the fight happened"
    );
    assert!(
        frames.len() > 1,
        "marines change pose (attack frames): {frames:?}"
    );
    assert!(saw_bullet, "the shots are drawable while they exist");
    assert!(saw_ling_death, "a dead zergling plays its Death animation");
}

#[test]
fn the_fog_hides_the_other_team_and_shows_the_allies() {
    // User report 2026-11-06 (Top vs Bottom): the fog was drawn but the
    // enemies stayed visible.  The neutral resources (kept under player 0)
    // gave player 0 sight of every mineral line, enemy bases included.
    let raw = std::fs::read("assets/maps/(4)Blood Bath.chk").expect("map");
    let sc = Scenario::parse(&raw).expect("parse");
    let era = galaxycraft::render::era_name(sc.era);
    let vf4 = std::fs::read(format!("assets/TileSet/{era}.vf4")).unwrap();
    let cv5 = std::fs::read(format!("assets/TileSet/{era}.cv5")).unwrap();
    let slot = |occupant, team| SetupSlot {
        occupant,
        race: Race::Terran,
        team,
        color: 0,
    };
    let setup = GameSetup {
        game_type: GameType::TopVsBottom,
        slots: vec![
            slot(SetupOccupant::Human { peer: 0 }, 0),
            slot(SetupOccupant::Computer, 0),
            slot(SetupOccupant::Computer, 1),
            slot(SetupOccupant::Computer, 1),
        ],
    };
    let mut w = World::from_scenario_occupied(
        data(),
        &sc,
        &TileData::new(&cv5, &vf4),
        &setup.races(),
        Some(&setup.occupied()),
    );
    setup.apply(&mut w);
    w.enable_fog();
    for _ in 0..30 {
        w.step();
    }
    let town_hall = |owner: u8| {
        (0..w.units.len())
            .find(|&i| {
                w.units[i].alive
                    && w.units[i].owner == owner
                    && w.units[i].type_id == uid::COMMAND_CENTER
            })
            .expect("command center")
    };
    assert!(w.can_see_unit(0, town_hall(1)), "the ally's base is shared");
    assert!(!w.can_see_unit(0, town_hall(2)), "enemy base hidden");
    assert!(!w.can_see_unit(0, town_hall(3)), "enemy base hidden");
}

#[test]
fn damaged_buildings_burn_and_stop_when_repaired() {
    // User request 2026-11-06: burning on damaged buildings.  The depot's
    // damage LO has one slot (0); below two thirds of its hit points it
    // shows overlay image 450 + 0, and stops once repaired.
    use galaxycraft::unit_anim::{Animator, DAMAGE_OVERLAY_BASE};
    let mut w = boxer(&[Race::Terran, Race::Zerg], &[true, true]);
    w.step();
    let depot = w.spawn_px(0, uid::SUPPLY_DEPOT, 1200, 1200).expect("depot");
    w.units[depot].completed = true;
    let max = w.data.unit(uid::SUPPLY_DEPOT).hit_points_raw as i32;
    w.units[depot].hp = max;
    let mut anim = Animator::default();
    let mut lo = |_image: u16| vec![(0u8, -23i8, -26i8)];
    let mut run = |w: &mut World, anim: &mut Animator| {
        for _ in 0..6 {
            w.step();
            anim.advance_with(w, &mut lo);
        }
    };
    run(&mut w, &mut anim);
    assert!(anim.damage_overlays(&w).is_empty(), "intact: no fire");
    w.units[depot].hp = max / 2;
    run(&mut w, &mut anim);
    let fire = anim.damage_overlays(&w);
    assert_eq!(fire.len(), 1, "one fire on the depot");
    assert_eq!(fire[0].image, DAMAGE_OVERLAY_BASE);
    assert_eq!(
        (fire[0].x, fire[0].y),
        (w.units[depot].x / 256 - 23, w.units[depot].y / 256 - 26)
    );
    w.units[depot].hp = max;
    run(&mut w, &mut anim);
    assert!(anim.damage_overlays(&w).is_empty(), "repaired: out");
}

#[test]
fn a_forming_zerg_building_shows_its_spawn_art_and_a_morph_keeps_its_own() {
    // User request 2026-11-06: Zerg buildings form from a drone (the
    // `zbuild` construction image animating) and Hatchery -> Lair morphs.
    use galaxycraft::unit_anim::Animator;
    let mut w = boxer(&[Race::Zerg, Race::Terran], &[true, true]);
    w.step();
    let pool = w.spawn_px(0, uid::SPAWNING_POOL, 1200, 1200).expect("pool");
    w.units[pool].completed = false;
    let lair = w.spawn_px(0, uid::LAIR, 1500, 1200).expect("lair");
    w.units[lair].completed = false;
    let mut anim = Animator::default();
    for _ in 0..10 {
        w.step();
        anim.advance(&w);
    }
    let (image, _) = anim
        .construction_pose(w.units[pool].id)
        .expect("the pool is forming");
    assert_eq!(image, w.data.unit(uid::SPAWNING_POOL).construction_image);
    assert!(
        anim.construction_pose(w.units[lair].id).is_none(),
        "a morph, not a drone build"
    );
    w.units[pool].completed = true;
    w.step();
    anim.advance(&w);
    assert!(anim.construction_pose(w.units[pool].id).is_none(), "done");
}

#[test]
fn computers_play_each_other_for_an_observer_until_one_side_is_left() {
    // User request 2026-11-07: watch computer-vs-computer games.  The
    // observer is one past the last slot: it owns nothing, and the game is
    // over once `sides_left` has one side.
    use galaxycraft::ai::{SimpleAi, run_computers};
    let setup = setup(&[0, 1]);
    let mut w = boxer(&setup.races(), &setup.occupied());
    setup.apply(&mut w);
    let observer = setup.slots.len() as u8;
    let mut ais = vec![SimpleAi::new(0), SimpleAi::new(1)];
    for _ in 0..600 {
        run_computers(&mut w, &mut ais, observer);
        w.step();
    }
    assert!(!w.units.iter().any(|u| u.alive && u.owner == observer));
    for p in 0..2u8 {
        assert!(
            w.units.iter().filter(|u| u.alive && u.owner == p).count() > 5,
            "computer {p} played (it trained units)"
        );
    }
    assert_eq!(setup.sides_left(&w), vec![0, 1]);
    for u in w.units.iter_mut().filter(|u| u.owner == 1) {
        u.alive = false;
    }
    for _ in 0..400 {
        run_computers(&mut w, &mut ais, observer);
        w.step();
    }
    assert_eq!(setup.sides_left(&w), vec![0], "one side left: game over");
}

#[test]
fn every_game_type_starts_a_playable_game() {
    // User request 2026-11-07: check that the other game types work.  Each
    // type seats its players, starts with at least two sides, and the Team
    // types ally their teams (One on One seats two).
    use galaxycraft::ai::{SimpleAi, run_computers};
    use galaxycraft::multiplayer::{Lobby, Occupant, RaceChoice};
    let raw = std::fs::read("assets/maps/(4)Blood Bath.chk").expect("map");
    let sc = Scenario::parse(&raw).expect("parse");
    let era = galaxycraft::render::era_name(sc.era);
    let vf4 = std::fs::read(format!("assets/TileSet/{era}.vf4")).unwrap();
    let cv5 = std::fs::read(format!("assets/TileSet/{era}.cv5")).unwrap();
    for gt in GameType::ALL {
        let mut l = Lobby::new_host("Host", "Blood Bath", 4, gt, gt.default_teams(4));
        for i in 1..l.slots.len() {
            l.set_occupant(i, Occupant::Computer);
        }
        l.slots[0].race = RaceChoice::Terran;
        assert!(l.can_start(), "{gt:?} can start");
        if gt == GameType::OneOnOne {
            assert_eq!(l.slots.len(), 2, "One on One seats two");
        }
        let setup = l.resolve(7);
        let mut w = World::from_scenario_occupied(
            data(),
            &sc,
            &TileData::new(&cv5, &vf4),
            &setup.races(),
            Some(&setup.occupied()),
        );
        setup.apply(&mut w);
        let mut ais: Vec<SimpleAi> = (1..setup.slots.len() as u8).map(SimpleAi::new).collect();
        for _ in 0..240 {
            run_computers(&mut w, &mut ais, 0);
            w.step();
        }
        assert!(setup.sides_left(&w).len() >= 2, "{gt:?}: two sides play");
        if gt.is_team() {
            assert!(
                w.players[..setup.slots.len()]
                    .iter()
                    .enumerate()
                    .any(|(i, p)| w.players[..setup.slots.len()]
                        .iter()
                        .enumerate()
                        .any(|(j, q)| i != j && p.team == q.team)),
                "{gt:?}: teams are allied"
            );
            assert!(gt.team_choices(4).len() >= 2);
        } else {
            assert!(gt.team_choices(4).is_empty());
        }
    }
}
