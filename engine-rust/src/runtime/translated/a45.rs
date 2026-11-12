#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent45.rs
// ===========================================================================
// Translation batch 45.  Only this file is edited by the batch.
//
// The `decompiled_uncited` rows in `0x005ec000..=0x005edfff` are the tail of
// the retail client's command-line parser (the `boost::program_options`
// flag/argument matcher that fills in `options_description`) together with
// the MSVC container helpers that parser instantiates: `std::vector`
// insertion under the 1.5x capacity policy, `std::string` small-buffer
// release, `substr`-style clamping and the exception-unwind funclets of both.
//
// None of these rows reads a `CUnit`, a scenario table or an order queue, and
// the client parser has no simulation state at all.  The batch therefore
// translates the arithmetic each row performs instead of mapping it onto
// [`crate::sim::World`]: the growth policy and its per-element-size limit, the size
// request used when a small-buffer node is released, the record offsets each
// insertion produces, the substring clamp and the flag word the parser
// reports.  Every entry is a free function, so no `sim` record gains a field.
// The allocator itself, the MSVC exception frame (`ExceptionList`,
// `__CxxThrowException`), the virtual dispatch through the option-description
// vtables and the locale-facet construction have no Rust counterpart and are
// not reproduced; each doc comment states which part of the recovered body is
// modelled and which is not.
//
// Address to function:
//
// * `0x005ec590` [`a45_tail_range`]
// * `0x005ecab0` [`a45_tail_range_at_descriptor`]
// * `0x005ecf40` [`a45_record_byte`]
// * `0x005ed240`, `0x005ed2b0` [`a45_release_text_node`]
// * `0x005ed1dc` [`a45_report_flags`]
// * `0x005ed320` [`a45_array52_append_slot`]
// * `0x005ed3ab` [`a45_array52_grow_slot`]
// * `0x005ed940`, `0x005edf60` [`a45_array52_insert_slot`]
// * `0x005edaf0` [`a45_array52_pair_slot`]
// * `0x005edb14` [`a45_array52_range_slots`]
// * `0x005ed4d0`, `0x005ed660`, `0x005ed68b` [`a45_array28_slot`]
// * `0x005ed7e0`, `0x005ede40` [`a45_array24_slot`]
// * `0x005edcb0` [`a45_array4_grow`]
// * `0x005ede10` [`a45_array4_fresh_layout`]
// * `0x005edd90`, `0x005eddde` [`a45_array4_commit`]
// * `0x005ed556` [`a45_char_index`]

/// Element-count limit of the 0x34-byte growable arrays: `0xffffffff / 0x34`.
///
/// The recovered routines compare the element count against the literal
/// `0x004ec4ec4` before growing, and `0x004ec4ec4 * 0x34 = 0xffffffd0` is the
/// largest whole number of 52-byte elements that fits the client's 32-bit
/// `size_t`.  The test module pins the division.
pub const A45_MAX_ELEMENTS_52: u32 = 0x04ec_4ec4;
/// Element-count limit of the 0x1c-byte growable arrays
/// (`0x09249249 * 0x1c = 0xfffffffc`).
pub const A45_MAX_ELEMENTS_28: u32 = 0x0924_9249;
/// Element-count limit of the 0x18-byte growable arrays
/// (`0x0aaaaaaa * 0x18 = 0xfffffff0`).
pub const A45_MAX_ELEMENTS_24: u32 = 0x0aaa_aaaa;
/// Element-count limit of the 4-byte growable arrays
/// (`0x3fffffff * 4 = 0xfffffffc`).
pub const A45_MAX_ELEMENTS_4: u32 = 0x3fff_ffff;

/// `std::string`'s small-buffer capacity: a node whose capacity is below 16
/// keeps its bytes inside the object and is never released.
pub const A45_SMALL_STRING_CAPACITY: u32 = 0xf;
/// The client's heap rounds a request to a 0x24-byte chunk header; requests
/// under 0x1000 bytes go through the small-block path and are passed as-is.
pub const A45_SMALL_BLOCK_LIMIT: u32 = 0x1000;
pub const A45_CHUNK_HEADER: u32 = 0x24;
/// Extra bytes added by the recovered reallocation paths before rounding.
pub const A45_ROUNDING: u32 = 0x23;

/// `_Xlen()`: the recovered bodies jump to the standard-library
/// `length_error` throw when the element count has reached the limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A45LengthError;

/// `_Xran()`: the substring helper throws `out_of_range` when the requested
/// position is past the end of the source string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A45RangeError;

/// Result of one growable-array insertion: the capacity retail settled on and
/// the byte offset of the inserted element from the buffer base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A45Insert {
    pub capacity: u32,
    pub slot: u32,
}

