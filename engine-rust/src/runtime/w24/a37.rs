#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent37.rs
// ===========================================================================
// Wave 24 agent 37: shared-pointer manager glue and a state predicate.

/// `0x007ad390`: returns true when the state word at `+0x54` of the object
/// referenced by the first field of `this` is non-zero.
pub fn w24a37_state_active(state_word: i32) -> bool {
    state_word != 0
}

/// Scalar deleting destructor shared by the shared-pointer manager
/// variants `0x007accd0`, `0x007acd00`, `0x007acd30`, `0x007acd60` and
/// `0x007acd90`: resets the vtable to the base manager and reports whether
/// the object memory must be freed (bit 0 of the flag byte).
pub fn w24a37_manager_dtor_should_free(flags: u8) -> bool {
    flags & 1 != 0
}

#[cfg(test)]
mod tests_wave24_agent37 {
    use super::*;

    #[test]
    fn state_active() {
        assert!(!w24a37_state_active(0));
        assert!(w24a37_state_active(2));
        assert!(w24a37_state_active(-1));
    }

    #[test]
    fn dtor_free_flag() {
        assert!(w24a37_manager_dtor_should_free(1));
        assert!(w24a37_manager_dtor_should_free(0xff));
        assert!(!w24a37_manager_dtor_should_free(0));
        assert!(!w24a37_manager_dtor_should_free(2));
    }
}
