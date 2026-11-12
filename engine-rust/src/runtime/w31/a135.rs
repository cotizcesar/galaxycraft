#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent135.rs
// ===========================================================================
// Wave 31 agent 135: CRT low-level I/O flag decoding and bounded string copy helpers.

/// Result of the CRT open-flag decoder (layout of the open-info record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W31a135OpenInfo {
    /// byte at +0x00: bit 0x10 append, 0x80 text mode
    pub flags: u8,
    /// +0x04: desired access
    pub access: u32,
    /// +0x08: share mode
    pub share: u32,
    /// +0x0c: creation disposition
    pub disposition: u32,
    /// +0x10: file attributes
    pub attributes: u32,
    /// +0x14: flags-and-attributes
    pub flags_and_attrs: u32,
}

/// `0x00ca7315`: maps the share-mode bits (`oflag & 0x700`) to a Win32 share
/// value; `None` means invalid argument (errno 0x16, returns -1).
pub fn w31a135_share_mode(oflag: u32) -> Option<u32> {
    match oflag & 0x700 {
        0x000 | 0x400 => Some(3),
        0x100 => Some(4),
        0x300 => Some(2),
        0x200 | 0x600 => Some(5),
        0x500 | 0x700 => Some(1),
        _ => None,
    }
}

/// `0x00ca7385`: decodes open flags into an open-info record.  `query_mode`
/// stands in for the default-mode query (`None` = it failed and the original
/// never returns); `umask_byte` is the global permission mask byte.
pub fn w31a135_decode_open(
    oflag: u32,
    creation: i32,
    perm: u8,
    umask_byte: u8,
    query_mode: impl FnOnce() -> Option<u32>,
) -> Option<W31a135OpenInfo> {
    let mut r = W31a135OpenInfo::default();
    r.access = match oflag & 3 {
        0 => 0x8000_0000,
        1 => {
            let hi = if (oflag & 0x7_0000) != 0 && (oflag & 8) != 0 {
                0x8000_0000u32
            } else {
                0
            };
            hi.wrapping_add(0x4000_0000)
        }
        2 => 0xc000_0000,
        _ => 0xffff_ffff,
    };
    r.share = w31a135_share_mode(oflag).unwrap_or(0xffff_ffff);
    r.disposition = match creation {
        0x10 => 0,
        0x20 => 1,
        0x30 => 2,
        0x40 => 3,
        0x80 => (r.access == 0x8000_0000) as u32,
        _ => 0xffff_ffff,
    };
    r.attributes = 0x80;
    if (oflag as u8) & 0x80 != 0 {
        r.flags |= 0x10;
    }
    if oflag & 0x8000 == 0 {
        if oflag & 0x7_4000 == 0 && query_mode()? == 0x8000 {
            // binary default: text flag not set
        } else {
            r.flags |= 0x80;
        }
    }
    if oflag & 0x100 != 0 && (!umask_byte & perm) & 0x80 == 0 {
        r.attributes = 1;
    }
    if oflag & 0x40 != 0 {
        r.flags_and_attrs |= 0x400_0000;
        r.access |= 0x1_0000;
        r.disposition |= 4;
    }
    if oflag & 0x1000 != 0 {
        r.attributes |= 0x100;
    }
    if oflag & 0x2000 != 0 {
        r.flags_and_attrs |= 0x200_0000;
    }
    if oflag & 0x20 != 0 {
        r.flags_and_attrs |= 0x800_0000;
    } else if oflag & 0x10 != 0 {
        r.flags_and_attrs |= 0x1000_0000;
    }
    Some(r)
}

/// `0x00ca92c2`: bounded narrow string copy (`strcpy_s`).  Returns
/// `(code, errno_set)`: 0 on success, 0x16 invalid argument, 0x22 range.
pub fn w31a135_strcpy_s(
    dst: Option<&mut [u8]>,
    size: usize,
    src: Option<&[u8]>,
) -> (u32, Option<u32>) {
    if let Some(d) = dst {
        let size = size.min(d.len());
        if size != 0 {
            if let Some(s) = src {
                let mut remaining = size;
                let mut i = 0;
                loop {
                    let c = s.get(i).copied().unwrap_or(0);
                    d[i] = c;
                    i += 1;
                    if c == 0 {
                        break;
                    }
                    remaining -= 1;
                    if remaining == 0 {
                        break;
                    }
                }
                if remaining != 0 {
                    return (0, None);
                }
                d[0] = 0;
                return (0x22, Some(0x22));
            }
            d[0] = 0;
        }
    }
    (0x16, Some(0x16))
}

