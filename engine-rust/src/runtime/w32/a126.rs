#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent126.rs
// ===========================================================================
// Wave 32 agent 126: unsigned 64-bit division helper.

/// `0x00c878b0`: CRT unsigned 64-bit divide (`__aulldiv`).  Arguments are
/// the dividend low/high words then the divisor low/high words; returns the
/// 64-bit quotient.  The 32-bit-divisor path and the normalise-and-correct
/// path of the retail routine both yield the exact quotient.
pub fn w32a126_aulldiv(n_lo: u32, n_hi: u32, d_lo: u32, d_hi: u32) -> u64 {
    let n = ((n_hi as u64) << 32) | n_lo as u64;
    let d = ((d_hi as u64) << 32) | d_lo as u64;
    n / d
}

#[cfg(test)]
mod tests_wave32_agent126 {
    use super::*;

    #[test]
    fn small_divisor() {
        assert_eq!(w32a126_aulldiv(100, 0, 7, 0), 14);
        assert_eq!(w32a126_aulldiv(0, 5, 2, 0), 5u64 << 31);
    }

    #[test]
    fn wide_divisor() {
        assert_eq!(w32a126_aulldiv(0, 8, 0, 2), 4);
        assert_eq!(
            w32a126_aulldiv(u32::MAX, u32::MAX, 1, 1),
            0xFFFF_FFFF_FFFF_FFFF / 0x1_0000_0001
        );
    }
}
