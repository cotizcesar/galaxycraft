#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent55.rs
// ===========================================================================
// Translation batch 55.  Only this file is edited by the batch.
//
// Batch 55 owns `0x00600000..=0x00601fff`.  The ledger lists **11** rows in
// that window, all `decompiled_uncited`, and all 11 are translated below:
//
// | address      | recovered role                                             |
// |--------------|------------------------------------------------------------|
// | `0x006000c0` | MSVC `std::string::insert(pos, const char*, count)`         |
// | `0x006002a0` | `error_condition` construction through the iostream singleton |
// | `0x006002e0` | generic error-category `message` (`_Syserror_map` payload)  |
// | `0x00600330` | iostream error-category `message`                           |
// | `0x006003b0` | generic error-category `name` (`"generic"`)                 |
// | `0x006003c0` | iostream error-category `name` (`"iostream"`)               |
// | `0x006003d0` | six-byte `return 0xffffffff` virtual slot                   |
// | `0x006003e0` | growing streambuf put-area byte writer                      |
// | `0x00600540` | recovered prefix of the option-table initializer            |
// | `0x00601cc1` | catch funclet (resume trampoline `0x00601ce0`)              |
// | `0x00601ce5` | catch funclet (resume `0x006019d6`)                         |
//
// The window is sparse: the next ledger row after `0x00601ce5` is
// `FUN_006022c0`, so 15 rows cannot be reached without leaving the window or
// duplicating another batch.  Every row present is translated here.
//
// The routines are MSVC C++ runtime/standard-library code, not game logic.
// They read no `World`/`SimUnit` state, so they are reproduced as pure helpers
// over the recovered object shapes.  `0x00600540`'s cached body stops after
// 115 bytes because the decompiler treats the out-of-window call at
// `FUN_005f16a0` as non-returning; disassembly shows the same block continues
// past that call.  The translation reproduces the recovered literal
// construction and documents the truncation rather than inventing the tail.
//
// Recovered `std::string` layout used by `0x006000c0`, `0x006002e0`,
// `0x00600330` and `0x00600540`: inline buffer at `+0x00`, size at `+0x10`,
// capacity at `+0x14`; capacity `0xf` means the inline buffer is live,
// anything larger is a heap block whose capacity excludes the terminating
// NUL.  [`A55String`] stores exactly the `size` content bytes and the
// capacity, which is all any observable arithmetic in these rows uses.

/// Inline (`std::string`) capacity written by every zero-initialisation in the
/// window (`*(this + 0x14) = 0xf`, e.g. `0x006002e0`, `0x00600330`,
/// `0x00600540`).
pub const A55_SSO_CAPACITY: u32 = 0xf;

/// Largest string size: `0x7fffffff` clamps both the `needed | 0xf` growth
/// term and the `capacity + capacity / 2` growth term in `0x006000c0` and in
/// the assign helper used by `0x006002e0`/`0x00600330`.
pub const A55_STRING_MAX: u32 = 0x7fff_ffff;

/// The `0x006003d0` vtable slot's constant (`or eax, 0xffffffff; ret`).
pub const A55_MINUS_ONE: i32 = -1;

/// `&DAT_00fa3940`: the `_Iostream_error_category` storage whose address
/// `0x006002a0` writes into the second dword of the output condition.  The
/// object itself is constructed by the out-of-window initializer passed to
/// the one-time guard, so only its address is an observable of this row.
pub const A55_IOSTREAM_CATEGORY_OBJECT: u32 = 0x00fa_3940;

/// `std::exception::vftable` (the pointer the exception destructor at
/// `FUN_00535d20` and both catch funclets store at object offset `+0x00`).
pub const A55_STD_EXCEPTION_VFTABLE: u32 = 0x00d3_27d8;

/// Resume address returned by `0x00601cc1`: the two-instruction trampoline at
/// `0x00601ce0` (`jmp 0x006019d6`).
pub const A55_CATCH_00601CC1_RESUME: u32 = 0x0060_1ce0;

/// Resume address returned directly by `0x00601ce5` (`0x006019d6`).
pub const A55_CATCH_00601CE5_RESUME: u32 = 0x0060_19d6;

/// The zero-initialised `std::string` value model: content bytes plus the
/// capacity stored at `+0x14`.  `capacity == 0xf` (or any value `<= 0xf`)
/// denotes the inline buffer; larger values denote the heap policy.  The
/// terminating NUL is implicit: both recovered builders write it, but no
/// arithmetic in the window reads it as content.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct A55String {
    pub bytes: Vec<u8>,
    pub capacity: u32,
}

impl A55String {
    /// The state written before every `std::string` construction in the
    /// window: size `0`, capacity `0xf`, first byte `0`.
    pub fn new_inline() -> Self {
        Self {
            bytes: Vec::new(),
            capacity: A55_SSO_CAPACITY,
        }
    }
}

/// `length_error` condition of the string growth policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A55LengthError;

/// Conditions `0x006000c0` can raise.  Retail throws `out_of_range` for
/// `size < pos` and `length_error` when `count > 0x7fffffff - size`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A55StringError {
    OutOfRange,
    Length,
}

