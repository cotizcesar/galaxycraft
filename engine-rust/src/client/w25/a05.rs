#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent05.rs
// ===========================================================================
// Wave 25 agent 05: small client-side helpers (deque pop, hash bucket, wrappers).

/// `0x00878280`: calls the shared helper with the global at the first
/// slot and flag `1`.
pub fn w25a05_call_with_flag_a(global: u32, helper: &mut dyn FnMut(u32, u32)) {
    helper(global, 1);
}

/// `0x008782d0`: same as the sibling wrapper but with the second global.
pub fn w25a05_call_with_flag_b(global: u32, helper: &mut dyn FnMut(u32, u32)) {
    helper(global, 1);
}

/// `0x00887f10`: returns `Some(2)` when the probe is non-zero and the
/// global is not `-1`; otherwise the original tail-calls a non-returning
/// handler (modelled as `None`).
pub fn w25a05_state_or_fail(probe: i32, global: i32) -> Option<u32> {
    if probe != 0 && global != -1 {
        Some(2)
    } else {
        None
    }
}

/// `0x00864630`: stores `value` into `*out` when the two iterators are
/// equal; otherwise the original raises an error (modelled as `false`).
pub fn w25a05_store_if_equal(out: &mut u32, a: i32, b: i32, value: u32) -> bool {
    if a != b {
        return false;
    }
    *out = value;
    true
}

/// Block/slot pair of a deque element (16 elements per block).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W25a05DequePos {
    pub block: usize,
    pub slot: usize,
}

/// `0x0087c7d0`: pops the front of a block deque (16 entries per block,
/// block map length power of two).  Returns the position read, and
/// advances `start` while decrementing `size`.
pub fn w25a05_deque_pop_front(start: &mut u32, size: &mut u32, map_len: u32) -> W25a05DequePos {
    let block = ((*start >> 4) & map_len.wrapping_sub(1)) as usize;
    let slot = (*start & 0xf) as usize;
    *start = start.wrapping_add(1);
    *size = size.wrapping_sub(1);
    W25a05DequePos { block, slot }
}

/// `0x0087be60`: bucket index of a 16-bit key in the hash table
/// (`buckets` is a power of two).
pub fn w25a05_hash_bucket(key: u32, buckets: u32) -> u32 {
    let k = key & 0xffff;
    let a = k.wrapping_mul(0x85ebca6b);
    let b = (a >> 13 ^ a).wrapping_mul(0xc2b2ae35);
    (b >> 16 ^ b) & buckets.wrapping_sub(1)
}

#[cfg(test)]
mod tests_wave25_agent05 {
    use super::*;

    #[test]
    fn wrappers() {
        let mut got = vec![];
        w25a05_call_with_flag_a(7, &mut |a, b| got.push((a, b)));
        w25a05_call_with_flag_b(9, &mut |a, b| got.push((a, b)));
        assert_eq!(got, vec![(7, 1), (9, 1)]);
    }

    #[test]
    fn state_and_store() {
        assert_eq!(w25a05_state_or_fail(1, 3), Some(2));
        assert_eq!(w25a05_state_or_fail(1, -1), None);
        assert_eq!(w25a05_state_or_fail(0, 3), None);
        let mut o = 0;
        assert!(!w25a05_store_if_equal(&mut o, 1, 2, 5));
        assert!(w25a05_store_if_equal(&mut o, 2, 2, 5));
        assert_eq!(o, 5);
    }

    #[test]
    fn deque() {
        let mut s = 0x1f;
        let mut n = 3;
        let p = w25a05_deque_pop_front(&mut s, &mut n, 4);
        assert_eq!(p, W25a05DequePos { block: 1, slot: 15 });
        assert_eq!((s, n), (0x20, 2));
    }

    #[test]
    fn hash() {
        assert_eq!(w25a05_hash_bucket(0, 8), 0);
        assert!(w25a05_hash_bucket(0x1234, 16) < 16);
        assert_eq!(
            w25a05_hash_bucket(0x11234, 16),
            w25a05_hash_bucket(0x1234, 16)
        );
    }
}
