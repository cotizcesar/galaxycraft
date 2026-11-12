#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent37.rs
// ===========================================================================
// Translation batch 37.  Only this file is edited by the batch.
//
// Seventeen `decompiled_uncited` ledger rows in
// `0x005dc000..=0x005ddfff` are reconstructed here.  The window is the tail
// of the spell/status helpers and the `CUnit` order-handler cluster:
//
// * `0x005dc1e0`, `0x005dc1e9` - the two entry points that clamp a point and
//   create a Dark Swarm (`units.dat` row `0xca`), then set the hidden-status
//   bit and the 900-tick removal timer.
// * `0x005dc510` - the ensnare update gate: a pair of different units where
//   the first is not a building, not capability-flagged and not completed
//   triggers the fixed `0x4b` update count.
// * `0x005dcbf0` - the two-point margin/walk/build test (Larva margins).
// * `0x005dccc0`, `0x005dcd45`, `0x005dd048`, `0x005dd066`,
//   `0x005dd2c6`, `0x005dd315`, `0x005dd5d4` - Burrow-family order phases
//   and their cleanup/transition tails.
// * `0x005dd807`, `0x005dd817`, `0x005dd865`, `0x005dd8bc`,
//   `0x005dd92c`, `0x005dd97f` - the movement-step guards and the
//   destination dispatch wrappers.
//
// The decompiler split the order handlers at call and jump targets, so
// several rows are fragments rather than complete functions.  Each method
// below names its exact ledger address and reproduces the recovered branch.
// Retail state that [`SimUnit`] does not model (`CUnit+0x4e` phase,
// `CUnit+0x4f` end flag, the sprite image/direction fields, the
// `0x00f8de50`/`0x00f8f2c8` per-type tables, and the unresolved indirect
// callback) is supplied by the caller or returned to it.  No new persistent
// fields are added.

/// Result of the `0x005dd315` phase transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A37AnimationStep {
    /// Animation id passed through to the sprite (retail reads it from the
    /// per-type `0x00f8de50` table).
    pub animation: u16,
    /// Sprite direction byte read from `CUnit+0x21`.
    pub sprite_direction: u8,
    /// Value stored in the `CUnit+0x4e` order-phase byte.
    pub phase: u8,
}

/// Result of the `0x005dd048`/`0x005dd066` Burrowed-handler cleanup.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A37BurrowedCleanup {
    /// Sprite animation id: `0x1f8` for a Lurker, `300` for every other
    /// type (the routine picks the value from the Lurker-only test).
    pub animation: u16,
    /// Retail order row enqueued at the end of the cleanup (`0x75`).
    pub order_row: u8,
    /// Status cue pairs emitted by the status refresh, in the retail order:
    /// irradiate (`0x118`), ensnare (`0x116`), plague (`0x11a`).
    pub status_cues: Vec<(u16, u16)>,
}

impl World {
    /// Clamp a point to the playable map rectangle using the per-type
    /// `units.dat` extents.  The retail clamp takes the margin row by type id
    /// and adds the retail 32-pixel bottom margin to the vertical bound; this
    /// is the Rust equivalent for the two callers in this batch (`0x23`
    /// Larva and `0xca` Dark Swarm).
    fn a37_clamped_point(&self, type_id: u16, pos: (i32, i32)) -> (i32, i32) {
        let (left, top, right, bottom) = self.data.unit(type_id).dimensions.extents();
        let width = self.terrain.width as i32 * TILE_PX;
        let height = self.terrain.height as i32 * TILE_PX;
        let min_x = left.max(0);
        let max_x = (width - right - 1).max(min_x);
        let min_y = top.max(0);
        let max_y = (height - bottom - 0x21).max(min_y);
        (pos.0.clamp(min_x, max_x), pos.1.clamp(min_y, max_y))
    }

    /// `0x005dc1e0`: create a Dark Swarm (`units.dat` row `0xca`) at a
    /// clamped point.
    ///
    /// The routine clamps the two caller coordinates with the `0xca` margin
    /// row, builds the creation properties for type `0xca` with the recovered
    /// creation count `0x0b`, calls the unit constructor and, on success, sets
    /// the hidden-status bit (`CUnit+0xdc` bit 21), writes the sprite byte and
    /// lowers `CUnit+0x110` to 900 (the removal-timer write keeps the smaller
    /// of the current value and its argument).  A failed creation runs the
    /// caller's error callback; the Rust form reports the error instead.
    ///
    /// The creation status bit and the removal timer are the modelled
    /// effects; [`SimUnit::visibility_mask`] is untouched by this routine.
    pub fn a37_spawn_dark_swarm(&mut self, owner: usize, pos: (i32, i32)) -> Result<u32, SimError> {
        self.a37_spawn_dark_swarm_clamped(owner, pos)
    }

