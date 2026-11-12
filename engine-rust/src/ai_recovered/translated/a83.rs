#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent83.rs
// ===========================================================================
// Translation batch 83.  Only this file is edited by the batch.
//
// Batch 83 owns `0x00638000..=0x00639fff`.  The ledger lists 57
// `decompiled_uncited` rows in that window (plus `FUN_00638db0`, already
// `translated`); 36 are translated below and the 21 rows that are not are
// listed at the end with their reason.
//
// The window is the client-side "controller" (order-pad) layer, not the
// simulation: the per-player controller lists, the shared node pool, the
// command-pad entry decoder, the refresh gates, the selection/command-id
// predicates and the two static ring tables that the whole layer is built on.
// Almost every row is one branch of an inlined routine that the decompiler
// split at shared labels, so the rows are translated where they contain a
// self-contained selection, arithmetic or table walk, and the frame-only
// continuations are skipped.  No `sim` record gains a field.
//
// Recovered layouts (all offsets in bytes):
//
// | layout                 | words                                                              |
// |------------------------|--------------------------------------------------------------------|
// | controller node        | `+0x00` next, `+0x04` prev, `+0x0c` active unit, `+0x14` busy unit |
// |                        | `+0x18` group, `+0x19` index, `+0x1a` stock, `+0x1b` capacity     |
// |                        | `+0x1c` icon, `+0x1d` dirty, `+0x1e` countdown                    |
// |                        | `+0x20`/`+0x22` argument words, `+0x24`/`+0x28` unit links         |
// |                        | `+0x2c`/`+0x30`/`+0x34`/`+0x38` command slots                      |
// |                        | `+0x3c` 100 four-byte command entries                              |
// | group record           | `0x00fc86a8 + group * 8`: `+0xb3b0` free-list head                  |
// | active heads           | `0x00fc86ac + group * 8`                                           |
// | per-group counters     | `(0x00f063cc ^ 0x10fae560) + group * 4`, second table at `+0x30`   |
// | group ring (`0x18`)    | `+0x00`/`+0x04` links, `+0x08..+0x17` zeroed; 998 loop records + 0 |
// | command ring (`0x2c`)  | 998 loop records + 0; `0x00fbd308` gets `0x1b4` zero bytes         |
// | rotation byte          | `0x00fb2704`, incremented and wrapped at 8                          |
//
// Address to function:
//
// * `0x006381e0` [`a83_release_gate`] (with [`a83_nearest_distance_index`],
//   [`a83_move_to_free_list`])
// * `0x006382b1` [`a83_release_gate_b`]
// * `0x006382e1` [`a83_release_gate_c`]
// * `0x006383d0` [`a83_spawn_group_node`]
// * `0x006384cf` [`a83_shortfall_units`] (with [`a83_top_up_wanted`])
// * `0x00638688` [`a83_selection_gate`]
// * `0x006387f0` [`a83_ranked_slot`]
// * `0x00638880` [`a83_unit_slot_lookup`]
// * `0x006388b7` [`a83_probe_result`]
// * `0x006388f9` [`a83_accept_unless_cancel`]
// * `0x0063890a` [`a83_accept_only_0x23`]
// * `0x00638917` [`a83_accept_only_0x24`]
// * `0x00638924` [`a83_is_target_command_0x2a`]
// * `0x006389e0` [`a83_status_message`]
// * `0x00638b6c` [`a83_ability_slot_flag`]
// * `0x00638ca0` [`a83_group_score_sum`]
// * `0x00638d30` [`a83_first_matching_target`]
// * `0x00638f10` [`a83_self_detach`]
// * `0x00638fc0` [`a83_order_queue_room`] (with [`a83_order_char_eligible`])
// * `0x006390f0` [`a83_ability_requested`] (with [`a83_ability_entry`],
//   [`a83_fill_count`], [`a83_multi_select_gate`])
// * `0x0063918c` [`a83_skip_queue_flush`]
// * `0x0063921a` [`a83_selection_bias`]
// * `0x0063925f` [`a83_funclet_selector_abort`]
// * `0x00639381` [`a83_existing_order_count`]
// * `0x006396cd` [`a83_refresh_gate_slot_0x30`]
// * `0x00639719` [`a83_refresh_gate_slot_0x34`]
// * `0x00639765` [`a83_refresh_gate_slot_0x38`]
// * `0x00639871` [`a83_force_refresh_slot_0x30`]
// * `0x006398bd` [`a83_force_refresh_slot_0x34`]
// * `0x00639911` [`a83_force_refresh_slot_0x38`]
// * `0x00639990` [`a83_table_layout_words`]
// * `0x006399b9` [`a83_table_layout_bytes`]
// * `0x00639d00` [`a83_clear_unit_references`]
// * `0x00639e10` [`a83_release_controller_link`]
// * `0x00639e60` [`a83_lookup_art_or_abort`]
// * `0x00639ed0` [`a83_controller_gate`] (with [`a83_rotate_controller`],
//   [`a83_ready_slots`])
//
// Every external callee that has no body in this window (`FUN_0063ca40`,
// `FUN_00748d40`, `FUN_005b4cd0`, `FUN_005a3180`, `FUN_005b5970`, `FUN_005b5a30`,
// `FUN_00638b00`, `FUN_00638c00`, `FUN_00638940`, `FUN_00635060`, `FUN_006350f0`,
// `FUN_00635d90`, `FUN_00635720`, `FUN_00642a00`, ...) is represented by an input
// or an outcome variant, never called.  The two rows that write the static
// rings keep the absolute addresses of their pointers so the arithmetic stays
// auditable.

// ---------------------------------------------------------------------------
// Shared constants
// ---------------------------------------------------------------------------

/// One controller list per player: `(&DAT_00fc86ac)[group * 2]`, i.e. a
/// dword stride of eight bytes, eight groups (`uVar3 < 0x40` step 8).
pub const A83_GROUP_COUNT: u32 = 8;
/// Four-byte command entries per node (`piVar13 + 0xf`, 100 iterations).
pub const A83_ENTRY_COUNT: usize = 100;
/// Byte offset of the first command entry (`piVar13 + 0xf`).
pub const A83_ENTRY_OFFSET: u32 = 0x3c;
/// The sentinel that seeds the nearest-node search at `0x006381e0`.
pub const A83_NEAREST_LIMIT: u32 = 99_999_999;
/// Free-list-head offset inside the per-group record (`46000`).
pub const A83_FREE_HEAD_OFFSET: u32 = 0xb3b0;
/// Base of the per-group records (`DAT_00fc86a8`).
pub const A83_GROUP_TABLE: u32 = 0x00fc_86a8;
/// Base of the per-group active-list heads (`DAT_00fc86ac`).
pub const A83_ACTIVE_HEADS: u32 = 0x00fc_86ac;
/// The xor operand that turns `DAT_00f063cc` into the counter table base.
pub const A83_COUNTER_XOR: u32 = 0x10fa_e560;
/// Offset of the second per-group counter table.
pub const A83_COUNTER_SECOND: u32 = 0x30;
/// First index of the group ring built by `0x00639990` (`0x18`).
pub const A83_GROUP_RING_FIRST: u32 = 0x18;
/// Stride of the group ring (`0x18`).
pub const A83_GROUP_RING_STRIDE: u32 = 0x18;
/// Exclusive limit of the group-ring loop (`0x5da8`).
pub const A83_GROUP_RING_LIMIT: u32 = 0x5da8;
/// First index of the command ring (`0x2c`).
pub const A83_COMMAND_RING_FIRST: u32 = 0x2c;
/// Stride of the command ring (`0x2c`).
pub const A83_COMMAND_RING_STRIDE: u32 = 0x2c;
/// Exclusive limit of the command-ring loop (`0xabb4`).
pub const A83_COMMAND_RING_LIMIT: u32 = 0xabb4;
/// Bytes zeroed at `0x00fbd308` after the two rings are linked (`0x1b4`).
pub const A83_TAIL_ZERO_BYTES: u32 = 0x1b4;
/// Bytes of the zeroed section in every ring record.
pub const A83_RECORD_ZERO_BYTES: u32 = 0x10;

// ---------------------------------------------------------------------------
// Controller node: `0x006383d0` record layout
// ---------------------------------------------------------------------------

/// The record initialised by `0x006383d0` and shared by every row in the
/// window.  Only the fields the rows actually read or write are modelled;
/// `+0x1e` (`countdown`) is deliberately present because `0x006383d0` does
/// *not* write it and `0x00639ed0` decrements it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A83OrderNode {
    /// `+0x00`: next node in the owning chain.
    pub next: Option<u32>,
    /// `+0x04`: previous node in the owning chain.
    pub prev: Option<u32>,
    /// `+0x18`: owning group byte (`(char)param_1`).
    pub group: u8,
    /// `+0x19`: zeroed by the `u16` store at `+0x19`.
    pub index_0x19: u8,
    /// `+0x1a`: queue depth, zeroed by the `u16` store at `+0x19`.
    pub stock: u8,
    /// `+0x1b`: queue capacity, zeroed separately.
    pub capacity: u8,
    /// `+0x1c`: icon id, zeroed by the `u16` store at `+0x1c`.
    pub icon: u8,
    /// `+0x1d`: dirty marker set by `0x00639d00`.
    pub dirty: u8,
    /// `+0x1e`: countdown, untouched by `0x006383d0`.
    pub countdown: u8,
    /// `+0x20`: first spawn word (`*(u16 *)(piVar1 + 8)`).
    pub word_0x20: u16,
    /// `+0x22`: second spawn word.
    pub word_0x22: u16,
    /// `+0x24`: first attached-unit link.
    pub link_0x24: u32,
    /// `+0x28`: second attached-unit link.
    pub link_0x28: u32,
    /// `+0x2c`: command slot 3.
    pub slot_0x2c: u32,
    /// `+0x30`: command slot 4.
    pub slot_0x30: u32,
    /// `+0x34`: command slot 5.
    pub slot_0x34: u32,
    /// `+0x38`: command slot 6.
    pub slot_0x38: u32,
    /// `+0x3c`: 100 command entries (`piVar1 + 0xf`, 400 bytes).
    pub entries_0x3c: [u32; A83_ENTRY_COUNT],
}

impl Default for A83OrderNode {
    fn default() -> Self {
        A83OrderNode {
            next: None,
            prev: None,
            group: 0,
            index_0x19: 0,
            stock: 0,
            capacity: 0,
            icon: 0,
            dirty: 0,
            countdown: 0,
            word_0x20: 0,
            word_0x22: 0,
            link_0x24: 0,
            link_0x28: 0,
            slot_0x2c: 0,
            slot_0x30: 0,
            slot_0x34: 0,
            slot_0x38: 0,
            entries_0x3c: [0; A83_ENTRY_COUNT],
        }
    }
}

