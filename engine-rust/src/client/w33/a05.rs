#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent05.rs
// ===========================================================================
// Wave 33 translation batch 05: TrueType cmap lookup and two small
// error-code guards.

fn w33a05_u8(d: &[u8], i: usize) -> u32 {
    d.get(i).copied().unwrap_or(0) as u32
}

fn w33a05_u16(d: &[u8], i: usize) -> u32 {
    (w33a05_u8(d, i) << 8) | w33a05_u8(d, i + 1)
}

fn w33a05_u32(d: &[u8], i: usize) -> u32 {
    (w33a05_u16(d, i) << 16) | w33a05_u16(d, i + 2)
}

/// `0x00960490`: TrueType cmap subtable lookup (glyph index for a code
/// point).  `data` is the font file, `index_map` the offset of the chosen
/// cmap subtable.  Formats 0, 4, 6, 12 and 13 are handled; format 2 and
/// unknown formats return 0.  Out-of-range reads yield 0.
pub fn w33a05_find_glyph_index(data: &[u8], index_map: usize, cp: u32) -> u32 {
    let m = index_map;
    let format = w33a05_u16(data, m);
    match format {
        0 => {
            let len = w33a05_u16(data, m + 2) as i32 - 6;
            if (cp as i32) < len {
                return w33a05_u8(data, m + 6 + cp as usize);
            }
            0
        }
        6 => {
            let first = w33a05_u16(data, m + 6);
            let count = w33a05_u16(data, m + 8);
            if first <= cp && cp < count + first {
                return w33a05_u16(data, m + 10 + (cp - first) as usize * 2);
            }
            0
        }
        4 => {
            let segcount = (w33a05_u16(data, m + 6) >> 1) as usize;
            let mut search_range = (w33a05_u16(data, m + 8) >> 1) as u16;
            let mut selector = w33a05_u16(data, m + 10) as u16;
            let end_count = m + 14;
            if cp >= 0x10000 {
                return 0;
            }
            let range_shift = (w33a05_u16(data, m + 12) >> 1) as usize;
            let mut search = end_count;
            if w33a05_u16(data, end_count + range_shift * 2) <= cp {
                search = end_count + range_shift * 2;
            }
            let mut search = search as isize - 2;
            while selector != 0 {
                search_range >>= 1;
                let probe = (search + search_range as isize * 2) as usize;
                if w33a05_u16(data, probe) < cp {
                    search = probe as isize;
                }
                selector -= 1;
            }
            let item = (((search + 2) as usize - end_count) >> 1) & 0xffff;
            let start = w33a05_u16(data, m + 16 + (segcount + item) * 2);
            if start > cp {
                return 0;
            }
            let offset = w33a05_u16(data, m + 16 + (segcount * 3 + item) * 2);
            if offset == 0 {
                let delta = w33a05_u16(data, m + 16 + (segcount * 2 + item) * 2 - 0);
                return (delta + cp) & 0xffff;
            }
            let idx = (segcount * 3 + item) as u32 + cp - start;
            let addr = m + 16 + offset as usize + idx as usize * 2;
            w33a05_u16(data, addr)
        }
        12 | 13 => {
            let mut high = w33a05_u32(data, m + 12) as i32;
            let mut low = 0i32;
            while low < high {
                let mid = ((high - low) >> 1) + low;
                let g = m + mid as usize * 12;
                let start = w33a05_u32(data, g + 16);
                if start <= cp {
                    if cp <= w33a05_u32(data, g + 20) {
                        let glyph = w33a05_u32(data, g + 24);
                        return if format == 12 {
                            glyph.wrapping_sub(start).wrapping_add(cp)
                        } else {
                            glyph
                        };
                    }
                    low = mid + 1;
                } else {
                    high = mid;
                }
            }
            0
        }
        _ => 0,
    }
}

