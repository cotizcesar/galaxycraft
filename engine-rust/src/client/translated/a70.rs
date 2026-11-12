#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent70.rs
// ===========================================================================
// Translation batch 70.  Only this file is edited by the batch.
//
// Batch 70 owns `0x0061e000..=0x0061ffff`.  The ledger lists 72
// `decompiled_uncited` rows in that window; 30 are translated below and the
// 42 rows that are not are listed at the end with their reason.
//
// The window is one C++ support library, not simulation code: a circular
// doubly-linked list, a chained hash table, the small-string record the hash
// values use, and the locale-id predicates and language-tag table that sit on
// top of it.  Every recovered body in this window addresses its state through
// `ECX`/`EAX` (there is no stack argument for the object), so the translations
// are value types plus free functions; no `sim` record gains a field.
//
// Recovered layouts:
//
// | layout               | words                                                        |
// |----------------------|--------------------------------------------------------------|
// | list header          | `+0x00` first, `+0x04` last, `+0x08` count                     |
// | list node (`0x18`)   | `+0x00` next, `+0x04` prev, `+0x08..=+0x17` four payload words |
// | hash node            | `+0x00` next, `+0x04..=+0x13` 16-byte key, `+0x1c` value       |
// | inline string record | 16 bytes; `+0x0d` terminator, `+0x0e` tag word                 |
// | vector header        | `+0x00` data, `+0x04` count, `+0x08` capacity + top flag       |
//
// Address to function:
//
// * `0x0061e170` [`A70List::a70_new`]
// * `0x0061e1ed` [`A70List::a70_push_front`]
// * `0x0061ec30` [`a70_seasons_index_report`]
// * `0x0061ef80` [`a70_string_record`]
// * `0x0061f040` [`a70_hash_lookup`]
// * `0x0061f0e0` [`a70_move_assign`]
// * `0x0061f177` [`a70_element_array_layout`]
// * `0x0061f260` [`a70_find_link`]
// * `0x0061f360` [`a70_bucket_target`]
// * `0x0061f406` [`a70_rehash`]
// * `0x0061f4b0` [`a70_node_eq`]
// * `0x0061f4d0` [`a70_node_ne`]
// * `0x0061f4f0` [`a70_store`]
// * `0x0061f500` [`a70_pair_report_args`]
// * `0x0061f630` [`a70_language_tag`]
// * `0x0061f820` [`A70LocaleIds::a70_not_reserved`]
// * `0x0061f850` [`A70LocaleIds::a70_is_00faa12c`]
// * `0x0061f860` [`A70LocaleIds::a70_is_00faa118`]
// * `0x0061f870` [`A70LocaleIds::a70_in_00faa118_or_00faa148`]
// * `0x0061f890` [`A70LocaleIds::a70_is_00faa148`]
// * `0x0061f8a0` [`A70LocaleIds::a70_is_00faa14c`]
// * `0x0061f8b0` [`A70LocaleIds::a70_is_00faa140`]
// * `0x0061f8c0` [`a70_parse_locale_code`]
// * `0x0061f96e` [`a70_parse_locale_flag`]
// * `0x0061fa94` [`a70_noop`]
// * `0x0061fc30` [`a70_expand_ids_count10`]
// * `0x0061fd10` [`a70_expand_ids_count04`]
// * `0x0061fdf0` [`a70_function_object_4f30617c`]
// * `0x0061fe10` [`a70_function_object_dbfea748`]
// * `0x0061fe30` [`a70_id_or_default`]
//
// Several addresses in this window are `0x1c`-byte-element vector helpers.
// Element size `0x1c` is recovered from `0x0061f177`
// (`(n * 8 - n) * 4 == 0x1c * n`), from the `uVar9 * 4` / `uVar6 * 0x1c`
// strides in `0x0061f040`'s neighbourhood and from the `puVar8 = puVar8 + 7`
// walks.  Only the header arithmetic is reproduced; the per-element inline
// string move the shipped loop performs is not turned into extra state.

// ---------------------------------------------------------------------------
// Circular doubly-linked list: 0x0061e170 and 0x0061e1ed
// ---------------------------------------------------------------------------

/// One `0x18`-byte node of the list built by `0x0061e1ed`.
///
/// The shipped allocator call is `[vtable + 4](0x18, 8)`: six dwords,
/// alignment 8.  The four payload words are copied from the caller's stack
/// copy (`[ebp - 0x30 .. ebp - 0x24]`), not from a node already in the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A70ListNode {
    /// `+0x00`: next node index; the sentinel is index 0.
    pub next: usize,
    /// `+0x04`: previous node index; the sentinel is index 0.
    pub prev: usize,
    /// `+0x08..=+0x17`: the four copied payload dwords.
    pub payload: [u32; 4],
}

/// Circular list with a sentinel header.
///
/// `0x0061e170` writes the three header words `(this, this, 0)` at `+0x00`,
/// `+0x04` and `+0x08`; the same three stores are repeated by the callee
/// `FUN_0061f2c0`, whose only additional work is the not-recovered destruction
/// of a non-empty old list.  For the freshly constructed header that tail is
/// a no-op, so the constructor is exactly the self-linked empty list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A70List {
    /// Node storage.  Index 0 is the sentinel and is always present.
    pub nodes: Vec<A70ListNode>,
    /// `+0x08` of the header.
    pub count: u32,
}

impl A70List {
    /// `0x0061e170`: construct the empty header.
    ///
    /// `*(int *)param = param; *(int *)(param + 4) = param;
    /// *(undefined4 *)(param + 8) = 0;` followed by the `FUN_0061f2c0` tail that
    /// repeats the same three stores.
    pub fn a70_new() -> Self {
        A70List {
            nodes: vec![A70ListNode {
                next: 0,
                prev: 0,
                payload: [0; 4],
            }],
            count: 0,
        }
    }

    /// `0x0061e1ed`: allocate one `0x18`-byte node and insert it at the front.
    ///
    /// The shipped body, with `EDI` the sentinel address and `*EDI` the old
    /// first link:
    ///
    /// ```text
    /// node = alloc(0x18, 8)
    /// node[2..5] = payload
    /// node[0] = *EDI          // next = old first
    /// node[1] = EDI           // prev = sentinel
    /// *EDI = node             // sentinel -> first
    /// node->next->prev = node
    /// EDI[2] += 1             // count
    /// ```
    ///
    /// Returns the new node's index.
    pub fn a70_push_front(&mut self, payload: [u32; 4]) -> usize {
        let old_first = self.nodes[0].next;
        let index = self.nodes.len();
        self.nodes.push(A70ListNode {
            next: old_first,
            prev: 0,
            payload,
        });
        self.nodes[0].next = index;
        self.nodes[old_first].prev = index;
        self.count = self.count.wrapping_add(1);
        index
    }

    /// The payload sequence from the sentinel's next link to the sentinel.
    pub fn a70_snapshot(&self) -> Vec<[u32; 4]> {
        let mut out = Vec::new();
        let mut cur = self.nodes[0].next;
        while cur != 0 {
            out.push(self.nodes[cur].payload);
            cur = self.nodes[cur].next;
        }
        out
    }
}

// ---------------------------------------------------------------------------
// SEH catch body: 0x0061ec30
// ---------------------------------------------------------------------------

/// Message literal passed by `0x0061ec30`.
pub const A70_SEASONS_INDEX_MESSAGE: &str = "OnSeasonsIndexFileUnreadable";

