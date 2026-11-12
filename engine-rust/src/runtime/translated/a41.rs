#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent41.rs
// ===========================================================================
// Translation batch 41.  Only this file is edited by the batch.
//
// The `decompiled_uncited` rows in `0x005e4000..=0x005e5fff` are client-side
// library plumbing, not simulation code: the settings-backed
// "Last Joined Channel List" / "Gateway History" initialisers, the MSVC
// exception-cleanup funclets of the inlined `std::string` assign/destruct
// templates, the bounds-checked reads of the `gameSLOWEST` string table and a
// timestamp-keyed hash table.
//
// Almost every row is a *fragment* of a larger inlined routine.  The
// decompiler cut the bodies at the capacity helper `FUN_00538810` (reserve) and
// at `FUN_0093ec70` (the allocation/deallocation tail), marked the remainder
// unreachable, and left the live registers as `unaff_*`.  What survives is
// the arithmetic and the layout constants; those are what the functions below
// reproduce, exactly, without inventing state that the simulation does not
// have.  Each doc comment names the slice of the retail body it covers, so a
// reader can tell a recovered computation from an omitted side effect.
//
// The rows that carry no computation at all (destructor/cleanup funclets,
// catch filters, thunks and misdecoded instructions) are listed at the end of
// the file.

/// Size in bytes of one `Last Joined Channel List` element (`0x10`).
///
/// `0x005e4060`/`0x005e4073` test the recovered record with
/// `base != count * 0x10 + base`, i.e. they compare the element span against
/// zero rather than the element count.
pub const A41_CHANNEL_RECORD_SIZE: u32 = 0x10;

/// Capacity word written when a string switches to its inline buffer
/// (`0x005e4579`, `0x005e45c2`, `0x005e5af2`): bit 31 is the inline marker and
/// the low bits hold the 15-byte inline capacity.
pub const A41_INLINE_CAPACITY: u32 = 0x8000_000f;

/// Capacity value of the nine parser strings built by `0x005e41dc`.
pub const A41_CLIENT_STRING_CAPACITY: u32 = 0xf;

/// Byte offset of the inline data buffer inside the 12-byte-header string
/// whose reset `0x005e45c2` performs (`object + 3` dwords).
pub const A41_INLINE_DATA_OFFSET: usize = 0xc;

/// Allocation size and alignment of a timestamp-map node (`0x005e5af2`).
pub const A41_HASH_NODE_SIZE: u32 = 0x50;
pub const A41_HASH_NODE_ALIGN: u32 = 8;

/// Offset of the node's inline string buffer (`node[5]` dwords).
pub const A41_HASH_NODE_DATA_OFFSET: usize = 0x14;

/// One `std::string` member of the client parser object created by
/// `0x005e41dc`: a 16-byte inline buffer followed by a size and a capacity
/// field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A41StringMember {
    /// Offset of the inline buffer.
    pub buffer: usize,
    /// Offset of the size field (`buffer + 0x10`).
    pub size_field: usize,
    /// Offset of the capacity field (`buffer + 0x14`).
    pub capacity_field: usize,
    /// Value stored in the capacity field.
    pub capacity: u32,
}

/// Buffer offsets of the nine parser strings created empty by `0x005e41dc`.
///
/// The recovered stores come in pairs (`size = 0`, `capacity = 0xf`, first
/// buffer byte `= 0`) with a stride of `0x18` inside each group.  The first
/// group is `0x58, 0x70, 0x88, 0xa0, 0xb8`; the second follows the parser's
/// vtable and flag block at `0x120, 0x138, 0x150, 0x168`.
pub const A41_CLIENT_STRING_BUFFERS: [usize; 9] =
    [0x58, 0x70, 0x88, 0xa0, 0xb8, 0x120, 0x138, 0x150, 0x168];

/// Layout of a string reset to its inline buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A41InlineString {
    /// `data` points at the object itself plus this offset.
    pub data_offset: usize,
    /// Length field.
    pub size: u32,
    /// Capacity word (inline marker + capacity).
    pub capacity: u32,
    /// First byte of the inline buffer.
    pub first_byte: u8,
}

/// One node of the circular doubly-linked ring whose sentinel `0x005e4060`
/// initialises and into which `0x005e474d` links a new node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A41Link {
    pub next: usize,
    pub prev: usize,
}

/// Index of the ring sentinel; every ring starts with exactly this node.
pub const A41_RING_HEAD: usize = 0;

