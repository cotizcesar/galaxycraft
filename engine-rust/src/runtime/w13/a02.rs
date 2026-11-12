#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave13/agent02.rs
// ===========================================================================
// Wave 13 translation batch 02: container/allocator helpers and small
// state predicates in `0065e000..=0065ffff`.
//
// The ledger lists 32 `decompiled_uncited` rows in this window plus the
// already-translated `0065f590` (`return 1`).  The window is the tail of
// the `blz` byte/word/record vector implementation, the `TimedScope` /
// `InGameLagTimedScope` header setup, and the first button-condition
// routines.  Most rows are continuation fragments of the two large string /
// registry routines at `0065e8a0` and `0065ee50`, or compiler-generated
// vtable/epilogue stubs.  Nine rows are translated here; every other row is
// listed under [Skipped rows](#skipped-rows) with the reason it is not
// modelled.
//
// Nothing here touches the simulation.  The helpers are auditable reference
// code over `Vec`/slices/values only; allocation is modelled with `Vec`
// instead of the shipped allocator singleton returned by `0093ec70`.
//
// ## Recovered layouts
//
// | location | layout |
// |----------|--------|
// | vector header | `+0x00` begin pointer, `+0x04` element count, `+0x08` capacity word |
// | capacity word | bit 31 is the ownership flag (clear = owned), bits 0..30 are the capacity in elements |
// | record | `10` bytes, five `u16` words: `[id, 0xffff, 0xffff, 0, 0]` |
// | table object | `+0x10` record array, `+0x24` per-slot flag bytes |
// | scope header | `+0x00` vtable, `+0x08` context word |
//
// The three capacity-growing routines (`0x0065e460`, `0x0065e640`,
// `0x0065e700`) all use the same policy: when the requested capacity exceeds
// the low 31 bits of the header word, the new capacity is
// `max(needed, capacity + capacity / 2)` and bit 31 is preserved.  The byte
// and record routines allocate through the allocator singleton
// `0093ec70` with alignment `8`.
//
// ## Skipped rows
//
// * `0065e009` - Ghidra hit bad instruction data and an overlapping
//   instruction at `0065e135`; every local is an `unaff_EBP` slot and the
//   surviving body mixes three non-returning teardown calls.
// * `0065e335` - bare thunk: `this->vtable[4]([ebp - 0x54])` with an
//   unrecovered stack word, then the non-returning `00c86b6e`.
// * `0065e450` - one-line dispatch through the object at `this + 0x0c`
//   with a constant `0`; neither the object type nor the slot has a
//   recovered contract.
// * `0065e8a0`, `0065e9d0`, `0065ea1c`, `0065ead0`, `0065eb45`,
//   `0065eb81` - one large string/hash-table routine.  `0065e8a0`
//   starts it (`strlen`, a local SSO string, hash bucket lookup) and the
//   other rows are interior continuation fragments whose operands live in
//   `unaff_EBP`/`unaff_ESI`/`unaff_EDI`; the hash table globals are not
//   self-contained.
// * `0065ec45` - fragment: a 64-bit delta from `[esi + 0x10]` and
//   `[esi + 0x14]`, `FUN_00709680(3)`, then the non-returning
//   `thunk_FUN_0093f6c0`; no arguments or return value survive.
// * `0065ee0c` - epilogue fragment (`add esp, 8`, `mov eax, esi`, SEH
//   restore, `ret 4`); no recovered operands.
// * `0065ee50` - registry/string insert that spans to `0065f3af`
//   through the globals `0x00f05e60..0x00f05e64`; the decompiled body is
//   truncated at the first table insert and is not a self-contained
//   routine.
// * `0065f1a0`, `0065f1c5` - dominated by
//   `WARNING: Control flow encountered bad instruction data` and
//   `halt_baddata`; the recovered blocks are pointer arithmetic on
//   `unaff_*` registers.
// * `0065f39f` - a one-byte `halt_baddata`.
// * `0065f3d0` - hash-table insert/erase over the globals
//   `0x00f05f18`, `0x00f05f1c` and `0x00f05f20`, with an SEH frame and a
//   bucket-chain removal; the table object layout is not visible in the
//   body.
// * `0065f413` - 11-byte thunk: push `EAX`, call `0065e8a0`, jump to
//   the routine epilogue.
// * `0065f4f7`, `0065f53d`, `0065f553` - cleanup landing pads of
//   `0065f3d0`; they start mid-sequence and use `unaff_EDI`/`unaff_EBX`
//   and the global counter `0x00f05f20`.
// * `0065f5e9` - a bare `ret`.
// * `0065f7a0` - button-condition routine that reads `0x0119f7bc`,
//   `0x011b8692` and the table through `0x011a0e38`; it always answers `1`
//   and the side-effect body is a client reset that cannot be expressed
//   without those global layouts.
// * `0065fe00` - button-condition routine whose body continues far past
//   the listed 74 bytes and reads the scenario/team globals
//   `0x0119f7bc`, `0x00f2acf4` and `0x00f063cc`.

