#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent56.rs
// ===========================================================================
// Wave 34 agent 56: wide-char path helpers.

/// `0x00ca41af`: forwards its three arguments plus `param_1` to the
/// five-argument worker with `-1` as the second argument.
pub fn w34a56_forward_with_minus_one<F: FnOnce(u32, u32, u32, u32, u32) -> R, R>(
    worker: F,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
) -> R {
    worker(a, 0xffff_ffff, b, c, d)
}

/// `0x00ca5752`: drive number of a wide path.  `"X:"` followed by a non-NUL
/// char yields 1..=26 (case-insensitive); `"X:"` alone is rejected (`None`,
/// the original raises error 2).  Otherwise the current drive from
/// `current_drive` is returned.
pub fn w34a56_drive_number(path: &[u16], current_drive: impl FnOnce() -> i32) -> Option<i32> {
    let at = |i: usize| path.get(i).copied().unwrap_or(0);
    let c = at(0);
    let is_letter = (0x41..=0x5a).contains(&c) || (0x61..=0x7a).contains(&c);
    if !is_letter || at(1) != 0x3a {
        return Some(current_drive());
    }
    if at(2) == 0 {
        return None;
    }
    let lower = if (0x41..=0x5a).contains(&c) {
        c + 0x20
    } else {
        c
    };
    Some(lower as i32 - 0x60)
}

/// `0x00ca58ed`: true when the wide char is a path separator (`\` or `/`).
pub fn w34a56_is_path_separator(c: u16) -> bool {
    c == 0x5c || c == 0x2f
}

#[cfg(test)]
mod tests_wave34_agent56 {
    use super::*;

    #[test]
    fn sep() {
        assert!(w34a56_is_path_separator(0x5c));
        assert!(w34a56_is_path_separator(0x2f));
        assert!(!w34a56_is_path_separator(0x3a));
    }

    #[test]
    fn drive() {
        let w = |s: &str| s.encode_utf16().collect::<Vec<u16>>();
        assert_eq!(w34a56_drive_number(&w("C:\\x"), || 9), Some(3));
        assert_eq!(w34a56_drive_number(&w("a:x"), || 9), Some(1));
        assert_eq!(w34a56_drive_number(&w("C:"), || 9), None);
        assert_eq!(w34a56_drive_number(&w("x"), || 9), Some(9));
    }

    #[test]
    fn fwd() {
        assert_eq!(
            w34a56_forward_with_minus_one(|a, b, c, d, e| (a, b, c, d, e), 1, 2, 3, 4),
            (1, 0xffff_ffff, 2, 3, 4)
        );
    }
}
