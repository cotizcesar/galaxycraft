#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent24.rs
// ===========================================================================
// Translation batch 24.  Only this file is edited by the batch.
//
// `0x005a6000..=0x005a7fff` is the retail unit-sprite/overlay helper block.
// Every routine reconstructed below is a `decompiled_uncited` row from that
// range.  Retail reaches the sprite state through `CUnit+0x0c`, the `CSprite`
// object; [`crate::sim::SimUnit`] keeps no sprite objects, so the recovered
// sprite-local values (the state byte and the child-image lists) are explicit
// arguments or return values here, never new fields.  The `CUnit` fields the
// routines also read come from the existing public model: `owner`, `hp`,
// `type_id`, `mask_121` (`CUnit+0x121`), `visibility_mask` (`CUnit+0xe4`) and
// the `units.dat` flag word.
//
// Several ledger rows are shared entries in the middle of a larger retail
// routine (the sprite code calls them directly with registers already set);
// they are translated as the corresponding fragment and the surrounding
// routine is reconstructed separately.  Helper routines that were not
// recovered in this batch are described by role only, never by a claimed
// address.

/// Which sprite answered the child-image search of `0x005a6280`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A24OverlayOwner {
    /// The unit's own sprite (`CUnit+0x0c`).
    Unit,
    /// The addon's sprite, reached through the `CUnit+0x70` companion.
    Addon,
}

/// Route the sprite refresh of `0x005a64c0` takes after a type/addon change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A24SpriteRoute {
    /// `units.dat` flag `0x20000000`: reset the sprite direction bits.
    ResetDirection,
    /// Ordinary sprite-state restore (the shared ordinary tail).
    Restore,
    /// Ordinary restore followed by the local player's Pylon-only update.
    PylonUpdate,
}

/// How `0x005a6530` applies the caller's overlay byte to the sprite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A24OverlayApply {
    /// Image-only types clip the existing image and take the owner variant.
    Clip { image: u8, variant: u8 },
    /// Ordinary types set a new frame and the owner variant.
    Frame { image: u8, variant: u8, pylon: bool },
}

/// `0x005a6280`: search the unit's child-image list for an id in `lo..=hi`,
/// then (only when the unit has an addon) repeat the search on the addon's
/// list.  Retail makes the found child current by calling its image helper;
/// that operation is sprite-side, so the translated routine reports which
/// sprite owns the match.  `None` is the early return that leaves both sprites
/// untouched.  The caller passes the two child-id lists because the retail
/// lists live in the sprite object.
pub fn a24_overlay_child_owner(
    unit_images: &[u16],
    addon_images: Option<&[u16]>,
    lo: u16,
    hi: u16,
) -> Option<A24OverlayOwner> {
    let matches = |images: &[u16]| images.iter().any(|&id| lo <= id && id <= hi);
    if matches(unit_images) {
        return Some(A24OverlayOwner::Unit);
    }
    let addon_images = addon_images?;
    matches(addon_images).then_some(A24OverlayOwner::Addon)
}

/// `0x005a64a0`: convert the caller's overlay byte to the five-bit image slot
/// retail passes to its sprite image constructor:
/// `((value - 0x7c) >> 3) & 0x1f`.  The subtraction wraps, matching the exact
/// byte arithmetic of the retail code.
pub fn a24_overlay_index(value: u8) -> u8 {
    (value.wrapping_sub(0x7c) >> 3) & 0x1f
}

/// `0x005a65f0`: advance the sprite mirror/direction field (`bits 1-2`) down
/// one step modulo four.  Returns the new state and whether retail then asks
/// the image helper for the fallback image byte, which happens exactly when the
/// new direction field became zero.  A state whose field is already zero is
/// left alone.
pub fn a24_cycle_sprite_mirror(state: u8) -> (u8, bool) {
    let direction = state >> 1;
    if direction & 3 == 0 {
        return (state, false);
    }
    let updated = (((direction.wrapping_sub(1) << 1) ^ state) & 6) ^ state;
    (updated, (updated & 6) == 0)
}

/// `0x005a6600`: clear the sprite mirror/direction field.  `None` when the
/// field is already zero (retail returns early with zero); otherwise the
/// cleared state, which is also the branch where retail asks the image helper
/// for the fallback image byte and returns that byte instead.
pub fn a24_clear_sprite_mirror(state: u8) -> Option<u8> {
    if state & 6 == 0 {
        None
    } else {
        Some(state & !6)
    }
}

