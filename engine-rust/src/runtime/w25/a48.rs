#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent48.rs
// ===========================================================================
/// Three-word record moved by `0x008a0480`: pointer, pointer, and a
/// size word whose top bit is a flag.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W25a48Triple {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

/// `0x008a0480`: move-construct from `src`.  Returns `None` when the source
/// size word has bit 31 set (the retail path diverges into a non-returning
/// helper).  Otherwise the destination takes `a`, `b` and `c & 0x7fffffff`;
/// the source keeps only its top flag bit in `c` and has `a`, `b` cleared.
pub fn w25a48_move_construct(src: &mut W25a48Triple) -> Option<W25a48Triple> {
    if src.c & 0x8000_0000 != 0 {
        return None;
    }
    let dst = W25a48Triple {
        a: src.a,
        b: src.b,
        c: src.c & 0x7fff_ffff,
    };
    src.c &= 0x8000_0000;
    src.a = 0;
    src.b = 0;
    Some(dst)
}

#[cfg(test)]
mod tests_wave25_agent48 {
    use super::*;

    #[test]
    fn move_construct_clears_source() {
        let mut s = W25a48Triple { a: 1, b: 2, c: 7 };
        let d = w25a48_move_construct(&mut s).unwrap();
        assert_eq!(d, W25a48Triple { a: 1, b: 2, c: 7 });
        assert_eq!(s, W25a48Triple::default());
    }

    #[test]
    fn move_construct_flagged_diverges() {
        let mut s = W25a48Triple {
            a: 1,
            b: 2,
            c: 0x8000_0003,
        };
        assert!(w25a48_move_construct(&mut s).is_none());
    }
}
