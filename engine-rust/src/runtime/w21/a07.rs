#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent07.rs
// ===========================================================================
// Wave 21 agent 07: image (iscript sprite) record helpers.

fn w21a07_u16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn w21a07_set_u16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

/// `0x005720a0`: draw-function table lookup; ids below `0x12` index the
/// table (stride 2 pointers in retail), otherwise "Invalid Draw RLE".
pub fn w21a07_draw_rle_entry<T: Copy>(table: &[T], id: u8) -> Option<T> {
    if id < 0x12 {
        table.get(id as usize).copied()
    } else {
        None
    }
}

/// `0x005720d0`: reads the signed (x, y) frame offset pair at
/// `frame_table[frame] + index * 2` of an images blob; x is negated when
/// bit 1 of the image flags (`flags_lo`) is set (horizontal flip).
pub fn w21a07_frame_offset_flip(blob: &[u8], frame: usize, index: i32, flags_lo: u8) -> (i32, i32) {
    let base = u32::from_le_bytes(blob[8 + frame * 4..12 + frame * 4].try_into().unwrap());
    let o = (base as i32).wrapping_add(index.wrapping_mul(2)) as usize;
    let mut x = blob[o] as i8 as i32;
    if flags_lo & 2 != 0 {
        x = -x;
    }
    (x, blob[o + 1] as i8 as i32)
}

/// `0x00572330`: same as the flip variant without negation.
pub fn w21a07_frame_offset(blob: &[u8], frame: usize, index: i32) -> (i32, i32) {
    w21a07_frame_offset_flip(blob, frame, index, 0)
}

/// `0x00572160`: 1-based image id to record address (stride `0x40`), 0 stays 0.
pub fn w21a07_image_addr(base: i32, id: i32) -> i32 {
    if id != 0 {
        base.wrapping_sub(0x40).wrapping_add(id.wrapping_mul(0x40))
    } else {
        0
    }
}

/// `0x00572190`: record address to 1-based id (arithmetic shift by 6).
pub fn w21a07_image_id(base: i32, addr: i32) -> i32 {
    if addr != 0 {
        (addr.wrapping_sub(base) >> 6).wrapping_add(1)
    } else {
        0
    }
}

/// `0x005727b0`: initialises an image record (`this`, >= 0x40 bytes).
/// `flag_a`/`flag_b` are the per-image bytes of two tables (bit 0 used),
/// `dat_kind`/`dat_param` the type byte and its lookup values.
#[allow(clippy::too_many_arguments)]
pub fn w21a07_init_image(
    this: &mut [u8],
    image_id: u16,
    param_2: u32,
    param_3: u8,
    param_4: u8,
    cached_ptr: u32,
    flag_a: u8,
    flag_b: u8,
    kind: u8,
    byte_at_param2_10: u8,
    kind9_value: u32,
) {
    w21a07_set_u16(this, 8, image_id);
    this[0x2c..0x30].copy_from_slice(&cached_ptr.to_le_bytes());
    let f = ((flag_a & 1) as u16) << 3;
    w21a07_set_u16(this, 0xc, f);
    w21a07_set_u16(this, 0xc, (((flag_b & 1) as u16) << 5) | f);
    this[0x18..0x1c].fill(0);
    this[0xb] = 0;
    this[0xe] = param_3;
    this[0xf] = param_4;
    this[0x3c..0x40].copy_from_slice(&param_2.to_le_bytes());
    this[0x24..0x2c].fill(0);
    this[0x30..0x34].fill(0);
    this[0x10..0x18].fill(0);
    if kind == 0x0e {
        this[0x30..0x34].copy_from_slice(&(byte_at_param2_10 as u32).to_le_bytes());
    }
    if kind == 9 {
        this[0x30..0x34].copy_from_slice(&kind9_value.to_le_bytes());
    }
}

/// `0x00572b00`: per-image count of non-`0x7f,0x7f` offset pairs among the
/// 22 pairs ending at `blob[8]`-relative `0x2c`, doubled (byte truncated).
/// Returns a 1000-entry table indexed by 1-based image id (`[0]` unused).
#[allow(clippy::needless_range_loop)]
pub fn w21a07_image_extent_table(blobs: &[Option<&[u8]>]) -> Vec<u8> {
    let mut out = vec![0u8; 1000];
    for id in 1..=999usize {
        let Some(blob) = blobs.get(id - 1).copied().flatten() else {
            continue;
        };
        let base = u32::from_le_bytes(blob[8..12].try_into().unwrap()) as usize;
        let mut n = 0u32;
        let mut k = 0x2cusize;
        while k != 0 {
            let p = k + base;
            if blob[p - 2] != 0x7f || blob[p - 1] != 0x7f {
                n += 1;
            }
            k -= 2;
        }
        if n != 0 {
            out[id] = (n as u8).wrapping_mul(2);
        }
    }
    out
}

