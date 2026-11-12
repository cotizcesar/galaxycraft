#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent08.rs
// ===========================================================================
// Wave 34 translation batch 08 (MMX/SSE2 pixel kernels).

/// `0x004bbe7f`: 6-tap vertical filter producing 8 output bytes per row.
/// Source rows sit at byte offsets `0, s, 2s, 3s, 4s, 5s` from the row start
/// (the retail code advances the base by `s` between loads); taps are
/// `coef[0..8]`, `coef[8..16]`, `coef[16..24]` for rows 0, 1, 2, then
/// `coef[24..32]` for row 3, `coef[32..40]` for row 4, `coef[40..48]` for row 5
/// (row order in the retail code is rows 0,1,2,4 then 3,5).  Products are
/// 16-bit wrapping, accumulation uses signed saturating adds in the retail
/// order (r1+r4, +r2, +r0, +r3, +r5), then `round` (the constant at
/// the retail global) is added, `>> 7` arithmetic, and each word is clamped
/// to `0..=255`.  `rows` counts output rows; `dst_stride` is in bytes.
pub fn w34a08_filter6_rows(
    src: &[u8],
    src_start: usize,
    src_stride: usize,
    dst: &mut [u8],
    dst_start: usize,
    dst_stride: usize,
    rows: usize,
    coef: &[i16; 48],
    round: &[i16; 8],
) {
    let mut s = src_start;
    let mut d = dst_start;
    for _ in 0..rows {
        // Row k of the retail loads: A=0, B=s, C=2s, D=4s, E=3s, F=5s.
        let offs = [
            0usize,
            src_stride,
            2 * src_stride,
            4 * src_stride,
            3 * src_stride,
            5 * src_stride,
        ];
        let cbase = [0usize, 8, 16, 32, 24, 40];
        let mut prod = [[0i16; 8]; 6];
        for r in 0..6 {
            for i in 0..8 {
                let px = src[s + offs[r] + i] as i16;
                prod[r][i] = px.wrapping_mul(coef[cbase[r] + i]);
            }
        }
        for i in 0..8 {
            // prod order: 0=u1(row0) 1=u2 2=u3 3=u4 4=u5 5=u6
            let mut acc = prod[1][i].saturating_add(prod[3][i]);
            acc = acc.saturating_add(prod[2][i]);
            acc = acc.saturating_add(prod[0][i]);
            acc = acc.saturating_add(prod[4][i]);
            acc = acc.saturating_add(prod[5][i]);
            acc = acc.saturating_add(round[i]);
            acc >>= 7;
            dst[d + i] = acc.clamp(0, 255) as u8;
        }
        s += src_stride;
        d += dst_stride;
    }
}

/// `0x004bbf22`: widens bytes to 16-bit words.  Each row reads 16 source
/// bytes and writes 16 `u16` words (32 bytes, little endian) to the
/// destination; `src_stride` and `dst_stride` are byte strides, `rows` is
/// the loop count (retail do-while, so at least one row is written).
pub fn w34a08_widen_u8_to_u16_rows(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
) {
    let mut s = 0usize;
    let mut d = 0usize;
    for _ in 0..rows.max(1) {
        for i in 0..16 {
            let w = src[s + i] as u16;
            dst[d + 2 * i..d + 2 * i + 2].copy_from_slice(&w.to_le_bytes());
        }
        s += src_stride;
        d += dst_stride;
    }
}

#[cfg(test)]
mod tests_wave34_agent08 {
    use super::*;

    #[test]
    fn widen() {
        let src: Vec<u8> = (0..32).collect();
        let mut dst = vec![0u8; 64];
        w34a08_widen_u8_to_u16_rows(&src, 16, &mut dst, 32, 2);
        assert_eq!(dst[0], 0);
        assert_eq!(dst[2], 1);
        assert_eq!(dst[30], 15);
        assert_eq!(dst[32], 16);
        assert_eq!(dst[62], 31);
        assert_eq!(dst[1], 0);
    }

    #[test]
    fn filter_identity_and_clamp() {
        let src = vec![100u8; 8 * 8];
        let mut dst = vec![0u8; 8];
        let mut coef = [0i16; 48];
        for i in 0..8 {
            coef[i] = 128; // row 0 tap = 1.0 in Q7
        }
        let round = [64i16; 8];
        w34a08_filter6_rows(&src, 0, 8, &mut dst, 0, 8, 1, &coef, &round);
        assert_eq!(dst, vec![100u8; 8]);
        coef[8] = 300;
        w34a08_filter6_rows(&src, 0, 8, &mut dst, 0, 8, 1, &coef, &round);
        assert_eq!(dst[0], 255);
        let neg = [-32768i16; 8];
        let coef0 = [0i16; 48];
        w34a08_filter6_rows(&src, 0, 8, &mut dst, 0, 8, 1, &coef0, &neg);
        assert_eq!(dst[0], 0);
    }
}
