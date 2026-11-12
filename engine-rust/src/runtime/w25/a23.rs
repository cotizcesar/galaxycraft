#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent23.rs
// ===========================================================================
// Wave 25 agent 23: string-id selectors recovered from runtime glue.

/// `0x0080e800`: picks the string-table id for a status record.  `flag_13e`
/// is the byte at +0x13e, `state_180` the int at +0x180 and `field_164` the
/// int at +0x164.  Returns 0x1f when the byte is zero, 0x1e when the state
/// is 2 or 4 or `field_164` is nonzero, otherwise 0x28.
pub fn w25a23_status_string_id(flag_13e: u8, state_180: i32, field_164: i32) -> u32 {
    if flag_13e == 0 {
        0x1f
    } else if state_180 == 2 || state_180 == 4 || field_164 != 0 {
        0x1e
    } else {
        0x28
    }
}

/// `0x00810b90`: picks the string-table id from three flag bytes at
/// +0x111, +0x134 and +0x112.  A nonzero +0x111 gives 0x1f; every other
/// path (the +0x134/+0x112 tests included) yields 0x1e.
pub fn w25a23_flag_string_id(flag_111: u8, _flag_134: u8, _flag_112: u8) -> u32 {
    if flag_111 != 0 { 0x1f } else { 0x1e }
}

#[cfg(test)]
mod tests_wave25_agent23 {
    use super::*;

    #[test]
    fn status_ids() {
        assert_eq!(w25a23_status_string_id(0, 2, 1), 0x1f);
        assert_eq!(w25a23_status_string_id(1, 2, 0), 0x1e);
        assert_eq!(w25a23_status_string_id(1, 4, 0), 0x1e);
        assert_eq!(w25a23_status_string_id(1, 0, 5), 0x1e);
        assert_eq!(w25a23_status_string_id(1, 0, 0), 0x28);
    }

    #[test]
    fn flag_ids() {
        assert_eq!(w25a23_flag_string_id(1, 0, 0), 0x1f);
        assert_eq!(w25a23_flag_string_id(0, 1, 1), 0x1e);
        assert_eq!(w25a23_flag_string_id(0, 0, 0), 0x1e);
    }
}
