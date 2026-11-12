#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave13/agent06.rs
// ===========================================================================
// Wave 13 translation batch 06: the fade-state helpers and the small
// string-keyed variant map in `0x00666000..=0x00667fff`.
//
// Nineteen `decompiled_uncited` rows are translated below.  The window is
// dominated by two object families:
//
// * a three-field fade accumulator (`+0x00` active byte, `+0x10` elapsed
//   `f32`, `+0x14` secondary `f32`, `+0x1a` tail byte) whose reset/tick
//   bodies are fully recovered; and
// * an MSVC-style chained hash map whose nodes hold a string key at
//   `+0x08` and a 16-byte tagged value at `+0x28`.  The header layout, the
//   MurmurHash3 bucket selection and the load-factor growth path are
//   reproduced below; the tagged value keeps its tag at `+0x00` and its
//   payload at `+0x08`.
//
// ## Recovered map layout
//
// | offset | field |
// |--------|-------|
// | `+0x00` | bucket count (power of two, starts at `8`) |
// | `+0x04` | bucket-head array pointer (`0` until the first insert) |
// | `+0x08` | live entry count |
// | `+0x0c` | load limit, `1.0f` |
// | `+0x10` | comparison mode (`0` selects the stored mode) |
// | `+0x14` | generation counter |
//
// | node offset | field |
// |-------------|-------|
// | `+0x00` | next pointer |
// | `+0x08` | key: MSVC string header (`+0x08` data, `+0x0c` length, `+0x10` capacity, inline buffer at `+0x14`) |
// | `+0x28` | value: tagged value (`+0x28` tag, `+0x30..+0x37` payload) |
//
// Tag semantics recovered from the equality dispatcher reached by the
// shared tag comparator: tag `0` compares true unconditionally, tag `1`
// compares string bytes, tags `2`/`3` compare both payload dwords, tag `4`
// compares a little-endian `f64`, and tag `5` compares the first payload
// byte.
//
// Nothing here touches `crate::sim::World` or `SimUnit`; all
// routines are free functions over plain data and caller-supplied
// visitors, and the module adds no save/replay state.

/// Bucket count written by both map constructors.
pub const W13A06_DEFAULT_BUCKETS: u32 = 8;

/// Load limit stored at map `+0x0c`.
pub const W13A06_LOAD_LIMIT: f32 = 1.0;

/// Mode selected by a zero argument to the constructors.
pub const W13A06_DEFAULT_MODE: u32 = 1;

/// Generation value written by both map constructors.
pub const W13A06_INITIAL_GENERATION: u32 = 1;

/// Global table address returned by the getter at `0x00666370`.
pub const W13A06_GLOBAL_TABLE_ADDRESS: u32 = 0x00fe_5918;

/// Tag for the empty/always-equal value.
pub const W13A06_TAG_EMPTY: u32 = 0;
/// Tag compared as string bytes.
pub const W13A06_TAG_TEXT: u32 = 1;
/// Tag written by the first pair setter.
pub const W13A06_TAG_PAIR_A: u32 = 2;
/// Tag written by the second pair setter.
pub const W13A06_TAG_PAIR_B: u32 = 3;
/// Tag compared as an `f64`.
pub const W13A06_TAG_QWORD: u32 = 4;
/// Tag written by the byte setter.
pub const W13A06_TAG_BYTE: u32 = 5;

// ---------------------------------------------------------------------------
// Fade accumulator (0x00666390, 0x006663d0, 0x006663f0)
// ---------------------------------------------------------------------------

/// Recovered fade accumulator.
///
/// `0x006663f0` reads `+0x00`, updates only `+0x10`, and compares `+0x10`
/// and `+0x14`; the reset rows also clear `+0x1a`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct W13a06FadeState {
    /// `+0x00`: non-zero while the accumulator is running.
    pub active: u8,
    /// `+0x10`: elapsed time, advanced by `0.1f32` per tick.
    pub elapsed: f32,
    /// `+0x14`: secondary timer compared against `1.0`.
    pub secondary: f32,
    /// `+0x1a`: tail byte cleared by both reset rows.
    pub tail: u8,
}

/// `0x00666390`: reset the accumulator to the inactive state.
///
/// ```c
/// *(u32 *)(this + 0x10) = 0;
/// *(u32 *)(this + 0x14) = 0;
/// *(u8  *)(this + 0x00) = 0;
/// *(u8  *)(this + 0x1a) = 0;
/// ```
pub fn w13a06_fade_reset_inactive() -> W13a06FadeState {
    W13a06FadeState {
        active: 0,
        elapsed: 0.0,
        secondary: 0.0,
        tail: 0,
    }
}

/// `0x006663d0`: reset the accumulator and mark it active.
///
/// Byte-for-byte the same body as [`w13a06_fade_reset_inactive`] except for
/// the `+0x00` store, which is `1`.
pub fn w13a06_fade_reset_active() -> W13a06FadeState {
    W13a06FadeState {
        active: 1,
        elapsed: 0.0,
        secondary: 0.0,
        tail: 0,
    }
}

