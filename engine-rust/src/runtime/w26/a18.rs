#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent18.rs
// ===========================================================================
/// `0x00943c50`: converts an integer rectangle `[left, top, right, bottom]`
/// into the float rectangle `[x, y, width, height]`.
pub fn w26a18_rect_to_xywh(r: [i32; 4]) -> [f32; 4] {
    [
        r[0] as f32,
        r[1] as f32,
        r[2] as f32 - r[0] as f32,
        r[3] as f32 - r[1] as f32,
    ]
}

/// `0x00946fc0`: builds the float rectangle `[x, y, w, h]` from an integer
/// origin and size (`w = (x + w) - x` computed in float).
pub fn w26a18_origin_size_to_xywh(x: i32, y: i32, w: i32, h: i32) -> [f32; 4] {
    [
        x as f32,
        y as f32,
        w.wrapping_add(x) as f32 - x as f32,
        h.wrapping_add(y) as f32 - y as f32,
    ]
}

/// `0x009442d0`: pixel-copy core of the image-to-bitmap helper.  Reads a
/// bottom-up source image (`width`, `height`, `bpp` = bytes per pixel, at
/// least 4) and writes rows top-down, swapping bytes 0 and 2 of each
/// pixel (BGRA <-> RGBA).  Returns `None` when buffers are too small.
pub fn w26a18_flip_swap_rb(src: &[u8], width: usize, height: usize, bpp: usize) -> Option<Vec<u8>> {
    if bpp < 4 || src.len() < width.checked_mul(height)?.checked_mul(bpp)? {
        return None;
    }
    let mut out = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let row = (height - y - 1) * bpp * width;
        for x in 0..width {
            let p = row + x * bpp;
            out.extend_from_slice(&[src[p + 2], src[p + 1], src[p], src[p + 3]]);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests_wave26_agent18 {
    use super::*;

    #[test]
    fn rects() {
        assert_eq!(w26a18_rect_to_xywh([1, 2, 11, 7]), [1.0, 2.0, 10.0, 5.0]);
        assert_eq!(w26a18_origin_size_to_xywh(3, 4, 5, 6), [3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn flip_swap() {
        let src = [1, 2, 3, 4, 5, 6, 7, 8];
        let out = w26a18_flip_swap_rb(&src, 1, 2, 4).unwrap();
        assert_eq!(out, vec![7, 6, 5, 8, 3, 2, 1, 4]);
        assert!(w26a18_flip_swap_rb(&src, 2, 2, 4).is_none());
    }
}
