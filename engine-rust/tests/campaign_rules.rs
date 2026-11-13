//! Use-map-settings scenario rules that campaign missions rely on.

use std::sync::Arc;

use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::{Scenario, TileData};
use galaxycraft::sim::{Race, World};

/// An 8x8 scenario with the given OWNR / FORC, start locations for
/// `starts`, a Marine for every player in `marines` and one trigger.
fn chk(owners: [u8; 8], forces: [u8; 8], flags: [u8; 4], starts: &[u8], marines: &[u8]) -> Vec<u8> {
    let placed: Vec<(u16, u8, u16, u16)> = marines
        .iter()
        .map(|&p| (uid::MARINE, p, 32 + 16 * p as u16, 64))
        .collect();
    chk_with(owners, forces, flags, starts, &placed, &[0u8; 2400])
}

/// Like [`chk`], with `(type, owner, x, y)` units and a `TRIG` section.
fn chk_with(
    owners: [u8; 8],
    forces: [u8; 8],
    flags: [u8; 4],
    starts: &[u8],
    placed: &[(u16, u8, u16, u16)],
    trig: &[u8],
) -> Vec<u8> {
    let mut chk = Vec::new();
    let mut push = |name: &[u8; 4], bytes: &[u8]| {
        chk.extend_from_slice(name);
        chk.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        chk.extend_from_slice(bytes);
    };
    push(b"DIM ", &[8, 0, 8, 0]);
    push(b"ERA ", &1u16.to_le_bytes());
    push(b"MTXM", &[32u8, 0].repeat(64));
    push(b"MASK", &[1u8; 64]);
    let mut ownr = [0u8; 12];
    ownr[..8].copy_from_slice(&owners);
    push(b"OWNR", &ownr);
    let mut forc = forces.to_vec();
    forc.extend_from_slice(&[0u8; 8]);
    forc.extend_from_slice(&flags);
    push(b"FORC", &forc);
    let mut units = Vec::new();
    let mut unit = |ty: u16, owner: u8, x: u16, y: u16| {
        let mut u = vec![0u8; 36];
        u[4..6].copy_from_slice(&x.to_le_bytes());
        u[6..8].copy_from_slice(&y.to_le_bytes());
        u[8..10].copy_from_slice(&ty.to_le_bytes());
        u[16] = owner;
        units.extend_from_slice(&u);
    };
    for &p in starts {
        unit(uid::START_LOCATION, p, 32 + 16 * p as u16, 64);
    }
    for &(ty, p, x, y) in placed {
        unit(ty, p, x, y);
    }
    push(b"UNIT", &units);
    push(b"TRIG", trig);
    chk
}

fn world(chk: &[u8]) -> World {
    let sc = Scenario::parse(chk).expect("parse");
    let mut vf4 = vec![0u8; 96];
    for i in 0..16 {
        vf4[64 + i * 2] = 1;
    }
    World::from_scenario(
        Arc::new(GameData::bundled().expect("data")),
        &sc,
        &TileData::synthetic(&vf4),
        &[Race::Terran; 8],
    )
}

#[test]
fn computer_players_start_allied_whatever_their_force() {
    // Hammer Falls: human seat 1, computers 0 and 6 in different forces,
    // no force alliance flags, no alliance trigger.
    let w = world(&chk(
        [5, 6, 0, 0, 0, 0, 5, 0],
        [1, 0, 3, 3, 3, 3, 2, 3],
        [0; 4],
        &[0, 1, 6],
        &[0, 1, 6],
    ));
    assert!(!w.hostile(0, 6), "the two computers are allied");
    assert!(
        w.hostile(0, 1) && w.hostile(6, 1),
        "both are the human's enemies"
    );
}

#[test]
fn a_computer_without_a_start_location_stays_apart() {
    let w = world(&chk(
        [5, 6, 5, 0, 0, 0, 0, 0],
        [1, 0, 2, 3, 3, 3, 3, 3],
        [0; 4],
        &[0, 1],
        &[0, 1, 2],
    ));
    assert!(w.hostile(0, 2));
}

