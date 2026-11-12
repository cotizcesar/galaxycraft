#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave12/agent01.rs
// ===========================================================================
// Wave 12 translation batch 01: per-player selection records and the
// client-side helpers in `0x00652000..=0x00653fff`.
//
// The ledger lists 35 `decompiled_uncited` rows in this window. Fourteen
// value-level routines are translated below; every address cited as an
// implementation names a function or record that exists in this file. The
// remaining rows are listed under [Skipped rows](#skipped-rows) with the
// reason each one is not modelled.
//
// This is client/UI code. The shipped executable keeps a per-player array
// of `0x34`-byte selection slots plus a linked list of selected units at
// slot `+0x30`. Nothing below touches [`crate::sim::World`] or
// [`crate::sim::SimUnit`]; every translation is a free function over
// values, slices or caller-supplied predicates.
//
// ## Recovered layout
//
// | offset | field |
// |--------|-------|
// | `+0x00` | unit type id (`u16`) |
// | `+0x02` | cached target id (`u16`) |
// | `+0x04` | owner byte |
// | `+0x05` | display state byte |
// | `+0x06` | animation byte |
// | `+0x08` | flags byte |
// | `+0x09` | substate byte |
// | `+0x0c` | sixteen extent words (`+0x0c..+0x2b`) |
// | `+0x30` | linked-list head index (`+0x00` next, `+0x0c` unit) |
//
// ## Skipped rows
//
// * `FUN_006520e0` - player-list walk whose accepted branch is a
//   non-returning runtime call; no value survives the routine.
// * `FUN_00652250` - reads an `extraout_CX` left by a call, so the comparison
//   chain is not recoverable.
// * `FUN_006523a9` - `unaff_EBP`/`unaff_EBX` frame slots; no routine
//   arguments.
// * `FUN_006523f0` - four opaque slot tables plus two placement helpers; the
//   result is entirely determined by out-of-window lookups.
// * `FUN_00652874` - three calls, the last non-returning; no value.
// * `FUN_006528a0` - scans two global tables and a per-player array through
//   opaque helpers; the only visible effect is an invalidating call.
// * `FUN_00652b60` - score aggregation over global player/unit lists and a
//   non-returning tail; the local array depends on unknown counts.
// * `FUN_00652d4d` - `unaff_EBP`/`unaff_ESI` frame, plus `DAT ^ constant`
//   seeds; no recovered arguments.
// * `FUN_00653190` - unit-list scan with three opaque predicates and a
//   non-returning inner path.
// * `FUN_006532ce` - one-register `in_ZF` predicate; no argument.
// * `FUN_00653435`, `FUN_006534c2`, `FUN_006534da` - `in_EAX` is a byte pointer
//   and the body is an inlined comparison fragment, not a routine.
// * `FUN_0065359c` - `in_CF || in_ZF` fragment.
// * `FUN_00653612` - empty one-byte body (`ret`).
// * `FUN_00653940`, `FUN_00653947` - inter-unit order refresh whose unit layout
//   is not recovered; `FUN_00653947` duplicates the body through `unaff_EBX`.
// * `FUN_00653ae9`, `FUN_00653bca` - frame slots and `unaff_` registers;
//   `FUN_00653bca` is an inlined fragment.
// * `FUN_00653d2f` - `!ZF` fragment.
// * `FUN_00653eee` - reads `unaff_EBP` slots and a stack value at `+0xc`;
//   no recovered arguments.

/// Number of player slots in the alliance matrix and selection array.
pub const W12A01_PLAYER_COUNT: usize = 8;

/// Byte stride of one selection slot.
pub const W12A01_SLOT_STRIDE: usize = 0x34;

/// Flags bit tested by the group-command rows.
pub const W12A01_GROUP_FLAG: u8 = 0x02;

/// Flags bit cleared on every display-state transition.
pub const W12A01_STATE_FLAG: u8 = 0x20;

/// Animation written when a group slot switches to display state `4`.
pub const W12A01_GROUP_ANIM: u8 = 0x28;

/// One `0x34`-byte client selection slot.
///
/// Only the fields touched by the translated rows are modelled. The
/// display-state setter also rewrites extent words and flags; functions that
/// only need the state byte say so in their own documentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12a01SelectionSlot {
    /// `+0x00`: unit type id.
    pub unit_id: u16,
    /// `+0x02`: cached target id.
    pub target_id: u16,
    /// `+0x04`: owner/player index.
    pub owner: u8,
    /// `+0x05`: display state byte.
    pub state: u8,
    /// `+0x06`: animation byte.
    pub anim: u8,
    /// `+0x08`: flags byte.
    pub flags: u8,
    /// `+0x09`: substate byte.
    pub substate: u8,
    /// `+0x0c..+0x2b`: sixteen extent words.
    pub extent: [i16; 16],
    /// `+0x30`: head of the linked unit list, as an index into a caller
    /// supplied slice.
    pub link: Option<usize>,
}

/// One node of the linked unit list hanging off [`W12a01SelectionSlot::link`].
///
/// The retail node stores a `next` pointer at `+0x00` and the unit pointer at
/// `+0x0c`. Those pointers are represented here as indices.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12a01LinkedUnit {
    /// `+0x00`: next node index.
    pub next: Option<usize>,
    /// `+0x00` unit type id.
    pub unit_id: u16,
    /// Cached first dimension (width when the slot is used for bounds).
    pub dim_a: i16,
    /// Cached second dimension (height when the slot is used for bounds).
    pub dim_b: i16,
    /// Placement coordinate compared with the slot's unit id.
    pub x: i16,
}

