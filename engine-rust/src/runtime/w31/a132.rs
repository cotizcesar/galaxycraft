#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent132.rs
// ===========================================================================
// Wave 31 agent 132: CRT wide-string helpers and a binary search core.
// Errno values are returned as the C return code (EINVAL 0x16, ERANGE 0x22).

/// EINVAL as stored by the CRT.
pub const W31A132_EINVAL: u32 = 0x16;
/// ERANGE as stored by the CRT.
pub const W31A132_ERANGE: u32 = 0x22;
/// STRUNCATE returned by the truncating copy.
pub const W31A132_STRUNCATE: u32 = 0x50;

fn w31a132_at(s: &[u16], i: usize) -> u16 {
    s.get(i).copied().unwrap_or(0)
}

/// `0x00ca1200`: `wcscat_s`.  `dst` is the whole buffer (its length is the
/// size argument); `src` is `None` for a null pointer.  Returns 0 or errno.
pub fn w31a132_wcscat_s(dst: Option<&mut [u16]>, src: Option<&[u16]>) -> u32 {
    let Some(dst) = dst else {
        return W31A132_EINVAL;
    };
    if dst.is_empty() {
        return W31A132_EINVAL;
    }
    let Some(src) = src else {
        dst[0] = 0;
        return W31A132_EINVAL;
    };
    let Some(mut pos) = dst.iter().position(|&c| c == 0) else {
        dst[0] = 0;
        return W31A132_EINVAL;
    };
    let mut i = 0;
    loop {
        let c = w31a132_at(src, i);
        i += 1;
        if pos >= dst.len() {
            dst[0] = 0;
            return W31A132_ERANGE;
        }
        dst[pos] = c;
        pos += 1;
        if c == 0 {
            return 0;
        }
    }
}

/// `0x00ca1275`: `wcscpy_s`.  Returns 0 or errno.
pub fn w31a132_wcscpy_s(dst: Option<&mut [u16]>, src: Option<&[u16]>) -> u32 {
    let Some(dst) = dst else {
        return W31A132_EINVAL;
    };
    if dst.is_empty() {
        return W31A132_EINVAL;
    }
    let Some(src) = src else {
        dst[0] = 0;
        return W31A132_EINVAL;
    };
    for i in 0..dst.len() {
        let c = w31a132_at(src, i);
        dst[i] = c;
        if c == 0 {
            return 0;
        }
    }
    dst[0] = 0;
    W31A132_ERANGE
}

/// `0x00ca12d9`: `wcsncpy_s`; `count == u32::MAX` is `_TRUNCATE`.
pub fn w31a132_wcsncpy_s(dst: Option<&mut [u16]>, src: Option<&[u16]>, count: u32) -> u32 {
    if count == 0 {
        if dst.is_none() {
            return if src.is_none() { 0 } else { W31A132_EINVAL };
        }
    }
    let Some(dst) = dst else {
        return W31A132_EINVAL;
    };
    if dst.is_empty() {
        return W31A132_EINVAL;
    }
    if count == 0 {
        dst[0] = 0;
        return 0;
    }
    let Some(src) = src else {
        dst[0] = 0;
        return W31A132_EINVAL;
    };
    let mut pos = 0usize;
    let mut room = dst.len();
    if count == u32::MAX {
        loop {
            let c = w31a132_at(src, pos);
            dst[pos] = c;
            pos += 1;
            if c == 0 {
                break;
            }
            room -= 1;
            if room == 0 {
                break;
            }
        }
    } else {
        let mut left = count;
        loop {
            let c = w31a132_at(src, pos);
            dst[pos] = c;
            pos += 1;
            if c == 0 {
                break;
            }
            room -= 1;
            if room == 0 {
                break;
            }
            left -= 1;
            if left == 0 {
                break;
            }
        }
        if left == 0 {
            dst[pos] = 0;
        }
    }
    if room != 0 {
        return 0;
    }
    if count == u32::MAX {
        let n = dst.len();
        dst[n - 1] = 0;
        return W31A132_STRUNCATE;
    }
    dst[0] = 0;
    W31A132_ERANGE
}

/// `0x00ca20fe`: bsearch core over `count` elements of `stride` bytes
/// starting at offset `base`; `cmp(offset)` compares the key with the
/// element at that offset.  Returns the matching offset.
pub fn w31a132_bsearch(
    base: u32,
    count: u32,
    stride: u32,
    cmp: &mut dyn FnMut(u32) -> i32,
) -> Option<u32> {
    if (base == 0 && count != 0) || stride == 0 {
        return None;
    }
    let mut lo = base;
    let mut n = count;
    let mut hi = n.wrapping_sub(1).wrapping_mul(stride).wrapping_add(lo);
    while lo <= hi {
        let half = n >> 1;
        if half == 0 {
            if n == 0 {
                return None;
            }
            return if cmp(lo) == 0 { Some(lo) } else { None };
        }
        let odd = n & 1;
        let mid_idx = if odd == 0 { half - 1 } else { half };
        let mid = mid_idx.wrapping_mul(stride).wrapping_add(lo);
        let r = cmp(mid);
        if r == 0 {
            return Some(mid);
        }
        n = half;
        if r < 0 {
            hi = mid.wrapping_sub(stride);
            if odd == 0 {
                n = half - 1;
            }
        } else {
            lo = mid.wrapping_add(stride);
        }
    }
    None
}

