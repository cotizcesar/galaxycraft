#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave13/agent07.rs
// ===========================================================================
// Wave 13 translation batch 07 (staging).
//
// Window: `00668000..00669fff` (hex).
//
// This window is client/runtime support code, not simulation state.  It
// contains the 1.23.10 instantiations of the MSVC `std::unordered_map`
// bucket helpers (the same `_Hash` layout used by the `0x005e....` rows of
// batch 43), the type-erased 20-byte callable storage used by
// `std::function`, and the `ClientSdk::IInternetFileUploadInfoV1`
// aggregate/constructor/destructor family.  Nothing below touches
// [`crate::sim::World`] or the renderer; every translation is a free
// function over values, slices or caller-supplied callbacks.
//
// The decompiler cut several routines at the global allocator singleton
// (`FUN_0093ec70`, which it marks non-returning) or at the first 16-bit
// store, so those bodies were read from the shipped instruction bytes as
// well.  Each doc comment says exactly which slice of the retail body the
// Rust function covers; a routine whose remaining work is an opaque
// allocator/vtable call reports the recovered words and leaves the call to
// the caller.
//
// ## Recovered layout
//
// | object | offset | field |
// |--------|--------|-------|
// | hash table | `+0x00` | bucket count, power of two |
// | hash table | `+0x04` | bucket base (null before the first grow) |
// | hash table | `+0x08` | element count |
// | hash table | `+0x0c` | maximum load factor (`f32`) |
// | key node | `+0x00` | next node in the bucket chain |
// | key node | `+0x08` | key bytes pointer |
// | key node | `+0x0c` | key length |
// | key node | `+0x28` | mapped value (released by `0x00668a60`) |
// | upload info | `+0x00` | three zero words |
// | upload info | `+0x0c` | kind byte / subtype byte |
// | upload info | `+0x0e` | 16-bit type word |
// | upload info | `+0x10` | stream word, or child pointer |
// | upload info | `+0x14` | child pointer alias |
// | upload info | `+0x18` | fourth constructor argument |
// | upload info | `+0x2c` | third constructor argument |
// | upload info | `+0x34` | last word (record size `0x38`) |
// | upload child | `+0x00` | five words, second word `0x00010000` |
// | callable slot | `+0x00` | storage word (20-byte slot) |
//
// ## Skipped rows
//
// The throw-only wrappers at the start of the window (the integer, float,
// `int64`, string and string-plus-length key formatters) have no surviving
// value: every path ends in the non-returning out-of-range throw, so they
// are not modelled.  The `halt_baddata` row is not a routine.  The hash
// table constructors and the iterator walk whose tail is a non-returning
// cleanup call are not reproduced.  The array destructor in
// the middle of the window was truncated after its first element and needs
// a `0x98`-byte element layout that the body does not expose.  The
// static-table guard depends on the `0x00f94a30` global and on a
// `0x20`-stride record whose key field is behind an unrecovered table.  The
// range initialiser whose decompile stops at the allocator prologue (the
// `0x1c`-stride string list) is left to a later pass.  The upload-manager
// destructors, their duplicate, and the jump-table dispatcher are skipped:
// the former need the full `0xb8`-byte layout and the latter's jump table
// is unrecovered.  The frame-restore stub accumulates `unaff_EBP` slots and
// is not a routine; it and the other argument-free fragments are listed in
// the batch report.

// ---------------------------------------------------------------------------
// Hash-container helpers (`_Hash` instantiations)
// ---------------------------------------------------------------------------

/// Byte offset of the next-node word in a keyed hash node.
pub const W13A07_HASH_NODE_NEXT: usize = 0x00;
/// Byte offset of the key-bytes pointer in a keyed hash node.
pub const W13A07_HASH_NODE_KEY_PTR: usize = 0x08;
/// Byte offset of the key length in a keyed hash node.
pub const W13A07_HASH_NODE_KEY_LEN: usize = 0x0c;
/// Byte offset of the mapped value in this instantiation's nodes.
pub const W13A07_HASH_NODE_VALUE: usize = 0x28;
/// Smallest bucket count the growth routine ever returns.
pub const W13A07_HASH_MIN_BUCKETS: u32 = 8;

/// The three words written by the bucket probe at `0x00668f80`.
///
/// The retail routine stores a node pointer and two bucket-relative byte
/// offsets.  `next_offset` is the matching bucket slot when the key is
/// present and the table's end offset otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a07HashProbe {
    /// First output word: the chain node, or `None` when the bucket probe
    /// stopped at a null link.
    pub node: Option<u32>,
    /// Second output word: `(bucket_count - 1 & hash) * 4` on a hit, else
    /// `end_offset`.
    pub next_offset: u32,
    /// Third output word: `bucket_count * 4`.
    pub end_offset: u32,
}

/// `0x00668f80`: probe the bucket chain for a key.
///
/// ```c
/// if (table[1] == 0) {                 // no bucket array yet
///   out[0] = 0;
///   out[1] = table[0] << 2;
///   out[2] = table[0] << 2;
///   return;
/// }
/// hash = FUN_00c62800(key.ptr, key.len, 0);       // MurmurHash3 x86-32
/// slot = table[1] + (table[0] - 1 & hash) * 4;
/// link = FUN_005e8660(slot, &key);                // chain walk
/// out[0] = *link;
/// out[1] = (*link != 0) ? slot : table[1] + table[0] * 4;
/// out[2] = table[1] + table[0] * 4;
/// ```
///
/// `found_node` is `*link`, i.e. the node the chain walk reached.  The
/// chain walk itself (`FUN_005e8660`) compares the node key at `+0x08`/`+0x0c`
/// against the search key and returns the link pointing at the matching
/// node, or the last link on a miss.
pub fn w13a07_hash_bucket_probe(
    bucket_count: u32,
    buckets_allocated: bool,
    key: &[u8],
    found_node: Option<u32>,
) -> W13a07HashProbe {
    let end_offset = bucket_count.wrapping_mul(4);
    if !buckets_allocated {
        return W13a07HashProbe {
            node: None,
            next_offset: end_offset,
            end_offset,
        };
    }
    let hash = a43_hash_bytes(key);
    let slot = (bucket_count.wrapping_sub(1) & hash).wrapping_mul(4);
    match found_node {
        Some(node) => W13a07HashProbe {
            node: Some(node),
            next_offset: slot,
            end_offset,
        },
        None => W13a07HashProbe {
            node: None,
            next_offset: end_offset,
            end_offset,
        },
    }
}

