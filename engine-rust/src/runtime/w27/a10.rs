#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent10.rs
// ===========================================================================
// Wave 27 translation batch 10.

/// `0x009763c0`: clears every entry of the 25-slot table equal to `value`
/// and each of the four extra single cells (`extra`) equal to `value`.
pub fn w27a10_clear_matching(slots: &mut [i32; 25], extra: &mut [i32; 4], value: i32) {
    for s in slots.iter_mut().chain(extra.iter_mut()) {
        if *s == value {
            *s = 0;
        }
    }
}

/// Small-string header (MSVC SSO layout): data pointer slot, size, capacity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W27a10SsoString {
    pub size: u32,
    pub cap: u32,
    pub buf: [u8; 16],
}

/// `0x00977650`: initialises an empty small string (size 0, capacity
/// `0x8000000f`, first byte 0) before the append of a 9-byte literal.
pub fn w27a10_sso_init_empty() -> W27a10SsoString {
    W27a10SsoString {
        size: 0,
        cap: 0x8000_000f,
        buf: [0; 16],
    }
}

/// `0x009774c0`: same empty init as the routine above; returns the string
/// plus the C-string length (up to the first NUL) handed to the append.
pub fn w27a10_sso_init_from_cstr(src: &[u8]) -> (W27a10SsoString, usize) {
    let n = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    (w27a10_sso_init_empty(), n)
}

/// Shared tail of `0x00975d10` and `0x00975e50` on a window record.
/// Returns false (the original does not return) when any pending slot is set.
/// `delta` is the base value of the full-width step; it is halved when the
/// record is neither of the two predicate kinds.
fn w27a10_window_tail(
    rec: &mut [u8; 0x6c],
    pending: &[i32],
    arg: i32,
    kind_a: bool,
    kind_b: bool,
    step: i32,
) -> bool {
    if pending.iter().any(|&p| p != 0) {
        return false;
    }
    let rd32 = |r: &[u8], o: usize| u32::from_le_bytes([r[o], r[o + 1], r[o + 2], r[o + 3]]);
    let rd16 = |r: &[u8], o: usize| i16::from_le_bytes([r[o], r[o + 1]]);
    let f = rd32(rec, 0x30) | 0x8000_0000 | 0x4400_0000;
    rec[0x30..0x34].copy_from_slice(&f.to_le_bytes());
    rec[0x64..0x68].fill(0);
    if arg != 0 {
        rec[0x44..0x48].copy_from_slice(&arg.to_le_bytes());
    }
    rec[0x3a..0x3c].fill(0);
    let mut d = 0i32;
    if !kind_a {
        d = step;
        if !kind_b {
            d /= 2;
        }
    }
    let d16 = d as i16;
    let x0 = rd16(rec, 4).wrapping_add(d16);
    rec[4..6].copy_from_slice(&x0.to_le_bytes());
    let x1 = x0.wrapping_add(rd16(rec, 8));
    rec[8..10].copy_from_slice(&x1.to_le_bytes());
    rec[0x68..0x6a].copy_from_slice(&d16.to_le_bytes());
    let y1 = rd16(rec, 6).wrapping_add(rd16(rec, 10));
    rec[10..12].copy_from_slice(&y1.to_le_bytes());
    true
}

/// `0x00975d10`: window-record reset (flags, optional field at +0x44, x
/// offset adjust).  `step` is the callee value `FUN_007b30e0`.
pub fn w27a10_window_reset(
    rec: &mut [u8; 0x6c],
    pending: &[i32],
    arg: i32,
    kind_a: bool,
    kind_b: bool,
    step: i32,
) -> bool {
    w27a10_window_tail(rec, pending, arg, kind_a, kind_b, step)
}

/// `0x00975e50`: identical body to the one above with the argument in the
/// third parameter position.
pub fn w27a10_window_reset3(
    rec: &mut [u8; 0x6c],
    pending: &[i32],
    arg: i32,
    kind_a: bool,
    kind_b: bool,
    step: i32,
) -> bool {
    w27a10_window_tail(rec, pending, arg, kind_a, kind_b, step)
}

#[cfg(test)]
mod tests_wave27_agent10 {
    use super::*;

    #[test]
    fn clear() {
        let mut s = [0; 25];
        s[3] = 7;
        s[24] = 7;
        s[5] = 8;
        let mut e = [7, 1, 7, 0];
        w27a10_clear_matching(&mut s, &mut e, 7);
        assert_eq!((s[3], s[24], s[5]), (0, 0, 8));
        assert_eq!(e, [0, 1, 0, 0]);
    }

    #[test]
    fn sso() {
        let s = w27a10_sso_init_empty();
        assert_eq!((s.size, s.cap, s.buf[0]), (0, 0x8000_000f, 0));
        assert_eq!(w27a10_sso_init_from_cstr(b"abc\0d").1, 3);
        assert_eq!(w27a10_sso_init_from_cstr(b"").1, 0);
    }

    #[test]
    fn window() {
        let mut r = [0u8; 0x6c];
        r[4] = 10;
        r[8] = 5;
        r[6] = 2;
        r[10] = 3;
        assert!(!w27a10_window_reset(&mut r, &[0, 1], 0, false, false, 8));
        assert!(w27a10_window_reset(&mut r, &[0, 0], 9, false, false, 8));
        assert_eq!(r[4], 14);
        assert_eq!(r[8], 19);
        assert_eq!(r[10], 5);
        assert_eq!(r[0x68], 4);
        assert_eq!(r[0x44], 9);
        assert_eq!(r[0x33], 0xc4);
        let mut r2 = [0u8; 0x6c];
        assert!(w27a10_window_reset3(&mut r2, &[], 0, false, true, 8));
        assert_eq!(r2[0x68], 8);
        assert!(w27a10_window_reset3(&mut r2, &[], 0, true, true, 8));
        assert_eq!(r2[0x68], 0);
    }
}
