#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent25.rs
// ===========================================================================
// Wave 31 agent 25: small field-update helpers (runtime.rs target).

/// Recovered fields of the scalar/direction record used by the 0x38/0x40/0x44 helpers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct W31a25Motion {
    /// +0x34: upper clamp.
    pub limit: i32,
    /// +0x38: current value.
    pub value: i32,
    /// +0x3c: target value.
    pub target: i32,
    /// +0x40: x component.
    pub x: i32,
    /// +0x44: y component.
    pub y: i32,
    /// +0x4a: direction index.
    pub dir: u8,
    /// +0x20: saved byte.
    pub flag20: u8,
}

fn w31a25_apply(m: &mut W31a25Motion, table: &[(i32, i32)], v: i32) {
    let (tx, ty) = table[m.dir as usize];
    m.value = v;
    m.x = tx.wrapping_mul(v) >> 8;
    m.y = ty.wrapping_mul(v) >> 8;
}

/// `0x0056c7e0`: set the value, recomputing the x/y components when it changed.
pub fn w31a25_set_value(m: &mut W31a25Motion, table: &[(i32, i32)], v: i32) {
    if m.value != v {
        w31a25_apply(m, table, v);
    }
}

/// `0x00569580`: add `delta` to the value, clamping to `[0, limit]`.
pub fn w31a25_add_clamped(m: &mut W31a25Motion, table: &[(i32, i32)], delta: i32) {
    let cur = m.value;
    let n = delta.wrapping_add(cur);
    if m.limit < n {
        if cur != m.limit {
            w31a25_apply(m, table, m.limit);
        }
    } else if n < 0 {
        if cur != 0 {
            m.value = 0;
            m.x = 0;
            m.y = 0;
        }
    } else if cur != n {
        w31a25_apply(m, table, n);
    }
}

/// `0x005694a0`: set direction and target/value, run the two caller-supplied
/// steps, and swap the saved byte at +0x20 through `saved` (global byte pair).
pub fn w31a25_set_dir_and_value(
    m: &mut W31a25Motion,
    table: &[(i32, i32)],
    dir: u8,
    value: i32,
    saved: &mut (u8, u8),
    mut step: impl FnMut(&mut W31a25Motion),
) {
    saved.0 = m.flag20;
    let old = m.value;
    if m.dir != dir {
        m.dir = dir;
        let (tx, ty) = table[dir as usize];
        m.x = tx.wrapping_mul(old) >> 8;
        m.y = ty.wrapping_mul(old) >> 8;
    }
    m.target = value;
    if old != value {
        w31a25_apply(m, table, value);
    }
    step(m);
    step(m);
    saved.1 = m.flag20;
    m.flag20 = saved.0;
}

/// `0x00570dd0`: sets bit 0 of the u16 flags, byte at +10 to 2 and u16 at +0x30 to 0x300.
pub fn w31a25_init_flags(flags: &mut u16, b10: &mut u8, w30: &mut u16) {
    *flags |= 1;
    *b10 = 2;
    *w30 = 0x300;
}

/// `0x00572180`: indexed table lookup.
pub fn w31a25_table_get(table: &[u32], i: usize) -> u32 {
    table[i]
}

/// `0x00572300`: global plus 6.
pub fn w31a25_plus6(g: i32) -> i32 {
    g.wrapping_add(6)
}

/// `0x005775b0`: difference of two globals.
pub fn w31a25_diff(a: i32, b: i32) -> i32 {
    a.wrapping_sub(b)
}

/// `0x00572860`: walks an RLE row of the sprite at `data` (row offsets as u16
/// at the start); returns 1 if column `x` falls on a drawn run, else 0.
pub fn w31a25_rle_hit(w: u8, h: u8, data: &[u8], x: u32, y: u32) -> u8 {
    if x >= w as u32 || y >= h as u32 {
        return 0;
    }
    let mut pos = u16::from_le_bytes([data[y as usize * 2], data[y as usize * 2 + 1]]) as usize;
    let mut x = x as i32;
    loop {
        let b = data[pos];
        pos += 1;
        if b & 0x80 != 0 {
            x -= (b & 0x7f) as i32;
        } else if b & 0x40 == 0 {
            x -= b as i32;
            pos += b as usize;
        } else {
            x -= (b & 0x3f) as i32;
            pos += 1;
        }
        if x <= 0 {
            return !(b >> 7) & 1;
        }
    }
}

