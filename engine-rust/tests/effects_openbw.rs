//! Spell effects against OpenBW (`bwgame.h`) and the retail executable: each
//! spell is cast in the simulation and the timer, hit points or lifetime it
//! leaves are compared with the number OpenBW applies.  Status timers count
//! 8-frame updates in both engines.

use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::Terrain;
use galaxycraft::sim::{
    BROODLING_LIFETIME_TICKS, ENERGY_SCALE, HALLUCINATION_LIFETIME_TICKS, Spell, SpellTarget, World,
};
use std::sync::Arc;

fn world() -> World {
    let mut w = World::new(
        Arc::new(GameData::bundled().unwrap()),
        Terrain::open(64, 64),
        2,
    );
    for p in 0..2 {
        for t in 0..44 {
            w.players[p].techs[t] = true;
        }
    }
    w
}

/// Cast `spell` from a fresh `caster` (player 0, full energy) on a `target`
/// unit of `target_owner`; step up to 200 frames, returning the world and the
/// target's index.
fn cast_on(caster: u16, spell: Spell, target: u16, target_owner: usize) -> (World, usize, usize) {
    let mut w = world();
    let c = w.spawn_px(0, caster, 400, 400).unwrap();
    let t = w.spawn_px(target_owner, target, 470, 400).unwrap();
    w.units[c].set_energy_fixed(250 * ENERGY_SCALE);
    let tid = w.units[t].id;
    w.cast(w.units[c].id, spell, SpellTarget::Unit(tid))
        .unwrap_or_else(|e| panic!("{spell:?}: {e:?}"));
    (w, c, t)
}

/// Largest value `read` takes on the target while stepping 200 frames.
fn peak(w: &mut World, t: usize, read: impl Fn(&World, usize) -> i32) -> i32 {
    let mut best = read(w, t);
    for _ in 0..200 {
        w.step();
        best = best.max(read(w, t));
    }
    best
}

#[test]
fn status_spell_durations_match_openbw() {
    // (caster, spell, target, owner, OpenBW timer, field)
    let lockdown = |w: &World, t: usize| w.units[t].status.lockdown_ticks;
    let stasis = |w: &World, t: usize| w.units[t].status.stasis_ticks;
    let ensnare = |w: &World, t: usize| w.units[t].status.ensnare_ticks;
    let plague = |w: &World, t: usize| w.units[t].status.plague_ticks;
    let maelstrom = |w: &World, t: usize| w.units[t].status.maelstrom_ticks;
    // OpenBW: lockdown_timer 131, stasis_timer 131, ensnare_timer 75,
    // plague_timer 75, maelstrom_timer 22.
    let (mut w, _, t) = cast_on(uid::GHOST, Spell::Lockdown, uid::SIEGE_TANK, 1);
    assert_eq!(peak(&mut w, t, lockdown), 131, "Lockdown");
    let mut w = world();
    let c = w.spawn_px(0, uid::ARBITER, 400, 400).unwrap();
    let t = w.spawn_px(1, uid::MARINE, 470, 400).unwrap();
    w.units[c].set_energy_fixed(250 * ENERGY_SCALE);
    w.cast(
        w.units[c].id,
        Spell::StasisField,
        SpellTarget::Point(470, 400),
    )
    .unwrap();
    assert_eq!(peak(&mut w, t, stasis), 131, "Stasis Field");
    let mut w = world();
    let c = w.spawn_px(0, uid::QUEEN, 400, 400).unwrap();
    let t = w.spawn_px(1, uid::MARINE, 470, 400).unwrap();
    w.units[c].set_energy_fixed(250 * ENERGY_SCALE);
    w.cast(w.units[c].id, Spell::Ensnare, SpellTarget::Point(470, 400))
        .unwrap();
    assert_eq!(peak(&mut w, t, ensnare), 75, "Ensnare");
    let mut w = world();
    let c = w.spawn_px(0, uid::DEFILER, 400, 400).unwrap();
    let t = w.spawn_px(1, uid::MARINE, 470, 400).unwrap();
    w.units[c].set_energy_fixed(250 * ENERGY_SCALE);
    w.cast(w.units[c].id, Spell::Plague, SpellTarget::Point(470, 400))
        .unwrap();
    assert_eq!(peak(&mut w, t, plague), 75, "Plague");
    let (mut w, _, t) = cast_on(uid::DARK_ARCHON, Spell::Maelstrom, uid::ZERGLING, 1);
    assert_eq!(peak(&mut w, t, maelstrom), 22, "Maelstrom");
}

