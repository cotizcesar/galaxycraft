#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent97.rs
// ===========================================================================
// Wave 31 translation batch 97.
//
// Only the hash-bucket lookup is recoverable; the rest of the window is
// pm_debug graphics-layer glue (vtable calls, SEH, non-returning tails).
// Skipped: FUN_00c159a0, FUN_00c15f90, FUN_00c16010, FUN_00c16040,
// FUN_00c16070, FUN_00c160a0, FUN_00c161d0, FUN_00c16870, FUN_00c17750,
// FUN_00c178a0, FUN_00c17a10, FUN_00c17b90, FUN_00c17e90, FUN_00c17fb0,
// FUN_00c180e0, FUN_00c18220 (graphics debug layer / non-returning / SEH).

/// `0x00c15cf0` (hash step): 32-bit integer mix applied to the key before
/// masking with `bucket_count - 1`.
pub fn w31a97_hash(key: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    h >> 16 ^ h
}

/// `0x00c15cf0`: chained hash-table lookup.  `buckets[i]` is the head node
/// index (or `None`), `next[n]` the chain link and `keys[n]` the node key.
/// Returns the matching node index, `None` when the table has no buckets
/// (the retail code then calls a non-returning error path) or on a miss.
pub fn w31a97_find(
    buckets: &[Option<usize>],
    next: &[Option<usize>],
    keys: &[u32],
    key: u32,
) -> Option<usize> {
    if buckets.is_empty() {
        return None;
    }
    let slot = (w31a97_hash(key) as usize) & (buckets.len() - 1);
    let mut cur = buckets[slot];
    while let Some(n) = cur {
        if keys[n] == key {
            return Some(n);
        }
        cur = next[n];
    }
    None
}

#[cfg(test)]
mod tests_wave31_agent97 {
    use super::*;

    #[test]
    fn hash_and_find() {
        assert_eq!(w31a97_hash(0), 0);
        let mut buckets = vec![None; 4];
        let keys = [5u32, 9];
        let mut next = vec![None; 2];
        let s0 = (w31a97_hash(5) as usize) & 3;
        buckets[s0] = Some(0);
        let s1 = (w31a97_hash(9) as usize) & 3;
        if s1 == s0 {
            next[0] = Some(1);
        } else {
            buckets[s1] = Some(1);
        }
        assert_eq!(w31a97_find(&buckets, &next, &keys, 5), Some(0));
        assert_eq!(w31a97_find(&buckets, &next, &keys, 9), Some(1));
        assert_eq!(w31a97_find(&buckets, &next, &keys, 7), None);
        assert_eq!(w31a97_find(&[], &next, &keys, 5), None);
    }
}
