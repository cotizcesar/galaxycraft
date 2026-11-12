#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave14/agent01.rs
// ===========================================================================
// Wave 14 translation batch 01 (staging).
//
// Window: `00670000..=00671fff`.  The window holds the support layer of
// the MSVC `std::string`-shaped `bc::basic_string` (byte-identity tables,
// copy assignment, small-buffer construction, C-string forwarding) plus a
// small client object with a `+0x110` child and an ordered 16-byte-value
// key/value table.
//
// ## Recovered layout
//
// | offset | shape | meaning |
// |---|---|---|
// | `+0x00` | `u32` | keyed-table count / string data pointer |
// | `+0x04` | `u32` | keyed-table capacity / string size |
// | `+0x08` | `u32` | keyed-table data / string capacity word |
// | `+0x0c` | 16 bytes | inline string buffer |
// | `+0x1c` | `u32` | pointer-table slot count |
// | `+0x20` | `u32` | pointer-table base |
// | `+0x110` | `u32` | child object used by every trampoline |
// | `+0xe8` | `u32` | mode word tested by `w14a01_is_mode_one` |
// | `+0xec` | `u8` | state byte returned by `w14a01_state_byte` |
// | `+0x100` | `u8` | flag cleared by `w14a01_reset_child` |
//
// A string object is the 28-byte MSVC `std::string`: data pointer at
// `+0x00`, size at `+0x04`, capacity/flags at `+0x08` and a 16-byte inline
// buffer at `+0x0c`.  The small-buffer capacity tag is `0x8000_000f`
// ("capacity 15, heap flag clear"); the inline buffer starts zeroed.
//
// The keyed table holds `0x20`-byte elements: a 16-byte `bc::value` key at
// `+0x00` and a 16-byte `bc::value` mapped value at `+0x10`.  Both carry a
// `u16` tag at `+0x0e`; tag bit `0x1000` marks an inline string.  Key
// equality is the value's string compare (`u32` length at `+0x00` with the
// bytes reached through `+0x08` for the heap form, or the inline bytes with
// length `0xd - *(i8 *)(key + 0xd)`), so the translation stores the
// resolved key bytes and compares them byte-wise.  The original clears both
// source tags after moving the records into the table.
//
// Skipped rows (detailed in the batch report): `FUN_00670047` (mid-function
// fragment of a constructor that begins before the window),
// `FUN_006706c0` (tagged variant assignment through an allocator singleton),
// the six `bc::copyable_function` copy constructors
// `FUN_00670e10`/`FUN_00670e30`/`FUN_00670e50`/`FUN_00670e70`/
// `FUN_00670e80`/`FUN_00670ea0`, the SEH funclets
// `FUN_0067102c`/`FUN_00671050`/`FUN_0067107d`/`FUN_006710eb`, the static
// RTTI descriptor getters `FUN_00671340`/`FUN_00671360`/`FUN_00671370`/
// `FUN_00671380`/`FUN_00671390`, `FUN_00670b90` and `FUN_00671b31` (control
// flow on bad instruction data), `FUN_00671562` (`halt_baddata`) and the two
// failed decompilations `FUN_00670ef0`/`FUN_00671441`.
//
// Every routine is a detached reference helper: nothing is wired into
// `World`, `SimUnit`, the renderer or the client, and no save/replay state
// is introduced.

// ---------------------------------------------------------------------------
// Recovered constants
// ---------------------------------------------------------------------------

/// Small-buffer capacity tag of the recovered string object: capacity `15`
/// with the heap flag clear.
pub const W14A01_SSO_CAPACITY_TAG: u32 = 0x8000_000f;

/// Offset of the pointer-table slot count in the swept object.
pub const W14A01_SLOT_COUNT_OFFSET: u32 = 0x1c;

/// Offset of the pointer-table base in the swept object.
pub const W14A01_SLOT_TABLE_OFFSET: u32 = 0x20;

/// Offset of the child object used by every trampoline in this batch.
pub const W14A01_CHILD_OFFSET: u32 = 0x110;