/// `0x005748d0`: marks the record dirty (flag bit 0), stores the type byte
/// at `+0xa`; type `0x11` also sets the u16 at `+0x30` to `0x230`.
pub fn w21a07_set_type(this: &mut [u8], kind: u8) {
    let f = w21a07_u16(this, 0xc) | 1;
    w21a07_set_u16(this, 0xc, f);
    this[0xa] = kind;
    if kind == 0x11 {
        w21a07_set_u16(this, 0x30, 0x230);
    }
}

/// `0x00574980`: updates bytes `+0xe`/`+0xf`; sets the dirty bit only on change.
pub fn w21a07_set_pair(this: &mut [u8], a: u8, b: u8) {
    if this[0xe] != a || this[0xf] != b {
        let f = w21a07_u16(this, 0xc) | 1;
        w21a07_set_u16(this, 0xc, f);
        this[0xe] = a;
        this[0xf] = b;
    }
}

/// `0x00577170`: inserts node `new` before node `this` in a doubly linked
/// list; nodes hold `[next, prev]`.  `head` is rewritten when it was `this`.
pub fn w21a07_list_insert_before(
    nodes: &mut [[Option<usize>; 2]],
    this: usize,
    new: usize,
    head: &mut Option<usize>,
) {
    if *head == Some(this) {
        *head = Some(new);
    }
    let prev = nodes[this][1];
    nodes[new][0] = Some(this);
    nodes[new][1] = prev;
    if let Some(p) = prev {
        nodes[p][0] = Some(new);
    }
    nodes[this][1] = Some(new);
}

#[cfg(test)]
mod tests_wave21_agent07 {
    use super::*;

    #[test]
    fn draw_rle_and_ids() {
        let t = [1u8, 2, 3];
        assert_eq!(w21a07_draw_rle_entry(&t, 2), Some(3));
        assert_eq!(w21a07_draw_rle_entry(&t, 0x12), None);
        assert_eq!(w21a07_image_addr(0x1000, 0), 0);
        assert_eq!(w21a07_image_addr(0x1000, 3), 0x1080);
        assert_eq!(w21a07_image_id(0x1000, 0x1080), 3);
        assert_eq!(w21a07_image_id(0x1000, 0), 0);
    }

    fn blob() -> Vec<u8> {
        let mut b = vec![0u8; 0x80];
        b[8..12].copy_from_slice(&0x20u32.to_le_bytes());
        b[0x22] = 5;
        b[0x23] = 0xfe;
        b
    }

    #[test]
    fn frame_offsets() {
        let b = blob();
        assert_eq!(w21a07_frame_offset(&b, 0, 1), (5, -2));
        assert_eq!(w21a07_frame_offset_flip(&b, 0, 1, 2), (-5, -2));
        assert_eq!(w21a07_frame_offset_flip(&b, 0, 1, 0), (5, -2));
    }

    #[test]
    fn init_and_setters() {
        let mut r = vec![0xffu8; 0x40];
        w21a07_init_image(&mut r, 7, 0x1234, 3, 4, 9, 1, 1, 9, 0, 0x55);
        assert_eq!(w21a07_u16(&r, 8), 7);
        assert_eq!(w21a07_u16(&r, 0xc), 0x28);
        assert_eq!(r[0xe], 3);
        assert_eq!(r[0x30], 0x55);
        w21a07_init_image(&mut r, 7, 0x1234, 3, 4, 9, 0, 0, 0x0e, 0x42, 0);
        assert_eq!(r[0x30], 0x42);
        assert_eq!(w21a07_u16(&r, 0xc), 0);
        w21a07_set_pair(&mut r, 3, 4);
        assert_eq!(w21a07_u16(&r, 0xc), 0);
        w21a07_set_pair(&mut r, 3, 5);
        assert_eq!(w21a07_u16(&r, 0xc), 1);
        w21a07_set_type(&mut r, 0x11);
        assert_eq!(w21a07_u16(&r, 0x30), 0x230);
        assert_eq!(r[0xa], 0x11);
    }

    #[test]
    fn extent_table() {
        let mut b = [0x7fu8; 0x80];
        b[8..12].copy_from_slice(&0x20u32.to_le_bytes());
        b[0x20 + 0x2a] = 0;
        let blobs = [Some(&b[..]), None];
        let t = w21a07_image_extent_table(&blobs);
        assert_eq!(t[1], 2);
        assert_eq!(t[2], 0);
        assert_eq!(t.len(), 1000);
    }

    #[test]
    fn list_insert() {
        let mut n = vec![[None; 2]; 3];
        let mut head = Some(0);
        w21a07_list_insert_before(&mut n, 0, 1, &mut head);
        assert_eq!(head, Some(1));
        assert_eq!(n[1][0], Some(0));
        assert_eq!(n[0][1], Some(1));
        w21a07_list_insert_before(&mut n, 0, 2, &mut head);
        assert_eq!(n[1][0], Some(2));
        assert_eq!(n[2][1], Some(1));
        assert_eq!(head, Some(1));
    }
}