/// `0x006663f0`: advance the running accumulator by `0.1f32` and report
/// whether any of the three trigger comparisons fired.
///
/// The shipped comparisons are `elapsed >= 1.0`, `secondary >= 1.0`, or
/// `secondary + elapsed >= 1.5`.  An inactive accumulator is not advanced
/// and reports `false`.  The `f32` arithmetic and comparison order match the
/// `addss`/`comiss` sequence in the shipped tick body.
///
/// When a comparison fires, the shipped body tail-jumps to the fade-complete
/// handler; this reference helper reports the predicate so a caller can model
/// that hand-off without the out-of-window handler.
pub fn w13a06_fade_tick(state: &mut W13a06FadeState) -> bool {
    if state.active == 0 {
        return false;
    }
    state.elapsed += 0.1f32;
    state.elapsed >= 1.0f32
        || state.secondary >= 1.0f32
        || state.secondary + state.elapsed >= 1.5f32
}

// ---------------------------------------------------------------------------
// Tagged value (tag at +0x00, 8-byte payload at +0x08)
// ---------------------------------------------------------------------------

/// A tagged value stored at map node `+0x28`.
///
/// The retail value occupies more than the modelled bytes; only the tag and
/// the first eight payload bytes are observed by the translated rows.
/// `text` carries the inline string copied by the tag-0 constructor for
/// tag-0 values; tag-0 equality ignores it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W13a06Value {
    /// `+0x00`: active union member.
    pub tag: u32,
    /// `+0x08`: first eight payload bytes.
    pub payload: [u8; 8],
    /// String copied by the tag-0 constructor, when present.
    pub text: Option<String>,
}

/// Tag-0 value: empty and unconditionally equal to every other tag-0 value.
pub fn w13a06_value_empty() -> W13a06Value {
    W13a06Value::default()
}

/// Tag-0 value carrying a copy of `text`, matching the shipped tag-0
/// constructor.
///
/// That constructor also writes the inline-string header (`+0x08` data,
/// `+0x0c` length, `+0x10` capacity `0x8000000f`, `+0x14` first byte) and a
/// NUL terminator.  The translated map only retains the text because tag-0
/// equality returns true without inspecting the payload.
pub fn w13a06_value_empty_with_text(text: &str) -> W13a06Value {
    W13a06Value {
        tag: W13A06_TAG_EMPTY,
        payload: [0; 8],
        text: Some(text.to_owned()),
    }
}

/// Tag-5 byte value, matching the shipped byte setter.
///
/// Only payload byte `0` is written by the shipped setter; the remaining
/// bytes are left undefined there.  This constructor zeroes them so the
/// translated value is deterministic, and equality for tag `5` reads only
/// byte `0`, exactly like the recovered comparator.
pub fn w13a06_value_byte(byte: u8) -> W13a06Value {
    let mut payload = [0u8; 8];
    payload[0] = byte;
    W13a06Value {
        tag: W13A06_TAG_BYTE,
        payload,
        text: None,
    }
}

/// Tag-2 pair value, matching the first shipped pair setter.
pub fn w13a06_value_pair_a(low: u32, high: u32) -> W13a06Value {
    let mut payload = [0u8; 8];
    payload[0..4].copy_from_slice(&low.to_le_bytes());
    payload[4..8].copy_from_slice(&high.to_le_bytes());
    W13a06Value {
        tag: W13A06_TAG_PAIR_A,
        payload,
        text: None,
    }
}

/// Tag-3 pair value, matching the second shipped pair setter.
pub fn w13a06_value_pair_b(low: u32, high: u32) -> W13a06Value {
    let mut payload = [0u8; 8];
    payload[0..4].copy_from_slice(&low.to_le_bytes());
    payload[4..8].copy_from_slice(&high.to_le_bytes());
    W13a06Value {
        tag: W13A06_TAG_PAIR_B,
        payload,
        text: None,
    }
}

/// Recovered tag dispatcher as a pure equality predicate.
///
/// * tag `0`: `true` without reading the payload;
/// * tag `1`: compare the two strings (falling back to payload bytes when
///   the translated value has no text);
/// * tags `2`, `3`: compare both payload dwords;
/// * tag `4`: compare a little-endian `f64`;
/// * tag `5`: compare payload byte `0`;
/// * any other tag: compare the full payload.
pub fn w13a06_value_equals(a: &W13a06Value, b: &W13a06Value) -> bool {
    if a.tag != b.tag {
        return false;
    }
    match a.tag {
        W13A06_TAG_EMPTY => true,
        W13A06_TAG_TEXT => match (&a.text, &b.text) {
            (Some(left), Some(right)) => left == right,
            _ => a.payload == b.payload,
        },
        W13A06_TAG_QWORD => f64::from_le_bytes(a.payload) == f64::from_le_bytes(b.payload),
        W13A06_TAG_BYTE => a.payload[0] == b.payload[0],
        _ => a.payload == b.payload,
    }
}

/// Two dwords of a tag-2/tag-3 pair value.
pub fn w13a06_value_pair(value: &W13a06Value) -> Option<(u32, u32)> {
    if value.tag != W13A06_TAG_PAIR_A && value.tag != W13A06_TAG_PAIR_B {
        return None;
    }
    let low = u32::from_le_bytes([
        value.payload[0],
        value.payload[1],
        value.payload[2],
        value.payload[3],
    ]);
    let high = u32::from_le_bytes([
        value.payload[4],
        value.payload[5],
        value.payload[6],
        value.payload[7],
    ]);
    Some((low, high))
}

/// Payload byte `0` of a tag-5 value.
pub fn w13a06_value_byte_of(value: &W13a06Value) -> Option<u8> {
    (value.tag == W13A06_TAG_BYTE).then_some(value.payload[0])
}

// ---------------------------------------------------------------------------
// Hash-map model
// ---------------------------------------------------------------------------

