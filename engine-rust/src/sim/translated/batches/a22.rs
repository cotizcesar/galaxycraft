#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent22.rs
// ===========================================================================
// Translation batch 22.  Only this file is edited by the batch.
//
// The batch covers `decompiled_uncited` rows of the
// `0x005a2000..=0x005a3fff` window.  That stretch of the retail image is
// dense with unit-order helpers, but Ghidra split several of the routines
// into one ledger row per branch target, so many rows are fragments of a
// larger body.  This module reconstructs the observable part of each row
// named below; where a fragment depends on an unrecovered `CUnit` byte or
// on a retail helper that is not part of the simulation model, the caller
// supplies that input and the item doc says so.
//
// Only existing [`World`] / [`SimUnit`] fields are used.  The fixed-point
// position convention is the one already used by the simulation: `x`/`y`
// are 1/256 pixel, and [`fp`] converts whole pixels.

impl SimUnit {
    /// `0x005a2998`: tail of the order-link setup routine.
    ///
    /// The retail row first clears `CUnit+0xdc` bit 19, writes the link id
    /// into the `CUnit+0x5c` order-target slot and marks the target's
    /// sprite/state byte, and then drops `CUnit+0x7c` when it is the same
    /// unit as the new order link.  Only the two modelled fields are
    /// reproduced here: [`SimUnit::flag_19`] is cleared and
    /// [`SimUnit::target`] is cleared when it aliases
    /// [`SimUnit::order_unit_target`].  The `CUnit+0x12` word and the target
    /// sprite byte are not part of the Rust model.
    ///
    /// Returns true when the aliased current target was cleared.
    pub fn a22_clear_link_target_alias(&mut self) -> bool {
        self.flag_19 = false;
        if self.target.is_some() && self.target == self.order_unit_target() {
            self.target = None;
            true
        } else {
            false
        }
    }

    /// `0x005a3370`: quarter-inset of a 16-bit rectangle followed by the
    /// centre of the inset rectangle.
    ///
    /// Each axis is shrunk from both ends by a quarter of its extent
    /// (`start + (end - start) >> 2` and `end - that delta`) and the two
    /// results are averaged.  Retail stores every intermediate as an `i16`
    /// and uses a truncating divide by two, which is reproduced here.  The
    /// row is the rectangle-to-midpoint step used by the effect-placement
    /// routine.
    pub fn a22_inset_rect_center(&self, rect: (i32, i32, i32, i32)) -> (i32, i32) {
        fn inset(start: i32, end: i32) -> (i32, i32) {
            let start = start as i16;
            let end = end as i16;
            let quarter = ((i32::from(end) - i32::from(start)) >> 2) as i16;
            (
                i32::from(start.wrapping_add(quarter)),
                i32::from(end.wrapping_sub(quarter)),
            )
        }
        let (left, right) = inset(rect.0, rect.2);
        let (top, bottom) = inset(rect.1, rect.3);
        (((right - left) / 2) + left, ((bottom - top) / 2) + top)
    }
}

impl World {
    /// `0x005a2a8c`: candidate gate for the outgoing order-unit link.
    ///
    /// Retail keeps the `CUnit+0x5c` link only when the candidate exists and
    /// passes the target predicate.  When the "allow critters" argument is
    /// clear, wildlife rows (the same group the order dispatcher treats as
    /// critters) are rejected.  The target predicate itself is a retail
    /// helper that is not part of the simulation, so its answer is supplied
    /// as `weapon_target_ok`.  Returns whether the link stays attached.
    pub fn a22_order_link_candidate_ok(
        &self,
        candidate_id: u32,
        weapon_target_ok: bool,
        allow_critter: bool,
    ) -> bool {
        let Some(candidate) = self.unit(candidate_id) else {
            return false;
        };
        candidate.alive
            && weapon_target_ok
            && (allow_critter || !is_critter_type(candidate.type_id))
    }