/// The shared image-state step behind [`World::a24_apply_owner_variant`]:
/// move the sprite mirror/direction field (`bits 1-2`) one step towards the
/// owner's colour variant.  Retail looks the variant up in the per-owner byte
/// table and then runs the image helper it shares with the two routines above:
/// the direction is advanced by one (`(dir + 1) & 3`) and, when the resulting
/// direction is 0 or 1 and no image is available, the direction bits are
/// cleared.  `owner_variant` is the raw table byte; the comparison is against
/// `owner_variant - 1` in full byte width, exactly as retail compares it.
pub fn a24_step_sprite_mirror(state: u8, owner_variant: u8, image_available: bool) -> u8 {
    let direction = (state >> 1) & 3;
    if direction == owner_variant.wrapping_sub(1) {
        return state;
    }
    let mut updated = ((state.wrapping_add(2) ^ state) & 6) ^ state;
    if (updated & 6) < 3 && !image_available {
        updated &= !6;
    }
    updated
}

/// `0x005a67c6`: build the 32-bit overlay flag word.  The CTF Flag row `0xd7`
/// with subtype byte `0x11` short-circuits to `0xff`.  Every other unit starts
/// from the `CUnit+0x121` mask, ORs the flag-table entry for owners `0..8`, and
/// ORs one entry per set `CUnit+0x121` bit (bits 0-11).  Retail indexes one
/// twelve-entry dword table (`0x00fc..0x0128` off the recovered data base) for
/// both the owner and the mask-bit lookups; the caller supplies that table.
pub fn a24_overlay_flag_word(
    type_id: u16,
    subtype_4d: u8,
    mask_121: u8,
    owner: u8,
    flags: &[u32; 12],
) -> u32 {
    if type_id == 0xd7 && subtype_4d == 0x11 {
        return 0xff;
    }
    let mut word = u32::from(mask_121);
    if owner < 8 {
        word |= flags[usize::from(owner)];
    }
    for (bit, &value) in flags.iter().enumerate() {
        if u32::from(mask_121) & (1u32 << bit) != 0 {
            word |= value;
        }
    }
    word
}

/// Which overlay code the builder of `0x005a6780` uses next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A24OverlayCode {
    /// The Nuclear Missile row returns without an overlay.
    Return,
    /// The capability probe short-circuits the flag assembly with `0xff`.
    ShortCircuit(u8),
    /// Fall through to the `CUnit+0x121` mask/flag-table assembly.
    Assemble,
}

impl World {
    /// `0x005a61d0`: move a unit's damage-overlay frame towards the frame the
    /// current HP selects.  Retail stores the current frame in `CUnit+0xa7`
    /// and steps it one frame at a time, calling the sprite frame helper once
    /// per step.  The method returns the target frame and the number of
    /// one-frame sprite steps needed to reach it.
    ///
    /// With `m` the raw maximum HP (`0x00f8eab0[type]`), `t = m / 3` (the
    /// recovered max-HP-third lookup, [`World::unit_max_hp_third`]), `n =
    /// frame_count` and `span = (m - t) / n`, the target is
    /// `max(0, (hp - ((m - t) % span) - t - 1) / span)`, truncated to eight
    /// bits.  `frame_count` is the retail sprite byte plus one and is clamped
    /// to one here so the translated division is defined.
    pub fn a24_damage_overlay_transition(
        &self,
        unit_id: u32,
        frame_count: u8,
        current_frame: u8,
    ) -> Option<(u8, u32)> {
        let index = self.index_of(unit_id)?;
        let unit = &self.units[index];
        let max_hp = self.unit_max_hp_raw(unit.type_id);
        let third = max_hp / 3;
        let span = (max_hp - third) / i32::from(frame_count.max(1));
        let target = if span <= 0 {
            0
        } else {
            let remainder = (max_hp - third) % span;
            let value = (unit.hp - remainder - third - 1) / span;
            if value <= 0 { 0 } else { (value & 0xff) as u8 }
        };
        Some((target, u32::from(current_frame.abs_diff(target))))
    }

