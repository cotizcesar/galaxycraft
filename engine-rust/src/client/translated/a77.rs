#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent77.rs
// ===========================================================================
// Translation batch 77.  Only this file is edited by the batch.
//
// Batch 77 owns `FUN_0062c000..=0x0062dfff`.  The ledger lists 78
// `decompiled_uncited` rows in that window; 26 rows are translated below in
// 23 address groups (three rows repeat a body at a second address, and the
// shared arithmetic is factored into a few helpers), and the 52 rows that
// are not are listed at the end with their reason.
//
// The window is a C++ UI/research module: `ObserverUI::Player` research and
// upgrade state, three texture-sampler constructors, a chained-record lookup
// and a lot of compiler-generated vector/lambda/SEH glue.  Most rows are
// exception-unwind fragments whose visible stores address state through
// `unaff_EBP`/`unaff_ESI` frame slots of a caller outside the window, so they
// cannot be given a self-contained signature.  The rows that do carry
// standalone work are reproduced here as exact arithmetic over the dwords the
// bodies touch.
//
// Recovered arithmetic:
//
// | row | recovered work |
// |-----|----------------|
// | `0x0062c330` | `+0x0c` dword of an element is non-zero |
// | `0x0062c3b0` | `(key & 0xffffff00) | found`, found = `key == head && tail == 0` |
// | `0x0062c512` | `ObserverUI` header: vtable word, three zero words, zero byte at `+0x40` |
// | `0x0062c8db`, `0x0062c916` | indexed state slot `index` or `(index & 0xffffff7f) + 8`, marker store, `(int)(index+1) < 0x84` guard |
// | `0x0062c9c3`, `0x0062cbf3` | 0x24-byte state entry at byte `0x1b0 + 0x24*index`, kind byte at `+8` |
// | `0x0062c7fb` | research state word `(flag ^ 1) + 1` |
// | `0x0062ca60` | first record of `0x68` bytes whose key dword matches |
// | `0x0062cad0` | first `0x14`-byte element accepted by an indirect predicate |
// | `0x0062cfd0` | singleton slot present |
// | `0x0062d3b2`, `0x0062d3e5` | approach-to-target by `+ trunc(delta/16)`, clamp at 0, half-ceil sample sums |
// | `0x0062d5b0`, `0x0062d670` | move-assign of a 16-byte / 20-byte element vector |
// | `0x0062d624`, `0x0062d6f7` | clone of a 16-byte / 20-byte element vector |
// | `0x0062d750` | `(cap ^ grown) & 0x7fffffff ^ cap` growth policy |
// | `0x0062d900` | table-byte to upgrade-code switch |
// | `0x0062db60`, `0x0062dc30`, `0x0062dd00` | sampler header words, `(dim + 3) & ~3`, `width*height` |
// | `0x0062dde0`, `0x0062de50` | destructor plan: deallocate when capacity sign is clear, free self when flag bit 0 is set |
// | `0x0062dec0` | RGB565 to three floats |
//
// Address to function:
//
// * `0x0062c330` [`a77_entry_word_c_is_set`]
// * `0x0062c3b0` [`a77_lookup_result_word`]
// * `0x0062c512` [`a77_observer_header_new`]
// * `0x0062c7fb` [`a77_research_state_word`]
// * `0x0062c8db` [`a77_state_marker_slot`], [`a77_store_state_marker`]
// * `0x0062c916` [`a77_state_guard_below_84`]
// * `0x0062c9c3` [`a77_state_entry_probe`]
// * `0x0062cbf3` [`a77_masked_state_entry_probe`]
// * `0x0062ca60` [`a77_find_record_68`]
// * `0x0062cad0` [`a77_find_record_20`]
// * `0x0062cb18` [`a77_noop`]
// * `0x0062cfd0` [`a77_singleton_present`]
// * `0x0062d3b2`, `0x0062d3e5` [`a77_frame_stats`] (helpers
//   [`a77_approach_zero_clamped`] and [`a77_half_ceil_sum3`])
// * `0x0062d5b0` [`a77_vec16_move_assign`]
// * `0x0062d624` [`a77_vec16_clone`]
// * `0x0062d670` [`a77_vec20_move_assign`]
// * `0x0062d6f7` [`a77_vec20_clone`]
// * `0x0062d750` [`a77_grown_capacity_word`]
// * `0x0062d900` [`a77_trait_upgrade_code`], [`a77_trait_code_for_byte`]
// * `0x0062db60`, `0x0062dc30` [`a77_block_sampler_header`]
// * `0x0062dd00` [`a77_rgba_sampler_header`]
// * `0x0062dde0`, `0x0062de50` [`a77_object_destructor_plan`]
// * `0x0062dec0` [`a77_rgb565_to_rgb`]
//
// Every function is a free function: the rows address their objects through
// `ECX`/`EAX`, but none of those objects is a `sim` record and no `sim`
// record gains a field here.  Value types defined in this file only describe
// the dwords the recovered stores touch.

// ---------------------------------------------------------------------------
// Small predicates and result words: 0x0062c330, 0x0062c3b0, 0x0062cfd0
// ---------------------------------------------------------------------------

/// `0x0062c330`: `return *(int *)(param_1 + 0xc) != 0;`
///
/// The row takes one element address and tests the dword at `+0x0c`; it is the
/// player-upgrade "state word is set" predicate.  Element layout beyond that
/// dword is not part of the row, so the caller supplies the element words.
pub fn a77_entry_word_c_is_set(entry: &[u32]) -> bool {
    entry[3] != 0
}

/// `0x0062c3b0`: compare an element key against the head key and build the
/// packed result word.
///
/// The shipped body is
///
/// ```text
/// uVar1 = (uint3)((uint)*param_1 >> 8);
/// if (*param_1 == **(int **)(ECX + 4) && param_1[3] == 0)
///     return CONCAT31(uVar1, 1);
/// return (uint)uVar1 << 8;
/// ```
///
/// `CONCAT31(uVar1, 1)` is `((key >> 8) << 8) | 1`, so the low byte is the
/// found flag `1` or `0` and the upper 24 bits are the key word with its low
/// byte cleared.  Only the low byte is a meaningful return value in the
/// original callers; the upper bits are the shipped leftover of the key.
pub fn a77_lookup_result_word(key: u32, head_value: u32, tail_word: u32) -> u32 {
    let payload = key & 0xffff_ff00;
    if key == head_value && tail_word == 0 {
        payload | 1
    } else {
        payload
    }
}

/// `0x0062cfd0`: `return DAT_00faa1c4 != 0;`
///
/// The row is the "ObserverUI singleton installed" test.  The global is
/// modelled as the slot value the caller reads, so the predicate stays pure.
pub fn a77_singleton_present(slot: usize) -> bool {
    slot != 0
}

/// `0x0062cb18`: empty body, `return;`.
pub fn a77_noop() {}

// ---------------------------------------------------------------------------
// ObserverUI header: 0x0062c512
// ---------------------------------------------------------------------------