/// Size of one keyed-table element: two 16-byte `bc::value` records.
pub const W14A01_KEYED_ELEMENT_BYTES: usize = 0x20;

/// First capacity of the keyed table (`0x10` elements, `0x200` bytes).
pub const W14A01_KEYED_FIRST_CAPACITY: u32 = 0x10;

/// `bc::value` tag bit that marks an inline string payload.
pub const W14A01_VALUE_INLINE_STRING: u16 = 0x1000;

/// Inline string length: `13 - *(i8 *)(value + 0x0d)`.
pub const W14A01_VALUE_INLINE_LEN_BASE: i32 = 0xd;

/// Second argument passed to the `+0x110` child by
/// `w14a01_apply_text_buffer`.
pub const W14A01_TEXT_NOTIFY_FLAGS: u32 = 0x4400_00ff;

/// Recovered object size used by the scalar deleting destructor.
pub const W14A01_SCALAR_BYTES: usize = 0x118;

// ---------------------------------------------------------------------------
// 0x006706a0 / 0x006706b0 - byte-identity table initialisers
// ---------------------------------------------------------------------------

/// `0x006706a0`: fills the 32-byte (`0x20`) table at the object base with the
/// byte sequence `0..=0x1f` (`dst[i] = i as u8`).
pub fn w14a01_fill_identity_32(dst: &mut [u8; 0x20]) {
    for (index, slot) in dst.iter_mut().enumerate() {
        *slot = index as u8;
    }
}

/// `0x006706b0`: fills the 56-byte (`0x38`) table at the object base with the
/// byte sequence `0..=0x37`.
pub fn w14a01_fill_identity_56(dst: &mut [u8; 0x38]) {
    for (index, slot) in dst.iter_mut().enumerate() {
        *slot = index as u8;
    }
}

// ---------------------------------------------------------------------------
// 0x00670b30 - constant predicate
// ---------------------------------------------------------------------------

/// `0x00670b30`: returns `1` (`true`).  The routine is `stdcall` with one
/// stack argument (`ret 4`) which it ignores; the argument is kept so the
/// recovered calling convention stays visible.
pub fn w14a01_always_true(_argument: u32) -> bool {
    true
}

// ---------------------------------------------------------------------------
// 0x006719d0 / 0x006719f0 / 0x00671e80 - client-object accessors
// ---------------------------------------------------------------------------

/// Recovered fields of the client object touched by the accessors and the
/// reset trampoline in this batch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a01UiState {
    /// `+0xe8`: mode word; the object is in mode one when this equals `1`.
    pub mode_e8: i32,
    /// `+0xec`: state byte returned verbatim.
    pub flag_ec: u8,
    /// `+0x100`: byte cleared by [`w14a01_reset_child`].
    pub flag_100: u8,
}

/// `0x006719d0`: returns the state byte at `+0xec`.
pub fn w14a01_state_byte(state: &W14a01UiState) -> u8 {
    state.flag_ec
}

/// `0x006719f0`: returns `true` exactly when the mode word at `+0xe8` is `1`
/// (`cmp dword [ecx+0xe8],1; sete al`).
pub fn w14a01_is_mode_one(state: &W14a01UiState) -> bool {
    state.mode_e8 == 1
}

/// `0x00671e80`: calls the `+0x110` child's reset entry point, clears the
/// object's `+0x100` byte and returns `true` (`mov al,1`).
pub fn w14a01_reset_child<F>(state: &mut W14a01UiState, reset: F) -> bool
where
    F: FnOnce(),
{
    reset();
    state.flag_100 = 0;
    true
}

// ---------------------------------------------------------------------------
// 0x00670750 - scalar deleting destructor
// ---------------------------------------------------------------------------

/// `0x00670750`: scalar deleting destructor.  Runs the object destructor and,
/// when bit 0 of `flag` is set, releases the object with the recovered size
/// `0x118`.  The original returns `this`, which is reproduced here.
pub fn w14a01_scalar_delete<D, F>(object: usize, flag: u32, destroy: D, release: F) -> usize
where
    D: FnOnce(),
    F: FnOnce(usize, usize),
{
    destroy();
    if flag & 1 != 0 {
        release(object, W14A01_SCALAR_BYTES);
    }
    object
}

