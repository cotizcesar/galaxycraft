#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent84.rs
// ===========================================================================
// Translation batch 84.  Only this file is edited by the batch.
//
// Batch 84 owns `0x0063a000..=0x0063bfff`.  The ledger lists 33 rows in that
// window: 32 `decompiled_uncited` and one `translated`.  Of the 32 open rows
// 24 are translated below; the 8 that are not are listed at the end with
// their reason.  Nothing here touches `sim`: every function is a free
// function over values supplied by the caller, so no simulation record gains
// a field.
//
// The window is the client-side anchor/selection controller that sits
// between the command card and the unit list.  The decompiler recovered the
// record offsets but not the record types, so the tables below name the
// offsets it printed.  All offsets are in bytes:
//
// | layout                       | words                                                                    |
// |------------------------------|--------------------------------------------------------------------------|
// | anchor record                | `+0x18` sprite/kind byte, `+0x1c` marker byte, `+0x1d` enabled byte, `+0x1e` visited byte, `+0x20` centre X, `+0x22` centre Y |
// | anchor table `DAT_00fc86ac`  | one pointer pair per player (`index = player * 2`), `+0x00` head        |
// | chain node                   | `+0x00` next, `+0x0c` children, `+0x14` tail/owner link                  |
// | focus object                 | `+0x134` anchor, `+0x09` follow state, `+0x0b` cooldown, `+0x0c` stamp, `+0x14` owner |
// | owner record                 | `+0x14` unit list, `+0x24` busy word, `+0x28` claimant, `+0x2c` unit      |
// | unit                         | `+0x4c` player byte, `+0x4d` state byte                                  |
// | anchor id table              | `0x00fbd124`, stride `0x1cc`                                             |
// | slot table                   | `0x00fac940`, stride `0x18`                                              |
//
// Recovered result conventions: the anchor rows return 1/0 (`bool`), the
// mode rows return 1 on the "handled" path and 0 on the "ignored" path, and
// every path that reaches `FUN_005b4cd0` is a non-returning abort.  Aborts
// are represented as [`A84ModalPlan::terminate`] / [`A84ChoiceOutcome::Invalid`]
// / [`A84FollowAction::Terminate`] flags instead of calling the unimplemented
// terminator.
//
// Address to function:
//
// * `0x0063a151` [`a84_anchor_from_rect`] (plus [`a84_all_anchors_live`])
// * `0x0063a2d0` [`a84_anchor_create_outcome`]
// * `0x0063a3d1` [`a84_modal_key_handler`] (core in [`a84_modal_plan`])
// * `0x0063a413` [`a84_modal_modifier_handler`] (prelude in [`a84_modifier_prelude`])
// * `0x0063a625` [`a84_confirm_notification`]
// * `0x0063a950` [`a84_anchor_position`]
// * `0x0063a980` [`a84_chain_len`]
// * `0x0063ac10` [`a84_anchor_id_from_pointer`]
// * `0x0063ac40` [`a84_anchor_current`]
// * `0x0063ac60` [`a84_effect_bounds_gate`]
// * `0x0063ac90` [`a84_follow_plan`] (candidates in [`a84_best_follow_candidate`])
// * `0x0063af10` [`a84_shrink_rect_by_selection`]
// * `0x0063b000` [`a84_anchor_pointer_from_id`]
// * `0x0063b100` [`a84_slot_bounds_gate`]
// * `0x0063b197` [`a84_noop`]
// * `0x0063b1a0` [`a84_detach_anchor`]
// * `0x0063b1f0` [`a84_slot_id_from_pointer`]
// * `0x0063b420` [`a84_anchor_marker`]
// * `0x0063b430` [`a84_anchor_mark_visited`]
// * `0x0063b450` [`a84_chain_child_probe`]
// * `0x0063b620` [`a84_choice_outcome`]
// * `0x0063b858` [`a84_finish_dispatch`]
// * `0x0063ba0e` [`a84_mode_combine`]
// * `0x0063bbcd` [`a84_escape_dispatch`]
//
// Skipped rows:
//
// * `FUN_0063a265` - the decompiler could not track the spacebase; the body is
//   overlapping byte arithmetic (`*in_EDX += ...`) that ends in `swi(3)` and
//   has no recoverable call, table or string semantics.
// * `FUN_0063a3c0` - 17-byte non-returning trampoline to `FUN_005b4cd0`; the
//   only behaviour is the unused terminator.
// * `FUN_0063a703` - 22-byte tail call to `FUN_00638640` returning 1; pure
//   delegation with no arithmetic of its own.
// * `FUN_0063a9a0` - walks a caller-frame list; every switch arm collapses into
//   the same non-returning `FUN_005b4cd0`, so no case body survives.
// * `FUN_0063aa37` - jump table over a caller-frame local (`EBP-4`) with
//   unreachable arms; only the counter increment is recoverable.
// * `FUN_0063b825` - frame-based error check whose both paths return or abort;
//   no value is produced.
// * `FUN_0063b8f0` - Ghidra truncated this 173-byte body at bad instruction
//   data; after `*param_1 = 100` the control flow is unrecoverable.
// * `FUN_0063bb34` - the notification id is `DAT_00f8f0f8[key]`, a runtime
//   pointer table that is not in the row; the remaining chain is only query
//   plumbing and cannot be checked against a recovered value.

// ---------------------------------------------------------------------------
// Shared records
// ---------------------------------------------------------------------------

/// The `0x24`-byte anchor record allocated by `FUN_006383d0`.
///
/// Only the fields the window reads or writes are named.  `0x0063a151` writes
/// `+0x1c`, `+0x1d`, `+0x20` and `+0x22`; `0x0063a2d0` writes the first two;
/// `0x0063b420` reads `+0x1c` and `0x0063b430` writes `+0x1e`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A84Anchor {
    /// `+0x18`: the sprite/handle byte passed to `FUN_0076af60`/`FUN_00657260`.
    pub kind: u8,
    /// `+0x1c`: marker, the low byte of the caller's type word.
    pub marker: u8,
    /// `+0x1d`: byte copy of `marker != 0`.
    pub enabled: bool,
    /// `+0x1e`: "visited" byte, set to 1 by `0x0063b430`.
    pub visited: u8,
    /// `+0x20`: centre X, stored as a signed 16-bit value.
    pub x: i16,
    /// `+0x22`: centre Y, stored as a signed 16-bit value.
    pub y: i16,
}

/// One node of the intrusive singly linked lists under the anchor table.
///
/// `next` is the node's `+0x00` word (`None` is the null terminator); the
/// payload is whatever the row reads through `+0x0c`, `+0x14` or `+0x10`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A84Link<T> {
    /// `+0x00`: the next node, `None` for null.
    pub next: Option<u32>,
    /// The payload read through the node by the row.
    pub value: T,
}

/// Result of `FUN_005b7d00` / `FUN_005b7cd0` in the mode handlers.
///
/// The two shipped functions queue different notification classes but share
/// the `(code)` argument shape, so the variant records which entry point the
/// row used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A84Notify {
    /// `FUN_005b7d00(code)`.
    Command(u32),
    /// `FUN_005b7cd0(code)`.
    Cancel(u32),
}

// ---------------------------------------------------------------------------
// 0x0063a151 - anchor record from the selection rectangle
// ---------------------------------------------------------------------------

/// `0x0063a151`: fill the anchor record from the selection rectangle.
///
/// ```c
/// iVar3 = FUN_006383d0(id);
/// if (iVar3 != 0) {
///   *(char *)(iVar3 + 0x1c) = (char)id;
///   *(bool *)(iVar3 + 0x1d) = id != 0;
///   ...
///   piVar2 = rect;                       // [left, top, right, bottom]
///   *(short *)(iVar3 + 0x20) = (short)((piVar2[2] - *piVar2) / 2) + (short)*piVar2;
///   *(short *)(iVar3 + 0x22) = (short)((piVar2[3] - piVar2[1]) / 2) + (short)piVar2[1];
/// }
/// FUN_005a5770();
/// return iVar3;
/// ```
///
/// The centre is the half-span added to the low corner.  The C division
/// truncates toward zero before the addition, so an odd span of 5 gives
/// `left + 2`.  The sum is computed in `int` and truncated on the store to
/// the 16-bit field; both steps are reproduced here.  `kind` is the
/// pre-existing `+0x18` byte the allocator has already written and which this
/// row does not touch.
pub fn a84_anchor_from_rect(kind: u8, id: i32, rect: &[i32; 4]) -> A84Anchor {
    let centre_x = rect[2].wrapping_sub(rect[0]) / 2;
    let centre_y = rect[3].wrapping_sub(rect[1]) / 2;
    A84Anchor {
        kind,
        marker: id as u8,
        enabled: id != 0,
        visited: 0,
        x: centre_x.wrapping_add(rect[0]) as i16,
        y: centre_y.wrapping_add(rect[1]) as i16,
    }
}

/// The null-terminated validation loop at the top of `0x0063a151`.
///
/// The row walks a null-terminated array of object pointers and requires
/// `*(int *)(object + 0x134) != 0` on every entry; the first zero entry runs
/// `FUN_00637810(anchor)` and then the non-returning `FUN_005b4cd0`.  The
/// walk itself keeps no value, so its only observable result is "every entry
/// is live" (`true`) versus "the row aborts" (`false`).
pub fn a84_all_anchors_live(anchors: &[bool]) -> bool {
    anchors.iter().all(|&live| live)
}

// ---------------------------------------------------------------------------
// 0x0063a2d0 - anchor creation with the per-player fallback kind
// ---------------------------------------------------------------------------

/// The effect selector chosen by `0x0063a2d0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A84AnchorCreate {
    /// The record after `+0x1c`/`+0x1d` are overwritten.
    pub anchor: A84Anchor,
    /// `5` when `DAT_00fc86ac[player * 2]` was null at entry, else `2`.
    pub effect_kind: u32,
    /// The row called `FUN_00659120(player)` before allocating because the
    /// per-player head was null.
    pub initialise_head: bool,
}

