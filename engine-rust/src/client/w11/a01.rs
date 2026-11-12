#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave11/agent01.rs
// ===========================================================================
// Wave 11 batch 01: client skin records, their name lookup, and the two
// small chained maps that cache skin state.
//
// The range `0x00648000..=0x00649fff` is UI/client glue around the
// `selectedSkin` option.  Each item below is a stand-alone translation of
// one ledger row.  The shipped object layouts are expressed as Rust
// records using the offsets the decompiler reads (`+0x24` skin id,
// `+0x28`/`+0x2c` name view, `+4`/`+6` map key/value, `0x78` bytes per
// record).  Rows that decompile to exception-list plumbing, compiler
// vtable installation, or corrupted control flow are not translated, so
// every single-address citation below names an implementation that
// exists in this file.

/// Bytes per skin record: the pointer step in `0x00649580` and
/// `0x00649980`.
pub const W11A01_SKIN_RECORD_SIZE: usize = 0x78;

/// Records scanned before the shared sentinel record (`3 * 0x78`).
pub const W11A01_SKIN_RECORD_COUNT: usize = 3;

/// The skin id word (`+0x24`) compared by `0x006481c0` / `0x006481e0` and
/// handed to the setter by `0x00649320`.
pub const W11A01_SKIN_ID_OFFSET: usize = 0x24;

/// The name-view offset (`+0x28`) returned by `0x00649d60`.
pub const W11A01_SKIN_NAME_OFFSET: usize = 0x28;

/// Exact 71-byte (`0x47`) diagnostic literal formatted by `0x00649d70`.
pub const W11A01_MISSING_PRODUCT_ID_FORMAT: &str =
    "GetProductId was called for {0} skin but getProductId was never set up.";

/// `0x006481b0` (`FUN_006481b0`, 12 bytes): fill an eight-byte index ramp.
///
/// ```c
/// uVar1 = 0;
/// do { *(char *)(uVar1 + in_ECX) = (char)uVar1; uVar1++; } while (uVar1 < 8);
/// ```
///
/// The row overwrites the eight bytes at `this + 0` with `0..7`.
pub fn w11a01_fill_index_bytes(out: &mut [u8; 8]) {
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = index as u8;
    }
}

/// `0x006481c0` (`FUN_006481c0`, 21 bytes): compare the skin id words.
///
/// ```c
/// return *(short *)(in_ECX + 0x24) == *(short *)(param_1 + 0x24);
/// ```
///
/// The two records are compared by the `u16` at `+0x24` only.
pub fn w11a01_skin_ids_equal(this_id: u16, other_id: u16) -> bool {
    this_id == other_id
}

/// `0x006481e0` (`FUN_006481e0`, 21 bytes): the inequality twin of
/// `0x006481c0`.
///
/// ```c
/// return *(short *)(in_ECX + 0x24) != *(short *)(param_1 + 0x24);
/// ```
pub fn w11a01_skin_ids_differ(this_id: u16, other_id: u16) -> bool {
    this_id != other_id
}

/// One node of the `u16 -> u8` skin refresh cache.
///
/// The shipped record is `{ next at +0, key at +4, value at +6 }`;
/// `0x00648200` returns the address `node + 6`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W11a01SkinNode {
    /// Link at `+0`.
    pub next: Option<usize>,
    /// Key at `+4`.
    pub key: u16,
    /// Value byte at `+6`.
    pub value: u8,
}

/// Failure modes of the skin refresh cache (`0x00648200`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W11a01SkinFault {
    /// `this[1]` (the bucket array) is null.
    NoBuckets,
    /// The chain ended without a key match.
    NotFound,
}

/// A cache hit: the node index and the value byte at `node + 6`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W11a01SkinHit {
    /// Index into the caller's node slice.
    pub node: usize,
    /// Value byte at `node + 6`.
    pub value: u8,
}

