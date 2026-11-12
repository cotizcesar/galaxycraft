#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent10.rs
// ===========================================================================
// Wave 33 translation batch 10.
//
// Skipped: FUN_00cbb420 (system-info dump through virtual log calls on an
// implicit `this` register and unmodelled callees).

/// `0x00cb7b60`: x87 `frndint` wrapper, rounds to nearest with ties to even
/// (default FPU rounding mode).
pub fn w33a10_round(x: f64) -> f64 {
    x.round_ties_even()
}

/// `0x00cbcc80`: C `strtoll`-style parser.  Returns `(value, end_index,
/// overflow_flag)`.  Reads past the slice yield NUL.  `base == 0` autodetects
/// (`0x` hex, leading `0` octal, else decimal).  Overflow is detected only
/// when the wrapping `acc*base+digit` is signed-smaller than the previous
/// accumulator (retail behaviour) and is sticky; on overflow the result clamps
/// to `i64::MIN` / `i64::MAX` by sign.
pub fn w33a10_strtoll(s: &[u8], base: u32) -> (i64, usize, u8) {
    let at = |i: usize| -> u8 { s.get(i).copied().unwrap_or(0) };
    let mut i = 0usize;
    let first;
    loop {
        let b = at(i);
        i += 1;
        if b == 0x20 || (9..=13).contains(&b) {
            continue;
        }
        first = b;
        break;
    }
    // `p` is the index of the byte after `cur`.
    let mut cur = first;
    let mut p = i;
    if first == b'-' || first == b'+' {
        cur = at(p);
        p += 1;
    }
    let mut base = base;
    let hex_prefix = |p: usize| at(p) == b'x' || at(p) == b'X';
    if base == 0 {
        if cur != b'0' {
            base = 10;
        } else if !hex_prefix(p) {
            base = 8;
        } else {
            base = 16;
            cur = at(p + 1);
            p += 2;
        }
    } else if base == 16 && cur == b'0' && hex_prefix(p) {
        cur = at(p + 1);
        p += 2;
    }
    let mut acc: i64 = 0;
    let mut overflow = false;
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
        let next = acc.wrapping_mul(base as i64).wrapping_add(d as i64);
        if next < acc {
            overflow = true;
        }
        acc = next;
        cur = at(p);
        p += 1;
    }
    let end = p - 1;
    let neg = first == b'-';
    let mut res = acc;
    if overflow {
        let clamp = if neg { i64::MIN } else { i64::MAX };
        res = clamp;
        if acc == clamp {
            overflow = false;
        }
    }
    if neg && !overflow {
        res = res.wrapping_neg();
    }
    (res, end, overflow as u8)
}

#[cfg(test)]
mod tests_wave33_agent10 {
    use super::*;

    #[test]
    fn round_ties_even() {
        assert_eq!(w33a10_round(2.5), 2.0);
        assert_eq!(w33a10_round(3.5), 4.0);
        assert_eq!(w33a10_round(-1.4), -1.0);
    }

    #[test]
    fn parse_basic() {
        assert_eq!(w33a10_strtoll(b"  123abc", 0), (123, 5, 0));
        assert_eq!(w33a10_strtoll(b"-0x1F", 0), (-31, 5, 0));
        assert_eq!(w33a10_strtoll(b"017", 0), (15, 3, 0));
        assert_eq!(w33a10_strtoll(b"ff", 16), (255, 2, 0));
        assert_eq!(w33a10_strtoll(b"0xff", 16).0, 255);
    }

    #[test]
    fn parse_overflow() {
        let (v, _, f) = w33a10_strtoll(b"99999999999999999999", 10);
        assert_eq!((v, f), (i64::MAX, 1));
        let (v, _, f) = w33a10_strtoll(b"-99999999999999999999", 10);
        assert_eq!((v, f), (i64::MIN, 1));
    }
}
