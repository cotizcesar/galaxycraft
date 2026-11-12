#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent09.rs
// ===========================================================================
// Wave 32 translation batch 09.

/// `0x007e1ff0`: scans a chained hash table (bucket heads, then node chains)
/// for a node whose key (second dword) equals `wanted`.  `buckets` holds the
/// head node index plus one (0 = empty chain); `nodes` holds `(next, key)`
/// with `next` encoded the same way.  Returns `None` where the retail code
/// reaches its non-returning assertion (first bucket head is null), and
/// `Some(found)` otherwise; an empty bucket array yields `Some(false)`.
pub fn w32a09_hash_contains_key(
    buckets: &[usize],
    nodes: &[(usize, u32)],
    wanted: u32,
) -> Option<bool> {
    if buckets.is_empty() {
        return Some(false);
    }
    if buckets[0] == 0 {
        return None;
    }
    for &head in buckets {
        let mut cur = head;
        while cur != 0 {
            let (next, key) = *nodes.get(cur - 1)?;
            if key == wanted {
                return Some(true);
            }
            cur = next;
        }
    }
    Some(false)
}

#[cfg(test)]
mod tests_wave32_agent09 {
    use super::*;

    #[test]
    fn finds_across_buckets() {
        let nodes = [(0, 5), (0, 9), (2, 7)];
        let b = [1, 0, 3];
        assert_eq!(w32a09_hash_contains_key(&b, &nodes, 9), Some(true));
        assert_eq!(w32a09_hash_contains_key(&b, &nodes, 5), Some(true));
        assert_eq!(w32a09_hash_contains_key(&b, &nodes, 4), Some(false));
        assert_eq!(w32a09_hash_contains_key(&[], &nodes, 4), Some(false));
        assert_eq!(w32a09_hash_contains_key(&[0, 1], &nodes, 5), None);
    }
}
