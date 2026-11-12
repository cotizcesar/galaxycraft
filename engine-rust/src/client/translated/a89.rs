#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent89.rs
// ===========================================================================
// Translation batch 89.  Only this file is edited by the batch.
//
// The `decompiled_uncited` rows in `0x00644000..=0x00645fff` are not
// simulation code.  They are the compiler-emitted MSVC exception/unwind
// funclets of one classic-client routine: the "Session map/replay file
// history" dialog.  The dialog appends a `{name, value}` record to a
// `std::set`/`std::map`, prints the recovered header and count line
// (`0x00644130`) and then tears the temporary string records down through a
// five-member destructor loop.  The decompiler split that single routine into
// one row per EH state: the bodies stop at the capacity helper
// `FUN_00538810` (an inlined reserve) and at `FUN_0093ec70` (the
// allocation-failure tail), and the live registers of the shared loop survive
// only as `unaff_*`.  What *is* exactly recoverable is the arithmetic, the
// literal strings and the object layout; those are what the functions below
// reproduce.
//
// Layout recovered from `0x00644080`, `0x006447e0` and the destructor rows:
// the string object is 28 bytes, `data` at `+0x00`, `size` at `+0x04`, the
// capacity word at `+0x08` and a 16-byte inline buffer at `+0x0c`.  Bit 31 of
// the capacity word marks the inline buffer; the low bits hold the capacity
// (`15` while inline).  The string constructor seeds `data = object + 0xc`,
// `size = 0`, `capacity = 0x8000000f`, `buffer[0] = 0`, then reserves the
// literal length.  `FUN_00538810` grows a capacity to
// `max(capacity + capacity / 2, requested)` and clears bit 31 of the
// difference, which [`a89_reserve_capacity_word`] reproduces bit for bit.
//
// The table rows that carry no recoverable computation are listed at the end
// of the file.

// ---------------------------------------------------------------------------
// Recovered layout constants
// ---------------------------------------------------------------------------

/// Capacity word written by every string initialiser in the window: bit 31
/// marks the inline buffer and `0xf` is the usable inline capacity.  Seen at
/// `0x00644080`, `0x006440a6`, `0x006447e0`, `0x00644a50`, `0x00644c8f`,
/// `0x00645165`, `0x0064535d`, `0x00645b2b` and in the capacity checks of the
/// destructor rows.
pub const A89_INLINE_CAPACITY_WORD: u32 = 0x8000_000f;

/// Low 31 bits of [`A89_INLINE_CAPACITY_WORD`]: the largest length that still
/// fits the inline buffer.
pub const A89_SSO_CAPACITY: u32 = 0x0000_000f;

/// Offset of the `data` pointer inside the 28-byte string (`*param_1 =
/// param_1 + 3` at `0x00644080`).
pub const A89_STRING_DATA_OFFSET: u32 = 0x00;

/// Offset of the `size` dword (`param_1[1] = 0` at `0x00644080`).
pub const A89_STRING_SIZE_OFFSET: u32 = 0x04;

/// Offset of the capacity word (`param_1[2] = 0x8000000f` at `0x00644080` and
/// the word tested with `& 0x80000000` by every destructor row).
pub const A89_STRING_CAPACITY_OFFSET: u32 = 0x08;

/// Offset of the inline buffer inside the object: `data` is seeded with
/// `object + 3` dwords, i.e. `+0x0c`.
pub const A89_STRING_DATA_IN_OBJECT: u32 = 0x0c;

/// Total size of one string object: three header dwords plus the 16-byte
/// inline buffer (`0x0c + 0x10`).  This is also the stride between the
/// destructor's capacity checks (`0x08`, `0x24`, `0x40`).
pub const A89_STRING_OBJECT_SIZE: u32 = 0x1c;

/// Stride of one history record: the destructor rows advance their cursor by
/// `0x2c` dwords and test `cursor + 0x11` dwords against the end.
pub const A89_RECORD_STRIDE: u32 = 0x2c * 4;

/// Cursor offset inside a record (`piVar3 = param_1 + 0x1b` at `0x00644630`).
pub const A89_RECORD_CURSOR_OFFSET: u32 = 0x1b * 4;

/// Distance from the cursor to the end of its record (`piVar3 + 0x11`).
pub const A89_RECORD_END_FROM_CURSOR: u32 = 0x11 * 4;

/// Byte offsets of the five capacity words tested by `0x00644630`, i.e. the
/// capacity field of each record string member.  Subtracting
/// [`A89_STRING_CAPACITY_OFFSET`] gives the member offsets
/// `[0x00, 0x1c, 0x38, 0x64, 0x94]`.
pub const A89_RECORD_CAPACITY_OFFSETS: [u32; 5] = [0x08, 0x24, 0x40, 0x6c, 0x9c];

/// Destruction order recovered from `0x00644630`: the last-constructed
/// member (`0x9c`) goes first, the first member (`0x08`) last.
pub const A89_RECORD_DESTRUCTOR_ORDER: [u32; 5] = [0x9c, 0x6c, 0x40, 0x24, 0x08];

/// Header literal built by `0x00644130`; the call passes its length `0x20`.
pub const A89_HISTORY_HEADER: &str = "Session map/replay file history:";

/// Length argument passed with [`A89_HISTORY_HEADER`].
pub const A89_HISTORY_HEADER_LEN: u32 = 0x20;

/// Count line built by `0x00644130`; `{0}` is replaced by the session counter
/// at object `+0x08`.  The call passes the literal's length `0x2d`.
pub const A89_HISTORY_COUNT_FORMAT: &str = "  ({0} map/replay files opened this session).";

/// Length argument passed with [`A89_HISTORY_COUNT_FORMAT`].
pub const A89_HISTORY_COUNT_FORMAT_LEN: u32 = 0x2d;

/// Object offset written by `0x00644110` (`*(iVar1 + 0x50) = param_1`).
pub const A89_TARGET_VALUE_OFFSET: u32 = 0x50;

/// Sticky dirty flag tested and set by `0x00644110` (`+0x51`).
pub const A89_TARGET_DIRTY_OFFSET: u32 = 0x51;

