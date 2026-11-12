#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent74.rs
// ===========================================================================
// Translation batch 74.  Only this file is edited by the batch.
//
// Batch 74 owns `0x00626000..=0x00627fff`.  The ledger lists 46 rows in that
// window: 45 `decompiled_uncited` and one `decompile_failed`.  Thirty-nine
// of the 45 are translated below; the six that are not are listed at the end
// with their reason.
//
// The window is not a sequence of functions.  A linear disassembly of the
// shipped image (`sc32_runtime_pe.bin`, `.text` at `0x00401000`) shows that
// most ledger rows are interior blocks of large, heavily inlined C++
// routines.  Every translation below states which row it covers and, where
// the row is interior, which enclosing routine it belongs to:
//
// | enclosing routine | extent | ledger rows |
// |-------------------|--------|-------------|
// | minimap-cache map insert | `0x00626400..=0x00626bff` | `0x00626400`, `0x0062646d`, `0x00626490`, `0x006264c4`, `0x00626610`, `0x0062670a`, `0x0062672a`, `0x0062692c`, `0x00626a5e`, `0x00626ae4`, `0x00626b94` |
// | cache-key map destructor | `0x00626c10..=0x00626cdc` | `0x00626c10`, `0x00626c77`, `0x00626c94`, `0x00626cbc` |
// | 4-byte-key map insert | `0x00626d90..=0x00626f54` | `0x00626d90`, `0x00626f42` |
// | `Skin` copy constructor | `0x00627210..=0x0062751d` | `0x00627210`, `0x006272d0`, `0x006272e3`, `0x00627465`, `0x006274d5` |
// | `Skin` destructor | `0x006275b0..=0x006276b9` | `0x006275b0`, `0x0062760e`, `0x00627645`, `0x00627664`, `0x00627683`, `0x006276a1` |
//
// The routine at `0x00626400` is the minimap pixel cacher.  Three `0x78`-byte
// records at `0x00fc9b88` are probed with argument `0x0b`; each accepted
// record runs eight map inserts keyed by the pair at record `+0x44` / `+0x48`.
// The key is a `0x1c`-byte small-string record; the mapped value is a
// `PixelCache` with vftable `0x00d3b47c`, eight buckets and a `1.0` maximum
// load factor.
//
// Recovered layouts:
//
// | layout | words |
// |--------|-------|
// | small-string record (`0x1c`) | `+0x00` data, `+0x04` length, `+0x08` capacity word (`0x8000000f` = inline), `+0x0c..=+0x1b` inline bytes |
// | cache node (`0x3c`) | `+0x00` next, `+0x04` key string, `+0x20` `PixelCache` |
// | `PixelCache` (`0x1c`) | `+0x00` vftable `0x00d3b47c`, `+0x04` prototype word, `+0x08`/`+0x09` bytes, `+0x0c` bucket count `8`, `+0x10` count, `+0x14` zero, `+0x18` max-load `1.0f` |
// | 4-byte-key node (`0x14`) | `+0x00` next, `+0x04` key, `+0x08` data, `+0x0c` count, `+0x10` capacity word (`count & 0x7fffffff`) |
// | 12-byte vector | `+0x30` data, `+0x34` size, `+0x38` capacity word (top bit preserved separately) |
// | `Skin` | four `0x1c` strings at `+0x00/+0x1c/+0x38/+0x54`, word `+0x70`, tagged pointer `+0x74`, `0x1c`-string vector `+0x88/+0x8c/+0x90` |
//
// Address to function:
//
// * `0x00626400` [`a74_cache_record_offsets`]
// * `0x0062646d` [`a74_cache_record_key`]
// * `0x00626490` [`A74Sso::a74_new`]
// * `0x006264c4` [`a74_chain_find`]
// * `0x00626610` [`A74PixelCache::a74_new`], [`a74_load_factor_exceeded`]
// * `0x0062670a`, `0x00626c77` [`a74_release_heap_string`]
// * `0x0062672a` [`a74_cache_thread_name`], [`a74_cache_thread_header`],
//   [`a74_copy_cache_thread_name`]
// * `0x0062692c` [`a74_vec12_growth`]
// * `0x00626a5e` [`a74_vec12_relocate`]
// * `0x00626ae4` [`a74_vec12_append`]
// * `0x00626b94` [`a74_inner_slot_next`]
// * `0x00626c10` [`a74_destroy_order`]
// * `0x00626c94` [`a74_chain_advance`]
// * `0x00626cbc` [`a74_clear_bucket_word`]
// * `0x00626ce0` [`a74_forward_plan`]
// * `0x00626d20` [`a74_hash_contains`]
// * `0x00626d90` [`a74_hash_word`], [`A74U32Map::a74_insert`]
// * `0x00626f42` [`a74_insert_result`]
// * `0x00626f60` [`a74_vftable_lambda_1dd056d5`]
// * `0x00626f70` [`a74_rtti_lambda_1dd056d5`]
// * `0x00626f85` [`a74_song_tail_fields`]
// * `0x00627030` [`a74_song_function_object`]
// * `0x00627050` [`a74_rtti_song_factory`]
// * `0x00627070` [`a74_vftable_lambda_699267ad`]
// * `0x00627210` [`A74Sso::a74_assign_inline`]
// * `0x006272d0`, `0x006274d5` [`A74Sso::a74_terminate_at`]
// * `0x006272e3` [`a74_skin_string_slots`]
// * `0x00627465` [`a74_string_vector_bytes`]
// * `0x00627520` [`a74_tagged_target`]
// * `0x00627575` [`a74_tagged_released_word`]
// * `0x006275b0` [`a74_skin_dtor_plan`]
// * `0x0062760e`, `0x00627645`, `0x00627664`, `0x00627683`, `0x006276a1`
//   [`a74_skin_dtor_slots_from_5c`], [`a74_skin_dtor_slots_from_40`],
//   [`a74_skin_dtor_slots_from_24`], [`a74_skin_dtor_slots_from_08`],
//   [`a74_skin_dtor_slots_from_none`]
// * `0x00627780` [`a74_option_callback_plan`]
//
// Everything is a pure free function or a value type owned by this module:
// the batch adds no fields to `SimUnit` or `World` and calls no helper that
// is not implemented here or in another translated batch.

// ---------------------------------------------------------------------------
// Minimap-cache scan: 0x00626400 and 0x0062646d
// ---------------------------------------------------------------------------

/// Base of the three `0x78`-byte cache-skin records probed by `0x00626400`.
pub const A74_CACHE_RECORD_BASE: u32 = 0x00fc_9b88;
/// Record stride: `add eax,0x78` at `0x00626bca`.
pub const A74_CACHE_RECORD_STRIDE: u32 = 0x78;
/// Total span checked by the outer loop (`cmp esi,0x168`).
pub const A74_CACHE_RECORD_BYTES: u32 = 0x168;
/// Predicate argument pushed at `0x00626452`.
pub const A74_CACHE_PREDICATE_ARG: u32 = 0x0b;
/// Inserts performed for each accepted record (`cmp eax,0x8`).
pub const A74_CACHE_INSERTIONS_PER_RECORD: u32 = 8;
/// The key pointer word inside a cache record (`mov eax,[eax+0x44]`).
pub const A74_CACHE_KEY_PTR_OFFSET: u32 = 0x44;
/// The key length word inside a cache record (`mov ecx,[eax+0x48]`).
pub const A74_CACHE_KEY_LEN_OFFSET: u32 = 0x48;

/// `0x00626400`: the three record addresses visited by the outer scan.
///
/// The shipped loop starts `esi = 0`, `record = 0x00fc9b88`, calls the
/// predicate with `(record, 0x0b)`, then adds `0x78` to both the byte counter
/// and the record pointer while `counter < 0x168`.  Three records follow; the
/// scan is reproduced as its address table, because the predicate
/// (`FUN_00649280`) and the eight-slot insert body that follows an accepted
/// record are outside this block.
pub fn a74_cache_record_offsets() -> [u32; 3] {
    [
        A74_CACHE_RECORD_BASE,
        A74_CACHE_RECORD_BASE + A74_CACHE_RECORD_STRIDE,
        A74_CACHE_RECORD_BASE + 2 * A74_CACHE_RECORD_STRIDE,
    ]
}

/// `0x0062646d`: read the key pair of one cache record.
///
/// `0x0062646d` is the second block of `0x00626400`.  It loads the record
/// pointer from `[ebp-0x174]`, reads `+0x48` (length) into `ecx` and `+0x44`
/// (data pointer) into `eax`, stores both into the frame, and reserves the
/// `0x1c`-byte temporary string record (`sub esp,0x1c`).  The returned pair is
/// `(data, length)` in that order.
pub fn a74_cache_record_key(record: &[u8; A74_CACHE_RECORD_STRIDE as usize]) -> (u32, u32) {
    let data = u32::from_le_bytes(
        record[A74_CACHE_KEY_PTR_OFFSET as usize..A74_CACHE_KEY_PTR_OFFSET as usize + 4]
            .try_into()
            .unwrap(),
    );
    let len = u32::from_le_bytes(
        record[A74_CACHE_KEY_LEN_OFFSET as usize..A74_CACHE_KEY_LEN_OFFSET as usize + 4]
            .try_into()
            .unwrap(),
    );
    (data, len)
}

