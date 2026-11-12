#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent76.rs
// ===========================================================================
// Translation batch 76.  Only this file is edited by the batch.
//
// Batch 76 owns `0x0062a000..=0x0062bfff`.  The ledger lists 52
// `decompiled_uncited` rows in that window; 27 are translated below and the
// 25 rows that are not are listed at the end with their reason.
//
// The whole window is one C++ template family over a **0x68-byte record**.
// The record shape is recovered from the two constructors and every copy
// loop in the window:
//
// | offset            | meaning                                                |
// |-------------------|--------------------------------------------------------|
// | `+0x00`           | `u32` key; rank lookup is by this dword                |
// | `+0x04..=+0x1f`   | `std::string`-shaped header: data at `+0x04`, size at `+0x08`, capacity/flags at `+0x0c`, inline bytes at `+0x10..=+0x1f` |
// | `+0x20..=+0x4f`   | copied payload; dwords `+0x20..=+0x33`, bytes `+0x34..=+0x36`, dwords `+0x38`, `+0x3c`, 16 bytes at `+0x40..=+0x4f` (byte `+0x37` is a hole the shipped copy never writes) |
// | `+0x50`           | 20-byte-element vector header: data, count at `+0x54`, capacity/flags at `+0x58` |
// | `+0x5c`           | 16-byte-element vector header: data, count at `+0x60`, capacity/flags at `+0x64` |
//
// The capacity flags use bit 31 as the "inline / no deallocation" mark:
// `0x8000_000f` is the inline string, `0x8000_0000` is the empty inline
// vector, and a non-negative word means a heap allocation of that many
// elements.  The copied bodies and the two move helpers (`FUN_0062d670` for
// the 20-byte vector, `FUN_0062d5b0` for the 16-byte vector) use exactly this
// test.
//
// Comparison does not read the record's own bytes.  The key dword indexes a
// global table of `0x24`-byte entries and the compared byte is entry `+0x0a`:
// `0x0062bd60` computes `rank(a) < rank(b)`.  `FUN_00748d40` remaps keys
// `0x80..=0x83` to indices `0x0c..=0x0f`; [`a76_key_index`] reproduces that
// remap and [`a76_rank_of`] takes the rank column as a supplied slice.
//
// Address to function:
//
// * `0x0062a543` [`a76_swap_if_not_less`]
// * `0x0062a8d0` [`a76_insertion_sort`]
// * `0x0062aa39` [`a76_insertion_shift_head`]
// * `0x0062aa79` [`a76_shift_tail`]
// * `0x0062aae5` [`a76_move_member50`]
// * `0x0062ab21` [`a76_move_member5c`]
// * `0x0062ab6e` [`a76_finish_insert`]
// * `0x0062ac82` [`a76_insertion_next`]
// * `0x0062acd0` [`a76_push_heap`]
// * `0x0062ad71` [`a76_move_head`]
// * `0x0062ae70` [`a76_adjust_heap`]
// * `0x0062af2e` [`a76_adjust_copy_head`]
// * `0x0062b040` [`a76_make_heap`]
// * `0x0062b2f0` [`a76_sort_heap`]
// * `0x0062b327` [`a76_sort_heap_offsets`]
// * `0x0062b7c0` [`a76_new_node_44`]
// * `0x0062b820` [`a76_locale_init`]
// * `0x0062b8f0` [`a76_move_construct`]
// * `0x0062b9e0` [`a76_copy_construct`]
// * `0x0062bbe0` [`a76_destroy`]
// * `0x0062bc80` [`a76_move_assign`]
// * `0x0062bd60` [`a76_record_less`]
// * `0x0062bd70` [`a76_rank_table_base`]
// * `0x0062bde0` [`a76_name_storage`]
// * `0x0062be30` [`a76_should_refresh`]
// * `0x0062be5c` [`a76_find_append16`]
// * `0x0062bfb4` [`a76_finish_append16`]
//
// The cached `.c` files are often truncated mid-routine (the ledger sizes
// are not instruction boundaries).  Where the slice is a prologue or a loop
// tail of a larger routine, the doc comment cites the cached bytes and the
// translation reproduces the whole recovered routine, verified against the
// instruction stream in `~/.cache/galaxycraft/sc32_runtime_pe.bin`.

// ---------------------------------------------------------------------------
// Recovered layout
// ---------------------------------------------------------------------------

/// Size of one element of the vectors handled in this window: `0x68` bytes
/// (`imul ...,0x68` at `0x0062a8d0`, `0x0062be5c`, `0x0062b040`).
pub const A76_RECORD_BYTES: usize = 0x68;

/// Key dword at `+0x00`.
pub const A76_KEY_OFFSET: usize = 0x00;

/// String header at `+0x04` (data `+0x04`, size `+0x08`, capacity `+0x0c`,
/// inline bytes `+0x10..=+0x1f`).
pub const A76_NAME_OFFSET: usize = 0x04;

/// Payload starts at `+0x20`.
pub const A76_TAIL_OFFSET: usize = 0x20;

/// 20-byte-element vector at `+0x50`.
pub const A76_MEMBER50_OFFSET: usize = 0x50;

/// 16-byte-element vector at `+0x5c`.
pub const A76_MEMBER5C_OFFSET: usize = 0x5c;

/// Inline `std::string` capacity: `0x8000000f` is the empty header written by
/// `0x0062b9e0` and by the empty-string constructor.
pub const A76_STRING_INLINE_FLAGS: u32 = 0x8000_000f;

/// Empty inline vector header (`data = 0`, `count = 0`, bit 31 set).
pub const A76_VECTOR_INLINE_FLAGS: u32 = 0x8000_0000;

/// Global rank table stride: `0x24` bytes per entry
/// (`lea eax,[esi+esi*8]; shl eax,2` at `0x0062ad20`).
pub const A76_RANK_STRIDE: usize = 0x24;

/// Rank byte inside one table entry (`mov al,[eax+0xa]`).
pub const A76_RANK_OFFSET: usize = 0x0a;

/// Smallest key remapped by `FUN_00748d40` before the table lookup.
pub const A76_KEY_REMAP_FIRST: u32 = 0x80;

/// Largest key remapped by `FUN_00748d40`.
pub const A76_KEY_REMAP_LAST: u32 = 0x83;

/// Target index of the first remapped key (`0x80 -> 0x0c`).
pub const A76_KEY_REMAP_TARGET: u32 = 0x0c;

/// Storage decision of the two name-handling rows.
///
/// The `FUN_0093ec70` call in these bodies is the CRT allocator accessor, not
/// an error path: the slow branch performs `operator new(len, 8)` and sets
/// the ownership bit.  The model reports which branch the shipped code takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A76NameStorage {
    /// The short-name branch uses the existing inline storage and returns
    /// `this` unchanged.
    Inline,
    /// The slow branch requests `new(bytes, align)`.
    Heap { bytes: u32, align: u32 },
}

/// One `std::string`-shaped member: the sequence of bytes plus the shipped
/// capacity/flags word.  The data pointer and inline buffer locations are not
/// reproduced; only the bytes and the flags are observable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A76String {
    /// Bytes at the data pointer (`+0x04`) with the recovered size (`+0x08`).
    pub bytes: Vec<u8>,
    /// Capacity word at `+0x0c`: low 31 bits capacity, bit 31 = inline.
    pub cap_flags: u32,
}

