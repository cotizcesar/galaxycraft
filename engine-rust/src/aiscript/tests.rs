use super::*;

#[test]
fn report_opcode_usage() {
    let ai = parse_aiscript(embedded!("fixtures/aiscript.bin"));
    let bw = parse_bwscript(embedded!("fixtures/bwscript.bin"));
    let mut counts = std::collections::BTreeMap::new();
    for set in [&ai, &bw] {
        for s in &set.scripts {
            for ins in s.instructions() {
                *counts.entry(ins.opcode).or_insert(0usize) += 1;
                if ins.opcode as usize >= OPCODE_NAMES.len() {
                    println!(
                        "unknown opcode {} in {} at {}",
                        ins.opcode,
                        s.id_str(),
                        ins.offset
                    );
                    let all = s.instructions();
                    if let Some(pos) = all.iter().position(|i| i.offset == ins.offset) {
                        for i in all[pos.saturating_sub(4)..=pos].iter() {
                            println!("   {} @{} {:?}", i.name(), i.offset, i.args);
                        }
                    }
                }
            }
        }
    }
    for (op, n) in &counts {
        println!(
            "{op:3} {:24} {n}",
            OPCODE_NAMES.get(*op as usize).copied().unwrap_or("unknown")
        );
    }
    assert!(counts.keys().all(|op| (*op as usize) < OPCODE_NAMES.len()));
}

#[test]
fn every_shipped_script_decodes_with_known_opcodes() {
    let ai = parse_aiscript(embedded!("fixtures/aiscript.bin"));
    let bw = parse_bwscript(embedded!("fixtures/bwscript.bin"));
    let mut total = 0usize;
    for set in [&ai, &bw] {
        for s in &set.scripts {
            for ins in s.instructions() {
                assert!(
                    (ins.opcode as usize) < OPCODE_NAMES.len(),
                    "unknown opcode {} in {} at {}",
                    ins.opcode,
                    s.id_str(),
                    ins.offset
                );
                total += 1;
            }
        }
    }
    assert!(
        total > 10_000,
        "the shipped scripts contain a large program"
    );
}

#[test]
fn melee_ai_runs_a_shipped_script_on_a_real_world() {
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    let ai_set = parse_aiscript(embedded!("fixtures/aiscript.bin"));
    let script = ai_set.get_str("TMCu").expect("Terran melee script");
    let mut ai = CampaignAi::new(0, script);
    let mut world = World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(64, 64),
        2,
    );
    world.players[0].minerals = 2000;
    world.spawn_px(0, uid::COMMAND_CENTER, 512, 512).unwrap();
    for k in 0..4 {
        world.spawn_px(0, uid::SCV, 560 + k * 24, 560).unwrap();
    }
    world.spawn_px(1, uid::COMMAND_CENTER, 1600, 1600).unwrap();
    for _ in 0..2400 {
        ai.update(&mut world, Some((50, 50)));
        world.step();
    }
    assert!(
        world
            .units
            .iter()
            .filter(|u| u.alive && u.owner == 0)
            .count()
            > 5,
        "the shipped melee AI builds or trains units"
    );
}

#[test]
fn parses_both_shipped_tables() {
    let ai = parse_aiscript(embedded!("fixtures/aiscript.bin"));
    let bw = parse_bwscript(embedded!("fixtures/bwscript.bin"));
    assert_eq!(ai.scripts.len(), 293);
    assert_eq!(bw.scripts.len(), 156);
    assert_eq!(ai.scripts.iter().filter(|s| s.external).count(), 156);
    assert!(ai.get_str("Ter3").is_some());
    assert!(bw.get_str("PB1A").is_some());
    assert!(ai.scripts.iter().all(|s| s.label > 0));
}

#[test]
fn campaign_interpreter_runs_build_and_train_commands() {
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    // start_town; train 1 SCV; stop
    let code = vec![3, 76, 1, 7, 0, 36];
    let script = AiScript {
        id: *b"Test",
        label: 1,
        flags: 0,
        code,
        external: false,
    };
    let mut ai = CampaignAi::new(0, &script);
    let mut world = World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(32, 32),
        2,
    );
    world.players[0].minerals = 500;
    world.spawn(0, uid::COMMAND_CENTER, 320, 320).unwrap();
    world.spawn(0, uid::SUPPLY_DEPOT, 420, 320).unwrap();
    for _ in 0..1200 {
        ai.update(&mut world, None);
        world.step();
    }
    assert!(
        world
            .units
            .iter()
            .any(|u| u.alive && u.owner == 0 && u.type_id == uid::SCV),
        "the AI script should train an SCV"
    );
}

