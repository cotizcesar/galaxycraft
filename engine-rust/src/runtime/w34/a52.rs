#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent52.rs
// ===========================================================================
// Wave 34 translation batch 52.

/// `0x00c82530`: basename of a wide path.  Finds the last `\` (wcsrchr
/// semantics, stopping at the NUL) and returns the index just after it, or
/// 0 when there is no separator.
pub fn w34a52_path_basename_start(path: &[u16]) -> usize {
    let end = path.iter().position(|&c| c == 0).unwrap_or(path.len());
    match path[..end].iter().rposition(|&c| c == 0x5c) {
        Some(i) => i + 1,
        None => 0,
    }
}

#[cfg(test)]
mod tests_wave34_agent52 {
    use super::*;

    #[test]
    fn basename() {
        let a: Vec<u16> = "a\\b\\c.txt\0".encode_utf16().collect();
        assert_eq!(w34a52_path_basename_start(&a), 4);
        let b: Vec<u16> = "c.txt\0".encode_utf16().collect();
        assert_eq!(w34a52_path_basename_start(&b), 0);
    }
}
