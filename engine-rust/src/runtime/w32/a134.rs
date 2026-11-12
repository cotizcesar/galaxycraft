#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent134.rs
// ===========================================================================
// Wave 32 agent 134 translations.

/// `0x00c9a912`: maps a type-code (0..=0xb) to a byte size: 1 -> 1, 2 -> 2,
/// {0,3,6,7,9,10} -> 4, {4,5,0xb} -> 8, anything else -> 0.
pub fn w32a134_type_code_size(code: u32) -> u32 {
    match code {
        0 | 3 | 6 | 7 | 9 | 10 => 4,
        1 => 1,
        2 => 2,
        4 | 5 | 0xb => 8,
        _ => 0,
    }
}

#[cfg(test)]
mod tests_wave32_agent134 {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(w32a134_type_code_size(0), 4);
        assert_eq!(w32a134_type_code_size(1), 1);
        assert_eq!(w32a134_type_code_size(2), 2);
        assert_eq!(w32a134_type_code_size(0xb), 8);
        assert_eq!(w32a134_type_code_size(12), 0);
    }
}