// ---------------------------------------------------------------------------
// 0x00644110 - sticky byte store
// ---------------------------------------------------------------------------

/// Mutable target of the `0x00644110` setter: the byte at `+0x50` and the
/// "has been written" byte at `+0x51`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A89TargetState {
    /// `target[0x50]`.
    pub value: u8,
    /// `target[0x51]`, nonzero once [`a89_sticky_byte_store`] has run.
    pub dirty: bool,
}

/// `0x00644110` (`FUN_00644110`, 25 bytes).
///
/// ```c
/// iVar1 = *in_ECX;
/// *(undefined1 *)(iVar1 + 0x50) = param_1;
/// if (*(char *)(iVar1 + 0x51) == '\0') {
///   *(undefined1 *)(iVar1 + 0x51) = 1;
/// }
/// ```
///
/// The wrapper stores the new byte and latches the dirty flag on the first
/// store.  The conditional is kept rather than replaced by an unconditional
/// `true` because the decompiled body tests the old byte.
pub fn a89_sticky_byte_store(target: &mut A89TargetState, value: u8) {
    target.value = value;
    if !target.dirty {
        target.dirty = true;
    }
}

// ---------------------------------------------------------------------------
// 0x00644130 - session map/replay history report
// ---------------------------------------------------------------------------

/// `0x00644130` (`FUN_00644130`, 345 bytes): the two lines of the session
/// history report.
///
/// The routine always formats
/// [`A89_HISTORY_HEADER`] (length `0x20`) and hands it to the client output
/// helper `FUN_00633bd0` (outside this window).  It then
/// tests the history list head (`*(in_ECX + 4) != in_ECX`, the empty-list
/// sentinel check) and, only for a non-empty list, formats the count line with
/// `0x2d` bytes of literal text and the signed counter at `+0x08` substituted
/// for `{0}`.
///
/// Returns the header and the optional count line.
pub fn a89_history_report(non_empty: bool, count: i32) -> (String, Option<String>) {
    let header = A89_HISTORY_HEADER.to_string();
    let line = if non_empty {
        Some(A89_HISTORY_COUNT_FORMAT.replace("{0}", &count.to_string()))
    } else {
        None
    };
    (header, line)
}

// ---------------------------------------------------------------------------
// Capacity / vector growth policy (FUN_00538810 and 0x00644430)
// ---------------------------------------------------------------------------

/// `0x00644430` / `0x0064452a` / `0x00644591` / `0x00644a50`: the inlined
/// reserve helper `FUN_00538810`.
///
/// ```c
/// uVar1 = obj->capacity;                     // capacity | 0x80000000
/// if ((uVar1 & 0x7fffffff) < requested) {
///   uVar2 = (uVar1 >> 1 & 0x3fffffff) + (uVar1 & 0x7fffffff);
///   if (uVar2 < requested) uVar2 = requested;
///   obj->capacity = (uVar1 ^ uVar2) & 0x7fffffff ^ uVar1;
/// }
/// ```
///
/// The masked-xor store keeps bit 31 exactly as it was and replaces the low
/// 31 bits with the grown capacity.  Only the capacity update is translated;
/// the out-of-memory path (`FUN_0093ec70`) is out of the window.
pub fn a89_reserve_capacity_word(old: u32, requested: u32) -> u32 {
    let low = old & 0x7fff_ffff;
    if low >= requested {
        return old;
    }
    let grown = (old >> 1 & 0x3fff_ffff).wrapping_add(low);
    let target = if grown < requested { requested } else { grown };
    (old ^ target) & 0x7fff_ffff ^ old
}

/// `0x00644430` / `0x0064452a` / `0x00644591`: the vector growth branch taken
/// when `size == capacity`.
///
/// ```c
/// uVar7 = capacity >> 1 & 0x3fffffff;
/// uVar6 = 1;
/// if (capacity + 1 <= uVar7 + capacity) uVar6 = uVar7;
/// new_capacity = capacity + uVar6;
/// ```
///
/// The `capacity + 1 <= ...` test is the overflow guard: for a capacity whose
/// `capacity / 2` addend would wrap, the growth falls back to a single slot.
pub fn a89_vector_growth_capacity(capacity: u32) -> u32 {
    let grow = capacity >> 1 & 0x3fff_ffff;
    let step = if capacity.wrapping_add(1) <= grow.wrapping_add(capacity) {
        grow
    } else {
        1
    };
    capacity.wrapping_add(step)
}

// ---------------------------------------------------------------------------
// 0x00644430 / 0x00644591 - set lookup and range copy
// ---------------------------------------------------------------------------

/// `0x00644430` and `0x00644591`: byte-wise key comparison against the tree
/// node.
///
/// The decompiled body compares whole dwords for equality and then the first
/// differing byte, returning `-1`/`1`; when one key is a prefix of the other
/// it returns the `u32` difference of the lengths, which is consumed as a
/// signed value.  That is exactly `memcmp` followed by a length tie-break.
pub fn a89_compare_bytes(a: &[u8], b: &[u8]) -> i32 {
    let common = a.len().min(b.len());
    for i in 0..common {
        if a[i] != b[i] {
            return if a[i] < b[i] { -1 } else { 1 };
        }
    }
    (a.len() as u32).wrapping_sub(b.len() as u32) as i32
}

/// `0x00644430` (`FUN_00644430`, 250 bytes): lower-bound then equal-range
/// collection.
///
/// The routine walks the tree with the comparator above (left while the node
/// compares greater-or-equal, right otherwise) to find the first node whose key
/// is `>= target`.  It then appends the dword at node `+0x2c` for every node
/// that compares *equal* to the target and stops at the first unequal key.
/// The output vector starts empty; each append grows the capacity with
/// [`a89_vector_growth_capacity`].
///
/// `entries` is the ordered `(key, value)` sequence recovered from the tree.
/// Returns the collected values together with the final capacity word.
pub fn a89_equal_range_values(entries: &[(&[u8], u32)], target: &[u8]) -> (Vec<u32>, u32) {
    let mut values = Vec::new();
    let mut capacity_word = 0u32;
    let mut size = 0u32;

    let mut lo = 0usize;
    let mut hi = entries.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if a89_compare_bytes(entries[mid].0, target) < 0 {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }

    for (key, value) in entries[lo..].iter() {
        if a89_compare_bytes(key, target) != 0 {
            break;
        }
        if size == (capacity_word & 0x7fff_ffff) {
            capacity_word = a89_vector_growth_capacity(capacity_word & 0x7fff_ffff);
        }
        values.push(*value);
        size += 1;
    }
    (values, capacity_word)
}

