#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent62.rs
// ===========================================================================
// Wave 33 translation batch 62.
//
// Skipped: FUN_00cb115d (locale/alloc glue), FUN_00cb124d (stream table
// allocator with atomics and locks).

/// `0x00cb1311`: CRT stream buffer release.  `s` is the four-word stream
/// record; when flag bits 13 and 6 are both set, the buffer pointer (word 1)
/// is passed to `free`, flags lose bits 6 and 8 (mask `0xfffffebf`) and words
/// 0, 1 and 2 are zeroed.  Returns true when the release happened.
pub fn w33a62_release_stream_buffer(s: &mut [u32; 4], mut free: impl FnMut(u32)) -> bool {
    if (s[3] >> 13) & 1 != 0 && (s[3] >> 6) & 1 != 0 {
        free(s[1]);
        s[3] &= 0xffff_febf;
        s[1] = 0;
        s[0] = 0;
        s[2] = 0;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests_wave33_agent62 {
    use super::*;

    #[test]
    fn releases_when_flags_set() {
        let mut s = [1, 0x1000, 3, 0x2140 | 0x1];
        let mut freed = vec![];
        assert!(w33a62_release_stream_buffer(&mut s, |p| freed.push(p)));
        assert_eq!(freed, vec![0x1000]);
        assert_eq!(s, [0, 0, 0, 0x2001]);
    }

    #[test]
    fn noop_without_flags() {
        let mut s = [1, 2, 3, 0x2000];
        assert!(!w33a62_release_stream_buffer(&mut s, |_| panic!()));
        assert_eq!(s, [1, 2, 3, 0x2000]);
    }
}
