#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent08.rs
// ===========================================================================
// Wave 28 translation batch 08.
//
// libvpx-style variance tails, IDCT add helpers and an 8-wide loop filter.
// Unresolved callees (sum/SSE kernels, 1-D 8-point transform) are closures.

/// Variance tail with a signed 32-bit square: `sse - ((sum * sum) >> shift)`.
pub fn w28a08_var_sub_i32(sse: i32, sum: i32, shift: u32) -> i32 {
    sse.wrapping_sub(sum.wrapping_mul(sum) >> shift)
}

/// Variance tail with an unsigned 32-bit square.
pub fn w28a08_var_sub_u32(sse: i32, sum: i32, shift: u32) -> i32 {
    sse.wrapping_sub(((sum.wrapping_mul(sum) as u32) >> shift) as i32)
}

/// Variance tail with a 64-bit square, truncated to 32 bits after the shift.
pub fn w28a08_var_sub_i64(sse: i32, sum: i32, shift: u32) -> i32 {
    let p = (sum as i64) * (sum as i64);
    sse.wrapping_sub((p >> shift) as i32)
}

/// `0x00a381b0`: kernel returns the sum and writes sse; result is
/// `sse - (sum*sum >> 5)`.  Returns `(variance, sse)`.
pub fn w28a08_var_a381b0(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_i32(sse, sum, 5), sse)
}

/// `0x00a381f0`: shift 6, signed square; kernel returns `(sum, sse)`.
pub fn w28a08_var_a381f0(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_i32(sse, sum, 6), sse)
}

/// `0x00a38230`: shift 6, signed square; kernel returns `(sum, sse)`.
pub fn w28a08_var_a38230(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_i32(sse, sum, 6), sse)
}

/// `0x00a38270`: kernel yields `(sum, sse)`; unsigned square shift 8.
pub fn w28a08_var_a38270(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_u32(sse, sum, 8), sse)
}

/// `0x00a382f0`: signed square shift 7.
pub fn w28a08_var_a382f0(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_i32(sse, sum, 7), sse)
}

/// `0x00a38330`: 64-bit square shift 9 (blocks 32x16).
pub fn w28a08_var_a38330(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_i64(sse, sum, 9), sse)
}

/// `0x00a38370`: 64-bit square shift 10 (32x32).
pub fn w28a08_var_a38370(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_i64(sse, sum, 10), sse)
}

/// `0x00a383b0`: 64-bit square shift 11 (32x64).
pub fn w28a08_var_a383b0(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_i64(sse, sum, 11), sse)
}

/// `0x00a383f0`: signed square shift 4.
pub fn w28a08_var_a383f0(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_i32(sse, sum, 4), sse)
}

/// `0x00a38470`: 64-bit square shift 11 (64x32).
pub fn w28a08_var_a38470(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_i64(sse, sum, 11), sse)
}

/// `0x00a384b0`: 64-bit square shift 12 (64x64).
pub fn w28a08_var_a384b0(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_i64(sse, sum, 12), sse)
}

/// `0x00a384f0`: signed square shift 7.
pub fn w28a08_var_a384f0(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_i32(sse, sum, 7), sse)
}

/// `0x00a38530`: signed square shift 5.
pub fn w28a08_var_a38530(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_i32(sse, sum, 5), sse)
}

/// `0x00a38570`: signed square shift 6.
pub fn w28a08_var_a38570(kernel: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = kernel();
    (w28a08_var_sub_i32(sse, sum, 6), sse)
}

