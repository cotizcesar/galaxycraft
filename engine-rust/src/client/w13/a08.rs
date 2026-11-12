#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave13/agent08.rs
// ===========================================================================
// Wave 13 translation batch 08.
//
// Window: `0x0066a000..0x0066bfff`.  The shipped client's callable,
// string and settings glue lives here: six copyable-function constructors,
// their RTTI descriptors, the two string headers used by the settings
// loader, the filtered callable walk and the ranked settings-record scan.
//
// Every translation below is a free function over caller-owned values.
// The engine keeps the actual containers, allocator and CRT once-guard, so
// those remain opaque; only the arithmetic, the recovered constants and the
// object layouts are reproduced.
//
// ## Recovered layout
//
// | object | offset | field |
// |--------|--------|-------|
// | copyable function | `+0x00` | concrete vtable identity |
// | copyable function | `+0x04` | captured byte or word |
// | copyable function | `+0x08..+0x17` | four captured words (four-word lambda only) |
// | callable handle | bit 0 | `1` = heap form, `0` = inline form |
// | callable handle | bits 1.. | heap object address when bit 0 is set |
// | owned string | `+0x00` | data pointer |
// | owned string | `+0x04` | byte length |
// | owned string | `+0x08` | capacity word; `0x8000000f` = 15 inline bytes |
// | owned string | `+0x0c` | sixteen-byte inline buffer |
// | string view | `+0x00` | byte length |
// | string view | `+0x04` | reserved (zero) |
// | string view | `+0x08` | flags, `0x04050000` |
// | string view | `+0x0c` | data pointer |
// | small-string copy | `+0x08` | `0x1c050000 \| spare << 8` inline, `0x0c050000` heap |
// | small-string copy | `+0x0c` | last inline byte / heap pointer |
// | ranked record | stride `0x98` | payload at `+0x00`, `+0x04` |
// | ranked record | `+0x48` | unsigned low rank |
// | ranked record | `+0x4c` | signed high rank |
// | settings host | `+0x10` | record count |
// | settings host | `+0x18` | record base |
// | settings host | `+0x04` | owner object |
// | settings owner | `+0x48` | load-state word |
//
// ## Untranslated rows
//
// The remaining rows in the window are unrecovered jump-table dispatchers,
// decompiler failures (`bad instruction data`, overlapping instructions),
// or exception-cleanup fragments built entirely from `unaff_*` frame slots.
// Their addresses and individual reasons are listed in the batch report
// rather than here, so this file cites only code that it actually
// reproduces.

/// Byte offset of the embedded sub-object returned by the four-byte row.
pub const W13A08_EMBEDDED_OBJECT_OFFSET: usize = 0x08;

/// Capacity word of an empty owned string: fifteen inline bytes plus the
/// inline tag bit.
pub const W13A08_OWNED_STRING_INLINE_CAPACITY: u32 = 0x8000_000f;

/// Offset of the inline buffer inside an owned string object.
pub const W13A08_OWNED_STRING_INLINE_OFFSET: u32 = 0x0000_000c;

/// Flags written into the sixteen-byte string view built by the loader.
pub const W13A08_STRING_VIEW_FLAGS: u32 = 0x0405_0000;

/// Maximum number of bytes that fit in the small-string copy inline form.
pub const W13A08_SMALL_STRING_MAX_INLINE: usize = 0x0d;

/// Exclusive length limit of the small-string copy inline form: a length of
/// fourteen or more switches to a heap buffer.
pub const W13A08_SMALL_STRING_INLINE_LIMIT: usize = 0x0e;

/// High half of the inline capacity word (`0x1c050000 | spare << 8`).
pub const W13A08_SMALL_STRING_INLINE_FLAGS: u32 = 0x1c05_0000;

/// Capacity word used when the small-string copy allocates a heap buffer.
pub const W13A08_SMALL_STRING_HEAP_FLAGS: u32 = 0x0c05_0000;

/// Stride of one settings record in the byte table addressed by the host.
pub const W13A08_TABLE_RECORD_STRIDE: usize = 0x20;

/// Stride of one ranked settings record.
pub const W13A08_RANKED_RECORD_STRIDE: usize = 0x98;

/// Offset of the unsigned low rank inside a ranked settings record.
pub const W13A08_RANKED_RECORD_LOW_OFFSET: usize = 0x48;

