#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent56.rs
// ===========================================================================
// Translation batch 56.  Only this file is edited by the batch.
//
// Batch 56 owns `0x00602000..=0x00603fff`.  The ledger lists 37 rows in the
// window, all `decompiled_uncited`; 30 are translated below and seven are
// skipped:
//
// * `FUN_00603480` (56 bytes) - MSVC exception frame whose only data path is
//   the tail call `FUN_006036b0(param[0], param[1])`, i.e. the flagged-string
//   assign already translated below.  There is no arithmetic, table or
//   string literal of its own.
// * `FUN_00602e40` (456 bytes) and `FUN_00603020` (283 bytes) - MSVC
//   exception funclets around a temporary-string copy/convert.  The data
//   path is one conversion (`FUN_005ea100`, or `FUN_005f13c0` followed by
//   `FUN_006000c0`/`FUN_005f12e0`); the rest is `__finally` scaffolding and
//   block-header release bookkeeping.
// * `FUN_006031a0` (87 bytes) - string constructor that formats through
//   `FUN_005f0320` into a three-byte stack buffer and assigns the returned
//   range.  The formatter is outside the window and its contract (what the
//   three bytes encode) is not recoverable from this body.
// * `FUN_006036a0` (4 bytes) - `return *(u32 *)(this + 0x20)`; the record
//   type at `this` is not established anywhere in the window, so there is
//   no table or arithmetic to reproduce.
// * `FUN_00603a60` (211 bytes) - device/driver start-up that writes four
//   global dwords and calls six out-of-window services.  Only the written
//   constants are visible, and no in-window state or table is involved.
// * `FUN_00603f50` (42 bytes) - writes the two 0x00fa3960 globals and stores
//   the out-of-window clock value at `0x011cb522`; there is no in-window
//   arithmetic.
//
// Three retail layouts recur in the translated rows:
//
// | offset | meaning |
// |--------|---------|
// | MSVC `std::string` (0x18 bytes) | 16-byte inline/heap union at `+0x00`, size at `+0x10`, capacity at `+0x14` |
// | flagged small string (0x1c bytes) | data pointer at `+0x00`, size at `+0x04`, capacity at `+0x08` with the storage tag `0x80000000`, 16-byte inline buffer at `+0x0c` |
// | seekable stream (0x40 bytes) | get-next `+0xc`, get-first `+0x10`, put-next `+0x1c`, put-end `+0x20`, put count `+0x2c`, get count `+0x30`, high-water `+0x38`, flags `+0x3c`/`+0x40` |
//
// The high bit of the flagged capacity field is the storage tag read by the
// move constructor at `FUN_0053a5b0`: a clear bit marks an owned block the
// constructor may steal, a set bit marks storage it must deep-copy.  Every
// constructor in the window writes `0x8000000f`, and capacity comparisons
// always mask the tag with `0x7fffffff`.
// Every recovered constant - the `0xf` inline capacity, the `0x7fffffff`
// size clamp, the `cap + cap / 2` growth, the `0x1c` vector stride and the
// 32-bit `(addr - begin) / 0x1c` element index - is reproduced exactly and
// named at the site that uses it.
//
// These routines manipulate client text and stream buffers only; none of
// them reads or writes `World`/`SimUnit` simulation state, so the batch is
// written as pure free functions, matching the precedent of batches 49 and
// 50.  Out-of-window virtual slots and CRT helpers are supplied by the
// caller as closures or documented as such; no unimplemented helper is
// called.

/// `std::string` inline capacity (`+0x14 = 0xf` at every construction site in
/// the window, e.g. `0x00602730` and `0x00603150`).
pub const A56_SSO_CAPACITY: u32 = 0xf;

/// Largest representable string size/slot count (`0x7fffffff` clamps the
/// growth policy in `0x006024d0` and `0x00602790`).
pub const A56_STRING_MAX: u32 = 0x7fff_ffff;

/// Capacity values below this select the 16-byte inline buffer
/// (`cap < 0x10` tests in `0x00602790` and `0x00603150`).
pub const A56_SSO_LIMIT: u32 = 0x10;

/// High bit of the 0x1c string capacity field: the storage tag tested by the
/// move constructor at `FUN_0053a5b0` (clear = stealable block, set = deep
/// copy).  Written as set by `0x00602670`, `0x006028b0` and `0x006036b0`.
pub const A56_SMALL_FLAG: u32 = 0x8000_0000;

/// Initial flagged capacity field written at the default-construct site
/// `0x00602670`: high bit plus the 0xf inline capacity.
pub const A56_SMALL_INLINE: u32 = 0x8000_000f;

/// First growth of the flagged string from the default field:
/// `(0x8000000f >> 1 & 0x3fffffff) + (0x8000000f & 0x7fffffff)` = `7 + 15`
/// = 22 (`0x16`), the value written when 16..22 bytes are assigned.
pub const A56_SMALL_FIRST_GROWTH: u32 = 0x16;

/// Stride of the flagged-string vector: `0x006022c0` walks records by `0x1c`
/// and `0x006025e0`/`0x00602670` divide addresses by `0x1c`.
pub const A56_SMALL_STRIDE: u32 = 0x1c;

/// Stride of the `std::string` vector in `0x00602730` (`end += 0x18`).
pub const A56_STD_STRING_STRIDE: u32 = 0x18;

/// Address returned by `0x00603440` (`&DAT_00fa3930`, the static object whose
/// first two dwords `0x00603450` compares).
pub const A56_STATIC_OBJECT: usize = 0x00fa_3930;

/// A 24-byte MSVC `std::string` as needed by the push-back, reserve, move and
/// substring paths.  The 16-byte inline/heap union is represented by `bytes`;
/// `cap` is the recovered `+0x14` field (a capacity below `0x10` means the
/// inline buffer is live).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A56String {
    pub bytes: Vec<u8>,
    pub cap: u32,
}

/// A 28-byte flagged string (`0x006025e0`, `0x00602670`, `0x006028b0`,
/// `0x006036b0`).  `cap_field` keeps the `0x80000000` storage tag exactly as
/// stored at `+0x08`; `bytes` is the content behind the pointer at `+0x00`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A56SmallString {
    pub bytes: Vec<u8>,
    pub cap_field: u32,
}

/// The empty `std::string` written by the inline clear: size 0, capacity
/// `0xf`, first byte 0 (`0x00602730`, `0x00603150`).
pub fn a56_string_empty() -> A56String {
    A56String {
        bytes: Vec::new(),
        cap: A56_SSO_CAPACITY,
    }
}

/// The capacity policy shared by `0x006024d0` (push_back) and `0x00602790`
/// (reserve):
///
/// ```text
/// if (request >= 0x80000000)                    capacity = 0x7fffffff
/// else if (0x7fffffff - (cap >> 1) < cap)       capacity = 0x7fffffff
/// else                                          capacity = max(request, cap + cap / 2)
/// ```
///
/// `request` is the already-rounded byte count (`size | 0xf` for push_back,
/// `new_size | 0xf` for reserve).
fn a56_grow_capacity(request: u32, cap: u32) -> u32 {
    if request >= 0x8000_0000 {
        return A56_STRING_MAX;
    }
    if A56_STRING_MAX.wrapping_sub(cap >> 1) < cap {
        return A56_STRING_MAX;
    }
    let doubled = (cap >> 1).wrapping_add(cap);
    request.max(doubled)
}

/// Builds a `std::string` from `bytes` with the recovered capacity policy:
/// the 0xf inline capacity is kept for up to 15 bytes, otherwise the first
/// growth is `max(len | 0xf, 0xf + 0xf / 2)`.
pub fn a56_string_from_bytes(bytes: &[u8]) -> A56String {
    let mut s = a56_string_empty();
    let len = bytes.len() as u32;
    if len > s.cap {
        s.cap = a56_grow_capacity(len | A56_SSO_CAPACITY, s.cap);
    }
    s.bytes = bytes.to_vec();
    s
}

/// `0x006024d0` (`FUN_006024d0`, 169 bytes, `decompiled_uncited`):
/// `std::string::push_back`.
///
/// ```c
/// if (size < cap) { data[size] = ch; data[size + 1] = 0; ++size; return; }
/// if (size == 0x7fffffff) length_error;
/// need = (size + 1) | 0xf;
/// if (need >= 0x80000000) cap = 0x7fffffff;
/// else if (0x7fffffff - (cap >> 1) < cap) cap = 0x7fffffff;
/// else cap = max(need, cap + cap / 2);
/// ```
///
/// The translation reports the length-error path as `false` instead of
/// throwing.  The terminator written at `data[size + 1]` needs no separate
/// store because [`Vec`] keeps the content bounded by its length.
pub fn a56_string_push_back(s: &mut A56String, byte: u8) -> bool {
    let size = s.bytes.len() as u32;
    if size < s.cap {
        s.bytes.push(byte);
        return true;
    }
    if size == A56_STRING_MAX {
        return false;
    }
    let need = size.wrapping_add(1) | A56_SSO_CAPACITY;
    s.cap = a56_grow_capacity(need, s.cap);
    s.bytes.push(byte);
    true
}

