#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent24.rs
// ===========================================================================
// Wave 31 translation batch 24: fmt::v6 argument-id parsing and a
// four-byte-stride memory compare.

/// Result of parsing a replacement-field argument id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum W31a24ArgId {
    /// Automatic or manual positional index.
    Index(u32),
    /// Named argument: byte range `[start, end)` into the input.
    Name(usize, usize),
}

/// Outcome of `w31a24_parse_arg_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W31a24Parse {
    /// Position after the consumed id.
    pub pos: usize,
    /// Parsed id, when one was produced.
    pub id: Option<W31a24ArgId>,
    /// Error messages raised, in order.
    pub errors: Vec<&'static str>,
}

/// `0x00557cc0`: fmt::v6 `parse_arg_id`.  `next_index` is the context's
/// auto-index counter (negative means manual indexing is in force).
pub fn w31a24_parse_arg_id(text: &[u8], start: usize, next_index: &mut i32) -> W31a24Parse {
    let end = text.len();
    let at = |i: usize| -> u8 { text.get(i).copied().unwrap_or(0) };
    let mut errors = Vec::new();
    let c = at(start);
    if c == b'}' || c == b':' {
        let idx = if *next_index < 0 {
            errors.push("cannot switch from manual to automatic argument indexing");
            0
        } else {
            let v = *next_index;
            *next_index = v + 1;
            v as u32
        };
        return W31a24Parse {
            pos: start,
            id: Some(W31a24ArgId::Index(idx)),
            errors,
        };
    }
    if !c.is_ascii_digit() {
        if !(c.is_ascii_alphabetic() || c == b'_') {
            errors.push("invalid format string");
            return W31a24Parse {
                pos: start,
                id: None,
                errors,
            };
        }
        let mut p = start + 1;
        while p != end {
            let d = at(p);
            if !(d.is_ascii_alphanumeric() || d == b'_') {
                break;
            }
            p += 1;
        }
        return W31a24Parse {
            pos: p,
            id: Some(W31a24ArgId::Name(start, p)),
            errors,
        };
    }
    let mut pos = start;
    let mut v: u32 = 0;
    if c == b'0' {
        pos += 1;
    } else {
        let mut c = c;
        loop {
            if v > 0x0ccc_cccc {
                v = 0x8000_0000;
                errors.push("number is too big");
                break;
            }
            pos += 1;
            v = (c as u32).wrapping_add(v.wrapping_mul(5).wrapping_sub(0x18).wrapping_mul(2));
            if pos == end {
                break;
            }
            c = at(pos);
            if !c.is_ascii_digit() {
                break;
            }
        }
        if errors.is_empty() && v > 0x7fff_ffff {
            errors.push("number is too big");
        }
    }
    if pos != end && (at(pos) == b'}' || at(pos) == b':') {
        if *next_index < 1 {
            *next_index = -1;
        } else {
            errors.push("cannot switch from automatic to manual argument indexing");
        }
        return W31a24Parse {
            pos,
            id: Some(W31a24ArgId::Index(v)),
            errors,
        };
    }
    errors.push("invalid format string");
    W31a24Parse {
        pos,
        id: None,
        errors,
    }
}

/// `0x0055c520`: memcmp over `n` bytes; returns 0 when equal, otherwise
/// `-1` (as u32 `0xffffffff`) when the first differing byte of `a` is
/// smaller, else `1`.
pub fn w31a24_memcmp(a: &[u8], b: &[u8], n: usize) -> u32 {
    for i in 0..n {
        if a[i] != b[i] {
            return if a[i] < b[i] { 0xffff_ffff } else { 1 };
        }
    }
    0
}

#[cfg(test)]
mod tests_wave31_agent24 {
    use super::*;

    #[test]
    fn memcmp_cases() {
        assert_eq!(w31a24_memcmp(b"abcdefg", b"abcdefg", 7), 0);
        assert_eq!(w31a24_memcmp(b"abcdefg", b"abcdefh", 7), 0xffff_ffff);
        assert_eq!(w31a24_memcmp(b"abcdxfg", b"abcdefg", 7), 1);
        assert_eq!(w31a24_memcmp(b"x", b"y", 0), 0);
        assert_eq!(w31a24_memcmp(b"bz", b"az", 2), 1);
    }

    #[test]
    fn arg_id_auto_and_manual() {
        let mut n = 0;
        let r = w31a24_parse_arg_id(b"}", 0, &mut n);
        assert_eq!(r.id, Some(W31a24ArgId::Index(0)));
        assert_eq!(n, 1);
        let mut m = -1;
        let r = w31a24_parse_arg_id(b":", 0, &mut m);
        assert_eq!(r.errors.len(), 1);
        let mut n = 0;
        let r = w31a24_parse_arg_id(b"12}", 0, &mut n);
        assert_eq!(r.id, Some(W31a24ArgId::Index(12)));
        assert_eq!(r.pos, 2);
        assert_eq!(n, -1);
        let mut n = 2;
        let r = w31a24_parse_arg_id(b"0}", 0, &mut n);
        assert_eq!(r.errors.len(), 1);
    }

    #[test]
    fn arg_id_name_and_errors() {
        let mut n = 0;
        let r = w31a24_parse_arg_id(b"ab_1}", 0, &mut n);
        assert_eq!(r.id, Some(W31a24ArgId::Name(0, 4)));
        let r = w31a24_parse_arg_id(b"!", 0, &mut n);
        assert_eq!(r.errors, vec!["invalid format string"]);
        let mut n = 0;
        let r = w31a24_parse_arg_id(b"99999999999}", 0, &mut n);
        assert_eq!(r.errors[0], "number is too big");
        let r = w31a24_parse_arg_id(b"12x", 0, &mut n);
        assert!(r.id.is_none());
    }
}
