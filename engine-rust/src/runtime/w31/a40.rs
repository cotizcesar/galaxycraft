#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent40.rs
// ===========================================================================
// Wave 31 agent 40: hash bucket lookup, regex literal compare, vector helpers.

/// `0x007e8070`: bucket hash used by the id table.  Returns the bucket index
/// for `key` given the bucket count (`count` must be non-zero).
pub fn w31a40_bucket_index(key: u32, count: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    (h >> 16 ^ h) & count.wrapping_sub(1)
}

/// `0x007e8070`: chain walk.  `buckets[i]` is the 1-based head node of bucket
/// `i` (0 = empty); `nodes[n-1] = (next, key)`.  Returns the 1-based node whose
/// key equals `key`, or `None` (the original returns the `+8` of null).
pub fn w31a40_find_node(buckets: &[u32], nodes: &[(u32, u32)], key: u32) -> Option<u32> {
    if buckets.is_empty() {
        return None;
    }
    let mut cur = buckets[w31a40_bucket_index(key, buckets.len() as u32) as usize];
    while cur != 0 {
        let (next, k) = *nodes.get(cur as usize - 1)?;
        if k == key {
            return Some(cur);
        }
        cur = next;
    }
    None
}

/// `0x007f3840`: match the literal `pat` against `text[start..end]`.  `flags`
/// bit 0x100 uses `fold_locale`, bit 0x800 uses `fold_nocase`.  Returns the
/// new position on a full match of the pattern, else `start`.
pub fn w31a40_match_literal(
    text: &[u8],
    start: usize,
    end: usize,
    pat: &[u8],
    flags: u32,
    fold_locale: impl Fn(u8) -> u8,
    fold_nocase: impl Fn(u8) -> u8,
) -> usize {
    let mut t = start;
    let mut p = 0usize;
    if flags & 0x100 != 0 {
        if start != end {
            while p != pat.len() {
                let pc = pat[p];
                p += 1;
                let tc = text[t];
                t += 1;
                if fold_locale(tc) != fold_locale(pc) {
                    return start;
                }
                if t == end {
                    return if p == pat.len() { t } else { start };
                }
            }
        }
    } else if flags & 0x800 != 0 {
        while t != end && p != pat.len() {
            let pc = pat[p];
            p += 1;
            let tc = text[t];
            t += 1;
            if fold_nocase(tc) != fold_nocase(pc) {
                return start;
            }
        }
    } else {
        while t != end {
            if p == pat.len() {
                return t;
            }
            let pc = pat[p];
            p += 1;
            let tc = text[t];
            t += 1;
            if tc != pc {
                return start;
            }
        }
        return if p == pat.len() { t } else { start };
    }
    if p == pat.len() { t } else { start }
}

/// `0x007f46c0`: builds a vector of `n` copies of `value`; `None` when
/// `n > 0x3fffffff` (the original calls a non-returning length error).
pub fn w31a40_vec_fill(n: u32, value: u32) -> Option<Vec<u32>> {
    if n > 0x3fff_ffff {
        return None;
    }
    Some(vec![value; n as usize])
}

/// `0x00800400`: reserve for `n` 4-byte elements.  `Ok(false)` for zero,
/// `Ok(true)` with capacity set, `Err(())` when `n >= 0x40000000`.
pub fn w31a40_reserve4(n: u32) -> Result<(bool, usize), ()> {
    if n == 0 {
        Ok((false, 0))
    } else if n < 0x4000_0000 {
        Ok((true, n as usize))
    } else {
        Err(())
    }
}

/// `0x00800460`: reserve for `n` 8-byte elements.  Same contract as
/// the 4-byte variant with limit `0x20000000`.
pub fn w31a40_reserve8(n: u32) -> Result<(bool, usize), ()> {
    if n == 0 {
        Ok((false, 0))
    } else if n < 0x2000_0000 {
        Ok((true, n as usize))
    } else {
        Err(())
    }
}

#[cfg(test)]
mod tests_wave31_agent40 {
    use super::*;

    #[test]
    fn hash_and_find() {
        assert_eq!(w31a40_bucket_index(0, 8), 0);
        assert!(w31a40_bucket_index(12345, 16) < 16);
        let b = w31a40_bucket_index(7, 4) as usize;
        let mut buckets = vec![0u32; 4];
        buckets[b] = 1;
        let nodes = [(2u32, 99u32), (0, 7)];
        assert_eq!(w31a40_find_node(&buckets, &nodes, 7), Some(2));
        assert_eq!(w31a40_find_node(&buckets, &nodes, 5), None);
        assert_eq!(w31a40_find_node(&[], &nodes, 7), None);
    }

    #[test]
    fn literal() {
        let id = |c: u8| c;
        let lo = |c: u8| c.to_ascii_lowercase();
        assert_eq!(w31a40_match_literal(b"abc", 0, 3, b"abc", 0, id, lo), 3);
        assert_eq!(w31a40_match_literal(b"abcd", 0, 4, b"abc", 0, id, lo), 3);
        assert_eq!(w31a40_match_literal(b"ab", 0, 2, b"abc", 0, id, lo), 0);
        assert_eq!(w31a40_match_literal(b"abx", 0, 3, b"abc", 0, id, lo), 0);
        assert_eq!(w31a40_match_literal(b"ABC", 0, 3, b"abc", 0x800, id, lo), 3);
        assert_eq!(w31a40_match_literal(b"ABC", 0, 3, b"abc", 0x100, lo, id), 3);
        assert_eq!(w31a40_match_literal(b"AB", 0, 2, b"abc", 0x100, lo, id), 0);
    }

    #[test]
    fn vectors() {
        assert_eq!(w31a40_vec_fill(3, 9), Some(vec![9, 9, 9]));
        assert_eq!(w31a40_vec_fill(0x4000_0000, 1), None);
        assert_eq!(w31a40_reserve4(0), Ok((false, 0)));
        assert_eq!(w31a40_reserve4(4), Ok((true, 4)));
        assert!(w31a40_reserve4(0x4000_0000).is_err());
        assert_eq!(w31a40_reserve8(2), Ok((true, 2)));
        assert!(w31a40_reserve8(0x2000_0000).is_err());
    }
}