/// Result of the 0x34-byte two-string insertion (`0x005edaf0`).
///
/// The record the three insertions build is 0x34 bytes: the key dword at `+0`,
/// one 0x18-byte `std::string` at `+4` and the second at `+0x1c`.  The two
/// string offsets are INFERENCE from that size (the recovered body stores the
/// key first and then constructs two strings through `FUN_005f12e0`), not
/// individually visible stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A45PairInsert {
    pub capacity: u32,
    pub slot: u32,
}

impl A45PairInsert {
    /// Offset of the key dword inside the new record.
    pub fn key_off(&self) -> u32 {
        self.slot
    }

    /// Offset of the first `std::string` inside the new record.
    pub fn first_text_off(&self) -> u32 {
        self.slot + 4
    }

    /// Offset of the second `std::string` inside the new record.
    pub fn second_text_off(&self) -> u32 {
        self.slot + 0x1c
    }
}

/// Result of the range insertion (`0x005edb14`): `slot` is the first element
/// of the copied range and `tail` the element just past its last one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A45RangeInsert {
    pub capacity: u32,
    pub slot: u32,
    pub tail: u32,
}

/// Result of the 4-byte-array insertion (`0x005edcb0`): the new capacity, the
/// slot offset, the byte size of the new block and the size the allocator is
/// asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A45Array4Grow {
    pub capacity: u32,
    pub slot: u32,
    pub bytes: u32,
    pub request: u32,
}

/// Result of committing a reallocated 4-byte array (`0x005edd90`,
/// `0x005eddde`): the new `[begin, finish, end)` range, the position the
/// insertion routine returns, and - when the old block existed - the size the
/// allocator is asked to release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A45Array4Commit {
    pub begin: u32,
    pub finish: u32,
    pub end: u32,
    pub position: u32,
    pub released: Option<u32>,
}

/// `0x005ec590`: clamp a substring request against the source string's size.
///
/// The routine loads the position from `this+0x130`, compares it with the
/// destination string's size (`dst+0x10`) and, when the position is in range,
/// assigns `min(npos, size - pos)` bytes starting at `data + pos`.  A position
/// past the end jumps to the `out_of_range` helper, which is the returned
/// error.  The `npos` test in the recovered body is the standard
/// `min(count, size - pos)` with `npos` as the incoming count: when the
/// distance is exactly `npos` retail stores `npos` again, so the count is
/// always simply `size - pos`.
///
/// The small-buffer branch (`0xf < capacity`, re-reading the object's own data
/// pointer instead of using the inline buffer) has no observable effect on the
/// computed range, so only the returned `(offset, count)` pair is modelled.
pub fn a45_tail_range(pos: u32, size: u32) -> Result<(u32, u32), A45RangeError> {
    if pos > size {
        return Err(A45RangeError);
    }
    Ok((pos, size - pos))
}

/// `0x005ecab0`: the same substring clamp reached with the position held in
/// the second descriptor member (`this+0x148`).
///
/// Apart from the member offset the body is the tail-range helper above: the
/// comparison is `pos <= size`, the count is `size - pos` and the error path
/// is the same `out_of_range` throw.  It is kept as its own entry point so the
/// two inlined `substr` sites map one to one onto addresses.
pub fn a45_tail_range_at_descriptor(pos: u32, size: u32) -> Result<(u32, u32), A45RangeError> {
    a45_tail_range(pos, size)
}

/// `0x005ecf40`: store one byte into the record a keyed lookup returned.
///
/// The helper first resolves the record with `FUN_005f9c30(key, table)` and
/// then writes the byte through the caller's pointer at `record+8`.  No other
/// byte of the record is touched, so the surrounding dword keeps its upper
/// bytes; the previous low byte is returned here to make the single-byte store
/// observable.  The lookup itself is client-side table plumbing and is taken
/// as given.
pub fn a45_record_byte(slot: &mut u32, value: u8) -> u8 {
    let previous = (*slot & 0xff) as u8;
    *slot = (*slot & !0xff) | u32::from(value);
    previous
}

/// `0x005ed240`, `0x005ed2b0`: the size request for releasing one text node.
///
/// Both routines walk a range of records and reset the `std::string` they
/// embed: `0x005ed240` over 0x1c-byte nodes (`{key, string}` - capacity at
/// `+0x18`), `0x005ed2b0` over 0x18-byte nodes (`{string}` - capacity at
/// `+0x14`).  In both the release path is the same: a capacity below 16 means
/// the bytes live inside the object and nothing is freed; otherwise the
/// allocator is called with `capacity + 1` bytes, or with `capacity + 0x24`
/// once `capacity + 1` leaves the 0x1000-byte small-block range.  The reset
/// that follows writes size 0, capacity 0xf and a zero first byte.
///
/// The recovered large-block path also validates the heap block with
/// `(ptr - *(ptr - 4)) - 4 < 0x20`; that is debug-heap bookkeeping and has no
/// caller-visible value, so it is not modelled.  `None` is the small-buffer
/// case (no free at all), `Some(n)` the byte count retail releases.
pub fn a45_release_text_node(capacity: u32) -> Option<u32> {
    if capacity <= A45_SMALL_STRING_CAPACITY {
        return None;
    }
    let small = capacity.wrapping_add(1);
    if small < A45_SMALL_BLOCK_LIMIT {
        Some(small)
    } else {
        Some(capacity.wrapping_add(A45_CHUNK_HEADER))
    }
}

