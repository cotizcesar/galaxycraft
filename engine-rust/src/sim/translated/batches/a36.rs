#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent36.rs
// ===========================================================================
// Translation batch 36.  Only this file is edited by the batch.
//
// The `decompiled_uncited` rows in `0x005da000..=0x005dbfff` are the retail
// command-card / building-placement cluster.  Eight of the entries are
// control-flow-obfuscated jump-table dispatchers whose shared tail walks a
// player's unit list and runs a callback; the callback table itself is not
// recoverable, so they are skipped rather than guessed.  Several more rows
// are one-byte `halt_baddata` stubs or MSVC exception-unwind fragments.
//
// What is recoverable is the record-level part: the unit-type getter, the
// research countdown write, the Zerg queued-morph cancellation (with its
// three-quarter refund), the production-dialog icon choice, the Zerg morph
// state transitions, the Nydus Canal placement, and the placement footprint
// walk with its two preview geometries.  Those are translated here.
// Presentation effects (cue ids, dialog windows, footprint minimap flags) are
// not simulation state, so they are returned to the caller or documented as
// unmodelled instead of being invented as new fields.

/// One map tile in pixels.
const A36_TILE: i32 = 32;

/// `units.dat` flag `0x0002_0000` marks the Zerg structures whose placement
/// preview the recovered guard suppresses.
const A36_FLAG_NO_PREVIEW: u32 = 0x0002_0000;

/// `units.dat` flag bit 31: the structure uses its `placement` box with the
/// two-tile growth margin instead of the plain box.
const A36_FLAG_WIDE_BOX: u32 = 0x8000_0000;

/// Rows `0x83`, `0x84`, `0x85`, `0x8f`, `0x90`, `0x92`: the creep-generating
/// Zerg structures (Hatchery, Lair, Hive, Creep/Spore/Sunken Colony).  They
/// are exempt from the [`A36_FLAG_NO_PREVIEW`] filter, so their previews stay
/// available.
const A36_CREEP_ROWS: [u16; 6] = [
    uid::HATCHERY,
    uid::LAIR,
    uid::HIVE,
    uid::CREEP_COLONY,
    uid::SPORE_COLONY,
    uid::SUNKEN_COLONY,
];

/// `orders.dat` row 43 (`0x2b`), Zerg Building Morph.
const A36_ROW_ZERG_BUILDING_MORPH: u8 = 0x2b;

/// Cue the shared dialog-close routine plays (`FUN_0058b460(0x13)`).
const A36_CLOSE_CUE: u8 = 0x13;

/// Geometry of one placement preview, in tile units.
///
/// `drawn` is the rectangle the client paints (the `units.dat` placement box,
/// centred on the cursor).  `scanned` is the rectangle whose tiles the
/// footprint walk tests; for wide structures it grows by two tiles on the
/// left, one at the top, four in width and two in height.  `skip_corners`
/// marks the wide cases, whose four corner tiles are not tested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A36Placement {
    pub drawn: (i32, i32, i32, i32),
    pub scanned: (i32, i32, i32, i32),
    pub skip_corners: bool,
}

/// True for the six creep-generating Zerg structures.
fn a36_is_creep_row(type_id: u16) -> bool {
    A36_CREEP_ROWS.contains(&type_id)
}

/// The two-tile growth margin is exactly the `placement` box of the wide
/// structures, so a tile rectangle derived from it is centred on the cursor.
fn a36_placement_geometry(ty: &UnitType, cursor: (i32, i32)) -> A36Placement {
    let (pw, ph) = (ty.placement.x as i32, ty.placement.y as i32);
    // C division truncates toward zero; the following `>> 5` is the retail
    // sign-fixed divide by 32 (an arithmetic shift).
    let left = (cursor.0 - pw / 2) >> 5;
    let top = (cursor.1 - ph / 2) >> 5;
    let (w, h) = (pw >> 5, ph >> 5);
    let wide = ty.flags & A36_FLAG_WIDE_BOX != 0;
    let scanned = if wide {
        (left - 2, top - 1, w + 4, h + 2)
    } else {
        (left, top, w, h)
    };
    A36Placement {
        drawn: (left, top, w, h),
        scanned,
        skip_corners: wide,
    }
}

