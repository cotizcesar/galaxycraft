#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent43.rs
// ===========================================================================
// Translation batch 43.  Only this file is edited by the batch.
//
// Twenty-four `decompiled_uncited` rows in `0x005e8000..=0x005e9fff` are
// reconstructed here.  The window is the client `bc` support library
// (`bcCopyableFunction*`, `CommandString`, the node/hash-bucket containers and
// the `streambuf` wrappers) wrapped around the 32-bit DirectPlay/COM startup
// helpers.  Very little of it is simulation state, but a large part of it is
// exact container arithmetic, which is what this batch translates:
//
// * `0x005e8400`, `0x005e842a` - the 16-byte header tag decision
//   (`0x1c05` inline with spare-byte counter, `0x0c05` heap + `len + 1`).
// * `0x005e84f0`, `0x005e8500`, `0x005e8514` - the event-info word
//   `0x1d60000 | (~(value >> 26) & 0x20) << 16` and its 16-bit form.
// * `0x005e85b1`, `0x005e85c0` - the dialog selector word/flag triple
//   (`0x116` negative, `0x196` positive, `|0x40` zero, `|0x20` zero with a
//   non-negative mask).
// * `0x005e8660` - the keyed tree probe (prefix compare, then length
//   difference; only the left child is followed, exactly as decompiled).
// * `0x005e8ea0`, `0x005e9170` - `bcCopyableFunction` heap/inline storage
//   release (bit 0 of the storage word is the heap tag, `1` the empty word).
// * `0x005e8f50`, `0x005e8fd0`, `0x005e8fdf` - the refcount guard chains
//   (`+0x24`/`+0x8`, then `+0x2a8`/`+0x270`/`+0x254`/`+0x234`) and the
//   `0xbaadf00d` poison write.
// * `0x005e9040`, `0x005e905b` - the recovered key triple gate
//   (`present && index <= 0x14 && sub <= 4`).
// * `0x005e9470`, `0x005e9481`, `0x005e98b0`, `0x005e952d` - the hash
//   bucket insert/probe and load-factor check.  The hash kernel at
//   `FUN_00c62800` is MurmurHash3 x86-32 (the constants `0xcc9e2d51`,
//   `0x1b873593`, `0xe6546b64`, `0x85ebca6b`, `0xc2b2ae35` are all present
//   in its body), reproduced inline as [`a43_hash_bytes`].
// * `0x005e9890` - the `{0x18, 4, 0}` layout descriptor.
// * `0x005e9990`, `0x005e9fc0` - the 24-byte small-string header: the
//   inline/heap view (`0xf < capacity`) and the two-candidate move choice.
// * `0x005e9f20`, `0x005e9f21` - the stream-cursor equality operator
//   (`checked` byte at `+4`, peeked byte at `+5`, end-of-stream is a null
//   stream word).
//
// The rest of the range is not translatable as arithmetic: C++ throw
// wrappers, `__break`/`halt_baddata` stubs, DirectPlay/COM init that calls
// through import pointers, exception-unwind funclets, and the `streambuf`
// `xsputn` engine whose state lives behind an unrecovered vtable.  Those rows
// are listed in the batch report with their skip reason and are not given
// invented methods.  The batch adds no persistent state; every helper is a
// free function over caller-supplied words.

// ---------------------------------------------------------------------------
// 0x005e8400 / 0x005e842a - inline/heap value-header decision
// ---------------------------------------------------------------------------

/// Inline length boundary: `len < 0xe` selects the inline form.
pub const A43_VALUE_INLINE_LIMIT: u32 = 0xe;
/// Tag high half for the inline form (`0x1c05`).
pub const A43_VALUE_TAG_INLINE: u32 = 0x1c05_0000;
/// Tag high half for the heap form (`0x0c05`).
pub const A43_VALUE_TAG_HEAP: u32 = 0x0c05_0000;
/// First free-byte counter value for an empty inline payload (`0xd`).
pub const A43_VALUE_SPARE_BASE: u32 = 0xd;

/// Storage decision recovered from `0x005e8400`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A43ValueStore {
    /// `length < 0xe`: the tag word carries `0xd - length` in bits 8..=15.
    Inline {
        /// Remaining inline bytes (`0xd - length`).
        spare: u8,
        /// Word written to header offset `+0xc`.
        tag: u32,
    },
    /// Otherwise the payload is allocated with `length + 1` bytes and the
    /// length is written to the first header word.
    Heap {
        /// Payload length.
        length: u32,
        /// Allocation request (`length + 1`).
        allocation: u32,
        /// Word written to header offset `+0xc`.
        tag: u32,
    },
}

/// `0x005e8400` (`FUN_005e8400`, 42 bytes): decide how a `(pointer, length)`
/// record is stored.
///
/// ```c
/// length = record[1];
/// if (length < 0xe) {
///   tag = CONCAT22(0x1c05, (ushort)(byte)(0xd - (char)length) << 8);
/// } else {
///   tag = 0x0c050000;
///   header[0] = length;
///   allocation = FUN_00562fd0(length + 1);      // bytes
/// }
/// memcpy(payload, record[0], length);
/// ```
///
/// The tag's high half is the form id (`0x1c05`/`0x0c05`); the middle byte of
/// the inline word is the free-byte counter, which is why the empty value is
/// `0x1c050d00`.  The payload copy is the caller's business, so only the
/// decision and its words are returned.
pub fn a43_value_store(length: u32) -> A43ValueStore {
    if length < A43_VALUE_INLINE_LIMIT {
        let spare = (A43_VALUE_SPARE_BASE - length) as u8;
        A43ValueStore::Inline {
            spare,
            tag: A43_VALUE_TAG_INLINE | ((spare as u32) << 8),
        }
    } else {
        A43ValueStore::Heap {
            length,
            allocation: length.wrapping_add(1),
            tag: A43_VALUE_TAG_HEAP,
        }
    }
}

/// `0x005e842a` (`FUN_005e842a`, 117 bytes): the same body reached through
/// the caller frame.
///
/// The decompile reads the record from `[ebp + 0xc]`
/// (`(*(undefined4 **)(unaff_EBP + 0xc))[1]` for the length) instead of the
/// stack copy used by `0x005e8400`; every constant and branch is identical.
pub fn a43_value_store_from_frame(length: u32) -> A43ValueStore {
    a43_value_store(length)
}

// ---------------------------------------------------------------------------
// 0x005e84f0 / 0x005e8500 / 0x005e8514 - event info word
// ---------------------------------------------------------------------------

/// Base of the packed event-info word (`0x1d6 << 16`).
pub const A43_EVENT_BASE: u32 = 0x1d6_0000;
/// Base of the 16-bit event flag word (`0x1d6`).
pub const A43_EVENT_FLAGS_BASE: u16 = 0x1d6;
/// Bit set when the event value is non-negative (`0x20`, shifted by 16 in the
/// packed word).
pub const A43_EVENT_NONNEGATIVE: u32 = 0x20;