/// Order-target record read by the order-update rows.
///
/// The target pointer is stored at unit `+0x134`; its tag is at `+0x08`, the
/// owning unit at `+0x0c`, the payload pointer at `+0x10`, and the payload
/// itself begins with a unit id and has a state byte at `+0x05`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12a01TargetRecord {
    /// `+0x08`: type tag. Only tag `4` is inspected.
    pub tag: u8,
    /// `+0x0c`: owning-unit identity.
    pub owner_unit: i32,
    /// `+0x10 -> +0x00`: payload unit id.
    pub unit_id: u16,
    /// `+0x10 -> +0x05`: payload state byte.
    pub state: u8,
    /// `+0x0a`: cached word updated by `0x00653700` when command `0x9e`
    /// reaches the target.
    pub cached_word: u16,
}

/// Action selected by [`w12a01_update_order_target`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W12a01OrderDispatch {
    /// No order is issued.
    None,
    /// The primary order routine is called with the command id.
    Primary(u16),
    /// The alternate order routine is called with the command id and zero.
    Secondary(u16),
}

/// Result of [`w12a01_choose_available_target`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W12a01TargetChoice {
    /// Neither candidate resolved to an eligible unit.
    None,
    /// Only the first candidate is eligible.
    First(u32),
    /// Only the second candidate is eligible.
    Second(u32),
    /// Both candidates are eligible; retail enters its non-returning
    /// consistency check here.
    Conflict(u32, u32),
}

/// Command emitted by [`w12a01_issue_first_flagged`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W12a01SelectionCommand {
    /// Player whose selection emitted the command.
    pub player: u8,
    /// Target value passed through unchanged.
    pub target: u32,
    /// Command id (`0x7d`).
    pub command: u16,
    /// Command argument (`0x50`).
    pub argument: u16,
}

/// Request emitted by [`w12a01_consume_flagged_group`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W12a01SelectionMove {
    /// Slot whose group flag was consumed.
    pub slot: usize,
    /// Value passed to the selection-move routine.
    pub value: u32,
}

/// Base pointer used by the `0x14`-byte record-index arithmetic.
pub const W12A01_RECORD_POINTER_BASE: u32 = 0x00fc_dee8;

/// Stride used by the `0x14`-byte record-index arithmetic.
pub const W12A01_RECORD_POINTER_STRIDE: u32 = 0x14;

/// Command id checked by the order/priority rows.
pub const W12A01_COMMAND_0X7D: u16 = 0x7d;

/// Command id whose target cache is refreshed by `0x00653700`.
pub const W12A01_COMMAND_0X9E: u16 = 0x9e;

/// `0x00652060` (`FUN_00652060`, 116 bytes): decay one 16-bit selection
/// counter.
///
/// ```c
/// if (param_2 != 0) {
///     if (300 < value) value -= 300; else value = 0;
/// } else if (1000 < value) value -= 13;
/// else if (200 < value) value -= 6;
/// else if (2 < value) value -= 2;
/// else value = 0;
/// ```
///
/// `fade` is the non-zero `param_2` case.  The comparisons are strict, so
/// `300` clears in the fade path while `1000` still takes the `-6` branch in
/// the normal path.
pub fn w12a01_decay_counter(value: u16, fade: bool) -> u16 {
    if fade {
        if value > 300 { value - 300 } else { 0 }
    } else if value > 1000 {
        value - 0x0d
    } else if value > 200 {
        value - 6
    } else if value > 2 {
        value - 2
    } else {
        0
    }
}

/// `0x00652aa0` (`FUN_00652aa0`, 178 bytes): mirror one player's active
/// group slots onto mutually allied players.
///
/// ```c
/// for (other = 0; other < 8; other++) {
///     if (ally[player][other] && ally[other][player] && count > 1) {
///         for (slot = 1; slot < count; slot++) {
///             if (selection[player][slot].state != 0
///                 && selection[other][slot].state == 0) {
///                 set_state(selection[other][slot], 4);
///             }
///         }
///     }
/// }
/// ```
///
/// The retail state setter also rewrites animation and extent words; this
/// model records the state byte, which is the only field the mirroring test
/// reads.  Slot `0` is never mirrored, and the self pair is skipped because
/// its two state tests cannot both hold.
pub fn w12a01_mirror_ally_selection(
    alliances: &[[u8; W12A01_PLAYER_COUNT]; W12A01_PLAYER_COUNT],
    selections: &mut [Vec<W12a01SelectionSlot>],
    player: usize,
    count: usize,
) -> usize {
    if count <= 1 || player >= W12A01_PLAYER_COUNT || player >= selections.len() {
        return 0;
    }
    let own_states: Vec<u8> = selections[player].iter().map(|slot| slot.state).collect();
    let limit = count.min(own_states.len());
    let mut changed = 0;
    let players = W12A01_PLAYER_COUNT.min(selections.len());
    for other in 0..players {
        if other == player {
            continue;
        }
        if alliances[player][other] == 0 || alliances[other][player] == 0 {
            continue;
        }
        for slot in 1..limit {
            if own_states[slot] == 0 {
                continue;
            }
            if let Some(target) = selections[other].get_mut(slot) {
                if target.state == 0 {
                    target.state = 4;
                    changed += 1;
                }
            }
        }
    }
    changed
}