/// The retail guard lets a type through when it is a creep structure or when
/// it does not carry the Zerg-structure preview flag.
fn a36_preview_eligible(type_id: u16, ty: &UnitType) -> bool {
    a36_is_creep_row(type_id) || ty.flags & A36_FLAG_NO_PREVIEW == 0
}

/// Does `unit` occupy map tile `(tx, ty)`?
///
/// The recovered walk asks two spatial queries per tile (a unit query with
/// mask `0x20800000` and a second occupancy query).  The Rust model keeps a
/// single unit list, so both collapse into one scan.  Structures and
/// resources use their `units.dat` placement box, mobile units their
/// directional box, both centred on the unit position.
fn a36_blocks_tile(data: &GameData, unit: &SimUnit, tx: i32, ty: i32) -> bool {
    if !unit.alive || unit.lifted || unit.in_transport.is_some() {
        return false;
    }
    let ty_ = data.unit(unit.type_id);
    let centre = (unit.x / fp(1), unit.y / fp(1));
    let unit_box = if ty_.is_building() || is_resource_type(unit.type_id) {
        let (pw, ph) = (ty_.placement.x as i32, ty_.placement.y as i32);
        Rect::new(
            centre.0 - pw / 2,
            centre.1 - ph / 2,
            centre.0 + pw / 2,
            centre.1 + ph / 2,
        )
    } else {
        let (left, top, right, bottom) = ty_.dimensions.rect_at(centre);
        Rect::new(left, top, right, bottom)
    };
    let tile = Rect::new(
        tx * A36_TILE,
        ty * A36_TILE,
        tx * A36_TILE + A36_TILE,
        ty * A36_TILE + A36_TILE,
    );
    unit_box.intersects(&tile)
}

impl SimUnit {
    /// `0x005da480` (17-byte entry; the body is the shared `CUnit+0x64`
    /// getter).
    ///
    /// ```c
    /// return *(u16 *)(this + 0x64);   // units.dat row of this record
    /// ```
    ///
    /// `CUnit+0x64` is the unit's type id; the same getter is used by the
    /// dispatcher tails to look up the row after they set a per-unit flag.
    pub fn a36_unit_type_id(&self) -> u16 {
        self.type_id
    }

    /// `0x005da628` (19 bytes).
    ///
    /// ```c
    /// *(u16 *)(this + 0xc6) = in_CX + (u16)*(u32 *)(&DAT_00f92f60 + in_EDX * 2);
    /// ```
    ///
    /// `CUnit+0xc6` is the remaining research/upgrade countdown, which
    /// [`SimUnit::research_remaining`] exposes from `build_remaining`.  The
    /// recovered write is the start of a research or upgrade timer: the
    /// caller's base duration plus the per-level table entry selected by
    /// `in_EDX`.  The sum is stored as a 16-bit word, so it wraps.  Returns
    /// the stored value.
    pub fn a36_set_research_timer(&mut self, base_ticks: i32, table_ticks: i32) -> i32 {
        let stored = base_ticks.wrapping_add(table_ticks) & 0xffff;
        self.build_remaining = stored;
        stored
    }

    /// `0x005db326` (37 bytes).
    ///
    /// ```c
    /// FUN_0058cdc0();                       // refresh the build UI
    /// if (type != 0x85 && type != 0x84 && type != 0x89 &&
    ///     type != 0x90 && type != 0x92) {
    ///   FUN_0058fb90(FUN_005b4a80());        // play the "unit ready" sound
    /// }
    /// ```
    ///
    /// The five excluded rows are the Zerg morph family
    /// ([`is_zerg_queue_building`]): those buildings morph in place, so the
    /// completion cue is suppressed.  The sound id itself comes from
    /// `CUnit+0xa8`, which the Rust record does not keep; the caller supplies
    /// it.  Returns the cue id to play, or `None` when the cue is suppressed.
    pub fn a36_completion_sound(&self, sound_id: u16) -> Option<u16> {
        if is_zerg_queue_building(self.type_id) {
            None
        } else {
            Some(sound_id)
        }
    }
}