// ---------------------------------------------------------------------------
// 0x00670130 - pointer-table release sweep
// ---------------------------------------------------------------------------

/// One node of the chains held by the `+0x20` pointer table.
///
/// The recovered node has a `next` link at `+0x00`, an intrusive handle pair
/// at `+0x04`/`+0x0c` and a second pair at `+0x20`/`+0x28`.  A pair's second
/// word is its ownership tag: a non-negative value means the first word is
/// owned and must be released, a negative value means the pair is empty.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a01SweepNode {
    /// `+0x00`: next node index; `0` terminates the chain.
    pub next: u32,
    /// `+0x04`: first handle.
    pub first_handle: u32,
    /// `+0x0c`: first ownership tag.
    pub first_owner: i32,
    /// `+0x20`: second handle.
    pub second_handle: u32,
    /// `+0x28`: second ownership tag.
    pub second_owner: i32,
}

/// `0x00670130`: release every chain of the pointer table.
///
/// The recovered order is: for each table slot (ascending), follow the chain,
/// release the `+0x20` handle, then the `+0x04` handle, then the node itself.
/// The original then releases the table block and finally the trailing pair at
/// `+0x00`/`+0x08` when its ownership tag is non-negative.
///
/// `table` is the slot array (the original reads the count from `+0x1c` and
/// the base from `+0x20`).  Node indices are 1-based; `0` terminates a chain,
/// mirroring the null link of the original.  The table block is owned by the
/// caller in this safe model, so only the node and trailing handles are passed
/// to `release`.  Returns the number of handles released (the table block is
/// not counted).
pub fn w14a01_release_pointer_table(
    table: &[u32],
    nodes: &[W14a01SweepNode],
    tail: (u32, i32),
    mut release: impl FnMut(u32),
) -> usize {
    let mut released = 0usize;
    for &head in table {
        let mut cursor = head;
        while cursor != 0 {
            let Some(node) = nodes.get(cursor as usize).copied() else {
                break;
            };
            if node.second_owner >= 0 {
                release(node.second_handle);
                released += 1;
            }
            if node.first_owner >= 0 {
                release(node.first_handle);
                released += 1;
            }
            release(cursor);
            released += 1;
            cursor = node.next;
        }
    }
    if tail.1 >= 0 {
        release(tail.0);
        released += 1;
    }
    released
}

// ---------------------------------------------------------------------------
// 0x00670780 - ordered key/value table insert-or-assign
// ---------------------------------------------------------------------------

/// A 16-byte `bc::value` record: an eight-byte payload, four pad/tag bytes and
/// a `u16` tag at `+0x0e`.  The table stores keys and mapped values in this
/// form.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a01Value16(pub [u8; 0x10]);

/// One keyed-table element with its key resolved to bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W14a01KeyedEntry {
    /// The key's string bytes (the original stores the 16-byte `bc::value`).
    pub key: Vec<u8>,
    /// The 16-byte mapped value at element offset `+0x10`.
    pub value: W14a01Value16,
}

/// The ordered key/value table: count at `+0x00`, capacity at `+0x04`, data at
/// `+0x08`; elements are `0x20` bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W14a01KeyedTable {
    /// Current capacity in elements (`+0x04`); `0` before the first insert.
    pub capacity: u32,
    /// Insertion-ordered entries; the length is the recovered count.
    pub entries: Vec<W14a01KeyedEntry>,
}

