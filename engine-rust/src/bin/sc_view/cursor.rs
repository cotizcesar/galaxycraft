//! The in-game pointer: retail cursor shapes and art (HD and SD), edge
//! scrolling, and the right-click feedback (target blink, ground marker).

use super::*;

/// The operating-system pointer is visible only when there is no virtual
/// pointer to draw.  With the mouse inside the game, the retail/fallback
/// cursor is drawn instead, so the player never sees two pointers.
pub(super) fn system_cursor_visible_for(pointer_present: bool) -> bool {
    !pointer_present
}

/// Index of the cursor frame for `elapsed`, in GRP order (0..4).
pub(super) fn cursor_frame_index(elapsed: Duration, frames: usize) -> usize {
    if frames <= 1 {
        return 0;
    }
    ((elapsed.as_secs_f32() * CURSOR_FPS) as usize) % frames
}

/// Retail cursor scale for a framebuffer of `height` pixels: the cursor
/// loader `FUN_0069df10` sets `height / 1920`, at least 0.25.
pub(super) fn cursor_scale_for_height(height: i32) -> f32 {
    (height.max(1) as f32 / CURSOR_ASSET_HEIGHT).max(0.25)
}

/// Which retail target cursor matches the unit under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum CursorTarget {
    /// Own unit or building (`cursor/targg.dds.grp`).
    Own,
    /// Hostile unit or building (`cursor/targr.dds.grp`).
    Enemy,
    /// Neutral unit or resource (`cursor/targy.dds.grp`).
    Neutral,
}

/// Which retail cursor the pointer shows.  The executable's cursor table
/// (x86/StarCraft.exe `0x00f07430..0x00f07560`, 16-byte rows: name, flag,
/// hotspot x, hotspot y as a fraction of the HD image) lists `arrow`,
/// `Illegal`, `TargY/R/G/N`, `MagG/R/Y`, `Drag`, `Time` and the eight
/// `Scroll*` cursors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum CursorShape {
    Arrow,
    /// Hovering over a unit: the animated magnifier (`MagG/MagR/MagY`).
    Mag(CursorTarget),
    /// A command button waiting for its target: `TargN` over terrain,
    /// `TargG/R/Y` over a unit.
    Targ(Option<CursorTarget>),
    /// A selection box being dragged: the crosshair (`Drag`).
    Drag,
    /// Scrolling at a screen edge: 0 = up, then clockwise.
    Scroll(u8),
}

pub(super) const SCROLL_CURSORS: [(&str, (f32, f32)); 8] = [
    ("scrollu", (0.5, 0.0)),
    ("scrollur", (1.0, 0.0)),
    ("scrollr", (1.0, 0.5)),
    ("scrolldr", (1.0, 1.0)),
    ("scrolld", (0.5, 1.0)),
    ("scrolldl", (0.0, 1.0)),
    ("scrolll", (0.0, 0.5)),
    ("scrollul", (0.0, 0.0)),
];

impl CursorShape {
    pub(super) const ALL: [CursorShape; 17] = [
        CursorShape::Arrow,
        CursorShape::Mag(CursorTarget::Own),
        CursorShape::Mag(CursorTarget::Enemy),
        CursorShape::Mag(CursorTarget::Neutral),
        CursorShape::Targ(None),
        CursorShape::Targ(Some(CursorTarget::Own)),
        CursorShape::Targ(Some(CursorTarget::Enemy)),
        CursorShape::Targ(Some(CursorTarget::Neutral)),
        CursorShape::Drag,
        CursorShape::Scroll(0),
        CursorShape::Scroll(1),
        CursorShape::Scroll(2),
        CursorShape::Scroll(3),
        CursorShape::Scroll(4),
        CursorShape::Scroll(5),
        CursorShape::Scroll(6),
        CursorShape::Scroll(7),
    ];