impl World {
    /// `0x005da595` (49 bytes).
    ///
    /// ```c
    /// FUN_00781600();                 // set global UI status bit 0x13
    /// FUN_0058b460(0x13);             // play cue 0x13
    /// FUN_0062ca50();                 // current dialog
    /// FUN_0062c000();                 // destroy the dialog window
    /// return 1;                       // handled
    /// ```
    ///
    /// A dialog-close callback.  The window and the global status bit are
    /// presentation state, so the method reports "handled" plus the cue the
    /// caller has to play.  Returns `(true, 0x13)`.
    pub fn a36_close_window_cue(&self) -> (bool, u8) {
        (true, A36_CLOSE_CUE)
    }

    /// `0x005dacd0` (83 bytes).
    ///
    /// ```c
    /// if (cookie_ok) {
    ///   type = *(u16 *)(this + 0x98 + this[0xa4] * 2);   // last queue slot
    ///   if (type == 0x85 || type == 0x84 || type == 0x89 ||
    ///       type == 0x90 || type == 0x92) {
    ///     FUN_0067ce70(this->owner, type);              // refund 3/4
    ///     *(u16 *)(this + 0x98 + this[0xa4] * 2) = 0xe4; // free the slot
    ///     ...                                           // HP-bar refresh
    ///   }
    /// }
    /// ```
    ///
    /// Cancel the most recently queued Zerg morph.  Only the five morph rows
    /// [`is_zerg_queue_building`] reports are cancellable this way; the
    /// refund is three quarters of the row's mineral and gas cost, matching
    /// the other recovered cancellations.  The queue index at `CUnit+0xa4`
    /// is read as the slot just filled (the command card cancels the newest
    /// entry), which the Rust `production` deque models as its back.  Returns
    /// the cancelled row.
    pub fn a36_cancel_queued_morph(&mut self, unit_id: u32) -> Option<u16> {
        let i = self.index_of(unit_id)?;
        let queued = match self.units[i].production.back() {
            Some(Order::Train { unit_type }) => *unit_type,
            _ => return None,
        };
        if !is_zerg_queue_building(queued) {
            return None;
        }
        self.units[i].production.pop_back();
        let (minerals, gas) = {
            let ty = self.data.unit(queued);
            (ty.mineral_cost as i32 * 3 / 4, ty.gas_cost as i32 * 3 / 4)
        };
        let owner = self.units[i].owner as usize;
        if let Some(player) = self.players.get_mut(owner) {
            player.minerals += minerals;
            player.gas += gas;
        }
        Some(queued)
    }

    /// `0x005db9a2` (74 bytes).
    ///
    /// ```c
    /// if (row != 0x2b) return;                       // Zerg Building Morph only
    /// type = *(u16 *)(this + 0x98 + this[0xa4] * 2); // queued morph target
    /// if (FUN_0067ced0(owner, type, 1) == 0) {       // requirements/supply
    ///   FUN_0058e1c0();                              // error cue
    ///   return;
    /// }
    /// this->flags &= ~1;
    /// ```
    ///
    /// The Zerg Building Morph row checks the queued target against the
    /// player's build tree before starting.  `None` when the routine returns
    /// immediately (any other row), otherwise the requirement verdict from
    /// [`World::has_requirements`] (which also reports `false` for rows the
    /// recovered build tree does not list).
    pub fn a36_zerg_morph_requirements(
        &self,
        owner: u8,
        order_row: u8,
        queued: u16,
    ) -> Option<bool> {
        if order_row != A36_ROW_ZERG_BUILDING_MORPH {
            return None;
        }
        Some(self.has_requirements(owner, queued))
    }

    /// `0x005dba00` (20 bytes).
    ///
    /// ```c
    /// if (in_AX == 0x23) {
    ///   FUN_005afa00(0x24);   // switch the record to the 0x24 morph row
    /// }
    /// ```
    ///
    /// The selector values are `units.dat` rows: `0x23` is Larva and `0x24`
    /// is Egg, the intermediate of the larva hatch.  Returns the row the
    /// handler installs, or `None` when the selector does not match.
    pub fn a36_larva_morph_state(selector: u16) -> Option<u16> {
        (selector == uid::LARVA).then_some(uid::EGG)
    }

