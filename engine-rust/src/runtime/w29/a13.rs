#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent13.rs
// ===========================================================================
// Wave 29 translation batch 13: refcount and TACT query-state helpers.

/// `0x00ad13d0`: stores `obj` in the smart-pointer slot and, when non-null
/// (non-zero), atomically increments the refcount dword at offset 4.
pub fn w29a13_addref_assign(
    slot: &mut u32,
    obj: u32,
    refcount: Option<&std::sync::atomic::AtomicI32>,
) {
    *slot = obj;
    if obj != 0
        && let Some(rc) = refcount
    {
        rc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// `0x00ad1550`: releases the smart pointer; decrements the refcount and
/// returns true when the previous value was 1 (virtual slot 1 must be called).
pub fn w29a13_release(rc: Option<&std::sync::atomic::AtomicI32>) -> bool {
    match rc {
        None => false,
        Some(rc) => rc.fetch_sub(1, std::sync::atomic::Ordering::SeqCst) == 1,
    }
}

/// Action chosen by `0x00ad15a0`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W29a13Delete {
    Nothing,
    VirtualDelete3,
    FreeBlock,
}

/// `0x00ad15a0`: null pointer does nothing; a non-zero dword before the
/// object means call the virtual deleting destructor with flag 3, otherwise
/// the raw block at `ptr - 4` is freed.
pub fn w29a13_delete_choice(ptr: u32, word_before: u32) -> W29a13Delete {
    if ptr == 0 {
        W29a13Delete::Nothing
    } else if word_before != 0 {
        W29a13Delete::VirtualDelete3
    } else {
        W29a13Delete::FreeBlock
    }
}

/// `0x00ad1a90`: true when the capacity field (offset 0x40) is smaller than
/// the requested size, i.e. the grow/throw path is taken.
pub fn w29a13_needs_grow(capacity: u32, wanted: u32) -> bool {
    capacity < wanted
}

/// Query-state fields touched by `0x00ad1ae0` / `0x00ad1b40`.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct W29a13QueryState {
    pub state: i32,    // +0x08
    pub buf: u32,      // +0x3c
    pub handle70: u32, // +0x70
    pub handle7c: u32, // +0x7c
    pub field78: u32,  // +0x78
    pub field84: u32,  // +0x84
}

/// `0x00ad1ae0`: takes and releases handle70, sets 0x78/0x7c to -1, clears
/// 0x84, takes buffer 0x3c (freed) and clears the state word.  Returns the
/// (handle70, buffer) values to release.
pub fn w29a13_reset_a(s: &mut W29a13QueryState) -> (u32, u32) {
    let h = std::mem::take(&mut s.handle70);
    s.field78 = u32::MAX;
    s.handle7c = u32::MAX;
    s.field84 = 0;
    let b = std::mem::take(&mut s.buf);
    s.state = 0;
    (h, b)
}

/// `0x00ad1b40`: takes handle7c and buffer 0x3c for release, clears state.
pub fn w29a13_reset_b(s: &mut W29a13QueryState) -> (u32, u32) {
    let h = std::mem::take(&mut s.handle7c);
    let b = std::mem::take(&mut s.buf);
    s.state = 0;
    (h, b)
}

/// `0x00ad21e0`: initialises a 0x84-byte query record in place: state=1 at
/// +8, copies the 24-byte keys `a` to +0xc and `b` to +0x24, zeroes the
/// listed dwords and returns the old +0x7c handle (to release); +0x80 byte=0.
pub fn w29a13_init_query(rec: &mut [u8], a: &[u8; 24], b: &[u8; 24]) -> u32 {
    let put = |r: &mut [u8], off: usize, v: u32| r[off..off + 4].copy_from_slice(&v.to_le_bytes());
    put(rec, 8, 1);
    rec[0xc..0xc + 24].copy_from_slice(a);
    rec[0x24..0x24 + 24].copy_from_slice(b);
    for off in [
        0x40, 0x44, 0x48, 0x4c, 0x60, 0x64, 0x58, 0x5c, 0x70, 0x74, 0x68, 0x6c, 0x78,
    ] {
        put(rec, off, 0);
    }
    let old = u32::from_le_bytes(rec[0x7c..0x80].try_into().unwrap());
    put(rec, 0x7c, 0);
    rec[0x80] = 0;
    old
}

#[cfg(test)]
mod tests_wave29_agent13 {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};

    #[test]
    fn refcount() {
        let rc = AtomicI32::new(1);
        let mut slot = 0;
        w29a13_addref_assign(&mut slot, 5, Some(&rc));
        assert_eq!((slot, rc.load(Ordering::SeqCst)), (5, 2));
        w29a13_addref_assign(&mut slot, 0, Some(&rc));
        assert_eq!(rc.load(Ordering::SeqCst), 2);
        assert!(!w29a13_release(Some(&rc)));
        assert!(w29a13_release(Some(&rc)));
        assert!(!w29a13_release(None));
    }

    #[test]
    fn delete_and_grow() {
        assert_eq!(w29a13_delete_choice(0, 1), W29a13Delete::Nothing);
        assert_eq!(w29a13_delete_choice(8, 1), W29a13Delete::VirtualDelete3);
        assert_eq!(w29a13_delete_choice(8, 0), W29a13Delete::FreeBlock);
        assert!(w29a13_needs_grow(3, 4));
        assert!(!w29a13_needs_grow(4, 4));
    }

    #[test]
    fn resets() {
        let mut s = W29a13QueryState {
            state: 1,
            buf: 2,
            handle70: 3,
            handle7c: 4,
            field78: 5,
            field84: 6,
        };
        let mut t = s.clone();
        assert_eq!(w29a13_reset_a(&mut s), (3, 2));
        assert_eq!(
            (s.field78, s.handle7c, s.field84, s.state),
            (u32::MAX, u32::MAX, 0, 0)
        );
        assert_eq!(w29a13_reset_b(&mut t), (4, 2));
        assert_eq!((t.handle7c, t.buf, t.state), (0, 0, 0));
    }

    #[test]
    fn init_query() {
        let mut rec = vec![0xffu8; 0x88];
        rec[0x7c..0x80].copy_from_slice(&9u32.to_le_bytes());
        let old = w29a13_init_query(&mut rec, &[1; 24], &[2; 24]);
        assert_eq!(old, 9);
        assert_eq!(rec[8], 1);
        assert_eq!(rec[0xc], 1);
        assert_eq!(rec[0x24], 2);
        assert_eq!(rec[0x40], 0);
        assert_eq!(rec[0x80], 0);
        assert_eq!(rec[0x84], 0xff);
    }
}
