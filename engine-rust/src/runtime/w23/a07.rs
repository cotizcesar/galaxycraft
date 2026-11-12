#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent07.rs
// ===========================================================================
// Wave 23 agent 07: small state predicates and a quantity helper.

/// `0x006e7c80`: true when the state byte at the global equals 4.
pub fn w23a07_state_is_4(state: u8) -> bool {
    state == 4
}

/// `0x006e7de0`: true when the state byte at the global equals 5.
pub fn w23a07_state_is_5(state: u8) -> bool {
    state == 5
}

/// `0x006e7e00`: true when `kind` is `0x80` (signed -0x80) and `flag` is non-zero.
pub fn w23a07_kind_80_flagged(kind: u8, flag: u8) -> bool {
    kind == 0x80 && flag != 0
}

/// `0x006e7e50`: true when `kind` is in `0x11..=0x16`.
pub fn w23a07_kind_in_11_16(kind: u8) -> bool {
    kind.wrapping_sub(0x11) < 6
}

/// `0x006e9130` (amount part only): computes the adjusted quantity from the
/// base value (`+0xc`), operand (`+0x14`) and mode byte (`+0x1b`): mode 7
/// replaces with the operand, 8 adds (wrapping), 9 subtracts saturating at 0,
/// any other mode keeps the base.
pub fn w23a07_apply_mode(mode: u8, base: u32, operand: u32) -> u32 {
    match mode {
        7 => operand,
        8 => base.wrapping_add(operand),
        9 => base.saturating_sub(operand),
        _ => base,
    }
}

#[cfg(test)]
mod tests_wave23_agent07 {
    use super::*;

    #[test]
    fn predicates() {
        assert!(w23a07_state_is_4(4) && !w23a07_state_is_4(5));
        assert!(w23a07_state_is_5(5) && !w23a07_state_is_5(4));
        assert!(w23a07_kind_80_flagged(0x80, 1) && !w23a07_kind_80_flagged(0x80, 0));
        assert!(!w23a07_kind_80_flagged(0x7f, 1));
    }

    #[test]
    fn range() {
        assert!(!w23a07_kind_in_11_16(0x10));
        assert!(w23a07_kind_in_11_16(0x11) && w23a07_kind_in_11_16(0x16));
        assert!(!w23a07_kind_in_11_16(0x17));
    }

    #[test]
    fn modes() {
        assert_eq!(w23a07_apply_mode(7, 10, 3), 3);
        assert_eq!(w23a07_apply_mode(8, 10, 3), 13);
        assert_eq!(w23a07_apply_mode(9, 10, 3), 7);
        assert_eq!(w23a07_apply_mode(9, 2, 3), 0);
        assert_eq!(w23a07_apply_mode(1, 10, 3), 10);
    }
}