/// The vtable slot stamped by `0x0062c512`.
///
/// The shipped value is the `ObserverUI::vftable` relocation; the numeric
/// address is not part of the row, so the slot is symbolic here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A77ObserverVtable {
    /// `ObserverUI::vftable`.
    ObserverUi,
}

/// The `0x40`-byte prefix written by the `0x0062c512` constructor.
///
/// The row stores `vtable` at `+0x00`, zero at `+0x04`, `+0x08` and `+0x0c`,
/// and a zero byte at `+0x40`.  Everything else is left untouched by the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A77ObserverHeader {
    /// `+0x00`: `ObserverUI::vftable`.
    pub vtable: A77ObserverVtable,
    /// `+0x04..=+0x0f`: three cleared dwords.
    pub words: [u32; 3],
    /// `+0x40`: cleared byte.
    pub byte_40: u8,
}

/// `0x0062c512`: initialise the `ObserverUI` header prefix.
///
/// The row then publishes the object into `DAT_00faa1c4`, replacing the
/// previous singleton and calling the displaced object's first vtable slot.
/// That publication is an object-graph side effect, not an arithmetic value,
/// and the vtable target is outside this window; only the stores are
/// reproduced here.
pub fn a77_observer_header_new() -> A77ObserverHeader {
    A77ObserverHeader {
        vtable: A77ObserverVtable::ObserverUi,
        words: [0; 3],
        byte_40: 0,
    }
}

// ---------------------------------------------------------------------------
// Indexed state slots: 0x0062c8db, 0x0062c916, 0x0062c9c3, 0x0062cbf3
// ---------------------------------------------------------------------------

/// Marker dword stored by `0x0062c8db` (`*(...+offset) = 8`).
pub const A77_STATE_MARKER: u32 = 8;

/// `0x0062c916` guard limit: the row compares `(int)(index + 1)` against
/// `0x84`.
pub const A77_STATE_INDEX_LIMIT: u32 = 0x84;

/// Stride of the indexed state entries addressed by `0x0062c9c3` and
/// `0x0062cbf3`: `(index * 9 + 0x6c) * 4 == 0x1b0 + 0x24 * index`.
pub const A77_STATE_ENTRY_STRIDE: u32 = 0x24;

/// Byte offset of state entry 0: `0x6c * 4`.
pub const A77_STATE_ENTRY_BASE: u32 = 0x1b0;

/// `0x0062c8db`: word slot selected by the two-bank index arithmetic.
///
/// ```text
/// if (in_EAX == 0) offset = index * 4;
/// else             offset = (index & 0xffffff7f) * 4 + 0x20;
/// ```
///
/// Both offsets are byte offsets into the same dword array, so the word slot
/// is `index` or `(index & 0xffffff7f) + 8`.  Bit 7 of `index` is masked away
/// in the second bank, which is exactly what the shipped `& 0xffffff7f`
/// does.
pub fn a77_state_marker_slot(high_bank: bool, index: u32) -> u32 {
    if high_bank {
        (index & 0xffff_ff7f).wrapping_add(8)
    } else {
        index
    }
}

/// `0x0062c8db`: store the marker `8` at the selected state slot.
///
/// The row stores `8` and then tests `index == 0`; both continuations in the
/// row are `noreturn` unwind calls, so only the guard condition is
/// recoverable.  Returns `(slot, index_was_zero)`.
pub fn a77_store_state_marker(words: &mut [u32], high_bank: bool, index: u32) -> (u32, bool) {
    let slot = a77_state_marker_slot(high_bank, index);
    words[slot as usize] = A77_STATE_MARKER;
    (slot, index == 0)
}

/// `0x0062c916`: the guard word compared by the second copy of the slot
/// store.
///
/// The row computes the same slot as `0x0062c8db` and then tests
/// `(int)(index + 1) < 0x84`.  The comparison is signed (the decompiler casts
/// the `uint` sum to `int`), so a wrapped index above `0x7fffffff` compares
/// negative and satisfies the comparison.  Both exits of the row are unwind
/// calls; this function reproduces the comparison itself.
pub fn a77_state_guard_below_84(index: u32) -> bool {
    (index.wrapping_add(1) as i32) < A77_STATE_INDEX_LIMIT as i32
}

/// What `0x0062c9c3` reads from indexed state entry `index`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A77StateEntryProbe {
    /// Byte offset of the entry: `0x1b0 + 0x24 * index`.
    pub byte_offset: u32,
    /// Byte at `byte_offset + 8`.
    pub kind: u8,
    /// `kind == 9`, the row's first abort test.
    pub kind_is_nine: bool,
    /// `0x83 < index + 1` (unsigned), the only non-abort exit in the row.
    pub index_clears_guard: bool,
}

/// `0x0062c9c3`: read the kind byte of a `0x24`-byte state entry.
///
/// The row computes `(loop_index * 9 + 0x6c) * 4` as the entry offset, reads
/// the byte at `+8`, aborts when it is `9`, and otherwise returns only when
/// `0x83 < index + 1` holds.  The other branch is an unwind call.
pub fn a77_state_entry_probe(table: &[u8], index: u32) -> A77StateEntryProbe {
    let byte_offset = A77_STATE_ENTRY_BASE.wrapping_add(index.wrapping_mul(A77_STATE_ENTRY_STRIDE));
    let kind = table[byte_offset.wrapping_add(8) as usize];
    A77StateEntryProbe {
        byte_offset,
        kind,
        kind_is_nine: kind == 9,
        index_clears_guard: 0x83 < index.wrapping_add(1),
    }
}

/// What `0x0062cbf3` reads from the masked state entry `index`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A77MaskedStateEntryProbe {
    /// Byte offset of the entry: `((index & 0xffffff7f) + 0xc) * 0x24`.
    pub byte_offset: u32,
    /// Byte at `byte_offset + 8`.
    pub kind: u8,
    /// `kind == 2`, the row's abort test.
    pub kind_is_two: bool,
}

/// `0x0062cbf3`: the masked variant of the state-entry probe.
///
/// When the row's address flag is set, the offset is
/// `((index & 0xffffff7f) + 0xc) * 0x24`, i.e. the same `0x1b0 + 0x24 * index`
/// as `0x0062c9c3` while bit 7 of the index is clear, and
/// `0x1b0 + 0x24 * (index & 0xffffff7f)` otherwise.  With the flag clear the
/// row keeps a caller frame word as the offset instead; that fallback has no
/// arithmetic.  The byte at `+8` is compared against `2`; that comparison is
/// the row's abort test.
pub fn a77_masked_state_entry_probe(table: &[u8], index: u32) -> A77MaskedStateEntryProbe {
    let byte_offset = (index & 0xffff_ff7f)
        .wrapping_add(0xc)
        .wrapping_mul(A77_STATE_ENTRY_STRIDE);
    let kind = table[byte_offset.wrapping_add(8) as usize];
    A77MaskedStateEntryProbe {
        byte_offset,
        kind,
        kind_is_two: kind == 2,
    }
}

