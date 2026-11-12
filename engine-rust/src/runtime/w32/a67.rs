#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent67.rs
// ===========================================================================
// Wave 32 agent 67.

/// `0x0095b830`: exact-key lookup in a sorted map (std::map lower_bound
/// then equality check).  Returns the index of the entry whose key equals
/// `wanted`, or `None` (the original returns a null value pointer).
pub fn w32a67_map_find_index(keys_sorted: &[u32], wanted: u32) -> Option<usize> {
    let i = keys_sorted.partition_point(|&k| k < wanted);
    if i < keys_sorted.len() && keys_sorted[i] <= wanted {
        Some(i)
    } else {
        None
    }
}

#[cfg(test)]
mod tests_wave32_agent67 {
    use super::*;

    #[test]
    fn finds_exact_key() {
        let k = [1, 5, 9];
        assert_eq!(w32a67_map_find_index(&k, 5), Some(1));
        assert_eq!(w32a67_map_find_index(&k, 6), None);
        assert_eq!(w32a67_map_find_index(&k, 10), None);
        assert_eq!(w32a67_map_find_index(&[], 1), None);
    }
}
