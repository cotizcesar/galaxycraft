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
//! hard it is to play.  `--episode` is any campaign key: `terran`, `zerg`,
//! `protoss`, `expterran`, `expzerg`, `expprotoss`.
//!
//! Debug output (stderr): `SOLVE_DUMP=1` every trigger and the starting
//! units, `SOLVE_WHY=1` the victory conditions left and the switch chain
//! behind them, `SOLVE_DEBUG=1` the goals and the player's units,
//! `SOLVE_TRIGGER=k` trigger k's conditions.

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
    // `SOLVE_IDLE=1`: no cheat and no solver; report when an enemy first
    // hurts one of the player's units (the computer's pace with the player
    // idle, to compare with the retail game).
    let idle = std::env::var_os("SOLVE_IDLE").is_some();
    if !idle {
        let _ = w.apply_cheat(human as u8, cheat::POWER_OVERWHELMING);
    }
    let health = |w: &World| -> i64 {
        w.units
            .iter()
            .filter(|u| {
                u.alive && u.owner as usize == human && !u.is_resource() && u.type_id != 101
            })
            .map(|u| (u.hp + u.shields) as i64)
            .sum()
    };
    let mut last_health = health(&w);
    let mut last_count = w
        .units
        .iter()
        .filter(|u| u.alive && u.owner as usize == human && !u.is_resource())
        .count();
    if std::env::var_os("SOLVE_DUMP").is_some()
        && let Some(t) = w.triggers.as_ref()
    {
        for (k, tr) in t.triggers.iter().enumerate() {
            let en: Vec<usize> = (0..28).filter(|&i| tr.enabled[i]).collect();
            let c: Vec<_> = tr
                .conditions
                .iter()
                .map(|c| {
                    (
                        c.kind,
                        c.group,
                        c.unit_id,
                        c.comparison,
                        c.quantity,
                        c.amount_modifier,
                    )
                })
                .collect();
            let a: Vec<_> = tr
                .actions
                .iter()
                .map(|a| (a.kind, a.group, a.group2, a.num, a.extra, a.location))
                .collect();
            eprintln!("T{k} {en:?} C{c:?} A{a:?}");
        }
        let mut by: std::collections::BTreeMap<u8, Vec<u16>> = Default::default();
        for u in w.units.iter().filter(|u| u.alive) {
            by.entry(u.owner).or_default().push(u.type_id);
        }
        for (k, tr) in t.triggers.iter().enumerate() {
            if tr.actions.iter().any(|a| a.kind == 2) {
                let h: Vec<bool> = tr
                    .conditions
                    .iter()
                    .map(|c| t.condition_holds(&w, human, c))
                    .collect();
                eprintln!("start defeat T{k}: {h:?}");
            }
        }
        eprintln!(
            "human P{} owners {:?} forces {:?} units {by:?}",
            human + 1,
            &scenario.owners[..8],
            scenario.forces
        );
    }
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
                // Also "brings at most / exactly 0 to a location": clear it.
                matches!(c.kind, 2 | 3)
                    && matches!(c.comparison, 1 | 10)
                    && c.quantity == 0
                    && !matches!(c.group as usize, 13)
                    && c.group as usize != human
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
                for c in tr.conditions.iter().filter(|c| {
                    c.kind == 3
                        && (matches!(c.comparison, 0 | 10) || (c.comparison == 1 && c.quantity > 0))
                        && about_me(c)
                }) {
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
                    && (c.comparison == 0
                        || (c.kind == 3
                            && (c.comparison == 10 || (c.comparison == 1 && c.quantity > 0))))
                    && !defeat
                    && (about_me(c) || matches!(c.group, 13 | 17) || (c.kind == 3 && c.group < 8))
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
            // As in sc_view: the human's town hall first.
            let enemy_base = w.enemy_town_hall(ai.player, human as u8);
            ai.update(&mut w, enemy_base);
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
            if std::env::var_os("SOLVE_DEBUG").is_some()
                && let Some(t) = w.triggers.as_ref()
            {
                for (k, tr) in t.triggers.iter().enumerate() {
                    if tr.actions.iter().any(|a| a.kind == 2) {
                        let held: Vec<_> = tr
                            .conditions
                            .iter()
                            .map(|c| {
                                (
                                    c.kind,
                                    c.group,
                                    c.unit_id,
                                    c.comparison,
                                    c.quantity,
                                    t.condition_holds(&w, human, c),
                                )
                            })
                            .collect();
                        eprintln!(
                            "defeat trigger {k} enabled {:?}: {held:?}",
                            tr.enabled
                                .iter()
                                .enumerate()
                                .filter(|e| *e.1)
                                .map(|e| e.0)
                                .collect::<Vec<_>>()
                        );
                    }
                }
            }
            return format!("DEFEAT at tick {} (human P{})", w.tick, human + 1);
        }
        for u in &w.units {
            seen.entry(u.id).or_insert(w.tick);
        }
        if idle {
            // Units and buildings only: resources the player owns are mined
            // out, and triggers remove units (Map Revealers, scripted
            // departures) without any attack.
            let mine = |u: &galaxycraft::sim::SimUnit| {
                u.owner as usize == human && !u.is_resource() && u.type_id != 101
            };
            let (now, count) = (
                health(&w),
                w.units.iter().filter(|u| u.alive && mine(u)).count(),
            );
            // Losing health with no fewer units (a new unit adds health,
            // a finished construction too): an attack.
            if now < last_health && count >= last_count {
                return format!(
                    "FIRST ATTACK at tick {} ({:.0} s at Fastest)",
                    w.tick,
                    w.tick as f64 / 23.81
                );
            }
            // A loss is a unit killed (no hit points left), not removed.
            if count < last_count
                && w.units
                    .iter()
                    .any(|u| !u.alive && mine(u) && u.hp <= 0 && u.last_damaged + 2 >= w.tick)
            {
                return format!(
                    "FIRST LOSS at tick {} ({:.0} s at Fastest)",
                    w.tick,
                    w.tick as f64 / 23.81
                );
            }
            (last_health, last_count) = (now, count);
            continue;
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
    if std::env::var_os("SOLVE_WHY").is_some()
        && let Some(t) = w.triggers.as_ref()
    {
        for (k, tr) in t.triggers.iter().enumerate() {
            if tr.actions.iter().any(|a| a.kind == 1) {
                let c: Vec<_> = tr
                    .conditions
                    .iter()
                    .map(|c| {
                        (
                            c.kind,
                            c.group,
                            c.unit_id,
                            c.comparison,
                            c.quantity,
                            c.amount_modifier,
                            c.location,
                            t.condition_holds(&w, human, c),
                        )
                    })
                    .collect();
                let en: Vec<usize> = (0..28).filter(|&i| tr.enabled[i]).collect();
                eprintln!("victory T{k} {en:?}: {c:?}");
                let mut want: Vec<u8> = tr
                    .conditions
                    .iter()
                    .filter(|c| c.kind == 11 && !t.condition_holds(&w, human, c))
                    .map(|c| c.amount_modifier)
                    .collect();
                let mut seen_sw: Vec<u8> = Vec::new();
                while let Some(sw) = want.pop() {
                    if seen_sw.contains(&sw) || seen_sw.len() > 12 {
                        continue;
                    }
                    seen_sw.push(sw);
                    for (j, sr) in t.triggers.iter().enumerate() {
                        if sr
                            .actions
                            .iter()
                            .any(|a| a.kind == 13 && a.group2 == sw as u32 && a.num == 4)
                        {
                            let c: Vec<_> = sr
                                .conditions
                                .iter()
                                .map(|c| {
                                    (
                                        c.kind,
                                        c.group,
                                        c.unit_id,
                                        c.comparison,
                                        c.quantity,
                                        c.amount_modifier,
                                        c.location,
                                        t.condition_holds(&w, human, c),
                                    )
                                })
                                .collect();
                            let en: Vec<usize> = (0..28).filter(|&i| sr.enabled[i]).collect();
                            eprintln!("  switch {sw} set by T{j} {en:?}: {c:?}");
                            want.extend(
                                sr.conditions
                                    .iter()
                                    .filter(|c| {
                                        c.kind == 11
                                            && c.comparison == 2
                                            && !t.condition_holds(&w, human, c)
                                    })
                                    .map(|c| c.amount_modifier),
                            );
                        }
                    }
                }
            }
        }
        let mut on = Vec::new();
        for i in 0..256 {
            if t.switches.get(i).copied().unwrap_or(false) {
                on.push(i);
            }
        }
        eprintln!(
            "switches {on:?} human P{} forces {:?}",
            human + 1,
            scenario.forces
        );
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

/// "Brings at most / exactly N": units past N inside the location walk
/// out, to where one of the player's units outside it stands.
fn trim_location(w: &mut World, mover: u8, c: &TriggerCondition) {
    let Some((l, t, r, b)) = w.triggers.as_ref().and_then(|tr| {
        tr.locations
            .get((c.location as usize).wrapping_sub(1))
            .copied()
    }) else {
        return;
    };
    let inside = |x: i32, y: i32| (l..r).contains(&(x / 256)) && (t..b).contains(&(y / 256));
    let Some((ox, oy)) = w
        .units
        .iter()
        .find(|u| u.alive && u.owner == mover && !u.is_resource() && !inside(u.x, u.y))
        .map(|u| (u.x, u.y))
    else {
        return;
    };
    let mut keep = c.quantity as usize;
    for i in 0..w.units.len() {
        let u = &w.units[i];
        if u.alive
            && u.owner == mover
            && !u.is_resource()
            && !w.data.unit(u.type_id).is_building()
            && matches(w, u.type_id, c.unit_id)
            && inside(u.x, u.y)
        {
            if keep > 0 {
                keep -= 1;
                continue;
            }
            w.units[i].x = ox;
            w.units[i].y = oy;
            w.units[i].path.clear();
        }
    }
}

/// Whether `(x, y)` (fixed point) is inside trigger location `loc`.
fn in_location(w: &World, loc: u32, x: i32, y: i32) -> bool {
    if loc == 0 || loc == 64 {
        return true;
    }
    w.triggers
        .as_ref()
        .and_then(|t| t.locations.get(loc as usize - 1).copied())
        .is_some_and(|(l, t, r, b)| (l..r).contains(&(x / 256)) && (t..b).contains(&(y / 256)))
}

/// Whether the trigger with exactly these conditions has run for good.
fn fired(w: &World, conds: &[TriggerCondition]) -> bool {
    let same = |a: &TriggerCondition, b: &TriggerCondition| {
        (
            a.kind,
            a.group,
            a.unit_id,
            a.quantity,
            a.location,
            a.comparison,
            a.amount_modifier,
        ) == (
            b.kind,
            b.group,
            b.unit_id,
            b.quantity,
            b.location,
            b.comparison,
            b.amount_modifier,
        )
    };
    w.triggers.as_ref().is_some_and(|t| {
        t.triggers.iter().enumerate().any(|(ti, tr)| {
            tr.conditions.len() == conds.len()
                && tr.conditions.iter().zip(conds).all(|(a, b)| same(a, b))
                && t.has_fired_for_good(ti)
        })
    })
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
    // Heroes a goal names are kept for it, not spent on "any unit" goals.
    let named: Vec<u16> = bring
        .iter()
        .map(|(c, _)| c.unit_id)
        .filter(|&t| t < 229)
        .collect();
    // The active stage first: conditions whose trigger needs a switch that
    // is now set get the units before older stages gated only by cleared
    // switches.
    // Goals whose trigger has run for good are done and free their units
    // (one Zeratul for four beacons in turn).
    let mut order: Vec<(usize, &(TriggerCondition, Vec<TriggerCondition>))> = bring
        .iter()
        .enumerate()
        .filter(|(_, (_, rest))| !fired(w, rest))
        .collect();
    if let Some(t) = w.triggers.as_ref() {
        order.sort_by_key(|(_, (c, rest))| {
            (
                std::cmp::Reverse(
                    rest.iter()
                        .filter(|r| {
                            r.kind == 11 && r.comparison == 2 && t.condition_holds(w, human, r)
                        })
                        .count(),
                ),
                t.condition_holds(w, human, c),
                c.unit_id >= 229,
            )
        });
    }
    for (_, (c, rest)) in order {
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
        } else if c.kind == 3 && (c.group as usize) < 8 {
            // A friendly seat, or a computer the player's own objective
            // waits on (Protoss BW 7: the enemy hero reaching a location).
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
            // Built for the player; a hero goes to the seat the objective
            // names (Protoss BW 3 and 7 wait on another player's hero).
            let owner = if w.data.unit(c.unit_id).is_hero() {
                mover
            } else {
                me
            };
            let have = w
                .units
                .iter()
                .filter(|u| u.alive && u.owner == owner && u.type_id == c.unit_id)
                .count() as u32;
            if have < c.quantity && !w.data.unit(c.unit_id).is_building() {
                for _ in have..c.quantity {
                    let _ = w.spawn_px(owner as usize, c.unit_id, x, y);
                }
            }
        }
        // At least N / exactly N / at most N (then only the extras leave).
        let mut left = match c.comparison {
            0 => c.quantity.max(1) as usize,
            10 => c.quantity as usize,
            _ => 0,
        };
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
                if c.unit_id >= 229
                    && (w.data.unit(u.type_id).is_building() || named.contains(&u.type_id))
                {
                    continue;
                }
                used.push(u.id);
                w.units[i].x = x * 256;
                w.units[i].y = y * 256;
                w.units[i].path.clear();
                left -= 1;
            }
        }
        if c.kind == 3 && matches!(c.comparison, 1 | 10) {
            trim_location(w, mover, c);
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
                        in_group(c.group, u.owner)
                            && matches(w, u.type_id, c.unit_id)
                            && (c.kind == 2 || in_location(w, c.location, u.x, u.y))
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
