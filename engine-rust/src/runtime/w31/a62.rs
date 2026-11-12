#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent62.rs
// ===========================================================================
// Wave 31 agent 62: provider list accessors and small helpers.

/// Error code returned for an out-of-range index or null out pointer.
pub const W31A62_ERR_RANGE: u32 = 0x25;
/// Error code returned when the intrusive list is empty.
pub const W31A62_ERR_EMPTY: u32 = 0x21;

/// `0x0099d700`: fetch the `index`-th entry of the primary list.  Returns
/// `(item, field_e8)`; `0x25` when `index >= count`, `0x21` when the list is
/// empty.  Locking (done only when the caller asked) is not modelled.
pub fn w31a62_nth_primary(list: &[(u32, u32)], index: i32) -> Result<(u32, u32), u32> {
    if index >= list.len() as i32 {
        return Err(W31A62_ERR_RANGE);
    }
    if list.is_empty() {
        return Err(W31A62_ERR_EMPTY);
    }
    let i = index.max(0) as usize;
    list.get(i).copied().ok_or(W31A62_ERR_RANGE)
}

/// `0x0099d9a0`: same as the primary lookup but over the secondary list
/// (count at +0x48, field at +0xec).
pub fn w31a62_nth_secondary(list: &[(u32, u32)], index: i32) -> Result<(u32, u32), u32> {
    w31a62_nth_primary(list, index)
}

/// `0x0099d8b0`: store the primary list count; `0x25` when the out slot is
/// missing.
pub fn w31a62_get_primary_count(out: Option<&mut u32>, count: u32) -> u32 {
    match out {
        None => W31A62_ERR_RANGE,
        Some(o) => {
            *o = count;
            0
        }
    }
}

/// `0x0099d910`: store the secondary list count; `0x25` when the out slot is
/// missing.
pub fn w31a62_get_secondary_count(out: Option<&mut u32>, count: u32) -> u32 {
    match out {
        None => W31A62_ERR_RANGE,
        Some(o) => {
            *o = count;
            0
        }
    }
}

/// `0x0099ed50`: bounded string copy that stops after copying the NUL and
/// does not pad.  Copies at most `n` bytes from `src` into `dst`.
pub fn w31a62_copy_bounded(dst: &mut [u8], src: &[u8], n: usize) {
    for i in 0..n {
        let c = src.get(i).copied().unwrap_or(0);
        if i < dst.len() {
            dst[i] = c;
        }
        if c == 0 {
            break;
        }
    }
}

/// `0x0099f110`: closes a handle through the supplied callback; `-1` is the
/// invalid handle and yields `0x25`.
pub fn w31a62_close_handle(handle: i32, close: impl FnOnce(i32)) -> u32 {
    if handle == -1 {
        return W31A62_ERR_RANGE;
    }
    close(handle);
    0
}

/// Event node queued by `0x0099ce70`: kind 2, with the owner, two params.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W31a62Event {
    pub owner: u32,
    pub param_a: u32,
    pub param_b: u32,
    pub kind: u32,
}

/// `0x0099ce70`: builds the kind-2 event and returns the flags with `0x100`
/// set on `param_a`'s object (or on the owner when `param_a == 0`).
/// Returns `(event, owner_flags, target_flags)`.
pub fn w31a62_queue_event(
    owner: u32,
    owner_flags: u32,
    param_a: u32,
    target_flags: u32,
    param_b: u32,
) -> (W31a62Event, u32, u32) {
    let ev = W31a62Event {
        owner,
        param_a,
        param_b,
        kind: 2,
    };
    if param_a == 0 {
        (ev, owner_flags | 0x100, target_flags)
    } else {
        (ev, owner_flags, target_flags | 0x100)
    }
}

#[cfg(test)]
mod tests_wave31_agent62 {
    use super::*;

    #[test]
    fn nth_lookup() {
        let l = [(1, 10), (2, 20)];
        assert_eq!(w31a62_nth_primary(&l, 1), Ok((2, 20)));
        assert_eq!(w31a62_nth_primary(&l, 2), Err(0x25));
        assert_eq!(w31a62_nth_secondary(&l, 0), Ok((1, 10)));
        assert_eq!(w31a62_nth_primary(&[], -1), Err(0x21));
    }

    #[test]
    fn counts() {
        let mut v = 0;
        assert_eq!(w31a62_get_primary_count(Some(&mut v), 5), 0);
        assert_eq!(v, 5);
        assert_eq!(w31a62_get_secondary_count(None, 5), 0x25);
        assert_eq!(w31a62_get_secondary_count(Some(&mut v), 7), 0);
        assert_eq!(v, 7);
    }

    #[test]
    fn copy_and_close() {
        let mut d = [9u8; 6];
        w31a62_copy_bounded(&mut d, b"ab\0cd", 5);
        assert_eq!(d, [b'a', b'b', 0, 9, 9, 9]);
        let mut got = 0;
        assert_eq!(w31a62_close_handle(-1, |_| got = 1), 0x25);
        assert_eq!(w31a62_close_handle(4, |h| got = h), 0);
        assert_eq!(got, 4);
    }

    #[test]
    fn event() {
        let (e, o, t) = w31a62_queue_event(1, 0, 0, 0, 3);
        assert_eq!((e.kind, o, t), (2, 0x100, 0));
        let (_, o, t) = w31a62_queue_event(1, 0, 5, 0, 3);
        assert_eq!((o, t), (0, 0x100));
    }
}