/// `0x00652e70` (`FUN_00652e70`, 113 bytes): clear every selected slot whose
/// position falls inside an inclusive rectangle.
///
/// The row queries the position of each slot from `1..count` and calls the
/// display-state setter with state `0` when
/// `left <= x <= right && top <= y <= bottom`.  Slot `0` is never tested.
/// `positions[slot]` is that query result; the rectangle is
/// `[left, top, right, bottom]`.  `count` is the client selection count, so
/// slots past `count` are left untouched.
///
/// Only the state byte is modelled, as in [`w12a01_mirror_ally_selection`].
pub fn w12a01_clear_selection_in_rectangle(
    entries: &mut [W12a01SelectionSlot],
    positions: &[(i32, i32)],
    count: usize,
    bounds: [i32; 4],
) -> usize {
    let [left, top, right, bottom] = bounds;
    let mut cleared = 0;
    let limit = count.min(entries.len()).min(positions.len());
    for slot in 1..limit {
        let (x, y) = positions[slot];
        if left <= x && x <= right && top <= y && y <= bottom {
            entries[slot].state = 0;
            cleared += 1;
        }
    }
    cleared
}

/// `0x00653070` (`FUN_00653070`, 284 bytes): refresh the group slots that
/// point at a unit type with at most one stale entry.
///
/// The loop starts at slot `1` and refreshes that slot from its own unit id.
/// It asks the caller's `matching_slots` for every selection index carrying
/// that unit type, counts those whose state is below `4`, and stops the count
/// as soon as a second one is seen:
///
/// ```c
/// if (count_of_low_state_slots == matching_count && unit_id != 0) {
///     if (slot.state == 0) {
///         base = building(owner) ? owner_extent(owner) : 0;
///         slot.extent[0] = selected(unit_id) ? 0 : base;
///         slot.extent[1] = selected(unit_id) ? base : 0;
///         slot.extent[2..] = 0;
///         slot.target_id = 0;
///     }
///     slot.flags &= 0xdf;
///     slot.state = 4;
///     slot.anim = 0x28;
/// }
/// ```
///
/// `slot_states` is the state byte of the same per-player selection array
/// that `matching_slots` indexes.  An out-of-range index is treated as state
/// `0` (stale), matching the read-through behavior of the original array.
/// `count` is the client selection count; only slots `1..count` are refreshed.
/// Returns the number of slots updated.
pub fn w12a01_refresh_group_entries(
    entries: &mut [W12a01SelectionSlot],
    count: usize,
    slot_states: &[u8],
    mut matching_slots: impl FnMut(u16) -> Vec<usize>,
    mut selected: impl FnMut(u16) -> bool,
    mut owner_extent: impl FnMut(u8) -> u16,
    mut is_building: impl FnMut(u8) -> bool,
) -> usize {
    let mut changed = 0;
    let limit = count.min(entries.len());
    for index in 1..limit {
        if entries[index].state >= 4 {
            continue;
        }
        let unit_id = entries[index].unit_id;
        let matches = matching_slots(unit_id);
        let mut stale = 0;
        let mut too_many = false;
        for slot in matches {
            let state = slot_states.get(slot).copied().unwrap_or(0);
            if state < 4 {
                stale += 1;
                if stale >= 2 {
                    too_many = true;
                    break;
                }
            }
        }
        if too_many || unit_id == 0 {
            continue;
        }
        let owner = entries[index].owner;
        let was_idle = entries[index].state == 0;
        if was_idle {
            let biased = if is_building(owner) {
                owner_extent(owner)
            } else {
                0
            };
            let is_selected = selected(unit_id);
            entries[index].target_id = 0;
            entries[index].extent[0] = if is_selected { 0 } else { biased as i16 };
            entries[index].extent[1] = if is_selected { biased as i16 } else { 0 };
            for extent in &mut entries[index].extent[2..] {
                *extent = 0;
            }
        }
        entries[index].flags &= !W12A01_STATE_FLAG;
        entries[index].state = 4;
        entries[index].anim = W12A01_GROUP_ANIM;
        changed += 1;
    }
    changed
}

/// `0x006535f0` (`FUN_006535f0`, 34 bytes): convert a record pointer to its
/// one-based index.
///
/// ```c
/// if (pointer == 0) return 0;
/// return (pointer - 0x00fcdee8U) / 0x14 + 1;
/// ```
///
/// The subtraction and division are unsigned; a pointer below the base wraps
/// exactly like the original.
pub fn w12a01_record_index_from_pointer(pointer: u32) -> u32 {
    if pointer == 0 {
        0
    } else {
        pointer
            .wrapping_sub(W12A01_RECORD_POINTER_BASE)
            .wrapping_div(W12A01_RECORD_POINTER_STRIDE)
            .wrapping_add(1)
    }
}

