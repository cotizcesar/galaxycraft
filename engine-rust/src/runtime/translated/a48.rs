#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent48.rs
// ===========================================================================
// Translation batch 48.  Only this file is edited by the batch.
//
// `0x005f2000..=0x005f3fff` is the C++ support-code block that the retail
// executable uses for its `args` option parser and for the iostream /
// `std::string` helpers that parser instantiates.  The address-keyed routines
// here are destructors, release tails and a buffer-extent computation; none of
// them touches the simulation.  Each of those is a `decompiled_uncited`
// ledger row from that range.  The small arithmetic helpers they share are
// marked as helpers, not as ledger rows.
//
// Two pieces of exact retail arithmetic dominate the block and are modelled
// directly:
//
// * A `std::string` is small-string optimised.  The length word sits at
//   `buffer + 0x10`, the capacity word at `buffer + 0x14`, and only a
//   capacity of at least `0x10` owns a heap buffer.  The destructor resets the
//   three fields for the small-string case, and otherwise releases
//   `capacity + 1` bytes, or `capacity + 0x24` when `capacity + 1` exceeds
//   `0xfff` (the extra `0x23` is the allocator's header rounding).
// * A vector/byte-array release rounds the extent down: `(end - start) & !3`
//   for byte arrays, `(end - start) / 0x1c * 0x1c` for a vector with
//   `0x1c`-byte elements, then applies the same `+0x23` rounding when the
//   byte count exceeds `0xfff`.
//
// The ledger rows at `0x005f3b2a..0x005f3fd0` are the small-string branches
// and continuation entries of one nineteen-string destructor chain.  Retail
// reaches them by jumping into the middle of the chain with a fixed slot
// index, so each row is translated as "force this slot's small-string reset,
// then keep walking the lower slots".  The chain tables below preserve the
// exact byte offsets and descending order.
//
// Release helpers that are only referenced by role (operator delete, the
// virtual deleting destructor, the sentry-list teardown) are described by
// role only; no address is claimed for a routine that is not reconstructed
// in this batch.

/// One small-string header as the retail destructor fragments address it.
///
/// `buffer` is the 16-byte inline buffer, `size` the length word and
/// `capacity` the capacity word.  The three offsets are the ones the
/// decompiled instructions use; they are never merged into one field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A48Slot {
    /// Offset of the inline buffer (`+0x10` before the length word).
    pub buffer: u32,
    /// Offset of the length word.
    pub size: u32,
    /// Offset of the capacity word.
    pub capacity: u32,
}

impl A48Slot {
    /// Build a slot from the buffer offset using the recovered layout.
    const fn at(buffer: u32) -> Self {
        Self {
            buffer,
            size: buffer + 0x10,
            capacity: buffer + 0x14,
        }
    }
}

/// What one destructor step does to a slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A48Step {
    /// Capacity below `0x10`: write `size = 0`, `capacity = 0xf`,
    /// `buffer[0] = 0`.  The fragment emits no deallocation.
    ResetSso { buffer: u32 },
    /// Capacity of at least `0x10`: release the heap block behind the
    /// inline buffer with the recovered request size.
    FreeHeap { buffer: u32, request: u32 },
}

/// Which shared tail the fragment falls into after its last slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A48Tail {
    /// The nine-slot group destructor rebuilt by
    /// [`a48_chain_3670`]/[`a48_chain_3840`]'s shared continuation.
    GroupTeardown,
    /// The shared out-of-range guard that follows the nineteen-slot chain in
    /// retail.  It is outside this batch's rows, so only its role is named.
    SharedGuard,
}

/// Steps of a destructor fragment plus the tail it continues into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A48Teardown {
    /// Slot operations in retail order, highest offset first.
    pub steps: Vec<A48Step>,
    /// Shared continuation reached after the last slot.
    pub tail: A48Tail,
}

