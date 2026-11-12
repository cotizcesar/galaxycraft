#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent03.rs
// ===========================================================================
// Wave 28 translation batch 03.
//
// Third-party codec glue (VP8 frame tag, Vorbis residue setup) plus
// zeroing helpers for fixed-size codec state blocks.

/// `0x009fa1c0`: clears a `0x208`-byte state block when present.
pub fn w28a03_clear_208(buf: Option<&mut [u8]>) {
    if let Some(b) = buf {
        let n = b.len().min(0x208);
        b[..n].fill(0);
    }
}

/// `0x009ff3e0`: clears a `0x30`-byte state block.
pub fn w28a03_clear_30(buf: &mut [u8]) {
    let n = buf.len().min(0x30);
    buf[..n].fill(0);
}

/// `0x009ff670`: clears a `0x60`-byte state block when present.
pub fn w28a03_clear_60(buf: Option<&mut [u8]>) {
    if let Some(b) = buf {
        let n = b.len().min(0x60);
        b[..n].fill(0);
    }
}

/// Result of the VP8 frame-tag parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W28a03Vp8Info {
    /// Dword at +0xc: 1 when the frame is a key frame.
    pub key_frame: u32,
    /// Dword at +4 (14-bit width), only set on key frames.
    pub width: u32,
    /// Dword at +8 (14-bit height), only set on key frames.
    pub height: u32,
}

/// `0x00a05a10`: parses a VP8 frame tag (key-frame flag, start code
/// `9d 01 2a`, 14-bit width/height).  `decrypt` optionally copies up to
/// 10 bytes through a transform first.  `None` is the non-returning
/// failure path (bad start code, zero dimension, bad range).
#[allow(clippy::type_complexity)]
pub fn w28a03_vp8_frame_info(
    data: &[u8],
    decrypt: Option<&dyn Fn(&[u8], &mut [u8; 12], usize)>,
) -> Option<W28a03Vp8Info> {
    let size = data.len();
    let mut local = [0u8; 12];
    let hdr: &[u8] = if let Some(f) = decrypt {
        f(data, &mut local, size.min(10));
        &local
    } else {
        data
    };
    let mut info = W28a03Vp8Info::default();
    if size > 9 && hdr.len() >= 10 && hdr[0] & 1 == 0 {
        info.key_frame = 1;
        if hdr[3] != 0x9d || hdr[4] != 1 || hdr[5] != 0x2a {
            return None;
        }
        let w = u16::from_le_bytes([hdr[6], hdr[7]]) as u32 & 0x3fff;
        let h = u16::from_le_bytes([hdr[8], hdr[9]]) as u32 & 0x3fff;
        info.width = w;
        info.height = h;
        if w == 0 || h == 0 {
            return None;
        }
    }
    Some(info)
}

/// Vorbis residue setup record (`0xb18`-byte block in the original).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct W28a03Residue {
    pub begin: i32,
    pub end: i32,
    pub grouping: i32,
    pub partitions: i32,
    pub groupbook: i32,
    pub part_vals: i32,
    pub cascade: Vec<u32>,
    pub booklist: Vec<i32>,
}

/// `0x00a024c0`: unpacks a residue header from a bit reader (`read(bits)`,
/// negative = EOF).  `book_count` is the codebook count; `book_info(i)`
/// returns `(dim, entries, has_codes)` for codebook `i`.  Returns `None`
/// on any validation failure.
pub fn w28a03_residue_unpack(
    mut read: impl FnMut(u32) -> i32,
    book_count: i32,
    book_info: impl Fn(usize) -> (i32, i32, bool),
) -> Option<W28a03Residue> {
    let mut r = W28a03Residue {
        begin: read(24),
        end: read(24),
        grouping: read(24).wrapping_add(1),
        partitions: read(6).wrapping_add(1),
        groupbook: read(8),
        ..Default::default()
    };
    if r.groupbook < 0 {
        return None;
    }
    let mut acc = 0i32;
    for _ in 0..r.partitions.max(0) {
        let mut v = read(3) as u32;
        let flag = read(1);
        if flag < 0 {
            return None;
        }
        if flag != 0 {
            let hi = read(5);
            if hi < 0 {
                return None;
            }
            v |= (hi as u32) << 3;
        }
        r.cascade.push(v);
        acc += v.count_ones() as i32;
    }
    for _ in 0..acc {
        let b = read(8);
        if b < 0 {
            return None;
        }
        r.booklist.push(b);
    }
    if r.groupbook >= book_count {
        return None;
    }
    for &b in &r.booklist {
        if b >= book_count || !book_info(b as usize).2 {
            return None;
        }
    }
    let (mut dim, entries, _) = book_info(r.groupbook as usize);
    let mut ac = 1i32;
    if dim > 0 {
        loop {
            ac = ac.wrapping_mul(r.partitions);
            if ac > entries {
                return None;
            }
            dim -= 1;
            if dim < 1 {
                r.part_vals = ac;
                return Some(r);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests_wave28_agent03 {
    use super::*;

    #[test]
    fn clears() {
        let mut a = [1u8; 0x300];
        w28a03_clear_208(Some(&mut a));
        assert!(a[..0x208].iter().all(|&x| x == 0) && a[0x208] == 1);
        w28a03_clear_208(None);
        let mut b = [1u8; 0x40];
        w28a03_clear_30(&mut b);
        assert!(b[..0x30].iter().all(|&x| x == 0) && b[0x30] == 1);
        let mut c = [1u8; 0x80];
        w28a03_clear_60(Some(&mut c));
        assert!(c[..0x60].iter().all(|&x| x == 0) && c[0x60] == 1);
    }

    #[test]
    fn vp8() {
        let key = [0x10, 0, 0, 0x9d, 1, 0x2a, 0x40, 0xc1, 0x20, 0];
        let i = w28a03_vp8_frame_info(&key, None).unwrap();
        assert_eq!((i.key_frame, i.width, i.height), (1, 0x140, 0x20));
        let inter = [1u8; 10];
        assert_eq!(w28a03_vp8_frame_info(&inter, None).unwrap().key_frame, 0);
        let mut bad = key;
        bad[3] = 0;
        assert!(w28a03_vp8_frame_info(&bad, None).is_none());
        let mut zero = key;
        zero[6] = 0;
        zero[7] = 0;
        assert!(w28a03_vp8_frame_info(&zero, None).is_none());
        assert_eq!(w28a03_vp8_frame_info(&key[..9], None).unwrap().key_frame, 0);
    }

    #[test]
    fn residue() {
        let vals = [0, 8, 3, 1, 0, 0b101, 0, 0, 0];
        let mut it = vals.iter().copied();
        let r = w28a03_residue_unpack(|_| it.next().unwrap_or(0), 4, |_| (2, 100, true));
        // begin 0,end 8,grouping 4,partitions 2,groupbook 0
        let r = r.unwrap();
        assert_eq!((r.end, r.grouping, r.partitions), (8, 4, 2));
        assert_eq!(r.part_vals, 4);
        let bad = w28a03_residue_unpack(|_| 0, 4, |_| (2, 100, false));
        assert!(bad.is_none() || bad.unwrap().booklist.is_empty());
        assert!(w28a03_residue_unpack(|_| -1, 4, |_| (1, 1, true)).is_none());
    }
}
