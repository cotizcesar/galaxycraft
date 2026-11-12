#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent128.rs
// ===========================================================================
// Wave 31 agent 128: CRT string and bounded-format helpers.

/// `0x00c980d0`: CRT `strcspn`.  Builds a 256-bit set from `reject` (up to
/// its NUL) and returns the index of the first byte of `s` (up to its NUL)
/// that is in the set, or the length of `s`.
pub fn w31a128_strcspn(s: &[u8], reject: &[u8]) -> usize {
    let mut set = [0u8; 32];
    for &b in reject.iter().take_while(|&&b| b != 0) {
        set[(b >> 3) as usize] |= 1 << (b & 7);
    }
    let mut i = 0;
    while let Some(&b) = s.get(i) {
        if b == 0 || (set[(b >> 3) as usize] >> (b & 7)) & 1 != 0 {
            break;
        }
        i += 1;
    }
    i
}

/// Result of the bounded-format wrapper: value plus errno that was set.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W31a128Result {
    pub ret: i32,
    pub errno: Option<i32>,
    pub terminate: bool,
}

fn w31a128_bounded(
    buf_len: usize,
    has_buf: bool,
    has_fmt: bool,
    inner: impl FnOnce() -> i32,
) -> W31a128Result {
    if !has_fmt {
        return W31a128Result {
            ret: -1,
            errno: Some(0x16),
            terminate: false,
        };
    }
    if !has_buf || buf_len == 0 {
        return W31a128Result {
            ret: -1,
            errno: Some(0x16),
            terminate: false,
        };
    }
    let r = inner();
    let terminate = r < 0;
    if r != -2 {
        return W31a128Result {
            ret: r,
            errno: None,
            terminate,
        };
    }
    W31a128Result {
        ret: -1,
        errno: Some(0x22),
        terminate,
    }
}

/// `0x00c98b93`: byte-width bounded format wrapper.  Null format gives
/// EINVAL (0x16); null/empty buffer gives EINVAL; otherwise the inner
/// formatter runs, a negative result NUL-terminates the buffer, and -2
/// becomes -1 with ERANGE (0x22).
pub fn w31a128_vsnprintf_s(
    buf_len: usize,
    has_buf: bool,
    has_fmt: bool,
    inner: impl FnOnce() -> i32,
) -> W31a128Result {
    w31a128_bounded(buf_len, has_buf, has_fmt, inner)
}

/// `0x00c98c0b`: wide-character twin of the bounded format wrapper, same
/// error contract.
pub fn w31a128_vsnwprintf_s(
    buf_len: usize,
    has_buf: bool,
    has_fmt: bool,
    inner: impl FnOnce() -> i32,
) -> W31a128Result {
    w31a128_bounded(buf_len, has_buf, has_fmt, inner)
}

/// Outcome of a buffer-size reservation.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W31a128Reserve {
    /// Fits already (inline buffer or capacity sufficient).
    Ok,
    /// Needs allocation of the given byte count.
    Grow(u32),
    /// Overflow: errno ENOMEM (0xc).
    NoMem,
}

fn w31a128_reserve(count: u32, limit: u32, shift: u32, heap: bool, cap: u32) -> W31a128Reserve {
    if count >= limit {
        return W31a128Reserve::NoMem;
    }
    let bytes = count << shift;
    if (!heap && bytes < 0x401) || bytes <= cap {
        W31a128Reserve::Ok
    } else {
        W31a128Reserve::Grow(bytes)
    }
}

/// `0x00c98cb6`: reserve for `count` 16-bit units (limit 0x80000000;
/// inline buffer is 0x400 bytes, `cap` is the field at +0x400).
pub fn w31a128_reserve_wide(count: u32, heap: bool, cap: u32) -> W31a128Reserve {
    w31a128_reserve(count, 0x8000_0000, 1, heap, cap)
}

/// `0x00c98d35`: reserve for `count` 32-bit units (limit 0x40000000).
pub fn w31a128_reserve_dwords(count: u32, heap: bool, cap: u32) -> W31a128Reserve {
    w31a128_reserve(count, 0x4000_0000, 2, heap, cap)
}

#[cfg(test)]
mod tests_wave31_agent128 {
    use super::*;

    #[test]
    fn strcspn() {
        assert_eq!(w31a128_strcspn(b"hello world\0", b"o \0"), 4);
        assert_eq!(w31a128_strcspn(b"abc\0", b"xyz\0"), 3);
        assert_eq!(w31a128_strcspn(b"\0", b"a\0"), 0);
        assert_eq!(w31a128_strcspn(b"\xffa\0", b"\xff\0"), 0);
    }

    #[test]
    fn bounded() {
        assert_eq!(w31a128_vsnprintf_s(4, true, false, || 0).errno, Some(0x16));
        assert_eq!(w31a128_vsnprintf_s(0, true, true, || 0).ret, -1);
        let r = w31a128_vsnprintf_s(4, true, true, || -2);
        assert_eq!((r.ret, r.errno, r.terminate), (-1, Some(0x22), true));
        let r = w31a128_vsnwprintf_s(4, true, true, || 3);
        assert_eq!((r.ret, r.errno, r.terminate), (3, None, false));
        assert!(w31a128_vsnwprintf_s(4, true, true, || -1).terminate);
    }

    #[test]
    fn reserve() {
        assert_eq!(w31a128_reserve_wide(0x200, false, 0), W31a128Reserve::Ok);
        assert_eq!(
            w31a128_reserve_wide(0x201, false, 0),
            W31a128Reserve::Grow(0x402)
        );
        assert_eq!(
            w31a128_reserve_wide(0x8000_0000, false, 0),
            W31a128Reserve::NoMem
        );
        assert_eq!(w31a128_reserve_dwords(0x100, false, 0), W31a128Reserve::Ok);
        assert_eq!(
            w31a128_reserve_dwords(0x100, true, 0x400),
            W31a128Reserve::Ok
        );
        assert_eq!(
            w31a128_reserve_dwords(0x4000_0000, true, 0),
            W31a128Reserve::NoMem
        );
    }
}