/// Offset of the signed high rank inside a ranked settings record.
pub const W13A08_RANKED_RECORD_HIGH_OFFSET: usize = 0x4c;

/// Upper bound accepted for the first descriptor word.
pub const W13A08_FILTER_FIRST_LIMIT: u32 = 0x14;

/// Upper bound accepted for the second descriptor word.
pub const W13A08_FILTER_SECOND_LIMIT: u32 = 0x04;

/// Load-state word written when the settings load fails (`3`).
pub const W13A08_LOAD_STATE_FAILED: i32 = 3;

/// Payload size of the lazily constructed settings singleton.
pub const W13A08_SINGLETON_BYTES: usize = 0xb8;

/// First constructor argument of the settings singleton.
pub const W13A08_SINGLETON_CTOR_ARG0: u32 = 0x0000_0000;

/// Second constructor argument of the settings singleton.
pub const W13A08_SINGLETON_CTOR_ARG1: u32 = 0x0000_0032;

/// Path-constructor argument used by the settings-path builder (`1`).
pub const W13A08_PATH_CONSTRUCT_ARGUMENT: u32 = 1;

// ---------------------------------------------------------------------------
// Owned string header
// ---------------------------------------------------------------------------

/// Where the `+0x00` data pointer of an owned string points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a08OwnedStringData {
    /// The pointer names the inline buffer at `object + offset`.
    Inline { offset: u32 },
    /// The pointer names a heap allocation.
    Heap { address: u32 },
}

/// Recovered view of the twenty-eight-byte owned string.
///
/// The row at `0x0066a680` only constructs the empty inline state; the
/// character store is not copied here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a08OwnedString {
    /// `+0x00`: data pointer.
    pub data: W13a08OwnedStringData,
    /// `+0x04`: byte length.
    pub len: u32,
    /// `+0x08`: capacity word.
    pub capacity: u32,
    /// `+0x0c`: sixteen-byte inline buffer.
    pub inline: [u8; 16],
}

/// `0x0066a680`: initialise an empty owned string.
///
/// The row stores `+0x00 = object + 0x0c`, `+0x04 = 0`, `+0x08 =
/// 0x8000000f` and clears the first inline byte.  The length it measured
/// from the caller's character source is forwarded to the assignment helper
/// (an out-of-window tail call), so it is not part of the returned header.
pub fn w13a08_empty_owned_string() -> W13a08OwnedString {
    W13a08OwnedString {
        data: W13a08OwnedStringData::Inline {
            offset: W13A08_OWNED_STRING_INLINE_OFFSET,
        },
        len: 0,
        capacity: W13A08_OWNED_STRING_INLINE_CAPACITY,
        inline: [0; 16],
    }
}

/// Recovered view of the settings-path object: the path constructor argument
/// plus the empty owned string the row attaches to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a08SettingsPath {
    /// Argument passed to the path constructor (`1`).
    pub path_argument: u32,
    /// Empty string the row prepares for the settings file name.
    pub name: W13a08OwnedString,
}

/// `0x0066a8d0`: build the settings path and its empty name buffer.
///
/// The row constructs a path object with the constant argument `1`, measures
/// the settings file name, then initialises an empty owned string with the
/// same `0x8000000f` capacity word as [`w13a08_empty_owned_string`] before
/// tail-calling the assignment helper.  Only the constructor argument and
/// the empty string layout are visible in the body.
pub fn w13a08_new_settings_path() -> W13a08SettingsPath {
    W13a08SettingsPath {
        path_argument: W13A08_PATH_CONSTRUCT_ARGUMENT,
        name: w13a08_empty_owned_string(),
    }
}

// ---------------------------------------------------------------------------
// String view and small-string copy
// ---------------------------------------------------------------------------

/// Recovered view of the sixteen-byte string view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a08StringView<'a> {
    /// `+0x00`: byte length, excluding the terminator.
    pub len: u32,
    /// `+0x04`: reserved word, always zero.
    pub reserved: u32,
    /// `+0x08`: flags, `0x04050000`.
    pub flags: u32,
    /// `+0x0c`: data pointer.
    pub data: &'a [u8],
}

