#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent23.rs
// ===========================================================================
// Wave 32 translation batch 23: handle-validating wrappers.

/// Shared shape of the three wrappers: the handle is validated by
/// FUN_009945c0 (status 0 means valid) and only then is the inner call made.
fn w32a23_if_valid<F: FnOnce()>(validate_status: u32, inner: F) {
    if validate_status == 0 {
        inner();
    }
}

/// `0x00984e80`: validates the handle (status via FUN_009945c0) and, when the
/// status is 0, forwards `(a, b, c)` to the inner routine FUN_009907b0.
pub fn w32a23_forward3<F: FnOnce(u32, u32, u32)>(
    validate_status: u32,
    a: u32,
    b: u32,
    c: u32,
    inner: F,
) {
    w32a23_if_valid(validate_status, || inner(a, b, c));
}

/// `0x00984f20`: validates the handle and, when the status is 0, calls the
/// inner routine FUN_00992070 with one argument.
pub fn w32a23_forward1<F: FnOnce(u32)>(validate_status: u32, a: u32, inner: F) {
    w32a23_if_valid(validate_status, || inner(a));
}

/// `0x00984f50`: validates the handle and, when the status is 0, calls the
/// inner routine FUN_00992090 with two arguments.
pub fn w32a23_forward2<F: FnOnce(u32, u32)>(validate_status: u32, a: u32, b: u32, inner: F) {
    w32a23_if_valid(validate_status, || inner(a, b));
}

#[cfg(test)]
mod tests_wave32_agent23 {
    use super::*;

    #[test]
    fn forwards_only_when_valid() {
        let mut got = None;
        w32a23_forward3(0, 1, 2, 3, |a, b, c| got = Some((a, b, c)));
        assert_eq!(got, Some((1, 2, 3)));
        got = None;
        w32a23_forward3(0x24, 1, 2, 3, |a, b, c| got = Some((a, b, c)));
        assert_eq!(got, None);
        let mut n = 0;
        w32a23_forward1(0, 5, |a| n = a);
        assert_eq!(n, 5);
        w32a23_forward1(0x25, 6, |a| n = a);
        assert_eq!(n, 5);
        let mut p = (0, 0);
        w32a23_forward2(0, 7, 8, |a, b| p = (a, b));
        assert_eq!(p, (7, 8));
    }
}
