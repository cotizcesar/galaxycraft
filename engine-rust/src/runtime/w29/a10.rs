#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent10.rs
// ===========================================================================
// Wave 29 translation batch 10: blz stream-buffer / token helpers.

/// `0x00ac80b0`: splits `text` on the ASCII whitespace set (space, \t, \v,
/// \r, \f, \n) into tokens; the output vector is cleared first.
pub fn w29a10_split_whitespace(text: &[u8], out: &mut Vec<Vec<u8>>) {
    out.clear();
    let is_ws = |c: u8| matches!(c, b' ' | b'\t' | 0x0b | b'\r' | 0x0c | b'\n');
    let mut i = 0;
    while i < text.len() {
        while i < text.len() && is_ws(text[i]) {
            i += 1;
        }
        if i >= text.len() {
            break;
        }
        let start = i;
        while i < text.len() && !is_ws(text[i]) {
            i += 1;
        }
        out.push(text[start..i].to_vec());
    }
}

/// `0x00ac8590`: formats a token list (0x1c-byte records) into `dst` as
/// space-joined text and returns the full length needed.  `skip_if_default`
/// models the (flags & 4 == 0 && default-equal) early return of 0.
pub fn w29a10_join_tokens(tokens: &[Vec<u8>], dst: &mut [u8], skip_if_default: bool) -> usize {
    if skip_if_default {
        return 0;
    }
    let mut room = dst.len();
    let mut pos = 0usize;
    let mut total = 0usize;
    for (i, tok) in tokens.iter().enumerate() {
        if i > 0 {
            if room != 0 {
                dst[pos] = b' ';
                pos += 1;
                room -= 1;
            }
            total += 1;
        }
        let n = tok.len().min(room);
        dst[pos..pos + n].copy_from_slice(&tok[..n]);
        pos += n;
        room -= n;
        total += tok.len();
    }
    total
}

/// `0x00ac8650`: boolean branch of the option formatter.  Returns 0 when the
/// value equals the default and flag bit 2 is clear; when `kind == 2` writes
/// `'0'`/`'1'` and returns 1.  `None` means the string-conversion branch.
pub fn w29a10_format_bool(
    dst: &mut [u8],
    flags: u8,
    value: u8,
    default: u8,
    kind: i32,
) -> Option<u32> {
    if flags & 4 == 0 && value == default {
        return Some(0);
    }
    if kind != 2 {
        return None;
    }
    if let Some(b) = dst.first_mut() {
        *b = if value != 0 { b'1' } else { b'0' };
    }
    Some(1)
}

/// Output stream buffer state used by `0x00ac86e0` and `0x00ac88f0`.
#[derive(Debug, Clone, Default)]
pub struct W29a10OutBuf {
    pub has_file: bool,
    pub buf: Vec<u8>,
    pub buf_len: usize,
    pub owns: bool,
    pub pbase: Option<usize>,
    pub pptr: usize,
    pub epptr: usize,
    pub get_area: [usize; 3],
}

/// `0x00ac86e0`: overflow.  Lazily sets the put area (one inline byte when
/// unbuffered, `buf_len - 1` otherwise), stores `ch`, flushes through `write`
/// when full or `ch == -1`.  Returns `ch` (0 for a successful flush of -1,
/// -1 on error).
pub fn w29a10_overflow(
    s: &mut W29a10OutBuf,
    mut ch: i32,
    write: &mut dyn FnMut(&[u8]) -> usize,
) -> i32 {
    if !s.has_file {
        return -1;
    }
    if s.pbase.is_none() {
        let (end, _) = if s.buf_len == 0 {
            (1, 0)
        } else {
            (s.buf_len - 1, 0)
        };
        if s.buf.len() < end + 1 {
            s.buf.resize(end + 1, 0);
        }
        s.pbase = Some(0);
        s.pptr = 0;
        s.epptr = end;
        s.get_area = [0; 3];
    }
    if ch != -1 {
        let p = s.pptr;
        s.buf[p] = ch as u8;
        s.pptr += 1;
    }
    if s.epptr <= s.pptr || ch == -1 {
        let n = s.pptr - s.pbase.unwrap_or(0);
        let wrote = write(&s.buf[s.pbase.unwrap_or(0)..s.pbase.unwrap_or(0) + n]);
        if wrote == n {
            if ch == -1 {
                ch = 0;
            }
        } else {
            ch = -1;
        }
        s.pptr = s.pbase.unwrap_or(0);
    }
    if s.buf_len == 0 {
        s.pbase = None;
        s.pptr = 0;
        s.epptr = 0;
    }
    ch
}