/// `0x0062c7fb`: the research state word written to the found element's
/// `+0x0c` dword.
///
/// The row first locates an element with the predicate search recovered as
/// [`a77_find_record_20`], then stores
/// `(*(byte *)(EBP + 0xc) ^ 1) + 1`.  With the stack byte `0` the state
/// becomes `2`; with `1` it becomes `1`.  Wider inputs follow the same byte
/// arithmetic (`2` becomes `4`, and so on), so the expression is kept as
/// written rather than as a boolean.
pub fn a77_research_state_word(flag: u8) -> u32 {
    ((flag as u32) ^ 1).wrapping_add(1)
}

// ---------------------------------------------------------------------------
// Record lookups: 0x0062ca60, 0x0062cad0
// ---------------------------------------------------------------------------

/// Dwords in one record of the `0x0062ca60` array: `0x68 / 4`.
pub const A77_RECORD68_WORDS: usize = 0x1a;

/// Dwords in one element of the `0x0062cad0` array: `0x14 / 4`.
pub const A77_RECORD20_WORDS: usize = 5;

/// `0x0062ca60`: find the first `0x68`-byte record whose first dword equals
/// `key`.
///
/// ```text
/// p = base;
/// end = base + count * 0x1a;        // dword end: count * 0x68 bytes
/// while (p != end && *p != key) p += 0x1a;
/// return p == base + count * 0x68 ? 0 : p;
/// ```
///
/// The returned match is the record index here.  The shipped loop bound is
/// the dword count `count * 0x1a`; the final found test recomputes the same
/// bound in bytes as `count * 0x68`.
pub fn a77_find_record_68(records: &[[u32; A77_RECORD68_WORDS]], key: u32) -> Option<usize> {
    records.iter().position(|record| record[0] == key)
}

/// `0x0062cad0`: find the first `0x14`-byte element accepted by the predicate.
///
/// ```text
/// p = base;
/// while (p != base + count * 0x14) {
///     functor = handle;
///     if (*handle & 1) functor = *handle & 0xfffffffe;
///     if ((*(code **)(*functor + 4))(p)) break;
///     p += 0x14;
/// }
/// return p == base + count * 0x14 ? 0 : p;
/// ```
///
/// The low tag bit of the function handle is an MSVC callable-object detail:
/// an odd handle is an indirect pointer with the tag cleared before the vtable
/// slot 1 call.  The translation takes the predicate directly and returns the
/// element index; the predicate is called once per element in order and the
/// search stops at the first `true`.
pub fn a77_find_record_20(
    elements: &[[u32; A77_RECORD20_WORDS]],
    predicate: impl Fn(&[u32; A77_RECORD20_WORDS]) -> bool,
) -> Option<usize> {
    elements.iter().position(predicate)
}

// ---------------------------------------------------------------------------
// Frame approach and sample sums: 0x0062d3b2, 0x0062d3e5
// ---------------------------------------------------------------------------

/// One step of the recovered approach-to-target update.
///
/// The row computes
///
/// ```text
/// delta = target - current;
/// if (delta != 0) {
///     current += (delta > 0 ? 2 : 0) - 1
///              + ((delta + ((delta >> 31) & 0xf)) >> 4);
/// }
/// if (current < 0) current = 0;
/// ```
///
/// `(delta + ((delta >> 31) & 0xf)) >> 4` is the signed `delta / 16` rounded
/// toward zero, so the step is `sign(delta) + trunc(delta/16)`: one unit of
/// the target's direction plus a sixteenth of the remaining distance.  The
/// step never passes the target; the only clamp in the row is at zero.
pub fn a77_approach_zero_clamped(current: i32, target: i32) -> i32 {
    let delta = target.wrapping_sub(current);
    let mut next = current;
    if delta != 0 {
        let sixteenth = delta.wrapping_add((delta >> 31) & 0xf) >> 4;
        let step = (((delta > 0) as i32) * 2 - 1).wrapping_add(sixteenth);
        next = current.wrapping_add(step);
    }
    next.max(0)
}

/// Sum of the half-ceil of three low-16-bit samples.
///
/// Each loop iteration of the row adds `((sample & 0xffff) + 1) >> 1`: the
/// low word rounded up by one and halved.  The three samples of the row come
/// from two helpers outside the batch, so the translation takes their values.
pub fn a77_half_ceil_sum3(samples: [u32; 3]) -> u32 {
    samples.into_iter().fold(0u32, |sum, sample| {
        sum.wrapping_add(((sample & 0xffff).wrapping_add(1)) >> 1)
    })
}

/// The four dwords written by `0x0062d3b2` / `0x0062d3e5` for one frame
/// record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A77FrameStats {
    /// `+0x18`: first approach result.
    pub at_0x18: i32,
    /// `+0x1c`: second approach result.
    pub at_0x1c: i32,
    /// `+0x20`: sum of the first three half-ceil samples.
    pub at_0x20: u32,
    /// `+0x24`: sum of the second three half-ceil samples.
    pub at_0x24: u32,
}

/// `0x0062d3b2` and `0x0062d3e5`: update a frame record's two approach words
/// and two sample sums.
///
/// Both rows contain the identical block:
///
/// ```text
/// a = clamp0(approach(rec[0x18], table_a[index]));
/// b = clamp0(approach(rec[0x1c], table_b[index]));
/// rec[0x18] = a; rec[0x1c] = b;
/// sum0 = 0; sum1 = 0;
/// for (i = 0; i < 3; i++) {
///     sum0 += ((sample_a(i)       & 0xffff) + 1) >> 1;
///     sum1 += ((sample_b(i,index) & 0xffff) + 1) >> 1;
/// }
/// rec[0x20] = sum0; rec[0x24] = sum1;
/// ```
///
/// The two tables are `base + index*4` and `base + 0x30 + index*4` in
/// `0x0062d3b2`; `0x0062d3e5` obtains the same index from the dword at
/// `+0x10 + i*4` of the array it walks.  In both rows the index resolution
/// and the two sample helpers are outside the window, so the translation
/// takes the resolved targets and the six sample words as parameters.
pub fn a77_frame_stats(
    current: [i32; 2],
    target: [i32; 2],
    samples_lo: [u32; 3],
    samples_hi: [u32; 3],
) -> A77FrameStats {
    A77FrameStats {
        at_0x18: a77_approach_zero_clamped(current[0], target[0]),
        at_0x1c: a77_approach_zero_clamped(current[1], target[1]),
        at_0x20: a77_half_ceil_sum3(samples_lo),
        at_0x24: a77_half_ceil_sum3(samples_hi),
    }
}

// ---------------------------------------------------------------------------
// 16-byte and 20-byte element vectors
// ---------------------------------------------------------------------------

/// Low 31-bit capacity mask of the vector headers in this window.
pub const A77_VEC_CAP_MASK: u32 = 0x7fff_ffff;

/// Sign bit with which the vector headers mark one of their two storage
/// states; every recovered test is `(int)cap_word < 0`.
pub const A77_VEC_CAP_FLAG: u32 = 0x8000_0000;

/// A vector of four-dword elements as the batch-77 rows see it: data, element
/// count and the capacity/flags word.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A77Vec16 {
    /// `+0x00`: element storage.
    pub data: Vec<[u32; 4]>,
    /// `+0x04`: element count.
    pub count: u32,
    /// `+0x08`: capacity in the low 31 bits, flag in bit 31.
    pub cap_word: u32,
}

