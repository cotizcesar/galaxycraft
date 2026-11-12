#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent12.rs
// ===========================================================================
// Wave 34 translation batch 12.
//
// Window: queue: 3 addresses, module `runtime.rs`.
// Skipped (MMX bodies whose decompilation mangles byte unpacking):
// FUN_004bd000, FUN_004bd124.

/// `0x004bd09b`: six-tap vertical MMX filter over rows of four signed 16-bit
/// words.  `taps` holds the qwords at indices 0,2,4,6,8,10 of the filter
/// table (each four i16 lanes).  Row weights apply to rows -2,-1,0,+1,+2,+3
/// relative to the current row in the order
/// taps[0]*r(-2) + taps[1]*r(-1) + taps[2]*r(0) + taps[3]*r(+1)... see below:
/// the saturating sum order is r(-1)*t[1], r(+2)*t[4], r(0)*t[2], r(-2)*t[0],
/// r(+1)*t[3], r(+3)*t[5].  Each product keeps the low 16 bits (`pmullw`),
/// sums saturate (`paddsw`), then `+0x40`, `>> 7`, and each lane is clamped
/// to 0..=255 and packed to one byte.  `src_start` is the byte offset of the
/// current row; it advances by `src_stride` per output row; `dst` advances by
/// `dst_stride`.
pub fn w34a12_vertical_6tap_filter(
    src: &[u8],
    src_start: usize,
    src_stride: usize,
    dst: &mut [u8],
    dst_start: usize,
    dst_stride: usize,
    rows: usize,
    taps: &[[i16; 4]; 6],
) {
    let rd = |off: usize| -> [i16; 4] {
        let mut o = [0i16; 4];
        for (i, v) in o.iter_mut().enumerate() {
            *v = i16::from_le_bytes([src[off + i * 2], src[off + i * 2 + 1]]);
        }
        o
    };
    let mut cur = src_start;
    let mut d = dst_start;
    for _ in 0..rows {
        // (row offset in strides relative to current, tap index)
        let order: [(isize, usize); 6] = [(-1, 1), (2, 4), (0, 2), (-2, 0), (1, 3), (3, 5)];
        let mut acc = [0i16; 4];
        for (k, (rel, t)) in order.iter().enumerate() {
            let off = (cur as isize + rel * src_stride as isize) as usize;
            let r = rd(off);
            for l in 0..4 {
                let p = r[l].wrapping_mul(taps[*t][l]);
                acc[l] = if k == 0 { p } else { acc[l].saturating_add(p) };
            }
        }
        for l in 0..4 {
            let v = acc[l].saturating_add(0x40) >> 7;
            dst[d + l] = v.clamp(0, 255) as u8;
        }
        cur += src_stride;
        d += dst_stride;
    }
}

#[cfg(test)]
mod tests_wave34_agent12 {
    #[test]
    fn identity_tap() {
        let mut src = vec![0u8; 8 * 16];
        // row index 2 (current row when start = 2*stride), lane values 128,256,-5,1000
        let vals = [128i16, 100, -5, 200];
        for (i, v) in vals.iter().enumerate() {
            src[2 * 8 + i * 2..2 * 8 + i * 2 + 2].copy_from_slice(&v.to_le_bytes());
        }
        let mut taps = [[0i16; 4]; 6];
        taps[2] = [128; 4];
        let mut dst = [0u8; 4];
        super::w34a12_vertical_6tap_filter(&src, 16, 8, &mut dst, 0, 4, 1, &taps);
        assert_eq!(dst, [128, 100, 0, 200]);
    }
}
