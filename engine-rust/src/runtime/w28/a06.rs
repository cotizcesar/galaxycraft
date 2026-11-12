#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent06.rs
// ===========================================================================
// Wave 28 translation batch 06: H.264-style intra predictors and dispatch
// helpers from the video decoder (module target `runtime.rs`).
//
// Skipped (SIMD / noreturn tails / bad decompile): FUN_00a1dee0,
// FUN_00a1e010, FUN_00a1f420, FUN_00a20940, FUN_00a20bb0, FUN_00a20eb0,
// FUN_00a21090.

/// Callee selector for the block-filter dispatchers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W28a06Kind {
    /// Coefficient triple is zero, 16-wide step.
    Zero16,
    /// Coefficient triple is zero, 8-wide tail.
    Zero8,
    /// Coefficient triple is zero, 4-wide tail.
    Zero4,
    /// Non-zero coefficients, 16-wide step (source pointer biased by -3 rows).
    Nz16,
    /// Non-zero coefficients, 8-wide tail.
    Nz8,
    /// Non-zero coefficients, 4-wide tail.
    Nz4,
}

fn w28a06_dispatch<F: FnMut(W28a06Kind, [i32; 5])>(
    args: [i32; 5],
    coefs: [i16; 3],
    width: i32,
    f: &mut F,
) {
    let [p1, p2, p3, p4, p10] = args;
    let mut p1 = p1;
    let mut p3 = p3;
    let zero = coefs == [0, 0, 0];
    let mut rem = width;
    if zero {
        if width > 15 {
            for _ in 0..(width >> 4) {
                f(W28a06Kind::Zero16, [p1, p2, p3, p4, p10]);
                p1 = p1.wrapping_add(16);
                p3 = p3.wrapping_add(16);
            }
            rem = width - (width >> 4) * 16;
        }
        if rem == 8 {
            f(W28a06Kind::Zero8, [p1, p2, p3, p4, p10]);
        } else if rem == 4 {
            f(W28a06Kind::Zero4, [p1, p2, p3, p4, p10]);
        }
    } else {
        let bias = p2.wrapping_mul(-3);
        let mut it = p1.wrapping_add(bias);
        if width > 15 {
            let n = width >> 4;
            rem = width - n * 16;
            p1 = p1.wrapping_add(n * 16);
            for _ in 0..n {
                f(W28a06Kind::Nz16, [it, p2, p3, p4, p10]);
                it = it.wrapping_add(16);
                p3 = p3.wrapping_add(16);
            }
        }
        let base = p1.wrapping_add(bias);
        if rem == 8 {
            f(W28a06Kind::Nz8, [base, p2, p3, p4, p10]);
        } else if rem == 4 {
            f(W28a06Kind::Nz4, [base, p2, p3, p4, p10]);
        }
    }
}

/// `0x00a1e470`: splits `width` into 16-wide steps plus an 8/4 tail and calls
/// the matching worker (zero-coefficient workers when the three coefficient
/// shorts are all zero, otherwise the -3-row-biased ones).  Workers are
/// modelled by `f(kind, [src, stride, dst, p4, p10])`.
pub fn w28a06_filter_dispatch_a<F: FnMut(W28a06Kind, [i32; 5])>(
    args: [i32; 5],
    coefs: [i16; 3],
    width: i32,
    mut f: F,
) {
    w28a06_dispatch(args, coefs, width, &mut f);
}

/// `0x00a1e810`: same control flow as the sibling at 00a1e470 with a different
/// worker set.
pub fn w28a06_filter_dispatch_b<F: FnMut(W28a06Kind, [i32; 5])>(
    args: [i32; 5],
    coefs: [i16; 3],
    width: i32,
    mut f: F,
) {
    w28a06_dispatch(args, coefs, width, &mut f);
}

/// `0x00a1e970`: splits two packed values into `(v & !0xff, (v & 0xff) >> 4)`
/// pairs and returns the 12 arguments forwarded to the callee (FUN_00a1d850).
#[allow(clippy::too_many_arguments)]
pub fn w28a06_split_args(
    p1: u32,
    p2: u32,
    p3: u32,
    p4: u32,
    p5: u32,
    p6: u32,
    p7: u32,
    p8: u32,
    p9: u32,
    p10: u32,
) -> [u32; 12] {
    let a = p5 & 0xffff_ff00;
    let b = p7 & 0xffff_ff00;
    [
        p1,
        p2,
        p3,
        p4,
        a,
        ((p5.wrapping_sub(a)) as i32 >> 4) as u32,
        p6,
        b,
        ((p7.wrapping_sub(b)) as i32 >> 4) as u32,
        p8,
        p9,
        p10,
    ]
}