/// MurmurHash3 32-bit finalizer shared by the two skin maps
/// (`0x00648200` and `0x00649280`).
///
/// ```text
/// h ^= h >> 16; h *= 0x85ebca6b; h ^= h >> 13; h *= 0xc2b2ae35; h ^= h >> 16;
/// ```
///
/// `0x00648200` hashes a zero-extended `u16`, so the first `>> 16` xor is
/// a no-op and its decompiled row starts at the `0x85ebca6b` multiply.
pub fn w11a01_mix32(key: u32) -> u32 {
    let mut hash = key ^ (key >> 16);
    hash = hash.wrapping_mul(0x85eb_ca6b);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(0xc2b2_ae35);
    hash ^ (hash >> 16)
}

/// `0x00648200` (`FUN_00648200`, 119 bytes): look up a `u16` key in the
/// skin refresh cache.
///
/// The row computes
///
/// ```text
/// slot = w11a01_mix32(key) & (*this - 1)          // *this = bucket count
/// ```
///
/// then walks the chain, remembering the link that points at the match;
/// a hit returns `node + 6`.  A null bucket array is first passed to the
/// container's lazy initialiser, which is outside this batch, so
/// [`W11a01SkinFault::NoBuckets`] reports the same precondition.  A miss
/// ends at the row's noreturn handler and is modelled as
/// [`W11a01SkinFault::NotFound`].  A chain index outside `nodes` is also
/// reported as `NotFound` rather than dereferenced.
///
/// The bucket count is assumed to be the power of two the mask arithmetic
/// expects, as in the shipped containers.
pub fn w11a01_skin_lookup(
    buckets: &[Option<usize>],
    nodes: &[W11a01SkinNode],
    key: u16,
) -> Result<W11a01SkinHit, W11a01SkinFault> {
    if buckets.is_empty() {
        return Err(W11a01SkinFault::NoBuckets);
    }
    let slot = (w11a01_mix32(u32::from(key)) & (buckets.len() as u32 - 1)) as usize;
    let mut link = buckets[slot];
    while let Some(index) = link {
        let node = nodes.get(index).ok_or(W11a01SkinFault::NotFound)?;
        if node.key == key {
            return Ok(W11a01SkinHit {
                node: index,
                value: node.value,
            });
        }
        link = node.next;
    }
    Err(W11a01SkinFault::NotFound)
}

/// `0x00648300` (`FUN_00648300`, 113 bytes): recompute one skin's cached
/// byte.
///
/// The row reads the current skin id (the `u16` at `+0x24`); `0` and
/// `0xffff` are sentinels and return `0` without touching the cache.  For
/// any other id it calls the stack table entry selected by the id, stores
/// the returned byte through `0x00648200`, and calls `0x00648200` a second
/// time to return the stored byte.  Because the store goes through the
/// already-created node, a missing key ends at the noreturn handler;
/// [`W11a01SkinFault::NotFound`] models that here.  The value is computed
/// before the lookup, matching the instruction order.
pub fn w11a01_cache_selected_skin_value(
    buckets: &[Option<usize>],
    nodes: &mut [W11a01SkinNode],
    skin_id: u16,
    value_of: impl FnOnce(u16) -> u8,
) -> Result<u8, W11a01SkinFault> {
    if skin_id == 0 || skin_id == 0xffff {
        return Ok(0);
    }
    let value = value_of(skin_id);
    let hit = w11a01_skin_lookup(buckets, nodes, skin_id)?;
    nodes[hit.node].value = value;
    Ok(nodes[hit.node].value)
}

/// One node of the `u32`-keyed skin set read by `0x00649280`.
///
/// The shipped record is `{ next at +0, key at +4 }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W11a01KeyNode {
    /// Link at `+0`.
    pub next: Option<usize>,
    /// Key at `+4`.
    pub key: u32,
}

