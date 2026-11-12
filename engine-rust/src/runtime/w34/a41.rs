#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent41.rs
// ===========================================================================
// Wave 34 agent 41.

/// `0x009a3710`: widens a NUL-terminated byte string in place to UTF-16LE
/// style (each byte followed by a zero).  Returns `false` when the buffer is
/// too small (`len*2+2 > buf.len()`), mirroring the null return.
pub fn w34a41_widen_in_place(buf: &mut [u8]) -> bool {
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    if len * 2 + 2 > buf.len() {
        return false;
    }
    let mut i = len as isize;
    while i >= 0 {
        let u = i as usize;
        buf[u * 2] = buf[u];
        buf[u * 2 + 1] = 0;
        i -= 1;
    }
    true
}

#[cfg(test)]
mod tests_wave34_agent41 {
    use super::*;

    #[test]
    fn widen() {
        let mut b = *b"abc\0\0\0\0\0";
        assert!(w34a41_widen_in_place(&mut b));
        assert_eq!(&b, b"a\0b\0c\0\0\0");
        let mut s = *b"abc\0";
        assert!(!w34a41_widen_in_place(&mut s));
    }
}
