#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent27.rs
// ===========================================================================
// Wave 24 agent 27: small UI-state helpers (runtime.rs target).

/// State touched by the 0x011b13b8..0x011b14d0 family.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W24a27Ui {
    pub handle_13b8: u32,
    pub flag_14ce: u8,
    pub flag_14d0: u32,
    pub flag_a4580: u32,
}

/// `0x007836f0`: calls the redraw callee when a handle is set, then sets the 14d0 flag to 1.
pub fn w24a27_set_active(s: &mut W24a27Ui, mut redraw: impl FnMut()) {
    if s.handle_13b8 != 0 {
        redraw();
    }
    s.flag_14d0 = 1;
}

/// `0x00784db0`: stores eight 16-bit parameters into the table.
pub fn w24a27_store8(table: &mut [u16; 8], v: [u16; 8]) {
    *table = v;
}

/// `0x00785410`: runs the callee, optionally refreshes via handle, clears the 14d0 flag.
pub fn w24a27_deactivate(s: &mut W24a27Ui, mut redraw: impl FnMut(), mut refresh: impl FnMut(u32)) {
    redraw();
    if s.flag_a4580 != 0 && s.flag_14ce == 0 {
        refresh(s.handle_13b8);
    }
    s.flag_14d0 = 0;
}

/// `0x00785860`: like deactivate, but only when handle and a4580 are both non-zero.
pub fn w24a27_deactivate_if_active(
    s: &mut W24a27Ui,
    redraw: impl FnMut(),
    refresh: impl FnMut(u32),
) {
    if s.handle_13b8 != 0 && s.flag_a4580 != 0 {
        w24a27_deactivate(s, redraw, refresh);
    }
}

/// `0x00785fe0`: resets three globals (flag=1, two words cleared).
pub fn w24a27_reset3(flag_15dc: &mut u32, w_162c: &mut u32, w_1630: &mut u32) {
    *flag_15dc = 1;
    *w_162c = 0;
    *w_1630 = 0;
}

/// `0x00787230`: stores a byte into the 15dd global.
pub fn w24a27_set_byte(slot: &mut u8, v: u8) {
    *slot = v;
}

/// `0x00786072`: resyncs the cached id with the current one, frees the 1634 object, clears 15de.
pub fn w24a27_resync(
    cached: &mut i32,
    current: i32,
    obj_1634: &mut u32,
    flag_15de: &mut u8,
    mut release_id: impl FnMut(i32),
    mut free_obj: impl FnMut(u32),
) {
    if *cached != current {
        release_id(*cached);
        *cached = current;
    }
    if *obj_1634 != 0 {
        free_obj(*obj_1634);
        *obj_1634 = 0;
    }
    *flag_15de = 0;
}

/// `0x00786120`: walks a singly linked list from `start`; if the node's
/// short at word 0xf is zero, starts from its word 0x17 child instead.
pub fn w24a27_walk_list<N: Copy>(
    start: N,
    short_0xf: impl Fn(N) -> i16,
    child_0x17: impl Fn(N) -> Option<N>,
    next: impl Fn(N) -> Option<N>,
    mut visit: impl FnMut(N),
) {
    let mut cur = if short_0xf(start) == 0 {
        child_0x17(start)
    } else {
        Some(start)
    };
    while let Some(n) = cur {
        visit(n);
        cur = next(n);
    }
}

/// `0x00786150`: visits every node after the head found by lookup (-7).
pub fn w24a27_walk_after_head<N: Copy>(
    head: N,
    next: impl Fn(N) -> Option<N>,
    mut visit: impl FnMut(N),
) {
    let mut cur = next(head);
    while let Some(n) = cur {
        visit(n);
        cur = next(n);
    }
}

#[cfg(test)]
mod tests_wave24_agent27 {
    use super::*;

    #[test]
    fn active_flags() {
        let mut s = W24a27Ui::default();
        let mut n = 0;
        w24a27_set_active(&mut s, || n += 1);
        assert_eq!((n, s.flag_14d0), (0, 1));
        s.handle_13b8 = 5;
        s.flag_a4580 = 1;
        w24a27_set_active(&mut s, || n += 1);
        assert_eq!(n, 1);
        let mut r = vec![];
        w24a27_deactivate(&mut s, || n += 1, |h| r.push(h));
        assert_eq!((n, r.clone(), s.flag_14d0), (2, vec![5], 0));
        s.flag_14ce = 1;
        w24a27_deactivate_if_active(&mut s, || n += 1, |h| r.push(h));
        assert_eq!((n, r.len()), (3, 1));
        s.flag_a4580 = 0;
        w24a27_deactivate_if_active(&mut s, || n += 1, |h| r.push(h));
        assert_eq!(n, 3);
    }

    #[test]
    fn simple_stores() {
        let mut t = [0u16; 8];
        w24a27_store8(&mut t, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(t[7], 8);
        let (mut a, mut b, mut c) = (0, 9, 9);
        w24a27_reset3(&mut a, &mut b, &mut c);
        assert_eq!((a, b, c), (1, 0, 0));
        let mut x = 0u8;
        w24a27_set_byte(&mut x, 7);
        assert_eq!(x, 7);
    }

    #[test]
    fn resync() {
        let (mut c, mut o, mut f) = (1, 4u32, 1u8);
        let (mut rel, mut fr) = (vec![], vec![]);
        w24a27_resync(&mut c, 2, &mut o, &mut f, |i| rel.push(i), |x| fr.push(x));
        assert_eq!((c, o, f, rel, fr), (2, 0, 0, vec![1], vec![4]));
    }

    #[test]
    fn walks() {
        let next = |n: u32| if n < 4 { Some(n + 1) } else { None };
        let mut v = vec![];
        w24a27_walk_list(1u32, |_| 1, |_| Some(3), next, |n| v.push(n));
        assert_eq!(v, vec![1, 2, 3, 4]);
        v.clear();
        w24a27_walk_list(1u32, |_| 0, |_| Some(3), next, |n| v.push(n));
        assert_eq!(v, vec![3, 4]);
        v.clear();
        w24a27_walk_after_head(2u32, next, |n| v.push(n));
        assert_eq!(v, vec![3, 4]);
    }
}