/// Result of the reserve policy in `0x00602790`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A56Reserve {
    /// Nothing is written (`size > requested`, `cap == requested`, or a
    /// non-shrink path below the inline limit).
    Unchanged,
    /// New `+0x14` capacity; the body also stores `requested` at `+0x10`.
    Grew(u32),
    /// `requested < 0x10 && cap > 0xf`: the out-of-window shrink-in-place
    /// helper at `FUN_005f9be0` is taken.
    ShrankInline,
}

/// `0x00602790` (`FUN_00602790`, 195 bytes, `decompiled_uncited`): the
/// string capacity/resize policy.
///
/// The body returns immediately when `size > requested`, when
/// `cap == requested`, or when the shrink test is false.  Growing checks
/// `0x7fffffff - size < requested - size` (a length error) and then uses the
/// same clamp/growth sequence as [`a56_string_push_back`], storing the new
/// capacity at `+0x14` and `requested` at `+0x10`.  The shrinking branch below
/// 0x10 calls `FUN_005f9be0`, reported here as [`A56Reserve::ShrankInline`].
pub fn a56_string_reserve(size: u32, cap: u32, requested: u32) -> Result<A56Reserve, ()> {
    if size > requested {
        return Ok(A56Reserve::Unchanged);
    }
    if cap == requested {
        return Ok(A56Reserve::Unchanged);
    }
    if cap < requested {
        if A56_STRING_MAX.wrapping_sub(size) < requested.wrapping_sub(size) {
            return Err(());
        }
        let need = requested | A56_SSO_CAPACITY;
        return Ok(A56Reserve::Grew(a56_grow_capacity(need, cap)));
    }
    if requested < A56_SSO_LIMIT && cap > A56_SSO_CAPACITY {
        return Ok(A56Reserve::ShrankInline);
    }
    Ok(A56Reserve::Unchanged)
}

/// `0x00603150` (`FUN_00603150`, 70 bytes, `decompiled_uncited`):
/// `std::string::substr(pos, n)`.
///
/// The out string is first cleared to size 0 / capacity 0xf / first byte 0.
/// When `pos <= size`, the copied length is `min(n, size - pos)` and the
/// source pointer is the data pointer (`*(this)` when `cap > 0xf`, otherwise
/// the inline buffer).  A `pos > size` goes to `FUN_005fd620`, the
/// `out_of_range` throw, reported as `None`.
pub fn a56_string_substr(s: &A56String, pos: u32, n: u32) -> Option<A56String> {
    let size = s.bytes.len() as u32;
    if pos > size {
        return None;
    }
    let take = n.min(size - pos);
    let start = pos as usize;
    Some(a56_string_from_bytes(
        &s.bytes[start..start + take as usize],
    ))
}

/// The empty flagged string: pointer at the `+0x0c` inline buffer, size 0,
/// capacity field `0x8000000f`, first inline byte 0 (`0x00602670`,
/// `0x006036b0`).
pub fn a56_small_empty() -> A56SmallString {
    A56SmallString {
        bytes: Vec::new(),
        cap_field: A56_SMALL_INLINE,
    }
}

/// `cap_field & 0x7fffffff`: the usable capacity behind the storage tag.
pub fn a56_small_capacity(cap_field: u32) -> u32 {
    cap_field & A56_STRING_MAX
}

/// The exact field merge of `0x006028b0` and `0x006036b0`:
/// `((old ^ low) & 0x7fffffff) ^ old`.  Bits 0..30 become `low`; the
/// `0x80000000` storage tag of `old` is preserved.
pub fn a56_capacity_field_with_flag(old: u32, low: u32) -> u32 {
    ((old ^ low) & A56_STRING_MAX) ^ old
}

/// `0x006028b0` (`FUN_006028b0`, 78 bytes, `decompiled_uncited`): grow a
/// flagged vector capacity field.
///
/// ```c
/// cap = field & 0x7fffffff;
/// if (cap < requested) {
///     grown = (field >> 1 & 0x3fffffff) + cap;
///     if (grown < requested) grown = requested;
///     field = ((field ^ grown) & 0x7fffffff) ^ field;
///     /* FUN_0093ec70 reallocates the block */
/// }
/// ```
///
/// `None` means the requested count already fits.  `0x006025e0` and
/// `0x00602670` call it with `count + 1` when the vector is full.
pub fn a56_vector_growth_field(field: u32, requested: u32) -> Option<u32> {
    let cap = a56_small_capacity(field);
    if cap >= requested {
        return None;
    }
    let mut grown = (field >> 1 & 0x3fff_ffff).wrapping_add(cap);
    if grown < requested {
        grown = requested;
    }
    Some(a56_capacity_field_with_flag(field, grown))
}

/// The address-to-index divide of `0x006025e0`/`0x00602670`:
/// `(addr - begin) / 0x1c` as a 32-bit unsigned division.
pub fn a56_index_1c(addr: u32, begin: u32) -> u32 {
    addr.wrapping_sub(begin) / A56_SMALL_STRIDE
}

/// `0x00602670` (`FUN_00602670`, 140 bytes, `decompiled_uncited`): append a
/// copy of a flagged string to a 0x1c-stride vector.
///
/// The recovered body:
///
/// ```c
/// index = (param - begin) / 0x1c;          /* 0x92492493 magic divide */
/// if (count == (cap_field & 0x7fffffff))
///     FUN_006028b0((cap_field & 0x7fffffff) + 1);
/// slot = begin + count * 0x1c;
/// source = (index < count) ? &begin[index].str : param;
/// slot->ptr = slot + 0xc; slot->size = 0; slot->cap = 0x8000000f; inline[0] = 0;
/// FUN_00538810(slot, source->size);        /* capacity reserve */
/// memcpy(slot->ptr, source->ptr, source->size);
/// slot->ptr[source->size] = 0; slot->size = source->size;
/// ++count;
/// ```
///
/// The source selection reduces to the argument itself when the argument is
/// a slot of this vector; it is kept here to reproduce the `index < count`
/// branch.  `FUN_00538810` only applies the recovered capacity policy
/// (`max(22, len)` from the default field), so the copied element is exactly
/// [`a56_small_assign`] of the selected content.
///
/// Returns the (possibly grown) capacity field and the appended element.
pub fn a56_small_vector_emplace(
    vec: &mut Vec<A56SmallString>,
    cap_field: u32,
    index: u32,
    source: &A56SmallString,
) -> (u32, A56SmallString) {
    let mut field = cap_field;
    let count = vec.len() as u32;
    if count == a56_small_capacity(field) {
        if let Some(grown) = a56_vector_growth_field(field, a56_small_capacity(field) + 1) {
            field = grown;
        }
    }
    let picked = if index < count {
        &vec[index as usize]
    } else {
        source
    };
    let appended = a56_small_assign(&picked.bytes);
    vec.push(appended.clone());
    (field, appended)
}

/// `0x00602730` (`FUN_00602730`, 82 bytes, `decompiled_uncited`):
/// `std::vector<std::string>::push_back(move(value))`.
///
/// The fast path is taken when `end != capacity_end` (the caller's
/// `has_room`).  It copies the 16-byte union, the size/capacity pair and then
/// resets the source to size 0 / capacity `0xf` / first byte 0, advancing
/// `end` by `0x18`.  The full-vector path tail-calls `FUN_005ede40`, which is
/// outside this window; `false` reports that path without modelling it.
pub fn a56_push_string_move(
    vec: &mut Vec<A56String>,
    value: &mut A56String,
    has_room: bool,
) -> bool {
    if !has_room {
        return false;
    }
    let moved = std::mem::replace(value, a56_string_empty());
    vec.push(moved);
    true
}

/// `0x006036b0` (`FUN_006036b0`, 113 bytes, `decompiled_uncited`): assign a
/// byte range into a flagged 0x1c string.
///
/// The body resets the object to the inline form (`ptr = this + 0xc`,
/// size 0, capacity field `0x8000000f`, inline[0] = 0), grows the field when
/// `0xf < len` (first growth 22, then `max(grown, len)`), reserves through
/// `FUN_0093ec70`, copies `len` bytes through `FUN_00c89580`, writes the
/// terminator at `data[len]` and finally stores `size = len`.
///
/// The cached decompile renders the terminator as `*(this) = 0` and the size
/// store as `+0x04 = 0`; the instruction stream at `0x00603762`-`0x00603768`
/// is `mov byte [ebx+eax], 0` / `mov [edi+4], ebx`, i.e. the normal
/// terminator and size stores.  The translation keeps both.
pub fn a56_small_assign(src: &[u8]) -> A56SmallString {
    let len = src.len() as u32;
    let mut field = A56_SMALL_INLINE;
    if a56_small_capacity(field) < len {
        let grown = (field >> 1 & 0x3fff_ffff).wrapping_add(a56_small_capacity(field));
        let low = if grown < len { len } else { grown };
        field = a56_capacity_field_with_flag(field, low);
    }
    A56SmallString {
        bytes: src.to_vec(),
        cap_field: field,
    }
}

