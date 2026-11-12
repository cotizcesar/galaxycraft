#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent129.rs
// ===========================================================================
// Wave 31 agent 129: UCRT printf-engine helper bodies (format state machine).

/// Output state shared by the printf writers (`0x00c9b65f`, `0x00c9b6c6`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct W31a129Out {
    pub buf_limit: u32,
    pub buf_pos: u32,
    pub is_counting: bool,
    pub written: Vec<u32>,
}

/// Result of a unit write: how many elements the buffered path copies.
fn w31a129_write_core(out: &W31a129Out, n: u32, count: &mut i32) -> Option<u32> {
    if n == 0 {
        return None;
    }
    if out.buf_pos != out.buf_limit {
        let room = out.buf_limit.wrapping_sub(out.buf_pos);
        return Some(n.min(room));
    }
    if !out.is_counting {
        *count = -1;
    } else {
        *count = count.wrapping_add(n as i32);
    }
    None
}

/// `0x00c9b65f`: narrow write; returns elements to copy when a buffer has room.
pub fn w31a129_write_narrow(out: &W31a129Out, n: u32, count: &mut i32) -> Option<u32> {
    w31a129_write_core(out, n, count)
}

/// `0x00c9b6c6`: wide write; returns the byte length (2 per element) to copy.
pub fn w31a129_write_wide(out: &W31a129Out, n: u32, count: &mut i32) -> Option<u32> {
    w31a129_write_core(out, n, count).map(|m| m.wrapping_mul(2))
}

/// `0x00c9b2e6`: state must be 0 or 7, else EINVAL (0x16) and false.
pub fn w31a129_state_ok(state: i32, errno: &mut i32) -> bool {
    if state != 0 && state != 7 {
        *errno = 0x16;
        return false;
    }
    true
}

/// `0x00c9b2c5`: null stream guard, then delegates (`0x00c9b308` model).
pub fn w31a129_stream_guard(
    stream: Option<u32>,
    errno: &mut i32,
    check: impl FnOnce(u32, &mut i32) -> bool,
) -> bool {
    match stream {
        None => {
            *errno = 0x16;
            false
        }
        Some(s) => check(s, errno),
    }
}

/// `0x00c9b308`: bit 12 of stream flags short-circuits to true; otherwise
/// the fd info bytes (`b29`, `b2d`) must satisfy `b29 == 0 && b2d & 1 == 0`.
pub fn w31a129_stream_check(flags: u32, b29: u8, b2d: u8, errno: &mut i32) -> bool {
    if (flags >> 12) & 1 != 0 {
        return true;
    }
    if b29 == 0 && (b2d & 1) == 0 {
        return true;
    }
    *errno = 0x16;
    false
}

/// `0x00c9b0b8`: `%n` store.  Writes the character count at width 1/2/4/8.
pub fn w31a129_store_count(size: u32, count: i32, dest: &mut [u8], errno: &mut i32) -> bool {
    let n = match size {
        1 => 1,
        2 => 2,
        4 => 4,
        8 => 8,
        _ => {
            *errno = 0x16;
            return false;
        }
    };
    if dest.len() < n {
        *errno = 0x16;
        return false;
    }
    let v = count as i64 as u64;
    dest[..n].copy_from_slice(&v.to_le_bytes()[..n]);
    if size == 8 {
        dest[4..8].copy_from_slice(&((count >> 31) as u32).to_le_bytes());
    }
    true
}

/// `0x00c9b739`: counts characters, tracking lead-byte validity.
pub fn w31a129_count_chars(
    s: &[u8],
    errno: &mut i32,
    count: &mut i32,
    is_valid: impl Fn(u8) -> bool,
) {
    let saved = *errno;
    *errno = 0;
    for &c in s {
        if is_valid(c) {
            *count = count.wrapping_add(1);
        } else {
            if *errno != 0x2a {
                *count = -1;
                break;
            }
            if is_valid(0x3f) {
                *count = count.wrapping_add(1);
            } else {
                *count = -1;
            }
        }
    }
    if *errno == 0 && saved != 0 {
        *errno = saved;
    }
}

/// Integer argument after fetch/sign handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W31a129Int {
    pub lo: u32,
    pub hi: u32,
    pub flags: u32,
    pub precision: i32,
    pub clamp_precision: bool,
    pub zero_flag_cleared: bool,
}

