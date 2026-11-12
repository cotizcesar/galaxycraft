#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent50.rs
// ===========================================================================
// Wave 32 translation batch 50 (staging).
//
// Skipped: FUN_007e8490 (opaque callee), FUN_00802c00 (non-returning throw).

/// `0x007f50e0`: vector<bool> iterator arithmetic.  Given the base word
/// index `base` (in 32-bit words), bit offset `bit` (0..32) and a signed
/// displacement `delta`, returns the new (word index, bit offset).
pub fn w32a50_vecbool_iter_add(base: u32, bit: u32, delta: i32) -> (u32, u32) {
    let new_bit = bit.wrapping_add(delta as u32) & 0x1f;
    if delta < 0 && bit < delta.wrapping_neg() as u32 {
        let t = !(bit.wrapping_add(delta as u32)) >> 5;
        (
            base.wrapping_sub(t.wrapping_mul(4).wrapping_add(4)),
            new_bit,
        )
    } else {
        (
            base.wrapping_add((bit.wrapping_add(delta as u32) >> 5).wrapping_mul(4)),
            new_bit,
        )
    }
}

#[cfg(test)]
mod tests_wave32_agent50 {
    use super::*;

    #[test]
    fn forward_and_backward() {
        assert_eq!(w32a50_vecbool_iter_add(100, 5, 30), (104, 3));
        assert_eq!(w32a50_vecbool_iter_add(100, 5, 2), (100, 7));
        assert_eq!(w32a50_vecbool_iter_add(100, 5, -3), (100, 2));
        assert_eq!(w32a50_vecbool_iter_add(100, 5, -6), (96, 31));
        assert_eq!(w32a50_vecbool_iter_add(100, 0, -32), (96, 0));
    }
}
