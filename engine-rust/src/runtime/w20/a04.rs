#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave20/agent04.rs
// ===========================================================================
// Wave 20 agent 04: motion-estimation SAD kernels, small helpers.
// Strides are in bytes, as in the retail code.

/// Generic SAD of `w` x `h` blocks (helper for the kernels below).
fn w20a04_sad_block(a: &[u8], sa: usize, b: &[u8], sb: usize, w: usize, h: usize) -> u32 {
    let mut s = 0u32;
    for y in 0..h {
        for x in 0..w {
            s += (a[y * sa + x] as i32 - b[y * sb + x] as i32).unsigned_abs();
        }
    }
    s
}

/// `0x004c4fa0`: 8x8 sum of absolute differences between `ref_` (stride
/// `sa`) and `cur` (stride `sb`).
pub fn w20a04_sad_8x8(ref_: &[u8], sa: usize, cur: &[u8], sb: usize) -> u32 {
    w20a04_sad_block(ref_, sa, cur, sb, 8, 8)
}

/// `0x004c5010`: 8x4 sum of absolute differences.
pub fn w20a04_sad_8x4(ref_: &[u8], sa: usize, cur: &[u8], sb: usize) -> u32 {
    w20a04_sad_block(ref_, sa, cur, sb, 8, 4)
}

/// `0x004c52b0`: 4x4 sum of absolute differences.
pub fn w20a04_sad_4x4(ref_: &[u8], sa: usize, cur: &[u8], sb: usize) -> u32 {
    w20a04_sad_block(ref_, sa, cur, sb, 4, 4)
}

/// SAD of `cur` averaged (pavgb, `(a+b+1)>>1`) with a packed `pred`
/// block (rows of `w` bytes) against `ref_`.
fn w20a04_sad_avg(
    ref_: &[u8],
    sa: usize,
    cur: &[u8],
    sb: usize,
    pred: &[u8],
    w: usize,
    h: usize,
) -> u32 {
    let mut s = 0u32;
    for y in 0..h {
        for x in 0..w {
            let avg = ((cur[y * sb + x] as u32 + pred[y * w + x] as u32 + 1) >> 1) as i32;
            s += (avg - ref_[y * sa + x] as i32).unsigned_abs();
        }
    }
    s
}

/// `0x004c5080`: 8x16 SAD of `ref_` against `cur` averaged with the packed
/// 8-byte-per-row prediction `pred` (0x20 bytes per 4 rows).
pub fn w20a04_sad_8x16_avg(ref_: &[u8], sa: usize, cur: &[u8], sb: usize, pred: &[u8]) -> u32 {
    w20a04_sad_avg(ref_, sa, cur, sb, pred, 8, 16)
}

/// `0x004c5330`: 4x8 SAD of `ref_` against `cur` averaged with the packed
/// 4-byte-per-row prediction `pred` (0x10 bytes per 4 rows).
pub fn w20a04_sad_4x8_avg(ref_: &[u8], sa: usize, cur: &[u8], sb: usize, pred: &[u8]) -> u32 {
    w20a04_sad_avg(ref_, sa, cur, sb, pred, 4, 8)
}

fn w20a04_sad3(ref_: &[u8], sa: usize, cur: &[u8], sb: usize, w: usize, h: usize) -> [u32; 3] {
    let mut out = [0u32; 3];
    for (k, o) in out.iter_mut().enumerate() {
        for y in 0..h {
            for x in 0..w {
                let d = ref_[y * sa + x] as i32 - cur[y * sb + k + x] as i32;
                *o += d.unsigned_abs();
            }
        }
    }
    out
}

/// `0x004c5952`: 8-wide, 16-row SADs of `ref_` against `cur` shifted by
/// 0, 1 and 2 bytes; result is written as three u32 (`param_5`).
pub fn w20a04_sad_8x16_x3(ref_: &[u8], sa: usize, cur: &[u8], sb: usize) -> [u32; 3] {
    w20a04_sad3(ref_, sa, cur, sb, 8, 16)
}

/// `0x004c5bb7`: as above for 8x8 blocks.
pub fn w20a04_sad_8x8_x3(ref_: &[u8], sa: usize, cur: &[u8], sb: usize) -> [u32; 3] {
    w20a04_sad3(ref_, sa, cur, sb, 8, 8)
}

/// `0x004c5cf4`: as above for 4x4 blocks.
pub fn w20a04_sad_4x4_x3(ref_: &[u8], sa: usize, cur: &[u8], sb: usize) -> [u32; 3] {
    w20a04_sad3(ref_, sa, cur, sb, 4, 4)
}

/// `0x004c3ad0`: fills four 4-byte rows (row stride `stride`) with the
/// bytes of the little-endian word `v`: row `i` = byte `i` repeated 4x.
pub fn w20a04_splat_rows(dst: &mut [u8], stride: usize, v: u32) {
    for (i, b) in v.to_le_bytes().iter().enumerate() {
        dst[i * stride..i * stride + 4].fill(*b);
    }
}

