#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent10.rs
// ===========================================================================
// Wave 28 translation batch 10: SSE2 sum-of-absolute-differences kernels
// (video codec motion search) from the retail image.

/// Shared SAD helper: sums `|src - ref|` over a `w` x `h` block.
pub fn w28a10_sad(
    src: &[u8],
    src_stride: usize,
    rf: &[u8],
    ref_stride: usize,
    w: usize,
    h: usize,
) -> u32 {
    let mut sum = 0u32;
    for y in 0..h {
        let a = &src[y * src_stride..y * src_stride + w];
        let b = &rf[y * ref_stride..y * ref_stride + w];
        sum += a
            .iter()
            .zip(b)
            .map(|(&p, &q)| p.abs_diff(q) as u32)
            .sum::<u32>();
    }
    sum
}

/// Shared x4d helper: one SAD per reference block (four blocks).
pub fn w28a10_sad_x4d(
    src: &[u8],
    src_stride: usize,
    refs: [&[u8]; 4],
    ref_stride: usize,
    w: usize,
    h: usize,
    out: &mut [u32; 4],
) {
    for (o, r) in out.iter_mut().zip(refs) {
        *o = w28a10_sad(src, src_stride, r, ref_stride, w, h);
    }
}

/// `0x00a508a0`: 16x16 SAD.
pub fn w28a10_sad16x16(s: &[u8], ss: usize, r: &[u8], rs: usize) -> u32 {
    w28a10_sad(s, ss, r, rs, 16, 16)
}
/// `0x00a50aa0`: 16x32 SAD.
pub fn w28a10_sad16x32(s: &[u8], ss: usize, r: &[u8], rs: usize) -> u32 {
    w28a10_sad(s, ss, r, rs, 16, 32)
}
/// `0x00a50be0`: 16x8 SAD.
pub fn w28a10_sad16x8(s: &[u8], ss: usize, r: &[u8], rs: usize) -> u32 {
    w28a10_sad(s, ss, r, rs, 16, 8)
}
/// `0x00a50e60`: 32x16 SAD against four references.
pub fn w28a10_sad32x16x4d(s: &[u8], ss: usize, refs: [&[u8]; 4], rs: usize, out: &mut [u32; 4]) {
    w28a10_sad_x4d(s, ss, refs, rs, 32, 16, out)
}
/// `0x00a50fe0`: 32x32 SAD against four references.
pub fn w28a10_sad32x32x4d(s: &[u8], ss: usize, refs: [&[u8]; 4], rs: usize, out: &mut [u32; 4]) {
    w28a10_sad_x4d(s, ss, refs, rs, 32, 32, out)
}
/// `0x00a51160`: 32x64 SAD against four references.
pub fn w28a10_sad32x64x4d(s: &[u8], ss: usize, refs: [&[u8]; 4], rs: usize, out: &mut [u32; 4]) {
    w28a10_sad_x4d(s, ss, refs, rs, 32, 64, out)
}
/// `0x00a516f0`: 64x32 SAD.
pub fn w28a10_sad64x32(s: &[u8], ss: usize, r: &[u8], rs: usize) -> u32 {
    w28a10_sad(s, ss, r, rs, 64, 32)
}
/// `0x00a51760`: 64x32 SAD against four references.
pub fn w28a10_sad64x32x4d(s: &[u8], ss: usize, refs: [&[u8]; 4], rs: usize, out: &mut [u32; 4]) {
    w28a10_sad_x4d(s, ss, refs, rs, 64, 32, out)
}
/// `0x00a518d0`: 64x64 SAD.
pub fn w28a10_sad64x64(s: &[u8], ss: usize, r: &[u8], rs: usize) -> u32 {
    w28a10_sad(s, ss, r, rs, 64, 64)
}
/// `0x00a51940`: 64x64 SAD against four references.
pub fn w28a10_sad64x64x4d(s: &[u8], ss: usize, refs: [&[u8]; 4], rs: usize, out: &mut [u32; 4]) {
    w28a10_sad_x4d(s, ss, refs, rs, 64, 64, out)
}

#[cfg(test)]
mod tests_wave28_agent10 {
    use super::*;

    fn buf(n: usize, k: u8) -> Vec<u8> {
        (0..n).map(|i| (i as u8).wrapping_mul(k)).collect()
    }

    #[test]
    fn sad_single() {
        let a = vec![10u8; 64 * 64];
        let b = vec![13u8; 64 * 64];
        assert_eq!(w28a10_sad16x16(&a, 64, &b, 64), 3 * 256);
        assert_eq!(w28a10_sad16x32(&a, 64, &b, 64), 3 * 512);
        assert_eq!(w28a10_sad16x8(&a, 64, &b, 64), 3 * 128);
        assert_eq!(w28a10_sad64x32(&a, 64, &b, 64), 3 * 2048);
        assert_eq!(w28a10_sad64x64(&a, 64, &b, 64), 3 * 4096);
        assert_eq!(w28a10_sad16x16(&a, 64, &a, 64), 0);
    }

    #[test]
    fn sad_strides() {
        let a = buf(40 * 16, 7);
        let b = buf(40 * 16, 3);
        let mut e = 0;
        for y in 0..8 {
            for x in 0..16 {
                e += a[y * 40 + x].abs_diff(b[y * 40 + x]) as u32;
            }
        }
        assert_eq!(w28a10_sad16x8(&a, 40, &b, 40), e);
    }

    #[test]
    fn sad_x4d() {
        let a = vec![0u8; 64 * 64];
        let r: Vec<Vec<u8>> = (1..=4).map(|k| vec![k as u8; 64 * 64]).collect();
        let refs = [&r[0][..], &r[1][..], &r[2][..], &r[3][..]];
        let mut o = [0u32; 4];
        w28a10_sad32x16x4d(&a, 64, refs, 64, &mut o);
        assert_eq!(o, [512, 1024, 1536, 2048]);
        w28a10_sad32x32x4d(&a, 64, refs, 64, &mut o);
        assert_eq!(o[3], 4 * 1024);
        w28a10_sad32x64x4d(&a, 64, refs, 64, &mut o);
        assert_eq!(o[0], 2048);
        w28a10_sad64x32x4d(&a, 64, refs, 64, &mut o);
        assert_eq!(o[1], 2 * 2048);
        w28a10_sad64x64x4d(&a, 64, refs, 64, &mut o);
        assert_eq!(o[2], 3 * 4096);
    }
}
