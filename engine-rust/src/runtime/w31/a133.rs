#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent133.rs
// ===========================================================================
// Wave 31 translation batch 133: CRT stdio / string helpers.

/// `0x00ca38fe`: forwards to the formatter with `flag = 1` only when
/// `radix == 10 && value < 0`; returns `buffer` unchanged. Returns
/// `(buffer, is_signed_negative_flag)`.
pub fn w31a133_itoa_flag(value: i32, buffer: u32, radix: i32) -> (u32, u32) {
    let flag = if radix == 10 && value < 0 { 1 } else { 0 };
    (buffer, flag)
}

/// `0x00ca3c13`: bounded-copy status wrapper. `copy` performs the inner
/// formatted write (`0xca_396a` equivalent) returning the count or -1.
/// Returns `(errno-style status, out_len)`.
pub fn w31a133_bounded_format(
    dst: Option<&mut [u8]>,
    dst_size: u32,
    count: u32,
    copy: impl FnOnce(Option<&mut [u8]>, u32) -> i32,
) -> (u32, Option<u32>) {
    match (&dst, dst_size) {
        (None, n) if n != 0 => return (0x16, None),
        (Some(_), 0) => return (0x16, None),
        _ => {}
    }
    let mut dst = dst;
    if let Some(d) = dst.as_mut() {
        if let Some(b) = d.first_mut() {
            *b = 0;
        }
    }
    let limit = dst_size.min(count);
    if limit >= 0x8000_0000 {
        return (0x16, Some(0));
    }
    let has_dst = dst.is_some();
    let n = copy(dst.as_deref_mut(), limit);
    if n == -1 {
        if let Some(b) = dst.as_deref_mut().and_then(|d| d.first_mut()) {
            *b = 0;
        }
        return (0, Some(0));
    }
    let mut len = n as u32 + 1;
    let mut status = 0;
    if has_dst {
        if dst_size < len {
            if count != 0xffff_ffff {
                if let Some(b) = dst.as_deref_mut().and_then(|d| d.first_mut()) {
                    *b = 0;
                }
                if dst_size <= len {
                    return (0x22, Some(0));
                }
            }
            status = 0x50;
            len = dst_size;
        }
        if let Some(d) = dst.as_mut() {
            if let Some(b) = d.get_mut(len as usize - 1) {
                *b = 0;
            }
        }
    }
    (status, Some(len))
}

/// `0x00ca3fac`: sign-extends a 32-bit offset to 64 bits for the 64-bit
/// seek core; returns `(low, high)`.
pub fn w31a133_sign_extend_offset(offset: i32) -> (u32, u32) {
    (offset as u32, (offset >> 31) as u32)
}

/// `0x00ca42c6`: 32-bit tell; a 64-bit position above `i32::MAX` yields
/// `u32::MAX` with errno `EINVAL` (0x16).  Returns `(result, errno)`.
pub fn w31a133_tell32(pos64: i64) -> (u32, Option<u32>) {
    if pos64 > 0x7fff_ffff {
        (0xffff_ffff, Some(0x16))
    } else {
        (pos64 as u32, None)
    }
}

/// `0x00ca47d6`: counts newline characters in `[begin, end)` of a text
/// buffer. `mode` 1 or 2 means UTF-16 units and the result is doubled
/// (bytes added by CRLF expansion in wide streams); otherwise bytes.
pub fn w31a133_count_newlines(bytes: &[u8], mode: u8) -> u64 {
    if mode == 1 || mode == 2 {
        let n = bytes
            .chunks_exact(2)
            .filter(|c| u16::from_le_bytes([c[0], c[1]]) == 10)
            .count() as u64;
        n * 2
    } else {
        bytes.iter().filter(|&&b| b == b'\n').count() as u64
    }
}

/// `0x00ca4854`: thin wrapper over the 64-bit tell core; returns its value.
pub fn w31a133_tell_wrapper(tell64: impl FnOnce() -> i64) -> i64 {
    tell64()
}

/// `0x00ca5092`: pure forwarder to the 64-bit seek core with identical args.
pub fn w31a133_seek_forward<T>(
    core: impl FnOnce(u32, u32, u32, u32) -> T,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
) -> T {
    core(a, b, c, d)
}

