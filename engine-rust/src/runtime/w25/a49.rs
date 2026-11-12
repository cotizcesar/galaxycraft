#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent49.rs
// ===========================================================================
// Wave 25 agent 49: record-array sweeps and tagged-pointer teardown loops.

/// `0x008a2e10`: scans records of stride `0x44` bytes (17 words) and returns
/// the indices whose word at `+0x3c` is not 1 and whose word at `+0x40` is
/// below 3 (signed); the original calls a per-record helper for each.
pub fn w25a49_pending_records(words: &[i32]) -> Vec<usize> {
    words
        .chunks_exact(17)
        .enumerate()
        .filter(|(_, r)| r[15] != 1 && r[16] < 3)
        .map(|(i, _)| i)
        .collect()
}

/// `0x008a5e00`: walks `words` in 5-word elements; for each element whose
/// first word is a non-null (tag bit masked) pointer, calls `release(index,
/// heap_owned)` where `heap_owned` is the tag bit, then resets the word to 1.
pub fn w25a49_release_tagged_range(words: &mut [u32], mut release: impl FnMut(usize, bool)) {
    for (i, e) in words.chunks_exact_mut(5).enumerate() {
        let tagged = e[0] & 1 != 0;
        let ptr = if tagged { e[0] & !1 } else { e[0] };
        if ptr != 0 {
            release(i, tagged);
            e[0] = 1;
        }
    }
}

/// `0x008a3110`: releases the tagged-pointer slots at byte offsets 4 and
/// `0x18` of an object (word indices 1 and 6), same rule as the range walker.
pub fn w25a49_release_two_slots(obj: &mut [u32], mut release: impl FnMut(usize, bool)) {
    for slot in [1usize, 6] {
        let w = obj[slot];
        let tagged = w & 1 != 0;
        let ptr = if tagged { w & !1 } else { 4 * slot as u32 };
        if ptr != 0 {
            release(slot, tagged);
            obj[slot] = 1;
        }
    }
}

/// `0x008a6420`: checks the signed words at byte offsets `0x60`, `0x44`,
/// `0x28`, `0xc` (in that order) and returns the offsets whose value is
/// non-negative (the buffers that need freeing).
pub fn w25a49_owned_buffers(obj: &[i32]) -> Vec<usize> {
    [0x60usize, 0x44, 0x28, 0xc]
        .into_iter()
        .filter(|&o| obj[o / 4] >= 0)
        .collect()
}

#[cfg(test)]
mod tests_wave25_agent49 {
    use super::*;

    #[test]
    fn pending() {
        let mut v = vec![0i32; 34];
        v[15] = 1;
        v[16] = 0;
        v[17 + 15] = 0;
        v[17 + 16] = 2;
        assert_eq!(w25a49_pending_records(&v), vec![1]);
        v[17 + 16] = 3;
        assert!(w25a49_pending_records(&v).is_empty());
    }

    #[test]
    fn range_release() {
        let mut v = vec![0u32; 10];
        v[0] = 0x1001;
        v[5] = 0x2000;
        let mut got = vec![];
        w25a49_release_tagged_range(&mut v, |i, h| got.push((i, h)));
        assert_eq!(got, vec![(0, true), (1, false)]);
        assert_eq!((v[0], v[5]), (1, 1));
        w25a49_release_tagged_range(&mut v, |_, _| panic!());
    }

    #[test]
    fn two_slots() {
        let mut v = vec![0u32; 7];
        v[1] = 1;
        v[6] = 0x3001;
        let mut got = vec![];
        w25a49_release_two_slots(&mut v, |i, h| got.push((i, h)));
        assert_eq!(got, vec![(6, true)]);
    }

    #[test]
    fn owned() {
        let mut v = vec![-1i32; 25];
        v[0x44 / 4] = 0;
        v[3] = 5;
        assert_eq!(w25a49_owned_buffers(&v), vec![0x44, 0xc]);
    }
}