/// Size of the frame buffer passed as the second argument by `0x0061ec30`
/// (`unaff_EBP - 0x169`).
pub const A70_SEASONS_INDEX_CONTEXT_BYTES: usize = 0x169;

/// `0x0061ec30`: the `Catch@0061ec30` row reports the seasons index failure.
///
/// `FUN_0053a640("OnSeasonsIndexFileUnreadable", unaff_EBP + -0x169);` — the
/// message pointer is the literal's address; the second argument points into
/// the handler frame, so only its size is recoverable.
pub fn a70_seasons_index_report() -> (&'static str, usize) {
    (A70_SEASONS_INDEX_MESSAGE, A70_SEASONS_INDEX_CONTEXT_BYTES)
}

// ---------------------------------------------------------------------------
// 16-byte inline string record: 0x0061ef80
// ---------------------------------------------------------------------------

/// Tag word stored at `+0x0e` for the inline form (`len < 0x0e`).
pub const A70_RECORD_TAG_INLINE: u16 = 0x1c05;

/// Tag word stored at `+0x0e` for the copied heap form.
pub const A70_RECORD_TAG_COPIED: u16 = 0x0c05;

/// Tag word stored at `+0x0e` for the borrowed form (`param_3 == 0`).
pub const A70_RECORD_TAG_BORROWED: u16 = 0x0405;

/// Inline capacity, in bytes: `0x0d` is the last inline byte, so the length
/// test is `len < 0x0e` and the terminator index is `0x0d - len`.
pub const A70_RECORD_INLINE_CAPACITY: u8 = 0x0d;

/// Where the bytes of a [`A70StringRecord`] live.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum A70RecordPayload {
    /// The shipped `FUN_00c89580` copied the bytes into the record itself.
    Inline,
    /// The shipped `FUN_00562fd0(len + 1)` allocation plus `FUN_00c89580`
    /// copy; the allocation address is not reproducible and is left out of
    /// the raw bytes.
    Owned(Vec<u8>),
    /// No copy: `+0x08` holds the caller's data pointer.
    Borrowed { ptr: usize, len: u32 },
}

/// The 16-byte record appended by `0x0061ef80`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A70StringRecord {
    /// The sixteen bytes as stored, except that the copied form's `+0x08`
    /// allocation pointer is zero here (see [`A70RecordPayload::Owned`]).
    pub raw: [u8; 16],
    /// Byte ownership of the record.
    pub payload: A70RecordPayload,
}

/// `0x0061ef80`: build one 16-byte string record.
///
/// `param_1` is the source data pointer (`data_ptr`), `param_2` the byte count
/// (`data.len()`) and `param_3` the copy flag.  With `copy` set:
///
/// * `len < 0x0e`: zero the record, copy `len` bytes at `+0x00`, write
///   `0x0d - (u8)len` at `+0x0d` and `0x1c05` at `+0x0e`.
/// * otherwise: store `len` at `+0x00`, allocate `len + 1`, store the
///   allocation at `+0x08`, write `0x0c05` at `+0x0e` and copy `len` bytes.
///
/// Without `copy` the record is zeroed, `len` is stored at `+0x00`, the
/// source pointer at `+0x08` and `0x0405` at `+0x0e`; nothing is copied.  The
/// function returns `1` (`returns 1` after the second append path).
pub fn a70_string_record(data_ptr: usize, data: &[u8], copy: bool) -> A70StringRecord {
    let mut raw = [0u8; 16];
    let len = data.len() as u32;
    if copy && (len as usize) < A70_RECORD_INLINE_CAPACITY as usize + 1 {
        raw[..data.len()].copy_from_slice(data);
        raw[0x0d] = A70_RECORD_INLINE_CAPACITY.wrapping_sub(len as u8);
        raw[0x0e..0x10].copy_from_slice(&A70_RECORD_TAG_INLINE.to_le_bytes());
        A70StringRecord {
            raw,
            payload: A70RecordPayload::Inline,
        }
    } else if copy {
        raw[0..4].copy_from_slice(&len.to_le_bytes());
        raw[0x0e..0x10].copy_from_slice(&A70_RECORD_TAG_COPIED.to_le_bytes());
        A70StringRecord {
            raw,
            payload: A70RecordPayload::Owned(data.to_vec()),
        }
    } else {
        raw[0..4].copy_from_slice(&len.to_le_bytes());
        raw[0x08..0x0c].copy_from_slice(&(data_ptr as u32).to_le_bytes());
        raw[0x0e..0x10].copy_from_slice(&A70_RECORD_TAG_BORROWED.to_le_bytes());
        A70StringRecord {
            raw,
            payload: A70RecordPayload::Borrowed { ptr: data_ptr, len },
        }
    }
}

/// `0x0061ef80`: the cursor/end check that precedes every record store.
///
/// The shipped sink keeps a cursor at `this + 0x24` and an end at
/// `this + 0x28`.  When `end < cursor + 0x10` the body calls `FUN_0061c370(1)`
/// and reloads both words from `this`; either way the record store advances
/// the cursor by `0x10`.
///
/// Returns `(grew, cursor_after)`.  `end` is only meaningful for the caller
/// when `grew` is false; the grown end comes from the relocation helper and is
/// not recoverable from the row.
pub fn a70_reserve_record(cursor: usize, end: usize) -> (bool, usize) {
    let grew = end < cursor.wrapping_add(0x10);
    (grew, cursor.wrapping_add(0x10))
}

// ---------------------------------------------------------------------------
// Chained hash table: 0x0061f040, 0x0061f260, 0x0061f406
// ---------------------------------------------------------------------------

/// One chained hash node.
///
/// `FUN_0061f260` compares the sixteen bytes at `+0x04` against the probe key
/// and `0x0061f040` reads the value dword at `+0x1c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A70HashNode {
    /// `+0x00`: next node index in the bucket chain.
    pub next: Option<usize>,
    /// `+0x04..=+0x13`: the 16-byte key.
    pub key: [u8; 16],
    /// `+0x1c`: the value word.
    pub value: u32,
}

/// The link cell returned by `0x0061f260`.
///
/// `*cell` is either a bucket head or the next link of an existing node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A70LinkCell {
    /// The head slot of bucket `bucket`.
    Head(usize),
    /// The `next` field of the node at this index.
    Next(usize),
}

/// `0x0061f260`: walk a bucket chain and return the link cell that holds the
/// matching node, or the cell that would hold it.
///
/// ```text
/// if (*head == 0) return head;
/// prev = head;
/// do {
///   node = *prev;
///   if (key[0..16] == node->key) return prev;
///   prev = &node->next;
/// } while (*prev != 0);
/// return prev;
/// ```
///
/// The comparison is a full sixteen bytes: the decompiler's
/// `pcVar2 == param_2 + 0x10` loop bound is the key size, and the byte index
/// `piVar3 + (4 - param_2)` is the node key at `+0x04`.  A miss on the last
/// node returns that node's `next` cell, whose value is null — exactly what
/// the `0x0061f040` caller dereferences.
pub fn a70_find_link(
    buckets: &[Option<usize>],
    nodes: &[A70HashNode],
    bucket: usize,
    key: &[u8; 16],
) -> A70LinkCell {
    let mut cell = A70LinkCell::Head(bucket);
    let mut cur = buckets.get(bucket).copied().flatten();
    while let Some(index) = cur {
        if nodes[index].key == *key {
            return cell;
        }
        cell = A70LinkCell::Next(index);
        cur = nodes[index].next;
    }
    cell
}

