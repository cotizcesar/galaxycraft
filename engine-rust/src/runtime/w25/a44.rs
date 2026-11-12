#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent44.rs
// ===========================================================================
/// `0x0088c9c0`: scans four 0x24-stride slots starting at offset `0x1b8`
/// of the decoded table; the first whose state byte equals `0x0c` yields
/// `0x80 + index`, otherwise 0.
pub fn w25a44_find_state_c_slot(table: &[u8]) -> u32 {
    for i in 0..4usize {
        if table.get(0x1b8 + i * 0x24).copied() == Some(0x0c) {
            return 0x80 + i as u32;
        }
    }
    0
}

/// `0x0088cf90`: initialises an empty small-string header
/// (`ptr = inline buffer`, `len = 0`, `cap_flags = 0x8000000f`, byte 0 = 0).
/// Returns `(len, cap_flags, first_byte)`.
pub fn w25a44_empty_string_header() -> (u32, u32, u8) {
    (0, 0x8000_000f, 0)
}

#[cfg(test)]
mod tests_wave25_agent44 {
    use super::*;

    #[test]
    fn slot_scan() {
        let mut t = vec![0u8; 0x240];
        assert_eq!(w25a44_find_state_c_slot(&t), 0);
        t[0x1b8 + 0x24 * 3] = 0x0c;
        assert_eq!(w25a44_find_state_c_slot(&t), 0x83);
        t[0x1b8 + 0x24] = 0x0c;
        assert_eq!(w25a44_find_state_c_slot(&t), 0x81);
        t[0x1b8] = 0x0c;
        assert_eq!(w25a44_find_state_c_slot(&t), 0x80);
        assert_eq!(w25a44_find_state_c_slot(&[]), 0);
    }

    #[test]
    fn empty_header() {
        assert_eq!(w25a44_empty_string_header(), (0, 0x8000000f, 0));
    }
}