// ---------------------------------------------------------------------------
// 0x0064452a - dword-buffer copy plan
// ---------------------------------------------------------------------------

/// Result of the copy prologue in `0x0064452a`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A89CopyPlan {
    /// Bytes requested from the allocator (`length * 4`).
    pub alloc_bytes: u32,
    /// Dwords copied, `(length * 4 + 3) >> 2`, or zero when the span wraps.
    pub elements: u32,
    /// The recovered overflow guard `source + length < source` fired.
    pub wrapped: bool,
    /// Allocation alignment passed as the second argument.
    pub alignment: u32,
}

/// `0x0064452a` (`FUN_0064452a`, 103 bytes): the prologue of the range copy.
///
/// ```c
/// uVar3 = alloc(length * 4, 8);
/// uVar4 = length * 4 + 3 >> 2;
/// if (source + length < source) uVar4 = 0;
/// ```
///
/// The guard is evaluated on dword pointers, so it is the byte-span wrap of
/// `source + length * 4`.  The allocation is 4-byte sized and 8-byte aligned.
pub fn a89_bulk_copy_plan(length: u32, source_addr: u32) -> A89CopyPlan {
    let alloc_bytes = length.wrapping_mul(4);
    let wrapped = source_addr.wrapping_add(alloc_bytes) < source_addr;
    let elements = if wrapped {
        0
    } else {
        alloc_bytes.wrapping_add(3) >> 2
    };
    A89CopyPlan {
        alloc_bytes,
        elements,
        wrapped,
        alignment: 8,
    }
}

// ---------------------------------------------------------------------------
// 0x00644591 - tree successor
// ---------------------------------------------------------------------------

/// One node of the recovered red-black tree: `_Parent`, `_Left`, `_Right` in
/// the same dword order the decompiler walks (`node[0]`, `node[1]`,
/// `node[2]`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A89TreeNode {
    /// `node[0]`.
    pub parent: Option<u32>,
    /// `node[1]`.
    pub left: Option<u32>,
    /// `node[2]`.
    pub right: Option<u32>,
}

/// `0x00644591` (`FUN_00644591`, 146 bytes) and the identical successor code
/// inlined in `0x00644430` and `0x0064452a`.
///
/// ```c
/// p = node[2];                                   // right child
/// if (p == 0) {
///   p = node[0];                                 // parent
///   if (node == p[2]) {                          // node is a right child
///     do { node = p; p = node[0]; } while (node == p[2]);
///   }
///   if (node[2] != p) node = p;
/// } else {
///   for (q = p[1]; p = ???, q != 0; q = q[1]) p = q;   // leftmost of right
/// }
/// ```
///
/// This is the MSVC `_Next` successor: the leftmost node of the right subtree,
/// otherwise the first ancestor reached from a left link.  Returns `None` at
/// the last node.
pub fn a89_tree_successor(nodes: &[A89TreeNode], index: u32) -> Option<u32> {
    let node = *nodes.get(index as usize)?;
    if let Some(mut child) = node.right {
        loop {
            match nodes.get(child as usize).and_then(|n| n.left) {
                Some(left) => child = left,
                None => return Some(child),
            }
        }
    }
    let mut current = index;
    let mut parent = node.parent;
    while let Some(p) = parent {
        let ancestor = *nodes.get(p as usize)?;
        if ancestor.right == Some(current) {
            current = p;
            parent = ancestor.parent;
        } else {
            break;
        }
    }
    parent
}

// ---------------------------------------------------------------------------
// 0x00644630 ..= 0x00644700 - five-member record destructor
// ---------------------------------------------------------------------------

/// `0x00644630` (`FUN_00644630`, 85 bytes): offsets of the five string
/// members inside one history record.
///
/// The destructor tests the capacity word of each member for bit 31 (inline)
/// and frees the heap buffer when it is clear.  The member offsets are the
/// tested capacity offsets minus [`A89_STRING_CAPACITY_OFFSET`].
pub fn a89_record_member_offsets() -> [u32; 5] {
    let mut out = [0u32; 5];
    for (slot, capacity) in out.iter_mut().zip(A89_RECORD_CAPACITY_OFFSETS) {
        *slot = capacity - A89_STRING_CAPACITY_OFFSET;
    }
    out
}

/// Destruction order of the five members, as emitted by `0x00644630`:
/// `[0x9c, 0x6c, 0x40, 0x24, 0x08]` capacity words.
pub fn a89_record_destructor_order() -> [u32; 5] {
    A89_RECORD_DESTRUCTOR_ORDER
}

/// Number of member destructors that run for `records` records when the first
/// record enters the loop with `entry_state` members already destroyed.
///
/// Every destructor funclet in `0x00644685..=0x00644700` is the same loop at a
/// different EH entry state: the first record skips the already-destroyed
/// prefix, all later records destroy all five members.
pub fn a89_record_slots_destroyed(records: u32, entry_state: u32) -> u32 {
    if records == 0 {
        return 0;
    }
    let first = 5u32.saturating_sub(entry_state);
    first + 5 * (records - 1)
}

/// EH entry state recorded by `0x00644630`: state 0, all five members still
/// live for the first record.
pub fn a89_record_entry_state_644630() -> u32 {
    0
}

/// EH entry state recorded by `0x00644685`: the `0x9c` member is gone.
pub fn a89_record_entry_state_644685() -> u32 {
    1
}

/// EH entry state recorded by `0x006446a3`: `0x9c` and `0x6c` are gone.
pub fn a89_record_entry_state_6446a3() -> u32 {
    2
}