/// `0x0063a2d0`: create an anchor and choose its draw kind.
///
/// ```c
/// if (func_0x005b0fb0(player) == 0) return 0;
/// iVar1 = (&DAT_00fc86ac)[player * 2];
/// if (iVar1 == 0) FUN_00659120(player);
/// iVar3 = FUN_006383d0(player, ...);
/// if (iVar3 == 0) return 0;
/// *(char *)(iVar3 + 0x1c) = (char)param_4;
/// *(bool *)(iVar3 + 0x1d) = param_4 != 0;
/// ...
/// uVar5 = (iVar1 == 0) ? 5 : 2;
/// FUN_0076af60(*(u8 *)(iVar3 + 0x18), x, y, uVar5);
/// FUN_00657260(*(u8 *)(iVar3 + 0x18), x, y);
/// return iVar3;
/// ```
///
/// `lookup_ok` is `func_0x005b0fb0(player) != 0` and `alloc_ok` is
/// `FUN_006383d0(...) != 0`.  The body loop is not modelled: it walks the
/// objects returned by `FUN_005a3180`/`FUN_005b4bb0` and, for each object
/// that `FUN_00658130` does not already own and whose `+0x134` is null,
/// detaches the new anchor from that object with `FUN_00637810`.  That loop
/// only rewrites owner links; it reads and writes none of the returned
/// anchor's `+0x18..+0x22` fields modelled below.
pub fn a84_anchor_create_outcome(
    mut anchor: A84Anchor,
    lookup_ok: bool,
    alloc_ok: bool,
    head_present: bool,
    id: i32,
) -> Option<A84AnchorCreate> {
    if !lookup_ok || !alloc_ok {
        return None;
    }
    anchor.marker = id as u8;
    anchor.enabled = id != 0;
    Some(A84AnchorCreate {
        anchor,
        effect_kind: if head_present { 2 } else { 5 },
        initialise_head: !head_present,
    })
}

// ---------------------------------------------------------------------------
// 0x0063a3d1 / 0x0063a413 - the modal key dispatcher
// ---------------------------------------------------------------------------

/// Keys accepted by mode 3, in the order the row tests them.
pub const A84_MODAL_KEYS: [u32; 5] = [0x84, 0x85, 0x92, 0x90, 0x89];

/// Keys that make `FUN_005b900` treat the failed selection as a recoverable
/// notification instead of a plain "ignored" result.
pub const A84_ERROR_CODES: [u32; 6] = [2, 0x17, 0x15, 8, 7, 0xd];

/// The pre-key action performed before the focus gate by `0x0063a3d1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum A84PreKeyAction {
    /// No pre-key action for this key.
    #[default]
    None,
    /// `0x2a` with `FUN_005bb740() != 0`: `FUN_00655e10` then `FUN_005d8f00`.
    ScreenPing,
    /// `0x6b`: `FUN_0063c1b0`.
    ToggleBuilding,
    /// `0x6c`: `FUN_00635d10(player, 0xe)` gates the branch, a zero result
    /// runs the non-returning `FUN_005b4cd0`.
    FoodCheck,
}

/// The modifier prelude of `0x0063a413`.
///
/// The row picks the required bit from `DAT_00f91fe8[key]` by the caller's
/// modifier word (`& 2` first, then `& 1`, otherwise `& 4`).  A missing bit
/// calls `FUN_005b5b910(0x19)` and continues; a matching bit sends the two
/// notifications only when all three following queries are non-zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A84ModifierPrelude {
    /// `FUN_005b7d00(0x7c)`.
    pub command: bool,
    /// `FUN_005b7cd0(0x26)`.
    pub cancel: bool,
    /// `FUN_005b5b910(0x19)`.
    pub reset: bool,
}

/// See [`A84ModifierPrelude`].  `gates` are, in order, the results of
/// `FUN_0065b2b0(0xe, player) != 0`, `FUN_00634bc0(player, 0xe) != 0` and
/// `FUN_0058e0e0(0xe, 8, 0) != 0`.
pub fn a84_modifier_prelude(modifiers: u32, key_flags: u8, gates: [bool; 3]) -> A84ModifierPrelude {
    let required = if modifiers & 2 != 0 {
        2
    } else if modifiers & 1 != 0 {
        1
    } else {
        4
    };
    if key_flags & required == 0 {
        A84ModifierPrelude {
            command: false,
            cancel: false,
            reset: true,
        }
    } else {
        let send = gates[0] && gates[1] && gates[2];
        A84ModifierPrelude {
            command: send,
            cancel: send,
            reset: false,
        }
    }
}

/// The caller-side query results consumed by the common mode switch.
///
/// The switch body never opens a record itself: it asks the helpers for a
/// mode (`FUN_00635c40`), a key (`FUN_00635c20`), a record id (`FUN_00635c00`)
/// and a handful of predicates.  Each field below is one of those results,
/// named after the helper that produced it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A84ModalGates {
    /// `FUN_00635c40(player)`: the switch selector.
    pub mode: u32,
    /// `FUN_00635c20(player)`: the decoded key/index word used by modes 3-8.
    pub key: u32,
    /// `(&DAT_00f91fe8)[key & 0xffff]`, loaded by mode 4.
    pub key_flags: u8,
    /// `FUN_00635c00(player)`.
    pub record_id: i32,
    /// `focus + 0x14`.
    pub object_id: i32,
    /// `*(int *)(focus + 0x134) != 0`.
    pub focus_live: bool,
    /// `*(char *)(focus + 8)`, required to be 3.
    pub focus_state: u8,
    /// `FUN_00637510(player) != 0`, the extra gate of mode 0.
    pub tracking: bool,
    /// `FUN_00634bc0(player, key) != 0`.
    pub enabled: bool,
    /// `FUN_00638ba0() != 0`.
    pub selectable: bool,
    /// `FUN_0065b2b0(key, player) == 1`.
    pub is_unit: bool,
    /// `FUN_0065b900(key)`.
    pub error_code: u32,
    /// `FUN_0058e040(key) != 0`.
    pub spawn_ok: bool,
    /// `FUN_00534c20` / `FUN_005348f0` returned 1.
    pub command_ok: bool,
    /// `func_0x00634ca0` / `func_0x00634c30` did not return 0.
    pub attach_ok: bool,
    /// `FUN_005da5d0` / `FUN_005da550` did not return 0.
    pub target_ok: bool,
}

/// The plan produced by the common mode switch.
///
/// Calls the row makes are recorded as fields instead of being executed.
/// `terminate` dominates every other field: the row reaches `FUN_005b4cd0`,
/// which does not return.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A84ModalPlan {
    /// Value returned to the caller (1 handled, 0 ignored).
    pub result: u32,
    /// The single notification pushed before finishing.
    pub notify: Option<A84Notify>,
    /// `FUN_00635240(player)` runs before the abort/return.
    pub cleanup: bool,
    /// `FUN_00635150(player)` runs after the notification.
    pub finish: bool,
    /// `FUN_005b4cd0` is reached (non-returning).
    pub terminate: bool,
    /// Tail call to `0x0063b620`, whose every returning path yields 0.
    pub decode_choice: bool,
    /// `FUN_00638640(player)` (mode 7).
    pub refresh: bool,
    /// The byte mode 4 loads from `DAT_00f91fe8` before aborting.
    pub key_flags: Option<u8>,
    /// The pre-key action of `0x0063a3d1`.
    pub pre: A84PreKeyAction,
    /// The modifier prelude of `0x0063a413`.
    pub modifier: A84ModifierPrelude,
}

fn a84_empty_plan() -> A84ModalPlan {
    A84ModalPlan {
        result: 1,
        ..A84ModalPlan::default()
    }
}

/// The common body shared by `0x0063a3d1` and `0x0063a413`.
///
/// ```c
/// if (*(int *)(focus + 0x134) == 0) return 1;
/// if (*(char *)(focus + 8) != 3) return 1;
/// switch (FUN_00635c40(player)) {
/// case 0:    if (!FUN_00637510(player)) return 1; return FUN_0063b620(player);
/// case 1:
/// case 2:    return FUN_0063b620(player);
/// case 3:    ... exact key set {0x84,0x85,0x92,0x90,0x89} ...
/// case 4:    ... compare record id, load DAT_00f91fe8, abort ...
/// case 5:    ... FUN_00534c20 == 1, 0x4c notification ...
/// case 6:    ... FUN_005348f0 == 1, 0x4b notification ...
/// case 7:    FUN_00638640(player); return 1;
/// case 8:    if (key == 0x54) abort; return 0;
/// default:   return 1;
/// }
/// ```
///
/// The mode-3 error arm tests the key against [`A84_ERROR_CODES`] only after
/// `FUN_00638ba0` or `FUN_0065b2b0` failed; an accepted code runs
/// `FUN_00635240` and then aborts, so the row still returns 0.  The mode-5/6
/// error arm is identical except that it returns 1.
pub fn a84_modal_plan(g: &A84ModalGates) -> A84ModalPlan {
    let mut plan = a84_empty_plan();
    if !g.focus_live || g.focus_state != 3 {
        return plan;
    }
    match g.mode {
        0 => {
            if g.tracking {
                plan.result = 0;
                plan.decode_choice = true;
            }
        }
        1 | 2 => {
            plan.result = 0;
            plan.decode_choice = true;
        }
        3 => {
            if !g.enabled {
                plan.result = 0;
                return plan;
            }
            if g.key == 0x2a {
                plan.terminate = true;
                return plan;
            }
            if !A84_MODAL_KEYS.contains(&g.key) {
                plan.result = 0;
                return plan;
            }
            if g.record_id != g.object_id {
                plan.result = 0;
                return plan;
            }
            if !(g.selectable && g.is_unit) {
                plan.result = 0;
                if A84_ERROR_CODES.contains(&g.error_code) {
                    plan.cleanup = true;
                    plan.terminate = true;
                }
                return plan;
            }
            if !g.spawn_ok {
                plan.result = 0;
                return plan;
            }
            plan.notify = Some(A84Notify::Command(0x2b));
            plan.finish = true;
        }
        4 => {
            if g.record_id != g.object_id {
                return plan;
            }
            plan.key_flags = Some(g.key_flags);
            plan.terminate = true;
        }
        5 | 6 => {
            let code = if g.mode == 5 { 0x4c } else { 0x4b };
            if !g.command_ok {
                if A84_ERROR_CODES.contains(&g.error_code) {
                    plan.cleanup = true;
                    plan.terminate = true;
                }
                return plan;
            }
            if !g.attach_ok || !g.target_ok {
                return plan;
            }
            plan.notify = Some(A84Notify::Command(code));
            plan.finish = true;
        }
        7 => {
            plan.refresh = true;
        }
        8 => {
            if g.key == 0x54 {
                plan.terminate = true;
            } else {
                plan.result = 0;
            }
        }
        _ => {}
    }
    plan
}