/// `0x006686a0`: `contains(key)`.
///
/// The body probes into a 12-byte local and returns `local[0] != 0`; the
/// lookup and key comparison are the probe at `0x00668f80`.  The returned
/// boolean is exactly the probe's node word.
pub fn w13a07_hash_contains(
    bucket_count: u32,
    buckets_allocated: bool,
    key: &[u8],
    found_node: Option<u32>,
) -> bool {
    w13a07_hash_bucket_probe(bucket_count, buckets_allocated, key, found_node)
        .node
        .is_some()
}

/// Byte written by `0x00668a60` into the destination's presence flag.
pub const W13A07_PRESENCE_OFFSET: usize = 0x28;
/// Set when the lookup found a node.
pub const W13A07_PRESENCE_SET: u8 = 1;
/// Cleared when the lookup missed.
pub const W13A07_PRESENCE_CLEAR: u8 = 0;

/// `0x00668a60`: store the lookup presence flag.
///
/// ```c
/// FUN_00668f80(&local, key);
/// if (local[0] != 0) {
///   *(u8 *)(dest + 0x28) = 1;
///   FUN_0060ffb0(local[0] + 0x28);     // release the mapped string
///   return dest;
/// }
/// *(u8 *)(dest + 0x28) = 0;
/// return dest;
/// ```
///
/// Only the flag value is returned here; the release of the found value
/// (its `+0x28` word, driven by `FUN_0060ffb0`) is a side effect the caller
/// performs when it owns the node.
pub fn w13a07_presence_flag(found_node: Option<u32>) -> u8 {
    match found_node {
        Some(_) => W13A07_PRESENCE_SET,
        None => W13A07_PRESENCE_CLEAR,
    }
}

/// One node of the keyed bucket chain used by `0x00668ca0`.
///
/// The retail node stores the next pointer at `+0x00`, the key pointer at
/// `+0x08` and the key length at `+0x0c`.  Here the key is a borrowed byte
/// slice and the next node is an index into the node table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a07KeyNode<'a> {
    /// `+0x00`: next node in the same bucket chain.
    pub next: Option<usize>,
    /// `+0x08`/`+0x0c`: key bytes and their length.
    pub key: &'a [u8],
}

/// Where an iterator's predecessor link lives.
///
/// The retail `equal_range` result stores raw addresses: a bucket slot
/// address, the `next` word of a preceding node, or the table end address.
/// Indices keep the same three cases without expressing a pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a07Link {
    /// The link is the bucket-head slot with this index.
    Bucket(usize),
    /// The link is the `next` word of this node.
    Node(usize),
    /// The link is the table end (`bucket_count * 4`).
    End,
}

/// The `0x18`-byte result of `0x00668ca0`: two iterators sharing one end.
///
/// Each iterator is `{node, link}`; the retail record repeats the end
/// offset at `+0x08` and `+0x14`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a07EqualRange {
    /// First matching node, or `None` when the key is absent.
    pub first: Option<usize>,
    /// Link that points at [`Self::first`].
    pub first_link: W13a07Link,
    /// First node past the equal run, or `None` at the table end.
    pub last: Option<usize>,
    /// Link that points at [`Self::last`].
    pub last_link: W13a07Link,
    /// `bucket_count * 4`, stored at `+0x08` and `+0x14`.
    pub end: u32,
}

fn w13a07_empty_range(end: u32) -> W13a07EqualRange {
    W13a07EqualRange {
        first: None,
        first_link: W13a07Link::End,
        last: None,
        last_link: W13a07Link::End,
        end,
    }
}