/// Model of the intrusive ring used by the `Last Joined Channel List`
/// record: `{next, prev, count}` in the sentinel, `{next, prev}` in every
/// node, and a 32-bit element count.
///
/// Retail stores raw pointers; the model keeps the same link graph with
/// indices so the invariant (`head.next.prev == head`, ring order) can be
/// checked directly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A41Ring {
    links: Vec<A41Link>,
    count: u32,
}

impl A41Ring {
    /// `0x005e4060`: create the sentinel.
    ///
    /// The three stores at the head of the body write `param_1` into the
    /// sentinel's `next` and `prev` slots and zero the count, so an empty ring
    /// points both links at itself.  The remainder of that row looks the
    /// "Last Joined Channel List" setting up and tests the recovered record;
    /// the test is [`a41_channels_present`] (`0x005e4073`) and the settings
    /// lookup has no simulation counterpart.
    pub fn a41_empty() -> Self {
        Self {
            links: vec![A41Link {
                next: A41_RING_HEAD,
                prev: A41_RING_HEAD,
            }],
            count: 0,
        }
    }

    /// `0x005e474d`: link a fresh node in front of the sentinel.
    ///
    /// The recovered tail of the row is the classic `push_front` of a
    /// circular doubly-linked list:
    ///
    /// ```text
    /// node->next      = head->next
    /// node->prev      = head
    /// head->next      = node
    /// node->next->prev = node
    /// head->count     = head->count + 1
    /// ```
    ///
    /// The same funclet first writes the NUL terminator of the pending string
    /// (`buffer[len] = 0`, then stores the buffer pointer); that store is the
    /// one modelled by [`a41_terminate_at`] for `0x005e4f42`.  The returned
    /// value is the new node's index, which stands in for the retail pointer.
    pub fn a41_push_front(&mut self) -> usize {
        let node = self.links.len();
        let next = self.links[A41_RING_HEAD].next;
        self.links.push(A41Link {
            next,
            prev: A41_RING_HEAD,
        });
        self.links[A41_RING_HEAD].next = node;
        self.links[next].prev = node;
        // Retail increments the 32-bit counter; wrapping is the only
        // observable behaviour once it saturates.
        self.count = self.count.wrapping_add(1);
        node
    }

    /// Element count from the sentinel's third dword.
    pub fn a41_count(&self) -> u32 {
        self.count
    }

    /// Ring contents from the sentinel's `next` link to its `prev` link,
    /// newest node first.  Indices are the model's stand-in for pointers.
    pub fn a41_order(&self) -> Vec<usize> {
        let mut order = Vec::new();
        let mut at = self.links[A41_RING_HEAD].next;
        while at != A41_RING_HEAD {
            order.push(at);
            at = self.links[at].next;
        }
        order
    }

    /// Link record of one node (index 0 is the sentinel).
    pub fn a41_link(&self, node: usize) -> Option<A41Link> {
        self.links.get(node).copied()
    }
}

/// `0x005e4073`: does the recovered container record describe a non-empty
/// `Last Joined Channel List`?
///
/// The row looks the setting up, requires the stored value's type word
/// (`record + 0xe`) to be `4`, and then evaluates
///
/// ```text
/// record[2] != record[0] * 0x10 + record[2]
/// ```
///
/// where `record[0]` is the element count, `record[2]` the element base and
/// `0x10` the element size.  The right-hand side subtracts out, so the test is
/// `count * 0x10 != 0`; because the multiply is 32-bit and wrapping, a count
/// that is a multiple of `0x1000_0000` also passes as "empty".  `kind` is the
/// type word, `elements` the count.
///
/// The other half of the row initialises the same sentinel modelled by
/// [`A41Ring::a41_empty`] (`0x005e4060`).
pub fn a41_channels_present(kind: u16, elements: u32) -> bool {
    kind == 4 && elements.wrapping_mul(A41_CHANNEL_RECORD_SIZE) != 0
}

/// `0x005e41dc`: the empty-string members of the client parser constructor.
///
/// The recovered stores repeat the triple `size = 0`, `capacity = 0xf`,
/// `first byte = 0`, and the buffer offsets follow from the decompiler's
/// dword-indexed stores (for example the byte store at `ESI + 0x16` is byte
/// offset `0x58`, and the size store at `ESI[0x1a]` is `0x68 = 0x58 + 0x10`).
///
/// The constructor also installs the parser vtable and zeroes several flag
/// dwords; those need the client object that [`crate::sim::World`] does not
/// keep, so only the string table is reproduced.
pub fn a41_client_string_members() -> [A41StringMember; 9] {
    A41_CLIENT_STRING_BUFFERS.map(|buffer| A41StringMember {
        buffer,
        size_field: buffer + 0x10,
        capacity_field: buffer + 0x14,
        capacity: A41_CLIENT_STRING_CAPACITY,
    })
}