/// `0x0066aea0`: build the sixteen-byte string view for a C string.
///
/// The row measures the NUL-terminated argument, stores the length at `+0x00`
/// and the pointer at `+0x0c`, zeroes `+0x04`, writes `0x04050000` at
/// `+0x08`, and then tail-calls the shared lookup helper.  Pass the bytes
/// before the terminator here.
pub fn w13a08_string_view(data: &[u8]) -> W13a08StringView<'_> {
    W13a08StringView {
        len: data.len() as u32,
        reserved: 0,
        flags: W13A08_STRING_VIEW_FLAGS,
        data,
    }
}

/// Copy plan selected by the small-string builder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a08SmallStringCopy {
    /// Copy into the object's inline storage.
    Inline {
        /// Remaining inline bytes after the copy.
        spare: u8,
        /// Capacity word written at `+0x08`.
        capacity_word: u32,
    },
    /// Allocate a heap buffer for the copy.
    Heap {
        /// Allocation size in bytes (`len + 1`).
        bytes: usize,
        /// Capacity word written at `+0x08`.
        capacity_word: u32,
    },
}

/// `0x0066bf60`: choose the copy plan for the sixteen-byte small string.
///
/// A length below `0x0e` stays inline and encodes the remaining bytes in the
/// second byte of the capacity word: `0x1c050000 | ((0x0d - len) << 8)`.
/// Longer strings take a heap allocation of `len + 1` bytes and the
/// `0x0c050000` capacity word.  The row performs the copy right after this
/// decision; only the plan is reported here.
pub fn w13a08_small_string_copy(len: usize) -> W13a08SmallStringCopy {
    if len < W13A08_SMALL_STRING_INLINE_LIMIT {
        let spare = (W13A08_SMALL_STRING_MAX_INLINE - len) as u8;
        W13a08SmallStringCopy::Inline {
            spare,
            capacity_word: W13A08_SMALL_STRING_INLINE_FLAGS | ((spare as u32) << 8),
        }
    } else {
        W13a08SmallStringCopy::Heap {
            bytes: len.wrapping_add(1),
            capacity_word: W13A08_SMALL_STRING_HEAP_FLAGS,
        }
    }
}

/// `0x0066bf60`: does the lookup result equal the end of the byte table?
///
/// The row compares its lookup result against `count * 0x20 + base`, where
/// the count is at host `+0x10` and the base at host `+0x18`.  Equality takes
/// the name-copy branch; otherwise the row goes straight to the notification
/// helper.
pub fn w13a08_hash_equals_table_end(hash: usize, base: usize, count: usize) -> bool {
    hash == base.wrapping_add(count.wrapping_mul(W13A08_TABLE_RECORD_STRIDE))
}

// ---------------------------------------------------------------------------
// Copyable-function objects
// ---------------------------------------------------------------------------

/// Identity of the six copyable-function specialisations in this window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a08CallableKind {
    /// One-word capture, `void(blz::string const&)` lambda.
    StringRef,
    /// Four-word capture, `void` lambda.
    FourWord,
    /// One-word capture, first `void` lambda.
    Word,
    /// Empty `void(blz::function<void()>)` lambda.
    FunctionVoid,
    /// One-word capture, second `void` lambda.
    WordB,
    /// One-byte capture, `void(bool)` lambda.
    BoolPredicate,
}

/// Captured payload copied by a copyable-function constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a08Capture {
    /// No captured payload.
    None,
    /// One captured word at `+0x04`.
    Word(u32),
    /// One captured byte at `+0x04`.
    Byte(u8),
    /// Four captured words at `+0x08..+0x17`.
    FourWords([u32; 4]),
}

/// Recovered view of a freshly constructed copyable-function object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a08CallableObject {
    /// Concrete vtable identity written at `+0x00`.
    pub kind: W13a08CallableKind,
    /// Captured payload.
    pub capture: W13a08Capture,
}

/// `0x0066a1c0`: copy the string-reference lambda.
///
/// The row writes the base vtable, copies the source word at `+0x04`, then
/// overwrites the vtable with the concrete string-reference identity.  The
/// captured word is the reference/handle payload.
pub fn w13a08_copy_string_ref_lambda(captured: u32) -> W13a08CallableObject {
    W13a08CallableObject {
        kind: W13a08CallableKind::StringRef,
        capture: W13a08Capture::Word(captured),
    }
}