/// `0x00653700` (`FUN_00653700`, 141 bytes): issue the follow-up order for a
/// unit and refresh the target cache on command `0x9e`.
///
/// ```c
/// if (first_mode == 0) {
///     if (order_slot == 0) issue_primary(command);
/// } else {
///     if (second_mode != 0 && order_slot != 0) return;
///     opaque_side_effect();
///     if (alternate_slot == 0) issue_secondary(command, 0);
/// }
/// if (command == 0x9e && target != NULL && target->tag == 4
///     && target->owner == unit && !multiplayer) {
///     target->cached_word = unit_value;
/// }
/// ```
///
/// `first_mode` and `second_mode` are the two no-argument mode queries in the
/// row.  `opaque_side_effect` has no recovered value and is not modelled.
#[allow(clippy::too_many_arguments)]
pub fn w12a01_update_order_target(
    order_slot: i32,
    alternate_slot: i32,
    command: u16,
    first_mode: bool,
    second_mode: bool,
    multiplayer: bool,
    unit_identity: i32,
    unit_value: u16,
    target: Option<&mut W12a01TargetRecord>,
) -> W12a01OrderDispatch {
    let mut dispatch = W12a01OrderDispatch::None;
    if !first_mode {
        if order_slot == 0 {
            dispatch = W12a01OrderDispatch::Primary(command);
        }
    } else {
        if second_mode && order_slot != 0 {
            return W12a01OrderDispatch::None;
        }
        if alternate_slot == 0 {
            dispatch = W12a01OrderDispatch::Secondary(command);
        }
    }
    if command == W12A01_COMMAND_0X9E {
        if let Some(record) = target {
            if record.tag == 4 && record.owner_unit == unit_identity && !multiplayer {
                record.cached_word = unit_value;
            }
        }
    }
    dispatch
}

/// `0x00653790` (`FUN_00653790`, 41 bytes): two-step value handoff.
///
/// The first call stores the value in the first destination and advances the
/// state from `1` to `2`, reporting `0`.  Every later call stores into the
/// second destination and reports `1`.
pub fn w12a01_two_step_handoff(
    state: &mut i32,
    first: &mut u32,
    second: &mut u32,
    value: u32,
) -> u32 {
    if *state == 1 {
        *first = value;
        *state = 2;
        0
    } else {
        *second = value;
        1
    }
}

/// `0x00653800` (`FUN_00653800`, 310 bytes): recompute a selection slot's
/// extent box from its linked units.
///
/// The four extent words at `+0x10..+0x16` are cleared, then every linked
/// unit is folded in:
///
/// ```c
/// record[0x14] += unit.dim_b;
/// record[0x16] += unit.dim_a;
/// if (unit.x == record.unit_id) {
///     record[0x12] += unit.dim_a;
///     record[0x10] += unit.dim_b;
/// }
/// ```
///
/// Afterwards the two cached extents at `+0x18` and `+0x1a` seed the bounds
/// at `+0x0e` and `+0x0c`; a non-building unit gets an extra half extent, and
/// a non-zero `cap` clamps both words from above.  All adds and compares are
/// unsigned 16-bit, as in the original `short` stores.
///
/// `include` corresponds to the linked-unit gate that returns zero (the unit
/// is included when the caller says so).  The building branch also registers
/// a screen-space overlay; that side effect is not modelled.  A cyclic list is
/// cut after one full pass rather than looping forever.
pub fn w12a01_recompute_selection_extents(
    record: &mut W12a01SelectionSlot,
    units: &[W12a01LinkedUnit],
    building: bool,
    mut include: impl FnMut(&W12a01LinkedUnit) -> bool,
    cap: u16,
) {
    record.extent[2] = 0;
    record.extent[3] = 0;
    record.extent[4] = 0;
    record.extent[5] = 0;
    let mut index = record.link;
    let mut visited = 0;
    while let Some(current) = index {
        if current >= units.len() || visited >= units.len() {
            break;
        }
        visited += 1;
        let unit = &units[current];
        if include(unit) {
            record.extent[4] = record.extent[4].wrapping_add(unit.dim_b);
            record.extent[5] = record.extent[5].wrapping_add(unit.dim_a);
            if unit.x == record.unit_id as i16 {
                record.extent[3] = record.extent[3].wrapping_add(unit.dim_a);
                record.extent[2] = record.extent[2].wrapping_add(unit.dim_b);
            }
        }
        index = unit.next;
    }
    let scale = |value: i16| -> u16 {
        let base = value as u16;
        if building {
            base
        } else {
            base.wrapping_add(base >> 1)
        }
    };
    let height = scale(record.extent[6]);
    if (record.extent[1] as u16) < height {
        record.extent[1] = height as i16;
    }
    let width = scale(record.extent[7]);
    if (record.extent[0] as u16) < width {
        record.extent[0] = width as i16;
    }
    if cap != 0 {
        if cap < record.extent[1] as u16 {
            record.extent[1] = cap as i16;
        }
        if cap < record.extent[0] as u16 {
            record.extent[0] = cap as i16;
        }
    }
}

/// `0x00653d00` (`FUN_00653d00`, 55 bytes): whether the unit's current order
/// target still accepts it.
///
/// ```c
/// if (target->tag != 1 && target->tag == 4) {
///     if (unit_value == target->payload_unit_id) {
///         return target->payload_state != 7;
///     }
/// }
/// return true;
/// ```
///
/// A missing target, tag `1`, and every tag other than `4` all accept.
pub fn w12a01_order_target_accepts(target: Option<&W12a01TargetRecord>, unit_value: u16) -> bool {
    match target {
        Some(record) if record.tag == 4 => record.unit_id != unit_value || record.state != 7,
        _ => true,
    }
}

/// `0x00653d40` (`FUN_00653d40`, 125 bytes): pick the one eligible target
/// out of two candidates.
///
/// The two lookups resolve to optional ids; each id is then checked with the
/// caller's eligibility predicate.  Retail enters its non-returning error
/// path when both ids are eligible, represented here by
/// [`W12a01TargetChoice::Conflict`].
pub fn w12a01_choose_available_target(
    candidate_a: Option<u32>,
    candidate_b: Option<u32>,
    mut eligible: impl FnMut(u32) -> bool,
) -> W12a01TargetChoice {
    let first = candidate_a.filter(|id| eligible(*id));
    let second = candidate_b.filter(|id| eligible(*id));
    match (first, second) {
        (Some(a), Some(b)) => W12a01TargetChoice::Conflict(a, b),
        (Some(a), None) => W12a01TargetChoice::First(a),
        (None, Some(b)) => W12a01TargetChoice::Second(b),
        (None, None) => W12a01TargetChoice::None,
    }
}