    /// `0x005dc1e9`: second entry point of the Dark Swarm creation body.
    ///
    /// The decompiler split the same source routine at a register-passed
    /// argument sequence: `0x005dc1e9` takes the point through `EAX`/`EBP`
    /// instead of the two `short` stack slots used by `0x005dc1e0`.  The
    /// observable sequence is identical, so it shares the clamped spawn
    /// implementation.
    pub fn a37_spawn_dark_swarm_reg(
        &mut self,
        owner: usize,
        pos: (i32, i32),
    ) -> Result<u32, SimError> {
        self.a37_spawn_dark_swarm_clamped(owner, pos)
    }

    fn a37_spawn_dark_swarm_clamped(
        &mut self,
        owner: usize,
        pos: (i32, i32),
    ) -> Result<u32, SimError> {
        if owner >= self.players.len() {
            return Err(SimError::BadOwner);
        }
        let point = self.a37_clamped_point(uid::DARK_SWARM, pos);
        let idx = self.spawn_px(owner, uid::DARK_SWARM, point.0, point.1)?;
        let unit = &mut self.units[idx];
        // The hidden/status bit toggled by the creation tail.
        unit.flag_21 = true;
        // The 900-tick removal floor; 0 means "no timer yet".
        if unit.death_timer == 0 || 900 < unit.death_timer {
            unit.death_timer = 900;
        }
        Ok(unit.id)
    }

    /// `0x005dc510`: ensnare update gate.
    ///
    /// When the two ids differ and the first unit is not a building
    /// (`units.dat` flag 0x1 for ids below `0xe4`), does not carry
    /// `CUnit+0xdc` bit 26 and is not marked completed (`CUnit+0xdc`
    /// bit 4), the routine writes the fixed `0x4b` update count to
    /// `CUnit+0x116` (`Status::apply_ensnare`).  The routine always returns
    /// zero; the Rust form returns whether the write happened so the caller
    /// can observe the branch.
    pub fn a37_ensnare_gate(&mut self, unit_id: u32, other_id: u32) -> bool {
        if unit_id == other_id {
            return false;
        }
        let Some(i) = self.index_of(unit_id) else {
            return false;
        };
        if self.unit_is_building_flagged(self.units[i].type_id) {
            return false;
        }
        if self.units[i].flag_26() || self.units[i].is_completed_flag() {
            return false;
        }
        self.units[i].status.apply_ensnare();
        true
    }

    /// `0x005dcbf0`: two-point Larva margin/walk/build test.
    ///
    /// The base point must already lie inside the `0x23` (Larva) margin
    /// rectangle and its tile must pass the `0x400000` terrain query.  The
    /// point shifted by `(dx, dy)` must also remain inside the margins and
    /// its tile must pass the `0x8000000` query.  The two terrain queries are
    /// mapped to the stored walk and build grids
    /// ([`crate::map::Terrain::walkable_px`] and
    /// [`crate::map::Terrain::buildable_px`]); the numeric flag names are not
    /// recovered by the decompiler.
    pub fn a37_offset_spot_valid(&self, base: (i32, i32), offset: (i32, i32)) -> bool {
        if self.a37_clamped_point(0x23, base) != base {
            return false;
        }
        if !self.terrain.walkable_px(base.0, base.1) {
            return false;
        }
        let shifted = (base.0 + offset.0, base.1 + offset.1);
        if self.a37_clamped_point(0x23, shifted) != shifted {
            return false;
        }
        self.terrain.buildable_px(shifted.0, shifted.1)
    }

    /// `0x005dd807`: horizontal step guard.
    ///
    /// The routine aborts when the destination X equals the current X, and
    /// when the destination tile fails the `0x400000` terrain query.  It then
    /// checks a path-link field pair and aborts on a repeated waypoint; those
    /// two 32-bit fields are not modelled.  The Rust form reports whether the
    /// destination passes the recovered checks.
    pub fn a37_step_x_clear(&self, unit_id: u32, dest: (i32, i32)) -> bool {
        let Some(i) = self.index_of(unit_id) else {
            return false;
        };
        let cur_x = self.units[i].x / fp(1);
        dest.0 != cur_x && self.terrain.walkable_px(dest.0, dest.1)
    }