/// `0x00670780`: insert a key/value pair, or replace the mapped value when the
/// key is already present.  Returns the table, as the original returns
/// `param_1`.
///
/// The recovered search compares the 16-byte key records by their string
/// payload; a match overwrites the stored value and leaves the order
/// untouched.  Otherwise the pair is appended.  When the table is full the
/// capacity grows from `0` to `0x10`, or from `n` to `n + (n + 1) / 2`, and
/// the element storage is reallocated (the original calls its allocator and
/// copies `count * 0x20` bytes).
pub fn w14a01_object_insert<'a>(
    table: &'a mut W14a01KeyedTable,
    key: &[u8],
    value: W14a01Value16,
) -> &'a mut W14a01KeyedTable {
    if let Some(entry) = table
        .entries
        .iter_mut()
        .find(|entry| entry.key.as_slice() == key)
    {
        entry.value = value;
        return table;
    }
    if table.entries.len() as u32 >= table.capacity {
        table.capacity = if table.capacity == 0 {
            W14A01_KEYED_FIRST_CAPACITY
        } else {
            table.capacity + table.capacity.div_ceil(2)
        };
    }
    table.entries.push(W14a01KeyedEntry {
        key: key.to_vec(),
        value,
    });
    table
}

// ---------------------------------------------------------------------------
// 0x00670990 / 0x00670b40 - tagged dispatch
// ---------------------------------------------------------------------------

/// `0x00670990`: dispatch on the tagged word at `[this+4] + 0x24`.
///
/// Bit 0 of the stored word marks a heap reference and is cleared before use;
/// a clear bit means the object is embedded at the slot address, represented
/// here by the stored word itself.  A zero target does nothing.  The original
/// tail-jumps through vtable slot 1 of the resolved object and returns
/// nothing; the call is supplied by the caller.  Returns whether the dispatch
/// happened.
pub fn w14a01_dispatch_tagged_at_24(stored: u32, invoke: impl FnOnce(u32)) -> bool {
    let target = if stored & 1 != 0 { stored & !1 } else { stored };
    if target == 0 {
        return false;
    }
    invoke(target);
    true
}

/// `0x00670b40`: the same tagged dispatch over the slot at `[this+4] + 0x10`.
pub fn w14a01_dispatch_tagged_at_10(stored: u32, invoke: impl FnOnce(u32)) -> bool {
    let target = if stored & 1 != 0 { stored & !1 } else { stored };
    if target == 0 {
        return false;
    }
    invoke(target);
    true
}

// ---------------------------------------------------------------------------
// 0x00670ec0 / 0x00670ee0 / 0x006712d0 / 0x006719e0 / 0x00671e60 -
// +0x110 trampolines
// ---------------------------------------------------------------------------

/// `0x00670ec0`: forwards `(a, b, c)` to the `+0x110` child's three-argument
/// message entry point (the callee is outside this window and is supplied by
/// the caller).  The original loads the child into `ecx` and calls it.
pub fn w14a01_forward_child3<F>(child: F, a: u32, b: u32, c: u32)
where
    F: FnOnce(u32, u32, u32),
{
    child(a, b, c);
}

/// `0x00670ee0`: tail-forwards no arguments to the `+0x110` child's
/// zero-argument entry point.
pub fn w14a01_forward_child0<F>(child: F)
where
    F: FnOnce(),
{
    child();
}

/// `0x006712d0`: tail-forwards no arguments to the `+0x110` child's second
/// zero-argument entry point.
pub fn w14a01_forward_child0_b<F>(child: F)
where
    F: FnOnce(),
{
    child();
}

/// `0x006719e0`: tail-forwards no arguments to the `+0x110` child's third
/// zero-argument entry point.
pub fn w14a01_forward_child0_c<F>(child: F)
where
    F: FnOnce(),
{
    child();
}

/// `0x00671e60`: forwards `(a, b, c)` to the `+0x110` child's second
/// three-argument message entry point.
pub fn w14a01_forward_child3_b<F>(child: F, a: u32, b: u32, c: u32)
where
    F: FnOnce(u32, u32, u32),
{
    child(a, b, c);
}

// ---------------------------------------------------------------------------
// 0x006712e0 - string copy assignment
// ---------------------------------------------------------------------------

