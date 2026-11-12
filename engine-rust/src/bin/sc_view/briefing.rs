//! `impl App`: briefing (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    pub(super) fn handle_briefing(&mut self, window: &Window) {
        if self.ready.is_some() {
            // Moving onto a button ticks like the menus' hover.
            let hovered = self
                .pointer
                .and_then(|(mx, my)| self.ready_button_at(mx, my));
            if hovered != self.ready_hover {
                self.ready_hover = hovered;
                if hovered.is_some() {
                    self.play("ui_move");
                }
            }
            if (window.get_mouse_down(MouseButton::Left) || self.pad_frame.lmb)
                && !self.mouse_was_down
                && let Some((mx, my)) = self.pointer
                && let Some(key) = self.ready_button_at(mx, my)
            {
                self.briefing_key(key);
            }
            self.mouse_was_down = window.get_mouse_down(MouseButton::Left) || self.pad_frame.lmb;
        }
        let keys: Vec<Key> = window
            .get_keys_pressed(KeyRepeat::No)
            .into_iter()
            .chain(self.pad_frame.menu_pressed.iter().copied())
            .collect();
        for key in keys {
            self.briefing_key(key);
        }
    }

    /// One briefing key (also `--menu-script briefing-key:<Key>`).  In the
    /// ready room: Start (Enter/S), Replay (R), Cancel (Esc/C), as the
    /// `glurdyt` hotkeys.
    pub(super) fn briefing_key(&mut self, key: Key) {
        if self.ready.is_some() {
            match key {
                Key::Enter | Key::Space | Key::S => self.start_briefed_mission(),
                Key::R => self.enter_ready_room(),
                Key::T if self.ready.as_ref().is_some_and(|r| r.skip_tutorial) => {
                    self.skip_tutorial()
                }
                Key::Escape | Key::C => {
                    self.play("ui_click");
                    self.ready = None;
                    self.open_menu();
                }
                _ => {}
            }
            return;
        }
        let retail = !self.briefing_pages.is_empty();
        {
            match key {
                Key::Enter | Key::Space => {
                    if retail && self.briefing_page + 1 < self.briefing_pages.len() {
                        self.briefing_page += 1;
                        self.briefing_page_epoch = Instant::now();
                        self.refresh_briefing_poster();
                        self.play("transmission");
                    } else if self.ready_script.is_some() {
                        self.enter_ready_room();
                    } else {
                        self.start_briefed_mission();
                    }
                }
                Key::Right | Key::Down if retail => {
                    if self.briefing_page + 1 < self.briefing_pages.len() {
                        self.briefing_page += 1;
                        self.refresh_briefing_poster();
                        self.play("ui_move");
                    }
                }
                Key::Left | Key::Up if retail => {
                    self.briefing_page = self.briefing_page.saturating_sub(1);
                    self.refresh_briefing_poster();
                    self.play("ui_move");
                }
                Key::Escape | Key::M => {
                    self.play("ui_click");
                    self.open_menu();
                }
                _ => {}
            }
        }
    }

    /// "Skip Tutorial": Boot Camp counts as done (campaign progress only,
    /// no game in the statistics) and the next mission's briefing opens.
    /// INFERENCE (no retail recording of the button).
    pub(super) fn skip_tutorial(&mut self) {
        self.play("ui_click");
        self.ready = None;
        if let (Some(i), Some(mission)) = (self.active_index(), self.map_mission(self.map_index)) {
            self.profile_store.profiles[i].progress.record_win(mission);
            self.sync_profiles();
        }
        let next = self
            .map_mission(self.map_index)
            .and_then(galaxycraft::campaign_progress::after_victory)
            .and_then(|a| a.next)
            .and_then(|m| self.campaign_map_index(m));
        match next {
            Some(index) => self.start_campaign_mission(index),
            None => self.open_menu(),
        }
    }

    /// Leave the briefing and start the mission.
    pub(super) fn start_briefed_mission(&mut self) {
        self.ready = None;
        self.briefing = false;
        self.last_step = Instant::now();
        self.play("ui_click");
    }

    /// Start (or Replay) the ready-room script from its first action.
    pub(super) fn enter_ready_room(&mut self) {
        self.ready = self.ready_script.clone();
        self.ready_epoch = Instant::now();
        // The plates' build-in whooshes in like the glue dialogs'.
        if !self.headless {
            self.play("ui_swish");
        }
        self.ready_players =
            std::array::from_fn(|i| galaxycraft::portrait::PortraitPlayer::new(i as u32 + 1));
        self.ready_race = self
            .world
            .players
            .get(self.local_player)
            .map(|p| p.race)
            .unwrap_or(galaxycraft::sim::Race::Terran);
        let (dir, clip, projector) = match self.ready_race {
            galaxycraft::sim::Race::Zerg => ("readyz", "start.webm", false),
            galaxycraft::sim::Race::Protoss => ("readyp", "ring.webm", false),
            _ => ("readyt", "start.webm", true),
        };
        let headless = self.headless;
        self.ready_glow = MenuClip::open_at(
            self.assets_root.join(format!("glue/{dir}/{clip}")),
            0.0,
            headless,
        );
        // The Terran projector intro plays once per room; Zerg and Protoss
        // rooms have their own backdrop art and no projector.
        self.ready_proj = (projector && !headless && self.ready_at.is_none_or(|t| t < 4000))
            .then(|| MenuClip::open_at(self.assets_root.join("glue/readyt/proj.webm"), 0.0, false))
            .flatten();
        self.ready_proloop = projector
            .then(|| {
                MenuClip::open_at(
                    self.assets_root.join("glue/readyt/proloop.webm"),
                    0.0,
                    headless,
                )
            })
            .flatten();
        if self.ready.is_some() {
            self.update_briefing();
        }
    }

    /// Ready-room script clock (ms), pinned in scripted runs.  The script
    /// starts [`READY_SCRIPT_DELAY_MS`] after the room appears.
    pub(super) fn ready_clock_ms(&self) -> u64 {
        self.ready_at.unwrap_or_else(|| {
            (self.ready_epoch.elapsed().as_millis() as u64).saturating_sub(READY_SCRIPT_DELAY_MS)
        })
    }

    /// Whether the ready-room script has started (after the fade-in).
    pub(super) fn ready_script_running(&self) -> bool {
        self.ready_at.is_some()
            || self.ready_epoch.elapsed().as_millis() as u64 >= READY_SCRIPT_DELAY_MS
    }

    /// Advance the briefing: establishing-shot pages leave after their
    /// `DISPLAYTIME`, then the ready-room script runs and plays its WAVs.
    pub(super) fn update_briefing(&mut self) {
        if self.ready.is_none() {
            let shown = self.briefing_page_epoch.elapsed().as_millis() as u64;
            let key = (self.map_index, self.briefing_page);
            if self.briefing_due.map(|(m, p, _)| (m, p)) != Some(key) {
                let page = self.briefing_pages.get(self.briefing_page).cloned();
                let video_ms = page
                    .as_ref()
                    .and_then(|p| p.video.as_ref())
                    .zip(self.video_store.as_ref())
                    .and_then(|(v, store)| store.resolve(v))
                    .and_then(|path| galaxycraft::video::probe_info(&path))
                    .and_then(|info| info.duration_ms);
                let due = page.and_then(|p| p.due_ms(video_ms));
                self.briefing_due = Some((key.0, key.1, due));
            }
            let due = self
                .briefing_due
                .and_then(|(_, _, d)| d)
                .is_some_and(|ms| shown >= ms);
            if due && self.ready_at.is_none() && !self.headless {
                if self.briefing_page + 1 < self.briefing_pages.len() {
                    self.briefing_page += 1;
                    self.briefing_page_epoch = Instant::now();
                    self.refresh_briefing_poster();
                } else if self.ready_script.is_some() {
                    self.enter_ready_room();
                }
            }
            return;
        }
        if !self.ready_script_running() {
            return;
        }
        let now = self.ready_clock_ms();
        let events = self
            .ready
            .as_mut()
            .map(|r| r.advance(now))
            .unwrap_or_default();
        for galaxycraft::ready_room::ReadyEvent::Wav { string } in events {
            let samples = self
                .scenario
                .string(string)
                .zip(self.maps.get(self.map_index))
                .and_then(|(raw, map)| {
                    galaxycraft::transmission::find_map_wav(
                        &map.path,
                        raw,
                        self.game_options.original_voice_overs,
                    )
                })
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|bytes| galaxycraft::audio::decode_sound(&bytes));
            if let Some(samples) = samples {
                self.audio.play(Some(&samples));
            }
        }
    }

    /// Ready-room button under a window point: Start, Replay or Cancel
    /// (`glurdyt` FLCBTN 13 hot box, 20 and 14), as the key it stands for.
    pub(super) fn ready_button_at(&self, mx: i32, my: i32) -> Option<Key> {
        let dialog = self.glue.as_ref()?.ready_t.as_ref()?;
        let (cx, _, _, ch) = self.content_box;
        let k = ch as f32 / 1440.0;
        let hit = |r: UiRect| {
            let x0 = cx + ((320 + r.x * 3) as f32 * k) as i32;
            let y0 = (r.y as f32 * 3.0 * k) as i32;
            let (w, h) = ((r.w as f32 * 3.0 * k) as i32, (r.h as f32 * 3.0 * k) as i32);
            mx >= x0 && mx < x0 + w && my >= y0 && my < y0 + h
        };
        let control = |id: i32| dialog.controls.iter().find(|c| c.id == id).map(|c| c.abs);
        let start_hot = dialog
            .controls
            .iter()
            .find(|c| c.kind == "HOTBOX")
            .map(|c| c.abs);
        let skip = self
            .ready
            .as_ref()
            .is_some_and(|r| r.skip_tutorial)
            .then(|| control(19))
            .flatten();
        [
            (start_hot, Key::S),
            (control(20), Key::R),
            (control(14), Key::C),
            (skip, Key::T),
        ]
        .into_iter()
        .find(|(r, _)| r.is_some_and(hit))
        .map(|(_, k)| k)
    }

    pub(super) fn refresh_briefing_poster(&mut self) {
        self.briefing_player = None;
        self.briefing_player_key = None;
        self.briefing_poster = None;
        self.briefing_video = None;
        let Some(page) = self.briefing_pages.get(self.briefing_page) else {
            return;
        };
        let Some(video) = page.video.clone() else {
            return;
        };
        if let Some(store) = &self.video_store
            && let Some((_, poster)) = store.poster(&video)
        {
            self.briefing_poster = Some(poster);
            self.briefing_video = Some(video);
        }
    }

    pub(super) fn draw_briefing(&mut self, fb: &mut Framebuffer) {
        if self.ready.is_some() {
            self.draw_ready_room(fb);
            return;
        }
        if !self.briefing_pages.is_empty() {
            self.draw_retail_briefing(fb);
            return;
        }
        let w = fb.width;
        let h = fb.height;
        // Same deep-space backdrop as the menu.
        for y in 0..h {
            let t = y as f32 / h as f32;
            let c = Rgb::new(
                (6.0 + 18.0 * t) as u8,
                (10.0 + 24.0 * t) as u8,
                (24.0 + 40.0 * t) as u8,
            );
            fb.rect(0, y, w, 1, c);
        }
        for x in 0..80 {
            let sx = (x * 197 + 31) % w.max(1);
            let sy = (x * 89 + 17) % (h.max(1) / 2);
            fb.rect(sx, sy, 1, 1, Rgb::new(150, 160, 190));
        }
        let Some((name, briefing, objectives)) = briefing_content(&self.scenario) else {
            return;
        };
        let name = if name.trim().is_empty() {
            self.maps
                .get(self.map_index)
                .map(|m| m.name.clone())
                .unwrap_or_default()
        } else {
            name
        };
        let (px, py, pw, ph) = (60, 96, w - 120, h - 180);
        fb.rect(px, py, pw, ph, Rgb::new(12, 16, 26));
        fb.rect_outline(px, py, pw, ph, Rgb::new(70, 110, 150));
        fb.text_centered(w / 2, 28, "MISSION BRIEFING", Rgb::new(255, 214, 90), 3);
        fb.text_centered(w / 2, 66, &name, Rgb::new(225, 235, 245), 2);

        let text_x = px + 34;
        let mut y = py + 28;
        for line in wrap_text(&briefing, 96).into_iter().take(20) {
            fb.text(text_x, y, &line, Rgb::new(200, 210, 220), 1);
            y += 14;
        }
        if !objectives.trim().is_empty() {
            y += 18;
            fb.text(text_x, y, "OBJECTIVES", Rgb::new(255, 210, 100), 2);
            y += 26;
            for line in wrap_text(&objectives, 96).into_iter().take(8) {
                fb.text(text_x, y, &line, Rgb::new(240, 225, 160), 1);
                y += 14;
            }
        }
        fb.text_centered(
            w / 2,
            h - 62,
            "ENTER: deploy     ESC: back to menu",
            Rgb::new(150, 180, 210),
            1,
        );
    }

    /// The Terran ready room (`glurdyt.ui.json`, 640x480 at 3x in the 16:9
    /// content box): `palrt` backdrop, the four portrait slots, the HD
    /// frames (anchored top-left, or top-right with `RC_ANCHOR_TOPRIGHT`,
    /// the HD art being wider than the SD rect), the transmission box, the
    /// objectives and the Start / Replay / Cancel labels.
    pub(super) fn draw_ready_room(&mut self, fb: &mut Framebuffer) {
        fb.clear(Rgb::new(0, 0, 0));
        let Some(glue) = self.glue.take() else {
            return;
        };
        self.draw_ready_room_with(fb, &glue);
        self.glue = Some(glue);
    }

    pub(super) fn draw_ready_room_with(&mut self, fb: &mut Framebuffer, glue: &GlueArt) {
        let (cx, _cy, cw, ch) = Self::content_rect(fb);
        let k = ch as f32 / 1440.0;
        let (dialog, backdrop, art, frames) = match self.ready_race {
            galaxycraft::sim::Race::Zerg => (
                &glue.ready_z,
                &glue.ready_backdrop_z,
                &glue.ready_art_z,
                &glue.ready_slot_frames_z,
            ),
            galaxycraft::sim::Race::Protoss => (
                &glue.ready_p,
                &glue.ready_backdrop_p,
                &glue.ready_art_p,
                &glue.ready_slot_frames_p,
            ),
            _ => (
                &glue.ready_t,
                &glue.ready_backdrop,
                &glue.ready_art,
                &glue.ready_slot_frames,
            ),
        };
        let Some(dialog) = dialog.as_ref() else {
            return;
        };
        if let Some(bg) = backdrop {
            bg.draw_scaled(fb, cx, 0, cw, ch);
        }
        let sx = |ux: f32| cx + ((320.0 + ux * 3.0) * k).round() as i32;
        let sy = |uy: f32| (uy * 3.0 * k).round() as i32;
        let sl = |u: f32| (u * 3.0 * k).round() as i32;
        // Start hologram: `rez/flc.json` places `glue\ReadyT\start.webm`
        // at [28, -50, 197, 289] from FLCBTN 13; it is `FLC_FADED` (half
        // alpha, fitted on the recording: 17.7 -> 14.5 in the region) until
        // the pointer is on Start.
        if let Some(button) = dialog.controls.iter().find(|c| c.id == 13).map(|c| c.abs) {
            let hot = self
                .pointer
                .and_then(|(mx, my)| self.ready_button_at(mx, my))
                == Some(Key::S);
            // Projector: the one-shot intro until it ends, then the loop.
            let proj_running = self.ready_proj.as_ref().is_some_and(|c| {
                c.started.elapsed().as_secs_f32() < c.info.duration_ms.unwrap_or(0) as f32 / 1000.0
            });
            let proj = if proj_running {
                self.ready_proj.as_mut()
            } else {
                self.ready_proloop.as_mut()
            };
            if let Some(frame) = proj.and_then(|c| c.frame()) {
                // Opaque wall-and-projector footage (alpha 255 throughout):
                // drawn over the backdrop, never added to it.
                let (ux, uy) = (button.x as f32 + 42.0, button.y as f32 - 50.0);
                draw_poster_rgba_additive(
                    fb,
                    frame,
                    (sx(ux), sy(uy), sl(197.0), sl(289.0)),
                    0.0,
                    false,
                );
            }
            if let Some(frame) = self.ready_glow.as_mut().and_then(|c| c.frame()) {
                let (ux, uy) = (button.x as f32 + 28.0, button.y as f32 - 50.0);
                draw_poster_rgba_additive(
                    fb,
                    frame,
                    (sx(ux), sy(uy), sl(197.0), sl(289.0)),
                    if hot { 0.0 } else { 0.5 },
                    false,
                );
            }
        }
        let now = self.ready_clock_ms();
        let Some(room) = self.ready.clone() else {
            return;
        };
        // Build-in (Boot Camp recording 38.3-39.1 s): the empty room first,
        // its plates sliding in from the screen edges, the portraits, text
        // and labels once they are in place.  Pinned captures draw the
        // settled room.
        let built = self.headless || self.ready_at.is_some();
        let build_t = if built {
            f32::MAX
        } else {
            self.ready_epoch.elapsed().as_secs_f32()
        };
        let step = (build_t * galaxycraft::menu::SWISH_STEPS_PER_SEC) as u32;
        for (id, img) in art {
            let Some(c) = dialog.controls.iter().find(|c| c.id == *id) else {
                continue;
            };
            let visible = c.has_flag("WS_VISIBLE") || (*id == -13 && room.skip_tutorial);
            if !visible {
                continue;
            }
            let w = (img.width as f32 * 0.75 * k).round() as i32;
            let h = (img.height as f32 * 0.75 * k).round() as i32;
            let r = c.abs;
            let x = if c.has_flag("RC_ANCHOR_TOPRIGHT") {
                sx((r.x + r.w) as f32) - w
            } else {
                sx(r.x as f32)
            };
            // Left-half plates come from the left edge, the others from the
            // right; each starts just off screen.
            let from_left = r.x + r.w / 2 < 320;
            let dist = if from_left {
                (r.x + r.w) as f32
            } else {
                (640 - r.x) as f32
            };
            let off = if built {
                0.0
            } else {
                galaxycraft::menu::swish_in_offset(dist, step)
            };
            let dx = (off * 3.0 * k).round() as i32;
            let x = if from_left { x - dx } else { x + dx };
            img.draw_scaled(fb, x, sy(r.y as f32), w, h);
        }
        if build_t < READY_BUILD_SECS {
            return;
        }
        // Portraits over the frames (the layout lists the `gluRdyT-5..8`
        // slots, ids 15..18, after the RECT art).
        for (slot, id) in [15, 16, 17, 18].into_iter().enumerate() {
            let (Some(unit), Some(c)) = (
                room.portraits[slot],
                dialog.controls.iter().find(|c| c.id == id),
            ) else {
                continue;
            };
            let talking = room.speaking.is_some_and(|(s, _)| s == slot);
            let r = c.abs;
            if let Some(frame) = self.ready_portrait_frame(slot, unit, talking, now as f64 / 1000.0)
            {
                let (fx, fy, fw, fh) = READY_PORTRAIT_FACE;
                let (px, py, pw, ph) = (sx(r.x as f32 + fx), sy(r.y as f32 + fy), sl(fw), sl(fh));
                blit_opaque(&frame, fb, px, py, pw, ph);
                // Transmission scanlines: every other UI row at half
                // brightness (retail: 6 content px period, dark rows where
                // y/3 is odd, +-40 luminance).
                for y in py.max(0)..(py + ph).min(fb.height) {
                    if ((y as f32 / (3.0 * k)) as i32) % 2 == 1 {
                        let row = y as usize * fb.width as usize;
                        for x in px.max(0)..(px + pw).min(fb.width) {
                            let p = &mut fb.pixels[row + x as usize];
                            *p = Rgb(*p).blend(Rgb::new(0, 0, 0), READY_SCANLINE_DIM).0;
                        }
                    }
                }
            }
            if let Some((idle, hot)) = frames.get(slot)
                && let Some(img) = if talking { hot.as_ref() } else { idle.as_ref() }
            {
                let w = (img.width as f32 * 0.75 * k).round() as i32;
                let h = (img.height as f32 * 0.75 * k).round() as i32;
                img.draw_scaled(fb, sx(r.x as f32), sy(r.y as f32), w, h);
            }
        }
        let to_content_x = |ux: f32| 320.0 + ux * 3.0;
        let text_color = GLUE_INFO;
        // Transmission text (LTEXT -10): wrapped to the box and scrolled up
        // over the line's duration, clipped to the box.
        if let Some(c) = dialog.controls.iter().find(|c| c.id == -10) {
            let r = c.abs;
            let mut lines: Vec<(String, f32)> = Vec::new();
            if let Some(line) = room.transmission {
                let text = self.scenario.string(line.string).unwrap_or("").to_string();
                let max_w = r.w as f32 * 3.0;
                for para in text.replace("\r\n", "\n").split('\n') {
                    let words = wrap_words(para.trim(), max_w as i32, |t| {
                        self.glue_measure(t, READY_TEXT_PX, GlueFace::Hud) as i32
                    });
                    for w in words {
                        lines.push((w, 0.0));
                    }
                }
                let box_h = r.h as f32 * 3.0;
                let text_h = lines.len() as f32 * READY_TEXT_PITCH;
                let _ = box_h;
                let scroll = ready_text_scroll(now.saturating_sub(line.start_ms), line.ms, text_h);
                let mut clip = Framebuffer::new(fb.width as usize, fb.height as usize);
                clip.pixels.copy_from_slice(&fb.pixels);
                for (i, (t, _)) in lines.iter().enumerate() {
                    let base =
                        r.y as f32 * 3.0 + READY_TEXT_BASE + i as f32 * READY_TEXT_PITCH - scroll;
                    // Lines partly inside the box are drawn and clipped, as
                    // retail shows the next line cut at the bottom edge.
                    if base < r.y as f32 * 3.0 || base > (r.y + r.h) as f32 * 3.0 + READY_TEXT_PX {
                        continue;
                    }
                    self.glue_text_face(
                        &mut clip,
                        to_content_x(r.x as f32),
                        base,
                        t,
                        READY_TEXT_PX,
                        text_color,
                        None,
                        GlueFace::Hud,
                    );
                }
                let (y0, y1) = (sy(r.y as f32).max(0), sy((r.y + r.h) as f32).min(fb.height));
                let stride = fb.width as usize;
                for y in y0..y1 {
                    let row = y as usize * stride;
                    fb.pixels[row..row + stride].copy_from_slice(&clip.pixels[row..row + stride]);
                }
            }
            // Text Message (CTEXT -14): "End of Briefing." centred.
            if let Some(m) = room.message.filter(|m| now < m.start_ms + m.ms as u64) {
                let text = self
                    .scenario
                    .string(m.string)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let w = self.glue_measure(&text, READY_TEXT_PX, GlueFace::Hud);
                self.glue_text_face(
                    fb,
                    to_content_x(r.x as f32 + r.w as f32 / 2.0) - w / 2.0,
                    r.y as f32 * 3.0 + READY_TEXT_BASE,
                    &text,
                    READY_TEXT_PX,
                    text_color,
                    None,
                    GlueFace::Hud,
                );
            }
        }
        // Objectives heading (LTEXT -1, `gluRdy?-13`) and list (LTEXT -11).
        let prefix = match self.ready_race {
            galaxycraft::sim::Race::Zerg => "GluRdyZ",
            galaxycraft::sim::Race::Protoss => "gluRdyP",
            _ => "gluRdyT",
        };
        let objectives = self.ui_text(&format!("{prefix}-13"), "Objectives");
        if let Some(c) = dialog.controls.iter().find(|c| c.id == -1) {
            self.glue_text_face(
                fb,
                to_content_x(c.abs.x as f32),
                c.abs.y as f32 * 3.0 + READY_HEAD_BASE,
                &objectives,
                READY_HEAD_PX,
                text_color,
                None,
                GlueFace::Big,
            );
        }
        if let (Some(c), Some(obj)) = (
            dialog.controls.iter().find(|c| c.id == -11),
            room.objectives
                .and_then(|s| self.scenario.string(s))
                .map(str::to_string),
        ) {
            for (i, line) in obj
                .replace("\r\n", "\n")
                .split('\n')
                .filter(|l| !l.trim().is_empty())
                .enumerate()
            {
                self.glue_text_face(
                    fb,
                    to_content_x(c.abs.x as f32),
                    c.abs.y as f32 * 3.0 + READY_OBJ_BASE + i as f32 * READY_OBJ_PITCH,
                    line.trim(),
                    READY_OBJ_PX,
                    text_color,
                    None,
                    GlueFace::Hud,
                );
            }
        }
        // Buttons: Start label in the FLCBTN 13 text box, Replay (20) and
        // Cancel (14) centred in their rects.
        let start = dialog
            .controls
            .iter()
            .find(|c| c.kind == "TEXTBOX" && c.parent.is_some())
            .map(|c| {
                // The child box reads (left, top, right, bottom).
                let p = dialog.controls[c.parent.unwrap_or(0)].abs;
                let (l, t, rr, b) = (c.rect.x, c.rect.y, c.rect.w, c.rect.h);
                ((p.x + (l + rr) / 2) as f32, (p.y + (t + b) / 2) as f32)
            });
        let centre = |id: i32| {
            dialog.controls.iter().find(|c| c.id == id).map(|c| {
                (
                    (c.abs.x as f32 + c.abs.w as f32 / 2.0),
                    (c.abs.y as f32 + c.abs.h as f32 / 2.0),
                )
            })
        };
        // Boot Camp's briefing enables "Skip Tutorial" (MBRF action 9):
        // `gluRdyT-3` on the TutBtn plate (drawn with the RECT art above).
        let skip = room.skip_tutorial.then(|| centre(19)).flatten();
        // The pointed button's label lights up (Boot Camp recording 55-63 s,
        // Start and Replay under the pointer), as the glue buttons do.
        let hovered = self
            .pointer
            .and_then(|(mx, my)| self.ready_button_at(mx, my));
        for (label, at, key) in [
            (self.ui_text(&format!("{prefix}-1"), "Start"), start, Key::S),
            (
                self.ui_text(&format!("{prefix}-4"), "Replay"),
                centre(20),
                Key::R,
            ),
            (
                self.ui_text(&format!("{prefix}-2"), "Cancel"),
                centre(14),
                Key::C,
            ),
            (
                self.ui_text(&format!("{prefix}-3"), "Skip Tutorial"),
                skip,
                Key::T,
            ),
        ] {
            let Some((ux, uy)) = at else {
                continue;
            };
            let colour = if hovered == Some(key) {
                GLUE_BRIGHT
            } else {
                GLUE_GREEN
            };
            let w = self.glue_measure(&label, GLUE_BIGGER_PX, GlueFace::Bigger);
            self.glue_text_face(
                fb,
                to_content_x(ux) - w / 2.0,
                uy * 3.0 + GLUE_BIGGER_PX * GLUE_CAP / 2.0,
                &label,
                GLUE_BIGGER_PX,
                colour,
                None,
                GlueFace::Bigger,
            );
        }
    }

    /// Animated portrait of a ready-room slot (idle or talking clips, as
    /// the HUD portrait).
    pub(super) fn ready_portrait_frame(
        &mut self,
        slot: usize,
        type_id: u16,
        talking: bool,
        now: f64,
    ) -> Option<PosterFrame> {
        let portrait = self.data.unit(type_id).portrait;
        let (dat, _) = self.portrait_tables()?.clone();
        let cur = self.ready_players[slot].current;
        if cur.map(|c| (c.portrait, c.talking)) != Some((portrait, talking)) {
            self.ready_players[slot].set(&dat, Some(portrait), talking, true, now);
        }
        let fps = galaxycraft::portrait::PORTRAIT_FPS as f64;
        for _ in 0..256 {
            let clip = self.ready_players[slot].current?;
            let frames = self.portrait_clip_frames(clip)?;
            let at = ((now - self.ready_players[slot].started).max(0.0) * fps) as usize;
            if at < frames.len() {
                return Some(frames[at].clone());
            }
            let ended = self.ready_players[slot].started + frames.len() as f64 / fps;
            self.ready_players[slot].clip_ended(&dat, &mut self.crt_rand, talking, ended);
        }
        None
    }

    /// Draw a page of the retail briefing script over its shipped backdrop.
    /// The establishing-shot scripts place the location copy at the lower
    /// left of the screen; the intermission pages centre their text.
    pub(super) fn draw_retail_briefing(&mut self, fb: &mut Framebuffer) {
        let w = fb.width as usize;
        let h = fb.height as usize;
        let page = &self.briefing_pages[self.briefing_page];
        let lower_left = page.screen_lower_left;
        let text = page.text.clone();
        let video = page.video.clone();

        // Fit the retail backdrop once per window size.
        let stale = self
            .briefing_fit
            .as_ref()
            .map(|(fw, fh, _)| *fw != w || *fh != h)
            .unwrap_or(true);
        if stale && let Some(art) = &self.briefing_art {
            self.briefing_fit = Some((w, h, art.cover(w, h)));
        }
        if let Some((fw, fh, art)) = &self.briefing_fit {
            for y in 0..(*fh).min(h) {
                for x in 0..(*fw).min(w) {
                    let p = art.pixels[y * *fw + x];
                    fb.pixels[y * w + x] = p & 0x00ff_ffff;
                }
            }
        } else {
            fb.clear(Rgb::new(4, 5, 10));
        }

        // Every offset below is in 1440p pixels scaled by `k`, and text is
        // in UI px (`ui_label`): the page keeps its layout from 720p up
        // (`scripts/resolution_diff.py`).  It used fixed screen pixels and
        // whole-size bitmap text, which moved and enlarged the copy below
        // 1440p.
        let k = h as f32 / 1440.0;
        let kp = |v: f32| (v * k).round() as i32;
        // A subtle vignette keeps the copy legible over bright artwork.
        // The splash has no shading under its copy (38 s); video pages get
        // their own band below.
        let shade = if lower_left || video.is_some() {
            0.0
        } else {
            0.35
        };
        let band_top = if lower_left {
            (h * 42 / 100) as i32
        } else {
            h as i32 / 2 - kp(140.0)
        };
        let band_h = if lower_left {
            (h * 46 / 100) as i32
        } else {
            kp(300.0)
        };
        for y in band_top..(band_top + band_h).min(h as i32) {
            let t = (y - band_top) as f32 / band_h.max(1) as f32;
            let a = (t * shade * 255.0) as u8;
            for x in 0..w {
                let c = fb.pixels[y as usize * w + x];
                let base = Rgb(c & 0x00ff_ffff);
                fb.pixels[y as usize * w + x] = base.blend(Rgb::new(0, 0, 0), a).0;
            }
        }

        // Intermission clip: stream it at 12 fps while the page is open; the
        // poster stays as the first paint and as the fallback without ffmpeg.
        if let Some(rel) = self.briefing_video.clone() {
            let stale = self.briefing_player_key.as_deref() != Some(rel.as_str());
            if stale {
                self.briefing_player = None;
                self.briefing_player_key = Some(rel.clone());
                self.briefing_last = Instant::now();
                if let Some(store) = &self.video_store {
                    // The script names the clip `interstitials\TR1010.webm`
                    // while the cache stores `interstitials/tr1010.webm`;
                    // `resolve` normalises the separators and the case.
                    if let Some(path) = store.resolve(&rel)
                        && let Some(info) = galaxycraft::video::probe_info(&path)
                    {
                        self.briefing_player =
                            galaxycraft::cinematic::Cinematic::open(&path, &info, 12.0, 960);
                    }
                }
            }
        }
        let dt = self.briefing_last.elapsed().as_secs_f32();
        self.briefing_last = Instant::now();
        let streamed = self.briefing_player.as_mut().and_then(|p| p.advance(dt));
        let (cx, _, cw, ch) = Self::content_rect(fb);
        if video.is_some()
            && let Some(poster) = streamed.clone().or_else(|| self.briefing_poster.clone())
        {
            // Rebel Yell's text pages (Boot Camp recording 8-35 s): the clip
            // fills the content box (cover), no frame around it.
            let (pw, ph) = (poster.width as f32, poster.height as f32);
            let scale = (cw as f32 / pw).max(ch as f32 / ph);
            let (dw, dh) = ((pw * scale) as i32, (ph * scale) as i32);
            let (ox, oy) = (cx + (cw - dw) / 2, (ch - dh) / 2);
            for y in 0..ch.min(h as i32) {
                let sy = ((y - oy) as f32 / scale) as i32;
                if sy < 0 || sy >= poster.height as i32 {
                    continue;
                }
                for x in cx..(cx + cw).min(w as i32) {
                    let sx = ((x - ox) as f32 / scale) as i32;
                    if sx < 0 || sx >= poster.width as i32 {
                        continue;
                    }
                    let o = (sy as usize * poster.width as usize + sx as usize) * 3;
                    fb.pixels[y as usize * w + x as usize] =
                        Rgb::new(poster.rgb[o], poster.rgb[o + 1], poster.rgb[o + 2]).0;
                }
            }
        }
        let k = ch as f32 / 1440.0;
        if text.trim().is_empty() {
            return;
        }
        if video.is_some() {
            // The copy sits on a dark band across the bottom of the clip:
            // pale green BlizzardGlobal, title, a blank line, then the body
            // (measured at 10 s: x 460, baselines 1228 / 1342 / 1392 of
            // 1440).  The text is complete at once; the page waits for a
            // click (`BriefingPage::due_ms`).
            let band_top = (BRIEF_BAND_TOP * k) as i32;
            for y in band_top.max(0)..ch.min(h as i32) {
                for x in cx..(cx + cw).min(w as i32) {
                    let c = Rgb(fb.pixels[y as usize * w + x as usize] & 0x00ff_ffff);
                    fb.pixels[y as usize * w + x as usize] = c.blend(Rgb::new(0, 0, 0), 120).0;
                }
            }
            let max_w = 1560.0;
            let mut lines: Vec<String> = Vec::new();
            for para in text.lines() {
                if para.trim().is_empty() {
                    lines.push(String::new());
                    continue;
                }
                let mut line = String::new();
                for word in para.split_whitespace() {
                    let cand = if line.is_empty() {
                        word.to_string()
                    } else {
                        format!("{line} {word}")
                    };
                    if !line.is_empty()
                        && self.glue_measure(&cand, BRIEF_TEXT_PX, GlueFace::Hud) > max_w
                    {
                        lines.push(std::mem::take(&mut line));
                        line = word.to_string();
                    } else {
                        line = cand;
                    }
                }
                lines.push(line);
            }
            let mut base = BRIEF_TEXT_BASE;
            for line in &lines {
                if line.is_empty() {
                    base += BRIEF_BLANK_PITCH;
                    continue;
                }
                for (d, colour) in [(2.0, Rgb::new(0, 0, 0)), (0.0, BRIEF_GREEN)] {
                    self.glue_text_face(
                        fb,
                        BRIEF_TEXT_X + d,
                        base + d,
                        line,
                        BRIEF_TEXT_PX,
                        colour,
                        None,
                        GlueFace::Hud,
                    );
                }
                base += BRIEF_LINE_PITCH;
            }
            return;
        }
        // Mission splash (Boot Camp 36-42 s): the location copy at the lower
        // left in pale green Eurostile with a drop shadow, the quoted mission
        // name smaller above it (x 65, pitch 60, last baseline ~1347).
        // Blank lines of the script keep their pitch (the gap under the
        // quoted name, 38 s: baselines 1120 / 1236 / 1292 / 1352).
        let lines: Vec<&str> = text.trim_matches('\n').lines().collect();
        let n = lines.len() as f32;
        let mut base = SPLASH_LAST_BASE - (n - 1.0) * SPLASH_PITCH;
        for line in &lines {
            if !line.trim().is_empty() {
                for (d, colour) in [(3.0, Rgb::new(0, 0, 0)), (0.0, BRIEF_GREEN)] {
                    self.glue_text_face(
                        fb,
                        SPLASH_X + d,
                        base + d,
                        line.trim(),
                        SPLASH_PX,
                        colour,
                        None,
                        GlueFace::Bigger,
                    );
                }
            }
            base += SPLASH_PITCH;
        }
        let _ = lower_left;
    }
}
