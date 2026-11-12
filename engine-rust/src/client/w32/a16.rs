#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent16.rs
// ===========================================================================
// Wave 32 agent 16: small call-sequence helpers modelled with closures.

/// `0x009196c0`: teardown sequence; runs the first destructor twice, then the second step.
pub fn w32a16_teardown(mut a: impl FnMut(), mut b: impl FnMut()) {
    a();
    a();
    b();
}

/// `0x00923510`: if the probe reports non-zero, runs the secondary step; always runs the final step.
pub fn w32a16_conditional_cleanup(
    probe: impl FnOnce() -> u8,
    mut step: impl FnMut(),
    mut fin: impl FnMut(),
) {
    if probe() != 0 {
        step();
    }
    fin();
}

/// `0x009321c0`: tries the first matcher on a copy of `*cursor`; on success stores the
/// advanced cursor and returns 0.  Otherwise returns 1 when `allow_fallback == 0`, else
/// runs the fallback and returns 0 if it succeeded or -1 if it failed.
pub fn w32a16_try_match(
    cursor: &mut u32,
    allow_fallback: i32,
    first: impl FnOnce(&mut u32) -> i32,
    fallback: impl FnOnce(&mut u32) -> i32,
) -> i32 {
    let mut cur = *cursor;
    if first(&mut cur) == 0 {
        *cursor = cur;
        return 0;
    }
    if allow_fallback == 0 {
        return 1;
    }
    (fallback(cursor) != 0) as i32 - 1
}

#[cfg(test)]
mod tests_wave32_agent16 {
    use super::*;

    #[test]
    fn teardown_order() {
        let mut log = vec![];
        let l = std::cell::RefCell::new(&mut log);
        w32a16_teardown(|| l.borrow_mut().push(1), || l.borrow_mut().push(2));
        assert_eq!(log, vec![1, 1, 2]);
    }

    #[test]
    fn cleanup() {
        let mut n = 0;
        w32a16_conditional_cleanup(|| 0, || n += 10, || {});
        assert_eq!(n, 0);
        w32a16_conditional_cleanup(|| 1, || n += 10, || {});
        assert_eq!(n, 10);
    }

    #[test]
    fn matching() {
        let mut c = 5;
        assert_eq!(
            w32a16_try_match(
                &mut c,
                0,
                |p| {
                    *p = 9;
                    0
                },
                |_| 1
            ),
            0
        );
        assert_eq!(c, 9);
        assert_eq!(w32a16_try_match(&mut c, 0, |_| 1, |_| 0), 1);
        assert_eq!(w32a16_try_match(&mut c, 1, |_| 1, |_| 0), -1);
        assert_eq!(w32a16_try_match(&mut c, 1, |_| 1, |_| 1), 0);
    }
}
