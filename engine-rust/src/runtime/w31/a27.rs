#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent27.rs
// ===========================================================================
// Wave 31 translation batch 27: small range/forwarding helpers.

/// `0x005f3110`: forwards the (begin, end, extra) triple stored at the start
/// of the container to the callee `f`.
pub fn w31a27_forward_triple<R>(triple: &[u32; 3], f: impl FnOnce(u32, u32, u32) -> R) -> R {
    f(triple[0], triple[1], triple[2])
}

/// `0x005f3130`: same forwarding shape as the sibling, calling a different
/// callee with the (begin, end, extra) triple.
pub fn w31a27_forward_triple_b<R>(triple: &[u32; 3], f: impl FnOnce(u32, u32, u32) -> R) -> R {
    f(triple[0], triple[1], triple[2])
}

/// `0x005f3150`: walks the half-open range `[begin, end)` in `0x34` byte
/// strides and invokes the element destructor on each offset.
pub fn w31a27_destroy_range(begin: u32, end: u32, mut destroy: impl FnMut(u32)) {
    let mut p = begin;
    while p != end {
        destroy(p);
        p = p.wrapping_add(0x34);
    }
}

/// `0x005f4220`: stores the field at index 0x22 of the object into the
/// field at offset `0x11c` of the object it points to, then runs two
/// releases and a final cleanup.
pub fn w31a27_sync_and_release(
    target: &mut [u8],
    source_field: u32,
    mut release: impl FnMut(),
    cleanup: impl FnOnce(),
) {
    if target.len() >= 0x120 {
        target[0x11c..0x120].copy_from_slice(&source_field.to_le_bytes());
    }
    release();
    release();
    cleanup();
}

/// `0x005f4110`: runs two destructor steps in order.
pub fn w31a27_two_step(mut a: impl FnMut(), mut b: impl FnMut()) {
    a();
    b();
}

#[cfg(test)]
mod tests_wave31_agent27 {
    use super::*;

    #[test]
    fn forwards() {
        assert_eq!(w31a27_forward_triple(&[1, 2, 3], |a, b, c| a + b + c), 6);
        assert_eq!(w31a27_forward_triple_b(&[1, 2, 3], |a, b, c| a * b * c), 6);
    }

    #[test]
    fn range() {
        let mut v = vec![];
        w31a27_destroy_range(0x100, 0x100 + 0x68, |p| v.push(p));
        assert_eq!(v, vec![0x100, 0x134]);
        w31a27_destroy_range(5, 5, |_| panic!());
    }

    #[test]
    fn sync() {
        let mut t = [0u8; 0x130];
        let mut n = 0;
        let mut done = false;
        w31a27_sync_and_release(&mut t, 0xAABBCCDD, || n += 1, || done = true);
        assert_eq!(&t[0x11c..0x120], &[0xDD, 0xCC, 0xBB, 0xAA]);
        assert_eq!(n, 2);
        assert!(done);
        let mut order = vec![];
        let o = std::cell::RefCell::new(&mut order);
        w31a27_two_step(|| o.borrow_mut().push(1), || o.borrow_mut().push(2));
        assert_eq!(order, vec![1, 2]);
    }
}
