#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent29.rs
// ===========================================================================
// Wave 25 agent 29: small runtime helpers (capacity growth, hash bucket lookup).

/// `0x0083b1a0`: new capacity for a small-string-optimised buffer whose
/// flags word keeps the capacity in the low 31 bits.  If the current
/// capacity is below `wanted`, grows by 1.5x (at least `wanted`) and keeps
/// the top bit of the word.  Returns the updated word.
pub fn w25a29_grow_capacity_word(word: u32, wanted: u32) -> u32 {
    let cap = word & 0x7fff_ffff;
    if cap >= wanted {
        return word;
    }
    let mut new_cap = ((word >> 1) & 0x3fff_ffff).wrapping_add(cap);
    if new_cap < wanted {
        new_cap = wanted;
    }
    ((word ^ new_cap) & 0x7fff_ffff) ^ word
}

/// Bucket index mix used by the hash lookups in `0x00839740` (32-bit
/// finalizer; `bucket_count` is a power of two).
pub fn w25a29_bucket_index(key: u32, bucket_count: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    (h ^ h >> 16) & bucket_count.wrapping_sub(1)
}

/// `0x00839740` (lookup part): finds the node with key `(lo, hi)` in a
/// chained table; `buckets[i]` lists the nodes of bucket `i` as `(lo, hi)`
/// pairs.  Returns the position inside the bucket, or `None` for an empty
/// table or a miss.
pub fn w25a29_find_node(buckets: &[Vec<(u32, u32)>], lo: u32, hi: u32) -> Option<usize> {
    if buckets.is_empty() {
        return None;
    }
    let idx = w25a29_bucket_index(lo, buckets.len() as u32) as usize;
    buckets
        .get(idx)?
        .iter()
        .position(|&(a, b)| a == lo && b == hi)
}

#[cfg(test)]
mod tests_wave25_agent29 {
    use super::*;

    #[test]
    fn grow() {
        assert_eq!(w25a29_grow_capacity_word(0x10, 8), 0x10);
        assert_eq!(w25a29_grow_capacity_word(0x10, 100), 100);
        assert_eq!(w25a29_grow_capacity_word(0x8000_0010, 20), 0x8000_0018);
        assert_eq!(w25a29_grow_capacity_word(0x10, 17), 24);
    }

    #[test]
    fn lookup() {
        assert_eq!(w25a29_bucket_index(0, 16), 0);
        assert!(w25a29_bucket_index(12345, 16) < 16);
        assert_eq!(w25a29_find_node(&[], 1, 2), None);
        let mut b = vec![Vec::new(); 8];
        let i = w25a29_bucket_index(7, 8) as usize;
        b[i].push((3, 3));
        b[i].push((7, 9));
        assert_eq!(w25a29_find_node(&b, 7, 9), Some(1));
        assert_eq!(w25a29_find_node(&b, 7, 8), None);
    }
}
