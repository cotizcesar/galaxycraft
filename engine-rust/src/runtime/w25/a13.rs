#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent13.rs
// ===========================================================================
// Wave 25 agent 13: small field setters and string initialisers.

/// Empty small-string header as laid out by `0x007e4e90`: `[ptr, len, cap]`
/// in the 16-byte SSO form (inline buffer at offset 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W25a13EmptyString {
    /// Length field (offset 4).
    pub len: u32,
    /// Capacity/flags field (offset 8).
    pub cap: u32,
    /// First inline byte (offset 12).
    pub first: u8,
}

/// `0x007e4e90`: initialises an empty SSO string: len 0, capacity word
/// `0x8000000f`, first inline byte 0.  The trailing call is not modelled.
pub fn w25a13_init_empty_string() -> W25a13EmptyString {
    W25a13EmptyString {
        len: 0,
        cap: 0x8000_000f,
        first: 0,
    }
}

/// `0x007e4f20`: stores two words at object offsets `0x84` and `0x88`.
pub fn w25a13_set_pair_84(obj: &mut [u8], a: u32, b: u32) {
    obj[0x84..0x88].copy_from_slice(&a.to_le_bytes());
    obj[0x88..0x8c].copy_from_slice(&b.to_le_bytes());
}

/// `0x007e50c0`: stores two words at object offsets `0x20` and `0x24`.
pub fn w25a13_set_pair_20(obj: &mut [u8], a: u32, b: u32) {
    obj[0x20..0x24].copy_from_slice(&a.to_le_bytes());
    obj[0x24..0x28].copy_from_slice(&b.to_le_bytes());
}

#[cfg(test)]
mod tests_wave25_agent13 {
    use super::*;

    #[test]
    fn empty_string() {
        let s = w25a13_init_empty_string();
        assert_eq!((s.len, s.cap, s.first), (0, 0x8000_000f, 0));
    }

    #[test]
    fn setters() {
        let mut o = [0u8; 0x90];
        w25a13_set_pair_84(&mut o, 1, 2);
        assert_eq!(o[0x84], 1);
        assert_eq!(o[0x88], 2);
        w25a13_set_pair_20(&mut o, 3, 4);
        assert_eq!(o[0x20], 3);
        assert_eq!(o[0x24], 4);
    }
}
