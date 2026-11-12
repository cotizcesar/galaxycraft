#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent17.rs
// ===========================================================================
// Wave 22 agent 17 translations.

/// `0x00684590`: message filter.  A record whose 16-bit kind (offset 0x10)
/// is 6 is swallowed and returns 1.  Kind 0xe with a zero first word first
/// runs `on_kind_e`; every other record (and kind 0xe) is forwarded to
/// `default_handler`, whose result is returned.
pub fn w22a17_filter_message(
    first_word: i32,
    kind: u16,
    mut on_kind_e: impl FnMut(),
    default_handler: impl FnOnce() -> u32,
) -> u32 {
    if kind == 6 {
        return 1;
    }
    if kind == 0xe && first_word == 0 {
        on_kind_e();
    }
    default_handler()
}

#[cfg(test)]
mod tests_wave22_agent17 {
    use super::*;

    #[test]
    fn filter_message_paths() {
        let mut calls = 0;
        assert_eq!(w22a17_filter_message(0, 6, || calls += 1, || 9), 1);
        assert_eq!(calls, 0);
        assert_eq!(w22a17_filter_message(0, 0xe, || calls += 1, || 9), 9);
        assert_eq!(calls, 1);
        assert_eq!(w22a17_filter_message(1, 0xe, || calls += 1, || 7), 7);
        assert_eq!(w22a17_filter_message(0, 5, || calls += 1, || 3), 3);
        assert_eq!(calls, 1);
    }
}
