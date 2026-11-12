#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave18/agent10.rs
// ===========================================================================
// Wave 18 translation batch 10: unaligned reads, region search, tile rects.

/// Signed divide by 32 truncating toward zero (the `(v + (v >> 31 & 0x1f)) >> 5` idiom).
pub fn w18a10_div32(v: i32) -> i32 {
    (v + ((v >> 31) & 0x1f)) >> 5
}

/// `0x006b0350`: reads a little-endian u32 at an arbitrary address through an
/// aligned-dword reader.  Aligned: one read.  Unaligned: reads the dwords at
/// `a & !3` and `(a + 4) & !3` (both must succeed) and extracts the 4 bytes.
pub fn w18a10_read_u32_unaligned(
    addr: u32,
    mut read_aligned: impl FnMut(u32) -> Option<u32>,
) -> Option<u32> {
    let base = addr & !3;
    if addr == base {
        return read_aligned(addr);
    }
    let lo = read_aligned(base)?;
    let hi = read_aligned(addr.wrapping_add(4) & !3)?;
    let wide = (lo as u64) | ((hi as u64) << 32);
    Some((wide >> ((addr - base) * 8)) as u32)
}

/// `0x00776990`: removes `value` from one 12-entry row (u32 slots, zero
/// terminated) of the 8-row table at stride 12.  Finds the first zero slot
/// (`count`, 12 if none) and the last slot equal to `value`; the tail is
/// shifted down and the final live slot cleared.  No-op if `row >= 8`,
/// `value == 0`, the row is empty or the value is absent.
pub fn w18a10_remove_row_entry(table: &mut [u32], row: i32, value: u32) {
    if !(0..8).contains(&row) {
        return;
    }
    let r = &mut table[row as usize * 12..row as usize * 12 + 12];
    let count = r.iter().position(|&v| v == 0).unwrap_or(12);
    let last = r[..count].iter().rposition(|&v| v == value);
    let Some(last) = last else { return };
    if value == 0 || count == 0 {
        return;
    }
    let end = count - 1;
    if last < end {
        r.copy_within(last + 1..=end, last);
    }
    r[end] = 0;
}

/// `0x0078fdd0`: maps an index through a u16 table; ids 0x26 and 0x3c are
/// replaced by `alt_26` / `alt_3c` when the map version (`+0x154`) exceeds
/// 0x1f / 0x1b respectively; otherwise the table value is returned.
pub fn w18a10_versioned_table_lookup(
    table: &[u16],
    index: u16,
    version: u16,
    alt_26: u16,
    alt_3c: u16,
) -> u16 {
    let v = table[index as usize];
    if v == 0x26 && version > 0x1f {
        alt_26
    } else if v == 0x3c && version > 0x1b {
        alt_3c
    } else {
        v
    }
}

/// `0x00983600`: visits the 9 neighbour offsets (`offsets[i] = (dx, dy)`)
/// around `(x, y)` inside a `w` x `h` grid, calling
/// `cb(i, cell_index, nx, ny)` for in-bounds cells and stopping at the first
/// zero result.  Returns the index where it stopped (9 if exhausted).
pub fn w18a10_visit_neighbours(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    offsets: &[(i32, i32); 9],
    mut cb: impl FnMut(u32, i32, i32, i32) -> i32,
) -> u32 {
    for (i, &(dx, dy)) in offsets.iter().enumerate() {
        let nx = dx + x;
        let ny = dy + y;
        if nx >= 0 && nx < w && ny >= 0 && ny < h && cb(i as u32, ny * w + nx, nx, ny) == 0 {
            return i as u32;
        }
    }
    9
}

/// `0x00983810`: tile rectangle `[x0, y0, x1, y1]` touched by an object of
/// id `id` at pixel `(px, py)`.  `wide` is the caller's flag for ids 0x83 and
/// 0x8f (ids 0x84/0x85/0x90/0x92 force it to 1, others to 0).  Wide mode uses
/// +-0x140 x +-200 pixels; otherwise `dims` (pixel w, h from the id table) is
/// centred.  Min clamps to 0, max to map-1 (the max clamp is skipped when
/// the min was negative).  Returns `(wide, rect)`.
pub fn w18a10_object_tile_rect(
    id: u16,
    px: i32,
    py: i32,
    wide: bool,
    dims: (i16, i16),
    map_w: i32,
    map_h: i32,
) -> (bool, [i32; 4]) {
    let wide = match id {
        0x83 | 0x8f => wide,
        0x84 | 0x85 | 0x90 | 0x92 => true,
        _ => false,
    };
    let mut r = [0i32; 4];
    if wide {
        r[0] = w18a10_div32(px - 0x140);
        r[2] = w18a10_div32(px + 0x140);
        r[1] = w18a10_div32(py - 200);
        r[3] = w18a10_div32(py + 200);
    } else {
        let tw = w18a10_div32(dims.0 as i32);
        let th = w18a10_div32(dims.1 as i32);
        r[0] = w18a10_div32(px) - tw / 2;
        r[2] = tw - 1 + r[0];
        r[1] = w18a10_div32(py) - th / 2;
        r[3] = th - 1 + r[1];
    }
    if r[0] < 0 {
        r[0] = 0;
    } else if map_w <= r[2] {
        r[2] = map_w - 1;
    }
    if r[1] < 0 {
        r[1] = 0;
    } else if map_h <= r[3] {
        r[3] = map_h - 1;
    }
    (wide, r)
}