    /// File stem under `cursor/` (`.grp` for SD, `.dds.grp` for HD).
    pub(super) fn file(self) -> &'static str {
        match self {
            CursorShape::Arrow => "arrow",
            CursorShape::Mag(CursorTarget::Own) => "magg",
            CursorShape::Mag(CursorTarget::Enemy) => "magr",
            CursorShape::Mag(CursorTarget::Neutral) => "magy",
            CursorShape::Targ(None) => "targn",
            CursorShape::Targ(Some(CursorTarget::Own)) => "targg",
            CursorShape::Targ(Some(CursorTarget::Enemy)) => "targr",
            CursorShape::Targ(Some(CursorTarget::Neutral)) => "targy",
            CursorShape::Drag => "drag",
            CursorShape::Scroll(d) => SCROLL_CURSORS[d as usize % 8].0,
        }
    }

    /// HD hotspot, as a fraction of the image (the executable's table).
    /// The SD GRPs are 128 x 128 canvases drawn around their centre.
    pub(super) fn hotspot(self) -> (f32, f32) {
        match self {
            CursorShape::Arrow => (0.0, 0.0),
            CursorShape::Mag(_) => (0.48, 0.48),
            CursorShape::Targ(_) | CursorShape::Drag => (0.5, 0.5),
            CursorShape::Scroll(d) => SCROLL_CURSORS[d as usize % 8].1,
        }
    }
}

/// Retail pointer skins: HD (`cursor/*.dds.grp`) and SD (`cursor/*.grp`).
#[derive(Clone, Debug, Default)]
pub(super) struct CursorArt {
    pub(super) hd: std::collections::HashMap<CursorShape, galaxycraft::dds::DdsGrp>,
    pub(super) sd: std::collections::HashMap<CursorShape, galaxycraft::render::Grp>,
}

/// Edge-scroll direction for a pointer at the border of a `size` surface:
/// 0 = up, then clockwise; `None` away from the edges.
pub(super) fn edge_scroll_dir(pointer: (i32, i32), size: (i32, i32)) -> Option<u8> {
    let (x, y) = pointer;
    let l = x <= 1;
    let r = x >= size.0 - 2;
    let u = y <= 1;
    let d = y >= size.1 - 2;
    Some(match (u, r, d, l) {
        (true, true, _, _) => 1,
        (_, true, true, _) => 3,
        (_, _, true, true) => 5,
        (true, _, _, true) => 7,
        (true, ..) => 0,
        (_, true, ..) => 2,
        (_, _, true, _) => 4,
        (.., true) => 6,
        _ => return None,
    })
}

/// Nearest-neighbour blit of an SD cursor GRP frame, its 128 x 128 canvas
/// centred on the pointer (the classic cursors put their hotspot there).
pub(super) fn blit_sd_cursor(
    fb: &mut Framebuffer,
    grp: &galaxycraft::render::Grp,
    index: usize,
    palette: &galaxycraft::render::Palette,
    at: (i32, i32),
    scale: f32,
) {
    let Some(frame) = grp.frame(index) else {
        return;
    };
    let ox = at.0 as f32 - grp.width as f32 / 2.0 * scale;
    let oy = at.1 as f32 - grp.height as f32 / 2.0 * scale;
    for y in 0..frame.h {
        let y0 = (oy + (frame.y + y) as f32 * scale).round() as i32;
        let y1 = (oy + (frame.y + y + 1) as f32 * scale).round() as i32;
        for x in 0..frame.w {
            let c = frame.indices[(y * frame.w + x) as usize];
            if c == 0 {
                continue;
            }
            let colour = palette.colors[c as usize];
            let x0 = (ox + (frame.x + x) as f32 * scale).round() as i32;
            let x1 = (ox + (frame.x + x + 1) as f32 * scale).round() as i32;
            for py in y0..y1.max(y0 + 1) {
                for px in x0..x1.max(x0 + 1) {
                    fb.put(px, py, colour);
                }
            }
        }
    }
}