    /// `0x005dba14` (20 bytes).
    ///
    /// ```c
    /// if (in_AX == 0x26) {
    ///   FUN_005afa00(0x61);   // switch the record to the 0x61 morph row
    /// }
    /// ```
    ///
    /// `0x26` is Hydralisk and `0x61` Lurker Egg, the intermediate of the
    /// Lurker morph.
    pub fn a36_hydralisk_morph_state(selector: u16) -> Option<u16> {
        (selector == uid::HYDRALISK).then_some(uid::LURKER_EGG)
    }

    /// `0x005dba28` (30 bytes).
    ///
    /// ```c
    /// if (in_AX == 0x2b) {
    ///   FUN_005afa00(0x3b);   // switch the record to the 0x3b morph row
    /// }
    /// return;
    /// ```
    ///
    /// `0x2b` is Mutalisk and `0x3b` Cocoon, the intermediate shared by the
    /// Guardian and Devourer morphs.
    pub fn a36_mutalisk_morph_state(selector: u16) -> Option<u16> {
        (selector == uid::MUTALISK).then_some(uid::COCOON)
    }

    /// `0x005dbdd0` (54 bytes).
    ///
    /// ```c
    /// FUN_0079fca0();
    /// id = FUN_005b6030();                         // production-building test
    /// FUN_005a6300((id != 0) + 0xe9);              // dialog/button id
    /// FUN_005a6890(1);                             // select the dialog
    /// ```
    ///
    /// The cancel dialog uses button id `0xea` for a structure that owns a
    /// rally point (town halls and unit producers) and `0xe9` otherwise.
    pub fn a36_cancel_dialog_icon(&self, type_id: u16) -> u16 {
        0xe9 + u16::from(is_production_building(type_id))
    }

    /// `0x005dbec0` (242 bytes).
    ///
    /// ```c
    /// for (row = 0; row < height; row++) {
    ///   for (col = 0; col < width; col++) {
    ///     x = left + col;  y = top + row;              // unsigned compares
    ///     if (!((uint)x < map_w)) continue;            // off-map skipped
    ///     if (!((uint)y < map_h)) continue;
    ///     if (skip && !((col != 0 && col != w-1) ||
    ///                   (row != 0 && row != h-1))) continue;  // corners only
    ///     if (no unit at (x, y) && no building at (x, y)) {
    ///       mark the tile;                            // terrain/minimap flags
    ///     }
    ///   }
    /// }
    /// ```
    ///
    /// Walk a placement rectangle tile by tile and return `(checked, free)`
    /// counts.  Tiles outside the map are skipped without failing, exactly
    /// like the unsigned bounds test in the original.  `skip_corners` drops
    /// only the four corner tiles - the original's condition is an `or`
    /// between the column and row interior tests.  The routine's side effect
    /// (stamping terrain/minimap flags for each free tile) is presentation
    /// state and is not modelled.
    pub fn a36_footprint_scan(
        &self,
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        skip_corners: bool,
    ) -> (usize, usize) {
        let map_w = self.terrain.width as i32;
        let map_h = self.terrain.height as i32;
        let mut checked = 0usize;
        let mut free = 0usize;
        for row in 0..height.max(0) {
            let ty = top + row;
            if ty < 0 || ty >= map_h {
                continue;
            }
            for col in 0..width.max(0) {
                let tx = left + col;
                if tx < 0 || tx >= map_w {
                    continue;
                }
                if skip_corners {
                    let inner_x = col != 0 && col != width - 1;
                    let inner_y = row != 0 && row != height - 1;
                    if !inner_x && !inner_y {
                        continue;
                    }
                }
                checked += 1;
                if !self
                    .units
                    .iter()
                    .any(|u| a36_blocks_tile(&self.data, u, tx, ty))
                {
                    free += 1;
                }
            }
        }
        (checked, free)
    }

