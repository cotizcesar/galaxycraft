#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent119.rs
// ===========================================================================
// Wave 32 translation batch 119.
//
// Skipped: FUN_00c66b20 (calls the unmodelled integer formatter FUN_00c6b9f0).

fn w32a119_strlen(b: &[u8]) -> usize {
    b.iter().position(|&c| c == 0).unwrap_or(b.len())
}

fn w32a119_at(b: &[u8], i: usize) -> u8 {
    b.get(i).copied().unwrap_or(0)
}

fn w32a119_strlcpy(dst: &mut [u8], src: &[u8]) {
    if dst.is_empty() {
        return;
    }
    let n = (dst.len() - 1).min(w32a119_strlen(src));
    dst[..n].copy_from_slice(&src[..n]);
    dst[n] = 0;
}

fn w32a119_strlcat(dst: &mut [u8], src: &[u8]) {
    let d = w32a119_strlen(dst);
    if d < dst.len() {
        let room = dst.len() - d;
        let n = (room - 1).min(w32a119_strlen(src));
        dst[d..d + n].copy_from_slice(&src[..n]);
        dst[d + n] = 0;
    }
}

/// `0x00c673c0`: normalises a path into `dst` (capacity `dst.len()`): collapses
/// `.` and empty segments, resolves `..` against previously emitted segments,
/// keeps a leading `/`, and lower-cases a `X:/` drive letter.  `src` is the
/// NUL-terminated input (a copy of `dst` when the original runs in place).
pub fn w32a119_normalize_path(dst: &mut [u8], src: &[u8]) {
    if dst.is_empty() {
        return;
    }
    let limit = dst.len() - 1;
    let (mut o, mut s, mut p) = (0usize, 0usize, 0usize);
    let (mut local_c, mut local_10) = (0i32, 0i32);
    loop {
        let e = p;
        let c = w32a119_at(src, e);
        if c != b'/' && c != b'\\' && c != 0 {
            p += 1;
            continue;
        }
        p = e + 1;
        let mut is_dd = false;
        let mut len = e - s;
        // true: goto LAB_00c67473 (emit); false: goto LAB_00c67456 / skip check
        let mut emit = false;
        let mut check_skip = false;
        if len == 1 {
            if w32a119_at(src, s) != b'.' {
                emit = true;
            } else {
                len = 0;
                check_skip = true;
            }
        } else {
            if len == 2 {
                if w32a119_at(src, s) != b'.' || w32a119_at(src, s + 1) != b'.' {
                    emit = true;
                } else {
                    if local_c != 0 {
                        len = 0;
                        while o > 0 {
                            o -= 1;
                            if dst[o] == b'/' {
                                break;
                            }
                        }
                        local_c -= 1;
                    }
                    is_dd = true;
                }
            }
            if !emit {
                if len == 0 {
                    check_skip = true;
                } else {
                    emit = true;
                }
            }
        }
        if check_skip && !(o == 0 || (o == 1 && dst[0] == b'/') || c == 0) {
            emit = false;
        } else if check_skip {
            emit = true;
        }
        if emit {
            if s == 0 && e == 0 && c != 0 {
                if o < limit {
                    dst[o] = b'/';
                    o += 1;
                }
            } else if s == 1 && e == 1 && o == 1 && dst[0] == b'/' && c != 0 {
                if o < limit {
                    dst[o] = b'/';
                    o += 1;
                }
            } else {
                if (local_c != 0 || local_10 != 0) && o < limit {
                    dst[o] = b'/';
                    o += 1;
                }
                for k in 0..len {
                    if o < limit {
                        dst[o] = w32a119_at(src, s + k);
                        o += 1;
                    }
                }
                if len != 0 {
                    if is_dd {
                        local_10 += 1;
                    } else {
                        local_c += 1;
                    }
                }
            }
        }
        s = p;
        if c == 0 {
            dst[o] = 0;
            let d0 = dst[0];
            if d0 != 0 && w32a119_at(dst, 1) == b':' && w32a119_at(dst, 2) == b'/' {
                dst[0] = d0.to_ascii_lowercase();
            }
            return;
        }
    }
}

/// `0x00c672e0`: joins `base` and `rel` into `dst`: copies `base`, forces a
/// trailing `/` (backslash converted), restarts at `dst` if `rel` is absolute
/// (`/`, `\` or `X:/`), appends `rel`, then normalises in place with
/// `0x00c673c0`.  The separator literal is taken to be `"/"`.
pub fn w32a119_join_path(dst: &mut [u8], base: Option<&[u8]>, rel: Option<&[u8]>) {
    if let Some(b) = base {
        w32a119_strlcpy(dst, b);
    }
    let end = w32a119_strlen(dst);
    if end > 0 {
        let mut c = dst[end - 1];
        if c == b'\\' {
            dst[end - 1] = b'/';
            c = b'/';
        }
        if c != b'/' {
            w32a119_strlcat(dst, b"/\0");
        }
    }
    if let Some(r) = rel {
        let c = w32a119_at(r, 0);
        if c != 0 {
            let drive = w32a119_at(r, 1) == b':' && matches!(w32a119_at(r, 2), b'/' | b'\\');
            if c == b'/' || c == b'\\' || drive {
                dst[0] = 0;
            }
            w32a119_strlcat(dst, r);
        }
    }
    let copy = dst.to_vec();
    w32a119_normalize_path(dst, &copy);
}

#[cfg(test)]
mod tests_wave32_agent119 {
    use super::*;

    fn norm(s: &str) -> String {
        let mut src = s.as_bytes().to_vec();
        src.push(0);
        let mut dst = vec![0u8; 64];
        w32a119_normalize_path(&mut dst, &src);
        String::from_utf8(dst[..w32a119_strlen(&dst)].to_vec()).unwrap()
    }

    #[test]
    fn normalizes() {
        assert_eq!(norm("a/./b//c"), "a/b/c");
        assert_eq!(norm("a/b/../c"), "a/c");
        assert_eq!(norm("/a/b"), "/a/b");
        assert_eq!(norm("C:/x/../y"), "c:/y");
        assert_eq!(norm("a\\b"), "a/b");
    }

    #[test]
    fn joins() {
        let mut d = vec![0u8; 64];
        w32a119_join_path(&mut d, Some(b"root\\dir\0"), Some(b"sub/../f.txt\0"));
        assert_eq!(&d[..w32a119_strlen(&d)], b"root/dir/f.txt");
        w32a119_join_path(&mut d, Some(b"root\0"), Some(b"/abs\0"));
        assert_eq!(&d[..w32a119_strlen(&d)], b"/abs");
    }
}