/// `0x005e4579`: capacity word transferred to a string during assignment.
///
/// The body reads the source capacity (`puVar2[2]`) and the destination
/// capacity, then writes back
///
/// ```text
/// (src ^ (dest & 0x7fff_ffff)) & 0x7fff_ffff ^ (dest & 0x7fff_ffff)
/// ```
///
/// which is the identity `src & 0x7fff_ffff`: the two XORs with the masked
/// destination cancel on bits 0..30 and bit 31 ends up clear.  The function
/// keeps the literal expression so the recovered form is visible, and the
/// test pins the identity.
///
/// `0x005e45c2`, `FUN_005e58c2` and `FUN_005e5908` contain the same expression on
/// their own destination/source pairs.
pub fn a41_transfer_capacity(dest: u32, src: u32) -> u32 {
    let dest = dest & 0x7fff_ffff;
    ((src ^ dest) & 0x7fff_ffff) ^ dest
}

/// `0x005e45c2`: reset the moved-from string to its inline buffer.
///
/// After the copy the funclet writes `size = 0`, `data = object + 0xc`,
/// `capacity = 0x8000_000f` and a NUL into the first inline byte.  The same
/// reset appears in `0x005e4579` and in the assign funclet `FUN_005e6120`.
///
/// `FUN_005e4ad0` and the `reserve(0)` funclets (`FUN_005e4c40`, `FUN_005e52e0`,
/// `FUN_005e5420`) perform only the `data = inline buffer` store before calling
/// the capacity helper, so they need no separate entry point.
pub fn a41_inline_string_reset() -> A41InlineString {
    A41InlineString {
        data_offset: A41_INLINE_DATA_OFFSET,
        size: 0,
        capacity: A41_INLINE_CAPACITY,
        first_byte: 0,
    }
}

/// `0x005e4945`: find the list node whose two dwords match the target.
///
/// The row walks a circular list (`node + 4` is the next link, the loop stops
/// at the head) and compares the node's `+0x28` and `+0x2c` dwords with the
/// two locals.  `compare` is the recovered `AL` byte: when it is zero the
/// comparison is skipped for every node, so the walk always runs to the head
/// and nothing matches.  The first matching node wins.
///
/// The body then tail-calls `FUN_005e4920` with the node's `+8`/`+0xc` dwords;
/// that callee is outside this batch and is not modelled.
pub fn a41_find_matching_pair(
    nodes: &[(u32, u32)],
    target: (u32, u32),
    compare: bool,
) -> Option<usize> {
    if !compare {
        return None;
    }
    nodes
        .iter()
        .position(|&(a, b)| a == target.0 && b == target.1)
}

/// Slot chosen for the next 28-byte parser record, as recovered from the
/// coherent tail of `0x005e41ef`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A41ArgumentSlot {
    /// Byte offset of the record: `size * 0x1c`.
    pub offset: u32,
    /// Byte offset of the record's inline string buffer: `offset + 0xc`.
    pub data_offset: u32,
    /// Size field written into the new record.
    pub size: u32,
    /// Capacity word written into the new record.
    pub capacity: u32,
    /// First inline byte written into the new record.
    pub first_byte: u8,
    /// The vector is full (`size == capacity & 0x7fff_ffff`) and its grow
    /// routine (`FUN_006028b0`, outside the batch) must run before the store.
    pub needs_growth: bool,
}

/// `0x005e41ef`: select and initialise the next parser record.
///
/// The row is only partly recovered: its first half is decoded from operand
/// data and contains `halt_baddata`, but the tail is the inlined
/// `vector<parser_record>::push_back` with a 28-byte stride:
///
/// ```text
/// if (size == (capacity & 0x7fff_ffff)) { grow(); size = v[1]; }
/// slot = base + size * 0x1c;
/// slot[1] = 0;               // string size
/// slot[0] = &slot[3];        // inline data at +0xc
/// slot[2] = 0x8000_000f;     // inline marker + 15
/// slot[3].byte[0] = 0;
/// ```
///
/// `size` and `capacity` are the vector's second and third dwords.  The
/// capacity comparison is made against the low 31 bits because bit 31 is the
/// inline marker.  `0x005e41dc` contains the same push_back tail.
pub fn a41_argument_slot(size: u32, capacity: u32) -> A41ArgumentSlot {
    let offset = size.wrapping_mul(0x1c);
    A41ArgumentSlot {
        offset,
        data_offset: offset.wrapping_add(A41_INLINE_DATA_OFFSET as u32),
        size: 0,
        capacity: A41_INLINE_CAPACITY,
        first_byte: 0,
        needs_growth: size == (capacity & 0x7fff_ffff),
    }
}

