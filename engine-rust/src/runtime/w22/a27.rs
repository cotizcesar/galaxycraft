#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent27.rs
// ===========================================================================
// Wave 22 agent 27: buffered 32-bit patch cache helpers.

/// One cache slot (8 bytes in retail: value dword + loaded flag byte).
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct W22a27Slot {
    pub value: u32,
    pub loaded: bool,
}

fn w22a27_apply(cur: u32, value: u32, mode: i32) -> u32 {
    match mode {
        0 => value,
        1 => cur.wrapping_add(value),
        -1 => cur.saturating_sub(value),
        _ => cur,
    }
}

fn w22a27_span(size: u32, off: u32) -> u32 {
    off.abs_diff(size)
}

/// `0x006b2cc0`: read-modify-write of a dword at `base[off]`.  Mode 0 stores
/// `value`, 1 adds, -1 saturating-subtracts.  When `|size - off| < 4` only
/// that many bytes are copied in and out (zero padded).  Returns 0 without a
/// base, otherwise 1.
#[allow(clippy::needless_range_loop)]
pub fn w22a27_patch_dword(
    base: Option<&mut [u8]>,
    off: u32,
    value: u32,
    mode: i32,
    size: u32,
) -> u32 {
    let Some(base) = base else { return 0 };
    let n = w22a27_span(size, off).min(4) as usize;
    let o = off as usize;
    let mut tmp = [0u8; 4];
    for i in 0..n {
        tmp[i] = base.get(o + i).copied().unwrap_or(0);
    }
    let v = w22a27_apply(u32::from_le_bytes(tmp), value, mode);
    let out = v.to_le_bytes();
    for i in 0..n {
        if let Some(b) = base.get_mut(o + i) {
            *b = out[i];
        }
    }
    1
}

/// Shared body of the five cache wrappers: load the slot at `off >> 2` from
/// `base` if not loaded (return 0 if there is no base), apply the mode, then
/// if `ok(new_value)` clear the loaded flag and flush with `0x006b2cc0`
/// semantics; otherwise return 1.
#[allow(clippy::needless_range_loop, clippy::too_many_arguments)]
fn w22a27_cached(
    base: Option<&mut [u8]>,
    cache: &mut [W22a27Slot],
    off: u32,
    value: u32,
    mode: i32,
    size: u32,
    ok: impl Fn(u32) -> bool,
) -> u32 {
    let idx = (off >> 2) as usize;
    if !cache[idx].loaded {
        let Some(b) = base.as_deref() else { return 0 };
        let n = w22a27_span(size, off).min(4) as usize;
        let mut tmp = [0u8; 4];
        for i in 0..n {
            tmp[i] = b.get(off as usize + i).copied().unwrap_or(0);
        }
        cache[idx].value = u32::from_le_bytes(tmp);
        cache[idx].loaded = true;
    }
    cache[idx].value = w22a27_apply(cache[idx].value, value, mode);
    let v = cache[idx].value;
    if ok(v) {
        cache[idx].loaded = false;
        w22a27_patch_dword(base, off, v, 0, size)
    } else {
        1
    }
}

/// `0x006b2110`: cached patch; flushes only when every byte of the result is < 0x12.
pub fn w22a27_cached_bytes_lt_0x12(
    base: Option<&mut [u8]>,
    cache: &mut [W22a27Slot],
    off: u32,
    value: u32,
    mode: i32,
    size: u32,
) -> u32 {
    w22a27_cached(base, cache, off, value, mode, size, |v| {
        v.to_le_bytes().iter().all(|&b| b < 0x12)
    })
}

/// `0x006b2220`: cached patch; flushes only when every byte of the result is < 0xd1.
pub fn w22a27_cached_bytes_lt_0xd1(
    base: Option<&mut [u8]>,
    cache: &mut [W22a27Slot],
    off: u32,
    value: u32,
    mode: i32,
    size: u32,
) -> u32 {
    w22a27_cached(base, cache, off, value, mode, size, |v| {
        v.to_le_bytes().iter().all(|&b| b < 0xd1)
    })
}

