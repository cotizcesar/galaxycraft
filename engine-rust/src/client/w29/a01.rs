#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent01.rs
// ===========================================================================
// Wave 29 translation batch 01: GL vertex-buffer helpers.

/// `0x00a98340`: classifies a value: below 11 gives 2, otherwise 1 when
/// below 31 and 0 when not.
pub fn w29a01_classify_small(v: u16) -> u8 {
    if v < 0xb { 2 } else { u8::from(v < 0x1f) }
}

/// `0x00a98750`: element count per draw mode (0 and 1 give 4, 2 gives 10,
/// 3 gives 2).  Other modes raise "Unknown DrawMode" and yield `None`.
pub fn w29a01_draw_mode_size(mode: u32) -> Option<u32> {
    match mode {
        0 | 1 => Some(4),
        2 => Some(10),
        3 => Some(2),
        _ => None,
    }
}

/// Vertex stride in bytes from the flag word used by `0x00a98830` and
/// `0x00a97dd0` (bits 0..3 and 5 add 8, bit 4 adds 4).
pub fn w29a01_vertex_stride(flags: u32) -> u32 {
    let mut s = (flags & 1) * 8;
    if flags & 2 != 0 {
        s += 8;
    }
    if flags & 4 != 0 {
        s += 8;
    }
    if flags & 8 != 0 {
        s += 8;
    }
    if flags & 0x10 != 0 {
        s += 4;
    }
    if flags & 0x20 != 0 {
        s += 8;
    }
    s
}

/// Vertex arena used by `0x00a98830`.
#[derive(Debug, Clone, Default)]
pub struct W29a01Arena {
    pub capacity: u32,
    pub used: u32,
    pub growable: bool,
    pub free: Vec<(u32, u32)>,
    pub dirty: bool,
    pub padding_total: u32,
    pub free_total: u32,
}

/// `0x00a98830`: allocates `count` vertices of the flag-derived stride,
/// first-fit from the free list (aligned to the stride), otherwise from the
/// tail, doubling capacity when growable.  Returns the byte offset, or
/// `None` for "No vertex flags present".
pub fn w29a01_arena_alloc(a: &mut W29a01Arena, flags: u32, count: u32) -> Option<u32> {
    let stride = w29a01_vertex_stride(flags);
    if stride == 0 {
        return None;
    }
    let size = stride.wrapping_mul(count);
    let mut i = 0;
    while i < a.free.len() {
        let (start, end) = a.free[i];
        let pad = if start % stride != 0 {
            stride - start % stride
        } else {
            0
        };
        if size.wrapping_add(pad).wrapping_add(start) <= end {
            let off = pad + start;
            let new_start = size + off;
            a.free[i].0 = new_start;
            if off != 0 {
                if end - new_start < 0x80 {
                    a.free.remove(i);
                }
                a.free_total = a.free_total.wrapping_sub(size);
                return Some(off);
            }
        }
        i += 1;
    }
    let used = a.used;
    let pad = if used % stride != 0 {
        stride - used % stride
    } else {
        0
    };
    let off = used + pad;
    a.padding_total += pad;
    a.used = size + off;
    while a.capacity.wrapping_mul(4) < a.used {
        if a.growable {
            a.capacity *= 2;
        } else {
            break;
        }
        a.dirty = true;
    }
    Some(off)
}

/// `0x00a987c0`: appends a 16-byte record to a byte arena, bumping the
/// logical length by 16 and doubling capacity (when growable) until
/// `capacity * 4` covers it.  Returns the offset written.
pub fn w29a01_push16(
    data: &mut Vec<u8>,
    len: &mut u32,
    capacity: &mut u32,
    growable: bool,
    dirty: &mut bool,
    rec: [u32; 4],
) -> u32 {
    let off = *len;
    *len = off + 0x10;
    while capacity.wrapping_mul(4) < *len {
        if growable {
            *capacity *= 2;
        } else {
            *dirty = true;
            break;
        }
        *dirty = true;
    }
    let at = (off & !3) as usize;
    if data.len() < at + 16 {
        data.resize(at + 16, 0);
    }
    for (k, w) in rec.iter().enumerate() {
        data[at + k * 4..at + k * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    off
}

/// `0x00a99350`: resizes a `u32` vector to `n` entries (truncate, or extend
/// with zeros).
pub fn w29a01_resize_u32(v: &mut Vec<u32>, n: usize) {
    v.resize(n, 0);
}

/// `0x00a99480`: resizes a vector of 8-byte pairs to `n` entries (truncate,
/// or extend with zero pairs).
pub fn w29a01_resize_pairs(v: &mut Vec<(u32, u32)>, n: usize) {
    v.resize(n, (0, 0));
}

#[cfg(test)]
mod tests_wave29_agent01 {
    use super::*;

    #[test]
    fn small_and_modes() {
        assert_eq!(w29a01_classify_small(10), 2);
        assert_eq!(w29a01_classify_small(11), 1);
        assert_eq!(w29a01_classify_small(30), 1);
        assert_eq!(w29a01_classify_small(31), 0);
        assert_eq!(w29a01_draw_mode_size(1), Some(4));
        assert_eq!(w29a01_draw_mode_size(2), Some(10));
        assert_eq!(w29a01_draw_mode_size(3), Some(2));
        assert_eq!(w29a01_draw_mode_size(4), None);
    }

    #[test]
    fn stride_and_alloc() {
        assert_eq!(w29a01_vertex_stride(0x3f), 44);
        assert_eq!(w29a01_vertex_stride(0x10), 4);
        let mut a = W29a01Arena {
            capacity: 16,
            growable: true,
            ..Default::default()
        };
        assert_eq!(w29a01_arena_alloc(&mut a, 0, 1), None);
        assert_eq!(w29a01_arena_alloc(&mut a, 1, 3), Some(0));
        assert_eq!(a.used, 24);
        assert_eq!(w29a01_arena_alloc(&mut a, 0x10, 40), Some(24));
        assert_eq!(a.capacity, 64);
        assert!(a.dirty);
        a.free.push((8, 1000));
        a.used = 2000;
        a.free_total = 5000;
        assert_eq!(w29a01_arena_alloc(&mut a, 1, 2), Some(8));
        assert_eq!(a.free[0].0, 24);
        assert_eq!(a.free_total, 4984);
    }

    #[test]
    fn push_and_resize() {
        let mut d = Vec::new();
        let (mut len, mut cap, mut dirty) = (0, 2, false);
        let o = w29a01_push16(&mut d, &mut len, &mut cap, true, &mut dirty, [1, 2, 3, 4]);
        assert_eq!((o, len, cap, dirty), (0, 16, 4, true));
        assert_eq!(&d[4..8], &2u32.to_le_bytes());
        let mut v = vec![1, 2, 3];
        w29a01_resize_u32(&mut v, 5);
        assert_eq!(v, [1, 2, 3, 0, 0]);
        w29a01_resize_u32(&mut v, 1);
        assert_eq!(v, [1]);
        let mut p = vec![(1, 2)];
        w29a01_resize_pairs(&mut p, 2);
        assert_eq!(p, [(1, 2), (0, 0)]);
    }
}