#[test]
fn attack_do_sends_the_requested_count_and_no_periodic_attack() {
    // Desperate Alliance: `attack_add 2 Zergling` sent all 75 Zerglings
    // (plus Drones and Overlords), and an invented every-480-ticks
    // commit sent the whole army at tick 480.
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::{Order, Race, World};
    use std::sync::Arc;

    let mut world = World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(64, 64),
        2,
    );
    world.players[0].race = Race::Zerg;
    world.spawn_px(0, uid::HATCHERY, 320, 320).unwrap();
    for k in 0..10 {
        world.spawn_px(0, uid::ZERGLING, 420 + k * 20, 420).unwrap();
    }
    world.spawn_px(0, uid::DRONE, 300, 420).unwrap();
    world.spawn_px(0, uid::OVERLORD, 300, 460).unwrap();
    let attacking = |w: &World| {
        w.units
            .iter()
            .filter(|u| u.alive && u.owner == 0 && matches!(u.order, Order::AttackMove { .. }))
            .map(|u| u.type_id)
            .collect::<Vec<_>>()
    };
    // start_town; wait 600; stop: nothing may attack on its own.
    let idle = AiScript {
        id: *b"Idle",
        label: 1,
        flags: 0,
        code: vec![3, 2, 0x58, 0x02, 36],
        external: false,
    };
    let mut ai = CampaignAi::new(0, &idle);
    for _ in 0..600 {
        ai.update(&mut world, Some((1800, 1800)));
        world.step();
    }
    assert!(attacking(&world).is_empty(), "no attack without attack_do");
    // attack_add 2 Zergling; attack_do; stop.
    let wave = AiScript {
        id: *b"Wave",
        label: 1,
        flags: 0,
        code: vec![12, 2, 37, 0, 14, 36],
        external: false,
    };
    let mut ai = CampaignAi::new(0, &wave);
    ai.update(&mut world, Some((1800, 1800)));
    assert_eq!(attacking(&world), vec![uid::ZERGLING; 2]);
}

#[test]
fn set_attacks_budgets_target_expansion_launches() {
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::{Order, World};
    use std::sync::Arc;

    let data = Arc::new(GameData::bundled().unwrap());
    let mut world = World::new(data.clone(), Terrain::open(64, 64), 2);
    for k in 0..2 {
        world.spawn_px(0, uid::MARINE, 300 + k * 20, 300).unwrap();
    }
    world.spawn_px(1, uid::COMMAND_CENTER, 1200, 1200).unwrap();
    let attacking = |w: &World| {
        w.units
            .iter()
            .filter(|u| u.alive && u.owner == 0 && matches!(u.order, Order::AttackMove { .. }))
            .count()
    };
    let script = AiScript {
        id: *b"Atk1",
        label: 0,
        flags: 0,
        code: vec![79, 1, 77, 36], // set_attacks 1; target_expansion; stop
        external: false,
    };
    let mut ai = CampaignAi::new(0, &script);
    ai.update(&mut world, Some((40, 40)));
    assert_eq!(attacking(&world), 2);
    assert_eq!(ai.attacks_left, 0);

    let mut world = World::new(data, Terrain::open(64, 64), 2);
    world.spawn_px(0, uid::MARINE, 300, 300).unwrap();
    let mut ai = CampaignAi::new(
        0,
        &AiScript {
            id: *b"Atk2",
            label: 0,
            flags: 0,
            code: vec![77, 36], // no set_attacks budget
            external: false,
        },
    );
    ai.update(&mut world, Some((40, 40)));
    assert_eq!(attacking(&world), 0);
}

