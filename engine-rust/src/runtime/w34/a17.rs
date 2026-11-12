#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent17.rs
// ===========================================================================
// Wave 34 agent 17: MMX/SSE2 pixel kernels (4x4 IDCT-add, bilinear blends).

fn w34a17_mulhi(a: i16, k: i16) -> i16 {
    ((a as i32 * k as i32) >> 16) as i16
}

/// `0x004bdcef`: 4x4 inverse transform with per-coefficient multiply
/// (`coef[i] * dq[i]`, 16-bit wrapping), added with signed saturation to
/// the 4x4 byte block at `dst[base..]` (row stride `stride`) and clamped to
/// 0..=255.  The coefficient block is zeroed afterwards.
pub fn w34a17_idct4x4_add(
    coef: &mut [i16; 16],
    dq: &[i16; 16],
    dst: &mut [u8],
    base: usize,
    stride: usize,
) {
    const S: i16 = 0x8a8cu16 as i16;
    const K: i16 = 0x4e7b;
    let mut p = [[0i16; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            p[r][c] = coef[r * 4 + c].wrapping_mul(dq[r * 4 + c]);
        }
    }
    coef.fill(0);
    // pass 1 over columns: x[r][c]
    let mut x = [[0i16; 4]; 4];
    for c in 0..4 {
        let (p0, p1, p2, p3) = (p[0][c], p[1][c], p[2][c], p[3][c]);
        let a = p0.wrapping_add(p2);
        let b = p0.wrapping_sub(p2);
        let cn = w34a17_mulhi(p3, K)
            .wrapping_add(p3)
            .wrapping_sub(w34a17_mulhi(p1, S).wrapping_add(p1));
        let d = w34a17_mulhi(p3, S)
            .wrapping_add(p3)
            .wrapping_add(w34a17_mulhi(p1, K))
            .wrapping_add(p1);
        x[0][c] = a.wrapping_add(d);
        x[1][c] = b.wrapping_sub(cn);
        x[2][c] = b.wrapping_add(cn);
        x[3][c] = a.wrapping_sub(d);
    }
    for r in 0..4 {
        let (x0, x1, x2, x3) = (x[r][0], x[r][1], x[r][2], x[r][3]);
        let a = x0.wrapping_add(x2).wrapping_add(4);
        let b = x0.wrapping_sub(x2).wrapping_add(4);
        let cn = w34a17_mulhi(x3, K)
            .wrapping_add(x3)
            .wrapping_sub(w34a17_mulhi(x1, S).wrapping_add(x1));
        let d = w34a17_mulhi(x3, S)
            .wrapping_add(x3)
            .wrapping_add(w34a17_mulhi(x1, K))
            .wrapping_add(x1);
        let out = [
            a.wrapping_add(d) >> 3,
            b.wrapping_sub(cn) >> 3,
            b.wrapping_add(cn) >> 3,
            a.wrapping_sub(d) >> 3,
        ];
        for (k, o) in out.iter().enumerate() {
            let i = base + r * stride + k;
            let s = o.saturating_add(dst[i] as i16);
            dst[i] = s.clamp(0, 255) as u8;
        }
    }
}

fn w34a17_blend(
    src: &[u8],
    sstride: usize,
    dst: &mut [u8],
    dstride: usize,
    w: u32,
    wd: usize,
    ht: usize,
) {
    let w = (w & 0xffff) as u16;
    let iw = 0x10u16.wrapping_sub(w);
    for r in 0..ht {
        for c in 0..wd {
            let a = src[r * sstride + c] as u16;
            let b = dst[r * dstride + c] as u16;
            let v = a
                .wrapping_mul(w)
                .wrapping_add(b.wrapping_mul(iw))
                .wrapping_add(8)
                >> 4;
            dst[r * dstride + c] = v.min(255) as u8;
        }
    }
}

/// `0x004bde90`: blends a 16x16 source block into the destination block:
/// `dst = min(255, (src*w + dst*(16-w) + 8) >> 4)` with 16-bit arithmetic,
/// `w` the low 16 bits of the weight argument.
pub fn w34a17_blend16x16(src: &[u8], sstride: usize, dst: &mut [u8], dstride: usize, w: u32) {
    w34a17_blend(src, sstride, dst, dstride, w, 16, 16);
}

/// `0x004bdf29`: the 8x8 variant of the blend above.
pub fn w34a17_blend8x8(src: &[u8], sstride: usize, dst: &mut [u8], dstride: usize, w: u32) {
    w34a17_blend(src, sstride, dst, dstride, w, 8, 8);
}

#[cfg(test)]
mod tests_wave34_agent17 {
    use super::*;

    #[test]
    fn dc_idct() {
        let mut c = [0i16; 16];
        c[0] = 64;
        let dq = [1i16; 16];
        let mut d = [250u8; 16];
        d[0] = 10;
        w34a17_idct4x4_add(&mut c, &dq, &mut d, 0, 4);
        assert_eq!(d[0], 18);
        assert_eq!(d[1], 255);
        assert_eq!(c, [0; 16]);
    }

    #[test]
    fn blends() {
        let s = [100u8; 256];
        let mut d = [20u8; 256];
        w34a17_blend16x16(&s, 16, &mut d, 16, 8);
        assert_eq!(d[0], 60);
        let mut d = [20u8; 64];
        w34a17_blend8x8(&s, 8, &mut d, 8, 16);
        assert_eq!(d[5], 100);
    }
}
