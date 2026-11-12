#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent53.rs
// ===========================================================================
// Wave 34 agent 53: small format-spec helpers.

/// `0x00c98db5`: format-conversion predicate.  Modes 2 and 13 give false,
/// modes 3 and 12 give true, otherwise true unless `spec` is `c` or `s`.
pub fn w34a53_spec_flag(spec: u8, mode: i32) -> bool {
    match mode {
        2 | 13 => false,
        3 | 12 => true,
        _ => spec != b'c' && spec != b's',
    }
}

/// `0x00c98e34`: returns `callee() + base`, where `base` is the object's
/// field at offset 0x404 or, when that is zero, the object address itself.
pub fn w34a53_offset_base(field_404: i32, this: i32, callee: impl FnOnce() -> i32) -> i32 {
    let r = callee();
    let base = if field_404 == 0 { this } else { field_404 };
    r.wrapping_add(base)
}

/// `0x00c990e7`: calls `step` up to `count` times, stopping early once
/// `*state` becomes -1 after a call.
pub fn w34a53_repeat_put(ch: u8, count: i32, state: &mut i32, mut step: impl FnMut(i16, &mut i32)) {
    let mut i = 0;
    while i < count {
        step(ch as i8 as i16, state);
        if *state == -1 {
            return;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests_wave34_agent53 {
    use super::*;

    #[test]
    fn flag() {
        assert!(!w34a53_spec_flag(b'x', 2));
        assert!(w34a53_spec_flag(b'c', 3));
        assert!(!w34a53_spec_flag(b'x', 13));
        assert!(!w34a53_spec_flag(b's', 5));
        assert!(w34a53_spec_flag(b'd', 5));
    }

    #[test]
    fn base() {
        assert_eq!(w34a53_offset_base(0, 100, || 5), 105);
        assert_eq!(w34a53_offset_base(50, 100, || 5), 55);
    }

    #[test]
    fn repeat() {
        let mut n = 0;
        let mut st = 0;
        w34a53_repeat_put(b'a', 5, &mut st, |_, s| {
            n += 1;
            if n == 3 {
                *s = -1;
            }
        });
        assert_eq!(n, 3);
    }
}