#[test]
fn defensive_matrix_and_irradiate_match_openbw() {
    // OpenBW `order_DefensiveMatrix`: 250 hit points, timer 168.
    let (mut w, _, t) = cast_on(uid::SCIENCE_VESSEL, Spell::DefensiveMatrix, uid::MARINE, 0);
    let hp = peak(&mut w, t, |w, t| w.units[t].status.matrix);
    assert_eq!(hp, 250 << 8, "matrix hit points");
    let (mut w, _, t) = cast_on(uid::SCIENCE_VESSEL, Spell::DefensiveMatrix, uid::MARINE, 0);
    let ticks = peak(&mut w, t, |w, t| w.units[t].status.matrix_ticks as i32);
    assert_eq!(ticks, 168, "matrix timer");
    // OpenBW `irradiate_unit`: the timer is the Irradiate weapon's cooldown.
    let (mut w, _, t) = cast_on(uid::SCIENCE_VESSEL, Spell::Irradiate, uid::ZERGLING, 1);
    let want = w
        .data
        .weapon(galaxycraft::ids::weapon::IRRADIATE)
        .unwrap()
        .cooldown as i32;
    assert_eq!(
        peak(&mut w, t, |w, t| w.units[t].status.irradiate_ticks),
        want,
        "Irradiate"
    );
}

#[test]
fn hallucinations_and_broodlings_live_as_long_as_in_retail() {
    // OpenBW `order_CastHallucination`: two copies owned by the caster,
    // `default_remove_timer` 1350; retail `0x005cc810` returns 0x546.
    let (w, _, t) = cast_on(uid::HIGH_TEMPLAR, Spell::Hallucination, uid::ZERGLING, 1);
    let copies: Vec<_> = w
        .units
        .iter()
        .filter(|u| u.alive && u.status.hallucination)
        .collect();
    assert_eq!(copies.len(), 2, "two hallucinations");
    assert!(
        copies
            .iter()
            .all(|u| u.owner == 0 && u.type_id == w.units[t].type_id)
    );
    assert!(
        copies
            .iter()
            .all(|u| u.expires_at == w.tick as i64 + HALLUCINATION_LIFETIME_TICKS)
    );
    assert_eq!(HALLUCINATION_LIFETIME_TICKS, 1350);
    // Broodlings: 0x708 = 1800 frames (OpenBW `default_remove_timer`).
    assert_eq!(BROODLING_LIFETIME_TICKS, 1800);
    let (mut w, _, _) = cast_on(uid::QUEEN, Spell::SpawnBroodlings, uid::MARINE, 1);
    let mut lifetime = None;
    for _ in 0..200 {
        w.step();
        if let Some(b) = w
            .units
            .iter()
            .find(|u| u.alive && u.type_id == uid::BROODLING)
        {
            lifetime = Some(b.expires_at - w.tick as i64);
            break;
        }
    }
    let l = lifetime.expect("broodlings spawned");
    assert!((1790..=1800).contains(&l), "broodling lifetime {l}");
}

#[test]
fn dark_swarm_and_disruption_web_last_as_in_openbw() {
    // OpenBW `dark_swarm`: set_remove_timer 900; `disruption_web`: 360.
    for (caster, spell, unit, want) in [
        (uid::DEFILER, Spell::DarkSwarm, uid::DARK_SWARM, 900),
        (uid::CORSAIR, Spell::DisruptionWeb, uid::DISRUPTION_WEB, 360),
    ] {
        let mut w = world();
        let c = w.spawn_px(0, caster, 400, 400).unwrap();
        w.units[c].set_energy_fixed(250 * ENERGY_SCALE);
        w.cast(w.units[c].id, spell, SpellTarget::Point(470, 400))
            .unwrap();
        let mut left = None;
        for _ in 0..200 {
            w.step();
            if let Some(s) = w.units.iter().find(|u| u.alive && u.type_id == unit) {
                left = Some(s.expires_at - w.tick as i64);
                break;
            }
        }
        let l = left.unwrap_or_else(|| panic!("{spell:?} spawned nothing"));
        assert!(
            (want - 10..=want).contains(&l),
            "{spell:?}: {l} frames left, want {want}"
        );
    }
}