#[test]
fn a_force_with_the_allies_flag_allies_its_members() {
    // Human and a computer in force 0 with "allies" (flag 2).
    let w = world(&chk(
        [6, 5, 5, 0, 0, 0, 0, 0],
        [0, 0, 1, 3, 3, 3, 3, 3],
        [2, 0, 0, 0],
        &[0, 1, 2],
        &[0, 1, 2],
    ));
    assert!(!w.hostile(0, 1), "same allied force");
    // (One team per player makes alliances transitive, so here the human
    // also ends up allied with computer 2; retail keeps that pair enemies.
    // No Terran mission puts the human in an allied force with a computer.)
}

/// One `TRIG` record run by All Players: `conds` as `(kind, group, unit,
/// comparison)`, `acts` as `(kind, time ms, group2, num)`.
fn trigger(conds: &[(u8, u32, u16, u8)], acts: &[(u8, u32, u32, u8)]) -> Vec<u8> {
    let mut r = vec![0u8; 2400];
    for (k, &(kind, group, unit, cmp)) in conds.iter().enumerate() {
        let o = k * 20;
        r[o + 4..o + 8].copy_from_slice(&group.to_le_bytes());
        r[o + 12..o + 14].copy_from_slice(&unit.to_le_bytes());
        r[o + 14] = cmp;
        r[o + 15] = kind;
    }
    for (k, &(kind, time, group2, num)) in acts.iter().enumerate() {
        let o = 320 + k * 32;
        r[o + 12..o + 16].copy_from_slice(&time.to_le_bytes());
        r[o + 20..o + 24].copy_from_slice(&group2.to_le_bytes());
        r[o + 26] = kind;
        r[o + 27] = num;
    }
    r[2372 + 17] = 1;
    r
}

#[test]
fn a_rescuable_unit_joins_by_box_gap_per_axis() {
    // Marines 50 px apart on both axes: ~71 px centre to centre, but their
    // boxes are 34 px apart on x and 31 on y (Brood War Protoss 5's
    // Artanis must change hands like this at once).
    let mut w = world(&chk_with(
        [6, 3, 0, 0, 0, 0, 0, 0],
        [0, 1, 3, 3, 3, 3, 3, 3],
        [0; 4],
        &[0],
        &[(uid::MARINE, 0, 64, 64), (uid::MARINE, 1, 114, 114)],
        &trigger(&[(23, 0, 0, 0)], &[]),
    ));
    w.step();
    let rescued = w
        .units
        .iter()
        .filter(|u| u.alive && u.type_id == uid::MARINE && u.owner == 0);
    assert_eq!(rescued.count(), 2);
}

#[test]
fn map_resources_are_player_twelve_to_triggers() {
    // "Player 1 commands at most 0 buildings" holds although the engine
    // keeps the map's Vespene Geyser under player 1 (Protoss tutorial).
    let w = world(&chk_with(
        [5, 6, 0, 0, 0, 0, 0, 0],
        [1, 0, 3, 3, 3, 3, 3, 3],
        [0; 4],
        &[0, 1],
        &[
            (uid::VESPENE_GEYSER, 11, 128, 128),
            (uid::MARINE, 1, 64, 64),
        ],
        &trigger(&[(2, 0, 231, 1)], &[]),
    ));
    let t = w.triggers.as_ref().expect("triggers");
    assert!(
        w.units
            .iter()
            .any(|u| u.alive && u.type_id == uid::VESPENE_GEYSER)
    );
    assert!(t.condition_holds(&w, 1, &t.triggers[0].conditions[0]));
}

#[test]
fn a_preserved_trigger_with_a_wait_does_not_starve_later_ones() {
    // Trigger 0: Always -> Wait 100 ms, Preserve.  Trigger 1: Always ->
    // Set Switch 5.  After the wait the pass goes on to trigger 1
    // (Brood War Zerg 1's ending).
    let mut trig = trigger(&[(22, 0, 0, 0)], &[(4, 100, 0, 0), (3, 0, 0, 0)]);
    trig.extend(trigger(&[(22, 0, 0, 0)], &[(13, 0, 5, 4)]));
    let mut w = world(&chk_with(
        [6, 0, 0, 0, 0, 0, 0, 0],
        [0, 3, 3, 3, 3, 3, 3, 3],
        [0; 4],
        &[0],
        &[(uid::MARINE, 0, 64, 64)],
        &trig,
    ));
    for _ in 0..40 {
        w.step();
    }
    assert!(w.triggers.as_ref().expect("triggers").switches[5]);
}
