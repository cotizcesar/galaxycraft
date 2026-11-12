#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent35.rs
// ===========================================================================
// Wave 34 translation batch 35.
//
// Skipped: FUN_005358a0 (ends in a non-returning call).

/// `0x005356e0`: when the guard byte (`active`) and the byte behind the
/// first pointer (`*enabled`) are both non-zero, sets the flag at offset
/// `0x24` of the `0x14`-stride record `index` in `table` to 1.
pub fn w34a35_mark_record_flag(
    active: bool,
    enabled: bool,
    table: &mut [u8],
    index: usize,
) -> bool {
    if !active || !enabled {
        return false;
    }
    match table.get_mut(index * 0x14 + 0x24) {
        Some(b) => {
            *b = 1;
            true
        }
        None => false,
    }
}

/// `0x00535780`: if the handle is non-zero, releases it through the
/// caller-supplied release routine; returns whether it was called.
pub fn w34a35_release_if_set(handle: u32, release: &mut dyn FnMut(u32)) -> bool {
    if handle != 0 {
        release(handle);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests_wave34_agent35 {
    use super::*;

    #[test]
    fn flag() {
        let mut t = vec![0u8; 0x100];
        assert!(!w34a35_mark_record_flag(true, false, &mut t, 1));
        assert!(w34a35_mark_record_flag(true, true, &mut t, 1));
        assert_eq!(t[0x14 + 0x24], 1);
        assert!(!w34a35_mark_record_flag(true, true, &mut t, 100));
    }

    #[test]
    fn release() {
        let mut got = vec![];
        assert!(!w34a35_release_if_set(0, &mut |h| got.push(h)));
        assert!(w34a35_release_if_set(7, &mut |h| got.push(h)));
        assert_eq!(got, vec![7]);
    }
}
