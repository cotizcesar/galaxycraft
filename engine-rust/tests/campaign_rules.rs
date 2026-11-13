//! Use-map-settings scenario rules that campaign missions rely on.

use std::sync::Arc;

use galaxycraft::data::GameData;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::{Scenario, TileData};
use galaxycraft::sim::{Race, World};

/// An 8x8 scenario with the given OWNR / FORC, start locations for
/// `starts`, a Marine for every player in `marines` and one trigger.
fn chk(owners: [u8; 8], forces: [u8; 8], flags: [u8; 4], starts: &[u8], marines: &[u8]) -> Vec<u8> {
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
    let mut unit = |ty: u16, owner: u8, x: u16| {
        let mut u = vec![0u8; 36];
        u[4..6].copy_from_slice(&x.to_le_bytes());
        u[6..8].copy_from_slice(&64u16.to_le_bytes());
        u[8..10].copy_from_slice(&ty.to_le_bytes());
        u[16] = owner;
        units.extend_from_slice(&u);
    };
    for &p in starts {
        unit(uid::START_LOCATION, p, 32 + 16 * p as u16);
    }
    for &p in marines {
        unit(uid::MARINE, p, 32 + 16 * p as u16);
    }
    push(b"UNIT", &units);
    push(b"TRIG", &[0u8; 2400]);
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