/// `0x00983f30`: takes the u16 at `src` (if non-zero) into `dst`, clears
/// `src`, sets `flag = 1` and returns true; otherwise returns false.
pub fn w18a10_take_nonzero(src: &mut u16, flag: &mut u32, dst: &mut u16) -> bool {
    if *src == 0 {
        return false;
    }
    *dst = *src;
    *src = 0;
    *flag = 1;
    true
}

/// `0x0079fef0`: converts a pixel rect `[l, t, r, b]` into the tile-space
/// arguments `[l/32, t/32, (r-l+31)/32, (b-t+31)/32]` (trunc toward zero)
/// passed on to the indirect callee.
pub fn w18a10_rect_to_tile_args(rect: [i32; 4]) -> [i32; 4] {
    let w = rect[2] - rect[0] + 0x1f;
    let h = rect[3] - rect[1] + 0x1f;
    [
        w18a10_div32(rect[0]),
        w18a10_div32(rect[1]),
        w18a10_div32(w),
        w18a10_div32(h),
    ]
}

/// `0x00766ec0`: moves a unit position `pos = [x, y]` (+0x18/+0x1a) into
/// region `region`.  The position is clamped to `rect = [l, t, r, b]`, then
/// square rings (size 1, 3, .. 15; 4 sides with per-side steps `dx`/`dy`,
/// u16 wrapping) are scanned around its tile for a tile in-bounds
/// (`map_tiles`) whose `region_at(px, py)` equals `region`; the hit is
/// snapped to the tile centre (+0x10).  If none is found, `on_fail` runs
/// (callee 0x75_d660 in the retail binary).  `*flag` (+0x1f) is set to 1
/// and true returned in all cases.
#[allow(clippy::too_many_arguments)]
pub fn w18a10_move_to_region(
    pos: &mut [i16; 2],
    flag: &mut u8,
    rect: [i16; 4],
    region: i16,
    map_tiles: (u16, u16),
    dx: [i16; 4],
    dy: [i16; 4],
    mut region_at: impl FnMut(i32, i32) -> i16,
    mut on_fail: impl FnMut(&mut [i16; 2]),
) -> bool {
    let y = if pos[1] < rect[1] || rect[3] < pos[1] {
        if pos[1] < rect[1] { rect[1] } else { rect[3] }
    } else {
        pos[1]
    };
    let x = if pos[0] < rect[0] || rect[2] < pos[0] {
        if pos[0] < rect[0] { rect[0] } else { rect[2] }
    } else {
        pos[0]
    };
    let mut tx = (w18a10_div32(x as i32) & 0xffff) as u16;
    let mut ty = (w18a10_div32(y as i32) & 0xffff) as u16;
    let mut n: i32 = 1;
    while n < 0x10 {
        for side in 0..4 {
            for _ in 0..n {
                if tx < map_tiles.0
                    && ty < map_tiles.1
                    && region_at((tx as i16 as i32) << 5, (ty as i16 as i32) << 5) == region
                {
                    pos[0] = (tx << 5).wrapping_add(0x10) as i16;
                    pos[1] = (ty << 5).wrapping_add(0x10) as i16;
                    *flag = 1;
                    return true;
                }
                tx = tx.wrapping_add(dx[side] as u16);
                ty = ty.wrapping_add(dy[side] as u16);
            }
            tx = tx.wrapping_sub(dx[side] as u16);
            ty = ty.wrapping_sub(dy[side] as u16);
        }
        tx = tx.wrapping_sub(1);
        ty = ty.wrapping_sub(1);
        n += 2;
    }
    on_fail(pos);
    *flag = 1;
    true
}

#[cfg(test)]
mod tests_wave18_agent10 {
    use super::*;

    #[test]
    fn unaligned_read() {
        let mem = [0x0403_0201u32, 0x0807_0605];
        let rd = |a: u32| mem.get((a / 4) as usize).copied();
        assert_eq!(w18a10_read_u32_unaligned(4, rd), Some(0x0807_0605));
        assert_eq!(w18a10_read_u32_unaligned(1, rd), Some(0x0504_0302));
        assert_eq!(w18a10_read_u32_unaligned(3, rd), Some(0x0706_0504));
        assert_eq!(w18a10_read_u32_unaligned(5, rd), None);
    }