/// `0x00668ca0`: the hash container's `equal_range(key)`.
///
/// ```c
/// if (table[1] == 0) {                  // no bucket base
///   out = {0, count << 2, count << 2, 0, count << 2, count << 2};
///   return;
/// }
/// hash = FUN_00c62800(key.ptr, key.len, 0);
/// slot = table[1] + (table[0] - 1 & hash) * 4;
/// link = FUN_005e8660(slot, &key);      // first match or last link
/// first = *link;
/// if (first == 0) { out = {0, end, end, 0, end, end}; return; }
/// out[0..1] = {first, link};
/// // second iterator: advance while the node key compares equal
/// out[2..3] = {stop, stop_link};
/// out[4] = end;
/// ```
///
/// Equality is the retail `memcmp(min(len))` followed by the length
/// difference, i.e. exact byte equality (including length).  `buckets`
/// holds the bucket-head node indices; `nodes[next]` follows the chain.
/// The scan past the end of one chain continues with the next non-empty
/// bucket slot, exactly like the container's iterator.
pub fn w13a07_equal_range(
    bucket_count: u32,
    buckets: &[Option<usize>],
    nodes: &[W13a07KeyNode<'_>],
    key: &[u8],
) -> W13a07EqualRange {
    let end = bucket_count.wrapping_mul(4);
    if bucket_count == 0 || buckets.is_empty() {
        return w13a07_empty_range(end);
    }
    debug_assert_eq!(bucket_count as usize, buckets.len());

    let hash = a43_hash_bytes(key);
    let mut slot = ((bucket_count - 1) & hash) as usize;
    let mut current = buckets[slot];
    let mut predecessor = W13a07Link::Bucket(slot);
    while let Some(index) = current {
        if nodes[index].key == key {
            break;
        }
        predecessor = W13a07Link::Node(index);
        current = nodes[index].next;
    }
    let Some(first) = current else {
        return w13a07_empty_range(end);
    };
    let first_link = predecessor;

    let mut last = first;
    let mut last_link;
    loop {
        let previous = last;
        if let Some(next) = nodes[previous].next {
            last = next;
            last_link = W13a07Link::Node(previous);
        } else {
            slot += 1;
            while slot < buckets.len() && buckets[slot].is_none() {
                slot += 1;
            }
            if slot == buckets.len() {
                return W13a07EqualRange {
                    first: Some(first),
                    first_link,
                    last: None,
                    last_link: W13a07Link::End,
                    end,
                };
            }
            last = buckets[slot].expect("non-empty bucket slot");
            last_link = W13a07Link::Bucket(slot);
        }
        if nodes[last].key != key {
            break;
        }
    }

    W13a07EqualRange {
        first: Some(first),
        first_link,
        last: Some(last),
        last_link,
        end,
    }
}

/// `0x00668dc0`: number of nodes in a key's equal range.
///
/// The retail row is the `_Hash::erase(key)` instantiation.  Its return
/// value is the number of nodes it unlinks, destroys (`node + 0x28` plus the
/// node itself) and subtracts from `table[2]`.  Only the count is modelled:
/// `w13a07_equal_range` supplies the first matching node and the walk to
/// the first non-equal node.  Every equal key hashes to the same bucket, so
/// the run never crosses into another bucket.
pub fn w13a07_equal_count(
    bucket_count: u32,
    buckets: &[Option<usize>],
    nodes: &[W13a07KeyNode<'_>],
    key: &[u8],
) -> u32 {
    let range = w13a07_equal_range(bucket_count, buckets, nodes, key);
    let Some(first) = range.first else {
        return 0;
    };
    let mut count = 0u32;
    let mut current = Some(first);
    while let Some(index) = current {
        if nodes[index].key != key {
            break;
        }
        count = count.wrapping_add(1);
        current = nodes[index].next;
    }
    count
}

/// The two values computed by the load-factor growth helper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a07HashGrowth {
    /// `max(ceil(elements / max_load), minimum)`.
    pub required: u32,
    /// Smallest power of two `>= 8` and `>= required`.
    pub buckets: u32,
}

/// `0x00669000`: compute the bucket count for a rehash.
///
/// ```c
/// d = (double)elements;                     // unsigned-to-double helper
/// need = (int)((float)d / max_load);        // truncation
/// if ((float)d <= (float)need * max_load) candidate = need;
/// else candidate = need + 1;                // ceil
/// if (candidate < minimum) candidate = minimum;
/// buckets = 8;
/// while (buckets < candidate) buckets *= 2;
/// if (buckets == table[0] && table[1] != 0) return;  // already sized
/// FUN_0093ec70();                           // rehash/allocate tail
/// ```
///
/// `table[2]` is the element count and `table[3]` the maximum load factor
/// (`f32`).  The comparison is the compiler's float `ceil` idiom: truncate
/// the quotient, then add one when the truncated product falls short.  The
/// rehash itself is an allocator/vtable tail and is not reproduced; the
/// returned pair is the input the tail receives.
pub fn w13a07_hash_growth(elements: u32, max_load: f32, minimum: u32) -> W13a07HashGrowth {
    let elements_f = elements as f32;
    let quotient = elements_f / max_load;
    let truncated = quotient as u32;
    let candidate = if elements_f <= truncated as f32 * max_load {
        truncated
    } else {
        truncated.wrapping_add(1)
    };
    let required = candidate.max(minimum);

    let mut buckets = W13A07_HASH_MIN_BUCKETS;
    while buckets < required {
        let next = buckets.wrapping_mul(2);
        if next == 0 {
            break;
        }
        buckets = next;
    }
    W13a07HashGrowth { required, buckets }
}

// ---------------------------------------------------------------------------
// Upload-info record (`ClientSdk::IInternetFileUploadInfoV1` family)
// ---------------------------------------------------------------------------

/// Size in bytes of the aggregate initialised by `0x00669490` (`0x38`).
pub const W13A07_UPLOAD_INFO_SIZE: usize = 0x38;
/// Number of 32-bit words in that aggregate.
pub const W13A07_UPLOAD_INFO_WORDS: usize = 0x0e;
/// Byte offset of the kind byte (`0x0c`).
pub const W13A07_UPLOAD_INFO_BYTE_KIND: usize = 0x0c;
/// Byte offset of the subtype byte (`0x0d`).
pub const W13A07_UPLOAD_INFO_BYTE_SUBTYPE: usize = 0x0d;
/// Byte offset of the 16-bit type word (`0x0e`).
pub const W13A07_UPLOAD_INFO_WORD_TYPE: usize = 0x0e;
/// Word offset of the stream word / child pointer (`0x10`).
pub const W13A07_UPLOAD_INFO_WORD_STREAM: usize = 0x10;
/// Word offset of the child-pointer alias (`0x14`).
pub const W13A07_UPLOAD_INFO_WORD_CHILD_ALIAS: usize = 0x14;
/// Word offset of the fourth constructor argument (`0x18`).
pub const W13A07_UPLOAD_INFO_WORD_ARG4: usize = 0x18;
/// Word offset of the third constructor argument (`0x2c`).
pub const W13A07_UPLOAD_INFO_WORD_ARG3: usize = 0x2c;
/// Size in bytes of the child allocation created by `0x00669490` (`0x14`).
pub const W13A07_UPLOAD_INFO_CHILD_SIZE: u32 = 0x14;
/// Number of words in the child allocation.
pub const W13A07_UPLOAD_INFO_CHILD_WORDS: usize = 5;

/// Kind value that also writes `0x0d` into the subtype byte.
pub const W13A07_UPLOAD_INFO_KIND_SPECIAL: u32 = 5;

/// The five-word child allocation made by `0x00669490`.
///
/// The words are `{0, 0x00010000, 0, 0, 0}`; retail allocates `0x14` bytes
/// and stores the pointer in both the stream word (`+0x10`) and the alias
/// word (`+0x14`) when the stream argument is zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a07UploadChild {
    /// The five words.
    pub words: [u32; W13A07_UPLOAD_INFO_CHILD_WORDS],
}

impl W13a07UploadChild {
    /// The recovered child contents.
    pub fn new() -> Self {
        Self {
            words: [0, 0x0001_0000, 0, 0, 0],
        }
    }
}

/// The `0x38`-byte aggregate built by `0x00669490`.
///
/// `words[3]` packs the kind byte, the subtype byte and the type word;
/// `words[4]`/`words[5]` are the stream word and its alias, and the word at
/// `+0x18` and `+0x2c` carry the fourth and third constructor arguments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a07UploadInfo {
    /// The fourteen record words.
    pub words: [u32; W13A07_UPLOAD_INFO_WORDS],
    /// Child allocation, present only on the zero-stream path.
    pub child: Option<W13a07UploadChild>,
}

