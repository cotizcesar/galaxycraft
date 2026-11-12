#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent47.rs
// ===========================================================================
// Translation batch 47.  Only this file is edited by the batch.
//
// Every `decompiled_uncited` row of `0x005f0000..=0x005f1fff` belongs to the
// compiler-generated C++ support block of the shipped image: the 24-byte SSO
// `std::string` helpers, the `std::vector` growth and range-construction
// arithmetic, the `std::list` insert limits, the `std::locale` facet lookup,
// the `iostream` open-mode setup and the `args::ArgumentParser` construction
// fragments.  None of these routines touches unit or world state, so the
// recovered behaviour is exposed as free functions; each one names its exact
// ledger address.
//
// The shipped failure helpers (`_Xlength_error`, `_Xout_of_range`,
// `_Xbad_cast`, the maximum-size guard) never return.  They are reported as
// `None`/`Err` results so callers can tell the accepted branch; no abort is
// re-executed.
//
// Rows the decompiler recovered only as a mid-routine branch fragment, as a
// register-only entry, or with a return value inherited from the caller's
// `EAX` are deliberately not guessed:
//
// * the exception-unwinding/cleanup funclets (`FUN_005f0082`, `FUN_005f017f`,
//   `FUN_005f0193`, `FUN_005f0cd3`, `FUN_005f167d`, `FUN_005f196e`, `FUN_005f19cd`,
//   `FUN_005f1b29`) only run the cleanup chain and abort;
// * `FUN_005f025b` is a flag-selected branch inside the resize of `0x005f01a0`;
// * `FUN_005f02b0` is the same byte compare as `0x005f02bb` but its result is
//   masked from the caller's entry `EAX`, so it is not self-contained;
// * `FUN_005f0450` and `FUN_005f1550` repeat the string-record moves of
//   `0x005f03f0` (an allocator variant and a two-string pair layout);
// * `FUN_005f035a`, `FUN_005f04c0`, `FUN_005f0520`, `FUN_005f0570`, `FUN_005f0680`,
//   `FUN_005f075b`, `FUN_005f0b60`, `FUN_005f0cf2`, `FUN_005f0e10`, `FUN_005f0ed0`,
//   `FUN_005f1210`, `FUN_005f14d0`, `FUN_005f16a0`, `FUN_005f1752`, `FUN_005f1de0` and
//   `FUN_005f1e32` are constructor/assign/cleanup tails whose visible effect is
//   a duplicated string copy or depends on object layouts and indirect
//   vtable calls the dump does not recover;
// * `FUN_005f1c80`, `FUN_005f1cc0`, `FUN_005f1d10`, `FUN_005f1d50` and `FUN_005f1d90`
//   are the `args::Completion`/`Error`/`ExtraError` copy constructors, whose
//   only recovered behaviour is the verbatim message carried by `+4`.

// ---------------------------------------------------------------------------
// 0x005f00a0 - range constructor of a 52-byte-element vector.
// ---------------------------------------------------------------------------

/// Element stride used by the range constructor at `0x005f00a0`.
pub const A47_VEC52_STRIDE: u32 = 0x34;

/// Maximum element count the shipped `0x005f00a0` accepts before it calls the
/// `std::vector` "too long" helper.
pub const A47_VEC52_MAX_COUNT: u32 = 0x04ec_4ec4;

/// The reason the range constructor at `0x005f00a0` rejects a length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A47Vec52Error {
    /// The requested count exceeds [`A47_VEC52_MAX_COUNT`]; retail calls the
    /// noreturn `_Xlength_error`.
    TooManyElements,
}

/// `0x005f00a0`: element count of the byte range passed to the 52-byte
/// `std::vector` range constructor.
///
/// The shipped code computes `(last - first) / 0x34` with the `0x4ec4ec4f`
/// reciprocal (`mulhi`, then a four-bit shift, i.e. `(len * 0x4ec4ec4f) >> 36`
/// over the zero-extended 32-bit length); that is the exact unsigned quotient
/// for every `u32` length, and it is what this function evaluates.  A count
/// above `0x4ec4ec4` takes the noreturn length-error path.
pub fn a47_vec52_count(byte_len: u32) -> Result<u32, A47Vec52Error> {
    let count = a47_mulhi_div_52(byte_len);
    if count > A47_VEC52_MAX_COUNT {
        Err(A47Vec52Error::TooManyElements)
    } else {
        Ok(count)
    }
}

/// The `0x005f00a0` quotient: `(byte_len * 0x4ec4ec4f) >> 36`, the reciprocal
/// multiply the shipped code uses for `/ 0x34`.
pub fn a47_mulhi_div_52(byte_len: u32) -> u32 {
    let product = u64::from(byte_len) * 0x4ec4_ec4f;
    (product >> 36) as u32
}

// ---------------------------------------------------------------------------
// 0x005f01a0 - resize of a 24-byte-element vector.
// ---------------------------------------------------------------------------

/// Maximum element count of the 24-byte `std::vector` instantiation used by
/// `0x005f01a0` (`0xaaaaaaa`, the `max_size()` the grow branch checks).
pub const A47_VEC24_MAX_COUNT: u32 = 0x0aaa_aaaa;

/// What the 24-byte `std::vector` resize at `0x005f01a0` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A47Vec24Plan {
    /// `requested == size`: retail returns without touching the storage.
    Unchanged,
    /// `requested < size`: the tail is destroyed and `end` moves back to
    /// `begin + requested * 0x18`.
    Trim { new_len: u32 },
    /// `size < requested <= capacity`: `requested - size` elements are
    /// constructed past the current end.
    Extend { new_len: u32, added: u32 },
    /// `requested > capacity`: a buffer of `new_capacity` elements is
    /// allocated, the `copied` live elements are copied, the old buffer is
    /// destroyed and the capacity becomes `new_capacity`.
    Reallocate { new_capacity: u32, copied: u32 },
}