/// `0x005ed1dc`: the flag word handed to the parser's report routine.
///
/// The recovered body computes
/// `flags = this[+0xc] | carried`, then `flags |= 4` only when the optional
/// object at `this[+0x38]` is null, passes the result to `FUN_005fdee0` and
/// finally releases that optional object if present.  The bitwise part is the
/// whole computation: bit 2 means "no attached object".  Two neighbouring
/// serialise rows inline the same three instructions around their own client
/// work; their bodies are listed as untranslated below.
pub fn a45_report_flags(persisted: u32, carried: u32, has_object: bool) -> u32 {
    let flags = persisted | carried;
    if has_object { flags } else { flags | 4 }
}

/// The 1.5x growth policy every insertion in this batch shares.
///
/// `size` is the current element count and `capacity` the current allocation,
/// both counted in elements; `max_size` is the element limit of the array.
///
/// * a full array is a `length_error` (retail compares against the limit
///   literal and jumps to the throw),
/// * the request is one element past the end,
/// * when `capacity <= max_size - capacity / 2` the new capacity is
///   `capacity + capacity / 2`, raised to the request if the doubling is not
///   enough (only reachable for capacities 0 and 1),
/// * otherwise doubling would overflow the limit, so the capacity is exactly
///   the request.
pub fn a45_grow_capacity(size: u32, capacity: u32, max_size: u32) -> Result<u32, A45LengthError> {
    if size == max_size {
        return Err(A45LengthError);
    }
    let need = size + 1;
    if capacity > max_size - (capacity >> 1) {
        return Ok(need);
    }
    let doubled = (capacity >> 1) + capacity;
    Ok(if doubled < need { need } else { doubled })
}

/// The overflow-guarded reserve used by `0x005ed3ab`.
///
/// That frame receives the growth base in two registers summed together
/// (`size + count`), the requested capacity in `ebx` and the limit in `eax`.
/// When `max_size - base / 2 < base` the requested capacity is kept as-is,
/// otherwise the result is the larger of `base + base / 2` and the requested
/// capacity.  Unlike [`a45_grow_capacity`] the overflowing branch does not
/// throw: the caller already holds a legal request.
fn a45_reserve_capacity(base: u32, requested: u32, max_size: u32) -> u32 {
    if max_size.wrapping_sub(base >> 1) < base {
        return requested;
    }
    let doubled = (base >> 1) + base;
    if doubled < requested {
        requested
    } else {
        doubled
    }
}

/// `0x005ed320`: append one 0x34-byte record to a growable array.
///
/// The routine takes the element count `(finish - begin) / 0x34` and the
/// capacity `(end - begin) / 0x34`, runs the growth policy, releases the old
/// storage range and returns `begin + index * 0x34` for the slot it built at
/// the array end.  This entry point is the `push_back` instantiation, so the
/// position is the element count itself.
pub fn a45_array52_append_slot(size: u32, capacity: u32) -> Result<A45Insert, A45LengthError> {
    let capacity = a45_grow_capacity(size, capacity, A45_MAX_ELEMENTS_52)?;
    Ok(A45Insert {
        capacity,
        slot: size * 0x34,
    })
}

/// `0x005ed940`, `0x005edf60`: insert one 0x34-byte record at a position.
///
/// The two rows are two instantiations of the same insertion that differ only
/// in the element-construction call (`0x005ed940` reaches `FUN_005f0570`,
/// `0x005edf60` reaches `0x005f0600`); growth, the
/// `(position - begin) / 0x34 * 0x34` slot arithmetic and the release of the
/// old range are identical.  `pos_off` is the insertion position as a byte
/// offset into the buffer, which is what the caller's element pointer yields
/// after subtracting the base; the division truncates exactly as the recovered
/// `idiv` does.
pub fn a45_array52_insert_slot(
    pos_off: u32,
    size: u32,
    capacity: u32,
) -> Result<A45Insert, A45LengthError> {
    let capacity = a45_grow_capacity(size, capacity, A45_MAX_ELEMENTS_52)?;
    Ok(A45Insert {
        capacity,
        slot: (pos_off / 0x34) * 0x34,
    })
}

/// `0x005edaf0`: insert one 0x34-byte `{key, string, string}` record.
///
/// Same growth and slot arithmetic as [`a45_array52_insert_slot`]; this
/// instantiation is the one that shows what the 0x34 bytes hold: it stores the
/// caller's key dword at the new slot and then constructs two `std::string`
/// members (each 0x18 bytes) after it.  The returned record carries the slot
/// and the field offsets.
pub fn a45_array52_pair_slot(
    pos_off: u32,
    size: u32,
    capacity: u32,
) -> Result<A45PairInsert, A45LengthError> {
    let capacity = a45_grow_capacity(size, capacity, A45_MAX_ELEMENTS_52)?;
    Ok(A45PairInsert {
        capacity,
        slot: (pos_off / 0x34) * 0x34,
    })
}

