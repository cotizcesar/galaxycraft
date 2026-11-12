#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent03.rs
// ===========================================================================
// Wave 25 translation batch 03.
//
// Hash-table helpers of the retail unordered containers (murmur3 finalizer
// bucket selection).  Bucket model: `buckets[i]` is the chain of entries.

/// Bucket hash shared by both routines (inlined in `0x007fe730` and
/// `0x00808980`): murmur3 `fmix32` with a final mask by `bucket_count - 1`.
pub fn w25a03_hash_mix(key: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    h >> 16 ^ h
}

/// Bucket index for `key`; `None` when the table has no buckets (retail
/// calls a non-returning allocator/throw helper in that case).
pub fn w25a03_bucket_index(key: u32, bucket_count: usize) -> Option<usize> {
    if bucket_count == 0 {
        return None;
    }
    Some((w25a03_hash_mix(key) as usize) & (bucket_count - 1))
}

/// `0x007fe730`: for every key of the source set, finds its entry in the
/// destination table and overwrites the entry value with `value`.  Returns
/// `Err(key)` for the first key not present (retail hits a non-returning
/// helper there) or `Err(u32::MAX)` style never; empty table yields `Err`.
pub fn w25a03_set_all(
    buckets: &mut [Vec<(u32, u32)>],
    source_keys: &[u32],
    value: u32,
) -> Result<(), u32> {
    for &k in source_keys {
        let idx = w25a03_bucket_index(k, buckets.len()).ok_or(k)?;
        match buckets[idx].iter_mut().find(|e| e.0 == k) {
            Some(e) => e.1 = value,
            None => return Err(k),
        }
    }
    Ok(())
}

/// `0x00808980`: checks every key of a range (elements are 32 bytes wide,
/// key first) against the table; `Err(key)` for the first missing one.
pub fn w25a03_check_all(buckets: &[Vec<u32>], keys: &[u32]) -> Result<(), u32> {
    for &k in keys {
        let idx = w25a03_bucket_index(k, buckets.len()).ok_or(k)?;
        if !buckets[idx].contains(&k) {
            return Err(k);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests_wave25_agent03 {
    use super::*;

    #[test]
    fn hash_and_index() {
        assert_eq!(w25a03_hash_mix(0), 0);
        assert_eq!(w25a03_bucket_index(5, 0), None);
        let i = w25a03_bucket_index(12345, 8).unwrap();
        assert!(i < 8);
    }

    #[test]
    fn set_all_updates_and_fails() {
        let mut b: Vec<Vec<(u32, u32)>> = vec![Vec::new(); 8];
        for k in [1u32, 2, 3] {
            let i = w25a03_bucket_index(k, 8).unwrap();
            b[i].push((k, 0));
        }
        assert_eq!(w25a03_set_all(&mut b, &[1, 3], 9), Ok(()));
        let i = w25a03_bucket_index(3, 8).unwrap();
        assert_eq!(b[i][0].1, 9);
        assert_eq!(w25a03_set_all(&mut b, &[1, 77], 1), Err(77));
        assert_eq!(w25a03_set_all(&mut [], &[1], 1), Err(1));
    }

    #[test]
    fn check_all_reports_missing() {
        let mut b: Vec<Vec<u32>> = vec![Vec::new(); 4];
        let i = w25a03_bucket_index(10, 4).unwrap();
        b[i].push(10);
        assert_eq!(w25a03_check_all(&b, &[10]), Ok(()));
        assert_eq!(w25a03_check_all(&b, &[10, 11]), Err(11));
    }
}
