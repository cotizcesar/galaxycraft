#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent104.rs
// ===========================================================================
// Wave 32 agent 104 translations.

/// `0x00b4efe0`: forwards its four arguments to the five-argument worker with
/// a trailing flag of `1`.
pub fn w32a104_forward_with_flag<F>(a: u32, b: u32, c: u32, d: u32, mut worker: F)
where
    F: FnMut(u32, u32, u32, u32, u32),
{
    worker(a, b, c, d, 1);
}

/// `0x00b53430`: state selector.  Returns 3 only when the object's field at
/// index 3 is zero, the check on `param` succeeds and the object's first
/// virtual method reports true; otherwise 4.
pub fn w32a104_select_state<C, V>(field3: u32, check: C, vcall: V) -> u32
where
    C: FnOnce() -> bool,
    V: FnOnce() -> bool,
{
    if field3 == 0 && check() && vcall() {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests_wave32_agent104 {
    use super::*;

    #[test]
    fn forward_adds_flag() {
        let mut got = None;
        w32a104_forward_with_flag(1, 2, 3, 4, |a, b, c, d, e| got = Some((a, b, c, d, e)));
        assert_eq!(got, Some((1, 2, 3, 4, 1)));
    }

    #[test]
    fn select_state_cases() {
        assert_eq!(w32a104_select_state(0, || true, || true), 3);
        assert_eq!(w32a104_select_state(1, || true, || true), 4);
        assert_eq!(w32a104_select_state(0, || false, || true), 4);
        assert_eq!(w32a104_select_state(0, || true, || false), 4);
    }
}
