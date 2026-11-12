#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent92.rs
// ===========================================================================
// Wave 32 agent 92 translations.

/// `0x00a99310`: sums a size from flag bits: bits 0..=3 add 8 bytes each,
/// bit 4 adds 4 bytes, bit 5 adds 8 bytes.
pub fn w32a92_flags_size(flags: u32) -> i32 {
    let mut n = ((flags & 1) * 8) as i32;
    if flags & 2 != 0 {
        n += 8;
    }
    if flags & 4 != 0 {
        n += 8;
    }
    if flags & 8 != 0 {
        n += 8;
    }
    if flags & 0x10 != 0 {
        n += 4;
    }
    if flags & 0x20 != 0 {
        n += 8;
    }
    n
}

#[cfg(test)]
mod tests_wave32_agent92 {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(w32a92_flags_size(0), 0);
        assert_eq!(w32a92_flags_size(1), 8);
        assert_eq!(w32a92_flags_size(0x10), 4);
        assert_eq!(w32a92_flags_size(0x3f), 8 * 3 + 8 + 4 + 8);
    }
}
