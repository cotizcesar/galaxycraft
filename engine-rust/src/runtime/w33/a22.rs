#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent22.rs
// ===========================================================================
// Wave 33 agent 22 translations.

/// `0x009a37b0`: bounded UTF-16 copy (wcsncpy without padding).  Copies up
/// to `count` units from `src` into `dst`, stopping after the terminating
/// zero has been copied.  Returns the number of units written.
pub fn w33a22_wcsncpy_nopad(dst: &mut [u16], src: &[u16], count: usize) -> usize {
    let mut n = 0;
    while n < count && n < dst.len() {
        let c = src.get(n).copied().unwrap_or(0);
        dst[n] = c;
        n += 1;
        if c == 0 {
            break;
        }
    }
    n
}

#[cfg(test)]
mod tests_wave33_agent22 {
    use super::*;

    #[test]
    fn copies_until_nul() {
        let mut d = [9u16; 6];
        let n = w33a22_wcsncpy_nopad(&mut d, &[1, 2, 0, 7], 5);
        assert_eq!(n, 3);
        assert_eq!(d, [1, 2, 0, 9, 9, 9]);
    }

    #[test]
    fn respects_count() {
        let mut d = [9u16; 4];
        assert_eq!(w33a22_wcsncpy_nopad(&mut d, &[1, 2, 3, 4], 2), 2);
        assert_eq!(d, [1, 2, 9, 9]);
        assert_eq!(w33a22_wcsncpy_nopad(&mut d, &[1], 0), 0);
    }
}