impl A83OrderNode {
    /// `0x006383d0`: write the fields the row writes.
    ///
    /// ```c
    /// *(char *)(piVar1 + 6) = (char)param_1;                    // +0x18
    /// *(undefined2 *)(piVar1 + 8) = param_2;                    // +0x20
    /// *(undefined2 *)((int)piVar1 + 0x19) = 0;                  // +0x19/+0x1a
    /// *(undefined2 *)((int)piVar1 + 0x22) = param_3;            // +0x22
    /// piVar1[0xb] = piVar1[0xc] = piVar1[0xd] = piVar1[0xe] = 0; // +0x2c..+0x38
    /// piVar1[10] = 0; piVar1[9] = 0;                             // +0x24/+0x28
    /// *(undefined2 *)(piVar1 + 7) = 0;                           // +0x1c/+0x1d
    /// *(undefined1 *)((int)piVar1 + 0x1b) = 0;                   // +0x1b
    /// FUN_00c8a080(piVar1 + 0xf, 0, 400);                        // +0x3c
    /// ```
    ///
    /// `+0x1e` is not in that list, so a pooled node keeps its countdown.
    pub fn a83_initialize(&mut self, group: u8, word_0x20: u16, word_0x22: u16) {
        self.group = group;
        self.word_0x20 = word_0x20;
        self.index_0x19 = 0;
        self.stock = 0;
        self.word_0x22 = word_0x22;
        self.slot_0x2c = 0;
        self.slot_0x30 = 0;
        self.slot_0x34 = 0;
        self.slot_0x38 = 0;
        self.link_0x24 = 0;
        self.link_0x28 = 0;
        self.icon = 0;
        self.dirty = 0;
        self.capacity = 0;
        self.entries_0x3c = [0; A83_ENTRY_COUNT];
    }

    /// `0x00639d00`: clear the four command slots that point at `target` and
    /// mark the node dirty when it has an icon.
    ///
    /// ```c
    /// if (puVar1[0xb] == param_1) puVar1[0xb] = 0;   // +0x2c
    /// if (puVar1[0xc] == param_1) puVar1[0xc] = 0;   // +0x30
    /// if (puVar1[0xd] == param_1) puVar1[0xd] = 0;   // +0x34
    /// if (puVar1[0xe] == param_1) puVar1[0xe] = 0;   // +0x38
    /// if (*(char *)(puVar1 + 7) != '\0')
    ///     *(undefined1 *)((int)puVar1 + 0x1d) = 1;   // icon != 0 -> dirty
    /// ```
    ///
    /// Returns the number of slots that were cleared (0..=4).
    pub fn a83_clear_reference(&mut self, target: u32) -> u32 {
        let mut cleared = 0;
        if self.slot_0x2c == target {
            self.slot_0x2c = 0;
            cleared += 1;
        }
        if self.slot_0x30 == target {
            self.slot_0x30 = 0;
            cleared += 1;
        }
        if self.slot_0x34 == target {
            self.slot_0x34 = 0;
            cleared += 1;
        }
        if self.slot_0x38 == target {
            self.slot_0x38 = 0;
            cleared += 1;
        }
        if self.icon != 0 {
            self.dirty = 1;
        }
        cleared
    }

    /// `0x00639ed0`: decrement the countdown when it is nonzero.
    ///
    /// ```c
    /// if (*(char *)((int)piVar4 + 0x1e) != '\0') {
    ///     *(char *)((int)piVar4 + 0x1e) =
    ///         *(char *)((int)piVar4 + 0x1e) + -1;
    /// }
    /// ```
    pub fn a83_tick_countdown(&mut self) {
        if self.countdown != 0 {
            self.countdown = self.countdown.wrapping_sub(1);
        }
    }
}

// ---------------------------------------------------------------------------
// 0x006381e0 / 0x006382b1 / 0x006382e1 - controller release control flow
// ---------------------------------------------------------------------------

/// What `0x006381e0` does before it reaches the shared publish tail.
///
/// The row first guards on the global re-entry byte `DAT_00fc86e8`, then
/// asserts that `+0x14` is clear, and only then looks at the attached unit at
/// `+0x0c`.  When `+0x0c` is nonzero it searches the group chain for the
/// nearest other node; both remaining assertions then run `FUN_0063ca40`
/// with the word at `+0x10` of the object they name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A83Release {
    /// `DAT_00fc86e8 != 0`: return immediately.
    Reentrant,
    /// A nonzero `+0x14` (or, in `0x006382b1`/`0x006382e1`, the qualified
    /// `+0x0c`): the row's non-returning assertion.
    Busy,
    /// `+0x0c == 0`: nothing to search, walk into the publish tail.
    Publish,
    /// `+0x0c != 0` and no candidate is closer than [`A83_NEAREST_LIMIT`].
    NoCandidate,
    /// `0x006381e0`: `+0x0c != 0` and a candidate exists, so
    /// `FUN_0063ca40(*(+0x0c + 0x10), 0)` runs with the attached object's
    /// `+0x10` word.
    Abort(u32),
}

/// `0x006381e0`: the nearest-node search that precedes the release.
///
/// ```c
/// uStack_c = 99999999; piStack_8 = 0;
/// piVar5 = (&DAT_00fc86ac)[(uint)*(byte *)(param_1 + 6) * 2];
/// if (piVar5 == 0) return;
/// do {
///     if ((piVar5 != param_1) &&
///         (uVar1 = FUN_00604060(...), piVar4 = piStack_8, uVar1 < uStack_c)) {
///         piVar4 = piVar5;
///         uStack_c = uVar1;
///         piStack_8 = piVar5;
///     }
///     piVar5 = (int *)*piVar5;
/// } while (piVar5 != 0);
/// ```
///
/// `FUN_00604060` is the distance function and stays outside the row; the
/// selection itself is the strict `<` comparison against the `99_999_999`
/// seed, with the first of equal distances winning because later equal
/// distances are not `<`.
pub fn a83_nearest_distance_index(distances: &[u32], skip: Option<usize>) -> Option<usize> {
    let mut best = A83_NEAREST_LIMIT;
    let mut found = None;
    for (index, &distance) in distances.iter().enumerate() {
        if skip == Some(index) {
            continue;
        }
        if distance < best {
            best = distance;
            found = Some(index);
        }
    }
    found
}

/// `0x006381e0`: fold the row's guards and the search result into one outcome.
///
/// `word_0x14`/`word_0x0c` are the node's `+0x14` and `+0x0c` words;
/// `active_id` is the `+0x10` word of the object at `+0x0c`, i.e. the first
/// argument of the second `FUN_0063ca40` call in the row.  The first
/// `FUN_0063ca40` argument (`+0x14`'s `+0x10`) is unreachable because the
/// `+0x14 != 0` case was already rejected by `FUN_005b4cd0`.
pub fn a83_release_gate(
    reentrant: bool,
    word_0x14: u32,
    word_0x0c: u32,
    active_id: u32,
    nearest: Option<usize>,
) -> A83Release {
    if reentrant {
        return A83Release::Reentrant;
    }
    if word_0x14 != 0 {
        return A83Release::Busy;
    }
    if word_0x0c == 0 {
        return A83Release::Publish;
    }
    match nearest {
        None => A83Release::NoCandidate,
        Some(_) => A83Release::Abort(active_id),
    }
}

/// `0x006382b1`: the continuation after the search, with its two guards.
///
/// ```c
/// if ((unaff_ESI[5] != 0) && (unaff_EBX != 0))  FUN_0063ca40();  // +0x14
/// if ((unaff_ESI[3] != 0) && (unaff_ESI[3] != 0)) FUN_0063ca40(); // +0x0c
/// ```
///
/// The second test's two operands are the same word, so a nonzero `+0x0c`
/// always takes the abort path; the publish tail is unconditional otherwise.
/// This row's `FUN_0063ca40()` calls pass no recovered argument.
pub fn a83_release_gate_b(word_0x14: u32, ebx_nonzero: bool, word_0x0c: u32) -> A83Release {
    if word_0x14 != 0 && ebx_nonzero {
        return A83Release::Busy;
    }
    if word_0x0c != 0 {
        return A83Release::Busy;
    }
    A83Release::Publish
}

/// `0x006382e1`: the same continuation with only the `+0x0c` guard, which is
/// additionally qualified by `EBX`.
///
/// ```c
/// if ((unaff_ESI[3] != 0) && (unaff_EBX != 0)) FUN_0063ca40();
/// ```
pub fn a83_release_gate_c(word_0x0c: u32, ebx_nonzero: bool) -> A83Release {
    if word_0x0c != 0 && ebx_nonzero {
        return A83Release::Busy;
    }
    A83Release::Publish
}

/// `0x006381e0`/`0x006382b1`/`0x006382e1`: move `this` from the active chain
/// to the front of the free chain.
///
/// ```c
/// if ((int *)(&DAT_00fc86ac)[uVar1 * 2] == param_1)
///     (&DAT_00fc86ac)[uVar1 * 2] = *param_1;
/// if ((int *)param_1[1] != 0) *(int *)param_1[1] = *param_1;
/// if (*param_1 != 0) *(int *)(*param_1 + 4) = param_1[1];
/// iVar2 = *(int *)(&DAT_00fc86a8 + uVar1 * 8);
/// param_1[1] = 0;
/// *param_1 = *(int *)(iVar2 + 46000);
/// if (*(int *)(iVar2 + 46000) != 0)
///     *(int **)(*(int *)(iVar2 + 46000) + 4) = param_1;
/// *(int **)(iVar2 + 46000) = param_1;
/// ```
///
/// The order matters: the head update happens before the `prev`/`next`
/// rewiring, so a head node's `prev` is already `None` when the rewiring runs.
pub fn a83_move_to_free_list(
    active_head: &mut Option<u32>,
    free_head: &mut Option<u32>,
    nodes: &mut [A83OrderNode],
    this: u32,
) {
    let index = this as usize;
    let next = nodes[index].next;
    let prev = nodes[index].prev;
    if *active_head == Some(this) {
        *active_head = next;
    }
    if let Some(prev) = prev {
        nodes[prev as usize].next = next;
    }
    if let Some(next) = next {
        nodes[next as usize].prev = prev;
    }
    nodes[index].prev = None;
    nodes[index].next = *free_head;
    if let Some(head) = *free_head {
        nodes[head as usize].prev = Some(this);
    }
    *free_head = Some(this);
}

// ---------------------------------------------------------------------------
// 0x006383d0 - spawn a pooled node into the group's active chain
// ---------------------------------------------------------------------------

/// `0x006383d0`: pop the free head and push it onto the group's active chain,
/// then run [`A83OrderNode::a83_initialize`].
///
/// ```c
/// piVar1 = *(int **)(*(int *)(&DAT_00fc86a8 + param_1 * 8) + 46000);
/// if (piVar1 != 0) {
///     iVar2 = *piVar1;
///     *(int *)(head + 46000) = iVar2;
///     if (iVar2 != 0) *(undefined4 *)(iVar2 + 4) = 0;
///     piVar1[1] = 0;
///     *piVar1 = (&DAT_00fc86ac)[param_1 * 2];
///     if ((&DAT_00fc86ac)[param_1 * 2] != 0)
///         *(int **)((&DAT_00fc86ac)[param_1 * 2] + 4) = piVar1;
///     (&DAT_00fc86ac)[param_1 * 2] = piVar1;
///     ... field stores ...
/// }
/// return 0;
/// ```
///
/// Returns the popped node's index, or `None` when the free chain is empty.
pub fn a83_spawn_group_node(
    free_head: &mut Option<u32>,
    active_head: &mut Option<u32>,
    nodes: &mut [A83OrderNode],
    group: u8,
    word_0x20: u16,
    word_0x22: u16,
) -> Option<u32> {
    let this = (*free_head)?;
    let index = this as usize;
    assert!(
        index < nodes.len(),
        "0x006383d0: free-list head points past the node pool"
    );
    let next = nodes[index].next;
    *free_head = next;
    if let Some(next) = next {
        nodes[next as usize].prev = None;
    }
    nodes[index].prev = None;
    nodes[index].next = *active_head;
    if let Some(head) = *active_head {
        nodes[head as usize].prev = Some(this);
    }
    *active_head = Some(this);
    nodes[index].a83_initialize(group, word_0x20, word_0x22);
    Some(this)
}