/// `0x005e84f0` (`FUN_005e84f0`, 11 bytes) and `0x005e8500`
/// (`FUN_005e8500`, 20 bytes): the packed event-info word.
///
/// ```c
/// word = ((ushort)~(ushort)((uint)value >> 0x1a) & 0x20) << 0x10 | 0x1d60000;
/// ```
///
/// The complement test is the sign test on the packed value: bits 26..=31 of
/// `value` are folded into bit 5 of the low word before the shift, so a
/// non-negative `value` sets `0x200000` and the empty word is `0x1f60000`.
pub fn a43_event_info_word(value: u32) -> u32 {
    let flags = (!((value >> 26) as u16) & 0x20) as u32;
    A43_EVENT_BASE | (flags << 16)
}

/// `0x005e8514` (`FUN_005e8514`, 141 bytes): the 16-bit header word of the
/// same event, plus the two header fields it fills.
///
/// ```c
/// header[0] = value;
/// header[1] = 0;
/// *(ushort *)(header + 2) = ~(ushort)((uint)value >> 0x1a) & 0x20 | 0x1d6;
/// ```
///
/// The returned word is the low half of [`a43_event_info_word`] for the same
/// input (`0x1d6`/`0x1f6`); `0x005e8514` stores it at header offset `+0xe`
/// (while `0x005e84f0`/`0x005e8500` pack the 32-bit form at `+0xc`).
pub fn a43_event_header_word(value: u32) -> (u32, u32, u16) {
    let flags = (!((value >> 26) as u16) & 0x20) as u16;
    (value, 0, A43_EVENT_FLAGS_BASE | flags)
}

// ---------------------------------------------------------------------------
// 0x005e85b1 / 0x005e85c0 - dialog selector flags
// ---------------------------------------------------------------------------

/// Flag word for a negative selector.
pub const A43_DIALOG_NEGATIVE: u16 = 0x116;
/// Flag word for a positive selector.
pub const A43_DIALOG_POSITIVE: u16 = 0x196;
/// Zero-selector bit.
pub const A43_DIALOG_ZERO: u16 = 0x40;
/// Zero-selector bit added when the companion mask is non-negative.
pub const A43_DIALOG_ZERO_MASK: u16 = 0x20;

/// `0x005e85c0` (`FUN_005e85c0`, 151 bytes): the dialog selector flags.
///
/// ```c
/// flags = 0x116;                      // selector < 0
/// if (selector >= 0) flags = 0x196;
/// if (selector == 0) flags |= 0x40;   // 0x1d6
/// if ((mask & 0x80000000) == 0 && selector == 0) flags |= 0x20;   // 0x1f6
/// ```
///
/// `selector` is the signed word read from `[ebp + 0x14]`, `mask` the word
/// read from `[ebp + 0x10]`.  The function writes the result as a `u16` into
/// the header at `+0xe`, so the sign of the mask only matters for the zero
/// selector.
pub fn a43_dialog_flags(selector: i32, mask: u32) -> u16 {
    let mut flags = if selector < 0 {
        A43_DIALOG_NEGATIVE
    } else {
        A43_DIALOG_POSITIVE
    };
    if selector == 0 {
        flags |= A43_DIALOG_ZERO;
        if mask & 0x8000_0000 == 0 {
            flags |= A43_DIALOG_ZERO_MASK;
        }
    }
    flags
}

/// `0x005e85b1` (`FUN_005e85b1`, 15 bytes): the packed 32-bit form.
///
/// The routine builds the same triple as `0x005e85c0` and stores
/// `flags << 16` at `[ebp - 0x14]` (the low half is left zero).  Its split
/// entry `0x005e85c0` keeps the 16-bit word instead.
pub fn a43_dialog_info_word(selector: i32, mask: u32) -> u32 {
    (a43_dialog_flags(selector, mask) as u32) << 16
}

// ---------------------------------------------------------------------------
// 0x005e8660 - keyed tree probe
// ---------------------------------------------------------------------------

/// Result of [`a43_key_prefix_compare`] / the tree probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A43TreeSlot {
    /// The caller's own node slot pointer.
    Root,
    /// The `left` slot of the given node.
    LeftOf(u32),
}

/// Outcome of the `0x005e8660` descent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A43TreeProbe {
    /// The root slot was null; the routine returned the caller slot verbatim.
    Empty,
    /// The comparison returned zero; the reported slot links the match.
    Matched(A43TreeSlot),
    /// The left descent reached a null child; that node is returned.
    MissingThrough(u32),
}

/// One node of the retail tree: `+0x0` left child, `+0x2` key pointer,
/// `+0x3` key length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A43TreeNode<'a> {
    /// Key bytes (`*(u32 *)(node + 8)` for `node[3]` bytes).
    pub key: &'a [u8],
    /// Left child index, `None` for a null slot.
    pub left: Option<u32>,
}

/// `FUN_0055c520` (`FUN_0055c520`, 106 bytes), the comparison kernel used by
/// `0x005e8660`.
///
/// The routine compares the first `n` bytes and returns `-1` for "less",
/// `1` for "greater" and `0` for equal (the 32-bit word form
/// `-(uint)less | 1`), including the partial-tail cases.  It never looks past
/// `n`, so the caller composes the full ordering with the length difference.
pub fn a43_key_prefix_compare(left: &[u8], right: &[u8]) -> i32 {
    let n = left.len().min(right.len());
    for i in 0..n {
        if left[i] != right[i] {
            return if left[i] < right[i] { -1 } else { 1 };
        }
    }
    0
}

/// `0x005e8660` (`FUN_005e8660`, 102 bytes): descend the keyed tree.
///
/// ```c
/// if (*slot == 0) return slot;
/// for (;;) {
///   node = *slot;
///   n = min(keyLen, node[3]);
///   cmp = FUN_0055c520(keyPtr, node[2], n);
///   if (cmp == 0) cmp = keyLen - node[3];
///   if (cmp == 0) break;                 // matched
///   slot = node;
///   if (node[0] == 0) return node;       // insert through this node
///   node = node[0];                      // only the left child is followed
/// }
/// return slot;
/// ```
///
/// Two details are preserved because they are visible in the body: the loop
/// follows the **left** child unconditionally (the decompile has no right
/// branch), and a match returns the *previous* slot, not the matching node.
/// `slot` starts as the caller's pointer, modelled by [`A43TreeSlot::Root`].
pub fn a43_tree_probe(root: Option<u32>, nodes: &[A43TreeNode<'_>], key: &[u8]) -> A43TreeProbe {
    let Some(first) = root else {
        return A43TreeProbe::Empty;
    };
    let mut index = first;
    let mut cursor: Option<u32> = None;
    loop {
        let Some(node) = nodes.get(index as usize) else {
            // A dangling child index cannot occur in a well-formed tree; the
            // retail dereference would fault.  Report the node as missing.
            return A43TreeProbe::MissingThrough(index);
        };
        let n = key.len().min(node.key.len());
        let mut cmp = a43_key_prefix_compare(&key[..n], &node.key[..n]);
        if cmp == 0 {
            cmp = (key.len() as u32).wrapping_sub(node.key.len() as u32) as i32;
        }
        if cmp == 0 {
            return A43TreeProbe::Matched(cursor.map_or(A43TreeSlot::Root, A43TreeSlot::LeftOf));
        }
        cursor = Some(index);
        match node.left {
            None => return A43TreeProbe::MissingThrough(index),
            Some(next) => index = next,
        }
    }
}

// ---------------------------------------------------------------------------
// 0x005e8ea0 / 0x005e9170 - bcCopyableFunction storage release
// ---------------------------------------------------------------------------

/// Failure tags for the `bc` storage helpers.  Each one is a
/// `FUN_0093ec70`/`FUN_00923e90` abort in retail; they are returned instead so
/// a caller can tell the fatal branches apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A43Fault {
    /// The storage word still carries the heap tag when it must not.
    HeapFlagged,
    /// A refcount that must already be dead is non-negative.
    RefcountNotNegative,
}

