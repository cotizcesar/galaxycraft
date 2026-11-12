#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent32.rs
// ===========================================================================
// Translation batch 32.  Only this file is edited by the batch.
//
// Fifteen `decompiled_uncited` ledger rows in `0x005d2000..=0x005d3fff`.
// The window is the retail order/status-effect tail: the archon-meld target
// link, the Nuclear Silo launch and Ghost aim state machines, and the
// order-completion cue paths.  Every row below is an inherent helper with
// the exact ledger address in its doc comment.
//
// Several ledger rows are Ghidra split points rather than real function
// entries: `0x005d2112`, `0x005d3160`, `0x005d31c0`, `0x005d31e2`,
// `0x005d32bf`, `0x005d336c`, `0x005d36cf`, `0x005d37e4` and `0x005d3bb7`
// start inside a larger routine at a branch or call target.  They are still
// translated because the block they enter has a recoverable record-level
// effect; each doc comment says where the block sits.  Rows that are
// `int3` padding, mid-operand bytes or exception-unwind scaffolding have no
// record effect and are reported as skipped.
//
// Only the existing public [`World`] / [`SimUnit`] state is written.  Sound
// ids, sprite-vertex writes and window refreshes have no Rust field; those
// are returned to the caller or described in the doc comment instead of
// being invented as new persistent state.

impl World {
    /// `0x005d2112`: link the unit created by a morph/spawn tail to its
    /// creator.
    ///
    /// The recovered block stores the new unit in the creator's current
    /// target slot (`CUnit+0xec`), stores the creator in the new unit's
    /// target slot and in its linked-parent slot (`CUnit+0x80`), and runs the
    /// order-row setter on the creator with row `0x25`.  In the Rust record
    /// those slots are [`SimUnit::target`] and
    /// [`SimUnit::connected_unit`]; the row is [`SimUnit::order_row`].
    ///
    /// Returns the cue `0x2c` that the recovered block plays for the new
    /// unit, or `None` when either id is unknown or the ids coincide.
    pub fn a32_link_created_unit(&mut self, creator_id: u32, created_id: u32) -> Option<u8> {
        if creator_id == created_id {
            return None;
        }
        let creator = self.index_of(creator_id)?;
        let created = self.index_of(created_id)?;
        self.units[creator].target = Some(created_id);
        self.units[creator].order_row = 0x25;
        self.units[created].target = Some(creator_id);
        self.units[created].connected_unit = Some(creator_id);
        Some(0x2c)
    }

    /// `0x005d2ce0`: ownership validation for a freshly linked launch unit.
    ///
    /// The recovered routine takes the new unit as its argument, checks that
    /// it belongs to the caller's owner, then — when the caller's type is the
    /// Nuclear Silo row `0x6c` and its missile link (`CUnit+0xd0`) is
    /// non-null — checks that the linked missile belongs to the same owner as
    /// well.  The Rust model keeps the silo side of that link in
    /// [`World::nuke_strikes`]; a strike whose recorded owner disagrees with
    /// the silo is the mismatch case.  The `CUnit+0xc0` slot and the
    /// owner-colour/sound replays around the assertions have no Rust field,
    /// so only the assertions are reproduced.
    ///
    /// Returns `true` only when every recovered owner assertion holds.
    pub fn a32_silo_missile_owner_ok(&self, silo_id: u32, other_id: u32) -> bool {
        let Some(silo) = self.unit(silo_id) else {
            return false;
        };
        let Some(other) = self.unit(other_id) else {
            return false;
        };
        if silo.owner != other.owner {
            return false;
        }
        if silo.type_id == uid::NUCLEAR_SILO {
            for strike in &self.nuke_strikes {
                if strike.silo == silo_id && strike.owner != silo.owner {
                    return false;
                }
            }
        }
        true
    }