/// `0x0066a1e0`: copy the four-word lambda.
///
/// The row copies the sixteen-byte block at source `+0x08..+0x17` to
/// destination `+0x08..+0x17` and writes the four-word vtable.  It does not
/// touch the destination word at `+0x04`.
pub fn w13a08_copy_four_word_lambda(words: [u32; 4]) -> W13a08CallableObject {
    W13a08CallableObject {
        kind: W13a08CallableKind::FourWord,
        capture: W13a08Capture::FourWords(words),
    }
}

/// `0x0066a200`: copy the first one-word `void` lambda.
pub fn w13a08_copy_word_lambda(captured: u32) -> W13a08CallableObject {
    W13a08CallableObject {
        kind: W13a08CallableKind::Word,
        capture: W13a08Capture::Word(captured),
    }
}

/// `0x0066a220`: write the empty `void(blz::function<void()>)` identity.
///
/// This row writes only the concrete vtable at `+0x00`; the remaining object
/// bytes are supplied by the caller.
pub fn w13a08_copy_empty_lambda() -> W13a08CallableObject {
    W13a08CallableObject {
        kind: W13a08CallableKind::FunctionVoid,
        capture: W13a08Capture::None,
    }
}

/// `0x0066a230`: copy the second one-word `void` lambda.
pub fn w13a08_copy_word_lambda_b(captured: u32) -> W13a08CallableObject {
    W13a08CallableObject {
        kind: W13a08CallableKind::WordB,
        capture: W13a08Capture::Word(captured),
    }
}

/// `0x0066a250`: copy the one-byte predicate lambda.
///
/// Only the byte at source `+0x04` is copied; the three following bytes of
/// the destination word are left as the caller provided them.
pub fn w13a08_copy_bool_lambda(flag: u8) -> W13a08CallableObject {
    W13a08CallableObject {
        kind: W13a08CallableKind::BoolPredicate,
        capture: W13a08Capture::Byte(flag),
    }
}

/// `0x0066b640`: build the one-byte bool callback.
///
/// The row writes the concrete bool-predicate vtable and stores the argument
/// byte at `+0x04` before handing the object to the shared invoker.
pub fn w13a08_bool_callback(value: bool) -> W13a08CallableObject {
    W13a08CallableObject {
        kind: W13a08CallableKind::BoolPredicate,
        capture: W13a08Capture::Byte(value as u8),
    }
}

/// Recovered view of one lambda RTTI descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a08TypeDescriptor {
    /// Lambda whose descriptor this is.
    pub kind: W13a08CallableKind,
}

/// `0x0066ae40`: RTTI descriptor of the string-reference lambda.
///
/// The row returns the address of the lambda's `RTTI_Type_Descriptor`; the
/// descriptor identity is reported as the matching callable kind.
pub fn w13a08_rtti_string_ref() -> W13a08TypeDescriptor {
    W13a08TypeDescriptor {
        kind: W13a08CallableKind::StringRef,
    }
}

/// `0x0066ae50`: RTTI descriptor of the four-word lambda.
pub fn w13a08_rtti_four_word() -> W13a08TypeDescriptor {
    W13a08TypeDescriptor {
        kind: W13a08CallableKind::FourWord,
    }
}

/// `0x0066ae60`: RTTI descriptor of the first one-word `void` lambda.
pub fn w13a08_rtti_word() -> W13a08TypeDescriptor {
    W13a08TypeDescriptor {
        kind: W13a08CallableKind::Word,
    }
}

/// `0x0066ae70`: RTTI descriptor of the empty function lambda.
pub fn w13a08_rtti_function_void() -> W13a08TypeDescriptor {
    W13a08TypeDescriptor {
        kind: W13a08CallableKind::FunctionVoid,
    }
}

/// `0x0066ae80`: RTTI descriptor of the second one-word `void` lambda.
pub fn w13a08_rtti_word_b() -> W13a08TypeDescriptor {
    W13a08TypeDescriptor {
        kind: W13a08CallableKind::WordB,
    }
}

/// `0x0066ae90`: RTTI descriptor of the bool-predicate lambda.
pub fn w13a08_rtti_bool() -> W13a08TypeDescriptor {
    W13a08TypeDescriptor {
        kind: W13a08CallableKind::BoolPredicate,
    }
}

