use super::{Framebuffer, Grp, Rgb};

/// Draw every opaque pixel of GRP frame `index` in one colour, centred on
/// `screen` (`images.dat` draw function 13: selection circles and the
/// right-click marker, recoloured by the engine).
pub fn draw_grp_flat(
    fb: &mut Framebuffer,
    grp: &Grp,
    index: usize,
    screen: (i32, i32),
    color: Rgb,
) {
    let Some(frame) = grp.frame(index) else {
        return;
    };
    let ox = screen.0 - grp.width / 2 + frame.x;
    let oy = screen.1 - grp.height / 2 + frame.y;
    for y in 0..frame.h {
        for x in 0..frame.w {
            if frame.indices[(y * frame.w + x) as usize] != 0 {
                fb.put(ox + x, oy + y, color);
            }
        }
    }
}