// ---------------------------------------------------------------------------
// 0x006384cf - queue shortfall arithmetic
// ---------------------------------------------------------------------------

/// `0x006384cf`: the "how many more units are needed" division.
///
/// ```c
/// if (0xc < iVar3) return;
/// uVar1 = FUN_00695df0(0x6d);
/// iVar3 = FUN_00695ae0(uVar1, *(undefined1 *)(unaff_ESI + 0x18));
/// iVar4 = FUN_00638940(*(undefined1 *)(unaff_ESI + 0x18));
/// if (iVar3 + 0xdU <= (uint)(byte)(&DAT_00f920d0)[iVar4]) return;
/// uVar5 = ((iVar3 + 0xdU) - (uint)(byte)(&DAT_00f920d0)[iVar4]) /
///        (uint)DAT_00f9213d;
/// ```
///
/// `base` is the recovered `iVar3`, `table_value` the byte from the
/// `0x00f920d0` row and `divisor` the `DAT_00f9213d` byte.  The subtractions
/// are unsigned 32-bit and the division truncates.  A divisor of zero is the
/// row's own divide fault, reported as `None`.
pub fn a83_shortfall_units(base: i32, table_value: u8, divisor: u8) -> Option<u32> {
    if base > 0xc {
        return None;
    }
    let available = base.wrapping_add(0xd) as u32;
    if available <= table_value as u32 {
        return None;
    }
    if divisor == 0 {
        return None;
    }
    Some((available - table_value as u32) / divisor as u32)
}

/// `0x006384cf`: the top-up decision that consumes the shortfall.
///
/// ```c
/// if ((((*(int *)(unaff_EBP + 8) == 0) ||
///       (iVar3 = FUN_006358d0(...), iVar3 == 0)) ||
///      (iVar3 = FUN_00637b70(...), iVar3 != 0)) &&
///     (uVar6 = FUN_0063b450(..., 0x6d), uVar6 < uVar5 + 1)) {
///     FUN_00635060(...);
/// }
/// ```
///
/// `count_zero` is the `+8` word being zero, `has_source` the
/// `FUN_006358d0` result and `is_transport` the `FUN_00637b70` result; `have`
/// is `FUN_0063b450`.  The final addition is 32-bit wrapping.
pub fn a83_top_up_wanted(
    count_zero: bool,
    has_source: bool,
    is_transport: bool,
    have: u32,
    shortfall: u32,
) -> bool {
    let wants = count_zero || !has_source || is_transport;
    wants && have < shortfall.wrapping_add(1)
}

// ---------------------------------------------------------------------------
// 0x00638688 - the two-bit selection gate
// ---------------------------------------------------------------------------

/// `0x00638688`: choose between the reject call (`FUN_0065b910(0x19)`) and
/// the accept path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A83Gate {
    /// The row calls `FUN_0065b910(0x19)`.
    Reject,
    /// The row falls into the accept path.
    Proceed,
}

/// `0x00638688`: the bit tests on the flags byte and the query word.
///
/// ```c
/// if ((*(byte *)(unaff_EBP + 0x13) & 2) == 0) {
///     if ((*(byte *)(unaff_EBP + 0x13) & 1) != 0) {
///         bVar1 = (&DAT_00f91fe8)[in_EAX & 0xffff] & 1;  goto LAB_0063869e;
///     }
///     if (((&DAT_00f91fe8)[in_EAX & 0xffff] & 4) != 0) goto LAB_006386a4;
/// LAB_00638741: FUN_0065b910(0x19);
/// } else {
///     bVar1 = (&DAT_00f91fe8)[in_EAX & 0xffff] & 2;
/// LAB_0063869e: if (bVar1 == 0) goto LAB_00638741;
/// LAB_006386a4: ...accept...
/// }
/// ```
///
/// `query` is the caller-frame byte at `+0x13`, `flags` the byte from the
/// `0x00f91fe8` table.  Bit 1 of `query` wins over bit 0; bit 0 needs `flags &
/// 1`, the no-bit case needs `flags & 4`, and the bit-1 case needs `flags & 2`.
pub fn a83_selection_gate(flags: u8, query: u8) -> A83Gate {
    if query & 2 == 0 {
        if query & 1 != 0 {
            if flags & 1 == 0 {
                return A83Gate::Reject;
            }
            return A83Gate::Proceed;
        }
        if flags & 4 != 0 {
            return A83Gate::Proceed;
        }
        return A83Gate::Reject;
    }
    if flags & 2 == 0 {
        return A83Gate::Reject;
    }
    A83Gate::Proceed
}

// ---------------------------------------------------------------------------
// 0x006387f0 / 0x00638880 - ranked command slots
// ---------------------------------------------------------------------------

/// `0x00638880`: the row returns 0 after asserting that a nonzero slot still
/// resolves.
///
/// ```c
/// if (param_2 != 0) {
///     iVar1 = FUN_005a3180();
///     if (iVar1 == 0) FUN_005b4cd0();       // assertion
/// }
/// return 0;
/// ```
///
/// The decompiled body reports `0`, which is why the callers' comparisons with
/// `1` and `3` are kept as a resolver input in [`a83_ranked_slot`] instead of
/// being folded to constants.
pub fn a83_unit_slot_lookup(has_slot: bool, slot_valid: bool) -> Result<u32, ()> {
    if has_slot && !slot_valid {
        return Err(());
    }
    Ok(0)
}

/// `0x006387f0`: return the slot with the requested rank.
///
/// ```c
/// iVar2 = 0;
/// if (*(int *)(param_1 + 0x30) != 0) {
///     iVar1 = FUN_00638880(...);
///     if ((iVar1 == param_2) && (iVar2 = 1, param_3 == 1))
///         return *(undefined4 *)(param_1 + 0x30);
/// }
/// if (*(int *)(param_1 + 0x34) != 0) {
///     iVar1 = FUN_00638880(...);
///     if ((iVar1 == param_2) && (iVar2 = iVar2 + 1, iVar2 == param_3))
///         return *(undefined4 *)(param_1 + 0x34);
/// }
/// if (*(int *)(param_1 + 0x38) != 0) {
///     iVar1 = FUN_00638880(...);
///     if ((iVar1 == param_2) && (iVar2 + 1 == param_3))
///         return *(undefined4 *)(param_1 + 0x38);
/// }
/// return 0;
/// ```
///
/// The first block only bumps the rank counter when the value matches; the
/// third block compares without bumping.  That asymmetry is reproduced, so a
/// null `+0x30` slot with `rank == 1` can return the `+0x38` slot.
pub fn a83_ranked_slot<F>(
    slot_0x30: u32,
    slot_0x34: u32,
    slot_0x38: u32,
    match_value: u32,
    rank: i32,
    group: u8,
    resolve: F,
) -> u32
where
    F: Fn(u8, u32) -> u32,
{
    let mut seen = 0i32;
    if slot_0x30 != 0 {
        if resolve(group, slot_0x30) == match_value {
            seen = 1;
            if rank == 1 {
                return slot_0x30;
            }
        }
    }
    if slot_0x34 != 0 {
        if resolve(group, slot_0x34) == match_value {
            seen += 1;
            if seen == rank {
                return slot_0x34;
            }
        }
    }
    if slot_0x38 != 0 && resolve(group, slot_0x38) == match_value && seen + 1 == rank {
        return slot_0x38;
    }
    0
}

// ---------------------------------------------------------------------------
// 0x006388b7 / 0x006388f9 / 0x0063890a / 0x00638917 / 0x00638924
// ---------------------------------------------------------------------------

/// `0x006388b7`: `3 - (FUN_005b5a30() != 0)` when the group matches, else 0.
///
/// ```c
/// uVar1 = FUN_00638c00(*(undefined4 *)(unaff_EBP + 8));
/// if ((in_EAX & 0xffff) == uVar1) {
///     iVar2 = FUN_005b5a30();
///     return 3 - (uint)(iVar2 != 0);
/// }
/// return 0;
/// ```
pub fn a83_probe_result(command_matches: bool, peer_present: bool) -> i32 {
    if !command_matches {
        return 0;
    }
    if peer_present { 2 } else { 3 }
}

/// `0x006388f9`: accept everything except the `0xbc` command.
///
/// ```c
/// if (in_AX == 0xbc) FUN_005b4cd0();   // not returning
/// return 1;
/// ```
pub fn a83_accept_unless_cancel(ax: i16) -> Option<u32> {
    if ax == 0xbc { None } else { Some(1) }
}

/// `0x0063890a`: only `0x23` passes the assertion.
pub fn a83_accept_only_0x23(ax: i16) -> Option<u32> {
    if ax == 0x23 { Some(1) } else { None }
}

/// `0x00638917`: only `0x24` passes the assertion.
pub fn a83_accept_only_0x24(ax: i16) -> Option<u32> {
    if ax == 0x24 { Some(1) } else { None }
}

/// `0x00638924`: the `0x2a` command predicate (returns 0 instead of aborting).
pub fn a83_is_target_command_0x2a(ax: i16) -> u32 {
    if ax == 0x2a { 1 } else { 0 }
}

// ---------------------------------------------------------------------------
// 0x006389e0 - status-message selection
// ---------------------------------------------------------------------------

/// Which call `0x006389e0` performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A83Notice {
    /// `FUN_005b7d00(0x5a)`.
    First,
    /// `FUN_005b7d00(0x54)`.
    Second,
    /// Neither status flag is set and both trailing arguments are zero.
    Nothing,
    /// One of the trailing arguments is nonzero: `FUN_005b4cd0`.
    Assert,
}

/// `0x006389e0`: pick the notice id.
///
/// ```c
/// iVar1 = FUN_005b5630();
/// if (iVar1 != 0) { FUN_005b7d00(0x5a); return; }
/// iVar1 = FUN_005b5620();
/// if (iVar1 != 0) { FUN_005b7d00(0x54); return; }
/// if (param_2 != 0) FUN_005b4cd0();
/// if (param_3 != 0) FUN_005b4cd0();
/// ```
pub fn a83_status_message(first: bool, second: bool, param_2: u32, param_3: u32) -> A83Notice {
    if first {
        return A83Notice::First;
    }
    if second {
        return A83Notice::Second;
    }
    if param_2 != 0 || param_3 != 0 {
        return A83Notice::Assert;
    }
    A83Notice::Nothing
}

// ---------------------------------------------------------------------------
// 0x00638b6c - command-slot byte address and its 0x29/0x40 value
// ---------------------------------------------------------------------------

/// The address calculation and value selection of `0x00638b6c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A83AbilitySlot {
    /// Byte offset inside the caller's controller block.
    pub byte_offset: i32,
    /// `0x40` when the addressed byte is nonzero, `0x29` when it is zero.
    pub limit: u32,
}

/// `0x00638b6c`: build the byte address and pick the limit.
///
/// ```c
/// if (in_EAX != 0) unaff_ESI = (unaff_EBX + 0xc) * 0x24;
/// uVar1 = 0x40;
/// if (*(char *)(unaff_ESI + 9 + unaff_EDI) == '\0') uVar1 = 0x29;
/// return uVar1;
/// ```
///
/// `first_path` is `in_EAX != 0`, `base` the `EBX + 0xc` operand, `fallback`
/// the already-scaled `ESI` and `index` the `EDI` byte index.  All arithmetic
/// is wrapping 32-bit and the result is returned as the row's signed `i32`.
pub fn a83_ability_slot_flag(
    first_path: bool,
    base: i32,
    fallback: i32,
    index: i32,
    byte_is_zero: bool,
) -> A83AbilitySlot {
    let scaled = if first_path {
        base.wrapping_add(0xc).wrapping_mul(0x24)
    } else {
        fallback
    };
    A83AbilitySlot {
        byte_offset: scaled.wrapping_add(9).wrapping_add(index),
        limit: if byte_is_zero { 0x29 } else { 0x40 },
    }
}

