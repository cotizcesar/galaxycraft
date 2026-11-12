#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent01.rs
// ===========================================================================
// Wave 34 translation batch 01.
//
// Thin wrappers around the code-point decoder FUN_007a5320 (already
// translated as `w33a01_decode_step`); the decoder is passed as a closure.

/// `0x007a7530`: calls the decoder with a (position, length) pair built from
/// its two arguments and discards the code point; `pos` and `len` are updated
/// by the decoder.
pub fn w34a01_decode_discard(
    pos: &mut usize,
    len: &mut usize,
    decode: &mut dyn FnMut(&mut usize, &mut usize) -> u32,
) {
    decode(pos, len);
}

/// `0x007a7560`: reads the `(pos, len)` pair from a two-word record and runs
/// the decoder on local copies, discarding the result (the record itself is
/// not written back).
pub fn w34a01_decode_record(
    record: &[usize; 2],
    decode: &mut dyn FnMut(&mut usize, &mut usize) -> u32,
) {
    let (mut pos, mut len) = (record[0], record[1]);
    decode(&mut pos, &mut len);
}

/// `0x007a7680`: runs the decoder on a (position, length) pair and returns
/// whether the decoded code point equals `0xfeff` (byte order mark).
pub fn w34a01_is_bom(
    mut pos: usize,
    mut len: usize,
    decode: &mut dyn FnMut(&mut usize, &mut usize) -> u32,
) -> bool {
    decode(&mut pos, &mut len) == 0xfeff
}

#[cfg(test)]
mod tests_wave34_agent01 {
    #[test]
    fn wrappers() {
        let mut d = |p: &mut usize, l: &mut usize| {
            *p += 1;
            *l -= 1;
            0xfeff
        };
        let (mut p, mut l) = (0, 3);
        super::w34a01_decode_discard(&mut p, &mut l, &mut d);
        assert_eq!((p, l), (1, 2));
        assert!(super::w34a01_is_bom(0, 3, &mut d));
        super::w34a01_decode_record(&[0, 3], &mut d);
        let mut e = |_: &mut usize, _: &mut usize| 0x41;
        assert!(!super::w34a01_is_bom(0, 1, &mut e));
    }
}
