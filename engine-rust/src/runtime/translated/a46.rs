#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent46.rs
// ===========================================================================
// Translation batch 46.  Only this file is edited by the batch.
//
// The `decompiled_uncited` rows in `0x005ee000..=0x005effff` are the tail of
// the retail client's MSVC C++ container support veneer, not gameplay code.
// Four families appear in the window:
//
// * `vector` growth and range-construction helpers for 52-byte, 24-byte and
//   4-byte elements (`0x005ee110`, `0x005ef110`, `0x005ef420`,
//   `0x005ef600`, `0x005efd60`, `0x005efe20`, `0x005efea0`, `0x005effb0`).
//   The recovered bodies contain the complete element-count arithmetic: the
//   element limit is `u32::MAX / stride`, growth is
//   `max(needed, capacity + capacity / 2)`
//   while that cannot overflow the limit, and the in-place branches compare
//   the free slot count with the inserted count before deciding between a
//   suffix shift and a tail append.  Those decisions are reproduced exactly.
// * One-byte and string-keyed `unordered_map` hashing (`0x005ee680`,
//   `0x005ee9a0`, `0x005eecc0`).  The key hash is FNV-1a 32-bit over the key
//   bytes (basis `0x811c9dc5`, prime `0x1000193`) and the bucket slot is
//   `(hash & mask) * 8`.  The string row additionally shows the MSVC small
//   string rule: `capacity > 0xf` means the bytes live behind the string's
//   first dword, otherwise in the inline buffer; only the first `length`
//   bytes take part in the hash.
// * Element bookkeeping helpers: the `std::fill` wrapper at `0x005ee250`, the
//   three-pointer initialiser at `0x005ef014`, the shared
//   `_Reallocate_grow_by` capacity policy at `0x005ef1f8`, and the two
//   `string` move loops at `0x005efc20`/`0x005efcc0`.
// * `std::_Iostream_error_category::` condition builder at `0x005ee660`.
//
// Rows in the window that are **not** translated, and why:
//
// * `FUN_005ee0e1`, `FUN_005ee7c7`, `FUN_005ee978`, `FUN_005eeb16`, `FUN_005eec97`,
//   `FUN_005eee80`, `FUN_005ef38c`, `FUN_005ef3b7`, `FUN_005efb91`, `FUN_005eff95` are
//   `Catch_All@` unwind funclets.  Every body reads a stack slot from the
//   original frame, calls an exception helper outside this window, and then
//   calls the terminate thunk; there is no self-contained arithmetic.
// * `FUN_005ee630`, `FUN_005ee675`, `FUN_005eeb30`, `FUN_005eff99` are compiler
//   thunks that restore `ExceptionList`/`FS:0` or raise `swi(3)` and never
//   return, so they have no callable behaviour to reconstruct.
// * `FUN_005eeab0`, `FUN_005eebe1`, `FUN_005eec51`, `FUN_005eefb6`, `FUN_005ef505`,
//   `FUN_005ef8e5`, `FUN_005efbb0` are tail fragments of longer routines: each
//   reads the middleware registers (`unaff_ESI`/`unaff_EDI`/`unaff_EBX`) of
//   the enclosing body, so its inputs do not exist as a standalone call.
// * `FUN_005eeb50` repeats the `0x005ee680` hash and bucket arithmetic and
//   differs only in relinking a caller-owned node instead of allocating one.
// * `FUN_005ef9b0` walks an iostream buffer through virtual calls on the
//   `basic_streambuf` vftable and has no pure arithmetic beyond the count
//   adjustment already modelled by `a46_grow_by_1_5`.
//
// Everything below is a pure free function: the rows are allocator and
// iterator plumbing with no [`crate::sim`] counterpart, so nothing here adds
// state to [`crate::sim::World`] or [`crate::sim::SimUnit`] and no method
// needs an `a46_` inherent form.  The helpers take element counts and byte
// spans in the same units as the recovered bodies, return
// [`A46LengthError`] where retail throws `length_error`, and otherwise
// report the count or capacity the retail code would have used; element
// payload copies stay with the caller because the retail rows are templated
// over the element type.

/// A retail container helper rejected its requested element count.
///
/// The original paths call the MSVC `length_error` thrower (`FUN_005fd610` or
/// the `"list<T> too long"` thrower); the reconstruction reports the rejected
/// count instead of unwinding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A46LengthError;

/// Element limit for 52-byte elements (`0x04ec4ec4`), the bound tested by
/// `0x005ee110` and `0x005ef600`.
pub const A46_MAX_ELEMS_52: usize = 0x04ec_4ec4;

/// Element limit for 24-byte elements (`0x00aaaaaaa`), the bound tested by
/// every 24-byte range row in the window.
pub const A46_MAX_ELEMS_24: usize = 0x0aaa_aaaa;