    /// `0x005a2b12`: retention rule for the current target (`CUnit+0x7c`).
    ///
    /// The row clears the current target when the candidate is neither the
    /// Bunker row `0x7d` nor linked through an order target owned by the
    /// same player as the source.  The final retail condition uses a helper
    /// that is not modelled, so it is supplied as
    /// `retain_for_other_reason`.
    ///
    /// Returns true when the current target survives the gate.
    pub fn a22_current_target_kept(
        &self,
        unit_id: u32,
        candidate_id: u32,
        retain_for_other_reason: bool,
    ) -> bool {
        let (Some(source), Some(candidate)) = (self.unit(unit_id), self.unit(candidate_id)) else {
            return false;
        };
        if candidate.type_id == 0x7d || retain_for_other_reason {
            return true;
        }
        candidate
            .order_unit_target()
            .and_then(|id| self.unit(id))
            .is_some_and(|link| link.owner == source.owner)
    }

    /// `0x005a2c70`: first guard of the placement query.
    ///
    /// The retail body returns zero immediately for an airborne unit
    /// (`CUnit+0xdc` bit 2) and for a `units.dat` building row, and only
    /// then runs the rectangle/elevation placement search.  That search is
    /// not modelled; this method reports the early result so the caller can
    /// skip it.
    ///
    /// Returns `Some(0)` when the guard fires and `None` when the full
    /// placement query would have to run.
    pub fn a22_placement_short_circuit(&self, unit_id: u32) -> Option<u8> {
        let i = self.index_of(unit_id)?;
        if self.is_air_unit(i) || self.data.unit(self.units[i].type_id).is_building() {
            Some(0)
        } else {
            None
        }
    }

    /// `0x005a301f`: fixed-point damage contribution of one weapon volley.
    ///
    /// The row multiplies the `weapons.dat` damage-bonus column by the
    /// owner's upgrade level, truncates the product to 16 bits, adds the
    /// caller's damage roll and shifts the sum into 1/256-point damage.
    /// [`crate::data::WeaponType::damage_bonus`] exposes that column; the
    /// roll comes from the retail random-damage routine and is supplied as
    /// `roll`.  A missing weapon row yields zero.
    pub fn a22_weapon_damage_fixed(&self, weapon_id: u8, upgrade_level: u8, roll: u16) -> i32 {
        let Some(weapon) = self.data.weapon(weapon_id) else {
            return 0;
        };
        let bonus = (u32::from(weapon.damage_bonus) * u32::from(upgrade_level)) & 0xffff;
        (i32::from(roll) + bonus as i32) << 8
    }

    /// `0x005a3040`: placement result adjusted for the ground-elevation
    /// relationship between the source and the target.
    ///
    /// The row returns the target's placement code unchanged when either
    /// unit is airborne; otherwise, when the source stands on lower terrain
    /// than the target, it raises the code to at least `0x77`.  The two
    /// elevations are the tile-height values read for each position.
    /// Returns `None` when either id is unknown.
    pub fn a22_placement_code_with_elevation(
        &self,
        source_id: u32,
        target_id: u32,
        target_code: u8,
    ) -> Option<u8> {
        let source = self.index_of(source_id)?;
        let target = self.index_of(target_id)?;
        if self.is_air_unit(source) || self.is_air_unit(target) {
            return Some(target_code);
        }
        let source_elevation = self
            .terrain
            .elevation_px(self.units[source].x / fp(1), self.units[source].y / fp(1));
        let target_elevation = self
            .terrain
            .elevation_px(self.units[target].x / fp(1), self.units[target].y / fp(1));
        Some(if source_elevation < target_elevation {
            target_code.max(0x77)
        } else {
            target_code
        })
    }

    /// `0x005a3098`: tile-height value at an absolute pixel position.
    ///
    /// This is the second of the two terrain lookups in the placement
    /// routine.  The megatile containing the position is classified from
    /// its tile height flags: the very-high bit yields 2, otherwise the high
    /// bit yields 1, and 0 is returned for every other megatile.  The row
    /// computes the megatile index with a floor divide by 32, reproduced
    /// with `div_euclid`.
    pub fn a22_tile_elevation_at(&self, x: i32, y: i32) -> u8 {
        let flags = self
            .terrain
            .tile_height_flags(x.div_euclid(32), y.div_euclid(32));
        if flags & TILE_VERY_HIGH != 0 {
            2
        } else {
            u8::from(flags & TILE_HIGH != 0)
        }
    }