// ---------------------------------------------------------------------------
// The 0x1c-byte small-string record: 0x00626490, 0x00627210, 0x006272d0,
// 0x006274d5, 0x0062670a and 0x00626c77
// ---------------------------------------------------------------------------

/// Inline capacity word: `0x8000000f` is fifteen usable bytes plus the top
/// "small" bit.  Written by `0x00626490`, `0x00627210`, `0x006272e3`,
/// `0x00627465` and `0x006274d5`.
pub const A74_SSO_CAPACITY_INLINE: u32 = 0x8000_000f;
/// Number of inline bytes in the record (`+0x0c..=+0x1b`).
pub const A74_SSO_INLINE_BYTES: usize = 16;
/// Record-relative offset of the inline buffer (`lea eax,[esi+0xc]`).
pub const A74_SSO_INLINE_OFFSET: u32 = 0x0c;
/// Bytes reserved for one temporary key record (`sub esp,0x1c`).
pub const A74_SSO_RECORD_BYTES: u32 = 0x1c;

/// The `0x1c`-byte small-string record used for map keys and `Skin` fields.
///
/// The shipped record is `{data, length, capacity, inline[16]}`.  `data` is a
/// real address in the shipped image; here an inline record stores the
/// record-relative offset [`A74_SSO_INLINE_OFFSET`] instead, which keeps the
/// store sequence `[esi] = esi+0xc`, `[esi+4] = 0`, `[esi+8] = 0x8000000f`,
/// `byte [esi+0xc] = 0` exactly reproducible without inventing an allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A74Sso {
    /// `+0x00`: data word.
    pub data: u32,
    /// `+0x04`: byte length.
    pub len: u32,
    /// `+0x08`: capacity word; a negative value marks the inline form.
    pub cap: u32,
    /// `+0x0c..=+0x1b`: inline bytes.
    pub inline: [u8; A74_SSO_INLINE_BYTES],
}

/// Result of [`A74Sso::a74_assign_inline`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A74SsoAssign {
    /// The source fitted in the inline buffer; the terminator is at `len`.
    Inline {
        /// Byte index of the terminator, equal to the source length.
        terminator_at: u32,
    },
    /// The source needs the heap path (`FUN_00538810`/`FUN_00c89580`); the
    /// inline record is left untouched.
    NeedsHeap {
        /// Source length that has to be allocated.
        len: u32,
    },
}

impl A74Sso {
    /// `0x00626490`: construct the empty inline record.
    ///
    /// The row is `[esi+4] = 0; [esi] = eax; [esi+8] = 0x8000000f;
    /// byte [eax] = 0` where `eax` is the inline buffer (`esi + 0xc`).  The
    /// same four stores open `0x00627210` (string at `+0x00`) and `0x006272e3`
    /// (string at `+0x38`, buffer at `+0x44`).
    pub fn a74_new() -> Self {
        A74Sso {
            data: A74_SSO_INLINE_OFFSET,
            len: 0,
            cap: A74_SSO_CAPACITY_INLINE,
            inline: [0; A74_SSO_INLINE_BYTES],
        }
    }

    /// Whether the capacity word marks the inline form (`cap < 0`, signed).
    ///
    /// Every release site tests the capacity word with `cmp ...,0; jl`, so the
    /// test is signed: `0x8000000f` and `0xffffffff` are both "inline".
    pub fn a74_is_inline(&self) -> bool {
        (self.cap as i32) < 0
    }

    /// The pointer word a destructor would free, or `None` for an inline
    /// record.  This is the `0x0062670a` / `0x00626c77` decision.
    pub fn a74_heap_ptr(&self) -> Option<u32> {
        if self.a74_is_inline() {
            None
        } else {
            Some(self.data)
        }
    }

    /// `0x00627210` (and the first string of the `Skin` copy constructor):
    /// initialise the record and copy an inline source.
    ///
    /// `0x00627210` writes the four empty-record stores, calls the assign
    /// helper `FUN_00538810(data, src, len)`, then stores the terminator at
    /// `data[len]` and `len` at `+0x04` (`0x006272d0` is exactly that tail and
    /// `0x006274d5` is the same tail inside the `0x1c`-element loop).  Sources
    /// of at most fifteen bytes stay inline and are reproduced byte for byte.
    /// Longer sources enter the helper's allocation path; that path is outside
    /// the row, so it is reported as [`A74SsoAssign::NeedsHeap`].
    pub fn a74_assign_inline(&mut self, src: &[u8]) -> A74SsoAssign {
        if src.len() > A74_SSO_INLINE_BYTES - 1 {
            return A74SsoAssign::NeedsHeap {
                len: src.len() as u32,
            };
        }
        self.inline = [0; A74_SSO_INLINE_BYTES];
        self.inline[..src.len()].copy_from_slice(src);
        self.data = A74_SSO_INLINE_OFFSET;
        self.len = src.len() as u32;
        self.cap = A74_SSO_CAPACITY_INLINE;
        A74SsoAssign::Inline {
            terminator_at: src.len() as u32,
        }
    }

    /// `0x006272d0` / `0x006274d5`: write the terminator and the length.
    ///
    /// `mov byte [ecx+eax],0; mov [edi+0x20],eax` with `ecx` the data word and
    /// `eax` the length.  Returns `false` when `len` is outside the inline
    /// buffer (the heap form's pointer is not modelled).
    pub fn a74_terminate_at(&mut self, len: u32) -> bool {
        if len as usize >= A74_SSO_INLINE_BYTES {
            return false;
        }
        self.inline[len as usize] = 0;
        self.len = len;
        true
    }
}

/// `0x0062670a`, `0x00626c77`: the pointer word after a key release.
///
/// Both rows test the string's capacity word and, for the heap form
/// (`cap >= 0` signed), call the allocator's virtual free on the data word and
/// then store zero there.  For the inline form the data word is untouched.
/// The returned word is the value stored back at the pointer field.
pub fn a74_release_heap_string(ptr: u32, cap: u32) -> u32 {
    if (cap as i32) < 0 { ptr } else { 0 }
}

// ---------------------------------------------------------------------------
// "Minimap cacher" thread-name block: 0x0062672a
// ---------------------------------------------------------------------------

/// The literal copied by the `0x0062672a` block (`mov esi,0xd3b488`).
pub const A74_CACHE_THREAD_NAME: &str = "Minimap cacher";
/// Address of the literal in `.rdata`.
pub const A74_CACHE_THREAD_NAME_LITERAL: u32 = 0x00d3_b488;
/// The header word at `[ebp-0x38]` (`0x100000`).
pub const A74_CACHE_THREAD_CAPACITY: u32 = 0x0010_0000;
/// The header word at `[ebp-0x30]` (`4`).
pub const A74_CACHE_THREAD_KIND: u32 = 4;
/// The copy loop's bound: `cmp edx,0x1f; jge done`, so at most 31 bytes.
pub const A74_CACHE_THREAD_NAME_MAX: usize = 0x1f;

/// `0x0062672a`: the thread-name literal and its shipped address.
///
/// `lea ecx,[ebp-0x58]` is the destination buffer, `mov esi,0xd3b488` the
/// literal and `esi -= eax` the offset trick used by the copy loop below.
/// This row also contains a pending-node free and a walk of an already-built
/// bucket table (`0x00626761` onward).  That path re-enters `FUN_00626d90`
/// with a 4-byte key, while the cache map's value is a `PixelCache` and not
/// the 4-byte-key map, so the owning container cannot be established from the
/// row and only the name block is reproduced.
pub fn a74_cache_thread_name() -> (&'static str, u32) {
    (A74_CACHE_THREAD_NAME, A74_CACHE_THREAD_NAME_LITERAL)
}

/// `0x0062672a`: the three-word header stored next to the name buffer.
///
/// ```text
/// [ebp-0x38] = 0x100000
/// [ebp-0x34] = 0
/// [ebp-0x30] = 4
/// ```
pub fn a74_cache_thread_header() -> (u32, u32, u32) {
    (A74_CACHE_THREAD_CAPACITY, 0, A74_CACHE_THREAD_KIND)
}

