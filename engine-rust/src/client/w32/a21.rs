#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent21.rs
// ===========================================================================
// Wave 32 agent 21: handle-validating dispatch wrappers.

/// Handle lookup result shared by the wrappers: the handle validator
/// (callee FUN_009945c0) returns 0 when the handle is found in the registry list.
pub fn w32a21_handle_valid(registry: &[u32], handle: u32) -> bool {
    handle != 0 && registry.contains(&handle)
}

/// `0x00984c70`: validates `handle`; on success forwards two args to the callee.
pub fn w32a21_dispatch2<R>(
    registry: &[u32],
    handle: u32,
    a: u32,
    b: u32,
    f: impl FnOnce(u32, u32) -> R,
) -> Option<R> {
    if w32a21_handle_valid(registry, handle) {
        Some(f(a, b))
    } else {
        None
    }
}

/// `0x00984ca0`: validates `handle`; on success forwards four args to the callee.
pub fn w32a21_dispatch4_a<R>(
    registry: &[u32],
    handle: u32,
    args: [u32; 4],
    f: impl FnOnce(u32, u32, u32, u32) -> R,
) -> Option<R> {
    if w32a21_handle_valid(registry, handle) {
        Some(f(args[0], args[1], args[2], args[3]))
    } else {
        None
    }
}

/// `0x00984ce0`: same shape as the previous wrapper with a different callee.
pub fn w32a21_dispatch4_b<R>(
    registry: &[u32],
    handle: u32,
    args: [u32; 4],
    f: impl FnOnce(u32, u32, u32, u32) -> R,
) -> Option<R> {
    w32a21_dispatch4_a(registry, handle, args, f)
}

#[cfg(test)]
mod tests_wave32_agent21 {
    use super::*;

    #[test]
    fn dispatch() {
        let reg = [5, 9];
        assert_eq!(w32a21_dispatch2(&reg, 5, 1, 2, |a, b| a + b), Some(3));
        assert_eq!(w32a21_dispatch2(&reg, 0, 1, 2, |a, b| a + b), None);
        assert_eq!(
            w32a21_dispatch4_a(&reg, 9, [1, 2, 3, 4], |a, b, c, d| a + b + c + d),
            Some(10)
        );
        assert_eq!(
            w32a21_dispatch4_b(&reg, 7, [1, 2, 3, 4], |a, _, _, _| a),
            None
        );
    }
}