/// `0x005edb14`: insert a range of 0x34-byte records.
///
/// The insertion index is `(first - begin) / 0x34`, the growth policy runs on
/// the count and capacity read from the vector, and the copied range is
/// `count` elements long, so the element past its last one sits at
/// `slot + count * 0x34`.  The routine then copies `[begin, first)` in front of
/// the range and `[first, finish)` behind it.  The two leading `*eax += eax`
/// instructions in the recovered listing are misdecoded data, not code.
pub fn a45_array52_range_slots(
    first_off: u32,
    count: u32,
    size: u32,
    capacity: u32,
) -> Result<A45RangeInsert, A45LengthError> {
    let capacity = a45_grow_capacity(size, capacity, A45_MAX_ELEMENTS_52)?;
    let slot = (first_off / 0x34) * 0x34;
    Ok(A45RangeInsert {
        capacity,
        slot,
        tail: slot + count * 0x34,
    })
}

/// `0x005ed3ab`: the register-frame growth decision and returned slot.
///
/// The decompiler started inside the caller's frame, so the growth base is the
/// sum of two registers (`size + count`), the requested capacity arrives in
/// `ebx` and the limit in `eax` (the register roles are INFERENCE from the
/// guard shape).  The policy is the guarded reserve of
/// [`a45_reserve_capacity`]; the returned pointer is
/// `begin + (target - begin) / 0x34 * 0x34`, reproduced as
/// `index * 0x34`.  This row shares the 0x34-byte array with the insertions
/// above but has no `length_error` branch: the requested capacity is already
/// legal.
pub fn a45_array52_grow_slot(index: u32, base: u32, requested: u32) -> A45Insert {
    A45Insert {
        capacity: a45_reserve_capacity(base, requested, A45_MAX_ELEMENTS_52),
        slot: index * 0x34,
    }
}

/// `0x005ed4d0`, `0x005ed660`, `0x005ed68b`: insert one 0x1c-byte record.
///
/// All three rows are the 28-byte instantiation of the same insertion: element
/// count from `(finish - begin) / 0x1c`, capacity likewise, the growth policy
/// against `0x09249249`, the slot at `(position - begin) / 0x1c * 0x1c` and the
/// release of the old range.
///
/// The payload differs: `0x005ed4d0` writes a `1` marker byte at `+0`, the
/// caller's value byte at `+1` and leaves the `std::string` at `+4` empty
/// (size at `+0x14`, capacity `0xf` at `+0x18`); `0x005ed660` and
/// `0x005ed68b` write a zero word at `+0` and copy a string into `+4`.
pub fn a45_array28_slot(
    pos_off: u32,
    size: u32,
    capacity: u32,
) -> Result<A45Insert, A45LengthError> {
    let capacity = a45_grow_capacity(size, capacity, A45_MAX_ELEMENTS_28)?;
    Ok(A45Insert {
        capacity,
        slot: (pos_off / 0x1c) * 0x1c,
    })
}

/// `0x005ed7e0`, `0x005ede40`: insert one 0x18-byte `std::string` record.
///
/// The 24-byte instantiation: the growth policy runs against `0x0aaaaaaa`, the
/// slot is `(position - begin) / 0x18 * 0x18` and the old range is released.
/// `0x005ed7e0` inserts at an arbitrary position and copies the existing
/// elements around the new string; `0x005ede40` is the `push_back` move form -
/// it copies the four leading dwords plus the trailing eight bytes and then
/// clears the source string (size 0, capacity 0xf, zero first byte).  Both
/// return the same slot.
pub fn a45_array24_slot(
    pos_off: u32,
    size: u32,
    capacity: u32,
) -> Result<A45Insert, A45LengthError> {
    let capacity = a45_grow_capacity(size, capacity, A45_MAX_ELEMENTS_24)?;
    Ok(A45Insert {
        capacity,
        slot: (pos_off / 0x18) * 0x18,
    })
}

