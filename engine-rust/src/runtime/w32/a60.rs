#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent60.rs
// ===========================================================================
// Wave 32 translation batch 60.
//
// Skipped: FUN_008f9980 (ends in a non-returning call), FUN_008ff6d0
// (SEH / lambda glue ending in a non-returning call).

/// `0x008ee0b0`: linear search of a table of 0x28-byte records for an exact
/// byte-string key.  Each entry is `(data, len)` (data pointer at +8, length
/// at +0xc).  The first `min(len, key_len)` bytes are compared with the
/// word/byte memcmp (`FUN_0055c520`); on equality the result is
/// `len - key_len`, so only equal-length entries match.  Returns the index
/// of the first match, or `entries.len()` (the end iterator) when none.
pub fn w32a60_find_record(entries: &[(&[u8], u32)], key: &[u8], key_len: u32) -> usize {
    for (i, (data, len)) in entries.iter().enumerate() {
        let n = (*len).min(key_len) as usize;
        let mut diff = 0u32;
        if data[..n] != key[..n] {
            diff = 1;
        }
        if diff == 0 {
            diff = len.wrapping_sub(key_len);
        }
        if diff == 0 {
            return i;
        }
    }
    entries.len()
}

#[cfg(test)]
mod tests_wave32_agent60 {
    use super::*;

    #[test]
    fn finds_exact_length_match() {
        let e: [(&[u8], u32); 3] = [(b"abc", 3), (b"abcd", 4), (b"xyz", 3)];
        assert_eq!(w32a60_find_record(&e, b"abcd", 4), 1);
        assert_eq!(w32a60_find_record(&e, b"xyz", 3), 2);
        assert_eq!(w32a60_find_record(&e, b"abd", 3), 3);
        assert_eq!(w32a60_find_record(&[], b"a", 1), 0);
    }
}
