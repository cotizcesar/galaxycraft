#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent58.rs
// ===========================================================================
// Wave 33 agent 58: runtime helpers.

/// `0x00cadb44`: replicates the low six bits of `v` into bytes 1 and 2 and
/// keeps `v` itself in the low bytes (`(m << 8 | m) << 16 | v`).
pub fn w33a58_replicate_mask(v: u32) -> u32 {
    (((v & 0x3f) << 8 | (v & 0x3f)) << 16) | v
}

/// `0x00cae1d0`: ASCII case-insensitive compare of at most `count` bytes of
/// two NUL-terminated strings; returns -1, 0 or 1.  A string ending (slice
/// end counts as NUL) stops the scan and the raw bytes are compared.
pub fn w33a58_strnicmp(a: &[u8], b: &[u8], count: i32) -> i32 {
    let mut n = count;
    let mut i = 0usize;
    let lower = |c: u8| if c > 0x40 && c < 0x5b { c + 0x20 } else { c };
    while n != 0 {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x == 0 || y == 0 {
            return if x == y {
                0
            } else if x < y {
                -1
            } else {
                1
            };
        }
        let (lx, ly) = (lower(x), lower(y));
        if lx != ly {
            return if lx < ly { -1 } else { 1 };
        }
        i += 1;
        n -= 1;
    }
    0
}

#[cfg(test)]
mod tests_wave33_agent58 {
    use super::*;

    #[test]
    fn mask() {
        assert_eq!(w33a58_replicate_mask(0x1f), 0x1f1f_001f);
    }

    #[test]
    fn cmp() {
        assert_eq!(w33a58_strnicmp(b"AbC", b"aBc", 3), 0);
        assert_eq!(w33a58_strnicmp(b"abd", b"abc", 3), 1);
        assert_eq!(w33a58_strnicmp(b"ab", b"abc", 5), -1);
        assert_eq!(w33a58_strnicmp(b"x", b"y", 0), 0);
    }
}