#[test]
fn plague_and_emp_change_hit_points_shields_and_energy_as_in_openbw() {
    // OpenBW: each 8-frame update Plague deals the weapon's damage / 76 and
    // never kills (`if (u->hp > damage)`).
    let mut w = world();
    let c = w.spawn_px(0, uid::DEFILER, 400, 400).unwrap();
    // A target that does not regenerate (Zerg units heal between hits).
    let t = w.spawn_px(1, uid::SIEGE_TANK, 470, 400).unwrap();
    w.units[c].set_energy_fixed(250 * ENERGY_SCALE);
    w.cast(w.units[c].id, Spell::Plague, SpellTarget::Point(470, 400))
        .unwrap();
    let dmg = (w
        .data
        .weapon(galaxycraft::ids::weapon::PLAGUE)
        .unwrap()
        .damage as i32)
        * 256
        / 76;
    let mut drops = Vec::new();
    let mut hp = w.units[t].hp;
    for _ in 0..200 {
        w.step();
        if w.units[t].hp < hp {
            drops.push(hp - w.units[t].hp);
        }
        hp = w.units[t].hp;
    }
    assert!(
        drops.len() > 10 && drops.iter().all(|&d| d == dmg),
        "plague hits {drops:?}, want {dmg}"
    );
    // OpenBW `emp_shockwave`: energy and shields to 0 (hallucinations die).
    let mut w = world();
    let c = w.spawn_px(0, uid::SCIENCE_VESSEL, 400, 400).unwrap();
    let t = w.spawn_px(1, uid::HIGH_TEMPLAR, 470, 400).unwrap();
    w.units[c].set_energy_fixed(250 * ENERGY_SCALE);
    w.units[t].set_energy_fixed(200 * ENERGY_SCALE);
    w.cast(
        w.units[c].id,
        Spell::EmpShockwave,
        SpellTarget::Point(470, 400),
    )
    .unwrap();
    // Both recharge afterwards: look at the lowest values reached.
    let (mut energy, mut shields) = (i32::MAX, i32::MAX);
    for _ in 0..200 {
        w.step();
        energy = energy.min(w.units[t].energy_fixed());
        shields = shields.min(w.units[t].shields);
    }
    assert_eq!(energy, 0, "EMP energy");
    assert_eq!(shields, 0, "EMP shields");
}

