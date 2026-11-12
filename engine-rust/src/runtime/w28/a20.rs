#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent20.rs
// ===========================================================================
// Wave 28 translation batch 20: render command-buffer helpers.

/// Appends a 32-bit little-endian word at `at`.
fn w28a20_put(buf: &mut [u8], at: usize, v: u32) {
    buf[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn w28a20_get(buf: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]])
}

/// Links the record at `rec_ptr` (a pointer value `base + offset`) to the end
/// of the singly linked chain whose first link lives at `head_slot`
/// (`param_1 + 0x44` in the retail layout); next pointers live at record
/// offset 4.  `base` is the buffer's address.
fn w28a20_append_chain(buf: &mut [u8], base: u32, head: &mut u32, rec_off: usize) {
    let rec_ptr = base + rec_off as u32;
    if *head == 0 {
        *head = rec_ptr;
    } else {
        let mut cur = *head;
        loop {
            let nxt = w28a20_get(buf, (cur - base) as usize + 4);
            if nxt == 0 {
                w28a20_put(buf, (cur - base) as usize + 4, rec_ptr);
                break;
            }
            cur = nxt;
        }
    }
    w28a20_put(buf, rec_off + 4, 0);
}

/// `0x00a932c0`: pushes a type-4 command record (0x24 bytes: opcode 4,
/// next 0, then the 7-dword payload) at `*cursor` and links it at the end of
/// the chain `head`.  Returns the record offset.
pub fn w28a20_push_cmd4(
    buf: &mut [u8],
    cursor: &mut usize,
    base: u32,
    head: &mut u32,
    payload: &[u32; 7],
) -> usize {
    let off = *cursor;
    w28a20_put(buf, off, 4);
    w28a20_put(buf, off + 4, 0);
    *cursor += 0x24;
    for (i, v) in payload.iter().enumerate() {
        w28a20_put(buf, off + 8 + i * 4, *v);
    }
    w28a20_append_chain(buf, base, head, off);
    off
}

/// `0x00a934b0`: pushes a type-6 command record (0xc bytes: opcode 6,
/// next 0, one dword) and links it at the end of the chain.
pub fn w28a20_push_cmd6(
    buf: &mut [u8],
    cursor: &mut usize,
    base: u32,
    head: &mut u32,
    value: u32,
) -> usize {
    let off = *cursor;
    w28a20_put(buf, off, 6);
    w28a20_put(buf, off + 4, 0);
    *cursor += 0xc;
    w28a20_put(buf, off + 8, value);
    w28a20_append_chain(buf, base, head, off);
    off
}

/// `0x00a97240`: grows a small-string style capacity word (low 31 bits =
/// capacity, bit 31 = flag).  Returns the new word when `wanted` exceeds the
/// capacity (growth is 1.5x, at least `wanted`), else `None`.
pub fn w28a20_grow_capacity(word: u32, wanted: u32) -> Option<u32> {
    let cap = word & 0x7fff_ffff;
    if cap >= wanted {
        return None;
    }
    let grown = ((word >> 1) & 0x3fff_ffff).wrapping_add(cap).max(wanted);
    Some(((word ^ grown) & 0x7fff_ffff) ^ word)
}

/// `0x00a957f0`: unlinks `target` from the singly linked list of records
/// (next index in `next`, `None` terminated) starting at `head`, then clears
/// the record's six cached fields.  Returns the new head.
pub fn w28a20_unlink_record(
    head: Option<usize>,
    next: &mut [Option<usize>],
    target: usize,
    fields: &mut [u32; 5],
) -> Option<usize> {
    let mut new_head = head;
    if head == Some(target) {
        new_head = next[target];
    } else {
        let mut cur = head;
        while let Some(c) = cur {
            if next[c] == Some(target) {
                next[c] = next[target];
                break;
            }
            cur = next[c];
        }
    }
    *fields = [0; 5];
    next[target] = None;
    new_head
}

/// `0x00a956a0`: claims the first free 0xd8-byte slot among 0x100 entries
/// (limit `0xd800` bytes).  Returns `(index, generation)`; `used` marks
/// occupied slots, `gen` holds each slot's generation (bumped to at least 1).
pub fn w28a20_alloc_slot(used: &mut [bool], gen_: &mut [u32]) -> Option<(usize, u32)> {
    let n = used.len().min(gen_.len()).min(0xd800 / 0xd8);
    let i = (0..n).find(|&i| !used[i])?;
    used[i] = true;
    gen_[i] = gen_[i].max(1);
    Some((i, gen_[i]))
}

