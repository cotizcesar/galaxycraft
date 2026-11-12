#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave17/agent06.rs
// ===========================================================================
// Wave 17 translation batch 06: small sim helpers (pure reference code).

/// `0x0064f890`: forwards `(a, 0, b, len_of(b), 0)` to the helper at
/// `FUN_0064f1f0`; `len_of` models `FUN_005a5790`.
pub fn w17a06_forward_f890<R>(
    a: u32,
    b: u32,
    mut len_of: impl FnMut(u32) -> u32,
    mut inner: impl FnMut(u32, u32, u32, u32, u32) -> R,
) -> R {
    let l = len_of(b);
    inner(a, 0, b, l, 0)
}

/// `0x0064f8b0`: same as `0x0064f890` with the trailing flag set to 1.
pub fn w17a06_forward_f8b0<R>(
    a: u32,
    b: u32,
    mut len_of: impl FnMut(u32) -> u32,
    mut inner: impl FnMut(u32, u32, u32, u32, u32) -> R,
) -> R {
    let l = len_of(b);
    inner(a, 0, b, l, 1)
}

/// `0x00657b70`: reads the pointer at `this+0x134`; when non-null, its node
/// has kind byte (`+8`) == 4 and owner (`+0xc`) == `this`, and the check
/// (`FUN_005b5820`) returns 0, the non-returning handler runs.  Returns
/// true when that handler path is taken.
pub fn w17a06_node_needs_abort(
    this: u32,
    node: Option<(u8, u32)>,
    mut check_nonzero: impl FnMut() -> bool,
) -> bool {
    match node {
        Some((4, owner)) if owner == this => !check_nonzero(),
        _ => false,
    }
}

/// `0x00658dc0`: true when the byte at `this+0x4c` is below 8 (the
/// retail code then calls `FUN_00634ac0` and a non-returning helper).
pub fn w17a06_player_byte_valid(byte_4c: u8) -> bool {
    byte_4c < 8
}

/// `0x00695760`: forwards `(1, a, b)` to the helper at `FUN_00695e60`.
pub fn w17a06_fwd_95760<R>(a: u32, b: u32, mut inner: impl FnMut(u32, u32, u32) -> R) -> R {
    inner(1, a, b)
}

/// `0x00695780`: forwards `(1, a)` to the helper at `FUN_006960f0`.
pub fn w17a06_fwd_95780<R>(a: u32, mut inner: impl FnMut(u32, u32) -> R) -> R {
    inner(1, a)
}

/// `0x006b2670`: read-modify-write of a u32 at an arbitrary address over
/// 4-byte-aligned words.  Aligned: delegates to the word writer
/// (`FUN_006b2960`).  Unaligned: reads the two covering words
/// (`FUN_006b06e0`), applies `mode` (0 set, 1 wrapping add, -1 saturating
/// subtract to 0, anything else leaves the value) at byte offset
/// `addr & 3`, then writes both words back with mode 0.  Returns 1 on
/// success, 0 on any failure.
pub fn w17a06_rmw_unaligned(
    addr: u32,
    value: u32,
    mode: i32,
    mut read_word: impl FnMut(u32) -> Option<u32>,
    mut write_word: impl FnMut(u32, u32, i32) -> u32,
) -> u32 {
    let lo_addr = addr & !3;
    if addr == lo_addr {
        return write_word(addr, value, mode);
    }
    let hi_addr = addr.wrapping_add(4) & !3;
    let Some(lo) = read_word(lo_addr) else {
        return 0;
    };
    let Some(hi) = read_word(hi_addr) else {
        return 0;
    };
    let mut bytes = [0u8; 8];
    bytes[..4].copy_from_slice(&lo.to_le_bytes());
    bytes[4..].copy_from_slice(&hi.to_le_bytes());
    let off = (addr - lo_addr) as usize;
    let cur = u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]]);
    let new = match mode {
        0 => value,
        1 => cur.wrapping_add(value),
        -1 => cur.saturating_sub(value),
        _ => cur,
    };
    bytes[off..off + 4].copy_from_slice(&new.to_le_bytes());
    let lo = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let hi = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if write_word(lo_addr, lo, 0) == 0 {
        return 0;
    }
    if write_word(hi_addr, hi, 0) == 0 {
        return 0;
    }
    1
}

