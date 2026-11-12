#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent20.rs
// ===========================================================================
// Wave 31 translation batch 20: hash-bucket helpers used by the render
// command-list containers.  The TLS/SChannel certificate routines in this
// assignment are online-service code and were skipped.

/// Murmur-style finaliser shared by the two bucket routines below
/// (constants `-0x7a143595` and `-0x3d4d51cb` as `u32`).
pub fn w31a20_hash_mix(key: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    h >> 16 ^ h
}

/// Hash map node: one `(key, value)` entry of a bucket chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W31a20Node<V> {
    pub key: u32,
    pub value: V,
}

/// Bucket table; `buckets.len()` must be a power of two (mask = len - 1).
#[derive(Clone, Debug, Default)]
pub struct W31a20Table<V> {
    pub buckets: Vec<Vec<W31a20Node<V>>>,
}

impl<V> W31a20Table<V> {
    fn bucket_index(&self, key: u32) -> Option<usize> {
        if self.buckets.is_empty() {
            return None;
        }
        Some((w31a20_hash_mix(key) as usize) & (self.buckets.len() - 1))
    }
}

/// `0x00c15df0`: looks up `key` in the bucket table and returns a reference
/// to the stored value.  The retail code calls a non-returning error path
/// when the table is unallocated or the key is absent; here both are `None`.
pub fn w31a20_lookup<V>(t: &W31a20Table<V>, key: u32) -> Option<&V> {
    let idx = t.bucket_index(key)?;
    t.buckets[idx]
        .iter()
        .find(|n| n.key == key)
        .map(|n| &n.value)
}

/// `0x00c41460`: appends `value` to the vector stored under the key of
/// `node_key` (skipped when `None`; a missing entry is the non-returning
/// error path, reported as `false`), then always appends `value` to
/// `shared`.  Returns `true` on success.
pub fn w31a20_push_both(
    t: &mut W31a20Table<Vec<u32>>,
    node_key: Option<u32>,
    shared: &mut Vec<u32>,
    value: u32,
) -> bool {
    if let Some(k) = node_key {
        let Some(idx) = t.bucket_index(k) else {
            return false;
        };
        match t.buckets[idx].iter_mut().find(|n| n.key == k) {
            Some(n) => n.value.push(value),
            None => return false,
        }
    }
    shared.push(value);
    true
}

#[cfg(test)]
mod tests_wave31_agent20 {
    use super::*;

    fn table() -> W31a20Table<Vec<u32>> {
        let mut t = W31a20Table {
            buckets: vec![Vec::new(); 4],
        };
        for k in [1u32, 2, 3, 100] {
            let i = (w31a20_hash_mix(k) as usize) & 3;
            t.buckets[i].push(W31a20Node {
                key: k,
                value: vec![k],
            });
        }
        t
    }

    #[test]
    fn hash_and_lookup() {
        assert_eq!(w31a20_hash_mix(0), 0);
        let t = table();
        assert_eq!(w31a20_lookup(&t, 100), Some(&vec![100]));
        assert_eq!(w31a20_lookup(&t, 7), None);
        let e: W31a20Table<u32> = W31a20Table { buckets: vec![] };
        assert_eq!(w31a20_lookup(&e, 1), None);
    }

    #[test]
    fn push_both() {
        let mut t = table();
        let mut s = vec![];
        assert!(w31a20_push_both(&mut t, Some(2), &mut s, 9));
        assert_eq!(w31a20_lookup(&t, 2), Some(&vec![2, 9]));
        assert_eq!(s, vec![9]);
        assert!(w31a20_push_both(&mut t, None, &mut s, 5));
        assert_eq!(s, vec![9, 5]);
        assert!(!w31a20_push_both(&mut t, Some(77), &mut s, 1));
    }
}
