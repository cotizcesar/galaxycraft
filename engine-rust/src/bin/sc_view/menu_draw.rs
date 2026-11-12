//! `impl App`: menu draw (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    pub(super) fn draw_menu(&mut self, fb: &mut Framebuffer) {
        self.draw_menu_screen(fb);
        if std::mem::take(&mut self.swish_restart) {
            // The first frame of the phase is on screen (step 0): count the
            // slide from here.
            let now = Instant::now();
            match &mut self.glue_swish {
                Some(GlueSwish::In { started }) | Some(GlueSwish::Out { started, .. }) => {
                    *started = now
                }
                None => {}
            }
        }
        let (cx, _, cw, _) = Self::content_rect(fb);
        mask_pillarbox(fb, cx, cw);
    }

    /// Start, advance and finish the classic-dialog swish from the screen
    /// changes the input handlers make.  Pinned captures (`--menu-at`,
    /// `--menu-phase`) draw the settled screen.
    pub(super) fn advance_glue_swish(&mut self) {
        let current = self.menu_screen.clone();
        let previous = self.swish_screen.replace(current.clone());
        let changed = previous
            .as_ref()
            .is_some_and(|p| std::mem::discriminant(p) != std::mem::discriminant(&current));
        // Transition sounds (user report on the retail client, 2026-11-04:
        // every screen change "whooshes"): `swishout` when a classic dialog
        // starts leaving, `swishin` when the next screen shows.
        let leaving = changed && previous.as_ref().is_some_and(is_glue_list);
        if leaving {
            self.play("ui_swish_out");
        }
        // Leaving the main menu through one of its entries plays only the
        // press (`menu::screen_change_swish`).
        let swish_in = previous
            .as_ref()
            .is_some_and(|p| galaxycraft::menu::screen_change_swish(p, &current));
        if self.menu_at.is_some() || !self.menu_phases.is_empty() {
            // Pinned captures: no motion, the sounds still follow the change.
            if swish_in {
                self.play("ui_swish");
            }
            self.glue_swish = None;
            return;
        }
        if changed {
            let now = Instant::now();
            self.swish_restart = true;
            self.glue_swish = match previous {
                Some(from) if is_glue_list(&from) => Some(GlueSwish::Out { from, started: now }),
                _ if slides_in(&current) => Some(GlueSwish::In { started: now }),
                _ => None,
            };
            if !leaving && swish_in {
                self.play("ui_swish");
            }
        }
        self.glue_swish = match self.glue_swish.take() {
            Some(GlueSwish::Out { started, .. })
                if swish_step(started) >= galaxycraft::menu::swish_out_steps(SWISH_LIST_UI) =>
            {
                self.play("ui_swish");
                self.swish_restart = true;
                slides_in(&current).then(|| GlueSwish::In {
                    started: Instant::now(),
                })
            }
            Some(GlueSwish::In { started })
                if swish_step(started) >= galaxycraft::menu::swish_in_steps(SWISH_LIST_UI) =>
            {
                None
            }
            other => other,
        };
    }

    /// Offset (UI px, away from the box) of a plate `distance` UI px from
    /// leaving the 16:9 box, for the running swish.
    pub(super) fn swish_offset(&self, distance: f32) -> f32 {
        use galaxycraft::menu::{swish_in_offset, swish_out_offset};
        match &self.glue_swish {
            Some(GlueSwish::In { started }) => {
                let n = swish_step(*started);
                self.swish_drawn.set(Some((true, n)));
                swish_in_offset(distance, n)
            }
            Some(GlueSwish::Out { started, .. }) => {
                let n = swish_step(*started);
                self.swish_drawn.set(Some((false, n)));
                swish_out_offset(n)
            }
            None => {
                self.swish_drawn.set(None);
                0.0
            }
        }
    }

    /// Whether the dialog text shows: not while plates move (in: until the
    /// farthest plate is in place; out: never).
    pub(super) fn swish_text_shown(&self) -> bool {
        match &self.glue_swish {
            Some(GlueSwish::In { started }) => {
                galaxycraft::menu::swish_in_offset(SWISH_LIST_UI, swish_step(*started)) == 0.0
            }
            Some(GlueSwish::Out { .. }) => false,
            None => true,
        }
    }

    pub(super) fn draw_menu_screen(&mut self, fb: &mut Framebuffer) {
        // Ultrawide pillarbox: the menu content is a 16:9 box, so the side
        // bars are not touched by any sub-screen.  Clear the whole frame
        // first, otherwise the software cursor leaves stale arrows in them.
        // The main screen copies its fully composed cache over every pixel,
        // so it does not need the extra clear.
        if !matches!(self.menu_screen, MenuScreen::Main) {
            clear_menu_frame(fb);
        }
        // Metrics must be fresh here: the headless `--shot-screen` path calls
        // `draw_menu` directly (bypassing `draw`, which calls `update_view`),
        // so stale defaults (scale 2, ox 0) would misplace every panel and
        // entry at any other resolution.  Idempotent in the live path.
        self.update_view(fb);
        self.advance_glue_swish();
        if let Some(GlueSwish::Out { from, .. }) = self.glue_swish.clone() {
            // The previous dialog's plates leave before the new screen shows.
            match from {
                MenuScreen::SavedGames => self.draw_saved(fb),
                MenuScreen::Replays => self.draw_create(fb),
                MenuScreen::Connection
                | MenuScreen::Games
                | MenuScreen::MpCreate
                | MenuScreen::MpLobby => self.draw_mp(fb, &from),
                _ => self.draw_registry(fb),
            }
            return;
        }
        // Retail menu flow from the 44 original screenshots: dispatch to the
        // active sub-screen.  The Main screen lists all eight retail entries;
        // Episode/Mission/Lobby cover Single/Multi; Options/Hotkeys/
        // Collections/Credits/Registry/GameType cover the footer buttons.
        match self.menu_screen.clone() {
            MenuScreen::Registry => {
                self.draw_registry(fb);
                if self.registry_entry.is_some() {
                    self.draw_registry_new_id(fb);
                }
                if let Some(text) = self.registry_notice {
                    self.draw_registry_notice(fb, text);
                }
            }
            MenuScreen::GameType => self.draw_gametype(fb),
            MenuScreen::Episode => self.draw_episode(fb),
            MenuScreen::Mission { .. } => self.draw_mission(fb),
            MenuScreen::Lan => self.draw_lan(fb),
            ref s @ (MenuScreen::Connection
            | MenuScreen::Games
            | MenuScreen::MpCreate
            | MenuScreen::MpLobby) => self.draw_mp(fb, s),
            MenuScreen::Replays => self.draw_create(fb),
            MenuScreen::SavedGames => self.draw_saved(fb),
            MenuScreen::Lobby { .. } => self.draw_lobby(fb),
            MenuScreen::Options { tab } => {
                self.draw_options(fb);
                if self.options_reset {
                    self.draw_options_reset(fb, tab);
                }
            }
            MenuScreen::Hotkeys { .. } => self.draw_hotkeys(fb),
            MenuScreen::Collections => self.draw_collections(fb),
            MenuScreen::Credits => self.draw_credits(fb),
            MenuScreen::Intro { .. } => self.draw_intro(fb),
            MenuScreen::Main => self.draw_main_full(fb),
        }
    }

    pub(super) fn draw_overlay(&mut self, fb: &mut Framebuffer) {
        // Persistent modal requested with --start-screen (live client).
        match self.overlay {
            1 => {
                self.draw_objectives_panel(fb);
                return;
            }
            2 => {
                self.draw_tips_panel(fb);
                return;
            }
            3 => {
                self.draw_game_dialog(fb, GameDialog::Menu);
                return;
            }
            _ => {}
        }
        if self.game_result.is_some() && !self.end_dialog {
            // Lobby games use the multiplayer score screen; campaign and
            // single-player games the gluscore one.
            if self.mp.setup.is_some() {
                self.draw_mp_score(fb);
            } else {
                draw_in_box(self, fb, Self::draw_score_screen);
            }
            return;
        }
        if self.ingame_options {
            let tab = match self.menu_screen {
                MenuScreen::Options { tab } => tab,
                _ => OptionsTab::Speed,
            };
            self.draw_options(fb);
            if self.options_reset {
                self.draw_options_reset(fb, tab);
            }
            return;
        }
        if let Some(dialog) = self.game_dialog {
            self.draw_game_dialog(fb, dialog);
            return;
        }
        let w = fb.width;
        let h = fb.height;
        let (title, color, help) = if self.paused {
            (
                "PAUSED",
                Rgb::new(255, 220, 80),
                "F10 / ESC / click: Game Menu",
            )
        } else if let Some(win) = self.game_result.filter(|_| !self.end_dialog) {
            if win {
                (
                    "VICTORY",
                    Rgb::new(100, 255, 120),
                    if self.next_map.is_some() {
                        "ENTER next mission   R play again   Q quit"
                    } else {
                        "ENTER menu   R play again   Q quit"
                    },
                )
            } else {
                (
                    "DEFEAT",
                    Rgb::new(255, 90, 90),
                    "ENTER menu   R play again   Q quit",
                )
            }
        } else if self.replay.as_ref().map(|r| r.paused).unwrap_or(false) {
            (
                "REPLAY PAUSED",
                Rgb::new(255, 220, 80),
                "SPACE resume   +/- speed   R restart   ESC menu",
            )
        } else {
            return;
        };
        for y in (h / 2 - 120)..(h / 2 + 120) {
            for x in (w / 2 - 360)..(w / 2 + 360) {
                let c = fb.get(x, y);
                fb.put(x, y, c.blend(Rgb::new(0, 0, 0), 190));
            }
        }
        fb.text_centered(w / 2, h / 2 - 80, title, color, 4);
        if self.paused || self.replay.as_ref().map(|r| r.paused).unwrap_or(false) {
            let detail = if let Some(rp) = self.replay.as_ref() {
                format!(
                    "{} - REPLAY tick {}/{}",
                    rp.save.map_name, self.world.tick, rp.save.tick
                )
            } else {
                format!(
                    "{} - tick {}",
                    self.maps[self.map_index].name, self.world.tick
                )
            };
            fb.text_centered(w / 2, h / 2 + 10, &detail, Rgb::new(220, 220, 220), 2);
            fb.text_centered(w / 2, h / 2 + 60, help, Rgb::new(190, 200, 220), 1);
            return;
        }
        let win = self.game_result == Some(true);
        let p = &self.world.players[self.view_player()];
        fb.text_centered(
            w / 2,
            h / 2 - 18,
            &format!(
                "{}   {} elapsed",
                self.maps[self.map_index].name,
                format_time(self.elapsed_ticks())
            ),
            Rgb::new(230, 230, 230),
            2,
        );
        let stat = format!(
            "Units killed {:<5} losses {:<5} buildings razed {:<4}    Minerals {}   Gas {}",
            p.kills, p.deaths, p.razings, p.minerals_gathered, p.gas_gathered
        );
        fb.text_centered(w / 2, h / 2 + 26, &stat, Rgb::new(170, 200, 170), 1);
        let score = if win {
            "MISSION ACCOMPLISHED"
        } else {
            "MISSION FAILED - the colony was lost"
        };
        fb.text_centered(w / 2, h / 2 + 52, score, Rgb::new(200, 200, 200), 1);
        let next = if win {
            self.next_map
                .and_then(|i| self.maps.get(i))
                .map(|m| m.name.as_str())
        } else {
            None
        };
        let help = if let Some(name) = next {
            fb.text_centered(
                w / 2,
                h / 2 + 76,
                &format!("NEXT: {name}"),
                Rgb::new(255, 220, 120),
                1,
            );
            "ENTER next mission    M menu    R replay    Q quit"
        } else {
            help
        };
        fb.text_centered(w / 2, h / 2 + 100, help, Rgb::new(190, 200, 220), 1);
    }

    /// Text at `size_ui` UI pixels (cap height scales with the content box)
    /// in the HUD face, top-left or top-centred at a screen point.  The 8x8
    /// bitmap font is the fallback when the install has no fonts.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn ui_label(
        &mut self,
        fb: &mut Framebuffer,
        text: &str,
        x: i32,
        y: i32,
        size_ui: f32,
        color: Rgb,
        centered: bool,
    ) {
        let px = (size_ui * self.ui_scale).max(6.0);
        if let Some(font) = self.ui_font_info.as_mut() {
            if centered {
                font.draw_centered(fb, x, y, text, px, 0.0, color, None);
            } else {
                font.draw_top(fb, x, y, text, px, 0.0, color, None);
            }
        } else if centered {
            fb.text_centered(x, y, text, color, 1);
        } else {
            fb.text(x, y, text, color, 1);
        }
    }
    pub(super) fn draw_menu_panel(
        &mut self,
        fb: &mut Framebuffer,
        title: &str,
        lines: &[String],
        sel: usize,
        help: &str,
    ) {
        // `menu::lan_panel_rect` is in the 2560x1440 box space; text is in
        // UI px (`ui_label`), so the panel keeps its look from 720p up.
        let (cx, cy, _cw, ch) = Self::content_rect(fb);
        let k = ch as f32 / 1440.0;
        let px = |r: galaxycraft::menu::HitRect| {
            (
                cx + (r.0 * k).round() as i32,
                cy + (r.1 * k).round() as i32,
                (r.2 * k).round() as i32,
                (r.3 * k).round() as i32,
            )
        };
        let panel = galaxycraft::menu::lan_panel_rect(lines.len());
        let (bx, by, bw, bh) = px(panel);
        for y in by..by + bh {
            for x in bx..bx + bw {
                let c = fb.get(x, y);
                fb.put(x, y, c.blend(Rgb::new(6, 10, 18), 200));
            }
        }
        fb.rect_outline(bx, by, bw, bh, Rgb::new(110, 150, 185));
        let s = self.ui_scale;
        self.ui_label(
            fb,
            title,
            bx + bw / 2,
            by + (10.0 * s) as i32,
            12.0,
            Rgb::new(220, 230, 245),
            true,
        );
        for (i, line) in lines.iter().enumerate() {
            let (rx, ry, rw, rh) = px(galaxycraft::menu::lan_row_rect(panel, i));
            let selected = i == sel;
            if selected {
                fb.rect(rx, ry, rw, rh, Rgb::new(36, 58, 82));
                fb.rect_outline(rx, ry, rw, rh, Rgb::new(100, 190, 255));
            }
            let color = if selected {
                Rgb::new(255, 245, 180)
            } else {
                Rgb::new(190, 200, 215)
            };
            let ty = ry + (rh - (8.0 * s) as i32) / 2;
            self.ui_label(fb, line, rx + (4.0 * s) as i32, ty, 8.0, color, false);
        }
        self.ui_label(
            fb,
            help,
            bx + bw / 2,
            by + bh - (16.0 * s) as i32,
            7.0,
            Rgb::new(150, 170, 195),
            true,
        );
    }

    pub(super) fn draw_main_full(&mut self, fb: &mut Framebuffer) {
        // Retail `glumain` layout: full-bleed space backdrop, the animated
        // fleet / battlecruiser / vortex clips at their `flc.json`
        // rectangles, the eight green entries, and the title frame with the
        // Build footer at the bottom-right of the title frame.
        //
        // The backdrop and title chrome are static: compose them once per
        // resolution and copy the cached pixels every frame.  Only the clips,
        // entries and profile text are redrawn.
        let t0 = Instant::now();
        self.copy_menu_static(fb, true);
        let t1 = Instant::now();
        self.draw_menu_clips(fb);
        let t2 = Instant::now();
        self.draw_menu_entries(fb, true);
        let t3 = Instant::now();
        self.draw_menu_frame_text(fb);
        let t4 = Instant::now();
        if std::env::var_os("GC_PROFILE").is_some() {
            eprintln!(
                "menu: static {:.1} clips {:.1} entries {:.1} text {:.1} ms",
                (t1 - t0).as_secs_f64() * 1000.0,
                (t2 - t1).as_secs_f64() * 1000.0,
                (t3 - t2).as_secs_f64() * 1000.0,
                (t4 - t3).as_secs_f64() * 1000.0
            );
        }
    }

    /// Copy the main menu's static layer (backdrop, detail art, title
    /// frame and panel ring), composing it once per window size.
    ///
    /// `panel_text` draws the offline news loading ring in the title-frame
    /// panel.  Only the main menu itself shows it: behind every modal and
    /// sub-screen the retail panel is empty, so the ring does not bleed
    /// through the Options and Collections plates.
    pub(super) fn copy_menu_static(&mut self, fb: &mut Framebuffer, panel_text: bool) {
        let (w, h) = (fb.width as usize, fb.height as usize);
        let slot = if panel_text {
            &self.menu_static
        } else {
            &self.menu_static_bare
        };
        let stale = slot
            .as_ref()
            .map(|(cw, ch, _)| *cw != w || *ch != h)
            .unwrap_or(true);
        if stale {
            let mut cache = Framebuffer::new(w, h);
            cache.clear(Rgb::new(0, 0, 0));
            self.draw_menu_backdrop_plain(&mut cache);
            self.draw_menu_detail_art(&mut cache);
            self.draw_menu_frame_art(&mut cache, 0, 0);
            if panel_text {
                self.draw_menu_panel_text(&mut cache, 0, 0);
                self.menu_static = Some((w, h, cache.pixels));
            } else {
                self.menu_static_bare = Some((w, h, cache.pixels));
            }
        }
        let slot = if panel_text {
            &self.menu_static
        } else {
            &self.menu_static_bare
        };
        if let Some((_, _, pixels)) = slot {
            fb.pixels.copy_from_slice(pixels);
        }
    }

    /// Static chrome shared by the main menu and the sub-screens: the menu
    /// detail art behind the title frame.
    pub(super) fn draw_menu_detail_art(&self, fb: &mut Framebuffer) {
        if let Some(glue) = &self.glue
            && let (Some(dialog), Some(art)) = (&glue.menu, &glue.etail)
            && let Some(c) = dialog.control(1)
        {
            let (scale, ox, oy) = self.ui_metrics(fb);
            let r = c.abs.scaled(scale, ox, oy);
            art.draw_scaled(fb, r.x, r.y, r.w, r.h);
        }
    }

    /// Draw the six Remastered button clips at the rectangles shipped in
    /// `rez/flc.json` (relative to each FLCBTN's 640x480 origin).
    pub(super) fn draw_menu_clips(&mut self, fb: &mut Framebuffer) {
        let hover = self.main_sel();
        let (scale, ox, oy) = self.ui_metrics(fb);
        let ui = |x: f32, y: f32, w: f32, h: f32| -> (i32, i32, i32, i32) {
            (
                ox + (x * scale).round() as i32,
                oy + (y * scale).round() as i32,
                (w * scale).round() as i32,
                (h * scale).round() as i32,
            )
        };
        // `flc.json`: Single [-20,0,425,175] / SingleOn [-40,-10,425,200];
        // Multi [-10,-5,282,270] / MultiOn [-52,-2,330,220]; Exit
        // [-2,-4,204,182] / ExitOn [5,0,176,160].
        // One source of truth: `menu::FLC_RECTS` is the shipped `flc.json`
        // rectangle list, unit-tested against the fixture.
        #[allow(clippy::type_complexity)]
        let rect = |idx: usize, is_hover: bool| -> (f32, f32, f32, f32) {
            let (slot, base) = match idx {
                0 => (0, (0.0, 0.0)),
                1 => (2, (78.0, 190.0)),
                _ => (4, (395.0, 320.0)),
            };
            let r = galaxycraft::menu::FLC_RECTS[slot + usize::from(is_hover)].1;
            (base.0 + r.0, base.1 + r.1, r.2, r.3)
        };
        let Some(layers) = &mut self.menu_layers else {
            return;
        };
        // Every clip runs on one continuous clock, hovered or not
        // (`menu_clip_visibility`).  Hidden mouse-over clips are still
        // pulled each frame, so a hover shows whatever frame that clock has
        // reached.
        for (idx, is_hover) in MENU_CLIP_ORDER {
            let (x, y, w, h) = rect(idx, is_hover);
            if !menu_clip_visibility(idx, is_hover, hover).drawn {
                if let Some(clip) = layers.clip(idx, is_hover) {
                    let _ = clip.frame();
                }
                continue;
            }
            if let Some(clip) = layers.clip(idx, is_hover) {
                let alpha_known = clip.alpha_known;
                let cached_alpha = clip.has_alpha;
                if let Some(frame) = clip.frame() {
                    let additive = if alpha_known {
                        !cached_alpha
                    } else {
                        !frame.has_alpha()
                    };
                    draw_poster_rgba_additive(fb, frame, ui(x, y, w, h), 0.0, additive);
                }
            }
        }
    }

    /// Blend a measured coverage mask over the current framebuffer.  Masks
    /// are calibrated at the native 2560x1440 content box; other scales fall
    /// back to the font renderer.
    pub(super) fn draw_text_mask(
        &self,
        fb: &mut Framebuffer,
        mask: &galaxycraft::menu_masks::TextMask,
        scale: f32,
        _ox: i32,
        _oy: i32,
    ) -> bool {
        // Masks absent from this checkout (gitignored retail data) are empty.
        if (scale - 3.0).abs() > 0.01 || mask.alpha.len() < mask.w as usize * mask.h as usize {
            return false;
        }
        let (cx, _cy, _cw, _ch) = Self::content_rect(fb);
        let x0 = cx + mask.x;
        let y0 = mask.y;
        for row in 0..mask.h as usize {
            for col in 0..mask.w as usize {
                let a = mask.alpha[row * mask.w as usize + col];
                if a == 0 {
                    continue;
                }
                let c = if mask.split > 0 && mask.x + col as i32 >= mask.split {
                    mask.c1
                } else {
                    mask.c0
                };
                let dst = fb.get(x0 + col as i32, y0 + row as i32);
                fb.put(
                    x0 + col as i32,
                    y0 + row as i32,
                    dst.blend(Rgb::new(c[0], c[1], c[2]), a),
                );
            }
        }
        true
    }

    /// The eight green entries at their shipped 640x480 spots. Drawn by the
    /// main menu (with the cursor highlight) and, without highlight, behind
    /// every sub-screen panel — the retail client composites them that way.
    /// Localized classic glue label, or the English fallback when the
    /// selected locale has no `rc3.json` entry.
    pub(super) fn ui_text(&self, key: &str, fallback: &str) -> String {
        self.ui_strings
            .get(key)
            .cloned()
            .filter(|s| !s.trim().is_empty() && !s.contains("PLACEHOLDER"))
            .unwrap_or_else(|| fallback.to_string())
    }

    /// Localized CEF chrome label (`gluall.json`), with an English fallback
    /// for strings the selected locale does not ship.
    pub(super) fn cef_text(&self, key: &str, fallback: &str) -> String {
        self.cef_strings
            .get(key)
            .cloned()
            .filter(|s| !s.trim().is_empty() && !s.contains("PLACEHOLDER"))
            .unwrap_or_else(|| fallback.to_string())
    }

    /// Lobby / create-screen term (`lobbyScreen_*`): race, colour, occupant
    /// or "Select Race".  The key is the English label without spaces; the
    /// lower-case form covers the race keys (`lobbyScreen_zerg`).
    pub(super) fn lobby_label(&self, name: &str) -> String {
        if name.is_empty() {
            return String::new();
        }
        if name == "Select Race" {
            return self.cef_text("lobbyScreen_selectRace", name);
        }
        let compact = name.replace(' ', "");
        for key in [
            format!("lobbyScreen_{compact}"),
            format!("lobbyScreen_{}", compact.to_ascii_lowercase()),
        ] {
            if let Some(value) = self.cef_strings.get(&key)
                && !value.trim().is_empty()
                && !value.contains("PLACEHOLDER")
            {
                return value.clone();
            }
        }
        name.to_string()
    }

    /// Hotkey command-card heading (`glue_hotkeysPanel_*_actionCard`).
    pub(super) fn action_card_label(&self, label: &str) -> String {
        let key = match label {
            "General" => "glue_hotkeysPanel_general_actionCard",
            "Basic Structure" => "glue_hotkeysPanel_basicStructure_actionCard",
            "Advanced Structure" => "glue_hotkeysPanel_advancedStructure_actionCard",
            "General - Tank Mode" => "glue_hotkeysPanel_tankMode_actionCard",
            "General - Siege Mode" => "glue_hotkeysPanel_siegeMode_actionCard",
            "General - Uncloaked" => "glue_hotkeysPanel_uncloaked_actionCard",
            "General - Cloaked" => "glue_hotkeysPanel_cloaked_actionCard",
            "General - Lifted" => "glue_hotkeysPanel_lifted_actionCard",
            "General - Burrowed" => "glue_hotkeysPanel_burrowed_actionCard",
            _ => return label.to_string(),
        };
        self.cef_text(key, label)
    }

    /// Shipped tileset name (`gameMapERA*`; index 0 has no suffix).
    pub(super) fn tileset_label(&self, name: &str) -> String {
        let key = match name {
            "Badlands" => "gameMapERA",
            "Space" => "gameMapERA1",
            "Installation" => "gameMapERA2",
            "Ash World" | "Ashworld" => "gameMapERA3",
            "Jungle" => "gameMapERA4",
            "Desert" => "gameMapERA5",
            "Ice" => "gameMapERA6",
            _ => "gameMapERA7",
        };
        self.cef_text(key, name)
    }

    /// Race name in the lobby terms (`lobbyScreen_zerg`, ...).
    pub(super) fn race_label(&self, race: galaxycraft::sim::Race) -> String {
        match race {
            galaxycraft::sim::Race::Terran => self.lobby_label("Terran"),
            galaxycraft::sim::Race::Zerg => self.lobby_label("Zerg"),
            galaxycraft::sim::Race::Protoss => self.lobby_label("Protoss"),
            galaxycraft::sim::Race::Neutral => self.lobby_label("Random"),
        }
    }

    /// Main-menu labels in the current locale (`gluMain-*`).
    pub(super) fn main_labels(&self) -> [String; 8] {
        [
            self.ui_text("gluMain-3", "Single Player"),
            self.ui_text("gluMain-4", "Multiplayer"),
            self.ui_text("gluMain-5", "Options"),
            self.ui_text("gluMain-6", "Collections"),
            self.ui_text("gluMain-8", "Hotkeys"),
            self.ui_text("gluMain-9", "Show Credits"),
            self.ui_text("gluMain-7", "View Intro"),
            self.ui_text("gluMain-1", "Exit"),
        ]
    }

    pub(super) fn draw_menu_entries(&mut self, fb: &mut Framebuffer, highlight: bool) {
        if std::env::var_os("GC_NO_TEXT").is_some() {
            return;
        }
        // Behind a sub-screen panel the retail client shows these entries
        // dimmed (measured mean 81,101,50 against 110,233,98 when lit).
        let dim = !highlight;
        let sel = if highlight { self.main_sel() } else { None };
        let items = self.main_labels();
        // Measured on the retail captures: the idle face peaks at
        // (102, 227, 53) and the mouse-over face at (196, 255, 124).
        let normal = if dim {
            Rgb::new(45, 110, 45)
        } else {
            Rgb::new(102, 227, 53)
        };
        let hover = if dim {
            Rgb::new(85, 110, 85)
        } else {
            Rgb::new(196, 255, 124)
        };
        let (scale, ox, oy) = self.ui_metrics(fb);
        if highlight
            && std::env::var("GC_TEXT_MASKS").ok().as_deref() == Some("1")
            && std::env::var_os("GC_RAW_RENDER").is_none()
            && (scale - 3.0).abs() < 0.01
            && matches!(sel, None | Some(0) | Some(1))
        {
            use galaxycraft::menu_masks as m;
            const IDLE: [&m::TextMask; 8] = [
                &m::SINGLE,
                &m::MULTI,
                &m::OPTIONS,
                &m::COLLECTIONS,
                &m::HOTKEYS,
                &m::CREDITS,
                &m::INTRO,
                &m::EXIT,
            ];
            for (idx, mask) in IDLE.iter().enumerate() {
                let draw = match (sel, idx) {
                    (Some(0), 0) => &m::SINGLE_HOVER,
                    (Some(1), 1) => &m::MULTI_HOVER,
                    _ => *mask,
                };
                self.draw_text_mask(fb, draw, scale, ox, oy);
            }
            return;
        }
        let bright = if dim {
            Rgb::new(85, 110, 85)
        } else {
            Rgb::new(208, 255, 131)
        };
        for label in galaxycraft::menu::MAIN_LABELS {
            let Some(text) = items.get(label.idx) else {
                continue;
            };
            let hovered = Some(label.idx) == sel;
            self.draw_main_label(
                fb, text, label, hovered, normal, bright, hover, scale, ox, oy,
            );
        }
    }

    /// Retail menu label: the first glyph is the hotkey letter, drawn with
    /// the wide Eurostile Extended face and a brighter green; the rest uses
    /// the regular face.  On hover the whole word switches to the extended,
    /// bright face (reference #130704/#130712).  The full ink width and
    /// anchor come from [`galaxycraft::menu::MAIN_LABELS`].
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_main_label(
        &mut self,
        fb: &mut Framebuffer,
        text: &str,
        label: galaxycraft::menu::MainLabel,
        hovered: bool,
        normal: Rgb,
        bright: Rgb,
        hover: Rgb,
        scale: f32,
        ox: i32,
        oy: i32,
    ) {
        use galaxycraft::menu::LabelAlign;
        let left = match label.align {
            LabelAlign::Left => label.x,
            LabelAlign::Center => label.x - label.width / 2.0,
        };
        // Every entry keeps its glyph layout on hover and only changes
        // colour: switching the face re-spaced every letter after the
        // first, which read as the word sliding (user reports on the retail
        // client: the footer entries 2026-11-04, Single Player / Multiplayer
        // 2026-11-05).  [`main_label_hover_colors`] is the shared rule.
        let (normal, bright) = main_label_hover_colors(hovered, normal, bright, hover);
        let mut chars = text.chars();
        let Some(first) = chars.next() else {
            return;
        };
        let rest = chars.as_str();
        let first_string = first.to_string();
        let advance = self.hd_advance_ui(&first_string, scale, true);
        self.draw_hd_text(
            fb,
            &first_string,
            left,
            label.y,
            LabelAlign::Left,
            1.0,
            16.0,
            scale,
            ox,
            oy,
            bright,
            true,
        );
        if !rest.is_empty() {
            self.draw_hd_text(
                fb,
                rest,
                left + advance,
                label.y,
                LabelAlign::Left,
                (label.width - advance).max(1.0),
                16.0,
                scale,
                ox,
                oy,
                normal,
                true,
            );
        }
    }

    /// Advance width of `text` in UI pixels for one of the two faces.
    pub(super) fn hd_advance_ui(&mut self, text: &str, scale: f32, hover_face: bool) -> f32 {
        self.hd_advance_px(text, 16.0, scale, hover_face)
    }

    /// [`Self::hd_advance_ui`] at an explicit em size in UI pixels.
    pub(super) fn hd_advance_px(
        &mut self,
        text: &str,
        font_ui: f32,
        scale: f32,
        hover_face: bool,
    ) -> f32 {
        let px = (font_ui * scale).max(7.0);
        let use_hover = hover_face && self.ui_font_hover.is_some();
        let font = if use_hover {
            self.ui_font_hover.as_mut()
        } else {
            self.ui_font.as_mut()
        };
        font.map(|f| f.measure(text, px, 0.0) as f32 / scale)
            .unwrap_or(text.chars().count() as f32 * 8.0)
    }

    /// Draw one label with the retail face, a selectable em size and the
    /// measured ink width.  Main-menu labels use 16 UI px; the smaller
    /// `Cancel` in the game-type popup uses 14.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_hd_text(
        &mut self,
        fb: &mut Framebuffer,
        text: &str,
        anchor_x: f32,
        top: f32,
        align: galaxycraft::menu::LabelAlign,
        width: f32,
        font_ui: f32,
        scale: f32,
        ox: i32,
        oy: i32,
        color: Rgb,
        hover_face: bool,
    ) {
        use galaxycraft::menu::LabelAlign;
        let px = (font_ui * scale).max(7.0);
        let x = ox + (anchor_x * scale).round() as i32;
        let y = oy + (top * scale).round() as i32;
        let use_hover = hover_face && self.ui_font_hover.is_some();
        let font_opt = if use_hover {
            self.ui_font_hover.as_mut()
        } else {
            self.ui_font.as_mut()
        };
        if let Some(font) = font_opt {
            let natural = font.measure(text, px, 0.0) as f32;
            let target = width * scale;
            let gaps = text.chars().count().saturating_sub(1).max(1) as f32;
            // The retail labels are letter-spaced Eurostile: reproduce the
            // measured ink width by distributing the difference as tracking.
            let tracking = ((target - natural) / gaps).clamp(-px * 0.04, px * 0.35);
            // The retail glue labels carry the same hard drop shadow as the
            // build footer: an opaque black copy one UI pixel down-right
            // (measured 3 px at 1440p on reference 130637 for "Options",
            // "Single Player" and "Show Credits").  The font's built-in
            // 1 px / alpha-200 shadow was nearly invisible at 3x.
            let shadow = std::env::var_os("GC_NO_LABEL_SHADOW")
                .is_none()
                .then(|| label_drop_shadow(scale));
            let mut pass = |fb: &mut Framebuffer, x: i32, y: i32, c: Rgb| match align {
                LabelAlign::Left => {
                    font.draw_top(fb, x, y, text, px, tracking, c, None);
                }
                LabelAlign::Center => {
                    font.draw_centered(fb, x, y, text, px, tracking, c, None);
                }
            };
            if let Some((off, shade)) = shadow {
                pass(fb, x + off, y + off, shade);
            }
            pass(fb, x, y, color);
        } else {
            let big = (scale.round() as i32).clamp(1, 3);
            let w = text.chars().count() as i32 * 8 * big;
            let bx = match align {
                LabelAlign::Left => x,
                LabelAlign::Center => x - w / 2,
            };
            fb.text(bx, y, text, color, big);
        }
    }

    pub(super) fn main_sel(&self) -> Option<usize> {
        self.main_cursor
    }

    /// Set the hovered entry.  Reference #130704 does **not** keep an
    /// outgoing Multi overlay: its apparent plate is the battlecruiser hull
    /// and nebula, and the green-mask area in the MultiOn rectangle drops
    /// from 11 827 px (#130712) to 133 px (#130704).  Hover is therefore a
    /// plain active-clip switch, as the shipped `FLC_MOUSEOVER` data shows.
    pub(super) fn set_main_hover(&mut self, new: Option<usize>) {
        self.main_cursor = new;
    }

    /// Remastered space scene: animated WebM layers composited additively
    /// over the nebula. Shared by the main menu and every sub-screen so the
    /// whole front-end sits on the same backdrop as the retail client.
    pub(super) fn draw_space_scene(&mut self, fb: &mut Framebuffer) {
        // No-install fallback: deterministic star field.
        draw_space_backdrop(fb);
    }

    /// Backdrop only (no frame): the main menu draws the frame at its own
    /// anchor, every other screen at the sub-screen anchor.
    pub(super) fn draw_menu_backdrop_plain(&mut self, fb: &mut Framebuffer) {
        // `glue/palmm/backgnd.dds` is a 3412x1920 (16:9) painting.  The
        // retail references show it edge to edge inside the content box, so
        // it is drawn at exactly that rectangle: no inset, no crop.
        let (cx, cy, cw, ch) = Self::content_rect(fb);
        let stale = self
            .backdrop_fit
            .as_ref()
            .map(|(w, h, _)| *w != cw as usize || *h != ch as usize)
            .unwrap_or(true);
        if stale {
            self.backdrop_fit = self
                .glue
                .as_ref()
                .and_then(|g| g.palmm_backdrop.as_ref())
                .map(|art| {
                    (
                        cw as usize,
                        ch as usize,
                        art.resize(cw as usize, ch as usize),
                    )
                });
        }
        if let Some((_, _, art)) = &self.backdrop_fit {
            for y in 0..ch as usize {
                for x in 0..cw as usize {
                    let p = art.pixels[y * art.width + x];
                    fb.put(cx + x as i32, cy + y as i32, Rgb(p & 0x00ff_ffff));
                }
            }
        } else {
            self.draw_space_scene(fb);
        }
    }

    /// Sub-screen backdrop: same scene, frame anchored 190 px further right
    /// (measured: the menubar structure sits at x 2497..2873 in the sub-screens
    /// and at 2135..2607 in the main menu).
    pub(super) fn draw_menu_backdrop(&mut self, fb: &mut Framebuffer) {
        let (_, _, cw, _) = Self::content_rect(fb);
        self.draw_menu_backdrop_at(fb, (cw as f32 * 0.0555) as i32);
    }

    /// Backdrop with the frame at an explicit offset. The Hotkeys screen keeps
    /// the main-menu anchor (measured: shifting it there is worse).
    pub(super) fn draw_menu_backdrop_at(&mut self, fb: &mut Framebuffer, dx: i32) {
        self.draw_menu_backdrop_plain(fb);
        self.draw_menu_frame(fb, dx, 0);
        // The retail sub-screens composite the menu entries under the panel.
        self.draw_menu_entries(fb, false);
    }

    /// Options backdrop: the retail CEF modal sits over the *running* main
    /// menu, so the Single/Multi/Exit clips stay visible under the dim
    /// (references 130933..130954 show the Exit galaxy and the Multi ship).
    /// The title frame keeps the main-menu anchor (dx 0), not the 5.55 %
    /// sub-screen shift: on 130933 the frame's left arm is at x 2170..2250
    /// of the content box, 142 px left of the shifted art.
    pub(super) fn draw_menu_backdrop_live(&mut self, fb: &mut Framebuffer) {
        // Same static layer as the main menu (backdrop, detail art, frame),
        // composed once per size, with an empty panel.
        self.copy_menu_static(fb, false);
        self.draw_menu_frame_text(fb);
        self.draw_menu_clips(fb);
        // The live menu labels in their idle faces (nothing hovered); the
        // modal's 149 dim then gives the measured (42,93,22) = 0.416 x
        // (102,227,53).
        let hovered = self.main_cursor.take();
        self.draw_menu_entries(fb, true);
        self.main_cursor = hovered;
    }

    /// Title-frame surfaces plus the profile name, as every retail front-end
    /// screen shows them behind its panel.
    pub(super) fn draw_menu_frame_art(&self, fb: &mut Framebuffer, dx: i32, dy: i32) {
        let (scale, ox, oy) = self.ui_metrics(fb);
        let mut drew_art = false;
        if let Some(glue) = &self.glue
            && let Some(dialog) = &glue.menu
        {
            for (id, art) in [
                (22, glue.title_bg.as_ref()),
                (23, glue.title_overlay.as_ref()),
            ] {
                if let (Some(c), Some(art)) = (dialog.control(id), art) {
                    let mut r = c.abs.scaled(scale, ox, oy);
                    if id == 22 {
                        r.x -= 3;
                    }
                    // The Remastered surfaces are authored at 4x the
                    // 640x480 UI and drawn at that native size from the
                    // control origin.  The overlay is 1584x1054, so its
                    // red panel border (152..974) lands exactly on the
                    // 820x820 `titleframe_bg` panel; the stale control
                    // width (266) would squash it to half size.
                    let w = (art.width as f32 * scale / 4.0).round() as i32;
                    let h = (art.height as f32 * scale / 4.0).round() as i32;
                    art.draw_scaled(fb, r.x + dx, r.y + dy, w, h);
                    drew_art = true;
                }
            }
        }
        let panel = UiRect::new(413, 66, 205, 205).scaled(scale, ox, oy);
        if !drew_art {
            fb.rect(
                panel.x + dx,
                panel.y + dy,
                panel.w,
                panel.h,
                Rgb::new(6, 3, 3),
            );
            fb.rect_outline(
                panel.x + dx,
                panel.y + dy,
                panel.w,
                panel.h,
                Rgb::new(150, 25, 25),
            );
        }
    }

    /// The title-frame panel above Exit.  Offline the retail client shows a
    /// small green loading ring there (reference picture-7); the text that
    /// used to introduce the reconstruction is gone so the main menu matches
    /// the person's capture.
    pub(super) fn draw_menu_panel_text(&mut self, fb: &mut Framebuffer, dx: i32, dy: i32) {
        if std::env::var_os("GC_NO_TEXT").is_some() {
            return;
        }
        let (scale, ox, oy) = self.ui_metrics(fb);
        let (px0, py0, pw, ph) = MENU_PANEL_RECT;
        let cx = dx as f32 + ox as f32 + (px0 + pw / 2.0) * scale;
        let cy = dy as f32 + oy as f32 + (py0 + ph / 2.0) * scale;
        let radius = MENU_SPINNER_RADIUS_UI * scale;
        let stroke = MENU_SPINNER_STROKE_UI * scale;
        let outer = radius + stroke / 2.0;
        let inner = radius - stroke / 2.0;
        let left = cx - radius;
        let span = (radius * 2.0).max(1.0);
        let y0 = (cy - outer).floor() as i32;
        let y1 = (cy + outer).ceil() as i32;
        let x0 = (cx - outer).floor() as i32;
        let x1 = (cx + outer).ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (fx, fy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let d = (fx * fx + fy * fy).sqrt();
                // One-pixel antialias on both edges of the ring.
                let edge = (d - radius).abs();
                let a = (1.0 - (edge - stroke / 2.0).max(0.0)).clamp(0.0, 1.0);
                if a <= 0.0 || d < inner || d > outer {
                    continue;
                }
                // Bright right side / dark left side of the spinner.
                let t = ((x as f32 + 0.5 - left) / span).clamp(0.0, 1.0);
                let g = (70.0 + 180.0 * t).round() as u8;
                let colour = Rgb::new(0, g, 4);
                let base = fb.get(x, y);
                fb.put(x, y, base.blend(colour, (a * 255.0).round() as u8));
            }
        }
    }

    /// Build footer of the title frame.  Measured on reference 130637:
    /// `blizzard-regular.ttf` (the shipped HUD face; IoU 0.55 against the
    /// retail ink, LiberationSans 0.18), ink box 431x28 px at 1440p with its
    /// right edge at UI x 633.67 and its top at UI y 468.67, colour
    /// (198,218,255) and a soft dark shadow one UI pixel down-right.
    pub(super) fn draw_menu_frame_text(&mut self, fb: &mut Framebuffer) {
        if std::env::var_os("GC_NO_TEXT").is_some() {
            return;
        }
        let footer = format!("1.23.10.13515 | {} | 64-bit", locale_tag(&self.locale_code));
        let (scale, ox, oy) = self.ui_metrics(fb);
        let Some(font) = self.ui_font_info.as_mut() else {
            return;
        };
        let (off, shade) = label_drop_shadow(scale);
        let colour = Rgb::new(198, 218, 255);
        let px = BUILD_FOOTER_PX * scale;
        let width = font.measure(&footer, px, 0.0);
        let x = ox + (BUILD_FOOTER_RIGHT * scale).round() as i32 - width;
        let top = oy + (BUILD_FOOTER_TOP * scale).round() as i32;
        font.draw_top(fb, x + off, top + off, &footer, px, 0.0, shade, None);
        font.draw_top(fb, x, top, &footer, px, 0.0, colour, None);
    }

    /// Title-frame surfaces plus the profile name, as every retail front-end
    /// screen shows them behind its panel.
    /// The panel stays empty behind sub-screens (see `copy_menu_static`).
    pub(super) fn draw_menu_frame(&mut self, fb: &mut Framebuffer, dx: i32, dy: i32) {
        self.draw_menu_frame_art(fb, dx, dy);
        self.draw_menu_frame_text(fb);
    }

    /// Retail moon-landscape backdrop used by the Registry, Saved Games and
    /// Create screens (`glue/palnl/backgnd.dds`, 16:9 like the content box).
    pub(super) fn draw_moon_backdrop(&mut self, fb: &mut Framebuffer) {
        let (cx, _cy, cw, ch) = Self::content_rect(fb);
        // Scaled once per size: the 3840x2160 art used to be cloned and
        // resampled every frame.
        if self.moon_fit.as_ref().map(|f| (f.width, f.height)) != Some((cw, ch)) {
            let Some(glue) = self.glue.as_ref() else {
                self.draw_space_scene(fb);
                return;
            };
            let Some(art) = glue.palnl_backdrop.as_ref() else {
                self.draw_space_scene(fb);
                return;
            };
            let mut fit = Framebuffer::new(cw.max(1) as usize, ch.max(1) as usize);
            art.draw_scaled(&mut fit, 0, 0, cw, ch);
            self.moon_fit = Some(fit);
        }
        if let Some(fit) = &self.moon_fit {
            copy_into(fb, fit, cx, 0);
        }
    }

    /// Single Player Registry (`rez/glulogin.ui.json`), see
    /// [`Self::draw_glue_list`].  Reference 130734.
    pub(super) fn draw_registry(&mut self, fb: &mut Framebuffer) {
        let names = self.profile_store.names();
        let first = registry_top(self.profile_sel);
        let rows: Vec<(String, bool)> = names
            .iter()
            .enumerate()
            .skip(first)
            .take(REGISTRY_ROWS)
            .map(|(row, name)| (name.clone(), row == self.profile_sel))
            .collect();
        let has_profile = !names.is_empty();
        self.draw_glue_list(
            fb,
            &galaxycraft::menu::LOGIN_DIALOG,
            |g| g.login_list.as_ref(),
            "Registry",
            &rows,
            &[
                ("New ID", true),
                ("Delete", has_profile),
                ("Ok", has_profile),
                ("Cancel", true),
            ],
        );
    }

    /// Classic glue list dialog (Registry, Load Saved Game), measured on
    /// 130734 / 130848:
    /// - the `pList` plate is 4x art drawn at `scale / 4` with its right
    ///   edge on the control's right edge (`RC_ANCHOR_TOPRIGHT`); the
    ///   `PalNl` Ok/Cancel plates sit at their measured places;
    /// - `RC_BIGGERTEXT` text (Registry title, every `FLCBTN`) is Eurostile
    ///   Extended and `RC_BIGTEXT` (Saved Games title, list rows) the regular
    ///   face, both 16 UI px with natural spacing (glyph widths matched to
    ///   1 px);
    /// - an enabled button shows its hotkey letter in #daff8c and the rest
    ///   in #5ced30, centred in the control; a disabled one is all #6a6a6a;
    /// - the title is #c6daff; rows are #5ced30, the selected one #daff8c.
    ///
    /// `buttons` lists `(label, enabled)` for New ID (when the dialog has
    /// it), Delete, Ok and Cancel, in that order.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_glue_list(
        &mut self,
        fb: &mut Framebuffer,
        dialog: &galaxycraft::menu::GlueListDialog,
        plate: fn(&galaxycraft::ui_art::GlueArt) -> Option<&galaxycraft::dds::DdsImage>,
        title: &str,
        rows: &[(String, bool)],
        buttons: &[(&str, bool)],
    ) {
        use galaxycraft::menu::{GLUE_LIST_ROW_PITCH, GLUE_LIST_TEXT_INSET, LabelAlign};
        self.draw_moon_backdrop(fb);
        let (cx, _cy, _cw, ch) = Self::content_rect(fb);
        let (scale, ox, oy) = self.ui_metrics(fb);
        let k = ch as f32 / 1440.0;
        let sx = |x: f32| cx + (x * k).round() as i32;
        let sy = |y: f32| (y * k).round() as i32;
        let sw = |w: f32| (w * k).round().max(1.0) as i32;
        let (list_dx, ok_dx, cancel_dy) = (
            -self.swish_offset(SWISH_LIST_UI),
            self.swish_offset(SWISH_OK_UI),
            self.swish_offset(SWISH_CANCEL_UI),
        );
        let show_text = self.swish_text_shown();
        let px = |ui: f32| (ui * scale).round() as i32;
        if let Some(glue) = self.glue.as_ref() {
            if let Some(art) = plate(glue) {
                let w = (art.width as f32 * scale / 4.0).round() as i32;
                let h = (art.height as f32 * scale / 4.0).round() as i32;
                let right = ox + ((dialog.plate.0 + dialog.plate.2) * scale).round() as i32;
                art.draw_scaled(
                    fb,
                    right - w + px(list_dx),
                    oy + (dialog.plate.1 * scale).round() as i32,
                    w,
                    h,
                );
            }
            if let Some(art) = glue.palnl_ok.as_ref() {
                art.draw_scaled(
                    fb,
                    sx(1627.0) + px(ok_dx),
                    sy(1006.0),
                    sw(1062.0),
                    sw(214.0),
                );
            }
            if let Some(art) = glue.palnl_cancel.as_ref() {
                art.draw_scaled(
                    fb,
                    sx(1734.0),
                    sy(1068.0) + px(cancel_dy),
                    sw(940.0),
                    sw(273.0),
                );
            }
        }
        if !show_text {
            return;
        }
        const TITLE: Rgb = Rgb::new(0xc6, 0xda, 0xff);
        const SELECTED: Rgb = Rgb::new(0xda, 0xff, 0x8c);
        const ROW: Rgb = Rgb::new(0x5c, 0xed, 0x30);
        let bigger = dialog.title_bigger;
        let width = self.hd_advance_ui(title, scale, bigger);
        self.draw_hd_text(
            fb,
            title,
            dialog.title.0,
            dialog.title.1 + GLUE_LIST_TEXT_INSET.1,
            LabelAlign::Left,
            width,
            16.0,
            scale,
            ox,
            oy,
            TITLE,
            bigger,
        );
        for (i, (text, selected)) in rows.iter().enumerate() {
            let width = self.hd_advance_ui(text, scale, false);
            self.draw_hd_text(
                fb,
                text,
                dialog.list.0 + GLUE_LIST_TEXT_INSET.0,
                dialog.list.1 + GLUE_LIST_TEXT_INSET.1 + GLUE_LIST_ROW_PITCH * i as f32,
                LabelAlign::Left,
                width,
                16.0,
                scale,
                ox,
                oy,
                if *selected { SELECTED } else { ROW },
                false,
            );
        }
        let rects = dialog
            .new_id
            .into_iter()
            .chain([dialog.delete, dialog.ok, dialog.cancel]);
        for ((label, enabled), rect) in buttons.iter().zip(rects) {
            self.draw_glue_button(fb, label, rect, *enabled);
        }
    }

    /// One glue `FLCBTN` label (`WS_HOTKEY`, `FB_HTEXTCENTER`,
    /// `FB_VTEXTCENTER`): Eurostile Extended 16 UI px centred in `rect`
    /// (640x480 UI), the hotkey letter #daff8c and the rest #5ced30, or all
    /// #6a6a6a when disabled (measured on 130734 / 130848 / 130858).
    pub(super) fn draw_glue_button(
        &mut self,
        fb: &mut Framebuffer,
        label: &str,
        rect: galaxycraft::menu::HitRect,
        enabled: bool,
    ) {
        use galaxycraft::menu::LabelAlign;
        const HOTKEY: Rgb = Rgb::new(0xda, 0xff, 0x8c);
        const LABEL: Rgb = Rgb::new(0x5c, 0xed, 0x30);
        const DISABLED: Rgb = Rgb::new(0x6a, 0x6a, 0x6a);
        let (scale, ox, oy) = self.ui_metrics(fb);
        let mut chars = label.chars();
        let Some(first) = chars.next() else {
            return;
        };
        let first = first.to_string();
        let rest = chars.as_str();
        let lead = self.hd_advance_ui(&first, scale, true);
        let tail = self.hd_advance_ui(rest, scale, true);
        let left = rect.0 + (rect.2 - lead - tail) / 2.0;
        // FB_VTEXTCENTER: cap top 6 UI px above the control's centre.
        let top = rect.1 + rect.3 / 2.0 - 6.0;
        let (lead_colour, rest_colour) = if enabled {
            (HOTKEY, LABEL)
        } else {
            (DISABLED, DISABLED)
        };
        for (text, x, w, colour) in [
            (first.as_str(), left, lead, lead_colour),
            (rest, left + lead, tail, rest_colour),
        ] {
            if text.is_empty() {
                continue;
            }
            self.draw_hd_text(
                fb,
                text,
                x,
                top,
                LabelAlign::Left,
                w,
                16.0,
                scale,
                ox,
                oy,
                colour,
                true,
            );
        }
    }

    /// Retail "Select Game Type" screen: a modal dialog drawn OVER the live
    /// main menu, not a screen of its own (verified on references 4 and 28).
    pub(super) fn draw_gametype(&mut self, fb: &mut Framebuffer) {
        // The title-frame panel is empty behind a modal (reference 28); only
        // the live main menu keeps the welcome copy.
        self.draw_menu_backdrop_live(fb);
        self.draw_gametype_modal(fb);
    }

    /// The dialog plate is `glue/palmm/retail_ex.dds`, which already carries
    /// the metal frame, the red hairline and both wordmarks. Only the text
    /// layers are drawn here.
    pub(super) fn draw_gametype_modal(&mut self, fb: &mut Framebuffer) {
        use galaxycraft::menu::LabelAlign;
        let (scale, ox, oy) = self.ui_metrics(fb);
        // The shipped `glugamemode.ui.json` dialog is 360x200 and the client
        // centres popups: `(640-360)/2 = (480-200)/2 = 140`.  The plate DDS is
        // authored at 4x that rectangle (1438x799), so it must be stretched to
        // the whole dialog: its own metal frame and red hairline then land at
        // the measured 757..1802 x 435..1002 pixels of the reference.
        let dialog = galaxycraft::menu::GAME_TYPE_DIALOG.scaled(scale, ox, oy);
        let art = self.glue.as_ref().and_then(|g| g.gametype.as_ref());
        match art {
            Some(art) => art.draw_scaled(fb, dialog.x + 1, dialog.y, dialog.w - 2, dialog.h - 1),
            None => {
                fb.rect(dialog.x, dialog.y, dialog.w, dialog.h, Rgb::new(4, 4, 6));
                fb.rect_outline(
                    dialog.x,
                    dialog.y,
                    dialog.w,
                    dialog.h,
                    Rgb::new(120, 120, 130),
                );
                fb.rect_outline(
                    dialog.x + 3,
                    dialog.y + 3,
                    dialog.w - 6,
                    dialog.h - 6,
                    Rgb::new(150, 25, 25),
                );
            }
        }
        // Copies of the two wordmark buttons and `Cancel` from
        // `glugamemode.ui.json` are at [26,38,190,56], [134,114,190,56] and
        // [8,160,120,29].  The text is bottom-aligned inside the button; the
        // top anchors and ink widths below were measured on reference #4.
        // Original/Expansion use the 16 px front-end face, Cancel 14 px.
        let d = galaxycraft::menu::GAME_TYPE_DIALOG;
        let top = |local_y: f32| d.y as f32 + local_y;
        let bold = Rgb::new(208, 255, 131);
        let green = Rgb::new(108, 227, 66);
        let grey = Rgb::new(205, 220, 255);
        let select_game_type = self.cef_text("gluSELECTTYPE", "Select Game Type");
        self.draw_hd_text(
            fb,
            &select_game_type,
            d.x as f32 + 180.0,
            top(15.3),
            LabelAlign::Center,
            117.0,
            16.0,
            scale,
            ox,
            oy,
            grey,
            false,
        );
        let selected = self.gametype_hover.unwrap_or(self.episode_sel.min(1));
        // The two wordmark buttons use the 14 px button face (the title uses
        // 16 px); their cap line is shifted up by the smaller ascent so the
        // measured ink top stays at y 649 / 880 in the reference.
        for (idx, label, cx, ty, width, font_ui) in [
            (0, "Original", 121.0, 76.3, 42.0, 14.0),
            (1, "Expansion", 229.0, 152.3, 54.3, 14.0),
        ] {
            self.draw_hd_text(
                fb,
                label,
                d.x as f32 + cx,
                top(ty),
                LabelAlign::Center,
                width,
                font_ui,
                scale,
                ox,
                oy,
                if idx == selected { bold } else { green },
                false,
            );
        }
        self.draw_hd_text(
            fb,
            "Cancel",
            d.x as f32 + 68.0,
            top(169.3),
            LabelAlign::Center,
            37.0,
            14.0,
            scale,
            ox,
            oy,
            bold,
            false,
        );
    }

    /// Retail episode select (references 6-12 and 29): black plate, the three
    /// animated race portraits over glowing purple rings, and the
    /// Load Saved / Play Custom / Load Replay / Cancel buttons.
    /// Ring centres and button rects are measured on screenshot #6 at
    /// 3440x1440 and expressed as fractions of the 2560x1440 content box.
    /// Retail episode select (`glucmpgn` / `gluexpcmpgn`, dialog
    /// `RaceSelection`): the two `palcs` backdrop halves anchored to the
    /// content-box edges, the console plates, three FLCBTN slots with their
    /// animated disk/portrait/mouse-over clips, the measured labels and the
    /// four command buttons drawn as text centred in their shipped rects.
    /// The Episode screen's static art: black box, backdrop halves and the
    /// console plates (see `draw_episode`).
    pub(super) fn draw_episode_static(
        &self,
        fb: &mut Framebuffer,
        scale: f32,
        ox: i32,
        plates: bool,
    ) {
        let (cx, cy, cw, ch) = Self::content_rect(fb);
        // The campaign screen is black between the shipped art pieces.
        for y in cy..cy + ch {
            fb.rect(cx, y, cw, 1, Rgb::new(0, 0, 0));
        }
        // Screen-pixel scale of the 2560x1440 reference measurements.
        let k = ch as f32 / 1440.0;
        let pw = |w: f32| (w * k).round().max(1.0) as i32;
        if let Some(glue) = &self.glue {
            // `palcs` backdrop: each 320x480 SD half ships as 1280x1920 HD
            // art drawn at 0.75 (960x1440) and anchored to its screen edge
            // (matched at 0.99 against capture 130744).
            if let Some(art) = &glue.campaign_backl {
                art.draw_scaled(fb, cx, cy, pw(960.0), ch);
            }
            if let Some(art) = &glue.campaign_backr {
                // The right half is anchored 3 px inside the content edge
                // (measured: best match at x=1597 of 2560, residual 0.37).
                let inset = (3.0 * k).round() as i32;
                art.draw_scaled(fb, cx + cw - pw(960.0) - inset, cy, pw(960.0), ch);
            }
        }
        if plates {
            self.draw_episode_plates(fb, scale, ox, 0, 0);
        }
    }

    /// The two console plates of the campaign list, `dl` / `dr` screen px
    /// left of / right of their place (the slide-in).
    pub(super) fn draw_episode_plates(
        &self,
        fb: &mut Framebuffer,
        scale: f32,
        ox: i32,
        dl: i32,
        dr: i32,
    ) {
        let (cx, cy, _, _) = Self::content_rect(fb);
        if let Some(glue) = &self.glue {
            // Console plates: the 4x art is drawn at scale/4 and anchored to
            // its shipped control rect (`glucmpgn` ids 1/3).  `pBotL` carries
            // `RC_ANCHOR_TOPRIGHT`, so its right edge sits on the control's
            // right edge (x = 320+3*436 - 1695 = -66; ORB affine measured
            // tx -66.0, ty 1054.8, scale 0.7507).  `pBotR` grows to the right
            // from x = 320+3*217 = 971 (measured tx 969.0).
            let art_k = scale / 4.0;
            if let Some(art) = &glue.campaign_botl {
                let w = (art.width as f32 * art_k).round() as i32;
                let h = (art.height as f32 * art_k).round() as i32;
                let x = cx + ox + (scale * 436.0).round() as i32 - w - dl;
                let y = cy + (scale * 352.0).round() as i32;
                art.draw_scaled(fb, x, y, w, h);
            }
            if let Some(art) = &glue.campaign_botr {
                let w = (art.width as f32 * art_k).round() as i32;
                let h = (art.height as f32 * art_k).round() as i32;
                let x = cx + ox + (scale * 217.0).round() as i32 + dr;
                let y = cy + (scale * 370.0).round() as i32;
                art.draw_scaled(fb, x, y, w, h);
            }
        }
    }

    pub(super) fn draw_episode(&mut self, fb: &mut Framebuffer) {
        use galaxycraft::menu::CampaignSlot;
        let (cx, cy, cw, ch) = Self::content_rect(fb);
        let (scale, oxf) = galaxycraft::menu::campaign_scale(cw, ch);
        let ox = oxf as i32;
        let expansion = self.episode_sel >= 3;
        let slots: &[CampaignSlot] = if expansion {
            &galaxycraft::menu::CAMPAIGN_X_SLOTS
        } else {
            &galaxycraft::menu::CAMPAIGN_SLOTS
        };
        // Static layer (black box, `palcs` halves, console plates), composed
        // once per window size and campaign set; only the clips and the text
        // are drawn every frame.
        let key = (fb.width, fb.height, expansion);
        let sliding = matches!(self.glue_swish, Some(GlueSwish::In { .. }));
        if sliding {
            // Plates sliding in over the bare backdrop; the portraits and
            // labels show once they are in place (441.35-442.1 s).
            if self.episode_bare.as_ref().map(|c| c.0) != Some(key) {
                let mut layer = Framebuffer::new(fb.width as usize, fb.height as usize);
                layer.clear(Rgb::new(0, 0, 0));
                self.draw_episode_static(&mut layer, scale, ox, false);
                self.episode_bare = Some((key, layer));
            }
            if let Some((_, layer)) = &self.episode_bare {
                fb.pixels.copy_from_slice(&layer.pixels);
            }
            let dl = (self.swish_offset(SWISH_EPISODE_LEFT_UI) * scale).round() as i32;
            let dr = (self.swish_offset(SWISH_EPISODE_RIGHT_UI) * scale).round() as i32;
            self.draw_episode_plates(fb, scale, ox, dl, dr);
            if !self.swish_text_shown() {
                return;
            }
        } else {
            if self.episode_static.as_ref().map(|c| c.0) != Some(key) {
                let mut layer = Framebuffer::new(fb.width as usize, fb.height as usize);
                layer.clear(Rgb::new(0, 0, 0));
                self.draw_episode_static(&mut layer, scale, ox, true);
                self.episode_static = Some((key, layer));
            }
            if let Some((_, layer)) = &self.episode_static {
                fb.pixels.copy_from_slice(&layer.pixels);
            }
        }
        // Which slot carries the keyboard selection (retail opens on the
        // episode chosen in `Select Game Type`).
        let selected_slot = if expansion {
            self.episode_sel.saturating_sub(3).min(2)
        } else {
            [1usize, 0, 2][self.episode_sel % 3]
        };
        self.ensure_campaign_layers(expansion);
        let hover = self.episode_hover;
        for (i, slot) in slots.iter().enumerate() {
            let rect =
                |r: (f32, f32, f32, f32)| galaxycraft::menu::campaign_rect(r, scale, oxf, cx, cy);
            let portrait = rect((
                slot.button.0 + slot.portrait.0,
                slot.button.1 + slot.portrait.1,
                slot.portrait.2,
                slot.portrait.3,
            ));
            let disk = rect((
                slot.button.0 + slot.disk.0,
                slot.button.1 + slot.disk.1,
                slot.disk.2,
                slot.disk.3,
            ));
            let on = rect((
                slot.button.0 + slot.on.0,
                slot.button.1 + slot.on.1,
                slot.on.2,
                slot.on.3,
            ));
            if let Some(layers) = &mut self.campaign_layers {
                let clips = &mut layers.slots[i];
                // Child order from the shipped dialog: disk, portrait, On.
                if let Some(clip) = clips.disk.as_mut()
                    && let Some(frame) = clip.frame()
                {
                    draw_poster_rgba(fb, frame, (disk.0, disk.1, disk.2, disk.3), 0.0);
                }
                if let Some(clip) = clips.portrait.as_mut()
                    && let Some(frame) = clip.frame()
                {
                    // `FLC_FADED` keeps the idle portrait at ~0.25 opacity;
                    // the hovered slot fades all the way in (measured gain
                    // 0.97 on 130750/130757/130804) and the `On` clip adds
                    // the glow.
                    let dim = if hover == Some(i) {
                        0.0
                    } else {
                        1.0 - galaxycraft::menu::CAMPAIGN_PORTRAIT_IDLE_ALPHA
                    };
                    draw_poster_rgba(
                        fb,
                        frame,
                        (portrait.0, portrait.1, portrait.2, portrait.3),
                        dim,
                    );
                }
                if hover == Some(i)
                    && let Some(clip) = clips.on.as_mut()
                    && let Some(frame) = clip.frame()
                {
                    draw_poster_rgba(fb, frame, (on.0, on.1, on.2, on.3), 0.0);
                }
            }
            // Labels (`t\x04T\x01erran\n\x02Episode I\x01`): both lines in
            // `RC_BIGGERTEXT`; race green with the bright hotkey cap (the
            // whole word bright on hover/selection), episode lavender.
            let (label_x, label_y) = slot.label;
            let hovered = hover == Some(i);
            let race_color = if hovered || selected_slot == i {
                GLUE_BRIGHT
            } else {
                GLUE_GREEN
            };
            // Advance-centred on the `TEXTBOX` (or button) like the
            // command buttons: 3.1 px left of the box centre on 130744.
            let _ = label_x;
            let center = 320.0 + slot.text_center() * 3.0 - 3.1;
            self.glue_label(
                fb,
                slot.race,
                center,
                label_y * 3.0,
                race_color,
                Some((0, GLUE_BRIGHT)),
                true,
            );
            self.glue_label(
                fb,
                slot.episode,
                center,
                (label_y + 21.0) * 3.0,
                GLUE_INFO,
                None,
                true,
            );
        }
        // Bottom command buttons: text centred in the shipped control rects,
        // the hotkey glyph bright (`l\x04L\x01oad Saved`, `rLoad
        // \x04R\x01eplay`, `uPlay C\x04u\x01stom`, `c\x04C\x01ancel`).
        for button in galaxycraft::menu::CAMPAIGN_BUTTONS {
            // Advance centred 3.1 px left of the rect centre and ink centred
            // 1.5 px above it: all four buttons of 130744 within 0.3 px.
            let center = 320.0 + (button.rect.0 + button.rect.2 / 2.0) * 3.0 - 3.1;
            let middle = (button.rect.1 + button.rect.3 / 2.0) * 3.0 - 1.5;
            self.glue_ink_centered(
                fb,
                button.label,
                center,
                middle,
                GLUE_BIGGER_PX,
                GLUE_GREEN,
                Some((button.hot, GLUE_BRIGHT)),
                GlueFace::Bigger,
            );
        }
        // Locked slot under the pointer: `<Race> Campaign\nincomplete`
        // (`gluCmpgn-8/9/10`, CTEXT 200x50 at x 10 or 440).
        let campaign_at =
            |i: usize| galaxycraft::campaign_progress::Campaign::of_slot(expansion, i);
        if let Some(i) = hover
            && let Some(caption) = self.progress().lock_caption(campaign_at(i))
        {
            let center = 320.0 + if i == 0 { 110.0 } else { 540.0 } * 3.0 - 3.0;
            for (line, text) in caption.split('\n').enumerate() {
                let w = self.glue_measure(text, GLUE_BIG_PX, GlueFace::Big);
                self.glue_text(
                    fb,
                    center - w / 2.0,
                    GLUE_CAPTION_BASE + line as f32 * GLUE_CAPTION_PITCH,
                    text,
                    GLUE_BIG_PX,
                    GLUE_INFO,
                    None,
                );
            }
        }
        if let Some(campaign) = self.campaign_prompt {
            self.draw_campaign_prompt(fb, campaign);
        }
    }

    /// `glupokcancel` over the episode screen (live capture): the
    /// `glue/palcs/pdpopup.dds` plate at (140,140) of the 640x480 dialog,
    /// the `strRACE_ERR_*` text wrapped in its 340 px box and the Ok /
    /// Cancel labels with their hotkey glyph bright.  Nothing is dimmed.
    pub(super) fn draw_campaign_prompt(
        &mut self,
        fb: &mut Framebuffer,
        campaign: galaxycraft::campaign_progress::Campaign,
    ) {
        let (cx, cy, _cw, ch) = Self::content_rect(fb);
        let k = ch as f32 / 1440.0;
        let (x, y, w, h) = galaxycraft::menu::PROMPT_RECT;
        let r = |v: f32| (v * 3.0 * k).round() as i32;
        if let Some(art) = self.glue.as_ref().and_then(|g| g.campaign_prompt.as_ref()) {
            art.draw_scaled(fb, cx + r(320.0 / 3.0 + x), cy + r(y), r(w), r(h));
        } else {
            fill_alpha_rect(fb, cx + r(320.0 / 3.0 + x), cy + r(y), r(w), r(h), 230);
        }
        let text = galaxycraft::campaign_progress::lock_prompt(campaign);
        let box_w = galaxycraft::menu::PROMPT_TEXT.2 * 3.0;
        let mut lines: Vec<String> = Vec::new();
        for para in text.split('\n') {
            let mut line = String::new();
            for word in para.split(' ').filter(|w| !w.is_empty()) {
                let candidate = if line.is_empty() {
                    word.to_string()
                } else {
                    format!("{line} {word}")
                };
                if !line.is_empty()
                    && self.glue_measure(&candidate, GLUE_BIG_PX, GlueFace::Big) > box_w
                {
                    lines.push(std::mem::take(&mut line));
                    line = word.to_string();
                } else {
                    line = candidate;
                }
            }
            lines.push(line);
        }
        for (i, line) in lines.iter().enumerate() {
            self.glue_text(
                fb,
                GLUE_PROMPT_X,
                GLUE_PROMPT_BASE + i as f32 * GLUE_PROMPT_PITCH,
                line,
                GLUE_BIG_PX,
                GLUE_INFO,
                None,
            );
        }
        for (label, rect) in ["Ok", "Cancel"]
            .into_iter()
            .zip(galaxycraft::menu::PROMPT_BUTTONS)
        {
            self.glue_ink_centered(
                fb,
                label,
                320.0 + (rect.0 + rect.2 / 2.0) * 3.0 + GLUE_SMALL_BUTTON_DX,
                (rect.1 + rect.3 / 2.0) * 3.0 + GLUE_SMALL_BUTTON_DY,
                GLUE_SMALL_BUTTON_PX,
                GLUE_GREEN,
                Some((0, GLUE_BRIGHT)),
                GlueFace::Small,
            );
        }
        let _ = cy;
    }

    /// Build the streamed campaign clips for the active campaign set.  The
    /// clips are the shipped `glue/campaign` WebMs consumed at the same
    /// 10.5 fps the retail glue player uses for every front-end loop.
    pub(super) fn ensure_campaign_layers(&mut self, expansion: bool) {
        if self
            .campaign_layers
            .as_ref()
            .is_some_and(|layers| layers.expansion == expansion)
        {
            return;
        }
        let dir = if expansion { "expcampaign" } else { "campaign" };
        let slots: &[galaxycraft::menu::CampaignSlot] = if expansion {
            &galaxycraft::menu::CAMPAIGN_X_SLOTS
        } else {
            &galaxycraft::menu::CAMPAIGN_SLOTS
        };
        let root = self.assets_root.clone();
        let frozen = self.menu_at.is_some() || !self.menu_phases.is_empty();
        let at = self.menu_at.unwrap_or(0.0);
        let open = |stem: &str| {
            let path = root.join(format!("videos/glue/{dir}/{stem}.webm"));
            let start = self.menu_phases.get(stem).copied().unwrap_or(at);
            MenuClip::open_at(path, start, frozen)
        };
        let mut layers = CampaignLayers {
            expansion,
            ..CampaignLayers::default()
        };
        for (i, slot) in slots.iter().enumerate() {
            layers.slots[i] = CampaignSlotClips {
                portrait: open(slot.art),
                disk: open(&format!("disk{}", slot.art)),
                on: open(&format!("{}on", slot.art)),
            };
        }
        self.campaign_layers = Some(layers);
    }

    /// Glue text in 2560x1440 content units through the outline rasteriser
    /// (`UiFont::draw_cef`, Eurostile regular): pen `x`, baseline `base`,
    /// em size `ppem`; `hot` paints one glyph in another colour (the
    /// `\x04..\x01` hotkey span of the shipped strings).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn glue_text(
        &mut self,
        fb: &mut Framebuffer,
        x: f32,
        base: f32,
        text: &str,
        ppem: f32,
        color: Rgb,
        hot: Option<(usize, Rgb)>,
    ) -> f32 {
        self.glue_text_face(fb, x, base, text, ppem, color, hot, GlueFace::Big)
    }

    /// [`Self::glue_text`] in any glue face.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn glue_text_face(
        &mut self,
        fb: &mut Framebuffer,
        x: f32,
        base: f32,
        text: &str,
        ppem: f32,
        color: Rgb,
        hot: Option<(usize, Rgb)>,
        face: GlueFace,
    ) -> f32 {
        let (cx, _cy, _cw, ch) = Self::content_rect(fb);
        let k = ch as f32 / 1440.0;
        let font = match face {
            GlueFace::Hud => self
                .hud_font
                .as_mut()
                .or(self.glue_font_small.as_mut())
                .or(self.ui_font.as_mut()),
            GlueFace::Small => self.glue_font_small.as_mut().or(self.ui_font.as_mut()),
            GlueFace::Big => self.glue_font.as_mut().or(self.ui_font.as_mut()),
            GlueFace::Bigger => self.glue_font_ext.as_mut().or(self.ui_font_hover.as_mut()),
        };
        let Some(font) = font else {
            return x;
        };
        let mut pen = cx as f32 + x * k;
        let base = base * k;
        let px = ppem * k;
        let tr = px * GLUE_TRACKING;
        let mut prev: Option<char> = None;
        for (i, ch_) in text.char_indices() {
            if let Some(p) = prev {
                pen += font.kerning(p, ch_, px);
            }
            prev = Some(ch_);
            let c = match hot {
                Some((h, hc)) if text[..i].chars().count() == h => hc,
                _ => color,
            };
            let glyph = &text[i..i + ch_.len_utf8()];
            if face == GlueFace::Hud {
                pen = font.draw_cef(fb, pen, base, glyph, px, 0.0, c, c);
            } else {
                pen = font.draw_cef(fb, pen, base, glyph, px, 0.0, c, c) + tr;
            }
        }
        (pen - cx as f32) / k
    }

    /// Width of glue text in content units.
    pub(super) fn glue_measure(&self, text: &str, ppem: f32, face: GlueFace) -> f32 {
        let Some(font) = self.glue_face(face) else {
            return 0.0;
        };
        let chars: Vec<char> = text.chars().collect();
        let kern: f32 = chars
            .windows(2)
            .map(|w| font.kerning(w[0], w[1], ppem))
            .sum();
        if face == GlueFace::Hud {
            return font.measure_cef(text, ppem, 0.0) + kern;
        }
        font.measure_cef(text, ppem, ppem * GLUE_TRACKING) + kern
    }

    pub(super) fn glue_face(&self, face: GlueFace) -> Option<&galaxycraft::text::UiFont> {
        match face {
            GlueFace::Hud => self
                .hud_font
                .as_ref()
                .or(self.glue_font_small.as_ref())
                .or(self.ui_font.as_ref()),
            GlueFace::Small => self.glue_font_small.as_ref().or(self.ui_font.as_ref()),
            GlueFace::Big => self.glue_font.as_ref().or(self.ui_font.as_ref()),
            GlueFace::Bigger => self.glue_font_ext.as_ref().or(self.ui_font_hover.as_ref()),
        }
    }

    /// Glue button label (`FB_HTEXTCENTER | FB_VTEXTCENTER`): the advance
    /// width is centred on `cx` and the ink box on `cy` (content px), which
    /// is how descenders lift `Play Custom` / `Load Replay` on 130744.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn glue_ink_centered(
        &mut self,
        fb: &mut Framebuffer,
        text: &str,
        cx: f32,
        cy: f32,
        px: f32,
        color: Rgb,
        hot: Option<(usize, Rgb)>,
        face: GlueFace,
    ) {
        let Some((x0, x1, top, bottom)) = self
            .glue_face(face)
            .and_then(|f| f.ink_bounds_cef(text, px, px * GLUE_TRACKING))
        else {
            return;
        };
        let _ = (x0, x1);
        let advance = self.glue_measure(text, px, face);
        self.glue_text_face(
            fb,
            cx - advance / 2.0,
            cy - (top + bottom) / 2.0,
            text,
            px,
            color,
            hot,
            face,
        );
    }

    /// Glue label whose advance is centred on `center` with its cap line at
    /// `cap_top` (content px): the race / episode lines of the slots.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn glue_label(
        &mut self,
        fb: &mut Framebuffer,
        text: &str,
        center: f32,
        cap_top: f32,
        color: Rgb,
        hot: Option<(usize, Rgb)>,
        extended: bool,
    ) {
        let (px, face) = if extended {
            (GLUE_BIGGER_PX, GlueFace::Bigger)
        } else {
            (GLUE_BIG_PX, GlueFace::Big)
        };
        let advance = self.glue_measure(text, px, face);
        self.glue_text_face(
            fb,
            center - advance / 2.0,
            cap_top + px * GLUE_CAP,
            text,
            px,
            color,
            hot,
            face,
        );
    }

    /// Retail `Select starting mission` modal (`glue/campaign/phist.dds`):
    /// the shipped plate already carries the metal frame, the list box and
    /// the Ok/Cancel buttons; only the title and the mission rows are text.
    pub(super) fn draw_mission(&mut self, fb: &mut Framebuffer) {
        // Episode select stays behind the modal (reference 131132): the
        // retail client does not dim it (measured gain 1.000).
        self.draw_episode(fb);
        let (cx, cy, _cw, ch) = Self::content_rect(fb);
        let k = ch as f32 / 1440.0;
        let px = |x: f32| cx + (x * k).round() as i32;
        let py = |y: f32| cy + (y * k).round() as i32;
        let pw = |w: f32| (w * k).round().max(1.0) as i32;
        let mut has_plate = false;
        // The mission-history popup is centred in the content box
        // (measured (708,342) for the 1140x753 plate at 2560x1440).
        let popup_w = pw(1140.0);
        let popup_h = pw(753.0);
        let popup_x = px(708.0);
        let popup_y = py(342.0);
        if let Some(glue) = &self.glue
            && let Some(art) = &glue.campaign_hist
        {
            art.draw_scaled(fb, popup_x, popup_y, popup_w, popup_h);
            has_plate = true;
        }
        if !has_plate {
            fb.rect(
                px(708.0),
                py(342.0),
                pw(1140.0),
                pw(753.0),
                Rgb::new(8, 6, 6),
            );
            fb.rect_outline(px(708.0), py(342.0), pw(1140.0), pw(753.0), CSS_INNER);
        }
        // Title (`GLUHIST-1`, LTEXT `RC_SMALLTEXT`): BlizzardGlobal in the
        // info colour, fitted on 131132 (pen 744.8, baseline 409.5).
        self.glue_text_face(
            fb,
            744.8,
            409.5,
            "Select starting mission:",
            GLUE_SMALL_PX,
            GLUE_INFO,
            None,
            GlueFace::Small,
        );
        // Rows: the profile's history (`gluhist`), 8 visible from the
        // scroll position; the selected entry is the bright glue green.
        let campaign = match self.menu_screen {
            MenuScreen::Mission { episode } => {
                galaxycraft::campaign_progress::Campaign::from_episode(episode)
            }
            _ => None,
        }
        .unwrap_or(galaxycraft::campaign_progress::Campaign::Terran);
        let history = self.progress().history(campaign);
        let list = self.mission_list;
        for row in 0..galaxycraft::campaign_progress::MissionList::ROWS {
            let idx = list.top + row;
            let Some(entry) = history.get(idx) else {
                break;
            };
            let color = if idx == list.sel {
                Rgb::new(208, 255, 131)
            } else {
                Rgb::new(102, 227, 53)
            };
            self.glue_text(
                fb,
                GLUE_ROW_X,
                GLUE_ROW_BASE + row as f32 * 57.0,
                entry.label,
                GLUE_BIG_PX,
                color,
                None,
            );
        }
        draw_mission_scrollbar(fb, cx, k, list.thumb());
        // Ok/Cancel (`o\x04O\x01k`, `c\x04C\x01ancel`): plain FLCBTN labels
        // centred in the `gluhist` rects (65,210) / (195,210), 123x29, of
        // the 380x251 dialog at (708,342).
        for (label, bx) in [("Ok", 65.0f32), ("Cancel", 195.0)] {
            self.glue_ink_centered(
                fb,
                label,
                708.0 + (bx + 61.5) * 3.0 + GLUE_SMALL_BUTTON_DX,
                342.0 + (210.0 + 14.5) * 3.0 + GLUE_SMALL_BUTTON_DY,
                GLUE_SMALL_BUTTON_PX,
                GLUE_GREEN,
                Some((0, GLUE_BRIGHT)),
                GlueFace::Small,
            );
        }
    }

    /// The MENU button in the HD console, measured on the references:
    /// x 0.7363..0.8035, y 0.8194..0.8542 of the content box (the classic
    /// Stat_F10 rect sits ~340 px to the left).
    /// `stat_f10` button 1 (MENU), right-anchored.
    pub(super) fn menu_button_rect(&self) -> UiRect {
        self.hud_control("stat_f10", 1, UiRect::new(416, 388, 64, 20))
    }
}
