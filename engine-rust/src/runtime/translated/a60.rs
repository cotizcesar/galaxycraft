#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent60.rs
// ===========================================================================
// Translation batch 60.  Only this file is edited by the batch.
//
// Batch 60 owns `0x0060a000..=0x0060bfff`.  The ledger lists 41
// `decompiled_uncited` rows in that window; 21 are translated below.  The
// remaining rows are either mid-instruction fragments, bodies that end in a
// decompiler `noreturn` marker before the real tail, or bodies whose only
// interesting work is an indirect call into a routine outside this batch.
// The skip reasons are recorded in each section comment and summarised at the
// end of this file.
//
// Two object families dominate the window:
//
// * A 0x70-byte record whose first member is an MSVC-style string header
//   (`+0x00` data, `+0x04` length, `+0x08` capacity/flags), with a second
//   identical string at `+0x50`.  The constructors at `0x0060a310`,
//   `0x0060a4d0`, `0x0060a5e0` and `0x0060a820` all initialise the same
//   header shape and then request capacity through the shipped helper.
//   The capacity request is reproduced exactly in [`a60_grown_capacity`]:
//   `(cap >> 1 & 0x3fffffff) + (cap & 0x7fffffff)`, clamped up to the
//   request, then merged back under the original top bit.
// * A chained hash table whose buckets hold a `(next, key-data, key-len,
//   value)` node.  `0x0060b0f0` walks a chain using the same four-byte block
//   compare as the shipped helper; `0x0060a880` computes the MurmurHash3
//   x86-32 bucket with the exact recovered constants and follows that
//   chain.  [`a60_hash_bytes`] is the shipped 32-bit MurmurHash3, seed zero.
//
// The remaining rows are the game-speed configuration block
// (`FUN_0060b4a0..=0x0060b7c0`), the map-tile flag decoder at `0x0060bfb0`,
// two small C++ callable-object helpers (`0x0060bf20`, `0x0060bf40`) and the
// 32-byte record flag builder at `0x0060b1b0`.

// ---------------------------------------------------------------------------
// Recovered string and record layout
// ---------------------------------------------------------------------------

/// Capacity/flags word written by every empty-string initialiser in this
/// window (`param[2] = 0x8000000f` in `0x0060a310`, `0x0060a4d0`,
/// `0x0060a5e0`, `0x0060a820` and `0x0060a950`).
pub const A60_STRING_CAP: u32 = 0x8000_000f;

/// Low 31-bit capacity mask used by the growth policy (`0x7fffffff` in the
/// shipped capacity helper).
pub const A60_STRING_CAP_MASK: u32 = 0x7fff_ffff;

/// One recovered string header plus the first byte of its inline buffer.
///
/// The full inline buffer is not part of the initialiser's stores: each
/// constructor writes exactly one zero byte at `+0x0c` (and `+0x5c` for the
/// second member).  The untouched remainder is deliberately not modelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A60String {
    /// `+0x04`: stored byte count.
    pub len: u32,
    /// `+0x08`: capacity with the top flag bit.
    pub cap: u32,
    /// `+0x0c`: first inline byte written by the initialiser.
    pub first: u8,
}

/// The empty header written by the constructors.
pub const fn a60_empty_string() -> A60String {
    A60String {
        len: 0,
        cap: A60_STRING_CAP,
        first: 0,
    }
}

/// The 0x70-byte record shared by `0x0060a310`, `0x0060a4d0` and
/// `0x0060a5e0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A60Record {
    /// `+0x00`: first string member.
    pub name: A60String,
    /// `+0x20`: cleared dword.
    pub word20: u32,
    /// `+0x24`: cleared dword.
    pub word24: u32,
    /// `+0x28`: cleared word.
    pub word28: u16,
    /// `+0x30..+0x4c`: eight cleared dwords.
    pub words30: [u32; 8],
    /// `+0x50`: second string member.
    pub tag: A60String,
    /// `+0x29`: mode byte read by `0x0060af90`.
    pub flag29: u8,
    /// `+0x6c`: mode byte read by `0x0060ad70`.
    pub byte6c: u8,
}

/// Result of one of the three record constructors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A60RecordInit {
    /// Fully initialised record after the constructor's stores.
    pub record: A60Record,
    /// Capacity request: the length word read from the source object
    /// (`+0x0c` in `0x0060a310`, `+0x04` in `0x0060a4d0`/`0x0060a5e0`).
    pub requested_capacity: u32,
    /// `0x0060a310` only: the 64-bit duration returned by the shipped
    /// 64-bit multiply helper for the source pair at `+0x48`/`+0x4c`,
    /// scaled by 1,000,000,000.
    pub span_ns: Option<u64>,
}

/// Capacity request used by the record constructors: request `requested`
/// bytes of string capacity.
///
/// The stored capacity is `(cap ^ grown) & 0x7fffffff ^ cap`, which preserves
/// the original top bit and replaces the low 31 bits with
///
/// ```text
/// grown = (cap >> 1 & 0x3fffffff) + (cap & 0x7fffffff)
/// if grown < requested { grown = requested }
/// ```
///
/// Growth only happens when the current low capacity is smaller than the
/// request; the length and inline byte are left alone.
pub fn a60_grown_capacity(cap: u32, requested: u32) -> u32 {
    let low = cap & A60_STRING_CAP_MASK;
    if low < requested {
        let mut grown = (cap >> 1 & 0x3fff_ffff) + low;
        if grown < requested {
            grown = requested;
        }
        (cap ^ grown) & A60_STRING_CAP_MASK ^ cap
    } else {
        cap
    }
}

