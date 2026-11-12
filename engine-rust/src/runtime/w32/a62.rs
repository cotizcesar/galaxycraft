#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent62.rs
// ===========================================================================
// Wave 32 translation batch 62: tiny forwarding wrappers.

/// `0x00910600`: forwards a constant zero argument to the callee
/// (FUN_0087b4a0); the callee is caller-supplied.
pub fn w32a62_call_with_zero<F: FnOnce(u32)>(callee: F) {
    callee(0);
}

/// `0x00917d00`: control-flow-guard check on an indirect call, then a tail
/// call to FUN_009233d0 (modelled as a closure).
pub fn w32a62_guarded_forward<F: FnOnce()>(callee: F) {
    callee();
}

/// `0x00918290`: invokes the virtual method at vtable offset `0x78` of the
/// receiver with `param`, and always returns 1.
pub fn w32a62_vcall_0x78_returns_one<F: FnOnce(u32)>(vcall: F, param: u32) -> u32 {
    vcall(param);
    1
}

#[cfg(test)]
mod tests_wave32_agent62 {
    use super::*;

    #[test]
    fn wrappers_forward() {
        let mut v = 9;
        w32a62_call_with_zero(|a| v = a);
        assert_eq!(v, 0);
        let mut hit = false;
        w32a62_guarded_forward(|| hit = true);
        assert!(hit);
        let mut got = 0;
        assert_eq!(w32a62_vcall_0x78_returns_one(|p| got = p, 7), 1);
        assert_eq!(got, 7);
    }
}