/// `0x005edcb0`: grow a 4-byte-element array by one and size its block.
///
/// The count and capacity are `(finish - begin) >> 2` and
/// `(end - begin) >> 2`, the limit is `0x3fffffff` and a full array throws
/// `length_error`.  The growth policy picks the new capacity; the block is
/// `capacity * 4` bytes.  A capacity of `0x40000000` or more (which the policy
/// cannot produce, but the row guards for anyway) collapses the byte size to
/// `0xffffffff`.  Requests under `0x1000` bytes are handed to the small-block
/// allocator unchanged; larger ones are passed as `bytes + 0x23`, the 0x24-byte
/// chunk header less one, and any wrap-around collapses to `0xffffffff`.
///
/// The recovered small-block branch performs the element store and the two
/// range copies inline and then leaves through the allocator; only the sizing
/// arithmetic is modelled here, not the copy.
pub fn a45_array4_grow(
    pos_off: u32,
    size: u32,
    capacity: u32,
) -> Result<A45Array4Grow, A45LengthError> {
    let capacity = a45_grow_capacity(size, capacity, A45_MAX_ELEMENTS_4)?;
    let bytes = if capacity >= 0x4000_0000 {
        u32::MAX
    } else {
        capacity * 4
    };
    let request = if bytes < A45_SMALL_BLOCK_LIMIT {
        bytes
    } else {
        let rounded = bytes.wrapping_add(A45_ROUNDING);
        if rounded <= bytes { u32::MAX } else { rounded }
    };
    Ok(A45Array4Grow {
        capacity,
        slot: (pos_off >> 2) * 4,
        bytes,
        request,
    })
}

/// `0x005ede10`: install a freshly allocated 4-byte array.
///
/// The routine stores the three vector fields and returns the element at the
/// insertion offset: `begin = block`, `finish = block + count * 4`,
/// `end = block + capacity * 4` and `position = begin + offset`.  The
/// multiplication by four is recovered; that `end` counts capacity elements is
/// the standard vector layout (INFERENCE, the capacity argument itself is not
/// visible in this frame).
pub fn a45_array4_fresh_layout(block: u32, count: u32, capacity: u32, offset: u32) -> [u32; 4] {
    let begin = block;
    [
        begin,
        begin.wrapping_add(count.wrapping_mul(4)),
        begin.wrapping_add(capacity.wrapping_mul(4)),
        begin.wrapping_add(offset),
    ]
}

/// `0x005edd90`, `0x005eddde`: commit a reallocated 4-byte array.
///
/// Two copies of the same routine (they differ only in the exception cookie
/// passed to the allocator).  When the array had no storage the routine takes
/// the fresh path of [`a45_array4_fresh_layout`].  Otherwise it releases the old
/// block, sized `(old_end - old_start) & ~3` bytes, through the same small-block
/// rule as the growth row: the byte count as-is below `0x1000`, else
/// `bytes + 0x23` and `0xffffffff` on wrap-around.  Both paths return
/// `begin + offset`.
///
/// `old_start`/`old_end` are the pre-call vector fields; `None` for a zero
/// `old_start` is the fresh path.  The debug-heap block validation is not
/// modelled, as in [`a45_release_text_node`].
pub fn a45_array4_commit(
    old_start: u32,
    old_end: u32,
    block: u32,
    count: u32,
    capacity: u32,
    offset: u32,
) -> A45Array4Commit {
    let fresh = a45_array4_fresh_layout(block, count, capacity, offset);
    let released = if old_start == 0 {
        None
    } else {
        let bytes = (old_end.wrapping_sub(old_start)) & !3;
        Some(if bytes < A45_SMALL_BLOCK_LIMIT {
            bytes
        } else {
            let rounded = bytes.wrapping_add(A45_ROUNDING);
            if rounded <= bytes { u32::MAX } else { rounded }
        })
    };
    A45Array4Commit {
        begin: fresh[0],
        finish: fresh[1],
        end: fresh[2],
        position: fresh[3],
        released,
    }
}

/// `0x005ed556`: fold one character into the parser's index space.
///
/// The recovered body is `return in_AL + -0x75`, i.e. the signed distance from
/// `'u'` (`0x75`).  The parser uses it as the letter half of its dispatch key;
/// the comparison that follows lives in the caller's frame and is not part of
/// this row, so only the exact wrapping subtraction is reproduced.
pub fn a45_char_index(c: u8) -> i8 {
    (c as i8).wrapping_sub(0x75)
}

