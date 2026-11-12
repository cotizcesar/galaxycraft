#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent37.rs
// ===========================================================================
// Wave 33 translation batch 37.
//
// Skipped (vftable/exception glue): FUN_00c83f18, FUN_00c83f79.

/// `0x00c82260`: finds the extension start in a UTF-16 path.  Scans from
/// the terminator backwards while the index is above 0 and the unit is
/// neither `\` nor `.`; returns that index when it is a `.`, otherwise the
/// terminator index (string length).
pub fn w33a37_find_extension(s: &[u16]) -> usize {
    let len = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    let at = |i: usize| if i < s.len() { s[i] } else { 0 };
    let mut i = len;
    while i > 0 && at(i) != 0x5c && at(i) != 0x2e {
        i -= 1;
    }
    if at(i) == 0x2e { i } else { len }
}

#[cfg(test)]
mod tests_wave33_agent37 {
    use super::*;
    fn w(s: &str) -> Vec<u16> {
        let mut v: Vec<u16> = s.encode_utf16().collect();
        v.push(0);
        v
    }
    #[test]
    fn ext() {
        assert_eq!(w33a37_find_extension(&w("a\\b.txt")), 3);
        assert_eq!(w33a37_find_extension(&w("a.b\\c")), 5);
        assert_eq!(w33a37_find_extension(&w("abc")), 3);
        assert_eq!(w33a37_find_extension(&w(".x")), 0);
    }
}