/// `0x0063a3d1`: the key-down entry point around [`a84_modal_plan`].
///
/// The row first handles three raw keys (`AX`):
///
/// ```c
/// if (in_AX == 0x2a) { if (FUN_005bb740()) { FUN_00655e10(); FUN_005d8f00(...); } }
/// else if (in_AX == 0x6b) FUN_0063c1b0();
/// else if (in_AX == 0x6c && FUN_00635d10(player, 0xe) == 0) FUN_005b4cd0();
/// ```
///
/// `ping_gate` is `FUN_005bb740() != 0` and `food_gate` is
/// `FUN_00635d10(player, 0xe) != 0`.  The food failure aborts before the mode
/// switch, so the returned plan carries only the pre-key action and
/// `terminate`.
pub fn a84_modal_key_handler(
    prelude_key: u16,
    ping_gate: bool,
    food_gate: bool,
    g: &A84ModalGates,
) -> A84ModalPlan {
    let mut pre = a84_empty_plan();
    match prelude_key {
        0x2a => {
            if ping_gate {
                pre.pre = A84PreKeyAction::ScreenPing;
            }
        }
        0x6b => pre.pre = A84PreKeyAction::ToggleBuilding,
        0x6c => {
            pre.pre = A84PreKeyAction::FoodCheck;
            if !food_gate {
                pre.terminate = true;
                return pre;
            }
        }
        _ => {}
    }
    let mut plan = a84_modal_plan(g);
    plan.pre = pre.pre;
    plan
}

/// `0x0063a413`: the key-up entry point around [`a84_modal_plan`].
///
/// The row has no pre-key actions; instead it gates the call on the modifier
/// bit selected from `DAT_00f91fe8[key & 0xffff]` and sends `0x7c`/`0x26`
/// when the three `player`/`0xe` queries all succeed.  `flags` is the byte
/// table at `DAT_00f91fe8` (the row indexes it with the raw `AX` value).
pub fn a84_modal_modifier_handler(
    prelude_key: u16,
    modifiers: u32,
    flags: &[u8],
    prelude_gates: [bool; 3],
    g: &A84ModalGates,
) -> A84ModalPlan {
    let key_flags = flags[(prelude_key & 0xffff) as usize];
    let mut plan = a84_modal_plan(g);
    plan.modifier = a84_modifier_prelude(modifiers, key_flags, prelude_gates);
    plan
}

// ---------------------------------------------------------------------------
// 0x0063a625 - confirmation notification
// ---------------------------------------------------------------------------

/// The notification emitted for the confirmation key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A84Confirm {
    /// The queued notification.
    pub call: A84Notify,
    /// `FUN_00635150()` always runs.
    pub finish: bool,
    /// The row always returns 1.
    pub result: u32,
}

/// `0x0063a625`: send the confirmation/cancel notification for a raw key.
///
/// ```c
/// if (in_AX == 0x23) FUN_005b7d00(0x2a);
/// else               FUN_005b7cd0(0x26);
/// FUN_00635150();
/// return 1;
/// ```
///
/// The two arms use different notification entry points, not just different
/// codes, so the distinction is kept in [`A84Notify`].
pub fn a84_confirm_notification(key: u16) -> A84Confirm {
    A84Confirm {
        call: if key == 0x23 {
            A84Notify::Command(0x2a)
        } else {
            A84Notify::Cancel(0x26)
        },
        finish: true,
        result: 1,
    }
}

// ---------------------------------------------------------------------------
// 0x0063a950 / 0x0063a980 - anchor table queries
// ---------------------------------------------------------------------------

/// `0x0063a950`: read the centre of the player's first anchor.
///
/// ```c
/// iVar1 = (&DAT_00fc86ac)[param_1 * 2];
/// if (iVar1 == 0) return 0;
/// *param_2 = (int)*(short *)(iVar1 + 0x20);
/// *param_3 = (int)*(short *)(iVar1 + 0x22);
/// return 1;
/// ```
///
/// The table is an array of pointer pairs, hence the `player * 2` dword
/// index.  Both fields are signed 16-bit and are widened to `int` by the
/// out-parameter stores.  The row performs no bounds check on `player`; the
/// slice index panics where the shipped load would fault.
pub fn a84_anchor_position(table: &[[Option<A84Anchor>; 2]], player: u32) -> Option<(i32, i32)> {
    let head = table[player as usize][0]?;
    Some((i32::from(head.x), i32::from(head.y)))
}

/// `0x0063a980`: count the nodes in the player's anchor chain.
///
/// ```c
/// iVar2 = 0;
/// for (puVar1 = (&DAT_00fc86ac)[param_1 * 2]; puVar1 != 0; puVar1 = *puVar1)
///   iVar2 = iVar2 + 1;
/// return iVar2;
/// ```
///
/// `head` is the `player * 2` table word; `None` is the shipped null.
pub fn a84_chain_len<T>(chain: &[A84Link<T>], head: Option<u32>) -> u32 {
    let mut count = 0u32;
    let mut cursor = head;
    while let Some(index) = cursor {
        count = count.wrapping_add(1);
        cursor = chain[index as usize].next;
    }
    count
}

// ---------------------------------------------------------------------------
// 0x0063ac10 / 0x0063b000 / 0x0063b1f0 - table id <-> pointer arithmetic
// ---------------------------------------------------------------------------

/// `0x0063ac10`: anchor-table id from a record pointer.
///
/// ```c
/// if (param_1 == 0) return 0;
/// return (param_1 - 0xfbd2f0U) / 0x1cc + 1;
/// ```
///
/// The subtraction and division are unsigned 32-bit operations; a pointer
/// below `0x00fbd2f0` wraps before the division.  `0x1cc` is the record
/// stride and the `+ 1` makes the first record id 1 (0 means null).
pub fn a84_anchor_id_from_pointer(pointer: u32) -> u32 {
    if pointer == 0 {
        0
    } else {
        pointer
            .wrapping_sub(0x00fb_d2f0)
            .wrapping_div(0x1cc)
            .wrapping_add(1)
    }
}

/// `0x0063b000`: anchor-table pointer from an id.
///
/// ```c
/// if (param_1 == 0) return 0;
/// return &DAT_00fbd124 + param_1 * 0x1cc;
/// ```
///
/// `0x00fbd124 + 0x1cc == 0x00fbd2f0`, the subtraction base used by
/// [`a84_anchor_id_from_pointer`], so the two conversions round-trip.
pub fn a84_anchor_pointer_from_id(id: u32) -> Option<u32> {
    if id == 0 {
        None
    } else {
        Some(0x00fb_d124u32.wrapping_add(id.wrapping_mul(0x1cc)))
    }
}

/// `0x0063b1f0`: slot id from a record pointer.
///
/// ```c
/// if (param_1 == 0) return 0;
/// return (param_1 - 0xfac940U) / 0x18 + 1;
/// ```
///
/// Same unsigned shape as [`a84_anchor_id_from_pointer`] with the slot table
/// base `0x00fac940` and the `0x18` stride.
pub fn a84_slot_id_from_pointer(pointer: u32) -> u32 {
    if pointer == 0 {
        0
    } else {
        pointer
            .wrapping_sub(0x00fa_c940)
            .wrapping_div(0x18)
            .wrapping_add(1)
    }
}

// ---------------------------------------------------------------------------
// 0x0063ac40 / 0x0063b420 / 0x0063b430 - anchor field accessors
// ---------------------------------------------------------------------------

/// `0x0063ac40`: read the current selection word through the focus chain.
///
/// ```c
/// if (*(int *)(param_1 + 0x134) == 0) return 0;
/// return *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x134) + 0x14) + 0x28);
/// ```
///
/// `anchor` is the `+0x134` word (`None` for the null gate); `current` is the
/// word at `[[anchor + 0x14] + 0x28]`.  The inner two dereferences are not
/// null-checked by the row, so the caller supplies the already-loaded word.
pub fn a84_anchor_current(anchor: Option<u32>, current: u32) -> u32 {
    match anchor {
        None => 0,
        Some(_) => current,
    }
}

/// `0x0063b420`: read the anchor's `+0x1c` marker byte.
///
/// The row is a single `mov al, [param_1 + 0x1c]`; the byte is returned
/// without sign extension.
pub fn a84_anchor_marker(anchor: &A84Anchor) -> u8 {
    anchor.marker
}

/// `0x0063b430`: mark the anchor as visited.
///
/// ```c
/// if (*(int *)(param_1 + 0x134) != 0)
///   *(undefined1 *)(*(int *)(*(int *)(param_1 + 0x134) + 0x14) + 0x1e) = 1;
/// ```
///
/// `attached` is `*(int *)(param_1 + 0x134) != 0`; the function returns
/// whether the store ran.
pub fn a84_anchor_mark_visited(anchor: &mut A84Anchor, attached: bool) -> bool {
    if attached {
        anchor.visited = 1;
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// 0x0063ac60 / 0x0063b100 - bounds-gated effect placement
// ---------------------------------------------------------------------------

/// The arguments of the `FUN_00638a70` call from `0x0063ac60`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A84PlaceCall {
    /// The second argument, a constant 0 in this row.
    pub mode: u32,
    /// The X argument.
    pub x: u32,
    /// The Y argument.
    pub y: u32,
    /// The first payload word.
    pub first: u32,
    /// The second payload word.
    pub second: u32,
}

/// `0x0063ac60`: place an effect when the position is inside the map bounds.
///
/// ```c
/// if (param_2 < 0x1f && param_3 < 0xe4)
///   FUN_00638a70(param_1, 0, param_2, param_3, param_4, param_5);
/// ```
///
/// Both comparisons are unsigned.  The call is reproduced as a value so the
/// exact `(0, x, y, first, second)` argument order stays testable.
pub fn a84_effect_bounds_gate(x: u32, y: u32, first: u32, second: u32) -> Option<A84PlaceCall> {
    if x < 0x1f && y < 0xe4 {
        Some(A84PlaceCall {
            mode: 0,
            x,
            y,
            first,
            second,
        })
    } else {
        None
    }
}

/// The arguments of the `FUN_00638a70` call from `0x0063b100`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A84SlotCall {
    /// The second argument, the constant 2.
    pub mode: u32,
    /// The third argument, the constant 1.
    pub column: u32,
    /// The bounded slot index.
    pub index: u32,
    /// The payload word.
    pub value: u32,
    /// The last argument, the constant 0.
    pub extra: u32,
}

