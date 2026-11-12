#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent18.rs
// ===========================================================================
// Wave 34 agent 18: SSE2 16x16 block statistics kernel.
// Skipped: FUN_004be080 and FUN_004be291 (MMX loop-filter bodies whose
// decompiled lane shuffles are not reliably recoverable).

/// `0x004bdf99`: 16x16 block statistics.  Returns `(var_like, sad_like)`
/// where `sad_like = (sum |src-ref| + 0x80) >> 8` (written to `*param_6`)
/// and `var_like = (sumsq(ref) - (sum(ref)^2 >> 8) + 0x80) >> 8`
/// (written to `*param_5`).  Both blocks are 16 rows of 16 bytes, each
/// with its own stride.  Arithmetic wraps in 32 bits like the original.
pub fn w34a18_block16_stats(
    src: &[u8],
    src_stride: usize,
    reference: &[u8],
    ref_stride: usize,
) -> (u32, u32) {
    let mut sad: u32 = 0;
    let mut sum: u32 = 0;
    let mut sumsq: u32 = 0;
    for row in 0..16 {
        let s = &src[row * src_stride..row * src_stride + 16];
        let r = &reference[row * ref_stride..row * ref_stride + 16];
        for i in 0..16 {
            sad += (s[i] as i32 - r[i] as i32).unsigned_abs();
            sum += r[i] as u32;
            sumsq = sumsq.wrapping_add(r[i] as u32 * r[i] as u32);
        }
    }
    let sad_out = sad.wrapping_add(0x80) >> 8;
    let var_out = sumsq
        .wrapping_sub(sum.wrapping_mul(sum) >> 8)
        .wrapping_add(0x80)
        >> 8;
    (var_out, sad_out)
}

#[cfg(test)]
mod tests_wave34_agent18 {
    use super::*;

    #[test]
    fn flat_blocks() {
        let a = vec![10u8; 256];
        let b = vec![10u8; 256];
        assert_eq!(w34a18_block16_stats(&a, 16, &b, 16), (0, 0));
    }

    #[test]
    fn sad_and_variance() {
        let a = vec![0u8; 256];
        let b: Vec<u8> = (0..256).map(|i| if i % 2 == 0 { 255 } else { 0 }).collect();
        let (v, s) = w34a18_block16_stats(&a, 16, &b, 16);
        assert_eq!(s, (128 * 255 + 0x80) >> 8);
        let sumsq = 128u32 * 255 * 255;
        let sum = 128u32 * 255;
        assert_eq!(v, (sumsq - (sum * sum >> 8) + 0x80) >> 8);
    }
}