/// What a byte-array release fragment does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A48ArrayAction {
    /// The start pointer is null and the sentry-list teardown follows.
    SentryList,
    /// The start pointer is null and the fixed `0x20`-byte block at `+4` is
    /// released.
    Fixed20 { pointer: u32 },
    /// The start pointer is null and the fragment returns without releasing.
    Nothing,
    /// A non-null buffer: release `request` bytes starting at `start`.
    Free { start: u32, request: u32 },
}

/// Extra iostream bookkeeping block released by the streambuf destructor.
///
/// The block has a secondary pointer at `+4`; when present, retail calls its
/// virtual deleting destructor with the "not the inline sub-object" flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A48ExtraBlock {
    /// Whether the secondary pointer at `+4` is non-null.
    pub secondary: bool,
}

/// Outcome of releasing an [`A48ExtraBlock`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A48ExtraRelease {
    /// Whether the secondary virtual destruction ran.
    pub secondary_released: bool,
    /// Fixed block size passed to the deallocation helper.
    pub request: u32,
}

/// Outcome of the stringbuf destructor at [`a48_stringbuf_teardown`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A48StringbufTeardown {
    /// The buffer was not owned: six pointer words are cleared, the low flag
    /// bit and word `+0xe` are cleared, and the vtable changes to the base
    /// streambuf.  Any extra block is released first.
    Reset { extra: Option<A48ExtraRelease> },
    /// The buffer is owned: release the computed view extent.
    FreeBuffer { request: u32 },
}

/// Outcome of the nine-slot group destructor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A48GroupTeardown {
    /// Whether the owned pointer at `+0x54` was present and cleared.
    pub owned_released: bool,
    /// Release request for the pointer vector at `+0x24`, when non-empty.
    pub vector_request: Option<u32>,
    /// Destructor step for the base string at `+0x0c`.
    pub base_string: A48Step,
}

/// `0x005f2cf0`: NUL-terminated byte length.  Retail advances one byte at a
/// time until it reads zero and passes `end - (start + 1)` to its string
/// constructor, which is exactly the number of bytes before the terminator.
pub fn a48_strlen(bytes: &[u8]) -> u32 {
    bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len()) as u32
}

/// `0x005f3010`: drop the owned object behind slot word 9.
///
/// Retail skips a null slot; otherwise it invokes the object's virtual
/// deleting destructor with a "this is not the inline sub-object" flag and
/// then stores zero in the slot.  The flag only selects the deleting versus
/// non-deleting destructor, so the translated result is whether a value was
/// released.
pub fn a48_take_owned<T>(slot: &mut Option<T>) -> bool {
    slot.take().is_some()
}

/// `0x005f3040`: release the byte array at `[+0xc]..[+0x14]`.
///
/// A null start pointer jumps to the sentry-list teardown instead.  A non-null
/// buffer is released over `(end - start) & 0xfffffffc` bytes, rounded up by
/// `0x23` when that count exceeds `0xfff`.
pub fn a48_array_release_3040(start: u32, end: u32) -> A48ArrayAction {
    if start == 0 {
        return A48ArrayAction::SentryList;
    }
    A48ArrayAction::Free {
        start,
        request: a48_dealloc_request(end.wrapping_sub(start) & !3),
    }
}

/// `0x005f30a0`: release the byte array at `[+0xc]..[+0x14]`.
///
/// This row shares the non-null arithmetic of [`a48_array_release_3040`]; a
/// null start pointer releases the fixed `0x20`-byte block whose address is
/// the word at `+4` (`fallback` here) instead.
pub fn a48_array_release_30a0(start: u32, end: u32, fallback: u32) -> A48ArrayAction {
    if start == 0 {
        return A48ArrayAction::Fixed20 { pointer: fallback };
    }
    A48ArrayAction::Free {
        start,
        request: a48_dealloc_request(end.wrapping_sub(start) & !3),
    }
}

/// `0x005f3570`: release the byte array at `[+0]..[+8]`.
///
/// A null start pointer returns without releasing anything.
pub fn a48_array_release_3570(start: u32, end: u32) -> A48ArrayAction {
    if start == 0 {
        return A48ArrayAction::Nothing;
    }
    A48ArrayAction::Free {
        start,
        request: a48_dealloc_request(end.wrapping_sub(start) & !3),
    }
}