    /// `0x005a3610`: propagate a target to nearby same-owner combat units.
    ///
    /// This is the tail loop of the target-propagation routine.  It walks
    /// the units inside a square of `radius` whole pixels around the source
    /// and gives each eligible unit the new target.  Retail excludes the
    /// source itself, other owners, Interceptor/Scarab rows (`0x47`/`0x56`),
    /// workers, units whose `CUnit+0xdc` bit 4 is set (modelled as
    /// [`SimUnit::completed`]) and units with no weapon at all.  A unit that
    /// already has the same target is skipped, as are targets that are not
    /// enemies of the candidate.  The optional `CUnit+0x70` subunit copy is
    /// handled by [`Self::a22_propagate_target_to_subunit`].
    ///
    /// Returns the number of units whose current target was changed.
    pub fn a22_link_target_to_nearby_units(
        &mut self,
        source_id: u32,
        target_id: u32,
        radius: i32,
    ) -> usize {
        let Some(source_index) = self.index_of(source_id) else {
            return 0;
        };
        let Some(target_index) = self.index_of(target_id) else {
            return 0;
        };
        let source = &self.units[source_index];
        let source_owner = source.owner;
        let source_x = source.x / fp(1);
        let source_y = source.y / fp(1);
        let target_owner = self.units[target_index].owner;

        let mut chosen = Vec::new();
        for (index, unit) in self.units.iter().enumerate() {
            if index == source_index || unit.owner != source_owner {
                continue;
            }
            if matches!(unit.type_id, 0x47 | 0x56) {
                continue;
            }
            let stats = self.data.unit(unit.type_id);
            if stats.is_worker() || unit.completed {
                continue;
            }
            if stats.ground_weapon == 0x82 && stats.air_weapon == 0x82 {
                continue;
            }
            if (unit.x / fp(1) - source_x).abs() > radius
                || (unit.y / fp(1) - source_y).abs() > radius
            {
                continue;
            }
            if unit.target == Some(target_id) || !self.is_enemy(unit.owner, target_owner) {
                continue;
            }
            chosen.push(index);
        }

        for index in &chosen {
            self.units[*index].target = Some(target_id);
        }
        chosen.len()
    }

    /// `0x005a3803`: copy the current target (and `CUnit+0x90`) to the
    /// attached subunit.
    ///
    /// The row writes the new target into `CUnit+0x7c`, and when the
    /// `CUnit+0x70` link points at a `units.dat` subunit row it copies the
    /// parent's `CUnit+0x90` byte and the target into that unit.  The
    /// subunit link is not a [`SimUnit`] field, so the candidate is passed
    /// explicitly and its `units.dat` subunit flag is checked here.
    ///
    /// Returns true when the target was propagated to the subunit.
    pub fn a22_propagate_target_to_subunit(
        &mut self,
        unit_id: u32,
        subunit_id: u32,
        target_id: u32,
    ) -> bool {
        let (Some(unit_index), Some(subunit_index)) =
            (self.index_of(unit_id), self.index_of(subunit_id))
        else {
            return false;
        };
        self.units[unit_index].target = Some(target_id);
        if !self
            .data
            .unit(self.units[subunit_index].type_id)
            .is_subunit()
        {
            return false;
        }
        let color = self.units[unit_index].field_090;
        self.units[subunit_index].field_090 = color;
        self.units[subunit_index].target = Some(target_id);
        true
    }

    /// `0x005a385c`: the square used to find same-owner units when a target
    /// is propagated.
    ///
    /// The row builds the four 16-bit words `x - 0x60`, `y - 0x60`,
    /// `x + 0x60`, `y + 0x60` around the unit position and hands them to
    /// the nearby-unit query.  Positions are converted from the fixed-point
    /// fields to whole pixels.  Returns `None` when the id is unknown.
    pub fn a22_target_propagation_rect(&self, unit_id: u32) -> Option<(i32, i32, i32, i32)> {
        let i = self.index_of(unit_id)?;
        let x = self.units[i].x / fp(1);
        let y = self.units[i].y / fp(1);
        Some((x - 0x60, y - 0x60, x + 0x60, y + 0x60))
    }