/// Source operand of `std::string::insert(pos, ptr, count)`.
///
/// `External` models a pointer outside the destination object.  `Inline`
/// models the self-insert case where the source pointer is an offset into the
/// destination's own buffer; `0x006000c0` detects that overlap by comparing
/// the raw `dest`, `src`, `src + count` and `data + size` addresses.
#[derive(Clone, Copy, Debug)]
pub enum A55InsertSource<'a> {
    External(&'a [u8]),
    Inline(usize),
}

/// Growth policy shared by the appended/replaced string paths in the window.
///
/// ```c
/// uVar5 = needed | 0xf;
/// if (0x7fffffff < uVar5) {
///     uVar5 = 0x7fffffff;
/// } else if (0x7fffffff - (cap >> 1) < cap) {
///     uVar5 = 0x7fffffff;
/// } else if (uVar5 < cap + (cap >> 1)) {
///     uVar5 = cap + (cap >> 1);
/// }
/// ```
///
/// This is the exact sequence at `0x006000c0` (`0x600187..0x6001bd`) and in
/// the assign helper used by `0x006002e0`/`0x00600330`
/// (`0x5fdb6a..0x5fdb99`).  `needed` is always `<= 0x7fffffff`.
pub fn a55_grow_capacity(old_capacity: u32, needed: u32) -> u32 {
    let mut new_capacity = needed | 0xf;
    if new_capacity > A55_STRING_MAX {
        new_capacity = A55_STRING_MAX;
    } else {
        let half = old_capacity >> 1;
        if A55_STRING_MAX - half < old_capacity {
            new_capacity = A55_STRING_MAX;
        } else {
            let grown = old_capacity + half;
            if new_capacity < grown {
                new_capacity = grown;
            }
        }
    }
    new_capacity
}

/// Capacity after assigning `length` bytes to a string of capacity
/// `old_capacity`, as performed by the assign helper called from
/// `0x006002e0`/`0x00600330`/`0x00600540`.
///
/// If the request fits (`length <= old_capacity`) the capacity is unchanged;
/// otherwise `length > 0x7fffffff` is the `length_error` path and any other
/// request runs [`a55_grow_capacity`].
pub fn a55_assign_capacity(length: u32, old_capacity: u32) -> Result<u32, A55LengthError> {
    if length <= old_capacity {
        Ok(old_capacity)
    } else if length > A55_STRING_MAX {
        Err(A55LengthError)
    } else {
        Ok(a55_grow_capacity(old_capacity, length))
    }
}

/// Capacity chosen by `0x006000c0` when `count` bytes are inserted at `pos`
/// into a string of `size` bytes and `old_capacity` capacity.
///
/// The length guard is the unsigned `0x7fffffff - size < count` test at
/// `0x600178..0x600181`; the rest is [`a55_grow_capacity`] over
/// `needed = size + count`.
pub fn a55_insert_capacity(
    size: u32,
    count: u32,
    old_capacity: u32,
) -> Result<u32, A55StringError> {
    if count > A55_STRING_MAX - size {
        return Err(A55StringError::Length);
    }
    Ok(a55_grow_capacity(old_capacity, size + count))
}

/// Overlap adjustment of `0x006000c0`'s in-place branch
/// (`0x600103..0x600135`).
///
/// `dest` is `pos`, `src` is the source offset and `size` the prior content
/// size, all rebased at the destination buffer.  The recovered comparisons are:
///
/// ```c
/// if (src + count <= dest)                 n = count;
/// else if (src > data + size)              n = count;
/// else if (dest <= src)                    n = 0;
/// else                                     n = dest - src;
/// ```
///
/// The first `n` bytes are copied from the pre-shift source; the remaining
/// `count - n` bytes are copied from `src + n + count` in the shifted buffer.
pub fn a55_insert_overlap(pos: u32, size: u32, src: u32, count: u32) -> u32 {
    let src_end = src.wrapping_add(count);
    if src_end <= pos {
        count
    } else if src > size {
        count
    } else if pos <= src {
        0
    } else {
        pos - src
    }
}