/// Resolved storage of a `bcCopyableFunction` word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A43CallableRef {
    /// Bit 0 clear: the object lives at the storage word itself.
    Inline,
    /// Bit 0 set: the object lives at `word & !1`.
    Heap(u32),
}

/// The `& ~1` heap test shared by `0x005e8ea0`, `0x005e9170`, `FUN_005e9210`,
/// `0x005e9470`, `FUN_005e94f8`, `FUN_005e95c5`, `FUN_005e95f9` and `FUN_005e912a`.
///
/// ```c
/// if ((word & 1) == 0) object = &word; else object = word & 0xfffffffe;
/// ```
///
/// A heap-tagged zero word (`1`) is the null state; the empty word written by
/// the resets is exactly `1`.
pub fn a43_callable_ref(word: u32) -> Option<A43CallableRef> {
    if word & 1 == 0 {
        Some(A43CallableRef::Inline)
    } else {
        match word & 0xffff_fffe {
            0 => None,
            pointer => Some(A43CallableRef::Heap(pointer)),
        }
    }
}

/// `0x005e8ea0` (`FUN_005e8ea0`, 127 bytes): release the storage word at
/// `[ecx + 0x1c]` and then require the `+0x8` refcount to be negative.
///
/// ```c
/// object = (word & 1) ? word & ~1 : &word;
/// if (object) {
///   (*object->vftable[0])(0);            // destroy the stored callable
///   if (word & 1) FUN_0093ec70();        // inline objects must not be heap-tagged
///   word = 1;                            // empty storage word
/// }
/// if (refcount_at_+8 >= 0) FUN_0093ec70();
/// ```
///
/// The vtable destruction is not modelled; the flag transition and the guard
/// are.  On success the new storage word (`1`) is returned.
pub fn a43_release_callable(storage_word: u32, refcount: i32) -> Result<u32, A43Fault> {
    let mut word = storage_word;
    match a43_callable_ref(storage_word) {
        // Null object: the destruction block is skipped, the word is kept.
        None => {}
        Some(A43CallableRef::Inline) => word = 1,
        Some(A43CallableRef::Heap(_)) => return Err(A43Fault::HeapFlagged),
    }
    if refcount >= 0 {
        return Err(A43Fault::RefcountNotNegative);
    }
    Ok(word)
}

/// Result of the `0x005e9170` release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A43CallableRelease {
    /// Storage word after the release (`1` when an object was destroyed).
    pub storage_word: u32,
    /// The low bit of the destructor argument: retail calls the deleting
    /// destructor (`FUN_00923e90`) when it is set.
    pub delete_object: bool,
}

/// `0x005e9170` (`FUN_005e9170`, 125 bytes): release the storage word at
/// `[ecx + 0x4]` and honour the deleting-destructor flag.
///
/// This is the same resolution and heap-tag check as
/// [`a43_release_callable`] on `+0x4`, followed by
/// `if (param_1 & 1) FUN_00923e90();`.  The flag is reported instead of
/// freeing the object.
pub fn a43_release_callable_flagged(
    storage_word: u32,
    delete_flag: u8,
) -> Result<A43CallableRelease, A43Fault> {
    let mut word = storage_word;
    match a43_callable_ref(storage_word) {
        None => {}
        Some(A43CallableRef::Inline) => word = 1,
        Some(A43CallableRef::Heap(_)) => return Err(A43Fault::HeapFlagged),
    }
    Ok(A43CallableRelease {
        storage_word: word,
        delete_object: delete_flag & 1 != 0,
    })
}

// ---------------------------------------------------------------------------
// 0x005e8f50 / 0x005e8fd0 / 0x005e8fdf - refcount guard chains
// ---------------------------------------------------------------------------

/// Guard offsets of `0x005e8f50`, in test order.
pub const A43_DESTRUCTOR_GUARDS: [i32; 2] = [0x24, 0x08];

/// `0x005e8f50` (`FUN_005e8f50`, 96 bytes): the two-refcount destructor gate.
///
/// ```c
/// if (*(int *)(this + 0x24) >= 0) FUN_0093ec70();
/// if (*(int *)(this + 0x8)  >= 0) FUN_0093ec70();
/// ```
///
/// Both words must have the sign bit set (retail keeps them as negative
/// refcounts).  Returns the offset of the first violated guard so the order
/// stays observable.
pub fn a43_guard_pair(values: [i32; 2]) -> Result<(), i32> {
    for (value, offset) in values.iter().zip(A43_DESTRUCTOR_GUARDS) {
        if *value >= 0 {
            return Err(offset);
        }
    }
    Ok(())
}

/// Teardown guard offsets of `FUN_007ed860`, in test order; `0x005e8fd0`
/// tests `0x2a8` first and then tail-calls that routine.
pub const A43_TEARDOWN_GUARDS: [i32; 4] = [0x2a8, 0x270, 0x254, 0x234];
/// Poison written to `+0xd4` by the teardown body.
pub const A43_TEARDOWN_POISON: u32 = 0xbaad_f00d;

/// Observable result of [`a43_teardown`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A43Teardown {
    /// `Some(0xbaadf00d)` once the poison word has been written.
    pub poison: Option<u32>,
    /// Offset of the first non-negative refcount, if any.
    pub failed: Option<i32>,
}

/// `0x005e8fd0` (`FUN_005e8fd0`, 10 bytes): the resource teardown gate.
///
/// ```c
/// if (*(int *)(this + 0x2a8) >= 0) FUN_0093ec70();
/// FUN_007ed860();          // 0x5e8fd0 tail-call
/// ```
///
/// `FUN_007ed860` writes `0xbaadf00d` to `+0xd4` and then checks `+0x270`,
/// `+0x254` and `+0x234` in that order.  The poison write therefore happens
/// between the first and second guard, which is why the result records both.
pub fn a43_teardown(values: [i32; 4]) -> A43Teardown {
    if values[0] >= 0 {
        return A43Teardown {
            poison: None,
            failed: Some(A43_TEARDOWN_GUARDS[0]),
        };
    }
    for (value, offset) in values[1..].iter().zip(A43_TEARDOWN_GUARDS.iter().skip(1)) {
        if *value >= 0 {
            return A43Teardown {
                poison: Some(A43_TEARDOWN_POISON),
                failed: Some(*offset),
            };
        }
    }
    A43Teardown {
        poison: Some(A43_TEARDOWN_POISON),
        failed: None,
    }
}

/// `0x005e8fdf` (`FUN_005e8fdf`, 75 bytes): the same gate with the frame
/// bookkeeping visible (`[ebp - 4] = 0` before the `+0x2a8` test).
///
/// The row is the split continuation of `0x005e8fd0`; the guard order and the
/// tail-call are identical.
pub fn a43_teardown_split(values: [i32; 4]) -> A43Teardown {
    a43_teardown(values)
}

