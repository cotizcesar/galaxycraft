//! `impl App`: game loop (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    /// Race identity for music/console selection.
    /// Watching a lobby game of computers from an observer seat: the local
    /// player is one past the last slot, so it owns no unit and gives no
    /// order; the whole map is shown.
    pub(super) fn observing(&self) -> bool {
        self.mp.setup.is_some() && self.local_player >= self.world.players.len()
    }

    /// Fog drawn for the local player (an observer sees everything, and
    /// `black sheep wall` / `war aint what it used to be` reveal the map).
    pub(super) fn fog_on(&self) -> bool {
        self.fog
            && !self.observing()
            && !self
                .world
                .players
                .get(self.local_player)
                .is_some_and(|p| p.cheats.reveal)
    }

    /// Player whose resources, supply and upgrades the HUD shows: the local
    /// one, or for an observer the owner of the selected unit.
    pub(super) fn view_player(&self) -> usize {
        if self.local_player < self.world.players.len() {
            return self.local_player;
        }
        self.first_selected()
            .map(|u| u.owner as usize)
            .filter(|&o| o < self.world.players.len())
            .unwrap_or(0)
    }

    /// End-of-game line for an observer: the side still standing.
    pub(super) fn observer_result_text(&self) -> String {
        let Some(setup) = self.mp.setup.as_ref() else {
            return "The game is over!".to_string();
        };
        let left = setup.sides_left(&self.world);
        let winner = match left.as_slice() {
            [side] => {
                if setup.game_type == galaxycraft::multiplayer::GameType::TopVsBottom {
                    Some(if *side == 0 { "Home Team" } else { "Away Team" }.to_string())
                } else if setup.game_type.is_team() {
                    Some(format!("Team {}", side + 1))
                } else {
                    setup
                        .slots
                        .iter()
                        .enumerate()
                        .find(|(i, _)| self.world.players.get(*i).is_some_and(|p| p.team == *side))
                        .and_then(|(_, s)| {
                            galaxycraft::multiplayer::COLOR_CHOICES.get(s.color as usize)
                        })
                        .map(|c| c.to_string())
                }
            }
            _ => None,
        };
        match winner {
            Some(w) => format!("The game is over!\n{w} wins!"),
            None => "The game is over!\nNobody wins.".to_string(),
        }
    }

    /// Lobby game types that change the rules rather than only the teams.
    /// Greed races to 10,000 gathered resources; Slaughter scores kills for
    /// 30 minutes; Sudden Death ends a player when a town hall is destroyed.
    /// Capture the Flag needs the flag units, which the reconstruction does
    /// not create yet.
    pub(super) fn apply_special_rules(&mut self) {
        self.world.sudden_death = false;
        self.world.greed_goal = None;
        self.world.slaughter_deadline = None;
        let Some(setup) = self.mp.setup.as_ref() else {
            return;
        };
        use galaxycraft::multiplayer::GameType;
        match setup.game_type {
            GameType::Greed => self.world.greed_goal = Some(10_000),
            GameType::Slaughter => {
                self.world.slaughter_deadline =
                    Some(30 * 60 * galaxycraft::sim::TICKS_PER_SECOND as u64)
            }
            GameType::SuddenDeath => self.world.sudden_death = true,
            GameType::CaptureTheFlag | GameType::TeamCaptureTheFlag => self.world.start_ctf(),
            _ => {}
        }
    }

    pub(super) fn local_race(&self) -> Race {
        self.world
            .players
            .get(self.local_player)
            .map(|p| p.race)
            .unwrap_or(Race::Terran)
    }

    pub(super) fn content_rect(fb: &Framebuffer) -> (i32, i32, i32, i32) {
        let h = fb.height.max(1);
        // The Video option "Pillarbox" restores the original 4:3 playfield
        // (the retail `OriginalAspectRatio` setting) for the game screens;
        // menus and movies stay 16:9.
        let aspect = if GAME_PILLARBOX.load(Ordering::Relaxed) {
            4.0 / 3.0
        } else {
            16.0 / 9.0
        };
        let want_w = ((h as f32 * aspect).round() as i32).max(1);
        let w = want_w.min(fb.width.max(1));
        let x0 = (fb.width - w) / 2;
        (x0, 0, w, h)
    }

    pub(super) fn ui_metrics(&self, fb: &Framebuffer) -> (f32, i32, i32) {
        let (cx, cy, cw, ch) = Self::content_rect(fb);
        let scale = (ch as f32 / 480.0).max(0.5);
        let ox = cx + ((cw as f32 - 640.0 * scale) / 2.0).round() as i32;
        (scale, ox, cy)
    }

    pub(super) fn update_view(&mut self, fb: &Framebuffer) {
        GAME_PILLARBOX.store(
            self.game_options.video_pillarbox && !self.menu && !self.intro && !self.briefing,
            Ordering::Relaxed,
        );
        let (scale, ox, oy) = self.ui_metrics(fb);
        self.ui_scale = scale;
        self.ui_ox = ox;
        self.ui_oy = oy;
        // Logical world viewport inside the 16:9 content box.  The divisor
        // must equal the blit scale used in `draw` (world_fb * scale fills
        // the content width exactly); a fixed divisor only fills the box at
        // the one resolution where it coincides with the scale (1440p) and
        // leaves black bands plus a detached HUD everywhere else.
        let (cx, _cy, cw, ch) = Self::content_rect(fb);
        self.content_x0 = cx;
        self.content_box = (cx, _cy, cw, ch);
        // The console band is 28.13% of the content height (measured on the
        // retail references); the world viewport is the rest, in logical units.
        let hud_h = ch as f32 * 0.2813;
        let div = scale.max(0.5);
        // An integer number of game px, stretched to the box width like
        // retail (`render::stretch_columns`).
        let view_w = ((cw as f32) / div).round().max(320.0) as i32;
        let view_h = (((ch as f32 - hud_h) / div).round()).max(180.0) as i32;
        self.view = (view_w, view_h);
    }

    /// Bottom of the playfield in surface pixels: the view's logical height
    /// at the UI scale (the console band below is the HUD).
    pub(super) fn playfield_bottom(&self) -> i32 {
        self.content_box.1 + (self.view.1 as f32 * self.ui_scale.max(0.5)).round() as i32
    }

    /// Show a retail `stat_txt.tbl` error line (1-based id) in the game.
    pub(super) fn show_game_error(&mut self, id: u16) {
        if let Some(text) = self.data.strings.get(id - 1) {
            self.game_error = Some((text.to_string(), Instant::now()));
        }
    }

    /// The retail error line of a refused order or production.
    pub(super) fn show_sim_error(&mut self, e: galaxycraft::sim::SimError) {
        let race = self.local_race();
        if let Some(id) = stat_error_id(e, race) {
            self.show_game_error(id);
        }
    }

    pub(super) fn status(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.status_until = Instant::now() + Duration::from_secs(4);
    }

    pub(super) fn update(&mut self) {
        // Registry, Options and result changes reach `profiles.bin` here.
        self.sync_profiles();
        // The Sound tab's volumes and checkboxes drive the mixer.
        let gains = self.game_options.mixer_gains();
        if self.volumes_sent != Some(gains) {
            self.audio.set_volumes(gains.0, gains.1);
            self.volumes_sent = Some(gains);
        }
        if self.intro {
            // The intro is the retail movie (138 s).  It ends when the player
            // reports the last frame, or on Enter/Esc; the old 5-second timer
            // truncated every intro after the streaming player landed.
            if self.intro_player.as_ref().is_some_and(|p| p.finished()) {
                self.end_intro();
            } else if self.intro_player.is_none()
                && self.intro_started.elapsed() > Duration::from_secs(5)
            {
                // Poster fallback (no ffmpeg / no install): keep the old timing.
                self.end_intro();
            }
            self.last_step = Instant::now();
            return;
        }
        if self.menu {
            // While hosting, keep announcing and watch for a peer that joins.
            self.lan_poll_host();
            return;
        }
        if self.briefing {
            self.update_briefing();
            return;
        }
        self.rotate_music();
        if self.game_result == Some(true) && self.next_map.is_none() {
            // Action 41 may have run after the winning condition; resolve as
            // soon as the trigger VM has had its chance.
            self.next_map = self.resolve_next_map();
        }
        // After the result the world runs on under the end dialog (and the
        // wait for the next game-end check); the score screen stops it.
        if let Some((t, key)) = self.score_leave
            && t.elapsed().as_secs_f32() >= SCORE_LEAVE_SECS
        {
            self.score_leave = None;
            self.finish_score(key);
            return;
        }
        if self.paused || (self.game_result.is_some() && !self.end_dialog) {
            self.last_step = Instant::now();
            return;
        }
        if self.replay.is_some() {
            self.update_replay();
            return;
        }
        if self.net.is_some() {
            self.update_net();
            return;
        }
        let now = Instant::now();
        let dt = now.duration_since(self.last_step).as_secs_f32().min(0.1);
        self.last_step = now;
        self.accumulator += dt * TICKS_PER_REAL_SECOND * self.speed;
        let mut steps = 0;
        while self.accumulator >= 1.0 && steps < 12 {
            self.step_once();
            self.accumulator -= 1.0;
            steps += 1;
        }
        self.after_steps();
    }

    /// One simulation tick with the AIs, as the frame loop runs it (shared
    /// with `--game-script` `ticks:N`).
    pub(super) fn step_once(&mut self) {
        // The end dialog waits for the next game-end check after the
        // result.
        if self.game_result.is_some() {
            self.end_dialog_hold = self.end_dialog_hold.saturating_sub(1);
        }
        {
            // Each AI attacks an enemy of its own (never a teammate).
            let local = self.local_player as u8;
            // Trigger actions 15/16 queue campaign AI scripts.
            if let Some(t) = self.world.triggers.as_mut() {
                let pending = std::mem::take(&mut t.pending_ai_scripts);
                for (player, id, at) in pending {
                    if self.campaign_ais.iter().any(|c| c.player == player as u8) {
                        continue;
                    }
                    if let Some(script) = self
                        .data
                        .aiscripts
                        .get(&id)
                        .or_else(|| self.data.bwscripts.get(&id))
                    {
                        let mut ai = CampaignAi::new(player as u8, script);
                        ai.location = at;
                        self.campaign_ais.push(ai);
                    }
                }
            }
            for ai in &mut self.campaign_ais {
                let enemy_base = self.world.enemy_town_hall(ai.player, local);
                ai.update(&mut self.world, enemy_base);
            }
            for ai in &mut self.ais {
                if self.campaign_ais.iter().any(|c| c.player == ai.player) {
                    continue;
                }
                let enemy_base = self.world.enemy_town_hall(ai.player, local);
                ai.update(&mut self.world, enemy_base);
            }
            self.world.step();
        }
    }

    /// Client follow-ups after the ticks of a frame.
    pub(super) fn after_steps(&mut self) {
        self.follow_trigger_portraits();
        let _ = self.follow_trigger_cues();
        self.follow_rescues();
        // Drop dead selections.
        self.selected
            .retain(|id| self.world.unit(*id).is_some_and(|u| u.alive));
        self.check_game_result();
    }

    /// End the game for the local player: defeat when it is out, victory
    /// when its side won (allies win together) or every enemy is out.
    /// Shared by the local loop and the LAN lockstep.
    pub(super) fn check_game_result(&mut self) {
        // `staying alive`: the mission never ends by itself; the player can
        // keep playing after the victory/defeat condition.
        if self
            .world
            .players
            .get(self.local_player)
            .is_some_and(|p| p.cheats.stay_alive)
        {
            return;
        }
        if self.game_result.is_none() && self.observing() {
            // An observer neither wins nor loses: the game ends once one
            // side is left, and nothing is recorded in the profile.
            let over = self
                .mp
                .setup
                .as_ref()
                .is_some_and(|s| s.sides_left(&self.world).len() <= 1);
            if over {
                self.game_result = Some(true);
                self.end_dialog = true;
                self.end_dialog_hold = frames_to_end_check(self.world.tick);
                self.result_tick = self.world.tick;
                self.score_tab = 0;
                self.score_epoch = None;
                self.score_count_epoch = None;
                self.next_map = None;
                self.result_panel =
                    GlueArt::load_score_panel(&self.assets_root, self.local_race(), true);
                self.result_fit = None;
            }
            return;
        }
        if self.game_result.is_none() {
            let local = self.local_player as u8;
            let defeat = self
                .world
                .players
                .get(self.local_player)
                .is_some_and(|p| p.defeated);
            let victory = self.world.has_won(local)
                || (self.world.players.len() > 1
                    && self
                        .world
                        .players
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| self.world.is_enemy(local, *i as u8))
                        .all(|(_, p)| p.defeated));
            if defeat || victory {
                let won = !defeat && victory;
                self.game_result = Some(won);
                self.end_dialog = true;
                self.end_dialog_hold = frames_to_end_check(self.world.tick);
                self.result_tick = self.world.tick;
                self.score_tab = 0;
                self.score_epoch = None;
                self.score_count_epoch = None;
                if won {
                    self.next_map = self.resolve_next_map();
                }
                self.record_game_result(won);
                self.play(if won { "victory" } else { "defeat" });
                self.result_panel =
                    GlueArt::load_score_panel(&self.assets_root, self.local_race(), won);
                self.result_fit = None;
                self.play_result_music(won);
            }
        }
    }

    /// Advance the render-only facing used by "Smooth Unit Turning".  The
    /// simulation keeps its exact byte facing; only the drawn sprite turns
    /// through the intermediate directions, at a constant rate per tick so
    /// captures and replays stay deterministic.
    pub(super) fn update_smooth_facings(&mut self) {
        if !self.game_options.video_smooth_turning {
            self.smooth_facing.clear();
            self.smooth_facing_tick = self.world.tick;
            return;
        }
        let tick = self.world.tick;
        if tick < self.smooth_facing_tick {
            // A new mission / loaded save restarted the clock.
            self.smooth_facing.clear();
        }
        let dt = tick.saturating_sub(self.smooth_facing_tick) as f32;
        self.smooth_facing_tick = tick;
        for u in &self.world.units {
            if !u.alive {
                continue;
            }
            let entry = self.smooth_facing.entry(u.id).or_insert(u.facing as f32);
            if dt > 0.0 {
                *entry = smooth_step_facing(*entry, u.facing, SMOOTH_TURN_RATE * dt);
            }
        }
        self.smooth_facing
            .retain(|id, _| self.world.unit(*id).is_some_and(|u| u.alive));
    }

    /// Actions per minute from the mission clock, using the same command
    /// count the score screen reports.
    pub(super) fn current_apm(&self) -> i64 {
        let actions = self
            .world
            .recorder
            .as_ref()
            .map_or(self.actions, |r| r.cmds.len() as u32);
        apm_value(actions, self.elapsed_ticks())
    }

    /// Game time for Elapsed Time displays: frozen at the result.
    pub(super) fn elapsed_ticks(&self) -> u64 {
        if self.game_result.is_some() {
            self.result_tick
        } else {
            self.world.tick
        }
    }
}