impl A76String {
    /// The empty inline string: no bytes and the `0x8000000f` header.
    pub fn a76_empty() -> Self {
        A76String {
            bytes: Vec::new(),
            cap_flags: A76_STRING_INLINE_FLAGS,
        }
    }

    /// True when the header still uses the inline buffer (`bit 31`).
    pub fn a76_is_inline(&self) -> bool {
        self.cap_flags & 0x8000_0000 != 0
    }

    /// Low 31 bits of the capacity word.
    pub fn a76_capacity(&self) -> u32 {
        self.cap_flags & 0x7fff_ffff
    }
}

impl Default for A76String {
    fn default() -> Self {
        Self::a76_empty()
    }
}

/// The 20-byte-element vector at record `+0x50`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A76Vector20 {
    /// Elements, five dwords each (the copy loop moves `0x14` bytes).
    pub items: Vec<[u32; 5]>,
    /// `+0x58`: low 31 bits capacity, bit 31 = inline / not allocated.
    pub cap_flags: u32,
}

impl A76Vector20 {
    /// The empty inline header.
    pub fn a76_empty() -> Self {
        A76Vector20 {
            items: Vec::new(),
            cap_flags: A76_VECTOR_INLINE_FLAGS,
        }
    }

    /// `+0x58 & 0x7fffffff`.
    pub fn a76_capacity(&self) -> u32 {
        self.cap_flags & 0x7fff_ffff
    }

    /// True while the header is the inline/empty form.
    pub fn a76_is_inline(&self) -> bool {
        self.cap_flags & 0x8000_0000 != 0
    }
}

impl Default for A76Vector20 {
    fn default() -> Self {
        Self::a76_empty()
    }
}

/// The 16-byte-element vector at record `+0x5c`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A76Vector16 {
    /// Elements, four dwords each (`movups` copies `0x10` bytes).
    pub items: Vec<[u32; 4]>,
    /// `+0x64`: low 31 bits capacity, bit 31 = inline / not allocated.
    pub cap_flags: u32,
}

impl A76Vector16 {
    /// The empty inline header.
    pub fn a76_empty() -> Self {
        A76Vector16 {
            items: Vec::new(),
            cap_flags: A76_VECTOR_INLINE_FLAGS,
        }
    }

    /// `+0x64 & 0x7fffffff`.
    pub fn a76_capacity(&self) -> u32 {
        self.cap_flags & 0x7fff_ffff
    }

    /// True while the header is the inline/empty form.
    pub fn a76_is_inline(&self) -> bool {
        self.cap_flags & 0x8000_0000 != 0
    }
}

impl Default for A76Vector16 {
    fn default() -> Self {
        Self::a76_empty()
    }
}

/// The copied payload between `+0x20` and `+0x4f`.
///
/// The shipped copy constructor writes the five dwords at `+0x20`, the three
/// bytes at `+0x34`, the dwords at `+0x38`/`+0x3c`, and the 16 bytes at
/// `+0x40`.  Byte `+0x37` is a hole and is not part of the model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A76Tail {
    /// `+0x20..=+0x33`.
    pub w20: [u32; 5],
    /// `+0x34..=+0x36`.
    pub b34: [u8; 3],
    /// `+0x38`.
    pub w38: u32,
    /// `+0x3c`.
    pub w3c: u32,
    /// `+0x40..=+0x4f`.
    pub w40: [u32; 4],
}

/// One `0x68`-byte record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A76Record {
    /// `+0x00`.
    pub key: u32,
    /// `+0x04..=+0x1f`.
    pub name: A76String,
    /// `+0x20..=+0x4f`.
    pub tail: A76Tail,
    /// `+0x50`.
    pub member50: A76Vector20,
    /// `+0x5c`.
    pub member5c: A76Vector16,
}

impl A76Record {
    /// The empty record with the given key: empty inline string and both
    /// vectors inline.
    pub fn a76_new(key: u32) -> Self {
        A76Record {
            key,
            name: A76String::a76_empty(),
            tail: A76Tail::default(),
            member50: A76Vector20::a76_empty(),
            member5c: A76Vector16::a76_empty(),
        }
    }

    /// The rank byte of this record's key.
    pub fn a76_rank(&self, ranks: &[u8]) -> u8 {
        a76_rank_of(self.key, ranks)
    }
}

impl Default for A76Record {
    fn default() -> Self {
        Self::a76_new(0)
    }
}

/// The key-index remap performed by `FUN_00748d40` before the table stride is
/// applied.
///
/// `FUN_00748d40` returns `1` exactly for keys `0x80..=0x83`
/// (`x - 0x80 <= 3` unsigned); the comparator then replaces such a key with
/// `(key & 0x7f) + 0x0c`.  Every other key is its own index.
pub fn a76_key_index(key: u32) -> u32 {
    if key.wrapping_sub(A76_KEY_REMAP_FIRST) <= A76_KEY_REMAP_LAST - A76_KEY_REMAP_FIRST {
        (key & 0x7f).wrapping_add(A76_KEY_REMAP_TARGET)
    } else {
        key
    }
}

/// Read the compared rank byte for `key`.
///
/// `ranks` is the `+0x0a` byte of each `0x24`-byte table entry, indexed by
/// [`a76_key_index`]; entries not covered by the supplied table panic, which
/// keeps test tables honest.
pub fn a76_rank_of(key: u32, ranks: &[u8]) -> u8 {
    ranks[a76_key_index(key) as usize]
}

/// `0x0062bd60`: `return rank(a) < rank(b);` (`setb al` after the two rank
/// lookups).
///
/// The cached 16-byte body stops after the first lookup and shows
/// `FUN_00748d40(*param_1)`; the complete routine is
/// `0x0062bd60..=FUN_0062bddd`.  Both lookups are folded into
/// [`a76_rank_of`].
pub fn a76_record_less(a: &A76Record, b: &A76Record, ranks: &[u8]) -> bool {
    a.rank_byte(ranks) < b.rank_byte(ranks)
}

impl A76Record {
    fn rank_byte(&self, ranks: &[u8]) -> u8 {
        a76_rank_of(self.key, ranks)
    }
}

/// `0x0062bd70`: the obfuscated rank-table base.
///
/// The cached body is the middle of `0x0062bd60`; its arithmetic is
///
/// ```text
/// base = (0xfb5969c2 - DAT_00f7f0e4) ^ DAT_011a0e38
/// ```
///
/// with the two globals supplied by the caller.  Both operations are wrapping
/// 32-bit operations.
pub fn a76_rank_table_base(g_00f7f0e4: u32, g_011a0e38: u32) -> u32 {
    0xfb59_69c2u32.wrapping_sub(g_00f7f0e4) ^ g_011a0e38
}

// ---------------------------------------------------------------------------
// String and vector move helpers used by the record constructors
// ---------------------------------------------------------------------------

/// Move-assign one [`A76String`] as `FUN_0053ae90` does.
///
/// * Heap source (bit 31 clear): steal the bytes and copy the low capacity
///   bits; the source becomes the empty inline header.
/// * Inline source: copy the bytes and leave the source untouched.  The
///   shipped inline branch of `0x53ae90` does not write the destination
///   capacity word, so the destination header is preserved.
pub fn a76_string_move_assign(dst: &mut A76String, src: &mut A76String) {
    if src.cap_flags & 0x8000_0000 == 0 {
        dst.bytes = std::mem::take(&mut src.bytes);
        dst.cap_flags = src.cap_flags & 0x7fff_ffff;
        src.cap_flags = A76_STRING_INLINE_FLAGS;
    } else {
        dst.bytes = src.bytes.clone();
    }
}

