#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent50.rs
// ===========================================================================
// Wave 33 agent 50: CRT stream helpers.

/// Minimal model of the CRT `FILE` write cursor: `ptr` is the write
/// position in `buf`, `cnt` the remaining count (signed).
pub struct W33a50Stream {
    pub buf: Vec<u8>,
    pub ptr: usize,
    pub cnt: i32,
}

/// `0x00ca486a`: putc-style store.  Decrements the stream counter; when it
/// goes negative, defers to `overflow` (the flush-and-store callee);
/// otherwise stores the low byte at the cursor, advances it, and returns the
/// byte value.
pub fn w33a50_putc(
    ch: u32,
    s: &mut W33a50Stream,
    overflow: impl FnOnce(u32, &mut W33a50Stream) -> u32,
) -> u32 {
    s.cnt -= 1;
    if s.cnt < 0 {
        return overflow(ch, s);
    }
    if s.ptr >= s.buf.len() {
        s.buf.resize(s.ptr + 1, 0);
    }
    s.buf[s.ptr] = ch as u8;
    s.ptr += 1;
    ch & 0xff
}

#[cfg(test)]
mod tests_wave33_agent50 {
    use super::*;

    #[test]
    fn stores_and_advances() {
        let mut s = W33a50Stream {
            buf: vec![0; 4],
            ptr: 1,
            cnt: 2,
        };
        assert_eq!(w33a50_putc(0x141, &mut s, |_, _| 9), 0x41);
        assert_eq!(s.buf[1], 0x41);
        assert_eq!((s.ptr, s.cnt), (2, 1));
    }

    #[test]
    fn overflow_defers() {
        let mut s = W33a50Stream {
            buf: vec![0; 4],
            ptr: 0,
            cnt: 0,
        };
        assert_eq!(w33a50_putc(7, &mut s, |c, _| c + 100), 107);
        assert_eq!(s.ptr, 0);
    }
}