fn w31a132_digits(mut val: u32, radix: u32, size: u32, neg: bool) -> Result<Vec<u8>, ()> {
    let mut out = Vec::new();
    let mut used = u32::from(neg);
    if neg {
        out.push(b'-');
        val = val.wrapping_neg();
    }
    let mut digits = Vec::new();
    loop {
        let d = val % radix;
        val /= radix;
        used += 1;
        digits.push(if d > 9 {
            d as u8 + 0x27 + 0x30
        } else {
            d as u8 + 0x30
        });
        if val == 0 || used >= size {
            break;
        }
    }
    if used < size {
        digits.reverse();
        out.extend(digits);
        Ok(out)
    } else {
        Err(())
    }
}

/// `0x00ca3735`: `_ultoa_s` core (narrow).  `Ok(text)` without the NUL,
/// `Err(ERANGE)` when the buffer of `size` bytes is too small.
pub fn w31a132_xtoa(val: u32, size: u32, radix: u32, neg: bool) -> Result<Vec<u8>, u32> {
    w31a132_digits(val, radix, size, neg).map_err(|_| W31A132_ERANGE)
}

/// `0x00ca37d7`: same as the narrow core but producing UTF-16 units.
pub fn w31a132_xtow(val: u32, size: u32, radix: u32, neg: bool) -> Result<Vec<u16>, u32> {
    w31a132_xtoa(val, size, radix, neg).map(|v| v.into_iter().map(u16::from).collect())
}

/// `0x00ca388c`: `_itow_s`-style validator around `w31a132_xtow`.
/// `neg_flag` is the low byte of the last argument.
pub fn w31a132_xtow_s(val: u32, size: u32, radix: u32, neg_flag: u32) -> Result<Vec<u16>, u32> {
    if size == 0 {
        return Err(W31A132_EINVAL);
    }
    if size <= (neg_flag & 0xff) + 1 {
        return Err(W31A132_ERANGE);
    }
    if radix.wrapping_sub(2) < 0x23 {
        return w31a132_xtow(val, size, radix, neg_flag & 0xff != 0);
    }
    Err(W31A132_EINVAL)
}

#[cfg(test)]
mod tests_wave31_agent132 {
    use super::*;

    fn w(s: &str) -> Vec<u16> {
        s.encode_utf16().chain([0]).collect()
    }

    #[test]
    fn cat_cpy() {
        let mut d = [0u16; 8];
        assert_eq!(w31a132_wcscpy_s(Some(&mut d), Some(&w("ab"))), 0);
        assert_eq!(w31a132_wcscat_s(Some(&mut d), Some(&w("cd"))), 0);
        assert_eq!(&d[..5], &w("abcd")[..]);
        assert_eq!(
            w31a132_wcscat_s(Some(&mut d), Some(&w("efgh"))),
            W31A132_ERANGE
        );
        assert_eq!(d[0], 0);
        assert_eq!(w31a132_wcscpy_s(None, Some(&w("a"))), W31A132_EINVAL);
        let mut s = [0u16; 2];
        assert_eq!(
            w31a132_wcscpy_s(Some(&mut s), Some(&w("ab"))),
            W31A132_ERANGE
        );
    }

    #[test]
    fn ncpy() {
        let mut d = [9u16; 4];
        assert_eq!(w31a132_wcsncpy_s(Some(&mut d), Some(&w("abcdef")), 2), 0);
        assert_eq!(&d[..3], &[97, 98, 0]);
        let mut d = [9u16; 4];
        assert_eq!(
            w31a132_wcsncpy_s(Some(&mut d), Some(&w("abcdef")), u32::MAX),
            W31A132_STRUNCATE
        );
        assert_eq!(d, [97, 98, 99, 0]);
        let mut d = [9u16; 3];
        assert_eq!(
            w31a132_wcsncpy_s(Some(&mut d), Some(&w("abcdef")), 5),
            W31A132_ERANGE
        );
        assert_eq!(w31a132_wcsncpy_s(None, None, 0), 0);
        assert_eq!(w31a132_wcsncpy_s(None, Some(&w("a")), 3), W31A132_EINVAL);
    }

    #[test]
    fn bsearch() {
        let arr = [1, 3, 5, 7, 9, 11];
        for (i, &k) in arr.iter().enumerate() {
            let r = w31a132_bsearch(100, 6, 4, &mut |o| k - arr[((o - 100) / 4) as usize]);
            assert_eq!(r, Some(100 + 4 * i as u32));
        }
        assert_eq!(
            w31a132_bsearch(100, 6, 4, &mut |o| 4 - arr[((o - 100) / 4) as usize]),
            None
        );
        assert_eq!(w31a132_bsearch(100, 0, 4, &mut |_| 0), None);
    }

    #[test]
    fn xtoa() {
        assert_eq!(w31a132_xtoa(255, 8, 16, false).unwrap(), b"ff");
        assert_eq!(
            w31a132_xtoa(5u32.wrapping_neg(), 8, 10, true).unwrap(),
            b"-5"
        );
        assert_eq!(w31a132_xtoa(12345, 4, 10, false), Err(W31A132_ERANGE));
        assert_eq!(
            w31a132_xtow(10, 8, 2, false).unwrap(),
            w("1010")[..4].to_vec()
        );
        assert_eq!(w31a132_xtow_s(1, 0, 10, 0), Err(W31A132_EINVAL));
        assert_eq!(w31a132_xtow_s(1, 1, 10, 0), Err(W31A132_ERANGE));
        assert_eq!(w31a132_xtow_s(1, 4, 1, 0), Err(W31A132_EINVAL));
        assert_eq!(w31a132_xtow_s(35, 4, 36, 0).unwrap(), vec![b'z' as u16]);
    }
}
