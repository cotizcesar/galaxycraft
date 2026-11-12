#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent22.rs
// ===========================================================================
// Wave 24 agent 22: Cohen-Sutherland line clip against a rectangle.

fn w24a22_outcode(x: i16, y: i16, r: &[i16; 4]) -> u8 {
    let mut c = 0u8;
    if r[2] < x {
        c |= 2;
    }
    if y < r[1] {
        c |= 8;
    }
    if r[3] < y {
        c |= 4;
    }
    if x < r[0] {
        c |= 1;
    }
    c
}

fn w24a22_div(n: i32, d: i32) -> i16 {
    if d == 0 {
        0
    } else {
        (n.wrapping_div(d)) as i16
    }
}

/// `0x0076b970`: clips segment `p1`-`p2` (each `(x, y)`) to rect
/// `[left, top, right, bottom]`.  Returns the clipped endpoints, or `None`
/// when the segment is entirely outside.  Outcodes: 1 left, 2 right,
/// 4 bottom, 8 top.
pub fn w24a22_clip_line(
    p1: (i16, i16),
    p2: (i16, i16),
    r: &[i16; 4],
) -> Option<((i16, i16), (i16, i16))> {
    let (mut x1, mut y1) = p1;
    let (mut x2, mut y2) = p2;
    let mut c1 = w24a22_outcode(x1, y1, r);
    let mut c2 = w24a22_outcode(x2, y2, r);
    loop {
        if c1 == 0 && c2 == 0 {
            return Some(((x1, y1), (x2, y2)));
        }
        if c1 & c2 != 0 {
            return None;
        }
        let c = if c1 != 0 { c1 } else { c2 };
        let (nx, ny);
        if c & 8 != 0 {
            nx = w24a22_div(
                (r[1] as i32 - y1 as i32) * (x2 as i32 - x1 as i32),
                y2 as i32 - y1 as i32,
            )
            .wrapping_add(x1);
            ny = r[1];
        } else if c & 4 != 0 {
            nx = w24a22_div(
                (r[3] as i32 - y1 as i32) * (x2 as i32 - x1 as i32),
                y2 as i32 - y1 as i32,
            )
            .wrapping_add(x1);
            ny = r[3];
        } else {
            let edge = if c & 2 != 0 { r[2] } else { r[0] };
            ny = w24a22_div(
                (edge as i32 - x1 as i32) * (y2 as i32 - y1 as i32),
                x2 as i32 - x1 as i32,
            )
            .wrapping_add(y1);
            nx = edge;
        }
        if c == c1 {
            x1 = nx;
            y1 = ny;
            c1 = w24a22_outcode(x1, y1, r);
        } else {
            x2 = nx;
            y2 = ny;
            c2 = w24a22_outcode(x2, y2, r);
        }
    }
}

#[cfg(test)]
mod tests_wave24_agent22 {
    use super::*;

    #[test]
    fn clip_line_cases() {
        let r = [0, 0, 100, 100];
        assert_eq!(
            w24a22_clip_line((10, 10), (20, 20), &r),
            Some(((10, 10), (20, 20)))
        );
        assert_eq!(w24a22_clip_line((-10, 10), (-5, 20), &r), None);
        assert_eq!(
            w24a22_clip_line((-50, 50), (50, 50), &r),
            Some(((0, 50), (50, 50)))
        );
        assert_eq!(
            w24a22_clip_line((50, -50), (50, 150), &r),
            Some(((50, 0), (50, 100)))
        );
    }
}
