#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent70.rs
// ===========================================================================
// Wave 32 translation batch 70.

/// `0x009866f0`: writes an object's name string into a caller sink.  The
/// byte count is clamped to 0x100; when flag 0x1000000 of `flags` is set the
/// string is wide and only `count / 2` units are copied.  A missing string
/// uses `(null)` (wide) or the default narrow text.  Returns 0x25 when there
/// is no destination, else 0.
pub fn w32a70_copy_name(
    has_dest: bool,
    requested: i32,
    flags: u32,
    narrow: Option<&[u8]>,
    wide: Option<&[u16]>,
    default_narrow: &[u8],
    mut copy_narrow: impl FnMut(&[u8], i32),
    mut copy_wide: impl FnMut(&[u16], i32),
) -> u32 {
    if !has_dest {
        return 0x25;
    }
    let count = if requested < 0x101 { requested } else { 0x100 };
    let is_wide = flags & 0x100_0000 != 0;
    if is_wide {
        match wide {
            Some(w) => copy_wide(w, count / 2),
            None => {
                let null: Vec<u16> = "(null)".encode_utf16().chain(Some(0)).collect();
                copy_wide(&null, count / 2)
            }
        }
    } else {
        match narrow {
            Some(n) => copy_narrow(n, count),
            None => copy_narrow(default_narrow, count),
        }
    }
    0
}

/// `0x00986db0`: reports a surface/format description.  Optionally copies a
/// name, then fills `out` by `query` (2 = field 0x18, 1 = `query_one`, 4 =
/// byte size computed from `format`, `row_count` and `class_map`).
/// Returns 0x25 on missing arguments, else 0.
pub fn w32a70_format_query(
    has_obj: bool,
    has_name_dest: bool,
    name_copy: &mut dyn FnMut(),
    out: Option<&mut u32>,
    query: u32,
    field18: u32,
    format: u32,
    row_count: u32,
    class_map: &[u8],
    query_one: u32,
) -> u32 {
    if !has_obj {
        return 0x25;
    }
    let out = if has_name_dest {
        name_copy();
        match out {
            None => return 0,
            Some(o) => o,
        }
    } else {
        match out {
            None => return 0x25,
            Some(o) => o,
        }
    };
    match query {
        2 => {
            *out = field18;
            return 0;
        }
        1 => {
            *out = query_one;
            return 0;
        }
        4 => {}
        _ => return 0,
    }
    let u = field18;
    let bits = match format {
        0 | 6..=15 => {
            let c = class_map.get(format as usize).copied().unwrap_or(5);
            match c {
                0 => *out = 0,
                1 => {
                    let v = ((u as u64 + 0xd) * 0x1_2492_4925) >> 0x21;
                    *out = ((v as u32) & 0xffff_fff8).wrapping_mul(row_count);
                }
                2 => {
                    *out = (u.wrapping_add(0x3f) >> 6)
                        .wrapping_mul(0x24)
                        .wrapping_mul(row_count)
                }
                3 => {
                    let v = ((u as u64 + 0x1b) * 0x1_2492_4925) >> 0x21;
                    *out = ((v as u32) & 0xffff_fff0).wrapping_mul(row_count);
                }
                4 => *out = u,
                _ => {}
            }
            return 0;
        }
        1 => 8u64,
        2 => 0x10,
        3 => 0x18,
        4 | 5 => 0x20,
        _ => return 0,
    };
    *out = (((bits * u as u64) >> 3) as u32).wrapping_mul(row_count);
    0
}

#[cfg(test)]
mod tests_wave32_agent70 {
    use super::*;

    #[test]
    fn copy_name_clamps() {
        let mut got = (0, 0);
        let r = w32a70_copy_name(
            true,
            999,
            0,
            Some(b"ab"),
            None,
            b"x",
            |_, n| got.0 = n,
            |_, _| {},
        );
        assert_eq!((r, got.0), (0, 0x100));
        let r = w32a70_copy_name(
            true,
            10,
            0x100_0000,
            None,
            None,
            b"x",
            |_, _| {},
            |s, n| got.1 = n + s.len() as i32,
        );
        assert_eq!((r, got.1), (0, 5 + 7));
        assert_eq!(
            w32a70_copy_name(false, 1, 0, None, None, b"", |_, _| {}, |_, _| {}),
            0x25
        );
    }

    #[test]
    fn format_query_sizes() {
        let mut o = 0;
        w32a70_format_query(true, false, &mut || {}, Some(&mut o), 4, 16, 3, 2, &[], 0);
        assert_eq!(o, ((0x18 * 16) >> 3) * 2);
        w32a70_format_query(true, false, &mut || {}, Some(&mut o), 2, 77, 0, 0, &[], 0);
        assert_eq!(o, 77);
        assert_eq!(
            w32a70_format_query(true, false, &mut || {}, None, 2, 0, 0, 0, &[], 0),
            0x25
        );
        w32a70_format_query(
            true,
            false,
            &mut || {},
            Some(&mut o),
            4,
            100,
            6,
            3,
            &[0, 0, 0, 0, 0, 0, 2],
            0,
        );
        assert_eq!(o, (100 + 0x3f >> 6) * 0x24 * 3);
    }
}
