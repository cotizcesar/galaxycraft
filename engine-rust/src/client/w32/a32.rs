#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent32.rs
// ===========================================================================
// Wave 32 translation batch 32.
//
// Skipped: FUN_00a8dca0 (SEH prologue ending in a non-returning call),
// FUN_00a9abf0 (GL function-pointer calls through unmodelled imports).

/// Result of the letterbox / texture-tile computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W32a32Layout {
    pub tex_size: (i32, i32),
    pub aspect_margin: (i32, i32),
    pub four_three_margin: (i32, i32),
    pub tile: u32,
}

fn w32a32_round_pow2(v: u32) -> u32 {
    if v == 1 || (v & v.wrapping_sub(1)) == 0 {
        return v;
    }
    if (v as i32) < 0 {
        return 0;
    }
    let mut x = v.wrapping_sub(1) as i32;
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x |= x >> 8;
    (((x >> 16) | x) as u32).wrapping_add(1)
}

/// `0x00a98370`: computes aspect-ratio letterbox margins for a
/// `width` x `height` surface against a `cols` x `rows` grid, the 4:3
/// margins, and the tile size (power of two clamped to 2..=4, optionally
/// limited by `max_tile`).
pub fn w32a32_layout(width: i32, height: i32, cols: i32, rows: i32, max_tile: u32) -> W32a32Layout {
    let (mut mx, mut my) = (0i32, 0i32);
    let w = width as f64;
    let h = height as f64;
    let target = cols as f64 / rows as f64;
    let ratio = w / h;
    if (ratio - target).abs() > 0.001 {
        if ratio > target {
            mx = (width - (h * target) as i32) / 2;
        } else if ratio < target {
            my = (height - ((1.0 / target) * w) as i32) / 2;
        }
    }
    let (mut fx, mut fy) = (0i32, 0i32);
    let r43 = 1.3333333333333333f64;
    if (ratio - r43).abs() > 0.001 {
        if ratio > r43 {
            fx = (width - (h * r43) as i32) / 2;
        } else if ratio < r43 {
            fy = (height - (w * 0.75) as i32) / 2;
        }
    }
    let a = w32a32_round_pow2(((width as f32 / cols as f32) as f64).ceil() as i32 as u32);
    let b = w32a32_round_pow2(((height as f32 / rows as f32) as f64).ceil() as i32 as u32);
    let mut t = if (b as i32) < (a as i32) { a } else { b } as i32;
    if t < 5 {
        if t < 3 {
            t = 2;
        }
    } else {
        t = 4;
    }
    let mut tile = t as u32;
    if 0 < max_tile as i32 && (max_tile as i32) < t {
        tile = max_tile;
    }
    W32a32Layout {
        tex_size: (tile as i32 * cols, tile as i32 * rows),
        aspect_margin: (mx, my),
        four_three_margin: (fx, fy),
        tile,
    }
}

#[cfg(test)]
mod tests_wave32_agent32 {
    use super::*;

    #[test]
    fn square_pixels_4_3() {
        let l = w32a32_layout(640, 480, 640, 480, 0);
        assert_eq!(l.aspect_margin, (0, 0));
        assert_eq!(l.four_three_margin, (0, 0));
        assert_eq!(l.tile, 2);
        assert_eq!(l.tex_size, (1280, 960));
    }

    #[test]
    fn widescreen_margins_and_clamp() {
        let l = w32a32_layout(1920, 1080, 640, 480, 1);
        assert_eq!(l.aspect_margin, ((1920 - 1440) / 2, 0));
        assert_eq!(l.four_three_margin, (240, 0));
        assert_eq!(l.tile, 1);
    }

    #[test]
    fn big_scale_caps_at_four() {
        let l = w32a32_layout(6400, 4800, 640, 480, 0);
        assert_eq!(l.tile, 4);
    }
}