/// Element limit for 4-byte elements (`0x03fffffff`), the largest count the
/// dword range rows accept below their `0x40000000` test.
pub const A46_MAX_ELEMS_U32: usize = 0x3fff_ffff;

/// Element limit for a container whose elements are `stride` bytes wide.
///
/// Every range-construction row in this window tests its count against
/// `u32::MAX / stride`, i.e. against the largest count whose byte size still
/// fits in the 32-bit address space:
///
/// * stride 52 -> `0x04ec4ec4` (`0x005ee110`, `0x005ef600`);
/// * stride 24 -> `0x00aaaaaaa` (`0x005ef110`, `0x005ef420`, `0x005efd60`,
///   `0x005effb0`, `0x005efea0`);
/// * stride 4 -> `0x03fffffff` (`0x005efe20`).
pub fn a46_max_elements(stride: usize) -> usize {
    assert!(stride > 1, "the retail rows all use a multi-byte stride");
    u32::MAX as usize / stride
}

/// Element count of a byte span, with the retail limit check.
///
/// `bytes` is the recovered `last - first` byte difference, `stride` the
/// element size and `max_elems` the limit the row compares against.  A span
/// shorter than one element yields zero (no allocation); a count above the
/// limit is the `length_error` path.
pub fn a46_range_count(
    bytes: usize,
    stride: usize,
    max_elems: usize,
) -> Result<usize, A46LengthError> {
    let count = bytes / stride;
    if count > max_elems {
        Err(A46LengthError)
    } else {
        Ok(count)
    }
}

/// FNV-1a over a byte string, 32-bit.
///
/// `0x005eecc0` hashes a `basic_string` key with exactly this loop: start at
/// `0x811c9dc5`, then for each of the first `length` bytes do
/// `hash = (byte ^ hash) * 0x1000193` in 32-bit wrapping arithmetic.  The
/// single-byte rows (`0x005ee680`, `0x005ee9a0`) are the one-iteration form,
/// `(key ^ 0x811c9dc5) * 0x1000193`.
pub fn a46_fnv1a32(bytes: &[u8]) -> u32 {
    const OFFSET_BASIS: u32 = 0x811c_9dc5;
    const PRIME: u32 = 0x0100_0193;
    let mut hash = OFFSET_BASIS;
    for &byte in bytes {
        hash = (u32::from(byte) ^ hash).wrapping_mul(PRIME);
    }
    hash
}

/// Bucket slot byte offset for a key hash.
///
/// All three hash rows in the window scale the masked hash by eight:
/// `slot = (hash & mask) * 8`, where the map's `mask` is `bucket_count - 1`.
/// The result is the byte offset of the `{first, last}` bucket pair inside the
/// bucket vector at `map+0x0c`.
pub fn a46_bucket_offset(hash: u32, mask: u32) -> usize {
    ((hash & mask) as usize) * 8
}

/// `0x005ee680` (`FUN_005ee680`, ledger size 295): bucket slot for the
/// one-byte-key `unordered_map` emplace.
///
/// The row computes the key hash, masks it with the map's bucket mask and
/// scales the result by the eight-byte bucket record.  The rest of the body
/// splices the newly allocated node into the intrusive list, which is pointer
/// bookkeeping over the caller's allocator layout; the arithmetic recovered
/// here is the hash and the bucket address, and the caller supplies the
/// `mask` exactly as the retail map does.
pub fn a46_byte_key_bucket(key: u8, mask: u32) -> usize {
    let hash = (u32::from(key) ^ 0x811c_9dc5).wrapping_mul(0x0100_0193);
    a46_bucket_offset(hash, mask)
}

/// `0x005ee9a0` (`FUN_005ee9a0`, ledger size 280): bucket-chain scan for the
/// one-byte key before an emplace.
///
/// `order` is the map's iteration order (`head.next` first), `bucket` the
/// bucket's `(first, last)` node indices, or `None` for the empty bucket whose
/// both links point at the list head.  The recovered loop starts at the node
/// after `last`, tests that node against `first` (the "not present" exit),
/// advances and compares the byte key stored at node `+8`.  The compared
/// indices therefore run from `last + 2` through `first` (inclusive, wrapping
/// through zero) and the loop exits at `first` without a further compare.  A
/// hit is returned as an index into `order`; `None` is the not-found path that
/// goes on to allocate a node.
pub fn a46_byte_key_scan(order: &[u8], key: u8, bucket: Option<(usize, usize)>) -> Option<usize> {
    let (first, last) = bucket?;
    let len = order.len();
    if len == 0 || first >= len || last >= len {
        return None;
    }
    let mut cursor = (last + 1) % len;
    loop {
        if cursor == first {
            return None;
        }
        cursor = (cursor + 1) % len;
        if order[cursor] == key {
            return Some(cursor);
        }
    }
}