/// `0x00653dbd` (`FUN_00653dbd`, 57 bytes): command/selection guard.
///
/// The normal return is reached only when the `0x7d` special case does not
/// fire, the first lookup is zero, and the second lookup is non-zero; every
/// other path enters the non-returning error routine.  Returns `true` for the
/// normal path and `false` for the fatal one.
pub fn w12a01_command_guard(
    command: u16,
    special_blocker: bool,
    first_lookup: bool,
    second_lookup: bool,
) -> bool {
    if command == W12A01_COMMAND_0X7D && special_blocker {
        return false;
    }
    if !first_lookup && second_lookup {
        return true;
    }
    false
}

/// `0x00653def` (`FUN_00653def`, 127 bytes): choose between two order
/// targets, preferring the first when it is no farther away.
///
/// The gate is
/// `(command == 0x7d && lookup_a) || lookup_b || lookup_c_zero`, followed by
/// `(first_ok || second_lacks) && first_unblocked && second_unblocked`.  When
/// the gate fails the second id is returned; otherwise the first id wins on a
/// distance tie.  The two distance values are the caller's distance queries.
#[allow(clippy::too_many_arguments)]
pub fn w12a01_choose_priority_target(
    command: u16,
    lookup_a: bool,
    lookup_b: bool,
    lookup_c_zero: bool,
    first_ok: bool,
    second_lacks: bool,
    first_unblocked: bool,
    second_unblocked: bool,
    first_id: u32,
    second_id: u32,
    first_distance: u32,
    second_distance: u32,
) -> u32 {
    let gate = (command == W12A01_COMMAND_0X7D && lookup_a) || lookup_b || lookup_c_zero;
    let pass = (first_ok || second_lacks) && first_unblocked && second_unblocked;
    if !(gate && pass) {
        return second_id;
    }
    if first_distance <= second_distance {
        first_id
    } else {
        second_id
    }
}

/// `0x00653e80` (`FUN_00653e80`, 110 bytes): emit the group command for the
/// first flagged selection slot.
///
/// ```c
/// for (slot = 1; slot < count; slot++) {
///     if (selection[player][slot].flags & 2) {
///         issue(player, target, 0, 0x7d, 0x50);
///         break;
///     }
/// }
/// ```
///
/// The tail then walks the player's unit list through opaque predicates and
/// ends in a non-returning routine; it cannot change the emitted command, so
/// it is not modelled.
pub fn w12a01_issue_first_flagged(
    player: u8,
    target: u32,
    entries: &[W12a01SelectionSlot],
    count: usize,
) -> Option<W12a01SelectionCommand> {
    let limit = count.min(entries.len());
    for slot in 1..limit {
        if entries[slot].flags & W12A01_GROUP_FLAG != 0 {
            return Some(W12a01SelectionCommand {
                player,
                target,
                command: W12A01_COMMAND_0X7D,
                argument: 0x50,
            });
        }
    }
    None
}

/// `0x00653ff0` (`FUN_00653ff0`, 89 bytes): consume the first flagged group
/// slot and hand it to the selection-move routine.
///
/// The scan runs from slot `1`; the found slot's flags bit `0x02` is cleared
/// and the move routine receives `(slot, value)`.  Returns the move request,
/// or `None` when no slot is flagged.
pub fn w12a01_consume_flagged_group(
    entries: &mut [W12a01SelectionSlot],
    count: usize,
    value: u32,
) -> Option<W12a01SelectionMove> {
    let limit = count.min(entries.len());
    for slot in 1..limit {
        if entries[slot].flags & W12A01_GROUP_FLAG != 0 {
            entries[slot].flags &= !W12A01_GROUP_FLAG;
            return Some(W12a01SelectionMove { slot, value });
        }
    }
    None
}

#[cfg(test)]
mod tests_wave12_agent01 {
    use super::*;

    fn slot(unit_id: u16, state: u8) -> W12a01SelectionSlot {
        W12a01SelectionSlot {
            unit_id,
            state,
            ..W12a01SelectionSlot::default()
        }
    }

    // -- 0x00652060 --------------------------------------------------------

    #[test]
    fn counter_decay_uses_strict_thresholds() {
        assert_eq!(w12a01_decay_counter(1001, true), 701);
        assert_eq!(w12a01_decay_counter(301, true), 1);
        assert_eq!(w12a01_decay_counter(300, true), 0);
        assert_eq!(w12a01_decay_counter(1001, false), 988);
        assert_eq!(w12a01_decay_counter(1000, false), 994);
        assert_eq!(w12a01_decay_counter(201, false), 195);
        assert_eq!(w12a01_decay_counter(200, false), 198);
        assert_eq!(w12a01_decay_counter(3, false), 1);
        assert_eq!(w12a01_decay_counter(2, false), 0);
    }

    // -- 0x00652aa0 --------------------------------------------------------