// ---------------------------------------------------------------------------
// 0x005e9040 / 0x005e905b - recovered key triple gate
// ---------------------------------------------------------------------------

/// Upper bound of the recovered record index (`0x14`).
pub const A43_KEY_INDEX_LIMIT: u32 = 0x14;
/// Upper bound of the recovered record sub-index (`4`).
pub const A43_KEY_SUB_LIMIT: u32 = 0x4;

/// `0x005e9040` (`FUN_005e9040`, 27 bytes): the key/record range gate.
///
/// ```c
/// if (present == 0 || index > 0x14 || sub > 4) FUN_0093ec70();
/// ```
///
/// The three components come back from the virtual getter
/// `(**(code **)(*object + 8))(&index)`; `present` is the byte it returns at
/// `-0x2c`, `index` the word at `-0x34` and `sub` the word at `-0x30`.
pub fn a43_key_in_range(present: bool, index: u32, sub: u32) -> bool {
    present && index <= A43_KEY_INDEX_LIMIT && sub <= A43_KEY_SUB_LIMIT
}

/// `0x005e905b` (`FUN_005e905b`, 101 bytes): the same triple gate reached
/// from the caller frame.
///
/// The body reads the object through `[ebp + 8]`, resolves the heap tag with
/// the usual `& ~1` test, then performs the identical
/// `present/index/sub` checks.  It is the split entry of `0x005e9040`.
pub fn a43_key_in_range_entry(present: bool, index: u32, sub: u32) -> bool {
    a43_key_in_range(present, index, sub)
}

// ---------------------------------------------------------------------------
// 0x005e9470 / 0x005e9481 / 0x005e98b0 - hash bucket arithmetic
// ---------------------------------------------------------------------------

/// MurmurHash3 x86-32 block multiplier.
pub const A43_HASH_C1: u32 = 0xcc9e_2d51;
/// MurmurHash3 x86-32 rotate multiplier.
pub const A43_HASH_C2: u32 = 0x1b87_3593;
/// MurmurHash3 x86-32 block addend.
pub const A43_HASH_MIX_ADD: u32 = 0xe654_6b64;
/// MurmurHash3 x86-32 first finalizer multiplier.
pub const A43_HASH_FINAL1: u32 = 0x85eb_ca6b;
/// MurmurHash3 x86-32 second finalizer multiplier.
pub const A43_HASH_FINAL2: u32 = 0xc2b2_ae35;

/// `FUN_00c62800` (`FUN_00c62800`, 166 bytes): MurmurHash3 x86-32 with seed 0.
///
/// The decompiled body contains the two split-multiply forms Ghidra emits for
/// the 32-bit constants: `(x * 0x16a88000) | (x * 0xcc9e2d51 >> 17)` is
/// `rotl32(x * 0xcc9e2d51, 15)`, and `(h + 0xfaddaf14) * 5` is
/// `h * 5 + 0xe6546b64` (both identities verified).  The tail byte order is
/// little-endian (`b0 | b1 << 8 | b2 << 16`), the block count is `len >> 2`,
/// and the finalizer uses `0x85ebca6b`/`0xc2b2ae35`.  Reproduced here because
/// three rows in this batch (`0x005e9470`, `0x005e9481`, `0x005e98b0`) call
/// it.
pub fn a43_hash_bytes(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0;
    let mut offset = 0usize;
    while offset + 4 <= bytes.len() {
        let mut k = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        k = k.wrapping_mul(A43_HASH_C1);
        k = k.rotate_left(15);
        k = k.wrapping_mul(A43_HASH_C2);
        hash ^= k;
        hash = hash.rotate_left(13);
        hash = hash.wrapping_mul(5).wrapping_add(A43_HASH_MIX_ADD);
        offset += 4;
    }
    if offset < bytes.len() {
        let mut k: u32 = 0;
        for (shift, byte) in bytes[offset..].iter().enumerate() {
            k |= (*byte as u32) << (8 * shift);
        }
        k = k.wrapping_mul(A43_HASH_C1);
        k = k.rotate_left(15);
        k = k.wrapping_mul(A43_HASH_C2);
        hash ^= k;
    }
    hash ^= bytes.len() as u32;
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(A43_HASH_FINAL1);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(A43_HASH_FINAL2);
    hash ^= hash >> 16;
    hash
}

/// Result of the `0x005e9470` insert-slot computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A43HashInsert {
    /// The bucket array is empty (`in_ECX[1] == 0`): the growth path runs
    /// before the slot is used.
    pub grow: bool,
    /// `(bucket_count - 1) & hash`, once the bucket array exists.
    pub slot: Option<u32>,
}

/// `0x005e9470` (`FUN_005e9470`, 17 bytes): the insert slot of a key.
///
/// ```c
/// if (this[1] == 0) FUN_0053cdd0(this[0]);       // grow the bucket store
/// hash = FUN_00c62800(key.ptr, key.len, 0);
/// slot = this[1] + (this[0] - 1 & hash) * 4;
/// node = FUN_00553f50(slot, &key);
/// if (*node == 0) FUN_0093ec70();
/// FUN_005e9040(&value);
/// ```
///
/// The bucket count is `this[0]` and the bucket base `this[1]`; the mask is
/// the usual power-of-two `count - 1`.  The bucket-tree walk and the value
/// gate are separate rows, so only the growth flag and the masked slot are
/// returned.
pub fn a43_hash_insert_slot(bucket_count: u32, key: &[u8]) -> A43HashInsert {
    if bucket_count == 0 {
        return A43HashInsert {
            grow: true,
            slot: None,
        };
    }
    let hash = a43_hash_bytes(key);
    A43HashInsert {
        grow: false,
        slot: Some(bucket_count.wrapping_sub(1) & hash),
    }
}

/// `0x005e9481` (`FUN_005e9481`, 119 bytes): the same insert body with the
/// record read through `[ebp + 8]`.
///
/// The only difference from `0x005e9470` is the frame layout (`local_8 = 1`
/// after the hash, and the value passed to `0x005e9040` comes from
/// `[ebp + 0xc]`); the growth test and the `(count - 1) & hash` mask match.
pub fn a43_hash_insert_slot_entry(bucket_count: u32, key: &[u8]) -> A43HashInsert {
    a43_hash_insert_slot(bucket_count, key)
}

/// Raw three-word result of the `0x005e98b0` probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A43HashRange {
    /// First output word: the non-null node the bucket walk returned.
    pub node: Option<u32>,
    /// Second output word: the bucket slot offset (`slot * 4`) when the key is
    /// present, otherwise the map end offset.
    pub next_offset: u32,
    /// Third output word: `bucket_count * 4`.
    pub end_offset: u32,
}