#[test]
fn move_dt_sends_dark_templar_to_the_trigger_location() {
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::{Order, World};
    use std::sync::Arc;

    let mut world = World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(64, 64),
        2,
    );
    let dt = world.spawn_px(0, uid::DARK_TEMPLAR, 200, 200).unwrap();
    let script = AiScript {
        id: *b"MvTe",
        label: 0,
        flags: 0,
        code: vec![38, 36], // move_dt; stop
        external: false,
    };
    let mut ai = CampaignAi::new(0, &script);
    ai.location = Some((768, 512));
    ai.update(&mut world, None);
    assert_eq!(world.units[dt].order, Order::Move { pos: (3, 2) });
}

#[test]
fn if_dif_compares_the_active_thread_difficulty() {
    use crate::data::GameData;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    let mut world = World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(32, 32),
        2,
    );
    // start_town; if_dif less-than 0 -> stop; otherwise wait 100; stop
    let script = AiScript {
        id: *b"Dif1",
        label: 0,
        flags: 0,
        code: vec![3, 95, 1, 0, 9, 0, 2, 100, 0, 36],
        external: false,
    };
    let mut ai = CampaignAi::new(0, &script);
    ai.update(&mut world, None);
    assert!(ai.finished, "active thread difficulty is 1, so 0 < 1");
}

#[test]
fn fake_nuke_resets_the_pending_nuke_state() {
    let script = AiScript {
        id: *b"Nuk1",
        label: 0,
        flags: 0,
        code: vec![102, 36],
        external: false,
    };
    let mut ai = CampaignAi::new(0, &script);
    ai.nukes = 5;
    let mut world = crate::sim::World::new(
        std::sync::Arc::new(crate::data::GameData::bundled().unwrap()),
        crate::map::Terrain::open(16, 16),
        2,
    );
    ai.update(&mut world, None);
    assert_eq!(ai.nukes, 0);
}

#[test]
fn region_state_commands_mark_the_retail_region_graph() {
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    let data = Arc::new(GameData::bundled().unwrap());
    let mut world = World::new(data.clone(), Terrain::open(64, 64), 2);
    world.spawn_px(0, uid::SCV, 20, 20).unwrap();
    let script = AiScript {
        id: *b"Rgn1",
        label: 0,
        flags: 0,
        code: vec![92, 36], // guard_all; stop
        external: false,
    };
    let mut ai = CampaignAi::new(0, &script);
    ai.update(&mut world, None);
    assert!(
        world.ai_region_states[0].contains(&5),
        "guard_all marks the unit region as a town"
    );

    // A single isolated tile has no ground neighbours: implode turns
    // its state 0 into a defended state 4.
    let mut terrain = Terrain::open(8, 8);
    for y in 0..32 {
        for x in 0..32 {
            if !(x / 4 == 2 && y / 4 == 2) {
                terrain.set_walkable_cell(x, y, false);
            }
        }
    }
    let mut world = World::new(data.clone(), terrain, 2);
    let mut ai = CampaignAi::new(
        0,
        &AiScript {
            id: *b"Rgn2",
            label: 0,
            flags: 0,
            code: vec![91, 36], // implode; stop
            external: false,
        },
    );
    ai.update(&mut world, None);
    assert!(
        world.ai_region_states[0].contains(&4),
        "implode marks low-connectivity regions as defend (regions {}, states {:?})",
        world.regions.count(),
        world.ai_region_states[0]
    );

    let mut world = World::new(data, Terrain::open(64, 64), 2);
    let mut ai = CampaignAi::new(
        0,
        &AiScript {
            id: *b"Rgn3",
            label: 0,
            flags: 0,
            code: vec![42, 36], // value_area; stop
            external: false,
        },
    );
    ai.location = Some((20 * 256, 20 * 256));
    ai.update(&mut world, None);
    assert!(
        world.ai_region_states[0].contains(&5),
        "value_area marks its trigger region as a town"
    );
}

#[test]
fn a_shipped_guard_all_script_marks_regions() {
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    let ai_set = parse_aiscript(embedded!("fixtures/aiscript.bin"));
    let bw_set = parse_bwscript(embedded!("fixtures/bwscript.bin"));
    let script = bw_set
        .get_str("TB8B")
        .or_else(|| ai_set.get_str("TB8B"))
        .expect("shipped guard_all script");
    let mut world = World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(64, 64),
        2,
    );
    world.spawn_px(0, uid::COMMAND_CENTER, 20, 20).unwrap();
    world.spawn_px(0, uid::SCV, 22, 22).unwrap();
    let mut ai = CampaignAi::new(0, script);
    for _ in 0..120 {
        ai.update(&mut world, None);
        world.step();
    }
    assert!(
        world.ai_region_states[0].contains(&5),
        "TB8B's guard_all should mark the town region"
    );
}