/// `0x005f35d0`: release the `0x1c`-byte-element vector at `[+0]..[+8]`.
///
/// Retail runs the element destructor loop first and then releases
/// `((end - start) / 0x1c) * 0x1c` bytes, rounded up by `0x23` when the count
/// exceeds `0xfff`.  The element loop is represented only by that exact
/// extent arithmetic; the per-element destructor is a separate routine, not
/// reconstructed here.
pub fn a48_vector_release_35d0(start: u32, end: u32) -> A48ArrayAction {
    if start == 0 {
        return A48ArrayAction::Nothing;
    }
    let span = end.wrapping_sub(start);
    let bytes = (span / 0x1c) * 0x1c;
    A48ArrayAction::Free {
        start,
        request: a48_dealloc_request(bytes),
    }
}

/// `0x005f3180`: release the pointer in slot word 0 and clear the slot.
/// Returns whether the pointer was non-null.
pub fn a48_release_pointer(slot: &mut u32) -> bool {
    let present = *slot != 0;
    *slot = 0;
    present
}

/// `0x005f3230`: release the extra iostream block at `+0x34`.
///
/// A present block first runs the secondary virtual destruction when its `+4`
/// pointer is non-null, then releases the eight-byte block itself.  Both the
/// virtual call and the deallocation helper are described by role.
pub fn a48_release_extra(block: Option<A48ExtraBlock>) -> Option<A48ExtraRelease> {
    block.map(|block| A48ExtraRelease {
        secondary_released: block.secondary,
        request: 8,
    })
}

/// `0x005f32a0`: destroy the string whose buffer starts at offset zero.
/// Capacity below `0x10` resets the inline form; otherwise the heap block is
/// released over `capacity + 1` bytes (`capacity + 0x24` past `0xfff`).
pub fn a48_string_release_00(capacity: u32) -> A48Step {
    a48_string_step(A48Slot::at(0), capacity)
}

/// `0x005f3470`: destroy the string whose buffer starts at offset `0x04`
/// (length at `+0x14`, capacity at `+0x18`).
pub fn a48_string_release_04(capacity: u32) -> A48Step {
    a48_string_step(A48Slot::at(0x04), capacity)
}

/// `0x005f34c0`: destroy the string pair of this object.
///
/// The first slot has its buffer at `0x1c` (length `0x2c`, capacity `0x30`);
/// the second at `0x04` (length `0x14`, capacity `0x18`).  Retail handles the
/// first slot, then the lower slot, in that order.
pub fn a48_string_pair_34c0(first_capacity: u32, second_capacity: u32) -> [A48Step; 2] {
    [
        a48_string_step(A48Slot::at(0x1c), first_capacity),
        a48_string_step(A48Slot::at(0x04), second_capacity),
    ]
}

/// `0x005f32f0`: destroy a stringbuf.
///
/// With the low flag bit clear, retail clears the six view pointers, clears
/// that flag bit and word `+0xe`, switches to the base streambuf vtable and
/// releases the extra block at `+0x34` when present.  With the flag set it
/// computes the buffer end from the two view pairs and releases the extent:
/// when the value read through word 8 is zero the end is
/// `word[0xb] + word[7]`, otherwise `word[0xc] + word[8]`; the start is read
/// through word 3.  `buf_start`, `put_end`, `put_end_alt`, `eg_end` and
/// `eg_end_alt` are those dereferenced words.
pub fn a48_stringbuf_teardown(
    owned_buffer: bool,
    buf_start: u32,
    put_end: u32,
    put_end_alt: u32,
    eg_end: u32,
    eg_end_alt: u32,
    extra: Option<A48ExtraBlock>,
) -> A48StringbufTeardown {
    if !owned_buffer {
        return A48StringbufTeardown::Reset {
            extra: a48_release_extra(extra),
        };
    }
    let end = if eg_end == 0 {
        put_end.wrapping_add(put_end_alt)
    } else {
        eg_end.wrapping_add(eg_end_alt)
    };
    A48StringbufTeardown::FreeBuffer {
        request: a48_dealloc_request(end.wrapping_sub(buf_start)),
    }
}