/// EH entry state recorded by `0x006446c2`: the top three members are gone.
pub fn a89_record_entry_state_6446c2() -> u32 {
    3
}

/// EH entry state recorded by `0x006446e1`: only the `0x08` member is live.
pub fn a89_record_entry_state_6446e1() -> u32 {
    4
}

/// EH entry state recorded by `0x00644700`: the current record is fully
/// destroyed and the loop starts at its successor.
pub fn a89_record_entry_state_644700() -> u32 {
    5
}

// ---------------------------------------------------------------------------
// 0x0064476c - tree insert bookkeeping
// ---------------------------------------------------------------------------

/// Result of the link/size half of `0x0064476c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A89InsertLinks {
    /// Container size after `container[4] += 1`.
    pub size_after: u32,
    /// New leftmost ("head") node.
    pub head: Option<u32>,
    /// New rightmost ("tail") node.
    pub tail: Option<u32>,
}

/// `0x0064476c` (`FUN_0064476c`, 115 bytes): the insert fix-up after the
/// caller located the insertion point.
///
/// ```c
/// new->field_1c = source->field_1c;      // payload copy
/// container[4] = container[4] + 1;       // size
/// new->[1] = 0; new->[2] = 0; new->byte_c = 0;
/// if (tree empty) { container[0] = new; container[1] = new; }
/// else { *link = new; ... container[1] = new; ... }
/// ```
///
/// Only the endpoint/size bookkeeping is translated; the tree rebalance
/// (`FUN_00597290`) and the key search (`FUN_00646d00`) live outside this
/// window.
pub fn a89_tree_insert_step(
    size_before: u32,
    head: Option<u32>,
    tail: Option<u32>,
    new_index: u32,
    inserted_leftmost: bool,
    inserted_rightmost: bool,
    tree_was_empty: bool,
) -> A89InsertLinks {
    if tree_was_empty {
        return A89InsertLinks {
            size_after: size_before.wrapping_add(1),
            head: Some(new_index),
            tail: Some(new_index),
        };
    }
    A89InsertLinks {
        size_after: size_before.wrapping_add(1),
        head: if inserted_leftmost {
            Some(new_index)
        } else {
            head
        },
        tail: if inserted_rightmost {
            Some(new_index)
        } else {
            tail
        },
    }
}

// ---------------------------------------------------------------------------
// 0x006447e0 / 0x00644080 - string seed
// ---------------------------------------------------------------------------

/// Recovered initial image of a string built by the constructor funclets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A89Seed {
    /// Literal length handed to the reserve call (`size` field afterwards).
    pub len: u32,
    /// Length fits the 15-byte inline buffer.
    pub inline: bool,
    /// Capacity word after the reserve: `0x8000000f` inline, otherwise the
    /// value produced by [`a89_reserve_capacity_word`].
    pub capacity_word: u32,
    /// Offset of the buffer inside the object, `Some(0x0c)` only when the
    /// buffer is the inline one.
    pub data_in_object: Option<u32>,
    /// Index of the NUL terminator, relative to the buffer.
    pub terminator: u32,
    /// Last content byte, when the decompiled body stores one explicitly.
    pub last_byte: Option<u8>,
}

/// `0x006447e0` (`FUN_006447e0`, 84 bytes): seed the string from the length
/// stored at `param_1 + 4`.
///
/// ```c
/// uVar1 = *(param_1 + 4);
/// obj[1] = 0; obj[0] = obj + 3; obj[2] = 0x8000000f; obj[3].byte = 0;
/// reserve(uVar1);
/// ```
///
/// `len <= 15` leaves the inline capacity word untouched; a longer literal
/// runs the 1.5x growth from the inline capacity (so a 16-byte literal lands
/// on 22).
pub fn a89_string_seed_from_length(len: u32) -> A89Seed {
    let inline = len <= A89_SSO_CAPACITY;
    let capacity_word = if inline {
        A89_INLINE_CAPACITY_WORD
    } else {
        a89_reserve_capacity_word(A89_INLINE_CAPACITY_WORD, len)
    };
    A89Seed {
        len,
        inline,
        capacity_word,
        data_in_object: inline.then_some(A89_STRING_DATA_IN_OBJECT),
        terminator: len,
        last_byte: None,
    }
}

/// `0x00644080` (`FUN_00644080`, 79 bytes): seed a string from a
/// NUL-terminated literal.
///
/// The visible body only covers the two-byte case: when `literal[1] == 0` the
/// constructor is the empty/one-byte one (`reserve(0)`), otherwise the tail is
/// the `strlen` loop that the decompiler cut at the reserve.  Reconstructing
/// the length from the terminator is the only reading consistent with both
/// branches.
pub fn a89_cstr_seed(literal: &[u8]) -> A89Seed {
    let len = literal
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(literal.len()) as u32;
    a89_string_seed_from_length(len)
}

// ---------------------------------------------------------------------------
// 0x00644a50 - map append step
// ---------------------------------------------------------------------------

/// Result of one `std::map` append as performed by `0x00644a50`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A89MapAppend {
    /// The size word equalled the capacity word, so the node table grew.
    pub grew: bool,
    /// Capacity word after the optional growth.
    pub capacity_word: u32,
    /// Global size after `DAT_00fc9b78 += 1`.
    pub size_after: u32,
    /// Key string seeded from `param + 4`.
    pub key: A89Seed,
    /// Value string reserved from `param + 0x20`.
    pub value: A89Seed,
}

/// `0x00644a50` (`FUN_00644a50`, 152 bytes): append one history entry.
///
/// ```c
/// if (DAT_00fc9b78 == (DAT_00fc9b7c & 0x7fffffff))
///   grow((DAT_00fc9b7c & 0x7fffffff) + 1);
/// seed(param);                       // string length at param + 4
/// DAT_00fc9b78 = DAT_00fc9b78 + 1;
/// reserve(*(param + 0x20));          // second string of the pair
/// ```
///
/// The two lengths are the `size` fields of the 28-byte strings at `param` and
/// `param + 0x1c`, i.e. the key and value of one map entry.
pub fn a89_map_append_step(
    size: u32,
    capacity_word: u32,
    key_len: u32,
    value_len: u32,
) -> A89MapAppend {
    let low = capacity_word & 0x7fff_ffff;
    let grew = size == low;
    let capacity_word = if grew {
        a89_reserve_capacity_word(capacity_word, low.wrapping_add(1))
    } else {
        capacity_word
    };
    A89MapAppend {
        grew,
        capacity_word,
        size_after: size.wrapping_add(1),
        key: a89_string_seed_from_length(key_len),
        value: a89_string_seed_from_length(value_len),
    }
}