    /// `0x005d2d60`: step the caller's facing toward its linked add-on.
    ///
    /// The recovered body loads the attached unit (`CUnit+0x70`, the Rust
    /// [`SimUnit::addon`] slot), reads that sprite's direction byte as the
    /// stepper target, and pushes an unused `0x40` word alongside it.  The
    /// stepper keeps the current direction when the signed byte difference is
    /// within `-0x20..=0x20`, moves toward a target more than `0x20` ahead,
    /// and takes the short wrap-around for a target more than `0x7f` behind.
    ///
    /// Returns the new facing, or `None` when no add-on is linked.
    pub fn a32_turn_toward_addon(&mut self, unit_id: u32) -> Option<u8> {
        let unit = self.index_of(unit_id)?;
        let addon_id = self.units[unit].addon?;
        let addon = self.index_of(addon_id)?;
        let current = self.units[unit].facing;
        let target = self.units[addon].facing;
        let step = a32_direction_step(current, target);
        self.units[unit].facing = step;
        Some(step)
    }

    /// `0x005d31e2`: tail of the finish path that dispatches the launch-unit
    /// link.
    ///
    /// The recovered block calls the `0x005d2ce0` binder only when the
    /// incoming dispatch value is non-zero; the following window checks only
    /// refresh the command card and are not simulation state.  `missile` is
    /// that dispatch value.
    ///
    /// Returns the binder verdict, or `false` when the dispatch value is
    /// absent.
    pub fn a32_bind_finished_unit(&self, silo_id: u32, missile: Option<u32>) -> bool {
        match missile {
            Some(missile_id) => self.a32_silo_missile_owner_ok(silo_id, missile_id),
            None => false,
        }
    }

    /// `0x005d36cf`: store the Ghost's aimed point and target link.
    ///
    /// The recovered block copies the aimed object's `x`/`y` into the unit's
    /// position (`CUnit+0x58`/`+0x5a`), writes the `0x0f` retry value to the
    /// order wait byte (`CUnit+0x54`), and then issues the point row `0xb0`
    /// followed by row `0xb1` either against the resolved target unit or
    /// against the aimed point.
    /// The Rust model has no separate nuke-aim order, so the row and wait
    /// byte are stored directly; the caller-visible result is whether a unit
    /// target was recorded.
    ///
    /// Returns `true` when `target` names an existing unit (the `0xb1` order
    /// then carries that unit), `false` for a point-only aim or an unknown
    /// ghost.
    pub fn a32_nuke_aim_at(&mut self, ghost_id: u32, pos: (i32, i32), target: Option<u32>) -> bool {
        let Some(ghost) = self.index_of(ghost_id) else {
            return false;
        };
        self.units[ghost].x = pos.0;
        self.units[ghost].y = pos.1;
        self.units[ghost].order_wait = 0x0f;
        self.units[ghost].order_row = 0xb1;
        let target = target.filter(|&id| self.unit(id).is_some());
        self.units[ghost].target = target;
        target.is_some()
    }

    /// `0x005d3754`: issue the nuke-strike order row `0xb1`.
    ///
    /// The recovered block pushes the target unit and row `0xb1` into the
    /// shared order issue helper with no point argument.  The Rust front door
    /// for an `orders.dat` row is [`World::issue_order_id`].
    pub fn a32_issue_nuke_target_order(
        &mut self,
        unit_id: u32,
        target: u32,
    ) -> Result<(), SimError> {
        self.issue_order_id(unit_id, 0xb1, Some(target), None)
    }
}

impl SimUnit {
    /// `0x005d2160`: true when an order `0x25` row is waiting on a linked
    /// unit.
    ///
    /// The recovered predicate requires the current order row
    /// (`CUnit+0xa6`) to be `0x25`, status bit 1 of `CUnit+0xdc` to be set,
    /// the current target slot (`CUnit+0xec`) to be non-null, and status
    /// bit 0 (the removal/death-in-progress bit) to be clear.  Bit 1 is not
    /// stored by [`SimUnit`] and is supplied by the caller; bit 0 is modelled
    /// by [`SimUnit::alive`] (the same mapping used elsewhere in this batch
    /// cluster).
    pub fn a32_waits_for_linked_target(&self, status_bit1: bool) -> bool {
        self.order_row == 0x25 && status_bit1 && self.target.is_some() && self.alive
    }

