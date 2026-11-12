#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent71.rs
// ===========================================================================
// Wave 33 agent 71: UTF-16 alpha run length and UTF-8 encoder helpers.

/// `0x00cb70f6`: counts the leading ASCII letters (A-Z, a-z) of a UTF-16
/// buffer; stops at the first other unit (or the end of the slice).
pub fn w33a71_ascii_alpha_run_len(s: &[u16]) -> i32 {
    let mut n = 0;
    for &c in s {
        let is_alpha = (0x41..=0x5a).contains(&c) || (0x61..=0x7a).contains(&c);
        if !is_alpha {
            break;
        }
        n += 1;
    }
    n
}

/// `0x00cb7d54`: encodes one code point as UTF-8 into `out`, clearing the
/// two-word conversion state `state` on success.  A missing buffer or a zero
/// code point returns 1 (zero writes a NUL byte when a buffer exists).
/// Surrogates and values above 0x10ffff call `on_invalid` (the retail
/// error helper `FUN_00cb7e03`) and return its result.
pub fn w33a71_utf8_encode(
    out: Option<&mut [u8]>,
    cp: u32,
    state: &mut [u32; 2],
    on_invalid: impl FnOnce(&mut [u32; 2]) -> i32,
) -> i32 {
    let Some(out) = out else {
        *state = [0, 0];
        return 1;
    };
    if cp == 0 {
        out[0] = 0;
        *state = [0, 0];
        return 1;
    }
    if cp < 0x80 {
        out[0] = cp as u8;
        return 1;
    }
    let (n, lead): (usize, u8) = if cp < 0x800 {
        (1, 0xc0)
    } else if cp < 0x10000 {
        if (0xd800..0xe000).contains(&cp) {
            return on_invalid(state);
        }
        (2, 0xe0)
    } else {
        if cp > 0x10ffff {
            return on_invalid(state);
        }
        (3, 0xf0)
    };
    let mut v = cp;
    for i in (1..=n).rev() {
        out[i] = (v as u8 & 0x3f) | 0x80;
        v >>= 6;
    }
    out[0] = v as u8 | lead;
    *state = [0, 0];
    n as i32 + 1
}

#[cfg(test)]
mod tests_wave33_agent71 {
    use super::*;

    #[test]
    fn alpha_run() {
        let s: Vec<u16> = "abcZ1x".encode_utf16().collect();
        assert_eq!(w33a71_ascii_alpha_run_len(&s), 4);
        assert_eq!(w33a71_ascii_alpha_run_len(&[0x40]), 0);
    }

    #[test]
    fn utf8() {
        for cp in [0x24u32, 0xa2, 0x20ac, 0x10348] {
            let mut b = [0u8; 4];
            let mut st = [1, 1];
            let n = w33a71_utf8_encode(Some(&mut b), cp, &mut st, |_| -1);
            let want = char::from_u32(cp).unwrap().to_string();
            assert_eq!(&b[..n as usize], want.as_bytes());
        }
        let mut st = [1, 1];
        let mut b = [0u8; 4];
        assert_eq!(
            w33a71_utf8_encode(Some(&mut b), 0xd800, &mut st, |_| -7),
            -7
        );
        assert_eq!(w33a71_utf8_encode(None, 5, &mut st, |_| -7), 1);
        assert_eq!(st, [0, 0]);
    }
}