/// Move-assign one [`A76Vector20`] as `FUN_0062d670` does.
///
/// Heap source: steal the elements and low capacity bits, leaving the source
/// as the empty inline header.  Inline source: copy the elements; the
/// destination keeps its old capacity only when that capacity is large enough
/// *and* still marked inline, otherwise the new capacity is the element
/// count (the `count >= cap || cap >= 0` test at `0x0062d6dd`).
pub fn a76_vector20_move_assign(dst: &mut A76Vector20, src: &mut A76Vector20) {
    if src.cap_flags & 0x8000_0000 == 0 {
        dst.items = std::mem::take(&mut src.items);
        dst.cap_flags = src.cap_flags & 0x7fff_ffff;
        src.cap_flags = A76_VECTOR_INLINE_FLAGS;
        return;
    }
    let count = src.items.len() as u32;
    let dst_cap = dst.cap_flags & 0x7fff_ffff;
    let keep_capacity = count < dst_cap && (dst.cap_flags as i32) < 0;
    if !keep_capacity {
        dst.cap_flags = count;
    }
    dst.items = src.items.clone();
}

/// Move assignment for the 16-byte vector (`+0x5c`), the counterpart of the
/// `+0x50` helper above.
pub fn a76_vector16_move_assign(dst: &mut A76Vector16, src: &mut A76Vector16) {
    if src.cap_flags & 0x8000_0000 == 0 {
        dst.items = std::mem::take(&mut src.items);
        dst.cap_flags = src.cap_flags & 0x7fff_ffff;
        src.cap_flags = A76_VECTOR_INLINE_FLAGS;
        return;
    }
    let count = src.items.len() as u32;
    let dst_cap = dst.cap_flags & 0x7fff_ffff;
    let keep_capacity = count < dst_cap && (dst.cap_flags as i32) < 0;
    if !keep_capacity {
        dst.cap_flags = count;
    }
    dst.items = src.items.clone();
}

// ---------------------------------------------------------------------------
// Record constructors, destructor and move assignment
// ---------------------------------------------------------------------------

/// `0x0062b8f0`: move-construct a record.
///
/// The cached 3-byte body is only `push ebp; mov ebp, esp`; the body is the
/// same routine that continues at `FUN_0062b8f3` (the ledger lists that address
/// as a separate row) and runs to `0x0062b9d4`:
///
/// ```text
/// dst->key = src->key;
/// FUN_0053a5b0(dst + 4, src + 4);       // string move constructor
/// copy payload +0x20..+0x3c;
/// copy 16 bytes at +0x40;
/// FUN_0062d670(dst + 0x50, src + 0x50); // vector20 move
/// FUN_0062d5b0(dst + 0x5c, src + 0x5c); // vector16 move
/// ```
///
/// The shipped string move constructor is `0x53a5b0`; it is the same
/// heap-steal / inline-copy split modelled by [`a76_string_move_assign`].
pub fn a76_move_construct(src: &mut A76Record) -> A76Record {
    let mut dst = A76Record::a76_new(src.key);
    a76_string_move_assign(&mut dst.name, &mut src.name);
    dst.tail = src.tail;
    a76_vector20_move_assign(&mut dst.member50, &mut src.member50);
    a76_vector16_move_assign(&mut dst.member5c, &mut src.member5c);
    dst
}

/// `0x0062b9e0`: copy-construct a record by deep-copying both vectors and the
/// string bytes.
///
/// The cached 94-byte body stops in the middle of the string setup; the full
/// routine runs to `0x0062bddf`.  The source is not modified.  The destination
/// string header is the empty inline `0x8000000f` after the copy, and both
/// vector headers are `count` with bit 31 clear (the shipped
/// `(0 ^ count) & 0x7fffffff ^ 0` sequence).
pub fn a76_copy_construct(src: &A76Record) -> A76Record {
    A76Record {
        key: src.key,
        name: A76String {
            bytes: src.name.bytes.clone(),
            cap_flags: A76_STRING_INLINE_FLAGS,
        },
        tail: src.tail,
        member50: A76Vector20 {
            items: src.member50.items.clone(),
            cap_flags: src.member50.items.len() as u32,
        },
        member5c: A76Vector16 {
            items: src.member5c.items.clone(),
            cap_flags: src.member5c.items.len() as u32,
        },
    }
}

/// `0x0062bbe0`: destroy the three heap-capable members.
///
/// The cached 84-byte body stops in the third release; the complete routine
/// runs to `0x0062bc7f`.  The decompiled body shows three
/// `if (-1 < *(int *)(this + N))` tests; the sign check is the capacity
/// word's inline bit (`N = 0x64`, `0x58`, `0x0c`).
/// A non-negative word frees the data pointer and clears it.  In the value
/// model the whole member is reset to its empty inline form.
pub fn a76_destroy(record: &mut A76Record) {
    if (record.member5c.cap_flags as i32) >= 0 {
        record.member5c = A76Vector16::a76_empty();
    }
    if (record.member50.cap_flags as i32) >= 0 {
        record.member50 = A76Vector20::a76_empty();
    }
    if (record.name.cap_flags as i32) >= 0 {
        record.name = A76String::a76_empty();
    }
}

/// `0x0062bc80`: move-assign a record into an already constructed one.
///
/// The cached 28-byte body shows only `*in_ECX = *param_1` and the string
/// tail; the complete routine is `0x0062bc80..=0x0062bd57`.  After the string
/// and payload copy the shipped code frees a heap-backed destination member
/// when its capacity word is non-negative (`+0x58 >= 0` / `+0x64 >= 0`) and
/// then move-assigns the source member.  The moved-from source vectors become
/// empty inline.
pub fn a76_move_assign(dst: &mut A76Record, src: &mut A76Record) {
    dst.key = src.key;
    a76_string_move_assign(&mut dst.name, &mut src.name);
    dst.tail = src.tail;
    if (dst.member50.cap_flags as i32) >= 0 {
        dst.member50 = A76Vector20::a76_empty();
    }
    a76_vector20_move_assign(&mut dst.member50, &mut src.member50);
    if (dst.member5c.cap_flags as i32) >= 0 {
        dst.member5c = A76Vector16::a76_empty();
    }
    a76_vector16_move_assign(&mut dst.member5c, &mut src.member5c);
}

// ---------------------------------------------------------------------------
// Sort fragments
// ---------------------------------------------------------------------------

/// `0x0062a543`: the compare-and-swap slice of the partition routine.
///
/// The cached 117-byte body is
///
/// ```text
/// if (rank(left) < rank(right)) goto done;   // jb 0x62a592
/// temp = *right; *right = *left; ... *left = temp;
/// done: FUN_00748d40(...);                   // slice tail
/// ```
///
/// The three record assignments are a swap through the frame temporary.  The
/// function returns `true` when it swapped.
pub fn a76_swap_if_not_less(left: &mut A76Record, right: &mut A76Record, ranks: &[u8]) -> bool {
    if left.rank_byte(ranks) < right.rank_byte(ranks) {
        return false;
    }
    std::mem::swap(left, right);
    true
}

