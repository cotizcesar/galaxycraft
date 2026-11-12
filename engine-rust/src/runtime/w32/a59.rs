#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent59.rs
// ===========================================================================
// Wave 32 translation batch 59: small forwarding helpers with callees
// supplied by the caller.

/// `0x008c5190`: if the event dispatcher (`FUN_0087c220`) does not handle
/// the event (returns 0), forwards the event to the fallback handler
/// (`FUN_0097e630`) using the object's field at offset 4.
pub fn w32a59_dispatch_or_forward<E: Copy, T: Copy>(
    event: E,
    target: T,
    dispatch: impl FnOnce(E) -> u8,
    forward: impl FnOnce(T, E),
) -> bool {
    if dispatch(event) == 0 {
        forward(target, event);
        true
    } else {
        false
    }
}

/// `0x008c6900`: thin forwarder of `(x, y)` to the point-in-rect test.
pub fn w32a59_forward_point_test(x: i32, y: i32, test: impl FnOnce(i32, i32) -> bool) -> bool {
    test(x, y)
}

/// `0x008df83e`: runs three teardown steps in order; the middle one takes 0.
pub fn w32a59_teardown_sequence(
    first: impl FnOnce(),
    second: impl FnOnce(u32),
    third: impl FnOnce(),
) {
    first();
    second(0);
    third();
}

#[cfg(test)]
mod tests_wave32_agent59 {
    #[test]
    fn dispatch() {
        let mut log = vec![];
        assert!(super::w32a59_dispatch_or_forward(
            1,
            2,
            |_| 0,
            |t, e| log.push((t, e))
        ));
        assert_eq!(log, vec![(2, 1)]);
        assert!(!super::w32a59_dispatch_or_forward(
            1,
            2,
            |_| 1,
            |_, _| panic!()
        ));
    }
    #[test]
    fn point() {
        assert!(super::w32a59_forward_point_test(3, 4, |x, y| x + y == 7));
    }
    #[test]
    fn teardown() {
        let log = std::cell::RefCell::new(vec![]);
        super::w32a59_teardown_sequence(
            || log.borrow_mut().push(1),
            |v| log.borrow_mut().push(10 + v),
            || log.borrow_mut().push(3),
        );
        assert_eq!(*log.borrow(), vec![1, 10, 3]);
    }
}