/// `0x006712e0`: copy-assigns the string held at `+0xb8` (`+0xbc` bytes) of
/// the source object into `dst`.
///
/// The recovered destination prologue resets the header to the small-buffer
/// form: data pointer to the inline buffer at `+0x0c`, size `0`, capacity
/// `0x8000_000f`, first buffer byte `0`.  The assign helper then reserves
/// `len` bytes, copies them, appends the NUL terminator and stores `len` at
/// `+0x04`.  The model keeps the resulting bytes and returns the new size.
pub fn w14a01_string_copy_assign(dst: &mut Vec<u8>, src: &[u8]) -> u32 {
    dst.clear();
    dst.extend_from_slice(src);
    dst.len() as u32
}

// ---------------------------------------------------------------------------
// 0x00671d40 - capability word
// ---------------------------------------------------------------------------

/// `0x00671d40`: assembles the capability word from nine queries.
///
/// `probe` is the recovered `FUN_006bbcd0` byte probe (three ids per group);
/// `feature` is the recovered `FUN_00944e70` mode query (`== 1` sets a bit).
/// Bit layout:
///
/// | bit | source |
/// |---|---|
/// | `0x002` | probe `0x10`, `0xa0`, `0xa1` (any non-zero) |
/// | `0x004` | probe `0x11`, `0xa2`, `0xa3` (any non-zero) |
/// | `0x008` | probe `0x12`, `0xa4`, `0xa5` (any non-zero) |
/// | `0x200` | feature `1` |
/// | `0x400` | feature `2` |
/// | `0x800` | feature `4` |
pub fn w14a01_capability_bits(
    mut probe: impl FnMut(i32) -> u8,
    mut feature: impl FnMut(i32) -> u32,
) -> u16 {
    let group_a = probe(0x10) != 0 || probe(0xa0) != 0 || probe(0xa1) != 0;
    let group_b = probe(0x11) != 0 || probe(0xa2) != 0 || probe(0xa3) != 0;
    let group_c = probe(0x12) != 0 || probe(0xa4) != 0 || probe(0xa5) != 0;
    let feature_1 = feature(1) == 1;
    let feature_4 = feature(4) == 1;

    let mut bits = 0u16;
    if group_a {
        bits |= 0x002;
    }
    if group_b {
        bits |= 0x004;
    }
    if group_c {
        bits |= 0x008;
    }
    if feature_1 {
        bits |= 0x200;
    }
    if feature_4 {
        bits |= 0x800;
    }
    if feature(2) == 1 {
        bits |= 0x400;
    }
    bits
}

// ---------------------------------------------------------------------------
// 0x00670a40 / 0x00671fd0 - C-string forwarding
// ---------------------------------------------------------------------------

/// `0x00670a40`: converts the NUL-terminated byte string reached through
/// `c_string` and forwards `(1, key, string)` to the handler at `[this+4]`.
///
/// The recovered body computes the C-string length (zero for an empty first
/// byte), builds the string, and calls the handler through the member at
/// `[this+4]`.  The handler is supplied by the caller.
pub fn w14a01_forward_c_string<F>(key: u32, c_string: &[u8], forward: F)
where
    F: FnOnce(u32, u32, &[u8]),
{
    let len = c_string
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(c_string.len());
    forward(1, key, &c_string[..len]);
}

