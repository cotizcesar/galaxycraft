//! Drive campaign missions to their end through their own triggers.
//!
//! ```text
//! sc_solve --install "$SC_INSTALL" --episode terran [--mission terran05] [--ticks 120000]
//! ```
//!
//! Each mission runs in the real simulation (triggers, campaign AI scripts)
//! while a solver plays the human seat the blunt way: its units cannot be
//! hurt (`power overwhelming`), units that a "Bring" condition asks for are
//! moved into the location, rescuable units are rescued, and hostile
//! forces are destroyed (credited to the player, so "Kill" conditions
//! count), except the units a Defeat trigger says must survive.  Time runs
//! for "Elapsed" and countdown objectives.  The report says whether the
//! mission's Victory fired, its Defeat fired, or neither within the ticks:
//! a check that every mission's trigger chain can be completed, not of how
//! hard it is to play.

use std::sync::Arc;

use galaxycraft::aiscript::CampaignAi;
use galaxycraft::data::GameData;
use galaxycraft::map::{Scenario, TileTables};
use galaxycraft::retail::RetailInstall;
use galaxycraft::sim::{Race, World, cheat};
use galaxycraft::trigger::TriggerCondition;

const TILESETS: [&str; 8] = [
    "badlands.vf4",
    "platform.vf4",
    "install.vf4",
    "ashworld.vf4",
    "jungle.vf4",
    "Desert.vf4",
    "Ice.vf4",
    "Twilight.vf4",
];

/// Ticks before the solver starts destroying: the opening triggers
/// (transmissions, scripted moves) get to run first.
const WARMUP: u64 = 2_000;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let get = |k: &str| {
        args.iter()
            .position(|a| a == k)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let install = RetailInstall::open(std::path::Path::new(&get("--install").expect("--install")))
        .expect("install");
    let episode = get("--episode").unwrap_or_else(|| "terran".into());
    let only = get("--mission");
    let ticks: u64 = get("--ticks")
        .and_then(|t| t.parse().ok())
        .unwrap_or(120_000);
    let data = Arc::new(install.game_data().expect("retail tables"));
    let tiles: Vec<Option<TileTables>> = TILESETS
        .iter()
        .map(|name| {
            let cv5 = std::path::Path::new(name).with_extension("cv5");
            Some(TileTables {
                vf4: install.read(&format!("TileSet/{name}"))?,
                cv5: install.read(&format!("TileSet/{}", cv5.to_string_lossy()))?,
            })
        })
        .collect();
    let mut failed = 0;
    for ep in galaxycraft::campaign::episodes(&install) {
        if ep.key != episode {
            continue;
        }
        for m in ep.missions {
            if only.as_ref().is_some_and(|o| *o != m.key) {
                continue;
            }
            let Some(raw) = install.read(&m.chk_path) else {
                continue;
            };
            let outcome = solve(&data, &tiles, &raw, ticks);
            if !outcome.starts_with("VICTORY") {
                failed += 1;
            }
            println!("{:12} {}", m.key, outcome);
        }
    }
    std::process::exit(if failed > 0 { 1 } else { 0 });
}

