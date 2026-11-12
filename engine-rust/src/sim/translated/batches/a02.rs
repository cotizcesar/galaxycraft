#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent02.rs
// ===========================================================================
// Translation batch 02.
//
// Only this file is edited by the batch.  Every item below is a translation
// of one routine in the `0x005a8000..=0x005affff` window whose ledger status
// is `decompiled_uncited`; the decompiled bodies live in
// `~/.cache/galaxycraft/decomp_full/` and the addresses / `FUN_` names come
// from `engine-rust/docs/decomp-ledger.tsv`.
//
// The routines in this window are the selection/status panel helpers of the
// classic client (`CUnit`-adjacent objects with a `u16` accumulator at
// `+0xd0`, a latched cue byte at `+0xd2`, an overlay/state byte at `+0xe0`).
// Three of them reduce to data that the simulation already owns, so they are
// attached to [`SimUnit`]; the rest are pure functions plus the documented
// `A02PanelStep` record.  Nothing here adds state to `World` or `SimUnit`.

// ---------------------------------------------------------------------------
// 0x005ada00 - resource-node type predicate
// ---------------------------------------------------------------------------

/// Unit ids tested by `0x005ada00`: the three mineral-field variants and the
/// vespene geyser (`units.dat` rows 176, 177, 178 and 188).
pub const A02_RESOURCE_NODE_TYPES: [u16; 4] = [0x00b0, 0x00b1, 0x00b2, 0x00bc];

/// `0x005ada00` (`FUN_005ada00`, 58 bytes).
///
/// ```c
/// if ((((param_1 != 0xb0) && (param_1 != 0xb1)) && (param_1 != 0xb2)) &&
///     (param_1 != 0xbc)) {
///   return 0;
/// }
/// return 1;
/// ```
///
/// The routine is a straight four-way membership test on a `short` unit id and
/// is the guard the panel uses before it treats an object as a resource node.
/// The ids are the stock `units.dat` rows for the mineral fields and the
/// geyser, i.e. the same set `SimUnit::is_resource` recognises through the
/// table flags.
pub fn a02_is_resource_node_type(type_id: u16) -> bool {
    matches!(type_id, 0x00b0 | 0x00b1 | 0x00b2 | 0x00bc)
}

// ---------------------------------------------------------------------------
// 0x005ac454 / 0x005ae170 - resource-reserve cue ladder
// ---------------------------------------------------------------------------

/// Cue emitted while the reserve is below 250.
pub const A02_CUE_LOWEST: u8 = 0x0d;
/// Cue emitted for a reserve in `250..500`.
pub const A02_CUE_LOW: u8 = 0x0e;
/// Cue emitted for a reserve in `500..threshold`.
pub const A02_CUE_MID: u8 = 0x0f;
/// Cue emitted once the reserve reaches the threshold.
pub const A02_CUE_HIGH: u8 = 0x14;
/// Threshold baked into `0x005ae170` (`0x2ee` = 750).
pub const A02_RESERVE_THRESHOLD: u16 = 0x02ee;

/// `0x005ac454` (`FUN_005ac454`, 75 bytes).
///
/// The routine receives the live reserve in `DX` and the cut-off in `EAX`:
///
/// ```c
/// if (in_DX < (ushort)in_EAX) {
///   if (in_DX < 500) { cVar1 = '\x0e' - (in_DX < 0xfa); }
///   else             { cVar1 = '\x0f'; }
/// } else {
///   cVar1 = '\x14';
/// }
/// if (cVar1 != *(char *)(unaff_ESI + 0xd2)) {
///   *(char *)(unaff_ESI + 0xd2) = cVar1;
///   FUN_0058b460(cVar1, 1);          // play the cue
/// }
/// ```
///
/// Three bands below the cut-off (`< 250`, `< 500`, `< threshold`) plus one
/// band at or above it.  The value is an unsigned `u16` in the original, so
/// the comparisons are unsigned.
pub fn a02_threshold_cue(value: u16, threshold: u16) -> u8 {
    if value < threshold {
        if value < 500 {
            A02_CUE_LOW - u8::from(value < 0x00fa)
        } else {
            A02_CUE_MID
        }
    } else {
        A02_CUE_HIGH
    }
}

