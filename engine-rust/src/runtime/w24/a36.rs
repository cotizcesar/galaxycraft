#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent36.rs
// ===========================================================================
// Wave 24 agent 36: small container / refcount helpers (runtime.rs target).

/// `0x007ab170`: resets list slot `index` of the 12-byte list-header table.
/// Optionally calls the provider release (slot vtable +0x18) when both
/// globals are set, then re-links the sentinel to itself and zeroes the
/// size.  Returns true when the old tail link was not the sentinel (the
/// retail code then walks and frees the nodes).
pub fn w24a36_reset_list_slot(
    heads: &mut [[u32; 3]],
    index: usize,
    sentinel_addr: u32,
    provider_ready: bool,
    release: &mut dyn FnMut(usize),
) -> bool {
    if provider_ready {
        release(index);
    }
    let old_tail = heads[index][1];
    heads[index][1] = sentinel_addr;
    heads[index][0] = sentinel_addr;
    heads[index][2] = 0;
    old_tail != sentinel_addr
}

/// `0x007ab770`: resets all eight list slots in order.
pub fn w24a36_reset_all_lists(reset: &mut dyn FnMut(usize)) {
    for i in 0..8 {
        reset(i);
    }
}

/// `0x007ac414`: zero-initialises a six-word record.
pub fn w24a36_zero_record(rec: &mut [u32; 6]) {
    *rec = [0; 6];
}

/// `0x007ac360`: drops one strong reference of a control block
/// (`counts[0]` strong at +4, `counts[1]` weak at +8).  When the strong
/// count reaches zero `dispose` runs (vtable +8) and one weak reference is
/// dropped; when that reaches zero `destroy` runs (vtable +0xc).
pub fn w24a36_release_ref(
    counts: &mut [i32; 2],
    dispose: &mut dyn FnMut(),
    destroy: &mut dyn FnMut(),
) {
    counts[0] -= 1;
    if counts[0] == 0 {
        dispose();
        counts[1] -= 1;
        if counts[1] == 0 {
            destroy();
        }
    }
}

#[cfg(test)]
mod tests_wave24_agent36 {
    use super::*;

    #[test]
    fn reset_slot() {
        let mut h = [[1, 2, 3]; 8];
        let mut calls = vec![];
        let r = w24a36_reset_list_slot(&mut h, 2, 100, true, &mut |i| calls.push(i));
        assert!(r);
        assert_eq!(h[2], [100, 100, 0]);
        assert_eq!(calls, vec![2]);
        let r = w24a36_reset_list_slot(&mut h, 2, 100, false, &mut |_| panic!());
        assert!(!r);
    }

    #[test]
    fn reset_all() {
        let mut v = vec![];
        w24a36_reset_all_lists(&mut |i| v.push(i));
        assert_eq!(v, (0..8).collect::<Vec<_>>());
    }

    #[test]
    fn zero() {
        let mut r = [7; 6];
        w24a36_zero_record(&mut r);
        assert_eq!(r, [0; 6]);
    }

    #[test]
    fn release() {
        let mut c = [2, 1];
        let (mut d, mut x) = (0, 0);
        w24a36_release_ref(&mut c, &mut || d += 1, &mut || x += 1);
        assert_eq!((d, x), (0, 0));
        w24a36_release_ref(&mut c, &mut || d += 1, &mut || x += 1);
        assert_eq!((d, x), (1, 1));
    }
}