/// `0x0062a8d0`: the insertion-sort routine.
///
/// The cached 94-byte body stops after the first temporary copy (the
/// decompiler reports the `param_1 + 0x68 != param_2` guard and the string
/// copy into the frame local).  The complete routine is
/// `0x0062a8d0..=0x0062acc5` and sorts **descending** by the compared rank:
/// it shifts the previous record down while
/// `rank(previous) < rank(saved)` (`0x62aa58`: `cmp prev_rank, saved_rank;
/// jae done`), which places the larger saved element first.
pub fn a76_insertion_sort(records: &mut [A76Record], ranks: &[u8]) {
    for i in 1..records.len() {
        let mut j = i;
        while j > 0 && records[j - 1].rank_byte(ranks) < records[j].rank_byte(ranks) {
            records.swap(j - 1, j);
            j -= 1;
        }
    }
}

/// `0x0062aa39`: the comparison half of the insertion-sort shift.
///
/// The cached 64-byte body compares the previous record's rank with the saved
/// record's rank and exits when `prev_rank >= saved_rank`; otherwise it copies
/// the previous key and string into the hole.  Returns `true` when the head
/// was copied.
pub fn a76_insertion_shift_head(
    dst: &mut A76Record,
    prev: &mut A76Record,
    saved: &A76Record,
    ranks: &[u8],
) -> bool {
    if prev.rank_byte(ranks) >= saved.rank_byte(ranks) {
        return false;
    }
    dst.key = prev.key;
    a76_string_move_assign(&mut dst.name, &mut prev.name);
    true
}

/// `0x0062aa79`: the payload half of the insertion-sort shift.
///
/// The cached body copies `+0x20..=+0x3c` and the 16 bytes at `+0x40` from
/// the previous record to the hole, then move-assigns the two vectors at
/// `+0x50`/`+0x5c` of the shifted record.  In the record model that is the
/// payload copy plus both vector moves; the source vectors become empty.
pub fn a76_shift_tail(dst: &mut A76Record, src: &mut A76Record) {
    dst.tail = src.tail;
    a76_vector20_move_assign(&mut dst.member50, &mut src.member50);
    a76_vector16_move_assign(&mut dst.member5c, &mut src.member5c);
}

/// `0x0062ab6e`: write the saved record into the insertion hole.
///
/// The cached body copies the key, the string, the payload, and both members
/// from the frame temporary into `[ebp-0x7c]`, then destroys the temporary.
/// The function models the copy as a move of the source record.
pub fn a76_finish_insert(dst: &mut A76Record, saved: &mut A76Record) {
    dst.key = saved.key;
    a76_string_move_assign(&mut dst.name, &mut saved.name);
    dst.tail = saved.tail;
    a76_vector20_move_assign(&mut dst.member50, &mut saved.member50);
    a76_vector16_move_assign(&mut dst.member5c, &mut saved.member5c);
}

/// `0x0062ac82`: the insertion-sort outer-loop continuation.
///
/// The cached body runs the virtual release for the temporary, advances the
/// current pointer by `0x68`, and compares it with the range end.  Only the
/// index arithmetic is reproduced: `None` at the end of the range.
pub fn a76_insertion_next(current: usize, last: usize) -> Option<usize> {
    let next = current + 1;
    if next != last { Some(next) } else { None }
}

// ---------------------------------------------------------------------------
// Heap routines
// ---------------------------------------------------------------------------

/// `0x0062acd0`: `std::__push_heap(first, hole, top, value, comp)`.
///
/// The cached 64-byte body stops after the first parent comparison; the
/// complete routine is `0x0062acd0..=0x0062ae64`.  It percolates `value` up
/// from `hole` while the parent's rank is smaller, never past `top`, and
/// returns the final hole index.
pub fn a76_push_heap(
    records: &mut [A76Record],
    hole: usize,
    top: usize,
    value: A76Record,
    ranks: &[u8],
) -> usize {
    let mut hole = hole;
    if hole > top {
        let value_rank = value.rank_byte(ranks);
        let mut parent = (hole - 1) / 2;
        loop {
            if records[parent].rank_byte(ranks) >= value_rank {
                break;
            }
            records[hole] = records[parent].clone();
            hole = parent;
            if parent <= top {
                break;
            }
            parent = (parent - 1) / 2;
        }
    }
    records[hole] = value;
    hole
}

/// `0x0062ad71`: the key/string copy inside a heap record copy.
///
/// The cached 25-byte body is `*dst = *src` followed by the string move
/// assignment on `src + 4`; the payload copy continues at `0x0062ad8a`.
pub fn a76_move_head(dst: &mut A76Record, src: &mut A76Record) {
    dst.key = src.key;
    a76_string_move_assign(&mut dst.name, &mut src.name);
}

/// `0x0062ae70`: `std::__adjust_heap(first, hole, len, value, comp)`.
///
/// The cached 82-byte body is the child-selection slice; the complete routine
/// is `0x0062ae70..=0x0062b038`.  It walks the larger child (`right` wins
/// ties, `cmovae` at `0x62af1e`) down to a leaf, handles the single-left-child
/// case when `second_child == len`, and finishes with [`a76_push_heap`] from
/// the last hole back to the original `hole`.
pub fn a76_adjust_heap(
    records: &mut [A76Record],
    hole: usize,
    len: usize,
    value: A76Record,
    ranks: &[u8],
) -> usize {
    let top = hole;
    let mut hole = hole;
    let mut second = hole * 2 + 2;
    while second < len {
        let mut chosen = second;
        if records[second].rank_byte(ranks) < records[second - 1].rank_byte(ranks) {
            chosen = second - 1;
        }
        records[hole] = records[chosen].clone();
        hole = chosen;
        second = hole * 2 + 2;
    }
    if second == len {
        records[hole] = records[second - 1].clone();
        hole = second - 1;
    }
    a76_push_heap(records, hole, top, value, ranks)
}

/// `0x0062af2e`: the key/string copy of the adjust-heap child move.
///
/// The cached 22-byte body copies `*src` to `*dst` and calls the string move
/// assignment on `src + 4`; it is the same head copy as [`a76_move_head`] in
/// the `0x0062ae70` routine.
pub fn a76_adjust_copy_head(dst: &mut A76Record, src: &mut A76Record) {
    a76_move_head(dst, src);
}

/// `0x0062b040`: `std::make_heap(first, last, comp)`.
///
/// The cached 114-byte body stops at the first temporary copy; the complete
/// routine is `0x0062b040..=0x0062b2e8`.  The recovered element count is
/// `(last - first) / 0x68` (the `0x4ec4ec4f` magic multiply with the `>> 5`
/// correction), the start parent is `count / 2 - 1`, and each parent is moved
/// into a temporary and sifted down by [`a76_adjust_heap`] until parent 0.
pub fn a76_make_heap(records: &mut [A76Record], ranks: &[u8]) {
    let len = records.len();
    if len < 2 {
        return;
    }
    let mut parent = len / 2;
    while parent > 0 {
        parent -= 1;
        let value = records[parent].clone();
        a76_adjust_heap(records, parent, len, value, ranks);
    }
}

/// `0x0062b2f0`: `std::sort_heap(first, last, comp)`.
///
/// The cached 55-byte body stops after the `0xcf < last - first` threshold;
/// the complete routine is `0x0062b2f0..=0x0062b5a9`.  Each iteration moves
/// the root record to the shrinking end, takes the old end record as the
/// value, and calls [`a76_adjust_heap`] with the reduced length.
pub fn a76_sort_heap(records: &mut [A76Record], ranks: &[u8]) {
    let mut len = records.len();
    while len > 1 {
        len -= 1;
        let value = records[len].clone();
        records[len] = records[0].clone();
        a76_adjust_heap(records, 0, len, value, ranks);
    }
}

