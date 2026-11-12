#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent20.rs
// ===========================================================================
// Translation batch 20.
//
// Only this file is edited by the batch.  Every method below is an
// inherent helper (prefixed `a20_`) whose observable behaviour is
// recovered from a `decompiled_uncited` routine in
// `0x005d0000..=0x005d7fff`.  That range is the retail status-panel /
// unit-removal helper cluster: the routines read `CUnit` flags and the
// per-unit status timers, and the comments name the `CUnit` offsets each
// one touches.  None of the translated routines needs new persistent
// state; the existing [`crate::sim::Status`], [`crate::sim::SimUnit`]
// flags and `CUnit+0x94` storage already model the retail fields.
//
// Retail's failure branches call a noreturn abort helper.  Translating
// those branches as `None` / `false` keeps the observable distinction:
// the caller either gets the recovered value or knows the retail path
// would have aborted.

/// Score-screen race class bit-tests (`group_flags` low bits: Zerg = 0,
/// Terran = 1, Protoss = 2, other = 4).  Same table the status panel uses.
fn a20_race_class(group_flags: u8) -> u8 {
    if group_flags & 1 != 0 {
        0
    } else if group_flags & 4 != 0 {
        2
    } else if group_flags & 2 != 0 {
        1
    } else {
        4
    }
}

/// The status panel's low-health Command Center test: type `0x6a` with the
/// removal/death status bit set and current whole HP below 50% of the raw
/// maximum.  Retail rounds both values up with `(value + 0xff) >> 8` before
/// forming the percentage.
fn a20_low_health_command_center(unit: &SimUnit, data: &GameData) -> bool {
    const COMMAND_CENTER: u16 = 0x6a;
    if unit.type_id != COMMAND_CENTER {
        return false;
    }
    // The retail predicate also requires `CUnit+0xdc` bit 0, the
    // removal/death-in-progress bit.  The Rust model has no separate bit-0
    // field; `!alive` is the closest existing state.
    if unit.alive {
        return false;
    }
    let current = (unit.hp.max(0) + 0xff) >> 8;
    let maximum = (data.unit(unit.type_id).hit_points_raw as i32 + 0xff) >> 8;
    maximum > 0 && current * 100 / maximum < 0x32
}

impl SimUnit {
    /// `0x005d24d0` (`FUN_005d24d0`, size 66): removal precondition.  A
    /// completed unit (`CUnit+0xdc` bit 4) is accepted only when the
    /// organic/building validation below holds; an incomplete unit must be
    /// loaded inside a Bunker (`CUnit+0xdc` bit 6), otherwise retail reaches
    /// its noreturn abort.  A unit with a linked parent (`CUnit+0x80`) is
    /// additionally handed to the parent-notification callback, which has no
    /// persistent effect of its own.
    pub fn a20_removal_precondition_holds(&self, data: &GameData) -> bool {
        if self.completed {
            self.a20_organic_implies_building(data)
        } else {
            self.in_bunker
        }
    }

    /// `0x005d2570` (`FUN_005d2570`, size 45) and `0x005d2c80`
    /// (`FUN_005d2c80`, size 36) are byte-identical: when the unit type
    /// carries the organic flag (`units.dat` bit `0x10000`) it must also be
    /// a building, otherwise retail aborts.  Returns `true` for the accepted
    /// combinations: organic buildings, and every non-organic type.
    pub fn a20_organic_implies_building(&self, data: &GameData) -> bool {
        let ty = data.unit(self.type_id);
        !ty.is_organic() || ty.is_building()
    }

    /// `0x005d2720` (`FUN_005d2720`, size 69): periodic Irradiate status
    /// update.  With a live `CUnit+0x118` timer the routine re-tests whether
    /// the irradiation source is still a valid spell target; an invalid
    /// target clears the timer and the source pointer (`CUnit+0x11c`).  A
    /// valid target leaves the timer untouched, temporarily zeroes it only
    /// to gate the cue, and returns cue `0x17c + local_player` for a unit
    /// that is not complete.
    ///
    /// The Rust status has no irradiate-source field, so only the timer is
    /// cleared on the invalid branch.  `targetable` carries the recovered
    /// targeting verdict.
    pub fn a20_irradiate_status_update(&mut self, targetable: bool, player: u8) -> Option<u16> {
        if self.status.irradiate_ticks == 0 {
            return None;
        }
        if !targetable {
            self.status.irradiate_ticks = 0;
            return None;
        }
        if !self.completed {
            Some(0x17c + u16::from(player))
        } else {
            None
        }
    }

