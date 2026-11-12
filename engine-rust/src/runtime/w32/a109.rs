#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent109.rs
// ===========================================================================
// Wave 32 agent 109: bounded C-string append and MD5 update.

/// `0x00b8f790`: appends up to `max` bytes of NUL-terminated `src` to a
/// fixed-capacity buffer (`buf[..cap]`, current length `*len`).  Bytes that do
/// not fit are dropped but still consumed; the length is stored back.
pub fn w32a109_bounded_append(buf: &mut [u8], cap: usize, len: &mut usize, src: &[u8], max: usize) {
    let mut pos = *len;
    let cap = cap.min(buf.len());
    let mut i = 0usize;
    while i < max && i < src.len() && src[i] != 0 {
        if pos < cap {
            buf[pos] = src[i];
            pos += 1;
        }
        i += 1;
    }
    *len = pos;
}

/// `0x00b90460`: MD5 update.  `count` is the 64-bit bit counter (low, high),
/// `buffer` the 64-byte block buffer; `transform` is the block function.
pub fn w32a109_md5_update(
    count: &mut [u32; 2],
    buffer: &mut [u8; 64],
    data: &[u8],
    mut transform: impl FnMut(&[u8]),
) {
    let n = data.len() as u32;
    let bits = (n as u64) << 3;
    let mut idx = ((count[0] >> 3) & 0x3f) as usize;
    let (lo, carry) = count[0].overflowing_add(bits as u32);
    count[0] = lo;
    count[1] = count[1]
        .wrapping_add((bits >> 32) as u32)
        .wrapping_add(carry as u32);
    let mut off = 0usize;
    let mut rem = data.len();
    if idx != 0 {
        if rem + idx < 0x40 {
            buffer[idx..idx + rem].copy_from_slice(data);
            return;
        }
        let fill = 0x40 - idx;
        buffer[idx..].copy_from_slice(&data[..fill]);
        transform(&buffer[..]);
        off += fill;
        rem -= fill;
        idx = 0;
    }
    while rem >= 0x40 {
        transform(&data[off..off + 0x40]);
        off += 0x40;
        rem -= 0x40;
    }
    buffer[idx..idx + rem].copy_from_slice(&data[off..off + rem]);
}

#[cfg(test)]
mod tests_wave32_agent109 {
    use super::*;

    #[test]
    fn append_truncates() {
        let mut b = [0u8; 8];
        let mut len = 2;
        w32a109_bounded_append(&mut b, 5, &mut len, b"abcdef\0x", 8);
        assert_eq!(len, 5);
        assert_eq!(&b[2..5], b"abc");
        let mut len = 0;
        w32a109_bounded_append(&mut b, 8, &mut len, b"xyz", 2);
        assert_eq!(len, 2);
    }

    #[test]
    fn md5_update_blocks() {
        let mut c = [0u32; 2];
        let mut buf = [0u8; 64];
        let mut blocks = 0;
        w32a109_md5_update(&mut c, &mut buf, &[1u8; 30], |_| blocks += 1);
        assert_eq!((c[0], blocks), (240, 0));
        w32a109_md5_update(&mut c, &mut buf, &[2u8; 100], |_| blocks += 1);
        assert_eq!(blocks, 2);
        assert_eq!(c[0], 1040);
        assert_eq!(buf[..2], [2, 2]);
        let mut c = [u32::MAX - 7, 0];
        w32a109_md5_update(&mut c, &mut buf, &[0u8; 2], |_| {});
        assert_eq!(c, [8, 1]);
    }
}