/// A vector of five-dword elements as the batch-77 rows see it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A77Vec20 {
    /// `+0x00`: element storage.
    pub data: Vec<[u32; 5]>,
    /// `+0x04`: element count.
    pub count: u32,
    /// `+0x08`: capacity in the low 31 bits, flag in bit 31.
    pub cap_word: u32,
}

/// `0x0062d5b0`: move-assign a 16-byte element vector.
///
/// ```text
/// if (src.cap_word >= 0) {                 // signed test
///     dst.cap_word &= 0x7fffffff;
///     dst.data = src.data;
///     dst.count = src.count;
///     dst.cap_word = src.cap_word & 0x7fffffff;
///     src.cap_word &= 0x80000000;
///     src.data = 0; src.count = 0;
/// } else {
///     dst.count = 0;
///     if (src.count < (dst.cap_word & 0x7fffffff) && dst.cap_word < 0) {
///         copy src.count elements into dst.data;
///         dst.count = src.count;
///     } else {
///         throw;
///     }
/// }
/// ```
///
/// The second branch copies into the destination's existing storage rather
/// than stealing the pointer.  The row trusts that storage to hold
/// `src.count` elements; the translation returns `Err(())` instead of
/// indexing past the modelled `data` buffer (the shipped throw path is the
/// `FUN_0093ec70` call).  In the steal branch the row overwrites the
/// destination data pointer without showing a deallocation, while the
/// translation drops the displaced `Vec`.
pub fn a77_vec16_move_assign(dst: &mut A77Vec16, src: &mut A77Vec16) -> Result<(), ()> {
    if (src.cap_word as i32) >= 0 {
        dst.data = core::mem::take(&mut src.data);
        dst.count = src.count;
        dst.cap_word = src.cap_word & A77_VEC_CAP_MASK;
        src.count = 0;
        src.cap_word &= A77_VEC_CAP_FLAG;
        Ok(())
    } else {
        dst.count = 0;
        let need = src.count as usize;
        if src.count < (dst.cap_word & A77_VEC_CAP_MASK)
            && (dst.cap_word as i32) < 0
            && need <= dst.data.len()
        {
            for index in 0..need {
                dst.data[index] = src.data[index];
            }
            dst.count = src.count;
            Ok(())
        } else {
            Err(())
        }
    }
}

/// `0x0062d624`: clone `count` 16-byte elements into a fresh buffer.
///
/// The row allocates `count << 4` bytes with alignment `8`, stores the buffer
/// at `+0x00`, stores `count & 0x7fffffff` at `+0x08`, copies the elements
/// and finally stores `count` at `+0x04`.  The allocation address is not
/// reproducible; the element copies and both header words are.
pub fn a77_vec16_clone(src: &[[u32; 4]], count: u32) -> A77Vec16 {
    A77Vec16 {
        data: src[..count as usize].to_vec(),
        count,
        cap_word: count & A77_VEC_CAP_MASK,
    }
}

/// `0x0062d670`: move-assign a 20-byte element vector.
///
/// Same three-word protocol as [`a77_vec16_move_assign`]; the only difference
/// in the row is the `0x14`-byte element stride and the five-dword copy
/// (`(param_1[1] * 0x14) / 0x14` is the element count).  The same
/// ownership note applies: the steal branch drops the displaced destination
/// storage, which the row does not show being freed.
pub fn a77_vec20_move_assign(dst: &mut A77Vec20, src: &mut A77Vec20) -> Result<(), ()> {
    if (src.cap_word as i32) >= 0 {
        dst.data = core::mem::take(&mut src.data);
        dst.count = src.count;
        dst.cap_word = src.cap_word & A77_VEC_CAP_MASK;
        src.count = 0;
        src.cap_word &= A77_VEC_CAP_FLAG;
        Ok(())
    } else {
        dst.count = 0;
        let need = src.count as usize;
        if src.count < (dst.cap_word & A77_VEC_CAP_MASK)
            && (dst.cap_word as i32) < 0
            && need <= dst.data.len()
        {
            for index in 0..need {
                dst.data[index] = src.data[index];
            }
            dst.count = src.count;
            Ok(())
        } else {
            Err(())
        }
    }
}

/// `0x0062d6f7`: clone `count` 20-byte elements into a fresh buffer.
///
/// The row allocates `count * 0x14` bytes with alignment `8`, stores the
/// buffer at `+0x00` and `count & 0x7fffffff` at `+0x08`, copies five dwords
/// per element and stores `count` at `+0x04`.
pub fn a77_vec20_clone(src: &[[u32; 5]], count: u32) -> A77Vec20 {
    A77Vec20 {
        data: src[..count as usize].to_vec(),
        count,
        cap_word: count & A77_VEC_CAP_MASK,
    }
}

/// `0x0062d750`: the vector growth policy word.
///
/// ```text
/// low = cap & 0x7fffffff;
/// if (low < requested) {
///     grown = (cap >> 1 & 0x3fffffff) + low;
///     if (grown < requested) grown = requested;
///     cap = ((cap ^ grown) & 0x7fffffff) ^ cap;
/// }
/// ```
///
/// The last expression keeps the original bit 31 and replaces the low 31
/// bits with `grown`, i.e. it is `(cap & 0x80000000) | (grown & 0x7fffffff)`.
/// The row stores that word and then enters the relocation path; only the
/// word arithmetic is reproduced here.  The same policy appears at another
/// address as `a60_grown_capacity`; this copy is the batch-77 formulation.
pub fn a77_grown_capacity_word(cap: u32, requested: u32) -> u32 {
    let low = cap & A77_VEC_CAP_MASK;
    if low < requested {
        let mut grown = (cap >> 1 & 0x3fff_ffff).wrapping_add(low);
        if grown < requested {
            grown = requested;
        }
        ((cap ^ grown) & A77_VEC_CAP_MASK) ^ cap
    } else {
        cap
    }
}

// ---------------------------------------------------------------------------
// Upgrade code table: 0x0062d900
// ---------------------------------------------------------------------------

/// Byte offset of the trait table read by `0x0062d900`, relative to the
/// image-base word `DAT_00f063cc ^ 0x10fae560`.  The row indexes the table
/// with a byte, so the recovered table window is 256 bytes.
pub const A77_TRAIT_TABLE_OFFSET: u32 = 0x2ce6;

/// `0x0062d900`: look the trait byte up in the image table and map it to an
/// upgrade code.
///
/// The row loads `*(byte *)(image_base + 0x2ce6 + index)` and switches on the
/// loaded byte:
///
/// ```text
/// switch (image[0x2ce6 + index]) {
/// case 0x13: return 0x15;   case 0x33: return 0x1c;
/// case 0x4d: return 0x1d;   case 0x54: return 0x16;
/// case 0x6f: return 8;      case 0x80: return 0x1f;
/// case 0x86: return 0x1b;   case 0x87: return 0x17;
/// case 0x88: return 0x19;   case 0x9a: return 0x1e;
/// case 0x9c: return 0x11;   case 0x9f: return 0x0f;
/// case 0xa4: return 0x10;   case 0xa5: return 0x0e;
/// case 0xb9: return 0x18;   default: return 2;
/// }
/// ```
///
/// `image_base` is the de-obfuscated `DAT_00f063cc ^ 0x10fae560`; the table
/// bytes themselves are data outside the row, so the caller supplies the
/// image slice.  The switch itself is [`a77_trait_code_for_byte`].
pub fn a77_trait_upgrade_code(image: &[u8], index: u8) -> u32 {
    a77_trait_code_for_byte(image[A77_TRAIT_TABLE_OFFSET as usize + index as usize])
}

