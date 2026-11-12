#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent01.rs
// ===========================================================================
// Wave 25 translation batch 01: handle-lookup wrapper thunks.
//
// Each wrapper resolves a handle through a lookup callback that returns
// `0` on success, then forwards to a worker only on success.

/// `0x00984a80`: looks up `handle`; on success invokes the object's
/// virtual slot at offset `0x14` with argument `1`.  Returns whether the
/// call was made.
pub fn w25a01_lookup_call_slot14(
    handle: u32,
    lookup: impl FnOnce(u32) -> Result<u32, i32>,
    call_slot14: impl FnOnce(u32, u32),
) -> bool {
    match lookup(handle) {
        Ok(obj) => {
            call_slot14(obj, 1);
            true
        }
        Err(_) => false,
    }
}

/// `0x00984eb0`: if the lookup of `handle` returns 0, forwards the four
/// remaining arguments to the worker.
pub fn w25a01_lookup_forward4(
    handle: u32,
    args: [u32; 4],
    lookup: impl FnOnce(u32) -> i32,
    worker: impl FnOnce(u32, u32, u32, u32),
) -> bool {
    if lookup(handle) == 0 {
        worker(args[0], args[1], args[2], args[3]);
        true
    } else {
        false
    }
}

/// `0x00985060`: if the lookup of `handle` returns 0, calls the worker
/// with `arg`.
pub fn w25a01_lookup_forward1(
    handle: u32,
    arg: u32,
    lookup: impl FnOnce(u32) -> i32,
    worker: impl FnOnce(u32),
) -> bool {
    if lookup(handle) == 0 {
        worker(arg);
        true
    } else {
        false
    }
}

/// `0x00985090`: if the lookup of `handle` returns 0, calls the worker
/// with `(arg, 1)`.
pub fn w25a01_lookup_forward_true(
    handle: u32,
    arg: u32,
    lookup: impl FnOnce(u32) -> i32,
    worker: impl FnOnce(u32, u32),
) -> bool {
    if lookup(handle) == 0 {
        worker(arg, 1);
        true
    } else {
        false
    }
}

/// `0x009850f0`: if the lookup of `handle` returns 0, calls the worker
/// with `(arg, 0)`.
pub fn w25a01_lookup_forward_false(
    handle: u32,
    arg: u32,
    lookup: impl FnOnce(u32) -> i32,
    worker: impl FnOnce(u32, u32),
) -> bool {
    if lookup(handle) == 0 {
        worker(arg, 0);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests_wave25_agent01 {
    use super::*;

    #[test]
    fn slot14_only_on_success() {
        let mut got = None;
        assert!(w25a01_lookup_call_slot14(
            7,
            |_| Ok(42),
            |o, a| got = Some((o, a))
        ));
        assert_eq!(got, Some((42, 1)));
        let mut called = false;
        assert!(!w25a01_lookup_call_slot14(
            7,
            |_| Err(-1),
            |_, _| called = true
        ));
        assert!(!called);
    }

    #[test]
    fn forwards() {
        let mut v = vec![];
        assert!(w25a01_lookup_forward4(
            1,
            [1, 2, 3, 4],
            |_| 0,
            |a, b, c, d| v.push(a + b + c + d)
        ));
        assert!(!w25a01_lookup_forward4(
            1,
            [1, 2, 3, 4],
            |_| 5,
            |_, _, _, _| v.push(0)
        ));
        assert!(w25a01_lookup_forward1(1, 9, |_| 0, |a| v.push(a)));
        assert!(w25a01_lookup_forward_true(
            1,
            3,
            |_| 0,
            |a, b| v.push(a * 10 + b)
        ));
        assert!(w25a01_lookup_forward_false(
            1,
            3,
            |_| 0,
            |a, b| v.push(a * 10 + b)
        ));
        assert!(!w25a01_lookup_forward_false(1, 3, |_| 1, |_, _| v.push(99)));
        assert_eq!(v, vec![10, 9, 31, 30]);
    }
}