/// `0x005eecc0` (`FUN_005eecc0`, ledger size 458): bucket slot for the
/// string-keyed `unordered_map` emplace.
///
/// The row hashes the key with the FNV-1a loop and masks the hash with the
/// map's bucket mask.  `length` is the string's stored size (the dword at
/// string `+0x10`); only those bytes are hashed, so spare capacity does not
/// change the slot.  The small-string decision in the body - `capacity > 0xf`
/// selects the heap pointer at string `+0`, otherwise the inline buffer - only
/// changes where the bytes are read from, not the arithmetic, so the caller
/// passes the byte view and the capacity is documented here rather than
/// stored.
pub fn a46_string_key_bucket(text: &[u8], length: usize, mask: u32) -> usize {
    let hashed = &text[..length.min(text.len())];
    a46_bucket_offset(a46_fnv1a32(hashed), mask)
}

/// `0x005ee250` (`FUN_005ee250`, ledger size 117): `std::fill` over a dword
/// range.
///
/// The decompiled count is `(end + 3 - begin) >> 2` over the byte difference,
/// with a reversed range clamped to zero, and the value is stored `count`
/// times from `begin`.  The `+3` is the rounding the 32-bit pointer difference
/// needs for an unaligned end pointer, so the argument pair stays in bytes
/// here.  The write count is returned; `span` must cover the range, exactly
/// like the retail pointer pair.
pub fn a46_fill_dword_span(
    span: &mut [u32],
    begin_bytes: usize,
    end_bytes: usize,
    value: u32,
) -> usize {
    let count = if end_bytes < begin_bytes {
        0
    } else {
        (end_bytes + 3 - begin_bytes) >> 2
    };
    let start = begin_bytes / 4;
    assert!(
        start + count <= span.len(),
        "span does not cover the retail fill range"
    );
    span[start..start + count].fill(value);
    count
}

/// `0x005ee660` (`FUN_005ee660`, ledger size 19): build the
/// `_Iostream_error_category` `error_condition`.
///
/// The recovered body ignores its first argument, stores the category object
/// (the `_Iostream_error_category` vftable) in the first output dword, stores
/// the constant `5` in the second and returns `1`.  The category identity is
/// represented by [`A46ErrorCategory::Iostream`]; the numeric payload is what
/// the retail routine actually writes, so it is kept verbatim.
pub fn a46_iostream_error_condition(errval: u32, out: &mut A46ErrorCondition) -> bool {
    let _ = errval;
    out.category = A46ErrorCategory::Iostream;
    out.value = 5;
    true
}

/// The two `error_condition` dwords written by `0x005ee660`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A46ErrorCondition {
    /// Category object identity (retail stores the category vftable).
    pub category: A46ErrorCategory,
    /// Numeric condition value.
    pub value: i32,
}

/// Categories observed in the batch window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A46ErrorCategory {
    /// `std::_Iostream_error_category`.
    Iostream,
}

/// `0x005ef014` (`FUN_005ef014`, ledger size 32): finish a fresh dword
/// allocation.
///
/// The funclet stores the same base pointer into all three vector slots and
/// then adds the two element counts scaled by the four-byte stride:
/// `begin = base`, `end = base + filled * 4`, `capacity_end = base + cap * 4`.
/// The additions wrap in the 32-bit address space of the client, so they are
/// written with [`u32::wrapping_add`].
pub fn a46_dword_vector_triple(base: u32, filled: usize, capacity: usize) -> (u32, u32, u32) {
    let filled_bytes = (filled as u32).wrapping_mul(4);
    let capacity_bytes = (capacity as u32).wrapping_mul(4);
    (
        base,
        base.wrapping_add(filled_bytes),
        base.wrapping_add(capacity_bytes),
    )
}

/// `0x005ee110` (`FUN_005ee110`, ledger size 314): capacity chosen when a
/// 52-byte-element vector inserts one element at an arbitrary position.
///
/// The body divides the vector's three pointers by `0x34` to get the element
/// count and capacity, rejects a full vector (`size == 0x04ec4ec4`) on the
/// `length_error` path, and otherwise grows the capacity to
/// `max(needed, capacity + capacity / 2)` while the half-capacity addition
/// cannot pass the element limit; once it can, only the needed slot is
/// reserved.  The reconstruction returns that chosen capacity in elements;
/// retail spends it on the allocation and returns the new slot pointer at the
/// caller's insertion index.
pub fn a46_insert_capacity_52(size: usize, capacity: usize) -> Result<usize, A46LengthError> {
    if size == A46_MAX_ELEMS_52 {
        return Err(A46LengthError);
    }
    let needed = size + 1;
    let grown = if capacity <= A46_MAX_ELEMS_52 - (capacity >> 1) {
        (capacity + (capacity >> 1)).max(needed)
    } else {
        needed
    };
    Ok(grown)
}

