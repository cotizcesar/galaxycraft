#![allow(unused_imports)]
use super::super::*;

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