/// `0x00ac88f0`: setbuf.  Calls `sync`; on -1 returns false.  Otherwise
/// replaces the buffer (inline when size and ptr are both zero) and resets
/// all six get/put pointers.  `new_buf_len` is the requested size.
pub fn w29a10_setbuf(
    s: &mut W29a10OutBuf,
    new_buf_len: usize,
    sync: &mut dyn FnMut() -> i32,
) -> bool {
    if sync() == -1 {
        return false;
    }
    s.owns = new_buf_len != 0;
    s.buf = vec![0; new_buf_len];
    s.buf_len = new_buf_len;
    s.pbase = None;
    s.pptr = 0;
    s.epptr = 0;
    s.get_area = [0; 3];
    true
}

/// `0x00ac9310`: copies up to `min(dst.len(), src_len)` bytes of `src`.
pub fn w29a10_copy_limited(dst: &mut [u8], src: &[u8], src_len: usize) -> usize {
    let n = dst.len().min(src_len).min(src.len());
    dst[..n].copy_from_slice(&src[..n]);
    n
}

/// `0x00ac9600`: zero-initialises `count` 24-byte records; returns 0.
pub fn w29a10_zero_records(records: &mut [[u32; 6]], count: usize) -> u32 {
    for r in records.iter_mut().take(count) {
        *r = [0; 6];
    }
    0
}

/// `0x00ac99a0`: strips trailing `/` and `\` from a path.  Returns true when
/// the result is empty (the caller then appends its default component).
pub fn w29a10_trim_trailing_slashes(path: &mut Vec<u8>) -> bool {
    while let Some(&c) = path.last() {
        if c != b'/' && c != b'\\' {
            break;
        }
        path.pop();
    }
    path.is_empty()
}

#[cfg(test)]
mod tests_wave29_agent10 {
    use super::*;

    #[test]
    fn split_and_join() {
        let mut v = Vec::new();
        w29a10_split_whitespace(b"  ab \t cd\n", &mut v);
        assert_eq!(v, vec![b"ab".to_vec(), b"cd".to_vec()]);
        let mut d = [0u8; 8];
        assert_eq!(w29a10_join_tokens(&v, &mut d, false), 5);
        assert_eq!(&d[..5], b"ab cd");
        let mut small = [0u8; 3];
        assert_eq!(w29a10_join_tokens(&v, &mut small, false), 5);
        assert_eq!(&small, b"ab ");
        assert_eq!(w29a10_join_tokens(&v, &mut d, true), 0);
    }

    #[test]
    fn bool_format() {
        let mut d = [0u8; 2];
        assert_eq!(w29a10_format_bool(&mut d, 0, 1, 1, 2), Some(0));
        assert_eq!(w29a10_format_bool(&mut d, 4, 7, 7, 2), Some(1));
        assert_eq!(d[0], b'1');
        assert_eq!(w29a10_format_bool(&mut d, 0, 0, 1, 2), Some(1));
        assert_eq!(d[0], b'0');
        assert_eq!(w29a10_format_bool(&mut d, 0, 0, 1, 1), None);
    }

    #[test]
    fn overflow_flush() {
        let mut s = W29a10OutBuf {
            has_file: true,
            buf_len: 4,
            ..Default::default()
        };
        let mut out = Vec::new();
        let mut w = |b: &[u8]| {
            out.extend_from_slice(b);
            b.len()
        };
        assert_eq!(w29a10_overflow(&mut s, b'a' as i32, &mut w), b'a' as i32);
        assert_eq!(w29a10_overflow(&mut s, -1, &mut w), 0);
        let mut n = W29a10OutBuf::default();
        assert_eq!(w29a10_overflow(&mut n, 1, &mut w), -1);
        assert_eq!(out, b"a");
    }

    #[test]
    fn setbuf_and_misc() {
        let mut s = W29a10OutBuf::default();
        assert!(!w29a10_setbuf(&mut s, 4, &mut || -1));
        assert!(w29a10_setbuf(&mut s, 4, &mut || 0));
        assert!(s.owns && s.buf.len() == 4);
        let mut d = [0u8; 3];
        assert_eq!(w29a10_copy_limited(&mut d, b"hello", 5), 3);
        assert_eq!(&d, b"hel");
        let mut r = [[1u32; 6]; 2];
        assert_eq!(w29a10_zero_records(&mut r, 1), 0);
        assert_eq!(r[0], [0; 6]);
        assert_eq!(r[1], [1; 6]);
        let mut p = b"a/b//\\".to_vec();
        assert!(!w29a10_trim_trailing_slashes(&mut p));
        assert_eq!(p, b"a/b");
        let mut e = b"//".to_vec();
        assert!(w29a10_trim_trailing_slashes(&mut e));
    }
}