/// Branch chosen by the 24-byte and 52-byte element insert helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A46InsertPlan {
    /// The source range was empty; the routine returns without touching the
    /// vector.
    NoOp,
    /// The free slots after the end are fewer than the inserted count; the
    /// vector reallocates and `new_capacity` is the new element capacity
    /// (`max(needed, capacity + capacity / 2)` while the growth cannot pass
    /// the element limit).
    Reallocate {
        /// New element capacity chosen by the growth policy.
        new_capacity: usize,
    },
    /// The suffix at the insertion point is longer than the inserted count:
    /// the last `suffix` elements are constructed past the old end, the range
    /// `[pos, size - suffix)` is moved backwards and the overwritten elements
    /// are destroyed.
    ShiftSuffix {
        /// Number of elements in the suffix after `pos`.
        suffix: usize,
    },
    /// The suffix is no longer than the inserted count: it is constructed at
    /// `pos + count`, the destroyed prefix covers `[pos, size)`.
    GrowTail {
        /// Number of elements in the suffix after `pos`.
        suffix: usize,
    },
}

/// Shared decision table for `0x005ef110`, `0x005ef420` and `0x005ef600`.
///
/// `size` is the live element count, `capacity` the allocated count, `pos` the
/// insertion index, `count` the source element count and `stride` the element
/// size that fixes the limit.  The free-slot test is
/// `capacity - size < count`; the length check is
/// `count > max_elems - size`; the in-place test is
/// `count < size - pos` (suffix shift) against the tail append.
fn a46_plan_insert(
    size: usize,
    capacity: usize,
    pos: usize,
    count: usize,
    stride: usize,
) -> Result<A46InsertPlan, A46LengthError> {
    let max_elems = a46_max_elements(stride);
    if count == 0 {
        return Ok(A46InsertPlan::NoOp);
    }
    let free = capacity.saturating_sub(size);
    if free < count {
        if size > max_elems || count > max_elems - size {
            return Err(A46LengthError);
        }
        let needed = size + count;
        let grown = if capacity <= max_elems - (capacity >> 1) {
            (capacity + (capacity >> 1)).max(needed)
        } else {
            needed
        };
        return Ok(A46InsertPlan::Reallocate {
            new_capacity: grown,
        });
    }
    let suffix = size.saturating_sub(pos);
    if count < suffix {
        Ok(A46InsertPlan::ShiftSuffix { suffix })
    } else {
        Ok(A46InsertPlan::GrowTail { suffix })
    }
}

/// `0x005ef110` (`FUN_005ef110`, ledger size 294): insert a counted range of
/// 24-byte elements, constructing them into the destination.
///
/// The body chooses between reallocation, a suffix shift and a tail append
/// exactly as recorded, then fills the insertion point with the copy
/// primitive `func_0x005f0360` (uninitialised copy construction).  The plan
/// below is the recovered branch and capacity arithmetic; the element copies
/// themselves are the caller's payload.
pub fn a46_insert_plan_24(
    size: usize,
    capacity: usize,
    pos: usize,
    count: usize,
) -> Result<A46InsertPlan, A46LengthError> {
    a46_plan_insert(size, capacity, pos, count, 0x18)
}

/// `0x005ef420` (`FUN_005ef420`, ledger size 368): insert a counted range of
/// 24-byte elements by assignment.
///
/// The row is the assignment twin of `0x005ef110`: the recovered growth and
/// branch arithmetic is the same (its free-slot test is the equivalent
/// `count <= capacity - size`, the complement of the reallocation guard), and
/// the difference is the fill primitive - `FUN_005f03f0` copy-assigns into the
/// already-live prefix instead of constructing into raw storage.  The plan is
/// therefore shared, and the equality boundary (free slots exactly equal to
/// the inserted count) stays in place in both rows.
pub fn a46_insert_plan_24_assign(
    size: usize,
    capacity: usize,
    pos: usize,
    count: usize,
) -> Result<A46InsertPlan, A46LengthError> {
    a46_plan_insert(size, capacity, pos, count, 0x18)
}

/// `0x005ef600` (`FUN_005ef600`, ledger size 731): insert a counted range of
/// 52-byte elements.
///
/// The row is the 52-byte twin of `0x005ef110`: the source count is the byte
/// span over `0x34`, the vector's free slots are compared with that count, the
/// element limit is `0x04ec4ec4` and the reallocation growth is
/// `max(needed, capacity + capacity / 2)` bounded by that limit.  The same
/// in-place decision tree applies - a suffix shift when the tail past the
/// insertion point is longer than the inserted run, otherwise a tail append -
/// and the destination fill is the move/copy helper pair used by the row.
pub fn a46_insert_plan_52(
    size: usize,
    capacity: usize,
    pos: usize,
    count: usize,
) -> Result<A46InsertPlan, A46LengthError> {
    a46_plan_insert(size, capacity, pos, count, 0x34)
}

