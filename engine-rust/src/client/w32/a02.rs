#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent02.rs
// ===========================================================================
// Wave 32 translation batch 02: small forwarding wrappers.

/// `0x00561800`: forwards three arguments to the callee and returns 0.
pub fn w32a02_forward3_return_zero<F: FnMut(u32, u32, u32)>(
    a: u32,
    b: u32,
    c: u32,
    mut callee: F,
) -> u8 {
    callee(a, b, c);
    0
}

/// `0x0056eae0`: looks up a record; when found and its field at +8 is
/// below `0x80`, passes that value to the callee.  Returns whether the
/// callee ran.
pub fn w32a02_call_if_small<L, F>(lookup: L, mut callee: F) -> bool
where
    L: FnOnce() -> Option<u32>,
    F: FnMut(u32),
{
    match lookup() {
        Some(v) if v < 0x80 => {
            callee(v);
            true
        }
        _ => false,
    }
}

/// `0x00674000`: forwards its single stack argument to the callee.
pub fn w32a02_forward1<T, F: FnOnce(T)>(arg: T, callee: F) {
    callee(arg)
}

#[cfg(test)]
mod tests_wave32_agent02 {
    use super::*;

    #[test]
    fn forwards() {
        let mut got = vec![];
        assert_eq!(
            w32a02_forward3_return_zero(1, 2, 3, |a, b, c| got.push(a + b + c)),
            0
        );
        assert_eq!(got, vec![6]);
        let mut n = None;
        w32a02_forward1(5, |x| n = Some(x));
        assert_eq!(n, Some(5));
    }

    #[test]
    fn small() {
        let mut n = 0;
        assert!(w32a02_call_if_small(|| Some(0x7f), |v| n = v));
        assert_eq!(n, 0x7f);
        assert!(!w32a02_call_if_small(|| Some(0x80), |_| panic!()));
        assert!(!w32a02_call_if_small(|| None, |_| panic!()));
    }
}