/// `0x0062672a`: the bounded literal copy (`0x00626880..=0x00626892`).
///
/// ```text
/// i = 0
/// do {
///     if (i >= 0x1f) break;
///     byte [dst + i] = byte [literal + i];
///     i += 1;
/// } while (byte [literal + i] != 0);
/// byte [dst + i] = 0;
/// ```
///
/// The first byte is copied unconditionally, so an empty literal still stores
/// one byte.  Returns the terminator index (`i`).
pub fn a74_copy_cache_thread_name(dst: &mut [u8; 0x20]) -> u32 {
    let src = A74_CACHE_THREAD_NAME.as_bytes();
    let mut i = 0usize;
    loop {
        if i >= A74_CACHE_THREAD_NAME_MAX {
            break;
        }
        dst[i] = src.get(i).copied().unwrap_or(0);
        i += 1;
        if src.get(i).copied().unwrap_or(0) == 0 {
            break;
        }
    }
    dst[i] = 0;
    i as u32
}

// ---------------------------------------------------------------------------
// PixelCache value: 0x00626610
// ---------------------------------------------------------------------------

/// `PixelCache::vftable` stored at value `+0x00` (`mov [esi+0x20],
/// 0xd3b47c`).
pub const A74_PIXEL_CACHE_VTABLE: u32 = 0x00d3_b47c;
/// Default bucket count written to value `+0x0c` (`mov [esi+0x2c],8`).
pub const A74_PIXEL_CACHE_BUCKETS: u32 = 8;
/// Default maximum load factor `0x3f800000` (1.0f).
pub const A74_PIXEL_CACHE_LOAD_FACTOR: f32 = 1.0;

/// The `0x1c`-byte `PixelCache` value stored at cache node `+0x20`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A74PixelCache {
    /// `+0x00`: vftable, always [`A74_PIXEL_CACHE_VTABLE`].
    pub vtable: u32,
    /// `+0x04`: prototype word copied from the map (`[edi+0x14]`).
    pub prototype_word: u32,
    /// `+0x08`: byte copied from the map (`[edi+0x18]`).
    pub byte_18: u8,
    /// `+0x09`: byte copied from the map (`[edi+0x19]`).
    pub byte_19: u8,
    /// `+0x0c`: bucket count, initialised to eight.
    pub bucket_count: u32,
    /// `+0x10`: element count, initialised to zero.
    pub size: u32,
    /// `+0x14`: zeroed word.
    pub zero_14: u32,
    /// `+0x18`: maximum load factor, initialised to `1.0f`.
    pub max_load: f32,
}

impl A74PixelCache {
    /// `0x00626610`: build the value from the three prototype words.
    ///
    /// The row, after the inlined `unordered_map` insert, runs:
    ///
    /// ```text
    /// [esi+0x20] = 0x00d3b47c   // PixelCache::vftable
    /// [esi+0x2c] = 8
    /// [esi+0x34] = 0
    /// [esi+0x30] = 0
    /// [esi+0x38] = 0x3f800000
    /// [esi+0x24] = [edi+0x14]
    /// [esi+0x28] = byte [edi+0x18]
    /// [esi+0x29] = byte [edi+0x19]
    /// ```
    ///
    /// (`esi` is the node, so the value fields are the absolute offsets above
    /// minus `0x20`.)
    pub fn a74_new(prototype_word: u32, byte_18: u8, byte_19: u8) -> Self {
        A74PixelCache {
            vtable: A74_PIXEL_CACHE_VTABLE,
            prototype_word,
            byte_18,
            byte_19,
            bucket_count: A74_PIXEL_CACHE_BUCKETS,
            size: 0,
            zero_14: 0,
            max_load: A74_PIXEL_CACHE_LOAD_FACTOR,
        }
    }
}

/// `0x00626610`: the post-insert load-factor test (also at `0x00626ed0` in the
/// 4-byte-key map insert).
///
/// ```text
/// ++size;
/// ratio = (float)(double)size / (float)(double)bucket_count;   // unsigned
/// if (ratio > max_load) rehash(this, bucket_count * 2);
/// ```
///
/// The unsigned conversion is the `cvtdq2pd` + two-entry table at
/// `0x00d33180` (`+2^32` for negative inputs); for every 32-bit count it is
/// equal to the `u32 -> f64 -> f32` conversion used here.  `comiss` +
/// `jbe` grows only for a strictly greater, ordered ratio, so a NaN threshold
/// never grows.
pub fn a74_load_factor_exceeded(size: u32, bucket_count: u32, max_load: f32) -> bool {
    let ratio = (size as f64) as f32 / (bucket_count as f64) as f32;
    ratio > max_load
}

// ---------------------------------------------------------------------------
// Cache-key chain lookup: 0x006264c4
// ---------------------------------------------------------------------------

/// One node of a cache-key bucket chain.
///
/// The node's key is a `0x1c`-byte small-string record at `+0x04`.
/// `FUN_00539640` compares two such records by length and then byte content
/// (`data` at `+0x00`, `length` at `+0x04`, `sete` at the end), so the logical
/// key is the string, not a fixed-width field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A74KeyNode {
    /// `+0x00`: next node in the chain.
    pub next: Option<usize>,
    /// `+0x04`: the key string's bytes.
    pub key: Vec<u8>,
}

/// Bucket and match of [`a74_chain_find`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A74Find {
    /// `(bucket_count - 1) & hash`.
    pub bucket: u32,
    /// The matching node, when the chain contains the key.
    pub node: Option<usize>,
}

/// `0x006264c4`: finish the key string, hash it, mask and walk the chain.
///
/// The row first stores the terminator (`byte [key.data + key.len] = 0`) and
/// the length, then calls `FUN_00c62800(key.data, key.len, 0)` for the hash,
/// computes `(bucket_count - 1) & hash`, and walks the bucket chain comparing
/// each node's sixteen-byte key.  It stops before the node allocation (that
/// is the next block, `0x006265ab`).
///
/// The hash is recomputed with
/// [`crate::runtime::a43_hash_bytes`] and the key comparison is
/// [`A74KeyNode::key`] equality, which mirrors `FUN_00539640` (length, then
/// bytes).
pub fn a74_chain_find(
    buckets: &[Option<usize>],
    nodes: &[A74KeyNode],
    key: &[u8],
) -> Option<A74Find> {
    if buckets.is_empty() {
        return None;
    }
    let hash = crate::runtime::a43_hash_bytes(key);
    let bucket = (buckets.len() as u32).wrapping_sub(1) & hash;
    let mut cur = buckets[bucket as usize];
    while let Some(index) = cur {
        if nodes[index].key.as_slice() == key {
            return Some(A74Find {
                bucket,
                node: Some(index),
            });
        }
        cur = nodes[index].next;
    }
    Some(A74Find { bucket, node: None })
}

// ---------------------------------------------------------------------------
// 12-byte element vector: 0x0062692c, 0x00626a5e, 0x00626ae4
// ---------------------------------------------------------------------------

/// One element: three dwords.  The stride is recovered from the signed divide
/// by 12 (`0x2aaaaaab` magic at `0x006269d4`) and from `lea ecx,[esi+esi*2];
/// shl ecx,2` when the buffer is allocated.
pub type A74Word12 = [u32; 3];

/// `0x0062692c`: the capacity word after a growth test, or `None` when the
/// vector is not full.
///
/// ```text
/// cap = cap_word & 0x7fffffff;
/// if (size != cap) return;                 // no growth
/// step = max(1, cap_word >> 1 & 0x3fffffff);
/// new_capacity = cap + step;               // 1.5x
/// cap_word = (old ^ new_capacity) & 0x7fffffff ^ old;
/// ```
///
/// The last expression preserves the top bit of the capacity word and writes
/// the new 31-bit capacity.  The shipped `cap < cap + 1` test is on the masked
/// capacity, so the `0xffffffff` case cannot occur; the wrapping arithmetic is
/// kept literal.
pub fn a74_vec12_growth(size: u32, cap_word: u32) -> Option<u32> {
    let cap = cap_word & 0x7fff_ffff;
    if size != cap {
        return None;
    }
    let half = (cap_word >> 1) & 0x3fff_ffff;
    let step = if 1 <= half { half } else { 1 };
    let new_capacity = cap.wrapping_add(step);
    Some(((cap_word ^ new_capacity) & 0x7fff_ffff) ^ cap_word)
}

/// `0x00626a5e`: steal word 0 of every old element into the new buffer.
///
/// The shipped move loop is
///
/// ```text
/// for (src = old; src != end; src += 0xc) {
///     w = src[0]; src[0] = 0;
///     dst[0] = w; dst[4] = 0; dst[8] = 0;
///     dst += 0xc;
/// }
/// ```
///
/// so only the first dword moves and the old slot keeps its other two words.
/// The returned vector is the new buffer in element order.
pub fn a74_vec12_relocate(old: &mut [A74Word12]) -> Vec<A74Word12> {
    old.iter_mut()
        .map(|element| {
            let word = element[0];
            element[0] = 0;
            [word, 0, 0]
        })
        .collect()
}