// Rows in `0x005ec000..=0x005edfff` that were not translated:
//
// * `FUN_005ec322`, `FUN_005ece7b`, `FUN_005ecee1` - restore `ExceptionList`
//   from the caller frame and jump into the unwinder; no computation and no
//   independent routine (the decompiler started mid-function).
// * `FUN_005ec618`, `FUN_005ed64f`, `FUN_005ecfaf` - recovered as
//   `halt_baddata` / `int3` / a misdecoded `*eax += eax`; not on an
//   instruction boundary.
// * `FUN_005ec1fc`, `FUN_005ec784`, `FUN_005ec7db`, `FUN_005ec86b`,
//   `FUN_005ec8a9`, `FUN_005ec909`, `FUN_005ecbe3`, `FUN_005ecc38`,
//   `FUN_005ecd10` - the flag-matcher loops.  Each iteration resolves the
//   current token through the option-description vtable, calls `FUN_005ebf50`
//   to match it, advances the token cursor by the 0x18-byte token stride and,
//   on a mismatch, concatenates the `Flag could not be matched: `/`requires ...
//   arguments` text before jumping to `__CxxThrowException`.  The cursor and
//   the option map are caller-side client objects, so nothing they compute is
//   returned; `FUN_005ec1fc` additionally appends through the 0x18-byte
//   insertion translated above as `0x005ed7e0`.
// * `FUN_005ec3dc`, `FUN_005ecdf6`, `FUN_005ece47`, `FUN_005eceda` - the
//   `invalid_command_line_syntax` message rebuild and the temporary-string
//   destructors the parser runs while unwinding: fixed-size stores (size 0,
//   capacity 0xf, zero first byte) and a tail call into the small-block
//   deallocator.  That arithmetic is already covered by
//   `0x005ed240`/`0x005ed2b0`; there is no result to return.
// * `FUN_005ecf60`, `FUN_005edd50` - exception-frame wrappers that only set up
//   the frame, tail-call a range copy/free and never return.
// * `Catch_All@005ecfc6`, `Catch_All@005ed1c0`, `Catch_All@005ed4a1`,
//   `Catch_All@005ed627`, `Catch_All@005ed7b2`, `Catch_All@005ed914`,
//   `Catch_All@005edac1`, `Catch_All@005edc81` - `__finally` funclets: free one
//   temporary range and terminate the process/exception path.
// * `FUN_005ecfe0`, `FUN_005ed108` - serialise/deserialise dispatch through the
//   client archive's virtual table (locale facet construction, `num_get`
//   vtable install, stream state fixups).  The only computable part is the
//   flag word of `0x005ed1dc`, which is translated there.

#[cfg(test)]
mod tests_agent45 {
    use super::*;

    /// The four limit literals in the recovered bodies are exactly the
    /// 32-bit element counts of their strides.
    #[test]
    fn element_limits_are_the_stride_quotients() {
        assert_eq!(A45_MAX_ELEMENTS_52, u32::MAX / 0x34);
        assert_eq!(A45_MAX_ELEMENTS_28, u32::MAX / 0x1c);
        assert_eq!(A45_MAX_ELEMENTS_24, u32::MAX / 0x18);
        assert_eq!(A45_MAX_ELEMENTS_4, u32::MAX / 4);
        assert_eq!(0x04ec_4ec4, 0x4ec4ec4);
        assert_eq!(A45_MAX_ELEMENTS_52 * 0x34, 0xffff_ffd0);
        assert_eq!(A45_MAX_ELEMENTS_28 * 0x1c, 0xffff_fffc);
        assert_eq!(A45_MAX_ELEMENTS_24 * 0x18, 0xffff_fff0);
    }

    /// `0x005ec590` and `0x005ecab0`: the tail slice is in range up to the
    /// string size, empty at the size and `out_of_range` one past it.  The
    /// `npos` distance survives unchanged.
    #[test]
    fn tail_range_clamps_at_the_string_size() {
        assert_eq!(a45_tail_range(5, 9), Ok((5, 4)));
        assert_eq!(a45_tail_range(0, 9), Ok((0, 9)));
        assert_eq!(a45_tail_range(9, 9), Ok((9, 0)));
        assert_eq!(a45_tail_range(10, 9), Err(A45RangeError));
        assert_eq!(a45_tail_range(0, 0), Ok((0, 0)));
        assert_eq!(a45_tail_range(0, u32::MAX), Ok((0, u32::MAX)));

        assert_eq!(a45_tail_range_at_descriptor(4, 6), Ok((4, 2)));
        assert_eq!(a45_tail_range_at_descriptor(7, 6), Err(A45RangeError));
    }

    /// `0x005ecf40`: exactly one byte is written; the rest of the dword is
    /// preserved and the previous low byte is reported.
    #[test]
    fn record_byte_store_is_one_byte_wide() {
        let mut slot = 0x1234_56aa;
        assert_eq!(a45_record_byte(&mut slot, 0x7f), 0xaa);
        assert_eq!(slot, 0x1234_567f);
        assert_eq!(a45_record_byte(&mut slot, 0x00), 0x7f);
        assert_eq!(slot, 0x1234_5600);
    }

    /// `0x005ed240`/`0x005ed2b0`: small-buffer nodes are never released; the
    /// request is `capacity + 1` inside the small-block range and
    /// `capacity + 0x24` above it, with both boundaries pinned.
    #[test]
    fn release_request_switches_at_the_small_block_limit() {
        assert_eq!(a45_release_text_node(0), None);
        assert_eq!(a45_release_text_node(0xe), None);
        assert_eq!(a45_release_text_node(0xf), None);
        assert_eq!(a45_release_text_node(0x10), Some(0x11));
        assert_eq!(a45_release_text_node(0xffe), Some(0xfff));
        assert_eq!(a45_release_text_node(0xfff), Some(0x1023));
        assert_eq!(a45_release_text_node(0x1000), Some(0x1024));
    }

    /// `0x005ed1dc`: bit 2 reports the missing optional object and the carried
    /// bits are merged first.
    #[test]
    fn report_flags_sets_bit_two_without_an_object() {
        assert_eq!(a45_report_flags(0x10, 0x21, false), 0x35);
        assert_eq!(a45_report_flags(0x10, 0x21, true), 0x31);
        assert_eq!(a45_report_flags(0, 0, true), 0);
        assert_eq!(a45_report_flags(4, 0, false), 4);
    }

