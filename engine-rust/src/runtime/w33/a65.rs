#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent65.rs
// ===========================================================================
// Wave 33 translation batch 65.
//
// CRT helpers: locale-aware `towlower`-style mapping and a pointer-vector
// "reserve one slot" routine.

/// Vector header used by `0x00cb4865`: `[begin, cur, cap]` as element
/// indices/addresses (stored at offsets 0, 4, 8 of the object).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W33a65PtrVec {
    pub begin: u32,
    pub cur: u32,
    pub cap: u32,
}

/// `0x00cb4865`: ensure room for one more 4-byte entry.  Returns 0 on
/// success or 0xc (ENOMEM) on failure.  `alloc(4 * 4)` allocates the first
/// block (4 entries); `realloc(begin, new_entries)` grows an existing block
/// to twice its entry count (refused at >= 0x80000000 entries).  Both return
/// 0 on failure.
pub fn w33a65_reserve_slot(
    v: &mut W33a65PtrVec,
    alloc: impl FnOnce(u32) -> u32,
    realloc: impl FnOnce(u32, u32) -> u32,
) -> u32 {
    if v.cur != v.cap {
        return 0;
    }
    if v.begin == 0 {
        let p = alloc(4);
        v.begin = p;
        if p != 0 {
            v.cur = p;
            v.cap = p.wrapping_add(0x10);
            return 0;
        }
        return 0xc;
    }
    let n = v.cap.wrapping_sub(v.begin) >> 2;
    if n < 0x8000_0000 {
        let p = realloc(v.begin, n.wrapping_mul(2));
        if p == 0 {
            return 0xc;
        }
        v.begin = p;
        v.cur = p.wrapping_add(n.wrapping_mul(4));
        v.cap = p.wrapping_add(n.wrapping_mul(8));
        return 0;
    }
    0xc
}

/// Locale data consulted by `0x00cb2d30`.
pub struct W33a65Locale<'a> {
    /// Locale code page / name handle at +0xa8 (0 means the "C" locale).
    pub lc_handle: u32,
    /// 256-entry lower-case table at +0x94.
    pub lower_table: &'a [u8; 256],
}

/// `0x00cb2d30`: locale-aware lower-casing of a 16-bit character.
/// `0xffff` passes through.  In the C locale only `A..Z` is mapped.
/// Otherwise values above 0xff go through `lcmap` (None => unchanged) and
/// single-byte values flagged by `is_upper(c)` use the table.
pub fn w33a65_tolower(
    c: u16,
    loc: &W33a65Locale,
    is_upper: impl Fn(u16) -> bool,
    lcmap: impl Fn(u32, u16) -> Option<u16>,
) -> u16 {
    if c == 0xffff {
        return 0xffff;
    }
    if loc.lc_handle == 0 {
        if c.wrapping_sub(0x41) < 0x1a {
            return c + 0x20;
        }
        return c;
    }
    if c > 0xff {
        return lcmap(loc.lc_handle, c).unwrap_or(c);
    }
    if is_upper(c) {
        return loc.lower_table[c as usize] as u16;
    }
    c
}

#[cfg(test)]
mod tests_wave33_agent65 {
    use super::*;

    #[test]
    fn reserve() {
        let mut v = W33a65PtrVec::default();
        assert_eq!(w33a65_reserve_slot(&mut v, |_| 0x1000, |_, _| 0), 0);
        assert_eq!((v.begin, v.cur, v.cap), (0x1000, 0x1000, 0x1010));
        // room left: nothing happens
        v.cur = 0x1004;
        assert_eq!(w33a65_reserve_slot(&mut v, |_| 0, |_, _| 0), 0);
        v.cur = v.cap;
        assert_eq!(
            w33a65_reserve_slot(
                &mut v,
                |_| 0,
                |_, n| {
                    assert_eq!(n, 8);
                    0x2000
                }
            ),
            0
        );
        assert_eq!((v.begin, v.cur, v.cap), (0x2000, 0x2010, 0x2020));
        assert_eq!(w33a65_reserve_slot(&mut v, |_| 0, |_, _| 0), 0);
        v.cur = v.cap;
        assert_eq!(w33a65_reserve_slot(&mut v, |_| 0, |_, _| 0), 0xc);
        let mut e = W33a65PtrVec::default();
        assert_eq!(w33a65_reserve_slot(&mut e, |_| 0, |_, _| 0), 0xc);
    }

    #[test]
    fn lower() {
        let mut t = [0u8; 256];
        t[0xc0] = 0xe0;
        let c = W33a65Locale {
            lc_handle: 0,
            lower_table: &t,
        };
        let l = W33a65Locale {
            lc_handle: 5,
            lower_table: &t,
        };
        let up = |c: u16| c == 0xc0;
        assert_eq!(w33a65_tolower(0xffff, &c, up, |_, _| None), 0xffff);
        assert_eq!(w33a65_tolower(0x41, &c, up, |_, _| None), 0x61);
        assert_eq!(w33a65_tolower(0x5b, &c, up, |_, _| None), 0x5b);
        assert_eq!(w33a65_tolower(0xc0, &c, up, |_, _| None), 0xc0);
        assert_eq!(w33a65_tolower(0xc0, &l, up, |_, _| None), 0xe0);
        assert_eq!(w33a65_tolower(0x100, &l, up, |_, c| Some(c + 1)), 0x101);
        assert_eq!(w33a65_tolower(0x100, &l, up, |_, _| None), 0x100);
    }
}