    /// `0x005d2ae0` (`FUN_005d2ae0`, size 36) and `0x005d2b04`
    /// (`FUN_005d2b04`, size 81): the death-splat path guards HP first and
    /// aborts when `CUnit+0x08 < 0xa01`.  On the accepted path both routines
    /// continue into the noreturn removal call.  `true` therefore means the
    /// unit has more than ten whole hit points (0xa00 in 1/256 points).
    pub fn a20_death_splat_hp_ok(&self) -> bool {
        self.hp > 0xa00
    }

    /// `0x005d2d70` (`FUN_005d2d70`, size 27): asserts that the linked
    /// parent at `CUnit+0x80` is null and aborts when it is not.  Returns
    /// `true` when the link is already clear.
    pub fn a20_has_no_hatchery_link(&self) -> bool {
        self.connected_unit.is_none()
    }

    /// `0x005d263a` (`FUN_005d263a`, size 35): stores the incoming u16 in
    /// `CUnit+0x94` through the guarded setter and then runs the shared
    /// status-cue path with the random cue range `0x169..=0x16b`, which also
    /// rewrites the order wait counter `CUnit+0x85` to `0xf`.
    ///
    /// The store is skipped when a status effect is active (`CUnit+0xdc`
    /// bit 10, Lockdown, Stasis or Maelstrom), the unit is not a building,
    /// and the value is not the `0xe4` sentinel.  The cue/order-counter
    /// update itself is unconditional in the recovered control flow.
    /// Returns whether the value reached `CUnit+0x94`.
    pub fn a20_store_field_094(&mut self, data: &GameData, value: u16) -> bool {
        let status_active = self.flag_10
            || self.status.lockdown_ticks != 0
            || self.status.stasis_ticks != 0
            || self.status.maelstrom_ticks != 0;
        let blocked = status_active && !data.unit(self.type_id).is_building() && value != 0xe4;
        if !blocked {
            self.field_094 = value;
        }
        self.order_carry = 0xf;
        !blocked
    }

    /// `0x005d2487` (`FUN_005d2487`, size 65): the removal path queues sound
    /// id `0x21b` before tail-calling the noreturn cleanup.  The helper
    /// returns that id so the client can consume the cue without repeating
    /// the teardown.
    pub fn a20_removal_sound_cue() -> u16 {
        0x21b
    }

    /// `0x005d2a1f` (`FUN_005d2a1f`, size 84): derives a 0..2 class from the
    /// unit type's `units.dat` flags (`0x02000000` -> 1, else `0x04000000`
    /// on ids below `0xe4` -> 2, else 0), queues cue `0x3d0 + class`, and
    /// sets the global victory latch.  The latch is not simulation state, so
    /// the helper returns the cue id.
    pub fn a20_status_sound_cue(&self, data: &GameData) -> u16 {
        let ty = data.unit(self.type_id);
        let class = if ty.flags & 0x0200_0000 != 0 {
            1
        } else if self.type_id < 0xe4 && ty.flags & 0x0400_0000 != 0 {
            2
        } else {
            0
        };
        0x3d0 + class
    }

    /// `0x005d4753` (`FUN_005d4753`, size 10): constant getter; the recovered
    /// body returns `3` without reading any unit or world field.
    pub fn a20_dialog_choice_count() -> i32 {
        3
    }
}

