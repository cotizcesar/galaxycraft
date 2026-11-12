#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent26.rs
// ===========================================================================
// Wave 34 translation batch 26.

/// `0x004d9aa0`: two-row bilinear blend (SSE2 kernel).  For each of
/// `rows` rows, every output byte is
/// `clamp((a*f[3] + b*f[4] + 64) >> 7, 0, 255)` where `a` is the source
/// byte in the current row and `b` the byte `src_stride` bytes later.
/// Four bytes are produced per row; the source advances by `src_stride`
/// and the destination by `dst_stride` per iteration.  `filter` holds the
/// eight 16-bit taps; only words 3 and 4 are used.
pub fn w34a26_bilinear_pair_rows(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[i16; 8],
) {
    let (f0, f1) = (filter[3] as i32, filter[4] as i32);
    for r in 0..rows {
        for i in 0..4 {
            let a = src[r * src_stride + i] as i32;
            let b = src[r * src_stride + src_stride + i] as i32;
            let sum = (a * f0 + b * f1).clamp(-32768, 32767);
            let v = ((sum + 64).clamp(-32768, 32767)) >> 7;
            dst[r * dst_stride + i] = v.clamp(0, 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests_wave34_agent26 {
    use super::*;

    #[test]
    fn blend_halves() {
        let src = [100u8, 0, 255, 10, 200, 0, 255, 30];
        let mut dst = [0u8; 4];
        let f = [0, 0, 0, 64, 64, 0, 0, 0];
        w34a26_bilinear_pair_rows(&src, 4, &mut dst, 4, 1, &f);
        assert_eq!(dst, [150, 0, 255, 20]);
    }
}