    /// `0x005d2960`: set one `CUnit+0x123` bit, or drop the order list on
    /// the dying path.
    ///
    /// The recovered routine first tests status bit 30 of `CUnit+0xdc`
    /// (`dying` here).  When it is set the routine runs the order-list
    /// teardown, which clears the intrusive order list (`CUnit+0x74`, the
    /// Rust [`SimUnit::queue`]) and sets `CUnit+0x93` bit 0
    /// ([`SimUnit::flags_93`]), then aborts.  Otherwise it sets bit
    /// `bit & 31` of `CUnit+0x123` through the mask setter and throws.
    ///
    /// Returns `true` for the teardown path and `false` for the mask path.
    pub fn a32_mark_state_or_drop_orders(&mut self, bit: u8, dying: bool) -> bool {
        if dying {
            self.queue.clear();
            self.flags_93 |= 1;
            true
        } else {
            self.set_mask_123_bit(bit);
            false
        }
    }

    /// `0x005d3160`: per-type cue for a unit whose order list has drained.
    ///
    /// The recovered block runs after the order-list loop: when the list
    /// (`CUnit+0x74`) is empty it reads the unit type, looks up the per-type
    /// sound byte, and plays it through the cue helper that also sets
    /// `CUnit+0x93` bit 0.  The per-type sound column is not part of
    /// [`GameData`], so the caller supplies it.
    ///
    /// Returns the cue id when the list was empty (and sets
    /// [`SimUnit::flags_93`] bit 0), otherwise `None`.
    pub fn a32_idle_cue(&mut self, per_type_sound: u8) -> Option<u8> {
        if !self.queue.is_empty() {
            return None;
        }
        self.flags_93 |= 1;
        Some(per_type_sound)
    }

    /// `0x005d31c0`: the global-alert cue branch.
    ///
    /// The recovered block is entered only while the process-global alert
    /// latch is non-zero; it plays the fixed cue `0x48`, then the per-type
    /// sound byte and the shared order-tail helper.  The latch has no Rust
    /// field, so it is the `alert` argument.  The fixed `0x48` cue sets
    /// `CUnit+0x93` bit 0; the per-type sound is returned for the caller.
    ///
    /// Returns the per-type cue when the latch is set, otherwise `None`.
    pub fn a32_global_alert_cue(&mut self, alert: bool, per_type_sound: u8) -> Option<u8> {
        if !alert {
            return None;
        }
        self.flags_93 |= 1;
        Some(per_type_sound)
    }

    /// `0x005d32bf`: completion flags written by the spawn tail.
    ///
    /// The recovered dword update clears status bit 1, sets status bit 16
    /// (`CUnit+0xdc` bit 16, [`SimUnit::flag_16`]), then clears bit 17 and
    /// re-sets it only when the type does **not** carry the `units.dat`
    /// subunit flag `0x10`.  Bit 1 and the other dword bits are not modelled;
    /// bits 16 and 17 are.
    pub fn a32_apply_completion_flags(&mut self, data: &GameData) {
        self.flag_16 = true;
        self.flag_17 = !data.unit(self.type_id).is_subunit();
    }

    /// `0x005d336c`: per-type cue for the drained-order completion block.
    ///
    /// The ledger row starts one byte inside the preceding `jne` and enters
    /// the block that checks the order list (`CUnit+0x74`) and, when it is
    /// empty, plays the per-type sound through the non-mask cue helper.  That
    /// helper does **not** touch `CUnit+0x93`; unlike
    /// [`SimUnit::a32_idle_cue`] this method leaves [`SimUnit::flags_93`]
    /// alone.
    ///
    /// Returns the cue id for an empty list, otherwise `None`.
    pub fn a32_order_drained_cue(&self, per_type_sound: u8) -> Option<u8> {
        if self.queue.is_empty() {
            Some(per_type_sound)
        } else {
            None
        }
    }