// ---------------------------------------------------------------------------
// Recovered constants
// ---------------------------------------------------------------------------

/// Size of one table record (`0x0065e3a0`, `0x0065e700`).
pub const W13A02_RECORD_BYTES: usize = 10;

/// Number of `u16` words in one table record.
pub const W13A02_RECORD_WORDS: usize = 5;

/// The empty record written by `0x0065e700` and reset by `0x0065e3a0`:
/// `[0xfffe, 0xffff, 0xffff, 0, 0]`.
pub const W13A02_RECORD_EMPTY: [u16; W13A02_RECORD_WORDS] =
    [0xfffe, 0xffff, 0xffff, 0x0000, 0x0000];

/// Low 31 bits of a vector header capacity word.
pub const W13A02_CAPACITY_MASK: u32 = 0x7fff_ffff;

/// Bit 31 of a vector header capacity word.  When it is **clear** the
/// routine owns the buffer and frees it after reallocation; when it is set
/// the buffer belongs to someone else and is left alone.
pub const W13A02_CAPACITY_OWNED_FLAG: u32 = 0x8000_0000;

/// Nanoseconds per second used by the `0x0065e870` comparison.
pub const W13A02_NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Vtable installed by the `TimedScope` constructor `0x0065ebe0`.
pub const W13A02_TIMED_SCOPE_VTABLE: u32 = 0x00d3_d364;

/// Vtable installed by the `InGameLagTimedScope` rows `0x0065ec10` and
/// `0x0065ed10`.
pub const W13A02_IN_GAME_LAG_TIMED_SCOPE_VTABLE: u32 = 0x00d3_d36c;

/// Object size passed to the CRT delete helper by `0x0065ee30`.
pub const W13A02_DELETED_OBJECT_BYTES: usize = 0x18;

// ---------------------------------------------------------------------------
// 0x0065e460 / 0x0065e640 / 0x0065e700 - shared capacity word
// ---------------------------------------------------------------------------

/// `0x0065e460`, `0x0065e640`, `0x0065e700`: the capacity stored in the low
/// 31 bits of a vector header word.
pub fn w13a02_capacity(word: u32) -> u32 {
    word & W13A02_CAPACITY_MASK
}

/// `0x0065e460`, `0x0065e640`, `0x0065e700`: whether the header owns its
/// buffer.  The shipped code tests bit 31 with a signed "less than zero"
/// branch, so a clear bit means owned.
pub fn w13a02_capacity_is_owned(word: u32) -> bool {
    word & W13A02_CAPACITY_OWNED_FLAG == 0
}

/// `0x0065e460`, `0x0065e640`, `0x0065e700`: build a header word with the
/// given low-31-bit capacity while preserving the ownership flag.
pub fn w13a02_capacity_word(capacity: u32, owned: bool) -> u32 {
    let flag = if owned { 0 } else { W13A02_CAPACITY_OWNED_FLAG };
    (capacity & W13A02_CAPACITY_MASK) | flag
}

/// `0x0065e460`, `0x0065e640`, `0x0065e700`: capacity growth.
///
/// The low 31 bits are the current capacity and the high bit is preserved.
/// When `needed` does not fit, the new capacity is
/// `max(needed, capacity + capacity / 2)`.  A request that already fits
/// returns the word unchanged, exactly like the three routines' early
/// `jbe` exits.
pub fn w13a02_grow_capacity_word(word: u32, needed: u32) -> u32 {
    let capacity = w13a02_capacity(word);
    if needed <= capacity {
        return word;
    }
    let grown = capacity + (capacity >> 1);
    let new_capacity = grown.max(needed);
    (word ^ new_capacity) & W13A02_CAPACITY_MASK ^ word
}

