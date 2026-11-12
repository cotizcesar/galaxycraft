#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent76.rs
// ===========================================================================
// Wave 32 translation batch 76.
//
// Skipped: FUN_009d9900 and FUN_009daca0 (D3D device-object glue over opaque
// vtable/struct layouts and unmodelled callees).

/// `0x009d8f40`: if `param_1 != param_2` or `force` is set, first tries
/// `release_cached` (only when `param_1 != 0` and its field `+0x20` is
/// non-zero; the result is returned when non-zero), then `fallback`; the
/// first non-zero result is returned, else 0.
pub fn w32a76_try_refresh(
    param_1: i32,
    param_2: i32,
    force: bool,
    field_20_of_param_1: i32,
    release_cached: impl FnOnce() -> i32,
    fallback: impl FnOnce() -> i32,
) -> i32 {
    if param_1 == param_2 && !force {
        return 0;
    }
    if param_1 != 0 && field_20_of_param_1 != 0 {
        let r = release_cached();
        if r != 0 {
            return r;
        }
    }
    fallback()
}

#[cfg(test)]
mod tests_wave32_agent76 {
    use super::*;

    #[test]
    fn refresh_paths() {
        assert_eq!(w32a76_try_refresh(1, 1, false, 1, || 5, || 6), 0);
        assert_eq!(w32a76_try_refresh(1, 1, true, 1, || 5, || 6), 5);
        assert_eq!(w32a76_try_refresh(1, 2, false, 0, || 5, || 6), 6);
        assert_eq!(w32a76_try_refresh(0, 2, false, 1, || 5, || 0), 0);
        assert_eq!(w32a76_try_refresh(1, 2, false, 1, || 0, || 7), 7);
    }
}
