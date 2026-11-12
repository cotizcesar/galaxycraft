#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent15.rs
// ===========================================================================
// Wave 26 agent 15: stream seek helper, MD5 context routines, bounded copy.

/// `0x00939860`: file-unit seek.  `origin` 0 = set, 1 = add to the current
/// 64-bit position (offset +0x10/+0x14), 2 = end-relative using the length
/// returned by the caller-supplied `size` probe.  Unknown origins (or a
/// failed probe) leave the position unchanged.  Returns the position.
pub fn w26a15_seek(
    pos: &mut u64,
    offset: u64,
    origin: u32,
    size: impl FnOnce() -> Option<u64>,
) -> u64 {
    match origin {
        0 => *pos = offset,
        1 => *pos = pos.wrapping_add(offset),
        2 => {
            if let Some(len) = size() {
                *pos = len.wrapping_add(offset);
            }
        }
        _ => {}
    }
    *pos
}

/// MD5 context as laid out by the retail routines: state words at +0x00,
/// 64-bit bit count at +0x10/+0x14, 64-byte buffer at +0x18.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W26a15Md5 {
    pub state: [u32; 4],
    pub count: [u32; 2],
    pub buffer: [u8; 64],
}

impl Default for W26a15Md5 {
    fn default() -> Self {
        Self {
            state: [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476],
            count: [0; 2],
            buffer: [0; 64],
        }
    }
}

const W26A15_K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];
const W26A15_S: [u32; 16] = [7, 12, 17, 22, 5, 9, 14, 20, 4, 11, 16, 23, 6, 10, 15, 21];

/// `0x0093a250`: MD5 block transform over a 64-byte little-endian block,
/// adding the result into `state`.
pub fn w26a15_md5_transform(state: &mut [u32; 4], block: &[u8; 64]) {
    let mut m = [0u32; 16];
    for (i, w) in m.iter_mut().enumerate() {
        *w = u32::from_le_bytes([
            block[i * 4],
            block[i * 4 + 1],
            block[i * 4 + 2],
            block[i * 4 + 3],
        ]);
    }
    let [mut a, mut b, mut c, mut d] = *state;
    for i in 0..64 {
        let (f, g) = match i / 16 {
            0 => ((b & c) | (!b & d), i),
            1 => ((b & d) | (c & !d), (5 * i + 1) % 16),
            2 => (b ^ c ^ d, (3 * i + 5) % 16),
            _ => (c ^ (b | !d), (7 * i) % 16),
        };
        let t = a
            .wrapping_add(f)
            .wrapping_add(W26A15_K[i])
            .wrapping_add(m[g]);
        a = d;
        d = c;
        c = b;
        b = b.wrapping_add(t.rotate_left(W26A15_S[(i / 16) * 4 + i % 4]));
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
}

/// `0x0093aa80`: MD5 update.  Bumps the 64-bit bit count, fills the buffer
/// and runs the block transform on every completed 64-byte block.
pub fn w26a15_md5_update(ctx: &mut W26a15Md5, data: &[u8]) {
    let len = data.len() as u32;
    let mut idx = ((ctx.count[0] >> 3) & 0x3f) as usize;
    let (lo, carry) = ctx.count[0].overflowing_add(len << 3);
    ctx.count[0] = lo;
    ctx.count[1] = ctx.count[1]
        .wrapping_add(carry as u32)
        .wrapping_add(len >> 29);
    for &byte in data {
        ctx.buffer[idx] = byte;
        idx += 1;
        if idx == 64 {
            let block = ctx.buffer;
            w26a15_md5_transform(&mut ctx.state, &block);
            idx = 0;
        }
    }
}

/// `0x0093a960`: MD5 finalise.  Saves the bit count, pads to 56 mod 64 with
/// `0x80, 0...`, appends the count, writes the 16-byte digest and wipes the
/// 0x58-byte context.
pub fn w26a15_md5_final(ctx: &mut W26a15Md5) -> [u8; 16] {
    let mut bits = [0u8; 8];
    bits[..4].copy_from_slice(&ctx.count[0].to_le_bytes());
    bits[4..].copy_from_slice(&ctx.count[1].to_le_bytes());
    let idx = ((ctx.count[0] >> 3) & 0x3f) as usize;
    let pad_len = if idx < 56 { 56 - idx } else { 120 - idx };
    let mut pad = [0u8; 64];
    pad[0] = 0x80;
    w26a15_md5_update(ctx, &pad[..pad_len]);
    w26a15_md5_update(ctx, &bits);
    let mut out = [0u8; 16];
    for (i, w) in ctx.state.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    *ctx = W26a15Md5 {
        state: [0; 4],
        count: [0; 2],
        buffer: [0; 64],
    };
    out
}

/// `0x0093ab70`: copies the NUL-terminated `src` into `dst`, at most 0x3f
/// bytes (the terminator is not copied).  Returns the byte count copied.
pub fn w26a15_copy_clamped(dst: &mut [u8], src: &[u8]) -> usize {
    let len = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    let n = len.min(0x3f).min(dst.len());
    dst[..n].copy_from_slice(&src[..n]);
    n
}

#[cfg(test)]
mod tests_wave26_agent15 {
    use super::*;

    fn hex(d: [u8; 16]) -> String {
        d.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn seek_modes() {
        let mut p = 10u64;
        assert_eq!(w26a15_seek(&mut p, 5, 0, || None), 5);
        assert_eq!(w26a15_seek(&mut p, 0xffff_ffff, 1, || None), 0x1_0000_0004);
        assert_eq!(w26a15_seek(&mut p, 3, 2, || Some(100)), 103);
        assert_eq!(w26a15_seek(&mut p, 3, 2, || None), 103);
        assert_eq!(w26a15_seek(&mut p, 9, 7, || Some(1)), 103);
    }

    #[test]
    fn md5_vectors() {
        let mut c = W26a15Md5::default();
        assert_eq!(
            hex(w26a15_md5_final(&mut c)),
            "d41d8cd98f00b204e9800998ecf8427e"
        );
        let mut c = W26a15Md5::default();
        w26a15_md5_update(&mut c, b"abc");
        assert_eq!(
            hex(w26a15_md5_final(&mut c)),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(c.state, [0; 4]);
        let mut c = W26a15Md5::default();
        let msg =
            b"12345678901234567890123456789012345678901234567890123456789012345678901234567890";
        w26a15_md5_update(&mut c, &msg[..33]);
        w26a15_md5_update(&mut c, &msg[33..]);
        assert_eq!(
            hex(w26a15_md5_final(&mut c)),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }

    #[test]
    fn transform_count() {
        let mut c = W26a15Md5::default();
        w26a15_md5_update(&mut c, &[0u8; 64]);
        assert_eq!(c.count, [512, 0]);
        let mut s = W26a15Md5::default().state;
        w26a15_md5_transform(&mut s, &[0; 64]);
        assert_eq!(s, c.state);
    }

    #[test]
    fn clamped_copy() {
        let mut d = [0xffu8; 100];
        assert_eq!(w26a15_copy_clamped(&mut d, b"hi\0xx"), 2);
        assert_eq!(&d[..3], b"hi\xff");
        let long = [b'a'; 80];
        assert_eq!(w26a15_copy_clamped(&mut d, &long), 0x3f);
    }
}