// ---------------------------------------------------------------------------
// 0x00638ca0 - group signature sum
// ---------------------------------------------------------------------------

/// `0x00638ca0`: sum `FUN_0076a2e0(icon)` over every node with an icon.
///
/// ```c
/// iVar3 = 0;
/// for (puVar1 = (&DAT_00fc86ac)[param_1 * 2]; puVar1 != 0; puVar1 = *puVar1) {
///     if (*(char *)(puVar1 + 7) != '\0') {
///         iVar2 = FUN_0076a2e0(*(char *)(puVar1 + 7));
///         iVar3 = iVar3 + iVar2;
///     }
/// }
/// return iVar3;
/// ```
///
/// The caller supplies `(icon byte, FUN_0076a2e0 result)` pairs; a zero icon
/// is skipped.  The sum wraps like the 32-bit add in the row.
pub fn a83_group_score_sum<I>(nodes: I) -> i32
where
    I: IntoIterator<Item = (u8, i32)>,
{
    let mut sum = 0i32;
    for (icon, value) in nodes {
        if icon != 0 {
            sum = sum.wrapping_add(value);
        }
    }
    sum
}

// ---------------------------------------------------------------------------
// 0x00638d30 - first matching target
// ---------------------------------------------------------------------------

/// One entry of the null-terminated target array scanned by `0x00638d30`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A83TargetEntry {
    /// Byte at `+0x4c`.
    pub word_0x4c: u8,
    /// `FUN_005b64c0() != 0`.
    pub primary_match: bool,
    /// `FUN_005b5820() != 0`.
    pub secondary_match: bool,
}

/// `0x00638d30`: report whether any entry matches the owner byte and one of
/// the two predicates.
///
/// ```c
/// local_8 = 0;
/// piVar1 = (int *)FUN_005a5790(param_2);
/// iVar2 = *piVar1;
/// piVar3 = piVar1;
/// do {
///     if (iVar2 == 0) { FUN_005a5770(piVar1); return local_8; }
///     if ((*(byte *)(iVar2 + 0x4c) == param_1) &&
///        ((iVar2 = FUN_005b64c0(), iVar2 != 0 ||
///         (iVar2 = FUN_005b5820(), iVar2 != 0)))) {
///         local_8 = 1;
///         goto return;
///     }
///     iVar2 = piVar3[1];
///     piVar3 = piVar3 + 1;
/// } while (true);
/// ```
///
/// The second predicate is only evaluated when the first one is false, exactly
/// like `primary_match || secondary_match`.
pub fn a83_first_matching_target(entries: &[A83TargetEntry], word_0x4c: u8) -> bool {
    entries
        .iter()
        .any(|entry| entry.word_0x4c == word_0x4c && (entry.primary_match || entry.secondary_match))
}

// ---------------------------------------------------------------------------
// 0x00638f10 - self detach sequence
// ---------------------------------------------------------------------------

/// The two calls `0x00638f10` orders when the target is the unit itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A83SelfDetach {
    /// Run the `0x00639d00` release first.
    pub release_self_first: bool,
    /// The `+0x14` word of the attached record, passed to the second call.
    pub link_0x14: u32,
}

/// `0x00638f10`: detach a unit from its controller.
///
/// ```c
/// if (*(int *)(param_1 + 0x134) != 0) {
///     uVar1 = *(undefined4 *)(*(int *)(param_1 + 0x134) + 0x14);
///     if (param_1 == param_2) FUN_00639d00(param_2, 1);
///     FUN_00637810(uVar1, param_2);
/// }
/// ```
///
/// `attached_0x134` is the record pointer, `link_0x14` its `+0x14` word and
/// `same_unit` the `param_1 == param_2` test.
pub fn a83_self_detach(
    attached_0x134: u32,
    link_0x14: u32,
    same_unit: bool,
) -> Option<A83SelfDetach> {
    if attached_0x134 == 0 {
        return None;
    }
    Some(A83SelfDetach {
        release_self_first: same_unit,
        link_0x14,
    })
}

// ---------------------------------------------------------------------------
// 0x00638fc0 - queue room and order eligibility
// ---------------------------------------------------------------------------

/// `0x00638fc0`: the requested count after the stock/capacity clamp.
///
/// ```c
/// if (*(char *)((int)param_1 + 0x1e) != '\0') return;
/// bVar1 = *(byte *)((int)param_1 + 0x1a);          // stock
/// if (bVar1 != 0) {
///     bVar2 = *(byte *)((int)param_1 + 0x1b);      // capacity
///     if (bVar2 <= bVar1) return;
///     if ((uint)bVar2 < (uint)bVar1 + param_2)
///         param_2 = (uint)bVar2 - (uint)bVar1;
/// }
/// if (param_2 != 0) { ... }
/// ```
///
/// A countdown byte at `+0x1e` suppresses the whole row.  A zero stock skips
/// the clamp entirely, so a request larger than the capacity still survives
/// that case.
pub fn a83_order_queue_room(
    countdown_0x1e: u8,
    stock_0x1a: u8,
    capacity_0x1b: u8,
    requested: i32,
) -> i32 {
    if countdown_0x1e != 0 {
        return 0;
    }
    let mut requested = requested;
    if stock_0x1a != 0 {
        if capacity_0x1b <= stock_0x1a {
            return 0;
        }
        if (capacity_0x1b as u32) < (stock_0x1a as u32).wrapping_add(requested as u32) {
            requested = capacity_0x1b as i32 - stock_0x1a as i32;
        }
    }
    requested
}

/// `0x00638fc0`: the order-id filter inside the queue walk.
///
/// ```c
/// cVar3 = *(char *)(iVar7 + 0x4d);
/// if ((((cVar3 != 'S') && (((cVar3 != 'W' && (cVar3 != 'X')) &&
///      (cVar3 != '\x1e')))) && (((cVar3 != '!' && (cVar3 != '\x1f')) &&
///      (cVar3 != 'F')))) && (((cVar3 != '\x1a' && (cVar3 != '\x19')) &&
///      (iVar8 = FUN_005b5970(), iVar8 != 0)))) { ...release... }
/// ```
///
/// `undiscovered` is the `FUN_005b5970() != 0` test.  The nine excluded bytes
/// are `S`, `W`, `X`, `0x1e`, `!`, `0x1f`, `F`, `0x1a` and `0x19`.
pub fn a83_order_char_eligible(order_char: u8, undiscovered: bool) -> bool {
    if matches!(
        order_char,
        b'S' | b'W' | b'X' | 0x1e | b'!' | 0x1f | b'F' | 0x1a | 0x19
    ) {
        return false;
    }
    undiscovered
}

// ---------------------------------------------------------------------------
// 0x006390f0 - command-entry decode and multi-select gate
// ---------------------------------------------------------------------------

/// One four-byte command entry at `node + 0x3c`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A83AbilityEntry {
    /// `flag >= 8`, the loop's `7 < bVar1` test.
    pub present: bool,
    /// `(flag >> 1) & 3`, the third `FUN_006350f0` argument.
    pub kind: u8,
    /// `flag >> 3`, the raw requested count.
    pub requested: u32,
    /// `*(u16 *)(entry + 2)`, little-endian.
    pub id: u16,
    /// `entry[1]`, the per-order amount.
    pub amount: u8,
}

/// `0x006390f0`: decode one entry.
///
/// ```c
/// while (bVar1 = *pbVar11, 7 < bVar1) {
///     uVar2 = *(ushort *)(pbVar11 + 2);
///     ...
///     FUN_006350f0((char)piVar13[6], piVar13, *pbVar11 >> 1 & 3,
///                  *(undefined2 *)(pbVar11 + 2), uVar14);
///     pbVar11 = pbVar11 + 4;
/// }
/// ```
///
/// The walk is bounded by 100 iterations ([`A83_ENTRY_COUNT`]) and advances by
/// four bytes; an entry below `8` ends the walk.
pub fn a83_ability_entry(raw: [u8; 4]) -> A83AbilityEntry {
    let flag = raw[0];
    A83AbilityEntry {
        present: flag >= 8,
        kind: (flag >> 1) & 3,
        requested: (flag >> 3) as u32,
        id: u16::from_le_bytes([raw[2], raw[3]]),
        amount: raw[1],
    }
}

/// `0x006390f0`: apply the capacity clamp to the entry's requested count.
///
/// ```c
/// uVar12 = (uint)(*pbVar11 >> 3);
/// uVar9 = FUN_00638b00(param_1);
/// if ((*(ushort *)(pbVar11 + 2) == uVar9) &&
///     (*(byte *)((int)piVar13 + 0x1b) < uVar12)) {
///     uVar12 = (uint)*(byte *)((int)piVar13 + 0x1b);
/// }
/// ```
///
/// `id_matches_group_flag` stands for `*(u16 *)(entry + 2) ==
/// FUN_00638b00(group)`, which is the row's call into the non-returning
/// `FUN_00638b00`.
pub fn a83_ability_requested(flag_0: u8, capacity_0x1b: u8, id_matches_group_flag: bool) -> u32 {
    let requested = (flag_0 >> 3) as u32;
    if id_matches_group_flag && (capacity_0x1b as u32) < requested {
        capacity_0x1b as u32
    } else {
        requested
    }
}

/// The fill loop shared by every `0x006390f0`-family row.
///
/// ```c
/// if (uVar14 < uVar12) {
///     for (iVar8 = uVar12 - uVar14; iVar8 != 0; iVar8 = iVar8 + -1) { ... }
/// }
/// ```
///
/// `limit` is the clamped request, `existing` the number already issued.
pub fn a83_fill_count(limit: u32, existing: u32) -> u32 {
    if limit <= existing {
        0
    } else {
        limit - existing
    }
}

/// `0x006390f0`: the multi-selection branch that runs after the entry walk.
///
/// ```c
/// if (((1 < *(byte *)((int)piVar13 + 0x1a)) ||
///     (((*(byte *)((int)piVar13 + 0x1a) == 1 && (piVar13[9] != 0)) &&
///      (iVar8 = FUN_00638c00((char)piVar13[6]), iVar8 == 0x95)))) &&
///    ((FUN_00635600((char)piVar13[6], &uStack_6, &cStack_5), cStack_5 != '\0' ||
///     (piVar13[0xb] == 0)))) { ...validate +0x30/+0x34/+0x38 twice... }
/// ```
///
/// `stock_0x1a` is `+0x1a`, `link_0x24` is `piVar13[9]`, `group_is_0x95` the
/// `FUN_00638c00` result, `second_flag` the `cStack_5` output and `word_0x2c`
/// `piVar13[0xb]`.
pub fn a83_multi_select_gate(
    stock_0x1a: u8,
    link_0x24: u32,
    group_is_0x95: bool,
    second_flag: bool,
    word_0x2c: u32,
) -> bool {
    let multi = stock_0x1a > 1 || (stock_0x1a == 1 && link_0x24 != 0 && group_is_0x95);
    multi && (second_flag || word_0x2c == 0)
}

