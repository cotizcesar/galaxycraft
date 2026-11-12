#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent06.rs
// ===========================================================================
// Wave 26 translation batch 06: glue screen helpers.

/// `0x00916fc0`: forwards to the inner routine dropping the first argument.
pub fn w26a06_forward_drop_first<R>(
    _p1: u32,
    p2: u32,
    p3: u32,
    p4: u32,
    p5: u32,
    inner: impl FnOnce(u32, u32, u32, u32) -> R,
) -> R {
    inner(p2, p3, p4, p5)
}

/// `0x009196f0`: maps a dialog result code to the follow-up code passed to
/// the notifier; `None` means the notifier is skipped (code -1).
pub fn w26a06_result_to_code(result: i32) -> Option<u32> {
    match result {
        -1 => None,
        12 => Some(15),
        -3 | 13 => Some(2),
        0x3e9 => Some(14),
        _ => Some(12),
    }
}

/// `0x00918110`: dispatches vtable slot 0xcc (no flag) or 0xc8 (flag set).
pub fn w26a06_dispatch_by_flag<R>(
    flag: bool,
    slot_cc: impl FnOnce() -> R,
    slot_c8: impl FnOnce() -> R,
) -> R {
    if flag { slot_c8() } else { slot_cc() }
}

/// `0x00918360`: resets the screen's string state: u16 at `0x1f4` and u8 at
/// `0x1f6` cleared, 0x50 bytes at `0x1a4` zeroed.
pub fn w26a06_reset_screen_fields(obj: &mut [u8]) {
    obj[0x1f4..0x1f6].fill(0);
    obj[0x1f6] = 0;
    obj[0x1a4..0x1a4 + 0x50].fill(0);
}

/// `0x00918580`: if the u16 at `0x1f4` is nonzero, runs `release` and clears
/// the field and the 0x50-byte buffer at `0x1a4`.  Returns whether it did.
pub fn w26a06_release_buffer(obj: &mut [u8], release: impl FnOnce()) -> bool {
    if obj[0x1f4] == 0 && obj[0x1f5] == 0 {
        return false;
    }
    release();
    obj[0x1f4..0x1f6].fill(0);
    obj[0x1a4..0x1a4 + 0x50].fill(0);
    true
}

#[cfg(test)]
mod tests_wave26_agent06 {
    use super::*;

    #[test]
    fn forward_and_dispatch() {
        assert_eq!(
            w26a06_forward_drop_first(9, 1, 2, 3, 4, |a, b, c, d| a + b + c + d),
            10
        );
        assert_eq!(w26a06_dispatch_by_flag(true, || 1, || 2), 2);
        assert_eq!(w26a06_dispatch_by_flag(false, || 1, || 2), 1);
    }

    #[test]
    fn result_codes() {
        assert_eq!(w26a06_result_to_code(-1), None);
        assert_eq!(w26a06_result_to_code(12), Some(15));
        assert_eq!(w26a06_result_to_code(-3), Some(2));
        assert_eq!(w26a06_result_to_code(13), Some(2));
        assert_eq!(w26a06_result_to_code(0x3e9), Some(14));
        assert_eq!(w26a06_result_to_code(5), Some(12));
    }

    #[test]
    fn reset_and_release() {
        let mut o = vec![0xffu8; 0x200];
        w26a06_reset_screen_fields(&mut o);
        assert_eq!(o[0x1f4], 0);
        assert_eq!(o[0x1f6], 0);
        assert_eq!(o[0x1a4 + 0x4f], 0);
        assert_eq!(o[0x1a3], 0xff);
        let mut called = false;
        assert!(!w26a06_release_buffer(&mut o, || called = true));
        assert!(!called);
        o[0x1f5] = 1;
        o[0x1a4] = 7;
        assert!(w26a06_release_buffer(&mut o, || called = true));
        assert!(called && o[0x1f5] == 0 && o[0x1a4] == 0);
    }
}