/// `0x00649280` (`FUN_00649280`, 106 bytes): membership test for the
/// `u32`-keyed skin set.
///
/// A null bucket array returns `false`.  Otherwise the row starts at
/// `w11a01_mix32(key) & (*this - 1)` and walks the chain comparing the
/// `u32` at node `+4`; it returns whether the walk stopped on a node
/// (`*link != 0`), and the only early stop is a key match.  A chain index
/// outside `nodes` is treated as absence instead of being dereferenced.
pub fn w11a01_key_set_contains(
    buckets: &[Option<usize>],
    nodes: &[W11a01KeyNode],
    key: u32,
) -> bool {
    if buckets.is_empty() {
        return false;
    }
    let slot = (w11a01_mix32(key) & (buckets.len() as u32 - 1)) as usize;
    let mut link = buckets[slot];
    while let Some(index) = link {
        let Some(node) = nodes.get(index) else {
            return false;
        };
        if node.key == key {
            return true;
        }
        link = node.next;
    }
    false
}

/// The per-skin entry whose `+4` byte `0x006492f0` returns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a01SkinEntry {
    /// `+4`: nonzero once the skin is configured.
    pub defined: u8,
}

/// `0x006492f0` (`FUN_006492f0`, 9 bytes): read the `defined` byte at
/// `+4` of the current skin entry.
///
/// The shipped row fetches the entry through a global accessor (outside
/// this batch) and returns `*(undefined1 *)(entry + 4)`; the byte is
/// returned unchanged, so callers can keep the full `0..=0xff` domain.
pub fn w11a01_skin_entry_defined(entry: &W11a01SkinEntry) -> u8 {
    entry.defined
}

/// `0x00649310` (`FUN_00649310`, 12 bytes): refresh tail call.
///
/// The shipped body is a single call to `0x00648300`; the decompiler shows
/// no arguments because the row inherits the caller's registers and stack.
/// They are made explicit here, and the call forwards to
/// [`w11a01_cache_selected_skin_value`].
pub fn w11a01_refresh_selected_skin(
    buckets: &[Option<usize>],
    nodes: &mut [W11a01SkinNode],
    skin_id: u16,
    value_of: impl FnOnce(u16) -> u8,
) -> Result<u8, W11a01SkinFault> {
    w11a01_cache_selected_skin_value(buckets, nodes, skin_id, value_of)
}

/// One fixed-name skin record.
///
/// `0x00649580` compares the name view at `+0x28` (data) / `+0x2c`
/// (length) and `0x00649980` steps over records of `0x78` bytes.  The id
/// consumed by `0x00649320` is the `u16` at `+0x24`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W11a01SkinRecord<'a> {
    /// Name bytes at `+0x28`, length at `+0x2c`.
    pub name: &'a [u8],
    /// Skin id at `+0x24`.
    pub skin_id: u16,
}

/// Result of the `0x00649580` scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W11a01SkinRecordMatch {
    /// Index into the caller's record slice.
    Found(usize),
    /// No record matched; the shared sentinel record is selected.
    Sentinel,
}

/// `0x00649580` (`FUN_00649580`, 89 bytes): find a skin record by name.
///
/// The row walks records in `0x78`-byte steps until the sentinel table,
/// comparing `memcmp(record.name, name, min(record.len, name.len))`.  When
/// that compare is zero it tests `record.len - name.len == 0`, so the match
/// is an exact-length match even though only the common prefix is
/// compared; differing lengths continue the scan.  A miss returns the
/// sentinel table, which callers then read as a record; the refresh row
/// treats an id of `0xffff` as "none".
pub fn w11a01_find_skin_record<'a>(
    records: &[W11a01SkinRecord<'a>],
    name: &[u8],
) -> W11a01SkinRecordMatch {
    for (index, record) in records.iter().enumerate() {
        let common = record.name.len().min(name.len());
        let prefixes_equal = record.name[..common] == name[..common];
        if prefixes_equal {
            let length_difference = record.name.len() as isize - name.len() as isize;
            if length_difference == 0 {
                return W11a01SkinRecordMatch::Found(index);
            }
        }
    }
    W11a01SkinRecordMatch::Sentinel
}

