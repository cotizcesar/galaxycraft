#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent13.rs
// ===========================================================================
// Wave 33 translation batch 13: guarded virtual-dispatch forwarders.

/// `0x00850eb0`: if the membership check on `(a, b)` passes, forwards
/// `(a, b, c)` to the inner object's vtable slot `0x64`; otherwise no-op.
/// Returns whether the forward happened.
pub fn w33a13_forward3(
    contains: impl FnOnce(u32, u32) -> bool,
    a: u32,
    b: u32,
    c: u32,
    forward: impl FnOnce(u32, u32, u32),
) -> bool {
    if !contains(a, b) {
        return false;
    }
    forward(a, b, c);
    true
}

/// `0x00850ee0`: guarded forward of six arguments to vtable slot `0x44`.
pub fn w33a13_forward6(
    contains: impl FnOnce(u32, u32) -> bool,
    args: [u32; 6],
    forward: impl FnOnce([u32; 6]),
) -> bool {
    if !contains(args[0], args[1]) {
        return false;
    }
    forward(args);
    true
}

/// `0x00850f20`: guarded forward of seven arguments to vtable slot `0x4c`.
pub fn w33a13_forward7(
    contains: impl FnOnce(u32, u32) -> bool,
    args: [u32; 7],
    forward: impl FnOnce([u32; 7]),
) -> bool {
    if !contains(args[0], args[1]) {
        return false;
    }
    forward(args);
    true
}

#[cfg(test)]
mod tests_wave33_agent13 {
    use super::*;

    #[test]
    fn guard_blocks_and_allows() {
        let mut hit = 0;
        assert!(!w33a13_forward3(|_, _| false, 1, 2, 3, |_, _, _| hit += 1));
        assert_eq!(hit, 0);
        assert!(w33a13_forward3(
            |a, b| a == 1 && b == 2,
            1,
            2,
            3,
            |_, _, c| hit += c
        ));
        assert_eq!(hit, 3);
        assert!(w33a13_forward6(
            |_, _| true,
            [1, 2, 3, 4, 5, 6],
            |a| hit += a[5]
        ));
        assert!(!w33a13_forward7(|_, _| false, [0; 7], |_| hit = 0));
        assert!(w33a13_forward7(|_, _| true, [7; 7], |a| hit += a[6]));
        assert_eq!(hit, 16);
    }
}