/// One node in a bucket chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W13a06Entry {
    /// `+0x00`: next node in the same bucket.
    pub next: Option<usize>,
    /// `false` once the node has been unlinked by the erase path.
    pub alive: bool,
    /// `+0x08`: string key.
    pub key: String,
    /// `+0x28`: tagged value.
    pub value: W13a06Value,
}

/// Recovered map header plus the translated node arena.
#[derive(Clone, Debug, PartialEq)]
pub struct W13a06Map {
    /// `+0x00`: bucket count.
    pub bucket_count: u32,
    /// `+0x04`: bucket heads; empty until the first insert allocates them.
    pub buckets: Vec<Option<usize>>,
    /// `+0x08`: live node count.
    pub size: u32,
    /// `+0x0c`: load limit.
    pub load_limit: f32,
    /// `+0x10`: stored comparison mode.
    pub mode: u32,
    /// `+0x14`: generation counter.
    pub generation: u32,
    /// Node arena.  Dead nodes stay in place so chain indices remain valid.
    pub entries: Vec<W13a06Entry>,
}

/// Iterator state copied by `0x00667530`: current node plus the bucket that
/// produced it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a06Cursor {
    /// Bucket index currently being walked.
    pub bucket: u32,
    /// Current chain node, `None` once the iterator is exhausted.
    pub current: Option<usize>,
}

/// Snapshot returned by [`w13a06_iter_next`]: the pre-advance state and the
/// one-past-the-end bucket index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a06CursorSnapshot {
    /// Bucket index before advancing.
    pub bucket: u32,
    /// Current node before advancing.
    pub current: Option<usize>,
    /// `bucket_count`, the end of the bucket array.
    pub end: u32,
}

/// `0x00667390`: construct a map with eight unallocated buckets, a `1.0`
/// load limit and generation `1`.
///
/// A zero `mode` argument selects the default mode `1`; any other value is
/// stored unchanged.
pub fn w13a06_map_new(mode: u32) -> W13a06Map {
    W13a06Map {
        bucket_count: W13A06_DEFAULT_BUCKETS,
        buckets: Vec::new(),
        size: 0,
        load_limit: W13A06_LOAD_LIMIT,
        mode: if mode == 0 { W13A06_DEFAULT_MODE } else { mode },
        generation: W13A06_INITIAL_GENERATION,
        entries: Vec::new(),
    }
}

/// `0x00667310`: copy constructor.
///
/// The destination starts exactly like [`w13a06_map_new`] with mode `1` and
/// then runs the clear-and-copy body at `0x00667a10`, which adopts the source
/// mode and bumps the generation from `1` to `2` when the source is empty.
pub fn w13a06_map_from(source: &W13a06Map) -> W13a06Map {
    let mut map = w13a06_map_new(W13A06_DEFAULT_MODE);
    w13a06_assign_from(&mut map, source);
    map
}

/// Bucket-order walk over the live nodes of `map`.
fn w13a06_entry_order(map: &W13a06Map) -> Vec<usize> {
    let mut order = Vec::with_capacity(map.size as usize);
    for bucket in 0..map.buckets.len() {
        let mut cursor = map.buckets[bucket];
        while let Some(index) = cursor {
            if map.entries[index].alive {
                order.push(index);
            }
            cursor = map.entries[index].next;
        }
    }
    order
}

/// MurmurHash3 x86-32 with seed `0`, as called by the map's find and
/// lookup helpers.
fn w13a06_hash(bytes: &[u8]) -> u32 {
    const C1: u32 = 0xcc9e_2d51;
    const C2: u32 = 0x1b87_3593;
    let mut hash: u32 = 0;
    let mut chunks = bytes.chunks_exact(4);
    for chunk in &mut chunks {
        let mut word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        word = word.wrapping_mul(C1).rotate_left(15).wrapping_mul(C2);
        hash ^= word;
        hash = hash
            .rotate_left(13)
            .wrapping_mul(5)
            .wrapping_add(0xe654_6b64);
    }
    let tail = chunks.remainder();
    let mut word: u32 = 0;
    if tail.len() >= 3 {
        word ^= (tail[2] as u32) << 16;
    }
    if tail.len() >= 2 {
        word ^= (tail[1] as u32) << 8;
    }
    if !tail.is_empty() {
        word ^= tail[0] as u32;
        word = word.wrapping_mul(C1).rotate_left(15).wrapping_mul(C2);
        hash ^= word;
    }
    hash ^= bytes.len() as u32;
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x85eb_ca6b);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(0xc2b2_ae35);
    hash ^= hash >> 16;
    hash
}

/// Bucket index for `key`: `hash(key) & (bucket_count - 1)`.
fn w13a06_bucket(map: &W13a06Map, key: &str) -> usize {
    (w13a06_hash(key.as_bytes()) as usize) & (map.bucket_count as usize).wrapping_sub(1)
}

/// Live nodes whose key is byte-for-byte equal to `key`.
fn w13a06_find_indices(map: &W13a06Map, key: &str) -> Vec<usize> {
    let mut found = Vec::new();
    if map.buckets.is_empty() {
        return found;
    }
    let bucket = w13a06_bucket(map, key);
    let mut cursor = map.buckets[bucket];
    while let Some(index) = cursor {
        let entry = &map.entries[index];
        if entry.alive && entry.key == key {
            found.push(index);
        }
        cursor = entry.next;
    }
    found
}

