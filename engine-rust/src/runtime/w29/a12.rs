#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent12.rs
// ===========================================================================
// Wave 29 agent 12: small runtime helpers.

/// `0x00ace190`: min of two 64-bit values stored as (lo, hi) u32 pairs;
/// returns `b` only when `b < a` (hi compared first), else `a`.
pub fn w29a12_min_pair(a: (u32, u32), b: (u32, u32)) -> (u32, u32) {
    // tuples are (lo, hi)
    if b.1 <= a.1 && (b.1 < a.1 || b.0 < a.0) {
        b
    } else {
        a
    }
}

/// Layout of the object initialised by `0x00ace270` (0x1d bytes used).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W29a12Obj {
    pub dw: [u32; 7],
    pub flag: u8,
}

/// `0x00ace270`: stores `param` at dword 0, zeroes dwords 2..=6 and the
/// byte at offset 0x1c (dword 1 is left untouched).
pub fn w29a12_init_obj(o: &mut W29a12Obj, param: u32) {
    o.dw[0] = param;
    for i in 2..7 {
        o.dw[i] = 0;
    }
    o.flag = 0;
}

/// Two owned buffers plus a trailing dword, as used by `0x00ace320`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W29a12Pair {
    pub a: Vec<u8>,
    pub b: Vec<u8>,
    pub extra: u32,
}

/// `0x00ace320`: move-assign; takes `src.a` and `src.b` (leaving them
/// empty), frees the previous buffers of `dst`, copies the dword at +8.
pub fn w29a12_move_assign(dst: &mut W29a12Pair, src: &mut W29a12Pair) {
    dst.a = std::mem::take(&mut src.a);
    dst.b = std::mem::take(&mut src.b);
    dst.extra = src.extra;
}

#[cfg(test)]
mod tests_wave29_agent12 {
    use super::*;

    #[test]
    fn min_pair() {
        assert_eq!(w29a12_min_pair((5, 1), (3, 1)), (3, 1));
        assert_eq!(w29a12_min_pair((5, 1), (5, 1)), (5, 1));
        assert_eq!(w29a12_min_pair((5, 1), (9, 0)), (9, 0));
        assert_eq!(w29a12_min_pair((5, 1), (0, 2)), (5, 1));
    }

    #[test]
    fn init_obj() {
        let mut o = W29a12Obj {
            dw: [9; 7],
            flag: 7,
        };
        w29a12_init_obj(&mut o, 42);
        assert_eq!(o.dw, [42, 9, 0, 0, 0, 0, 0]);
        assert_eq!(o.flag, 0);
    }

    #[test]
    fn move_assign() {
        let mut d = W29a12Pair {
            a: vec![1],
            b: vec![2],
            extra: 0,
        };
        let mut s = W29a12Pair {
            a: vec![3],
            b: vec![4, 5],
            extra: 6,
        };
        w29a12_move_assign(&mut d, &mut s);
        assert_eq!(
            d,
            W29a12Pair {
                a: vec![3],
                b: vec![4, 5],
                extra: 6
            }
        );
        assert!(s.a.is_empty() && s.b.is_empty());
    }
}