/// `0x005ae170` (`FUN_005ae170`, 82 bytes).
///
/// The same ladder with the cut-off pinned at `0x2ee` (750), which is the
/// specialisation the shipped panel actually uses:
///
/// ```c
/// uVar1 = *(ushort *)(in_ECX + 0xd0);
/// if (uVar1 < 0x2ee) {
///   if (uVar1 < 500) { cVar2 = '\x0e' - (uVar1 < 0xfa); }
///   else             { cVar2 = '\x0f'; }
/// } else {
///   cVar2 = '\x14';
/// }
/// ```
pub fn a02_reserve_cue(value: u16) -> u8 {
    a02_threshold_cue(value, A02_RESERVE_THRESHOLD)
}

impl SimUnit {
    /// `0x005ae170` (`FUN_005ae170`, 82 bytes): latch the reserve cue.
    ///
    /// The routine compares the newly selected cue against the byte it stores
    /// at `+0xd2` and only plays the sound when the byte changes.  The
    /// simulation keeps the "last cue id" in `CUnit+0x8b`
    /// ([`SimUnit::speech_id`], the same slot the voice timer reads through
    /// `FUN_005b4ae0`), so this method is the change detector: it returns the
    /// cue exactly when the caller has to play it.
    pub fn a02_latch_reserve_cue(&mut self, value: u16) -> Option<u8> {
        self.a02_latch_threshold_cue(value, A02_RESERVE_THRESHOLD)
    }

    /// `0x005ac454` (`FUN_005ac454`, 75 bytes): the variable-threshold form of
    /// [`SimUnit::a02_latch_reserve_cue`].
    pub fn a02_latch_threshold_cue(&mut self, value: u16, threshold: u16) -> Option<u8> {
        let cue = a02_threshold_cue(value, threshold);
        if cue == self.speech_id {
            None
        } else {
            self.speech_id = cue;
            Some(cue)
        }
    }
}

// ---------------------------------------------------------------------------
// 0x005ac29f - dialog sound id and route
// ---------------------------------------------------------------------------

/// Destination of the dialog cue selected by `0x005ac29f`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A02SoundRoute {
    /// `FUN_005b71a0(id)`: the owner slot at `+0xc8` is zero, so the cue is
    /// played through the global channel.
    Global(u8),
    /// `FUN_005b7250(id, owner)`: the cue is forwarded to the owning object.
    ToOwner(u8, u32),
}

/// `0x005ac29f` (`FUN_005ac29f`, 72 bytes).
///
/// ```c
/// iVar1 = (in_EAX & 0xfffffffc) + 0x55;
/// if (*(int *)(unaff_ESI + 200) == 0) { FUN_005b71a0(iVar1); }
/// else                               { FUN_005b7250(iVar1, *(int *)(unaff_ESI + 200)); }
/// ```
///
/// The selector is rounded down to a multiple of four and biased by `0x55`,
/// which is how the panel maps a four-state button/row group onto the
/// consecutive cue block `0x55, 0x59, 0x5d, ...`.
pub fn a02_dialog_sound_id(selector: u32) -> u8 {
    (selector & 0xffff_fffc).wrapping_add(0x55) as u8
}

/// `0x005ac29f` (`FUN_005ac29f`, 72 bytes): pick the cue and its destination.
///
/// `owner` is the `+0xc8` slot; zero means "no owner", exactly like the
/// `*(int *)(unaff_ESI + 200) == 0` test in the decompiled body.
pub fn a02_route_dialog_sound(selector: u32, owner: u32) -> A02SoundRoute {
    let id = a02_dialog_sound_id(selector);
    if owner == 0 {
        A02SoundRoute::Global(id)
    } else {
        A02SoundRoute::ToOwner(id, owner)
    }
}

// ---------------------------------------------------------------------------
// 0x005abdaf - scope counter and overlay-dependent cue
// ---------------------------------------------------------------------------