/// `0x006c2170`: builds the i16 rect `[x-r, y-r, x+r, y+r]`, converts both
/// coordinates with `tile` (`FUN_0058b820`, axis 0 = x table, 1 = y table,
/// flag 1 in retail) and calls `FUN_006c2250` with
/// `(0, x, y, tx-1, ty-1, tx, ty, rect, p4, p5)`.
pub fn w17a06_rect_query<R>(
    x: u32,
    y: u32,
    r: i16,
    p4: u32,
    p5: u32,
    mut tile: impl FnMut(u8, i32) -> i32,
    mut inner: impl FnMut(u32, u32, u32, i32, i32, i32, i32, [i16; 4], u32, u32) -> R,
) -> R {
    let sx = x as i16;
    let sy = y as i16;
    let rect = [
        sx.wrapping_sub(r),
        sy.wrapping_sub(r),
        sx.wrapping_add(r),
        sy.wrapping_add(r),
    ];
    let tx = tile(0, sx as i32);
    let ty = tile(1, sy as i32);
    inner(
        0,
        x,
        y,
        tx.wrapping_sub(1),
        ty.wrapping_sub(1),
        tx,
        ty,
        rect,
        p4,
        p5,
    )
}

/// `0x006c21f0`: like `0x006c2170` but without a radius; the rect argument
/// is the caller's `p3` passed through unchanged.
pub fn w17a06_tile_query<R>(
    x: u32,
    y: u32,
    p3: u32,
    p4: u32,
    p5: u32,
    mut tile: impl FnMut(u8, i32) -> i32,
    mut inner: impl FnMut(u32, u32, u32, i32, i32, i32, i32, u32, u32, u32) -> R,
) -> R {
    let tx = tile(0, x as i16 as i32);
    let ty = tile(1, y as i16 as i32);
    inner(
        0,
        x,
        y,
        tx.wrapping_sub(1),
        ty.wrapping_sub(1),
        tx,
        ty,
        p3,
        p4,
        p5,
    )
}

/// `0x00686860`: classifies a `w x h` block of map cells starting at
/// `(x, y)` into `out[(row + slot*8)*6 + col]` (stride 6; values 0 free,
/// 2 blocked by overlap, 4 blocked by cell flags) and returns the maximum.
/// `size` packs width (low i16) and height (high i16); `cells` is the
/// u32 map (row width `map_w`, index `map_w*y + x`); `id_flags` is the
/// per-id u32 table entry (bit 0x1000 enables the overlap test) and
/// `id_byte` the per-id byte (bit 0 selects mask 0xa2/0xa3 << 22).
/// `overlap(rect)` models `FUN_006c1f70(rect, callback, 0)`.  Cells or
/// outputs outside the slices read as 0 / are skipped.
#[allow(clippy::too_many_arguments)]
pub fn w17a06_classify_block(
    id: u32,
    x: u16,
    y: i16,
    size: u32,
    slot: i32,
    map_w: u16,
    cells: &[u32],
    id_flags: u32,
    id_byte: u8,
    out: &mut [u8],
    mut overlap: impl FnMut([i16; 4]) -> bool,
) -> u8 {
    let rows = (size >> 16) as i16;
    let cols = size as i16;
    let id16 = id & 0xffff;
    let range_id = id.wrapping_sub(0xc2) & 0xffff;
    let check_overlap = id_flags & 0x1000 != 0;
    let mask = ((u32::from(!id_byte) & 1) + 0xa2).wrapping_mul(0x400000);
    let mut idx: i64 = i64::from(map_w) * i64::from(y) + i64::from(x as i16);
    let _ = id16;
    let mut best = 0u8;
    if rows < 1 {
        return 0;
    }
    let mut ypix = y.wrapping_add(4).wrapping_mul(0x20);
    for row in 0..rows as i32 {
        if cols > 0 {
            let base = ((row + slot.wrapping_mul(8)) as i64) * 6;
            let mut xpix = (((u32::from(x) + 4) * 0x20) & 0xffff) as i16;
            for col in 0..cols as i64 {
                let cell = if idx >= 0 {
                    cells.get(idx as usize).copied().unwrap_or(0)
                } else {
                    0
                };
                let mut v = 0u8;
                if (mask & cell) == 0 || (range_id < 6 && (cell as i32) < 0) {
                    if check_overlap {
                        let rect = [xpix.wrapping_sub(0xe0), ypix.wrapping_sub(0xe0), xpix, ypix];
                        if overlap(rect) {
                            v = 2;
                        }
                    }
                } else {
                    v = 4;
                }
                idx += 1;
                xpix = xpix.wrapping_add(0x20);
                let o = base + col;
                if o >= 0 && (o as usize) < out.len() {
                    out[o as usize] = v;
                }
                best = best.max(v);
            }
        }
        ypix = ypix.wrapping_add(0x20);
        idx += i64::from(map_w) - i64::from(cols);
    }
    best
}

#[cfg(test)]
mod tests_wave17_agent06 {
    use super::*;

