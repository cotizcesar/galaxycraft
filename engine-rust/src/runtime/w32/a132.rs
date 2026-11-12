#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent132.rs
// ===========================================================================
// Wave 32 agent 132: digit formatters of a runtime number-to-text buffer.

/// Result of a backwards digit fill.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W32a132Fill {
    /// Index of the first written digit.
    pub start: usize,
    /// Number of digits written (stored at +0x38).
    pub len: usize,
    /// Remaining minimum-digit counter (+0x28 after the loop).
    pub min_digits: i32,
}

fn w32a132_digit(rem: u32, upper: u8) -> u8 {
    let c = (rem as u8).wrapping_add(b'0');
    if c > b'9' {
        c.wrapping_add((upper ^ 1).wrapping_mul(32)).wrapping_add(7)
    } else {
        c
    }
}

/// `0x00c98f4c`: writes the digits of `value` in `base` backwards into a
/// byte buffer, ending at `end` (the callee-computed last slot).  Loops while
/// the minimum-digit counter is positive or the value is non-zero; digits
/// above 9 become `A..` when `upper == 1`, else `a..`.
pub fn w32a132_fill_digits_narrow(
    buf: &mut [u8],
    end: usize,
    mut value: u64,
    base: u32,
    upper: u8,
    mut min_digits: i32,
) -> W32a132Fill {
    let mut pos = end as isize;
    while min_digits > 0 || value != 0 {
        min_digits -= 1;
        let rem = (value % base as u64) as u32;
        value /= base as u64;
        buf[pos as usize] = w32a132_digit(rem, upper);
        pos -= 1;
    }
    W32a132Fill {
        start: (pos + 1) as usize,
        len: (end as isize - pos) as usize,
        min_digits,
    }
}

/// `0x00c98fdc`: same as the narrow variant but writes 16-bit characters
/// (stride 2 bytes, the length is the byte distance shifted right by one).
pub fn w32a132_fill_digits_wide(
    buf: &mut [u16],
    end: usize,
    mut value: u64,
    base: u32,
    upper: u8,
    mut min_digits: i32,
) -> W32a132Fill {
    let mut pos = end as isize;
    while min_digits > 0 || value != 0 {
        min_digits -= 1;
        let rem = (value % base as u64) as u32;
        value /= base as u64;
        buf[pos as usize] = w32a132_digit(rem, upper) as u16;
        pos -= 1;
    }
    W32a132Fill {
        start: (pos + 1) as usize,
        len: (end as isize - pos) as usize,
        min_digits,
    }
}

/// `0x00c99223`: releases the buffer handle at +0x404 through `free` and
/// clears the field.
pub fn w32a132_release_handle(handle: &mut u32, free: impl FnOnce(u32)) {
    free(*handle);
    *handle = 0;
}

#[cfg(test)]
mod tests_wave32_agent132 {
    use super::*;

    #[test]
    fn narrow_hex() {
        let mut b = [0u8; 8];
        let r = w32a132_fill_digits_narrow(&mut b, 7, 0xBEEF, 16, 1, 0);
        assert_eq!(&b[r.start..8], b"BEEF");
        assert_eq!(r.len, 4);
        let r = w32a132_fill_digits_narrow(&mut b, 7, 0xBEEF, 16, 0, 0);
        assert_eq!(&b[r.start..8], b"beef");
    }

    #[test]
    fn narrow_padding_and_zero() {
        let mut b = [0u8; 4];
        let r = w32a132_fill_digits_narrow(&mut b, 3, 0, 10, 0, 2);
        assert_eq!(&b[r.start..4], b"00");
        assert_eq!(r.min_digits, 0);
    }

    #[test]
    fn wide_dec() {
        let mut b = [0u16; 6];
        let r = w32a132_fill_digits_wide(&mut b, 5, 1234, 10, 0, 0);
        assert_eq!(r.len, 4);
        assert_eq!(b[r.start], b'1' as u16);
    }

    #[test]
    fn release() {
        let mut h = 5;
        let mut got = 0;
        w32a132_release_handle(&mut h, |x| got = x);
        assert_eq!((h, got), (0, 5));
    }
}