/// Dereference a [`A70LinkCell`].
pub fn a70_link_value(
    buckets: &[Option<usize>],
    nodes: &[A70HashNode],
    cell: A70LinkCell,
) -> Option<usize> {
    match cell {
        A70LinkCell::Head(bucket) => buckets.get(bucket).copied().flatten(),
        A70LinkCell::Next(index) => nodes.get(index).and_then(|node| node.next),
    }
}

/// `0x0061f040`: hash-table lookup.
///
/// `*this` is the bucket count and `this[1]` the bucket array.  The shipped
/// body bails out when `this[1] == 0`, otherwise computes
///
/// ```text
/// bucket = (count - 1) & FUN_00c62800(key, 0x10, 0)
/// ```
///
/// and walks the chain with `0x0061f260`.  The hash helper is outside this
/// batch, so the recovered hash value is a parameter; the masking arithmetic
/// is the shipped `(count - 1U & hash)`.
///
/// On a hit the shipped tail builds an empty string at `param_1` and enters
/// `FUN_00538810(node[0x1c])` (decompiled as `noreturn`); the value word is
/// returned here as `Some(value)`.  A miss stores the found flag `0` and
/// returns nothing, which is the `None` case.
///
/// A zero bucket count with a non-null array is an out-of-bounds read in the
/// shipped code; it is rejected here instead.
pub fn a70_hash_lookup(
    buckets: &[Option<usize>],
    nodes: &[A70HashNode],
    key: &[u8; 16],
    hash: u32,
) -> Option<u32> {
    if buckets.is_empty() {
        return None;
    }
    let count = buckets.len() as u32;
    let bucket = (count.wrapping_sub(1) & hash) as usize;
    let cell = a70_find_link(buckets, nodes, bucket, key);
    a70_link_value(buckets, nodes, cell).map(|index| nodes[index].value)
}

/// Result of the `0x0061f406` rehash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A70Rehash {
    /// `new_bytes >> 2`: dwords the shipped allocator zero-fill covers.
    pub cleared_dwords: u32,
    /// The freshly built bucket array.
    pub table: Vec<Option<usize>>,
}

/// `0x0061f406`: redistribute one bucket array into a new table.
///
/// The shipped body allocates the new table, zero-fills `new_bytes >> 2`
/// dwords, and for every old bucket walks its chain.  Each node is unlinked
/// first (`piVar1 = *node`), then relinked with
///
/// ```text
/// slot = FUN_00c62800(node + 1, 0x10, 0) & (new_count - 1)
/// node->next = new[slot]
/// new[slot] = node
/// ```
///
/// so every new chain is the reverse of the old chain.  The hash helper is
/// outside this batch; the recovered hash of each node is supplied in
/// `hashes`, indexed like `nodes`.
pub fn a70_rehash(
    old: &[Option<usize>],
    nodes: &mut [A70HashNode],
    hashes: &[u32],
    new_bytes: u32,
    new_count: u32,
) -> A70Rehash {
    let mut table = vec![None; new_count as usize];
    let mask = new_count.wrapping_sub(1);
    for slot in old {
        let mut cur = *slot;
        while let Some(index) = cur {
            let next = nodes[index].next;
            let target = (hashes[index] & mask) as usize;
            nodes[index].next = table[target];
            table[target] = Some(index);
            cur = next;
        }
    }
    A70Rehash {
        cleared_dwords: new_bytes >> 2,
        table,
    }
}

// ---------------------------------------------------------------------------
// 0x1c-byte element vectors: 0x0061f0e0 and 0x0061f177
// ---------------------------------------------------------------------------

/// The three-word header used by the element vectors in this window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A70VectorHeader {
    /// `+0x00`: element array.
    pub data: u32,
    /// `+0x04`: element count.
    pub count: u32,
    /// `+0x08`: capacity in the low 31 bits; bit 31 is the ownership flag.
    pub cap_flags: u32,
}

/// Outcome of [`a70_move_assign`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A70MoveAssign {
    /// Source is inline (`cap_flags` bit 31 clear): the three header words are
    /// moved and the source is left empty with only the flag bit kept.
    HeaderStolen,
    /// Source is heap-owned and the destination has room: the destination
    /// count is replaced by the source count and each `0x1c`-byte element is
    /// moved with its inline string header.
    ElementsMoved,
    /// The shipped `FUN_0093ec70` invalid-argument path (destination has no
    /// room or its flag bit is clear).  The destination count has already been
    /// cleared at this point.
    Invalid,
}

/// `0x0061f0e0`: move-assign one element vector into another.
///
/// ```text
/// if (src[2] >= 0) {                 // inline source
///   dst[2] &= 0x7fffffff;            // dead store, immediately overwritten
///   dst[0]  = src[0];
///   dst[1]  = src[1];
///   dst[2]  = src[2] & 0x7fffffff;
///   src[2] &= 0x80000000;
///   src[0]  = 0;  src[1] = 0;
///   return;
/// }
/// dst[1] = 0;
/// n = (src[1] * 0x1c) / 0x1c;        // the shipped 0x1c stride, == src count
/// if (n < (dst[2] & 0x7fffffff) && (int)dst[2] < 0) {
///   move n elements; dst[1] = n; return;
/// }
/// FUN_0093ec70();
/// ```
///
/// The element loop moves each element's inline string header with the
/// `0x8000000f` empty header and leaves an empty header in the source; only
/// the header arithmetic is modelled here.
pub fn a70_move_assign(dst: &mut A70VectorHeader, src: &mut A70VectorHeader) -> A70MoveAssign {
    if src.cap_flags & 0x8000_0000 == 0 {
        dst.cap_flags &= 0x7fff_ffff;
        dst.data = src.data;
        dst.count = src.count;
        dst.cap_flags = src.cap_flags & 0x7fff_ffff;
        src.cap_flags &= 0x8000_0000;
        src.data = 0;
        src.count = 0;
        return A70MoveAssign::HeaderStolen;
    }
    dst.count = 0;
    let n = src.count.wrapping_mul(0x1c) / 0x1c;
    if n < (dst.cap_flags & 0x7fff_ffff) && (dst.cap_flags as i32) < 0 {
        dst.count = n;
        return A70MoveAssign::ElementsMoved;
    }
    A70MoveAssign::Invalid
}

/// `0x0061f177`: layout of the `0x1c`-byte element array built for `n`
/// elements.
///
/// The shipped allocation is
/// `[vtable + 4]((n * 8 - n) * 4, 8)`, i.e. `0x1c * n` bytes with alignment
/// 8, and the capacity word at `+0x08` is `n & 0x7fffffff`.  The body then
/// moves the source elements and stores `n` at `this + 4`.
///
/// Returns `(byte_size, capacity_word)`.
pub fn a70_element_array_layout(n: u32) -> (u32, u32) {
    (
        n.wrapping_mul(8).wrapping_sub(n).wrapping_mul(4),
        n & 0x7fff_ffff,
    )
}

// ---------------------------------------------------------------------------
// Bucket-count policy: 0x0061f360
// ---------------------------------------------------------------------------