/// `0x005e98b0` (`FUN_005e98b0`, 125 bytes): the bucket range left behind by
/// a lookup.
///
/// ```c
/// if (this[1] == 0) {                    // no bucket base yet
///   out[0] = 0;
///   out[1] = this[0] << 2;
///   out[2] = this[0] << 2;
///   return;
/// }
/// hash  = FUN_00c62800(key.ptr, key.len, 0);
/// slot  = this[1] + (this[0] - 1 & hash) * 4;
/// node  = FUN_00553f50(slot, &key);      // bucket-tree walk
/// out[0] = *node;
/// out[1] = (*node != 0) ? slot : this[1] + this[0] * 4;
/// out[2] = this[1] + this[0] * 4;
/// ```
///
/// `buckets_allocated` is `this[1] != 0` and `found_node` is the non-null
/// result of the bucket-tree walk, which is not part of this row's arithmetic.
/// Offsets are reported relative to `this[1]`, so `slot` is
/// `(bucket_count - 1 & hash) * 4`.
pub fn a43_hash_probe(
    bucket_count: u32,
    buckets_allocated: bool,
    key: &[u8],
    found_node: Option<u32>,
) -> A43HashRange {
    let end = bucket_count.wrapping_mul(4);
    if !buckets_allocated {
        return A43HashRange {
            node: None,
            next_offset: end,
            end_offset: end,
        };
    }
    let hash = a43_hash_bytes(key);
    let slot = (bucket_count.wrapping_sub(1) & hash).wrapping_mul(4);
    match found_node {
        Some(node) => A43HashRange {
            node: Some(node),
            next_offset: slot,
            end_offset: end,
        },
        None => A43HashRange {
            node: None,
            next_offset: end,
            end_offset: end,
        },
    }
}

// ---------------------------------------------------------------------------
// 0x005e952d - load-factor growth check
// ---------------------------------------------------------------------------

/// `0x005e952d` (`FUN_005e952d`, 150 bytes): the hash-container load-factor
/// check plus the empty node header.
///
/// ```c
/// node[+0x24] = node[+0x28] = node[+0x2c] = node[+0x30] = 0;
/// node[+0x20] = 1;                       // empty callable storage word
/// *link = node;
/// ++elements;
/// f = (float)elements / (float)buckets;
/// if ((float)max_load <= f && f != (float)max_load) FUN_0053cdd0(buckets * 2);
/// ```
///
/// The `(double)value + DAT_00d33180[-(value >> 31)]` pairs are the compiler's
/// unsigned-int-to-double conversion (`+0.0` for non-negative values,
/// `+2^32` for negative ones); after the cast to `float` the quotient is a
/// plain `f32` division.  The growth routine is not reimplemented: the new
/// bucket count it is called with is reported as `grow_to`.
pub struct A43GrowthProbe {
    /// The five words written at `+0x20..=+0x30`.
    pub node_words: [u32; 5],
    /// `Some(buckets * 2)` when the load factor exceeded `max_load`.
    pub grow_to: Option<u32>,
}

/// See [`A43GrowthProbe`].  `elements_before` is the counter before the
/// increment performed by the routine.
pub fn a43_growth_probe(buckets: u32, elements_before: u32, max_load: u32) -> A43GrowthProbe {
    let elements = elements_before.wrapping_add(1);
    let fraction = elements as f32 / buckets as f32;
    let limit = max_load as f32;
    let grow_to = if limit <= fraction && fraction != limit {
        Some(buckets.wrapping_mul(2))
    } else {
        None
    };
    A43GrowthProbe {
        node_words: [1, 0, 0, 0, 0],
        grow_to,
    }
}

// ---------------------------------------------------------------------------
// 0x005e9890 - layout descriptor
// ---------------------------------------------------------------------------

/// Descriptor written by `0x005e9890`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A43LayoutDescriptor {
    /// Word at `+0x0` (`0x18`).
    pub bits: u32,
    /// Word at `+0x4` (`4`).
    pub bytes: u32,
    /// Byte at `+0x8` (`0`).
    pub flag: u8,
}

/// `0x005e9890` (`FUN_005e9890`, 27 bytes): `{0x18, 4, 0}`.
///
/// ```c
/// param_1[0] = 0x18;
/// param_1[1] = 4;
/// *(u8 *)(param_1 + 2) = 0;
/// ```
///
/// The three fields are stored as two words and one byte, i.e. a 0x18-bit
/// format with a 4-byte stride and a clear trailing flag.
pub fn a43_layout_descriptor() -> A43LayoutDescriptor {
    A43LayoutDescriptor {
        bits: 0x18,
        bytes: 4,
        flag: 0,
    }
}

// ---------------------------------------------------------------------------
// 0x005e9990 / 0x005e9fc0 - 24-byte small-string header
// ---------------------------------------------------------------------------

/// Inline capacity boundary of the 24-byte header (`0xf < capacity` = heap).
pub const A43_SMALL_CAPACITY: u32 = 0xf;
/// Number of words in the header (`24` bytes).
pub const A43_SMALL_WORDS: usize = 6;

/// Data side of [`a43_small_string_view`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A43SmallStringData {
    /// Bit 0 of the capacity is not the test; the 16 inline bytes are words
    /// `0..=3` of the header.
    Inline([u32; 4]),
    /// Heap form: word 0 of the header is the data pointer.
    Heap(u32),
}

/// View produced by `0x005e9990`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A43SmallStringView {
    /// Word at `+0x10` (the length).
    pub length: u32,
    /// Where the bytes live.
    pub data: A43SmallStringData,
}

/// `0x005e9990` (`FUN_005e9990`, 32 bytes): split the header into
/// `(pointer, length)`.
///
/// ```c
/// length = header[4];
/// if (0xf < header[5]) pointer = *(u32 *)header; else pointer = header;
/// FUN_005ef9b0(out, pointer, length);
/// ```
///
/// `+0x10` holds the length and `+0x14` the capacity; capacity above `0xf`
/// means the first word is a heap pointer, otherwise the header itself is the
/// buffer.
pub fn a43_small_string_view(header: &[u32; A43_SMALL_WORDS]) -> A43SmallStringView {
    let length = header[4];
    let data = if header[5] > A43_SMALL_CAPACITY {
        A43SmallStringData::Heap(header[0])
    } else {
        A43SmallStringData::Inline([header[0], header[1], header[2], header[3]])
    };
    A43SmallStringView { length, data }
}

/// Candidate chosen by [`a43_string_assign_choice`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A43StringSource {
    /// The first header.
    First,
    /// The second header.
    Second,
}

/// Result of `0x005e9fc0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A43StringAssign {
    /// Which candidate was moved.
    pub source: A43StringSource,
    /// The 24-byte header copied into the destination.
    pub dest: [u32; A43_SMALL_WORDS],
    /// The chosen header after the retail reset (`length = 0`,
    /// `capacity = 0xf`, first byte cleared).
    pub cleared: [u32; A43_SMALL_WORDS],
}

