#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent10.rs
// ===========================================================================
// Wave 30 translation batch 10.
//
// Window: queue: 20 addresses, module `runtime.rs`.
// Implemented: 0x00b1b8a0 (string erase-range helper).

/// 0x00b1b8a0: erase up to `count` bytes starting at `pos` from a
/// NUL-terminated byte string, keeping the terminator semantics.
/// `data` holds the characters without the terminator.  When `pos` is past
/// the end, the clamp mirrors the unsigned wrap of the original (no-op).
pub fn w30a10_string_erase(data: &mut Vec<u8>, pos: usize, count: usize) {
    let len = data.len();
    let avail = len.wrapping_sub(pos);
    let n = count.min(avail);
    if pos > len || n == 0 {
        return;
    }
    data.drain(pos..pos + n);
}

#[cfg(test)]
mod tests_wave30_agent10 {
    use super::*;

    #[test]
    fn erase_middle() {
        let mut v = b"abcdef".to_vec();
        w30a10_string_erase(&mut v, 1, 2);
        assert_eq!(v, b"adef");
    }

    #[test]
    fn erase_clamps_count() {
        let mut v = b"abcdef".to_vec();
        w30a10_string_erase(&mut v, 4, 99);
        assert_eq!(v, b"abcd");
    }

    #[test]
    fn erase_noop() {
        let mut v = b"abc".to_vec();
        w30a10_string_erase(&mut v, 1, 0);
        w30a10_string_erase(&mut v, 9, 1);
        assert_eq!(v, b"abc");
    }
}