/// Retail's allocator-header adjustment for one release request.
///
/// Extents above `0xfff` are rounded up by `0x23` to cover the header the
/// allocator keeps in front of the block.
pub fn a48_dealloc_request(bytes: u32) -> u32 {
    if bytes > 0xfff {
        bytes.wrapping_add(0x23)
    } else {
        bytes
    }
}

/// Retail's `std::string` heap request for a given capacity word.
///
/// The fragment always computes `capacity + 1` first; when that exceeds
/// `0xfff` retail replaces it with `capacity + 0x24`.
pub fn a48_string_request(capacity: u32) -> u32 {
    let bytes = capacity.wrapping_add(1);
    if bytes > 0xfff {
        capacity.wrapping_add(0x24)
    } else {
        bytes
    }
}

/// One `std::string` destructor step: small-string reset or heap release.
pub fn a48_string_step(slot: A48Slot, capacity: u32) -> A48Step {
    if capacity < 0x10 {
        A48Step::ResetSso {
            buffer: slot.buffer,
        }
    } else {
        A48Step::FreeHeap {
            buffer: slot.buffer,
            request: a48_string_request(capacity),
        }
    }
}

/// The four string slots of the `0x005f3670` destructor, highest first.
pub const A48_TEARDOWN_3670_SLOTS: [A48Slot; 4] = [
    A48Slot::at(0x168),
    A48Slot::at(0x150),
    A48Slot::at(0x138),
    A48Slot::at(0x120),
];

/// The five string slots of the `0x005f3840` group destructor, highest first.
pub const A48_TEARDOWN_3840_SLOTS: [A48Slot; 5] = [
    A48Slot::at(0xb8),
    A48Slot::at(0xa0),
    A48Slot::at(0x88),
    A48Slot::at(0x70),
    A48Slot::at(0x58),
];

/// The nineteen string slots of the shared teardown chain, highest first.
///
/// The offsets are the exact buffer offsets recovered from the rows at
/// `0x005f3af0..0x005f3fd0`; the lower part of the object uses a `0x18`-byte
/// stride.
pub const A48_GIANT_SLOTS: [A48Slot; 19] = [
    A48Slot::at(0x1e8),
    A48Slot::at(0x1cc),
    A48Slot::at(0x1b0),
    A48Slot::at(0x198),
    A48Slot::at(0x17c),
    A48Slot::at(0x164),
    A48Slot::at(0x148),
    A48Slot::at(0x130),
    A48Slot::at(0x118),
    A48Slot::at(0x100),
    A48Slot::at(0xe8),
    A48Slot::at(0xd0),
    A48Slot::at(0xb8),
    A48Slot::at(0xa0),
    A48Slot::at(0x84),
    A48Slot::at(0x6c),
    A48Slot::at(0x54),
    A48Slot::at(0x3c),
    A48Slot::at(0x24),
];

/// Walk `slots` in order, resolving each slot from its stored capacity word.
pub fn a48_release_slots(slots: &[A48Slot], capacities: &[u32]) -> Vec<A48Step> {
    slots
        .iter()
        .zip(capacities.iter().copied())
        .map(|(&slot, capacity)| a48_string_step(slot, capacity))
        .collect()
}

/// `0x005f3670`: destroy the four slots of [`A48_TEARDOWN_3670_SLOTS`] in
/// order, then continue into the group teardown at `0x005f3840` (rebuilt by
/// [`a48_chain_3840`]).
///
/// Retail first invokes the sub-object teardown of `0x005f3af0` (rebuilt by
/// [`a48_chain_3af0`]).  That call runs on an adjusted sub-object base — the
/// two slot tables use different `this`-relative offsets — so it is kept as a
/// separate routine rather than folded into the offsets below.
pub fn a48_chain_3670(capacities: &[u32; 4]) -> A48Teardown {
    A48Teardown {
        steps: a48_release_slots(&A48_TEARDOWN_3670_SLOTS, capacities),
        tail: A48Tail::GroupTeardown,
    }
}