/// `0x005ef1f8` (`FUN_005ef1f8`, ledger size 173): the
/// `_Reallocate_grow_by` capacity policy.
///
/// The funclet computes `needed = size + count` and then:
/// `max_elems - needed / 2 < needed` (the 1.5x growth would pass the element
/// limit) returns the caller's old capacity unchanged, otherwise it returns
/// `max(old_capacity, needed + needed / 2)`.  The comparison is unsigned and
/// the multiplication-free `needed / 2` form is kept so the boundary is
/// identical.
pub fn a46_grow_by_1_5(needed: usize, old_capacity: usize, max_elems: usize) -> usize {
    if max_elems.saturating_sub(needed >> 1) < needed {
        old_capacity
    } else {
        (needed + (needed >> 1)).max(old_capacity)
    }
}

/// `0x005efc20` (`FUN_005efc20`, ledger size 143): move a `basic_string`
/// range backwards.
///
/// The routine walks from the end of the source range and move-assigns each
/// 24-byte string into the destination, leaving the source an empty MSVC
/// small string (`size = 0`, `capacity = 0xf`, first byte zero).  The
/// per-element guard skips the copy when the source and destination element
/// pointers coincide; that is `ranges_coincide` here, which is only reachable
/// when both spans start at the same offset.  The returned value is the number
/// of elements moved; the retail return is the destination start pointer.
pub fn a46_move_strings_backward(
    src: &mut [String],
    dst: &mut [String],
    ranges_coincide: bool,
) -> usize {
    let count = src.len().min(dst.len());
    if !ranges_coincide {
        for index in (0..count).rev() {
            dst[index] = std::mem::take(&mut src[index]);
        }
    }
    count
}

/// `0x005efcc0` (`FUN_005efcc0`, ledger size 146): move a `basic_string`
/// range forwards.
///
/// Same 24-byte move-assignment as `0x005efc20` in the opposite direction;
/// the row is the forward loop (`destination += 24` until it reaches the end
/// of the source span) and resets each source string to the empty small
/// string.  The coincident-range guard is reproduced by `ranges_coincide`.
pub fn a46_move_strings_forward(
    src: &mut [String],
    dst: &mut [String],
    ranges_coincide: bool,
) -> usize {
    let count = src.len().min(dst.len());
    if !ranges_coincide {
        for index in 0..count {
            dst[index] = std::mem::take(&mut src[index]);
        }
    }
    count
}

/// `0x005efd60` (`FUN_005efd60`, ledger size 167): range-construct a
/// 24-byte-element vector with a trivial element copy.
///
/// `byte_len` is the recovered `last - first` span.  The element count is
/// `byte_len / 0x18`, the capacity is set to that count, and the copy is
/// `func_0x005f0360` (a byte-wise block copy, i.e. the trivially-copyable
/// path).  A count above `0x0aaaaaaa` is the `length_error` path and a zero
/// span leaves the three slots null.
pub fn a46_vector_from_range_24(byte_len: usize) -> Result<usize, A46LengthError> {
    a46_range_count(byte_len, 0x18, a46_max_elements(0x18))
}

/// `0x005effb0` (`FUN_005effb0`, ledger size 210): range-construct a
/// 24-byte-element vector with a per-element copy constructor.
///
/// Same span arithmetic and limit as `0x005efd60`
/// (`count = byte_len / 0x18`, max `0x0aaaaaaa`, capacity = count), but the
/// body constructs every element through `FUN_005f12e0` instead of the block
/// copy, so the element type is not trivially copyable.  The reconstructed
/// helper reports the count that the allocation and the constructor loop use.
pub fn a46_vector_from_range_24_ctor(byte_len: usize) -> Result<usize, A46LengthError> {
    a46_range_count(byte_len, 0x18, a46_max_elements(0x18))
}

/// `0x005efe20` (`FUN_005efe20`, ledger size 111): range-construct a
/// dword vector.
///
/// The count is the raw span shifted right by two (`byte_len >> 2`), the
/// limit is strictly below `0x40000000` (the row's `< 0x40000000` test is the
/// same as `count <= 0x3fffffff`), allocation is `count * 4` bytes and the
/// fill is a `FUN_00c89b00` block move of the whole span.  The returned count
/// is what both the allocator and the move use.
pub fn a46_vector_from_range_u32(byte_len: usize) -> Result<usize, A46LengthError> {
    a46_range_count(byte_len, 4, a46_max_elements(4))
}

/// `0x005efea0` (`FUN_005efea0`, ledger size 234): build a `vector<string>`
/// from a `const char*` range.
///
/// The source span is an array of four-byte pointers, so the count is
/// `pointer_bytes >> 2`; the output elements are 24-byte strings, so the limit
/// is `0x0aaaaaaa`, the capacity equals the count and each element is
/// constructed empty and then assigned from a NUL-terminated source (the
/// length is `strlen`, the `FUN_005fdb20` assignment).  `sources` supplies the
/// pointed-to text and its length stands in for the pointer-array span.
pub fn a46_string_vector_from_cstr_range(sources: &[&str]) -> Result<Vec<String>, A46LengthError> {
    let pointer_bytes = sources.len().wrapping_mul(4);
    let count = a46_range_count(pointer_bytes, 4, a46_max_elements(0x18))?;
    Ok(sources[..count]
        .iter()
        .map(|text| (*text).to_string())
        .collect())
}

