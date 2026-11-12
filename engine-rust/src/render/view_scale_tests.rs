use super::{Framebuffer, Rgb, box_blur, downsample_rgba, stretch_columns};
use crate::dds::DdsImage;

#[test]
fn the_853_px_view_is_stretched_across_the_2560_px_box() {
    // 2559 drawn columns (853 game px at 3x): black, then white from
    // source x 2000 on; column 2559 was never drawn.
    let mut fb = Framebuffer::new(2560, 1);
    for x in 0..2559 {
        let v = if x < 2000 { 0 } else { 255 };
        fb.put(x, 0, Rgb::new(v, v, v));
    }
    fb.put(2559, 0, Rgb::new(255, 0, 0));
    stretch_columns(&mut fb, (0, 0, 2560, 1), 2559);
    let at = |x: i32| fb.get(x, 0);
    // The edge moves ~0.78 px right (retail drift measured on the
    // terrain: 0.95 px at x 2200, 1.00 at 2360): output 2000 samples
    // source 2000.5 * 2559 / 2560 - 0.5 = 1999.22 (a 22 % blend, 56)
    // instead of the white texel; the 50 % point moves to 2000.28.
    assert_eq!(at(1999), Rgb::new(0, 0, 0));
    assert_eq!(at(2000), Rgb::new(56, 56, 56));
    assert_eq!(at(2001), Rgb::new(255, 255, 255));
    // The undrawn column is replaced by the last drawn one.
    assert_eq!(at(2559), Rgb::new(255, 255, 255));
    // Equal widths are a no-op.
    let mut same = Framebuffer::new(4, 1);
    same.put(1, 0, Rgb::new(9, 9, 9));
    stretch_columns(&mut same, (0, 0, 4, 1), 4);
    assert_eq!(same.get(1, 0), Rgb::new(9, 9, 9));
}

fn image(w: usize, h: usize, px: &[u32]) -> DdsImage {
    DdsImage {
        width: w,
        height: h,
        pixels: px.to_vec(),
    }
}

#[test]
fn hd_tiles_are_area_filtered_with_straight_alpha_out() {
    // 4 texels -> 3 px (the 128 -> 96 ratio): coverage 1 + 1/3,
    // 2/3 + 2/3, 1/3 + 1.  The old integer box gave 0, 255, 127.
    let g = |v: u32| 0xff00_0000 | (v << 16) | (v << 8) | v;
    let img = image(4, 1, &[g(0), g(255), g(0), g(255)]);
    let out = downsample_rgba(&img, 3, 1);
    let red = |p: u32| (p >> 16) & 0xff;
    // The middle pixel is exactly 127.5, so allow either rounding.
    let got: Vec<i32> = out.iter().map(|&p| red(p) as i32).collect();
    for (g, want) in got.iter().zip([64, 128, 191]) {
        assert!((g - want).abs() <= 1, "{got:?}");
    }
    assert!(out.iter().all(|&p| p >> 24 == 255));
    // A half-transparent texel keeps its colour (blit_tile blends with
    // straight alpha); the old path stored the premultiplied 100.
    let img = image(1, 1, &[(128 << 24) | (200 << 16) | (200 << 8) | 200]);
    let out = downsample_rgba(&img, 1, 1);
    assert_eq!((out[0] >> 24, red(out[0])), (128, 200));
}

#[test]
fn the_fog_edge_is_a_linear_ramp() {
    // Step from 0 to 255 between samples 19 and 20 of a 40-wide row.
    let (w, h) = (40usize, 1usize);
    let mut v: Vec<f32> = (0..w).map(|x| if x < 20 { 0.0 } else { 255.0 }).collect();
    box_blur(&mut v, w, h, 5);
    // 11-sample ramp: 0 up to x 14, then equal steps of 255 / 11.
    assert_eq!(v[14], 0.0);
    let step = 255.0 / 11.0;
    for x in 15..25 {
        assert!((v[x] - v[x - 1] - step).abs() < 1e-3, "x {x}");
    }
    assert!((v[25] - 255.0).abs() < 1e-3);
}
