#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent12.rs
// ===========================================================================
// Wave 23 agent 12 translations (runtime helpers).

/// `0x006fc7a0`: zeroes the `0x1b00`-byte block at offset `0xd70` of the
/// runtime table; the caller supplies that block.
pub fn w23a12_clear_block(block: &mut [u8]) {
    let n = block.len().min(0x1b00);
    block[..n].fill(0);
}

/// `0x006fcea0`: with `clear_slot_kind == 0` the original calls a
/// non-returning helper on the slot's first dword (reported through
/// `on_zero`); otherwise zeroes the `0x30`-byte slot `slot` of the group.
pub fn w23a12_clear_slot(
    group: &mut [u8],
    slot: u8,
    clear_slot_kind: i32,
    on_zero: impl FnOnce(u32),
) {
    let off = slot as usize * 0x30;
    if clear_slot_kind == 0 {
        let v = group
            .get(off..off + 4)
            .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        on_zero(v);
        return;
    }
    if let Some(s) = group.get_mut(off..off + 0x30) {
        s.fill(0);
    }
}

/// `0x006fce10`: finds the smallest of 8 u16 values (scanning from index 7
/// down, strict less-than so ties keep the higher index; 0xff if none is
/// below 0xffff), clears slot `idx + 10` and stores `new_value` there.
pub fn w23a12_replace_min(row: &mut [u16; 8], new_value: u16, mut clear: impl FnMut(u8)) -> u8 {
    let mut best_idx: u8 = 0xff;
    let mut best: u16 = 0xffff;
    for i in (0..8usize).rev() {
        if row[i] < best {
            best = row[i];
            best_idx = i as u8;
        }
    }
    clear(best_idx.wrapping_add(10));
    if let Some(r) = row.get_mut(best_idx as usize) {
        *r = new_value;
    }
    best_idx
}

/// `0x006fd480`: returns 1 when the kind (low 16 bits of dword 4) is 6;
/// when it is 0xe and dword 0 is zero first runs `pre` (`FUN_006fd4c0`),
/// then forwards to `forward`.
pub fn w23a12_dispatch(
    dword0: i32,
    kind_dword: i32,
    pre: impl FnOnce(),
    forward: impl FnOnce() -> u32,
) -> u32 {
    let kind = kind_dword as i16;
    if kind == 6 {
        return 1;
    }
    if kind == 0xe && dword0 == 0 {
        pre();
    }
    forward()
}

/// `0x006fda90`: zeroes the four dwords at offset `0x10` of an object.
pub fn w23a12_zero_fields(obj: &mut [u32; 8]) {
    obj[4..8].fill(0);
}

/// `0x006ff830`: percentage scaling; returns the percent and stores the
/// count rounded to a multiple of three (minimum 3) in `out`.
pub fn w23a12_scaled_count(total: i32, part: i32, count: i32, out: &mut i32) -> i32 {
    let pct = if total == 0 { 100 } else { part * 100 / total };
    let mut v = pct * (count - 1) / 100;
    if v < 3 {
        v = 3;
    }
    let base = v / 3 * 3;
    *out = match v % 3 {
        0 => v,
        1 => base,
        _ => base + 3,
    };
    pct
}

/// `0x006fffc0`: capacity-grow policy.  `word` packs flags in the top bit
/// and capacity in the low 31 bits.  Returns the new word when growth is
/// required (then the original reallocates), else `None`.
pub fn w23a12_grow_word(word: u32, wanted: u32) -> Option<u32> {
    if (word & 0x7fff_ffff) < wanted {
        let mut n = ((word >> 1) & 0x3fff_ffff).wrapping_add(word & 0x7fff_ffff);
        if n < wanted {
            n = wanted;
        }
        Some(((word ^ n) & 0x7fff_ffff) ^ word)
    } else {
        None
    }
}

