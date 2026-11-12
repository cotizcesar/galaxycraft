#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent34.rs
// ===========================================================================
// Wave 33 agent 34 translations.

/// `0x00c4f2b0`: maps a small kind code to a flag mask.  Kind 5 depends on
/// `format_flags` (the result of the format-info lookup `FUN_00c12eb0`):
/// bit `0x20` selects `0x10`, otherwise `4`.  Unknown kinds (0, 6, > 10)
/// return 0.
pub fn w33a34_kind_flag_mask(kind: u8, format_flags: u32) -> u32 {
    match kind {
        1 => 0x80,
        2 => 0x40,
        3 => 0xc0,
        4 => 8,
        5 => {
            if format_flags & 0x20 != 0 {
                0x10
            } else {
                4
            }
        }
        7 => 0x400,
        8 => 0x800,
        9 => 0x1000,
        10 => 0x2000,
        _ => 0,
    }
}

#[cfg(test)]
mod tests_wave33_agent34 {
    use super::*;

    #[test]
    fn masks() {
        assert_eq!(w33a34_kind_flag_mask(3, 0), 0xc0);
        assert_eq!(w33a34_kind_flag_mask(5, 0x20), 0x10);
        assert_eq!(w33a34_kind_flag_mask(5, 0), 4);
        assert_eq!(w33a34_kind_flag_mask(6, 0), 0);
        assert_eq!(w33a34_kind_flag_mask(10, 0), 0x2000);
    }
}