    /// `0x005dbfc0` (260 bytes).
    ///
    /// ```c
    /// if (FUN_00983ae0(type, 1) != 0 || (flags[type] & 0x20000) == 0) {
    ///   pw = placement_x[type];  ph = placement_y[type];
    ///   left = (cursor_x - pw / 2) >> 5;     // tile units
    ///   top  = (cursor_y - ph / 2) >> 5;
    ///   w = pw >> 5;  h = ph >> 5;
    ///   draw rect (left, top, w, h);
    ///   if (flags[type] < 0) {               // bit 31
    ///     left -= 2; top -= 1; w += 4; h += 2;
    ///   }
    ///   scan footprint (left, top, w, h, flags[type] < 0);
    ///   undraw rect;
    /// }
    /// ```
    ///
    /// Returns the painted box and the scanned box for a placement preview at
    /// `cursor` (pixels), or `None` when the guard suppresses the preview for
    /// this type.
    pub fn a36_placement_rects(&self, type_id: u16, cursor: (i32, i32)) -> Option<A36Placement> {
        let ty = self.data.unit(type_id);
        if !a36_preview_eligible(type_id, ty) {
            return None;
        }
        Some(a36_placement_geometry(ty, cursor))
    }

    /// `0x005db4df` (97 bytes).
    ///
    /// The twin of `0x005dbfc0` with the cursor supplied the other way round;
    /// the geometry and the `(type, 1)` guard are identical, and the routine
    /// ends with the footprint walk rather than with the undraw.  Returns
    /// `(checked, free)` for the scanned box, or `None` when the preview is
    /// suppressed.
    pub fn a36_placement_scan(&self, type_id: u16, cursor: (i32, i32)) -> Option<(usize, usize)> {
        let placement = self.a36_placement_rects(type_id, cursor)?;
        let (left, top, width, height) = placement.scanned;
        Some(self.a36_footprint_scan(left, top, width, height, placement.skip_corners))
    }

    /// `0x005dba92` (16-byte entry into the shared create path).
    ///
    /// ```c
    /// if (FUN_00687340(...) != 0) {                 // placement/validation
    ///   FUN_006e39d0(); func_0x005b8040(); return;  // abort, no unit
    /// }
    /// unit = FUN_005af840(0x86, this+0x58, this+0x5a, this->owner, reason);
    /// if (unit == 0) { FUN_005b04b0(owner); return; }
    /// FUN_005a6890(1);                              // select the new canal
    /// ```
    ///
    /// Place a Nydus Canal (`units.dat` row `0x86`) at the stored order
    /// target; the caller is the canal-placement command.  The model's
    /// placement check is the terrain/blocker footprint used by every other
    /// structure; a blocked cursor aborts with [`SimError::InvalidTarget`],
    /// mirroring the retail early-out.  Like the create primitive, this entry
    /// does not move resources (the build command pays for the canal), so the
    /// caller keeps the charge.  Returns the new unit id.
    pub fn a36_place_nydus_canal(&mut self, owner: u8, pos: (i32, i32)) -> Result<u32, SimError> {
        if owner as usize >= self.players.len() {
            return Err(SimError::BadOwner);
        }
        let (_, _, tiles) = self.placement_tiles(uid::NYDUS_CANAL, pos, None);
        if tiles.iter().any(|free| !free) {
            return Err(SimError::InvalidTarget);
        }
        let index = self.spawn_px(owner as usize, uid::NYDUS_CANAL, pos.0, pos.1)?;
        Ok(self.units[index].id)
    }
}