/// `0x006000c0` (`FUN_006000c0`, 287 bytes, `decompiled_uncited`):
/// `std::string::insert(size_type pos, const char* ptr, size_type count)`.
///
/// The recovered body is:
///
/// ```c
/// if (size < pos) _Xran();                          /* 0x6000d3 */
/// if (count <= cap - size) {                        /* 0x6000e6 */
///     size += count;                                /* 0x6000f4 */
///     move(data + pos + count, data + pos, size - pos + 1);
///     /* copy split by the overlap adjustment, see a55_insert_overlap */
///     return this;
/// }
/// if (count > 0x7fffffff - size) _Xlength();        /* 0x600181 */
/// cap = (size + count) | 0xf;                       /* growth policy */
/// new_data = allocate(cap + 1);
/// copy(new_data, data, pos);
/// copy(new_data + pos, ptr, count);
/// copy(new_data + pos + count, data + pos, size - pos + 1);
/// free_old_if_heap();
/// return this;
/// ```
///
/// The capacity includes no NUL slot in [`A55String`]; retail allocates
/// `cap + 1` bytes, which is why the growth policy tests `needed | 0xf` and
/// then allocates one more byte.  The in-place overlap split is reproduced
/// byte-for-byte, including the `n = 0` case where the source lies at or after
/// the insertion point.
pub fn a55_string_insert(
    string: &mut A55String,
    pos: u32,
    source: A55InsertSource<'_>,
    count: u32,
) -> Result<(), A55StringError> {
    let size = string.bytes.len() as u32;
    assert!(
        string.capacity >= size,
        "A55String capacity must cover its content"
    );
    if size < pos {
        return Err(A55StringError::OutOfRange);
    }

    if count <= string.capacity - size {
        let old = string.bytes.clone();
        let overlap = match source {
            A55InsertSource::External(_) => count,
            A55InsertSource::Inline(offset) => a55_insert_overlap(pos, size, offset as u32, count),
        };

        string.bytes.resize((size + count) as usize, 0);
        string
            .bytes
            .copy_within(pos as usize..size as usize, (pos + count) as usize);

        match source {
            A55InsertSource::External(bytes) => {
                assert!(
                    bytes.len() >= count as usize,
                    "insert source must hold count bytes"
                );
                string.bytes[pos as usize..(pos + overlap) as usize]
                    .copy_from_slice(&bytes[..overlap as usize]);
            }
            A55InsertSource::Inline(offset) => {
                assert!(
                    offset as u64 + count as u64 <= size as u64,
                    "self-insert source must lie inside the old content"
                );
                for i in 0..overlap as usize {
                    string.bytes[pos as usize + i] = old[offset + i];
                }
                for i in overlap as usize..count as usize {
                    string.bytes[pos as usize + i] = string.bytes[offset + i + count as usize];
                }
            }
        }
        return Ok(());
    }

    // Length is checked before the source is touched, matching the retail
    // order (throw first, allocate second).
    let new_capacity = a55_insert_capacity(size, count, string.capacity)?;
    let old = string.bytes.clone();

    let mut grown = Vec::with_capacity(new_capacity as usize);
    grown.extend_from_slice(&old[..pos as usize]);
    match source {
        A55InsertSource::External(bytes) => {
            assert!(
                bytes.len() >= count as usize,
                "insert source must hold count bytes"
            );
            grown.extend_from_slice(&bytes[..count as usize]);
        }
        A55InsertSource::Inline(offset) => {
            assert!(
                offset as u64 + count as u64 <= size as u64,
                "self-insert source must lie inside the old content"
            );
            grown.extend_from_slice(&old[offset..offset + count as usize]);
        }
    }
    grown.extend_from_slice(&old[pos as usize..]);
    debug_assert_eq!(grown.len(), (size + count) as usize);

    string.bytes = grown;
    string.capacity = new_capacity;
    Ok(())
}

/// `0x006002a0` (`FUN_006002a0`, 51 bytes, `decompiled_uncited`):
/// build an `error_condition` for value `value` in the iostream category.
///
/// The body first runs the one-time initializer
/// `FUN_00c84922(&DAT_00fa393c, FUN_005ee660, &DAT_00fa3940)` and branches on
/// its result.  On success it writes exactly two dwords:
///
/// ```c
/// out[0] = value;
/// out[1] = &DAT_00fa3940;              /* the iostream category object */
/// ```
///
/// A zero result jumps to the out-of-window throw/abort path at `0xc98116`
/// (construct an 8-byte exception object, run the terminate handler, call the
/// abort routine).  `initialized` supplies that guard's observable result;
/// `Err(A55InitFailed)` models the non-returning failure branch.
pub fn a55_iostream_default_error_condition(
    value: u32,
    initialized: bool,
) -> Result<A55ErrorCondition, A55InitFailed> {
    if !initialized {
        return Err(A55InitFailed);
    }
    Ok(A55ErrorCondition {
        value,
        category: A55_IOSTREAM_CATEGORY_OBJECT,
    })
}

/// The two dwords written by `0x006002a0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A55ErrorCondition {
    pub value: u32,
    pub category: u32,
}

/// Marker for the non-returning failure branch of `0x006002a0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A55InitFailed;

/// `0x006002e0` (`FUN_006002e0`, 66 bytes, `decompiled_uncited`): construct
/// the generic error-category message string from the `_Syserror_map` text.
///
/// The body zero-initialises the destination (`size = 0`, `capacity = 0xf`,
/// first byte `0`), measures the supplied C string with a byte scan and then
/// assigns exactly those bytes through the helper at `0x5fdb20`.  The lookup
/// that maps the row's integer argument to this text lives outside the
/// window, so the text is an input here; the string construction itself is
/// reproduced with the exact capacity policy in [`a55_assign_capacity`].
///
/// Disassembly cross-check: this routine occupies the `message` slot of the
/// generic error-category vtable at `0x00d381e0` (next to the `"generic"`
/// name slot translated as [`a55_generic_error_category_name`]).
pub fn a55_generic_error_category_message(syserror: &[u8]) -> Result<A55String, A55LengthError> {
    a55_string_assign(syserror)
}

