#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent123.rs
// ===========================================================================
// Wave 31 agent 123: CRT string-to-integer helpers and small runtime glue.

fn w31a123_is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn w31a123_digit(c: u8, base: u32) -> Option<u32> {
    let d = (c as char).to_digit(36)?;
    if d < base { Some(d) } else { None }
}

/// Result of the shared strtol-family scanner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W31a123Scan {
    /// Magnitude (saturated to all-ones on overflow).
    pub value: u64,
    /// Index one past the last consumed byte (0 when nothing parsed).
    pub end: usize,
    /// True when the magnitude overflowed `max`.
    pub overflow: bool,
    /// True when a leading '-' was present.
    pub negative: bool,
}

/// Scanner shared by `0x00c84d10` (32-bit) and `0x00c84f51` (64-bit): skips
/// whitespace, an optional sign, resolves base 0 (octal/decimal/hex) and an
/// optional `0x` prefix for base 16, then accumulates digits.  A bad base
/// (negative, 1, > 36) or no digits yields value 0 and `end == 0`.
fn w31a123_scan(s: &[u8], base: i32, max: u64) -> W31a123Scan {
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let mut p = 0;
    while w31a123_is_space(at(p)) {
        p += 1;
    }
    let mut negative = false;
    if at(p) == b'-' || at(p) == b'+' {
        negative = at(p) == b'-';
        p += 1;
    }
    let none = W31a123Scan {
        value: 0,
        end: 0,
        overflow: false,
        negative,
    };
    if !(0..=36).contains(&base) || base == 1 {
        return none;
    }
    let mut base = base as u32;
    let hex = at(p) == b'0' && matches!(at(p + 1), b'x' | b'X');
    if base == 0 {
        base = if at(p) != b'0' {
            10
        } else if hex {
            16
        } else {
            8
        };
    }
    if base == 16 && hex {
        p += 2;
    }
    let start = p;
    let mut value: u64 = 0;
    let mut overflow = false;
    while let Some(d) = w31a123_digit(at(p), base) {
        match value
            .checked_mul(base as u64)
            .and_then(|v| v.checked_add(d as u64))
        {
            Some(v) if v <= max => value = v,
            _ => overflow = true,
        }
        p += 1;
    }
    if p == start {
        return none;
    }
    if overflow {
        value = max;
    }
    W31a123Scan {
        value,
        end: p,
        overflow,
        negative,
    }
}

/// `0x00c84d10`: `strtoul` core.  Returns `(value, end, range_error)`;
/// on overflow the value is `u32::MAX` and the sign is treated as `+`.
pub fn w31a123_strtoul(s: &[u8], base: i32) -> (u32, usize, bool) {
    let r = w31a123_scan(s, base, u32::MAX as u64);
    let mut v = r.value as u32;
    if r.negative && !r.overflow {
        v = v.wrapping_neg();
    }
    (v, r.end, r.overflow)
}

/// `0x00c84f51`: `_strtoui64` core, 64-bit twin of the above.
pub fn w31a123_strtoull(s: &[u8], base: i32) -> (u64, usize, bool) {
    let r = w31a123_scan(s, base, u64::MAX);
    let mut v = r.value;
    if r.negative && !r.overflow {
        v = v.wrapping_neg();
    }
    (v, r.end, r.overflow)
}

/// `0x00c84c6b`: `strtol`.  Range errors clamp to `i32::MAX`/`i32::MIN`
/// (the original returns `(neg) + 0x7fffffff`).  Returns `(value, end, erange)`.
pub fn w31a123_strtol(s: &[u8], base: i32) -> (i32, usize, bool) {
    let r = w31a123_scan(s, base, u32::MAX as u64);
    let end = r.end;
    let v = r.value as u32;
    let ok = (!r.overflow)
        && if r.negative {
            v <= 0x8000_0000
        } else {
            v < 0x8000_0000
        };
    if ok || (end == 0 && v == 0) {
        let out = if r.negative { v.wrapping_neg() } else { v };
        return (out as i32, end, false);
    }
    (if r.negative { i32::MIN } else { i32::MAX }, end, true)
}

/// `0x00c84e83`: `_strtoi64` (returned as `i64`).  Clamps to the i64 range
/// on overflow, setting the range-error flag.
pub fn w31a123_strtoll(s: &[u8], base: i32) -> (i64, usize, bool) {
    let r = w31a123_scan(s, base, u64::MAX);
    let v = r.value;
    let ok = (!r.overflow)
        && if r.negative {
            v <= 0x8000_0000_0000_0000
        } else {
            v < 0x8000_0000_0000_0000
        };
    if ok || (r.end == 0 && v == 0) {
        let out = if r.negative { v.wrapping_neg() } else { v };
        return (out as i64, r.end, false);
    }
    (if r.negative { i64::MIN } else { i64::MAX }, r.end, true)
}