/// `0x005e4f42`: terminate a string at the recovered length.
///
/// The funclet writes the NUL at `buffer[len]` and then stores `len` in the
/// object's size field; the following call (`FUN_006fa550`) is outside the
/// batch.  `Some(len)` reports the size that is stored, `None` the
/// out-of-range buffer, which retail would have written past.
pub fn a41_terminate_at(buffer: &mut [u8], len: usize) -> Option<usize> {
    *buffer.get_mut(len)? = 0;
    Some(len)
}

/// `0x005e4f0b`: length of a NUL-terminated string.
///
/// The recovered loop advances while the loaded byte is non-zero and hands
/// `end - start` to the capacity helper as `len + 1` (the terminator is
/// included).  The function returns the length without the terminator, which
/// is the value the capacity helper receives minus one.
pub fn a41_c_string_len(bytes: &[u8]) -> usize {
    bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())
}

/// `0x005e4fd0` and `0x005e5010`: length of the first `gameSLOWEST` entry.
///
/// Both rows are funclets of one `string = table[0]` assignment.  They read
/// the global entry count (`0x00f04684`) and only when it is positive read
/// entry zero; the length is the recovered
///
/// ```text
/// (*entry == 0) ? 0 : end - (entry + 1)
/// ```
///
/// loop, i.e. [`a41_c_string_len`].  `None` is the empty-table path, where the
/// assignment is not reached and no length is computed.  The rows then pass
/// the length to the capacity helper; the copy itself was pruned.
pub fn a41_slowest_len0(table: &[&[u8]]) -> Option<usize> {
    let first = table.first()?;
    Some(a41_c_string_len(first))
}

/// `0x005e50ec`: length of `gameSLOWEST[index + 1]` on the cleanup path.
///
/// The funclet probes the table only when the recovered flag byte is zero
/// (`in_AL == '\0'`) and when `index + 1` is strictly below the global entry
/// count, then computes the same length expression as
/// [`a41_slowest_len0`].  `None` covers both the skipped and the
/// out-of-range path; retail simply moves on to the next temporary.
pub fn a41_slowest_len_next_on_cleanup(
    table: &[&[u8]],
    index: usize,
    cleanup: bool,
) -> Option<usize> {
    if !cleanup {
        return None;
    }
    let slot = index.checked_add(1)?;
    table.get(slot).map(|entry| a41_c_string_len(entry))
}

/// `0x005e510c`: length of `gameSLOWEST[index + 1]`.
///
/// Same bounds test and length expression as
/// [`a41_slowest_len_next_on_cleanup`], without the flag-byte gate: the row is
/// the unconditional copy of the next entry.
pub fn a41_slowest_len_next(table: &[&[u8]], index: usize) -> Option<usize> {
    let slot = index.checked_add(1)?;
    table.get(slot).map(|entry| a41_c_string_len(entry))
}

/// `0x005e56f0`: default record of the "Gateway History" setting.
///
/// The row branches on whether the setting exists, but both arms write the
/// same four dwords: `8`, `0`, `0` and `0x3f80_0000`.  The last value is the
/// IEEE-754 encoding of `1.0f32`, so the record is
/// `{kind = 8, first = 0, second = 0, scale = 1.0}`.
///
/// The present arm also applies the kind-4/non-empty record test modelled by
/// [`a41_channels_present`]; the "Gateway History" key itself has no
/// simulation counterpart.
pub fn a41_gateway_history_default() -> [u32; 4] {
    [8, 0, 0, 0x3f80_0000]
}

/// `0x005e5a3f` and `0x005e5a78`: byte offset of a timestamp-map bucket.
///
/// Both rows hash the JSON key (`FUN_00c62800`, outside this batch), then index
/// the bucket array as
///
/// ```text
/// base + ((count - 1) & hash) * 4
/// ```
///
/// The subtraction is unsigned, so a table with zero slots degenerates to
/// `hash` itself (`* 4` still applies); retail initialises the table first,
/// which is the `table_present` argument (`None`).  The mask is the
/// power-of-two bucket selection of a chained hash table, and the returned
/// dword is the offset to add to the bucket base.
///
/// `0x005e5a2f` computes the same offset and additionally records the
/// timestamp pair modelled by [`a41_timestamp_pair`].
pub fn a41_timestamp_bucket_offset(hash: u32, slots: u32, table_present: bool) -> Option<u32> {
    if !table_present {
        return None;
    }
    Some((slots.wrapping_sub(1) & hash).wrapping_mul(4))
}