    /// The shared policy: one element past the end, 1.5x the capacity when
    /// that covers the request, the request itself for capacities 0 and 1,
    /// and `length_error` only at the limit.
    #[test]
    fn growth_policy_doubles_then_falls_back_to_the_request() {
        let max = A45_MAX_ELEMENTS_52;
        assert_eq!(a45_grow_capacity(0, 0, max), Ok(1));
        assert_eq!(a45_grow_capacity(0, 1, max), Ok(1));
        assert_eq!(a45_grow_capacity(1, 1, max), Ok(2));
        assert_eq!(a45_grow_capacity(2, 2, max), Ok(3));
        assert_eq!(a45_grow_capacity(8, 8, max), Ok(12));
        assert_eq!(a45_grow_capacity(0, 8, max), Ok(12));
        assert_eq!(a45_grow_capacity(8, 0, max), Ok(9));
        assert_eq!(a45_grow_capacity(11, 12, max), Ok(18));

        // At the limit the throw wins over growth.
        assert_eq!(a45_grow_capacity(max, max, max), Err(A45LengthError));
        // One below the limit is still admissible and grows to the limit.
        assert_eq!(
            a45_grow_capacity(max - 1, max - 1, max),
            Ok(max),
            "the overflow guard keeps the request once doubling would pass the limit"
        );
    }

    /// `0x005ed320`: appending reports the end slot; `0x005ed940` and
    /// `0x005edf60` truncate the position to an element boundary.
    #[test]
    fn array52_slots_are_element_aligned() {
        assert_eq!(
            a45_array52_append_slot(3, 4),
            Ok(A45Insert {
                capacity: 6,
                slot: 0x9c
            })
        );
        assert_eq!(
            a45_array52_insert_slot(0x68, 3, 4),
            Ok(A45Insert {
                capacity: 6,
                slot: 0x68
            })
        );
        // The recovered `idiv` truncates an unaligned offset.
        assert_eq!(a45_array52_insert_slot(0x40, 3, 4).unwrap().slot, 0x34);
        assert_eq!(
            a45_array52_insert_slot(0, A45_MAX_ELEMENTS_52, 4),
            Err(A45LengthError)
        );
    }

    /// `0x005edaf0`: the new 0x34-byte record is `{dword, string, string}`.
    #[test]
    fn array52_pair_record_layout() {
        let insert = a45_array52_pair_slot(0x68, 3, 4).unwrap();
        assert_eq!(insert.capacity, 6);
        assert_eq!(insert.key_off(), 0x68);
        assert_eq!(insert.first_text_off(), 0x6c);
        assert_eq!(insert.second_text_off(), 0x84);
        assert_eq!(insert.second_text_off() + 0x18, insert.key_off() + 0x34);
    }

    /// `0x005edb14`: the returned range covers `count` elements from the
    /// insertion slot.
    #[test]
    fn array52_range_insert_spans_the_copied_count() {
        let range = a45_array52_range_slots(0x34, 2, 3, 4).unwrap();
        assert_eq!(range.slot, 0x34);
        assert_eq!(range.tail, 0x34 + 2 * 0x34);
        assert_eq!(range.capacity, 6);
        assert_eq!(range.tail - range.slot, 0x68);
    }

    /// `0x005ed3ab`: the reserve decision keeps the requested capacity when
    /// doubling would overflow and otherwise takes the larger of the two.
    #[test]
    fn array52_register_frame_reserve_and_slot() {
        let slot = a45_array52_grow_slot(2, 8, 8);
        assert_eq!(slot.slot, 0x68);
        assert_eq!(slot.capacity, 12);

        // base + base / 2 dominates a smaller request
        assert_eq!(a45_array52_grow_slot(0, 8, 5).capacity, 12);
        // the guard: max - base / 2 < base keeps the request verbatim
        let near = A45_MAX_ELEMENTS_52 - 1;
        assert_eq!(a45_array52_grow_slot(0, near, near).capacity, near);
        assert_eq!(a45_array52_grow_slot(0, 0, 1).capacity, 1);
    }

