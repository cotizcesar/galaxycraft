#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent44.rs
// ===========================================================================
// Wave 33 agent 44 translations.

/// `0x00c98c85`: returns the default size hint of a buffer object: `0x200`
/// when the field at `+0x404` is zero, otherwise the field at `+0x400`
/// shifted right by one.
pub fn w33a44_half_size_or_default(field_404: u32, field_400: u32) -> u32 {
    if field_404 == 0 {
        0x200
    } else {
        field_400 >> 1
    }
}

#[cfg(test)]
mod tests_wave33_agent44 {
    use super::*;

    #[test]
    fn default_when_zero() {
        assert_eq!(w33a44_half_size_or_default(0, 1000), 0x200);
    }

    #[test]
    fn halves_when_set() {
        assert_eq!(w33a44_half_size_or_default(1, 1000), 500);
        assert_eq!(w33a44_half_size_or_default(7, 0xffff_ffff), 0x7fff_ffff);
    }
}
