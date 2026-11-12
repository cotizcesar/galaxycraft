#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent53.rs
// ===========================================================================
// Wave 32 agent 53 translations.

/// `0x0084f6d0`: membership test in a chained hash table keyed by a
/// `(u32, i32)` pair.  The hash mixes the first key with a murmur3-style
/// finalizer, masked by `bucket_count - 1`; each chain node is compared on
/// both key words.  `chains` is the bucket array (each bucket is a list of
/// `(key_a, key_b)` nodes).  An empty bucket array reports `false`.
pub fn w32a53_hash_contains(chains: &[Vec<(u32, i32)>], key_a: u32, key_b: i32) -> bool {
    if chains.is_empty() {
        return false;
    }
    let mut h = (key_a >> 16 ^ key_a).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    let idx = ((h >> 16 ^ h) as usize) & (chains.len().wrapping_sub(1));
    chains
        .get(idx)
        .is_some_and(|b| b.iter().any(|&(a, b2)| a == key_a && b2 == key_b))
}

#[cfg(test)]
mod tests_wave32_agent53 {
    use super::*;

    #[test]
    fn empty_is_false() {
        assert!(!w32a53_hash_contains(&[], 1, 2));
    }

    #[test]
    fn finds_inserted() {
        let mut t: Vec<Vec<(u32, i32)>> = vec![Vec::new(); 8];
        let mut h = (7u32 >> 16 ^ 7).wrapping_mul(0x85eb_ca6b);
        h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
        let i = ((h >> 16 ^ h) & 7) as usize;
        t[i].push((7, 3));
        assert!(w32a53_hash_contains(&t, 7, 3));
        assert!(!w32a53_hash_contains(&t, 7, 4));
    }
}