/// `0x00669490`: construct the `0x38`-byte upload-info aggregate.
///
/// ```c
/// words[0..3] = 0;                      // 16 zero bytes
/// word[+0x0e] = table[kind];            // DAT_00d3dba4[kind]
/// if (kind == 5) byte[+0x0d] = 0x0d;
/// word[+0x10] = arg2;
/// word[+0x14] = 0;
/// word[+0x18] = arg4;
/// word[+0x1c..0x28] = 0;
/// word[+0x2c] = arg3;
/// word[+0x30] = word[+0x34] = 0;
/// if (word[+0x10] == 0) {
///   child = allocate(0x14);             // {0, 0x10000, 0, 0, 0}
///   word[+0x10] = word[+0x14] = child;
/// }
/// ```
///
/// `table_byte` is the 16-bit value the caller read from the recovered
/// `0x00d3dba4` table; that global's initialised contents are not part of
/// the shipped image copy available to this batch, so the lookup stays an
/// argument.  Ghidra split the child allocation into a separate row; it is
/// the tail of this constructor and is folded into this translation.
pub fn w13a07_upload_info_new(
    kind: u32,
    table_byte: u16,
    stream: u32,
    arg4: u32,
    arg3: u32,
) -> W13a07UploadInfo {
    let mut info = W13a07UploadInfo {
        words: [0; W13A07_UPLOAD_INFO_WORDS],
        child: None,
    };
    info.words[3] = (table_byte as u32) << 16;
    if kind == W13A07_UPLOAD_INFO_KIND_SPECIAL {
        info.words[3] |= (W13A07_UPLOAD_INFO_BYTE_SUBTYPE as u32) << 8;
    }
    info.words[W13A07_UPLOAD_INFO_WORD_STREAM / 4] = stream;
    info.words[W13A07_UPLOAD_INFO_WORD_ARG4 / 4] = arg4;
    info.words[W13A07_UPLOAD_INFO_WORD_ARG3 / 4] = arg3;
    if stream == 0 {
        info.child = Some(W13a07UploadChild::new());
    }
    info
}

/// `0x00669ab0`: move-assign one upload-info record.
///
/// ```c
/// dst[0..4] = src[0..4];                 // 16 bytes
/// *(u16 *)(src + 0x0e) = 0;              // clear the type word
/// FUN_0066a2b0(dst);                     // release dst's nested block
/// dst[+0x10] = src[+0x10];
/// dst[+0x14] = src[+0x14];
/// dst[+0x18..+0x2c] = src[+0x18..+0x2c];  // stolen nested block
/// dst[+0x30] = src[+0x30];
/// dst[+0x34] = src[+0x34];
/// src[+0x10] = src[+0x14] = 0;
/// src[+0x30] = src[+0x34] = 0;
/// ```
///
/// The nested block at `+0x18` is a 24-byte MSVC string plus two trailing
/// words; the retail move steals it without clearing the source's
/// `+0x18..+0x2c` span (only the source's `+0x30`/`+0x34` are cleared).
/// `release_dst_nested` stands in for `FUN_0066a2b0`, which frees the
/// destination's old block through the allocator singleton.
pub fn w13a07_move_upload_info(
    dst: &mut [u32; W13A07_UPLOAD_INFO_WORDS],
    src: &mut [u32; W13A07_UPLOAD_INFO_WORDS],
    release_dst_nested: impl FnOnce(&mut [u32; W13A07_UPLOAD_INFO_WORDS]),
) {
    for (index, word) in src.iter().take(4).enumerate() {
        dst[index] = *word;
    }
    src[3] &= 0xffff_0000;
    release_dst_nested(dst);
    dst[4] = src[4];
    dst[5] = src[5];
    dst[6..W13A07_UPLOAD_INFO_WORDS].copy_from_slice(&src[6..W13A07_UPLOAD_INFO_WORDS]);
    src[4] = 0;
    src[5] = 0;
    src[12] = 0;
    src[13] = 0;
}

// ---------------------------------------------------------------------------
// Type-erased callable slots (`std::function` storage)
// ---------------------------------------------------------------------------

/// Size of one callable slot (`0x14` bytes).
pub const W13A07_CALLABLE_SLOT_SIZE: usize = 0x14;
/// Word stored in a released/empty slot.
pub const W13A07_CALLABLE_EMPTY_WORD: u32 = 1;
/// Bit 0 of the storage word marks a heap pointer (`word & ~1`).
pub const W13A07_CALLABLE_HEAP_TAG: u32 = 1;

/// Decoded callable storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a07Callable {
    /// The slot is the empty word `1` (retail skips the destroy call).
    Empty,
    /// Bit 0 is clear: the target lives inline at the slot address.
    Inline,
    /// Bit 0 is set: the target is the pointer `word & ~1`.
    Heap(u32),
}

/// `0x006696e0` (and the slot pattern repeated by `0x00669800`): decode a
/// 20-byte callable slot's storage word.
///
/// ```c
/// word = slot[0];
/// if ((word & 1) == 0) target = slot;          // inline
/// else if ((word & ~1) != 0) target = word & ~1; // heap
/// else target = 0;                              // empty
/// ```
///
/// The retail check is on bit 0 only; the empty word `1` therefore maps to
/// the null target and is the only value for which the destroy call is
/// skipped.
pub fn w13a07_callable_storage(word: u32) -> W13a07Callable {
    if word & W13A07_CALLABLE_HEAP_TAG == 0 {
        W13a07Callable::Inline
    } else {
        let pointer = word & !W13A07_CALLABLE_HEAP_TAG;
        if pointer == 0 {
            W13a07Callable::Empty
        } else {
            W13a07Callable::Heap(pointer)
        }
    }
}

