#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent25.rs
// ===========================================================================
// Wave 34 agent 25 (queue, `runtime.rs`).

/// `0x004d7af0`: dequantised DC-only inverse transform for two side-by-side
/// 4x4 blocks (8 pixels wide, 4 rows).  `coeffs[0]*dq[0]` and
/// `coeffs[16]*dq[0]` are the two DCs (16-bit wrapping multiply; the second
/// block's DC is `coeffs[16]`), the low-block DC also uses `coeffs[1]*dq[1]`
/// only as a discarded lane.  Each DC becomes `(dc + 4) >> 3` and is added
/// to the left (pixels 0..4) or right (pixels 4..8) block, saturating to
/// 0..=255.  Both DC slots (`coeffs[0..2]` and `coeffs[16..18]`) are cleared.
pub fn w34a25_dc_only_idct_add_pair(coeffs: &mut [i16], dq: &[i16], dst: &mut [u8], stride: usize) {
    let dc0 = coeffs[0].wrapping_mul(dq[0]);
    let dc1 = coeffs[16].wrapping_mul(dq[0]);
    coeffs[0] = 0;
    coeffs[1] = 0;
    coeffs[16] = 0;
    coeffs[17] = 0;
    let a = dc0.wrapping_add(4) >> 3;
    let b = dc1.wrapping_add(4) >> 3;
    for row in 0..4 {
        for x in 0..8 {
            let d = if x < 4 { a } else { b };
            let p = &mut dst[row * stride + x];
            *p = (*p as i16).wrapping_add(d).clamp(0, 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests_wave34_agent25 {
    use super::*;

    #[test]
    fn dc_pair() {
        let mut c = [0i16; 32];
        c[0] = 8;
        c[16] = -100;
        let mut d = vec![10u8; 8 * 4];
        w34a25_dc_only_idct_add_pair(&mut c, &[2, 1], &mut d, 8);
        assert_eq!(d[0], 12);
        assert_eq!(d[7], 0);
        assert_eq!(c[0], 0);
        assert_eq!(c[16], 0);
    }
}
