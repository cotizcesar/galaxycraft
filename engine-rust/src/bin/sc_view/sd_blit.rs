//! The SD view: the 1x world frame scaled up to the window.

use super::*;

pub(super) fn blit_scaled_at(
    src: &Framebuffer,
    dst: &mut Framebuffer,
    scale: f32,
    x0: i32,
    filter: f32,
) {
    let s = scale.max(0.5);
    let dw = ((src.width as f32 * s).round() as i32).min(dst.width);
    let dh = ((src.height as f32 * s).round() as i32).min(dst.height);
    if dw <= 0 || dh <= 0 || src.width <= 0 || src.height <= 0 {
        return;
    }
    let (sw, sh) = (src.width as usize, src.height as usize);
    let max_sx = (src.width - 1) as f32;
    let max_sy = (src.height - 1) as f32;
    // Destination columns that land on the screen, with their source
    // columns and 8-bit horizontal weights (computed once per frame, not
    // per pixel; the per-pixel float maths made a 1440p SD frame ~70 ms).
    let dst_w = dst.width as usize;
    let cols: Vec<(usize, usize, usize, u32)> = (0..dw)
        .filter_map(|x| {
            let dx = x0 + x;
            (dx >= 0 && dx < dst.width).then_some((x, dx as usize))
        })
        .map(|(x, dx)| {
            let fx = (((x as f32 + 0.5) / s) - 0.5).clamp(0.0, max_sx);
            let a = fx.floor() as usize;
            let b = (a + 1).min(sw - 1);
            (dx, a, b, ((fx - a as f32) * 256.0) as u32)
        })
        .collect();
    // The Video tab's SD Graphics Filter: the sharp end is nearest-neighbour
    // (the classic 640x480 look), while the filtered stops use the bilinear
    // upscale measured on the retail HD screenshots.
    let sharp = filter <= 0.01;
    let pixels = &src.pixels;
    let rows = &mut dst.pixels[..dh as usize * dst_w];
    galaxycraft::render::par_bands(rows, dst_w, |first, band| {
        for (k, row) in band.chunks_exact_mut(dst_w).enumerate() {
            let y = (first + k) as f32;
            if sharp {
                let sy = (y / s).floor().clamp(0.0, max_sy) as usize * sw;
                for &(dx, _, _, _) in &cols {
                    let sx = (((dx as i32 - x0) as f32) / s).floor().clamp(0.0, max_sx) as usize;
                    row[dx] = pixels[sy + sx];
                }
                continue;
            }
            // Bilinear, like the retail client's HD upscale: the SD art is
            // magnified smoothly there, while a nearest-neighbour blit
            // leaves hard 3 px steps that the references do not have.
            let fy = (((y + 0.5) / s) - 0.5).clamp(0.0, max_sy);
            let y0 = fy.floor() as usize;
            let y1 = (y0 + 1).min(sh - 1);
            let wy = ((fy - y0 as f32) * 256.0) as u32;
            let (r0, r1) = (y0 * sw, y1 * sw);
            for &(dx, a, b, wx) in &cols {
                let (p00, p10, p01, p11) = (
                    pixels[r0 + a],
                    pixels[r0 + b],
                    pixels[r1 + a],
                    pixels[r1 + b],
                );
                let ch = |sh_: u32| {
                    let c = |p: u32| (p >> sh_) & 0xff;
                    let top = c(p00) * (256 - wx) + c(p10) * wx;
                    let bot = c(p01) * (256 - wx) + c(p11) * wx;
                    ((top * (256 - wy) + bot * wy + (1 << 15)) >> 16).min(255)
                };
                row[dx] = (ch(16) << 16) | (ch(8) << 8) | ch(0);
            }
        }
    });
}