    /// `0x005a62c0`: choose the unit whose sprite receives an overlay image.
    /// Retail queries the `units.dat` subunit flag (`0x10`): a subunit row
    /// forwards the image to its parent at `CUnit+0x70` and records that parent
    /// in its 256-slot global table under the caller's byte.  Every other unit
    /// receives the image itself and does not touch the slot table.
    ///
    /// Returns the receiving unit id.  `None` is used for an unknown unit and
    /// for a subunit without a parent, the case retail would fault on when it
    /// dereferences the null `CUnit+0x70`.
    pub fn a24_register_subunit_overlay(
        &self,
        unit_id: u32,
        parent: Option<u32>,
        slot: u8,
        slots: &mut [Option<u32>; 256],
    ) -> Option<u32> {
        let index = self.index_of(unit_id)?;
        if self.data.unit(self.units[index].type_id).is_subunit() {
            let parent = parent?;
            slots[usize::from(slot)] = Some(parent);
            Some(parent)
        } else {
            Some(unit_id)
        }
    }

    /// `0x005a64c0` (the ordinary branch is the shared tail): route the
    /// sprite refresh after a type or addon change.  Types with `units.dat`
    /// flag `0x20000000` (image-only effect rows such as Disruption Web) take
    /// the reset path that clears the sprite direction bits and flag `0x08`.
    /// All other types restore the sprite state; when the game has not
    /// started, the unit belongs to the local player, its `CUnit+0xdc` bit 0
    /// is set (caller-supplied, not modelled) and the row is the Pylon `0x9c`,
    /// the Pylon-only update follows.
    pub fn a24_sprite_refresh_route(
        &self,
        unit_id: u32,
        game_started: bool,
        local_owner: u8,
        status_bit0: bool,
    ) -> Option<A24SpriteRoute> {
        let index = self.index_of(unit_id)?;
        let unit = &self.units[index];
        if self.data.unit(unit.type_id).flags & 0x2000_0000 != 0 {
            return Some(A24SpriteRoute::ResetDirection);
        }
        if !game_started
            && unit.owner == local_owner
            && status_bit0
            && unit.a01_pylon_update_required()
        {
            return Some(A24SpriteRoute::PylonUpdate);
        }
        Some(A24SpriteRoute::Restore)
    }

    /// `0x005a6530` (the ordinary branch is the `0x005a658f` tail): apply the
    /// caller's overlay byte to a unit's sprite.  Image-only types (flag
    /// `0x20000000`) clip the existing image, set the `0x08` sprite state bit
    /// and store the overlay byte.  Ordinary types set a new frame with the
    /// overlay byte and the owner variant, then run the same local-player
    /// Pylon tail as [`World::a24_sprite_refresh_route`].
    ///
    /// `owner_variant` is the per-owner byte retail reads from its colour table
    /// (`0x00f2acf8 + owner`); `overlay` is the caller's image/state byte.
    pub fn a24_overlay_apply(
        &self,
        unit_id: u32,
        overlay: u8,
        owner_variant: u8,
        game_started: bool,
        local_owner: u8,
        status_bit0: bool,
    ) -> Option<A24OverlayApply> {
        let index = self.index_of(unit_id)?;
        let unit = &self.units[index];
        if self.data.unit(unit.type_id).flags & 0x2000_0000 != 0 {
            return Some(A24OverlayApply::Clip {
                image: overlay,
                variant: owner_variant,
            });
        }
        let (image, variant, pylon) = self.a24_overlay_apply_frame(
            unit_id,
            overlay,
            owner_variant,
            game_started,
            local_owner,
            status_bit0,
        )?;
        Some(A24OverlayApply::Frame {
            image,
            variant,
            pylon,
        })
    }

    /// `0x005a658f`: the ordinary overlay branch as a directly callable entry.
    /// Retail jumps here from the sprite code that has already selected the
    /// unit, writes the frame/overlay byte and the owner variant to the sprite,
    /// and then runs the local-player Pylon tail.  The sprite write itself is
    /// not modelled, so the method reports the values that would be written
    /// plus whether the Pylon-only update follows.  `None` for an unknown unit.
    pub fn a24_overlay_apply_frame(
        &self,
        unit_id: u32,
        overlay: u8,
        owner_variant: u8,
        game_started: bool,
        local_owner: u8,
        status_bit0: bool,
    ) -> Option<(u8, u8, bool)> {
        let index = self.index_of(unit_id)?;
        let unit = &self.units[index];
        let pylon = !game_started
            && unit.owner == local_owner
            && status_bit0
            && unit.a01_pylon_update_required();
        Some((overlay, owner_variant, pylon))
    }

