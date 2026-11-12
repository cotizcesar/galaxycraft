#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent142.rs
// ===========================================================================
// Wave 31 agent 142: CRT multibyte helpers (UTF-8 decoder, mbsdec-like walk).

/// Shift state of the UTF-8 decoder at `0x00cbaa59` (layout of the 8-byte
/// state: value at +0, total length at +4, bytes still needed at +6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W31a142Utf8State {
    pub value: u32,
    pub total: u8,
    pub remaining: u8,
}

/// Result of one decode step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W31a142Decode {
    /// Complete code point; `consumed` bytes were read.
    Char { value: u32, consumed: usize },
    /// Input ended mid-sequence; state saved (`consumed` bytes read).
    Incomplete { consumed: usize },
    /// Invalid sequence (the original calls an error helper).
    Invalid,
}

/// `0x00cbaa59`: UTF-8 `mbrtowc` step.  A zero `len` yields `Incomplete`
/// with zero bytes consumed; a `None` buffer behaves like one NUL byte.
pub fn w31a142_utf8_decode(
    input: Option<&[u8]>,
    len: usize,
    st: &mut W31a142Utf8State,
) -> W31a142Decode {
    let (buf, len): (&[u8], usize) = match input {
        Some(b) => (b, len.min(b.len())),
        None => (&[0u8], 1),
    };
    if len == 0 {
        return W31a142Decode::Incomplete { consumed: 0 };
    }
    let (total, mut value, need, mut pos);
    if st.remaining == 0 {
        let lead = buf[0];
        pos = 1usize;
        if lead < 0x80 {
            return W31a142Decode::Char {
                value: lead as u32,
                consumed: 1,
            };
        }
        total = if lead & 0xe0 == 0xc0 {
            2u8
        } else if lead & 0xf0 == 0xe0 {
            3
        } else if lead & 0xf8 == 0xf0 {
            4
        } else {
            return W31a142Decode::Invalid;
        };
        value = ((1u32 << (7 - total as u32)) - 1) & lead as u32;
        need = total as usize;
    } else {
        total = st.total;
        value = st.value;
        pos = 0;
        if !(2..5).contains(&total) || st.remaining >= total {
            return W31a142Decode::Invalid;
        }
        need = st.remaining as usize;
    }
    let limit = len.min(need);
    while pos < limit {
        let b = buf[pos];
        pos += 1;
        if b & 0xc0 != 0x80 {
            return W31a142Decode::Invalid;
        }
        value = (value << 6) | (b & 0x3f) as u32;
    }
    if limit < need {
        st.value = value;
        st.total = total;
        st.remaining = (need - limit) as u8;
        return W31a142Decode::Incomplete { consumed: pos };
    }
    let min = [0x80u32, 0x800, 0x10000][total as usize - 2];
    if !(0xd800..=0xdfff).contains(&value) && value < 0x110000 && value >= min {
        *st = W31a142Utf8State::default();
        return W31a142Decode::Char {
            value,
            consumed: pos,
        };
    }
    W31a142Decode::Invalid
}

/// `0x00cbacce`: `_mbsdec`-style step back.  `None` for null/empty ranges
/// (`start >= cur`).  `multibyte` mirrors the codepage's non-zero mb flag,
/// `is_trail(b)` tests bit 4 of the per-byte ctype table.
pub fn w31a142_mbsdec(
    buf: &[u8],
    start: usize,
    cur: usize,
    multibyte: bool,
    is_trail: impl Fn(u8) -> bool,
) -> Option<usize> {
    if start >= cur || cur > buf.len() {
        return None;
    }
    let mut p = cur as isize - 1;
    if multibyte {
        loop {
            p -= 1;
            if p < start as isize {
                break;
            }
            if !is_trail(buf[p as usize]) {
                break;
            }
        }
        p = cur as isize - 1 - ((cur as isize - p) & 1);
    }
    Some(p as usize)
}

#[cfg(test)]
mod tests_wave31_agent142 {
    use super::*;

    #[test]
    fn utf8_decode() {
        let mut st = W31a142Utf8State::default();
        assert_eq!(
            w31a142_utf8_decode(Some(b"A"), 1, &mut st),
            W31a142Decode::Char {
                value: 65,
                consumed: 1
            }
        );
        assert_eq!(
            w31a142_utf8_decode(Some("é".as_bytes()), 2, &mut st),
            W31a142Decode::Char {
                value: 0xe9,
                consumed: 2
            }
        );
        let e = "€".as_bytes();
        assert_eq!(
            w31a142_utf8_decode(Some(&e[..2]), 2, &mut st),
            W31a142Decode::Incomplete { consumed: 2 }
        );
        assert_eq!(st.remaining, 1);
        assert_eq!(
            w31a142_utf8_decode(Some(&e[2..]), 1, &mut st),
            W31a142Decode::Char {
                value: 0x20ac,
                consumed: 1
            }
        );
        assert_eq!(
            w31a142_utf8_decode(Some(&[0xc0, 0x80]), 2, &mut st),
            W31a142Decode::Invalid
        );
        assert_eq!(
            w31a142_utf8_decode(Some(&[0xed, 0xa0, 0x80]), 3, &mut st),
            W31a142Decode::Invalid
        );
        assert_eq!(
            w31a142_utf8_decode(Some(&[0xff]), 1, &mut st),
            W31a142Decode::Invalid
        );
        assert_eq!(
            w31a142_utf8_decode(None, 0, &mut st),
            W31a142Decode::Char {
                value: 0,
                consumed: 1
            }
        );
    }

    #[test]
    fn mbsdec() {
        let b = [b'a', 0x81, 0x40, b'b'];
        let tr = |c: u8| (0x40..0x7f).contains(&c);
        assert_eq!(w31a142_mbsdec(&b, 0, 0, true, tr), None);
        assert_eq!(w31a142_mbsdec(&b, 0, 4, false, tr), Some(3));
        assert_eq!(w31a142_mbsdec(&b, 0, 3, true, tr), Some(2));
    }
}