/// Rehash every live node into a bucket array of `new_count` heads.
fn w13a06_rehash(map: &mut W13a06Map, new_count: u32) {
    map.bucket_count = new_count;
    map.buckets = vec![None; new_count as usize];
    for index in 0..map.entries.len() {
        if !map.entries[index].alive {
            continue;
        }
        let bucket = w13a06_bucket(map, &map.entries[index].key);
        map.entries[index].next = map.buckets[bucket];
        map.buckets[bucket] = Some(index);
    }
}

/// `0x00667180`: insert one key/value pair at the bucket head, increment the
/// live count, and double the bucket array when the load exceeds the stored
/// limit.
///
/// Ghidra's 21-byte listing stops at the CRT allocator call
/// (`FUN_0093ec70`); the allocation of the bucket array, the node/string
/// copies, the MurmurHash3 bucket selection and the load-factor growth below
/// are read from the rest of the shipped instruction stream.
pub fn w13a06_insert_entry(map: &mut W13a06Map, key: &str, value: W13a06Value) {
    if map.buckets.is_empty() {
        map.buckets = vec![None; map.bucket_count as usize];
    }
    let bucket = w13a06_bucket(map, key);
    let index = map.entries.len();
    map.entries.push(W13a06Entry {
        next: map.buckets[bucket],
        alive: true,
        key: key.to_owned(),
        value,
    });
    map.buckets[bucket] = Some(index);
    map.size = map.size.wrapping_add(1);
    let load = ((map.size as f64) / (map.bucket_count as f64)) as f32;
    if load > map.load_limit {
        let doubled = map.bucket_count.wrapping_mul(2);
        w13a06_rehash(map, doubled);
    }
}

/// Unlink and retire every live node whose key equals `key`; returns the
/// number removed.  This is the erase body used by the key-erase and mode-1
/// paths.
fn w13a06_erase_all(map: &mut W13a06Map, key: &str) -> u32 {
    let mut removed = 0u32;
    for bucket in 0..map.buckets.len() {
        let mut previous: Option<usize> = None;
        let mut cursor = map.buckets[bucket];
        while let Some(index) = cursor {
            let next = map.entries[index].next;
            if map.entries[index].key == key {
                match previous {
                    Some(link) => map.entries[link].next = next,
                    None => map.buckets[bucket] = next,
                }
                map.entries[index].alive = false;
                map.entries[index].next = None;
                map.size = map.size.wrapping_sub(1);
                removed += 1;
            } else {
                previous = Some(index);
            }
            cursor = next;
        }
    }
    removed
}

/// `0x006678b0`: mode-selectable insert.
///
/// A zero `mode` argument selects `map.mode`.  Then:
///
/// * mode `1`: if the map already holds exactly one entry for `key` and its
///   value equals `value`, return without touching the map; otherwise erase
///   every `key` match (bumping the generation when anything was removed),
///   insert the new pair and bump once more;
/// * mode `2`: assign `value` to every existing `key` match and bump once,
///   or insert and bump twice when there is no match;
/// * any other mode: insert and bump twice.
pub fn w13a06_insert(map: &mut W13a06Map, key: &str, value: W13a06Value, mode: u32) {
    let effective = if mode == 0 { map.mode } else { mode };
    match effective {
        1 => {
            let matches = w13a06_find_indices(map, key);
            if !matches.is_empty() {
                let mut equal = 0u32;
                let mut different = 0u32;
                for index in matches {
                    if w13a06_value_equals(&map.entries[index].value, &value) {
                        equal += 1;
                    } else {
                        different += 1;
                    }
                }
                if equal == 1 && different == 0 {
                    return;
                }
            }
            if w13a06_erase_all(map, key) != 0 {
                map.generation = map.generation.wrapping_add(1);
            }
            w13a06_insert_entry(map, key, value);
            map.generation = map.generation.wrapping_add(1);
        }
        2 => {
            let matches = w13a06_find_indices(map, key);
            if matches.is_empty() {
                w13a06_insert_entry(map, key, value);
                map.generation = map.generation.wrapping_add(2);
            } else {
                for index in matches {
                    map.entries[index].value = value.clone();
                }
                map.generation = map.generation.wrapping_add(1);
            }
        }
        _ => {
            w13a06_insert_entry(map, key, value);
            map.generation = map.generation.wrapping_add(2);
        }
    }
}

/// `0x00667510`: copy-assignment entry point.
///
/// The body is a single forward to `0x00667a10`, so the destination is
/// cleared and rebuilt from `source` exactly like [`w13a06_assign_from`].
pub fn w13a06_copy_assign(destination: &mut W13a06Map, source: &W13a06Map) {
    w13a06_assign_from(destination, source);
}

/// `0x00667a10`: clear the destination and rebuild it from `source`.
///
/// Order of the shipped stores:
///
/// 1. adopt `source.mode`;
/// 2. destroy every destination node and null every bucket head;
/// 3. store `0` in the live count;
/// 4. insert every source entry through `0x006678b0` with the adopted mode;
/// 5. bump the generation once.
pub fn w13a06_assign_from(destination: &mut W13a06Map, source: &W13a06Map) {
    destination.mode = source.mode;
    for entry in &mut destination.entries {
        entry.alive = false;
    }
    destination.entries.clear();
    destination.buckets.fill(None);
    destination.size = 0;
    for index in w13a06_entry_order(source) {
        let entry = &source.entries[index];
        let key = entry.key.clone();
        let value = entry.value.clone();
        w13a06_insert(destination, &key, value, destination.mode);
    }
    destination.generation = destination.generation.wrapping_add(1);
}