/// The switch half of `0x0062d900`, applied to the byte the table supplies.
pub fn a77_trait_code_for_byte(table_byte: u8) -> u32 {
    match table_byte {
        0x13 => 0x15,
        0x33 => 0x1c,
        0x4d => 0x1d,
        0x54 => 0x16,
        0x6f => 0x08,
        0x80 => 0x1f,
        0x86 => 0x1b,
        0x87 => 0x17,
        0x88 => 0x19,
        0x9a => 0x1e,
        0x9c => 0x11,
        0x9f => 0x0f,
        0xa4 => 0x10,
        0xa5 => 0x0e,
        0xb9 => 0x18,
        _ => 2,
    }
}

// ---------------------------------------------------------------------------
// Texture sampler headers: 0x0062db60, 0x0062dc30, 0x0062dd00
// ---------------------------------------------------------------------------

/// Which block sampler vtable `0x0062db60` / `0x0062dc30` stamps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A77BlockSampler {
    /// `0x0062db60`: `DXT1Sampler`.
    Dxt1,
    /// `0x0062dc30`: `DXT5Sampler`.
    Dxt5,
}

/// The header words written by the two block-sampler constructors.
///
/// Both rows write `vtable` at `+0x00`, zero at `+0x04`, `+0x08` and `+0x0c`,
/// then store the two descriptor dwords rounded up to a multiple of four.
/// The decompiler frame names those dwords `local_88` (first, at `+8` of the
/// 16-byte descriptor) and `local_84` (second, at `+12`):
///
/// ```text
/// +0x10 = (local_84 + 3) & 0xfffffffc
/// +0x14 = (local_88 + 3) & 0xfffffffc
/// ```
///
/// The roles follow `0x0062dd00`, which stores the same class family's
/// `+0x10` as the pitch and `+0x14` as the width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A77BlockSamplerHeader {
    /// `+0x00`: the sampler vtable.
    pub kind: A77BlockSampler,
    /// `+0x04..=+0x0f`: three cleared dwords.
    pub zero_words: [u32; 3],
    /// `+0x10`: `(pitch + 3) & 0xfffffffc`.
    pub pitch_rounded: u32,
    /// `+0x14`: `(width + 3) & 0xfffffffc`.
    pub width_rounded: u32,
}

/// `0x0062db60` (`DXT1Sampler`) and `0x0062dc30` (`DXT5Sampler`): build the
/// sampler header.
///
/// The two rows are identical apart from the vtable stamped at `+0x00`; both
/// round two descriptor dwords up to a multiple of four and clear the three
/// header words between the vtable and the rounded dimensions.
pub fn a77_block_sampler_header(
    kind: A77BlockSampler,
    pitch: u32,
    width: u32,
) -> A77BlockSamplerHeader {
    A77BlockSamplerHeader {
        kind,
        zero_words: [0; 3],
        pitch_rounded: pitch.wrapping_add(3) & 0xffff_fffc,
        width_rounded: width.wrapping_add(3) & 0xffff_fffc,
    }
}

/// The header words written by `0x0062dd00` (`RGBASampler`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A77RgbaSamplerHeader {
    /// `+0x04..=+0x0f`: three cleared dwords.
    pub zero_words: [u32; 3],
    /// `+0x10`: the surface pitch (stored unrounded).
    pub at_0x10: u32,
    /// `+0x14`: the surface width.
    pub at_0x14: u32,
    /// `+0x18`: the surface height.
    pub at_0x18: u32,
    /// The `width * height` product passed to the sizing helper.
    pub pixel_count: u32,
}

/// `0x0062dd00`: build the `RGBASampler` header.
///
/// The row stores the vtable, clears `+0x04..=+0x0f`, stores the descriptor
/// dwords at `+0x10`, `+0x14` and `+0x18` and calls the sizing helper with
/// `iStack_88 * iStack_80`, i.e. `width * height`.  The product wraps as a
/// 32-bit multiply.
pub fn a77_rgba_sampler_header(pitch: u32, width: u32, height: u32) -> A77RgbaSamplerHeader {
    A77RgbaSamplerHeader {
        zero_words: [0; 3],
        at_0x10: pitch,
        at_0x14: width,
        at_0x18: height,
        pixel_count: width.wrapping_mul(height),
    }
}

// ---------------------------------------------------------------------------
// Destructor plan and colour unpack: 0x0062dde0, 0x0062de50, 0x0062dec0
// ---------------------------------------------------------------------------

/// The branch decisions of the two sampler-destructor rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A77DestructorPlan {
    /// `true` when the capacity word at `+0x0c` has its sign bit clear, the
    /// branch that calls the storage-deallocation helper.
    pub deallocate: bool,
    /// `true` when bit 0 of the deleting flag is set.
    pub free_self: bool,
}

/// `0x0062dde0` and `0x0062de50`: the sampler destructor's two tests.
///
/// Both rows are the same shape:
///
/// ```text
/// if (-1 < *(int *)(ECX + 0xc)) deallocate();   // signed test on the cap word
/// if ((param_1 & 1) != 0)        free_self();    // deleting destructor
/// ```
///
/// The 94-byte row at `0x0062de50` differs from the 42-byte row at
/// `0x0062dde0` only in its exception frame setup; the visible tests are
/// identical.
pub fn a77_object_destructor_plan(cap_word: u32, delete_flags: u8) -> A77DestructorPlan {
    A77DestructorPlan {
        deallocate: (cap_word as i32) >= 0,
        free_self: delete_flags & 1 != 0,
    }
}

