#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent122.rs
// ===========================================================================
// Wave 32 agent 122: zlib deflate bit-buffer flush helpers.

/// Minimal model of the deflate state fields touched by the bit-buffer
/// helpers: output buffer (`+8`), output position (`+0x14`), `bi_buf`
/// (`+0x16b8`, 16 bits) and `bi_valid` (`+0x16bc`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct W32a122BitState {
    pub pending_buf: Vec<u8>,
    pub pending: usize,
    pub bi_buf: u16,
    pub bi_valid: i32,
}

fn put(s: &mut W32a122BitState, b: u8) {
    if s.pending < s.pending_buf.len() {
        s.pending_buf[s.pending] = b;
    }
    s.pending += 1;
}

/// `0x00c76ed0`: zlib `bi_flush`.  With 16 valid bits writes both bytes and
/// clears the buffer; with 8 or more writes the low byte and shifts.
pub fn w32a122_bi_flush(s: &mut W32a122BitState) {
    if s.bi_valid == 16 {
        put(s, s.bi_buf as u8);
        put(s, (s.bi_buf >> 8) as u8);
        s.bi_buf = 0;
        s.bi_valid = 0;
        return;
    }
    if s.bi_valid > 7 {
        put(s, s.bi_buf as u8);
        s.bi_valid -= 8;
        s.bi_buf >>= 8;
    }
}

/// `0x00c76f50`: zlib `bi_windup`.  Writes remaining bits (one or two
/// bytes) and clears the bit buffer.
pub fn w32a122_bi_windup(s: &mut W32a122BitState) {
    if s.bi_valid > 8 {
        put(s, s.bi_buf as u8);
        put(s, (s.bi_buf >> 8) as u8);
    } else if s.bi_valid > 0 {
        put(s, s.bi_buf as u8);
    }
    s.bi_buf = 0;
    s.bi_valid = 0;
}

#[cfg(test)]
mod tests_wave32_agent122 {
    use super::*;

    fn st(buf: u16, valid: i32) -> W32a122BitState {
        W32a122BitState {
            pending_buf: vec![0; 8],
            pending: 0,
            bi_buf: buf,
            bi_valid: valid,
        }
    }

    #[test]
    fn flush_full() {
        let mut s = st(0x1234, 16);
        w32a122_bi_flush(&mut s);
        assert_eq!(&s.pending_buf[..2], &[0x34, 0x12]);
        assert_eq!((s.pending, s.bi_buf, s.bi_valid), (2, 0, 0));
    }

    #[test]
    fn flush_partial() {
        let mut s = st(0x1234, 10);
        w32a122_bi_flush(&mut s);
        assert_eq!((s.pending, s.bi_buf, s.bi_valid), (1, 0x12, 2));
        let mut s = st(0x1234, 5);
        w32a122_bi_flush(&mut s);
        assert_eq!(s.pending, 0);
    }

    #[test]
    fn windup() {
        let mut s = st(0x1234, 9);
        w32a122_bi_windup(&mut s);
        assert_eq!(s.pending, 2);
        let mut s = st(0x1234, 3);
        w32a122_bi_windup(&mut s);
        assert_eq!((s.pending, s.bi_buf, s.bi_valid), (1, 0, 0));
        let mut s = st(0x1234, 0);
        w32a122_bi_windup(&mut s);
        assert_eq!(s.pending, 0);
    }
}
