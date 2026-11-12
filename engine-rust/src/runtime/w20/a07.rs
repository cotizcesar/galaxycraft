#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave20/agent07.rs
// ===========================================================================
// Wave 20 translation batch 07: fmt v6 helpers (argument lookup, hex and
// char writers, string-view row search).  Most other rows of the assignment
// were SEH/destructor glue ending in non-returning calls and are skipped.

/// Alignment mode of a fmt v6 format spec (low nibble of the flags word,
/// sign-extended): 2 = right, 3 = center, anything else = left.
pub const W20A07_ALIGN_RIGHT: i32 = 2;
/// Center alignment code.
pub const W20A07_ALIGN_CENTER: i32 = 3;

/// Format spec subset read by `0x0055c9f0`: width (`spec[0]`), fill byte
/// (`+0x11`) and the sign-extended low nibble of the word at `+0xc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W20a07CharSpec {
    pub width: u32,
    pub fill: u8,
    pub flags_word: u32,
}

/// `0x0055c9f0`: appends one character to `out` honouring width/fill/align.
/// With no spec or width < 2 the byte is appended alone.  Otherwise `width`
/// bytes are appended: padding `width-1` split by alignment (right = all
/// before, center = `pad/2` before and the rest after, left = all after).
pub fn w20a07_write_char(out: &mut Vec<u8>, spec: Option<&W20a07CharSpec>, ch: u8) {
    let spec = match spec {
        Some(s) if s.width >= 2 => s,
        _ => {
            out.push(ch);
            return;
        }
    };
    let pad = (spec.width - 1) as usize;
    let align = ((spec.flags_word << 28) as i32) >> 28;
    let (before, after) = match align {
        W20A07_ALIGN_RIGHT => (pad, 0),
        W20A07_ALIGN_CENTER => (pad >> 1, pad - (pad >> 1)),
        _ => (0, pad),
    };
    out.extend(std::iter::repeat_n(spec.fill, before));
    out.push(ch);
    out.extend(std::iter::repeat_n(spec.fill, after));
}

/// Number of hex digits of `v` (at least 1), the `>> 4` loop of `0x0055cb00`.
pub fn w20a07_hex_digit_count(mut v: u32) -> usize {
    let mut n = 0;
    loop {
        v >>= 4;
        n += 1;
        if v == 0 {
            return n;
        }
    }
}

/// `0x0055cb00`: no-spec path; appends `"0x"` followed by lowercase hex
/// digits of `v` (`2 + digits` bytes).  The spec path calls the padded
/// writer and is not modelled.
pub fn w20a07_write_hex_prefixed(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(b"0x");
    let n = w20a07_hex_digit_count(v);
    for i in (0..n).rev() {
        out.push(b"0123456789abcdef"[((v >> (4 * i)) & 0xf) as usize]);
    }
}

/// One named-argument entry of `0x0055c300`: stride 6 words
/// `[name_ptr, name_len, value0, value1, value2, value3]`.
pub type W20a07NamedArg = [u32; 6];

/// `0x0055c300`: finds the named argument whose length equals `name_len`
/// and whose first `min(len, name_len)` bytes compare equal (`cmp(entry_name,
/// name_ptr, n) == 0`, the `0x55_c520` compare).  Returns the 4-word value;
/// `Err("argument not found")` when absent or when the value's third word is 0.
pub fn w20a07_find_named_arg(
    entries: &[W20a07NamedArg],
    name_ptr: u32,
    name_len: u32,
    mut cmp: impl FnMut(u32, u32, u32) -> i32,
) -> Result<[u32; 4], &'static str> {
    for e in entries {
        let n = e[1].min(name_len);
        if cmp(e[0], name_ptr, n) == 0 && e[1] == name_len {
            let v = [e[2], e[3], e[4], e[5]];
            if v[2] != 0 {
                return Ok(v);
            }
            return Err("argument not found");
        }
    }
    Err("argument not found")
}

/// String-view row of `0x0055cbe0` (8 words = 32 bytes).  Flag `0x1000` in
/// the u16 at `+0xe` means inline: length `0xd - (i8)byte[0xd]`, data is the
/// row itself; else length is word 0 and the data pointer is word 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W20a07Row(pub [u8; 32]);

