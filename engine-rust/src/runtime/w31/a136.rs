#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent136.rs
// ===========================================================================
// Wave 31 translation batch 136: CRT string helpers (strncpy_s, wide
// case-insensitive compares, wide upper-casing).

/// errno EINVAL.
pub const W31A136_EINVAL: u32 = 0x16;
/// errno ERANGE.
pub const W31A136_ERANGE: u32 = 0x22;
/// STRUNCATE.
pub const W31A136_STRUNCATE: u32 = 0x50;

/// `0x00ca931c`: bounds-checked narrow string copy (`strncpy_s`).  `dst`
/// is the destination (its length is the buffer size, `None` is a null
/// pointer), `count` is the copy limit (`-1` means truncate).  Returns the
/// errno-style status (0 on success).
pub fn w31a136_strncpy_s(dst: Option<&mut [u8]>, src: Option<&[u8]>, count: i32) -> u32 {
    let Some(dst) = dst else {
        return if count == 0 { 0 } else { W31A136_EINVAL };
    };
    let size = dst.len();
    if size == 0 {
        return W31A136_EINVAL;
    }
    if count == 0 {
        dst[0] = 0;
        return 0;
    }
    let Some(src) = src else {
        dst[0] = 0;
        return W31A136_EINVAL;
    };
    let at = |i: usize| src.get(i).copied().unwrap_or(0);
    let mut i = 0usize;
    let mut left = size;
    let mut cnt = count;
    if count == -1 {
        loop {
            let c = at(i);
            dst[i] = c;
            i += 1;
            if c == 0 {
                break;
            }
            left -= 1;
            if left == 0 {
                break;
            }
        }
    } else {
        loop {
            let c = at(i);
            dst[i] = c;
            i += 1;
            if c == 0 {
                break;
            }
            left -= 1;
            if left == 0 {
                break;
            }
            cnt -= 1;
            if cnt == 0 {
                break;
            }
        }
        if cnt == 0 {
            dst[i] = 0;
        }
    }
    if left != 0 {
        return 0;
    }
    if count == -1 {
        dst[size - 1] = 0;
        return W31A136_STRUNCATE;
    }
    dst[0] = 0;
    W31A136_ERANGE
}

fn w31a136_ascii_lower(c: u16) -> u16 {
    if c.wrapping_sub(0x41) < 0x1a {
        c + 0x20
    } else {
        c
    }
}

/// Shared compare loop of the "C" locale paths: compares up to `limit`
/// units (None = unbounded), lower-casing ASCII letters through `fold`.
fn w31a136_wcmp(a: &[u16], b: &[u16], limit: Option<usize>, fold: &dyn Fn(u16) -> u16) -> i32 {
    let at = |s: &[u16], i: usize| s.get(i).copied().unwrap_or(0);
    let mut i = 0usize;
    loop {
        let x = fold(at(a, i));
        let y = fold(at(b, i));
        i += 1;
        let more = match limit {
            Some(n) => i < n,
            None => true,
        };
        if !(more && x != 0 && x == y) {
            return x as i32 - y as i32;
        }
    }
}

/// `0x00ca942a`: `_wcsicmp` with the ASCII-only fold used when no locale
/// is active.  `None` operands give `i32::MAX` (EINVAL path).
pub fn w31a136_wcsicmp_c(a: Option<&[u16]>, b: Option<&[u16]>) -> i32 {
    match (a, b) {
        (Some(a), Some(b)) => w31a136_wcmp(a, b, None, &w31a136_ascii_lower),
        _ => i32::MAX,
    }
}

/// `0x00ca94b8`: `_wcsicmp_l`.  `locale_fold` is `None` for the ASCII
/// fold (locale record field `+0xa8 == 0`), otherwise the locale's
/// lower-case mapping.
pub fn w31a136_wcsicmp_l(
    a: Option<&[u16]>,
    b: Option<&[u16]>,
    locale_fold: Option<&dyn Fn(u16) -> u16>,
) -> i32 {
    match (a, b) {
        (Some(a), Some(b)) => match locale_fold {
            None => w31a136_wcmp(a, b, None, &w31a136_ascii_lower),
            Some(f) => w31a136_wcmp(a, b, None, f),
        },
        _ => i32::MAX,
    }
}

