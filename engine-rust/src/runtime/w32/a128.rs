#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent128.rs
// ===========================================================================
// Wave 32 translation batch 128: CRT OS-error mapping helpers.
// Skipped: FUN_00c93b3e (opaque callees, out-parameter glue).

/// `0x00c9479c`: maps an OS error code to an errno value.  `table` holds the
/// 0x2d `(os_error, errno)` pairs; first match wins.  Otherwise codes
/// 0x13..0x24 give 0xd, and the rest give 0x16 for 0xbc..=0xca, else 8.
pub fn w32a128_oserr_to_errno(code: u32, table: &[(u32, i32)]) -> i32 {
    for &(os, errno) in table.iter().take(0x2d) {
        if os == code {
            return errno;
        }
    }
    if code.wrapping_sub(0x13) < 0x12 {
        return 0xd;
    }
    if code.wrapping_sub(0xbc) > 0xe {
        16 + 6
    } else {
        8
    }
}

/// `0x00c94802`: returns offset of the per-thread field (`base + 0x14`) when a
/// thread-data base exists, else the static default address.
pub fn w32a128_thread_field_or_default(base: Option<usize>, default: usize) -> usize {
    match base {
        Some(b) => b + 0x14,
        None => default,
    }
}

#[cfg(test)]
mod tests_wave32_agent128 {
    use super::*;

    #[test]
    fn errno_map() {
        let t = [(2u32, 2i32), (5, 13)];
        assert_eq!(w32a128_oserr_to_errno(5, &t), 13);
        assert_eq!(w32a128_oserr_to_errno(0x13, &t), 0xd);
        assert_eq!(w32a128_oserr_to_errno(0x24, &t), 0xd);
        assert_eq!(w32a128_oserr_to_errno(0xbc, &t), 8);
        assert_eq!(w32a128_oserr_to_errno(0xca, &t), 8);
        assert_eq!(w32a128_oserr_to_errno(0xcb, &t), 22);
        assert_eq!(w32a128_oserr_to_errno(1, &t), 22);
    }

    #[test]
    fn field() {
        assert_eq!(w32a128_thread_field_or_default(Some(0x100), 7), 0x114);
        assert_eq!(w32a128_thread_field_or_default(None, 7), 7);
    }
}