/// `0x00649320` (`FUN_00649320`, 32 bytes): apply the skin id of the
/// record named by the view at `this + 4`.
///
/// The row looks the name up with `0x00649580`, then passes the found
/// record's `u16` at `+0x24` to a setter.  The setter's address is outside
/// this batch, so it is supplied as a closure.  On a miss the sentinel
/// record's id is passed; the refresh row treats `0xffff` as the "none"
/// id, so tests supply `0xffff` here.
pub fn w11a01_apply_named_skin<'a>(
    records: &[W11a01SkinRecord<'a>],
    name: &[u8],
    sentinel_skin_id: u16,
    mut set_skin: impl FnMut(u16),
) {
    let id = match w11a01_find_skin_record(records, name) {
        W11a01SkinRecordMatch::Found(index) => records[index].skin_id,
        W11a01SkinRecordMatch::Sentinel => sentinel_skin_id,
    };
    set_skin(id);
}

/// `0x00649980` (`FUN_00649980`, 39 bytes): byte offset of a skin record.
///
/// ```c
/// if (param_1 < 3) return &DAT_00fc9b88 + param_1 * 0x78;
/// return &DAT_00fc9cf0;
/// ```
///
/// Ids `0..=2` select one of three records; every other id, including the
/// `0xffff` sentinel, selects the shared sentinel record, which begins
/// `3 * 0x78` bytes past the table base.
pub fn w11a01_skin_record_offset(skin_id: u16) -> usize {
    if usize::from(skin_id) < W11A01_SKIN_RECORD_COUNT {
        usize::from(skin_id) * W11A01_SKIN_RECORD_SIZE
    } else {
        W11A01_SKIN_RECORD_SIZE * W11A01_SKIN_RECORD_COUNT
    }
}

/// Skin table selected by the client skin rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W11a01SkinTable {
    /// The primary skin table (`PTR_DAT_00f05834`).
    Primary,
    /// The shared fallback table (`&DAT_00fc9cf0`).
    Fallback,
}

/// `0x00649c90` (`FUN_00649c90`, 33 bytes): choose the skin table for a
/// classifier result.
///
/// The row calls the class classifier (outside this batch) and returns the
/// primary table only when the result is `2`; every other value selects
/// the fallback table.
pub fn w11a01_skin_table_for_class(class: i32) -> W11a01SkinTable {
    if class == 2 {
        W11a01SkinTable::Primary
    } else {
        W11a01SkinTable::Fallback
    }
}

/// `0x00649d20` (`FUN_00649d20`, 56 bytes): choose a table for a key.
///
/// The primary table is selected only when the `u32` key is present in the
/// set read by `0x00649280` **and** the primary skin id (the record word at
/// `+0x24`) passes the caller's validity check.  Otherwise the fallback
/// table is used.  The shipped `&&` short-circuits, so the validity
/// closure is not called for an absent key; the `if` chain preserves that.
pub fn w11a01_skin_table_for_key(
    key: u32,
    buckets: &[Option<usize>],
    nodes: &[W11a01KeyNode],
    primary_skin_id: u16,
    is_valid: impl FnOnce(u16) -> bool,
) -> W11a01SkinTable {
    if w11a01_key_set_contains(buckets, nodes, key) && is_valid(primary_skin_id) {
        W11a01SkinTable::Primary
    } else {
        W11a01SkinTable::Fallback
    }
}

/// `0x00649d60` (`FUN_00649d60`, 9 bytes): offset of the primary table's
/// first-record name view.
///
/// The row is `return PTR_DAT_00f05834 + 0x28`; this returns the `0x28`
/// displacement from the primary table base, and `+0x28` is the name
/// data/pointer word that `0x00649580` compares.
pub fn w11a01_skin_name_offset() -> usize {
    W11A01_SKIN_NAME_OFFSET
}

