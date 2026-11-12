#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent40.rs
// ===========================================================================
// Wave 33 translation batch 40.
//
// Skipped: FUN_00c936a0 and FUN_00c936bc (return value of unmodelled
// FUN_00c93722 callee chain).

/// `0x00c8af78`: wide `wcsrchr`.  Returns the index of the last u16 equal to
/// `wanted` in the NUL-terminated string `s` (searching `0` finds the
/// terminator itself), or `None`.  Both the scalar and SSE4.2 paths agree.
pub fn w33a40_wcsrchr(s: &[u16], wanted: u16) -> Option<usize> {
    let mut found = None;
    for (i, &c) in s.iter().enumerate() {
        if c == wanted {
            found = Some(i);
        }
        if c == 0 {
            return found;
        }
    }
    found
}

#[cfg(test)]
mod tests_wave33_agent40 {
    use super::*;
    #[test]
    fn rchr() {
        let s = [1u16, 2, 1, 3, 0, 1];
        assert_eq!(w33a40_wcsrchr(&s, 1), Some(2));
        assert_eq!(w33a40_wcsrchr(&s, 0), Some(4));
        assert_eq!(w33a40_wcsrchr(&s, 9), None);
    }
}