#[test]
fn help_iftrouble_marks_allied_regions() {
    use crate::data::GameData;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    let mut world = World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(64, 64),
        2,
    );
    world.players[1].team = 0;
    world.ai_region_states[0][0] = 5;
    let mut ai = CampaignAi::new(
        0,
        &AiScript {
            id: *b"Hlp1",
            label: 0,
            flags: 0,
            code: vec![110, 36], // help_iftrouble; stop
            external: false,
        },
    );
    ai.update(&mut world, None);
    assert_eq!(world.ai_region_states[1][0], 4);
}

#[test]
fn wait_finishattack_waits_for_the_attack_deadline() {
    use crate::data::GameData;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    let mut world = World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(64, 64),
        2,
    );
    world.ai_region_states[0][0] = 2;
    let mut ai = CampaignAi::new(
        0,
        &AiScript {
            id: *b"Wat1",
            label: 0,
            flags: 0,
            code: vec![99, 36], // wait_finishattack; stop
            external: false,
        },
    );
    ai.attack_finish = world.tick + 32;
    ai.update(&mut world, None);
    assert!(!ai.finished, "the attack deadline has not elapsed");
    for _ in 0..40 {
        world.step();
    }
    ai.update(&mut world, None);
    assert!(ai.finished, "the expired attack releases the thread");
    assert_eq!(world.ai_region_states[0][0], 0);
}

#[test]
fn quick_attack_sets_a_five_second_deadline() {
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    let mut world = World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(64, 64),
        2,
    );
    for k in 0..2 {
        world.spawn_px(0, uid::MARINE, 300 + k * 20, 300).unwrap();
    }
    world.spawn_px(1, uid::COMMAND_CENTER, 1200, 1200).unwrap();
    let mut ai = CampaignAi::new(
        0,
        &AiScript {
            id: *b"Qck1",
            label: 0,
            flags: 0,
            code: vec![100, 14, 36], // quick_attack; attack_do; stop
            external: false,
        },
    );
    ai.update(&mut world, Some((40, 40)));
    assert!(ai.attack_finish > 0);
    assert!(
        ai.attack_finish - world.tick <= 5 * crate::sim::TICKS_PER_SECOND as u64,
        "quick_attack caps the deadline at five seconds"
    );
}

#[test]
fn the_town_manager_sends_idle_workers_to_the_nearest_field() {
    let data = std::sync::Arc::new(crate::data::GameData::bundled().expect("tables"));
    let mut world = World::new(data, crate::map::Terrain::open(64, 64), 2);
    let mineral = world
        .spawn_px(0, crate::ids::unit::MINERAL_FIELD, 200, 200)
        .expect("field");
    world.units[mineral].resource_amount = 1000;
    let scv = world
        .spawn_px(0, crate::ids::unit::SCV, 300, 300)
        .expect("scv");
    let script = AiScript {
        id: *b"Tst1",
        label: 0,
        flags: 0,
        code: Vec::new(),
        external: false,
    };
    let ai = CampaignAi::new(0, &script);
    ai.keep_workers_mining(&mut world);
    assert!(matches!(
        world.units[scv].order,
        crate::sim::Order::Gather { resource } if resource == world.units[mineral].id
    ));
}

#[test]
fn decodes_known_instructions() {
    let ai = parse_aiscript(embedded!("fixtures/aiscript.bin"));
    let script = ai.get_str("Ter3").unwrap();
    let ins = script.instructions();
    assert!(!ins.is_empty());
    assert!(
        ins.iter()
            .any(|i| i.name() == "build" || i.name() == "train")
    );
    for i in &ins {
        assert!(
            i.opcode < OPCODE_NAMES.len() as u8,
            "bad opcode {} at {} in {}",
            i.opcode,
            i.offset,
            script.id_str()
        );
    }
}