// ---------------------------------------------------------------------------
// Empty funclets and literal seeds 0x00644bd9 ..= 0x00645ea0
// ---------------------------------------------------------------------------

/// `0x00644bd9` (`FUN_00644bd9`, 4 bytes): the EH funclet that has nothing
/// left to destroy.
///
/// ```c
/// return;
/// ```
pub fn a89_eh_noop_644bd9() {}

/// `0x00645d56` (`FUN_00645d56`, 4 bytes): identical empty funclet.
///
/// ```c
/// return;
/// ```
pub fn a89_eh_noop_645d56() {}

/// `0x006440a6` and `0x00645200`: seed the empty literal, `reserve(0)`.
pub fn a89_history_literal_0() -> A89Seed {
    a89_string_seed_from_length(0)
}

fn a89_with_last_byte(len: u32, last_byte: Option<u8>) -> A89Seed {
    let mut seed = a89_string_seed_from_length(len);
    seed.last_byte = last_byte;
    seed
}

/// `0x00645330` call with `0xb` and `0x0064535d` (reset then `reserve(0xb)`):
/// the 11-byte literal.
pub fn a89_history_literal_11() -> A89Seed {
    a89_string_seed_from_length(0x0b)
}

/// `0x00644c8f` (134 bytes): the 13-byte literal whose last content byte is
/// stored explicitly as `'e'` (`0x65`) at `buffer + 0xc`, followed by the NUL
/// at `buffer + 0xd`.
pub fn a89_history_literal_13e() -> A89Seed {
    a89_with_last_byte(0x0d, Some(b'e'))
}

/// `0x006450f0`, `0x00645810` (`reserve(0xe)`) and `0x00645165` (stores size
/// `0xe` and the NUL at `buffer + 0xe`): the 14-byte literal.
pub fn a89_history_literal_14() -> A89Seed {
    a89_string_seed_from_length(0x0e)
}

/// `0x00644d30` and `0x00645580` (`reserve(0xf)`): the 15-byte literal, the
/// largest one that still fits the inline buffer.
pub fn a89_history_literal_15() -> A89Seed {
    a89_string_seed_from_length(0x0f)
}

/// `0x00645440` and `0x00645ea0` (`reserve(0x10)`): the first 16-byte
/// literal, which forces the heap path (capacity 22).
pub fn a89_history_literal_16() -> A89Seed {
    a89_string_seed_from_length(0x10)
}

/// `0x00644ff0` (`reserve(0x11)`): the 17-byte literal (capacity 22).
pub fn a89_history_literal_17() -> A89Seed {
    a89_string_seed_from_length(0x11)
}

/// `0x00645980` (`reserve(0x13)`): the 19-byte literal (capacity 22).
pub fn a89_history_literal_19() -> A89Seed {
    a89_string_seed_from_length(0x13)
}

/// `0x00644be0` (`reserve(0x14)`) and the second literal of `0x00645b2b`: the
/// 20-byte literal (capacity 22).
pub fn a89_history_literal_20() -> A89Seed {
    a89_string_seed_from_length(0x14)
}

/// `0x00645c10` (`reserve(0x15)`): the 21-byte literal (capacity 22).
pub fn a89_history_literal_21() -> A89Seed {
    a89_string_seed_from_length(0x15)
}

/// `0x00644ea0` (`reserve(0x16)`): the 22-byte literal, the last one the
/// inline growth step covers.
pub fn a89_history_literal_22() -> A89Seed {
    a89_string_seed_from_length(0x16)
}

/// `0x00645ad0` (`reserve(0x17)`) and the first literal of `0x00645b2b`
/// (size `0x17`, NUL stored at `buffer + 0x17`): the 23-byte literal, the
/// first one whose capacity is the literal length itself.
pub fn a89_history_literal_23() -> A89Seed {
    a89_string_seed_from_length(0x17)
}

/// `0x00645d60` (`reserve(0x19)`): the 25-byte literal.
pub fn a89_history_literal_25() -> A89Seed {
    a89_string_seed_from_length(0x19)
}

// ---------------------------------------------------------------------------
// Rows without recoverable computation
// ---------------------------------------------------------------------------
//
// The following `decompiled_uncited` rows in the window are not translated
// because their bodies contain no arithmetic, table or string behaviour:
//
// * `FUN_00644070` - 9-byte trampoline into the non-returning `FUN_00643920`.
// * `FUN_00644740` - 17-byte bare allocation-failure tail.
// * `FUN_0064475c` - 16-byte argument free followed by a non-returning unwind.
// * `FUN_006452a5` - 14 bytes of misdecoded instructions (`halt_baddata`).
// * `FUN_006452dd`, `FUN_006452ff`, `FUN_00645a83`, `FUN_00645aa5` - virtual call
//   through vtable slot `+0x10` followed by a rethrow; no data operation.
// * `FUN_00645664` - 96 bytes whose only surviving expression is the
//   misdecoded `edi * 0x875ffff` address computation.
// * `FUN_006456e9` - 26 bytes of misdecoded instructions.
// * `FUN_00645a1b` - 14 bytes, partly misdecoded, ending in a throw.

#[cfg(test)]
mod tests_agent89 {
    use super::*;