/// `0x00700350`: rearranges 52 u16 words (8 groups of 6 after a 4-word
/// head) into `out` in reverse group order, negating some words and
/// mapping others through `f` (`FUN_00700610`).
pub fn w23a12_reorder(input: &[u16; 52], out: &mut [u16; 52], f: impl Fn(u16) -> u16) {
    out[48] = f(input[0]);
    out[49] = input[1].wrapping_neg();
    out[50] = input[2].wrapping_neg();
    out[51] = f(input[3]);
    for k in 0..8usize {
        let b = 48 - 6 * k;
        let i = 4 + 6 * k;
        out[b - 1] = input[i + 1];
        out[b - 2] = input[i];
        out[b - 3] = f(input[i + 5]);
        out[b - 4] = input[i + 3].wrapping_neg();
        out[b - 5] = input[i + 4].wrapping_neg();
        out[b - 6] = f(input[i + 2]);
    }
}

#[cfg(test)]
mod tests_wave23_agent12 {
    use super::*;

    #[test]
    fn clear_and_zero() {
        let mut b = vec![1u8; 0x2000];
        w23a12_clear_block(&mut b);
        assert!(b[..0x1b00].iter().all(|&x| x == 0) && b[0x1b00] == 1);
        let mut o = [1u32; 8];
        w23a12_zero_fields(&mut o);
        assert_eq!(o, [1, 1, 1, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn slot_clear() {
        let mut g = vec![7u8; 0x90];
        w23a12_clear_slot(&mut g, 1, 1, |_| panic!());
        assert_eq!(g[0x2f], 7);
        assert!(g[0x30..0x60].iter().all(|&x| x == 0));
        assert_eq!(g[0x60], 7);
        let mut seen = 0;
        w23a12_clear_slot(&mut g, 0, 0, |v| seen = v);
        assert_eq!(seen, 0x0707_0707);
    }

    #[test]
    fn min_replace() {
        let mut r = [5, 3, 9, 3, 7, 8, 9, 9];
        let mut cleared = 0;
        let i = w23a12_replace_min(&mut r, 42, |s| cleared = s);
        assert_eq!(i, 3);
        assert_eq!(cleared, 13);
        assert_eq!(r[3], 42);
        let mut full = [0xffffu16; 8];
        assert_eq!(w23a12_replace_min(&mut full, 1, |_| {}), 0xff);
    }

    #[test]
    fn dispatch_kinds() {
        assert_eq!(w23a12_dispatch(0, 6, || panic!(), || 9), 1);
        let mut pre = false;
        assert_eq!(w23a12_dispatch(0, 0xe, || pre = true, || 9), 9);
        assert!(pre);
        assert_eq!(w23a12_dispatch(1, 0xe, || panic!(), || 4), 4);
    }

    #[test]
    fn scaled() {
        let mut o = 0;
        assert_eq!(w23a12_scaled_count(0, 5, 10, &mut o), 100);
        assert_eq!(o, 9);
        assert_eq!(w23a12_scaled_count(10, 1, 10, &mut o), 10);
        assert_eq!(o, 3);
        assert_eq!(w23a12_scaled_count(10, 5, 15, &mut o), 50);
        assert_eq!(o, 6);
    }

    #[test]
    fn grow() {
        assert_eq!(w23a12_grow_word(10, 5), None);
        assert_eq!(w23a12_grow_word(10, 100), Some(100));
        assert_eq!(w23a12_grow_word(0x8000_0010, 17), Some(0x8000_0018));
    }

    #[test]
    fn reorder() {
        let mut inp = [0u16; 52];
        for (i, v) in inp.iter_mut().enumerate() {
            *v = i as u16 + 1;
        }
        let mut out = [0u16; 52];
        w23a12_reorder(&inp, &mut out, |x| x + 100);
        assert_eq!(out[48], 101);
        assert_eq!(out[49], 0u16.wrapping_sub(2));
        assert_eq!(out[47], inp[5]);
        assert_eq!(out[46], inp[4]);
        assert_eq!(out[45], 100 + inp[9]);
        assert_eq!(out[0], 100 + inp[48]);
        assert_eq!(out[5], inp[47]);
    }
}