/// The fields common to the three record constructors.
fn a60_record_skeleton(flag29: u8) -> A60Record {
    A60Record {
        name: a60_empty_string(),
        word20: 0,
        word24: 0,
        word28: 0,
        words30: [0; 8],
        tag: a60_empty_string(),
        flag29,
        byte6c: 0,
    }
}

/// `0x0060a310` (`FUN_0060a310`, 223 bytes): initialise a 0x70-byte record
/// and request capacity for its first string.
///
/// The body first evaluates the shipped 64-bit multiply helper on
/// `(src[0x48], src[0x4c], 1_000_000_000, 0)`; with the low/high pair
/// `(lo, hi)` that is exactly `((hi << 32) | lo) * 1_000_000_000` modulo
/// 2^64.  It then stores the skeleton (`name` length 0, capacity
/// `0x8000000f`, `+0x0c = 0`, `+0x20`, `+0x24`, `+0x28`, `+0x30..+0x4c` all
/// zero, `tag` at `+0x50` with the same empty header, `+0x5c = 0`,
/// `+0x6c = 0`, mode byte at `+0x29 = 0`) and finishes with the capacity
/// request for `src[0x0c]`.
pub fn a60_record_init_a310(lo: u32, hi: u32, requested: u32) -> A60RecordInit {
    let mut record = a60_record_skeleton(0);
    record.name.cap = a60_grown_capacity(record.name.cap, requested);
    A60RecordInit {
        record,
        requested_capacity: requested,
        span_ns: Some(a60_span_ns(lo, hi)),
    }
}

/// The shipped 64-bit multiply helper on `(lo, hi, 1_000_000_000, 0)`, as
/// used by `0x0060a310`.
pub fn a60_span_ns(lo: u32, hi: u32) -> u64 {
    (((hi as u64) << 32) | lo as u64).wrapping_mul(1_000_000_000)
}

/// `0x0060a4d0` (`FUN_0060a4d0`, 208 bytes): initialise a record with mode
/// byte `+0x29 = 1` and move a grown temporary string into `name`.
///
/// The temporary starts as the empty header and receives the capacity
/// request with the source length at `+0x04`.  The move helper then copies the
/// temporary's data word, length (always zero here) and
/// `capacity & 0x7fffffff` into the record, so the high flag bit is cleared
/// on the destination.  For the shipped empty capacity the growth step
/// yields `max(0x16, requested)` for requests above `0xf`, hence the
/// `0x16` floor in the tests.
pub fn a60_record_init_a4d0(requested: u32) -> A60RecordInit {
    let mut record = a60_record_skeleton(1);
    let grown = a60_grown_capacity(A60_STRING_CAP, requested);
    record.name.cap = grown & A60_STRING_CAP_MASK;
    A60RecordInit {
        record,
        requested_capacity: requested,
        span_ns: None,
    }
}

/// `0x0060a5e0` (`FUN_0060a5e0`, 195 bytes): initialise a record with mode
/// byte `+0x29 = 0` and request capacity for the source dword at `+0x04`.
pub fn a60_record_init_a5e0(requested: u32) -> A60RecordInit {
    let mut record = a60_record_skeleton(0);
    record.name.cap = a60_grown_capacity(record.name.cap, requested);
    A60RecordInit {
        record,
        requested_capacity: requested,
        span_ns: None,
    }
}

/// Result of `0x0060a820`: one initialised string plus the source length
/// requested through the shipped capacity helper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A60StringInit {
    pub string: A60String,
    pub requested_capacity: u32,
}

/// `0x0060a820` (`FUN_0060a820`, 44 bytes): initialise one string header and
/// request capacity `src[1]` (`+0x04`).  Only the four header stores and the
/// first inline byte are in this row.
pub fn a60_string_init_a820(requested: u32) -> A60StringInit {
    let string = A60String {
        len: 0,
        cap: a60_grown_capacity(A60_STRING_CAP, requested),
        first: 0,
    };
    A60StringInit {
        string,
        requested_capacity: requested,
    }
}

/// `0x0060a870` (`FUN_0060a870`, 7 bytes): the 64-bit member at `+0x38`.
/// The record stores that range as `words30[2]` (low dword) and
/// `words30[3]` (high dword).
impl A60Record {
    /// `0x0060a870`: `return *(u64 *)(this + 0x38)`.
    pub fn a60_qword_38(&self) -> u64 {
        (self.words30[2] as u64) | ((self.words30[3] as u64) << 32)
    }

    /// `0x0060ad70` (`FUN_0060ad70`, 4 bytes): byte member at `+0x6c`.
    pub fn a60_byte_6c(&self) -> u8 {
        self.byte6c
    }

    /// `0x0060af90` (`FUN_0060af90`, 4 bytes): mode byte at `+0x29`.
    pub fn a60_flag_29(&self) -> u8 {
        self.flag29
    }
}

// ---------------------------------------------------------------------------
// 0x0060a950 - basename length
// ---------------------------------------------------------------------------

/// Maximum copied basename bytes (`0x104 - 1` in `0x0060a950`).
pub const A60_BASENAME_MAX: u32 = 0x103;

/// `0x0060a950` (`FUN_0060a950`, 113 bytes): length of the basename of a
/// NUL-terminated path.
///
/// The shipped helper scans for the last `/` or `\`, computes the distance
/// to the terminating NUL, clamps it to `0x103` (the destination buffer is
/// `0x104` bytes), and passes that length to the string capacity request.
/// Bytes after the first NUL are never examined.
pub fn a60_basename_len(path: &[u8]) -> u32 {
    let end = path.iter().position(|&b| b == 0).unwrap_or(path.len());
    let head = &path[..end];
    let start = head
        .iter()
        .rposition(|&b| b == b'/' || b == b'\\')
        .map(|i| i + 1)
        .unwrap_or(0);
    (head.len() - start).min(A60_BASENAME_MAX as usize) as u32
}

