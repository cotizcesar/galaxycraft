#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent07.rs
// ===========================================================================
// Wave 30 translation batch 07: container/table helpers.

/// `0x00b0cd30`: clamps `*out` to the smaller of `*len` and `*out`, stores the
/// result in both, and returns the number of bytes to copy (0 status).
pub fn w30a07_clamp_copy_len(len: &mut u32, out: &mut u32) -> u32 {
    let v = (*len).min(*out);
    *out = v;
    *len = v;
    v
}

/// `0x00b083b0`: resize of a vector of 16-byte elements.  Shrinking drops the
/// tail, growing returns the new element count and (via `grow`) lets the
/// caller initialise new slots.  Returns the new capacity-word when a
/// reallocation is required (low 31 bits = capacity, top bit preserved).
pub fn w30a07_resize16(len: &mut u32, cap_word: &mut u32, new_len: u32) -> Option<u32> {
    if new_len < *len {
        *len = new_len;
        return None;
    }
    if *len < new_len {
        let cap = *cap_word & 0x7fff_ffff;
        if cap < new_len {
            let mut want = (*cap_word >> 1 & 0x3fff_ffff).wrapping_add(cap);
            if want < new_len {
                want = new_len;
            }
            *cap_word = (*cap_word ^ want) & 0x7fff_ffff ^ *cap_word;
            return Some(want);
        }
        *len = new_len;
    }
    None
}

/// Error from the BLTE header validation.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W30a07BlteError {
    NeedBytes(u32),
    BadMagic,
    BadTableFormat(u8),
    HeaderSizeMismatch,
    Complete,
}

/// `0x00b08f60`: validates a BLTE header (`"BLTE"`, big-endian header size,
/// flag byte 0x0f and 24-bit chunk count, `size == count*0x18+0xc`).
pub fn w30a07_blte_header_check(buf: &[u8], avail: u32) -> W30a07BlteError {
    if avail < 8 || buf.len() < 8 {
        return W30a07BlteError::NeedBytes(8);
    }
    if buf[0..4] != *b"BLTE" {
        return W30a07BlteError::BadMagic;
    }
    let hsize = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if hsize == 0 {
        return W30a07BlteError::Complete;
    }
    if avail < 0xc || buf.len() < 0xc {
        return W30a07BlteError::NeedBytes(0xc);
    }
    let count = u32::from_be_bytes([0, buf[9], buf[10], buf[11]]);
    if buf[8] != 0x0f || count == 0 {
        return W30a07BlteError::BadTableFormat(buf[8]);
    }
    let want = count.wrapping_mul(0x18).wrapping_add(0xc);
    if hsize != want {
        return W30a07BlteError::HeaderSizeMismatch;
    }
    if want <= avail {
        return W30a07BlteError::Complete;
    }
    W30a07BlteError::NeedBytes(want)
}

/// `0x00b09e60`: over 28-byte chunk records `(a, b)` (first two u32 words,
/// cumulative sums), finds the chunks fully inside `[off, off+len)` in the
/// `a` space and returns `(min_b_start, max_b_extent)`; `None` if the range
/// extends past the total.
pub fn w30a07_chunk_range(chunks: &[(u32, u32)], off: u64, len: u64) -> Option<(u64, u64)> {
    let (mut lo, mut ext) = (u64::MAX, 0u64);
    let (mut ca, mut cb) = (0u64, 0u64);
    for &(a, b) in chunks {
        if ca >= off && ca.wrapping_add(a as u64) <= off.wrapping_add(len) {
            lo = lo.min(cb);
            let v = (b as u64).wrapping_sub(lo).wrapping_add(cb);
            ext = ext.max(v);
        }
        ca = ca.wrapping_add(a as u64);
        cb = cb.wrapping_add(b as u64);
    }
    if ca < off.wrapping_add(len) {
        return None;
    }
    Some((lo, ext))
}

#[cfg(test)]
mod tests_wave30_agent07 {
    use super::*;

    #[test]
    fn clamp() {
        let (mut a, mut b) = (5, 3);
        assert_eq!(w30a07_clamp_copy_len(&mut a, &mut b), 3);
        assert_eq!((a, b), (3, 3));
    }

    #[test]
    fn resize() {
        let (mut l, mut c) = (4, 8);
        assert_eq!(w30a07_resize16(&mut l, &mut c, 2), None);
        assert_eq!(l, 2);
        assert_eq!(w30a07_resize16(&mut l, &mut c, 6), None);
        assert_eq!(l, 6);
        assert_eq!(w30a07_resize16(&mut l, &mut c, 20), Some(20));
        assert_eq!(l, 6);
        assert_eq!(c & 0x7fff_ffff, 20);
    }

    #[test]
    fn blte() {
        let mut h = b"BLTE\0\0\0\x24\x0f\0\0\x01".to_vec();
        assert_eq!(
            w30a07_blte_header_check(&h, 4),
            W30a07BlteError::NeedBytes(8)
        );
        assert_eq!(
            w30a07_blte_header_check(&h, 12),
            W30a07BlteError::NeedBytes(0x24)
        );
        assert_eq!(
            w30a07_blte_header_check(&h, 0x30),
            W30a07BlteError::Complete
        );
        h[7] = 0x20;
        assert_eq!(
            w30a07_blte_header_check(&h, 0x30),
            W30a07BlteError::HeaderSizeMismatch
        );
        h[0] = b'X';
        assert_eq!(
            w30a07_blte_header_check(&h, 0x30),
            W30a07BlteError::BadMagic
        );
    }

    #[test]
    fn range() {
        let c = [(10, 4), (10, 6), (10, 8)];
        assert_eq!(w30a07_chunk_range(&c, 10, 10), Some((4, 6)));
        assert_eq!(w30a07_chunk_range(&c, 0, 31), None);
    }
}