/// `0x005f3840`: destroy the five slots of [`A48_TEARDOWN_3840_SLOTS`] in
/// order, then continue into the group destructor at `0x005f3a20` (rebuilt by
/// [`a48_group_teardown`]).
///
/// Two base-class teardown calls precede the slot walk in retail; they are
/// outside this batch's rows and are represented only by role, so the model
/// starts at the first string slot.
pub fn a48_chain_3840(capacities: &[u32; 5]) -> A48Teardown {
    A48Teardown {
        steps: a48_release_slots(&A48_TEARDOWN_3840_SLOTS, capacities),
        tail: A48Tail::GroupTeardown,
    }
}

/// `0x005f3af0`: the checked entry of the nineteen-slot teardown chain.
///
/// `capacities` are the capacity words of [`A48_GIANT_SLOTS`], highest slot
/// first.  Each step resets or frees as the row does, and the chain ends at
/// the shared out-of-range guard.
pub fn a48_chain_3af0(capacities: &[u32; 19]) -> A48Teardown {
    A48Teardown {
        steps: a48_release_slots(&A48_GIANT_SLOTS, capacities),
        tail: A48Tail::SharedGuard,
    }
}

/// Shared body of the small-string entries of the nineteen-slot chain.
///
/// The entry row is reached only when its capacity was already found below
/// `0x10`, so it resets that slot unconditionally; `capacities` then belongs
/// to the slots below it, highest first.
fn a48_giant_after_sso(entry: usize, capacities: &[u32]) -> A48Teardown {
    let slots = &A48_GIANT_SLOTS[entry..];
    let mut steps = Vec::with_capacity(slots.len());
    steps.push(A48Step::ResetSso {
        buffer: slots[0].buffer,
    });
    steps.extend(a48_release_slots(&slots[1..], capacities));
    A48Teardown {
        steps,
        tail: A48Tail::SharedGuard,
    }
}

/// `0x005f3b2a`: small-string branch of slot `0x1e8`; resets it and walks the
/// eighteen lower slots.
pub fn a48_chain_3b2a(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(0, capacities)
}

/// `0x005f3b7f`: small-string branch of slot `0x1cc`; resets it and walks the
/// seventeen lower slots.
pub fn a48_chain_3b7f(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(1, capacities)
}

/// `0x005f3bd4`: small-string branch of slot `0x1b0`; resets it and walks the
/// sixteen lower slots.
pub fn a48_chain_3bd4(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(2, capacities)
}

/// `0x005f3c29`: small-string branch of slot `0x198`; resets it and walks the
/// fifteen lower slots.
pub fn a48_chain_3c29(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(3, capacities)
}

/// `0x005f3c7e`: small-string branch of slot `0x17c`; resets it and walks the
/// fourteen lower slots.
pub fn a48_chain_3c7e(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(4, capacities)
}

/// `0x005f3cd3`: small-string branch of slot `0x164`; resets it and walks the
/// thirteen lower slots.
pub fn a48_chain_3cd3(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(5, capacities)
}

/// `0x005f3d28`: small-string branch of slot `0x148`; resets it and walks the
/// twelve lower slots.
pub fn a48_chain_3d28(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(6, capacities)
}

/// `0x005f3dd2`: small-string branch of slot `0x118`; resets it and walks the
/// ten lower slots.
pub fn a48_chain_3dd2(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(8, capacities)
}

/// `0x005f3e27`: small-string branch of slot `0x100`; resets it and walks the
/// nine lower slots.
pub fn a48_chain_3e27(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(9, capacities)
}

/// `0x005f3e7c`: small-string branch of slot `0xe8`; resets it and walks the
/// eight lower slots.
pub fn a48_chain_3e7c(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(10, capacities)
}