#[test]
fn feedback_consume_and_restoration_match_openbw() {
    // OpenBW `order_Feedback`: damage equal to the target's energy, then 0.
    let mut w = world();
    let c = w.spawn_px(0, uid::DARK_ARCHON, 400, 400).unwrap();
    let t = w.spawn_px(1, uid::SCIENCE_VESSEL, 470, 400).unwrap();
    w.units[c].set_energy_fixed(250 * ENERGY_SCALE);
    w.units[t].set_energy_fixed(100 * ENERGY_SCALE);
    let hp = w.units[t].hp;
    w.cast(
        w.units[c].id,
        Spell::Feedback,
        SpellTarget::Unit(w.units[t].id),
    )
    .unwrap();
    let mut energy = i32::MAX;
    for _ in 0..100 {
        w.step();
        energy = energy.min(w.units[t].energy_fixed());
    }
    assert_eq!(hp - w.units[t].hp, 100 << 8, "Feedback damage = energy");
    // Emptied; one frame of recharge (8/256) can land in the same step.
    assert!(energy <= 8, "Feedback empties the energy: {energy}");
    // OpenBW `consume`: the own Zerg target dies, the Defiler gains 50.
    let mut w = world();
    let c = w.spawn_px(0, uid::DEFILER, 400, 400).unwrap();
    // Within the Consume weapon's 16 px (`cast` does not walk).
    let t = w.spawn_px(0, uid::ZERGLING, 410, 400).unwrap();
    w.units[c].set_energy_fixed(100 * ENERGY_SCALE);
    w.cast(
        w.units[c].id,
        Spell::Consume,
        SpellTarget::Unit(w.units[t].id),
    )
    .unwrap();
    for _ in 0..100 {
        w.step();
    }
    assert!(!w.units[t].alive, "consumed Zergling dies");
    let e = w.units[c].energy_fixed() / ENERGY_SCALE;
    assert!(
        (150..=154).contains(&e),
        "Defiler energy {e}, want 100 + 50 (+ ~3 regenerated)"
    );
    // OpenBW `restore_unit`: Ensnare, Plague, Irradiate, Lockdown,
    // Maelstrom, blindness and parasite are removed.
    let mut w = world();
    let c = w.spawn_px(0, uid::MEDIC, 400, 400).unwrap();
    let t = w.spawn_px(0, uid::MARINE, 450, 400).unwrap();
    w.units[c].set_energy_fixed(250 * ENERGY_SCALE);
    {
        let s = &mut w.units[t].status;
        s.ensnare_ticks = 50;
        s.plague_ticks = 50;
        s.lockdown_ticks = 50;
        s.maelstrom_ticks = 10;
        s.optical_flare_ticks = 100;
    }
    w.cast(
        w.units[c].id,
        Spell::Restoration,
        SpellTarget::Unit(w.units[t].id),
    )
    .unwrap();
    for _ in 0..60 {
        w.step();
    }
    let s = &w.units[t].status;
    assert_eq!(
        (
            s.ensnare_ticks,
            s.plague_ticks,
            s.lockdown_ticks,
            s.maelstrom_ticks,
            s.optical_flare_ticks
        ),
        (0, 0, 0, 0, 0),
        "Restoration clears every effect"
    );
}

#[test]
fn psionic_storm_deals_eight_hits_of_its_weapon() {
    let mut w = world();
    let c = w.spawn_px(0, uid::HIGH_TEMPLAR, 400, 400).unwrap();
    let t = w.spawn_px(1, uid::SIEGE_TANK, 470, 400).unwrap();
    w.units[c].set_energy_fixed(250 * ENERGY_SCALE);
    let hp = w.units[t].hp;
    w.cast(
        w.units[c].id,
        Spell::PsionicStorm,
        SpellTarget::Point(470, 400),
    )
    .unwrap();
    for _ in 0..300 {
        w.step();
    }
    // Eight hits of the weapon's 14 (one storm at a time per unit, OpenBW
    // `storm_timer`): 112.
    let per_hit = w
        .data
        .weapon(galaxycraft::ids::weapon::PSIONIC_STORM)
        .unwrap()
        .damage as i32;
    assert_eq!(hp - w.units[t].hp, (8 * per_hit) << 8, "storm total");
}

#[test]
fn mind_control_optical_flare_and_parasite_match_openbw() {
    // OpenBW `order_MindControl`: the target changes owner, the Dark
    // Archon's shields go to 0.
    let (mut w, c, t) = cast_on(uid::DARK_ARCHON, Spell::MindControl, uid::MARINE, 1);
    for _ in 0..60 {
        w.step();
    }
    assert_eq!(w.units[t].owner, 0, "mind-controlled unit changes owner");
    assert!(
        w.units[c].shields < 2 << 8,
        "caster shields emptied: {}",
        w.units[c].shields
    );
    // OpenBW `blind_unit`: the target is blinded.
    let (mut w, _, t) = cast_on(uid::MEDIC, Spell::OpticalFlare, uid::MARINE, 1);
    for _ in 0..60 {
        w.step();
    }
    assert!(
        w.units[t].status.optical_flare_ticks > 0,
        "Optical Flare blinds"
    );
    // OpenBW `parasite_flags |= 1 << owner`.
    let (mut w, _, t) = cast_on(uid::QUEEN, Spell::Parasite, uid::MARINE, 1);
    for _ in 0..120 {
        w.step();
    }
    assert!(w.units[t].status.parasite, "Parasite marks the target");
}