/// `0x006025e0` (`FUN_006025e0`, 169 bytes, `decompiled_uncited`):
/// `std::vector<flagged string>::push_back(move(value))`.
///
/// The body divides `(param - begin) / 0x1c` with the `0x92492493` magic,
/// grows the capacity field with `(cap & 0x7fffffff) + 1` when full, calls the
/// move constructor `FUN_0053a5b0(slot, source)` and increments the count.
/// The constructor branches on the source capacity field:
///
/// * bit 31 clear: steal the pointer/size/capacity, clear the bit on the new
///   element, reset the source to the inline empty form;
/// * bit 31 set: deep-copy (`FUN_00538810` reserve, `FUN_00c89580` copy,
///   terminator and size stores), leaving the source untouched.
///
/// The source is the element at `index` when `index < count`, otherwise the
/// caller's argument; both resolve to the same address for a slot argument.
/// Returns the (possibly grown) capacity field and whether the steal branch
/// ran.
pub fn a56_small_vector_push_move(
    vec: &mut Vec<A56SmallString>,
    cap_field: u32,
    index: u32,
    source: &mut A56SmallString,
) -> (u32, bool) {
    let mut field = cap_field;
    let count = vec.len() as u32;
    if count == a56_small_capacity(field) {
        if let Some(grown) = a56_vector_growth_field(field, a56_small_capacity(field) + 1) {
            field = grown;
        }
    }
    let (stolen, copied) = {
        let picked = if index < count {
            &mut vec[index as usize]
        } else {
            source
        };
        if (picked.cap_field & A56_SMALL_FLAG) == 0 {
            let mut stolen = std::mem::replace(picked, a56_small_empty());
            stolen.cap_field &= A56_STRING_MAX;
            (Some(stolen), None)
        } else {
            (None, Some(a56_small_assign(&picked.bytes)))
        }
    };
    let moved = stolen.is_some();
    let appended = stolen
        .or(copied)
        .expect("one branch always yields an element");
    vec.push(appended);
    (field, moved)
}

/// A `std::streambuf`-style putback area (`0x00602470`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A56Putback {
    /// The character storage; `gptr` and `putback_end` index into it.
    pub buf: Vec<u8>,
    /// `*(this + 0x0c)`: current get pointer.
    pub gptr: usize,
    /// `*(this + 0x1c)`: end of the putback area; 0 means null.
    pub putback_end: usize,
    /// `*(this + 0x2c)`: putback counter, incremented per accepted byte.
    pub put_count: i32,
    /// `*(this + 0x3c)`: bit 2 makes the "same character" test strict.
    pub flags: u8,
}

/// `0x00602470` (`FUN_00602470`, 84 bytes, `decompiled_uncited`):
/// `basic_streambuf::sputbackc`.
///
/// ```c
/// end = *(+0x1c);
/// if (end != 0 && gptr < end &&
///     (c == -1 || c == data[end - 1] || (flags & 2) == 0)) {
///     ++*(+0x2c);
///     --gptr;
///     if (c == -1) return 0;
///     data[gptr] = c;
///     return c;
/// }
/// return -1;
/// ```
///
/// Bit 2 of `+0x3c` is the strict-match flag: when set, the pushed-back byte
/// must equal the byte already at `end - 1`.  A zero `gptr` with a non-zero
/// `end` would step outside the buffer in retail; the translation reports
/// `-1` instead of underflowing.
pub fn a56_putback(p: &mut A56Putback, c: i32) -> i32 {
    let end = p.putback_end;
    if end != 0 && p.gptr < end && (c == -1 || c as u8 == p.buf[end - 1] || (p.flags & 2) == 0) {
        let Some(new_gptr) = p.gptr.checked_sub(1) else {
            return -1;
        };
        p.put_count += 1;
        p.gptr = new_gptr;
        if c == -1 {
            return 0;
        }
        p.buf[new_gptr] = c as u8;
        return c;
    }
    -1
}

/// `0x006022c0` (`FUN_006022c0`, 203 bytes, `decompiled_uncited`):
/// convert the 0x1c-stride record list to wide strings for a sink.
///
/// The body reads the record count from the stack pair filled by
/// `FUN_005f8130` (`local_1c` pointer, `local_18` count), allocates
/// `count * 4` bytes with `FUN_009241e0`, fills each pointer with
/// `FUN_009240e0(record->data, DAT_00d32a20, 0)`, calls the sink
/// `FUN_00600540(count, array)`, releases each wide string with
/// `FUN_00924130(ptr, DAT_00d32a20, 0)`, and finally frees the pointer array
/// with `FUN_00924220`.  Record order is preserved and the conversion is
/// performed once per record before the sink runs.  The formatter string and
/// the CRT conversion are outside the window, so the conversion is supplied
/// by the caller; the translation returns the converted array exactly as the
/// sink receives it.
pub fn a56_convert_record_strings<F>(records: &[A56SmallString], mut convert: F) -> Vec<Vec<u16>>
where
    F: FnMut(&[u8]) -> Vec<u16>,
{
    let mut wide = Vec::with_capacity(records.len());
    for record in records {
        wide.push(convert(&record.bytes));
    }
    wide
}

/// 64-bit position plus the four zeroed dwords written after it: the 0x18-byte
/// result slot used by `0x00602a10`, `0x00602a40`, `0x00602ba0` and
/// `0x00602bd0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A56SeekResult {
    /// Low dword of the 64-bit position.
    pub lo: u32,
    /// High dword of the 64-bit position.
    pub hi: u32,
    /// `param[2..6]`, always zero in this window.
    pub tail: [u32; 4],
}

/// Valid seek result: position `(lo, hi)` and four zero dwords.
pub fn a56_seek_result(lo: u32, hi: u32) -> A56SeekResult {
    A56SeekResult {
        lo,
        hi,
        tail: [0; 4],
    }
}

/// `0x00602a10` (`FUN_00602a10`, 45 bytes, `decompiled_uncited`): initialize
/// the 0x18-byte seek result to the invalid position `0xffffffff:0xffffffff`
/// followed by four zeros.
pub fn a56_seek_result_invalid_2a10() -> A56SeekResult {
    A56SeekResult {
        lo: u32::MAX,
        hi: u32::MAX,
        tail: [0; 4],
    }
}

/// `0x00602ba0` (`FUN_00602ba0`, 45 bytes, `decompiled_uncited`): the second
/// initializer site, byte-for-byte identical to [`a56_seek_result_invalid_2a10`].
pub fn a56_seek_result_invalid_2ba0() -> A56SeekResult {
    a56_seek_result_invalid_2a10()
}

/// The seekable-stream cursor recovered from `0x00602a40`/`0x00602bd0`.
/// All fields are 32-bit retail pointers; the `Option` fields carry the
/// "pointer is null" tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A56SeekState {
    /// `*(this + 0x1c)` put pointer, null-tested before every update.
    pub put_next: Option<u32>,
    /// `*(this + 0x20)` put end, also the high-water source.
    pub put_end: Option<u32>,
    /// `*(this + 0x0c)` get pointer; the seek target is relative to it.
    pub get_next: u32,
    /// `(this + 0x38)` cached high-water pointer.
    pub high_water: u32,
    /// `*(this + 0x2c)` put-area byte count.
    pub put_count: i32,
    /// `*(this + 0x10)` get first.
    pub get_first: u32,
    /// `*(this + 0x30)` get-area byte count.
    pub get_count: i32,
}

/// Refresh the high-water cache and return `(available, sign)` exactly as the
/// two seek routines do before their bounds comparison.
fn a56_seek_available(state: &mut A56SeekState) -> (u32, u32) {
    if let Some(end) = state.put_end {
        if state.high_water < end {
            state.high_water = end;
        }
    }
    let avail = state.high_water.wrapping_sub(state.get_next);
    (avail, ((avail as i32) >> 31) as u32)
}