/// `0x005f3ed1`: small-string branch of slot `0xd0`; resets it and walks the
/// seven lower slots.
pub fn a48_chain_3ed1(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(11, capacities)
}

/// `0x005f3f26`: small-string branch of slot `0xb8`; resets it and walks the
/// six lower slots.
pub fn a48_chain_3f26(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(12, capacities)
}

/// `0x005f3f7b`: small-string branch of slot `0xa0`; resets it and walks the
/// five lower slots.
pub fn a48_chain_3f7b(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(13, capacities)
}

/// `0x005f3fd0`: small-string branch of slot `0x84`; resets it and walks the
/// four lower slots.
pub fn a48_chain_3fd0(capacities: &[u32]) -> A48Teardown {
    a48_giant_after_sso(14, capacities)
}

/// `0x005f3a20`: the group destructor.
///
/// Retail first releases the owned pointer at `+0x54` through its virtual
/// deleting destructor and clears the slot.  It then releases the pointer
/// vector at `+0x24` over `(word[0xb] - pointer) & 0xfffffffc` bytes when it
/// is non-empty, and finally destroys the base string at `+0x0c` (length
/// `+0x1c`, capacity `+0x20`).  In the decompiled listing the non-empty branch
/// ends in the non-returning deallocation helper, so the inlined string tail
/// is only visible on the empty-vector branch; the translated model reports
/// both parts and keeps the retail order (owned, vector, string).
pub fn a48_group_teardown(
    owned: bool,
    vector: Option<(u32, u32)>,
    string_capacity: u32,
) -> A48GroupTeardown {
    A48GroupTeardown {
        owned_released: owned,
        vector_request: vector
            .map(|(start, end)| a48_dealloc_request(end.wrapping_sub(start) & !3)),
        base_string: a48_string_step(A48Slot::at(0x0c), string_capacity),
    }
}

#[cfg(test)]
mod tests_agent48 {
    use super::*;

    #[test]
    fn dealloc_request_boundaries() {
        assert_eq!(a48_dealloc_request(0), 0);
        assert_eq!(a48_dealloc_request(0xfff), 0xfff);
        assert_eq!(a48_dealloc_request(0x1000), 0x1023);
        assert_eq!(a48_string_request(0xf), 0x10);
        assert_eq!(a48_string_request(0xffe), 0xfff);
        assert_eq!(a48_string_request(0xfff), 0x1023);
    }

    #[test]
    fn strlen_stops_at_terminator() {
        assert_eq!(a48_strlen(b"abc\0def"), 3);
        assert_eq!(a48_strlen(b"\0"), 0);
        assert_eq!(a48_strlen(b"abc"), 3);
    }

    #[test]
    fn owned_and_pointer_release() {
        let mut owned = Some(7u32);
        assert!(a48_take_owned(&mut owned));
        assert_eq!(owned, None);
        assert!(!a48_take_owned(&mut owned));

        let mut pointer = 0x1234;
        assert!(a48_release_pointer(&mut pointer));
        assert_eq!(pointer, 0);
        assert!(!a48_release_pointer(&mut pointer));
    }

    #[test]
    fn byte_array_release_variants() {
        assert_eq!(a48_array_release_3040(0, 0x800), A48ArrayAction::SentryList);
        assert_eq!(
            a48_array_release_3040(0x1000, 0x1007),
            A48ArrayAction::Free {
                start: 0x1000,
                request: 4
            }
        );
        assert_eq!(
            a48_array_release_3040(0x2000, 0x4004),
            A48ArrayAction::Free {
                start: 0x2000,
                request: 0x2004 + 0x23
            }
        );
        assert_eq!(
            a48_array_release_30a0(0, 0x40, 0x9000),
            A48ArrayAction::Fixed20 { pointer: 0x9000 }
        );
        assert_eq!(a48_array_release_3570(0, 0x40), A48ArrayAction::Nothing);
        assert_eq!(
            a48_array_release_3570(0x1000, 0x1003),
            A48ArrayAction::Free {
                start: 0x1000,
                request: 0
            }
        );
    }