/// Bounded descriptor returned by a node's callable before dispatch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a08CallableDescriptor {
    /// Presence byte; zero rejects the node.
    pub present: bool,
    /// First bounded word; must be at most `0x14`.
    pub first: u32,
    /// Second bounded word; must be at most `4`.
    pub second: u32,
}

impl W13a08CallableDescriptor {
    /// The acceptance test applied by both filtered walks.
    pub fn acceptable(&self) -> bool {
        self.present
            && self.first <= W13A08_FILTER_FIRST_LIMIT
            && self.second <= W13A08_FILTER_SECOND_LIMIT
    }
}

/// `0x0066b920`: walk callable-bearing nodes in key order and dispatch the
/// acceptable ones.
///
/// For every node the row decodes its callable handle (bit 0 is the heap
/// tag; the heap address is `handle & !1`), asks the descriptor method for
/// the three bounded words, aborts when the presence byte is zero or either
/// bound is exceeded, and otherwise hands a copy of the callable to the
/// caller's filter.  Nodes that fail the bound test are skipped.  The return
/// is the number of dispatched nodes.
///
/// The row also tears the owner container down before the walk; that is
/// container plumbing and is not reproduced by this pure visitor.
pub fn w13a08_visit_callable_descriptors<T>(
    items: &[T],
    mut describe: impl FnMut(&T) -> W13a08CallableDescriptor,
    mut visit: impl FnMut(&T),
) -> usize {
    let mut dispatched = 0;
    for item in items {
        if describe(item).acceptable() {
            visit(item);
            dispatched += 1;
        }
    }
    dispatched
}

/// `0x0066b720`: the clearing twin of
/// [`w13a08_visit_callable_descriptors`].
///
/// The row invokes the container teardown helper once before the walk and
/// once after it, in addition to the same descriptor checks and dispatch.
/// `reset` stands in for that teardown helper.
pub fn w13a08_visit_callable_descriptors_clearing<T>(
    items: &[T],
    mut reset: impl FnMut(),
    describe: impl FnMut(&T) -> W13a08CallableDescriptor,
    visit: impl FnMut(&T),
) -> usize {
    reset();
    let dispatched = w13a08_visit_callable_descriptors(items, describe, visit);
    reset();
    dispatched
}

// ---------------------------------------------------------------------------
// Ranked settings records
// ---------------------------------------------------------------------------

/// One `0x98`-byte ranked settings record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a08RankedRecord {
    /// `+0x00`: first payload word.
    pub payload0: u32,
    /// `+0x04`: second payload word.
    pub payload1: u32,
    /// `+0x48`: unsigned low rank.
    pub low: u32,
    /// `+0x4c`: signed high rank.
    pub high: i32,
}

/// `0x0066acfa`: select the greatest record of a `0x98`-stride array.
///
/// The row scans the array with the running best supplied in its frame
/// slots.  A record replaces the best when its signed high rank is greater,
/// or when the high ranks are equal and its unsigned low rank is greater.
/// Ties keep the earlier record, and an empty array leaves the seed
/// untouched.  `None` models a caller that has no running best yet; the
/// first record then becomes the best.
pub fn w13a08_select_ranked_record(
    seed: Option<W13a08RankedRecord>,
    records: &[W13a08RankedRecord],
) -> Option<W13a08RankedRecord> {
    let mut best = seed;
    for record in records {
        let better = match best {
            None => true,
            Some(current) => {
                record.high > current.high
                    || (record.high == current.high && record.low > current.low)
            }
        };
        if better {
            best = Some(*record);
        }
    }
    best
}

// ---------------------------------------------------------------------------
// Settings host and singleton
// ---------------------------------------------------------------------------

/// `0x0066a0e0`: mark the owner's load state as failed and notify `false`.
///
/// The row loads the owner pointer from host `+0x04`, writes `3` to the
/// owner's `+0x48` state word, and then routes `false` through the bool
/// callback path.
pub fn w13a08_enter_state3(owner_state: &mut i32, mut notify: impl FnMut(bool)) {
    *owner_state = W13A08_LOAD_STATE_FAILED;
    notify(false);
}

