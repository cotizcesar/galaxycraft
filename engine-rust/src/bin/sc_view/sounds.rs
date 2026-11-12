//! `impl App`: sounds (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    pub(super) fn play(&mut self, name: &str) {
        self.audio.play_named(&self.sounds, name);
        self.sound_log.push(name.to_string());
        if self.sound_log.len() > 512 {
            self.sound_log.drain(..256);
        }
    }

    /// Play the retail acknowledgement for the selected unit, cycling takes.
    pub(super) fn play_ack(&mut self) {
        // Retail's Sound Options: "Unit Speech" disables unit voices and
        // "Unit Acknowledgments" disables the command replies on their own.
        if !self.game_options.unit_speech || !self.game_options.unit_acks {
            return;
        }
        // Retail picks one of the unit's replies at random.
        self.ack_variant = random_take(self.ack_variant);
        let take = self.ack_variant;
        if let Some(u) = self.first_selected() {
            let unit_type = u.type_id;
            if let Some(samples) = self.voices.cue(unit_type, VoiceCue::Yes, take) {
                self.audio.play(Some(&samples));
                self.start_portrait_talk(unit_type, samples.len());
                return;
            }
        }
        let race = self
            .world
            .players
            .get(self.local_player)
            .map(|p| p.race)
            .unwrap_or(Race::Terran);
        self.audio.play(self.sounds.race_yes(race));
    }

    /// The selected unit speaks for `samples` mono 44.1 kHz samples: its
    /// portrait talks until the line ends.
    pub(super) fn start_portrait_talk(&mut self, unit_type: u16, samples: usize) {
        let secs = samples as f64 / 44_100.0;
        self.portrait_talk = Some((unit_type, Instant::now() + Duration::from_secs_f64(secs)));
        self.portrait_talk_started = true;
    }

    /// Trigger Transmissions (action 7) and Talking Portrait (29) of the
    /// local player make the named unit's portrait talk for the action's
    /// duration, whatever is selected.
    pub(super) fn follow_trigger_portraits(&mut self) {
        let Some(t) = self.world.triggers.as_ref() else {
            return;
        };
        let (latest, seen) = latest_trigger_portrait(
            &t.transmissions,
            &t.talking_portraits,
            self.trigger_portraits_seen,
            self.local_player,
        );
        self.trigger_portraits_seen = seen;
        if let Some((unit, ms)) = latest {
            self.start_portrait_talk(unit, ms as usize * 441 / 10);
        }
    }

    /// Show and play the trigger cues of the local player that ran since the
    /// last frame: Transmission and Display Text lines (gated by the
    /// Subtitles option unless the action says "always display") and the
    /// map WAV of Transmission / Play WAV.
    /// Returns `(text lines added, sound files played)`.
    pub(super) fn follow_trigger_cues(&mut self) -> (usize, Vec<PathBuf>) {
        use galaxycraft::trigger::TriggerCue;
        let mut done = (0, Vec::new());
        // Center View (action 10) moves the local player's camera.
        if let Some(t) = self.world.triggers.as_ref() {
            let from = self.center_views_seen.min(t.center_views.len());
            let target = t.center_views[from..]
                .iter()
                .rev()
                .find(|(p, _)| *p == self.local_player)
                .and_then(|(_, l)| t.location_center(*l));
            self.center_views_seen = t.center_views.len();
            if let Some((x, y)) = target {
                self.camera = (x - self.view.0 / 2, y - self.view.1 / 2);
                self.clamp_camera();
            }
        }
        let Some(t) = self.world.triggers.as_ref() else {
            return done;
        };
        let from = if self.trigger_cues_seen > t.cues.len() {
            0
        } else {
            self.trigger_cues_seen
        };
        let new: Vec<TriggerCue> = t.cues[from..]
            .iter()
            .copied()
            .filter(|c| c.player() == self.local_player)
            .collect();
        self.trigger_cues_seen = t.cues.len();
        let now = self.trigger_text_epoch.elapsed().as_millis() as u64;
        for cue in new {
            match cue {
                TriggerCue::Text {
                    string, always, ms, ..
                } => {
                    if !(always || self.game_options.subtitles) {
                        continue;
                    }
                    if let Some(raw) = self.scenario.string(string) {
                        let life = galaxycraft::transmission::text_life_ms(
                            ms,
                            galaxycraft::transmission::text_chars(raw),
                        );
                        let shown = if self.game_options.game_language_filter {
                            mature_language_filter(raw)
                        } else {
                            raw.to_string()
                        };
                        self.trigger_text.push(&shown, now, life);
                        done.0 += galaxycraft::transmission::clean_lines(raw).len();
                    }
                }
                TriggerCue::Wav { string, .. } => {
                    let path = self
                        .scenario
                        .string(string)
                        .zip(self.maps.get(self.map_index))
                        .and_then(|(raw, map)| {
                            galaxycraft::transmission::find_map_wav(
                                &map.path,
                                raw,
                                self.game_options.original_voice_overs,
                            )
                        });
                    let samples = path
                        .as_ref()
                        .and_then(|p| std::fs::read(p).ok())
                        .and_then(|bytes| galaxycraft::audio::decode_sound(&bytes));
                    if let (Some(path), Some(samples)) = (path, samples) {
                        self.audio.play(Some(&samples));
                        done.1.push(path);
                    }
                }
            }
        }
        done
    }

    /// Retail rescue sound (`sound/misc/<race>rescue.wav`, by the local
    /// player's race) when the local player rescues units.
    pub(super) fn follow_rescues(&mut self) {
        let rescues = &self.world.rescues;
        let from = self.rescues_seen.min(rescues.len());
        let mine = rescues[from..]
            .iter()
            .any(|&(_, owner)| owner as usize == self.local_player);
        self.rescues_seen = rescues.len();
        if mine {
            self.play(rescue_cue(self.local_race()));
        }
    }

    /// Unit whose portrait the HUD shows: the one talking for a trigger, else
    /// the first selected unit.
    pub(super) fn portrait_unit(&self) -> Option<u16> {
        let talking = self
            .portrait_talk
            .filter(|&(_, until)| Instant::now() < until)
            .map(|(unit, _)| unit);
        talking.or_else(|| self.first_selected().map(|u| u.type_id))
    }

    /// Retail "ready" line when a unit is first selected.
    /// A selection click: "what?" lines, then the annoyed ones when the
    /// same unit is clicked again and again (gaps under 3 s), as retail
    /// (`voice::selection_cue`).  It played the "ready" line before.
    pub(super) fn play_selected(&mut self) {
        if !self.game_options.unit_speech {
            return;
        }
        self.voices
            .set_classic(self.game_options.original_1998_campaign);
        let Some(u) = self.first_selected() else {
            return;
        };
        let (id, unit_type) = (u.id, u.type_id);
        if !building_sound_allowed(&self.game_options, self.data.unit(unit_type).is_building()) {
            return;
        }
        let streak = match self.select_streak {
            Some((last, n, at)) if last == id && at.elapsed() < Duration::from_secs(3) => n + 1,
            _ => 1,
        };
        let pissed = self.voices.takes(unit_type, VoiceCue::Pissed);
        let (cue, take, reset) = galaxycraft::voice::selection_cue(streak, pissed);
        // The "what?" line is a random one; the annoyed lines keep their
        // order.
        let take = if cue == VoiceCue::What {
            self.ack_variant = random_take(self.ack_variant);
            self.ack_variant
        } else {
            take
        };
        self.select_streak = Some((id, if reset { 0 } else { streak }, Instant::now()));
        if let Some(samples) = self.voices.cue(unit_type, cue, take) {
            self.audio.play(Some(&samples));
            self.start_portrait_talk(unit_type, samples.len());
        }
    }

    /// Voices and sounds that follow the world: a unit of the player's that
    /// finishes training (or hatches) says its "ready" line, and the
    /// animations' `playsnd` sounds (gunfire, deaths, explosions) play
    /// when they happen on screen where the player can see.
    pub(super) fn play_world_sounds(&mut self) {
        self.voices
            .set_classic(self.game_options.original_1998_campaign);
        let local = self.local_player as u8;
        // A new map or a reload: remember what is there, say nothing.
        let first_look = self.unit_types_seen.is_empty() || self.world.tick <= 2;
        let mut ready: Option<u16> = None;
        let mut now = std::collections::HashMap::new();
        for u in self.world.units.iter().filter(|u| u.alive) {
            now.insert(u.id, u.type_id);
            if first_look || u.owner != local || !u.completed {
                continue;
            }
            let ty = self.data.unit(u.type_id);
            if ty.is_building() && !self.game_options.building_sounds {
                continue;
            }
            let was = self.unit_types_seen.get(&u.id).copied();
            if was != Some(u.type_id) && was != Some(galaxycraft::ids::unit::LARVA) {
                ready = Some(u.type_id);
            }
        }
        self.unit_types_seen = now;
        if let Some(t) = ready
            && self.game_options.unit_speech
            && let Some(samples) = self.voices.cue(t, VoiceCue::Ready, 0)
        {
            self.audio.play(Some(&samples));
        }
        let (vw, vh) = self.view;
        let (cx, cy) = self.camera;
        let mut played: Vec<u16> = Vec::new();
        for (id, x, y) in self.animator.take_sounds() {
            let on_screen = x >= cx - 64 && x <= cx + vw + 64 && y >= cy - 64 && y <= cy + vh + 64;
            if !on_screen
                || (self.fog_on() && !self.world.is_visible_to(local, x, y))
                || played.contains(&id)
                || played.len() >= 4
            {
                continue;
            }
            played.push(id);
            if let Some(samples) = self.voices.clip(id) {
                self.audio.play(Some(&samples));
            }
        }
    }

    pub(super) fn play_music(&mut self, name: &str) {
        if self.music_track == name {
            return;
        }
        if let Some(track) = self.music.track(name) {
            self.audio.play_music(Some(track));
            self.music_track = name.to_string();
            self.music_started = Instant::now();
        } else {
            self.music_track = name.to_string();
        }
    }

    pub(super) fn start_menu_music(&mut self) {
        self.play_music("title");
    }

    pub(super) fn race_music_prefix(&self) -> &'static str {
        match self.local_race() {
            Race::Zerg => "z",
            Race::Protoss => "p",
            _ => "t",
        }
    }

    /// Sound tab "Sound Plays While In Background": when off, pause the
    /// mixer while the window is not focused.
    pub(super) fn update_audio_background(&mut self, window_active: bool) {
        let allowed = self.game_options.sound_background || window_active;
        if allowed != self.audio_background {
            self.audio_background = allowed;
            self.audio.set_background(allowed);
        }
    }

    pub(super) fn start_mission_music(&mut self) {
        let prefix = self.race_music_prefix();
        let name = format!("{}rdyroom", prefix);
        self.music_phase = 0;
        self.play_music(&name);
        if let Some(samples) = self.voices.announcer_clip(&[
            "gamestart_1.wav",
            "gamestart_2.wav",
            "gamestart_3.wav",
            "gamestart.wav",
        ]) {
            self.audio.play(Some(&samples));
        }
    }

    /// After the ready room, rotate the four shipped battle tracks, then the
    /// victory/defeat cue is selected by the result path.
    pub(super) fn rotate_music(&mut self) {
        if self.music_track.is_empty() {
            self.start_mission_music();
            return;
        }
        if self.music_track.ends_with("vict") || self.music_track.ends_with("defeat") {
            return;
        }
        if self.music_started.elapsed() < Duration::from_secs(32) {
            return;
        }
        let race = match self.local_race() {
            Race::Zerg => "zerg",
            Race::Protoss => "protoss",
            _ => "terran",
        };
        self.music_phase = self.music_phase % 4 + 1;
        let name = format!("{}{}", race, self.music_phase);
        self.play_music(&name);
    }

    pub(super) fn play_result_music(&mut self, won: bool) {
        let prefix = self.race_music_prefix();
        let name = format!("{}{}", prefix, if won { "vict" } else { "defeat" });
        self.music_phase = 0;
        self.play_music(&name);
    }
}
