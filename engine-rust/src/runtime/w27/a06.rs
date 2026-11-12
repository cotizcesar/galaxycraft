#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent06.rs
// ===========================================================================
// Wave 27 agent 06: flag setters and vector growth helpers.

fn w27a06_set_bit(word: &mut u32, mask: u32, on: bool) {
    if on {
        *word |= mask;
    } else {
        *word &= !mask;
    }
}

/// `0x00964940`: sets/clears bit 3 (0x8) of the dword at +0x34.
pub fn w27a06_set_flag34_08(w34: &mut u32, on: bool) {
    w27a06_set_bit(w34, 0x8, on);
}

/// `0x00964da0`: sets/clears bit 2 (0x4) of the dword at +0x34.
pub fn w27a06_set_flag34_04(w34: &mut u32, on: bool) {
    w27a06_set_bit(w34, 0x4, on);
}

/// `0x00964dc0`: sets/clears bit 4 (0x10) of the dword at +0x34.
pub fn w27a06_set_flag34_10(w34: &mut u32, on: bool) {
    w27a06_set_bit(w34, 0x10, on);
}

/// `0x00964e10`: sets/clears bit 5 (0x20) of the dword at +0x34.
pub fn w27a06_set_flag34_20(w34: &mut u32, on: bool) {
    w27a06_set_bit(w34, 0x20, on);
}

/// `0x00964f60`: sets/clears bit 11 (0x800) of the dword at +0x30.
pub fn w27a06_set_flag30_800(w30: &mut u32, on: bool) {
    w27a06_set_bit(w30, 0x800, on);
}

/// `0x00964fe0`: sets/clears bit 1 (0x2) of the dword at +0x30.
pub fn w27a06_set_flag30_02(w30: &mut u32, on: bool) {
    w27a06_set_bit(w30, 0x2, on);
}

/// `0x00965200`: returns bit 3 of the byte at +10 (0 or 1).
pub fn w27a06_byte10_bit3(byte10: u8) -> u32 {
    ((byte10 >> 3) & 1) as u32
}

/// Vector header used by the three growth helpers (`ECX` layout).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W27a06Vec {
    /// +0x00: allocator/proxy word (0 with null buffer is a fatal path).
    pub proxy: u32,
    /// +0x08: buffer start (0 = unallocated).
    pub begin: u32,
    /// +0x0c: used end.
    pub end: u32,
    /// +0x10: capacity end.
    pub cap_end: u32,
    /// +0x14: default capacity used when unallocated.
    pub default_cap: u32,
}

/// Shared body of `0x009664c0`, `0x00966540` and `0x009665c0`.  `want` is
/// the byte count requested; `realloc(old_begin, bytes)` returns the new
/// buffer (0 when `bytes == 0`, which frees).  Returns `None` for the
/// non-returning null-proxy path.
fn w27a06_grow(
    v: &mut W27a06Vec,
    want_bytes: u32,
    mut realloc: impl FnMut(u32, u32) -> u32,
) -> Option<()> {
    let old = v.begin;
    let cur_cap = if old == 0 {
        if v.proxy == 0 {
            return None;
        }
        v.default_cap
    } else {
        let used = v.cap_end.wrapping_sub(old);
        used.wrapping_add(used.wrapping_add(1) >> 1)
    };
    let len = v.end;
    let mut new_cap = want_bytes.wrapping_add(len.wrapping_sub(old));
    if new_cap <= cur_cap {
        new_cap = cur_cap;
    }
    let nb = if new_cap == 0 {
        realloc(old, 0);
        0
    } else {
        realloc(old, new_cap)
    };
    v.begin = nb;
    v.end = nb.wrapping_add(len.wrapping_sub(old));
    v.cap_end = nb.wrapping_add(new_cap);
    Some(())
}

/// `0x009664c0`: grows by `extra` bytes (element size 1).
pub fn w27a06_grow_bytes(
    v: &mut W27a06Vec,
    extra: u32,
    realloc: impl FnMut(u32, u32) -> u32,
) -> Option<()> {
    w27a06_grow(v, extra, realloc)
}

/// `0x00966540`: grows by `count` 4-byte elements.
pub fn w27a06_grow_dwords(
    v: &mut W27a06Vec,
    count: u32,
    realloc: impl FnMut(u32, u32) -> u32,
) -> Option<()> {
    w27a06_grow(v, count.wrapping_mul(4), realloc)
}

/// `0x009665c0`: grows by `count` 0x44-byte elements.
pub fn w27a06_grow_rec44(
    v: &mut W27a06Vec,
    count: u32,
    realloc: impl FnMut(u32, u32) -> u32,
) -> Option<()> {
    w27a06_grow(v, count.wrapping_mul(0x44), realloc)
}

#[cfg(test)]
mod tests_wave27_agent06 {
    use super::*;

    #[test]
    fn flags() {
        let mut w = 0u32;
        w27a06_set_flag34_08(&mut w, true);
        w27a06_set_flag34_04(&mut w, true);
        w27a06_set_flag34_10(&mut w, true);
        w27a06_set_flag34_20(&mut w, true);
        assert_eq!(w, 0x3c);
        w27a06_set_flag34_08(&mut w, false);
        w27a06_set_flag34_04(&mut w, false);
        w27a06_set_flag34_10(&mut w, false);
        w27a06_set_flag34_20(&mut w, false);
        assert_eq!(w, 0);
        let mut x = 0xffff_ffffu32;
        w27a06_set_flag30_800(&mut x, false);
        w27a06_set_flag30_02(&mut x, false);
        assert_eq!(x, 0xffff_f7fd);
        w27a06_set_flag30_800(&mut x, true);
        w27a06_set_flag30_02(&mut x, true);
        assert_eq!(x, 0xffff_ffff);
    }

    #[test]
    fn bit3() {
        assert_eq!(w27a06_byte10_bit3(0x08), 1);
        assert_eq!(w27a06_byte10_bit3(0xf7), 0);
    }

    #[test]
    fn grow() {
        let mut v = W27a06Vec {
            proxy: 0,
            ..Default::default()
        };
        assert!(w27a06_grow_bytes(&mut v, 4, |_, _| 1).is_none());
        let mut v = W27a06Vec {
            proxy: 1,
            default_cap: 16,
            ..Default::default()
        };
        let mut calls = vec![];
        w27a06_grow_dwords(&mut v, 8, |o, n| {
            calls.push((o, n));
            1000
        })
        .unwrap();
        assert_eq!(calls, vec![(0, 32)]);
        assert_eq!((v.begin, v.end, v.cap_end), (1000, 1000, 1032));
        let mut v = W27a06Vec {
            proxy: 1,
            begin: 100,
            end: 110,
            cap_end: 120,
            default_cap: 0,
        };
        w27a06_grow_bytes(&mut v, 1, |_, n| {
            assert_eq!(n, 30);
            200
        })
        .unwrap();
        assert_eq!((v.begin, v.end, v.cap_end), (200, 210, 230));
        let mut v = W27a06Vec {
            proxy: 1,
            begin: 0,
            end: 0,
            cap_end: 0,
            default_cap: 0,
        };
        w27a06_grow_rec44(&mut v, 0, |_, n| {
            assert_eq!(n, 0);
            0
        })
        .unwrap();
        assert_eq!(v.cap_end, 0);
    }
}