/// Recovered once-state used by the lazy settings singleton.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a08OnceState {
    /// Per-thread generation compared with `guard`.
    pub thread_generation: i32,
    /// Once guard; the claiming thread sees `-1`.
    pub guard: i32,
    /// Set once the payload has been constructed.
    pub initialized: bool,
    /// Cached word returned to every caller.
    pub value: i32,
}

/// `0x0066aa00`: return the cached settings singleton word.
///
/// When the thread-local generation is behind the once guard, the row enters
/// the CRT once protocol.  The claiming thread constructs a `0xb8`-byte
/// object with the constant arguments `(0, 0x32)`, resets the cached word to
/// zero and completes the guard.  Every call returns the cached word.
///
/// The thread-local bookkeeping is modelled by `thread_generation`/`guard`;
/// the payload itself is opaque and is not represented.
pub fn w13a08_lazy_singleton(state: &mut W13a08OnceState) -> i32 {
    if state.thread_generation < state.guard && !state.initialized {
        state.initialized = true;
        state.value = 0;
    }
    state.value
}

/// `0x0066ae30`: address of the sub-object embedded at `+0x08`.
///
/// The four-byte row returns `this + 8` unchanged; it is reproduced as the
/// pointer adjustment it performs.
pub fn w13a08_embedded_object(base: usize) -> usize {
    base.wrapping_add(W13A08_EMBEDDED_OBJECT_OFFSET)
}

#[cfg(test)]
mod tests_wave13_agent08 {
    use super::*;

    #[test]
    fn copy_constructors_reproduce_the_capture_blocks() {
        assert_eq!(
            w13a08_copy_string_ref_lambda(0x1122_3344),
            W13a08CallableObject {
                kind: W13a08CallableKind::StringRef,
                capture: W13a08Capture::Word(0x1122_3344),
            }
        );
        assert_eq!(
            w13a08_copy_four_word_lambda([1, 2, 3, 4]),
            W13a08CallableObject {
                kind: W13a08CallableKind::FourWord,
                capture: W13a08Capture::FourWords([1, 2, 3, 4]),
            }
        );
        assert_eq!(
            w13a08_copy_word_lambda(7),
            W13a08CallableObject {
                kind: W13a08CallableKind::Word,
                capture: W13a08Capture::Word(7),
            }
        );
        assert_eq!(
            w13a08_copy_empty_lambda(),
            W13a08CallableObject {
                kind: W13a08CallableKind::FunctionVoid,
                capture: W13a08Capture::None,
            }
        );
        assert_eq!(
            w13a08_copy_word_lambda_b(9),
            W13a08CallableObject {
                kind: W13a08CallableKind::WordB,
                capture: W13a08Capture::Word(9),
            }
        );
        assert_eq!(
            w13a08_copy_bool_lambda(0),
            W13a08CallableObject {
                kind: W13a08CallableKind::BoolPredicate,
                capture: W13a08Capture::Byte(0),
            }
        );
        assert_eq!(
            w13a08_copy_bool_lambda(0xff),
            W13a08CallableObject {
                kind: W13a08CallableKind::BoolPredicate,
                capture: W13a08Capture::Byte(0xff),
            }
        );
    }

    #[test]
    fn rtti_getters_are_distinct_and_match_the_copy_kinds() {
        let kinds = [
            w13a08_rtti_string_ref().kind,
            w13a08_rtti_four_word().kind,
            w13a08_rtti_word().kind,
            w13a08_rtti_function_void().kind,
            w13a08_rtti_word_b().kind,
            w13a08_rtti_bool().kind,
        ];
        for (index, kind) in kinds.iter().enumerate() {
            for other in &kinds[index + 1..] {
                assert_ne!(kind, other);
            }
        }
        assert_eq!(kinds[0], w13a08_copy_string_ref_lambda(0).kind);
        assert_eq!(kinds[1], w13a08_copy_four_word_lambda([0; 4]).kind);
        assert_eq!(kinds[2], w13a08_copy_word_lambda(0).kind);
        assert_eq!(kinds[3], w13a08_copy_empty_lambda().kind);
        assert_eq!(kinds[4], w13a08_copy_word_lambda_b(0).kind);
        assert_eq!(kinds[5], w13a08_copy_bool_lambda(0).kind);
        assert_eq!(kinds[5], w13a08_bool_callback(false).kind);
    }

