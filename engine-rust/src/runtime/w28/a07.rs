#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent07.rs
// ===========================================================================
// Wave 28 translation batch 07: video-codec block metrics (sum / SSD /
// variance) helpers.  Unresolved callees become closures.

/// `0x00a23e20`: sum of a 4x4 block of `i16` (row stride `stride` shorts),
/// wrapping in 16 bits, then doubled.
pub fn w28a07_sum4x4_x2(src: &[i16], stride: usize) -> i16 {
    let mut s: i16 = 0;
    for r in 0..4 {
        for c in 0..4 {
            s = s.wrapping_add(src[r * stride + c]);
        }
    }
    s.wrapping_mul(2)
}

/// `0x00a24010`: wrapping 16-bit sum of an 8x8 block of `i16`.
pub fn w28a07_sum8x8(src: &[i16], stride: usize) -> i16 {
    let mut s: i16 = 0;
    for r in 0..8 {
        for c in 0..8 {
            s = s.wrapping_add(src[r * stride + c]);
        }
    }
    s
}

/// `0x00a24450`: 16x16 block of `i16`; eight per-column-lane 16-bit
/// accumulators (lane = column mod 8), summed in 32 bits and halved.
pub fn w28a07_sum16x16_half(src: &[i16], stride: usize) -> i16 {
    let mut lanes = [0i16; 8];
    for r in 0..16 {
        for c in 0..16 {
            lanes[c & 7] = lanes[c & 7].wrapping_add(src[r * stride + c]);
        }
    }
    (lanes.iter().map(|&v| v as i32).sum::<i32>() >> 1) as i16
}

/// `0x00a30c00`: sum of 4 rows of four `i16` (row `r` starts `r * 2 *
/// stride_bytes` bytes in; i.e. `r * stride_bytes` shorts), doubled.
pub fn w28a07_sum4x4_bytes_stride_x2(src: &[i16], stride_bytes: usize) -> i16 {
    let mut s: i16 = 0;
    for r in 0..4 {
        let base = r * stride_bytes; // byte offset param_3 * 2 * r, in shorts
        for c in 0..4 {
            s = s.wrapping_add(src[base + c]);
        }
    }
    s.wrapping_mul(2)
}

/// `0x00a317f0`: sum of squared differences of two 16x16 byte blocks.
/// Returns `(sse, sse)` (the routine stores the running total through its
/// out pointer too).
pub fn w28a07_ssd16x16(a: &[u8], a_stride: usize, b: &[u8], b_stride: usize) -> (i32, i32) {
    let mut t: i32 = 0;
    for r in 0..16 {
        for c in 0..16 {
            let d = a[r * a_stride + c] as i32 - b[r * b_stride + c] as i32;
            t = t.wrapping_add(d.wrapping_mul(d));
        }
    }
    (t, t)
}

/// `0x00a36d00`: 16x8 variance; `sad(...)` returns `(sum, sse)`; result is
/// `(sse, sse - (sum^2 >> 7))` (unsigned shift).
pub fn w28a07_var16x8(sub: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = sub();
    (
        sse,
        sse.wrapping_sub(((sum.wrapping_mul(sum)) as u32 >> 7) as i32),
    )
}

/// `0x00a37810`: 16x16 variance, unsigned `>> 8`.  Returns `(sse, var)`.
pub fn w28a07_var16_u8(sub: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = sub();
    (
        sse,
        sse.wrapping_sub(((sum.wrapping_mul(sum)) as u32 >> 8) as i32),
    )
}

fn w28a07_shr64(sum: i32, sh: u32) -> i32 {
    let p = (sum as i64).wrapping_mul(sum as i64) as u64;
    (p >> sh) as u32 as i32
}

/// `0x00a37850`: 32-wide variance, 64-bit square `>> 9`.  `(sse, var)`.
pub fn w28a07_var32_s9(sub: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = sub();
    (sse, sse.wrapping_sub(w28a07_shr64(sum, 9)))
}

/// `0x00a37930`: 8-wide variance, signed `>> 7`.  `(sse, var)`.
pub fn w28a07_var_s7_a(sub: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = sub();
    (sse, sse.wrapping_sub(sum.wrapping_mul(sum) >> 7))
}

/// `0x00a38130`: variance, signed `>> 7`.  `(sse, var)`.
pub fn w28a07_var_s7_b(sub: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    w28a07_var_s7_a(sub)
}

/// `0x00a37cb0`: 4-wide variance, signed `>> 4`.  `(sse, var)`.
pub fn w28a07_var_s4(sub: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = sub();
    (sse, sse.wrapping_sub(sum.wrapping_mul(sum) >> 4))
}