    /// `0x00644110`: the first store latches the dirty byte, later stores
    /// only update the value.
    #[test]
    fn sticky_byte_store_latches_dirty() {
        let mut target = A89TargetState::default();
        assert_eq!(target.value, 0);
        assert!(!target.dirty);

        a89_sticky_byte_store(&mut target, 0x42);
        assert_eq!(target.value, 0x42);
        assert!(target.dirty);

        a89_sticky_byte_store(&mut target, 0);
        assert_eq!(target.value, 0);
        assert!(target.dirty);
        assert_eq!(A89_TARGET_VALUE_OFFSET, 0x50);
        assert_eq!(A89_TARGET_DIRTY_OFFSET, 0x51);
    }

    /// `0x00644130`: the two recovered literals and the `{0}` substitution.
    #[test]
    fn history_report_uses_the_recovered_literals() {
        let (header, line) = a89_history_report(true, 0);
        assert_eq!(header, "Session map/replay file history:");
        assert_eq!(header.len() as u32, A89_HISTORY_HEADER_LEN);
        assert_eq!(
            line.as_deref(),
            Some("  (0 map/replay files opened this session).")
        );
        // The literal is 45 bytes; `{0}` shrinks to the one-digit count.
        assert_eq!(
            A89_HISTORY_COUNT_FORMAT.len() as u32,
            A89_HISTORY_COUNT_FORMAT_LEN
        );
        assert_eq!(
            line.as_ref().unwrap().len() as u32,
            A89_HISTORY_COUNT_FORMAT_LEN - 2
        );

        let (_, two_digits) = a89_history_report(true, 12);
        let two_digits = two_digits.unwrap();
        assert_eq!(two_digits, "  (12 map/replay files opened this session).");
        assert_eq!(two_digits.len() as u32, A89_HISTORY_COUNT_FORMAT_LEN - 1);

        let (_, negative) = a89_history_report(true, -3);
        assert_eq!(
            negative.unwrap(),
            "  (-3 map/replay files opened this session)."
        );

        assert_eq!(a89_history_report(false, 7).1, None);
    }

    /// `FUN_00538810` as inlined by `0x00644430`: the 1.5x growth and the
    /// masked-xor capacity store.
    #[test]
    fn reserve_capacity_word_grows_like_the_retail_helper() {
        assert_eq!(a89_reserve_capacity_word(0, 5), 5);
        assert_eq!(a89_reserve_capacity_word(0, 0), 0);
        assert_eq!(a89_reserve_capacity_word(5, 5), 5);
        assert_eq!(a89_reserve_capacity_word(5, 4), 5);

        // Inline capacity word: 15 low bits, bit 31 preserved by the store.
        assert_eq!(
            a89_reserve_capacity_word(A89_INLINE_CAPACITY_WORD, 15),
            A89_INLINE_CAPACITY_WORD
        );
        assert_eq!(
            a89_reserve_capacity_word(A89_INLINE_CAPACITY_WORD, 16),
            0x8000_0016
        );
        assert_eq!(
            a89_reserve_capacity_word(A89_INLINE_CAPACITY_WORD, 23),
            0x8000_0017
        );
        // A request past the 1.5x step is used verbatim.
        assert_eq!(
            a89_reserve_capacity_word(A89_INLINE_CAPACITY_WORD, 0x40),
            0x8000_0040
        );
    }

    /// `0x00644430`: growth sequence of an initially empty vector.
    #[test]
    fn vector_growth_matches_the_capacity_sequence() {
        let mut capacity = 0u32;
        for expected in [1u32, 2, 3, 4, 6, 9, 13, 19] {
            capacity = a89_vector_growth_capacity(capacity);
            assert_eq!(capacity, expected);
        }
        // The overflow guard falls back to the masked 1.5x step.
        assert_eq!(a89_vector_growth_capacity(u32::MAX), 0x3fff_fffe);
    }

    /// `0x00644430`/`0x00644591`: byte-wise comparison with the length
    /// tie-break.
    #[test]
    fn compare_bytes_is_memcmp_then_length() {
        assert_eq!(a89_compare_bytes(b"abc", b"abc"), 0);
        assert_eq!(a89_compare_bytes(b"abc", b"abd"), -1);
        assert_eq!(a89_compare_bytes(b"abd", b"abc"), 1);
        assert_eq!(a89_compare_bytes(b"ab", b"abc"), -1);
        assert_eq!(a89_compare_bytes(b"abc", b"ab"), 1);
        assert_eq!(a89_compare_bytes(b"", b""), 0);
        assert_eq!(a89_compare_bytes(b"", b"a"), -1);
        assert_eq!(a89_compare_bytes(b"B", b"a"), -1);
    }

    /// `0x00644430`: lower bound plus equal-range collection, including the
    /// output vector's final capacity.
    #[test]
    fn equal_range_collects_only_the_matching_key() {
        let entries: [(&[u8], u32); 5] = [(b"a", 1), (b"b", 2), (b"b", 3), (b"c", 4), (b"c", 5)];
        assert_eq!(a89_equal_range_values(&entries, b"b").0, vec![2, 3]);
        assert_eq!(a89_equal_range_values(&entries, b"a").0, vec![1]);
        assert_eq!(a89_equal_range_values(&entries, b"c").0, vec![4, 5]);
        assert_eq!(a89_equal_range_values(&entries, b"bb").0, Vec::<u32>::new());
        assert_eq!(a89_equal_range_values(&entries, b"").0, Vec::<u32>::new());
        assert_eq!(a89_equal_range_values(&[], b"a").0, Vec::<u32>::new());

        // One push: capacity 0 grows to 1.  Two duplicates: 1 grows to 2.
        assert_eq!(a89_equal_range_values(&entries, b"a").1, 1);
        assert_eq!(a89_equal_range_values(&entries, b"b").1, 2);
    }

    /// `0x0064452a`: allocation size, 8-byte alignment and the wrap guard.
    #[test]
    fn bulk_copy_plan_reproduces_the_span_guard() {
        let plan = a89_bulk_copy_plan(4, 0x1000);
        assert_eq!(plan.alloc_bytes, 16);
        assert_eq!(plan.elements, 4);
        assert_eq!(plan.alignment, 8);
        assert!(!plan.wrapped);

        // 0x4000_0000 * 4 wraps to zero bytes, so nothing is copied.
        let zero = a89_bulk_copy_plan(0x4000_0000, 0x1000);
        assert_eq!(zero.alloc_bytes, 0);
        assert_eq!(zero.elements, 0);
        assert!(!zero.wrapped);

        // A source near the top of the address space wraps the span.
        let wrapped = a89_bulk_copy_plan(0xffff_ffff, 0x10);
        assert_eq!(wrapped.alloc_bytes, 0xffff_fffc);
        assert!(wrapped.wrapped);
        assert_eq!(wrapped.elements, 0);
    }