/// `0x00984ab0`: returns error 0x25 when `handle` is zero, otherwise the
/// result of the delegate (`FUN_00984a50`).
pub fn w33a05_handle_guard(handle: u32, delegate: impl FnOnce() -> u32) -> u32 {
    if handle == 0 { 0x25 } else { delegate() }
}

/// `0x0098b260`: returns 0x25 when `handle` is zero, 0x21 when the object's
/// field at +0x7a4 is zero, otherwise the delegate (`FUN_0099dd10`) called
/// with `(handle, 0, 0, arg)`.
pub fn w33a05_guarded_dispatch(
    handle: u32,
    field_7a4: u32,
    arg: u32,
    delegate: impl FnOnce(u32, u32, u32, u32) -> u32,
) -> u32 {
    if handle == 0 {
        0x25
    } else if field_7a4 != 0 {
        delegate(handle, 0, 0, arg)
    } else {
        0x21
    }
}

#[cfg(test)]
mod tests_wave33_agent05 {
    use super::*;

    #[test]
    fn format0_and_6() {
        let mut d = vec![0u8; 16];
        d[3] = 10;
        d[6] = 7;
        d[7] = 9;
        assert_eq!(w33a05_find_glyph_index(&d, 0, 1), 9);
        assert_eq!(w33a05_find_glyph_index(&d, 0, 4), 0);
        let d6 = [0, 6, 0, 0, 0, 0, 0, 65, 0, 2, 0, 5, 0, 6];
        assert_eq!(w33a05_find_glyph_index(&d6, 0, 66), 6);
        assert_eq!(w33a05_find_glyph_index(&d6, 0, 67), 0);
        assert_eq!(w33a05_find_glyph_index(&d6, 0, 64), 0);
    }

    #[test]
    fn format12_and_13() {
        let mk = |f: u8| {
            let mut d = vec![0, f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
            d.extend_from_slice(&[0, 0, 0, 0x41, 0, 0, 0, 0x45, 0, 0, 0, 10]);
            d
        };
        assert_eq!(w33a05_find_glyph_index(&mk(12), 0, 0x43), 12);
        assert_eq!(w33a05_find_glyph_index(&mk(13), 0, 0x43), 10);
        assert_eq!(w33a05_find_glyph_index(&mk(12), 0, 0x46), 0);
        assert_eq!(w33a05_find_glyph_index(&mk(12), 0, 0x40), 0);
    }

    #[test]
    fn format4_delta() {
        // one segment A..C (0x41..0x43) delta 5, plus 0xffff terminator
        let mut d = vec![0, 4, 0, 0, 0, 0];
        d.extend_from_slice(&[0, 4, 0, 4, 0, 0, 0, 0]); // segx2=4 (2 segs), range, sel, shift
        d.extend_from_slice(&[0, 0x43, 0xff, 0xff]); // endCount
        d.extend_from_slice(&[0, 0]); // pad
        d.extend_from_slice(&[0, 0x41, 0xff, 0xff]); // startCount
        d.extend_from_slice(&[0, 5, 0, 1]); // delta
        d.extend_from_slice(&[0, 0, 0, 0]); // rangeOffset
        assert_eq!(w33a05_find_glyph_index(&d, 0, 0x42), 0x47);
        assert_eq!(w33a05_find_glyph_index(&d, 0, 0x10), 0);
        assert_eq!(w33a05_find_glyph_index(&d, 0, 0x10000), 0);
    }

    #[test]
    fn guards() {
        assert_eq!(w33a05_handle_guard(0, || 1), 0x25);
        assert_eq!(w33a05_handle_guard(3, || 1), 1);
        assert_eq!(w33a05_guarded_dispatch(0, 1, 2, |_, _, _, _| 9), 0x25);
        assert_eq!(w33a05_guarded_dispatch(1, 0, 2, |_, _, _, _| 9), 0x21);
        assert_eq!(
            w33a05_guarded_dispatch(1, 5, 2, |h, a, b, c| h + a + b + c),
            3
        );
    }
}
