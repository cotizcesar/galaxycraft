#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent06.rs
// ===========================================================================
// Wave 21 translation batch 06.

/// `0x005712d0`: applies the per-frame offset of a type-`0x132` record to
/// `(x, y)`.  `rec` is the record (u16 kind at +8, u8 frame at +0xb) and
/// `table` is the blob at the global base whose i32 at +8 is the row-table
/// offset; rows are two signed bytes `(dx, dy)`.  `flip` is `param_4 != 0`.
/// Both branches of the original reduce to `x -= dx`, `y -= dy`.
pub fn w21a06_apply_frame_offset(rec: &[u8], table: &[u8], x: &mut i32, y: &mut i32, flip: bool) {
    if rec.len() < 0xc || u16::from_le_bytes([rec[8], rec[9]]) != 0x132 {
        return;
    }
    let frame = rec[0xb] as i32;
    let v = if flip { 0x20 - frame } else { frame };
    let idx = (v + ((v >> 31) & 3)) >> 2;
    let idx = if flip { idx - 1 } else { idx };
    let base = i32::from_le_bytes([table[8], table[9], table[10], table[11]]);
    let off = (base + idx * 2) as usize;
    let dx = table[off] as i8 as i32;
    let dy = table[off + 1] as i8 as i32;
    *x += -dx;
    *y -= dy;
}

/// `0x00570600`: clamped substring length of a string whose length is at
/// `len`: `min(count, len - pos)` (unsigned, wrapping as in the original).
pub fn w21a06_substr_len(len: u32, pos: u32, count: u32) -> u32 {
    len.wrapping_sub(pos).min(count)
}

/// `0x005700e0`: insertion planning for a vector of 0x4c-byte elements.
/// Returns `(index, grow_to)`; `index = (ptr - begin) / 0x4c`.  When
/// `size == capacity` the vector must grow to `cap + max(1, cap/2)` where
/// the half is `flags >> 1 & 0x3fffffff` (`grow_to` is `Some`).
pub fn w21a06_vec76_insert_plan(ptr: u32, begin: u32, size: u32, flags: u32) -> (u32, Option<u32>) {
    let index = ptr.wrapping_sub(begin) / 0x4c;
    let cap = flags & 0x7fff_ffff;
    if size == cap && cap < cap.wrapping_add(1) {
        let half = (flags >> 1) & 0x3fff_ffff;
        let extra = if half >= 1 { half } else { 1 };
        (index, Some(cap.wrapping_add(extra)))
    } else {
        (index, None)
    }
}

/// `0x00570aa0`: pops the head of a global list: returns the current head
/// and, when non-zero, advances it via `advance`; zero head returns 0.
pub fn w21a06_take_head(head: u32, mut advance: impl FnMut()) -> u32 {
    if head == 0 {
        return 0;
    }
    advance();
    head
}

#[cfg(test)]
mod tests_wave21_agent06 {
    use super::*;

    #[test]
    fn frame_offset() {
        let mut t = vec![0u8; 0x20];
        t[8] = 0x10;
        t[0x10 + 2] = 3;
        t[0x10 + 3] = 0xfe; // dy=-2
        let mut rec = [0u8; 12];
        rec[8] = 0x32;
        rec[9] = 0x01;
        rec[0xb] = 4;
        let (mut x, mut y) = (10, 10);
        w21a06_apply_frame_offset(&rec, &t, &mut x, &mut y, false);
        assert_eq!((x, y), (7, 12));
        rec[0xb] = 28; // flip: 4>>2 -1 = 0
        t[0x10] = 1;
        t[0x11] = 1;
        let (mut x, mut y) = (0, 0);
        w21a06_apply_frame_offset(&rec, &t, &mut x, &mut y, true);
        assert_eq!((x, y), (-1, -1));
        rec[8] = 0;
        let (mut x, mut y) = (5, 5);
        w21a06_apply_frame_offset(&rec, &t, &mut x, &mut y, true);
        assert_eq!((x, y), (5, 5));
    }

    #[test]
    fn substr() {
        assert_eq!(w21a06_substr_len(10, 4, 100), 6);
        assert_eq!(w21a06_substr_len(10, 4, 2), 2);
    }

    #[test]
    fn plan() {
        assert_eq!(w21a06_vec76_insert_plan(0x4c * 2, 0, 3, 8), (2, None));
        assert_eq!(w21a06_vec76_insert_plan(0x98, 0, 8, 8), (2, Some(8 + 4)));
        assert_eq!(w21a06_vec76_insert_plan(0, 0, 0, 0), (0, Some(1)));
    }

    #[test]
    fn head() {
        let mut n = 0;
        assert_eq!(w21a06_take_head(0, || n += 1), 0);
        assert_eq!(w21a06_take_head(7, || n += 1), 7);
        assert_eq!(n, 1);
    }
}