/// `0x00600330` (`FUN_00600330`, 107 bytes, `decompiled_uncited`): construct
/// the iostream error-category message string.
///
/// ```c
/// if (code == 1) {
///     assign("iostream stream error", 0x15);
/// } else {
///     assign(_Syserror_map(code), strlen(...));
/// }
/// ```
///
/// The literal is 21 bytes, so its capacity is `0x15 | 0xf = 0x1f` (larger
/// than the `0xf + 0xf/2 = 0x16` fallback).  The `_Syserror_map` text is
/// supplied by the caller because the map is outside the window.  Disassembly
/// places this routine in the `message` slot of the iostream category vtable
/// at `0x00d38204`.
pub fn a55_iostream_error_category_message(
    code: i32,
    syserror: &[u8],
) -> Result<A55String, A55LengthError> {
    if code == 1 {
        a55_string_assign(b"iostream stream error")
    } else {
        a55_string_assign(syserror)
    }
}

/// Assign `text` to a freshly zero-initialised [`A55String`].
///
/// This reproduces the visible part of the assign helper invoked from
/// `0x006002e0`, `0x00600330` and `0x00600540`: the destination starts at
/// `capacity = 0xf`, and `len <= 0xf` keeps the inline capacity while larger
/// requests run [`a55_grow_capacity`].  No content byte is dropped or added;
/// the NUL the retail code writes after the last byte is implicit.
pub fn a55_string_assign(text: &[u8]) -> Result<A55String, A55LengthError> {
    let mut string = A55String::new_inline();
    string.capacity = a55_assign_capacity(text.len() as u32, A55_SSO_CAPACITY)?;
    string.bytes.extend_from_slice(text);
    Ok(string)
}

/// `0x006003b0` (`FUN_006003b0`, 6 bytes, `decompiled_uncited`): the generic
/// error-category `name()` accessor.
///
/// The body is `mov eax, 0xd381f8; ret` in the client, i.e. a pointer to the
/// NUL-terminated literal `"generic"` immediately following the vtable.
pub fn a55_generic_error_category_name() -> &'static str {
    "generic"
}

/// `0x006003c0` (`FUN_006003c0`, 6 bytes, `decompiled_uncited`): the iostream
/// error-category `name()` accessor.
///
/// The body returns a pointer to the NUL-terminated literal `"iostream"`.
pub fn a55_iostream_error_category_name() -> &'static str {
    "iostream"
}

/// `0x006003d0` (`FUN_006003d0`, 6 bytes, `decompiled_uncited`): a virtual
/// `int` slot whose whole body is `or eax, 0xffffffff; ret`.
///
/// The row uses neither `this` nor arguments, so the only recoverable
/// behaviour is the sentinel.  The slot sits in the streambuf vtable at
/// `0x00d382bc`, next to the put-area writer translated as
/// [`A55PutArea::a55_put_byte`]; its C++ identity is not provable from the
/// six bytes, so the translation reproduces exactly the constant.
pub fn a55_stream_vtable_minus_one() -> i32 {
    A55_MINUS_ONE
}

/// Growth policy of `0x006003e0` (`0x60044e..0x600478`).
///
/// `used < 0x20` allocates `0x20`; `used < 0x3fffffff` allocates `2 * used`;
/// otherwise `used >= 0x7fffffff` fails with `-1` and any remaining value
/// allocates `0x7fffffff`.  The `0x006003e0` body computes this after
/// `used = current + available - base`, i.e. the number of live bytes in the
/// buffer.
pub fn a55_streambuf_growth(used: u32) -> Option<u32> {
    if used < 0x20 {
        Some(0x20)
    } else if used < 0x3fff_ffff {
        Some(used * 2)
    } else if used >= 0x7fff_ffff {
        None
    } else {
        Some(0x7fff_ffff)
    }
}

/// Put area of the streambuf object handled by `0x006003e0`.
///
/// Recovered member offsets (all are pointers-to-cells in retail, because the
/// same cell can be swapped when the buffer is replaced):
///
/// | offset | modelled field | meaning                          |
/// |--------|----------------|----------------------------------|
/// | `+0x0c`| (implicit base)| buffer start pointer             |
/// | `+0x1c`| `get_start`    | get-area current pointer         |
/// | `+0x20`| `current`      | put pointer                      |
/// | `+0x2c`| `get_end`      | get-area remaining count         |
/// | `+0x30`| `available`    | put-area remaining count         |
/// | `+0x38`| `end_marker`   | `current + 1` after each store   |
/// | `+0x3c`| `flags`        | bit 1 error, bit 2 reset get area, bit 0 buffer owned |
///
/// [`A55PutArea`] rebases every allocation at offset 0, so the `+0x0c` start
/// slot is always 0 and the other fields are offsets into `buffer`.  That
/// preserves every comparison and difference used by the body (`used`,
/// `available`, `end_marker - get_start`) without inventing client addresses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A55PutArea {
    pub flags: u32,
    pub buffer: Vec<u8>,
    pub current: u32,
    pub available: i32,
    pub end_marker: u32,
    pub get_start: u32,
    pub get_end: u32,
}