/// `0x005f01a0`: plan of `std::vector<T>::resize(requested)` for a 24-byte
/// element type.
///
/// The grow branch rejects `requested > 0xaaaaaaa` with the noreturn
/// length-error helper, then applies retail's exact geometric growth:
///
/// ```text
/// new_capacity = requested
/// if capacity <= 0xaaaaaaa - (capacity >> 1) {
///     new_capacity = max(requested, capacity + (capacity >> 1))
/// }
/// ```
///
/// The subtraction guard is the shipped check that `capacity * 1.5` still fits
/// in `max_size`; when it fails the requested count is used unchanged, even if
/// it is below the current capacity.
pub fn a47_vec24_resize_plan(size: u32, capacity: u32, requested: u32) -> Option<A47Vec24Plan> {
    if requested <= capacity {
        if requested <= size {
            if requested == size {
                return Some(A47Vec24Plan::Unchanged);
            }
            return Some(A47Vec24Plan::Trim { new_len: requested });
        }
        return Some(A47Vec24Plan::Extend {
            new_len: requested,
            added: requested - size,
        });
    }
    if requested > A47_VEC24_MAX_COUNT {
        return None;
    }
    Some(A47Vec24Plan::Reallocate {
        new_capacity: a47_vec24_grow_capacity(capacity, requested),
        copied: size,
    })
}

/// The grow branch's capacity choice, isolated so the exact `1.5 x` arithmetic
/// (including the `0xaaaaaaa` overflow guard) is directly testable.
pub fn a47_vec24_grow_capacity(capacity: u32, requested: u32) -> u32 {
    let mut new_capacity = requested;
    if capacity <= A47_VEC24_MAX_COUNT.wrapping_sub(capacity >> 1) {
        let grown = (capacity >> 1) + capacity;
        if grown >= requested {
            new_capacity = grown;
        }
    }
    new_capacity
}

// ---------------------------------------------------------------------------
// 0x005f02bb - byte-wise ordering helper.
// ---------------------------------------------------------------------------

/// `0x005f02bb`: ordering of a caller-sized byte range, as the shipped helper
/// computes it: four bytes at a time while at least four remain, then the
/// remainder a byte at a time.
///
/// The result is `-1` when the first differing byte is smaller in `lhs` and
/// `1` otherwise.  The shipped body has no zero result: the equal-tail path
/// falls through to a literal `1`, and the zero-count entry returns `1` as
/// well.  Only the common length is compared.
pub fn a47_bytes_lex_cmp(lhs: &[u8], rhs: &[u8]) -> i32 {
    let common = lhs.len().min(rhs.len());
    for i in 0..common {
        if lhs[i] != rhs[i] {
            return if lhs[i] < rhs[i] { -1 } else { 1 };
        }
    }
    1
}

// ---------------------------------------------------------------------------
// 0x005f0320 - decimal formatting.
// ---------------------------------------------------------------------------

/// `0x005f0320`: write the decimal digits of `value` backwards into `buf`,
/// exactly like the shipped loop:
///
/// ```text
/// do { --p; q = v / 10; *p = (char)(v + q * -10 + '0'); v = q } while (q)
/// ```
///
/// Returns the index of the most significant digit.  `None` means `buf` cannot
/// hold every digit (the shipped code writes past its caller's buffer in that
/// case; the bound is made explicit here).
pub fn a47_decimal_digits(value: u32, buf: &mut [u8]) -> Option<usize> {
    let mut scaled = value;
    let mut cursor = buf.len();
    loop {
        if cursor == 0 {
            return None;
        }
        cursor -= 1;
        let quotient = scaled / 10;
        let digit = scaled - quotient * 10;
        buf[cursor] = b'0' + digit as u8;
        scaled = quotient;
        if quotient == 0 {
            return Some(cursor);
        }
    }
}

// ---------------------------------------------------------------------------
// 0x005f03f0 - move a run of 24-byte string records.
// ---------------------------------------------------------------------------

/// One 24-byte `std::string` record as the shipped copy loop sees it: four
/// dwords of inline buffer, the size at `[4]` and the capacity at `[5]`.
pub type A47StringRecord = [u32; 6];

/// `0x005f03f0`: move every 24-byte `std::string` record out of `src` into
/// `dst`, then reset the donor to the empty SSO form.
///
/// The shipped loop copies the four inline dwords and then the two scalar
/// words (`size` and `capacity`) with a single eight-byte move.  Afterwards it
/// writes the donor's `size = 0`, `capacity = 0xf` and first inline byte `0`;
/// the other three inline dwords are *not* cleared.  Returns the number of
/// records moved; the shipped routine returns the end of the destination run.
pub fn a47_move_string_records(dst: &mut [A47StringRecord], src: &mut [A47StringRecord]) -> usize {
    let moved = dst.len().min(src.len());
    for i in 0..moved {
        dst[i] = src[i];
        src[i][4] = 0;
        src[i][5] = 0xf;
        src[i][0] = 0;
    }
    moved
}

// ---------------------------------------------------------------------------
// 0x005f0740 / 0x005f07d0 - std::list<T>::insert range limits.
// ---------------------------------------------------------------------------

/// `std::list<unsigned int>` node size is 12 bytes, so MSVC's `max_size()`
/// is `0xffffffff / 12 = 0x15555555`; the shipped check at `0x005f0740`
/// compares the live size against that value minus one before each insert.
pub const A47_LIST_MAX_12_BYTE_NODE: u32 = 0x1555_5554;