    #[test]
    fn vector_release_rounds_down_to_element_multiple() {
        assert_eq!(a48_vector_release_35d0(0, 0x80), A48ArrayAction::Nothing);
        assert_eq!(
            a48_vector_release_35d0(0x1000, 0x1000 + 0x1c * 3 + 5),
            A48ArrayAction::Free {
                start: 0x1000,
                request: 0x1c * 3
            }
        );
        assert_eq!(
            a48_vector_release_35d0(0x1000, 0x1000 + 0x1c * 0x100),
            A48ArrayAction::Free {
                start: 0x1000,
                request: 0x1c * 0x100 + 0x23
            }
        );
    }

    #[test]
    fn string_slots_use_recovered_offsets() {
        assert_eq!(a48_string_release_00(0xf), A48Step::ResetSso { buffer: 0 });
        assert_eq!(
            a48_string_release_00(0x10),
            A48Step::FreeHeap {
                buffer: 0,
                request: 0x11
            }
        );
        assert_eq!(
            a48_string_release_04(0x20),
            A48Step::FreeHeap {
                buffer: 0x04,
                request: 0x21
            }
        );
        assert_eq!(
            a48_string_pair_34c0(0xf, 0x10),
            [
                A48Step::ResetSso { buffer: 0x1c },
                A48Step::FreeHeap {
                    buffer: 0x04,
                    request: 0x11
                }
            ]
        );
    }

    #[test]
    fn extra_block_release_is_eight_bytes() {
        assert_eq!(a48_release_extra(None), None);
        assert_eq!(
            a48_release_extra(Some(A48ExtraBlock { secondary: true })),
            Some(A48ExtraRelease {
                secondary_released: true,
                request: 8
            })
        );
        assert_eq!(
            a48_release_extra(Some(A48ExtraBlock { secondary: false })),
            Some(A48ExtraRelease {
                secondary_released: false,
                request: 8
            })
        );
    }

    #[test]
    fn stringbuf_reset_branch_clears_and_releases_extra() {
        assert_eq!(
            a48_stringbuf_teardown(false, 0x1000, 0, 0, 0, 0, None),
            A48StringbufTeardown::Reset { extra: None }
        );
        assert_eq!(
            a48_stringbuf_teardown(
                false,
                0x1000,
                0,
                0,
                0,
                0,
                Some(A48ExtraBlock { secondary: true })
            ),
            A48StringbufTeardown::Reset {
                extra: Some(A48ExtraRelease {
                    secondary_released: true,
                    request: 8
                })
            }
        );
    }

    #[test]
    fn stringbuf_owned_extent_uses_both_view_pairs() {
        assert_eq!(
            a48_stringbuf_teardown(true, 0x1000, 0x1000, 0x20, 0, 0xdead, None),
            A48StringbufTeardown::FreeBuffer { request: 0x20 }
        );
        assert_eq!(
            a48_stringbuf_teardown(true, 0xff8, 0xdead, 0xdead, 8, 0x1000, None),
            A48StringbufTeardown::FreeBuffer { request: 0x10 }
        );
        assert_eq!(
            a48_stringbuf_teardown(true, 0x1000, 0, 0, 8, 0x1001, None),
            A48StringbufTeardown::FreeBuffer { request: 9 }
        );
    }

    #[test]
    fn four_and_five_slot_chains_keep_descending_order() {
        let four = a48_chain_3670(&[0xf, 0x10, 0xf, 0x20]);
        assert_eq!(
            four.steps,
            vec![
                A48Step::ResetSso { buffer: 0x168 },
                A48Step::FreeHeap {
                    buffer: 0x150,
                    request: 0x11
                },
                A48Step::ResetSso { buffer: 0x138 },
                A48Step::FreeHeap {
                    buffer: 0x120,
                    request: 0x21
                },
            ]
        );
        assert_eq!(four.tail, A48Tail::GroupTeardown);

        let five = a48_chain_3840(&[0xf; 5]);
        assert_eq!(five.steps.len(), 5);
        assert_eq!(five.steps[0], A48Step::ResetSso { buffer: 0xb8 });
        assert_eq!(five.steps[4], A48Step::ResetSso { buffer: 0x58 });
        assert_eq!(five.tail, A48Tail::GroupTeardown);
    }