/// `0x00649d70` (`FUN_00649d70`, 279 bytes): build the missing
/// `getProductId` diagnostic.
///
/// The row formats the skin name view at `+0x28`/`+0x2c` into the exact
/// `0x47`-byte literal held in [`W11A01_MISSING_PRODUCT_ID_FORMAT`] and
/// then enters its noreturn throw path.  The formatted message is returned
/// here so the caller can raise its own error type.
pub fn w11a01_missing_product_id_message(skin: &str) -> String {
    W11A01_MISSING_PRODUCT_ID_FORMAT.replacen("{0}", skin, 1)
}

/// Product-id table selected by `0x00649fa0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W11a01ProductIdTable {
    /// The primary table's `+0x44` slot.
    Primary,
    /// The fallback product-id table (`&DAT_00fc9d34`).
    Fallback,
}

/// `0x00649fa0` (`FUN_00649fa0`, 21 bytes): choose the `getProductId`
/// table.
///
/// Only class `2` whose key is present (the set read by `0x00649280`) and
/// whose primary skin id passes the validity check gets the primary slot
/// at `+0x44`; everything else selects the fallback product-id table.
pub fn w11a01_product_id_table(
    class: i32,
    key: u32,
    buckets: &[Option<usize>],
    nodes: &[W11a01KeyNode],
    primary_skin_id: u16,
    is_valid: impl FnOnce(u16) -> bool,
) -> W11a01ProductIdTable {
    if class == 2 && w11a01_key_set_contains(buckets, nodes, key) && is_valid(primary_skin_id) {
        W11a01ProductIdTable::Primary
    } else {
        W11a01ProductIdTable::Fallback
    }
}

/// `0x00649ff0` (`FUN_00649ff0`, 6 bytes): the RTTI type descriptor of the
/// skin class.
///
/// The shipped row is `return &.P6AXXZ::RTTI_Type_Descriptor`, i.e. the
/// descriptor for a `void (__cdecl *)(void)` pointer type.  The MSVC symbol
/// text is returned for comparison against the recovered symbol.
pub fn w11a01_rtti_type_descriptor() -> &'static str {
    ".P6AXXZ::RTTI_Type_Descriptor"
}

#[cfg(test)]
mod tests_wave11_agent01 {
    use super::*;
    use std::cell::Cell;

    fn node(next: Option<usize>, key: u16, value: u8) -> W11a01SkinNode {
        W11a01SkinNode { next, key, value }
    }

    fn key_node(next: Option<usize>, key: u32) -> W11a01KeyNode {
        W11a01KeyNode { next, key }
    }