/// Release one callable slot and reset it to the empty word.
///
/// This is the shared destroy step of `0x006696e0` (three slots) and
/// `0x00669800` (the `+0x28` slot).
/// Returns the storage that was released, or `None` for an already empty
/// slot.  The retail body calls `target->vtable[0](target, 0)` and, for the
/// heap form, hands the pointer to the allocator singleton at
/// `FUN_0093ec70` (`vtable + 0x10`); those two calls stay with the caller.
pub fn w13a07_release_callable(word: &mut u32) -> Option<W13a07Callable> {
    let storage = w13a07_callable_storage(*word);
    if storage == W13a07Callable::Empty {
        return None;
    }
    *word = W13A07_CALLABLE_EMPTY_WORD;
    Some(storage)
}

/// The three callable slots released by `0x006696e0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a07CallableTriple {
    /// Slot at `+0x28`.
    pub at_28: u32,
    /// Slot at `+0x14`.
    pub at_14: u32,
    /// Slot at `+0x00`.
    pub at_00: u32,
}

/// `0x006696e0`: release the three callable slots of a manager object.
///
/// The order is `+0x28`, then `+0x14`, then `+0x00`, exactly the order the
/// retail body destroys them (each through the 20-byte slot protocol).
/// Every slot is left holding `1`.
pub fn w13a07_release_callable_triple(
    triple: &mut W13a07CallableTriple,
) -> [Option<W13a07Callable>; 3] {
    let first = w13a07_release_callable(&mut triple.at_28);
    let second = w13a07_release_callable(&mut triple.at_14);
    let third = w13a07_release_callable(&mut triple.at_00);
    [first, second, third]
}

/// Which manager function a callable uses.
///
/// The table at the frame local is exactly two entries: the CFG guard check
/// (`guard_check_icall`) and `0x00669430`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a07FunctionManager {
    /// Entry 0: the control-flow-guard check.
    Guard,
    /// Entry 1: the checked buffer release at `0x00669430`.
    Checked,
}

/// Decode the manager index stored at `+0x20` (the two-entry local table
/// used by `0x006698c0` and the tail of `0x00669800`).
///
/// The retail table dispatch accepts indices `0` and `1`; `-1` marks an
/// unset manager.  Other values have no table entry and are reported as
/// unsupported rather than read past the two-entry local table.
pub fn w13a07_function_manager(index: i32) -> Option<W13a07FunctionManager> {
    match index {
        0 => Some(W13a07FunctionManager::Guard),
        1 => Some(W13a07FunctionManager::Checked),
        _ => None,
    }
}

/// `0x006698c0`: dispatch and clear the manager index at `+0x20`.
///
/// ```c
/// if (*(int *)(this + 0x20) != -1) {
///   table = {guard_check_icall, FUN_00669430};
///   table[*(int *)(this + 0x20)](this);
///   *(int *)(this + 0x20) = -1;
/// }
/// ```
///
/// `0x00669800` contains the same tail after releasing its `+0x28` slot, so
/// both rows share this helper.  The index is cleared only when the table
/// actually has an entry.
pub fn w13a07_take_function_manager(index: &mut i32) -> Option<W13a07FunctionManager> {
    let manager = w13a07_function_manager(*index);
    if manager.is_some() {
        *index = -1;
    }
    manager
}

/// The actions recovered from the two scalar-deleting-destructor rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a07DeleteAction {
    /// Vtable word written before destruction, when the row writes one.
    pub vtable_write: Option<u32>,
    /// Whether a base destructor is called (`FUN_0083d770`).
    pub destroy: bool,
    /// Object size passed to the deallocator, when `flags & 1` is set.
    pub deallocate: Option<u32>,
}

/// Vtable stored by `0x00669930` and `0x00669bf0`.
pub const W13A07_UPLOAD_INFO_V1_VTABLE: u32 = 0x00d3_d9ac;

/// `0x00669930`: reset the four-byte `IInternetFileUploadInfoV1` object's
/// vtable word.
///
/// ```c
/// *(u32 *)this = ClientSdk::IInternetFileUploadInfoV1::vftable; // 0x00d3d9ac
/// ```
pub fn w13a07_upload_info_v1_init(object: &mut u32) {
    *object = W13A07_UPLOAD_INFO_V1_VTABLE;
}

/// `0x00669800`: release the `+0x28` callable slot, then dispatch and clear
/// the `+0x20` manager index.
///
/// The slot protocol is the same 20-byte storage handled by
/// [`w13a07_release_callable`]; the manager tail is
/// [`w13a07_take_function_manager`].
pub fn w13a07_release_upload_slot(
    slot: &mut u32,
    manager: &mut i32,
) -> (Option<W13a07Callable>, Option<W13a07FunctionManager>) {
    let released = w13a07_release_callable(slot);
    let taken = w13a07_take_function_manager(manager);
    (released, taken)
}

/// `0x00669430`: the checked-manager body.
///
/// ```c
/// if (*(int *)(record + 8) >= 0) {
///   allocator = FUN_0093ec70();
///   allocator->vtable[0x10](*(u32 *)record);   // deallocate
/// }
/// ```
///
/// The record's capacity word at `+0x8` decides whether the buffer word at
/// `+0x0` is owned.  The function returns the pointer to deallocate, or
/// `None` when the capacity is negative.  A null pointer is still reported
/// when the capacity allows it; the allocator tail is the caller's.
pub fn w13a07_checked_buffer_release(pointer: u32, capacity: i32) -> Option<u32> {
    if capacity >= 0 { Some(pointer) } else { None }
}

// ---------------------------------------------------------------------------
// Destructor / serial helpers
// ---------------------------------------------------------------------------

/// `0x00669bc0`: scalar deleting destructor of the `0xb8`-byte manager.
///
/// ```c
/// FUN_0083d770(this);                  // base destructor
/// if (flags & 1) FUN_00923e90(this, 0xb8);
/// return this;
/// ```
pub fn w13a07_upload_manager_deleting_destructor(flags: u8) -> W13a07DeleteAction {
    W13a07DeleteAction {
        vtable_write: None,
        destroy: true,
        deallocate: if flags & 1 != 0 { Some(0xb8) } else { None },
    }
}

