#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent78.rs
// ===========================================================================
/// `0x009f0b40`: integer k-th root used for codebook quantisation values
/// (`dim` = exponent, `entries` = value).  Starting from `guess` (the
/// retail code seeds it from a floating-point exp/log estimate, which is
/// supplied by the caller), it walks up or down until
/// `r^dim <= entries < (r+1)^dim`.  Products use wrapping 32-bit math.
pub fn w32a78_quant_values(dim: i32, entries: i32, guess: i32) -> i32 {
    let mut r = guess;
    loop {
        let mut lo: i32 = 1;
        let mut hi: i32 = 1;
        let mut n = dim;
        if n > 0 {
            while n != 0 {
                lo = lo.wrapping_mul(r);
                hi = hi.wrapping_mul(r.wrapping_add(1));
                n -= 1;
            }
        }
        if lo <= entries && entries < hi {
            return r;
        }
        r = r.wrapping_add(if lo <= entries { 1 } else { -1 });
        if r.unsigned_abs() > 1 << 20 {
            return r;
        }
    }
}

#[cfg(test)]
mod tests_wave32_agent78 {
    use super::*;

    #[test]
    fn quant_values_roots() {
        assert_eq!(w32a78_quant_values(2, 16, 3), 4);
        assert_eq!(w32a78_quant_values(3, 26, 5), 2);
        assert_eq!(w32a78_quant_values(3, 27, 1), 3);
        assert_eq!(w32a78_quant_values(1, 9, 9), 9);
    }
}