/// `0x00c9c63a`: leap-year test on a `tm_year`-style value (years since
/// 1900): divisible by 4 and not by 100, or `year + 1900` divisible by 400.
pub fn w20a04_is_leap_tm_year(year: i32) -> bool {
    if year % 4 == 0 && year % 100 != 0 {
        return true;
    }
    year.wrapping_add(0x76c) % 400 == 0
}

/// `0x00c9c8c1`: calls `get` (unresolved callee); when nonzero asks
/// `reject(value, arg)` and returns 0 if it reports nonzero, else the value.
pub fn w20a04_checked_value(
    arg: u32,
    get: impl FnOnce() -> u32,
    reject: impl FnOnce(u32, u32) -> i32,
) -> u32 {
    let v = get();
    if v != 0 && reject(v, arg) == 0 { v } else { 0 }
}

/// `0x00c9d5d7`: runs `f(&mut slot, 1)`; the stored value is 0 on a return
/// of 1 (with whatever `f` wrote) and `0xffffffff` otherwise.
pub fn w20a04_read_u32_or_ff(f: impl FnOnce(&mut [u32; 1], i32) -> i32) -> u32 {
    let mut l = [0u32; 1];
    if f(&mut l, 1) != 1 {
        l[0] = 0xffff_ffff;
    }
    l[0]
}

/// `0x00c9d609`: two-word variant of the above (both words become
/// `0xffffffff` when `f(&mut buf, 1)` does not return 1).
pub fn w20a04_read_u64_or_ff(f: impl FnOnce(&mut [u32; 4], i32) -> i32) -> [u32; 2] {
    let mut l = [0u32; 4];
    if f(&mut l, 1) != 1 {
        l[0] = 0xffff_ffff;
        l[1] = 0xffff_ffff;
    }
    [l[0], l[1]]
}

#[cfg(test)]
mod tests_wave20_agent04 {
    use super::*;

    fn img(n: usize, seed: u32) -> Vec<u8> {
        (0..n)
            .map(|i| ((i as u32 * 37 + seed * 11) % 251) as u8)
            .collect()
    }

    #[test]
    fn sad_basic() {
        let a = vec![10u8; 16 * 16];
        let b = vec![13u8; 16 * 16];
        assert_eq!(w20a04_sad_8x8(&a, 16, &b, 16), 3 * 64);
        assert_eq!(w20a04_sad_8x4(&a, 16, &b, 16), 3 * 32);
        assert_eq!(w20a04_sad_4x4(&a, 16, &b, 16), 3 * 16);
    }

    #[test]
    fn sad_avg() {
        let r = vec![5u8; 16 * 16];
        let c = vec![4u8; 16 * 16];
        let p = vec![7u8; 128];
        // avg(4,7)=6 -> |6-5| = 1
        assert_eq!(w20a04_sad_8x16_avg(&r, 16, &c, 16, &p), 128);
        assert_eq!(w20a04_sad_4x8_avg(&r, 16, &c, 16, &p), 32);
    }

    #[test]
    fn sad_x3() {
        let r = img(32 * 20, 1);
        let c = img(32 * 20, 2);
        let o = w20a04_sad_8x16_x3(&r, 32, &c, 32);
        for k in 0..3 {
            assert_eq!(o[k], w20a04_sad_block(&r, 32, &c[k..], 32, 8, 16));
        }
        let o = w20a04_sad_8x8_x3(&r, 32, &c, 32);
        assert_eq!(o[2], w20a04_sad_block(&r, 32, &c[2..], 32, 8, 8));
        let o = w20a04_sad_4x4_x3(&r, 32, &c, 32);
        assert_eq!(o[1], w20a04_sad_block(&r, 32, &c[1..], 32, 4, 4));
    }

    #[test]
    fn splat() {
        let mut d = [0u8; 32];
        w20a04_splat_rows(&mut d, 8, 0x04030201);
        assert_eq!(&d[0..4], &[1; 4]);
        assert_eq!(&d[8..12], &[2; 4]);
        assert_eq!(&d[16..20], &[3; 4]);
        assert_eq!(&d[24..28], &[4; 4]);
    }

    #[test]
    fn leap() {
        assert!(w20a04_is_leap_tm_year(96));
        assert!(w20a04_is_leap_tm_year(100));
        assert!(!w20a04_is_leap_tm_year(200));
        assert!(!w20a04_is_leap_tm_year(97));
    }

    #[test]
    fn callbacks() {
        assert_eq!(w20a04_checked_value(1, || 9, |_, _| 0), 9);
        assert_eq!(w20a04_checked_value(1, || 9, |_, _| 1), 0);
        assert_eq!(w20a04_checked_value(1, || 0, |_, _| 0), 0);
        assert_eq!(
            w20a04_read_u32_or_ff(|s, _| {
                s[0] = 5;
                1
            }),
            5
        );
        assert_eq!(w20a04_read_u32_or_ff(|_, _| 0), 0xffff_ffff);
        assert_eq!(
            w20a04_read_u64_or_ff(|s, _| {
                s[1] = 7;
                1
            }),
            [0, 7]
        );
        assert_eq!(w20a04_read_u64_or_ff(|_, _| 0), [u32::MAX; 2]);
    }
}
