#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent39.rs
// ===========================================================================
// Wave 34 agent 39: dialog widget geometry setters (rect at +4, kind at +0x3c).

/// Widget state touched by the setters: rect (+4..+0xc) as left/top/right/bottom,
/// kind (+0x3c), and the kind-0xe shadow fields.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct w34a39_Widget {
    pub rect: [i16; 4],
    pub kind: i16,
    pub f50: i16,
    pub f52: i16,
    pub f54: i16,
    pub f56: i16,
    pub f60: i16,
    pub f62: i16,
    pub f64: i16,
    pub f66: i16,
}

/// `0x0087c530`: sets the widget height; kind 0xe also stores `f62 + h` in
/// `f66`.  Returns the rect handed to the (unmodelled) apply helper:
/// bottom = top + h.
pub fn w34a39_set_height(w: &mut w34a39_Widget, h: i16) -> [i16; 4] {
    if w.kind == 0xe {
        w.f66 = w.f62.wrapping_add(h);
    }
    [w.rect[0], w.rect[1], w.rect[2], w.rect[1].wrapping_add(h)]
}

/// `0x0087c650`: moves the widget so left equals `x`; kind 0xe shifts
/// f60/f64/f50/f54 by the delta.  Returns the shifted rect.
pub fn w34a39_set_left(w: &mut w34a39_Widget, x: i16) -> [i16; 4] {
    let d = x.wrapping_sub(w.rect[0]);
    if w.kind == 0xe {
        w.f60 = w.f60.wrapping_add(d);
        w.f64 = w.f64.wrapping_add(d);
        w.f50 = w.f50.wrapping_add(d);
        w.f54 = w.f54.wrapping_add(d);
    }
    [
        w.rect[0].wrapping_add(d),
        w.rect[1],
        w.rect[2].wrapping_add(d),
        w.rect[3],
    ]
}

/// `0x0087c6d0`: moves the widget so top equals `y`; kind 0xe shifts
/// f62/f66/f52/f56 by the delta.  Returns the shifted rect.
pub fn w34a39_set_top(w: &mut w34a39_Widget, y: i16) -> [i16; 4] {
    let d = y.wrapping_sub(w.rect[1]);
    if w.kind == 0xe {
        w.f62 = w.f62.wrapping_add(d);
        w.f66 = w.f66.wrapping_add(d);
        w.f52 = w.f52.wrapping_add(d);
        w.f56 = w.f56.wrapping_add(d);
    }
    [
        w.rect[0],
        w.rect[1].wrapping_add(d),
        w.rect[2],
        w.rect[3].wrapping_add(d),
    ]
}

#[cfg(test)]
mod tests_wave34_agent39 {
    use super::*;

    fn wd(kind: i16) -> w34a39_Widget {
        w34a39_Widget {
            rect: [10, 20, 50, 60],
            kind,
            f50: 1,
            f52: 2,
            f54: 3,
            f56: 4,
            f60: 5,
            f62: 6,
            f64: 7,
            f66: 8,
        }
    }

    #[test]
    fn height() {
        let mut w = wd(0xe);
        assert_eq!(w34a39_set_height(&mut w, 15), [10, 20, 50, 35]);
        assert_eq!(w.f66, 21);
        let mut w = wd(1);
        w34a39_set_height(&mut w, 15);
        assert_eq!(w.f66, 8);
    }

    #[test]
    fn left_top() {
        let mut w = wd(0xe);
        assert_eq!(w34a39_set_left(&mut w, 15), [15, 20, 55, 60]);
        assert_eq!((w.f60, w.f64, w.f50, w.f54), (10, 12, 6, 8));
        let mut w = wd(0xe);
        assert_eq!(w34a39_set_top(&mut w, 0), [10, 0, 50, 40]);
        assert_eq!((w.f62, w.f66, w.f52, w.f56), (-14, -12, -18, -16));
        let mut w = wd(2);
        assert_eq!(w34a39_set_top(&mut w, 25)[1], 25);
        assert_eq!(w.f62, 6);
    }
}
