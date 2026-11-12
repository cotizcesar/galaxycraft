#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent32.rs
// ===========================================================================
// Wave 25 agent 32 translations (hash-map lookup and tree-empty predicate).

/// Node of the chained hash table used by `0x00848940`: next link plus a
/// two-word key (node offsets 0, 8 and 0xc; payload at +0x10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W25a32Node {
    pub next: Option<usize>,
    pub key: [u32; 2],
}

/// Mixing hash of the first key word used by `0x00848940`.
pub fn w25a32_hash(key0: u32) -> u32 {
    let mut h = (key0 >> 16 ^ key0).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    h >> 16 ^ h
}

/// `0x00848940`: looks up a two-word key in a chained hash table with
/// `bucket_count` (power of two) buckets; returns the node index (the
/// payload lives at node + 0x10).  `None` models the fatal-error path
/// (also taken when the table has no bucket array).
pub fn w25a32_hash_find(
    buckets: &[Option<usize>],
    nodes: &[W25a32Node],
    bucket_count: u32,
    key: [u32; 2],
) -> Option<usize> {
    if buckets.is_empty() {
        return None;
    }
    let idx = (w25a32_hash(key[0]) & bucket_count.wrapping_sub(1)) as usize;
    let mut cur = *buckets.get(idx)?;
    while let Some(i) = cur {
        let n = nodes.get(i)?;
        if n.key == key {
            return Some(i);
        }
        cur = n.next;
    }
    None
}

/// `0x0084d550`: true (the cleanup branch is taken) when both words at +0x20
/// and +0x24 are zero and the pointer at +0x10 differs from `base + 0xc`.
pub fn w25a32_tree_needs_cleanup(w20: u32, w24: u32, ptr10: u32, base: u32) -> bool {
    w20 == 0 && w24 == 0 && ptr10 != base.wrapping_add(0xc)
}

#[cfg(test)]
mod tests_wave25_agent32 {
    use super::*;

    #[test]
    fn hash_lookup() {
        assert_eq!(w25a32_hash(0), 0);
        let key = [5u32, 9];
        let b = (w25a32_hash(5) & 3) as usize;
        let mut buckets = vec![None; 4];
        let nodes = vec![
            W25a32Node {
                next: Some(1),
                key: [5, 1],
            },
            W25a32Node { next: None, key },
        ];
        buckets[b] = Some(0);
        assert_eq!(w25a32_hash_find(&buckets, &nodes, 4, key), Some(1));
        assert_eq!(w25a32_hash_find(&buckets, &nodes, 4, [5, 2]), None);
        assert_eq!(w25a32_hash_find(&[], &nodes, 4, key), None);
    }

    #[test]
    fn cleanup_predicate() {
        assert!(w25a32_tree_needs_cleanup(0, 0, 0x100, 0x200));
        assert!(!w25a32_tree_needs_cleanup(0, 0, 0x20c, 0x200));
        assert!(!w25a32_tree_needs_cleanup(1, 0, 0x100, 0x200));
        assert!(!w25a32_tree_needs_cleanup(0, 1, 0x100, 0x200));
    }
}
