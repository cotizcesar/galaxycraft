#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent14.rs
// ===========================================================================
// Wave 33 agent 14: guarded virtual-forwarding wrappers.

/// `0x00850f60`: when `guard(a, b)` is true, forwards the five arguments to
/// the inner object's vtable slot `0x54`, modelled as `forward`.
pub fn w33a14_guarded_forward5<G, F>(
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    guard: G,
    mut forward: F,
) where
    G: FnOnce(u32, u32) -> bool,
    F: FnMut(u32, u32, u32, u32, u32),
{
    if guard(a, b) {
        forward(a, b, c, d, e);
    }
}

/// `0x00850fa0`: when `guard(a, b)` is true, forwards the six arguments to
/// the inner object's vtable slot `0x50`, modelled as `forward`.
pub fn w33a14_guarded_forward6<G, F>(args: [u32; 6], guard: G, mut forward: F)
where
    G: FnOnce(u32, u32) -> bool,
    F: FnMut([u32; 6]),
{
    if guard(args[0], args[1]) {
        forward(args);
    }
}

/// `0x00850fe0`: when `guard(a, b)` is true, forwards the four arguments to
/// the inner object's vtable slot `0x34` and returns 1; otherwise returns 0.
pub fn w33a14_guarded_forward4<G, F>(args: [u32; 4], guard: G, mut forward: F) -> u32
where
    G: FnOnce(u32, u32) -> bool,
    F: FnMut([u32; 4]),
{
    if guard(args[0], args[1]) {
        forward(args);
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests_wave33_agent14 {
    use super::*;

    #[test]
    fn forwards_only_when_guard_passes() {
        let mut n = 0;
        w33a14_guarded_forward5(1, 2, 3, 4, 5, |_, _| true, |_, _, _, _, _| n += 1);
        w33a14_guarded_forward5(1, 2, 3, 4, 5, |_, _| false, |_, _, _, _, _| n += 10);
        w33a14_guarded_forward6([1; 6], |_, _| true, |_| n += 100);
        assert_eq!(n, 101);
        assert_eq!(w33a14_guarded_forward4([1; 4], |a, b| a == b, |_| {}), 1);
        assert_eq!(
            w33a14_guarded_forward4([1, 2, 0, 0], |a, b| a == b, |_| {}),
            0
        );
    }
}