impl A55PutArea {
    /// `0x006003e0` (`FUN_006003e0`, 186 bytes, `decompiled_uncited`):
    /// store one byte, growing the put buffer when the current cell is full.
    ///
    /// Recovered control flow:
    ///
    /// ```c
    /// if (flags & 2) return -1;                       /* 0x6003eb */
    /// if (ch == -1) return 0;                         /* 0x6003f5 */
    /// end = available + current;
    /// if (current != 0 && current < end) {            /* 0x600413 */
    ///     --available;                                /* 0x60041b */
    ///     *current++ = ch;
    ///     end_marker = old_current + 1;
    ///     return ch;
    /// }
    /// used = current != 0 ? end - base : 0;           /* 0x60043e..0x60044c */
    /// cap = growth(used);                             /* 0x60044e..0x600476 */
    /// if (cap == -1) return -1;
    /// new = allocate(cap); copy(new, base, used);
    /// end_marker = new + used + 1;                    /* 0x600499 */
    /// if (flags & 4) { get_start = new; get_end = new; }
    /// else { get_start = old_get_start - old_base + new;
    ///        get_end = end_marker - get_start; }     /* 0x6004b2..0x6004ef */
    /// if (flags & 1) free(old_base, used);
    /// flags |= 1;
    /// --available; *current++ = ch;
    /// return ch;
    /// ```
    ///
    /// The exact 32-bit arithmetic is kept: the `current != 0 ? ... : 0`
    /// selection is the retail `neg`/`sbb`/`and` sequence, `end` uses a
    /// wrapping add, and the capacity thresholds are those of
    /// [`a55_streambuf_growth`].
    pub fn a55_put_byte(&mut self, ch: i32) -> i32 {
        if self.flags & 2 != 0 {
            return -1;
        }
        if ch == -1 {
            return 0;
        }

        let old_current = self.current;
        let end = (self.available as u32).wrapping_add(old_current);
        if old_current != 0 && old_current < end {
            self.available -= 1;
            let index = old_current as usize;
            assert!(
                index < self.buffer.len(),
                "put pointer must address the modeled buffer"
            );
            self.buffer[index] = ch as u8;
            self.current = old_current + 1;
            self.end_marker = old_current + 1;
            return ch;
        }

        let used = if old_current != 0 { end } else { 0 };
        let capacity = match a55_streambuf_growth(used) {
            Some(capacity) => capacity,
            None => return -1,
        };

        assert!(
            used as usize <= self.buffer.len(),
            "live byte count must fit the modeled buffer"
        );
        let mut grown = vec![0u8; capacity as usize];
        grown[..used as usize].copy_from_slice(&self.buffer[..used as usize]);
        self.buffer = grown;

        if self.flags & 4 != 0 {
            self.get_start = 0;
            self.get_end = 0;
        } else {
            // `new` is offset 0 in the rebased model, so the recovered
            // `get_start - old_base + new` reduces to the old get_start.
            let new_get_start = self.get_start;
            let end_marker = used.wrapping_add(1);
            self.get_end = end_marker.wrapping_sub(new_get_start);
            self.get_start = new_get_start;
        }

        self.flags |= 1;

        // Old buffer release (flags & 1) is a no-op in the model: `buffer`
        // is replaced above and owns no client allocation.
        self.available = (capacity - used) as i32;
        self.available -= 1;
        self.buffer[used as usize] = ch as u8;
        self.current = used + 1;
        self.end_marker = used + 1;
        ch
    }
}

/// `0x00600540` (`FUN_00600540`, ledger size 115, `decompiled_uncited`):
/// the recovered prefix of the option-table initializer.
///
/// The cached 115-byte body is the SEH prologue (`0x10ac` frame,
/// `FUN_00c86dc0`) followed by the zero-initialisation of a local string and
/// the call `FUN_005fdb20("Starcraft", 9)`.  The decompiler marks that call
/// non-returning, so the body stored in the ledger stops there; disassembly
/// shows the same code block continuing at `0x006005b3` with further option
/// strings (`"locale"`, `"tag"`, `"profile"`, ...), outside the recorded body.
///
/// Only the recovered prefix is translated: the zero-initialised local string
/// with the nine literal bytes.  No parameters of the row are read before the
/// truncation point.
pub fn a55_starcraft_literal() -> A55String {
    // The assign helper sees a zero-initialised destination (cap 0xf) and a
    // 9-byte literal, so the inline capacity is retained.
    a55_string_assign(b"Starcraft").expect("9 bytes always fit the inline capacity")
}

/// The owned payload carried at exception-object offset `+0x04`.
///
/// The release helper called by both catch funclets and by the exception
/// destructor checks the byte flag at `+0x04`, deletes the pointer at `+0x00`
/// when set, then clears the pointer and the flag unconditionally.  The model
/// has no client heap block, so the delete is represented by clearing the
/// pointer; the observable post-state is identical.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A55OwnedBlock {
    pub pointer: u32,
    pub owned: bool,
}

/// Release the owned payload exactly like the helper at `FUN_00c89561`.
pub fn a55_release_owned(block: &mut A55OwnedBlock) {
    if block.owned {
        // `operator delete(block.pointer)` in retail.
        block.pointer = 0;
    }
    block.pointer = 0;
    block.owned = false;
}

/// Exception object touched by the two catch funclets: vtable pointer at
/// `+0x00`, owned payload at `+0x04`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A55ExceptionObject {
    pub vtable: u32,
    pub block: A55OwnedBlock,
}