    /// `0x005a6610`: move a unit's sprite mirror/direction byte towards the
    /// owner's colour variant.  Retail skips a unit without a sprite and reads
    /// the variant from the per-owner byte table, so the caller supplies the
    /// table.  `None` means the unit is unknown or its owner has no table entry
    /// (retail's table is indexed by the raw owner byte).
    pub fn a24_apply_owner_variant(
        &self,
        unit_id: u32,
        state: u8,
        variants: &[u8],
        image_available: bool,
    ) -> Option<u8> {
        let index = self.index_of(unit_id)?;
        let variant = *variants.get(usize::from(self.units[index].owner))?;
        Some(a24_step_sprite_mirror(state, variant, image_available))
    }

    /// `0x005a6780`: status-overlay draw gate.  Retail returns before touching
    /// the sprite when the owner is outside `0..8` and the `CUnit+0x121` mask
    /// is empty, and for the Nuclear Missile row `0x0e`; everything else
    /// proceeds to the overlay builder.  `false` is that early return (also
    /// used for an unknown id).
    pub fn a24_overlay_draw_gate(&self, unit_id: u32) -> bool {
        let Some(index) = self.index_of(unit_id) else {
            return false;
        };
        let unit = &self.units[index];
        if unit.owner >= 8 && unit.mask_121() == 0 {
            return false;
        }
        unit.type_id != 0x0e
    }
}

impl SimUnit {
    /// `0x005a67a4`: overlay-code selection.  The Nuclear Missile row `0x0e`
    /// returns without a code.  A unit that passes the capability probe
    /// (passed in as `capable`) short-circuits the flag
    /// assembly with `0xff`; every other unit falls through to the
    /// `CUnit+0x121` mask/table assembly of [`a24_overlay_flag_word`].
    ///
    /// The ledger's decompiled body is a fragment entered in the middle of the
    /// builder with caller-owned registers, so it reads the shared draw tail as
    /// a non-returning helper.  The recovered instruction stream only branches
    /// to that tail, which is why the fall-through is kept as `Assemble`.
    pub fn a24_overlay_code(&self, capable: bool) -> A24OverlayCode {
        if self.type_id == 0x0e {
            return A24OverlayCode::Return;
        }
        if capable {
            return A24OverlayCode::ShortCircuit(0xff);
        }
        A24OverlayCode::Assemble
    }

    /// `0x005a6909`: the shared draw tail's cloak gate.  The ledger body is a
    /// fragment entered with caller-owned registers: `BL` carries the
    /// `CUnit+0xdc` cloak field (bits 8-9) that the draw wrapper read, and the
    /// tail applies the cloak offset/shimmer only when that field is non-zero
    /// and the unit is visible to the current player (`CUnit+0xe4` mask bit).
    /// [`SimUnit::cloak_state`] already holds that field, so the fragment is
    /// exposed as the gate itself; the image draw and the status-overlay
    /// update that follow are renderer-side and are not modelled here.
    pub fn a24_cloak_shimmer_visible(&self, viewer: u8) -> bool {
        self.cloak_state() != 0 && self.visibility_mask & (1u32 << (viewer & 31)) != 0
    }
}

