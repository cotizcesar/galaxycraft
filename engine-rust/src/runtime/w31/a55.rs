#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent55.rs
// ===========================================================================
// Wave 31 agent 55: virtual-dispatch glue (runtime.rs).

/// `0x00918660`: calls virtual slot `0x9c`, then if the predicate (callee
/// FUN_00880d90) is false returns 1000; otherwise calls virtual slot `0xb4`,
/// hands the owner's second field to the callback and returns 0.
pub fn w31a55_dispatch_or_retry(
    slot_9c: &mut dyn FnMut(),
    predicate: &mut dyn FnMut() -> bool,
    slot_b4: &mut dyn FnMut(),
    finish: &mut dyn FnMut(),
) -> u32 {
    slot_9c();
    if !predicate() {
        return 1000;
    }
    slot_b4();
    finish();
    0
}

/// `0x009233d0`: calls virtual slot `0xa4`, then the follow-up helper.
pub fn w31a55_call_a4_then_follow(slot_a4: &mut dyn FnMut(), follow: &mut dyn FnMut()) {
    slot_a4();
    follow();
}

#[cfg(test)]
mod tests_wave31_agent55 {
    use super::*;

    #[test]
    fn retry_when_predicate_false() {
        let mut n = 0;
        let r = w31a55_dispatch_or_retry(&mut || n += 1, &mut || false, &mut || {}, &mut || {});
        assert_eq!(r, 1000);
        assert_eq!(n, 1);
    }

    #[test]
    fn done_when_predicate_true() {
        let mut order = Vec::new();
        let o = std::cell::RefCell::new(&mut order);
        let r = w31a55_dispatch_or_retry(
            &mut || o.borrow_mut().push(1),
            &mut || true,
            &mut || o.borrow_mut().push(2),
            &mut || o.borrow_mut().push(3),
        );
        assert_eq!(r, 0);
        assert_eq!(*o.borrow(), &vec![1, 2, 3]);
    }

    #[test]
    fn a4_order() {
        let mut v = 0;
        let c = std::cell::Cell::new(0);
        w31a55_call_a4_then_follow(&mut || c.set(1), &mut || v = c.get() + 1);
        assert_eq!(v, 2);
    }
}