    /// `0x00644591`: MSVC `_Next` over the recovered parent/left/right
    /// fields.
    #[test]
    fn tree_successor_walks_the_in_order_chain() {
        //        0 (root)
        //       /  \
        //      1    2
        //            \
        //             3
        let nodes = [
            A89TreeNode {
                parent: None,
                left: Some(1),
                right: Some(2),
            },
            A89TreeNode {
                parent: Some(0),
                left: None,
                right: None,
            },
            A89TreeNode {
                parent: Some(0),
                left: None,
                right: Some(3),
            },
            A89TreeNode {
                parent: Some(2),
                left: None,
                right: None,
            },
        ];
        assert_eq!(a89_tree_successor(&nodes, 1), Some(0));
        assert_eq!(a89_tree_successor(&nodes, 0), Some(2));
        assert_eq!(a89_tree_successor(&nodes, 2), Some(3));
        assert_eq!(a89_tree_successor(&nodes, 3), None);

        // Right subtree: descend to its leftmost node.
        let right = [
            A89TreeNode {
                parent: None,
                left: Some(1),
                right: Some(2),
            },
            A89TreeNode {
                parent: Some(0),
                left: None,
                right: None,
            },
            A89TreeNode {
                parent: Some(0),
                left: Some(3),
                right: None,
            },
            A89TreeNode {
                parent: Some(2),
                left: None,
                right: None,
            },
        ];
        // In order: 1, 0, 3, 2.
        assert_eq!(a89_tree_successor(&right, 0), Some(3));
        assert_eq!(a89_tree_successor(&right, 1), Some(0));
        assert_eq!(a89_tree_successor(&right, 3), Some(2));
        assert_eq!(a89_tree_successor(&right, 2), None);
        assert_eq!(a89_tree_successor(&right, 9), None);
    }

    /// `0x00644630`: record stride, member offsets and destruction order.
    #[test]
    fn record_destructor_layout_is_recovered() {
        assert_eq!(A89_RECORD_STRIDE, 0xb0);
        assert_eq!(A89_RECORD_CURSOR_OFFSET, 0x6c);
        assert_eq!(A89_RECORD_END_FROM_CURSOR, 0x44);
        assert_eq!(
            A89_RECORD_CURSOR_OFFSET + A89_RECORD_END_FROM_CURSOR,
            A89_RECORD_STRIDE
        );
        assert_eq!(A89_RECORD_CAPACITY_OFFSETS, [0x08, 0x24, 0x40, 0x6c, 0x9c]);
        assert_eq!(a89_record_member_offsets(), [0x00, 0x1c, 0x38, 0x64, 0x94]);
        assert_eq!(A89_RECORD_DESTRUCTOR_ORDER, [0x9c, 0x6c, 0x40, 0x24, 0x08]);
        assert_eq!(a89_record_destructor_order(), A89_RECORD_DESTRUCTOR_ORDER);

        // The first member plus four strides stays inside the record.
        assert!(a89_record_member_offsets()[4] + A89_STRING_OBJECT_SIZE <= A89_RECORD_STRIDE);
    }

    /// `0x00644685..=0x00644700`: the same loop at five EH entry states.
    #[test]
    fn record_destructor_entry_states_are_ordered() {
        assert_eq!(a89_record_entry_state_644630(), 0);
        assert_eq!(a89_record_entry_state_644685(), 1);
        assert_eq!(a89_record_entry_state_6446a3(), 2);
        assert_eq!(a89_record_entry_state_6446c2(), 3);
        assert_eq!(a89_record_entry_state_6446e1(), 4);
        assert_eq!(a89_record_entry_state_644700(), 5);

        // 3 records, state 2: 3 slots for the first + 5 + 5.
        assert_eq!(
            a89_record_slots_destroyed(3, a89_record_entry_state_6446a3()),
            13
        );
        assert_eq!(
            a89_record_slots_destroyed(3, a89_record_entry_state_644630()),
            15
        );
        assert_eq!(
            a89_record_slots_destroyed(1, a89_record_entry_state_644700()),
            0
        );
        assert_eq!(a89_record_slots_destroyed(0, 0), 0);
        // A state past the last member cannot destroy a negative count.
        assert_eq!(a89_record_slots_destroyed(1, 99), 0);
        assert_eq!(a89_record_slots_destroyed(2, 99), 5);
    }

    /// `0x0064476c`: size increment and the head/tail endpoint updates.
    #[test]
    fn tree_insert_step_updates_size_and_endpoints() {
        let empty = a89_tree_insert_step(0, None, None, 7, true, true, true);
        assert_eq!(
            empty,
            A89InsertLinks {
                size_after: 1,
                head: Some(7),
                tail: Some(7)
            }
        );

        let left = a89_tree_insert_step(3, Some(2), Some(9), 1, true, false, false);
        assert_eq!(
            left,
            A89InsertLinks {
                size_after: 4,
                head: Some(1),
                tail: Some(9)
            }
        );

        let right = a89_tree_insert_step(3, Some(2), Some(9), 11, false, true, false);
        assert_eq!(
            right,
            A89InsertLinks {
                size_after: 4,
                head: Some(2),
                tail: Some(11)
            }
        );

        let middle = a89_tree_insert_step(3, Some(2), Some(9), 5, false, false, false);
        assert_eq!(
            middle,
            A89InsertLinks {
                size_after: 4,
                head: Some(2),
                tail: Some(9)
            }
        );
    }

