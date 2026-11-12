#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent47.rs
// ===========================================================================
// Wave 31 agent 47: small UI/texture helpers.

/// Result of the size-class selection in `0x008a3910`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W31a47SizeClass {
    /// Vtable/handler id written to the first dword (low address bits).
    pub handler: u32,
    pub width: u16,
    pub height: u16,
    pub mode: u32,
}

/// `0x008a3910`: picks a scale class from the larger of two u16 dimensions
/// (0x40 doubles, 0x60/0x80 keep, 0xc0/0x100 halve); other sizes fail.
pub fn w31a47_size_class(w: u16, h: u16) -> Option<W31a47SizeClass> {
    let m = w.max(h);
    match m {
        0x40 => Some(W31a47SizeClass {
            handler: 0x71_baa0,
            width: w.wrapping_mul(2),
            height: h.wrapping_mul(2),
            mode: 0x10001,
        }),
        0x60 | 0x80 => Some(W31a47SizeClass {
            handler: 0x71_b9c0,
            width: w,
            height: h,
            mode: 1,
        }),
        0xc0 | 0x100 => Some(W31a47SizeClass {
            handler: 0x71_ba20,
            width: w >> 1,
            height: h >> 1,
            mode: 2,
        }),
        _ => None,
    }
}

fn w31a47_in_range(i: i32, count: i32) -> bool {
    i > -1 && i < count
}

/// `0x008a7840`: stores `index` as the current slot only if within
/// `0..count`, selects it, then always runs the refresh callback.
pub fn w31a47_set_slot(
    slot: &mut i32,
    index: i32,
    count: i32,
    mut select: impl FnMut(i32),
    mut refresh: impl FnMut(),
) {
    if w31a47_in_range(index, count) {
        *slot = index;
        select(index);
    }
    refresh();
}

/// `0x008a7800`: reselects the current slot if valid, then refresh and a
/// trailing call.
pub fn w31a47_reselect_slot(
    slot: i32,
    count: i32,
    mut select: impl FnMut(i32),
    mut refresh: impl FnMut(),
    mut tail: impl FnMut(),
) {
    if w31a47_in_range(slot, count) {
        select(slot);
    }
    refresh();
    tail();
}

/// `0x008a7da0`: selects the slot if valid, runs the refresh, then forwards
/// `arg` to the final callback.
pub fn w31a47_select_and_forward<T>(
    slot: i32,
    count: i32,
    arg: T,
    mut select: impl FnMut(i32),
    mut refresh: impl FnMut(),
    mut forward: impl FnMut(T),
) {
    if w31a47_in_range(slot, count) {
        select(slot);
    }
    refresh();
    forward(arg);
}

#[cfg(test)]
mod tests_wave31_agent47 {
    use super::*;

    #[test]
    fn size_class() {
        let a = w31a47_size_class(0x20, 0x40).unwrap();
        assert_eq!((a.width, a.height, a.mode), (0x40, 0x80, 0x10001));
        let b = w31a47_size_class(0x60, 0x10).unwrap();
        assert_eq!((b.width, b.mode), (0x60, 1));
        let c = w31a47_size_class(0x100, 0x80).unwrap();
        assert_eq!((c.width, c.height, c.mode), (0x80, 0x40, 2));
        assert!(w31a47_size_class(0x50, 0x50).is_none());
    }

    #[test]
    fn slots() {
        let mut slot = 5;
        let log = std::cell::RefCell::new(vec![]);
        w31a47_set_slot(
            &mut slot,
            9,
            4,
            |i| log.borrow_mut().push(i),
            || log.borrow_mut().push(-100),
        );
        assert_eq!((slot, log.borrow().clone()), (5, vec![-100]));
        w31a47_set_slot(
            &mut slot,
            3,
            4,
            |i| log.borrow_mut().push(i),
            || log.borrow_mut().push(-100),
        );
        assert_eq!(slot, 3);
        assert_eq!(*log.borrow(), vec![-100, 3, -100]);

        let n = std::cell::Cell::new(0);
        w31a47_reselect_slot(
            -1,
            4,
            |_| n.set(n.get() + 1),
            || n.set(n.get() + 10),
            || n.set(n.get() + 100),
        );
        assert_eq!(n.get(), 110);
        let mut out = None;
        w31a47_select_and_forward(2, 4, 7u8, |_| {}, || {}, |v| out = Some(v));
        assert_eq!(out, Some(7));
    }
}