// ---------------------------------------------------------------------------
// 0x0065e460 - grow a byte vector and return the inserted span
// ---------------------------------------------------------------------------

/// The three output words `0x0065e460` stores through its first argument.
///
/// The shipped routine returns pointers; the helper returns the equivalent
/// offsets from the (possibly relocated) buffer start.  The middle-insert
/// branch stores the old end in the second word, which is why the fields are
/// named positionally instead of being documented as a plain range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a02ByteSpan {
    /// First output word: the insertion position.
    pub first: usize,
    /// Second output word: the insertion position for the append/relocate
    /// paths and the old end for the middle-insert path.
    pub second: usize,
    /// Third output word: one past the inserted range.
    pub third: usize,
}

/// `0x0065e460`: make room for `count` uninitialised bytes at offset `at`,
/// updating the recovered capacity word and returning the output span.
///
/// The full body is longer than the ledger's 306-byte row; the recovered
/// disassembly continues to `0x0065e640`.  The routine reads `begin`,
/// `count` and the capacity word from the header, grows through the
/// allocator singleton `0093ec70` (alignment `8`) when necessary, moves
/// the old tail right by `count`, and stores three words through `out`.
///
/// All recovered call sites pass `at = 0` (a fresh local vector), so the
/// relocation path is the common one: it answers `(0, 0, count)` relative
/// to the new allocation.  The non-relocating append path answers
/// `(at, at, at + count)`; the middle-insert path answers
/// `(at, old_len, at + count)`.  The new bytes are uninitialised in the
/// shipped code; this safe model writes zeroes.
///
/// Returns `None` when `at` lies past the end or the requested size does not
/// fit a 32-bit capacity word.
pub fn w13a02_byte_vector_insert_uninit(
    bytes: &mut Vec<u8>,
    capacity_word: &mut u32,
    at: usize,
    count: usize,
) -> Option<W13a02ByteSpan> {
    let old_len = bytes.len();
    if at > old_len {
        return None;
    }
    if count == 0 {
        let second = if at < old_len { old_len } else { at };
        return Some(W13a02ByteSpan {
            first: at,
            second,
            third: at,
        });
    }
    let new_len = old_len.checked_add(count)?;
    let required = u32::try_from(new_len).ok()?;
    let relocates = required > w13a02_capacity(*capacity_word);
    if relocates {
        *capacity_word = w13a02_grow_capacity_word(*capacity_word, required);
    }
    bytes.resize(new_len, 0);
    bytes.copy_within(at..old_len, at + count);
    // The shipped code leaves the inserted range uninitialised; use zeroes.
    bytes[at..at + count].fill(0);
    if relocates {
        Some(W13a02ByteSpan {
            first: 0,
            second: 0,
            third: count,
        })
    } else if at + count >= old_len {
        Some(W13a02ByteSpan {
            first: at,
            second: at,
            third: at + count,
        })
    } else {
        Some(W13a02ByteSpan {
            first: at,
            second: old_len,
            third: at + count,
        })
    }
}

// ---------------------------------------------------------------------------
// 0x0065e640 - reserve a u16 vector
// ---------------------------------------------------------------------------

/// Result of the `0x0065e640` reserve step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a02Reserve {
    /// Whether the requested capacity exceeded the old capacity.
    pub grew: bool,
    /// Whether the old buffer was owned (bit 31 clear).
    pub old_was_owned: bool,
    /// Capacity in elements after the step.
    pub capacity: u32,
}

/// `0x0065e640`: reserve room for `requested` `u16` elements.
///
/// The header is `begin` at `+0x00`, element count at `+0x04` and the
/// capacity word at `+0x08`; the element stride is `2`.  A request that fits
/// the low 31 bits of the word does nothing.  Otherwise the routine applies
/// [`w13a02_grow_capacity_word`], allocates `new_capacity * 2` bytes through
/// `0093ec70` with alignment `8`, copies the current elements, frees the
/// old buffer only when the old ownership flag was clear, and stores the new
/// begin and capacity.
///
/// `Vec` performs the copy; the capacity word is updated with the recovered
/// policy rather than `Vec::capacity()`, because the shipped word is the
/// authoritative size for the caller.
pub fn w13a02_reserve_u16(
    values: &mut Vec<u16>,
    capacity_word: &mut u32,
    requested: usize,
) -> Option<W13a02Reserve> {
    let requested = u32::try_from(requested).ok()?;
    let old_word = *capacity_word;
    let capacity = w13a02_capacity(old_word);
    if requested <= capacity {
        return Some(W13a02Reserve {
            grew: false,
            old_was_owned: w13a02_capacity_is_owned(old_word),
            capacity,
        });
    }
    *capacity_word = w13a02_grow_capacity_word(old_word, requested);
    let new_capacity = w13a02_capacity(*capacity_word);
    if new_capacity as usize > values.len() {
        values.reserve(new_capacity as usize - values.len());
    }
    Some(W13a02Reserve {
        grew: true,
        old_was_owned: w13a02_capacity_is_owned(old_word),
        capacity: new_capacity,
    })
}

