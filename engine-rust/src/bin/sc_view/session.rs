//! `impl App`: session (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    pub(super) fn load_map(&mut self, index: usize) {
        let Some(entry) = self.maps.get(index).cloned() else {
            return;
        };
        // A map started from the map list is not a lobby game.
        self.mp.setup = None;
        let raw = match std::fs::read(&entry.path) {
            Ok(raw) => raw,
            Err(e) => {
                self.status(format!("Cannot read {}: {e}", entry.path.display()));
                return;
            }
        };
        let scenario = match Scenario::parse(&raw) {
            Ok(sc) => sc,
            Err(e) => {
                self.status(format!("Invalid map: {e}"));
                return;
            }
        };
        let Some(tileset) = Tileset::load(&self.tiles_dir, scenario.era, &scenario) else {
            self.status(format!("Missing {} tileset assets", era_name(scenario.era)));
            return;
        };
        let vf4 = match std::fs::read(
            self.tiles_dir
                .join(format!("{}.vf4", era_name(scenario.era))),
        ) {
            Ok(v) => v,
            Err(e) => {
                self.status(format!("Missing vf4: {e}"));
                return;
            }
        };
        let slots = scenario.player_slots();
        self.local_player = self
            .melee_slot
            .filter(|&s| !scenario.has_placed_force() && slots.iter().any(|p| p.slot == s))
            .or_else(|| slots.iter().find(|s| s.active && s.human).map(|s| s.slot))
            .unwrap_or(0);
        let races = races_for(&scenario, &slots, &self.races, self.menu_race);
        // The console follows the race actually played: a melee slot's SIDE
        // byte is "user select" and would always fall back to Terran.
        let console_race = races
            .get(self.local_player)
            .copied()
            .unwrap_or(Race::Terran);
        self.console_race = console_race;
        self.console = ConsoleArt::load(
            &self.assets_root,
            console_race,
            self.game_options.game_console_skin,
            self.game_options.game_unit_skin,
        );
        let mut world = World::from_scenario(
            self.data.clone(),
            &scenario,
            &galaxycraft::map::TileData::new(tileset.cv5(), &vf4),
            &races,
        );
        apply_slots(&mut world, &slots);
        // Melee CHKs mark every slot as computer (OWNR 6); the local player
        // is the human one, never driven by the AI.
        if let Some(p) = world.players.get_mut(self.local_player) {
            p.is_computer = false;
        }
        world.enable_fog();
        let local = self.local_player;
        self.ais = slots
            .iter()
            .filter(|s| s.computer && s.slot != local && skirmish_ai(&world, s.slot))
            .map(|s| SimpleAi::new(s.slot as u8))
            .collect();
        self.campaign_ais.clear();
        // Campaign missions carry a retail `est*.txt` briefing script and a
        // retail backdrop; use them when present and only fall back to the
        // map-authored STR copy for missions without one.
        let mut briefing_pages = Vec::new();
        let mut briefing_art = None;
        if let Some(meta) = &entry.campaign {
            if let Some(script_path) = &meta.script
                && let Ok(text) = std::fs::read_to_string(script_path)
            {
                briefing_pages = galaxycraft::campaign::parse_script(&text)
                    .pages
                    .into_iter()
                    .filter(|p| !p.text.trim().is_empty() || p.video.is_some())
                    .collect();
            }
            if let Some(background) = &meta.background {
                briefing_art = std::fs::read(background)
                    .ok()
                    .and_then(|bytes| galaxycraft::dds::parse(&bytes));
            }
            if briefing_art.is_none()
                && let Some(page) = briefing_pages.first()
                && let Some(path) = &page.background
            {
                let file = self.tiles_dir.join("../glue").join(path);
                briefing_art = std::fs::read(file)
                    .ok()
                    .and_then(|bytes| galaxycraft::dds::parse(&bytes));
            }
        }
        let show_retail = !briefing_pages.is_empty();
        let show_briefing =
            show_retail || (scenario.has_placed_force() && briefing_content(&scenario).is_some());
        self.briefing_pages = briefing_pages;
        self.briefing_page = 0;
        self.briefing_page_epoch = Instant::now();
        // The ready room follows the establishing shot when the mission has
        // an `MBRF` script and its race's room art is installed.
        let human = scenario
            .player_slots()
            .iter()
            .position(|s| s.active && s.human)
            .unwrap_or(self.local_player);
        let human_race = races.get(human).copied().unwrap_or(Race::Terran);
        self.ready_race = human_race;
        let art_ready = self.glue.as_ref().is_some_and(|g| match human_race {
            Race::Terran => g.ready_t.is_some(),
            Race::Zerg => g.ready_z.is_some(),
            Race::Protoss => g.ready_p.is_some(),
            Race::Neutral => false,
        });
        self.ready_script = art_ready
            .then(|| galaxycraft::ready_room::ReadyRoom::from_scenario(&scenario, human))
            .flatten();
        self.ready = None;
        self.ready_at = None;
        self.briefing_art = briefing_art;
        self.briefing_fit = None;
        // Decoded briefing intermission: the first page that names a
        // `</VIDEO>` file gets its poster frame from the materialised
        // `videos/` cache, so the briefing screen shows the retail movie
        // instead of only the INTERMISSION caption.
        self.briefing_poster = None;
        self.briefing_video = None;
        if let Some(store) = &self.video_store {
            for page in self.briefing_pages.iter() {
                if let Some(video) = &page.video
                    && let Some((_, poster)) = store.poster(video)
                {
                    self.briefing_poster = Some(poster);
                    self.briefing_video = Some(video.clone());
                    break;
                }
            }
        }
        if self
            .hd_terrain
            .as_ref()
            .is_none_or(|_| self.scenario.era != scenario.era)
        {
            self.hd_terrain = galaxycraft::render::HdTerrain::load(&self.assets_root, scenario.era);
        }
        self.scenario = scenario;
        self.tileset = tileset;
        self.sprites.set_palette(self.tileset.palette.clone());
        self.chk_raw = raw;
        self.last_races = races.clone();
        world.recorder = Some(Recorder::new());
        self.world = world;
        self.fog_mem.seen.clear();
        self.apply_special_rules();
        self.trigger_portraits_seen = (0, 0);
        self.trigger_cues_seen = 0;
        self.center_views_seen = 0;
        self.actions = 0;
        self.smooth_facing.clear();
        self.smooth_facing_tick = self.world.tick;
        self.rescues_seen = 0;
        self.trigger_text.clear();
        self.replay = None;
        self.save_sel = 0;
        self.map_index = index;
        self.menu_tab = if index < self.campaign_len { 0 } else { 1 };
        self.menu_index = if index < self.campaign_len {
            index
        } else {
            index - self.campaign_len
        };
        self.menu = false;
        self.briefing = show_briefing;
        self.paused = false;
        self.game_result = None;
        self.next_map = None;
        self.selected.clear();
        self.groups = Default::default();
        self.build_mode = None;
        self.command_list.clear();
        self.build_list.clear();
        self.attack_mode = false;
        self.patrol_mode = false;
        self.camera = (0, 0);
        self.minimap_cache = None;
        self.speed = 1.0;
        self.accumulator = 0.0;
        self.last_step = Instant::now();
        self.center_on_base();
        self.refresh_commands();
        self.result_panel = None;
        self.result_fit = None;
        self.show_stats = false;
        self.show_leaderboard = false;
        self.intro = false;
        self.start_mission_music();
        // Retail shows no map banner at game start (134905/135020/135051).
        self.game_dialog = None;
        // A single-player game opens with StarCraft Tips while "Show Tips
        // at Startup" is on (2026-11-07 recording, 478 s).
        if self.startup_tips && self.game_options.tips_at_startup && !self.tips_text.is_empty() {
            self.tip_index %= self.tips_text.len();
            self.open_game_dialog(GameDialog::Tips);
        }
    }

    pub(super) fn restart(&mut self) {
        let index = self.map_index;
        self.load_map(index);
    }

    /// Start a campaign mission.  The cinematics between missions come from
    /// the retail `MenuEntry` table (`campaign_progress::after_victory`);
    /// the first mission of a Brood War campaign opens with
    /// `starxintr.webm` (INFERENCE kept from the 2026-10-29 intro work).
    pub(super) fn start_campaign_mission(&mut self, index: usize) {
        let first_expansion = self.map_mission(index).is_some_and(|m| {
            galaxycraft::campaign_progress::Campaign::of_mission(m).is_some_and(|c| {
                c.index() >= galaxycraft::campaign_progress::Campaign::ExpProtoss.index()
                    && c.mission_range().0 == m
            })
        });
        if first_expansion {
            self.begin_intro(
                vec![galaxycraft::video::INTRO_EXPANSION.to_string()],
                Some(index),
            );
        } else {
            self.load_map(index);
        }
    }

    /// Current recording (empty when nothing was issued yet).
    pub(super) fn take_recording(&self) -> Recording {
        let mut rec = Recording::new(0x1234_5678_9abc_def0, [0u8; 32]);
        if let Some(r) = self.world.recorder.as_ref() {
            rec.cmds = r.cmds.clone();
        }
        rec
    }

    pub(super) fn save_list(&self, replays: bool) -> Vec<PathBuf> {
        let dir = if replays {
            &self.replays_dir
        } else {
            &self.saves_dir
        };
        let ext = if replays { "gcrep" } else { "gcsave" };
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some(ext) {
                    out.push(p);
                }
            }
        }
        out.sort();
        out
    }

    pub(super) fn slot_label(path: &Path) -> String {
        match SaveFile::load(path) {
            Some(s) => format!(
                "{}  tick {}  ({} cmds)",
                s.map_name,
                s.tick,
                s.rec.cmds.len()
            ),
            None => format!("{} (corrupt)", path.display()),
        }
    }

    /// Save Game dialog: the save under the typed name (Boot Camp 131 s);
    /// the same name overwrites.
    pub(super) fn save_game_named(&mut self, name: &str) {
        self.write_save(Some(name));
    }

    pub(super) fn write_save(&mut self, name: Option<&str>) {
        let rec = self.take_recording();
        let tick = self.world.tick;
        let save = SaveFile {
            map_name: self
                .maps
                .get(self.map_index)
                .map(|m| m.name.clone())
                .unwrap_or_default(),
            map_index: Some(self.map_index),
            map_chk: self.chk_raw.clone(),
            races: self.last_races.clone(),
            local_player: self.local_player,
            tick,
            rec,
            setup: self.mp.setup.clone(),
        };
        let _ = std::fs::create_dir_all(&self.saves_dir);
        let n = self.save_list(false).len();
        let path = match name {
            Some(name) => self
                .saves_dir
                .join(format!("{}.gcsave", save_file_stem(name))),
            None => self.saves_dir.join(format!("save-{:03}.gcsave", n + 1)),
        };
        match save.save(&path) {
            Ok(()) => self.status(format!(
                "Saved {} (tick {}, hash {:x})",
                path.display(),
                tick,
                state_hash(&self.world)
            )),
            Err(e) => self.status(format!("Save failed: {e}")),
        }
        self.play("ui_click");
    }

    /// Save the current game as a replay; returns the file written.
    pub(super) fn save_replay(&mut self) -> Option<PathBuf> {
        let rec = self.take_recording();
        let tick = self.world.tick;
        let save = SaveFile {
            map_name: self
                .maps
                .get(self.map_index)
                .map(|m| m.name.clone())
                .unwrap_or_default(),
            map_index: Some(self.map_index),
            map_chk: self.chk_raw.clone(),
            races: self.last_races.clone(),
            local_player: self.local_player,
            tick,
            rec,
            setup: self.mp.setup.clone(),
        };
        let _ = std::fs::create_dir_all(&self.replays_dir);
        let n = self.save_list(true).len();
        let path = self.replays_dir.join(format!("replay-{:03}.gcrep", n + 1));
        let written = match save.save(&path) {
            Ok(()) => {
                self.status(format!("Replay saved {}", path.display()));
                Some(path)
            }
            Err(e) => {
                self.status(format!("Replay save failed: {e}"));
                None
            }
        };
        self.play("ui_click");
        written
    }

    /// Rebuild a world from a save file (shared by load-game and replay).
    pub(super) fn build_from_save(&mut self, save: &SaveFile) -> bool {
        let scenario = match Scenario::parse(&save.map_chk) {
            Ok(sc) => sc,
            Err(e) => {
                self.status(format!("Bad save map: {e}"));
                return false;
            }
        };
        let Some(tileset) = Tileset::load(&self.tiles_dir, scenario.era, &scenario) else {
            self.status("Missing tileset assets".to_string());
            return false;
        };
        let vf4 = match std::fs::read(
            self.tiles_dir
                .join(format!("{}.vf4", era_name(scenario.era))),
        ) {
            Ok(v) => v,
            Err(e) => {
                self.status(format!("Missing vf4: {e}"));
                return false;
            }
        };
        let slots = scenario.player_slots();
        let occupied = save.setup.as_ref().map(|s| s.occupied());
        let mut world = World::from_scenario_occupied(
            self.data.clone(),
            &scenario,
            &galaxycraft::map::TileData::new(tileset.cv5(), &vf4),
            &save.races,
            occupied.as_deref(),
        );
        apply_slots(&mut world, &slots);
        // A lobby game: its setup decides occupants, teams and computers
        // (the CHK's OWNR bytes describe the map, not this game).
        if let Some(setup) = &save.setup {
            setup.apply(&mut world);
            self.ais = setup
                .slots
                .iter()
                .enumerate()
                .filter(|(_, s)| s.occupant == galaxycraft::multiplayer::SetupOccupant::Computer)
                .map(|(i, _)| SimpleAi::new(i as u8))
                .collect();
            self.campaign_ais.clear();
        }
        world.enable_fog();
        world.rng = save.rec.seed;
        galaxycraft::replay::replay_to(&mut world, &save.rec, save.tick);
        if self
            .hd_terrain
            .as_ref()
            .is_none_or(|_| self.scenario.era != scenario.era)
        {
            self.hd_terrain = galaxycraft::render::HdTerrain::load(&self.assets_root, scenario.era);
        }
        self.scenario = scenario;
        self.tileset = tileset;
        self.sprites.set_palette(self.tileset.palette.clone());
        if let Some(index) = self.saved_map_index(save) {
            self.map_index = index;
            self.menu_tab = usize::from(index >= self.campaign_len);
            self.menu_index = if index < self.campaign_len {
                index
            } else {
                index - self.campaign_len
            };
        }
        self.chk_raw = save.map_chk.clone();
        self.last_races = save.races.clone();
        // A lobby game may be watched: one past the last slot is the
        // observer (`observing`), which owns nothing and sees everything.
        self.local_player = if save.setup.is_some() {
            save.local_player.min(world.players.len())
        } else {
            save.local_player.min(world.players.len().saturating_sub(1))
        };
        self.console_race = if save.setup.is_some() {
            world
                .players
                .get(self.local_player)
                .map(|p| p.race)
                .unwrap_or(Race::Terran)
        } else {
            slots
                .iter()
                .find(|s| s.slot == self.local_player)
                .map(|s| s.race(Race::Terran))
                .unwrap_or(Race::Terran)
        };
        self.mp.setup = save.setup.clone();
        self.console = ConsoleArt::load(
            &self.assets_root,
            self.console_race,
            self.game_options.game_console_skin,
            self.game_options.game_unit_skin,
        );
        self.world = world;
        self.fog_mem.seen.clear();
        self.apply_special_rules();
        self.trigger_portraits_seen = (0, 0);
        self.trigger_cues_seen = 0;
        self.center_views_seen = 0;
        self.actions = 0;
        self.smooth_facing.clear();
        self.smooth_facing_tick = self.world.tick;
        // A restored game does not replay the rescues it already had.
        self.rescues_seen = self.world.rescues.len();
        self.trigger_text.clear();
        self.menu = false;
        self.briefing = false;
        self.paused = false;
        self.game_result = None;
        self.next_map = None;
        self.selected.clear();
        self.groups = Default::default();
        self.build_mode = None;
        self.command_list.clear();
        self.build_list.clear();
        self.attack_mode = false;
        self.patrol_mode = false;
        self.camera = (0, 0);
        self.minimap_cache = None;
        self.speed = 1.0;
        self.accumulator = 0.0;
        self.last_step = Instant::now();
        self.center_on_base();
        self.refresh_commands();
        self.result_panel = None;
        self.result_fit = None;
        self.show_stats = false;
        self.show_leaderboard = false;
        self.intro = false;
        self.start_mission_music();
        true
    }

    pub(super) fn load_save(&mut self, path: &Path, as_replay: bool) {
        let Some(save) = SaveFile::load(path) else {
            self.status(format!("Cannot load {}", path.display()));
            return;
        };
        let tick = save.tick;
        if !self.build_from_save(&save) {
            return;
        }
        if as_replay {
            self.ais.clear();
            self.campaign_ais.clear();
            self.replay = Some(ReplayPlayer::new(save));
            self.status(format!(
                "Replaying ({} cmds)  SPACE: pause  +/-: speed  ESC: stop",
                self.replay.as_ref().unwrap().save.rec.cmds.len()
            ));
        } else {
            self.replay = None;
            // Keep recording on top of the loaded commands.
            if let Some(r) = self.world.recorder.as_mut() {
                r.cmds = save.rec.cmds.clone();
            }
            self.status(format!(
                "Loaded (tick {tick}, hash {:x})",
                state_hash(&self.world)
            ));
        }
        self.play("ui_swish");
    }

    /// Index of the profile in use: the one chosen in the Registry, else the
    /// first ID (the selection was deleted or never made).
    pub(super) fn active_index(&self) -> Option<usize> {
        self.profile_store
            .active
            .filter(|&i| i < self.profile_store.profiles.len())
            .or_else(|| (!self.profile_store.profiles.is_empty()).then_some(0))
    }

    /// Name of the profile in use (`Player` when the Registry is empty).
    pub(super) fn active_name(&self) -> String {
        self.active_index()
            .map(|i| self.profile_store.profiles[i].name.clone())
            .unwrap_or_else(|| "Player".into())
    }

    /// Progress of the profile chosen in the Registry.
    pub(super) fn progress(&self) -> galaxycraft::campaign_progress::Progress {
        self.active_index()
            .map(|i| self.profile_store.profiles[i].progress)
            .unwrap_or_default()
    }

    pub(super) fn set_progress(&mut self, progress: galaxycraft::campaign_progress::Progress) {
        if let Some(i) = self.active_index() {
            self.profile_store.profiles[i].progress = progress;
        }
    }

    /// Registry `Ok`: the highlighted ID becomes the profile in use.
    pub(super) fn capture_active_profile(&mut self) {
        if self.profile_sel < self.profile_store.profiles.len() {
            self.profile_store.active = Some(self.profile_sel);
        }
    }

    /// Registry `New ID`: open the "Enter new character name:" popup
    /// (screenshot 2026-11-06 14:15:16); its Ok creates and highlights the
    /// typed ID.
    pub(super) fn registry_new_id(&mut self) {
        self.registry_entry = Some(galaxycraft::profile::NameEntry::default());
        self.play("ui_click");
    }

    /// Close the `New ID` popup; `ok` creates the typed ID.  A taken name
    /// closes it and shows "That name already exists!" (user screenshot);
    /// other refusals (store full) keep the popup open (INFERENCE: not
    /// captured).
    pub(super) fn registry_new_id_close(&mut self, ok: bool) {
        let Some(entry) = self.registry_entry.take() else {
            return;
        };
        if ok {
            match self.profile_store.create(entry.name(), unix_now()) {
                Ok(i) => self.profile_sel = i,
                Err(e) => match e.notice() {
                    Some(text) => self.registry_notice = Some(text),
                    None => {
                        self.registry_entry = Some(entry);
                        return;
                    }
                },
            }
        }
        self.play("ui_click");
    }

    /// Registry `Delete`: remove the highlighted ID (the last one stays).
    pub(super) fn registry_delete(&mut self) {
        if self.profile_store.profiles.len() > 1 {
            let sel = self.profile_sel.min(self.profile_store.profiles.len() - 1);
            self.profile_store.delete(sel);
            self.profile_sel = sel.min(self.profile_store.profiles.len() - 1);
        }
        self.play("ui_click");
    }

    /// Add the finished game to the active profile's statistics (and, on a
    /// campaign win, advance its progress).  Replays are not games played.
    pub(super) fn record_game_result(&mut self, won: bool) {
        if self.replay.is_some() {
            return;
        }
        let Some(i) = self.active_index() else {
            return;
        };
        if let Some(player) = self.world.players.get(self.local_player) {
            let game =
                galaxycraft::profile::GameRecord::from_player(player, won, self.elapsed_ticks());
            self.profile_store.profiles[i].stats.record(&game);
        }
        if won && let Some(mission) = self.map_mission(self.map_index) {
            self.profile_store.profiles[i].progress.record_win(mission);
        }
        self.sync_profiles();
    }

    /// Write `profiles.bin` when IDs, statistics, progress or settings
    /// changed.  Settings are taken once the Options dialog is closed, so a
    /// slider drag is one write, not one per frame.
    pub(super) fn sync_profiles(&mut self) {
        if !matches!(self.menu_screen, MenuScreen::Options { .. }) {
            self.flush_profiles();
        }
    }

    /// Write `profiles.bin` now if anything changed (also on exit, even with
    /// the Options dialog still open).
    pub(super) fn flush_profiles(&mut self) {
        if !self.persist_profiles {
            return;
        }
        self.profile_store.settings = self.game_options.clone();
        let bytes = self.profile_store.to_bytes();
        if bytes == self.profile_saved {
            return;
        }
        let Some(path) = profile_save_path() else {
            return;
        };
        match self.profile_store.save(&path) {
            Ok(()) => self.profile_saved = bytes,
            Err(e) => {
                eprintln!("profiles not saved to {}: {e}", path.display());
                // Do not retry every frame; the next change tries again.
                self.profile_saved = bytes;
            }
        }
    }

    /// Map-list index of a save: the stored index when it still names the
    /// same CHK, else the entry whose file is byte-identical to the embedded
    /// CHK (v1-v4 saves), else the saved display name.
    pub(super) fn saved_map_index(&self, save: &SaveFile) -> Option<usize> {
        let same_chk = |i: usize| {
            self.maps
                .get(i)
                .is_some_and(|m| std::fs::read(&m.path).is_ok_and(|raw| raw == save.map_chk))
        };
        save.map_index
            .filter(|&i| same_chk(i))
            .or_else(|| (0..self.maps.len()).find(|&i| same_chk(i)))
            .or_else(|| {
                if save.map_name.is_empty() {
                    None
                } else {
                    self.maps.iter().position(|m| m.name == save.map_name)
                }
            })
    }

    /// Map list index of a mapdata mission (`campaign/<dir>/<key>`).
    pub(super) fn campaign_map_index(&self, mission: u16) -> Option<usize> {
        let (dir, key) = galaxycraft::campaign_progress::mission_key(mission)?;
        self.maps.iter().position(|m| {
            m.campaign
                .as_ref()
                .is_some_and(|c| c.episode == dir && c.mission == key)
        })
    }

    /// Mapdata mission of the map at `index`, for progress bookkeeping.
    pub(super) fn map_mission(&self, index: usize) -> Option<u16> {
        let tag = self.maps.get(index)?.campaign.as_ref()?;
        (1..=64u16).find(|&n| {
            galaxycraft::campaign_progress::mission_key(n)
                .is_some_and(|(dir, key)| tag.episode == dir && tag.mission == key)
        })
    }

    /// Click on a campaign slot: a locked campaign asks first
    /// (`strRACE_ERR_*`), an unlocked one is entered.
    pub(super) fn select_campaign(&mut self, campaign: galaxycraft::campaign_progress::Campaign) {
        self.episode_sel = (campaign.episode() - 1) as usize;
        // The modal takes the pointer: the slot under it is no longer
        // hovered (live capture 08: Zerg dim under the prompt).
        self.episode_hover = None;
        if self.progress().blocker(campaign).is_some() {
            self.campaign_prompt = Some(campaign);
        } else {
            self.enter_campaign(campaign);
        }
    }

    /// Retail: a campaign with history opens `Select starting mission`
    /// (first entry selected, list at the top); without history it starts
    /// straight away.
    pub(super) fn enter_campaign(&mut self, campaign: galaxycraft::campaign_progress::Campaign) {
        self.episode_sel = (campaign.episode() - 1) as usize;
        let history = self.progress().history(campaign);
        // Live capture 21: the Terran slot is dim under the list modal.
        self.episode_hover = None;
        if history.len() > 1 {
            self.mission_list = galaxycraft::campaign_progress::MissionList::open(history.len());
            self.menu_screen = MenuScreen::Mission {
                episode: campaign.episode(),
            };
        } else if let Some(entry) = history.first() {
            self.start_history_entry(entry);
        }
    }

    /// Episode-screen command buttons (`glucmpgn` ids 5/10/11/9).
    pub(super) fn episode_command(&mut self, which: usize) {
        self.menu_screen = match which {
            0 => MenuScreen::SavedGames,
            1 => {
                // Play Custom opens the map tree at its root with the first
                // (2-player) map selected; the retail browser shows the
                // folder rows above it.
                self.lobby_dir.clear();
                self.lobby_top = 0;
                if let Some(ClassicRow::Map(i)) = self
                    .classic_rows()
                    .into_iter()
                    .find(|r| matches!(r, ClassicRow::Map(_)))
                {
                    self.map_index = i;
                }
                MenuScreen::Lobby {
                    map: Some(self.map_index),
                }
            }
            2 => MenuScreen::Replays,
            // Live client: Cancel returns to the Registry.
            _ => MenuScreen::Registry,
        };
    }

    /// Start a history entry: a cinematic entry plays its movie and then
    /// the mission it precedes.
    pub(super) fn start_history_entry(
        &mut self,
        entry: &galaxycraft::campaign_progress::HistEntry,
    ) {
        let Some(idx) = self.campaign_map_index(entry.mission) else {
            return;
        };
        self.map_index = idx;
        let movies = galaxycraft::campaign_progress::cinematic_movies(entry.cinematic);
        if movies.is_empty() {
            self.start_campaign_mission(idx);
        } else {
            self.begin_intro(movies.iter().map(|m| m.to_string()).collect(), Some(idx));
        }
    }

    pub(super) fn launch_mission(&mut self, episode: u8) {
        self.play("ui_click");
        let Some(campaign) = galaxycraft::campaign_progress::Campaign::from_episode(episode) else {
            return;
        };
        let history = self.progress().history(campaign);
        if let Some(entry) = history.get(self.mission_list.sel) {
            self.start_history_entry(entry);
        }
    }

    /// Walkthrough record of the campaign flow outside the front end:
    /// queued movies and the mission they lead to, the briefing, or the game.
    pub(super) fn flow_state(&self) -> String {
        let name = |i: Option<usize>| {
            i.and_then(|i| self.maps.get(i))
                .map(|m| m.name.clone())
                .unwrap_or_else(|| "-".into())
        };
        if self.intro {
            format!(
                "flow=intro clips={} pending={}",
                self.intro_clips.join("+"),
                name(self.intro_pending_map)
            )
        } else if let Some(room) = self.ready.as_ref().filter(|_| self.briefing) {
            let line = room
                .transmission
                .and_then(|l| self.scenario.string(l.string))
                .map(|t| t.trim().chars().take(24).collect::<String>())
                .unwrap_or_default();
            let objectives = room
                .objectives
                .and_then(|s| self.scenario.string(s))
                .and_then(|t| t.lines().find(|l| !l.trim().is_empty()).map(str::to_string))
                .unwrap_or_default();
            format!(
                "flow=ready map={} t={} portraits={:?} speaking={:?} line={line:?} objectives={:?} finished={}",
                name(Some(self.map_index)),
                self.ready_clock_ms(),
                room.portraits,
                room.speaking.map(|s| s.0),
                objectives.trim(),
                room.finished()
            )
        } else if self.briefing {
            let first = self
                .briefing_pages
                .iter()
                .map(|p| p.text.lines().find(|l| !l.trim().is_empty()).unwrap_or(""))
                .next()
                .unwrap_or("");
            format!(
                "flow=briefing map={} pages={} first={:?}",
                name(Some(self.map_index)),
                self.briefing_pages.len(),
                first.trim()
            )
        } else {
            format!(
                "flow=game map={} result={:?} next={} dialog={} tab={}",
                name(Some(self.map_index)),
                self.game_result,
                name(self.next_map),
                self.end_dialog,
                self.score_tab
            )
        }
    }

    /// Resolve the next mission.  Distribution missions (Enslavers) carry a
    /// `Next Scenario` trigger string; retail campaign missions advance
    /// through `arr/mapdata.tbl`, because the campaign progression is stored
    /// in the campaign definition rather than the CHK.
    pub(super) fn resolve_next_map(&self) -> Option<usize> {
        if let Some(index) = self.world.triggers.as_ref().and_then(|t| t.next_scenario) {
            let raw = self.scenario.strings.get(index as usize)?.trim();
            let key = scenario_file_key(raw);
            if !key.is_empty()
                && let Some(found) = self
                    .maps
                    .iter()
                    .position(|m| scenario_file_key(&m.name) == key)
            {
                return Some(found);
            }
        }
        // The mapdata order, not the storage order: Norad II is followed
        // by The Trump Card, never by the unused `terran07`.
        let mission = self.map_mission(self.map_index)?;
        let next = galaxycraft::campaign_progress::after_victory(mission)?.next?;
        self.campaign_map_index(next)
    }
}
