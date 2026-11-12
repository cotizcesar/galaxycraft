#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent33.rs
// ===========================================================================
// Wave 34 agent 33 translations.

/// `0x00513ee0`: resets seven consecutive u32 globals (starting at the
/// block at 0xfa9030) to their defaults: 10000, 5000, 1000, 0xd48, 400,
/// 4000, 1000.
pub fn w34a33_reset_defaults(block: &mut [u32; 7]) {
    *block = [10000, 5000, 1000, 0xd48, 400, 4000, 1000];
}

/// `0x00532a30`: stores the result of the callee (taking the 24-bit ECX
/// argument) into a global.
pub fn w34a33_store_call(arg: u32, callee: impl Fn(u32) -> u32, global: &mut u32) {
    *global = callee(arg & 0x00ff_ffff);
}

#[cfg(test)]
mod tests_wave34_agent33 {
    use super::*;

    #[test]
    fn reset() {
        let mut b = [0u32; 7];
        w34a33_reset_defaults(&mut b);
        assert_eq!(b[4], 400);
        assert_eq!(b[5], 4000);
        assert_eq!(b[3], 0xd48);
    }

    #[test]
    fn store() {
        let mut g = 0;
        w34a33_store_call(0x0100_0005, |x| x + 1, &mut g);
        assert_eq!(g, 6);
    }
}