#[cfg(test)]
mod tests_agent36 {
    use super::*;
    use crate::data::GameData;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, type_id: u16, x: i32, y: i32) -> u32 {
        let index = world.spawn(0, type_id, x, y).unwrap();
        world.units[index].id
    }

    #[test]
    fn a36_unit_type_id_reads_the_record_row() {
        let mut world = world();
        let id = spawn(&mut world, uid::MARINE, 100, 100);
        assert_eq!(world.unit(id).unwrap().a36_unit_type_id(), uid::MARINE);
        let id = spawn(&mut world, uid::HATCHERY, 300, 300);
        assert_eq!(world.unit(id).unwrap().a36_unit_type_id(), uid::HATCHERY);
    }

    #[test]
    fn a36_research_timer_sums_and_wraps_like_the_u16_store() {
        let mut world = world();
        let id = spawn(&mut world, uid::COMMAND_CENTER, 300, 300);
        let unit = world.units.iter_mut().find(|u| u.id == id).unwrap();
        unit.order = Order::Research { tech_id: 0 };
        assert_eq!(unit.a36_set_research_timer(100, 20), 120);
        assert_eq!(unit.build_remaining, 120);
        assert_eq!(unit.research_remaining(), 120);
        // The original stores a 16-bit word.
        assert_eq!(unit.a36_set_research_timer(0xffff, 1), 0);
        assert_eq!(unit.build_remaining, 0);
    }

    #[test]
    fn a36_close_window_cue_reports_handled_and_cue() {
        let world = world();
        assert_eq!(world.a36_close_window_cue(), (true, 0x13));
    }

    #[test]
    fn a36_cancel_queued_morph_refunds_three_quarters() {
        let mut world = world();
        let id = spawn(&mut world, uid::HATCHERY, 300, 300);
        let index = world.index_of(id).unwrap();
        world.units[index].production.push_back(Order::Train {
            unit_type: uid::HIVE,
        });
        assert_eq!(world.players[0].minerals, 50);
        assert_eq!(world.players[0].gas, 0);
        assert_eq!(world.a36_cancel_queued_morph(id), Some(uid::HIVE));
        assert!(world.units[index].production.is_empty());
        assert_eq!(world.players[0].minerals, 50 + 200 * 3 / 4);
        assert_eq!(world.players[0].gas, 150 * 3 / 4);
        // Nothing left to cancel.
        assert_eq!(world.a36_cancel_queued_morph(id), None);
    }

    #[test]
    fn a36_cancel_queued_morph_ignores_other_rows() {
        let mut world = world();
        let id = spawn(&mut world, uid::HATCHERY, 300, 300);
        let index = world.index_of(id).unwrap();
        world.units[index].production.push_back(Order::Train {
            unit_type: uid::ZERGLING,
        });
        assert_eq!(world.a36_cancel_queued_morph(id), None);
        assert_eq!(world.units[index].production.len(), 1);
        assert_eq!(world.a36_cancel_queued_morph(0xdead_beef), None);
    }

    #[test]
    fn a36_completion_sound_is_suppressed_for_the_morph_family() {
        let mut world = world();
        let lair = spawn(&mut world, uid::LAIR, 300, 300);
        let center = spawn(&mut world, uid::COMMAND_CENTER, 500, 500);
        assert_eq!(world.unit(lair).unwrap().a36_completion_sound(7), None);
        assert_eq!(world.unit(center).unwrap().a36_completion_sound(7), Some(7));
    }

    #[test]
    fn a36_zerg_morph_requirements_checks_only_the_morph_row() {
        let mut world = world();
        // Any other row returns before the check.
        assert_eq!(world.a36_zerg_morph_requirements(0, 0x1f, uid::LAIR), None);
        // Lair needs a Hatchery and a Spawning Pool.
        assert_eq!(
            world.a36_zerg_morph_requirements(0, A36_ROW_ZERG_BUILDING_MORPH, uid::LAIR),
            Some(false)
        );
        spawn(&mut world, uid::HATCHERY, 300, 300);
        spawn(&mut world, uid::SPAWNING_POOL, 500, 300);
        assert_eq!(
            world.a36_zerg_morph_requirements(0, A36_ROW_ZERG_BUILDING_MORPH, uid::LAIR),
            Some(true)
        );
    }

    #[test]
    fn a36_zerg_morph_state_transitions() {
        assert_eq!(World::a36_larva_morph_state(uid::LARVA), Some(uid::EGG));
        assert_eq!(World::a36_larva_morph_state(uid::HYDRALISK), None);
        assert_eq!(
            World::a36_hydralisk_morph_state(uid::HYDRALISK),
            Some(uid::LURKER_EGG)
        );
        assert_eq!(World::a36_hydralisk_morph_state(uid::LARVA), None);
        assert_eq!(
            World::a36_mutalisk_morph_state(uid::MUTALISK),
            Some(uid::COCOON)
        );
        assert_eq!(World::a36_mutalisk_morph_state(uid::EGG), None);
    }

    #[test]
    fn a36_cancel_dialog_icon_depends_on_the_rally_owners() {
        let world = world();
        assert_eq!(world.a36_cancel_dialog_icon(uid::COMMAND_CENTER), 0xea);
        assert_eq!(world.a36_cancel_dialog_icon(uid::HATCHERY), 0xea);
        assert_eq!(world.a36_cancel_dialog_icon(uid::MARINE), 0xe9);
        assert_eq!(world.a36_cancel_dialog_icon(uid::SUPPLY_DEPOT), 0xe9);
    }

    #[test]
    fn a36_footprint_scan_counts_map_tiles_only() {
        let world = world();
        assert_eq!(world.a36_footprint_scan(10, 10, 4, 3, false), (12, 12));
        // Only the four corners drop under the retail `or` test.
        assert_eq!(world.a36_footprint_scan(10, 10, 4, 3, true), (8, 8));
        // Past the map edge the remaining tiles are skipped, not failed.
        assert_eq!(world.a36_footprint_scan(62, 62, 4, 4, false), (4, 4));
    }

    #[test]
    fn a36_footprint_scan_sees_structures() {
        let mut world = world();
        spawn(&mut world, uid::COMMAND_CENTER, 320, 320);
        assert_eq!(world.a36_footprint_scan(8, 8, 4, 4, false), (16, 0));
    }

    #[test]
    fn a36_placement_rects_matches_the_recovered_geometry() {
        let world = world();
        let center = world
            .a36_placement_rects(uid::COMMAND_CENTER, (100, 100))
            .unwrap();
        assert_eq!(center.drawn, (1, 1, 4, 3));
        assert_eq!(center.scanned, (-1, 0, 8, 5));
        assert!(center.skip_corners);

        let depot = world
            .a36_placement_rects(uid::SUPPLY_DEPOT, (100, 100))
            .unwrap();
        assert_eq!(depot.drawn, (1, 2, 3, 2));
        assert_eq!(depot.scanned, depot.drawn);
        assert!(!depot.skip_corners);

        // A Zerg structure carrying the preview flag but not building creep
        // is filtered out by the guard.
        assert_eq!(world.a36_placement_rects(uid::SPIRE, (100, 100)), None);
        assert!(
            world
                .a36_placement_rects(uid::HATCHERY, (100, 100))
                .is_some()
        );
    }

    #[test]
    fn a36_placement_scan_counts_the_expanded_box() {
        let mut world = world();
        assert_eq!(
            world.a36_placement_scan(uid::HATCHERY, (320, 320)),
            Some((36, 36))
        );
        // A Command Center at the same point blocks the 4x4 inner tiles of
        // the scanned 8x5 box (none of them are corners).
        spawn(&mut world, uid::COMMAND_CENTER, 320, 320);
        assert_eq!(
            world.a36_placement_scan(uid::HATCHERY, (320, 320)),
            Some((36, 20))
        );
        assert_eq!(world.a36_placement_scan(uid::SPIRE, (320, 320)), None);
    }

    #[test]
    fn a36_place_nydus_canal_spawns_and_rejects_a_blocked_cursor() {
        let mut world = world();
        let id = world.a36_place_nydus_canal(0, (320, 320)).unwrap();
        let unit = world.unit(id).unwrap();
        assert_eq!(unit.type_id, uid::NYDUS_CANAL);
        assert_eq!(unit.owner, 0);
        // The first canal blocks its own footprint.
        assert_eq!(
            world.a36_place_nydus_canal(0, (320, 320)),
            Err(SimError::InvalidTarget)
        );
        assert_eq!(
            world.a36_place_nydus_canal(5, (320, 320)),
            Err(SimError::BadOwner)
        );
    }
}
