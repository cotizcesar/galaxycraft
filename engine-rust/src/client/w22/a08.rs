#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent08.rs
// ===========================================================================
// Wave 22 translation batch 08.

/// `0x0097a370`: the index guard before the two-step slot update; the update
/// runs when `index < count` (byte compare, `count` read at offset `0x60`) or
/// when the index byte is the `0xff` sentinel.
pub fn w22a08_slot_update_guard(index: u32, count_at_0x60: u8) -> bool {
    let b = index as u8;
    b < count_at_0x60 || b == 0xff
}

/// `0x006cd480`: releases a block only when the pointer is non-null; returns
/// whether the release callback was invoked.
pub fn w22a08_release_if_nonnull(ptr: usize, mut release: impl FnMut(usize)) -> bool {
    if ptr == 0 {
        return false;
    }
    release(ptr);
    true
}

/// `0x006cd300`: repacks a loaded frame-table blob.  Layout: three `u16`
/// header words (first word low 15 bits = entry count), then `count` entries
/// of 8 bytes, then a tail.  Returns the rebuilt copy (header + entries +
/// tail), or `None` when the blob is empty, has no entries, or is shorter
/// than its header (`count * 8 + 6`).
pub fn w22a08_repack_frame_table(src: &[u8]) -> Option<Vec<u8>> {
    if src.len() < 6 {
        return None;
    }
    let first = u16::from_le_bytes([src[0], src[1]]);
    if first == 0 {
        return None;
    }
    let count = (first & 0x7fff) as usize;
    let hdr = count * 8 + 6;
    if hdr > src.len() {
        return None;
    }
    let mut out = vec![0u8; src.len()];
    out[..hdr].copy_from_slice(&src[..hdr]);
    out[hdr..].copy_from_slice(&src[hdr..]);
    Some(out)
}

#[cfg(test)]
mod tests_wave22_agent08 {
    use super::*;

    #[test]
    fn guard() {
        assert!(w22a08_slot_update_guard(2, 3));
        assert!(!w22a08_slot_update_guard(3, 3));
        assert!(w22a08_slot_update_guard(0x1ff, 0));
        assert!(!w22a08_slot_update_guard(0x100, 0));
    }

    #[test]
    fn release() {
        let mut got = vec![];
        assert!(!w22a08_release_if_nonnull(0, |p| got.push(p)));
        assert!(w22a08_release_if_nonnull(7, |p| got.push(p)));
        assert_eq!(got, vec![7]);
    }

    #[test]
    fn repack() {
        let mut b = vec![0u8; 6 + 16 + 3];
        b[0] = 2;
        for (i, v) in b.iter_mut().enumerate().skip(1) {
            *v = i as u8;
        }
        b[0] = 2;
        b[1] = 0;
        assert_eq!(w22a08_repack_frame_table(&b), Some(b.clone()));
        assert_eq!(w22a08_repack_frame_table(&b[..10]), None);
        assert_eq!(w22a08_repack_frame_table(&[0, 0, 0, 0, 0, 0]), None);
        assert_eq!(w22a08_repack_frame_table(&[]), None);
    }
}
