#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent73.rs
// ===========================================================================
// Wave 33 agent 73: strtol-style integer parser.

/// `0x00cbca30`: CRT `strtol`.  Skips leading whitespace, an optional sign
/// and (for base 0 or 16) a `0x` prefix, then accumulates digits with
/// wrapping signed arithmetic.  Overflow is flagged when the running value
/// decreases; the result then saturates to `i32::MAX` (or `i32::MIN` for a
/// negative sign, represented as `0x7fffffff + 1`).  Returns
/// `(value, end_index, overflow)`.  Bytes past the slice read as NUL.
pub fn w33a73_strtol(s: &[u8], base: u32) -> (i32, usize, bool) {
    let at = |i: usize| -> u8 { s.get(i).copied().unwrap_or(0) };
    let mut p = 0usize;
    let mut c;
    loop {
        c = at(p);
        p += 1;
        if !matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
            break;
        }
    }
    let sign = c;
    let mut cur = c;
    if c == b'-' || c == b'+' {
        cur = at(p);
        p += 1;
    }
    let mut base = base;
    let mut skip = false;
    if base == 0 {
        if cur != b'0' {
            base = 10;
        } else if at(p) != b'x' && at(p) != b'X' {
            base = 8;
        } else {
            base = 16;
            skip = true;
        }
    } else if base == 16 && cur == b'0' && (at(p) == b'x' || at(p) == b'X') {
        skip = true;
    }
    if skip {
        cur = at(p + 1);
        p += 2;
    }
    let mut overflow = false;
    let mut acc: i32 = 0;
    loop {
        let d: u32 = match cur {
            b'0'..=b'9' => (cur - b'0') as u32,
            b'a'..=b'z' => (cur - b'a') as u32 + 10,
            b'A'..=b'Z' => (cur - b'A') as u32 + 10,
            _ => u32::MAX,
        };
        if base <= d {
            break;
        }
        let next = acc.wrapping_mul(base as i32).wrapping_add(d as i32);
        cur = at(p);
        if next < acc {
            overflow = true;
        }
        p += 1;
        acc = next;
    }
    let end = p - 1;
    let mut val = acc;
    if overflow {
        val = (i32::MAX as i64 + (sign == b'-') as i64) as i32;
        if acc == val {
            overflow = false;
        }
    }
    if sign == b'-' && !overflow {
        val = val.wrapping_neg();
    }
    (val, end, overflow)
}

#[cfg(test)]
mod tests_wave33_agent73 {
    use super::*;

    #[test]
    fn parses_bases() {
        assert_eq!(w33a73_strtol(b"  -42x", 10), (-42, 5, false));
        assert_eq!(w33a73_strtol(b"0x1F", 0), (31, 4, false));
        assert_eq!(w33a73_strtol(b"017", 0), (15, 3, false));
        assert_eq!(w33a73_strtol(b"+7", 0), (7, 2, false));
        assert_eq!(w33a73_strtol(b"zz", 36), (1295, 2, false));
    }

    #[test]
    fn overflow() {
        let (v, _, o) = w33a73_strtol(b"99999999999", 10);
        assert!(o);
        assert_eq!(v, i32::MAX);
    }
}