    /// `0x005dd817`: vertical step guard.
    ///
    /// Same structure as `0x005dd807`, but the abort condition compares the
    /// destination Y with the current Y.  Returns whether the destination
    /// passes both the delta test and the `0x400000` terrain query.
    pub fn a37_step_y_clear(&self, unit_id: u32, dest: (i32, i32)) -> bool {
        let Some(i) = self.index_of(unit_id) else {
            return false;
        };
        let cur_y = self.units[i].y / fp(1);
        dest.1 != cur_y && self.terrain.walkable_px(dest.0, dest.1)
    }

    /// `0x005dd865`: single-tile terrain query.
    ///
    /// The routine performs the `0x400000` query at the caller's point.  A
    /// zero result calls the iscript frame helper and returns; a nonzero
    /// result falls into the fatal path.  The Rust form returns the query
    /// result directly (true = clear/walkable tile).
    pub fn a37_tile_clear(&self, pos: (i32, i32)) -> bool {
        self.terrain.walkable_px(pos.0, pos.1)
    }

    /// `0x005dd8bc`: directional 10-pixel vertical step.
    ///
    /// Bit 7 of the direction byte adds 10 to the vertical coordinate; the
    /// horizontal coordinate is unchanged.  The destination must be inside
    /// the map, pass the `0x400000` terrain query and the unit must have no
    /// `CUnit+0x80` link (`SimUnit::connected_unit`), after which the move
    /// dispatch runs.  Returns whether the move was issued.
    pub fn a37_vertical_step(
        &mut self,
        unit_id: u32,
        x: i32,
        y: i32,
        direction: u8,
    ) -> Result<bool, SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        let dy = if direction & 0x80 != 0 { 10 } else { 0 };
        let dest = (x, y.saturating_add(dy));
        let width = self.terrain.width as i32 * TILE_PX;
        let height = self.terrain.height as i32 * TILE_PX;
        if dest.0 < 0 || dest.1 < 0 || dest.0 >= width || dest.1 >= height {
            return Ok(false);
        }
        if !self.terrain.walkable_px(dest.0, dest.1) {
            return Ok(false);
        }
        if self.units[i].connected_unit.is_some() {
            return Ok(false);
        }
        self.issue(unit_id, Order::Move { pos: dest })?;
        Ok(true)
    }

    /// `0x005dd92c`: guarded move dispatch.
    ///
    /// The `CUnit+0x80` test returns 1 when the unit has no link and reaches
    /// the fatal path otherwise, so the following move dispatch only runs for
    /// an unlinked unit.  Returns whether the move was issued.
    pub fn a37_move_if_unlinked(
        &mut self,
        unit_id: u32,
        dest: (i32, i32),
    ) -> Result<bool, SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        if self.units[i].connected_unit.is_some() {
            return Ok(false);
        }
        self.issue(unit_id, Order::Move { pos: dest })?;
        Ok(true)
    }

    /// `0x005dd97f`: destination dispatch wrapper.
    ///
    /// The whole body is a tail call to the move helper with the packed
    /// destination.  The Rust form issues the typed move order.
    pub fn a37_move_to_point(&mut self, unit_id: u32, dest: (i32, i32)) -> Result<(), SimError> {
        self.issue(unit_id, Order::Move { pos: dest })
    }

    /// `0x005dd5d4`: default-order transition at rest.
    ///
    /// The routine dispatches the per-unit-type default order row read from
    /// `0x00f8f2c8`, then checks the move destination against the unit's own
    /// position (`CUnit+0x58`/`+0x5a`), which always matches, and on success
    /// sets `CUnit+0xdc` bit 12 (the burrow/hidden bit, modelled by
    /// [`SimUnit::status`] `burrowed`) and the `CUnit+0x4e` phase `2`.
    ///
    /// The default row is a caller input because the table is not part of
    /// [`crate::data::GameData`].  Returns the stored phase.
    pub fn a37_default_order_at_rest(
        &mut self,
        unit_id: u32,
        default_order_row: u8,
    ) -> Result<u8, SimError> {
        self.issue_order_id(unit_id, default_order_row, None, None)?;
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        self.units[i].status.burrowed = true;
        Ok(2)
    }
}