/// `0x00a956a0` post-processing: default positive scale to 1.0, keep the
/// max scale, and fill non-positive width/height from the texture header.
pub fn w28a20_fix_slot_dims(
    scale: &mut f32,
    max_scale: &mut f32,
    w: &mut f32,
    h: &mut f32,
    tex_w: i16,
    tex_h: i16,
) {
    if *scale <= 0.0 {
        *scale = 1.0;
    }
    if *max_scale <= *scale {
        *max_scale = *scale;
    }
    if *w <= 0.0 {
        *w = f32::from(tex_w);
    }
    if *h <= 0.0 {
        *h = f32::from(tex_h);
    }
}

/// `0x00a974f0`: writes the quad index pattern used when the vertex format
/// is 1: per quad base `b` (step 4) the six shorts `b, b+1, b+2, b, b+3, b+1`
/// starting at byte `start & !1` of `indices`.  Returns the quad count
/// `(n - 1) / 6 + 1` processed (0 when `count6` is 0).
pub fn w28a20_fill_quad_indices(indices: &mut [u16], start_bytes: usize, count6: usize) -> usize {
    if count6 == 0 {
        return 0;
    }
    let quads = (count6 - 1) / 6 + 1;
    let mut p = (start_bytes & !1) / 2;
    let mut b: u16 = 0;
    for _ in 0..quads {
        let q = [b, b + 1, b + 2, b, b + 3, b + 1];
        indices[p..p + 6].copy_from_slice(&q);
        p += 6;
        b = b.wrapping_add(4);
    }
    quads
}

#[cfg(test)]
mod tests_wave28_agent20 {
    use super::*;

    #[test]
    fn cmd_chain() {
        let mut buf = vec![0u8; 0x100];
        let mut cur = 0;
        let mut head = 0u32;
        let a = w28a20_push_cmd4(
            &mut buf,
            &mut cur,
            0x1000,
            &mut head,
            &[1, 2, 3, 4, 5, 6, 7],
        );
        assert_eq!(cur, 0x24);
        assert_eq!(head, 0x1000);
        let b = w28a20_push_cmd6(&mut buf, &mut cur, 0x1000, &mut head, 9);
        assert_eq!((a, b, cur), (0, 0x24, 0x30));
        assert_eq!(w28a20_get(&buf, 4), 0x1024);
        assert_eq!(w28a20_get(&buf, 0x24), 6);
        assert_eq!(w28a20_get(&buf, 0x2c), 9);
    }

    #[test]
    fn grow() {
        assert_eq!(w28a20_grow_capacity(0x10, 8), None);
        assert_eq!(w28a20_grow_capacity(0x10, 100), Some(100));
        assert_eq!(w28a20_grow_capacity(0x8000_0010, 17), Some(0x8000_0018));
    }

    #[test]
    fn unlink() {
        let mut next = [Some(1), Some(2), None];
        let mut f = [1; 5];
        assert_eq!(w28a20_unlink_record(Some(0), &mut next, 1, &mut f), Some(0));
        assert_eq!(next[0], Some(2));
        assert_eq!(f, [0; 5]);
        assert_eq!(w28a20_unlink_record(Some(0), &mut next, 0, &mut f), Some(2));
    }

    #[test]
    fn slots() {
        let mut used = [true, false];
        let mut g = [0, 0];
        assert_eq!(w28a20_alloc_slot(&mut used, &mut g), Some((1, 1)));
        assert_eq!(w28a20_alloc_slot(&mut used, &mut g), None);
        let (mut s, mut m, mut w, mut h) = (0.0, 0.5, 0.0, 3.0);
        w28a20_fix_slot_dims(&mut s, &mut m, &mut w, &mut h, 16, 8);
        assert_eq!((s, m, w, h), (1.0, 1.0, 16.0, 3.0));
    }

    #[test]
    fn quads() {
        let mut ix = [0u16; 12];
        assert_eq!(w28a20_fill_quad_indices(&mut ix, 0, 12), 2);
        assert_eq!(&ix[..6], &[0, 1, 2, 0, 3, 1]);
        assert_eq!(&ix[6..], &[4, 5, 6, 4, 7, 5]);
        assert_eq!(w28a20_fill_quad_indices(&mut ix, 0, 0), 0);
    }
}
