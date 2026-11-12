#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent31.rs
// ===========================================================================
// Translation batch 31.  Only this file is edited by the batch.
//
// The ledger window `0x005d0000..=0x005d1fff` contains eight
// `decompiled_uncited` rows.  Four of them (`FUN_005d1099`, `FUN_005d121a`,
// `FUN_005d13ef`, `FUN_005d18de`) are the obfuscated anti-decompilation block
// in front of the order dispatch table: their bodies overlap each other,
// consist of overlapping/self-modifying instructions, and `FUN_005d18de` is
// the computed-jump table itself (it lists the handler entry points of this
// block), so there is no routine to translate there.  The remaining
// four rows, plus the contiguous order-handler leaves that follow the
// table, are translated below.
//
// These leaves are the tail of the retail order dispatcher.  They only
// read/write `CUnit` records, so the existing [`SimUnit`] order, target,
// facing, timer and flag fields model them.  Presentation work (voice
// tables, sprite image ids, sprite flags) is not stored by the simulation
// and is called out in the doc comments instead of being invented as new
// state.

impl SimUnit {
    /// `0x005d1d90` (`FUN_005d1d90`, size 36): cleanup guard for a unit
    /// that carries an order target.
    ///
    /// Retail proceeds only when the target pointer at `CUnit+0xec` is
    /// non-null and the removal-path bit (`CUnit+0xdc` bit 0, readable as
    /// `flags & 1`) is clear.  It then calls the target-cleanup helper; that
    /// helper returns when the unit has no sprite / no primary order or is
    /// in the removal path, and otherwise reaches a noreturn abort.
    ///
    /// The Rust model stores the order target in [`SimUnit::target`] and has
    /// no separate removal-path bit, so `!alive` is the closest existing
    /// state for bit 0.  Returns whether the cleanup branch is entered.
    pub fn a31_order_target_cleanup_pending(&self) -> bool {
        self.target.is_some() && self.alive
    }

    /// `0x005d1e5b` (`FUN_005d1e5b`, size 30): voice cue, then the
    /// secondary order reset.
    ///
    /// The routine plays the per-type voice entry from the 232-byte table at
    /// `0x00f8f0f8` and immediately writes secondary order `0x17`, clearing
    /// the order-data slots `CUnit+0xe8`, `CUnit+0xec` and `CUnit+0xe2`.
    /// Voice playback and the byte table are not part of the simulation.
    ///
    /// The secondary order id at `CUnit+0xa6` is modelled by
    /// [`SimUnit::order_row`] (the same mapping the teardown batch uses) and
    /// `CUnit+0xec` is [`SimUnit::target`].  The other two slots have no
    /// dedicated field, so the modelled resume order is cleared as the
    /// closest analogue and the order-phase byte is left alone.  Returns
    /// whether a modelled field changed.
    pub fn a31_end_order_0x17(&mut self) -> bool {
        self.a31_reset_order_row_0x17()
    }

    /// `0x005d2160` (`FUN_005d2160`, size 51): true for a training order
    /// that is still linked to its target.
    ///
    /// All four retail conditions are reproduced: the secondary order byte
    /// at `CUnit+0xa6` is `0x25` (orders row 37, Train), the dynamic
    /// building bit (`CUnit+0xdc` bit 1) is set, the target pointer at
    /// `CUnit+0xec` is non-null, and the removal-path bit (bit 0) is clear.
    /// Bit 1 is modelled by the `units.dat` building flag; bit 0 by
    /// `!alive`.
    pub fn a31_train_order_linked(&self, data: &GameData) -> bool {
        self.order_row == 0x25
            && data.unit(self.type_id).is_building()
            && self.target.is_some()
            && self.alive
    }