    /// `0x006447e0`: the seed image and the inline/heap boundary at 15 bytes.
    #[test]
    fn string_seed_switches_to_the_heap_after_15_bytes() {
        let empty = a89_string_seed_from_length(0);
        assert!(empty.inline);
        assert_eq!(empty.capacity_word, A89_INLINE_CAPACITY_WORD);
        assert_eq!(empty.data_in_object, Some(0x0c));
        assert_eq!(empty.terminator, 0);

        let fifteen = a89_string_seed_from_length(15);
        assert!(fifteen.inline);
        assert_eq!(fifteen.capacity_word, A89_INLINE_CAPACITY_WORD);
        assert_eq!(fifteen.terminator, 15);

        let sixteen = a89_string_seed_from_length(16);
        assert!(!sixteen.inline);
        assert_eq!(sixteen.capacity_word, 0x8000_0016);
        assert_eq!(sixteen.capacity_word & 0x7fff_ffff, 22);
        assert_eq!(sixteen.data_in_object, None);
        assert_eq!(sixteen.terminator, 16);

        for (len, capacity) in [
            (17u32, 22u32),
            (19, 22),
            (20, 22),
            (21, 22),
            (22, 22),
            (23, 23),
            (25, 25),
        ] {
            let seed = a89_string_seed_from_length(len);
            assert!(!seed.inline, "len {len} must use the heap");
            assert_eq!(seed.capacity_word & 0x7fff_ffff, capacity, "len {len}");
        }
    }

    /// `0x00644080`: length recovered from the NUL terminator.
    #[test]
    fn cstr_seed_measures_to_the_terminator() {
        assert_eq!(a89_cstr_seed(b"").len, 0);
        assert_eq!(a89_cstr_seed(b"\0ignored").len, 0);
        assert_eq!(a89_cstr_seed(b"a\0").len, 1);
        assert_eq!(a89_cstr_seed(b"a\0b\0").len, 1);
        assert_eq!(a89_cstr_seed(b"abc\0").len, 3);
        // A missing terminator uses the whole slice.
        let full = a89_cstr_seed(b"0123456789abcdef");
        assert_eq!(full.len, 16);
        assert!(!full.inline);
        assert_eq!(full.capacity_word & 0x7fff_ffff, 22);
    }

    /// `0x00644a50`: the global map grows only when size == capacity and the
    /// two strings of the entry are seeded independently.
    #[test]
    fn map_append_grows_at_capacity_and_seeds_both_strings() {
        let room = a89_map_append_step(0, A89_INLINE_CAPACITY_WORD, 5, 20);
        assert!(!room.grew);
        assert_eq!(room.size_after, 1);
        assert_eq!(room.capacity_word, A89_INLINE_CAPACITY_WORD);
        assert_eq!(room.key.len, 5);
        assert!(room.key.inline);
        assert_eq!(room.value.len, 20);
        assert!(!room.value.inline);

        let full = a89_map_append_step(15, A89_INLINE_CAPACITY_WORD, 3, 3);
        assert!(full.grew);
        assert_eq!(full.size_after, 16);
        assert_eq!(full.capacity_word, 0x8000_0016);

        // A 31-bit-capacity word is grown to max(1.5x, low + 1).
        let word = a89_map_append_step(4, 4, 0, 0);
        assert!(word.grew);
        assert_eq!(word.capacity_word, 6);
    }

    /// `0x006440a6` ..= `0x00645d60`: every literal length recovered from the
    /// reserve argument (and the explicitly stored bytes).
    #[test]
    fn literal_seeds_match_the_reserve_arguments() {
        assert_eq!(a89_history_literal_0().len, 0);
        assert_eq!(a89_history_literal_11().len, 0x0b);
        assert_eq!(
            a89_history_literal_11().capacity_word,
            A89_INLINE_CAPACITY_WORD
        );

        let thirteen = a89_history_literal_13e();
        assert_eq!(thirteen.len, 0x0d);
        assert_eq!(thirteen.last_byte, Some(b'e'));
        assert_eq!(thirteen.terminator, 0x0d);
        assert!(thirteen.inline);

        assert_eq!(a89_history_literal_14().len, 0x0e);
        assert!(a89_history_literal_14().inline);
        assert_eq!(a89_history_literal_15().len, 0x0f);
        assert!(a89_history_literal_15().inline);

        assert_eq!(a89_history_literal_16().len, 0x10);
        assert!(!a89_history_literal_16().inline);
        assert_eq!(a89_history_literal_17().len, 0x11);
        assert_eq!(a89_history_literal_19().len, 0x13);
        assert_eq!(a89_history_literal_20().len, 0x14);
        assert_eq!(a89_history_literal_21().len, 0x15);
        assert_eq!(a89_history_literal_22().len, 0x16);

        let twenty_three = a89_history_literal_23();
        assert_eq!(twenty_three.len, 0x17);
        assert!(!twenty_three.inline);
        assert_eq!(twenty_three.capacity_word & 0x7fff_ffff, 0x17);

        assert_eq!(a89_history_literal_25().len, 0x19);
        assert_eq!(a89_history_literal_25().capacity_word & 0x7fff_ffff, 0x19);
    }

    /// `0x00644bd9`/`0x00645d56`: the empty funclets have no side effects.
    #[test]
    fn empty_funclets_are_noops() {
        a89_eh_noop_644bd9();
        a89_eh_noop_645d56();
    }

    /// The recovered string object layout is self-consistent: the inline
    /// buffer starts after the three header dwords and the object size covers
    /// it plus the 16-byte buffer.
    #[test]
    fn string_object_layout_is_self_consistent() {
        assert_eq!(A89_STRING_DATA_OFFSET, 0);
        assert_eq!(A89_STRING_SIZE_OFFSET, 4);
        assert_eq!(A89_STRING_CAPACITY_OFFSET, 8);
        assert_eq!(A89_STRING_DATA_IN_OBJECT, A89_STRING_CAPACITY_OFFSET + 4);
        assert_eq!(A89_STRING_OBJECT_SIZE, A89_STRING_DATA_IN_OBJECT + 0x10);
        assert_eq!(A89_SSO_CAPACITY, A89_INLINE_CAPACITY_WORD & 0x7fff_ffff);
    }
}