/// `0x005e9fc0` (`FUN_005e9fc0`, 123 bytes): choose which of two string
/// headers is moved into the destination.
///
/// ```c
/// len1 = first[4];  len2 = second[4];
/// if (first[5] - len1 < len2 && len1 <= second[5] - len2)
///   temp = copy_of(first);          // FUN_006000c0
/// else
///   temp = copy_of(second);         // FUN_005fd9c0
/// *dest = temp;                     // 24 bytes
/// temp[4] = 0; temp[5] = 0xf; *(u8 *)temp = 0;
/// ```
///
/// Both subtractions are unsigned (`u32`) and wrap, exactly like the retail
/// words.  The first branch is taken when the first header's spare capacity is
/// smaller than the second length **and** the first length fits the second
/// header's spare capacity; otherwise the second header is used.
pub fn a43_string_assign_choice(
    first: [u32; A43_SMALL_WORDS],
    second: [u32; A43_SMALL_WORDS],
) -> A43StringAssign {
    let len1 = first[4];
    let len2 = second[4];
    let use_first = first[5].wrapping_sub(len1) < len2 && len1 <= second[5].wrapping_sub(len2);
    let (source, chosen) = if use_first {
        (A43StringSource::First, first)
    } else {
        (A43StringSource::Second, second)
    };
    let mut cleared = chosen;
    cleared[4] = 0;
    cleared[5] = A43_SMALL_CAPACITY;
    // `*(u8 *)temp = 0` clears only the low byte of the first word.
    cleared[0] &= 0xffff_ff00;
    A43StringAssign {
        source,
        dest: chosen,
        cleared,
    }
}

// ---------------------------------------------------------------------------
// 0x005e9f20 / 0x005e9f21 - stream cursor equality
// ---------------------------------------------------------------------------

/// Iterator state of the retail input stream.
///
/// The retail struct is `{ stream_word, checked_byte(+4), peek_byte(+5) }`;
/// end-of-stream is represented by a null stream word, modelled here by
/// [`A43StreamCursor::at_end`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A43StreamCursor {
    /// `+4`: the peek has already been resolved.
    pub checked: bool,
    /// `*cursor == 0`: the stream word was replaced by the end marker.
    pub at_end: bool,
    /// `+5`: the peeked byte.
    pub peek: u8,
}

/// Resolve the peek byte once, as both bodies do inline.
///
/// ```c
/// if (checked) skip;
/// byte = (buffer && remaining > 0) ? *buffer : (*vtable[0x18])();
/// if (byte == 0xffffffff) *stream = 0; else peek = byte;
/// checked = 1;
/// ```
///
/// `next` is the byte that the buffer/fill path would produce; `None` is the
/// `0xffffffff` end-of-stream marker.
pub fn a43_stream_fill(cursor: &mut A43StreamCursor, next: Option<u8>) {
    if cursor.checked {
        return;
    }
    match next {
        None => cursor.at_end = true,
        Some(byte) => {
            cursor.peek = byte;
            cursor.at_end = false;
        }
    }
    cursor.checked = true;
}

/// `0x005e9f20` (`FUN_005e9f20`, ledger size 1; the body is the shared
/// `0x005e9f21` sequence): compare two input-stream cursors.
///
/// After both cursors are filled, the routine returns `1` when both stream
/// words are null or both are non-null, and `0` otherwise:
///
/// ```c
/// if (*a == 0) { if (*b == 0) return 1; }
/// else if (*b != 0) return 1;
/// return 0;
/// ```
///
/// That is exactly [`A43StreamCursor::at_end`] equality.
pub fn a43_stream_cursor_match(
    first: &mut A43StreamCursor,
    first_next: Option<u8>,
    second: &mut A43StreamCursor,
    second_next: Option<u8>,
) -> bool {
    a43_stream_fill(first, first_next);
    a43_stream_fill(second, second_next);
    first.at_end == second.at_end
}

/// `0x005e9f21` (`FUN_005e9f21`, 153 bytes): the same comparison with the
/// unused leading argument of the split entry.
///
/// The decompiled first parameter (`[ebp + 8]`) is never read; it is kept so
/// the split entry's calling convention is visible.  The two cursor bodies and
/// the final equality test are identical to [`a43_stream_cursor_match`].
pub fn a43_stream_cursor_match_entry(
    _unused: u32,
    first: &mut A43StreamCursor,
    first_next: Option<u8>,
    second: &mut A43StreamCursor,
    second_next: Option<u8>,
) -> bool {
    a43_stream_cursor_match(first, first_next, second, second_next)
}

#[cfg(test)]
mod tests_agent43 {
    use super::*;

    // -- 0x005e8400 / 0x005e842a -------------------------------------------

    #[test]
    fn value_store_uses_the_inline_spare_counter() {
        assert_eq!(
            a43_value_store(0),
            A43ValueStore::Inline {
                spare: 0xd,
                tag: 0x1c05_0d00,
            }
        );
        assert_eq!(
            a43_value_store(0xc),
            A43ValueStore::Inline {
                spare: 1,
                tag: 0x1c05_0100,
            }
        );
        assert_eq!(
            a43_value_store(0xd),
            A43ValueStore::Inline {
                spare: 0,
                tag: 0x1c05_0000,
            }
        );
        assert_eq!(
            a43_value_store_from_frame(0xd),
            A43ValueStore::Inline {
                spare: 0,
                tag: 0x1c05_0000,
            }
        );
    }

    #[test]
    fn value_store_allocates_len_plus_one_on_the_heap_form() {
        assert_eq!(
            a43_value_store(0xe),
            A43ValueStore::Heap {
                length: 0xe,
                allocation: 0xf,
                tag: 0x0c05_0000,
            }
        );
        assert_eq!(
            a43_value_store(0x1000),
            A43ValueStore::Heap {
                length: 0x1000,
                allocation: 0x1001,
                tag: 0x0c05_0000,
            }
        );
        // `len + 1` is a plain 32-bit increment in retail.
        assert_eq!(
            a43_value_store(u32::MAX),
            A43ValueStore::Heap {
                length: u32::MAX,
                allocation: 0,
                tag: 0x0c05_0000,
            }
        );
    }

    // -- 0x005e84f0 / 0x005e8500 / 0x005e8514 ------------------------------

    #[test]
    fn event_word_sets_bit_21_only_for_non_negative_values() {
        assert_eq!(a43_event_info_word(0), 0x1f6_0000);
        assert_eq!(a43_event_info_word(0x7fff_ffff), 0x1f6_0000);
        assert_eq!(a43_event_info_word(0x8000_0000), 0x1d6_0000);
        assert_eq!(a43_event_info_word(0xffff_ffff), 0x1d6_0000);
        // Bit 26 alone does not change the non-negative verdict.
        assert_eq!(a43_event_info_word(0x0400_0000), 0x1f6_0000);
    }

    #[test]
    fn event_header_writes_value_and_zero_then_the_low_word() {
        assert_eq!(a43_event_header_word(0xdead_beef), (0xdead_beef, 0, 0x1d6));
        assert_eq!(a43_event_header_word(0x1234), (0x1234, 0, 0x1f6));
        // The 16-bit word is the low half of the packed word.
        for value in [0u32, 1, 0x8000_0000, 0x7fff_ffff, 0xffff_ffff] {
            assert_eq!(
                a43_event_header_word(value).2 as u32,
                a43_event_info_word(value) >> 16
            );
        }
    }

    // -- 0x005e85b1 / 0x005e85c0 -------------------------------------------

    #[test]
    fn dialog_flags_follow_the_selector_sign_and_zero_mask() {
        assert_eq!(a43_dialog_flags(-1, 0), 0x0116);
        assert_eq!(a43_dialog_flags(i32::MIN, 0xffff_ffff), 0x0116);
        assert_eq!(a43_dialog_flags(1, 0), 0x0196);
        assert_eq!(a43_dialog_flags(i32::MAX, 0x8000_0000), 0x0196);
        assert_eq!(a43_dialog_flags(0, 0x8000_0000), 0x01d6);
        assert_eq!(a43_dialog_flags(0, 0), 0x01f6);
        assert_eq!(a43_dialog_flags(0, 0x7fff_ffff), 0x01f6);
    }

