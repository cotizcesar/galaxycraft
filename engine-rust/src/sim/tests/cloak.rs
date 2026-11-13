//! Permanent cloak (`units.dat` 0x400000).

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