// ---------------------------------------------------------------------------
// 0x0060a880 / 0x0060b0f0 - chained hash table
// ---------------------------------------------------------------------------

/// One chained bucket node.  `+0x00` next, `+0x04` key data, `+0x08` key
/// length and `+0x20` the value returned by `0x0060a880`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A60MapNode<'a> {
    pub key: &'a [u8],
    /// Index into the node array, or `None` for the end of the chain.
    pub next: Option<usize>,
    /// `+0x20`: associated value.
    pub value: u32,
}

/// MurmurHash3 x86-32, seed 0, exactly as the shipped hash routine computes
/// it.
///
/// The decompiler's split multiplies are the usual constants:
/// `0x16a88000` / `-0x3361d2af` form `0xcc9e2d51`, and
/// `0xfaddaf14` plus a `*5` is `0xe6546b64`.  The finaliser is
/// `0x85ebca6b` then `0xc2b2ae35`, with the length xor before it.
pub fn a60_hash_bytes(bytes: &[u8]) -> u32 {
    const C1: u32 = 0xcc9e_2d51;
    const C2: u32 = 0x1b87_3593;

    let mut hash: u32 = 0;
    let mut offset = 0usize;
    while offset + 4 <= bytes.len() {
        let mut k = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        k = k.wrapping_mul(C1).rotate_left(15).wrapping_mul(C2);
        hash ^= k;
        hash = hash
            .rotate_left(13)
            .wrapping_mul(5)
            .wrapping_add(0xe654_6b64);
        offset += 4;
    }
    if offset < bytes.len() {
        let mut k: u32 = 0;
        for (shift, byte) in bytes[offset..].iter().enumerate() {
            k |= (*byte as u32) << (8 * shift);
        }
        k = k.wrapping_mul(C1).rotate_left(15).wrapping_mul(C2);
        hash ^= k;
    }
    hash ^= bytes.len() as u32;
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x85eb_ca6b);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(0xc2b2_ae35);
    hash ^= hash >> 16;
    hash
}

/// `(bucket_count - 1) & hash`, the bucket-index form used by the hash rows
/// in this window.
pub fn a60_bucket_index(bucket_count: u32, key: &[u8]) -> u32 {
    bucket_count.wrapping_sub(1) & a60_hash_bytes(key)
}

/// Comparison step inside `0x0060b0f0`: the shipped loop.
///
/// The shipped loop compares `min(len_search, len_stored)` bytes four at a
/// time, then a byte at a time, returning `-1`/`1` on the first differing
/// byte.  When the common prefix is equal it returns
/// `len_search - len_stored` as an unsigned dword.  The caller only tests
/// zero, so a match requires both equal bytes and equal lengths.
pub fn a60_key_compare(search: &[u8], stored: &[u8]) -> i32 {
    let common = search.len().min(stored.len());
    for i in 0..common {
        if search[i] != stored[i] {
            return if search[i] < stored[i] { -1 } else { 1 };
        }
    }
    (search.len() as u32).wrapping_sub(stored.len() as u32) as i32
}

/// `0x0060b0f0` (`FUN_0060b0f0`, 190 bytes): walk a chain from `head` and
/// return the matching node.
///
/// Retail returns a pointer to the link word rather than the node itself;
/// the only observable use (`0x0060a880`) dereferences that link and then
/// reads node `+0x20`, so the translation reports the matching node index.
/// `None` covers both "bucket empty" and "chain exhausted".
pub fn a60_chain_find(nodes: &[A60MapNode<'_>], head: Option<usize>, key: &[u8]) -> Option<usize> {
    let mut cursor = head;
    while let Some(index) = cursor {
        let node = nodes.get(index)?;
        if a60_key_compare(key, node.key) == 0 {
            return Some(index);
        }
        cursor = node.next;
    }
    None
}

/// `0x0060a880` (`FUN_0060a880`, 75 bytes): look up `key` in the chained
/// table and return the value stored at node `+0x20`.
///
/// The shipped body does nothing when the bucket base is null (`this[6] ==
/// 0`).  Otherwise it hashes the key with the shipped MurmurHash3 routine,
/// selects bucket
/// `(this[5] - 1) & hash`, walks the chain, and returns
/// `*(node + 0x20)` when the link is non-null.  `None` is the shipped
/// fallback: the caller keeps its original key pointer.
pub fn a60_map_lookup<'a>(
    buckets: &[Option<usize>],
    nodes: &'a [A60MapNode<'a>],
    key: &[u8],
) -> Option<&'a A60MapNode<'a>> {
    if buckets.is_empty() {
        return None;
    }
    let slot = a60_bucket_index(buckets.len() as u32, key) as usize;
    let head = *buckets.get(slot)?;
    let index = a60_chain_find(nodes, head, key)?;
    nodes.get(index)
}

// ---------------------------------------------------------------------------
// 0x0060b1b0 - 32-byte record flag builder
// ---------------------------------------------------------------------------