#[cfg(test)]
mod tests_agent46 {
    use super::*;

    /// `0x005ee110`: the growth policy returns the next capacity in
    /// elements for 52-byte elements, including the limit boundary.
    #[test]
    fn insert_capacity_52_follows_retail_growth() {
        assert_eq!(a46_insert_capacity_52(0, 0), Ok(1));
        assert_eq!(a46_insert_capacity_52(0, 4), Ok(6));
        assert_eq!(a46_insert_capacity_52(4, 8), Ok(12));
        assert_eq!(a46_insert_capacity_52(11, 12), Ok(18));
        // A capacity that cannot grow by half only gets the needed slot.
        assert_eq!(a46_insert_capacity_52(5, A46_MAX_ELEMS_52 - 1), Ok(6));
        // The last representable element count can still grow by one.
        assert_eq!(
            a46_insert_capacity_52(A46_MAX_ELEMS_52 - 1, A46_MAX_ELEMS_52),
            Ok(A46_MAX_ELEMS_52)
        );
        // A full vector is the length_error path.
        assert_eq!(
            a46_insert_capacity_52(A46_MAX_ELEMS_52, 0),
            Err(A46LengthError)
        );
    }

    /// `0x005ee250`: the dword fill count is the byte span over four, with
    /// the reversed range writing nothing.
    #[test]
    fn fill_dword_span_writes_only_the_span() {
        let mut words = [0u32; 4];
        assert_eq!(a46_fill_dword_span(&mut words, 4, 16, 0xdead_beef), 3);
        assert_eq!(words, [0, 0xdead_beef, 0xdead_beef, 0xdead_beef]);

        let mut words = [7u32; 3];
        assert_eq!(a46_fill_dword_span(&mut words, 12, 0, 1), 0);
        assert_eq!(words, [7, 7, 7]);

        // The `+3` rounding cannot invent an extra dword for an aligned end.
        let mut words = [0u32; 2];
        assert_eq!(a46_fill_dword_span(&mut words, 0, 8, 9), 2);
        assert_eq!(words, [9, 9]);
    }

    /// `0x005ee660`: the condition stores the iostream category and the
    /// literal value 5, and reports success for every input.
    #[test]
    fn iostream_error_condition_is_constant() {
        let mut condition = A46ErrorCondition {
            category: A46ErrorCategory::Iostream,
            value: -1,
        };
        assert!(a46_iostream_error_condition(0, &mut condition));
        assert_eq!(condition.value, 5);

        assert!(a46_iostream_error_condition(0xffff_ffff, &mut condition));
        assert_eq!(condition.category, A46ErrorCategory::Iostream);
        assert_eq!(condition.value, 5);
    }

    /// `0x005eecc0`/`0x005ee680`: the key hash is standard FNV-1a 32-bit and
    /// the bucket slot is the masked hash scaled by eight.
    #[test]
    fn key_hash_matches_fnv1a_and_bucket_slots() {
        assert_eq!(a46_fnv1a32(b""), 0x811c_9dc5);
        assert_eq!(a46_fnv1a32(b"a"), 0xe40c_292c);
        assert_eq!(a46_fnv1a32(b"b"), 0xe70c_2de5);
        assert_eq!(a46_fnv1a32(b"foobar"), 0xbf9c_f968);
        assert_eq!(a46_fnv1a32(b"hello"), 0x4f9f_2cab);

        // Single-byte rows are the one-iteration FNV-1a form.
        assert_eq!(a46_byte_key_bucket(0x41, 0x1f), 0x60);
        assert_eq!(a46_byte_key_bucket(0x41, 0xff), 0x660);
        assert_eq!(a46_byte_key_bucket(0xff, 0x1f), 0x70);
        assert_eq!(a46_byte_key_bucket(0x00, 0xff), 0xf8);
        assert_eq!(a46_byte_key_bucket(0x01, 0x1f), 0x60);
    }

    /// `0x005eecc0`: only the stored length is hashed, and the masks select
    /// the low bucket bits.
    #[test]
    fn string_key_bucket_hashes_the_stored_prefix() {
        assert_eq!(a46_string_key_bucket(b"hello world", 5, 0xff), 0x558);
        assert_eq!(a46_string_key_bucket(b"supply depot", 12, 0x3f), 0xa0);
        assert_eq!(
            a46_string_key_bucket(b"supply depot", 0, 0x3f),
            a46_bucket_offset(0x811c_9dc5, 0x3f)
        );
    }

