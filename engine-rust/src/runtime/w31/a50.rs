#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent50.rs
// ===========================================================================
// Wave 31 agent 50.

/// `0x008c6bb0`: point-in-rectangle test.  The rectangle (left, top, right,
/// bottom as i16) and an offset (x, y as i16) come from two callees; the
/// point is inside when `left+ox <= x < right+ox` and `top+oy <= y < bottom+oy`
/// (16-bit wrapping sums).
pub fn w31a50_point_in_offset_rect(rect: [i16; 4], offset: [i16; 2], x: i32, y: i32) -> bool {
    let l = rect[0].wrapping_add(offset[0]) as i32;
    let t = rect[1].wrapping_add(offset[1]) as i32;
    let r = rect[2].wrapping_add(offset[0]) as i32;
    let b = rect[3].wrapping_add(offset[1]) as i32;
    l <= x && t <= y && x < r && y < b
}

#[cfg(test)]
mod tests_wave31_agent50 {
    use super::*;

    #[test]
    fn rect_bounds() {
        let r = [10, 20, 30, 40];
        let o = [1, 2];
        assert!(w31a50_point_in_offset_rect(r, o, 11, 22));
        assert!(!w31a50_point_in_offset_rect(r, o, 10, 22));
        assert!(!w31a50_point_in_offset_rect(r, o, 31, 22));
        assert!(w31a50_point_in_offset_rect(r, o, 30, 41));
        assert!(!w31a50_point_in_offset_rect(r, o, 30, 42));
    }
}
