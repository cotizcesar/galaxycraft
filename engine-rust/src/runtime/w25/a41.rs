#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent41.rs
// ===========================================================================
// Wave 25 agent 41 translations (runtime glue helpers).

/// `0x0087b610`: true when the string object's length field (offset 0x18 of
/// the inner record) is non-zero.
pub fn w25a41_inner_len_nonzero(inner_len: i32) -> bool {
    inner_len != 0
}

/// `0x0087b070`: when `lookup(key)` returns zero, forwards to `forward(key)`
/// (callee at 979390 with the inner pointer); otherwise does nothing.
/// Returns whether the forward happened.
pub fn w25a41_forward_if_missing(
    key: u32,
    mut lookup: impl FnMut(u32) -> i32,
    mut forward: impl FnMut(u32),
) -> bool {
    if lookup(key) == 0 {
        forward(key);
        true
    } else {
        false
    }
}

/// `0x0087b6b0`: string-assign prologue.  When the dirty flag (offset 0x3c)
/// is set, the inner buffer's first byte and length (offset 0x18) are cleared
/// and the flag reset.  Returns the C-string length of `src` (bytes up to the
/// first NUL, or the slice length) that is passed to the follow-up call.
pub fn w25a41_assign_prologue(
    dirty: &mut bool,
    buf_first: &mut u8,
    buf_len: &mut u32,
    src: &[u8],
) -> usize {
    if *dirty {
        *buf_first = 0;
        *buf_len = 0;
        *dirty = false;
    }
    src.iter().position(|&b| b == 0).unwrap_or(src.len())
}

/// `0x0087ba30`: hash for a u16 key (murmur3 finalizer-style constants
/// `0x85ebca6b`, `0xc2b2ae35`), masked by `bucket_count - 1`.
pub fn w25a41_hash_bucket(key: u16, bucket_count: u32) -> usize {
    let mut h = (key as u32).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    ((h >> 16 ^ h) & bucket_count.wrapping_sub(1)) as usize
}

/// `0x0087ba30`: chained-bucket lookup of a u16 key.  `heads[b]` is the first
/// node index (or `None`); `node(i)` yields `(next, key)`.  Returns the
/// matching node (value lives at node + 8) or `None` (retail aborts).
pub fn w25a41_hash_find(
    heads: &[Option<usize>],
    key: u16,
    node: impl Fn(usize) -> (Option<usize>, u16),
) -> Option<usize> {
    let bucket = w25a41_hash_bucket(key, heads.len() as u32);
    let mut cur = *heads.get(bucket)?;
    while let Some(i) = cur {
        let (next, k) = node(i);
        if k == key {
            return Some(i);
        }
        cur = next;
    }
    None
}

#[cfg(test)]
mod tests_wave25_agent41 {
    use super::*;

    #[test]
    fn small_predicates() {
        assert!(!w25a41_inner_len_nonzero(0));
        assert!(w25a41_inner_len_nonzero(-1));
        let mut n = 0;
        assert!(w25a41_forward_if_missing(5, |_| 0, |k| n = k));
        assert_eq!(n, 5);
        assert!(!w25a41_forward_if_missing(5, |_| 1, |_| panic!()));
    }

    #[test]
    fn prologue() {
        let (mut d, mut c, mut l) = (true, 9u8, 4u32);
        assert_eq!(w25a41_assign_prologue(&mut d, &mut c, &mut l, b"abc\0d"), 3);
        assert!(!d && c == 0 && l == 0);
        let (mut d, mut c, mut l) = (false, 9u8, 4u32);
        assert_eq!(w25a41_assign_prologue(&mut d, &mut c, &mut l, b"ab"), 2);
        assert_eq!((c, l), (9, 4));
    }

    #[test]
    fn hash_lookup() {
        assert_eq!(w25a41_hash_bucket(0, 8), 0);
        let b = w25a41_hash_bucket(7, 8);
        assert!(b < 8);
        let mut heads = vec![None; 8];
        heads[b] = Some(0);
        let nodes = [(Some(1), 99u16), (None, 7u16)];
        assert_eq!(w25a41_hash_find(&heads, 7, |i| nodes[i]), Some(1));
        assert_eq!(w25a41_hash_find(&heads, 8, |i| nodes[i]), None);
    }
}
