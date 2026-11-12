#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent28.rs
// ===========================================================================
// Wave 25 agent 28: small container helpers (capacity growth, refcount
// release loop, murmur-style bucket hash and chain lookup).

/// Murmur3-style finalizer used by the hash-map bucket selection
/// (shared by the lookups below).
pub fn w25a28_mix(key: u32) -> u32 {
    let a = (key >> 16 ^ key).wrapping_mul(0x85EB_CA6B);
    let b = (a >> 13 ^ a).wrapping_mul(0xC2B2_AE35);
    b ^ (b >> 16)
}

/// `0x00836260`: growth step of a vector whose word at +8 packs a 31-bit
/// capacity and a high flag bit.  Returns `Some(new_word)` (flag preserved)
/// when `wanted` exceeds the capacity: new capacity is
/// `max(cap + cap/2, wanted)`.  `None` when no growth is needed.
pub fn w25a28_grow_capacity_word(word: u32, wanted: u32) -> Option<u32> {
    let cap = word & 0x7fff_ffff;
    if cap >= wanted {
        return None;
    }
    let grown = ((word >> 1) & 0x3fff_ffff) + cap;
    let new_cap = grown.max(wanted);
    Some(((word ^ new_cap) & 0x7fff_ffff) ^ word)
}

/// `0x008367e0`: releases every refcounted slot of an array of
/// `(id, Option<refcount>)` pairs (stride 8).  Each count is decremented
/// atomically; when the previous value was 1 the `dispose` callback runs
/// with the slot index.  Returns how many slots were disposed.
pub fn w25a28_release_refs(
    slots: &[(u32, Option<&std::sync::atomic::AtomicI32>)],
    mut dispose: impl FnMut(usize),
) -> usize {
    let mut n = 0;
    for (i, (_, rc)) in slots.iter().enumerate() {
        if let Some(rc) = rc
            && rc.fetch_sub(1, std::sync::atomic::Ordering::SeqCst) == 1
        {
            dispose(i);
            n += 1;
        }
    }
    n
}

/// `0x008364e0`: bucket-chain search by one `u32` key (node key at +8).
/// `buckets` is a power-of-two table of chains; returns `(bucket, pos)`
/// of the match, or `None` (the retail code then inserts a node).
pub fn w25a28_find_u32(buckets: &[Vec<u32>], key: u32) -> Option<(usize, usize)> {
    if buckets.is_empty() {
        return None;
    }
    let b = (w25a28_mix(key) as usize) & (buckets.len() - 1);
    buckets[b].iter().position(|&k| k == key).map(|p| (b, p))
}

/// `0x00837ae0` and `0x00837d00`: bucket-chain search by a two-word key
/// (words at node +8/+12; the value lives at node +0x10).  The first
/// word is hashed.  `None` is the retail abort path.
pub fn w25a28_find_pair(buckets: &[Vec<(u32, u32)>], key: (u32, u32)) -> Option<(usize, usize)> {
    if buckets.is_empty() {
        return None;
    }
    let b = (w25a28_mix(key.0) as usize) & (buckets.len() - 1);
    buckets[b].iter().position(|&k| k == key).map(|p| (b, p))
}

#[cfg(test)]
mod tests_wave25_agent28 {
    use super::*;
    use std::sync::atomic::AtomicI32;

    #[test]
    fn grow() {
        assert_eq!(w25a28_grow_capacity_word(10, 5), None);
        assert_eq!(w25a28_grow_capacity_word(10, 11), Some(15));
        assert_eq!(w25a28_grow_capacity_word(10, 100), Some(100));
        assert_eq!(w25a28_grow_capacity_word(0x8000_0004, 5), Some(0x8000_0006));
    }

    #[test]
    fn release() {
        let a = AtomicI32::new(1);
        let b = AtomicI32::new(2);
        let slots = [(0, Some(&a)), (1, None), (2, Some(&b))];
        let mut hit = vec![];
        assert_eq!(w25a28_release_refs(&slots, |i| hit.push(i)), 1);
        assert_eq!(hit, vec![0]);
        assert_eq!(b.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn lookup() {
        assert_eq!(w25a28_mix(0), 0);
        let mut t: Vec<Vec<u32>> = vec![vec![]; 8];
        let b = (w25a28_mix(77) as usize) & 7;
        t[b].push(5);
        t[b].push(77);
        assert_eq!(w25a28_find_u32(&t, 77), Some((b, 1)));
        assert_eq!(w25a28_find_u32(&t, 78), None);
        let mut p: Vec<Vec<(u32, u32)>> = vec![vec![]; 4];
        let b = (w25a28_mix(9) as usize) & 3;
        p[b].push((9, 1));
        assert_eq!(w25a28_find_pair(&p, (9, 1)), Some((b, 0)));
        assert_eq!(w25a28_find_pair(&p, (9, 2)), None);
    }
}