/// Saturate to `0..=255` (the `< 0x100` / `< 0` ladder of the retail code).
fn w28a08_clamp_u8(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// `0x00a3b560`: 8x8 inverse transform added to `dst` (row stride `stride`,
/// block at `base`).  `tx` is the 1-D 8-point transform (callee a3a490).
/// Pass 1 transforms each coefficient row, pass 2 each column; every output is
/// `((v + 0x10) >> 5)` added to the pixel with saturation.
pub fn w28a08_idct8_add(
    coeffs: &[i16; 64],
    dst: &mut [u8],
    base: usize,
    stride: usize,
    mut tx: impl FnMut(&[i16; 8]) -> [i16; 8],
) {
    let mut tmp = [[0i16; 8]; 8];
    for (r, row) in tmp.iter_mut().enumerate() {
        let mut inp = [0i16; 8];
        inp.copy_from_slice(&coeffs[r * 8..r * 8 + 8]);
        *row = tx(&inp);
    }
    for c in 0..8 {
        let mut col = [0i16; 8];
        for (r, v) in col.iter_mut().enumerate() {
            *v = tmp[r][c];
        }
        let out = tx(&col);
        for (r, &o) in out.iter().enumerate() {
            let idx = base + r * stride + c;
            let v = ((o as i32 + 0x10) >> 5) + dst[idx] as i32;
            dst[idx] = w28a08_clamp_u8(v);
        }
    }
}

/// `0x00a3ba10`: 4x4 add of a single DC-style coefficient with the odd
/// per-column split (`dc>>2 - dc>>3` for column 0, `dc>>3` for the others);
/// the first row gets `v - (v>>1) + v`, the
/// other rows `v>>1`.
pub fn w28a08_dc_add_4x4_split(dc: i16, dst: &mut [u8], base: usize, stride: usize) {
    let t3 = dc >> 3;
    let t = [(dc >> 2).wrapping_sub(t3), t3, t3, t3];
    for (i, &v) in t.iter().enumerate() {
        let v = v as i32;
        let h = v >> 1;
        let i0 = base + i;
        dst[i0] = w28a08_clamp_u8(dst[i0] as i32 - h + v);
        for r in 1..4 {
            let idx = i0 + r * stride;
            dst[idx] = w28a08_clamp_u8(dst[idx] as i32 + h);
        }
    }
}

/// `0x00a48e50`: DC-only 4x4 inverse transform add.  The DC passes through two
/// `* 0x2d41 + 0x2000 >> 14` stages, `+8 >> 4`, truncated to i16, then added to
/// all 16 pixels with saturation (16-bit wrapping add first).
pub fn w28a08_dc_only_idct_add_4x4(dc: i16, dst: &mut [u8], base: usize, stride: usize) {
    let a = (dc as i32 * 0x2d41 + 0x2000) >> 14;
    let b = (a * 0x2d41 + 0x2000) >> 14;
    let dc16 = ((b + 8) >> 4) as i16;
    for r in 0..4 {
        for c in 0..4 {
            let idx = base + r * stride + c;
            let s = (dst[idx] as i16).wrapping_add(dc16);
            dst[idx] = w28a08_clamp_u8(s as i32);
        }
    }
}

fn w28a08_sat_i8(v: i32) -> i32 {
    v.clamp(-128, 127)
}

/// `0x00a4aaf0`: VP8-style normal loop filter across a horizontal edge for 8
/// columns.  `s` is the index of the first q0 pixel (needs 4 rows above and 3
/// below); `blimit`, `limit` and `thresh` are the scalar bytes the retail code
/// reads (without advancing) from its three pointers.
pub fn w28a08_loop_filter_h8(
    buf: &mut [u8],
    s: usize,
    pitch: usize,
    blimit: u8,
    limit: u8,
    thresh: u8,
) {
    let (blimit, limit, thresh) = (blimit as i32, limit as i32, thresh as i32);
    for i in 0..8 {
        let q0i = s + i;
        let p0i = q0i - pitch;
        let p1i = q0i - 2 * pitch;
        let q1i = q0i + pitch;
        let g = |ix: usize| buf[ix] as i32;
        let (p3, p2, p1, p0) = (g(q0i - 4 * pitch), g(q0i - 3 * pitch), g(p1i), g(p0i));
        let (q0, q1, q2, q3) = (g(q0i), g(q1i), g(q0i + 2 * pitch), g(q0i + 3 * pitch));
        let mut mask = false;
        mask |= (p1 - q1).abs() / 2 + (p0 - q0).abs() * 2 > blimit;
        mask |= (p3 - p2).abs() > limit;
        mask |= (q3 - q2).abs() > limit;
        mask |= (q2 - q1).abs() > limit;
        mask |= (q1 - q0).abs() > limit;
        mask |= (p1 - p0).abs() > limit;
        mask |= (p2 - p1).abs() > limit;
        let hev = (q1 - q0).abs() > thresh || (p1 - p0).abs() > thresh;
        let sg = |v: i32| v - 128;
        let (ps1, ps0, qs0, qs1) = (sg(p1), sg(p0), sg(q0), sg(q1));
        let mut f = w28a08_sat_i8(ps1 - qs1);
        if !hev {
            f = 0;
        }
        f = w28a08_sat_i8(f + (qs0 - ps0) * 3);
        if mask {
            f = 0;
        }
        let f1 = w28a08_sat_i8(f + 4) >> 3;
        let f2 = w28a08_sat_i8(f + 3) >> 3;
        buf[q0i] = (w28a08_sat_i8(qs0 - f1) + 128) as u8;
        buf[p0i] = (w28a08_sat_i8(f2 + ps0) + 128) as u8;
        let f3 = if hev { 0 } else { (f1 + 1) >> 1 };
        buf[q1i] = (w28a08_sat_i8(qs1 - f3) + 128) as u8;
        buf[p1i] = (w28a08_sat_i8(ps1 + f3) + 128) as u8;
    }
}

#[cfg(test)]
mod tests_wave28_agent08 {
    use super::*;

    #[test]
    fn variance_tails() {
        assert_eq!(w28a08_var_a381b0(|| (64, 1000)), (1000 - 128, 1000));
        assert_eq!(w28a08_var_a381f0(|| (-64, 500)), (500 - 64, 500));
        assert_eq!(w28a08_var_a38230(|| (8, 10)), (9, 10));
        assert_eq!(w28a08_var_a38270(|| (16, 300)).0, 299);
        assert_eq!(w28a08_var_a382f0(|| (16, 300)).0, 298);
        assert_eq!(w28a08_var_a383f0(|| (4, 20)).0, 19);
        assert_eq!(w28a08_var_a384f0(|| (16, 300)).0, 298);
        assert_eq!(w28a08_var_a38530(|| (8, 100)).0, 98);
        assert_eq!(w28a08_var_a38570(|| (8, 100)).0, 99);
        assert_eq!(w28a08_var_a38330(|| (1024, 0)).0, -(1 << 11));
        assert_eq!(w28a08_var_a38370(|| (1024, 0)).0, -(1 << 10));
        assert_eq!(w28a08_var_a383b0(|| (1024, 0)).0, -(1 << 9));
        assert_eq!(w28a08_var_a38470(|| (1024, 0)).0, -(1 << 9));
        assert_eq!(w28a08_var_a384b0(|| (1024, 0)).0, -(1 << 8));
        // 64-bit path keeps the high bits that the u32 path would lose
        assert_eq!(w28a08_var_sub_i64(0, 1 << 17, 12), -(1 << 22));
        assert_eq!(w28a08_var_sub_u32(0, 1 << 16, 8), 0);
    }

    #[test]
    fn idct8_identity_add() {
        let mut c = [0i16; 64];
        c[0] = 64;
        let mut dst = vec![100u8; 64];
        // pass-through transform: output = input
        w28a08_idct8_add(&c, &mut dst, 0, 8, |v| *v);
        // only tmp[0][0] = 64 -> (64+16)>>5 = 2; others (16>>5)=0
        assert_eq!(dst[0], 102);
        assert_eq!(dst[1], 100);
        let mut sat = vec![250u8; 64];
        c[0] = 32000;
        w28a08_idct8_add(&c, &mut sat, 0, 8, |v| *v);
        assert_eq!(sat[0], 255);
        c[0] = -32000;
        w28a08_idct8_add(&c, &mut sat, 0, 8, |v| *v);
        assert_eq!(sat[0], 0);
    }

    #[test]
    fn dc_add_split() {
        let mut d = vec![50u8; 16];
        w28a08_dc_add_4x4_split(64, &mut d, 0, 4);
        // t = [8, 8,8,8]; row0: 50 - 4 + 8 = 54, other rows +4
        assert_eq!(d[0], 54);
        assert_eq!(d[4], 54);
        assert_eq!(d[12], 54);
        let mut e = vec![255u8; 16];
        w28a08_dc_add_4x4_split(800, &mut e, 0, 4);
        assert!(e.iter().all(|&x| x == 255));
    }

    #[test]
    fn dc_only_add() {
        let mut d = vec![10u8; 16];
        w28a08_dc_only_idct_add_4x4(128, &mut d, 0, 4);
        // ((128*0x2d41+0x2000)>>14)=91; (91*0x2d41+0x2000)>>14=64; (64+8)>>4=4
        assert!(d.iter().all(|&x| x == 14));
        let mut e = vec![3u8; 16];
        w28a08_dc_only_idct_add_4x4(-128, &mut e, 0, 4);
        assert!(e.iter().all(|&x| x == 0));
    }

    #[test]
    fn loop_filter_flat_and_step() {
        let pitch = 8;
        let mut flat = vec![100u8; 8 * pitch];
        w28a08_loop_filter_h8(&mut flat, 4 * pitch, pitch, 40, 10, 5);
        assert!(flat.iter().all(|&x| x == 100));
        // small step across the edge gets smoothed
        let mut b = vec![100u8; 8 * pitch];
        for v in b[4 * pitch..].iter_mut() {
            *v = 108;
        }
        w28a08_loop_filter_h8(&mut b, 4 * pitch, pitch, 255, 255, 255);
        assert!(b[3 * pitch] > 100 && b[4 * pitch] < 108);
        // mask off (limit 0, large step) leaves data alone
        let mut c = vec![100u8; 8 * pitch];
        for v in c[4 * pitch..].iter_mut() {
            *v = 160;
        }
        let orig = c.clone();
        w28a08_loop_filter_h8(&mut c, 4 * pitch, pitch, 10, 10, 5);
        assert_eq!(c, orig);
    }
}
