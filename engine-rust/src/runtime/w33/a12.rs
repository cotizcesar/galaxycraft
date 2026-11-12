#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent12.rs
// ===========================================================================
// Wave 33 agent 12: guarded virtual-dispatch forwarders.

/// `0x0084f690`: calls the predicate (`FUN_0084f6d0`) with `(a, b)`; when it
/// returns nonzero, invokes the inner object's vtable slot `0x58` with
/// the nine arguments (the third dereferenced from its pointer).
pub fn w33a12_forward_slot58<P, D>(
    a: u32,
    b: u32,
    c: &u32,
    rest: [u32; 6],
    pred: P,
    dispatch: D,
) -> bool
where
    P: FnOnce(u32, u32) -> bool,
    D: FnOnce(u32, u32, u32, [u32; 6]),
{
    if pred(a, b) {
        dispatch(a, b, *c, rest);
        true
    } else {
        false
    }
}

/// `0x0084f750`: same guard; invokes vtable slot `0x40` with `(a, b, *c)`.
pub fn w33a12_forward_slot40<P, D>(a: u32, b: u32, c: &u32, pred: P, dispatch: D) -> bool
where
    P: FnOnce(u32, u32) -> bool,
    D: FnOnce(u32, u32, u32),
{
    if pred(a, b) {
        dispatch(a, b, *c);
        true
    } else {
        false
    }
}

/// `0x00850e80`: same guard; invokes vtable slot `0x48` with `(a, b, c, d)`.
pub fn w33a12_forward_slot48<P, D>(a: u32, b: u32, c: u32, d: u32, pred: P, dispatch: D) -> bool
where
    P: FnOnce(u32, u32) -> bool,
    D: FnOnce(u32, u32, u32, u32),
{
    if pred(a, b) {
        dispatch(a, b, c, d);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests_wave33_agent12 {
    use super::*;

    #[test]
    fn slot58() {
        let mut got = None;
        assert!(w33a12_forward_slot58(
            1,
            2,
            &3,
            [4, 5, 6, 7, 8, 9],
            |_, _| true,
            |a, b, c, r| got = Some((a, b, c, r))
        ));
        assert_eq!(got, Some((1, 2, 3, [4, 5, 6, 7, 8, 9])));
        assert!(!w33a12_forward_slot58(
            1,
            2,
            &3,
            [0; 6],
            |_, _| false,
            |_, _, _, _| panic!()
        ));
    }

    #[test]
    fn slot40_48() {
        let mut g = 0;
        assert!(w33a12_forward_slot40(
            1,
            2,
            &9,
            |a, b| a < b,
            |_, _, c| g = c
        ));
        assert_eq!(g, 9);
        assert!(!w33a12_forward_slot40(
            2,
            1,
            &9,
            |a, b| a < b,
            |_, _, _| panic!()
        ));
        assert!(w33a12_forward_slot48(
            1,
            2,
            3,
            4,
            |_, _| true,
            |a, b, c, d| g = a + b + c + d
        ));
        assert_eq!(g, 10);
        assert!(!w33a12_forward_slot48(
            1,
            2,
            3,
            4,
            |_, _| false,
            |_, _, _, _| panic!()
        ));
    }
}
