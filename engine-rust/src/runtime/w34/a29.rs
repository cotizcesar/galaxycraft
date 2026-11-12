#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent29.rs
// ===========================================================================
// Wave 34 agent 29: SSE2 bilinear 2-tap horizontal predictors.

/// Shared bilinear tap kernel: `sat16(p0*t0 + p1*t1)`, +64, >>7, clamp to u8.
/// The SSE2 code uses `pmullw` (wrapping), `paddsw` (saturating) and `packuswb`.
fn w34a29_tap(p0: u8, p1: u8, t0: i16, t1: i16) -> u8 {
    let a = (p0 as i16).wrapping_mul(t0);
    let b = (p1 as i16).wrapping_mul(t1);
    let s = a.saturating_add(b).saturating_add(0x40) >> 7;
    s.clamp(0, 255) as u8
}

/// Taps are words 3 and 4 of the 16-byte filter block.
fn w34a29_taps(filter: &[i16; 8]) -> (i16, i16) {
    (filter[3], filter[4])
}

/// `0x004d9e6e`: 8-wide horizontal bilinear filter, `rows` rows; each output
/// pixel is `clamp((src[i]*f[3] + src[i+1]*f[4] + 64) >> 7)`.
pub fn w34a29_bilinear_h8(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[i16; 8],
) {
    let (t0, t1) = w34a29_taps(filter);
    for r in 0..rows {
        for i in 0..8 {
            dst[r * dst_stride + i] =
                w34a29_tap(src[r * src_stride + i], src[r * src_stride + i + 1], t0, t1);
        }
    }
}

/// `0x004d9eec`: 16-wide variant of the same horizontal bilinear filter.
pub fn w34a29_bilinear_h16(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[i16; 8],
) {
    let (t0, t1) = w34a29_taps(filter);
    for r in 0..rows {
        for i in 0..16 {
            dst[r * dst_stride + i] =
                w34a29_tap(src[r * src_stride + i], src[r * src_stride + i + 1], t0, t1);
        }
    }
}

/// `0x004d9f8b`: 4-wide horizontal bilinear filter averaged (`pavgb`,
/// `(a+b+1)>>1`) into the existing destination pixels.
pub fn w34a29_bilinear_h4_avg(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[i16; 8],
) {
    let (t0, t1) = w34a29_taps(filter);
    for r in 0..rows {
        for i in 0..4 {
            let v = w34a29_tap(src[r * src_stride + i], src[r * src_stride + i + 1], t0, t1);
            let d = &mut dst[r * dst_stride + i];
            *d = ((v as u16 + *d as u16 + 1) >> 1) as u8;
        }
    }
}

#[cfg(test)]
mod tests_wave34_agent29 {
    use super::*;

    const F: [i16; 8] = [0, 0, 0, 96, 32, 0, 0, 0];

    #[test]
    fn h8_and_h16() {
        let src: Vec<u8> = (0..40).map(|i| (i * 7) as u8).collect();
        let mut d = [0u8; 16];
        w34a29_bilinear_h8(&src, 20, &mut d, 8, 2, &F);
        assert_eq!(d[0], ((0 * 96 + 7 * 32 + 64) >> 7) as u8);
        let mut d16 = [0u8; 16];
        w34a29_bilinear_h16(&src, 20, &mut d16, 16, 1, &[0, 0, 0, 128, 0, 0, 0, 0]);
        assert_eq!(d16[5], src[5]);
    }

    #[test]
    fn clamp_and_avg() {
        let src = [255u8; 10];
        let mut d = [0u8; 4];
        w34a29_bilinear_h8(
            &[255u8; 9],
            9,
            &mut [0u8; 8],
            8,
            1,
            &[0, 0, 0, 0x7fff, 0x7fff, 0, 0, 0],
        );
        w34a29_bilinear_h4_avg(&src, 10, &mut d, 4, 1, &[0, 0, 0, 128, 0, 0, 0, 0]);
        assert_eq!(d, [128; 4]);
    }
}
