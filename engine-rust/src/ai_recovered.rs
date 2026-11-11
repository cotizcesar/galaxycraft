//! Recovered AI build-queue, card and script-runtime helpers.
//!
//! These pure/self-contained translations were merged out of the per-agent
//! staging files; they have no direct rules call site but document the AI
//! queue, card and script allocation behaviour.

#![allow(
    clippy::unusual_byte_groupings,
    clippy::unnecessary_cast,
    clippy::too_many_arguments,
    clippy::collapsible_if,
    clippy::result_unit_err,
    clippy::byte_char_slices,
    clippy::while_let_loop,
    clippy::redundant_closure,
    clippy::manual_is_multiple_of,
    clippy::manual_repeat_n,
    clippy::assign_op_pattern,
    clippy::implicit_saturating_sub,
    clippy::if_same_then_else,
    clippy::doc_lazy_continuation,
    clippy::field_reassign_with_default,
    clippy::identity_op,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::manual_range_patterns,
    clippy::needless_range_loop,
    clippy::erasing_op,
    clippy::bool_assert_comparison,
    clippy::chunks_exact_to_as_chunks,
    clippy::unnecessary_map_or,
    clippy::precedence,
    clippy::derivable_impls,
    clippy::manual_div_ceil
)]

// ===========================================================================
// Merged from src/translated/agent81.rs
// ===========================================================================
// Translation batch 81.  Only this file is edited by the batch.
//
// Batch 81 owns `0x00634000..=0x00635fff`.  The ledger lists 97
// `decompiled_uncited` rows in that window; 40 are translated below and the
// remaining 57 are listed at the end with their reason.
//
// The whole window is one per-player build queue: eight `0x4e8`-byte records
// based at `DAT_00faa1f8` (`8 * 0x4e8 = 0x2740` bytes, the extent cleared by
// `FUN_006356b0`).  Recovered layout, offsets relative to the record base:
//
// | offset           | owner                                                     |
// |------------------|-----------------------------------------------------------|
// | `+0x00..+0x08`   | remaining cost accumulator, three `i32`                   |
// | `+0x0c..+0x14`   | charged cost accumulator, three `i32`                     |
// | `+0x18`          | priority heap, 63 `{u32 key, u32 value}` entries          |
// | `+0x210`         | heap count byte (first byte of the 64th entry slot)       |
// | `+0x211`         | queue-kind / started byte                                 |
// | `+0x212`         | seconds multiplier                                        |
// | `+0x213`         | counter paired with flag `0x800`                          |
// | `+0x214`         | completion tick                                           |
// | `+0x218`         | flag word                                                 |
// | `+0x21a`         | word: current queue entry type                            |
// | `+0x21c`         | word                                                      |
// | `+0x21e`         | word                                                      |
// | `+0x220`         | pending count                                             |
// | `+0x222..+0x22f` | bytes                                                     |
// | `+0x4d8`         | four tail words copied by `0x00635fc0`                    |
//
// The heap is a 63-entry max-heap keyed on the **low byte** of `key`; the
// 64th 8-byte slot holds the count byte, so the largest real index is `0x3e`
// and a full heap (`count == 0x3f`) overwrites that last entry instead of
// growing.  Every sift comparison is a byte compare, reproduced here with
// `key as u8`.
//
// Address to function:
//
// * `0x006344c0` [`a81_cost_query_plan`]
// * `0x00634520` [`a81_queue_append`]
// * `0x00634570` [`a81_queue_push`]
// * `0x006347e0` [`a81_refund_head`]
// * `0x0063484c` [`a81_refund_head_split`]
// * `0x006348c0` [`A81QueueHeap::a81_pop`]
// * `0x00634957` [`A81QueueHeap::a81_sift_down_from`]
// * `0x006349a0` [`a81_flag_bit0`]
// * `0x006349f0` [`a81_array_contains_id`]
// * `0x00634a40` [`a81_slot_328`]
// * `0x00634b30` [`a81_append_checked`]
// * `0x00634bc0` [`a81_thresholds_ok`]
// * `0x00634d10` [`a81_read_repeat`]
// * `0x00634e70` [`A81BuildQueue::a81_clear_repeat_flag`]
// * `0x006350f0` [`a81_queue_kind_for_training`]
// * `0x00635150` [`a81_finish_order`]
// * `0x00635170` [`A81BuildQueue::a81_take_pending`]
// * `0x006351b0` [`a81_read_word_21a`]
// * `0x006351d0` [`a81_flag_200`]
// * `0x00635240` [`a81_cancel_order`]
// * `0x00635260` [`a81_flag_80`]
// * `0x00635280` [`A81BuildQueue::a81_set_flag_200`]
// * `0x006353d0` [`A81BuildQueue::a81_mark_started`]
// * `0x00635410` [`A81BuildQueue::a81_started_byte`]
// * `0x00635430` [`A81BuildQueue::a81_set_flag_20`]
// * `0x00635470` [`A81BuildQueue::a81_charge_offset`]
// * `0x00635490` [`a81_flag_100`]
// * `0x00635590` [`a81_read_word_21c`]
// * `0x006355b0` [`a81_extend_stall`]
// * `0x00635600` [`a81_cost_split`]
// * `0x00635720` [`a81_flag_20`]
// * `0x00635820` [`a81_ready_at`]
// * `0x00635870` [`A81BuildQueue::a81_set_flag_100`]
// * `0x006359d0` [`a81_next_slot`]
// * `0x00635b90` [`A81BuildQueue::a81_clear_flag_800`]
// * `0x00635c60` [`A81BuildQueue::a81_start_elapsed`]
// * `0x00635d10` [`a81_supply_sufficient`]
// * `0x00635dd0` [`A81BuildQueue::a81_add_cost`]
// * `0x00635e20` [`A81BuildQueue::a81_add_cost_by_kind`]
// * `0x00635fc0` [`A81BuildQueue::a81_tail_words`]
//
// The rows that only move a frame register, call an out-of-batch helper or
// decode to overlapping instructions are listed in the final section.  No
// `sim` record gains a field; everything here is a standalone value model.

// ---------------------------------------------------------------------------
// Record geometry
// ---------------------------------------------------------------------------

/// Stride of one per-player record (`param_1 * 0x4e8` in every row).
pub const A81_RECORD_STRIDE: u32 = 0x4e8;

/// Number of records; every bounded accessor tests `param_1 < 8`.
pub const A81_RECORD_COUNT: u32 = 8;

/// Extent cleared by `FUN_006356b0` (`memset(&DAT_00faa1f8, 0, 0x2740)`).
pub const A81_RECORD_BYTES: u32 = A81_RECORD_STRIDE * A81_RECORD_COUNT;

/// Base of the array (`DAT_00faa1f8`).
pub const A81_RECORD_BASE: u32 = 0x00fa_a1f8;

const _: () = assert!(A81_RECORD_BYTES == 0x2740);

/// Real entry slots in the `+0x18` heap.  The following (64th) 8-byte slot is
/// the count byte, so the maximum index a row can use is `0x3e`.
pub const A81_HEAP_ENTRIES: usize = 0x3f;

/// The count byte value that means "heap full".
pub const A81_HEAP_FULL: u8 = 0x3f;

/// `+0x18` heap entry.  `key` is packed by [`a81_pack_heap_key`]; only its
/// low byte participates in the sift comparisons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A81HeapEntry {
    /// Key word at entry `+0x00`.
    pub key: u32,
    /// Payload word at entry `+0x04`.
    pub value: u32,
}

impl A81HeapEntry {
    /// The byte used as the heap priority (`(byte)key`).
    pub fn a81_priority(&self) -> u8 {
        self.key as u8
    }
}

/// The per-player priority heap at `+0x18` with its count byte at `+0x210`.
///
/// `count` never exceeds [`A81_HEAP_FULL`]; a push into a full heap reuses
/// index `0x3e`, exactly like the shipped code.  The byte count is stored
/// separately here because the shipped count byte aliases the first byte of
/// the 64th entry slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A81QueueHeap {
    /// `+0x210`, in `0..=0x3f`.
    pub count: u8,
    /// Entries `0..0x3e`.
    pub entries: [A81HeapEntry; A81_HEAP_ENTRIES],
}

impl Default for A81QueueHeap {
    fn default() -> Self {
        A81QueueHeap {
            count: 0,
            entries: [A81HeapEntry::default(); A81_HEAP_ENTRIES],
        }
    }
}

impl A81QueueHeap {
    /// `0x00634570`: push `{key, value}` and sift up on the key byte.
    ///
    /// ```c
    /// len = count_byte;
    /// if (len == 0x3f) index = 0x3e;
    /// else { index = len; count_byte = len + 1; }
    /// heap[index] = { key, value };
    /// while (index != 0) {
    ///     parent = (index - 1) / 2;
    ///     if ((byte)heap[index].key <= (byte)heap[parent].key) return;
    ///     swap(index, parent);
    ///     index = parent;
    /// }
    /// ```
    ///
    /// Returns the slot the entry was written to before sifting; that is the
    /// `uVar7` value the row uses as its index and is useful in tests, but it
    /// is not otherwise observable.
    pub fn a81_push(&mut self, key: u32, value: u32) -> usize {
        let index = if self.count == A81_HEAP_FULL {
            0x3e
        } else {
            let index = self.count as usize;
            self.count += 1;
            index
        };
        self.entries[index] = A81HeapEntry { key, value };
        let slot = index;

        let mut index = index;
        while index != 0 {
            let parent = (index - 1) / 2;
            if self.entries[index].a81_priority() <= self.entries[parent].a81_priority() {
                break;
            }
            self.entries.swap(index, parent);
            index = parent;
        }
        slot
    }

    /// `0x006348c0`: remove the heap root and restore the heap order.
    ///
    /// ```c
    /// count_byte -= 1;                         // 8-bit wrap
    /// if (count_byte != 0) {
    ///     heap[0] = heap[count_byte];
    ///     if (count_byte > 1) {                // sift down from 0
    ///         i = 0;
    ///         child = 1;
    ///         do {
    ///             right = child + 1;
    ///             if (right >= count_byte || (byte)heap[right].key
    ///                                      <  (byte)heap[child].key)
    ///                 right = child;
    ///             if ((byte)heap[right].key <= (byte)heap[i].key) return;
    ///             swap(i, right);
    ///             i = right;
    ///             child = i * 2 + 1;
    ///         } while (child <= count_byte - 1);
    ///     }
    /// }
    /// ```
    ///
    /// The row returns nothing; the removed entry is returned here because it
    /// identifies what the caller (`0x00635150`/`0x00635240`) just refunded.
    /// An already-empty heap would wrap the byte to `0xff` and read past the
    /// record, so the model reports `None` instead.
    pub fn a81_pop(&mut self) -> Option<A81HeapEntry> {
        if self.count == 0 {
            return None;
        }
        let removed = self.entries[0];
        self.count = self.count.wrapping_sub(1);
        if self.count != 0 {
            let last = self.count as usize;
            self.entries[0] = self.entries[last];
            if self.count > 1 {
                self.a81_sift_down_from(0);
            }
        }
        Some(removed)
    }

    /// `0x00634957`: the sift-down continuation used by `0x006348c0`.
    ///
    /// ```c
    /// do {
    ///     count = count_byte;
    ///     left = index * 2 + 1;
    ///     if (count - 1 < left) return;
    ///     right = index * 2 + 2;
    ///     if (count <= right || (byte)heap[right].key < (byte)heap[left].key)
    ///         right = left;
    ///     if ((byte)heap[right].key <= (byte)heap[index].key) return;
    ///     swap(index, right);
    ///     index = right;
    /// } while (true);
    /// ```
    ///
    /// The row is the loop body of `0x006348c0` with `index` in `EBX` and the
    /// count byte re-read from `EAX + 0x210`; it is exposed here so the heap
    /// can be repaired from an arbitrary index.
    pub fn a81_sift_down_from(&mut self, index: usize) {
        let mut index = index;
        loop {
            let count = self.count as usize;
            let left = index * 2 + 1;
            if left >= count {
                return;
            }
            let mut child = index * 2 + 2;
            if child >= count
                || self.entries[child].a81_priority() < self.entries[left].a81_priority()
            {
                child = left;
            }
            if self.entries[child].a81_priority() <= self.entries[index].a81_priority() {
                return;
            }
            self.entries.swap(index, child);
            index = child;
        }
    }
}

// ---------------------------------------------------------------------------
// The record value model
// ---------------------------------------------------------------------------

/// One `0x4e8`-byte build-queue record.
///
/// This is a standalone model of the fields the batch touches; it is not a
/// `SimUnit` or `World` field.  Offsets in the comments are relative to
/// `DAT_00faa1f8 + player * 0x4e8`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A81BuildQueue {
    /// `+0x00`: remaining cost accumulator (`0x006347e0` saturates it).
    pub remaining: [i32; 3],
    /// `+0x0c`: charged cost accumulator (`0x006347e0` wraps it on refund).
    pub charged: [i32; 3],
    /// `+0x18`: priority heap, count byte at `+0x210`.
    pub heap: A81QueueHeap,
    /// `+0x211`: kind selector handed to `0x006344c0`; `1` starts the queue.
    pub byte_211: u8,
    /// `+0x212`: seconds multiplier (`+0x214 + byte_212 * 0x3c`).
    pub byte_212: u8,
    /// `+0x213`: counter cleared with flag `0x800`.
    pub byte_213: u8,
    /// `+0x214`: completion tick read by `0x00635820`.
    pub tick_214: i32,
    /// `+0x218`: flag word.
    pub flags: u16,
    /// `+0x21a`: current queue entry type (`0x006347e0`'s skip set).
    pub word_21a: u16,
    /// `+0x21c`.
    pub word_21c: u16,
    /// `+0x21e`.
    pub word_21e: u16,
    /// `+0x220`: pending count consumed by `0x00635170`.
    pub pending: u16,
    /// `+0x222`.
    pub byte_222: u8,
    /// `+0x223`: repeat count (`0x00634d10`).
    pub repeat: u8,
    /// `+0x224`: start tick (`0x00635c60`).
    pub start_tick: i32,
    /// `+0x22c`: repeat timer cleared with flag `0x40`.
    pub byte_22c: u8,
    /// `+0x22d`.
    pub byte_22d: u8,
    /// `+0x22e`.
    pub byte_22e: u8,
    /// `+0x22f`.
    pub byte_22f: u8,
    /// `+0x4d8`: four words copied out by `0x00635fc0`.
    pub tail_words: [u32; 4],
}

impl A81BuildQueue {
    /// `0x006347e0`/`0x0063484c`: refund a cost against the record.
    ///
    /// The remaining accumulator (`+0x00`, `+0x04`, `+0x08`) saturates at
    /// zero using **unsigned** comparisons; the charged accumulator
    /// (`+0x0c`, `+0x10`, `+0x14`) is a plain 32-bit wrapping subtraction and
    /// is only touched when `charge_back` is set.
    pub fn a81_refund(&mut self, cost: [i32; 3], charge_back: bool) {
        for i in 0..3 {
            self.remaining[i] = if (self.remaining[i] as u32) < (cost[i] as u32) {
                0
            } else {
                self.remaining[i].wrapping_sub(cost[i])
            };
        }
        if charge_back {
            for i in 0..3 {
                self.charged[i] = self.charged[i].wrapping_sub(cost[i]);
            }
        }
    }

    /// `0x00634e70`: clear flag `0x40` and decrement the repeat timer.
    ///
    /// ```c
    /// if (flags & 0x40) {
    ///     if (byte_22c != 0) byte_22c -= 1;
    ///     flags &= 0xffbf;
    /// }
    /// ```
    pub fn a81_clear_repeat_flag(&mut self) {
        if self.flags & 0x40 != 0 {
            if self.byte_22c != 0 {
                self.byte_22c -= 1;
            }
            self.flags &= 0xffbf;
        }
    }

    /// `0x00635b90`: clear flag `0x800` and decrement `+0x213`.
    ///
    /// ```c
    /// flags &= 0xf7ff;
    /// if (byte_213 != 0) byte_213 -= 1;
    /// ```
    pub fn a81_clear_flag_800(&mut self) {
        self.flags &= 0xf7ff;
        if self.byte_213 != 0 {
            self.byte_213 -= 1;
        }
    }

    /// `0x00635dd0`: add a three-word cost to the remaining accumulator.
    ///
    /// The three values are the outputs of the out-of-batch
    /// `FUN_0067d2d0`, so they are supplied by the caller.  The additions are
    /// 32-bit wrapping `i32` additions.
    pub fn a81_add_cost(&mut self, cost: [i32; 3]) {
        for i in 0..3 {
            self.remaining[i] = self.remaining[i].wrapping_add(cost[i]);
        }
    }

    /// `0x00635e20`: add a cost chosen by the queue kind.
    ///
    /// `kind == 5` and `kind == 6` call the two-word cost helpers and then
    /// use `param_2 >> 24`, where `param_2` still holds the constant `5`/`6`,
    /// so the third word is always zero for those kinds.  Every other kind
    /// calls `FUN_0067d2d0`, whose byte output lands in the high byte of
    /// `param_2` and supplies the third word.
    pub fn a81_add_cost_by_kind(&mut self, kind: u32, cost: [i32; 3]) {
        let cost = match kind {
            5 | 6 => [cost[0], cost[1], 0],
            _ => cost,
        };
        self.a81_add_cost(cost);
    }

    /// `0x00635170`: consume the pending count.
    ///
    /// ```c
    /// if (pending == 0) return 0;
    /// *out = pending - 1;
    /// pending = 0;
    /// return 1;
    /// ```
    pub fn a81_take_pending(&mut self) -> Option<u16> {
        if self.pending == 0 {
            return None;
        }
        let taken = self.pending - 1;
        self.pending = 0;
        Some(taken)
    }

    /// `0x00635280`: set or clear flag `0x200`.
    pub fn a81_set_flag_200(&mut self, on: bool) {
        if on {
            self.flags |= 0x200;
        } else {
            self.flags &= 0xfdff;
        }
    }

    /// `0x006353d0`: `+0x211 = 1`.
    pub fn a81_mark_started(&mut self) {
        self.byte_211 = 1;
    }

    /// `0x00635410`: `+0x211 != 0`, the byte variant of the started flag.
    pub fn a81_started_byte(&self) -> bool {
        self.byte_211 != 0
    }

    /// `0x00635430`: set flag `0x20`.
    pub fn a81_set_flag_20(&mut self) {
        self.flags |= 0x20;
    }

    /// `0x00635470`: add to the first two charged words.
    pub fn a81_charge_offset(&mut self, first: i32, second: i32) {
        self.charged[0] = self.charged[0].wrapping_add(first);
        self.charged[1] = self.charged[1].wrapping_add(second);
    }

    /// `0x00635870`: set flag `0x100`.
    pub fn a81_set_flag_100(&mut self) {
        self.flags |= 0x100;
    }

    /// `0x00635c60`: has the start tick aged by more than `0x1d` ticks?
    ///
    /// ```c
    /// if (start_tick == 0) return true;
    /// return 0x1d < (uint)(now - start_tick);
    /// ```
    pub fn a81_start_elapsed(&self, now: u32) -> bool {
        if self.start_tick == 0 {
            return true;
        }
        now.wrapping_sub(self.start_tick as u32) > 0x1d
    }

    /// `0x00635fc0`: copy the four words at `+0x4d8`.
    ///
    /// ```c
    /// out[0] = *(u32 *)(&DAT_00faa6d0 + index * 0x4e8);
    /// out[1] = *(u32 *)(&DAT_00faa6d4 + index * 0x4e8);
    /// out[2] = *(u32 *)(&DAT_00faa6d8 + index * 0x4e8);
    /// out[3] = *(u32 *)(&DAT_00faa6dc + index * 0x4e8);
    /// ```
    pub fn a81_tail_words(&self) -> [u32; 4] {
        self.tail_words
    }
}

// ---------------------------------------------------------------------------
// 0x006344c0 - cost query dispatch
// ---------------------------------------------------------------------------

/// Which cost helper `0x006344c0` selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A81CostQuery {
    /// `param_2 == 5`: `FUN_0067d320(param_1, param_3, param_4, param_5)`.
    Leading,
    /// `param_2 == 6`: `FUN_0067d2a0(param_3, param_4, param_5)`.
    Plain,
    /// Everything else: `FUN_0067d2d0(param_3, param_4, param_5, byte)`.
    WithByte,
}

/// `0x006344c0`: the cost-query dispatch.
///
/// ```c
/// if (kind == 5) { *byte = 0; FUN_0067d320(...); return; }
/// if (kind == 6) { *byte = 0; FUN_0067d2a0(...); return; }
/// FUN_0067d2d0(...);                  // writes the byte
/// ```
///
/// The three callees are out of the batch; the recovered content is the
/// branch and the pre-clear of the byte out-parameter on the first two paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A81CostQueryPlan {
    /// The selected helper.
    pub query: A81CostQuery,
    /// `true` when the row writes zero to the byte out-parameter itself.
    pub clears_byte: bool,
}

/// See [`A81CostQueryPlan`].
pub fn a81_cost_query_plan(kind: u32) -> A81CostQueryPlan {
    match kind {
        5 => A81CostQueryPlan {
            query: A81CostQuery::Leading,
            clears_byte: true,
        },
        6 => A81CostQueryPlan {
            query: A81CostQuery::Plain,
            clears_byte: true,
        },
        _ => A81CostQueryPlan {
            query: A81CostQuery::WithByte,
            clears_byte: false,
        },
    }
}

// ---------------------------------------------------------------------------
// 0x00634520 / 0x00634b30 - 20-slot append
// ---------------------------------------------------------------------------

/// Number of `u16` slots in the append target.
pub const A81_APPEND_SLOTS: usize = 0x14;

/// Guard of `0x00634520`: the second argument must be below `0xb`.
pub const A81_APPEND_COUNT_LIMIT: u32 = 0xb;

/// Guard of `0x00634520`: the written value must be below `0xe4`.
pub const A81_APPEND_VALUE_LIMIT: u32 = 0xe4;

/// `0x00634520`: append `value + 1` up to `count` times to a zero-terminated
/// `u16` array.
///
/// ```c
/// if (count < 0xb && value < 0xe4) {
///     i = 0;
///     while (slots[i] != 0) {
///         if (i > 0x13) return;
///         i += 1;
///     }
///     for (; count != 0 && (count--, i < 0x14); i++)
///         slots[i] = (short)value + 1;
/// }
/// ```
///
/// The same argument is the guard bound and the write count in the shipped
/// decompilation, so it is reproduced with one `count` parameter.  A read at
/// index `0x14` falls past the logical array; the model treats it as a zero
/// terminator, which produces the same observable result (the write loop is
/// already bounded by `0x14`).
///
/// Returns the number of slots written; the row itself returns nothing.
pub fn a81_queue_append(slots: &mut [u16], count: u32, value: u32) -> usize {
    if count >= A81_APPEND_COUNT_LIMIT || value >= A81_APPEND_VALUE_LIMIT {
        return 0;
    }
    let mut index = 0usize;
    loop {
        let slot = slots.get(index).copied().unwrap_or(0);
        if slot == 0 {
            break;
        }
        if index > 0x13 {
            return 0;
        }
        index += 1;
    }
    let mut remaining = count;
    let mut written = 0usize;
    while remaining != 0 {
        remaining -= 1;
        if index >= A81_APPEND_SLOTS {
            break;
        }
        slots[index] = (value as u16).wrapping_add(1);
        written += 1;
        index += 1;
    }
    written
}

/// `0x00634b30`: the bounded wrapper around `0x00634520`.
///
/// ```c
/// if (player < 8 && value < 0xe4) FUN_00634520();
/// ```
///
/// The identity of the target array is not in the decompilation (the wrapper
/// reaches `0x00634520` with already-prepared registers); the caller passes
/// the slot array it wants written.  `FUN_00634b60` and `FUN_00634b90` are
/// byte-identical copies with other hidden targets.
pub fn a81_append_checked(player: u32, count: u32, value: u32, slots: &mut [u16]) -> usize {
    if player < A81_RECORD_COUNT && value < A81_APPEND_VALUE_LIMIT {
        a81_queue_append(slots, count, value)
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// 0x00634570 - gated heap push
// ---------------------------------------------------------------------------

/// Kinds that consult the supply table before being queued: `1, 2, 4, 8`.
pub const A81_GATED_KINDS: [u8; 4] = [1, 2, 4, 8];

/// Pack the heap key word.
///
/// `CONCAT22((short)param_3, CONCAT11(param_2, param_4))`, i.e.
/// `(unit_type & 0xffff) << 16 | (kind as u32) << 8 | param_4`.
pub fn a81_pack_heap_key(kind: u8, unit_type: u32, param4: u8) -> u32 {
    ((unit_type & 0xffff) << 16) | ((kind as u32) << 8) | param4 as u32
}

/// `0x00634570` supply gate: `true` when the row abandons the push.
///
/// ```c
/// if (kind in {1,2,4,8} && player < 8 && unit_type < 0xe4) {
///     slot = supply_table[player][unit_type];
///     if (slot != 0) {
///         if (slot == 0xff) return;                 // infinite supply
///         available = FUN_00695be0(unit_type, player);
///         if (unit_type == 5) available += FUN_00695be0(0x1e, player);
///         if (slot <= available) return;            // not enough supply
///     }
/// }
/// ```
///
/// `slot` is the byte at `DAT_00faa5e8 + player * 0x4e8 + unit_type` and
/// `available` is the caller-supplied result of the out-of-batch supply
/// counter (`FUN_00695be0`), already summed with the `0x1e` term when the
/// unit type is `5`.  When the kind is not gated, or the type is out of the
/// table, the gate does not apply and the push proceeds.
pub fn a81_supply_gate_blocks(
    kind: u8,
    player: u32,
    unit_type: u32,
    slot: u8,
    available: u32,
) -> bool {
    if !A81_GATED_KINDS.contains(&kind) {
        return false;
    }
    if player >= A81_RECORD_COUNT || unit_type >= A81_APPEND_VALUE_LIMIT {
        return false;
    }
    if slot == 0 {
        return false;
    }
    if slot == 0xff {
        return true;
    }
    slot as u32 <= available
}

/// `0x00634570`: queue one entry into the per-player heap.
///
/// The row gates kinds `1, 2, 4, 8` on the supply table, then quietly
/// returns; every other kind (and every gated push whose supply is
/// insufficient) packs `{kind, unit_type, param4}` into the key, stores the
/// payload, and sifts up on the key byte.  A full heap overwrites slot `0x3e`.
///
/// The row performs no `player < 8` check on the push path itself; the caller
/// owns the record selection, so this model writes one heap only.
///
/// Returns the slot used, or `None` when the supply gate abandoned the push.
pub fn a81_queue_push(
    heap: &mut A81QueueHeap,
    player: u32,
    kind: u8,
    unit_type: u32,
    param4: u8,
    value: u32,
    slot: u8,
    available: u32,
) -> Option<usize> {
    if a81_supply_gate_blocks(kind, player, unit_type, slot, available) {
        return None;
    }
    Some(heap.a81_push(a81_pack_heap_key(kind, unit_type, param4), value))
}

// ---------------------------------------------------------------------------
// 0x006347e0 / 0x0063484c - refund the head entry
// ---------------------------------------------------------------------------

/// Entry types that `0x006347e0` does not refund: the expansion pseudo-types.
pub const A81_EXPANSION_TYPES: [u16; 5] = [0x2c, 0x67, 0x3e, 0x44, 0x3f];

/// `0x006347e0`: refund the cost of the current head entry.
///
/// ```c
/// head = *(short *)(&DAT_00faa212 + player * 0x4e8);
/// if (head not in {0x2c, 0x67, 0x3e, 0x44, 0x3f}) {
///     FUN_006344c0(player, kind_byte, head, &cost0, &cost1, &cost2);
///     remaining[0] = saturating_sub(remaining[0], cost0);
///     ...
///     if (param_2 != 0) {
///         charged[0] -= cost0; charged[1] -= cost1; charged[2] -= cost2;
///     }
/// }
/// ```
///
/// `cost` stands for the three outputs of `0x006344c0`.  Returns `true` when
/// the record was refunded.
pub fn a81_refund_head(queue: &mut A81BuildQueue, cost: [i32; 3], charge_back: bool) -> bool {
    if A81_EXPANSION_TYPES.contains(&queue.word_21a) {
        return false;
    }
    queue.a81_refund(cost, charge_back);
    true
}

/// `0x0063484c`: the split continuation of `0x006347e0`.
///
/// The compiler moved the six accumulator stores of `0x006347e0` into this
/// funclet, which addresses the record through `ESI` and the cost words
/// through the caller's frame.  The first saturating subtraction takes its
/// underflow value from `unaff_EDI` (zero in the recovered frame), so the
/// arithmetic matches [`a81_refund_head`] exactly; the charged words are
/// written only when the caller's flag word at `EBP + 0xc` is non-zero.
pub fn a81_refund_head_split(queue: &mut A81BuildQueue, cost: [i32; 3], charge_back: bool) -> bool {
    a81_refund_head(queue, cost, charge_back)
}

/// `0x00635150`: refund the head with the charged accumulator and pop it.
///
/// ```c
/// FUN_006347e0(player, 1);
/// FUN_006348c0();
/// ```
pub fn a81_finish_order(queue: &mut A81BuildQueue, cost: [i32; 3]) -> bool {
    let refunded = a81_refund_head(queue, cost, true);
    queue.heap.a81_pop();
    refunded
}

/// `0x00635240`: refund the head without touching the charged accumulator,
/// then pop it.
///
/// ```c
/// FUN_006347e0(player, 0);
/// FUN_006348c0();
/// ```
pub fn a81_cancel_order(queue: &mut A81BuildQueue, cost: [i32; 3]) -> bool {
    let refunded = a81_refund_head(queue, cost, false);
    queue.heap.a81_pop();
    refunded
}

// ---------------------------------------------------------------------------
// 0x006349a0 / 0x006349f0 - flag and membership scans
// ---------------------------------------------------------------------------

/// `0x006349a0`: bit 0 of the flag word, with the `player < 8` bound.
///
/// ```c
/// if (7 < param_1) return 0;
/// return *(u16 *)(&DAT_00faa410 + param_1 * 0x4e8) & 1;
/// ```
pub fn a81_flag_bit0(player: u32, flags: u16) -> u16 {
    if player >= A81_RECORD_COUNT {
        return 0;
    }
    flags & 1
}

/// `0x006349f0`: does the zero-terminated `u16` array contain `id + 1`?
///
/// ```c
/// v = *slots;
/// if (v != 0) {
///     do {
///         slots++;
///         if (v == id + 1) return 1;
///         v = *slots;
///     } while (v != 0);
/// }
/// return 0;
/// ```
pub fn a81_array_contains_id(slots: &[u16], id: u32) -> bool {
    for &slot in slots {
        if slot == 0 {
            return false;
        }
        if slot as u32 == id.wrapping_add(1) {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// 0x00634a40 - per-player slot address
// ---------------------------------------------------------------------------

/// `0x00634a40`: `&DAT_00faa520 + index * 0x4e8`.
///
/// The rows `FUN_00634a60`, `FUN_00634a80`, `FUN_00634aa0`, `FUN_006352c0`,
/// `FUN_006352e0`, `FUN_00635300`, `FUN_00635320` and `FUN_00635f80` are the same
/// expression with bases `0x00faa4f8`, `0x00faa4d0`, `0x00faa4a8`,
/// `0x00faa5c0`, `0x00faa598`, `0x00faa570`, `0x00faa548` and `0x00faa428`.
/// The arithmetic is a 32-bit wrapping multiply/add, as in the shipped code.
pub fn a81_slot_328(index: i32) -> u32 {
    0x00fa_a520u32.wrapping_add((index as u32).wrapping_mul(A81_RECORD_STRIDE))
}

// ---------------------------------------------------------------------------
// 0x00634bc0 - threshold probe
// ---------------------------------------------------------------------------

/// `0x00634bc0`: do the three charged accumulators meet the probe thresholds?
///
/// ```c
/// if (FUN_0067d0f0(param_1, param_2, 1, 1) == 0) return false;
/// if (need0 > charged[0]) return false;
/// if (need1 > charged[1]) return false;
/// return need2 <= charged[2];
/// ```
///
/// `probe` is the out-of-batch `FUN_0067d0f0` result; `need` are
/// `FUN_0067d290`/`FUN_0067d280`/`FUN_0067d270`.  The comparisons are
/// unsigned and evaluated in the shipped order.
pub fn a81_thresholds_ok(probe: bool, need: [u32; 3], charged: [i32; 3]) -> bool {
    if !probe {
        return false;
    }
    if need[0] > charged[0] as u32 {
        return false;
    }
    if need[1] > charged[1] as u32 {
        return false;
    }
    need[2] <= charged[2] as u32
}

// ---------------------------------------------------------------------------
// 0x00634d10 - repeat count
// ---------------------------------------------------------------------------

/// `0x00634d10`: the byte at `+0x223`.
pub fn a81_read_repeat(queue: &A81BuildQueue) -> u8 {
    queue.repeat
}

// ---------------------------------------------------------------------------
// 0x006350f0 - queue kind selection
// ---------------------------------------------------------------------------

/// `0x006350f0`: choose the queue kind for a training/build order.
///
/// ```c
/// if (param_3 == 0) {
///     flags = DAT_00f8fa08[param_4];       // unit-type flag word
///     if (flags & 8) kind = 4;
///     else kind = (flags & 2) * 2 | 3;     // 7 or 3
/// } else {
///     kind = (param_3 != 1) + 5;           // 5 or 6
/// }
/// ```
///
/// `selector` is the row's `param_3` and `unit_flags` the word pulled from
/// the unit-type flags table for `param_4`.
pub fn a81_queue_kind_for_training(selector: i32, unit_flags: u32) -> u32 {
    if selector == 0 {
        if unit_flags & 8 != 0 {
            4
        } else {
            (unit_flags & 2) * 2 | 3
        }
    } else if selector == 1 {
        5
    } else {
        6
    }
}

// ---------------------------------------------------------------------------
// flag and word accessors
// ---------------------------------------------------------------------------

/// `0x006351b0`: the word at `+0x21a`.
pub fn a81_read_word_21a(queue: &A81BuildQueue) -> u16 {
    queue.word_21a
}

/// `0x006351d0`: bit `0x200` of the flag word.
pub fn a81_flag_200(flags: u16) -> u16 {
    flags & 0x200
}

/// `0x00635260`: bit `0x80` of the flag word.
pub fn a81_flag_80(flags: u16) -> u16 {
    flags & 0x80
}

/// `0x00635490`: bit `0x100` of the flag word.
pub fn a81_flag_100(flags: u16) -> u16 {
    flags & 0x100
}

/// `0x00635720`: bit `0x20` of the flag word.
pub fn a81_flag_20(flags: u16) -> u16 {
    flags & 0x20
}

/// `0x00635590`: the word at `+0x21c`, with the `player < 8` bound.
pub fn a81_read_word_21c(player: u32, queue: &A81BuildQueue) -> u16 {
    if player >= A81_RECORD_COUNT {
        return 0;
    }
    queue.word_21c
}

// ---------------------------------------------------------------------------
// 0x006355b0 - extend the two stall timers
// ---------------------------------------------------------------------------

/// How long a timer must have run before it can be extended.
pub const A81_STALL_THRESHOLD: i32 = 500;

/// The extension added by `0x006355b0`.
pub const A81_STALL_EXTENSION: i32 = 2000;

/// `0x006355b0`: bump both global timers when they are still below 500.
///
/// ```c
/// if (timer0[player] < 500) timer0[player] += 2000;
/// if (timer1[player] < 500) timer1[player] += 2000;
/// ```
///
/// The two arrays are `DAT_00f063cc ^ 0x10fae560` at `+0` and `+0x30`.  The
/// two words are supplied and returned independently.
pub fn a81_extend_stall(first: i32, second: i32) -> (i32, i32) {
    let first = if first < A81_STALL_THRESHOLD {
        first + A81_STALL_EXTENSION
    } else {
        first
    };
    let second = if second < A81_STALL_THRESHOLD {
        second + A81_STALL_EXTENSION
    } else {
        second
    };
    (first, second)
}

// ---------------------------------------------------------------------------
// 0x00635600 - cost split
// ---------------------------------------------------------------------------

/// The two bytes written by `0x00635600` (`param_2`, `param_3`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A81CostSplit {
    /// The byte written through `param_2`.
    pub first: u8,
    /// The byte written through `param_3`.
    pub second: u8,
}

/// `0x00635600`: split the remaining cost into a `'d'`-based pair.
///
/// ```c
/// gas = rem[1];
/// if (gas == 0) { *p3 = 0; *p2 = 'd'; return; }
/// mins = rem[0];
/// if (mins == 0) { *p3 = 'd'; *p2 = 0; return; }
/// d0 = mins - timer0;  d1 = gas - timer1;
/// if (d0 < 1) {
///     if (gas == timer1 || d1 < 0) goto split;
/// } else if (d1 < 1) {
///     *p2 = 'd'; *p3 = 0; return;
/// }
/// if (mins == timer0 || d0 < 0) { *p2 = 0; *p3 = 'd'; return; }
/// split:
/// c = (char)((uint)(gas * 100) / (uint)(mins + gas));
/// *p3 = c;
/// *p2 = 'd' - c;
/// ```
///
/// `'d'` is `0x64`; the final subtraction is byte arithmetic.  The division
/// is unsigned after the two `i32` products/additions, as in the row.
pub fn a81_cost_split(remaining: [i32; 2], timers: [i32; 2]) -> A81CostSplit {
    let gas = remaining[1];
    if gas == 0 {
        return A81CostSplit {
            first: b'd',
            second: 0,
        };
    }
    let mins = remaining[0];
    if mins == 0 {
        return A81CostSplit {
            first: 0,
            second: b'd',
        };
    }

    let delta0 = mins.wrapping_sub(timers[0]);
    let delta1 = gas.wrapping_sub(timers[1]);

    let mut split = false;
    if delta0 < 1 {
        if gas == timers[1] || delta1 < 0 {
            split = true;
        }
    } else if delta1 < 1 {
        return A81CostSplit {
            first: b'd',
            second: 0,
        };
    }

    if !split && (mins == timers[0] || delta0 < 0) {
        return A81CostSplit {
            first: 0,
            second: b'd',
        };
    }

    let percent = (gas.wrapping_mul(100) as u32) / (mins.wrapping_add(gas) as u32);
    let percent = percent as u8;
    A81CostSplit {
        first: b'd'.wrapping_sub(percent),
        second: percent,
    }
}

// ---------------------------------------------------------------------------
// 0x00635820 - completion tick
// ---------------------------------------------------------------------------

/// `0x00635820`: has the record's completion tick been reached?
///
/// ```c
/// if (7 < param_1) return false;
/// return *(int *)(&DAT_00faa40c + param_1 * 0x4e8)
///      + (uint)(byte)(&DAT_00faa40a)[param_1 * 0x4e8] * 0x3c
///      <= *(uint *)(clock + 0xe608);
/// ```
///
/// The `i32` at `+0x214` and the `byte * 0x3c` term are widened through
/// `u32`, so the sum wraps like the shipped comparison.
pub fn a81_ready_at(player: u32, queue: &A81BuildQueue, now: u32) -> bool {
    if player >= A81_RECORD_COUNT {
        return false;
    }
    (queue.tick_214 as u32).wrapping_add((queue.byte_212 as u32).wrapping_mul(0x3c)) <= now
}

// ---------------------------------------------------------------------------
// 0x006359d0 - next player slot
// ---------------------------------------------------------------------------

/// `0x006359d0`: advance the current-player index and wrap at 8.
///
/// ```c
/// DAT_00fac938 += 1;
/// if (DAT_00fac938 == 8) DAT_00fac938 = 0;
/// FUN_00748d40(DAT_00fac938);
/// ```
///
/// The byte increment is reproduced without an `& 7` mask because the shipped
/// store only compares against 8.
pub fn a81_next_slot(current: u8) -> u8 {
    let next = current.wrapping_add(1);
    if next == 8 { 0 } else { next }
}

// ---------------------------------------------------------------------------
// 0x00635d10 - supply sufficiency
// ---------------------------------------------------------------------------

/// `0x00635d10`: is the supply table slot satisfied by the available supply?
///
/// ```c
/// if (player < 8 && type < 0xe4) {
///     slot = supply_table[player][type];
///     if (slot != 0) {
///         if (slot == 0xff) return true;
///         available = FUN_00695be0(type, player);
///         if (type == 5) available += FUN_00695be0(0x1e, player);
///         return slot <= available;
///     }
/// }
/// return false;
/// ```
///
/// `slot` and `available` have the same meaning as in
/// [`a81_supply_gate_blocks`]: the table byte and the caller-computed supply
/// counter sum.  Unlike the gate, slot `0xff` (infinite supply) returns
/// `true`.
pub fn a81_supply_sufficient(player: u32, unit_type: u32, slot: u8, available: u32) -> bool {
    if player >= A81_RECORD_COUNT || unit_type >= A81_APPEND_VALUE_LIMIT {
        return false;
    }
    if slot == 0 {
        return false;
    }
    if slot == 0xff {
        return true;
    }
    slot as u32 <= available
}

// ---------------------------------------------------------------------------
// Not translated
// ---------------------------------------------------------------------------
//
// The remaining 57 `decompiled_uncited` rows of the window are not translated
// here:
//
// * `FUN_006340c0` - SEH frame plus a fixed 63-iteration call to the
//   out-of-batch `FUN_00535250`; no recoverable arithmetic.
// * `FUN_006341c2` - `__finally` funclet: restores `ExceptionList` and jumps to
//   the dispatcher.
// * `FUN_006343c0` - SEH frame plus an out-of-batch throw tagged with the
//   allocation size `0x2740`.
// * `FUN_006346b0` - queue-expansion dispatch; every branch calls the
//   out-of-batch cost lookup `FUN_0067d2d0` and re-enters the translated push.
// * `FUN_0063499d`, `FUN_00635256` - `__break` (`int3` / `swi(3)`) padding.
// * `FUN_00634a09` - the same scan as `0x006349f0` with the first element in
//   `EAX` and the cursor in `ECX`.
// * `FUN_00634a60`, `FUN_00634a80`, `FUN_00634aa0` - byte-identical slot accessors
//   to `0x00634a40` with bases `0x00faa4f8`, `0x00faa4d0`, `0x00faa4a8`.
// * `FUN_00634a7b` - one-byte overlapping row that spills the x87 environment
//   words and then returns `&DAT_00faa4d0 + index * 0x4e8`; not coherent.
// * `FUN_00634ab4` - `out(0x53, ...)` port I/O plus x87 stack debris; not a
//   coherent function.
// * `FUN_00634ac0` - probes `func_0x005b3b60` and calls an out-of-batch throw.
// * `FUN_00634b60`, `FUN_00634b90` - byte-identical checked appends to
//   `0x00634b30` with different hidden target arrays.
// * `FUN_00634eb0` - cost from the out-of-batch `FUN_0067d2d0`, then the
//   translated push with kind `8`.
// * `FUN_00634f10` - flag guard, out-of-batch cost, translated push, then
//   `FUN_006346b0`.
// * `FUN_00634fc0` - game-clock guard plus out-of-batch `FUN_00656f80` and
//   `FUN_0067d2d0`.
// * `FUN_00635060` - the guard calls the out-of-batch `FUN_00638c00`; the kind
//   arithmetic is already covered by `0x006350f0`.
// * `FUN_006352c0`, `FUN_006352e0`, `FUN_00635300`, `FUN_00635320` - slot accessors
//   identical to `0x00634a40`.
// * `FUN_00635340`, `FUN_00635360`, `FUN_00635380` - single call to `0x00634520`
//   with all arguments already in registers; no recovered parameters.
// * `FUN_006353a0` - the same checked append as `0x00634b30`.
// * `FUN_00635450` - one-byte store (`+0x22d = 1`); left for the batch cap.
// * `FUN_006354b0` - requires out-of-batch `FUN_00638b00` and `FUN_00640a90`.
// * `FUN_00635518` - `out(0, ...)` port I/O, not a coherent function.
// * `FUN_006356b0` - out-of-batch `memset`; its `0x2740` extent is recorded as
//   `A81_RECORD_BYTES`.
// * `FUN_00635760` - out-of-batch throw (`FUN_005b4cd0`).
// * `FUN_0063576e`, `FUN_006357a7` - unit-slot funclets calling the out-of-batch
//   `func_0x005b0fb0` / `FUN_005b4bb0`.
// * `FUN_00635890` - constant `0`; left for the batch cap.
// * `FUN_006358d0`, `FUN_00635900`, `FUN_00635903` - `(flags >> 4 | 3) & 1` style
//   byte accessors; left for the batch cap.
// * `FUN_00635940`, `FUN_00635960`, `FUN_00635990`, `FUN_006359b0` - byte accessors;
//   left for the batch cap.
// * `FUN_00635a0a` - sixteen-call setup sequence; every callee is out of batch.
// * `FUN_00635b50`, `FUN_00635bc0` - single-byte stores; left for the batch cap.
// * `FUN_00635c00`, `FUN_00635c20`, `FUN_00635c40`, `FUN_00635ca0` - word/tick
//   accessors; left for the batch cap.
// * `FUN_00635d04` - overlapping disassembly with port I/O; not coherent.
// * `FUN_00635d90`, `FUN_00635eb0`, `FUN_00635ed0`, `FUN_00635f40`, `FUN_00635f80`,
//   `FUN_00635fa0` - flag/pointer/word rows; left for the batch cap.
// * `FUN_00635f76` - `swi(6)` plus a frame-relative address computation.

#[cfg(test)]
mod tests_agent81 {
    use super::*;

    // -- record geometry ----------------------------------------------------

    #[test]
    fn record_geometry_is_eight_times_0x4e8() {
        assert_eq!(A81_RECORD_BYTES, 0x2740);
        assert_eq!(A81_RECORD_BYTES, 8 * A81_RECORD_STRIDE);
        assert_eq!(A81_RECORD_BASE + A81_RECORD_BYTES, 0x00fa_c938);
    }

    // -- 0x006344c0 ---------------------------------------------------------

    #[test]
    fn cost_query_kind_selects_three_paths() {
        assert_eq!(
            a81_cost_query_plan(5),
            A81CostQueryPlan {
                query: A81CostQuery::Leading,
                clears_byte: true,
            }
        );
        assert_eq!(
            a81_cost_query_plan(6),
            A81CostQueryPlan {
                query: A81CostQuery::Plain,
                clears_byte: true,
            }
        );
        assert_eq!(
            a81_cost_query_plan(0x2b),
            A81CostQueryPlan {
                query: A81CostQuery::WithByte,
                clears_byte: false,
            }
        );
    }

    // -- 0x00634520 ---------------------------------------------------------

    #[test]
    fn append_writes_value_plus_one_after_the_terminator() {
        let mut slots = [0u16; A81_APPEND_SLOTS];
        slots[0] = 3;
        slots[1] = 4;
        assert_eq!(a81_queue_append(&mut slots, 3, 7), 3);
        assert_eq!(&slots[..6], &[3, 4, 8, 8, 8, 0]);
    }

    #[test]
    fn append_rejects_out_of_range_arguments() {
        let mut slots = [0u16; A81_APPEND_SLOTS];
        assert_eq!(a81_queue_append(&mut slots, 0xb, 1), 0);
        assert_eq!(a81_queue_append(&mut slots, 1, 0xe4), 0);
        assert_eq!(slots, [0u16; A81_APPEND_SLOTS]);

        // 0xe3 is the largest value that passes the guard; it writes 0xe4.
        assert_eq!(a81_queue_append(&mut slots, 1, 0xe3), 1);
        assert_eq!(slots[0], 0xe4);
    }

    #[test]
    fn append_stops_at_the_twentieth_slot() {
        let mut slots = [1u16; A81_APPEND_SLOTS];
        assert_eq!(a81_queue_append(&mut slots, 5, 2), 0);
        assert_eq!(slots, [1u16; A81_APPEND_SLOTS]);

        let mut tail = [0u16; A81_APPEND_SLOTS];
        tail[..18].fill(1);
        tail[19] = 9;
        assert_eq!(a81_queue_append(&mut tail, 5, 2), 2);
        assert_eq!(&tail[18..], &[3, 3]);
    }

    // -- 0x00634b30 ---------------------------------------------------------

    #[test]
    fn append_checked_applies_the_player_bound() {
        let mut slots = [0u16; A81_APPEND_SLOTS];
        assert_eq!(a81_append_checked(8, 2, 1, &mut slots), 0);
        assert_eq!(slots, [0u16; A81_APPEND_SLOTS]);
        assert_eq!(a81_append_checked(7, 2, 1, &mut slots), 2);
        assert_eq!(&slots[..3], &[2, 2, 0]);
    }

    // -- 0x00634570 ---------------------------------------------------------

    #[test]
    fn supply_gate_only_applies_to_kinds_1_2_4_8() {
        // Ungated kinds never consult the slot.
        assert!(!a81_supply_gate_blocks(3, 0, 0, 0xff, 0));
        // Gated kind with an empty table slot proceeds.
        assert!(!a81_supply_gate_blocks(1, 0, 0, 0, 0));
        // Infinite supply always blocks the queue.
        assert!(a81_supply_gate_blocks(8, 0, 0, 0xff, 0));
        // Enough available supply blocks; one short proceeds.
        assert!(a81_supply_gate_blocks(2, 0, 5, 5, 5));
        assert!(!a81_supply_gate_blocks(2, 0, 5, 5, 4));
        // Out-of-range player/type disables the gate.
        assert!(!a81_supply_gate_blocks(4, 8, 0, 0xff, 0));
        assert!(!a81_supply_gate_blocks(4, 0, 0xe4, 0xff, 0));
    }

    #[test]
    fn heap_key_packs_type_kind_and_priority_byte() {
        assert_eq!(a81_pack_heap_key(2, 0x1234, 0x56), 0x1234_0256);
        assert_eq!(a81_pack_heap_key(0xff, 0xffff, 0xaa), 0xffff_ffaa);
    }

    #[test]
    fn queue_push_gates_then_stores_the_packed_key() {
        let mut heap = A81QueueHeap::default();
        assert_eq!(
            a81_queue_push(&mut heap, 0, 2, 0x10, 0x30, 0x99, 0, 0),
            Some(0)
        );
        assert_eq!(heap.count, 1);
        assert_eq!(
            heap.entries[0],
            A81HeapEntry {
                key: 0x0010_0230,
                value: 0x99
            }
        );

        // Slot 0xff (infinite supply) abandons the push.
        assert_eq!(
            a81_queue_push(&mut heap, 0, 1, 0x10, 0x40, 0x11, 0xff, 0),
            None
        );
        assert_eq!(heap.count, 1);
    }

    // -- heap ---------------------------------------------------------------

    fn heap_with(values: &[u8]) -> A81QueueHeap {
        let mut heap = A81QueueHeap::default();
        for &v in values {
            heap.a81_push(v as u32, v as u32);
        }
        heap
    }

    #[test]
    fn heap_push_sifts_up_on_the_priority_byte() {
        let heap = heap_with(&[5, 3, 9, 1, 7]);
        assert_eq!(heap.count, 5);
        assert_eq!(heap.entries[0].a81_priority(), 9);
        assert_eq!(heap.entries[1].a81_priority(), 7);
        assert_eq!(heap.entries[2].a81_priority(), 5);
    }

    #[test]
    fn heap_push_into_a_full_heap_reuses_slot_0x3e() {
        let mut heap = A81QueueHeap::default();
        heap.count = A81_HEAP_FULL;
        for entry in &mut heap.entries {
            entry.key = 1;
            entry.value = 1;
        }
        assert_eq!(heap.a81_push(0xff, 42), 0x3e);
        assert_eq!(heap.count, A81_HEAP_FULL);
        assert_eq!(heap.entries[0].value, 42);
        assert_eq!(heap.entries[0].a81_priority(), 0xff);
    }

    #[test]
    fn heap_pop_returns_entries_in_priority_order() {
        let mut heap = heap_with(&[5, 3, 9, 1, 7]);
        let mut order = Vec::new();
        while let Some(entry) = heap.a81_pop() {
            order.push(entry.value as u8);
        }
        assert_eq!(order, vec![9, 7, 5, 3, 1]);
        assert_eq!(heap.count, 0);
    }

    #[test]
    fn heap_pop_of_an_empty_heap_is_none() {
        let mut heap = A81QueueHeap::default();
        assert_eq!(heap.a81_pop(), None);
        assert_eq!(heap.count, 0);
    }

    #[test]
    fn sift_down_restores_the_heap_property() {
        let mut heap = A81QueueHeap::default();
        heap.count = 4;
        for (i, priority) in [1u8, 9, 8, 7].iter().enumerate() {
            heap.entries[i] = A81HeapEntry {
                key: *priority as u32,
                value: i as u32,
            };
        }
        heap.a81_sift_down_from(0);
        let priorities: Vec<u8> = heap.entries[..4].iter().map(|e| e.a81_priority()).collect();
        assert_eq!(priorities, vec![9, 7, 8, 1]);
    }

    // -- 0x006347e0 / 0x0063484c / 0x00635150 / 0x00635240 ----------------

    #[test]
    fn refund_head_saturates_remaining_and_wraps_charged() {
        let mut queue = A81BuildQueue {
            remaining: [5, 5, 5],
            charged: [1, 2, 3],
            word_21a: 0x2b,
            ..A81BuildQueue::default()
        };
        assert!(a81_refund_head(&mut queue, [10, 3, 600], true));
        assert_eq!(queue.remaining, [0, 2, 0]);
        assert_eq!(
            queue.charged,
            [1i32.wrapping_sub(10), -1, 3i32.wrapping_sub(600)]
        );
    }

    #[test]
    fn refund_head_skips_expansion_types() {
        for &head in &A81_EXPANSION_TYPES {
            let mut queue = A81BuildQueue {
                remaining: [9, 9, 9],
                charged: [9, 9, 9],
                word_21a: head,
                ..A81BuildQueue::default()
            };
            assert!(!a81_refund_head(&mut queue, [1, 2, 3], true));
            assert_eq!(queue.remaining, [9, 9, 9]);
            assert_eq!(queue.charged, [9, 9, 9]);
        }
    }

    #[test]
    fn refund_head_split_matches_the_main_row() {
        let mut queue = A81BuildQueue {
            remaining: [4, 5, 6],
            charged: [7, 8, 9],
            word_21a: 0x3d,
            ..A81BuildQueue::default()
        };
        assert!(a81_refund_head_split(&mut queue, [2, 1, 10], true));
        assert_eq!(queue.remaining, [2, 4, 0]);
        assert_eq!(queue.charged, [5, 7, -1]);
    }

    #[test]
    fn finish_and_cancel_refund_then_pop() {
        let mut finish = A81BuildQueue {
            remaining: [5, 5, 5],
            charged: [20, 20, 20],
            word_21a: 0x2b,
            ..A81BuildQueue::default()
        };
        finish.heap.a81_push(1, 1);
        assert!(a81_finish_order(&mut finish, [1, 1, 1]));
        assert_eq!(finish.remaining, [4, 4, 4]);
        assert_eq!(finish.charged, [19, 19, 19]);
        assert_eq!(finish.heap.count, 0);

        let mut cancel = A81BuildQueue {
            remaining: [5, 5, 5],
            charged: [20, 20, 20],
            word_21a: 0x2b,
            ..A81BuildQueue::default()
        };
        cancel.heap.a81_push(1, 1);
        assert!(a81_cancel_order(&mut cancel, [1, 1, 1]));
        assert_eq!(cancel.remaining, [4, 4, 4]);
        assert_eq!(cancel.charged, [20, 20, 20]);
        assert_eq!(cancel.heap.count, 0);
    }

    // -- 0x00635170 ---------------------------------------------------------

    #[test]
    fn take_pending_consumes_one_and_clears() {
        let mut queue = A81BuildQueue {
            pending: 6,
            ..A81BuildQueue::default()
        };
        assert_eq!(queue.a81_take_pending(), Some(5));
        assert_eq!(queue.pending, 0);
        assert_eq!(queue.a81_take_pending(), None);
    }

    // -- 0x006349a0 / 0x00634bc0 / 0x00635590 ------------------------------

    #[test]
    fn flag_bit0_is_bounded_by_player_count() {
        assert_eq!(a81_flag_bit0(0, 0xffff), 1);
        assert_eq!(a81_flag_bit0(7, 0x8000), 0);
        assert_eq!(a81_flag_bit0(8, 0xffff), 0);
    }

    #[test]
    fn thresholds_check_each_accumulator_in_order() {
        assert!(!a81_thresholds_ok(false, [0, 0, 0], [0, 0, 0]));
        assert!(!a81_thresholds_ok(true, [10, 0, 0], [9, 100, 100]));
        assert!(!a81_thresholds_ok(true, [10, 20, 0], [10, 19, 100]));
        assert!(!a81_thresholds_ok(true, [10, 20, 30], [10, 20, 29]));
        assert!(a81_thresholds_ok(true, [10, 20, 30], [10, 20, 30]));
    }

    #[test]
    fn word_accessors_return_the_recovered_slots() {
        let queue = A81BuildQueue {
            word_21a: 0x1234,
            word_21c: 0x5678,
            ..A81BuildQueue::default()
        };
        assert_eq!(a81_read_word_21a(&queue), 0x1234);
        assert_eq!(a81_read_word_21c(7, &queue), 0x5678);
        assert_eq!(a81_read_word_21c(8, &queue), 0);
    }

    // -- 0x00634a40 ---------------------------------------------------------

    #[test]
    fn slot_address_uses_the_stride() {
        assert_eq!(a81_slot_328(0), 0x00fa_a520);
        assert_eq!(a81_slot_328(2), 0x00fa_a520 + 2 * 0x4e8);
    }

    #[test]
    fn slot_address_wraps_like_a_32_bit_add() {
        assert_eq!(
            a81_slot_328(-1),
            0x00fa_a520u32.wrapping_sub(A81_RECORD_STRIDE)
        );
    }

    // -- 0x006349f0 ---------------------------------------------------------

    #[test]
    fn membership_scan_looks_for_id_plus_one() {
        let slots = [4u16, 9, 2, 0, 7];
        assert!(a81_array_contains_id(&slots, 1));
        assert!(a81_array_contains_id(&slots, 3));
        assert!(!a81_array_contains_id(&slots, 4));
        assert!(!a81_array_contains_id(&slots, 6));
        assert!(!a81_array_contains_id(&[], 0));
    }

    // -- 0x006350f0 ---------------------------------------------------------

    #[test]
    fn training_kind_uses_the_unit_flag_word() {
        assert_eq!(a81_queue_kind_for_training(0, 0), 3);
        assert_eq!(a81_queue_kind_for_training(0, 2), 7);
        assert_eq!(a81_queue_kind_for_training(0, 8), 4);
        assert_eq!(a81_queue_kind_for_training(0, 0xa), 4);
        assert_eq!(a81_queue_kind_for_training(1, 0), 5);
        assert_eq!(a81_queue_kind_for_training(2, 0), 6);
        assert_eq!(a81_queue_kind_for_training(-1, 0), 6);
    }

    // -- 0x00634d10 / 0x00634e70 -------------------------------------------

    #[test]
    fn repeat_count_round_trips() {
        let mut queue = A81BuildQueue::default();
        assert_eq!(a81_read_repeat(&queue), 0);
        queue.repeat = 9;
        assert_eq!(a81_read_repeat(&queue), 9);
    }

    #[test]
    fn clear_repeat_flag_only_touches_flag_0x40() {
        let mut queue = A81BuildQueue {
            flags: 0x40,
            byte_22c: 2,
            ..A81BuildQueue::default()
        };
        queue.a81_clear_repeat_flag();
        assert_eq!(queue.flags, 0);
        assert_eq!(queue.byte_22c, 1);

        // A zero repeat timer is not decremented past zero.
        queue.flags = 0x40;
        queue.byte_22c = 0;
        queue.a81_clear_repeat_flag();
        assert_eq!(queue.flags, 0);
        assert_eq!(queue.byte_22c, 0);

        // Without flag 0x40 nothing changes.
        queue.flags = 0xffbf;
        queue.byte_22c = 5;
        queue.a81_clear_repeat_flag();
        assert_eq!(queue.flags, 0xffbf);
        assert_eq!(queue.byte_22c, 5);
    }

    // -- flags --------------------------------------------------------------

    #[test]
    fn flag_accessors_extract_their_bits() {
        assert_eq!(a81_flag_200(0xffff), 0x200);
        assert_eq!(a81_flag_80(0xffff), 0x80);
        assert_eq!(a81_flag_100(0xffff), 0x100);
        assert_eq!(a81_flag_20(0xffff), 0x20);
        assert_eq!(a81_flag_200(0xfdff), 0);
    }

    #[test]
    fn set_flag_200_sets_and_clears() {
        let mut queue = A81BuildQueue::default();
        queue.a81_set_flag_200(true);
        assert_eq!(queue.flags, 0x200);
        queue.a81_set_flag_200(false);
        assert_eq!(queue.flags, 0);
        queue.flags = 0xffff;
        queue.a81_set_flag_200(false);
        assert_eq!(queue.flags, 0xfdff);
    }

    #[test]
    fn started_byte_round_trips() {
        let mut queue = A81BuildQueue::default();
        assert!(!queue.a81_started_byte());
        queue.a81_mark_started();
        assert!(queue.a81_started_byte());
        assert_eq!(queue.byte_211, 1);
    }

    #[test]
    fn set_flag_20_and_100_or_into_the_word() {
        let mut queue = A81BuildQueue::default();
        queue.a81_set_flag_20();
        queue.a81_set_flag_100();
        assert_eq!(queue.flags, 0x120);
    }

    // -- 0x00635470 ---------------------------------------------------------

    #[test]
    fn charge_offset_adds_two_words() {
        let mut queue = A81BuildQueue {
            charged: [1, 2, 3],
            ..A81BuildQueue::default()
        };
        queue.a81_charge_offset(4, 5);
        assert_eq!(queue.charged, [5, 7, 3]);
    }

    // -- 0x006355b0 ---------------------------------------------------------

    #[test]
    fn stall_extension_applies_per_timer() {
        assert_eq!(a81_extend_stall(400, 600), (2400, 600));
        assert_eq!(a81_extend_stall(500, 499), (500, 2499));
        assert_eq!(a81_extend_stall(-1, 2000), (1999, 2000));
    }

    // -- 0x00635600 ---------------------------------------------------------

    #[test]
    fn cost_split_handles_empty_accumulators() {
        // Gas empty: the second output is zero and the first is 'd'.
        assert_eq!(
            a81_cost_split([100, 0], [0, 0]),
            A81CostSplit {
                first: b'd',
                second: 0
            }
        );
        // Minerals empty with non-empty gas: the pair is inverted.
        assert_eq!(
            a81_cost_split([0, 100], [0, 0]),
            A81CostSplit {
                first: 0,
                second: b'd'
            }
        );
    }

    #[test]
    fn cost_split_percentage_matches_the_unsigned_division() {
        assert_eq!(
            a81_cost_split([300, 100], [200, 50]),
            A81CostSplit {
                first: 75,
                second: 25
            }
        );
        // delta0 < 1, but gas == timer1 takes the split path.
        assert_eq!(
            a81_cost_split([200, 100], [200, 100]),
            A81CostSplit {
                first: 67,
                second: 33
            }
        );
        // delta0 >= 1 while delta1 < 1 writes the second byte as zero.
        assert_eq!(
            a81_cost_split([300, 100], [200, 100]),
            A81CostSplit {
                first: b'd',
                second: 0
            }
        );
        // A negative delta0 falls into the (0, 'd') return.
        assert_eq!(
            a81_cost_split([300, 100], [400, 50]),
            A81CostSplit {
                first: 0,
                second: b'd'
            }
        );
    }

    // -- 0x00635820 / 0x00635c60 -------------------------------------------

    #[test]
    fn ready_at_adds_the_seconds_multiplier() {
        let queue = A81BuildQueue {
            tick_214: 1000,
            byte_212: 2,
            ..A81BuildQueue::default()
        };
        assert!(a81_ready_at(0, &queue, 1120));
        assert!(!a81_ready_at(0, &queue, 1119));
        assert!(!a81_ready_at(8, &queue, 1120));
    }

    #[test]
    fn start_elapsed_waits_29_ticks_and_wraps_unsigned() {
        let queue = A81BuildQueue {
            start_tick: 100,
            ..A81BuildQueue::default()
        };
        assert!(!queue.a81_start_elapsed(129));
        assert!(queue.a81_start_elapsed(130));
        // Unsigned subtraction wraps, so a clock behind the start counts as
        // elapsed.
        assert!(queue.a81_start_elapsed(50));
        // A zero start tick is always ready.
        let fresh = A81BuildQueue::default();
        assert!(fresh.a81_start_elapsed(0));
    }

    // -- 0x006359d0 ---------------------------------------------------------

    #[test]
    fn next_slot_wraps_at_eight() {
        assert_eq!(a81_next_slot(0), 1);
        assert_eq!(a81_next_slot(6), 7);
        assert_eq!(a81_next_slot(7), 0);
        assert_eq!(a81_next_slot(200), 201);
    }

    // -- 0x00635b90 ---------------------------------------------------------

    #[test]
    fn clear_flag_800_decrements_the_counter() {
        let mut queue = A81BuildQueue {
            flags: 0x800,
            byte_213: 2,
            ..A81BuildQueue::default()
        };
        queue.a81_clear_flag_800();
        assert_eq!(queue.flags, 0);
        assert_eq!(queue.byte_213, 1);

        queue.flags = 0xffff;
        queue.byte_213 = 0;
        queue.a81_clear_flag_800();
        assert_eq!(queue.flags, 0xf7ff);
        assert_eq!(queue.byte_213, 0);
    }

    // -- 0x00635d10 ---------------------------------------------------------

    #[test]
    fn supply_sufficiency_distinguishes_infinite_supply() {
        assert!(!a81_supply_sufficient(8, 0, 0xff, 0));
        assert!(!a81_supply_sufficient(0, 0xe4, 0xff, 0));
        assert!(!a81_supply_sufficient(0, 0, 0, 100));
        assert!(a81_supply_sufficient(0, 0, 0xff, 0));
        assert!(a81_supply_sufficient(0, 5, 5, 5));
        assert!(!a81_supply_sufficient(0, 5, 5, 4));
    }

    // -- 0x00635dd0 / 0x00635e20 -------------------------------------------

    #[test]
    fn add_cost_wraps_all_three_words() {
        let mut queue = A81BuildQueue {
            remaining: [1, 2, 3],
            ..A81BuildQueue::default()
        };
        queue.a81_add_cost([10, 20, 30]);
        assert_eq!(queue.remaining, [11, 22, 33]);
        queue.a81_add_cost([i32::MAX, 0, 0]);
        assert_eq!(queue.remaining[0], 11i32.wrapping_add(i32::MAX));
    }

    #[test]
    fn add_cost_by_kind_zeroes_the_third_word_for_kinds_5_and_6() {
        let mut queue = A81BuildQueue::default();
        queue.a81_add_cost_by_kind(5, [10, 20, 30]);
        assert_eq!(queue.remaining, [10, 20, 0]);

        queue.a81_add_cost_by_kind(6, [1, 2, 3]);
        assert_eq!(queue.remaining, [11, 22, 0]);

        queue.a81_add_cost_by_kind(0x2b, [1, 2, 3]);
        assert_eq!(queue.remaining, [12, 24, 3]);
    }

    // -- 0x00635fc0 ---------------------------------------------------------

    #[test]
    fn tail_words_are_copied_verbatim() {
        let queue = A81BuildQueue {
            tail_words: [0x1122_3344, 0x5566_7788, 0x99aa_bbcc, 0xddee_ff00],
            ..A81BuildQueue::default()
        };
        assert_eq!(
            queue.a81_tail_words(),
            [0x1122_3344, 0x5566_7788, 0x99aa_bbcc, 0xddee_ff00]
        );
    }
}

// ===========================================================================
// Merged from src/translated/agent82.rs
// ===========================================================================
// Translation batch 82.  Only this file is edited by the batch.
//
// Batch 82 owns `0x00636000..=0x00637fff`.  The ledger lists 41
// `decompiled_uncited` rows in that window plus one `thunk` row.  Thirty-one
// rows are translated below; the eleven rows that are not are listed at the
// end with their reason.
//
// The window is the per-player order/heap and command-card state used by the
// client's order card and its card-node lists.  The record stride is `0x4e8`
// (the same record copied by `FUN_00633df0` in batch 80), indexed by an eight-
// bit player/owner number.  Recovered layout:
//
// | offset | words |
// |--------|-------|
// | `+0x000` | `0x3f` order-heap entries, eight bytes each: priority `u8`, kind `u8`, `u16` order id, `u32` payload |
// | `+0x1f8` | order-heap count, one byte |
// | `+0x200` | `u16` flags (`0x20` selects the longer retry, `0x40` marks a pending order) |
// | `+0x202` | top entry's order id (`u16`) |
// | `+0x206` | `u16` pending card word, always stored as `value + 1` |
// | `+0x208` | `u16` popped order word, also biased by one |
// | `+0x20b` | byte counter decremented by `FUN_00634e70` |
// | `+0x210` | `u32` timer (wrap-around tick counter) |
// | `+0x218` | `0x3f` `u16` card slots |
// | `+0x4c0` | four dwords copied by `0x006362d0` |
//
// The card nodes are 44-byte intrusive-list records:
//
// | offset | words |
// |--------|-------|
// | `+0x00` | next index |
// | `+0x04` | previous index |
// | `+0x08` | tag byte (`3` is the slot-card node) |
// | `+0x09` | five bytes used by the slot helpers; `0x637be0` also reads a dword timer at `+0x0c` |
// | `+0x0e` | two trailing bytes (the timer's high half) |
// | `+0x10` | card/unit word |
// | `+0x14` | owner word |
// | `+0x18` | five slot-value dwords |
//
// Address to function:
//
// * `0x00636130` [`a82_order_timer_expired`]
// * `0x006361b0` [`a82_mark_card_slot`]
// * `0x00636290` [`a82_begin_order_timer`]
// * `0x006362d0` [`a82_store_order_quad`]
// * `0x006362f0` [`a82_start_order_delay`]
// * `0x00636340` [`a82_clear_order_chain`]
// * `0x00636520` [`a82_error_buffer_choice`]
// * `0x00636910` [`a82_scaled_u32_size`]
// * `0x006369a0` [`a82_release_if_flagged`]
// * `0x006369b0` [`a82_free_slot_abe4`]
// * `0x006369d0` [`a82_free_slot_b3b4`]
// * `0x006369f0` [`a82_free_slot_5dc4`]
// * `0x00636a10` [`a82_relink_card_list`]
// * `0x00636a70` [`a82_relink_unit_list`]
// * `0x00636ad0` [`a82_set_card_slot`]
// * `0x00636ba6` [`A82CardNode::a82_snapshot`]
// * `0x006370b2` [`a82_null_cleanup_result`]
// * `0x006370cb` [`a82_clear_pending_chain`]
// * `0x0063711f` [`A82CardNode::a82_restore`]
// * `0x006371a2` [`a82_pool_index`]
// * `0x00637286` [`a82_null_release_result`]
// * `0x0063729c` [`a82_cleanup_if_present`]
// * `0x006372e1` [`a82_card_set_counts`]
// * `0x00637510` [`a82_issue_slot_decision`]
// * `0x00637810` [`a82_init_card_node`]
// * `0x00637ae0` [`a82_card_table_entry`]
// * `0x00637b10` [`a82_clear_card_slot`]
// * `0x00637b40` [`a82_copy_card_slot`]
// * `0x00637be0` [`a82_card_page_limit`], [`a82_card_assign_slots`],
//   [`a82_card_node_timer`]
// * `0x00637eb0` [`a82_order_refresh_gate`]
// * `0x00637f00` [`a82_card_click_precheck`]
//
// Every out-of-window helper is an input to these value functions (or an
// explicit `Option`/`Result` tail); no unimplemented helper is called or
// presented as translated.

// ---------------------------------------------------------------------------
// Recovered record and node constants
// ---------------------------------------------------------------------------

/// Bytes in one per-owner order/card record (`FUN_00633df0`).
pub const A82_RECORD_BYTES: usize = 0x4e8;

/// Dwords in one record.
pub const A82_RECORD_WORDS: usize = A82_RECORD_BYTES / 4;

/// Byte offset of the order-heap count (`DAT_00faa408`).
pub const A82_ORDER_COUNT_OFFSET: usize = 0x1f8;

/// Byte offset of the record flags (`DAT_00faa410`).
pub const A82_ORDER_FLAGS_OFFSET: usize = 0x200;

/// Byte offset of the biased pending card word (`DAT_00faa416`).
pub const A82_ORDER_WORD_OFFSET: usize = 0x206;

/// Byte offset of the popped order word (`DAT_00faa418`).
pub const A82_ORDER_SLOT_OFFSET: usize = 0x208;

/// Byte offset of the retry counter byte (`DAT_00faa41b`).
pub const A82_ORDER_RETRY_OFFSET: usize = 0x20b;

/// Byte offset of the record timer (`DAT_00faa420`).
pub const A82_ORDER_TIMER_OFFSET: usize = 0x210;

/// Byte offset of the `0x3f` card slots (`DAT_00faa428`).
pub const A82_CARD_SLOT_OFFSET: usize = 0x218;

/// Number of `u16` slots scanned by `0x006361b0`.
pub const A82_CARD_SLOT_COUNT: usize = 0x3f;

/// Byte offset of the four-dword block copied by `0x006362d0`.
pub const A82_ORDER_QUAD_OFFSET: usize = 0x4c0;

/// Normal order retry, in ticks (`0x78`).
pub const A82_ORDER_TIMEOUT: u32 = 0x78;

/// Longer order retry used when flag `0x20` is set (`0xb4`).
pub const A82_ORDER_TIMEOUT_LONG: u32 = 0xb4;

/// Flag selecting the longer retry.
pub const A82_ORDER_FLAG_LONG: u16 = 0x20;

/// Default added delay in `0x006362f0` (`0x14`).
pub const A82_ORDER_DELAY_DEFAULT: i32 = 0x14;

/// Bias subtracted by `0x006362f0` (`0xb4`).
pub const A82_ORDER_DELAY_BIAS: u32 = 0xb4;

/// Slot value written by `0x006361b0` (`0xe5`).
pub const A82_CARD_SLOT_EMPTY: u16 = 0xe5;

/// Stride of the `0x1cc`-byte pools indexed by `0x006371a2` and
/// `0x006372e1`.
pub const A82_SET_STRIDE: u32 = 0x1cc;

/// Base of the larger pool used by `0x006371a2` (`0x00fbd2f0`).
pub const A82_POOL_BASE: u32 = 0x00fb_d2f0;

/// Base of the `0x18`-byte node array used by `0x006371a2` (`0x00fac940`).
pub const A82_NODE_POOL_BASE: u32 = 0x00fa_c940;

/// Tag byte of a slot-card node (checked by `0x00637b10`/`0x00637b40`).
pub const A82_CARD_NODE_TAG: u8 = 3;

/// Tag byte of the alternate node built by `0x00637810` (the shipped
/// `0x102` half-word store leaves `2` in the tag byte and `1` at `+0x09`).
pub const A82_OTHER_NODE_TAG: u8 = 2;

/// List head offset reached as `this + 44000` by `0x00636a10` (`0xabe0`).
pub const A82_CARD_LIST_OFFSET: u32 = 44_000;

/// List head offset reached as `this + 24000` by `0x00636a70` (`0x5dc0`).
pub const A82_UNIT_LIST_OFFSET: u32 = 24_000;

// ---------------------------------------------------------------------------
// Card node
// ---------------------------------------------------------------------------

/// One 44-byte intrusive card node.
///
/// The five slot bytes cover `+0x09..=+0x0d` and the five slot values cover
/// `+0x18..=+0x2b`.  `next`/`prev` are modelled as arena indices.
///
/// `0x637be0` reads a dword retry timer at `+0x0c`, i.e. over bytes
/// `states[3]`, `states[4]`, `pad[0]`, `pad[1]`.  That timer is modelled as a
/// separate value by [`a82_card_node_timer`], so the byte layout here stays
/// exact for the slot, snapshot and clear helpers that touch those offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A82CardNode {
    /// `+0x00`.
    pub next: Option<usize>,
    /// `+0x04`.
    pub prev: Option<usize>,
    /// `+0x08`.
    pub tag: u8,
    /// `+0x09..=+0x0d`.
    pub states: [u8; 5],
    /// `+0x0e..=+0x0f`.
    pub pad: [u8; 2],
    /// `+0x10`.
    pub item: u32,
    /// `+0x14`.
    pub owner: u32,
    /// `+0x18..=+0x2b`.
    pub values: [u32; 5],
}

impl Default for A82CardNode {
    fn default() -> Self {
        A82CardNode {
            next: None,
            prev: None,
            tag: 0,
            states: [0; 5],
            pad: [0; 2],
            item: 0,
            owner: 0,
            values: [0; 5],
        }
    }
}

impl A82CardNode {
    /// `0x00636ba6`: copy the eight bytes at `+0x08..+0x10` into the
    /// handler's cache (`DAT_00fac948..DAT_00fac94f`).
    ///
    /// The shipped row writes `DAT_00fac948` from `+0x08`, then the three
    /// bytes at `+0x09/+0x0a/+0x0b` through pointers biased by the node
    /// address, then the dword at `+0x0c`.  The four stores are one
    /// contiguous copy.  The separate dword read from `+0x10` is only a
    /// stack argument to the out-of-batch tail call, so it is not cached.
    pub fn a82_snapshot(&self) -> [u8; 8] {
        let mut out = [0u8; 8];
        out[0] = self.tag;
        out[1..6].copy_from_slice(&self.states);
        out[6..8].copy_from_slice(&self.pad);
        out
    }

    /// `0x0063711f`: restore the eight cached bytes into `+0x08..+0x10`.
    ///
    /// ```c
    /// *(u8 *)(edi + 8)       = DAT_00fac948;
    /// *(u8 *)(edi + 9)       = DAT_00fac949;
    /// *(u8 *)(edi + 10)      = DAT_00fac94a;
    /// *(u8 *)(edi + 0xb)     = DAT_00fac94b;
    /// *(u32 *)(edi + 0xc)    = DAT_00fac94c;
    /// ```
    pub fn a82_restore(&mut self, cache: &[u8; 8]) {
        self.tag = cache[0];
        self.states.copy_from_slice(&cache[1..6]);
        self.pad.copy_from_slice(&cache[6..8]);
    }
}

/// The two global list heads addressed by `0x00636a10`/`0x00636a70`.
///
/// `0x00636a10` inserts into `*(this) + 44000` (`0xabe0`) and `0x00636a70`
/// into `*(this) + 24000` (`0x5dc0`).  `source` is `this[1]`, the head of the
/// list the node is unlinked from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A82OwnerLists {
    /// `this[1]`: source-list head.
    pub source: Option<usize>,
    /// `*(this) + 44000` (`0xabe0`).
    pub card_head: Option<usize>,
    /// `*(this) + 24000` (`0x5dc0`).
    pub unit_head: Option<usize>,
}

fn a82_relink_to(
    nodes: &mut [A82CardNode],
    lists: &mut A82OwnerLists,
    node: usize,
    card_list: bool,
) {
    let next = nodes[node].next;
    let prev = nodes[node].prev;
    if lists.source == Some(node) {
        lists.source = next;
    }
    if let Some(prev) = prev {
        nodes[prev].next = next;
    }
    if let Some(next) = next {
        nodes[next].prev = prev;
    }
    let target = if card_list {
        &mut lists.card_head
    } else {
        &mut lists.unit_head
    };
    nodes[node].prev = None;
    nodes[node].next = *target;
    if let Some(head) = *target {
        nodes[head].prev = Some(node);
    }
    *target = Some(node);
}

/// `0x00636a10`: unlink a node and insert it at the head of the card list
/// (`*(this) + 44000`, `0xabe0`).
///
/// ```c
/// if (this[1] == node) this[1] = node->next;
/// if (node->prev) node->prev->next = node->next;
/// if (node->next) node->next->prev = node->prev;
/// owner = *this;
/// node->prev = 0;
/// node->next = owner->head_abe0;
/// if (owner->head_abe0) owner->head_abe0->prev = node;
/// owner->head_abe0 = node;
/// ```
pub fn a82_relink_card_list(nodes: &mut [A82CardNode], lists: &mut A82OwnerLists, node: usize) {
    a82_relink_to(nodes, lists, node, true);
}

/// `0x00636a70`: the same relink for the list at `*(this) + 24000`
/// (`0x5dc0`).  The two rows differ only in the destination head offset.
pub fn a82_relink_unit_list(nodes: &mut [A82CardNode], lists: &mut A82OwnerLists, node: usize) {
    a82_relink_to(nodes, lists, node, false);
}

/// `0x00636ad0`: write one card slot on a node reached through
/// `*(record + 0x134) + 9` and `+ 0x18`.
///
/// ```c
/// base = *(int *)(param_1 + 0x134);
/// *(u8 *)(base + 9 + index)       = state;
/// *(u32 *)(base + 0x18 + index*4) = value;
/// ```
pub fn a82_set_card_slot(node: &mut A82CardNode, index: usize, state: u8, value: u32) {
    node.states[index] = state;
    node.values[index] = value;
}

/// `0x00637b10`: clear one slot when the node tag is `3`.
///
/// ```c
/// if (node != 0 && node->tag == 3) {
///     node->state[index] = 0;
///     node->value[index] = 0;
/// }
/// ```
///
/// The row performs the tag test itself, so a node of another kind is left
/// untouched and `false` is returned.
pub fn a82_clear_card_slot(node: &mut A82CardNode, index: usize) -> bool {
    if node.tag != A82_CARD_NODE_TAG {
        return false;
    }
    node.states[index] = 0;
    node.values[index] = 0;
    true
}

/// `0x00637b40`: copy one slot when the node tag is `3`.
///
/// The source state and value are read before either destination store, so
/// `dst == src` and overlapping arena slots behave like the shipped code.
pub fn a82_copy_card_slot(node: &mut A82CardNode, dst: usize, src: usize) -> bool {
    if node.tag != A82_CARD_NODE_TAG {
        return false;
    }
    let state = node.states[src];
    let value = node.values[src];
    node.states[dst] = state;
    node.values[dst] = value;
    true
}

/// `0x00637810`: initialise one card node for the two allocation branches.
///
/// Branch `Card` (taken when the out-of-window allocator probe returns zero)
/// stores tag `3` and zeroes the five states, five values, item and owner
/// before the caller fills item/owner.  Branch `Other` stores the half-word
/// `0x102` at `+0x08` — tag `2`, `states[0] = 1` — then a zero half-word at
/// `+0x0a` and the tick at `+0x0c`.  Both branches leave `+0x0e/+0x0f`
/// untouched; a fresh allocation is modelled as zero.  The list insertion
/// itself is [`a82_relink_card_list`] / [`a82_relink_unit_list`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A82NodeInit {
    /// Tag `3`, zeroed slots.
    Card,
    /// Tag `2`, `states[0] = 1`, tick at `+0x0c`.
    Other,
}

/// See [`A82NodeInit`].
pub fn a82_init_card_node(init: A82NodeInit, item: u32, owner: u32, tick: u32) -> A82CardNode {
    match init {
        A82NodeInit::Card => A82CardNode {
            tag: A82_CARD_NODE_TAG,
            item,
            owner,
            ..A82CardNode::default()
        },
        A82NodeInit::Other => {
            let t = tick.to_le_bytes();
            A82CardNode {
                tag: A82_OTHER_NODE_TAG,
                states: [1, 0, 0, t[0], t[1]],
                pad: [t[2], t[3]],
                item,
                owner,
                ..A82CardNode::default()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 0x00636130 - order retry timer
// ---------------------------------------------------------------------------

/// `0x00636130`: test and consume the per-owner order retry timer.
///
/// ```c
/// timer = *(u32 *)(record + 0x210);
/// if (timer == 0) return false;
/// elapsed = now - timer;                       // unsigned wrap
/// limit = (flags & 0x20) ? 0xb4 : 0x78;
/// if (elapsed >= limit) *(u32 *)(record + 0x210) = 0;
/// return elapsed >= limit;
/// ```
///
/// The timer field is `DAT_00faa420` (`+0x210` of the `0x4e8` record).  The
/// returned word is the timer value after the row (`0` when consumed).
pub fn a82_order_timer_expired(start: u32, now: u32, flags: u16) -> (bool, u32) {
    if start == 0 {
        return (false, start);
    }
    let elapsed = now.wrapping_sub(start);
    let limit = if flags & A82_ORDER_FLAG_LONG != 0 {
        A82_ORDER_TIMEOUT_LONG
    } else {
        A82_ORDER_TIMEOUT
    };
    if elapsed >= limit {
        (true, 0)
    } else {
        (false, start)
    }
}

// ---------------------------------------------------------------------------
// 0x006361b0 - card-slot scan
// ---------------------------------------------------------------------------

/// `0x006361b0`: find and empty one of the `0x3f` card slots.
///
/// ```c
/// target = (param == 0x1e) ? 5 : param;
/// for (i = 0; i != 0x3f; ++i)
///     if ((u32)slots[i] == target + 1U) { slots[i] = 0xe5; return; }
/// ```
///
/// The comparison is 32-bit unsigned, so a negative `param` wraps through
/// `target + 1`.  `Some(index)` reports the slot that was changed.
pub fn a82_mark_card_slot(slots: &mut [u16; A82_CARD_SLOT_COUNT], param: i32) -> Option<usize> {
    let target = if param == 0x1e { 5i32 } else { param };
    let wanted = (target as u32).wrapping_add(1);
    for (index, slot) in slots.iter_mut().enumerate() {
        if u32::from(*slot) == wanted {
            *slot = A82_CARD_SLOT_EMPTY;
            return Some(index);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// 0x00636290 / 0x006362d0 / 0x006362f0 - start timer, store quad, add delay
// ---------------------------------------------------------------------------

/// `0x00636290`: start the order timer and store the biased card word.
///
/// ```c
/// if (force || *(u32 *)(record + 0x210) == 0)
///     *(u32 *)(record + 0x210) = now;
/// *(u16 *)(record + 0x206) = param_2 + 1;
/// ```
///
/// The word is stored at `DAT_00faa416` (`+0x206`).  The return value is
/// `(timer after the row, biased word)`.
pub fn a82_begin_order_timer(start: u32, now: u32, word: i16, force: bool) -> (u32, u16) {
    let timer = if force || start == 0 { now } else { start };
    (timer, (word as u16).wrapping_add(1))
}

/// `0x006362d0`: copy four dwords from the caller into `+0x4c0`.
///
/// The shipped body reads all four source words before the stores, which
/// [`a82_store_order_quad`] reproduces by taking the source by value.
pub fn a82_store_order_quad(record: &mut [u32; A82_RECORD_WORDS], src: [u32; 4]) {
    let word = A82_ORDER_QUAD_OFFSET / 4;
    record[word..word + 4].copy_from_slice(&src);
}

/// `0x006362f0`: set the order timer to `now + delay - 0xb4`.
///
/// ```c
/// if (delay != 0 || timer == 0) {
///     delay = (delay != 0) ? delay : 0x14;
///     timer = delay + now - 0xb4;          // 32-bit wrap
/// }
/// ```
pub fn a82_start_order_delay(start: u32, now: u32, delay: i32) -> u32 {
    if delay != 0 || start == 0 {
        let delay = if delay != 0 {
            delay as u32
        } else {
            A82_ORDER_DELAY_DEFAULT as u32
        };
        now.wrapping_add(delay).wrapping_sub(A82_ORDER_DELAY_BIAS)
    } else {
        start
    }
}

// ---------------------------------------------------------------------------
// 0x00636340 - clear an order-node chain
// ---------------------------------------------------------------------------

/// `0x00636340`: walk the chain at `this + 44000` and clear each node.
///
/// Every visited node gets `tag = 0`, five zero states, item `0`, owner `0`
/// and five zero values.  The two padding bytes at `+0x0e/+0x0f` are not
/// touched by the row, and neither are `next`/`prev`.  The row ends in the
/// out-of-batch `FUN_00c8a080(param_2, 0, 0xabe4)`; the pure function returns
/// the number of nodes cleared before that tail.
pub fn a82_clear_order_chain(nodes: &mut [A82CardNode], head: Option<usize>) -> usize {
    let mut cursor = head;
    let mut count = 0usize;
    while let Some(index) = cursor {
        if index >= nodes.len() {
            break;
        }
        let node = &mut nodes[index];
        node.tag = 0;
        node.states = [0; 5];
        node.item = 0;
        node.owner = 0;
        node.values = [0; 5];
        cursor = node.next;
        count += 1;
    }
    count
}

/// `0x006370cb`: the shorter chain clear used by the second list.
///
/// ```c
/// for (p = DAT_00fb2700; p != 0; p = *p) {
///     p[2] = 0; p[4] = 0; p[5] = 0; p[3] = 0;
/// }
/// ```
///
/// The four dword stores cover `+0x08..+0x18` — tag, five states, the two
/// padding bytes, item and owner.  The slot values are left intact.  The
/// return value reproduces `in_EAX & 0xffffff00`; the non-zero `unaff_EDI`
/// arm is the caller's out-of-batch abort and is not representable as a
/// value, so this function models the clean-return path only.
pub fn a82_clear_pending_chain(nodes: &mut [A82CardNode], head: Option<usize>, in_eax: u32) -> u32 {
    let mut cursor = head;
    while let Some(index) = cursor {
        if index >= nodes.len() {
            break;
        }
        let node = &mut nodes[index];
        node.tag = 0;
        node.states = [0; 5];
        node.pad = [0; 2];
        node.item = 0;
        node.owner = 0;
        cursor = node.next;
    }
    in_eax & 0xffff_ff00
}

// ---------------------------------------------------------------------------
// 0x00636520 / FUN_00636740 - error-buffer selection
// ---------------------------------------------------------------------------

/// One out-of-batch non-returning error call: buffer and error code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A82ErrorCall {
    /// First argument of the shipped error routine.
    pub buffer: u32,
    /// Third argument of the shipped error routine.
    pub code: u32,
}

/// `0x00636520`: choose the error buffer and code.
///
/// ```c
/// if (*(int *)(this + 46000) != 0)
///     FUN_00c8a080(*(int *)(this + 46000) + 0x18, 0, 0x1b4);
/// FUN_00c8a080(param_2, 0, 0xb3b4);
/// ```
///
/// The first call is taken when the owned pointer at `+0xb3b0` is non-zero.
/// Both calls are non-returning, so only their arguments are value data.
pub fn a82_error_buffer_choice(primary: Option<u32>, fallback: u32) -> A82ErrorCall {
    match primary {
        Some(primary) => A82ErrorCall {
            buffer: primary.wrapping_add(0x18),
            code: 0x1b4,
        },
        None => A82ErrorCall {
            buffer: fallback,
            code: 0xb3b4,
        },
    }
}

// ---------------------------------------------------------------------------
// 0x00636910 - checked 32-bit size multiply
// ---------------------------------------------------------------------------

/// `0x00636910`: multiply by eight, saturating to `u32::MAX` on overflow.
///
/// ```c
/// wide = (u64)value * 8;
/// return -(u32)((wide >> 32) != 0) | (u32)wide;
/// ```
///
/// When the high dword is non-zero the negation produces `0xffffffff`, and
/// the OR keeps it; otherwise the low dword is returned unchanged.
pub fn a82_scaled_u32_size(count: u32) -> u32 {
    let wide = u64::from(count) * 8;
    if wide >> 32 != 0 {
        u32::MAX
    } else {
        wide as u32
    }
}

// ---------------------------------------------------------------------------
// 0x006369a0 / 0x006369b0 / 0x006369d0 / 0x006369f0 - small gates
// ---------------------------------------------------------------------------

/// `0x006369a0`: report the release call made when `*(this + 1) != 0`.
///
/// The row is `if (*(char *)(ecx + 1) != 0) FUN_0094fec0(0);`.  `Some(0)` is
/// the out-of-batch call; `None` means no call was made.
pub fn a82_release_if_flagged(flag: u8) -> Option<u32> {
    if flag != 0 { Some(0) } else { None }
}

/// `0x006369b0`: the `if (*this != 0) FUN_00923e90(*this, 0xabe4);` gate.
pub fn a82_free_slot_abe4(value: u32) -> Option<(u32, u32)> {
    if value != 0 {
        Some((value, 0xabe4))
    } else {
        None
    }
}

/// `0x006369d0`: the same gate with code `0xb3b4`.
pub fn a82_free_slot_b3b4(value: u32) -> Option<(u32, u32)> {
    if value != 0 {
        Some((value, 0xb3b4))
    } else {
        None
    }
}

/// `0x006369f0`: the same gate with code `0x5dc4`.
pub fn a82_free_slot_5dc4(value: u32) -> Option<(u32, u32)> {
    if value != 0 {
        Some((value, 0x5dc4))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// 0x006370b2 / 0x00637286 / 0x0063729c - cleanup results
// ---------------------------------------------------------------------------

/// `0x006370b2`: null-allocation cleanup result.
///
/// ```c
/// if (value == 0) { three locals = 0; return 0; }
/// FUN_00c8a080();        // non-returning
/// ```
///
/// `Some(0)` is the recovered return; `None` is the abort arm.
pub fn a82_null_cleanup_result(value: u32) -> Option<u32> {
    if value == 0 { Some(0) } else { None }
}

/// `0x00637286`: the second null-allocation cleanup result (`0xabe4` arm).
pub fn a82_null_release_result(value: u32) -> Option<u32> {
    if value == 0 { Some(0) } else { None }
}

/// The outcome of `0x0063729c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A82CleanupOutcome {
    /// No work: the row returns `in_EAX & 0xffffff00`.
    Return(u32),
    /// Clear the chain through `0x00636340` and take the non-returning
    /// out-of-batch cleanup path.
    CleanAndAbort,
}

/// `0x0063729c`: decide whether the `0x00636340` chain clear runs.
///
/// ```c
/// if (esi == 0) return eax & 0xffffff00;
/// FUN_00636340();
/// if (FUN_00687d90() != 0) { ... FUN_00923e90(); }
/// FUN_00923e90();
/// ```
pub fn a82_cleanup_if_present(present: bool, in_eax: u32) -> A82CleanupOutcome {
    if present {
        A82CleanupOutcome::CleanAndAbort
    } else {
        A82CleanupOutcome::Return(in_eax & 0xffff_ff00)
    }
}

// ---------------------------------------------------------------------------
// 0x006371a2 / 0x006372e1 - pool indices and set counts
// ---------------------------------------------------------------------------

/// `0x006371a2`: convert a non-null pool pointer into its one-based index.
///
/// ```c
/// index = (ptr == 0) ? 0 : (ptr - base) / stride + 1;
/// ```
///
/// The subtraction and division are unsigned, exactly as the shipped
/// `(ptr - 0xfbd2f0U) / 0x1cc + 1` and `(ptr - 0xfac940U) / 0x18 + 1` are.
pub fn a82_pool_index(ptr: u32, base: u32, stride: u32) -> u32 {
    if ptr == 0 {
        0
    } else {
        ptr.wrapping_sub(base) / stride + 1
    }
}

/// `0x006372e1`: one set count, with the shipped `0` sentinel for an unused
/// range.
///
/// ```c
/// count = (end == 0) ? 0 : (end - begin) / 0x1cc + 1;
/// ```
pub fn a82_set_count(begin: u32, end: u32) -> u32 {
    if end == 0 {
        0
    } else {
        end.wrapping_sub(begin) / A82_SET_STRIDE + 1
    }
}

/// `0x006372e1`: fill the sixteen-word structure from eight pool pairs.
///
/// The shipped stores alternate a zero at even indices and the one-based
/// count at odd indices:
/// `puVar1[0] = 0`, `puVar1[1] = count`, `puVar1[2] = 0`, `puVar1[3] = count`,
/// and so on through `puVar1[0xf]`.
pub fn a82_card_set_counts(bounds: &[(u32, u32); 8]) -> [u32; 16] {
    let mut out = [0u32; 16];
    for (index, &(begin, end)) in bounds.iter().enumerate() {
        out[index * 2] = 0;
        out[index * 2 + 1] = a82_set_count(begin, end);
    }
    out
}

// ---------------------------------------------------------------------------
// 0x00637510 - order-slot issue gate
// ---------------------------------------------------------------------------

/// The decision produced by the `0x00637510` gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A82IssueDecision {
    /// One of the early tests failed; the function returns `false`.
    Reject,
    /// The pending slot became `-1`; the row takes the non-returning
    /// out-of-batch path.
    Abort,
    /// Slot `0xe4` is skipped without issuing.
    Skip,
    /// The surviving slot is issued through the out-of-batch helpers;
    /// `accepted` is the `FUN_00635c40(player) != 0` return test.
    Issue {
        /// Surviving order slot.
        slot: i32,
        /// Result of the out-of-batch post-issue check.
        accepted: bool,
    },
}

/// `0x00637510`: the recovered order-slot gate.
///
/// ```c
/// if (FUN_00658130(unit) != 0 || unit->byte_a6 != 0x17) return false;
/// if (!FUN_00635170(player, &slot)) slot = -1;
/// if (slot == -1) {
///     if (minerals < 600) return false;
///     if (gas < 300) return false;
///     if (!FUN_00635eb0(player)) return false;
/// } else if (FUN_00659530(player, slot) != 0) {
///     FUN_00659770(player, slot);
///     slot = -1;
/// }
/// if (slot == -1) FUN_005b4cd0();          // non-returning
/// if (slot == 0xe4) return false;
/// FUN_00634f10(player, FUN_00656fb0(unit, slot, arg, 1, 1));
/// return FUN_00635c40(player) != 0;
/// ```
///
/// The out-of-window calls become explicit inputs; the resource thresholds,
/// the `-1` sentinel, the `0xe4` skip and the final `FUN_00635c40` test are
/// reproduced exactly.
pub fn a82_issue_slot_decision(
    unit_free: bool,
    unit_state: u8,
    slot: i32,
    minerals: i32,
    gas: i32,
    available: bool,
    owner_ok: bool,
    accepted: bool,
) -> A82IssueDecision {
    if !unit_free || unit_state != 0x17 {
        return A82IssueDecision::Reject;
    }
    let mut slot = slot;
    if slot == -1 {
        if minerals < 600 {
            return A82IssueDecision::Reject;
        }
        if gas < 300 {
            return A82IssueDecision::Reject;
        }
        if !available {
            return A82IssueDecision::Reject;
        }
    } else if owner_ok {
        slot = -1;
    }
    if slot == -1 {
        return A82IssueDecision::Abort;
    }
    if slot == 0xe4 {
        return A82IssueDecision::Skip;
    }
    A82IssueDecision::Issue { slot, accepted }
}

// ---------------------------------------------------------------------------
// 0x00637ae0 - card owner dispatch
// ---------------------------------------------------------------------------

/// `0x00637ae0`: look up the card-table owner for a unit byte.
///
/// ```c
/// if (FUN_005b3b60() != 0) {
///     entry = (&DAT_00fc86ac)[unit->byte4c * 2];       // eight-byte pairs
///     if (entry != 0) FUN_00637810(entry, unit);
/// }
/// ```
///
/// The table is addressed in dwords with a doubled index, so the first dword
/// of pair `index` is `table[index * 2]`.  `None` is the no-dispatch path.
pub fn a82_card_table_entry(ready: bool, table: &[u32], index: usize) -> Option<u32> {
    if !ready {
        return None;
    }
    let value = *table.get(index.checked_mul(2)?)?;
    if value != 0 { Some(value) } else { None }
}

// ---------------------------------------------------------------------------
// 0x00637be0 - card page sizing and node slot assignment
// ---------------------------------------------------------------------------

/// `0x00637be0`: the per-frame card-slot limit.
///
/// ```c
/// current = (alpha * fraction) / 100;
/// limit = candidate;
/// if (current <= candidate) {
///     limit = current;
///     if (screen_height < 800) {
///         if (used < 0x14) {
///             if (used < 0xf) {
///                 if (used < 10) {
///                     if (used > 4 && current == 0) limit = 1;
///                 } else if (current < 2) limit = 2;
///             } else if (current < 3) limit = 3;
///         } else if (current < 4) limit = 4;
///     }
/// }
/// ```
///
/// `used` is the out-of-window `FUN_00695c10` result; `candidate` is
/// `FUN_0076a310(...) << 2`; `alpha` is the record byte at `+0x1a` and
/// `fraction` the high byte of the row argument.
pub fn a82_card_page_limit(
    candidate: u32,
    alpha: u8,
    fraction: u8,
    used: u32,
    screen_height: i32,
) -> u32 {
    let current = (u32::from(alpha) * u32::from(fraction)) / 100;
    if current <= candidate {
        let mut limit = current;
        if screen_height < 800 {
            if used < 0x14 {
                if used < 0xf {
                    if used < 10 {
                        if used > 4 && current == 0 {
                            limit = 1;
                        }
                    } else if current < 2 {
                        limit = 2;
                    }
                } else if current < 3 {
                    limit = 3;
                }
            } else if current < 4 {
                limit = 4;
            }
        }
        limit
    } else {
        candidate
    }
}

/// `0x00637be0`: assign the per-node slot byte (`+0x09`) and advance the two
/// rolling counters.
///
/// ```c
/// uVar10 = limit - 1; uVar3 = second;
/// if (limit == 0) { uVar10 = 0; uVar3 = first; }
/// node->state[0] = uVar3;
/// if (uVar3 != 1 && ++streak > 3) {
///     streak = 0;
///     second = (second != uVar3) ? second : second + 1;
///     first  = (first  != uVar3) ? first  : first + 1;
///     if (second > 4) second = 2;
///     if (first  > 4) first  = 2;
/// }
/// limit = uVar10;
/// ```
///
/// The returned pair is the final `(first, second)` (`local_14`,
/// `local_10`).
pub fn a82_card_assign_slots(
    nodes: &mut [A82CardNode],
    mut limit: u32,
    mut first: u32,
    mut second: u32,
) -> (u32, u32) {
    let mut streak = 0u32;
    for node in nodes.iter_mut() {
        let mut next_limit = limit.wrapping_sub(1);
        let mut slot = second;
        if limit == 0 {
            next_limit = 0;
            slot = first;
        }
        node.states[0] = slot as u8;
        if slot != 1 {
            streak = streak.wrapping_add(1);
            if streak > 3 {
                streak = 0;
                let mut next_second = second.wrapping_add(1);
                if second != slot {
                    next_second = second;
                }
                let mut next_first = first.wrapping_add(1);
                if first != slot {
                    next_first = first;
                }
                if next_second > 4 {
                    next_second = 2;
                }
                if next_first > 4 {
                    next_first = 2;
                }
                first = next_first;
                second = next_second;
            }
        }
        limit = next_limit;
    }
    (first, second)
}

/// Result of the per-node timer branch of `0x00637be0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A82CardTimer {
    /// `+0x210`-style timer word stored on the node.
    pub timer: u32,
    /// Byte at `+0x0a`.
    pub byte_a: u8,
    /// Byte at `+0x0b`.
    pub byte_b: u8,
    /// The row calls `FUN_00638fc0(node[5], 1)` exactly when this is true.
    pub refresh: bool,
}

/// `0x00637be0`: per-node retry refresh.
///
/// ```c
/// kind = unit->byte4d;
/// if (kind == '!' || kind == '`' || kind == 0x1a || FUN_005b5690() != 0) {
///     node->timer = now;
/// } else if (now - node->timer > 0x3b) {
///     FUN_005b7d00(0x9c);
///     node->byte_b = 10;
///     node->byte_a += 1;
///     node->timer = now;
///     FUN_00638fc0(node[5], 1);
/// }
/// ```
///
/// The out-of-window `FUN_005b5690` result is `special`.
pub fn a82_card_node_timer(
    kind: u8,
    timer: u32,
    byte_a: u8,
    byte_b: u8,
    now: u32,
    special: bool,
) -> A82CardTimer {
    if kind == b'!' || kind == b'`' || kind == 0x1a || special {
        A82CardTimer {
            timer: now,
            byte_a,
            byte_b,
            refresh: false,
        }
    } else if now.wrapping_sub(timer) > 0x3b {
        A82CardTimer {
            timer: now,
            byte_a: byte_a.wrapping_add(1),
            byte_b: 10,
            refresh: true,
        }
    } else {
        A82CardTimer {
            timer,
            byte_a,
            byte_b,
            refresh: false,
        }
    }
}

// ---------------------------------------------------------------------------
// 0x00637eb0 / 0x00637f00 - order refresh and click gates
// ---------------------------------------------------------------------------

/// `0x00637eb0`: the order-refresh gate.
///
/// ```c
/// code = FUN_0065b900();
/// if (param_4 != 0 || code == 2 || code == 0x17 || code == 0x15 ||
///     code == 8 || code == 7 || code == 0xd) {
///     FUN_00635240(param_1);
///     if (param_3 == 0) FUN_0063a3c0(param_2);
/// }
/// ```
///
/// Returns `(refresh_order_list, refresh_card)`.
pub fn a82_order_refresh_gate(code: i32, install: bool, force: bool) -> (bool, bool) {
    let apply = force || matches!(code, 2 | 0x17 | 0x15 | 8 | 7 | 0xd);
    (apply, apply && !install)
}

/// `0x00637f00`: the preconditions checked before a card click is dispatched.
///
/// ```c
/// if (FUN_00635c40(player) != 3) return false;
/// node = *(int *)(unit + 0x134);
/// slot = FUN_00635c20(player);
/// value = FUN_00635c00(player);
/// if (value != node->owner && slot != 0x6d) return false;
/// if (node->byte_0a > 2) return false;
/// ```
///
/// The first byte is the top order kind, `slot` the top order id and `value`
/// the top order payload (`DAT_00faa212` / `DAT_00faa214`).
pub fn a82_card_click_precheck(
    order_kind: u8,
    order_value: u32,
    node_owner: u32,
    order_slot: i32,
    node_page: u8,
) -> bool {
    if order_kind != 3 {
        return false;
    }
    if order_value != node_owner && order_slot != 0x6d {
        return false;
    }
    if node_page > 2 {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent82 {
    use super::*;

    fn node() -> A82CardNode {
        A82CardNode::default()
    }

    // -- 0x00636130 ---------------------------------------------------------

    #[test]
    fn order_timer_uses_120_or_180_ticks() {
        assert_eq!(a82_order_timer_expired(0, 10_000, 0), (false, 0));
        assert_eq!(a82_order_timer_expired(100, 219, 0), (false, 100));
        assert_eq!(a82_order_timer_expired(100, 220, 0), (true, 0));
        assert_eq!(
            a82_order_timer_expired(100, 279, A82_ORDER_FLAG_LONG),
            (false, 100)
        );
        assert_eq!(
            a82_order_timer_expired(100, 280, A82_ORDER_FLAG_LONG),
            (true, 0)
        );
        // Unsigned wrap: a start after `now` is a huge elapsed value.
        assert_eq!(a82_order_timer_expired(0x10, 0xffff_fff0, 0), (true, 0));
    }

    // -- 0x006361b0 ---------------------------------------------------------

    #[test]
    fn card_slot_scan_biases_and_empties() {
        let mut slots = [0u16; A82_CARD_SLOT_COUNT];
        slots[3] = 7;
        assert_eq!(a82_mark_card_slot(&mut slots, 6), Some(3));
        assert_eq!(slots[3], A82_CARD_SLOT_EMPTY);

        let mut slots = [0u16; A82_CARD_SLOT_COUNT];
        slots[10] = 6; // 0x1e is normalised to 5, so target word is 6.
        assert_eq!(a82_mark_card_slot(&mut slots, 0x1e), Some(10));
        assert_eq!(a82_mark_card_slot(&mut slots, 0x1e), None);

        let mut slots = [0xffffu16; A82_CARD_SLOT_COUNT];
        assert_eq!(a82_mark_card_slot(&mut slots, -1), None);
        slots[0] = 0;
        assert_eq!(a82_mark_card_slot(&mut slots, -1), Some(0));
    }

    // -- 0x00636290 / 0x006362d0 / 0x006362f0 ------------------------------

    #[test]
    fn order_timer_begin_and_delay() {
        assert_eq!(a82_begin_order_timer(0, 100, 4, false), (100, 5));
        assert_eq!(a82_begin_order_timer(50, 100, 4, false), (50, 5));
        assert_eq!(a82_begin_order_timer(50, 100, -1, true), (100, 0));

        let mut record = [0u32; A82_RECORD_WORDS];
        record[(A82_ORDER_QUAD_OFFSET / 4) - 1] = 0xaaaa;
        a82_store_order_quad(&mut record, [1, 2, 3, 4]);
        let at = A82_ORDER_QUAD_OFFSET / 4;
        assert_eq!(&record[at - 1..at + 4], &[0xaaaa, 1, 2, 3, 4]);
        assert_eq!(record[at + 4], 0);

        assert_eq!(a82_start_order_delay(0, 100, 0), 0xffff_ffc4);
        assert_eq!(a82_start_order_delay(0, 200, 10), 30);
        assert_eq!(a82_start_order_delay(50, 100, 0), 50);
        assert_eq!(a82_start_order_delay(50, 100, 10), 0xffff_ffba);
    }

    // -- 0x00636340 / 0x006370cb -------------------------------------------

    #[test]
    fn order_chain_clear_leaves_padding_and_links() {
        let mut nodes = [
            A82CardNode {
                tag: 3,
                states: [1, 2, 3, 4, 5],
                pad: [6, 7],
                item: 8,
                owner: 9,
                values: [10, 11, 12, 13, 14],
                next: Some(1),
                prev: None,
            },
            A82CardNode {
                tag: 3,
                states: [1, 2, 3, 4, 5],
                pad: [6, 7],
                item: 8,
                owner: 9,
                values: [10, 11, 12, 13, 14],
                next: None,
                prev: Some(0),
            },
        ];
        assert_eq!(a82_clear_order_chain(&mut nodes, Some(0)), 2);
        for node in &nodes {
            assert_eq!(node.tag, 0);
            assert_eq!(node.states, [0; 5]);
            assert_eq!(node.item, 0);
            assert_eq!(node.owner, 0);
            assert_eq!(node.values, [0; 5]);
            assert_eq!(node.pad, [6, 7]);
        }
        assert_eq!(nodes[0].next, Some(1));
        assert_eq!(nodes[1].prev, Some(0));
    }

    #[test]
    fn pending_chain_clear_keeps_values_and_returns_masked_eax() {
        let mut nodes = [A82CardNode {
            tag: 3,
            states: [1, 2, 3, 4, 5],
            pad: [6, 7],
            item: 8,
            owner: 9,
            values: [10, 11, 12, 13, 14],
            next: None,
            prev: None,
        }];
        assert_eq!(
            a82_clear_pending_chain(&mut nodes, Some(0), 0x1234_5678),
            0x1234_5600
        );
        assert_eq!(nodes[0].tag, 0);
        assert_eq!(nodes[0].states, [0; 5]);
        assert_eq!(nodes[0].pad, [0; 2]);
        assert_eq!(nodes[0].item, 0);
        assert_eq!(nodes[0].owner, 0);
        assert_eq!(nodes[0].values, [10, 11, 12, 13, 14]);
    }

    // -- 0x00636520 ---------------------------------------------------------

    #[test]
    fn error_buffer_choice_uses_the_two_codes() {
        assert_eq!(
            a82_error_buffer_choice(Some(0x1000), 0x2000),
            A82ErrorCall {
                buffer: 0x1018,
                code: 0x1b4
            }
        );
        assert_eq!(
            a82_error_buffer_choice(None, 0x2000),
            A82ErrorCall {
                buffer: 0x2000,
                code: 0xb3b4
            }
        );
    }

    // -- 0x00636910 ---------------------------------------------------------

    #[test]
    fn scaled_size_saturates_on_high_dword() {
        assert_eq!(a82_scaled_u32_size(0), 0);
        assert_eq!(a82_scaled_u32_size(1), 8);
        assert_eq!(a82_scaled_u32_size(0x1fff_ffff), 0xffff_fff8);
        assert_eq!(a82_scaled_u32_size(0x2000_0000), u32::MAX);
        assert_eq!(a82_scaled_u32_size(u32::MAX), u32::MAX);
    }

    // -- 0x006369a0 / 0x006369b0 / 0x006369d0 / 0x006369f0 -----------------

    #[test]
    fn small_release_gates() {
        assert_eq!(a82_release_if_flagged(0), None);
        assert_eq!(a82_release_if_flagged(1), Some(0));

        assert_eq!(a82_free_slot_abe4(0), None);
        assert_eq!(a82_free_slot_abe4(0x1234), Some((0x1234, 0xabe4)));
        assert_eq!(a82_free_slot_b3b4(0), None);
        assert_eq!(a82_free_slot_b3b4(0x1234), Some((0x1234, 0xb3b4)));
        assert_eq!(a82_free_slot_5dc4(0), None);
        assert_eq!(a82_free_slot_5dc4(0x1234), Some((0x1234, 0x5dc4)));
    }

    // -- 0x00636a10 / 0x00636a70 -------------------------------------------

    #[test]
    fn relink_moves_a_node_to_each_head() {
        let mut nodes = [node(), node(), node()];
        nodes[0].next = None;
        nodes[1].next = Some(2);
        nodes[1].prev = Some(0);
        nodes[2].prev = Some(1);
        nodes[0].next = Some(1);

        let mut lists = A82OwnerLists {
            source: Some(1),
            card_head: Some(0),
            unit_head: None,
        };
        a82_relink_card_list(&mut nodes, &mut lists, 1);
        assert_eq!(lists.source, Some(2));
        assert_eq!(nodes[0].next, Some(2));
        assert_eq!(nodes[2].prev, Some(0));
        assert_eq!(nodes[1].prev, None);
        assert_eq!(nodes[1].next, Some(0));
        assert_eq!(nodes[0].prev, Some(1));
        assert_eq!(lists.card_head, Some(1));

        a82_relink_unit_list(&mut nodes, &mut lists, 2);
        assert_eq!(lists.source, None);
        assert_eq!(nodes[0].next, None);
        assert_eq!(nodes[2].prev, None);
        assert_eq!(nodes[2].next, None);
        assert_eq!(lists.unit_head, Some(2));
    }

    // -- 0x00636ad0 / 0x00637b10 / 0x00637b40 ------------------------------

    #[test]
    fn card_slot_set_clear_and_copy() {
        let mut node = node();
        node.tag = A82_CARD_NODE_TAG;
        a82_set_card_slot(&mut node, 2, 7, 0xabcd);
        assert_eq!(node.states[2], 7);
        assert_eq!(node.values[2], 0xabcd);

        node.states[3] = 9;
        node.values[3] = 0x1111;
        assert!(a82_clear_card_slot(&mut node, 3));
        assert_eq!(node.states[3], 0);
        assert_eq!(node.values[3], 0);

        node.states[0] = 4;
        node.values[0] = 0x2222;
        assert!(a82_copy_card_slot(&mut node, 1, 0));
        assert_eq!(node.states[1], 4);
        assert_eq!(node.values[1], 0x2222);

        node.tag = 2;
        assert!(!a82_clear_card_slot(&mut node, 1));
        assert!(!a82_copy_card_slot(&mut node, 1, 0));
    }

    // -- 0x00636ba6 / 0x0063711f -------------------------------------------

    #[test]
    fn node_snapshot_round_trips_eight_bytes() {
        let mut node = A82CardNode {
            tag: 3,
            states: [1, 2, 3, 4, 5],
            pad: [6, 7],
            item: 0x1122_3344,
            owner: 0x5566_7788,
            values: [1, 2, 3, 4, 5],
            next: None,
            prev: None,
        };
        let cache = node.a82_snapshot();
        assert_eq!(cache, [3, 1, 2, 3, 4, 5, 6, 7]);

        let mut restored = A82CardNode {
            item: 0xaaaa_bbbb,
            owner: 0xcccc_dddd,
            ..A82CardNode::default()
        };
        restored.a82_restore(&cache);
        assert_eq!(restored.tag, 3);
        assert_eq!(restored.states, [1, 2, 3, 4, 5]);
        assert_eq!(restored.pad, [6, 7]);
        // The eight cached bytes stop before `+0x10`.
        assert_eq!(restored.item, 0xaaaa_bbbb);
        assert_eq!(restored.owner, 0xcccc_dddd);

        node.states[2] = 0xee;
        restored.a82_restore(&node.a82_snapshot());
        assert_eq!(restored.states[2], 0xee);
    }

    // -- 0x006370b2 / 0x00637286 / 0x0063729c ------------------------------

    #[test]
    fn cleanup_result_gates() {
        assert_eq!(a82_null_cleanup_result(0), Some(0));
        assert_eq!(a82_null_cleanup_result(1), None);
        assert_eq!(a82_null_release_result(0), Some(0));
        assert_eq!(a82_null_release_result(1), None);

        assert_eq!(
            a82_cleanup_if_present(false, 0x1234_5678),
            A82CleanupOutcome::Return(0x1234_5600)
        );
        assert_eq!(
            a82_cleanup_if_present(true, 0x1234_5678),
            A82CleanupOutcome::CleanAndAbort
        );
    }

    // -- 0x006371a2 / 0x006372e1 -------------------------------------------

    #[test]
    fn pool_index_is_one_based() {
        assert_eq!(a82_pool_index(0, A82_POOL_BASE, A82_SET_STRIDE), 0);
        assert_eq!(
            a82_pool_index(A82_POOL_BASE, A82_POOL_BASE, A82_SET_STRIDE),
            1
        );
        assert_eq!(
            a82_pool_index(
                A82_POOL_BASE + 2 * A82_SET_STRIDE,
                A82_POOL_BASE,
                A82_SET_STRIDE
            ),
            3
        );
        assert_eq!(
            a82_pool_index(A82_NODE_POOL_BASE + 5 * 0x18, A82_NODE_POOL_BASE, 0x18),
            6
        );
    }

    #[test]
    fn set_counts_alternate_zero_and_count() {
        let bounds = [
            (0x1000, 0x1000 + 2 * A82_SET_STRIDE),
            (0x2000, 0),
            (0x3000, 0x3000),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
        ];
        let counts = a82_card_set_counts(&bounds);
        assert_eq!(counts[0], 0);
        assert_eq!(counts[1], 3);
        assert_eq!(counts[2], 0);
        assert_eq!(counts[3], 0);
        assert_eq!(counts[4], 0);
        assert_eq!(counts[5], 1);
        for index in (6..16).step_by(2) {
            assert_eq!(counts[index], 0);
            assert_eq!(counts[index + 1], 0);
        }
    }

    // -- 0x00637510 ---------------------------------------------------------

    #[test]
    fn issue_slot_gate_thresholds_and_sentinels() {
        use A82IssueDecision::*;
        assert_eq!(
            a82_issue_slot_decision(false, 0x17, 1, 1000, 1000, true, false, false),
            Reject
        );
        assert_eq!(
            a82_issue_slot_decision(true, 0x16, 1, 1000, 1000, true, false, false),
            Reject
        );
        assert_eq!(
            a82_issue_slot_decision(true, 0x17, -1, 599, 1000, true, false, false),
            Reject
        );
        assert_eq!(
            a82_issue_slot_decision(true, 0x17, -1, 600, 299, true, false, false),
            Reject
        );
        assert_eq!(
            a82_issue_slot_decision(true, 0x17, -1, 600, 300, false, false, false),
            Reject
        );
        assert_eq!(
            a82_issue_slot_decision(true, 0x17, -1, 600, 300, true, false, false),
            Abort
        );
        assert_eq!(
            a82_issue_slot_decision(true, 0x17, 5, 0, 0, false, true, false),
            Abort
        );
        assert_eq!(
            a82_issue_slot_decision(true, 0x17, 5, 0, 0, false, false, true),
            Issue {
                slot: 5,
                accepted: true
            }
        );
        assert_eq!(
            a82_issue_slot_decision(true, 0x17, 0xe4, 0, 0, false, false, false),
            Skip
        );
    }

    // -- 0x00637810 ---------------------------------------------------------

    #[test]
    fn node_init_matches_both_branches() {
        let card = a82_init_card_node(A82NodeInit::Card, 0x1111, 0x2222, 0);
        assert_eq!(card.tag, 3);
        assert_eq!(card.states, [0; 5]);
        assert_eq!(card.pad, [0; 2]);
        assert_eq!(card.item, 0x1111);
        assert_eq!(card.owner, 0x2222);
        assert_eq!(card.values, [0; 5]);

        let other = a82_init_card_node(A82NodeInit::Other, 0x1111, 0x2222, 0x1122_3344);
        assert_eq!(other.tag, 2);
        assert_eq!(other.states, [1, 0, 0, 0x44, 0x33]);
        assert_eq!(other.pad, [0x22, 0x11]);
        assert_eq!(other.item, 0x1111);
        assert_eq!(other.owner, 0x2222);
    }

    // -- 0x00637ae0 ---------------------------------------------------------

    #[test]
    fn card_table_uses_eight_byte_pairs() {
        let table = [10u32, 11, 0, 0, 30, 31, 40, 41];
        assert_eq!(a82_card_table_entry(true, &table, 0), Some(10));
        assert_eq!(a82_card_table_entry(true, &table, 1), None);
        assert_eq!(a82_card_table_entry(true, &table, 2), Some(30));
        assert_eq!(a82_card_table_entry(true, &table, 3), Some(40));
        assert_eq!(a82_card_table_entry(false, &table, 0), None);
        assert_eq!(a82_card_table_entry(true, &table, 99), None);
    }

    // -- 0x00637be0 ---------------------------------------------------------

    #[test]
    fn card_page_limit_thresholds() {
        assert_eq!(a82_card_page_limit(10, 0, 0, 0, 700), 0);
        assert_eq!(a82_card_page_limit(10, 0, 0, 5, 700), 1);
        assert_eq!(a82_card_page_limit(10, 100, 1, 10, 700), 2);
        assert_eq!(a82_card_page_limit(10, 100, 1, 15, 700), 3);
        assert_eq!(a82_card_page_limit(10, 100, 1, 20, 700), 4);
        // The screen-height test only guards the threshold raises.
        assert_eq!(a82_card_page_limit(10, 100, 1, 20, 800), 1);
        // A candidate below the percentage is returned unchanged.
        assert_eq!(a82_card_page_limit(8, 100, 100, 5, 700), 8);
    }

    #[test]
    fn card_slot_assignment_sequence() {
        let mut nodes = vec![node(); 4];
        assert_eq!(a82_card_assign_slots(&mut nodes, 1, 1, 2), (1, 2));
        assert_eq!(
            nodes.iter().map(|n| n.states[0]).collect::<Vec<_>>(),
            vec![2, 1, 1, 1]
        );

        let mut nodes = vec![node(); 3];
        assert_eq!(a82_card_assign_slots(&mut nodes, 2, 1, 2), (1, 2));
        assert_eq!(
            nodes.iter().map(|n| n.states[0]).collect::<Vec<_>>(),
            vec![2, 2, 1]
        );

        let mut nodes = vec![node(); 3];
        assert_eq!(a82_card_assign_slots(&mut nodes, 0, 1, 2), (1, 2));
        assert_eq!(
            nodes.iter().map(|n| n.states[0]).collect::<Vec<_>>(),
            vec![1, 1, 1]
        );
    }

    #[test]
    fn card_node_timer_branches() {
        let touched = a82_card_node_timer(b'!', 5, 1, 2, 100, false);
        assert_eq!(
            touched,
            A82CardTimer {
                timer: 100,
                byte_a: 1,
                byte_b: 2,
                refresh: false
            }
        );
        let special = a82_card_node_timer(0, 5, 1, 2, 100, true);
        assert_eq!(special.timer, 100);
        assert!(!special.refresh);

        let quiet = a82_card_node_timer(0, 5, 1, 2, 0x40, false);
        assert_eq!(quiet.timer, 5);
        assert!(!quiet.refresh);

        let expired = a82_card_node_timer(0, 5, 1, 2, 0x41, false);
        assert_eq!(
            expired,
            A82CardTimer {
                timer: 0x41,
                byte_a: 2,
                byte_b: 10,
                refresh: true
            }
        );
        assert!(a82_card_node_timer(0, 0x100, 0, 0, 0x10, false).refresh);
    }

    // -- 0x00637eb0 / 0x00637f00 -------------------------------------------

    #[test]
    fn order_refresh_gate_codes() {
        for code in [2, 0x17, 0x15, 8, 7, 0xd] {
            assert_eq!(a82_order_refresh_gate(code, false, false), (true, true));
            assert_eq!(a82_order_refresh_gate(code, true, false), (true, false));
        }
        assert_eq!(a82_order_refresh_gate(3, false, false), (false, false));
        assert_eq!(a82_order_refresh_gate(3, false, true), (true, true));
    }

    #[test]
    fn card_click_preconditions() {
        assert!(a82_card_click_precheck(3, 0x1000, 0x1000, 0, 2));
        assert!(a82_card_click_precheck(3, 0x1000, 0x2000, 0x6d, 2));
        assert!(!a82_card_click_precheck(3, 0x1000, 0x2000, 0x6c, 2));
        assert!(!a82_card_click_precheck(3, 0x1000, 0x1000, 0, 3));
        assert!(!a82_card_click_precheck(2, 0x1000, 0x1000, 0, 0));
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// The eleven rows not translated above, and why:
//
// * `FUN_00636740`: the whole body is the non-returning
//   `FUN_00c8a080(param_2, 0, 0xb3b4)`.  There is no computation to state
//   beyond the constant code already recorded by
//   [`a82_error_buffer_choice`]'s fallback arm.
// * `FUN_00636960`: the whole body is `FUN_00923df0(0xb3b4)`; a single
//   non-returning call with a constant argument.
// * `FUN_00636c22`: the continuation of `0x00636ba6`.  It reads `unaff_ESI`,
//   `unaff_EDI`, `unaff_EBP`, `unaff_EBX` and kind-tag-indexed globals
//   (`DAT_00fbd124 + n * 0x1cc`, `DAT_00fac928 + n * 0x18`) whose bases and
//   live registers belong to the previous funclet; the row does not name the
//   object it serialises.
// * `FUN_00637080`: an MSVC SEH frame installation
//   (`ExceptionList = &local_10`, handler `0x00cc71d0`) followed by the
//   non-returning `FUN_00923df0(0x5dc4, ...)`; no recoverable value work.
// * `FUN_006370ad`: ledger status `thunk`, not `decompiled_uncited`; the body
//   is the bare `FUN_00923df0()` forwarder.
// * `FUN_00637324`: only `ExceptionList = *(void **)(ebp - 0xc); return 0;`.
//   The constant zero is the cleanup result, not a computation.
// * `FUN_00637516`: the `__usercall` duplicate of `0x00637510`.  Its stack
//   arguments are the same player/unit/key/card words and every guard is the
//   one reproduced by [`a82_issue_slot_decision`]; the only difference is the
//   argument-passing convention, which leaves no behaviour to translate.
// * `FUN_00637b70`: the only recovered arithmetic is the `used <= cap` compare;
//   the body then walks an out-of-batch object chain through
//   `func_0x005b0fb0`, `FUN_005a3180`, `FUN_005b6610` and `FUN_006547b0`,
//   none of which is in the window, so the loop result is not recoverable.
// * `FUN_00637e8f`: an overlapping-instruction continuation of the
//   `0x00637be0` slot loop.  It reads caller `CF`/`ZF`, `unaff_ESI`,
//   `unaff_EBX` and `unaff_EBP`, and its body starts in the middle of the
//   previous function's state updates.  The clean loop is
//   [`a82_card_assign_slots`].
// * `FUN_00637f93`: the overlapping continuation of `0x00637f00`; the
//   decompiler begins with `9 < ((byte)eax & 0xf) | in_af` and a
//   `CONCAT31` of an unnamed input, so the first-order arguments are lost.
//   The readable precondition is [`a82_card_click_precheck`].
// * `FUN_00637fea`: the tail of `0x00637f00` (`FUN_00635150` followed by the
//   non-returning `FUN_005b4cd0`), reading `unaff_EBP`/`unaff_EDI` only.

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

// ===========================================================================
// Merged from src/translated/agent86.rs
// ===========================================================================
// Translation batch 86.  Only this file is edited by the batch.
//
// Batch 86 owns `0x0063e000..=0x0063ffff`.  The ledger lists 23
// `decompiled_uncited` rows in that window; 22 are translated below and the
// single row that is not is listed at the end with its reason.
//
// The window sits in the AI-script interpreter and the `CUnit` command
// helpers that feed it.  Seven of the rows are whole-function obfuscated
// bodies that Ghidra split into `~2.5k`-line listings: `0x0063e2e2`,
// `0x0063e35a`, `0x0063e565` and `0x0063eb0a`/`0x0063ed3c`/`0x0063efa0`/
// `0x0063f269`.  Those listings contain three things that survive the damage
// and are reproduced exactly here as pure arithmetic:
//
// * the integrity / cookie preamble (`0x93af10f8` seed, `0x93af` fold,
//   `0x00f7ef98` table index pair, `0x112da` bias, always-taken guard);
// * the `units.dat` records they walk (`0x34` stride, `u16` limits at
//   `+0x0c`/`+0x0e`, two counters at `+0x14`/`+0x16`);
// * the shared opcode dispatch (112 cases, `0x24` absent, `0x3e`/`0x46`/
//   `0x4c` aliased, `0xe4` unit-type operand bound) and the pool scan
//   (`count = raw ^ (key + 0x4163f040)`, `0xdce84bd8` slot tag,
//   `(i + 1) % count` ring advance) plus the `FNV-1a` recurrence with
//   basis `0x811c9dc5` and prime `0x01000193`.
//
// The other 15 translated rows are ordinary `CUnit` / command handlers.
// Recovered state used by them:
//
// | owner          | words                                                        |
// |----------------|--------------------------------------------------------------|
// | `CUnit`        | `+0x4c` owner, `+0x4d` type byte, `+0x5c`, `+0x64` type u16, `+0x6c` next, `+0x74`, `+0x7c`, `+0x92` mask, `+0x93` flags, `+0xa2` energy `Q8`, `+0xa3` byte, `+0xa6` state byte, `+0xdc` status dword, `+0x134` order target |
// | region chunks  | `map+0x0c` `u16[0x100]` grid, `map+0x1400c` 6-byte split entries      |
// | tile records   | `map+tile*0x40+0x44a0c` / `+0x44a10`, world = `>> 8`                 |
// | per-player     | `0x00faa410[player*0x4e8] & 0x20`, `0x00f8fa08[type*4]` masks       |
//
// Address to function:
//
// * `0x0063e2e2` [`a86_cookie_indices`], [`a86_cookie_mix`],
//   [`a86_cookie_gate`], [`a86_completed_flag`], [`a86_completed_gate`]
// * `0x0063e35a` [`a86_entry_tag_compare`], [`a86_entry_tag_taken`]
// * `0x0063e565` [`a86_units_dat_record`], [`a86_units_dat_top_up`]
// * `0x0063e838` [`a86_retry_guard`]
// * `0x0063e875` [`a86_returns_immediately`]
// * `0x0063e880` [`a86_unit_kind_or_player_flag`]
// * `0x0063eb0a` [`a86_opcode_valid`], [`a86_opcode_group`],
//   [`a86_unit_type_in_range`], [`a86_opcode_reads_unit_type`],
//   [`a86_decode_stride`]
// * `0x0063ed3c` [`a86_script_table_base`], [`a86_stride_3c_fits`]
// * `0x0063efa0` [`a86_fnv1a32`]
// * `0x0063f269` [`a86_pool_count`], [`a86_pool_tag_matches`],
//   [`a86_pool_ring_next`], [`a86_pool_pair_index`], [`a86_pool_not_found`]
// * `0x0063f7b0` [`a86_udivmod64`], [`a86_f7b0_divisor`]
// * `0x0063f7e0` [`a86_orphan_len`], [`a86_orphan_sweep`]
// * `0x0063f874` [`a86_orphan_sweep_from`]
// * `0x0063f8b0` [`a86_validate_target_list`]
// * `0x0063f950` [`a86_tile_world`], [`a86_chunk_index`],
//   [`a86_subtile_bit`], [`a86_region_at`], [`a86_region_unit_query`],
//   [`a86_region_unit_return`]
// * `0x0063fa20` [`a86_point_unit_return`]
// * `0x0063fad0` [`a86_area_mark_slot`]
// * `0x0063fb10` [`a86_lift_transition`]
// * `0x0063fb70` [`a86_ac_continues`], [`a86_ac_scan`],
//   [`a86_command_dispatch`]
// * `0x0063ff60` [`a86_voice_word`], [`a86_ability_gate`]
// * `0x0063ffa0` [`a86_mark_once`]
// * `0x0063fff0` [`a86_release_slot`]
//
// No `sim` record gains a field; every item below is local to this batch.

// ---------------------------------------------------------------------------
// 0x0063e2e2 - integrity preamble and the frame-cookie fold
// ---------------------------------------------------------------------------

/// The read-only probe address the preamble of `0x0063e2e2` and `0x0063e565`
/// stores at `EBP-0x64` (the listing prints the slot as `-100`, decimal) as
/// `&DAT_0f81369a`, and later feeds to `FUN_004a97c0`.
pub const A86_GUARD_PROBE: u32 = 0x0f81_369a;

/// The plaintext word the same two rows store at `EBP-0x80` (`0x93af10f8`)
/// and xor into the recomputed cookie.
pub const A86_GUARD_SEED: u32 = 0x93af_10f8;

/// The low-half fold constant (`uVar17 - 0x93af`).
pub const A86_GUARD_FOLD: u32 = 0x93af;

/// `0x0063e2e2`/`0x0063e565`: the two `0x00f7ef98` table indices taken out of
/// the probe word.
///
/// ```c
/// table[probe & 0xfff]        // low half of the cookie table entry
/// table[probe >> 0x14]        // entry used by the second guard
/// *(ushort *)(&DAT_00f7ef9a + (probe & 0xfff))   // high half, same entry
/// ```
///
/// `0x00f7ef9a` is `0x00f7ef98 + 2`, i.e. the second `u16` of the same
/// 4-byte cookie-table entry.
pub fn a86_cookie_indices(probe: u32) -> (usize, usize) {
    ((probe & 0x0fff) as usize, (probe >> 20) as usize)
}

/// `0x0063e2e2`/`0x0063e565`: recompute the cookie word held at `EBP-0x0c`.
///
/// ```c
/// low   = field & 0xffff;
/// field = ((field >> 0x10 ^ table[probe & 0xfff] ^ low ^ (low - 0x93af)) << 0x10
///          | low) ^ seed;
/// ```
///
/// The `|` sits inside explicit parentheses in the listing, so the shift and
/// the `or` bind before the final `^ seed`.  `table` is the `u16` read at
/// `&DAT_00f7ef98 + (probe & 0xfff)` (or `&DAT_00f7ef9a` for the high half,
/// which is the caller's choice, not the fold's).
pub fn a86_cookie_mix(field: u32, table_word: u16, seed: u32) -> u32 {
    let low = field & 0xffff;
    let mixed = (field >> 16) ^ (table_word as u32) ^ low ^ low.wrapping_sub(A86_GUARD_FOLD);
    ((mixed << 16) | low) ^ seed
}

/// The verdict of the preamble's second guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A86CookieGate {
    /// The guard compared equal and the low half was zero, so the row's body
    /// is skipped.
    Clean,
    /// The shipped branch was taken; the row falls into the integrity abort.
    Dirty,
}

/// `0x0063e2e2`/`0x0063e565`: the second guard, verbatim.
///
/// ```c
/// if (((field >> 0x10 ^ low * 2 + table[probe >> 0x14] * -0x200) & 0xffff) !=
///     (table[probe & 0xfff] ^ low) || low != 0)
/// ```
///
/// `low_word` is `table[probe & 0xfff]` and `high_word` is
/// `table[probe >> 0x14]`; [`a86_cookie_indices`] produces the two indices.
/// `-0x200` is `0xfffffe00`, i.e. the table entry shifted into the high half
/// and subtracted, which is how the compiler expressed `- (entry << 9)`.
pub fn a86_cookie_gate(field: u32, low_word: u16, high_word: u16) -> A86CookieGate {
    let low = field & 0xffff;
    let mixed = (field >> 16)
        ^ low
            .wrapping_mul(2)
            .wrapping_add((high_word as u32).wrapping_mul(0xffff_fe00));
    let equal = (mixed & 0xffff) == ((low_word as u32) ^ low);
    if equal && low == 0 {
        A86CookieGate::Clean
    } else {
        A86CookieGate::Dirty
    }
}

/// `CUnit+0xdc >> 4 & 1`, the completed-construction bit, which is the value
/// `FUN_005b5840` returns at the tail of `0x0063e2e2`, `0x0063e35a` and
/// `0x0063e565`.
pub fn a86_completed_flag(field_dc: u32) -> u32 {
    (field_dc >> 4) & 1
}

/// The tail of `0x0063e2e2`, `0x0063e35a` and `0x0063e565`: the flag is
/// returned when set, and a clear flag reaches the `0x0063e838` continuation
/// that the row pushes as the return address immediately before `FUN_005b4cd0`.
pub fn a86_completed_gate(field_dc: u32) -> Result<u32, u32> {
    match a86_completed_flag(field_dc) {
        0 => Err(A86_CONTINUATION),
        flag => Ok(flag),
    }
}

/// The continuation address both guards leave on the stack (`0x0063e838`).
pub const A86_CONTINUATION: u32 = 0x0063_e838;

// ---------------------------------------------------------------------------
// 0x0063e35a - the second entry tag check
// ---------------------------------------------------------------------------

/// The literal low half `0x0063e35a` builds with `CONCAT31(uVar6, 0x6c)`.
pub const A86_ENTRY_TAG: u32 = 0x6c;

/// The bias the same row adds before comparing (`0x112da`).
pub const A86_ENTRY_TAG_BIAS: u32 = 0x0001_12da;

/// `0x0063e35a`: the comparison half of the entry guard.
///
/// ```c
/// ((high ^ (0x6c + 0x112da)) & 0xffff) != (table[probe >> 0x14] ^ 0x6c)
/// ```
///
/// Returns `true` when the two halves differ, i.e. when the guard fires.
pub fn a86_entry_tag_compare(high: u32, table_word: u16) -> bool {
    let low = A86_ENTRY_TAG;
    ((high ^ low.wrapping_add(A86_ENTRY_TAG_BIAS)) & 0xffff) != ((table_word as u32) ^ low)
}

/// `0x0063e35a`: the whole guard.
///
/// The listing ends the condition with `|| uVar5 != 0` and `uVar5` is the
/// literal `0x6c`, so the `0x0063e35a` guard is a tautology and the row
/// always runs its integrity path regardless of the tag values.  The constant
/// is returned as data instead of a `true` literal so the recovery stays
/// visible.
pub fn a86_entry_tag_taken(high: u32, table_word: u16) -> bool {
    a86_entry_tag_compare(high, table_word) || A86_ENTRY_TAG != 0
}

// ---------------------------------------------------------------------------
// 0x0063e565 - the units.dat record pair it tops up
// ---------------------------------------------------------------------------

/// `units.dat` record stride (`uVar24 * 0x34 + iVar9`).
pub const A86_UNITS_DAT_STRIDE: u32 = 0x34;

/// The row's payload offset: the first statement is
/// `*(int *)(EBP - 0x9c) = in_EAX + 0x1c`.
pub const A86_E565_PAYLOAD: u32 = 0x1c;

/// `0x0063e565`: `units.dat` record address for a unit type.
///
/// ```c
/// pcVar34 = (code *)((uint)uVar24 * 0x34 + iVar9);   // iVar9 = table base
/// ```
pub fn a86_units_dat_record(unit_type: u16, table_base: u32) -> u32 {
    (unit_type as u32)
        .wrapping_mul(A86_UNITS_DAT_STRIDE)
        .wrapping_add(table_base)
}

/// `0x0063e565`: one of the two counter top-up loops.
///
/// ```c
/// uVar1  = *(ushort *)(record + 0xe);      // limit
/// uVar11 = *(ushort *)(unit + 0x16);       // counter
/// if (uVar1 != 0 && uVar11 < uVar1) {
///     do {
///         iVar9  = FUN_006523f0(...);      // step
///         uVar11 = uVar11 + iVar9;
///         uVar17 = *(ushort *)(record + 0xe);   // limit re-read
///     } while (uVar11 < uVar17);
/// }
/// ```
///
/// The second loop is identical with `+0xc` and the unit's `+0x14` counter.
/// The limit is reloaded from the record on every pass, so a step of zero
/// spins rather than terminating; that is reproduced rather than hidden.
pub fn a86_units_dat_top_up(current: u16, limit: u16, mut step: impl FnMut() -> u32) -> u32 {
    let mut value = current as u32;
    let mut bound = limit as u32;
    if bound == 0 || value >= bound {
        return value;
    }
    loop {
        value = value.wrapping_add(step());
        bound = limit as u32;
        if value >= bound {
            return value;
        }
    }
}

// ---------------------------------------------------------------------------
// 0x0063e838 - the continuation the three guards jump to
// ---------------------------------------------------------------------------

/// The two unit type bytes the continuation matches (`0x9c`, i.e. `-100` as a
/// signed byte, and `2`).
pub const A86_RETRY_TYPES: [u8; 2] = [0x9c, 0x02];

/// The `in_AX` value the continuation skips (`0x29`).
pub const A86_RETRY_SKIP_AX: u16 = 0x29;

/// The command byte and the order the unreachable half would post
/// (`FUN_00534aa0(0xb, owner)` and `FUN_005b7d00(0x74)`).
pub const A86_RETRY_COMMAND: u8 = 0x0b;
pub const A86_RETRY_ORDER: u16 = 0x74;

/// `0x0063e838`: the retry gate.
///
/// ```c
/// if (in_AX != 0x29 && (*(char *)(unit + 0x4d) == 0x9c || == 2)) {
///     FUN_005bb740();        // FUN_005bb740 tail-calls FUN_0058b640, noreturn
///     ...                    // dead
/// }
/// ```
///
/// `FUN_005bb740` is an eight-byte tail call into the noreturn `FUN_0058b640`, so
/// the `FUN_00534aa0(0xb, ...)` / `FUN_005b7d00(0x74)` half can never run.
/// The recovered predicate is what the row actually decides.
pub fn a86_retry_guard(ax: u16, unit_type: u8) -> bool {
    ax != A86_RETRY_SKIP_AX && A86_RETRY_TYPES.contains(&unit_type)
}

// ---------------------------------------------------------------------------
// 0x0063e875 - four-byte empty body
// ---------------------------------------------------------------------------

/// `0x0063e875`: the whole row is `retn` (four bytes, no stack adjustment).
/// It is kept as an explicit no-op so callers that model the dispatch slot
/// have a defined target.
pub fn a86_returns_immediately() {}

// ---------------------------------------------------------------------------
// 0x0063e880 - owner flag fallback
// ---------------------------------------------------------------------------

/// `0x00faa410` per-player stride used by `FUN_00635720`.
pub const A86_PLAYER_STRIDE: u32 = 0x4e8;

/// The bit `FUN_00635720` masks out (`0x20`).
pub const A86_PLAYER_FLAG_BIT: u16 = 0x20;

/// `0x0063e880`: the row's return value.
///
/// ```c
/// uVar3 = FUN_00635720(*(byte *)(unit + 0x4c));   // player word & 0x20
/// if (uVar3 == 0) {
///     bVar1 = *(byte *)(unit + 0x4d);             // unit type byte
///     uVar3 = bVar1;
///     ...                                         // 0x9c/2 branch aborts
/// }
/// return uVar3;
/// ```
///
/// `FUN_00635720` is `*(ushort *)(&DAT_00faa410 + player * 0x4e8) & 0x20`, so
/// a set bit yields `0x20` and a clear bit yields the unit type byte widened
/// to `u32`.
pub fn a86_unit_kind_or_player_flag(player_word: u16, unit_type: u8) -> u32 {
    let flag = (player_word & A86_PLAYER_FLAG_BIT) as u32;
    if flag == 0 { unit_type as u32 } else { flag }
}

// ---------------------------------------------------------------------------
// 0x0063eb0a / 0x0063ed3c / 0x0063efa0 / 0x0063f269 - opcode dispatch
// ---------------------------------------------------------------------------

/// Highest opcode byte the dispatch accepts (`0x70`).
pub const A86_OPCODE_MAX: u8 = 0x70;

/// The one gap in the dispatch table: `0x24` has no `case`.
pub const A86_OPCODE_HOLE: u8 = 0x24;

/// The cases that share one body (`case 0x3e: case 0x46: case 0x4c:`).
pub const A86_OPCODE_ALIASES: [u8; 3] = [0x3e, 0x46, 0x4c];

/// Number of `case` labels the listing carries, aliases included.
pub const A86_OPCODE_CASES: usize = 112;

/// The unit-type operand bound the dispatch tests (`< 0xe4`).
pub const A86_UNIT_TYPE_LIMIT: u16 = 0xe4;

/// The case labels whose body validates a `u16` unit-type operand against
/// `0xe4`, in table order.  Extracted from the `&DAT_000000e4` stores in
/// `0x0063eb0a`, `0x0063efa0` and `0x0063f269`; all three listings agree.
pub const A86_UNIT_TYPE_BOUNDED: [u8; 15] = [
    0x0a, 0x0c, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x45, 0x4b, 0x4c, 0x4e, 0x53,
];

/// `0x0063eb0a`: is the opcode byte a `case` of the dispatch?
///
/// The listing has `0x00..=0x23` and `0x25..=0x70`, which is 112 labels.
pub fn a86_opcode_valid(opcode: u8) -> bool {
    opcode <= A86_OPCODE_MAX && opcode != A86_OPCODE_HOLE
}

/// `0x0063eb0a`: the canonical body for an opcode.
///
/// `0x46` and `0x4c` have no body of their own; they fall into `0x3e`.
pub fn a86_opcode_group(opcode: u8) -> u8 {
    if A86_OPCODE_ALIASES[1..].contains(&opcode) {
        A86_OPCODE_ALIASES[0]
    } else {
        opcode
    }
}

/// `0x0063eb0a`: `if (uVar2 < 0xe4)` - the unit-type operand bound.
pub fn a86_unit_type_in_range(unit_type: u16) -> bool {
    unit_type < A86_UNIT_TYPE_LIMIT
}

/// `0x0063eb0a`: does this opcode validate its first operand as a unit type?
pub fn a86_opcode_reads_unit_type(opcode: u8) -> bool {
    A86_UNIT_TYPE_BOUNDED.contains(&opcode)
}

/// `0x0063eb0a`: the repeat/stride word the prologue installs.
///
/// ```c
/// n = 1;
/// if ((DAT_00f7ea50 & 0x500) == 0) {
///     if (iVar27 != 0xfd57f28b) n = 8;
/// } else {
///     n = 0x10;
/// }
/// ```
///
/// `iVar27` is `CONCAT31(in_EAX >> 8, DAT_00f7ea50._1_1_)`, i.e. a value the
/// obfuscation seeds from caller state; the comparison is against the 32-bit
/// form of `-0x2a80d75`, which is `0xfd57f28b`.
pub fn a86_decode_stride(flags: u16, seeded: u32) -> u32 {
    if flags & 0x0500 != 0 {
        0x10
    } else if seeded != 0xfd57_f28b {
        8
    } else {
        1
    }
}

/// `0x0063ed3c`: the tagged script-table pointer base.
///
/// The row reads the table through `DAT_00f063cc ^ 0x10fae560`; the same
/// xor-obfuscated global is used by every unit-table read in the window.
pub fn a86_script_table_base(stored: u32) -> u32 {
    stored ^ 0x10fa_e560
}

/// `0x0063ed3c`, `case 0x31`: the record bound test.
///
/// ```c
/// if ((uint)bVar12 * 0x3c <= *(uint *)(table + 0xe608)) { ... }
/// ```
///
/// `0x3c` is the stride of the record the byte indexes; `+0xe608` is the
/// stored record count/limit in the same tagged table.
pub fn a86_stride_3c_fits(index: u8, limit: u32) -> bool {
    (index as u32).wrapping_mul(0x3c) <= limit
}

/// `0x0063efa0`: the 32-bit `FNV-1a` recurrence the row runs over the head
/// (up to a 16-byte boundary) and the tail of the buffer.
///
/// ```c
/// uVar13 = 0x811c9dc5;
/// do { uVar13 = (*pbVar20 ^ uVar13) * 0x1000193; } while (++i < len);
/// ```
///
/// The middle of the row is the compiler's 16-byte unrolled form of the same
/// recurrence: it xors a whole 16-byte block and multiplies each `u16` lane by
/// the constant vector at `0x00d3c560`, then folds four dwords with `^`.  That
/// constant vector is outside the row, so only the scalar recurrence is
/// reproduced; it is the one the row uses for every byte before the first
/// aligned block and after the last one.
pub const A86_FNV_OFFSET: u32 = 0x811c_9dc5;

/// `FNV-1a` prime for the same row.
pub const A86_FNV_PRIME: u32 = 0x0100_0193;

/// `0x0063efa0`: scalar `FNV-1a` over `bytes`, from `A86_FNV_OFFSET`.
pub fn a86_fnv1a32(bytes: &[u8]) -> u32 {
    let mut hash = A86_FNV_OFFSET;
    for &byte in bytes {
        hash = (byte as u32 ^ hash).wrapping_mul(A86_FNV_PRIME);
    }
    hash
}

/// `0x0063f269`: the slot tag the pool scan looks for (`0xdce84bd8`).
pub const A86_POOL_TAG: u32 = 0xdce8_4bd8;

/// `0x0063f269`: the bias added to the pool key before the xor
/// (`0x4163f040`).
pub const A86_POOL_COUNT_BIAS: u32 = 0x4163_f040;

/// `0x0063f269`/`0x0063eb0a`: the ring length.
///
/// ```c
/// count = *(uint *)(pool + 0xfc) ^ (DAT_00f7f79e + 0x4163f040);
/// ```
///
/// `+` binds tighter than `^`, so the bias is added to the key first.
pub fn a86_pool_count(raw: u32, pool_key: u32) -> u32 {
    raw ^ pool_key.wrapping_add(A86_POOL_COUNT_BIAS)
}

/// `0x0063f269`: the slot tag test.
///
/// ```c
/// if ((*(uint *)(slot * 8 + 4 + base) ^ _DAT_00f7fcf6 ^ 0xdce84bd8) != 0) break;
/// ```
///
/// The scan keeps walking while the expression is non-zero, so a slot is valid
/// exactly when the tag equals `key ^ 0xdce84bd8`.
pub fn a86_pool_tag_matches(raw_tag: u32, tag_key: u32) -> bool {
    raw_tag ^ tag_key ^ A86_POOL_TAG == 0
}

/// `0x0063f269`: the ring advance `(index + 1) % count`.
///
/// The listing computes this modulo with the tag-stripped count in two places
/// (`uStack_24` and `EBP-0x20`); a zero count would fault in the shipped code
/// and is reported as `None` here.
pub fn a86_pool_ring_next(index: u32, count: u32) -> Option<u32> {
    if count == 0 {
        None
    } else {
        Some(index.wrapping_add(1) % count)
    }
}

/// `0x0063f269`: the composite index the row multiplies once it settles on a
/// slot (`*(int *)(EBP-0x20) * *(int *)(EBP-0x24)`) and the value it returns
/// when the scan found nothing (`DAT_00f7f79e + 0x4163f040`).
pub fn a86_pool_pair_index(inner: u32, outer: u32) -> u32 {
    inner.wrapping_mul(outer)
}

/// `0x0063f269`: the scan's "nothing found" return value.
pub fn a86_pool_not_found(pool_key: u32) -> u32 {
    pool_key.wrapping_add(A86_POOL_COUNT_BIAS)
}

// ---------------------------------------------------------------------------
// 0x0063f7b0 - the frame sweep
// ---------------------------------------------------------------------------

/// `0x0063f7b0`: the mask applied to the frame word (`0x82b`).
pub const A86_F7B0_MASK: u32 = 0x082b;

/// `0x0063f7b0`: the second frame word, `0x83b ^ 0x82b`, which folds to
/// `0x10`.
pub const A86_F7B0_SECOND: u32 = 0x083b ^ 0x082b;

/// `0x0063f7b0`: the 64-bit divisor the row derives from the masked word.
///
/// ```c
/// *(uint *)(EBP-8) ^= 0x82b;
/// _aulldvrm(rdtsc_lo, rdtsc_hi, *(int *)(EBP-8), *(int *)(EBP-8) >> 0x1f);
/// ```
///
/// `FUN_00c87420` is `_aulldvrm`: it returns the quotient in `EAX` and the
/// remainder in `EDX` for a 64-by-64 unsigned divide, and the fourth argument
/// supplies the divisor's high half, so the divisor is the masked word
/// sign-extended to 64 bits.
pub fn a86_f7b0_divisor(word: u32) -> u64 {
    (word ^ A86_F7B0_MASK) as i32 as i64 as u64
}

/// `0x0063f7b0`: the divide itself, with `_aulldvrm`'s `(quotient,
/// remainder)` result.  A zero divisor raises `#DE` on the shipped CPU and is
/// reported as `None`.
pub fn a86_udivmod64(numerator: u64, divisor: u64) -> Option<(u64, u64)> {
    let quotient = numerator.checked_div(divisor)?;
    let remainder = numerator.checked_rem(divisor)?;
    Some((quotient, remainder))
}

// ---------------------------------------------------------------------------
// 0x0063f7e0 / 0x0063f874 - the orphan sweeps
// ---------------------------------------------------------------------------

/// `0x0063f7e0`: `0x182b ^ 0x82b`, the bias subtracted from the range length.
pub const A86_ORPHAN_BIAS: i32 = 0x182b ^ 0x082b;

/// `0x0063f7e0`/`0x0063f874`: `0x8d4 ^ 0x82b`, the failure code.
pub const A86_ORPHAN_FAIL: u32 = 0x08d4 ^ 0x082b;

/// The six stack slots a sweep probe must avoid (the `EBP-0x28`, `-0x68`,
/// `-0x6c`, `-0x70`, `-0x74` and `-0x78` addresses of the frame that owns the
/// container).
pub const A86_ORPHAN_SLOTS: usize = 6;

/// `0x0063f7e0`: the sweep length.
///
/// ```c
/// end = *(int *)(container + 8) + offset + count;      // EBP-0x28
/// begin = *(int *)(container + 4);
/// len = (begin - end) - 0x1000;
/// ```
///
/// The `0x1000` is a debug-range bias, not a byte count, so `len` is usually
/// negative; the loop below compares it as an unsigned word exactly as the
/// listing does.
pub fn a86_orphan_len(begin: i32, end: i32) -> i32 {
    begin.wrapping_sub(end).wrapping_sub(A86_ORPHAN_BIAS)
}

/// `0x0063f7e0`: walk the sweep and report the first probe that collides with
/// one of the six frame slots.
///
/// ```c
/// for (i = 0; i != len; i += 4) {
///     p = end + i;
///     if (p == EBP-0x28 || p == EBP-0x68 || ... ) abort(0xff);
/// }
/// ```
///
/// Returns `Err(A86_ORPHAN_FAIL)` on the collision and `Ok(probes)` with the
/// number of probes taken otherwise.
pub fn a86_orphan_sweep(end: u32, len: i32, slots: &[u32; A86_ORPHAN_SLOTS]) -> Result<u32, u32> {
    if len == 0 {
        return Ok(0);
    }
    let mut index: u32 = 0;
    let mut probes = 0;
    loop {
        let probe = end.wrapping_add(index);
        if slots.contains(&probe) {
            return Err(A86_ORPHAN_FAIL);
        }
        probes += 1;
        index = index.wrapping_add(4);
        if index >= len as u32 {
            return Ok(probes);
        }
    }
}

/// `0x0063f874`: the same sweep entered from a stored index.
///
/// ```c
/// *out = in_AL;                       // one byte stored before the walk
/// i = *(int *)(EBP-0xc);
/// loop {
///     next = i + 1;
///     if (len <= next) { unlock(); return; }
///     p = first + next;
///     if (p collides) abort(0xff);
///     i += 4;
/// }
/// ```
///
/// The counter advances by four but the probe uses `i + 1`, so the probes are
/// `first + start + 1 + 4k` while `start + 1 + 4k < len`.
pub fn a86_orphan_sweep_from(
    start: u32,
    len: u32,
    first: u32,
    slots: &[u32; A86_ORPHAN_SLOTS],
) -> Result<u32, u32> {
    let mut counter = start;
    let mut probes = 0;
    loop {
        let next = counter.wrapping_add(1);
        if len <= next {
            return Ok(probes);
        }
        let probe = first.wrapping_add(next);
        if slots.contains(&probe) {
            return Err(A86_ORPHAN_FAIL);
        }
        probes += 1;
        counter = counter.wrapping_add(4);
    }
}

// ---------------------------------------------------------------------------
// 0x0063f8b0 - target list validation walk
// ---------------------------------------------------------------------------

/// One record of the list rows `0x0063f8b0`, `0x0063f950` and `0x0063fa20`
/// walk.  Each field is one of the predicates the shipped loop calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86ListUnit {
    /// `FUN_005a3180`: the record is gone (dead, or `+0x4d == 0`).
    pub gone: bool,
    /// `FUN_005b6370`: `0x00f8fa08[+0x64] & 0x10`, the row's skip mask.
    pub hidden: bool,
    /// `FUN_005b5b90(player, unit)`: the owner test.
    pub matched: bool,
    /// `FUN_0075d940(FUN_0075d7b0(unit))`: the tile marker is `0x1ffd`.
    pub has_target: bool,
}

/// `0x0063f8b0`: the walk's outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A86ListVerdict {
    /// The walk reached the null link and the row returns `0`.
    Exhausted,
    /// A matching record failed the target test and the row reaches
    /// `FUN_0058b650`.
    TargetMissing(usize),
}

/// `0x0063f8b0`: the validation walk.
///
/// ```c
/// while (true) {
///     if (unit == 0) return 0;
///     if (!gone && !hidden && match(player, unit)) {
///         tile = FUN_0075d7b0(unit);
///         if (!FUN_0075d940(tile)) FUN_0058b650();
///     }
///     unit = *(int *)(unit + 0x6c);          // FUN_005b4bb0
/// }
/// ```
///
/// The match does not terminate the walk: `0x0063f8b0` is a checker, not a
/// finder, and only the `FUN_0058b650` failure exits early.  The link word is
/// `+0x6c`, unlike the `+4` link the two nearest-unit rows follow.
pub fn a86_validate_target_list(units: &[A86ListUnit]) -> A86ListVerdict {
    for (index, unit) in units.iter().enumerate() {
        if unit.gone || unit.hidden || !unit.matched {
            continue;
        }
        if !unit.has_target {
            return A86ListVerdict::TargetMissing(index);
        }
    }
    A86ListVerdict::Exhausted
}

// ---------------------------------------------------------------------------
// 0x0063f950 / 0x0063fa20 - region and point queries
// ---------------------------------------------------------------------------

/// One 64-byte tile record as `FUN_0075d630` reads it: the two scaled position
/// words at `+0x0c` and `+0x10` of `map + tile * 0x40 + 0x44a0c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86Tile {
    /// `*(int *)(map + tile * 0x40 + 0x44a0c)`.
    pub x_word: i32,
    /// `*(int *)(map + tile * 0x40 + 0x44a10)`.
    pub y_word: i32,
}

/// `0x0063f950`: tile to world.  `FUN_0075d630` shifts both stored words right
/// by eight to drop the `1/256` sub-pixel fraction.
pub fn a86_tile_world(tile: &A86Tile) -> (i32, i32) {
    (tile.x_word >> 8, tile.y_word >> 8)
}

/// `0x0063f950`: chunk-grid width (`0x100` entries of `u16` at `map + 0x0c`).
pub const A86_CHUNK_WIDTH: u32 = 0x100;

/// `0x0063f950`: `FUN_0075d720`'s split threshold.  A chunk word above `0x1fff`
/// is an index into the six-byte split entries, not a region id.
pub const A86_REGION_SPLIT_MIN: u16 = 0x2000;

/// One six-byte split entry at `map + 0x1400c + index * 6`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86RegionSplit {
    /// `+0x00`: the 4x4 sub-tile bitmap.
    pub mask: u16,
    /// `+0x02`: the region returned when the bit is clear.
    pub outside: u16,
    /// `+0x04`: the region returned when the bit is set.
    pub inside: u16,
}

/// `0x0063f950`: the chunk index.
///
/// ```c
/// ((y + (y >> 31 & 0x1f)) >> 5) * 0x100 + ((x + (x >> 31 & 0x1f)) >> 5)
/// ```
///
/// The `>> 31 & 0x1f` term is the compiler's signed-divide bias.  It is not a
/// floor for every negative input: `-1` maps to chunk `0`, and only `-32`
/// reaches chunk `-1`.
pub fn a86_chunk_index(x: i32, y: i32) -> u32 {
    let cx = (x + ((x >> 31) & 0x1f)) >> 5;
    let cy = (y + ((y >> 31) & 0x1f)) >> 5;
    (cy as u32)
        .wrapping_mul(A86_CHUNK_WIDTH)
        .wrapping_add(cx as u32)
}

/// `0x0063f950`: the 4x4 sub-tile bit, `((x >> 3) & 3) + ((y >> 3) & 3) * 4`.
pub fn a86_subtile_bit(x: i32, y: i32) -> u32 {
    let bx = ((x + ((x >> 31) & 0x07)) >> 3) & 3;
    let by = ((y + ((y >> 31) & 0x07)) >> 3) & 3;
    (bx + by * 4) as u32
}

/// `0x0063f950`: `FUN_0075d720`, the region id for a world point.
///
/// ```c
/// uVar2 = chunks[(y >> 5) * 0x100 + (x >> 5)];
/// if (0x1fff < uVar2) {
///     entry = splits[uVar2];
///     if (entry.mask & (1 << bit)) return entry.inside;
///     return entry.outside;
/// }
/// return uVar2;
/// ```
///
/// An out-of-range chunk index or split index would read past the map in the
/// shipped code; the Rust form reports `None` instead of inventing a value.
pub fn a86_region_at(chunks: &[u16], splits: &[A86RegionSplit], x: i32, y: i32) -> Option<u16> {
    let chunk = *chunks.get(a86_chunk_index(x, y) as usize)?;
    if chunk < A86_REGION_SPLIT_MIN {
        return Some(chunk);
    }
    let split = *splits.get(chunk as usize)?;
    if split.mask & (1 << a86_subtile_bit(x, y)) != 0 {
        Some(split.inside)
    } else {
        Some(split.outside)
    }
}

/// The sentinel both query rows write into the "best distance" slot
/// (`local_8 = 99999999`).
pub const A86_BEST_DISTANCE_INIT: i32 = 99_999_999;

/// The initial "best unit" slot (`local_c = 0`).
pub const A86_BEST_UNIT_INIT: u32 = 0;

/// `0x0063f950`'s "nothing found" return (`return 0xffffffff`).
pub const A86_NO_UNIT: u32 = 0xffff_ffff;

/// `0x0063fa20`'s "nothing found" return (`return 0`).
pub const A86_NO_POINT_UNIT: u32 = 0;

/// One candidate of the two nearest-unit rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86CandidateUnit {
    /// `FUN_005a3180`.
    pub gone: bool,
    /// `FUN_005b5810`: `CUnit+0xdc >> 1 & 1`.
    pub active: bool,
    /// `FUN_005b5b90(player, unit)`.
    pub matched: bool,
}

/// `0x0063f950`/`0x0063fa20`: the filter chain.
///
/// ```c
/// for (unit = head; unit != 0; unit = *(int *)(unit + 4)) {
///     if (!gone && active && matched) break;
/// }
/// if (unit == 0) return <row sentinel>;
/// FUN_0058b650();                    // the listing's noreturn cut
/// ```
///
/// Both rows follow the `+4` link (unlike `0x0063f8b0`'s `+0x6c`), and both
/// initialise the `99999999` / `0` best-distance pair *before* the walk.  The
/// distance minimisation itself sits past the cut, so only the filter and the
/// sentinels are reproduced.
pub fn a86_region_unit_query(units: &[A86CandidateUnit]) -> Option<usize> {
    units
        .iter()
        .position(|unit| !unit.gone && unit.active && unit.matched)
}

/// `0x0063f950`: the whole row's return value given the candidate list.
///
/// `None` maps to `0xffffffff`, a hit to the matching unit's slot.
pub fn a86_region_unit_return(units: &[A86CandidateUnit]) -> u32 {
    match a86_region_unit_query(units) {
        None => A86_NO_UNIT,
        Some(index) => index as u32,
    }
}

/// `0x0063fa20`: the whole row's return value given the candidate list.
///
/// Identical filter, but the empty case returns `0` and the row takes the
/// point coordinates directly instead of converting a tile first.
pub fn a86_point_unit_return(units: &[A86CandidateUnit]) -> u32 {
    match a86_region_unit_query(units) {
        None => A86_NO_POINT_UNIT,
        Some(index) => index as u32,
    }
}

// ---------------------------------------------------------------------------
// 0x0063fad0 - area mark
// ---------------------------------------------------------------------------

/// `0x0063fad0`: the third argument the row forwards, `FUN_0063cdd0(a, b, 2)`.
pub const A86_AREA_MARK_VALUE: u8 = 2;

/// `FUN_0063cdd0`'s mark-table stride (`(param_1 + 0x131b) * 0xc`).
pub const A86_AREA_MARK_STRIDE: u32 = 0x0c;

/// `FUN_0063cdd0`'s mark-table base offset (`0x131b`).
pub const A86_AREA_MARK_BASE: u32 = 0x131b;

/// `0x0063fad0`: the offset the called helper writes for one unit.
///
/// ```c
/// *(byte *)(unit_owner + (player + 0x131b) * 0xc + table) = 2;
/// ```
///
/// The helper skips units whose `+0x4c` equals `player`, so only other owners
/// are marked.
pub fn a86_area_mark_slot(player: u8, unit_owner: u8) -> Option<u32> {
    if player == unit_owner {
        None
    } else {
        Some(
            (player as u32)
                .wrapping_add(A86_AREA_MARK_BASE)
                .wrapping_mul(A86_AREA_MARK_STRIDE)
                .wrapping_add(unit_owner as u32),
        )
    }
}

// ---------------------------------------------------------------------------
// 0x0063fb10 - lift state transition
// ---------------------------------------------------------------------------

/// `0x0063fb10`: the state byte the row requires (`'m'`).
pub const A86_LIFT_STATE_UP: u8 = 0x6d;

/// `0x0063fb10`: the state byte the row writes (`'n'`, via `FUN_005b7cd0`).
pub const A86_LIFT_STATE_DOWN: u8 = 0x6e;

/// `0x0063fb10`: the inclusive unit-type window that suppresses the write.
pub const A86_LIFT_TYPE_WINDOW: (u8, u8) = (0x7b, 0x81);

/// `0x0063fb10`: the mask `FUN_005b5950` tests (`0x00f8fa08[type] & 0x200`).
pub const A86_LIFT_FLAG: u32 = 0x200;

/// `FUN_005b7cd0`'s side effects: the three words cleared when the state byte
/// actually changes.
pub const A86_LIFT_CLEARED_FIELDS: [u16; 3] = [0x00e8, 0x00ec, 0x00e2];

/// The `CUnit` facts `0x0063fb10` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86LiftFacts {
    /// `+0xa6`, the animation/state byte.
    pub state: u8,
    /// `0x00f8fa08[+0x64] & 0x200`, from `FUN_005b5950`.
    pub flags: u32,
    /// `FUN_005a30c0(0) != 0`.
    pub pred_a: bool,
    /// `FUN_005b6a30()` - `*(int *)(+0x7c) != 0`.
    pub pred_b: bool,
    /// `FUN_005c7870(unit) != 0`.
    pub pred_c: bool,
    /// `+0x4d`, the unit type byte.
    pub unit_type: u8,
}

/// `0x0063fb10`: the state transition.
///
/// ```c
/// if (*(char *)(unit + 0xa6) == 'm') {
///     if (FUN_005b5950() != 0)
///         if (FUN_005a30c0(0) == 0)
///             if (FUN_005b6a30() == 0)
///                 if (FUN_005c7870(unit) == 0 &&
///                     (type < 0x7b || 0x81 < type))
///                     FUN_005b7cd0(0x6e);
/// }
/// ```
///
/// Returns the byte to write (`0x6e`) when every gate opens.  The write helper
/// `FUN_005b7cd0` only stores when the byte differs, which the `0x6d`/`0x6e`
/// gate guarantees, so it also clears `+0xe8`, `+0xec` and `+0xe2`.
pub fn a86_lift_transition(facts: &A86LiftFacts) -> Option<u8> {
    if facts.state != A86_LIFT_STATE_UP {
        return None;
    }
    if facts.flags & A86_LIFT_FLAG == 0 {
        return None;
    }
    if facts.pred_a || facts.pred_b || facts.pred_c {
        return None;
    }
    if A86_LIFT_TYPE_WINDOW.0 <= facts.unit_type && facts.unit_type <= A86_LIFT_TYPE_WINDOW.1 {
        return None;
    }
    Some(A86_LIFT_STATE_DOWN)
}

// ---------------------------------------------------------------------------
// 0x0063fb70 - command dispatch
// ---------------------------------------------------------------------------

/// An order/message the dispatch posts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A86Action {
    /// `FUN_005b7d00(order)`.
    Order(u16),
    /// `FUN_005b7dd0(order, operand)`.
    OrderTyped { order: u16, operand: u32 },
    /// `FUN_005b7d80(message, x, y)`.
    Message { message: u16, x: u16, y: u16 },
    /// `FUN_005b7c70(message)`.
    Post(u16),
    /// The row returns its value without posting anything.
    Nothing,
}

/// The dispatch result: the row's `return` value plus the posted action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A86DispatchResult {
    /// The row's return value (`0` or `1`).
    pub ret: u32,
    /// The posted action, if any.
    pub action: A86Action,
}

/// `0x0063fb70`: one entry of the `0xac` walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86AcEntry {
    /// `FUN_005c1f10(unit) == 0`.
    pub pred_a_clear: bool,
    /// `FUN_005b49d0(unit) != 0`.
    pub pred_b_set: bool,
    /// `FUN_00656b00(order, 0) != 0`.
    pub pred_c_set: bool,
    /// `FUN_005b4a20()` value, compared against `0x28`.
    pub value: u32,
}

/// `0x0063fb70`: the facts the row reads for one `AX` message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86DispatchInput {
    /// The message id (`in_AX`).
    pub ax: u16,
    /// `+0x4c`, the owner byte.
    pub owner: u8,
    /// `+0x4d`, the unit type byte.
    pub unit_type: u8,
    /// `+0x74`.
    pub field_74: u32,
    /// `FUN_00534aa0(3, owner) == 1`.
    pub can_3: bool,
    /// `FUN_006510c0() != 0`.
    pub pred_510c0: bool,
    /// `*(char *)(*(int *)(unit + 0x134) + 8) == 1`.
    pub target_kind_1: bool,
    /// `FUN_00659030(...) != 0`.
    pub pred_59030: bool,
    /// `FUN_00534aa0(5, owner) == 1`.
    pub can_5: bool,
    /// `FUN_00588d60()`.
    pub cmp_a: u32,
    /// `FUN_005b4b30()`.
    pub cmp_b: u32,
    /// `FUN_005c7850()` value; a non-zero result is the row's third gate and
    /// is also the operand of the typed order it posts.
    pub c7850: u32,
    /// `FUN_005d5d10() != 0`.
    pub pred_d5d10: bool,
    /// `FUN_006560f0() != 0`.
    pub pred_560f0: bool,
    /// `FUN_00591ba0()`: `0x49`, `0x55` or `0`.
    pub msg_kind: u16,
    /// `FUN_0065b2b0(msg_kind, owner) == 1`.
    pub can_msg: bool,
    /// `FUN_0058e040(msg_kind) != 0`.
    pub pred_e040: bool,
    /// `FUN_00590f50() != 0`.
    pub pred_90f50: bool,
    /// `+0xa3`.
    pub state_a3: u8,
    /// The message's two coordinate words (`EBP-4` / `EBP+8`).
    pub x: u16,
    /// The message's second coordinate word.
    pub y: u16,
}

/// The non-posting outcome of `0x0063fb70`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A86Dispatch {
    /// The row posted something.
    Posted(A86DispatchResult),
    /// The row fell through the switch and returned `0`.
    Ignored,
    /// The row reached the `0x47` fatal continuation (`FUN_0058b640`).
    Fatal,
    /// The `0xac` walk found an entry that fails its continue test, which
    /// reaches `FUN_0058b650`.
    NotFound,
}

/// `0x0063fb70`: the `0xac` continue test.
///
/// ```c
/// while (a == 0 || b != 0 || c != 0 || d > 0x28) { next; if (next == 0) return 0; }
/// FUN_0058b650();
/// ```
pub fn a86_ac_continues(entry: &A86AcEntry) -> bool {
    entry.pred_a_clear || entry.pred_b_set || entry.pred_c_set || entry.value > 0x28
}

/// `0x0063fb70`: the `0xac` walk.
///
/// Returns `true` when the walk runs off the list (the row returns `0`) and
/// `false` when it stops on an entry (`FUN_0058b650`).
pub fn a86_ac_scan(entries: &[A86AcEntry]) -> bool {
    entries.iter().all(a86_ac_continues)
}

/// `0x0063fb70`: the message dispatch.
///
/// The `case` bodies, with the `0x0b -> 0x45 -> 0x2a` cascade reproduced as
/// the fallthroughs the listing shows:
///
/// * `0x02`: `can_3 && pred_510c0` -> message `0x84` with the two words.
/// * `0x05`: target kind `1`, `!pred_59030`, `can_5`, `+0x74 == 0` -> order
///   `0x62`.
/// * `0x0b`: `cmp_a != cmp_b && c7850 != 0`, then either the typed order
///   `0x31` carrying the `FUN_005c7850` result or, when `pred_d5d10`, the plain
///   order `0x6f`.
/// * `0x45`: `pred_560f0` -> return `1` without posting; else fall into
///   `0x2a`.
/// * `0x2a`: `pred_d5d10` -> order `0x6f`.
/// * `0x47`: fatal.
/// * `0x48`/`0x53`: `can_msg && pred_e040 && !pred_90f50` -> post `0x3f`
///   and return `0`.
/// * `0x7c`/`0x90`/`0x92`/`0xa2`: order `0x12`.
/// * `0xac`: only when `+0xa3 > 0x31`; walks the list.
pub fn a86_command_dispatch(input: &A86DispatchInput, ac_entries: &[A86AcEntry]) -> A86Dispatch {
    match input.ax {
        0x02 => {
            if input.can_3 && input.pred_510c0 {
                A86Dispatch::Posted(A86DispatchResult {
                    ret: 1,
                    action: A86Action::Message {
                        message: 0x84,
                        x: input.x,
                        y: input.y,
                    },
                })
            } else {
                A86Dispatch::Ignored
            }
        }
        0x05 => {
            if input.target_kind_1 && !input.pred_59030 && input.can_5 && input.field_74 == 0 {
                A86Dispatch::Posted(A86DispatchResult {
                    ret: 1,
                    action: A86Action::Order(0x62),
                })
            } else {
                A86Dispatch::Ignored
            }
        }
        0x0b => {
            if input.cmp_a != input.cmp_b && input.c7850 != 0 {
                if input.pred_d5d10 {
                    A86Dispatch::Posted(A86DispatchResult {
                        ret: 1,
                        action: A86Action::Order(0x6f),
                    })
                } else {
                    // `FUN_005b7dd0(0x31, iVar3)` reuses the `FUN_005c7850`
                    // result, which the decompiler assigned back into the same
                    // local as the `FUN_00588d60` comparison.
                    A86Dispatch::Posted(A86DispatchResult {
                        ret: 1,
                        action: A86Action::OrderTyped {
                            order: 0x31,
                            operand: input.c7850,
                        },
                    })
                }
            } else {
                a86_command_dispatch_group_45(input)
            }
        }
        0x45 => a86_command_dispatch_group_45(input),
        0x2a => {
            if input.pred_d5d10 {
                A86Dispatch::Posted(A86DispatchResult {
                    ret: 1,
                    action: A86Action::Order(0x6f),
                })
            } else {
                A86Dispatch::Ignored
            }
        }
        0x47 => A86Dispatch::Fatal,
        0x48 | 0x53 => {
            if input.can_msg && input.pred_e040 && !input.pred_90f50 {
                A86Dispatch::Posted(A86DispatchResult {
                    ret: 0,
                    action: A86Action::Post(0x3f),
                })
            } else {
                A86Dispatch::Ignored
            }
        }
        0x7c | 0x90 | 0x92 | 0xa2 => A86Dispatch::Posted(A86DispatchResult {
            ret: 1,
            action: A86Action::Order(0x12),
        }),
        0xac => {
            if input.state_a3 > 0x31 {
                if a86_ac_scan(ac_entries) {
                    // The walk ran off the list and the row returns 0 without
                    // posting anything.
                    A86Dispatch::Ignored
                } else {
                    A86Dispatch::NotFound
                }
            } else {
                A86Dispatch::Ignored
            }
        }
        _ => A86Dispatch::Ignored,
    }
}

/// The `0x45`/`0x2a` half of `0x0063fb70`, shared by the `0x0b` fallthrough.
fn a86_command_dispatch_group_45(input: &A86DispatchInput) -> A86Dispatch {
    if input.pred_560f0 {
        A86Dispatch::Posted(A86DispatchResult {
            ret: 1,
            action: A86Action::Nothing,
        })
    } else if input.pred_d5d10 {
        A86Dispatch::Posted(A86DispatchResult {
            ret: 1,
            action: A86Action::Order(0x6f),
        })
    } else {
        A86Dispatch::Ignored
    }
}

// ---------------------------------------------------------------------------
// 0x0063ff60 - ability gate
// ---------------------------------------------------------------------------

/// `0x0063ff60`: the per-player table `FUN_005b2510` indexes (`0x00f8abd0`).
pub const A86_VOICE_LIMIT: u8 = 0x2c;

/// `0x0063ff60`: `FUN_005cc8e0`'s "ignore energy" switch
/// (`DAT_01165d00 & 0x800`).
pub const A86_ENERGY_BYPASS: u16 = 0x0800;

/// `0x0063ff60`: `FUN_005b2510`'s table walk (`&DAT_00f8abd0` ->
/// `&DAT_00f8dc10`, `u16 << 8`).
///
/// ```c
/// if ((byte)(&DAT_00f8abd0)[param_1] < 0x2c)
///     return *(short *)(&DAT_00f8dc10 + table[param_1] * 2) << 8;
/// return 0;
/// ```
pub fn a86_voice_word(index_table: &[u8], words: &[i16], player: u8) -> i16 {
    let index = *index_table.get(player as usize).unwrap_or(&0xff);
    if index < A86_VOICE_LIMIT {
        *words.get(index as usize).unwrap_or(&0) << 8
    } else {
        0
    }
}

/// `0x0063ff60`: the gate.
///
/// ```c
/// if (FUN_005cc8e0(FUN_005b2510(param_2))) return false;      // not enough
/// return FUN_00534aa0(param_3, *(byte *)(unit + 0x4c), unit) == 1;
/// ```
///
/// `FUN_005cc8e0` reads `if (DAT_01165d00 & 0x800) return false;` first, so the
/// energy comparison only runs when that bypass is clear; it then returns
/// `energy < cost`.
pub fn a86_ability_gate(energy_q8: u16, cost_q8: u16, bypass: bool, can_execute: bool) -> bool {
    let blocked = if bypass { false } else { energy_q8 < cost_q8 };
    !blocked && can_execute
}

// ---------------------------------------------------------------------------
// 0x0063ffa0 - one-shot mark
// ---------------------------------------------------------------------------

/// `0x0063ffa0`: the mark state the row reads and updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A86Mark {
    /// `+0x4d`, the byte compared with the row's `param_2`.
    pub type_byte: u8,
    /// `+0x92`, the accumulated mask.
    pub bits: u8,
}

/// The outcome of one `0x0063ffa0` call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A86MarkOutcome {
    /// The row's return value.
    pub ret: bool,
    /// `true` when `FUN_005b7dd0(value, unit)` is posted.
    pub posted: bool,
}

/// `0x0063ffa0`: the one-shot mark.
///
/// ```c
/// if (unit != 0 && (*(byte *)(unit + 0x92) & mask) == 0) {
///     if (*(byte *)(unit + 0x4d) != value) {
///         FUN_005b7dd0(value, unit);
///         *(byte *)(unit + 0x92) |= mask;
///     }
///     return 1;
/// }
/// return 0;
/// ```
///
/// The bit is only accumulated inside the value-changed branch, which is the
/// non-obvious part: a call whose value already matches returns `1` without
/// touching `+0x92`.  `FUN_005b7dd0` posts the order but writes neither `+0x4d`
/// nor `+0x92`; the `or` in the row is the only store.
pub fn a86_mark_once(unit: Option<&mut A86Mark>, value: u8, mask: u8) -> A86MarkOutcome {
    let Some(unit) = unit else {
        return A86MarkOutcome {
            ret: false,
            posted: false,
        };
    };
    if unit.bits & mask != 0 {
        return A86MarkOutcome {
            ret: false,
            posted: false,
        };
    }
    let posted = unit.type_byte != value;
    if posted {
        unit.bits |= mask;
    }
    A86MarkOutcome { ret: true, posted }
}

// ---------------------------------------------------------------------------
// 0x0063fff0 - owned-slot release guard
// ---------------------------------------------------------------------------

/// `0x0063fff0`: the second argument of the release call (`0x1454`).
pub const A86_RELEASE_CODE: u32 = 0x1454;

/// `0x0063fff0`: the release decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A86Release {
    /// The slot is null; the row returns without calling anything.
    Empty,
    /// The slot is non-null; the row reaches `FUN_00923e90(slot, 0x1454)`.
    Leak(u32),
}

/// `0x0063fff0`: `if (*slot != 0) FUN_00923e90(*slot, 0x1454);`.
///
/// `FUN_00923e90` forwards to the noreturn `FUN_00924220` with the diagnostic
/// descriptor at `0x00d32a20`, so the row's only decision is null versus
/// non-null.
pub fn a86_release_slot(slot: u32) -> A86Release {
    if slot == 0 {
        A86Release::Empty
    } else {
        A86Release::Leak(slot)
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// One row is not translated:
//
// * `FUN_0063ff05` (`FUN_0063ff05`, 79 bytes): the whole body is 34
//   `*in_ECX = *in_ECX | (uint)in_ECX;` stores, four
//   `*unaff_EDI = *unaff_EDI | (uint)in_EAX;` stores, one
//   `*in_EAX = *in_EAX | (uint)in_ECX;` store and `swi(3)`.  Every store ORs a
//   register into the word it is writing, the source and destination alias,
//   and Ghidra reports truncated control flow at the end, so the row is an
//   overlapping-instruction artefact with no arithmetic, table or string
//   behaviour to reproduce.  It stays `decompiled_uncited`.
//
// Four rows in the window share one 112-case dispatch listing
// (`0x0063eb0a`, `0x0063ed3c`, `0x0063efa0`, `0x0063f269`).  Only the
// dispatch facts that are stable across all four copies are translated above;
// the per-opcode call targets are ordinary `CUnit` helpers outside the window
// and are not claimed.

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent86 {
    use super::*;

    #[test]
    fn cookie_indices_split_the_probe_word() {
        let probe = A86_GUARD_PROBE;
        let (low, high) = a86_cookie_indices(probe);
        assert_eq!(low, (probe & 0x0fff) as usize);
        assert_eq!(high, (probe >> 20) as usize);
        assert_eq!(low, 0x69a);
        assert_eq!(high, 0x0f8);
    }

    #[test]
    fn cookie_mix_folds_the_low_half_and_seed() {
        // low = 0x1234, high = 0xabcd, table entry 0x0102.
        let field = 0xabcd_1234;
        let table = 0x0102u16;
        let low = 0x1234u32;
        let mixed = (field >> 16) ^ (table as u32) ^ low ^ low.wrapping_sub(0x93af);
        let expected = ((mixed << 16) | low) ^ A86_GUARD_SEED;
        assert_eq!(a86_cookie_mix(field, table, A86_GUARD_SEED), expected);
        // The low half comes back xor-ed with the seed's own low half.
        assert_eq!(
            a86_cookie_mix(field, table, A86_GUARD_SEED) & 0xffff,
            low ^ (A86_GUARD_SEED & 0xffff)
        );
    }

    #[test]
    fn cookie_mix_uses_the_plain_low_word_twice() {
        // The low half is folded and then re-emitted unchanged.
        let a = a86_cookie_mix(0x0000_0001, 0, 0);
        assert_eq!(a & 0xffff, 1);
        // Changing only the seed changes only the high half.
        let b = a86_cookie_mix(0x0000_0001, 0, 0xffff_ffff);
        assert_eq!(a ^ b, 0xffff_ffff);
    }

    #[test]
    fn cookie_gate_is_clean_only_for_a_zero_low_half() {
        // With low == 0 the comparison reduces to `(field >> 16) == table`.
        let field = 0x1111_0000;
        let table = 0x1111u16;
        assert_eq!(a86_cookie_gate(field, table, 0), A86CookieGate::Clean);
        assert_eq!(a86_cookie_gate(0x2222_0000, table, 0), A86CookieGate::Dirty);
        // A non-zero low half always takes the branch.
        assert_eq!(a86_cookie_gate(field, 0x1111, 0), A86CookieGate::Clean);
        assert_eq!(
            a86_cookie_gate(0x1111_0001, 0x1111, 0),
            A86CookieGate::Dirty
        );
    }

    #[test]
    fn completed_gate_matches_the_bit_four_flag() {
        assert_eq!(a86_completed_flag(0x0000_0010), 1);
        assert_eq!(a86_completed_flag(0x0000_000f), 0);
        assert_eq!(a86_completed_flag(0xffff_ffff), 1);
        assert_eq!(a86_completed_gate(0x10), Ok(1));
        assert_eq!(a86_completed_gate(0x0), Err(A86_CONTINUATION));
    }

    #[test]
    fn entry_tag_compare_is_the_low_half_test() {
        // low = 0x6c, bias 0x112da, so the wrapped sum is 0x11346.
        let biased = A86_ENTRY_TAG.wrapping_add(A86_ENTRY_TAG_BIAS);
        assert_eq!(biased, 0x1_1346);
        let table = 0x0012u16;
        // Pick `high` so that the masked xor equals table ^ 0x6c.
        let wanted = (table as u32) ^ A86_ENTRY_TAG;
        let high = 0xdead_0000 | (wanted ^ (biased & 0xffff)) & 0xffff;
        assert!(!a86_entry_tag_compare(high, table));
        assert!(a86_entry_tag_compare(high ^ 1, table));
    }

    #[test]
    fn entry_tag_guard_always_fires() {
        // The trailing `|| 0x6c != 0` makes the shipped guard a tautology.
        assert!(a86_entry_tag_taken(0, 0));
        assert!(a86_entry_tag_taken(0xffff_ffff, 0xffff));
        assert_eq!(A86_ENTRY_TAG, 0x6c);
    }

    #[test]
    fn units_dat_records_use_the_52_byte_stride() {
        assert_eq!(a86_units_dat_record(0, 0x1000), 0x1000);
        assert_eq!(a86_units_dat_record(1, 0x1000), 0x1034);
        assert_eq!(a86_units_dat_record(3, 0), 0x9c);
        assert_eq!(A86_UNITS_DAT_STRIDE, 0x34);
        assert_eq!(A86_E565_PAYLOAD, 0x1c);
    }

    #[test]
    fn units_dat_top_up_stops_at_the_reloaded_limit() {
        // Four fixed steps of 3 from 10 against a limit of 20.
        let mut calls = 0;
        let value = a86_units_dat_top_up(10, 20, || {
            calls += 1;
            3
        });
        assert_eq!(value, 22);
        assert_eq!(calls, 4);
        // A zero limit never enters the loop.
        assert_eq!(a86_units_dat_top_up(99, 0, || 1), 99);
        // Already at the limit: no step.
        assert_eq!(a86_units_dat_top_up(20, 20, || panic!("no step")), 20);
    }

    #[test]
    fn retry_guard_skips_everything_for_ax_29() {
        assert!(!a86_retry_guard(0x29, 0x9c));
        assert!(a86_retry_guard(0x00, 0x9c));
        assert!(a86_retry_guard(0x2a, 0x02));
        assert!(!a86_retry_guard(0x2a, 0x03));
        assert_eq!(A86_RETRY_COMMAND, 0x0b);
        assert_eq!(A86_RETRY_ORDER, 0x74);
    }

    #[test]
    fn immediate_return_is_a_no_op() {
        a86_returns_immediately();
    }

    #[test]
    fn unit_kind_falls_back_to_the_type_byte() {
        // A clear bit 5 returns the type byte.
        assert_eq!(a86_unit_kind_or_player_flag(0x0000, 0x55), 0x55);
        assert_eq!(a86_unit_kind_or_player_flag(0x00df, 0x55), 0x55);
        // A set bit 5 returns 0x20, not the byte.
        assert_eq!(a86_unit_kind_or_player_flag(0x0020, 0x55), 0x20);
        assert_eq!(a86_unit_kind_or_player_flag(0xffff, 0x00), 0x20);
    }

    #[test]
    fn opcode_window_has_the_0x24_hole() {
        assert!(a86_opcode_valid(0x00));
        assert!(a86_opcode_valid(0x23));
        assert!(!a86_opcode_valid(0x24));
        assert!(a86_opcode_valid(0x25));
        assert!(a86_opcode_valid(0x70));
        assert!(!a86_opcode_valid(0x71));
        assert!(!a86_opcode_valid(0xff));
        assert_eq!(A86_OPCODE_CASES, 112);
        // 0x00..=0x23 is 36 labels and 0x25..=0x70 is 76.
        assert_eq!(0x24 + (0x70 - 0x25 + 1), A86_OPCODE_CASES);
    }

    #[test]
    fn alias_opcodes_share_the_0x3e_body() {
        assert_eq!(a86_opcode_group(0x3e), 0x3e);
        assert_eq!(a86_opcode_group(0x46), 0x3e);
        assert_eq!(a86_opcode_group(0x4c), 0x3e);
        assert_eq!(a86_opcode_group(0x0a), 0x0a);
        assert_eq!(a86_opcode_group(0x70), 0x70);
    }

    #[test]
    fn unit_type_bound_is_0xe4() {
        assert!(a86_unit_type_in_range(0));
        assert!(a86_unit_type_in_range(0xe3));
        assert!(!a86_unit_type_in_range(0xe4));
        assert!(!a86_unit_type_in_range(0xffff));
        assert!(a86_opcode_reads_unit_type(0x0a));
        assert!(a86_opcode_reads_unit_type(0x1a));
        assert!(a86_opcode_reads_unit_type(0x4c));
        assert!(!a86_opcode_reads_unit_type(0x00));
        assert!(!a86_opcode_reads_unit_type(0x24));
        for opcode in A86_UNIT_TYPE_BOUNDED {
            assert!(a86_opcode_valid(opcode), "opcode {opcode:#04x} must exist");
        }
    }

    #[test]
    fn decode_stride_prefers_the_0x500_flag() {
        assert_eq!(a86_decode_stride(0x0500, 0xfd57_f28b), 0x10);
        assert_eq!(a86_decode_stride(0x0100, 0x0000_0000), 0x10);
        assert_eq!(a86_decode_stride(0x0000, 0x0000_0000), 8);
        assert_eq!(a86_decode_stride(0x0000, 0xfd57_f28b), 1);
        assert_eq!(a86_decode_stride(0x00ff, 0x1234_5678), 8);
    }

    #[test]
    fn script_table_base_round_trips() {
        assert_eq!(
            a86_script_table_base(0x00f0_63cc),
            0x00f0_63cc ^ 0x10fa_e560
        );
        assert_eq!(
            a86_script_table_base(a86_script_table_base(0x1234_5678)),
            0x1234_5678
        );
    }

    #[test]
    fn stride_3c_bound_matches_a_60_byte_record() {
        assert!(a86_stride_3c_fits(0, 0));
        assert!(a86_stride_3c_fits(1, 0x3c));
        assert!(!a86_stride_3c_fits(1, 0x3b));
        assert!(a86_stride_3c_fits(0xff, 0xff * 0x3c));
        assert!(!a86_stride_3c_fits(0xff, 0xff * 0x3c - 1));
    }

    #[test]
    fn fnv1a_matches_the_published_vectors() {
        assert_eq!(a86_fnv1a32(b""), 0x811c_9dc5);
        assert_eq!(a86_fnv1a32(b"a"), 0xe40c_292c);
        assert_eq!(a86_fnv1a32(b"foobar"), 0xbf9c_f968);
        // The recurrence is byte-oriented, not word-oriented.
        assert_eq!(
            a86_fnv1a32(b"ab"),
            (b'b' as u32 ^ (b'a' as u32 ^ 0x811c_9dc5).wrapping_mul(0x0100_0193))
                .wrapping_mul(0x0100_0193)
        );
    }

    #[test]
    fn pool_count_and_tag_arithmetic() {
        let key = 0x1122_3344;
        assert_eq!(
            a86_pool_count(0x0000_00ff, key),
            0x00ff ^ (key + 0x4163_f040)
        );
        // A tag is valid exactly when it equals key ^ tag.
        let tag = key ^ A86_POOL_TAG;
        assert!(a86_pool_tag_matches(tag, key));
        assert!(!a86_pool_tag_matches(tag ^ 1, key));
        // With a zero key the bare tag constant is the valid value.
        assert!(a86_pool_tag_matches(A86_POOL_TAG, 0));
        assert!(!a86_pool_tag_matches(0, 0));
        assert_eq!(a86_pool_not_found(key), key.wrapping_add(0x4163_f040));
    }

    #[test]
    fn pool_ring_wraps_and_rejects_zero_count() {
        assert_eq!(a86_pool_ring_next(0, 4), Some(1));
        assert_eq!(a86_pool_ring_next(3, 4), Some(0));
        assert_eq!(a86_pool_ring_next(0, 1), Some(0));
        assert_eq!(a86_pool_ring_next(u32::MAX, 0), None);
        assert_eq!(a86_pool_pair_index(3, 5), 15);
        assert_eq!(a86_pool_pair_index(u32::MAX, 2), 0xffff_fffe);
    }

    #[test]
    fn udivmod64_is_unsigned_and_sign_extends_the_divisor() {
        assert_eq!(a86_udivmod64(100, 7), Some((14, 2)));
        assert_eq!(a86_udivmod64(u64::MAX, 2), Some((u64::MAX / 2, 1)));
        assert_eq!(a86_udivmod64(1, 0), None);
        // 0x83b ^ 0x82b folds to 0x10.
        assert_eq!(A86_F7B0_SECOND, 0x10);
        assert_eq!(a86_f7b0_divisor(0x083b), 0x10);
        assert_eq!(a86_f7b0_divisor(0), 0x082b);
        // A word whose masked form has bit 31 set becomes a huge unsigned
        // divisor, which is the whole point of the sign extension.
        assert_eq!(a86_f7b0_divisor(0xffff_ffff), 0xffff_ffff_ffff_f7d4);
    }

    #[test]
    fn orphan_sweep_steps_by_four_and_reports_the_bias() {
        assert_eq!(A86_ORPHAN_BIAS, 0x1000);
        assert_eq!(A86_ORPHAN_FAIL, 0xff);
        let slots = [0u32; A86_ORPHAN_SLOTS];
        // len == 0 takes the early out.
        assert_eq!(a86_orphan_sweep(0x1000, 0, &slots), Ok(0));
        // 12 bytes of range is three probes at +0, +4, +8.
        assert_eq!(a86_orphan_sweep(0x1000, 12, &slots), Ok(3));
        // The first probe collides.
        let hit = [0x2000u32, 0, 0, 0, 0, 0];
        assert_eq!(a86_orphan_sweep(0x2000, 12, &hit), Err(0xff));
        // A collision on the third probe still reports the same code.
        let mut late = [0u32; A86_ORPHAN_SLOTS];
        late[5] = 0x2008;
        assert_eq!(a86_orphan_sweep(0x2000, 12, &late), Err(0xff));
        // len is compared as an unsigned word, so a "negative" length runs.
        assert_eq!(a86_orphan_len(0x1000, 0x2000), -0x2000);
        assert_eq!(a86_orphan_len(0x0000_0010, 0), 0x10 - 0x1000);
    }

    #[test]
    fn orphan_sweep_from_uses_the_plus_one_probe() {
        let slots = [0u32; A86_ORPHAN_SLOTS];
        // start = 0, len = 3: probes are first+1 (index 0), then 0 + 4 + 1 = 5
        // which is >= len so the walk stops after one probe.
        assert_eq!(a86_orphan_sweep_from(0, 3, 0x8000, &slots), Ok(1));
        // start = 0, len = 10: probes at +1, +5 and +9; the next counter is
        // 12, so `len <= next` ends the walk after three probes.
        assert_eq!(a86_orphan_sweep_from(0, 10, 0x8000, &slots), Ok(3));
        // The colliding probe is the third one, first + 9.
        let mut hit = [0u32; A86_ORPHAN_SLOTS];
        hit[2] = 0x8009;
        assert_eq!(a86_orphan_sweep_from(0, 10, 0x8000, &hit), Err(0xff));
        // start past the length: no probes at all.
        assert_eq!(a86_orphan_sweep_from(3, 3, 0x8000, &slots), Ok(0));
    }

    #[test]
    fn target_list_walk_reports_the_first_missing_target() {
        let ok = A86ListUnit {
            gone: false,
            hidden: false,
            matched: true,
            has_target: true,
        };
        assert_eq!(a86_validate_target_list(&[]), A86ListVerdict::Exhausted);
        assert_eq!(
            a86_validate_target_list(&[ok, ok]),
            A86ListVerdict::Exhausted
        );
        // A matching record without a target stops the walk.
        let missing = A86ListUnit {
            has_target: false,
            ..ok
        };
        assert_eq!(
            a86_validate_target_list(&[ok, missing, ok]),
            A86ListVerdict::TargetMissing(1)
        );
        // Skipped records never produce the failure.
        let skipped = A86ListUnit {
            hidden: true,
            has_target: false,
            ..ok
        };
        assert_eq!(
            a86_validate_target_list(&[skipped]),
            A86ListVerdict::Exhausted
        );
        let gone = A86ListUnit {
            gone: true,
            ..skipped
        };
        assert_eq!(a86_validate_target_list(&[gone]), A86ListVerdict::Exhausted);
    }

    #[test]
    fn tile_world_drops_the_subpixel_fraction() {
        let tile = A86Tile {
            x_word: 0x1234_5600,
            y_word: -0x0000_0100,
        };
        assert_eq!(a86_tile_world(&tile), (0x1234_56, -1));
        assert_eq!(a86_tile_world(&A86Tile::default()), (0, 0));
    }

    #[test]
    fn chunk_and_subtile_indices() {
        assert_eq!(a86_chunk_index(0, 0), 0);
        assert_eq!(a86_chunk_index(31, 0), 0);
        assert_eq!(a86_chunk_index(32, 0), 1);
        assert_eq!(a86_chunk_index(0, 32), 0x100);
        // The `>> 31 & 0x1f` bias makes -1 land in chunk 0, not chunk -1.
        assert_eq!(a86_chunk_index(-1, 0), 0);
        // -32 is the first negative coordinate that floors to -1.
        assert_eq!(a86_chunk_index(-32, 0), 0xffff_ffff);
        assert_eq!(a86_chunk_index(-32, -32), 0xffff_feff);
        assert_eq!(a86_subtile_bit(0, 0), 0);
        assert_eq!(a86_subtile_bit(8, 0), 1);
        assert_eq!(a86_subtile_bit(0, 8), 4);
        assert_eq!(a86_subtile_bit(24, 24), 15);
        assert_eq!(a86_subtile_bit(32, 32), 0);
    }

    #[test]
    fn region_lookup_uses_the_split_bitmap() {
        let mut chunks = vec![0u16; 0x100];
        chunks[0] = 7;
        chunks[1] = 0x2000;
        // The split table is indexed by the raw chunk word, so entry 0x2000.
        let mut splits = vec![A86RegionSplit::default(); 0x2001];
        splits[0x2000] = A86RegionSplit {
            mask: 1 << 3,
            outside: 0x0011,
            inside: 0x0022,
        };
        // Plain chunk id.
        assert_eq!(a86_region_at(&chunks, &splits, 0, 0), Some(7));
        // Split with sub-tile bit 0 clear.
        assert_eq!(a86_region_at(&chunks, &splits, 32, 0), Some(0x0011));
        // x = 56 puts the point in sub-tile column 3, so bit 3 is set.
        assert_eq!(a86_region_at(&chunks, &splits, 56, 0), Some(0x0022));
        // y = 8 moves the bit to the second row, which the mask omits.
        assert_eq!(a86_region_at(&chunks, &splits, 32, 8), Some(0x0011));
        // Out-of-range chunk reports None rather than reading past the table.
        assert_eq!(a86_region_at(&chunks, &splits, 32 * 0x100, 0), None);
        assert_eq!(A86_REGION_SPLIT_MIN, 0x2000);
    }

    #[test]
    fn region_unit_query_returns_the_row_sentinels() {
        let live = A86CandidateUnit {
            gone: false,
            active: true,
            matched: true,
        };
        assert_eq!(a86_region_unit_query(&[]), None);
        assert_eq!(a86_region_unit_return(&[]), A86_NO_UNIT);
        assert_eq!(a86_point_unit_return(&[]), A86_NO_POINT_UNIT);
        assert_eq!(A86_NO_UNIT, 0xffff_ffff);
        assert_eq!(A86_NO_POINT_UNIT, 0);
        assert_eq!(A86_BEST_DISTANCE_INIT, 99_999_999);
        assert_eq!(A86_BEST_UNIT_INIT, 0);
        assert_eq!(a86_region_unit_query(&[live]), Some(0));
        // An inactive record is skipped even when it matches.
        let inactive = A86CandidateUnit {
            active: false,
            ..live
        };
        assert_eq!(a86_region_unit_query(&[inactive, live]), Some(1));
        // A record that is gone is skipped as well.
        let gone = A86CandidateUnit { gone: true, ..live };
        assert_eq!(a86_region_unit_return(&[gone, inactive]), A86_NO_UNIT);
    }

    #[test]
    fn area_mark_skips_the_owning_player() {
        assert_eq!(a86_area_mark_slot(3, 3), None);
        // (player + 0x131b) * 0xc + owner
        assert_eq!(a86_area_mark_slot(0, 1), Some(0x131b * 0x0c + 1));
        assert_eq!(a86_area_mark_slot(1, 0), Some(0x131c * 0x0c));
        assert_eq!(A86_AREA_MARK_VALUE, 2);
    }

    #[test]
    fn lift_transition_opens_only_outside_the_type_window() {
        let base = A86LiftFacts {
            state: A86_LIFT_STATE_UP,
            flags: A86_LIFT_FLAG,
            pred_a: false,
            pred_b: false,
            pred_c: false,
            unit_type: 0x00,
        };
        assert_eq!(a86_lift_transition(&base), Some(0x6e));
        // State must be 'm'.
        assert_eq!(
            a86_lift_transition(&A86LiftFacts {
                state: 0x00,
                ..base
            }),
            None
        );
        // The 0x200 flag is required.
        assert_eq!(
            a86_lift_transition(&A86LiftFacts {
                flags: 0x100,
                ..base
            }),
            None
        );
        for pred in 0..3 {
            let mut facts = base;
            match pred {
                0 => facts.pred_a = true,
                1 => facts.pred_b = true,
                _ => facts.pred_c = true,
            }
            assert_eq!(a86_lift_transition(&facts), None);
        }
        // The excluded type window is inclusive at both ends.
        assert_eq!(
            a86_lift_transition(&A86LiftFacts {
                unit_type: 0x7b,
                ..base
            }),
            None
        );
        assert_eq!(
            a86_lift_transition(&A86LiftFacts {
                unit_type: 0x81,
                ..base
            }),
            None
        );
        assert_eq!(
            a86_lift_transition(&A86LiftFacts {
                unit_type: 0x7a,
                ..base
            }),
            Some(0x6e)
        );
        assert_eq!(
            a86_lift_transition(&A86LiftFacts {
                unit_type: 0x82,
                ..base
            }),
            Some(0x6e)
        );
        assert_eq!(A86_LIFT_CLEARED_FIELDS, [0xe8, 0xec, 0xe2]);
    }

    #[test]
    fn dispatch_handles_the_default_and_fatal_cases() {
        let input = A86DispatchInput {
            ax: 0x00,
            ..A86DispatchInput::default()
        };
        assert_eq!(a86_command_dispatch(&input, &[]), A86Dispatch::Ignored);
        let fatal = A86DispatchInput {
            ax: 0x47,
            ..A86DispatchInput::default()
        };
        assert_eq!(a86_command_dispatch(&fatal, &[]), A86Dispatch::Fatal);
        // The four order-0x12 messages.
        for ax in [0x7c, 0x90, 0x92, 0xa2] {
            let input = A86DispatchInput {
                ax,
                ..A86DispatchInput::default()
            };
            assert_eq!(
                a86_command_dispatch(&input, &[]),
                A86Dispatch::Posted(A86DispatchResult {
                    ret: 1,
                    action: A86Action::Order(0x12),
                })
            );
        }
    }

    #[test]
    fn dispatch_case_02_and_05_gates() {
        let mut input = A86DispatchInput {
            ax: 0x02,
            x: 0x10,
            y: 0x20,
            can_3: true,
            pred_510c0: true,
            ..A86DispatchInput::default()
        };
        assert_eq!(
            a86_command_dispatch(&input, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 1,
                action: A86Action::Message {
                    message: 0x84,
                    x: 0x10,
                    y: 0x20,
                },
            })
        );
        input.can_3 = false;
        assert_eq!(a86_command_dispatch(&input, &[]), A86Dispatch::Ignored);

        let mut five = A86DispatchInput {
            ax: 0x05,
            target_kind_1: true,
            can_5: true,
            ..A86DispatchInput::default()
        };
        assert_eq!(
            a86_command_dispatch(&five, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 1,
                action: A86Action::Order(0x62),
            })
        );
        // A non-zero +0x74 or a positive FUN_00659030 both block the order.
        five.field_74 = 1;
        assert_eq!(a86_command_dispatch(&five, &[]), A86Dispatch::Ignored);
        five.field_74 = 0;
        five.pred_59030 = true;
        assert_eq!(a86_command_dispatch(&five, &[]), A86Dispatch::Ignored);
    }

    #[test]
    fn dispatch_case_0b_falls_through_to_the_45_group() {
        let mut input = A86DispatchInput {
            ax: 0x0b,
            cmp_a: 7,
            cmp_b: 9,
            c7850: 0x5a,
            ..A86DispatchInput::default()
        };
        // Without FUN_005d5d10 the typed order is posted.
        assert_eq!(
            a86_command_dispatch(&input, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 1,
                action: A86Action::OrderTyped {
                    order: 0x31,
                    operand: 0x5a,
                },
            })
        );
        // With it, the plain order wins.
        input.pred_d5d10 = true;
        assert_eq!(
            a86_command_dispatch(&input, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 1,
                action: A86Action::Order(0x6f),
            })
        );
        // Equal ids skip the case body and fall into 0x45.
        input.cmp_a = 9;
        input.pred_560f0 = true;
        assert_eq!(
            a86_command_dispatch(&input, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 1,
                action: A86Action::Nothing,
            })
        );
        input.pred_560f0 = false;
        assert_eq!(
            a86_command_dispatch(&input, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 1,
                action: A86Action::Order(0x6f),
            })
        );
        input.pred_d5d10 = false;
        assert_eq!(a86_command_dispatch(&input, &[]), A86Dispatch::Ignored);
    }

    #[test]
    fn dispatch_case_48_posts_and_returns_zero() {
        let input = A86DispatchInput {
            ax: 0x48,
            msg_kind: 0x49,
            can_msg: true,
            pred_e040: true,
            ..A86DispatchInput::default()
        };
        assert_eq!(
            a86_command_dispatch(&input, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 0,
                action: A86Action::Post(0x3f),
            })
        );
        let blocked = A86DispatchInput {
            pred_90f50: true,
            ..input
        };
        assert_eq!(a86_command_dispatch(&blocked, &[]), A86Dispatch::Ignored);
        assert_eq!(
            a86_command_dispatch(&A86DispatchInput { ax: 0x53, ..input }, &[]),
            A86Dispatch::Posted(A86DispatchResult {
                ret: 0,
                action: A86Action::Post(0x3f)
            })
        );
    }

    #[test]
    fn dispatch_case_ac_walks_only_for_big_state_bytes() {
        let low = A86DispatchInput {
            ax: 0xac,
            state_a3: 0x31,
            ..A86DispatchInput::default()
        };
        assert_eq!(a86_command_dispatch(&low, &[]), A86Dispatch::Ignored);
        let high = A86DispatchInput {
            state_a3: 0x32,
            ..low
        };
        // An empty list runs off the end and the row returns 0 unposted.
        assert_eq!(a86_command_dispatch(&high, &[]), A86Dispatch::Ignored);
        // An entry that fails the continue test stops the walk.
        let stop = [A86AcEntry::default()];
        assert_eq!(a86_command_dispatch(&high, &stop), A86Dispatch::NotFound);
        // Any of the four continue terms keeps the walk going.
        for entry in [
            A86AcEntry {
                pred_a_clear: true,
                ..A86AcEntry::default()
            },
            A86AcEntry {
                pred_b_set: true,
                ..A86AcEntry::default()
            },
            A86AcEntry {
                pred_c_set: true,
                ..A86AcEntry::default()
            },
            A86AcEntry {
                value: 0x29,
                ..A86AcEntry::default()
            },
        ] {
            assert!(a86_ac_continues(&entry));
            assert_eq!(a86_command_dispatch(&high, &[entry]), A86Dispatch::Ignored);
        }
        // `0x28` is the last value inside the range: `0x28 < d` is false.
        assert!(!a86_ac_continues(&A86AcEntry {
            value: 0x28,
            ..A86AcEntry::default()
        }));
    }

    #[test]
    fn voice_word_uses_the_index_table_and_shift() {
        let index = [3u8, 0x2c, 0x2b, 0];
        let words: Vec<i16> = (0..0x2c).map(|i| i as i16).collect();
        assert_eq!(a86_voice_word(&index, &words, 0), 3 << 8);
        // 0x2c is the first index the row rejects, so the result is zero.
        assert_eq!(a86_voice_word(&index, &words, 1), 0);
        // 0x2b is the last accepted index.
        assert_eq!(a86_voice_word(&index, &words, 2), 0x2b00i16);
        assert_eq!(a86_voice_word(&index, &words, 3), 0);
        // A missing player also yields zero.
        assert_eq!(a86_voice_word(&index, &words, 9), 0);
    }

    #[test]
    fn ability_gate_combines_energy_and_the_command_check() {
        // Enough energy, command allowed.
        assert!(a86_ability_gate(100, 50, false, true));
        // Not enough energy.
        assert!(!a86_ability_gate(49, 50, false, true));
        // The 0x800 bypass skips the energy test.
        assert!(a86_ability_gate(0, 50, true, true));
        // The command check still applies.
        assert!(!a86_ability_gate(100, 50, false, false));
        assert_eq!(A86_ENERGY_BYPASS, 0x0800);
    }

    #[test]
    fn mark_once_tracks_the_bit_and_the_value() {
        let mut unit = A86Mark {
            type_byte: 0x10,
            bits: 0,
        };
        // First call changes the value and accumulates the bit.
        let first = a86_mark_once(Some(&mut unit), 0x20, 0x04);
        assert!(first.ret);
        assert!(first.posted);
        // `FUN_005b7dd0` posts the order; it does not write `+0x4d`.
        assert_eq!(unit.type_byte, 0x10);
        assert_eq!(unit.bits, 0x04);
        // Second call is a no-op because the bit is already set.
        let second = a86_mark_once(Some(&mut unit), 0x30, 0x04);
        assert!(!second.ret);
        assert!(!second.posted);
        assert_eq!(unit.bits, 0x04);
        // Same value with a fresh mask returns 1 without posting.
        let third = a86_mark_once(Some(&mut unit), 0x10, 0x08);
        assert!(third.ret);
        assert!(!third.posted);
        assert_eq!(unit.bits, 0x04);
        // A different value with a fresh mask posts and sets the bit.
        let fourth = a86_mark_once(Some(&mut unit), 0x40, 0x08);
        assert!(fourth.ret);
        assert!(fourth.posted);
        assert_eq!(unit.bits, 0x0c);
        // A null unit returns 0.
        let none = a86_mark_once(None, 0x20, 0x08);
        assert!(!none.ret);
        assert!(!none.posted);
    }

    #[test]
    fn release_slot_distinguishes_null() {
        assert_eq!(a86_release_slot(0), A86Release::Empty);
        assert_eq!(a86_release_slot(0x1234), A86Release::Leak(0x1234));
        assert_eq!(A86_RELEASE_CODE, 0x1454);
    }
}

// ===========================================================================
// Merged from src/translated/agent87.rs
// ===========================================================================
// Translation batch 87.  Only this file is edited by the batch.
//
// Batch 87 owns `0x00640000..=0x00641fff`.  The ledger lists 24 rows in that
// window: 23 `decompiled_uncited` and one (`FUN_00640b20`) already `translated`
// with `src/aiscript.rs` as its evidence (that module's header cites the
// interpreter at `0x7a1e0b20`).  20 of the 23 uncited rows are reconstructed
// below; the three rows that are only re-decompiled interior entries of
// `FUN_00640b20` are listed at the end of this file with their reason.
//
// The window is the lifetime plumbing of the AI-script interpreter at
// `FUN_00640b20`: the static 100-record script pool, the two loaded
// `aiscript.bin` / `bwscript.bin` images, the active/free doubly linked
// lists, the per-tick delay sweep, the owner sweep, the script-id dispatch
// table, and the operand/cost queries the interpreter calls from its switch.
//
// Recovered globals (all absolute addresses; `pool` is `0x00fc8710`):
//
// | address      | role in the recovered bodies                                  |
// |--------------|---------------------------------------------------------------|
// | `0x00fc8700` | mapped `aiscript.bin` base                                     |
// | `0x00fc8704` | mapped `aiscript.bin` length (rounded up, see `0x00640620`)    |
// | `0x00fc8708` | mapped `bwscript.bin` base                                     |
// | `0x00fc870c` | mapped `bwscript.bin` length                                   |
// | `0x00fc8710` | first of 100 pool records, `0x34` bytes each (`0x1450` bytes)  |
// | `0x00fc86dc` | index origin: `pool - 0x34`, i.e. saved index 1 -> record 0     |
// | `0x00fc9b60` | free-list head (pool `+0x1450`)                                |
// | `0x00fc9b64` | active-list owner word (equal to the pool base)                |
// | `0x00fc9b68` | active-list head                                              |
// | `0x00f056b8` | 5-entry script-id word table `"EnBkEnTrExTrDWHePB3F"`          |
// | `0x00f4e578` | security cookie used by the `0x1454`-byte frame prologue       |
//
// Recovered record layout (13 dwords, byte offsets from the record base):
//
// | offset | role                                                        |
// |--------|-------------------------------------------------------------|
// | `+0x00`| next link (absolute address, 0 = end)                       |
// | `+0x04`| previous link (absolute address, 0 = end)                   |
// | `+0x08`| interpreter read cursor                                     |
// | `+0x0c`| delay counter decremented by `0x006408b0`                   |
// | `+0x10`| player word handed to `FUN_00640b20`                          |
// | `+0x14`| script operand block (two dwords)                           |
// | `+0x1c`| interpreter scratch                                         |
// | `+0x20`| interpreter scratch                                         |
// | `+0x24`| interpreter scratch                                         |
// | `+0x28`| interpreter scratch                                         |
// | `+0x2c`| owner word compared by `0x00640830`                         |
// | `+0x30`| flags; bit 2 (`0x4`) marks a record that is unlinked by     |
// |         | `0x006408b0` without running it                             |
//
// Two conventions used by the rows below:
//
// * a *saved* pool stores the two link words as **1-based indices**: a zero
//   word means "end of list", and `n` means the record at
//   `0x00fc86dc + n * 0x34` (so `n = 1` is the first record).  `0x006401a6`
//   turns indices into addresses and `0x006403f0` turns them back;
// * helper results that the shipped code receives from outside this window
//   (allocator handles, script-table file loads, the interpreter's
//   `FUN_00695be0`/`FUN_00695c10` cost tables) are explicit parameters or
//   model scaffolding here, never cited as implemented Rust functions.
//
// Address to function:
//
// * `0x00640010` [`a87_fill_index_table`]
// * `0x00640020` [`A87ScriptRuntime::a87_release_record`]
// * `0x00640080` [`a87_check_no_pending_object`]
// * `0x006400a7` [`a87_selector_matches`]
// * `0x006401a6` [`a87_restore_script_pool`]
// * `0x00640320` [`a87_push_script_exception_frame`]
// * `0x00640348` [`a87_script_frame_bytes`]
// * `0x00640390` [`A87ScriptRuntime::a87_clear_chain`]
// * `0x006403f0` [`a87_save_script_pool`]
// * `0x006405c0` [`a87_unmap_script_image`]
// * `0x00640620` [`a87_script_image_size`] / [`A87ScriptRuntime::a87_init`] /
//   [`A87MappedImage::a87_load`]
// * `0x006406b8` [`a87_rotate_and_resize`]
// * `0x00640830` [`A87ScriptRuntime::a87_kill_owned`]
// * `0x006408b0` [`A87ScriptRuntime::a87_tick`]
// * `0x00640940` [`a87_dispatch_script`] / [`a87_script_id_accepted`]
// * `0x00640999` [`a87_panel_slot`] / [`a87_panel_state`]
// * `0x006409d3` [`a87_special_script_matches`]
// * `0x00640a90` [`a87_script_cost`]
// * `0x00640be8` [`a87_interpreter_unwind`] (interior epilogue entry)
// * `0x00640e25` [`a87_operand_word_advance`] (interior operand-guard entry)

/// `0x00fc8710`: first record of the static script pool.
pub const A87_SCRIPT_POOL_BASE: u32 = 0x00fc_8710;

/// `0x00fc8710 + 0x1450`: the free-list head word.
pub const A87_SCRIPT_FREE_LIST: u32 = 0x00fc_9b60;

/// `0x00fc9b64`: the active-list owner word (holds the pool base).
pub const A87_SCRIPT_LIST_OWNER: u32 = 0x00fc_9b64;

/// `0x00fc9b68`: the active-list head word.
pub const A87_SCRIPT_ACTIVE_HEAD: u32 = 0x00fc_9b68;

/// `0x00fc8700`: mapped `aiscript.bin` base word.
pub const A87_AISCRIPT_HANDLE: u32 = 0x00fc_8700;

/// `0x00fc8704`: mapped `aiscript.bin` length word.
pub const A87_AISCRIPT_SIZE: u32 = 0x00fc_8704;

/// `0x00fc8708`: mapped `bwscript.bin` base word.
pub const A87_BWSCRIPT_HANDLE: u32 = 0x00fc_8708;

/// `0x00fc870c`: mapped `bwscript.bin` length word.
pub const A87_BWSCRIPT_SIZE: u32 = 0x00fc_870c;

/// Size of one pool record: 13 dwords.
pub const A87_SCRIPT_RECORD_DWORDS: usize = 13;

/// Byte size of one pool record (`0x34`), used by both index conversions.
pub const A87_SCRIPT_RECORD_BYTES: u32 = 0x34;

/// Number of records in the pool: `0x1450 / 0x34`.
pub const A87_SCRIPT_POOL_RECORDS: usize = 100;

/// Total pool byte size (`0x1450`), which is also the offset of the free-list
/// head inside the pool block.
pub const A87_SCRIPT_POOL_BYTES: u32 = 0x1450;

/// `pool - 0x34`: the origin of the 1-based saved-index formula
/// `0x00fc86dc + index * 0x34`.
pub const A87_SCRIPT_INDEX_ORIGIN: u32 = A87_SCRIPT_POOL_BASE - A87_SCRIPT_RECORD_BYTES;

/// `0x00640010` writes this many identity bytes (`0..=0x1b`).
pub const A87_INDEX_TABLE_BYTES: usize = 0x1c;

/// `0x00640620` adds this much headroom before aligning a script image.
pub const A87_SCRIPT_IMAGE_HEADROOM: u32 = 0x6000;

/// Alignment mask applied to the script image length (`and 0xfffffffc`).
pub const A87_SCRIPT_IMAGE_ALIGN_MASK: u32 = 0xffff_fffc;

/// Dword index of the `next` link inside a record (byte `+0x00`).
pub const A87_REC_NEXT: usize = 0;

/// Dword index of the `previous` link inside a record (byte `+0x04`).
pub const A87_REC_PREV: usize = 1;

/// Dword index of the interpreter read cursor (byte `+0x08`).
pub const A87_REC_CURSOR: usize = 2;

/// Dword index of the delay counter (byte `+0x0c`).
pub const A87_REC_DELAY: usize = 3;

/// Dword index of the player word (byte `+0x10`).
pub const A87_REC_PLAYER: usize = 4;

/// Dword index of the owner word compared by `0x00640830` (byte `+0x2c`).
pub const A87_REC_OWNER: usize = 11;

/// Dword index of the flag dword (byte `+0x30`); only the low byte is read.
pub const A87_REC_FLAGS: usize = 12;

/// Flag bit 2: `0x006408b0` unlinks such a record instead of running it.
pub const A87_REC_FLAG_UNLINK: u32 = 4;

/// The delay value the interpreter reloads on its loop/exit path.
pub const A87_SCRIPT_DELAY_RELOAD: u32 = 0x1e;

/// `0x75637352` = `"Rscu"`, the single id `0x006409d3` dispatches.
pub const A87_SPECIAL_SCRIPT_ID: u32 = u32::from_le_bytes(*b"Rscu");

/// Panel record stride used by `0x00640999` and `0x006409d3`.
pub const A87_PANEL_RECORD_BYTES: u32 = 0x24;

/// Panel slot bias: `(index + 0xc) * 0x24`.
pub const A87_PANEL_SLOT_BIAS: u32 = 0x0c;

/// Panel state byte tested by `0x00640999` is at `slot + 8`.
pub const A87_PANEL_STATE_OFFSET: u32 = 8;

/// Panel state value that `0x00640999` accepts.
pub const A87_PANEL_STATE_RUNNING: u8 = 1;

/// Panel kind that `0x006409d3` requires (a byte at `+8` equal to 7).
pub const A87_PANEL_KIND_SPECIAL: u8 = 7;

/// The 5-entry script-id table at `0x00f056b8`, recovered from the string
/// `"EnBkEnTrExTrDWHePB3F"` that the symbol name spells out.
pub const A87_SCRIPT_ID_TABLE: [u32; 5] = [
    u32::from_le_bytes(*b"EnBk"),
    u32::from_le_bytes(*b"EnTr"),
    u32::from_le_bytes(*b"ExTr"),
    u32::from_le_bytes(*b"DWHe"),
    u32::from_le_bytes(*b"PB3F"),
];

/// `0x00640a90` special-cases this cost-table entry (`0x1e`).
pub const A87_COST_TYPE_DELAY: u32 = 0x1e;

/// `0x00640a90` special-cases this cost-table entry (`5`) too.
pub const A87_COST_TYPE_BUILD: u32 = 5;

/// `0x00640320`/`0x00640348`: the interpreter frame size.
pub const A87_SCRIPT_FRAME_BYTES: u32 = 0x1454;

/// `0x00640320`: the exception handler installed for the frame
/// (`&DAT_00cc7258`).
pub const A87_SCRIPT_SEH_HANDLER: u32 = 0x00cc_7258;

/// `0x00f4e578`: the security-cookie word xor-ed with the frame pointer.
pub const A87_SECURITY_COOKIE_ADDRESS: u32 = 0x00f4_e578;

// ---------------------------------------------------------------------------
// The 100-record script pool
// ---------------------------------------------------------------------------

/// One 100-record script pool.  Link words hold absolute addresses, exactly
/// like the shipped initialiser at `0x00640620` (which writes
/// `0x00fc8744 + n * 0x34` into `next` and `0x00fc86dc + n * 0x34` into
/// `previous`); a zero word terminates the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A87ScriptPool {
    pub records: [[u32; A87_SCRIPT_RECORD_DWORDS]; A87_SCRIPT_POOL_RECORDS],
}

impl Default for A87ScriptPool {
    fn default() -> Self {
        Self {
            records: [[0; A87_SCRIPT_RECORD_DWORDS]; A87_SCRIPT_POOL_RECORDS],
        }
    }
}

/// Address of pool record `index` (`0x00fc8710 + index * 0x34`).
pub fn a87_record_address(index: usize) -> u32 {
    A87_SCRIPT_POOL_BASE.wrapping_add(index as u32 * A87_SCRIPT_RECORD_BYTES)
}

/// Pool index of an absolute record address, or `None` for a null/foreign
/// word.
pub fn a87_record_index(address: u32) -> Option<usize> {
    if address < A87_SCRIPT_POOL_BASE {
        return None;
    }
    let offset = address - A87_SCRIPT_POOL_BASE;
    if offset % A87_SCRIPT_RECORD_BYTES != 0 {
        return None;
    }
    let index = (offset / A87_SCRIPT_RECORD_BYTES) as usize;
    (index < A87_SCRIPT_POOL_RECORDS).then_some(index)
}

/// Saved-pool word for an absolute link address.
///
/// `0x006403f0` computes `(address - 0x00fc8710) / 0x34 + 1` on the unsigned
/// dwords and stores `0` for a null link, so a link to record 0 is saved as
/// `1` and a link to record 99 as `100`.
pub fn a87_saved_index(address: u32) -> u32 {
    if address == 0 {
        0
    } else {
        address
            .wrapping_sub(A87_SCRIPT_POOL_BASE)
            .wrapping_div(A87_SCRIPT_RECORD_BYTES)
            .wrapping_add(1)
    }
}

/// Absolute link address for a saved-pool word.
///
/// `0x006401a6` reloads `0x00fc86dc + word * 0x34` for every nonzero word,
/// i.e. the exact inverse of [`a87_saved_index`] for in-range indices.
pub fn a87_restored_address(saved_index: u32) -> u32 {
    if saved_index == 0 {
        0
    } else {
        A87_SCRIPT_INDEX_ORIGIN.wrapping_add(saved_index.wrapping_mul(A87_SCRIPT_RECORD_BYTES))
    }
}

/// A pool image as `0x006403f0` writes it: the 13-dword records with the two
/// link words replaced by 1-based indices, plus the free-list head index at
/// pool offset `0x1450`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A87SavedScriptPool {
    pub records: [[u32; A87_SCRIPT_RECORD_DWORDS]; A87_SCRIPT_POOL_RECORDS],
    pub free_head: u32,
}

impl Default for A87SavedScriptPool {
    fn default() -> Self {
        Self {
            records: [[0; A87_SCRIPT_RECORD_DWORDS]; A87_SCRIPT_POOL_RECORDS],
            free_head: 0,
        }
    }
}

/// The script pool plus the two list heads and the active-list tail.
///
/// The shipped code keeps the free-list head at `0x00fc9b60`, the active head
/// at `0x00fc9b68` and the active tail in the second dword of the list header
/// at `0x00fc9b64`.  All three words hold absolute addresses here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A87ScriptRuntime {
    pub pool: A87ScriptPool,
    pub free_head: u32,
    pub active_head: u32,
    pub active_tail: u32,
}

impl A87ScriptRuntime {
    /// `0x00640620`: build the shipped pool.
    ///
    /// The initialiser zeroes the header words, then walks `uVar2` from
    /// `0x34` while `uVar2 < 0x141c` in `0x34` steps writing
    /// `next = 0x00fc8744 + u`, `previous = 0x00fc86dc + u` and zeroing the
    /// remaining eleven dwords.  Record 0 is written separately
    /// (`next = 0x00fc8744`, `previous = 0`), and record 99 is written by the
    /// second header block at `0x00fc9b2c` (`next = 0`,
    /// `previous = 0x00fc9af8`).  The result is one 100-record chain with the
    /// free-list head at record 0 and no active records.
    pub fn a87_init() -> Self {
        let mut pool = A87ScriptPool::default();
        for index in 0..A87_SCRIPT_POOL_RECORDS {
            pool.records[index][A87_REC_NEXT] = if index + 1 < A87_SCRIPT_POOL_RECORDS {
                a87_record_address(index + 1)
            } else {
                0
            };
            pool.records[index][A87_REC_PREV] = if index == 0 {
                0
            } else {
                a87_record_address(index - 1)
            };
        }
        Self {
            pool,
            free_head: a87_record_address(0),
            active_head: 0,
            active_tail: 0,
        }
    }

    /// `FUN_00640b20` exit paths and `0x00640be1`: the delay is reloaded with
    /// `0x1e` and the following scratch word is cleared.
    pub fn a87_reload_delay(&mut self, index: usize) -> bool {
        if index >= A87_SCRIPT_POOL_RECORDS {
            return false;
        }
        self.pool.records[index][A87_REC_DELAY] = A87_SCRIPT_DELAY_RELOAD;
        self.pool.records[index][A87_REC_DELAY + 1] = 0;
        true
    }

    /// `0x00640020`: unlink a record from the active list and push it onto the
    /// free list.
    ///
    /// The shipped body updates the tail when the record is the tail, patches
    /// `prev->next` and `next->prev`, clears the record's `previous` word,
    /// links its `next` word to the old free-list head and stores the record
    /// as the new head.  The old head's `previous` word is patched back to the
    /// record, so the free list keeps both link words.
    pub fn a87_release_record(&mut self, index: usize) -> bool {
        if index >= A87_SCRIPT_POOL_RECORDS {
            return false;
        }
        let address = a87_record_address(index);
        let next = self.pool.records[index][A87_REC_NEXT];
        let prev = self.pool.records[index][A87_REC_PREV];

        if self.active_tail == address {
            self.active_tail = next;
        }
        if self.active_head == address {
            self.active_head = next;
        }
        if let Some(prev_index) = a87_record_index(prev) {
            self.pool.records[prev_index][A87_REC_NEXT] = next;
        }
        if let Some(next_index) = a87_record_index(next) {
            self.pool.records[next_index][A87_REC_PREV] = prev;
        }

        let old_free_head = self.free_head;
        self.pool.records[index][A87_REC_PREV] = 0;
        self.pool.records[index][A87_REC_NEXT] = old_free_head;
        if let Some(head_index) = a87_record_index(old_free_head) {
            self.pool.records[head_index][A87_REC_PREV] = address;
        }
        self.free_head = address;
        true
    }

    /// Model scaffolding for the insert side of the list.  No row in this
    /// window inserts; `FUN_00642a80` (out of batch) does.  This helper exists
    /// so the sweep rows below can be exercised on a live chain: it removes
    /// the record from the free list and appends it to the active list.
    pub fn a87_push_active(&mut self, index: usize) -> bool {
        if index >= A87_SCRIPT_POOL_RECORDS {
            return false;
        }
        let address = a87_record_address(index);
        if !self.a87_unlink_free(address) {
            return false;
        }
        self.pool.records[index][A87_REC_NEXT] = 0;
        self.pool.records[index][A87_REC_PREV] = self.active_tail;
        if self.active_tail == 0 {
            self.active_head = address;
        } else if let Some(tail) = a87_record_index(self.active_tail) {
            self.pool.records[tail][A87_REC_NEXT] = address;
        }
        self.active_tail = address;
        true
    }

    fn a87_unlink_free(&mut self, address: u32) -> bool {
        let mut cursor = self.free_head;
        let mut prev = 0;
        while cursor != 0 {
            let Some(index) = a87_record_index(cursor) else {
                return false;
            };
            let next = self.pool.records[index][A87_REC_NEXT];
            if cursor == address {
                if prev == 0 {
                    self.free_head = next;
                } else if let Some(prev_index) = a87_record_index(prev) {
                    self.pool.records[prev_index][A87_REC_NEXT] = next;
                }
                if let Some(next_index) = a87_record_index(next) {
                    self.pool.records[next_index][A87_REC_PREV] = prev;
                }
                return true;
            }
            prev = cursor;
            cursor = next;
        }
        false
    }

    /// `0x00640390`: zero dwords 2..=12 of every record in the chain that
    /// starts at `head`, following `next` until the null terminator.
    ///
    /// The shipped loop leaves the two link words alone and then falls into
    /// the non-returning routine at `FUN_00c8a080`; the model returns the number
    /// of visited records.  The 100-record cap only guards against a corrupt
    /// cycle, which the shipped loop would not survive either.
    pub fn a87_clear_chain(&mut self, head: usize) -> usize {
        let mut cursor = Some(head);
        let mut visited = 0;
        while let Some(index) = cursor {
            if index >= A87_SCRIPT_POOL_RECORDS || visited >= A87_SCRIPT_POOL_RECORDS {
                break;
            }
            for field in 2..A87_SCRIPT_RECORD_DWORDS {
                self.pool.records[index][field] = 0;
            }
            visited += 1;
            cursor = a87_record_index(self.pool.records[index][A87_REC_NEXT]);
        }
        visited
    }

    /// `0x00640830`: remove every active record whose `+0x2c` owner word
    /// equals `owner`.
    ///
    /// The shipped walker saves `next` before touching the record and then
    /// relinks the record onto the free list, so a record removed earlier in
    /// the walk cannot break the traversal.  Returns the removed indices in
    /// visit order.
    pub fn a87_kill_owned(&mut self, owner: u32) -> Vec<usize> {
        let mut removed = Vec::new();
        let mut cursor = self.active_head;
        while cursor != 0 {
            let Some(index) = a87_record_index(cursor) else {
                break;
            };
            let next = self.pool.records[index][A87_REC_NEXT];
            if self.pool.records[index][A87_REC_OWNER] == owner {
                self.a87_release_record(index);
                removed.push(index);
            }
            cursor = next;
        }
        removed
    }

    /// `0x006408b0`: one delay-sweep pass over the active list.
    ///
    /// For every visited record the shipped code saves `next`, reads the delay
    /// word, stores `delay - 1` and only then tests the value it read: a
    /// record whose delay was already zero runs (flag bit 2 clear) or is
    /// unlinked (flag bit 2 set).  The decrement wraps like the original
    /// 32-bit store.
    pub fn a87_tick(&mut self) -> Vec<A87ScriptStep> {
        let mut steps = Vec::new();
        let mut cursor = self.active_head;
        while cursor != 0 {
            let Some(index) = a87_record_index(cursor) else {
                break;
            };
            let next = self.pool.records[index][A87_REC_NEXT];
            let delay = self.pool.records[index][A87_REC_DELAY];
            self.pool.records[index][A87_REC_DELAY] = delay.wrapping_sub(1);
            if delay == 0 {
                if self.pool.records[index][A87_REC_FLAGS] & A87_REC_FLAG_UNLINK == 0 {
                    steps.push(A87ScriptStep::Executed(index));
                } else {
                    self.a87_release_record(index);
                    steps.push(A87ScriptStep::Unlinked(index));
                }
            } else {
                steps.push(A87ScriptStep::Decremented(index, delay - 1));
            }
            cursor = next;
        }
        steps
    }

    /// Walk the active list and return the indices in visit order (test and
    /// inspection helper; never longer than the pool).
    pub fn a87_active_list(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut cursor = self.active_head;
        while let Some(index) = a87_record_index(cursor) {
            if out.len() >= A87_SCRIPT_POOL_RECORDS {
                break;
            }
            out.push(index);
            cursor = self.pool.records[index][A87_REC_NEXT];
        }
        out
    }

    /// Walk the free list and return the indices in visit order.
    pub fn a87_free_list(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut cursor = self.free_head;
        while let Some(index) = a87_record_index(cursor) {
            if out.len() >= A87_SCRIPT_POOL_RECORDS {
                break;
            }
            out.push(index);
            cursor = self.pool.records[index][A87_REC_NEXT];
        }
        out
    }
}

/// Result of one record visited by [`A87ScriptRuntime::a87_tick`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A87ScriptStep {
    /// The delay word was nonzero; it was decremented to the given value.
    Decremented(usize, u32),
    /// The delay word was zero and the record's flag byte had bit 2 clear.
    Executed(usize),
    /// The delay word was zero and the record was relinked onto the free list.
    Unlinked(usize),
}

// ---------------------------------------------------------------------------
// Row 0x00640010: identity byte table
// ---------------------------------------------------------------------------

/// `0x00640010`: fill `0x1c` bytes with their own indices.
///
/// The shipped loop is `do { *(char *)(base + i) = (char)i; i++; } while (i <
/// 0x1c);` with `uVar1` starting at zero, so the table is the byte sequence
/// `00 01 .. 1b`.
pub fn a87_fill_index_table(out: &mut [u8; A87_INDEX_TABLE_BYTES]) {
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = index as u8;
    }
}

// ---------------------------------------------------------------------------
// Row 0x00640080: "no object may be pending" guard
// ---------------------------------------------------------------------------

/// `0x00640080`: succeed only when the head-of-list probe returns null.
///
/// The row calls `func_0x005b0fb0(param_1)` (a list probe outside this
/// window) and either returns `0` or enters the non-returning failure routine
/// `FUN_005b4cd0`.  The probe result is therefore the input here: `None` and
/// a zero word both mean "empty" and yield `Ok`; any live pointer yields
/// `Err(pointer)` for the caller to turn into the shipped failure.
pub fn a87_check_no_pending_object(first: Option<u32>) -> Result<(), u32> {
    match first {
        None | Some(0) => Ok(()),
        Some(pointer) => Err(pointer),
    }
}

// ---------------------------------------------------------------------------
// Row 0x006400a7: selector predicate
// ---------------------------------------------------------------------------

/// `0x006400a7`: the exact match predicate of the register-based selector.
///
/// The recovered body returns the `ESI` object when all three conditions
/// hold: `AX == DI`, `EBX <= (FUN_005b4af0() & 0xff)` (an unsigned compare)
/// and the class byte at `ESI+0x4d` is one of `-100`, `-0x60` or `2`.
/// Otherwise it consults the global iterator (`FUN_005b4bb0`) and fails when
/// that returns null, so the predicate below covers the direct-match half.
pub const A87_SELECTOR_CLASSES: [i8; 3] = [-100, -0x60, 2];

/// The class-byte test of `0x006400a7`.
pub fn a87_selector_class_matches(class_byte: i8) -> bool {
    A87_SELECTOR_CLASSES.contains(&class_byte)
}

/// The full direct-match predicate of `0x006400a7`.
pub fn a87_selector_matches(ax: i16, di: i16, ebx: u32, query: u8, class_byte: i8) -> bool {
    ax == di && ebx <= u32::from(query) && a87_selector_class_matches(class_byte)
}

// ---------------------------------------------------------------------------
// Rows 0x006401a6 / 0x006403f0: pool restore and save
// ---------------------------------------------------------------------------

/// `0x006401a6`: rebuild the pool from a saved image.
///
/// The row copies the 100 saved records into the static block, replacing each
/// nonzero link-index word with `0x00fc86dc + index * 0x34`, and reloads the
/// free-list head from the saved word at offset `0x1450` with the same
/// conversion.  The interpreter then returns through the non-returning
/// routine `FUN_00923e90`; the model returns the rebuilt runtime with an empty
/// active list.
pub fn a87_restore_script_pool(saved: &A87SavedScriptPool) -> A87ScriptRuntime {
    let mut pool = A87ScriptPool::default();
    for index in 0..A87_SCRIPT_POOL_RECORDS {
        pool.records[index] = saved.records[index];
        pool.records[index][A87_REC_NEXT] =
            a87_restored_address(saved.records[index][A87_REC_NEXT]);
        pool.records[index][A87_REC_PREV] =
            a87_restored_address(saved.records[index][A87_REC_PREV]);
    }
    A87ScriptRuntime {
        pool,
        free_head: a87_restored_address(saved.free_head),
        active_head: 0,
        active_tail: 0,
    }
}

/// `0x006403f0`: save the pool.
///
/// The mirror of `0x006401a6`: every record is copied and the two link words
/// are replaced by their 1-based indices through
/// `(address - 0x00fc8710) / 0x34 + 1`; the free-list head index is written
/// at saved offset `0x1450`.  The row then calls `FUN_00687d90` and exits
/// through `FUN_00923e90`; those calls are outside this window.
pub fn a87_save_script_pool(runtime: &A87ScriptRuntime) -> A87SavedScriptPool {
    let mut saved = A87SavedScriptPool::default();
    for index in 0..A87_SCRIPT_POOL_RECORDS {
        saved.records[index] = runtime.pool.records[index];
        saved.records[index][A87_REC_NEXT] =
            a87_saved_index(runtime.pool.records[index][A87_REC_NEXT]);
        saved.records[index][A87_REC_PREV] =
            a87_saved_index(runtime.pool.records[index][A87_REC_PREV]);
    }
    saved.free_head = a87_saved_index(runtime.free_head);
    saved
}

// ---------------------------------------------------------------------------
// Rows 0x00640320 / 0x00640348: interpreter frame setup
// ---------------------------------------------------------------------------

/// One SEH frame as `0x00640320` links it: the previous list head and the
/// handler word `&DAT_00cc7258`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A87ExceptionFrame {
    pub previous: Option<u32>,
    pub handler: u32,
}

/// `0x00640320`: link a new exception frame for the `0x1454`-byte
/// interpreter frame and compute the cookie.
///
/// The row stores `0xffffffff` in the frame's state slot, sets
/// `handler = &DAT_00cc7258`, pushes the current `ExceptionList` and calls
/// `FUN_00923df0(0x1454, DAT_00f4e578 ^ &stack0xfffffffc)`.  The first
/// argument is the frame size and the second is the security cookie xor-ed
/// with the frame pointer; both are reproduced here, while the stack probe
/// itself is outside the window.
pub fn a87_push_script_exception_frame(
    previous: Option<u32>,
    security_cookie: u32,
    stack_pointer: u32,
) -> (A87ExceptionFrame, u32) {
    (
        A87ExceptionFrame {
            previous,
            handler: A87_SCRIPT_SEH_HANDLER,
        },
        security_cookie ^ stack_pointer,
    )
}

/// `0x00640348`: the bare frame probe.  The row is a single
/// `FUN_00923df0(0x1454)` call, so the recovered constant is the frame size.
pub fn a87_script_frame_bytes() -> u32 {
    A87_SCRIPT_FRAME_BYTES
}

// ---------------------------------------------------------------------------
// Row 0x006405c0: release one mapped script image
// ---------------------------------------------------------------------------

/// One of the two loaded script images (`0x00fc8700/0x04` or
/// `0x00fc8708/0x0c`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A87MappedImage {
    /// Nonzero ownership token standing in for the mapping address.
    pub handle: u32,
    /// Length word, already rounded by [`a87_script_image_size`].
    pub size: u32,
}

impl A87MappedImage {
    /// `0x00640620`: round the file length, then allocate and copy it.
    ///
    /// The shipped code sets the length word first, allocates
    /// `FUN_009241e0(length, &DAT_00d32a20, 0, 8)` and copies `local_8`
    /// bytes with `FUN_00c89580`.  Both calls are outside this window; the
    /// model records the rounded length and a nonzero handle for a present
    /// file, and an all-zero image for an absent one.
    pub fn a87_load(file_size: Option<u32>) -> Self {
        match file_size {
            Some(size) => Self {
                handle: 1,
                size: a87_script_image_size(size),
            },
            None => Self::default(),
        }
    }
}

/// `0x00640620`: the length stored for a mapped script file.
///
/// The row computes `local_8 + 0x6000U & 0xfffffffc`, i.e. `0x6000` bytes of
/// headroom followed by a four-byte alignment.  The addition wraps like the
/// shipped unsigned add.
pub fn a87_script_image_size(file_size: u32) -> u32 {
    file_size.wrapping_add(A87_SCRIPT_IMAGE_HEADROOM) & A87_SCRIPT_IMAGE_ALIGN_MASK
}

/// `0x006405c0`: release a mapped script image.
///
/// The row zeroes the length word only when the base word is nonzero, then
/// enters the non-returning release routine `FUN_00924220(base,
/// &DAT_00d32a20, 0, 0)`.  The model performs the conditional length clear and
/// clears the handle; `true` means a mapping was present.
pub fn a87_unmap_script_image(image: &mut A87MappedImage) -> bool {
    if image.handle != 0 {
        image.size = 0;
        image.handle = 0;
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Row 0x006406b8: rotate the word and rebind the bwscript length
// ---------------------------------------------------------------------------

/// Result of `0x006406b8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A87BwScriptRebind {
    /// The rotated dword the row stores back through `in_EAX`.
    pub rotated: u32,
    /// The new `0x00fc870c` length word (`in_ECX`).
    pub size: u32,
}

/// `0x006406b8`: rotate a dword and publish a new bwscript length.
///
/// The recovered body is
/// `*in_EAX = *in_EAX << (in_ECX & 0x1f) | *in_EAX >> (0x20 - (in_ECX & 0x1f))`
/// followed by `0x00fc870c = in_ECX` and a fresh allocation at
/// `0x00fc8708`.  On x86 the shift counts are masked to five bits, which makes
/// the expression exactly `word.rotate_left(count & 0x1f)`.  The allocation
/// is outside this window.
pub fn a87_rotate_and_resize(word: u32, count: u32) -> A87BwScriptRebind {
    A87BwScriptRebind {
        rotated: word.rotate_left(count & 0x1f),
        size: count,
    }
}

// ---------------------------------------------------------------------------
// Rows 0x00640940 / 0x00640999 / 0x006409d3: script-id dispatch
// ---------------------------------------------------------------------------

/// One 8-byte `(id, code offset)` table entry of `aiscript.bin` or
/// `bwscript.bin`.  Both tables are scanned in 8-byte steps and terminated by
/// an entry whose id word is zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A87ScriptEntry {
    pub id: u32,
    pub offset: u32,
}

/// A matched script: the code offset and whether it came from
/// `bwscript.bin` (the row's fourth argument `uVar4`, `0` for
/// `aiscript.bin` and `1` for `bwscript.bin`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A87ScriptHit {
    pub offset: u32,
    pub external: bool,
}

/// Reasons `0x00640940` does not reach its dispatch call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A87DispatchError {
    /// The id is not one of the five words at `0x00f056b8`.
    UnknownId,
    /// Neither table contains a non-zero code offset for the id.
    NotFound,
}

/// `0x00640940`: the five ids the dispatch row accepts, recovered from the
/// literal string `"EnBkEnTrExTrDWHePB3F"` at `0x00f056b8`.
pub fn a87_script_id_accepted(id: u32) -> bool {
    A87_SCRIPT_ID_TABLE.contains(&id)
}

/// Scan one script table for `id`.
///
/// Mirrors the row's loop: stop at the first entry whose id is zero, accept
/// the first id match, and treat a matching entry with a zero code offset as
/// "no dispatch".
pub fn a87_find_script_entry(table: &[A87ScriptEntry], id: u32) -> Option<u32> {
    for entry in table {
        if entry.id == 0 {
            return None;
        }
        if entry.id == id {
            return (entry.offset != 0).then_some(entry.offset);
        }
    }
    None
}

/// `0x00640940`: validate the id, then search `aiscript.bin` before
/// `bwscript.bin`.
///
/// The row masks the id table with the five-word loop and falls into
/// `FUN_00748d40` for an unknown id.  It scans the first mapping only when
/// its length is nonzero, then the second, and passes `0` or `1` as the
/// external flag to `FUN_00642a80`.
pub fn a87_dispatch_script(
    ai_table: &[A87ScriptEntry],
    bw_table: &[A87ScriptEntry],
    id: u32,
) -> Result<A87ScriptHit, A87DispatchError> {
    if !a87_script_id_accepted(id) {
        return Err(A87DispatchError::UnknownId);
    }
    if let Some(offset) = a87_find_script_entry(ai_table, id) {
        return Ok(A87ScriptHit {
            offset,
            external: false,
        });
    }
    if let Some(offset) = a87_find_script_entry(bw_table, id) {
        return Ok(A87ScriptHit {
            offset,
            external: true,
        });
    }
    Err(A87DispatchError::NotFound)
}

/// `0x00640999`: the panel record offset.
///
/// The row writes the raw `EBX` when the object word is zero and
/// `(EBX + 0xc) * 0x24` otherwise.  The multiply wraps like the shipped
/// 32-bit `imul`.
pub fn a87_panel_slot(index: u32, has_object: bool) -> u32 {
    if has_object {
        index
            .wrapping_add(A87_PANEL_SLOT_BIAS)
            .wrapping_mul(A87_PANEL_RECORD_BYTES)
    } else {
        index
    }
}

/// `0x00640999`: read the panel state byte at `slot + 8`.
pub fn a87_panel_state(panel: &[u8], slot: u32) -> Option<u8> {
    panel
        .get(slot.wrapping_add(A87_PANEL_STATE_OFFSET) as usize)
        .copied()
}

/// `0x00640999`: the state byte must be exactly `1`.
pub fn a87_panel_state_running(state: u8) -> bool {
    state == A87_PANEL_STATE_RUNNING
}

/// `0x006409d3`: the special-case test.
///
/// The row continues only when the panel byte at `+8` is `7` and the id word
/// is `0x75637352` (`"Rscu"`); it then searches both script tables and
/// dispatches with the external flag fixed to `0`.
pub fn a87_special_script_matches(panel_kind: u8, id: u32) -> bool {
    panel_kind == A87_PANEL_KIND_SPECIAL && id == A87_SPECIAL_SCRIPT_ID
}

// ---------------------------------------------------------------------------
// Row 0x00640a90: interpreter cost lookup
// ---------------------------------------------------------------------------

/// `0x00640a90`: the `0x1e`/`5` cost special case.
///
/// The row looks `script_type & 0xffff` up in one of two tables
/// (`FUN_00695be0` when `param_3 == 0`, `FUN_00695c10` otherwise), then adds
/// the other special entry when the type is `0x1e` or `5`:
///
/// * type `0x1e` returns `lookup(0x1e) + lookup(5)`;
/// * type `5` returns `lookup(5) + lookup(0x1e)`;
/// * every other type returns `lookup(type)`.
///
/// The two lookups are passed in so the tables stay outside this revision;
/// `slot` is the second register argument the decompiler preserves for the
/// nested call.  The adds wrap like the shipped 32-bit `add`.
pub fn a87_script_cost<F, G>(
    script_type: u32,
    slot: u8,
    absolute: bool,
    relative: F,
    absolute_table: G,
) -> i32
where
    F: Fn(u16, u8) -> i32,
    G: Fn(u16, u8) -> i32,
{
    let masked = (script_type & 0xffff) as u16;
    if !absolute {
        let mut value = relative(masked, slot);
        if script_type == A87_COST_TYPE_DELAY {
            return relative(A87_COST_TYPE_BUILD as u16, slot).wrapping_add(value);
        }
        if script_type == A87_COST_TYPE_BUILD {
            value = relative(A87_COST_TYPE_DELAY as u16, slot).wrapping_add(value);
        }
        value
    } else {
        let value = absolute_table(masked, slot);
        if script_type == A87_COST_TYPE_DELAY {
            return absolute_table(A87_COST_TYPE_BUILD as u16, slot).wrapping_add(value);
        }
        if script_type == A87_COST_TYPE_BUILD {
            return absolute_table(A87_COST_TYPE_DELAY as u16, slot).wrapping_add(value);
        }
        value
    }
}

// ---------------------------------------------------------------------------
// Rows 0x00640be8 / 0x00640e25: interior entries of FUN_00640b20
// ---------------------------------------------------------------------------

/// `0x00640be8`: the shared unwind epilogue of `FUN_00640b20`.
///
/// The four instructions restore `ExceptionList` from the interpreter frame
/// at `[ebp-0xc]` and jump to the non-returning unwinder `FUN_00c86b6e`.
/// Ghidra also lists this address as a function because the interpreter's
/// switch jumps here; there is no independent entry point, so the model
/// returns the recovered exception-list word for the caller to install.
pub fn a87_interpreter_unwind(saved_exception_list: Option<u32>) -> Option<u32> {
    saved_exception_list
}

/// Result of the `0x00640e25` operand guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A87OperandFetch {
    /// Carry was set and `cursor + 2` is still below the limit; the new
    /// cursor is returned.
    Word { cursor: u32 },
    /// The guard failed and the shipped row falls into its diagnostic tail.
    Truncated,
}

/// `0x00640e25`: guard an already-advanced word operand.
///
/// The recovered block is `if (CF && in_EAX + 2U < in_ECX)` then store
/// `in_EAX + 2` in the record's cursor word and return; otherwise the
/// decompiler loses the flow in the diagnostic string builder.  The unsigned
/// comparison and the wrapping add are reproduced exactly.
pub fn a87_operand_word_advance(carry: bool, cursor: u32, limit: u32) -> A87OperandFetch {
    let advanced = cursor.wrapping_add(2);
    if carry && advanced < limit {
        A87OperandFetch::Word { cursor: advanced }
    } else {
        A87OperandFetch::Truncated
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent87 {
    use super::*;

    /// `0x00640010`: the byte table is the identity sequence.
    #[test]
    fn index_table_matches_0x00640010() {
        let mut table = [0xffu8; A87_INDEX_TABLE_BYTES];
        a87_fill_index_table(&mut table);
        let expected: Vec<u8> = (0..A87_INDEX_TABLE_BYTES as u8).collect();
        assert_eq!(table.to_vec(), expected);
        assert_eq!(table[0], 0);
        assert_eq!(table[0x1b], 0x1b);
    }

    /// `0x00640080`: only a null probe passes.
    #[test]
    fn object_list_check_matches_0x00640080() {
        assert_eq!(a87_check_no_pending_object(None), Ok(()));
        assert_eq!(a87_check_no_pending_object(Some(0)), Ok(()));
        assert_eq!(
            a87_check_no_pending_object(Some(0x00fc_8710)),
            Err(0x00fc_8710)
        );
    }

    /// `0x006400a7`: the three-condition direct match.
    #[test]
    fn selector_matches_0x006400a7() {
        assert!(a87_selector_matches(7, 7, 5, 5, 2));
        assert!(a87_selector_matches(7, 7, 0, 0xff, -100));
        assert!(a87_selector_matches(-1, -1, 0xff, 0xff, -0x60));
        // Query byte is the low byte of the probe result.
        assert!(!a87_selector_matches(7, 7, 6, 5, 2));
        // AX must equal DI.
        assert!(!a87_selector_matches(7, 8, 0, 0xff, 2));
        // Only the three recovered class bytes are accepted.
        assert!(!a87_selector_matches(7, 7, 0, 0xff, 1));
        assert!(!a87_selector_matches(7, 7, 0, 0xff, 7));
        assert_eq!(A87_SELECTOR_CLASSES, [-100, -96, 2]);
    }

    /// `0x00640620`: the full 100-record chain, the free head and the
    /// separately written last record.
    #[test]
    fn pool_init_matches_0x00640620() {
        let runtime = A87ScriptRuntime::a87_init();
        assert_eq!(a87_record_address(0), 0x00fc_8710);
        assert_eq!(a87_record_address(99), 0x00fc_9b2c);
        assert_eq!(a87_record_address(98), 0x00fc_9af8);
        assert_eq!(a87_record_address(100), 0x00fc_9b60);
        assert_eq!(runtime.pool.records[0][A87_REC_NEXT], a87_record_address(1));
        assert_eq!(runtime.pool.records[0][A87_REC_PREV], 0);
        assert_eq!(runtime.pool.records[99][A87_REC_NEXT], 0);
        assert_eq!(
            runtime.pool.records[99][A87_REC_PREV],
            a87_record_address(98)
        );
        assert_eq!(
            runtime.pool.records[98][A87_REC_NEXT],
            a87_record_address(99)
        );
        // Every non-link dword starts zero.
        for index in 0..A87_SCRIPT_POOL_RECORDS {
            for field in 2..A87_SCRIPT_RECORD_DWORDS {
                assert_eq!(runtime.pool.records[index][field], 0);
            }
        }
        assert_eq!(runtime.free_head, a87_record_address(0));
        assert_eq!(runtime.active_head, 0);
        assert_eq!(runtime.active_tail, 0);
        assert_eq!(runtime.a87_free_list().len(), A87_SCRIPT_POOL_RECORDS);
        assert_eq!(runtime.a87_active_list(), Vec::<usize>::new());
        // 0x00fc9b30 is record 99's `previous` word.
        assert_eq!(a87_record_address(99) + 4, 0x00fc_9b30);
        // 0x00fc9b60 is the free-list word at pool offset 0x1450.
        assert_eq!(
            A87_SCRIPT_FREE_LIST,
            A87_SCRIPT_POOL_BASE + A87_SCRIPT_POOL_BYTES
        );
    }

    /// `0x006401a6` / `0x006403f0`: the saved-index formula and the round
    /// trip through both conversions.
    #[test]
    fn save_restore_roundtrip_matches_pool_rows() {
        assert_eq!(a87_saved_index(0), 0);
        assert_eq!(a87_saved_index(a87_record_address(0)), 1);
        assert_eq!(a87_saved_index(a87_record_address(5)), 6);
        assert_eq!(a87_saved_index(a87_record_address(99)), 100);
        assert_eq!(a87_restored_address(0), 0);
        assert_eq!(a87_restored_address(1), a87_record_address(0));
        assert_eq!(a87_restored_address(100), a87_record_address(99));
        assert_eq!(A87_SCRIPT_INDEX_ORIGIN, 0x00fc_86dc);

        let runtime = A87ScriptRuntime::a87_init();
        let saved = a87_save_script_pool(&runtime);
        assert_eq!(saved.free_head, 1);
        assert_eq!(saved.records[0][A87_REC_NEXT], 2);
        assert_eq!(saved.records[99][A87_REC_NEXT], 0);
        assert_eq!(saved.records[99][A87_REC_PREV], 99);

        let restored = a87_restore_script_pool(&saved);
        assert_eq!(restored.pool, runtime.pool);
        assert_eq!(restored.free_head, runtime.free_head);
        assert_eq!(restored.active_head, 0);
    }

    /// `0x00640020`: unlink from the middle, head and tail of the active
    /// list, then push onto the free list.
    #[test]
    fn release_record_matches_0x00640020() {
        let mut runtime = A87ScriptRuntime::a87_init();
        // Build an active list 0,1,2 out of the free list.
        assert!(runtime.a87_push_active(0));
        assert!(runtime.a87_push_active(1));
        assert!(runtime.a87_push_active(2));
        assert_eq!(runtime.a87_active_list(), vec![0, 1, 2]);
        assert_eq!(runtime.active_tail, a87_record_address(2));

        // Middle record.
        assert!(runtime.a87_release_record(1));
        assert_eq!(runtime.a87_active_list(), vec![0, 2]);
        assert_eq!(runtime.free_head, a87_record_address(1));
        // The push links the record to the free head that existed before it:
        // records 0..2 were taken off the free list, so that head is record 3.
        assert_eq!(runtime.pool.records[1][A87_REC_NEXT], a87_record_address(3));
        assert_eq!(runtime.pool.records[1][A87_REC_PREV], 0);

        // Head record: the next record's previous link becomes null.
        assert!(runtime.a87_release_record(0));
        assert_eq!(runtime.a87_active_list(), vec![2]);
        assert_eq!(runtime.free_head, a87_record_address(0));
        assert_eq!(runtime.pool.records[2][A87_REC_PREV], 0);

        // Tail record: the active tail follows the null next link.
        assert!(runtime.a87_release_record(2));
        assert_eq!(runtime.a87_active_list(), Vec::<usize>::new());
        assert_eq!(runtime.active_head, 0);
        assert_eq!(runtime.active_tail, 0);
        assert_eq!(runtime.free_head, a87_record_address(2));
        // Free-list order is LIFO: 2, 0, 1, then the untouched records.
        let free = runtime.a87_free_list();
        assert_eq!(&free[..3], &[2, 0, 1]);
        // `0x006403f0` saves the free head as a 1-based index: record 2 -> 3.
        let saved = a87_save_script_pool(&runtime);
        assert_eq!(saved.free_head, 3);
        assert_eq!(saved.records[1][A87_REC_NEXT], 4);
        assert!(!runtime.a87_release_record(A87_SCRIPT_POOL_RECORDS));
    }

    /// `0x00640390`: the chain clear zeroes dwords 2..=12 and keeps links.
    #[test]
    fn clear_chain_matches_0x00640390() {
        let mut runtime = A87ScriptRuntime::a87_init();
        runtime.pool.records[0][A87_REC_FLAGS] = 0xffff_ffff;
        runtime.pool.records[0][A87_REC_OWNER] = 0x1234_5678;
        runtime.pool.records[57][A87_REC_DELAY] = 9;
        assert_eq!(runtime.a87_clear_chain(0), A87_SCRIPT_POOL_RECORDS);
        for index in 0..A87_SCRIPT_POOL_RECORDS {
            for field in 2..A87_SCRIPT_RECORD_DWORDS {
                assert_eq!(runtime.pool.records[index][field], 0);
            }
        }
        // Links survive the clear.
        assert_eq!(runtime.pool.records[0][A87_REC_NEXT], a87_record_address(1));
        assert_eq!(
            runtime.pool.records[99][A87_REC_PREV],
            a87_record_address(98)
        );
    }

    /// `0x006405c0`: a present image loses its length before the release; an
    /// absent one is left alone.
    #[test]
    fn release_image_matches_0x006405c0() {
        let mut image = A87MappedImage::a87_load(Some(0x1235));
        assert_eq!(image.size, 0x7234);
        assert_ne!(image.handle, 0);
        assert!(a87_unmap_script_image(&mut image));
        assert_eq!(image, A87MappedImage::default());

        let mut absent = A87MappedImage::default();
        assert!(!a87_unmap_script_image(&mut absent));
        assert_eq!(absent, A87MappedImage::default());

        // The length is stored before the handle, so an image whose handle is
        // zero keeps whatever length word was written (the row only clears it
        // inside the nonzero-handle branch).
        let mut zero_handle = A87MappedImage {
            handle: 0,
            size: 0x10,
        };
        assert!(!a87_unmap_script_image(&mut zero_handle));
        assert_eq!(zero_handle.size, 0x10);
    }

    /// `0x00640620`: the `+0x6000` headroom and four-byte alignment.
    #[test]
    fn image_size_matches_0x00640620() {
        assert_eq!(a87_script_image_size(0), 0x6000);
        assert_eq!(a87_script_image_size(1), 0x6000);
        assert_eq!(a87_script_image_size(2), 0x6000);
        assert_eq!(a87_script_image_size(3), 0x6000);
        assert_eq!(a87_script_image_size(4), 0x6004);
        assert_eq!(a87_script_image_size(0x1235), 0x7234);
        assert_eq!(a87_script_image_size(u32::MAX), 0x5ffc);
        assert_eq!(a87_script_image_size(0x1000), 0x7000);
        // Absent files leave the length and handle words at zero.
        assert_eq!(A87MappedImage::a87_load(None), A87MappedImage::default());
    }

    /// `0x006406b8`: the x86-masked rotate plus the published length.
    #[test]
    fn rotate_matches_0x006406b8() {
        assert_eq!(a87_rotate_and_resize(0x8000_0001, 1).rotated, 0x0000_0003);
        assert_eq!(a87_rotate_and_resize(0x1234_5678, 0).rotated, 0x1234_5678);
        assert_eq!(a87_rotate_and_resize(0x1234_5678, 32).rotated, 0x1234_5678);
        assert_eq!(a87_rotate_and_resize(0x1234_5678, 33).rotated, 0x2468_acf0);
        assert_eq!(
            a87_rotate_and_resize(0xdead_beef, 0x1f).rotated,
            0xef56_df77
        );
        let rebound = a87_rotate_and_resize(0x1111_1111, 0x99);
        assert_eq!(rebound.size, 0x99);
        assert_eq!(rebound.rotated, 0x2222_2222);
    }

    /// `0x00640830`: the owner sweep takes every matching record and leaves
    /// the rest linked, in order.
    #[test]
    fn kill_owned_matches_0x00640830() {
        let mut runtime = A87ScriptRuntime::a87_init();
        for index in 0..4 {
            assert!(runtime.a87_push_active(index));
        }
        runtime.pool.records[0][A87_REC_OWNER] = 0xaa;
        runtime.pool.records[1][A87_REC_OWNER] = 0xbb;
        runtime.pool.records[2][A87_REC_OWNER] = 0xaa;
        runtime.pool.records[3][A87_REC_OWNER] = 0xcc;

        assert_eq!(runtime.a87_kill_owned(0xaa), vec![0, 2]);
        assert_eq!(runtime.a87_active_list(), vec![1, 3]);
        assert_eq!(runtime.active_head, a87_record_address(1));
        assert_eq!(runtime.active_tail, a87_record_address(3));
        assert_eq!(runtime.a87_kill_owned(0xaa), Vec::<usize>::new());
        assert_eq!(runtime.a87_kill_owned(0xcc), vec![3]);
        assert_eq!(runtime.a87_active_list(), vec![1]);
        // The removed records are at the head of the free list.
        let free = runtime.a87_free_list();
        assert_eq!(&free[..2], &[3, 2]);
    }

    /// `0x006408b0`: decrement, run at a pre-decrement zero, and unlink the
    /// bit-2 records; traversal follows the saved next word.
    #[test]
    fn tick_matches_0x006408b0() {
        let mut runtime = A87ScriptRuntime::a87_init();
        for index in 0..3 {
            assert!(runtime.a87_push_active(index));
        }
        runtime.pool.records[0][A87_REC_DELAY] = 0;
        runtime.pool.records[1][A87_REC_DELAY] = 1;
        runtime.pool.records[2][A87_REC_DELAY] = 0;
        runtime.pool.records[2][A87_REC_FLAGS] = A87_REC_FLAG_UNLINK;

        assert_eq!(
            runtime.a87_tick(),
            vec![
                A87ScriptStep::Executed(0),
                A87ScriptStep::Decremented(1, 0),
                A87ScriptStep::Unlinked(2),
            ]
        );
        // The executed record wrapped its zero delay.
        assert_eq!(runtime.pool.records[0][A87_REC_DELAY], u32::MAX);
        assert_eq!(runtime.pool.records[1][A87_REC_DELAY], 0);
        assert_eq!(runtime.a87_active_list(), vec![0, 1]);

        // Second pass: record 0 still has a nonzero (wrapped) delay, record 1
        // now runs because its delay reads zero.
        assert_eq!(
            runtime.a87_tick(),
            vec![
                A87ScriptStep::Decremented(0, u32::MAX - 1),
                A87ScriptStep::Executed(1),
            ]
        );
        // The interpreter reload path writes 0x1e and clears the next word.
        assert!(runtime.a87_reload_delay(0));
        assert_eq!(runtime.pool.records[0][A87_REC_DELAY], 0x1e);
        assert_eq!(runtime.pool.records[0][A87_REC_DELAY + 1], 0);
        assert!(!runtime.a87_reload_delay(A87_SCRIPT_POOL_RECORDS));
    }

    /// `0x00640940`: the five-id gate, the aiscript-before-bwscript order and
    /// the external flag.
    #[test]
    fn dispatch_matches_0x00640940() {
        let enbk = A87_SCRIPT_ID_TABLE[0];
        assert_eq!(a87_script_id_accepted(enbk), true);
        assert!(!a87_script_id_accepted(u32::from_le_bytes(*b"Zzzz")));
        assert_eq!(A87_SCRIPT_ID_TABLE.len(), 5);
        assert_eq!(A87_SCRIPT_ID_TABLE[4], u32::from_le_bytes(*b"PB3F"));

        let ai = [
            A87ScriptEntry {
                id: enbk,
                offset: 0x40,
            },
            A87ScriptEntry { id: 0, offset: 0 },
        ];
        let bw = [
            A87ScriptEntry {
                id: enbk,
                offset: 0x80,
            },
            A87ScriptEntry { id: 0, offset: 0 },
        ];
        assert_eq!(
            a87_dispatch_script(&ai, &bw, enbk),
            Ok(A87ScriptHit {
                offset: 0x40,
                external: false
            })
        );
        assert_eq!(
            a87_dispatch_script(&[], &bw, enbk),
            Ok(A87ScriptHit {
                offset: 0x80,
                external: true
            })
        );
        assert_eq!(
            a87_dispatch_script(&ai, &bw, u32::from_le_bytes(*b"Zzzz")),
            Err(A87DispatchError::UnknownId)
        );
        // The terminator stops the scan even when the id appears later.
        let terminated = [
            A87ScriptEntry { id: 0, offset: 0 },
            A87ScriptEntry {
                id: enbk,
                offset: 0x40,
            },
        ];
        assert_eq!(
            a87_dispatch_script(&terminated, &[], enbk),
            Err(A87DispatchError::NotFound)
        );
        // A matching id with a zero code offset is not dispatched.
        let zero_offset = [A87ScriptEntry {
            id: enbk,
            offset: 0,
        }];
        assert_eq!(
            a87_dispatch_script(&zero_offset, &[], enbk),
            Err(A87DispatchError::NotFound)
        );
        assert_eq!(a87_find_script_entry(&zero_offset, enbk), None);
    }

    /// `0x00640999` / `0x006409d3`: the `(index + 0xc) * 0x24` panel offset,
    /// the state byte at `+8`, and the special kind/id test.
    #[test]
    fn panel_rows_match_0x00640999_and_0x006409d3() {
        assert_eq!(a87_panel_slot(5, false), 5);
        assert_eq!(a87_panel_slot(5, true), 0x11 * 0x24);
        assert_eq!(a87_panel_slot(0, true), 0x0c * 0x24);
        // `u32::MAX + 0xc` wraps to `0xb`, so the offset is `0xb * 0x24`.
        assert_eq!(a87_panel_slot(u32::MAX, true), 0x0b * 0x24);

        let mut panel = vec![0u8; 0x200];
        let slot = a87_panel_slot(1, true);
        panel[slot as usize + A87_PANEL_STATE_OFFSET as usize] = 1;
        assert_eq!(a87_panel_state(&panel, slot), Some(1));
        assert!(a87_panel_state_running(1));
        assert!(!a87_panel_state_running(0));
        assert!(!a87_panel_state_running(4));
        assert_eq!(a87_panel_state(&panel, 0xffff_fff0), None);

        assert!(a87_special_script_matches(7, A87_SPECIAL_SCRIPT_ID));
        assert_eq!(A87_SPECIAL_SCRIPT_ID, 0x7563_7352);
        assert!(!a87_special_script_matches(1, A87_SPECIAL_SCRIPT_ID));
        assert!(!a87_special_script_matches(7, 0x1234_5678));
    }

    /// `0x00640a90`: both tables and both special-cased types.
    #[test]
    fn cost_matches_0x00640a90() {
        let relative = |script_type: u16, slot: u8| i32::from(script_type) * 10 + i32::from(slot);
        let absolute = |script_type: u16, slot: u8| i32::from(script_type) * 100 + i32::from(slot);

        assert_eq!(
            a87_script_cost(0x1e, 7, false, relative, absolute),
            0x1e * 10 + 7 + 5 * 10 + 7
        );
        assert_eq!(
            a87_script_cost(5, 7, false, relative, absolute),
            5 * 10 + 7 + 0x1e * 10 + 7
        );
        assert_eq!(
            a87_script_cost(0x1e, 7, true, relative, absolute),
            0x1e * 100 + 7 + 5 * 100 + 7
        );
        assert_eq!(
            a87_script_cost(5, 7, true, relative, absolute),
            5 * 100 + 7 + 0x1e * 100 + 7
        );
        assert_eq!(a87_script_cost(9, 3, false, relative, absolute), 9 * 10 + 3);
        assert_eq!(a87_script_cost(9, 3, true, relative, absolute), 9 * 100 + 3);
        // The lookup receives the masked type word; the special comparisons
        // use the unmasked value, so `0x1_001e` is not special.
        assert_eq!(
            a87_script_cost(0x1_0000 | 9, 0, false, relative, absolute),
            9 * 10
        );
        assert_eq!(
            a87_script_cost(0x1_001e, 0, false, relative, absolute),
            0x1e * 10
        );
        // The adds wrap.
        let max = |_: u16, _: u8| i32::MAX;
        assert_eq!(
            a87_script_cost(0x1e, 0, false, max, absolute),
            i32::MAX.wrapping_add(i32::MAX)
        );
    }

    /// `0x00640320` / `0x00640348`: the frame size, the handler word and the
    /// cookie xor.
    #[test]
    fn seh_rows_match_0x00640320_and_0x00640348() {
        assert_eq!(a87_script_frame_bytes(), 0x1454);
        assert_eq!(A87_SCRIPT_FRAME_BYTES, 0x1454);
        assert_eq!(A87_SCRIPT_SEH_HANDLER, 0x00cc_7258);
        assert_eq!(A87_SECURITY_COOKIE_ADDRESS, 0x00f4_e578);

        let (frame, cookie) =
            a87_push_script_exception_frame(Some(0xdead_0000), 0x1122_3344, 0x5000);
        assert_eq!(
            frame,
            A87ExceptionFrame {
                previous: Some(0xdead_0000),
                handler: 0x00cc_7258
            }
        );
        assert_eq!(cookie, 0x1122_6344);

        let (frame, cookie) = a87_push_script_exception_frame(None, 0, 0xffff_ffff);
        assert_eq!(frame.previous, None);
        assert_eq!(cookie, 0xffff_ffff);
    }

    /// `0x00640be8`: the epilogue hands the saved exception-list word back.
    #[test]
    fn unwind_matches_0x00640be8() {
        assert_eq!(a87_interpreter_unwind(Some(0x00cc_7290)), Some(0x00cc_7290));
        assert_eq!(a87_interpreter_unwind(None), None);
    }

    /// `0x00640e25`: the carry-gated word cursor.
    #[test]
    fn operand_guard_matches_0x00640e25() {
        assert_eq!(
            a87_operand_word_advance(true, 0, 4),
            A87OperandFetch::Word { cursor: 2 }
        );
        assert_eq!(
            a87_operand_word_advance(true, 1, 4),
            A87OperandFetch::Word { cursor: 3 }
        );
        assert_eq!(
            a87_operand_word_advance(true, 2, 4),
            A87OperandFetch::Truncated
        );
        assert_eq!(
            a87_operand_word_advance(false, 0, 4),
            A87OperandFetch::Truncated
        );
        assert_eq!(
            a87_operand_word_advance(true, 0xffff_fffd, u32::MAX),
            A87OperandFetch::Truncated
        );
        assert_eq!(
            a87_operand_word_advance(true, 0xffff_fffe, u32::MAX),
            A87OperandFetch::Word { cursor: 0 }
        );
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// * `FUN_00641021` (`FUN_00641021`, 87 bytes) is not a function entry.  The
//   recovered body opens with `*unaff_EDI = *unaff_EDI >> 3 | *unaff_EDI <<
//   0x1d; out(*unaff_ESI,(short)in_EDX); *(char *)in_EAX = (char)*in_EAX +
//   (char)in_EAX;` and then resumes the `FUN_00640b20` switch at
//   `code_r0x00641028`.  It is a jump-table target inside the already
//   translated interpreter body.
// * `FUN_006416a0` (`FUN_006416a0`, 71 bytes) is the same kind of interior
//   entry: `*pbVar1 = *pbVar1 + (byte)in_ECX` then
//   `code_rFUN_006416a0`, the shared case-0x17/0x18 resumption point of the
//   interpreter.
// * `FUN_0064176c` (`FUN_0064176c`, 23 bytes) is the third one: `*in_EAX =
//   *in_EAX + (char)in_EAX` immediately followed by `code_r0x0064176e`, the
//   `FUN_00635340` case tail of the interpreter.  Translating these three
//   rows separately would duplicate `FUN_00640b20`, whose ledger row is already
//   translated with `src/aiscript.rs` as evidence.
//
// The two rows that are translated above only as their recovered interior
// entry are called out in their doc comments: `0x00640be8` is the interpreter
// epilogue and `0x00640e25` is the operand-truncation guard.  They are listed
// in the ledger as separate `decompiled_uncited` functions, so they are
// reconstructed rather than skipped, but neither is an independent shipped
// entry point.

// ===========================================================================
// Merged from src/translated/agent88.rs
// ===========================================================================
// Translation batch 88.  Only this file is edited by the batch.
//
// Batch 88 owns `0x00642000..=0x00643fff`.  The ledger lists 34
// `decompiled_uncited` rows in that window; 20 are translated below and the
// 14 rows that are not are listed at the end with their reason.
//
// Most of the window is the retail AI-script runtime (`FUN_00640080`'s
// opcode interpreter) plus the spawn/region list it drives.  Ghidra split
// that one interpreter into dozens of "functions" whose bodies are the same
// dispatch loop entered at a case label; those rows address their state
// through the caller's `ESI`/`EDI`/`EBP` and carry no self-contained
// operation.  Where a row *does* contain a closed operation (a cursor
// fetch, a string build, a list link, a rect copy) it is translated here as
// a value type plus free functions.  No `sim` record gains a field.
//
// Recovered state:
//
// | owner                 | words                                                                    |
// |-----------------------|--------------------------------------------------------------------------|
// | script state          | `+0x04` byte cursor, `+0x08` unit, `+0x16` location, `+0x18` bank flag   |
// | spawn pool            | `0x1450` free head, `0x00fc9b68` list head, node `{next, prev, ...}`     |
// | spawn node            | `+0x08` location, `+0x10` unit type, `+0x14..0x20` rect, `+0x24/28` center, `+0x30` flags |
// | region node           | `+0x00/04` links, `+0x08/0c` keys, `+0x10` destructor tag, `+0x34..40` rect, `+0x44` valid, `+0x4d/4f/51` bytes |
// | region list           | `+0x00` head, `+0x04` tail, `+0x08` count                                 |
//
// Address to function:
//
// * `0x00642037` [`a88_script_read_byte`]
// * `0x00642484` [`a88_script_store_cursor`]
// * `0x00642687` [`a88_add_al_twice`]
// * `0x00642696` [`a88_invalid_ai_script_words`]
// * `0x006427c6` [`a88_tsc_carry_add`]
// * `0x00642a00` [`a88_spawn_request`]
// * `0x00642a80` [`A88SpawnPool::a88_alloc`]
// * `0x00642b50` [`a88_spawn_at_location`]
// * `0x00642bf0` [`a88_spawn_gate`]
// * `0x00642e9b` [`a88_sso_string_init`]
// * `0x006433d7` [`a88_rotate_right_through_carry`]
// * `0x00643860` [`A88RegionList::a88_reset`]
// * `0x00643910` [`A88RegionList::a88_clear_head`]
// * `0x006439eb` [`A88RegionList::a88_unlink_tail`]
// * `0x00643aa0` [`a88_region_list_present`]
// * `0x00643b10` [`a88_region_snapshot`]
// * `0x00643b30` [`a88_region_scratch`]
// * `0x00643ca4` [`a88_sso_string_init_catch_frame`]
// * `0x00643f40` [`a88_region_marshal_point`]
// * `0x00643fe0` [`a88_region_rect_store`]

// ---------------------------------------------------------------------------
// 0x00642037 - AI-script byte fetch
// ---------------------------------------------------------------------------

/// One of the two script banks selected by the state's `+0x18` flag.
///
/// Retail keeps two pointer pairs: `DAT_00fc8700`/`DAT_00fc8704` for flag
/// bit 0 clear and `DAT_00fc8708`/`DAT_00fc870c` when it is set.  The first
/// word of each pair is the bank base, the second one past its last byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88ScriptBank {
    /// First script byte (`DAT_00fc8700` / `DAT_00fc8708`).
    pub base: u32,
    /// One past the last script byte (`DAT_00fc8704` / `DAT_00fc870c`).
    pub end: u32,
}

/// Address of the primary pointer pair (`base`, `end`).
pub const A88_SCRIPT_PAIR_PRIMARY_ADDR: u32 = 0x00fc8700;

/// Address of the alternate pointer pair (`base`, `end`).
pub const A88_SCRIPT_PAIR_ALTERNATE_ADDR: u32 = 0x00fc8708;

/// Why the fetch failed.  Retail calls `0x00642696`, which raises
/// `"Invalid AI script"` (see [`a88_invalid_ai_script_words`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A88ScriptFault {
    /// `0x00642696`: `end <= cursor` or `end <= cursor + 1`.
    InvalidScript,
}

/// Result of one successful fetch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88ScriptRead {
    /// Byte at `base + cursor`.
    pub byte: u8,
    /// The stored cursor, `cursor + 1`.
    pub cursor: u32,
}

/// `0x00642037`: fetch the next script byte.
///
/// ```c
/// puVar18 = *(ushort **)(ESI + 0x18);           // bank flag
/// iVar15  = DAT_00fc8700;
/// if (((uint)puVar18 & 1) != 0) iVar15 = DAT_00fc8708;
/// puVar20 = *(ushort **)(ESI + 4);              // cursor
/// puVar22 = DAT_00fc8704;
/// if (((uint)puVar18 & 1) != 0) puVar22 = DAT_00fc870c;
/// if (puVar22 <= puVar20) return FUN_00642696();
/// if (puVar22 <= (ushort *)((int)puVar20 + 1)) return FUN_00642696();
/// *(undefined1 **)(ESI + 4) = (undefined1 *)((int)puVar20 + 1);
/// puVar28 = *(undefined1 *)((int)puVar20 + iVar15);
/// ```
///
/// Both comparisons are unsigned pointer comparisons on 32-bit values, so
/// the cursor arithmetic is reproduced with `u32` wraparound.  The second
/// test means the *last* byte of a bank can never be fetched; that is the
/// shipped check, not an artefact.  `memory` is the bank's byte image, so
/// the byte index is `cursor - bank.base`.
pub fn a88_script_read_byte(
    cursor: u32,
    flags: u32,
    primary: A88ScriptBank,
    alternate: A88ScriptBank,
    memory: &[u8],
) -> Result<A88ScriptRead, A88ScriptFault> {
    let bank = if flags & 1 != 0 { alternate } else { primary };
    if bank.end <= cursor || bank.end <= cursor.wrapping_add(1) {
        return Err(A88ScriptFault::InvalidScript);
    }
    let offset = cursor.wrapping_sub(bank.base) as usize;
    let byte = *memory.get(offset).ok_or(A88ScriptFault::InvalidScript)?;
    Ok(A88ScriptRead {
        byte,
        cursor: cursor.wrapping_add(1),
    })
}

/// `0x00642484`: commit the next instruction pointer.
///
/// The row is the common epilogue label
/// `*(undefined4 **)(ESI + 4) = in_EAX;` followed by the fetch loop above.
/// It stores a 32-bit pointer into the script state's cursor slot.
pub fn a88_script_store_cursor(cursor: &mut u32, next: u32) {
    *cursor = next;
}

// ---------------------------------------------------------------------------
// 0x00642687 / 0x00642696 - fatal diagnostics
// ---------------------------------------------------------------------------

/// `0x00642687`: add `AL` into the byte addressed by `EAX`, twice.
///
/// ```c
/// *in_EAX = *in_EAX + (char)in_EAX;
/// *in_EAX = *in_EAX + (char)in_EAX;
/// FUN_00952b40(&DAT_00d3b408);          // does not return
/// ```
///
/// The addend is the low byte of the *address* register, so the translated
/// value keeps it as an explicit argument.  The tail is a fatal error
/// (`FUN_00952b40`), reproduced by the caller, not by this arithmetic helper.
pub fn a88_add_al_twice(value: u8, address_low: u8) -> u8 {
    value.wrapping_add(address_low).wrapping_add(address_low)
}

/// `0x00642696`: the five words that spell `"Invalid AI script"`.
///
/// The row builds the message in place instead of pointing at `.rdata`:
///
/// ```c
/// *(u32 *)(EBP - 0x44) = 0x70695c67;
/// *(u32 *)(EBP - 0x44) = *(u32 *)(EBP - 0x44) + 0x5ff;
/// *(u32 *)(EBP - 0x44) = *(u32 *)(EBP - 0x44) ^ 0x1005;   // "crip"
/// *(u32 *)(EBP - 0x18) = *(u32 *)(EBP - 0x44);
/// *(u32 *)(EBP - 0x14) = 0x74;                            // "t\0\0\0"
/// *(u32 *)(EBP - 0x20) = 0x2064696c;                      // "lid "
/// *(u32 *)(EBP - 0x24) = 0x61766e49;                      // "Inva"
/// *(u32 *)(EBP - 0x44) = 0x732033ad;
/// *(u32 *)(EBP - 0x44) += 0xae9;
/// *(u32 *)(EBP - 0x44) += 0x1011;
/// *(u32 *)(EBP - 0x44) -= 0x566;                          // "AI s"
/// *(u32 *)(EBP - 0x1c) = *(u32 *)(EBP - 0x44);
/// FUN_00952b40(&DAT_00d3b408, EBP - 0x24, 0xa4046452);
/// ```
///
/// The returned array is in ascending memory order, exactly the 20 bytes
/// passed to the fatal-error reporter.  The message is 17 characters plus
/// the NUL at byte 17; bytes 18 and 19 are the padding of the last dword.
pub fn a88_invalid_ai_script_words() -> [u32; 5] {
    let mut word = 0x7069_5c67u32;
    word = word.wrapping_add(0x05ff);
    word ^= 0x1005;

    let tail = 0x7320_33adu32
        .wrapping_add(0x0ae9)
        .wrapping_add(0x1011)
        .wrapping_sub(0x0566);

    [
        0x6176_6e49, // "Inva"  (EBP - 0x24)
        0x2064_696c, // "lid "  (EBP - 0x20)
        tail,        // "AI s"  (EBP - 0x1c)
        word,        // "crip"  (EBP - 0x18)
        0x0000_0074, // "t\0\0\0" (EBP - 0x14)
    ]
}

/// The decoded message built by [`a88_invalid_ai_script_words`].
pub const A88_INVALID_AI_SCRIPT: &[u8] = b"Invalid AI script";

// ---------------------------------------------------------------------------
// 0x006427c6 - TSC compare / carry add
// ---------------------------------------------------------------------------

/// Compare constant in `0x006427c6` (`cmp eax, 0xfe98531c`).
pub const A88_TSC_COMPARE: u32 = 0xfe98_531c;

/// Addend in `0x006427c6` (`adc dword [mem], 0x33adbc45`).
pub const A88_TSC_ADDEND: u32 = 0x33ad_bc45;

/// `0x006427c6`: recoverable arithmetic of the row.
///
/// ```c
/// uVar1 = rdtsc();
/// bVar2 = (uint)uVar1 < 0xfe98531c;
/// *(int *)(ptr) = *(int *)(ptr) + 0x33adbc45 + (uint)bVar2;
/// ```
///
/// The `rdtsc` low word is compared with `0xfe98531c`; only the resulting
/// borrow/carry feeds the 32-bit add.  The rest of the row is decompiled as
/// bad-instruction data and is not modelled.  Returns the stored value and
/// the carry that was added.
pub fn a88_tsc_carry_add(value: u32, tsc_low: u32) -> (u32, bool) {
    let carry = tsc_low < A88_TSC_COMPARE;
    (
        value
            .wrapping_add(A88_TSC_ADDEND)
            .wrapping_add(carry as u32),
        carry,
    )
}

// ---------------------------------------------------------------------------
// 0x00642a00 / 0x00642a80 - spawn request and pool
// ---------------------------------------------------------------------------

/// The 0x34-byte request record built by `0x00642a00`.
///
/// Offsets are the record's own: `+0x00`/`+0x04` are not written by the row
/// (they are the caller stack slots it inherits), `+0x08`..`+0x0c` are
/// cleared, `+0x10` receives the unit type, `+0x24`/`+0x28`/`+0x2c` receive
/// the three arguments, and `+0x14`/`+0x18`/`+0x1c`/`+0x20`/`+0x30` are
/// cleared.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A88SpawnRequest {
    /// `+0x00`, inherited (not stored by the row).
    pub head_00: u32,
    /// `+0x04`, inherited (not stored by the row).
    pub head_04: u32,
    /// `+0x08`, cleared.
    pub clear_08: u32,
    /// `+0x0c`, cleared.
    pub clear_0c: u32,
    /// `+0x10`, the unit-type index (`FUN_006351b0`'s argument).
    pub unit_type: u32,
    /// `+0x14`, cleared.
    pub clear_14: u32,
    /// `+0x18`, cleared.
    pub clear_18: u32,
    /// `+0x1c`, cleared.
    pub clear_1c: u32,
    /// `+0x20`, cleared.
    pub clear_20: u32,
    /// `+0x24`, first argument.
    pub arg_24: u32,
    /// `+0x28`, second argument.
    pub arg_28: u32,
    /// `+0x2c`, third argument (the record's region tag source).
    pub arg_2c: u32,
    /// `+0x30`, cleared.
    pub clear_30: u32,
}

impl A88SpawnRequest {
    /// The record as the 13 dwords `0x00642a00` lays out on its stack.
    pub fn a88_words(&self) -> [u32; 13] {
        [
            self.head_00,
            self.head_04,
            self.clear_08,
            self.clear_0c,
            self.unit_type,
            self.clear_14,
            self.clear_18,
            self.clear_1c,
            self.clear_20,
            self.arg_24,
            self.arg_28,
            self.arg_2c,
            self.clear_30,
        ]
    }
}

/// Third argument `0x00642a00` passes to `0x00642bf0`.
pub const A88_SPAWN_REQUEST_SELECTOR: u32 = 2;

/// `0x00642a00`: assemble the spawn request.
///
/// ```c
/// iVar1 = FUN_006351b0(param_1);          // units.dat u16 at
///                                         // 0x00faa412 + type * 0x4e8
/// if (iVar1 == 0) return;
/// local_14 = param_2; local_10 = param_3; local_c = param_4;
/// local_30 = 0; local_2c = 0; local_24 = 0; local_1c = 0;
/// local_20 = 0; local_18 = 0; local_8 = 0; local_28 = param_1;
/// FUN_00642bf0(&local_38, iVar1, 2);
/// ```
///
/// `FUN_006351b0` is the units.dat lookup (a `u16` at a fixed stride); the
/// caller supplies its result as `resolved_unit`, so this function returns
/// `None` exactly when that lookup returns zero.  The request is emitted
/// only in that non-zero case.
pub fn a88_spawn_request(
    head: [u32; 2],
    unit_type: u32,
    resolved_unit: u16,
    arg_24: u32,
    arg_28: u32,
    arg_2c: u32,
) -> Option<A88SpawnRequest> {
    if resolved_unit == 0 {
        return None;
    }
    Some(A88SpawnRequest {
        head_00: head[0],
        head_04: head[1],
        clear_08: 0,
        clear_0c: 0,
        unit_type,
        clear_14: 0,
        clear_18: 0,
        clear_1c: 0,
        clear_20: 0,
        arg_24,
        arg_28,
        arg_2c,
        clear_30: 0,
    })
}

/// One node of the spawn list.  Offsets are the retail node's dwords.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A88SpawnNode {
    /// `+0x00`: free-list next while the node is free, list next once live.
    pub next: u32,
    /// `+0x04`: list previous.
    pub prev: u32,
    /// `+0x08`: location index (`FUN_00642a80`'s second argument).
    pub location: u32,
    /// `+0x0c`: cleared.
    pub reserved_0c: u32,
    /// `+0x10`: unit type.
    pub unit_type: u32,
    /// `+0x14..+0x20`: left, top, right, bottom.
    pub rect: [i32; 4],
    /// `+0x24`/`+0x28`: rect centre.
    pub center: [i32; 2],
    /// `+0x2c`: cleared.
    pub reserved_2c: u32,
    /// `+0x30`: flag bit 0 (low three bits are preserved from the old node).
    pub flags: u32,
}

/// The spawn list at `0x00fc9b64 + 0x1450` (free head) and `0x00fc9b68`
/// (live head).
///
/// Node indices are 1-based because the retail code uses a null pointer as
/// "no node"; index 0 is therefore the null link.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A88SpawnPool {
    /// Free-list head, `*(DAT_00fc9b64 + 0x1450)`.
    pub free_head: u32,
    /// Live-list head, `DAT_00fc9b68`.
    pub head: u32,
    /// Backing nodes.
    pub nodes: Vec<A88SpawnNode>,
}

impl A88SpawnPool {
    /// Build a pool of `capacity` free nodes chained through `+0x00`.
    pub fn a88_new(capacity: usize) -> Self {
        let mut nodes = vec![A88SpawnNode::default(); capacity];
        for (i, node) in nodes.iter_mut().enumerate() {
            node.next = if i + 1 < capacity { (i + 2) as u32 } else { 0 };
        }
        Self {
            free_head: if capacity == 0 { 0 } else { 1 },
            head: 0,
            nodes,
        }
    }

    /// Node by 1-based index.
    pub fn a88_node(&self, index: u32) -> Option<&A88SpawnNode> {
        if index == 0 {
            return None;
        }
        self.nodes.get((index - 1) as usize)
    }

    /// `0x00642a80`: take a free node, push it on the live list and fill it.
    ///
    /// ```c
    /// piVar1 = *(int **)(DAT_00fc9b64 + 0x1450);
    /// if (piVar1 == 0) return 0;
    /// iVar4 = *piVar1;
    /// *(DAT_00fc9b64 + 0x1450) = iVar4;
    /// if (iVar4 != 0) *(iVar4 + 4) = 0;
    /// piVar1[1] = 0;
    /// piVar1[0] = DAT_00fc9b68;
    /// if (DAT_00fc9b68 != 0) *(DAT_00fc9b68 + 4) = piVar1;
    /// DAT_00fc9b68 = piVar1;
    /// piVar1[2] = param_2;  piVar1[4] = param_1;  piVar1[3] = 0;
    /// piVar1[0xc] = (param_4 != 0) | piVar1[0xc] & 0xfffffffe;
    /// if (param_3 == 0) { piVar1[5..9] = 0; iVar4 = 0; }
    /// else {
    ///   iVar4 = param_3[1]; iVar2 = param_3[2]; iVar3 = param_3[3];
    ///   piVar1[5] = param_3[0]; piVar1[6] = iVar4;
    ///   piVar1[7] = iVar2;      piVar1[8] = iVar3;
    ///   piVar1[9] = (param_3[2] - param_3[0]) / 2 + param_3[0];
    ///   iVar4 = (param_3[3] - param_3[1]) / 2 + param_3[1];
    /// }
    /// piVar1[10] = iVar4;
    /// piVar1[0xc] = piVar1[0xc] & 7;
    /// piVar1[0xb] = 0;
    /// ```
    ///
    /// The division is C signed division (towards zero); `wrapping_*` keeps
    /// the 32-bit arithmetic defined for the extrema C leaves undefined.
    pub fn a88_alloc(
        &mut self,
        unit_type: u32,
        location: u32,
        rect: Option<[i32; 4]>,
        flag: bool,
    ) -> Option<u32> {
        let index = self.free_head;
        if index == 0 {
            return None;
        }
        let slot = (index - 1) as usize;
        let mut node = self.nodes[slot];
        let next_free = node.next;
        self.free_head = next_free;
        if next_free != 0 {
            self.nodes[(next_free - 1) as usize].prev = 0;
        }

        let old_head = self.head;
        node.prev = 0;
        node.next = old_head;
        if old_head != 0 {
            self.nodes[(old_head - 1) as usize].prev = index;
        }
        self.head = index;

        node.location = location;
        node.unit_type = unit_type;
        node.reserved_0c = 0;
        node.flags = ((flag as u32) | (node.flags & 0xffff_fffe)) & 7;
        match rect {
            None => {
                node.rect = [0; 4];
                node.center = [0; 2];
            }
            Some([left, top, right, bottom]) => {
                node.rect = [left, top, right, bottom];
                node.center = [
                    right.wrapping_sub(left) / 2 + left,
                    bottom.wrapping_sub(top) / 2 + top,
                ];
            }
        }
        node.reserved_2c = 0;
        self.nodes[slot] = node;
        Some(index)
    }
}

// ---------------------------------------------------------------------------
// 0x00642b50 / 0x00642bf0 - spawn placement
// ---------------------------------------------------------------------------

/// Upper bound of the location table used by `FUN_0076a4d0`/`FUN_0076a500`.
pub const A88_LOCATION_LIMIT: u32 = 0xfa;

/// Rect and word built by `0x00642b50` for one spawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88LocationSpawn {
    /// `{x - 1, y - 1, x + 1, y + 1}`.
    pub rect: [i32; 4],
    /// `(location << 3) | (old_flags & 7)`.
    pub flags_word: u32,
    /// Retail's final `FUN_00635720(...) == 0` test: the unit still needs
    /// the `FUN_00657260(unit, x, y)` placement call.
    pub needs_placement: bool,
}

/// `FUN_0076a4d0` / `FUN_0076a500`: location centre from the 0x30-byte table.
///
/// Both helpers return `0` for `location == 0` or `location >= 0xfa`, and
/// otherwise read the `i16` at `0x011ac918 + location * 0x30` (`x`) or
/// `0x011ac91a + location * 0x30` (`y`).  The table is supplied by the
/// caller; a missing entry reproduces the out-of-range zero.
pub fn a88_location_center(location: u32, table: &[(i16, i16)]) -> (i32, i32) {
    if location != 0 && location < A88_LOCATION_LIMIT {
        if let Some(&(x, y)) = table.get(location as usize) {
            return (x as i32, y as i32);
        }
    }
    (0, 0)
}

/// `0x00642b50`: turn a location index into the spawn rect and flag word.
///
/// ```c
/// iVar2 = FUN_0076a4d0(param_2);          // x
/// iVar3 = FUN_0076a500(param_2);          // y
/// local_14 = iVar2 - 1; local_10 = iVar3 - 1;
/// local_c  = iVar2 + 1; local_8  = iVar3 + 1;
/// iVar4 = FUN_00642a80(*(u32 *)(param_1 + 0x10), param_3, &local_14,
///                      *(u32 *)(param_1 + 0x30) & 1);
/// if (iVar4 != 0) {
///   *(u32 *)(iVar4 + 0x30) = param_2 << 3 | *(u32 *)(iVar4 + 0x30) & 7;
///   uVar1 = FUN_0075d720(iVar2, iVar3);   // zone lookup, out of batch
///   FUN_00656540(*(u32 *)(param_1 + 0x10), uVar1, 0);
///   if (FUN_00635720(*(u32 *)(param_1 + 0x10)) == 0)
///       FUN_00657260(*(u32 *)(param_1 + 0x10), iVar2, iVar3);
/// }
/// ```
///
/// The three callees are engine state (zone lookup, zone assignment and the
/// placement predicate); the caller supplies `needs_placement`, which is
/// the `FUN_00635720(...) == 0` result.
pub fn a88_spawn_at_location(
    location: u32,
    center: (i32, i32),
    old_flags: u32,
    needs_placement: bool,
) -> A88LocationSpawn {
    let (x, y) = center;
    A88LocationSpawn {
        rect: [
            x.wrapping_sub(1),
            y.wrapping_sub(1),
            x.wrapping_add(1),
            y.wrapping_add(1),
        ],
        flags_word: location.wrapping_shl(3) | (old_flags & 7),
        needs_placement,
    }
}

/// `0x00642bf0`: the per-unit-type size gate.
///
/// ```c
/// iVar1 = *(int *)(param_1 + 0x10);                     // unit type
/// if ((4999 < *(int *)((DAT_00f063cc ^ 0x10fae560) + iVar1 * 4)) &&
///     (2999 < *(int *)((DAT_00f063cc ^ 0x10fae560) + 0x30 + iVar1 * 4)) &&
///     (FUN_00635720(iVar1) == 0)) return;
/// iVar1 = FUN_0063b420(*(u32 *)(param_1 + 0x2c));       // byte at +0x1c
/// if (iVar1 != 0 && (iVar1 = FUN_0076a530(type, iVar1, param_3)) != 0)
///     FUN_00642b50(param_1, iVar1, param_2);
/// ```
///
/// The two arrays at `(DAT_00f063cc ^ 0x10fae560)` and `+0x30` are indexed
/// by unit type; callers supply the two `i32` entries as `width`/`height`.
/// `skip` is the early return (a large unit type that already owns a
/// placement).  `region_tag` is the `FUN_0063b420` byte, zero meaning "no
/// location spawn".
pub fn a88_spawn_gate(width: i32, height: i32, placed: bool, region_tag: u8) -> A88SpawnGate {
    A88SpawnGate {
        skip: 4999 < width && 2999 < height && !placed,
        region_tag,
    }
}

/// Result of [`a88_spawn_gate`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88SpawnGate {
    /// The `if (...) return;` early exit.
    pub skip: bool,
    /// `FUN_0063b420`'s byte, `*(u8 *)(*(u32 *)(record + 0x2c) + 0x1c)`.
    pub region_tag: u8,
}

// ---------------------------------------------------------------------------
// 0x00642e9b / 0x00643ca4 - MSVC `std::string` SSO constructor
// ---------------------------------------------------------------------------

/// Offset of the inline buffer in the 28-byte MSVC string object.
pub const A88_SSO_PTR_OFFSET: u32 = 0x0c;

/// Capacity dword of a 15-byte SSO string: `0x80000000 | 15`.
pub const A88_SSO_CAPACITY_TAG: u32 = 0x8000_000f;

/// Bytes of the inline buffer.
pub const A88_SSO_BUFFER_BYTES: usize = 16;

/// The MSVC `std::string` prologue recovered from the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A88SsoString {
    /// `+0x00`: pointer to the inline buffer (`this + 0x0c`).
    pub ptr_offset: u32,
    /// `+0x04`: size, zero.
    pub size: u32,
    /// `+0x08`: capacity/flags dword.
    pub capacity: u32,
    /// `+0x0c..+0x1b`: inline buffer, first byte zero.
    pub buffer: [u8; A88_SSO_BUFFER_BYTES],
}

impl Default for A88SsoString {
    fn default() -> Self {
        Self {
            ptr_offset: A88_SSO_PTR_OFFSET,
            size: 0,
            capacity: A88_SSO_CAPACITY_TAG,
            buffer: [0; A88_SSO_BUFFER_BYTES],
        }
    }
}

/// `0x00642e9b`: construct an empty SSO string.
///
/// ```c
/// extraout_ECX[1] = 0;
/// *extraout_ECX   = extraout_ECX + 3;     // dwords: this + 0x0c
/// extraout_ECX[2] = 0x8000000f;
/// *(u8 *)(extraout_ECX + 3) = 0;
/// ```
///
/// `+3` is in dword units, so the buffer begins at `+0x0c`; `0x8000000f`
/// is MSVC's "capacity 15, heap flag clear" SSO tag.
pub fn a88_sso_string_init() -> A88SsoString {
    A88SsoString::default()
}

/// `0x00643ca4`: the same constructor in a catch frame.
///
/// ```c
/// *(u32 *)(EBP - 0x44) = 0;
/// *(u32 *)(EBP - 0x48) = EBP - 0x3c;
/// *(u32 *)(EBP - 0x40) = 0x8000000f;
/// *(u8  *)(EBP - 0x3c) = 0;
/// ```
///
/// The pointer/store order differs (the buffer is `EBP - 0x3c` and the
/// object starts at `EBP - 0x48`) but the resulting object is identical.
pub fn a88_sso_string_init_catch_frame() -> A88SsoString {
    A88SsoString::default()
}

// ---------------------------------------------------------------------------
// 0x006433d7 - 33-bit rotate
// ---------------------------------------------------------------------------

/// `0x006433d7`: rotate a 32-bit value with the carry flag as bit 32.
///
/// ```c
/// uVar3 = *(u32 *)(*(int *)(EBP - 0x3c) + 8);
/// bVar5 = (byte)uVar3 & 0x1f;
/// uVar2 = *(u32 *)(EBX + 0x90664989);
/// *(u32 *)(EBX + 0x90664989) =
///     (u32)(CONCAT14(EBX < 0x1c0272d7, uVar2) >> bVar5) |
///     uVar2 << 0x21 - bVar5;
/// ```
///
/// This is `rcr dword [ebx+0x90664989], cl` with the pre-existing carry
/// (`EBX < 0x1c0272d7`) as bit 32.  The x86 count is masked to five bits
/// and a zero count is a no-op, so both are reproduced here rather than the
/// decompiler's `0x21 - count` expression (which is only valid for
/// non-zero counts).
pub fn a88_rotate_right_through_carry(value: u32, carry: bool, count: u32) -> (u32, bool) {
    let n = count & 31;
    if n == 0 {
        return (value, carry);
    }
    let ring = ((carry as u64) << 32) | value as u64;
    let rotated = ((ring >> n) | (ring << (33 - n))) & 0x1_ffff_ffff;
    ((rotated & 0xffff_ffff) as u32, rotated >> 32 != 0)
}

// ---------------------------------------------------------------------------
// 0x00643860 .. 0x00643aa0 - region list
// ---------------------------------------------------------------------------

/// One region-list node.  Only the fields the batch's rows touch are kept.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A88RegionNode {
    /// `+0x00`: next node.
    pub next: Option<u32>,
    /// `+0x04`: previous node.
    pub prev: Option<u32>,
    /// `+0x08`: first key (`0x00643f40` forwards it).
    pub key_08: u32,
    /// `+0x0c`: second key (`0x00643f40` forwards it).
    pub key_0c: u32,
    /// `+0x10`: destructor tag checked by `0x00643860`/`0x006439eb`.
    pub key_10: i32,
    /// `+0x34..+0x40`: the 16-byte rect.
    pub rect: [u32; 4],
    /// `+0x44`: "rect valid" byte.
    pub valid: u8,
    /// `+0x4d`: cleared by the unlink/reset paths.
    pub byte_4d: u8,
    /// `+0x4f`: cleared by the unlink/reset paths.
    pub byte_4f: u8,
    /// `+0x51`: cleared by the unlink/reset paths.
    pub byte_51: u8,
}

/// Clear the four bytes the region teardown rows zero
/// (`0x00643860` at `+0x51/+0x4f/+0x4d/+0x44` and `0x006439eb` with the
/// same four offsets).
fn a88_clear_region_bytes(node: &mut A88RegionNode) {
    node.byte_51 = 0;
    node.byte_4f = 0;
    node.byte_4d = 0;
    node.valid = 0;
}

/// The region list whose sentinel object holds `+0x00` head, `+0x04` tail
/// and `+0x08` count.
///
/// Retail stores both links in one sentinel object and compares the tail
/// pointer with the sentinel to detect an empty list; here the links are
/// `Option<u32>` indices into `nodes`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A88RegionList {
    /// `+0x00`.
    pub head: Option<u32>,
    /// `+0x04`.
    pub tail: Option<u32>,
    /// `+0x08`.
    pub count: u32,
    /// Nodes addressed by the two links.
    pub nodes: Vec<A88RegionNode>,
}

impl A88RegionList {
    /// An empty list (empty lists have no node to clear).
    pub fn a88_new() -> Self {
        Self::default()
    }

    /// Node by index.
    pub fn a88_node(&self, index: u32) -> Option<&A88RegionNode> {
        self.nodes.get(index as usize)
    }

    /// `0x00643860`: reset the sentinel and tear down the old tail node.
    ///
    /// ```c
    /// iVar1 = *(int *)(ECX + 4);        // old tail
    /// *(int *)(ECX + 4) = ECX;          // tail = sentinel
    /// *(int *)ECX = ECX;                // head = sentinel
    /// *(u32 *)(ECX + 8) = 0;            // count = 0
    /// if (iVar1 == ECX) return;         // was empty
    /// *(u8 *)(iVar1 + 0x51) = 0;
    /// *(u8 *)(iVar1 + 0x4f) = 0;
    /// *(u8 *)(iVar1 + 0x4d) = 0;
    /// *(u8 *)(iVar1 + 0x44) = 0;
    /// ```
    ///
    /// Returns the node that was torn down, if the list was not empty.  The
    /// row's two `FUN_0093ec70` calls are frame unwinds (one per destructor
    /// branch), not engine state.
    pub fn a88_reset(&mut self) -> Option<u32> {
        let old_tail = if self.head.is_none() { None } else { self.tail };
        self.head = None;
        self.tail = None;
        self.count = 0;
        if let Some(index) = old_tail {
            if let Some(node) = self.nodes.get_mut(index as usize) {
                a88_clear_region_bytes(node);
            }
        }
        old_tail
    }

    /// `0x00643910`: clear only the list head.
    ///
    /// The row is the single store `*in_ECX = 0;`; the tail and count are
    /// deliberately left untouched, matching the shipped code.
    pub fn a88_clear_head(&mut self) {
        self.head = None;
    }

    /// `0x006439eb`: unlink the tail node and clear its flag bytes.
    ///
    /// ```c
    /// piVar1 = *(int **)(EDI + 4);          // tail
    /// *(int *)(*piVar1 + 4) = piVar1[1];    // node->next->prev = node->prev
    /// *(int *)piVar1[1] = *piVar1;          // node->prev->next = node->next
    /// *(u8 *)((int)piVar1 + 0x51) = 0;
    /// *(u8 *)((int)piVar1 + 0x4f) = 0;
    /// *(u8 *)((int)piVar1 + 0x4d) = 0;
    /// *(u8 *)(piVar1 + 0x11) = 0;           // +0x44
    /// iVar2 = piVar1[4];                    // +0x10
    /// if (-1 < iVar2) FUN_0093ec70(...);    // element destructor
    /// ```
    ///
    /// The `+0x00`/`+0x04` stores target the sentinel when the removed node
    /// is the first/last, which in this model is the list head/tail slot.
    pub fn a88_unlink_tail(&mut self) -> A88RegionUnlink {
        let Some(index) = self.tail else {
            return A88RegionUnlink {
                removed: None,
                destructor_ran: false,
            };
        };
        let mut node = self.nodes[index as usize].clone();
        match node.next {
            Some(next) => self.nodes[next as usize].prev = node.prev,
            None => self.tail = node.prev,
        }
        match node.prev {
            Some(prev) => self.nodes[prev as usize].next = node.next,
            None => self.head = node.next,
        }
        a88_clear_region_bytes(&mut node);
        let destructor_ran = node.key_10 >= 0;
        self.nodes[index as usize] = node;
        A88RegionUnlink {
            removed: Some(index),
            destructor_ran,
        }
    }
}

/// Result of [`A88RegionList::a88_unlink_tail`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88RegionUnlink {
    /// The unlinked node, `None` for an empty list.
    pub removed: Option<u32>,
    /// The `-1 < node->+0x10` destructor branch.
    pub destructor_ran: bool,
}

/// `0x00643aa0`: the region-list non-null gate.
///
/// ```c
/// if (DAT_00fc9b70 == 0) FUN_00923df0(0xc);
/// ```
///
/// Returns `true` when the list exists; a null list takes the abort path
/// with code `0xc`.
pub fn a88_region_list_present(pointer: u32) -> bool {
    pointer != 0
}

/// A rect snapshot produced by `0x00643b10`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88RegionSnapshot {
    /// The node's `+0x44` byte.
    pub valid: u8,
    /// The caller's rect when the node is invalid, else the node's rect.
    pub rect: [u32; 4],
}

/// `0x00643b10`: copy the node's rect out when `+0x44` is set.
///
/// ```c
/// iVar2 = *in_ECX;
/// cVar1 = *(char *)(iVar2 + 0x44);
/// *(char *)(param_1 + 4) = cVar1;          // byte at out + 0x10
/// if (cVar1 != '\0') {
///     uVar3 = *(u32 *)(iVar2 + 0x38);  uVar4 = *(u32 *)(iVar2 + 0x3c);
///     uVar5 = *(u32 *)(iVar2 + 0x40);
///     *param_1 = *(u32 *)(iVar2 + 0x34);
///     param_1[1] = uVar3;  param_1[2] = uVar4;  param_1[3] = uVar5;
/// }
/// ```
///
/// `out_rect` is the destination buffer as the caller left it: it is only
/// overwritten when the node's valid byte is non-zero.
pub fn a88_region_snapshot(node: &A88RegionNode, out_rect: [u32; 4]) -> A88RegionSnapshot {
    A88RegionSnapshot {
        valid: node.valid,
        rect: if node.valid != 0 { node.rect } else { out_rect },
    }
}

/// Scratch record built by `0x00643b30`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A88RegionScratch {
    /// `param_1[0..3]`, zeroed by the row.
    pub cleared: [u32; 3],
    /// The two dwords read at `ECX + 0x1c`.
    pub probe: [u32; 2],
    /// The `std::string` constructed at `EBP - 0x30`.
    pub string: A88SsoString,
}

/// `0x00643b30`: zero the output triple, probe `ECX + 0x1c` and set up the
/// scratch string.
///
/// ```c
/// *param_1 = 0; param_1[1] = 0; param_1[2] = 0;
/// FUN_0074f3f0(local_30); FUN_0074fb10(local_68, ...);
/// FUN_0074f340(local_30); FUN_006025e0(...);
/// puVar3 = (u32 *)FUN_00715e10(local_68, ECX + 0x1c);   // two dwords
/// FUN_0067dcc0(local_30, *puVar3, puVar3[1]);
/// FUN_00538810(0);                                      // does not return
/// ```
///
/// The two dwords at `ECX + 0x1c` are the only engine input the row reads;
/// everything else is MFC string construction and the terminal report.
pub fn a88_region_scratch(probe: [u32; 2]) -> A88RegionScratch {
    A88RegionScratch {
        cleared: [0; 3],
        probe,
        string: a88_sso_string_init(),
    }
}

/// `0x00643f40`: forward the point and the node's two keys.
///
/// ```c
/// FUN_0067d6b0(*param_1, param_1[1], *(*in_ECX + 8), *(*in_ECX + 0xc));
/// ```
///
/// The callee is out of batch; the row's whole contribution is the argument
/// order, returned here as `[x, y, key_08, key_0c]`.
pub fn a88_region_marshal_point(point: [u32; 2], node: &A88RegionNode) -> [u32; 4] {
    [point[0], point[1], node.key_08, node.key_0c]
}

/// Result of [`a88_region_rect_store`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A88RegionRectStore {
    /// The output buffer after the row: the old node rect on swap, else the
    /// incoming rect unchanged.
    pub buffer: [u32; 4],
    /// `true` when the node already had a valid rect and the two buffers
    /// were exchanged; `false` when the rect was installed.
    pub swapped: bool,
    /// The node's `+0x44` byte after the row (always set).
    pub valid: u8,
}

/// `0x00643fe0`: store a 16-byte rect, or swap it with the existing one.
///
/// ```c
/// puVar3 = FUN_00715a40(local_2c, param_1, param_2);   // 16-byte rect
/// local_1c = *puVar3; uStack_18 = puVar3[1];
/// uStack_14 = puVar3[2]; uStack_10 = puVar3[3];
/// puVar1 = (u32 *)(node + 0x34);
/// if (*(char *)(node + 0x44) != '\0') {
///     iVar4 = 0x10;
///     do { uVar2 = *(u8 *)((int)&local_1c + -(int)puVar1 + (int)puVar3);
///          (local_2c + -(int)puVar1 + 0xf)[(int)puVar3 + 1] = *(u8 *)puVar3;
///          *(u8 *)puVar3 = uVar2; puVar3++; } while (--iVar4);
///     FUN_00c86b6e();                                  // does not return
/// }
/// *puVar1 = *puVar3;
/// *(u32 *)(node + 0x38) = uStack_18;
/// *(u32 *)(node + 0x3c) = uStack_14;
/// *(u32 *)(node + 0x40) = uStack_10;
/// *(u8  *)(node + 0x44) = 1;
/// ```
///
/// The 16-byte byte-wise exchange is equivalent to swapping the two
/// `[u32; 4]` values.
pub fn a88_region_rect_store(node: &mut A88RegionNode, incoming: [u32; 4]) -> A88RegionRectStore {
    if node.valid != 0 {
        let old = node.rect;
        node.rect = incoming;
        A88RegionRectStore {
            buffer: old,
            swapped: true,
            valid: node.valid,
        }
    } else {
        node.rect = incoming;
        node.valid = 1;
        A88RegionRectStore {
            buffer: incoming,
            swapped: false,
            valid: 1,
        }
    }
}

#[cfg(test)]
mod tests_agent88 {
    use super::*;

    // -- 0x00642037 ---------------------------------------------------------

    #[test]
    fn script_fetch_uses_the_selected_bank_and_advances_one_byte() {
        let primary = A88ScriptBank {
            base: 0x1000,
            end: 0x1004,
        };
        let alternate = A88ScriptBank {
            base: 0x2000,
            end: 0x2002,
        };
        let primary_bytes = b"abcd";
        let alternate_bytes = b"xy";

        assert_eq!(
            a88_script_read_byte(0x1000, 0, primary, alternate, primary_bytes),
            Ok(A88ScriptRead {
                byte: b'a',
                cursor: 0x1001
            })
        );
        assert_eq!(
            a88_script_read_byte(0x1002, 0, primary, alternate, primary_bytes),
            Ok(A88ScriptRead {
                byte: b'c',
                cursor: 0x1003
            })
        );
        // Flag bit 0 selects the second pointer pair.
        assert_eq!(
            a88_script_read_byte(0x2000, 1, primary, alternate, alternate_bytes),
            Ok(A88ScriptRead {
                byte: b'x',
                cursor: 0x2001
            })
        );
        // The last byte is rejected by `end <= cursor + 1`.
        assert_eq!(
            a88_script_read_byte(0x1003, 0, primary, alternate, primary_bytes),
            Err(A88ScriptFault::InvalidScript)
        );
        assert_eq!(
            a88_script_read_byte(0x2001, 1, primary, alternate, alternate_bytes),
            Err(A88ScriptFault::InvalidScript)
        );
        // An empty bank fails the first test.
        assert_eq!(
            a88_script_read_byte(0x1004, 0, primary, alternate, primary_bytes),
            Err(A88ScriptFault::InvalidScript)
        );
    }

    #[test]
    fn script_cursor_store_replaces_the_slot() {
        let mut cursor = 0x1000u32;
        a88_script_store_cursor(&mut cursor, 0x2222_3333);
        assert_eq!(cursor, 0x2222_3333);
    }

    // -- 0x00642687 ---------------------------------------------------------

    #[test]
    fn al_is_added_twice_and_wraps() {
        assert_eq!(a88_add_al_twice(0x10, 0xff), 0x0e);
        assert_eq!(a88_add_al_twice(0x00, 0x00), 0x00);
        assert_eq!(a88_add_al_twice(0x80, 0x80), 0x80);
        assert_eq!(a88_add_al_twice(0xff, 0x01), 0x01);
    }

    // -- 0x00642696 ---------------------------------------------------------

    #[test]
    fn invalid_ai_script_words_spell_the_message() {
        let words = a88_invalid_ai_script_words();
        let mut bytes = [0u8; 20];
        for (i, word) in words.iter().enumerate() {
            bytes[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        assert_eq!(&bytes[..17], A88_INVALID_AI_SCRIPT);
        assert_eq!(A88_INVALID_AI_SCRIPT.len(), 17);
        assert_eq!(bytes[17], 0);
        assert_eq!(&bytes[18..], &[0, 0]);
        assert_eq!(words[0], 0x6176_6e49);
        assert_eq!(words[1], 0x2064_696c);
        assert_eq!(words[2], 0x7320_4941);
    }

    // -- 0x006427c6 ---------------------------------------------------------

    #[test]
    fn tsc_carry_add_uses_the_compare_borrow() {
        assert_eq!(a88_tsc_carry_add(0, 0), (A88_TSC_ADDEND + 1, true));
        assert_eq!(
            a88_tsc_carry_add(0, A88_TSC_COMPARE),
            (A88_TSC_ADDEND, false)
        );
        assert_eq!(
            a88_tsc_carry_add(0xffff_ffff, 0xffff_ffff),
            (0xffff_ffffu32.wrapping_add(A88_TSC_ADDEND), false)
        );
        assert_eq!(A88_TSC_COMPARE, 0xfe98_531c);
        assert_eq!(A88_TSC_ADDEND, 0x33ad_bc45);
    }

    // -- 0x00642a00 ---------------------------------------------------------

    #[test]
    fn spawn_request_layout_and_gate() {
        let request = a88_spawn_request([0xaaaa, 0xbbbb], 7, 1, 2, 3, 4).unwrap();
        assert_eq!(
            request.a88_words(),
            [0xaaaa, 0xbbbb, 0, 0, 7, 0, 0, 0, 0, 2, 3, 4, 0]
        );
        assert_eq!(request.unit_type, 7);
        assert_eq!(request.arg_24, 2);
        assert_eq!(request.arg_28, 3);
        assert_eq!(request.arg_2c, 4);
        assert_eq!(A88_SPAWN_REQUEST_SELECTOR, 2);

        // `FUN_006351b0` returning zero skips the request entirely.
        assert_eq!(a88_spawn_request([1, 2], 7, 0, 2, 3, 4), None);
    }

    // -- 0x00642a80 ---------------------------------------------------------

    #[test]
    fn pool_alloc_pops_free_list_and_pushes_live_list() {
        let mut pool = A88SpawnPool::a88_new(3);
        assert_eq!(pool.free_head, 1);
        assert_eq!(pool.head, 0);
        assert_eq!(pool.nodes[0].next, 2);
        assert_eq!(pool.nodes[2].next, 0);

        let first = pool.a88_alloc(7, 4, Some([10, 20, 30, 40]), true).unwrap();
        assert_eq!(first, 1);
        assert_eq!(pool.head, 1);
        assert_eq!(pool.free_head, 2);
        let node = pool.a88_node(first).unwrap();
        assert_eq!(node.next, 0);
        assert_eq!(node.prev, 0);
        assert_eq!(node.location, 4);
        assert_eq!(node.reserved_0c, 0);
        assert_eq!(node.unit_type, 7);
        assert_eq!(node.rect, [10, 20, 30, 40]);
        assert_eq!(node.center, [20, 30]);
        assert_eq!(node.reserved_2c, 0);
        assert_eq!(node.flags, 1);

        let second = pool.a88_alloc(9, 5, None, false).unwrap();
        assert_eq!(second, 2);
        assert_eq!(pool.head, 2);
        assert_eq!(pool.nodes[0].prev, 2);
        let node = pool.a88_node(second).unwrap();
        assert_eq!(node.next, 1);
        assert_eq!(node.rect, [0; 4]);
        assert_eq!(node.center, [0; 2]);
        assert_eq!(node.flags, 0);

        assert_eq!(pool.a88_alloc(1, 1, None, false), Some(3));
        assert_eq!(pool.a88_alloc(1, 1, None, false), None);

        // Negative centres use C's truncate-toward-zero division.
        let mut pool = A88SpawnPool::a88_new(1);
        let index = pool.a88_alloc(1, 1, Some([-3, -3, 0, 0]), false).unwrap();
        let node = pool.a88_node(index).unwrap();
        // (0 - -3) / 2 + -3 = 1 - 3.
        assert_eq!(node.center, [-2, -2]);
    }

    // -- 0x00642b50 ---------------------------------------------------------

    #[test]
    fn location_center_and_spawn_rect() {
        let table = [(-1i16, -1i16), (100, 200), (300, 400)];
        assert_eq!(a88_location_center(0, &table), (0, 0));
        assert_eq!(a88_location_center(1, &table), (100, 200));
        assert_eq!(a88_location_center(2, &table), (300, 400));
        assert_eq!(a88_location_center(3, &table), (0, 0));
        assert_eq!(a88_location_center(0xfa, &table), (0, 0));

        let spawn = a88_spawn_at_location(4, (100, 200), 0b1010, true);
        assert_eq!(spawn.rect, [99, 199, 101, 201]);
        assert_eq!(spawn.flags_word, 4 << 3 | 2);
        assert!(spawn.needs_placement);
        // The low three bits of the node flags are preserved.
        assert_eq!(
            a88_spawn_at_location(1, (0, 0), 0xffff_ffff, false).flags_word,
            8 | 7
        );
    }

    // -- 0x00642bf0 ---------------------------------------------------------

    #[test]
    fn spawn_gate_thresholds_are_4999_by_2999() {
        assert!(!a88_spawn_gate(4999, 2999, false, 0).skip);
        assert!(!a88_spawn_gate(5000, 2999, false, 0).skip);
        assert!(!a88_spawn_gate(4999, 3000, false, 0).skip);
        // Both bounds exceeded and no placement yet: early return.
        assert!(a88_spawn_gate(5000, 3000, false, 0).skip);
        // A placed unit type takes the location path instead.
        assert!(!a88_spawn_gate(5000, 3000, true, 0).skip);
        // The region tag is passed through even when the gate does not skip.
        assert_eq!(a88_spawn_gate(0, 0, false, 7).region_tag, 7);
    }

    // -- 0x00642e9b / 0x00643ca4 -------------------------------------------

    #[test]
    fn sso_string_constructor_layout() {
        let string = a88_sso_string_init();
        assert_eq!(string.ptr_offset, 0x0c);
        assert_eq!(string.size, 0);
        assert_eq!(string.capacity, 0x8000_000f);
        assert_eq!(string.capacity & 0x8000_0000, 0x8000_0000);
        assert_eq!(string.capacity & 0x7fff_ffff, 15);
        assert_eq!(string.buffer, [0u8; 16]);
        assert_eq!(a88_sso_string_init_catch_frame(), string);
    }

    // -- 0x006433d7 ---------------------------------------------------------

    #[test]
    fn rotate_right_through_carry_is_a_33_bit_ring() {
        assert_eq!(
            a88_rotate_right_through_carry(0, true, 1),
            (0x8000_0000, false)
        );
        assert_eq!(a88_rotate_right_through_carry(1, false, 1), (0, true));
        assert_eq!(
            a88_rotate_right_through_carry(0x8000_0000, false, 1),
            (0x4000_0000, false)
        );
        assert_eq!(
            a88_rotate_right_through_carry(0xffff_ffff, true, 1),
            (0xffff_ffff, true)
        );
        // A zero (and 32, after masking) count is a no-op.
        assert_eq!(
            a88_rotate_right_through_carry(0x1234, true, 0),
            (0x1234, true)
        );
        assert_eq!(
            a88_rotate_right_through_carry(0x1234, true, 32),
            (0x1234, true)
        );
        // Rotating 33 times is the identity.
        let (value, carry) = (0xdead_beefu32, true);
        let mut ring = (value, carry);
        for _ in 0..33 {
            ring = a88_rotate_right_through_carry(ring.0, ring.1, 1);
        }
        assert_eq!(ring, (value, carry));
    }

    // -- 0x00643b10 / 0x00643b30 / 0x00643f40 / 0x00643fe0 ----------------

    #[test]
    fn region_snapshot_only_copies_a_valid_rect() {
        let mut node = A88RegionNode {
            rect: [11, 22, 33, 44],
            ..A88RegionNode::default()
        };
        assert_eq!(
            a88_region_snapshot(&node, [1, 2, 3, 4]),
            A88RegionSnapshot {
                valid: 0,
                rect: [1, 2, 3, 4]
            }
        );
        node.valid = 1;
        assert_eq!(
            a88_region_snapshot(&node, [1, 2, 3, 4]),
            A88RegionSnapshot {
                valid: 1,
                rect: [11, 22, 33, 44]
            }
        );
    }

    #[test]
    fn region_scratch_zeroes_three_words_and_keeps_the_probe() {
        let scratch = a88_region_scratch([0x10, 0x20]);
        assert_eq!(scratch.cleared, [0; 3]);
        assert_eq!(scratch.probe, [0x10, 0x20]);
        assert_eq!(scratch.string, a88_sso_string_init());
    }

    #[test]
    fn region_marshal_forwards_point_then_keys() {
        let node = A88RegionNode {
            key_08: 0xaa,
            key_0c: 0xbb,
            ..A88RegionNode::default()
        };
        assert_eq!(
            a88_region_marshal_point([0x11, 0x22], &node),
            [0x11, 0x22, 0xaa, 0xbb]
        );
    }

    #[test]
    fn region_rect_store_swaps_or_installs() {
        let mut node = A88RegionNode::default();
        let stored = a88_region_rect_store(&mut node, [1, 2, 3, 4]);
        assert!(!stored.swapped);
        assert_eq!(stored.buffer, [1, 2, 3, 4]);
        assert_eq!(stored.valid, 1);
        assert_eq!(node.rect, [1, 2, 3, 4]);
        assert_eq!(node.valid, 1);

        let stored = a88_region_rect_store(&mut node, [5, 6, 7, 8]);
        assert!(stored.swapped);
        assert_eq!(stored.buffer, [1, 2, 3, 4]);
        assert_eq!(node.rect, [5, 6, 7, 8]);
        assert_eq!(node.valid, 1);
    }

    // -- 0x00643860 / 0x00643910 / 0x006439eb / 0x00643aa0 ----------------

    fn region_node(index: u32) -> A88RegionNode {
        A88RegionNode {
            next: None,
            prev: None,
            key_08: index,
            key_0c: index + 1,
            key_10: index as i32,
            rect: [index; 4],
            valid: 1,
            byte_4d: 1,
            byte_4f: 1,
            byte_51: 1,
        }
    }

    #[test]
    fn region_list_reset_tears_down_the_old_tail() {
        let mut list = A88RegionList::a88_new();
        assert_eq!(list.a88_reset(), None);

        list.nodes = vec![region_node(0), region_node(1), region_node(2)];
        list.head = Some(0);
        list.tail = Some(2);
        list.count = 2;
        assert_eq!(list.a88_reset(), Some(2));
        assert_eq!(list.head, None);
        assert_eq!(list.tail, None);
        assert_eq!(list.count, 0);
        assert_eq!(list.nodes[2].valid, 0);
        assert_eq!(list.nodes[2].byte_4d, 0);
        assert_eq!(list.nodes[2].byte_4f, 0);
        assert_eq!(list.nodes[2].byte_51, 0);
        // The body of the torn-down node is untouched.
        assert_eq!(list.nodes[2].rect, [2; 4]);
    }

    #[test]
    fn region_clear_head_leaves_tail_and_count() {
        let mut list = A88RegionList::a88_new();
        list.head = Some(0);
        list.tail = Some(1);
        list.count = 2;
        list.a88_clear_head();
        assert_eq!(list.head, None);
        assert_eq!(list.tail, Some(1));
        assert_eq!(list.count, 2);
    }

    #[test]
    fn region_unlink_tail_rewires_neighbours_and_clears_bytes() {
        let mut list = A88RegionList::a88_new();
        list.nodes = vec![region_node(0), region_node(1), region_node(2)];
        list.nodes[0].next = Some(1);
        list.nodes[0].prev = None;
        list.nodes[1].next = Some(2);
        list.nodes[1].prev = Some(0);
        list.nodes[2].next = None;
        list.nodes[2].prev = Some(1);
        list.head = Some(0);
        list.tail = Some(2);

        let unlink = list.a88_unlink_tail();
        assert_eq!(
            unlink,
            A88RegionUnlink {
                removed: Some(2),
                destructor_ran: true
            }
        );
        assert_eq!(list.head, Some(0));
        assert_eq!(list.tail, Some(1));
        assert_eq!(list.nodes[1].next, None);
        assert_eq!(list.nodes[2].valid, 0);

        // Unlinking the last remaining node leaves both links empty.
        let mut list = A88RegionList::a88_new();
        list.nodes = vec![region_node(0), region_node(1)];
        list.head = Some(1);
        list.tail = Some(1);
        assert_eq!(list.a88_unlink_tail().removed, Some(1));
        assert_eq!(list.head, None);
        assert_eq!(list.tail, None);
        assert_eq!(list.a88_unlink_tail().removed, None);
    }

    #[test]
    fn region_null_gate() {
        assert!(!a88_region_list_present(0));
        assert!(a88_region_list_present(1));
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// The 14 rows not translated above, grouped by why:
//
// * `FUN_00642203`, `FUN_0064221e`, `FUN_00642236`, `FUN_00642253`, `FUN_00642291`,
//   `FUN_006423e9`: entries into the shared AI-script dispatch.  Each body is
//   the same `switch (*cursor++)` loop entered at a different case label and
//   addresses its state through the caller's `ESI`/`EDI`/`EBP` (`in_AX`,
//   `unaff_ESI`, `unaff_EDI`, `puRam00fc9b6c`).  The only closed operation
//   they share is the cursor fetch translated at `0x00642037`; the case
//   bodies themselves call out-of-batch engine helpers with caller-frame
//   operands.
// * `FUN_00642833`: the decompiler reports overlapping instructions
//   (`0x0064292e` over `0x0064292d`), an unrecovered jump table and a lost
//   stack base; every store is `CONCAT`-chained bad-instruction data.
// * `FUN_00642c60`: stack pointer lost, `swi(0x80)` and `rdtsc` garbage; the
//   only legible call is `FUN_006439eb` (translated at its own row).
// * `FUN_006430eb`: `__break`, `halt_baddata` and `rdtsc` garbage around one
//   `FUN_006439eb` call; no recoverable state.
// * `FUN_00643459`: privileged `out`/`swi(3)` instructions and
//   bad-instruction data; no engine arithmetic.
// * `FUN_006434b0`, `FUN_00643519`: catch funclets that restore
//   `ExceptionList` and enter `FUN_00c86b6e`; the only arithmetic is a
//   `rdtsc`-derived pointer in unreachable code.
// * `FUN_00643749`: three stores against the caller's frame
//   (`*(u8 *)(ESI + *EDI) = 0`, `EDI[1] = ESI`, `*piVar1 - *EDI`), where
//   `ESI` is used as a byte offset and `EDI`/`piVar1` are caller nodes; the
//   row does not define a self-contained operation.
// * `FUN_00643920`: SEH scope-table teardown; its only constant is the `-1`
//   scope index pushed onto the stack.

// ===========================================================================
// Merged from src/wave13/agent03.rs
// ===========================================================================
// Wave 13 translation batch 03: player-command dispatch, region queries and
// per-player counter/relation updates recovered from
// `0x00660000..=0x00661fff`.
//
// This window sits on the client/AI boundary. A command record is passed to
// several routines that either dispatch an AI-script handler opcode
// (`0x0e..=0x1a`) or update one of the per-player counter tables through the
// obfuscated image base (`DAT_00f063cc ^ 0x10fae560`). The translations below
// keep only the value-level decisions: the table bases and record strides are
// documented as recovered layout, while the caller supplies the records.
//
// ## Recovered layout
//
// | offset | field |
// |--------|-------|
// | `+0x00` | 1-based player/owner slot (`u8`) |
// | `+0x04` | source word (`u32`) |
// | `+0x10` | player selector or AI-script opcode (`u32`) |
// | `+0x14` | region/script id or bitfield value (`u32`) |
// | `+0x18` | target word (`u16`) |
// | `+0x1a` | command id used by `0x006605d0` (`u8`) |
// | `+0x1b` | mode/repeat byte (`u8`) |
//
// Per-player area records live at `0xeb70 + i * 0x14`: `u16` words at
// `+0x00`, `+0x04`, `+0x08`, `+0x0c` and a marker word at `+0x12`. Region
// ids are 1-based for the query rows (`0xeb70 + (id - 1) * 0x14`), and the
// region rectangle is read as four `i32` at `+0x00..+0x0c`, so its centre is
// `(left + right) / 2, (top + bottom) / 2`.
//
// The relation matrix has `0xc`-byte rows based at `0x131b * 0xc` (`0xe544`).
// The command matrix has `0xc`-dword rows based at `0xb274` and accepts
// `0xe4` command rows. The pending-command record starts at `0xe61c`.
//
// Every routine below returns a source-level decision. The batch/query rows
// return the constant `1` (or fall into a non-returning consistency trap);
// the counter dispatchers `0x00661450`, `0x00661620`, `0x0066175a` and
// `0x00661990` return zero on every recovered path. Each routine documents
// which convention it models.
//
// ## Skipped rows
//
// * `FUN_00660234` - register-only `*in_EAX += (char)in_EAX` fragment; no
//   recoverable arguments.
// * `FUN_00660313` - `unaff_EBP` twin of `0x00660310`; the body is the same
//   routine viewed through the caller frame, so it is not translated twice.
// * `FUN_00660aab` - inline fragment over `in_*`/`unaff_*` registers and flags;
//   only the rectangle-clamp tail is visible.
// * `FUN_00660d7b` - `*(int *)(in_EAX + 1) += 1` increment fragment.
// * `FUN_00660d9e` - `in_EAX`/`unaff_ESI`/`unaff_BL` byte-add fragment.
// * `FUN_00660e5d` - `unaff_EBX`/`in_AL` fragment ending in a non-returning
//   runtime trap.
// * `FUN_006616dd` - register-only inline copy of the selector-`1` path of
//   `0x00661620`; its arguments arrive in `EAX`/`ECX`/`EDX`/`SI`.
// * `FUN_006619b9` - `in_EAX`/`unaff_ESI` stack-arithmetic fragment.
// * `FUN_00661db0` - callback whose body is a recursion through four
//   unrecovered calls plus non-returning tails.
// * `FUN_00661f10` - callback reduced to two opaque calls and a non-returning
//   tail; no value survives.
// * `FUN_00661f7d` - `in_EAX` indexes a global byte table at `DAT_00f92388`, then
//   the body calls opaque helpers and traps.

/// Number of per-player slots accepted by the counter routines (`7 < player`
/// is the exclusive guard used by `0x00661450` and `0x00661620`).
pub const W13A03_PLAYER_COUNT: usize = 8;

/// Stride of one per-player area record.
pub const W13A03_AREA_STRIDE: u32 = 0x14;

/// Base offset of the per-player area-record table.
pub const W13A03_AREA_BASE: u32 = 0xeb70;

/// Offset of the relation-matrix chunk; multiplied by the `0xc` row stride it
/// gives `0xe544`.
pub const W13A03_RELATION_CHUNK: u32 = 0x131b;

/// Byte stride of one relation-matrix row.
pub const W13A03_RELATION_STRIDE: usize = 0xc;

/// Number of relation-matrix rows and columns.
pub const W13A03_RELATION_SLOTS: usize = 0xc;

/// Image offset of the pending-command record (`0xe61c..=0xe627`).
pub const W13A03_PENDING_COMMAND_BASE: u32 = 0xe61c;

/// Image address returned by `0x00661410`.
pub const W13A03_COMMAND_TABLE_BASE: usize = 0x00fdf900;

/// Number of dword rows in the `0xb274` command matrix.
pub const W13A03_COMMAND_MATRIX_ROWS: usize = 0xe4;

/// Dwords per command-matrix row.
pub const W13A03_COMMAND_MATRIX_STRIDE: usize = 0xc;

/// One command record consumed by the dispatch rows in this window.
///
/// Only the fields touched by the translated routines are modelled. The
/// record is the first argument of `0x00660070`, `0x00660280`, `0x00660310`,
/// `0x00660440`, `0x00660580`, `0x006605d0`, `0x00660b90` and `0x00660d38`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03CommandRequest {
    /// `+0x00`: 1-based player/owner slot. Zero means "no player".
    pub player: u8,
    /// `+0x04`: source word copied into the pending command by
    /// `0x006605d0`.
    pub source: u32,
    /// `+0x10`: player selector (`< 8`) or AI-script handler opcode
    /// (`0x0e..=0x1a`).
    pub command: u32,
    /// `+0x14`: region/script id, bitfield value or counter amount.
    pub value: u32,
    /// `+0x18`: target word.
    pub target: u16,
    /// `+0x1a`: command byte inspected by `0x006605d0`.
    pub action: u8,
    /// `+0x1b`: mode/repeat byte.
    pub mode: u8,
}

/// One per-player area record (`u16` view of the `0xeb70` table).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03PlayerArea {
    /// `+0x00`: first extent word.
    pub left: u16,
    /// `+0x04`: second extent word.
    pub top: u16,
    /// `+0x08`: third extent word.
    pub right: u16,
    /// `+0x0c`: fourth extent word.
    pub bottom: u16,
    /// `+0x12`: marker/filter word.
    pub marker: u16,
}

/// One region rectangle (`i32` view used by the 1-based region queries).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03RegionRect {
    /// `+0x00`: left edge.
    pub left: i32,
    /// `+0x04`: top edge.
    pub top: i32,
    /// `+0x08`: right edge.
    pub right: i32,
    /// `+0x0c`: bottom edge.
    pub bottom: i32,
}

/// AI-script handler selected by an opcode in `0x0e..=0x1a`.
///
/// The variant documentation names the retail callee used by the dispatch
/// rows. The shared `0x12..=0x15` arm also serves `0x13`, `0x14` and `0x15`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03ScriptHandler {
    /// Opcode `0x0e`: `FUN_00664b60`.
    Script0e,
    /// Opcode `0x0f`: `FUN_00664700`.
    Script0f,
    /// Opcode `0x10`: `FUN_006650a0`.
    Script10,
    /// Opcode `0x11`: `FUN_00664610`.
    Script11,
    /// Opcodes `0x12..=0x15`: `FUN_00664ce0`.
    Script12To15,
    /// Opcode `0x1a`: `FUN_00664f50`.
    Script1a,
}

/// Source-level outcome shared by the dispatch rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03Outcome<T> {
    /// The row tail-calls the named AI-script handler.
    Delegate(W13a03ScriptHandler),
    /// The row updated a caller-owned table and reports the applied case.
    Applied(T),
    /// `0x00661450`'s `DAT_01165f58 != 0` branch: the legacy hook
    /// `FUN_006ae540` is called with the original arguments.
    LegacyHook,
    /// The row performed no table update and returned zero.
    Ignored,
}

/// Action selected for one iteration of the `0x00660070` batch loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03BatchAction {
    /// The actor slot is below `8`; retail calls `FUN_005ada00` and
    /// `FUN_005b1da0` with the target word and the centre of the actor's area
    /// record, then optionally `FUN_00712ce0` when the value at `+0x14` is in
    /// `1..=0x40`.
    Placement {
        /// Resolved player slot (`0x0d` is replaced by the current player).
        player: u32,
    },
    /// The opcode names a dedicated AI-script handler.
    Script(W13a03ScriptHandler),
    /// Opcodes `0x16..=0x19`, `0x08..=0x0c` and values above `0x1a` do
    /// nothing.
    None,
}

/// One plan for the `0x00660070` batch loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a03BatchPlan {
    /// Number of iterations: `+0x1b`, or `1` when that byte is zero.
    pub iterations: u8,
    /// Work performed on each iteration for a stable command record.
    pub action: W13a03BatchAction,
}

/// Post-batch handler selected by `0x00660280` from the `+0x1b` byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03Followup {
    /// Mode `7` selects `FUN_00662990`.
    Mode7,
    /// Mode `8` selects `0x00661450`, the player-command counter dispatch.
    Mode8,
    /// Mode `9` selects `FUN_00663290`.
    Mode9,
}

/// Pending-command record written by `0x006605d0` at `0xe61c`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03PendingCommand {
    /// `+0x00` (`0xe61c`): command id.
    pub command: u8,
    /// `+0x01` (`0xe61d`): zero-based player slot, only written by commands
    /// `0x12` and `0x22`.
    pub slot: u8,
    /// `+0x02` (`0xe61e`): target word.
    pub target: u16,
    /// `+0x04` (`0xe620`): value word.
    pub value: u32,
    /// `+0x08` (`0xe624`): source word.
    pub source: u32,
}

/// Region/script dispatch data computed by `0x00660580`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a03ScriptRegionRequest {
    /// Script id passed as the first argument of `FUN_00640940` (record `+0x14`).
    pub script_id: u32,
    /// Image offset of the per-player area record passed as the third
    /// argument.
    pub area_offset: u32,
}

/// Query record built by `0x00660310` for the callback `FUN_006620a0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03AreaQuery {
    /// Extent words copied from the resolved player area.
    pub area: W13a03PlayerArea,
    /// Command word from record `+0x10`.
    pub command: u32,
    /// Target word from record `+0x18`, zero-extended.
    pub target: u32,
    /// Marker word copied from the player area.
    pub marker: u16,
    /// Centre of the resolved region, `(left + right) / 2`.
    pub center_x: i32,
    /// Centre of the resolved region, `(top + bottom) / 2`.
    pub center_y: i32,
    /// Zero-extended mode byte from record `+0x1b`.
    pub mode: u32,
}

/// Query record built by `0x00660b90` for the callback `FUN_006625e0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03ScriptQuery {
    /// Extent words copied from the resolved player area.
    pub area: W13a03PlayerArea,
    /// Command word from record `+0x10`.
    pub command: u32,
    /// Target word from record `+0x18`.
    pub target: u16,
    /// Marker word copied from the player area.
    pub marker: u16,
    /// Last byte of the callback record: the low byte of `value - 1`, i.e. the
    /// zero-based region slot.
    pub region_slot: u8,
    /// Value written to the global replay limit `DAT_00fdfe0c`: the mode byte
    /// from record `+0x1b`, or `999999` when that byte is zero.
    pub replay_limit: u32,
}

/// Query record built by `0x00661d10` for the callback `FUN_00661db0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03RegionQuery {
    /// Extent words copied from the resolved player area.
    pub area: W13a03PlayerArea,
    /// Command word passed as the second argument.
    pub command: u32,
    /// Target word passed as the first argument.
    pub target: u16,
    /// Marker word copied from the player area.
    pub marker: u16,
    /// Fixed mode word (`1`).
    pub mode: u32,
}

/// One of the three `0xe6`/`0xe7`/`0xe8` counter arrays updated by
/// `0x00661450`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03TypeCounter {
    /// Selector `0xe6`, array `0x2d84`.
    At0x2d84,
    /// Selector `0xe7`, array `0x2ed4`.
    At0x2ed4,
    /// Selector `0xe8`, array `0x2ff4`.
    At0x2ff4,
}

/// Applied case of `0x00661450`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03CommandUpdate {
    /// One of the three type counters was incremented.
    TypeCounter(W13a03TypeCounter),
    /// The `0xb274` command matrix cell was incremented.
    Matrix,
}

/// Counter arrays updated by `0x00661450`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03CommandCounters {
    /// `0x2d84 + player * 4`: selector `0xe6` counts.
    pub at_0x2d84: [i32; W13A03_PLAYER_COUNT],
    /// `0x2ed4 + player * 4`: selector `0xe7` counts.
    pub at_0x2ed4: [i32; W13A03_PLAYER_COUNT],
    /// `0x2ff4 + player * 4`: selector `0xe8` counts.
    pub at_0x2ff4: [i32; W13A03_PLAYER_COUNT],
}

/// The `0xb274` command matrix: `0xe4` rows of `0xc` dwords.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a03CommandMatrix {
    /// `0xb274 + (player + command * 0xc) * 4`.
    pub rows: [[i32; W13A03_COMMAND_MATRIX_STRIDE]; W13A03_COMMAND_MATRIX_ROWS],
}

impl Default for W13a03CommandMatrix {
    fn default() -> Self {
        Self {
            rows: [[0; W13A03_COMMAND_MATRIX_STRIDE]; W13A03_COMMAND_MATRIX_ROWS],
        }
    }
}

/// Applied selector of `0x00661620`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03SelectionSet {
    /// Selector `0`: `0x00` and `0x90` were incremented.
    Selector0,
    /// Selector `1`: `0x30` and `0x60` were incremented.
    Selector1,
    /// Selector `2`: all four arrays were incremented.
    Selector2,
}

/// Counter arrays updated by `0x00661620`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03SelectionCounters {
    /// `0x00 + player * 4`.
    pub at_0x00: [i32; W13A03_PLAYER_COUNT],
    /// `0x30 + player * 4`.
    pub at_0x30: [i32; W13A03_PLAYER_COUNT],
    /// `0x60 + player * 4`.
    pub at_0x60: [i32; W13A03_PLAYER_COUNT],
    /// `0x90 + player * 4`.
    pub at_0x90: [i32; W13A03_PLAYER_COUNT],
}

/// Applied selector of `0x0066175a`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03RatioSet {
    /// Selector `0`: a quarter of the amount reached all four quadrant
    /// arrays (`0x2de4`, `0x2e14`, `0x2f34`, `0x2f64`).
    QuarterAll,
    /// Selector `1`: the whole amount reached `0x2de4`.
    WholeA,
    /// Selector `2`: the whole amount reached `0x2f34`.
    WholeC,
    /// Selector `3`: half the amount reached `0x2de4` and `0x2f34`.
    HalfAC,
    /// Selector `4`: the whole amount reached `0x2e14`.
    WholeB,
    /// Selector `5`: the whole amount reached `0x2f64`.
    WholeD,
    /// Selector `6`: half the amount reached `0x2e14` and `0x2f64`.
    HalfBD,
    /// Selector `7`: the whole amount reached `0x3204`.
    WholeE,
}

/// Counter arrays updated by `0x0066175a`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03RatioCounters {
    /// `0x2de4 + player * 4`.
    pub at_0x2de4: [i32; W13A03_PLAYER_COUNT],
    /// `0x2e14 + player * 4`.
    pub at_0x2e14: [i32; W13A03_PLAYER_COUNT],
    /// `0x2f34 + player * 4`.
    pub at_0x2f34: [i32; W13A03_PLAYER_COUNT],
    /// `0x2f64 + player * 4`.
    pub at_0x2f64: [i32; W13A03_PLAYER_COUNT],
    /// `0x3204 + player * 4`.
    pub at_0x3204: [i32; W13A03_PLAYER_COUNT],
}

/// Result of the relation-matrix update in `0x00661990`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03RelationWrite {
    /// Relation byte written to the actor's row: `0`, `1` or `2`.
    pub relation: u8,
    /// Whether the non-zero relation was propagated across the actor's row.
    pub propagated: bool,
}

/// `0x00660070` (`FUN_00660070`, 295 bytes): plan one execution of the
/// per-player command batch.
///
/// The row returns the constant `1` and does nothing when the record's `+0x00`
/// player byte is zero; `None` reports that case. Otherwise it saves the dword
/// returned by `FUN_0073f110`, writes `0x75bcd15` through `FUN_0073f120` (the
/// same global), restores the saved value at the end, and loops `+0x1b` times,
/// defaulting that byte to `1` when it is zero.
///
/// Each iteration re-reads `+0x10` and decides:
///
/// * `0x0d` substitutes the current-player global (`FUN_007a9d00`) and then
///   takes the placement path when the substitute is below `8`;
/// * `0x00..=0x07` take the placement path directly;
/// * `0x0e`, `0x0f`, `0x10`, `0x11`, `0x12..=0x15` and `0x1a` delegate to the
///   AI-script handlers named by [`W13a03ScriptHandler`];
/// * `0x16..=0x19` and every other value do nothing.
///
/// The placement path itself depends on a unit lookup (`FUN_005ada00` /
/// `FUN_005b1da0`) and a callback (`FUN_00712ce0`); only the dispatch decision
/// is reported here.
pub fn w13a03_batch_plan(
    record_player: u8,
    mode: u8,
    command: u32,
    current_player: u32,
) -> Option<W13a03BatchPlan> {
    if record_player == 0 {
        return None;
    }
    let iterations = if mode == 0 { 1 } else { mode };
    let action = if command == 0x0d {
        if current_player < W13A03_PLAYER_COUNT as u32 {
            W13a03BatchAction::Placement {
                player: current_player,
            }
        } else {
            W13a03BatchAction::None
        }
    } else if command < W13A03_PLAYER_COUNT as u32 {
        W13a03BatchAction::Placement { player: command }
    } else {
        match w13a03_script_handler(command) {
            Some(handler) => W13a03BatchAction::Script(handler),
            None => W13a03BatchAction::None,
        }
    };
    Some(W13a03BatchPlan { iterations, action })
}

/// Shared opcode-to-handler mapping used by the dispatch rows.
fn w13a03_script_handler(opcode: u32) -> Option<W13a03ScriptHandler> {
    match opcode {
        0x0e => Some(W13a03ScriptHandler::Script0e),
        0x0f => Some(W13a03ScriptHandler::Script0f),
        0x10 => Some(W13a03ScriptHandler::Script10),
        0x11 => Some(W13a03ScriptHandler::Script11),
        0x12..=0x15 => Some(W13a03ScriptHandler::Script12To15),
        0x1a => Some(W13a03ScriptHandler::Script1a),
        _ => None,
    }
}

/// `0x00660280` (`FUN_00660280`, 143 bytes): post-batch follow-up dispatch.
///
/// The row runs the lifecycle call `FUN_006ae4e0`, reads the mode byte at
/// `+0x1b`, and selects:
///
/// * `7` -> `FUN_00662990`;
/// * `8` -> `0x00661450` (player-command counter dispatch);
/// * `9` -> `FUN_00663290`.
///
/// All paths then call `FUN_006ae520` and return `1`. Modes outside `7..=9`
/// still run the lifecycle calls but hit no handler.
pub fn w13a03_batch_followup(mode: u8) -> Option<W13a03Followup> {
    match mode {
        7 => Some(W13a03Followup::Mode7),
        8 => Some(W13a03Followup::Mode8),
        9 => Some(W13a03Followup::Mode9),
        _ => None,
    }
}

/// `0x00660310` (`FUN_00660310`, 3-byte entry / shared body): build the
/// area-query record passed to `FUN_006c2670` with callback `FUN_006620a0`.
///
/// The row requires a non-zero player byte at `+0x00` and a non-zero region id
/// at `+0x14`. It copies the player area record at
/// `0xeb70 + (player - 1) * 0x14` (four `u16` extent words plus the `+0x12`
/// marker), takes the region rectangle at `0xeb70 + (region - 1) * 0x14`
/// (four `i32` edges), and computes the rectangle centre:
///
/// ```c
/// x = (region.right + region.left) / 2;
/// y = (region.bottom + region.top) / 2;
/// ```
///
/// The query also carries the command word at `+0x10`, the zero-extended
/// target at `+0x18` and the mode byte at `+0x1b`. The callback itself is not
/// modelled.
pub fn w13a03_area_query(
    areas: &[W13a03PlayerArea],
    regions: &[W13a03RegionRect],
    request: &W13a03CommandRequest,
) -> Option<W13a03AreaQuery> {
    if request.player == 0 || request.value == 0 {
        return None;
    }
    let area = *areas.get(usize::from(request.player - 1))?;
    let region_index = usize::try_from((request.value as i32).checked_sub(1)?).ok()?;
    let region = *regions.get(region_index)?;
    Some(W13a03AreaQuery {
        area,
        command: request.command,
        target: u32::from(request.target),
        marker: area.marker,
        center_x: region.left.wrapping_add(region.right) / 2,
        center_y: region.top.wrapping_add(region.bottom) / 2,
        mode: u32::from(request.mode),
    })
}

/// `0x00660440` (`FUN_00660440`, 173 bytes): update one bit of a per-player
/// dword array based at `0xeb50`.
///
/// The value at `+0x14` must be below `0x100`. Word index is `value >> 5`,
/// bit index is `value & 0x1f`, and the mode byte at `+0x1b` selects:
///
/// * `4`: set the bit;
/// * `5`: clear the bit;
/// * `6`: toggle the bit;
/// * `0x0b`: clear the bit when `probe_ok`, otherwise set it. `probe_ok` is
///   the result of `FUN_0073f3b0(0x39) >= 0`;
/// * anything else: no change.
///
/// Returns `true` when a word was touched. The retail row always returns `1`.
pub fn w13a03_update_player_bit(
    words: &mut [u32; W13A03_PLAYER_COUNT],
    value: u32,
    mode: u8,
    probe_ok: bool,
) -> bool {
    if value >= 0x100 {
        return false;
    }
    let word = (value >> 5) as usize;
    let bit = value & 0x1f;
    let mask = 1u32 << bit;
    match mode {
        0x04 => words[word] |= mask,
        0x05 => words[word] &= !mask,
        0x06 => words[word] ^= mask,
        0x0b => {
            if probe_ok {
                words[word] &= !mask;
            } else {
                words[word] |= mask;
            }
        }
        _ => return false,
    }
    true
}

/// `0x006604c8` (`FUN_006604c8`, 8 bytes): constant success result.
///
/// The whole body is `return 1;`. It is the default handler installed for a
/// command-card row that performs no work.
pub fn w13a03_default_success_a() -> u32 {
    1
}

/// `0x00660580` (`FUN_00660580`, 72 bytes): script-dispatch request.
///
/// With a non-zero player byte at `+0x00`, the row reads the script id at
/// `+0x14` and computes the third argument of `FUN_00640940` as
/// `(player - 1) * 5 * 4 + 0x3adc * 4`, i.e.
/// `0xeb70 + (player - 1) * 0x14`. The second argument is the current-player
/// global (`FUN_007a9d00`), which is not part of this value helper.
pub fn w13a03_script_region_request(
    player: u8,
    script_id: u32,
) -> Option<W13a03ScriptRegionRequest> {
    if player == 0 {
        return None;
    }
    Some(W13a03ScriptRegionRequest {
        script_id,
        area_offset: W13A03_AREA_BASE + u32::from(player - 1) * W13A03_AREA_STRIDE,
    })
}

/// `0x006605d0` (`FUN_006605d0`, 200 bytes): write the pending-command record.
///
/// The preamble call `FUN_007aaf50` is engine state and is not modelled; the
/// row then branches on the command byte at `+0x1a`:
///
/// * `0x12` / `0x22`: a zero player byte at `+0x00` aborts; otherwise the
///   zero-based slot `player - 1` is stored at `0xe61d`, and the common tail
///   runs;
/// * `0x28`: `0xe61c = 0x28`, `0xe620 = value`, `0xe61e = 2`, then return;
/// * any other command: the common tail runs.
///
/// The common tail writes `0xe61c = command`, `0xe61e = target`,
/// `0xe620 = value` and `0xe624 = source` (record `+0x04`).
///
/// Returns `false` only for the aborted `0x12`/`0x22` zero-player case. The
/// retail row returns `1` on every path.
pub fn w13a03_write_pending_command(
    pending: &mut W13a03PendingCommand,
    request: &W13a03CommandRequest,
) -> bool {
    match request.action {
        0x12 | 0x22 => {
            if request.player == 0 {
                return false;
            }
            pending.slot = request.player - 1;
        }
        0x28 => {
            pending.command = 0x28;
            pending.target = 2;
            pending.value = request.value;
            return true;
        }
        _ => {}
    }
    pending.command = request.action;
    pending.target = request.target;
    pending.value = request.value;
    pending.source = request.source;
    true
}

/// `0x00660820` (`FUN_00660820`, 7 bytes): constant success result.
///
/// The whole body is `return 1;`; it is a second no-work handler in the same
/// dispatch family as `0x006604c8`.
pub fn w13a03_default_success_b() -> u32 {
    1
}

/// `0x0066098d` (`FUN_0066098d`, 27 bytes): current-player gate.
///
/// The row compares the current-player global (`FUN_007a9d00`) with
/// `DAT_00f2acf4` and calls `FUN_006e4f30` when they are equal, then returns
/// `1`. `true` reports that the conditional call is taken.
pub fn w13a03_current_player_gate(current: u32, expected: u32) -> bool {
    current == expected
}

/// `0x00660b90` (`FUN_00660b90`, 209 bytes): build the script query passed to
/// `FUN_006c2670` with callback `FUN_006625e0`.
///
/// The row requires a non-zero player byte at `+0x00` and a non-zero word at
/// `+0x14`. It copies the player area record at
/// `0xeb70 + (player - 1) * 0x14`. The callback record carries the command
/// word at `+0x10`, the target word at `+0x18`, the area marker, and the low
/// byte of `value - 1` as its last byte (the zero-based region slot). The
/// mode byte at `+0x1b` is written to the global replay limit
/// `DAT_00fdfe0c`, or `999999` when that byte is zero. After the callback the
/// row calls `FUN_00664810` and falls into the non-returning consistency trap
/// `FUN_00c86b6e`.
pub fn w13a03_script_query(
    areas: &[W13a03PlayerArea],
    request: &W13a03CommandRequest,
) -> Option<W13a03ScriptQuery> {
    if request.player == 0 || request.value == 0 {
        return None;
    }
    let area = *areas.get(usize::from(request.player - 1))?;
    let replay_limit = if request.mode == 0 {
        999_999
    } else {
        u32::from(request.mode)
    };
    Some(W13a03ScriptQuery {
        area,
        command: request.command,
        target: request.target,
        marker: area.marker,
        region_slot: (request.value as u8).wrapping_sub(1),
        replay_limit,
    })
}

/// `0x00660d38` (`FUN_00660d38`, 63 bytes): target-command dispatch.
///
/// After masking a global with an unrecovered call result, the row returns
/// immediately unless the player byte at `+0x00` is non-zero and the region
/// query `0x00661d10(target at +0x18, command at +0x10, player - 1)` succeeds.
/// It then switches on the mode byte at `+0x1b`:
///
/// * `0` and `6`: call `FUN_005b69b0`;
/// * `4`: call `FUN_005b4880`;
/// * `5`: call `FUN_005b3bf0`;
/// * anything else: no handler.
///
/// The global-mask update and the region-query guard are not modelled; this
/// helper reports only the recovered mode dispatch.
pub fn w13a03_target_mode_handler(mode: u8) -> Option<W13a03TargetHandler> {
    match mode {
        0 | 6 => Some(W13a03TargetHandler::Mode0Or6),
        4 => Some(W13a03TargetHandler::Mode4),
        5 => Some(W13a03TargetHandler::Mode5),
        _ => None,
    }
}

/// Handler selected by [`w13a03_target_mode_handler`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03TargetHandler {
    /// Mode `0`/`6`: `FUN_005b69b0`.
    Mode0Or6,
    /// Mode `4`: `FUN_005b4880`.
    Mode4,
    /// Mode `5`: `FUN_005b3bf0`.
    Mode5,
}

/// `0x00661410` (`FUN_00661410`, 6 bytes): command-table base address.
///
/// The row is a constant address getter for `0x00fdf900`; it reads no memory.
pub fn w13a03_command_table_base() -> usize {
    W13A03_COMMAND_TABLE_BASE
}

/// `0x00661450` (`FUN_00661450`, 129 bytes): apply one player-command effect.
///
/// The first argument is a player selector with two special forms: `0x0d`
/// substitutes the current player, and `0x0e..=0x1a` name AI-script handlers.
/// `0x16..=0x19` return immediately. The remaining values fall through to the
/// low 16 bits of the second argument:
///
/// * `0xe5`: no update;
/// * `0xe6` / `0xe7` / `0xe8`: when the player is below `8`, add the amount to
///   `0x2d84` / `0x2ed4` / `0x2ff4` respectively;
/// * otherwise, when the legacy global `DAT_01165f58` is non-zero, call
///   `FUN_006ae540` and stop; when the player is below `8` and the command is at
///   most `0xe3`, add the amount to
///   `0xb274 + (player + command * 0xc) * 4`.
///
/// Every recovered path returns zero in retail; [`W13a03Outcome`] reports the
/// selected action instead.
pub fn w13a03_apply_player_counter(
    player: u32,
    selector: u16,
    amount: i32,
    current_player: u32,
    legacy_hook: bool,
    counters: &mut W13a03CommandCounters,
    matrix: &mut W13a03CommandMatrix,
) -> W13a03Outcome<W13a03CommandUpdate> {
    // The retail switch inspects the original selector; only the fall-through
    // uses the substituted current player, so do not re-dispatch it.
    if player != 0x0d && (0x0e..=0x1a).contains(&player) {
        return match w13a03_script_handler(player) {
            Some(handler) => W13a03Outcome::Delegate(handler),
            None => W13a03Outcome::Ignored,
        };
    }
    let player = if player == 0x0d {
        current_player
    } else {
        player
    };
    let player_index = usize::try_from(player).ok();
    match selector {
        0xe5 => return W13a03Outcome::Ignored,
        0xe6 | 0xe7 | 0xe8 => {
            let Some(index) = player_index.filter(|&i| i < W13A03_PLAYER_COUNT) else {
                return W13a03Outcome::Ignored;
            };
            let (array, kind) = match selector {
                0xe6 => (&mut counters.at_0x2d84, W13a03TypeCounter::At0x2d84),
                0xe7 => (&mut counters.at_0x2ed4, W13a03TypeCounter::At0x2ed4),
                _ => (&mut counters.at_0x2ff4, W13a03TypeCounter::At0x2ff4),
            };
            array[index] = array[index].wrapping_add(amount);
            return W13a03Outcome::Applied(W13a03CommandUpdate::TypeCounter(kind));
        }
        _ => {}
    }
    if legacy_hook {
        return W13a03Outcome::LegacyHook;
    }
    let Some(index) = player_index.filter(|&i| i < W13A03_PLAYER_COUNT) else {
        return W13a03Outcome::Ignored;
    };
    if selector > 0xe3 {
        return W13a03Outcome::Ignored;
    }
    let cell = &mut matrix.rows[usize::from(selector)][index];
    *cell = cell.wrapping_add(amount);
    W13a03Outcome::Applied(W13a03CommandUpdate::Matrix)
}

/// `0x00661620` (`FUN_00661620`, 50 bytes): apply a selection counter effect.
///
/// The dispatch header is the same as [`w13a03_apply_player_counter`]
/// (`0x0d` substitutes the current player; `0x0e..=0x1a` delegate;
/// `0x16..=0x19` stop). The low 16 bits of the second argument then select:
///
/// * `0`: add the amount to `0x00` and `0x90`;
/// * `1`: add the amount to `0x30` and `0x60`;
/// * `2`: add the amount to all four arrays;
/// * other values: no update.
///
/// The header also requires a player below `8`. Retail returns zero on every
/// path.
pub fn w13a03_apply_selection_counter(
    player: u32,
    selector: u16,
    amount: i32,
    current_player: u32,
    counters: &mut W13a03SelectionCounters,
) -> W13a03Outcome<W13a03SelectionSet> {
    // The retail switch inspects the original selector; only the fall-through
    // uses the substituted current player, so do not re-dispatch it.
    if player != 0x0d && (0x0e..=0x1a).contains(&player) {
        return match w13a03_script_handler(player) {
            Some(handler) => W13a03Outcome::Delegate(handler),
            None => W13a03Outcome::Ignored,
        };
    }
    let player = if player == 0x0d {
        current_player
    } else {
        player
    };
    let Some(index) = usize::try_from(player)
        .ok()
        .filter(|&i| i < W13A03_PLAYER_COUNT)
    else {
        return W13a03Outcome::Ignored;
    };
    match selector {
        0 => {
            counters.at_0x00[index] = counters.at_0x00[index].wrapping_add(amount);
            counters.at_0x90[index] = counters.at_0x90[index].wrapping_add(amount);
            W13a03Outcome::Applied(W13a03SelectionSet::Selector0)
        }
        1 => {
            counters.at_0x30[index] = counters.at_0x30[index].wrapping_add(amount);
            counters.at_0x60[index] = counters.at_0x60[index].wrapping_add(amount);
            W13a03Outcome::Applied(W13a03SelectionSet::Selector1)
        }
        2 => {
            counters.at_0x00[index] = counters.at_0x00[index].wrapping_add(amount);
            counters.at_0x90[index] = counters.at_0x90[index].wrapping_add(amount);
            counters.at_0x30[index] = counters.at_0x30[index].wrapping_add(amount);
            counters.at_0x60[index] = counters.at_0x60[index].wrapping_add(amount);
            W13a03Outcome::Applied(W13a03SelectionSet::Selector2)
        }
        _ => W13a03Outcome::Ignored,
    }
}

/// `0x0066175a` (`FUN_0066175a`, 64 bytes): apply a ratio counter effect.
///
/// The dispatch header matches the other player-command rows. For a player
/// below `8`, the low 16 bits of the second argument select:
///
/// | selector | effect |
/// |----------|--------|
/// | `0` | quarter `(amount + 3) >> 2` to `0x2de4`, `0x2f34`, `0x2e14`, `0x2f64` |
/// | `1` | whole amount to `0x2de4` |
/// | `2` | whole amount to `0x2f34` |
/// | `3` | half `(amount + 1) >> 1` to `0x2de4` and `0x2f34` |
/// | `4` | whole amount to `0x2e14` |
/// | `5` | whole amount to `0x2f64` |
/// | `6` | half `(amount + 1) >> 1` to `0x2e14` and `0x2f64` |
/// | `7` | whole amount to `0x3204` |
///
/// The quarter/half sums use unsigned 32-bit arithmetic in retail. Retail
/// returns zero on every path.
pub fn w13a03_apply_ratio_counter(
    player: u32,
    selector: u16,
    amount: i32,
    current_player: u32,
    counters: &mut W13a03RatioCounters,
) -> W13a03Outcome<W13a03RatioSet> {
    // The retail switch inspects the original selector; only the fall-through
    // uses the substituted current player, so do not re-dispatch it.
    if player != 0x0d && (0x0e..=0x1a).contains(&player) {
        return match w13a03_script_handler(player) {
            Some(handler) => W13a03Outcome::Delegate(handler),
            None => W13a03Outcome::Ignored,
        };
    }
    let player = if player == 0x0d {
        current_player
    } else {
        player
    };
    let Some(index) = usize::try_from(player)
        .ok()
        .filter(|&i| i < W13A03_PLAYER_COUNT)
    else {
        return W13a03Outcome::Ignored;
    };
    let add = |array: &mut [i32; W13A03_PLAYER_COUNT], value: u32| {
        array[index] = array[index].wrapping_add(value as i32);
    };
    let quarter = (amount as u32).wrapping_add(3) >> 2;
    let half = (amount as u32).wrapping_add(1) >> 1;
    let set = match selector {
        0 => {
            add(&mut counters.at_0x2de4, quarter);
            add(&mut counters.at_0x2f34, quarter);
            add(&mut counters.at_0x2e14, quarter);
            add(&mut counters.at_0x2f64, quarter);
            W13a03RatioSet::QuarterAll
        }
        1 => {
            add(&mut counters.at_0x2de4, amount as u32);
            W13a03RatioSet::WholeA
        }
        2 => {
            add(&mut counters.at_0x2f34, amount as u32);
            W13a03RatioSet::WholeC
        }
        3 => {
            add(&mut counters.at_0x2de4, half);
            add(&mut counters.at_0x2f34, half);
            W13a03RatioSet::HalfAC
        }
        4 => {
            add(&mut counters.at_0x2e14, amount as u32);
            W13a03RatioSet::WholeB
        }
        5 => {
            add(&mut counters.at_0x2f64, amount as u32);
            W13a03RatioSet::WholeD
        }
        6 => {
            add(&mut counters.at_0x2e14, half);
            add(&mut counters.at_0x2f64, half);
            W13a03RatioSet::HalfBD
        }
        7 => {
            add(&mut counters.at_0x3204, amount as u32);
            W13a03RatioSet::WholeE
        }
        _ => return W13a03Outcome::Ignored,
    };
    W13a03Outcome::Applied(set)
}

/// `0x00661990` (`FUN_00661990`, 81 bytes): write one relation-matrix cell.
///
/// The row runs only for a first argument below `0x1b`. `0x0d`, `0x16..=0x19`
/// do nothing; `0x0e..=0x15` and `0x1a` delegate to the AI-script handlers.
/// Every other value is a zero-based relation slot:
///
/// * the target slot equal to the current player is skipped;
/// * the relation byte is `1` for selector `1`, `2` for selector `2`, and `0`
///   otherwise;
/// * the byte is written to `matrix[current][slot]`;
/// * when the relation is non-zero and `matrix[current][current]` differs,
///   every non-zero cell in the current player's row is overwritten with the
///   relation.
///
/// The matrix rows are `0xc` bytes based at `0xe544`. The row is passed as a
/// flat byte slice because the `+0xc` slot aliases the next row in retail.
/// The retail body re-reads the current-player global for the self-relation
/// test and for every column of the propagation loop; this helper assumes the
/// value is stable for the duration of the call. The row returns zero on every
/// path.
pub fn w13a03_apply_relation(
    player: u32,
    selector: u16,
    current_player: u32,
    cells: &mut [u8],
) -> W13a03Outcome<W13a03RelationWrite> {
    if player >= 0x1b {
        return W13a03Outcome::Ignored;
    }
    match player {
        0x0d | 0x16..=0x19 => return W13a03Outcome::Ignored,
        0x0e..=0x15 | 0x1a => {
            if let Some(handler) = w13a03_script_handler(player) {
                return W13a03Outcome::Delegate(handler);
            }
            return W13a03Outcome::Ignored;
        }
        _ => {}
    }
    if player == current_player {
        return W13a03Outcome::Ignored;
    }
    let actor = usize::try_from(current_player).ok();
    let target = usize::try_from(player).ok();
    let (Some(actor), Some(target)) = (actor, target) else {
        return W13a03Outcome::Ignored;
    };
    if actor >= W13A03_RELATION_SLOTS || cells.len() < W13A03_RELATION_SLOTS * W13A03_RELATION_SLOTS
    {
        return W13a03Outcome::Ignored;
    }
    let row = actor * W13A03_RELATION_STRIDE;
    let cell = row + target;
    if cell >= cells.len() {
        return W13a03Outcome::Ignored;
    }
    let relation = match selector {
        1 => 1u8,
        2 => 2u8,
        _ => 0u8,
    };
    cells[cell] = relation;
    let mut propagated = false;
    if relation != 0 && cells[row + actor] != relation {
        for column in 0..W13A03_RELATION_SLOTS {
            if cells[row + column] != 0 {
                cells[row + column] = relation;
            }
        }
        propagated = true;
    }
    W13a03Outcome::Applied(W13a03RelationWrite {
        relation,
        propagated,
    })
}

/// `0x00661b40` (`FUN_00661b40`, 40 bytes): text-driven time budget.
///
/// A null pointer returns `0`. Otherwise the row measures the C string length,
/// multiplies by `0x32`, and clamps values below `0xfa1` up to `4000`:
///
/// ```c
/// uVar3 = strlen(text) * 50;
/// if (uVar3 < 4001) uVar3 = 4000;
/// ```
pub fn w13a03_text_budget(text: Option<&[u8]>) -> u32 {
    let Some(bytes) = text else {
        return 0;
    };
    let length = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    let scaled = (length as u32).wrapping_mul(0x32);
    if scaled < 0x0fa1 { 4000 } else { scaled }
}

/// `0x00661d10` (`FUN_00661d10`, 154 bytes): build the region query passed to
/// `FUN_006c2670` with callback `FUN_00661db0`.
///
/// `player_index` is already zero-based: the call site `0x00660d38` passes
/// `record.player - 1`. The row copies the area record at
/// `0xeb70 + player_index * 0x14`, carries the caller's target word and command
/// word, and uses the fixed mode byte `1`. It then falls into the
/// non-returning consistency trap `FUN_00c86b6e`.
pub fn w13a03_region_query(
    areas: &[W13a03PlayerArea],
    player_index: u8,
    target: u16,
    command: u32,
) -> Option<W13a03RegionQuery> {
    let area = *areas.get(usize::from(player_index))?;
    Some(W13a03RegionQuery {
        area,
        command,
        target,
        marker: area.marker,
        mode: 1,
    })
}

#[cfg(test)]
mod tests_wave13_agent03 {
    use super::*;

    fn sample_area(marker: u16) -> W13a03PlayerArea {
        W13a03PlayerArea {
            left: 0x10,
            top: 0x20,
            right: 0x30,
            bottom: 0x40,
            marker,
        }
    }

    fn sample_request(player: u8, command: u32, value: u32) -> W13a03CommandRequest {
        W13a03CommandRequest {
            player,
            source: 0x1122_3344,
            command,
            value,
            target: 0x5566,
            action: 0x04,
            mode: 0,
        }
    }

    #[test]
    fn batch_plan_matches_0x00660070() {
        // A zero player byte aborts before the loop.
        assert_eq!(w13a03_batch_plan(0, 0, 0, 0), None);

        let zero_mode = w13a03_batch_plan(1, 0, 0, 0).unwrap();
        assert_eq!(zero_mode.iterations, 1);
        assert_eq!(zero_mode.action, W13a03BatchAction::Placement { player: 0 });

        assert_eq!(w13a03_batch_plan(1, 9, 5, 0).unwrap().iterations, 9);

        // 0x0d is replaced by the current player.
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x0d, 3).unwrap().action,
            W13a03BatchAction::Placement { player: 3 }
        );
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x0d, 8).unwrap().action,
            W13a03BatchAction::None
        );

        assert_eq!(
            w13a03_batch_plan(1, 1, 0x0e, 0).unwrap().action,
            W13a03BatchAction::Script(W13a03ScriptHandler::Script0e)
        );
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x12, 0).unwrap().action,
            W13a03BatchAction::Script(W13a03ScriptHandler::Script12To15)
        );
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x1a, 0).unwrap().action,
            W13a03BatchAction::Script(W13a03ScriptHandler::Script1a)
        );
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x16, 0).unwrap().action,
            W13a03BatchAction::None
        );
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x0c, 0).unwrap().action,
            W13a03BatchAction::None
        );
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x1b, 0).unwrap().action,
            W13a03BatchAction::None
        );
    }

    #[test]
    fn batch_followup_matches_0x00660280() {
        assert_eq!(w13a03_batch_followup(7), Some(W13a03Followup::Mode7));
        assert_eq!(w13a03_batch_followup(8), Some(W13a03Followup::Mode8));
        assert_eq!(w13a03_batch_followup(9), Some(W13a03Followup::Mode9));
        for mode in [0, 6, 10, 0xff] {
            assert_eq!(w13a03_batch_followup(mode), None);
        }
    }

    #[test]
    fn area_query_matches_0x00660310() {
        let areas = [sample_area(7); 8];
        let regions = [
            W13a03RegionRect {
                left: 10,
                top: 20,
                right: 30,
                bottom: 40,
            },
            W13a03RegionRect {
                left: -9,
                top: -3,
                right: 5,
                bottom: 9,
            },
        ];
        let request = sample_request(2, 0x33, 2);

        let query = w13a03_area_query(&areas, &regions, &request).unwrap();
        assert_eq!(query.area, sample_area(7));
        assert_eq!(query.command, 0x33);
        assert_eq!(query.target, 0x5566);
        assert_eq!(query.marker, 7);
        assert_eq!(query.mode, 0);
        // Region 2 is regions[1]; C division truncates toward zero.
        assert_eq!(query.center_x, (-9 + 5) / 2);
        assert_eq!(query.center_y, (-3 + 9) / 2);

        assert_eq!(
            w13a03_area_query(&areas, &regions, &sample_request(0, 0, 1)),
            None
        );
        assert_eq!(
            w13a03_area_query(&areas, &regions, &sample_request(1, 0, 0)),
            None
        );
        assert_eq!(
            w13a03_area_query(&areas, &regions, &sample_request(1, 0, 3)),
            None
        );
        assert_eq!(
            w13a03_area_query(&areas, &regions, &sample_request(9, 0, 1)),
            None
        );
    }

    #[test]
    fn player_bit_update_matches_0x00660440() {
        let mut words = [0u32; 8];
        assert!(w13a03_update_player_bit(&mut words, 0x25, 4, false));
        assert_eq!(words[1], 1 << 5);
        assert!(w13a03_update_player_bit(&mut words, 0x25, 6, false));
        assert_eq!(words[1], 0);
        assert!(w13a03_update_player_bit(&mut words, 0xdf, 4, false));
        assert_eq!(words[6], 1 << 0x1f);
        assert!(w13a03_update_player_bit(&mut words, 0xdf, 5, false));
        assert_eq!(words[6], 0);

        // Mode 0x0b clears only when the probe succeeds.
        assert!(w13a03_update_player_bit(&mut words, 0, 4, false));
        assert!(w13a03_update_player_bit(&mut words, 0, 0x0b, true));
        assert_eq!(words[0], 0);
        assert!(w13a03_update_player_bit(&mut words, 0, 0x0b, false));
        assert_eq!(words[0], 1);

        // Out-of-range value and unknown mode leave the array alone.
        let before = words;
        assert!(!w13a03_update_player_bit(&mut words, 0x100, 4, false));
        assert!(!w13a03_update_player_bit(&mut words, 1, 3, false));
        assert_eq!(words, before);
    }

    #[test]
    fn success_stubs_match_0x006604c8_and_0x00660820() {
        assert_eq!(w13a03_default_success_a(), 1);
        assert_eq!(w13a03_default_success_b(), 1);
    }

    #[test]
    fn script_region_request_matches_0x00660580() {
        assert_eq!(w13a03_script_region_request(0, 7), None);
        let first = w13a03_script_region_request(1, 0x1234).unwrap();
        assert_eq!(first.script_id, 0x1234);
        assert_eq!(first.area_offset, 0xeb70);
        let last = w13a03_script_region_request(8, 9).unwrap();
        assert_eq!(last.area_offset, 0xeb70 + 7 * 0x14);
    }

    #[test]
    fn pending_command_matches_0x006605d0() {
        assert_eq!(W13A03_PENDING_COMMAND_BASE, 0xe61c);
        let mut pending = W13a03PendingCommand::default();
        let mut request = sample_request(3, 0, 0x99);
        request.action = 0x12;
        assert!(w13a03_write_pending_command(&mut pending, &request));
        assert_eq!(pending.slot, 2);
        assert_eq!(pending.command, 0x12);
        assert_eq!(pending.target, 0x5566);
        assert_eq!(pending.value, 0x99);
        assert_eq!(pending.source, 0x1122_3344);

        // The 0x12/0x22 path aborts for a zero player.
        let mut untouched = W13a03PendingCommand {
            command: 0xaa,
            slot: 0xbb,
            target: 0xcccc,
            value: 0xdddd,
            source: 0xeeee,
        };
        request.action = 0x22;
        request.player = 0;
        assert!(!w13a03_write_pending_command(&mut untouched, &request));
        assert_eq!(untouched.command, 0xaa);

        // Command 0x28 fixes the target at 2 and leaves the source alone.
        let mut fixed = W13a03PendingCommand::default();
        request.action = 0x28;
        request.player = 1;
        request.target = 0x7777;
        request.source = 0x8888;
        assert!(w13a03_write_pending_command(&mut fixed, &request));
        assert_eq!(fixed.command, 0x28);
        assert_eq!(fixed.target, 2);
        assert_eq!(fixed.value, 0x99);
        assert_eq!(fixed.source, 0);
        assert_eq!(fixed.slot, 0);

        // Any other command runs the common tail.
        let mut common = W13a03PendingCommand::default();
        request.action = 0x05;
        assert!(w13a03_write_pending_command(&mut common, &request));
        assert_eq!(common.command, 0x05);
        assert_eq!(common.target, 0x7777);
        assert_eq!(common.source, 0x8888);
    }

    #[test]
    fn current_player_gate_matches_0x0066098d() {
        assert!(w13a03_current_player_gate(3, 3));
        assert!(!w13a03_current_player_gate(3, 4));
        assert!(w13a03_current_player_gate(0, 0));
    }

    #[test]
    fn script_query_matches_0x00660b90() {
        let areas = [sample_area(0xbeef); 8];
        let request = sample_request(1, 0x77, 1);
        let query = w13a03_script_query(&areas, &request).unwrap();
        assert_eq!(query.command, 0x77);
        assert_eq!(query.target, 0x5566);
        assert_eq!(query.marker, 0xbeef);
        assert_eq!(query.region_slot, 0);
        assert_eq!(query.replay_limit, 999_999);

        // The callback's last byte is the low byte of value - 1; the mode byte
        // only feeds the global replay limit.
        let mut mode_request = request;
        mode_request.mode = 5;
        mode_request.value = 3;
        let mode_query = w13a03_script_query(&areas, &mode_request).unwrap();
        assert_eq!(mode_query.region_slot, 2);
        assert_eq!(mode_query.replay_limit, 5);
        assert_eq!(mode_query.marker, 0xbeef);
        assert_eq!(w13a03_script_query(&areas, &sample_request(0, 0, 1)), None);
        assert_eq!(w13a03_script_query(&areas, &sample_request(1, 0, 0)), None);
        assert_eq!(w13a03_script_query(&areas, &sample_request(9, 0, 1)), None);
    }

    #[test]
    fn target_mode_dispatch_matches_0x00660d38() {
        assert_eq!(
            w13a03_target_mode_handler(0),
            Some(W13a03TargetHandler::Mode0Or6)
        );
        assert_eq!(
            w13a03_target_mode_handler(6),
            Some(W13a03TargetHandler::Mode0Or6)
        );
        assert_eq!(
            w13a03_target_mode_handler(4),
            Some(W13a03TargetHandler::Mode4)
        );
        assert_eq!(
            w13a03_target_mode_handler(5),
            Some(W13a03TargetHandler::Mode5)
        );
        for mode in [1, 2, 3, 7, 0xff] {
            assert_eq!(w13a03_target_mode_handler(mode), None);
        }
    }

    #[test]
    fn command_table_base_matches_0x00661410() {
        assert_eq!(w13a03_command_table_base(), 0x00fdf900);
    }

    #[test]
    fn player_counter_matches_0x00661450() {
        let mut counters = W13a03CommandCounters::default();
        let mut matrix = W13a03CommandMatrix::default();

        assert_eq!(
            w13a03_apply_player_counter(2, 0xe6, 5, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Applied(W13a03CommandUpdate::TypeCounter(
                W13a03TypeCounter::At0x2d84
            ))
        );
        assert_eq!(counters.at_0x2d84[2], 5);
        assert_eq!(
            w13a03_apply_player_counter(2, 0xe7, -1, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Applied(W13a03CommandUpdate::TypeCounter(
                W13a03TypeCounter::At0x2ed4
            ))
        );
        assert_eq!(counters.at_0x2ed4[2], -1);
        assert_eq!(
            w13a03_apply_player_counter(2, 0xe8, 7, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Applied(W13a03CommandUpdate::TypeCounter(
                W13a03TypeCounter::At0x2ff4
            ))
        );
        assert_eq!(counters.at_0x2ff4[2], 7);

        // 0xe5 is ignored.
        assert_eq!(
            w13a03_apply_player_counter(2, 0xe5, 9, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Ignored
        );
        // A player outside 0..7 is ignored for the type counters.
        assert_eq!(
            w13a03_apply_player_counter(8, 0xe6, 9, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Ignored
        );

        // 0x0d resolves to the current player and reaches the command matrix.
        assert_eq!(
            w13a03_apply_player_counter(0x0d, 0x10, 3, 4, false, &mut counters, &mut matrix),
            W13a03Outcome::Applied(W13a03CommandUpdate::Matrix)
        );
        assert_eq!(matrix.rows[0x10][4], 3);
        // The matrix rejects commands above 0xe3.
        assert_eq!(
            w13a03_apply_player_counter(1, 0xe4, 3, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Ignored
        );
        // The legacy hook wins before the matrix write.
        assert_eq!(
            w13a03_apply_player_counter(1, 0x10, 3, 0, true, &mut counters, &mut matrix),
            W13a03Outcome::LegacyHook
        );
        // Delegated opcodes.
        assert_eq!(
            w13a03_apply_player_counter(0x0f, 0, 0, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Delegate(W13a03ScriptHandler::Script0f)
        );
        assert_eq!(
            w13a03_apply_player_counter(0x16, 0, 0, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Ignored
        );
    }

    #[test]
    fn selection_counter_matches_0x00661620() {
        let mut counters = W13a03SelectionCounters::default();
        assert_eq!(
            w13a03_apply_selection_counter(1, 0, 4, 0, &mut counters),
            W13a03Outcome::Applied(W13a03SelectionSet::Selector0)
        );
        assert_eq!(counters.at_0x00[1], 4);
        assert_eq!(counters.at_0x90[1], 4);
        assert_eq!(counters.at_0x30[1], 0);
        assert_eq!(counters.at_0x60[1], 0);

        assert_eq!(
            w13a03_apply_selection_counter(1, 1, 6, 0, &mut counters),
            W13a03Outcome::Applied(W13a03SelectionSet::Selector1)
        );
        assert_eq!(counters.at_0x30[1], 6);
        assert_eq!(counters.at_0x60[1], 6);
        assert_eq!(counters.at_0x00[1], 4);

        assert_eq!(
            w13a03_apply_selection_counter(1, 2, 2, 0, &mut counters),
            W13a03Outcome::Applied(W13a03SelectionSet::Selector2)
        );
        assert_eq!(counters.at_0x00[1], 6);
        assert_eq!(counters.at_0x90[1], 6);
        assert_eq!(counters.at_0x30[1], 8);
        assert_eq!(counters.at_0x60[1], 8);

        assert_eq!(
            w13a03_apply_selection_counter(1, 3, 2, 0, &mut counters),
            W13a03Outcome::Ignored
        );
        assert_eq!(
            w13a03_apply_selection_counter(8, 0, 2, 0, &mut counters),
            W13a03Outcome::Ignored
        );
        assert_eq!(
            w13a03_apply_selection_counter(0x0d, 0, 2, 2, &mut counters),
            W13a03Outcome::Applied(W13a03SelectionSet::Selector0)
        );
        assert_eq!(counters.at_0x00[2], 2);
        assert_eq!(
            w13a03_apply_selection_counter(0x12, 0, 2, 0, &mut counters),
            W13a03Outcome::Delegate(W13a03ScriptHandler::Script12To15)
        );
    }

    #[test]
    fn ratio_counter_matches_0x0066175a() {
        let mut counters = W13a03RatioCounters::default();
        assert_eq!(
            w13a03_apply_ratio_counter(3, 0, 7, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::QuarterAll)
        );
        assert_eq!(counters.at_0x2de4[3], 2);
        assert_eq!(counters.at_0x2e14[3], 2);
        assert_eq!(counters.at_0x2f34[3], 2);
        assert_eq!(counters.at_0x2f64[3], 2);
        assert_eq!(counters.at_0x3204[3], 0);

        assert_eq!(
            w13a03_apply_ratio_counter(4, 1, 5, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::WholeA)
        );
        assert_eq!(counters.at_0x2de4[4], 5);
        assert_eq!(
            w13a03_apply_ratio_counter(4, 2, 5, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::WholeC)
        );
        assert_eq!(counters.at_0x2f34[4], 5);

        assert_eq!(
            w13a03_apply_ratio_counter(5, 3, 5, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::HalfAC)
        );
        assert_eq!(counters.at_0x2de4[5], 3);
        assert_eq!(counters.at_0x2f34[5], 3);
        assert_eq!(counters.at_0x2e14[5], 0);

        assert_eq!(
            w13a03_apply_ratio_counter(6, 4, 9, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::WholeB)
        );
        assert_eq!(counters.at_0x2e14[6], 9);
        assert_eq!(
            w13a03_apply_ratio_counter(6, 5, 9, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::WholeD)
        );
        assert_eq!(counters.at_0x2f64[6], 9);

        assert_eq!(
            w13a03_apply_ratio_counter(7, 6, 9, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::HalfBD)
        );
        assert_eq!(counters.at_0x2e14[7], 5);
        assert_eq!(counters.at_0x2f64[7], 5);
        assert_eq!(
            w13a03_apply_ratio_counter(7, 7, 9, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::WholeE)
        );
        assert_eq!(counters.at_0x3204[7], 9);

        assert_eq!(
            w13a03_apply_ratio_counter(1, 8, 9, 0, &mut counters),
            W13a03Outcome::Ignored
        );
        assert_eq!(
            w13a03_apply_ratio_counter(8, 1, 9, 0, &mut counters),
            W13a03Outcome::Ignored
        );
        assert_eq!(
            w13a03_apply_ratio_counter(0x0d, 1, 9, 2, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::WholeA)
        );
        assert_eq!(counters.at_0x2de4[2], 9);
    }

    #[test]
    fn relation_update_matches_0x00661990() {
        assert_eq!(
            W13A03_RELATION_CHUNK * W13A03_RELATION_STRIDE as u32,
            0xe544
        );
        let mut cells = [0u8; W13A03_RELATION_SLOTS * W13A03_RELATION_SLOTS];
        assert_eq!(
            w13a03_apply_relation(1, 1, 0, &mut cells),
            W13a03Outcome::Applied(W13a03RelationWrite {
                relation: 1,
                propagated: true,
            })
        );
        assert_eq!(cells[1], 1);

        // Self-relation differs, so every non-zero cell of the actor's row is
        // overwritten with the new relation.
        cells[0] = 1;
        assert_eq!(
            w13a03_apply_relation(2, 2, 0, &mut cells),
            W13a03Outcome::Applied(W13a03RelationWrite {
                relation: 2,
                propagated: true,
            })
        );
        assert_eq!(cells[0], 2);
        assert_eq!(cells[1], 2);
        assert_eq!(cells[2], 2);
        assert_eq!(cells[3], 0);

        // Selector 0 writes a zero relation and does not propagate.
        assert_eq!(
            w13a03_apply_relation(3, 0, 0, &mut cells),
            W13a03Outcome::Applied(W13a03RelationWrite {
                relation: 0,
                propagated: false,
            })
        );
        assert_eq!(cells[3], 0);

        // Same-slot writes are skipped.
        let before = cells;
        assert_eq!(
            w13a03_apply_relation(0, 1, 0, &mut cells),
            W13a03Outcome::Ignored
        );
        assert_eq!(cells, before);

        // Opcodes delegate; 0x16..=0x19 and values above 0x1a are inert.
        assert_eq!(
            w13a03_apply_relation(0x10, 0, 0, &mut cells),
            W13a03Outcome::Delegate(W13a03ScriptHandler::Script10)
        );
        assert_eq!(
            w13a03_apply_relation(0x16, 0, 0, &mut cells),
            W13a03Outcome::Ignored
        );
        assert_eq!(
            w13a03_apply_relation(0x1b, 0, 0, &mut cells),
            W13a03Outcome::Ignored
        );
        // A short matrix is rejected instead of indexing past the end.
        let mut short = [0u8; 8];
        assert_eq!(
            w13a03_apply_relation(1, 1, 0, &mut short),
            W13a03Outcome::Ignored
        );
    }

    #[test]
    fn text_budget_matches_0x00661b40() {
        assert_eq!(w13a03_text_budget(None), 0);
        assert_eq!(w13a03_text_budget(Some(b"")), 4000);
        assert_eq!(w13a03_text_budget(Some(&[b'a'; 80])), 4000);
        assert_eq!(w13a03_text_budget(Some(&[b'a'; 81])), 4050);
        assert_eq!(w13a03_text_budget(Some(b"abc\0ignored")), 4000);
        assert_eq!(w13a03_text_budget(Some(&[b'a'; 200])), 10_000);
    }

    #[test]
    fn region_query_matches_0x00661d10() {
        let areas = [sample_area(0x1234), sample_area(0x5678)];
        let query = w13a03_region_query(&areas, 1, 0x9abc, 0xdef0).unwrap();
        assert_eq!(query.area, sample_area(0x5678));
        assert_eq!(query.target, 0x9abc);
        assert_eq!(query.command, 0xdef0);
        assert_eq!(query.marker, 0x5678);
        assert_eq!(query.mode, 1);
        assert_eq!(w13a03_region_query(&areas, 2, 0, 0), None);
    }
}

// ===========================================================================
// Merged from src/wave15/agent01.rs
// ===========================================================================
// Wave 15 translation batch 01.
//
// Skipped (call-only wrappers, noreturn stubs or unresolved callee chains):
// FUN_00535520 (exception constructor), FUN_00596cf0, FUN_005c7cc0,
// FUN_005d5810, FUN_00635340, FUN_00635360, FUN_00635380 (pure thunks to
// the same callee), FUN_006384a0, FUN_00638940 (noreturn forwarders),
// FUN_006387c0, FUN_006388e0 (call sequences only).

/// Stride of one per-player record in the 0xfaa1f8 block.
pub const W15A01_PLAYER_STRIDE: usize = 0x4e8;

/// One step emitted by [`w15a01_dispatch_command`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W15a01Step {
    /// Accumulate the offset (and `hi`) into the player's position triple.
    AddOffset { code: u32, hi: u32 },
    /// Call `FUN_00635dd0(player, code)`.
    Prepare { code: u32 },
    /// Call `FUN_00634570(player, 1, code, count, arg)`.
    Emit { code: u32, count: i8 },
}

/// `0x006346b0`: maps a command byte to the sequence of prepare/emit steps
/// (0x2c/0x3e add an offset then emit 0x2b; 0x44 -> 0x43 twice; 0x3f -> 0x3d
/// twice; 0x67 -> 0x26 once).  `count` is `param_3 - 1`; `hi` is the
/// high byte of the packed argument word (the Ghidra view aliases it).
pub fn w15a01_dispatch_command(cmd: u32, hi: u32, param_3: i8) -> Vec<W15a01Step> {
    let count = param_3.wrapping_sub(1);
    match cmd {
        0x2c | 0x3e => vec![
            W15a01Step::AddOffset { code: 0x2b, hi },
            W15a01Step::Emit { code: 0x2b, count },
        ],
        0x44 => vec![
            W15a01Step::Prepare { code: 0x43 },
            W15a01Step::Emit { code: 0x43, count },
            W15a01Step::Prepare { code: 0x43 },
            W15a01Step::Emit { code: 0x43, count },
        ],
        0x3f => vec![
            W15a01Step::Prepare { code: 0x3d },
            W15a01Step::Emit { code: 0x3d, count },
            W15a01Step::Prepare { code: 0x3d },
            W15a01Step::Emit { code: 0x3d, count },
        ],
        0x67 => vec![
            W15a01Step::Prepare { code: 0x26 },
            W15a01Step::Emit { code: 0x26, count },
        ],
        _ => Vec::new(),
    }
}

/// `0x00634b60`: argument guard; the inner call at 634520 only runs when
/// the player index is below 8 and the second index below 0xe4.
pub fn w15a01_guard_634b60(player: u32, index: u32) -> bool {
    player < 8 && index < 0xe4
}

/// `0x00634b90`: same guard as the sibling 634b60 (player < 8, index < 0xe4).
pub fn w15a01_guard_634b90(player: u32, index: u32) -> bool {
    player < 8 && index < 0xe4
}

/// `0x006353a0`: same guard again (player < 8, index < 0xe4).
pub fn w15a01_guard_6353a0(player: u32, index: u32) -> bool {
    player < 8 && index < 0xe4
}

/// `0x00634f10`: updates a player record.  Returns false (no-op) when
/// `param_6 == 0` and flag 0x200 is set.  Otherwise clears bit 0 when
/// `param_5 == 0`, sets 0x40, adds `offset(param_3)` to x/y and the high
/// byte of `param_6` to z.  The caller then runs the emit/dispatch calls.
pub fn w15a01_apply_move(
    flags: &mut u16,
    pos: &mut [i32; 3],
    param_3: u32,
    param_5: i32,
    param_6: u32,
    offset: impl Fn(u32) -> (i32, i32),
) -> bool {
    if param_6 == 0 && (*flags & 0x200) != 0 {
        return false;
    }
    if param_5 == 0 {
        *flags &= 0xfffe;
    }
    *flags |= 0x40;
    let (dx, dy) = offset(param_3);
    pos[0] = pos[0].wrapping_add(dx);
    pos[1] = pos[1].wrapping_add(dy);
    pos[2] = pos[2].wrapping_add((param_6 >> 24) as i32);
    true
}

/// `0x00635060`: returns the mode word (3, 7 or 4) to pass on, or `None`
/// when the call is suppressed (recent stamp within 30 ticks, different
/// owner, and code != 0x8c).  `table_entry` is the 0xf8fa08 flag word.
pub fn w15a01_select_mode(
    stamp: u32,
    now: u32,
    owner_matches: bool,
    code: i32,
    table_entry: u32,
) -> Option<u32> {
    if stamp != 0 && now.wrapping_sub(stamp) < 0x1e && !owner_matches && code != 0x8c {
        return None;
    }
    if table_entry & 8 == 0 {
        Some(((table_entry & 2) * 2) | 3)
    } else {
        Some(4)
    }
}

/// `0x00635d90`: sets (`on != 0`) or clears bit 0x400 of the player flags.
pub fn w15a01_set_flag_400(flags: u16, on: bool) -> u16 {
    if on { flags | 0x400 } else { flags & 0xfbff }
}

/// `0x00635ed0`: zeroes the 0x80-byte block at record offset 0x18 for
/// `player < 8`; returns whether it ran.  `block` is the whole table.
pub fn w15a01_clear_player_block(block: &mut [u8], player: u32) -> bool {
    if player >= 8 {
        return false;
    }
    let start = player as usize * W15A01_PLAYER_STRIDE + 0x18;
    match block.get_mut(start..start + 0x80) {
        Some(s) => {
            s.fill(0);
            true
        }
        None => false,
    }
}

/// `0x00590400`: returns whether the unit matches param (`wanted`).  `unit`
/// is the raw record; `c1`/`c2` are results of the helper calls at
/// 5baa30 / 5d2160.  Returns `None` where the original calls a noreturn
/// stub (helper 5b64c0 false).
pub fn w15a01_unit_matches(
    unit: &[u8],
    wanted: u16,
    helper_ok: bool,
    c1: bool,
    c2: bool,
) -> Option<bool> {
    if !helper_ok || unit.len() < 0xf0 {
        return None;
    }
    let kind = unit[0x4d];
    let rd32 = |o: usize| u32::from_le_bytes([unit[o], unit[o + 1], unit[o + 2], unit[o + 3]]);
    let blocked = if matches!(kind, 0x1e | 0x22 | 0x21) || c1 {
        rd32(0x5c) != 0
    } else if unit[0xa6] == b'&' || unit[0xa6] == b'?' || c2 {
        rd32(0xec) != 0
    } else {
        false
    };
    if blocked {
        return Some(false);
    }
    let o = 0x98 + (unit[0xa4] as usize % 5) * 2;
    let v = u16::from_le_bytes([unit[o], unit[o + 1]]);
    Some(v == wanted && matches!(kind, 0x1e | 0x1f | 0x46 | 0x1a | 0x19))
}

#[cfg(test)]
mod tests_wave15_agent01 {
    use super::*;

    #[test]
    fn dispatch() {
        assert_eq!(w15a01_dispatch_command(0x44, 0, 5).len(), 4);
        assert_eq!(
            w15a01_dispatch_command(0x67, 0, 1),
            vec![
                W15a01Step::Prepare { code: 0x26 },
                W15a01Step::Emit {
                    code: 0x26,
                    count: 0
                }
            ]
        );
        assert_eq!(
            w15a01_dispatch_command(0x2c, 5, 3)[0],
            W15a01Step::AddOffset { code: 0x2b, hi: 5 }
        );
        assert!(w15a01_dispatch_command(1, 0, 1).is_empty());
    }

    #[test]
    fn guards() {
        assert!(w15a01_guard_634b60(7, 0xe3));
        assert!(!w15a01_guard_634b90(8, 0));
        assert!(!w15a01_guard_6353a0(0, 0xe4));
    }

    #[test]
    fn apply_move() {
        let mut f = 0x201u16;
        let mut p = [1, 2, 3];
        assert!(!w15a01_apply_move(&mut f, &mut p, 0, 0, 0, |_| (1, 1)));
        assert!(w15a01_apply_move(&mut f, &mut p, 0, 0, 0x0200_0000, |_| (
            10, 20
        )));
        assert_eq!(f, 0x240);
        assert_eq!(p, [11, 22, 5]);
    }

    #[test]
    fn mode() {
        assert_eq!(w15a01_select_mode(0, 100, false, 1, 0), Some(3));
        assert_eq!(w15a01_select_mode(0, 100, false, 1, 2), Some(7));
        assert_eq!(w15a01_select_mode(0, 100, false, 1, 8), Some(4));
        assert_eq!(w15a01_select_mode(90, 100, false, 1, 0), None);
        assert_eq!(w15a01_select_mode(90, 100, false, 0x8c, 0), Some(3));
    }

    #[test]
    fn flags_and_clear() {
        assert_eq!(w15a01_set_flag_400(0, true), 0x400);
        assert_eq!(w15a01_set_flag_400(0xffff, false), 0xfbff);
        let mut b = vec![0xffu8; 8 * W15A01_PLAYER_STRIDE];
        assert!(w15a01_clear_player_block(&mut b, 1));
        assert_eq!(b[W15A01_PLAYER_STRIDE + 0x18], 0);
        assert_eq!(b[W15A01_PLAYER_STRIDE + 0x98], 0xff);
        assert!(!w15a01_clear_player_block(&mut b, 8));
    }

    #[test]
    fn unit_match() {
        let mut u = vec![0u8; 0x100];
        u[0x4d] = 0x1e;
        u[0x98] = 7;
        assert_eq!(w15a01_unit_matches(&u, 7, true, false, false), Some(true));
        u[0x5c] = 1;
        assert_eq!(w15a01_unit_matches(&u, 7, true, false, false), Some(false));
        assert_eq!(w15a01_unit_matches(&u, 7, false, false, false), None);
    }
}

// ===========================================================================
// Merged from src/wave15/agent02.rs
// ===========================================================================
// Wave 15 translation batch 02.
//
// Small pure helpers: a per-player table lookup, a unit-record cost table
// reader, and two callback-driven predicates.  Skipped rows (written as
// `FUN_00xxxxxx`) are listed at the bottom of this header:
//
// - FUN_00638b00, FUN_00638c00, FUN_0063a3c0: tail-call a non-returning stub
// - FUN_0063a9a0: only non-returning calls, no recoverable result
// - FUN_006520e0, FUN_00652250, FUN_00653190: end in non-returning calls
// - FUN_006523f0, FUN_00655ac0, FUN_006551a0, FUN_006565b0: depend on opaque
//   globals / many unmodelled callees, control flow does not return normally
// - FUN_0065ee50, FUN_00713200: string/exception glue ending in noreturn
// - FUN_00687d90: file writer with noreturn error paths
// - FUN_0073f120: single global store (DAT_011a2434), no state to model

/// `0x00695be0`: reads the u32 at index `col + row * 12` of the per-player
/// table that starts at offset `0x3234` of the global state block.  `table`
/// is that table viewed as u32 words.  Returns `None` when out of range.
pub fn w15a02_player_table_value(table: &[u32], row: u16, col: u8) -> Option<u32> {
    let idx = usize::from(col) + usize::from(row) * 12;
    table.get(idx).copied()
}

/// One 0x30-byte record of the table based at `DAT_011ac918`; only the
/// fields read by the helpers below are kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct W15a02Record {
    /// Signed word at record offset `0x0`.
    pub word0: i16,
    /// Byte at record offset `0x4`.
    pub byte4: u8,
    /// Byte at record offset `0x5`.
    pub byte5: u8,
    /// Dword at record offset `0x8`.
    pub dword8: u32,
}

/// `0x0076a4d0`: returns the signed word at the start of record `id`
/// (valid ids are `1..0xfa`), else 0.
pub fn w15a02_record_word(rec: &W15a02Record, id: u32) -> i32 {
    if id != 0 && id < 0xfa {
        i32::from(rec.word0)
    } else {
        0
    }
}

/// `0x00769aa0`: cost estimate for record `id` (valid `1..0xfa`, else 0).
/// `limit` is the result of the callee queried with the second argument.
/// With `c = dword8 / 500`: if `limit <= c` the base is `min(c, byte4 * 2)`,
/// otherwise it is `byte4`; the result adds `byte5 * 3`.
pub fn w15a02_record_cost(rec: &W15a02Record, id: u32, limit: u32) -> i32 {
    if id == 0 || id >= 0xfa {
        return 0;
    }
    let c = rec.dword8 / 500;
    let a = u32::from(rec.byte4);
    let base = if limit <= c { c.min(a * 2) } else { a };
    (base + u32::from(rec.byte5) * 3) as i32
}

/// `0x007693f0`: guarded call.  When `guarded` and `is_blocked()` reports
/// the player blocked, returns 0.  Otherwise calls `inner(arg)` where `arg`
/// is `0x28` if `kind as i16 == 0x7c`, else `param6`; if the inner result is
/// 0 and `guarded`, `on_fail()` runs.  Returns the inner result.
pub fn w15a02_guarded_call(
    kind: u32,
    param6: u32,
    guarded: bool,
    mut is_blocked: impl FnMut() -> bool,
    mut on_fail: impl FnMut(),
    mut inner: impl FnMut(u32) -> i32,
) -> i32 {
    if guarded && is_blocked() {
        return 0;
    }
    let arg = if kind as u16 == 0x7c { 0x28 } else { param6 };
    let r = inner(arg);
    if r == 0 && guarded {
        on_fail();
    }
    r
}

/// `0x00657a00`: walks a zero-terminated list of unit owner bytes (the byte
/// at unit offset `0x4c`) and invokes `visit` for every entry whose owner
/// equals `player`.  Returns the number of visits.
pub fn w15a02_visit_owned(owners: &[u8], player: u32, mut visit: impl FnMut(usize)) -> usize {
    let mut n = 0;
    for (i, &o) in owners.iter().enumerate() {
        if u32::from(o) == player {
            visit(i);
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests_wave15_agent02 {
    use super::*;

    #[test]
    fn table_lookup() {
        let t: Vec<u32> = (0..36).collect();
        assert_eq!(w15a02_player_table_value(&t, 2, 5), Some(29));
        assert_eq!(w15a02_player_table_value(&t, 3, 0), None);
    }

    #[test]
    fn record_helpers() {
        let r = W15a02Record {
            word0: -7,
            byte4: 10,
            byte5: 2,
            dword8: 5000,
        };
        assert_eq!(w15a02_record_word(&r, 0), 0);
        assert_eq!(w15a02_record_word(&r, 0xfa), 0);
        assert_eq!(w15a02_record_word(&r, 1), -7);
        // c = 10, limit <= c -> min(10, 20) + 6
        assert_eq!(w15a02_record_cost(&r, 1, 10), 16);
        assert_eq!(w15a02_record_cost(&r, 1, 11), 16);
        let r2 = W15a02Record {
            byte4: 3,
            dword8: 5000,
            byte5: 0,
            ..r
        };
        assert_eq!(w15a02_record_cost(&r2, 1, 5), 6);
        assert_eq!(w15a02_record_cost(&r2, 1, 50), 3);
        assert_eq!(w15a02_record_cost(&r2, 0, 5), 0);
    }

    #[test]
    fn guarded() {
        let mut failed = 0;
        let mut seen = 0;
        let r = w15a02_guarded_call(
            0x7c,
            9,
            true,
            || false,
            || failed += 1,
            |a| {
                seen = a;
                0
            },
        );
        assert_eq!((r, seen, failed), (0, 0x28, 1));
        let r = w15a02_guarded_call(1, 9, true, || true, || {}, |_| 5);
        assert_eq!(r, 0);
        let r = w15a02_guarded_call(1, 9, false, || true, || {}, |a| a as i32);
        assert_eq!(r, 9);
    }

    #[test]
    fn visit() {
        let mut hits = vec![];
        let n = w15a02_visit_owned(&[1, 2, 1], 1, |i| hits.push(i));
        assert_eq!((n, hits), (2, vec![0, 2]));
    }
}

// ===========================================================================
// Merged from src/wave16/agent01.rs
// ===========================================================================
// Wave 16 translation batch 01.
//
// Pure helpers; unresolved callees are closure parameters.

/// Resource-style triple used by the per-player accumulators (+0x1f8/+0x1fc/+0x200 of
/// the 0x4e8-byte player record).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct W16a01Tally {
    pub a: i32,
    pub b: i32,
    pub c: u32,
}

/// Slot block of a record: five u16 slots at +0x98 and a start index byte at +0xa4.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct W16a01SlotBlock {
    pub slots: [u16; 5],
    pub start: u8,
}

/// Wrapped search for slot value `needle` beginning at `start` (index >= 5 wraps to 0).
fn w16a01_find_slot(b: &W16a01SlotBlock, needle: u16) -> Option<usize> {
    let mut idx = b.start as usize;
    for _ in 0..5 {
        if idx >= 5 {
            idx = 0;
        }
        if b.slots[idx] == needle {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

/// 0x0058e0e0: replace the first free (0xe4) slot, scanning from the start index.
/// `commit(slot_is_start)` is the unresolved reservation call (returns false on
/// failure); `flag_bit0` is bit 0 of the per-kind table entry for `value`;
/// `on_no_flag` / `on_done` are the trailing callees.
pub fn w16a01_replace_empty_slot(
    b: &mut W16a01SlotBlock,
    value: u16,
    flag_bit0: bool,
    mut commit: impl FnMut(bool) -> bool,
    mut on_no_flag: impl FnMut(),
    mut on_done: impl FnMut(),
) -> bool {
    let Some(idx) = w16a01_find_slot(b, 0xe4) else {
        return false;
    };
    if !commit(idx == b.start as usize) {
        return false;
    }
    b.slots[idx] = value;
    if !flag_bit0 {
        on_no_flag();
    }
    on_done();
    true
}

/// 0x005b6680: store a u16 at +0xc4.
pub fn w16a01_set_c4(field: &mut u16, v: u16) {
    *field = v;
}

/// 0x005b7c70: set byte +0xa6 and clear +0xe8, +0xec (u32) and +0xe2 (u8).
pub fn w16a01_set_a6_reset(a6: &mut u8, e8: &mut u32, ec: &mut u32, e2: &mut u8, v: u8) {
    *a6 = v;
    *e8 = 0;
    *ec = 0;
    *e2 = 0;
}

/// 0x005baa30: true when byte +0x4d is 0x1f, `pred1()` holds, +0x5c is non-zero and
/// `pred2()` is false.
pub fn w16a01_state1f_ready(
    b4d: u8,
    f5c: u32,
    pred1: impl FnOnce() -> bool,
    pred2: impl FnOnce() -> bool,
) -> bool {
    b4d == 0x1f && pred1() && f5c != 0 && !pred2()
}

/// 0x006358d0: bit 4 of the player flags word, 0 when player >= 8.
pub fn w16a01_player_flag4(player: u32, flags: u16) -> u16 {
    if player > 7 { 0 } else { (flags >> 4) & 1 }
}

/// 0x00635eb0: inverted bit 2 of the player flags word.
pub fn w16a01_player_not_flag2(flags: u16) -> u16 {
    !(flags >> 2) & 1
}

/// 0x00634eb0: add the (a, b, c) triple for `kind` into the tally. Returns the
/// arguments for the follow-up call (event code 8).
pub fn w16a01_tally_add(t: &mut W16a01Tally, triple: (i32, i32, u8)) -> u32 {
    t.a = t.a.wrapping_add(triple.0);
    t.b = t.b.wrapping_add(triple.1);
    t.c = t.c.wrapping_add(triple.2 as u32);
    8
}

/// 0x00634fc0: rate-limited tally. `last` is +0x41c of the player record, `now` the
/// frame counter, `kind` the u16 at +0x10 of the subject. Returns the effective kind
/// (0x1e maps to 5) when applied; the caller then issues events with code 2 and
/// weight 0x32. `None` when still inside the 0x1d-frame window.
pub fn w16a01_tally_rate_limited(
    t: &mut W16a01Tally,
    last: u32,
    now: u32,
    kind: u16,
    triple_of: impl FnOnce(u16) -> (i32, i32, u8),
) -> Option<u16> {
    if last != 0 && now.wrapping_sub(last) <= 0x1d {
        return None;
    }
    let k = if kind == 0x1e { 5 } else { kind };
    w16a01_tally_add(t, triple_of(k));
    Some(k)
}

/// 0x005c7a20: mask chosen for the lookup; 0x120 when `alt`, else 0x400 doubled to
/// 0x800 when `flag` (callee result) is set.
pub fn w16a01_mask_c7a20(alt: bool, flag: bool) -> u32 {
    if alt {
        0x120
    } else {
        (if flag { 0x400 } else { 0 }) + 0x400
    }
}

/// 0x005c79a3: mask 0x800 or 0x1000 depending on `flag`.
pub fn w16a01_mask_c79a3(flag: bool) -> u32 {
    (if flag { 0x800 } else { 0 }) + 0x800
}

/// 0x005c79d0: base mask is 0x120 when `alt`, else 0x800. `lookup(mask, which)` is
/// the unresolved search (which 0 = first handler, 1 = fallback); the fallback runs
/// only when `!alt` and the first returned 0.
pub fn w16a01_c79d0(alt: bool, mut lookup: impl FnMut(u32, u8) -> i32) {
    let mask = if alt { 0x120 } else { 0x800 };
    if lookup(mask, 0) == 0 && !alt {
        lookup(mask, 1);
    }
}

/// 0x0053bfd0: argument shuffle (p1, p3, p2, p3) for the wrapped call.
pub fn w16a01_args_bfd0(p1: u32, p2: u32, p3: u32) -> (u32, u32, u32, u32) {
    (p1, p3, p2, p3)
}

/// 0x006574c5: run `act` only when `pred()` is zero.
pub fn w16a01_unless(pred: impl FnOnce() -> i32, act: impl FnOnce()) {
    if pred() == 0 {
        act();
    }
}

// FUN_0054b6e0: noreturn call
// FUN_005525a0: SEH + noreturn path
// FUN_00597290: red-black tree erase fixup, STL, skipped for size/cost
// FUN_005c7870: bare call
// FUN_00636960: noreturn call
// FUN_00657260: opaque callees, stack-cookie noreturn
// FUN_00658240: noreturn, opaque callees

#[cfg(test)]
mod tests_wave16_agent01 {
    use super::*;

    #[test]
    fn slot_replace() {
        let mut b = W16a01SlotBlock {
            slots: [1, 0xe4, 2, 0xe4, 3],
            start: 2,
        };
        let n = std::cell::Cell::new(0);
        assert!(w16a01_replace_empty_slot(
            &mut b,
            9,
            false,
            |s| !s,
            || n.set(n.get() + 1),
            || n.set(n.get() + 10)
        ));
        assert_eq!(b.slots, [1, 0xe4, 2, 9, 3]);
        assert_eq!(n.get(), 11);
        let mut full = W16a01SlotBlock {
            slots: [1; 5],
            start: 0,
        };
        assert!(!w16a01_replace_empty_slot(
            &mut full,
            9,
            true,
            |_| true,
            || {},
            || {}
        ));
        let mut r = W16a01SlotBlock {
            slots: [0xe4, 1, 1, 1, 1],
            start: 0,
        };
        assert!(!w16a01_replace_empty_slot(
            &mut r,
            9,
            true,
            |s| !s,
            || {},
            || {}
        ));
        assert_eq!(r.slots[0], 0xe4);
    }

    #[test]
    fn flags() {
        assert_eq!(w16a01_player_flag4(8, 0xff), 0);
        assert_eq!(w16a01_player_flag4(3, 0x10), 1);
        assert_eq!(w16a01_player_not_flag2(0b100), 0);
        assert_eq!(w16a01_player_not_flag2(0), 1);
    }

    #[test]
    fn tally() {
        let mut t = W16a01Tally::default();
        assert_eq!(
            w16a01_tally_rate_limited(&mut t, 100, 110, 5, |_| (1, 2, 3)),
            None
        );
        let k = w16a01_tally_rate_limited(&mut t, 100, 200, 0x1e, |k| (k as i32, 2, 3));
        assert_eq!(k, Some(5));
        assert_eq!(t, W16a01Tally { a: 5, b: 2, c: 3 });
        assert_eq!(w16a01_tally_add(&mut t, (1, 1, 1)), 8);
    }

    #[test]
    fn misc() {
        assert!(w16a01_state1f_ready(0x1f, 1, || true, || false));
        assert!(!w16a01_state1f_ready(0x1e, 1, || true, || false));
        assert_eq!(w16a01_mask_c7a20(true, true), 0x120);
        assert_eq!(w16a01_mask_c7a20(false, true), 0x800);
        assert_eq!(w16a01_mask_c7a20(false, false), 0x400);
        assert_eq!(w16a01_mask_c79a3(true), 0x1000);
        let mut calls = vec![];
        w16a01_c79d0(false, |m, w| {
            calls.push((m, w));
            0
        });
        assert_eq!(calls, vec![(0x800, 0), (0x800, 1)]);
        assert_eq!(w16a01_args_bfd0(1, 2, 3), (1, 3, 2, 3));
        let (mut a, mut e8, mut ec, mut e2) = (0, 5, 6, 7);
        w16a01_set_a6_reset(&mut a, &mut e8, &mut ec, &mut e2, 4);
        assert_eq!((a, e8, ec, e2), (4, 0, 0, 0));
        let mut f = 0;
        w16a01_set_c4(&mut f, 7);
        assert_eq!(f, 7);
        let mut hit = false;
        w16a01_unless(|| 0, || hit = true);
        assert!(hit);
    }
}

// ===========================================================================
// Merged from src/wave17/agent01.rs
// ===========================================================================
// Wave 17 translation batch 01.
//
// Window: queue, module `ai_recovered.rs`.  Pure helpers; unresolved
// callees and globals are caller-supplied parameters.

/// `0x00695df0`: classifies a 16-bit id by its flag byte (flag table
/// `DAT_00f91fe8`, one byte per id; the caller passes `flags[id]`).  Bit 0
/// gives `hi << 8`, bit 1 gives `hi << 8 | 1`, otherwise
/// `((hi << 8 | (bit2 == 0)) * 2) + 2`.
pub fn w17a01_flag_class(id: u16, flags: &[u8]) -> u32 {
    let b = flags.get(id as usize).copied().unwrap_or(0);
    let hi = (id >> 8) as u32;
    if b & 1 != 0 {
        return hi << 8;
    }
    if b & 2 != 0 {
        return (hi << 8) | 1;
    }
    (((hi << 8) | ((b & 4) == 0) as u32).wrapping_mul(2)).wrapping_add(2)
}

/// `0x00695930`: effective value for `(id, player)`.  `flags` is the
/// `DAT_00f91fe8` table; `table` is the decoded global block (offsets
/// 0x3054/0x3084 for flag 1, 0x30e4/0x3114 for flag 2, 0x3174/0x31a4 for
/// flag 4, stride 4 per `param_2 & 0xff`).  `base` is `FUN_00695a70`
/// evaluated through `base_fn`; `override_flag` is `DAT_01165d00 & 0x20000`.
pub fn w17a01_effective_value(
    id: u16,
    param_2: u32,
    flags: &[u8],
    table: &[u8],
    override_flag: bool,
    mut base_fn: impl FnMut(u32, u32) -> u32,
) -> i32 {
    let rd = |off: usize| -> u32 {
        table
            .get(off..off + 4)
            .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
            .unwrap_or(0)
    };
    let b = flags.get(id as usize).copied().unwrap_or(0);
    let slot = ((param_2 & 0xff) * 4) as usize;
    let (mut v, sub) = if b & 1 != 0 {
        (None, Some((0x3054, 0x3084)))
    } else if b & 2 != 0 {
        (None, Some((0x30e4, 0x3114)))
    } else if b & 4 != 0 {
        (None, Some((0x3174, 0x31a4)))
    } else {
        (Some(0i32), None)
    };
    let mut sub_v = 0i32;
    if let Some((cap_off, sub_off)) = sub {
        let cap = rd(cap_off + slot);
        let a = base_fn(id as u32, param_2);
        v = Some(if a < cap {
            base_fn(id as u32, param_2)
        } else {
            cap
        } as i32);
        sub_v = rd(sub_off + slot) as i32;
    }
    let mut out = v.unwrap_or(0);
    if override_flag {
        out = base_fn(id as u32, param_2) as i32;
    }
    out.wrapping_sub(sub_v)
}

/// `0x006ae4e0`: records a "SC" tag marker.  `state` is the 8-byte record
/// at `DAT_01165f54` (byte 0 flag, dword at +4); `active` is
/// `DAT_01165f58 != 0`.  `src` is the source block: dword at 0, tag bytes
/// at 0x1e/0x1f.
pub fn w17a01_set_sc_marker(active: bool, state: Option<&mut [u8]>, src: &[u8]) {
    if !active {
        return;
    }
    let Some(st) = state else { return };
    if st.len() < 8 || src.len() < 0x20 {
        return;
    }
    let is_sc = src[0x1e] == b'S' && src[0x1f] == b'C';
    st[0] = is_sc as u8;
    if is_sc {
        st[4..8].copy_from_slice(&src[0..4]);
    }
}

/// `0x006ae520`: clears the marker flag byte when the record exists and
/// `active` (`DAT_01165f58 != 0`).
pub fn w17a01_clear_marker(active: bool, state: Option<&mut [u8]>) {
    if active
        && let Some(st) = state
        && !st.is_empty()
    {
        st[0] = 0;
    }
}

/// `0x0070fe00`: returns true when `param_2 == 0x500` and the callee
/// (`FUN_00984770` with the value of `FUN_00661410`) returns non-zero.
pub fn w17a01_check_500(param_2: i32, mut probe: impl FnMut() -> bool) -> bool {
    param_2 == 0x500 && probe()
}

/// `0x00715a40`: calls the copy helper `FUN_00c671d0` and returns its
/// first argument unchanged.
pub fn w17a01_pass_through(param_1: u32, call: impl FnOnce(u32)) -> u32 {
    call(param_1);
    param_1
}

/// `0x0076a310`: byte at `0x011ac91d + index * 0x30`; `rows` starts at
/// that address.
pub fn w17a01_row_byte(rows: &[u8], index: usize) -> u8 {
    rows.get(index * 0x30).copied().unwrap_or(0)
}

/// `0x007aaf50`: sets the global flag `DAT_011b8700` to 1.
pub fn w17a01_set_flag(flag: &mut u32) {
    *flag = 1;
}

/// `0x0094fec0`: stores `v` into the byte global `DAT_011cacb5`.
pub fn w17a01_store_byte(slot: &mut u8, v: u8) {
    *slot = v;
}

#[cfg(test)]
mod tests_wave17_agent01 {
    use super::*;

    #[test]
    fn flag_class() {
        let mut f = vec![0u8; 0x300];
        f[0x201] = 1;
        f[0x202] = 2;
        f[0x203] = 4;
        assert_eq!(w17a01_flag_class(0x201, &f), 0x200);
        assert_eq!(w17a01_flag_class(0x202, &f), 0x201);
        assert_eq!(w17a01_flag_class(0x203, &f), (0x200 * 2 + 2));
        assert_eq!(w17a01_flag_class(0x200, &f), (0x200 | 1) * 2 + 2);
    }

    #[test]
    fn effective() {
        let mut t = vec![0u8; 0x3200];
        t[0x3054 + 8..0x3054 + 12].copy_from_slice(&10u32.to_le_bytes());
        t[0x3084 + 8..0x3084 + 12].copy_from_slice(&3u32.to_le_bytes());
        let mut f = vec![0u8; 4];
        f[1] = 1;
        assert_eq!(w17a01_effective_value(1, 2, &f, &t, false, |_, _| 5), 2);
        assert_eq!(w17a01_effective_value(1, 2, &f, &t, false, |_, _| 50), 7);
        assert_eq!(w17a01_effective_value(1, 2, &f, &t, true, |_, _| 50), 47);
        assert_eq!(w17a01_effective_value(0, 2, &f, &t, false, |_, _| 50), 0);
    }

    #[test]
    fn marker() {
        let mut st = [9u8; 8];
        let mut src = [0u8; 0x20];
        src[..4].copy_from_slice(&7u32.to_le_bytes());
        src[0x1e] = b'S';
        src[0x1f] = b'C';
        w17a01_set_sc_marker(true, Some(&mut st), &src);
        assert_eq!(st[0], 1);
        assert_eq!(st[4], 7);
        src[0x1f] = b'X';
        w17a01_set_sc_marker(true, Some(&mut st), &src);
        assert_eq!(st[0], 0);
        w17a01_set_sc_marker(false, Some(&mut st), &src);
        st[0] = 5;
        w17a01_clear_marker(true, Some(&mut st));
        assert_eq!(st[0], 0);
    }

    #[test]
    fn small() {
        assert!(w17a01_check_500(0x500, || true));
        assert!(!w17a01_check_500(0x501, || true));
        assert_eq!(w17a01_pass_through(7, |_| {}), 7);
        let mut r = vec![0u8; 0x60];
        r[0x30] = 4;
        assert_eq!(w17a01_row_byte(&r, 1), 4);
        let mut fl = 0;
        w17a01_set_flag(&mut fl);
        assert_eq!(fl, 1);
        let mut b = 0;
        w17a01_store_byte(&mut b, 3);
        assert_eq!(b, 3);
    }
}
