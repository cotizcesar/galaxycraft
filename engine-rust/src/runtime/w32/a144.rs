#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent144.rs
// ===========================================================================
// Wave 32 translation batch 144: CRT integer-conversion forwarding wrappers.

/// `0x00ca3717`: forwards `(a, b, c)` to the worker with the fixed flags
/// `(1, 0, 0)`.
pub fn w32a144_forward_flags_1_0_0<R>(
    a: u32,
    b: u32,
    c: u32,
    worker: impl FnOnce(u32, u32, u32, u32, u32, u32) -> R,
) -> R {
    worker(a, b, c, 1, 0, 0)
}

/// `0x00ca3935`: computes the "signed negative" flag (1 only when the radix
/// is 10 and the value is negative) and forwards to the converter.
pub fn w32a144_radix_sign_flag<R>(
    value: i32,
    b: u32,
    c: u32,
    radix: i32,
    worker: impl FnOnce(i32, u32, u32, i32, u32) -> R,
) -> R {
    let flag = if radix == 10 && value < 0 { 1 } else { 0 };
    worker(value, b, c, radix, flag)
}

/// `0x00ca485f`: argument-less forwarder to its worker.
pub fn w32a144_forward_void<R>(worker: impl FnOnce() -> R) -> R {
    worker()
}

#[cfg(test)]
mod tests_wave32_agent144 {
    use super::*;

    #[test]
    fn forwards() {
        assert_eq!(
            w32a144_forward_flags_1_0_0(1, 2, 3, |a, b, c, d, e, f| [a, b, c, d, e, f]),
            [1, 2, 3, 1, 0, 0]
        );
        assert_eq!(w32a144_radix_sign_flag(-5, 0, 0, 10, |_, _, _, _, f| f), 1);
        assert_eq!(w32a144_radix_sign_flag(-5, 0, 0, 16, |_, _, _, _, f| f), 0);
        assert_eq!(w32a144_radix_sign_flag(5, 0, 0, 10, |_, _, _, _, f| f), 0);
        assert_eq!(w32a144_forward_void(|| 7), 7);
    }
}