/// Shared validation and commit of the two seek routines.
///
/// The offset `(lo, hi)` is accepted when the unsigned 64-bit compare against
/// the sign-extended available count succeeds and when either the offset is
/// zero or every mode-gated pointer is non-null.  On success `mode & 1`
/// rewrites the put pointer/count and `mode & 2` rewrites the get first, put
/// end and get count; both then return the 64-bit offset with a zero tail.
fn a56_seek_commit(
    state: &mut A56SeekState,
    lo: u32,
    hi: u32,
    mode: u8,
    avail: u32,
    avail_sign: u32,
) -> A56SeekResult {
    let in_range = hi < avail_sign || (hi <= avail_sign && lo <= avail);
    if !in_range {
        return a56_seek_result_invalid_2a10();
    }
    let zero = lo == 0 && hi == 0;
    let gated =
        (mode & 1 == 0 || state.put_next.is_some()) && (mode & 2 == 0 || state.put_end.is_some());
    if !(zero || gated) {
        return a56_seek_result_invalid_2a10();
    }

    let target = state.get_next.wrapping_add(lo);
    if (mode & 1) != 0 && state.put_next.is_some() {
        state.put_next = Some(target);
        state.put_count = state.high_water.wrapping_sub(target) as i32;
    }
    if (mode & 2) != 0 {
        if let Some(old_end) = state.put_end {
            state.get_first = state.get_next;
            state.put_end = Some(target);
            state.get_count = state
                .get_count
                .wrapping_add(old_end as i32)
                .wrapping_sub(target as i32);
        }
    }
    a56_seek_result(lo, hi)
}

/// `0x00602a40` (`FUN_00602a40`, 346 bytes, `decompiled_uncited`):
/// `basic_streambuf::seekoff(off, way, mode)`.
///
/// The 64-bit offset is formed from `(param_2, param_3)`; `way` selects the
/// base:
///
/// * `0` - zero offset from the get pointer;
/// * `1` - `put_next - get_next` when `mode & 1`, else
///   `put_end - get_next` when `mode & 2` (both only when the relevant
///   pointer is non-null or the get pointer itself is null), 64-bit
///   subtraction sign-extended from the low dword;
/// * `2` - the sign-extended available count `high_water - get_next`;
/// * any other value - invalid.
///
/// The offset is added with 32-bit carry, bounds-checked, and committed by
/// [`a56_seek_commit`].  On the invalid paths the result is
/// `0xffffffff:0xffffffff` with a zero tail, matching the two initializer
/// sites.
pub fn a56_seekoff(
    state: &mut A56SeekState,
    off_lo: u32,
    off_hi: i32,
    way: u32,
    mode: u8,
) -> A56SeekResult {
    let (avail, avail_sign) = a56_seek_available(state);
    let (base_lo, base_hi) = match way {
        0 => (0, 0),
        1 => {
            if (mode & 3) == 3 {
                return a56_seek_result_invalid_2a10();
            }
            let base = if (mode & 1) == 0 {
                if (mode & 2) != 0 && (state.put_end.is_some() || state.get_next == 0) {
                    Some(state.put_end.unwrap_or(0))
                } else {
                    None
                }
            } else if state.put_next.is_some() || state.get_next == 0 {
                Some(state.put_next.unwrap_or(0))
            } else {
                None
            };
            let Some(base) = base else {
                return a56_seek_result_invalid_2a10();
            };
            let diff = base.wrapping_sub(state.get_next);
            (diff, ((diff as i32) >> 31) as u32)
        }
        2 => (avail, avail_sign),
        _ => return a56_seek_result_invalid_2a10(),
    };

    let lo = base_lo.wrapping_add(off_lo);
    let carry = u32::from(lo < base_lo);
    let hi = (base_hi as u32)
        .wrapping_add(off_hi as u32)
        .wrapping_add(carry);
    a56_seek_commit(state, lo, hi, mode, avail, avail_sign)
}

/// `0x00602bd0` (`FUN_00602bd0`, 236 bytes, `decompiled_uncited`): seek with
/// two explicit 64-bit words.
///
/// ```c
/// lo = param_4 + param_2;                    /* CARRY4(param_4, param_2) */
/// hi = param_5 + param_3 + carry;
/// ```
///
/// The sum is bounds-checked against `high_water - get_next` exactly like
/// [`a56_seekoff`] and committed through the same mode gates; invalid seeks
/// return `0xffffffff:0xffffffff` with a zero tail.
pub fn a56_seek_base(
    state: &mut A56SeekState,
    base_lo: u32,
    base_hi: i32,
    off_lo: u32,
    off_hi: i32,
    mode: u8,
) -> A56SeekResult {
    let (avail, avail_sign) = a56_seek_available(state);
    let lo = off_lo.wrapping_add(base_lo);
    let carry = u32::from(lo < off_lo);
    let hi = (off_hi as u32)
        .wrapping_add(base_hi as u32)
        .wrapping_add(carry);
    a56_seek_commit(state, lo, hi, mode, avail, avail_sign)
}

/// `0x00602cc0` (`FUN_00602cc0`, 5 bytes, `decompiled_uncited`): empty body
/// (`ret`), the no-op virtual slot.
pub fn a56_noop() {}

/// `0x00602d00` (`FUN_00602d00`, 5 bytes, `decompiled_uncited`): `xor eax,
/// eax; ret` - the 64-bit zero returned by the empty virtual slot.
pub fn a56_zero_result() -> u64 {
    0
}

/// `0x00602cd0` (`FUN_00602cd0`, 34 bytes, `decompiled_uncited`): state-flag
/// merge.
///
/// ```c
/// state = *(this + 0x0c) | incoming;
/// if (*(this + 0x38) == 0) state |= 4;   /* badbit */
/// FUN_005fdee0(state, param_2);
/// ```
///
/// The cached pointer at `+0x38` suppresses the extra `4`; the trailing call
/// is out-of-window and only receives the merged value, which is returned.
pub fn a56_set_state(existing: u32, incoming: u32, cached_pointer: u32) -> u32 {
    let merged = existing | incoming;
    if cached_pointer == 0 {
        merged | 4
    } else {
        merged
    }
}

/// A buffered reader with the two pointer fields used by `0x00602d10` and
/// `0x00603200`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A56Reader {
    pub buf: Vec<u8>,
    /// `*(this + 0x1c)`, the current get pointer; 0 means null.
    pub next: u32,
    /// `*(this + 0x2c)`, the get-area byte count.
    pub count: i32,
}

/// Bounds-checked byte load used by the stream helpers; retail trusts its
/// pointers, the translation reports an out-of-range read as `-1`.
fn a56_byte_at(buf: &[u8], pos: u32) -> i32 {
    buf.get(pos as usize).map_or(-1, |b| i32::from(*b))
}

/// `0x00602d10` (`FUN_00602d10`, 100 bytes, `decompiled_uncited`):
/// `basic_streambuf::sbumpc` with the recovered two-step count handling.
///
/// * null get pointer: call virtual `+0x1c`;
/// * count > 1: decrement count, advance the pointer, return the byte at the
///   new pointer;
/// * count == 1: clear the count, advance the pointer, remember the byte at
///   the old pointer;
/// * count < 1: call virtual `+0x1c`.
///
/// A remembered/underflow byte of `0xffffffff` returns EOF.  Otherwise, when
/// the pointer is null or the count is below 1, virtual `+0x18` is called;
/// else the byte at the pointer is returned.
pub fn a56_sbumpc<U, G>(r: &mut A56Reader, mut underflow: U, mut getc: G) -> i32
where
    U: FnMut(&mut A56Reader) -> i32,
    G: FnMut(&mut A56Reader) -> i32,
{
    let value;
    if r.next == 0 {
        value = underflow(r);
    } else {
        let count = r.count;
        if count > 1 {
            r.count = count - 1;
            r.next = r.next.wrapping_add(1);
            return a56_byte_at(&r.buf, r.next);
        }
        if count < 1 {
            value = underflow(r);
        } else {
            r.count = 0;
            let old = r.next;
            r.next = old.wrapping_add(1);
            value = a56_byte_at(&r.buf, old);
        }
    }

    if value == -1 {
        return -1;
    }
    if r.next == 0 || r.count < 1 {
        return getc(r);
    }
    a56_byte_at(&r.buf, r.next)
}

/// `0x00603200` (`FUN_00603200`, 37 bytes, `decompiled_uncited`): advance the
/// get pointer after a successful virtual `+0x18` call.
///
/// The virtual slot is called first; `-1` returns `-1` unchanged.  Otherwise
/// `+0x2c` is decremented, `+0x1c` advances, and the byte at the old pointer
/// is returned.
pub fn a56_advance_get<F>(r: &mut A56Reader, mut underflow: F) -> i32
where
    F: FnMut(&mut A56Reader) -> i32,
{
    if underflow(r) == -1 {
        return -1;
    }
    r.count = r.count.wrapping_sub(1);
    let old = r.next;
    r.next = old.wrapping_add(1);
    a56_byte_at(&r.buf, old)
}

/// `0x00603230` (`FUN_00603230`, 4 bytes, `decompiled_uncited`): `or eax,
/// 0xffffffff; ret`, the EOF value.
pub fn a56_eof() -> i32 {
    -1
}