    #[test]
    fn dialog_info_word_is_flags_shifted_sixteen() {
        for (selector, mask) in [(-5, 0u32), (0, 0), (0, 0x8000_0000), (9, 0x8000_0000)] {
            assert_eq!(
                a43_dialog_info_word(selector, mask),
                (a43_dialog_flags(selector, mask) as u32) << 16
            );
        }
    }

    // -- 0x005e8660 --------------------------------------------------------

    const TREE: [A43TreeNode<'static>; 3] = [
        A43TreeNode {
            key: b"m",
            left: Some(1),
        },
        A43TreeNode {
            key: b"b",
            left: Some(2),
        },
        A43TreeNode {
            key: b"a",
            left: None,
        },
    ];

    #[test]
    fn tree_probe_distinguishes_root_match_from_a_link_slot() {
        assert_eq!(
            a43_tree_probe(Some(0), &TREE, b"m"),
            A43TreeProbe::Matched(A43TreeSlot::Root)
        );
        assert_eq!(
            a43_tree_probe(Some(0), &TREE, b"a"),
            A43TreeProbe::Matched(A43TreeSlot::LeftOf(1))
        );
        assert_eq!(a43_tree_probe(None, &TREE, b"a"), A43TreeProbe::Empty);
    }

    #[test]
    fn tree_probe_follows_only_the_left_child() {
        // "z" is greater than every node, yet the body always descends left.
        assert_eq!(
            a43_tree_probe(Some(0), &TREE, b"z"),
            A43TreeProbe::MissingThrough(2)
        );
        // "b" matches the second node.
        assert_eq!(
            a43_tree_probe(Some(0), &TREE, b"b"),
            A43TreeProbe::Matched(A43TreeSlot::LeftOf(0))
        );
    }

    #[test]
    fn key_prefix_compare_matches_the_three_way_kernel() {
        assert_eq!(a43_key_prefix_compare(b"", b""), 0);
        assert_eq!(a43_key_prefix_compare(b"a", b"a"), 0);
        assert_eq!(a43_key_prefix_compare(b"ab", b"abc"), 0);
        assert_eq!(a43_key_prefix_compare(b"abc", b"abd"), -1);
        assert_eq!(a43_key_prefix_compare(b"abd", b"abc"), 1);
        assert_eq!(a43_key_prefix_compare(b"b", b"a"), 1);
    }

    // -- 0x005e8ea0 / 0x005e9170 -------------------------------------------

    #[test]
    fn callable_storage_tag_is_bit_zero() {
        assert_eq!(a43_callable_ref(0), Some(A43CallableRef::Inline));
        assert_eq!(a43_callable_ref(2), Some(A43CallableRef::Inline));
        assert_eq!(a43_callable_ref(1), None);
        assert_eq!(a43_callable_ref(0x1001), Some(A43CallableRef::Heap(0x1000)));
    }

    #[test]
    fn release_callable_clears_inline_storage_and_checks_the_refcount() {
        assert_eq!(a43_release_callable(0, -1), Ok(1));
        assert_eq!(a43_release_callable(2, -2), Ok(1));
        assert_eq!(a43_release_callable(1, -1), Ok(1));
        assert_eq!(
            a43_release_callable(0, 0),
            Err(A43Fault::RefcountNotNegative)
        );
        assert_eq!(
            a43_release_callable(1, 0),
            Err(A43Fault::RefcountNotNegative)
        );
        assert_eq!(a43_release_callable(0x1001, -1), Err(A43Fault::HeapFlagged));
    }

    #[test]
    fn flagged_release_reports_the_deleting_destructor() {
        assert_eq!(
            a43_release_callable_flagged(0, 0),
            Ok(A43CallableRelease {
                storage_word: 1,
                delete_object: false,
            })
        );
        assert_eq!(
            a43_release_callable_flagged(0, 1),
            Ok(A43CallableRelease {
                storage_word: 1,
                delete_object: true,
            })
        );
        assert_eq!(
            a43_release_callable_flagged(1, 0xfe),
            Ok(A43CallableRelease {
                storage_word: 1,
                delete_object: false,
            })
        );
        assert_eq!(
            a43_release_callable_flagged(0x1001, 0),
            Err(A43Fault::HeapFlagged)
        );
    }

    // -- 0x005e8f50 / 0x005e8fd0 / 0x005e8fdf ------------------------------

    #[test]
    fn destructor_gate_checks_0x24_before_0x8() {
        assert_eq!(a43_guard_pair([-1, -1]), Ok(()));
        assert_eq!(a43_guard_pair([0, -1]), Err(0x24));
        assert_eq!(a43_guard_pair([-1, 0]), Err(0x8));
        assert_eq!(a43_guard_pair([0, 0]), Err(0x24));
    }

    #[test]
    fn teardown_writes_the_poison_between_the_first_two_guards() {
        assert_eq!(
            a43_teardown([-1, -1, -1, -1]),
            A43Teardown {
                poison: Some(0xbaad_f00d),
                failed: None,
            }
        );
        assert_eq!(
            a43_teardown([0, -1, -1, -1]),
            A43Teardown {
                poison: None,
                failed: Some(0x2a8),
            }
        );
        assert_eq!(
            a43_teardown([-1, 0, -1, -1]),
            A43Teardown {
                poison: Some(0xbaad_f00d),
                failed: Some(0x270),
            }
        );
        assert_eq!(
            a43_teardown([-1, -1, -1, 0]),
            A43Teardown {
                poison: Some(0xbaad_f00d),
                failed: Some(0x234),
            }
        );
        assert_eq!(
            a43_teardown_split([-1, -1, -1, -1]),
            a43_teardown([-1, -1, -1, -1])
        );
    }

    // -- 0x005e9040 / 0x005e905b -------------------------------------------

    #[test]
    fn key_gate_accepts_the_inclusive_bounds() {
        assert!(a43_key_in_range(true, 0, 0));
        assert!(a43_key_in_range(true, 0x14, 4));
        assert!(!a43_key_in_range(true, 0x15, 0));
        assert!(!a43_key_in_range(true, 0, 5));
        assert!(!a43_key_in_range(false, 0, 0));
        assert!(a43_key_in_range_entry(true, 0x14, 4));
        assert!(!a43_key_in_range_entry(false, 0x14, 4));
    }

    // -- FUN_00c62800 / 0x005e9470 / 0x005e9481 / 0x005e98b0 ----------------

    #[test]
    fn hash_is_murmurhash3_x86_32_with_seed_zero() {
        assert_eq!(a43_hash_bytes(b""), 0x0000_0000);
        assert_eq!(a43_hash_bytes(b"a"), 0x3c25_69b2);
        assert_eq!(a43_hash_bytes(b"ab"), 0x9bbf_d75f);
        assert_eq!(a43_hash_bytes(b"abc"), 0xb3dd_93fa);
        assert_eq!(a43_hash_bytes(b"abcd"), 0x43ed_676a);
        assert_eq!(a43_hash_bytes(b"hello"), 0x248b_fa47);
        assert_eq!(a43_hash_bytes(b"StarCraft"), 0xa860_ba90);
        assert_eq!(a43_hash_bytes(b"0123456789abcdef"), 0x36c7_e0df);
    }