/// `0x0062b327`: the four pointer offsets set up by `sort_heap`.
///
/// The cached 80-byte body computes
/// `first + 4`, `first + 0x50`, `first + 0x5c` and `0xffffffa4 - first`
/// (that is `-0x5c - first`).  All four are wrapping 32-bit additions.
pub fn a76_sort_heap_offsets(first: u32) -> [u32; 4] {
    [
        first.wrapping_add(0x04),
        first.wrapping_add(0x50),
        first.wrapping_add(0x5c),
        0u32.wrapping_sub(0x5c).wrapping_sub(first),
    ]
}

// ---------------------------------------------------------------------------
// Locale / helper object rows
// ---------------------------------------------------------------------------

/// Size of the object allocated by `0x0062b7c0` (`push 0x44`).
pub const A76_NODE44_BYTES: u32 = 0x44;

/// Vtable word stored by `0x0062b7c0` (`mov DWORD PTR [esi],0xd3427c`).
pub const A76_NODE44_VTABLE: u32 = 0x00d3_427c;

/// The `0x44`-byte object built by `0x0062b7c0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A76Node44 {
    /// `+0x00`.
    pub vtable: u32,
    /// `+0x04`.
    pub word_04: u32,
    /// `+0x08`.
    pub word_08: u32,
    /// `+0x0c`.
    pub word_0c: u32,
    /// `+0x10..=+0x3f`, zeroed by the shipped `memset`.
    pub bytes_10_3f: [u8; 0x30],
    /// `+0x40`.
    pub byte_40: u8,
}

/// `0x0062b7c0`: allocate and initialise one `0x44`-byte object.
///
/// The cached 12-byte body is only the `FUN_00923df0(0x44)` call; the complete
/// routine runs to `0x0062b81b`.  The recovered sequence is `new(0x44)`,
/// return null on failure, otherwise
/// `memset(object, 0, 0x44)`, store vtable `0x00d3427c`, zero `+0x04`,
/// `+0x08`, `+0x0c` and `+0x40`, and write the object to the out-parameter.
/// Allocation failure is not modelled; the initialised object is returned.
pub fn a76_new_node_44() -> A76Node44 {
    A76Node44 {
        vtable: A76_NODE44_VTABLE,
        word_04: 0,
        word_08: 0,
        word_0c: 0,
        bytes_10_3f: [0; 0x30],
        byte_40: 0,
    }
}

/// The three values read by `0x0062b820` from the object's virtual
/// `vtable + 8` call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A76LocaleName {
    /// The byte at frame `-0x14`: nonzero selects the inline short-name path
    /// (the shipped code does not allocate in that branch).
    pub flag: bool,
    /// The dword at frame `-0x1c`; `0x14` is the largest inline length.
    pub len: u32,
    /// The dword at frame `-0x18`; `4` is the largest inline category.
    pub category: u32,
}

/// `0x0062b820`: the tagged locale initialiser.
///
/// The shipped body stores `1` in `*this` and strips bit 0 from the tagged
/// argument.  When the pointer is non-null it calls `vtable + 8` and takes the
/// inline path only when `flag != 0`, `len <= 0x14` and `category <= 4`.
/// Otherwise it performs `operator new(len, 8)` (the `FUN_0093ec70`
/// allocator accessor followed by `[vtable + 4]`), sets the ownership bit, and
/// then calls `vtable + 0x10`.  The shipped code stores `allocated_ptr | 1`
/// in `*this` on the heap branch and leaves `1` on the inline branch; the
/// pointer is not reproducible, so the model returns the `1` word and the
/// selected storage (absent when the pointer is null).
pub fn a76_locale_init(name: Option<A76LocaleName>) -> (u32, Option<A76NameStorage>) {
    let Some(name) = name else {
        return (1, None);
    };
    let storage = if name.flag && name.len <= 0x14 && name.category <= 4 {
        A76NameStorage::Inline
    } else {
        A76NameStorage::Heap {
            bytes: name.len,
            align: 8,
        }
    };
    (1, Some(storage))
}

/// `0x0062bde0`: choose inline storage or the allocating branch.
///
/// `if (flag != 0 && len < 0x15 && category < 5) return this;` otherwise the
/// body calls the allocator accessor and requests `operator new(len, 8)`.
/// `len < 0x15` is `<= 0x14`, matching `0x0062b820`'s inline bound.
pub fn a76_name_storage(flag: bool, len: u32, category: u32) -> A76NameStorage {
    if flag && len < 0x15 && category < 5 {
        A76NameStorage::Inline
    } else {
        A76NameStorage::Heap {
            bytes: len,
            align: 8,
        }
    }
}

/// `0x0062be30`: whether the refresh call is skipped.
///
/// `if (FUN_005b5970() != 0 && flag == 0) return;` otherwise the body calls
/// `FUN_005b4e00(param_1)`.  The boolean result is `true` when that call is
/// reached.
pub fn a76_should_refresh(probe_nonzero: u32, flag: bool) -> bool {
    !(probe_nonzero != 0 && !flag)
}

// ---------------------------------------------------------------------------
// Sorted-vector / 16-byte append rows
// ---------------------------------------------------------------------------

/// Result of a successful [`a76_find_append16`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A76Append16 {
    /// Index of the record whose key matched.
    pub element: usize,
    /// The 16-byte value actually stored (possibly a self-reference copy).
    pub value: [u32; 4],
    /// Capacity word after the optional growth step.
    pub cap_flags: u32,
}

/// `0x0062be5c`: find a record by key, then append a 16-byte value to the
/// record's `+0x5c` vector.
///
/// The cached 204-byte body ends inside the growth block; the complete
/// routine is `0x0062be5c..=0x0062bffd`.  Its recoverable arithmetic is:
///
/// ```text
/// for (p = data; p != data + count * 0x1a; p += 0x1a)
///     if (*p == (uint)(byte)key) break;
/// if (p == end) return;
/// code = FUN_005b4ce0(obj) & 0xffff;
/// if (code in {0x24, 0x61, 0x3b}) code = FUN_005b65c0(obj) & 0xffff;
/// if (code in {6, 0x1f, 0x1e, 4}) return;
/// value = { caller_word,
///           FUN_0058ddf0(obj) * FUN_006dfa30(),
///           FUN_006dfa30() * (FUN_005b49e0(obj) & 0xffff),
///           code };
/// if (count == cap) {
///     new_count = cap + 1;
///     if (new_count > cap) {
///         growth = (cap_word >> 1) & 0x3fffffff;
///         extra  = (cap + growth >= new_count) ? growth : 1;
///         cap_word = (cap_word & 0x80000000) | ((cap + extra) & 0x7fffffff);
///     }
/// }
/// if (index_of_local_in_vector < count) value = vector[index];
/// vector[count] = value; count += 1;
/// ```
///
/// The four callback results are supplied as `keycode`, `keycode_alt`,
/// `field_a` and `field_b`; `source_index` is the shipped
/// `(&value - data) >> 4` self-reference index.  Returns `None` on a key miss
/// or a keycode in the skip set.
#[allow(clippy::too_many_arguments)]
pub fn a76_find_append16(
    elements: &[A76Record],
    key: u8,
    vector: &mut A76Vector16,
    caller_word: u32,
    keycode: u32,
    keycode_alt: u32,
    field_a: u32,
    field_b: u32,
    source_index: Option<usize>,
) -> Option<A76Append16> {
    let element = elements.iter().position(|r| r.key == u32::from(key))?;
    let mut code = keycode & 0xffff;
    if matches!(code, 0x24 | 0x61 | 0x3b) {
        code = keycode_alt & 0xffff;
    }
    if matches!(code, 6 | 0x1f | 0x1e | 4) {
        return None;
    }
    let count = vector.items.len() as u32;
    let cap = vector.cap_flags & 0x7fff_ffff;
    if count == cap {
        let new_count = cap.wrapping_add(1);
        if new_count > cap {
            let growth = (vector.cap_flags >> 1) & 0x3fff_ffff;
            let extra = if cap.wrapping_add(growth) >= new_count {
                growth
            } else {
                1
            };
            let new_cap = cap.wrapping_add(extra);
            vector.cap_flags = (vector.cap_flags & 0x8000_0000) | (new_cap & 0x7fff_ffff);
        }
    }
    let mut value = [caller_word, field_a, field_b, code];
    if let Some(index) = source_index {
        if (index as u32) < count {
            value = vector.items[index];
        }
    }
    vector.items.push(value);
    Some(A76Append16 {
        element,
        value,
        cap_flags: vector.cap_flags,
    })
}

