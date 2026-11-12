#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent02.rs
// ===========================================================================
// Wave 21 agent 02 translations (pure reference helpers).

/// `0x00595a20`: bucket index of the u32 hash map lookup (murmur3 finalizer
/// masked by `bucket_count - 1`).
pub fn w21a02_hash_bucket(key: u32, bucket_count: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85EB_CA6B);
    h = (h >> 13 ^ h).wrapping_mul(0xC2B2_AE35);
    (h >> 16 ^ h) & bucket_count.wrapping_sub(1)
}

/// `0x00595a20`: chain lookup.  `buckets[i]` is the head node index (or 0 =
/// none, 1-based), `nodes[n-1] = (next, key)`.  Returns the 1-based node
/// index of the match; `None` where the retail code aborts.
pub fn w21a02_hash_find(buckets: &[u32], nodes: &[(u32, u32)], key: u32) -> Option<u32> {
    if buckets.is_empty() {
        return None;
    }
    let b = w21a02_hash_bucket(key, buckets.len() as u32) as usize;
    let mut cur = *buckets.get(b)?;
    while cur != 0 {
        let (next, k) = *nodes.get(cur as usize - 1)?;
        if k == key {
            return Some(cur);
        }
        cur = next;
    }
    None
}

/// `0x00597bd0`: maps a slot to its remapped value for slots below 8,
/// otherwise returns the slot unchanged.
pub fn w21a02_remap_slot(table: &[u32; 8], slot: u32) -> u32 {
    if slot > 7 { slot } else { table[slot as usize] }
}

/// `0x00597e60`: capacity growth.  `field` packs flag bit 31 and a 31-bit
/// capacity; returns `(new_field, needs_realloc)` where growth is 1.5x or
/// the requested size, whichever is larger.  The flag bit is preserved.
pub fn w21a02_grow_capacity(field: u32, wanted: u32) -> (u32, bool) {
    let cap = field & 0x7FFF_FFFF;
    if cap >= wanted {
        return (field, false);
    }
    let mut n = ((field >> 1) & 0x3FFF_FFFF).wrapping_add(cap);
    if n < wanted {
        n = wanted;
    }
    (((field ^ n) & 0x7FFF_FFFF) ^ field, true)
}

/// `0x00a8fbb0`: builds a homogeneous vec4 `(x, y, z, 1.0)` (w = 0x3f800000).
pub fn w21a02_vec4_point(x: u32, y: u32, z: u32) -> [u32; 4] {
    [x, y, z, 0x3F80_0000]
}

fn w21a02_pick(kind: u16, k1: u16, g1: u32, g2: u32) -> Option<u32> {
    let sel = if kind == k1 { g1 } else { g2 };
    if (kind == k1 || kind == 8) && sel != 0 {
        Some(sel)
    } else {
        None
    }
}

/// `0x006eaa20`: selects the global handle for a unit-like record whose
/// type word (+0x3a) is 6 (`g_a`) or 8 (`g_b`); `None` when neither
/// applies or the handle is zero.
pub fn w21a02_select_handle_6_8(kind: u16, g_a: u32, g_b: u32) -> Option<u32> {
    w21a02_pick(kind, 6, g_a, g_b)
}

/// `0x006ec1a0`: same selection with type 7 (`g_a`) or 8 (`g_b`).
pub fn w21a02_select_handle_7_8(kind: u16, g_a: u32, g_b: u32) -> Option<u32> {
    w21a02_pick(kind, 7, g_a, g_b)
}

#[cfg(test)]
mod tests_wave21_agent02 {
    use super::*;

    #[test]
    fn hash_and_find() {
        assert_eq!(w21a02_hash_bucket(0, 16), 0);
        assert!(w21a02_hash_bucket(12345, 16) < 16);
        let b = w21a02_hash_bucket(7, 4) as usize;
        let mut buckets = [0u32; 4];
        buckets[b] = 2;
        buckets[b] = 1;
        let nodes = [(2, 1), (0, 7)];
        assert_eq!(w21a02_hash_find(&buckets, &nodes, 7), Some(2));
        assert_eq!(w21a02_hash_find(&buckets, &nodes, 8), None);
        assert_eq!(w21a02_hash_find(&[], &nodes, 7), None);
    }

    #[test]
    fn remap() {
        let t = [10, 11, 12, 13, 14, 15, 16, 17];
        assert_eq!(w21a02_remap_slot(&t, 3), 13);
        assert_eq!(w21a02_remap_slot(&t, 8), 8);
    }

    #[test]
    fn grow() {
        assert_eq!(w21a02_grow_capacity(0x8000_0010, 8), (0x8000_0010, false));
        assert_eq!(w21a02_grow_capacity(0x8000_0010, 20), (0x8000_0018, true));
        assert_eq!(w21a02_grow_capacity(0x10, 100), (100, true));
    }

    #[test]
    fn vec4_and_select() {
        assert_eq!(w21a02_vec4_point(1, 2, 3)[3], 0x3F80_0000);
        assert_eq!(w21a02_select_handle_6_8(6, 5, 9), Some(5));
        assert_eq!(w21a02_select_handle_6_8(8, 5, 9), Some(9));
        assert_eq!(w21a02_select_handle_6_8(7, 5, 9), None);
        assert_eq!(w21a02_select_handle_6_8(6, 0, 9), None);
        assert_eq!(w21a02_select_handle_7_8(7, 4, 0), Some(4));
        assert_eq!(w21a02_select_handle_7_8(8, 4, 0), None);
    }
}