impl SimUnit {
    /// `0x005abdaf` (`FUN_005abdaf`, 85 bytes): the cue that follows a scope
    /// completion.
    ///
    /// ```c
    /// iVar3 = FUN_005b5630();                       // CUnit+0xe0 bit 1
    /// cue = (-(iVar3 != 0) & 5U) + 0x55;            // 0x5a or 0x55
    /// ```
    ///
    /// `FUN_005b5630` is the recovered `CUnit+0xe0` bit-1 getter
    /// ([`SimUnit::overlay_bit1`]); with the bit set the biased cue is `0x5a`
    /// instead of `0x55`, i.e. exactly `5` higher.
    pub fn a02_scope_cue(&self) -> u8 {
        if self.overlay_bit1() { 0x55 + 5 } else { 0x55 }
    }
}

/// `0x005abdaf` (`FUN_005abdaf`, 85 bytes): the byte counter in front of the
/// cue.
///
/// ```c
/// cVar1 = (char)in_EAX + '\x01' + in_CF;   // wrapping byte bump
/// *pcVar2 = *pcVar2 + cVar1;
/// if (*(char *)(unaff_EBX + 0x54) == cVar1) {
///   FUN_0058b460(8, 1);                    // cue 8
///   ...
/// }
/// ```
///
/// Returns the incremented counter and the cue to play (`Some(8)`) when the
/// new counter equals the scope byte at `+0x54`.
pub fn a02_scope_counter_step(counter: u8, scope: u8) -> (u8, Option<u8>) {
    let next = counter.wrapping_add(1);
    let cue = if next == scope { Some(8) } else { None };
    (next, cue)
}

// ---------------------------------------------------------------------------
// 0x005afdf0 - clear the panel scratch block
// ---------------------------------------------------------------------------

/// Size of the scratch block cleared by `0x005afdf0`, from `+0x110` through
/// `+0x133` inclusive.
pub const A02_EXTRA_BLOCK_LEN: usize = 0x24;

/// `0x005afdf0` (`FUN_005afdf0`, 69 bytes).
///
/// The body stores zero to `+0x110`, `+0x114`, `+0x118`, `+0x11c`, `+0x120`
/// (five `u32`), `+0x124` (`u16`), `+0x126` (`u8`), `+0x127` (eight bytes),
/// `+0x12f` (`u8`) and `+0x130` (`u32`) - one contiguous `0x24`-byte block.
/// Translating it as a block clear keeps the field layout visible without
/// inventing named fields for an unrecovered structure.
pub fn a02_clear_extra_block(block: &mut [u8; A02_EXTRA_BLOCK_LEN]) {
    *block = [0; A02_EXTRA_BLOCK_LEN];
}

// ---------------------------------------------------------------------------
// 0x005ac3dc - resource panel cue table and 8-tick step
// ---------------------------------------------------------------------------

/// `0x005ac3dc` (`FUN_005ac3dc`, ledger size 29; the body runs to the
/// `LAB_005ac5cd` epilogue at `0x005ac5cd`).
///
/// The switch at the head of the routine maps a resource-structure unit id to
/// the cue block the panel shows for it:
///
/// ```c
/// switch (in_AX) {
/// case 0x6e: iVar7 = 0x193; break;                  // Refinery
/// case 0x95: iVar7 = 0x191; break;                  // Extractor
/// case 0x9d: iVar7 = 399;   break;                  // Assimilator
/// case 0xb0: case 0xb1: case 0xb2:                  // mineral fields
///   cVar1 = 1; iVar7 = 0x18d;
///   *(undefined1 *)(unaff_EBP + -0x34) = 1;         // looping/animated
/// }
/// ```
///
/// Returns `(looping, cue_base)`; mineral fields animate (the frame counter
/// runs) and start the cue block one entry later than the gas structures.
pub fn a02_resource_type_cue(type_id: u16) -> Option<(bool, u16)> {
    match type_id {
        0x006e => Some((false, 0x0193)),
        0x0095 => Some((false, 0x0191)),
        0x009d => Some((false, 0x018f)),
        0x00b0..=0x00b2 => Some((true, 0x018d)),
        _ => None,
    }
}

