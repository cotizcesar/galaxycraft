#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent140.rs
// ===========================================================================
// Wave 31 agent 140: CRT helpers (fd flag lookup, recalloc size check,
// UTF-8 to UTF-16 conversion loop, default C ctype table).

/// `0x00cb2ae2`: returns the `0x40` flag bit of the fd entry (stride `0x38`,
/// 64 entries per block, flag byte at `+0x28`), or 0 when out of range.
pub fn w31a140_fd_flag40(blocks: &[Vec<u8>], fd: u32, fd_count: u32) -> u8 {
    if fd == 0xffff_fffe || (fd as i32) < 0 || fd >= fd_count {
        return 0;
    }
    let Some(block) = blocks.get((fd >> 6) as usize) else {
        return 0;
    };
    let off = 0x28 + (fd & 0x3f) as usize * 0x38;
    block.get(off).map_or(0, |b| b & 0x40)
}

/// `0x00cb55ad`: recalloc size check.  Returns `None` on overflow (errno
/// 0xc), otherwise `(new_size, zero_from)` where the tail
/// `[old_size..new_size)` must be zeroed when `old_size < new_size`.
pub fn w31a140_recalloc_sizes(
    has_ptr: bool,
    count: u32,
    elem: u32,
    old_size: u32,
) -> Option<(u32, Option<u32>)> {
    if count != 0 && elem > 0xffff_ffe0 / count {
        return None;
    }
    let old = if has_ptr { old_size } else { 0 };
    let new = count.wrapping_mul(elem);
    Some((new, if old < new { Some(old) } else { None }))
}

/// `0x00cb2ba6`: multibyte to UTF-16 loop.  `decode(pos)` returns
/// `(consumed, code_point)`, `consumed == -1` meaning invalid.  With
/// `out == None` it counts UTF-16 units.  Returns `(units, new_pos)`; units
/// is -1 on error.
pub fn w31a140_to_utf16(
    mut out: Option<&mut [u16]>,
    mut pos: usize,
    mut room: usize,
    decode: &mut dyn FnMut(usize) -> (i32, u32),
) -> (i32, Option<usize>) {
    let Some(buf) = out.as_mut() else {
        let mut n = 0i32;
        loop {
            let (c, _) = decode(pos);
            if c == -1 {
                return (-1, None);
            }
            if c == 0 {
                return (n, None);
            }
            if c == 4 {
                n += 1;
            }
            n += 1;
            pos += c as usize;
        }
    };
    let mut w = 0usize;
    while room != 0 {
        let (c, mut cp) = decode(pos);
        if c == -1 {
            return (-1, Some(pos));
        }
        if c == 0 {
            buf[w] = 0;
            return (w as i32, Some(0));
        }
        if cp > 0xffff {
            if room < 2 {
                break;
            }
            cp -= 0x10000;
            room -= 1;
            buf[w] = (cp >> 10) as u16 | 0xd800;
            w += 1;
            cp = (cp & 0x3ff) | 0xdc00;
        }
        buf[w] = cp as u16;
        pos += c as usize;
        w += 1;
        room -= 1;
    }
    (w as i32, Some(pos))
}

/// `0x00cb4b76` (fallback branch): default ctype table.  Fills `flags`
/// (0x100 bytes, record `+0x19`) with 0x10 upper / 0x20 lower and `map`
/// (record `+0x119`) with the case-swapped letter.
pub fn w31a140_default_ctype(flags: &mut [u8; 0x100], map: &mut [u8; 0x100]) {
    for i in 0..0x100usize {
        let u = i as u32;
        map[i] = if u.wrapping_sub(0x41) < 0x1a {
            flags[i] |= 0x10;
            (i as u8) + 0x20
        } else if u.wrapping_sub(0x61) < 0x1a {
            flags[i] |= 0x20;
            (i as u8) - 0x20
        } else {
            0
        };
    }
}

#[cfg(test)]
mod tests_wave31_agent140 {
    use super::*;

    #[test]
    fn fd_flag() {
        let mut b = vec![0u8; 0x38 * 64];
        b[0x28 + 3 * 0x38] = 0xff;
        let blocks = vec![b];
        assert_eq!(w31a140_fd_flag40(&blocks, 3, 10), 0x40);
        assert_eq!(w31a140_fd_flag40(&blocks, 10, 10), 0);
        assert_eq!(w31a140_fd_flag40(&blocks, 0xffff_fffe, 10), 0);
        assert_eq!(w31a140_fd_flag40(&blocks, u32::MAX, 10), 0);
    }

    #[test]
    fn recalloc() {
        assert_eq!(w31a140_recalloc_sizes(true, 4, 4, 8), Some((16, Some(8))));
        assert_eq!(w31a140_recalloc_sizes(false, 4, 4, 8), Some((16, Some(0))));
        assert_eq!(w31a140_recalloc_sizes(true, 2, 2, 8), Some((4, None)));
        assert_eq!(w31a140_recalloc_sizes(true, 0, 5, 8), Some((0, None)));
        assert_eq!(w31a140_recalloc_sizes(true, 2, 0x8000_0000, 0), None);
    }

    #[test]
    fn utf16() {
        let src = [0x41u32, 0x1F600, 0, 0, 0, 0];
        let lens = [1, 4, 0, 0, 0, 0];
        let mut d = |p: usize| {
            if p == 0 {
                (lens[0], src[0])
            } else if p == 1 {
                (lens[1], src[1])
            } else {
                (0, 0)
            }
        };
        let mut out = [0u16; 4];
        let (n, _) = w31a140_to_utf16(Some(&mut out), 0, 4, &mut d);
        assert_eq!(n, 3);
        assert_eq!(out[..3], [0x41, 0xd83d, 0xde00]);
    }

    #[test]
    fn ctype() {
        let mut f = [0u8; 256];
        let mut m = [0u8; 256];
        w31a140_default_ctype(&mut f, &mut m);
        assert_eq!((f[b'A' as usize], m[b'A' as usize]), (0x10, b'a'));
        assert_eq!((f[b'z' as usize], m[b'z' as usize]), (0x20, b'Z'));
        assert_eq!((f[0], m[0x80]), (0, 0));
    }
}