fn solve(data: &Arc<GameData>, tiles: &[Option<TileTables>], raw: &[u8], ticks: u64) -> String {
    let Ok(scenario) = Scenario::parse(raw) else {
        return "FAIL parse".into();
    };
    let Some(Some(vf4)) = tiles.get(scenario.era as usize) else {
        return "FAIL tileset".into();
    };
    let slots = scenario.player_slots();
    let human = slots
        .iter()
        .find(|s| s.active && s.human)
        .map(|s| s.slot)
        .unwrap_or(0);
    let races: Vec<Race> = slots.iter().map(|s| s.race(Race::Terran)).collect();
    let mut w = World::from_scenario(data.clone(), &scenario, &vf4.data(), &races);
    for s in &slots {
        if let Some(p) = w.players.get_mut(s.slot) {
            p.defeated = !s.active;
            p.is_computer = s.computer;
        }
    }
    let _ = w.apply_cheat(human as u8, cheat::POWER_OVERWHELMING);
    let force = scenario.forces.get(human).copied().unwrap_or(0) as usize;
    let mut deaths: Vec<(TriggerCondition, Vec<TriggerCondition>)> = Vec::new();
    let (bring, protected, eliminate) = {
        let Some(t) = w.triggers.as_ref() else {
            return "FAIL no triggers".into();
        };
        let mine = |tr: &galaxycraft::trigger::Trigger| {
            tr.enabled.get(human).copied().unwrap_or(false)
                || tr.enabled.get(17).copied().unwrap_or(false)
                || tr.enabled.get(18 + force).copied().unwrap_or(false)
        };
        let mut bring: Vec<(TriggerCondition, Vec<TriggerCondition>)> = Vec::new();
        // "Player X commands at most 0 of Y" in any trigger: an objective
        // to destroy (Mengsk's command center belongs to a neutral seat).
        let eliminate: Vec<TriggerCondition> = t
            .triggers
            .iter()
            .filter(|tr| !tr.actions.iter().any(|a| a.kind == 2))
            .flat_map(|tr| tr.conditions.iter())
            .filter(|c| {
                c.kind == 2 && c.comparison == 1 && c.quantity == 0 && c.group as usize != human
            })
            .cloned()
            .collect();
        let mut protected: Vec<TriggerCondition> = Vec::new();
        // Conditions on the human's units count wherever they are: a
        // computer's trigger can wait for the player (Dylarian Shipyards).
        let about_me = |c: &TriggerCondition| {
            c.group as usize == human
                || c.group as usize == 18 + force
                || ((c.group as usize) < 8 && c.kind == 3 && !w.hostile(human as u8, c.group as u8))
        };
        for tr in &t.triggers {
            let defeat = tr.actions.iter().any(|a| a.kind == 2);
            if !defeat {
                for (k, c) in tr.conditions.iter().enumerate() {
                    let own_wipe = c.kind == 2 && c.comparison == 1 && c.quantity == 0;
                    if ((c.kind == 15 && c.comparison == 0) || own_wipe) && about_me(c) {
                        let rest: Vec<TriggerCondition> = tr
                            .conditions
                            .iter()
                            .enumerate()
                            .filter(|&(j, _)| j != k)
                            .map(|(_, c)| c.clone())
                            .collect();
                        deaths.push((c.clone(), rest));
                    }
                }
            }
            if !mine(tr) {
                for c in tr
                    .conditions
                    .iter()
                    .filter(|c| c.kind == 3 && c.comparison == 0 && about_me(c))
                {
                    if !defeat {
                        bring.push((c.clone(), tr.conditions.clone()));
                    }
                }
                continue;
            }
            for c in &tr.conditions {
                // "Bring" and "Command at least" (a build objective) and
                // "Accumulate": the solver provides them.
                if matches!(c.kind, 2..=4)
                    && c.comparison == 0
                    && !defeat
                    && (about_me(c) || matches!(c.group, 13 | 17))
                {
                    bring.push((c.clone(), tr.conditions.clone()));
                }
                if defeat && matches!(c.kind, 2 | 3 | 15 | 16) {
                    protected.push(c.clone());
                }
            }
        }
        (bring, protected, eliminate)
    };
    let mut campaign: Vec<CampaignAi> = Vec::new();
    // Tick each unit was first seen: the solver leaves a unit alone for a
    // while so the triggers can react to it (a strike force arriving).
    let mut seen: std::collections::HashMap<u32, u64> = std::collections::HashMap::new();
    while w.tick < ticks {
        for ai in &mut campaign {
            ai.update(&mut w, None);
        }
        w.step();
        if let Some(t) = w.triggers.as_mut() {
            for (player, id, at) in std::mem::take(&mut t.pending_ai_scripts) {
                if let Some(script) = data.campaign_ai_script(&id) {
                    let mut ai = CampaignAi::new(player as u8, script);
                    ai.location = at;
                    campaign.push(ai);
                }
            }
        }
        if w.winner == Some(human as u8) {
            return format!("VICTORY at tick {}", w.tick);
        }
        if w.players[human].defeated {
            return format!("DEFEAT at tick {}", w.tick);
        }
        for u in &w.units {
            seen.entry(u.id).or_insert(w.tick);
        }
        if w.tick.is_multiple_of(8) {
            step_solver(
                &seen,
                &mut w,
                human,
                &bring,
                &protected,
                &eliminate,
                &scenario.forces,
            );
            sacrifice(&mut w, human, &deaths);
        }
    }
    if std::env::var_os("SOLVE_DEBUG").is_some() {
        for (c, _) in &bring {
            let center = w
                .triggers
                .as_ref()
                .and_then(|t| t.location_center(c.location));
            let near = center.map(|(x, y)| {
                w.units
                    .iter()
                    .filter(|u| {
                        u.alive
                            && u.owner as usize == human
                            && (u.x / 256 - x).abs() < 64
                            && (u.y / 256 - y).abs() < 64
                    })
                    .count()
            });
            eprintln!(
                "cond kind {} group {} unit {} qty {} loc {} center {:?} human units near {:?}",
                c.kind, c.group, c.unit_id, c.quantity, c.location, center, near
            );
        }
        let alive = w
            .units
            .iter()
            .filter(|u| u.alive && u.owner as usize == human)
            .count();
        eprintln!(
            "human {human} alive units {alive} paused {:?}",
            w.triggers.as_ref().map(|t| t.paused)
        );
        let mut types: Vec<(String, usize)> = Vec::new();
        for u in w
            .units
            .iter()
            .filter(|u| u.alive && u.owner as usize == human)
        {
            let name = format!("{}", u.type_id);
            match types.iter_mut().find(|(n, _)| *n == name) {
                Some(t) => t.1 += 1,
                None => types.push((name, 1)),
            }
        }
        eprintln!("human unit types {types:?}");
        for u in w
            .units
            .iter()
            .filter(|u| u.alive && u.owner as usize == human)
        {
            eprintln!(
                "  unit type {} at ({}, {}) order {:?} in_transport {:?}",
                u.type_id,
                u.x / 256,
                u.y / 256,
                u.order,
                u.in_transport
            );
        }
        if let Some(t) = w.triggers.as_ref()
            && let Ok(k) = std::env::var("SOLVE_TRIGGER").map(|v| v.parse::<usize>().unwrap_or(0))
        {
            for c in &t.triggers[k].conditions {
                if c.kind != 0 {
                    for p in 0..8 {
                        if t.triggers[k].enabled[p] {
                            eprintln!(
                                "T{k} p{p} kind {} group {} unit {} loc {} -> {}",
                                c.kind,
                                c.group,
                                c.unit_id,
                                c.location,
                                t.condition_holds(&w, p, c)
                            );
                        }
                    }
                }
            }
        }
        if let Some(t) = w.triggers.as_ref() {
            let on: Vec<usize> = (0..256).filter(|&k| t.switches[k]).collect();
            eprintln!("switches on {on:?}");
        }
    }
    format!("TIMEOUT after {ticks} ticks")
}

