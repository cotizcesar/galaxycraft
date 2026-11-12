#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent20.rs
// ===========================================================================
// Wave 32 agent 20: FMOD-style handle-validated call wrappers.

/// `0x00984a50`: resolves `handle` (status 0 means success) and, on success,
/// invokes the object's virtual slot at vtable offset `0xd0` with `arg`.
pub fn w32a20_call_vslot_d0<O, R>(
    handle: u32,
    arg: u32,
    lookup: impl FnOnce(u32) -> Result<O, i32>,
    vslot_d0: impl FnOnce(O, u32) -> R,
) -> Option<R> {
    match lookup(handle) {
        Ok(obj) => Some(vslot_d0(obj, arg)),
        Err(_) => None,
    }
}

/// `0x00984c10`: validates `handle` (status 0 means success) then forwards
/// `(a, b)` to the worker.
pub fn w32a20_validated_forward_a<R>(
    handle: u32,
    a: u32,
    b: u32,
    validate: impl FnOnce(u32) -> i32,
    worker: impl FnOnce(u32, u32) -> R,
) -> Option<R> {
    if validate(handle) == 0 {
        Some(worker(a, b))
    } else {
        None
    }
}

/// `0x00984c40`: same shape as the previous wrapper with a different worker.
pub fn w32a20_validated_forward_b<R>(
    handle: u32,
    a: u32,
    b: u32,
    validate: impl FnOnce(u32) -> i32,
    worker: impl FnOnce(u32, u32) -> R,
) -> Option<R> {
    if validate(handle) == 0 {
        Some(worker(a, b))
    } else {
        None
    }
}

#[cfg(test)]
mod tests_wave32_agent20 {
    use super::*;

    #[test]
    fn wrappers_gate_on_status() {
        assert_eq!(
            w32a20_call_vslot_d0(1, 5, |_| Ok(2u32), |o, a| o + a),
            Some(7)
        );
        assert_eq!(
            w32a20_call_vslot_d0(1, 5, |_| Err::<u32, _>(3), |o, a| o + a),
            None
        );
        assert_eq!(
            w32a20_validated_forward_a(1, 2, 3, |_| 0, |a, b| a * b),
            Some(6)
        );
        assert_eq!(
            w32a20_validated_forward_a(1, 2, 3, |_| 1, |a, b| a * b),
            None
        );
        assert_eq!(
            w32a20_validated_forward_b(1, 2, 3, |_| 0, |a, b| a + b),
            Some(5)
        );
        assert_eq!(
            w32a20_validated_forward_b(1, 2, 3, |_| 9, |a, b| a + b),
            None
        );
    }
}
