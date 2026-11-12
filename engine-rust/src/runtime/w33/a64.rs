#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent64.rs
// ===========================================================================
// Wave 33 agent 64: CRT multibyte length helpers.

/// `0x00cb2b38`: length class of a NUL-terminated byte run: 1 if the first
/// byte is zero, 2 if the second is zero, otherwise 3 or 4 depending on the
/// third byte.
pub fn w33a64_prefix_class(p: &[u8; 3]) -> u8 {
    if p[0] == 0 {
        1
    } else if p[1] == 0 {
        2
    } else {
        (p[2] != 0) as u8 + 3
    }
}

/// `0x00cb2b64`: calls the decoder (`FUN_00cbaa59`, supplied by the caller)
/// which yields `(status, value)`; when status < 5 the value is clamped to
/// 0xfffd if above 0xffff and stored into `out` (if present).  Returns the
/// status.
pub fn w33a64_decode_u16<F: FnOnce() -> (u32, u32)>(out: Option<&mut u16>, decode: F) -> u32 {
    let (status, mut v) = decode();
    if status < 5 {
        if v > 0xffff {
            v = 0xfffd;
        }
        if let Some(o) = out {
            *o = v as u16;
        }
    }
    status
}

#[cfg(test)]
mod tests_wave33_agent64 {
    use super::*;

    #[test]
    fn class() {
        assert_eq!(w33a64_prefix_class(&[0, 1, 1]), 1);
        assert_eq!(w33a64_prefix_class(&[1, 0, 1]), 2);
        assert_eq!(w33a64_prefix_class(&[1, 1, 0]), 3);
        assert_eq!(w33a64_prefix_class(&[1, 1, 1]), 4);
    }

    #[test]
    fn decode() {
        let mut o = 0u16;
        assert_eq!(w33a64_decode_u16(Some(&mut o), || (2, 0x12345)), 2);
        assert_eq!(o, 0xfffd);
        assert_eq!(w33a64_decode_u16(Some(&mut o), || (5, 7)), 5);
        assert_eq!(o, 0xfffd);
        assert_eq!(w33a64_decode_u16(Some(&mut o), || (1, 7)), 1);
        assert_eq!(o, 7);
        assert_eq!(w33a64_decode_u16(None, || (0, 1)), 0);
    }
}
