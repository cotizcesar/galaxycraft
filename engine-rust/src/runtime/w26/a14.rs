#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent14.rs
// ===========================================================================
// Wave 26 agent 14: IO unit helpers (resize, MRU cache slots, cursor unit).

/// `0x00937540`: resizes a `u16` vector to `new_len`; shrinking drops the tail,
/// growing appends copies of `fill`.
pub fn w26a14_resize_u16(v: &mut Vec<u16>, new_len: usize, fill: u16) {
    if new_len < v.len() {
        v.truncate(new_len);
    } else if v.len() < new_len {
        v.resize(new_len, fill);
    }
}

/// 16-byte cache slot: key (two words), dirty flags byte at +8, buffer at +0xc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W26a14Slot {
    pub key_lo: i32,
    pub key_hi: i32,
    pub flags: u8,
    pub buf: u32,
}

/// `0x00938eb0`: invalidates slot `idx` (keys -1, flags 0) and shifts every
/// later slot down by one (stride 0x10).  The last slot is only invalidated.
pub fn w26a14_slot_invalidate(slots: &mut [W26a14Slot], idx: usize) {
    slots[idx].key_lo = -1;
    slots[idx].key_hi = -1;
    slots[idx].flags = 0;
    if idx + 1 != slots.len() {
        slots[idx..].rotate_left(1);
    }
}

/// `0x00938de0`: looks up slot (`lo`,`hi`); on a miss the last slot is flushed
/// via `flush`, re-keyed and refilled through `fetch`.  The slot is moved to the
/// front, `flags |= or_flags`, and its buffer returned.  Layout of the front
/// move is inferred (noreturn memmove in the body).
pub fn w26a14_slot_get(
    slots: &mut [W26a14Slot],
    lo: i32,
    hi: i32,
    or_flags: u8,
    mut flush: impl FnMut(usize),
    mut fetch: impl FnMut(i32, i32) -> u32,
) -> u32 {
    let idx = match slots.iter().position(|s| s.key_lo == lo && s.key_hi == hi) {
        Some(i) => i,
        None => {
            let i = slots.len() - 1;
            flush(i);
            slots[i].key_lo = lo;
            slots[i].key_hi = hi;
            slots[i].buf = fetch(lo, hi);
            i
        }
    };
    if idx != 0 {
        slots[..=idx].rotate_right(1);
    }
    slots[0].flags |= or_flags;
    slots[0].buf
}

/// `0x009397d0`: builds an IOCursorUnit (0x18 bytes); `None` if `provider` is 0
/// (the original raises an error).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W26a14Cursor {
    pub provider: u32,
    pub refcount: u32,
    pub pos_lo: u32,
    pub pos_hi: u32,
}

pub fn w26a14_cursor_new(provider: u32) -> Option<W26a14Cursor> {
    if provider == 0 {
        return None;
    }
    Some(W26a14Cursor {
        provider,
        refcount: 1,
        pos_lo: 0,
        pos_hi: 0,
    })
}

/// `0x00939820`: calls `read` with the 64-bit position; on success advances
/// the position (with carry) by the returned count and returns it, else 0.
pub fn w26a14_cursor_advance(
    c: &mut W26a14Cursor,
    mut read: impl FnMut(u32, u32) -> Option<u32>,
) -> u32 {
    match read(c.pos_lo, c.pos_hi) {
        None => 0,
        Some(n) => {
            let p = ((c.pos_hi as u64) << 32 | c.pos_lo as u64).wrapping_add(n as u64);
            c.pos_lo = p as u32;
            c.pos_hi = (p >> 32) as u32;
            n
        }
    }
}

/// `0x009396e0`: copies `n` bytes of `src` into the 0x114-limited buffer;
/// `None` if `n` exceeds `capacity`.
pub fn w26a14_copy_checked(dst: &mut [u8], capacity: usize, src: &[u8], n: usize) -> Option<()> {
    if capacity < n {
        return None;
    }
    dst[..n].copy_from_slice(&src[..n]);
    Some(())
}

#[cfg(test)]
mod tests_wave26_agent14 {
    use super::*;

    fn s(a: i32) -> W26a14Slot {
        W26a14Slot {
            key_lo: a,
            key_hi: 0,
            flags: 0,
            buf: a as u32 * 10,
        }
    }

    #[test]
    fn resize() {
        let mut v = vec![1, 2, 3];
        w26a14_resize_u16(&mut v, 5, 9);
        assert_eq!(v, [1, 2, 3, 9, 9]);
        w26a14_resize_u16(&mut v, 1, 0);
        assert_eq!(v, [1]);
    }

    #[test]
    fn invalidate() {
        let mut v = [s(1), s(2), s(3)];
        w26a14_slot_invalidate(&mut v, 0);
        assert_eq!(v[0].key_lo, 2);
        assert_eq!(v[2].key_lo, -1);
        w26a14_slot_invalidate(&mut v, 2);
        assert_eq!(v[2].key_hi, -1);
    }

    #[test]
    fn get() {
        let mut v = [s(1), s(2), s(3)];
        assert_eq!(w26a14_slot_get(&mut v, 3, 0, 4, |_| {}, |_, _| 0), 30);
        assert_eq!(v[0].key_lo, 3);
        assert_eq!(v[0].flags, 4);
        let mut fl = 0;
        let r = w26a14_slot_get(&mut v, 7, 0, 1, |i| fl = i + 1, |_, _| 70);
        assert_eq!((r, fl, v[0].key_lo), (70, 3, 7));
    }

    #[test]
    fn cursor() {
        assert!(w26a14_cursor_new(0).is_none());
        let mut c = w26a14_cursor_new(5).unwrap();
        c.pos_lo = u32::MAX;
        assert_eq!(w26a14_cursor_advance(&mut c, |_, _| Some(2)), 2);
        assert_eq!((c.pos_lo, c.pos_hi), (1, 1));
        assert_eq!(w26a14_cursor_advance(&mut c, |_, _| None), 0);
    }

    #[test]
    fn copy() {
        let mut d = [0u8; 4];
        assert!(w26a14_copy_checked(&mut d, 2, &[1, 2, 3], 3).is_none());
        assert!(w26a14_copy_checked(&mut d, 3, &[1, 2, 3], 3).is_some());
        assert_eq!(d, [1, 2, 3, 0]);
    }
}