/// `0x00ca91ce`: bounded wide string concatenation (`wcsncat_s`).  `count`
/// of -1 is `_TRUNCATE` (returns 0x50 without errno on overflow).
pub fn w31a135_wcsncat_s(
    dst: Option<&mut [u16]>,
    size: usize,
    src: Option<&[u16]>,
    count: i32,
) -> (u32, Option<u32>) {
    let invalid = (0x16, Some(0x16));
    let Some(d) = dst else {
        return if count == 0 && size == 0 {
            (0, None)
        } else {
            invalid
        };
    };
    if size == 0 {
        return invalid;
    }
    let size = size.min(d.len());
    if !(count == 0 || src.is_some()) {
        d[0] = 0;
        return invalid;
    }
    let mut p = 0usize;
    let mut remaining = size;
    while d[p] != 0 {
        p += 1;
        remaining -= 1;
        if remaining == 0 {
            break;
        }
    }
    if remaining == 0 {
        d[0] = 0;
        return invalid;
    }
    let s = src.unwrap_or(&[]);
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let mut si = 0usize;
    if count == -1 {
        loop {
            let c = at(si);
            d[p] = c;
            p += 1;
            si += 1;
            if c == 0 {
                break;
            }
            remaining -= 1;
            if remaining == 0 {
                break;
            }
        }
    } else {
        let mut n = count;
        while n != 0 {
            let c = at(si);
            si += 1;
            d[p] = c;
            p += 1;
            if c == 0 {
                break;
            }
            remaining -= 1;
            if remaining == 0 {
                break;
            }
            n -= 1;
        }
        if n == 0 {
            d[p] = 0;
        }
    }
    if remaining != 0 {
        return (0, None);
    }
    if count == -1 {
        d[size - 1] = 0;
        return (0x50, None);
    }
    d[0] = 0;
    (0x22, Some(0x22))
}

#[cfg(test)]
mod tests_wave31_agent135 {
    use super::*;

    #[test]
    fn share_modes() {
        assert_eq!(w31a135_share_mode(0), Some(3));
        assert_eq!(w31a135_share_mode(0x100), Some(4));
        assert_eq!(w31a135_share_mode(0x300), Some(2));
        assert_eq!(w31a135_share_mode(0x600), Some(5));
        assert_eq!(w31a135_share_mode(0x700), Some(1));
        assert_eq!(w31a135_share_mode(0x1ff), Some(4));
    }

    #[test]
    fn decode_open_basic() {
        let r = w31a135_decode_open(0, 0x20, 0, 0, || Some(0x4000)).unwrap();
        assert_eq!(r.access, 0x8000_0000);
        assert_eq!(r.disposition, 1);
        assert_eq!(r.flags, 0x80);
        let r = w31a135_decode_open(2 | 0x8000 | 0x40, 0x80, 0, 0, || None).unwrap();
        assert_eq!(r.access, 0xc001_0000);
        assert_eq!(r.disposition, 4);
        assert_eq!(r.flags_and_attrs, 0x400_0000);
        assert!(w31a135_decode_open(0, 0x10, 0, 0, || None).is_none());
        let r = w31a135_decode_open(0x8000 | 0x100, 0x10, 0, 0, || None).unwrap();
        assert_eq!(r.attributes, 1);
    }

    #[test]
    fn strcpy() {
        let mut b = [9u8; 4];
        assert_eq!(w31a135_strcpy_s(Some(&mut b), 4, Some(b"ab\0")), (0, None));
        assert_eq!(&b[..3], b"ab\0");
        assert_eq!(
            w31a135_strcpy_s(Some(&mut b), 2, Some(b"abc\0")),
            (0x22, Some(0x22))
        );
        assert_eq!(b[0], 0);
        assert_eq!(w31a135_strcpy_s(None, 2, Some(b"a\0")), (0x16, Some(0x16)));
        assert_eq!(w31a135_strcpy_s(Some(&mut b), 4, None), (0x16, Some(0x16)));
    }

    #[test]
    fn wcsncat() {
        let mut d = [0u16; 8];
        d[0] = 65;
        let s = [66u16, 67, 0];
        assert_eq!(w31a135_wcsncat_s(Some(&mut d), 8, Some(&s), 2), (0, None));
        assert_eq!(&d[..4], &[65, 66, 67, 0]);
        let mut d = [0u16; 3];
        d[0] = 65;
        assert_eq!(
            w31a135_wcsncat_s(Some(&mut d), 3, Some(&s), -1),
            (0x50, None)
        );
        assert_eq!(d[2], 0);
        let mut d = [0u16; 3];
        d[0] = 65;
        assert_eq!(
            w31a135_wcsncat_s(Some(&mut d), 3, Some(&s), 2),
            (0x22, Some(0x22))
        );
        assert_eq!(w31a135_wcsncat_s(None, 0, None, 0), (0, None));
        assert_eq!(w31a135_wcsncat_s(None, 3, None, 0), (0x16, Some(0x16)));
    }
}