/// `0x00574770`: bounding rect [x0,y0,x1,y1]; with a frame table (`count` + 6 pad bytes,
/// then 8-byte entries whose bytes 2/3 are w/h) it extends by w-1,h-1.
pub fn w31a25_frame_rect(px: i16, py: i16, frame: u16, table: Option<(u16, &[u8])>) -> [i32; 4] {
    let (x, y) = (px as i32, py as i32);
    if let Some((count, t)) = table
        && frame < count
    {
        let o = 8 + frame as usize * 8;
        return [x, y, x - 1 + t[o] as i32, y - 1 + t[o + 1] as i32];
    }
    [x, y, x, y]
}

#[cfg(test)]
mod tests_wave31_agent25 {
    use super::*;

    fn tbl() -> Vec<(i32, i32)> {
        (0..4).map(|i| (256 * (i + 1), 128 * (i + 1))).collect()
    }

    #[test]
    fn motion() {
        let t = tbl();
        let mut m = W31a25Motion {
            limit: 10,
            dir: 1,
            ..Default::default()
        };
        w31a25_set_value(&mut m, &t, 4);
        assert_eq!((m.value, m.x, m.y), (4, 8, 4));
        w31a25_add_clamped(&mut m, &t, 100);
        assert_eq!(m.value, 10);
        w31a25_add_clamped(&mut m, &t, -100);
        assert_eq!((m.value, m.x, m.y), (0, 0, 0));
        w31a25_add_clamped(&mut m, &t, 3);
        assert_eq!(m.value, 3);
    }

    #[test]
    fn dir_and_value() {
        let t = tbl();
        let mut m = W31a25Motion {
            value: 2,
            flag20: 7,
            ..Default::default()
        };
        let mut s = (0, 0);
        let mut n = 0;
        w31a25_set_dir_and_value(&mut m, &t, 2, 5, &mut s, |mm| {
            n += 1;
            mm.flag20 = 9;
        });
        assert_eq!(
            (n, m.dir, m.value, m.target, s, m.flag20),
            (2, 2, 5, 5, (7, 9), 7)
        );
        assert_eq!(m.x, 3 * 256 * 5 >> 8);
    }

    #[test]
    fn misc() {
        let (mut f, mut b, mut w) = (0u16, 0u8, 0u16);
        w31a25_init_flags(&mut f, &mut b, &mut w);
        assert_eq!((f, b, w), (1, 2, 0x300));
        assert_eq!(w31a25_table_get(&[1, 2, 3], 2), 3);
        assert_eq!(w31a25_plus6(1), 7);
        assert_eq!(w31a25_diff(5, 2), 3);
    }

    #[test]
    fn rle() {
        // row 0 at offset 2: skip run (0x80|3), then draw run of 2 bytes (0x02, a, b)
        let d = [2u8, 0, 0x83, 0x02, 1, 1];
        assert_eq!(w31a25_rle_hit(8, 1, &d, 8, 0), 0);
        assert_eq!(w31a25_rle_hit(8, 1, &d, 2, 0), 0);
        assert_eq!(w31a25_rle_hit(8, 1, &d, 4, 0), 1);
    }

    #[test]
    fn rect() {
        assert_eq!(w31a25_frame_rect(1, 2, 0, None), [1, 2, 1, 2]);
        let t = [0u8; 16];
        assert_eq!(w31a25_frame_rect(1, 2, 0, Some((1, &t))), [1, 2, 0, 1]);
        assert_eq!(w31a25_frame_rect(1, 2, 1, Some((1, &t))), [1, 2, 1, 2]);
    }
}