impl SimUnit {
    /// `0x005dccc0`: Burrow-family retry gate.
    ///
    /// The handler proceeds when the current order row (`CUnit+0x4d`, the
    /// recovered [`SimUnit::order_row`]) is `0x74` or `0x75`, or when the
    /// unit is a Lurker (`units.dat` row `0x67`).  A Lurker additionally gets
    /// the hidden/burrow bit (`CUnit+0xdc` bit 12) set.  Retail then sets
    /// `CUnit+0x93` bit 0 and enqueues row `0x76`.
    ///
    /// Returns `Some(0x76)` when the transition runs, with
    /// [`SimUnit::flags_93`] bit 0 applied; `None` means the handler returned
    /// without an order.
    pub fn a37_burrow_retry(&mut self) -> Option<u8> {
        if self.order_row != 0x74 && self.order_row != 0x75 && self.type_id != uid::LURKER {
            return None;
        }
        if self.type_id == uid::LURKER {
            // CUnit+0xdc bit 12; SimUnit collapses the 0x3000 burrow bits
            // into `status.burrowed`.
            self.status.burrowed = true;
        }
        self.flags_93 |= 1;
        Some(0x76)
    }

    /// `0x005dcd45`: order countdown store.
    ///
    /// The routine writes `value - 1` to the 16-bit countdown at
    /// `CUnit+0xac` ([`SimUnit::field_0ac`]) and, when the *incoming* value
    /// was zero, runs the linked-target cleanup and the fatal end-of-order
    /// path.  Returns whether the expiry branch was taken.
    pub fn a37_set_order_countdown(&mut self, value: u16) -> bool {
        self.field_0ac = value.wrapping_sub(1);
        value == 0
    }

    /// `0x005dd048`: Burrowed-handler cleanup tail.
    ///
    /// `end_flag` is `CUnit+0x4f` bit 2, which has no [`SimUnit`] field.  When
    /// it is set the routine:
    ///
    /// * clears that bit;
    /// * toggles on `CUnit+0xdc` bit 21 ([`SimUnit::flag_21`]);
    /// * rewrites `CUnit+0x105` bit 0 from the sprite-state test and clears
    ///   the byte at `CUnit+0x97`;
    /// * ORs `0x810` into `CUnit+0xdc` (bit 4 is [`SimUnit::completed`],
    ///   bit 11 has no field);
    /// * writes the hidden bit into the sprite type word;
    /// * picks animation `0x1f8` for a Lurker and `300` otherwise, writes the
    ///   sprite direction byte and the `0x6d` overlay byte;
    /// * sets `CUnit+0xe4` to `0x80000000`
    ///   ([`SimUnit::visibility_mask`]);
    /// * refreshes the irradiate/ensnare/plague status cues;
    /// * sets `CUnit+0x93` bit 0 and enqueues row `0x75`.
    ///
    /// The sprite reads/writes and the unmodelled bit 11/`0x97` bytes are
    /// returned or documented rather than invented.  `sprite_state` is the
    /// value the sprite getter supplies to the `CUnit+0x105` bit-0 test.
    pub fn a37_burrowed_cleanup(
        &mut self,
        end_flag: bool,
        sprite_state: u8,
    ) -> Option<A37BurrowedCleanup> {
        self.a37_burrowed_cleanup_impl(end_flag, sprite_state)
    }

    /// `0x005dd066`: second copy of the Burrowed-handler cleanup.
    ///
    /// The unit is read from the handler's saved local slot instead of `ESI`
    /// and the body starts with a virtual dispatch on the recovered object;
    /// the state writes and cue selection are identical to `0x005dd048`.
    pub fn a37_burrowed_cleanup_saved(
        &mut self,
        end_flag: bool,
        sprite_state: u8,
    ) -> Option<A37BurrowedCleanup> {
        self.a37_burrowed_cleanup_impl(end_flag, sprite_state)
    }

    fn a37_burrowed_cleanup_impl(
        &mut self,
        end_flag: bool,
        sprite_state: u8,
    ) -> Option<A37BurrowedCleanup> {
        if !end_flag {
            return None;
        }
        self.flag_21 = true;
        self.flags_105 = (self.flags_105 & 0xfe) | u8::from(sprite_state < 0xc);
        // CUnit+0xdc |= 0x810; bit 4 is the modelled completed latch.
        self.completed = true;
        self.visibility_mask = 0x8000_0000;
        self.flags_93 |= 1;
        let mut status_cues = Vec::new();
        if self.status.irradiate_ticks != 0 {
            status_cues.push((0x17c, 0x17e));
        }
        if self.status.ensnare_ticks != 0 {
            status_cues.push((0x180, 0x182));
        }
        if self.status.plague_ticks != 0 {
            status_cues.push((0x184, 0x186));
        }
        Some(A37BurrowedCleanup {
            animation: if self.type_id == uid::LURKER {
                0x1f8
            } else {
                300
            },
            order_row: 0x75,
            status_cues,
        })
    }

