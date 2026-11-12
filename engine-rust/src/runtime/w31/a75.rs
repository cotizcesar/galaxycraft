#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent75.rs
// ===========================================================================
// Wave 31 agent 75 (queue, runtime.rs glue).

/// `0x00b48860`: path-fit check.  `path` is a NUL-free byte string; returns
/// the length to copy when the path is non-empty and
/// `required(len) < capacity`, where `required` is 1 for an empty string,
/// else `len + (1 if no trailing '/' or '\\') + 0xf`.  The original always
/// returns 0 to its caller; the copy length models the (non-returning) tail.
pub fn w31a75_path_fits(capacity: u32, path: &[u8]) -> Option<usize> {
    let len = path.len();
    if len == 0 {
        return None;
    }
    let last = path[len - 1];
    let mut n = len as u32;
    if last != b'/' && last != b'\\' {
        n += 1;
    }
    let needed = n + 0xf;
    if needed < capacity { Some(len) } else { None }
}

/// `0x00b4af50`: padding needed to round `size` up to a multiple of
/// `align` (power of two); `None` when `size == u32::MAX` (error path).
pub fn w31a75_align_padding(size: u32, align: u32) -> Option<u32> {
    if size == 0xffff_ffff {
        return None;
    }
    let rounded = size.wrapping_add(align).wrapping_sub(1) & align.wrapping_neg();
    Some(rounded.saturating_sub(size))
}

#[cfg(test)]
mod tests_wave31_agent75 {
    use super::*;

    #[test]
    fn path_fits() {
        assert_eq!(w31a75_path_fits(100, b""), None);
        assert_eq!(w31a75_path_fits(100, b"abc"), Some(3));
        // "abc": 3+1+15 = 19 -> needs capacity > 19
        assert_eq!(w31a75_path_fits(19, b"abc"), None);
        assert_eq!(w31a75_path_fits(20, b"abc"), Some(3));
        // trailing slash: 4+15 = 19
        assert_eq!(w31a75_path_fits(20, b"abc/"), Some(4));
        assert_eq!(w31a75_path_fits(19, b"abc\\"), None);
    }

    #[test]
    fn align_padding() {
        assert_eq!(w31a75_align_padding(5, 4), Some(3));
        assert_eq!(w31a75_align_padding(8, 4), Some(0));
        assert_eq!(w31a75_align_padding(0xffff_ffff, 4), None);
    }
}
