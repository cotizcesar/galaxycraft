#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent138.rs
// ===========================================================================
// Wave 32 agent 138.

/// `0x00c9d774`: overflow-check predicate on a flag byte.  Bit 2 forces
/// true; without bit 0 it is false; otherwise compares `value` against
/// `0x8000_0000` (bit 1 set makes the comparison strict).
pub fn w32a138_flag_range_check(flags: u8, value: u32) -> bool {
    if flags & 4 != 0 {
        return true;
    }
    if flags & 1 == 0 {
        return false;
    }
    if flags & 2 == 0 {
        value >= 0x8000_0000
    } else {
        value > 0x8000_0000
    }
}

#[cfg(test)]
mod tests_wave32_agent138 {
    use super::*;

    #[test]
    fn flag_range_check() {
        assert!(w32a138_flag_range_check(4, 0));
        assert!(!w32a138_flag_range_check(0, u32::MAX));
        assert!(w32a138_flag_range_check(1, 0x8000_0000));
        assert!(!w32a138_flag_range_check(1, 0x7fff_ffff));
        assert!(!w32a138_flag_range_check(3, 0x8000_0000));
        assert!(w32a138_flag_range_check(3, 0x8000_0001));
    }
}