    /// `0x005d33a0`: switch the record to order row `0x17`.
    ///
    /// The recovered body is a single order-row setter call with the
    /// constant `0x17`.  The setter only rewrites the row when it differs,
    /// and that rewrite also zeroes the target slot (`CUnit+0xec`) and the
    /// saved-order slot (`CUnit+0xe8`).  In the Rust record those are
    /// [`SimUnit::target`] and [`SimUnit::resume_order`].
    ///
    /// Returns `true` when the row changed.
    pub fn a32_set_order_row_17(&mut self) -> bool {
        if self.order_row == 0x17 {
            return false;
        }
        self.order_row = 0x17;
        self.target = None;
        self.resume_order = None;
        true
    }

    /// `0x005d37e4`: stage a resolved nuke aim target.
    ///
    /// The recovered block is the `CUnit+0x54 == 0` branch: it stores the
    /// `0x0f` retry value in the wait byte, resolves the aimed object, stores
    /// it in `CUnit+0x5c` and, when the object exists, copies its coordinates
    /// into the unit position (`CUnit+0x58`/`+0x5a`).
    ///
    /// `target` is the resolved aim position.  Returns `true` when a position
    /// was stored, `false` when the wait byte was already busy or the aim was
    /// empty (the wait byte is still set on the latter path).
    pub fn a32_stage_nuke_aim(&mut self, target: Option<(i32, i32)>) -> bool {
        if self.order_wait != 0 {
            return false;
        }
        self.order_wait = 0x0f;
        match target {
            Some((x, y)) => {
                self.x = x;
                self.y = y;
                true
            }
            None => false,
        }
    }

    /// `0x005d3bb7`: idle cue for the phase-6 parented branch.
    ///
    /// The recovered block runs when the order phase byte (`CUnit+0x4e`) is
    /// `6`.  It returns early when a linked parent (`CUnit+0x80`) exists and
    /// the parent's own phase byte is not `5`; otherwise, with an empty order
    /// list, it plays the per-type sound through the non-mask cue helper.
    /// Neither the unit phase nor the parent phase is stored by [`SimUnit`],
    /// so both are caller-supplied; the parent link is
    /// [`SimUnit::connected_unit`].
    ///
    /// Returns the cue id when the block reaches the sound call.
    pub fn a32_phase_six_idle_cue(
        &self,
        phase: u8,
        parent_phase: Option<u8>,
        per_type_sound: u8,
    ) -> Option<u8> {
        if phase != 6 {
            return None;
        }
        if self.connected_unit.is_some() && parent_phase != Some(5) {
            return None;
        }
        if !self.queue.is_empty() {
            return None;
        }
        Some(per_type_sound)
    }
}

/// The sprite-direction step shared by `0x005d2d60`: signed byte difference,
/// short wrap-around below `-0x7f`, and a `0x20` step on either side.
fn a32_direction_step(current: u8, target: u8) -> u8 {
    let diff = target.wrapping_sub(current) as i8;
    if diff < -0x7f {
        current.wrapping_sub(0x60)
    } else if diff < -0x20 {
        target.wrapping_add(0x20)
    } else if diff > 0x20 {
        target.wrapping_sub(0x20)
    } else {
        current
    }
}