/// `std::list<std::string>` uses a 32-byte node, so `max_size()` is
/// `0xffffffff / 32 = 0x7ffffff`; the shipped check at `0x005f07d0`
/// compares against that value minus one.
pub const A47_LIST_MAX_32_BYTE_NODE: u32 = 0x07ff_fffe;

/// `0x005f0740`: insert `added` elements into a `std::list<unsigned int>`.
///
/// The shipped loop inserts one node at a time and, before each insert, calls
/// the noreturn `"list<T> too long"` helper when the live size equals the
/// 12-byte-node threshold.  The recovered result is the new size, or `None`
/// when the threshold would be hit.
pub fn a47_list_insert_12_byte_node(size: u32, added: u32) -> Option<u32> {
    a47_list_insert_checked(size, added, A47_LIST_MAX_12_BYTE_NODE)
}

/// `0x005f07d0`: insert `added` elements into a `std::list<std::string>`
/// (32-byte node).  Identical shape to `0x005f0740`, with the string node
/// threshold.
pub fn a47_list_insert_32_byte_node(size: u32, added: u32) -> Option<u32> {
    a47_list_insert_checked(size, added, A47_LIST_MAX_32_BYTE_NODE)
}

fn a47_list_insert_checked(size: u32, added: u32, limit: u32) -> Option<u32> {
    let mut surviving = size;
    let mut left = added;
    while left != 0 {
        if surviving == limit {
            return None;
        }
        surviving = surviving.wrapping_add(1);
        left -= 1;
    }
    Some(surviving)
}

// ---------------------------------------------------------------------------
// 0x005f0860 / 0x005f09e0 - std::locale facet resolution.
// ---------------------------------------------------------------------------

/// `0x005f0860` and `0x005f09e0`: MSVC's `std::locale::_Getfacet` body for
/// two facet categories.
///
/// The prologue is the lock-guarded one-time id allocation shared by both
/// sites: while the category's cached id is zero (checked again under the
/// locale lock), the global counter is incremented and the new value is copied
/// into the cached slot.  The body then:
///
/// 1. returns `local[id]` when `id` is in range and the slot is non-null;
/// 2. otherwise, when the locale's `+0x14` byte allows it, returns
///    `global[id]` when that slot is in range and non-null;
/// 3. otherwise returns the static fallback facet;
/// 4. and calls the noreturn `bad_cast` helper when the fallback is null.
///
/// `None` reports step 4.  `next_id`/`category_id` are the two statics the two
/// sites use (`0x0140f278` for the shared counter and one cached id slot per
/// category).
pub fn a47_facet_resolve(
    next_id: &mut u32,
    category_id: &mut u32,
    local: &[u32],
    global: &[u32],
    fallback: u32,
    global_allowed: bool,
) -> Option<u32> {
    if *category_id == 0 {
        *next_id = next_id.wrapping_add(1);
        *category_id = *next_id;
    }
    let id = *category_id;
    if let Some(&facet) = local.get(id as usize) {
        if facet != 0 {
            return Some(facet);
        }
    }
    if global_allowed {
        if let Some(&facet) = global.get(id as usize) {
            if facet != 0 {
                return Some(facet);
            }
        }
    }
    if fallback != 0 { Some(fallback) } else { None }
}

// ---------------------------------------------------------------------------
// 0x005f0f90 / 0x005f1120 - iostream open-mode words.
// ---------------------------------------------------------------------------

/// `0x005f0f90`: the open-mode word the `std::istringstream` constructor
/// builds from the caller's mode.  The shipped arithmetic is
///
/// ```text
/// flags = (~mode & 2) | 8
/// if (mode & 8) == 0 { flags = ~mode & 2 }
/// if (mode & 4) != 0 { flags |= 0x10 }
/// ```
///
/// The returned pair is `(mode word, initial string length copied into the
/// string buffer)`.  The constructor aborts with the noreturn length-error
/// helper when the initial string's length reaches `0x80000000`; that branch
/// is reported as `None`.
pub fn a47_istringstream_init(mode: u32, initial_len: u32) -> Option<(u32, u32)> {
    let mut flags = (!mode & 2) | 8;
    if mode & 8 == 0 {
        flags = !mode & 2;
    }
    if mode & 4 != 0 {
        flags |= 0x10;
    }
    if initial_len >= 0x8000_0000 {
        return None;
    }
    Some((flags, initial_len))
}

/// `0x005f1120`: the open-mode word the `std::ostringstream` constructor
/// stores at `+0x40`.  The shipped arithmetic is
///
/// ```text
/// flags = (~mode & 1) << 2
/// if (mode & 8) != 0 { flags |= 8 }
/// if (mode & 4) != 0 { flags |= 0x10 }
/// ```
pub fn a47_ostringstream_mode(mode: u32) -> u32 {
    let mut flags = (!mode & 1) << 2;
    if mode & 8 != 0 {
        flags |= 8;
    }
    if mode & 4 != 0 {
        flags |= 0x10;
    }
    flags
}

// ---------------------------------------------------------------------------
// 0x005f10af - stream buffer _Init layout.
// ---------------------------------------------------------------------------