/// `0x0060b1b0` (`FUN_0060b1b0`, 293 bytes): build one dword of flags from a
/// 32-byte record.
///
/// The first loop counts the low bit of the sixteen little-endian words
/// (`pbVar[-4]`, `pbVar[-2]`, `pbVar[0]`, `pbVar[2]` at stride 8).  The
/// second loop counts bit 2 (`u & 4`), bit 1 (`u & 2`) and bit 3 (`u & 8`)
/// over the same sixteen words.  All thresholds and bits are reproduced
/// exactly:
///
/// ```text
/// flags  = (bit0 > 0xc) ? 0x10000 : 0
/// flags |= (bit0 != 0 && bit0 < 0x10) ? 0x20000000 : 0
/// bit4 < 0xc ? (bit4 + bit2 > 0xb ? flags |= 0x2000000)
///            : flags |= 0x4000000
/// flags |= (bit3 != 0) ? 0x1000000 : 0
/// flags |= (flags & 0x10000 != 0) ? 0 : 0x40000
/// ```
pub fn a60_flags_from_record(record: &[u8; 32]) -> u32 {
    let mut bit0 = 0u32;
    let mut bit4 = 0u32;
    let mut bit2 = 0u32;
    let mut bit3 = 0u32;
    for i in 0..16 {
        let word = u16::from_le_bytes([record[i * 2], record[i * 2 + 1]]);
        if word & 1 != 0 {
            bit0 += 1;
        }
        if word & 4 != 0 {
            bit4 += 1;
        }
        if word & 2 != 0 {
            bit2 += 1;
        }
        if word & 8 != 0 {
            bit3 += 1;
        }
    }
    let mut flags = if bit0 > 0xc { 0x10000 } else { 0 };
    if bit0 != 0 && bit0 < 0x10 {
        flags |= 0x2000_0000;
    }
    if bit4 < 0xc {
        if bit4 + bit2 > 0xb {
            flags |= 0x0200_0000;
        }
    } else {
        flags |= 0x0400_0000;
    }
    if bit3 != 0 {
        flags |= 0x0100_0000;
    }
    if flags & 0x10000 == 0 {
        flags |= 0x40000;
    }
    flags
}

/// `0x0060b1b0` loop over `param_3` records: each input record advances by
/// 0x20 bytes and writes one dword to the output array.
pub fn a60_flags_from_records(records: &[[u8; 32]]) -> Vec<u32> {
    records.iter().map(a60_flags_from_record).collect()
}

// ---------------------------------------------------------------------------
// FUN_0060b4a0..=0x0060b7c0 - game speed configuration
// ---------------------------------------------------------------------------

/// Values written by `0x0060b5b0`, in dword order.
pub const A60_SPEED_DEFAULTS: [i32; 7] = [5000, 2500, 500, 1700, 100, 2000, 500];

/// Values committed when `0x0060b5f0` rejects the incoming array.  The
/// eight-byte immediate at `_DAT_00fa9040` is `0x000000fa00000190`, i.e.
/// index 4 = 400 and index 5 = 250.  Index 5 therefore differs from the
/// `0x0060b5b0` default (2000) on the reject path.
pub const A60_SPEED_FALLBACKS: [i32; 7] = [10000, 5000, 1000, 3400, 400, 250, 1000];

/// Accepted ranges for the seven fields, in dword order.
pub const A60_SPEED_BOUNDS: [(i32, i32); 7] = [
    (5000, 10000),
    (2500, 5000),
    (500, 1000),
    (1700, 3400),
    (100, 400),
    (2000, 4000),
    (500, 1000),
];

/// The seven global speed words after `0x0060b5f0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A60Speed {
    pub values: [i32; 7],
}

/// Result of the `0x0060b5f0` validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A60SpeedCommit {
    /// True when every field equalled its clamped value and the originals
    /// were committed.
    pub accepted: bool,
    /// The seven committed words.
    pub values: [i32; 7],
}

/// `0x0060b5b0` (`FUN_0060b5b0`, 56 bytes): write the seven default speed
/// words.
pub fn a60_speed_defaults() -> [i32; 7] {
    A60_SPEED_DEFAULTS
}

/// `0x0060b5f0` (`FUN_0060b5f0`, 463 bytes): validate and commit the speed
/// configuration.
///
/// Each field is first clamped to its recovered range.  Commit happens only
/// when every clamped value still equals the original; otherwise the whole
/// fallback array is used.  A null pointer also takes the fallback path.
/// The shipped tail then stores the words and calls seven consumers; the
/// stored values are the observable state modelled here.
pub fn a60_speed_commit(input: Option<&[i32; 7]>) -> A60SpeedCommit {
    if let Some(values) = input {
        let mut accepted = true;
        for i in 0..7 {
            let (low, high) = A60_SPEED_BOUNDS[i];
            let mut clamped = values[i];
            if clamped > high {
                clamped = high;
            }
            if low > clamped {
                clamped = low;
            }
            if clamped != values[i] {
                accepted = false;
            }
        }
        if accepted {
            return A60SpeedCommit {
                accepted: true,
                values: *values,
            };
        }
    }
    A60SpeedCommit {
        accepted: false,
        values: A60_SPEED_FALLBACKS,
    }
}

impl A60Speed {
    /// `0x0060b510` (`FUN_0060b510`, 6 bytes): address of the seven-word
    /// block (`&DAT_00fa904c`).  The translation returns the block itself.
    pub fn a60_base(&self) -> [i32; 7] {
        self.values
    }

    /// `0x0060b520` (`FUN_0060b520`, 6 bytes): word 6 (`DAT_00fa9064`).
    pub fn a60_index6(&self) -> i32 {
        self.values[6]
    }

    /// `0x0060b530` (`FUN_0060b530`, 24 bytes): `(int)((float)word4 * 0.8)`.
    /// The integer is rounded to `float` first; the C `0.8` literal then
    /// promotes that float to double, so the product and truncation are
    /// performed in `f64`.
    pub fn a60_index4_scaled(&self) -> i32 {
        (self.values[4] as f32 as f64 * 0.8) as i32
    }

    /// `0x0060b550` (`FUN_0060b550`, 6 bytes): word 2 (`DAT_00fa9054`).
    pub fn a60_index2(&self) -> i32 {
        self.values[2]
    }
}

// ---------------------------------------------------------------------------
// 0x0060b7c0 - elapsed time split
// ---------------------------------------------------------------------------

/// Split elapsed milliseconds into the four `InGameUI` clock fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A60Clock {
    pub days: u32,
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32,
}