#[cfg(test)]
mod tests_agent32 {
    use super::*;
    use crate::map::Terrain;
    use crate::sim::Order;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, owner: usize, type_id: u16, x: i32, y: i32) -> u32 {
        let i = world.spawn(owner, type_id, x, y).unwrap();
        world.units[i].id
    }

    /// `0x005d2112`: both records point at each other, the new unit records
    /// the creator as parent, and the creator takes order row `0x25`.
    #[test]
    fn created_unit_is_linked_to_creator() {
        let mut w = world();
        let creator = spawn(&mut w, 0, uid::HIGH_TEMPLAR, 10, 10);
        let created = spawn(&mut w, 0, uid::ARCHON, 12, 10);

        assert_eq!(w.a32_link_created_unit(creator, created), Some(0x2c));
        assert_eq!(w.units[w.index_of(creator).unwrap()].target, Some(created));
        assert_eq!(w.units[w.index_of(creator).unwrap()].order_row, 0x25);
        assert_eq!(w.units[w.index_of(created).unwrap()].target, Some(creator));
        assert_eq!(
            w.units[w.index_of(created).unwrap()].connected_unit,
            Some(creator)
        );

        // Unknown ids and self-links do not touch the records.
        assert_eq!(w.a32_link_created_unit(creator, 9999), None);
        assert_eq!(w.a32_link_created_unit(creator, creator), None);
    }

    /// `0x005d2160`: all four recovered conditions are required.
    #[test]
    fn linked_target_predicate_requires_all_four_conditions() {
        let mut w = world();
        let unit = spawn(&mut w, 0, uid::MARINE, 10, 10);
        let other = spawn(&mut w, 0, uid::MARINE, 12, 10);
        let i = w.index_of(unit).unwrap();

        w.units[i].order_row = 0x25;
        w.units[i].target = Some(other);
        assert!(w.units[i].a32_waits_for_linked_target(true));

        assert!(!w.units[i].a32_waits_for_linked_target(false));
        w.units[i].target = None;
        assert!(!w.units[i].a32_waits_for_linked_target(true));
        w.units[i].target = Some(other);
        w.units[i].order_row = 0x24;
        assert!(!w.units[i].a32_waits_for_linked_target(true));
        w.units[i].order_row = 0x25;
        w.units[i].alive = false;
        assert!(!w.units[i].a32_waits_for_linked_target(true));
    }

    /// `0x005d2960`: the dying path drops orders and marks flags bit 0; the
    /// live path sets one `CUnit+0x123` bit.
    #[test]
    fn mask_bit_or_order_teardown_follows_the_dying_state() {
        let mut w = world();
        let unit = spawn(&mut w, 0, uid::MARINE, 10, 10);
        let i = w.index_of(unit).unwrap();
        w.units[i].queue.push_back(Order::Move { pos: (20, 20) });

        assert!(!w.units[i].a32_mark_state_or_drop_orders(3, false));
        assert_eq!(w.units[i].mask_123(), 1 << 3);
        assert_eq!(w.units[i].queue.len(), 1);

        assert!(w.units[i].a32_mark_state_or_drop_orders(5, true));
        assert!(w.units[i].queue.is_empty());
        assert_eq!(w.units[i].flags_93 & 1, 1);
    }

    /// `0x005d2ce0`: owner agreement, plus the silo missile-link check.
    #[test]
    fn silo_link_checks_the_owners() {
        let mut w = world();
        let silo = spawn(&mut w, 0, uid::NUCLEAR_SILO, 10, 10);
        let missile = spawn(&mut w, 0, uid::NUCLEAR_MISSILE, 12, 10);
        assert!(w.a32_silo_missile_owner_ok(silo, missile));

        let enemy_missile = spawn(&mut w, 1, uid::NUCLEAR_MISSILE, 14, 10);
        assert!(!w.a32_silo_missile_owner_ok(silo, enemy_missile));
        assert!(!w.a32_silo_missile_owner_ok(silo, 9999));

        // A strike that names the silo but a different owner is the
        // recovered mismatch case.
        w.a32_nuke_aim_at(missile, (12 * 256, 10 * 256), None);
        w.nuke_strikes.push(crate::sim::NukeStrike {
            owner: 1,
            ghost: missile,
            silo,
            target: (0, 0),
            timer: 5,
            dot: true,
            warned: false,
            missile: 0,
        });
        assert!(!w.a32_silo_missile_owner_ok(silo, missile));
    }

    /// `0x005d2d60`: the recovered three-way direction step.
    #[test]
    fn addon_direction_step_uses_the_recovered_thresholds() {
        let mut w = world();
        let unit = spawn(&mut w, 0, uid::SIEGE_TANK, 10, 10);
        let addon = spawn(&mut w, 0, uid::SIEGE_TANK_TURRET, 10, 10);
        let i = w.index_of(unit).unwrap();
        let a = w.index_of(addon).unwrap();
        w.units[i].addon = Some(addon);
        w.units[i].facing = 0;

        w.units[a].facing = 0x10;
        assert_eq!(w.a32_turn_toward_addon(unit), Some(0));
        w.units[a].facing = 0x60;
        assert_eq!(w.a32_turn_toward_addon(unit), Some(0x40));
        w.units[i].facing = 0;
        w.units[a].facing = 0xa0;
        assert_eq!(w.a32_turn_toward_addon(unit), Some(0xc0));
        // The exact -0x80 difference takes the short wrap-around branch.
        w.units[i].facing = 0;
        w.units[a].facing = 0x80;
        assert_eq!(w.a32_turn_toward_addon(unit), Some(0xa0));

        // No add-on link: the block never runs.
        let lone = spawn(&mut w, 0, uid::MARINE, 20, 10);
        assert_eq!(w.a32_turn_toward_addon(lone), None);
    }

    /// `0x005d3160` / `0x005d336c`: both cue paths require an empty order
    /// list, but only the first sets flags bit 0.
    #[test]
    fn drained_order_cues_differ_in_the_flags_bit() {
        let mut w = world();
        let unit = spawn(&mut w, 0, uid::MARINE, 10, 10);
        let i = w.index_of(unit).unwrap();

        assert_eq!(w.units[i].a32_idle_cue(0x21), Some(0x21));
        assert_eq!(w.units[i].flags_93 & 1, 1);

        w.units[i].flags_93 = 0;
        assert_eq!(w.units[i].a32_order_drained_cue(0x22), Some(0x22));
        assert_eq!(w.units[i].flags_93 & 1, 0);

        w.units[i].queue.push_back(Order::Move { pos: (20, 20) });
        assert_eq!(w.units[i].a32_idle_cue(0x21), None);
        assert_eq!(w.units[i].a32_order_drained_cue(0x22), None);
    }

    /// `0x005d31c0`: only the set latch reaches the cue.
    #[test]
    fn global_alert_cue_is_latched() {
        let mut w = world();
        let unit = spawn(&mut w, 0, uid::MARINE, 10, 10);
        let i = w.index_of(unit).unwrap();

        assert_eq!(w.units[i].a32_global_alert_cue(false, 0x33), None);
        assert_eq!(w.units[i].flags_93 & 1, 0);
        assert_eq!(w.units[i].a32_global_alert_cue(true, 0x33), Some(0x33));
        assert_eq!(w.units[i].flags_93 & 1, 1);
    }

    /// `0x005d31e2`: the binder only runs for a present dispatch value.
    #[test]
    fn finish_tail_binds_only_a_present_unit() {
        let mut w = world();
        let silo = spawn(&mut w, 0, uid::NUCLEAR_SILO, 10, 10);
        let missile = spawn(&mut w, 0, uid::NUCLEAR_MISSILE, 12, 10);

        assert!(!w.a32_bind_finished_unit(silo, None));
        assert!(w.a32_bind_finished_unit(silo, Some(missile)));
        assert!(!w.a32_bind_finished_unit(silo, Some(9999)));
    }

    /// `0x005d32bf`: bits 16 and 17 after the spawn tail.
    #[test]
    fn completion_flags_follow_the_subunit_row() {
        let mut w = world();
        let marine = spawn(&mut w, 0, uid::MARINE, 10, 10);
        let turret = spawn(&mut w, 0, uid::GOLIATH_TURRET, 12, 10);
        let data = Arc::clone(&w.data);
        let m = w.index_of(marine).unwrap();
        let t = w.index_of(turret).unwrap();

        w.units[m].a32_apply_completion_flags(&data);
        assert!(w.units[m].flag_16);
        assert!(w.units[m].flag_17);
        w.units[t].a32_apply_completion_flags(&data);
        assert!(w.units[t].flag_16);
        assert!(!w.units[t].flag_17);
    }

    /// `0x005d33a0`: row `0x17` clears the target/saved-order slots only when
    /// the row actually changes.
    #[test]
    fn order_row_17_clears_target_and_resume_order() {
        let mut w = world();
        let unit = spawn(&mut w, 0, uid::MARINE, 10, 10);
        let other = spawn(&mut w, 0, uid::MARINE, 12, 10);
        let i = w.index_of(unit).unwrap();
        w.units[i].order_row = 0x25;
        w.units[i].target = Some(other);
        w.units[i].resume_order = Some(Order::AttackMove { pos: (30, 30) });

        assert!(w.units[i].a32_set_order_row_17());
        assert_eq!(w.units[i].order_row, 0x17);
        assert_eq!(w.units[i].target, None);
        assert!(w.units[i].resume_order.is_none());
        assert!(!w.units[i].a32_set_order_row_17());
    }

    /// `0x005d36cf`: the aim point moves the Ghost and records the order row.
    #[test]
    fn nuke_aim_stores_point_and_target() {
        let mut w = world();
        let ghost = spawn(&mut w, 0, uid::GHOST, 10, 10);
        let target = spawn(&mut w, 1, uid::MARINE, 20, 20);
        let i = w.index_of(ghost).unwrap();

        assert!(w.a32_nuke_aim_at(ghost, (20 * 256, 22 * 256), Some(target)));
        assert_eq!(w.units[i].x, 20 * 256);
        assert_eq!(w.units[i].y, 22 * 256);
        assert_eq!(w.units[i].order_wait, 0x0f);
        assert_eq!(w.units[i].order_row, 0xb1);
        assert_eq!(w.units[i].target, Some(target));

        assert!(!w.a32_nuke_aim_at(ghost, (30 * 256, 30 * 256), None));
        assert_eq!(w.units[i].target, None);
        assert!(!w.a32_nuke_aim_at(9999, (0, 0), None));
    }

    /// `0x005d3754`: row `0xb1` is issued through the orders front door.
    #[test]
    fn nuke_target_order_uses_row_b1() {
        let mut w = world();
        let ghost = spawn(&mut w, 0, uid::GHOST, 10, 10);
        let target = spawn(&mut w, 1, uid::MARINE, 20, 20);

        assert!(w.a32_issue_nuke_target_order(ghost, target).is_ok());
        let i = w.index_of(ghost).unwrap();
        assert_eq!(w.units[i].order_row, 0xb1);
        assert!(w.a32_issue_nuke_target_order(9999, target).is_err());
    }

    /// `0x005d37e4`: the aim stage runs only while the wait byte is clear.
    #[test]
    fn nuke_aim_stage_respects_the_wait_byte() {
        let mut w = world();
        let ghost = spawn(&mut w, 0, uid::GHOST, 10, 10);
        let i = w.index_of(ghost).unwrap();
        assert_eq!(w.units[i].order_wait, 0);

        assert!(w.units[i].a32_stage_nuke_aim(Some((30 * 256, 40 * 256))));
        assert_eq!(w.units[i].x, 30 * 256);
        assert_eq!(w.units[i].y, 40 * 256);
        assert_eq!(w.units[i].order_wait, 0x0f);

        // Busy wait byte: no coordinate change.
        assert!(!w.units[i].a32_stage_nuke_aim(Some((1, 2))));
        assert_eq!(w.units[i].x, 30 * 256);

        // An empty aim still consumes the wait byte.
        w.units[i].order_wait = 0;
        assert!(!w.units[i].a32_stage_nuke_aim(None));
        assert_eq!(w.units[i].order_wait, 0x0f);
    }

    /// `0x005d3bb7`: only phase 6 with the parent phase rule reaches the cue.
    #[test]
    fn phase_six_cue_honours_the_parent_phase() {
        let mut w = world();
        let parent = spawn(&mut w, 0, uid::HATCHERY, 10, 10);
        let unit = spawn(&mut w, 0, uid::LARVA, 12, 10);
        let i = w.index_of(unit).unwrap();

        assert_eq!(w.units[i].a32_phase_six_idle_cue(5, None, 0x44), None);
        assert_eq!(w.units[i].a32_phase_six_idle_cue(6, None, 0x44), Some(0x44));

        w.units[i].connected_unit = Some(parent);
        assert_eq!(w.units[i].a32_phase_six_idle_cue(6, Some(4), 0x44), None);
        assert_eq!(
            w.units[i].a32_phase_six_idle_cue(6, Some(5), 0x44),
            Some(0x44)
        );

        w.units[i].queue.push_back(Order::Move { pos: (20, 20) });
        assert_eq!(w.units[i].a32_phase_six_idle_cue(6, Some(5), 0x44), None);
    }
}
