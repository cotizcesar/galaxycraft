#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent114.rs
// ===========================================================================
// Wave 31 translation batch 114.
//
// Only the two wrappers around opaque imported calls carry recoverable
// logic; the rest of the queue is SEH/CRT glue, non-returning bodies,
// driver (GLES) teardown or obfuscated code and was skipped.

/// `0x00c627c0`: calls the imported routine with `handle` (result ignored)
/// and always reports success.
pub fn w31a114_call_then_true<F: FnOnce(u32)>(handle: u32, import: F) -> bool {
    import(handle);
    true
}

/// `0x00c627e0`: calls the imported routine with `handle` and reports
/// whether its byte result equals exactly 1.
pub fn w31a114_import_result_is_one<F: FnOnce(u32) -> u8>(handle: u32, import: F) -> bool {
    import(handle) == 1
}

#[cfg(test)]
mod tests_wave31_agent114 {
    use super::*;

    #[test]
    fn wrappers() {
        let mut seen = 0;
        assert!(w31a114_call_then_true(7, |h| seen = h));
        assert_eq!(seen, 7);
        assert!(w31a114_import_result_is_one(1, |_| 1));
        assert!(!w31a114_import_result_is_one(1, |_| 2));
        assert!(!w31a114_import_result_is_one(1, |_| 0));
    }
}