    /// `0x005d37e4` (`FUN_005d37e4`, size 29): blocked-order retry arm.
    ///
    /// Entered when the preceding comparison succeeds.  Retail stores the
    /// 15-tick retry value into `CUnit+0x54` and the new target pointer into
    /// `CUnit+0x5c`, and only plays the acknowledgement cue when that
    /// pointer is non-null.  `CUnit+0x54` is [`SimUnit::order_wait`], the
    /// 15-tick Bunker Guard retry field; `CUnit+0x5c` is the modelled order
    /// target.
    ///
    /// Returns whether the cue would play (the target is non-null).
    pub fn a31_order_retry_wait(&mut self, retry: bool, target: Option<u32>) -> bool {
        if !retry {
            return false;
        }
        self.order_wait = 0x0f;
        self.target = target;
        target.is_some()
    }

    /// `0x005d2d60` (`FUN_005d2d60`, size 16): step the direction byte of
    /// the attached subunit (`CUnit+0x70`) toward `0x40`.
    ///
    /// The direction byte `CUnit+0x21` is written by the shared stepping
    /// helper with a 45-degree (`0x20`) dead band: a difference beyond
    /// `+/-0x20` snaps the value to `target +/- 0x20`, and a difference below
    /// `-0x7f` wraps by subtracting `0x60`.  [`SimUnit::facing`] is the
    /// simulated direction byte.
    ///
    /// The sim does not store the `CUnit+0x70` turret link, so the method
    /// applies the recovered step to the record it is called on; callers
    /// pass the subunit when one is linked.  Returns whether `facing`
    /// changed.
    pub fn a31_step_facing_toward(&mut self, target: u8) -> bool {
        let current = i32::from(self.facing);
        // Signed-byte delta, as the retail `char` subtraction produces.
        let raw = i32::from(target) - current;
        let diff = ((raw + 0x80).rem_euclid(0x100)) - 0x80;
        if diff < -0x7f {
            self.facing = self.facing.wrapping_sub(0x60);
            true
        } else if diff < -0x20 {
            self.facing = self.facing.wrapping_add(0x20).wrapping_add(diff as u8);
            true
        } else if diff > 0x20 {
            self.facing = target.wrapping_sub(0x20);
            true
        } else {
            false
        }
    }

    /// `0x005d2960` (`FUN_005d2960`, size 107): status-mask write that first
    /// checks the dying flag.
    ///
    /// When `CUnit+0xdc` bit 30 (the dying flag) is set, retail frees the
    /// order list at `CUnit+0x74` and leaves the `CUnit+0x123` mask alone;
    /// otherwise it sets the caller's bit in that mask.  Both paths then
    /// reach a noreturn helper, which is not reproduced.
    ///
    /// [`SimUnit::alive`] stands in for the dying bit, the modelled order
    /// list is [`SimUnit::queue`], and [`SimUnit::set_mask_123_bit`] stores
    /// the mask write.  Returns `true` for the mask branch.
    pub fn a31_status_bit_or_dying_reset(&mut self, bit: u8) -> bool {
        if !self.alive {
            self.queue.clear();
            return false;
        }
        self.set_mask_123_bit(bit);
        true
    }

    /// `0x005d32bf` (`FUN_005d32bf`, size 61): order-handler state write of
    /// the subunit/placement flags.
    ///
    /// Retail clears the removal-path bit (`CUnit+0xdc` bit 1), sets bit 16,
    /// and sets bit 17 only for non-subunits (`units.dat` flag `0x10`).
    /// It then refreshes the AI slot (`CUnit+0x100`), writes the sprite
    /// image byte `0xc` at sprite `+0xd`, sets the airborne bit (`+0xdc`
    /// bit 2), and recomputes the low bit of `CUnit+0x105` from
    /// `sprite[0xd] < 0xc`.
    ///
    /// Bits 16/17 are [`SimUnit::flag_16`]/[`SimUnit::flag_17`] and the
    /// `CUnit+0x105` low bit is [`SimUnit::flags_105`] bit 0; the AI slot,
    /// sprite image and airborne bit have no record field.  Returns `true`
    /// (the routine always writes the two flags).
    pub fn a31_set_placement_state_flags(
        &mut self,
        is_subunit: bool,
        image_below_12: bool,
    ) -> bool {
        self.flag_16 = true;
        self.flag_17 = !is_subunit;
        self.flags_105 = (self.flags_105 & 0xfe) | u8::from(image_below_12);
        true
    }

