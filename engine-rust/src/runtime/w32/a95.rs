#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent95.rs
// ===========================================================================
// Wave 32 agent 95 translations.

/// `0x00ac8850`: file-stream seek wrapper.  Returns `(low, high)` of the
/// resulting position: `(tell, 0)` on success, `(0xffffffff, 0xffffffff)`
/// on failure.  `is_open` models the virtual call at vtable +0x10 (-1 means
/// failure) and `has_handle` the handle field at +0x1c being non-null.
/// `origin` must be 0, 1 or 2; `seek` returns 0 on success.
pub fn w32a95_seek(
    has_handle: bool,
    stream_state: i32,
    offset: u32,
    origin: i32,
    seek: impl FnOnce(u32, u32) -> i32,
    tell: impl FnOnce() -> u32,
) -> (u32, u32) {
    const FAIL: (u32, u32) = (0xffff_ffff, 0xffff_ffff);
    if !has_handle || stream_state == -1 {
        return FAIL;
    }
    if !(0..=2).contains(&origin) {
        return FAIL;
    }
    if seek(offset, origin as u32) == 0 {
        (tell(), 0)
    } else {
        FAIL
    }
}

#[cfg(test)]
mod tests_wave32_agent95 {
    use super::*;

    #[test]
    fn seek_ok_and_fail() {
        assert_eq!(w32a95_seek(true, 0, 5, 1, |_, _| 0, || 9), (9, 0));
        assert_eq!(w32a95_seek(true, 0, 5, 3, |_, _| 0, || 9).0, u32::MAX);
        assert_eq!(w32a95_seek(true, -1, 5, 0, |_, _| 0, || 9).1, u32::MAX);
        assert_eq!(w32a95_seek(false, 0, 5, 0, |_, _| 0, || 9).0, u32::MAX);
        assert_eq!(w32a95_seek(true, 0, 5, 2, |_, _| 1, || 9).0, u32::MAX);
    }
}