    #[test]
    fn mutual_allies_mirror_non_idle_slots() {
        let mut alliances = [[0u8; W12A01_PLAYER_COUNT]; W12A01_PLAYER_COUNT];
        alliances[0][2] = 1;
        alliances[2][0] = 1;
        let mut selections: Vec<Vec<W12a01SelectionSlot>> = (0..W12A01_PLAYER_COUNT)
            .map(|_| vec![W12a01SelectionSlot::default(); 3])
            .collect();
        selections[0][1].state = 5;
        selections[0][2].state = 6;
        selections[2][1].state = 0;
        selections[2][2].state = 9;

        let changed = w12a01_mirror_ally_selection(&alliances, &mut selections, 0, 3);

        assert_eq!(changed, 1);
        assert_eq!(selections[2][1].state, 4);
        assert_eq!(selections[2][2].state, 9);
        // One-way alliance entries never copy.
        assert_eq!(selections[1][1].state, 0);
    }

    #[test]
    fn ally_mirror_needs_at_least_two_slots() {
        let mut alliances = [[1u8; W12A01_PLAYER_COUNT]; W12A01_PLAYER_COUNT];
        alliances[0][0] = 0;
        let mut selections: Vec<Vec<W12a01SelectionSlot>> =
            (0..W12A01_PLAYER_COUNT).map(|_| vec![slot(1, 5)]).collect();
        assert_eq!(
            w12a01_mirror_ally_selection(&alliances, &mut selections, 0, 1),
            0
        );
        assert_eq!(selections[3][0].state, 5);
    }

    // -- 0x00652e70 --------------------------------------------------------

    #[test]
    fn rectangle_clears_inclusive_entries_after_slot_zero() {
        let mut entries = vec![slot(1, 1), slot(2, 2), slot(3, 3), slot(4, 4)];
        let positions = [(0, 0), (5, 5), (20, 20), (5, 6)];
        let cleared =
            w12a01_clear_selection_in_rectangle(&mut entries, &positions, 4, [0, 0, 10, 10]);
        assert_eq!(cleared, 2);
        assert_eq!(entries[0].state, 1);
        assert_eq!(entries[1].state, 0);
        assert_eq!(entries[2].state, 3);
        assert_eq!(entries[3].state, 0);
    }

    #[test]
    fn rectangle_stops_at_the_client_selection_count() {
        let mut entries = vec![slot(1, 1), slot(2, 1), slot(3, 1)];
        let positions = [(0, 0), (5, 5), (5, 5)];
        let cleared =
            w12a01_clear_selection_in_rectangle(&mut entries, &positions, 2, [0, 0, 10, 10]);
        assert_eq!(cleared, 1);
        assert_eq!(entries[1].state, 0);
        assert_eq!(entries[2].state, 1);
    }

    // -- 0x00653070 --------------------------------------------------------