/// `0x0063b100`: the slot-index range gate.
///
/// ```c
/// if (param_2 < 0x2c)
///   FUN_00638a70(param_1, 2, 1, param_2, param_3, 0);
/// ```
///
/// The comparison is unsigned and the call always uses mode 2, column 1 and
/// a zero last argument.
pub fn a84_slot_bounds_gate(index: u32, value: u32) -> Option<A84SlotCall> {
    if index < 0x2c {
        Some(A84SlotCall {
            mode: 2,
            column: 1,
            index,
            value,
            extra: 0,
        })
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// 0x0063b197 - empty body
// ---------------------------------------------------------------------------

/// `0x0063b197`: an empty two-byte function.
///
/// The shipped body is a single `ret`; it has no observable effect and no
/// return value.
pub fn a84_noop() {}

// ---------------------------------------------------------------------------
// 0x0063b1a0 - detach the anchor from a focus
// ---------------------------------------------------------------------------

/// The result of detaching an anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A84Detach {
    /// The `func_0x005b3b60() != 0 && focus + 0x134 != 0` gate passed.
    pub detached: bool,
    /// The `+0x134` anchor word that was cleared, if any.
    pub anchor: Option<u32>,
    /// The `[anchor + 0x14]` owner word passed to `FUN_00637810`.
    pub owner: Option<u32>,
}

/// `0x0063b1a0`: clear the focus's anchor and release it.
///
/// ```c
/// if (func_0x005b3b60() == 0) return;
/// iVar2 = *(int *)(param_1 + 0x134);
/// if (iVar2 == 0) return;
/// uVar1 = *(undefined4 *)(iVar2 + 0x14);
/// *(undefined4 *)(param_1 + 0x134) = 0;
/// FUN_00636a10(iVar2);
/// FUN_00637810(uVar1, param_1);
/// ```
///
/// The store order matters: the focus's `+0x134` word is cleared before the
/// anchor is released, and the owner word (`anchor + 0x14`) is captured
/// before that store.  `owner` is that captured word, passed to
/// `FUN_00637810`.
pub fn a84_detach_anchor(lookup_ok: bool, anchor: Option<u32>, owner: u32) -> A84Detach {
    match (lookup_ok, anchor) {
        (true, Some(anchor)) => A84Detach {
            detached: true,
            anchor: Some(anchor),
            owner: Some(owner),
        },
        _ => A84Detach::default(),
    }
}

// ---------------------------------------------------------------------------
// 0x0063b450 - chain probe
// ---------------------------------------------------------------------------

/// One node of the chain probed by `0x0063b450`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A84ProbeNode {
    /// `+0x00` next node.
    pub next: Option<u32>,
    /// `+0x0c` child list (the first abort test).
    pub children: Option<u32>,
    /// `+0x14` tail link (the second abort test).
    pub tail: Option<u32>,
}

/// How the chain probe stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A84ChildProbe {
    /// Every node had `+0x0c == 0` and `+0x14 == 0`; the row returns 0.
    Exhausted,
    /// A node had a non-null `+0x0c`; the row aborts.
    Children,
    /// A node had a non-null `+0x14`; the row aborts.
    Tail,
}

/// `0x0063b450`: scan the player's chain for a non-empty child or tail link.
///
/// ```c
/// puVar1 = (&DAT_00fc86ac)[param_1 * 2];
/// for (;;) {
///   if (puVar1 == 0) return 0;
///   if (puVar1[3] != 0) FUN_005b4cd0();   // +0x0c
///   if (puVar1[5] != 0) FUN_005b4cd0();   // +0x14
///   puVar1 = *puVar1;
/// }
/// ```
///
/// Both abort arms call the same non-returning terminator, so the enum only
/// distinguishes which field was observed for the caller's diagnostics.
pub fn a84_chain_child_probe(chain: &[A84ProbeNode], head: Option<u32>) -> A84ChildProbe {
    let mut cursor = head;
    while let Some(index) = cursor {
        let node = &chain[index as usize];
        if node.children.is_some() {
            return A84ChildProbe::Children;
        }
        if node.tail.is_some() {
            return A84ChildProbe::Tail;
        }
        cursor = node.next;
    }
    A84ChildProbe::Exhausted
}

// ---------------------------------------------------------------------------
// 0x0063b620 - choice decoding
// ---------------------------------------------------------------------------

/// The two word pushes emitted by the pair decoders of `0x0063b620`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A84ChoiceReport {
    /// The first argument of `FUN_005b7dd0` (`0x69` or `0xb7`).
    pub code: u32,
    /// The first pushed word: the row's second out-parameter (`local_c`).
    pub first: i32,
    /// The second pushed word: the row's first out-parameter (`local_8`).
    pub second: i32,
}

/// The three outcomes of `0x0063b620`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A84ChoiceOutcome {
    /// `LAB_0063b705`: `FUN_00635240(player)` then return 0.
    Finish,
    /// `LAB_0063b729`: both arms test `DAT_00f91fe8[code] & 2` and then call
    /// the non-returning terminator, so the result never comes back.
    Invalid,
    /// A pair decoder failed: two `FUN_005b7dd0` pushes, then
    /// `FUN_00635150(player)` and return 0.
    Pair(A84ChoiceReport),
}

/// `0x0063b620`: decode a command-card choice into a notification pair.
///
/// ```c
/// code = FUN_00635c20(player);
/// if (code == 0x2c || code == 0x3e) {
///     local_8 = FUN_006568e0(player);
///     if (local_8 != 0) goto invalid;
/// } else if (code == 0x44) {
///     if (FUN_00656970(player, &local_8, &local_c) != 0) {
///         FUN_005b7dd0(0x69, local_c);
///         FUN_005b7dd0(0x69, local_8);
///         FUN_00635150(player);
///         return 0;
///     }
/// } else if (code != 0x3f) {
///     if (code == 0x67) {
///         local_8 = FUN_00656850(player);
///         if (local_8 == 0) goto finish;
///     }
///     invalid: if ((DAT_00f91fe8[code] & 2) != 0) abort; abort;
/// } else if (FUN_00656730(player, &local_8, &local_c) != 0) {
///     FUN_005b7dd0(0xb7, local_c);
///     FUN_005b7dd0(0xb7, local_8);
///     FUN_00635150(player);
///     return 0;
/// }
/// finish: FUN_00635240(player);
/// return 0;
/// ```
///
/// `decoded_nonzero` is the `local_8` result of `FUN_006568e0` (codes
/// `0x2c`/`0x3e`) or `FUN_00656850` (code `0x67`); `decode_failed` is the
/// non-zero result of `FUN_00656970`/`FUN_00656730`; `first`/`second` are the
/// `local_8`/`local_c` out-words.  The `DAT_00f91fe8` test on the invalid
/// path cannot change the outcome, because both of its arms abort.
pub fn a84_choice_outcome(
    code: u32,
    decoded_nonzero: bool,
    decode_failed: bool,
    first: i32,
    second: i32,
) -> A84ChoiceOutcome {
    match code {
        0x2c | 0x3e => {
            if decoded_nonzero {
                A84ChoiceOutcome::Invalid
            } else {
                A84ChoiceOutcome::Finish
            }
        }
        0x44 => {
            if decode_failed {
                A84ChoiceOutcome::Pair(A84ChoiceReport {
                    code: 0x69,
                    first: second,
                    second: first,
                })
            } else {
                A84ChoiceOutcome::Finish
            }
        }
        0x3f => {
            if decode_failed {
                A84ChoiceOutcome::Pair(A84ChoiceReport {
                    code: 0xb7,
                    first: second,
                    second: first,
                })
            } else {
                A84ChoiceOutcome::Finish
            }
        }
        0x67 => {
            if decoded_nonzero {
                A84ChoiceOutcome::Invalid
            } else {
                A84ChoiceOutcome::Finish
            }
        }
        _ => A84ChoiceOutcome::Invalid,
    }
}

// ---------------------------------------------------------------------------
// 0x0063b858 - finish dispatch
// ---------------------------------------------------------------------------

/// The two calls the finish dispatch can make.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A84FinishCall {
    /// `FUN_006594e0(record)` when `FUN_00635c40() == 2`.
    Clear(i32),
    /// `FUN_00654420(record, saved)` for every other mode.
    Cancel(i32, u32),
}

/// The finish dispatch of `0x0063b858`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A84Finish {
    /// The selected call.
    pub call: A84FinishCall,
    /// `FUN_00635150()` runs before the abort.
    pub finish: bool,
    /// The row ends in the non-returning `FUN_005b4cd0`.
    pub terminate: bool,
}

/// `0x0063b858`: clear or cancel the pending choice.
///
/// ```c
/// if (FUN_00635c40() == 2) {
///     uVar2 = FUN_00635c00();
///     FUN_006594e0(uVar2);
/// } else {
///     uVar2 = *(undefined4 *)(EBP - 4);
///     uVar3 = FUN_00635c00();
///     FUN_00654420(uVar3, uVar2);
/// }
/// FUN_00635150();
/// FUN_005b4cd0();
/// ```
///
/// `record` is the `FUN_00635c00()` result and `saved` is the caller-frame
/// word at `EBP-4`.  Both arms end in the terminator, so the plan always has
/// `finish` and `terminate` set.
pub fn a84_finish_dispatch(mode: u32, record: i32, saved: u32) -> A84Finish {
    A84Finish {
        call: if mode == 2 {
            A84FinishCall::Clear(record)
        } else {
            A84FinishCall::Cancel(record, saved)
        },
        finish: true,
        terminate: true,
    }
}

// ---------------------------------------------------------------------------
// 0x0063ba0e - mode combine
// ---------------------------------------------------------------------------

/// The value and truth flag produced by `0x0063ba0e`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A84Combine {
    /// The rebuilt dword.
    pub value: u32,
    /// The returned `value != 0` test.
    pub nonzero: bool,
}

/// `0x0063ba0e`: rebuild the low byte and combine it with the operand.
///
/// ```c
/// uVar1 = CONCAT31((int3)((uint)in_EAX >> 8), (char)in_EAX + '+');
/// if (in_EDX == 1)      uVar1 = uVar1 + in_ECX;
/// else if (in_EDX == 2) uVar1 = uVar1 ^ in_ECX;
/// *(uint *)(EBP + 8) = uVar1;
/// return CONCAT31(..., *(int *)(EBP + 8) != 0);
/// ```
///
/// `CONCAT31` keeps the upper 24 bits of `EAX` and replaces only the low
/// byte, so the carry out of `low + 0x2b` is discarded.  The mode-1 addition
/// and mode-2 exclusive-or then operate on the full dword.  Mode `0` (and
/// every other mode) leaves the rebuilt value unchanged.
pub fn a84_mode_combine(base: u32, operand: u32, mode: i32) -> A84Combine {
    let low = (base as u8).wrapping_add(0x2b);
    let mut value = (base & 0xffff_ff00) | u32::from(low);
    match mode {
        1 => value = value.wrapping_add(operand),
        2 => value ^= operand,
        _ => {}
    }
    A84Combine {
        value,
        nonzero: value != 0,
    }
}

