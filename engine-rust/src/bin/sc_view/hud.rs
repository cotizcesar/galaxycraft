//! `impl App`: hud (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    /// Select a few local units so headless captures show the selection HUD
    /// the retail screenshots have (wireframes + portrait).
    /// Decode (and cache) the retail painted portrait for a unit type.
    ///
    /// Portraits ship as short WebM clips under `portrait/<folder>/`; the
    /// folder is `<race letter><unit name, truncated to 8 chars>` and the
    /// first frame file is `<first three letters>fid00.webm`. Any miss is
    /// cached as `None` so the caller falls back to the wireframe.
    pub(super) fn portrait_frame(&mut self, type_id: u16) -> Option<PosterFrame> {
        if let Some(hit) = self.portraits.get(&type_id) {
            return hit.clone();
        }
        let (clip, at) = self.portrait_pick.unwrap_or((0, 0.6));
        let path = (|| {
            let portrait = self.data.unit(type_id).portrait;
            let (dat, tbl) = self.portrait_tables()?;
            let c = galaxycraft::portrait::PortraitClip {
                portrait,
                talking: false,
                clip,
            };
            galaxycraft::portrait::clip_path(dat, tbl, c, clip)
        })();
        let bytes = path
            .as_ref()
            .and_then(|p| self.install.as_ref()?.read_by_path(p).ok());
        let still = move |bytes: &[u8]| {
            let info = galaxycraft::video::parse_webm(bytes).ok()?;
            galaxycraft::video::decode_frame_at(bytes, &info, Some(at), 256)
        };
        // A live game decodes on a worker thread (see `start_decode`).
        if let (false, Some(path)) = (self.headless, &path) {
            let key = format!("still {at} {path}");
            match portrait_panel::poll_decode(&key) {
                portrait_panel::Decode::Ready(mut f) => {
                    let decoded = f.pop();
                    self.portraits.insert(type_id, decoded.clone());
                    return decoded;
                }
                portrait_panel::Decode::Pending => return None,
                portrait_panel::Decode::Idle => {
                    if let Some(bytes) = bytes {
                        portrait_panel::start_decode(key, move || {
                            still(&bytes).into_iter().collect()
                        });
                        return None;
                    }
                }
            }
        }
        let decoded = bytes.as_deref().and_then(still);
        if std::env::var("GC_DEBUG_PORTRAIT").is_ok() {
            eprintln!(
                "portrait type={type_id} name={:?} install={} -> {}",
                self.data.unit_name(type_id),
                self.install.is_some(),
                if decoded.is_some() { "ok" } else { "none" }
            );
        }
        self.portraits.insert(type_id, decoded.clone());
        decoded
    }

    /// Full-motion portrait frame of `type_id` now: idle clips re-picked by
    /// weight as each ends, talking clips while its voice line plays.
    pub(super) fn portrait_motion_frame(&mut self, type_id: u16, now: f64) -> Option<PosterFrame> {
        let portrait = self.data.unit(type_id).portrait;
        // A headless capture plays the portrait from second 0.
        let start = if self.headless { 0.0 } else { now };
        let talking = self
            .portrait_talk
            .is_some_and(|(t, until)| t == type_id && Instant::now() < until);
        let start_talk = std::mem::take(&mut self.portrait_talk_started) && talking;
        let (dat, _) = self.portrait_tables()?.clone();
        let cur = self.portrait_player.current;
        if cur.map(|c| c.portrait) != Some(portrait)
            || (start_talk && !cur.is_some_and(|c| c.talking))
        {
            self.portrait_player
                .set(&dat, Some(portrait), talking, true, start);
        }
        let fps = galaxycraft::portrait::PORTRAIT_FPS as f64;
        // Each clip starts where the previous one ended (bounded catch-up).
        for _ in 0..256 {
            let clip = self.portrait_player.current?;
            let frames = self.portrait_clip_frames(clip)?;
            let at = ((now - self.portrait_player.started).max(0.0) * fps) as usize;
            if at < frames.len() {
                return Some(frames[at].clone());
            }
            let ended = self.portrait_player.started + frames.len() as f64 / fps;
            self.portrait_player
                .clip_ended(&dat, &mut self.crt_rand, talking, ended);
        }
        None
    }

    /// First frame of the episode-select art for a race, with alpha.
    /// One Collections thumbnail, decoded from the shipped image.
    pub(super) fn thumb(&mut self, name: &str, w: i32, h: i32) -> Option<PosterFrame> {
        let key = format!("{name}@{w}x{h}");
        if let Some(hit) = self.thumbs.get(&key) {
            return hit.clone();
        }
        let decoded = (|| {
            let install = self.install.as_ref()?;
            let bytes = install
                .read_by_path(&format!("webui/dist/lib/images/{name}"))
                .ok()?;
            let ext = name.rsplit('.').next().unwrap_or("png");
            galaxycraft::video::decode_still(&bytes, ext, w.max(1) as u32, h.max(1) as u32)
        })();
        self.thumbs.insert(key, decoded.clone());
        decoded
    }

    /// One RGBA command/unit icon from the web UI, decoded on demand.
    pub(super) fn hotkey_icon(&mut self, name: &str, w: i32, h: i32) -> Option<RgbaFrame> {
        // Decode at the texture's native 128 px and pre-scale once with the
        // Catmull-Rom filter (CEF/Skia's downscale), then the per-frame blit
        // is a straight copy.
        let key = format!("{name}@{w}x{h}");
        if let Some(hit) = self.hotkey_icons.get(&key) {
            return hit.clone();
        }
        let decoded = (|| {
            let install = self.install.as_ref()?;
            let bytes = install
                .read_by_path(&format!("webui/dist/lib/images/{name}"))
                .ok()?;
            let native = galaxycraft::video::decode_still_rgba(&bytes, "png", 128, 128)?;
            if std::env::var("GC_ICON_FILTER").ok().as_deref() == Some("area") {
                Some(resize_rgba_area(&native, w.max(1) as u32, h.max(1) as u32))
            } else {
                Some(native)
            }
        })();
        self.hotkey_icons.insert(key, decoded.clone());
        decoded
    }

    pub(super) fn minimap_rect(&self) -> (i32, i32, i32, i32) {
        let r = self.hud_control("minimap", 1, UiRect::new(6, 348, 128, 128));
        (r.x, r.y, r.w, r.h)
    }

    /// Screen rectangle of a console-dialog rect given in absolute classic
    /// 640x480 coordinates (anchoring: [`console_dialog_shift`]).
    pub(super) fn hud_rect(&self, dialog: &str, r: UiRect) -> UiRect {
        let (cx, cy, cw, ch) = self.content_box;
        let s = (ch as f32 / 480.0).max(0.5);
        let shift = console_dialog_shift(dialog, self.view.0, cw, s)
            + minimap_centering_shift(self.game_options.minimap_position, dialog, cw, s);
        let x = cx as f32 + (r.x as f32 + shift) * s;
        UiRect::new(
            x.round() as i32,
            cy + (r.y as f32 * s).round() as i32,
            (r.w as f32 * s).round() as i32,
            (r.h as f32 * s).round() as i32,
        )
    }

    /// [`Self::hud_rect`] of control `id` of a shipped console dialog.
    pub(super) fn hud_control(&self, dialog: &str, id: i32, fallback: UiRect) -> UiRect {
        let r = self
            .console
            .as_ref()
            .and_then(|c| c.layout(dialog))
            .and_then(|d| d.control(id))
            .map(|c| c.abs)
            .unwrap_or(fallback);
        self.hud_rect(dialog, r)
    }

    /// Command-card dialog of the console race.
    ///
    /// Retail's HD console places the Zerg buttons like the others: the
    /// `statbtnz` offsets (4,7 instead of 9,4) would put them 12 px left and
    /// 8 px low of 134905 (shift search), so every race uses the default.
    pub(super) fn card_dialog(&self) -> &'static str {
        "statbtndefault"
    }

    pub(super) fn draw_hud(&mut self, fb: &mut Framebuffer) {
        let (scale, ox, oy) = self.ui_metrics(fb);
        let (cx, cy, cw, ch) = Self::content_rect(fb);
        let prof = std::env::var_os("GC_PROFILE").is_some();
        let mut marks = vec![("start", Instant::now())];
        // The console plates are static art: composed once per window size
        // (`LayerCache`) instead of resampled every frame (9.9 ms at 1440p).
        let key = (fb.width, fb.height, cx, cw, ch);
        if self.console.is_some() && self.plates_layer.as_ref().map(|p| p.0) != Some(key) {
            let (w, h) = (fb.width as usize, fb.height as usize);
            let mut over_black = Framebuffer::new(w, h);
            let mut over_white = Framebuffer::new(w, h);
            over_white.clear(Rgb::new(255, 255, 255));
            if let Some(console) = &self.console {
                let standard = self.game_options.minimap_position == 1;
                console.draw_plates_in(&mut over_black, cy, ch, cx, cw, standard);
                console.draw_plates_in(&mut over_white, cy, ch, cx, cw, standard);
            }
            self.plates_layer = Some((key, LayerCache::new(&over_black, &over_white)));
        }
        if let Some((_, layer)) = &self.plates_layer {
            layer.apply(fb);
        }
        marks.push(("plates", Instant::now()));
        self.draw_hud_resources(fb);
        self.draw_hud_optional_stats(fb);
        marks.push(("resources", Instant::now()));
        self.draw_hud_minimap(fb);
        marks.push(("minimap", Instant::now()));
        self.draw_hud_selection(fb);
        marks.push(("selection", Instant::now()));
        self.draw_hud_minimap_buttons(fb);
        self.draw_hud_upgrade(fb);
        marks.push(("buttons", Instant::now()));
        self.draw_hud_portrait(fb);
        marks.push(("portrait", Instant::now()));
        self.draw_hud_menu_label(fb);
        self.draw_hud_card(fb);
        marks.push(("card", Instant::now()));
        self.draw_hud_tooltip(fb);
        if prof {
            let parts: Vec<String> = marks
                .windows(2)
                .map(|w| format!("{} {:.1}", w[1].0, (w[1].1 - w[0].1).as_secs_f64() * 1000.0))
                .collect();
            eprintln!("hud: {}", parts.join(" | "));
        }
        // Trigger text (Transmission / Display Text) above the console,
        // newest at the bottom.
        let now_ms = self.trigger_text_epoch.elapsed().as_millis() as u64;
        let lines: Vec<String> = self
            .trigger_text
            .visible(now_ms)
            .into_iter()
            .map(str::to_string)
            .collect();
        // Retail (Wasteland recording, 1440p): `blizzard-regular` at 30.7
        // px/em in lavender, pen x 77, the newest line's baseline at 845
        // content px and older lines 48 px above it ("RAYNOR / Howdy, boys.
        // ... / of these parts.", "NEW MISSION OBJECTIVE: / - Raynor must
        // survive.").
        let _ = (cx, ch);
        // The maps pad lines with leading spaces (1998 layout); Remastered
        // draws every line flush at x 77 (recording: "RAYNOR" and its body
        // both start at 79).
        for (i, line) in lines.iter().rev().enumerate() {
            let line = line.trim_start();
            self.glue_text_face(
                fb,
                TRIGGER_TEXT_X,
                TRIGGER_TEXT_BOTTOM - TRIGGER_TEXT_PITCH * i as f32,
                line,
                TRIGGER_TEXT_PX,
                TRIGGER_TEXT_COLOUR,
                None,
                GlueFace::Hud,
            );
        }
        self.draw_trigger_leaderboard(fb);
        self.draw_countdown(fb);
        // Retail error line, centred over the console (recording: ink
        // 962..1595 x 894..921 content px, near white).
        if let Some((text, at)) = self.game_error.clone() {
            if at.elapsed() < GAME_ERROR_LIFE {
                let w = self.glue_measure(&text, TRIGGER_TEXT_PX, GlueFace::Hud);
                self.glue_text_face(
                    fb,
                    1280.0 - w / 2.0,
                    GAME_ERROR_BASE,
                    &text,
                    TRIGGER_TEXT_PX,
                    GAME_ERROR_COLOUR,
                    None,
                    GlueFace::Hud,
                );
            } else {
                self.game_error = None;
            }
        }
        // Status line in the game view.
        if Instant::now() < self.status_until && !self.status.is_empty() {
            fb.text_centered(
                self.view.0 / 2,
                self.view.1 - 28,
                &self.status,
                Rgb::new(255, 240, 140),
                1,
            );
        }
        if self.attack_mode {
            fb.text_centered(
                self.view.0 / 2,
                self.view.1 - 48,
                "ATTACK-MOVE: right click",
                Rgb::new(255, 120, 120),
                1,
            );
        }
        if self.patrol_mode {
            fb.text_centered(
                self.view.0 / 2,
                self.view.1 - 48,
                "PATROL: right click",
                Rgb::new(140, 200, 255),
                1,
            );
        }
        self.draw_net_chat(fb);
        self.draw_pad_radials(fb);

        if self.show_leaderboard {
            self.draw_leaderboard(fb, scale, ox, oy);
        }
        if self.show_stats {
            self.draw_mission_stats(fb, scale, ox, oy);
        }
    }

    /// Six newest chat lines and, while typing, the input line.  LAN matches
    /// use it for peer chat; single player uses the same prompt for the
    /// retail cheat codes.
    pub(super) fn draw_net_chat(&mut self, fb: &mut Framebuffer) {
        if self.net.is_none() && !self.chat_active && self.chat_log.is_empty() {
            return;
        }
        let lines: Vec<String> = self.chat_log.iter().rev().take(6).rev().cloned().collect();
        for (i, line) in lines.iter().enumerate() {
            let y = CHAT_TOP + CHAT_PITCH * i as f32;
            self.glue_text_face(
                fb,
                CHAT_X + 1.5,
                y + 1.5,
                line,
                CHAT_PX,
                Rgb::new(0, 0, 0),
                None,
                GlueFace::Hud,
            );
            self.glue_text_face(
                fb,
                CHAT_X,
                y,
                line,
                CHAT_PX,
                CHAT_COLOUR,
                None,
                GlueFace::Hud,
            );
        }
        if self.chat_active {
            let text = format!("Say: {}_", self.chat_input);
            let w = self.glue_measure(&text, CHAT_PX, GlueFace::Hud);
            for (d, colour) in [(1.5, Rgb::new(0, 0, 0)), (0.0, Rgb::new(255, 255, 255))] {
                self.glue_text_face(
                    fb,
                    1280.0 - w / 2.0 + d,
                    900.0 + d,
                    &text,
                    CHAT_PX,
                    colour,
                    None,
                    GlueFace::Hud,
                );
            }
        }
    }

    /// `statres`: minerals, gas and supply, each an HD icon (`icons`
    /// frames 0 / gas / supply by race) and its number in BlizzardGlobal.
    pub(super) fn draw_hud_resources(&mut self, fb: &mut Framebuffer) {
        let viewer = self.view_player();
        let p = &self.world.players[viewer];
        let (minerals, gas) = (p.minerals, p.gas);
        let used = self.world.supply_used(viewer as u8) / 2;
        let cap = self.world.supply_cap(viewer as u8) / 2;
        let (gas_icon, supply_icon) = match self.console_race {
            Race::Zerg => (1, 4),
            Race::Protoss => (3, 6),
            _ => (2, 5),
        };
        let fields = [
            (3, 0usize, minerals.to_string()),
            (2, gas_icon, gas.to_string()),
            (1, supply_icon, format!("{used}/{cap}")),
        ];
        let (_, _, _, ch) = self.content_box;
        let k = ch as f32 / 1440.0;
        for (id, icon, text) in fields {
            let r = self.hud_control("statres", id, UiRect::new(572 - (id - 1) * 68, 0, 66, 20));
            if let Some(img) = self
                .console
                .as_ref()
                .and_then(|c| c.icons4.as_ref())
                .and_then(|g| g.frame(icon))
            {
                let w = (img.width as f32 * 0.75 * k).round() as i32;
                let h = (img.height as f32 * 0.75 * k).round() as i32;
                img.draw_scaled(fb, r.x, r.y + (r.h - h) / 2, w, h);
            }
            let x = (r.x - self.content_box.0) as f32 / k + HUD_RES_TEXT_DX;
            self.glue_text_face(
                fb,
                x,
                (r.y as f32) / k + HUD_RES_BASE,
                &text,
                HUD_NAME_PX,
                HUD_RES_COLOR,
                None,
                GlueFace::Hud,
            );
        }
    }

    /// Optional Game-tab HUD stats: the shipped `console_time.dds` and
    /// `console_apm.dds` top plates, with the mission clock and the APM
    /// count in their black wells.  Retail draws these above the game view
    /// at the top right; the exact anchor is INFERENCE from the plate art.
    pub(super) fn draw_hud_optional_stats(&mut self, fb: &mut Framebuffer) {
        if !self.game_options.game_timer && !self.game_options.apm_display {
            return;
        }
        let Some(console) = self.console.as_ref() else {
            return;
        };
        let clock = console.clock.clone();
        let apm_art = console.apm.clone();
        let (_, _, _, ch) = Self::content_rect(fb);
        let k = ch as f32 / 1440.0;
        // The console pieces use art * console_scale / 4 (0.75x at 1440p).
        let s = 0.75 * k;
        // Anchor to the right edge of the shipped `statres` resource strip.
        let resource = self.hud_control("statres", 1, UiRect::new(572, 0, 66, 20));
        let gap = (6.0 * k).round() as i32;
        let mut right = resource.x + resource.w;
        let y = resource.y + resource.h + (6.0 * k).round() as i32;
        if self.game_options.game_timer
            && let Some(img) = clock
        {
            let w = (img.width as f32 * s).round() as i32;
            let h = (img.height as f32 * s).round() as i32;
            let x = right - w;
            img.draw_scaled(fb, x, y, w, h);
            let text = format_time(self.elapsed_ticks());
            self.draw_hud_plate_text(fb, (x, y, w, h), &text, Rgb::new(198, 218, 255));
            right = x - gap;
        }
        if self.game_options.apm_display
            && let Some(img) = apm_art
        {
            let w = (img.width as f32 * s).round() as i32;
            let h = (img.height as f32 * s).round() as i32;
            let x = right - w;
            img.draw_scaled(fb, x, y, w, h);
            let apm = self.current_apm();
            let alert = self.game_options.apm_alert
                && self.elapsed_ticks() >= galaxycraft::sim::TICKS_PER_SECOND as u64 * 60
                && apm < self.game_options.apm_threshold.round() as i64;
            if alert && !self.apm_alerted {
                if self.game_options.apm_play_sound {
                    // No dedicated retail APM cue is shipped as a loose
                    // asset; the short UI move cue is the audible warning.
                    self.play("ui_move");
                }
                self.apm_alerted = true;
            } else if !alert {
                self.apm_alerted = false;
            }
            let colour = if alert && self.game_options.apm_color_text {
                Rgb::new(255, 92, 70)
            } else {
                Rgb::new(198, 218, 255)
            };
            let text = format!("APM {apm}");
            self.draw_hud_plate_text(fb, (x, y, w, h), &text, colour);
        }
    }

    /// Centre HUD text in one of the optional plate's black wells.
    pub(super) fn draw_hud_plate_text(
        &mut self,
        fb: &mut Framebuffer,
        (x, y, w, h): (i32, i32, i32, i32),
        text: &str,
        colour: Rgb,
    ) {
        let (cx, _cy, _cw, ch) = Self::content_rect(fb);
        let k = ch as f32 / 1440.0;
        let px = HUD_SMALL_PX;
        let ink = self.glue_measure(text, px, GlueFace::Hud);
        let base = (y as f32 + h as f32 * 0.66) / k;
        let pen = (x as f32 - cx as f32) / k + (w as f32 / k - ink) / 2.0;
        self.glue_text_face(fb, pen, base, text, px, colour, None, GlueFace::Hud);
    }

    /// `minimap` control 1 (classic 6,348 128x128, left-anchored): units and
    /// the camera box over the console art's own black well.
    pub(super) fn draw_hud_minimap(&mut self, fb: &mut Framebuffer) {
        let (rx, ry, rw, rh) = self.minimap_rect();
        let mut units = self
            .world
            .minimap_units(self.local_player as u8, self.fog_on());
        units.extend(self.remembered_minimap_dots());
        // The box frames the playfield above the console: 368 game px tall
        // (135051: inner box 1371..1406 at 3 px per tile).
        let cam = (
            self.camera.0,
            self.camera.1,
            self.camera.0 + self.view.0,
            self.camera.1 + MINIMAP_VIEW_H,
        );
        // HD terrain minimap (134905): the terrain renderer at minimap scale
        // (3 px per tile at 1440p, creep included) under the smooth fog, so
        // the visible area fades out like the playfield's.
        let mode = if !self.minimap_terrain {
            galaxycraft::render::MinimapTerrain::Hidden
        } else if self.prepaint_minimap(fb, (rx, ry, rw, rh)) {
            galaxycraft::render::MinimapTerrain::Prepainted
        } else {
            galaxycraft::render::MinimapTerrain::Tiles
        };
        self.tileset.draw_minimap_with_terrain(
            fb,
            (rx, ry, rw, rh),
            &units,
            cam,
            self.world.vision.as_ref(),
            self.local_player as u8,
            mode,
        );
    }

    /// `statdata` for one selected unit: HD wireframe (LTEXT 1), name
    /// (CTEXT -5), shields/hit points (CTEXT -7) and, for town halls, the
    /// four supply rows (CTEXT -10..-13).
    pub(super) fn draw_hud_selection(&mut self, fb: &mut Framebuffer) {
        let Some(u) = self.first_selected().cloned() else {
            return;
        };
        if self.selected.len() > 1 {
            self.draw_hud_group(fb);
            return;
        }
        if self.selected.len() != 1 {
            return;
        }
        let k = self.content_box.3 as f32 / 1440.0;
        let cx0 = self.content_box.0;
        let to_x = |x: i32| (x - cx0) as f32 / k;
        let ty = self.data.unit(u.type_id).clone();
        let wire = self.hud_control("statdata", 1, UiRect::new(168, 388, 64, 64));
        if let Some(console) = &self.console {
            // HD wireframe: 256 px art at 0.75, centred on the control
            // (134905 / 135051: 192 px square centred at 918, 1261).
            let side = (wire.h as f32 * 1.0).round() as i32;
            let rect = (
                wire.x + (wire.w - side) / 2,
                wire.y + (wire.h - side) / 2,
                side,
                side,
            );
            // Damaged sections turn yellow, then red.
            let states =
                galaxycraft::ui_art::wireframe_states(u.hp, ty.hit_points_raw as i32, u.id);
            if !console.draw_wireframe_hd(fb, u.type_id, rect, states, ty.has_shields) {
                console.draw_wireframe(
                    fb,
                    u.type_id,
                    (wire.x, wire.y, wire.w, wire.h),
                    None,
                    states,
                );
            }
        }
        let name = self.data.unit_name(u.type_id).to_string();
        let nr = self.hud_control("statdata", -5, UiRect::new(240, 388, 152, 14));
        let w = self.glue_measure(&name, HUD_NAME_PX, GlueFace::Hud);
        self.glue_text_face(
            fb,
            to_x(nr.x) + (nr.w as f32 / k - w) / 2.0,
            nr.y as f32 / k + HUD_NAME_BASE,
            &name,
            HUD_NAME_PX,
            HUD_TEXT_COLOR,
            None,
            GlueFace::Hud,
        );
        self.draw_hud_status(fb, &u);
        self.draw_hud_cargo(fb, &u);
        // Hit points (and shields first for Protoss) under the wireframe.
        // Translation batch 30 (`0x005b2570`): the status-panel current HP
        // is the cached whole-point value with the retail zero clamp.
        let hp = (u.a30_cached_whole_hp() as i32).max(0);
        let max_hp = ty.hit_points_raw as i32 / 256;
        let hr = self.hud_control("statdata", -7, UiRect::new(148, 452, 100, 12));
        let mut parts: Vec<(String, Rgb)> = Vec::new();
        if ty.has_shields {
            parts.push((
                format!("{}/{} ", u.shields / 256, ty.shield_points),
                HUD_TEXT_COLOR,
            ));
        }
        // Retail colours the hit points by what is left: green, then
        // yellow under two thirds, red under one third.
        let hp_colour = if max_hp > 0 && hp * 3 < max_hp {
            Rgb::new(220, 40, 30)
        } else if max_hp > 0 && hp * 3 < max_hp * 2 {
            Rgb::new(232, 220, 40)
        } else {
            HUD_HP_COLOR
        };
        parts.push((format!("{hp}/{max_hp}"), hp_colour));
        let total: f32 = parts
            .iter()
            .map(|(t, _)| self.glue_measure(t, HUD_SMALL_PX, GlueFace::Hud))
            .sum();
        let mut x = to_x(hr.x) + (hr.w as f32 / k - total) / 2.0;
        for (t, c) in parts {
            x = self.glue_text_face(
                fb,
                x,
                hr.y as f32 / k + HUD_SMALL_BASE,
                &t,
                HUD_SMALL_PX,
                c,
                None,
                GlueFace::Hud,
            );
        }
        // Town halls list the player's supply, unless the production or
        // construction block is shown there (Backwater Station: a training
        // Command Center shows only "Building" and its queue).
        if u.completed
            && !matches!(
                u.order,
                Order::Train { .. } | Order::Research { .. } | Order::Upgrade { .. }
            )
            && matches!(
                u.type_id,
                galaxycraft::ids::unit::COMMAND_CENTER
                    | galaxycraft::ids::unit::NEXUS
                    | galaxycraft::ids::unit::HATCHERY
                    | galaxycraft::ids::unit::LAIR
                    | galaxycraft::ids::unit::HIVE
            )
        {
            let used = self.world.supply_used(self.local_player as u8) / 2;
            let cap = self.world.supply_cap(self.local_player as u8) / 2;
            // "Provided" is this building's own supply (134905: a Hatchery
            // shows 1 while the player total is 9).  Labels are the
            // `stat_txt.tbl` rows of the race (Terran "Supplies Used:", as
            // on the retail Wasteland recording; the old code said
            // "Control" for Terran).
            let provided = ty.supply_provided as i32 / 2;
            let labels = supply_labels(&self.data.strings, self.console_race);
            let rows = [
                format!("{} {used}", labels[0]),
                format!("{} {provided}", labels[1]),
                format!("{} {cap}", labels[2]),
                format!("{} 200", labels[3]),
            ];
            for (i, text) in rows.iter().enumerate() {
                let r = self.hud_control(
                    "statdata",
                    -10 - i as i32,
                    UiRect::new(281, 418 + 14 * i as i32, 111, 14),
                );
                let w = self.glue_measure(text, HUD_SMALL_PX, GlueFace::Hud);
                self.glue_text_face(
                    fb,
                    to_x(r.x) + (r.w as f32 / k - w) / 2.0,
                    r.y as f32 / k + HUD_SMALL_BASE,
                    text,
                    HUD_SMALL_PX,
                    HUD_TEXT_COLOR,
                    None,
                    GlueFace::Hud,
                );
            }
        }
    }

    /// HUD element under the pointer that carries a retail tooltip.
    pub(super) fn hud_hover(&self) -> Option<HudHover> {
        if self.pad_wheel.is_open() {
            return None;
        }
        let (mx, my) = self.pointer?;
        let inside = |r: UiRect| mx >= r.x && mx < r.x + r.w && my >= r.y && my < r.y + r.h;
        if let Some(i) = self.command_slots().into_iter().position(inside) {
            return Some(HudHover::Card(i));
        }
        let protoss_selected = self
            .first_selected()
            .is_some_and(|u| self.data.unit(u.type_id).has_shields);
        if protoss_selected && self.selected.len() == 1 && inside(self.upgrade_button_rect()) {
            return Some(HudHover::Shields);
        }
        if inside(self.hud_control("minimap", 2, UiRect::new(3, 320, 64, 24))) {
            return Some(HudHover::MinimapTerrain);
        }
        None
    }

    /// `statdata` button 3 (classic 242,441 39x36): the shield/armour
    /// upgrade of a single selection.
    pub(super) fn upgrade_button_rect(&self) -> UiRect {
        // Control 3 (classic 242,441): measured at y 1323 = 441 * 3 on 135109.
        self.hud_control("statdata", 3, UiRect::new(242, 441, 39, 36))
    }

    /// Retail tooltip of the hovered HUD element: a near-black box with a
    /// 3.5 px blue rim, BlizzardGlobal 27.2 px/em text whose `\x03` spans
    /// are yellow, and a cost row for buttons that make a unit.  Box
    /// placement relative to the element as measured on 135109 / 135114 /
    /// 135121 (the captures hovered the element's middle).
    pub(super) fn draw_hud_tooltip(&mut self, fb: &mut Framebuffer) {
        let Some(hover) = self.hud_hover() else {
            return;
        };
        let k = self.content_box.3 as f32 / 1440.0;
        let cx0 = self.content_box.0;
        let to_c = |x: i32, y: i32| ((x - cx0) as f32 / k, y as f32 / k);
        let mut lines: Vec<String> = Vec::new();
        let mut cost: Option<(i32, i32, i32)> = None;
        // (anchor point in content px, true = box bottom-left, false = top-left)
        let anchor: ((f32, f32), bool) = match hover {
            HudHover::Card(i) => {
                let Some((_, card)) = self.current_card() else {
                    return;
                };
                let Some((b, st)) = card[i] else {
                    return;
                };
                let id = if st == galaxycraft::buttons::ButtonState::Disabled
                    && b.disabled_string != 0
                {
                    b.disabled_string
                } else {
                    b.enabled_string
                };
                let raw = self
                    .data
                    .strings
                    .get(id.saturating_sub(1))
                    .unwrap_or("")
                    .to_string();
                lines.push(button_tooltip_text(&raw));
                if galaxycraft::buttons::Cond::of(b.cond) == galaxycraft::buttons::Cond::MakeUnit {
                    let ty = self.data.unit(b.cond_var);
                    cost = Some((
                        ty.mineral_cost as i32,
                        ty.gas_cost as i32,
                        (ty.supply_required as i32 + 1) / 2,
                    ));
                }
                let r = self.command_slots()[i];
                let (x, y) = to_c(r.x, r.y);
                ((x - 13.0, y + 5.0), true)
            }
            HudHover::Shields => {
                lines.push(self.data.upgrade_name(PLASMA_SHIELDS).to_string());
                let level = self.world.players[self.view_player()].upgrade_level(PLASMA_SHIELDS);
                lines.push(format!(
                    "{} {level}",
                    self.data.strings.get(778).unwrap_or("Shields:")
                ));
                let r = self.upgrade_button_rect();
                let (x, y) = to_c(r.x, r.y);
                // +96 from the hit rect (measured +94 before `statdata`
                // moved 2 px left; the retail text did not move).
                ((x + 96.0, y - 16.0), false)
            }
            HudHover::MinimapTerrain => {
                // The toggle offers the opposite of the current state:
                // 810 "Hide Terrain ..." while the terrain is shown (135121,
                // whose minimap shows the terrain), 809 "Show ..." otherwise.
                let id = minimap_toggle_string(self.minimap_terrain);
                lines.push(self.data.strings.get(id).unwrap_or("").to_string());
                let r = self.hud_control("minimap", 2, UiRect::new(3, 320, 64, 24));
                let (x, y) = to_c(r.x, r.y);
                ((x + 181.0, y - 8.0), false)
            }
        };
        let widths: Vec<f32> = lines
            .iter()
            .map(|l| self.glue_measure(&strip_codes(l), TOOLTIP_PX, GlueFace::Hud))
            .collect();
        let mut w = widths.iter().cloned().fold(0.0, f32::max);
        if let Some((m, g, sup)) = cost {
            w = w.max(self.tooltip_cost_width(m, g, sup));
        }
        let bw = (w + TOOLTIP_PAD_L + TOOLTIP_PAD_R).round();
        let single = lines.len() == 1 && cost.is_none();
        // A one-line box is 59 px tall and starts 2 px higher, with the text
        // where it would be (135121: rim 950..1008, ink 966..989).
        let bh = if single {
            TOOLTIP_SINGLE_H
        } else {
            TOOLTIP_LINE0_H
                + TOOLTIP_PITCH * (lines.len() as f32 - 1.0)
                + if cost.is_some() { TOOLTIP_COST_H } else { 0.0 }
        };
        let ((ax, ay), bottom) = anchor;
        let (bx, by) = if bottom { (ax, ay - bh) } else { (ax, ay) };
        let (by, text_dy) = if single { (by - 2.0, 2.0) } else { (by, 0.0) };
        let px = |x: f32| cx0 + (x * k).round() as i32;
        let py = |y: f32| (y * k).round() as i32;
        draw_rounded_box(
            fb,
            (
                px(bx),
                py(by),
                (bw * k).round() as i32,
                (bh * k).round() as i32,
            ),
            TOOLTIP_RADIUS * k,
            TOOLTIP_RIM * k,
            TOOLTIP_FILL,
            TOOLTIP_RIM_COLOR,
        );
        for (i, line) in lines.iter().enumerate() {
            self.glue_coded_text(
                fb,
                bx + TOOLTIP_PAD_L,
                by + text_dy + TOOLTIP_BASE + i as f32 * TOOLTIP_PITCH,
                line,
            );
        }
        if let Some((m, g, sup)) = cost {
            self.draw_tooltip_cost(
                fb,
                bx + TOOLTIP_PAD_L,
                by + TOOLTIP_LINE0_H + TOOLTIP_COST_DY,
                m,
                g,
                sup,
            );
        }
    }

    /// Text with `\x01` (default) / `\x03` (yellow) colour codes.
    pub(super) fn glue_coded_text(&mut self, fb: &mut Framebuffer, x: f32, base: f32, text: &str) {
        let mut pen = x;
        let mut colour = HUD_TEXT_COLOR;
        let mut run = String::new();
        for ch in text.chars().chain(std::iter::once('\u{0}')) {
            match ch {
                '\u{1}' | '\u{3}' | '\u{0}' => {
                    if !run.is_empty() {
                        pen = self.glue_text_face(
                            fb,
                            pen,
                            base,
                            &run,
                            TOOLTIP_PX,
                            colour,
                            None,
                            GlueFace::Hud,
                        );
                        run.clear();
                    }
                    colour = if ch == '\u{3}' {
                        TOOLTIP_HOTKEY
                    } else {
                        HUD_TEXT_COLOR
                    };
                }
                c if (c as u32) < 0x20 => {}
                c => run.push(c),
            }
        }
    }

    /// Width (content px) of a tooltip cost row.
    pub(super) fn tooltip_cost_width(&self, minerals: i32, gas: i32, supply: i32) -> f32 {
        let mut w = 0.0;
        for (v, show) in [(minerals, true), (gas, gas > 0), (supply, supply > 0)] {
            if show {
                w += TOOLTIP_ICON
                    + 8.0
                    + self.glue_measure(&v.to_string(), TOOLTIP_PX, GlueFace::Hud)
                    + TOOLTIP_COST_GAP;
            }
        }
        w
    }

    /// Cost row: each value after its resource icon in a rimmed square.
    pub(super) fn draw_tooltip_cost(
        &mut self,
        fb: &mut Framebuffer,
        x: f32,
        y: f32,
        minerals: i32,
        gas: i32,
        supply: i32,
    ) {
        let k = self.content_box.3 as f32 / 1440.0;
        let cx0 = self.content_box.0;
        let (gas_icon, supply_icon) = match self.console_race {
            Race::Zerg => (1, 4),
            Race::Protoss => (3, 6),
            _ => (2, 5),
        };
        let mut pen = x;
        for (v, icon, show) in [
            (minerals, 0usize, true),
            (gas, gas_icon, gas > 0),
            (supply, supply_icon, supply > 0),
        ] {
            if !show {
                continue;
            }
            let rect = (
                cx0 + (pen * k).round() as i32,
                (y * k).round() as i32,
                (TOOLTIP_ICON * k).round() as i32,
                (TOOLTIP_ICON * k).round() as i32,
            );
            draw_rounded_box(
                fb,
                rect,
                3.0 * k,
                2.0 * k,
                Rgb::new(10, 20, 50),
                TOOLTIP_RIM_COLOR,
            );
            if let Some(img) = self
                .console
                .as_ref()
                .and_then(|c| c.icons4.as_ref())
                .and_then(|g| g.frame(icon))
            {
                img.draw_scaled(fb, rect.0 + 2, rect.1 + 2, rect.2 - 4, rect.3 - 4);
            }
            pen += TOOLTIP_ICON + 8.0;
            pen = self.glue_text_face(
                fb,
                pen,
                y + TOOLTIP_ICON - 10.0,
                &v.to_string(),
                TOOLTIP_PX,
                HUD_TEXT_COLOR,
                None,
                GlueFace::Hud,
            ) + TOOLTIP_COST_GAP;
        }
    }

    /// The two buttons above the radar (`minimap` controls 2 and 3) from the
    /// HD `dlg` sheet; the terrain toggle shows its highlighted frame while
    /// hovered.
    pub(super) fn draw_hud_minimap_buttons(&mut self, fb: &mut Framebuffer) {
        let k = self.content_box.3 as f32 / 1440.0;
        let mut rect = self.hud_control("minimap", 2, UiRect::new(3, 320, 64, 24));
        rect.x += (MINIMAP_BUTTON_SHIFT.0 * k).round() as i32;
        rect.y += (MINIMAP_BUTTON_SHIFT.1 * k).round() as i32;
        // Only the terrain toggle is drawn, always in its normal frame 4
        // (retail keeps it while hovered, 135121).  The second button
        // (`minimap` control 3) is not drawn in these melee games: the
        // console's own socket shows (hazard stripes on 134905 / 135020,
        // the empty blue slot on 135051).
        if let Some(img) = self
            .console
            .as_ref()
            .and_then(|c| c.dlg4.as_ref())
            .and_then(|g| g.frame(4))
        {
            let w = (img.width as f32 * 0.75 * k).round() as i32;
            let h = (img.height as f32 * 0.75 * k).round() as i32;
            img.draw_scaled(
                fb,
                rect.x + (rect.w - w) / 2,
                rect.y + (rect.h - h) / 2,
                w,
                h,
            );
        }
    }

    /// Multi-selection panel: `statdata` controls 33-44, a 6 x 2 grid filled
    /// column by column (Wasteland recording 84-116 s: five Marines fill
    /// three on top, two below).  Each slot is a `cmdbtns` box (frame 13,
    /// dark blue; 19, white, for heroes: Jim Raynor at 100 s) holding the
    /// unit's wireframe.  INFERENCE: the HD wireframe scaled into the box
    /// stands in for `grpwire`; falsifier: a pixel diff against the slots.
    pub(super) fn draw_hud_group(&mut self, fb: &mut Framebuffer) {
        let units: Vec<SimUnit> = self
            .selected
            .iter()
            .filter_map(|id| self.world.unit(*id).cloned())
            .take(12)
            .collect();
        let k = self.content_box.3 as f32 / 1440.0;
        for (n, u) in units.iter().enumerate() {
            let r = self.group_slot_rect(n);
            let ty = self.data.unit(u.type_id).clone();
            let Some(console) = &self.console else {
                return;
            };
            let box_frame = if ty.flags & UNIT_FLAG_HERO != 0 {
                GROUP_BOX_HERO
            } else {
                GROUP_BOX
            };
            if let Some(img) = console
                .command_buttons_hd
                .as_ref()
                .and_then(|g| g.frame(box_frame))
            {
                let w = (img.width as f32 * 0.75 * k).round() as i32;
                let h = (img.height as f32 * 0.75 * k).round() as i32;
                img.draw_scaled(fb, r.x + (r.w - w) / 2, r.y + (r.h - h) / 2, w, h);
            }
            let side = (r.w.min(r.h) as f32 * 0.86).round() as i32;
            let rect = (r.x + (r.w - side) / 2, r.y + (r.h - side) / 2, side, side);
            let states =
                galaxycraft::ui_art::wireframe_states(u.hp, ty.hit_points_raw as i32, u.id);
            if !console.draw_wireframe_hd(fb, u.type_id, rect, states, ty.has_shields) {
                console.draw_wireframe(fb, u.type_id, rect, None, states);
            }
        }
    }

    /// Loaded units of a Bunker / transport (Desperate Alliance recording
    /// 272 s: a Bunker with four Marines shows them in a 2 x 2 block of
    /// `cmdbtns` boxes at `statdata` controls 27-30, the middle two columns
    /// of the eight cargo slots 25-32).  Capacity 4 uses 27-30; larger
    /// transports pack 1 x 1, 1 x 2 and 2 x 2 cargo into 25-32 / 21-24 /
    /// 18-20 (INFERENCE: column-major first fit, as the group grid).
    pub(super) fn draw_hud_cargo(&mut self, fb: &mut Framebuffer, u: &SimUnit) {
        let ty = self.data.unit(u.type_id).clone();
        if ty.space_provided == 0 || u.cargo.is_empty() {
            return;
        }
        let k = self.content_box.3 as f32 / 1440.0;
        let cargo: Vec<SimUnit> = u
            .cargo
            .iter()
            .filter_map(|id| self.world.unit(*id).cloned())
            .collect();
        let sizes: Vec<u8> = cargo
            .iter()
            .map(|c| self.data.unit(c.type_id).space_required)
            .collect();
        let layout = cargo_slots(&sizes, ty.space_provided);
        for (n, c) in cargo.iter().enumerate() {
            let Some(slot) = layout.get(n) else {
                break;
            };
            let r = self.cargo_slot_rect(slot.control);
            let cty = self.data.unit(c.type_id).clone();
            let Some(console) = &self.console else {
                return;
            };
            if let Some(img) = console
                .command_buttons_hd
                .as_ref()
                .and_then(|g| g.frame(GROUP_BOX))
            {
                let w = (img.width as f32 * 0.75 * k).round() as i32;
                let h = (img.height as f32 * 0.75 * k).round() as i32;
                img.draw_scaled(fb, r.x + (r.w - w) / 2, r.y + (r.h - h) / 2, w, h);
            }
            let side = (r.w.min(r.h) as f32 * 0.86).round() as i32;
            let rect = (r.x + (r.w - side) / 2, r.y + (r.h - side) / 2, side, side);
            let states =
                galaxycraft::ui_art::wireframe_states(c.hp, cty.hit_points_raw as i32, c.id);
            if !console.draw_wireframe_hd(fb, c.type_id, rect, states, cty.has_shields) {
                console.draw_wireframe(fb, c.type_id, rect, None, states);
            }
        }
    }

    /// Own transport and loaded unit under a HUD point (single selection).
    pub(super) fn cargo_slot_at(&self, mx: i32, my: i32) -> Option<(u32, u32)> {
        if self.selected.len() != 1 {
            return None;
        }
        let t = self.world.unit(self.selected[0])?;
        if t.owner as usize != self.local_player {
            return None;
        }
        let ty = self.data.unit(t.type_id);
        let sizes: Vec<u8> = t
            .cargo
            .iter()
            .filter_map(|id| self.world.unit(*id))
            .map(|u| self.data.unit(u.type_id).space_required)
            .collect();
        let layout = cargo_slots(&sizes, ty.space_provided);
        t.cargo
            .iter()
            .enumerate()
            .find(|(n, _)| {
                layout
                    .get(*n)
                    .is_some_and(|slot| self.cargo_slot_rect(slot.control).contains(mx, my))
            })
            .map(|(_, &c)| (t.id, c))
    }

    /// Screen rect of one `statdata` cargo control (see `cargo_slots`).
    pub(super) fn cargo_slot_rect(&self, control: i32) -> UiRect {
        self.hud_control("statdata", control, UiRect::new(0, 0, 0, 0))
    }

    /// Screen rect of multi-selection slot `n` (0..12): `statdata` control
    /// 33 + n (column-major).
    pub(super) fn group_slot_rect(&self, n: usize) -> UiRect {
        let col = (n / 2) as i32;
        let row = (n % 2) as i32;
        self.hud_control(
            "statdata",
            33 + n as i32,
            UiRect::new(168 + 36 * col, 396 + 37 * row, 33, 34),
        )
    }

    /// The selection panel's state block (shipped `statdata.ui.json`
    /// controls; look from the retail recordings):
    /// - a building under construction: "Under Construction" (-30) over the
    ///   `pbrempt` / `pbrfull` bar (13);
    /// - a building training: "Building" (-15), the current item (2, `cmdbtns`
    ///   frame 2, small "1"), the bar (7) and queue slots 2-5 (3-6): the queued
    ///   unit with its yellow number, else the big digit (frames 7-10), linked
    ///   by `cmdbtns` frame 4 (Backwater Station 126 s);
    /// - research / upgrade: "Researching" (-35) / "Upgrading" (-40), item 15 /
    ///   17, bar 14 / 16;
    /// - a unit: its rank (-20, `stat_txt` 1302 + rank) and "Kills: N" (-21),
    ///   and armour / weapon upgrade boxes in 3-6 (Boot Camp: SCV "Private",
    ///   "Kills: 0", two boxes).
    pub(super) fn draw_hud_status(&mut self, fb: &mut Framebuffer, u: &SimUnit) {
        let ty = self.data.unit(u.type_id).clone();
        let k = self.content_box.3 as f32 / 1440.0;
        let cx0 = self.content_box.0;
        let control = |app: &App, id: i32| app.hud_control("statdata", id, UiRect::new(0, 0, 0, 0));
        let text = |app: &mut App, fb: &mut Framebuffer, r: UiRect, t: &str, small: bool| {
            let (px, base) = if small {
                (HUD_SMALL_PX, HUD_SMALL_BASE)
            } else {
                (HUD_NAME_PX, HUD_NAME_BASE)
            };
            let w = app.glue_measure(t, px, GlueFace::Hud);
            app.glue_text_face(
                fb,
                (r.x - cx0) as f32 / k + (r.w as f32 / k - w) / 2.0,
                r.y as f32 / k + base,
                t,
                px,
                HUD_TEXT_COLOR,
                None,
                GlueFace::Hud,
            );
        };
        let string = |app: &App, id: u16| {
            app.data
                .strings
                .get(id - 1)
                .unwrap_or("")
                .trim_matches(|c: char| c.is_control())
                .to_string()
        };
        let bar = |app: &App, fb: &mut Framebuffer, r: UiRect, frac: f32| {
            let Some(console) = &app.console else {
                return;
            };
            if let Some(e) = &console.progress_empty {
                e.draw_scaled(fb, r.x, r.y, r.w, r.h);
            }
            if let Some(f) = &console.progress_full {
                let frac = frac.clamp(0.0, 1.0);
                let sw = (f.width as f32 * frac).round() as i32;
                let dw = (r.w as f32 * frac).round() as i32;
                if sw > 0 && dw > 0 {
                    f.draw_sub(fb, (0, 0, sw, f.height as i32), (r.x, r.y, dw, r.h));
                }
            }
        };
        let frame = |app: &App, fb: &mut Framebuffer, r: UiRect, n: usize| {
            if let Some(img) = app
                .console
                .as_ref()
                .and_then(|c| c.command_buttons_hd.as_ref())
                .and_then(|g| g.frame(n))
            {
                let w = (img.width as f32 * 0.75 * k).round() as i32;
                let h = (img.height as f32 * 0.75 * k).round() as i32;
                img.draw_scaled(fb, r.x + (r.w - w) / 2, r.y + (r.h - h) / 2, w, h);
            }
        };
        let icon = |app: &App, fb: &mut Framebuffer, r: UiRect, n: u16| {
            if let Some(img) = app
                .console
                .as_ref()
                .and_then(|c| c.cmdicons4.as_ref())
                .and_then(|g| g.frame(n as usize))
            {
                let w = (img.width as f32 * 0.75 * k).round() as i32;
                let h = (img.height as f32 * 0.75 * k).round() as i32;
                draw_tinted(
                    fb,
                    img,
                    (r.x + (r.w - w) / 2, r.y + (r.h - h) / 2, w, h),
                    HUD_GLYPH_ENABLED,
                );
            }
        };
        let frac = if u.build_total > 0 {
            1.0 - u.build_remaining as f32 / u.build_total as f32
        } else {
            0.0
        };
        if ty.is_building() && !u.completed {
            text(
                self,
                fb,
                control(self, -30),
                &string(self, STAT_UNDER_CONSTRUCTION),
                false,
            );
            bar(self, fb, control(self, 13), frac);
            return;
        }
        match u.order {
            Order::Train { unit_type } => {
                text(
                    self,
                    fb,
                    control(self, -15),
                    &string(self, STAT_BUILDING),
                    false,
                );
                let cur = control(self, 2);
                frame(self, fb, cur, 2);
                icon(self, fb, cur, unit_type);
                // FACT (Desperate Alliance recording 158 s): a small "1" at
                // the box's bottom right (right edge 15 px, bottom 15 px
                // inside the visible box at 1440p) and the `cmdbtns` frame 11
                // arrow centred under it, 7 px below the visible edge (the
                // frame-2 image has ~4 px of transparent margin, so these are
                // 21 / 0 px from its bounds).
                let side = (144.0 * 0.75 * k).round() as i32;
                let (bx1, by1) = (cur.x + (cur.w + side) / 2, cur.y + (cur.h + side) / 2);
                let one_w = self.glue_measure("1", HUD_SMALL_PX, GlueFace::Hud);
                self.glue_text_face(
                    fb,
                    (bx1 - cx0) as f32 / k - 15.0 - one_w,
                    by1 as f32 / k - 21.0,
                    "1",
                    HUD_SMALL_PX,
                    HUD_TEXT_COLOR,
                    None,
                    GlueFace::Hud,
                );
                if let Some(img) = self
                    .console
                    .as_ref()
                    .and_then(|c| c.command_buttons_hd.as_ref())
                    .and_then(|g| g.frame(QUEUE_ARROW_FRAME))
                {
                    let w = (img.width as f32 * 0.75 * k).round() as i32;
                    let h = (img.height as f32 * 0.75 * k).round() as i32;
                    let top = by1;
                    img.draw_scaled(fb, cur.x + (cur.w - w) / 2, top, w, h);
                }
                bar(self, fb, control(self, 7), frac);
                let queued: Vec<u16> = u
                    .production
                    .iter()
                    .filter_map(|o| match o {
                        Order::Train { unit_type } => Some(*unit_type),
                        _ => None,
                    })
                    .collect();
                for slot in 0..4usize {
                    let r = control(self, 3 + slot as i32);
                    frame(self, fb, r, if slot < 3 { 4 } else { 2 });
                    match queued.get(slot) {
                        Some(&t) => icon(self, fb, r, t),
                        None => frame(self, fb, r, 7 + slot),
                    }
                }
                return;
            }
            Order::Research { tech_id } => {
                text(
                    self,
                    fb,
                    control(self, -35),
                    &string(self, STAT_RESEARCHING),
                    false,
                );
                let r = control(self, 15);
                frame(self, fb, r, 2);
                let n = self.data.tech(tech_id).map_or(0, |t| t.icon);
                icon(self, fb, r, n);
                bar(self, fb, control(self, 14), frac);
                return;
            }
            Order::Upgrade { upgrade_id, .. } => {
                text(
                    self,
                    fb,
                    control(self, -40),
                    &string(self, STAT_UPGRADING),
                    false,
                );
                let r = control(self, 17);
                frame(self, fb, r, 2);
                let n = self.data.upgrade(upgrade_id).map_or(0, |t| t.icon);
                icon(self, fb, r, n);
                bar(self, fb, control(self, 16), frac);
                return;
            }
            _ => {}
        }
        if ty.is_building() {
            return;
        }
        if ty.rank != 0 {
            let rank = string(self, 1302 + ty.rank as u16);
            text(self, fb, control(self, -20), &rank, false);
        }
        let kills = format!("{} {}", string(self, STAT_KILLS), u.kills);
        text(self, fb, control(self, -21), &kills, false);
        // Armour then weapon boxes (shields first for Protoss, drawn by
        // `draw_hud_upgrade` in box 3).
        let mut boxes: Vec<(u16, u8)> = Vec::new();
        if let Some(up) = self.data.upgrade(ty.armor_upgrade) {
            boxes.push((up.icon, ty.armor_upgrade));
        }
        for w in [
            self.data.ground_weapon_id(&ty),
            self.data.air_weapon_id(&ty),
        ] {
            if w != NO_WEAPON
                && let Some(wt) = self.data.weapon(w)
                && !boxes.iter().any(|(i, _)| *i == wt.icon)
            {
                boxes.push((wt.icon, wt.damage_upgrade));
            }
        }
        let first = if ty.has_shields { 4 } else { 3 };
        for (n, (ic, up)) in boxes.into_iter().enumerate().take((7 - first) as usize) {
            let mut r = control(self, first + n as i32);
            r.x += (UPGRADE_DRAW_SHIFT.0 * k).round() as i32;
            r.y += (UPGRADE_DRAW_SHIFT.1 * k).round() as i32;
            // Upgrade icons are drawn untinted (Boot Camp: grey art on the
            // dark box), unlike the yellow queue icons.
            if let Some(img) = self
                .console
                .as_ref()
                .and_then(|c| c.cmdicons4.as_ref())
                .and_then(|g| g.frame(ic as usize))
            {
                let w = (img.width as f32 * 0.75 * k).round() as i32;
                let h = (img.height as f32 * 0.75 * k).round() as i32;
                img.draw_scaled(fb, r.x + (r.w - w) / 2, r.y + (r.h - h) / 2, w, h);
            }
            frame(self, fb, r, 12);
            let level = self.world.players[self.view_player()]
                .upgrade_level(up)
                .to_string();
            let x = (r.x + r.w - cx0) as f32 / k - UPGRADE_LEVEL_DX;
            self.glue_text_face(
                fb,
                x,
                (r.y + r.h) as f32 / k - UPGRADE_LEVEL_DY,
                &level,
                HUD_SMALL_PX,
                HUD_TEXT_COLOR,
                None,
                GlueFace::Hud,
            );
        }
    }

    /// Shield upgrade of a single Protoss selection: `cmdbtns` frame 12
    /// (button with the level box), the upgrade icon and its level.
    pub(super) fn draw_hud_upgrade(&mut self, fb: &mut Framebuffer) {
        let Some(u) = self.first_selected().cloned() else {
            return;
        };
        if self.selected.len() != 1 || !self.data.unit(u.type_id).has_shields {
            return;
        }
        let k = self.content_box.3 as f32 / 1440.0;
        // Drawn 6 px up-left of the hit rect (shift search on 135051).
        let mut r = self.upgrade_button_rect();
        r.x += (UPGRADE_DRAW_SHIFT.0 * k).round() as i32;
        r.y += (UPGRADE_DRAW_SHIFT.1 * k).round() as i32;
        let Some(console) = &self.console else {
            return;
        };
        let icon = self
            .data
            .upgrade(PLASMA_SHIELDS)
            .map(|t| t.icon)
            .unwrap_or(0);
        if let Some(img) = console
            .cmdicons4
            .as_ref()
            .and_then(|g| g.frame(icon as usize))
        {
            let w = (img.width as f32 * 0.75 * k).round() as i32;
            let h = (img.height as f32 * 0.75 * k).round() as i32;
            img.draw_scaled(fb, r.x + (r.w - w) / 2, r.y + (r.h - h) / 2, w, h);
        }
        // The button frame with its level box goes over the icon.
        if let Some(img) = console
            .command_buttons_hd
            .as_ref()
            .and_then(|g| g.frame(12))
        {
            let w = (img.width as f32 * 0.75 * k).round() as i32;
            let h = (img.height as f32 * 0.75 * k).round() as i32;
            img.draw_scaled(fb, r.x + (r.w - w) / 2, r.y + (r.h - h) / 2, w, h);
        }
        let level = self.world.players[self.view_player()]
            .upgrade_level(PLASMA_SHIELDS)
            .to_string();
        let x = (r.x + r.w - self.content_box.0) as f32 / k - UPGRADE_LEVEL_DX;
        self.glue_text_face(
            fb,
            x,
            (r.y + r.h) as f32 / k - UPGRADE_LEVEL_DY,
            &level,
            HUD_SMALL_PX,
            HUD_TEXT_COLOR,
            None,
            GlueFace::Hud,
        );
    }

    /// `stat_f10` button 1: the race's button plate (`dlg` frame 0) and the
    /// MENU label over the console's own well.
    pub(super) fn draw_hud_menu_label(&mut self, fb: &mut Framebuffer) {
        if self.console.is_none() {
            return;
        }
        let r = self.menu_button_rect();
        let k = self.content_box.3 as f32 / 1440.0;
        if let Some(img) = self
            .console
            .as_ref()
            .and_then(|c| c.dlg4.as_ref())
            .and_then(|g| g.frame(MENU_PLATE_FRAME))
        {
            let (x, y, w, h) = menu_plate_rect(r, img.width, img.height, k);
            img.draw_scaled(fb, x, y, w, h);
        }
        let label = "MENU";
        let w = self.glue_measure(label, HUD_SMALL_PX, GlueFace::Hud);
        // The label sits in the race console's own MENU well, not centred in
        // the `stat_f10` rect: ink offsets measured on 135020 / 134905 /
        // 135051 (content px).
        let (dx, dy) = match self.console_race {
            Race::Zerg => (-9.0, 18.0),
            Race::Protoss => (-8.0, 13.0),
            _ => (-11.0, 18.0),
        };
        self.glue_text_face(
            fb,
            (r.x - self.content_box.0) as f32 / k + (r.w as f32 / k - w) / 2.0 + dx,
            r.y as f32 / k + HUD_SMALL_BASE + dy,
            label,
            HUD_SMALL_PX,
            HUD_MENU_COLOR,
            None,
            GlueFace::Hud,
        );
    }

    /// Command card: the retail button set of the selected unit
    /// (`galaxycraft::buttons`), each glyph from the 4x `cmdicons` at the
    /// race dialog's button rect; disabled buttons are grey.
    pub(super) fn draw_hud_card(&mut self, fb: &mut Framebuffer) {
        let Some((_u, card)) = self.current_card() else {
            return;
        };
        let slots = self.command_slots();
        let Some(console) = &self.console else {
            return;
        };
        let k = self.content_box.3 as f32 / 1440.0;
        let (sx, sy) = card_draw_shift(self.console_race);
        let (sx, sy) = ((sx * k).round() as i32, (sy * k).round() as i32);
        let pressed = self
            .card_pressed
            .filter(|(_, t)| t.elapsed().as_secs_f32() < CARD_PRESS_SECS)
            .map(|(i, _)| i);
        for (i, (slot, entry)) in slots.iter().zip(card.iter()).enumerate() {
            let Some((button, state)) = entry else {
                continue;
            };
            let down = pressed == Some(i);
            // Pressed: the glyph sits one console pixel down-right on the
            // `cmdbtns` pressed frame (1).  INFERENCE: frame 1 as the
            // pressed state; 135114 shows frame 0 under the pointer.
            let shift = if down {
                (2.0 * k).round().max(1.0) as i32
            } else {
                0
            };
            let slot = &UiRect::new(slot.x + sx + shift, slot.y + sy + shift, slot.w, slot.h);
            let frame = usize::from(down);
            if let Some(img) = console
                .command_buttons_hd
                .as_ref()
                .and_then(|g| g.frame(frame))
            {
                let w = (img.width as f32 * 0.75 * k).round() as i32;
                let h = (img.height as f32 * 0.75 * k).round() as i32;
                img.draw_scaled(
                    fb,
                    slot.x + (slot.w - w) / 2,
                    slot.y + (slot.h - h) / 2,
                    w,
                    h,
                );
            }
            let colour = match state {
                galaxycraft::buttons::ButtonState::Enabled => HUD_GLYPH_ENABLED,
                galaxycraft::buttons::ButtonState::Disabled => HUD_GLYPH_DISABLED,
            };
            if let Some(img) = console
                .cmdicons4
                .as_ref()
                .and_then(|g| g.frame(button.icon as usize))
            {
                let w = (img.width as f32 * 0.75 * k).round() as i32;
                let h = (img.height as f32 * 0.75 * k).round() as i32;
                let x = slot.x + (slot.w - w) / 2;
                let y = slot.y + (slot.h - h) / 2;
                draw_tinted(fb, img, (x, y, w, h), colour);
            }
        }
    }

    /// The map's trigger leaderboard, top-left (retail Wasteland: a 30 px
    /// player-colour square at (6,6) and "9 of 10 Marines" in green,
    /// `blizzard-regular` 31.1 px/em, ink x 50, ink rows 8..29).  One row per
    /// player in the game, highest first; computers only when action 32
    /// leaves them on, rescuable players never.
    pub(super) fn draw_trigger_leaderboard(&mut self, fb: &mut Framebuffer) {
        let Some(t) = self.world.triggers.as_ref() else {
            return;
        };
        let Some(lb) = t.leaderboard.clone() else {
            return;
        };
        let label = self
            .scenario
            .string(lb.text_id)
            .unwrap_or("")
            .trim()
            .to_string();
        let computers = t.leaderboard_computer_players;
        let slots = self.scenario.player_slots();
        let mut rows: Vec<(i64, usize)> = slots
            .iter()
            .filter(|s| s.active && (s.human || s.computer))
            .filter(|s| {
                self.world
                    .players
                    .get(s.slot)
                    .is_some_and(|p| !p.rescuable && (computers || !p.is_computer))
            })
            .map(|s| (t.leaderboard_value(&self.world, s.slot), s.slot))
            .collect();
        rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let (cx, _, _, ch) = Self::content_rect(fb);
        let k = ch as f32 / 1440.0;
        let px = |v: f32| (v * k).round() as i32;
        for (n, (value, slot)) in rows.into_iter().enumerate() {
            let dy = n as f32 * LEADERBOARD_PITCH;
            fb.rect(
                cx + px(6.0),
                px(6.0 + dy),
                px(30.0),
                px(30.0),
                BW_PLAYER_COLOURS[slot % 8],
            );
            let text = format!("{value} {label}");
            for (d, colour) in [(2.0, Rgb::new(0, 0, 0)), (0.0, LEADERBOARD_GREEN)] {
                self.glue_text_face(
                    fb,
                    48.0 + d,
                    30.0 + dy + d,
                    &text,
                    TRIGGER_TEXT_PX,
                    colour,
                    None,
                    GlueFace::Hud,
                );
            }
        }
    }

    /// The trigger countdown at the top of the screen once a map sets it
    /// (retail Desperate Alliance: "25:43", Eurostile, ink centred on
    /// x 1418 content px, rows 16..46, also shown at 0:00).
    pub(super) fn draw_countdown(&mut self, fb: &mut Framebuffer) {
        let Some(t) = self.world.triggers.as_ref().filter(|t| t.countdown_shown) else {
            return;
        };
        let text = countdown_text(t.countdown_timer);
        let w = self.glue_measure(&text, COUNTDOWN_PX, GlueFace::Big);
        self.glue_text_face(
            fb,
            COUNTDOWN_CENTRE_X - w / 2.0,
            COUNTDOWN_BASE,
            &text,
            COUNTDOWN_PX,
            HUD_TEXT_COLOR,
            None,
            GlueFace::Big,
        );
    }

    /// `statlb.ui.json` compact leaderboard (L).
    pub(super) fn draw_leaderboard(&self, fb: &mut Framebuffer, scale: f32, ox: i32, oy: i32) {
        let Some(dialog) = self
            .console
            .as_ref()
            .and_then(|c| c.layout("statlb"))
            .cloned()
        else {
            return;
        };
        let base = dialog
            .rect
            .scaled(scale, ox, oy)
            .offset(0, (24.0 * scale) as i32);
        fb.rect(
            base.x - 4,
            base.y - 18,
            base.w + 8,
            base.h + 24,
            Rgb::new(6, 9, 14).blend(Rgb::new(30, 38, 50), 130),
        );
        fb.rect_outline(
            base.x - 4,
            base.y - 18,
            base.w + 8,
            base.h + 24,
            Rgb::new(100, 130, 165),
        );
        fb.text(
            base.x + 4,
            base.y - 16,
            "PL  RACE   SUPPLY  SCORE",
            Rgb::new(130, 225, 160),
            1,
        );
        for row in 0..8 {
            let Some(player) = self.world.players.get(row) else {
                break;
            };
            let y = base.y + (row as f32 * 14.0 * scale) as i32;
            let color = if row == self.local_player {
                Rgb::new(255, 235, 150)
            } else if player.defeated {
                Rgb::new(130, 135, 145)
            } else {
                Rgb::new(205, 215, 230)
            };
            fb.text(base.x + 4, y, &format!("{}", row + 1), color, 1);
            let race = match player.race {
                Race::Terran => "T",
                Race::Zerg => "Z",
                Race::Protoss => "P",
                Race::Neutral => "N",
            };
            fb.text(base.x + (28.0 * scale) as i32, y, race, color, 1);
            let used = self.world.supply_used(row as u8) / 2;
            let cap = self.world.supply_cap(row as u8) / 2;
            fb.text(
                base.x + (100.0 * scale) as i32,
                y,
                &format!("{used}/{cap}"),
                color,
                1,
            );
            fb.text(
                base.x + (160.0 * scale) as i32,
                y,
                &format!("{}", player.scores[0]),
                color,
                1,
            );
        }
    }

    /// `msnstats.ui.json` panel (F10): the shipped 320x240 mission table.
    pub(super) fn draw_mission_stats(&self, fb: &mut Framebuffer, scale: f32, ox: i32, oy: i32) {
        let Some(dialog) = self
            .glue
            .as_ref()
            .and_then(|g| g.mission.as_ref())
            .or_else(|| self.console.as_ref().and_then(|c| c.layout("msnstats")))
        else {
            return;
        };
        let base = dialog
            .rect
            .scaled(scale, ox, oy)
            .offset((160.0 * scale) as i32, (120.0 * scale) as i32);
        fb.rect(
            base.x,
            base.y,
            base.w,
            base.h,
            Rgb::new(8, 11, 16).blend(Rgb::new(30, 38, 50), 120),
        );
        fb.rect_outline(base.x, base.y, base.w, base.h, Rgb::new(120, 145, 175));
        fb.text(
            base.x + 10,
            base.y + 8,
            "PLAYER        KILLS LOSSES RAZED UNITS",
            Rgb::new(150, 220, 170),
            1,
        );
        for (row, player) in self.world.players.iter().enumerate().take(8) {
            let y = base.y + 28 + row as i32 * 22;
            let units = player
                .death_counts
                .iter()
                .sum::<u32>()
                .saturating_sub(player.deaths);
            fb.text(
                base.x + 10,
                y,
                &format!("P{}", row + 1),
                if row == self.local_player {
                    Rgb::new(255, 225, 120)
                } else {
                    Rgb::new(200, 210, 225)
                },
                1,
            );
            fb.text(
                base.x + 80,
                y,
                &format!("{}", player.kills),
                Rgb::new(190, 205, 220),
                1,
            );
            fb.text(
                base.x + 140,
                y,
                &format!("{}", player.deaths),
                Rgb::new(190, 205, 220),
                1,
            );
            fb.text(
                base.x + 200,
                y,
                &format!("{}", player.razings),
                Rgb::new(190, 205, 220),
                1,
            );
            fb.text(
                base.x + 260,
                y,
                &format!("{}", units),
                Rgb::new(190, 205, 220),
                1,
            );
            if player.defeated {
                fb.text(base.x + 300, y, "OUT", Rgb::new(210, 120, 120), 1);
            }
        }
        fb.text(
            base.x + 10,
            base.y + base.h - 18,
            "F10: close mission statistics",
            Rgb::new(160, 175, 195),
            1,
        );
    }
}
