#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent51.rs
// ===========================================================================
// Wave 32 agent 51: bit-vector iterator and string compare helpers.

/// `0x00802f60`: converts a bit-vector size/offset `bits` (signed view of
/// the u32 at +12) into a `(word_byte_offset, bit_index)` pair relative to
/// the base pointer.  Negative values round toward the previous word.
pub fn w32a51_bit_position(base: i32, bits: u32) -> (i32, u32) {
    let off = if (bits as i32) < 0 && bits != 0 {
        -((((!bits) >> 5) * 4 + 4) as i32)
    } else {
        ((bits >> 5) * 4) as i32
    };
    (base.wrapping_add(off), bits & 0x1f)
}

/// `0x00814cc0`: compares a counted buffer (`own`) with a NUL-terminated
/// string.  Compares `min(len, own.len())` bytes (memcmp, result -1/0/1);
/// when equal returns `own.len() - len` (wrapping, as unsigned).
pub fn w32a51_compare_cstr(own: &[u8], other: &[u8]) -> i32 {
    let len = other.iter().position(|&b| b == 0).unwrap_or(other.len());
    let n = len.min(own.len());
    for i in 0..n {
        if own[i] != other[i] {
            return if own[i] < other[i] { -1 } else { 1 };
        }
    }
    (own.len() as u32).wrapping_sub(len as u32) as i32
}

#[cfg(test)]
mod tests_wave32_agent51 {
    use super::*;

    #[test]
    fn bit_position() {
        assert_eq!(w32a51_bit_position(100, 0), (100, 0));
        assert_eq!(w32a51_bit_position(100, 33), (104, 1));
        assert_eq!(w32a51_bit_position(100, 0xFFFF_FFFF), (96, 31));
    }

    #[test]
    fn compare() {
        assert_eq!(w32a51_compare_cstr(b"abc", b"abc\0"), 0);
        assert_eq!(w32a51_compare_cstr(b"abcd", b"abc\0"), 1);
        assert_eq!(w32a51_compare_cstr(b"ab", b"abc\0"), -1);
        assert_eq!(w32a51_compare_cstr(b"abd", b"abc\0"), 1);
        assert_eq!(w32a51_compare_cstr(b"abb", b"abc\0"), -1);
    }
}