/// One iteration of the `0x005ac3dc` panel timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A02PanelStep {
    /// New value of the `u16` countdown at `+0xd0`.
    pub timer: u16,
    /// Animation frame byte handed to `FUN_005b31c0` as its `param_2`
    /// (`+0xcf` companion byte).
    pub frame: u8,
    /// Cue base plus the frame-dependent offset: `base + 1` while the frame
    /// index is `0..=7`, `base` once the frame reaches 8.
    pub cue_base: u16,
    /// Cue the caller has to play (and latch at `+0xd2`) because it changed.
    pub cue: Option<u8>,
    /// `mode` argument for `FUN_005b31c0` (the `param_3 != 0` test), i.e. the
    /// latched looping flag.
    pub overlay_mode: bool,
    /// The decompiled `FUN_00538810(0x10)` guard fired: after subtracting
    /// eight the non-looping countdown fell below eight.  Retail never
    /// returns from that call, so the remaining fields describe the state the
    /// routine had reached.
    pub guard: bool,
}

/// `0x005ac3dc` (`FUN_005ac3dc`): advance the resource panel countdown.
///
/// `type_id` selects the cue block through [`a02_resource_type_cue`]; `timer`
/// is the `u16` accumulator at `+0xd0`; `frame_now` is the low byte that
/// `FUN_005b7f10` returns for the current animation frame (the helper walks the
/// image list and is out of scope here, so its result is an input).
///
/// ```c
/// if (*(ushort *)(unaff_ESI + 0xd0) < 8) {
///   if (cVar1 == '\0') { *(u16 *)(unaff_ESI + 0xd0) = 0; frame = 2; }
///   else               { frame = (u8)FUN_005b7f10(); }
/// } else {
///   uVar3 = *(ushort *)(unaff_ESI + 0xd0) - 8;
///   *(ushort *)(unaff_ESI + 0xd0) = (u16)uVar3;
///   if (cVar1 == 0)          { if ((u16)uVar3 < 8) guard(); frame = 8; }
///   else if ((u16)uVar3 == 0){ frame = (u8)FUN_005b7f10(); }
///   else                     { cue = ladder(uVar3); frame = 8; }
/// }
/// iVar6 = base + 1; if (7 < frame) iVar6 = base;      // 0x18d..0x193 + 0/1
/// ```
///
/// Returns `None` for a type the switch does not handle (the routine jumps
/// straight to its epilogue).
pub fn a02_resource_panel_step(type_id: u16, timer: u16, frame_now: u8) -> Option<A02PanelStep> {
    let (looping, base) = a02_resource_type_cue(type_id)?;

    let mut guard = false;
    let mut cue = None;

    let (timer_out, frame) = if timer < 8 {
        if looping { (timer, frame_now) } else { (0, 2) }
    } else {
        let remaining = timer.wrapping_sub(8);
        if !looping {
            // Retail calls the non-returning FUN_00538810 here.
            guard = remaining < 8;
            (remaining, 8)
        } else if remaining == 0 {
            (remaining, frame_now)
        } else {
            cue = Some(a02_reserve_cue(remaining));
            (remaining, 8)
        }
    };

    let cue_base = if frame <= 7 { base + 1 } else { base };
    Some(A02PanelStep {
        timer: timer_out,
        frame,
        cue_base,
        cue,
        overlay_mode: looping,
        guard,
    })
}

// ---------------------------------------------------------------------------
// 0x005ad890 - latched looping/flag byte
// ---------------------------------------------------------------------------

/// `0x005ad890` (`FUN_005ad890`, 8 bytes): `return *(u8 *)(this + 0xd3)`.
///
/// The object window is `+0x00..=+0xd3`; the byte sits next to the cue byte
/// `+0xd2`, is copied by the panel copy helpers (`FUN_005aef85`, `FUN_005af13a`,
/// `FUN_005af181`) and carries the same looping flag `0x005ac3dc` derives for
/// animated mineral rows (`1`).  Taking the window as a slice keeps the offset
/// visible and needs no state.
pub fn a02_wire_flag_0d3(window: &[u8; 0xd4]) -> u8 {
    window[0xd3]
}