    #[test]
    fn giant_chain_root_covers_all_nineteen_slots() {
        let all_sso = a48_chain_3af0(&[0xf; 19]);
        assert_eq!(all_sso.steps.len(), 19);
        assert_eq!(all_sso.steps[0], A48Step::ResetSso { buffer: 0x1e8 });
        assert_eq!(all_sso.steps[18], A48Step::ResetSso { buffer: 0x24 });
        assert_eq!(all_sso.tail, A48Tail::SharedGuard);

        let all_heap = a48_chain_3af0(&[0x10; 19]);
        assert_eq!(
            all_heap.steps[0],
            A48Step::FreeHeap {
                buffer: 0x1e8,
                request: 0x11
            }
        );
        assert_eq!(
            all_heap.steps[18],
            A48Step::FreeHeap {
                buffer: 0x24,
                request: 0x11
            }
        );
    }

    #[test]
    fn giant_sso_entries_force_their_own_slot() {
        type Entry = fn(&[u32]) -> A48Teardown;
        let entries: [(usize, Entry); 14] = [
            (0, a48_chain_3b2a),
            (1, a48_chain_3b7f),
            (2, a48_chain_3bd4),
            (3, a48_chain_3c29),
            (4, a48_chain_3c7e),
            (5, a48_chain_3cd3),
            (6, a48_chain_3d28),
            (8, a48_chain_3dd2),
            (9, a48_chain_3e27),
            (10, a48_chain_3e7c),
            (11, a48_chain_3ed1),
            (12, a48_chain_3f26),
            (13, a48_chain_3f7b),
            (14, a48_chain_3fd0),
        ];
        for (entry, build) in entries {
            // The probe uses heap-sized capacities: the entry slot must still
            // reset because retail only reaches this row after the compare
            // found the capacity below `0x10`.
            let teardown = build(&[0x10; 18]);
            assert_eq!(teardown.steps.len(), 19 - entry, "entry {entry}");
            assert_eq!(
                teardown.steps[0],
                A48Step::ResetSso {
                    buffer: A48_GIANT_SLOTS[entry].buffer
                },
                "entry {entry}"
            );
            assert_eq!(
                teardown.steps.last(),
                Some(&A48Step::FreeHeap {
                    buffer: 0x24,
                    request: 0x11
                }),
                "entry {entry}"
            );
            assert_eq!(teardown.tail, A48Tail::SharedGuard);
        }
    }

    #[test]
    fn giant_sso_entry_then_mixed_lower_slots() {
        let mut caps = [0x10; 18];
        caps[0] = 0x1000;
        caps[17] = 0xfff;
        let teardown = a48_chain_3b2a(&caps);
        assert_eq!(teardown.steps[0], A48Step::ResetSso { buffer: 0x1e8 });
        assert_eq!(
            teardown.steps[1],
            A48Step::FreeHeap {
                buffer: 0x1cc,
                request: 0x1024
            }
        );
        assert_eq!(
            teardown.steps[18],
            A48Step::FreeHeap {
                buffer: 0x24,
                request: 0x1023
            }
        );
    }

    #[test]
    fn group_teardown_reports_owned_vector_and_base_string() {
        let teardown = a48_group_teardown(true, Some((0x1000, 0x1008)), 0x10);
        assert!(teardown.owned_released);
        assert_eq!(teardown.vector_request, Some(8));
        assert_eq!(
            teardown.base_string,
            A48Step::FreeHeap {
                buffer: 0x0c,
                request: 0x11
            }
        );

        let empty = a48_group_teardown(false, None, 0xf);
        assert!(!empty.owned_released);
        assert_eq!(empty.vector_request, None);
        assert_eq!(empty.base_string, A48Step::ResetSso { buffer: 0x0c });
    }
}
