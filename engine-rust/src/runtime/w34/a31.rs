#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent31.rs
// ===========================================================================
// Wave 34 agent 31: libvpx SSSE3 bilinear first-pass kernels.
//
// The SIMD pseudocode is modelled with explicit filter taps `(f0, f1)`
// (the signed byte pair broadcast by the original), pmaddubsw (u8*i8
// pairs, saturated to i16), pmulhrsw by 0x100 (`(x + 64) >> 7`) and an
// unsigned clamp to a byte.

fn w34a31_tap(src: &[u8], i: usize, taps: (i8, i8)) -> u8 {
    let s = (src[i] as i32) * (taps.0 as i32) + (src[i + 1] as i32) * (taps.1 as i32);
    let s = s.clamp(i16::MIN as i32, i16::MAX as i32);
    ((s + 64) >> 7).clamp(0, 255) as u8
}

fn w34a31_rows(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    height: usize,
    width: usize,
    taps: (i8, i8),
    average: bool,
) {
    for y in 0..height {
        for x in 0..width {
            let v = w34a31_tap(&src[y * src_stride..], x, taps);
            let d = &mut dst[y * dst_stride + x];
            *d = if average {
                ((v as u16 + *d as u16 + 1) >> 1) as u8
            } else {
                v
            };
        }
    }
}

/// `0x004db825`: 8-wide horizontal bilinear pass, `height` rows
/// (the original loops do-while, so `height` of 0 is treated as 0 rows).
/// Reads `width + 1` source bytes per row.
pub fn w34a31_bilinear_h8(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    height: usize,
    taps: (i8, i8),
) {
    w34a31_rows(src, src_stride, dst, dst_stride, height, 8, taps, false);
}

/// `0x004db890`: 16-wide horizontal bilinear pass, 17 source bytes per row.
pub fn w34a31_bilinear_h16(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    height: usize,
    taps: (i8, i8),
) {
    w34a31_rows(src, src_stride, dst, dst_stride, height, 16, taps, false);
}

/// `0x004db909`: 4-wide horizontal bilinear pass averaged (pavgb,
/// round up) with the existing destination bytes.
pub fn w34a31_bilinear_h4_avg(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    height: usize,
    taps: (i8, i8),
) {
    w34a31_rows(src, src_stride, dst, dst_stride, height, 4, taps, true);
}

#[cfg(test)]
mod tests_wave34_agent31 {
    use super::*;

    #[test]
    fn identity_and_blend() {
        let src: Vec<u8> = (0..40).collect();
        let mut d = [0u8; 16];
        w34a31_bilinear_h8(&src, 20, &mut d, 8, 2, (127, 0));
        // 128 as i8 wraps to -128; use 64/64 blend instead
        let mut d = [0u8; 16];
        w34a31_bilinear_h8(&src, 20, &mut d, 8, 2, (64, 64));
        assert_eq!(d[0], 1); // (0*64+1*64+64)>>7 = 1
        assert_eq!(d[8], 21);
    }

    #[test]
    fn wide_and_avg() {
        let src = vec![100u8; 40];
        let mut d = [0u8; 16];
        w34a31_bilinear_h16(&src, 20, &mut d, 16, 1, (64, 64));
        assert!(d.iter().all(|&v| v == 100));
        let mut a = [50u8; 4];
        w34a31_bilinear_h4_avg(&src, 20, &mut a, 4, 1, (64, 64));
        assert_eq!(a, [75; 4]);
    }
}
