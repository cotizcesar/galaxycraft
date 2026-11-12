#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent118.rs
// ===========================================================================
// Wave 31 agent 118: file-handle helpers (chunked I/O, open disposition).

/// `0x00c6d470` (open-mode part): maps the open flags to a Win32-style
/// creation disposition (0x400 -> 5, 0 -> 3, 0x200 -> 4, 0x600 -> 2,
/// 0xa00 -> 1, otherwise 0).
pub fn w31a118_open_disposition(flags: i32) -> u32 {
    match flags {
        0x400 => 5,
        0 => 3,
        0x200 => 4,
        0x600 => 2,
        0xa00 => 1,
        _ => 0,
    }
}

fn w31a118_chunked(len: u32, mut io: impl FnMut(u32, u32) -> Option<u32>) -> Result<u32, ()> {
    let mut done: u32 = 0;
    let mut left = len;
    while left != 0 {
        let want = left.min(0x4000_0000);
        let got = io(done, want).ok_or(())?;
        left = left.wrapping_sub(got);
        done = done.wrapping_add(got);
        if got != want {
            break;
        }
    }
    Ok(done)
}

/// `0x00c6d600`: loops a read primitive in chunks of at most 0x40000000
/// bytes until `len` is satisfied or a short read occurs; returns total bytes
/// or `Err` when the primitive fails (retail raises an error).  The closure
/// gets (offset, chunk) and returns the bytes transferred.
pub fn w31a118_read_chunked(len: u32, io: impl FnMut(u32, u32) -> Option<u32>) -> Result<u32, ()> {
    w31a118_chunked(len, io)
}

/// `0x00c6d710`: same chunk loop as the reader, over the write primitive.
pub fn w31a118_write_chunked(len: u32, io: impl FnMut(u32, u32) -> Option<u32>) -> Result<u32, ()> {
    w31a118_chunked(len, io)
}

/// `0x00c6e650`: negative callee results collapse to -1.
pub fn w31a118_clamp_neg(r: i32) -> i32 {
    if r < 0 { -1 } else { r }
}

/// `0x00c6ef50`: negative result collapses to -1 (selector 0x10).
pub fn w31a118_ef50(r: i32) -> i32 {
    w31a118_clamp_neg(r)
}

/// `0x00c6ef80`: negative result collapses to -1 (selector 0x28).
pub fn w31a118_ef80(r: i32) -> i32 {
    w31a118_clamp_neg(r)
}

/// `0x00c6efb0`: negative result collapses to -1 (selector 0x50).
pub fn w31a118_efb0(r: i32) -> i32 {
    w31a118_clamp_neg(r)
}

#[cfg(test)]
mod tests_wave31_agent118 {
    use super::*;

    #[test]
    fn disposition() {
        assert_eq!(w31a118_open_disposition(0x400), 5);
        assert_eq!(w31a118_open_disposition(0), 3);
        assert_eq!(w31a118_open_disposition(0x200), 4);
        assert_eq!(w31a118_open_disposition(0x600), 2);
        assert_eq!(w31a118_open_disposition(0xa00), 1);
        assert_eq!(w31a118_open_disposition(7), 0);
    }

    #[test]
    fn chunks() {
        let mut calls = vec![];
        let r = w31a118_read_chunked(0x4000_0001, |o, n| {
            calls.push((o, n));
            Some(n)
        });
        assert_eq!(r, Ok(0x4000_0001));
        assert_eq!(calls, vec![(0, 0x4000_0000), (0x4000_0000, 1)]);
        assert_eq!(w31a118_write_chunked(10, |_, _| Some(4)), Ok(4));
        assert_eq!(w31a118_write_chunked(10, |_, _| None), Err(()));
        assert_eq!(w31a118_read_chunked(0, |_, _| None), Ok(0));
    }

    #[test]
    fn clamps() {
        assert_eq!(w31a118_clamp_neg(-5), -1);
        assert_eq!(w31a118_ef50(3), 3);
        assert_eq!(w31a118_ef80(-1), -1);
        assert_eq!(w31a118_efb0(0), 0);
    }
}
