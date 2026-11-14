//! `units.dat` flags: permanent cloak (0x400000) and invincible types
//! (0x20000000: Beacons, markers, resources).

use super::*;

#[test]
fn observers_and_dark_templar_are_permanently_cloaked() {
    let w = open_world();
    for t in [uid::OBSERVER, 61] {
        assert!(w.data.unit(t).is_permanently_cloaked(), "type {t}");
    }
    for t in [uid::MARINE, uid::GHOST, uid::WRAITH, uid::PROBE] {
        assert!(!w.data.unit(t).is_permanently_cloaked(), "type {t}");
    }
    // Spawned cloaked: an enemy without a detector cannot see a Dark
    // Templar; an Observer next to it can.
    let mut w = open_world();
    let dt = w.spawn(0, uid::DARK_TEMPLAR, 10, 10).unwrap();
    assert!(w.units[dt].status.cloaked);
    assert!(!w.can_see_unit(1, dt));
    let obs = w.spawn(1, uid::OBSERVER, 12, 10).unwrap();
    w.units[obs].completed = true;
    assert!(w.can_see_unit(1, dt));
}

#[test]
fn beacons_are_invincible_and_never_shot() {
    // The Dylarian Shipyards: enemy Bunkers shot the player's Terran
    // Beacons (195), the first "attack" at 27 s that retail never shows.
    let mut w = open_world();
    let beacon = w.spawn(0, 195, 10, 10).unwrap();
    let marine = w.spawn(0, uid::MARINE, 20, 20).unwrap();
    assert!(w.units[beacon].invincible);
    assert!(!w.units[marine].invincible);
    let hp = w.units[beacon].hp;
    w.spawn(1, uid::MARINE, 11, 10).unwrap();
    for _ in 0..200 {
        w.step();
    }
    assert_eq!(w.units[beacon].hp, hp, "nothing damages a Beacon");
}