// ---------------------------------------------------------------------------
// 0x005abec9 - empty hook
// ---------------------------------------------------------------------------

/// `0x005abec9` (`FUN_005abec9`, 4 bytes): a bare `ret`.
///
/// The whole body is `return;`; the routine exists only as a no-op vtable
/// slot, so the translation is an empty function.
pub fn a02_empty_hook() {}

#[cfg(test)]
mod tests_agent02 {
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

    fn unit(world: &mut World, type_id: u16) -> usize {
        let index = world.spawn(0, uid::MARINE, 512, 512).unwrap();
        world.units[index].type_id = type_id;
        index
    }

    #[test]
    fn resource_node_ids_match_the_retail_membership_test() {
        // 0x005ada00
        for id in A02_RESOURCE_NODE_TYPES {
            assert!(a02_is_resource_node_type(id), "id {id:#x} is a node");
        }
        for id in [0x00adu16, 0x00af, 0x00b3, 0x00bb, 0x00bd, 0x00d6] {
            assert!(!a02_is_resource_node_type(id), "id {id:#x} is not");
        }
        // The ids are the stock mineral fields and the geyser.
        assert_eq!(A02_RESOURCE_NODE_TYPES, [176, 177, 178, 188]);
    }

    #[test]
    fn cue_ladder_uses_the_four_bands() {
        // 0x005ac454 / 0x005ae170
        assert_eq!(a02_threshold_cue(0, 750), A02_CUE_LOWEST);
        assert_eq!(a02_threshold_cue(249, 750), A02_CUE_LOWEST);
        assert_eq!(a02_threshold_cue(250, 750), A02_CUE_LOW);
        assert_eq!(a02_threshold_cue(499, 750), A02_CUE_LOW);
        assert_eq!(a02_threshold_cue(500, 750), A02_CUE_MID);
        assert_eq!(a02_threshold_cue(749, 750), A02_CUE_MID);
        assert_eq!(a02_threshold_cue(750, 750), A02_CUE_HIGH);
        assert_eq!(a02_threshold_cue(u16::MAX, 750), A02_CUE_HIGH);
        // The fixed-threshold entry point is the same ladder at 0x2ee.
        assert_eq!(A02_RESERVE_THRESHOLD, 750);
        for value in 0..2_000u16 {
            assert_eq!(a02_reserve_cue(value), a02_threshold_cue(value, 750));
        }
        // A different cut-off moves the top band but not the inner ones.
        assert_eq!(a02_threshold_cue(750, 1_000), A02_CUE_MID);
        assert_eq!(a02_threshold_cue(999, 1_000), A02_CUE_MID);
        assert_eq!(a02_threshold_cue(1_000, 1_000), A02_CUE_HIGH);
    }

    #[test]
    fn reserve_cue_is_latched_and_reported_once() {
        // 0x005ae170: only a changed cue byte plays a sound.
        let mut w = world();
        let i = unit(&mut w, uid::MARINE);
        w.units[i].speech_id = 0;
        assert_eq!(w.units[i].a02_latch_reserve_cue(400), Some(A02_CUE_LOW));
        assert_eq!(w.units[i].speech_id, A02_CUE_LOW);
        assert_eq!(w.units[i].a02_latch_reserve_cue(450), None);
        assert_eq!(w.units[i].a02_latch_reserve_cue(100), Some(A02_CUE_LOWEST));
        assert_eq!(w.units[i].a02_latch_reserve_cue(600), Some(A02_CUE_MID));
        assert_eq!(w.units[i].a02_latch_reserve_cue(750), Some(A02_CUE_HIGH));
        assert_eq!(w.units[i].a02_latch_reserve_cue(9_000), None);
    }