/// `0x00ca5611`: two-pass size query with errno preservation. `query`
/// returns `(result, errno)`. If the first call fails with errno 0x22 the
/// saved errno is restored and the query reruns with null buffer/zero.
pub fn w31a133_size_query_retry(
    saved_errno: i32,
    mut query: impl FnMut(bool) -> (i32, i32),
) -> (i32, i32, Option<i32>) {
    let (r, e) = query(false);
    if r != 0 {
        return (r, saved_errno, None);
    }
    if e == 0x22 {
        let (r2, _) = query(true);
        return (r2, saved_errno, Some(r2));
    }
    (0, e, None)
}

/// `0x00ca4ab9`: validates a CRT fd: -2 or out of range / unopened sets
/// errno 9 (EBADF) and returns -1; otherwise runs `op`.
pub fn w31a133_fd_checked(
    fd: u32,
    count: u32,
    opened: impl Fn(u32) -> bool,
    op: impl FnOnce(u32) -> u32,
) -> (u32, Option<u32>) {
    if fd == 0xffff_fffe || (fd as i32) < 0 || fd >= count || !opened(fd) {
        return (0xffff_ffff, Some(9));
    }
    (op(fd), None)
}

#[cfg(test)]
mod tests_wave31_agent133 {
    use super::*;

    #[test]
    fn small_helpers() {
        assert_eq!(w31a133_itoa_flag(-1, 5, 10), (5, 1));
        assert_eq!(w31a133_itoa_flag(-1, 5, 16), (5, 0));
        assert_eq!(w31a133_itoa_flag(1, 5, 10), (5, 0));
        assert_eq!(w31a133_sign_extend_offset(-2), (0xffff_fffe, 0xffff_ffff));
        assert_eq!(w31a133_tell32(0x8000_0000), (0xffff_ffff, Some(0x16)));
        assert_eq!(w31a133_tell32(7), (7, None));
        assert_eq!(w31a133_tell_wrapper(|| 9), 9);
        assert_eq!(
            w31a133_seek_forward(|a, b, c, d| a + b + c + d, 1, 2, 3, 4),
            10
        );
    }

    #[test]
    fn newlines() {
        assert_eq!(w31a133_count_newlines(b"a\nb\n", 0), 2);
        assert_eq!(w31a133_count_newlines(&[10, 0, 1, 0, 10, 0], 1), 4);
    }

    #[test]
    fn bounded() {
        let mut buf = [9u8; 4];
        let r = w31a133_bounded_format(Some(&mut buf), 4, 0xffff_ffff, |_, _| 2);
        assert_eq!(r, (0, Some(3)));
        assert_eq!(buf[2], 0);
        assert_eq!(w31a133_bounded_format(None, 1, 0, |_, _| 0).0, 0x16);
        let mut b2 = [0u8; 2];
        assert_eq!(
            w31a133_bounded_format(Some(&mut b2), 2, 0xffff_ffff, |_, _| 5),
            (0x50, Some(2))
        );
        let mut b3 = [0u8; 2];
        assert_eq!(
            w31a133_bounded_format(Some(&mut b3), 2, 4, |_, _| 5).0,
            0x22
        );
    }

    #[test]
    fn retry_and_fd() {
        let mut n = 0;
        let r = w31a133_size_query_retry(3, |_| {
            n += 1;
            if n == 1 { (0, 0x22) } else { (8, 0) }
        });
        assert_eq!(r, (8, 3, Some(8)));
        assert_eq!(w31a133_size_query_retry(3, |_| (4, 0)), (4, 3, None));
        assert_eq!(
            w31a133_fd_checked(0xffff_fffe, 4, |_| true, |f| f),
            (0xffff_ffff, Some(9))
        );
        assert_eq!(w31a133_fd_checked(2, 4, |_| true, |f| f + 1), (3, None));
        assert_eq!(
            w31a133_fd_checked(5, 4, |_| true, |f| f),
            (0xffff_ffff, Some(9))
        );
    }
}
