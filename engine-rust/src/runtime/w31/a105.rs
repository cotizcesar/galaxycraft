#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent105.rs
// ===========================================================================
// Wave 31 agent 105: small container capacity helper.

/// `0x00c3f080`: capacity-grow policy of a packed size/capacity word.
/// The low 31 bits of `word` hold the capacity; when `wanted` exceeds it the
/// new capacity is `cap + (word >> 1 & 0x3fffffff)` (1.5x growth), raised to
/// `wanted` if still too small.  Returns the updated word (top flag bit kept)
/// or `None` when no growth is needed.  The reallocation itself is not modelled.
pub fn w31a105_grow_capacity_word(word: u32, wanted: u32) -> Option<u32> {
    if (word & 0x7fff_ffff) >= wanted {
        return None;
    }
    let mut new_cap = ((word >> 1) & 0x3fff_ffff).wrapping_add(word & 0x7fff_ffff);
    if new_cap < wanted {
        new_cap = wanted;
    }
    Some(((word ^ new_cap) & 0x7fff_ffff) ^ word)
}

#[cfg(test)]
mod tests_wave31_agent105 {
    use super::*;

    #[test]
    fn grow_policy() {
        assert_eq!(w31a105_grow_capacity_word(10, 5), None);
        assert_eq!(w31a105_grow_capacity_word(10, 11), Some(15));
        assert_eq!(w31a105_grow_capacity_word(10, 100), Some(100));
        assert_eq!(
            w31a105_grow_capacity_word(0x8000_0010, 17),
            Some(0x8000_0018)
        );
    }
}