    /// `0x005d33a0` (`FUN_005d33a0`, size 8): secondary order reset only.
    ///
    /// The routine writes order `0x17` at `CUnit+0xa6`; the shared setter
    /// clears the order-data slots `CUnit+0xe8`, `CUnit+0xec` and
    /// `CUnit+0xe2` when the order actually changes.  The same mapping as
    /// [`SimUnit::a31_end_order_0x17`] applies.  Returns whether a modelled
    /// field changed.
    pub fn a31_reset_order_row_0x17(&mut self) -> bool {
        let mut changed = false;
        if self.order_row != 0x17 {
            self.order_row = 0x17;
            changed = true;
        }
        if self.target.take().is_some() {
            changed = true;
        }
        if self.resume_order.take().is_some() {
            changed = true;
        }
        changed
    }
}

impl World {
    /// `0x005d1fe4` (`FUN_005d1fe4`, size 302): placement-cursor alignment.
    ///
    /// The recovered arithmetic reads the ghost's box at `CUnit+0x58/+0x5a`
    /// and applies
    ///
    /// ```text
    /// x' = x - placement(ghost).x / 2 + placement(build).x / 2 + offset.x
    /// y' = y - placement(ghost).y / 2 + placement(build).y / 2 + offset.y
    /// ```
    ///
    /// where `placement(t)` is the per-type `units.dat` placement box
    /// (`0x00f90ad0`, entry `t*4`) and `offset(t)` is the second per-type
    /// pair the routine reads (`0x00f90cb8`).  When the ghost type and the
    /// build type are the same, which is how the placement ghost is created,
    /// the two placement terms cancel and only the offset remains.
    ///
    /// [`crate::data::UnitType::placement`] supplies the first term; the
    /// second table is not exposed by [`GameData`], so the caller passes the
    /// draw offset (zero when only the placement box matters).
    pub fn a31_placement_align(
        &self,
        ghost_type: u16,
        build_type: u16,
        cursor: (i32, i32),
        draw_offset: (i32, i32),
    ) -> (i32, i32) {
        let ghost = &self.data.unit(ghost_type).placement;
        let build = &self.data.unit(build_type).placement;
        let gx = i32::from(ghost.x);
        let gy = i32::from(ghost.y);
        let bx = i32::from(build.x);
        let by = i32::from(build.y);
        (
            cursor.0 - gx / 2 + bx / 2 + draw_offset.0,
            cursor.1 - gy / 2 + by / 2 + draw_offset.1,
        )
    }

    /// `0x005d1f16` (`FUN_005d1f16`, size 206): first placement tick of a
    /// building ghost.
    ///
    /// The recovered body gates on three things before the shared alignment
    /// and spawn block runs:
    ///
    /// 1. the dynamic building bit (`CUnit+0xdc` bit 1) must be set;
    /// 2. the octagonal distance between the fine position and the pixel
    ///    cursor must be zero (`distance(..., 0)`), i.e. the ghost already
    ///    sits on the cursor;
    /// 3. the destination write must report a change, which it does when
    ///    `CUnit+0x1c/+0x1e` differ from the requested point.
    ///
    /// A failure at gate 1 plays cue `0x47`, a failure at gate 2 plays cue
    /// `0x48` and the routine returns; neither cue is stored.  On success it
    /// sets order phase `CUnit+0x4e = 1`, aligns the ghost with
    /// [`World::a31_placement_align`] and attempts the placement.
    ///
    /// [`crate::data::UnitType::is_building`] stands in for bit 1,
    /// [`SimUnit::path_goal`] for the destination pair, and order phase 1 is
    /// reported by the `Some` return.  Returns the aligned pixel position
    /// and writes it to the ghost's `x`/`y`.
    pub fn a31_placement_ghost_accept(
        &mut self,
        ghost_id: u32,
        cursor: (i32, i32),
    ) -> Option<(i32, i32)> {
        let i = self.index_of(ghost_id)?;
        let type_id = self.units[i].type_id;
        let build_type = self.units[i].current_build_unit();
        if !self.data.unit(type_id).is_building() {
            return None;
        }
        if (self.units[i].x, self.units[i].y) != (fp(cursor.0), fp(cursor.1)) {
            return None;
        }
        if self.units[i].path_goal == Some(cursor) {
            return None;
        }
        let aligned = self.a31_placement_align(type_id, build_type, cursor, (0, 0));
        self.units[i].path_goal = Some(cursor);
        self.units[i].x = fp(aligned.0);
        self.units[i].y = fp(aligned.1);
        Some(aligned)
    }

