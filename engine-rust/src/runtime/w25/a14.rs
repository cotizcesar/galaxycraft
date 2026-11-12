#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent14.rs
// ===========================================================================
// Wave 25 agent 14: small MSVC `std::string` (SSO) initialisers.

/// Empty small-string header: length 0, capacity word `0x8000000f`, inline buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W25a14EmptyString {
    pub len: u32,
    pub cap_word: u32,
    pub first_byte: u8,
}

/// Header value shared by every initialiser in this batch.
pub fn w25a14_empty() -> W25a14EmptyString {
    W25a14EmptyString {
        len: 0,
        cap_word: 0x8000_000f,
        first_byte: 0,
    }
}

/// `0x007e7b80`: resets a string to empty and returns the length field
/// (`src[1]`) that the following assign call receives.
pub fn w25a14_7b80(src: &[u32; 2]) -> (W25a14EmptyString, u32) {
    (w25a14_empty(), src[1])
}

/// `0x007e8790`: copies the first two words of `src`, resets the embedded
/// string to empty and returns `src[3]` as the assign length.
pub fn w25a14_8790(src: &[u32; 4]) -> ([u32; 2], W25a14EmptyString, u32) {
    ([src[0], src[1]], w25a14_empty(), src[3])
}

/// `0x007e80e0`: resets a string to empty and returns the length handed to the
/// assign call for selector `sel`: 0 -> 4, 1 -> 6, otherwise 7.
pub fn w25a14_80e0(sel: i32) -> (W25a14EmptyString, u32) {
    let n = match sel {
        0 => 4,
        1 => 6,
        _ => 7,
    };
    (w25a14_empty(), n)
}

/// `0x007e81b0`: empty string plus the length at record offset 4.
pub fn w25a14_81b0(record: &[u32]) -> Option<(W25a14EmptyString, u32)> {
    record.get(1).map(|&n| (w25a14_empty(), n))
}

/// `0x007e83d0`: empty string plus the length at record byte offset `0x58`.
pub fn w25a14_83d0(record: &[u32]) -> Option<(W25a14EmptyString, u32)> {
    record.get(0x58 / 4).map(|&n| (w25a14_empty(), n))
}

/// `0x007e8430`: empty string plus the length at record byte offset `0x3c`.
pub fn w25a14_8430(record: &[u32]) -> Option<(W25a14EmptyString, u32)> {
    record.get(0x3c / 4).map(|&n| (w25a14_empty(), n))
}

#[cfg(test)]
mod tests_wave25_agent14 {
    use super::*;

    #[test]
    fn init_helpers() {
        assert_eq!(w25a14_7b80(&[1, 9]).1, 9);
        let (c, e, n) = w25a14_8790(&[1, 2, 3, 4]);
        assert_eq!((c, e.cap_word, n), ([1, 2], 0x8000_000f, 4));
        assert_eq!(w25a14_80e0(0).1, 4);
        assert_eq!(w25a14_80e0(1).1, 6);
        assert_eq!(w25a14_80e0(5).1, 7);
    }

    #[test]
    fn record_offsets() {
        let mut r = [0u32; 0x20];
        r[1] = 11;
        r[0x58 / 4] = 22;
        r[0x3c / 4] = 33;
        assert_eq!(w25a14_81b0(&r).unwrap().1, 11);
        assert_eq!(w25a14_83d0(&r).unwrap().1, 22);
        assert_eq!(w25a14_8430(&r).unwrap().1, 33);
        assert!(w25a14_83d0(&r[..4]).is_none());
        assert!(w25a14_81b0(&[]).is_none());
    }
}
