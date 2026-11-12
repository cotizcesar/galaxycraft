#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent34.rs
// ===========================================================================
// Wave 32 agent 34: decimal u64 formatting helpers.

/// `0x00b8f140`: writes the decimal digits of `value` into `out` (no NUL),
/// returning the digit count.  Digits are emitted least-significant first and
/// then reversed in place; zero yields a single `'0'`.
pub fn w32a34_u64_to_dec(value: u64, out: &mut [u8]) -> usize {
    if value == 0 {
        out[0] = b'0';
        return 1;
    }
    let mut v = value;
    let mut n = 0;
    while v != 0 {
        out[n] = (v % 10) as u8 + b'0';
        n += 1;
        v /= 10;
    }
    out[..n].reverse();
    n
}

/// `0x00b8f1d0`: bounded variant of the decimal formatter.  Returns the
/// digit count, or 0 when `out` is empty or too small to hold every digit
/// (the buffer may then contain partial low digits, as in retail).
pub fn w32a34_u64_to_dec_bounded(value: u64, out: &mut [u8]) -> usize {
    if out.is_empty() {
        return 0;
    }
    if value == 0 {
        out[0] = b'0';
        return 1;
    }
    let mut v = value;
    let mut n = 0;
    while v != 0 && n < out.len() {
        out[n] = (v % 10) as u8 + b'0';
        n += 1;
        v /= 10;
    }
    if v != 0 {
        return 0;
    }
    out[..n].reverse();
    n
}

#[cfg(test)]
mod tests_wave32_agent34 {
    use super::*;

    #[test]
    fn unbounded() {
        let mut b = [0u8; 20];
        assert_eq!(w32a34_u64_to_dec(0, &mut b), 1);
        assert_eq!(b[0], b'0');
        let n = w32a34_u64_to_dec(u64::MAX, &mut b);
        assert_eq!(&b[..n], b"18446744073709551615");
        let n = w32a34_u64_to_dec(1200, &mut b);
        assert_eq!(&b[..n], b"1200");
    }

    #[test]
    fn bounded() {
        let mut b = [0u8; 4];
        assert_eq!(w32a34_u64_to_dec_bounded(1234, &mut b), 4);
        assert_eq!(&b, b"1234");
        assert_eq!(w32a34_u64_to_dec_bounded(12345, &mut b), 0);
        assert_eq!(w32a34_u64_to_dec_bounded(0, &mut b), 1);
        assert_eq!(w32a34_u64_to_dec_bounded(5, &mut []), 0);
    }
}