/// Result of [`a74_vec12_append`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A74Vec12Append {
    /// Index of the new element (`old size`).
    pub slot: u32,
    /// Word 0 copied into the new slot (stolen or pending).
    pub value: u32,
}

/// `0x00626ae4`: append one element at `size`.
///
/// When `index < size` the shipped body steals `data[index][0]` and zeroes that
/// word (the moved-from slot); otherwise it uses `pending`.  The new element is
/// `[value, 0, 0]`, `size` increments, and the capacity word's top bit has
/// already been cleared by the caller.
pub fn a74_vec12_append(data: &mut Vec<A74Word12>, index: u32, pending: u32) -> A74Vec12Append {
    let size = data.len() as u32;
    let value = if index < size {
        let word = data[index as usize][0];
        data[index as usize][0] = 0;
        word
    } else {
        pending
    };
    data.push([value, 0, 0]);
    A74Vec12Append { slot: size, value }
}

// ---------------------------------------------------------------------------
// Inner insertion slot: 0x00626b94
// ---------------------------------------------------------------------------

/// `0x00626b94`: advance the eight-slot insert loop.
///
/// `0x00626b94` is the tail of the inner loop of `0x00626400`: after the
/// temporary key is released it does `[ebp-0x170] += 1`, compares against
/// eight, and jumps back to `0x00626470` while the counter is below eight.
/// Returns the next counter, or `None` when the loop is finished.
pub fn a74_inner_slot_next(index: u32) -> Option<u32> {
    let next = index.wrapping_add(1);
    if next < A74_CACHE_INSERTIONS_PER_RECORD {
        Some(next)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Cache-key map destructor: 0x00626c10, 0x00626c77, 0x00626c94, 0x00626cbc
// ---------------------------------------------------------------------------

/// `0x00626c10`: the order in which the destructor releases bucket nodes.
///
/// The shipped body returns immediately when the bucket array is null,
/// otherwise walks every bucket in ascending order and every chain from its
/// head, releasing each node's heap key (`0x00626c77`) and then the node
/// itself.  The returned indices are the node-release order; the key release
/// decision is [`a74_release_heap_string`], and the chain advance is
/// [`a74_chain_advance`].
///
/// A zero bucket count with a non-null array is an out-of-bounds read in the
/// shipped loop; the slice length is the bucket count here.
pub fn a74_destroy_order(buckets: &[Option<usize>], nodes: &[A74KeyNode]) -> Vec<usize> {
    let mut order = Vec::new();
    for head in buckets {
        let mut cur = *head;
        while let Some(index) = cur {
            order.push(index);
            cur = nodes[index].next;
        }
    }
    order
}

/// `0x00626c94`: continue with the next chain node.
///
/// `mov esi,ebx; test ebx,ebx; jne <chain body>` — a pure register move, kept
/// as a function so the destructor's three blocks map one-to-one onto the
/// ledger rows.
pub fn a74_chain_advance(next: Option<usize>) -> Option<usize> {
    next
}

/// `0x00626cbc`: the bucket-array word after the array is freed.
///
/// `mov [ebx+4],0` after the allocator's virtual call.  The pointer word is
/// cleared, so the function returns the stored zero.
pub fn a74_clear_bucket_word() -> u32 {
    0
}

// ---------------------------------------------------------------------------
// Destructor forwarder: 0x00626ce0
// ---------------------------------------------------------------------------

/// `0x00626ce0`: offsets used by the destroy-and-forward trampoline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A74ForwardPlan {
    /// `+0x30`: callback pointer (`mov ebx,[ecx+0x30]`).
    pub callback_offset: u32,
    /// `+0xb0`: first callback argument (`[ecx+0xb0]`, pushed last).
    pub arg_first_offset: u32,
    /// `+0xac`: second callback argument (`[ecx+0xac]`).
    pub arg_second_offset: u32,
    /// `+0x34`: the `0x78`-byte object copied to the stack.
    pub object_offset: u32,
    /// `0x78`: bytes copied (`sub esp,0x78`).
    pub object_bytes: u32,
}

/// `0x00626ce0`: the recovered call plan.
///
/// ```text
/// fn = [ecx+0x30]; a = [ecx+0xac]; b = [ecx+0xb0];
/// copy 0x78 bytes from (ecx+0x34) to the stack;
/// fn(b, a);
/// ```
pub fn a74_forward_plan() -> A74ForwardPlan {
    A74ForwardPlan {
        callback_offset: 0x30,
        arg_first_offset: 0xb0,
        arg_second_offset: 0xac,
        object_offset: 0x34,
        object_bytes: 0x78,
    }
}

// ---------------------------------------------------------------------------
// Hash containers: 0x00626d20 and 0x00626d90
// ---------------------------------------------------------------------------

/// `0x00626d20`: `contains` on a sixteen-byte-key hash set.
///
/// ```text
/// if (buckets == 0) return 0;
/// h = FUN_00c62800(key.data, key.len, 0);
/// node = buckets[(count - 1) & h];
/// while (node) {
///     if (FUN_00539640(key, node + 4)) return 1;
///     node = node->next;
/// }
/// return 0;
/// ```
///
/// The hash and the key comparison are reproduced here:
/// [`crate::runtime::a43_hash_bytes`] for `FUN_00c62800` and
/// [`A74KeyNode::key`] equality for `FUN_00539640` (length, then bytes).
pub fn a74_hash_contains(buckets: &[Option<usize>], nodes: &[A74KeyNode], key: &[u8]) -> bool {
    if buckets.is_empty() {
        return false;
    }
    let hash = crate::runtime::a43_hash_bytes(key);
    let bucket = (buckets.len() as u32).wrapping_sub(1) & hash;
    let mut cur = buckets[bucket as usize];
    while let Some(index) = cur {
        if nodes[index].key.as_slice() == key {
            return true;
        }
        cur = nodes[index].next;
    }
    false
}

/// `0x00626d90`: the four-byte MurmurHash3 finalizer used as the key hash.
///
/// ```text
/// h = key ^ (key >> 16);
/// h *= 0x85ebca6b;
/// h ^= h >> 13;
/// h *= 0xc2b2ae35;
/// h ^= h >> 16;
/// ```
///
/// The two multipliers are the MurmurHash3 x86-32 finalizer constants; the
/// row also uses the result as `(bucket_count - 1) & h`.
pub fn a74_hash_word(key: u32) -> u32 {
    let mut hash = key;
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x85eb_ca6b);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(0xc2b2_ae35);
    hash ^ (hash >> 16)
}

/// One node of the four-byte-key table (`0x14` bytes in the shipped image).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A74U32Node {
    /// `+0x00`: next node in the bucket chain.
    pub next: Option<usize>,
    /// `+0x04`: the four-byte key.
    pub key: u32,
    /// `+0x08/+0x0c`: the copied value words (`count` at `+0x0c`).
    pub value: Vec<u32>,
    /// `+0x10`: the value capacity word (`count & 0x7fffffff`).
    pub capacity: u32,
}

/// Result of [`A74U32Map::a74_insert`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A74U32Insert {
    /// True when a node was appended; false when the key already existed.
    pub inserted: bool,
    /// Index of the existing or new node.
    pub node: usize,
    /// The masked bucket used by the probe.
    pub bucket: u32,
    /// Load-factor growth target (`2 * bucket_count`), or `None`.
    pub regrow_to: Option<u32>,
}

/// The four-byte-key `unordered_map<uint32, vector<uint32>>` of `0x00626d90`.
///
/// The shipped map header is `+0x00` bucket count, `+0x04` bucket array,
/// `+0x08` element count and `+0x0c` maximum load factor.  A node is
/// `+0x00` next, `+0x04` key, `+0x08` value data, `+0x0c` value count and
/// `+0x10` value capacity (`count & 0x7fffffff`).
#[derive(Clone, Debug, PartialEq)]
pub struct A74U32Map {
    /// `+0x00`.
    pub bucket_count: u32,
    /// `+0x04`.
    pub buckets: Vec<Option<usize>>,
    /// `+0x08`.
    pub size: u32,
    /// `+0x0c`.
    pub max_load: f32,
    /// Nodes, in allocation order (the shipped allocator's addresses).
    pub nodes: Vec<A74U32Node>,
}

impl A74U32Map {
    /// Build a map with `bucket_count` empty buckets and load factor `1.0`.
    pub fn a74_new(bucket_count: u32) -> Self {
        A74U32Map {
            bucket_count,
            buckets: vec![None; bucket_count as usize],
            size: 0,
            max_load: 1.0,
            nodes: Vec::new(),
        }
    }

