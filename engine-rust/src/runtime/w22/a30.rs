#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent30.rs
// ===========================================================================
/// `0x006cd4c0`: 64-bit subtraction `a - b` over (lo, hi) u32 pairs, with
/// borrow propagated into the high word.  Returns `[lo, hi]`.
pub fn w22a30_sub64(a: [u32; 2], b: [u32; 2]) -> [u32; 2] {
    let lo = a[0].wrapping_sub(b[0]);
    let hi = a[1].wrapping_sub(b[1]).wrapping_sub(u32::from(a[0] < b[0]));
    [lo, hi]
}

/// `0x006cd550`: returns true when the signed 64-bit value `a` (lo, hi) is
/// strictly greater than the value produced by `other` (the callee computing
/// the comparison operand is supplied by the caller).
pub fn w22a30_greater_than(a: [u32; 2], other: impl FnOnce() -> [u32; 2]) -> bool {
    let b = other();
    let (ah, bh) = (a[1] as i32, b[1] as i32);
    !(ah < bh || (ah <= bh && a[0] <= b[0]))
}

#[cfg(test)]
mod tests_wave22_agent30 {
    use super::*;

    #[test]
    fn sub64_borrow() {
        assert_eq!(w22a30_sub64([0, 1], [1, 0]), [u32::MAX, 0]);
        assert_eq!(w22a30_sub64([5, 2], [3, 1]), [2, 1]);
        assert_eq!(w22a30_sub64([0, 0], [1, 0]), [u32::MAX, u32::MAX]);
    }

    #[test]
    fn greater_than_signed() {
        assert!(w22a30_greater_than([2, 0], || [1, 0]));
        assert!(!w22a30_greater_than([1, 0], || [1, 0]));
        assert!(w22a30_greater_than([0, 1], || [u32::MAX, 0]));
        assert!(!w22a30_greater_than([0, u32::MAX], || [0, 0]));
    }
}
