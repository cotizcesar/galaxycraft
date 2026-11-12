#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent18.rs
// ===========================================================================
// Wave 28 agent 18: small render-glue helpers (quad/vertex record setup).

/// `0x00a89780`: width divided by the `u16` at `+0xe` (signed 16-bit at `+4`).
/// Returns `None` when the divisor is zero.
pub fn w28a18_cell_width(width: i16, divisor: u16) -> Option<i32> {
    if divisor == 0 {
        None
    } else {
        Some(i32::from(width) / i32::from(divisor))
    }
}

/// `0x00a8a800`: maps a normalised rect (x, y, w, h) to pixels.  Returns
/// `(pos, size)`; `scale` = (`DAT_011fe1e4`, `DAT_011fe1e8`), `origin` =
/// (`_DAT_011fe1dc`, `_DAT_011fe1e0`).
pub fn w28a18_rect_to_pixels(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    scale: (f32, f32),
    origin: (i32, i32),
) -> ([i32; 2], [i32; 2]) {
    let sw = (scale.0 * w) as i32;
    let sh = (scale.1 * h) as i32;
    let px = (scale.0 * x + origin.0 as f32 + sw as f32 * 0.5) as i32;
    let py = ((scale.1 * y + origin.1 as f32) - sh as f32 * 0.5) as i32;
    ([px, py], [sw, sh])
}

/// `0x00a8a8b0`: true when both `f` and `n` are strictly positive.
pub fn w28a18_both_positive(f: f32, n: i32) -> bool {
    0.0 < f && 0 < n
}

/// State block touched by `0x00a8ab00`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct W28a18FlagState {
    /// `DAT_00f3c9d1`.
    pub armed: bool,
    /// `DAT_00f3c9d2`.
    pub arg: u8,
    /// `DAT_00f3ca34` (float bits).
    pub value_bits: u32,
    /// `DAT_00f3ca38`.
    pub zero: u32,
}

/// `0x00a8ab00`: one-shot arm; sets the flag, stores `arg`, 1.0f and 0.
pub fn w28a18_arm_once(st: &mut W28a18FlagState, arg: u8) {
    if !st.armed {
        st.armed = true;
        st.arg = arg;
        st.value_bits = 0x3f80_0000;
        st.zero = 0;
    }
}

fn w28a18_set(rec: &mut [u32], off: usize, v: u32) {
    rec[off / 4] = v;
}

/// `0x00a8bd50`: after the quad record is built, stores the 12 colour words
/// (`p6..p17`) at `+0x60,+0x64,+0x68,+0x6c`, `+0x50..+0x5c`, `+0x70..+0x7c`.
pub fn w28a18_store_quad_colors(rec: &mut [u32], p: &[u32; 12]) {
    for (i, off) in [
        0x60, 0x64, 0x68, 0x6c, 0x50, 0x54, 0x58, 0x5c, 0x70, 0x74, 0x78, 0x7c,
    ]
    .into_iter()
    .enumerate()
    {
        w28a18_set(rec, off, p[i]);
    }
}

/// `0x00a8bde0`: stores 4 colour words at `+0x60,+0x64,+0x68,+0x6c`.
pub fn w28a18_store_rect_colors(rec: &mut [u32], c: &[u32; 4]) {
    for (i, off) in [0x60, 0x64, 0x68, 0x6c].into_iter().enumerate() {
        w28a18_set(rec, off, c[i]);
    }
}

/// `0x00a8c270`: normalises a vertical line to `(x, min(y0,y1), 0, y1max-ymin)`
/// (the inputs of the vertex builder); returns `(x, y_min, height)`.
pub fn w28a18_vline_extent(x: i32, y0: i32, y1: i32) -> (i32, i32, i32) {
    let lo = y0.min(y1);
    let hi = y0.max(y1);
    (x, lo, hi - lo)
}

/// `0x00a8c300`: copies words `+0x8,+0x18,+0x28` of `src` to `+0x8,+0xc,+0x10`
/// of `rec`.
pub fn w28a18_copy_tri(rec: &mut [u32], src: &[u32]) {
    rec[0x8 / 4] = src[0x8 / 4];
    rec[0xc / 4] = src[0x18 / 4];
    rec[0x10 / 4] = src[0x28 / 4];
}

/// `0x00a8c390`: like the triangle copy plus `+0x14` from `src+0x38`.
pub fn w28a18_copy_quad(rec: &mut [u32], src: &[u32]) {
    rec[0x8 / 4] = src[0x8 / 4];
    rec[0xc / 4] = src[0x18 / 4];
    rec[0x10 / 4] = src[0x28 / 4];
    rec[0x14 / 4] = src[0x38 / 4];
}

#[cfg(test)]
mod tests_wave28_agent18 {
    use super::*;

    #[test]
    fn width_div() {
        assert_eq!(w28a18_cell_width(-9, 2), Some(-4));
        assert_eq!(w28a18_cell_width(9, 0), None);
    }

    #[test]
    fn rect_pixels() {
        let (p, s) = w28a18_rect_to_pixels(0.5, 0.5, 0.25, 0.5, (100.0, 200.0), (10, 20));
        assert_eq!(s, [25, 100]);
        assert_eq!(p, [72, 70]);
    }

    #[test]
    fn positive() {
        assert!(w28a18_both_positive(0.1, 1));
        assert!(!w28a18_both_positive(0.0, 1));
        assert!(!w28a18_both_positive(1.0, 0));
    }

    #[test]
    fn arm() {
        let mut s = W28a18FlagState::default();
        w28a18_arm_once(&mut s, 7);
        w28a18_arm_once(&mut s, 9);
        assert_eq!((s.armed, s.arg, s.value_bits), (true, 7, 0x3f80_0000));
    }

    #[test]
    fn stores() {
        let mut r = [0u32; 0x20];
        let p: [u32; 12] = core::array::from_fn(|i| i as u32 + 1);
        w28a18_store_quad_colors(&mut r, &p);
        assert_eq!((r[0x60 / 4], r[0x50 / 4], r[0x7c / 4]), (1, 5, 12));
        let mut r2 = [0u32; 0x20];
        w28a18_store_rect_colors(&mut r2, &[1, 2, 3, 4]);
        assert_eq!(r2[0x6c / 4], 4);
    }

    #[test]
    fn line_and_copy() {
        assert_eq!(w28a18_vline_extent(3, 9, 4), (3, 4, 5));
        let mut src = [0u32; 0x10];
        for (i, v) in src.iter_mut().enumerate() {
            *v = i as u32 * 10;
        }
        let mut r = [0u32; 8];
        w28a18_copy_tri(&mut r, &src);
        assert_eq!((r[2], r[3], r[4], r[5]), (20, 60, 100, 0));
        w28a18_copy_quad(&mut r, &src);
        assert_eq!(r[5], 140);
    }
}