// ---------------------------------------------------------------------------
// 0x0063bbcd - escape dispatch
// ---------------------------------------------------------------------------

/// The branch taken by `0x0063bbcd`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A84Escape {
    /// `in_AX == 5`: `FUN_0063e880`.
    Close,
    /// `FUN_005b5850() != 0`: `FUN_0063ce60`.
    Secondary,
    /// Both tests failed; the row returns without a call.
    None,
}

/// `0x0063bbcd`: escape-key dispatch.
///
/// ```c
/// if (in_AX == 5) { FUN_0063e880(); return; }
/// if (FUN_005b5850() != 0) FUN_0063ce60();
/// ```
///
/// The `AX == 5` test is checked first and suppresses the secondary branch.
pub fn a84_escape_dispatch(key: u16, secondary_gate: bool) -> A84Escape {
    if key == 5 {
        A84Escape::Close
    } else if secondary_gate {
        A84Escape::Secondary
    } else {
        A84Escape::None
    }
}

// ---------------------------------------------------------------------------
// 0x0063af10 - shrink a rectangle by the selected units
// ---------------------------------------------------------------------------

/// Unit state bytes that make `0x0063af10` subtract their footprint.
pub const A84_SHRINK_STATES: [u8; 5] = [0x1e, 0x1f, 0x46, 0x1a, 0x19];

/// One child of a selection node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A84ShrinkNode {
    /// `*(char *)(node[4] + 0x4d)`.
    pub state: u8,
    /// `FUN_0067d0f0(player, ...) != 0`, the per-unit gate.
    pub accepted: bool,
}

/// `0x0063af10`: subtract each matching unit's footprint from a rectangle.
///
/// ```c
/// if (param_1 < 8)
///   for (puVar2 = (&DAT_00fc86ac)[param_1 * 2]; puVar2 != 0; puVar2 = *puVar2)
///     for (puVar3 = puVar2[3]; puVar3 != 0; puVar3 = *puVar3) {
///       cVar1 = *(char *)(puVar3[4] + 0x4d);
///       if (cVar1 == 0x1e || cVar1 == 0x1f || cVar1 == 0x46 ||
///           cVar1 == 0x1a || cVar1 == 0x19) {
///         if (FUN_0067d0f0(param_1, FUN_0058deb0(0), 1, 1) != 0) {
///           *param_2 -= FUN_0067d290(param_1);   // width
///           *param_3 -= FUN_0067d280(param_1);   // height
///         }
///       }
///     }
/// ```
///
/// `groups` is the list of child lists (`puVar2[3]` for each anchor node),
/// `measure` is `[width, height]`, and `rect` is `[width, height]`.  The
/// `player < 8` gate is reproduced exactly; `accepted` stands for the
/// per-unit `FUN_0067d0f0` result.
pub fn a84_shrink_rect_by_selection(
    player: u32,
    groups: &[&[A84ShrinkNode]],
    measure: [i32; 2],
    rect: [i32; 2],
) -> [i32; 2] {
    let mut out = rect;
    if player >= 8 {
        return out;
    }
    for group in groups {
        for node in *group {
            if A84_SHRINK_STATES.contains(&node.state) && node.accepted {
                out[0] = out[0].wrapping_sub(measure[0]);
                out[1] = out[1].wrapping_sub(measure[1]);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 0x0063ac90 - follow/order controller
// ---------------------------------------------------------------------------

/// One node of the owner's unit list, as consumed by the candidate scan.
///
/// `eligible` is `FUN_005b4bd0() == 0 && FUN_006547b0(value) != 0`;
/// `measured` is the following `FUN_005b5970() != 0`; `abort` is the second
/// `FUN_005b4bd0() == 0` taken when `measured` is false.  `start`/`end` are
/// the `FUN_00588d60()`/`FUN_005b4b30()` results whose signed difference the
/// row compares as unsigned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A84FollowCandidate {
    /// `node[4]`, the value reported on a win.
    pub value: u32,
    /// The two-part eligibility gate.
    pub eligible: bool,
    /// `FUN_005b5970() != 0`.
    pub measured: bool,
    /// Stop the outer walk after this node.
    pub abort: bool,
    /// `FUN_00588d60()`.
    pub start: i32,
    /// `FUN_005b4b30()`.
    pub end: i32,
}

/// The player/owner state consumed by [`a84_follow_plan`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A84FollowInputs {
    /// The owning player index, `param_1` of the row.
    pub player: u32,
    /// `*(int *)(focus + 0x134) != 0`.
    pub focus_live: bool,
    /// `*(char *)(focus + 0x0b)`, the cooldown byte.
    pub cooldown: u8,
    /// `FUN_005b5250() == 1`.
    pub single_player: bool,
    /// `*(int *)(owner + 0x28)`.
    pub owner_current: u32,
    /// `FUN_005c7af0(player)`; 0 means no locked target.
    pub lock_value: u32,
    /// `FUN_00637f00(owner, player) != 0` (the row skips the follow block).
    pub follow_open: bool,
    /// `*(int *)(owner + 0x24) != 0`.
    pub owner_busy: bool,
    /// `FUN_005b6170() != 0`.
    pub order_ok: bool,
    /// `FUN_005b55c0()`; 0 means no scripted target.
    pub have_target: u32,
    /// `FUN_005ad950() != 0`.
    pub hero_ok: bool,
    /// `*(char *)(focus + 9)`, the follow state.
    pub state: u8,
    /// `*(int *)(owner + 0x2c)`; 0 means no unit.
    pub owner_unit: u32,
    /// `*(char *)(unit + 0xd9) == 0`.
    pub unit_ready: bool,
    /// `FUN_006387f0(owner, kind, mode) != 0` for `kind * 4 + mode`.
    pub ready: [bool; 16],
}

/// The action selected by [`a84_follow_plan`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum A84FollowAction {
    /// Nothing to do; the row falls through and returns.
    #[default]
    None,
    /// Decrement `focus + 0x0b` and return.
    TickCooldown(u8),
    /// `FUN_005b7dd0(0x22, value)` then `FUN_005b67c0(player)`.
    Track(u32),
    /// `FUN_006389e0(player, target, 0)` from the `focus + 9 == 1` hero arm.
    Hero {
        /// `FUN_005b55c0()`.
        target: u32,
    },
    /// `FUN_006389e0(player, target, probe)` where `probe` is the first
    /// non-zero `FUN_006387f0(kind, mode)`.
    Select {
        /// `owner + 0x2c`.
        target: u32,
        /// The first successful `(kind, mode)` probe, or `None` when every
        /// probe returned zero (the call then passes 0).
        probe: Option<(u8, u8)>,
    },
    /// `FUN_0058b640` (non-returning) from the state-1 arm.
    Terminate,
}

/// The result of [`a84_follow_plan`], including the row's two stores.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A84FollowPlan {
    /// The action the row selects.
    pub action: A84FollowAction,
    /// `focus + 0x0c` receives the global frame counter.
    pub stamp: bool,
    /// `owner + 0x28` receives the player before the lock query.
    pub claim_owner: bool,
}

/// `0x0063ac90`: the follow/order controller.
///
/// Recovered fragments:
///
/// ```c
/// if (focus == 0) return;
/// if (*(char *)(focus + 0xb) != 0) { *(char *)(focus + 0xb) -= 1; return; }
/// *(undefined4 *)(focus + 0xc) = frame_counter;
/// if (FUN_005b5250() != 1) goto follow;
/// for (node = *(int **)(owner + 0x14); node; node = *node) {
///   value = node[4];
///   if (FUN_005b4bd0() != 0) continue;
///   if (FUN_006547b0(value) == 0) continue;
///   if (FUN_005b5970() == 0) { if (FUN_005b4bd0() == 0) break; continue; }
///   start = FUN_00588d60(); end = FUN_005b4b30();
///   if (best < (uint)(end - start)) { best = end - start; choice = value; }
/// }
/// if (choice != 0) { FUN_005b7dd0(0x22, choice); FUN_005b67c0(player); return; }
/// if (*(int *)(owner + 0x28) == 0) *(int *)(owner + 0x28) = player;
/// else if (*(int *)(owner + 0x28) != player) goto follow;
/// if (FUN_005c7af0(player) != 0) { FUN_005b7dd0(0x22, ...); FUN_005b67c0(player); return; }
/// follow:
/// if (FUN_00637f00(owner, player) == 0 && *(int *)(owner + 0x24) != 0 &&
///     FUN_005b6170() != 0) {
///   have = FUN_005b55c0();
///   state = *(char *)(focus + 9);
///   if (state == 1) {
///     if (have != 0 && FUN_005ad950() != 0) { FUN_006389e0(player, have, 0); return; }
///     unit = *(int *)(owner + 0x2c);
///     if (unit != 0 && *(char *)(unit + 0xd9) == 0) {
///       pick = FUN_006387f0(owner, 3, 1); if (!pick) pick = FUN_006387f0(owner, 2, 1);
///       FUN_006389e0(player, unit, pick); return;
///     }
///     FUN_0058b640();
///   }
///   mode = state == 2 ? 1 : state == 3 ? 2 : 3;
///   pick = 0;
///   for ((kind, mode) in [(3,mode),(3,1),(3,2),(3,3),(2,1),(2,2),(2,3)])
///     if (!pick) pick = FUN_006387f0(owner, kind, mode);
///   FUN_006389e0(player, pick, *(undefined4 *)(owner + 0x2c));
/// }
/// ```
///
/// The candidate scan compares the signed `end - start` as an unsigned
/// delta and replaces the winner only on a strict `<`, so the first node
/// wins ties.  The frame counter itself lives in a runtime table
/// (`DAT_00f063cc ^ 0x10fae560`, offset `0xe608`), so only the store is
/// recorded (`stamp`).
pub fn a84_follow_plan(
    inputs: &A84FollowInputs,
    candidates: &[A84FollowCandidate],
) -> A84FollowPlan {
    let mut plan = A84FollowPlan::default();
    if !inputs.focus_live {
        return plan;
    }
    if inputs.cooldown != 0 {
        plan.action = A84FollowAction::TickCooldown(inputs.cooldown.wrapping_sub(1));
        return plan;
    }
    plan.stamp = true;
    if inputs.single_player {
        if let Some(best) = a84_best_follow_candidate(candidates) {
            plan.action = A84FollowAction::Track(best);
            return plan;
        }
        if inputs.owner_current == 0 {
            plan.claim_owner = true;
        } else if inputs.owner_current != inputs.player {
            return a84_follow_block(inputs, plan);
        }
        if inputs.lock_value != 0 {
            plan.action = A84FollowAction::Track(inputs.lock_value);
            return plan;
        }
    }
    a84_follow_block(inputs, plan)
}