/// `0x0062bfb4`: finish a 16-byte append after storage has been allocated.
///
/// The cached 76-byte body stores the data pointer at `+0x5c`, clears the
/// capacity word's high bit (`*+0x64 &= 0x7fffffff`), selects `data[index]`
/// when `index < count` and otherwise the caller's value, stores it at
/// `count`, and increments the count.  The virtual call at `vtable + 0x50` is
/// outside the value model.
pub fn a76_finish_append16(
    vector: &mut A76Vector16,
    value: [u32; 4],
    source_index: Option<usize>,
) -> [u32; 4] {
    vector.cap_flags &= 0x7fff_ffff;
    let count = vector.items.len();
    let chosen = match source_index {
        Some(index) if index < count => vector.items[index],
        _ => value,
    };
    vector.items.push(chosen);
    chosen
}

// ---------------------------------------------------------------------------
// Member move fragments (the two nested vectors of the record copy)
// ---------------------------------------------------------------------------

/// `0x0062aae5`: move the `+0x50` member during an insertion-sort shift.
///
/// The cached 60-byte body clears the destination data word, frees it when
/// its capacity word is non-negative, calls the vector20 move helper, then
/// continues into the `+0x5c` member.
pub fn a76_move_member50(dst: &mut A76Record, src: &mut A76Record) {
    if (dst.member50.cap_flags as i32) >= 0 {
        dst.member50 = A76Vector20::a76_empty();
    }
    a76_vector20_move_assign(&mut dst.member50, &mut src.member50);
}