/// `0x005e5a2f`: the pair of timestamps kept for a hit node.
///
/// The row stores the current stamp in one local and, in the next, the
/// unsigned difference against the node's `+0x28` field: `(now,
/// now - node_stamp)`.  The subtraction wraps exactly like the 32-bit store.
pub fn a41_timestamp_pair(now: u32, node_stamp: u32) -> (u32, u32) {
    (now, now.wrapping_sub(node_stamp))
}

/// Layout of a timestamp-map node allocated by `0x005e5af2`.
///
/// The allocator call is `(0x50, 8)`.  The recovered field stores are
/// `node[0] = *bucket` (the chain head), `node[2] = node + 5` (inline data
/// pointer, `0x14` bytes in), `node[3] = 0` (size), `node[4] = 0x8000_000f`
/// (inline capacity) and a NUL in the first inline byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A41HashNode {
    pub allocation: u32,
    pub alignment: u32,
    /// Value copied out of the bucket slot (the previous chain head).
    pub chain: Option<u32>,
    pub data_offset: usize,
    pub size: u32,
    pub capacity: u32,
    pub first_byte: u8,
}

/// `0x005e5af2`: initialise a timestamp-map node.
///
/// `chain` is the bucket slot's previous head; `None` is a null slot.
pub fn a41_hash_node_layout(chain: Option<u32>) -> A41HashNode {
    A41HashNode {
        allocation: A41_HASH_NODE_SIZE,
        alignment: A41_HASH_NODE_ALIGN,
        chain,
        data_offset: A41_HASH_NODE_DATA_OFFSET,
        size: 0,
        capacity: A41_INLINE_CAPACITY,
        first_byte: 0,
    }
}

// Rows in `0x005e4000..=0x005e5fff` that were not translated:
//
// * `FUN_005e400f`, `FUN_005e402e`, `FUN_005e44eb`, `FUN_005e450e`,
//   `FUN_005e466a`, `FUN_005e468d`, `FUN_005e478e`, `FUN_005e47ab`,
//   `FUN_005e49f1`, `FUN_005e4a48`, `FUN_005e514a`, `FUN_005e5172`,
//   `FUN_005e51ad`, `FUN_005e528c`, `FUN_005e52b3`, `FUN_005e5e43`,
//   `FUN_005e5e63` - destructor/cleanup funclets: a virtual call through
//   `vtable + 0x10`, an exception-list restore and the `FUN_00c86b6e` stack
//   cookie check.  There is no arithmetic to reproduce and the object being
//   destroyed lives in the caller's frame.
// * `FUN_005e404c` - only the stack-cookie check (`FUN_00c86b6e`), no body.
// * `FUN_005e41a2`, `FUN_005e41a5`, `FUN_005e42cb`, `FUN_005e5fc0` - recovered
//   as `halt_baddata`/overlapping instructions; the decompiler started inside
//   operand data, so no instruction boundary exists at the row's address.
// * `FUN_005e44c3`, `FUN_005e4642` - cleanup chains that terminate the pending
//   string (`buffer[len] = 0`, `size = len`), release a heap buffer when the
//   inline bit is clear and then reset the inline string.  Every store is one
//   already modelled by [`a41_terminate_at`] or [`a41_inline_string_reset`];
//   the rows add only the exception-list plumbing.
// * `FUN_005e41ef` - only its push_back tail is coherent (translated as
//   [`a41_argument_slot`]); the head of the row is decoded from operand data
//   and contains `halt_baddata`, so the routine's entry cannot be attributed.
// * `FUN_005e4632`, `FUN_005e5985` - two-byte thunks into rows of this batch.
// * `FUN_005e47d2`, `FUN_005e5cc8` - `Catch@` filter stubs that only return a
//   handler address; nothing to translate.
// * `FUN_005e47c8` - virtual destructor call followed by a tail call to
//   `FUN_005e47e4`; the callee is a cleanup funclet already covered above.
// * `FUN_005e47e4` - advances a 16-byte-stride element pointer (`+0x10`),
//   compares it with the recovered end pointer, then builds the throw record.
//   Only the stride/end test is arithmetic, and the consuming tail was pruned.
// * `FUN_005e4820` - builds a channel-list local through `0x005e4060` and
//   then tail-calls the capacity helper with the record's `[1]` dword; the
//   copy that consumes it was pruned as unreachable.
// * `FUN_005e4920` - tail call to `FUN_0067d6b0` with three frame dwords
//   (`node + 8`, `node + 0xc`, local); no local computation.
// * `FUN_005e509b`, `FUN_005e50b3` - tail calls to `FUN_0067d6b0` with two and
//   four frame dwords (the delete of the recovered objects); no local
//   computation.
// * `FUN_005e4a80`, `FUN_005e4b60`, `FUN_005e51c0`, `FUN_005e5560`,
//   `FUN_005e5e90` - exception-frame setup followed by the capacity helper
//   with a constant (`0x13`, `0x19`, `0xc`, `0x11`, `0xd`).  The constant is a
//   reserve request, and the return path after the call was pruned, so the
//   rows do not carry self-contained behaviour.
// * `FUN_005e4d94` - copies the first argument dword into a frame local and
//   calls the capacity helper with zero; no arithmetic.
// * `FUN_005e523b`, `FUN_005e54d0` - frames around a call to `FUN_005e6060`
//   (preceded by `FUN_00754c10`) and to `FUN_005e6120` respectively, plus a
//   misdecoded prologue in `FUN_005e54d0` that adds registers into client
//   fields; the callees are outside the batch.
// * `FUN_005e586a`, `FUN_005e595b`, `FUN_005e599a`, `FUN_005e59c8`,
//   `FUN_005e59e7` - reject/reset funclets of the `gameSLOWEST` copy: a
//   string reset, a data-pointer store, the exception-list restore and the
//   capacity helper; the only reusable computation is the reset modelled by
//   [`a41_inline_string_reset`].
// * `FUN_005e58c2`, `FUN_005e5908` - two more copies of the string
//   move-assign funclets; their capacity expression is the one implemented by
//   [`a41_transfer_capacity`] and the rest is the same reset/reserve tail.