/// Bilinear, premultiplied blit of one cursor frame.  The GRP is 4K art, so
/// at 1440p every source pixel spans less than one screen pixel; nearest
/// sampling would make the arrow visibly chunky.
pub(super) fn blit_cursor_scaled(
    fb: &mut Framebuffer,
    img: &galaxycraft::dds::DdsImage,
    x: i32,
    y: i32,
    scale: f32,
) {
    if img.width == 0 || img.height == 0 || scale <= 0.0 {
        return;
    }
    let sw = img.width as f32;
    let sh = img.height as f32;
    let dw = (sw * scale).round().max(1.0) as i32;
    let dh = (sh * scale).round().max(1.0) as i32;
    for dy in 0..dh {
        let sy = ((dy as f32 + 0.5) / scale - 0.5).clamp(0.0, sh - 1.0);
        let y0 = sy.floor() as usize;
        let y1 = (y0 + 1).min(img.height - 1);
        let fy = sy - y0 as f32;
        for dx in 0..dw {
            let sx = ((dx as f32 + 0.5) / scale - 0.5).clamp(0.0, sw - 1.0);
            let x0 = sx.floor() as usize;
            let x1 = (x0 + 1).min(img.width - 1);
            let fx = sx - x0 as f32;
            let sample = |px: u32| {
                (
                    ((px >> 16) & 0xff) as f32,
                    ((px >> 8) & 0xff) as f32,
                    (px & 0xff) as f32,
                    ((px >> 24) & 0xff) as f32 / 255.0,
                )
            };
            let (r00, g00, b00, a00) = sample(img.pixel(x0, y0));
            let (r10, g10, b10, a10) = sample(img.pixel(x1, y0));
            let (r01, g01, b01, a01) = sample(img.pixel(x0, y1));
            let (r11, g11, b11, a11) = sample(img.pixel(x1, y1));
            let w00 = (1.0 - fx) * (1.0 - fy);
            let w10 = fx * (1.0 - fy);
            let w01 = (1.0 - fx) * fy;
            let w11 = fx * fy;
            let alpha = a00 * w00 + a10 * w10 + a01 * w01 + a11 * w11;
            if alpha <= 0.003 {
                continue;
            }
            // Interpolate premultiplied colour, then unpremultiply, so the
            // transparent black outside the arrow does not darken its edge.
            let r = (r00 * a00 * w00 + r10 * a10 * w10 + r01 * a01 * w01 + r11 * a11 * w11) / alpha;
            let g = (g00 * a00 * w00 + g10 * a10 * w10 + g01 * a01 * w01 + g11 * a11 * w11) / alpha;
            let b = (b00 * a00 * w00 + b10 * a10 * w10 + b01 * a01 * w01 + b11 * a11 * w11) / alpha;
            let a8 = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
            if a8 == 0 {
                continue;
            }
            let dst = fb.get(x + dx, y + dy);
            fb.put(
                x + dx,
                y + dy,
                dst.blend(
                    Rgb::new(r.round() as u8, g.round() as u8, b.round() as u8),
                    a8,
                ),
            );
        }
    }
}