    /// `0x00626d90`: find-or-append a node for `key`.
    ///
    /// The chain walk appends at the tail (`*last_next = node`), skips the
    /// allocation when the key is found, copies the value words into a new
    /// `count * 4` buffer, increments the element count, and finally runs the
    /// load-factor test.  When it triggers, the shipped body calls the growth
    /// helper with `2 * bucket_count`; the target is reported in
    /// [`A74U32Insert::regrow_to`] but the rehash itself is outside the row.
    ///
    /// The row's empty-bucket path (`FUN_0053e930(*this)`) is not reproduced;
    /// callers must start with a non-empty bucket array.
    pub fn a74_insert(&mut self, key: u32, value: &[u32]) -> A74U32Insert {
        assert!(
            self.bucket_count > 0 && !self.buckets.is_empty(),
            "0x00626d90 defers the empty-bucket case to FUN_0053e930"
        );
        let hash = a74_hash_word(key);
        let bucket = self.bucket_count.wrapping_sub(1) & hash;

        let mut last: Option<usize> = None;
        let mut cur = self.buckets[bucket as usize];
        while let Some(index) = cur {
            if self.nodes[index].key == key {
                return A74U32Insert {
                    inserted: false,
                    node: index,
                    bucket,
                    regrow_to: None,
                };
            }
            last = Some(index);
            cur = self.nodes[index].next;
        }

        let index = self.nodes.len();
        self.nodes.push(A74U32Node {
            next: None,
            key,
            value: value.to_vec(),
            capacity: (value.len() as u32) & 0x7fff_ffff,
        });
        match last {
            Some(previous) => self.nodes[previous].next = Some(index),
            None => self.buckets[bucket as usize] = Some(index),
        }
        self.size = self.size.wrapping_add(1);

        let regrow_to = if a74_load_factor_exceeded(self.size, self.bucket_count, self.max_load) {
            Some(self.bucket_count.wrapping_mul(2))
        } else {
            None
        };
        A74U32Insert {
            inserted: true,
            node: index,
            bucket,
            regrow_to,
        }
    }
}

/// `0x00626f42`: the three words copied into the insert result.
///
/// `movq xmm0,[ebp-0x24]; movq [eax],xmm0; mov [eax+8],ecx` — the result is
/// `{node, bucket_slot, bucket_end}`.
pub fn a74_insert_result(node: u32, bucket_slot: u32, bucket_end: u32) -> [u32; 3] {
    [node, bucket_slot, bucket_end]
}

// ---------------------------------------------------------------------------
// Function-object vtables and RTTI: 0x00626f60, 0x00626f70, 0x00627030,
// 0x00627050, 0x00627070
// ---------------------------------------------------------------------------

/// `_bcCopyableFunctionObjectImpl<<lambda_1dd056d53516b876749ee1090b9c5462>&,bool>::vftable`.
pub const A74_VTABLE_LAMBDA_1DD056D5: u32 = 0x00d3_b7dc;
/// RTTI descriptor for the same lambda.
pub const A74_RTTI_LAMBDA_1DD056D5: u32 = 0x00f5_2e68;
/// `_bcCopyableFunctionImpl<Song,SongId>::vftable` written first.
pub const A74_VTABLE_SONG_BASE: u32 = 0x00d3_b8a0;
/// `_bcCopyableFunctionObjectImpl<Song (__cdecl*&)(SongId),Song,SongId>::vftable`.
pub const A74_VTABLE_SONG_OBJECT: u32 = 0x00d3_b860;
/// `_bcCopyableFunctionObjectImpl<<lambda_699267ade7e37a9b2b5f909b3ca40083>&,bool>::vftable`.
pub const A74_VTABLE_LAMBDA_699267AD: u32 = 0x00d3_b880;
/// RTTI descriptor for `Song (__cdecl*)(SongId)`.
pub const A74_RTTI_SONG_FACTORY: u32 = 0x00f5_2e44;

/// `0x00626f60`: store the lambda's vftable word.
pub fn a74_vftable_lambda_1dd056d5() -> u32 {
    A74_VTABLE_LAMBDA_1DD056D5
}

/// `0x00626f70`: the lambda's RTTI descriptor address.
///
/// The decompiled row is `return &<lambda_1dd056d53516b876749ee1090b9c5462>
/// ::RTTI_Type_Descriptor`; the descriptor name is returned with it.
pub fn a74_rtti_lambda_1dd056d5() -> (u32, &'static str) {
    (
        A74_RTTI_LAMBDA_1DD056D5,
        "<lambda_1dd056d53516b876749ee1090b9c5462>",
    )
}

/// The callback object built by `0x00627030` and `0x00627070`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A74SongFunctionObject {
    /// The final vftable word (`0x00d3b860`).
    pub vtable: u32,
    /// The captured word from `[ecx+0x4]`.
    pub captured: u32,
}

/// `0x00627030`: construct the `Song` function object.
///
/// `[eax] = 0x00d3b8a0; [eax+4] = [ecx+4]; [eax] = 0x00d3b860` — the base
/// vftable is written and immediately overwritten by the derived one.  The
/// store sequence is returned alongside the final object.
pub fn a74_song_function_object(captured: u32) -> (A74SongFunctionObject, [u32; 2]) {
    (
        A74SongFunctionObject {
            vtable: A74_VTABLE_SONG_OBJECT,
            captured,
        },
        [A74_VTABLE_SONG_BASE, A74_VTABLE_SONG_OBJECT],
    )
}

/// `0x00627050`: RTTI descriptor for the `Song (__cdecl*)(SongId)` type.
///
/// The recovered mangled name is `.P6A?AUSong@@W4SongId@@@Z`.
pub fn a74_rtti_song_factory() -> (u32, &'static str) {
    (A74_RTTI_SONG_FACTORY, ".P6A?AUSong@@W4SongId@@@Z")
}

/// `0x00627070`: store the second lambda's vftable word.
pub fn a74_vftable_lambda_699267ad() -> u32 {
    A74_VTABLE_LAMBDA_699267AD
}

/// `0x00626f85`: the tail fields copied out of a constructed `Song`.
///
/// `mov eax,[esi+0x1c]; mov [edi+0x1c],eax; mov al,[esi+0x20];
/// mov [edi+0x20],al; mov eax,[esi+0x24]; mov [edi+0x24],eax`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A74SongTail {
    /// `+0x1c`.
    pub word_1c: u32,
    /// `+0x20`.
    pub byte_20: u8,
    /// `+0x24`.
    pub word_24: u32,
}

/// `0x00626f85`: copy the three `Song` tail fields.
pub fn a74_song_tail_fields(word_1c: u32, byte_20: u8, word_24: u32) -> A74SongTail {
    A74SongTail {
        word_1c,
        byte_20,
        word_24,
    }
}

// ---------------------------------------------------------------------------
// Skin layout: 0x006272e3, 0x00627465
// ---------------------------------------------------------------------------

/// The four `0x1c`-byte string records of `Skin`, as
/// `(record offset, inline-buffer offset)`.
pub static A74_SKIN_STRING_SLOTS: [(u32, u32); 4] =
    [(0x00, 0x0c), (0x1c, 0x28), (0x38, 0x44), (0x54, 0x60)];
/// `+0x70`: the word copied verbatim by the `Skin` copy constructor.
pub const A74_SKIN_WORD_OFFSET: u32 = 0x70;
/// `+0x74`: the tagged pointer cloned by the copy constructor.
pub const A74_SKIN_TAGGED_OFFSET: u32 = 0x74;
/// `+0x88`: data of the `0x1c`-string vector.
pub const A74_SKIN_VECTOR_DATA: u32 = 0x88;
/// `+0x8c`: element count of the `0x1c`-string vector.
pub const A74_SKIN_VECTOR_COUNT: u32 = 0x8c;
/// `+0x90`: capacity word (`count & 0x7fffffff`) of the vector.
pub const A74_SKIN_VECTOR_CAPACITY: u32 = 0x90;
/// One `0x1c`-byte string element: the copy loop advances both cursors by
/// `0x1c` (`add ebx,0x1c; add esi,0x1c` at `0x006274da`).
pub const A74_SKIN_STRING_STRIDE: u32 = 0x1c;

/// `0x006272e3`: the four string slots copied by the `Skin` copy constructor.
///
/// `0x00627210` initialises the slot at `+0x00` (inline buffer `+0x0c`);
/// `0x006272e3` is the same block for the slot at `+0x38` (buffer `+0x44`);
/// the remaining two slots are `+0x1c` and `+0x54`.  Each slot is
/// `{data, length, 0x8000000f, inline[16]}`.
pub fn a74_skin_string_slots() -> [(u32, u32); 4] {
    A74_SKIN_STRING_SLOTS
}

/// `0x00627465`: allocation arithmetic for the `0x1c`-string vector.
///
/// ```text
/// count = (src_end - src_data) / 0x1c;   // signed divide by 0x1c
/// data  = alloc(count * 0x1c, 8);
/// [dest+0x88] = data;
/// [dest+0x8c] = count;
/// [dest+0x90] = count & 0x7fffffff;
/// ```
///
/// Returns `(bytes, count, capacity word)`.
pub fn a74_string_vector_bytes(count: u32) -> (u32, u32, u32) {
    (
        count.wrapping_mul(A74_SKIN_STRING_STRIDE),
        count,
        count & 0x7fff_ffff,
    )
}