/// The eight slots written by the stream-buffer `_Init` at `0x005f10af`.
/// `None` marks a slot the shipped body leaves untouched for that mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A47StreambufInit {
    /// `+0x38`: `base + size`.
    pub buffer_end: u32,
    /// `+0x0c`: start of the get area, always `base`.
    pub get_start: u32,
    /// `+0x10`: current get position, written only when `mode & 2 == 0`.
    pub get_next: Option<u32>,
    /// `+0x1c`: start of the put area, always `base`.
    pub put_start: u32,
    /// `+0x20`: current put position, written only when `mode & 2 == 0`.
    pub put_next: Option<u32>,
    /// `+0x2c`: get-area byte count, always `size`.
    pub get_count: u32,
    /// `+0x30`: put-area byte count, written only when `mode & 2 == 0`.
    pub put_count: Option<u32>,
    /// `+0x3c`: the mode word with bit 0 forced on.
    pub mode: u32,
}

/// `0x005f10af`: the `_Init(base, size, mode)` layout of the 0x40-byte stream
/// buffer.
///
/// Retail always writes `+0x38 = base + size`, `+0x0c = base`, `+0x1c = base`
/// and `+0x2c = size`, then (only when `mode & 2 == 0`) sets
/// `+0x10 = base`, `+0x20 = base` or `+0x38` depending on `mode & 0x18`, and
/// `+0x30 = buffer_end - put_next`.  Finally `+0x3c = mode | 1`.
pub fn a47_streambuf_init(base: u32, size: u32, mode: u32) -> A47StreambufInit {
    let buffer_end = base.wrapping_add(size);
    let (get_next, put_next, put_count) = if mode & 2 == 0 {
        let put_next = if mode & 0x18 != 0 { buffer_end } else { base };
        (
            Some(base),
            Some(put_next),
            Some(buffer_end.wrapping_sub(put_next)),
        )
    } else {
        (None, None, None)
    };
    A47StreambufInit {
        buffer_end,
        get_start: base,
        get_next,
        put_start: base,
        put_next,
        get_count: size,
        put_count,
        mode: mode | 1,
    }
}

// ---------------------------------------------------------------------------
// 0x005f12e0 - std::string copy construction.
// ---------------------------------------------------------------------------

/// Where the `std::string` copy constructor at `0x005f12e0` keeps the new
/// characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A47StringCopyPlan {
    /// `len < 0x10`: the sixteen inline bytes are copied and the stored
    /// capacity is `0xf`.
    Inline,
    /// Heap copy: capacity `len | 0xf` clamped to `0x7fffffff`, with one extra
    /// byte allocated for the terminator.
    Heap { capacity: u32, allocation: u32 },
}

/// `0x005f12e0`: `std::string` copy-constructor layout.
///
/// The shipped body clears the destination's size/capacity, then reads the
/// donor's size; below `0x10` it copies the four inline dwords and stores
/// `capacity = 0xf`.  Otherwise it forms `len | 0xf`, clamps that to
/// `0x7fffffff`, allocates `capacity + 1` bytes and copies `len + 1` bytes
/// (the terminator included).
pub fn a47_string_copy_plan(len: u32) -> A47StringCopyPlan {
    if len < 0x10 {
        return A47StringCopyPlan::Inline;
    }
    let mut capacity = len | 0xf;
    if capacity > 0x7fff_ffff {
        capacity = 0x7fff_ffff;
    }
    A47StringCopyPlan::Heap {
        capacity,
        allocation: capacity.wrapping_add(1),
    }
}

// ---------------------------------------------------------------------------
// 0x005f1370 - std::string::assign(const string&, pos, count).
// ---------------------------------------------------------------------------

/// `0x005f1370`: substring window of the `assign(const string&, pos, count)`
/// form.
///
/// Retail first resets the destination to the empty SSO form, then checks
/// `pos <= size`; the noreturn `out_of_range` helper fires otherwise (`None`
/// here).  The number of characters taken is `min(size - pos, count)`.
/// Returns `(pos, count)`.
pub fn a47_substr_window(size: u32, pos: u32, count: u32) -> Option<(u32, u32)> {
    if pos > size {
        return None;
    }
    let available = size - pos;
    let taken = if available < count { available } else { count };
    Some((pos, taken))
}

// ---------------------------------------------------------------------------
// 0x005f13c0 - std::string::assign(count, char).
// ---------------------------------------------------------------------------

/// Where `std::string::assign(count, ch)` at `0x005f13c0` stores its fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A47StringAssignPlan {
    /// `count <= 0xf`: the just-reset SSO buffer is filled in place and the
    /// size becomes `count`.
    Inline { len: u32 },
    /// Heap fill: capacity `max(count | 0xf, 0x16)` clamped at `0x7fffffff`,
    /// with one extra byte for the terminator.
    Heap {
        len: u32,
        capacity: u32,
        allocation: u32,
    },
}

/// `0x005f13c0`: `std::string::assign(count, ch)` layout.
///
/// The routine first resets the object (`size = 0`, `capacity = 0xf`, first
/// inline byte 0), so the growth baseline is the SSO capacity `0xf`, not the
/// old capacity.  A count at or above `0x80000000` calls the noreturn
/// length-error helper (`None`).  Otherwise the shipped capacity is
///
/// ```text
/// capacity = count | 0xf
/// if capacity < 0x80000000 { capacity = max(capacity, 0x16) }
/// else                     { capacity = 0x7fffffff }
/// ```
///
/// where `0x16 = 0xf + (0xf >> 1)` is the `1.5 x` growth from the reset SSO
/// capacity.
pub fn a47_string_assign_plan(count: u32) -> Option<A47StringAssignPlan> {
    if count <= 0xf {
        return Some(A47StringAssignPlan::Inline { len: count });
    }
    if count >= 0x8000_0000 {
        return None;
    }
    let mut capacity = count | 0xf;
    if capacity < 0x8000_0000 {
        // The reset SSO capacity is 0xf, so the shipped growth term is
        // 0xf + (0xf >> 1) = 0x16.  For every count above 0xf the `| 0xf`
        // result is already at least 0x1f, but the comparison is kept.
        if capacity < 0x16 {
            capacity = 0x16;
        }
    } else {
        capacity = 0x7fff_ffff;
    }
    Some(A47StringAssignPlan::Heap {
        len: count,
        capacity,
        allocation: capacity.wrapping_add(1),
    })
}

