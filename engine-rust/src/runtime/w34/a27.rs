#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent27.rs
// ===========================================================================
// Wave 34 agent 27: libvpx 2-tap bilinear SSE2 kernels.

fn w34a27_tap(p0: u8, p1: u8, f0: i16, f1: i16) -> u8 {
    let a = (p0 as i16).wrapping_mul(f0);
    let b = (p1 as i16).wrapping_mul(f1);
    let s = a.saturating_add(b).saturating_add(64) >> 7;
    s.clamp(0, 255) as u8
}

/// `0x004d9b20`: 8-wide bilinear filter. Each output byte blends the source
/// pixel and the pixel `src_stride` bytes below with taps `filter[3]` and
/// `filter[4]` (16-bit wrapping multiply, saturating add, +64, >>7, clamp).
pub fn w34a27_bilinear8(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[i16; 8],
) {
    for r in 0..rows {
        for i in 0..8 {
            dst[r * dst_stride + i] = w34a27_tap(
                src[r * src_stride + i],
                src[(r + 1) * src_stride + i],
                filter[3],
                filter[4],
            );
        }
    }
}

/// `0x004d9b9a`: 16-wide variant of the same bilinear blend.
pub fn w34a27_bilinear16(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[i16; 8],
) {
    for r in 0..rows {
        for i in 0..16 {
            dst[r * dst_stride + i] = w34a27_tap(
                src[r * src_stride + i],
                src[(r + 1) * src_stride + i],
                filter[3],
                filter[4],
            );
        }
    }
}

/// `0x004d9c39`: 4-wide bilinear blend averaged (rounding up) with the
/// existing destination bytes.
pub fn w34a27_bilinear4_avg(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[i16; 8],
) {
    for r in 0..rows {
        for i in 0..4 {
            let v = w34a27_tap(
                src[r * src_stride + i],
                src[(r + 1) * src_stride + i],
                filter[3],
                filter[4],
            );
            let d = &mut dst[r * dst_stride + i];
            *d = ((v as u16 + *d as u16 + 1) >> 1) as u8;
        }
    }
}

#[cfg(test)]
mod tests_wave34_agent27 {
    use super::*;

    #[test]
    fn blend_and_avg() {
        let f = [0, 0, 0, 64, 64, 0, 0, 0];
        let src = [
            10u8, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160,
        ];
        let mut d = [0u8; 8];
        w34a27_bilinear8(&src, 8, &mut d, 8, 1, &f);
        assert_eq!(d[0], 50);
        let mut d16 = [0u8; 16];
        let s16: Vec<u8> = (0..32).map(|x| x as u8 * 8).collect();
        w34a27_bilinear16(&s16, 16, &mut d16, 16, 1, &f);
        assert_eq!(d16[0], 64);
        let mut a = [100u8; 4];
        w34a27_bilinear4_avg(&src, 8, &mut a, 4, 1, &f);
        assert_eq!(a[0], 75);
    }
}