impl World {
    /// `0x005d1bf0` (`FUN_005d1bf0`, size 63): status-panel icon selection.
    /// The routine reads the unit's Stasis timer (`CUnit+0x119`) first and
    /// returns icon `900` while it is live, then switches on the incoming
    /// order/button id.  The unit pointer supplies the type, owner
    /// (`CUnit+0x4c`) and status flags; the third argument is the owner the
    /// panel is drawing for.
    ///
    /// Returned icon ids:
    /// * `6, 7, 0x18, 0x1e` -> `0` for non-buildings, `0x36d` for buildings;
    /// * `0xc` -> `0` for a low-health Command Center in the removal path,
    ///   otherwise `0x366`;
    /// * `0x10` -> `0x381`, except the Zerg-owned non-building case that
    ///   aborts;
    /// * `0x1b` -> `0x52f` for the panel owner, abort for another owner's
    ///   non-building, `0x36d` for another owner's building;
    /// * `0x1d` -> `0` for a non-building with the energy flag and no dying
    ///   bit, `0x532` for other non-buildings, `0x36d` for buildings;
    /// * every other id -> `0x36c`.
    ///
    /// `None` marks the three noreturn branches (`0x10` Zerg-owned
    /// non-building, `0x14`, and `0x1b` another owner's non-building).
    pub fn a20_status_icon_for_order(&self, unit_id: u32, order_id: u32, owner: u8) -> Option<u16> {
        let unit = self.unit(unit_id)?;
        if unit.status.stasis_ticks != 0 {
            return Some(900);
        }
        let ty = self.data.unit(unit.type_id);
        let building = ty.is_building();
        match order_id {
            6 | 7 | 0x18 | 0x1e => Some(if building { 0x36d } else { 0 }),
            0xc => Some(if a20_low_health_command_center(unit, self.data.as_ref()) {
                0
            } else {
                0x366
            }),
            0x10 => {
                if !building && unit.owner == owner && a20_race_class(ty.group_flags) == 0 {
                    None
                } else {
                    Some(0x381)
                }
            }
            0x14 => None,
            0x1b => {
                if unit.owner == owner {
                    Some(0x52f)
                } else if !building {
                    None
                } else {
                    Some(0x36d)
                }
            }
            0x1d => {
                if building {
                    Some(0x36d)
                } else if unit.alive && ty.has_energy() {
                    Some(0)
                } else {
                    Some(0x532)
                }
            }
            _ => Some(0x36c),
        }
    }
}

