#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent16.rs
// ===========================================================================
// Wave 23 agent 16: small runtime/render-glue helpers.

/// `0x007142a0`: clears (`0`) the first byte of every active slot whose
/// matching flag byte has bit 0 set.  11 slots, stride `0xda`.
pub fn w23a16_clear_flagged_slots(slots: &mut [u8], flags: &[u8]) {
    for i in 0..11usize {
        let off = i * 0xda;
        if off >= slots.len() || i >= flags.len() {
            break;
        }
        if slots[off] != 0 && (flags[i] & 1) != 0 {
            slots[off] = 0;
        }
    }
}

/// Result of `0x00714dc0`: arguments handed to the selected blitter.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W23a16BlitPlan {
    /// Index into the handler table (`+1` when `alt`, `+2` when clipped).
    pub handler: u32,
    pub tile_x: i32,
    pub tile_y: i32,
    pub flags: u32,
    pub mask: u32,
    /// Offset in dwords into the destination map layer.
    pub dest_index: i64,
}

/// `0x00714dc0`: converts pixel coordinates to 32px tiles, picks the flag
/// word from the tile class (`tile_class` 2 -> `0x1000000`, 1 -> `0x5000000`,
/// else `0x7000000`) and the handler (clipped against map size).  `size` is
/// the (w, h) row for `shape` (stride `0x1c`).  Returns `None` if
/// `color >= 0x100` or `shape >= 12`.
#[allow(clippy::too_many_arguments)]
pub fn w23a16_blit_plan(
    x: i32,
    y: i32,
    alt: bool,
    color: u32,
    shape: u32,
    tile_class: i32,
    size: (i32, i32),
    map_w: u16,
    map_h: u16,
) -> Option<W23a16BlitPlan> {
    if color >= 0x100 || shape >= 0xc {
        return None;
    }
    let mut handler = 0u32;
    let flags = if !alt {
        match tile_class {
            2 => color | 0x100_0000,
            1 => color | 0x500_0000,
            _ => color | 0x700_0000,
        }
    } else {
        handler = 1;
        color
    };
    let tx = (x + ((x >> 31) & 0x1f)) >> 5;
    let ty = (y + ((y >> 31) & 0x1f)) >> 5;
    let hw = size.0 / 2;
    let hh = size.1 / 2;
    if tx - hw < 0 || map_w as i32 <= tx + hw || ty - hh < 0 || map_h as i32 <= hh + ty {
        handler += 2;
    }
    Some(W23a16BlitPlan {
        handler,
        tile_x: tx,
        tile_y: ty,
        flags,
        mask: !((color << 8) | color),
        dest_index: map_w as i64 * ty as i64 + tx as i64,
    })
}

/// `0x00715f40`: on a found record, copies `table[idx]` into the u16 at
/// `+0x3e` and ORs `bits` into the u32 at `+0x30`.  Returns whether found.
pub fn w23a16_apply_record(rec: Option<&mut [u8]>, table: &[u16], idx: u16, bits: u32) -> bool {
    let Some(r) = rec else { return false };
    let v = table.get(idx as usize).copied().unwrap_or(0);
    r[0x3e..0x40].copy_from_slice(&v.to_le_bytes());
    let f = u32::from_le_bytes([r[0x30], r[0x31], r[0x32], r[0x33]]) | bits;
    r[0x30..0x34].copy_from_slice(&f.to_le_bytes());
    true
}

/// `0x00716070`: point-in-rect test.  `pt` is the converted point, `rect`
/// is (left, top, right, bottom); false whenever `disabled`.
pub fn w23a16_point_in_view(pt: (i32, i32), rect: (i32, i32, i32, i32), disabled: bool) -> bool {
    if disabled {
        return false;
    }
    let x = pt.0.wrapping_sub(rect.0);
    let y = pt.1.wrapping_sub(rect.1);
    x > -1 && y > -1 && x < rect.2.wrapping_sub(rect.0) && y < rect.3.wrapping_sub(rect.1)
}

/// `0x00716ee0`: identity-palette table init (when not in the early-out
/// state).  Fills four 256-byte tables: `fill`, copy of `src`, and two
/// identity ramps.
pub fn w23a16_init_palette_tables(
    fill: u8,
    src: &[u8; 256],
    t0: &mut [u8; 256],
    t1: &mut [u8; 256],
    t2: &mut [u8; 256],
    t3: &mut [u8; 256],
) {
    for i in 0..256usize {
        t0[i] = fill;
        t1[i] = src[i];
        t2[i] = i as u8;
        t3[i] = i as u8;
    }
}

#[cfg(test)]
mod tests_wave23_agent16 {
    use super::*;

    #[test]
    fn clear_slots() {
        let mut s = vec![1u8; 11 * 0xda];
        let mut f = [0u8; 11];
        f[0] = 1;
        f[10] = 3;
        f[3] = 2;
        w23a16_clear_flagged_slots(&mut s, &f);
        assert_eq!((s[0], s[0xda * 3], s[0xda * 10]), (0, 1, 0));
    }

    #[test]
    fn plan() {
        assert!(w23a16_blit_plan(0, 0, false, 0x100, 0, 0, (2, 2), 64, 64).is_none());
        assert!(w23a16_blit_plan(0, 0, false, 0, 12, 0, (2, 2), 64, 64).is_none());
        let p = w23a16_blit_plan(320, 64, false, 5, 0, 2, (2, 2), 64, 64).unwrap();
        assert_eq!(
            (p.tile_x, p.tile_y, p.handler, p.flags),
            (10, 2, 0, 0x100_0005)
        );
        assert_eq!(p.mask, !0x505);
        assert_eq!(p.dest_index, 64 * 2 + 10);
        let p = w23a16_blit_plan(0, 0, true, 5, 0, 0, (2, 2), 64, 64).unwrap();
        assert_eq!((p.handler, p.flags), (3, 5));
        let p = w23a16_blit_plan(320, 64, false, 5, 0, 1, (2, 2), 64, 64).unwrap();
        assert_eq!(p.flags, 0x500_0005);
        let p = w23a16_blit_plan(320, 64, false, 5, 0, 0, (2, 2), 64, 64).unwrap();
        assert_eq!(p.flags, 0x700_0005);
    }

    #[test]
    fn record() {
        let mut r = vec![0u8; 0x40];
        r[0x30] = 1;
        assert!(w23a16_apply_record(Some(&mut r), &[7, 9], 1, 4));
        assert_eq!(r[0x30], 5);
        assert_eq!(r[0x3e], 9);
        assert!(!w23a16_apply_record(None, &[], 0, 0));
    }

    #[test]
    fn view() {
        let r = (10, 10, 20, 20);
        assert!(w23a16_point_in_view((10, 10), r, false));
        assert!(!w23a16_point_in_view((20, 15), r, false));
        assert!(!w23a16_point_in_view((9, 15), r, false));
        assert!(!w23a16_point_in_view((15, 15), r, true));
    }

    #[test]
    fn palette() {
        let src = [3u8; 256];
        let (mut a, mut b, mut c, mut d) = ([0u8; 256], [0u8; 256], [0u8; 256], [0u8; 256]);
        w23a16_init_palette_tables(9, &src, &mut a, &mut b, &mut c, &mut d);
        assert_eq!((a[255], b[0], c[200], d[7]), (9, 3, 200, 7));
    }
}