// ---------------------------------------------------------------------------
// Tagged pointer release: 0x00627520 and 0x00627575
// ---------------------------------------------------------------------------

/// What a tagged pointer word designates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A74TaggedTarget {
    /// Tag bit set and the payload is null: the empty state, no destructor.
    Empty,
    /// Tag bit clear: the object lives in the field itself.
    Field,
    /// Tag bit set with a non-null payload: a heap object.
    Heap(u32),
}

/// `0x00627520`: decode the tagged pointer at `+0x94` (and `+0x74` in
/// `0x006275b0`).
///
/// `test byte [p],1; je <field>; p = [p] & 0xfffffffe; test p,p; je <empty>`.
/// A tag-zero word names the field's own address, which is never null; a
/// tag-one word with a null payload is the empty state.
pub fn a74_tagged_target(word: u32) -> A74TaggedTarget {
    if word & 1 == 0 {
        A74TaggedTarget::Field
    } else {
        let pointer = word & 0xffff_fffe;
        if pointer == 0 {
            A74TaggedTarget::Empty
        } else {
            A74TaggedTarget::Heap(pointer)
        }
    }
}

/// `0x00627575`: the word stored after a tagged pointer is released.
///
/// The row is the continuation of the `+0x94` release in `0x00627520`
/// (`mov dword ptr [esi],1`); the same empty-state store closes the `+0x74`
/// release in `0x006275b0`.
pub fn a74_tagged_released_word() -> u32 {
    1
}

// ---------------------------------------------------------------------------
// Skin destructor: 0x006275b0 and its five continuation rows
// ---------------------------------------------------------------------------

/// One string slot of the `Skin` destructor: `(capacity word offset, data
/// word offset)`.
pub static A74_SKIN_DTOR_STRING_SLOTS: [(u32, u32); 4] =
    [(0x5c, 0x54), (0x40, 0x38), (0x24, 0x1c), (0x08, 0x00)];
/// `+0x88`: lock/guard word passed to `FUN_0053aba0`.
pub const A74_SKIN_DTOR_LOCK_OFFSET: u32 = 0x88;
/// `+0x74`: tagged pointer released before the strings.
pub const A74_SKIN_DTOR_TAGGED_OFFSET: u32 = 0x74;

/// `0x006275b0`: the recovered `Skin` destructor plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A74SkinDtorPlan {
    /// `+0x88`: `lea ecx,[edi+0x88]; call FUN_0053aba0`.
    pub lock_offset: u32,
    /// `+0x74`: tagged pointer release.
    pub tagged_offset: u32,
    /// The four `(capacity, data)` string slots, in check order.
    pub strings: [(u32, u32); 4],
}

/// `0x006275b0`: the lock word, tagged pointer and four strings destroyed by
/// the `Skin` destructor.
///
/// The body first locks `+0x88`, releases the tagged pointer at `+0x74`, then
/// for each slot tests the capacity word (`cmp [edi+off],0; jl skip`) and frees
/// the data word when it is non-negative.  The order is `+0x5c`, `+0x40`,
/// `+0x24`, `+0x08`.
pub fn a74_skin_dtor_plan() -> A74SkinDtorPlan {
    A74SkinDtorPlan {
        lock_offset: A74_SKIN_DTOR_LOCK_OFFSET,
        tagged_offset: A74_SKIN_DTOR_TAGGED_OFFSET,
        strings: A74_SKIN_DTOR_STRING_SLOTS,
    }
}

/// `0x0062760e`: the destructor's continuation after the tagged release.
///
/// This row is the block that starts at `0x0062760e`; its decompiled body
/// contains all four capacity checks (`+0x5c`, `+0x40`, `+0x24`, `+0x08`).
pub fn a74_skin_dtor_slots_from_5c() -> &'static [(u32, u32)] {
    &A74_SKIN_DTOR_STRING_SLOTS[..]
}

/// `0x00627645`: the continuation that begins with the `+0x40` check.
pub fn a74_skin_dtor_slots_from_40() -> &'static [(u32, u32)] {
    &A74_SKIN_DTOR_STRING_SLOTS[1..]
}

/// `0x00627664`: the continuation that begins with the `+0x24` check.
pub fn a74_skin_dtor_slots_from_24() -> &'static [(u32, u32)] {
    &A74_SKIN_DTOR_STRING_SLOTS[2..]
}

/// `0x00627683`: the continuation that begins with the `+0x08` check.
pub fn a74_skin_dtor_slots_from_08() -> &'static [(u32, u32)] {
    &A74_SKIN_DTOR_STRING_SLOTS[3..]
}

/// `0x006276a1`: the final continuation, which only performs the virtual free
/// of the last data word and returns.
pub fn a74_skin_dtor_slots_from_none() -> &'static [(u32, u32)] {
    &A74_SKIN_DTOR_STRING_SLOTS[4..]
}

// ---------------------------------------------------------------------------
// Option callback: 0x00627780
// ---------------------------------------------------------------------------

/// `0x00627780`: the lambda object and target used by the option handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A74OptionCallbackPlan {
    /// Lambda vftable word stored at `[ebp-0x24]`.
    pub vtable: u32,
    /// Initial tagged word (`[ebp-0x38] = 1`).
    pub tagged_word: u32,
    /// Offset passed as the callback target (`param + 0x78`).
    pub target_offset: u32,
}

