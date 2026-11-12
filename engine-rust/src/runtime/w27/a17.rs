#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent17.rs
// ===========================================================================
// Wave 27 agent 17: small buffer/parse helpers.

/// `0x009c89c0`: decimal string to wrapping i64. A leading `-` negates; any
/// other non-digit byte yields 0.
pub fn w27a17_parse_i64(s: &[u8]) -> i64 {
    let mut v: u64 = 0;
    let mut neg = false;
    let mut first = true;
    for &c in s {
        if c == 0 {
            break;
        }
        if c == b'-' && first {
            neg = true;
        } else {
            if !c.is_ascii_digit() {
                return 0;
            }
            v = v.wrapping_mul(10).wrapping_add((c - b'0') as u64);
        }
        first = false;
    }
    if neg {
        (v as i64).wrapping_neg()
    } else {
        v as i64
    }
}

/// `0x009bfba0`: when the buffer exists and the count is positive, clears
/// `0x14020` bytes; always returns 0.
pub fn w27a17_clear_buffer(buf: Option<&mut [u8]>, count: i32) -> u32 {
    if let Some(b) = buf
        && count > 0
    {
        let n = 0x14020.min(b.len());
        b[..n].fill(0);
    }
    0
}

/// `0x009c48f0`: sets `+0x2c4 = 0`, `+0x2c8 = 1` and clears `0x100` bytes
/// at `+1`. `obj` must hold at least `0x2cc` bytes.
pub fn w27a17_reset_state(obj: &mut [u8]) {
    obj[0x2c4..0x2c8].copy_from_slice(&0u32.to_le_bytes());
    obj[0x2c8..0x2cc].copy_from_slice(&1u32.to_le_bytes());
    obj[1..0x101].fill(0);
}

/// `0x009cc1c0`: zeroes the dword array (`count` entries) when present and
/// the `0x38` bytes at `+0x78` of `obj`.
pub fn w27a17_clear_table(array: Option<&mut [u32]>, count: usize, obj: &mut [u8]) {
    if let Some(a) = array {
        let n = count.min(a.len());
        a[..n].fill(0);
    }
    obj[0x78..0x78 + 0x38].fill(0);
}

/// `0x009c3c30`: returns the first call's result when nonzero, else the
/// second call on `handle` when `handle != 0`, else 0.
pub fn w27a17_first_nonzero(
    first: impl FnOnce() -> i32,
    handle: i32,
    second: impl FnOnce(i32) -> i32,
) -> i32 {
    let r = first();
    if r != 0 {
        return r;
    }
    if handle != 0 { second(handle) } else { 0 }
}

#[cfg(test)]
mod tests_wave27_agent17 {
    use super::*;

    #[test]
    fn parse() {
        assert_eq!(w27a17_parse_i64(b"123"), 123);
        assert_eq!(w27a17_parse_i64(b"-45"), -45);
        assert_eq!(w27a17_parse_i64(b"4-5"), 0);
        assert_eq!(w27a17_parse_i64(b"x"), 0);
        assert_eq!(w27a17_parse_i64(b""), 0);
    }

    #[test]
    fn clears() {
        let mut b = vec![1u8; 0x14100];
        assert_eq!(w27a17_clear_buffer(Some(&mut b), 0), 0);
        assert_eq!(b[0], 1);
        w27a17_clear_buffer(Some(&mut b), 1);
        assert_eq!(b[0x14020 - 1], 0);
        assert_eq!(b[0x14020], 1);
        let mut o = vec![9u8; 0x300];
        w27a17_reset_state(&mut o);
        assert_eq!((o[0], o[1], o[0x100], o[0x101]), (9, 0, 0, 9));
        assert_eq!((o[0x2c4], o[0x2c8]), (0, 1));
        let mut a = [5u32; 4];
        let mut o = vec![7u8; 0xc0];
        w27a17_clear_table(Some(&mut a), 3, &mut o);
        assert_eq!(a, [0, 0, 0, 5]);
        assert_eq!((o[0x77], o[0x78], o[0xaf], o[0xb0]), (7, 0, 0, 7));
    }

    #[test]
    fn first_nonzero() {
        assert_eq!(w27a17_first_nonzero(|| 3, 1, |_| 9), 3);
        assert_eq!(w27a17_first_nonzero(|| 0, 1, |h| h + 8), 9);
        assert_eq!(w27a17_first_nonzero(|| 0, 0, |_| 9), 0);
    }
}
