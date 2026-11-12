#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent16.rs
// ===========================================================================
// Wave 34 agent 16: libvpx-style block copy, DC-only add and 16-lane multiply.

/// `0x004bd87a`: copies a 16x16 byte block (16 rows of 16 bytes) from `src`
/// (row stride `src_stride`) to `dst` (row stride `dst_stride`).
pub fn w34a16_copy_16x16(src: &[u8], src_stride: usize, dst: &mut [u8], dst_stride: usize) {
    for r in 0..16 {
        let s = r * src_stride;
        let d = r * dst_stride;
        dst[d..d + 16].copy_from_slice(&src[s..s + 16]);
    }
}

/// `0x004bdc33`: DC-only inverse transform add.  `dc` is the low 16 bits of
/// the coefficient; `(dc + 4) >> 3` (arithmetic, 16-bit wrap on the add) is
/// added with saturation to a 4x4 predictor block and clamped to 0..=255.
pub fn w34a16_dc_only_add_4x4(
    dc: i32,
    pred: &[u8],
    pred_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
) {
    let v = ((dc as i16).wrapping_add(4) >> 3) as i16;
    for r in 0..4 {
        for c in 0..4 {
            let p = pred[r * pred_stride + c] as i16;
            let s = p.saturating_add(v);
            dst[r * dst_stride + c] = s.clamp(0, 255) as u8;
        }
    }
}

/// `0x004bdcb0`: element-wise 16-bit multiply of two 16-lane vectors,
/// keeping the low 16 bits of each product.
pub fn w34a16_mul_lo_i16x16(a: &[i16; 16], b: &[i16; 16]) -> [i16; 16] {
    let mut out = [0i16; 16];
    for i in 0..16 {
        out[i] = a[i].wrapping_mul(b[i]);
    }
    out
}

#[cfg(test)]
mod tests_wave34_agent16 {
    use super::*;

    #[test]
    fn copy_block() {
        let src: Vec<u8> = (0..16 * 20).map(|i| (i % 251) as u8).collect();
        let mut dst = vec![0u8; 16 * 18];
        w34a16_copy_16x16(&src, 20, &mut dst, 18);
        assert_eq!(&dst[18 * 15..18 * 15 + 16], &src[20 * 15..20 * 15 + 16]);
        assert_eq!(dst[16], 0);
    }

    #[test]
    fn dc_add() {
        let mut pred = [0u8; 16];
        pred[0] = 250;
        pred[1] = 5;
        pred[2] = 100;
        let mut dst = [0u8; 16];
        w34a16_dc_only_add_4x4(60, &pred, 4, &mut dst, 4); // (60+4)>>3 = 8
        assert_eq!(dst[0], 255);
        assert_eq!(dst[1], 13);
        assert_eq!(dst[2], 108);
        w34a16_dc_only_add_4x4(-100, &pred, 4, &mut dst, 4); // (-96)>>3 = -12
        assert_eq!(dst[0], 238);
        assert_eq!(dst[1], 0);
    }

    #[test]
    fn mul_lo() {
        let mut a = [0i16; 16];
        let mut b = [0i16; 16];
        a[0] = 300;
        b[0] = 300;
        a[15] = -3;
        b[15] = 7;
        let r = w34a16_mul_lo_i16x16(&a, &b);
        assert_eq!(r[0], 90000u32 as u16 as i16);
        assert_eq!(r[15], -21);
    }
}
