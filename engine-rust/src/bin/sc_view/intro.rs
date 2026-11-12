//! `impl App`: intro (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    /// Start a retail intro playlist.  `pending_map` is loaded when the last
    /// clip finishes (campaign pre-mission movies); otherwise the client
    /// returns to the main menu.
    pub(super) fn begin_intro(&mut self, clips: Vec<String>, pending_map: Option<usize>) {
        self.intro_clips = clips;
        self.intro_clip = 0;
        self.intro_pending_map = pending_map;
        self.intro = true;
        self.menu = false;
        self.briefing = false;
        self.intro_player = None;
        self.intro_probed = false;
        self.intro_caption_done = false;
        self.intro_audio = None;
        self.intro_started = Instant::now();
        self.intro_last = Instant::now();
        self.audio.play_music(None);
    }

    pub(super) fn handle_intro(&mut self, window: &Window) {
        let keys = window
            .get_keys_pressed(KeyRepeat::No)
            .into_iter()
            .chain(self.pad_frame.menu_pressed.iter().copied());
        for key in keys {
            match key {
                Key::Enter | Key::Space | Key::Escape => {
                    self.play("ui_swish");
                    self.advance_intro();
                    return;
                }
                _ => {}
            }
        }
        if window.get_mouse_down(MouseButton::Left) || self.pad_frame.lmb {
            self.advance_intro();
        }
    }

    /// Move to the next clip, or leave the intro when the playlist is done.
    pub(super) fn advance_intro(&mut self) {
        if self.intro_clip + 1 < self.intro_clips.len() {
            self.intro_clip += 1;
            self.intro_player = None;
            self.intro_probed = false;
            self.intro_caption_done = false;
            self.intro_audio = None;
            self.intro_started = Instant::now();
            self.intro_last = Instant::now();
            self.audio.play_music(None);
            return;
        }
        self.end_intro();
    }

    pub(super) fn end_intro(&mut self) {
        self.intro = false;
        self.intro_player = None;
        self.intro_clips.clear();
        if let Some(index) = self.intro_pending_map.take() {
            self.audio.play_music(None);
            self.load_map(index);
            return;
        }
        self.menu = true;
        self.start_menu_music();
    }

    pub(super) fn draw_intro(&mut self, fb: &mut Framebuffer) {
        // Retail opening playlists: `smk/blizzard.webm` then
        // `smk/starintr.webm` on first launch (`smk/starxintr.webm` for the
        // expansion), or one `Pre*`/episode movie during the campaign.  Each
        // clip streams from the materialised cache; an install without
        // `ffmpeg` falls back to the shipped title art.
        let (w, h) = (fb.width as usize, fb.height as usize);
        if self.intro_clips.is_empty() {
            self.end_intro();
            return;
        }
        let rel = self.intro_clips[self.intro_clip.min(self.intro_clips.len() - 1)].clone();
        let path = self
            .video_store
            .as_ref()
            .map(|store| store.root.join(&rel))
            .unwrap_or_default();
        // A campaign film may open with its location caption typed on
        // black (Wasteland Patrol, Backwater Station recording 653 s).
        if !self.intro_caption_done && self.intro_player.is_none() && !self.intro_probed {
            let stem = Path::new(&rel)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            match galaxycraft::video::cinematic_caption(&stem) {
                Some(lines)
                    if self.intro_started.elapsed().as_secs_f32()
                        < galaxycraft::video::caption_secs(lines) =>
                {
                    self.draw_cinematic_caption(fb, lines);
                    return;
                }
                _ => {
                    self.intro_caption_done = true;
                    self.intro_started = Instant::now();
                    self.intro_last = Instant::now();
                }
            }
        }
        if self.intro_player.is_none() && !self.intro_probed {
            self.intro_probed = true;
            if let Some(info) = galaxycraft::video::probe_info(&path) {
                let start_at = self.intro_started.elapsed().as_secs_f32();
                self.intro_player = galaxycraft::cinematic::Cinematic::open_at(
                    &path,
                    &info,
                    INTRO_FPS,
                    INTRO_WIDTH,
                    start_at,
                );
                self.intro_last = Instant::now();
                if info.has_audio {
                    let audio_path = path.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.intro_audio = Some(rx);
                    std::thread::spawn(move || {
                        if let Some(pcm) = galaxycraft::video::decode_audio_pcm(&audio_path) {
                            let _ = tx.send(pcm);
                        }
                    });
                }
            }
        }
        if let Some(rx) = &self.intro_audio
            && let Ok(pcm) = rx.try_recv()
        {
            self.audio.play_music(Some(std::sync::Arc::new(pcm)));
            // Mark the track so `start_menu_music` knows it must replace
            // the movie soundtrack when the intro ends or is skipped.
            self.music_track = "intro-movie".to_string();
            self.intro_audio = None;
        }
        let dt = self.intro_last.elapsed().as_secs_f32();
        self.intro_last = Instant::now();
        let mut frame = self
            .intro_player
            .as_mut()
            .and_then(|player| player.advance(dt));
        if frame.is_none() {
            // Fallback: the retail title art, cover-fitted.
            frame = self.intro_poster.clone();
        }
        if let Some(poster) = frame {
            draw_movie_frame(fb, &poster);
        } else {
            fb.clear(Rgb::new(0, 0, 0));
            let stale = self
                .intro_fit
                .as_ref()
                .map(|(fw, fh, _)| *fw != w || *fh != h)
                .unwrap_or(true);
            if stale {
                self.intro_fit = self
                    .glue
                    .as_ref()
                    .and_then(|g| g.title.as_ref())
                    .map(|art| (w, h, art.cover(w, h)));
            }
            if let Some((_, _, art)) = &self.intro_fit {
                for y in 0..h {
                    for x in 0..w {
                        let p = art.pixels[y * w + x];
                        fb.put(x as i32, y as i32, Rgb(p & 0x00ff_ffff));
                    }
                }
            }
        }
        self.draw_cinematic_subtitle(fb, &rel);
        if self.intro_player.as_ref().is_some_and(|p| p.finished()) {
            self.advance_intro();
        }
    }

    /// The typed caption: green on black at the lower left, characters
    /// appearing at [`galaxycraft::video::CAPTION_CHARS_PER_SEC`]
    /// (positions read on the Backwater recording at 655 s: x 80, line
    /// baselines 1120 / 1170 / 1220 of 1440).
    pub(super) fn draw_cinematic_caption(&mut self, fb: &mut Framebuffer, lines: &[&str]) {
        fb.clear(Rgb::new(0, 0, 0));
        let mut budget = (self.intro_started.elapsed().as_secs_f32()
            * galaxycraft::video::CAPTION_CHARS_PER_SEC) as usize;
        for (i, line) in lines.iter().enumerate() {
            if budget == 0 {
                break;
            }
            let shown: String = line.chars().take(budget).collect();
            budget = budget.saturating_sub(line.chars().count());
            self.glue_text_face(
                fb,
                CAPTION_X,
                CAPTION_FIRST_BASE + CAPTION_PITCH * i as f32,
                &shown,
                CAPTION_PX,
                CAPTION_GREEN,
                None,
                GlueFace::Hud,
            );
        }
    }

    /// Cinematic Subtitles (Sound tab, on by default): the retail `.srt`
    /// line of the playing movie, white with a shadow, centred low in the
    /// letterbox (Complete UI recording 341-372 s, the intro's lines under
    /// the picture).
    pub(super) fn draw_cinematic_subtitle(&mut self, fb: &mut Framebuffer, rel: &str) {
        if !self.game_options.cinematic_subtitles {
            return;
        }
        if self.intro_subs.as_ref().map(|(k, _)| k.as_str()) != Some(rel) {
            let subs = self
                .video_store
                .as_ref()
                .map(|store| {
                    let srt = Path::new(&rel.to_ascii_lowercase()).with_extension("srt");
                    store.root.join(srt)
                })
                .and_then(|p| std::fs::read_to_string(p).ok())
                .map(|t| galaxycraft::video::parse_srt(&t))
                .unwrap_or_default();
            self.intro_subs = Some((rel.to_string(), subs));
        }
        let Some(t) = self.intro_player.as_ref().map(|p| p.position()) else {
            return;
        };
        let Some(line) = self
            .intro_subs
            .as_ref()
            .and_then(|(_, s)| galaxycraft::video::subtitle_at(s, t))
            .map(|s| s.text.clone())
        else {
            return;
        };
        let line = if self.game_options.game_language_filter {
            mature_language_filter(&line)
        } else {
            line
        };
        let lines: Vec<&str> = line.lines().collect();
        let n = lines.len() as f32;
        for (i, l) in lines.iter().enumerate() {
            let w = self.glue_measure(l, SUBTITLE_PX, GlueFace::Hud);
            let base = SUBTITLE_LAST_BASE - (n - 1.0 - i as f32) * SUBTITLE_PITCH;
            for (d, colour) in [(2.0, Rgb::new(0, 0, 0)), (0.0, Rgb::new(255, 255, 255))] {
                self.glue_text_face(
                    fb,
                    1280.0 - w / 2.0 + d,
                    base + d,
                    l,
                    SUBTITLE_PX,
                    colour,
                    None,
                    GlueFace::Hud,
                );
            }
        }
    }
}
