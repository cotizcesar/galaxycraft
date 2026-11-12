#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent54.rs
// ===========================================================================
// Wave 34 agent 54: small MSVC iostream/locale helpers.

/// `0x00c9b27e`: counts up to `count` characters of a NUL-terminated
/// buffer starting at `pos`, skipping the trail byte of each lead byte
/// (`is_lead(byte)` models the ctype mask 0x8000 test).  Stops at NUL or
/// end of slice; returns the number of characters consumed.
pub fn w34a54_count_mb_chars(
    buf: &[u8],
    pos: usize,
    count: i32,
    is_lead: impl Fn(u8) -> bool,
) -> i32 {
    let mut p = pos;
    let mut n = 0;
    while n < count {
        let b = buf.get(p).copied().unwrap_or(0);
        if b == 0 {
            return n;
        }
        if is_lead(b) {
            p += 1;
        }
        p += 1;
        n += 1;
    }
    n
}

/// Output cursor for `w34a54_put_byte`.
pub struct W34a54Out<'a> {
    pub buf: &'a mut [u8],
    pub pos: usize,
    pub avail: usize,
    pub limit: usize,
    pub flag: u8,
}

/// `0x00c9b38f`: bounded put.  When the remaining-space counter equals the
/// limit, no write happens: `written` becomes -1 if `flag` is 0, else is
/// incremented, and `flag` is returned.  Otherwise the byte is stored,
/// counters advance and 1 is returned.
pub fn w34a54_put_byte(out: &mut W34a54Out, byte: u8, written: &mut i32) -> u8 {
    if out.avail == out.limit {
        if out.flag == 0 {
            *written = -1;
        } else {
            *written += 1;
        }
        return out.flag;
    }
    *written += 1;
    out.avail += 1;
    out.buf[out.pos] = byte;
    out.pos += 1;
    1
}

/// `0x00c9fa8d`: reads the next byte of `buf` at `*pos` (end = `buf.len()`),
/// advancing; returns 0xffffffff at end.
pub fn w34a54_next_byte(buf: &[u8], pos: &mut usize) -> u32 {
    match buf.get(*pos) {
        Some(&b) if *pos < buf.len() => {
            *pos += 1;
            b as u32
        }
        _ => 0xffff_ffff,
    }
}

#[cfg(test)]
mod tests_wave34_agent54 {
    use super::*;
    #[test]
    fn count() {
        assert_eq!(w34a54_count_mb_chars(b"ab\0", 0, 5, |_| false), 2);
        assert_eq!(w34a54_count_mb_chars(b"\x81xab", 0, 2, |b| b == 0x81), 2);
    }
    #[test]
    fn put() {
        let mut b = [0u8; 2];
        let mut o = W34a54Out {
            buf: &mut b,
            pos: 0,
            avail: 0,
            limit: 1,
            flag: 0,
        };
        let mut w = 0;
        assert_eq!(w34a54_put_byte(&mut o, 7, &mut w), 1);
        assert_eq!(w34a54_put_byte(&mut o, 8, &mut w), 0);
        assert_eq!(w, -1);
        assert_eq!(b[0], 7);
    }
    #[test]
    fn next() {
        let mut p = 0;
        assert_eq!(w34a54_next_byte(b"a", &mut p), 97);
        assert_eq!(w34a54_next_byte(b"a", &mut p), 0xffff_ffff);
    }
}
