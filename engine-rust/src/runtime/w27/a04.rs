#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent04.rs
// ===========================================================================
// Wave 27 agent 04: small config/bitmap helpers.

/// Default-application kinds used by `0x0095a710` (first table pass).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W27a04Default {
    /// 0 = store zero, 1 = store `value`, 2 = set/clear `mask` bits.
    pub kind: u32,
    pub mask: u32,
    pub value: u32,
    /// Index of the destination dword in the caller's array.
    pub dest: usize,
}

/// `0x0095a710` (first pass): applies one default-table entry to `dests`.
pub fn w27a04_apply_default(e: &W27a04Default, dests: &mut [u32]) {
    let d = &mut dests[e.dest];
    match e.kind {
        0 => *d = 0,
        1 => *d = e.value,
        2 => {
            if e.value == 0 {
                *d &= !e.mask;
            } else {
                *d |= e.mask;
            }
        }
        _ => {}
    }
}

/// Registry-backed setting kinds used by `0x0095a710` (second pass).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W27a04Setting {
    /// 0 = string, 1 = ranged dword, 2 = flag bit.
    pub kind: u32,
    pub key: u32,
    pub min: u32,
    pub max_or_mask: u32,
    pub default: u32,
    pub dest: usize,
}

/// `0x0095a710` (second pass): applies a registry-read setting.
/// `read_str(key)` / `read_u32(key)` are the "Starcraft" registry readers;
/// strings are only copied when shorter than `cap` (stored in `default`).
pub fn w27a04_apply_setting(
    s: &W27a04Setting,
    dests: &mut [u32],
    str_dest: &mut Vec<u8>,
    read_str: impl Fn(u32) -> Option<Vec<u8>>,
    read_u32: impl Fn(u32) -> Option<u32>,
) {
    match s.kind {
        0 => {
            if let Some(v) = read_str(s.key) {
                let len = v.iter().position(|&b| b == 0).unwrap_or(v.len());
                if (len as u32) < s.default {
                    *str_dest = v[..len].to_vec();
                }
            }
        }
        1 => {
            if let Some(mut v) = read_u32(s.key) {
                if v < s.min || s.max_or_mask < v {
                    v = s.default;
                }
                dests[s.dest] = v;
            }
        }
        2 => {
            if let Some(v) = read_u32(s.key) {
                if v == 0 {
                    dests[s.dest] &= !s.max_or_mask;
                } else {
                    dests[s.dest] |= s.max_or_mask;
                }
            }
        }
        _ => {}
    }
}

/// `0x0095bec0`: returns byte at `+4`, optionally writing dword at `+8`.
pub fn w27a04_get_flag_and_value(flag: u8, value: u32, out: Option<&mut u32>) -> u8 {
    if let Some(o) = out {
        *o = value;
    }
    flag
}

/// `0x0095bd70`: forwards to the loader with an optional dword that is
/// present only when the flag byte at `+0x9c` is non-zero.
pub fn w27a04_optional_arg(flag: u8, value: u32) -> Option<u32> {
    if flag == 0 { None } else { Some(value) }
}

/// Decoded record of `0x00961780`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W27a04Entry {
    pub out4: u16,
    pub out5: u16,
    pub out6: u16,
    pub out7: u16,
    pub a: u16,
    pub b: u16,
    /// Offset into the table (`(dword & 0x7fffffff) + base`).
    pub offset: u32,
}

/// `0x00961780`: reads entry `idx` (8-byte stride from byte 6) of a table
/// whose first word is the count; an index not below the count selects 0.
pub fn w27a04_read_entry(t: &[u8], idx: u8) -> Option<W27a04Entry> {
    let w = |o: usize| t.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let count = w(0)?;
    let i = if (idx as u16) < count {
        idx as usize
    } else {
        0
    };
    let base = 8 * i;
    let d = t.get(base + 10..base + 14)?;
    let dw = u32::from_le_bytes([d[0], d[1], d[2], d[3]]);
    Some(W27a04Entry {
        out4: *t.get(base + 6)? as u16,
        out5: *t.get(base + 7)? as u16,
        out6: w(2)?,
        out7: w(4)?,
        a: *t.get(base + 8)? as u16,
        b: *t.get(base + 9)? as u16,
        offset: (dw & 0x7fff_ffff),
    })
}

/// Simple 8-bit surface: `stride` is the width word of the descriptor.
pub struct W27a04Surface<'a> {
    pub stride: usize,
    pub height: usize,
    pub pixels: &'a mut [u8],
}

/// `0x00961700`: fills the whole surface (`width * height`) with `color`.
pub fn w27a04_fill(s: &mut W27a04Surface, color: u8) {
    let n = (s.stride * s.height).min(s.pixels.len());
    s.pixels[..n].fill(color);
}