/// Whether `type_id` matches a trigger unit selector (229 any unit, 230
/// men, 231 buildings, 232 factories).
fn matches(w: &World, type_id: u16, selector: u16) -> bool {
    let building = w.data.unit(type_id).is_building();
    match selector {
        229 => true,
        230 => !building,
        231 => building,
        232 => {
            use galaxycraft::ids::unit as uid;
            matches!(
                type_id,
                uid::BARRACKS | uid::FACTORY | uid::STARPORT | uid::GATEWAY
            )
        }
        s => s == type_id,
    }
}

fn step_solver(
    seen: &std::collections::HashMap<u32, u64>,
    w: &mut World,
    human: usize,
    bring: &[(TriggerCondition, Vec<TriggerCondition>)],
    protected: &[TriggerCondition],
    eliminate: &[TriggerCondition],
    forces: &[u8; 8],
) {
    // A trigger player group: a player, foes / all players, or a force.
    let in_group = |group: u32, owner: u8| match group {
        g if (g as usize) < 8 => g as u8 == owner,
        14 | 17 => true,
        18..=21 => forces
            .get(owner as usize)
            .is_some_and(|f| *f as u32 == group - 18),
        _ => false,
    };
    let me = human as u8;
    // Rescue every rescuable unit at once.
    for i in 0..w.units.len() {
        let o = w.units[i].owner as usize;
        if w.units[i].alive && w.players.get(o).is_some_and(|p| p.rescuable) {
            w.units[i].owner = me;
        }
    }
    // Bring: move the asked-for units into the location (each condition
    // gets its own units); Command: train the missing ones; Accumulate:
    // hand over the resources.
    let mut used: Vec<u32> = Vec::new();
    // The active stage first: conditions whose trigger needs a switch that
    // is now set get the units before older stages gated only by cleared
    // switches.
    let mut order: Vec<&(TriggerCondition, Vec<TriggerCondition>)> = bring.iter().collect();
    if let Some(t) = w.triggers.as_ref() {
        order.sort_by_key(|(_, rest)| {
            std::cmp::Reverse(
                rest.iter()
                    .filter(|r| r.kind == 11 && r.comparison == 2 && t.condition_holds(w, human, r))
                    .count(),
            )
        });
    }
    for (c, rest) in order {
        // A stage at a time: only once the trigger's gates (switches,
        // timers...) hold, so units go where the mission is now.
        let gated = w.triggers.as_ref().is_some_and(|t| {
            rest.iter()
                .filter(|r| r.kind != 0 && !matches!(r.kind, 2 | 3 | 4 | 15))
                .all(|r| t.condition_holds(w, human, r))
        });
        if !gated {
            continue;
        }
        // Whose units: the player's, or a friendly seat a condition names.
        let mover = if c.group == 13 || (c.group != 14 && in_group(c.group, me)) {
            me
        } else if c.kind == 3 && (c.group as usize) < 8 && !w.hostile(me, c.group as u8) {
            c.group as u8
        } else {
            continue;
        };
        if c.kind == 4 {
            let p = &mut w.players[human];
            let need = c.quantity as i32;
            match c.amount_modifier {
                0 => p.minerals_gathered = p.minerals_gathered.max(need),
                1 => p.gas_gathered = p.gas_gathered.max(need),
                _ => p.minerals_gathered = p.minerals_gathered.max(need),
            }
            continue;
        }
        if c.kind == 2 {
            if c.unit_id >= 229 || c.location != 0 {
                continue;
            }
            let have = w
                .units
                .iter()
                .filter(|u| u.alive && u.owner == me && u.type_id == c.unit_id && u.completed)
                .count() as u32;
            let at = w
                .units
                .iter()
                .find(|u| u.alive && u.owner == me && !u.is_resource())
                .map(|u| (u.x / 256, u.y / 256));
            if let Some((x, y)) = at {
                for _ in have..c.quantity {
                    let _ = w.spawn_px(human, c.unit_id, x + 96, y + 96);
                }
            }
            continue;
        }
        let Some((x, y)) = w
            .triggers
            .as_ref()
            .and_then(|t| t.location_center(c.location))
        else {
            continue;
        };
        // A specific unit the player does not have enough of is built first
        // (dropships for Norad II).
        if c.unit_id < 229 {
            let have = w
                .units
                .iter()
                .filter(|u| u.alive && u.owner == me && u.type_id == c.unit_id)
                .count() as u32;
            if have < c.quantity && !w.data.unit(c.unit_id).is_building() {
                for _ in have..c.quantity {
                    let _ = w.spawn_px(human, c.unit_id, x, y);
                }
            }
        }
        let mut left = c.quantity.max(1) as usize;
        for i in 0..w.units.len() {
            if left == 0 {
                break;
            }
            let u = &w.units[i];
            if u.alive
                && u.owner == mover
                && !u.is_resource()
                && !used.contains(&u.id)
                && matches(w, u.type_id, c.unit_id)
            {
                if w.data.unit(u.type_id).is_building() && c.unit_id >= 229 {
                    continue;
                }
                used.push(u.id);
                w.units[i].x = x * 256;
                w.units[i].y = y * 256;
                w.units[i].path.clear();
                left -= 1;
            }
        }
    }
    if w.tick < WARMUP {
        return;
    }
    // Destroy hostile forces, sparing what a Defeat trigger protects.
    let Some(credit) = w
        .units
        .iter()
        .find(|u| u.alive && u.owner == me)
        .map(|u| u.id)
    else {
        return;
    };
    let spared = |w: &World, owner: u8, type_id: u16| {
        protected.iter().any(|c| {
            let g = c.group as usize;
            (g == owner as usize || g == 17 || g == 14) && matches(w, type_id, c.unit_id)
        })
    };
    let victims: Vec<u32> = w
        .units
        .iter()
        .filter(|u| {
            u.alive
                && !u.is_resource()
                && (u.owner as usize) < 8
                && u.owner != me
                && (w.hostile(me, u.owner)
                    || eliminate.iter().any(|c| {
                        in_group(c.group, u.owner) && c.unit_id < 229 && c.unit_id == u.type_id
                    }))
                && !spared(w, u.owner, u.type_id)
                && seen.get(&u.id).is_some_and(|&t| w.tick >= t + 600)
        })
        .map(|u| u.id)
        .collect();
    for id in victims {
        if let Some(i) = w.index_of(id) {
            w.units[i].last_damage_source = credit;
            w.units[i].invincible = false;
            let hp = w.units[i].hp + w.units[i].shields + 1;
            let _ = w.damage_unit(id, hp);
        }
    }
}

/// "Deaths" conditions on the player's units: sacrifice the asked-for unit
/// once the rest of that trigger's conditions hold (a pilot boarding a
/// stolen battlecruiser in The Dylarian Shipyards).
fn sacrifice(w: &mut World, human: usize, deaths: &[(TriggerCondition, Vec<TriggerCondition>)]) {
    for (c, rest) in deaths {
        if c.unit_id >= 229 {
            continue;
        }
        let ready = w.triggers.as_ref().is_some_and(|t| {
            rest.iter().all(|r| t.condition_holds(w, human, r)) && !t.condition_holds(w, human, c)
        });
        if !ready {
            continue;
        }
        let me = human as u8;
        // "Commands at most 0": all of them go; "Deaths": one at a time.
        let ids: Vec<u32> = w
            .units
            .iter()
            .filter(|u| u.alive && u.owner == me && u.type_id == c.unit_id)
            .map(|u| u.id)
            .take(if c.kind == 2 { usize::MAX } else { 1 })
            .collect();
        for id in ids {
            w.players[human].cheats.invincible = false;
            let hp = w.unit(id).map_or(0, |u| u.hp + u.shields + 1);
            let _ = w.damage_unit(id, hp);
            w.players[human].cheats.invincible = true;
        }
    }
}