#[test]
fn recall_brings_own_units_within_64_px_of_the_point() {
    // OpenBW `order_Recall`: after 22 frames, the caster's own units within
    // a 64 px square of the point (not buildings, hallucinations or burrowed
    // units) move to the Arbiter.
    let mut w = world();
    let a = w.spawn_px(0, uid::ARBITER, 300, 300).unwrap();
    let near = w.spawn_px(0, uid::MARINE, 1200, 1200).unwrap();
    let far = w.spawn_px(0, uid::MARINE, 1200, 1400).unwrap();
    let enemy = w.spawn_px(1, uid::MARINE, 1230, 1200).unwrap();
    w.units[a].set_energy_fixed(250 * ENERGY_SCALE);
    w.cast(w.units[a].id, Spell::Recall, SpellTarget::Point(1200, 1200))
        .unwrap();
    for _ in 0..60 {
        w.step();
    }
    let d = |w: &World, i: usize| {
        ((w.units[i].x - w.units[a].x).abs() + (w.units[i].y - w.units[a].y).abs()) / 256
    };
    assert!(
        d(&w, near) < 120,
        "own unit in the field is recalled ({} px)",
        d(&w, near)
    );
    assert!(d(&w, far) > 800, "a unit outside the 64 px square stays");
    assert!(d(&w, enemy) > 800, "enemy units are not recalled");
}

#[test]
fn hallucinations_take_double_damage() {
    // OpenBW `weapon_deal_damage`: `if (u_hallucination(target)) damage *= 2`.
    let hit = |halluc: bool| {
        let mut w = world();
        let m = w.spawn_px(0, uid::MARINE, 400, 400).unwrap();
        let t = w.spawn_px(1, uid::ULTRALISK, 470, 400).unwrap();
        w.units[t].status.hallucination = halluc;
        w.units[t].order = galaxycraft::sim::Order::HoldPosition;
        let (mid, tid) = (w.units[m].id, w.units[t].id);
        w.issue(mid, galaxycraft::sim::Order::AttackUnit { target: tid })
            .unwrap();
        let hp = w.units[t].hp;
        for _ in 0..60 {
            w.step();
            if w.units[t].hp < hp {
                return hp - w.units[t].hp;
            }
        }
        0
    };
    let (real, fake) = (hit(false), hit(true));
    // Doubled before the armour comes off: Gauss Rifle 6 against the
    // Ultralisk's 1 armour is 5 real, (2 * 6) - 1 = 11 on a hallucination.
    assert!(
        real > 0 && fake == 2 * real + 256,
        "hallucination took {fake}, real unit {real}"
    );
}

#[test]
fn consume_takes_own_non_terran_units() {
    // OpenBW `unit_tech_target_valid`: own, not a building, not Terran, not
    // a Larva: a Zerg unit or a mind-controlled Protoss one, never a Marine.
    for (target, ok) in [(uid::ZERGLING, true), (65u16, true), (uid::MARINE, false)] {
        let mut w = world();
        let c = w.spawn_px(0, uid::DEFILER, 400, 400).unwrap();
        let t = w.spawn_px(0, target, 410, 400).unwrap();
        w.units[c].set_energy_fixed(100 * ENERGY_SCALE);
        let r = w.cast(
            w.units[c].id,
            Spell::Consume,
            SpellTarget::Unit(w.units[t].id),
        );
        assert_eq!(r.is_ok(), ok, "Consume on type {target}: {r:?}");
    }
}

#[test]
fn yamato_deals_the_executable_damage() {
    // Yamato Gun (weapon 30) against a large, armour-1 Supply Depot: the
    // executable's damage matrix (fixtures/oracle_damage.tsv) says 259 hp.
    let mut w = world();
    let c = w.spawn_px(0, uid::BATTLECRUISER, 300, 400).unwrap();
    let t = w.spawn_px(1, uid::SUPPLY_DEPOT, 500, 400).unwrap();
    w.units[t].completed = true;
    w.units[c].set_energy_fixed(250 * ENERGY_SCALE);
    let hp = w.units[t].hp;
    w.issue(
        w.units[c].id,
        galaxycraft::sim::Order::Cast {
            spell: Spell::YamatoGun,
            target: SpellTarget::Unit(w.units[t].id),
        },
    )
    .unwrap();
    // The Battlecruiser's lasers hit too; one hit must be the Yamato shot.
    let (mut hits, mut last) = (Vec::new(), hp);
    for _ in 0..300 {
        w.step();
        if w.units[t].hp < last {
            hits.push((last - w.units[t].hp) / 256);
        }
        last = w.units[t].hp;
    }
    assert!(hits.contains(&259), "Yamato damage among hits {hits:?}");
}

