#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent28.rs
// ===========================================================================
// Wave 34 translation batch 28: bilinear filter kernels (two-tap, 7-bit).

fn w34a28_taps(filter: &[u8; 16]) -> (i16, i16) {
    (
        i16::from_le_bytes([filter[6], filter[7]]),
        i16::from_le_bytes([filter[8], filter[9]]),
    )
}

fn w34a28_tap(a: u8, b: u8, w0: i16, w1: i16) -> u8 {
    let p0 = (a as i16).wrapping_mul(w0);
    let p1 = (b as i16).wrapping_mul(w1);
    let s = p0.saturating_add(p1).saturating_add(0x40) >> 7;
    s.clamp(0, 255) as u8
}

fn w34a28_avg(a: u8, b: u8) -> u8 {
    ((a as u16 + b as u16 + 1) >> 1) as u8
}

fn w34a28_rows_avg(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[u8; 16],
    width: usize,
) {
    let (w0, w1) = w34a28_taps(filter);
    for r in 0..rows {
        let s = r * src_stride;
        let d = r * dst_stride;
        for i in 0..width {
            let v = w34a28_tap(src[s + i], src[s + src_stride + i], w0, w1);
            dst[d + i] = w34a28_avg(v, dst[d + i]);
        }
    }
}

/// `0x004d9cc1`: vertical two-tap bilinear filter over 8 pixels per row,
/// blended with the existing destination via rounding average.  Taps are
/// the 16-bit words 3 and 4 of `filter`; `(a*w0 + b*w1 + 64) >> 7` with
/// signed saturation, clamped to a byte.  `rows == 0` is a no-op here
/// (the original do-while would wrap).
pub fn w34a28_bilinear8_avg(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[u8; 16],
) {
    w34a28_rows_avg(src, src_stride, dst, dst_stride, rows, filter, 8);
}

/// `0x004d9d43`: same as the 8-wide kernel but 16 pixels per row.
pub fn w34a28_bilinear16_avg(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[u8; 16],
) {
    w34a28_rows_avg(src, src_stride, dst, dst_stride, rows, filter, 16);
}

/// `0x004d9dea`: horizontal two-tap bilinear filter over 4 pixels per row
/// (pixel `i` mixes source bytes `i` and `i+1`), stored without averaging.
pub fn w34a28_bilinear4_h(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[u8; 16],
) {
    let (w0, w1) = w34a28_taps(filter);
    for r in 0..rows {
        for i in 0..4 {
            dst[r * dst_stride + i] =
                w34a28_tap(src[r * src_stride + i], src[r * src_stride + i + 1], w0, w1);
        }
    }
}

#[cfg(test)]
mod tests_wave34_agent28 {
    use super::*;

    fn f(w0: i16, w1: i16) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[6..8].copy_from_slice(&w0.to_le_bytes());
        b[8..10].copy_from_slice(&w1.to_le_bytes());
        b
    }

    #[test]
    fn half_mix_avg() {
        let src = [100u8; 32];
        let mut dst = [50u8; 16];
        w34a28_bilinear8_avg(&src, 8, &mut dst, 8, 1, &f(64, 64));
        assert_eq!(&dst[..8], &[75u8; 8]);
        assert_eq!(&dst[8..], &[50u8; 8]);
    }

    #[test]
    fn wide16_and_clamp() {
        let src = [255u8; 48];
        let mut dst = [0u8; 16];
        w34a28_bilinear16_avg(&src, 16, &mut dst, 16, 1, &f(128, 128));
        assert_eq!(dst, [128u8; 16]);
    }

    #[test]
    fn horizontal4() {
        let src = [10u8, 20, 30, 40, 50, 0, 0, 0];
        let mut dst = [0u8; 4];
        w34a28_bilinear4_h(&src, 8, &mut dst, 4, 1, &f(64, 64));
        assert_eq!(dst, [15, 25, 35, 45]);
    }
}