/// `0x00601cc1` (`Catch_00601cc1`, 31 bytes, `decompiled_uncited`): unwind
/// catch funclet.
///
/// ```c
/// *(object + 0x00) = std::exception::vftable;    /* 0xd327d8 */
/// release_owned(object + 0x04);                  /* FUN_00c89561 */
/// return &DAT_00601ce0;                          /* jmp 0x006019d6 */
/// ```
///
/// The returned address is the trampoline at `0x00601ce0`, whose only
/// instruction is `jmp 0x006019d6`; the model keeps the exact value the
/// funclet returns.
pub fn a55_catch_00601cc1(object: &mut A55ExceptionObject) -> u32 {
    object.vtable = A55_STD_EXCEPTION_VFTABLE;
    a55_release_owned(&mut object.block);
    A55_CATCH_00601CC1_RESUME
}

/// `0x00601ce5` (`Catch_00601ce5`, 31 bytes, `decompiled_uncited`): second
/// unwind catch funclet.
///
/// Same two operations as [`a55_catch_00601cc1`] on the object at
/// `ebp - 0x10bc`, but the funclet returns the resume address `0x006019d6`
/// directly instead of the `0x00601ce0` trampoline.
pub fn a55_catch_00601ce5(object: &mut A55ExceptionObject) -> u32 {
    object.vtable = A55_STD_EXCEPTION_VFTABLE;
    a55_release_owned(&mut object.block);
    A55_CATCH_00601CE5_RESUME
}

#[cfg(test)]
mod tests_agent55 {
    use super::*;

    fn text(bytes: &[u8]) -> A55String {
        a55_string_assign(bytes).unwrap()
    }

    #[test]
    fn zero_init_matches_retail_layout_constants() {
        // 0x006002e0 / 0x00600330 / 0x00600540 write size 0, cap 0xf, byte 0.
        let s = A55String::new_inline();
        assert!(s.bytes.is_empty());
        assert_eq!(s.capacity, 0xf);
        assert_eq!(A55_SSO_CAPACITY, 0xf);
        assert_eq!(A55_STRING_MAX, 0x7fff_ffff);
    }

    #[test]
    fn assign_capacity_policy_is_exact() {
        // 0x5fdb20 policy: fit keeps capacity, growth is max(len|0xf, cap*1.5).
        assert_eq!(a55_assign_capacity(0xf, 0xf), Ok(0xf));
        assert_eq!(a55_assign_capacity(0x10, 0xf), Ok(0x1f));
        assert_eq!(a55_assign_capacity(0x1f, 0xf), Ok(0x1f));
        assert_eq!(a55_assign_capacity(0x20, 0x1f), Ok(0x2f));
        assert_eq!(a55_assign_capacity(0x30, 0x2f), Ok(0x46));
        assert_eq!(a55_assign_capacity(0x7fff_ffff, 0xf), Ok(0x7fff_ffff));
    }

    #[test]
    fn grow_capacity_clamps_and_uses_half_again() {
        // 0x600187..0x6001bd.
        assert_eq!(a55_grow_capacity(0, 0), 0xf);
        assert_eq!(a55_grow_capacity(0xf, 0x10), 0x1f);
        assert_eq!(a55_grow_capacity(0x1f, 0x20), 0x2f);
        assert_eq!(a55_grow_capacity(0x20, 0x21), 0x30);
        // `0x7fffffff - (cap >> 1) < cap` takes the max clamp.
        assert_eq!(a55_grow_capacity(0x6000_0000, 0x7000_0000), A55_STRING_MAX);
        assert_eq!(
            a55_grow_capacity(A55_STRING_MAX, A55_STRING_MAX),
            A55_STRING_MAX
        );
    }

    #[test]
    fn insert_capacity_guards_size_plus_count() {
        // 0x600178..0x600181.
        assert_eq!(a55_insert_capacity(0, 0x7fff_ffff, 0xf), Ok(0x7fff_ffff));
        assert_eq!(
            a55_insert_capacity(1, 0x7fff_ffff, 0xf),
            Err(A55StringError::Length)
        );
        assert_eq!(a55_insert_capacity(0x1f, 1, 0x1f), Ok(0x2f));
    }

    #[test]
    fn insert_in_place_middle_keeps_capacity() {
        let mut s = text(b"abcdef");
        a55_string_insert(&mut s, 2, A55InsertSource::External(b"XY"), 2).unwrap();
        assert_eq!(s.bytes, b"abXYcdef");
        assert_eq!(s.capacity, 0xf);
    }

    #[test]
    fn insert_on_boundary_and_empty() {
        let mut s = text(b"abc");
        let before = s.clone();
        a55_string_insert(&mut s, 1, A55InsertSource::External(b""), 0).unwrap();
        assert_eq!(s, before);

        a55_string_insert(&mut s, 3, A55InsertSource::External(b"Z"), 1).unwrap();
        assert_eq!(s.bytes, b"abcZ");
        assert_eq!(s.capacity, 0xf);
    }

    #[test]
    fn insert_out_of_range() {
        let mut s = text(b"abc");
        assert_eq!(
            a55_string_insert(&mut s, 4, A55InsertSource::External(b"x"), 1),
            Err(A55StringError::OutOfRange)
        );
        assert_eq!(s.bytes, b"abc");
    }