/// A get area for `0x00603240`: the put pointer/count plus the null-tested
/// end pointer, cached high-water and flags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A56GetArea {
    pub buf: Vec<u8>,
    /// `*(this + 0x1c)`.
    pub next: u32,
    /// `*(this + 0x2c)`.
    pub count: i32,
    /// `*(this + 0x20)`, null when 0.
    pub end: u32,
    /// `*(this + 0x38)` cached high-water.
    pub high_water: u32,
    /// `*(this + 0x3c)`; bit 2 disables the high-water extension.
    pub flags: u8,
}

/// `0x00603240` (`FUN_00603240`, 90 bytes, `decompiled_uncited`):
/// `basic_streambuf::sgetc`.
///
/// With a non-null pointer and positive count the current byte is returned.
/// Otherwise, when the end pointer is non-null and bit 2 of `+0x3c` is clear,
/// the high-water mark is raised to the end pointer and, if the get pointer
/// is below the raised mark, the count is reset to `mark - pointer` and the
/// byte is returned.  Every other case returns `-1`.
pub fn a56_sgetc(a: &mut A56GetArea) -> i32 {
    if a.next != 0 {
        if a.count > 0 {
            return a56_byte_at(&a.buf, a.next);
        }
        if a.end != 0 && (a.flags & 4) == 0 {
            let mut mark = a.high_water;
            if mark < a.end {
                mark = a.end;
            }
            if a.next < mark {
                a.high_water = mark;
                a.count = mark.wrapping_sub(a.next) as i32;
                return a56_byte_at(&a.buf, a.next);
            }
        }
    }
    -1
}

/// `0x006032a0` (`FUN_006032a0`, 174 bytes, `decompiled_uncited`):
/// `basic_streambuf::xsgetn`.
///
/// The 64-bit request is the `(param_2, param_3)` pair; a negative high word
/// reads nothing.  Per iteration the available count is
/// `FUN_005fc650()` (the get-area length).  A positive count copies
/// `min(remaining, available)` bytes; otherwise the virtual `+0x1c` slot is
/// called, `-1` stops the loop and any other byte is appended.  The return is
/// the number of bytes actually produced.
///
/// `FUN_005fc650` and the virtual underflow are outside the window; the
/// translation derives the available count from the get area (`end - next`)
/// and takes the underflow as a closure that receives the same area.  The
/// bulk path also decrements the `+0x2c` byte count by the copied amount,
/// exactly as the instruction stream at `0x0060330c` does.
pub fn a56_sgetn<U>(area: &mut A56GetArea, count: u64, mut underflow: U) -> (Vec<u8>, u64)
where
    U: FnMut(&mut A56GetArea) -> i32,
{
    let mut out = Vec::new();
    if (count as i64) < 0 {
        return (out, 0);
    }
    let mut remaining = count;
    while remaining != 0 {
        let available = area.end.wrapping_sub(area.next) as i64;
        if available > 0 {
            let take = remaining.min(available as u64) as usize;
            let start = (area.next as usize).min(area.buf.len());
            let end = (start + take).min(area.buf.len());
            out.extend_from_slice(&area.buf[start..end]);
            area.next = area.next.wrapping_add(take as u32);
            area.count = area.count.wrapping_sub(take as i32);
            remaining -= take as u64;
            continue;
        }
        let byte = underflow(area);
        if byte == -1 {
            break;
        }
        out.push(byte as u8);
        remaining -= 1;
    }
    let produced = count - remaining;
    (out, produced)
}

/// An output area for `0x00603370`: the put pointer and its end plus the
/// backing bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A56OutputArea {
    pub buf: Vec<u8>,
    /// `*(this + 0x20)`, the put pointer.
    pub next: usize,
    /// End of the put area; `next..end` is the writable window.
    pub end: usize,
}

/// `0x00603370` (`FUN_00603370`, 171 bytes, `decompiled_uncited`):
/// `basic_streambuf::xsputn`.
///
/// Mirrors `0x006032a0` on the output side: `FUN_005fd060` supplies the free
/// room in the put area, a positive room copies `min(remaining, room)` bytes
/// from the caller's block, and otherwise virtual `+0x0c` (`sputc`) is asked
/// to accept the next byte; `-1` stops the loop.  The produced byte count is
/// returned.  The translation derives the room as `end - next` and takes
/// `sputc` as a closure.
pub fn a56_sputn<S>(area: &mut A56OutputArea, src: &[u8], count: u64, mut sputc: S) -> u64
where
    S: FnMut(&mut A56OutputArea, u8) -> i32,
{
    if (count as i64) < 0 {
        return 0;
    }
    let mut remaining = count;
    let mut consumed = 0usize;
    while remaining != 0 && consumed < src.len() {
        let room = area.end.saturating_sub(area.next);
        if room > 0 {
            let take = remaining.min(room as u64) as usize;
            let take = take.min(src.len() - consumed);
            if area.next + take <= area.buf.len() {
                area.buf[area.next..area.next + take]
                    .copy_from_slice(&src[consumed..consumed + take]);
            }
            area.next += take;
            consumed += take;
            remaining -= take as u64;
            continue;
        }
        let byte = src[consumed];
        if sputc(area, byte) == -1 {
            break;
        }
        consumed += 1;
        remaining -= 1;
    }
    count - remaining
}

/// Fields read by `0x00602d80` when it materializes a pending area.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A56PendingAreas {
    pub data: Vec<u8>,
    /// `*(this + 0x40)`; bit 2 disables the output branch, bit 4 the input
    /// branch.
    pub flags: u8,
    /// `*(this + 0x24)`, null when 0.
    pub out_end: u32,
    /// `*(this + 0x14)`.
    pub out_first: u32,
    /// `(this + 0x3c)`.
    pub limit: u32,
    /// `*(this + 0x20)`, null when 0.
    pub in_end: u32,
    /// `*(this + 0x10)`.
    pub in_first: u32,
    /// `*(this + 0x30)`.
    pub in_count: i32,
}

/// `0x00602d80` (`FUN_00602d80`, 180 bytes, `decompiled_uncited`): build a
/// string from the pending output or input area.
///
/// The output branch is taken when bit 2 of `+0x40` is clear and `*(+0x24)` is
/// non-null; its end is raised to at least the `+0x3c` limit and the length is
/// `end - *(+0x14)`.  The input branch (bit 4 clear and `*(+0x20)`
/// non-null) uses `(*(+0x30) - *(+0x10)) + *(+0x20)`.  An empty result is
/// returned when neither branch is available.  The body then assigns the
/// range through `FUN_005fdb20`; the translation returns the bytes directly.
pub fn a56_pending_area(a: &A56PendingAreas) -> Vec<u8> {
    if (a.flags & 2) == 0 && a.out_end != 0 {
        let end = a.out_end.max(a.limit);
        let start = a.out_first;
        let len = end.wrapping_sub(start);
        return a56_slice_clamped(&a.data, start, len);
    }
    if (a.flags & 4) != 0 || a.in_end == 0 {
        return Vec::new();
    }
    let start = a.in_first;
    let len = a
        .in_count
        .wrapping_sub(start as i32)
        .wrapping_add(a.in_end as i32) as u32;
    a56_slice_clamped(&a.data, start, len)
}

/// Bounds-checked range read; retail trusts the recovered offsets, the
/// translation clamps them to the backing bytes.
fn a56_slice_clamped(data: &[u8], start: u32, len: u32) -> Vec<u8> {
    let start = (start as usize).min(data.len());
    let end = start.saturating_add(len as usize).min(data.len());
    data[start..end].to_vec()
}

/// `0x00603440` (`FUN_00603440`, 6 bytes, `decompiled_uncited`): return
/// `&DAT_00fa3930`, the static object compared by `0x00603450`.
pub fn a56_static_object_address() -> usize {
    A56_STATIC_OBJECT
}

/// Compare-order clamp of `0x00603450`:
/// `if (order < 0) order = -1; return order;`.
pub fn a56_compare_order(order: i32) -> i32 {
    if order < 0 { -1 } else { order }
}

/// Byte-wise comparison in the shape of the CRT comparator called at
/// `0x00603450` (`FUN_00c9b8a9`): the first differing byte decides; a proper
/// prefix compares shorter-first.  The result is passed through
/// [`a56_compare_order`], so every negative order becomes `-1`.
pub fn a56_compare_bytes(a: &[u8], b: &[u8]) -> i32 {
    for (left, right) in a.iter().zip(b.iter()) {
        if left != right {
            return a56_compare_order(i32::from(*left) - i32::from(*right));
        }
    }
    a56_compare_order((a.len() as i32) - (b.len() as i32))
}

/// `0x00603450` (`FUN_00603450`, 46 bytes, `decompiled_uncited`): compare the
/// first two dwords of the static object returned by `0x00603440`, then clamp
/// a negative order to `-1`.
///
/// ```c
/// p = FUN_00603440();
/// order = FUN_00c9b8a9(p[0], p[1]);
/// if (order < 0) order = -1;
/// return order;
/// ```
///
/// The comparator is outside the window; the translation takes the two byte
/// sequences and applies the same clamp through [`a56_compare_bytes`].
pub fn a56_compare_static_object(first: &[u8], second: &[u8]) -> i32 {
    a56_compare_bytes(first, second)
}