/// `0x009617f0`: horizontal line of `len` pixels at (x, y).
pub fn w27a04_hline(s: &mut W27a04Surface, x: i16, y: i16, len: u16, color: u8) {
    let start = s.stride as isize * y as isize + x as isize;
    for k in 0..len as isize {
        if let Some(p) = usize::try_from(start + k)
            .ok()
            .and_then(|i| s.pixels.get_mut(i))
        {
            *p = color;
        }
    }
}

/// `0x00961830`: vertical line of `len` pixels at (x, y).
pub fn w27a04_vline(s: &mut W27a04Surface, x: i16, y: u16, len: u16, color: u8) {
    let mut pos = s.stride as isize * y as isize + x as isize;
    for _ in 0..len {
        if let Some(p) = usize::try_from(pos).ok().and_then(|i| s.pixels.get_mut(i)) {
            *p = color;
        }
        pos += s.stride as isize;
    }
}

/// `0x00961730`: outlines the rectangle (x, y, w, h) with `color`.
pub fn w27a04_frame(s: &mut W27a04Surface, x: i16, y: i16, w: i16, h: i16, color: u8) {
    w27a04_hline(s, x, y, w as u16, color);
    w27a04_hline(s, x, h.wrapping_sub(1).wrapping_add(y), w as u16, color);
    w27a04_vline(s, x, y as u16, h as u16, color);
    w27a04_vline(
        s,
        w.wrapping_add(x).wrapping_sub(1),
        y as u16,
        h as u16,
        color,
    );
}

#[cfg(test)]
mod tests_wave27_agent04 {
    use super::*;

    #[test]
    fn defaults_and_settings() {
        let mut d = [7u32, 0xff];
        let e = |kind, dest| W27a04Default {
            kind,
            mask: 0xf0,
            value: 5,
            dest,
        };
        w27a04_apply_default(&e(0, 0), &mut d);
        assert_eq!(d[0], 0);
        w27a04_apply_default(&e(1, 0), &mut d);
        assert_eq!(d[0], 5);
        let mut z = W27a04Default {
            value: 0,
            ..e(2, 1)
        };
        w27a04_apply_default(&z, &mut d);
        assert_eq!(d[1], 0x0f);
        z.value = 1;
        w27a04_apply_default(&z, &mut d);
        assert_eq!(d[1], 0xff);

        let mut s = Vec::new();
        let st = W27a04Setting {
            kind: 1,
            key: 1,
            min: 2,
            max_or_mask: 4,
            default: 3,
            dest: 0,
        };
        w27a04_apply_setting(&st, &mut d, &mut s, |_| None, |_| Some(9));
        assert_eq!(d[0], 3);
        w27a04_apply_setting(&st, &mut d, &mut s, |_| None, |_| Some(4));
        assert_eq!(d[0], 4);
        let st0 = W27a04Setting {
            kind: 0,
            default: 4,
            ..st
        };
        w27a04_apply_setting(&st0, &mut d, &mut s, |_| Some(b"abc\0".to_vec()), |_| None);
        assert_eq!(s, b"abc");
        w27a04_apply_setting(&st0, &mut d, &mut s, |_| Some(b"abcd\0".to_vec()), |_| None);
        assert_eq!(s, b"abc");
    }

    #[test]
    fn small_accessors() {
        let mut o = 0;
        assert_eq!(w27a04_get_flag_and_value(3, 9, Some(&mut o)), 3);
        assert_eq!(o, 9);
        assert_eq!(w27a04_optional_arg(0, 5), None);
        assert_eq!(w27a04_optional_arg(1, 5), Some(5));
    }

    #[test]
    fn entry_read() {
        let mut t = vec![0u8; 32];
        t[0] = 2;
        t[2] = 11;
        t[4] = 12;
        t[6 + 8] = 1;
        t[7 + 8] = 2;
        t[8 + 8] = 3;
        t[9 + 8] = 4;
        t[18..22].copy_from_slice(&0x8000_0010u32.to_le_bytes());
        let e = w27a04_read_entry(&t, 1).unwrap();
        assert_eq!(
            (e.out4, e.out5, e.a, e.b, e.out6, e.out7),
            (1, 2, 3, 4, 11, 12)
        );
        assert_eq!(e.offset, 0x10);
        let e0 = w27a04_read_entry(&t, 9).unwrap();
        assert_eq!(e0.out4, 0);
        assert!(w27a04_read_entry(&t[..4], 0).is_none());
    }

    #[test]
    fn drawing() {
        let mut px = vec![0u8; 36];
        let mut s = W27a04Surface {
            stride: 6,
            height: 6,
            pixels: &mut px,
        };
        w27a04_frame(&mut s, 1, 1, 4, 3, 9);
        let row = |y: usize| s.pixels[y * 6..y * 6 + 6].to_vec();
        assert_eq!(row(1), [0, 9, 9, 9, 9, 0]);
        assert_eq!(row(2), [0, 9, 0, 0, 9, 0]);
        assert_eq!(row(3), [0, 9, 9, 9, 9, 0]);
        w27a04_fill(&mut s, 2);
        assert!(s.pixels.iter().all(|&p| p == 2));
    }
}
