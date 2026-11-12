#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent08.rs
// ===========================================================================
// Wave 33 agent 08.

/// Stream state touched by the seek routine (dword indices of the retail object).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W33a08Stream {
    /// dword 5: size limit, `-1` (0xffffffff) means unbounded.
    pub limit: u32,
    /// byte at dword 0xb.
    pub flag_b: u8,
    /// dword 0x5e: base offset.
    pub base: u32,
    /// dword 0x5b: current position.
    pub pos: u32,
    /// dword 0x59.
    pub w59: u32,
    /// dword 0x57.
    pub w57: u32,
    /// dword 0x5c.
    pub w5c: u32,
    /// dword 99 (bit 0 tested).
    pub flags: u32,
    /// dword 0x54 (divisor for `w55`).
    pub w54: u32,
    /// dword 0x55.
    pub w55: u32,
    /// dword 0x56.
    pub w56: u32,
}

/// `0x009a16c0`: seek.  `whence` 0 = from base, 1 = from current, 2 = from
/// base + limit.  Returns 0x25 for a bad whence, 0x14 for out of range,
/// otherwise 0 or the result of `backend_seek(new_pos)` when `w57 == 0`.
pub fn w33a08_seek(
    s: &mut W33a08Stream,
    offset: i32,
    whence: i32,
    mut backend_seek: impl FnMut(u32) -> u32,
) -> u32 {
    if !(0..=2).contains(&whence) {
        return 0x25;
    }
    s.flag_b = 0;
    let start = match whence {
        0 => s.base,
        1 => s.pos,
        _ => s.base.wrapping_add(s.limit),
    };
    let mut p = start.wrapping_add(offset as u32);
    if s.limit != u32::MAX {
        let top = s.base.wrapping_add(s.limit);
        if top < p {
            p = 0;
            if offset >= 0 {
                p = top;
            }
        }
    }
    if s.w5c == 0 && s.flags & 1 == 0 && s.w59 <= p {
        return 0x14;
    }
    if s.w59 == s.w57 && s.flags & 1 == 0 {
        let (a, b) = (s.w59, s.w5c);
        if a <= b {
            if b != 0 && p < b - a {
                return 0x14;
            }
            if a.wrapping_add(b) <= p {
                return 0x14;
            }
        }
    }
    s.pos = p;
    if s.w54 != 0 {
        s.w55 = p % s.w54;
    }
    if s.w57 == 0 {
        return backend_seek(p);
    }
    s.w56 = p % s.w57;
    0
}

/// `0x00a5ffd0`: forwards its four arguments to the worker with a leading 0.
pub fn w33a08_forward4<R>(
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    mut worker: impl FnMut(u32, u32, u32, u32, u32) -> R,
) -> R {
    worker(0, a, b, c, d)
}

#[cfg(test)]
mod tests_wave33_agent08 {
    use super::*;
    #[test]
    fn seek_basic() {
        let mut s = W33a08Stream {
            limit: u32::MAX,
            base: 10,
            w59: 100,
            w57: 0,
            ..Default::default()
        };
        assert_eq!(w33a08_seek(&mut s, 0, 3, |_| 0), 0x25);
        assert_eq!(w33a08_seek(&mut s, 5, 0, |p| p + 1), 16);
        assert_eq!(s.pos, 15);
        assert_eq!(w33a08_seek(&mut s, 500, 1, |_| 0), 0x14);
    }
    #[test]
    fn seek_mod_and_clamp() {
        let mut s = W33a08Stream {
            limit: 20,
            base: 0,
            w59: 100,
            w57: 7,
            w5c: 1,
            w54: 4,
            ..Default::default()
        };
        assert_eq!(w33a08_seek(&mut s, 50, 0, |_| 9), 0);
        assert_eq!(s.pos, 20);
        assert_eq!(s.w56, 6);
        assert_eq!(s.w55, 0);
    }
    #[test]
    fn fwd() {
        assert_eq!(
            w33a08_forward4(1, 2, 3, 4, |z, a, b, c, d| z + a + b + c + d),
            10
        );
    }
}