    /// `0x005d2112` (`FUN_005d2112`, size 70): begin a linked secondary
    /// order.
    ///
    /// The routine plays the per-type voice entry, writes secondary order
    /// `0x25` (orders row 37, Train) at `CUnit+0xa6`, stores the other
    /// record in the order-target slot `CUnit+0xec`, plays the fixed cue
    /// `0x2c`, and writes the back-link `CUnit+0x80` of the target to this
    /// record.  The voice/cue playback and the sprite-side teardown are not
    /// stored.
    ///
    /// [`SimUnit::order_row`] holds `CUnit+0xa6`, [`SimUnit::target`] holds
    /// `CUnit+0xec`, and the target's [`SimUnit::connected_unit`] holds the
    /// `CUnit+0x80` back-link.  Returns `false` when either record is
    /// missing.
    pub fn a31_begin_linked_order(&mut self, unit_id: u32, target_id: u32) -> bool {
        let (Some(unit), Some(target)) = (self.index_of(unit_id), self.index_of(target_id)) else {
            return false;
        };
        self.units[unit].order_row = 0x25;
        self.units[unit].target = Some(target_id);
        self.units[target].connected_unit = Some(unit_id);
        true
    }

    /// `0x005d3e5c` (`FUN_005d3e5c`, size 162): spawn a Scanner Sweep at
    /// the source unit's position.
    ///
    /// The recovered body takes the source record's pixel position
    /// (`CUnit+0x58/+0x5a`), clamps it with the per-type `units.dat` box
    /// (`x` to `[left, map_width - right - 1]`, `y` to
    /// `[top, map_height - bottom - 0x21]`), allocates unit type `0x21`
    /// (`units.dat` row 33, Scanner Sweep) for the source owner, and aborts
    /// when the allocation fails.  The sound-id lookup and the sprite
    /// initialisation that follow allocation are presentation-only.
    ///
    /// [`World::spawn_px`] performs the allocation; the clamp uses
    /// [`crate::data::UnitType::dimensions`].  Returns the new unit id, or
    /// `None` when the source record or the allocation is missing.
    pub fn a31_spawn_scanner_sweep(&mut self, source_id: u32) -> Option<u32> {
        let i = self.index_of(source_id)?;
        let owner = self.units[i].owner as usize;
        let x = self.units[i].x / fp(1);
        let y = self.units[i].y / fp(1);
        let (left, top, right, bottom) = self.data.unit(uid::SCANNER_SWEEP).dimensions.extents();
        let map_w = self.terrain.width as i32 * 32;
        let map_h = self.terrain.height as i32 * 32;
        let x = x.max(left).min(map_w - right - 1);
        let y = y.max(top).min(map_h - bottom - 0x21);
        let idx = self.spawn_px(owner, uid::SCANNER_SWEEP, x, y).ok()?;
        Some(self.units[idx].id)
    }
}