/// `0x00ca9595`: `_wcsnicmp` in the ASCII-fold case.  A zero `count`
/// compares equal; `None` operands give `i32::MAX`.
pub fn w31a136_wcsnicmp_c(a: Option<&[u16]>, b: Option<&[u16]>, count: usize) -> i32 {
    if count == 0 {
        return 0;
    }
    match (a, b) {
        (Some(a), Some(b)) => w31a136_wcmp(a, b, Some(count), &w31a136_ascii_lower),
        _ => i32::MAX,
    }
}

/// `0x00ca9646`: `_wcsnicmp_l`; `locale_fold` as in [`w31a136_wcsicmp_l`].
pub fn w31a136_wcsnicmp_l(
    a: Option<&[u16]>,
    b: Option<&[u16]>,
    count: usize,
    locale_fold: Option<&dyn Fn(u16) -> u16>,
) -> i32 {
    if count == 0 {
        return 0;
    }
    match (a, b) {
        (Some(a), Some(b)) => match locale_fold {
            None => w31a136_wcmp(a, b, Some(count), &w31a136_ascii_lower),
            Some(f) => w31a136_wcmp(a, b, Some(count), f),
        },
        _ => i32::MAX,
    }
}

/// `0x00ca98c0`: `_wcsupr` in the ASCII case: upper-cases `a-z` in place up
/// to the terminating zero.  Returns false for a null string.
pub fn w31a136_wcsupr_c(s: Option<&mut [u16]>) -> bool {
    let Some(s) = s else { return false };
    for c in s.iter_mut() {
        if *c == 0 {
            break;
        }
        if c.wrapping_sub(0x61) < 0x1a {
            *c -= 0x20;
        }
    }
    true
}

#[cfg(test)]
mod tests_wave31_agent136 {
    use super::*;

    fn w(s: &str) -> Vec<u16> {
        let mut v: Vec<u16> = s.encode_utf16().collect();
        v.push(0);
        v
    }

    #[test]
    fn strncpy() {
        let mut d = [9u8; 8];
        assert_eq!(w31a136_strncpy_s(Some(&mut d), Some(b"abc\0"), -1), 0);
        assert_eq!(&d[..4], b"abc\0");
        let mut d = [9u8; 3];
        assert_eq!(w31a136_strncpy_s(Some(&mut d), Some(b"abcdef\0"), -1), 0x50);
        assert_eq!(&d, b"ab\0");
        let mut d = [9u8; 8];
        assert_eq!(w31a136_strncpy_s(Some(&mut d), Some(b"abcdef\0"), 2), 0);
        assert_eq!(&d[..3], b"ab\0");
        let mut d = [9u8; 2];
        assert_eq!(w31a136_strncpy_s(Some(&mut d), Some(b"abcdef\0"), 5), 0x22);
        assert_eq!(d[0], 0);
        assert_eq!(w31a136_strncpy_s(None, None, 0), 0);
        assert_eq!(w31a136_strncpy_s(None, None, 1), 0x16);
    }

    #[test]
    fn compares() {
        assert_eq!(w31a136_wcsicmp_c(Some(&w("AbC")), Some(&w("abc"))), 0);
        assert!(w31a136_wcsicmp_c(Some(&w("abd")), Some(&w("ABC"))) > 0);
        assert_eq!(w31a136_wcsicmp_c(None, Some(&w("a"))), i32::MAX);
        assert_eq!(w31a136_wcsicmp_l(Some(&w("A")), Some(&w("a")), None), 0);
        assert_eq!(w31a136_wcsnicmp_c(Some(&w("abcX")), Some(&w("ABCy")), 3), 0);
        assert!(w31a136_wcsnicmp_c(Some(&w("abcX")), Some(&w("ABCy")), 4) < 0);
        assert_eq!(w31a136_wcsnicmp_c(None, None, 0), 0);
        assert_eq!(
            w31a136_wcsnicmp_l(Some(&w("a")), Some(&w("B")), 1, None),
            -1
        );
    }

    #[test]
    fn upper() {
        let mut s = w("aZ-q");
        assert!(w31a136_wcsupr_c(Some(&mut s)));
        assert_eq!(s, w("AZ-Q"));
        assert!(!w31a136_wcsupr_c(None));
    }
}