/// `0x00603640` (`FUN_00603640`, 68 bytes, `decompiled_uncited`): extract one
/// `{start, end}` entry from the 8-byte table at `+0x1c` of a record.
///
/// ```c
/// if (index < count) {
///     start = table[index].start;
///     if (start != -1) {
///         len = table[index].end - start;
///         if (total - start <= len) len = total - start;
///         assign(data + start, len);
///         return;
///     }
/// }
/// assign("", 0);
/// ```
///
/// `total` is the record's `+0x04` size.  The append is performed through
/// `FUN_006036b0`; the translation returns the selected bytes.  Retail trusts
/// the table offsets; the slice is clamped to the backing bytes.
pub fn a56_table_slice(data: &[u8], entries: &[(u32, u32)], total: u32, index: usize) -> Vec<u8> {
    let Some(&(start, end)) = entries.get(index) else {
        return Vec::new();
    };
    if start == u32::MAX {
        return Vec::new();
    }
    let mut len = end.wrapping_sub(start);
    let available = total.wrapping_sub(start);
    if available <= len {
        len = available;
    }
    a56_slice_clamped(data, start, len)
}

/// The four globals zeroed by `0x00603ee0`, named by their retail addresses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A56Globals {
    /// `DAT_00fa3964` (dword).
    pub at_fa3964: u32,
    /// `DAT_00fa3960` (dword).
    pub at_fa3960: u32,
    /// `DAT_011cb51c` (byte store at `0x00603ef4`).
    pub at_11cb51c: u8,
    /// `_DAT_011cb52c` (dword).
    pub at_11cb52c: u32,
}

/// `0x00603ee0` (`FUN_00603ee0`, 38 bytes, `decompiled_uncited`): clear
/// `DAT_00fa3964`, `DAT_00fa3960`, `DAT_011cb51c` (one byte) and
/// `DAT_011cb52c` to zero.
pub fn a56_reset_globals() -> A56Globals {
    A56Globals::default()
}

/// `0x00603f10` (`FUN_00603f10`, 25 bytes, `decompiled_uncited`): probe the
/// handle at `0x00fa3964`.
///
/// ```c
/// if (DAT_00fa3964 != 0) return FUN_00964630(DAT_00fa3964);
/// return 0;
/// ```
///
/// The probe service is outside the window; the translation takes it as a
/// closure and only calls it for a non-zero handle.
pub fn a56_probe<P>(handle: u32, mut probe: P) -> u8
where
    P: FnMut(u32) -> u8,
{
    if handle != 0 { probe(handle) } else { 0 }
}

/// `0x00603ff0` (`FUN_00603ff0`, 106 bytes, `decompiled_uncited`): integer
/// distance estimate.
///
/// ```c
/// a = |param_2 - param_1|;      /* (d ^ (d >> 31)) - (d >> 31) */
/// b = |param_4 - param_3|;
/// min = min(a, b); max = max(a, b);
/// c = min + min / 2;
/// estimate = max + max / 32 + c / 128;
/// if (max / 4 < min)
///     estimate += (c / 32 - max / 128 + c / 4) - max / 32;
/// return estimate;
/// ```
///
/// All operations are 32-bit wrapping, matching the recovered `int` body.
pub fn a56_distance_estimate(param_1: i32, param_2: i32, param_3: i32, param_4: i32) -> i32 {
    let ab = param_2.wrapping_sub(param_1);
    let ab_abs = (ab ^ (ab >> 31)).wrapping_sub(ab >> 31);
    let cd = param_4.wrapping_sub(param_3);
    let cd_abs = (cd ^ (cd >> 31)).wrapping_sub(cd >> 31);

    let (min, max) = if cd_abs <= ab_abs {
        (cd_abs, ab_abs)
    } else {
        (ab_abs, cd_abs)
    };
    let mid = (min >> 1).wrapping_add(min);
    let mut estimate = (mid >> 7).wrapping_add(max >> 5).wrapping_add(max);
    if max >> 2 < min {
        estimate = estimate
            .wrapping_add((mid >> 5).wrapping_sub(max >> 7))
            .wrapping_add(mid >> 2)
            .wrapping_sub(max >> 5);
    }
    estimate
}

#[cfg(test)]
mod tests_agent56 {
    use super::*;

    fn s(text: &[u8]) -> A56String {
        a56_string_from_bytes(text)
    }

    fn small(text: &[u8]) -> A56SmallString {
        a56_small_assign(text)
    }

    fn seek_state() -> A56SeekState {
        A56SeekState {
            put_next: Some(0x108),
            put_end: Some(0x110),
            get_next: 0x100,
            high_water: 0x10c,
            put_count: 8,
            get_first: 0x100,
            get_count: 0x10,
        }
    }

    /// `0x006024d0`: the fast path writes below the capacity; a full string
    /// grows to `max((size + 1) | 0xf, cap + cap / 2)`.
    #[test]
    fn push_back_follows_the_growth_policy() {
        let mut text = A56String {
            bytes: b"abc".to_vec(),
            cap: 4,
        };
        assert!(a56_string_push_back(&mut text, b'd'));
        assert_eq!(text.bytes, b"abcd");
        assert_eq!(text.cap, 4, "3 < 4 writes in place without growing");

        let mut text = A56String {
            bytes: vec![b'x'; 15],
            cap: A56_SSO_CAPACITY,
        };
        assert!(a56_string_push_back(&mut text, b'y'));
        assert_eq!(text.cap, 0x1f, "(15 + 1) | 0xf = 0x1f beats 0xf + 7");
        assert_eq!(text.bytes.len(), 16);

        assert_eq!(a56_grow_capacity(0x8000_0000, 0), A56_STRING_MAX);
        assert_eq!(a56_grow_capacity(0x20, 0x7fff_fffe), A56_STRING_MAX);
    }

    /// `0x00602790`: reserve ignores requests below the size, grows with the
    /// shared policy, and reports the below-inline shrink call.
    #[test]
    fn reserve_matches_the_recovered_branches() {
        assert_eq!(a56_string_reserve(5, 0x20, 4), Ok(A56Reserve::Unchanged));
        assert_eq!(a56_string_reserve(5, 0x20, 0x20), Ok(A56Reserve::Unchanged));
        assert_eq!(
            a56_string_reserve(5, 0x20, 0x40),
            Ok(A56Reserve::Grew(0x4f)),
            "0x40 | 0xf = 0x4f beats 0x20 + 0x10 = 0x30"
        );
        assert_eq!(
            a56_string_reserve(16, 0x10, 0x20),
            Ok(A56Reserve::Grew(0x2f))
        );
        assert_eq!(
            a56_string_reserve(2, 0x20, 0x8),
            Ok(A56Reserve::ShrankInline)
        );
        assert_eq!(a56_string_reserve(2, 0xf, 0x8), Ok(A56Reserve::Unchanged));
        assert_eq!(a56_string_reserve(0, 0, u32::MAX), Err(()));
        assert_eq!(
            a56_string_reserve(0, 0, A56_STRING_MAX),
            Ok(A56Reserve::Grew(A56_STRING_MAX))
        );
    }

    /// `0x00603150`: `substr` clamps the length at `size - pos` and reports
    /// the `pos > size` throw as `None`.
    #[test]
    fn substr_clamps_length_and_rejects_bad_positions() {
        let text = s(b"abcdef");
        assert_eq!(a56_string_substr(&text, 2, 3).unwrap().bytes, b"cde");
        assert_eq!(a56_string_substr(&text, 4, 99).unwrap().bytes, b"ef");
        assert_eq!(a56_string_substr(&text, 6, 1).unwrap().bytes, b"");
        assert!(a56_string_substr(&text, 7, 1).is_none());
        assert_eq!(
            a56_string_substr(&text, 1, 5).unwrap().cap,
            A56_SSO_CAPACITY
        );
    }

    /// `0x00602470`: the putback accepts a matching byte (or any byte when
    /// strict bit 2 is clear) and rewinds the get pointer.
    #[test]
    fn putback_obeys_the_strict_bit() {
        let mut area = A56Putback {
            buf: b"abcd".to_vec(),
            gptr: 2,
            putback_end: 4,
            put_count: 0,
            flags: 0,
        };
        assert_eq!(a56_putback(&mut area, i32::from(b'd')), i32::from(b'd'));
        assert_eq!((area.gptr, area.put_count), (1, 1));
        assert_eq!(area.buf, b"adcd");

        let mut strict = A56Putback {
            buf: b"abcd".to_vec(),
            gptr: 2,
            putback_end: 4,
            put_count: 0,
            flags: 2,
        };
        assert_eq!(a56_putback(&mut strict, i32::from(b'z')), -1);
        assert_eq!(a56_putback(&mut strict, -1), 0);
        assert_eq!((strict.gptr, strict.put_count), (1, 1));

        let mut null_end = A56Putback {
            buf: Vec::new(),
            gptr: 0,
            putback_end: 0,
            put_count: 0,
            flags: 0,
        };
        assert_eq!(a56_putback(&mut null_end, -1), -1);
    }