    #[test]
    fn variable_threshold_latch_matches_the_decompiled_branch() {
        // 0x005ac454
        let mut w = world();
        let i = unit(&mut w, uid::MARINE);
        w.units[i].speech_id = 0;
        assert_eq!(
            w.units[i].a02_latch_threshold_cue(0, 900),
            Some(A02_CUE_LOWEST)
        );
        assert_eq!(
            w.units[i].a02_latch_threshold_cue(600, 900),
            Some(A02_CUE_MID)
        );
        assert_eq!(
            w.units[i].a02_latch_threshold_cue(900, 900),
            Some(A02_CUE_HIGH)
        );
        assert_eq!(w.units[i].a02_latch_threshold_cue(950, 900), None);
    }

    #[test]
    fn dialog_sound_id_rounds_down_to_a_multiple_of_four() {
        // 0x005ac29f
        assert_eq!(a02_dialog_sound_id(0), 0x55);
        assert_eq!(a02_dialog_sound_id(1), 0x55);
        assert_eq!(a02_dialog_sound_id(3), 0x55);
        assert_eq!(a02_dialog_sound_id(4), 0x59);
        assert_eq!(a02_dialog_sound_id(5), 0x59);
        assert_eq!(a02_dialog_sound_id(8), 0x5d);
        assert_eq!(a02_dialog_sound_id(0x100), 0x55);
        // The add is a 32-bit add truncated to the cue byte.
        assert_eq!(a02_dialog_sound_id(u32::MAX), 0x51);
    }

    #[test]
    fn dialog_sound_route_uses_the_owner_slot() {
        // 0x005ac29f
        assert_eq!(a02_route_dialog_sound(4, 0), A02SoundRoute::Global(0x59));
        assert_eq!(
            a02_route_dialog_sound(4, 0x2a),
            A02SoundRoute::ToOwner(0x59, 0x2a)
        );
        assert_eq!(
            a02_route_dialog_sound(0xffff_fffc, 7),
            A02SoundRoute::ToOwner(0x51, 7)
        );
    }

    #[test]
    fn scope_cue_depends_on_overlay_bit_one() {
        // 0x005abdaf, cue = (-(FUN_005b5630() != 0) & 5) + 0x55.
        let mut w = world();
        let i = unit(&mut w, uid::MARINE);
        w.units[i].overlay_flags = 0;
        assert_eq!(w.units[i].a02_scope_cue(), 0x55);
        w.units[i].overlay_flags = 1; // bit 0 only
        assert_eq!(w.units[i].a02_scope_cue(), 0x55);
        w.units[i].overlay_flags = 2; // bit 1 = FUN_005b5630
        assert_eq!(w.units[i].a02_scope_cue(), 0x5a);
        w.units[i].overlay_flags = 0xff;
        assert_eq!(w.units[i].a02_scope_cue(), 0x5a);
    }

    #[test]
    fn scope_counter_wraps_and_gates_cue_eight() {
        // 0x005abdaf
        assert_eq!(a02_scope_counter_step(0, 1), (1, Some(8)));
        assert_eq!(a02_scope_counter_step(3, 5), (4, None));
        assert_eq!(a02_scope_counter_step(255, 0), (0, Some(8)));
        assert_eq!(a02_scope_counter_step(255, 255), (0, None));
    }

    #[test]
    fn extra_block_is_cleared_byte_for_byte() {
        // 0x005afdf0
        let mut block = [0xffu8; A02_EXTRA_BLOCK_LEN];
        assert_eq!(A02_EXTRA_BLOCK_LEN, 0x133 - 0x110 + 1);
        a02_clear_extra_block(&mut block);
        assert!(block.iter().all(|&b| b == 0));
    }