// ---------------------------------------------------------------------------
// 0x0065e700 - resize the 10-byte record vector
// ---------------------------------------------------------------------------

/// Result of the `0x0065e700` resize step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a02Resize {
    /// The new length equals the old length.
    Unchanged,
    /// The vector was truncated; the removed records are at the end.
    Truncated { len: usize },
    /// The vector was extended with [`W13A02_RECORD_EMPTY`] records.
    Extended { len: usize, reallocated: bool },
}

/// `0x0065e700`: resize a vector of 10-byte records.
///
/// The ledger row is 209 bytes, but the recovered body continues to
/// `0x0065e870`.  Shrinking only rewrites the count at `+0x04`; the shipped
/// `memmove` at the shrink site is passed a zero length because the removed
/// records sit at the end (the records are trivially destructible), so the
/// safe model is a plain truncation.
///
/// Growing checks the capacity word at `+0x08`, applies
/// [`w13a02_grow_capacity_word`], allocates `new_capacity * 10` bytes through
/// `0093ec70` with alignment `8`, copies the old records, frees the old
/// buffer only when it was owned, and fills every new record with
/// `[0xfffe, 0xffff, 0xffff, 0, 0]`.
///
/// Returns `None` only when the requested length does not fit a 32-bit
/// capacity.
pub fn w13a02_resize_records(
    records: &mut Vec<[u16; W13A02_RECORD_WORDS]>,
    capacity_word: &mut u32,
    new_len: usize,
) -> Option<W13a02Resize> {
    let old_len = records.len();
    if new_len < old_len {
        records.truncate(new_len);
        return Some(W13a02Resize::Truncated { len: new_len });
    }
    if new_len == old_len {
        return Some(W13a02Resize::Unchanged);
    }
    let required = u32::try_from(new_len).ok()?;
    let reallocated = required > w13a02_capacity(*capacity_word);
    if reallocated {
        *capacity_word = w13a02_grow_capacity_word(*capacity_word, required);
    }
    records.resize(new_len, W13A02_RECORD_EMPTY);
    Some(W13a02Resize::Extended {
        len: new_len,
        reallocated,
    })
}

// ---------------------------------------------------------------------------
// 0x0065e3a0 - initialise one table slot
// ---------------------------------------------------------------------------

/// Two possible binding call sequences `0x0065e3a0` performs through
/// `0065c1f0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a02BindPlan {
    /// `param_4 == 0`: one call with selector `0` and `param_2`.
    Single { value: u32 },
    /// `param_4 != 0`: call selector `1` with `param_2`, then selector `2`
    /// with `param_4`.
    Pair { first: u32, second: u32 },
}

/// `0x0065e3a0`: initialise table record `index` and return the binding plan.
///
/// ```c
/// record = records + index * 10;
/// if (record[0] == index) return 0;          // already present
/// record[0..5] = [0xfffe, 0xffff, 0xffff, 0, 0];
/// record[0] = index;
/// if (param_4 == 0) bind(0, record, param_2);
/// else { bind(1, record, param_2); bind(2, record, param_4); }
/// flags[index] = param_3;
/// return 1;
/// ```
///
/// The second argument of `0065c1f0` is the record pointer, so the
/// callback receives the record index in that position.  `bind` is called
/// with `(selector, record_index, value)` in the shipped order; the flag
/// byte is written last.  Returns `None` when the record already exists or
/// either the record array or the flag array is too short.
pub fn w13a02_init_table_slot<F>(
    records: &mut [[u16; W13A02_RECORD_WORDS]],
    flags: &mut [u8],
    index: u16,
    param_2: u32,
    param_3: u8,
    param_4: u32,
    mut bind: F,
) -> Option<W13a02BindPlan>
where
    F: FnMut(u16, u16, u32),
{
    let slot = index as usize;
    if slot >= records.len() || slot >= flags.len() {
        return None;
    }
    if records[slot][0] == index {
        return None;
    }
    records[slot] = W13A02_RECORD_EMPTY;
    records[slot][0] = index;
    let plan = if param_4 == 0 {
        bind(0, index, param_2);
        W13a02BindPlan::Single { value: param_2 }
    } else {
        bind(1, index, param_2);
        bind(2, index, param_4);
        W13a02BindPlan::Pair {
            first: param_2,
            second: param_4,
        }
    };
    flags[slot] = param_3;
    Some(plan)
}

