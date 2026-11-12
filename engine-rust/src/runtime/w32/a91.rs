#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent91.rs
// ===========================================================================
// Wave 32 agent 91 translations.

/// `0x00a8dc70`: reads the (width, height) pair at offsets 8 and 12 of the
/// record returned by the lookup `FUN_00a98320(1, param_2)`.
pub fn w32a91_record_size(record: &[u32; 4]) -> (u32, u32) {
    (record[2], record[3])
}

/// `0x00a93870`: true when either of two lookups (tables `DAT_0135e308`,
/// then `DAT_0135e30c`) reports a non-zero result for `key`.
pub fn w32a91_either_contains(
    key: u32,
    first: impl Fn(u32) -> u32,
    second: impl Fn(u32) -> u32,
) -> bool {
    first(key) != 0 || second(key) != 0
}

#[cfg(test)]
mod tests_wave32_agent91 {
    use super::*;

    #[test]
    fn size_pair() {
        assert_eq!(w32a91_record_size(&[0, 0, 640, 480]), (640, 480));
    }

    #[test]
    fn either() {
        assert!(w32a91_either_contains(1, |_| 0, |k| k));
        assert!(!w32a91_either_contains(1, |_| 0, |_| 0));
        assert!(w32a91_either_contains(1, |_| 5, |_| 0));
    }
}