#[test]
fn scanner_sweep_detects_for_156_frames() {
    // OpenBW: a Scanner Sweep unit (sight 10, detector) that its iscript
    // removes after 156 frames.
    let mut w = world();
    let cc = w.spawn_px(0, uid::COMMAND_CENTER, 300, 300).unwrap();
    let cs = w.spawn_px(0, uid::COMSAT_STATION, 380, 320).unwrap();
    w.units[cc].completed = true;
    w.units[cs].completed = true;
    w.units[cs].addon_attached = true;
    w.units[cs].set_energy_fixed(200 * ENERGY_SCALE);
    let ghost = w.spawn_px(1, uid::GHOST, 1500, 1500).unwrap();
    w.units[ghost].status.cloaked = true;
    assert!(!w.can_see_unit(0, ghost));
    w.cast(
        w.units[cs].id,
        Spell::ScannerSweep,
        SpellTarget::Point(1560, 1500),
    )
    .unwrap();
    w.step();
    assert!(
        w.can_see_unit(0, ghost),
        "the sweep detects a cloaked Ghost 60 px away"
    );
    for _ in 0..170 {
        w.step();
    }
    assert!(
        !w.units
            .iter()
            .any(|u| u.alive && u.type_id == uid::SCANNER_SWEEP),
        "gone after 156 frames"
    );
    assert!(!w.can_see_unit(0, ghost), "no detection once it is gone");
}

#[test]
fn spider_mines_three_per_vulture_and_the_executable_damage() {
    // OpenBW: a Vulture carries 3 mines; the blast is the Spider Mines
    // weapon (6): 124 hp against a large armour-1 target
    // (fixtures/oracle_damage.tsv).
    let mut w = world();
    let v = w.spawn_px(0, uid::VULTURE, 400, 400).unwrap();
    assert_eq!(w.units[v].hangar, 3);
    w.cast(
        w.units[v].id,
        Spell::SpiderMines,
        SpellTarget::Point(420, 400),
    )
    .unwrap();
    for _ in 0..40 {
        w.step();
    }
    assert_eq!(w.units[v].hangar, 2, "one mine laid");
    w.units[v].x = 100 << 8;
    for _ in 0..200 {
        w.step();
    }
    let t = w.spawn_px(1, uid::ULTRALISK, 470, 400).unwrap();
    w.units[t].order = galaxycraft::sim::Order::HoldPosition;
    let hp = w.units[t].hp;
    let mut hit = 0;
    for _ in 0..400 {
        w.step();
        if w.units[t].hp < hp {
            hit = hp - w.units[t].hp;
            break;
        }
    }
    assert_eq!(hit, 124 << 8, "mine blast damage");
}

#[test]
fn a_depleted_refinery_still_gives_two_gas_per_trip() {
    // OpenBW `gather_resource`: fewer than 8 left, the geyser is emptied and
    // the worker brings 2; from then on every trip brings 2.
    let mut w = world();
    let cc = w.spawn_px(0, uid::COMMAND_CENTER, 300, 300).unwrap();
    w.units[cc].completed = true;
    let geyser = w.spawn_px(0, uid::REFINERY, 300, 520).unwrap();
    w.units[geyser].completed = true;
    w.units[geyser].resource_amount = 5;
    let scv = w.spawn_px(0, uid::SCV, 300, 450).unwrap();
    let gid = w.units[geyser].id;
    w.issue(
        w.units[scv].id,
        galaxycraft::sim::Order::Gather { resource: gid },
    )
    .unwrap();
    let gas0 = w.players[0].gas;
    for _ in 0..1500 {
        w.step();
    }
    let got = w.players[0].gas - gas0;
    assert!(
        got >= 6 && got % 2 == 0,
        "depleted refinery trips bring 2 each: {got}"
    );
    assert_eq!(w.units[geyser].resource_amount, 0);
}

