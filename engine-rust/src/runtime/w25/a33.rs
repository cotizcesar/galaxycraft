#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent33.rs
// ===========================================================================
// Wave 25 agent 33: hash-bucket lookup helper.

/// Integer mix used by the bucket index (shared by the hash-set routines).
pub fn w25a33_mix_hash(key: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    h >> 16 ^ h
}

/// `0x0084ef00`: finds the node whose 64-bit key (two u32 words) equals
/// `key` in a chained hash table of `bucket_count` buckets (power of two).
/// Each bucket is a singly linked chain of `(next, key_lo, key_hi)` nodes.
/// Returns `Some((bucket, node_index))` on hit, `None` when the table is
/// empty or the key is absent (the retail code then calls a throw helper).
pub fn w25a33_find_node(
    buckets: &[Option<usize>],
    nodes: &[(Option<usize>, u32, u32)],
    key: (u32, u32),
) -> Option<(usize, usize)> {
    if buckets.is_empty() {
        return None;
    }
    let b = (w25a33_mix_hash(key.0) as usize) & (buckets.len() - 1);
    let mut cur = buckets[b];
    while let Some(i) = cur {
        let n = nodes.get(i)?;
        if n.1 == key.0 && n.2 == key.1 {
            return Some((b, i));
        }
        cur = n.0;
    }
    None
}

#[cfg(test)]
mod tests_wave25_agent33 {
    use super::*;

    #[test]
    fn mix_constants() {
        assert_eq!(w25a33_mix_hash(0), 0);
        assert_ne!(w25a33_mix_hash(1), w25a33_mix_hash(2));
    }

    #[test]
    fn find_in_chain() {
        let k = (7u32, 9u32);
        let b = (w25a33_mix_hash(7) as usize) & 3;
        let mut buckets = vec![None; 4];
        buckets[b] = Some(0);
        let nodes = vec![(Some(1), 1, 1), (None, 7, 9)];
        assert_eq!(w25a33_find_node(&buckets, &nodes, k), Some((b, 1)));
        assert_eq!(w25a33_find_node(&buckets, &nodes, (7, 8)), None);
        assert_eq!(w25a33_find_node(&[], &nodes, k), None);
    }
}
