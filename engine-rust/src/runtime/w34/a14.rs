#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent14.rs
// ===========================================================================
// Wave 34 agent 14: SSE2 masked sum-of-squared-differences kernel.

/// `0x004bd548`: sum of squared differences over 16 blocks of 16 `i16`
/// samples (256 values).  In each block the first eight lanes are masked:
/// lane 0 is zeroed when `lo != 0`, lane 1 when `hi != 0` (the mask is
/// `value == 0`), lane 7 is zeroed when `lane7_nonzero` (an undefined
/// input in the pseudocode, so the caller supplies it).  The pmaddwd pair
/// sums and accumulators wrap as `i32`.
pub fn w34a14_masked_ssd(a: &[i16], b: &[i16], lo: u16, hi: u16, lane7_nonzero: bool) -> i32 {
    let mut acc: i32 = 0;
    for blk in 0..16usize {
        for l in 0..16usize {
            let i = blk * 16 + l;
            let mut d = a[i].wrapping_sub(b[i]);
            if l < 8 {
                let keep = match l {
                    0 => lo == 0,
                    1 => hi == 0,
                    7 => !lane7_nonzero,
                    _ => true,
                };
                if !keep {
                    d = 0;
                }
            }
            acc = acc.wrapping_add((d as i32).wrapping_mul(d as i32));
        }
    }
    acc
}

#[cfg(test)]
mod tests_wave34_agent14 {
    use super::*;

    #[test]
    fn masked_ssd() {
        let a = [3i16; 256];
        let b = [1i16; 256];
        assert_eq!(w34a14_masked_ssd(&a, &b, 0, 0, false), 256 * 4);
        assert_eq!(w34a14_masked_ssd(&a, &b, 1, 0, false), 240 * 4);
        assert_eq!(w34a14_masked_ssd(&a, &b, 1, 1, true), 16 * 13 * 4);
    }
}