/// `0x0061f360`: choose the bucket count for a table of `needed` entries.
///
/// The recovered arithmetic is
///
/// ```text
/// n = 8;
/// if (8 < needed) do { n *= 2; } while (n < needed);
/// ```
///
/// after the load-factor comparison the shipped body performs with
/// `DAT_00d33180[...]` and the `FUN_00c86e10()` probe.  The function returns
/// early (success) only when `n == *this && this[1] != 0`; every other path is
/// the `FUN_0093ec70` invalid-argument tail, represented by `None`.
///
/// The shipped `n *= 2` is a wrapping 32-bit shift; a `needed` above
/// `0x8000_0000` makes it wrap and loop forever, so that case is rejected.
pub fn a70_bucket_target(needed: u32, current_cap: u32, has_buckets: bool) -> Option<u32> {
    let mut n: u32 = 8;
    if n < needed {
        loop {
            n = n.checked_mul(2)?;
            if n >= needed {
                break;
            }
        }
    }
    if n == current_cap && has_buckets {
        Some(n)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Trivial accessors: 0x0061f4b0, 0x0061f4d0, 0x0061f4f0, 0x0061f500
// ---------------------------------------------------------------------------

/// `0x0061f4b0`: `return *in_ECX == *param_1;` (`sete al`).
pub fn a70_node_eq(this_word: u32, other: u32) -> bool {
    this_word == other
}

/// `0x0061f4d0`: `return *in_ECX != *param_1;` (`setne al`).
pub fn a70_node_ne(this_word: u32, other: u32) -> bool {
    this_word != other
}

/// `0x0061f4f0`: `*param_1 = param_2;`.
pub fn a70_store(out: &mut u32, value: u32) {
    *out = value;
}

/// Constant buffer address passed by `0x0061f500` (`&DAT_00d334cc`).
pub const A70_PAIR_REPORT_TAG: u32 = 0x00d3_34cc;

/// `0x0061f500`: forwards `(pair[0], pair[1], &DAT_00d334cc, 4)` to
/// `FUN_0067d6b0`; the first argument of the row is unused.
///
/// Returns the four call arguments in order.
pub fn a70_pair_report_args(pair: [u32; 2]) -> (u32, u32, u32, u32) {
    (pair[0], pair[1], A70_PAIR_REPORT_TAG, 4)
}

// ---------------------------------------------------------------------------
// Language tag table: 0x0061f630
// ---------------------------------------------------------------------------

/// `0x0061f630`: map an LCID to its four-byte language tag.
///
/// The switch is on the low ten bits (`lcid & 0x3ff`); the primary language
/// picks the tag and the sublanguage field (`lcid & 0xfc00`) picks between the
/// two alternatives of the multi-variant languages:
///
/// | primary | condition         | tag          |
/// |---------|-------------------|--------------|
/// | `0x04`  | `0x400` -> `zhTW` | else `zhCN`  |
/// | `0x07`  | —                 | `deDE`       |
/// | `0x09`  | `0x4800` -> `enSG`| else `enUS`  |
/// | `0x0a`  | `0x800` -> `esMX` | else `esES`  |
/// | `0x0c`  | —                 | `frFR`       |
/// | `0x10`  | —                 | `itIT`       |
/// | `0x11`  | —                 | `jaJP`       |
/// | `0x12`  | `0x400` -> `koKR` | else `koUS`  |
/// | `0x15`  | —                 | `plPL`       |
/// | `0x16`  | `0x400` -> `ptBR` | else `ptPT`  |
/// | `0x19`  | —                 | `ruRU`       |
/// | `0x1e`  | —                 | `thTH`       |
///
/// The default arm stores `0` at `param_1 + 0x1c` and enters the `noreturn`
/// `FUN_00c86b6e` path, represented as `None`.  The selected tag is passed to
/// `FUN_006036b0` with its `strlen` length in the shipped tail.
pub fn a70_language_tag(lcid: u16) -> Option<&'static str> {
    let primary = lcid & 0x03ff;
    let sub = lcid & 0xfc00;
    Some(match primary {
        0x04 => {
            if sub == 0x0400 {
                "zhTW"
            } else {
                "zhCN"
            }
        }
        0x07 => "deDE",
        0x09 => {
            if sub == 0x4800 {
                "enSG"
            } else {
                "enUS"
            }
        }
        0x0a => {
            if sub == 0x0800 {
                "esMX"
            } else {
                "esES"
            }
        }
        0x0c => "frFR",
        0x10 => "itIT",
        0x11 => "jaJP",
        0x12 => {
            if sub == 0x0400 {
                "koKR"
            } else {
                "koUS"
            }
        }
        0x15 => "plPL",
        0x16 => {
            if sub == 0x0400 {
                "ptBR"
            } else {
                "ptPT"
            }
        }
        0x19 => "ruRU",
        0x1e => "thTH",
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Locale id predicates: 0x0061f820 ..= 0x0061f8b0
// ---------------------------------------------------------------------------

/// The identity words compared by the seven predicates in
/// `0x0061f820..=0x0061f8b0`.
///
/// The values live in writable globals (`DAT_00faa118` .. `DAT_00faa14c`) and
/// are assigned at run time, so they are carried as data rather than baked in
/// as constants.  Field names are the shipped addresses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A70LocaleIds {
    /// `DAT_00faa118`
    pub g_00faa118: u32,
    /// `DAT_00faa11c`
    pub g_00faa11c: u32,
    /// `DAT_00faa120`
    pub g_00faa120: u32,
    /// `DAT_00faa128`
    pub g_00faa128: u32,
    /// `DAT_00faa12c`
    pub g_00faa12c: u32,
    /// `DAT_00faa13c`
    pub g_00faa13c: u32,
    /// `DAT_00faa140`
    pub g_00faa140: u32,
    /// `DAT_00faa144`
    pub g_00faa144: u32,
    /// `DAT_00faa148`
    pub g_00faa148: u32,
    /// `DAT_00faa14c`
    pub g_00faa14c: u32,
}

impl A70LocaleIds {
    /// `0x0061f820`: true when `*this` is none of
    /// `DAT_00faa11c`, `DAT_00faa144`, `DAT_00faa120`, `DAT_00faa128`,
    /// `DAT_00faa13c`.
    ///
    /// The shipped body returns `1` from the all-unequal path and `0` from the
    /// match path (`CONCAT31(uVar2, 1)` / `(uint)uVar2 << 8`).
    pub fn a70_not_reserved(&self, id: u32) -> bool {
        id != self.g_00faa11c
            && id != self.g_00faa144
            && id != self.g_00faa120
            && id != self.g_00faa128
            && id != self.g_00faa13c
    }

    /// `0x0061f850`: `*this == DAT_00faa12c`.
    pub fn a70_is_00faa12c(&self, id: u32) -> bool {
        id == self.g_00faa12c
    }

    /// `0x0061f860`: `*this == DAT_00faa118`.
    pub fn a70_is_00faa118(&self, id: u32) -> bool {
        id == self.g_00faa118
    }

    /// `0x0061f870`: true when `*this` is `DAT_00faa118` or `DAT_00faa148`.
    ///
    /// Unlike `0x0061f820` this one returns `1` on a match:
    /// `if (id != A && id != B) return 0; return 1;`.
    pub fn a70_in_00faa118_or_00faa148(&self, id: u32) -> bool {
        id == self.g_00faa118 || id == self.g_00faa148
    }

    /// `0x0061f890`: `*this == DAT_00faa148`.
    pub fn a70_is_00faa148(&self, id: u32) -> bool {
        id == self.g_00faa148
    }

    /// `0x0061f8a0`: `*this == DAT_00faa14c`.
    pub fn a70_is_00faa14c(&self, id: u32) -> bool {
        id == self.g_00faa14c
    }

    /// `0x0061f8b0`: `*this == DAT_00faa140`.
    pub fn a70_is_00faa140(&self, id: u32) -> bool {
        id == self.g_00faa140
    }
}

// ---------------------------------------------------------------------------
// Locale parse helpers: 0x0061f8c0, 0x0061f96e, 0x0061fa94
// ---------------------------------------------------------------------------

/// Value returned by `0x0061f8c0` when the input is not four bytes.
pub const A70_LOCALE_CODE_OK: u32 = 0x409;

/// First constant buffer passed by the `0x0061f8c0` report tail
/// (`&DAT_00d3aea0`).
pub const A70_LOCALE_CODE_REPORT_TAG: u32 = 0x00d3_aea0;

/// `0x0061f8c0`: reject a four-byte locale code.
///
/// The body computes `strlen(param_1)` and, when it is exactly `4`, calls
/// `FUN_0067d6b0(param_1, 2, &DAT_00d3aea0, 2)` — decompiled as `noreturn`.
/// Any other length returns `0x409`.
///
/// `None` is the report path (the shipped call does not fall through);
/// `Some(0x409)` is the returned value.
pub fn a70_parse_locale_code(code: &[u8]) -> Option<u32> {
    if code.len() == 4 {
        None
    } else {
        Some(A70_LOCALE_CODE_OK)
    }
}

/// Value returned by `0x0061f96e` when its flag byte is set.
pub const A70_LOCALE_FLAG_OK: u32 = 0x407;

/// First constant buffer passed by the `0x0061f96e` report tail
/// (`&DAT_00d3aeac`).
pub const A70_LOCALE_FLAG_REPORT_TAG: u32 = 0x00d3_aeac;

/// `0x0061f96e`: return `0x407` for a set flag, otherwise report.
///
/// `if (in_AL != 0) return 0x407;` else the body stores `2` twice and calls
/// `FUN_0067d6b0(&DAT_00d3aeac, 2)` (decompiled as `noreturn`).
pub fn a70_parse_locale_flag(flag: bool) -> Option<u32> {
    if flag { Some(A70_LOCALE_FLAG_OK) } else { None }
}

/// `0x0061fa94`: the row is a single `ret`.
pub fn a70_noop() {}

// ---------------------------------------------------------------------------
// Locale-id list expansion: 0x0061fc30 and 0x0061fd10
// ---------------------------------------------------------------------------

/// Result of one of the two locale-id expansion rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A70IdExpansion {
    /// The count word the row clears before appending: `+0x10` for
    /// `0x0061fc30`, `+0x04` for `0x0061fd10`.
    pub reset_offset: u32,
    /// The vector contents after the append loop.
    pub values: Vec<u32>,
}

/// `0x0061fc30`: append `input`, duplicating `g_00faa148` after every element
/// when `*this == DAT_00faa118`.
///
/// The row starts with `*(*(this + 4) + 0x10) = 0` (the count of the
/// `+0x0c/+0x10/+0x14` vector header), then for each input element calls the
/// generic append:
///
/// ```text
/// index = (&element - data) >> 2
/// if (count == (cap & 0x7fffffff)) FUN_00597e60((cap & 0x7fffffff) + 1);
/// item = index < count ? data[index] : element;
/// data[count] = *item;  count += 1;
/// ```
///
/// and, when `0x0061f860` reports `*this == DAT_00faa118`, appends
/// `DAT_00faa148`.  The key `0x00faa148` is looked up in the destination
/// storage first, so a self-append reads the stored copy.
pub fn a70_expand_ids_count10(
    this_id: u32,
    input: &[u32],
    g_00faa118: u32,
    g_00faa148: u32,
) -> A70IdExpansion {
    a70_expand_ids(0x10, this_id, input, g_00faa118, g_00faa148)
}

/// `0x0061fd10`: the same loop as [`a70_expand_ids_count10`], over the vector
/// whose count is at `+0x04` (`*(*(this + 4) + 4) = 0`).
pub fn a70_expand_ids_count04(
    this_id: u32,
    input: &[u32],
    g_00faa118: u32,
    g_00faa148: u32,
) -> A70IdExpansion {
    a70_expand_ids(0x04, this_id, input, g_00faa118, g_00faa148)
}

fn a70_expand_ids(
    reset_offset: u32,
    this_id: u32,
    input: &[u32],
    g_00faa118: u32,
    g_00faa148: u32,
) -> A70IdExpansion {
    let mut values = Vec::with_capacity(input.len());
    for &value in input {
        values.push(value);
        if this_id == g_00faa118 {
            values.push(g_00faa148);
        }
    }
    A70IdExpansion {
        reset_offset,
        values,
    }
}

/// The append growth test shared by both rows: the shipped `FUN_00597e60`
/// request is issued when the current count equals the low 31 bits of the
/// capacity word, and asks for `low + 1`.
pub fn a70_push_request(count: u32, cap_flags: u32) -> Option<u32> {
    let low = cap_flags & 0x7fff_ffff;
    if count == low {
        Some(low.wrapping_add(1))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Copyable function objects: 0x0061fdf0 and 0x0061fe10
// ---------------------------------------------------------------------------

/// The vftable identities written by the two function-object constructors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A70Vftable {
    /// `_bcCopyableFunctionImpl<void, blz::vector<Locale, ...> const&>::vftable`
    Base,
    /// `_bcCopyableFunctionObjectImpl<<lambda_4f30617cb2c63d2b36d46e4c12668551>&, ...>::vftable`
    Lambda4f30617cb2c63d2b36d46e4c12668551,
    /// `_bcCopyableFunctionObjectImpl<<lambda_dbfea748a930bb5dfd67feba18932425>&, ...>::vftable`
    LambdaDbfea748a930bb5dfd67feba18932425,
}

/// A two-word function object: `+0x00` vftable identity, `+0x04` captured
/// `this` pointer read from `in_ECX + 4`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A70FunctionObject {
    /// `+0x00`
    pub vtable: A70Vftable,
    /// `+0x04`
    pub captured: u32,
}

/// A constructed function object plus the vftable stores in program order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A70FunctionObjectInit {
    /// The final object.
    pub object: A70FunctionObject,
    /// The two stores: base vftable first, derived vftable second.
    pub stores: [A70Vftable; 2],
}

/// `0x0061fdf0`: copy-construct the `lambda_4f30617c` function object.
///
/// ```text
/// *param_1   = _bcCopyableFunctionImpl<...>::vftable;
/// param_1[1] = *(in_ECX + 4);
/// *param_1   = _bcCopyableFunctionObjectImpl<<lambda_4f...>&, ...>::vftable;
/// ```
///
/// The first store is immediately overwritten; both are kept in `stores`.
pub fn a70_function_object_4f30617c(captured: u32) -> A70FunctionObjectInit {
    A70FunctionObjectInit {
        object: A70FunctionObject {
            vtable: A70Vftable::Lambda4f30617cb2c63d2b36d46e4c12668551,
            captured,
        },
        stores: [
            A70Vftable::Base,
            A70Vftable::Lambda4f30617cb2c63d2b36d46e4c12668551,
        ],
    }
}

/// `0x0061fe10`: the same two stores with the `lambda_dbfea748` vftable.
pub fn a70_function_object_dbfea748(captured: u32) -> A70FunctionObjectInit {
    A70FunctionObjectInit {
        object: A70FunctionObject {
            vtable: A70Vftable::LambdaDbfea748a930bb5dfd67feba18932425,
            captured,
        },
        stores: [
            A70Vftable::Base,
            A70Vftable::LambdaDbfea748a930bb5dfd67feba18932425,
        ],
    }
}

// ---------------------------------------------------------------------------
// Defaulted id accessor: 0x0061fe30
// ---------------------------------------------------------------------------

/// `0x0061fe30`: `id = DAT_00faa11c; if (*(in_ECX + 0x18) != 0) id =
/// *(in_ECX + 0x18); *param_1 = id;`.
///
/// The stored override at `+0x18` wins only when it is non-zero; otherwise the
/// global default is stored.
pub fn a70_id_or_default(override_id: u32, default_id: u32) -> u32 {
    if override_id != 0 {
        override_id
    } else {
        default_id
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent70 {
    use super::*;

    // -- 0x0061e170 / 0x0061e1ed ------------------------------------------

    #[test]
    fn list_header_self_links() {
        let list = A70List::a70_new();
        assert_eq!(list.count, 0);
        assert_eq!(list.nodes.len(), 1);
        assert_eq!(list.nodes[0].next, 0);
        assert_eq!(list.nodes[0].prev, 0);
        assert!(list.a70_snapshot().is_empty());
    }

    #[test]
    fn push_front_keeps_circular_links_and_order() {
        let mut list = A70List::a70_new();
        let first = list.a70_push_front([1, 2, 3, 4]);
        let second = list.a70_push_front([5, 6, 7, 8]);

        assert_eq!(list.count, 2);
        assert_eq!(list.nodes[0].next, second);
        assert_eq!(list.nodes[0].prev, first);
        assert_eq!(list.nodes[second].next, first);
        assert_eq!(list.nodes[second].prev, 0);
        assert_eq!(list.nodes[first].next, 0);
        assert_eq!(list.nodes[first].prev, second);
        assert_eq!(list.a70_snapshot(), vec![[5, 6, 7, 8], [1, 2, 3, 4]]);
    }

    // -- 0x0061ec30 -------------------------------------------------------

    #[test]
    fn seasons_index_message_is_exact() {
        assert_eq!(
            a70_seasons_index_report(),
            ("OnSeasonsIndexFileUnreadable", 0x169)
        );
    }

    // -- 0x0061ef80 -------------------------------------------------------

    #[test]
    fn string_record_inline_form() {
        let record = a70_string_record(0x1000, b"hello", true);
        assert_eq!(&record.raw[..5], b"hello");
        assert_eq!(record.raw[0x0d], 0x0d - 5);
        assert_eq!(
            u16::from_le_bytes([record.raw[0x0e], record.raw[0x0f]]),
            A70_RECORD_TAG_INLINE
        );
        assert_eq!(record.payload, A70RecordPayload::Inline);
    }

    #[test]
    fn string_record_copied_form() {
        let text = b"fourteen bytes";
        assert_eq!(text.len(), 14);
        let record = a70_string_record(0x2000, text, true);
        assert_eq!(u32::from_le_bytes(record.raw[0..4].try_into().unwrap()), 14);
        assert_eq!(
            u16::from_le_bytes([record.raw[0x0e], record.raw[0x0f]]),
            A70_RECORD_TAG_COPIED
        );
        assert_eq!(record.payload, A70RecordPayload::Owned(text.to_vec()));
    }

    #[test]
    fn string_record_borrowed_form_keeps_pointer() {
        let record = a70_string_record(0x1234_5600, b"abc", false);
        assert_eq!(u32::from_le_bytes(record.raw[0..4].try_into().unwrap()), 3);
        assert_eq!(
            u32::from_le_bytes(record.raw[8..12].try_into().unwrap()),
            0x1234_5600
        );
        assert_eq!(
            u16::from_le_bytes([record.raw[0x0e], record.raw[0x0f]]),
            A70_RECORD_TAG_BORROWED
        );
        assert_eq!(
            record.payload,
            A70RecordPayload::Borrowed {
                ptr: 0x1234_5600,
                len: 3
            }
        );
        // The borrowed form has no inline terminator byte.
        assert_eq!(record.raw[0x0d], 0);
    }

    #[test]
    fn record_reservation_grows_only_when_full() {
        assert_eq!(a70_reserve_record(0x1000, 0x1010), (false, 0x1010));
        assert_eq!(a70_reserve_record(0x1000, 0x100f), (true, 0x1010));
        assert_eq!(a70_reserve_record(0x1000, 0x1020), (false, 0x1010));
    }

    // -- 0x0061f177 -------------------------------------------------------

    #[test]
    fn element_array_uses_the_0x1c_stride() {
        assert_eq!(a70_element_array_layout(0), (0, 0));
        assert_eq!(a70_element_array_layout(1), (0x1c, 1));
        assert_eq!(a70_element_array_layout(5), (0x8c, 5));
        // (n * 8 - n) * 4 == n * 0x1c, including the top-bit capacity mask.
        assert_eq!(a70_element_array_layout(0x1000), (0x1c000, 0x1000));
        assert_eq!(a70_element_array_layout(0x8000_0000), (0, 0));
    }

    // -- 0x0061f260 -------------------------------------------------------

    fn chain_nodes() -> Vec<A70HashNode> {
        vec![
            A70HashNode {
                next: Some(1),
                key: [0x01; 16],
                value: 0x1111,
            },
            A70HashNode {
                next: Some(2),
                key: [0x02; 16],
                value: 0x2222,
            },
            A70HashNode {
                next: None,
                key: [0x03; 16],
                value: 0x3333,
            },
        ]
    }

    #[test]
    fn find_link_returns_the_cell_holding_the_key() {
        let nodes = chain_nodes();
        let buckets = vec![Some(0)];

        assert_eq!(
            a70_find_link(&buckets, &nodes, 0, &[0x01; 16]),
            A70LinkCell::Head(0)
        );
        assert_eq!(
            a70_find_link(&buckets, &nodes, 0, &[0x02; 16]),
            A70LinkCell::Next(0)
        );
        assert_eq!(
            a70_find_link(&buckets, &nodes, 0, &[0x03; 16]),
            A70LinkCell::Next(1)
        );
        // A miss on the last node returns that node's (null) next cell.
        assert_eq!(
            a70_find_link(&buckets, &nodes, 0, &[0x04; 16]),
            A70LinkCell::Next(2)
        );
        assert_eq!(a70_link_value(&buckets, &nodes, A70LinkCell::Next(2)), None);
        // An empty bucket returns the head cell.
        assert_eq!(
            a70_find_link(&[None], &nodes, 0, &[0x01; 16]),
            A70LinkCell::Head(0)
        );
    }

    // -- 0x0061f040 -------------------------------------------------------

    #[test]
    fn hash_lookup_masks_with_count_minus_one() {
        // One node per bucket, so the mask chooses the node directly.
        let nodes = vec![
            A70HashNode {
                next: None,
                key: [0x01; 16],
                value: 0x1111,
            },
            A70HashNode {
                next: None,
                key: [0x02; 16],
                value: 0x2222,
            },
            A70HashNode {
                next: None,
                key: [0x03; 16],
                value: 0x3333,
            },
        ];
        // count 4 -> mask 3; hashes 0x04..0x06 land in buckets 0..2.
        let buckets = vec![Some(0), Some(1), Some(2), None];

        assert_eq!(
            a70_hash_lookup(&buckets, &nodes, &[0x01; 16], 0x04),
            Some(0x1111)
        );
        assert_eq!(
            a70_hash_lookup(&buckets, &nodes, &[0x02; 16], 0x05),
            Some(0x2222)
        );
        assert_eq!(
            a70_hash_lookup(&buckets, &nodes, &[0x03; 16], 0x06),
            Some(0x3333)
        );
        // Hash 0x05 selects bucket 1, which holds the 0x02 key.
        assert_eq!(a70_hash_lookup(&buckets, &nodes, &[0x04; 16], 0x05), None);
        // `this[1] == 0` short-circuits before the hash is even used.
        assert_eq!(a70_hash_lookup(&[], &nodes, &[0x01; 16], 0x05), None);
    }

    // -- 0x0061f360 -------------------------------------------------------

    #[test]
    fn bucket_target_rounds_up_to_a_power_of_two_minimum_eight() {
        assert_eq!(a70_bucket_target(0, 8, true), Some(8));
        assert_eq!(a70_bucket_target(8, 8, true), Some(8));
        assert_eq!(a70_bucket_target(8, 8, false), None);
        assert_eq!(a70_bucket_target(9, 16, true), Some(16));
        assert_eq!(a70_bucket_target(100, 128, true), Some(128));
        assert_eq!(a70_bucket_target(100, 128, false), None);
        // The current table is not the computed size.
        assert_eq!(a70_bucket_target(100, 64, true), None);
        // A size that would wrap the shipped `shl` is rejected.
        assert_eq!(a70_bucket_target(0x8000_0001, 0, true), None);
    }

    // -- 0x0061f406 -------------------------------------------------------

    #[test]
    fn rehash_relinks_and_reverses_chains() {
        let mut nodes = vec![
            A70HashNode {
                next: None,
                key: [0x0a; 16],
                value: 1,
            },
            A70HashNode {
                next: None,
                key: [0x0b; 16],
                value: 2,
            },
            A70HashNode {
                next: Some(0),
                key: [0x0c; 16],
                value: 3,
            },
        ];
        // node 2 -> node 0 in bucket 0 of the old table, node 1 in bucket 2.
        let old = vec![Some(2), None, Some(1)];
        // hashes: 0b001, 0b010, 0b011.
        let hashes = [0b001, 0b010, 0b011];

        let result = a70_rehash(&old, &mut nodes, &hashes, 0x40, 2);
        assert_eq!(result.cleared_dwords, 16);
        // mask is 1: nodes 0 and 2 go to bucket 1, node 1 to bucket 0.
        assert_eq!(result.table, vec![Some(1), Some(0)]);
        assert_eq!(nodes[0].next, Some(2));
        assert_eq!(nodes[2].next, None);
        assert_eq!(nodes[1].next, None);
    }

    // -- 0x0061f0e0 -------------------------------------------------------

    #[test]
    fn move_assign_steals_inline_headers() {
        let mut dst = A70VectorHeader {
            data: 0xaaaa,
            count: 9,
            cap_flags: 0x8000_0004,
        };
        let mut src = A70VectorHeader {
            data: 0xbbbb,
            count: 3,
            cap_flags: 0x0000_0007,
        };
        assert_eq!(
            a70_move_assign(&mut dst, &mut src),
            A70MoveAssign::HeaderStolen
        );
        assert_eq!(
            dst,
            A70VectorHeader {
                data: 0xbbbb,
                count: 3,
                cap_flags: 7
            }
        );
        assert_eq!(
            src,
            A70VectorHeader {
                data: 0,
                count: 0,
                cap_flags: 0
            }
        );
    }

    #[test]
    fn move_assign_requires_heap_destination_room() {
        let mut dst = A70VectorHeader {
            data: 0x1000,
            count: 9,
            cap_flags: 0x8000_0010,
        };
        let mut src = A70VectorHeader {
            data: 0x2000,
            count: 4,
            cap_flags: 0x8000_0004,
        };
        assert_eq!(
            a70_move_assign(&mut dst, &mut src),
            A70MoveAssign::ElementsMoved
        );
        assert_eq!(dst.count, 4);
        assert_eq!(dst.data, 0x1000);
        assert_eq!(src.count, 4);

        // Not enough room in the destination.
        let mut small = A70VectorHeader {
            data: 0x1000,
            count: 9,
            cap_flags: 0x8000_0004,
        };
        let mut five = A70VectorHeader {
            data: 0x2000,
            count: 5,
            cap_flags: 0x8000_0005,
        };
        assert_eq!(
            a70_move_assign(&mut small, &mut five),
            A70MoveAssign::Invalid
        );
        assert_eq!(small.count, 0);

        // Inline destination (flag bit clear) is also invalid.
        let mut inline_dst = A70VectorHeader {
            data: 0,
            count: 9,
            cap_flags: 0x0000_0010,
        };
        assert_eq!(
            a70_move_assign(&mut inline_dst, &mut five),
            A70MoveAssign::Invalid
        );
    }

    // -- 0x0061f4b0 / 0x0061f4d0 / 0x0061f4f0 / 0x0061f500 ---------------

    #[test]
    fn word_predicates_and_store() {
        assert!(a70_node_eq(7, 7));
        assert!(!a70_node_eq(7, 8));
        assert!(a70_node_ne(7, 8));
        assert!(!a70_node_ne(7, 7));

        let mut word = 0u32;
        a70_store(&mut word, 0xdead_beef);
        assert_eq!(word, 0xdead_beef);
    }

    #[test]
    fn pair_report_forwards_the_four_arguments() {
        assert_eq!(
            a70_pair_report_args([0x11, 0x22]),
            (0x11, 0x22, 0x00d3_34cc, 4)
        );
    }

    // -- 0x0061f630 -------------------------------------------------------

    #[test]
    fn language_tag_table_is_exact() {
        assert_eq!(a70_language_tag(0x0404), Some("zhTW"));
        assert_eq!(a70_language_tag(0x0804), Some("zhCN"));
        assert_eq!(a70_language_tag(0x0004), Some("zhCN"));
        assert_eq!(a70_language_tag(0x0407), Some("deDE"));
        assert_eq!(a70_language_tag(0x4809), Some("enSG"));
        assert_eq!(a70_language_tag(0x0409), Some("enUS"));
        assert_eq!(a70_language_tag(0x080a), Some("esMX"));
        assert_eq!(a70_language_tag(0x040a), Some("esES"));
        assert_eq!(a70_language_tag(0x040c), Some("frFR"));
        assert_eq!(a70_language_tag(0x0410), Some("itIT"));
        assert_eq!(a70_language_tag(0x0411), Some("jaJP"));
        assert_eq!(a70_language_tag(0x0412), Some("koKR"));
        assert_eq!(a70_language_tag(0x0412 | 0x0800), Some("koUS"));
        assert_eq!(a70_language_tag(0x0415), Some("plPL"));
        assert_eq!(a70_language_tag(0x0416), Some("ptBR"));
        assert_eq!(a70_language_tag(0x0816), Some("ptPT"));
        assert_eq!(a70_language_tag(0x0419), Some("ruRU"));
        assert_eq!(a70_language_tag(0x041e), Some("thTH"));
        // The switch only consumes the low ten bits.
        assert_eq!(a70_language_tag(0xfc07), Some("deDE"));
        assert_eq!(a70_language_tag(0x0000), None);
        assert_eq!(a70_language_tag(0x0401), None);
    }

    // -- 0x0061f820 ..= 0x0061f8b0 ---------------------------------------

    #[test]
    fn locale_id_predicates() {
        let ids = A70LocaleIds {
            g_00faa118: 0x118,
            g_00faa11c: 0x11c,
            g_00faa120: 0x120,
            g_00faa128: 0x128,
            g_00faa12c: 0x12c,
            g_00faa13c: 0x13c,
            g_00faa140: 0x140,
            g_00faa144: 0x144,
            g_00faa148: 0x148,
            g_00faa14c: 0x14c,
        };

        // 0x0061f820: five ids are excluded, and only those five.
        assert!(!ids.a70_not_reserved(0x11c));
        assert!(!ids.a70_not_reserved(0x144));
        assert!(!ids.a70_not_reserved(0x120));
        assert!(!ids.a70_not_reserved(0x128));
        assert!(!ids.a70_not_reserved(0x13c));
        assert!(ids.a70_not_reserved(0x118));
        assert!(ids.a70_not_reserved(0x12c));

        assert!(ids.a70_is_00faa12c(0x12c));
        assert!(!ids.a70_is_00faa12c(0x118));
        assert!(ids.a70_is_00faa118(0x118));
        assert!(!ids.a70_is_00faa118(0x148));
        assert!(ids.a70_in_00faa118_or_00faa148(0x118));
        assert!(ids.a70_in_00faa118_or_00faa148(0x148));
        assert!(!ids.a70_in_00faa118_or_00faa148(0x140));
        assert!(ids.a70_is_00faa148(0x148));
        assert!(!ids.a70_is_00faa148(0x118));
        assert!(ids.a70_is_00faa14c(0x14c));
        assert!(ids.a70_is_00faa140(0x140));
    }

    // -- 0x0061f8c0 / 0x0061f96e / 0x0061fa94 -----------------------------

    #[test]
    fn locale_parse_helpers() {
        assert_eq!(a70_parse_locale_code(b"enUS"), None);
        assert_eq!(a70_parse_locale_code(b"enU"), Some(0x409));
        assert_eq!(a70_parse_locale_code(b""), Some(0x409));

        assert_eq!(a70_parse_locale_flag(true), Some(0x407));
        assert_eq!(a70_parse_locale_flag(false), None);

        a70_noop();
    }

    // -- 0x0061fc30 / 0x0061fd10 ------------------------------------------

    #[test]
    fn id_expansion_appends_the_extra_id_only_for_0xfaa118() {
        let input = [0x11, 0x22];

        let with_extra = a70_expand_ids_count10(0x118, &input, 0x118, 0x148);
        assert_eq!(with_extra.reset_offset, 0x10);
        assert_eq!(with_extra.values, vec![0x11, 0x148, 0x22, 0x148]);

        let without_extra = a70_expand_ids_count10(0x140, &input, 0x118, 0x148);
        assert_eq!(without_extra.values, vec![0x11, 0x22]);

        // The second row only differs in the count word it clears.
        let alt = a70_expand_ids_count04(0x118, &input, 0x118, 0x148);
        assert_eq!(alt.reset_offset, 0x04);
        assert_eq!(alt.values, vec![0x11, 0x148, 0x22, 0x148]);
    }

    #[test]
    fn push_request_matches_the_capacity_test() {
        assert_eq!(a70_push_request(3, 0x8000_0003), Some(4));
        assert_eq!(a70_push_request(2, 0x8000_0003), None);
        assert_eq!(a70_push_request(0, 0x0000_0000), Some(1));
    }

    // -- 0x0061fdf0 / 0x0061fe10 / 0x0061fe30 -----------------------------

    #[test]
    fn function_object_stores_base_then_derived() {
        let a = a70_function_object_4f30617c(0x1234);
        assert_eq!(
            a.stores,
            [
                A70Vftable::Base,
                A70Vftable::Lambda4f30617cb2c63d2b36d46e4c12668551
            ]
        );
        assert_eq!(
            a.object,
            A70FunctionObject {
                vtable: A70Vftable::Lambda4f30617cb2c63d2b36d46e4c12668551,
                captured: 0x1234
            }
        );

        let b = a70_function_object_dbfea748(0x5678);
        assert_eq!(
            b.stores,
            [
                A70Vftable::Base,
                A70Vftable::LambdaDbfea748a930bb5dfd67feba18932425
            ]
        );
        assert_eq!(
            b.object,
            A70FunctionObject {
                vtable: A70Vftable::LambdaDbfea748a930bb5dfd67feba18932425,
                captured: 0x5678
            }
        );
    }

    #[test]
    fn id_or_default_prefers_the_non_zero_override() {
        assert_eq!(a70_id_or_default(0x77, 0x11c), 0x77);
        assert_eq!(a70_id_or_default(0, 0x11c), 0x11c);
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// The 42 rows not translated above, grouped by why:
//
// * `FUN_0061e158`: seven-byte tail call into `FUN_00c86b6e`; no local work.
// * `FUN_0061e1cd`, `FUN_0061e339`, `FUN_0061e412`: bodies begin mid-instruction
//   (`*in_EAX = *in_EAX + (char)in_EAX`, string-literal byte arithmetic); the
//   decompiler lost the instruction boundary, so no behaviour is recoverable.
// * `FUN_0061e250`: 228-byte body whose real work is `FUN_0074f610`,
//   `FUN_0060afa0`, `FUN_0060a820`, `FUN_00623870` and `FUN_00538810`; only
//   the inlined list/string cleanup is present in the row.
// * `FUN_0061e468`, `FUN_0061e47d`, `FUN_0061e4a0`, `FUN_0061e50b`, `FUN_0061e527`,
//   `FUN_0061e53a`, `FUN_0061e56c`, `FUN_0061e64f`, `FUN_0061e667`, `FUN_0061e72f`,
//   `FUN_0061e730`, `FUN_0061e7dd`, `FUN_0061e7fb`, `FUN_0061e847`, `FUN_0061e880`,
//   `FUN_0061e8d0`, `FUN_0061e911`, `FUN_0061e93d`, `FUN_0061ea8d`, `FUN_0061eac7`,
//   `FUN_0061eb5e`, `FUN_0061eb8f`, `FUN_0061ebd0`, `FUN_0061ebfc`, `FUN_0061ec1a`,
//   `FUN_0061ee82`: SEH unwind fragments of one `blz::vector<Locale>` copy /
//   append loop.  They address every value through `unaff_EBP`/`unaff_EDI`
//   stack offsets belonging to a caller outside the batch, and their visible
//   work is `FUN_0093ec70` unwind calls plus half of an inlined `0x1c`-byte
//   element move.
// * `FUN_0061e5a0`, `FUN_0061eeb0`: build an empty string and enter the
//   `noreturn` `FUN_00538810` / `FUN_0064c480` path; the row's remaining
//   arguments are not part of the decompilation.
// * `FUN_0061ec90`, `FUN_0061fe50`, `FUN_0061fea0`: jump-table bodies the decompiler
//   flagged as bad instruction data / overlapping instructions; the recovered
//   text contains `halt_baddata`, `rdtsc` and non-canonical stores, so no
//   source-level behaviour can be claimed.
// * `FUN_0061f222`, `FUN_0061f483`: continuation halves of the element move loops
//   at `0x0061f177` and `0x0061f406`; they start with `*dst = 0` /
//   `while (carry)` and depend on the caller's loop registers.
// * `FUN_0061f2c0`: list destructor prologue.  The three header stores are
//   reproduced by [`A70List::a70_new`], but the element-destruction loop for a
//   non-empty old list is not in the row (it is an exception-cleanup path).
// * `FUN_0061f560`: constructs the `0x0061f630` language object and forwards it
//   to `FUN_006036b0`; both call targets are outside the batch.
// * `FUN_0061fae3`, `FUN_0061fba2`: partial stores (`+0x04 = 2`, a bad-instruction
//   global byte update) followed by `FUN_0067d6b0` with an incomplete argument
//   list.