/// `0x00a37cf0`: variance, signed `>> 5`.  `(sse, var)`.
pub fn w28a07_var_s5_a(sub: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    let (sum, sse) = sub();
    (sse, sse.wrapping_sub(sum.wrapping_mul(sum) >> 5))
}

/// `0x00a37d30`: variance, signed `>> 5`.  `(sse, var)`.
pub fn w28a07_var_s5_b(sub: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    w28a07_var_s5_a(sub)
}

/// `0x00a38170`: variance, signed `>> 5`.  `(sse, var)`.
pub fn w28a07_var_s5_c(sub: impl FnOnce() -> (i32, i32)) -> (i32, i32) {
    w28a07_var_s5_a(sub)
}

/// `0x00a37970`: two 16-wide halves (offsets 0 and 16) combined, 64-bit
/// square `>> 9`.  `sub(offset)` returns `(sum, sse)`.
pub fn w28a07_var_two_halves_9(mut sub: impl FnMut(usize) -> (i32, i32)) -> (i32, i32) {
    let (s0, e0) = sub(0);
    let (s1, e1) = sub(0x10);
    let sse = e0.wrapping_add(e1);
    (sse, sse.wrapping_sub(w28a07_shr64(s0.wrapping_add(s1), 9)))
}

/// `0x00a37bf0`: two 16-wide halves, 64-bit square `>> 11`.
pub fn w28a07_var_two_halves_11(mut sub: impl FnMut(usize) -> (i32, i32)) -> (i32, i32) {
    let (s0, e0) = sub(0);
    let (s1, e1) = sub(0x10);
    let sse = e0.wrapping_add(e1);
    (sse, sse.wrapping_sub(w28a07_shr64(s0.wrapping_add(s1), 11)))
}

/// `0x00a37d70`: four 16-wide quarters (offsets 0, 0x10, 0x20, 0x30),
/// 64-bit square `>> 11`.
pub fn w28a07_var_four_quarters_11(mut sub: impl FnMut(usize) -> (i32, i32)) -> (i32, i32) {
    let mut sum = 0i32;
    let mut sse = 0i32;
    for k in 0..4 {
        let (s, e) = sub(k * 0x10);
        sum = sum.wrapping_add(s);
        sse = sse.wrapping_add(e);
    }
    (sse, sse.wrapping_sub(w28a07_shr64(sum, 11)))
}

#[cfg(test)]
mod tests_wave28_agent07 {
    use super::*;

    #[test]
    fn sums() {
        let a = [1i16; 64];
        assert_eq!(w28a07_sum4x4_x2(&a, 8), 32);
        assert_eq!(w28a07_sum8x8(&a, 8), 64);
        let b = [3i16; 256];
        assert_eq!(w28a07_sum16x16_half(&b, 16), 384);
        assert_eq!(w28a07_sum4x4_bytes_stride_x2(&[2i16; 64], 4), 64);
    }

    #[test]
    fn ssd() {
        let a = [5u8; 256];
        let b = [3u8; 256];
        assert_eq!(w28a07_ssd16x16(&a, 16, &b, 16), (1024, 1024));
    }

    #[test]
    fn variances() {
        assert_eq!(w28a07_var16x8(|| (128, 1000)), (1000, 1000 - 128));
        assert_eq!(w28a07_var16_u8(|| (16, 300)), (300, 299));
        assert_eq!(w28a07_var32_s9(|| (512, 5000)), (5000, 5000 - 512));
        assert_eq!(w28a07_var_s7_a(|| (-16, 100)), (100, 98));
        assert_eq!(w28a07_var_s7_b(|| (-16, 100)), (100, 98));
        assert_eq!(w28a07_var_s4(|| (4, 10)), (10, 9));
        assert_eq!(w28a07_var_s5_a(|| (8, 10)), (10, 8));
        assert_eq!(w28a07_var_s5_b(|| (8, 10)), (10, 8));
        assert_eq!(w28a07_var_s5_c(|| (8, 10)), (10, 8));
    }

    #[test]
    fn composite() {
        let r = w28a07_var_two_halves_9(|o| if o == 0 { (256, 10) } else { (256, 20) });
        assert_eq!(r, (30, 30 - 512));
        let r = w28a07_var_two_halves_11(|o| (o as i32, 1));
        assert_eq!(r, (2, 2));
        let r = w28a07_var_four_quarters_11(|_| (64, 100));
        assert_eq!(r, (400, 400 - 32));
    }
}
