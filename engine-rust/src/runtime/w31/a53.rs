#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent53.rs
// ===========================================================================
// Wave 31 agent 53: dialog press hit-test.

/// Widget state written by `0x0090c700` (offsets 0x1c0/0x1c4/0x1c8).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct W31a53Press {
    pub active: u8,
    pub x: i32,
    pub y: i32,
}

/// `0x0090c700`: after the caller-supplied pre-hook, reads a rect
/// `(left, top, right)` (shorts) and, when the point lies strictly inside
/// (`left < x < right - 0x14`, `top < y < top + 0xf`), records the press.
/// The post-hook runs afterwards unconditionally.
pub fn w31a53_press_hit(
    st: &mut W31a53Press,
    x: i32,
    y: i32,
    pre: impl FnOnce(),
    rect: impl FnOnce() -> (i16, i16, i16),
    post: impl FnOnce(),
) {
    pre();
    let (left, top, right) = rect();
    if (left as i32) < x && x < right as i32 - 0x14 && (top as i32) < y && y < top as i32 + 0xf {
        st.active = 1;
        st.x = x;
        st.y = y;
    }
    post();
}

#[cfg(test)]
mod tests_wave31_agent53 {
    use super::*;

    #[test]
    fn hit_and_miss() {
        let mut s = W31a53Press::default();
        w31a53_press_hit(&mut s, 50, 10, || {}, || (0, 5, 100), || {});
        assert_eq!(
            s,
            W31a53Press {
                active: 1,
                x: 50,
                y: 10
            }
        );
        let mut m = W31a53Press::default();
        w31a53_press_hit(&mut m, 90, 10, || {}, || (0, 5, 100), || {});
        assert_eq!(m.active, 0);
    }
}
