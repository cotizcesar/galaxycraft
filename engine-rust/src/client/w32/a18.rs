#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent18.rs
// ===========================================================================
// Wave 32 translation batch 18.

/// Glyph rectangle edges produced by `0x00960830`: `[left, top, right, bottom]`.
pub type W32a18Rect = [i32; 4];

/// `0x00960830`: scales a glyph's four raw metrics into pixel edges.
/// `raw` is `[a, b, c, d]` where a/c are x-like and b/d y-like (the original
/// reads them either from a big-endian i16 record at +2..+9 or from a
/// callee, both supplied by the caller as `a=x0, b=y0, c=x1, d=y1` with the
/// record order x0,y0,x1,y1 = bytes 2,4,6,8).  `left = floor(x0*sx+ox)`,
/// `top = floor(oy - y1*sy)`, `right = ceil(x1*sx+ox)`,
/// `bottom = ceil(oy - y0*sy)`.  `None` (lookup failure) zeroes all outputs.
pub fn w32a18_glyph_rect(raw: Option<[i32; 4]>, sx: f32, sy: f32, ox: f32, oy: f32) -> W32a18Rect {
    let Some([x0, y0, x1, y1]) = raw else {
        return [0; 4];
    };
    let left = ((x0 as f32) * sx + ox) as f64;
    let top = (oy - (y1 as f32) * sy) as f64;
    let right = ((x1 as f32) * sx + ox) as f64;
    let bottom = (oy - (y0 as f32) * sy) as f64;
    [
        left.floor() as i32,
        top.floor() as i32,
        right.ceil() as i32,
        bottom.ceil() as i32,
    ]
}

/// Decodes the big-endian i16 at `rec[off..off+2]` (used by `0x00960830`).
pub fn w32a18_be_i16(rec: &[u8], off: usize) -> i32 {
    i16::from_be_bytes([rec[off], rec[off + 1]]) as i32
}

/// `0x00962570`: searches named tables `(name, entries)` in order; the first
/// whose name equals `name` and whose length exceeds `index` yields
/// `entries[index]`; otherwise 0.
pub fn w32a18_table_lookup(tables: &[(&str, &[u32])], name: &str, index: u32) -> u32 {
    for (n, entries) in tables {
        if *n == name && (index as usize) < entries.len() {
            return entries[index as usize];
        }
    }
    0
}

/// `0x00964f30`: sets (`on != 0`) or clears bit `0x1000` of the flags word.
pub fn w32a18_set_flag_1000(flags: u32, on: bool) -> u32 {
    if on {
        flags | 0x1000
    } else {
        flags & 0xffff_efff
    }
}

#[cfg(test)]
mod tests_wave32_agent18 {
    use super::*;

    #[test]
    fn rect() {
        assert_eq!(w32a18_glyph_rect(None, 1.0, 1.0, 0.0, 0.0), [0; 4]);
        assert_eq!(
            w32a18_glyph_rect(Some([1, 2, 3, 4]), 1.5, 2.0, 0.5, 10.0),
            [2, 2, 5, 6]
        );
        assert_eq!(w32a18_be_i16(&[0xff, 0xfe], 0), -2);
    }

    #[test]
    fn lookup() {
        let a = [5u32, 6];
        let b = [7u32];
        let t: [(&str, &[u32]); 2] = [("a", &a), ("a", &b)];
        assert_eq!(w32a18_table_lookup(&t, "a", 1), 6);
        assert_eq!(w32a18_table_lookup(&t, "a", 2), 0);
        assert_eq!(w32a18_table_lookup(&t, "z", 0), 0);
    }

    #[test]
    fn flag() {
        assert_eq!(w32a18_set_flag_1000(0, true), 0x1000);
        assert_eq!(w32a18_set_flag_1000(0x1001, false), 1);
    }
}