    #[test]
    fn insert_slot_is_masked_by_bucket_count_minus_one() {
        assert_eq!(
            a43_hash_insert_slot(0, b"hello"),
            A43HashInsert {
                grow: true,
                slot: None,
            }
        );
        assert_eq!(
            a43_hash_insert_slot(8, b"hello"),
            A43HashInsert {
                grow: false,
                slot: Some(7),
            }
        );
        // 0xb3dd93fa & 6 == 2.
        assert_eq!(
            a43_hash_insert_slot(7, b"abc"),
            A43HashInsert {
                grow: false,
                slot: Some(2),
            }
        );
        assert_eq!(
            a43_hash_insert_slot_entry(4, b"a"),
            a43_hash_insert_slot(4, b"a")
        );
    }

    #[test]
    fn probe_reports_the_slot_only_when_the_bucket_holds_the_key() {
        let absent = a43_hash_probe(8, true, b"hello", None);
        assert_eq!(absent.node, None);
        assert_eq!(absent.next_offset, 32);
        assert_eq!(absent.end_offset, 32);

        let present = a43_hash_probe(8, true, b"hello", Some(0x1234));
        assert_eq!(present.node, Some(0x1234));
        assert_eq!(present.next_offset, 28);
        assert_eq!(present.end_offset, 32);

        // 0x3c2569b2 & 15 == 2 => slot 8.
        let low = a43_hash_probe(16, true, b"a", Some(0x99));
        assert_eq!(low.next_offset, 8);
        assert_eq!(low.end_offset, 64);

        let unallocated = a43_hash_probe(8, false, b"hello", None);
        assert_eq!(
            unallocated,
            A43HashRange {
                node: None,
                next_offset: 32,
                end_offset: 32,
            }
        );
    }

    // -- 0x005e952d --------------------------------------------------------

    #[test]
    fn growth_probe_grows_at_the_strictly_greater_load_factor() {
        // 8/8 == 1.0 and the limit is 1: not strictly greater.
        let at_limit = a43_growth_probe(8, 7, 1);
        assert_eq!(at_limit.node_words, [1, 0, 0, 0, 0]);
        assert_eq!(at_limit.grow_to, None);

        // 9/8 = 1.125 > 1.
        let over = a43_growth_probe(8, 8, 1);
        assert_eq!(over.grow_to, Some(16));
        assert_eq!(over.node_words, [1, 0, 0, 0, 0]);

        // 1/4 = 0.25 > 0.
        assert_eq!(a43_growth_probe(4, 0, 0).grow_to, Some(8));
        // 1/3 is compared in f32, still greater than 0.3.
        assert_eq!(a43_growth_probe(3, 0, 0).grow_to, Some(6));
        // Negative loads never trigger the growth call.
        assert_eq!(a43_growth_probe(3, 0, u32::MAX).grow_to, None);
    }

    // -- 0x005e9890 --------------------------------------------------------

    #[test]
    fn layout_descriptor_is_24_bits_per_four_bytes() {
        assert_eq!(
            a43_layout_descriptor(),
            A43LayoutDescriptor {
                bits: 0x18,
                bytes: 4,
                flag: 0,
            }
        );
    }

    // -- 0x005e9990 / 0x005e9fc0 -------------------------------------------

    #[test]
    fn small_string_view_switches_at_capacity_sixteen() {
        let inline = [0x1111_1111, 0x2222_2222, 0x3333_3333, 0x4444_4444, 5, 0xf];
        assert_eq!(
            a43_small_string_view(&inline),
            A43SmallStringView {
                length: 5,
                data: A43SmallStringData::Inline([
                    0x1111_1111,
                    0x2222_2222,
                    0x3333_3333,
                    0x4444_4444,
                ]),
            }
        );
        let heap = [0x1234_5678, 0, 0, 0, 0x20, 0x10];
        assert_eq!(
            a43_small_string_view(&heap),
            A43SmallStringView {
                length: 0x20,
                data: A43SmallStringData::Heap(0x1234_5678),
            }
        );
    }

    #[test]
    fn string_assign_picks_the_header_with_less_spare_capacity() {
        let first = [0xaa, 0, 0, 0, 10, 10];
        let second = [0xbb, 0, 0, 0, 4, 15];
        // first spare = 0 < 4 and 10 <= 15 - 4: choose the first header.
        let picked = a43_string_assign_choice(first, second);
        assert_eq!(picked.source, A43StringSource::First);
        assert_eq!(picked.dest, first);
        assert_eq!(
            picked.cleared,
            [0xaa & 0xffff_ff00, 0, 0, 0, 0, A43_SMALL_CAPACITY]
        );

        // first spare = 3 < 4 but 12 > 11: fall back to the second header.
        let first = [0xaa, 0, 0, 0, 12, 15];
        let picked = a43_string_assign_choice(first, second);
        assert_eq!(picked.source, A43StringSource::Second);
        assert_eq!(picked.dest, second);
        assert_eq!(picked.cleared, [0xbb & 0xffff_ff00, 0, 0, 0, 0, 0xf]);

        // Equal spare capacity is not "less": the second header wins.
        let first = [0, 0, 0, 0, 11, 15];
        let second = [0, 0, 0, 0, 3, 15];
        assert_eq!(
            a43_string_assign_choice(first, second).source,
            A43StringSource::Second
        );
    }

    // -- 0x005e9f20 / 0x005e9f21 -------------------------------------------

    #[test]
    fn stream_cursors_compare_their_end_markers() {
        let mut first = A43StreamCursor::default();
        let mut second = A43StreamCursor::default();
        assert!(a43_stream_cursor_match(&mut first, None, &mut second, None));
        assert!(first.at_end && second.at_end);

        let mut first = A43StreamCursor::default();
        let mut second = A43StreamCursor::default();
        // One side at the end, the other still holding a byte: not equal.
        assert!(!a43_stream_cursor_match(
            &mut first,
            None,
            &mut second,
            Some(7)
        ));
        assert_eq!(second.peek, 7);
        assert!(!second.at_end);

        let mut first = A43StreamCursor::default();
        let mut second = A43StreamCursor::default();
        assert!(a43_stream_cursor_match(
            &mut first,
            Some(1),
            &mut second,
            Some(2)
        ));

        let mut third = A43StreamCursor::default();
        let mut fourth = A43StreamCursor::default();
        assert!(a43_stream_cursor_match_entry(
            0,
            &mut third,
            None,
            &mut fourth,
            None
        ));
        assert!(third.at_end && fourth.at_end);
    }

    #[test]
    fn stream_fill_only_reads_once() {
        let mut cursor = A43StreamCursor::default();
        a43_stream_fill(&mut cursor, Some(0x41));
        assert_eq!(
            cursor,
            A43StreamCursor {
                checked: true,
                at_end: false,
                peek: 0x41,
            }
        );
        // A second fill with an end marker is ignored: the checked flag wins.
        a43_stream_fill(&mut cursor, None);
        assert!(!cursor.at_end);
        assert_eq!(cursor.peek, 0x41);
    }
}
