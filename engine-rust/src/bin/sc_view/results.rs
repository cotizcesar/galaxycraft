//! `impl App`: results (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    /// Replay playback controls: SPACE pause, +/- speed, R restart, ESC out.
    pub(super) fn handle_replay(&mut self, window: &Window) {
        let keys: Vec<Key> = window
            .get_keys_pressed(KeyRepeat::No)
            .into_iter()
            .chain(self.pad_frame.menu_pressed.iter().copied())
            .collect();
        for key in keys {
            match key {
                Key::Space => {
                    let now_paused = if let Some(rp) = self.replay.as_mut() {
                        rp.paused = !rp.paused;
                        rp.paused
                    } else {
                        false
                    };
                    self.status(
                        if now_paused {
                            "Replay paused"
                        } else {
                            "Replaying"
                        }
                        .to_string(),
                    );
                    self.play("ui_click");
                }
                Key::Equal | Key::NumPadPlus => {
                    self.speed = (self.speed * 1.25).min(4.0);
                    self.status(format!("Replay speed {:.2}x", self.speed));
                }
                Key::Minus | Key::NumPadMinus => {
                    self.speed = (self.speed / 1.25).max(0.25);
                    self.status(format!("Replay speed {:.2}x", self.speed));
                }
                Key::R => self.restart_replay(),
                Key::Escape | Key::M => self.open_menu(),
                Key::Q => self.quit = true,
                _ => {}
            }
        }
    }

    pub(super) fn restart_replay(&mut self) {
        let Some(rp) = self.replay.take() else {
            return;
        };
        let save = rp.save.clone();
        if self.build_from_save(&save) {
            self.ais.clear();
            self.campaign_ais.clear();
            self.replay = Some(ReplayPlayer::new(save));
            self.status("Replaying from start".to_string());
        }
        self.play("ui_click");
    }

    /// Advance replay playback: due commands, then steps, honoring pause and
    /// the shared speed control. No AI runs here: their orders are in the log.
    pub(super) fn update_replay(&mut self) {
        if self.replay.as_ref().map(|r| r.paused).unwrap_or(true) {
            self.last_step = Instant::now();
            return;
        }
        let now = Instant::now();
        let dt = now.duration_since(self.last_step).as_secs_f32().min(0.1);
        self.last_step = now;
        self.accumulator += dt * TICKS_PER_REAL_SECOND * self.speed;
        let mut steps = 0;
        let target = self.replay.as_ref().map(|r| r.save.tick).unwrap_or(0);
        while self.accumulator >= 1.0 && steps < 24 {
            let due = if let Some(rp) = self.replay.as_mut() {
                let mut n = 0;
                while rp.idx < rp.save.rec.cmds.len()
                    && rp.save.rec.cmds[rp.idx].tick == self.world.tick
                {
                    let c = rp.save.rec.cmds[rp.idx].clone();
                    galaxycraft::replay::apply_act(&mut self.world, &c.act);
                    rp.idx += 1;
                    n += 1;
                }
                n
            } else {
                break;
            };
            let _ = due;
            if self.world.tick >= target {
                break;
            }
            self.world.step();
            self.accumulator -= 1.0;
            steps += 1;
        }
        if self.world.tick >= target {
            self.status(format!(
                "Replay ended at tick {} (hash {:x})",
                target,
                state_hash(&self.world)
            ));
            self.replay = None;
        }
        self.selected
            .retain(|id| self.world.unit(*id).is_some_and(|u| u.alive));
    }

    pub(super) fn handle_result(&mut self, window: &Window) {
        let down = window.get_mouse_down(MouseButton::Left) || self.pad_frame.lmb;
        if down
            && !self.mouse_was_down
            && let Some(key) = self.pointer.and_then(|(x, y)| self.result_button_at(x, y))
        {
            self.result_key(key);
        }
        self.mouse_was_down = down;
        let keys: Vec<Key> = window
            .get_keys_pressed(KeyRepeat::No)
            .into_iter()
            .chain(self.pad_frame.menu_pressed.iter().copied())
            .collect();
        for key in keys {
            self.result_key(key);
        }
    }

    /// Clickable control under a window point on the end dialog (its
    /// default button) or the score screen (tabs 3..6, Ok 7), as its key.
    pub(super) fn result_button_at(&self, mx: i32, my: i32) -> Option<Key> {
        let (cx, _, _, ch) = self.content_box;
        let k = ch as f32 / 1440.0;
        if self.end_dialog {
            let dialog = if self.game_result == Some(true) {
                "wmission"
            } else {
                "lmission"
            };
            let layout = self.console.as_ref()?.layout(dialog)?;
            let r = self.hud_rect(dialog, layout.controls.iter().find(|c| c.id == -2)?.abs);
            return (mx >= r.x && mx < r.x + r.w && my >= r.y && my < r.y + r.h)
                .then_some(Key::Enter);
        }
        let layout = self
            .glue
            .as_ref()
            .and_then(|g| g.score.as_ref())
            .or_else(|| self.console.as_ref().and_then(|c| c.layout("gluscore")))?;
        let hit = |id: i32| {
            layout
                .controls
                .iter()
                .find(|c| c.id == id)
                .is_some_and(|c| {
                    let r = c.abs;
                    let x0 = cx + ((320 + r.x * 3) as f32 * k) as i32;
                    let y0 = (r.y as f32 * 3.0 * k) as i32;
                    mx >= x0
                        && mx < x0 + (r.w as f32 * 3.0 * k) as i32
                        && my >= y0
                        && my < y0 + (r.h as f32 * 3.0 * k) as i32
                })
        };
        [
            (3, Key::V),
            (4, Key::U),
            (5, Key::T),
            (6, Key::R),
            (61, Key::F6),
            (63, Key::W),
            (7, Key::Enter),
        ]
        .into_iter()
        .find(|(id, _)| hit(*id))
        .map(|(_, key)| key)
    }

    /// One key on the score/result screen (also driven by `--menu-script`
    /// `result:<Key>` steps).
    pub(super) fn result_key(&mut self, key: Key) {
        if self.end_dialog && self.end_dialog_hold > 0 {
            return;
        }
        if key == Key::F6 && !self.end_dialog {
            // Retail's Save Replay button on the score screen (F6 also
            // saves during play).
            self.save_replay();
            return;
        }
        if key == Key::W && !self.end_dialog {
            // Watch Replay: write the finished match and open it in the
            // replay player, exactly like the Replays browser.
            if let Some(path) = self.save_replay() {
                self.load_save(&path, true);
            }
            return;
        }
        if self.end_dialog {
            // `wmission`/`lmission` default button: Victory (V) / OK (O),
            // also Enter; then the score screen.
            if matches!(key, Key::Enter | Key::Space | Key::V | Key::O | Key::Escape) {
                self.play("ui_click");
                self.end_dialog = false;
            }
            return;
        }
        // Score tabs: `gluScore-1..4` hotkeys (oVerview, Units, sTructures,
        // Resources); Ok is O as well as Enter.
        let tab = match key {
            Key::V => Some(0),
            Key::U => Some(1),
            Key::T => Some(2),
            Key::R if self.game_result.is_some() => Some(3),
            _ => None,
        };
        if let Some(tab) = tab {
            self.play("ui_click");
            if tab != self.score_tab {
                // Retail restarts the count-up on every tab change.
                self.score_count_epoch = Some(Instant::now());
            }
            self.score_tab = tab;
            return;
        }
        let key = if key == Key::O { Key::Enter } else { key };
        // Leaving the score screen is a transition (Backwater Station
        // recording 651.0-653.0 s): the panel slides out, the backdrop
        // stays, then black, then what comes next.
        if matches!(key, Key::Enter | Key::Space | Key::Escape | Key::M) {
            if self.score_leave.is_none() {
                self.play("ui_click");
                self.score_leave = Some((Instant::now(), key));
            }
            return;
        }
        self.finish_score(key);
    }

    /// What the score screen's Ok leads to, once its exit has played.
    pub(super) fn finish_score(&mut self, key: Key) {
        {
            match key {
                Key::Enter | Key::Space if self.game_result == Some(true) => {
                    self.play("ui_swish");
                    // Retail progression: the cinematic entry before the
                    // next mission (or the campaign's closing one) plays
                    // first, then the next mission's briefing.
                    let movies: Vec<String> = self
                        .map_mission(self.map_index)
                        .and_then(galaxycraft::campaign_progress::after_victory)
                        .map(|a| a.movies.iter().map(|m| m.to_string()).collect())
                        .unwrap_or_default();
                    match (self.next_map, movies.is_empty()) {
                        (Some(index), true) => self.start_campaign_mission(index),
                        (Some(index), false) => self.begin_intro(movies, Some(index)),
                        (None, false) => self.begin_intro(movies, None),
                        (None, true) => self.open_menu(),
                    }
                }
                Key::Enter | Key::Space | Key::Escape | Key::M => self.open_menu(),
                Key::R => self.restart(),
                Key::Q => self.quit = true,
                _ => {}
            }
        }
    }

    /// Retail `gamemenu.ui.json` pause panel (264x288 at 184,32).
    /// Retail `glue/score*` score screen with the eight shipped row
    /// rectangles and the `msnstats` summary panel.
    /// The retail score screen (`gluscore.ui.json` over the race's
    /// `pal<race><v|d>/backgnd`, `score<race><v|d>/pmain` anchored right):
    /// title, the four tabs, per-tab column headers, one row per player in
    /// the game (name, force name, APM, three bar boxes, Total Score),
    /// Elapsed Time and Save Replay / Watch Replay / Ok.  Columns per tab
    /// from the retail Wasteland recording (`Screencast_20261105_074531`).
    pub(super) fn draw_score_screen(&mut self, fb: &mut Framebuffer) {
        let won = self.game_result == Some(true);
        let race = self.local_race();
        if self.score_art.as_ref().map(|a| (a.0, a.1)) != Some((race, won)) {
            let part = |n: &str| GlueArt::load_score_part(&self.assets_root, race, won, n);
            self.score_art = Some((
                race,
                won,
                [part("backgnd"), part("pinset"), part("scorebox")],
            ));
            self.score_icons = GlueArt::load_score_icons(&self.assets_root, race, won);
            self.result_panel = GlueArt::load_score_panel(&self.assets_root, race, won);
        }
        let Some(layout) = self
            .glue
            .as_ref()
            .and_then(|g| g.score.as_ref())
            .or_else(|| self.console.as_ref().and_then(|c| c.layout("gluscore")))
            .cloned()
        else {
            return;
        };
        let (cx, _cy, cw, ch) = Self::content_rect(fb);
        let k = ch as f32 / 1440.0;
        // Timeline (retail recording): backdrop at 0, the panel slides in
        // from the left, the numbers count up; a pinned capture
        // (`--menu-at`) shows that second, otherwise the live clock.
        let since =
            |e: &mut Option<Instant>| e.get_or_insert_with(Instant::now).elapsed().as_secs_f32();
        let (t_screen, t_count) = match self.menu_at.filter(|_| self.headless) {
            Some(at) => (at, at - SCORE_COUNT_DELAY),
            None => {
                let t = since(&mut self.score_epoch);
                let c = match self.score_count_epoch {
                    Some(e) => e.elapsed().as_secs_f32(),
                    None => t - SCORE_COUNT_DELAY,
                };
                (t, c)
            }
        };
        let headless_settled = self.headless && self.menu_at.is_none();
        let leaving = self.score_leave.map(|(t, _)| t.elapsed().as_secs_f32());
        let shift = if headless_settled {
            0.0
        } else if let Some(lt) = leaving {
            score_leave_shift(lt)
        } else {
            score_panel_shift(t_screen)
        };
        if leaving.is_some_and(|lt| lt >= SCORE_LEAVE_BLACK_AT) {
            fb.clear(Rgb::new(0, 0, 0));
            return;
        }
        let count = |v: i64| {
            if headless_settled {
                v
            } else {
                score_count(v, t_count)
            }
        };
        let sx = |ux: f32| cx + ((320.0 + ux * 3.0 + shift) * k).round() as i32;
        let sy = |uy: f32| (uy * 3.0 * k).round() as i32;
        let sl = |u: f32| (u * 3.0 * k).round() as i32;
        let ctl = |id: i32| layout.controls.iter().find(|c| c.id == id).map(|c| c.abs);
        fb.clear(Rgb::new(0, 0, 0));
        let art = self.score_art.take();
        if let Some((_, _, [bg, pinset, scorebox])) = &art {
            if let Some(bg) = bg {
                bg.draw_scaled(fb, cx, 0, cw, ch);
            }
            if let Some(main) = &self.result_panel {
                let (w, h) = (
                    (main.width as f32 * 0.75 * k) as i32,
                    (main.height as f32 * 0.75 * k) as i32,
                );
                main.draw_scaled(fb, sx(640.0) - w, 0, w, h);
            }
            // The empty frame slides in; rows, labels and numbers appear
            // together when the count starts (retail: nothing but the
            // frame at 7.8 s, everything at 8.4 s).
            let rows = if headless_settled || t_screen >= SCORE_COUNT_DELAY {
                self.score_rows()
            } else {
                Vec::new()
            };
            for (n, _) in rows.iter().enumerate() {
                if let (Some(img), Some(r)) = (pinset, ctl(13 + 6 * n as i32)) {
                    let (w, h) = (
                        (img.width as f32 * 0.75 * k) as i32,
                        (img.height as f32 * 0.75 * k) as i32,
                    );
                    img.draw_scaled(fb, sx(r.x as f32), sy(r.y as f32), w, h);
                }
                if let (Some(img), Some(r)) = (scorebox, ctl(18 + 6 * n as i32)) {
                    let (w, h) = (
                        (img.width as f32 * 0.75 * k) as i32,
                        (img.height as f32 * 0.75 * k) as i32,
                    );
                    img.draw_scaled(fb, sx(r.x as f32), sy(r.y as f32), w, h);
                }
            }
        }
        self.score_art = art;
        if !headless_settled && t_screen < SCORE_COUNT_DELAY {
            return;
        }
        let cxu = |ux: f32| 320.0 + ux * 3.0 + shift;
        let centred = |app: &mut Self,
                       fb: &mut Framebuffer,
                       r: UiRect,
                       text: &str,
                       px: f32,
                       face: GlueFace,
                       colour: Rgb,
                       base: f32| {
            let w = app.glue_measure(text, px, face);
            app.glue_text_face(
                fb,
                cxu(r.x as f32 + r.w as f32 / 2.0) - w / 2.0,
                r.y as f32 * 3.0 + base,
                text,
                px,
                colour,
                None,
                face,
            );
        };
        let white = Rgb::new(0xee, 0xee, 0xee);
        if let Some(r) = ctl(2) {
            centred(
                self,
                fb,
                r,
                if won { "Victory!" } else { "Defeat!" },
                GLUE_BIGGER_PX,
                GlueFace::Bigger,
                white,
                SCORE_TITLE_BASE,
            );
        }
        for (t, (id, label)) in [
            (3, "Overview"),
            (4, "Units"),
            (5, "Structures"),
            (6, "Resources"),
        ]
        .into_iter()
        .enumerate()
        {
            if let Some(r) = ctl(id) {
                let colour = if t == self.score_tab {
                    GLUE_BRIGHT
                } else {
                    GLUE_GREEN
                };
                centred(
                    self,
                    fb,
                    r,
                    label,
                    SCORE_TAB_PX,
                    GlueFace::Big,
                    colour,
                    SCORE_TAB_BASE,
                );
            }
        }
        let heads: [&str; 3] = match self.score_tab {
            1 => ["Produced", "Killed", "Lost"],
            2 => ["Constructed", "Razed", "Lost"],
            3 => ["Gas Mined", "Minerals Mined", "Total Spent"],
            _ => ["Units", "Structures", "Resources"],
        };
        if let Some(r) = ctl(62) {
            let w = self.glue_measure("APM", SCORE_SMALL_PX, GlueFace::Hud);
            self.glue_text_face(
                fb,
                cxu((r.x + r.w) as f32) - w,
                r.y as f32 * 3.0 + SCORE_HEAD_BASE,
                "APM",
                SCORE_SMALL_PX,
                GLUE_INFO,
                None,
                GlueFace::Hud,
            );
        }
        for (id, h) in [10, 11, 12].into_iter().zip(heads) {
            if let Some(r) = ctl(id) {
                centred(
                    self,
                    fb,
                    r,
                    h,
                    SCORE_SMALL_PX,
                    GlueFace::Hud,
                    GLUE_INFO,
                    SCORE_HEAD_BASE,
                );
            }
        }
        if let Some(r) = ctl(8) {
            centred(
                self,
                fb,
                r,
                "Total Score",
                GLUE_BIGGER_PX,
                GlueFace::Bigger,
                white,
                SCORE_TOTAL_HEAD_BASE,
            );
        }
        let rows = self.score_rows();
        let columns: Vec<[i64; 4]> = rows
            .iter()
            .map(|r| r.columns(self.score_tab).map(count))
            .collect();
        for (n, row) in rows.iter().enumerate() {
            let base_id = 14 + 6 * n as i32;
            let icons = self.score_icons.take();
            if let (Some(g), Some(r)) = (&icons, ctl(base_id)) {
                let frame = match row.player.race {
                    Race::Zerg => 3,
                    Race::Protoss => 5,
                    _ => 4,
                };
                if let Some(img) = g.frame(frame) {
                    let (w, h) = (
                        (img.width as f32 * 0.75 * k) as i32,
                        (img.height as f32 * 0.75 * k) as i32,
                    );
                    img.draw_scaled(
                        fb,
                        sx(r.x as f32 + SCORE_ICON_DX),
                        sy(r.y as f32 + SCORE_ICON_DY),
                        w,
                        h,
                    );
                }
            }
            if let Some(r) = ctl(base_id) {
                self.glue_text_face(
                    fb,
                    cxu(r.x as f32),
                    r.y as f32 * 3.0 + SCORE_NAME_BASE,
                    &row.name,
                    SCORE_NAME_PX,
                    if row.human { white } else { GLUE_INFO },
                    None,
                    GlueFace::Big,
                );
                self.glue_text_face(
                    fb,
                    cxu(r.x as f32 + SCORE_FORCE_DX),
                    r.y as f32 * 3.0 + SCORE_FORCE_BASE,
                    &row.force,
                    SCORE_SMALL_PX,
                    white,
                    None,
                    GlueFace::Hud,
                );
            }
            if let (Some(apm), Some(r)) = (row.apm, ctl(62)) {
                let t = apm.to_string();
                let w = self.glue_measure(&t, SCORE_SMALL_PX, GlueFace::Hud);
                let _ = r;
                self.glue_text_face(
                    fb,
                    cxu(SCORE_APM_RIGHT) - w,
                    (76 + 40 * n as i32) as f32 * 3.0 + SCORE_VALUE_BASE,
                    &t,
                    SCORE_SMALL_PX,
                    white,
                    None,
                    GlueFace::Hud,
                );
            }
            for c in 0..3 {
                let Some(r) = ctl(base_id + 1 + c as i32) else {
                    continue;
                };
                let max = columns.iter().map(|v| v[c]).max().unwrap_or(0).max(1);
                let v = columns[n][c];
                let (bx, by, bw, bh) = (
                    sx(r.x as f32),
                    sy(r.y as f32),
                    sl(r.w as f32),
                    sl(SCORE_BOX_H),
                );
                let fill = (bw as i64 * v.max(0) / max) as i32;
                let base = 6 + 6 * (row.slot % 12);
                if let Some(g) = &icons {
                    // Fill share, then the frame: left / stretched middle /
                    // right pieces (16 UI px ends at 4x HD, 0.75 at 1440p).
                    let pieces =
                        |fb: &mut Framebuffer, first: usize, x: i32, y: i32, w: i32, h: i32| {
                            let (Some(l), Some(m), Some(rr)) =
                                (g.frame(first), g.frame(first + 1), g.frame(first + 2))
                            else {
                                return;
                            };
                            let end = ((l.width as f32 * 0.75 * k) as i32).min(w / 2);
                            l.draw_scaled(fb, x, y, end, h);
                            m.draw_scaled(fb, x + end, y, (w - 2 * end).max(0), h);
                            rr.draw_scaled(fb, x + w - end, y, end, h);
                        };
                    if fill > 0 {
                        let inset = sl(SCORE_FILL_INSET);
                        pieces(
                            fb,
                            base,
                            bx,
                            by + inset,
                            fill.max(2 * inset),
                            bh - 2 * inset,
                        );
                    }
                    pieces(fb, base + 3, bx, by, bw, bh);
                }
                centred(
                    self,
                    fb,
                    UiRect::new(r.x, r.y, r.w, r.h),
                    &v.to_string(),
                    SCORE_SMALL_PX,
                    GlueFace::Hud,
                    white,
                    SCORE_BOX_VALUE_BASE,
                );
            }
            self.score_icons = icons;
            if let Some(r) = ctl(base_id + 4) {
                centred(
                    self,
                    fb,
                    r,
                    &columns[n][3].to_string(),
                    GLUE_BIG_PX,
                    GlueFace::Big,
                    white,
                    SCORE_TOTAL_BASE,
                );
            }
        }
        if let Some(r) = ctl(9) {
            let secs = galaxycraft::sim::real_seconds(self.elapsed_ticks());
            let text = format!("Elapsed Time: {:02}:{:02}", secs / 60, secs % 60);
            self.glue_text_face(
                fb,
                cxu(r.x as f32),
                r.y as f32 * 3.0 + SCORE_ELAPSED_BASE,
                &text,
                GLUE_BIG_PX,
                white,
                None,
                GlueFace::Big,
            );
        }
        for (id, label, colour) in [
            (61, "Save Replay", GLUE_GREEN),
            (63, "Watch Replay", GLUE_GREEN),
            (7, "Ok", GLUE_GREEN),
        ] {
            if let Some(r) = ctl(id) {
                centred(
                    self,
                    fb,
                    r,
                    label,
                    GLUE_BIGGER_PX,
                    GlueFace::Bigger,
                    colour,
                    SCORE_BUTTON_BASE,
                );
            }
        }
    }

    /// Players on the score screen: every slot in the game except rescuable
    /// and neutral ones, in slot order.
    pub(super) fn score_rows(&mut self) -> Vec<ScoreRow> {
        let slots = self.scenario.player_slots();
        let minutes =
            (self.world.tick as f64 * galaxycraft::sim::TICK_MS as f64 / 60_000.0).max(1.0 / 60.0);
        let actions = self.world.recorder.as_ref().map_or(0, |r| r.cmds.len());
        let mut rows = Vec::new();
        for s in slots.iter().filter(|s| s.active && (s.human || s.computer)) {
            let Some(p) = self.world.players.get(s.slot).cloned() else {
                continue;
            };
            if p.rescuable {
                continue;
            }
            let human = s.slot == self.local_player;
            let name = if human {
                self.active_name()
            } else {
                self.nation_name(p.race, s.slot)
            };
            let force = self
                .scenario
                .force_names
                .get(self.scenario.forces.get(s.slot).copied().unwrap_or(0) as usize)
                .and_then(|&id| self.scenario.string(id as u32))
                .unwrap_or("")
                .to_string();
            rows.push(ScoreRow {
                slot: s.slot,
                human,
                name,
                force,
                apm: human.then(|| (actions as f64 / minutes).round() as i64),
                player: p,
            });
        }
        rows
    }

    /// Computer player name: `network.json` `strNATION_<RACE>_<slot>`
    /// ("Garm Brood" is the Zerg in slot 4).
    pub(super) fn nation_name(&mut self, race: Race, slot: usize) -> String {
        if self.nation_names.is_none() {
            let parsed = self
                .install
                .as_ref()
                .and_then(|i| {
                    i.read_by_path(&format!("locales/{}/assets/rez/network.json", i.locale()))
                        .or_else(|_| i.read_by_path("locales/enus/assets/rez/network.json"))
                        .ok()
                })
                .and_then(|b| {
                    let text =
                        String::from_utf8_lossy(b.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&b))
                            .to_string();
                    serde_json::from_str::<Vec<serde_json::Value>>(&text).ok()
                })
                .map(|v| {
                    v.iter()
                        .filter_map(|e| {
                            Some((
                                e.get("Key")?.as_str()?.to_string(),
                                e.get("Value")?.as_str()?.to_string(),
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default();
            self.nation_names = Some(parsed);
        }
        let r = match race {
            Race::Zerg => "ZERG",
            Race::Protoss => "PROTOSS",
            _ => "TERRAN",
        };
        self.nation_names
            .as_ref()
            .and_then(|m| m.get(&format!("strNATION_{r}_{slot}")).cloned())
            .unwrap_or_else(|| format!("Player {}", slot + 1))
    }
}