    #[test]
    fn resource_type_table_covers_the_four_rows() {
        // 0x005ac3dc
        assert_eq!(a02_resource_type_cue(0x6e), Some((false, 0x193)));
        assert_eq!(a02_resource_type_cue(0x95), Some((false, 0x191)));
        assert_eq!(a02_resource_type_cue(0x9d), Some((false, 0x18f)));
        for id in [0xb0u16, 0xb1, 0xb2] {
            assert_eq!(a02_resource_type_cue(id), Some((true, 0x18d)));
        }
        // 0xbc (the geyser) is not in the panel switch, nor is anything else.
        assert_eq!(a02_resource_type_cue(0xbc), None);
        assert_eq!(a02_resource_type_cue(uid::REFINERY), Some((false, 0x193)));
        assert_eq!(a02_resource_type_cue(uid::MARINE), None);
    }

    #[test]
    fn resource_panel_counts_down_in_eight_tick_steps() {
        // 0x005ac3dc: non-animated (gas structure) row.  The decompiled cue
        // ladder sits in the looping branch only, so a gas row never emits a
        // cue; it just counts down and shows frame 8.
        let step = a02_resource_panel_step(0x6e, 16, 0).unwrap();
        assert_eq!(step.timer, 8);
        assert_eq!(step.frame, 8);
        assert_eq!(step.cue, None);
        assert_eq!(step.cue_base, 0x193); // frame 8 -> no offset
        assert!(!step.overlay_mode);
        assert!(!step.guard);

        // Countdown below eight with a non-animated row resets to zero and
        // selects frame 2.
        let step = a02_resource_panel_step(0x95, 3, 0).unwrap();
        assert_eq!((step.timer, step.frame), (0, 2));
        assert_eq!(step.cue, None);
        assert_eq!(step.cue_base, 0x192); // frame 2 -> base + 1
    }

    #[test]
    fn resource_panel_animates_mineral_rows() {
        // 0x005ac3dc: mineral field, countdown still inside the first frame.
        let step = a02_resource_panel_step(0xb0, 5, 3).unwrap();
        assert_eq!((step.timer, step.frame), (5, 3));
        assert_eq!(step.cue, None);
        assert_eq!(step.cue_base, 0x18e); // 0x18d + 1
        assert!(step.overlay_mode);

        // Crossing the eight-tick boundary keeps the animation frame when the
        // counter lands exactly on zero.
        let step = a02_resource_panel_step(0xb1, 8, 6).unwrap();
        assert_eq!((step.timer, step.frame), (0, 6));
        assert_eq!(step.cue, None);

        // Any other crossing plays the ladder cue and uses frame 8.
        let step = a02_resource_panel_step(0xb2, 12, 6).unwrap();
        assert_eq!((step.timer, step.frame), (4, 8));
        assert_eq!(step.cue, Some(A02_CUE_LOWEST));
        assert_eq!(step.cue_base, 0x18d);
    }

    #[test]
    fn resource_panel_reports_the_guard_and_unknown_types() {
        // 0x005ac3dc: non-animated row whose remainder falls below eight.
        let step = a02_resource_panel_step(0x9d, 12, 0).unwrap();
        assert!(step.guard);
        assert_eq!((step.timer, step.frame), (4, 8));

        // Rows outside the switch jump to the epilogue.
        assert!(a02_resource_panel_step(uid::MARINE, 64, 0).is_none());
        assert!(a02_resource_panel_step(0xbc, 64, 0).is_none());
    }

    #[test]
    fn wire_flag_reads_the_latched_byte() {
        // 0x005ad890
        let mut window = [0u8; 0xd4];
        assert_eq!(a02_wire_flag_0d3(&window), 0);
        window[0xd3] = 1;
        assert_eq!(a02_wire_flag_0d3(&window), 1);
        window[0xd2] = 0x5a;
        assert_eq!(a02_wire_flag_0d3(&window), 1);
    }

    #[test]
    fn empty_hook_has_no_effect() {
        // 0x005abec9: a bare `ret`.
        let mut w = world();
        let i = unit(&mut w, uid::MARINE);
        let before = (
            w.units[i].id,
            w.units[i].hp,
            w.units[i].overlay_flags,
            w.units[i].speech_id,
        );
        a02_empty_hook();
        assert_eq!(
            before,
            (
                w.units[i].id,
                w.units[i].hp,
                w.units[i].overlay_flags,
                w.units[i].speech_id,
            )
        );
    }
}