/// `0x006676d0`: copy every live entry of `source` into `destination` by
/// calling the mode-selectable insert `0x006678b0` once per entry.
///
/// The routine itself does not bump the destination generation; the caller
/// (`0x00667a10`) does.
pub fn w13a06_copy_entries(destination: &mut W13a06Map, source: &W13a06Map, mode: u32) {
    for index in w13a06_entry_order(source) {
        let entry = &source.entries[index];
        let key = entry.key.clone();
        let value = entry.value.clone();
        w13a06_insert(destination, &key, value, mode);
    }
}

/// Iterator positioned at the first chain node of `map` in bucket order.
///
/// This is an auxiliary for [`w13a06_iter_next`]: the shipped iterator
/// constructor used by the copy and equality callers skips empty buckets from
/// index `0`, while `0x00667530` itself only snapshots and advances.
pub fn w13a06_cursor_begin(map: &W13a06Map) -> W13a06Cursor {
    for bucket in 0..map.bucket_count {
        if let Some(head) = map.buckets.get(bucket as usize).copied().flatten() {
            return W13a06Cursor {
                bucket,
                current: Some(head),
            };
        }
    }
    W13a06Cursor {
        bucket: map.bucket_count,
        current: None,
    }
}

/// `0x00667530`: copy the 12-byte iterator state into a snapshot and advance
/// to the next chain node or bucket head.
///
/// The shipped routine writes `{current, bucket cursor, end}` to the caller's
/// output before it advances.  The translated snapshot keeps the same
/// pre-advance values and represents the bucket cursor as an index.
pub fn w13a06_iter_next(map: &W13a06Map, cursor: &mut W13a06Cursor) -> W13a06CursorSnapshot {
    let snapshot = W13a06CursorSnapshot {
        bucket: cursor.bucket,
        current: cursor.current,
        end: map.bucket_count,
    };
    if let Some(index) = cursor.current {
        if let Some(next) = map.entries[index].next {
            cursor.current = Some(next);
            return snapshot;
        }
        cursor.current = None;
    }
    loop {
        cursor.bucket = cursor.bucket.wrapping_add(1);
        if cursor.bucket >= map.bucket_count {
            break;
        }
        if let Some(head) = map.buckets.get(cursor.bucket as usize).copied().flatten() {
            cursor.current = Some(head);
            break;
        }
    }
    snapshot
}

/// `0x00667dc0`: erase every entry whose key equals `key` and bump the
/// generation when at least one entry was removed.
///
/// The shipped body calls that erase helper and tests its non-zero result.
pub fn w13a06_erase_key(map: &mut W13a06Map, key: &str) -> u32 {
    let removed = w13a06_erase_all(map, key);
    if removed != 0 {
        map.generation = map.generation.wrapping_add(1);
    }
    removed
}

// ---------------------------------------------------------------------------
// Value-building insert wrappers (0x00667580..0x00667840)
// ---------------------------------------------------------------------------

/// `0x00667580`: build a tag-5 byte value from `byte` and insert it.
pub fn w13a06_insert_byte_value(map: &mut W13a06Map, key: &str, byte: u8, mode: u32) {
    w13a06_insert(map, key, w13a06_value_byte(byte), mode);
}

/// `0x006675f0`: build a tag-2 pair value `(value, value >> 31)` and insert
/// it.  The shifted dword is the sign extension of the signed argument.
pub fn w13a06_insert_i32_value(map: &mut W13a06Map, key: &str, value: i32, mode: u32) {
    w13a06_insert(
        map,
        key,
        w13a06_value_pair_a(value as u32, (value >> 31) as u32),
        mode,
    );
}

/// `0x00667660`: build a tag-2 pair value from `low`/`high` and insert it.
pub fn w13a06_insert_pair_a_value(map: &mut W13a06Map, key: &str, low: u32, high: u32, mode: u32) {
    w13a06_insert(map, key, w13a06_value_pair_a(low, high), mode);
}

/// `0x00667760`: build a tag-0 value carrying a copy of `text` and insert it.
pub fn w13a06_insert_text_value(map: &mut W13a06Map, key: &str, text: &str, mode: u32) {
    w13a06_insert(map, key, w13a06_value_empty_with_text(text), mode);
}

/// `0x006677d0`: build a tag-3 pair value `(low, 0)` and insert it.
pub fn w13a06_insert_pair_b_low_value(map: &mut W13a06Map, key: &str, low: u32, mode: u32) {
    w13a06_insert(map, key, w13a06_value_pair_b(low, 0), mode);
}

/// `0x00667840`: build a tag-3 pair value from `low`/`high` and insert it.
pub fn w13a06_insert_pair_b_value(map: &mut W13a06Map, key: &str, low: u32, high: u32, mode: u32) {
    w13a06_insert(map, key, w13a06_value_pair_b(low, high), mode);
}