/// The candidate scan nested in `0x0063ac90`.
///
/// `best_delta` is an unsigned 32-bit local initialised to 0, so a candidate
/// whose `end - start` wraps to a huge value still wins.  The scan stops at
/// the first `abort` node that is eligible but not measured.  `None` means
/// the row's `local_8 != 0` test failed; a winning node whose `value` is
/// literally 0 is indistinguishable from "no winner" in the shipped test,
/// so it is reported as `None` here too.
pub fn a84_best_follow_candidate(candidates: &[A84FollowCandidate]) -> Option<u32> {
    let mut best = 0u32;
    let mut best_delta = 0u32;
    for candidate in candidates {
        if !candidate.eligible {
            continue;
        }
        if candidate.measured {
            let delta = candidate.end.wrapping_sub(candidate.start) as u32;
            if best_delta < delta {
                best_delta = delta;
                best = candidate.value;
            }
        } else if candidate.abort {
            break;
        }
    }
    if best == 0 { None } else { Some(best) }
}

fn a84_first_ready(ready: &[bool; 16], probes: &[(u8, u8)]) -> Option<(u8, u8)> {
    probes
        .iter()
        .copied()
        .find(|&(kind, mode)| ready[kind as usize * 4 + mode as usize])
}

fn a84_follow_block(inputs: &A84FollowInputs, mut plan: A84FollowPlan) -> A84FollowPlan {
    if inputs.follow_open || !inputs.owner_busy || !inputs.order_ok {
        return plan;
    }
    if inputs.state == 1 {
        if inputs.have_target != 0 && inputs.hero_ok {
            plan.action = A84FollowAction::Hero {
                target: inputs.have_target,
            };
            return plan;
        }
        if inputs.owner_unit != 0 && inputs.unit_ready {
            plan.action = A84FollowAction::Select {
                target: inputs.owner_unit,
                probe: a84_first_ready(&inputs.ready, &[(3, 1), (2, 1)]),
            };
            return plan;
        }
        plan.action = A84FollowAction::Terminate;
        return plan;
    }
    let mode = match inputs.state {
        2 => 1,
        3 => 2,
        _ => 3,
    };
    plan.action = A84FollowAction::Select {
        target: inputs.owner_unit,
        probe: a84_first_ready(
            &inputs.ready,
            &[(3, mode), (3, 1), (3, 2), (3, 3), (2, 1), (2, 2), (2, 3)],
        ),
    };
    plan
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent84 {
    use super::*;

    // -- 0x0063a151 / 0x0063a2d0 ------------------------------------------

    #[test]
    fn anchor_rect_uses_the_half_span_added_to_the_low_corner() {
        let anchor = a84_anchor_from_rect(7, 0x1234_5678, &[10, 20, 15, 27]);
        assert_eq!(anchor.kind, 7);
        assert_eq!(anchor.marker, 0x78);
        assert!(anchor.enabled);
        // (15 - 10) / 2 = 2, so 10 + 2 = 12.
        assert_eq!(anchor.x, 12);
        // (27 - 20) / 2 = 3, so 20 + 3 = 23.
        assert_eq!(anchor.y, 23);
        assert_eq!(anchor.marker, a84_anchor_marker(&anchor));
    }

    #[test]
    fn anchor_rect_odd_span_drops_the_remainder_before_adding() {
        let anchor = a84_anchor_from_rect(0, 1, &[-5, -5, -2, -2]);
        assert_eq!(anchor.x, -4); // (-2 - -5) / 2 = 1, -5 + 1 = -4
        assert_eq!(anchor.y, -4);
    }

    #[test]
    fn anchor_rect_wraps_through_sixteen_bits() {
        let anchor = a84_anchor_from_rect(0, -1, &[0, 0, 0x2_0000, 0x2_0000]);
        assert_eq!(anchor.marker, 0xff);
        assert!(anchor.enabled);
        // 0x20000 / 2 = 0x10000, truncated to 16 bits.
        assert_eq!(anchor.x, 0);
        assert_eq!(anchor.y, 0);
        let zero = a84_anchor_from_rect(0, 0, &[0, 0, 0, 0]);
        assert!(!zero.enabled);
    }

    #[test]
    fn anchor_validation_requires_every_live_word() {
        assert!(a84_all_anchors_live(&[]));
        assert!(a84_all_anchors_live(&[true, true]));
        assert!(!a84_all_anchors_live(&[true, false, true]));
    }

    #[test]
    fn anchor_create_chooses_5_only_when_the_head_was_null() {
        let seed = A84Anchor {
            kind: 9,
            marker: 0,
            enabled: false,
            visited: 0,
            x: 11,
            y: 12,
        };
        let made = a84_anchor_create_outcome(seed, true, true, false, 0x1_0042).unwrap();
        assert_eq!(made.effect_kind, 5);
        assert!(made.initialise_head);
        assert_eq!(made.anchor.marker, 0x42);
        assert!(made.anchor.enabled);
        assert_eq!((made.anchor.x, made.anchor.y), (11, 12));

        let made = a84_anchor_create_outcome(seed, true, true, true, 0).unwrap();
        assert_eq!(made.effect_kind, 2);
        assert!(!made.initialise_head);
        assert!(!made.anchor.enabled);

        assert!(a84_anchor_create_outcome(seed, false, true, true, 1).is_none());
        assert!(a84_anchor_create_outcome(seed, true, false, true, 1).is_none());
    }

    // -- 0x0063a3d1 / 0x0063a413 ------------------------------------------

    fn gates(mode: u32) -> A84ModalGates {
        A84ModalGates {
            mode,
            key: 0x84,
            key_flags: 0x06,
            record_id: 5,
            object_id: 5,
            focus_live: true,
            focus_state: 3,
            tracking: true,
            enabled: true,
            selectable: true,
            is_unit: true,
            error_code: 0,
            spawn_ok: true,
            command_ok: true,
            attach_ok: true,
            target_ok: true,
        }
    }

    #[test]
    fn modal_plan_mode_three_accepts_the_recovered_key_set() {
        for key in [0x84, 0x85, 0x92, 0x90, 0x89] {
            let mut g = gates(3);
            g.key = key;
            let plan = a84_modal_plan(&g);
            assert_eq!(plan.result, 1, "key {key:#x}");
            assert_eq!(plan.notify, Some(A84Notify::Command(0x2b)));
            assert!(plan.finish);
            assert!(!plan.terminate);
        }
        let mut g = gates(3);
        g.key = 0x86;
        let plan = a84_modal_plan(&g);
        assert_eq!(plan.result, 0);
        assert_eq!(plan.notify, None);
    }

    #[test]
    fn modal_plan_mode_three_checks_enabled_key_and_record_before_action() {
        let mut g = gates(3);
        g.enabled = false;
        assert_eq!(a84_modal_plan(&g).result, 0);

        let mut g = gates(3);
        g.key = 0x2a;
        assert!(a84_modal_plan(&g).terminate);

        let mut g = gates(3);
        g.record_id = 6;
        let plan = a84_modal_plan(&g);
        assert_eq!(plan.result, 0);
        assert!(!plan.terminate);
    }

    #[test]
    fn modal_plan_mode_three_error_codes_abort_but_others_just_ignore() {
        for code in A84_ERROR_CODES {
            let mut g = gates(3);
            g.selectable = false;
            g.error_code = code;
            let plan = a84_modal_plan(&g);
            assert_eq!(plan.result, 0);
            assert!(plan.cleanup);
            assert!(plan.terminate, "code {code:#x}");
        }
        let mut g = gates(3);
        g.selectable = false;
        g.error_code = 0x99;
        let plan = a84_modal_plan(&g);
        assert_eq!(plan.result, 0);
        assert!(!plan.cleanup);
        assert!(!plan.terminate);

        let mut g = gates(3);
        g.spawn_ok = false;
        let plan = a84_modal_plan(&g);
        assert_eq!(plan.result, 0);
        assert_eq!(plan.notify, None);
    }

    #[test]
    fn modal_plan_covers_the_remaining_modes() {
        let mut g = gates(0);
        g.tracking = false;
        assert_eq!(a84_modal_plan(&g).result, 1);
        g.tracking = true;
        let plan = a84_modal_plan(&g);
        assert_eq!(plan.result, 0);
        assert!(plan.decode_choice);

        for mode in [1, 2] {
            let plan = a84_modal_plan(&gates(mode));
            assert_eq!(plan.result, 0);
            assert!(plan.decode_choice);
        }

        let mut g = gates(4);
        let plan = a84_modal_plan(&g);
        assert_eq!(plan.result, 1);
        assert_eq!(plan.key_flags, Some(0x06));
        assert!(plan.terminate);
        g.record_id = 9;
        assert!(!a84_modal_plan(&g).terminate);

        for (mode, code) in [(5, 0x4c), (6, 0x4b)] {
            let plan = a84_modal_plan(&gates(mode));
            assert_eq!(plan.result, 1);
            assert_eq!(plan.notify, Some(A84Notify::Command(code)));
            assert!(plan.finish);

            let mut g = gates(mode);
            g.attach_ok = false;
            let plan = a84_modal_plan(&g);
            assert_eq!(plan.result, 1);
            assert_eq!(plan.notify, None);

            let mut g = gates(mode);
            g.target_ok = false;
            assert_eq!(a84_modal_plan(&g).notify, None);

            let mut g = gates(mode);
            g.command_ok = false;
            g.error_code = 0x17;
            let plan = a84_modal_plan(&g);
            assert_eq!(plan.result, 1);
            assert!(plan.cleanup);
            assert!(plan.terminate);
        }

        let plan = a84_modal_plan(&gates(7));
        assert_eq!(plan.result, 1);
        assert!(plan.refresh);

        let mut g = gates(8);
        g.key = 0x54;
        assert!(a84_modal_plan(&g).terminate);
        g.key = 0x55;
        assert_eq!(a84_modal_plan(&g).result, 0);

        assert_eq!(a84_modal_plan(&gates(9)).result, 1);

        let mut g = gates(3);
        g.focus_live = false;
        assert_eq!(a84_modal_plan(&g).result, 1);
        g.focus_live = true;
        g.focus_state = 2;
        assert_eq!(a84_modal_plan(&g).result, 1);
    }

    #[test]
    fn modal_key_handler_runs_the_three_pre_key_actions() {
        let plan = a84_modal_key_handler(0x2a, true, true, &gates(3));
        assert_eq!(plan.pre, A84PreKeyAction::ScreenPing);
        assert_eq!(plan.notify, Some(A84Notify::Command(0x2b)));

        let plan = a84_modal_key_handler(0x2a, false, true, &gates(3));
        assert_eq!(plan.pre, A84PreKeyAction::None);

        let plan = a84_modal_key_handler(0x6b, false, true, &gates(3));
        assert_eq!(plan.pre, A84PreKeyAction::ToggleBuilding);

        let plan = a84_modal_key_handler(0x6c, false, true, &gates(3));
        assert_eq!(plan.pre, A84PreKeyAction::FoodCheck);
        assert!(!plan.terminate);

        let plan = a84_modal_key_handler(0x6c, false, false, &gates(3));
        assert_eq!(plan.pre, A84PreKeyAction::FoodCheck);
        assert!(plan.terminate);
        assert_eq!(plan.notify, None);
    }

    #[test]
    fn modifier_prelude_selects_the_recovered_bit() {
        let send = a84_modifier_prelude(2, 0b010, [true, true, true]);
        assert!(send.command && send.cancel && !send.reset);
        let send = a84_modifier_prelude(2, 0b100, [true, true, true]);
        assert!(!send.command && send.reset);

        let send = a84_modifier_prelude(1, 0b001, [true, true, true]);
        assert!(send.command && !send.reset);
        let send = a84_modifier_prelude(1, 0b010, [true, true, true]);
        assert!(send.reset);

        let send = a84_modifier_prelude(0, 0b100, [true, true, true]);
        assert!(send.command && !send.reset);
        let send = a84_modifier_prelude(0, 0b010, [true, true, true]);
        assert!(send.reset);

        let send = a84_modifier_prelude(0, 0b100, [true, false, true]);
        assert!(!send.command && !send.cancel && !send.reset);
    }

    #[test]
    fn modal_modifier_handler_indexes_the_flag_table() {
        let flags = [0u8, 0, 4];
        let mut g = gates(3);
        g.key = 0x84;
        let plan = a84_modal_modifier_handler(2, 0, &flags, [true, true, true], &g);
        assert!(plan.modifier.command);
        assert!(plan.modifier.cancel);
        assert_eq!(plan.notify, Some(A84Notify::Command(0x2b)));

        let plan = a84_modal_modifier_handler(1, 0, &flags, [true, true, true], &g);
        assert!(plan.modifier.reset);
        assert!(!plan.modifier.command);
    }

    // -- 0x0063a625 / 0x0063a950 / 0x0063a980 -----------------------------

    #[test]
    fn confirm_notification_uses_both_entry_points() {
        let ok = a84_confirm_notification(0x23);
        assert_eq!(ok.call, A84Notify::Command(0x2a));
        assert!(ok.finish);
        assert_eq!(ok.result, 1);

        let other = a84_confirm_notification(0x24);
        assert_eq!(other.call, A84Notify::Cancel(0x26));
    }

    #[test]
    fn anchor_position_sign_extends_both_shorts() {
        let mut table = [[None; 2]; 8];
        table[3][0] = Some(A84Anchor {
            x: -5,
            y: 300,
            ..A84Anchor::default()
        });
        assert_eq!(a84_anchor_position(&table, 3), Some((-5, 300)));
        assert_eq!(a84_anchor_position(&table, 2), None);
        // The row reads the first word of the pair only.
        table[3][1] = Some(A84Anchor {
            x: 1,
            y: 2,
            ..A84Anchor::default()
        });
        assert_eq!(a84_anchor_position(&table, 3), Some((-5, 300)));
    }

    #[test]
    fn chain_len_counts_until_null() {
        let chain = [
            A84Link {
                next: Some(1),
                value: 10,
            },
            A84Link {
                next: Some(2),
                value: 20,
            },
            A84Link {
                next: None,
                value: 30,
            },
        ];
        assert_eq!(a84_chain_len(&chain, Some(0)), 3);
        assert_eq!(a84_chain_len(&chain, Some(2)), 1);
        assert_eq!(a84_chain_len(&chain, None), 0);
    }

    // -- 0x0063ac10 / 0x0063b000 / 0x0063b1f0 -----------------------------

    #[test]
    fn anchor_pointer_and_id_round_trip() {
        for id in [1u32, 2, 7, 0x20, 0x123] {
            let pointer = a84_anchor_pointer_from_id(id).unwrap();
            assert_eq!(a84_anchor_id_from_pointer(pointer), id);
            assert_eq!(pointer, 0x00fb_d124 + id * 0x1cc);
        }
        assert_eq!(a84_anchor_pointer_from_id(0), None);
        assert_eq!(a84_anchor_id_from_pointer(0), 0);
    }

    #[test]
    fn anchor_id_uses_unsigned_wrapping_for_pointers_below_the_base() {
        let pointer = 1u32;
        let expected = pointer.wrapping_sub(0x00fb_d2f0) / 0x1cc + 1;
        assert_eq!(a84_anchor_id_from_pointer(pointer), expected);
        // A pointer inside a record still truncates to the same id.
        assert_eq!(a84_anchor_id_from_pointer(0x00fb_d2f0 + 3 * 0x1cc + 5), 4);
        assert_eq!(a84_anchor_id_from_pointer(0x00fb_d2f0), 1);
    }

    #[test]
    fn slot_id_uses_its_own_base_and_stride() {
        let pointer = 0x00fa_c940 + 5 * 0x18;
        assert_eq!(a84_slot_id_from_pointer(pointer), 6);
        assert_eq!(a84_slot_id_from_pointer(0x00fa_c940), 1);
        assert_eq!(a84_slot_id_from_pointer(0), 0);
    }

    // -- 0x0063ac40 / 0x0063b420 / 0x0063b430 -----------------------------

    #[test]
    fn anchor_current_is_zero_when_the_anchor_gate_is_null() {
        assert_eq!(a84_anchor_current(None, 0x1234), 0);
        assert_eq!(a84_anchor_current(Some(0x2000), 0x1234), 0x1234);
    }

    #[test]
    fn visited_store_only_runs_when_attached() {
        let mut anchor = A84Anchor::default();
        assert!(!a84_anchor_mark_visited(&mut anchor, false));
        assert_eq!(anchor.visited, 0);
        assert!(a84_anchor_mark_visited(&mut anchor, true));
        assert_eq!(anchor.visited, 1);
        assert_eq!(a84_anchor_marker(&anchor), 0);
    }

    // -- 0x0063ac60 / 0x0063b100 ------------------------------------------

    #[test]
    fn effect_gate_uses_unsigned_bounds_1f_by_e4() {
        assert!(a84_effect_bounds_gate(0x1e, 0xe3, 1, 2).is_some());
        assert!(a84_effect_bounds_gate(0x1f, 0, 1, 2).is_none());
        assert!(a84_effect_bounds_gate(0, 0xe4, 1, 2).is_none());

        let call = a84_effect_bounds_gate(5, 6, 7, 8).unwrap();
        assert_eq!(
            call,
            A84PlaceCall {
                mode: 0,
                x: 5,
                y: 6,
                first: 7,
                second: 8,
            }
        );
    }

    #[test]
    fn slot_gate_bounds_are_2c_exclusive() {
        let call = a84_slot_bounds_gate(0x2b, 9).unwrap();
        assert_eq!(call.mode, 2);
        assert_eq!(call.column, 1);
        assert_eq!(call.index, 0x2b);
        assert_eq!(call.value, 9);
        assert_eq!(call.extra, 0);
        assert!(a84_slot_bounds_gate(0x2c, 9).is_none());
    }

    // -- 0x0063b1a0 / 0x0063b450 ------------------------------------------

    #[test]
    fn detach_needs_both_gates_and_reports_the_cleared_words() {
        let out = a84_detach_anchor(true, Some(0x4000), 0x5000);
        assert!(out.detached);
        assert_eq!(out.anchor, Some(0x4000));
        assert_eq!(out.owner, Some(0x5000));
        assert!(!a84_detach_anchor(false, Some(0x4000), 0x5000).detached);
        assert!(!a84_detach_anchor(true, None, 0x5000).detached);
    }

    #[test]
    fn child_probe_reports_which_link_stopped_it() {
        let chain = [
            A84ProbeNode {
                next: Some(1),
                children: None,
                tail: None,
            },
            A84ProbeNode {
                next: Some(2),
                children: Some(0x10),
                tail: None,
            },
            A84ProbeNode::default(),
        ];
        assert_eq!(
            a84_chain_child_probe(&chain, Some(0)),
            A84ChildProbe::Children
        );
        // Starting at node 2 skips the child and sees the null next.
        assert_eq!(
            a84_chain_child_probe(&chain, Some(2)),
            A84ChildProbe::Exhausted
        );
        assert_eq!(
            a84_chain_child_probe(&chain, None),
            A84ChildProbe::Exhausted
        );

        let tail = [A84ProbeNode {
            next: None,
            children: None,
            tail: Some(1),
        }];
        assert_eq!(a84_chain_child_probe(&tail, Some(0)), A84ChildProbe::Tail);
    }

    // -- 0x0063b620 / 0x0063b858 ------------------------------------------

    #[test]
    fn choice_outcome_reproduces_every_code_branch() {
        assert_eq!(
            a84_choice_outcome(0x2c, false, false, 0, 0),
            A84ChoiceOutcome::Finish
        );
        assert_eq!(
            a84_choice_outcome(0x3e, true, false, 0, 0),
            A84ChoiceOutcome::Invalid
        );
        assert_eq!(
            a84_choice_outcome(0x67, false, false, 0, 0),
            A84ChoiceOutcome::Finish
        );
        assert_eq!(
            a84_choice_outcome(0x67, true, false, 0, 0),
            A84ChoiceOutcome::Invalid
        );
        assert_eq!(
            a84_choice_outcome(0x44, false, false, 0, 0),
            A84ChoiceOutcome::Finish
        );
        assert_eq!(
            a84_choice_outcome(0x44, false, true, 11, 22),
            A84ChoiceOutcome::Pair(A84ChoiceReport {
                code: 0x69,
                first: 22,
                second: 11,
            })
        );
        assert_eq!(
            a84_choice_outcome(0x3f, false, true, 11, 22),
            A84ChoiceOutcome::Pair(A84ChoiceReport {
                code: 0xb7,
                first: 22,
                second: 11,
            })
        );
        for code in [0u32, 1, 0x2b, 0x45, 0x66, 0x68, 0xffff_ffff] {
            assert_eq!(
                a84_choice_outcome(code, false, false, 0, 0),
                A84ChoiceOutcome::Invalid
            );
        }
    }

    #[test]
    fn finish_dispatch_selects_clear_only_for_mode_two() {
        let clear = a84_finish_dispatch(2, 5, 9);
        assert_eq!(clear.call, A84FinishCall::Clear(5));
        assert!(clear.finish && clear.terminate);
        let cancel = a84_finish_dispatch(3, 5, 9);
        assert_eq!(cancel.call, A84FinishCall::Cancel(5, 9));
        assert!(cancel.finish && cancel.terminate);
    }

    // -- 0x0063ba0e / 0x0063bbcd ------------------------------------------

    #[test]
    fn mode_combine_rebuilds_only_the_low_byte() {
        let out = a84_mode_combine(0x1234_5600, 0, 0);
        assert_eq!(out.value, 0x1234_562b);
        assert!(out.nonzero);

        // 0xff + 0x2b = 0x12a; CONCAT31 drops the carry into the upper bits.
        let out = a84_mode_combine(0x1234_56ff, 0, 0);
        assert_eq!(out.value, 0x1234_562a);

        let out = a84_mode_combine(0x0000_0001, 0x10, 1);
        assert_eq!(out.value, 0x3c);
        let out = a84_mode_combine(0x0000_00ff, 0x0f, 2);
        assert_eq!(out.value, 0x25);
        assert!(out.nonzero);

        let out = a84_mode_combine(0xffff_ffd5, 0, 0);
        assert_eq!(out.value, 0xffff_ff00);
        assert!(out.nonzero);

        // Mode 0 with a zero result: 0 - 0x2b wraps, so use a base that makes
        // the low byte 0x00: 0xd5 + 0x2b = 0x100 -> truncated to 0x00.
        let out = a84_mode_combine(0x0000_00d5, 0, 0);
        assert_eq!(out.value, 0);
        assert!(!out.nonzero);
    }

    #[test]
    fn escape_dispatch_prefers_the_close_branch() {
        assert_eq!(a84_escape_dispatch(5, true), A84Escape::Close);
        assert_eq!(a84_escape_dispatch(6, true), A84Escape::Secondary);
        assert_eq!(a84_escape_dispatch(6, false), A84Escape::None);
    }

    // -- 0x0063af10 --------------------------------------------------------

    #[test]
    fn shrink_rect_subtracts_each_matching_accepted_unit() {
        let group_a = [
            A84ShrinkNode {
                state: 0x1e,
                accepted: true,
            },
            A84ShrinkNode {
                state: 0x2a,
                accepted: true,
            },
        ];
        let group_b = [
            A84ShrinkNode {
                state: 0x46,
                accepted: false,
            },
            A84ShrinkNode {
                state: 0x19,
                accepted: true,
            },
        ];
        let groups: [&[A84ShrinkNode]; 2] = [&group_a, &group_b];
        assert_eq!(
            a84_shrink_rect_by_selection(1, &groups, [8, 4], [100, 60]),
            [84, 52]
        );
        // Player 8 is out of range and the loops do not run.
        assert_eq!(
            a84_shrink_rect_by_selection(8, &groups, [8, 4], [100, 60]),
            [100, 60]
        );
    }

    // -- 0x0063ac90 --------------------------------------------------------

    fn follow_base() -> A84FollowInputs {
        A84FollowInputs {
            player: 1,
            focus_live: true,
            cooldown: 0,
            single_player: true,
            owner_current: 1,
            lock_value: 0,
            follow_open: false,
            owner_busy: true,
            order_ok: true,
            have_target: 0,
            hero_ok: false,
            state: 0,
            owner_unit: 0x9000,
            unit_ready: true,
            ready: [false; 16],
        }
    }

    #[test]
    fn follow_plan_ticks_the_cooldown_first() {
        let mut inputs = follow_base();
        inputs.cooldown = 3;
        let plan = a84_follow_plan(&inputs, &[]);
        assert_eq!(plan.action, A84FollowAction::TickCooldown(2));
        assert!(!plan.stamp);
    }

    #[test]
    fn follow_plan_picks_the_strictly_largest_unsigned_delta() {
        let candidates = [
            A84FollowCandidate {
                value: 0x10,
                eligible: true,
                measured: true,
                abort: false,
                start: 100,
                end: 140,
            },
            A84FollowCandidate {
                value: 0x20,
                eligible: true,
                measured: true,
                abort: false,
                start: 100,
                end: 150,
            },
            // Equal delta does not replace the first winner.
            A84FollowCandidate {
                value: 0x30,
                eligible: true,
                measured: true,
                abort: false,
                start: 300,
                end: 350,
            },
        ];
        assert_eq!(a84_best_follow_candidate(&candidates), Some(0x20));
        // A negative signed delta becomes a huge unsigned delta.
        let wrapped = [A84FollowCandidate {
            value: 0x40,
            eligible: true,
            measured: true,
            abort: false,
            start: 10,
            end: -10,
        }];
        assert_eq!(a84_best_follow_candidate(&wrapped), Some(0x40));
        // An eligible but unmeasured abort node stops the scan.
        let stopped = [
            A84FollowCandidate {
                value: 0x50,
                eligible: true,
                measured: false,
                abort: true,
                start: 0,
                end: 0,
            },
            A84FollowCandidate {
                value: 0x60,
                eligible: true,
                measured: true,
                abort: false,
                start: 0,
                end: 10,
            },
        ];
        assert_eq!(a84_best_follow_candidate(&stopped), None);
        assert_eq!(a84_best_follow_candidate(&[]), None);
    }

    #[test]
    fn follow_plan_tracks_a_candidate_or_claims_a_free_owner() {
        let inputs = follow_base();
        let candidates = [A84FollowCandidate {
            value: 0x77,
            eligible: true,
            measured: true,
            abort: false,
            start: 0,
            end: 9,
        }];
        let plan = a84_follow_plan(&inputs, &candidates);
        assert_eq!(plan.action, A84FollowAction::Track(0x77));
        assert!(plan.stamp);

        let mut inputs = follow_base();
        inputs.owner_current = 0;
        let plan = a84_follow_plan(&inputs, &[]);
        assert!(plan.claim_owner);
        // With no locked value the claim falls through to the follow block.
        assert_eq!(
            plan.action,
            A84FollowAction::Select {
                target: 0x9000,
                probe: None,
            }
        );
    }

    #[test]
    fn follow_plan_uses_the_lock_and_the_owner_gate() {
        let mut inputs = follow_base();
        inputs.lock_value = 0x2222;
        let plan = a84_follow_plan(&inputs, &[]);
        assert_eq!(plan.action, A84FollowAction::Track(0x2222));
        assert!(!plan.claim_owner);

        let mut inputs = follow_base();
        inputs.owner_current = 2;
        let plan = a84_follow_plan(&inputs, &[]);
        assert_eq!(
            plan.action,
            A84FollowAction::Select {
                target: 0x9000,
                probe: None,
            }
        );

        let mut inputs = follow_base();
        inputs.single_player = false;
        inputs.lock_value = 0x2222;
        let plan = a84_follow_plan(&inputs, &[]);
        assert_eq!(
            plan.action,
            A84FollowAction::Select {
                target: 0x9000,
                probe: None,
            }
        );
    }

    #[test]
    fn follow_plan_follow_block_preconditions() {
        for (open, busy, order) in [
            (true, true, true),
            (false, false, true),
            (false, true, false),
        ] {
            let mut inputs = follow_base();
            inputs.follow_open = open;
            inputs.owner_busy = busy;
            inputs.order_ok = order;
            let plan = a84_follow_plan(&inputs, &[]);
            assert_eq!(plan.action, A84FollowAction::None);
        }
    }

    #[test]
    fn follow_plan_state_one_prefers_the_hero_then_the_unit() {
        let mut inputs = follow_base();
        inputs.state = 1;
        inputs.have_target = 0xabc;
        inputs.hero_ok = true;
        assert_eq!(
            a84_follow_plan(&inputs, &[]).action,
            A84FollowAction::Hero { target: 0xabc }
        );

        inputs.hero_ok = false;
        inputs.ready[3 * 4 + 1] = false;
        inputs.ready[2 * 4 + 1] = true;
        assert_eq!(
            a84_follow_plan(&inputs, &[]).action,
            A84FollowAction::Select {
                target: 0x9000,
                probe: Some((2, 1)),
            }
        );

        inputs.owner_unit = 0;
        assert_eq!(
            a84_follow_plan(&inputs, &[]).action,
            A84FollowAction::Terminate
        );
    }

    #[test]
    fn follow_plan_state_derived_mode_and_probe_order() {
        let mut inputs = follow_base();
        inputs.ready[3 * 4 + 2] = true;
        inputs.state = 2; // mode 1 -> first probe is (3, 1)
        assert_eq!(
            a84_follow_plan(&inputs, &[]).action,
            A84FollowAction::Select {
                target: 0x9000,
                probe: Some((3, 2)),
            }
        );

        inputs.ready = [false; 16];
        inputs.state = 3; // mode 2
        inputs.ready[2 * 4 + 3] = true;
        assert_eq!(
            a84_follow_plan(&inputs, &[]).action,
            A84FollowAction::Select {
                target: 0x9000,
                probe: Some((2, 3)),
            }
        );

        inputs.ready = [false; 16];
        assert_eq!(
            a84_follow_plan(&inputs, &[]).action,
            A84FollowAction::Select {
                target: 0x9000,
                probe: None,
            }
        );
    }

    #[test]
    fn noop_is_callable_and_returns_unit() {
        a84_noop();
    }
}
