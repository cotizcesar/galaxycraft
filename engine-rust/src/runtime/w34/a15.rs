#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent15.rs
// ===========================================================================
// Wave 34 agent 15: block SSD and block-copy kernels.

/// `0x004bd618`: sum of squared differences over 128 contiguous i16 samples
/// (16 iterations of 8 lanes; `pmaddwd` pairs, wrapping 32-bit accumulation,
/// 16-bit wrapping subtraction).
pub fn w34a15_ssd_128(a: &[i16], b: &[i16]) -> i32 {
    let mut sum = 0i32;
    for i in 0..128 {
        let d = a[i].wrapping_sub(b[i]) as i32;
        sum = sum.wrapping_add(d.wrapping_mul(d));
    }
    sum
}

/// `0x004bd730`: copies `rows` rows of 32 bytes from `src` (byte stride
/// `src_stride`) to `dst` (byte stride `dst_stride`).  Retail copies four
/// rows per pass then the remainder; the result is the same as a plain loop
/// for `rows >= 4` (smaller counts are undefined in retail).
pub fn w34a15_copy_rows32(
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    rows: usize,
) {
    for r in 0..rows {
        let s = r * src_stride;
        let d = r * dst_stride;
        dst[d..d + 32].copy_from_slice(&src[s..s + 32]);
    }
}

/// `0x004bd841`: copies four rows of 8 bytes between strided buffers.
pub fn w34a15_copy_4x8(src: &[u8], src_stride: usize, dst: &mut [u8], dst_stride: usize) {
    for r in 0..4 {
        let s = r * src_stride;
        let d = r * dst_stride;
        dst[d..d + 8].copy_from_slice(&src[s..s + 8]);
    }
}

#[cfg(test)]
mod tests_wave34_agent15 {
    use super::*;

    #[test]
    fn ssd() {
        let a = [3i16; 128];
        let b = [1i16; 128];
        assert_eq!(w34a15_ssd_128(&a, &b), 4 * 128);
        assert_eq!(w34a15_ssd_128(&a, &a), 0);
    }

    #[test]
    fn copy32() {
        let src: Vec<u8> = (0..=255u8).cycle().take(40 * 6 + 32).collect();
        let mut dst = vec![0u8; 36 * 6 + 32];
        w34a15_copy_rows32(&src, 40, &mut dst, 36, 6);
        for r in 0..6 {
            assert_eq!(&dst[r * 36..r * 36 + 32], &src[r * 40..r * 40 + 32]);
        }
    }

    #[test]
    fn copy4x8() {
        let src: Vec<u8> = (0..64u8).collect();
        let mut dst = vec![0u8; 64];
        w34a15_copy_4x8(&src, 16, &mut dst, 12);
        assert_eq!(&dst[36..44], &src[48..56]);
    }
}