#[cfg(test)]
mod tests_agent31 {
    use super::*;
    use crate::map::Terrain;
    use crate::sim::{Order, World};
    use std::sync::Arc;

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    /// `0x005d1d90`: the cleanup branch needs a target and a live record.
    #[test]
    fn order_target_cleanup_needs_target_and_live_unit() {
        let mut w = open_world();
        let marine = w.spawn_px(0, uid::MARINE, 100, 100).unwrap();
        let id = w.units[marine].id;

        assert!(!w.units[marine].a31_order_target_cleanup_pending());
        w.units[marine].target = Some(1);
        assert!(w.units[marine].a31_order_target_cleanup_pending());
        w.units[marine].alive = false;
        assert!(!w.units[marine].a31_order_target_cleanup_pending());
        assert_eq!(w.unit(id).unwrap().target, Some(1));
    }

    /// `0x005d1e5b`/`0x005d33a0`: order `0x17` resets the order-data slots.
    #[test]
    fn end_order_0x17_clears_links_once() {
        let mut w = open_world();
        let marine = w.spawn_px(0, uid::MARINE, 100, 100).unwrap();
        w.units[marine].order = Order::Move { pos: (4, 5) };
        w.units[marine].order_row = 6;
        w.units[marine].target = Some(9);
        w.units[marine].resume_order = Some(Order::AttackUnit { target: 9 });

        assert!(w.units[marine].a31_end_order_0x17());
        assert_eq!(w.units[marine].order_row, 0x17);
        assert_eq!(w.units[marine].target, None);
        assert_eq!(w.units[marine].resume_order, None);
        // The primary order is left alone: retail only writes `CUnit+0xa6`.
        assert_eq!(w.units[marine].order, Order::Move { pos: (4, 5) });
        // Nothing left to clear.
        assert!(!w.units[marine].a31_end_order_0x17());
    }

    /// `0x005d2160`: Train row, building, target and live bit all matter.
    #[test]
    fn train_order_link_requires_building_row_and_target() {
        let mut w = open_world();
        let cc = w.spawn_px(0, uid::COMMAND_CENTER, 400, 400).unwrap();
        w.units[cc].order_row = 0x25;
        w.units[cc].target = Some(42);
        assert!(w.units[cc].a31_train_order_linked(&w.data));

        w.units[cc].target = None;
        assert!(!w.units[cc].a31_train_order_linked(&w.data));
        w.units[cc].target = Some(42);
        w.units[cc].order_row = 0x17;
        assert!(!w.units[cc].a31_train_order_linked(&w.data));

        let marine = w.spawn_px(0, uid::MARINE, 120, 120).unwrap();
        w.units[marine].order_row = 0x25;
        w.units[marine].target = Some(42);
        assert!(!w.units[marine].a31_train_order_linked(&w.data));
        w.units[cc].order_row = 0x25;
        w.units[cc].alive = false;
        assert!(!w.units[cc].a31_train_order_linked(&w.data));
    }

    /// `0x005d37e4`: the retry arm stores 15 ticks and the new target.
    #[test]
    fn order_retry_waits_fifteen_ticks_with_target() {
        let mut w = open_world();
        let marine = w.spawn_px(0, uid::MARINE, 140, 140).unwrap();

        assert!(!w.units[marine].a31_order_retry_wait(false, Some(7)));
        assert_eq!(w.units[marine].order_wait, 0);
        assert_eq!(w.units[marine].target, None);

        assert!(w.units[marine].a31_order_retry_wait(true, Some(7)));
        assert_eq!(w.units[marine].order_wait, 0x0f);
        assert_eq!(w.units[marine].target, Some(7));

        // A null target still updates the wait counter but suppresses the cue.
        w.units[marine].order_wait = 0;
        assert!(!w.units[marine].a31_order_retry_wait(true, None));
        assert_eq!(w.units[marine].order_wait, 0x0f);
        assert_eq!(w.units[marine].target, None);
    }