// ---------------------------------------------------------------------------
// 0x005f1510 - C-string view length.
// ---------------------------------------------------------------------------

/// `0x005f1510`: the two-word view constructor.
///
/// The shipped body stores the pointer and then walks it until the terminator,
/// storing `end - (pointer + 1)` as the length (a plain `strlen`).  An empty
/// first byte stores length 0 without entering the loop.  The slice bound
/// stands in for the missing terminator.
pub fn a47_cstr_len(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len())
}

// ---------------------------------------------------------------------------
// 0x005f15d0 - copy construction of the 24-byte-element vector.
// ---------------------------------------------------------------------------

/// `0x005f15d0`: element count of the 24-byte `std::vector` copy constructor.
///
/// The shipped body divides the byte distance by `0x18`, rejects a count above
/// `0xaaaaaaa` with the noreturn length-error helper (the guard can only fire
/// for a byte distance that a 32-bit difference cannot produce, but it is kept
/// here for fidelity), allocates `count * 0x18` bytes and copy-constructs each
/// element.
pub fn a47_vec24_count(byte_len: u32) -> Option<u32> {
    let count = byte_len / 0x18;
    if count > A47_VEC24_MAX_COUNT {
        None
    } else {
        Some(count)
    }
}

// ---------------------------------------------------------------------------
// 0x005f1e80 - args::Group completion hook.
// ---------------------------------------------------------------------------

/// `0x005f1e80`: the `args::Group` constructor's completion hook.
///
/// After the base class and the name string are set up, retail stores zero in
/// the group's result slot (`+0x54`), then, when the source object's `+0x24`
/// functor is non-null, calls it with the group's `+0x30` slot and stores the
/// returned value.  The hook itself is caller-supplied here.
pub fn a47_group_completion<F: FnOnce() -> u32>(hook: Option<F>) -> u32 {
    match hook {
        Some(call) => call(),
        None => 0,
    }
}

// ---------------------------------------------------------------------------
// 0x005f1f20 - args::ArgumentParser style table.
// ---------------------------------------------------------------------------

/// The style words the `args::ArgumentParser` constructor at `0x005f1f20`
/// installs.  All but the last two are opaque integers in the shipped object;
/// the offsets in the field comments are the retail ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A47ParserStyle {
    /// `+0x00` = `0x50`.
    pub field_00: u32,
    /// `+0x04` = `2`.
    pub field_04: u32,
    /// `+0x08` = `4`.
    pub field_08: u32,
    /// `+0x0c` = `4`.
    pub field_0c: u32,
    /// `+0x10` = `6`.
    pub field_10: u32,
    /// `+0x14` = `0x28`.
    pub field_14: u32,
    /// `+0x18` = `2`.
    pub field_18: u32,
    /// `+0x1c` = `1`.
    pub field_1c: u32,
    /// `+0x20..=+0x21` = `0x0101` (two one-byte flags).
    pub flags_20: u16,
    /// `+0x22` = `1`.
    pub flag_22: u8,
    /// Number of 24-byte string slots the constructor clears (`+0x24`,
    /// `+0x3c`, `+0x54`, `+0x6c`, `+0x84`).
    pub cleared_strings: u8,
    /// Text the constructor assigns to the last string slot (`+0xa0`).
    pub options_label: &'static str,
}

/// `0x005f1f20`: the ArgumentParser style table.
///
/// The shipped body writes the integer words listed on [`A47ParserStyle`],
/// clears five 24-byte string slots (each to `size = 0`, `capacity = 0xf`,
/// first byte 0), zeroes the 16-bit pair at `+0x9c`, clears the sixth string
/// slot at `+0xa0`, and finally assigns the nine-character `"{OPTIONS}"`
/// placeholder to it.
pub fn a47_parser_style() -> A47ParserStyle {
    A47ParserStyle {
        field_00: 0x50,
        field_04: 2,
        field_08: 4,
        field_0c: 4,
        field_10: 6,
        field_14: 0x28,
        field_18: 2,
        field_1c: 1,
        flags_20: 0x0101,
        flag_22: 1,
        cleared_strings: 5,
        options_label: A47_OPTIONS_LABEL,
    }
}

/// The placeholder text assigned at the end of `0x005f1f20`; the shipped call
/// passes its nine bytes explicitly, which is what pins the terminator-less
/// `assign(ptr, 9)`.
pub const A47_OPTIONS_LABEL: &str = "{OPTIONS}";

#[cfg(test)]
mod tests_agent47 {
    use super::*;

    /// `0x005f00a0`: the reciprocal multiply is the exact unsigned quotient by
    /// `0x34`, and the guard is at the `u32` maximum count.
    #[test]
    fn vec52_count_uses_the_shipped_reciprocal() {
        for len in [0u32, 1, 0x33, 0x34, 0x35, 0x67, 0x68, 51 * 4, 52 * 4] {
            assert_eq!(a47_vec52_count(len), Ok(len / 52));
            assert_eq!(a47_mulhi_div_52(len), len / 52);
        }
        assert_eq!(a47_vec52_count(0), Ok(0));
        assert_eq!(a47_vec52_count(u32::MAX), Ok(0x04ec_4ec4));
        assert_eq!(A47_VEC52_STRIDE, 52);
        assert_eq!(A47_VEC52_MAX_COUNT, u32::MAX / 52);
        assert_eq!(a47_mulhi_div_52(u32::MAX), u32::MAX / 52);
        // The guard sits exactly at the largest quotient a u32 length can
        // produce, so the accepted branch always wins with a real distance.
        assert!(a47_vec52_count(u32::MAX).is_ok());
        // Exhaustively re-check the reciprocal against plain division.
        for len in 0..0x2000u32 {
            assert_eq!(a47_mulhi_div_52(len), len / 52);
        }
    }

