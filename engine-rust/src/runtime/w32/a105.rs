#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent105.rs
// ===========================================================================
// Wave 32 agent 105.

/// `0x00b53470`: maps the result of the predicate callee to a mode byte:
/// 2 when the predicate returned nonzero, 4 when it returned zero.
pub fn w32a105_mode_from_flag(flag: u8) -> u8 {
    if flag == 0 { 4 } else { 2 }
}

#[cfg(test)]
mod tests_wave32_agent105 {
    use super::*;

    #[test]
    fn mode() {
        assert_eq!(w32a105_mode_from_flag(0), 4);
        assert_eq!(w32a105_mode_from_flag(1), 2);
    }
}