impl W20a07Row {
    fn inline(&self) -> bool {
        u16::from_le_bytes([self.0[0xe], self.0[0xf]]) & 0x1000 != 0
    }
    fn word(&self, i: usize) -> u32 {
        u32::from_le_bytes(self.0[i * 4..i * 4 + 4].try_into().unwrap())
    }
    /// Length per the inline/heap rule.
    pub fn len(&self) -> u32 {
        if self.inline() {
            0xd_i32.wrapping_sub(self.0[0xd] as i8 as i32) as u32
        } else {
            self.word(0)
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// `0x0055cbe0`: returns the index of the first row whose string equals
/// `wanted` (equal length, then pointer identity or byte equality), or
/// `rows.len()` when none (the end sentinel).  `heap(ptr, len)` resolves a
/// heap data pointer to bytes.
pub fn w20a07_find_row(
    rows: &[W20a07Row],
    wanted: &W20a07Row,
    heap: impl Fn(u32, u32) -> Vec<u8>,
) -> usize {
    let bytes = |r: &W20a07Row| -> (Vec<u8>, Option<u32>) {
        let n = r.len();
        if r.inline() {
            (r.0[..(n as usize).min(32)].to_vec(), None)
        } else {
            (heap(r.word(2), n), Some(r.word(2)))
        }
    };
    let (wb, wp) = bytes(wanted);
    for (i, r) in rows.iter().enumerate() {
        if r.len() != wanted.len() {
            continue;
        }
        let (rb, rp) = bytes(r);
        let ident = match (wp, rp) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        };
        if ident || rb == wb {
            return i;
        }
    }
    rows.len()
}

#[cfg(test)]
mod tests_wave20_agent07 {
    use super::*;

    #[test]
    fn char_padding() {
        let mut o = vec![];
        w20a07_write_char(&mut o, None, b'a');
        assert_eq!(o, b"a");
        let s = |f| W20a07CharSpec {
            width: 4,
            fill: b'.',
            flags_word: f,
        };
        let mut o = vec![];
        w20a07_write_char(&mut o, Some(&s(2)), b'x');
        assert_eq!(o, b"...x");
        let mut o = vec![];
        w20a07_write_char(&mut o, Some(&s(3)), b'x');
        assert_eq!(o, b".x..");
        let mut o = vec![];
        w20a07_write_char(&mut o, Some(&s(1)), b'x');
        assert_eq!(o, b"x...");
        let mut o = vec![];
        let one = W20a07CharSpec {
            width: 1,
            fill: b'.',
            flags_word: 2,
        };
        w20a07_write_char(&mut o, Some(&one), b'x');
        assert_eq!(o, b"x");
    }

    #[test]
    fn hex() {
        assert_eq!(w20a07_hex_digit_count(0), 1);
        assert_eq!(w20a07_hex_digit_count(0xf), 1);
        assert_eq!(w20a07_hex_digit_count(0x10), 2);
        assert_eq!(w20a07_hex_digit_count(u32::MAX), 8);
        let mut o = vec![];
        w20a07_write_hex_prefixed(&mut o, 0xbeef0);
        assert_eq!(o, b"0xbeef0");
        let mut o = vec![];
        w20a07_write_hex_prefixed(&mut o, 0);
        assert_eq!(o, b"0x0");
    }

    #[test]
    fn named_arg() {
        let names = [b"ab".to_vec(), b"abc".to_vec()];
        let entries = [[0, 2, 1, 2, 3, 4], [1, 3, 5, 6, 7, 8]];
        let cmp = |a: u32, b: u32, n: u32| {
            let (x, y) = (&names[a as usize], &names[b as usize]);
            (x[..n as usize] != y[..n as usize]) as i32
        };
        assert_eq!(w20a07_find_named_arg(&entries, 1, 3, cmp), Ok([5, 6, 7, 8]));
        assert_eq!(w20a07_find_named_arg(&entries, 0, 2, cmp), Ok([1, 2, 3, 4]));
        let zero = [[0, 2, 1, 2, 0, 4]];
        assert!(w20a07_find_named_arg(&zero, 0, 2, cmp).is_err());
        assert!(w20a07_find_named_arg(&[], 0, 2, cmp).is_err());
    }

    fn inline_row(s: &[u8]) -> W20a07Row {
        let mut r = [0u8; 32];
        r[..s.len()].copy_from_slice(s);
        r[0xd] = (0xd - s.len()) as u8;
        r[0xf] = 0x10;
        W20a07Row(r)
    }
    fn heap_row(len: u32, ptr: u32) -> W20a07Row {
        let mut r = [0u8; 32];
        r[0..4].copy_from_slice(&len.to_le_bytes());
        r[8..12].copy_from_slice(&ptr.to_le_bytes());
        W20a07Row(r)
    }

    #[test]
    fn find_row() {
        let rows = [inline_row(b"abc"), inline_row(b"hello"), heap_row(5, 7)];
        let heap = |_p: u32, n: u32| b"hello"[..n as usize].to_vec();
        assert_eq!(rows[0].len(), 3);
        assert_eq!(w20a07_find_row(&rows, &inline_row(b"hello"), heap), 1);
        assert_eq!(w20a07_find_row(&rows, &heap_row(5, 7), heap), 1);
        assert_eq!(w20a07_find_row(&rows, &inline_row(b"nope"), heap), 3);
        assert_eq!(w20a07_find_row(&[], &inline_row(b"a"), heap), 0);
    }
}