/// `0x00666370`: return the address of the global table at `0x00fe5918`.
///
/// The shipped body is the six-byte load of that constant; no field of the
/// table is read here.
pub fn w13a06_global_table_address() -> u32 {
    W13A06_GLOBAL_TABLE_ADDRESS
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_wave13_agent06 {
    use super::*;

    fn live_value<'a>(map: &'a W13a06Map, key: &str) -> Option<&'a W13a06Value> {
        map.entries
            .iter()
            .find(|entry| entry.alive && entry.key == key)
            .map(|entry| &entry.value)
    }

    fn collect(map: &W13a06Map) -> Vec<(String, W13a06Value)> {
        let mut out = Vec::new();
        for bucket in 0..map.buckets.len() {
            let mut cursor = map.buckets[bucket];
            while let Some(index) = cursor {
                let entry = &map.entries[index];
                out.push((entry.key.clone(), entry.value.clone()));
                cursor = entry.next;
            }
        }
        out
    }

    /// `0x00666390` / `0x006663d0` / `0x006663f0`.
    #[test]
    fn w13a06_fade_reset_and_tick() {
        let mut inactive = w13a06_fade_reset_inactive();
        assert_eq!(inactive.active, 0);
        assert_eq!(inactive.elapsed, 0.0);
        assert_eq!(inactive.secondary, 0.0);
        assert_eq!(inactive.tail, 0);
        assert!(!w13a06_fade_tick(&mut inactive));
        assert_eq!(inactive.elapsed, 0.0);

        let mut active = w13a06_fade_reset_active();
        assert_eq!(active.active, 1);
        assert!(!w13a06_fade_tick(&mut active));
        assert!((active.elapsed - 0.1).abs() < 1e-6);
        for _ in 0..9 {
            w13a06_fade_tick(&mut active);
        }
        assert!(active.elapsed >= 1.0);

        let mut secondary_only = w13a06_fade_reset_active();
        secondary_only.secondary = 1.0;
        assert!(w13a06_fade_tick(&mut secondary_only));

        let mut combined = w13a06_fade_reset_active();
        combined.elapsed = 0.6;
        combined.secondary = 0.85;
        assert!(w13a06_fade_tick(&mut combined));

        let mut below = w13a06_fade_reset_active();
        below.elapsed = 0.6;
        below.secondary = 0.79;
        assert!(!w13a06_fade_tick(&mut below));
    }

    /// `0x00666370`.
    #[test]
    fn w13a06_global_table_constant() {
        assert_eq!(w13a06_global_table_address(), 0x00fe_5918);
        assert_eq!(W13A06_GLOBAL_TABLE_ADDRESS, w13a06_global_table_address());
    }

    /// `0x00667180`'s bucket hash: MurmurHash3 x86-32 with seed `0`.
    #[test]
    fn w13a06_hash_matches_murmur3_vectors() {
        assert_eq!(w13a06_hash(b""), 0);
        assert_eq!(w13a06_hash(b"a"), 0x3c25_69b2);
        assert_eq!(w13a06_hash(b"abc"), 0xb3dd_93fa);
        assert_eq!(w13a06_hash(b"hello"), 0x248b_fa47);
    }

    /// `0x00667390` / `0x00667310`.
    #[test]
    fn w13a06_map_constructors() {
        let default = w13a06_map_new(0);
        assert_eq!(default.bucket_count, 8);
        assert!(default.buckets.is_empty());
        assert_eq!(default.size, 0);
        assert_eq!(default.load_limit, 1.0);
        assert_eq!(default.mode, 1);
        assert_eq!(default.generation, 1);

        let custom = w13a06_map_new(7);
        assert_eq!(custom.mode, 7);

        let mut source = w13a06_map_new(3);
        w13a06_insert_byte_value(&mut source, "alpha", 0x11, 0);
        w13a06_insert_pair_b_value(&mut source, "beta", 7, 9, 0);
        let copied = w13a06_map_from(&source);
        assert_eq!(copied.mode, 3);
        assert_eq!(copied.size, 2);
        assert_eq!(copied.bucket_count, 8);
        // `0x00667a10` bumps once, and source mode 3 takes the two-step
        // insert path for each of the two entries.
        assert_eq!(copied.generation, 6);
        assert_eq!(collect(&copied), collect(&source));
    }

    /// `0x006678b0` tag-0 equality ignores the payload.
    #[test]
    fn w13a06_value_equality_tags() {
        assert!(w13a06_value_equals(
            &w13a06_value_empty(),
            &w13a06_value_empty_with_text("different")
        ));
        assert!(!w13a06_value_equals(
            &w13a06_value_pair_a(1, 2),
            &w13a06_value_pair_a(1, 3)
        ));
        assert!(w13a06_value_equals(
            &w13a06_value_pair_a(1, 2),
            &w13a06_value_pair_a(1, 2)
        ));
        assert!(!w13a06_value_equals(
            &w13a06_value_pair_a(1, 2),
            &w13a06_value_pair_b(1, 2)
        ));
        assert!(w13a06_value_equals(
            &w13a06_value_byte(0xab),
            &w13a06_value_byte(0xab)
        ));
        assert!(!w13a06_value_equals(
            &w13a06_value_byte(0xab),
            &w13a06_value_byte(0xac)
        ));
        let quiet_nan = W13a06Value {
            tag: W13A06_TAG_QWORD,
            payload: f64::NAN.to_le_bytes(),
            text: None,
        };
        assert!(!w13a06_value_equals(&quiet_nan, &quiet_nan));
        let one = W13a06Value {
            tag: W13A06_TAG_QWORD,
            payload: 1.0f64.to_le_bytes(),
            text: None,
        };
        assert!(w13a06_value_equals(&one, &one));
    }

    /// `0x00667580`.
    #[test]
    fn w13a06_insert_byte_value_round_trip() {
        let mut map = w13a06_map_new(0);
        w13a06_insert_byte_value(&mut map, "key", 0xab, 0);
        assert_eq!(map.size, 1);
        assert_eq!(map.generation, 2);
        let entry = &map.entries[0];
        assert_eq!(entry.value.tag, W13A06_TAG_BYTE);
        assert_eq!(w13a06_value_byte_of(&entry.value), Some(0xab));
    }

    /// `0x006675f0`.
    #[test]
    fn w13a06_insert_i32_value_sign_extends() {
        let mut map = w13a06_map_new(0);
        w13a06_insert_i32_value(&mut map, "negative", -2, 0);
        w13a06_insert_i32_value(&mut map, "positive", 5, 0);
        assert_eq!(
            w13a06_value_pair(&map.entries[0].value),
            Some((0xffff_fffe, 0xffff_ffff))
        );
        assert_eq!(w13a06_value_pair(&map.entries[1].value), Some((5, 0)));
    }

    /// `0x00667660`.
    #[test]
    fn w13a06_insert_pair_a_value_round_trip() {
        let mut map = w13a06_map_new(0);
        w13a06_insert_pair_a_value(&mut map, "pair", 0x1122_3344, 0x5566_7788, 0);
        let value = &map.entries[0].value;
        assert_eq!(value.tag, W13A06_TAG_PAIR_A);
        assert_eq!(w13a06_value_pair(value), Some((0x1122_3344, 0x5566_7788)));
    }

    /// `0x00667760`.
    #[test]
    fn w13a06_insert_text_value_keeps_tag_zero() {
        let mut map = w13a06_map_new(0);
        w13a06_insert_text_value(&mut map, "text", "hello", 0);
        let value = &map.entries[0].value;
        assert_eq!(value.tag, W13A06_TAG_EMPTY);
        assert_eq!(value.text.as_deref(), Some("hello"));
        assert!(w13a06_value_equals(value, &w13a06_value_empty()));
    }

    /// `0x006677d0`.
    #[test]
    fn w13a06_insert_pair_b_low_value_zeroes_high() {
        let mut map = w13a06_map_new(0);
        w13a06_insert_pair_b_low_value(&mut map, "low", 0xdead_beef, 0);
        let value = &map.entries[0].value;
        assert_eq!(value.tag, W13A06_TAG_PAIR_B);
        assert_eq!(w13a06_value_pair(value), Some((0xdead_beef, 0)));
    }

    /// `0x00667840`.
    #[test]
    fn w13a06_insert_pair_b_value_round_trip() {
        let mut map = w13a06_map_new(0);
        w13a06_insert_pair_b_value(&mut map, "pair", 1, 2, 0);
        let value = &map.entries[0].value;
        assert_eq!(value.tag, W13A06_TAG_PAIR_B);
        assert_eq!(w13a06_value_pair(value), Some((1, 2)));
    }

    /// `0x00667180`: eight entries fill the initial load exactly; the ninth
    /// doubles the bucket array.
    #[test]
    fn w13a06_insert_entry_grows_at_load_factor() {
        let mut map = w13a06_map_new(0);
        for index in 0..8u8 {
            w13a06_insert_entry(&mut map, &format!("key{index}"), w13a06_value_byte(index));
        }
        assert_eq!(map.size, 8);
        assert_eq!(map.bucket_count, 8);
        assert_eq!(map.buckets.len(), 8);
        w13a06_insert_entry(&mut map, "key8", w13a06_value_byte(8));
        assert_eq!(map.size, 9);
        assert_eq!(map.bucket_count, 16);
        assert_eq!(map.buckets.len(), 16);
        for index in 0..9u8 {
            assert!(
                w13a06_find_indices(&map, &format!("key{index}")).len() == 1,
                "key{index} missing after rehash"
            );
        }
    }

    /// `0x006678b0`, mode 1.
    #[test]
    fn w13a06_insert_mode_one_replaces_only_when_needed() {
        let mut map = w13a06_map_new(0);
        w13a06_insert_byte_value(&mut map, "a", 1, 0);
        assert_eq!(map.size, 1);
        assert_eq!(map.generation, 2);

        w13a06_insert_byte_value(&mut map, "a", 1, 1);
        assert_eq!(map.size, 1);
        assert_eq!(map.generation, 2);

        w13a06_insert_byte_value(&mut map, "a", 2, 1);
        assert_eq!(map.size, 1);
        assert_eq!(map.generation, 4);
        assert_eq!(
            w13a06_value_byte_of(live_value(&map, "a").unwrap()),
            Some(2)
        );
    }

    /// `0x006678b0`, mode 2.
    #[test]
    fn w13a06_insert_mode_two_updates_every_match() {
        let mut map = w13a06_map_new(0);
        w13a06_insert_entry(&mut map, "dup", w13a06_value_byte(1));
        w13a06_insert_entry(&mut map, "dup", w13a06_value_byte(2));
        assert_eq!(map.size, 2);
        let before = map.generation;

        w13a06_insert_byte_value(&mut map, "dup", 9, 2);
        assert_eq!(map.size, 2);
        assert_eq!(map.generation, before + 1);
        for entry in &map.entries {
            assert_eq!(w13a06_value_byte_of(&entry.value), Some(9));
        }

        w13a06_insert_byte_value(&mut map, "new", 9, 2);
        assert_eq!(map.size, 3);
        assert_eq!(map.generation, before + 3);
    }

    /// `0x006678b0`, neither mode 1 nor mode 2 appends.
    #[test]
    fn w13a06_insert_other_mode_appends() {
        let mut map = w13a06_map_new(5);
        w13a06_insert_entry(&mut map, "dup", w13a06_value_byte(1));
        w13a06_insert_byte_value(&mut map, "dup", 2, 3);
        assert_eq!(map.size, 2);
        let generation = map.generation;
        w13a06_insert_byte_value(&mut map, "dup", 3, 3);
        assert_eq!(map.size, 3);
        assert_eq!(map.generation, generation + 2);
    }

    /// `0x00667dc0`.
    #[test]
    fn w13a06_erase_key_removes_all_matches() {
        let mut map = w13a06_map_new(0);
        w13a06_insert_entry(&mut map, "dup", w13a06_value_byte(1));
        w13a06_insert_entry(&mut map, "dup", w13a06_value_byte(2));
        w13a06_insert_entry(&mut map, "keep", w13a06_value_byte(3));
        let generation = map.generation;

        assert_eq!(w13a06_erase_key(&mut map, "dup"), 2);
        assert_eq!(map.size, 1);
        assert_eq!(map.generation, generation + 1);
        assert_eq!(w13a06_find_indices(&map, "dup").len(), 0);
        assert_eq!(w13a06_find_indices(&map, "keep").len(), 1);

        assert_eq!(w13a06_erase_key(&mut map, "missing"), 0);
        assert_eq!(map.generation, generation + 1);
    }

    /// `0x006676d0`.
    #[test]
    fn w13a06_copy_entries_copies_all_nodes() {
        let mut source = w13a06_map_new(0);
        for index in 0..5u8 {
            w13a06_insert_byte_value(&mut source, &format!("s{index}"), index, 0);
        }
        let mut destination = w13a06_map_new(4);
        let destination_generation = destination.generation;
        w13a06_copy_entries(&mut destination, &source, 2);
        assert_eq!(destination.size, 5);
        assert_eq!(destination.mode, 4);
        assert_eq!(
            destination.generation,
            destination_generation + 2 * 5,
            "mode 2 bumps twice for every absent key"
        );
        assert_eq!(collect(&destination).len(), 5);
        for index in 0..5u8 {
            assert_eq!(
                w13a06_find_indices(&destination, &format!("s{index}")).len(),
                1
            );
        }
    }

    /// `0x00667a10`.
    #[test]
    fn w13a06_assign_from_rebuilds_destination() {
        let mut source = w13a06_map_new(2);
        w13a06_insert_pair_a_value(&mut source, "one", 1, 2, 0);
        w13a06_insert_pair_b_value(&mut source, "two", 3, 4, 0);

        let mut destination = w13a06_map_new(9);
        w13a06_insert_byte_value(&mut destination, "old", 0xff, 0);
        destination.generation = 40;

        w13a06_assign_from(&mut destination, &source);
        assert_eq!(destination.mode, 2);
        assert_eq!(destination.size, 2);
        // One clear-time bump plus `2` for each of the two mode-2 inserts.
        assert_eq!(destination.generation, 45);
        assert_eq!(w13a06_find_indices(&destination, "old").len(), 0);
        assert_eq!(w13a06_find_indices(&destination, "one").len(), 1);
        assert_eq!(w13a06_find_indices(&destination, "two").len(), 1);
    }

    /// `0x00667510`.
    #[test]
    fn w13a06_copy_assign_forwards_to_assign_from() {
        let mut source = w13a06_map_new(0);
        w13a06_insert_byte_value(&mut source, "x", 7, 0);
        let mut destination = w13a06_map_new(0);
        w13a06_copy_assign(&mut destination, &source);
        assert_eq!(destination.size, 1);
        assert_eq!(w13a06_value_byte_of(&destination.entries[0].value), Some(7));
    }

    /// `0x00667530`.
    #[test]
    fn w13a06_iter_next_walks_every_bucket() {
        let mut map = w13a06_map_new(0);
        for index in 0..12u8 {
            w13a06_insert_byte_value(&mut map, &format!("k{index}"), index, 0);
        }
        assert_eq!(map.size, 12);
        let begin = w13a06_cursor_begin(&map);
        assert!(begin.current.is_some());
        let mut cursor = begin;
        let first = w13a06_iter_next(&map, &mut cursor);
        assert_eq!(first.bucket, begin.bucket);
        assert_eq!(first.current, begin.current);
        assert_eq!(first.end, map.bucket_count);

        let mut seen = Vec::new();
        if let Some(index) = first.current {
            seen.push(map.entries[index].key.clone());
        }
        loop {
            let snapshot = w13a06_iter_next(&map, &mut cursor);
            match snapshot.current {
                Some(index) => seen.push(map.entries[index].key.clone()),
                None => break,
            }
        }
        seen.sort();
        let mut expected: Vec<String> = (0..12).map(|i| format!("k{i}")).collect();
        expected.sort();
        assert_eq!(seen, expected);
    }

    /// `0x00667530` with an unallocated bucket array is immediately done.
    #[test]
    fn w13a06_iter_next_empty_map() {
        let map = w13a06_map_new(0);
        let mut cursor = W13a06Cursor {
            bucket: 0,
            current: None,
        };
        let snapshot = w13a06_iter_next(&map, &mut cursor);
        assert_eq!(snapshot.current, None);
        assert_eq!(snapshot.end, map.bucket_count);
        assert_eq!(cursor.bucket, map.bucket_count);
    }
}
