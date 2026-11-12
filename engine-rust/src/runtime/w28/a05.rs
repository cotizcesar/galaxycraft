#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent05.rs
// ===========================================================================
// Wave 28 translation batch 05: block-sum / SAD / sub-pel filter helpers
// from the video-codec region of the retail image (pure functions over
// byte slices).

/// `0x00a1a840`: rounded mean of a 4x4 byte block.  Four rows of one
/// little-endian dword at `base`, `base + stride`, ...; result is
/// `(sum + 8) >> 4`.
pub fn w28a05_mean_4x4(src: &[u8], base: usize, stride: usize) -> i32 {
    let mut sum = 0i32;
    for r in 0..4 {
        for c in 0..4 {
            sum += src[base + r * stride + c] as i32;
        }
    }
    (sum + 8) >> 4
}

/// `0x00a1a890`: rounded mean of an 8x8 byte block, `(sum + 0x20) >> 6`.
pub fn w28a05_mean_8x8(src: &[u8], base: usize, stride: usize) -> i32 {
    let mut sum = 0i32;
    for r in 0..8 {
        for c in 0..8 {
            sum += src[base + r * stride + c] as i32;
        }
    }
    (sum + 0x20) >> 6
}

/// `0x00a1b5b0`: column sums of 16 byte columns over `rows` rows (wrapping
/// 16-bit accumulate), each divided (truncating) by `rows >> 1`.  Returns
/// `None` when that divisor is zero (retail would fault).
pub fn w28a05_column_avg16(src: &[u8], base: usize, stride: usize, rows: i32) -> Option<[i16; 16]> {
    let div = rows >> 1;
    if div == 0 {
        return None;
    }
    let mut out = [0i16; 16];
    for (c, o) in out.iter_mut().enumerate() {
        let mut acc: i16 = 0;
        for r in 0..rows.max(0) as usize {
            acc = acc.wrapping_add(src[base + c + r * stride] as i16);
        }
        *o = ((acc as i32) / div) as i16;
    }
    Some(out)
}

/// `0x00a1bcd0`: mean of the first four bytes of four rows (SSE saturating
/// 16-bit lane adds; the high four lanes are dropped by the fold),
/// `(sum + 8) >> 4`.
pub fn w28a05_mean_4x4_sse(src: &[u8], base: usize, stride: usize) -> u32 {
    let mut sum = 0u32;
    for r in 0..4 {
        for c in 0..4 {
            sum += src[base + r * stride + c] as u32;
        }
    }
    (sum.min(0xffff) + 8) >> 4
}

/// `0x00a1c070`: sum of absolute bytes (psadbw) over `ceil(len / 16)`
/// 16-byte rows (at least one), accumulated with unsigned 16-bit
/// saturation; the two 64-bit halves are folded together.
pub fn w28a05_sad_rows(src: &[u8], len: i32) -> u16 {
    let rows = if len > 16 {
        ((len - 0x11) as u32 >> 4) as usize + 2
    } else {
        1
    };
    let mut lo = 0u16;
    let mut hi = 0u16;
    for r in 0..rows {
        let row = &src[r * 16..r * 16 + 16];
        let l: u16 = row[..8].iter().map(|&b| b as u16).sum();
        let h: u16 = row[8..].iter().map(|&b| b as u16).sum();
        lo = lo.saturating_add(l);
        hi = hi.saturating_add(h);
    }
    lo.saturating_add(hi)
}

/// `0x00a1b620`: min and max of `|a - b|` over an 8x8 block of two byte
/// images with independent strides.  Returns `(min, max)`, starting from
/// `(255, 0)`.
pub fn w28a05_abs_diff_range(
    a: &[u8],
    a_base: usize,
    a_stride: usize,
    b: &[u8],
    b_base: usize,
    b_stride: usize,
) -> (i32, i32) {
    let (mut min, mut max) = (0xff, 0);
    for r in 0..8 {
        for c in 0..8 {
            let d =
                (a[a_base + r * a_stride + c] as i32 - b[b_base + r * b_stride + c] as i32).abs();
            min = min.min(d);
            max = max.max(d);
        }
    }
    (min, max)
}

/// `0x00a1c860`: horizontal 8-tap sub-pel filter averaged into `dst`.
/// `taps` is 16 phases of 8 `i16`; `phase0` is the low byte of the retail
/// phase pointer, `step` the per-pixel increment; the source pointer is
/// `src_base - 3`.  Each output is clamped to 0..=255 after
/// `(sum + 0x40) >> 7`, then rounded-averaged with the existing `dst`.
#[allow(clippy::too_many_arguments)]
pub fn w28a05_subpel_h8_avg(
    src: &[u8],
    src_base: usize,
    src_stride: usize,
    dst: &mut [u8],
    dst_base: usize,
    dst_stride: usize,
    taps: &[i16],
    phase0: u32,
    step: i32,
    width: i32,
    height: i32,
) {
    let mut s = src_base - 3;
    let mut d = dst_base;
    for _ in 0..height.max(0) {
        let mut pos = ((phase0 & 0xff) as i32) >> 4;
        for x in 0..width.max(0) as usize {
            let p = s + (pos >> 4) as usize;
            let t = ((pos & 0xf) * 8) as usize;
            let mut sum = 0x40i32;
            for k in 0..8 {
                sum = sum.wrapping_add((src[p + k] as i32).wrapping_mul(taps[t + k] as i32));
            }
            let v = (sum >> 7).clamp(0, 255) as u32;
            pos = pos.wrapping_add(step);
            dst[d + x] = ((v & 0xff) + dst[d + x] as u32 + 1 >> 1) as u8;
        }
        s += src_stride;
        d += dst_stride;
    }
}

#[cfg(test)]
mod tests_wave28_agent05 {
    use super::*;

    #[test]
    fn means() {
        let s = [10u8; 64];
        assert_eq!(w28a05_mean_4x4(&s, 0, 8), 10);
        assert_eq!(w28a05_mean_8x8(&s, 0, 8), 10);
        assert_eq!(w28a05_mean_4x4_sse(&s, 0, 8), 10);
        let t = [1u8; 64];
        assert_eq!(w28a05_mean_4x4(&t, 0, 8), 1);
    }

    #[test]
    fn column_avg() {
        let s = [4u8; 64];
        assert_eq!(w28a05_column_avg16(&s, 0, 16, 4).unwrap(), [8; 16]);
        assert!(w28a05_column_avg16(&s, 0, 16, 1).is_none());
    }

    #[test]
    fn sad() {
        let s = [1u8; 64];
        assert_eq!(w28a05_sad_rows(&s, 16), 16);
        assert_eq!(w28a05_sad_rows(&s, 17), 32);
        assert_eq!(w28a05_sad_rows(&s, 33), 48);
    }

    #[test]
    fn range() {
        let a = [5u8; 64];
        let mut b = [5u8; 64];
        b[9] = 105;
        b[0] = 4;
        assert_eq!(w28a05_abs_diff_range(&a, 0, 8, &b, 0, 8), (0, 100));
    }

    #[test]
    fn subpel() {
        let src = [100u8; 64];
        let mut taps = [0i16; 128];
        taps[3] = 128;
        let mut dst = [50u8; 16];
        w28a05_subpel_h8_avg(&src, 3, 16, &mut dst, 0, 4, &taps, 0, 16, 4, 1);
        assert_eq!(&dst[..4], &[75; 4]);
    }
}
