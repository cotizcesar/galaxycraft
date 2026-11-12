#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent37.rs
// ===========================================================================
// Wave 25 agent 37: vector growth helpers (MSVC-style containers).

/// Outcome of [`w25a37_vec_push`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W25a37Push {
    /// Value stored at the old end; size incremented.
    Stored,
    /// Buffer full: caller must reallocate to the packed capacity word given.
    Grow { new_cap_word: u32 },
}

/// `0x0086d720`: push_back on a vector whose capacity word packs a flag in
/// bit 31 and the capacity in the low 31 bits.  When size == capacity the
/// capacity grows by max(1, capacity / 2) (flag preserved) and the original
/// then reallocates; otherwise the value is stored at `data[size]`.
pub fn w25a37_vec_push(data: &mut [u32], size: &mut u32, cap_word: u32, value: u32) -> W25a37Push {
    let cap = cap_word & 0x7fff_ffff;
    if *size == cap && cap < cap.wrapping_add(1) {
        let half = (cap_word >> 1) & 0x3fff_ffff;
        let step = if cap.wrapping_add(1) <= cap.wrapping_add(half) {
            half
        } else {
            1
        };
        let new_cap_word = (cap.wrapping_add(step) ^ cap_word) & 0x7fff_ffff ^ cap_word;
        return W25a37Push::Grow { new_cap_word };
    }
    data[*size as usize] = value;
    *size += 1;
    W25a37Push::Stored
}

/// `0x0086d200`: capacity search for a vector with `size` elements; starts at
/// 8 (or `size` when non-zero) and doubles until it is at least `size + 1`.
pub fn w25a37_pow2_capacity(size: u32) -> u32 {
    let mut cap = if size != 0 { size } else { 8 };
    while cap < size.wrapping_add(1) {
        cap = cap.wrapping_mul(2);
    }
    cap
}

#[cfg(test)]
mod tests_wave25_agent37 {
    use super::*;

    #[test]
    fn push_stores_or_grows() {
        let mut d = [0u32; 4];
        let mut n = 0;
        assert_eq!(w25a37_vec_push(&mut d, &mut n, 4, 9), W25a37Push::Stored);
        assert_eq!((d[0], n), (9, 1));
        let mut n = 4;
        assert_eq!(
            w25a37_vec_push(&mut d, &mut n, 4, 1),
            W25a37Push::Grow { new_cap_word: 6 }
        );
        let mut n = 0;
        assert_eq!(
            w25a37_vec_push(&mut d, &mut n, 0x8000_0000, 1),
            W25a37Push::Grow {
                new_cap_word: 0x8000_0001
            }
        );
    }

    #[test]
    fn capacity_doubles() {
        assert_eq!(w25a37_pow2_capacity(0), 8);
        assert_eq!(w25a37_pow2_capacity(5), 10);
        assert_eq!(w25a37_pow2_capacity(8), 16);
    }
}
