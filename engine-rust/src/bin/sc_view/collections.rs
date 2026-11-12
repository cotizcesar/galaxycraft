//! `impl App`: collections (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    /// Credit entries from the shipped `creditspanel.js` bundle. Each entry is
    /// `{name:"...",style:N}`; style 1..5 are the CSS heading levels and
    /// style 6 is a divider.
    pub(super) fn credits_entries(&mut self) -> Vec<(String, u8)> {
        if self.credits.is_empty()
            && let Some(install) = &self.install
            && let Ok(bytes) = install.read_by_path("webui/dist/creditspanel/creditspanel.js")
        {
            let text = String::from_utf8_lossy(&bytes);
            self.credits = parse_credits(&text);
        }
        self.credits.clone()
    }

    /// Retail Collections screen (reference #22). Rects measured on that
    /// 3440x1440 screenshot as fractions of the 2560x1440 content box: main
    /// panel 0.141..0.601 x 0.045..0.836, preview column 0.618..0.863, Ok at
    /// 0.715..0.863 x 0.868..0.916, section rows at 0.147 / 0.373 / 0.598.
    /// Retail CEF `Collections` panel, rebuilt from the shipped
    /// `collectionspanel.js` layout (measured 1:1 on capture 131005):
    /// 640x480 CSS at 3x, one outer plate, two red frames, three section
    /// grids with width-fit thumbnails, scrollbar and the right-hand
    /// preview/EQUIP/Ok column.
    pub(super) fn draw_collections(&mut self, fb: &mut Framebuffer) {
        // A CEF modal over the running main menu, like Options: the clips,
        // the entries and the panel spinner show through the translucent
        // plate (user capture 2026-11-04 and reference 131005).
        self.draw_menu_backdrop_live(fb);
        let (cx, _cy, cw, ch) = Self::content_rect(fb);
        dim_screen(fb, cx, cw, ch, 149);
        // Everything above the dimmed menu is static: composed once per
        // window size (see `LayerCache`).
        let scroll = self.collections_scroll.round() as i32;
        let key = (
            fb.width,
            fb.height,
            scroll,
            self.collection_sel,
            self.game_options.game_unit_skin,
        );
        if self.collections_layer.as_ref().map(|c| c.0) != Some(key) {
            let (w, h) = (fb.width as usize, fb.height as usize);
            let mut over_black = Framebuffer::new(w, h);
            over_black.clear(Rgb::new(0, 0, 0));
            self.draw_collections_chrome(&mut over_black);
            let mut over_white = Framebuffer::new(w, h);
            over_white.clear(Rgb::new(255, 255, 255));
            self.draw_collections_chrome(&mut over_white);
            self.collections_layer = Some((key, LayerCache::new(&over_black, &over_white)));
        }
        if let Some((_, layer)) = &self.collections_layer {
            layer.apply(fb);
        }
    }

    /// The Collections plate, grid, preview and buttons over the dimmed menu.
    pub(super) fn draw_collections_chrome(&mut self, fb: &mut Framebuffer) {
        let (cx, _cy, _cw, ch) = Self::content_rect(fb);
        let (scale, uox, uoy) = self.ui_metrics(fb);
        let k = ch as f32 / 1440.0;
        let sx = |x: f32| cx + (x * k).round() as i32;
        let sy = |y: f32| (y * k).round() as i32;
        let scroll = self.collections_scroll;
        let cy = |y: f32| ((y - scroll) * k).round() as i32;
        let sw = |w: f32| (w * k).round().max(1.0) as i32;
        // Screen px -> UI units (draw_hd_text anchors/widths are CSS-space).
        let u = |v: f32| (v - 320.0) / 3.0;
        let v = |y: f32| y / 3.0;
        let w3 = |v: f32| v / 3.0;
        let r = |x: f32, y: f32, w: f32, h: f32| (sx(x), sy(y), sw(w), sw(h));
        let white = Rgb::new(0xff, 0xff, 0xff);
        let grey = Rgb::new(0xcc, 0xcc, 0xcc);
        let green = Rgb::new(0x4c, 0xc4, 0x28);
        let equip_grey = Rgb::new(0x80, 0x80, 0x80);

        // One outer plate (357..2205 x 71..1207): 5 px #331f1f, radius 4,
        // fill rgba(0,0,0,.5).  The old code drew two plates with a 43 px gap
        // and a 90 % fill (5x too dark).
        let (px, py, pw, ph) = r(357.0, 71.0, 1849.0, 1137.0);
        rounded_fill_alpha(fb, px, py, pw, ph, 4.0 * k, COLLECTIONS_PLATE_ALPHA);
        for t in 0..(5.0 * k).round().max(1.0) as i32 {
            fb.rect_outline(px + t, py + t, pw - t * 2, ph - t * 2, CSS_OUTER);
        }
        // frame0 (red, no right border) and frame1 (grey left hairline).
        let b = (3.0 * k).max(1.0) as i32;
        // Section separators (3 px #474747 at x 401..1513).
        for y in [247.0f32, 568.0, 889.0] {
            let top = cy(y);
            if top >= sy(80.0) && top < sy(1192.0) {
                fb.rect(sx(401.0), top, sw(1113.0), b, Rgb::new(0x47, 0x47, 0x47));
            }
        }
        // Title (Extended 9 px, #fff) and dropdown chip.
        let collections_title = self.cef_text(
            "scr.profile.tab.heading.5",
            &self.ui_text("gluMain-6", "Collections"),
        );
        self.draw_hd_text(
            fb,
            &collections_title,
            u(402.0),
            v(121.0),
            galaxycraft::menu::LabelAlign::Left,
            w3(174.0),
            9.0,
            scale,
            uox,
            uoy,
            white,
            true,
        );
        let (dx, dy, dw, dh) = r(1232.0, 115.0, 300.0, 44.0);
        rounded_fill_alpha(fb, dx, dy, dw, dh, 4.0 * k, 230);
        for t in 0..b {
            fb.rect_outline(dx + t, dy + t, dw - t * 2, dh - t * 2, CSS_INNER);
        }
        let all_collections = self.cef_text("profile_all_collections", "All Collections");
        self.draw_hd_text(
            fb,
            &all_collections,
            u(1241.0),
            v(129.0),
            galaxycraft::menu::LabelAlign::Left,
            w3(136.0),
            8.0,
            scale,
            uox,
            uoy,
            green,
            false,
        );
        // Olive dropdown arrow (15x8 at 1499,138).
        let (ax, ay) = (sx(1499.0), sy(138.0));
        for i in 0..8 {
            let w = (15.0 * k * (1.0 - i as f32 / 8.0)).round() as i32;
            fb.rect(
                ax + ((15.0 * k) as i32 - w) / 2,
                ay + (i as f32 * k) as i32,
                w.max(1),
                1,
                Rgb::new(0xb1, 0xb1, 0x0d),
            );
        }
        // Section headers and right-aligned counts.
        let available = self.cef_text("profile_collections_available", "{numAvailable} Available");
        let sections = [
            (
                self.cef_text("profile_console_skins", "Console Skins"),
                "3",
                216.0f32,
                214.0,
                92.0f32,
                152.0f32,
            ),
            (
                self.cef_text("profile_unit_skins", "Unit & Structure Skins"),
                "2",
                537.0,
                535.0,
                91.0,
                248.0,
            ),
            (
                self.cef_text("profile_collection_announcers_title", "Announcers"),
                "6",
                858.0,
                856.0,
                92.0,
                132.0,
            ),
        ];
        for (label, count, ink_y, count_y, count_w, label_w) in sections {
            let header_y = (ink_y - scroll) * k;
            if header_y < 180.0 * k || header_y > 1194.0 * k {
                continue;
            }
            let count = available.replace("{numAvailable}", count);
            self.draw_hd_text(
                fb,
                &label,
                u(402.0),
                (ink_y - scroll) / 3.0,
                galaxycraft::menu::LabelAlign::Left,
                w3(label_w),
                9.0,
                scale,
                uox,
                uoy,
                grey,
                false,
            );
            // Counts are right-aligned on the separator edge (x 1513).
            let center = 1513.0 - count_w / 2.0;
            self.draw_hd_text(
                fb,
                &count,
                u(center),
                (count_y - scroll) / 3.0,
                galaxycraft::menu::LabelAlign::Center,
                w3(count_w),
                7.0,
                scale,
                uox,
                uoy,
                green,
                false,
            );
        }
        // Grid: 353x224 cells, columns 411/787/1161, rows 274/595/916/1158.
        let cells: [(f32, f32, &str); 11] = [
            (411.0, 274.0, "default_console_thumbnail.jpg"),
            (787.0, 274.0, "krc_silver_console_thumbnail.png"),
            (1161.0, 274.0, "krc_gold_console_thumbnail.png"),
            (411.0, 595.0, "skins_default.jpg"),
            (787.0, 595.0, "skins_carbot.png"),
            (411.0, 916.0, "default_announcer.png"),
            (787.0, 916.0, "um_announcer.png"),
            (1161.0, 916.0, "jeon_announcer.png"),
            (411.0, 1158.0, "kim_announcer.png"),
            (787.0, 1158.0, "umjeonkim_announcer.png"),
            (1161.0, 1158.0, "heyjini_announcer.png"),
        ];
        let cell_w = sw(353.0);
        let cell_h = sw(224.0);
        let clip_bottom = sy(1192.0);
        let clip_top = sy(180.0);
        let equipped_skin = self.game_options.game_unit_skin;
        let equipped_console = self.game_options.game_console_skin;
        let equipped_announcer = self.game_options.game_announcer;
        for (i, (x, y, name)) in cells.into_iter().enumerate() {
            let selected = i == self.collection_sel;
            let equipped = (i <= 2 && i as u8 == equipped_console)
                || (i >= 5 && (i - 5) as u8 == equipped_announcer)
                || (i == 3 && equipped_skin == 0)
                || (i == 4 && equipped_skin == 2);
            let (cell_x, cell_y) = (sx(x), cy(y));
            let visible_h = cell_h.min(clip_bottom - cell_y).max(0);
            let visible_top = (clip_top - cell_y).max(0);
            if visible_h <= visible_top {
                continue;
            }
            if visible_h <= 0 {
                continue;
            }
            // Width-fit thumbnail inside the cell border, top-left anchored
            // and clipped by the cell.  Template matching against 131005 puts
            // every thumbnail at cell+(3,3) at ~347x239 (corr 0.95), selected
            // or not; the old stretch to 349x215 was the main grid error.
            let (img_w, img_h, off) = (347.0, 239.0, b);
            if let Some(img) = self.thumb(name, sw(img_w), sw(img_h)) {
                let rows = visible_h.min(img.height as i32 - off);
                let cols = cell_w.min(img.width as i32);
                let start = (clip_top - (cell_y + off)).max(0);
                for yy in start..rows.max(0) {
                    for xx in 0..cols.max(0) {
                        let sx_i = xx as usize;
                        let sy_i = yy as usize;
                        let kk = (sy_i * img.width as usize + sx_i) * 3;
                        if kk + 2 < img.rgb.len() {
                            fb.put(
                                cell_x + off + xx,
                                cell_y + off + yy,
                                Rgb::new(img.rgb[kk], img.rgb[kk + 1], img.rgb[kk + 2]),
                            );
                        }
                    }
                }
            }
            // Only the selected cell is framed (#9cb3fa); on 131005 the other
            // cells show their art up to the edge, with no grey border.
            if selected {
                let top = cell_y.max(clip_top);
                let h = (cell_y + visible_h - top).max(0);
                if h > 0 {
                    for t in 0..b {
                        fb.rect_outline(
                            cell_x + t,
                            top + t,
                            cell_w - t * 2,
                            h - t * 2,
                            Rgb::new(0x9c, 0xb3, 0xfa),
                        );
                    }
                }
            }
            // Equipment mark: a green dot in the cell's top-right corner, so
            // the active unit collection stays visible while scrolling.
            if equipped {
                let (dx, dy) = (cell_x + cell_w - sw(22.0), cell_y + sw(8.0));
                let dot = sw(9.0).max(3);
                if dy >= clip_top && dy + dot <= clip_bottom {
                    fb.rect(dx, dy, dot, dot, green);
                }
            }
        }
        // Frame borders are re-stated after the grid so the clipped
        // announcer row cannot cover the bottom hairline.
        let (f0x, f0y, f0w, f0h) = r(368.0, 82.0, 1224.0, 1113.0);
        let b = (3.0 * k).max(1.0) as i32;
        fb.rect(f0x, f0y, f0w, b, CSS_INNER);
        fb.rect(f0x, f0y + f0h - b, f0w, b, CSS_INNER);
        fb.rect(f0x, f0y, b, f0h, CSS_INNER);
        let (f1x, f1y, f1w, f1h) = r(1594.0, 82.0, 601.0, 1113.0);
        fb.rect(f1x, f1y, f1w, b, CSS_INNER);
        fb.rect(f1x, f1y + f1h - b, f1w, b, CSS_INNER);
        fb.rect(f1x + f1w - b, f1y, b, f1h, CSS_INNER);
        fb.rect(f1x, f1y, b, f1h, Rgb::new(74, 71, 71));
        // Scrollbar: track 1538..1561 x 241..1161, thumb to 990, arrows.
        let (sbx, sby, sbw, sbh) = r(1538.0, 241.0, 24.0, 921.0);
        fb.rect(sbx, sby, sbw, sbh, Rgb::new(0, 0, 0));
        for t in 0..b {
            fb.rect_outline(sbx + t, sby + t, sbw - t * 2, sbh - t * 2, CSS_INNER);
        }
        let (thx, thy, thw, thh) = r(1538.0, 241.0, 24.0, 750.0);
        for t in 0..b {
            fb.rect_outline(
                thx + t,
                thy + t,
                thw - t * 2,
                thh - t * 2,
                Rgb::new(0xd6, 0x0e, 0x00),
            );
        }
        let red = Rgb::new(0xfa, 0x00, 0x00);
        let (upx, upy) = (sx(1538.0), sy(215.0));
        for i in 0..10 {
            let w = (22.0 * k * (i as f32 + 1.0) / 10.0).round() as i32;
            fb.rect(
                upx + (sw(22.0) - w) / 2,
                upy + (i as f32 * k) as i32,
                w.max(1),
                1,
                red,
            );
        }
        let (dnx, dny) = (sx(1538.0), sy(1166.0));
        for i in 0..10 {
            let w = (22.0 * k * (10.0 - i as f32) / 10.0).round() as i32;
            fb.rect(
                dnx + (sw(22.0) - w) / 2,
                dny + (i as f32 * k) as i32,
                w.max(1),
                1,
                red,
            );
        }
        // Right column: preview of the selected grid cell, title, EQUIP, Ok.
        let sel = self
            .collection_sel
            .min(galaxycraft::menu::COLLECTION_ITEMS.len() - 1);
        let (sel_img, sel_title) = galaxycraft::menu::COLLECTION_ITEMS[sel];
        let sel_title = if sel_title == "Default" {
            self.cef_text("profile_collection_announcers_default_title", "Default")
        } else {
            sel_title.to_string()
        };
        if let Some(img) = self.thumb(sel_img, sw(349.0), sw(237.0)) {
            let (ix, iy) = (sx(1719.0), sy(118.0));
            for yy in 0..img.height as i32 {
                for xx in 0..img.width as i32 {
                    let kk = ((yy * img.width as i32 + xx) * 3) as usize;
                    if kk + 2 < img.rgb.len() {
                        fb.put(
                            ix + xx,
                            iy + yy,
                            Rgb::new(img.rgb[kk], img.rgb[kk + 1], img.rgb[kk + 2]),
                        );
                    }
                }
            }
        }
        self.draw_hd_text(
            fb,
            &sel_title,
            u(1825.0),
            v(373.0),
            galaxycraft::menu::LabelAlign::Left,
            w3(139.0),
            11.0,
            scale,
            uox,
            uoy,
            white,
            true,
        );
        let (ex, ey, ew, eh) = r(1688.0, 421.0, 410.0, 57.0);
        fb.rect(ex, ey, ew, eh, Rgb::new(0, 0, 0));
        for t in 0..b {
            fb.rect_outline(
                ex + t,
                ey + t,
                ew - t * 2,
                eh - t * 2,
                Rgb::new(0x45, 0x45, 0x45),
            );
        }
        let skin = self.game_options.game_unit_skin;
        let console_skin = self.game_options.game_console_skin;
        let announcer = self.game_options.game_announcer;
        let (equip_label, equip_color) = match self.collection_sel {
            0 if console_skin == 0 => {
                (self.cef_text("scr.profile.btn.equipped", "EQUIPPED"), green)
            }
            1 if console_skin == 1 => {
                (self.cef_text("scr.profile.btn.equipped", "EQUIPPED"), green)
            }
            2 if console_skin == 2 => {
                (self.cef_text("scr.profile.btn.equipped", "EQUIPPED"), green)
            }
            0..=2 => (self.cef_text("scr.profile.btn.equip", "EQUIP"), white),
            5 if announcer == 0 => (self.cef_text("scr.profile.btn.equipped", "EQUIPPED"), green),
            6 if announcer == 1 => (self.cef_text("scr.profile.btn.equipped", "EQUIPPED"), green),
            7 if announcer == 2 => (self.cef_text("scr.profile.btn.equipped", "EQUIPPED"), green),
            8 if announcer == 3 => (self.cef_text("scr.profile.btn.equipped", "EQUIPPED"), green),
            9 if announcer == 4 => (self.cef_text("scr.profile.btn.equipped", "EQUIPPED"), green),
            10 if announcer == 5 => (self.cef_text("scr.profile.btn.equipped", "EQUIPPED"), green),
            5..=10 => (self.cef_text("scr.profile.btn.equip", "EQUIP"), white),
            3 if skin == 0 => (self.cef_text("scr.profile.btn.equipped", "EQUIPPED"), green),
            4 if skin == 2 => (self.cef_text("scr.profile.btn.equipped", "EQUIPPED"), green),
            3 | 4 => (self.cef_text("scr.profile.btn.equip", "EQUIP"), white),
            _ => (self.cef_text("scr.profile.btn.equip", "EQUIP"), equip_grey),
        };
        self.draw_hd_text(
            fb,
            &equip_label,
            u(1848.0),
            v(441.0),
            galaxycraft::menu::LabelAlign::Left,
            w3(91.0),
            8.0,
            scale,
            uox,
            uoy,
            equip_color,
            true,
        );
        // Ok holder + button at the panel's bottom-right.
        let (hx, hy, hw, hh) = r(1828.0, 1236.0, 378.0, 84.0);
        rounded_fill_alpha(fb, hx, hy, hw, hh, 4.0 * k, 204);
        for t in 0..(5.0 * k).round().max(1.0) as i32 {
            fb.rect_outline(hx + t, hy + t, hw - t * 2, hh - t * 2, CSS_OUTER);
        }
        let (ox, oy, ow, oh) = r(1837.0, 1245.0, 360.0, 66.0);
        rounded_fill_alpha(fb, ox, oy, ow, oh, 2.0 * k, 235);
        for t in 0..b {
            fb.rect_outline(ox + t, oy + t, ow - t * 2, oh - t * 2, CSS_INNER);
        }
        let ok_label = self.ui_text("gluCreat-2", "Ok");
        self.draw_hd_text(
            fb,
            &ok_label,
            u(1995.0),
            v(1266.0),
            galaxycraft::menu::LabelAlign::Left,
            w3(47.0),
            10.0,
            scale,
            uox,
            uoy,
            Rgb::new(0xb4, 0xfc, 0x74),
            true,
        );
    }

    pub(super) fn draw_credits(&mut self, fb: &mut Framebuffer) {
        // The backdrop and panel are static: compose them once per size
        // (scaling the DDS art every frame cost 35 ms at 3440x1440).
        let size = (fb.width as usize, fb.height as usize);
        if let Some((key, pixels)) = &self.credits_view.backdrop
            && *key == size
            && pixels.len() == fb.pixels.len()
        {
            fb.pixels.copy_from_slice(pixels);
            self.draw_credit_lines(fb);
            return;
        }
        let (cx, _cy, cw, ch) = Self::content_rect(fb);
        // Backdrop: the shipped metal room (retail metal-room backdrop).
        match self.glue.as_ref().and_then(|g| g.credits_backdrop.clone()) {
            Some(art) => art.draw_scaled(fb, cx, 0, cw, ch),
            None => {
                for y in 0..ch {
                    let t = y as f32 / ch as f32;
                    let c = Rgb::new(
                        (14.0 + 10.0 * t) as u8,
                        (12.0 + 7.0 * t) as u8,
                        (10.0 + 4.0 * t) as u8,
                    );
                    fb.rect(cx, y, cw, 1, c);
                }
            }
        }
        // Panel: `credits_a.dds` at the layout's (120,30,420,450) rect,
        // centred in the 640x480 UI space.
        let (scale, ox, oy) = self.ui_metrics(fb);
        let panel = UiRect::new(120, 30, 418, 429).scaled(scale, ox, oy);
        if let Some(art) = self.glue.as_ref().and_then(|g| g.credits_frame.clone()) {
            art.draw_scaled(fb, panel.x, panel.y, panel.w, panel.h);
        } else {
            fb.rect(panel.x, panel.y, panel.w, panel.h, Rgb::new(6, 5, 5));
            fb.rect_outline(panel.x, panel.y, panel.w, panel.h, Rgb::new(150, 40, 20));
        }
        self.credits_view.backdrop = Some((size, fb.pixels.clone()));
        self.draw_credit_lines(fb);
    }

    /// Show the Credits screen: the panel mounts fresh (scrolled to the top,
    /// its 2000 ms auto-scroll timer starting now).  The retail screen cuts
    /// in with no transition (recording, one frame).
    pub(super) fn open_credits(&mut self) {
        self.menu_screen = MenuScreen::Credits;
        let view = &mut self.credits_view;
        view.scroll = Default::default();
        view.frames = 0;
        view.since = Some(Instant::now());
        view.keys = (false, false);
        view.wheel = 0;
    }

    /// CSS fallback faces of the credits list after Eurostile Extended.
    pub(super) fn credits_fallback_fonts(&self) -> Vec<galaxycraft::text::UiFont> {
        let Some(install) = self.install.as_ref() else {
            return Vec::new();
        };
        [
            "webui/dist/lib/fonts/nimbus(blizzardglobal).ttf",
            "webui/dist/lib/fonts/udtypos510b_p_h.ttf",
        ]
        .iter()
        .filter_map(|p| {
            let bytes = install.read_by_path(p).ok()?;
            galaxycraft::text::UiFont::from_bytes(bytes, p.to_string())
        })
        .collect()
    }

    /// The credits list (`.CreditsPanel`): laid out, scrolled and drawn as
    /// the CEF page does (see `galaxycraft::credits`).
    pub(super) fn draw_credit_lines(&mut self, fb: &mut Framebuffer) {
        use galaxycraft::credits::{CreditsLayout, EmMetrics, FRAME_HZ, VIEW_CSS};
        let entries = self.credits_entries();
        if entries.is_empty() {
            return;
        }
        let (cx, cy, cw, ch) = Self::content_rect(fb);
        // The CEF view is the centred 4:3 box at 960 CSS px of height.
        let s = ch as f32 / VIEW_CSS.1;
        let bw = ch as f32 * 4.0 / 3.0;
        let bx = cx as f32 + (cw as f32 - bw) / 2.0;
        let (lx, lw) = (bx + 0.23 * bw, 0.57 * bw);
        let (ly, lh) = (cy as f32 + 0.102 * ch as f32, 0.83 * ch as f32);
        if self.credits_view.layout.as_ref().map(|l| l.0) != Some(ch) {
            let metrics = |f: &Option<galaxycraft::text::UiFont>| {
                f.as_ref().and_then(|f| EmMetrics::from_font(f.bytes()))
            };
            let (Some(title), Some(body)) = (metrics(&self.ui_font_hover), metrics(&self.ui_font))
            else {
                return;
            };
            let layout = CreditsLayout::new(&entries, title, body, s);
            self.credits_view.layout = Some((ch, layout));
            self.credits_view.masks.clear();
        }
        // Run the page's requestAnimationFrame loop up to now (headless
        // captures pin the clock with `--menu-at`).
        let height = self
            .credits_view
            .layout
            .as_ref()
            .map_or(0.0, |l| l.1.height);
        let max_top = ((height - lh) / s).max(0.0);
        let elapsed = if self.headless {
            self.menu_at.unwrap_or(0.0) as f64
        } else {
            self.credits_view
                .since
                .map_or(0.0, |t| t.elapsed().as_secs_f64())
        };
        let target = ((elapsed * FRAME_HZ as f64) as u64)
            .saturating_sub(galaxycraft::credits::SHOWN_LATENCY_FRAMES);
        let view = &mut self.credits_view;
        let wheel = std::mem::take(&mut view.wheel);
        view.scroll.wheel(wheel, max_top);
        let mut budget = 200_000;
        while view.frames < target && budget > 0 {
            view.scroll.set_keys(view.keys.0, view.keys.1);
            view.scroll.frame(max_top);
            view.frames += 1;
            budget -= 1;
        }
        // Composited scrolling snaps the offset to device pixels; the
        // overflow clip is snapped too.
        let off = (view.scroll.top * s).round();
        let clip = (
            lx.round() as i32,
            ly.round() as i32,
            (lx + lw).round() as i32,
            (ly + lh).round() as i32,
        );
        let Some((_, layout)) = view.layout.take() else {
            return;
        };
        let mut ext = self.ui_font_hover.take();
        let mut fallback = view.fallback.take().unwrap_or_default();
        let green = Rgb::new(0xb4, 0xfc, 0x74);
        for (i, line) in layout.lines.iter().enumerate() {
            let base = (ly + line.baseline - off).round() as i32;
            let reach = (line.px * 1.5) as i32;
            if base + reach < clip.1 || base - reach > clip.3 {
                continue;
            }
            if line.rule {
                // `border: 1px inset #b4fc74`: dark top edge, light bottom.
                for (dy, c) in [(0, Rgb::new(0x5a, 0x7e, 0x3a)), (1, green)] {
                    let y = base + dy;
                    if y >= clip.1 && y < clip.3 {
                        fb.rect(clip.0, y, clip.2 - clip.0, 1, c);
                    }
                }
                continue;
            }
            if !self.credits_view.masks.contains_key(&i) {
                let missing = ext
                    .as_ref()
                    .is_some_and(|f| line.text.chars().any(|c| !f.has_glyph(c)));
                if missing && fallback.is_empty() {
                    fallback = self.credits_fallback_fonts();
                }
                if let Some(ext) = ext.as_mut() {
                    let m = render_credit_line(ext, &mut fallback, line, lx, lw, s);
                    self.credits_view.masks.insert(i, m);
                }
            }
            let Some(m) = self.credits_view.masks.get(&i) else {
                continue;
            };
            let (ox, oy) = (lx.floor() as i32 + m.x, base + m.top);
            for row in 0..m.h {
                let y = oy + row as i32;
                if y < clip.1 || y >= clip.3 {
                    continue;
                }
                for col in 0..m.w {
                    let x = ox + col as i32;
                    if x < clip.0 || x >= clip.2 {
                        continue;
                    }
                    let k = row * m.w + col;
                    let (t, sh) = (m.text[k], m.shadow[k]);
                    if t == 0 && sh == 0 {
                        continue;
                    }
                    let c = fb.get(x, y).blend(Rgb::new(0, 0, 0), sh).blend(green, t);
                    fb.put(x, y, c);
                }
            }
        }
        self.ui_font_hover = ext;
        self.credits_view.fallback = Some(fallback);
        self.credits_view.layout = Some((ch, layout));
    }
}