/// `0x00627780`: construct the option lambda, invoke the binder and release
/// it.
///
/// The recovered constants are the lambda vftable `0x00d3b938`, the initial
/// tagged word `1`, and the `+0x78` target (`lea esi,[eax+0x78]`).  The
/// binder/release helpers (`FUN_00534dc0`, `FUN_0053a470`) are outside the
/// row, so only the plan is returned.
pub fn a74_option_callback_plan() -> A74OptionCallbackPlan {
    A74OptionCallbackPlan {
        vtable: 0x00d3_b938,
        tagged_word: 1,
        target_offset: 0x78,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent74 {
    use super::*;

    // -- 0x00626400 / 0x0062646d ------------------------------------------

    #[test]
    fn cache_scan_is_three_0x78_byte_records() {
        assert_eq!(
            a74_cache_record_offsets(),
            [0x00fc_9b88, 0x00fc_9c00, 0x00fc_9c78]
        );
        assert_eq!(A74_CACHE_RECORD_STRIDE, 0x78);
        assert_eq!(A74_CACHE_RECORD_BYTES, 3 * A74_CACHE_RECORD_STRIDE);
        let end = a74_cache_record_offsets()[2] + A74_CACHE_RECORD_STRIDE;
        assert_eq!(end - A74_CACHE_RECORD_BASE, A74_CACHE_RECORD_BYTES);
        assert_eq!(A74_CACHE_PREDICATE_ARG, 0x0b);
        assert_eq!(A74_CACHE_INSERTIONS_PER_RECORD, 8);
    }

    #[test]
    fn cache_record_key_reads_the_two_words() {
        let mut record = [0u8; A74_CACHE_RECORD_STRIDE as usize];
        record[0x44..0x48].copy_from_slice(&0x00de_ad00u32.to_le_bytes());
        record[0x48..0x4c].copy_from_slice(&14u32.to_le_bytes());
        assert_eq!(a74_cache_record_key(&record), (0x00de_ad00, 14));
    }

    // -- 0x00626490 / 0x00627210 / 0x006272d0 / 0x006274d5 ----------------

    #[test]
    fn sso_record_starts_empty_and_inline() {
        let sso = A74Sso::a74_new();
        assert_eq!(sso.data, 0x0c);
        assert_eq!(sso.len, 0);
        assert_eq!(sso.cap, 0x8000_000f);
        assert_eq!(sso.inline, [0u8; 16]);
        assert!(sso.a74_is_inline());
        assert_eq!(sso.a74_heap_ptr(), None);
    }

    #[test]
    fn sso_inline_assign_terminates_at_length() {
        let mut sso = A74Sso::a74_new();
        assert_eq!(
            sso.a74_assign_inline(b"Minimap cacher"),
            A74SsoAssign::Inline { terminator_at: 14 }
        );
        assert_eq!(sso.len, 14);
        assert_eq!(&sso.inline[..14], b"Minimap cacher");
        assert_eq!(sso.inline[14], 0);
        assert_eq!(sso.cap, 0x8000_000f);
        assert!(sso.a74_terminate_at(14));
        assert_eq!((sso.len, sso.inline[14]), (14, 0));
    }

    #[test]
    fn sso_sixteen_bytes_need_the_heap_path() {
        let mut sso = A74Sso::a74_new();
        assert_eq!(
            sso.a74_assign_inline(b"0123456789abcdef"),
            A74SsoAssign::NeedsHeap { len: 16 }
        );
        // The inline record is untouched by the rejected assignment.
        assert_eq!(sso, A74Sso::a74_new());
        assert!(!sso.a74_terminate_at(16));
    }

    #[test]
    fn sso_fifteen_bytes_fit_exactly() {
        let mut sso = A74Sso::a74_new();
        assert_eq!(
            sso.a74_assign_inline(b"0123456789abcde"),
            A74SsoAssign::Inline { terminator_at: 15 }
        );
        assert_eq!(sso.inline[15], 0);
        assert_eq!(sso.len, 15);
    }

    // -- 0x0062670a / 0x00626c77 ------------------------------------------

    #[test]
    fn heap_release_only_zeroes_a_heap_word() {
        assert_eq!(
            a74_release_heap_string(0x0041_0000, 0x8000_000f),
            0x0041_0000
        );
        assert_eq!(
            a74_release_heap_string(0x0041_0000, 0xffff_ffff),
            0x0041_0000
        );
        assert_eq!(a74_release_heap_string(0x0041_0000, 0x1f), 0);
        assert_eq!(a74_release_heap_string(0x0041_0000, 0), 0);

        let mut sso = A74Sso::a74_new();
        sso.cap = 0x20;
        sso.data = 0x0050_0000;
        assert_eq!(sso.a74_heap_ptr(), Some(0x0050_0000));
    }

    // -- 0x006264c4 -------------------------------------------------------

    fn key_nodes() -> Vec<A74KeyNode> {
        vec![
            A74KeyNode {
                next: Some(1),
                key: b"aaaaaaaaaaaaaaaa".to_vec(),
            },
            A74KeyNode {
                next: Some(2),
                key: b"bbbbbbbbbbbbbbbb".to_vec(),
            },
            A74KeyNode {
                next: None,
                key: b"cccccccccccccccc".to_vec(),
            },
        ]
    }

    #[test]
    fn chain_find_masks_the_hash_and_walks_the_chain() {
        let nodes = key_nodes();
        let hash = crate::runtime::a43_hash_bytes(b"bbbbbbbbbbbbbbbb");
        let mut buckets = vec![None; 4];
        let bucket = (buckets.len() as u32 - 1) & hash;
        buckets[bucket as usize] = Some(0);
        let find = a74_chain_find(&buckets, &nodes, b"bbbbbbbbbbbbbbbb").unwrap();
        assert_eq!(find.bucket, bucket);
        assert_eq!(find.node, Some(1));

        let miss = a74_chain_find(&buckets, &nodes, b"dddddddddddddddd").unwrap();
        assert_eq!(miss.node, None);
        assert_eq!(a74_chain_find(&[], &nodes, b"x"), None);
    }

    // -- 0x00626610 -------------------------------------------------------

    #[test]
    fn pixel_cache_defaults_are_exact() {
        let cache = A74PixelCache::a74_new(0x1122_3344, 0x55, 0x66);
        assert_eq!(cache.vtable, 0x00d3_b47c);
        assert_eq!(cache.prototype_word, 0x1122_3344);
        assert_eq!((cache.byte_18, cache.byte_19), (0x55, 0x66));
        assert_eq!(cache.bucket_count, 8);
        assert_eq!(cache.size, 0);
        assert_eq!(cache.zero_14, 0);
        assert_eq!(cache.max_load, 1.0);
    }

    #[test]
    fn load_factor_is_a_strict_float_compare() {
        assert!(!a74_load_factor_exceeded(8, 8, 1.0));
        assert!(a74_load_factor_exceeded(9, 8, 1.0));
        assert!(!a74_load_factor_exceeded(1, 8, 0.125));
        assert!(a74_load_factor_exceeded(2, 8, 0.125));
        assert!(!a74_load_factor_exceeded(1, 8, f32::NAN));
    }

    // -- 0x0062672a -------------------------------------------------------

    #[test]
    fn cache_thread_name_is_the_exact_literal() {
        assert_eq!(a74_cache_thread_name(), ("Minimap cacher", 0x00d3_b488));
        assert_eq!(a74_cache_thread_header(), (0x0010_0000, 0, 4));
    }

    #[test]
    fn cache_thread_name_copy_stops_at_fourteen() {
        let mut dst = [0xaau8; 0x20];
        assert_eq!(a74_copy_cache_thread_name(&mut dst), 14);
        assert_eq!(&dst[..14], b"Minimap cacher");
        assert_eq!(dst[14], 0);
        // Bytes past the terminator are untouched.
        assert_eq!(dst[15], 0xaa);
    }

    // -- 0x0062692c / 0x00626a5e / 0x00626ae4 -----------------------------

    #[test]
    fn vec12_growth_is_one_and_a_half() {
        assert_eq!(a74_vec12_growth(7, 8), None);
        assert_eq!(a74_vec12_growth(8, 8), Some(12));
        assert_eq!(a74_vec12_growth(16, 16), Some(24));
        assert_eq!(a74_vec12_growth(0, 0), Some(1));
        // The top flag of the capacity word survives.
        assert_eq!(a74_vec12_growth(8, 0x8000_0008), Some(0x8000_000c));
        assert_eq!(a74_vec12_growth(0x10, 0x8000_0010), Some(0x8000_0018));
    }

    #[test]
    fn vec12_relocate_steals_only_word_zero() {
        let mut old = [[1, 2, 3], [4, 5, 6]];
        let new = a74_vec12_relocate(&mut old);
        assert_eq!(new, [[1, 0, 0], [4, 0, 0]]);
        assert_eq!(old, [[0, 2, 3], [0, 5, 6]]);
    }

    #[test]
    fn vec12_append_takes_the_pending_or_stolen_word() {
        let mut data = vec![[0x11, 0x22, 0x33], [0x44, 0x55, 0x66]];
        assert_eq!(
            a74_vec12_append(&mut data, 1, 0x99),
            A74Vec12Append {
                slot: 2,
                value: 0x44
            }
        );
        assert_eq!(data[1], [0, 0x55, 0x66]);
        assert_eq!(data[2], [0x44, 0, 0]);

        let mut data = vec![[0x11, 0x22, 0x33]];
        assert_eq!(
            a74_vec12_append(&mut data, 5, 0x99),
            A74Vec12Append {
                slot: 1,
                value: 0x99
            }
        );
        assert_eq!(data[1], [0x99, 0, 0]);
    }

    // -- 0x00626b94 -------------------------------------------------------

    #[test]
    fn inner_slots_stop_before_eight() {
        assert_eq!(a74_inner_slot_next(0), Some(1));
        assert_eq!(a74_inner_slot_next(6), Some(7));
        assert_eq!(a74_inner_slot_next(7), None);
        // `inc eax` wraps, so 0xffff_ffff becomes the in-range counter 0.
        assert_eq!(a74_inner_slot_next(u32::MAX), Some(0));
    }

    // -- 0x00626c10 / 0x00626c94 / 0x00626cbc -----------------------------

    #[test]
    fn destroy_order_walks_buckets_then_chains() {
        let nodes = key_nodes();
        let buckets = vec![Some(0), None, None, None];
        assert_eq!(a74_destroy_order(&buckets, &nodes), vec![0, 1, 2]);
        // A second bucket starts its own chain after the first one is done.
        let separate = vec![
            A74KeyNode {
                next: Some(1),
                key: b"x".to_vec(),
            },
            A74KeyNode {
                next: None,
                key: b"y".to_vec(),
            },
            A74KeyNode {
                next: None,
                key: b"z".to_vec(),
            },
        ];
        let buckets = vec![Some(0), Some(2), None, None];
        assert_eq!(a74_destroy_order(&buckets, &separate), vec![0, 1, 2]);
        assert_eq!(a74_chain_advance(Some(2)), Some(2));
        assert_eq!(a74_chain_advance(None), None);
        assert_eq!(a74_clear_bucket_word(), 0);
        assert!(a74_destroy_order(&[], &nodes).is_empty());
    }

    // -- 0x00626ce0 -------------------------------------------------------

    #[test]
    fn forward_plan_offsets() {
        assert_eq!(
            a74_forward_plan(),
            A74ForwardPlan {
                callback_offset: 0x30,
                arg_first_offset: 0xb0,
                arg_second_offset: 0xac,
                object_offset: 0x34,
                object_bytes: 0x78,
            }
        );
    }

    // -- 0x00626d20 -------------------------------------------------------

    #[test]
    fn contains_returns_only_a_full_key_match() {
        let nodes = key_nodes();
        let hash = crate::runtime::a43_hash_bytes(b"cccccccccccccccc");
        let mut buckets = vec![None; 4];
        buckets[((4u32 - 1) & hash) as usize] = Some(0);
        assert!(a74_hash_contains(&buckets, &nodes, b"cccccccccccccccc"));
        assert!(!a74_hash_contains(&buckets, &nodes, b"dddddddddddddddd"));
        assert!(!a74_hash_contains(&[], &nodes, b"x"));
    }

    // -- 0x00626d90 / 0x00626f42 ------------------------------------------

    #[test]
    fn key_hash_uses_the_murmur_finalizer() {
        assert_eq!(a74_hash_word(0), 0);
        assert_eq!(a74_hash_word(1), 0x514e_28b7);
        assert_eq!(a74_hash_word(0x1234_5678), 0xe37c_d1bc);
        assert_eq!(a74_hash_word(0xdead_beef), 0x0de5_c6a9);
        assert_eq!(a74_hash_word(0xffff_ffff), 0x81f1_6f39);
    }

    #[test]
    fn u32_map_insert_appends_and_reports_growth() {
        let mut map = A74U32Map::a74_new(2);
        let first = map.a74_insert(7, &[0x11, 0x22]);
        assert!(first.inserted);
        assert_eq!(first.node, 0);
        assert_eq!(first.bucket, 1 & a74_hash_word(7));
        // One element in two buckets is a 0.5 load factor: no growth yet.
        assert_eq!(first.regrow_to, None);

        let again = map.a74_insert(7, &[0x33]);
        assert!(!again.inserted);
        assert_eq!(again.node, 0);
        assert_eq!(map.nodes[0].value, vec![0x11, 0x22]);
        assert_eq!(map.nodes[0].capacity, 2);
        assert_eq!(map.size, 1);

        // A different key that maps to the same bucket is appended at the tail.
        let mut same_bucket = 1u32;
        while (1 & a74_hash_word(same_bucket)) != (1 & a74_hash_word(7)) || same_bucket == 7 {
            same_bucket += 1;
        }
        let tail = map.a74_insert(same_bucket, &[0x44]);
        assert!(tail.inserted);
        assert_eq!(tail.bucket, first.bucket);
        assert_eq!(map.nodes[0].next, Some(tail.node));
        assert_eq!(map.size, 2);
        // 2 / 2 == 1.0 is not strictly greater than the 1.0 default.
        assert_eq!(tail.regrow_to, None);

        // A third element pushes 3 / 2 above 1.0 and asks for four buckets.
        let mut third = 1000u32;
        while third == 7 || third == same_bucket {
            third += 1;
        }
        let grown = map.a74_insert(third, &[]);
        assert!(grown.inserted);
        assert_eq!(grown.regrow_to, Some(4));
        assert_eq!(map.size, 3);
    }

    #[test]
    fn u32_map_insert_result_words() {
        assert_eq!(a74_insert_result(0x11, 0x22, 0x33), [0x11, 0x22, 0x33]);
    }

    // -- 0x00626f60 / 0x00626f70 / 0x00627030 / 0x00627050 / 0x00627070 --

    #[test]
    fn lambda_vtable_and_rtti_words() {
        assert_eq!(a74_vftable_lambda_1dd056d5(), 0x00d3_b7dc);
        assert_eq!(
            a74_rtti_lambda_1dd056d5(),
            (0x00f5_2e68, "<lambda_1dd056d53516b876749ee1090b9c5462>")
        );
        assert_eq!(a74_vftable_lambda_699267ad(), 0x00d3_b880);
        assert_eq!(
            a74_rtti_song_factory(),
            (0x00f5_2e44, ".P6A?AUSong@@W4SongId@@@Z")
        );
    }

    #[test]
    fn song_function_object_stores_base_then_derived() {
        let (object, stores) = a74_song_function_object(0x1234);
        assert_eq!(stores, [A74_VTABLE_SONG_BASE, A74_VTABLE_SONG_OBJECT]);
        assert_eq!(
            object,
            A74SongFunctionObject {
                vtable: A74_VTABLE_SONG_OBJECT,
                captured: 0x1234,
            }
        );
    }

    #[test]
    fn song_tail_fields_match_the_copy_out() {
        assert_eq!(
            a74_song_tail_fields(0xaa, 0xbb, 0xcc),
            A74SongTail {
                word_1c: 0xaa,
                byte_20: 0xbb,
                word_24: 0xcc,
            }
        );
    }

    // -- 0x006272e3 / 0x00627465 ------------------------------------------

    #[test]
    fn skin_string_slots_are_0x1c_apart() {
        assert_eq!(
            a74_skin_string_slots(),
            [(0, 0x0c), (0x1c, 0x28), (0x38, 0x44), (0x54, 0x60)]
        );
        for (slot, inline) in a74_skin_string_slots() {
            assert_eq!(inline - slot, 0x0c);
        }
    }

    #[test]
    fn string_vector_bytes_use_the_0x1c_stride() {
        assert_eq!(a74_string_vector_bytes(0), (0, 0, 0));
        assert_eq!(a74_string_vector_bytes(1), (0x1c, 1, 1));
        assert_eq!(a74_string_vector_bytes(5), (0x8c, 5, 5));
        assert_eq!(a74_string_vector_bytes(0x8000_0000), (0, 0x8000_0000, 0));
    }

    // -- 0x00627520 / 0x00627575 ------------------------------------------

    #[test]
    fn tagged_pointer_decoding() {
        assert_eq!(a74_tagged_target(0), A74TaggedTarget::Field);
        assert_eq!(a74_tagged_target(1), A74TaggedTarget::Empty);
        assert_eq!(a74_tagged_target(0x1234), A74TaggedTarget::Field);
        assert_eq!(a74_tagged_target(0x1235), A74TaggedTarget::Heap(0x1234));
        assert_eq!(
            a74_tagged_target(0xffff_fffd),
            A74TaggedTarget::Heap(0xffff_fffc)
        );
        assert_eq!(a74_tagged_released_word(), 1);
    }

    // -- 0x006275b0 ..= 0x006276a1 ----------------------------------------

    #[test]
    fn skin_destructor_plan_order() {
        let plan = a74_skin_dtor_plan();
        assert_eq!(plan.lock_offset, 0x88);
        assert_eq!(plan.tagged_offset, 0x74);
        assert_eq!(
            plan.strings,
            [(0x5c, 0x54), (0x40, 0x38), (0x24, 0x1c), (0x08, 0x00)]
        );

        assert_eq!(a74_skin_dtor_slots_from_5c().len(), 4);
        assert_eq!(
            a74_skin_dtor_slots_from_40(),
            &[(0x40, 0x38), (0x24, 0x1c), (0x08, 0x00)]
        );
        assert_eq!(a74_skin_dtor_slots_from_24(), &[(0x24, 0x1c), (0x08, 0x00)]);
        assert_eq!(a74_skin_dtor_slots_from_08(), &[(0x08, 0x00)]);
        assert!(a74_skin_dtor_slots_from_none().is_empty());
    }

    // -- 0x00627780 -------------------------------------------------------

    #[test]
    fn option_callback_plan_constants() {
        assert_eq!(
            a74_option_callback_plan(),
            A74OptionCallbackPlan {
                vtable: 0x00d3_b938,
                tagged_word: 1,
                target_offset: 0x78,
            }
        );
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// Six of the 45 `decompiled_uncited` rows are not translated:
//
// * `FUN_00626e3c`: starts at `FUN_00626e3c`, the last byte of the `mov dword
//   [edx],0` immediate at `0x00626e37`; the decompiled
//   `*(char *)(unaff_EBX + 0x44a890e) += in_CL` is a misaligned instruction.
// * `FUN_00626f80` (`FUN_00626f80`, 5 bytes): prologue only (`push ebp;
//   mov ebp,esp; push -1`); the row's decompiled body belongs to the
//   continuation at `0x00626f85`, translated as [`a74_song_tail_fields`].
// * `FUN_00627750` (`FUN_00627750`, 16 bytes): a single tail call to
//   `FUN_0053ae90`; no local work.
// * `FUN_006277cf`: starts at `FUN_006277cf`, inside the `mov dword
//   [ebp-0x7c],4` immediate at `0x006277cc`; the row's body begins with the
//   misaligned `*in_EAX = *in_EAX + (char)in_EAX`.
// * `FUN_006279b8`: starts at `FUN_006279b8`, the operand byte of
//   `neg al` (`f6 d8`) inside the obfuscated byte sequence that follows
//   `0x00627907`; the decompiler's `in_ST0 + in_ST7` is a misaligned
//   `faddp` decode, not a real function.
// * `FUN_00627e90` (`FUN_00627e90`, 215 bytes): the entry bytes are `3e 00 00`
//   (`ds add [eax],al`) of the preceding obfuscated block, and the body is a
//   jump-table failure (`Could not recover jumptable at 0x0063ae1d`) with
//   misaligned stores.
//
// The one `decompile_failed` row, `FUN_00627b70` (`FUN_00627b70`, 374 bytes),
// is also not translated: its decompilation timed out, and the surrounding
// `0x00627907..=0x00627b63` bytes are the runtime's obfuscated instruction
// stream, so there is no recovered body to translate.