/// `0x0062dec0`: unpack a packed RGB565 word into three floats.
///
/// ```text
/// FUN_00a8fbb0(out,
///     (float)(packed >> 11 & 0x1f) / 31.0,
///     (float)(packed >>  5 & 0x3f) / 63.0,
///     (float) packed        & 0x1f  / 31.0);
/// ```
///
/// Each component divides in `f64` (the literal is `31.0`/`63.0`, so C
/// promotes the float operand before dividing) and the result is rounded to
/// `f32` for the colour helper's parameter.  The helper's only recovered
/// effect is receiving these three normalised components.
pub fn a77_rgb565_to_rgb(packed: u32) -> [f32; 3] {
    [
        (((packed >> 11) & 0x1f) as f64 / 31.0) as f32,
        (((packed >> 5) & 0x3f) as f64 / 63.0) as f32,
        ((packed & 0x1f) as f64 / 31.0) as f32,
    ]
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// The 52 rows not translated above, grouped by why:
//
// * `FUN_0062c000`, `FUN_0062c560`, `FUN_0062c8c0`, `FUN_0062cb30`,
//   `FUN_0062cc6e`, `FUN_0062d3a0`, `FUN_0062da60`, `FUN_0062da7d`,
//   `FUN_0062dae0`: bodies whose only call is a `noreturn` throw/abort helper
//   (`FUN_005b4e00`, `FUN_00748d40`, `FUN_00923df0`).  The forwarded
//   argument is a caller register or a constant (`7`, `0`, `0x18`) with no
//   arithmetic, table or string value to reproduce.
// * `FUN_0062c039`, `FUN_0062c04c`, `FUN_0062c0ed`, `FUN_0062c652`,
//   `FUN_0062c6fc`, `FUN_0062cc40`, `FUN_0062cd8f`, `FUN_0062ce2c`,
//   `FUN_0062ce55`, `FUN_0062ce7e`, `FUN_0062d7f7`: exception-unwind bodies
//   that read state through `unaff_EBP`/`unaff_ESI` slots of a caller outside
//   the window and end in `FUN_00538810` / `FUN_00748d40` /
//   `FUN_005426e0`.  Their visible work is an inlined vector append or a
//   lambda destroy whose operand addresses are not in the row.
// * `FUN_0062c156`, `FUN_0062c17c`, `FUN_0062c768`, `FUN_0062c791`,
//   `FUN_0062d8d3`: `(**(code **)(*EAX + 0x10))()` destructor-call fragments
//   followed by an unwind tail; there is no local arithmetic.
// * `FUN_0062c266`, `FUN_0062c2b6`, `FUN_0062ca15`, `FUN_0062cca7`,
//   `FUN_0062cd58`, `FUN_0062d4ed`, `FUN_0062d547`, `FUN_0062d7b5`: vector
//   relocation fragments that begin mid-operation (unaff registers, frame
//   index words) or whose element deep copy is `FUN_0053a5b0` /
//   `FUN_0062bbe0` / `FUN_005b4c60`, all outside the batch.  `FUN_0062cca7` and
//   `FUN_0062ca15` repeat the slot arithmetic already recovered at
//   `0x0062c9c3`/`0x0062c8db`; `FUN_0062c266`/`FUN_0062c2b6` repeat the
//   20-byte relocation and append halves.
// * `FUN_0062c3e0`, `FUN_0062c3f0`, `FUN_0062c410`, `FUN_0062caa0`,
//   `FUN_0062cab0`, `FUN_0062cac0`, `FUN_0062c430`, `FUN_0062c466`,
//   `FUN_0062c4fc`, `FUN_0062cfe0`, `FUN_0062df30`: bodies whose payload is
//   a vtable/RTTI address store or a call into a factory/singleton helper
//   outside the batch.  The static-init row additionally clears a guard and
//   swaps `DAT_00faa1c4`, but both sides of the swap are object addresses and
//   the displaced object's vtable call is outside the window.  The vtable and
//   type-descriptor values are relocated addresses, not arithmetic, table or
//   string data, so there is nothing exact to reproduce.
// * `FUN_0062c350`: three predicate calls followed by a teardown call; every
//   branch target and the only argument source are outside the row.
// * `FUN_0062c4ee`, `FUN_0062d1ba`, `FUN_0062d979`, `FUN_0062db3f`,
//   `FUN_0062ddb2`: the decompiler reports bad instruction data (`in`, `out`,
//   `halt_baddata`, non-canonical stores); no source-level behaviour is
//   recoverable.
// * `FUN_0062daf5`, `FUN_0062dd1a`: empty `do { } while (true)` bodies.

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent77 {
    use super::*;

    // -- 0x0062c330 --------------------------------------------------------

    #[test]
    fn entry_word_c_predicate_reads_the_fourth_dword() {
        assert!(!a77_entry_word_c_is_set(&[1, 2, 3, 0]));
        assert!(a77_entry_word_c_is_set(&[1, 2, 3, 7]));
    }

    // -- 0x0062c3b0 --------------------------------------------------------

    #[test]
    fn lookup_result_word_keeps_key_high_bytes_and_sets_low_flag() {
        assert_eq!(
            a77_lookup_result_word(0x1234_5678, 0x1234_5678, 0),
            0x1234_5601
        );
        assert_eq!(
            a77_lookup_result_word(0x1234_5678, 0x1234_5678, 1),
            0x1234_5600
        );
        assert_eq!(
            a77_lookup_result_word(0x1234_5678, 0x1234_0000, 0),
            0x1234_5600
        );
        assert_eq!(
            a77_lookup_result_word(0x0000_00ff, 0x0000_00ff, 0),
            0x0000_0001
        );
    }

    // -- 0x0062c512 --------------------------------------------------------

    #[test]
    fn observer_header_clears_the_recovered_words() {
        let header = a77_observer_header_new();
        assert_eq!(header.vtable, A77ObserverVtable::ObserverUi);
        assert_eq!(header.words, [0, 0, 0]);
        assert_eq!(header.byte_40, 0);
    }

    // -- 0x0062c8db / 0x0062c916 ------------------------------------------

    #[test]
    fn state_marker_slot_matches_both_index_forms() {
        assert_eq!(a77_state_marker_slot(false, 5), 5);
        assert_eq!(a77_state_marker_slot(true, 5), 13);
        assert_eq!(a77_state_marker_slot(true, 0x80), 8);
        assert_eq!(a77_state_marker_slot(true, 0x85), 13);
    }

    #[test]
    fn state_marker_store_writes_eight_and_reports_zero_index() {
        let mut words = [0u32; 0x20];
        assert_eq!(a77_store_state_marker(&mut words, false, 3), (3, false));
        assert_eq!(words[3], A77_STATE_MARKER);
        assert_eq!(a77_store_state_marker(&mut words, true, 0x82), (10, false));
        assert_eq!(words[10], A77_STATE_MARKER);
        assert_eq!(a77_store_state_marker(&mut words, true, 0), (8, true));
        assert_eq!(words[8], A77_STATE_MARKER);
    }

    #[test]
    fn state_guard_is_the_signed_comparison() {
        assert!(a77_state_guard_below_84(0));
        assert!(a77_state_guard_below_84(0x82));
        assert!(!a77_state_guard_below_84(0x83));
        assert!(!a77_state_guard_below_84(0x100));
        // Wrapped index: `(int)(index + 1)` is negative and compares below.
        assert!(a77_state_guard_below_84(0xffff_ffff));
    }

    // -- 0x0062c9c3 / 0x0062cbf3 ------------------------------------------

    #[test]
    fn state_entry_probe_uses_the_0x24_stride() {
        let mut table =
            vec![0u8; A77_STATE_ENTRY_BASE as usize + A77_STATE_ENTRY_STRIDE as usize * 0x85 + 8];
        table[0x1b8] = 9;
        table[0x1b8 + 0x24] = 3;
        let first = a77_state_entry_probe(&table, 0);
        assert_eq!(first.byte_offset, 0x1b0);
        assert_eq!(first.kind, 9);
        assert!(first.kind_is_nine);
        assert!(!first.index_clears_guard);
        let second = a77_state_entry_probe(&table, 1);
        assert_eq!(second.byte_offset, 0x1d4);
        assert_eq!(second.kind, 3);
        assert!(!second.kind_is_nine);
        assert!(!a77_state_entry_probe(&table, 0x82).index_clears_guard);
        assert!(a77_state_entry_probe(&table, 0x83).index_clears_guard);
    }

    #[test]
    fn masked_state_entry_probe_clears_bit_seven() {
        let mut table = vec![0u8; 0x1b8 + 0x24 * 4];
        table[0x1b8] = 2;
        table[0x1b8 + 0x24] = 5;
        let masked_zero = a77_masked_state_entry_probe(&table, 0x80);
        assert_eq!(masked_zero.byte_offset, 0x1b0);
        assert_eq!(masked_zero.kind, 2);
        assert!(masked_zero.kind_is_two);
        let masked_one = a77_masked_state_entry_probe(&table, 0x81);
        assert_eq!(masked_one.byte_offset, 0x1d4);
        assert_eq!(masked_one.kind, 5);
        assert!(!masked_one.kind_is_two);
    }

    // -- 0x0062c7fb --------------------------------------------------------

    #[test]
    fn research_state_word_is_two_minus_the_flag() {
        assert_eq!(a77_research_state_word(0), 2);
        assert_eq!(a77_research_state_word(1), 1);
        assert_eq!(a77_research_state_word(2), 4);
        assert_eq!(a77_research_state_word(0xff), 0xff);
    }

    // -- 0x0062ca60 / 0x0062cad0 ------------------------------------------

    #[test]
    fn find_record_68_stops_at_the_matching_key() {
        let mut first = [0u32; A77_RECORD68_WORDS];
        first[0] = 11;
        let mut second = [0u32; A77_RECORD68_WORDS];
        second[0] = 22;
        second[25] = 0xdead;
        let table = [first, second];
        assert_eq!(a77_find_record_68(&table, 11), Some(0));
        assert_eq!(a77_find_record_68(&table, 22), Some(1));
        assert_eq!(a77_find_record_68(&table, 33), None);
        assert_eq!(a77_find_record_68(&[], 22), None);
    }

    #[test]
    fn find_record_20_returns_the_first_accepted_element() {
        let elements = [[1, 2, 3, 4, 5], [6, 7, 8, 9, 10], [11, 12, 13, 14, 15]];
        assert_eq!(
            a77_find_record_20(&elements, |element| element[2] == 8),
            Some(1)
        );
        assert_eq!(
            a77_find_record_20(&elements, |element| element[0] == 11),
            Some(2)
        );
        assert_eq!(a77_find_record_20(&elements, |_| false), None);
    }

    // -- 0x0062cb18 / 0x0062cfd0 ------------------------------------------

    #[test]
    fn noop_and_singleton_predicate() {
        a77_noop();
        assert!(!a77_singleton_present(0));
        assert!(a77_singleton_present(0x00fa_a1c4));
    }

    // -- 0x0062d3b2 / 0x0062d3e5 ------------------------------------------

    #[test]
    fn approach_moves_by_sixteenth_plus_one_and_clamps_at_zero() {
        assert_eq!(a77_approach_zero_clamped(0, 0), 0);
        assert_eq!(a77_approach_zero_clamped(0, 1), 1);
        assert_eq!(a77_approach_zero_clamped(0, 16), 2);
        assert_eq!(a77_approach_zero_clamped(0, 32), 3);
        assert_eq!(a77_approach_zero_clamped(100, 0), 93);
        assert_eq!(a77_approach_zero_clamped(1, 0), 0);
        assert_eq!(a77_approach_zero_clamped(-10, 0), 0);
        assert_eq!(a77_approach_zero_clamped(0, -16), 0);
    }

    #[test]
    fn half_ceil_sum_uses_low_sixteen_bits() {
        assert_eq!(a77_half_ceil_sum3([0, 0, 0]), 0);
        assert_eq!(a77_half_ceil_sum3([1, 2, 3]), 1 + 1 + 2);
        assert_eq!(
            a77_half_ceil_sum3([0xffff, 0x1_0000, 0x2_0001]),
            0x8000 + 0 + 1
        );
    }

    #[test]
    fn frame_stats_combines_approach_and_sample_sums() {
        let stats = a77_frame_stats([10, 20], [26, 4], [1, 2, 3], [0xffff, 0, 0x1_0002]);
        assert_eq!(stats.at_0x18, 12);
        assert_eq!(stats.at_0x1c, 18);
        assert_eq!(stats.at_0x20, 1 + 1 + 2);
        assert_eq!(stats.at_0x24, 0x8000 + 0 + 1);
    }

    // -- 0x0062d5b0 / 0x0062d624 ------------------------------------------

    #[test]
    fn vec16_move_assign_steals_a_flagged_source() {
        let mut dst = A77Vec16 {
            data: vec![[9; 4]],
            count: 1,
            cap_word: 0x8000_0001,
        };
        let mut src = A77Vec16 {
            data: vec![[1; 4], [2; 4]],
            count: 2,
            cap_word: 0x7fff_0002,
        };
        assert_eq!(a77_vec16_move_assign(&mut dst, &mut src), Ok(()));
        assert_eq!(dst.data, vec![[1; 4], [2; 4]]);
        assert_eq!(dst.count, 2);
        assert_eq!(dst.cap_word, 0x7fff_0002);
        assert!(src.data.is_empty());
        assert_eq!(src.count, 0);
        assert_eq!(src.cap_word, 0);
    }

    #[test]
    fn vec16_move_assign_copies_into_a_flagged_destination() {
        let mut dst = A77Vec16 {
            data: vec![[0; 4]; 4],
            count: 3,
            cap_word: 0x8000_0004,
        };
        let mut src = A77Vec16 {
            data: vec![[5; 4], [6; 4]],
            count: 2,
            cap_word: 0x8000_0002,
        };
        assert_eq!(a77_vec16_move_assign(&mut dst, &mut src), Ok(()));
        assert_eq!(dst.count, 2);
        assert_eq!(dst.data[..2], [[5; 4], [6; 4]]);
        assert_eq!(dst.data[2], [0; 4]);
        assert_eq!(dst.cap_word, 0x8000_0004);
        assert_eq!(src.count, 2);

        let mut small = A77Vec16 {
            data: vec![[0; 4]; 1],
            count: 0,
            cap_word: 0x8000_0001,
        };
        let mut too_big = A77Vec16 {
            data: vec![[7; 4], [8; 4]],
            count: 2,
            cap_word: 0x8000_0002,
        };
        assert_eq!(a77_vec16_move_assign(&mut small, &mut too_big), Err(()));
    }

    #[test]
    fn vec16_clone_copies_count_elements_and_clears_the_flag() {
        let source = [[1; 4], [2; 4], [3; 4]];
        let cloned = a77_vec16_clone(&source, 2);
        assert_eq!(cloned.data, vec![[1; 4], [2; 4]]);
        assert_eq!(cloned.count, 2);
        assert_eq!(cloned.cap_word, 2);
        let empty = a77_vec16_clone(&source, 0);
        assert!(empty.data.is_empty());
        assert_eq!(empty.count, 0);
    }

    // -- 0x0062d670 / 0x0062d6f7 ------------------------------------------

    #[test]
    fn vec20_move_assign_follows_the_five_dword_stride() {
        let mut dst = A77Vec20 {
            data: vec![[0; 5]; 3],
            count: 0,
            cap_word: 0x8000_0003,
        };
        let mut src = A77Vec20 {
            data: vec![[1; 5], [2; 5]],
            count: 2,
            cap_word: 0x8000_0002,
        };
        assert_eq!(a77_vec20_move_assign(&mut dst, &mut src), Ok(()));
        assert_eq!(dst.count, 2);
        assert_eq!(dst.data[..2], [[1; 5], [2; 5]]);

        let mut steal_dst = A77Vec20::default();
        let mut steal_src = A77Vec20 {
            data: vec![[4; 5]],
            count: 1,
            cap_word: 9,
        };
        assert_eq!(
            a77_vec20_move_assign(&mut steal_dst, &mut steal_src),
            Ok(())
        );
        assert_eq!(steal_dst.data, vec![[4; 5]]);
        assert_eq!(steal_dst.count, 1);
        assert_eq!(steal_dst.cap_word, 9);
        assert!(steal_src.data.is_empty());
    }

    #[test]
    fn vec20_clone_copies_count_elements() {
        let source = [[1; 5], [2; 5], [3; 5]];
        let cloned = a77_vec20_clone(&source, 3);
        assert_eq!(cloned.data, source.to_vec());
        assert_eq!(cloned.count, 3);
        assert_eq!(cloned.cap_word, 3);
    }

    // -- 0x0062d750 --------------------------------------------------------

    #[test]
    fn grown_capacity_preserves_the_sign_bit() {
        assert_eq!(a77_grown_capacity_word(0x8000_0004, 5), 0x8000_0006);
        assert_eq!(a77_grown_capacity_word(0x8000_0010, 5), 0x8000_0010);
        assert_eq!(a77_grown_capacity_word(4, 100), 100);
        assert_eq!(a77_grown_capacity_word(0x8000_0000, 1), 0x8000_0001);
        assert_eq!(a77_grown_capacity_word(0, 0), 0);
    }

    // -- 0x0062d900 --------------------------------------------------------

    #[test]
    fn trait_table_switch_returns_the_exact_codes() {
        let table = [
            (0x13u8, 0x15u32),
            (0x33, 0x1c),
            (0x4d, 0x1d),
            (0x54, 0x16),
            (0x6f, 8),
            (0x80, 0x1f),
            (0x86, 0x1b),
            (0x87, 0x17),
            (0x88, 0x19),
            (0x9a, 0x1e),
            (0x9c, 0x11),
            (0x9f, 0x0f),
            (0xa4, 0x10),
            (0xa5, 0x0e),
            (0xb9, 0x18),
        ];
        for (input, expected) in table {
            assert_eq!(a77_trait_code_for_byte(input), expected, "input {input:#x}");
        }
        for input in [0u8, 1, 0x12, 0x14, 0xb8, 0xff] {
            assert_eq!(a77_trait_code_for_byte(input), 2);
        }
        assert_eq!(A77_TRAIT_TABLE_OFFSET, 0x2ce6);

        let mut image = vec![0u8; A77_TRAIT_TABLE_OFFSET as usize + 0x100];
        image[A77_TRAIT_TABLE_OFFSET as usize + 0x13] = 0x6f;
        image[A77_TRAIT_TABLE_OFFSET as usize + 0x54] = 0xb9;
        assert_eq!(a77_trait_upgrade_code(&image, 0x13), 8);
        assert_eq!(a77_trait_upgrade_code(&image, 0x54), 0x18);
        assert_eq!(a77_trait_upgrade_code(&image, 0x00), 2);
    }

    // -- 0x0062db60 / 0x0062dc30 / 0x0062dd00 -----------------------------

    #[test]
    fn block_sampler_header_rounds_both_dimensions() {
        let dxt1 = a77_block_sampler_header(A77BlockSampler::Dxt1, 5, 9);
        assert_eq!(dxt1.kind, A77BlockSampler::Dxt1);
        assert_eq!(dxt1.zero_words, [0, 0, 0]);
        assert_eq!(dxt1.pitch_rounded, 8);
        assert_eq!(dxt1.width_rounded, 12);

        let dxt5 = a77_block_sampler_header(A77BlockSampler::Dxt5, 0, 1);
        assert_eq!(dxt5.kind, A77BlockSampler::Dxt5);
        assert_eq!(dxt5.pitch_rounded, 0);
        assert_eq!(dxt5.width_rounded, 4);
    }

    #[test]
    fn rgba_sampler_header_stores_the_pitch_and_pixel_count() {
        let header = a77_rgba_sampler_header(0x280, 320, 200);
        assert_eq!(header.zero_words, [0, 0, 0]);
        assert_eq!(header.at_0x10, 0x280);
        assert_eq!(header.at_0x14, 320);
        assert_eq!(header.at_0x18, 200);
        assert_eq!(header.pixel_count, 64_000);
    }

    // -- 0x0062dde0 / 0x0062de50 ------------------------------------------

    #[test]
    fn destructor_plan_matches_the_two_tests() {
        assert_eq!(
            a77_object_destructor_plan(0x8000_0000, 0),
            A77DestructorPlan {
                deallocate: false,
                free_self: false
            }
        );
        assert_eq!(
            a77_object_destructor_plan(0, 1),
            A77DestructorPlan {
                deallocate: true,
                free_self: true
            }
        );
        assert!(a77_object_destructor_plan(0x7fff_ffff, 2).deallocate);
        assert!(!a77_object_destructor_plan(0x7fff_ffff, 2).free_self);
        assert!(!a77_object_destructor_plan(0xffff_ffff, 3).deallocate);
        assert!(a77_object_destructor_plan(0xffff_ffff, 3).free_self);
    }

    // -- 0x0062dec0 --------------------------------------------------------

    #[test]
    fn rgb565_unpack_normalises_each_component() {
        assert_eq!(a77_rgb565_to_rgb(0x0000), [0.0, 0.0, 0.0]);
        assert_eq!(a77_rgb565_to_rgb(0xffff), [1.0, 1.0, 1.0]);
        assert_eq!(a77_rgb565_to_rgb(0xf800), [1.0, 0.0, 0.0]);
        assert_eq!(a77_rgb565_to_rgb(0x07e0), [0.0, 1.0, 0.0]);
        assert_eq!(a77_rgb565_to_rgb(0x001f), [0.0, 0.0, 1.0]);
        let one = a77_rgb565_to_rgb(0x0001);
        assert_eq!(one[2].to_bits(), 0x3d04_2108);
        let mid = a77_rgb565_to_rgb(0x8410);
        assert_eq!(mid[0].to_bits(), 0x3f04_2108);
        assert_eq!(mid[1].to_bits(), 0x3f02_0821);
        assert_eq!(mid[2].to_bits(), 0x3f04_2108);
    }
}