// ---------------------------------------------------------------------------
// 0x0063918c / 0x0063921a / 0x0063925f / 0x00639381
// ---------------------------------------------------------------------------

/// `0x0063918c`: `0x83` skips the one-node queue flush.
///
/// ```c
/// if (in_AX == 0x83) { piVar12 = ...; goto LAB_006391ba; }
/// LAB_00639196:
/// piVar12 = ...;
/// FUN_00638fc0(piVar12, 1);
/// LAB_006391ba: ...
/// ```
pub fn a83_skip_queue_flush(ax: i16) -> bool {
    ax == 0x83
}

/// `0x0063921a`: add either one or the caller's lookup result.
///
/// ```c
/// if ((in_EAX & 0xffff) == uVar10) iVar4 = 1;
/// else { iVar4 = FUN_00590400(uVar10); uVar10 = ...; }
/// uVar13 = unaff_EDI + iVar4;
/// ```
pub fn a83_selection_bias(matches: bool, lookup_value: u32, accumulated: u32) -> u32 {
    let add = if matches { 1 } else { lookup_value };
    accumulated.wrapping_add(add)
}

/// `0x0063925f`: the jump-table selectors that abort.
///
/// ```c
/// switch (*(undefined4 *)(unaff_EBP + -0x20)) {
/// case 0:  FUN_005b4cd0();
/// case 1:  FUN_005b4cd0();
/// case 10: FUN_005b4cd0();
/// case 0xc: FUN_005b4cd0();
/// }
/// ```
///
/// The selector is the `uVar10 - 0x83` abort code built by the preceding
/// funclets; only those four values are handled by the table.
pub fn a83_funclet_selector_abort(selector: i32) -> bool {
    matches!(selector, 0 | 1 | 10 | 0xc)
}

/// `0x0063925f`: the abort code stored before the dispatch (`uVar10 - 0x83`).
pub fn a83_funclet_abort_code(local_0x10: u32) -> u32 {
    local_0x10.wrapping_sub(0x83)
}

/// `0x00639381`: the pre-count that seeds the entry fill.
///
/// ```c
/// uVar11 = in_EAX & 0xff;
/// iVar4 = func_0x005b0fb0(*(undefined1 *)(unaff_ESI + 0x18));
/// while (iVar4 != 0) {
///     iVar5 = FUN_005a3180();
///     if (((iVar5 == 0) && (*(char *)(iVar4 + 0x4d) == 'L')) &&
///        (uVar6 = FUN_005b5380(), (uVar6 & 0xff) == unaff_EBX)) {
///         uVar11 = uVar11 + 1;
///     }
///     iVar4 = FUN_005b4bb0();
/// }
/// ```
///
/// `base_byte` is `in_EAX & 0xff` and `matching_orders` the number of `L`
/// orders whose low id byte equals `EBX`.
pub fn a83_existing_order_count(base_byte: u32, matching_orders: u32) -> u32 {
    (base_byte & 0xff).wrapping_add(matching_orders)
}

// ---------------------------------------------------------------------------
// 0x006396cd / 0x00639719 / 0x00639765 / 0x00639871 / 0x006398bd /
// 0x00639911 - the refresh gates
// ---------------------------------------------------------------------------

/// The shared `0x006396cd`-family predicate.
///
/// ```c
/// if ((unaff_ESI[0xc] != 0) &&
///    (((*(byte *)((int)unaff_ESI + 0x1b) <= *(byte *)((int)unaff_ESI + 0x1a) ||
///       (0xe < *(byte *)((int)unaff_ESI + 0x1a))) &&
///      (499 < *(int *)((DAT_00f063cc ^ 0x10fae560) +
///                      (uint)*(byte *)(unaff_ESI + 6) * 4))))) {
///     FUN_00638c00(*(byte *)(unaff_ESI + 6));
///     FUN_00635060((char)unaff_ESI[6]);
/// }
/// ```
///
/// A zero slot skips the gate, a full queue (`capacity <= stock`) or a stock
/// above `0xe` passes it, and the per-group counter must exceed 499.
pub fn a83_refresh_gate(slot: u32, stock_0x1a: u8, capacity_0x1b: u8, counter: i32) -> bool {
    slot != 0 && (capacity_0x1b <= stock_0x1a || stock_0x1a > 0xe) && counter > 499
}

/// `0x006396cd`: the refresh gate on the `+0x30` slot.
pub fn a83_refresh_gate_slot_0x30(node: &A83OrderNode, counter: i32) -> bool {
    a83_refresh_gate(node.slot_0x30, node.stock, node.capacity, counter)
}

/// `0x00639719`: the refresh gate on the `+0x34` slot.
pub fn a83_refresh_gate_slot_0x34(node: &A83OrderNode, counter: i32) -> bool {
    a83_refresh_gate(node.slot_0x34, node.stock, node.capacity, counter)
}

/// `0x00639765`: the refresh gate on the `+0x38` slot.
pub fn a83_refresh_gate_slot_0x38(node: &A83OrderNode, counter: i32) -> bool {
    a83_refresh_gate(node.slot_0x38, node.stock, node.capacity, counter)
}

/// `0x00639871`: the unconditional refresh on the `+0x30` slot.
///
/// ```c
/// if (unaff_ESI[0xc] != 0) {
///     iVar4 = unaff_ESI[6];
///     *(undefined4 *)(unaff_EBP + -0x1c) = 0x5a;
///     FUN_00638c00((char)iVar4);
///     FUN_00635060((char)unaff_ESI[6]);
/// }
/// ```
pub fn a83_force_refresh_slot_0x30(node: &A83OrderNode) -> bool {
    node.slot_0x30 != 0
}

/// `0x006398bd`: the unconditional refresh on the `+0x34` slot.
pub fn a83_force_refresh_slot_0x34(node: &A83OrderNode) -> bool {
    node.slot_0x34 != 0
}

/// `0x00639911`: the unconditional refresh on the `+0x38` slot.
pub fn a83_force_refresh_slot_0x38(node: &A83OrderNode) -> bool {
    node.slot_0x38 != 0
}

// ---------------------------------------------------------------------------
// 0x00639990 / 0x006399b9 - the two static ring tables
// ---------------------------------------------------------------------------

/// One `while (uVar1 < limit) { ...; uVar1 += stride; }` ring loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A83RingLayout {
    /// First offset written by the loop (`0x18` or `0x2c`).
    pub first: u32,
    /// Loop stride (`0x18` or `0x2c`).
    pub stride: u32,
    /// Exclusive upper bound (`0x5da8` or `0xabb4`).
    pub limit: u32,
}

impl A83RingLayout {
    /// Number of loop iterations, i.e. `ceil((limit - first) / stride)`.
    pub fn a83_entries(self) -> u32 {
        if self.stride == 0 || self.first >= self.limit {
            return 0;
        }
        (self.limit - self.first + self.stride - 1) / self.stride
    }

    /// The last offset the loop writes.
    pub fn a83_last_offset(self) -> u32 {
        let entries = self.a83_entries();
        if entries == 0 {
            self.first
        } else {
            self.first + (entries - 1) * self.stride
        }
    }

    /// Total records in the ring, including the separately initialised slot 0.
    pub fn a83_total_records(self) -> u32 {
        self.a83_entries() + 1
    }
}

/// The group ring of `0x00639990`: `uVar1 = 0x18; ... while (uVar1 < 0x5da8)`.
pub const A83_GROUP_RING: A83RingLayout = A83RingLayout {
    first: A83_GROUP_RING_FIRST,
    stride: A83_GROUP_RING_STRIDE,
    limit: A83_GROUP_RING_LIMIT,
};

/// The command ring of `0x00639990`: `uVar1 = 0x2c; ... while (uVar1 < 0xabb4)`.
pub const A83_COMMAND_RING: A83RingLayout = A83RingLayout {
    first: A83_COMMAND_RING_FIRST,
    stride: A83_COMMAND_RING_STRIDE,
    limit: A83_COMMAND_RING_LIMIT,
};

/// The complete initialiser description written by `0x00639990` and
/// `0x006399b9`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A83TableInit {
    /// The group ring followed by the command ring.
    pub rings: [A83RingLayout; 2],
    /// Bytes zeroed in every ring record (`+0x08..+0x17`).
    pub zero_bytes_per_record: u32,
    /// Store width used for that zeroing: 4 for `0x00639990`, 1 for
    /// `0x006399b9`.
    pub store_width: u32,
    /// Bytes of the final `FUN_00c8a080(&DAT_00fbd308, 0, 0x1b4)`.
    pub tail_zero_bytes: u32,
}

/// `0x00639990`: the dword-store version of the two ring loops.
///
/// ```c
/// _DAT_00fac944 = 0; _DAT_00fac940 = &DAT_00fac958; ...
/// uVar1 = 0x18;
/// do {
///     *(&DAT_00fac944 + uVar1) = &DAT_00fac928 + uVar1;
///     *(&DAT_00fac940 + uVar1) = &DAT_00fac958 + uVar1;
///     *(&DAT_00fac948 + uVar1) = 0;
///     *(&DAT_00fac950 + uVar1) = 0;
///     *(&DAT_00fac954 + uVar1) = 0;
///     *(&DAT_00fac94c + uVar1) = 0;
///     uVar1 = uVar1 + 0x18;
/// } while (uVar1 < 0x5da8);
/// ```
///
/// The second loop is the same with `0x2c`/`0xabb4`; the row then links the
/// `0x00fb26xx` structure with the same `0x2c` stride and finally zeroes
/// `0x1b4` bytes at `0x00fbd308`.
pub fn a83_table_layout_words() -> A83TableInit {
    A83TableInit {
        rings: [A83_GROUP_RING, A83_COMMAND_RING],
        zero_bytes_per_record: A83_RECORD_ZERO_BYTES,
        store_width: 4,
        tail_zero_bytes: A83_TAIL_ZERO_BYTES,
    }
}

/// `0x006399b9`: the byte-store version of the same initialisation.
///
/// ```c
/// in_EAX[0xfac948] = '\0'; ... in_EAX[0xfac94f] = '\0';
/// in_EAX[0xfac950] = '\0'; ... in_EAX[0xfac957] = '\0';
/// in_EAX = in_EAX + 0x18;
/// } while (in_EAX < (char *)0x5da8);
/// ```
///
/// Sixteen byte stores replace the four dword stores; the offsets, strides and
/// final `0x1b4` block are identical.
pub fn a83_table_layout_bytes() -> A83TableInit {
    A83TableInit {
        rings: [A83_GROUP_RING, A83_COMMAND_RING],
        zero_bytes_per_record: A83_RECORD_ZERO_BYTES,
        store_width: 1,
        tail_zero_bytes: A83_TAIL_ZERO_BYTES,
    }
}

// ---------------------------------------------------------------------------
// 0x00639d00 - clear a unit from every controller list
// ---------------------------------------------------------------------------

/// Result of the eight-list scan in `0x00639d00`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A83ClearReport {
    /// Lists scanned (`uVar3` walks 0, 8, ..., 0x38).
    pub lists: u32,
    /// Slot words cleared across every list.
    pub cleared_fields: u32,
    /// Nodes whose icon byte was nonzero, i.e. the nodes marked dirty.
    pub dirty_nodes: u32,
}

