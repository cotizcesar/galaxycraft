#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent61.rs
// ===========================================================================
/// `0x00cb10e1`: writes a two-digit decimal field at `out[0..2]` and returns
/// the number of bytes written (2, i.e. the advanced pointer offset).  When
/// `value < 10` and `pad_zero` is false the tens digit is a space, otherwise
/// it is `value / 10 + '0'`; the units digit is `value % 10 + '0'`.
pub fn w33a61_format_two_digits(out: &mut [u8; 2], value: i32, pad_zero: bool) -> usize {
    out[0] = if value < 10 && !pad_zero {
        b' '
    } else {
        ((value / 10) as u8).wrapping_add(b'0')
    };
    out[1] = ((value % 10) as u8).wrapping_add(b'0');
    2
}

#[cfg(test)]
mod tests_wave33_agent61 {
    use super::*;

    #[test]
    fn two_digits() {
        let mut b = [0u8; 2];
        assert_eq!(w33a61_format_two_digits(&mut b, 7, false), 2);
        assert_eq!(&b, b" 7");
        w33a61_format_two_digits(&mut b, 7, true);
        assert_eq!(&b, b"07");
        w33a61_format_two_digits(&mut b, 42, false);
        assert_eq!(&b, b"42");
        w33a61_format_two_digits(&mut b, 10, false);
        assert_eq!(&b, b"10");
    }
}