/// `0x0062ab21`: move the `+0x5c` member during an insertion-sort shift.
///
/// The cached 74-byte body is the `+0xa4`/`+0x3c` half of the same sequence;
/// it frees the destination member when `+0x64 >= 0` and then move-assigns.
pub fn a76_move_member5c(dst: &mut A76Record, src: &mut A76Record) {
    if (dst.member5c.cap_flags as i32) >= 0 {
        dst.member5c = A76Vector16::a76_empty();
    }
    a76_vector16_move_assign(&mut dst.member5c, &mut src.member5c);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent76 {
    use super::*;

    /// Identity rank column: `rank(key) == key` for the small keys used here.
    fn ranks() -> Vec<u8> {
        (0..=0x90u32).map(|i| i as u8).collect()
    }

    fn record(key: u32, marker: u32) -> A76Record {
        let mut record = A76Record::a76_new(key);
        record.name.bytes = vec![marker as u8; 3];
        record.tail.w20 = [marker, marker + 1, marker + 2, marker + 3, marker + 4];
        record.tail.b34 = [1, 2, 3];
        record.tail.w38 = marker + 5;
        record.tail.w3c = marker + 6;
        record.tail.w40 = [marker + 7; 4];
        record.member50.items.push([marker; 5]);
        record.member50.cap_flags = 1;
        record.member5c.items.push([marker; 4]);
        record.member5c.cap_flags = 1;
        record
    }

    #[test]
    fn key_remap_matches_00748d40() {
        assert_eq!(a76_key_index(0x80), 0x0c);
        assert_eq!(a76_key_index(0x81), 0x0d);
        assert_eq!(a76_key_index(0x82), 0x0e);
        assert_eq!(a76_key_index(0x83), 0x0f);
        assert_eq!(a76_key_index(0x84), 0x84);
        assert_eq!(a76_key_index(0x0c), 0x0c);
        assert_eq!(a76_rank_of(0x80, &ranks()), 0x0c);
        assert_eq!(a76_rank_of(0x07, &ranks()), 0x07);
    }

    #[test]
    fn rank_table_base_is_wrapping() {
        assert_eq!(a76_rank_table_base(0, 0), 0xfb59_69c2);
        assert_eq!(a76_rank_table_base(0xfb59_69c2, 0), 0);
        assert_eq!(
            a76_rank_table_base(0x10, 0x20),
            (0xfb59_69c2u32 - 0x10) ^ 0x20
        );
    }

    #[test]
    fn comparator_and_swap_fragment() {
        let ranks = ranks();
        let mut low = A76Record::a76_new(3);
        let mut high = A76Record::a76_new(8);
        assert!(a76_record_less(&low, &high, &ranks));
        assert!(!a76_record_less(&high, &low, &ranks));
        assert!(!a76_swap_if_not_less(&mut low, &mut high, &ranks));
        assert_eq!((low.key, high.key), (3, 8));
        assert!(a76_swap_if_not_less(&mut high, &mut low, &ranks));
        assert_eq!((low.key, high.key), (8, 3));
    }

    #[test]
    fn insertion_sort_is_descending_by_rank() {
        let ranks = ranks();
        let mut records: Vec<_> = [3, 1, 4, 1, 5, 9, 2, 6]
            .into_iter()
            .map(|k| A76Record::a76_new(k))
            .collect();
        a76_insertion_sort(&mut records, &ranks);
        let keys: Vec<u32> = records.iter().map(|r| r.key).collect();
        assert_eq!(keys, vec![9, 6, 5, 4, 3, 2, 1, 1]);
        assert_eq!(a76_insertion_next(0, 8), Some(1));
        assert_eq!(a76_insertion_next(7, 8), None);
    }

    #[test]
    fn insertion_shift_head_and_tail() {
        let ranks = ranks();
        let mut saved = record(9, 0x30);
        let mut prev = record(4, 0x40);
        let mut dst = A76Record::a76_new(0);
        assert!(a76_insertion_shift_head(
            &mut dst, &mut prev, &saved, &ranks
        ));
        assert_eq!(dst.key, 4);
        assert_eq!(dst.name.bytes, vec![0x40; 3]);
        // The source string is inline (0x8000000f), so the move helper copies
        // it and leaves the source unchanged.
        assert_eq!(prev.name.bytes, vec![0x40; 3]);
        a76_shift_tail(&mut dst, &mut prev);
        assert_eq!(dst.tail.w20, [0x40, 0x41, 0x42, 0x43, 0x44]);
        assert_eq!(dst.member50.items, vec![[0x40; 5]]);
        assert!(prev.member50.items.is_empty());
        assert_eq!(prev.member50.cap_flags, A76_VECTOR_INLINE_FLAGS);
        a76_finish_insert(&mut dst, &mut saved);
        assert_eq!(dst.key, 9);
        assert_eq!(dst.tail.w40, [0x37; 4]);
        assert_eq!(dst.member5c.items, vec![[0x30; 4]]);
        assert!(saved.member5c.items.is_empty());
    }

    #[test]
    fn insertion_shift_stops_when_previous_is_not_smaller() {
        let ranks = ranks();
        let saved = A76Record::a76_new(2);
        let mut prev = A76Record::a76_new(2);
        let mut dst = A76Record::a76_new(0);
        assert!(!a76_insertion_shift_head(
            &mut dst, &mut prev, &saved, &ranks
        ));
        assert_eq!(dst.key, 0);
        assert_eq!(prev.key, 2);
    }

    #[test]
    fn move_construct_steals_heap_members_and_string() {
        let mut src = record(5, 0x11);
        src.name.cap_flags = 0x20; // heap-backed string
        let dst = a76_move_construct(&mut src);
        assert_eq!(dst.key, 5);
        assert_eq!(dst.name.bytes, vec![0x11; 3]);
        assert_eq!(dst.name.cap_flags, 0x20);
        assert!(src.name.bytes.is_empty());
        assert_eq!(src.name.cap_flags, A76_STRING_INLINE_FLAGS);
        assert_eq!(dst.member50.items, vec![[0x11; 5]]);
        assert!(src.member50.items.is_empty());
        assert_eq!(src.member50.cap_flags, A76_VECTOR_INLINE_FLAGS);
        assert_eq!(dst.member5c.items, vec![[0x11; 4]]);
        assert!(src.member5c.items.is_empty());
    }

    #[test]
    fn move_construct_copies_inline_members_and_leaves_source() {
        let mut src = record(5, 0x22);
        src.name.cap_flags = A76_STRING_INLINE_FLAGS;
        src.member50.cap_flags = A76_VECTOR_INLINE_FLAGS;
        src.member5c.cap_flags = A76_VECTOR_INLINE_FLAGS;
        let dst = a76_move_construct(&mut src);
        assert_eq!(dst.name.bytes, src.name.bytes);
        assert_eq!(dst.name.cap_flags, A76_STRING_INLINE_FLAGS);
        assert_eq!(dst.member50.items, src.member50.items);
        assert_eq!(dst.member5c.items, src.member5c.items);
        assert_eq!(src.member50.cap_flags, A76_VECTOR_INLINE_FLAGS);
    }

    #[test]
    fn copy_construct_is_deep_and_leaves_source() {
        let src = record(7, 0x55);
        let mut dst = a76_copy_construct(&src);
        assert_eq!(dst.key, src.key);
        assert_eq!(dst.name.bytes, src.name.bytes);
        assert_eq!(dst.name.cap_flags, A76_STRING_INLINE_FLAGS);
        assert_eq!(dst.member50.cap_flags, 1);
        assert_eq!(dst.member5c.cap_flags, 1);
        dst.name.bytes[0] = 0xff;
        dst.member50.items[0][0] = 0xff;
        assert_eq!(src.name.bytes[0], 0x55);
        assert_eq!(src.member50.items[0][0], 0x55);
    }

    #[test]
    fn move_assign_releases_heap_destination_members() {
        let mut src = record(3, 0x66);
        let mut dst = A76Record::a76_new(0);
        dst.name.cap_flags = 0x10;
        a76_move_assign(&mut dst, &mut src);
        assert_eq!(dst.key, 3);
        // Inline source: the destination's non-inline header is preserved.
        assert_eq!(dst.name.cap_flags, 0x10);
        assert_eq!(dst.name.bytes, vec![0x66; 3]);
        assert_eq!(dst.member50.items, vec![[0x66; 5]]);
        assert!(src.member50.items.is_empty());

        let mut src = record(4, 0x77);
        let mut dst = A76Record::a76_new(0);
        dst.member50 = A76Vector20 {
            items: vec![[0xee; 5]],
            cap_flags: 1,
        };
        dst.member5c = A76Vector16 {
            items: vec![[0xee; 4]],
            cap_flags: 1,
        };
        a76_move_assign(&mut dst, &mut src);
        assert_eq!(dst.member50.items, vec![[0x77; 5]]);
        assert_eq!(dst.member5c.items, vec![[0x77; 4]]);
    }

    #[test]
    fn member_move_fragments_and_destroy() {
        let mut src = record(1, 0x12);
        let mut dst = A76Record::a76_new(0);
        a76_move_member50(&mut dst, &mut src);
        assert_eq!(dst.member50.items, vec![[0x12; 5]]);
        a76_move_member5c(&mut dst, &mut src);
        assert_eq!(dst.member5c.items, vec![[0x12; 4]]);

        let mut heap_dst = A76Record::a76_new(0);
        heap_dst.member50 = A76Vector20 {
            items: vec![[0xee; 5]],
            cap_flags: 1,
        };
        a76_move_member50(&mut heap_dst, &mut src);
        assert_eq!(heap_dst.member50.items.len(), 0);

        let mut victim = record(2, 0x13);
        victim.name.cap_flags = 0x10;
        a76_destroy(&mut victim);
        assert_eq!(victim.member50.cap_flags, A76_VECTOR_INLINE_FLAGS);
        assert_eq!(victim.member5c.cap_flags, A76_VECTOR_INLINE_FLAGS);
        assert_eq!(victim.name.cap_flags, A76_STRING_INLINE_FLAGS);
        assert!(victim.member50.items.is_empty());
    }

    #[test]
    fn make_heap_and_sort_heap_round_trip() {
        let ranks = ranks();
        let mut records: Vec<_> = [3, 1, 4, 1, 5, 9, 2, 6]
            .into_iter()
            .map(|k| A76Record::a76_new(k))
            .collect();
        a76_make_heap(&mut records, &ranks);
        for i in 0..records.len() {
            for child in [2 * i + 1, 2 * i + 2] {
                if child < records.len() {
                    assert!(
                        records[i].rank_byte(&ranks) >= records[child].rank_byte(&ranks),
                        "heap property broken at {i} -> {child}"
                    );
                }
            }
        }
        assert_eq!(records[0].key, 9);
        a76_sort_heap(&mut records, &ranks);
        let keys: Vec<u32> = records.iter().map(|r| r.key).collect();
        assert_eq!(keys, vec![1, 1, 2, 3, 4, 5, 6, 9]);
    }

    #[test]
    fn push_heap_and_adjust_heap() {
        let ranks = ranks();
        let mut records: Vec<_> = [5, 3, 4, 1]
            .into_iter()
            .map(|k| A76Record::a76_new(k))
            .collect();
        a76_make_heap(&mut records, &ranks);
        records.push(A76Record::a76_new(9));
        let last = records.len() - 1;
        let value = records[last].clone();
        let hole = a76_push_heap(&mut records, last, 0, value, &ranks);
        assert_eq!(hole, 0);
        assert_eq!(records[0].key, 9);

        // pop_heap: stash the root, put the old end at the root, sift it down.
        let old_end = records[records.len() - 1].clone();
        let root = records[0].clone();
        let len = records.len() - 1;
        records[len] = root;
        a76_adjust_heap(&mut records, 0, len, old_end, &ranks);
        assert_eq!(records[0].key, 5);
    }

    #[test]
    fn sort_heap_offsets_match_the_recovered_pointers() {
        assert_eq!(
            a76_sort_heap_offsets(0x1000),
            [0x1004, 0x1050, 0x105c, 0xffff_efa4]
        );
        assert_eq!(
            a76_sort_heap_offsets(0xffff_fff0),
            [0xffff_fff4, 0x40, 0x4c, 0xffff_ffb4]
        );
    }

    #[test]
    fn node44_initialisation() {
        let node = a76_new_node_44();
        assert_eq!(node.vtable, 0x00d3_427c);
        assert_eq!((node.word_04, node.word_08, node.word_0c), (0, 0, 0));
        assert_eq!(node.bytes_10_3f, [0; 0x30]);
        assert_eq!(node.byte_40, 0);
    }

    #[test]
    fn locale_and_name_storage_choices() {
        let good = A76LocaleName {
            flag: true,
            len: 0x14,
            category: 4,
        };
        assert_eq!(
            a76_locale_init(Some(good)),
            (1, Some(A76NameStorage::Inline))
        );
        assert_eq!(a76_locale_init(None), (1, None));
        assert_eq!(
            a76_locale_init(Some(A76LocaleName {
                flag: true,
                len: 0x15,
                category: 4,
            })),
            (
                1,
                Some(A76NameStorage::Heap {
                    bytes: 0x15,
                    align: 8,
                })
            )
        );
        assert_eq!(
            a76_locale_init(Some(A76LocaleName {
                flag: false,
                len: 0,
                category: 0,
            })),
            (1, Some(A76NameStorage::Heap { bytes: 0, align: 8 }))
        );
        assert_eq!(a76_name_storage(true, 0x14, 4), A76NameStorage::Inline);
        assert_eq!(
            a76_name_storage(true, 0x15, 4),
            A76NameStorage::Heap {
                bytes: 0x15,
                align: 8,
            }
        );
        assert_eq!(
            a76_name_storage(true, 0x14, 5),
            A76NameStorage::Heap {
                bytes: 0x14,
                align: 8,
            }
        );
        assert_eq!(
            a76_name_storage(false, 0, 0),
            A76NameStorage::Heap { bytes: 0, align: 8 }
        );
    }

    #[test]
    fn refresh_predicate() {
        assert!(!a76_should_refresh(1, false));
        assert!(a76_should_refresh(1, true));
        assert!(a76_should_refresh(0, false));
    }

    #[test]
    fn find_and_append16_growth_and_self_reference() {
        let elements = vec![A76Record::a76_new(7), A76Record::a76_new(8)];
        let mut vector = A76Vector16::a76_empty();
        let result = a76_find_append16(
            &elements,
            7,
            &mut vector,
            0xaaaa,
            0x20,
            0,
            0x1111,
            0x2222,
            None,
        )
        .unwrap();
        assert_eq!(result.element, 0);
        assert_eq!(result.value, [0xaaaa, 0x1111, 0x2222, 0x20]);
        // count 0, low cap 0 -> growth extra 1; the inline bit is preserved.
        assert_eq!(result.cap_flags, 0x8000_0001);
        assert_eq!(vector.items, vec![[0xaaaa, 0x1111, 0x2222, 0x20]]);

        // A keycode in {6, 0x1f, 0x1e, 4} returns without appending.
        assert!(a76_find_append16(&elements, 8, &mut vector, 0, 6, 0, 0, 0, None).is_none());
        // A miss also returns None.
        assert!(a76_find_append16(&elements, 9, &mut vector, 0, 0x20, 0, 0, 0, None).is_none());

        // Existing entry of two items with capacity 2 grows by one.
        vector.items = vec![[1, 2, 3, 4], [5, 6, 7, 8]];
        vector.cap_flags = 2;
        let result = a76_find_append16(&elements, 7, &mut vector, 9, 0x20, 0, 9, 9, None).unwrap();
        assert_eq!(result.cap_flags, 3);
        assert_eq!(vector.items.len(), 3);
    }

    #[test]
    fn finish_append16_uses_self_reference() {
        let mut vector = A76Vector16 {
            items: vec![[1, 2, 3, 4], [5, 6, 7, 8]],
            cap_flags: 0x8000_0002,
        };
        let stored = a76_finish_append16(&mut vector, [9, 9, 9, 9], Some(0));
        assert_eq!(stored, [1, 2, 3, 4]);
        assert_eq!(vector.cap_flags, 2);
        assert_eq!(vector.items.len(), 3);
        let stored = a76_finish_append16(&mut vector, [9, 9, 9, 9], Some(99));
        assert_eq!(stored, [9, 9, 9, 9]);
    }
}

// ---------------------------------------------------------------------------
// Skipped rows in 0x0062a000..=0x0062bfff
// ---------------------------------------------------------------------------
//
// * `FUN_0062a186` (1 byte), `FUN_0062a41f` (1 byte), `FUN_0062a5da` (1 byte):
//   cached bodies are `halt_baddata()`; the addresses are mid-routine
//   instruction starts (a `call` target byte and `mov` bytes), so there is no
//   standalone function to reproduce.
// * `FUN_0062a315` (3 bytes): same bad-data stub; the bytes are the
//   `cmovae ecx,edx` inside the partition routine.
// * `FUN_0062abcc`, `FUN_0062abff`, `FUN_0062ac30`, `FUN_0062ac59` (41..51 bytes):
//   exception-cleanup slices that release the temporary record's string and
//   vectors after the insertion-sort shift.  The same release semantics are
//   covered by [`a76_destroy`] and [`a76_move_member50`]/[`a76_move_member5c`].
// * `FUN_0062adf6` (91 bytes): mid-`a76_push_heap` copy-and-continue slice; the
//   whole routine is [`a76_push_heap`] and the head copy is [`a76_move_head`].
// * `FUN_0062af44` (117 bytes): mid-`a76_adjust_heap` payload copy; the whole
//   routine is [`a76_adjust_heap`] and the head copy is
//   [`a76_adjust_copy_head`].
// * `FUN_0062afa2`, `FUN_0062afd5` (23/100 bytes): `a76_adjust_heap` cleanup and
//   loop-continuation tails, covered by the same routine.
// * `FUN_0062b0b2` (224 bytes): mid-`a76_make_heap` payload/member copy, covered
//   by [`a76_move_construct`] and [`a76_make_heap`].
// * `FUN_0062b193`, `FUN_0062b1bc`, `FUN_0062b1e5` (41/41/35 bytes): cleanup tails
//   inside `make_heap`/`sort_heap`; their release work is [`a76_destroy`].
// * `FUN_0062b208` (220 bytes): mid-`a76_sort_heap` record move into the frame
//   temporary, covered by [`a76_move_construct`].
// * `FUN_0062b426` (95 bytes): mid-`a76_sort_heap` move of the first record into
//   the last slot, covered by [`a76_move_head`] and [`a76_shift_tail`].
// * `FUN_0062b485`, `FUN_0062b4b5`, `FUN_0062b51f`, `FUN_0062b548`, `FUN_0062b571`
//   (48/106/41/40/53 bytes): `a76_sort_heap` cleanup tails, covered by
//   [`a76_destroy`] and [`a76_sort_heap`].
// * `FUN_0062b8f3` (58 bytes): the body of `0x0062b8f0` after its three-byte
//   prologue; it is the same move constructor, translated once at
//   [`a76_move_construct`].
// * `FUN_0062bddd` (3 bytes): `int3`/`swi(3)` break stub, no behaviour.