    fn record(name: &'static [u8], skin_id: u16) -> W11a01SkinRecord<'static> {
        W11a01SkinRecord { name, skin_id }
    }

    #[test]
    fn index_ramp_writes_zero_through_seven() {
        let mut bytes = [0xff; 8];
        w11a01_fill_index_bytes(&mut bytes);
        assert_eq!(bytes, [0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn skin_ids_compare_the_field_at_0x24() {
        assert!(w11a01_skin_ids_equal(7, 7));
        assert!(!w11a01_skin_ids_equal(7, 0xffff));
        assert!(!w11a01_skin_ids_differ(0xffff, 0xffff));
        assert!(w11a01_skin_ids_differ(0, 0xffff));
        // Every id except the two sentinels is an ordinary value.
        assert!(w11a01_skin_ids_equal(0, 0));
        assert!(w11a01_skin_ids_differ(1, 2));
    }

    #[test]
    fn mix32_matches_the_recovered_finalizer() {
        // MurmurHash3 fmix32 vectors; the shipped rows multiply by
        // 0x85ebca6b and 0xc2b2ae35 with >> 13 and >> 16 xors.
        assert_eq!(w11a01_mix32(0), 0);
        assert_eq!(w11a01_mix32(1), 0x514e_28b7);
        assert_eq!(w11a01_mix32(0x42), 0xb88e_20bd);
        assert_eq!(w11a01_mix32(0x1234_5678), 0xe37c_d1bc);
        // The u16 row hashes a zero-extended key.
        assert_eq!(w11a01_mix32(u32::from(0xffffu16)), 0xa23b_ae67);
    }

    #[test]
    fn skin_lookup_walks_the_chain_and_reports_faults() {
        // Eight buckets; keys 0x42, 4 and 5 all mix into bucket 5.
        let nodes = [
            node(Some(1), 0x42, 7),
            node(Some(2), 4, 8),
            node(None, 5, 9),
        ];
        let mut buckets = [None; 8];
        buckets[5] = Some(0);
        for key in [0x42u16, 4, 5] {
            assert_eq!(w11a01_mix32(u32::from(key)) & 7, 5);
        }
        assert_eq!(
            w11a01_skin_lookup(&buckets, &nodes, 0x42),
            Ok(W11a01SkinHit { node: 0, value: 7 })
        );
        assert_eq!(
            w11a01_skin_lookup(&buckets, &nodes, 4),
            Ok(W11a01SkinHit { node: 1, value: 8 })
        );
        assert_eq!(
            w11a01_skin_lookup(&buckets, &nodes, 5),
            Ok(W11a01SkinHit { node: 2, value: 9 })
        );
        // 0x1000 also lands in bucket 5 but is absent from the chain.
        assert_eq!(w11a01_mix32(0x1000) & 7, 5);
        assert_eq!(
            w11a01_skin_lookup(&buckets, &nodes, 0x1000),
            Err(W11a01SkinFault::NotFound)
        );
        assert_eq!(
            w11a01_skin_lookup(&[], &nodes, 0x42),
            Err(W11a01SkinFault::NoBuckets)
        );
    }

    #[test]
    fn cache_selected_skin_stores_then_reads_back() {
        let mut nodes = [node(None, 5, 0)];
        let mut buckets = [None; 4];
        buckets[1] = Some(0);
        let calls = Cell::new(0);
        let value = w11a01_cache_selected_skin_value(&buckets, &mut nodes, 5, |id| {
            calls.set(calls.get() + 1);
            assert_eq!(id, 5);
            0xab
        })
        .expect("existing node");
        assert_eq!(value, 0xab);
        assert_eq!(nodes[0].value, 0xab);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn cache_selected_skin_skips_zero_and_ffff() {
        let mut nodes = [node(None, 5, 0x22)];
        let mut buckets = [None; 4];
        buckets[1] = Some(0);
        let calls = Cell::new(0);
        for sentinel in [0u16, 0xffff] {
            let value = w11a01_cache_selected_skin_value(&buckets, &mut nodes, sentinel, |_| {
                calls.set(calls.get() + 1);
                0xee
            })
            .expect("sentinels bypass the map");
            assert_eq!(value, 0);
        }
        assert_eq!(nodes[0].value, 0x22);
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn cache_selected_skin_checks_the_slot_after_computing() {
        // Key 4 mixes into bucket 1 and shares the chain with key 5, but
        // is not present: the value is computed first, then the lookup
        // fails, as in the shipped instruction order.
        let mut nodes = [node(None, 5, 0)];
        let mut buckets = [None; 4];
        buckets[1] = Some(0);
        let calls = Cell::new(0);
        let result = w11a01_cache_selected_skin_value(&buckets, &mut nodes, 4, |_| {
            calls.set(calls.get() + 1);
            0x11
        });
        assert_eq!(result, Err(W11a01SkinFault::NotFound));
        assert_eq!(calls.get(), 1);
        assert_eq!(nodes[0].value, 0);
        // A null table also computes first and then reports NoBuckets.
        let result = w11a01_cache_selected_skin_value(&[], &mut nodes, 5, |_| 0x33);
        assert_eq!(result, Err(W11a01SkinFault::NoBuckets));
    }

    #[test]
    fn refresh_selected_skin_forwards_to_the_cache() {
        let mut nodes = [node(None, 5, 0)];
        let mut buckets = [None; 4];
        buckets[1] = Some(0);
        let value = w11a01_refresh_selected_skin(&buckets, &mut nodes, 5, |_| 0x5a)
            .expect("refresh tail call");
        assert_eq!(value, 0x5a);
        assert_eq!(nodes[0].value, 0x5a);
    }

    #[test]
    fn skin_set_contains_walks_the_u32_chain() {
        let nodes = [key_node(Some(1), 0x42), key_node(None, 0x99)];
        let mut buckets = [None; 4];
        buckets[1] = Some(0);
        // 0x42, 0x99 and 0x1234 all mix into bucket 1; 0x1234 is absent.
        for key in [0x42u32, 0x99, 0x1234] {
            assert_eq!(w11a01_mix32(key) & 3, 1);
        }
        assert!(w11a01_key_set_contains(&buckets, &nodes, 0x42));
        assert!(w11a01_key_set_contains(&buckets, &nodes, 0x99));
        assert!(!w11a01_key_set_contains(&buckets, &nodes, 0x1234));
        assert!(!w11a01_key_set_contains(&[], &nodes, 0x42));
    }

    #[test]
    fn skin_entry_defined_reads_the_byte_at_4() {
        assert_eq!(
            w11a01_skin_entry_defined(&W11a01SkinEntry { defined: 0 }),
            0
        );
        assert_eq!(
            w11a01_skin_entry_defined(&W11a01SkinEntry { defined: 1 }),
            1
        );
        assert_eq!(
            w11a01_skin_entry_defined(&W11a01SkinEntry { defined: 0xff }),
            0xff
        );
    }

    #[test]
    fn skin_record_search_matches_exact_names() {
        let records = [record(b"alpha", 1), record(b"beta", 2), record(b"gamma", 3)];
        assert_eq!(
            w11a01_find_skin_record(&records, b"alpha"),
            W11a01SkinRecordMatch::Found(0)
        );
        assert_eq!(
            w11a01_find_skin_record(&records, b"beta"),
            W11a01SkinRecordMatch::Found(1)
        );
        assert_eq!(
            w11a01_find_skin_record(&records, b"gamma"),
            W11a01SkinRecordMatch::Found(2)
        );
        // The prefix compare alone is not enough: lengths must agree too.
        for miss in [b"alph".as_slice(), b"alphax", b"", b"delta"] {
            assert_eq!(
                w11a01_find_skin_record(&records, miss),
                W11a01SkinRecordMatch::Sentinel
            );
        }
        assert_eq!(
            w11a01_find_skin_record(&[], b"alpha"),
            W11a01SkinRecordMatch::Sentinel
        );
    }

    #[test]
    fn named_skin_lookup_passes_the_record_id() {
        let records = [record(b"alpha", 7), record(b"beta", 9)];
        let mut seen = Vec::new();
        w11a01_apply_named_skin(&records, b"beta", 0xffff, |id| seen.push(id));
        assert_eq!(seen, [9]);
        w11a01_apply_named_skin(&records, b"missing", 0xffff, |id| seen.push(id));
        assert_eq!(seen, [9, 0xffff]);
        // The sentinel id is whatever the fallback record holds.
        w11a01_apply_named_skin(&records, b"alpha", 0x1234, |id| seen.push(id));
        assert_eq!(seen, [9, 0xffff, 7]);
    }

    #[test]
    fn skin_record_offset_selects_three_slots() {
        assert_eq!(w11a01_skin_record_offset(0), 0);
        assert_eq!(w11a01_skin_record_offset(1), 0x78);
        assert_eq!(w11a01_skin_record_offset(2), 0xf0);
        // 3 is already the sentinel, as is the 0xffff "none" id.
        assert_eq!(w11a01_skin_record_offset(3), 0x168);
        assert_eq!(w11a01_skin_record_offset(0xffff), 0x168);
        assert_eq!(
            w11a01_skin_record_offset(3),
            W11A01_SKIN_RECORD_SIZE * W11A01_SKIN_RECORD_COUNT
        );
    }

    #[test]
    fn skin_table_for_class_only_accepts_two() {
        assert_eq!(w11a01_skin_table_for_class(2), W11a01SkinTable::Primary);
        for class in [0, 1, 3, -1, i32::MIN, i32::MAX] {
            assert_eq!(
                w11a01_skin_table_for_class(class),
                W11a01SkinTable::Fallback
            );
        }
    }

    #[test]
    fn skin_table_for_key_requires_presence_and_validity() {
        let nodes = [key_node(None, 0x42)];
        let mut buckets = [None; 4];
        buckets[1] = Some(0);

        let seen = Cell::new(0u16);
        let table = w11a01_skin_table_for_key(0x42, &buckets, &nodes, 0xffff, |id| {
            seen.set(id);
            true
        });
        assert_eq!(table, W11a01SkinTable::Primary);
        assert_eq!(seen.get(), 0xffff);

        // Present but invalid.
        let checked = Cell::new(false);
        let table = w11a01_skin_table_for_key(0x42, &buckets, &nodes, 0xffff, |_| {
            checked.set(true);
            false
        });
        assert_eq!(table, W11a01SkinTable::Fallback);
        assert!(checked.get());

        // Absent: the validity check is short-circuited away.
        let checked = Cell::new(false);
        let table = w11a01_skin_table_for_key(0x1234, &buckets, &nodes, 0xffff, |_| {
            checked.set(true);
            true
        });
        assert_eq!(table, W11a01SkinTable::Fallback);
        assert!(!checked.get());
    }

    #[test]
    fn skin_name_offset_points_at_the_first_record_name() {
        assert_eq!(w11a01_skin_name_offset(), 0x28);
        // The first record's id word is at +0x24, four bytes before it.
        assert_eq!(W11A01_SKIN_ID_OFFSET + 4, w11a01_skin_name_offset());
    }

    #[test]
    fn missing_product_id_message_names_the_skin() {
        assert_eq!(W11A01_MISSING_PRODUCT_ID_FORMAT.len(), 0x47);
        assert_eq!(
            w11a01_missing_product_id_message("zerg"),
            "GetProductId was called for zerg skin but getProductId was never set up."
        );
        // Removing the three-byte placeholder leaves 68 literal bytes.
        assert_eq!(w11a01_missing_product_id_message("").len(), 68);
        assert_eq!(
            w11a01_missing_product_id_message("a{0}b"),
            "GetProductId was called for a{0}b skin but getProductId was never set up."
        );
    }

    #[test]
    fn product_id_table_requires_class_two() {
        let nodes = [key_node(None, 0x42)];
        let mut buckets = [None; 4];
        buckets[1] = Some(0);

        assert_eq!(
            w11a01_product_id_table(2, 0x42, &buckets, &nodes, 0xffff, |_| true),
            W11a01ProductIdTable::Primary
        );

        // Wrong class: neither lookup nor validity is consulted.
        let checked = Cell::new(false);
        let table = w11a01_product_id_table(1, 0x42, &buckets, &nodes, 0xffff, |_| {
            checked.set(true);
            true
        });
        assert_eq!(table, W11a01ProductIdTable::Fallback);
        assert!(!checked.get());

        // Right class, absent key.
        let checked = Cell::new(false);
        let table = w11a01_product_id_table(2, 0x1234, &buckets, &nodes, 0xffff, |_| {
            checked.set(true);
            true
        });
        assert_eq!(table, W11a01ProductIdTable::Fallback);
        assert!(!checked.get());

        // Right class and key, invalid skin id.
        assert_eq!(
            w11a01_product_id_table(2, 0x42, &buckets, &nodes, 0xffff, |_| false),
            W11a01ProductIdTable::Fallback
        );
    }

    #[test]
    fn rtti_descriptor_names_the_function_pointer_type() {
        assert_eq!(
            w11a01_rtti_type_descriptor(),
            ".P6AXXZ::RTTI_Type_Descriptor"
        );
    }
}