#[cfg(test)]
mod tests_agent41 {
    use super::*;

    /// `0x005e4060`: the empty ring points both sentinel links at itself and
    /// reports a zero count.
    #[test]
    fn empty_ring_is_self_linked() {
        let ring = A41Ring::a41_empty();
        assert_eq!(
            ring.a41_link(A41_RING_HEAD),
            Some(A41Link {
                next: A41_RING_HEAD,
                prev: A41_RING_HEAD,
            })
        );
        assert_eq!(ring.a41_count(), 0);
        assert!(ring.a41_order().is_empty());
    }

    /// `0x005e474d`: each insertion lands in front of the sentinel, links are
    /// reciprocal, and the count follows the nodes.
    #[test]
    fn push_front_keeps_the_ring_consistent() {
        let mut ring = A41Ring::a41_empty();
        let first = ring.a41_push_front();
        let second = ring.a41_push_front();
        let third = ring.a41_push_front();

        assert_eq!((first, second, third), (1, 2, 3));
        assert_eq!(ring.a41_order(), vec![third, second, first]);
        assert_eq!(ring.a41_count(), 3);

        // Every node's `next->prev` points back at it, sentinel included.
        for node in 0..=third {
            let link = ring.a41_link(node).unwrap();
            assert_eq!(ring.a41_link(link.next).unwrap().prev, node);
            assert_eq!(ring.a41_link(link.prev).unwrap().next, node);
        }
        assert_eq!(ring.a41_link(first).unwrap().next, A41_RING_HEAD);
        assert_eq!(ring.a41_link(A41_RING_HEAD).unwrap().next, third);
    }

    /// `0x005e4073`: the record is non-empty only for a kind-4 value whose
    /// `count * 0x10` span does not wrap to zero.
    #[test]
    fn channel_presence_is_the_wrapped_span_test() {
        assert!(!a41_channels_present(4, 0));
        assert!(a41_channels_present(4, 1));
        assert!(a41_channels_present(4, 7));
        // `count * 0x10` wraps to zero for a multiple of 0x1000_0000, so the
        // recovered subtraction sees an "empty" record.
        assert!(!a41_channels_present(4, 0x1000_0000));
        // Any other type word fails the kind test before the span is read.
        assert!(!a41_channels_present(3, 1));
        assert!(!a41_channels_present(0, 1));
    }

