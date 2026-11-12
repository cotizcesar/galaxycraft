#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent56.rs
// ===========================================================================
// Wave 25 agent 56 (queue, module runtime.rs).

/// `0x008df3b0`: maps the status returned by the lower-level call to the
/// error code passed to the reporter; `None` means "no report" (status -1).
pub fn w25a56_map_status_to_error(status: i32) -> Option<u32> {
    match status {
        -1 => None,
        6 | -3 | 3 => Some(2),
        0x3e9 => Some(0xe),
        0xffd0 => Some(9),
        _ => Some(0xc),
    }
}

#[cfg(test)]
mod tests_wave25_agent56 {
    use super::*;

    #[test]
    fn status_mapping() {
        assert_eq!(w25a56_map_status_to_error(-1), None);
        assert_eq!(w25a56_map_status_to_error(6), Some(2));
        assert_eq!(w25a56_map_status_to_error(-3), Some(2));
        assert_eq!(w25a56_map_status_to_error(3), Some(2));
        assert_eq!(w25a56_map_status_to_error(0x3e9), Some(0xe));
        assert_eq!(w25a56_map_status_to_error(0xffd0), Some(9));
        assert_eq!(w25a56_map_status_to_error(0), Some(0xc));
        assert_eq!(w25a56_map_status_to_error(7), Some(0xc));
    }
}