    /// `0x005ed4d0`, `0x005ed660`, `0x005ed68b`: 28-byte records, slot at
    /// `index * 0x1c`, with the marker/value bytes and the embedded string at
    /// `+4`, size `+0x14`, capacity `+0x18`.
    #[test]
    fn array28_slot_and_payload_offsets() {
        let insert = a45_array28_slot(0x38, 2, 3).unwrap();
        assert_eq!(insert.slot, 0x38);
        assert_eq!(insert.capacity, 4);
        assert_eq!(insert.slot + 0x1c, 0x54);
        assert_eq!(insert.slot + 0x14, 0x4c);
        assert_eq!(insert.slot + 0x18, 0x50);

        // payload of the byte form: marker 1 at +0, value at +1
        let mut record = [0u8; 0x1c];
        record[0] = 1;
        record[1] = 0x42;
        record[0x14..0x18].copy_from_slice(&0u32.to_le_bytes());
        record[0x18..0x1c].copy_from_slice(&0xfu32.to_le_bytes());
        assert_eq!(record[0], 1);
        assert_eq!(record[1], 0x42);
        assert_eq!(
            u32::from_le_bytes(record[0x14..0x18].try_into().unwrap()),
            0
        );
        assert_eq!(
            u32::from_le_bytes(record[0x18..0x1c].try_into().unwrap()),
            A45_SMALL_STRING_CAPACITY
        );

        assert_eq!(
            a45_array28_slot(0, A45_MAX_ELEMENTS_28, 0),
            Err(A45LengthError)
        );
    }

    /// `0x005ed7e0`, `0x005ede40`: 24-byte string records share the policy and
    /// slot arithmetic.
    #[test]
    fn array24_slot_is_one_string_wide() {
        let insert = a45_array24_slot(0x18, 1, 1).unwrap();
        assert_eq!(insert.slot, 0x18);
        assert_eq!(insert.capacity, 2);
        // push_back at the end uses the same formula with pos = size * 0x18.
        assert_eq!(a45_array24_slot(3 * 0x18, 3, 4).unwrap().slot, 0x48);
        assert_eq!(
            a45_array24_slot(0, A45_MAX_ELEMENTS_24, 0),
            Err(A45LengthError)
        );
    }

    /// `0x005edcb0`: the 4-byte array sizes its block at four bytes per
    /// element, adds the chunk header only above 0x1000 bytes and saturates.
    #[test]
    fn array4_growth_sizes_the_block() {
        let grow = a45_array4_grow(0, 0, 0).unwrap();
        assert_eq!(grow.capacity, 1);
        assert_eq!(grow.bytes, 4);
        assert_eq!(grow.request, 4);
        assert_eq!(grow.slot, 0);

        let grow = a45_array4_grow(0x10, 0x400, 0x400).unwrap();
        assert_eq!(grow.capacity, 0x600);
        assert_eq!(grow.bytes, 0x1800);
        assert_eq!(grow.request, 0x1823);
        assert_eq!(grow.slot, 0x10);

        // The limit throws instead of growing.
        assert_eq!(
            a45_array4_grow(0, A45_MAX_ELEMENTS_4, 0),
            Err(A45LengthError)
        );

        // capacity * 4 and bytes + 0x23 both saturate at u32::MAX.
        let grow = a45_array4_grow(0, A45_MAX_ELEMENTS_4 - 1, 0).unwrap();
        assert_eq!(grow.capacity, A45_MAX_ELEMENTS_4);
        assert_eq!(grow.bytes, 0xffff_fffc);
        assert_eq!(grow.request, u32::MAX);
    }

    /// `0x005ede10`: fresh storage installs `[begin, begin+count*4,
    /// begin+capacity*4)` and returns the element at the offset.
    #[test]
    fn array4_fresh_layout_installs_the_range() {
        assert_eq!(
            a45_array4_fresh_layout(0x1000, 3, 5, 4),
            [0x1000, 0x100c, 0x1014, 0x1004]
        );
    }

    /// `0x005edd90`/`0x005eddde`: the commit either installs fresh storage or
    /// releases the old block, and always reports the insertion position.
    #[test]
    fn array4_commit_releases_the_old_block_only_when_present() {
        let fresh = a45_array4_commit(0, 0, 0x1000, 3, 5, 4);
        assert_eq!(fresh.begin, 0x1000);
        assert_eq!(fresh.finish, 0x100c);
        assert_eq!(fresh.end, 0x1014);
        assert_eq!(fresh.position, 0x1004);
        assert_eq!(fresh.released, None);

        let small = a45_array4_commit(0x2000, 0x200c, 0x3000, 1, 2, 0);
        assert_eq!(small.released, Some(0xc));
        assert_eq!(small.position, 0x3000);

        // `(end - start) & ~3` then the small-block limit.
        let large = a45_array4_commit(0x2000, 0x3001, 0x4000, 1, 2, 0);
        assert_eq!(large.released, Some(0x1023));
        let unaligned = a45_array4_commit(0x2000, 0x2007, 0x3000, 0, 0, 0);
        assert_eq!(unaligned.released, Some(4));
    }

    /// `0x005ed556`: signed distance from `'u'`, wrapping on every byte.
    #[test]
    fn char_index_is_the_distance_from_u() {
        assert_eq!(a45_char_index(b'u'), 0);
        assert_eq!(a45_char_index(b'v'), 1);
        assert_eq!(a45_char_index(b'a'), -0x14);
        assert_eq!(a45_char_index(0x00), -0x75);
        assert_eq!(a45_char_index(0xff), (0xffu8 as i8).wrapping_sub(0x75));
    }
}