    /// `0x005f01a0`: the four resize branches and the exact `1.5 x` growth.
    #[test]
    fn vec24_resize_branches_and_growth() {
        assert_eq!(
            a47_vec24_resize_plan(4, 8, 4),
            Some(A47Vec24Plan::Unchanged)
        );
        assert_eq!(
            a47_vec24_resize_plan(8, 8, 3),
            Some(A47Vec24Plan::Trim { new_len: 3 })
        );
        assert_eq!(
            a47_vec24_resize_plan(2, 8, 6),
            Some(A47Vec24Plan::Extend {
                new_len: 6,
                added: 4
            })
        );
        assert_eq!(
            a47_vec24_resize_plan(0x10, 0x100, 0x120),
            Some(A47Vec24Plan::Reallocate {
                new_capacity: 0x180,
                copied: 0x10
            })
        );
        // A request above the grown capacity wins outright.
        assert_eq!(
            a47_vec24_resize_plan(1, 0x100, 0x200),
            Some(A47Vec24Plan::Reallocate {
                new_capacity: 0x200,
                copied: 1
            })
        );
        // Too many elements: the noreturn length-error branch.
        assert_eq!(a47_vec24_resize_plan(0, 1, 0x0aaa_aaab), None);
        // The 1.5x overflow guard refuses to grow and keeps the request.
        assert_eq!(a47_vec24_grow_capacity(0x0aaa_aaaa, 0x100), 0x100);
        assert_eq!(a47_vec24_grow_capacity(8, 9), 0xc);
        assert_eq!(a47_vec24_grow_capacity(0, 1), 1);
        // The guard subtraction wraps like the shipped unsigned word.
        assert_eq!(a47_vec24_grow_capacity(u32::MAX, 0x20), 0x20);
    }

    /// `0x005f02bb`: the helper orders by the first differing byte and keeps
    /// its `1` result for the equal tail.
    #[test]
    fn byte_compare_order() {
        assert_eq!(a47_bytes_lex_cmp(b"abc", b"abd"), -1);
        assert_eq!(a47_bytes_lex_cmp(b"abd", b"abc"), 1);
        assert_eq!(a47_bytes_lex_cmp(b"ab", b"ab"), 1);
        assert_eq!(a47_bytes_lex_cmp(b"", b""), 1);
        // Four-byte words are compared before the tail: 'Z' (0x5a) < 'a'.
        assert_eq!(a47_bytes_lex_cmp(b"aaaaZ", b"aaaaa"), -1);
        assert_eq!(a47_bytes_lex_cmp(b"aaaaa", b"aaaaZ"), 1);
        // Only the common length is inspected.
        assert_eq!(a47_bytes_lex_cmp(b"abc", b"ab"), 1);
    }

    /// `0x005f0320`: decimal digits are produced backwards, from the least
    /// significant position.
    #[test]
    fn decimal_digits_format() {
        let mut buf = [0u8; 16];
        let start = a47_decimal_digits(0, &mut buf).unwrap();
        assert_eq!(&buf[start..], b"0");

        let start = a47_decimal_digits(12345, &mut buf).unwrap();
        assert_eq!(&buf[start..], b"12345");

        let start = a47_decimal_digits(u32::MAX, &mut buf).unwrap();
        assert_eq!(&buf[start..], b"4294967295");
        assert_eq!(start, 16 - 10);

        let mut tiny = [0u8; 2];
        assert_eq!(a47_decimal_digits(123, &mut tiny), None);
        assert_eq!(a47_decimal_digits(99, &mut tiny), Some(0));
    }

    /// `0x005f03f0`: the six dwords move and the donor keeps only the empty
    /// SSO marker.
    #[test]
    fn string_records_move_and_reset_donor() {
        let mut src = [
            [
                0x0056_7890,
                0x1111_1111,
                0x2222_2222,
                0x3333_3333,
                0x16,
                0x1f,
            ],
            [0x0044_0000, 0, 0, 0, 3, 0xf],
        ];
        let mut dst = [[0u32; 6]; 2];
        assert_eq!(a47_move_string_records(&mut dst, &mut src), 2);
        assert_eq!(
            dst[0],
            [
                0x0056_7890,
                0x1111_1111,
                0x2222_2222,
                0x3333_3333,
                0x16,
                0x1f
            ]
        );
        assert_eq!(dst[1], [0x0044_0000, 0, 0, 0, 3, 0xf]);
        // Only the size, capacity and first inline byte are reset.
        assert_eq!(src[0], [0, 0x1111_1111, 0x2222_2222, 0x3333_3333, 0, 0xf]);
        assert_eq!(src[1], [0, 0, 0, 0, 0, 0xf]);
    }

