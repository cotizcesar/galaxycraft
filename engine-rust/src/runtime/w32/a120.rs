#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent120.rs
// ===========================================================================
// Wave 32 translation batch 120.

/// `0x00c68be0`: maps a selector 0..=3 to a code: 0 -> 0, 1 -> 0x7a,
/// 2 -> 0x57, anything else -> 1.
pub fn w32a120_selector_code(sel: i32) -> u32 {
    match sel {
        0 => 0,
        1 => 0x7a,
        2 => 0x57,
        _ => 1,
    }
}

/// `0x00c68620`: if `value` equals the code of selectors 0..=3 it runs
/// `on_match` (callee FUN_00c683a0); otherwise it runs `fallback`
/// (callee FUN_00c68c10) with the original arguments.
pub fn w32a120_dispatch<R>(
    value: i32,
    on_match: impl FnOnce() -> R,
    fallback: impl FnOnce() -> R,
) -> R {
    for sel in 0..4 {
        if value as u32 == w32a120_selector_code(sel) {
            return on_match();
        }
    }
    fallback()
}

#[cfg(test)]
mod tests_wave32_agent120 {
    use super::*;

    #[test]
    fn codes() {
        assert_eq!(w32a120_selector_code(0), 0);
        assert_eq!(w32a120_selector_code(1), 0x7a);
        assert_eq!(w32a120_selector_code(2), 0x57);
        assert_eq!(w32a120_selector_code(3), 1);
        assert_eq!(w32a120_selector_code(9), 1);
    }

    #[test]
    fn dispatch() {
        for v in [0, 1, 0x57, 0x7a] {
            assert_eq!(w32a120_dispatch(v, || 1, || 2), 1);
        }
        assert_eq!(w32a120_dispatch(5, || 1, || 2), 2);
    }
}
