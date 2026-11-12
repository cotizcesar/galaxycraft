#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent15.rs
// ===========================================================================
// Wave 21 agent 15 translations (runtime helpers).

/// `0x005c7290`: true when `a != b`, `pred_a(a)` holds and `second()` holds.
pub fn w21a15_pair_check(
    a: i32,
    b: i32,
    mut pred_a: impl FnMut(i32) -> bool,
    mut second: impl FnMut() -> bool,
) -> bool {
    a != b && pred_a(a) && second()
}

/// `0x005d4520`: when `a != b`, `first()` is false, `second()` is true and
/// the byte at offset `0x4d` of `obj` is non-zero, returns `true` and the
/// caller must set the flag global (`_DAT_00fa391c = 1`).
pub fn w21a15_flagged_check(
    a: i32,
    b: i32,
    obj: &[u8],
    mut first: impl FnMut() -> bool,
    mut second: impl FnMut() -> bool,
) -> bool {
    a != b && !first() && second() && obj.get(0x4d).is_some_and(|&v| v != 0)
}

/// `0x005cbc00`: resets the global at `00fa3918` to zero.
pub fn w21a15_clear_global(slot: &mut u32) {
    *slot = 0;
}

/// `0x005c7910`: registers a callback with flags `0x400`; returns
/// `(flags, param)` forwarded to the registration routine.
pub fn w21a15_register_args(param: u32) -> (u32, u32) {
    (0x400, param)
}

/// `0x005c9400`: resets a tree header `[root, first, last, _, count]`.
/// A non-empty header (root != 0) is the pseudocode's non-returning error
/// path and yields `Err(())`.  `self_addr` is the header's own address.
pub fn w21a15_tree_reset(hdr: &mut [i32; 5], self_addr: i32) -> Result<(), ()> {
    if hdr[0] != 0 {
        return Err(());
    }
    hdr[0] = 0;
    hdr[1] = self_addr;
    hdr[2] = self_addr;
    hdr[4] = 0;
    Ok(())
}

/// `0x005c9280`: looks the key up with `find` (node or `None`) and stores
/// `value` at node offset `0x14`; a missing node is the error path.
pub fn w21a15_set_node_value(
    key: u32,
    value: u32,
    mut find: impl FnMut(u32) -> Option<usize>,
    nodes: &mut [u32],
) -> Result<(), ()> {
    let n = find(key).ok_or(())?;
    *nodes.get_mut(n).ok_or(())? = value;
    Ok(())
}

/// `0x005c9190`: after resetting the tree (`reset`), for each
/// `(key, value)` pair (stride 8) finds the node and stores the low 16 bits
/// of `value` at node offset `0x14`.  Missing node is the error path.
pub fn w21a15_fill_nodes(
    pairs: &[(u32, u32)],
    mut reset: impl FnMut(),
    mut find: impl FnMut(u32) -> Option<usize>,
    nodes: &mut [u16],
) -> Result<(), ()> {
    reset();
    for &(k, v) in pairs {
        let n = find(k).ok_or(())?;
        *nodes.get_mut(n).ok_or(())? = v as u16;
    }
    Ok(())
}

#[cfg(test)]
mod tests_wave21_agent15 {
    use super::*;

    #[test]
    fn checks() {
        assert!(w21a15_pair_check(1, 2, |_| true, || true));
        assert!(!w21a15_pair_check(2, 2, |_| true, || true));
        assert!(!w21a15_pair_check(1, 2, |_| false, || true));
        let mut o = [0u8; 0x50];
        assert!(!w21a15_flagged_check(1, 2, &o, || false, || true));
        o[0x4d] = 1;
        assert!(w21a15_flagged_check(1, 2, &o, || false, || true));
        assert!(!w21a15_flagged_check(1, 2, &o, || true, || true));
        assert!(!w21a15_flagged_check(1, 1, &o, || false, || true));
    }

    #[test]
    fn small() {
        let mut g = 5;
        w21a15_clear_global(&mut g);
        assert_eq!(g, 0);
        assert_eq!(w21a15_register_args(7), (0x400, 7));
    }

    #[test]
    fn tree() {
        let mut h = [0, 0, 0, 9, 3];
        assert!(w21a15_tree_reset(&mut h, 0x100).is_ok());
        assert_eq!(h, [0, 0x100, 0x100, 9, 0]);
        let mut h2 = [1, 0, 0, 0, 0];
        assert!(w21a15_tree_reset(&mut h2, 5).is_err());
    }

    #[test]
    fn nodes() {
        let mut n = [0u32; 3];
        assert!(w21a15_set_node_value(2, 9, |k| Some(k as usize), &mut n).is_ok());
        assert_eq!(n[2], 9);
        assert!(w21a15_set_node_value(2, 9, |_| None, &mut n).is_err());
        let mut m = [0u16; 3];
        let mut r = 0;
        assert!(
            w21a15_fill_nodes(
                &[(0, 0x1_0005), (1, 7)],
                || r += 1,
                |k| Some(k as usize),
                &mut m
            )
            .is_ok()
        );
        assert_eq!(m, [5, 7, 0]);
        assert_eq!(r, 1);
        assert!(w21a15_fill_nodes(&[(5, 1)], || {}, |_| None, &mut m).is_err());
    }
}
