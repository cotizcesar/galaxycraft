#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent24.rs
// ===========================================================================
// Wave 32 translation batch 24.
//
// Skipped: FUN_00985dd0 (unrecovered jump table, opaque `this` layout).

/// `0x00984f80`: runs `probe(param_1)`; when it returns 0 (success) calls
/// `action(param_2)`.  Returns the probe status.
pub fn w32a24_probe_then_action_a(
    param_1: u32,
    param_2: u32,
    mut probe: impl FnMut(u32) -> i32,
    mut action: impl FnMut(u32),
) -> i32 {
    let r = probe(param_1);
    if r == 0 {
        action(param_2);
    }
    r
}

/// `0x00984fb0`: same shape as the sibling helper, with a different action.
pub fn w32a24_probe_then_action_b(
    param_1: u32,
    param_2: u32,
    probe: impl FnMut(u32) -> i32,
    action: impl FnMut(u32),
) -> i32 {
    w32a24_probe_then_action_a(param_1, param_2, probe, action)
}

#[cfg(test)]
mod tests_wave32_agent24 {
    use super::*;

    #[test]
    fn action_runs_only_on_zero() {
        let mut got = vec![];
        assert_eq!(w32a24_probe_then_action_a(1, 7, |_| 0, |x| got.push(x)), 0);
        assert_eq!(w32a24_probe_then_action_b(1, 8, |_| 5, |x| got.push(x)), 5);
        assert_eq!(got, vec![7]);
    }
}