    #[test]
    fn row_remove() {
        let mut t = vec![0u32; 96];
        t[12..16].copy_from_slice(&[5, 6, 7, 0]);
        w18a10_remove_row_entry(&mut t, 1, 5);
        assert_eq!(&t[12..16], &[6, 7, 0, 0]);
        w18a10_remove_row_entry(&mut t, 1, 7);
        assert_eq!(&t[12..16], &[6, 0, 0, 0]);
        w18a10_remove_row_entry(&mut t, 1, 9);
        assert_eq!(t[12], 6);
        w18a10_remove_row_entry(&mut t, 8, 6);
        for v in &mut t[24..36] {
            *v = 3;
        }
        w18a10_remove_row_entry(&mut t, 2, 3);
        assert_eq!(t[34], 3);
        assert_eq!(t[35], 0);
    }

    #[test]
    fn lookup() {
        let mut tb = [0u16; 4];
        tb[1] = 0x26;
        tb[2] = 0x3c;
        tb[3] = 7;
        assert_eq!(w18a10_versioned_table_lookup(&tb, 1, 0x20, 100, 200), 100);
        assert_eq!(w18a10_versioned_table_lookup(&tb, 1, 0x1f, 100, 200), 0x26);
        assert_eq!(w18a10_versioned_table_lookup(&tb, 2, 0x1c, 100, 200), 200);
        assert_eq!(w18a10_versioned_table_lookup(&tb, 2, 0x1b, 100, 200), 0x3c);
        assert_eq!(w18a10_versioned_table_lookup(&tb, 3, 99, 100, 200), 7);
    }

    #[test]
    fn neighbours() {
        let offs = [
            (0, 0),
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
        ];
        let mut seen = vec![];
        let r = w18a10_visit_neighbours(0, 0, 4, 4, &offs, |i, c, _, _| {
            seen.push((i, c));
            1
        });
        assert_eq!(r, 9);
        assert_eq!(seen, vec![(0, 0), (1, 1), (3, 4), (5, 5)]);
        let r = w18a10_visit_neighbours(2, 2, 4, 4, &offs, |i, _, _, _| (i != 2) as i32);
        assert_eq!(r, 2);
    }

    #[test]
    fn tile_rect() {
        let (w, r) = w18a10_object_tile_rect(0x84, 640, 320, false, (0, 0), 64, 64);
        assert!(w);
        assert_eq!(r, [10, 3, 30, 16]);
        let (w, r) = w18a10_object_tile_rect(0x10, 320, 320, true, (96, 64), 64, 64);
        assert!(!w);
        assert_eq!(r, [9, 9, 11, 10]);
        let (w, r) = w18a10_object_tile_rect(0x83, 0, 0, true, (0, 0), 8, 8);
        assert!(w);
        assert_eq!(r, [0, 0, 10, 6]);
        let (_, r) = w18a10_object_tile_rect(0x10, 0, 100, false, (64, 64), 64, 64);
        assert_eq!(r[0], 0);
    }

    #[test]
    fn take_and_args() {
        let (mut s, mut f, mut d) = (5u16, 0u32, 0u16);
        assert!(w18a10_take_nonzero(&mut s, &mut f, &mut d));
        assert_eq!((s, f, d), (0, 1, 5));
        assert!(!w18a10_take_nonzero(&mut s, &mut f, &mut d));
        assert_eq!(w18a10_rect_to_tile_args([64, 32, 100, 96]), [2, 1, 2, 2]);
    }

    #[test]
    fn region_move() {
        let dx = [1, 0, -1, 0];
        let dy = [0, 1, 0, -1];
        let mut pos = [100, 100];
        let mut flag = 0;
        let ok = w18a10_move_to_region(
            &mut pos,
            &mut flag,
            [0, 0, 1000, 1000],
            7,
            (64, 64),
            dx,
            dy,
            |x, y| if (x, y) == (3 << 5, 3 << 5) { 7 } else { 0 },
            |_| panic!(),
        );
        assert!(ok);
        assert_eq!((pos, flag), ([3 * 32 + 16, 3 * 32 + 16], 1));
        let mut pos = [5000, -5];
        let mut flag = 0;
        let mut failed = false;
        w18a10_move_to_region(
            &mut pos,
            &mut flag,
            [0, 0, 64, 64],
            7,
            (64, 64),
            dx,
            dy,
            |_, _| 0,
            |_| failed = true,
        );
        assert!(failed && flag == 1);
    }
}