// ---------------------------------------------------------------------------
// 0x0065e870 - nanosecond threshold comparison
// ---------------------------------------------------------------------------

/// `0x0065e870`: true when `threshold` is strictly after `ticks` converted
/// to whole seconds.
///
/// The shipped code calls the 64-bit unsigned divide helper
/// `00c86d80(ticks, 1_000_000_000)` and then compares the 64-bit result
/// with the two words at `param_1` (`+0x00` low, `+0x04` high`).  The high
/// words are compared with signed `jl`/`jg` branches while the final low
/// comparison is unsigned `jbe`, i.e. a signed 64-bit `>` on the bit
/// patterns.  The helper keeps that signed comparison.
pub fn w13a02_time_threshold_exceeded(threshold: u64, ticks: u64) -> bool {
    let seconds = ticks / W13A02_NANOS_PER_SECOND;
    (threshold as i64) > (seconds as i64)
}

// ---------------------------------------------------------------------------
// 0x0065ebe0 / 0x0065ec10 / 0x0065ed10 - scope headers
// ---------------------------------------------------------------------------

/// The two recovered scope vtable identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a02ScopeVtable {
    /// `0x00d3d364`: `TimedScope`.
    TimedScope,
    /// `0x00d3d36c`: `InGameLagTimedScope`.
    InGameLagTimedScope,
}

impl W13a02ScopeVtable {
    /// The shipped vtable address.
    pub const fn address(self) -> u32 {
        match self {
            Self::TimedScope => W13A02_TIMED_SCOPE_VTABLE,
            Self::InGameLagTimedScope => W13A02_IN_GAME_LAG_TIMED_SCOPE_VTABLE,
        }
    }

    /// Recover the identity from the shipped vtable address.
    pub const fn from_address(address: u32) -> Option<Self> {
        match address {
            W13A02_TIMED_SCOPE_VTABLE => Some(Self::TimedScope),
            W13A02_IN_GAME_LAG_TIMED_SCOPE_VTABLE => Some(Self::InGameLagTimedScope),
            _ => None,
        }
    }
}

/// The recovered prefix of both timer-scope objects: a vtable pointer at
/// `+0x00` and, for `TimedScope`, a context word at `+0x08`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a02ScopeHeader {
    /// `+0x00`: vtable address.
    pub vtable: u32,
    /// `+0x08`: context word (`TimedScope` constructor argument).
    pub context: u32,
}

/// `0x0065ebe0`: `TimedScope` constructor header.
///
/// ```c
/// this->vtable = TimedScope::vftable;   // 0x00d3d364
/// this->context = param_1;              // +0x08
/// ```
///
/// The trailing `00c66860` call is the shipped timer/cookie helper and
/// does not touch the two recovered header words.
pub fn w13a02_timed_scope_new(context: u32) -> W13a02ScopeHeader {
    W13a02ScopeHeader {
        vtable: W13a02ScopeVtable::TimedScope.address(),
        context,
    }
}

/// `0x0065ec10`: install the `InGameLagTimedScope` vtable on `header`.
///
/// The only recovered object store is `this->vtable = 0x00d3d36c`; the rest
/// of the routine is the MSVC SEH/prologue trailer.
pub fn w13a02_in_game_lag_scope_init(header: &mut W13a02ScopeHeader) {
    header.vtable = W13a02ScopeVtable::InGameLagTimedScope.address();
}

