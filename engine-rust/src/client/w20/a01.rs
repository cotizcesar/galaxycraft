#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave20/agent01.rs
// ===========================================================================
// Wave 20 translation batch 01: arena allocator, bounded compare, palette.

/// `0x0053a3e0`: bounded byte compare of `hay[pos..]` against `other`.
/// `n = min(len - pos, max)` (unsigned wrap), `m = min(n, other_len)`;
/// the first differing byte gives `-1`/`1` (as `i32`), else `n - other_len`.
pub fn w20a01_bounded_compare(
    hay: &[u8],
    len: u32,
    pos: u32,
    max: u32,
    other: &[u8],
    other_len: u32,
) -> i32 {
    let mut n = len.wrapping_sub(pos);
    if max < n {
        n = max;
    }
    let m = n.min(other_len) as usize;
    for i in 0..m {
        let a = hay[pos as usize + i];
        let b = other[i];
        if a != b {
            return if a < b { -1 } else { 1 };
        }
    }
    n.wrapping_sub(other_len) as i32
}

/// Chunk header: `+0` capacity, `+4` used, payload at `+0xc`.
const W20A01_CHUNK_HDR: usize = 0xc;

fn w20a01_rd(chunk: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([chunk[off], chunk[off + 1], chunk[off + 2], chunk[off + 3]])
}

