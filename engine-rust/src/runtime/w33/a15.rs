#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent15.rs
// ===========================================================================
// Wave 33 agent 15: small widget dispatch helpers.

/// `0x00851020`: if the predicate accepts `(a, b)`, forwards the call to the
/// owner's virtual slot at +0x38 and returns true; otherwise returns false.
pub fn w33a15_dispatch_if_accepted<P, F>(a: u32, b: u32, c: u64, accept: P, mut forward: F) -> bool
where
    P: FnOnce(u32, u32) -> bool,
    F: FnMut(u32, u32, u64),
{
    if accept(a, b) {
        forward(a, b, c);
        true
    } else {
        false
    }
}

/// `0x0087c750`: compares the four shorts at record offset +4..+12 with the
/// new rectangle; when any differs, stores it, calls `notify(old_x_y,
/// old_w_h)` (the two old dwords at +4 and +8) and returns true.
pub fn w33a15_update_rect<F>(rec: &mut [i16; 4], new: [i16; 4], mut notify: F) -> bool
where
    F: FnMut([i16; 2], [i16; 2]),
{
    if *rec == new {
        return false;
    }
    let old = *rec;
    *rec = new;
    notify([old[0], old[1]], [old[2], old[3]]);
    true
}

#[cfg(test)]
mod tests_wave33_agent15 {
    use super::*;

    #[test]
    fn dispatch() {
        let mut n = 0;
        assert!(w33a15_dispatch_if_accepted(
            1,
            2,
            3,
            |a, b| a < b,
            |_, _, _| n += 1
        ));
        assert!(!w33a15_dispatch_if_accepted(
            2,
            1,
            3,
            |a, b| a < b,
            |_, _, _| n += 1
        ));
        assert_eq!(n, 1);
    }

    #[test]
    fn rect() {
        let mut r = [1, 2, 3, 4];
        let mut got = None;
        assert!(!w33a15_update_rect(&mut r, [1, 2, 3, 4], |_, _| panic!()));
        assert!(w33a15_update_rect(&mut r, [1, 2, 3, 5], |a, b| got = Some((a, b))));
        assert_eq!(got, Some(([1, 2], [3, 4])));
        assert_eq!(r, [1, 2, 3, 5]);
    }
}