/// `0x0060b7c0` (`FUN_0060b7c0`, 259 bytes): derive the clock fields and
/// replace days/hours with the values returned by the two configuration
/// reads.
///
/// The shipped code first computes `seconds = ms / 1000`, then
///
/// ```text
/// days    = seconds / 86400
/// hours   = (seconds / 3600) % 24
/// minutes = (seconds / 60) % 60
/// seconds = seconds % 60
/// ```
///
/// and finally overwrites `days` and `hours` with the results of the
/// `"InGameUI"` / `"currentTimeDays"` and `"currentTimeHours"` reads.  Those
/// reads return their supplied default when the setting is absent, so the
/// arguments here are the two read results.
pub fn a60_clock_parts(elapsed_ms: u32, configured_days: u32, configured_hours: u32) -> A60Clock {
    let total = elapsed_ms / 1000;
    A60Clock {
        days: configured_days,
        hours: configured_hours,
        minutes: (total / 60) % 60,
        seconds: total % 60,
    }
}

/// The derived (pre-configuration) values, useful for the tests and for
/// callers that need the defaults handed to the two config reads.
pub fn a60_clock_defaults(elapsed_ms: u32) -> A60Clock {
    let total = elapsed_ms / 1000;
    A60Clock {
        days: total / 86400,
        hours: (total / 3600) % 24,
        minutes: (total / 60) % 60,
        seconds: total % 60,
    }
}

// ---------------------------------------------------------------------------
// 0x0060bf20 / 0x0060bf40 - callable and short-string helpers
// ---------------------------------------------------------------------------

/// The two vtables written by `0x0060bf20`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A60FunctorVTable {
    /// `_bcCopyableFunctionImpl<...>::vftable`, written first.
    CopyableImpl,
    /// `_bcCopyableFunctionObjectImpl<...>::vftable`, written second.
    CopyableObjectImpl,
}

/// A C++ callable object: vtable word at `+0x00`, captured target at `+0x04`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A60Functor {
    pub vtable: A60FunctorVTable,
    pub target: u32,
}

/// `0x0060bf20` (`FUN_0060bf20`, 28 bytes): copy-construct the callable
/// object.
///
/// The body writes the base vtable, copies the source target from `+0x04`,
/// then overwrites the vtable with the object implementation.  The
/// intermediate vtable store is not observable afterwards; the returned
/// value has the final vtable and the copied target.
pub fn a60_functor_copy(source: A60Functor) -> A60Functor {
    A60Functor {
        vtable: A60FunctorVTable::CopyableObjectImpl,
        target: source.target,
    }
}