    /// `0x006022c0`: every 0x1c record is converted once, in order, before
    /// the sink receives the pointer array.
    #[test]
    fn record_strings_convert_in_order() {
        let records = [small(b"one"), small(b"two")];
        let wide = a56_convert_record_strings(&records, |bytes| {
            bytes.iter().map(|b| u16::from(*b)).collect()
        });
        assert_eq!(wide, vec![vec![111, 110, 101], vec![116, 119, 111]]);
        assert_eq!(A56_SMALL_STRIDE, 0x1c);
    }

    /// `0x006028b0` / `0x006036b0`: the capacity-field merge preserves the
    /// 0x80000000 inline flag, grows by 1.5x and clamps to the request.
    #[test]
    fn flagged_capacity_keeps_the_inline_flag() {
        assert_eq!(a56_small_capacity(A56_SMALL_INLINE), 0xf);
        assert_eq!(
            a56_capacity_field_with_flag(A56_SMALL_INLINE, A56_SMALL_FIRST_GROWTH),
            0x8000_0016,
            "the storage tag survives the capacity merge"
        );
        assert_eq!(
            a56_vector_growth_field(A56_SMALL_INLINE, 0x20),
            Some(0x8000_0020)
        );
        assert_eq!(
            a56_vector_growth_field(A56_SMALL_INLINE, 0x10),
            Some(0x8000_0016),
            "0xf + 7 = 22 is the first growth"
        );
        assert_eq!(a56_vector_growth_field(A56_SMALL_INLINE, 0xf), None);

        let assigned = a56_small_assign(b"0123456789abcdef");
        assert_eq!(assigned.cap_field, 0x8000_0016);
        assert_eq!(assigned.bytes, b"0123456789abcdef");
        assert_eq!(small(b"short").cap_field, A56_SMALL_INLINE);
    }

    /// `0x006025e0`: the move insert steals unowned storage and deep-copies a
    /// flagged element; the vector capacity grows by one slot when full.
    #[test]
    fn push_move_steals_only_unflagged_storage() {
        let mut vec = vec![small(b"abc")];
        let mut argument = small(b"arg");
        let (field, moved) =
            a56_small_vector_push_move(&mut vec, A56_SMALL_INLINE, 0, &mut argument);
        assert!(!moved, "a flagged element takes the deep-copy branch");
        assert_eq!(field, A56_SMALL_INLINE);
        assert_eq!(vec[0].bytes, b"abc");
        assert_eq!(vec[1].bytes, b"abc");
        assert_eq!(argument.bytes, b"arg");

        let mut vec = vec![small(b"x")];
        let mut heap = A56SmallString {
            bytes: b"heap".to_vec(),
            cap_field: 0x20,
        };
        let (field, moved) = a56_small_vector_push_move(&mut vec, 0x8000_0001, 1, &mut heap);
        assert!(moved, "the clear high bit selects the steal branch");
        assert_eq!(field, 0x8000_0002);
        assert_eq!(vec[1].bytes, b"heap");
        assert_eq!(vec[1].cap_field, 0x20);
        assert_eq!(heap, a56_small_empty(), "the stolen source is reset inline");
    }

    /// `0x00602670`: the element index is the 32-bit `/0x1c` divide; a full
    /// vector grows by one slot and the appended element is a copy of the
    /// addressed slot (or of the argument when the index is out of range).
    #[test]
    fn emplace_uses_the_1c_stride_and_copies_the_source() {
        assert_eq!(a56_index_1c(0x1000 + 0x1c * 3, 0x1000), 3);
        let mut vec = vec![small(b"abc")];
        let (field, appended) =
            a56_small_vector_emplace(&mut vec, A56_SMALL_INLINE, 0, &small(b"source"));
        assert_eq!(appended.bytes, b"abc");
        assert_eq!(field, A56_SMALL_INLINE, "one of fifteen slots is not full");
        assert_eq!(vec.last(), Some(&small(b"abc")));

        let mut vec = vec![small(b"abc")];
        let (field, appended) =
            a56_small_vector_emplace(&mut vec, 0x8000_0001, 1, &small(b"source"));
        assert_eq!(field, 0x8000_0002);
        assert_eq!(appended.bytes, b"source");
        assert_eq!(vec.len(), 2);
    }

    /// `0x00602730`: the in-capacity path moves the string and resets the
    /// source to the empty inline state; the full path is reported as false.
    #[test]
    fn push_string_move_resets_the_source() {
        let mut vec = vec![s(b"a")];
        let mut value = s(b"b");
        assert!(a56_push_string_move(&mut vec, &mut value, true));
        assert_eq!(vec[1], s(b"b"));
        assert_eq!(value, a56_string_empty());
        assert!(!a56_push_string_move(&mut vec, &mut value, false));
        assert_eq!(vec.len(), 2);
        assert_eq!(A56_STD_STRING_STRIDE, 0x18);
    }

    /// `0x00602a10` and `0x00602ba0`: the 0x18-byte result slot starts as the
    /// invalid 64-bit position with a zero tail.
    #[test]
    fn seek_result_initializers_are_invalid() {
        let expected = A56SeekResult {
            lo: u32::MAX,
            hi: u32::MAX,
            tail: [0; 4],
        };
        assert_eq!(a56_seek_result_invalid_2a10(), expected);
        assert_eq!(a56_seek_result_invalid_2ba0(), expected);
        assert_eq!(a56_seek_result(4, 0).tail, [0; 4]);
    }

    /// `0x00602a40`: way 0 seeks from the get pointer and mode 3 updates both
    /// the put count and the get window.
    #[test]
    fn seekoff_from_begin_updates_both_areas() {
        let mut state = seek_state();
        let result = a56_seekoff(&mut state, 4, 0, 0, 3);
        assert_eq!(result, a56_seek_result(4, 0));
        assert_eq!(state.put_next, Some(0x104));
        assert_eq!(state.put_count, 0x110 - 0x104);
        assert_eq!(state.get_first, 0x100);
        assert_eq!(state.put_end, Some(0x104));
        assert_eq!(state.get_count, (0x10 + 0x110) - 0x104);
    }

    /// `0x00602a40`: way 1 starts at the put pointer (mode 1) and way 2 starts
    /// at the sign-extended available count.
    #[test]
    fn seekoff_from_put_and_end() {
        let mut state = seek_state();
        let result = a56_seekoff(&mut state, 1, 0, 1, 1);
        assert_eq!(result, a56_seek_result(9, 0));
        assert_eq!(state.put_next, Some(0x109));
        assert_eq!(state.put_count, 0x110 - 0x109);

        let mut state = seek_state();
        let result = a56_seekoff(&mut state, 0, 0, 2, 0);
        assert_eq!(result, a56_seek_result(0x10, 0));
        assert!(a56_seekoff(&mut state, 0, 1, 0, 0).hi == u32::MAX);
    }

    /// `0x00602bd0`: the explicit two-word base is added with carry, then the
    /// same bounds check and mode gates apply.
    #[test]
    fn seek_base_adds_with_carry() {
        let mut state = seek_state();
        let result = a56_seek_base(&mut state, 4, 0, 2, 0, 2);
        assert_eq!(result, a56_seek_result(6, 0));
        assert_eq!(state.put_end, Some(0x106));
        assert_eq!(state.get_count, (0x10 + 0x110) - 0x106);

        let mut state = seek_state();
        let result = a56_seek_base(&mut state, 0xffff_fffe, 0, 4, 0, 2);
        assert_eq!(result, a56_seek_result_invalid_2a10());
        assert_eq!(state.put_end, Some(0x110), "invalid seeks do not commit");
        assert_eq!(state.get_count, 0x10);
    }

    /// `0x00602cc0` / `0x00602d00`: the empty slot and the zero-returning slot.
    #[test]
    fn noop_and_zero_result() {
        a56_noop();
        assert_eq!(a56_zero_result(), 0u64);
        assert_eq!(a56_eof(), -1);
    }

    /// `0x00602cd0`: a zero cached pointer adds the badbit `4`.
    #[test]
    fn state_merge_sets_badbit_only_without_a_cache() {
        assert_eq!(a56_set_state(0, 1, 0x8000), 1);
        assert_eq!(a56_set_state(0, 1, 0), 5);
        assert_eq!(a56_set_state(1, 2, 0), 7);
    }

