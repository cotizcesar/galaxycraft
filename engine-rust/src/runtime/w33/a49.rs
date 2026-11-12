#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent49.rs
// ===========================================================================
// Wave 33 agent 49: small size-lookup helpers.

/// `0x00ca032e`: element size for a kind code: 0 -> 4, 3 or 8 -> 8, else 0.
pub fn w33a49_kind_size_a(kind: i32) -> u32 {
    match kind {
        0 => 4,
        3 | 8 => 8,
        _ => 0,
    }
}

/// `0x00ca0351`: element size for a type code (0..=10); unknown -> 0.
pub fn w33a49_kind_size_b(kind: u32) -> u32 {
    match kind {
        0 | 3 | 6 | 7 | 9 => 4,
        1 => 1,
        2 => 2,
        4 | 5 | 10 => 8,
        _ => 0,
    }
}

#[cfg(test)]
mod tests_wave33_agent49 {
    use super::*;

    #[test]
    fn size_a() {
        assert_eq!(w33a49_kind_size_a(0), 4);
        assert_eq!(w33a49_kind_size_a(3), 8);
        assert_eq!(w33a49_kind_size_a(8), 8);
        assert_eq!(w33a49_kind_size_a(5), 0);
    }

    #[test]
    fn size_b() {
        assert_eq!(w33a49_kind_size_b(1), 1);
        assert_eq!(w33a49_kind_size_b(2), 2);
        assert_eq!(w33a49_kind_size_b(9), 4);
        assert_eq!(w33a49_kind_size_b(10), 8);
        assert_eq!(w33a49_kind_size_b(11), 0);
    }
}