/// `0x00669bf0`: scalar deleting destructor of the four-byte
/// `IInternetFileUploadInfoV1` object.
///
/// ```c
/// *(u32 *)this = 0x00d3d9ac;           // ClientSdk vftable
/// if (flags & 1) FUN_00923e90(this, 4);
/// return this;
/// ```
///
/// This row performs no base-destructor call; the vtable store is its only
/// destruction step.
pub fn w13a07_upload_info_v1_deleting_destructor(flags: u8) -> W13a07DeleteAction {
    W13a07DeleteAction {
        vtable_write: Some(W13A07_UPLOAD_INFO_V1_VTABLE),
        destroy: false,
        deallocate: if flags & 1 != 0 { Some(4) } else { None },
    }
}

/// `0x00669ce0`: allocate a serial number from the manager counter.
///
/// ```c
/// serial = *(int *)(this + 0x20);
/// *(int *)(this + 0x20) = serial + 1;
/// // insert serial into the id map, allocate a 0x28-byte node
/// // (id at +0x10, empty callable at +0x14), link it, then assign the
/// // callable argument through FUN_00534dc0 and return the old counter.
/// return serial;
/// ```
///
/// The map insertion and the 0x28-byte node are not modelled; the recovered
/// arithmetic is the wrapping post-increment of the id counter.
pub fn w13a07_next_serial(counter: &mut i32) -> i32 {
    let serial = *counter;
    *counter = serial.wrapping_add(1);
    serial
}

/// The handle and dispatch arguments recovered from `0x00669fd0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a07ChildTake {
    /// The child handle that was cleared from `+0xb4`.
    pub child: Option<u32>,
    /// Size passed to the deallocator for [`Self::child`] (`0xb8`).
    pub child_size: Option<u32>,
    /// The `FUN_0066af10` this-pointer and its two arguments.
    pub dispatch: (u32, u32, u32),
}

/// Size in bytes of the child object destroyed by `0x00669fd0`.
pub const W13A07_UPLOAD_CHILD_MANAGER_SIZE: u32 = 0xb8;

/// `0x00669fd0`: clear the nested child handle, then dispatch.
///
/// ```c
/// owner = *(u32 *)(this + 8);
/// child = *(u32 *)(owner + 0xb4);
/// *(u32 *)(owner + 0xb4) = 0;
/// if (child != 0) {
///   FUN_0083d770(child);               // destroy the child
///   FUN_00923e90(child, 0xb8);         // deallocate it
/// }
/// FUN_0066af10(*(u32 *)(this + 0x10), *(u32 *)(this + 0x14));
/// ```
///
/// `child_slot` is the `+0xb4` word; `owner`, `first` and `second` are the
/// `this` pointer and the two arguments the tail dispatch receives.
pub fn w13a07_take_upload_child(
    child_slot: &mut u32,
    owner: u32,
    first: u32,
    second: u32,
) -> W13a07ChildTake {
    let child = *child_slot;
    *child_slot = 0;
    if child == 0 {
        W13a07ChildTake {
            child: None,
            child_size: None,
            dispatch: (owner, first, second),
        }
    } else {
        W13a07ChildTake {
            child: Some(child),
            child_size: Some(W13A07_UPLOAD_CHILD_MANAGER_SIZE),
            dispatch: (owner, first, second),
        }
    }
}

#[cfg(test)]
mod tests_wave13_agent07 {
    use super::*;