    /// `0x005d2d60`: 45-degree dead band and the 0x60 wrap fix-up.
    #[test]
    fn facing_step_uses_dead_band_and_wrap() {
        let mut w = open_world();
        let marine = w.spawn_px(0, uid::MARINE, 160, 160).unwrap();

        w.units[marine].facing = 0x00;
        assert!(w.units[marine].a31_step_facing_toward(0x40));
        assert_eq!(w.units[marine].facing, 0x20);
        // Exactly 0x20 away is inside the dead band.
        assert!(!w.units[marine].a31_step_facing_toward(0x40));
        assert_eq!(w.units[marine].facing, 0x20);

        w.units[marine].facing = 0x20;
        assert!(w.units[marine].a31_step_facing_toward(0xf0));
        assert_eq!(w.units[marine].facing, 0x10);

        w.units[marine].facing = 0x00;
        assert!(w.units[marine].a31_step_facing_toward(0x90));
        assert_eq!(w.units[marine].facing, 0xb0);

        // A difference below -0x7f takes the 0x60 wrap branch.
        w.units[marine].facing = 0x00;
        assert!(w.units[marine].a31_step_facing_toward(0x80));
        assert_eq!(w.units[marine].facing, 0xa0);

        // Small deltas are left untouched.
        w.units[marine].facing = 0x10;
        assert!(!w.units[marine].a31_step_facing_toward(0x00));
        assert_eq!(w.units[marine].facing, 0x10);
    }

    /// `0x005d2960`: dying records clear the order list instead of the mask.
    #[test]
    fn status_bit_write_yields_to_dying_reset() {
        let mut w = open_world();
        let marine = w.spawn_px(0, uid::MARINE, 180, 180).unwrap();
        w.units[marine].queue.push_back(Order::Move { pos: (1, 2) });

        assert!(w.units[marine].a31_status_bit_or_dying_reset(3));
        assert_eq!(w.units[marine].mask_123(), 1 << 3);
        assert_eq!(w.units[marine].queue.len(), 1);

        w.units[marine].alive = false;
        assert!(!w.units[marine].a31_status_bit_or_dying_reset(4));
        assert_eq!(w.units[marine].mask_123(), 1 << 3);
        assert!(w.units[marine].queue.is_empty());
    }

    /// `0x005d32bf`: bits 16/17 and the `CUnit+0x105` low bit.
    #[test]
    fn placement_state_flags_follow_subunit_and_image() {
        let mut w = open_world();
        let marine = w.spawn_px(0, uid::MARINE, 200, 200).unwrap();
        w.units[marine].flags_105 = 0xa5;

        assert!(w.units[marine].a31_set_placement_state_flags(false, false));
        assert!(w.units[marine].flag_16);
        assert!(w.units[marine].flag_17);
        assert_eq!(w.units[marine].flags_105, 0xa4);

        assert!(w.units[marine].a31_set_placement_state_flags(true, true));
        assert!(w.units[marine].flag_16);
        assert!(!w.units[marine].flag_17);
        assert_eq!(w.units[marine].flags_105, 0xa5);
    }

    /// `0x005d1fe4`: same ghost/build type cancels the placement terms.
    #[test]
    fn placement_align_cancels_equal_types_and_keeps_offset() {
        let w = open_world();
        let depot = uid::SUPPLY_DEPOT;
        assert_eq!(
            w.a31_placement_align(depot, depot, (1000, 2000), (3, -4)),
            (1003, 1996)
        );

        let cc = uid::COMMAND_CENTER;
        let (gx, gy) = (w.unit_placement_x(cc), w.unit_placement_y(cc));
        let (bx, by) = (w.unit_placement_x(depot), w.unit_placement_y(depot));
        assert_eq!(
            w.a31_placement_align(cc, depot, (1000, 2000), (0, 0)),
            (1000 - gx / 2 + bx / 2, 2000 - gy / 2 + by / 2)
        );
    }