#[cfg(test)]
mod tests_agent20 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, type_id: u16, x: i32) -> usize {
        world.spawn_px(0, type_id, x, 320).unwrap()
    }

    #[test]
    fn status_icon_stasis_short_circuits_every_order() {
        let mut w = open_world();
        let i = spawn(&mut w, uid::MARINE, 320);
        let id = w.units[i].id;
        w.units[i].status.stasis_ticks = 1;
        assert_eq!(w.a20_status_icon_for_order(id, 6, 0), Some(900));
        assert_eq!(w.a20_status_icon_for_order(id, 0x14, 0), Some(900));
        assert_eq!(w.a20_status_icon_for_order(id, 0xff, 0), Some(900));
        // Unknown unit id keeps the Option wrapper but no unit lookup.
        assert_eq!(w.a20_status_icon_for_order(9999, 6, 0), None);
    }

    #[test]
    fn status_icon_building_gated_orders() {
        let mut w = open_world();
        let marine = spawn(&mut w, uid::MARINE, 320);
        let barracks = spawn(&mut w, uid::BARRACKS, 400);
        let (m, b) = (w.units[marine].id, w.units[barracks].id);
        for order in [6, 7, 0x18, 0x1e] {
            assert_eq!(
                w.a20_status_icon_for_order(m, order, 0),
                Some(0),
                "{order:x}"
            );
            assert_eq!(
                w.a20_status_icon_for_order(b, order, 0),
                Some(0x36d),
                "{order:x}"
            );
        }
        assert_eq!(w.a20_status_icon_for_order(m, 0x99, 0), Some(0x36c));
    }

    #[test]
    fn status_icon_command_center_low_health_branch() {
        let mut w = open_world();
        let cc = spawn(&mut w, uid::COMMAND_CENTER, 256);
        let id = w.units[cc].id;
        // A healthy, living Command Center shows the ordinary icon.
        assert_eq!(w.a20_status_icon_for_order(id, 0xc, 0), Some(0x366));
        // Removal-path bit plus below-half HP selects icon 0.
        w.units[cc].alive = false;
        w.units[cc].hp = 384_000 * 40 / 100;
        assert_eq!(w.a20_status_icon_for_order(id, 0xc, 0), Some(0));
        // At exactly half HP the predicate is no longer below 50%.
        w.units[cc].hp = 384_000 / 2;
        assert_eq!(w.a20_status_icon_for_order(id, 0xc, 0), Some(0x366));
    }

    #[test]
    fn status_icon_owner_and_race_branches() {
        let mut w = open_world();
        let zergling = spawn(&mut w, uid::ZERGLING, 320);
        let marine = spawn(&mut w, uid::MARINE, 400);
        let barracks = spawn(&mut w, uid::BARRACKS, 480);
        let (z, m, b) = (
            w.units[zergling].id,
            w.units[marine].id,
            w.units[barracks].id,
        );
        // Zerg-owned non-building: the retail abort branch.
        assert_eq!(w.a20_status_icon_for_order(z, 0x10, 0), None);
        assert_eq!(w.a20_status_icon_for_order(m, 0x10, 0), Some(0x381));
        assert_eq!(w.a20_status_icon_for_order(z, 0x10, 1), Some(0x381));
        // Unconditional abort.
        assert_eq!(w.a20_status_icon_for_order(m, 0x14, 0), None);
        // Panel owner sees the special icon.
        assert_eq!(w.a20_status_icon_for_order(m, 0x1b, 0), Some(0x52f));
        assert_eq!(w.a20_status_icon_for_order(z, 0x1b, 1), None);
        assert_eq!(w.a20_status_icon_for_order(b, 0x1b, 1), Some(0x36d));
        // Energy-flag branch.
        let ghost = spawn(&mut w, uid::GHOST, 256);
        let g = w.units[ghost].id;
        assert_eq!(w.a20_status_icon_for_order(g, 0x1d, 0), Some(0));
        assert_eq!(w.a20_status_icon_for_order(z, 0x1d, 0), Some(0x532));
        assert_eq!(w.a20_status_icon_for_order(b, 0x1d, 0), Some(0x36d));
    }

    #[test]
    fn removal_precondition_requires_completed_validation_or_bunker() {
        let mut w = open_world();
        let marine = spawn(&mut w, uid::MARINE, 320);
        let barracks = spawn(&mut w, uid::BARRACKS, 400);
        let (m, b) = (w.units[marine].id, w.units[barracks].id);
        let data = Arc::clone(&w.data);
        let mi = w.index_of(m).unwrap();
        let bi = w.index_of(b).unwrap();
        // Completed organic non-building fails the validation.
        w.units[mi].completed = true;
        w.units[mi].in_bunker = false;
        assert!(!w.units[mi].a20_removal_precondition_holds(&data));
        // Completed building passes.
        w.units[bi].completed = true;
        assert!(w.units[bi].a20_removal_precondition_holds(&data));
        // Incomplete units need the Bunker bit.
        w.units[bi].completed = false;
        assert!(!w.units[bi].a20_removal_precondition_holds(&data));
        w.units[bi].in_bunker = true;
        assert!(w.units[bi].a20_removal_precondition_holds(&data));
    }

    #[test]
    fn organic_types_must_be_buildings() {
        let mut w = open_world();
        let zergling = spawn(&mut w, uid::ZERGLING, 320);
        let hatchery = spawn(&mut w, uid::HATCHERY, 400);
        let barracks = spawn(&mut w, uid::BARRACKS, 480);
        let data = Arc::clone(&w.data);
        let z = w.index_of(w.units[zergling].id).unwrap();
        let h = w.index_of(w.units[hatchery].id).unwrap();
        let b = w.index_of(w.units[barracks].id).unwrap();
        assert!(!w.units[z].a20_organic_implies_building(&data));
        assert!(w.units[h].a20_organic_implies_building(&data));
        assert!(w.units[b].a20_organic_implies_building(&data));
    }

    #[test]
    fn irradiate_update_clears_invalid_target_and_cues_incomplete_units() {
        let mut w = open_world();
        let marine = spawn(&mut w, uid::MARINE, 320);
        let i = w.index_of(w.units[marine].id).unwrap();
        assert_eq!(w.units[i].a20_irradiate_status_update(false, 2), None);
        w.units[i].status.irradiate_ticks = 75;
        assert_eq!(w.units[i].a20_irradiate_status_update(false, 2), None);
        assert_eq!(w.units[i].status.irradiate_ticks, 0);
        w.units[i].status.irradiate_ticks = 75;
        w.units[i].completed = false;
        assert_eq!(
            w.units[i].a20_irradiate_status_update(true, 2),
            Some(0x17c + 2)
        );
        assert_eq!(w.units[i].status.irradiate_ticks, 75);
        w.units[i].completed = true;
        assert_eq!(w.units[i].a20_irradiate_status_update(true, 2), None);
        assert_eq!(w.units[i].status.irradiate_ticks, 75);
    }

    #[test]
    fn death_splat_hp_requires_more_than_ten_points() {
        let mut w = open_world();
        let marine = spawn(&mut w, uid::MARINE, 320);
        let i = w.index_of(w.units[marine].id).unwrap();
        w.units[i].hp = 0xa00;
        assert!(!w.units[i].a20_death_splat_hp_ok());
        w.units[i].hp = 0xa01;
        assert!(w.units[i].a20_death_splat_hp_ok());
        w.units[i].hp = 0;
        assert!(!w.units[i].a20_death_splat_hp_ok());
    }

    #[test]
    fn hatchery_link_absence_predicate() {
        let mut w = open_world();
        let larva = spawn(&mut w, uid::LARVA, 320);
        let i = w.index_of(w.units[larva].id).unwrap();
        w.units[i].connected_unit = None;
        assert!(w.units[i].a20_has_no_hatchery_link());
        w.units[i].connected_unit = Some(7);
        assert!(!w.units[i].a20_has_no_hatchery_link());
    }

    #[test]
    fn removal_cue_is_the_recovered_sound_id() {
        assert_eq!(SimUnit::a20_removal_sound_cue(), 0x21b);
    }

    #[test]
    fn status_sound_cue_uses_the_units_dat_flag_class() {
        let mut w = open_world();
        let marine = spawn(&mut w, uid::MARINE, 320);
        let tank = spawn(&mut w, uid::SIEGE_TANK, 400);
        let cc = spawn(&mut w, uid::COMMAND_CENTER, 480);
        let data = Arc::clone(&w.data);
        assert_eq!(w.units[marine].a20_status_sound_cue(&data), 0x3d0);
        assert_eq!(w.units[tank].a20_status_sound_cue(&data), 0x3d1);
        assert_eq!(w.units[cc].a20_status_sound_cue(&data), 0x3d2);
    }

    #[test]
    fn dialog_choice_count_is_the_constant_body() {
        assert_eq!(SimUnit::a20_dialog_choice_count(), 3);
    }

    #[test]
    fn field_094_store_is_guarded_but_order_wait_always_resets() {
        let mut w = open_world();
        let marine = spawn(&mut w, uid::MARINE, 320);
        let barracks = spawn(&mut w, uid::BARRACKS, 400);
        let data = Arc::clone(&w.data);
        let m = w.index_of(w.units[marine].id).unwrap();
        let b = w.index_of(w.units[barracks].id).unwrap();

        // No status effect: the value is stored and the wait counter resets.
        w.units[m].flag_10 = false;
        w.units[m].field_094 = 0;
        w.units[m].order_carry = 0;
        assert!(w.units[m].a20_store_field_094(&data, 7));
        assert_eq!(w.units[m].field_094, 7);
        assert_eq!(w.units[m].order_carry, 0xf);

        // Active status on a non-building: store suppressed, counter reset.
        w.units[m].flag_10 = true;
        assert!(!w.units[m].a20_store_field_094(&data, 9));
        assert_eq!(w.units[m].field_094, 7);
        assert_eq!(w.units[m].order_carry, 0xf);

        // The 0xe4 sentinel bypasses the suppression.
        assert!(w.units[m].a20_store_field_094(&data, 0xe4));
        assert_eq!(w.units[m].field_094, 0xe4);

        // Buildings are exempt from the suppression.
        w.units[b].flag_10 = true;
        w.units[b].status.lockdown_ticks = 10;
        assert!(w.units[b].a20_store_field_094(&data, 0x1234));
        assert_eq!(w.units[b].field_094, 0x1234);

        // Maelstrom and Stasis also count as active status effects.
        w.units[m].flag_10 = false;
        w.units[m].status.maelstrom_ticks = 1;
        assert!(!w.units[m].a20_store_field_094(&data, 5));
        w.units[m].status.maelstrom_ticks = 0;
        w.units[m].status.stasis_ticks = 1;
        assert!(!w.units[m].a20_store_field_094(&data, 5));
    }
}
