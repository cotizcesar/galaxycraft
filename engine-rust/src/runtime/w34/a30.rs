#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent30.rs
// ===========================================================================
// Wave 34 translation batch 30: libvpx two-tap bilinear SSE kernels.

fn w34a30_two_tap_avg(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[i16; 8],
    width: usize,
) {
    let f0 = filter[3];
    let f1 = filter[4];
    for r in 0..rows {
        for i in 0..width {
            let a = src[r * src_stride + i] as i16;
            let b = src[r * src_stride + i + 1] as i16;
            let sum = a
                .wrapping_mul(f0)
                .saturating_add(b.wrapping_mul(f1))
                .saturating_add(0x40);
            let v = (sum >> 7).clamp(0, 255) as u32;
            let d = dst[r * dst_stride + i] as u32;
            dst[r * dst_stride + i] = ((v + d + 1) >> 1) as u8;
        }
    }
}

/// `0x004da017`: 8-wide bilinear row filter (taps are words 3 and 4 of the
/// filter), rounded by 0x40, shifted by 7, clamped, then averaged with the
/// existing destination bytes.  Runs `rows` (>= 1) rows.
pub fn w34a30_bilin_avg_8(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[i16; 8],
) {
    w34a30_two_tap_avg(src, src_stride, dst, dst_stride, rows, filter, 8);
}

/// `0x004da09d`: 16-wide variant of the same bilinear filter-and-average.
pub fn w34a30_bilin_avg_16(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[i16; 8],
) {
    w34a30_two_tap_avg(src, src_stride, dst, dst_stride, rows, filter, 16);
}

/// `0x004db7be`: 4-wide bilinear filter (taps are words 3 and 4 saturated to
/// signed bytes, pmaddubsw then pmulhrsw by 0x100), clamped, stored without
/// averaging.
pub fn w34a30_bilin_4(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
    filter: &[i16; 8],
) {
    let f0 = filter[3].clamp(-128, 127) as i16;
    let f1 = filter[4].clamp(-128, 127) as i16;
    for r in 0..rows {
        for i in 0..4 {
            let a = src[r * src_stride + i] as i16;
            let b = src[r * src_stride + i + 1] as i16;
            let m = (a * f0).saturating_add(b * f1);
            let v = ((((m as i32) * 256) >> 14) + 1) >> 1;
            dst[r * dst_stride + i] = (v as i16).clamp(0, 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests_wave34_agent30 {
    use super::*;

    const F: [i16; 8] = [0, 0, 0, 64, 64, 0, 0, 0];

    #[test]
    fn avg8() {
        let src: Vec<u8> = (0..9).map(|i| i * 10).collect();
        let mut dst = [100u8; 8];
        w34a30_bilin_avg_8(&src, 9, &mut dst, 8, 1, &F);
        // pixel 0: (0*64+10*64+64)>>7 = 5; avg(5,100)=53
        assert_eq!(dst[0], 53);
    }

    #[test]
    fn avg16_clamps() {
        let src = [255u8; 17];
        let f = [0, 0, 0, 128, 128, 0, 0, 0];
        let mut dst = [0u8; 16];
        w34a30_bilin_avg_16(&src, 17, &mut dst, 16, 1, &f);
        assert!(dst.iter().all(|&d| d == 128));
    }

    #[test]
    fn four_wide() {
        let src = [0u8, 10, 20, 30, 40];
        let mut dst = [0u8; 4];
        w34a30_bilin_4(&src, 5, &mut dst, 4, 1, &F);
        assert_eq!(dst, [5, 15, 25, 35]);
    }
}
