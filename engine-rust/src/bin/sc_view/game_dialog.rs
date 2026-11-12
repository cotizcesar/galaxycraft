//! `impl App`: game dialog (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    /// Open an in-game dialog.  A single-player game stops while one is
    /// open; a multiplayer game keeps running (it has Pause Game).
    pub(super) fn open_game_dialog(&mut self, dialog: GameDialog) {
        self.game_dialog = Some(dialog);
        self.game_dialog_hover = None;
        if !self.multiplayer_game() {
            self.paused = true;
        }
    }

    /// Back to the game (a LAN desync stays halted).
    pub(super) fn close_game_dialog(&mut self) {
        self.game_dialog = None;
        self.game_dialog_hover = None;
        self.paused = self.net.as_ref().is_some_and(|n| n.desync);
    }

    /// Retail in-game dialog: the shipped layout drawn like the other
    /// console dialogs (translucent navy pane, Eurostile, double-rim
    /// buttons with the hotkey white, the pointed button all white).
    pub(super) fn draw_game_dialog(&mut self, fb: &mut Framebuffer, dialog: GameDialog) {
        let hover = self.game_dialog_hover;
        let default_id = dialog.default_button();
        let buttons: Vec<(i32, &str, bool)> = dialog
            .buttons(self.multiplayer_game())
            .iter()
            .map(|b| (b.id, b.label, b.id == default_id))
            .collect();
        self.glue_disabled = self.game_dialog_disabled(dialog);
        match dialog {
            GameDialog::Objectives => {
                let body = self.mission_objectives_text();
                self.draw_objectives_panel_text(fb, hover, &body);
            }
            GameDialog::Tips => {
                let body = self
                    .tips_text
                    .get(self.tip_index)
                    .cloned()
                    .unwrap_or_default();
                let buttons: Vec<(i32, &str, bool)> =
                    buttons.into_iter().filter(|b| b.0 != 2).collect();
                let checked = self.game_options.tips_at_startup;
                self.draw_glue_dialog_full(
                    fb,
                    "tips_dlg",
                    "StarCraft Tips",
                    (-10, &body),
                    &buttons,
                    Some((2, "Show Tips at Startup", checked)),
                    hover,
                );
            }
            GameDialog::SaveGame | GameDialog::LoadGame => {
                self.draw_glue_dialog_full(fb, dialog.layout(), "", (0, ""), &buttons, None, hover);
                self.draw_save_load_body(fb, dialog);
            }
            GameDialog::KeystrokeHelp => {
                let title = dialog.title().join("\n");
                self.draw_glue_dialog_full(fb, "help", &title, (0, ""), &buttons, None, hover);
                self.draw_keystroke_help(fb);
            }
            _ => {
                let title = dialog.title().join("\n");
                self.draw_glue_dialog_hover(
                    fb,
                    dialog.layout(),
                    &title,
                    (0, ""),
                    &buttons,
                    None,
                    hover,
                );
            }
        }
        self.glue_disabled.clear();
    }

    /// Buttons of `dialog` that are greyed: Save without a name, Delete /
    /// Load without a selected save (Boot Camp 117-136 s).
    pub(super) fn game_dialog_disabled(&self, dialog: GameDialog) -> Vec<i32> {
        let none_selected = self
            .dialog_list_sel
            .is_none_or(|i| i >= self.save_list(false).len());
        match dialog {
            GameDialog::SaveGame => {
                let mut v = Vec::new();
                if self.save_name.trim().is_empty() {
                    v.push(-2);
                }
                if none_selected {
                    v.push(3);
                }
                v
            }
            GameDialog::LoadGame if none_selected => vec![-2],
            _ => Vec::new(),
        }
    }

    /// The local player's mission objectives (the map string set by the
    /// `Set Mission Objectives` trigger action), else the generic melee
    /// objective.  Boot Camp 331 s: "TUTORIAL: BOOT CAMP" and three lines.
    pub(super) fn mission_objectives_text(&self) -> String {
        self.world
            .triggers
            .as_ref()
            .and_then(|t| {
                t.mission_objectives
                    .get(self.local_player)
                    .copied()
                    .flatten()
            })
            .and_then(|id| self.scenario.string(id))
            .map(|s| s.replace("\r\n", "\n"))
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Destroy all enemy buildings to win.".to_string())
    }

    /// Saves listed by the Save / Load dialogs: their names (file stems).
    pub(super) fn save_names(&self) -> Vec<String> {
        self.save_list(false)
            .iter()
            .map(|p| {
                p.file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string()
            })
            .collect()
    }

    /// Screen rect of control `id` of an in-game dialog layout.
    pub(super) fn dialog_control_rect(&self, layout: &str, id: i32) -> Option<UiRect> {
        let l = self.console.as_ref()?.layout(layout)?;
        Some(self.hud_rect(layout, l.control(id)?.abs))
    }

    /// Row rects of a dialog list box (16 UI px rows).
    pub(super) fn dialog_list_rows(&self, layout: &str, list_id: i32, n: usize) -> Vec<UiRect> {
        let Some(r) = self.dialog_control_rect(layout, list_id) else {
            return Vec::new();
        };
        let k = self.content_box.3 as f32 / 480.0;
        let pitch = (16.0 * k).round() as i32;
        (0..n)
            .map(|i| UiRect::new(r.x + 2, r.y + 2 + pitch * i as i32, r.w - 4, pitch))
            .take_while(|row| row.y + row.h <= r.y + r.h)
            .collect()
    }

    /// Save Game / Load Game contents over the pane: the bigger title, the
    /// name field with its caret and "| EN" (Save), and the saves list with
    /// the selected row framed (Boot Camp 124 s / 135.6 s).
    pub(super) fn draw_save_load_body(&mut self, fb: &mut Framebuffer, dialog: GameDialog) {
        let layout = dialog.layout();
        let k = self.content_box.3 as f32 / 1440.0;
        let cx0 = self.content_box.0;
        let to_c = |r: UiRect| {
            (
                (r.x - cx0) as f32 / k,
                r.y as f32 / k,
                r.w as f32 / k,
                r.h as f32 / k,
            )
        };
        let title_id = if dialog == GameDialog::SaveGame {
            4
        } else {
            -1
        };
        if let Some(r) = self.dialog_control_rect(layout, title_id) {
            let (x, y, w, _) = to_c(r);
            let title = dialog.title()[0];
            let tw = self.glue_measure(title, GLUE_BIGGER_PX, GlueFace::Bigger);
            for (d, colour) in [(DIALOG_SHADOW, Rgb::new(0, 0, 0)), (0.0, HUD_TEXT_COLOR)] {
                self.glue_text_face(
                    fb,
                    x + (w - tw) / 2.0 + d,
                    y + DIALOG_TITLE_BASE + 4.0 + d,
                    title,
                    GLUE_BIGGER_PX,
                    colour,
                    None,
                    GlueFace::Bigger,
                );
            }
        }
        const TEXT_PX: f32 = 31.2;
        let white = Rgb::new(255, 255, 255);
        if dialog == GameDialog::SaveGame
            && let Some(r) = self.dialog_control_rect(layout, 2)
        {
            // Edit box: bright blue rim (focus), the typed name, a caret and
            // the input-language tag at the right.
            fb.rect_outline(r.x, r.y, r.w, r.h, DIALOG_BUTTON_OUTER);
            fb.rect_outline(r.x + 1, r.y + 1, r.w - 2, r.h - 2, DIALOG_BUTTON_OUTER);
            let (x, y, w, h) = to_c(r);
            let base = y + h / 2.0 + TEXT_PX * 0.35;
            let name = self.save_name.clone();
            let end = self.glue_text_face(
                fb,
                x + 8.0,
                base,
                &name,
                TEXT_PX,
                white,
                None,
                GlueFace::Hud,
            );
            let tw = end - (x + 8.0);
            let blink = (self.cursor_started.elapsed().as_millis() / 500).is_multiple_of(2);
            if blink {
                let caret = (x + 8.0 + tw + 2.0, y + 8.0);
                fb.rect(
                    cx0 + (caret.0 * k) as i32,
                    (caret.1 * k) as i32,
                    (2.0 * k).max(1.0) as i32,
                    ((h - 16.0) * k) as i32,
                    white,
                );
            }
            let tag = format!("| {}", input_language_tag(&self.locale_code));
            let tagw = self.glue_measure(&tag, TEXT_PX, GlueFace::Hud);
            self.glue_text_face(
                fb,
                x + w - tagw - 10.0,
                base,
                &tag,
                TEXT_PX,
                white,
                None,
                GlueFace::Hud,
            );
        }
        let list_id = 1;
        if let Some(r) = self.dialog_control_rect(layout, list_id) {
            fb.rect_outline(r.x, r.y, r.w, r.h, Rgb::new(10, 14, 40));
            fb.rect_outline(r.x + 1, r.y + 1, r.w - 2, r.h - 2, Rgb::new(70, 74, 96));
            let names = self.save_names();
            let rows = self.dialog_list_rows(layout, list_id, names.len());
            for (i, (row, name)) in rows.iter().zip(names.iter()).enumerate() {
                if self.dialog_list_sel == Some(i) {
                    draw_rounded_rim(
                        fb,
                        (row.x, row.y, row.w, row.h),
                        4.0 * k,
                        (2.0 * k).max(1.0),
                        Rgb::new(220, 220, 230),
                    );
                }
                let (x, y, _, h) = to_c(*row);
                self.glue_text_face(
                    fb,
                    x + 6.0,
                    y + h / 2.0 + TEXT_PX * 0.35,
                    name,
                    TEXT_PX,
                    white,
                    None,
                    GlueFace::Hud,
                );
            }
        }
    }

    /// StarCraft Help: ten `help_txt` lines from `help_scroll` (keys white,
    /// `\x03` spans yellow, the rest lavender) and the scroll bar (159 s).
    pub(super) fn draw_keystroke_help(&mut self, fb: &mut Framebuffer) {
        let Some(r) = self.dialog_control_rect("help", 1) else {
            return;
        };
        let k = self.content_box.3 as f32 / 1440.0;
        let cx0 = self.content_box.0;
        let (x, y, w, h) = (
            (r.x - cx0) as f32 / k,
            r.y as f32 / k,
            r.w as f32 / k,
            r.h as f32 / k,
        );
        const TEXT_PX: f32 = 31.2;
        let pitch = 48.0;
        let visible = (h / pitch).floor() as usize;
        let max = self.help_lines.len().saturating_sub(visible);
        self.help_scroll = self.help_scroll.min(max);
        let lines: Vec<String> = self
            .help_lines
            .iter()
            .skip(self.help_scroll)
            .take(visible)
            .cloned()
            .collect();
        for (i, line) in lines.iter().enumerate() {
            let base = y + 30.0 + pitch * i as f32;
            let mut px = x + 8.0;
            let mut colour = HUD_TEXT_COLOR;
            let mut run = String::new();
            let flush =
                |app: &mut Self, fb: &mut Framebuffer, px: &mut f32, run: &mut String, c: Rgb| {
                    if !run.is_empty() {
                        app.glue_text_face(
                            fb,
                            *px + DIALOG_SHADOW,
                            base + DIALOG_SHADOW,
                            run,
                            TEXT_PX,
                            Rgb::new(0, 0, 0),
                            None,
                            GlueFace::Hud,
                        );
                        *px =
                            app.glue_text_face(fb, *px, base, run, TEXT_PX, c, None, GlueFace::Hud);
                        run.clear();
                    }
                };
            for ch in line.chars() {
                let next = match ch {
                    '\u{4}' => Some(Rgb::new(255, 255, 255)),
                    '\u{3}' => Some(TOOLTIP_HOTKEY),
                    '\u{2}' | '\u{1}' => Some(HUD_TEXT_COLOR),
                    _ => None,
                };
                match next {
                    Some(c) => {
                        flush(self, fb, &mut px, &mut run, colour);
                        colour = c;
                    }
                    None => run.push(ch),
                }
            }
            flush(self, fb, &mut px, &mut run, colour);
        }
        // Scroll bar: arrows and a track with the knob (blue, as 159 s).
        let bar_x = cx0 + ((x + w - 34.0) * k) as i32;
        let bw = (24.0 * k) as i32;
        let top = r.y;
        let bottom = r.y + r.h;
        let arrow = (28.0 * k) as i32;
        let blue = DIALOG_BUTTON_OUTER;
        fb.rect_outline(bar_x, top + arrow, bw, (bottom - top) - 2 * arrow, blue);
        for i in 0..arrow {
            let half = i * bw / (2 * arrow.max(1));
            fb.rect(bar_x + bw / 2 - half, top + i, 2 * half + 1, 1, blue);
            fb.rect(bar_x + bw / 2 - half, bottom - i - 1, 2 * half + 1, 1, blue);
        }
        let track = (bottom - top) - 2 * arrow - bw;
        let knob_y = top
            + arrow
            + if max == 0 {
                0
            } else {
                track * self.help_scroll as i32 / max as i32
            };
        fb.rect_outline(
            bar_x + 2,
            knob_y + 2,
            bw - 4,
            bw - 4,
            Rgb::new(110, 140, 255),
        );
    }

    /// Button of the open dialog under a window point.
    pub(super) fn game_dialog_button_at(
        &self,
        x: i32,
        y: i32,
    ) -> Option<galaxycraft::menu::GameDialogButton> {
        let dialog = self.game_dialog?;
        let layout = self.console.as_ref()?.layout(dialog.layout())?;
        dialog
            .buttons(self.multiplayer_game())
            .into_iter()
            .find(|b| {
                layout
                    .control(b.id)
                    .map(|c| self.hud_rect(dialog.layout(), c.abs))
                    .is_some_and(|r| r.contains(x, y))
            })
    }

    /// Run a StarCraft Help keystroke.
    pub(super) fn run_keystroke(&mut self, action: galaxycraft::menu::Keystroke) {
        use galaxycraft::menu::Keystroke as K;
        let mp = self.multiplayer_game();
        match action {
            K::GameMenu => self.switch_game_dialog(GameDialog::Menu),
            K::SaveGame => self.switch_game_dialog(GameDialog::SaveGame),
            K::LoadGame if !mp => self.switch_game_dialog(GameDialog::LoadGame),
            K::LoadGame => {}
            K::HelpMenu => self.switch_game_dialog(GameDialog::HelpMenu),
            K::Options => {
                self.play("ui_click");
                self.open_ingame_options();
            }
            K::ExitStarCraft => self.switch_game_dialog(GameDialog::ConfirmExit),
            K::QuitMission if !mp => self.switch_game_dialog(GameDialog::ConfirmQuit),
            K::QuitMission => self.switch_game_dialog(GameDialog::EndMission),
            K::ToggleMusic => {
                self.game_options.music_enabled = !self.game_options.music_enabled;
                self.volumes_sent = None;
            }
            K::ToggleSfx => {
                self.game_options.sfx_enabled = !self.game_options.sfx_enabled;
                self.volumes_sent = None;
            }
            K::CenterOnUnit => {
                if let Some(u) = self.first_selected() {
                    self.camera = (u.x / 256 - self.view.0 / 2, u.y / 256 - self.view.1 / 2);
                    self.clamp_camera();
                }
            }
            K::SaveLocation(i) => self.map_locations[i] = Some(self.camera),
            K::RestoreLocation(i) => {
                if let Some(c) = self.map_locations[i] {
                    self.camera = c;
                    self.clamp_camera();
                }
            }
        }
    }

    /// Open a dialog (from another dialog or a hotkey), resetting its state.
    pub(super) fn switch_game_dialog(&mut self, d: GameDialog) {
        if self.game_dialog.is_none() {
            self.open_game_dialog(d);
        }
        self.game_dialog = Some(d);
        self.game_dialog_hover = None;
        match d {
            GameDialog::SaveGame => {
                self.save_name.clear();
                self.dialog_list_sel = None;
            }
            GameDialog::LoadGame => {
                self.dialog_list_sel = (!self.save_list(false).is_empty()).then_some(0);
            }
            GameDialog::KeystrokeHelp => self.help_scroll = 0,
            _ => {}
        }
    }

    /// Run a dialog button.
    pub(super) fn game_dialog_action(&mut self, action: GameDialogAction) {
        self.play("ui_click");
        match action {
            GameDialogAction::Close => {
                if self.game_dialog == Some(GameDialog::Tips) {
                    // INFERENCE: the next game opens on the following tip.
                    self.tip_index = (self.tip_index + 1) % self.tips_text.len().max(1);
                }
                self.close_game_dialog();
            }
            GameDialogAction::Open(d) => self.switch_game_dialog(d),
            GameDialogAction::SaveNamed => {
                let name = self.save_name.trim().to_string();
                if name.is_empty() {
                    return;
                }
                self.close_game_dialog();
                self.save_game_named(&name);
            }
            GameDialogAction::DeleteSave => {
                if let Some(p) = self
                    .dialog_list_sel
                    .and_then(|i| self.save_list(false).get(i).cloned())
                {
                    let _ = std::fs::remove_file(&p);
                    self.dialog_list_sel = None;
                }
            }
            GameDialogAction::LoadSelected => {
                if let Some(p) = self
                    .dialog_list_sel
                    .and_then(|i| self.save_list(false).get(i).cloned())
                {
                    self.close_game_dialog();
                    self.load_save(&p, false);
                }
            }
            GameDialogAction::PauseGame => {
                self.game_dialog = None;
                self.game_dialog_hover = None;
                if self.net.is_some() {
                    self.status("Pause Game is not available in LAN games yet".to_string());
                } else {
                    self.paused = true;
                    self.status("Game paused - F10 for the menu".to_string());
                }
            }
            GameDialogAction::Options => self.open_ingame_options(),
            GameDialogAction::Restart => {
                self.close_game_dialog();
                self.restart();
            }
            GameDialogAction::QuitMission => {
                // 495 s -> 497 s: the game closes straight to the screen
                // the game was started from (here the episode list).
                self.close_game_dialog();
                self.open_menu();
                self.episode_sel = galaxycraft::menu::episode_start(self.intro_expansion);
                self.menu_screen = MenuScreen::Episode;
            }
            GameDialogAction::Surrender => {
                // INFERENCE: the game ends as a defeat for this player (a
                // client-side result; the world is not changed).
                self.close_game_dialog();
                self.game_result = Some(false);
                self.end_dialog = true;
                self.end_dialog_hold = 0;
                self.result_tick = self.world.tick;
                self.result_panel =
                    GlueArt::load_score_panel(&self.assets_root, self.local_race(), false);
                self.result_fit = None;
                self.play_result_music(false);
            }
            GameDialogAction::ExitProgram => self.quit = true,
            GameDialogAction::PreviousTip | GameDialogAction::NextTip => {
                let n = self.tips_text.len().max(1);
                self.tip_index = if action == GameDialogAction::NextTip {
                    (self.tip_index + 1) % n
                } else {
                    (self.tip_index + n - 1) % n
                };
            }
            GameDialogAction::ToggleTipsAtStartup => {
                self.game_options.tips_at_startup = !self.game_options.tips_at_startup;
            }
        }
    }

    /// The Options popup over the game (Boot Camp 138-153 s: the main
    /// menu's Options, over the dimmed game).
    pub(super) fn open_ingame_options(&mut self) {
        self.game_dialog = None;
        self.game_dialog_hover = None;
        self.ingame_options = true;
        self.options_reset = false;
        self.menu_screen = MenuScreen::Options {
            tab: OptionsTab::Speed,
        };
        if !self.multiplayer_game() {
            self.paused = true;
        }
    }

    /// Input while a dialog is open (or a multiplayer game is paused).
    pub(super) fn handle_game_dialog_input(
        &mut self,
        keys: &[Key],
        mouse: Option<(i32, i32)>,
        lmb: bool,
        wheel: i32,
        shift: bool,
    ) {
        let Some(dialog) = self.game_dialog else {
            // Paused without a dialog: F10 / Esc / a click bring the menu.
            let click = lmb && !self.lmb_down;
            self.lmb_down = lmb;
            if click || keys.iter().any(|k| matches!(k, Key::F10 | Key::Escape)) {
                self.open_game_dialog(GameDialog::Menu);
            }
            return;
        };
        let mp = self.multiplayer_game();
        let disabled = self.game_dialog_disabled(dialog);
        for &key in keys {
            if dialog == GameDialog::KeystrokeHelp {
                match key {
                    Key::Up => self.help_scroll = self.help_scroll.saturating_sub(1),
                    Key::Down => self.help_scroll += 1,
                    _ => {}
                }
            }
            if dialog.takes_text() {
                match key {
                    Key::Backspace => {
                        self.save_name.pop();
                        continue;
                    }
                    Key::Space if self.save_name.len() < 32 => {
                        self.save_name.push(' ');
                        continue;
                    }
                    k => {
                        let c = match k {
                            Key::Minus => Some(if shift { '_' } else { '-' }),
                            Key::Period => Some('.'),
                            Key::Key9 if shift => Some('('),
                            Key::Key0 if shift => Some(')'),
                            _ => key_letter(k)
                                .map(|c| if shift { c } else { c.to_ascii_lowercase() })
                                .or_else(|| mp::key_digit(k)),
                        };
                        if let Some(c) = c
                            && self.save_name.len() < 32
                        {
                            self.save_name.push(c);
                            continue;
                        }
                    }
                }
            }
            let ch = match key {
                Key::Escape => Some('\u{1b}'),
                Key::Enter | Key::NumPadEnter => Some('\r'),
                Key::F10 if dialog == GameDialog::Menu => {
                    self.game_dialog_action(GameDialogAction::Close);
                    return;
                }
                k => key_letter(k),
            };
            if let Some(b) = ch.and_then(|c| dialog.button_for_key(mp, c))
                && !disabled.contains(&b.id)
            {
                self.game_dialog_action(b.action);
                return;
            }
        }
        if dialog == GameDialog::KeystrokeHelp && wheel != 0 {
            self.help_scroll = (self.help_scroll as i32 + wheel).max(0) as usize;
        }
        let hit = mouse.and_then(|(x, y)| self.game_dialog_button_at(x, y));
        let hover = hit.map(|b| b.id).filter(|id| !disabled.contains(id));
        if hover != self.game_dialog_hover {
            self.game_dialog_hover = hover;
        }
        let click = lmb && !self.lmb_down;
        self.lmb_down = lmb;
        if !click {
            return;
        }
        if let Some(b) = hit
            && !disabled.contains(&b.id)
        {
            self.game_dialog_action(b.action);
            return;
        }
        if let Some((x, y)) = mouse {
            match dialog {
                GameDialog::SaveGame | GameDialog::LoadGame => {
                    let n = self.save_list(false).len();
                    let rows = self.dialog_list_rows(dialog.layout(), 1, n);
                    if let Some(i) = rows.iter().position(|r| r.contains(x, y)) {
                        self.dialog_list_sel = Some(i);
                        if dialog == GameDialog::SaveGame {
                            self.save_name = self.save_names().get(i).cloned().unwrap_or_default();
                        }
                        self.play("ui_click");
                    }
                }
                GameDialog::KeystrokeHelp => {
                    if let Some(r) = self.dialog_control_rect("help", 1)
                        && r.contains(x, y)
                    {
                        let k = self.content_box.3 as f32 / 480.0;
                        let arrow = (9.3 * k) as i32;
                        if y < r.y + arrow {
                            self.help_scroll = self.help_scroll.saturating_sub(1);
                        } else if y > r.y + r.h - arrow {
                            self.help_scroll += 1;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// End-of-mission dialog over the game (`wmission` / `lmission`, 288x128
    /// centred): the two-line `RC_BIGTEXT` message centred in CTEXT -20 and
    /// the default button -2 (Victory / OK).  The campaign shows no
    /// Continue Playing / Become Observer button (retail recording).
    pub(super) fn draw_end_dialog(&mut self, fb: &mut Framebuffer) {
        let won = self.game_result == Some(true);
        let observer_text = self.observing().then(|| self.observer_result_text());
        let (dialog, text, button) = if let Some(t) = observer_text.as_deref() {
            ("wmission", t, "o\u{4}O\u{1}K")
        } else if won {
            (
                "wmission",
                "Congratulations!\nYou are victorious!",
                "v\u{4}V\u{1}ictory",
            )
        } else {
            (
                "lmission",
                "You failed to\nachieve victory!",
                "o\u{4}O\u{1}K",
            )
        };
        self.draw_glue_dialog(fb, dialog, "", (0, ""), &[(-2, button, true)], None);
        let Some(layout) = self
            .console
            .as_ref()
            .and_then(|c| c.layout(dialog))
            .cloned()
        else {
            return;
        };
        let Some(c) = layout.controls.iter().find(|c| c.id == -20) else {
            return;
        };
        let k = self.content_box.3 as f32 / 1440.0;
        let r = self.hud_rect(dialog, c.abs);
        let (x, y, w) = (
            (r.x - self.content_box.0) as f32 / k,
            r.y as f32 / k,
            r.w as f32 / k,
        );
        for (i, line) in text.lines().enumerate() {
            let tw = self.glue_measure(line, GLUE_BIG_PX, GlueFace::Big);
            self.glue_shadow_text(
                fb,
                x + (w - tw) / 2.0,
                y + END_DIALOG_BASE + i as f32 * END_DIALOG_PITCH,
                line,
                Some(END_DIALOG_TEXT),
            );
        }
    }

    /// Mission Objectives (`objctdlg`): title, the objective text and the
    /// default `Previous (Esc)` button (134919).
    pub(super) fn draw_objectives_panel(&mut self, fb: &mut Framebuffer) {
        self.draw_objectives_panel_hover(fb, None);
    }

    pub(super) fn draw_objectives_panel_hover(&mut self, fb: &mut Framebuffer, hover: Option<i32>) {
        self.draw_objectives_panel_text(fb, hover, "Destroy all enemy buildings to win.");
    }

    pub(super) fn draw_objectives_panel_text(
        &mut self,
        fb: &mut Framebuffer,
        hover: Option<i32>,
        body: &str,
    ) {
        self.draw_glue_dialog_hover(
            fb,
            "objctdlg",
            "Mission Objectives",
            (2, body),
            &[(-3, "\u{1b}Previous (\u{4}Esc\u{1})", true)],
            None,
            hover,
        );
    }

    /// StarCraft Tips (`tips_dlg`): title, tip text, the "Show Tips at
    /// Startup" checkbox and OK / Previous Tip / Next Tip (134931).
    pub(super) fn draw_tips_panel(&mut self, fb: &mut Framebuffer) {
        let body = "Tips are read from your StarCraft installation.";
        self.draw_glue_dialog(
            fb,
            "tips_dlg",
            "StarCraft Tips",
            (-10, body),
            &[
                (-2, "o\u{4}O\u{1}K", true),
                (3, "p\u{4}P\u{1}revious Tip", false),
                (1, "n\u{4}N\u{1}ext Tip", false),
            ],
            Some((2, "Show Tips at Startup")),
        );
    }

    /// In-game glue dialog (centre-anchored like `statdata`): a translucent
    /// navy pane (fitted on 134931 against 134905, same scene: colour
    /// (4,17,54) at 75 %), `RC_BIGTEXT` Eurostile with a drop shadow, and
    /// buttons whose label is yellow with the `\x04` hotkey span white.
    #[allow(clippy::type_complexity)]
    pub(super) fn draw_glue_dialog(
        &mut self,
        fb: &mut Framebuffer,
        dialog: &str,
        title: &str,
        body: (i32, &str),
        buttons: &[(i32, &str, bool)],
        checkbox: Option<(i32, &str)>,
    ) {
        let checkbox = checkbox.map(|(id, label)| (id, label, false));
        self.draw_glue_dialog_full(fb, dialog, title, body, buttons, checkbox, None);
    }

    /// [`Self::draw_glue_dialog_full`] with an unticked checkbox.
    #[allow(clippy::type_complexity, clippy::too_many_arguments)]
    pub(super) fn draw_glue_dialog_hover(
        &mut self,
        fb: &mut Framebuffer,
        dialog: &str,
        title: &str,
        body: (i32, &str),
        buttons: &[(i32, &str, bool)],
        checkbox: Option<(i32, &str)>,
        hover: Option<i32>,
    ) {
        let checkbox = checkbox.map(|(id, label)| (id, label, false));
        self.draw_glue_dialog_full(fb, dialog, title, body, buttons, checkbox, hover);
    }

    /// [`Self::draw_glue_dialog`] with the pointed button (`hover`, a
    /// control id) drawn all white, as retail does (494 s "Quit Mission"
    /// under the pointer).  `title` lines (`\n`) go to the layout's
    /// successive `CTEXT -1` controls (the prompts have three).
    /// `checkbox` is (control id, label, ticked); a ticked box shows the
    /// yellow cross of 478 s.
    #[allow(clippy::type_complexity, clippy::too_many_arguments)]
    pub(super) fn draw_glue_dialog_full(
        &mut self,
        fb: &mut Framebuffer,
        dialog: &str,
        title: &str,
        body: (i32, &str),
        buttons: &[(i32, &str, bool)],
        checkbox: Option<(i32, &str, bool)>,
        hover: Option<i32>,
    ) {
        let Some(layout) = self
            .console
            .as_ref()
            .and_then(|c| c.layout(dialog))
            .cloned()
        else {
            return;
        };
        let k = self.content_box.3 as f32 / 1440.0;
        let cx0 = self.content_box.0;
        let pane = self.hud_rect(dialog, layout.rect);
        let x0 = pane.x + (GLUE_PANE_DX * k).round() as i32;
        let w = pane.w + (GLUE_PANE_DW * k).round() as i32;
        let g = (3.0 * k).round().max(1.0) as i32;
        for y in pane.y..pane.y + pane.h {
            for x in x0..x0 + w {
                let (dx, dy) = (x - x0, y - pane.y);
                let cell = glue_pane_cell(dx, dy, w, pane.h, g);
                let (colour, alpha) = match cell {
                    PaneCell::Outside => continue,
                    PaneCell::Border => (DIALOG_EDGE, DIALOG_EDGE_ALPHA),
                    PaneCell::Fill => (DIALOG_FILL, DIALOG_ALPHA),
                };
                let c = fb.get(x, y);
                fb.put(x, y, c.blend(colour, alpha));
            }
        }
        let to_c = |r: UiRect| {
            (
                (r.x - cx0) as f32 / k,
                r.y as f32 / k,
                r.w as f32 / k,
                r.h as f32 / k,
            )
        };
        let rects: Vec<(i32, UiRect)> = layout
            .controls
            .iter()
            .filter(|c| c.id != 0)
            .map(|c| (c.id, self.hud_rect(dialog, c.abs)))
            .collect();
        let kind_of = |id: i32| {
            layout
                .controls
                .iter()
                .find(|c| c.id == id)
                .map_or("", |c| c.kind.as_str())
                .to_string()
        };
        let ctrl = |id: i32| rects.iter().find(|(i, _)| *i == id).map(|(_, r)| *r);
        // Title (CTEXT -1), centred; one line per CTEXT -1 control.
        let title_rects: Vec<UiRect> = rects
            .iter()
            .filter(|(i, _)| *i == -1)
            .map(|(_, r)| *r)
            .collect();
        for (r, line) in title_rects.into_iter().zip(title.lines()) {
            let (x, y, w, _) = to_c(r);
            let tw = self.glue_measure(line, GLUE_BIG_PX, GlueFace::Big);
            self.glue_shadow_text(
                fb,
                x + (w - tw) / 2.0 + DIALOG_TITLE_DX,
                y + DIALOG_TITLE_BASE,
                line,
                None,
            );
        }
        // Body, word-wrapped to the LTEXT width.
        if let Some(r) = ctrl(body.0) {
            let (x, y, w, _) = to_c(r);
            // Map strings break lines themselves (Boot Camp 331 s: the
            // objectives are one line each); each paragraph wraps.
            let mut lines = Vec::new();
            for para in body.1.split('\n') {
                let mut line = String::new();
                for word in para.split_whitespace() {
                    let cand = if line.is_empty() {
                        word.to_string()
                    } else {
                        format!("{line} {word}")
                    };
                    if !line.is_empty() && self.glue_measure(&cand, GLUE_BIG_PX, GlueFace::Big) > w
                    {
                        lines.push(std::mem::take(&mut line));
                        line = word.to_string();
                    } else {
                        line = cand;
                    }
                }
                lines.push(line);
            }
            for (i, l) in lines.iter().enumerate() {
                self.glue_shadow_text(
                    fb,
                    x + DIALOG_BODY_DX,
                    y + DIALOG_BODY_BASE + i as f32 * DIALOG_BODY_PITCH,
                    l,
                    None,
                );
            }
        }
        if let Some((id, label, ticked)) = checkbox
            && let Some(r) = ctrl(id)
        {
            // 134931: a 48 px box from 2 px left of the control, 3 px
            // (143,27,0) border around near-black (14,2,0).
            let side = (48.0 * k).round() as i32;
            let (bx, t) = (r.x - (2.0 * k).round() as i32, (3.0 * k).round() as i32);
            fb.rect(bx, r.y, side, side, DIALOG_CHECK_BORDER);
            fb.rect(
                bx + t,
                r.y + t,
                side - 2 * t,
                side - 2 * t,
                DIALOG_CHECK_FILL,
            );
            if ticked {
                // 478 s (sampled): a ticked box has a blue rim (12,48,145),
                // a brown face (73,48,18) and a bold yellow cross corner to
                // corner.
                fb.rect(bx, r.y, side, side, Rgb::new(12, 48, 145));
                fb.rect(
                    bx + t,
                    r.y + t,
                    side - 2 * t,
                    side - 2 * t,
                    Rgb::new(73, 48, 18),
                );
                let (x0, y0, n) = (bx + t, r.y + t, side - 2 * t);
                for i in 0..n {
                    for w in 0..3 * t {
                        let y = y0 + (i + w - 3 * t / 2).clamp(0, n - 1);
                        fb.put(x0 + i, y, TOOLTIP_HOTKEY);
                        fb.put(x0 + n - 1 - i, y, TOOLTIP_HOTKEY);
                    }
                }
            }
            let (x, y, _, _) = to_c(r);
            // `RC_SMALLTEXT`-sized label with the dialog's drop shadow
            // (134931: ink 824..1128 x 677..704 for "Show Tips at
            // Startup").
            // The HUD face at the unit-name size (glyph left edges within
            // 0.33 px of retail; see `GlueFace::Hud`).
            for (d, colour) in [(DIALOG_SHADOW, Rgb::new(0, 0, 0)), (0.0, HUD_TEXT_COLOR)] {
                self.glue_text_face(
                    fb,
                    x + DIALOG_CHECK_DX + d,
                    y + DIALOG_CHECK_BASE + d,
                    label,
                    DIALOG_CHECK_PX,
                    colour,
                    None,
                    GlueFace::Hud,
                );
            }
        }
        for &(id, label, default) in buttons {
            let Some(r) = ctrl(id) else {
                continue;
            };
            // Two 3 px rims per button, measured on 134931 relative to the
            // control rect: outer (-2, 0, +1, 0), inner rounded (+7, +9,
            // -17, -18); the default button is brighter and has a third rim
            // 3 px further out at its top and left.  Only rims: the face
            // stays the translucent pane.
            let px = |v: f32| (v * k).round() as i32;
            let disabled = self.glue_disabled.contains(&id);
            let rim = if disabled {
                Rgb::new(110, 110, 120)
            } else if default {
                DIALOG_BUTTON_OUTER
            } else {
                DIALOG_BUTTON_INNER
            };
            let outer = (r.x - px(2.0), r.y, r.w + px(1.0), r.h);
            draw_rounded_rim(fb, outer, 4.0 * k, 3.0 * k, rim);
            if default {
                let t = px(3.0);
                fb.rect(outer.0 - t, outer.1 - t, outer.2 + t, t, rim);
                fb.rect(outer.0 - t, outer.1 - t, t, outer.3 + t, rim);
            }
            draw_rounded_rim(
                fb,
                (r.x + px(7.0), r.y + px(9.0), r.w - px(17.0), r.h - px(18.0)),
                8.0 * k,
                3.0 * k,
                if disabled {
                    Rgb::new(110, 110, 120)
                } else {
                    DIALOG_BUTTON_INNER
                },
            );
            // `rc3.json` labels start with their hotkey (`o\x04O\x01K`,
            // `\x1bPrevious ...` for Esc), which is not drawn.
            let label: String = label.chars().skip(1).collect();
            let label = label.as_str();
            let (x, y, w, h) = to_c(r);
            let plain: String = label.chars().filter(|c| (*c as u32) >= 0x20).collect();
            let tw = self.glue_measure(&plain, GLUE_BIG_PX, GlueFace::Big);
            let (text, colour) = if disabled {
                (plain.as_str(), Rgb::new(140, 140, 150))
            } else if hover == Some(id) {
                (plain.as_str(), Rgb::new(255, 255, 255))
            } else {
                (label, TOOLTIP_HOTKEY)
            };
            self.glue_shadow_text(
                fb,
                x + (w - tw) / 2.0,
                y + h / 2.0 + dialog_button_base(dialog, &kind_of(id)),
                text,
                Some(colour),
            );
        }
    }

    /// Eurostile `RC_BIGTEXT` with the retail 2 px black drop shadow; `\x04`
    /// spans are white, the rest `base` (lavender when `None`, else e.g.
    /// the button yellow).
    pub(super) fn glue_shadow_text(
        &mut self,
        fb: &mut Framebuffer,
        x: f32,
        base: f32,
        text: &str,
        colour: Option<Rgb>,
    ) {
        let normal = colour.unwrap_or(HUD_TEXT_COLOR);
        let mut runs: Vec<(String, Rgb)> = Vec::new();
        let mut cur = normal;
        let mut run = String::new();
        for ch in text.chars() {
            match ch {
                '\u{4}' | '\u{1}' => {
                    if !run.is_empty() {
                        runs.push((std::mem::take(&mut run), cur));
                    }
                    cur = if ch == '\u{4}' {
                        Rgb::new(255, 255, 255)
                    } else {
                        normal
                    };
                }
                c if (c as u32) < 0x20 => {}
                c => run.push(c),
            }
        }
        if !run.is_empty() {
            runs.push((run, cur));
        }
        for (dx, dy, shadow) in [(DIALOG_SHADOW, DIALOG_SHADOW, true), (0.0, 0.0, false)] {
            let mut pen = x + dx;
            for (t, c) in &runs {
                let col = if shadow { Rgb::new(0, 0, 0) } else { *c };
                pen = self.glue_text_face(
                    fb,
                    pen,
                    base + dy,
                    t,
                    GLUE_BIG_PX,
                    col,
                    None,
                    GlueFace::Big,
                );
            }
        }
    }
}