    /// `0x00602d10`: count > 1 advances and returns the new byte; count == 1
    /// clears the count and asks virtual `+0x18`; a null pointer underflows.
    #[test]
    fn sbumpc_handles_the_three_count_branches() {
        let mut reader = A56Reader {
            buf: b"abcd".to_vec(),
            next: 1,
            count: 3,
        };
        assert_eq!(a56_sbumpc(&mut reader, |_| -1, |_| 0), i32::from(b'c'));
        assert_eq!((reader.next, reader.count), (2, 2));

        let mut reader = A56Reader {
            buf: b"abcd".to_vec(),
            next: 1,
            count: 1,
        };
        let got = a56_sbumpc(&mut reader, |_| -1, |r| a56_byte_at(&r.buf, r.next));
        assert_eq!(got, i32::from(b'c'));
        assert_eq!((reader.next, reader.count), (2, 0));

        let mut reader = A56Reader {
            buf: b"abcd".to_vec(),
            next: 0,
            count: 4,
        };
        let got = a56_sbumpc(&mut reader, |_| 65, |_| 66);
        assert_eq!(got, 66, "a null pointer defers to virtual +0x18");

        let mut reader = A56Reader {
            buf: b"abcd".to_vec(),
            next: 1,
            count: 0,
        };
        assert_eq!(a56_sbumpc(&mut reader, |_| -1, |_| 9), -1);
    }

    /// `0x00602d80`: the output area wins when bit 2 is clear and its end is
    /// non-null; the end is raised to at least the 0x3c limit.
    #[test]
    fn pending_area_prefers_the_output_window() {
        let area = A56PendingAreas {
            data: b"0123456789".to_vec(),
            flags: 0,
            out_end: 6,
            out_first: 2,
            limit: 4,
            in_end: 5,
            in_first: 1,
            in_count: 2,
        };
        assert_eq!(a56_pending_area(&area), b"2345");

        let area = A56PendingAreas { out_end: 0, ..area };
        assert_eq!(a56_pending_area(&area), b"123456");

        let area = A56PendingAreas { flags: 4, ..area };
        assert!(a56_pending_area(&area).is_empty());
        assert!(
            a56_pending_area(&A56PendingAreas {
                data: Vec::new(),
                flags: 0,
                out_end: 0,
                out_first: 0,
                limit: 0,
                in_end: 0,
                in_first: 0,
                in_count: 0,
            })
            .is_empty()
        );
    }

    /// `0x00603200`: the virtual call runs first; success decrements the
    /// count, advances the pointer and returns the old byte.
    #[test]
    fn advance_get_returns_the_old_byte() {
        let mut reader = A56Reader {
            buf: b"xyz".to_vec(),
            next: 1,
            count: 4,
        };
        let got = a56_advance_get(&mut reader, |_| 7);
        assert_eq!(got, i32::from(b'y'));
        assert_eq!((reader.next, reader.count), (2, 3));
        assert_eq!(a56_advance_get(&mut reader, |_| -1), -1);
        assert_eq!((reader.next, reader.count), (2, 3));
    }

    /// `0x00603240`: a positive count serves the current byte; otherwise the
    /// high-water mark extends to the end pointer unless bit 2 is set.
    #[test]
    fn sgetc_extends_the_high_water_mark() {
        let mut area = A56GetArea {
            buf: b"abcdef".to_vec(),
            next: 2,
            count: 2,
            end: 6,
            high_water: 4,
            flags: 0,
        };
        assert_eq!(a56_sgetc(&mut area), i32::from(b'c'));
        area.count = 0;
        assert_eq!(a56_sgetc(&mut area), i32::from(b'c'));
        assert_eq!(area.high_water, 6);
        assert_eq!(area.count, 4);

        let mut eof_area = A56GetArea {
            buf: b"abcdef".to_vec(),
            next: 2,
            count: 0,
            end: 6,
            high_water: 0,
            flags: 4,
        };
        assert_eq!(a56_sgetc(&mut eof_area), -1);
        eof_area.flags = 0;
        eof_area.end = 0;
        assert_eq!(a56_sgetc(&mut eof_area), -1);
        eof_area.next = 0;
        assert_eq!(a56_sgetc(&mut eof_area), -1);
    }

    /// `0x006032a0`: bulk bytes come from `end - next`, then the underflow
    /// closure supplies single bytes; the return counts what was produced.
    #[test]
    fn sgetn_copies_the_window_then_underflows() {
        let mut area = A56GetArea {
            buf: b"abcdef".to_vec(),
            next: 2,
            count: 3,
            end: 5,
            high_water: 0,
            flags: 0,
        };
        let (out, produced) = a56_sgetn(&mut area, 10, |_| -1);
        assert_eq!(out, b"cde");
        assert_eq!(produced, 3);
        assert_eq!(area.count, 0, "the bulk copy decrements the +0x2c count");

        let mut area = A56GetArea {
            buf: b"abcdef".to_vec(),
            next: 5,
            count: 0,
            end: 5,
            high_water: 0,
            flags: 0,
        };
        let (out, produced) = a56_sgetn(&mut area, 2, |a| {
            a.end = a.next + 1;
            a.next += 1;
            i32::from(b'Z')
        });
        assert_eq!(out, b"ZZ");
        assert_eq!(produced, 2);

        let mut area = A56GetArea {
            buf: Vec::new(),
            next: 0,
            count: 0,
            end: 0,
            high_water: 0,
            flags: 0,
        };
        assert_eq!(a56_sgetn(&mut area, u64::MAX, |_| 1).1, 0);
    }

    /// `0x00603370`: the writable window receives the source bytes, and the
    /// virtual `sputc` closure handles the overflow byte.
    #[test]
    fn sputn_fills_the_window_then_calls_sputc() {
        let mut area = A56OutputArea {
            buf: vec![0; 8],
            next: 1,
            end: 4,
        };
        let produced = a56_sputn(&mut area, b"WXYZ", 3, |_, _| -1);
        assert_eq!(produced, 3);
        assert_eq!(&area.buf[1..4], b"WXY");
        assert_eq!(area.next, 4);

        let mut area = A56OutputArea {
            buf: Vec::new(),
            next: 0,
            end: 0,
        };
        let produced = a56_sputn(&mut area, b"Q", 1, |a, byte| {
            a.buf.push(byte);
            i32::from(byte)
        });
        assert_eq!(produced, 1);
        assert_eq!(area.buf, b"Q");
    }

    /// `0x00603440` / `0x00603450`: the static address and the clamped
    /// compare order (negative orders collapse to -1).
    #[test]
    fn static_object_compare_clamps_negative_orders() {
        assert_eq!(a56_static_object_address(), 0x00fa_3930);
        assert_eq!(a56_compare_static_object(b"a", b"b"), -1);
        assert_eq!(a56_compare_static_object(b"b", b"a"), 1);
        assert_eq!(a56_compare_static_object(b"ab", b"a"), 1);
        assert_eq!(a56_compare_static_object(b"same", b"same"), 0);
        assert_eq!(a56_compare_order(-9), -1);
        assert_eq!(a56_compare_order(9), 9);
    }

    /// `0x00603640`: an entry at or beyond the table end gives the empty
    /// string, a `-1` start is the tombstone, and the length clamps to the
    /// record's 0x04 total.
    #[test]
    fn table_slice_clamps_to_total_and_skips_tombstones() {
        let data = b"0123456789";
        let entries = [(2, 5), (u32::MAX, 9), (7, 20), (1, 3)];
        assert_eq!(a56_table_slice(data, &entries, 10, 0), b"234");
        assert!(a56_table_slice(data, &entries, 10, 1).is_empty());
        assert_eq!(a56_table_slice(data, &entries, 10, 2), b"789");
        assert_eq!(a56_table_slice(data, &entries, 10, 4), b"");
        assert_eq!(a56_table_slice(data, &entries, 8, 2), b"7");
    }

    /// `0x00603ee0` / `0x00603f10`: four globals clear to zero; the probe is
    /// skipped for a zero handle and called otherwise.
    #[test]
    fn globals_reset_and_probe_are_gated_by_the_handle() {
        assert_eq!(a56_reset_globals(), A56Globals::default());
        let mut calls = 0;
        let answer = a56_probe(0, |_| {
            calls += 1;
            1
        });
        assert_eq!((answer, calls), (0, 0));
        let answer = a56_probe(0x00fa_3964, |handle| {
            calls += 1;
            (handle & 0xff) as u8
        });
        assert_eq!((answer, calls), (0x64, 1));
    }

    /// `0x00603ff0`: the recovered abs/min/max sequence and the two scaled
    /// terms.
    #[test]
    fn distance_estimate_matches_the_integer_body() {
        assert_eq!(a56_distance_estimate(0, 3, 0, 4), 5);
        assert_eq!(a56_distance_estimate(0, 10, 0, 10), 13);
        assert_eq!(a56_distance_estimate(0, 100, 0, 4), 103);
        assert_eq!(a56_distance_estimate(5, 5, 7, 7), 0);
        assert_eq!(a56_distance_estimate(4, 4, 0, 2), 2);
    }
}
