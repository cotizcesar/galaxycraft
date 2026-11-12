#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent115.rs
// ===========================================================================
// Wave 31 translation batch 115: bounded C-string copy/concat helpers and
// a path join (runtime.rs target).

fn w31a115_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

/// `0x00c68300`: bounded string append (strlcat).  `dst` holds a
/// NUL-terminated string in a buffer of `size` bytes; appends as much of
/// `src` as fits (leaving room for the NUL) and returns the length the
/// result would have had (`len(dst) + len(src)`).
pub fn w31a115_strlcat(dst: &mut [u8], size: usize, src: &[u8]) -> usize {
    let dlen = w31a115_strlen(dst);
    let slen = w31a115_strlen(src);
    let room = if dlen < size { size - dlen } else { 0 };
    if room != 0 {
        let n = (room - 1).min(slen);
        dst[dlen..dlen + n].copy_from_slice(&src[..n]);
        dst[dlen + n] = 0;
    }
    slen + dlen
}

/// `0x00c683a0`: bounded string copy (strlcpy); returns `len(src)`.
pub fn w31a115_strlcpy(dst: &mut [u8], size: usize, src: &[u8]) -> usize {
    let slen = w31a115_strlen(src);
    if size != 0 {
        let n = (size - 1).min(slen);
        dst[..n].copy_from_slice(&src[..n]);
        dst[n] = 0;
    }
    slen
}

/// `0x00c68490`: bounded copy of at most `max` source bytes (stops at NUL);
/// returns the counted length (strnlen of the source).
pub fn w31a115_strlncpy(dst: &mut [u8], size: usize, src: &[u8], max: usize) -> usize {
    let lim = max.min(src.len());
    let slen = src[..lim].iter().position(|&b| b == 0).unwrap_or(lim);
    if size == 0 {
        return slen;
    }
    let n = (size - 1).min(slen);
    dst[..n].copy_from_slice(&src[..n]);
    dst[n] = 0;
    slen
}

/// `0x00c67a90`: wrapper over an OS call that returns a value through an
/// out parameter.  On failure the out value is zero and the last-error
/// callback result is returned; on success returns 0.
pub fn w31a115_call_with_out(
    call: impl FnOnce() -> Option<u32>,
    last_error: impl FnOnce() -> u32,
) -> (u32, u32) {
    match call() {
        Some(v) => (v, 0),
        None => (0, last_error()),
    }
}

/// `0x00c67560`: builds `dir-prefix + '/' + name` into `dst`.  `dot` and
/// `slash` are the two string constants the retail code uses (current
/// directory fallback and separator); `normalize` is the final path clean-up
/// call.  A rooted or drive-qualified `name` replaces the directory part.
pub fn w31a115_path_join(
    dst: &mut [u8],
    size: usize,
    dir: &[u8],
    name: Option<&[u8]>,
    dot: &[u8],
    slash: &[u8],
    normalize: impl FnOnce(&mut [u8]),
) {
    let dlen = w31a115_strlen(dir);
    let last = dir[..dlen].iter().rposition(|&c| c == b'/' || c == b'\\');
    match last {
        Some(p) => {
            w31a115_strlncpy(dst, size, dir, p);
        }
        _ => {
            w31a115_strlcpy(dst, size, dot);
        }
    }
    let l = w31a115_strlen(dst);
    if l > 0 {
        let mut c = dst[l - 1];
        if c == b'\\' {
            dst[l - 1] = b'/';
            c = b'/';
        }
        if c != b'/' {
            w31a115_strlcat(dst, size, slash);
        }
    }
    if let Some(n) = name {
        if n.first().is_some_and(|&c| c != 0) {
            let c0 = n[0];
            let sep = |c: u8| c == b'/' || c == b'\\';
            if sep(c0) || (n.get(1) == Some(&b':') && n.get(2).is_some_and(|&c| sep(c))) {
                dst[0] = 0;
            }
            w31a115_strlcat(dst, size, n);
        }
    }
    normalize(dst);
}

#[cfg(test)]
mod tests_wave31_agent115 {
    use super::*;

    fn s(b: &[u8]) -> &[u8] {
        &b[..w31a115_strlen(b)]
    }

    #[test]
    fn copy_and_cat() {
        let mut b = [0xffu8; 8];
        assert_eq!(w31a115_strlcpy(&mut b, 8, b"abc\0"), 3);
        assert_eq!(s(&b), b"abc");
        assert_eq!(w31a115_strlcat(&mut b, 8, b"defghij\0"), 10);
        assert_eq!(s(&b), b"abcdefg");
        assert_eq!(w31a115_strlcat(&mut b, 4, b"x\0"), 8);
        let mut c = [9u8; 4];
        assert_eq!(w31a115_strlcpy(&mut c, 0, b"zz\0"), 2);
        assert_eq!(c, [9; 4]);
    }

    #[test]
    fn ncopy_and_out() {
        let mut b = [0xffu8; 8];
        assert_eq!(w31a115_strlncpy(&mut b, 8, b"hello\0", 3), 3);
        assert_eq!(s(&b), b"hel");
        assert_eq!(w31a115_strlncpy(&mut b, 0, b"hello\0", 9), 5);
        assert_eq!(w31a115_call_with_out(|| Some(7), || 5), (7, 0));
        assert_eq!(w31a115_call_with_out(|| None, || 5), (0, 5));
    }

    #[test]
    fn join() {
        let mut b = [0u8; 64];
        w31a115_path_join(
            &mut b,
            64,
            b"a\\b\\c.txt\0",
            Some(b"d.mpq\0"),
            b".\0",
            b"/\0",
            |_| {},
        );
        assert_eq!(s(&b), b"a\\b/d.mpq");
        let mut b = [0u8; 64];
        w31a115_path_join(
            &mut b,
            64,
            b"c.txt\0",
            Some(b"C:\\x\0"),
            b".\0",
            b"/\0",
            |_| {},
        );
        assert_eq!(s(&b), b"C:\\x");
    }
}
