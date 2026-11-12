#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent134.rs
// ===========================================================================
// Wave 31 agent 134: CRT helpers (fd table, strtok core).

/// CRT errno values used by these helpers.
pub const W31A134_EINVAL: i32 = 0x16;
pub const W31A134_EBADF: i32 = 9;
pub const W31A134_EOVERFLOW: i32 = 0x84;

/// One CRT file-descriptor slot (stride `0x38`; handle at `+0x18`, flags at `+0x28`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W31a134FdSlot {
    pub osfhandle: i32,
    pub flags: u8,
}

/// `0x00ca5671`: stores `lo` into `*out` when the high dword is zero and `lo < 0x8000_0000`.
/// On failure returns `Err(EOVERFLOW)` (errno 0x84) with `*out == 0`.
pub fn w31a134_checked_size(hi: i32, lo: u32, out: &mut u32) -> Result<u32, i32> {
    *out = 0;
    if hi == 0 && lo < 0x8000_0000 {
        *out = lo;
        Ok(lo)
    } else {
        Err(W31A134_EOVERFLOW)
    }
}

fn w31a134_std_handle_id(fd: u32) -> Option<u32> {
    match fd {
        0 => Some(0xffff_fff6),
        1 => Some(0xffff_fff5),
        2 => Some(0xffff_fff4),
        _ => None,
    }
}

/// `0x00ca61e0`: `_set_osfhandle`.  Slot must hold -1; std fds 0..2 also
/// notify the OS via `set_std(id, handle)` when `console` is true.
pub fn w31a134_set_osfhandle(
    table: &mut [W31a134FdSlot],
    fd: i32,
    handle: i32,
    console: bool,
    mut set_std: impl FnMut(u32, i32),
) -> i32 {
    if fd >= 0 && (fd as usize) < table.len() && table[fd as usize].osfhandle == -1 {
        if console {
            if let Some(id) = w31a134_std_handle_id(fd as u32) {
                set_std(id, handle);
            }
        }
        table[fd as usize].osfhandle = handle;
        return 0;
    }
    -1
}

/// `0x00ca63a0`: `_free_osfhandle`.  Slot must be open (flag bit 0) with a
/// handle other than -1; resets it to -1.
pub fn w31a134_free_osfhandle(
    table: &mut [W31a134FdSlot],
    fd: i32,
    console: bool,
    mut set_std: impl FnMut(u32, i32),
) -> i32 {
    if fd >= 0 && (fd as usize) < table.len() {
        let s = &mut table[fd as usize];
        if s.flags & 1 != 0 && s.osfhandle != -1 {
            if console {
                if let Some(id) = w31a134_std_handle_id(fd as u32) {
                    set_std(id, 0);
                }
            }
            s.osfhandle = -1;
            return 0;
        }
    }
    -1
}

/// `0x00ca6431`: `_get_osfhandle`; -2 and invalid/closed fds yield -1.
pub fn w31a134_get_osfhandle(table: &[W31a134FdSlot], fd: i32) -> i32 {
    if fd == -2 {
        return -1;
    }
    if fd >= 0 && (fd as usize) < table.len() && table[fd as usize].flags & 1 != 0 {
        return table[fd as usize].osfhandle;
    }
    -1
}

/// `0x00ca6827`: strtok core.  `s` is the buffer (NUL-terminated), `pos` the
/// start offset; returns `(token_start, next_pos)`.  The delimiter byte is
/// overwritten with 0.
pub fn w31a134_strtok_core(s: &mut [u8], mut pos: usize, delims: &[u8]) -> (usize, usize) {
    let mut set = [0u8; 32];
    for &b in delims.iter().chain(std::iter::once(&0u8)) {
        set[(b >> 3) as usize] |= 1 << (b & 7);
        if b == 0 {
            break;
        }
    }
    let inset = |b: u8| set[(b >> 3) as usize] & (1 << (b & 7)) != 0;
    while inset(s[pos]) && s[pos] != 0 {
        pos += 1;
    }
    let start = pos;
    loop {
        let b = s[pos];
        if b == 0 {
            return (start, pos);
        }
        if inset(b) {
            s[pos] = 0;
            return (start, pos + 1);
        }
        pos += 1;
    }
}

/// `0x00ca6915`: strtok argument validation; `None` is EINVAL.
pub fn w31a134_strtok_checked(
    s: &mut [u8],
    start: Option<usize>,
    saved: usize,
    delims: Option<&[u8]>,
) -> Result<(usize, usize), i32> {
    let delims = delims.ok_or(W31A134_EINVAL)?;
    let pos = match start {
        Some(p) => p,
        None if saved != 0 => saved,
        None => return Err(W31A134_EINVAL),
    };
    Ok(w31a134_strtok_core(s, pos, delims))
}

/// `0x00ca6ef1`: returns 0 when `ok`, else -1 with EINVAL.
pub fn w31a134_status_to_result(ok: bool) -> i32 {
    if ok { 0 } else { -1 }
}

#[cfg(test)]
mod tests_wave31_agent134 {
    use super::*;

    #[test]
    fn size_and_status() {
        let mut o = 5;
        assert_eq!(
            w31a134_checked_size(0, 0x7fff_ffff, &mut o),
            Ok(0x7fff_ffff)
        );
        assert_eq!(w31a134_checked_size(0, 0x8000_0000, &mut o), Err(0x84));
        assert_eq!(o, 0);
        assert_eq!(w31a134_checked_size(1, 3, &mut o), Err(0x84));
        assert_eq!(w31a134_status_to_result(true), 0);
        assert_eq!(w31a134_status_to_result(false), -1);
    }

    #[test]
    fn fd_table() {
        let mut t = [W31a134FdSlot {
            osfhandle: -1,
            flags: 0,
        }; 4];
        let mut calls = vec![];
        assert_eq!(
            w31a134_set_osfhandle(&mut t, 1, 77, true, |a, b| calls.push((a, b))),
            0
        );
        assert_eq!(calls, vec![(0xffff_fff5, 77)]);
        assert_eq!(w31a134_set_osfhandle(&mut t, 1, 78, true, |_, _| {}), -1);
        assert_eq!(w31a134_get_osfhandle(&t, 1), -1);
        t[1].flags = 1;
        assert_eq!(w31a134_get_osfhandle(&t, 1), 77);
        assert_eq!(w31a134_get_osfhandle(&t, -2), -1);
        assert_eq!(
            w31a134_free_osfhandle(&mut t, 1, true, |a, b| calls.push((a, b))),
            0
        );
        assert_eq!(calls[1], (0xffff_fff5, 0));
        assert_eq!(t[1].osfhandle, -1);
        assert_eq!(w31a134_free_osfhandle(&mut t, 9, false, |_, _| {}), -1);
    }

    #[test]
    fn strtok() {
        let mut s = *b"  ab,cd\0";
        let (st, nx) = w31a134_strtok_core(&mut s, 0, b" ,");
        assert_eq!((st, nx), (2, 5));
        assert_eq!(&s[2..5], b"ab\0");
        let r = w31a134_strtok_checked(&mut s, None, nx, Some(b","));
        assert_eq!(r, Ok((5, 7)));
        assert_eq!(
            w31a134_strtok_checked(&mut s, None, 0, Some(b",")),
            Err(0x16)
        );
        assert_eq!(w31a134_strtok_checked(&mut s, Some(0), 0, None), Err(0x16));
    }
}