    /// `0x005e41dc`: the nine parser strings are empty, 0xf-capacity members
    /// with a 16-byte buffer and 0x18 stride inside each group.
    #[test]
    fn parser_string_members_have_the_recovered_layout() {
        let members = a41_client_string_members();
        assert_eq!(members.len(), 9);
        for member in members {
            assert_eq!(member.size_field, member.buffer + 0x10);
            assert_eq!(member.capacity_field, member.buffer + 0x14);
            assert_eq!(member.capacity, 0xf);
        }

        let first_group = &members[..5];
        for pair in first_group.windows(2) {
            assert_eq!(pair[1].buffer - pair[0].buffer, 0x18);
        }
        let second_group = &members[5..];
        for pair in second_group.windows(2) {
            assert_eq!(pair[1].buffer - pair[0].buffer, 0x18);
        }
        assert_eq!(members[0].buffer, 0x58);
        assert_eq!(members[5].buffer, 0x120);
    }

    /// `0x005e4579`/`0x005e45c2`: the double-XOR capacity transfer is the
    /// source capacity with its top bit cleared, whatever the destination was.
    #[test]
    fn capacity_transfer_clears_only_the_source_top_bit() {
        let values = [
            0u32,
            1,
            0xf,
            0x7fff_ffff,
            0x8000_0000,
            0xffff_ffff,
            0x1234_5678,
        ];
        for dest in values {
            for src in values {
                assert_eq!(
                    a41_transfer_capacity(dest, src),
                    src & 0x7fff_ffff,
                    "dest={dest:#010x} src={src:#010x}"
                );
            }
        }
    }

    /// `0x005e45c2`: the reset string is inline, empty and carries the inline
    /// marker plus 15-byte capacity.
    #[test]
    fn inline_string_reset_uses_the_inline_layout() {
        let reset = a41_inline_string_reset();
        assert_eq!(reset.data_offset, 0xc);
        assert_eq!(reset.size, 0);
        assert_eq!(reset.capacity, 0x8000_000f);
        assert_eq!(reset.capacity & 0x7fff_ffff, 15);
        assert_eq!(reset.capacity & 0x8000_0000, 0x8000_0000);
        assert_eq!(reset.first_byte, 0);
    }

    /// `0x005e4945`: the walk returns the first node with both dwords equal,
    /// and the recovered flag byte suppresses the comparison entirely.
    #[test]
    fn matching_pair_is_the_first_full_match() {
        let nodes = [(1, 2), (3, 4), (3, 4), (5, 6)];
        assert_eq!(a41_find_matching_pair(&nodes, (3, 4), true), Some(1));
        assert_eq!(a41_find_matching_pair(&nodes, (5, 6), true), Some(3));
        assert_eq!(a41_find_matching_pair(&nodes, (5, 7), true), None);
        assert_eq!(a41_find_matching_pair(&[], (1, 2), true), None);
        assert_eq!(a41_find_matching_pair(&nodes, (1, 2), false), None);
    }

    /// `0x005e41ef`: the 28-byte record stride, the inline reset, and the
    /// full-vector test against the low 31 capacity bits.
    #[test]
    fn argument_slots_use_the_28_byte_stride() {
        let first = a41_argument_slot(0, 0);
        assert_eq!(first.offset, 0);
        assert_eq!(first.data_offset, 0xc);
        assert_eq!(first.size, 0);
        assert_eq!(first.capacity, 0x8000_000f);
        assert_eq!(first.first_byte, 0);
        // An empty vector with zero capacity is already full, so the grow
        // routine runs before the store.
        assert!(first.needs_growth);
        // With a real inline capacity it is not full.
        assert!(!a41_argument_slot(0, 15).needs_growth);

        let second = a41_argument_slot(1, 2);
        assert_eq!(second.offset, 0x1c);
        assert_eq!(second.data_offset, 0x1c + 0xc);
        assert!(!second.needs_growth);

        // The comparison ignores bit 31 of the capacity word.
        assert!(a41_argument_slot(2, 0x8000_0002).needs_growth);
        assert!(!a41_argument_slot(3, 0x8000_0002).needs_growth);
        assert!(a41_argument_slot(0, 0x8000_0000).needs_growth);
    }

    /// `0x005e4f42`: the terminator is written at the recovered length; an
    /// out-of-range length writes nothing.
    #[test]
    fn terminate_at_writes_the_nul_after_the_payload() {
        let mut buffer = *b"abcde";
        assert_eq!(a41_terminate_at(&mut buffer, 3), Some(3));
        assert_eq!(buffer, *b"abc\0e");
        assert_eq!(a41_terminate_at(&mut buffer, 5), None);
        assert_eq!(buffer, *b"abc\0e");
    }

    /// `0x005e4f0b`: the scan stops at the first NUL and ignores later bytes.
    #[test]
    fn c_string_length_stops_at_the_terminator() {
        assert_eq!(a41_c_string_len(b""), 0);
        assert_eq!(a41_c_string_len(b"\0abc"), 0);
        assert_eq!(a41_c_string_len(b"abc\0def"), 3);
        // A slice with no terminator reports its whole length; the retail
        // loop would keep reading, which the model cannot reproduce.
        assert_eq!(a41_c_string_len(b"abc"), 3);
    }