fn w31a129_fetch(
    size: u32,
    flags: u32,
    precision: i32,
    slot: u64,
    errno: &mut i32,
) -> Option<W31a129Int> {
    let signed = (flags >> 4) & 1 != 0;
    let (mut lo, mut hi): (u32, i32);
    match size {
        1 => {
            lo = if signed {
                slot as u8 as i8 as i32 as u32
            } else {
                slot as u8 as u32
            };
            hi = (lo as i32) >> 31;
        }
        2 => {
            lo = if signed {
                slot as u16 as i16 as i32 as u32
            } else {
                slot as u16 as u32
            };
            hi = (lo as i32) >> 31;
        }
        4 => {
            lo = slot as u32;
            hi = if signed { (lo as i32) >> 31 } else { 0 };
        }
        8 => {
            lo = slot as u32;
            hi = (slot >> 32) as i32;
        }
        _ => {
            *errno = 0x16;
            return None;
        }
    }
    let mut f = flags;
    if signed && hi < 0 {
        let b = lo != 0;
        lo = lo.wrapping_neg();
        hi = hi.wrapping_add(b as i32).wrapping_neg();
        f |= 0x40;
    }
    let mut prec = precision;
    let clamp = precision >= 0;
    if precision < 0 {
        prec = 1;
    } else {
        f &= !8;
    }
    let zero = lo == 0 && hi == 0;
    if zero {
        f &= !0x20;
    }
    Some(W31a129Int {
        lo,
        hi: hi as u32,
        flags: f,
        precision: prec,
        clamp_precision: clamp,
        zero_flag_cleared: zero,
    })
}

/// `0x00c9ae0e`: narrow integer argument prepare (digits written by caller).
pub fn w31a129_prepare_int_narrow(
    size: u32,
    flags: u32,
    precision: i32,
    slot: u64,
    errno: &mut i32,
) -> Option<W31a129Int> {
    w31a129_fetch(size, flags, precision, slot, errno)
}

/// `0x00c9af61`: wide integer argument prepare (digits written by caller).
pub fn w31a129_prepare_int_wide(
    size: u32,
    flags: u32,
    precision: i32,
    slot: u64,
    errno: &mut i32,
) -> Option<W31a129Int> {
    w31a129_fetch(size, flags, precision, slot, errno)
}

#[cfg(test)]
mod tests_wave31_agent129 {
    use super::*;

    #[test]
    fn writers() {
        let mut c = 0;
        let o = W31a129Out {
            buf_limit: 10,
            buf_pos: 4,
            is_counting: true,
            written: vec![],
        };
        assert_eq!(w31a129_write_narrow(&o, 20, &mut c), Some(6));
        assert_eq!(w31a129_write_wide(&o, 3, &mut c), Some(6));
        let o2 = W31a129Out {
            buf_limit: 4,
            buf_pos: 4,
            is_counting: true,
            written: vec![],
        };
        w31a129_write_narrow(&o2, 5, &mut c);
        assert_eq!(c, 5);
        let o3 = W31a129Out {
            is_counting: false,
            ..o2
        };
        w31a129_write_narrow(&o3, 5, &mut c);
        assert_eq!(c, -1);
    }

    #[test]
    fn state_and_stream() {
        let mut e = 0;
        assert!(w31a129_state_ok(7, &mut e));
        assert!(!w31a129_state_ok(3, &mut e));
        assert_eq!(e, 0x16);
        e = 0;
        assert!(!w31a129_stream_guard(None, &mut e, |_, _| true));
        assert_eq!(e, 0x16);
        assert!(w31a129_stream_check(0x1000, 1, 1, &mut e));
        assert!(w31a129_stream_check(0, 0, 0, &mut e));
        assert!(!w31a129_stream_check(0, 1, 0, &mut e));
    }

    #[test]
    fn store_and_count() {
        let mut e = 0;
        let mut d = [0u8; 8];
        assert!(w31a129_store_count(2, 0x1234, &mut d, &mut e));
        assert_eq!(&d[..2], &[0x34, 0x12]);
        assert!(w31a129_store_count(8, -1, &mut d, &mut e));
        assert_eq!(d, [0xff; 8]);
        assert!(!w31a129_store_count(3, 1, &mut d, &mut e));
        let mut cnt = 0;
        e = 5;
        w31a129_count_chars(b"abc", &mut e, &mut cnt, |_| true);
        assert_eq!((cnt, e), (3, 5));
        cnt = 0;
        w31a129_count_chars(b"a", &mut e, &mut cnt, |_| false);
        assert_eq!(cnt, -1);
    }

    #[test]
    fn prepare() {
        let mut e = 0;
        let r = w31a129_prepare_int_narrow(4, 0x10, -1, 0xffff_fffe, &mut e).unwrap();
        assert_eq!((r.lo, r.hi, r.flags & 0x40, r.precision), (2, 0, 0x40, 1));
        let r = w31a129_prepare_int_wide(1, 0x30, 3, 0, &mut e).unwrap();
        assert!(r.zero_flag_cleared && r.flags & 0x20 == 0 && r.flags & 8 == 0);
        assert!(w31a129_prepare_int_wide(3, 0, 0, 0, &mut e).is_none());
    }
}