    #[test]
    fn insert_length_error_precedes_source_access() {
        // The source slice is empty; the length guard must fire first.
        let mut s = text(b"a");
        assert_eq!(
            a55_string_insert(&mut s, 0, A55InsertSource::External(&[]), 0x7fff_ffff),
            Err(A55StringError::Length)
        );
        assert_eq!(s.bytes, b"a");
    }

    #[test]
    fn insert_growth_uses_needed_or_sso() {
        let mut s = text(b"0123456789abcde"); // 15 bytes, cap 0xf
        assert_eq!(s.capacity, 0xf);
        a55_string_insert(&mut s, 15, A55InsertSource::External(b"XYZ"), 3).unwrap();
        assert_eq!(s.bytes, b"0123456789abcdeXYZ");
        // needed 18 -> 18 | 0xf = 0x1f
        assert_eq!(s.capacity, 0x1f);
    }

    #[test]
    fn insert_growth_prefers_half_again_when_larger() {
        let mut s = text(&[b'a'; 0x1f]); // cap 0x1f, full
        assert_eq!(s.capacity, 0x1f);
        a55_string_insert(&mut s, 0, A55InsertSource::External(b"z"), 1).unwrap();
        // needed 0x20 -> 0x2f; cap + cap/2 = 0x2e -> 0x2f
        assert_eq!(s.capacity, 0x2f);
        assert_eq!(s.bytes.len(), 0x20);
        assert_eq!(s.bytes[0], b'z');
    }

    #[test]
    fn insert_self_overlap_copies_tail_from_shifted_buffer() {
        // dest=2, src=1, count=3 -> n = dest - src = 1.
        let mut s = text(b"abcdef");
        a55_string_insert(&mut s, 2, A55InsertSource::Inline(1), 3).unwrap();
        assert_eq!(s.bytes, b"abbcdcdef");
    }

    #[test]
    fn insert_self_at_or_after_dest_uses_shifted_source_only() {
        // dest=1 <= src=3 -> n = 0; all bytes come from src + count.
        let mut s = text(b"abcdef");
        a55_string_insert(&mut s, 1, A55InsertSource::Inline(3), 2).unwrap();
        assert_eq!(s.bytes, b"adebcdef");
    }

    #[test]
    fn insert_self_overlap_growth_reads_unshifted_old_buffer() {
        let mut s = text(b"0123456789abcdef"); // 16 bytes, cap 0x1f
        a55_string_insert(&mut s, 0, A55InsertSource::Inline(0), 16).unwrap();
        assert_eq!(s.bytes, b"0123456789abcdef0123456789abcdef");
        // needed 32 -> 0x2f
        assert_eq!(s.capacity, 0x2f);
    }

    #[test]
    fn insert_overlap_helper_matches_four_cases() {
        // src + count <= dest
        assert_eq!(a55_insert_overlap(10, 20, 2, 4), 4);
        // src > size
        assert_eq!(a55_insert_overlap(10, 20, 21, 4), 4);
        // dest <= src (but inside data)
        assert_eq!(a55_insert_overlap(10, 20, 10, 4), 0);
        // overlapping
        assert_eq!(a55_insert_overlap(10, 20, 7, 4), 3);
    }

    #[test]
    fn generic_category_message_builds_expected_strings() {
        let short = a55_generic_error_category_message(b"No such file").unwrap();
        assert_eq!(short.bytes, b"No such file");
        assert_eq!(short.capacity, 0xf);

        let long = a55_generic_error_category_message(b"0123456789abcdef").unwrap();
        assert_eq!(long.bytes, b"0123456789abcdef");
        assert_eq!(long.capacity, 0x1f);
    }

    #[test]
    fn iostream_category_message_special_cases_one() {
        let literal = a55_iostream_error_category_message(1, b"ignored").unwrap();
        assert_eq!(literal.bytes, b"iostream stream error");
        assert_eq!(literal.bytes.len(), 0x15);
        assert_eq!(literal.capacity, 0x1f);

        let mapped = a55_iostream_error_category_message(2, b"bad file").unwrap();
        assert_eq!(mapped.bytes, b"bad file");
        assert_eq!(mapped.capacity, 0xf);
    }

    #[test]
    fn default_error_condition_writes_value_and_singleton() {
        let condition = a55_iostream_default_error_condition(5, true).unwrap();
        assert_eq!(
            condition,
            A55ErrorCondition {
                value: 5,
                category: 0x00fa_3940,
            }
        );
        assert_eq!(
            a55_iostream_default_error_condition(5, false),
            Err(A55InitFailed)
        );
    }

    #[test]
    fn category_names_and_minus_one_are_exact() {
        assert_eq!(a55_generic_error_category_name(), "generic");
        assert_eq!(a55_iostream_error_category_name(), "iostream");
        assert_eq!(a55_stream_vtable_minus_one(), -1);
        assert_eq!(A55_MINUS_ONE, 0xffff_ffffu32 as i32);
    }