/// `0x006b2330`: cached patch; flushes only when low word < 999 and high word < 0x3e7.
pub fn w22a27_cached_words_lt_999(
    base: Option<&mut [u8]>,
    cache: &mut [W22a27Slot],
    off: u32,
    value: u32,
    mode: i32,
    size: u32,
) -> u32 {
    w22a27_cached(base, cache, off, value, mode, size, |v| {
        (v & 0xffff) < 999 && (v >> 16) < 0x3e7
    })
}

/// `0x006b2420`: cached patch; flushes only when the result is <= 7.
pub fn w22a27_cached_le_7(
    base: Option<&mut [u8]>,
    cache: &mut [W22a27Slot],
    off: u32,
    value: u32,
    mode: i32,
    size: u32,
) -> u32 {
    w22a27_cached(base, cache, off, value, mode, size, |v| v <= 7)
}

/// `0x006b2500`: cached patch; flushes only when the result is <= 0x19a.
pub fn w22a27_cached_le_0x19a(
    base: Option<&mut [u8]>,
    cache: &mut [W22a27Slot],
    off: u32,
    value: u32,
    mode: i32,
    size: u32,
) -> u32 {
    w22a27_cached(base, cache, off, value, mode, size, |v| v <= 0x19a)
}

/// `0x006bc690`: tagged-link accessor; returns the link when its low bit is
/// clear and it is non-zero, else 0.
pub fn w22a27_link_or_zero(link: u32) -> u32 {
    if link & 1 == 0 && link != 0 { link } else { 0 }
}

#[cfg(test)]
mod tests_wave22_agent27 {
    use super::*;

    #[test]
    fn patch_modes_and_tail() {
        let mut m = [1u8, 0, 0, 0, 9, 9];
        assert_eq!(w22a27_patch_dword(Some(&mut m), 0, 5, 1, 6), 1);
        assert_eq!(m[0], 6);
        w22a27_patch_dword(Some(&mut m), 0, 10, -1, 6);
        assert_eq!(&m[..4], &[0, 0, 0, 0]);
        w22a27_patch_dword(Some(&mut m), 4, 0x0101, 0, 6);
        assert_eq!(&m[4..], &[1, 1]);
        assert_eq!(w22a27_patch_dword(None, 0, 1, 0, 6), 0);
    }

    #[test]
    fn cached_wrappers() {
        let mut m = [0u8; 8];
        let mut c = [W22a27Slot::default(); 2];
        assert_eq!(w22a27_cached_le_7(None, &mut c, 0, 1, 0, 8), 0);
        // accepted: flushed to memory, slot cleared
        assert_eq!(w22a27_cached_le_7(Some(&mut m), &mut c, 0, 7, 0, 8), 1);
        assert_eq!(m[0], 7);
        assert!(!c[0].loaded);
        // rejected: stays cached, memory untouched
        w22a27_cached_le_7(Some(&mut m), &mut c, 0, 1, 1, 8);
        assert_eq!(m[0], 7);
        assert!(c[0].loaded && c[0].value == 8);
        w22a27_cached_le_0x19a(Some(&mut m), &mut c, 4, 0x19a, 0, 8);
        assert_eq!(&m[4..6], &[0x9a, 1]);
        w22a27_cached_bytes_lt_0x12(Some(&mut m), &mut c, 4, 0x1111_1111, 0, 8);
        assert_eq!(m[4], 0x11);
        w22a27_cached_bytes_lt_0xd1(Some(&mut m), &mut c, 4, 0xd1, 0, 8);
        assert_eq!(m[4], 0x11);
        w22a27_cached_words_lt_999(Some(&mut m), &mut c, 4, 998, 0, 8);
        assert_eq!(&m[4..6], &[0xe6, 3]);
    }

    #[test]
    fn link() {
        assert_eq!(w22a27_link_or_zero(0), 0);
        assert_eq!(w22a27_link_or_zero(5), 0);
        assert_eq!(w22a27_link_or_zero(8), 8);
    }
}
