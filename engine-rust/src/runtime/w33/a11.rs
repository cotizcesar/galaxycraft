#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent11.rs
// ===========================================================================
// Wave 33 agent 11 translations.

/// 0x006b0210: cached value getter; flag bit 14 of `flags` selects the cache.
pub fn w33a11_cached_value(
    flags: u32,
    cache: &mut u32,
    compute: impl FnOnce() -> u32,
    out: &mut u32,
) -> u32 {
    if (flags >> 14) & 1 == 0 {
        let v = compute();
        *cache = v;
        *out = v;
    } else {
        *out = *cache;
    }
    1
}

/// 0x0084f650: forward to a virtual slot (+0x5c) when the predicate holds.
pub fn w33a11_forward_if(pred: bool, forward: impl FnOnce()) {
    if pred {
        forward();
    }
}

/// 0x00cbce00: CRT `strtoull` core. Returns (value, end index, overflow).
pub fn w33a11_strtoull(s: &[u8], base: u32) -> (u64, usize, bool) {
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let mut i = 0;
    while matches!(at(i), b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let sign = at(i);
    if sign == b'-' || sign == b'+' {
        i += 1;
    }
    let mut base = base;
    let hex = at(i + 1) == b'x' || at(i + 1) == b'X';
    if base == 0 {
        if at(i) != b'0' {
            base = 10;
        } else if !hex {
            base = 8;
        } else {
            base = 16;
            i += 2;
        }
    } else if base == 16 && at(i) == b'0' && hex {
        i += 2;
    }
    let max = u64::MAX / base as u64;
    let rem = (u64::MAX % base as u64) as u32;
    let mut v: u64 = 0;
    let mut ovf = false;
    loop {
        let c = at(i);
        let d = match c {
            b'0'..=b'9' => (c - b'0') as u32,
            b'a'..=b'z' => (c - b'a') as u32 + 10,
            b'A'..=b'Z' => (c - b'A') as u32 + 10,
            _ => u32::MAX,
        };
        if d >= base {
            break;
        }
        if v < max || (v == max && d <= rem) {
            v = v * base as u64 + d as u64;
        } else {
            ovf = true;
        }
        i += 1;
    }
    if ovf {
        v = u64::MAX;
    }
    if sign == b'-' && !ovf {
        v = v.wrapping_neg();
    }
    (v, i, ovf)
}

#[cfg(test)]
mod tests_wave33_agent11 {
    use super::*;

    #[test]
    fn cached() {
        let (mut c, mut o) = (0, 0);
        assert_eq!(w33a11_cached_value(0, &mut c, || 7, &mut o), 1);
        assert_eq!((c, o), (7, 7));
        assert_eq!(w33a11_cached_value(1 << 14, &mut c, || 9, &mut o), 1);
        assert_eq!(o, 7);
    }

    #[test]
    fn forward() {
        let mut n = 0;
        w33a11_forward_if(false, || n += 1);
        w33a11_forward_if(true, || n += 1);
        assert_eq!(n, 1);
    }

    #[test]
    fn strtoull() {
        assert_eq!(w33a11_strtoull(b"  123x", 10), (123, 5, false));
        assert_eq!(w33a11_strtoull(b"0x1F", 0), (31, 4, false));
        assert_eq!(w33a11_strtoull(b"017", 0), (15, 3, false));
        assert_eq!(w33a11_strtoull(b"-1", 10).0, u64::MAX);
        assert_eq!(
            w33a11_strtoull(b"18446744073709551616", 10),
            (u64::MAX, 20, true)
        );
        assert_eq!(w33a11_strtoull(b"18446744073709551615", 10).0, u64::MAX);
    }
}