/// `0x00a20ad0`: 4x4 diagonal-down-left style intra predictor from 8 edge
/// samples (last pixel copied from edge sample 7).
pub fn w28a06_pred4x4_diag(dst: &mut [u8], stride: usize, e: &[u8; 8]) {
    let p: [u32; 8] = e.map(u32::from);
    let t = |a: usize, b: usize, c: usize| ((p[a] + 2 * p[b] + p[c] + 2) >> 2) as u8;
    let v = [
        t(0, 1, 2),
        t(1, 2, 3),
        t(2, 3, 4),
        t(3, 4, 5),
        t(4, 5, 6),
        ((p[7] + p[5] + 2 * p[6] + 2) >> 2) as u8,
    ];
    for y in 0..4 {
        for x in 0..4 {
            let k = x + y;
            dst[y * stride + x] = if k <= 5 { v[k] } else { e[7] };
        }
    }
}

/// `0x00a20fa0`: 4x4 vertical-left style predictor (alternating 2-tap and
/// 3-tap filters of 7 edge samples).
pub fn w28a06_pred4x4_vert_left(dst: &mut [u8], stride: usize, e: &[u8; 7]) {
    let p: [u32; 7] = e.map(u32::from);
    let avg = |a: usize, b: usize| ((p[a] + p[b] + 1) >> 1) as u8;
    let f3 = |a: usize, b: usize, c: usize| ((p[a] + 2 * p[b] + p[c] + 2) >> 2) as u8;
    let s = stride;
    dst[0] = avg(0, 1);
    dst[2 * s] = avg(1, 2);
    dst[1] = avg(1, 2);
    dst[2 * s + 1] = avg(3, 2);
    dst[2] = avg(3, 2);
    dst[2 * s + 2] = avg(3, 4);
    dst[3] = avg(3, 4);
    dst[2 * s + 3] = avg(5, 4);
    dst[s] = f3(0, 1, 2);
    dst[3 * s] = f3(1, 2, 3);
    dst[s + 1] = f3(1, 2, 3);
    dst[3 * s + 1] = f3(2, 3, 4);
    dst[s + 2] = f3(2, 3, 4);
    dst[3 * s + 2] = f3(3, 4, 5);
    dst[s + 3] = f3(3, 4, 5);
    dst[3 * s + 3] = f3(4, 5, 6);
}

/// `0x00a21420`: fills an 8x8 block (8 rows of 8 bytes) with 0x80.
pub fn w28a06_fill_8x8_grey(dst: &mut [u8], stride: usize) {
    for y in 0..8 {
        dst[y * stride..y * stride + 8].fill(0x80);
    }
}

/// `0x00a21840`: chroma 4x4 DC from four top and four left samples,
/// `(sum + 4) >> 3`, replicated over a 4x4 block.
pub fn w28a06_dc4x4_both(dst: &mut [u8], stride: usize, top: &[u8; 4], left: &[u8; 4]) {
    let sum: u32 = top.iter().chain(left.iter()).map(|&b| u32::from(b)).sum();
    let v = ((sum + 4) >> 3) as u8;
    for y in 0..4 {
        dst[y * stride..y * stride + 4].fill(v);
    }
}

/// `0x00a21950`: 16x16 DC from 16 samples, `(sum + 8) >> 4`.
pub fn w28a06_dc16x16_one(dst: &mut [u8], stride: usize, edge: &[u8; 16]) {
    let sum: i32 = edge.iter().map(|&b| i32::from(b)).sum::<i32>() + 8;
    let v = ((sum + ((sum >> 31) & 0xf)) >> 4) as u8;
    for y in 0..16 {
        dst[y * stride..y * stride + 16].fill(v);
    }
}

/// `0x00a21680`: 16x16 DC from 16 top and 16 left samples, `(sum + 16) >> 5`.
pub fn w28a06_dc16x16_both(dst: &mut [u8], stride: usize, left: &[u8; 16], top: &[u8; 16]) {
    let sum: i32 = left
        .iter()
        .chain(top.iter())
        .map(|&b| i32::from(b))
        .sum::<i32>()
        + 16;
    let v = ((sum + ((sum >> 31) & 0x1f)) >> 5) as u8;
    for y in 0..16 {
        dst[y * stride..y * stride + 16].fill(v);
    }
}

/// `0x00a21b80`: 16x16 horizontal prediction, each row filled with `left[y]`.
pub fn w28a06_pred16x16_horizontal(dst: &mut [u8], stride: usize, left: &[u8; 16]) {
    for (y, &l) in left.iter().enumerate() {
        dst[y * stride..y * stride + 16].fill(l);
    }
}

/// `0x00a21c00`: 4x4 horizontal prediction, each row filled with `left[y]`.
pub fn w28a06_pred4x4_horizontal(dst: &mut [u8], stride: usize, left: &[u8; 4]) {
    for (y, &l) in left.iter().enumerate() {
        dst[y * stride..y * stride + 4].fill(l);
    }
}