    /// `0x005f0740` / `0x005f07d0`: the insert loops stop exactly at the
    /// shipped node thresholds.
    #[test]
    fn list_insert_limits() {
        assert_eq!(a47_list_insert_12_byte_node(0, 3), Some(3));
        assert_eq!(
            a47_list_insert_12_byte_node(A47_LIST_MAX_12_BYTE_NODE - 1, 1),
            Some(A47_LIST_MAX_12_BYTE_NODE)
        );
        assert_eq!(
            a47_list_insert_12_byte_node(A47_LIST_MAX_12_BYTE_NODE, 1),
            None
        );
        assert_eq!(a47_list_insert_32_byte_node(0, 2), Some(2));
        assert_eq!(
            a47_list_insert_32_byte_node(A47_LIST_MAX_32_BYTE_NODE - 1, 1),
            Some(A47_LIST_MAX_32_BYTE_NODE)
        );
        assert_eq!(
            a47_list_insert_32_byte_node(A47_LIST_MAX_32_BYTE_NODE, 1),
            None
        );
        assert_eq!(a47_list_insert_12_byte_node(0, 0), Some(0));
        // The thresholds are `max_size() - 1` for 12- and 32-byte nodes.
        assert_eq!(A47_LIST_MAX_12_BYTE_NODE + 1, u32::MAX / 12);
        assert_eq!(A47_LIST_MAX_32_BYTE_NODE + 1, u32::MAX / 32);
    }

    /// `0x005f0860` / `0x005f09e0`: the cached id allocation happens once,
    /// and the lookup prefers the locale table, then the global table, then
    /// the static facet.
    #[test]
    fn facet_id_and_lookup_order() {
        let mut next = 0;
        let mut first = 0;
        let mut second = 0;
        assert_eq!(
            a47_facet_resolve(&mut next, &mut first, &[], &[], 0x77, false),
            Some(0x77)
        );
        assert_eq!(
            a47_facet_resolve(&mut next, &mut second, &[], &[], 0x77, false),
            Some(0x77)
        );
        assert_eq!((first, second), (1, 2));
        assert_eq!(next, 2);
        // The cached id is not re-registered.
        let _ = a47_facet_resolve(&mut next, &mut first, &[], &[], 0x77, false);
        assert_eq!((next, first), (2, 1));

        // 1 is the first category id; index 1 of the local table wins.
        assert_eq!(
            a47_facet_resolve(&mut next, &mut first, &[0, 0xaa], &[0, 0xbb], 0x77, true),
            Some(0xaa)
        );
        // Null local slot, global table hit.
        assert_eq!(
            a47_facet_resolve(&mut next, &mut first, &[0, 0], &[0, 0xbb], 0x77, true),
            Some(0xbb)
        );
        // Null local slot and global lookup disabled: static facet.
        assert_eq!(
            a47_facet_resolve(&mut next, &mut first, &[0, 0], &[0, 0xbb], 0x77, false),
            Some(0x77)
        );
        // No fallback: the noreturn bad_cast branch.
        assert_eq!(
            a47_facet_resolve(&mut next, &mut first, &[0, 0], &[], 0, false),
            None
        );
    }

    /// `0x005f0f90` / `0x005f1120`: the two open-mode words.
    #[test]
    fn iostream_open_modes() {
        // istringstream: `in` alone keeps `ate|in`, `ate` adds 0x10.
        assert_eq!(a47_istringstream_init(8, 0), Some((0x0a, 0)));
        assert_eq!(a47_istringstream_init(8, 5), Some((0x0a, 5)));
        assert_eq!(a47_istringstream_init(12, 5), Some((0x1a, 5)));
        assert_eq!(a47_istringstream_init(0, 0), Some((2, 0)));
        assert_eq!(a47_istringstream_init(8, 0x8000_0000), None);
        assert_eq!(
            a47_istringstream_init(8, 0x7fff_ffff),
            Some((0x0a, 0x7fff_ffff))
        );

        // ostringstream: bit 0 inverted into 4, then `in`/`ate` added.
        assert_eq!(a47_ostringstream_mode(0), 4);
        assert_eq!(a47_ostringstream_mode(8), 0x0c);
        assert_eq!(a47_ostringstream_mode(4), 0x14);
        assert_eq!(a47_ostringstream_mode(1), 0);
        assert_eq!(a47_ostringstream_mode(1 | 4 | 8), 0x18);
    }

    /// `0x005f10af`: the `_Init` slots and the two `mode` gates.
    #[test]
    fn streambuf_init_layout() {
        let layout = a47_streambuf_init(0x1000, 0x100, 0);
        assert_eq!(layout.buffer_end, 0x1100);
        assert_eq!(layout.get_start, 0x1000);
        assert_eq!(layout.get_next, Some(0x1000));
        assert_eq!(layout.put_start, 0x1000);
        assert_eq!(layout.put_next, Some(0x1000));
        assert_eq!(layout.get_count, 0x100);
        assert_eq!(layout.put_count, Some(0x100));
        assert_eq!(layout.mode, 1);

        let layout = a47_streambuf_init(0x1000, 0x100, 2);
        assert_eq!(layout.get_next, None);
        assert_eq!(layout.put_next, None);
        assert_eq!(layout.put_count, None);
        assert_eq!(layout.mode, 3);

        let layout = a47_streambuf_init(0x1000, 0x100, 0x18);
        assert_eq!(layout.put_next, Some(0x1100));
        assert_eq!(layout.put_count, Some(0));
        assert_eq!(layout.mode, 0x19);

        let layout = a47_streambuf_init(0xffff_ff00, 0x200, 0);
        assert_eq!(layout.buffer_end, 0x100);
        assert_eq!(layout.get_count, 0x200);
    }