/// `0x00639d00`: clear the four slot references across the eight group chains.
///
/// ```c
/// iVar2 = FUN_005b61d0();
/// if (iVar2 != 0) {
///     uVar3 = 0;
///     do {
///         for (puVar1 = *(undefined4 **)((int)&DAT_00fc86ac + uVar3); puVar1 != 0;
///              puVar1 = (undefined4 *)*puVar1) {
///             if (puVar1[0xb] == param_1) puVar1[0xb] = 0;
///             if (puVar1[0xc] == param_1) puVar1[0xc] = 0;
///             if (puVar1[0xd] == param_1) puVar1[0xd] = 0;
///             if (puVar1[0xe] == param_1) puVar1[0xe] = 0;
///             if (*(char *)(puVar1 + 7) != '\0')
///                 *(undefined1 *)((int)puVar1 + 0x1d) = 1;
///         }
///         uVar3 = uVar3 + 8;
///     } while (uVar3 < 0x40);
/// }
/// ```
///
/// The whole scan only runs when `FUN_005b61d0()` is nonzero; `scan` is that
/// gate.  The detach tail that follows in the row is the same controller link
/// release translated as [`a83_release_controller_link`].
pub fn a83_clear_unit_references(
    scan: bool,
    lists: &mut [Vec<A83OrderNode>],
    target: u32,
) -> A83ClearReport {
    let mut report = A83ClearReport::default();
    if !scan {
        return report;
    }
    for list in lists.iter_mut().take(A83_GROUP_COUNT as usize) {
        for node in list.iter_mut() {
            report.cleared_fields += node.a83_clear_reference(target);
            if node.icon != 0 {
                report.dirty_nodes += 1;
            }
        }
        report.lists += 1;
    }
    report
}

// ---------------------------------------------------------------------------
// 0x00639e10 - release the attached controller link
// ---------------------------------------------------------------------------

/// `0x00639e10`: the released link and its decremented depth byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A83ControllerRelease {
    /// The `+0x14` word of the attached record, passed to `FUN_00637810`.
    pub link_0x14: u32,
    /// The `+0x1a` byte of that link after the row's `-1`.
    pub depth_0x1a_after: u8,
}

/// `0x00639e10`: detach and decrement the linked unit's depth.
///
/// ```c
/// iVar3 = func_0x005b3b60();
/// if ((iVar3 != 0) && (iVar3 = *(int *)(param_1 + 0x134), iVar3 != 0)) {
///     iVar2 = *(int *)(iVar3 + 0x14);
///     *(undefined4 *)(param_1 + 0x134) = 0;
///     FUN_00636a70(iVar3);
///     pcVar1 = (char *)(iVar2 + 0x1a);
///     *pcVar1 = *pcVar1 + -1;
///     FUN_00637810(iVar2, param_1);
/// }
/// ```
///
/// Both the session gate and the `+0x134` record must be nonzero; the depth
/// decrement is an 8-bit wraparound add.
pub fn a83_release_controller_link(
    session_ok: bool,
    attached_0x134: u32,
    link_0x14: u32,
    depth_0x1a: u8,
) -> Option<A83ControllerRelease> {
    if !session_ok || attached_0x134 == 0 {
        return None;
    }
    Some(A83ControllerRelease {
        link_0x14,
        depth_0x1a_after: depth_0x1a.wrapping_sub(1),
    })
}

// ---------------------------------------------------------------------------
// 0x00639e60 - unit lookup and abort art
// ---------------------------------------------------------------------------

/// `0x00639e60`: return 1 after reporting the found unit's `+0x4c` byte.
///
/// ```c
/// iVar1 = FUN_0063fa20(*(undefined1 *)(param_1 + 0x18),
///                      (int)*(short *)(param_1 + 0x20),
///                      (int)*(short *)(param_1 + 0x22));
/// if (iVar1 != 0) FUN_00748d40(*(undefined1 *)(iVar1 + 0x4c));
/// return 1;
/// ```
///
/// The three lookup arguments are the `+0x18` byte and the `+0x20`/`+0x22`
/// words of the caller; `found_art_0x4c` is the byte at the found record's
/// `+0x4c`.  A hit yields `Err(art)` because the row's `FUN_00748d40` does not
/// return.
pub fn a83_lookup_art_or_abort(found_art_0x4c: Option<u8>) -> Result<u32, u8> {
    match found_art_0x4c {
        Some(art) => Err(art),
        None => Ok(1),
    }
}

// ---------------------------------------------------------------------------
// 0x00639ed0 - per-frame controller scan and invoke gate
// ---------------------------------------------------------------------------

/// `0x00639ed0`: rotate the player index byte.
///
/// ```c
/// DAT_00fb2704 = DAT_00fb2704 + 1;
/// if (DAT_00fb2704 == 8) DAT_00fb2704 = 0;
/// ```
pub fn a83_rotate_controller(current: u32) -> u32 {
    let next = current.wrapping_add(1);
    if next == A83_GROUP_COUNT { 0 } else { next }
}

/// `0x00639ed0`: count one "ready" node.
///
/// ```c
/// if ((piVar4[0xc] == 0) || (iVar2 = FUN_00638880(...), iVar2 != 3)) {
///     if ((piVar4[0xd] != 0) && (iVar2 = FUN_00638880(...), iVar2 == 3))
///         { iVar2 = piVar4[0xd]; goto ...; }
///     if ((piVar4[0xe] != 0) && (iVar2 = FUN_00638880(...), iVar2 == 3))
///         { iVar2 = piVar4[0xe]; goto ...; }
///     ... then the same three slots compared with 1 ...
/// }
/// if (iVar2 != 0) uVar5 = uVar5 + 1;
/// ```
///
/// A node counts once when any nonzero slot resolves to `3`, otherwise once
/// when any nonzero slot resolves to `1`.
pub fn a83_ready_slots<F>(slots: [u32; 3], group: u8, resolve: F) -> u32
where
    F: Fn(u8, u32) -> i32,
{
    let matches = |value: i32| {
        slots
            .iter()
            .any(|slot| *slot != 0 && resolve(group, *slot) == value)
    };
    if matches(3) || matches(1) { 1 } else { 0 }
}