    #[test]
    fn failed_load_writes_state_three_and_notifies_false() {
        let mut state = 7;
        let mut seen = Vec::new();
        w13a08_enter_state3(&mut state, |value| seen.push(value));
        assert_eq!(state, 3);
        assert_eq!(seen, vec![false]);
    }

    #[test]
    fn empty_owned_string_uses_the_inline_buffer_layout() {
        let string = w13a08_empty_owned_string();
        assert_eq!(string.len, 0);
        assert_eq!(string.capacity, 0x8000_000f);
        assert_eq!(string.data, W13a08OwnedStringData::Inline { offset: 0x0c });
        assert_eq!(string.inline, [0u8; 16]);

        let heap = W13a08OwnedString {
            data: W13a08OwnedStringData::Heap { address: 0x1234 },
            len: 5,
            capacity: W13A08_SMALL_STRING_HEAP_FLAGS,
            inline: [0; 16],
        };
        assert_eq!(heap.data, W13a08OwnedStringData::Heap { address: 0x1234 });
    }

    #[test]
    fn recovered_layout_constants_are_pinned() {
        assert_eq!(W13A08_RANKED_RECORD_STRIDE, 0x98);
        assert_eq!(W13A08_RANKED_RECORD_LOW_OFFSET, 0x48);
        assert_eq!(W13A08_RANKED_RECORD_HIGH_OFFSET, 0x4c);
        assert_eq!(W13A08_SINGLETON_BYTES, 0xb8);
        assert_eq!(W13A08_SINGLETON_CTOR_ARG0, 0);
        assert_eq!(W13A08_SINGLETON_CTOR_ARG1, 0x32);
        assert_eq!(W13A08_FILTER_FIRST_LIMIT, 0x14);
        assert_eq!(W13A08_FILTER_SECOND_LIMIT, 4);
        assert_eq!(W13A08_LOAD_STATE_FAILED, 3);
        assert_eq!(W13A08_EMBEDDED_OBJECT_OFFSET, 8);
    }

    #[test]
    fn settings_path_builder_attaches_an_empty_name() {
        let path = w13a08_new_settings_path();
        assert_eq!(path.path_argument, 1);
        assert_eq!(path.name, w13a08_empty_owned_string());
    }

    #[test]
    fn string_view_header_carries_the_recovered_flags() {
        let view = w13a08_string_view(b"CSettings.json");
        assert_eq!(view.len, 14);
        assert_eq!(view.reserved, 0);
        assert_eq!(view.flags, 0x0405_0000);
        assert_eq!(view.data, b"CSettings.json");

        let empty = w13a08_string_view(b"");
        assert_eq!(empty.len, 0);
        assert_eq!(empty.flags, 0x0405_0000);
        assert!(empty.data.is_empty());
    }

    #[test]
    fn small_string_copy_plan_switches_at_fourteen_bytes() {
        assert_eq!(
            w13a08_small_string_copy(0),
            W13a08SmallStringCopy::Inline {
                spare: 13,
                capacity_word: 0x1c05_0d00,
            }
        );
        assert_eq!(
            w13a08_small_string_copy(12),
            W13a08SmallStringCopy::Inline {
                spare: 1,
                capacity_word: 0x1c05_0100,
            }
        );
        assert_eq!(
            w13a08_small_string_copy(13),
            W13a08SmallStringCopy::Inline {
                spare: 0,
                capacity_word: 0x1c05_0000,
            }
        );
        assert_eq!(
            w13a08_small_string_copy(14),
            W13a08SmallStringCopy::Heap {
                bytes: 15,
                capacity_word: 0x0c05_0000,
            }
        );
    }

    #[test]
    fn hash_matches_the_end_of_the_stride_table() {
        assert!(w13a08_hash_equals_table_end(0x1060, 0x1000, 3));
        assert!(!w13a08_hash_equals_table_end(0x105f, 0x1000, 3));
        assert!(w13a08_hash_equals_table_end(0x1000, 0x1000, 0));
        assert!(w13a08_hash_equals_table_end(0, 0, 0));
    }

    #[test]
    fn bool_callback_captures_the_argument_byte() {
        let callback = w13a08_bool_callback(true);
        assert_eq!(callback.kind, W13a08CallableKind::BoolPredicate);
        assert_eq!(callback.capture, W13a08Capture::Byte(1));
        assert_eq!(w13a08_bool_callback(false).capture, W13a08Capture::Byte(0));
    }