#[test]
fn scourge_hits_its_target_before_dying() {
    // OpenBW `bullet_state_init`: a self-destruct bullet dies at once and
    // its normal hit lands (Suicide Scourge 110 against an armour-0
    // Overlord); then the Scourge is killed.
    let mut w = world();
    let s = w.spawn_px(0, uid::SCOURGE, 400, 400).unwrap();
    let t = w.spawn_px(1, uid::OVERLORD, 520, 400).unwrap();
    let hp = w.units[t].hp;
    let (sid, tid) = (w.units[s].id, w.units[t].id);
    w.issue(sid, galaxycraft::sim::Order::AttackUnit { target: tid })
        .unwrap();
    let mut lost = 0;
    for _ in 0..200 {
        w.step();
        if w.units[t].hp < hp {
            lost = hp - w.units[t].hp;
            break;
        }
    }
    assert_eq!(lost, 110 << 8, "Scourge damage");
    assert!(!w.units[s].alive, "the Scourge dies");
}

#[test]
fn burrowed_lurker_spines_hit_every_enemy_on_the_line_once() {
    // OpenBW: `unit_ground_weapon` is none for an unburrowed Lurker; a
    // burrowed one turns to its target and its spines hit each enemy within
    // the inner radius of their path once (`recent_lurker_hits`).
    let mut w = world();
    let l = w.spawn_px(0, uid::LURKER, 400, 400).unwrap();
    // Supply Depots (large, armour 1) do not shoot back.
    let near = w.spawn_px(1, uid::SUPPLY_DEPOT, 480, 400).unwrap();
    let far = w.spawn_px(1, uid::SUPPLY_DEPOT, 580, 400).unwrap();
    let off = w.spawn_px(1, uid::SUPPLY_DEPOT, 480, 540).unwrap();
    for d in [near, far, off] {
        w.units[d].completed = true;
    }
    let hp = w.units[near].hp;
    for _ in 0..60 {
        w.step();
    }
    assert_eq!(w.units[near].hp, hp, "no attack while unburrowed");
    let lid = w.units[l].id;
    w.issue(lid, galaxycraft::sim::Order::Burrow).unwrap();
    let tid = w.units[near].id;
    let mut issued = false;
    let mut hits = (0, 0, 0);
    for _ in 0..300 {
        w.step();
        if w.units[l].status.burrowed && !issued {
            w.issue(lid, galaxycraft::sim::Order::AttackUnit { target: tid })
                .unwrap();
            issued = true;
        }
        if w.units[near].hp < hp && hits.0 == 0 {
            hits = (
                hp - w.units[near].hp,
                hp - w.units[far].hp,
                hp - w.units[off].hp,
            );
        }
    }
    // Subterranean Spines 20 against armour 1: 19 hp, once per volley.
    assert_eq!(hits.0, 19 << 8, "first victim");
    assert!(
        hits.1 == 0 || hits.1 == 19 << 8,
        "second victim at most once"
    );
    assert!(w.units[far].hp < hp, "the spines reach the unit behind");
    assert_eq!(w.units[off].hp, hp, "a unit off the line is not hit");
}

#[test]
fn a_computer_lurker_burrows_to_attack_and_unburrows_after() {
    // INFERENCE (see `computer_lurkers`): the AI's Lurkers burrow when an
    // enemy is in range and come up when none is left.
    let mut w = world();
    w.players[0].is_computer = true;
    let l = w.spawn_px(0, uid::LURKER, 400, 400).unwrap();
    let d = w.spawn_px(1, uid::SUPPLY_DEPOT, 520, 400).unwrap();
    w.units[d].completed = true;
    let hp = w.units[d].hp;
    for _ in 0..200 {
        w.step();
    }
    assert!(w.units[l].status.burrowed, "burrowed next to an enemy");
    assert!(w.units[d].hp < hp, "and attacking it");
    w.units[d].alive = false;
    for _ in 0..200 {
        w.step();
    }
    assert!(!w.units[l].status.burrowed, "unburrowed once alone");
}
