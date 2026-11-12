#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent23.rs
// ===========================================================================
// Wave 22 agent 23: partial-buffer adapter flags and owned-pointer reset.

/// Fields written by the partial-buffer adapter constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W22a23PartialBuffer {
    /// Slot 1: data buffer handle.
    pub buffer: u32,
    /// Slot 2: flags.
    pub flags: u32,
    /// Slot 3: secondary buffer handle.
    pub secondary: u32,
    /// Slot 5: capacity.
    pub capacity: u32,
    /// Slot 6: owner/context parameter.
    pub owner: u32,
}

/// `0x006a94c0`: partial-buffer adapter constructor.  Base flags are 4 when
/// `flag7` or a caller secondary buffer exists, else 6.  Without `flag7` the
/// secondary is the caller's and bit 0x20000 is cleared; with it the
/// secondary comes from `alloc_secondary` and 0x20000 is set.  A zero
/// `buffer` is allocated through `alloc_buffer(capacity)` and sets bit 1.
#[allow(clippy::too_many_arguments)]
pub fn w22a23_partial_buffer_init(
    owner: u32,
    capacity: u32,
    flags_in: u32,
    secondary_in: u32,
    buffer_in: u32,
    flag7: bool,
    alloc_secondary: impl FnOnce(u32) -> u32,
    alloc_buffer: impl FnOnce(u32) -> u32,
) -> W22a23PartialBuffer {
    let base: u32 = if flag7 || secondary_in != 0 { 4 } else { 6 };
    let mut flags = base | flags_in;
    let secondary;
    if !flag7 {
        secondary = secondary_in;
        flags = base | (flags_in & 0xfffd_ffff);
    } else {
        secondary = alloc_secondary(owner);
        flags |= 0x20000;
    }
    let mut buffer = buffer_in;
    if buffer == 0 {
        buffer = alloc_buffer(capacity);
        flags |= 1;
    }
    W22a23PartialBuffer {
        buffer,
        flags,
        secondary,
        capacity,
        owner,
    }
}

/// `0x006a98c0`: takes the pointer out of `src` (leaving 0), stores it in
/// `dst` and returns the previous `dst` value when non-zero (to be freed).
pub fn w22a23_take_owned(dst: &mut i32, src: &mut i32) -> Option<i32> {
    let incoming = *src;
    *src = 0;
    let old = std::mem::replace(dst, incoming);
    if old != 0 { Some(old) } else { None }
}

#[cfg(test)]
mod tests_wave22_agent23 {
    use super::*;

    #[test]
    fn partial_buffer_flags() {
        let r = w22a23_partial_buffer_init(9, 64, 0x2_0000, 5, 7, false, |_| 99, |_| 77);
        assert_eq!(r.flags, 4);
        assert_eq!((r.secondary, r.buffer), (5, 7));
        let r = w22a23_partial_buffer_init(9, 64, 0, 0, 0, false, |_| 99, |c| c + 1);
        assert_eq!(r.flags, 7);
        assert_eq!(r.buffer, 65);
        let r = w22a23_partial_buffer_init(9, 64, 8, 0, 3, true, |o| o + 100, |_| 0);
        assert_eq!(r.flags, 0x2000c);
        assert_eq!(r.secondary, 109);
    }

    #[test]
    fn take_owned() {
        let (mut a, mut b) = (5, 6);
        assert_eq!(w22a23_take_owned(&mut a, &mut b), Some(5));
        assert_eq!((a, b), (6, 0));
        assert_eq!(w22a23_take_owned(&mut b, &mut a), None);
        assert_eq!((a, b), (0, 6));
    }
}
