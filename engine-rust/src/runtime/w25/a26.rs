#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent26.rs
// ===========================================================================
/// `0x0082e940` (hash step): murmur3-style finalizer bucket index
/// (`*0x85ebca6b`, `*0xc2b2ae35`) masked with `bucket_count - 1`.
pub fn w25a26_bucket_index(key: u32, bucket_count: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    (h >> 16 ^ h) & bucket_count.wrapping_sub(1)
}

/// `0x0082e940`: probes two chained hash tables (empty bucket arrays are
/// skipped) for `key`.  Returns 0 when absent, 1 when found in the table at
/// +0xa4/+0xa8, 2 when found in the table at +0x94/+0x98.  The retail code
/// reports a duplicate through a non-returning call in either case.
pub fn w25a26_find_duplicate(a: &[Vec<u32>], b: &[Vec<u32>], key: u32) -> u8 {
    let hit = |t: &[Vec<u32>]| {
        !t.is_empty() && t[w25a26_bucket_index(key, t.len() as u32) as usize].contains(&key)
    };
    if hit(a) {
        1
    } else if hit(b) {
        2
    } else {
        0
    }
}

/// `0x00822680`: for each deque slot `start..start+len` yields
/// `(block_index, byte_offset)` where block = `(blocks-1) & (i>>2)` and the
/// offset is `(i&3)*0x2c0` (4 elements of 0x2c0 bytes per block).
pub fn w25a26_deque_slots(start: u32, len: u32, blocks: u32) -> Vec<(u32, u32)> {
    (start..start.wrapping_add(len))
        .map(|i| ((blocks.wrapping_sub(1) & (i >> 2)), (i & 3) * 0x2c0))
        .collect()
}

#[cfg(test)]
mod tests_wave25_agent26 {
    use super::*;

    #[test]
    fn hash_index() {
        assert_eq!(w25a26_bucket_index(0, 16), 0);
        assert!(w25a26_bucket_index(12345, 8) < 8);
        assert_eq!(w25a26_bucket_index(7, 1), 0);
    }

    #[test]
    fn duplicate_lookup() {
        let mut a = vec![Vec::new(); 4];
        let mut b = vec![Vec::new(); 4];
        assert_eq!(w25a26_find_duplicate(&a, &b, 9), 0);
        a[w25a26_bucket_index(9, 4) as usize].push(9);
        b[w25a26_bucket_index(5, 4) as usize].push(5);
        assert_eq!(w25a26_find_duplicate(&a, &b, 9), 1);
        assert_eq!(w25a26_find_duplicate(&a, &b, 5), 2);
        assert_eq!(w25a26_find_duplicate(&[], &[], 5), 0);
    }

    #[test]
    fn deque_slots() {
        let s = w25a26_deque_slots(3, 3, 4);
        assert_eq!(s, vec![(0, 0x840), (1, 0), (1, 0x2c0)]);
        assert!(w25a26_deque_slots(0, 0, 4).is_empty());
    }
}