/// `0x0060bf40` (`FUN_0060bf40`, 84 bytes): format a short code when the
/// formatter object is non-null and the code is non-zero.
///
/// The shipped formatter-availability test returns `*object != 0`; when that
/// and `param_2 != 0` both
/// hold, the formatter fills the string and the row returns it.  Otherwise
/// the row initialises the empty header (`len 0`, `capacity 0x8000000f`,
/// first byte 0) and leaves the string empty.  The formatter's bytes are
/// supplied to the translation as `formatted`; `None` is the empty-header
/// branch.
pub fn a60_short_formatted(
    formatter_present: bool,
    code: u16,
    formatted: &[u8],
) -> Option<Vec<u8>> {
    if formatter_present && code != 0 {
        Some(formatted.to_vec())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// 0x0060bfb0 - map tile flags
// ---------------------------------------------------------------------------

/// Stride of the tile-type table at `0x011d3020 + 0x14` (`0x1a` words per
/// entry).
pub const A60_TILE_TYPE_STRIDE: usize = 0x1a;

/// Word prefix before the tile-type table (`0x14` bytes).
pub const A60_TILE_TYPE_PREFIX_WORDS: usize = 0x14 / 2;

/// `0x0060bfb0` (`FUN_0060bfb0`, 180 bytes): decode the per-subtile flags of
/// one pixel position.
///
/// The map width (words) selects a tile in the primary layer; a zero id is
/// replaced from the fallback layer.  The id's high 11 bits and low nibble
/// index the `0x1a`-word tile-type table at `0x011d3020 + 0x14`, whose word
/// selects a group of sixteen flags.  Within the group the position is
/// `(x >> 3 & 3) + ((y >> 3 & 3) + word * 4) * 4`.  Bit 2 returns 2,
/// otherwise bit 1 is returned as 0/1.
///
/// Pixel coordinates use the shipped floor division by 32 (the `+31` sign
/// fix before the arithmetic shift), reproduced with `div_euclid`.
pub fn a60_tile_flags(
    px: i32,
    py: i32,
    map_width: u16,
    primary: &[u16],
    fallback: &[u16],
    type_table: &[u16],
    flag_table: &[u16],
) -> Option<u8> {
    let tile_x = px.div_euclid(32);
    let tile_y = py.div_euclid(32);
    let tile = (map_width as u32)
        .wrapping_mul(tile_y as u32)
        .wrapping_add(tile_x as u32) as usize;
    let mut id = *primary.get(tile)?;
    if id == 0 {
        id = *fallback.get(tile)?;
    }
    let type_index = ((id >> 4 & 0x7ff) as usize) * A60_TILE_TYPE_STRIDE + (id & 0xf) as usize;
    let group = *type_table.get(A60_TILE_TYPE_PREFIX_WORDS + type_index)?;
    let sub_x = (px >> 3 & 3) as usize;
    let sub_y = (py >> 3 & 3) as usize;
    let flag_index = sub_x + (sub_y + (group as usize) * 4) * 4;
    let flags = *flag_table.get(flag_index)?;
    if flags & 4 != 0 {
        Some(2)
    } else {
        Some(((flags >> 1) & 1) as u8)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent60 {
    use super::*;

    /// `0x0060a310`: the 64-bit duration product and the complete skeleton.
    #[test]
    fn a310_initialises_the_record_and_scales_the_duration() {
        let init = a60_record_init_a310(1, 0, 0);
        let record = init.record;
        assert_eq!(record.name, a60_empty_string());
        assert_eq!(record.tag, a60_empty_string());
        assert_eq!((record.word20, record.word24, record.word28), (0, 0, 0));
        assert_eq!(record.words30, [0; 8]);
        assert_eq!((record.flag29, record.byte6c), (0, 0));
        assert_eq!(init.requested_capacity, 0);
        assert_eq!(init.span_ns, Some(1_000_000_000));

        assert_eq!(a60_span_ns(0xffff_ffff, 0), 4_294_967_295_000_000_000);
        assert_eq!(a60_span_ns(0, 1), 4_294_967_296_000_000_000);
        assert_eq!(
            a60_span_ns(0x1234_5678, 0x9abc_def0),
            8_684_754_030_815_784_960
        );
    }

    /// `0x0060a310` / `0x0060a5e0`: the capacity request grows the low 31
    /// bits while preserving the top bit.
    #[test]
    fn record_constructors_grow_the_first_string() {
        let small = a60_record_init_a310(0, 0, 15).record;
        assert_eq!(small.name.cap, 0x8000_000f);
        let at_16 = a60_record_init_a310(0, 0, 16).record;
        assert_eq!(at_16.name.cap, 0x8000_0016);
        let at_23 = a60_record_init_a310(0, 0, 23).record;
        assert_eq!(at_23.name.cap, 0x8000_0017);
        let large = a60_record_init_a5e0(0x100).record;
        assert_eq!(large.name.cap, 0x8000_0100);
        assert_eq!(large.name.len, 0);
        assert_eq!(large.flag29, 0);
    }

    /// `0x0060a4d0`: mode 1 and the move clears the temporary's top bit.
    #[test]
    fn a4d0_moves_the_grown_string_capacity() {
        let empty = a60_record_init_a4d0(0);
        assert_eq!(empty.record.name.cap, 0xf);
        assert_eq!(empty.record.flag29, 1);
        assert_eq!(empty.record.tag, a60_empty_string());

        let len_16 = a60_record_init_a4d0(16);
        assert_eq!(len_16.record.name.cap, 0x16);
        let len_100 = a60_record_init_a4d0(100);
        assert_eq!(len_100.record.name.cap, 100);
        assert_eq!(len_100.requested_capacity, 100);
        assert_eq!(len_100.span_ns, None);
    }

    /// `0x0060a820`: only the string header changes.
    #[test]
    fn a820_initialises_one_string_header() {
        let init = a60_string_init_a820(0x40);
        assert_eq!(
            init.string,
            A60String {
                len: 0,
                cap: 0x8000_0040,
                first: 0
            }
        );
        assert_eq!(init.requested_capacity, 0x40);
        assert_eq!(a60_string_init_a820(0).string, a60_empty_string());
    }

    /// `0x0060a870`, `0x0060ad70`, `0x0060af90`: the three raw getters.
    #[test]
    fn record_getters_read_the_recovered_offsets() {
        let mut record = a60_record_init_a5e0(0).record;
        record.words30[2] = 0x1122_3344;
        record.words30[3] = 0x5566_7788;
        record.byte6c = 0x5a;
        record.flag29 = 1;
        assert_eq!(record.a60_qword_38(), 0x5566_7788_1122_3344);
        assert_eq!(record.a60_byte_6c(), 0x5a);
        assert_eq!(record.a60_flag_29(), 1);
    }

    /// `0x0060a950`: last separator wins, NUL terminates, `0x103` clamps.
    #[test]
    fn basename_length_matches_the_shipped_scan() {
        assert_eq!(a60_basename_len(b"/a/b/c.txt\0"), 5);
        assert_eq!(a60_basename_len(b"C:\\dir\\file.dat\0"), 8);
        assert_eq!(a60_basename_len(b"plain\0"), 5);
        assert_eq!(a60_basename_len(b"trailing/\0"), 0);
        assert_eq!(a60_basename_len(b"\\\0"), 0);
        assert_eq!(a60_basename_len(b""), 0);
        assert_eq!(a60_basename_len(b"a/b\0c/d"), 1);
        let long = [b'x'; 300];
        assert_eq!(a60_basename_len(&long), A60_BASENAME_MAX);
    }

    /// `0x0060b0f0` / `0x0060a880`: hash bucket selection, chain walk and the
    /// value at node `+0x20`.
    #[test]
    fn map_lookup_hashes_and_walks_the_chain() {
        let keys: [&[u8]; 4] = [b"alpha", b"beta", b"gamma", b"delta"];
        let values = [11u32, 22, 33, 44];
        let mut buckets = [None; 8];
        let mut nodes = Vec::new();
        for (key, value) in keys.iter().zip(values) {
            let slot = a60_bucket_index(buckets.len() as u32, key) as usize;
            let head = buckets[slot];
            let index = nodes.len();
            nodes.push(A60MapNode {
                key,
                next: head,
                value,
            });
            buckets[slot] = Some(index);
        }
        for (key, value) in keys.iter().zip(values) {
            let found = a60_map_lookup(&buckets, &nodes, key).expect("key present");
            assert_eq!(found.value, value);
        }
        assert!(a60_map_lookup(&buckets, &nodes, b"missing").is_none());
        assert!(a60_map_lookup(&[], &nodes, b"alpha").is_none());
        assert_ne!(a60_bucket_index(8, b"alpha"), a60_bucket_index(8, b"beta"));
    }

    /// `0x0060b0f0`'s comparison: equal prefix plus length difference.
    #[test]
    fn key_compare_reports_prefix_and_length() {
        assert_eq!(a60_key_compare(b"abc", b"abc"), 0);
        assert_eq!(a60_key_compare(b"abc", b"abd"), -1);
        assert_eq!(a60_key_compare(b"abd", b"abc"), 1);
        assert_eq!(a60_key_compare(b"abcd", b"abc"), 1);
        assert_eq!(a60_key_compare(b"abc", b"abcd"), -1);
    }

    /// The shipped MurmurHash3 constants, checked against known digests.
    #[test]
    fn hash_matches_murmur3_x86_32() {
        assert_eq!(a60_hash_bytes(b""), 0x0000_0000);
        assert_eq!(a60_hash_bytes(b"a"), 0x3c25_69b2);
        assert_eq!(a60_hash_bytes(b"ab"), 0x9bbf_d75f);
        assert_eq!(a60_hash_bytes(b"abc"), 0xb3dd_93fa);
        assert_eq!(a60_hash_bytes(b"abcd"), 0x43ed_676a);
        assert_eq!(a60_hash_bytes(b"hello"), 0x248b_fa47);
        assert_eq!(a60_hash_bytes(b"alpha"), 0xa9c0_133d);
        assert_eq!(a60_hash_bytes(b"beta"), 0x7890_69a9);
    }

    /// `0x0060b1b0`: the four threshold groups and the trailing bits.
    #[test]
    fn record_flags_reproduce_all_thresholds() {
        let zero = [0u8; 32];
        assert_eq!(a60_flags_from_record(&zero), 0x0004_0000);

        let all = [0xffu8; 32];
        assert_eq!(a60_flags_from_record(&all), 0x0501_0000);

        let mut twelve_bit0 = [0u8; 32];
        for i in 0..12 {
            twelve_bit0[i * 2] = 1;
        }
        assert_eq!(a60_flags_from_record(&twelve_bit0), 0x2004_0000);

        let mut thirteen_bit0 = [0u8; 32];
        for i in 0..13 {
            thirteen_bit0[i * 2] = 1;
        }
        assert_eq!(a60_flags_from_record(&thirteen_bit0), 0x2001_0000);

        let mut one_bit3 = [0u8; 32];
        one_bit3[0] = 8;
        assert_eq!(a60_flags_from_record(&one_bit3), 0x0104_0000);

        let mut eleven_bit4_one_bit2 = [0u8; 32];
        for i in 0..11 {
            eleven_bit4_one_bit2[i * 2] = 4;
        }
        eleven_bit4_one_bit2[22] = 2;
        assert_eq!(a60_flags_from_record(&eleven_bit4_one_bit2), 0x0204_0000);

        let mut twelve_bit4 = [0u8; 32];
        for i in 0..12 {
            twelve_bit4[i * 2] = 4;
        }
        assert_eq!(a60_flags_from_record(&twelve_bit4), 0x0404_0000);
    }

    /// `0x0060b1b0`: the output array follows the input record stride.
    #[test]
    fn record_flags_batch_keeps_input_order() {
        let mut first = [0u8; 32];
        first[0] = 1;
        let second = [0xffu8; 32];
        let out = a60_flags_from_records(&[first, second]);
        assert_eq!(out, vec![0x2004_0000, 0x0501_0000]);
        assert!(a60_flags_from_records(&[]).is_empty());
    }

    /// `0x0060b5b0` / `0x0060b5f0`: defaults, boundaries and the all-or-
    /// nothing fallback.
    #[test]
    fn speed_commit_accepts_only_in_range_values() {
        assert_eq!(a60_speed_defaults(), A60_SPEED_DEFAULTS);
        let within = a60_speed_commit(Some(&A60_SPEED_DEFAULTS));
        assert!(within.accepted);
        assert_eq!(within.values, A60_SPEED_DEFAULTS);

        let high = [10000, 5000, 1000, 3400, 400, 2000, 1000];
        assert_eq!(
            a60_speed_commit(Some(&high)),
            A60SpeedCommit {
                accepted: true,
                values: high
            }
        );

        let mut too_high = A60_SPEED_DEFAULTS;
        too_high[0] = 10001;
        let rejected = a60_speed_commit(Some(&too_high));
        assert!(!rejected.accepted);
        assert_eq!(rejected.values, A60_SPEED_FALLBACKS);

        let mut too_low = A60_SPEED_DEFAULTS;
        too_low[5] = 1999;
        assert_eq!(
            a60_speed_commit(Some(&too_low)),
            A60SpeedCommit {
                accepted: false,
                values: A60_SPEED_FALLBACKS
            }
        );

        assert_eq!(
            a60_speed_commit(None),
            A60SpeedCommit {
                accepted: false,
                values: A60_SPEED_FALLBACKS
            }
        );
        assert_eq!(A60_SPEED_FALLBACKS[5], 250);
    }

    /// `0x0060b510`, `0x0060b520`, `0x0060b530`, `0x0060b550`.
    #[test]
    fn speed_getters_read_the_committed_words() {
        let speed = A60Speed {
            values: [5000, 2500, 500, 1700, 400, 2000, 500],
        };
        assert_eq!(speed.a60_base(), speed.values);
        assert_eq!(speed.a60_index6(), 500);
        assert_eq!(speed.a60_index2(), 500);
        assert_eq!(speed.a60_index4_scaled(), 320);

        let other = A60Speed {
            values: [1, 2, 3, 4, 333, 6, 7],
        };
        assert_eq!(other.a60_index4_scaled(), 266);
        assert_eq!(other.a60_index6(), 7);
        assert_eq!(other.a60_index2(), 3);
    }

    /// `0x0060b7c0`: the four-field split and the two config overrides.
    #[test]
    fn clock_split_and_configuration() {
        assert_eq!(
            a60_clock_defaults(0),
            A60Clock {
                days: 0,
                hours: 0,
                minutes: 0,
                seconds: 0
            }
        );
        assert_eq!(a60_clock_defaults(999).seconds, 0);
        assert_eq!(a60_clock_defaults(1_000).seconds, 1);
        assert_eq!(a60_clock_defaults(60_000).minutes, 1);
        assert_eq!(a60_clock_defaults(86_399_000).hours, 23);
        assert_eq!(a60_clock_defaults(86_399_000).minutes, 59);
        assert_eq!(a60_clock_defaults(86_399_000).seconds, 59);
        assert_eq!(
            a60_clock_defaults(90_061_000),
            A60Clock {
                days: 1,
                hours: 1,
                minutes: 1,
                seconds: 1
            }
        );
        assert_eq!(
            a60_clock_defaults(u32::MAX),
            A60Clock {
                days: 49,
                hours: 17,
                minutes: 2,
                seconds: 47
            }
        );
        assert_eq!(
            a60_clock_parts(90_061_000, 7, 9),
            A60Clock {
                days: 7,
                hours: 9,
                minutes: 1,
                seconds: 1
            }
        );
        let defaults = a60_clock_defaults(90_061_000);
        assert_eq!(
            a60_clock_parts(90_061_000, defaults.days, defaults.hours),
            defaults
        );
    }

    /// `0x0060bf20`: the target is copied and the final vtable is the object
    /// implementation.
    #[test]
    fn functor_copy_uses_the_object_vtable() {
        let source = A60Functor {
            vtable: A60FunctorVTable::CopyableImpl,
            target: 0xdead_beef,
        };
        let copy = a60_functor_copy(source);
        assert_eq!(copy.vtable, A60FunctorVTable::CopyableObjectImpl);
        assert_eq!(copy.target, 0xdead_beef);
        assert_eq!(source.target, 0xdead_beef);
    }

    /// `0x0060bf40`: only a present formatter and a non-zero code produce
    /// text; every other path leaves the empty header.
    #[test]
    fn short_formatting_requires_both_inputs() {
        assert_eq!(a60_short_formatted(true, 0, b"x"), None);
        assert_eq!(a60_short_formatted(false, 5, b"x"), None);
        assert_eq!(
            a60_short_formatted(true, 5, b"oops"),
            Some(b"oops".to_vec())
        );
    }

    /// `0x0060bfb0`: primary/fallback layer selection, 0x1a table stride and
    /// the bottom four bits of the flag word.
    #[test]
    fn tile_flags_follow_the_recovered_tables() {
        let width = 4u16;
        let mut primary = vec![0u16; 16];
        let mut fallback = vec![0u16; 16];
        let mut types = vec![0u16; A60_TILE_TYPE_PREFIX_WORDS + A60_TILE_TYPE_STRIDE * 3];
        let mut flags = vec![0u16; 64];

        // Pixel (32, 64) -> tile (1, 2) -> index 9; id 0x21 -> high 2, low 1.
        primary[9] = 0x21;
        let type_index = A60_TILE_TYPE_PREFIX_WORDS + 2 * A60_TILE_TYPE_STRIDE + 1;
        types[type_index] = 3;
        let flag_index = 0 + (0 + 3 * 4) * 4;
        flags[flag_index] = 4;
        assert_eq!(
            a60_tile_flags(32, 64, width, &primary, &fallback, &types, &flags),
            Some(2)
        );

        flags[flag_index] = 2;
        assert_eq!(
            a60_tile_flags(32, 64, width, &primary, &fallback, &types, &flags),
            Some(1)
        );
        flags[flag_index] = 0;
        assert_eq!(
            a60_tile_flags(32, 64, width, &primary, &fallback, &types, &flags),
            Some(0)
        );

        // A zero primary id falls back to the second layer.
        primary[9] = 0;
        fallback[9] = 0x21;
        flags[flag_index] = 4;
        assert_eq!(
            a60_tile_flags(32, 64, width, &primary, &fallback, &types, &flags),
            Some(2)
        );

        // Out-of-range positions and short tables are rejected.
        assert_eq!(
            a60_tile_flags(-1, 0, width, &primary, &fallback, &types, &flags),
            None
        );
        assert_eq!(
            a60_tile_flags(32, 64, width, &primary, &fallback, &types[..10], &flags),
            None
        );
        assert_eq!(
            a60_tile_flags(32, 64, width, &primary, &fallback, &types, &flags[..16]),
            None
        );
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// * FUN_0060a050, FUN_0060a070, FUN_0060a8d0: the recovered body's only remaining
//   work is an indirect call / exception-cleanup sequence whose target is not
//   part of the batch; no return value survives the decompiler's noreturn
//   marker.
// * FUN_0060a204, FUN_0060a416, FUN_0060a5a8: one-byte / mid-instruction
//   fragments; the bodies begin inside a previous instruction stream.
// * FUN_0060a417, FUN_0060a780, FUN_0060aa00, FUN_0060aa80, FUN_0060aa90,
//   FUN_0060aaf0, FUN_0060ad80, FUN_0060afa0, FUN_0060b020, FUN_0060b2e0,
//   FUN_0060b4a0, FUN_0060b990, FUN_0060bcb0: construction tails, one-time
//   initialisers or probe helpers whose real work is an indirect call into a
//   routine outside 0x0060a000..=0x0060bfff; translating only the wrapper
//   would claim behaviour the ledger does not contain.
// * FUN_0060af0a is `int3`; FUN_0060b980 is a five-byte thunk; FUN_0060bd70 is
//   `decompile_failed`; FUN_0060b560 was already translated outside this batch.
