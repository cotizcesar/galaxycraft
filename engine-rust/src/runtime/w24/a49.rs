#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent49.rs
// ===========================================================================
// Wave 24 agent 49 (queue, runtime.rs).  Most rows are SEH/throw glue and are skipped.

/// `0x007d6e40`: decision part of the state-change notifier.  The object's
/// field `+0x28` is forced to 3 (`new_state`); `old_state` is field `+0x24`.
/// Returns `None` when no callback is issued (old state is 1, or the handler
/// slot is empty); otherwise `Some(flag)` where `flag` is 1 when both states
/// are 0 or 2 (never true here since new state is 3, kept for fidelity).
pub fn w24a49_state_notify_flag(old_state: i32, handler_present: bool) -> Option<bool> {
    let new_state: i32 = 3;
    if old_state == 1 || new_state == 1 || !handler_present {
        return None;
    }
    Some((old_state == 0 || old_state == 2) && (new_state == 0 || new_state == 2))
}

#[cfg(test)]
mod tests_wave24_agent49 {
    use super::*;

    #[test]
    fn notify_flag() {
        assert_eq!(w24a49_state_notify_flag(1, true), None);
        assert_eq!(w24a49_state_notify_flag(0, false), None);
        assert_eq!(w24a49_state_notify_flag(0, true), Some(false));
        assert_eq!(w24a49_state_notify_flag(2, true), Some(false));
    }
}