    #[test]
    fn group_refresh_skips_duplicates_and_initializes_idle_slots() {
        let mut entries = vec![W12a01SelectionSlot::default(); 5];
        entries[1] = W12a01SelectionSlot {
            unit_id: 10,
            owner: 1,
            state: 0,
            target_id: 99,
            flags: 0x3f,
            extent: [5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
            ..W12a01SelectionSlot::default()
        };
        entries[2] = W12a01SelectionSlot {
            unit_id: 11,
            owner: 1,
            state: 3,
            target_id: 77,
            flags: 0xff,
            ..W12a01SelectionSlot::default()
        };
        entries[3] = W12a01SelectionSlot {
            unit_id: 12,
            owner: 2,
            state: 0,
            target_id: 88,
            flags: 0xff,
            extent: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            ..W12a01SelectionSlot::default()
        };
        entries[4] = W12a01SelectionSlot {
            unit_id: 13,
            owner: 2,
            state: 4,
            flags: 0xff,
            ..W12a01SelectionSlot::default()
        };
        let states = [0u8, 1, 2, 0];

        let changed = w12a01_refresh_group_entries(
            &mut entries,
            5,
            &states,
            |unit| match unit {
                10 => vec![0, 1],
                11 => vec![2],
                12 => vec![3],
                _ => Vec::new(),
            },
            |unit| unit == 11,
            |owner| if owner == 2 { 400 } else { 100 },
            |owner| owner == 2,
        );

        assert_eq!(changed, 2);
        // Unit 10 has two stale matching slots, so the entry is left alone.
        assert_eq!(entries[1].state, 0);
        assert_eq!(entries[1].target_id, 99);
        assert_eq!(entries[1].extent[0], 5);
        // Unit 11 was not idle: only flags/state/anim change.
        assert_eq!(entries[2].state, 4);
        assert_eq!(entries[2].anim, W12A01_GROUP_ANIM);
        assert_eq!(entries[2].flags, 0xdf);
        assert_eq!(entries[2].target_id, 77);
        // Unit 12 was idle and is a building: extents are rebuilt.
        assert_eq!(entries[3].state, 4);
        assert_eq!(entries[3].target_id, 0);
        assert_eq!(entries[3].extent[0], 400);
        assert_eq!(entries[3].extent[1], 0);
        assert!(entries[3].extent[2..].iter().all(|&word| word == 0));
        // Unit 13 is already past state 4.
        assert_eq!(entries[4].state, 4);
        assert_eq!(entries[4].flags, 0xff);
    }

    #[test]
    fn group_refresh_marks_selected_non_buildings_with_the_other_extent() {
        let mut entries = vec![W12a01SelectionSlot::default(); 2];
        entries[1] = W12a01SelectionSlot {
            unit_id: 7,
            owner: 3,
            state: 0,
            ..W12a01SelectionSlot::default()
        };
        let changed = w12a01_refresh_group_entries(
            &mut entries,
            2,
            &[0],
            |_| vec![0],
            |unit| unit == 7,
            |_| 500,
            |_| true,
        );
        assert_eq!(changed, 1);
        assert_eq!(entries[1].extent[0], 0);
        assert_eq!(entries[1].extent[1], 500);
    }

    // -- 0x006535f0 --------------------------------------------------------

    #[test]
    fn record_pointer_maps_to_one_based_index() {
        assert_eq!(w12a01_record_index_from_pointer(0), 0);
        assert_eq!(w12a01_record_index_from_pointer(0x00fc_dee8), 1);
        assert_eq!(w12a01_record_index_from_pointer(0x00fc_defc), 2);
        assert_eq!(w12a01_record_index_from_pointer(0x00fc_df10), 3);
    }

    // -- 0x00653700 --------------------------------------------------------

    #[test]
    fn order_update_issues_primary_and_refreshes_target() {
        let mut target = W12a01TargetRecord {
            tag: 4,
            owner_unit: 77,
            unit_id: 5,
            state: 3,
            cached_word: 0,
        };
        let dispatch = w12a01_update_order_target(
            0,
            0,
            0x9e,
            false,
            false,
            false,
            77,
            1234,
            Some(&mut target),
        );
        assert_eq!(dispatch, W12a01OrderDispatch::Primary(0x9e));
        assert_eq!(target.cached_word, 1234);
    }

    #[test]
    fn order_update_early_return_blocks_secondary_and_cache() {
        let mut target = W12a01TargetRecord {
            tag: 4,
            owner_unit: 77,
            state: 3,
            cached_word: 9,
            ..W12a01TargetRecord::default()
        };
        let dispatch =
            w12a01_update_order_target(4, 0, 0x9e, true, true, false, 77, 1234, Some(&mut target));
        assert_eq!(dispatch, W12a01OrderDispatch::None);
        assert_eq!(target.cached_word, 9);

        // A non-zero order slot suppresses the order call but still reaches
        // the command-0x9e target-cache refresh.
        let dispatch = w12a01_update_order_target(
            4,
            0,
            0x9e,
            false,
            false,
            false,
            77,
            1234,
            Some(&mut target),
        );
        assert_eq!(dispatch, W12a01OrderDispatch::None);
        assert_eq!(target.cached_word, 1234);
    }

    #[test]
    fn order_update_secondary_path_and_multiplayer_gate() {
        let mut target = W12a01TargetRecord {
            tag: 4,
            owner_unit: 77,
            state: 3,
            cached_word: 1,
            ..W12a01TargetRecord::default()
        };
        let dispatch =
            w12a01_update_order_target(3, 0, 0x50, true, false, false, 77, 42, Some(&mut target));
        assert_eq!(dispatch, W12a01OrderDispatch::Secondary(0x50));
        assert_eq!(target.cached_word, 1);

        let dispatch =
            w12a01_update_order_target(3, 0, 0x9e, true, false, true, 77, 42, Some(&mut target));
        assert_eq!(dispatch, W12a01OrderDispatch::Secondary(0x9e));
        assert_eq!(target.cached_word, 1);
    }

    // -- 0x00653790 --------------------------------------------------------

    #[test]
    fn handoff_switches_destination_after_the_first_value() {
        let mut state = 1;
        let mut first = 0;
        let mut second = 0;
        assert_eq!(
            w12a01_two_step_handoff(&mut state, &mut first, &mut second, 10),
            0
        );
        assert_eq!(state, 2);
        assert_eq!(first, 10);
        assert_eq!(second, 0);
        assert_eq!(
            w12a01_two_step_handoff(&mut state, &mut first, &mut second, 20),
            1
        );
        assert_eq!(first, 10);
        assert_eq!(second, 20);
    }

    // -- 0x00653800 --------------------------------------------------------

    #[test]
    fn extents_fold_visible_units_and_clamp_to_cap() {
        let mut record = W12a01SelectionSlot {
            unit_id: 5,
            link: Some(0),
            extent: [10, 10, 99, 99, 99, 99, 8, 6, 0, 0, 0, 0, 0, 0, 0, 0],
            ..W12a01SelectionSlot::default()
        };
        let units = [
            W12a01LinkedUnit {
                next: Some(1),
                unit_id: 5,
                dim_a: 4,
                dim_b: 2,
                x: 5,
            },
            W12a01LinkedUnit {
                next: Some(2),
                unit_id: 6,
                dim_a: 10,
                dim_b: 20,
                x: 9,
            },
            W12a01LinkedUnit {
                next: None,
                unit_id: 5,
                dim_a: 6,
                dim_b: 3,
                x: 5,
            },
        ];

        w12a01_recompute_selection_extents(
            &mut record,
            &units,
            false,
            |unit| unit.unit_id != 6,
            11,
        );

        assert_eq!(record.extent[0], 10);
        assert_eq!(record.extent[1], 11);
        assert_eq!(record.extent[2], 5);
        assert_eq!(record.extent[3], 10);
        assert_eq!(record.extent[4], 5);
        assert_eq!(record.extent[5], 10);
        assert_eq!(record.extent[6], 8);
        assert_eq!(record.extent[7], 6);
    }

    #[test]
    fn extents_skip_the_half_step_for_buildings() {
        let mut record = W12a01SelectionSlot {
            unit_id: 5,
            link: Some(0),
            extent: [10, 10, 0, 0, 0, 0, 8, 6, 0, 0, 0, 0, 0, 0, 0, 0],
            ..W12a01SelectionSlot::default()
        };
        let units = [W12a01LinkedUnit::default()];
        w12a01_recompute_selection_extents(&mut record, &units, true, |_| true, 0);
        assert_eq!(record.extent[0], 10);
        assert_eq!(record.extent[1], 10);
    }

    // -- 0x00653d00 --------------------------------------------------------

    #[test]
    fn target_acceptance_only_rejects_matching_state_seven() {
        assert!(w12a01_order_target_accepts(None, 5));
        let tag_one = W12a01TargetRecord {
            tag: 1,
            unit_id: 5,
            state: 7,
            ..W12a01TargetRecord::default()
        };
        assert!(w12a01_order_target_accepts(Some(&tag_one), 5));
        let matching = W12a01TargetRecord {
            tag: 4,
            unit_id: 5,
            state: 7,
            ..W12a01TargetRecord::default()
        };
        assert!(!w12a01_order_target_accepts(Some(&matching), 5));
        assert!(w12a01_order_target_accepts(Some(&matching), 6));
        let other_state = W12a01TargetRecord {
            tag: 4,
            unit_id: 5,
            state: 8,
            ..W12a01TargetRecord::default()
        };
        assert!(w12a01_order_target_accepts(Some(&other_state), 5));
    }

    // -- 0x00653d40 --------------------------------------------------------

    #[test]
    fn target_choice_prefers_the_only_eligible_candidate() {
        assert_eq!(
            w12a01_choose_available_target(Some(1), Some(2), |id| id == 1),
            W12a01TargetChoice::First(1)
        );
        assert_eq!(
            w12a01_choose_available_target(Some(1), Some(2), |id| id == 2),
            W12a01TargetChoice::Second(2)
        );
        assert_eq!(
            w12a01_choose_available_target(None, Some(2), |id| id == 2),
            W12a01TargetChoice::Second(2)
        );
        assert_eq!(
            w12a01_choose_available_target(None, None, |_| true),
            W12a01TargetChoice::None
        );
        assert_eq!(
            w12a01_choose_available_target(Some(1), Some(2), |_| true),
            W12a01TargetChoice::Conflict(1, 2)
        );
    }

    // -- 0x00653dbd --------------------------------------------------------

    #[test]
    fn command_guard_needs_the_second_lookup() {
        assert!(w12a01_command_guard(0x7d, false, false, true));
        assert!(w12a01_command_guard(0x50, false, false, true));
        assert!(!w12a01_command_guard(0x7d, true, false, true));
        assert!(!w12a01_command_guard(0x50, false, true, true));
        assert!(!w12a01_command_guard(0x50, false, false, false));
    }

    // -- 0x00653def --------------------------------------------------------

    #[test]
    fn priority_target_returns_second_when_the_gate_fails() {
        assert_eq!(
            w12a01_choose_priority_target(
                0x50, false, false, false, true, false, true, true, 11, 22, 5, 10
            ),
            22
        );
        assert_eq!(
            w12a01_choose_priority_target(
                0x50, false, true, false, true, false, true, true, 11, 22, 5, 10
            ),
            11
        );
    }

    #[test]
    fn priority_target_uses_distance_and_ties() {
        let choose = |first_distance, second_distance| {
            w12a01_choose_priority_target(
                0x7d,
                true,
                false,
                false,
                false,
                true,
                true,
                true,
                11,
                22,
                first_distance,
                second_distance,
            )
        };
        assert_eq!(choose(5, 10), 11);
        assert_eq!(choose(10, 5), 22);
        assert_eq!(choose(7, 7), 11);
        assert_eq!(
            w12a01_choose_priority_target(
                0x7d, true, false, false, true, false, false, true, 11, 22, 5, 10
            ),
            22
        );
    }

    // -- 0x00653e80 --------------------------------------------------------

    #[test]
    fn issue_command_picks_first_flagged_slot() {
        let mut entries = vec![W12a01SelectionSlot::default(); 4];
        entries[1].flags = 0x02;
        entries[2].flags = 0x02;
        let command = w12a01_issue_first_flagged(3, 0x1234, &entries, 4).unwrap();
        assert_eq!(
            command,
            W12a01SelectionCommand {
                player: 3,
                target: 0x1234,
                command: 0x7d,
                argument: 0x50,
            }
        );
        assert!(w12a01_issue_first_flagged(3, 0x1234, &entries, 1).is_none());
    }

    // -- 0x00653ff0 --------------------------------------------------------

    #[test]
    fn consume_flagged_group_clears_the_bit_and_moves_once() {
        let mut entries = vec![W12a01SelectionSlot::default(); 4];
        entries[2].flags = 0x02;
        entries[3].flags = 0x02;

        let first = w12a01_consume_flagged_group(&mut entries, 4, 9);
        assert_eq!(first, Some(W12a01SelectionMove { slot: 2, value: 9 }));
        assert_eq!(entries[2].flags, 0);

        let second = w12a01_consume_flagged_group(&mut entries, 4, 10);
        assert_eq!(second, Some(W12a01SelectionMove { slot: 3, value: 10 }));
        assert_eq!(entries[3].flags, 0);

        assert!(w12a01_consume_flagged_group(&mut entries, 4, 11).is_none());
    }

    #[test]
    fn consume_flagged_group_ignores_slot_zero() {
        let mut entries = vec![W12a01SelectionSlot::default(); 2];
        entries[0].flags = 0x02;
        assert!(w12a01_consume_flagged_group(&mut entries, 1, 1).is_none());
        assert_eq!(entries[0].flags, 0x02);
    }
}
