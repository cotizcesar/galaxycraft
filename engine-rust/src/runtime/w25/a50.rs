#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent50.rs
// ===========================================================================
// Wave 25 agent 50: MSVC `std::string` / small record initialisers.

/// Empty small-string header as left by the MSVC string constructors
/// (data pointer, length, capacity word at offsets 0, 4, 8; inline buffer at 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W25a50StrHeader {
    /// True when the data pointer designates the inline buffer (`this + 12`).
    pub points_inline: bool,
    pub len: u32,
    pub cap: u32,
    pub first_byte: u8,
}

/// `0x008a6990`: initialises an empty string header: pointer to inline
/// buffer, length 0, capacity word `0x8000000f`, first inline byte 0.
pub fn w25a50_string_init_empty() -> W25a50StrHeader {
    W25a50StrHeader {
        points_inline: true,
        len: 0,
        cap: 0x8000_000f,
        first_byte: 0,
    }
}

/// `0x008a86b0`: strlen of a NUL-terminated byte slice (0 when empty or
/// unterminated input ends first), then the same empty header as above.
/// Returns the length handed to the append routine and the header.
pub fn w25a50_string_init_from_cstr(s: &[u8]) -> (usize, W25a50StrHeader) {
    let n = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    (n, w25a50_string_init_empty())
}

/// Four-word record zeroed with the halfword at byte offset 0xe set to 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W25a50Rec16 {
    pub words: [u32; 4],
}

/// `0x008ab630`: zeroes four dwords, sets the word at byte offset 0xe to 3.
pub fn w25a50_rec16_init() -> W25a50Rec16 {
    let mut w = [0u32; 4];
    w[3] = 3u32 << 16;
    W25a50Rec16 { words: w }
}

/// `0x008ae120`: same initialiser as `0x008ab630` (zero 16 bytes, halfword at
/// 0xe = 3) before copying from `param_2 + 0x20`.
pub fn w25a50_rec16_init_copy() -> W25a50Rec16 {
    w25a50_rec16_init()
}

#[cfg(test)]
mod tests_wave25_agent50 {
    use super::*;

    #[test]
    fn string_init() {
        let h = w25a50_string_init_empty();
        assert_eq!((h.len, h.cap, h.first_byte), (0, 0x8000_000f, 0));
        assert!(h.points_inline);
        assert_eq!(w25a50_string_init_from_cstr(b"abc\0x").0, 3);
        assert_eq!(w25a50_string_init_from_cstr(b"\0").0, 0);
        assert_eq!(w25a50_string_init_from_cstr(b"").0, 0);
    }

    #[test]
    fn rec16() {
        let r = w25a50_rec16_init();
        assert_eq!(r.words, [0, 0, 0, 0x0003_0000]);
        assert_eq!(w25a50_rec16_init_copy(), r);
    }
}