    #[test]
    fn streambuf_growth_thresholds() {
        assert_eq!(a55_streambuf_growth(0), Some(0x20));
        assert_eq!(a55_streambuf_growth(0x1f), Some(0x20));
        assert_eq!(a55_streambuf_growth(0x20), Some(0x40));
        assert_eq!(a55_streambuf_growth(0x3fff_fffe), Some(0x7fff_fffc));
        assert_eq!(a55_streambuf_growth(0x3fff_ffff), Some(0x7fff_ffff));
        assert_eq!(a55_streambuf_growth(0x7fff_fffe), Some(0x7fff_ffff));
        assert_eq!(a55_streambuf_growth(0x7fff_ffff), None);
    }

    fn put_area(flags: u32, current: u32, available: i32, capacity: usize) -> A55PutArea {
        A55PutArea {
            flags,
            buffer: vec![0; capacity],
            current,
            available,
            end_marker: current,
            get_start: 0,
            get_end: 0,
        }
    }

    #[test]
    fn streambuf_fast_path_stores_without_growth() {
        let mut area = put_area(0, 4, 0x1c, 0x20);
        assert_eq!(area.a55_put_byte(0x41), 0x41);
        assert_eq!(area.buffer[4], 0x41);
        assert_eq!(area.current, 5);
        assert_eq!(area.available, 0x1b);
        assert_eq!(area.end_marker, 5);
        assert_eq!(area.flags, 0);
        assert_eq!(area.buffer.len(), 0x20);
    }

    #[test]
    fn streambuf_error_and_eof_short_circuits() {
        let mut error = put_area(2, 4, 0x1c, 0x20);
        assert_eq!(error.a55_put_byte(0x41), -1);
        assert_eq!(error.current, 4);
        assert_eq!(error.available, 0x1c);

        let mut eof = put_area(0, 0, 0, 0);
        assert_eq!(eof.a55_put_byte(-1), 0);
        assert_eq!(eof.current, 0);
        assert_eq!(eof.flags, 0);
    }

    #[test]
    fn streambuf_growth_from_null_pointer_allocates_0x20() {
        let mut area = A55PutArea {
            flags: 0,
            buffer: Vec::new(),
            current: 0,
            available: 0,
            end_marker: 0,
            get_start: 8,
            get_end: 0,
        };
        assert_eq!(area.a55_put_byte(0x42), 0x42);
        assert_eq!(area.buffer.len(), 0x20);
        assert_eq!(area.current, 1);
        assert_eq!(area.available, 0x1f);
        assert_eq!(area.end_marker, 1);
        assert_eq!(area.flags, 1);
        // used = 0 -> end_marker 1, get_end = 1 - get_start.
        assert_eq!(area.get_start, 8);
        assert_eq!(area.get_end, 1u32.wrapping_sub(8));
    }

    #[test]
    fn streambuf_growth_resets_get_area_when_flag4() {
        let mut area = A55PutArea {
            flags: 4,
            buffer: Vec::new(),
            current: 0,
            available: 0,
            end_marker: 0,
            get_start: 8,
            get_end: 9,
        };
        assert_eq!(area.a55_put_byte(0x42), 0x42);
        assert_eq!(area.get_start, 0);
        assert_eq!(area.get_end, 0);
        assert_eq!(area.flags, 5); // bit 4 preserved, bit 0 set
    }

    #[test]
    fn streambuf_growth_doubles_when_used_is_0x20() {
        let mut area = put_area(0, 0x20, 0, 0x20);
        assert_eq!(area.a55_put_byte(0x43), 0x43);
        assert_eq!(area.buffer.len(), 0x40);
        assert_eq!(area.current, 0x21);
        assert_eq!(area.available, 0x1f);
        assert_eq!(area.end_marker, 0x21);
        assert_eq!(area.buffer[0x20], 0x43);
    }

    #[test]
    fn streambuf_growth_failure_returns_eof() {
        // used = 0x7fffffff takes the failure branch before any allocation.
        let mut area = put_area(0, 0x7fff_ffff, 0, 0);
        assert_eq!(area.a55_put_byte(0x44), -1);
        assert_eq!(area.current, 0x7fff_ffff);
        assert_eq!(area.flags, 0);
    }

    #[test]
    fn starcraft_prefix_is_the_recovered_literal() {
        let s = a55_starcraft_literal();
        assert_eq!(s.bytes, b"Starcraft");
        assert_eq!(s.capacity, 0xf);
    }

    #[test]
    fn catch_funclets_reset_vtable_and_release_payload() {
        let mut object = A55ExceptionObject {
            vtable: 0xdead_beef,
            block: A55OwnedBlock {
                pointer: 0x1234_5678,
                owned: true,
            },
        };
        assert_eq!(a55_catch_00601cc1(&mut object), 0x0060_1ce0);
        assert_eq!(object.vtable, A55_STD_EXCEPTION_VFTABLE);
        assert_eq!(
            object.block,
            A55OwnedBlock {
                pointer: 0,
                owned: false,
            }
        );

        let mut second = A55ExceptionObject {
            vtable: 1,
            block: A55OwnedBlock {
                pointer: 0x99,
                owned: false,
            },
        };
        assert_eq!(a55_catch_00601ce5(&mut second), 0x0060_19d6);
        assert_eq!(second.vtable, 0x00d3_27d8);
        assert_eq!(second.block, A55OwnedBlock::default());
    }
}