#[cfg(test)]
mod tests_agent24 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
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

    fn spawn(world: &mut World, owner: usize, type_id: u16) -> u32 {
        let slot = world.spawn(owner, type_id, 320, 320).expect("spawn");
        world.units[slot].id
    }

    fn slot_of(world: &World, id: u32) -> usize {
        world.index_of(id).expect("unit present")
    }

    #[test]
    fn damage_overlay_frame_follows_hp_third() {
        let mut w = world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        // Raw maximum HP 10240, one third 3413, three frames: span 2275,
        // remainder 2.  Full HP selects frame 2, medium HP frame 0.
        let i = slot_of(&w, marine);
        w.units[i].hp = 10240;
        assert_eq!(w.a24_damage_overlay_transition(marine, 3, 0), Some((2, 2)));
        assert_eq!(w.a24_damage_overlay_transition(marine, 3, 2), Some((2, 0)));
        assert_eq!(w.a24_damage_overlay_transition(marine, 3, 3), Some((2, 1)));

        w.units[i].hp = 7000;
        assert_eq!(w.a24_damage_overlay_transition(marine, 3, 0), Some((1, 1)));

        w.units[i].hp = 5120;
        assert_eq!(w.a24_damage_overlay_transition(marine, 3, 0), Some((0, 0)));

        w.units[i].hp = 1;
        assert_eq!(w.a24_damage_overlay_transition(marine, 3, 0), Some((0, 0)));

        // A zero frame count is clamped to one (retail always adds one), which
        // makes the whole HP range a single frame.
        w.units[i].hp = 10240;
        assert_eq!(w.a24_damage_overlay_transition(marine, 0, 0), Some((0, 0)));

        assert_eq!(w.a24_damage_overlay_transition(0xdead_beef, 3, 0), None);
    }

    #[test]
    fn child_owner_prefers_unit_then_addon() {
        let unit = [0x100u16, 0x17c, 0x180];
        assert_eq!(
            a24_overlay_child_owner(&unit, Some(&[0x17d]), 0x17c, 0x17e),
            Some(A24OverlayOwner::Unit)
        );
        assert_eq!(
            a24_overlay_child_owner(&unit, Some(&[0x17d]), 0x17d, 0x17e),
            Some(A24OverlayOwner::Addon)
        );
        assert_eq!(a24_overlay_child_owner(&unit, None, 0x17d, 0x17e), None);
        assert_eq!(
            a24_overlay_child_owner(&unit, Some(&[0x17f]), 0x17d, 0x17e),
            None
        );
        // An empty range matches nothing, exactly like the retail comparisons.
        assert_eq!(
            a24_overlay_child_owner(&unit, Some(&unit), 0x17e, 0x17d),
            None
        );
    }

    #[test]
    fn subunit_overlay_forwards_to_parent_and_fills_slot() {
        let mut w = world();
        let turret = spawn(&mut w, 0, uid::GOLIATH_TURRET);
        let marine = spawn(&mut w, 0, uid::MARINE);
        let parent = spawn(&mut w, 0, uid::GOLIATH);
        let mut slots = [None; 256];

        assert_eq!(
            w.a24_register_subunit_overlay(turret, Some(parent), 7, &mut slots),
            Some(parent)
        );
        assert_eq!(slots[7], Some(parent));
        assert!(w.data.unit(uid::GOLIATH_TURRET).is_subunit());

        assert_eq!(
            w.a24_register_subunit_overlay(marine, Some(parent), 9, &mut slots),
            Some(marine)
        );
        assert_eq!(slots[9], None, "ordinary units do not write the slot table");

        assert_eq!(
            w.a24_register_subunit_overlay(turret, None, 7, &mut slots),
            None
        );
        assert_eq!(
            w.a24_register_subunit_overlay(0xdead_beef, None, 0, &mut slots),
            None
        );
    }

    #[test]
    fn overlay_index_is_a_five_bit_slot() {
        assert_eq!(a24_overlay_index(0x7c), 0);
        assert_eq!(a24_overlay_index(0x84), 1);
        assert_eq!(a24_overlay_index(0x7b), 0x1f);
        assert_eq!(a24_overlay_index(0x7f), 0);
        assert_eq!(a24_overlay_index(0x80), 0);
        assert_eq!(a24_overlay_index(0xff), 0x10);
        // Every input stays inside the five-bit slot range.
        for value in 0..=255u8 {
            assert!(a24_overlay_index(value) < 32);
        }
    }

    #[test]
    fn sprite_refresh_route_uses_effect_flag_and_pylon_tail() {
        let mut w = world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        let effect = spawn(&mut w, 0, uid::DISRUPTION_WEB);
        let pylon = spawn(&mut w, 0, uid::PYLON);

        assert!(w.data.unit(uid::DISRUPTION_WEB).flags & 0x2000_0000 != 0);
        assert_eq!(
            w.a24_sprite_refresh_route(effect, false, 0, false),
            Some(A24SpriteRoute::ResetDirection)
        );
        assert_eq!(
            w.a24_sprite_refresh_route(marine, false, 0, true),
            Some(A24SpriteRoute::Restore)
        );
        assert_eq!(
            w.a24_sprite_refresh_route(pylon, false, 0, true),
            Some(A24SpriteRoute::PylonUpdate)
        );
        // Wrong owner or a started game drops the Pylon tail.
        assert_eq!(
            w.a24_sprite_refresh_route(pylon, false, 1, true),
            Some(A24SpriteRoute::Restore)
        );
        assert_eq!(
            w.a24_sprite_refresh_route(pylon, true, 0, true),
            Some(A24SpriteRoute::Restore)
        );
        // Status bit 0 is required for the Pylon update.
        assert_eq!(
            w.a24_sprite_refresh_route(pylon, false, 0, false),
            Some(A24SpriteRoute::Restore)
        );
        assert_eq!(
            w.a24_sprite_refresh_route(0xdead_beef, false, 0, true),
            None
        );
    }

    #[test]
    fn overlay_apply_selects_clip_or_frame() {
        let mut w = world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        let effect = spawn(&mut w, 0, uid::DISRUPTION_WEB);
        let pylon = spawn(&mut w, 0, uid::PYLON);

        assert_eq!(
            w.a24_overlay_apply(effect, 0x21, 3, false, 0, true),
            Some(A24OverlayApply::Clip {
                image: 0x21,
                variant: 3
            })
        );
        assert_eq!(
            w.a24_overlay_apply(marine, 0x21, 3, false, 0, true),
            Some(A24OverlayApply::Frame {
                image: 0x21,
                variant: 3,
                pylon: false
            })
        );
        assert_eq!(
            w.a24_overlay_apply(pylon, 0x21, 3, false, 0, true),
            Some(A24OverlayApply::Frame {
                image: 0x21,
                variant: 3,
                pylon: true
            })
        );
        assert_eq!(
            w.a24_overlay_apply(pylon, 0x21, 3, true, 0, true),
            Some(A24OverlayApply::Frame {
                image: 0x21,
                variant: 3,
                pylon: false
            })
        );
        assert_eq!(
            w.a24_overlay_apply(0xdead_beef, 0, 0, false, 0, false),
            None
        );
    }

    #[test]
    fn sprite_mirror_cycle_and_clear_match_byte_arithmetic() {
        // Direction 1 -> 0 and the fallback image byte is requested.
        assert_eq!(a24_cycle_sprite_mirror(0b0000_0010), (0, true));
        // Direction 2 -> 1, fallback not requested.
        assert_eq!(a24_cycle_sprite_mirror(0b0000_0100), (0b0000_0010, false));
        // Direction 3 -> 2.
        assert_eq!(a24_cycle_sprite_mirror(0b0000_0110), (0b0000_0100, false));
        // Unrelated state bits survive.
        assert_eq!(a24_cycle_sprite_mirror(0b1010_0010), (0b1010_0000, true));
        // Direction 0 is untouched.
        assert_eq!(a24_cycle_sprite_mirror(0b1010_0000), (0b1010_0000, false));
        assert_eq!(a24_cycle_sprite_mirror(0b1010_0001), (0b1010_0001, false));

        assert_eq!(a24_clear_sprite_mirror(0b0000_0110), Some(0b0000_0000));
        assert_eq!(a24_clear_sprite_mirror(0b1010_0110), Some(0b1010_0000));
        assert_eq!(a24_clear_sprite_mirror(0b1010_0001), None);
    }

    #[test]
    fn owner_variant_step_moves_one_direction() {
        // Variant 3 asks for direction 2; direction 1 steps to 2.
        assert_eq!(a24_step_sprite_mirror(0b0000_0010, 3, true), 0b0000_0100);
        // Same direction: untouched.
        assert_eq!(a24_step_sprite_mirror(0b0000_0100, 3, true), 0b0000_0100);
        // Variant 0 compares against 0xff in the retail byte width, so it
        // always takes a step; the resulting direction 0 clears when no image.
        assert_eq!(a24_step_sprite_mirror(0b0000_0110, 0, false), 0b0000_0000);
        // Direction 2 is not below 3, so the image flag does not clear it.
        assert_eq!(a24_step_sprite_mirror(0b0000_0010, 0, false), 0b0000_0100);
    }

    #[test]
    fn owner_variant_lookup_uses_unit_owner() {
        let mut w = world();
        let marine = spawn(&mut w, 1, uid::MARINE);
        let variants = [2u8, 3, 4];
        assert_eq!(
            w.a24_apply_owner_variant(marine, 0b0000_0010, &variants, true),
            Some(0b0000_0100)
        );
        assert_eq!(
            w.a24_apply_owner_variant(marine, 0b0000_0100, &variants, true),
            Some(0b0000_0100)
        );
        // No table entry for the owner: retail would index past its table.
        assert_eq!(
            w.a24_apply_owner_variant(marine, 0b0000_0010, &[2u8], true),
            None
        );
        assert_eq!(
            w.a24_apply_owner_variant(0xdead_beef, 0, &variants, true),
            None
        );
    }

    #[test]
    fn overlay_draw_gate_rejects_unknown_owner_and_nuke() {
        let mut w = world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        let nuke = spawn(&mut w, 0, uid::NUCLEAR_MISSILE);
        assert!(w.a24_overlay_draw_gate(marine));
        assert!(!w.a24_overlay_draw_gate(nuke));

        let i = slot_of(&w, marine);
        w.units[i].owner = 8;
        assert!(!w.a24_overlay_draw_gate(marine));
        w.units[i].set_mask_121_bit(0);
        assert!(w.a24_overlay_draw_gate(marine));
        assert!(!w.a24_overlay_draw_gate(0xdead_beef));
    }

    #[test]
    fn overlay_code_selects_return_short_circuit_or_assembly() {
        let mut w = world();
        let nuke = spawn(&mut w, 0, uid::NUCLEAR_MISSILE);
        let marine = spawn(&mut w, 0, uid::MARINE);
        let nuke_idx = slot_of(&w, nuke);
        let marine_idx = slot_of(&w, marine);
        assert_eq!(
            w.units[nuke_idx].a24_overlay_code(false),
            A24OverlayCode::Return
        );
        assert_eq!(
            w.units[marine_idx].a24_overlay_code(true),
            A24OverlayCode::ShortCircuit(0xff)
        );
        assert_eq!(
            w.units[marine_idx].a24_overlay_code(false),
            A24OverlayCode::Assemble
        );
    }

    #[test]
    fn overlay_frame_entry_reports_pylon_tail() {
        let mut w = world();
        let marine = spawn(&mut w, 0, uid::MARINE);
        let pylon = spawn(&mut w, 0, uid::PYLON);
        assert_eq!(
            w.a24_overlay_apply_frame(marine, 0x21, 3, false, 0, true),
            Some((0x21, 3, false))
        );
        assert_eq!(
            w.a24_overlay_apply_frame(pylon, 0x21, 3, false, 0, true),
            Some((0x21, 3, true))
        );
        assert_eq!(
            w.a24_overlay_apply_frame(pylon, 0x21, 3, true, 0, true),
            Some((0x21, 3, false))
        );
        assert_eq!(
            w.a24_overlay_apply_frame(0xdead_beef, 0x21, 3, false, 0, true),
            None
        );
    }

    #[test]
    fn cloak_shimmer_needs_a_cloaked_visible_unit() {
        let mut w = world();
        let ghost = spawn(&mut w, 0, uid::GHOST);
        let i = slot_of(&w, ghost);
        // Uncloaked units take the plain draw path.
        assert!(!w.units[i].a24_cloak_shimmer_visible(0));
        w.units[i].status.cloaked = true;
        w.units[i].visibility_mask = u32::MAX;
        assert!(w.units[i].a24_cloak_shimmer_visible(0));
        w.units[i].visibility_mask = 0;
        assert!(!w.units[i].a24_cloak_shimmer_visible(0));
        w.units[i].visibility_mask = 1;
        assert!(w.units[i].a24_cloak_shimmer_visible(0));
        assert!(!w.units[i].a24_cloak_shimmer_visible(1));
    }

    #[test]
    fn overlay_flag_word_ors_owner_and_mask_entries() {
        let mut table = [0u32; 12];
        for (i, value) in table.iter_mut().enumerate() {
            *value = 0x100 + i as u32;
        }

        // Unknown-owner path: only the mask bits contribute.
        assert_eq!(
            a24_overlay_flag_word(uid::MARINE, 0, 0b0000_1001, 9, &table),
            0b1001 | 0x100 | 0x103
        );
        // Owner 2 adds its entry.
        assert_eq!(
            a24_overlay_flag_word(uid::MARINE, 0, 0b0000_1001, 2, &table),
            0b1001 | 0x100 | 0x103 | 0x102
        );
        // Empty mask and owner 9 keep the word at zero.
        assert_eq!(a24_overlay_flag_word(uid::MARINE, 0, 0, 9, &table), 0);
        // The CTF Flag row with subtype 0x11 short-circuits to 0xff.
        assert_eq!(a24_overlay_flag_word(0xd7, 0x11, 0b1, 0, &table), 0xff);
        assert_eq!(
            a24_overlay_flag_word(0xd7, 0x12, 0b1, 0, &table),
            0b1 | 0x100
        );
    }
}
