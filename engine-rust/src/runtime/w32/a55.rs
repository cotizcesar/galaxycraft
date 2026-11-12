#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent55.rs
// ===========================================================================
// Wave 32 agent 55.

/// `0x00880220`: scans the pointer-sized entry list (begin/count at object
/// offsets 0x148/0x14c) and returns the first entry whose key, computed by
/// `key_of` (the callee FUN_00609780 modelled as a closure), equals `wanted`;
/// returns 0 when no entry matches.
pub fn w32a55_find_entry(entries: &[u32], wanted: i32, key_of: impl Fn(u32) -> i32) -> u32 {
    entries
        .iter()
        .copied()
        .find(|&e| key_of(e) == wanted)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests_wave32_agent55 {
    use super::*;

    #[test]
    fn finds_first_match() {
        let v = [10u32, 20, 30, 20];
        assert_eq!(w32a55_find_entry(&v, 2, |e| (e / 10) as i32), 20);
    }

    #[test]
    fn missing_returns_zero() {
        assert_eq!(w32a55_find_entry(&[1, 2], 9, |e| e as i32), 0);
        assert_eq!(w32a55_find_entry(&[], 0, |e| e as i32), 0);
    }
}