    /// `0x005ee9a0`: the scan starts after the bucket's last node, stops at
    /// its first node and compares the byte stored at node `+8`.
    #[test]
    fn byte_key_scan_matches_the_recovered_loop() {
        let order = [b'a', b'b', b'c', b'd'];
        // Bucket (first = 3, last = 0) scans nodes 2 and 3.
        assert_eq!(a46_byte_key_scan(&order, b'c', Some((3, 0))), Some(2));
        assert_eq!(a46_byte_key_scan(&order, b'd', Some((3, 0))), Some(3));
        // The start node itself and the nodes inside the bucket are not
        // compared before the `first` exit.
        assert_eq!(a46_byte_key_scan(&order, b'a', Some((3, 0))), None);
        assert_eq!(a46_byte_key_scan(&order, b'b', Some((3, 0))), None);
        // The empty bucket has both links on the list head: immediate miss.
        assert_eq!(a46_byte_key_scan(&order, b'a', None), None);
        // A bucket covering node 0 still wraps past the start node and can
        // hit the first node itself.
        assert_eq!(a46_byte_key_scan(&order, b'a', Some((0, 0))), Some(0));
        assert_eq!(a46_byte_key_scan(&order, b'd', Some((0, 0))), Some(3));
    }

    /// `0x005ef014`: the fresh-allocation triple scales both counts by the
    /// four-byte stride, wrapping in 32 bits.
    #[test]
    fn dword_vector_triple_uses_four_byte_stride() {
        assert_eq!(
            a46_dword_vector_triple(0x1000, 3, 5),
            (0x1000, 0x100c, 0x1014)
        );
        assert_eq!(
            a46_dword_vector_triple(0xffff_fff8, 2, 4),
            (0xffff_fff8, 0x0, 0x8)
        );
    }

    /// `0x005ef110`/`0x005ef420`: branch selection and capacity growth for
    /// 24-byte elements.
    #[test]
    fn insert_plan_24_selects_the_recovered_branch() {
        // Empty source range is a no-op.
        assert_eq!(a46_insert_plan_24(3, 8, 1, 0), Ok(A46InsertPlan::NoOp));

        // Free slots < count -> reallocate with 1.5x growth.
        assert_eq!(
            a46_insert_plan_24(3, 3, 3, 2),
            Ok(A46InsertPlan::Reallocate { new_capacity: 5 })
        );
        assert_eq!(
            a46_insert_plan_24(10, 16, 10, 7),
            Ok(A46InsertPlan::Reallocate { new_capacity: 24 })
        );
        // Growth that would pass the limit falls back to the needed count.
        assert_eq!(
            a46_insert_plan_24(0xaaaa_aaa - 2, 0xaaaa_aaa - 1, 0xaaaa_aaa - 2, 2),
            Ok(A46InsertPlan::Reallocate {
                new_capacity: 0xaaaa_aaa
            })
        );

        // Suffix longer than the inserted run -> suffix shift.
        assert_eq!(
            a46_insert_plan_24(5, 8, 1, 2),
            Ok(A46InsertPlan::ShiftSuffix { suffix: 4 })
        );

        // Suffix shorter or equal -> tail append, including the equality
        // boundary the assignment row compares with `count <= free`.
        assert_eq!(
            a46_insert_plan_24(3, 5, 3, 2),
            Ok(A46InsertPlan::GrowTail { suffix: 0 })
        );
        assert_eq!(
            a46_insert_plan_24_assign(3, 5, 3, 2),
            Ok(A46InsertPlan::GrowTail { suffix: 0 })
        );
        assert_eq!(
            a46_insert_plan_24(10, 16, 4, 5),
            Ok(A46InsertPlan::ShiftSuffix { suffix: 6 })
        );

        // A full vector of 24-byte elements rejects one more element.
        assert_eq!(
            a46_insert_plan_24(0xaaaa_aaa, 0xaaaa_aaa, 0, 1),
            Err(A46LengthError)
        );
    }

    /// `0x005ef600`: the same decision table for 52-byte elements, with the
    /// `0x04ec4ec4` limit and the `0x34` element stride.
    #[test]
    fn insert_plan_52_uses_the_52_byte_limit() {
        assert_eq!(
            a46_insert_plan_52(0, 0, 0, 1),
            Ok(A46InsertPlan::Reallocate { new_capacity: 1 })
        );
        assert_eq!(
            a46_insert_plan_52(4, 4, 4, 2),
            Ok(A46InsertPlan::Reallocate { new_capacity: 6 })
        );
        assert_eq!(
            a46_insert_plan_52(6, 10, 1, 2),
            Ok(A46InsertPlan::ShiftSuffix { suffix: 5 })
        );
        assert_eq!(
            a46_insert_plan_52(6, 10, 6, 3),
            Ok(A46InsertPlan::GrowTail { suffix: 0 })
        );
        assert_eq!(a46_insert_plan_52(3, 8, 1, 0), Ok(A46InsertPlan::NoOp));
        // One element past the 52-byte limit is rejected.
        assert_eq!(
            a46_insert_plan_52(0x4ec_4ec4, 0x4ec_4ec4, 0, 1),
            Err(A46LengthError)
        );
        // The last representable capacity still accepts an append.
        assert_eq!(
            a46_insert_plan_52(0x4ec_4ec3, 0x4ec_4ec4, 0x4ec_4ec3, 1),
            Ok(A46InsertPlan::GrowTail { suffix: 0 })
        );
    }

