#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent52.rs
// ===========================================================================
// Wave 32 agent 52 translations.

/// `0x0083b650`: invokes a stored callable (std::function-like slot at
/// object offset 0x28).  Bit 0 of the slot word tags an out-of-line
/// pointer (mask `0xfffffffe`); when the resolved target is null nothing
/// happens, otherwise the callable's vtable entry at +4 is invoked with
/// both arguments.  Returns whether the callable was invoked.
pub fn w32a52_invoke_slot<F: FnOnce(u32, u32)>(slot: u32, a: u32, b: u32, call: F) -> bool {
    let target = if slot & 1 != 0 {
        slot & 0xffff_fffe
    } else {
        slot
    };
    if target == 0 {
        return false;
    }
    call(a, b);
    true
}

#[cfg(test)]
mod tests_wave32_agent52 {
    use super::*;

    #[test]
    fn invoke_slot() {
        let mut got = None;
        assert!(w32a52_invoke_slot(0x1001, 1, 2, |a, b| got = Some((a, b))));
        assert_eq!(got, Some((1, 2)));
        assert!(!w32a52_invoke_slot(1, 1, 2, |_, _| panic!()));
        assert!(!w32a52_invoke_slot(0, 1, 2, |_, _| panic!()));
    }
}
