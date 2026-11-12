#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent108.rs
// ===========================================================================
// Wave 32 agent 108 translations.

fn w32a108_is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | 0x0b | b'\r' | 0x0c | b'\n')
}

/// `0x00b8ccc0`: scans `buf` for `key` followed by optional whitespace and a
/// `:`; the `nth` (zero based) such key wins.  Returns the index of the first
/// non-whitespace byte after the colon (or the window end).  After a failed
/// match the scan resumes after the next NUL byte (memchr helper
/// `0xc8_a380`); when none is left the scan ends with `None`.
pub fn w32a108_find_key_value(buf: &[u8], key: &[u8], nth: i32) -> Option<usize> {
    let klen = key.len();
    if klen == 0 {
        return None;
    }
    let end = buf.len();
    let mut pos = 0usize;
    let mut count = 0i32;
    while pos < end {
        if klen <= end - pos && &buf[pos..pos + klen] == key {
            pos += klen;
            while pos < end {
                let c = buf[pos];
                if c == 0 || !w32a108_is_space(c) {
                    break;
                }
                pos += 1;
            }
            if pos >= end {
                return None;
            }
            if buf[pos] == 0 {
                pos -= 1;
            } else if buf[pos] == b':' {
                if count == nth {
                    loop {
                        pos += 1;
                        if pos >= end {
                            return Some(pos);
                        }
                        let c = buf[pos];
                        if c == 0 || !w32a108_is_space(c) {
                            return Some(pos);
                        }
                    }
                }
                count += 1;
            }
        }
        pos += buf[pos..end].iter().position(|&b| b == 0)? + 1;
    }
    None
}

/// `0x00b8f2e0`: writes the lowercase hex of `src` into `out` (two chars per
/// byte) and returns the number of chars written.
pub fn w32a108_hex_encode(src: &[u8], out: &mut [u8]) -> usize {
    const D: &[u8; 16] = b"0123456789abcdef";
    let mut n = 0;
    for &b in src {
        out[n] = D[(b >> 4) as usize];
        out[n + 1] = D[(b & 15) as usize];
        n += 2;
    }
    n
}

#[cfg(test)]
mod tests_wave32_agent108 {
    use super::*;

    #[test]
    fn hex() {
        let mut o = [0u8; 6];
        assert_eq!(w32a108_hex_encode(&[0xab, 0x01, 0xff], &mut o), 6);
        assert_eq!(&o, b"ab01ff");
    }

    #[test]
    fn key_value() {
        let b = b"name : foo\0name:  bar\0";
        assert_eq!(w32a108_find_key_value(b, b"name", 0), Some(7));
        assert_eq!(w32a108_find_key_value(b, b"name", 1), Some(18));
        assert_eq!(w32a108_find_key_value(b, b"zzz", 0), None);
    }
}
