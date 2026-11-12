#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent59.rs
// ===========================================================================
// Wave 25 agent 59 (runtime glue).

/// `0x008f7e10`: maps the error code returned by the I/O probe to the
/// error class passed to the reporter.  `None` means the code is `-1`
/// and no report is made.
pub fn w25a59_map_error_class(code: i32) -> Option<u32> {
    if code < 0x1d {
        match code {
            0x1c | -3 => Some(2),
            -1 => None,
            _ => Some(0xc),
        }
    } else if code == 0x1d {
        Some(2)
    } else if code != 0x3e9 {
        Some(0xc)
    } else {
        Some(0xe)
    }
}

/// `0x008f7690`: scalar-deleting destructor flag; the object memory is
/// freed only when bit 0 of the flag is set.
pub fn w25a59_dtor_frees_0x008f7690(flags: u8) -> bool {
    flags & 1 != 0
}

/// `0x008f76d0`: scalar-deleting destructor flag; same rule as above.
pub fn w25a59_dtor_frees_0x008f76d0(flags: u8) -> bool {
    flags & 1 != 0
}

#[cfg(test)]
mod tests_wave25_agent59 {
    use super::*;

    #[test]
    fn error_class() {
        assert_eq!(w25a59_map_error_class(-1), None);
        assert_eq!(w25a59_map_error_class(-3), Some(2));
        assert_eq!(w25a59_map_error_class(0x1c), Some(2));
        assert_eq!(w25a59_map_error_class(0x1d), Some(2));
        assert_eq!(w25a59_map_error_class(0x3e9), Some(0xe));
        assert_eq!(w25a59_map_error_class(5), Some(0xc));
        assert_eq!(w25a59_map_error_class(0x3ea), Some(0xc));
    }

    #[test]
    fn dtor_flags() {
        assert!(w25a59_dtor_frees_0x008f7690(3));
        assert!(!w25a59_dtor_frees_0x008f76d0(2));
    }
}