    #[test]
    fn forwards() {
        let r = w17a06_forward_f890(1, 2, |b| b + 10, |a, z, b, l, f| [a, z, b, l, f]);
        assert_eq!(r, [1, 0, 2, 12, 0]);
        let r = w17a06_forward_f8b0(1, 2, |b| b + 10, |a, z, b, l, f| [a, z, b, l, f]);
        assert_eq!(r[4], 1);
        assert_eq!(w17a06_fwd_95760(5, 6, |a, b, c| [a, b, c]), [1, 5, 6]);
        assert_eq!(w17a06_fwd_95780(5, |a, b| [a, b]), [1, 5]);
    }

    #[test]
    fn predicates() {
        assert!(w17a06_node_needs_abort(7, Some((4, 7)), || false));
        assert!(!w17a06_node_needs_abort(7, Some((4, 7)), || true));
        assert!(!w17a06_node_needs_abort(7, Some((3, 7)), || false));
        assert!(!w17a06_node_needs_abort(7, Some((4, 8)), || false));
        assert!(!w17a06_node_needs_abort(7, None, || false));
        assert!(w17a06_player_byte_valid(7));
        assert!(!w17a06_player_byte_valid(8));
    }

    #[test]
    fn rmw() {
        let mut mem = [0u8; 16];
        mem[..4].copy_from_slice(&0x0403_0201u32.to_le_bytes());
        let run = |mem: &mut [u8; 16], addr, v, mode| {
            let m = std::cell::RefCell::new(*mem);
            let r = w17a06_rmw_unaligned(
                addr,
                v,
                mode,
                |a| {
                    let m = m.borrow();
                    let a = a as usize;
                    Some(u32::from_le_bytes(m[a..a + 4].try_into().unwrap()))
                },
                |a, v, _| {
                    let a = a as usize;
                    m.borrow_mut()[a..a + 4].copy_from_slice(&v.to_le_bytes());
                    1
                },
            );
            *mem = *m.borrow();
            r
        };
        assert_eq!(run(&mut mem, 1, 0xAABBCCDD, 0), 1);
        assert_eq!(mem[..8], [0x01, 0xDD, 0xCC, 0xBB, 0xAA, 0, 0, 0]);
        assert_eq!(run(&mut mem, 1, 1, 1), 1);
        assert_eq!(mem[1], 0xDE);
        assert_eq!(run(&mut mem, 1, 0xFFFF_FFFF, -1), 1);
        assert_eq!(mem[1..5], [0, 0, 0, 0]);
        assert_eq!(w17a06_rmw_unaligned(1, 0, 0, |_| None, |_, _, _| 1), 0);
        assert_eq!(
            w17a06_rmw_unaligned(4, 9, 1, |_| None, |a, v, m| a + v + m as u32),
            14
        );
    }

    #[test]
    fn rects() {
        let r = w17a06_rect_query(
            10,
            20,
            3,
            8,
            9,
            |ax, v| v * 2 + ax as i32,
            |_, _, _, a, b, c, d, rc, p, q| (a, b, c, d, rc, p, q),
        );
        assert_eq!(r, (19, 40, 20, 41, [7, 17, 13, 23], 8, 9));
        let r = w17a06_tile_query(
            1,
            2,
            3,
            4,
            5,
            |_, v| v,
            |_, _, _, a, b, c, d, p3, _, _| (a, b, c, d, p3),
        );
        assert_eq!(r, (0, 1, 1, 2, 3));
    }

    #[test]
    fn classify() {
        // 2x2 block at (0,0), map width 3; cell 1 has the blocking bit.
        let cells = [0u32, 0xA2 << 22, 0, 0, 0, 0, 0, 0, 0];
        let mut out = [9u8; 64];
        let size = (2u32 << 16) | 2;
        let m = w17a06_classify_block(0x10, 0, 0, size, 1, 3, &cells, 0, 0, &mut out, |_| true);
        assert_eq!(m, 4);
        assert_eq!(out[48], 0);
        assert_eq!(out[49], 4);
        // overlap enabled: free cells report 2 and rect is passed
        let mut seen = vec![];
        let m = w17a06_classify_block(
            0x10,
            0,
            0,
            size,
            0,
            3,
            &[0u32; 9],
            0x1000,
            1,
            &mut out,
            |r| {
                seen.push(r);
                true
            },
        );
        assert_eq!(m, 2);
        assert_eq!(
            seen[0],
            [(4 * 0x20) - 0xe0, (4 * 0x20) - 0xe0, 4 * 0x20, 4 * 0x20]
        );
        assert_eq!(
            w17a06_classify_block(0, 0, 0, 0, 0, 3, &cells, 0, 0, &mut out, |_| true),
            0
        );
    }
}