/// Paint the virtual pointer: the retail cursor for `shape` (falling back
/// to the arrow), otherwise the 12x18 fallback arrow.  `sd` is the game
/// palette when the classic graphics are on: then the SD GRP is drawn,
/// scaled like the 640 x 480 screen.  Kept as a free function so the
/// rendering path is covered without constructing the whole `App`.
pub(super) fn draw_virtual_cursor(
    fb: &mut Framebuffer,
    pointer: Option<(i32, i32)>,
    art: Option<&CursorArt>,
    shape: CursorShape,
    elapsed: Duration,
    sd: Option<&galaxycraft::render::Palette>,
) {
    let Some((mx, my)) = pointer else {
        return;
    };
    // Every cursor shares the 6 fps clock measured for the arrow (ten
    // 60 Hz samples per authored frame): the loader `FUN_0069df10` builds
    // all eighteen with the same frame time, 10.
    if let (Some(palette), Some(art)) = (sd, art) {
        let grp = art
            .sd
            .get(&shape)
            .or_else(|| art.sd.get(&CursorShape::Arrow));
        if let Some(grp) = grp.filter(|g| !g.frames.is_empty()) {
            let frame = cursor_frame_index(elapsed, grp.frames.len());
            let scale = fb.height.max(1) as f32 / 480.0;
            blit_sd_cursor(fb, grp, frame, palette, (mx, my), scale);
            return;
        }
    }
    let scale = cursor_scale_for_height(fb.height);
    let found = art.and_then(|a| {
        a.hd.get(&shape).map(|g| (g, shape)).or_else(|| {
            a.hd.get(&CursorShape::Arrow)
                .map(|g| (g, CursorShape::Arrow))
        })
    });
    if let Some((grp, shape)) = found
        && !grp.frames.is_empty()
    {
        let frame = cursor_frame_index(elapsed, grp.frames.len());
        if let Some(img) = grp.frames.get(frame) {
            let w = img.width as f32 * scale;
            let h = img.height as f32 * scale;
            let (hx, hy) = shape.hotspot();
            let x = mx - (w * hx).round() as i32;
            let y = my - (h * hy).round() as i32;
            blit_cursor_scaled(fb, img, x, y, scale);
            return;
        }
    }
    // 12x18 arrow.
    let shape: [&str; 12] = [
        "X...........",
        "XX..........",
        "X.X.........",
        "X..X........",
        "X...X.......",
        "X....X......",
        "X.....X.....",
        "X......X....",
        "X....XXXX...",
        "X..X...X....",
        "X.X.....X...",
        "XX......X...",
    ];
    for (dy, row) in shape.iter().enumerate() {
        for (dx, c) in row.chars().enumerate() {
            if c == 'X' {
                let (x, y) = (mx + dx as i32, my + dy as i32);
                fb.put(x, y, Rgb::new(0, 0, 0));
                fb.put(x + 1, y, Rgb::new(240, 240, 240));
            }
        }
    }
}

impl App {
    pub(super) fn draw_cursor(&self, fb: &mut Framebuffer) {
        // The command wheel is modal: no pointer while it is open.
        if self.pad_wheel.is_open() {
            return;
        }
        let shape = self.cursor_shape();
        // Classic graphics use the classic (SD) cursors.
        let sd = (!self.game_options.video_hd).then(|| self.sprites.palette());
        draw_virtual_cursor(
            fb,
            self.pointer,
            self.cursor_art.as_ref(),
            shape,
            self.cursor_started.elapsed(),
            sd,
        );
    }

    /// Which retail cursor the pointer shows this frame: a scroll arrow at
    /// a screen edge, the crosshair while a selection box is dragged, the
    /// target cursors while a command waits for its target, the magnifier
    /// over a unit, otherwise the arrow.
    pub(super) fn cursor_shape(&self) -> CursorShape {
        let Some((mx, my)) = self.pointer else {
            return CursorShape::Arrow;
        };
        // Only the live game transforms the pointer; menus, briefing
        // screens and dialogs keep the arrow.
        if self.menu
            || self.intro
            || self.briefing
            || self.overlay != 0
            || self.game_dialog.is_some()
            || self.ingame_options
        {
            return CursorShape::Arrow;
        }
        if let Some(dir) = self.edge_scroll {
            return CursorShape::Scroll(dir);
        }
        if self.target_act.is_none()
            && self.build_mode.is_none()
            && drag_box(self.drag_start, (mx, my)).is_some()
        {
            return CursorShape::Drag;
        }
        // The console keeps the arrow.
        if my >= self.playfield_bottom() {
            return CursorShape::Arrow;
        }
        let relation = self
            .unit_at(mx, my)
            .and_then(|id| self.world.unit(id))
            .map(|u| {
                // Resources are stored under player 0: neutral first.
                let neutral = u.is_resource()
                    || u.type_id == galaxycraft::ids::unit::VESPENE_GEYSER
                    || u.owner >= 11
                    || u.is_detached_addon();
                if neutral {
                    CursorTarget::Neutral
                } else if u.owner as usize == self.local_player {
                    CursorTarget::Own
                } else if self.world.hostile(self.local_player as u8, u.owner) {
                    CursorTarget::Enemy
                } else {
                    CursorTarget::Neutral
                }
            });
        if self.target_act.is_some() {
            return CursorShape::Targ(relation);
        }
        if self.build_mode.is_some() {
            return CursorShape::Arrow;
        }
        relation.map_or(CursorShape::Arrow, CursorShape::Mag)
    }