/// `0x0065ed10`: teardown/install the `InGameLagTimedScope` vtable.
///
/// The routine writes the same `0x00d3d36c` vtable as `0x0065ec10`; its
/// `[ebp - 4] = 0` store is the SEH state slot, not a field of the object.
pub fn w13a02_in_game_lag_scope_reset(header: &mut W13a02ScopeHeader) {
    header.vtable = W13a02ScopeVtable::InGameLagTimedScope.address();
}

// ---------------------------------------------------------------------------
// 0x0065ee30 - scalar deleting destructor predicate
// ---------------------------------------------------------------------------

/// Result of the `0x0065ee30` scalar deleting destructor prologue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a02DeleteAction {
    /// Whether bit 0 of the delete flag is set.
    pub delete_object: bool,
    /// Size passed to the CRT delete helper when deleting (`0x18`).
    pub size: usize,
}

/// `0x0065ee30`: MSVC scalar deleting destructor predicate.
///
/// ```c
/// if (flags & 1) operator_delete(this, 0x18);
/// return this;
/// ```
///
/// The flag word is the second `thiscall` argument pushed by the caller.
/// Only the recovered predicate and the constant object size are modelled;
/// the delete itself is the CRT routine `00923e90`.
pub fn w13a02_scalar_deleting_destructor(flags: u8) -> W13a02DeleteAction {
    W13a02DeleteAction {
        delete_object: flags & 1 != 0,
        size: W13A02_DELETED_OBJECT_BYTES,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_wave13_agent02 {
    use super::*;

    // -- capacity word ------------------------------------------------------

    /// The shared capacity policy of `0x0065e460`, `0x0065e640` and
    /// `0x0065e700`.
    #[test]
    fn capacity_word_layout_and_growth() {
        assert_eq!(W13A02_RECORD_BYTES, W13A02_RECORD_WORDS * 2);
        assert_eq!(w13a02_capacity(0x8000_0004), 4);
        assert!(w13a02_capacity_is_owned(0x0000_0004));
        assert!(!w13a02_capacity_is_owned(0x8000_0004));
        assert_eq!(w13a02_capacity_word(7, true), 7);
        assert_eq!(w13a02_capacity_word(7, false), 0x8000_0007);

        // Fitting requests are a no-op, including the high bit.
        assert_eq!(w13a02_grow_capacity_word(0x8000_0008, 8), 0x8000_0008);
        assert_eq!(w13a02_grow_capacity_word(0x8000_0008, 9), 0x8000_000c);
        assert_eq!(w13a02_grow_capacity_word(0x0000_000a, 20), 20);
        assert_eq!(w13a02_grow_capacity_word(0, 1), 1);
        assert_eq!(w13a02_grow_capacity_word(3, 4), 4);
        // 3 + 1 = 4, but the request needs 5.
        assert_eq!(w13a02_grow_capacity_word(3, 5), 5);
    }

    // -- 0x0065e460 ---------------------------------------------------------

    /// `0x0065e460`: fresh allocation, append and middle-insert spans.
    #[test]
    fn byte_vector_insert_span() {
        let mut bytes = Vec::new();
        let mut word = 0u32;
        let span = w13a02_byte_vector_insert_uninit(&mut bytes, &mut word, 0, 3).unwrap();
        assert_eq!(
            span,
            W13a02ByteSpan {
                first: 0,
                second: 0,
                third: 3
            }
        );
        assert_eq!(bytes, vec![0, 0, 0]);
        assert_eq!(word, 3);

        // Append with enough capacity keeps the span at the old end.
        word = 10;
        let span = w13a02_byte_vector_insert_uninit(&mut bytes, &mut word, 3, 2).unwrap();
        assert_eq!(
            span,
            W13a02ByteSpan {
                first: 3,
                second: 3,
                third: 5
            }
        );
        assert_eq!(bytes.len(), 5);

        // Middle insert: the recovered second word is the old end.
        let mut bytes = vec![1u8, 2, 3];
        let mut word = 10;
        let span = w13a02_byte_vector_insert_uninit(&mut bytes, &mut word, 1, 1).unwrap();
        assert_eq!(
            span,
            W13a02ByteSpan {
                first: 1,
                second: 3,
                third: 2
            }
        );
        assert_eq!(bytes, vec![1, 0, 2, 3]);
        assert_eq!(word, 10);

        // Zero-byte insert at the end and in the middle, and a bad offset.
        assert_eq!(
            w13a02_byte_vector_insert_uninit(&mut bytes, &mut word, 4, 0).unwrap(),
            W13a02ByteSpan {
                first: 4,
                second: 4,
                third: 4
            }
        );
        assert_eq!(
            w13a02_byte_vector_insert_uninit(&mut bytes, &mut word, 1, 0).unwrap(),
            W13a02ByteSpan {
                first: 1,
                second: 4,
                third: 1
            }
        );
        assert!(w13a02_byte_vector_insert_uninit(&mut bytes, &mut word, 5, 1).is_none());

        // Relocation follows the recovered `max(needed, cap + cap / 2)` rule.
        let mut bytes = vec![9u8];
        let mut word = 1;
        let span = w13a02_byte_vector_insert_uninit(&mut bytes, &mut word, 0, 2).unwrap();
        assert_eq!(
            span,
            W13a02ByteSpan {
                first: 0,
                second: 0,
                third: 2
            }
        );
        assert_eq!(word, 3);
        assert_eq!(bytes, vec![0, 0, 9]);
    }

    // -- 0x0065e640 ---------------------------------------------------------

    /// `0x0065e640`: growth, ownership preservation and no-op requests.
    #[test]
    fn reserve_u16_updates_capacity_word() {
        let mut values = vec![7u16, 8];
        let mut word = 0x8000_0004; // four elements, foreign buffer

        let result = w13a02_reserve_u16(&mut values, &mut word, 4).unwrap();
        assert!(!result.grew);
        assert!(!result.old_was_owned);
        assert_eq!(result.capacity, 4);
        assert_eq!(word, 0x8000_0004);

        // 4 + 2 = 6, which already covers the request.
        let result = w13a02_reserve_u16(&mut values, &mut word, 5).unwrap();
        assert!(result.grew);
        assert_eq!(result.capacity, 6);
        assert_eq!(word, 0x8000_0006);
        assert_eq!(values, vec![7, 8]);

        // A request past the geometric growth wins verbatim.
        let mut word = 2u32;
        let result = w13a02_reserve_u16(&mut values, &mut word, 9).unwrap();
        assert_eq!(result.capacity, 9);
        assert_eq!(word, 9);
        assert!(result.old_was_owned);
    }

    // -- 0x0065e700 ---------------------------------------------------------

    /// `0x0065e700`: extend fills the canonical empty record, shrink truncates.
    #[test]
    fn resize_records_fills_and_truncates() {
        let mut records: Vec<[u16; 5]> = Vec::new();
        let mut word = 0u32;

        assert_eq!(
            w13a02_resize_records(&mut records, &mut word, 2).unwrap(),
            W13a02Resize::Extended {
                len: 2,
                reallocated: true
            }
        );
        assert_eq!(records.len(), 2);
        assert_eq!(records[0], W13A02_RECORD_EMPTY);
        assert_eq!(records[1], [0xfffe, 0xffff, 0xffff, 0, 0]);
        assert_eq!(word, 2);

        // A second extension that fits the word does not reallocate.
        assert_eq!(
            w13a02_resize_records(&mut records, &mut word, 2).unwrap(),
            W13a02Resize::Unchanged
        );
        assert_eq!(
            w13a02_resize_records(&mut records, &mut word, 3).unwrap(),
            W13a02Resize::Extended {
                len: 3,
                reallocated: true
            }
        );
        assert_eq!(word, 3); // max(3, 2 + 1)

        assert_eq!(
            w13a02_resize_records(&mut records, &mut word, 1).unwrap(),
            W13a02Resize::Truncated { len: 1 }
        );
        assert_eq!(records.len(), 1);
        assert_eq!(word, 3, "truncation does not touch the capacity word");
    }

    // -- 0x0065e3a0 ---------------------------------------------------------

    /// `0x0065e3a0`: duplicate detection, record reset and bind order.
    #[test]
    fn table_slot_init_and_bind_order() {
        let mut records = vec![W13A02_RECORD_EMPTY; 2];
        let mut flags = vec![0u8; 2];
        let mut calls = Vec::new();

        let plan = w13a02_init_table_slot(
            &mut records,
            &mut flags,
            1,
            0x1111,
            0x7f,
            0,
            |selector, index, value| calls.push((selector, index, value)),
        )
        .unwrap();
        assert_eq!(plan, W13a02BindPlan::Single { value: 0x1111 });
        assert_eq!(records[1], [1, 0xffff, 0xffff, 0, 0]);
        assert_eq!(flags[1], 0x7f);
        assert_eq!(calls, vec![(0, 1, 0x1111)]);

        // Existing records are left alone and do not bind again.
        calls.clear();
        assert!(
            w13a02_init_table_slot(&mut records, &mut flags, 1, 0, 0, 0, |s, i, v| {
                calls.push((s, i, v))
            })
            .is_none()
        );
        assert!(calls.is_empty());
        assert_eq!(flags[1], 0x7f);

        // A non-zero fourth argument issues the selector 1 then 2 pair.
        let plan =
            w13a02_init_table_slot(&mut records, &mut flags, 0, 0x2222, 3, 0x3333, |s, i, v| {
                calls.push((s, i, v))
            })
            .unwrap();
        assert_eq!(
            plan,
            W13a02BindPlan::Pair {
                first: 0x2222,
                second: 0x3333
            }
        );
        assert_eq!(calls, vec![(1, 0, 0x2222), (2, 0, 0x3333)]);
        assert_eq!(records[0], [0, 0xffff, 0xffff, 0, 0]);
        assert_eq!(flags[0], 3);

        // Out-of-range slots are refused rather than writing either array.
        assert!(
            w13a02_init_table_slot(&mut records, &mut flags, 2, 1, 1, 0, |_, _, _| {}).is_none()
        );
    }

    // -- 0x0065e870 ---------------------------------------------------------

    /// `0x0065e870`: whole-second conversion and the signed 64-bit compare.
    #[test]
    fn time_threshold_uses_whole_seconds() {
        assert!(!w13a02_time_threshold_exceeded(0, 0));
        assert!(!w13a02_time_threshold_exceeded(0, 999_999_999));
        assert!(!w13a02_time_threshold_exceeded(1, 1_000_000_000));
        assert!(w13a02_time_threshold_exceeded(2, 1_000_000_000));
        assert!(!w13a02_time_threshold_exceeded(1, 1_999_999_999));
        assert!(w13a02_time_threshold_exceeded(2, 1_999_999_999));

        // The shipped high-word branches are signed; a wrapped threshold is
        // therefore not "larger" than a small quotient.
        assert!(!w13a02_time_threshold_exceeded(u64::MAX, 0));
        assert!(w13a02_time_threshold_exceeded(0x8000_0000, 0));
    }

    // -- 0x0065ebe0 / 0x0065ec10 / 0x0065ed10 -------------------------------

    /// The two recovered vtable identities and the context field.
    #[test]
    fn scope_headers_install_recovered_vtables() {
        assert_eq!(W13a02ScopeVtable::TimedScope.address(), 0x00d3_d364);
        assert_eq!(
            W13a02ScopeVtable::InGameLagTimedScope.address(),
            0x00d3_d36c
        );
        assert_eq!(
            W13a02ScopeVtable::from_address(0x00d3_d36c),
            Some(W13a02ScopeVtable::InGameLagTimedScope)
        );
        assert_eq!(W13a02ScopeVtable::from_address(0), None);

        let mut header = w13a02_timed_scope_new(0x1234_5678);
        assert_eq!(
            header,
            W13a02ScopeHeader {
                vtable: W13A02_TIMED_SCOPE_VTABLE,
                context: 0x1234_5678
            }
        );

        w13a02_in_game_lag_scope_init(&mut header);
        assert_eq!(header.vtable, W13A02_IN_GAME_LAG_TIMED_SCOPE_VTABLE);
        assert_eq!(header.context, 0x1234_5678);

        header.vtable = W13A02_TIMED_SCOPE_VTABLE;
        w13a02_in_game_lag_scope_reset(&mut header);
        assert_eq!(header.vtable, W13A02_IN_GAME_LAG_TIMED_SCOPE_VTABLE);
    }

    // -- 0x0065ee30 ---------------------------------------------------------

    /// `0x0065ee30`: only bit 0 of the delete flag selects the free path.
    #[test]
    fn scalar_deleting_destructor_flag() {
        for (flags, deletes) in [
            (0u8, false),
            (1, true),
            (2, false),
            (0xfe, false),
            (0xff, true),
        ] {
            let action = w13a02_scalar_deleting_destructor(flags);
            assert_eq!(action.delete_object, deletes, "flags = {flags:#x}");
            assert_eq!(action.size, 0x18);
        }
    }
}
