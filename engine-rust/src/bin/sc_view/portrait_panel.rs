//! The console portrait: talking-head clip tables and drawing.

use super::*;

impl App {
    /// `portdata.dat` + `portdata.tbl`, read once from the install.
    pub(super) fn portrait_tables(
        &mut self,
    ) -> Option<&(
        galaxycraft::portrait::PortData,
        galaxycraft::tables::StringTable,
    )> {
        if self.portrait_data.is_none() {
            let install = self.install.as_ref();
            self.portrait_data = Some((|| {
                let install = install?;
                let dat =
                    galaxycraft::portrait::PortData::parse(&install.read("arr/portdata.dat")?)?;
                let tbl =
                    galaxycraft::tables::StringTable::parse(&install.read("arr/portdata.tbl")?)
                        .ok()?;
                Some((dat, tbl))
            })());
        }
        self.portrait_data.as_ref()?.as_ref()
    }

    /// Decoded frames of portrait clip `clip`, falling back to lower clip
    /// numbers when a file is missing (retail tries `c` down to 0).
    pub(super) fn portrait_clip_frames(
        &mut self,
        c: galaxycraft::portrait::PortraitClip,
    ) -> Option<std::rc::Rc<Vec<PosterFrame>>> {
        let (dat, tbl) = self.portrait_tables()?;
        let paths: Vec<String> = (0..=c.clip)
            .rev()
            .filter_map(|n| galaxycraft::portrait::clip_path(dat, tbl, c, n))
            .collect();
        for path in paths {
            if let Some(hit) = self.portrait_clips.get(&path) {
                if hit.is_some() {
                    return hit.clone();
                }
                continue;
            }
            let decoded = (|| {
                let install = self.install.as_ref()?;
                let bytes = install.read_by_path(&path).ok()?;
                let info = galaxycraft::video::parse_webm(&bytes).ok()?;
                let frames = galaxycraft::video::decode_frames(
                    &bytes,
                    &info,
                    0.0,
                    64,
                    galaxycraft::portrait::PORTRAIT_FPS,
                    256,
                );
                (!frames.is_empty()).then(|| std::rc::Rc::new(frames))
            })();
            self.portrait_clips.insert(path, decoded.clone());
            if decoded.is_some() {
                return decoded;
            }
        }
        None
    }

    /// `statport` button 1: the selected unit's idle portrait (the race
    /// advisor for buildings, via units.dat -> portdata).
    pub(super) fn draw_hud_portrait(&mut self, fb: &mut Framebuffer) {
        let Some(type_id) = self.portrait_unit() else {
            return;
        };
        let port = self.hud_control("statport", 1, UiRect::new(413, 410, 60, 56));
        // Geometry fitted on 135051 / 135109 / 135114 / 135121 (1440p, all
        // four agree within 2 px): the 224x240 clip is stretched into a
        // 160x164 rect at (+11, +17) of the control, and `conover.dds` at
        // 0.75 is centred (-9, +18) from the control's centre.  Portrait
        // region error 24.6 -> 8.4 on 135051.
        let k = self.content_box.3 as f32 / 1440.0;
        let px = |v: f32| (v * k).round() as i32;
        // Options: 0 Animated, 1 Still, 2 Disabled.  Recorded capture state
        // and headless captures show a fixed frame, unless `--menu-at` pins
        // the clip clock (the portrait then plays from 0 to that second).
        let clock = if self.headless {
            self.menu_at.map(f64::from)
        } else {
            Some(self.portrait_epoch.elapsed().as_secs_f64())
        };
        let frame = match (self.game_options.portraits, clock) {
            (2, _) => None,
            (0, Some(now)) if self.portrait_pick.is_none() => self
                .portrait_motion_frame(type_id, now)
                .or_else(|| self.portrait_frame(type_id)),
            _ => self.portrait_frame(type_id),
        };
        if let Some(frame) = frame {
            let (fx, fy, fw, fh) = PORTRAIT_FACE;
            // The clips are opaque (the advisor's dark backdrop hides the
            // well behind it).
            blit_opaque(&frame, fb, port.x + px(fx), port.y + px(fy), px(fw), px(fh));
        }
        if let Some(img) = self.console.as_ref().and_then(|c| c.con_over.as_ref()) {
            let w = (img.width as f32 * 0.75 * k).round() as i32;
            let h = (img.height as f32 * 0.75 * k).round() as i32;
            let cx = port.x + port.w / 2 + px(PORTRAIT_FRAME_SHIFT.0);
            let cy = port.y + port.h / 2 + px(PORTRAIT_FRAME_SHIFT.1);
            img.draw_scaled(fb, cx - w / 2, cy - h / 2, w, h);
        }
    }
}