fn w20a01_wr(chunk: &mut [u8], off: usize, v: u32) {
    chunk[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

/// `0x00562fd0`: bump-allocate `size` bytes (rounded up to 4) from the
/// chunk.  Returns the chunk-relative offset (`0xc + old used`), `Some(0)`
/// for a zero-size request, `None` when the chunk is missing or full (the
/// retail code then calls a non-returning handler).
pub fn w20a01_arena_alloc(chunk: Option<&mut [u8]>, size: u32) -> Option<usize> {
    if size == 0 {
        return Some(0);
    }
    let chunk = chunk?;
    let rounded = size.wrapping_add(3) & !3;
    let cap = w20a01_rd(chunk, 0);
    let used = w20a01_rd(chunk, 4);
    if used.wrapping_add(rounded) <= cap {
        w20a01_wr(chunk, 4, used + rounded);
        return Some(W20A01_CHUNK_HDR + used as usize);
    }
    None
}

/// `0x00564730`: arena realloc.  `ptr` is a chunk-relative offset (0 =
/// null).  Shrinking returns `ptr`; growing the last allocation extends in
/// place, otherwise allocates a new block and copies `old_size` (rounded)
/// bytes.  `Some(0)` is a null result.
pub fn w20a01_arena_realloc(
    chunk: &mut [u8],
    ptr: usize,
    old_size: u32,
    new_size: u32,
) -> Option<usize> {
    if ptr == 0 {
        return w20a01_arena_alloc(Some(chunk), new_size);
    }
    if new_size == 0 {
        return Some(0);
    }
    let new_r = new_size.wrapping_add(3) & !3;
    let old_r = old_size.wrapping_add(3) & !3;
    if old_r < new_r {
        let cap = w20a01_rd(chunk, 0);
        let used = w20a01_rd(chunk, 4);
        let base = used.wrapping_sub(old_r);
        let grown = base.wrapping_add(new_r);
        if ptr as u32 != base.wrapping_add(W20A01_CHUNK_HDR as u32) || cap < grown {
            let n = match w20a01_arena_alloc(Some(chunk), new_r)? {
                0 => return Some(0),
                n => n,
            };
            if old_r == 0 {
                return Some(n);
            }
            chunk.copy_within(ptr..ptr + old_r as usize, n);
            return Some(n);
        }
        w20a01_wr(chunk, 4, grown);
    }
    Some(ptr)
}

/// `0x00566870`: slot-availability predicate.  `is_net` is the result of
/// the network-mode probe, `flag` the byte at `+4` of the session object,
/// `cc` the dword at `this+0xcc`.
pub fn w20a01_slot_active(param: i32, is_net: bool, flag: bool, cc: i32) -> bool {
    if param == 0 {
        return is_net;
    }
    if (param as u32).wrapping_sub(1) > 1 {
        return false;
    }
    if flag {
        return true;
    }
    if param == 1 {
        return cc == 0;
    }
    cc == 1
}

fn w20a01_rgb(r: u8, g: u8, b: u8) -> [f32; 4] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}

/// `0x005962b0`: palette index (0..=0x15) to RGBA floats.  Indices 7 and 9
/// use (0xfc,0xfc,0x38) and (0xfc,0xfc,0x7c); any other index returns the
/// four dwords of the default colour global.
pub fn w20a01_palette_color(index: u32, default: [f32; 4]) -> [f32; 4] {
    const T: [(u8, u8, u8); 0x16] = [
        (0xf4, 4, 4),
        (0xc, 0x48, 0xcc),
        (0x2c, 0xb4, 0x94),
        (0x88, 0x40, 0x9c),
        (0xf8, 0x8c, 0x14),
        (0x70, 0x30, 0x14),
        (0xcc, 0xe0, 0xd0),
        (0xfc, 0xfc, 0x38),
        (8, 0x80, 8),
        (0xfc, 0xfc, 0x7c),
        (0xec, 0xc4, 0xb0),
        (0x40, 0x68, 0xd4),
        (0x74, 0xa4, 0x7c),
        (0x72, 0x90, 0xb8),
        (0, 0xe4, 0xfc),
        (0xff, 0xc4, 0xe4),
        (0x80, 0x80, 0),
        (0xd2, 0xf5, 0x3c),
        (0, 0, 0x80),
        (0xf0, 0x32, 0xe6),
        (0x80, 0x80, 0x80),
        (0x3c, 0x3c, 0x3c),
    ];
    match T.get(index as usize) {
        Some(&(r, g, b)) => w20a01_rgb(r, g, b),
        None => default,
    }
}

/// `0x00964640`: `(*(u32*)(obj+0x34) >> 4) & 0xffffff01`.
pub fn w20a01_flags_shift(dword_at_0x34: u32) -> u32 {
    (dword_at_0x34 >> 4) & 0xffff_ff01
}

#[cfg(test)]
mod tests_wave20_agent01 {
    use super::*;

    #[test]
    fn compare() {
        assert_eq!(w20a01_bounded_compare(b"abcdef", 6, 2, 10, b"cd", 2), 2);
        assert_eq!(w20a01_bounded_compare(b"abcdef", 6, 2, 2, b"cd", 2), 0);
        assert_eq!(w20a01_bounded_compare(b"abcdef", 6, 2, 10, b"cx", 2), -1);
        assert_eq!(w20a01_bounded_compare(b"abcdef", 6, 0, 10, b"aa", 2), 1);
        assert_eq!(w20a01_bounded_compare(b"ab", 2, 0, 10, b"abcd", 4), -2);
    }

    fn chunk(cap: u32) -> Vec<u8> {
        let mut c = vec![0u8; 12 + cap as usize];
        c[..4].copy_from_slice(&cap.to_le_bytes());
        c
    }

    #[test]
    fn alloc() {
        let mut c = chunk(16);
        assert_eq!(w20a01_arena_alloc(Some(&mut c), 0), Some(0));
        assert_eq!(w20a01_arena_alloc(Some(&mut c), 5), Some(12));
        assert_eq!(w20a01_arena_alloc(Some(&mut c), 8), Some(20));
        assert_eq!(w20a01_arena_alloc(Some(&mut c), 4), None);
        assert_eq!(w20a01_arena_alloc(None, 4), None);
    }

    #[test]
    fn realloc() {
        let mut c = chunk(32);
        let p = w20a01_arena_alloc(Some(&mut c), 4).unwrap();
        c[p] = 9;
        assert_eq!(w20a01_arena_realloc(&mut c, p, 4, 12), Some(p));
        assert_eq!(w20a01_arena_realloc(&mut c, p, 12, 4), Some(p));
        let q = w20a01_arena_alloc(Some(&mut c), 4).unwrap();
        let r = w20a01_arena_realloc(&mut c, p, 4, 8).unwrap();
        assert_ne!(r, p);
        assert_eq!(c[r], 9);
        assert!(q > p);
        assert_eq!(w20a01_arena_realloc(&mut c, 0, 0, 0), Some(0));
        assert_eq!(w20a01_arena_realloc(&mut c, r, 8, 0), Some(0));
    }

    #[test]
    fn slot() {
        assert!(w20a01_slot_active(0, true, false, 5));
        assert!(!w20a01_slot_active(3, true, true, 0));
        assert!(!w20a01_slot_active(-1, true, true, 0));
        assert!(w20a01_slot_active(1, false, true, 9));
        assert!(w20a01_slot_active(1, false, false, 0));
        assert!(!w20a01_slot_active(1, false, false, 1));
        assert!(w20a01_slot_active(2, false, false, 1));
    }

    #[test]
    fn palette() {
        let d = [0.1, 0.2, 0.3, 0.4];
        assert_eq!(w20a01_palette_color(0x16, d), d);
        assert_eq!(w20a01_palette_color(0, d)[0], 0xf4 as f32 / 255.0);
        assert_eq!(w20a01_palette_color(9, d)[2], 0x7c as f32 / 255.0);
        assert_eq!(w20a01_palette_color(0x15, d)[3], 1.0);
    }

    #[test]
    fn flags() {
        assert_eq!(w20a01_flags_shift(0x10), 1);
        assert_eq!(w20a01_flags_shift(0x20), 0);
        assert_eq!(w20a01_flags_shift(0xffff_ffff), 0x0fff_ff01);
    }
}
