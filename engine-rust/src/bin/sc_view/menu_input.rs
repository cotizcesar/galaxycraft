//! `impl App`: menu input (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    pub(super) fn open_menu(&mut self) {
        self.game_dialog = None;
        self.game_dialog_hover = None;
        self.menu = true;
        self.intro = false;
        self.briefing = false;
        self.paused = false;
        self.replay = None;
        self.game_result = None;
        self.next_map = None;
        self.next_map = None;
        self.selected.clear();
        self.status.clear();
        self.result_panel = None;
        self.result_fit = None;
        self.show_stats = false;
        self.show_leaderboard = false;
        self.start_menu_music();
    }

    /// Scrollbar of the mission list: arrows scroll one row, the track
    /// pages.  Returns true when the press was consumed.
    pub(super) fn mission_scrollbar(&mut self, input: &MenuInput) -> bool {
        let (cx, _cy, cw, ch) = self.content_box;
        let to_content = |(mx, my): (i32, i32)| {
            (
                (mx - cx) as f32 * 2560.0 / cw.max(1) as f32,
                my as f32 * 1440.0 / ch.max(1) as f32,
            )
        };
        // Thumb drag in progress: follow the pointer while the button is
        // held (the grab offset keeps the thumb under the cursor).
        if let Some(grab) = self.mission_drag {
            if !input.mouse_down {
                self.mission_drag = None;
                self.mouse_was_down = false;
                return true;
            }
            if let Some(p) = input.mouse {
                let (_, fy) = to_content(p);
                self.mission_list
                    .set_thumb(galaxycraft::menu::mission_thumb_fraction(fy - grab));
            }
            return true;
        }
        let pressed = input.mouse_down && !self.mouse_was_down;
        let Some((mx, my)) = input.mouse.filter(|_| pressed) else {
            return false;
        };
        let (fx, fy) = to_content((mx, my));
        let Some(hit) = galaxycraft::menu::mission_scrollbar_hit(fx, fy, self.mission_list.thumb())
        else {
            return false;
        };
        self.mouse_was_down = true;
        let page = galaxycraft::campaign_progress::MissionList::ROWS as i32;
        self.mission_list.scroll(match hit {
            galaxycraft::menu::ScrollHit::Up => -1,
            galaxycraft::menu::ScrollHit::Down => 1,
            galaxycraft::menu::ScrollHit::PageUp => -page,
            galaxycraft::menu::ScrollHit::PageDown => page,
            galaxycraft::menu::ScrollHit::Thumb => {
                let top = galaxycraft::menu::mission_thumb_top(self.mission_list.thumb());
                self.mission_drag = Some(fy - top);
                0
            }
        });
        self.play("ui_click");
        true
    }

    /// Pointer state of the `glupokcancel` prompt: Some(true) over Ok,
    /// Some(false) over Cancel, and whether this tick is a press.
    pub(super) fn campaign_prompt_pointer(&mut self, input: &MenuInput) -> (Option<bool>, bool) {
        let pressed = input.mouse_down && !self.mouse_was_down;
        if !input.mouse_down {
            self.mouse_was_down = false;
        }
        let Some((mx, my)) = input.mouse else {
            return (None, false);
        };
        let (cx, cy, cw, ch) = self.content_box;
        let (scale, ox) = galaxycraft::menu::campaign_scale(cw, ch);
        let inside = |r: (f32, f32, f32, f32)| {
            let (x, y, w, h) = galaxycraft::menu::campaign_rect(r, scale, ox, cx, cy);
            mx >= x && mx < x + w && my >= y && my < y + h
        };
        let hit = if inside(galaxycraft::menu::PROMPT_BUTTONS[0]) {
            Some(true)
        } else if inside(galaxycraft::menu::PROMPT_BUTTONS[1]) {
            Some(false)
        } else {
            None
        };
        if pressed {
            self.mouse_was_down = true;
        }
        (hit, pressed)
    }

    /// Input snapshot for one menu tick.  Live play fills it from the window;
    /// `--menu-script` fills it from a script so the real menu code can be
    /// walked headlessly (screens + sounds) without synthetic X11 events.
    pub(super) fn handle_menu(&mut self, window: &Window) {
        let mut input = MenuInput {
            keys: window.get_keys_pressed(KeyRepeat::No),
            mouse: if self.pad_control {
                self.pointer.or(Some(self.pad_cursor))
            } else {
                self.surface_mouse(window)
            },
            mouse_down: window.get_mouse_down(MouseButton::Left),
            wheel: window
                .get_scroll_wheel()
                .map(|(_, y)| -(y.signum() as i32))
                .unwrap_or(0),
            shift: window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift),
        };
        if self.pad_frame.active {
            input.mouse = Some(self.pad_cursor);
            input.mouse_down |= self.pad_frame.lmb;
            input
                .keys
                .extend(self.pad_frame.menu_pressed.iter().copied());
            input.wheel += self.pad_frame.wheel;
            input.shift |= self.pad_frame.shift;
        }
        if self.menu_screen == MenuScreen::Credits {
            // Held arrows drive the panel's key scrolling.
            self.credits_view.keys = (window.is_key_down(Key::Up), window.is_key_down(Key::Down));
        }
        self.handle_menu_input(&input);
    }

    pub(super) fn handle_menu_input(&mut self, input: &MenuInput) {
        // While a dialog's plates leave, the next screen is not shown yet.
        if matches!(self.glue_swish, Some(GlueSwish::Out { .. })) {
            return;
        }
        // Sub-screen navigation for the retail menu flow.  Each screen uses
        // Up/Down + Enter/Esc; the map-list Tab/R shortcuts stay on Main.
        let screen = self.menu_screen.clone();
        match screen {
            MenuScreen::Main => {
                // Mouse: hover highlights, left click activates.
                if let Some((mx, my)) = input.mouse {
                    // Use the shipped-hot-area resolver: the footer entries
                    // overlap the tall Single/Multiplayer boxes and must win
                    // the hit test (retail z-order).
                    let hit = galaxycraft::menu::main_entry_at(
                        self.ui_scale,
                        self.ui_ox,
                        self.ui_oy,
                        mx,
                        my,
                    );
                    if let Some(idx) = hit {
                        self.set_main_hover(Some(idx));
                        if idx != self.profile_sel {
                            self.profile_sel = idx;
                            self.play("ui_move");
                        }
                        if input.mouse_down && !self.mouse_was_down {
                            self.mouse_was_down = true;
                            self.activate_main_item(idx);
                            return;
                        }
                    } else {
                        self.set_main_hover(None);
                    }
                }
                if !input.mouse_down {
                    self.mouse_was_down = false;
                }
                for key in input.keys.iter().copied() {
                    match key {
                        Key::Up => {
                            self.profile_sel = (self.profile_sel + 7) % 8;
                            self.set_main_hover(Some(self.profile_sel));
                            self.play("ui_move");
                        }
                        Key::Down => {
                            self.profile_sel = (self.profile_sel + 1) % 8;
                            self.set_main_hover(Some(self.profile_sel));
                            self.play("ui_move");
                        }
                        Key::Enter | Key::Space => {
                            let item = self.profile_sel;
                            self.activate_main_item(item);
                        }
                        Key::I => {
                            self.begin_intro(
                                galaxycraft::video::intro_sequence(self.intro_expansion, true)
                                    .into_iter()
                                    .map(str::to_string)
                                    .collect(),
                                None,
                            );
                            return;
                        }
                        Key::X => {
                            self.intro_expansion = !self.intro_expansion;
                            if let Some(store) = &self.video_store
                                && let Some((info, poster)) =
                                    store.intro_poster(self.intro_expansion)
                            {
                                self.intro_info = Some(info);
                                self.intro_poster = Some(poster);
                            }
                            self.play("ui_click");
                        }
                        Key::R => {
                            self.menu_screen = MenuScreen::Registry;
                            self.profile_sel = self.active_index().unwrap_or(0);
                            self.play("ui_click");
                        }
                        Key::Escape => self.quit = true,
                        _ => {}
                    }
                }
            }
            MenuScreen::Registry => {
                if self.registry_notice.is_some() {
                    // The notice is modal: only its Ok answers.
                    self.handle_registry_notice(input);
                    return;
                }
                if self.registry_entry.is_some() {
                    // The New ID popup is modal.
                    self.handle_registry_new_id(input);
                    return;
                }
                // Mouse: the shipped `glulogin` controls, the same rects
                // `draw_registry` draws (`menu::LOGIN_DIALOG`).
                if let Some((mx, my)) = input.mouse {
                    use galaxycraft::menu::GlueListHit;
                    let first = registry_top(self.profile_sel);
                    let rows = self
                        .profile_store
                        .profiles
                        .len()
                        .saturating_sub(first)
                        .min(REGISTRY_ROWS);
                    let hit = galaxycraft::menu::LOGIN_DIALOG.hit(
                        (mx - self.ui_ox) as f32 / self.ui_scale,
                        (my - self.ui_oy) as f32 / self.ui_scale,
                        rows,
                    );
                    if input.mouse_down && !self.mouse_was_down {
                        self.mouse_was_down = true;
                        if hit == Some(GlueListHit::Ok) {
                            self.registry_continue();
                        } else if hit == Some(GlueListHit::Cancel) {
                            self.registry_after_gametype = false;
                            self.menu_screen = if std::mem::take(&mut self.mp.registry_next) {
                                MenuScreen::Connection
                            } else {
                                MenuScreen::Main
                            };
                            self.profile_sel = 0;
                            self.play("ui_click");
                        } else if hit == Some(GlueListHit::NewId) {
                            self.registry_new_id();
                        } else if hit == Some(GlueListHit::Delete) {
                            self.registry_delete();
                        } else if let Some(GlueListHit::Row(row)) = hit {
                            self.profile_sel = first + row;
                            self.play("ui_move");
                        }
                        return;
                    }
                }
                if !input.mouse_down {
                    self.mouse_was_down = false;
                }
                for key in input.keys.iter().copied() {
                    match key {
                        Key::Up => {
                            self.profile_sel = self.profile_sel.saturating_sub(1);
                            self.play("ui_move");
                        }
                        Key::Down => {
                            self.profile_sel = (self.profile_sel + 1)
                                .min(self.profile_store.profiles.len().saturating_sub(1));
                            self.play("ui_move");
                        }
                        Key::N => self.registry_new_id(),
                        Key::D => self.registry_delete(),
                        Key::Enter | Key::Space => {
                            self.registry_continue();
                        }
                        Key::Escape => {
                            self.registry_after_gametype = false;
                            if std::mem::take(&mut self.mp.registry_next) {
                                self.menu_screen = MenuScreen::Connection;
                            } else if let Some(t) = galaxycraft::menu::back_destination(&screen) {
                                self.menu_screen = t;
                            }
                            self.profile_sel = 0;
                            self.play("ui_click");
                        }
                        _ => {}
                    }
                }
            }
            MenuScreen::GameType => {
                // The retail popup is a real dialog: the pointer highlights
                // the wordmark buttons and `Cancel`, and a left click chooses
                // the entry.  The menu behind stays idle (no hover clip).
                self.main_cursor = None;
                let hit = input.mouse.and_then(|(x, y)| {
                    galaxycraft::menu::gametype_entry_at(
                        self.ui_scale,
                        self.ui_ox,
                        self.ui_oy,
                        x,
                        y,
                    )
                });
                self.gametype_hover = hit.filter(|idx| *idx < 2);
                if let Some(idx) = hit
                    && input.mouse_down
                    && !self.mouse_was_down
                {
                    self.mouse_was_down = true;
                    match idx {
                        0 | 1 => self.finish_gametype(idx == 1),
                        _ => {
                            self.menu_screen = MenuScreen::Main;
                            self.gametype_hover = None;
                            self.gametype_origin = galaxycraft::menu::GameTypeOrigin::SinglePlayer;
                            self.play("ui_click");
                        }
                    }
                    return;
                }
                if !input.mouse_down {
                    self.mouse_was_down = false;
                }
                for key in input.keys.iter().copied() {
                    match key {
                        Key::Up | Key::Down => {
                            self.episode_sel = 1 - self.episode_sel.min(1);
                            self.gametype_hover = None;
                            self.play("ui_move");
                        }
                        Key::Enter | Key::Space => {
                            self.finish_gametype(self.episode_sel.min(1) == 1);
                        }
                        Key::L => {
                            self.menu_screen = MenuScreen::Lan;
                            self.lan.map_index = self.map_index;
                            self.lan_scan();
                            self.play("ui_click");
                        }
                        Key::Escape => {
                            if let Some(t) = galaxycraft::menu::back_destination(&screen) {
                                self.menu_screen = t;
                            }
                            self.gametype_hover = None;
                            self.gametype_origin = galaxycraft::menu::GameTypeOrigin::SinglePlayer;
                            self.play("ui_click");
                        }
                        _ => {}
                    }
                }
            }
            MenuScreen::Episode => {
                // `glupokcancel` prompt over a locked campaign: Ok (O,
                // Enter) plays it anyway, Cancel (C, Esc) closes it.
                if let Some(campaign) = self.campaign_prompt {
                    let (hit, clicked) = self.campaign_prompt_pointer(input);
                    let mut choice = if clicked { hit } else { None };
                    for key in input.keys.iter().copied() {
                        match key {
                            Key::O | Key::Enter => choice = Some(true),
                            Key::C | Key::Escape => choice = Some(false),
                            _ => {}
                        }
                    }
                    if let Some(ok) = choice {
                        self.campaign_prompt = None;
                        self.play("ui_click");
                        if ok {
                            return self.enter_campaign(campaign);
                        }
                    }
                    return;
                }
                // Mouse: the three episode slots and the four shipped command
                // controls (`glucmpgn` ids 8/7/6/5/10/11/9).
                if let Some((mx, my)) = input.mouse {
                    let (cx, cy, cw, ch) = self.content_box;
                    let (scale, ox) = galaxycraft::menu::campaign_scale(cw, ch);
                    let expansion = self.episode_sel >= 3;
                    let slots: &[galaxycraft::menu::CampaignSlot] = if expansion {
                        &galaxycraft::menu::CAMPAIGN_X_SLOTS
                    } else {
                        &galaxycraft::menu::CAMPAIGN_SLOTS
                    };
                    let inside = |r: (f32, f32, f32, f32)| {
                        let (x, y, w, h) = galaxycraft::menu::campaign_rect(r, scale, ox, cx, cy);
                        mx >= x && mx < x + w && my >= y && my < y + h
                    };
                    let slot = slots.iter().position(|s| inside(s.button));
                    if slot.is_some() && slot != self.episode_hover {
                        self.play("ui_move");
                    }
                    self.episode_hover = slot;
                    if input.mouse_down && !self.mouse_was_down {
                        self.mouse_was_down = true;
                        if let Some(idx) = slot {
                            self.play("ui_click");
                            let campaign =
                                galaxycraft::campaign_progress::Campaign::of_slot(expansion, idx);
                            return self.select_campaign(campaign);
                        }
                        let which = galaxycraft::menu::CAMPAIGN_BUTTONS
                            .iter()
                            .position(|b| inside(b.rect));
                        if let Some(which) = which {
                            self.play("ui_click");
                            self.episode_command(which);
                        }
                        return;
                    }
                }
                if !input.mouse_down {
                    self.mouse_was_down = false;
                }
                let expansion = self.episode_sel >= 3;
                for key in input.keys.iter().copied() {
                    use galaxycraft::campaign_progress::Campaign as C;
                    // Slot hotkeys come from the shipped strings
                    // (`t\x04T\x01erran`, `z..`, `p..`).
                    let race = match (key, expansion) {
                        (Key::T, false) => Some(C::Terran),
                        (Key::Z, false) => Some(C::Zerg),
                        (Key::P, false) => Some(C::Protoss),
                        (Key::T, true) => Some(C::ExpTerran),
                        (Key::Z, true) => Some(C::ExpZerg),
                        (Key::P, true) => Some(C::ExpProtoss),
                        _ => None,
                    };
                    if let Some(c) = race {
                        self.play("ui_click");
                        return self.select_campaign(c);
                    }
                    match key {
                        Key::Up => {
                            self.episode_sel = (self.episode_sel + 5) % 6;
                            self.play("ui_move");
                        }
                        Key::Down => {
                            self.episode_sel = (self.episode_sel + 1) % 6;
                            self.play("ui_move");
                        }
                        Key::Enter | Key::Space => {
                            self.play("ui_click");
                            if let Some(c) = galaxycraft::campaign_progress::Campaign::from_episode(
                                (self.episode_sel + 1) as u8,
                            ) {
                                return self.select_campaign(c);
                            }
                        }
                        // `l\x04L\x01oad Saved`, `rLoad \x04R\x01eplay`,
                        // `uPlay C\x04u\x01stom`, `c\x04C\x01ancel`.
                        Key::L => {
                            self.play("ui_click");
                            return self.episode_command(0);
                        }
                        Key::U => {
                            self.play("ui_click");
                            return self.episode_command(1);
                        }
                        Key::R => {
                            self.play("ui_click");
                            return self.episode_command(2);
                        }
                        Key::C | Key::Escape => {
                            self.play("ui_click");
                            return self.episode_command(3);
                        }
                        _ => {}
                    }
                }
            }
            MenuScreen::Mission { episode } => {
                if self.mission_scrollbar(input) {
                    return;
                }
                if input.wheel != 0 {
                    self.mission_list.scroll(input.wheel);
                }
                let (hit, clicked) = self.menu_pointer(input);
                if clicked {
                    match hit {
                        Some(MenuHit::Ok) => return self.launch_mission(episode),
                        Some(MenuHit::Cancel) => {
                            self.menu_screen = MenuScreen::Episode;
                            self.play("ui_click");
                        }
                        Some(MenuHit::Row(i)) if self.mission_list.click_row(i) => {
                            self.play("ui_click");
                        }
                        _ => {}
                    }
                }
                for key in input.keys.iter().copied() {
                    match key {
                        Key::Up => {
                            self.mission_list.step(-1);
                            self.play("ui_move");
                        }
                        Key::Down => {
                            self.mission_list.step(1);
                            self.play("ui_move");
                        }
                        // `o\x04O\x01k` / `c\x04C\x01ancel` (GLUHIST-2/3).
                        Key::Enter | Key::Space | Key::O => return self.launch_mission(episode),
                        Key::Escape | Key::C => {
                            self.menu_screen = MenuScreen::Episode;
                            self.play("ui_click");
                        }
                        _ => {}
                    }
                }
            }
            ref s @ (MenuScreen::Connection
            | MenuScreen::Games
            | MenuScreen::MpCreate
            | MenuScreen::MpLobby) => self.handle_mp_input(input, s),
            MenuScreen::Lan => {
                let (hit, clicked) = self.menu_pointer(input);
                if clicked && let Some(MenuHit::Row(i)) = hit {
                    let entry = i.checked_sub(2);
                    if let Some(e) = entry.filter(|e| *e < self.lan.entries.len()) {
                        if self.lan.sel == e {
                            self.lan_join();
                        } else {
                            self.lan.sel = e;
                            self.play("ui_click");
                        }
                    }
                }
                for key in input.keys.iter().copied() {
                    match key {
                        Key::S => {
                            self.lan_scan();
                            self.play("ui_move");
                        }
                        Key::H => {
                            self.lan_host();
                            self.play("ui_click");
                        }
                        Key::Up => {
                            self.lan.sel = self.lan.sel.saturating_sub(1);
                            self.play("ui_move");
                        }
                        Key::Down => {
                            self.lan.sel =
                                (self.lan.sel + 1).min(self.lan.entries.len().saturating_sub(1));
                            self.play("ui_move");
                        }
                        Key::Enter | Key::Space => {
                            if self.lan.listener.is_none() {
                                self.lan_join();
                            } else {
                                self.lan_start_hosted();
                            }
                        }
                        Key::Escape => {
                            self.lan.listener = None;
                            self.lan.queue = None;
                            self.lan.announce_sock = None;
                            if let Some(t) = galaxycraft::menu::back_destination(&screen) {
                                self.menu_screen = t;
                            }
                            self.play("ui_click");
                        }
                        _ => {}
                    }
                }
            }
            MenuScreen::SavedGames | MenuScreen::Replays => {
                let replays = matches!(&self.menu_screen, MenuScreen::Replays);
                let list = self.save_list(replays);
                let (hit, clicked) = self.menu_pointer(input);
                if clicked {
                    match hit {
                        Some(MenuHit::Ok) => {
                            if let Some(path) = list.get(self.save_sel).cloned() {
                                self.load_save(&path, replays);
                            }
                        }
                        Some(MenuHit::Cancel) => {
                            self.menu_screen = MenuScreen::Episode;
                            self.save_sel = 0;
                            self.play("ui_click");
                        }
                        Some(MenuHit::Delete) => {
                            if let Some(path) = list.get(self.save_sel) {
                                let _ = std::fs::remove_file(path);
                                self.save_sel = self.save_sel.saturating_sub(1);
                                self.play("ui_click");
                            }
                        }
                        Some(MenuHit::Row(i)) => {
                            // Load Replay lists its folders first; they hold
                            // nothing this client can open.
                            let file = if replays {
                                i.checked_sub(REPLAY_FOLDERS.len())
                            } else {
                                Some(i)
                            };
                            if let Some(f) = file.filter(|&f| f != self.save_sel && f < list.len())
                            {
                                self.save_sel = f;
                                self.play("ui_click");
                            }
                        }
                        _ => {}
                    }
                }
                for key in input.keys.iter().copied() {
                    match key {
                        Key::Up => {
                            self.save_sel = self.save_sel.saturating_sub(1);
                            self.play("ui_move");
                        }
                        Key::Down => {
                            self.save_sel = self
                                .save_sel
                                .saturating_add(1)
                                .min(list.len().saturating_sub(1));
                            self.play("ui_move");
                        }
                        Key::Enter | Key::Space => {
                            if let Some(path) = list.get(self.save_sel).cloned() {
                                self.load_save(&path, replays);
                            }
                        }
                        Key::Delete | Key::Backspace => {
                            if let Some(path) = list.get(self.save_sel) {
                                let _ = std::fs::remove_file(path);
                                self.save_sel = self.save_sel.saturating_sub(1);
                                self.play("ui_click");
                            }
                        }
                        Key::Escape => {
                            if let Some(t) = galaxycraft::menu::back_destination(&screen) {
                                self.menu_screen = t;
                            }
                            self.save_sel = 0;
                            self.play("ui_click");
                        }
                        _ => {}
                    }
                }
            }
            MenuScreen::Lobby { .. } => {
                if input.wheel != 0 {
                    let rows = self.classic_rows().len();
                    let max = rows.saturating_sub(6) as i32;
                    self.lobby_top = (self.lobby_top as i32 - input.wheel).clamp(0, max) as usize;
                }
                let (hit, clicked) = self.menu_pointer(input);
                if clicked {
                    match hit {
                        Some(MenuHit::Row(i)) => {
                            let row = self.classic_rows().get(self.lobby_top + i).cloned();
                            match row {
                                Some(ClassicRow::Up) => {
                                    self.lobby_dir.pop();
                                    self.lobby_top = 0;
                                    self.play("ui_click");
                                }
                                Some(ClassicRow::Folder(f)) => {
                                    self.lobby_dir.push(f);
                                    self.lobby_top = 0;
                                    self.play("ui_click");
                                }
                                Some(ClassicRow::Map(index)) => {
                                    self.map_index = index;
                                    self.play("ui_click");
                                }
                                None => {}
                            }
                        }
                        Some(MenuHit::Ok) => {
                            self.play("ui_click");
                            self.load_map(self.map_index);
                        }
                        Some(MenuHit::Cancel) => {
                            self.menu_screen = MenuScreen::Main;
                            self.profile_sel = 0;
                            self.play("ui_click");
                        }
                        Some(MenuHit::Race(slot)) => {
                            self.lobby_race_open =
                                (self.lobby_race_open != Some(slot)).then_some(slot);
                            self.play("ui_click");
                        }
                        Some(MenuHit::RacePick(slot, k)) => {
                            let race = [
                                LobbyRace::Zerg,
                                LobbyRace::Terran,
                                LobbyRace::Protoss,
                                LobbyRace::Random,
                            ][k.min(3)];
                            if slot < self.lobby_races.len() {
                                self.lobby_races[slot] = race;
                            }
                            self.lobby_race_open = None;
                            self.play("ui_click");
                        }
                        _ => {}
                    }
                }
                for key in input.keys.iter().copied() {
                    match key {
                        Key::Enter | Key::Space => {
                            self.play("ui_click");
                            self.load_map(self.map_index);
                        }
                        Key::Up => {
                            self.map_index = self.map_index.saturating_sub(1);
                            self.play("ui_move");
                        }
                        Key::Down => {
                            self.map_index =
                                (self.map_index + 1).min(self.maps.len().saturating_sub(1));
                            self.play("ui_move");
                        }
                        Key::R => {
                            for r in self.lobby_races.iter_mut().take(4) {
                                *r = match *r {
                                    LobbyRace::Random => LobbyRace::Terran,
                                    LobbyRace::Terran => LobbyRace::Zerg,
                                    LobbyRace::Zerg => LobbyRace::Protoss,
                                    LobbyRace::Protoss => LobbyRace::Computer,
                                    LobbyRace::Computer => LobbyRace::Random,
                                };
                            }
                            self.play("ui_click");
                        }
                        Key::Escape => {
                            if let Some(t) = galaxycraft::menu::back_destination(&screen) {
                                self.menu_screen = t;
                            }
                            self.profile_sel = 0;
                            self.play("ui_click");
                        }
                        _ => {}
                    }
                }
            }
            MenuScreen::Options { tab } => {
                if self.options_reset {
                    // The confirmation is modal (screenshot 15:46:59).
                    self.handle_options_reset(input, tab);
                    return;
                }
                if self.changing_language {
                    // The language dialog has one action: Exit (reference 20).
                    if let Some((mx, my)) = input.mouse
                        && input.mouse_down
                        && !self.mouse_was_down
                    {
                        let (cx, _cy, cw, ch) = self.content_box;
                        let inside = |x0: f32, y0: f32, x1: f32, y1: f32| {
                            mx >= cx + (cw as f32 * x0 / 2560.0) as i32
                                && mx < cx + (cw as f32 * x1 / 2560.0) as i32
                                && my >= (ch as f32 * y0 / 1440.0) as i32
                                && my < (ch as f32 * y1 / 1440.0) as i32
                        };
                        if inside(1391.0, 813.0, 1751.0, 879.0) {
                            self.quit = true;
                        }
                    }
                    if input.keys.contains(&Key::Escape) {
                        self.changing_language = false;
                        self.play("ui_click");
                    }
                    self.mouse_was_down = input.mouse_down;
                    return;
                }
                let (hit, clicked) = self.menu_pointer(input);
                if clicked {
                    if !matches!(hit, Some(MenuHit::OptInput)) {
                        self.commit_apm_input();
                    }
                    // Any press outside the open rows closes the list; the
                    // two branches below re-open or choose as appropriate.
                    let was_open = self.options_drop;
                    self.options_drop = None;
                    match hit {
                        Some(MenuHit::OptInput) => {
                            self.apm_input =
                                Some(format!("{:.0}", self.game_options.apm_threshold));
                            self.play("ui_click");
                        }
                        Some(MenuHit::OptDrop(kind)) => {
                            self.options_drop = (was_open != Some(kind)).then_some(kind);
                            self.play("ui_click");
                        }
                        Some(MenuHit::OptDropItem(i)) => {
                            let kind = was_open;
                            match kind {
                                Some(OptionsDrop::DisplayMode) => {
                                    self.display_mode = i.min(DISPLAY_MODES.len() - 1) as u8;
                                    self.window_fullscreen = Some(self.display_mode != 0);
                                }
                                Some(OptionsDrop::Language) => {
                                    let picked = i.min(LANGUAGES.len() - 1);
                                    if picked != self.language {
                                        self.language = picked;
                                        // Retail asks for a restart; persist
                                        // the choice for the next launch.
                                        let code = LANGUAGE_CODES[picked];
                                        let path = saved_locale_path();
                                        if let Some(parent) = path.parent()
                                            && std::fs::create_dir_all(parent).is_ok()
                                        {
                                            let _ = std::fs::write(&path, code);
                                        }
                                        self.changing_language = true;
                                    }
                                }
                                None => {}
                            }
                            self.play("ui_click");
                        }
                        Some(MenuHit::Tab(i)) => {
                            // A tab switch stays on the same screen: the cue
                            // is the press sound, not the screen-in swish.
                            let all = OptionsTab::all();
                            self.menu_screen = MenuScreen::Options {
                                tab: all[i.min(all.len() - 1)],
                            };
                            self.play("ui_click");
                        }
                        Some(MenuHit::OptSlider(i)) => {
                            if let Some((mx, _)) = input.mouse {
                                let (cx, _cy, cw, _ch) = self.content_box;
                                let x = (mx - cx) as f32 / (cw as f32 / 2560.0);
                                let (x0, x1) = match (tab, i) {
                                    (OptionsTab::Speed, 0..=2) => (900.0, 1416.0),
                                    (OptionsTab::Speed, _) => (1456.0, 1972.0),
                                    (OptionsTab::Sound, _) => (900.0, 1416.0),
                                    (OptionsTab::Video, _) => (1456.0, 1972.0),
                                    _ => (900.0, 1416.0),
                                };
                                // The SD graphics filter rail has four stops.
                                let stops = if (tab, i) == (OptionsTab::Video, 1) {
                                    4
                                } else {
                                    7
                                };
                                let step = galaxycraft::menu::slider_step_n(x, x0, x1, stops);
                                let f = step as f32 / (stops - 1) as f32;
                                match (tab, i) {
                                    (OptionsTab::Speed, 0) => {
                                        self.game_options.game_speed = 0.5 + 0.5 * f;
                                    }
                                    (OptionsTab::Speed, 1) => self.game_options.key_scroll = f,
                                    (OptionsTab::Speed, 2) => self.game_options.mouse_scroll = f,
                                    (OptionsTab::Speed, _) => {
                                        self.game_options.mouse_sensitivity = f;
                                    }
                                    (OptionsTab::Sound, 0) => self.game_options.music_volume = f,
                                    (OptionsTab::Sound, _) => self.game_options.sfx_volume = f,
                                    (OptionsTab::Video, 0) => self.game_options.video_fps_limit = f,
                                    (OptionsTab::Video, _) => self.game_options.video_sd_filter = f,
                                    _ => {}
                                }
                                self.play("ui_click");
                            }
                        }
                        Some(MenuHit::OptCheck(i)) => {
                            let o = &mut self.game_options;
                            match (tab, i) {
                                (OptionsTab::Speed, 0) => {
                                    o.use_mouse_sensitivity = !o.use_mouse_sensitivity;
                                }
                                (OptionsTab::Speed, 1) => o.mouse_scaling = !o.mouse_scaling,
                                (OptionsTab::Speed, 2) => o.hardware_cursor = !o.hardware_cursor,
                                (OptionsTab::Speed, _) => {}
                                (OptionsTab::Sound, 0) => o.unit_speech = !o.unit_speech,
                                (OptionsTab::Sound, 1) => o.unit_acks = !o.unit_acks,
                                (OptionsTab::Sound, 2) => o.sound_background = !o.sound_background,
                                (OptionsTab::Sound, 3) => o.building_sounds = !o.building_sounds,
                                (OptionsTab::Sound, 4) => o.subtitles = !o.subtitles,
                                (OptionsTab::Sound, 5) => {
                                    o.cinematic_subtitles = !o.cinematic_subtitles;
                                }
                                (OptionsTab::Sound, 6) => {
                                    o.original_voice_overs = !o.original_voice_overs;
                                }
                                (OptionsTab::Sound, 7) => {
                                    o.original_1998_campaign = !o.original_1998_campaign;
                                }
                                (OptionsTab::Sound, 8) => o.music_enabled = !o.music_enabled,
                                (OptionsTab::Sound, _) => o.sfx_enabled = !o.sfx_enabled,
                                (OptionsTab::Video, 0) => o.video_fps_cap = !o.video_fps_cap,
                                (OptionsTab::Video, 1) => o.video_vsync = !o.video_vsync,
                                (OptionsTab::Video, 2) => o.video_hd = !o.video_hd,
                                (OptionsTab::Video, 3) => {
                                    o.video_environment = !o.video_environment
                                }
                                (OptionsTab::Video, 4) => o.video_lighting = !o.video_lighting,
                                (OptionsTab::Video, 5) => {
                                    o.video_smooth_turning = !o.video_smooth_turning;
                                }
                                (OptionsTab::Video, 6) => {
                                    o.video_shadow_stacking = !o.video_shadow_stacking;
                                }
                                (OptionsTab::Video, _) => o.video_pillarbox = !o.video_pillarbox,
                                (OptionsTab::Game, 0) => o.game_timer = !o.game_timer,
                                (OptionsTab::Game, 1) => {
                                    o.game_color_cycling = !o.game_color_cycling
                                }
                                (OptionsTab::Game, 2) => {
                                    o.game_language_filter = !o.game_language_filter;
                                }
                                (OptionsTab::Game, 3) => o.game_bonus_skins = !o.game_bonus_skins,
                                (OptionsTab::Game, 4) => {
                                    o.game_friend_notifications = !o.game_friend_notifications;
                                }
                                (OptionsTab::Game, 5) => o.portraits = 0,
                                (OptionsTab::Game, 6) => o.portraits = 1,
                                (OptionsTab::Game, 7) => o.portraits = 2,
                                (OptionsTab::Game, 8) => o.minimap_position = 0,
                                (OptionsTab::Game, 9) => o.minimap_position = 1,
                                (OptionsTab::Game, 10) => o.apm_display = !o.apm_display,
                                (OptionsTab::Game, 11) => o.apm_alert = !o.apm_alert,
                                (OptionsTab::Game, 12) => o.apm_color_text = !o.apm_color_text,
                                (OptionsTab::Game, _) => o.apm_play_sound = !o.apm_play_sound,
                                (OptionsTab::Network, i) if self.ingame_options => {
                                    o.network_latency = i.min(2) as u8;
                                }
                                (OptionsTab::Network, _) => {
                                    o.network_prefer_6112 = !o.network_prefer_6112;
                                }
                            }
                            self.play("ui_click");
                        }
                        Some(MenuHit::Restore) => {
                            // Asks first, then resets only this tab.
                            self.options_reset = true;
                            self.play("ui_click");
                        }
                        Some(MenuHit::Ok) => {
                            self.menu_screen = MenuScreen::Main;
                            self.profile_sel = 2;
                            self.play("ui_click");
                        }
                        _ => {}
                    }
                }
                for key in input.keys.iter().copied() {
                    if let Some(mut text) = self.apm_input.take() {
                        match key {
                            Key::Backspace => {
                                text.pop();
                                self.apm_input = Some(text);
                            }
                            Key::Enter | Key::Space => {
                                self.apm_input = Some(text);
                                self.commit_apm_input();
                            }
                            Key::Escape => {}
                            _ => {
                                if let Some(c) = chat_char(key)
                                    && c.is_ascii_digit()
                                    && text.len() < 3
                                {
                                    text.push(c);
                                }
                                self.apm_input = Some(text);
                            }
                        }
                        continue;
                    }
                    match key {
                        Key::Tab => {
                            let all = OptionsTab::all();
                            let i = all.iter().position(|t| *t == tab).unwrap_or(0);
                            self.menu_screen = MenuScreen::Options {
                                tab: all[(i + 1) % all.len()],
                            };
                            self.play("ui_click");
                        }
                        Key::Up => {
                            self.game_options.game_speed =
                                (self.game_options.game_speed + 0.1).min(3.0);
                            self.play("ui_move");
                        }
                        Key::Down => {
                            self.game_options.game_speed =
                                (self.game_options.game_speed - 0.1).max(0.2);
                            self.play("ui_move");
                        }
                        Key::F => {
                            self.game_options.fog = !self.game_options.fog;
                            self.play("ui_click");
                        }
                        Key::S => {
                            self.game_options.subtitles = !self.game_options.subtitles;
                            self.play("ui_click");
                        }
                        Key::Enter | Key::Space => {
                            self.menu_screen = MenuScreen::Main;
                            self.profile_sel = 2;
                            self.play("ui_click");
                        }
                        Key::Escape => {
                            if self.options_drop.take().is_some() {
                                self.play("ui_click");
                                continue;
                            }
                            if let Some(t) = galaxycraft::menu::back_destination(&screen) {
                                self.menu_screen = t;
                            }
                            self.profile_sel = 2;
                            self.play("ui_click");
                        }
                        _ => {}
                    }
                }
            }
            MenuScreen::Hotkeys { race } => {
                if self.hotkey_reset {
                    for key in input.keys.iter().copied() {
                        match key {
                            Key::Escape => {
                                self.hotkey_reset = false;
                                self.play("ui_click");
                            }
                            Key::Enter | Key::Space => {
                                self.restore_hotkey_defaults();
                                self.hotkey_reset = false;
                                self.play("ui_click");
                            }
                            _ => {}
                        }
                    }
                    if let Some((mx, my)) = input.mouse {
                        let (cx, cy, cw, ch) = self.content_box;
                        let hit = |x0: f32, y0: f32, x1: f32, y1: f32| {
                            mx >= cx + (cw as f32 * x0 / 2560.0) as i32
                                && mx < cx + (cw as f32 * x1 / 2560.0) as i32
                                && my >= cy + (ch as f32 * y0 / 1440.0) as i32
                                && my < cy + (ch as f32 * y1 / 1440.0) as i32
                        };
                        if input.mouse_down && !self.mouse_was_down {
                            self.mouse_was_down = true;
                            if hit(996.0, 813.0, 1356.0, 879.0) {
                                self.restore_hotkey_defaults();
                                self.hotkey_reset = false;
                                self.play("ui_click");
                            } else if hit(1387.0, 813.0, 1747.0, 879.0) {
                                self.hotkey_reset = false;
                                self.play("ui_click");
                            }
                        }
                    }
                    if !input.mouse_down {
                        self.mouse_was_down = false;
                    }
                    return;
                }
                for key in input.keys.iter().copied() {
                    if let Some(action) = self.hotkey_action
                        && let Some(c) = key_letter(key)
                    {
                        self.rebind_hotkey(action, c, c.to_ascii_lowercase() as u16);
                        self.play("ui_click");
                        continue;
                    }
                    match key {
                        Key::Tab => {
                            let all = HotkeyRace::all();
                            let i = all.iter().position(|r| *r == race).unwrap_or(0);
                            self.menu_screen = MenuScreen::Hotkeys {
                                race: all[(i + 1) % all.len()],
                            };
                            self.hotkey_sel = None;
                            self.hotkey_action = None;
                            self.hotkey_hover = None;
                            self.play("ui_click");
                        }
                        Key::Escape => {
                            if let Some(t) = galaxycraft::menu::back_destination(&screen) {
                                self.menu_screen = t;
                            }
                            self.profile_sel = 4;
                            self.play("ui_click");
                        }
                        Key::F12 => self.capture_request = true,
                        _ => {}
                    }
                }
                // Mouse: race tabs, unit cells and command cards.  The tab
                // hover edge is what capture 131026 shows (Protoss hovered
                // while Terran is still the active race).
                if let Some((mx, my)) = input.mouse {
                    let (cx, cy, cw, ch) = self.content_box;
                    // Bottom "Restore to Default" opens the confirmation dialog.
                    if input.mouse_down && !self.mouse_was_down {
                        let in_rect = |x0: f32, y0: f32, x1: f32, y1: f32| {
                            mx >= cx + (cw as f32 * x0 / 2560.0) as i32
                                && mx < cx + (cw as f32 * x1 / 2560.0) as i32
                                && my >= cy + (ch as f32 * y0 / 1440.0) as i32
                                && my < cy + (ch as f32 * y1 / 1440.0) as i32
                        };
                        if in_rect(354.0, 1302.0, 744.0, 1386.0) {
                            self.mouse_was_down = true;
                            self.hotkey_reset = true;
                            self.play("ui_click");
                            return;
                        }
                        if in_rect(1453.0, 1302.0, 1831.0, 1386.0) {
                            self.mouse_was_down = true;
                            if let Some(path) = hotkey_save_path()
                                && let Err(e) =
                                    hotkey_data::save_bindings(&path, &self.hotkey_bindings)
                            {
                                eprintln!("cannot save hotkeys {}: {e}", path.display());
                            }
                            self.hotkey_baseline = self.hotkey_bindings.clone();
                            self.menu_screen = MenuScreen::Main;
                            self.profile_sel = 4;
                            self.play("ui_click");
                            return;
                        }
                        if in_rect(1825.0, 1302.0, 2203.0, 1386.0) {
                            self.mouse_was_down = true;
                            self.hotkey_bindings = self.hotkey_baseline.clone();
                            self.hotkey_unbound.clear();
                            self.hotkey_binding_error = false;
                            self.menu_screen = MenuScreen::Main;
                            self.profile_sel = 4;
                            self.play("ui_click");
                            return;
                        }
                    }
                    let prev_tab_hover = self.hotkey_tab_hover;
                    let prev_unit_hover = self.hotkey_hover;
                    let mut tab_hover = None;
                    for (i, r) in HotkeyRace::all().iter().enumerate() {
                        let tx = cx + (cw as f32 * (0.316797 + 0.108984 * i as f32)) as i32;
                        let tw = (cw as f32 * 0.101953) as i32;
                        let ty = cy + (ch as f32 * 0.029167) as i32;
                        let th = (ch as f32 * 0.052083) as i32;
                        if mx >= tx && mx < tx + tw && my >= ty && my < ty + th {
                            tab_hover = Some(*r);
                            if input.mouse_down && !self.mouse_was_down {
                                self.mouse_was_down = true;
                                self.menu_screen = MenuScreen::Hotkeys { race: *r };
                                self.hotkey_sel = None;
                                self.hotkey_action = None;
                                self.hotkey_hover = None;
                                self.play("ui_click");
                            }
                        }
                    }
                    self.hotkey_tab_hover = tab_hover;
                    self.hotkey_hover = self
                        .hotkey_cells
                        .iter()
                        .find(|(_, r)| mx >= r.0 && mx < r.0 + r.2 && my >= r.1 && my < r.1 + r.3)
                        .map(|(id, _)| *id);
                    if self.hotkey_tab_hover != prev_tab_hover && self.hotkey_tab_hover.is_some() {
                        self.play("ui_move");
                    }
                    if self.hotkey_hover != prev_unit_hover && self.hotkey_hover.is_some() {
                        self.play("ui_move");
                    }
                    let action_hit = self
                        .hotkey_action_cells
                        .iter()
                        .find(|(_, r)| mx >= r.0 && mx < r.0 + r.2 && my >= r.1 && my < r.1 + r.3)
                        .map(|(id, _)| *id);
                    if input.mouse_down && !self.mouse_was_down {
                        self.mouse_was_down = true;
                        if let Some(id) = action_hit {
                            self.hotkey_action = Some(id);
                            self.play("ui_click");
                        }
                        if let Some(id) = self.hotkey_hover {
                            self.hotkey_sel = Some(id);
                            self.hotkey_action = None;
                            self.play("ui_click");
                        }
                    }
                }
                if !input.mouse_down {
                    self.mouse_was_down = false;
                }
            }
            MenuScreen::Credits => {
                // `creditspanel.js`: wheel ticks scroll the list (and pause
                // the auto scroll); keyCode 27 / 13 send `Exit`.
                self.credits_view.wheel += input.wheel;
                for key in input.keys.iter().copied() {
                    if matches!(key, Key::Enter | Key::Escape) {
                        self.menu_screen = MenuScreen::Main;
                        self.play("ui_click");
                    }
                }
            }
            MenuScreen::Collections => {
                if input.wheel != 0 {
                    self.collections_scroll =
                        (self.collections_scroll + input.wheel as f32 * 40.0).clamp(0.0, 480.0);
                }
                let (hit, clicked) = self.menu_pointer(input);
                if clicked {
                    match hit {
                        Some(MenuHit::Cell(i)) => {
                            self.collection_sel = i;
                            self.play("ui_click");
                        }
                        Some(MenuHit::Equip) => {
                            self.play("ui_click");
                            // Console skins (cells 0-2) and the unit &
                            // structure set (cell 3 base art, cell 4 Carbot
                            // from `anim/skins.json`) are applied here.
                            // Announcers remain preview-only until their
                            // voice sets are wired.
                            if let Some(skin) = collection_console_skin(self.collection_sel) {
                                self.game_options.game_console_skin = skin;
                                self.console = ConsoleArt::load(
                                    &self.assets_root,
                                    self.console_race,
                                    skin,
                                    self.game_options.game_unit_skin,
                                );
                                self.flush_profiles();
                            }
                            if let Some(id) = collection_announcer(self.collection_sel) {
                                self.game_options.game_announcer = id;
                                self.voices.set_announcer(id);
                                self.flush_profiles();
                            }
                            match self.collection_sel {
                                3 | 4 => {
                                    self.game_options.game_unit_skin =
                                        if self.collection_sel == 4 { 2 } else { 0 };
                                    self.console = ConsoleArt::load(
                                        &self.assets_root,
                                        self.console_race,
                                        self.game_options.game_console_skin,
                                        self.game_options.game_unit_skin,
                                    );
                                    self.flush_profiles();
                                }
                                _ => {}
                            }
                        }
                        Some(MenuHit::Ok) | Some(MenuHit::Skip) => {
                            self.menu_screen = MenuScreen::Main;
                            self.play("ui_click");
                        }
                        _ => {}
                    }
                }
                for key in input.keys.iter().copied() {
                    match key {
                        Key::Enter | Key::Space | Key::Escape => {
                            self.menu_screen = MenuScreen::Main;
                            self.play("ui_click");
                        }
                        _ => {}
                    }
                }
            }
            MenuScreen::Intro { .. } => {
                let (_hit, clicked) = self.menu_pointer(input);
                if clicked {
                    self.menu_screen = MenuScreen::Main;
                    self.play("ui_click");
                }
                for key in input.keys.iter().copied() {
                    match key {
                        Key::Enter | Key::Space | Key::Escape => {
                            self.menu_screen = MenuScreen::Main;
                            self.play("ui_click");
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Clickable rectangles of the eight main-menu entries, in window pixels.
    /// Used both for drawing and for hit-testing so they can never diverge.
    /// Control under a window point on the current sub-screen.  The rects
    /// come from the measured control tables in `menu.rs`; rows derive from
    /// the measured 57 px pitch.
    pub(super) fn menu_hit(&self, screen: &MenuScreen, x: i32, y: i32) -> Option<MenuHit> {
        use galaxycraft::menu as m;
        let (cx, _cy, cw, ch) = self.content_box;
        let kx = cw as f32 / 2560.0;
        let ky = ch as f32 / 1440.0;
        let inside = |r: m::HitRect| {
            x >= cx + (r.0 * kx) as i32
                && x < cx + ((r.0 + r.2) * kx) as i32
                && y >= (r.1 * ky) as i32
                && y < ((r.1 + r.3) * ky) as i32
        };
        let row_of = |r: m::HitRect, rows: usize| {
            let fy = y as f32 / ky;
            if fy < r.1 || fy >= r.1 + r.3 {
                return None;
            }
            let row = ((fy - r.1) / 57.0) as usize;
            (row < rows).then_some(row)
        };
        match screen {
            MenuScreen::Mission { .. } => {
                if inside(m::MISSION_BUTTONS[0]) {
                    Some(MenuHit::Ok)
                } else if inside(m::MISSION_BUTTONS[1]) {
                    Some(MenuHit::Cancel)
                } else {
                    row_of(m::MISSION_ROWS, 8).map(MenuHit::Row)
                }
            }
            MenuScreen::Replays => {
                if inside(m::CREATE_BUTTONS[0]) {
                    Some(MenuHit::Ok)
                } else if inside(m::CREATE_BUTTONS[1]) {
                    Some(MenuHit::Cancel)
                } else {
                    row_of(m::CREATE_ROWS, 6).map(MenuHit::Row)
                }
            }
            MenuScreen::Lobby { .. } => {
                if inside(m::CREATE_BUTTONS[0]) {
                    Some(MenuHit::Ok)
                } else if inside(m::CREATE_BUTTONS[1]) {
                    Some(MenuHit::Cancel)
                } else {
                    if let Some(slot) = self.lobby_race_open {
                        for k in 0..LobbyRace::all().len() {
                            if inside(m::lobby_race_item(slot, k)) {
                                return Some(MenuHit::RacePick(slot, k));
                            }
                        }
                    }
                    for i in 0..8 {
                        if inside(m::lobby_race_bay(i)) {
                            return Some(MenuHit::Race(i));
                        }
                    }
                    row_of(m::CREATE_ROWS, 6).map(MenuHit::Row)
                }
            }
            MenuScreen::SavedGames => {
                // The shipped `gluload` controls `draw_saved` draws.
                use galaxycraft::menu::GlueListHit;
                let rows = self.save_list(false).len().min(SAVED_ROWS);
                match galaxycraft::menu::LOAD_DIALOG.hit(
                    (x - self.ui_ox) as f32 / self.ui_scale,
                    (y - self.ui_oy) as f32 / self.ui_scale,
                    rows,
                )? {
                    GlueListHit::Ok => Some(MenuHit::Ok),
                    GlueListHit::Cancel => Some(MenuHit::Cancel),
                    GlueListHit::Delete => Some(MenuHit::Delete),
                    GlueListHit::Row(r) => Some(MenuHit::Row(r)),
                    GlueListHit::NewId => None,
                }
            }
            MenuScreen::Options { tab } => {
                // An open list takes the pointer first, like the CEF select.
                if let Some(kind) = self.options_drop {
                    let g = options_drop_geometry(kind);
                    for i in 0..g.count {
                        let row = (g.list.0, g.list.1 + g.pitch * i as f32, g.list.2, g.pitch);
                        if inside(row) {
                            return Some(MenuHit::OptDropItem(i));
                        }
                    }
                }
                if *tab == OptionsTab::Video
                    && inside(options_drop_geometry(OptionsDrop::DisplayMode).closed)
                {
                    return Some(MenuHit::OptDrop(OptionsDrop::DisplayMode));
                }
                if *tab == OptionsTab::Game
                    && inside(options_drop_geometry(OptionsDrop::Language).closed)
                {
                    return Some(MenuHit::OptDrop(OptionsDrop::Language));
                }
                for (i, r) in m::OPTIONS_TABS.iter().enumerate() {
                    if inside(*r) {
                        return Some(MenuHit::Tab(i));
                    }
                }
                let (sliders, checks): (&[m::HitRect], &[m::HitRect]) = match tab {
                    OptionsTab::Speed => (&m::OPTIONS_SLIDERS_SPEED, &m::OPTIONS_CHECKS_SPEED),
                    OptionsTab::Sound => (&m::OPTIONS_SLIDERS_SOUND, &m::OPTIONS_CHECKS_SOUND),
                    OptionsTab::Video => (&m::OPTIONS_SLIDERS_VIDEO, &m::OPTIONS_CHECKS_VIDEO),
                    OptionsTab::Game => (&[], &m::OPTIONS_CHECKS_GAME),
                    OptionsTab::Network if self.ingame_options => (&[], &m::OPTIONS_RADIOS_LATENCY),
                    OptionsTab::Network => (&[], &m::OPTIONS_CHECKS_NETWORK),
                };
                for (i, r) in sliders.iter().enumerate() {
                    if inside(*r) {
                        return Some(MenuHit::OptSlider(i));
                    }
                }
                for (i, r) in checks.iter().enumerate() {
                    if inside(*r) {
                        return Some(MenuHit::OptCheck(i));
                    }
                }
                // APM tracking: Color Text / Play Sound only answer while
                // the alert is on (they are greyed otherwise).
                if matches!(tab, OptionsTab::Game) {
                    if inside(m::OPTIONS_APM_INPUT) {
                        return Some(MenuHit::OptInput);
                    }
                    for (i, r) in m::OPTIONS_CHECKS_APM.iter().enumerate() {
                        if inside(*r) && (i < 2 || self.game_options.apm_alert) {
                            return Some(MenuHit::OptCheck(m::OPTIONS_CHECKS_GAME.len() + i));
                        }
                    }
                }
                if inside(m::OPTIONS_BUTTONS[0]) {
                    Some(MenuHit::Restore)
                } else if inside(m::OPTIONS_BUTTONS[1]) {
                    Some(MenuHit::Ok)
                } else {
                    None
                }
            }
            MenuScreen::Collections => {
                let scroll = self.collections_scroll;
                for (i, r) in m::COLLECTION_CELLS.iter().enumerate() {
                    if inside((r.0, r.1 - scroll, r.2, r.3)) {
                        return Some(MenuHit::Cell(i));
                    }
                }
                if inside(m::COLLECTION_BUTTONS[0]) {
                    Some(MenuHit::Ok)
                } else if inside(m::COLLECTION_BUTTONS[1]) {
                    Some(MenuHit::Equip)
                } else {
                    None
                }
            }
            MenuScreen::Lan => {
                let lines = 2 + if let Some(queue) = &self.lan.queue {
                    queue.capacity() as usize + 1
                } else if self.lan.entries.is_empty() {
                    1
                } else {
                    self.lan.entries.len()
                };
                let panel = m::lan_panel_rect(lines);
                for i in 0..lines {
                    if inside(m::lan_row_rect(panel, i)) {
                        return Some(MenuHit::Row(i));
                    }
                }
                None
            }
            // The credits page has no click handler: only Esc / Enter leave.
            MenuScreen::Intro { .. } => Some(MenuHit::Skip),
            _ => None,
        }
    }

    /// Pointer state for the current sub-screen: the control under the
    /// cursor (playing `ui_move` when it changes) and an edge-triggered left
    /// click.
    pub(super) fn menu_pointer(&mut self, input: &MenuInput) -> (Option<MenuHit>, bool) {
        let pointer = input.mouse;
        let screen = self.menu_screen.clone();
        let hit = pointer.and_then(|(x, y)| self.menu_hit(&screen, x, y));
        if hit != self.menu_hover {
            self.menu_hover = hit;
            if hit.is_some() {
                self.play("ui_move");
            }
        }
        let down = input.mouse_down;
        let clicked = down && !self.mouse_was_down;
        self.mouse_was_down = down;
        (hit, clicked)
    }

    /// Run the action of a main-menu entry (shared by Enter and mouse clicks).
    pub(super) fn activate_main_item(&mut self, idx: usize) {
        // The retail CEF menu has no dedicated transition cue: the press is
        // the only sound (GLUSND_MOUSE_DOWN).  Verified in the live capture.
        self.play("ui_click");
        // A sub-screen owns the pointer: the main menu must return to its
        // idle animation instead of keeping the button hover clip lit behind
        // the new panel (the retail client captures the mouse in the panel).
        self.main_cursor = None;
        match idx {
            0 => {
                self.gametype_origin = galaxycraft::menu::GameTypeOrigin::SinglePlayer;
                self.menu_screen = MenuScreen::GameType;
            }
            1 => {
                // Retail opens `Select Game Type` for Multiplayer too; the
                // lobby is only reached after choosing Original/Expansion.
                self.gametype_origin = galaxycraft::menu::GameTypeOrigin::Multiplayer;
                if self.campaign_len < self.maps.len() {
                    self.map_index = self.campaign_len;
                }
                self.menu_screen = MenuScreen::GameType;
            }
            2 => {
                self.menu_screen = MenuScreen::Options {
                    tab: OptionsTab::Speed,
                }
            }
            3 => self.menu_screen = MenuScreen::Collections,
            4 => {
                self.hotkey_baseline = self.hotkey_bindings.clone();
                self.hotkey_sel = None;
                self.hotkey_action = None;
                self.refresh_hotkey_errors();
                self.menu_screen = MenuScreen::Hotkeys {
                    race: HotkeyRace::Terran,
                };
            }
            5 => self.open_credits(),
            6 => {
                // Retail `View Intro` asks Original / Expansion with the same
                // `Select Game Type` dialog as Single Player, then replays
                // that title movie (the publisher logo is only part of the
                // first-launch sequence); `finish_gametype` starts it.
                self.gametype_origin = galaxycraft::menu::GameTypeOrigin::ViewIntro;
                self.menu_screen = MenuScreen::GameType;
            }
            _ => self.quit = true,
        }
    }

    /// Commit `Select Game Type` and continue on the flow that opened it.
    /// Single player goes to the episode list; multiplayer goes to the
    /// connection screen (our LAN lobby) instead of opening it directly from
    /// the main menu.
    /// Registry Ok: continue where the dialog was opened from (Select
    /// Connection -> Games, Select Game Type -> campaign, else Main).
    pub(super) fn registry_continue(&mut self) {
        self.capture_active_profile();
        if std::mem::take(&mut self.mp.registry_next) {
            self.mp_enter_games();
        } else if self.registry_after_gametype {
            self.registry_after_gametype = false;
            self.episode_sel = galaxycraft::menu::episode_start(false);
            self.menu_screen = MenuScreen::Episode;
        } else {
            self.menu_screen = MenuScreen::Main;
            self.profile_sel = 0;
        }
        self.play("ui_click");
    }

    pub(super) fn finish_gametype(&mut self, expansion: bool) {
        self.intro_expansion = expansion;
        self.episode_sel = galaxycraft::menu::episode_start(expansion);
        self.menu_screen = match galaxycraft::menu::gametype_followup(self.gametype_origin) {
            galaxycraft::menu::GameTypeFollowup::PlayIntro => {
                // The movie returns to the main menu; the popup is gone, so
                // the return is not a screen change that whooshes.  FACT
                // (2026-11-07 recording, 340.5 s Original / 382.5 s
                // Expansion): the Blizzard logo plays before either intro.
                self.swish_screen = Some(MenuScreen::Main);
                self.begin_intro(
                    galaxycraft::video::intro_sequence(expansion, true)
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    None,
                );
                MenuScreen::Main
            }
            galaxycraft::menu::GameTypeFollowup::SelectConnection => {
                self.mp.enter_connection();
                MenuScreen::Connection
            }
            galaxycraft::menu::GameTypeFollowup::Registry => {
                self.registry_after_gametype = true;
                self.profile_sel = self.active_index().unwrap_or(0);
                MenuScreen::Registry
            }
        };
        self.gametype_hover = None;
        self.gametype_origin = galaxycraft::menu::GameTypeOrigin::SinglePlayer;
        self.play("ui_click");
    }

    /// Retail Options panel (references 130933/130938/130942/130949/130954).
    /// Geometry is expressed in the 2560x1440 content pixels measured on the
    /// captures: panel 521..2038 x 181..1146, tab stack 521..847 with five
    /// 93 px cells from y=345, content columns at x=900 and x=1456 (516 px
    /// wide each), Restore/Ok buttons along y=1221..1286.
    /// Commit the APM threshold text field, keeping the 0..1000 range.
    pub(super) fn commit_apm_input(&mut self) {
        if let Some(text) = self.apm_input.take()
            && let Ok(value) = text.parse::<f32>()
        {
            self.game_options.apm_threshold = value.clamp(0.0, 1000.0);
        }
    }

    /// Input of the Options "Restore to Default" confirmation: Continue
    /// (or Enter) resets the open tab, Cancel, the close cross or Escape
    /// keep everything.
    pub(super) fn handle_options_reset(&mut self, input: &MenuInput, tab: OptionsTab) {
        let mut answer = None;
        for key in input.keys.iter().copied() {
            match key {
                Key::Escape => answer = Some(false),
                Key::Enter | Key::NumPadEnter | Key::Space => answer = Some(true),
                _ => {}
            }
        }
        if let Some((mx, my)) = input.mouse
            && input.mouse_down
            && !self.mouse_was_down
        {
            let (cx, cy, cw, ch) = self.content_box;
            let hit = |x0: f32, y0: f32, x1: f32, y1: f32| {
                mx >= cx + (cw as f32 * x0 / 2560.0) as i32
                    && mx < cx + (cw as f32 * x1 / 2560.0) as i32
                    && my >= cy + (ch as f32 * y0 / 1440.0) as i32
                    && my < cy + (ch as f32 * y1 / 1440.0) as i32
            };
            if hit(996.0, 813.0, 1356.0, 879.0) {
                answer = Some(true);
            } else if hit(1387.0, 813.0, 1747.0, 879.0) || hit(1815.0, 447.0, 1865.0, 497.0) {
                answer = Some(false);
            }
        }
        self.mouse_was_down = input.mouse_down;
        if let Some(reset) = answer {
            if reset {
                self.game_options.restore_tab(tab);
            }
            self.options_reset = false;
            self.play("ui_click");
        }
    }
}
