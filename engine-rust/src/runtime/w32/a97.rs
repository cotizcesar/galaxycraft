#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent97.rs
// ===========================================================================
// Wave 32 agent 97: config `key = value` line scanner.

fn w32a97_is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | 0x0b | b'\r' | 0x0c | b'\n')
}

fn w32a97_is_key(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'_')
}

/// `0x00ae46a0`: scans `buf` line by line for `name = value`.  Blank and
/// `#` lines are skipped.  Returns `(1, Some(value_offset), value_len)` when
/// the key equals `name` (NUL-terminated), `(0, None, None)` when no line
/// matches, and `(-1, None, None)` on a malformed line (empty key or no
/// `=`).  The value length is measured to the trimmed line end.
pub fn w32a97_find_key(buf: &[u8], name: Option<&[u8]>) -> (i32, Option<usize>, Option<isize>) {
    let name = name.map(|n| {
        let l = n.iter().position(|&c| c == 0).unwrap_or(n.len());
        &n[..l]
    });
    let end = buf.len();
    let mut cur = Some(0usize);
    loop {
        let start = match cur {
            Some(c) if c < end => c,
            _ => return (0, None, None),
        };
        let (mut e, next) = match buf[start..].iter().position(|&c| c == b'\n') {
            Some(i) => (start + i, Some(start + i + 1)),
            None => (end, None),
        };
        cur = next;
        while start < e && w32a97_is_ws(buf[e - 1]) {
            e -= 1;
        }
        let mut s = start;
        while s < e && w32a97_is_ws(buf[s]) {
            s += 1;
        }
        if s == e || buf[s] == b'#' {
            continue;
        }
        let mut p = s;
        while p < end && w32a97_is_key(buf[p]) {
            p += 1;
        }
        let klen = p - s;
        if klen == 0 {
            return (-1, None, None);
        }
        while p < end && w32a97_is_ws(buf[p]) {
            p += 1;
        }
        if p == end || buf[p] != b'=' {
            return (-1, None, None);
        }
        loop {
            p += 1;
            if p >= end || !w32a97_is_ws(buf[p]) {
                break;
            }
        }
        if let Some(n) = name {
            if n.len() == klen && &buf[s..s + klen] == n {
                return (1, Some(p), Some(e as isize - p as isize));
            }
        }
    }
}

#[cfg(test)]
mod tests_wave32_agent97 {
    use super::*;

    #[test]
    fn finds_value() {
        let b = b"# c\n\n  a = 1 \nfoo=  bar baz\r\n";
        let r = w32a97_find_key(b, Some(b"foo\0"));
        assert_eq!(r.0, 1);
        let o = r.1.unwrap();
        assert_eq!(&b[o..o + r.2.unwrap() as usize], b"bar baz");
        assert_eq!(w32a97_find_key(b, Some(b"zzz")).0, 0);
    }

    #[test]
    fn malformed() {
        assert_eq!(w32a97_find_key(b"abc\n", Some(b"abc")).0, -1);
        assert_eq!(w32a97_find_key(b"=x\n", Some(b"abc")).0, -1);
    }
}