    #[test]
    fn ranked_record_selection_uses_signed_high_then_unsigned_low() {
        let seed = W13a08RankedRecord {
            payload0: 0xaa,
            payload1: 0xbb,
            low: 10,
            high: -1,
        };
        let records = [
            W13a08RankedRecord {
                payload0: 1,
                payload1: 2,
                low: 20,
                high: -1,
            },
            W13a08RankedRecord {
                payload0: 3,
                payload1: 4,
                low: 5,
                high: 0,
            },
            W13a08RankedRecord {
                payload0: 5,
                payload1: 6,
                low: 100,
                high: 0,
            },
            W13a08RankedRecord {
                payload0: 7,
                payload1: 8,
                low: 200,
                high: -2,
            },
            W13a08RankedRecord {
                payload0: 9,
                payload1: 10,
                low: 50,
                high: 0,
            },
        ];
        assert_eq!(
            w13a08_select_ranked_record(Some(seed), &records),
            Some(records[2])
        );

        let first = W13a08RankedRecord {
            payload0: 11,
            payload1: 12,
            low: 7,
            high: 7,
        };
        let equal = W13a08RankedRecord {
            payload0: 13,
            payload1: 14,
            low: 7,
            high: 7,
        };
        assert_eq!(
            w13a08_select_ranked_record(Some(first), &[equal]),
            Some(first)
        );
        assert_eq!(w13a08_select_ranked_record(None, &[equal]), Some(equal));
        assert_eq!(w13a08_select_ranked_record(None, &[]), None);
        assert_eq!(w13a08_select_ranked_record(Some(first), &[]), Some(first));
    }

    #[test]
    fn lazy_singleton_constructs_once_and_returns_the_cached_word() {
        let mut waiting = W13a08OnceState {
            thread_generation: 0,
            guard: 0,
            initialized: false,
            value: 42,
        };
        assert_eq!(w13a08_lazy_singleton(&mut waiting), 42);
        assert!(!waiting.initialized);

        let mut claiming = W13a08OnceState {
            thread_generation: -1,
            guard: 0,
            initialized: false,
            value: 42,
        };
        assert_eq!(w13a08_lazy_singleton(&mut claiming), 0);
        assert!(claiming.initialized);

        claiming.value = 9;
        assert_eq!(w13a08_lazy_singleton(&mut claiming), 9);
    }

    #[test]
    fn embedded_object_sits_eight_bytes_into_the_owner() {
        assert_eq!(w13a08_embedded_object(0x1000), 0x1008);
        assert_eq!(
            w13a08_embedded_object(usize::MAX),
            W13A08_EMBEDDED_OBJECT_OFFSET - 1
        );
    }

    #[test]
    fn filtered_visits_apply_the_descriptor_bounds() {
        let nodes = [
            (
                1u32,
                W13a08CallableDescriptor {
                    present: true,
                    first: 0x14,
                    second: 4,
                },
            ),
            (
                2,
                W13a08CallableDescriptor {
                    present: false,
                    first: 0,
                    second: 0,
                },
            ),
            (
                3,
                W13a08CallableDescriptor {
                    present: true,
                    first: 0x15,
                    second: 0,
                },
            ),
            (
                4,
                W13a08CallableDescriptor {
                    present: true,
                    first: 0,
                    second: 5,
                },
            ),
            (
                5,
                W13a08CallableDescriptor {
                    present: true,
                    first: 0,
                    second: 0,
                },
            ),
        ];

        let mut seen = Vec::new();
        let dispatched =
            w13a08_visit_callable_descriptors(&nodes, |node| node.1, |node| seen.push(node.0));
        assert_eq!(dispatched, 2);
        assert_eq!(seen, vec![1, 5]);

        let mut resets = 0;
        let mut cleared_seen = Vec::new();
        let cleared_dispatched = w13a08_visit_callable_descriptors_clearing(
            &nodes,
            || resets += 1,
            |node| node.1,
            |node| cleared_seen.push(node.0),
        );
        assert_eq!(cleared_dispatched, 2);
        assert_eq!(cleared_seen, vec![1, 5]);
        assert_eq!(resets, 2);
    }
}
