#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent46.rs
// ===========================================================================
// Wave 34 agent 46 translations.

/// `0x00c12710`: returns whether `value` is compatible with the category
/// `kind` (0/1 accept 0,1; 2 accepts 2,3; 3/6/7 accept 2,3,6,7; 4/5 accept
/// 4,5; 8 accepts 8; anything else rejects).
pub fn w34a46_kind_accepts(kind: u8, value: u8) -> bool {
    match kind {
        0 | 1 => matches!(value, 0 | 1),
        2 => matches!(value, 2 | 3),
        3 | 6 | 7 => matches!(value, 2 | 3 | 6 | 7),
        4 | 5 => matches!(value, 4 | 5),
        8 => value == 8,
        _ => false,
    }
}

/// `0x00c12950`: true for the odd kinds 1, 3, 5 and 7.
pub fn w34a46_kind_is_odd_flag(kind: u8) -> bool {
    matches!(kind, 1 | 3 | 5 | 7)
}

#[cfg(test)]
mod tests_wave34_agent46 {
    use super::*;

    #[test]
    fn accepts() {
        assert!(w34a46_kind_accepts(0, 1));
        assert!(!w34a46_kind_accepts(0, 2));
        assert!(w34a46_kind_accepts(7, 6));
        assert!(w34a46_kind_accepts(8, 8));
        assert!(!w34a46_kind_accepts(9, 0));
    }

    #[test]
    fn odd() {
        assert!(w34a46_kind_is_odd_flag(5));
        assert!(!w34a46_kind_is_odd_flag(4));
    }
}