    /// Green circle on the ground where a move order was given, blinking
    /// like the target flash.  INFERENCE: retail's cursor-marker sprite is
    /// approximated with the 32 px selection circle.
    /// World point of the right-click ground marker while it blinks on.
    pub(super) fn order_marker_shown(&mut self) -> Option<(i32, i32)> {
        let (at, t) = self.order_marker?;
        let age = t.elapsed().as_secs_f32();
        if age > ORDER_FLASH_SECS {
            self.order_marker = None;
            return None;
        }
        order_flash_on(age).then_some(at)
    }

    /// SD right-click marker: the 32 px selection oval (image 562) in green.
    pub(super) fn draw_order_marker_sd(&mut self, fb: &mut Framebuffer) {
        let Some((wx, wy)) = self.order_marker_shown() else {
            return;
        };
        if let Some(grp) = self.sd_image_grp(562) {
            let at = (wx - self.camera.0, wy - self.camera.1);
            galaxycraft::render::draw_grp_flat(fb, &grp, 0, at, HUD_SELECT_GREEN);
        }
    }

    pub(super) fn draw_order_marker(
        &mut self,
        fb: &mut Framebuffer,
        clip: (i32, i32, i32, i32),
        scale: f32,
    ) {
        let Some((wx, wy)) = self.order_marker_shown() else {
            return;
        };
        let install = self.install.as_ref();
        let Some(img) = self.hd_sprites.image(561 + 1, |path| install?.read(path)) else {
            return;
        };
        let s = scale;
        let sx = clip.0 as f32 + (wx - self.camera.0) as f32 * scale;
        let sy = clip.1 as f32 + (wy - self.camera.1) as f32 * scale;
        galaxycraft::hd_sprites::draw_flat(fb, &img, 0, sx, sy, s, HUD_SELECT_GREEN, clip);
    }
}