    /// `0x005d1f16`: building/cursor/destination gates and the aligned move.
    #[test]
    fn placement_ghost_accept_gates_and_moves_record() {
        let mut w = open_world();
        let ghost = w.spawn_px(0, uid::SUPPLY_DEPOT, 300, 400).unwrap();
        let id = w.units[ghost].id;

        // Cursor is not on the ghost.
        assert_eq!(w.a31_placement_ghost_accept(id, (301, 400)), None);
        // Move the ghost under the cursor: the same-type alignment is a no-op.
        assert_eq!(
            w.a31_placement_ghost_accept(id, (300, 400)),
            Some((300, 400))
        );
        assert_eq!(w.units[ghost].path_goal, Some((300, 400)));
        assert_eq!((w.units[ghost].x, w.units[ghost].y), (fp(300), fp(400)));
        // The destination pair is now unchanged, so the second tick returns.
        assert_eq!(w.a31_placement_ghost_accept(id, (300, 400)), None);

        let marine = w.spawn_px(0, uid::MARINE, 100, 100).unwrap();
        let marine_id = w.units[marine].id;
        assert_eq!(w.a31_placement_ghost_accept(marine_id, (100, 100)), None);
        assert_eq!(w.a31_placement_ghost_accept(9999, (0, 0)), None);
    }

    /// `0x005d2112`: row 0x25, order target and the target back-link.
    #[test]
    fn linked_order_writes_both_directions() {
        let mut w = open_world();
        let cc = w.spawn_px(0, uid::COMMAND_CENTER, 500, 500).unwrap();
        let scv = w.spawn_px(0, uid::SCV, 540, 500).unwrap();
        let (cc_id, scv_id) = (w.units[cc].id, w.units[scv].id);

        assert!(w.a31_begin_linked_order(cc_id, scv_id));
        assert_eq!(w.units[cc].order_row, 0x25);
        assert_eq!(w.units[cc].target, Some(scv_id));
        assert_eq!(w.units[scv].connected_unit, Some(cc_id));

        assert!(!w.a31_begin_linked_order(cc_id, 9999));
        assert!(!w.a31_begin_linked_order(9999, scv_id));
        assert_eq!(w.units[cc].order_row, 0x25);
    }

    /// `0x005d3e5c`: Scanner Sweep spawn at the source position with the
    /// per-type `units.dat` position clamp.
    #[test]
    fn scanner_sweep_spawns_at_source_and_clamps_to_map() {
        let mut w = open_world();
        let scv = w.spawn_px(1, uid::SCV, 100, 120).unwrap();
        let source_id = w.units[scv].id;

        let sweep_id = w.a31_spawn_scanner_sweep(source_id).unwrap();
        let sweep = w.index_of(sweep_id).unwrap();
        assert_eq!(w.units[sweep].type_id, uid::SCANNER_SWEEP);
        assert_eq!(w.units[sweep].owner, 1);
        assert_eq!((w.units[sweep].x, w.units[sweep].y), (fp(100), fp(120)));

        // Off-map source: the clamped box limits are the recovered bounds.
        let far = w.spawn_px(0, uid::SCV, 5000, 5000).unwrap();
        let far_id = w.units[far].id;
        let clamped_id = w.a31_spawn_scanner_sweep(far_id).unwrap();
        let clamped = w.index_of(clamped_id).unwrap();
        let (left, top, right, bottom) = w.data.unit(uid::SCANNER_SWEEP).dimensions.extents();
        let map_w = w.terrain.width as i32 * 32;
        let map_h = w.terrain.height as i32 * 32;
        assert_eq!(w.units[clamped].x / fp(1), map_w - right - 1);
        assert_eq!(w.units[clamped].y / fp(1), map_h - bottom - 0x21);
        assert!(w.units[clamped].x / fp(1) >= left);
        assert!(w.units[clamped].y / fp(1) >= top);

        assert_eq!(w.a31_spawn_scanner_sweep(9999), None);
    }
}
