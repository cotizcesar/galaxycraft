#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent13.rs
// ===========================================================================
// Wave 34 translation batch 13: MMX bilinear predictors.

/// Saturating pack of a signed 16-bit lane to a byte (packuswb).
fn w34a13_sat(v: i16) -> u8 {
    v.clamp(0, 255) as u8
}

/// Shared two-pass bilinear 4-row kernel used by `0x004bd235` (8 wide) and
/// `0x004bd346` (4 wide).  Pass 1 filters `src[x]`,`src[x+1]` with
/// `hf` (tap0, tap1) as `(a*t0 + b*t1 + 0x40) >> 7` saturated to a byte; pass 2
/// blends the previous and current filtered rows with `vf` the same way.
fn w34a13_bilinear(
    src: &[u8],
    src_stride: usize,
    hf: (i16, i16),
    vf: (i16, i16),
    dst: &mut [u8],
    dst_stride: usize,
    width: usize,
) {
    let pass1 = |row: usize| -> Vec<u8> {
        (0..width)
            .map(|x| {
                let a = src[row * src_stride + x] as i16;
                let b = src[row * src_stride + x + 1] as i16;
                w34a13_sat(
                    (a.wrapping_mul(hf.0)
                        .wrapping_add(b.wrapping_mul(hf.1))
                        .wrapping_add(0x40))
                        >> 7,
                )
            })
            .collect()
    };
    let mut prev = pass1(0);
    for r in 0..4 {
        let cur = pass1(r + 1);
        for x in 0..width {
            let v = (prev[x] as i16)
                .wrapping_mul(vf.0)
                .wrapping_add((cur[x] as i16).wrapping_mul(vf.1))
                .wrapping_add(0x40)
                >> 7;
            dst[r * dst_stride + x] = w34a13_sat(v);
        }
        prev = cur;
    }
}

/// `0x004bd235`: 8x4 two-pass bilinear predictor (horizontal then vertical
/// filter, rounding 0x40, shift 7, byte saturation).  Needs 5 source rows of
/// 9 bytes.
pub fn w34a13_bilinear_8x4(
    src: &[u8],
    src_stride: usize,
    hf: (i16, i16),
    vf: (i16, i16),
    dst: &mut [u8],
    dst_stride: usize,
) {
    w34a13_bilinear(src, src_stride, hf, vf, dst, dst_stride, 8);
}

/// `0x004bd346`: 4x4 variant of the same two-pass bilinear predictor.
pub fn w34a13_bilinear_4x4(
    src: &[u8],
    src_stride: usize,
    hf: (i16, i16),
    vf: (i16, i16),
    dst: &mut [u8],
    dst_stride: usize,
) {
    w34a13_bilinear(src, src_stride, hf, vf, dst, dst_stride, 4);
}

#[cfg(test)]
mod tests_wave34_agent13 {
    use super::*;

    #[test]
    fn identity_filter_copies() {
        let src: Vec<u8> = (0..5 * 16).map(|i| (i * 7) as u8).collect();
        let mut dst = [0u8; 4 * 8];
        w34a13_bilinear_8x4(&src, 16, (128, 0), (128, 0), &mut dst, 8);
        for r in 0..4 {
            for x in 0..8 {
                assert_eq!(dst[r * 8 + x], src[r * 16 + x]);
            }
        }
        let mut d4 = [0u8; 16];
        w34a13_bilinear_4x4(&src, 16, (128, 0), (128, 0), &mut d4, 4);
        assert_eq!(d4[4], src[16]);
    }

    #[test]
    fn half_filter_averages_and_saturates() {
        let mut src = vec![0u8; 5 * 8];
        for r in 0..5 {
            src[r * 8] = 100;
            src[r * 8 + 1] = 51;
        }
        let mut d = [0u8; 16];
        w34a13_bilinear_4x4(&src, 8, (64, 64), (128, 0), &mut d, 4);
        assert_eq!(d[0], 76);
        assert_eq!(w34a13_sat(300), 255);
        assert_eq!(w34a13_sat(-5), 0);
    }
}
