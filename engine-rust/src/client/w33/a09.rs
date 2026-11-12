#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent09.rs
// ===========================================================================
// Wave 33 translation batch 09.

/// `0x00b8eda0`: reverses the bytes of `buf` in place (std::reverse over
/// a byte range, swapping from both ends until the cursors meet).
pub fn w33a09_reverse_bytes(buf: &mut [u8]) {
    buf.reverse();
}

/// `0x00cae75b`: classifies a double from its low (`lo`) and high (`hi`)
/// words: 1 = +inf, 2 = -inf, 3 = NaN, 4 = signalling NaN, 0 = finite.
pub fn w33a09_classify_double(lo: u32, hi: u32) -> u32 {
    if hi == 0x7ff0_0000 {
        if lo == 0 {
            return 1;
        }
    } else if hi == 0xfff0_0000 && lo == 0 {
        return 2;
    }
    let top = (hi >> 16) & 0x7ff8;
    if top == 0x7ff8 {
        3
    } else if top != 0x7ff0 || ((hi & 0x7_ffff) == 0 && lo == 0) {
        0
    } else {
        4
    }
}

#[cfg(test)]
mod tests_wave33_agent09 {
    use super::*;

    #[test]
    fn reverse() {
        let mut v = [1u8, 2, 3, 4, 5];
        w33a09_reverse_bytes(&mut v);
        assert_eq!(v, [5, 4, 3, 2, 1]);
    }

    #[test]
    fn classify() {
        assert_eq!(w33a09_classify_double(0, 0x7ff0_0000), 1);
        assert_eq!(w33a09_classify_double(0, 0xfff0_0000), 2);
        assert_eq!(w33a09_classify_double(0, 0x7ff8_0000), 3);
        assert_eq!(w33a09_classify_double(1, 0x7ff0_0000), 4);
        assert_eq!(w33a09_classify_double(0, 0x3ff0_0000), 0);
    }
}
