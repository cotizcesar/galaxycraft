#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent56.rs
// ===========================================================================
// Wave 32 translation batch 56: thiscall forwarders over an object whose
// field at +4 is the inner target.

/// `0x00886cb0`: when the event handler `handled` returns zero, forwards
/// `(inner, event)` to `forward`; otherwise does nothing.
pub fn w32a56_forward_unless_handled<E: Copy, I: Copy>(
    inner: I,
    event: E,
    handled: impl FnOnce(E) -> u8,
    forward: impl FnOnce(I, E),
) {
    if handled(event) == 0 {
        forward(inner, event);
    }
}

/// `0x00892460`: same shape as the 0x00886cb0 forwarder with a different
/// forward target: runs `forward(inner, event)` only if `handled` is zero.
pub fn w32a56_forward_unless_handled_b<E: Copy, I: Copy>(
    inner: I,
    event: E,
    handled: impl FnOnce(E) -> u8,
    forward: impl FnOnce(I, E),
) {
    if handled(event) == 0 {
        forward(inner, event);
    }
}

/// `0x00897750`: unconditionally forwards `(inner, arg)` to `forward`.
pub fn w32a56_forward<E, I, R>(inner: I, arg: E, forward: impl FnOnce(I, E) -> R) -> R {
    forward(inner, arg)
}

#[cfg(test)]
mod tests_wave32_agent56 {
    use super::*;

    #[test]
    fn forwards_only_when_unhandled() {
        let mut got = vec![];
        w32a56_forward_unless_handled(7u32, 9u32, |_| 0, |i, e| got.push((i, e)));
        w32a56_forward_unless_handled(7u32, 9u32, |_| 1, |i, e| got.push((i + 1, e)));
        w32a56_forward_unless_handled_b(1u32, 2u32, |_| 0, |i, e| got.push((i, e)));
        assert_eq!(got, vec![(7, 9), (1, 2)]);
    }

    #[test]
    fn plain_forward() {
        assert_eq!(w32a56_forward(3, 4, |i, e| i * 10 + e), 34);
    }
}
