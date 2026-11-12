//! `impl App`: options (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    /// The Options "Restore to Default" confirmation (screenshot
    /// 2026-11-06 15:46:59, 3440x1440): the whole menu and Options plate dim
    /// behind the shared dialog (744..1820 x 490..950 content px) titled
    /// "Reset <tab> settings?".
    pub(super) fn draw_options_reset(&mut self, fb: &mut Framebuffer, tab: OptionsTab) {
        let (cx, _cy, cw, ch) = Self::content_rect(fb);
        fill_alpha_rect(fb, cx, 0, cw, ch, 153);
        let key = match tab {
            OptionsTab::Speed => "glue_optionsPanel_reset_speed_settings",
            OptionsTab::Sound => "glue_optionsPanel_reset_sound_settings",
            OptionsTab::Video => "glue_optionsPanel_reset_video_settings",
            OptionsTab::Game => "glue_optionsPanel_reset_game_settings",
            OptionsTab::Network => "glue_optionsPanel_reset_network_settings",
        };
        let title = self.cef_text(key, &tab.reset_title());
        self.draw_confirm_dialog(fb, &title, (744.0, 490.0, 1076.0, 460.0), true);
    }

    /// The CEF "do you wish to continue?" dialog shared by Hotkeys
    /// ("Restore defaults?", reference 120018) and Options ("Reset <tab>
    /// settings?", screenshot 2026-11-06 15:46:59): opaque black rounded box
    /// at `rect` (2560x1440 content px) with the double red border, the
    /// lavender title, the warning line, Continue / Cancel at y 813, and
    /// for Options a close cross above the top-right corner.
    pub(super) fn draw_confirm_dialog(
        &mut self,
        fb: &mut Framebuffer,
        title: &str,
        rect: (f32, f32, f32, f32),
        close_cross: bool,
    ) {
        let (cx, _cy, cw, ch) = Self::content_rect(fb);
        let dx = |p: f32| (cw as f32 * p / 2560.0).round() as i32;
        let dy = |p: f32| (ch as f32 * p / 1440.0).round() as i32;
        let sx = |p: f32| cx + dx(p);
        let sy = |p: f32| dy(p);
        let fpx = |p: f32| p * ch as f32 / 1440.0;
        let border = dx(3.0);
        let dlg = (sx(rect.0), sy(rect.1), dx(rect.2), dy(rect.3));
        let radius = dx(16.0) as f32;
        rounded_fill(fb, dlg.0, dlg.1, dlg.2, dlg.3, radius, Rgb::new(0, 0, 0));
        rounded_band(fb, dlg.0, dlg.1, dlg.2, dlg.3, radius, 0, border, CSS_OUTER);
        rounded_band(
            fb,
            dlg.0,
            dlg.1,
            dlg.2,
            dlg.3,
            radius,
            border * 2,
            border,
            CSS_INNER,
        );
        if close_cross {
            // Two 3 px strokes, 22 px across, centred at (1840, 472).
            let (xc, yc, r) = (sx(1840.0), sy(472.0), dx(11.0));
            for t in -r..=r {
                for w in 0..dx(3.0).max(1) {
                    fb.put(xc + t + w - 1, yc + t, Rgb::new(220, 220, 220));
                    fb.put(xc + t + w - 1, yc - t, Rgb::new(220, 220, 220));
                }
            }
        }
        if let Some(font) = self.ui_font_hover.as_mut() {
            font.draw_top_stretched(
                fb,
                sx(826.0),
                sy(572.0),
                title,
                fpx(36.0),
                0.0,
                1.0,
                1.083,
                CSS_LAVENDER,
                None,
                1.0,
            );
        }
        let discard = self.cef_text(
            "glue_hotkeysPanel_restoreDefaultsExplanation",
            "Discard your unsaved changes?",
        );
        if let Some(font) = self.ui_font.as_mut() {
            font.draw_top(
                fb,
                sx(826.0),
                sy(628.0),
                &discard,
                fpx(27.0),
                0.23,
                Rgb::new(221, 221, 221),
                None,
            );
        }
        let continue_label = self.cef_text("glue_hotkeysPanel_continue_button", "Continue");
        let cancel_label = self.cef_text("glue_hotkeysPanel_cancel_button", "Cancel");
        for (label, x0) in [
            (continue_label.as_str(), 996.0f32),
            (cancel_label.as_str(), 1387.0),
        ] {
            let bx = sx(x0);
            let by = sy(813.0);
            let bw = dx(360.0);
            let bh = dy(66.0);
            let br = dx(6.0) as f32;
            rounded_fill(fb, bx, by, bw, bh, br, Rgb::new(0, 0, 0));
            rounded_band(fb, bx, by, bw, bh, br, 0, border, CSS_INNER);
            if let Some(font) = self.ui_font_hover.as_mut() {
                font.draw_centered_stretched(
                    fb,
                    bx + bw / 2,
                    by + dy(21.0),
                    label,
                    fpx(30.0),
                    0.0,
                    1.0,
                    1.1,
                    CSS_GREEN,
                    None,
                    1.0,
                );
            }
        }
    }

    pub(super) fn draw_options(&mut self, fb: &mut Framebuffer) {
        // Over the game the frozen game frame stays underneath (138 s).
        if !self.ingame_options {
            self.draw_menu_backdrop_live(fb);
        }
        let tab = match &self.menu_screen {
            MenuScreen::Options { tab } => *tab,
            _ => OptionsTab::Speed,
        };
        let (cx, _cy, cw, ch) = Self::content_rect(fb);
        // The retail CEF modal applies `rgba(0,0,0,.6)` to the whole content
        // box.  Measured against the five references: the background outside
        // the plate is exactly 0.4164x the undimmed menu (alpha 149), and the
        // plate interior follows once the same dim is used underneath it.
        dim_screen(fb, cx, cw, ch, 149);
        let unit = ch as f32 / 1440.0;
        let dx = |p: f32| cx + (cw as f32 * p / 2560.0).round() as i32;
        let dy = |p: f32| (ch as f32 * p / 1440.0).round() as i32;
        let px = |p: f32| p * unit;
        let white = Rgb::new(255, 255, 255);
        let dim_text = Rgb::new(150, 150, 155);
        let green = CSS_GREEN;
        let dxw = |p: f32| (cw as f32 * p / 2560.0).round() as i32;
        let fx = |p: f32| cx as f32 + cw as f32 * p / 2560.0;
        let fy = |p: f32| ch as f32 * p / 1440.0;

        // Plate and tab boxes (see `draw_options_plate` for the measured
        // geometry).  Tab boxes sit on a 93 px pitch.
        let rows = [345.0f32, 438.0, 531.0, 624.0, 717.0];
        let active_i = OptionsTab::all()
            .iter()
            .position(|t| *t == tab)
            .unwrap_or(0);
        let space = CefSpace {
            cx: cx as f32,
            kx: cw as f32 / 2560.0,
            ky: ch as f32 / 1440.0,
        };
        // The plate is static per tab: composed once over black and over
        // white, then applied per channel as `dst * (W - B) / 255 + B`.
        let key = (fb.width, fb.height, active_i);
        if self.options_plate.as_ref().map(|c| c.0) != Some(key) {
            let mut over_black = Framebuffer::new(fb.width as usize, fb.height as usize);
            over_black.clear(Rgb::new(0, 0, 0));
            draw_options_plate(&mut over_black, space, &rows, active_i);
            let mut over_white = Framebuffer::new(fb.width as usize, fb.height as usize);
            over_white.clear(Rgb::new(255, 255, 255));
            draw_options_plate(&mut over_white, space, &rows, active_i);
            self.options_plate = Some((key, LayerCache::new(&over_black, &over_white)));
        }
        if let Some((_, layer)) = &self.options_plate {
            layer.apply(fb);
        }
        // Extended-face text, fitted with the outline rasteriser on 130933:
        // title 41.5 px/em at pen 581.5 / baseline 267.06; tab labels
        // 30.05 px/em centred on x 680.0, baseline row + 69.
        // The CEF Options panel is a separate string set (`gluall.json`)
        // from the classic glue labels (`rc3.json`): title, tabs, row
        // labels, slider endpoints and button captions.
        let options_title = self.cef_text(
            "glue_optionsPanel_general_title",
            &self.ui_text("gluMain-5", "Options"),
        );
        let tab_keys = [
            "glue_optionsPanel_tabs_speed",
            "glue_optionsPanel_tabs_sound",
            "glue_optionsPanel_tabs_video",
            "glue_optionsPanel_tabs_game",
            "glue_optionsPanel_tabs_network",
        ];
        let tab_labels: Vec<String> = OptionsTab::all()
            .iter()
            .enumerate()
            .map(|(i, t)| self.cef_text(tab_keys[i], &self.ui_text("options-2", t.name())))
            .collect();
        let slow = self.cef_text("glue_optionsPanel_general_label_slow", "slow");
        let fast = self.cef_text("glue_optionsPanel_general_label_fast", "fast");
        let min = self.cef_text("glue_optionsPanel_general_label_min", "min");
        let max = self.cef_text("glue_optionsPanel_general_label_max", "max");
        let game_speed = self.cef_text("glue_optionsPanel_speed_label_gameSpeed", "Game Speed");
        let key_scroll = self.cef_text(
            "glue_optionsPanel_speed_label_keyScroll",
            "Key Scroll Speed",
        );
        let mouse_scroll = self.cef_text(
            "glue_optionsPanel_speed_label_mouseScroll",
            "Mouse Scroll Speed",
        );
        let mouse_sensitivity = self.cef_text(
            "glue_optionsPanel_speed_label_mouseSensitivity",
            "Mouse Sensitivity",
        );
        let mouse_scaling = self.cef_text(
            "glue_optionsPanel_speed_label_mouseScaling",
            "Use Mouse Scaling",
        );
        let music_volume =
            self.cef_text("glue_optionsPanel_sound_label_musicVolume", "Music Volume");
        let digital_volume = self.cef_text(
            "glue_optionsPanel_sound_label_digitalVolume",
            "Digital Volume",
        );
        let unit_speech = self.cef_text("glue_optionsPanel_sound_label_unitSpeech", "Unit Speech");
        let unit_ack = self.cef_text(
            "glue_optionsPanel_sound_label_unitAck",
            "Unit Acknowledgments",
        );
        let sound_background = self.cef_text(
            "glue_optionsPanel_sound_label_soundPlaysInBackground",
            "Sound Plays While In Background",
        );
        let building_sounds = self.cef_text(
            "glue_optionsPanel_sound_label_buildingSounds",
            "Building Sounds",
        );
        let game_subtitles = self.cef_text(
            "glue_optionsPanel_sound_label_gameSubtitles",
            "Game Subtitles",
        );
        let cinematic_subtitles = self.cef_text(
            "glue_optionsPanel_sound_label_cinematicSubtitles",
            "Cinematic Subtitles",
        );
        let original_vo = self.cef_text(
            "glue_optionsPanel_sound_label_originalUnitVO",
            "Original Unit Voice Overs",
        );
        let original_campaign = self.cef_text(
            "glue_optionsPanel_sound_label_originalCampaignVO",
            "Original 1998 Campaign",
        );
        let display_mode =
            self.cef_text("glue_optionsPanel_video_label_windowMode", "Display Mode");
        let monitor = self.cef_text("glue_optionsPanel_video_label_monitor", "Monitor");
        let gamma = self.cef_text("glue_optionsPanel_video_label_gamma", "Gamma Correction");
        let fps_cap = self.cef_text("glue_optionsPanel_video_label_fpsCap", "Enable FPS Cap");
        let sd_filter = self.cef_text("optionsPanel_video_Filtering_label", "SD Graphics Filter");
        let sharp = self.cef_text("optionsPanel_video_Sharp_label", "Sharp");
        let filtered = self.cef_text("optionsPanel_video_Filtered_label", "Filtered");
        let default_label = self.cef_text("optionsPanel_game_console_default_label", "Default");
        let vsync = self.cef_text(
            "glue_optionsPanel_video_label_vsync",
            "Enable Vertical Sync",
        );
        let hd_graphics = self.cef_text("glue_optionsPanel_video_label_hd", "HD Graphics");
        let environment = self.cef_text(
            "glue_optionsPanel_video_label_environmentEffects",
            "Environment Effects",
        );
        let real_time = self.cef_text(
            "glue_optionsPanel_video_label_realTimeLighting",
            "Real-Time Lighting",
        );
        let smooth_turning = self.cef_text(
            "glue_optionsPanel_video_label_smoothAnimations",
            "Smooth Unit Turning",
        );
        let shadow_stack = self.cef_text(
            "glue_optionsPanel_video_label_shadowStacking",
            "Shadow Stacking",
        );
        let pillarbox = self.cef_text(
            "glue_optionsPanel_video_label_aspect_ratio_letterbox",
            "Pillarbox",
        );
        let display_modes: Vec<String> = [
            (
                "glue_optionsPanel_video_label_windowMode_windowed",
                DISPLAY_MODES[0],
            ),
            (
                "glue_optionsPanel_video_label_windowMode_windowedFullscreen",
                DISPLAY_MODES[1],
            ),
            (
                "glue_optionsPanel_video_label_windowMode_fullscreen",
                DISPLAY_MODES[2],
            ),
        ]
        .iter()
        .map(|(key, fallback)| self.cef_text(key, fallback))
        .collect();
        let language_names: Vec<String> = [
            ("enUS", "English"),
            ("frFR", "Français"),
            ("itIT", "Italiano"),
            ("deDE", "Deutsch"),
            ("esES", "Español - España"),
            ("esMX", "Español - Latino"),
            ("ptBR", "Português"),
            ("zhCN", "简体中文"),
            ("zhTW", "繁體中文"),
            ("koKR", "한국어"),
            ("jaJP", "日本語"),
            ("plPL", "Polski"),
            ("ruRU", "Русский"),
        ]
        .iter()
        .map(|(tag, fallback)| {
            self.cef_text(&format!("glue_optionsPanel_language_{tag}"), fallback)
        })
        .collect();
        let gameplay = self.cef_text("glue_optionsPanel_game_subtitle_gameplay", "Gameplay");
        let game_timer = self.cef_text("glue_optionsPanel_game_label_gameTimer", "Game Timer");
        let color_cycling = self.cef_text(
            "glue_optionsPanel_game_label_colorCycling",
            "Enable Color Cycling",
        );
        let language_filter = self.cef_text(
            "glue_optionsPanel_game_label_languageFilter",
            "Mature Language Filter",
        );
        let bonus_skins = self.cef_text(
            "glue_optionsPanel_game_label_bonusSkins",
            "Show Bonus Skins",
        );
        let console_label = self.cef_text(
            "glue_optionsPanel_game_label_consolePosition",
            "Mini-Map Position",
        );
        let portraits = self.cef_text("glue_optionsPanel_game_label_portraits", "Portraits");
        let animated = self.cef_text(
            "glue_optionsPanel_game_label_unitPortraits_animated",
            "Animated",
        );
        let still = self.cef_text("glue_optionsPanel_game_label_unitPortraits_still", "Still");
        let disabled = self.cef_text(
            "glue_optionsPanel_game_label_unitPortraits_disable",
            "Disabled",
        );
        let left_corner = self.cef_text(
            "glue_optionsPanel_game_label_consolePosition_leftCorner",
            "Left Corner",
        );
        let standard = self.cef_text(
            "glue_optionsPanel_game_label_consolePosition_standard",
            "Standard",
        );
        let friends = "Friends presence text notifications";
        let language = self.cef_text("glue_optionsPanel_game_subtitle_language", "Language");
        let language_description = self.cef_text(
            "glue_optionsPanel_game_language_description",
            "Changing the language requires a game restart.",
        );
        let language_lines: Vec<String> = {
            let words: Vec<&str> = language_description.split_whitespace().collect();
            let half = words.len().div_ceil(2);
            let (a, b) = words.split_at(half.min(words.len()));
            vec![a.join(" "), b.join(" ")]
        };
        let apm_tracking = self.cef_text(
            "glue_optionsPanel_game_subtitle_apmTracking",
            "Actions Per Minute Tracking",
        );
        let display_apm = self.cef_text("glue_optionsPanel_game_label_apm", "Display APM In Game");
        let apm_alert = self.cef_text(
            "glue_optionsPanel_game_label_epm",
            "Alert when APM falls below",
        );
        let color_text = self.cef_text("glue_optionsPanel_game_label_colorText", "Color Text");
        let play_sound = self.cef_text("glue_optionsPanel_game_label_playSound", "Play Sound");
        let prefer_port = self.cef_text(
            "glue_optionsPanel_network_label_preferPort6112",
            "Prefer port 6112",
        );
        let latency = self.cef_text(
            "glue_optionsPanel_network_subtitle_defaultLatency",
            "Latency",
        );
        let low_latency = self.cef_text("glue_optionsPanel_network_label_lowLat", "Low");
        let high_latency = self.cef_text("glue_optionsPanel_network_label_highLat", "High");
        let extra_high_latency =
            self.cef_text("glue_optionsPanel_network_label_extraHighLat", "Extra High");
        let network_description = self.cef_text(
            "glue_optionsPanel_network_description",
            &galaxycraft::menu::NETWORK_LATENCY_TEXT.join(" "),
        );
        let network_lines: Vec<String> = {
            let words: Vec<&str> = network_description.split_whitespace().collect();
            let per = words.len().div_ceil(3).max(1);
            words.chunks(per).map(|chunk| chunk.join(" ")).collect()
        };
        let restore_default =
            self.cef_text("glue_optionsPanel_general_restore", "Restore to Default");
        if let Some(font) = self.ui_font_hover.as_mut() {
            font.draw_cef(
                fb,
                fx(581.5),
                fy(267.06),
                &options_title,
                px(41.5),
                0.0,
                white,
                white,
            );
            for (i, label) in tab_labels.iter().enumerate() {
                let active = OptionsTab::all()[i] == tab;
                // The first letter is #b4fc74 on every tab
                // (`::first-letter`); the selected tab is #b4fc74 throughout.
                let w = font.measure_cef(label, px(30.05), 0.0);
                font.draw_cef(
                    fb,
                    fx(680.0) - w / 2.0,
                    fy(rows[i] + 69.0),
                    label,
                    px(30.05),
                    0.0,
                    CSS_GREEN_HI,
                    if active { CSS_GREEN_HI } else { green },
                );
            }
        }

        let left = dx(900.0);
        let right = dx(1456.0);
        let col_w = dxw(516.0);
        let right_end = right + col_w;

        // ---- Speed -----------------------------------------------------
        if tab == OptionsTab::Speed {
            opt_text(
                fb,
                &mut self.ui_font,
                dx(900.0),
                dy(346.0),
                &game_speed,
                px(23.5),
                white,
            );
            // Seven retail game speeds; the shipped default is Fastest.
            let gs = ((self.game_options.game_speed.clamp(0.5, 1.0) - 0.5) / 0.5).clamp(0.0, 1.0);
            opt_slider(
                fb,
                &mut self.ui_font,
                left,
                left + col_w,
                dy(391.0),
                gs,
                &slow,
                &fast,
                dy(426.0),
                unit,
                px(20.0),
            );
            opt_text(
                fb,
                &mut self.ui_font,
                dx(900.0),
                dy(499.0),
                &key_scroll,
                px(23.5),
                white,
            );
            opt_slider(
                fb,
                &mut self.ui_font,
                left,
                left + col_w,
                dy(544.0),
                self.game_options.key_scroll.clamp(0.0, 1.0),
                &slow,
                &fast,
                dy(579.0),
                unit,
                px(20.0),
            );
            opt_text(
                fb,
                &mut self.ui_font,
                dx(900.0),
                dy(652.0),
                &mouse_scroll,
                px(23.5),
                white,
            );
            opt_slider(
                fb,
                &mut self.ui_font,
                left,
                left + col_w,
                dy(697.0),
                self.game_options.mouse_scroll.clamp(0.0, 1.0),
                &slow,
                &fast,
                dy(732.0),
                unit,
                px(20.0),
            );

            opt_checkbox(
                fb,
                &mut self.ui_font,
                right,
                dy(341.0),
                dxw(29.0),
                self.game_options.use_mouse_sensitivity,
                true,
                &mouse_sensitivity,
                true,
                px(24.0),
            );
            opt_slider(
                fb,
                &mut self.ui_font,
                right,
                right_end,
                dy(394.0),
                self.game_options.mouse_sensitivity.clamp(0.0, 1.0),
                &slow,
                &fast,
                dy(429.0),
                unit,
                px(20.0),
            );
            opt_text_right(
                fb,
                &mut self.ui_font,
                right_end,
                dy(349.0),
                &format!(
                    "{}%",
                    (self.game_options.mouse_sensitivity.clamp(0.0, 1.0) * 100.0).round() as i32
                ),
                px(24.0),
                white,
            );
            let hardware_cursor =
                self.cef_text("optionsPanel_speed_hardwareCursor_label", "Hardware Cursor");
            let use_mouse_confine =
                self.cef_text("optionsPanel_speed_mouseConfine_label", "Use Mouse Confine");
            for (i, (label, checked, enabled)) in [
                (
                    mouse_scaling.as_str(),
                    self.game_options.mouse_scaling,
                    true,
                ),
                (
                    hardware_cursor.as_str(),
                    self.game_options.hardware_cursor,
                    true,
                ),
                (use_mouse_confine.as_str(), true, false),
            ]
            .iter()
            .enumerate()
            {
                opt_checkbox(
                    fb,
                    &mut self.ui_font,
                    right,
                    dy(497.0 + 54.0 * i as f32),
                    dxw(29.0),
                    *checked,
                    *enabled,
                    label,
                    true,
                    px(24.0),
                );
            }
        }

        // ---- Sound -----------------------------------------------------
        if tab == OptionsTab::Sound {
            opt_checkbox(
                fb,
                &mut self.ui_font,
                left,
                dy(341.0),
                dxw(29.0),
                self.game_options.music_enabled,
                true,
                &music_volume,
                false,
                px(24.0),
            );
            opt_slider(
                fb,
                &mut self.ui_font,
                left,
                left + col_w,
                dy(394.0),
                self.game_options.music_volume.clamp(0.0, 1.0),
                &min,
                &max,
                dy(429.0),
                unit,
                px(20.0),
            );
            opt_checkbox(
                fb,
                &mut self.ui_font,
                left,
                dy(497.0),
                dxw(29.0),
                self.game_options.sfx_enabled,
                true,
                &digital_volume,
                false,
                px(24.0),
            );
            opt_slider(
                fb,
                &mut self.ui_font,
                left,
                left + col_w,
                dy(550.0),
                self.game_options.sfx_volume.clamp(0.0, 1.0),
                &min,
                &max,
                dy(585.0),
                unit,
                px(20.0),
            );
            for (i, (label, checked)) in [
                (unit_speech.as_str(), self.game_options.unit_speech),
                (unit_ack.as_str(), self.game_options.unit_acks),
                (
                    sound_background.as_str(),
                    self.game_options.sound_background,
                ),
                (building_sounds.as_str(), self.game_options.building_sounds),
                (game_subtitles.as_str(), self.game_options.subtitles),
                (
                    cinematic_subtitles.as_str(),
                    self.game_options.cinematic_subtitles,
                ),
                (original_vo.as_str(), self.game_options.original_voice_overs),
                (
                    original_campaign.as_str(),
                    self.game_options.original_1998_campaign,
                ),
            ]
            .iter()
            .enumerate()
            {
                opt_checkbox(
                    fb,
                    &mut self.ui_font,
                    right,
                    dy(341.0 + 54.0 * i as f32),
                    dxw(29.0),
                    *checked,
                    true,
                    label,
                    false,
                    px(24.0),
                );
            }
        }

        // ---- Video -----------------------------------------------------
        if tab == OptionsTab::Video {
            opt_text(
                fb,
                &mut self.ui_font,
                dx(900.0),
                dy(346.0),
                &display_mode,
                px(23.5),
                CSS_LAVENDER,
            );
            opt_help(
                fb,
                &mut self.ui_font,
                fx(1046.0),
                fy(349.5),
                9.0 * unit,
                CSS_LAVENDER,
                px(23.5),
            );
            opt_dropdown(
                fb,
                &mut self.ui_font,
                left,
                fy(373.5),
                col_w,
                &display_modes[self.display_mode.min(DISPLAY_MODES.len() as u8 - 1) as usize],
                true,
                unit,
            );
            let grey = Rgb::new(128, 128, 128);
            opt_text(
                fb,
                &mut self.ui_font,
                dx(900.0),
                dy(448.0),
                &monitor,
                px(23.5),
                CSS_LAVENDER,
            );
            opt_dropdown(
                fb,
                &mut self.ui_font,
                left,
                fy(475.5),
                col_w,
                &format!("1. {default_label}"),
                false,
                unit,
            );
            opt_text(
                fb,
                &mut self.ui_font,
                dx(900.0),
                dy(550.0),
                &gamma,
                px(23.5),
                grey,
            );
            opt_help(
                fb,
                &mut self.ui_font,
                fx(1105.0),
                fy(553.5),
                9.0 * unit,
                grey,
                px(23.5),
            );
            opt_slider_grey(
                fb,
                &mut self.ui_font,
                left,
                left + col_w,
                dy(595.0),
                0.5,
                &min,
                &max,
                dy(630.0),
                unit,
                px(20.0),
            );

            opt_checkbox(
                fb,
                &mut self.ui_font,
                right,
                dy(341.0),
                dxw(29.0),
                self.game_options.video_fps_cap,
                true,
                &fps_cap,
                false,
                px(24.0),
            );
            opt_slider(
                fb,
                &mut self.ui_font,
                right,
                right_end,
                dy(412.0),
                self.game_options.video_fps_limit.clamp(0.0, 1.0),
                "100",
                "300",
                dy(447.0),
                unit,
                px(20.0),
            );
            opt_text(
                fb,
                &mut self.ui_font,
                right,
                dy(520.0),
                &sd_filter,
                px(23.3),
                white,
            );
            opt_slider_stops(
                fb,
                &mut self.ui_font,
                right,
                right_end,
                dy(565.0),
                self.game_options.video_sd_filter.clamp(0.0, 1.0),
                &sharp,
                &filtered,
                dy(600.0),
                unit,
                px(20.0),
                4,
            );
            for (i, (label, checked)) in [
                (vsync.as_str(), self.game_options.video_vsync),
                (hd_graphics.as_str(), self.game_options.video_hd),
                (environment.as_str(), self.game_options.video_environment),
                (real_time.as_str(), self.game_options.video_lighting),
                (
                    smooth_turning.as_str(),
                    self.game_options.video_smooth_turning,
                ),
                (
                    shadow_stack.as_str(),
                    self.game_options.video_shadow_stacking,
                ),
                (pillarbox.as_str(), self.game_options.video_pillarbox),
            ]
            .iter()
            .enumerate()
            {
                opt_checkbox(
                    fb,
                    &mut self.ui_font,
                    right,
                    dy(668.0 + 54.0 * i as f32),
                    dxw(29.0),
                    *checked,
                    true,
                    label,
                    matches!(i, 2 | 3),
                    px(24.0),
                );
            }
        }

        // ---- Game ------------------------------------------------------
        if tab == OptionsTab::Game {
            opt_text(
                fb,
                &mut self.ui_font,
                dx(900.0),
                dy(346.0),
                &gameplay,
                px(30.0),
                white,
            );
            for (i, (label, checked)) in [
                (game_timer.as_str(), self.game_options.game_timer),
                (color_cycling.as_str(), self.game_options.game_color_cycling),
                (
                    language_filter.as_str(),
                    self.game_options.game_language_filter,
                ),
                (bonus_skins.as_str(), self.game_options.game_bonus_skins),
            ]
            .iter()
            .enumerate()
            {
                opt_checkbox(
                    fb,
                    &mut self.ui_font,
                    left,
                    dy(390.0 + 54.0 * i as f32),
                    dxw(29.0),
                    *checked,
                    true,
                    label,
                    false,
                    px(24.0),
                );
            }
            opt_text(
                fb,
                &mut self.ui_font,
                dx(900.0),
                dy(611.0),
                &console_label,
                px(24.0),
                CSS_LAVENDER,
            );
            opt_dropdown(
                fb,
                &mut self.ui_font,
                left,
                fy(638.17),
                col_w,
                &default_label,
                true,
                unit,
            );
            opt_text(
                fb,
                &mut self.ui_font,
                dx(900.0),
                dy(713.0),
                &portraits,
                px(24.0),
                CSS_LAVENDER,
            );
            for (x, label, selected) in [
                (
                    900.0f32,
                    animated.as_str(),
                    self.game_options.portraits == 0,
                ),
                (1075.0, still.as_str(), self.game_options.portraits == 1),
                (1190.0, disabled.as_str(), self.game_options.portraits == 2),
            ] {
                opt_radio(
                    fb,
                    &mut self.ui_font,
                    dx(x),
                    dy(752.0),
                    dxw(36.0),
                    selected,
                    true,
                    label,
                    px(24.0),
                    unit,
                );
            }
            opt_text(
                fb,
                &mut self.ui_font,
                dx(900.0),
                dy(830.0),
                &console_label,
                px(24.0),
                CSS_LAVENDER,
            );
            for (i, (label, selected)) in [
                (
                    left_corner.as_str(),
                    self.game_options.minimap_position == 0,
                ),
                (standard.as_str(), self.game_options.minimap_position == 1),
            ]
            .iter()
            .enumerate()
            {
                opt_radio(
                    fb,
                    &mut self.ui_font,
                    dx(900.0 + 196.0 * i as f32),
                    dy(869.0),
                    dxw(36.0),
                    *selected,
                    true,
                    label,
                    px(24.0),
                    unit,
                );
            }
            opt_checkbox(
                fb,
                &mut self.ui_font,
                left,
                dy(942.0),
                dxw(29.0),
                self.game_options.game_friend_notifications,
                true,
                friends,
                false,
                px(24.0),
            );

            opt_text(
                fb,
                &mut self.ui_font,
                right,
                dy(346.0),
                &language,
                px(30.0),
                white,
            );
            for (i, line) in language_lines.iter().enumerate() {
                opt_text(
                    fb,
                    &mut self.ui_font,
                    right,
                    dy(395.0 + 27.0 * i as f32),
                    line,
                    px(23.3),
                    CSS_LAVENDER,
                );
            }
            opt_dropdown(
                fb,
                &mut self.ui_font,
                right,
                fy(449.17),
                col_w,
                &language_names[self.language.min(language_names.len() - 1)],
                true,
                unit,
            );
            opt_text(
                fb,
                &mut self.ui_font,
                right,
                dy(554.0),
                &apm_tracking,
                px(30.0),
                white,
            );
            opt_checkbox(
                fb,
                &mut self.ui_font,
                right,
                dy(598.0),
                dxw(29.0),
                self.game_options.apm_display,
                true,
                &display_apm,
                false,
                px(24.0),
            );
            opt_checkbox(
                fb,
                &mut self.ui_font,
                right,
                dy(652.0),
                dxw(29.0),
                self.game_options.apm_alert,
                true,
                &apm_alert,
                false,
                px(24.0),
            );
            let threshold = self
                .apm_input
                .clone()
                .unwrap_or_else(|| format!("{:.0}", self.game_options.apm_threshold));
            opt_input(
                fb,
                &mut self.ui_font,
                dx(1780.0),
                dy(642.0),
                dxw(75.0),
                dy(46.0),
                &threshold,
                px(23.0),
                unit,
            );
            opt_checkbox(
                fb,
                &mut self.ui_font,
                right + dxw(42.0),
                dy(712.0),
                dxw(29.0),
                self.game_options.apm_color_text,
                self.game_options.apm_alert,
                &color_text,
                false,
                px(24.0),
            );
            opt_checkbox(
                fb,
                &mut self.ui_font,
                right + dxw(42.0),
                dy(766.0),
                dxw(29.0),
                self.game_options.apm_play_sound,
                self.game_options.apm_alert,
                &play_sound,
                false,
                px(24.0),
            );
        }

        // ---- Network ---------------------------------------------------
        if tab == OptionsTab::Network && self.ingame_options {
            // In-game Network tab (Boot Camp 151 s): the explanation, the
            // Latency title and three radios.
            for (i, line) in network_lines.iter().enumerate() {
                opt_text(
                    fb,
                    &mut self.ui_font,
                    dx(897.0),
                    dy(362.0 + 26.0 * i as f32),
                    line,
                    px(22.0),
                    CSS_LAVENDER,
                );
            }
            opt_text(
                fb,
                &mut self.ui_font,
                dx(897.0),
                dy(472.0),
                &latency,
                px(28.0),
                Rgb::new(255, 255, 255),
            );
            for (i, (x, label)) in [
                (896.0f32, low_latency.as_str()),
                (1018.0, high_latency.as_str()),
                (1144.0, extra_high_latency.as_str()),
            ]
            .iter()
            .enumerate()
            {
                opt_radio(
                    fb,
                    &mut self.ui_font,
                    dx(*x),
                    dy(516.0),
                    dxw(36.0),
                    self.game_options.network_latency as usize == i,
                    true,
                    label,
                    px(24.0),
                    unit,
                );
            }
        } else if tab == OptionsTab::Network {
            opt_checkbox(
                fb,
                &mut self.ui_font,
                left,
                dy(341.0),
                dxw(29.0),
                self.game_options.network_prefer_6112,
                true,
                &prefer_port,
                false,
                px(24.0),
            );
        }

        // Restore to Default (left) and Ok (right): 5 px #331f1f outer frame
        // with a 10 px radius, black gap, and a 3 px #6c0000 inner frame with
        // a 4 px radius.  Fills are rgba(0,0,0,.8) as in the CEF CSS.
        let ok_label = self.ui_text("gluCreat-2", "Ok");
        for (label, ox, ow, ix, iw) in [
            (restore_default.as_str(), 513.0f32, 389.0, 523.0, 370.0),
            (ok_label.as_str(), 1672.0, 378.0, 1682.0, 357.0),
        ] {
            let (bx, by) = (dx(ox), dy(1212.0));
            let (bw, bh) = (dxw(ow), dy(84.0));
            rounded_fill_alpha(fb, bx, by, bw, bh, 10.0 * unit, 204);
            rounded_band(fb, bx, by, bw, bh, 10.0 * unit, 0, 5, CSS_OUTER);
            let (px0, py0) = (dx(ix), dy(1221.0));
            let (pw, ph) = (dxw(iw), dy(65.0));
            rounded_band(fb, px0, py0, pw, ph, 4.0 * unit, 0, 3, CSS_INNER);
            // 30 px/em centred 0.35 px right of the outer box centre,
            // baseline 41.875 below the inner frame (fitted on 130933
            // "Restore to Default" / "Ok", tracking as the body text).
            if let Some(font) = self.ui_font_hover.as_mut() {
                let tr = px(30.0) * CEF_TRACKING;
                let w = font.measure_cef(label, px(30.0), tr);
                font.draw_cef(
                    fb,
                    bx as f32 + bw as f32 / 2.0 + px(0.35) - w / 2.0,
                    py0 as f32 + px(41.875),
                    label,
                    px(30.0),
                    tr,
                    CSS_GREEN_HI,
                    green,
                );
            }
        }
        self.draw_options_overlays(fb, tab);
        let _ = dim_text;
    }

    /// Open drop-down lists, the help tooltips and the change-language
    /// dialog, drawn over the finished Options page.
    pub(super) fn draw_options_overlays(&mut self, fb: &mut Framebuffer, tab: OptionsTab) {
        let (cx, _cy, cw, ch) = Self::content_rect(fb);
        let kx = cw as f32 / 2560.0;
        let ky = ch as f32 / 1440.0;
        let screen = |x: f32, y: f32, w: f32, h: f32| {
            (
                cx + (x * kx).round() as i32,
                (y * ky).round() as i32,
                (w * kx).round() as i32,
                (h * ky).round() as i32,
            )
        };
        let text_px = 8.3 * ky;
        let pointer = self.pointer.and_then(|(mx, my)| {
            let x = (mx - cx) as f32 / kx;
            let y = my as f32 / ky;
            ((0.0..2560.0).contains(&x) && (0.0..1440.0).contains(&y)).then_some((x, y))
        });
        // An open list covers the page content with its own select menu.
        if let Some(kind) = self.options_drop {
            if kind == OptionsDrop::Language && !self.language_font_loaded {
                self.language_font_loaded = true;
                for path in [
                    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/noto-cjk/NotoSansCJK-Medium.ttc",
                    "/usr/share/fonts/wenquanyi/wqy-zenhei/wqy-zenhei.ttc",
                ] {
                    if let Some(font) = galaxycraft::text::UiFont::load(std::path::Path::new(path))
                    {
                        self.language_font = Some(font);
                        break;
                    }
                }
            }
            let g = options_drop_geometry(kind);
            let list = g.list;
            let pitch = g.pitch;
            let count = g.count;
            let (bx, by, bw, bh) = screen(list.0, list.1, list.2, pitch * count as f32);
            for y in by..by + bh {
                for x in bx..bx + bw {
                    fb.put(x, y, Rgb::new(2, 2, 4));
                }
            }
            for t in 0..(2.0 * ky).round().max(1.0) as i32 {
                fb.rect_outline(bx + t, by + t, bw - 2 * t, bh - 2 * t, CSS_INNER);
            }
            for i in 0..count {
                let row_top = list.1 + pitch * i as f32;
                let hot = pointer.is_some_and(|(x, y)| {
                    x >= list.0 && x < list.0 + list.2 && y >= row_top && y < row_top + pitch
                });
                let label = match kind {
                    OptionsDrop::DisplayMode => self.cef_text(
                        [
                            "glue_optionsPanel_video_label_windowMode_windowed",
                            "glue_optionsPanel_video_label_windowMode_windowedFullscreen",
                            "glue_optionsPanel_video_label_windowMode_fullscreen",
                        ][i],
                        DISPLAY_MODES[i],
                    ),
                    OptionsDrop::Language => self.cef_text(
                        &format!(
                            "glue_optionsPanel_language_{}",
                            [
                                "enUS", "frFR", "itIT", "deDE", "esES", "esMX", "ptBR", "zhCN",
                                "zhTW", "koKR", "jaJP", "plPL", "ruRU"
                            ][i]
                        ),
                        LANGUAGES[i],
                    ),
                };
                let needs_fallback = self
                    .ui_font
                    .as_ref()
                    .is_some_and(|f| label.chars().any(|c| !f.has_glyph(c)));
                let font = if needs_fallback {
                    self.language_font.as_mut()
                } else {
                    self.ui_font.as_mut()
                };
                if let Some(font) = font {
                    font.draw_cef(
                        fb,
                        bx as f32 + 10.0 * kx,
                        by as f32 + (pitch * i as f32 + 28.0) * ky,
                        &label,
                        text_px * 3.0,
                        0.0,
                        if hot { CSS_GREEN_HI } else { CSS_GREEN },
                        CSS_GREEN,
                    );
                }
            }
        }
        // Help tooltips: retail shows a short explanation when the pointer
        // rests on a row's round ? badge (references 13-16 and 18).  The
        // rows below are the measured content-pixel bands.
        let help: Option<(String, (f32, f32), f32)> = match tab {
            OptionsTab::Speed => pointer.and_then(|(x, y)| {
                if !(1456.0..2020.0).contains(&x) {
                    return None;
                }
                if (338.0..373.0).contains(&y) {
                    Some((
                        self.cef_text(
                            "glue_optionsPanel_speed_tooltip_mouseSensitivity",
                            "Use the operating system mouse speed.",
                        ),
                        (1460.0, 260.0),
                        482.0,
                    ))
                } else if (494.0..529.0).contains(&y) {
                    Some((
                        self.cef_text(
                            "glue_optionsPanel_speed_tooltip_mouseScaling",
                            "Scale input to the original screen resolution.",
                        ),
                        (1460.0, 482.0),
                        482.0,
                    ))
                } else if (602.0..637.0).contains(&y) {
                    Some((
                        self.cef_text(
                            "optionsPanel_speed_mouseConfine_tooltip",
                            "Keep the cursor inside the window. Alt+F1 toggles this in windowed mode.",
                        ),
                        (1460.0, 482.0),
                        482.0,
                    ))
                } else {
                    None
                }
            }),
            OptionsTab::Video => pointer.and_then(|(x, y)| {
                ((900.0..1420.0).contains(&x) && (340.0..372.0).contains(&y)).then_some((
                    self.cef_text(
                        "glue_optionsPanel_video_tooltip_gamma",
                        "Gamma can only be adjusted in fullscreen mode.",
                    ),
                    (960.0, 482.0),
                    900.0,
                ))
            }),
            _ => None,
        };
        if let Some((text, (ax, ay), width)) = help {
            // Reference 13/18: the tooltip is a fixed band below the
            // sliders, opaque black with a red hairline, two lines wide.
            let lines: Vec<String> = wrap_words(&text, ((width - 20.0) * ky) as i32, |t| {
                self.ui_font
                    .as_ref()
                    .map(|f| f.measure_cef(t, 8.3 * ky * 3.0, 0.0).round() as i32)
                    .unwrap_or(t.len() as i32 * 5)
            });
            let line_h = 18.0;
            let bw_c = width;
            let bh_c = lines.len() as f32 * line_h + 16.0;
            let (bx, by, bw, bh) = screen(
                ax.min(2560.0 - bw_c - 12.0).max(8.0),
                ay - bh_c - 6.0,
                bw_c,
                bh_c,
            );
            for y in by..by + bh {
                for x in bx..bx + bw {
                    fb.put(x, y, Rgb::new(2, 2, 4));
                }
            }
            for t in 0..(2.0 * ky).round().max(1.0) as i32 {
                fb.rect_outline(bx + t, by + t, bw - 2 * t, bh - 2 * t, CSS_INNER);
            }
            if let Some(font) = self.ui_font.as_mut() {
                for (i, line) in lines.iter().enumerate() {
                    font.draw_cef(
                        fb,
                        bx as f32 + 10.0 * kx,
                        by as f32 + (8.0 + (i as f32 + 1.0) * line_h) * ky,
                        line,
                        8.3 * ky * 3.0,
                        0.0,
                        Rgb::new(225, 225, 235),
                        Rgb::new(225, 225, 235),
                    );
                }
            }
        }
        if self.changing_language {
            self.draw_changing_language(fb);
        }
    }

    /// "Changing Language" restart dialog (reference 20): black rounded box
    /// at 744..1820 x 490..950 content px, title, one sentence and Exit.
    pub(super) fn draw_changing_language(&mut self, fb: &mut Framebuffer) {
        let (cx, _cy, cw, ch) = Self::content_rect(fb);
        fill_alpha_rect(fb, cx, 0, cw, ch, 96);
        let dx = |p: f32| (cw as f32 * p / 2560.0).round() as i32;
        let dy = |p: f32| (ch as f32 * p / 1440.0).round() as i32;
        let sx = |p: f32| cx + dx(p);
        let fpx = |p: f32| p * ch as f32 / 1440.0;
        let (x, y, w, h) = (sx(744.0), dy(490.0), dx(1076.0), dy(460.0));
        let radius = dx(16.0) as f32;
        rounded_fill(fb, x, y, w, h, radius, Rgb::new(0, 0, 0));
        rounded_band(fb, x, y, w, h, radius, 0, dx(3.0), CSS_OUTER);
        rounded_band(fb, x, y, w, h, radius, dx(6.0), dx(3.0), CSS_INNER);
        let title = self.cef_text(
            "glue_optionsPanel_language_dialog_title",
            "Changing Language",
        );
        if let Some(font) = self.ui_font_hover.as_mut() {
            font.draw_top_stretched(
                fb,
                sx(826.0),
                dy(572.0),
                &title,
                fpx(36.0),
                0.0,
                1.0,
                1.083,
                CSS_LAVENDER,
                None,
                1.0,
            );
        }
        let body = self.cef_text(
            "glue_optionsPanel_language_dialog_body",
            "Restart the game to change the language",
        );
        if let Some(font) = self.ui_font.as_mut() {
            font.draw_top(
                fb,
                sx(826.0),
                dy(640.0),
                &body,
                fpx(27.0),
                0.23,
                Rgb::new(221, 221, 221),
                None,
            );
        }
        let bx = sx(1391.0);
        let by = dy(813.0);
        let bw = dx(360.0);
        let bh = dy(66.0);
        let br = dx(6.0) as f32;
        rounded_fill(fb, bx, by, bw, bh, br, Rgb::new(0, 0, 0));
        rounded_band(fb, bx, by, bw, bh, br, 0, dx(3.0), CSS_INNER);
        let exit_label = self.cef_text("glue_optionsPanel_language_dialog_button", "Exit");
        if let Some(font) = self.ui_font_hover.as_mut() {
            font.draw_centered_stretched(
                fb,
                bx + bw / 2,
                by + dy(21.0),
                &exit_label,
                fpx(30.0),
                0.0,
                1.0,
                1.1,
                CSS_GREEN,
                None,
                1.0,
            );
        }
    }
}
