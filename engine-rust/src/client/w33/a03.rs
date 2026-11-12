#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent03.rs
// ===========================================================================
// Wave 33 translation batch 03.
//
// Skipped: FUN_0090c780 (ends in a non-returning call), FUN_00919f40
// (virtual dispatch through an unmodelled callee).

/// `0x009185c0`: clears the flag byte at offset `0x1f6` of the object and
/// returns 0.
pub fn w33a03_clear_flag_1f6(obj: &mut [u8]) -> u8 {
    obj[0x1f6] = 0;
    0
}

#[cfg(test)]
mod tests_wave33_agent03 {
    use super::*;

    #[test]
    fn clears_flag() {
        let mut o = vec![0xffu8; 0x200];
        assert_eq!(w33a03_clear_flag_1f6(&mut o), 0);
        assert_eq!(o[0x1f6], 0);
        assert_eq!(o[0x1f5], 0xff);
    }
}