    /// `0x005ef1f8`: the `_Reallocate_grow_by` fallback returns the old
    /// capacity when `needed + needed / 2` would pass the limit.
    #[test]
    fn grow_by_1_5_matches_the_funclet() {
        assert_eq!(a46_grow_by_1_5(4, 0, 0x3fff_ffff), 6);
        assert_eq!(a46_grow_by_1_5(4, 16, 0x3fff_ffff), 16);
        assert_eq!(a46_grow_by_1_5(1, 0, 0x3fff_ffff), 1);
        // needed = 0x30000000: 1.5x needed would exceed the limit, so the
        // caller's old capacity survives untouched.
        assert_eq!(a46_grow_by_1_5(0x3000_0000, 7, 0x3fff_ffff), 7);
        // Just below that boundary the growth is exactly 1.5x needed.
        assert_eq!(a46_grow_by_1_5(0x2000_0000, 7, 0x3fff_ffff), 0x3000_0000);
    }

    /// `0x005efc20`/`0x005efcc0`: the two move loops empty their sources and
    /// leave both ranges untouched when the element pointers coincide.
    #[test]
    fn string_moves_forward_and_backward() {
        let mut src = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let mut dst = vec![String::new(); 3];
        assert_eq!(a46_move_strings_backward(&mut src, &mut dst, false), 3);
        assert_eq!(dst, ["a", "b", "c"]);
        assert!(src.iter().all(String::is_empty));

        let mut src = vec!["x".to_string(), "y".to_string()];
        let mut dst = vec![String::new(); 2];
        assert_eq!(a46_move_strings_forward(&mut src, &mut dst, false), 2);
        assert_eq!(dst, ["x", "y"]);
        assert!(src.iter().all(String::is_empty));

        let mut src = vec!["keep".to_string()];
        let mut dst = vec!["old".to_string()];
        assert_eq!(a46_move_strings_forward(&mut src, &mut dst, true), 1);
        assert_eq!(src, ["keep"]);
        assert_eq!(dst, ["old"]);
    }

    /// `0x005efd60`/`0x005effb0`: the 24-byte range count and its limit.
    #[test]
    fn range_counts_24_use_eighteen_byte_stride() {
        assert_eq!(a46_vector_from_range_24(0), Ok(0));
        assert_eq!(a46_vector_from_range_24(23), Ok(0));
        assert_eq!(a46_vector_from_range_24(0x18), Ok(1));
        assert_eq!(a46_vector_from_range_24(0x18 * 7), Ok(7));
        assert_eq!(a46_vector_from_range_24_ctor(0x18 * 7), Ok(7));
        assert_eq!(a46_vector_from_range_24(0x18 * 0xaaaa_aaa), Ok(0xaaaa_aaa));
        assert_eq!(
            a46_vector_from_range_24(0x18 * 0xaaaa_aab),
            Err(A46LengthError)
        );
        assert_eq!(
            a46_vector_from_range_24_ctor(0x18 * 0xaaaa_aab),
            Err(A46LengthError)
        );
    }

    /// `0x005efe20`: dword range counts and the strict `0x40000000` limit.
    #[test]
    fn range_counts_u32_use_four_byte_stride() {
        assert_eq!(a46_vector_from_range_u32(0), Ok(0));
        assert_eq!(a46_vector_from_range_u32(3), Ok(0));
        assert_eq!(a46_vector_from_range_u32(16), Ok(4));
        assert_eq!(a46_vector_from_range_u32(4 * 0x3fff_ffff), Ok(0x3fff_ffff));
        assert_eq!(
            a46_vector_from_range_u32(4 * 0x4000_0000),
            Err(A46LengthError)
        );
    }

    /// `0x005efea0`: the `const char*` range is counted at four bytes per
    /// pointer and each output string copies the NUL-terminated source.
    #[test]
    fn string_vector_from_cstr_range_copies_sources() {
        let built = a46_string_vector_from_cstr_range(&["alpha", "", "beta"]).unwrap();
        assert_eq!(built, ["alpha", "", "beta"]);
        assert!(a46_string_vector_from_cstr_range(&[]).unwrap().is_empty());
    }

    /// The per-stride element limits are the ones encoded in the window.
    #[test]
    fn element_limits_match_the_rows() {
        assert_eq!(a46_max_elements(0x34), 0x04ec_4ec4);
        assert_eq!(a46_max_elements(0x18), 0x0aaa_aaaa);
        assert_eq!(a46_max_elements(4), 0x3fff_ffff);
    }
}