    /// `0x005a3b26`: move the unit onto its order target's position.
    ///
    /// The row validates the order target, copies its position (`CUnit+0x58`
    /// and `CUnit+0x5a`) into the unit and then runs the map-side position
    /// update.  Only the position copy is modelled here; the map update has
    /// no separate Rust state.  Returns `None` for unknown ids and otherwise
    /// reports whether either coordinate changed.
    pub fn a22_snap_to_order_target(&mut self, unit_id: u32, target_id: u32) -> Option<bool> {
        let unit_index = self.index_of(unit_id)?;
        let target_index = self.index_of(target_id)?;
        let x = self.units[target_index].x;
        let y = self.units[target_index].y;
        let changed = self.units[unit_index].x != x || self.units[unit_index].y != y;
        self.units[unit_index].x = x;
        self.units[unit_index].y = y;
        Some(changed)
    }

    /// `0x005a3d19`: centre of the quarter-inset rectangle of the current
    /// order target.
    ///
    /// The row reads the `CUnit+0x5c` order target, obtains its `units.dat`
    /// box, applies the same quarter-inset/midpoint arithmetic as the
    /// effect-placement row and returns the resulting point.  The final
    /// isometric transform of the retail routine is not modelled.
    /// Returns `None` when the unit has no order unit target.
    pub fn a22_order_target_rect_center(&self, unit_id: u32) -> Option<(i32, i32)> {
        let unit = self.unit(unit_id)?;
        let target = self.unit(unit.order_unit_target()?)?;
        let rect = self
            .data
            .unit(target.type_id)
            .dimensions
            .rect_at((target.x / fp(1), target.y / fp(1)));
        Some(unit.a22_inset_rect_center(rect))
    }
}

