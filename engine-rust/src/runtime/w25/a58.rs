#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent58.rs
// ===========================================================================
/// `0x008f29d0`: seeds a 624-word Mersenne-Twister state.  A zero seed is
/// replaced by 1; word `i` is `1812433253 * (prev ^ prev >> 30) + i`; the
/// returned index is `0x270` (state exhausted, next draw regenerates).
pub fn w25a58_mt_seed(seed: u32) -> ([u32; 0x270], usize) {
    let mut mt = [0u32; 0x270];
    mt[0] = if seed == 0 { 1 } else { seed };
    for i in 1..0x270usize {
        let prev = mt[i - 1];
        mt[i] = (prev ^ (prev >> 30))
            .wrapping_mul(0x6c07_8965)
            .wrapping_add(i as u32);
    }
    (mt, 0x270)
}

#[cfg(test)]
mod tests_wave25_agent58 {
    use super::*;

    #[test]
    fn mt_seed_matches_reference() {
        let (mt, idx) = w25a58_mt_seed(5489);
        assert_eq!(idx, 0x270);
        assert_eq!(mt[0], 5489);
        assert_eq!(mt[1], 1301868182);
        assert_eq!(w25a58_mt_seed(0).0[0], 1);
    }
}