/// `0x00639ed0`: the final invoke gate.
///
/// The row counts all nodes (`count`), the ready nodes (`ready`), sums
/// `FUN_00769b00` over the icon bytes (`value_sum`), tracks whether any node
/// is unready (`any_unready`), and later sums `FUN_0076a2e0` over the same
/// icons (`icon_sum`).  The two per-group counters and `FUN_006351d0` form the
/// final gate.
///
/// ```c
/// if (bVar1) return;
/// if (local_8 == 0) {
/// LAB_0063a048: if ((1 < uVar3) && (uVar5 < 2)) goto LAB_0063a0cd;
/// } else {
///     if (local_8 < 0x10) goto LAB_0063a0cd;
///     if (1 < uVar3) {
///         if (local_8 < 0x14) goto LAB_0063a0cd;
///         goto LAB_0063a048;
///     }
/// }
/// iVar2 = *(int *)((DAT_00f063cc ^ 0x10fae560) + DAT_00fb2704 * 4);
/// if (iVar2 < 2000) return;
/// if (4999 < iVar2) return;
/// iVar2 = *(int *)((DAT_00f063cc ^ 0x10fae560) + 0x30 + DAT_00fb2704 * 4);
/// if (iVar2 < 1000) return;
/// if (2999 < iVar2) return;
/// iVar2 = FUN_006351d0(DAT_00fb2704);
/// if (iVar2 == 0) return;
/// ... icon_sum ...
/// if (0x17 < uVar3) return;
/// LAB_0063a0cd:
/// FUN_00642a00(...);
/// ```
///
/// Returns `true` when the row reaches the `FUN_00642a00` call.
pub fn a83_controller_gate(
    any_unready: bool,
    count: u32,
    ready: u32,
    value_sum: u32,
    icon_sum: u32,
    counter: i32,
    counter2: i32,
    gate_ok: bool,
) -> bool {
    if any_unready {
        return false;
    }
    let mut counters = true;
    if value_sum == 0 {
        if count > 1 && ready < 2 {
            return true;
        }
    } else if value_sum < 0x10 {
        return true;
    } else if count > 1 {
        if value_sum < 0x14 {
            return true;
        }
        if count > 1 && ready < 2 {
            return true;
        }
    }
    if counter < 2000 {
        counters = false;
    }
    if counter > 4999 {
        counters = false;
    }
    if counter2 < 1000 {
        counters = false;
    }
    if counter2 > 2999 {
        counters = false;
    }
    if !gate_ok {
        counters = false;
    }
    if icon_sum > 0x17 {
        counters = false;
    }
    counters
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent83 {
    use super::*;

    fn nodes(count: usize) -> Vec<A83OrderNode> {
        vec![A83OrderNode::default(); count]
    }

    #[test]
    fn recovered_layout_constants() {
        assert_eq!(A83_ACTIVE_HEADS, A83_GROUP_TABLE + 4);
        assert_eq!(A83_FREE_HEAD_OFFSET, 46_000);
        assert_eq!(A83_ENTRY_OFFSET, 0x3c);
        assert_eq!(A83_ENTRY_COUNT, 100);
        assert_eq!(A83_ENTRY_OFFSET + 4 * A83_ENTRY_COUNT as u32, 0x1cc);
        assert_eq!(A83_COUNTER_SECOND, 0x30);
        assert_eq!(A83_COUNTER_XOR, 0x10fa_e560);
        assert_eq!(A83_NEAREST_LIMIT, 99_999_999);
    }

    // -- 0x006381e0 ---------------------------------------------------------

    #[test]
    fn nearest_search_skips_the_target_and_uses_strict_less() {
        assert_eq!(
            a83_nearest_distance_index(&[400, 100, 100, 50], None),
            Some(3)
        );
        assert_eq!(
            a83_nearest_distance_index(&[400, 100, 100, 50], Some(3)),
            Some(1)
        );
        assert_eq!(
            a83_nearest_distance_index(&[A83_NEAREST_LIMIT, A83_NEAREST_LIMIT], None),
            None,
            "the seed itself is not selectable"
        );
        assert_eq!(
            a83_nearest_distance_index(&[A83_NEAREST_LIMIT - 1], Some(0)),
            None
        );
    }

    #[test]
    fn release_gate_matches_the_row_order() {
        assert_eq!(
            a83_release_gate(true, 7, 9, 0x22, None),
            A83Release::Reentrant
        );
        assert_eq!(a83_release_gate(false, 7, 0, 0, None), A83Release::Busy);
        assert_eq!(
            a83_release_gate(false, 0, 0, 0, Some(3)),
            A83Release::Publish
        );
        assert_eq!(
            a83_release_gate(false, 0, 9, 0x22, None),
            A83Release::NoCandidate
        );
        assert_eq!(
            a83_release_gate(false, 0, 9, 0x22, Some(3)),
            A83Release::Abort(0x22)
        );
    }

    // -- 0x006382b1 / 0x006382e1 -------------------------------------------

    #[test]
    fn release_gate_variants_differ_only_in_their_guards() {
        assert_eq!(a83_release_gate_b(5, true, 0), A83Release::Busy);
        assert_eq!(
            a83_release_gate_b(5, false, 0),
            A83Release::Publish,
            "the +0x14 guard needs EBX too"
        );
        assert_eq!(a83_release_gate_b(0, false, 9), A83Release::Busy);
        assert_eq!(
            a83_release_gate_c(9, false),
            A83Release::Publish,
            "the +0x0c guard is qualified by EBX in 0x006382e1"
        );
        assert_eq!(a83_release_gate_c(9, true), A83Release::Busy);
    }

    // -- 0x006381e0 list move ----------------------------------------------

    #[test]
    fn move_to_free_list_unlinks_then_pushes_front() {
        let mut pool = nodes(4);
        // active: 2 <-> 1 <-> 3, free: 0
        pool[2].next = Some(1);
        pool[1].prev = Some(2);
        pool[1].next = Some(3);
        pool[3].prev = Some(1);
        let mut active = Some(2);
        let mut free = Some(0);

        a83_move_to_free_list(&mut active, &mut free, &mut pool, 1);

        assert_eq!(active, Some(2));
        assert_eq!(pool[2].next, Some(3));
        assert_eq!(pool[3].prev, Some(2));
        assert_eq!(pool[3].next, None);
        assert_eq!(free, Some(1));
        assert_eq!(pool[1].prev, None);
        assert_eq!(pool[1].next, Some(0));
        assert_eq!(pool[0].prev, Some(1));

        a83_move_to_free_list(&mut active, &mut free, &mut pool, 2);
        assert_eq!(active, Some(3), "head update happens first");
        assert_eq!(pool[3].prev, None);
        assert_eq!(free, Some(2));
    }

    // -- 0x006383d0 ---------------------------------------------------------

    #[test]
    fn spawn_pops_the_free_head_and_zeroes_the_documented_fields() {
        let mut pool = nodes(2);
        pool[0].countdown = 7;
        pool[1].prev = None;
        let mut free = Some(0);
        let mut active = Some(1);

        let spawned = a83_spawn_group_node(&mut free, &mut active, &mut pool, 3, 0x1111, 0x2222);
        assert_eq!(spawned, Some(0));
        assert_eq!(free, None);
        assert_eq!(active, Some(0));
        assert_eq!(pool[0].next, Some(1));
        assert_eq!(pool[1].prev, Some(0));
        assert_eq!(pool[0].group, 3);
        assert_eq!(pool[0].word_0x20, 0x1111);
        assert_eq!(pool[0].word_0x22, 0x2222);
        assert_eq!(pool[0].slot_0x2c, 0);
        assert_eq!(pool[0].entries_0x3c, [0; A83_ENTRY_COUNT]);
        assert_eq!(pool[0].countdown, 7, "0x006383d0 does not write +0x1e");

        let empty = a83_spawn_group_node(&mut free, &mut active, &mut pool, 3, 0, 0);
        assert_eq!(empty, None);
    }

    // -- 0x006384cf ---------------------------------------------------------

    #[test]
    fn shortfall_is_an_unsigned_truncating_division() {
        assert_eq!(a83_shortfall_units(0, 0, 1), Some(13));
        assert_eq!(a83_shortfall_units(0xc, 0, 1), Some(0x19));
        assert_eq!(a83_shortfall_units(0xd, 0, 1), None, "0xc < iVar3 returns");
        assert_eq!(
            a83_shortfall_units(0, 0xd, 1),
            None,
            "iVar3 + 0xd <= table returns"
        );
        assert_eq!(a83_shortfall_units(0, 0xd, 4), None);
        assert_eq!(a83_shortfall_units(0, 0, 4), Some(3), "25 / 4 truncates");
        assert_eq!(a83_shortfall_units(0, 0, 0), None);
    }

    #[test]
    fn top_up_needs_one_of_three_conditions_and_strictly_less_have() {
        assert!(a83_top_up_wanted(true, false, false, 0, 0));
        assert!(a83_top_up_wanted(false, false, false, 0, 0));
        assert!(a83_top_up_wanted(false, true, true, 0, 0));
        assert!(!a83_top_up_wanted(false, true, false, 0, 0));
        assert!(a83_top_up_wanted(true, false, false, 3, 3));
        assert!(!a83_top_up_wanted(true, false, false, 4, 3));
        assert!(!a83_top_up_wanted(true, false, false, 0, u32::MAX));
    }

    // -- 0x00638688 ---------------------------------------------------------

    #[test]
    fn selection_gate_bit_order() {
        assert_eq!(a83_selection_gate(0, 2), A83Gate::Reject);
        assert_eq!(a83_selection_gate(2, 2), A83Gate::Proceed);
        assert_eq!(a83_selection_gate(1, 1), A83Gate::Proceed);
        assert_eq!(a83_selection_gate(0, 1), A83Gate::Reject);
        assert_eq!(a83_selection_gate(4, 0), A83Gate::Proceed);
        assert_eq!(a83_selection_gate(0, 0), A83Gate::Reject);
        assert_eq!(
            a83_selection_gate(1, 3),
            A83Gate::Reject,
            "query bit 1 wins over bit 0"
        );
    }

    // -- 0x006387f0 / 0x00638880 -------------------------------------------

    #[test]
    fn slot_lookup_reports_zero_and_asserts_on_a_bad_nonzero_slot() {
        assert_eq!(a83_unit_slot_lookup(false, false), Ok(0));
        assert_eq!(a83_unit_slot_lookup(true, true), Ok(0));
        assert_eq!(a83_unit_slot_lookup(true, false), Err(()));
    }

    #[test]
    fn ranked_slot_reproduces_the_three_comparisons() {
        let zero = |_: u8, _: u32| 0u32;
        assert_eq!(a83_ranked_slot(0x30, 0x34, 0x38, 0, 1, 0, zero), 0x30);
        assert_eq!(a83_ranked_slot(0x30, 0x34, 0x38, 0, 2, 0, zero), 0x34);
        assert_eq!(a83_ranked_slot(0x30, 0x34, 0x38, 0, 3, 0, zero), 0x38);
        assert_eq!(
            a83_ranked_slot(0, 0, 0x38, 0, 1, 0, zero),
            0x38,
            "the third block compares iVar2 + 1 without incrementing"
        );
        assert_eq!(a83_ranked_slot(0x30, 0x34, 0x38, 0, 1, 0, zero), 0x30);
        assert_eq!(
            a83_ranked_slot(0x30, 0, 0, 0, 2, 0, zero),
            0,
            "the second block was never entered"
        );
        let ones = |_: u8, _: u32| 1u32;
        assert_eq!(a83_ranked_slot(0x30, 0x34, 0x38, 1, 2, 0, ones), 0x34);
        assert_eq!(a83_ranked_slot(0x30, 0x34, 0x38, 2, 2, 0, ones), 0);
    }

    // -- 0x006388b7 / 0x006388f9 / 0x0063890a / 0x00638917 / 0x00638924 --

    #[test]
    fn probe_and_command_predicates() {
        assert_eq!(a83_probe_result(false, false), 0);
        assert_eq!(a83_probe_result(true, false), 3);
        assert_eq!(a83_probe_result(true, true), 2);

        assert_eq!(a83_accept_unless_cancel(0xbc), None);
        assert_eq!(a83_accept_unless_cancel(0x23), Some(1));
        assert_eq!(a83_accept_only_0x23(0x23), Some(1));
        assert_eq!(a83_accept_only_0x23(0x24), None);
        assert_eq!(a83_accept_only_0x24(0x24), Some(1));
        assert_eq!(a83_accept_only_0x24(0x23), None);
        assert_eq!(a83_is_target_command_0x2a(0x2a), 1);
        assert_eq!(a83_is_target_command_0x2a(0x2b), 0);
    }

    // -- 0x006389e0 ---------------------------------------------------------

    #[test]
    fn status_message_precedence_and_asserts() {
        assert_eq!(a83_status_message(true, true, 1, 1), A83Notice::First);
        assert_eq!(a83_status_message(false, true, 1, 1), A83Notice::Second);
        assert_eq!(a83_status_message(false, false, 1, 0), A83Notice::Assert);
        assert_eq!(a83_status_message(false, false, 0, 1), A83Notice::Assert);
        assert_eq!(a83_status_message(false, false, 0, 0), A83Notice::Nothing);
    }

    // -- 0x00638b6c ---------------------------------------------------------

    #[test]
    fn ability_slot_offset_and_limit() {
        let slot = a83_ability_slot_flag(true, 1, 0, 2, true);
        assert_eq!(slot.byte_offset, (1 + 0xc) * 0x24 + 9 + 2);
        assert_eq!(slot.limit, 0x29);
        let slot = a83_ability_slot_flag(false, 1, 0x100, 3, false);
        assert_eq!(slot.byte_offset, 0x100 + 9 + 3);
        assert_eq!(slot.limit, 0x40);
    }

    // -- 0x00638ca0 ---------------------------------------------------------

    #[test]
    fn score_sum_skips_zero_icons_and_wraps() {
        assert_eq!(a83_group_score_sum([(1u8, 5i32), (0, 9), (2, 7)]), 12);
        assert_eq!(a83_group_score_sum([(1u8, i32::MAX), (1, 1)]), i32::MIN);
        assert_eq!(a83_group_score_sum([]), 0);
    }

    // -- 0x00638d30 ---------------------------------------------------------

    #[test]
    fn first_matching_target_short_circuits() {
        let entries = [
            A83TargetEntry {
                word_0x4c: 1,
                primary_match: false,
                secondary_match: false,
            },
            A83TargetEntry {
                word_0x4c: 2,
                primary_match: false,
                secondary_match: true,
            },
        ];
        assert!(a83_first_matching_target(&entries, 2));
        assert!(!a83_first_matching_target(&entries, 1));
        assert!(!a83_first_matching_target(&entries, 3));
    }

    // -- 0x00638f10 ---------------------------------------------------------

    #[test]
    fn self_detach_needs_a_controller_and_orders_the_release() {
        assert_eq!(a83_self_detach(0, 0x44, true), None);
        assert_eq!(
            a83_self_detach(0x134, 0x44, true),
            Some(A83SelfDetach {
                release_self_first: true,
                link_0x14: 0x44,
            })
        );
        assert_eq!(
            a83_self_detach(0x134, 0x44, false),
            Some(A83SelfDetach {
                release_self_first: false,
                link_0x14: 0x44,
            })
        );
    }

    // -- 0x00638fc0 ---------------------------------------------------------

    #[test]
    fn queue_room_countdown_and_clamp() {
        assert_eq!(a83_order_queue_room(1, 2, 6, 3), 0);
        assert_eq!(a83_order_queue_room(0, 2, 2, 3), 0, "capacity <= stock");
        assert_eq!(a83_order_queue_room(0, 2, 6, 10), 4, "clamped to 6 - 2");
        assert_eq!(a83_order_queue_room(0, 2, 6, 3), 3);
        assert_eq!(
            a83_order_queue_room(0, 0, 6, 10),
            10,
            "zero stock skips the clamp"
        );
        assert_eq!(a83_order_queue_room(0, 0, 6, 0), 0);
        assert_eq!(a83_order_queue_room(0, 2, 6, -1), -1);

        for excluded in [b'S', b'W', b'X', 0x1e, b'!', 0x1f, b'F', 0x1a, 0x19] {
            assert!(!a83_order_char_eligible(excluded, true), "{excluded:#x}");
        }
        assert!(a83_order_char_eligible(b'L', true));
        assert!(!a83_order_char_eligible(b'L', false));
    }

    // -- 0x006390f0 ---------------------------------------------------------

    #[test]
    fn ability_entry_decode_and_clamp() {
        let entry = a83_ability_entry([0b0001_1110, 5, 0x34, 0x12]);
        assert!(entry.present);
        assert_eq!(entry.kind, 3);
        assert_eq!(entry.requested, 3);
        assert_eq!(entry.id, 0x1234);
        assert_eq!(entry.amount, 5);

        let entry = a83_ability_entry([7, 5, 0, 0]);
        assert!(!entry.present);
        assert_eq!(entry.requested, 0);

        assert_eq!(a83_ability_requested(0b0010_1000, 9, false), 5);
        assert_eq!(a83_ability_requested(0b0010_1000, 4, true), 4);
        assert_eq!(a83_ability_requested(0b0010_1000, 9, true), 5);
        assert_eq!(a83_fill_count(5, 2), 3);
        assert_eq!(a83_fill_count(2, 2), 0);
        assert_eq!(a83_fill_count(2, 5), 0);
    }

    #[test]
    fn multi_select_gate_conditions() {
        assert!(a83_multi_select_gate(2, 0, false, false, 0));
        assert!(!a83_multi_select_gate(2, 0, false, false, 7));
        assert!(a83_multi_select_gate(2, 0, false, true, 7));
        assert!(
            !a83_multi_select_gate(1, 0, true, true, 0),
            "stock 1 needs the +0x24 link"
        );
        assert!(a83_multi_select_gate(1, 5, true, false, 0));
        assert!(!a83_multi_select_gate(1, 5, false, false, 0));
        assert!(
            !a83_multi_select_gate(0, 5, true, true, 0),
            "stock 0 never enters the multi branch"
        );
    }

    // -- 0x0063918c / 0x0063921a / 0x0063925f / 0x00639381 ----------------

    #[test]
    fn funclet_arithmetic() {
        assert!(a83_skip_queue_flush(0x83));
        assert!(!a83_skip_queue_flush(0x84));
        assert_eq!(a83_selection_bias(true, 9, 0x10), 0x11);
        assert_eq!(a83_selection_bias(false, 9, 0x10), 0x19);
        assert_eq!(a83_selection_bias(false, u32::MAX, 2), 1, "the add wraps");
        assert!(a83_funclet_selector_abort(0));
        assert!(a83_funclet_selector_abort(0xc));
        assert!(!a83_funclet_selector_abort(2));
        assert_eq!(a83_funclet_abort_code(0x83), 0);
        assert_eq!(a83_funclet_abort_code(0x80), u32::MAX - 2);
        assert_eq!(a83_existing_order_count(0x1ff, 3), 0xff + 3);
        assert_eq!(a83_existing_order_count(0, u32::MAX), u32::MAX);
    }

    // -- refresh gates ------------------------------------------------------

    #[test]
    fn refresh_gates_read_the_right_slot() {
        let mut node = A83OrderNode {
            stock: 2,
            capacity: 2,
            ..A83OrderNode::default()
        };
        node.slot_0x30 = 0x30;
        node.slot_0x34 = 0;
        node.slot_0x38 = 0x38;
        assert!(a83_refresh_gate_slot_0x30(&node, 500));
        assert!(!a83_refresh_gate_slot_0x34(&node, 500));
        assert!(a83_refresh_gate_slot_0x38(&node, 500));
        assert!(!a83_refresh_gate_slot_0x30(&node, 499));
        assert!(a83_force_refresh_slot_0x30(&node));
        assert!(!a83_force_refresh_slot_0x34(&node));
        assert!(a83_force_refresh_slot_0x38(&node));

        node.stock = 0xf;
        node.capacity = 0;
        assert!(a83_refresh_gate_slot_0x30(&node, 500), "stock > 0xe");
        node.stock = 0;
        node.capacity = 3;
        assert!(
            !a83_refresh_gate_slot_0x30(&node, 500),
            "neither full nor stock > 0xe"
        );
    }

    // -- 0x00639990 / 0x006399b9 -------------------------------------------

    #[test]
    fn ring_layout_counts_and_limits() {
        let words = a83_table_layout_words();
        let bytes = a83_table_layout_bytes();
        assert_eq!(words.rings, [A83_GROUP_RING, A83_COMMAND_RING]);
        assert_eq!(words.rings[0].a83_entries(), 998);
        assert_eq!(words.rings[1].a83_entries(), 998);
        assert_eq!(words.rings[0].a83_total_records(), 999);
        assert_eq!(words.rings[1].a83_total_records(), 999);
        assert_eq!(words.rings[0].a83_last_offset(), 0x5d90);
        assert_eq!(words.rings[1].a83_last_offset(), 0xab88);
        assert_eq!(words.zero_bytes_per_record, 16);
        assert_eq!(words.store_width, 4);
        assert_eq!(bytes.store_width, 1);
        assert_eq!(bytes.tail_zero_bytes, 0x1b4);
        assert_eq!(words.tail_zero_bytes, bytes.tail_zero_bytes);
        assert_eq!(
            A83_GROUP_RING.a83_last_offset() + A83_GROUP_RING.stride,
            0x5da8
        );
        assert_eq!(
            A83_COMMAND_RING.a83_last_offset() + A83_COMMAND_RING.stride,
            0xabb4
        );
    }

    // -- 0x00639d00 ---------------------------------------------------------

    #[test]
    fn clear_unit_references_clears_four_slots_and_marks_dirty() {
        let mut lists = vec![Vec::new(); A83_GROUP_COUNT as usize];
        let mut node = A83OrderNode {
            icon: 4,
            ..A83OrderNode::default()
        };
        node.slot_0x2c = 0x99;
        node.slot_0x30 = 0x99;
        node.slot_0x34 = 0x98;
        node.slot_0x38 = 0x99;
        lists[0].push(node);
        lists[7].push(A83OrderNode {
            slot_0x30: 0x99,
            ..A83OrderNode::default()
        });

        let report = a83_clear_unit_references(true, &mut lists, 0x99);
        assert_eq!(report.lists, 8);
        assert_eq!(report.cleared_fields, 4);
        assert_eq!(report.dirty_nodes, 1);
        assert_eq!(lists[0][0].slot_0x2c, 0);
        assert_eq!(lists[0][0].slot_0x34, 0x98);
        assert_eq!(lists[0][0].dirty, 1);
        assert_eq!(lists[7][0].slot_0x30, 0);
        assert_eq!(lists[7][0].dirty, 0);

        let skipped = a83_clear_unit_references(false, &mut lists, 0x99);
        assert_eq!(skipped, A83ClearReport::default());
    }

    // -- 0x00639e10 ---------------------------------------------------------

    #[test]
    fn controller_link_release_decrements_the_depth() {
        assert_eq!(a83_release_controller_link(false, 1, 2, 3), None);
        assert_eq!(a83_release_controller_link(true, 0, 2, 3), None);
        assert_eq!(
            a83_release_controller_link(true, 1, 2, 3),
            Some(A83ControllerRelease {
                link_0x14: 2,
                depth_0x1a_after: 2,
            })
        );
        assert_eq!(
            a83_release_controller_link(true, 1, 2, 0)
                .unwrap()
                .depth_0x1a_after,
            0xff
        );
    }

    // -- 0x00639e60 ---------------------------------------------------------

    #[test]
    fn lookup_reports_the_art_on_a_hit() {
        assert_eq!(a83_lookup_art_or_abort(None), Ok(1));
        assert_eq!(a83_lookup_art_or_abort(Some(0x4c)), Err(0x4c));
    }

    // -- 0x00639ed0 ---------------------------------------------------------

    #[test]
    fn controller_rotation_and_ready_count() {
        assert_eq!(a83_rotate_controller(0), 1);
        assert_eq!(a83_rotate_controller(6), 7);
        assert_eq!(a83_rotate_controller(7), 0);
        assert_eq!(a83_rotate_controller(0xff), 0x100, "only 8 wraps");

        let mut node = A83OrderNode {
            countdown: 2,
            ..A83OrderNode::default()
        };
        node.a83_tick_countdown();
        assert_eq!(node.countdown, 1);
        node.a83_tick_countdown();
        assert_eq!(node.countdown, 0);
        node.a83_tick_countdown();
        assert_eq!(node.countdown, 0, "zero is not decremented");

        let three = |_: u8, _: u32| 3i32;
        let one = |_: u8, _: u32| 1i32;
        let zero = |_: u8, _: u32| 0i32;
        assert_eq!(a83_ready_slots([1, 0, 0], 0, three), 1);
        assert_eq!(a83_ready_slots([0, 1, 0], 0, one), 1);
        assert_eq!(a83_ready_slots([0, 0, 0], 0, one), 0);
        assert_eq!(a83_ready_slots([1, 0, 0], 0, zero), 0);
    }

    #[test]
    fn controller_gate_matrix() {
        let ok = |value_sum, count, ready| {
            a83_controller_gate(false, count, ready, value_sum, 0x17, 2000, 1000, true)
        };
        assert!(!a83_controller_gate(true, 2, 2, 0x10, 0, 2000, 1000, true));
        assert!(ok(0, 2, 1));
        assert!(ok(0x10, 1, 0));
        assert!(ok(0x13, 2, 2));
        assert!(ok(0x14, 2, 1));
        assert!(ok(0x14, 1, 0), "count <= 1 falls to the counters");
        assert!(ok(0, 2, 2), "the counters are all in range");
        assert!(!a83_controller_gate(false, 2, 2, 0, 0x18, 2000, 1000, true));
        assert!(!a83_controller_gate(false, 2, 2, 0, 0, 1999, 1000, true));
        assert!(!a83_controller_gate(false, 2, 2, 0, 0, 5000, 1000, true));
        assert!(!a83_controller_gate(false, 2, 2, 0, 0, 2000, 999, true));
        assert!(!a83_controller_gate(false, 2, 2, 0, 0, 2000, 3000, true));
        assert!(!a83_controller_gate(false, 2, 2, 0, 0, 2000, 1000, false));
        assert!(
            a83_controller_gate(false, 2, 1, 0, 0x18, 1, 1, false),
            "the early +0x048 branch skips every counter and icon check"
        );
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// The 21 rows not translated above, grouped by why:
//
// * `FUN_006384a0`, `FUN_006387c0`, `FUN_00638a70`, `FUN_00638b00`,
//   `FUN_00638940`, `FUN_00638c00`, `FUN_00638ba0`: one forwarding call
//   (`FUN_00748d40`, `FUN_0065b900`, `FUN_0065b910`) with a caller-frame
//   argument; the callee is outside the window and marked non-returning, so
//   there is no arithmetic or table left in the row.
// * `FUN_00638640`: the only logic is `FUN_00635c00(param_1) ==
//   *(int *)(param_3 + 0x14)` followed by an assertion; the accessor has no
//   body in the row.
// * `FUN_00638703`: another branch of the dialog/command handler in this
//   window; every store is at `EBP+0x12`/`EBP-4`/`EBP-2` in the caller's frame
//   and the tail is `FUN_005b6680`/`FUN_0063a3c0`.
// * `FUN_006388e0`: both branches end in the non-returning `FUN_005b4cd0`;
//   the visible call `FUN_005b5820` only chooses between two identical
//   aborts.
// * `FUN_006395b3`, `FUN_006395c4`, `FUN_00639614`, `FUN_00639625`,
//   `FUN_00639668`, `FUN_00639679`, `FUN_006396e2`, `FUN_0063972e`,
//   `FUN_0063977a`, `FUN_00639886`, `FUN_006398d2`: label continuations of the
//   same inlined command handler.  Their distinct heads only choose which of
//   the already-translated slot checks (`a83_refresh_gate_slot_*` /
//   `a83_force_refresh_slot_*`) and `FUN_00638fc0` calls run; the remaining
//   body is the shared `LAB_0063942a`/`LAB_00639529` walk whose state lives in
//   the caller's `EBP` frame, so an independent translation would invent that
//   frame.