/// `0x00c849e6`: linear lookup of an errno in `(code, message)` pairs;
/// falls back to `"unknown error"`.
pub fn w31a123_strerror<'a>(table: &[(i32, &'a str)], code: i32) -> &'a str {
    table
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, m)| *m)
        .unwrap_or("unknown error")
}

/// `0x00c848fe`: recovers the original allocation from an over-aligned
/// block (`0x23` extra bytes).  `ptr` is the user pointer, `stored` the
/// pointer saved just before it.  Returns `(original, adjusted_size)` when
/// `ptr - stored - 4 < 0x20`, else `None` (the original aborts).
pub fn w31a123_unalign(ptr: u32, stored: u32, size: u32) -> Option<(u32, u32)> {
    if ptr.wrapping_sub(stored).wrapping_sub(4) < 0x20 {
        Some((stored, size.wrapping_add(0x23)))
    } else {
        None
    }
}

/// `0x00c84be3`: decrements the per-index use count (when `index != 0`);
/// returns true when the object should be torn down (index 0 or the count
/// dropped to <= 0 as a signed byte).
pub fn w31a123_release_ref(counts: &mut [u8], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    counts[index] = counts[index].wrapping_sub(1);
    (counts[index] as i8) < 1
}

#[cfg(test)]
mod tests_wave31_agent123 {
    use super::*;

    #[test]
    fn strtoul_cases() {
        assert_eq!(w31a123_strtoul(b"  42x", 10), (42, 4, false));
        assert_eq!(w31a123_strtoul(b"0x1F", 0), (31, 4, false));
        assert_eq!(w31a123_strtoul(b"017", 0), (15, 3, false));
        assert_eq!(w31a123_strtoul(b"zz", 36), (35 * 36 + 35, 2, false));
        assert_eq!(w31a123_strtoul(b"abc", 10), (0, 0, false));
        assert_eq!(w31a123_strtoul(b"5", 1), (0, 0, false));
        assert_eq!(w31a123_strtoul(b"-1", 10), (u32::MAX, 2, false));
        assert_eq!(w31a123_strtoul(b"4294967296", 10), (u32::MAX, 10, true));
    }

    #[test]
    fn strtoull_cases() {
        assert_eq!(w31a123_strtoull(b"0xff", 16), (255, 4, false));
        assert_eq!(
            w31a123_strtoull(b"18446744073709551615", 10),
            (u64::MAX, 20, false)
        );
        assert!(w31a123_strtoull(b"18446744073709551616", 10).2);
    }

    #[test]
    fn strtol_cases() {
        assert_eq!(w31a123_strtol(b"-2147483648", 10), (i32::MIN, 11, false));
        assert_eq!(w31a123_strtol(b"2147483647", 10), (i32::MAX, 10, false));
        assert_eq!(w31a123_strtol(b"2147483648", 10), (i32::MAX, 10, true));
        assert_eq!(w31a123_strtol(b"-2147483649", 10), (i32::MIN, 11, true));
        assert_eq!(w31a123_strtol(b"+7", 10), (7, 2, false));
        assert_eq!(w31a123_strtol(b"x", 10), (0, 0, false));
    }

    #[test]
    fn strtoll_cases() {
        assert_eq!(
            w31a123_strtoll(b"-9223372036854775808", 10),
            (i64::MIN, 20, false)
        );
        assert_eq!(
            w31a123_strtoll(b"9223372036854775808", 10),
            (i64::MAX, 19, true)
        );
        assert_eq!(w31a123_strtoll(b"12", 10), (12, 2, false));
    }

    #[test]
    fn glue() {
        let t = [(2, "no file"), (13, "denied")];
        assert_eq!(w31a123_strerror(&t, 13), "denied");
        assert_eq!(w31a123_strerror(&t, 99), "unknown error");
        assert_eq!(w31a123_unalign(0x1004, 0x1000, 8), Some((0x1000, 0x2b)));
        assert_eq!(w31a123_unalign(0x1024, 0x1000, 8), None);
        let mut c = [0u8, 2, 1];
        assert!(w31a123_release_ref(&mut c, 0));
        assert!(!w31a123_release_ref(&mut c, 1));
        assert!(w31a123_release_ref(&mut c, 2));
    }
}