/// Mouse Speed / Mouse Scaling factor for the software pointer.  The Speed
/// tab's sensitivity slider (0..1) maps to 0.5..1.5 around the retail
/// default, and "Use Mouse Scaling" compensates for resolutions above the
/// original 480-line playfield.  A factor of exactly 1 keeps the system
/// pointer position untouched.
pub(super) fn mouse_move_factor(options: &GameOptions, surface: (i32, i32)) -> f32 {
    let base = if options.use_mouse_sensitivity {
        0.5 + options.mouse_sensitivity.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let scaled = if options.mouse_scaling {
        base * (surface.1.max(1) as f32 / 480.0).clamp(0.5, 4.0)
    } else {
        base
    };
    scaled.clamp(0.25, 4.0)
}

/// StarCraft 1 selection normalisation: a building is selected alone, and
/// mobile units stop at twelve.  `(id, is_building)` pairs are consumed in
/// selection order.
pub(super) fn normalize_selection(entries: impl IntoIterator<Item = (u32, bool)>) -> Vec<u32> {
    let mut out = Vec::new();
    for (id, building) in entries {
        if building {
            return vec![id];
        }
        if out.len() < 12 {
            out.push(id);
        }
    }
    out
}

/// Advance a virtual pointer by the raw movement delta times `factor`.
/// Only used when the factor differs from 1; the cursor remains inside the
/// drawing surface.
pub(super) fn advance_virtual_pointer(
    current: (f32, f32),
    last: (f32, f32),
    raw: (f32, f32),
    factor: f32,
    bounds: (i32, i32),
) -> (f32, f32) {
    let x = current.0 + (raw.0 - last.0) * factor;
    let y = current.1 + (raw.1 - last.1) * factor;
    (
        x.clamp(0.0, (bounds.0 - 1).max(0) as f32),
        y.clamp(0.0, (bounds.1 - 1).max(0) as f32),
    )
}

/// The box-selection rectangle `(x, y, w, h)` between the drag start and
/// the pointer, once the pointer has moved (a plain click draws none).
pub(super) fn drag_box(start: Option<(i32, i32)>, now: (i32, i32)) -> Option<(i32, i32, i32, i32)> {
    let s = start?;
    if s == now {
        return None;
    }
    let (x0, y0) = (s.0.min(now.0), s.1.min(now.1));
    let (x1, y1) = (s.0.max(now.0), s.1.max(now.1));
    Some((x0, y0, (x1 - x0).max(1), (y1 - y0).max(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_cursor(w: usize, h: usize, argb: u32) -> galaxycraft::dds::DdsGrp {
        galaxycraft::dds::DdsGrp {
            frames: vec![galaxycraft::dds::DdsImage {
                width: w,
                height: h,
                pixels: vec![argb; w * h],
            }],
        }
    }

    #[test]
    fn the_virtual_cursor_is_shown_with_the_mouse_and_pad() {
        // With a pointer present (mouse or pad), the OS pointer is hidden and
        // the drawn cursor is the only one.
        assert!(!system_cursor_visible_for(true));
        // Without a pointer there is nothing to draw, so keep the OS arrow.
        assert!(system_cursor_visible_for(false));
    }

    #[test]
    fn sc1_selection_caps_at_twelve_units_or_one_building() {
        let units: Vec<(u32, bool)> = (1..=13).map(|id| (id, false)).collect();
        let picked = normalize_selection(units);
        assert_eq!(picked.len(), 12);
        assert_eq!(picked.last(), Some(&12));
        // A building is selected alone even when it arrives after units.
        let mixed = [(1, false), (2, false), (9, true), (3, false)];
        assert_eq!(normalize_selection(mixed), vec![9]);
    }

    #[test]
    fn the_virtual_cursor_renders_its_arrow_at_the_pointer() {
        let mut fb = Framebuffer::new(64, 48);
        fb.clear(Rgb::new(0, 0, 0));
        let zero = std::time::Duration::ZERO;
        draw_virtual_cursor(&mut fb, Some((8, 6)), None, CursorShape::Arrow, zero, None);
        // The fallback arrow is black with a light right/bottom edge.
        assert_eq!(fb.get(9, 6), Rgb::new(240, 240, 240));
        assert_eq!(fb.get(8, 0), Rgb::new(0, 0, 0));
        // No pointer: nothing is painted and no out-of-range panic.
        let mut empty = Framebuffer::new(8, 8);
        draw_virtual_cursor(&mut empty, None, None, CursorShape::Arrow, zero, None);
        assert_eq!(empty.get(0, 0), Rgb::new(0, 0, 0));
    }

    fn two_frames(w: usize, h: usize) -> galaxycraft::dds::DdsGrp {
        galaxycraft::dds::DdsGrp {
            frames: vec![
                galaxycraft::dds::DdsImage {
                    width: w,
                    height: h,
                    pixels: vec![0xffff_0000; w * h],
                },
                galaxycraft::dds::DdsImage {
                    width: w,
                    height: h,
                    pixels: vec![0xff00_00ff; w * h],
                },
            ],
        }
    }

    #[test]
    fn the_target_cursor_is_centred_and_animated() {
        let mut art = CursorArt::default();
        art.hd
            .insert(CursorShape::Arrow, solid_cursor(2, 2, 0xff00_ff00));
        art.hd
            .insert(CursorShape::Targ(Some(CursorTarget::Own)), two_frames(4, 4));
        let targ = CursorShape::Targ(Some(CursorTarget::Own));
        // 1920-line framebuffer: one authored cursor pixel is one screen pixel.
        let mut fb = Framebuffer::new(64, 1920);
        fb.clear(Rgb::new(0, 0, 0));
        let zero = std::time::Duration::ZERO;
        draw_virtual_cursor(&mut fb, Some((32, 30)), Some(&art), targ, zero, None);
        // Centred hotspot (0.5, 0.5): the 4x4 frame starts at (30, 28).
        assert_eq!(fb.get(30, 28), Rgb::new(255, 0, 0));
        assert_eq!(fb.get(29, 27), Rgb::new(0, 0, 0));
        // The second authored frame follows the same 6 fps cursor clock.
        fb.clear(Rgb::new(0, 0, 0));
        let later = std::time::Duration::from_millis(167);
        draw_virtual_cursor(&mut fb, Some((32, 30)), Some(&art), targ, later, None);
        assert_eq!(fb.get(30, 28), Rgb::new(0, 0, 255));
        // The arrow sits at its top-left tip; a shape without art falls
        // back to it.
        for shape in [CursorShape::Arrow, CursorShape::Drag] {
            fb.clear(Rgb::new(0, 0, 0));
            draw_virtual_cursor(&mut fb, Some((10, 10)), Some(&art), shape, zero, None);
            assert_eq!(fb.get(10, 10), Rgb::new(0, 255, 0), "{shape:?}");
        }
    }

    #[test]
    fn hd_cursors_use_the_executable_hotspots() {
        // `MagG` 0.48 / 0.48, `ScrollDR` the bottom-right corner.
        let mut art = CursorArt::default();
        let mag = CursorShape::Mag(CursorTarget::Enemy);
        art.hd.insert(mag, solid_cursor(100, 100, 0xffff_0000));
        art.hd
            .insert(CursorShape::Scroll(3), solid_cursor(10, 10, 0xff00_ff00));
        let mut fb = Framebuffer::new(400, 1920);
        let zero = std::time::Duration::ZERO;
        draw_virtual_cursor(&mut fb, Some((200, 200)), Some(&art), mag, zero, None);
        assert_eq!(fb.get(152, 152), Rgb::new(255, 0, 0));
        assert_eq!(fb.get(151, 151), Rgb::new(0, 0, 0));
        fb.clear(Rgb::new(0, 0, 0));
        draw_virtual_cursor(
            &mut fb,
            Some((399, 399)),
            Some(&art),
            CursorShape::Scroll(3),
            zero,
            None,
        );
        assert_eq!(fb.get(398, 398), Rgb::new(0, 255, 0));
        assert_eq!(fb.get(389, 389), Rgb::new(0, 255, 0));
        assert_eq!(fb.get(388, 388), Rgb::new(0, 0, 0));
    }

    #[test]
    fn sd_cursors_are_centred_on_their_canvas_and_scaled_from_480_lines() {
        let mut palette = galaxycraft::render::Palette {
            colors: [Rgb::new(0, 0, 0); 256],
        };
        palette.colors[7] = Rgb::new(10, 200, 10);
        // A 128 x 128 canvas with one pixel at (64, 64): the hotspot.
        let grp = galaxycraft::render::Grp {
            width: 128,
            height: 128,
            frames: vec![galaxycraft::render::GrpFrame {
                x: 64,
                y: 64,
                w: 1,
                h: 1,
                indices: vec![7],
            }],
        };
        let mut art = CursorArt::default();
        art.sd.insert(CursorShape::Arrow, grp);
        let mut fb = Framebuffer::new(200, 720);
        let zero = std::time::Duration::ZERO;
        let p = Some(&palette);
        draw_virtual_cursor(
            &mut fb,
            Some((100, 100)),
            Some(&art),
            CursorShape::Drag,
            zero,
            p,
        );
        // 720 / 480 = 1.5x: the pixel covers (100..101.5) -> 100 and 101.
        assert_eq!(fb.get(100, 100), Rgb::new(10, 200, 10));
        assert_eq!(fb.get(101, 101), Rgb::new(10, 200, 10));
        assert_eq!(fb.get(99, 99), Rgb::new(0, 0, 0));
    }

    #[test]
    fn edge_scroll_follows_the_screen_border() {
        let size = (1280, 720);
        assert_eq!(edge_scroll_dir((640, 0), size), Some(0));
        assert_eq!(edge_scroll_dir((1279, 0), size), Some(1));
        assert_eq!(edge_scroll_dir((1279, 300), size), Some(2));
        assert_eq!(edge_scroll_dir((1279, 719), size), Some(3));
        assert_eq!(edge_scroll_dir((640, 719), size), Some(4));
        assert_eq!(edge_scroll_dir((0, 719), size), Some(5));
        assert_eq!(edge_scroll_dir((0, 300), size), Some(6));
        assert_eq!(edge_scroll_dir((1, 1), size), Some(7));
        assert_eq!(edge_scroll_dir((2, 300), size), None);
        assert_eq!(edge_scroll_dir((640, 360), size), None);
    }
}
