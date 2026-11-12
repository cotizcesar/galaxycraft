#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent48.rs
// ===========================================================================
// Wave 32 agent 48 translations.

/// `0x0077e060`: clamps `value` to 100, runs the pre-step callee, then passes
/// the clamped value (a zero stays zero) as a one-element slice to the sink.
pub fn w32a48_set_clamped(value: u32, pre: &mut dyn FnMut(), sink: &mut dyn FnMut(&[u32])) {
    let v = value.min(100);
    pre();
    sink(&[v]);
}

/// `0x007b3450`: true when the readiness probe passes and the global state
/// equals 2.
pub fn w32a48_state_is_two(ready: bool, state: u32) -> bool {
    ready && state == 2
}

#[cfg(test)]
mod tests_wave32_agent48 {
    use super::*;

    #[test]
    fn clamp() {
        let mut got = vec![];
        let mut n = 0;
        w32a48_set_clamped(250, &mut || n += 1, &mut |s| got.extend_from_slice(s));
        w32a48_set_clamped(7, &mut || n += 1, &mut |s| got.extend_from_slice(s));
        assert_eq!(got, vec![100, 7]);
        assert_eq!(n, 2);
    }

    #[test]
    fn state() {
        assert!(w32a48_state_is_two(true, 2));
        assert!(!w32a48_state_is_two(true, 1));
        assert!(!w32a48_state_is_two(false, 2));
    }
}
