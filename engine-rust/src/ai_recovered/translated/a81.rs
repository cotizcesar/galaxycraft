#![allow(unused_imports)]
use super::super::*;

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