/// `0x00a21ed0`: 8x8 plane/gradient predictor
/// `clamp(top[x] - corner + left[y], 0, 255)`.
pub fn w28a06_pred8x8_gradient(
    dst: &mut [u8],
    stride: usize,
    corner: u8,
    top: &[u8; 8],
    left: &[u8; 8],
) {
    for y in 0..8 {
        for x in 0..8 {
            let v = i32::from(top[x]) - i32::from(corner) + i32::from(left[y]);
            dst[y * stride + x] = v.clamp(0, 255) as u8;
        }
    }
}

/// `0x00a22180`: sums a 16x16 block of i16 (row stride `stride` shorts) and
/// returns `sum >> 1` truncated to i16.
pub fn w28a06_sum16x16_half(block: &[i16], stride: usize) -> i16 {
    let mut sum: i32 = 0;
    for y in 0..16 {
        for x in 0..16 {
            sum = sum.wrapping_add(i32::from(block[y * stride + x]));
        }
    }
    (sum >> 1) as i16
}

#[cfg(test)]
mod tests_wave28_agent06 {
    use super::*;

    #[test]
    fn dispatch_zero_and_nz() {
        let mut calls = vec![];
        w28a06_filter_dispatch_a([100, 10, 0, 1, 2], [0, 0, 0], 36, |k, a| calls.push((k, a)));
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[1], (W28a06Kind::Zero16, [116, 10, 16, 1, 2]));
        assert_eq!(calls[2].0, W28a06Kind::Zero4);
        let mut calls = vec![];
        w28a06_filter_dispatch_b([100, 10, 0, 1, 2], [0, 1, 0], 24, |k, a| calls.push((k, a)));
        assert_eq!(calls[0], (W28a06Kind::Nz16, [70, 10, 0, 1, 2]));
        assert_eq!(calls[1], (W28a06Kind::Nz8, [100 + 16 - 30, 10, 16, 1, 2]));
        let mut n = 0;
        w28a06_filter_dispatch_a([0; 5], [0; 3], 6, |_, _| n += 1);
        assert_eq!(n, 0);
    }

    #[test]
    fn split() {
        let r = w28a06_split_args(1, 2, 3, 4, 0x1234, 6, 0x2ff, 8, 9, 10);
        assert_eq!(r[4], 0x1200);
        assert_eq!(r[5], 3);
        assert_eq!(r[7], 0x200);
        assert_eq!(r[8], 0xf);
    }

    #[test]
    fn diag() {
        let mut d = [0u8; 16];
        w28a06_pred4x4_diag(&mut d, 4, &[10, 10, 10, 10, 10, 10, 10, 50]);
        assert_eq!(d[0], 10);
        assert_eq!(d[15], 50);
        assert_eq!(d[3], d[6]);
        assert_eq!(d[12], d[3]);
    }

    #[test]
    fn vert_left() {
        let mut d = [0u8; 16];
        w28a06_pred4x4_vert_left(&mut d, 4, &[0, 4, 8, 12, 16, 20, 24]);
        assert_eq!(d[0], 2);
        assert_eq!(d[4], 4);
        assert_eq!(d[8], 6);
        assert_eq!(d[15], 20);
    }

    #[test]
    fn fills_and_dc() {
        let mut d = [0u8; 64];
        w28a06_fill_8x8_grey(&mut d, 8);
        assert!(d.iter().all(|&b| b == 0x80));
        let mut d = [0u8; 16];
        w28a06_dc4x4_both(&mut d, 4, &[1, 2, 3, 4], &[5, 6, 7, 8]);
        assert!(d.iter().all(|&b| b == 5));
        let mut d = [0u8; 256];
        w28a06_dc16x16_one(&mut d, 16, &[255; 16]);
        assert!(d.iter().all(|&b| b == 255));
        w28a06_dc16x16_both(&mut d, 16, &[10; 16], &[20; 16]);
        assert!(d.iter().all(|&b| b == 15));
    }

    #[test]
    fn horizontal() {
        let mut d = [0u8; 256];
        let l: [u8; 16] = core::array::from_fn(|i| i as u8);
        w28a06_pred16x16_horizontal(&mut d, 16, &l);
        assert_eq!(d[16 * 5 + 15], 5);
        let mut d = [0u8; 16];
        w28a06_pred4x4_horizontal(&mut d, 4, &[1, 2, 3, 4]);
        assert_eq!(d[8], 3);
    }

    #[test]
    fn gradient_and_sum() {
        let mut d = [0u8; 64];
        w28a06_pred8x8_gradient(&mut d, 8, 10, &[250; 8], &[20; 8]);
        assert_eq!(d[0], 255);
        w28a06_pred8x8_gradient(&mut d, 8, 100, &[10; 8], &[20; 8]);
        assert_eq!(d[0], 0);
        let b = [3i16; 256];
        assert_eq!(w28a06_sum16x16_half(&b, 16), 384);
    }
}