    /// `0x005e4fd0`/`0x005e5010`: the first entry is read only when the table
    /// is non-empty, and an empty entry is length zero.
    #[test]
    fn slowest_first_entry_length_needs_a_table() {
        assert_eq!(a41_slowest_len0(&[]), None);
        assert_eq!(a41_slowest_len0(&[b"".as_slice()]), Some(0));
        assert_eq!(a41_slowest_len0(&[b"gameSLOWEST".as_slice()]), Some(11));
        assert_eq!(
            a41_slowest_len0(&[b"a\0b".as_slice(), b"ignored".as_slice()]),
            Some(1)
        );
    }

    /// `0x005e50ec` and `0x005e510c`: the next entry is probed only when the
    /// `index + 1` slot exists, and `0x005e50ec` also needs the cleanup flag.
    #[test]
    fn slowest_next_entry_is_bounds_checked() {
        let table: [&[u8]; 3] = [b"zero".as_slice(), b"one!".as_slice(), b"two!!".as_slice()];

        assert_eq!(a41_slowest_len_next(&table, 0), Some(4));
        assert_eq!(a41_slowest_len_next(&table, 1), Some(5));
        assert_eq!(a41_slowest_len_next(&table, 2), None);
        assert_eq!(a41_slowest_len_next(&table, usize::MAX), None);

        assert_eq!(a41_slowest_len_next_on_cleanup(&table, 0, true), Some(4));
        assert_eq!(a41_slowest_len_next_on_cleanup(&table, 0, false), None);
        assert_eq!(a41_slowest_len_next_on_cleanup(&table, 2, true), None);
    }

    /// `0x005e56f0`: both arms of the setting check write the same record,
    /// whose fourth dword is the float `1.0`.
    #[test]
    fn gateway_history_default_is_constant() {
        let record = a41_gateway_history_default();
        assert_eq!(record, [8, 0, 0, 0x3f80_0000]);
        assert_eq!(f32::from_bits(record[3]), 1.0);
        assert_eq!(record[0], 8);
    }

    /// `0x005e5a3f`/`0x005e5a78`: the bucket is selected with a power-of-two
    /// mask over the hash and scaled by the 4-byte slot; a missing table
    /// reports "initialise first".
    #[test]
    fn timestamp_bucket_masks_the_hash() {
        assert_eq!(a41_timestamp_bucket_offset(9, 4, true), Some(4));
        assert_eq!(a41_timestamp_bucket_offset(8, 4, true), Some(0));
        assert_eq!(a41_timestamp_bucket_offset(0xffff_ffff, 16, true), Some(60));
        assert_eq!(a41_timestamp_bucket_offset(0, 0, true), Some(0));
        assert_eq!(
            a41_timestamp_bucket_offset(0xdead_beef, 0, true),
            Some(0xdead_beefu32.wrapping_mul(4))
        );
        assert_eq!(a41_timestamp_bucket_offset(9, 4, false), None);
    }

    /// `0x005e5a2f`: the recorded pair is the current stamp and its wrapping
    /// distance from the node stamp.
    #[test]
    fn timestamp_pair_keeps_now_and_the_age() {
        assert_eq!(a41_timestamp_pair(10, 4), (10, 6));
        assert_eq!(a41_timestamp_pair(10, 10), (10, 0));
        assert_eq!(a41_timestamp_pair(1, 2), (1, u32::MAX));
    }

    /// `0x005e5af2`: the node is a 0x50-byte allocation whose 12-byte string
    /// header puts the inline buffer at 0x14.
    #[test]
    fn hash_node_layout_matches_the_allocator_call() {
        let node = a41_hash_node_layout(Some(0x1234));
        assert_eq!(node.allocation, 0x50);
        assert_eq!(node.alignment, 8);
        assert_eq!(node.chain, Some(0x1234));
        assert_eq!(node.data_offset, 0x14);
        assert_eq!(node.size, 0);
        assert_eq!(node.capacity, 0x8000_000f);
        assert_eq!(node.first_byte, 0);
        // Header (two dwords) plus a 12-byte string header is the inline
        // buffer offset, and it must stay inside the allocation.
        assert_eq!(node.data_offset, 8 + 0xc);
        assert!(node.data_offset < node.allocation as usize);

        assert_eq!(a41_hash_node_layout(None).chain, None);
    }
}
