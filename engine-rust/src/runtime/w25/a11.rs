#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent11.rs
// ===========================================================================
// Wave 25 agent 11: small container / functor-slot helpers (runtime.rs target).

/// Shared slot reset (helper for the routines below): a functor slot
/// word whose bit 0 marks a heap-held target (pointer = word & !1); otherwise
/// the target is the slot itself.  The target is invoked with `0` (destroy),
/// the heap block is released when bit 0 was set, and the slot becomes `1`.
/// Returns true when a target existed (always true in retail logic).
pub fn w25a11_reset_functor_slot(
    slot: &mut u32,
    mut destroy: impl FnMut(Option<u32>),
    mut free: impl FnMut(u32),
) -> bool {
    let heap = *slot & 1 != 0;
    let target = if heap { Some(*slot & !1) } else { None };
    // `None` stands for "the slot itself" (always non-null).
    destroy(target);
    if let Some(p) = target {
        free(p);
    }
    *slot = 1;
    true
}

/// `0x007e0310`: resets the functor slots at offsets 0x90 then 0xa4.
pub fn w25a11_reset_two_functors(
    obj: &mut [u32; 2],
    mut destroy: impl FnMut(usize, Option<u32>),
    mut free: impl FnMut(usize, u32),
) {
    for i in 0..2 {
        w25a11_reset_functor_slot(&mut obj[i], |t| destroy(i, t), |p| free(i, p));
    }
}

/// `0x00805e70`: copies fields 0x60/0x64 into 0x98/0x9c, zeroes the u64 at
/// 0x88, calls the unresolved clear callee, then resets the functor slots at
/// offsets 0x10, 0x24, 0x38, 0x4c (in that order).
pub fn w25a11_reset_state(
    f60: u32,
    f64_: u32,
    out98: &mut u32,
    out9c: &mut u32,
    out88: &mut u64,
    slots: &mut [u32; 4],
    mut clear: impl FnMut(),
    mut destroy: impl FnMut(usize, Option<u32>),
    mut free: impl FnMut(usize, u32),
) {
    *out98 = f60;
    *out9c = f64_;
    *out88 = 0;
    clear();
    for i in 0..4 {
        w25a11_reset_functor_slot(&mut slots[i], |t| destroy(i, t), |p| free(i, p));
    }
}

/// `0x007de910`: destroys `count` records of stride `0x98` by calling
/// `destroy` on each index in order; returns true when the buffer must be
/// freed (capacity word non-negative).
pub fn w25a11_destroy_range(count: u32, cap_word: i32, mut destroy: impl FnMut(usize)) -> bool {
    for i in 0..count as usize {
        destroy(i);
    }
    cap_word >= 0
}

/// `0x0086c530` and `0x0086fc00`: slot computation of deque
/// push_back (the two differ only in a trailing callee), 8 elements per block, 8-byte elements).  `offset`/`size` are
/// fields 0x1c/0x20, `map_size` field 0x18.  Returns
/// `(grow_needed, block_index, slot_in_block, new_size)`; after growing the
/// caller re-reads offset/size.
pub fn w25a11_deque_push_slot(offset: u32, size: u32, map_size: u32) -> (bool, u32, u32, u32) {
    let grow = (offset.wrapping_add(size) & 7) == 0 && map_size <= (size.wrapping_add(8) >> 3);
    let idx = offset.wrapping_add(size);
    (
        grow,
        (map_size.wrapping_sub(1) & (idx >> 3)),
        idx & 7,
        size.wrapping_add(1),
    )
}

#[cfg(test)]
mod tests_wave25_agent11 {
    use super::*;

    #[test]
    fn functor_slot() {
        let mut s = 0x1000u32 | 1;
        let log = std::cell::RefCell::new(vec![]);
        w25a11_reset_functor_slot(
            &mut s,
            |t| log.borrow_mut().push(("d", t)),
            |p| log.borrow_mut().push(("f", Some(p))),
        );
        assert_eq!(s, 1);
        assert_eq!(
            *log.borrow(),
            vec![("d", Some(0x1000)), ("f", Some(0x1000))]
        );
        let mut s = 0x2000u32;
        let n = std::cell::Cell::new(0);
        w25a11_reset_functor_slot(
            &mut s,
            |t| {
                assert!(t.is_none());
                n.set(n.get() + 1)
            },
            |_| n.set(n.get() + 10),
        );
        assert_eq!((s, n.get()), (1, 1));
    }

    #[test]
    fn two_and_state() {
        let mut o = [0x10, 0x21];
        let mut order = vec![];
        w25a11_reset_two_functors(&mut o, |i, _| order.push(i), |_, _| {});
        assert_eq!(o, [1, 1]);
        assert_eq!(order, vec![0, 1]);
        let (mut a, mut b, mut c) = (0, 0, 9u64);
        let mut sl = [4, 8, 12, 16];
        let mut cleared = false;
        w25a11_reset_state(
            5,
            6,
            &mut a,
            &mut b,
            &mut c,
            &mut sl,
            || cleared = true,
            |_, _| {},
            |_, _| {},
        );
        assert_eq!((a, b, c, sl, cleared), (5, 6, 0, [1; 4], true));
    }

    #[test]
    fn destroy_and_deque() {
        let mut v = vec![];
        assert!(w25a11_destroy_range(3, 0, |i| v.push(i)));
        assert_eq!(v, vec![0, 1, 2]);
        assert!(!w25a11_destroy_range(0, -1, |_| {}));
        assert_eq!(w25a11_deque_push_slot(0, 5, 4), (false, 0, 5, 6));
        assert_eq!(w25a11_deque_push_slot(0, 8, 1), (true, 0, 0, 9));
        assert_eq!(w25a11_deque_push_slot(3, 5, 2), (false, 1, 0, 6));
    }
}
