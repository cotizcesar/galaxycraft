#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent42.rs
// ===========================================================================
// Wave 33 agent 42: CRT float-parse helper matching the `SNAN)` tail.

/// `0x00c9544d`: matches the five characters of `SNAN)` (case-insensitive,
/// upper table at `0xe4_fedc`, lower table at `0xe4_fee4`) starting with the
/// current character `*cur`.  After each matching character the next one is
/// fetched from `next` and stored back into `*cur`.  Returns 1 when all five
/// matched, else 0 (the mismatching character stays in `*cur`).
pub fn w33a42_match_snan_tail(cur: &mut u8, mut next: impl FnMut() -> u8) -> u32 {
    const UPPER: &[u8; 5] = b"SNAN)";
    const LOWER: &[u8; 5] = b"snan)";
    let mut i = 0;
    while *cur == UPPER[i] || *cur == LOWER[i] {
        *cur = next();
        i += 1;
        if i == 5 {
            return 1;
        }
    }
    0
}

#[cfg(test)]
mod tests_wave33_agent42 {
    use super::*;

    fn run(s: &[u8]) -> (u32, u8) {
        let mut it = s.iter().copied().chain(std::iter::repeat(0));
        let mut cur = it.next().unwrap();
        let r = w33a42_match_snan_tail(&mut cur, || it.next().unwrap());
        (r, cur)
    }

    #[test]
    fn matches() {
        assert_eq!(run(b"SnAn)x"), (1, b'x'));
    }

    #[test]
    fn mismatch() {
        assert_eq!(run(b"SNAX)"), (0, b'X'));
    }
}