    fn node<'a>(next: Option<usize>, key: &'a [u8]) -> W13a07KeyNode<'a> {
        W13a07KeyNode { next, key }
    }

    #[test]
    fn hash_kernel_values_used_by_the_bucket_tests() {
        // The shared MurmurHash3 kernel (`FUN_00c62800`, batch 43) must agree
        // with the constants the bucket masks below are derived from.
        assert_eq!(a43_hash_bytes(b""), 0x0000_0000);
        assert_eq!(a43_hash_bytes(b"key"), 0xe29c_6a44);
        assert_eq!(a43_hash_bytes(b"hello"), 0x248b_fa47);
        assert_eq!(a43_hash_bytes(b"abc"), 0xb3dd_93fa);
    }

    #[test]
    fn bucket_probe_masks_the_hash_and_reports_the_end() {
        assert_eq!(
            w13a07_hash_bucket_probe(8, true, b"", None),
            W13a07HashProbe {
                node: None,
                next_offset: 32,
                end_offset: 32
            }
        );
        // "key" hashes to 0xe29c6a44; the three low bits are 4.
        assert_eq!(
            w13a07_hash_bucket_probe(8, true, b"key", Some(0x1234)),
            W13a07HashProbe {
                node: Some(0x1234),
                next_offset: 16,
                end_offset: 32
            }
        );
        assert_eq!(
            w13a07_hash_bucket_probe(8, true, b"key", None),
            W13a07HashProbe {
                node: None,
                next_offset: 32,
                end_offset: 32
            }
        );
        // Without a bucket base the probe never hashes the key.
        assert_eq!(
            w13a07_hash_bucket_probe(8, false, b"key", Some(0x1234)),
            W13a07HashProbe {
                node: None,
                next_offset: 32,
                end_offset: 32
            }
        );
    }

    #[test]
    fn contains_is_the_probe_node_word() {
        assert!(w13a07_hash_contains(8, true, b"key", Some(7)));
        assert!(!w13a07_hash_contains(8, true, b"key", None));
        assert!(!w13a07_hash_contains(8, false, b"key", Some(7)));
    }

    #[test]
    fn presence_flag_sets_one_only_on_a_hit() {
        assert_eq!(w13a07_presence_flag(Some(0x40)), 1);
        assert_eq!(w13a07_presence_flag(None), 0);
        assert_eq!(W13A07_PRESENCE_OFFSET, 0x28);
    }

    #[test]
    fn equal_range_returns_the_matching_run() {
        let nodes = [
            node(Some(1), b"alpha"),
            node(Some(2), b"key"),
            node(Some(3), b"key"),
            node(Some(4), b"kex"),
            node(None, b"key"),
        ];
        let buckets = [Some(0), None, None, None];
        let range = w13a07_equal_range(4, &buckets, &nodes, b"key");
        assert_eq!(range.first, Some(1));
        // The bucket head is "alpha", so the first match hangs off node 0.
        assert_eq!(range.first_link, W13a07Link::Node(0));
        assert_eq!(range.last, Some(3));
        assert_eq!(range.last_link, W13a07Link::Node(2));
        assert_eq!(range.end, 16);
        assert_eq!(w13a07_equal_count(4, &buckets, &nodes, b"key"), 2);
    }

    #[test]
    fn equal_range_misses_and_empty_tables() {
        let nodes = [node(None, b"alpha")];
        let buckets = [Some(0), None, None, None];
        let range = w13a07_equal_range(4, &buckets, &nodes, b"key");
        assert_eq!(range.first, None);
        assert_eq!(range.first_link, W13a07Link::End);
        assert_eq!(range.last, None);
        assert_eq!(w13a07_equal_count(4, &buckets, &nodes, b"key"), 0);

        let empty = w13a07_equal_range(4, &[], &nodes, b"key");
        assert_eq!(empty.first, None);
        assert_eq!(empty.end, 16);
    }

    #[test]
    fn equal_range_advances_to_the_next_bucket() {
        // "key" maps to bucket 0 and "hello" (odd hash) to bucket 1.
        let nodes = [node(None, b"key"), node(None, b"hello")];
        let buckets = [Some(0), Some(1)];
        let range = w13a07_equal_range(2, &buckets, &nodes, b"key");
        assert_eq!(range.first, Some(0));
        assert_eq!(range.first_link, W13a07Link::Bucket(0));
        assert_eq!(range.last, Some(1));
        assert_eq!(range.last_link, W13a07Link::Bucket(1));
        assert_eq!(w13a07_equal_count(2, &buckets, &nodes, b"key"), 1);
    }

    #[test]
    fn equal_count_walks_a_full_bucket() {
        let nodes = [
            node(Some(1), b"key"),
            node(Some(2), b"key"),
            node(None, b"key"),
        ];
        let buckets = [Some(0), None, None, None];
        assert_eq!(w13a07_equal_count(4, &buckets, &nodes, b"key"), 3);
    }

    #[test]
    fn growth_uses_ceil_and_the_power_of_two_ladder() {
        assert_eq!(
            w13a07_hash_growth(0, 1.0, 0),
            W13a07HashGrowth {
                required: 0,
                buckets: 8
            }
        );
        assert_eq!(
            w13a07_hash_growth(8, 1.0, 0),
            W13a07HashGrowth {
                required: 8,
                buckets: 8
            }
        );
        assert_eq!(
            w13a07_hash_growth(9, 1.0, 0),
            W13a07HashGrowth {
                required: 9,
                buckets: 16
            }
        );
        assert_eq!(
            w13a07_hash_growth(15, 2.0, 0),
            W13a07HashGrowth {
                required: 8,
                buckets: 8
            }
        );
        assert_eq!(
            w13a07_hash_growth(16, 2.0, 0),
            W13a07HashGrowth {
                required: 8,
                buckets: 8
            }
        );
        assert_eq!(
            w13a07_hash_growth(10, 0.75, 0),
            W13a07HashGrowth {
                required: 14,
                buckets: 16
            }
        );
        assert_eq!(
            w13a07_hash_growth(1, 1.0, 20),
            W13a07HashGrowth {
                required: 20,
                buckets: 32
            }
        );
        assert_eq!(w13a07_hash_growth(1, 1.0, 9).buckets, 16);
    }

    #[test]
    fn upload_info_constructor_writes_the_recovered_words() {
        let info = w13a07_upload_info_new(3, 0x1234, 0x55, 0x77, 0x99);
        assert!(info.child.is_none());
        assert_eq!(info.words[0], 0);
        assert_eq!(info.words[1], 0);
        assert_eq!(info.words[2], 0);
        assert_eq!(info.words[3], 0x1234_0000);
        assert_eq!(info.words[W13A07_UPLOAD_INFO_WORD_STREAM / 4], 0x55);
        assert_eq!(info.words[W13A07_UPLOAD_INFO_WORD_CHILD_ALIAS / 4], 0);
        assert_eq!(info.words[W13A07_UPLOAD_INFO_WORD_ARG4 / 4], 0x77);
        assert_eq!(&info.words[7..11], &[0, 0, 0, 0]);
        assert_eq!(info.words[W13A07_UPLOAD_INFO_WORD_ARG3 / 4], 0x99);
        assert_eq!(info.words[12], 0);
        assert_eq!(info.words[13], 0);
    }

    #[test]
    fn upload_info_constructor_special_kind_and_child_path() {
        let info = w13a07_upload_info_new(5, 0x0203, 0, 0, 0);
        assert_eq!(info.words[3], 0x0203_0d00);
        assert_eq!(info.words[W13A07_UPLOAD_INFO_WORD_STREAM / 4], 0);
        let child = info.child.expect("zero stream allocates the child");
        assert_eq!(child.words, [0, 0x0001_0000, 0, 0, 0]);
        assert_eq!(W13A07_UPLOAD_INFO_CHILD_SIZE, 0x14);
    }

    #[test]
    fn callable_slots_decode_inline_heap_and_empty() {
        assert_eq!(w13a07_callable_storage(1), W13a07Callable::Empty);
        assert_eq!(w13a07_callable_storage(0), W13a07Callable::Inline);
        assert_eq!(
            w13a07_callable_storage(0x1235),
            W13a07Callable::Heap(0x1234)
        );
        assert_eq!(
            w13a07_callable_storage(0x8000_0001),
            W13a07Callable::Heap(0x8000_0000)
        );
    }

    #[test]
    fn release_callable_resets_to_the_empty_word() {
        let mut heap = 0x1235;
        assert_eq!(
            w13a07_release_callable(&mut heap),
            Some(W13a07Callable::Heap(0x1234))
        );
        assert_eq!(heap, W13A07_CALLABLE_EMPTY_WORD);

        let mut inline = 0;
        assert_eq!(
            w13a07_release_callable(&mut inline),
            Some(W13a07Callable::Inline)
        );
        assert_eq!(inline, W13A07_CALLABLE_EMPTY_WORD);

        let mut empty = W13A07_CALLABLE_EMPTY_WORD;
        assert_eq!(w13a07_release_callable(&mut empty), None);
        assert_eq!(empty, W13A07_CALLABLE_EMPTY_WORD);
    }

    #[test]
    fn release_triple_follows_the_retail_order() {
        let mut triple = W13a07CallableTriple {
            at_28: 0x11,
            at_14: 0x15,
            at_00: 0,
        };
        let released = w13a07_release_callable_triple(&mut triple);
        assert_eq!(
            released,
            [
                Some(W13a07Callable::Heap(0x10)),
                Some(W13a07Callable::Heap(0x14)),
                Some(W13a07Callable::Inline),
            ]
        );
        assert_eq!(
            triple,
            W13a07CallableTriple {
                at_28: 1,
                at_14: 1,
                at_00: 1
            }
        );
    }

    #[test]
    fn manager_indices_map_to_the_two_entry_table() {
        assert_eq!(
            w13a07_function_manager(0),
            Some(W13a07FunctionManager::Guard)
        );
        assert_eq!(
            w13a07_function_manager(1),
            Some(W13a07FunctionManager::Checked)
        );
        assert_eq!(w13a07_function_manager(-1), None);
        assert_eq!(w13a07_function_manager(2), None);
    }

    #[test]
    fn take_manager_clears_only_known_entries() {
        let mut index = 1;
        assert_eq!(
            w13a07_take_function_manager(&mut index),
            Some(W13a07FunctionManager::Checked)
        );
        assert_eq!(index, -1);

        let mut unset = -1;
        assert_eq!(w13a07_take_function_manager(&mut unset), None);
        assert_eq!(unset, -1);
    }

    #[test]
    fn release_upload_slot_does_both_steps() {
        let mut slot = 0x21;
        let mut manager = 0;
        let (released, taken) = w13a07_release_upload_slot(&mut slot, &mut manager);
        assert_eq!(released, Some(W13a07Callable::Heap(0x20)));
        assert_eq!(taken, Some(W13a07FunctionManager::Guard));
        assert_eq!(slot, 1);
        assert_eq!(manager, -1);
    }

    #[test]
    fn checked_buffer_release_tests_the_capacity_sign() {
        assert_eq!(w13a07_checked_buffer_release(0x1000, 0), Some(0x1000));
        assert_eq!(w13a07_checked_buffer_release(0x1000, 15), Some(0x1000));
        assert_eq!(w13a07_checked_buffer_release(0x1000, -1), None);
        assert_eq!(w13a07_checked_buffer_release(0, 0), Some(0));
    }

    #[test]
    fn v1_vtable_store_uses_the_recovered_constant() {
        let mut object = 0;
        w13a07_upload_info_v1_init(&mut object);
        assert_eq!(object, 0x00d3_d9ac);
        assert_eq!(object, W13A07_UPLOAD_INFO_V1_VTABLE);
    }

    #[test]
    fn move_upload_info_copies_and_clears() {
        let mut dst = [0u32; W13A07_UPLOAD_INFO_WORDS];
        let mut src = [0u32; W13A07_UPLOAD_INFO_WORDS];
        for (index, word) in dst.iter_mut().enumerate() {
            *word = 0x1000 + index as u32;
        }
        for (index, word) in src.iter_mut().enumerate() {
            *word = 0x2000 + index as u32;
        }
        src[3] = 0x00aa_bbcc;
        let mut released_tail = None;
        w13a07_move_upload_info(&mut dst, &mut src, |destination| {
            // The nested-block release sees the freshly copied head words
            // and the destination's old tail.
            released_tail = Some((destination[0], destination[13]));
        });
        assert_eq!(released_tail, Some((0x2000, 0x100d)));
        // The head is copied before the source type word is cleared.
        assert_eq!(&dst[..3], &src[..3]);
        assert_eq!(dst[3], 0x00aa_bbcc);
        assert_eq!(dst[4], 0x2004);
        assert_eq!(dst[5], 0x2005);
        assert_eq!(&dst[6..12], &src[6..12]);
        assert_eq!(dst[12], 0x200c);
        assert_eq!(dst[13], 0x200d);
        // The source's type word is cleared and so are +0x10/+0x14 and the
        // two trailing words; the stolen nested block stays aliased.
        assert_eq!(src[3], 0x00aa_0000);
        assert_eq!(src[4], 0);
        assert_eq!(src[5], 0);
        assert_eq!(src[6], 0x2006);
        assert_eq!(src[12], 0);
        assert_eq!(src[13], 0);
    }

    #[test]
    fn deleting_destructors_report_their_sizes() {
        assert_eq!(
            w13a07_upload_manager_deleting_destructor(0),
            W13a07DeleteAction {
                vtable_write: None,
                destroy: true,
                deallocate: None
            }
        );
        assert_eq!(
            w13a07_upload_manager_deleting_destructor(1),
            W13a07DeleteAction {
                vtable_write: None,
                destroy: true,
                deallocate: Some(0xb8)
            }
        );
        assert_eq!(
            w13a07_upload_manager_deleting_destructor(0xff).deallocate,
            Some(0xb8)
        );
        assert_eq!(
            w13a07_upload_info_v1_deleting_destructor(0),
            W13a07DeleteAction {
                vtable_write: Some(0x00d3_d9ac),
                destroy: false,
                deallocate: None
            }
        );
        assert_eq!(
            w13a07_upload_info_v1_deleting_destructor(1).deallocate,
            Some(4)
        );
    }

    #[test]
    fn serial_allocation_is_a_wrapping_post_increment() {
        let mut counter = 41;
        assert_eq!(w13a07_next_serial(&mut counter), 41);
        assert_eq!(counter, 42);
        let mut wrap = i32::MAX;
        assert_eq!(w13a07_next_serial(&mut wrap), i32::MAX);
        assert_eq!(wrap, i32::MIN);
    }

    #[test]
    fn take_upload_child_clears_the_slot_and_reports_the_dispatch() {
        let mut child = 0x9000;
        let taken = w13a07_take_upload_child(&mut child, 0x100, 0x200, 0x300);
        assert_eq!(child, 0);
        assert_eq!(
            taken,
            W13a07ChildTake {
                child: Some(0x9000),
                child_size: Some(0xb8),
                dispatch: (0x100, 0x200, 0x300)
            }
        );

        let mut empty = 0;
        let taken = w13a07_take_upload_child(&mut empty, 1, 2, 3);
        assert_eq!(
            taken,
            W13a07ChildTake {
                child: None,
                child_size: None,
                dispatch: (1, 2, 3)
            }
        );
    }
}