/// `0x00671fd0`: copies the recovered 256-byte text buffer into a string up to
/// the first NUL, forwards `(text, 0x4400_00ff)` to the `+0x110` child and
/// returns `true`.
///
/// The original fills a `0x100`-byte stack buffer through `FUN_00940aa0`,
/// computes the C-string length, builds the string and calls the child's
/// two-argument entry point.  A buffer without a NUL is clamped to its full
/// length here instead of reading past it.
pub fn w14a01_apply_text_buffer<F>(buffer: &[u8], notify: F) -> bool
where
    F: FnOnce(&[u8], u32),
{
    let len = buffer
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(buffer.len());
    notify(&buffer[..len], W14A01_TEXT_NOTIFY_FLAGS);
    true
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_wave14_agent01 {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[test]
    fn identity_tables_fill_with_their_index() {
        let mut small = [0xaa_u8; 0x20];
        w14a01_fill_identity_32(&mut small);
        for (index, byte) in small.iter().enumerate() {
            assert_eq!(*byte, index as u8);
        }
        assert_eq!(small[0], 0);
        assert_eq!(small[0x1f], 0x1f);

        let mut large = [0xff_u8; 0x38];
        w14a01_fill_identity_56(&mut large);
        for (index, byte) in large.iter().enumerate() {
            assert_eq!(*byte, index as u8);
        }
        assert_eq!(large[0x20], 0x20);
        assert_eq!(large[0x37], 0x37);
    }

    #[test]
    fn constant_predicate_and_state_accessors() {
        assert!(w14a01_always_true(0));
        assert!(w14a01_always_true(0xffff_ffff));

        let state = W14a01UiState {
            mode_e8: 1,
            flag_ec: 0x5a,
            flag_100: 1,
        };
        assert_eq!(w14a01_state_byte(&state), 0x5a);
        assert!(w14a01_is_mode_one(&state));
        assert!(!w14a01_is_mode_one(&W14a01UiState {
            mode_e8: 0,
            ..state
        }));
        assert!(!w14a01_is_mode_one(&W14a01UiState {
            mode_e8: 2,
            ..state
        }));
    }

    #[test]
    fn reset_child_calls_first_then_clears_the_flag() {
        let mut state = W14a01UiState {
            mode_e8: 1,
            flag_ec: 0,
            flag_100: 1,
        };
        let flag_before = state.flag_100;
        let mut saw_flag_before_reset = false;
        let result = w14a01_reset_child(&mut state, || {
            saw_flag_before_reset = flag_before == 1;
        });
        assert!(result);
        assert!(saw_flag_before_reset);
        assert_eq!(state.flag_100, 0);
        assert_eq!(state.mode_e8, 1);
    }

    #[test]
    fn scalar_delete_honours_bit_zero_and_reports_the_size() {
        let destroyed = Cell::new(0u32);
        let released = Cell::new((0usize, 0usize));

        let returned = w14a01_scalar_delete(
            0x1234,
            2,
            || destroyed.set(destroyed.get() + 1),
            |object, size| released.set((object, size)),
        );
        assert_eq!(returned, 0x1234);
        assert_eq!(destroyed.get(), 1);
        assert_eq!(released.get(), (0, 0));

        let returned = w14a01_scalar_delete(
            0x5678,
            3,
            || destroyed.set(destroyed.get() + 1),
            |object, size| released.set((object, size)),
        );
        assert_eq!(returned, 0x5678);
        assert_eq!(destroyed.get(), 2);
        assert_eq!(released.get(), (0x5678, 0x118));
    }

    #[test]
    fn pointer_table_sweep_releases_payloads_nodes_and_tail() {
        // Node 0 is the chain terminator, so real nodes start at index 1.
        let nodes = [
            W14a01SweepNode::default(),
            W14a01SweepNode {
                next: 2,
                first_handle: 0x11,
                first_owner: 0,
                second_handle: 0x21,
                second_owner: 0,
            },
            W14a01SweepNode {
                next: 0,
                first_handle: 0x12,
                first_owner: -1,
                second_handle: 0x22,
                second_owner: -1,
            },
            W14a01SweepNode {
                next: 0,
                first_handle: 0x13,
                first_owner: 0,
                second_handle: 0x23,
                second_owner: 1,
            },
        ];
        let table = [1u32, 0, 3];
        let mut calls = Vec::new();
        let released =
            w14a01_release_pointer_table(&table, &nodes, (0x99, 0), |handle| calls.push(handle));

        assert_eq!(calls, vec![0x21, 0x11, 1, 2, 0x23, 0x13, 3, 0x99]);
        assert_eq!(released, 8);

        let mut calls = Vec::new();
        let released =
            w14a01_release_pointer_table(&table, &nodes, (0x99, -1), |handle| calls.push(handle));
        assert_eq!(calls, vec![0x21, 0x11, 1, 2, 0x23, 0x13, 3]);
        assert_eq!(released, 7);

        let mut calls = Vec::new();
        assert_eq!(
            w14a01_release_pointer_table(&[], &nodes, (0, -1), |handle| calls.push(handle)),
            0
        );
        assert!(calls.is_empty());
    }

    #[test]
    fn keyed_table_inserts_appends_and_replaces() {
        let value = |tag: u8| W14a01Value16([tag; 0x10]);
        let mut table = W14a01KeyedTable::default();

        w14a01_object_insert(&mut table, b"alpha", value(1));
        assert_eq!(table.entries.len(), 1);
        assert_eq!(table.capacity, 0x10);

        w14a01_object_insert(&mut table, b"beta", value(2));
        assert_eq!(table.entries.len(), 2);
        assert_eq!(table.entries[1].key, b"beta".to_vec());
        assert_eq!(table.entries[1].value, value(2));

        w14a01_object_insert(&mut table, b"alpha", value(9));
        assert_eq!(table.entries.len(), 2);
        assert_eq!(table.entries[0].key, b"alpha".to_vec());
        assert_eq!(table.entries[0].value, value(9));

        for index in 0..0xe_u8 {
            let key = [b'a' + index];
            w14a01_object_insert(&mut table, &key, value(index));
        }
        assert_eq!(table.entries.len(), 0x10);
        assert_eq!(table.capacity, 0x10);

        w14a01_object_insert(&mut table, b"seventeen", value(0x11));
        assert_eq!(table.entries.len(), 0x11);
        assert_eq!(table.capacity, 0x18);
        assert_eq!(table.entries[table.entries.len() - 1].value, value(0x11));
    }

    #[test]
    fn tagged_dispatch_normalises_the_heap_bit() {
        let mut seen = None;
        assert!(w14a01_dispatch_tagged_at_24(0x1001, |target| {
            seen = Some(target);
        }));
        assert_eq!(seen, Some(0x1000));

        let mut seen = None;
        assert!(w14a01_dispatch_tagged_at_10(0x2000, |target| {
            seen = Some(target);
        }));
        assert_eq!(seen, Some(0x2000));

        assert!(!w14a01_dispatch_tagged_at_24(0, |_| unreachable!()));
        assert!(!w14a01_dispatch_tagged_at_10(1, |_| unreachable!()));
    }

    #[test]
    fn child_trampolines_forward_their_arguments() {
        let seen = Cell::new((0u32, 0u32, 0u32));
        w14a01_forward_child3(|a, b, c| seen.set((a, b, c)), 0x11, 0x22, 0x33);
        assert_eq!(seen.get(), (0x11, 0x22, 0x33));

        let seen = Cell::new((0u32, 0u32, 0u32));
        w14a01_forward_child3_b(|a, b, c| seen.set((a, b, c)), 0x44, 0x55, 0x66);
        assert_eq!(seen.get(), (0x44, 0x55, 0x66));

        let calls = Cell::new(0u32);
        w14a01_forward_child0(|| calls.set(calls.get() + 1));
        w14a01_forward_child0_b(|| calls.set(calls.get() + 1));
        w14a01_forward_child0_c(|| calls.set(calls.get() + 1));
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn string_copy_assign_replaces_the_destination() {
        let mut dst = b"old".to_vec();
        assert_eq!(w14a01_string_copy_assign(&mut dst, b"new-bytes"), 9);
        assert_eq!(dst.as_slice(), b"new-bytes");

        assert_eq!(w14a01_string_copy_assign(&mut dst, b""), 0);
        assert!(dst.is_empty());

        let mut dst = Vec::new();
        assert_eq!(w14a01_string_copy_assign(&mut dst, &[0, 1, 2, 0xff]), 4);
        assert_eq!(dst, vec![0u8, 1, 2, 0xff]);
    }

    #[test]
    fn capability_bits_cover_every_source() {
        assert_eq!(w14a01_capability_bits(|_| 0, |_| 0), 0);

        assert_eq!(
            w14a01_capability_bits(|id| (id == 0x10) as u8, |_| 0),
            0x002
        );
        assert_eq!(
            w14a01_capability_bits(|id| (id == 0xa1) as u8, |_| 0),
            0x002
        );
        assert_eq!(
            w14a01_capability_bits(|id| (id == 0x11) as u8, |_| 0),
            0x004
        );
        assert_eq!(
            w14a01_capability_bits(|id| (id == 0xa3) as u8, |_| 0),
            0x004
        );
        assert_eq!(
            w14a01_capability_bits(|id| (id == 0x12) as u8, |_| 0),
            0x008
        );
        assert_eq!(
            w14a01_capability_bits(|id| (id == 0xa5) as u8, |_| 0),
            0x008
        );

        assert_eq!(w14a01_capability_bits(|_| 0, |id| (id == 1) as u32), 0x200);
        assert_eq!(w14a01_capability_bits(|_| 0, |id| (id == 2) as u32), 0x400);
        assert_eq!(w14a01_capability_bits(|_| 0, |id| (id == 4) as u32), 0x800);
        // A mode value other than 1 sets nothing.
        assert_eq!(w14a01_capability_bits(|_| 0, |_| 2), 0);

        assert_eq!(
            w14a01_capability_bits(|id| matches!(id, 0x10 | 0xa2 | 0x12) as u8, |_| 1,),
            0x002 | 0x004 | 0x008 | 0x200 | 0x400 | 0x800
        );
    }

    #[test]
    fn c_string_forwarding_computes_the_bounded_length() {
        let seen = RefCell::new(None);
        w14a01_forward_c_string(7, b"hello\0tail", |one, key, text| {
            *seen.borrow_mut() = Some((one, key, text.to_vec()));
        });
        assert_eq!(*seen.borrow(), Some((1, 7, b"hello".to_vec())));

        let seen = RefCell::new(None);
        w14a01_forward_c_string(0, b"", |one, key, text| {
            *seen.borrow_mut() = Some((one, key, text.to_vec()));
        });
        assert_eq!(*seen.borrow(), Some((1, 0, Vec::new())));

        let seen = RefCell::new(None);
        w14a01_forward_c_string(3, b"no-terminator", |one, key, text| {
            *seen.borrow_mut() = Some((one, key, text.to_vec()));
        });
        assert_eq!(*seen.borrow(), Some((1, 3, b"no-terminator".to_vec())));
    }

    #[test]
    fn text_buffer_stops_at_the_first_nul_and_forwards_the_flags() {
        let seen = RefCell::new(None);
        assert!(w14a01_apply_text_buffer(b"abc\0def", |text, flags| {
            *seen.borrow_mut() = Some((text.to_vec(), flags));
        }));
        assert_eq!(
            *seen.borrow(),
            Some((b"abc".to_vec(), W14A01_TEXT_NOTIFY_FLAGS))
        );

        let seen = RefCell::new(None);
        assert!(w14a01_apply_text_buffer(b"", |text, flags| {
            *seen.borrow_mut() = Some((text.to_vec(), flags));
        }));
        assert_eq!(*seen.borrow(), Some((Vec::new(), W14A01_TEXT_NOTIFY_FLAGS)));

        let mut buffer = [0x41u8; 0x100];
        buffer[0xff] = 0x42;
        let seen = Cell::new(None);
        assert!(w14a01_apply_text_buffer(&buffer, |text, _| {
            seen.set(Some(text.len()));
        }));
        assert_eq!(seen.get(), Some(0x100));

        assert_eq!(W14A01_SSO_CAPACITY_TAG, 0x8000_000f);
        assert_eq!(W14A01_KEYED_ELEMENT_BYTES, 0x20);
        assert_eq!(W14A01_SLOT_COUNT_OFFSET, 0x1c);
        assert_eq!(W14A01_SLOT_TABLE_OFFSET, 0x20);
        assert_eq!(W14A01_CHILD_OFFSET, 0x110);
        assert_eq!(W14A01_VALUE_INLINE_STRING, 0x1000);
        assert_eq!(W14A01_VALUE_INLINE_LEN_BASE, 0xd);
    }
}
