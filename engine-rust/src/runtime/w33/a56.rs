#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent56.rs
// ===========================================================================
// Wave 33 agent 56: CRT locale helpers.

/// `0x00cac41d`: runs the append helper `0xca_1200` once per extra
/// argument (`count` of them); the first nonzero status makes the original
/// call a non-returning abort, modelled here as `Err(index)`.
pub fn w33a56_append_all<F: FnMut(usize) -> i32>(count: i32, mut step: F) -> Result<(), usize> {
    let mut i = 0usize;
    while (i as i32) < count {
        if step(i) != 0 {
            return Err(i);
        }
        i += 1;
    }
    Ok(())
}

/// `0x00cacfd6`: when `ptr` is nonzero, runs the teardown helper and then
/// the free helper on it; returns whether anything was released.
pub fn w33a56_destroy_and_free<D: FnMut(usize), R: FnMut(usize)>(
    ptr: usize,
    mut teardown: D,
    mut free: R,
) -> bool {
    if ptr == 0 {
        return false;
    }
    teardown(ptr);
    free(ptr);
    true
}

#[cfg(test)]
mod tests_wave33_agent56 {
    use super::*;

    #[test]
    fn append_all_stops_on_error() {
        let mut seen = vec![];
        let r = w33a56_append_all(4, |i| {
            seen.push(i);
            (i == 2) as i32
        });
        assert_eq!(r, Err(2));
        assert_eq!(seen, vec![0, 1, 2]);
        assert_eq!(w33a56_append_all(0, |_| 1), Ok(()));
    }

    #[test]
    fn destroy_order() {
        let mut log = vec![];
        let log_ref = std::cell::RefCell::new(&mut log);
        assert!(w33a56_destroy_and_free(
            5,
            |p| log_ref.borrow_mut().push(('d', p)),
            |p| log_ref.borrow_mut().push(('f', p))
        ));
        assert!(!w33a56_destroy_and_free(0, |_| {}, |_| {}));
        assert_eq!(log, vec![('d', 5), ('f', 5)]);
    }
}