#[cfg(test)]
mod tests_agent22 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::Order;
    use std::sync::Arc;

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, owner: usize, type_id: u16, x: i32, y: i32) -> u32 {
        let index = world.spawn_px(owner, type_id, x, y).unwrap();
        world.units[index].id
    }

    fn index(world: &World, id: u32) -> usize {
        world.index_of(id).unwrap()
    }

    #[test]
    fn link_target_alias_is_dropped_and_flag_19_cleared() {
        let mut world = open_world();
        let marine = spawn(&mut world, 0, uid::MARINE, 640, 640);
        let enemy = spawn(&mut world, 1, uid::MARINE, 700, 640);
        let i = index(&world, marine);
        world.units[i].order = Order::AttackUnit { target: enemy };
        world.units[i].target = Some(enemy);
        world.units[i].flag_19 = true;

        assert!(world.units[i].a22_clear_link_target_alias());
        assert!(!world.units[i].flag_19);
        assert_eq!(world.units[i].target, None);

        // A target that is not the order link is preserved.
        let other = spawn(&mut world, 1, uid::MARINE, 720, 640);
        world.units[i].target = Some(other);
        assert!(!world.units[i].a22_clear_link_target_alias());
        assert_eq!(world.units[i].target, Some(other));
    }

    #[test]
    fn order_link_candidate_gate_rejects_missing_and_critters() {
        let mut world = open_world();
        let marine = spawn(&mut world, 0, uid::MARINE, 640, 640);
        let critter = spawn(&mut world, 0, 0x59, 660, 640);
        let friendly = spawn(&mut world, 0, uid::MARINE, 680, 640);

        assert!(!world.a22_order_link_candidate_ok(9999, true, true));
        assert!(!world.a22_order_link_candidate_ok(critter, true, false));
        assert!(world.a22_order_link_candidate_ok(critter, true, true));
        assert!(!world.a22_order_link_candidate_ok(friendly, false, true));
        assert!(world.a22_order_link_candidate_ok(friendly, true, true));
        assert!(world.unit(marine).is_some());
        assert!(is_critter_type(0x59));
    }

    #[test]
    fn current_target_kept_for_bunkers_and_same_owner_links() {
        let mut world = open_world();
        let marine = spawn(&mut world, 0, uid::MARINE, 640, 640);
        let bunker = spawn(&mut world, 0, uid::BUNKER, 660, 640);
        let linked = spawn(&mut world, 0, uid::MARINE, 680, 640);
        let protected = spawn(&mut world, 0, uid::MARINE, 700, 640);
        let li = index(&world, linked);
        world.units[li].order = Order::AttackUnit { target: protected };

        assert!(world.a22_current_target_kept(marine, bunker, false));
        assert!(world.a22_current_target_kept(marine, linked, false));
        assert!(world.a22_current_target_kept(marine, bunker, true));

        let plain = spawn(&mut world, 0, uid::MARINE, 720, 640);
        assert!(!world.a22_current_target_kept(marine, plain, false));
        assert!(world.a22_current_target_kept(marine, plain, true));
        assert!(!world.a22_current_target_kept(marine, 9999, true));
    }

    #[test]
    fn placement_short_circuit_covers_air_and_buildings() {
        let mut world = open_world();
        let wraith = spawn(&mut world, 0, uid::WRAITH, 640, 640);
        let barracks = spawn(&mut world, 0, uid::BARRACKS, 700, 640);
        let marine = spawn(&mut world, 0, uid::MARINE, 720, 640);

        assert_eq!(world.a22_placement_short_circuit(wraith), Some(0));
        assert_eq!(world.a22_placement_short_circuit(barracks), Some(0));
        assert_eq!(world.a22_placement_short_circuit(marine), None);
        assert_eq!(world.a22_placement_short_circuit(9999), None);
    }

    #[test]
    fn weapon_damage_fixed_matches_the_16_bit_bonus_product() {
        let world = open_world();
        let weapon_id = world.data.unit(uid::MARINE).ground_weapon;
        let weapon = world.data.weapon(weapon_id).unwrap().clone();
        let expected_bonus = (u32::from(weapon.damage_bonus) * 2) & 0xffff;

        assert_eq!(
            world.a22_weapon_damage_fixed(weapon_id, 2, 7),
            ((expected_bonus as i32) + 7) << 8
        );
        assert_eq!(world.a22_weapon_damage_fixed(0x82, 3, 9), 0);
    }

    #[test]
    fn placement_code_is_raised_when_the_target_stands_higher() {
        let mut world = open_world();
        let source = spawn(&mut world, 0, uid::MARINE, 320, 320);
        let target = spawn(&mut world, 1, uid::MARINE, 640, 640);

        assert_eq!(
            world.a22_placement_code_with_elevation(source, target, 5),
            Some(5)
        );

        // The target sits at walk cell (80, 80); give its cell high ground.
        world.terrain.set_elevation_cell(80, 80, 2);
        assert_eq!(
            world.a22_placement_code_with_elevation(source, target, 5),
            Some(0x77)
        );
        assert_eq!(
            world.a22_placement_code_with_elevation(source, target, 0x80),
            Some(0x80)
        );

        // Air units ignore the elevation relationship.
        let ti = index(&world, target);
        world.units[ti].lifted = true;
        assert_eq!(
            world.a22_placement_code_with_elevation(source, target, 5),
            Some(5)
        );
        assert_eq!(
            world.a22_placement_code_with_elevation(source, 9999, 5),
            None
        );
    }

    #[test]
    fn tile_elevation_uses_the_megatile_height_flags() {
        let mut world = open_world();
        assert_eq!(world.a22_tile_elevation_at(640, 640), 0);

        // Raising all 16 walk cells of one megatile sets its high flag.
        for wy in 80..84 {
            for wx in 80..84 {
                world.terrain.set_elevation_cell(wx, wy, 2);
            }
        }
        assert_eq!(world.a22_tile_elevation_at(20 * 32 + 16, 20 * 32 + 16), 1);
        assert_eq!(world.a22_tile_elevation_at(19 * 32 + 16, 20 * 32 + 16), 0);
    }

    #[test]
    fn inset_rectangle_center_uses_quarter_extents() {
        let mut world = open_world();
        let marine = spawn(&mut world, 0, uid::MARINE, 640, 640);
        let i = index(&world, marine);
        assert_eq!(
            world.units[i].a22_inset_rect_center((0, 0, 80, 40)),
            (40, 20)
        );
        assert_eq!(
            world.units[i].a22_inset_rect_center((0, 0, 100, 100)),
            (50, 50)
        );
        assert_eq!(
            world.units[i].a22_inset_rect_center((10, 20, 30, 60)),
            (20, 40)
        );
    }

    #[test]
    fn nearby_target_propagation_filters_owner_worker_and_completion() {
        let mut world = open_world();
        let source = spawn(&mut world, 0, uid::MARINE, 640, 640);
        let friend_a = spawn(&mut world, 0, uid::MARINE, 656, 656);
        let friend_b = spawn(&mut world, 0, uid::MARINE, 672, 640);
        let worker = spawn(&mut world, 0, uid::SCV, 640, 672);
        let completed = spawn(&mut world, 0, uid::MARINE, 640, 608);
        let target = spawn(&mut world, 1, uid::MARINE, 700, 700);

        for id in [friend_a, friend_b, worker] {
            let i = index(&world, id);
            world.units[i].completed = false;
        }

        assert_eq!(
            world.a22_link_target_to_nearby_units(source, target, 0x60),
            2
        );
        assert_eq!(world.units[index(&world, friend_a)].target, Some(target));
        assert_eq!(world.units[index(&world, friend_b)].target, Some(target));
        assert_eq!(world.units[index(&world, worker)].target, None);
        assert_eq!(world.units[index(&world, completed)].target, None);

        // A second application changes nothing because every candidate
        // already carries the target.
        assert_eq!(
            world.a22_link_target_to_nearby_units(source, target, 0x60),
            0
        );
    }

    #[test]
    fn target_reaches_the_subunit_only_when_it_carries_the_subunit_flag() {
        let mut world = open_world();
        let goliath = spawn(&mut world, 0, uid::GOLIATH, 640, 640);
        let turret = spawn(&mut world, 0, uid::GOLIATH_TURRET, 640, 640);
        let marine = spawn(&mut world, 0, uid::MARINE, 660, 640);
        let enemy = spawn(&mut world, 1, uid::MARINE, 700, 640);
        let gi = index(&world, goliath);
        let ti = index(&world, turret);
        world.units[gi].field_090 = 3;
        world.units[ti].field_090 = 9;

        assert!(world.a22_propagate_target_to_subunit(goliath, turret, enemy));
        assert_eq!(world.units[gi].target, Some(enemy));
        assert_eq!(world.units[ti].target, Some(enemy));
        assert_eq!(world.units[ti].field_090, 3);

        assert!(!world.a22_propagate_target_to_subunit(goliath, marine, enemy));
        assert_eq!(world.units[gi].target, Some(enemy));
        assert_eq!(world.units[index(&world, marine)].target, None);
    }

    #[test]
    fn target_propagation_rect_is_the_sixty_pixel_square() {
        let mut world = open_world();
        let marine = spawn(&mut world, 0, uid::MARINE, 640, 640);
        assert_eq!(
            world.a22_target_propagation_rect(marine),
            Some((544, 544, 736, 736))
        );
        assert_eq!(world.a22_target_propagation_rect(9999), None);
    }

    #[test]
    fn snap_to_order_target_copies_the_position_once() {
        let mut world = open_world();
        let marine = spawn(&mut world, 0, uid::MARINE, 100, 100);
        let target = spawn(&mut world, 1, uid::MARINE, 640, 640);
        assert_eq!(world.a22_snap_to_order_target(marine, target), Some(true));
        assert_eq!(world.units[index(&world, marine)].x, 640 * fp(1));
        assert_eq!(world.units[index(&world, marine)].y, 640 * fp(1));
        assert_eq!(world.a22_snap_to_order_target(marine, target), Some(false));
        assert_eq!(world.a22_snap_to_order_target(marine, 9999), None);
    }

    #[test]
    fn order_target_rect_center_follows_the_inset_rule() {
        let mut world = open_world();
        let marine = spawn(&mut world, 0, uid::MARINE, 640, 640);
        let enemy = spawn(&mut world, 1, uid::MARINE, 700, 640);
        let mi = index(&world, marine);
        world.units[mi].order = Order::AttackUnit { target: enemy };

        let rect = world
            .data
            .unit(world.units[index(&world, enemy)].type_id)
            .dimensions
            .rect_at((700, 640));
        let expected = world.units[mi].a22_inset_rect_center(rect);
        assert_eq!(world.a22_order_target_rect_center(marine), Some(expected));
        assert_eq!(expected, (700, 640));

        world.units[mi].order = Order::Idle;
        assert_eq!(world.a22_order_target_rect_center(marine), None);
    }
}