    /// `0x005dd2c6`: phase store after a virtual dispatch.
    ///
    /// The routine runs the recovered object's virtual slot `0x10`, writes
    /// `1` to the `CUnit+0x4e` order-phase byte and unwinds.  The phase byte
    /// is not stored in [`SimUnit`], so it is returned for the caller's
    /// handler state; the virtual dispatch has no modelled effect.
    pub fn a37_phase_after_virtual(&self) -> u8 {
        1
    }

    /// `0x005dd315`: phase store after the idle animation is installed.
    ///
    /// The routine looks the animation id up in the per-type `0x00f8de50`
    /// table, passes it to the sprite animation setter, refreshes the
    /// sprite, copies the sprite direction byte (`CUnit+0x21`) through the
    /// direction setter and stores `3` in the `CUnit+0x4e` phase byte.
    ///
    /// The per-type table and the sprite object are not part of the model,
    /// so the animation id and direction are caller inputs and the phase is
    /// returned with them.
    pub fn a37_phase_after_animation(
        &self,
        animation: u16,
        sprite_direction: u8,
    ) -> A37AnimationStep {
        A37AnimationStep {
            animation,
            sprite_direction,
            phase: 3,
        }
    }
}

#[cfg(test)]
mod tests_agent37 {
    use super::*;
    use crate::data::GameData;
    use crate::map::Terrain;
    use crate::sim::{ENSNARE_UPDATES, World};
    use std::sync::Arc;

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, type_id: u16, x: i32, y: i32) -> u32 {
        let i = world.spawn_px(0, type_id, x, y).unwrap();
        world.units[i].id
    }

    #[test]
    fn dark_swarm_spawn_clamps_to_the_margin_rectangle() {
        let mut w = open_world();
        let (left, top, right, bottom) = w.data.unit(uid::DARK_SWARM).dimensions.extents();
        let width = w.terrain.width as i32 * TILE_PX;
        let height = w.terrain.height as i32 * TILE_PX;

        // Low corner: the clamp lifts the point to the type's left/top
        // extents.
        let id = w.a37_spawn_dark_swarm(0, (-1000, -1000)).unwrap();
        let u = w.unit(id).unwrap();
        assert_eq!(u.x, left.max(0) * fp(1));
        assert_eq!(u.y, top.max(0) * fp(1));
        assert!(u.flag_21, "the creation tail sets CUnit+0xdc bit 21");
        assert_eq!(u.death_timer, 900);

        // High corner: width - right - 1 / height - bottom - 0x21.
        let id = w.a37_spawn_dark_swarm_reg(0, (100_000, 100_000)).unwrap();
        let u = w.unit(id).unwrap();
        assert_eq!(u.x, (width - right - 1) * fp(1));
        assert_eq!(u.y, (height - bottom - 0x21) * fp(1));
        assert_eq!(u.type_id, uid::DARK_SWARM);
        assert_eq!(u.owner, 0);

        // Bad owner is reported rather than silently stored.
        assert_eq!(
            w.a37_spawn_dark_swarm(9, (100, 100)),
            Err(SimError::BadOwner)
        );
    }

    #[test]
    fn ensnare_gate_needs_two_unfinished_nonbuildings() {
        let mut w = open_world();
        let marine = spawn(&mut w, uid::MARINE, 640, 640);
        let other = spawn(&mut w, uid::MARINE, 700, 640);
        let i = w.index_of(marine).unwrap();
        // Fresh completed units are rejected.
        w.units[i].completed = true;
        assert!(!w.a37_ensnare_gate(marine, other));
        w.units[i].completed = false;
        assert!(w.a37_ensnare_gate(marine, other));
        assert_eq!(w.units[i].status.ensnare_ticks, ENSNARE_UPDATES);

        // Same id, building flag and capability bit 26 all block the write.
        w.units[i].status.ensnare_ticks = 0;
        assert!(!w.a37_ensnare_gate(marine, marine));
        assert_eq!(w.units[i].status.ensnare_ticks, 0);
        w.units[i].flag_26 = true;
        assert!(!w.a37_ensnare_gate(marine, other));
        assert_eq!(w.units[i].status.ensnare_ticks, 0);

        let depot = spawn(&mut w, uid::SUPPLY_DEPOT, 800, 800);
        let d = w.index_of(depot).unwrap();
        w.units[d].completed = false;
        assert!(!w.a37_ensnare_gate(depot, other));
        assert_eq!(w.units[d].status.ensnare_ticks, 0);
    }

    #[test]
    fn offset_spot_checks_margins_and_both_terrain_grids() {
        let mut w = open_world();
        assert!(w.a37_offset_spot_valid((640, 640), (10, 0)));
        assert!(w.a37_offset_spot_valid((640, 640), (-10, 10)));

        // A point outside the Larva margin rectangle is clamped, so the
        // routine rejects it.
        assert!(!w.a37_offset_spot_valid((-10, 640), (0, 0)));
        assert!(!w.a37_offset_spot_valid((640, 100_000), (0, 0)));
        // The shifted point must also stay inside the margins.
        assert!(!w.a37_offset_spot_valid((640, 640), (100_000, 0)));

        // The base tile walk query (0x400000) blocks the check.
        w.terrain.set_walkable_cell(640 / 8, 640 / 8, false);
        assert!(!w.a37_offset_spot_valid((640, 640), (10, 0)));
    }

    #[test]
    fn burrow_retry_matches_rows_and_the_lurker_special_case() {
        let mut w = open_world();
        let marine = spawn(&mut w, uid::MARINE, 640, 640);
        let i = w.index_of(marine).unwrap();
        w.units[i].order_row = 0;
        assert_eq!(w.units[i].a37_burrow_retry(), None);

        w.units[i].order_row = 0x74;
        assert_eq!(w.units[i].a37_burrow_retry(), Some(0x76));
        assert_eq!(w.units[i].flags_93 & 1, 1);
        assert!(!w.units[i].status.burrowed, "only a Lurker sets the bit");

        w.units[i].order_row = 0x75;
        assert_eq!(w.units[i].a37_burrow_retry(), Some(0x76));

        let lurker = spawn(&mut w, uid::LURKER, 700, 700);
        let l = w.index_of(lurker).unwrap();
        w.units[l].order_row = 0;
        assert_eq!(w.units[l].a37_burrow_retry(), Some(0x76));
        assert!(w.units[l].status.burrowed);
        assert_eq!(w.units[l].flags_93 & 1, 1);
    }

    #[test]
    fn order_countdown_stores_value_minus_one() {
        let mut w = open_world();
        let id = spawn(&mut w, uid::MARINE, 640, 640);
        let i = w.index_of(id).unwrap();
        assert!(!w.units[i].a37_set_order_countdown(3));
        assert_eq!(w.units[i].field_0ac, 2);
        assert!(w.units[i].a37_set_order_countdown(0));
        assert_eq!(w.units[i].field_0ac, 0xffff);
    }

    #[test]
    fn burrowed_cleanup_writes_the_modelled_state_and_cues() {
        let mut w = open_world();
        let lurker = spawn(&mut w, uid::LURKER, 640, 640);
        let l = w.index_of(lurker).unwrap();
        w.units[l].flag_21 = false;
        w.units[l].completed = false;
        w.units[l].flags_105 = 0x40;
        w.units[l].visibility_mask = u32::MAX;
        w.units[l].status.irradiate_ticks = 4;
        w.units[l].status.ensnare_ticks = 0;
        w.units[l].status.plague_ticks = 2;

        // The end flag is clear: nothing changes.
        assert_eq!(w.units[l].a37_burrowed_cleanup(false, 3), None);
        assert_eq!(w.units[l].visibility_mask, u32::MAX);

        let step = w.units[l].a37_burrowed_cleanup(true, 3).unwrap();
        assert!(w.units[l].flag_21);
        assert!(w.units[l].completed);
        assert_eq!(w.units[l].visibility_mask, 0x8000_0000);
        assert_eq!(w.units[l].flags_105, 0x41);
        assert_eq!(w.units[l].flags_93 & 1, 1);
        assert_eq!(step.animation, 0x1f8);
        assert_eq!(step.order_row, 0x75);
        assert_eq!(step.status_cues, vec![(0x17c, 0x17e), (0x184, 0x186)]);

        // The saved-unit copy uses the same state writes; a non-Lurker gets
        // the 300 animation id.
        let marine = spawn(&mut w, uid::MARINE, 700, 640);
        let m = w.index_of(marine).unwrap();
        let step = w.units[m].a37_burrowed_cleanup_saved(true, 11).unwrap();
        assert_eq!(step.animation, 300);
        assert_eq!(w.units[m].flags_105, 1, "state 11 is below 0xc");
    }

    #[test]
    fn phase_stores_are_reported() {
        let mut w = open_world();
        let id = spawn(&mut w, uid::MARINE, 640, 640);
        let u = w.unit(id).unwrap();
        assert_eq!(u.a37_phase_after_virtual(), 1);
        let step = u.a37_phase_after_animation(0x1234, 0x0f);
        assert_eq!(step.animation, 0x1234);
        assert_eq!(step.sprite_direction, 0x0f);
        assert_eq!(step.phase, 3);
    }

    #[test]
    fn default_order_sets_the_hidden_bit_and_returns_phase_two() {
        let mut w = open_world();
        let lurker = spawn(&mut w, uid::LURKER, 640, 640);
        // Row 5 is the idle/ignore row in the shipped order table.
        let phase = w.a37_default_order_at_rest(lurker, 5).unwrap();
        assert_eq!(phase, 2);
        let u = w.unit(lurker).unwrap();
        assert!(u.status.burrowed);
        assert_eq!(u.order, Order::Stop);
        assert_eq!(
            w.a37_default_order_at_rest(0xdead_beef, 5),
            Err(SimError::BadUnit)
        );
    }

    #[test]
    fn step_guards_require_an_axis_delta_and_a_walkable_tile() {
        let mut w = open_world();
        let id = spawn(&mut w, uid::MARINE, 640, 640);
        assert!(!w.a37_step_x_clear(id, (640, 700)));
        assert!(!w.a37_step_y_clear(id, (700, 640)));
        assert!(w.a37_step_x_clear(id, (700, 640)));
        assert!(w.a37_step_y_clear(id, (640, 700)));
        assert!(w.a37_tile_clear((700, 640)));

        w.terrain.set_walkable_cell(700 / 8, 640 / 8, false);
        assert!(!w.a37_step_x_clear(id, (700, 640)));
        assert!(!w.a37_tile_clear((700, 640)));
        assert!(!w.a37_step_x_clear(0xdead_beef, (700, 640)));
    }

    #[test]
    fn vertical_step_and_move_wrappers_issue_the_typed_move() {
        let mut w = open_world();
        let id = spawn(&mut w, uid::MARINE, 640, 640);
        assert!(w.a37_vertical_step(id, 640, 640, 0x80).unwrap());
        assert_eq!(w.unit(id).unwrap().order, Order::Move { pos: (640, 650) });
        // Direction without bit 7 keeps the vertical coordinate.
        assert!(w.a37_vertical_step(id, 660, 660, 0x01).unwrap());
        assert_eq!(w.unit(id).unwrap().order, Order::Move { pos: (660, 660) });

        // Out-of-bounds and blocked tiles report false without a move.
        assert!(!w.a37_vertical_step(id, 100_000, 100, 0x80).unwrap());
        w.terrain.set_walkable_cell(700 / 8, 700 / 8, false);
        assert!(!w.a37_vertical_step(id, 700, 690, 0x80).unwrap());

        // The linked unit is rejected by the CUnit+0x80 gate.
        let a = spawn(&mut w, uid::MARINE, 640, 640);
        let b = spawn(&mut w, uid::MARINE, 660, 660);
        let ai = w.index_of(a).unwrap();
        w.units[ai].connected_unit = Some(b);
        assert!(!w.a37_move_if_unlinked(a, (700, 700)).unwrap());
        assert!(!w.a37_vertical_step(a, 760, 760, 0).unwrap());

        assert!(w.a37_move_if_unlinked(b, (720, 720)).unwrap());
        assert_eq!(w.unit(b).unwrap().order, Order::Move { pos: (720, 720) });
        w.a37_move_to_point(b, (740, 740)).unwrap();
        assert_eq!(w.unit(b).unwrap().order, Order::Move { pos: (740, 740) });
        assert_eq!(
            w.a37_move_to_point(0xdead_beef, (1, 1)),
            Err(SimError::BadUnit)
        );
    }
}