    /// `0x005f12e0`: the SSO threshold and the clamped heap capacity.
    #[test]
    fn string_copy_plan_arithmetic() {
        assert_eq!(a47_string_copy_plan(0), A47StringCopyPlan::Inline);
        assert_eq!(a47_string_copy_plan(0xf), A47StringCopyPlan::Inline);
        assert_eq!(
            a47_string_copy_plan(0x10),
            A47StringCopyPlan::Heap {
                capacity: 0x1f,
                allocation: 0x20
            }
        );
        assert_eq!(
            a47_string_copy_plan(0x21),
            A47StringCopyPlan::Heap {
                capacity: 0x2f,
                allocation: 0x30
            }
        );
        assert_eq!(
            a47_string_copy_plan(0x8000_0000),
            A47StringCopyPlan::Heap {
                capacity: 0x7fff_ffff,
                allocation: 0x8000_0000
            }
        );
        assert_eq!(
            a47_string_copy_plan(u32::MAX),
            A47StringCopyPlan::Heap {
                capacity: 0x7fff_ffff,
                allocation: 0x8000_0000
            }
        );
    }

    /// `0x005f1370`: the substring window clamps count and rejects `pos > size`.
    #[test]
    fn substr_window_clamps() {
        assert_eq!(a47_substr_window(10, 2, 4), Some((2, 4)));
        assert_eq!(a47_substr_window(10, 8, 4), Some((8, 2)));
        assert_eq!(a47_substr_window(10, 10, 4), Some((10, 0)));
        assert_eq!(a47_substr_window(10, 11, 4), None);
        assert_eq!(a47_substr_window(0, 0, u32::MAX), Some((0, 0)));
        assert_eq!(a47_substr_window(5, 0, u32::MAX), Some((0, 5)));
    }

    /// `0x005f13c0`: `assign(count, ch)` resets to the SSO capacity first, so
    /// every heap capacity is at least `0x16`.
    #[test]
    fn string_assign_plan_arithmetic() {
        assert_eq!(
            a47_string_assign_plan(0),
            Some(A47StringAssignPlan::Inline { len: 0 })
        );
        assert_eq!(
            a47_string_assign_plan(0xf),
            Some(A47StringAssignPlan::Inline { len: 0xf })
        );
        assert_eq!(
            a47_string_assign_plan(0x10),
            Some(A47StringAssignPlan::Heap {
                len: 0x10,
                capacity: 0x1f,
                allocation: 0x20
            })
        );
        // A count above 0xf already exceeds the 0x16 growth term, so the
        // `count | 0xf` capacity is the one that survives.
        assert_eq!(
            a47_string_assign_plan(0x11),
            Some(A47StringAssignPlan::Heap {
                len: 0x11,
                capacity: 0x1f,
                allocation: 0x20
            })
        );
        assert_eq!(a47_string_assign_plan(0x8000_0000), None);
        assert_eq!(
            a47_string_assign_plan(0x7fff_fff0),
            Some(A47StringAssignPlan::Heap {
                len: 0x7fff_fff0,
                capacity: 0x7fff_ffff,
                allocation: 0x8000_0000
            })
        );
        assert_eq!(
            a47_string_assign_plan(0x7fff_ffff),
            Some(A47StringAssignPlan::Heap {
                len: 0x7fff_ffff,
                capacity: 0x7fff_ffff,
                allocation: 0x8000_0000
            })
        );
    }

    /// `0x005f1510`: the view constructor stores the `strlen` result, with an
    /// empty first byte short-circuiting to zero.
    #[test]
    fn cstr_view_lengths() {
        assert_eq!(a47_cstr_len(b""), 0);
        assert_eq!(a47_cstr_len(&[0]), 0);
        assert_eq!(a47_cstr_len(b"abc"), 3);
        assert_eq!(a47_cstr_len(b"abc\0def"), 3);
        assert_eq!(a47_cstr_len(&[0xff, 0xfe, 0]), 2);
    }

    /// `0x005f15d0`: the copy constructor divides the byte distance by 24.
    #[test]
    fn vec24_copy_count() {
        assert_eq!(a47_vec24_count(0), Some(0));
        assert_eq!(a47_vec24_count(23), Some(0));
        assert_eq!(a47_vec24_count(24), Some(1));
        assert_eq!(a47_vec24_count(24 * 5), Some(5));
        assert_eq!(a47_vec24_count(0xffff_fff0), Some(A47_VEC24_MAX_COUNT));
        assert_eq!(a47_vec24_count(u32::MAX), Some(A47_VEC24_MAX_COUNT));
        assert_eq!(A47_VEC24_MAX_COUNT, u32::MAX / 24);
    }

    /// `0x005f1e80`: the completion hook runs only when configured.
    #[test]
    fn group_completion_hook() {
        assert_eq!(a47_group_completion::<fn() -> u32>(None), 0);
        assert_eq!(a47_group_completion(Some(|| 7u32)), 7);
        let mut calls = 0u32;
        let value = a47_group_completion(Some(|| {
            calls += 1;
            0x200
        }));
        assert_eq!((value, calls), (0x200, 1));
    }

    /// `0x005f1f20`: the style words and the `{OPTIONS}` placeholder.
    #[test]
    fn parser_style_table() {
        let style = a47_parser_style();
        assert_eq!(style.field_00, 0x50);
        assert_eq!(style.field_04, 2);
        assert_eq!(style.field_08, 4);
        assert_eq!(style.field_0c, 4);
        assert_eq!(style.field_10, 6);
        assert_eq!(style.field_14, 0x28);
        assert_eq!(style.field_18, 2);
        assert_eq!(style.field_1c, 1);
        assert_eq!(style.flags_20, 0x0101);
        assert_eq!(style.flag_22, 1);
        assert_eq!(style.cleared_strings, 5);
        assert_eq!(style.options_label, "{OPTIONS}");
        assert_eq!(style.options_label.len(), 9);
    }
}
