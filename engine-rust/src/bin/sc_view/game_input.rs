//! `impl App`: game input (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    /// UI scale and origin.
    ///
    /// Driven by the **height** so HUD art keeps its proportions from 16:9
    /// up to 4K and, at the same time, ultrawide screens expand the world
    /// view horizontally instead of getting letterboxed side bars.
    /// The 16:9 content box, centred. The retail client pillarboxes a 16:9
    /// render inside a wider desktop (verified on the 3440x1440 references:
    /// content occupies columns 440..2999, i.e. 2560x1440).
    /// Pointer in drawing-surface pixels (window pixels minus the
    /// letterbox offset).
    pub(super) fn surface_mouse(&self, window: &Window) -> Option<(i32, i32)> {
        window
            .get_unscaled_mouse_pos(MouseMode::Clamp)
            .map(|(x, y)| surface_point(self.present_offset, x as i32, y as i32))
    }

    /// Feed the raw surface pointer through the Speed tab's sensitivity and
    /// scaling.  With the default 0.5 sensitivity (factor 1) the pointer is
    /// the system position exactly; otherwise a virtual pointer follows the
    /// raw deltas so the drawn cursor and every click stay aligned.
    pub(super) fn update_pointer(&mut self, raw: Option<(i32, i32)>, surface: (i32, i32)) {
        let Some(raw) = raw else {
            self.pointer = None;
            self.last_raw_pointer = None;
            return;
        };
        // A physical mouse move takes the pointer back from the controller.
        // Control returns to the pad once the pad is neutral again.
        if let Some(last) = self.last_raw_pointer {
            let moved = (raw.0 - last.0).abs() + (raw.1 - last.1).abs();
            if moved > 2 && self.pad_control {
                self.pad_control = false;
                self.pad_suppressed = true;
            }
        }
        if self.pad_control {
            // Keep `last_raw_pointer` at the takeover position so even a slow
            // mouse movement accumulates and returns control to the mouse.
            return;
        }
        let factor = mouse_move_factor(&self.game_options, surface);
        if (factor - 1.0).abs() < 0.001 {
            self.pointer = Some(raw);
            self.virtual_pointer = Some((raw.0 as f32, raw.1 as f32));
            self.last_raw_pointer = Some(raw);
            return;
        }
        let current = self.virtual_pointer.unwrap_or((raw.0 as f32, raw.1 as f32));
        let last = self
            .last_raw_pointer
            .map(|p| (p.0 as f32, p.1 as f32))
            .unwrap_or((raw.0 as f32, raw.1 as f32));
        let next =
            advance_virtual_pointer(current, last, (raw.0 as f32, raw.1 as f32), factor, surface);
        self.virtual_pointer = Some(next);
        self.last_raw_pointer = Some(raw);
        self.pointer = Some((next.0.round() as i32, next.1.round() as i32));
    }

    /// Whether the operating-system pointer should be visible this frame.
    pub(super) fn system_cursor_visible(&self) -> bool {
        system_cursor_visible_for(self.pointer.is_some())
    }

    /// Screen pixels -> world-view pixels.  Must use the same divisor as
    /// `update_view`, otherwise clicks land off by (scale/3) at any
    /// resolution where the UI scale differs from 3.
    /// Surface point of the on-screen unit (own units first) whose name
    /// contains `name`, nearest the view centre.
    pub(super) fn screen_point_of_unit(&self, name: &str) -> Option<(i32, i32)> {
        let (vw, vh) = self.view;
        let centre = (vw / 2, vh / 2);
        self.world
            .units
            .iter()
            .filter(|u| u.alive && self.data.unit_name(u.type_id).contains(name))
            .map(|u| (u, self.screen_of(u)))
            .filter(|(_, (x, y))| *x >= 0 && *y >= 0 && *x < vw && *y < vh)
            .min_by_key(|(u, (x, y))| {
                (
                    u.owner as usize != self.local_player,
                    (x - centre.0).abs() + (y - centre.1).abs(),
                )
            })
            .map(|(_, (x, y))| self.logical_to_surface(x, y))
    }

    /// Surface point of a world position (game px).
    pub(super) fn screen_point_of_world(&self, wx: i32, wy: i32) -> (i32, i32) {
        self.logical_to_surface(wx - self.camera.0, wy - self.camera.1)
    }

    /// Inverse of [`Self::to_logical`] (pixel centre).
    pub(super) fn logical_to_surface(&self, lx: i32, ly: i32) -> (i32, i32) {
        let s = self.ui_scale.max(0.5);
        let sx_scale = if self.content_box.2 > 0 && self.view.0 > 0 {
            self.content_box.2 as f32 / self.view.0 as f32
        } else {
            s
        };
        (
            self.content_x0 + ((lx as f32 + 0.5) * sx_scale) as i32,
            ((ly as f32 + 0.5) * s) as i32,
        )
    }

    pub(super) fn to_logical(&self, sx: i32, sy: i32) -> (i32, i32) {
        let s = self.ui_scale.max(0.5);
        // Horizontally the view's `self.view.0` game px fill the content
        // width (`render::stretch_columns`), so the x scale is cw / view_w.
        let sx_scale = if self.content_box.2 > 0 && self.view.0 > 0 {
            self.content_box.2 as f32 / self.view.0 as f32
        } else {
            s
        };
        (
            ((sx - self.content_x0) as f32 / sx_scale) as i32,
            (sy as f32 / s) as i32,
        )
    }

    pub(super) fn center_on_base(&mut self) {
        let base = self
            .world
            .units
            .iter()
            .find(|u| {
                u.alive
                    && u.owner as usize == self.local_player
                    && matches!(
                        u.type_id,
                        uid::COMMAND_CENTER | uid::NEXUS | uid::HATCHERY | uid::LAIR | uid::HIVE
                    )
            })
            .or_else(|| {
                self.world
                    .units
                    .iter()
                    .find(|u| u.alive && u.owner as usize == self.local_player)
            });
        if let Some(u) = base {
            self.camera = (u.x / 256 - self.view.0 / 2, u.y / 256 - self.view.1 / 2);
            self.clamp_camera();
        }
    }

    pub(super) fn clamp_camera(&mut self) {
        let max_x = (self.scenario.width as i32 * TILE_PX - self.view.0).max(0);
        let max_y = (self.scenario.height as i32 * TILE_PX - self.view.1).max(0);
        self.camera.0 = self.camera.0.clamp(0, max_x);
        self.camera.1 = self.camera.1.clamp(0, max_y);
    }

    pub(super) fn screen_of(&self, u: &SimUnit) -> (i32, i32) {
        (u.x / 256 - self.camera.0, u.y / 256 - self.camera.1)
    }

    pub(super) fn world_of(&self, sx: i32, sy: i32) -> (i32, i32) {
        (sx + self.camera.0, sy + self.camera.1)
    }

    pub(super) fn handle_input(&mut self, window: &Window) {
        // The controller only produces the same pointer/keys the window
        // produces; it never touches the simulation directly.
        self.pad_frame = self.gamepad.poll();
        if !self.pad_frame.active {
            self.pad_suppressed = false;
        }
        if self.pad_frame.active && self.pad_suppressed {
            // The player moved the mouse: the pad stays ignored until it is
            // released, then the next pad input takes over again.
            self.pad_frame = gamepad::PadFrame::default();
        }
        let frame_now = Instant::now();
        let frame_dt = frame_now
            .duration_since(self.pad_last_poll)
            .as_secs_f32()
            .clamp(0.005, 0.1);
        self.pad_last_poll = frame_now;
        if self.pad_frame.active && !self.pad_wheel.is_open() {
            let (w, h) = window.get_size();
            let (ox, oy) = self.present_offset;
            let sw = (w as i32 - ox * 2).max(1);
            let sh = (h as i32 - oy * 2).max(1);
            if !self.pad_control {
                // Start from the mouse position so the virtual cursor does
                // not jump when the pad is picked up.  `pointer` is in
                // surface pixels, like every other hit test.
                self.pad_cursor = self.pointer.unwrap_or((sw / 2, sh / 2));
            }
            self.pad_control = true;
            let (dx, dy) = self.pad_frame.cursor;
            // CURSOR_SPEED is px per 60 Hz frame; scale by the real frame
            // time so the cursor feels the same in the main menu and in the
            // heavier briefing/ready-room renderer.
            // The sub-pixel rest carries over, so a slight tilt still moves
            // the cursor at high frame rates.
            let step = frame_dt * 60.0;
            let fx = self.pad_cursor_frac.0 + dx * step;
            let fy = self.pad_cursor_frac.1 + dy * step;
            self.pad_cursor_frac = (fx.fract(), fy.fract());
            self.pad_cursor.0 = (self.pad_cursor.0 + fx.trunc() as i32).clamp(0, sw - 1);
            self.pad_cursor.1 = (self.pad_cursor.1 + fy.trunc() as i32).clamp(0, sh - 1);
            if self.pad_minimap_mode && !self.menu && !self.intro {
                let (rx, ry, rw, rh) = self.minimap_rect();
                self.pad_cursor.0 = self.pad_cursor.0.clamp(rx, rx + rw - 1);
                self.pad_cursor.1 = self.pad_cursor.1.clamp(ry, ry + rh - 1);
            }
            self.pointer = Some(self.pad_cursor);
            self.pad_hint_until = Instant::now() + Duration::from_secs(5);
        }
        if let Some(k) = window.get_keys_pressed(KeyRepeat::No).first() {
            self.last_key = Some(format!("{k:?}"));
        }
        if self.intro {
            self.handle_intro(window);
            return;
        }
        if self.menu {
            self.handle_menu(window);
            return;
        }
        if self.briefing {
            self.handle_briefing(window);
            return;
        }
        if self.game_result.is_some() {
            if self.mp.setup.is_some() {
                self.handle_mp_score(window);
            } else {
                self.handle_result(window);
            }
            return;
        }
        if self.ingame_options {
            // The menu's Options code drives the popup over the game.
            self.handle_menu(window);
            if !matches!(self.menu_screen, MenuScreen::Options { .. }) {
                self.ingame_options = false;
                self.menu_screen = MenuScreen::Main;
                self.close_game_dialog();
            }
            return;
        }
        if self.game_dialog.is_some() || self.paused {
            let mut keys = window.get_keys_pressed(KeyRepeat::No);
            let mut mouse = if self.pad_control {
                self.pointer.or(Some(self.pad_cursor))
            } else {
                self.surface_mouse(window)
            };
            let mut lmb = window.get_mouse_down(MouseButton::Left);
            let mut wheel = window
                .get_scroll_wheel()
                .map(|(_, y)| -(y.signum() as i32))
                .unwrap_or(0);
            let mut shift =
                window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
            if self.pad_frame.active {
                keys.extend(self.pad_frame.menu_pressed.iter().copied());
                mouse = Some(self.pad_cursor);
                lmb |= self.pad_frame.lmb;
                wheel += self.pad_frame.wheel;
                shift |= self.pad_frame.shift;
            }
            self.handle_game_dialog_input(&keys, mouse, lmb, wheel, shift);
            return;
        }
        if self.replay.is_some() {
            self.handle_replay(window);
            return;
        }
        let mut input = GameInput {
            pressed: window.get_keys_pressed(KeyRepeat::No),
            ctrl: window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl),
            alt: window.is_key_down(Key::LeftAlt) || window.is_key_down(Key::RightAlt),
            shift: window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift),
            left: window.is_key_down(Key::Left),
            right: window.is_key_down(Key::Right),
            up: window.is_key_down(Key::Up),
            down: window.is_key_down(Key::Down),
            wheel: window.get_scroll_wheel().map_or(0.0, |(_, y)| y),
            pan: (0, 0),
            mouse: self.pointer,
            lmb: window.get_mouse_down(MouseButton::Left),
            rmb: window.get_mouse_down(MouseButton::Right),
            pad: gamepad::PadFrame::default(),
        };
        if self.pad_frame.active {
            let wheel_open = self.pad_wheel.is_open();
            input.pan = if wheel_open {
                (0, 0)
            } else {
                self.pad_frame.pan
            };
            input.mouse = Some(self.pad_cursor);
            // A/B no longer mean left/right mouse buttons in the world:
            // `input.pad` carries the controller edges to the in-game
            // handler, which routes them through the same selection and
            // command-card entry points.
            input.shift |= self.pad_frame.shift;
            input.ctrl |= self.pad_frame.ctrl;
            if !wheel_open {
                input.left |= self.pad_frame.nav.0;
                input.right |= self.pad_frame.nav.1;
                input.up |= self.pad_frame.nav.2;
                input.down |= self.pad_frame.nav.3;
            }
            input.wheel += self.pad_frame.wheel as f32;
            input.pressed.extend(self.pad_frame.pressed.iter().copied());
            input.pad = self.pad_frame.clone();
        }
        self.handle_game_input(&input);
    }

    /// In-game input (keys, camera, selection, orders, command card,
    /// minimap) from one frame's [`GameInput`]: the window and
    /// `--game-script` drive the same code.  The pointer is in surface
    /// pixels; the world is hit-tested in logical game pixels
    /// (`to_logical`), which the selection, box selection and right-click
    /// targets did not do (they compared screen pixels with unit positions,
    /// so nothing could be selected away from 1x scale).
    pub(super) fn handle_game_input(&mut self, input: &GameInput) {
        if self.chat_active {
            let mut submit = false;
            for key in input.pressed.iter().copied() {
                match key {
                    Key::Enter | Key::NumPadEnter => submit = true,
                    Key::Escape => {
                        self.chat_active = false;
                        self.chat_input.clear();
                        return;
                    }
                    Key::Backspace => {
                        self.chat_input.pop();
                    }
                    Key::Space => {
                        if self.chat_input.len() < 200 {
                            self.chat_input.push(' ');
                        }
                    }
                    _ => {
                        if let Some(c) = chat_char(key)
                            && self.chat_input.len() < 200
                        {
                            self.chat_input.push(c);
                        }
                    }
                }
            }
            if submit {
                let text = std::mem::take(&mut self.chat_input);
                self.chat_active = false;
                if !text.trim().is_empty() {
                    self.send_chat(text);
                }
            }
            return;
        }
        let (mx, my) = input.mouse.unwrap_or((0, 0));
        self.track_last_alert();
        self.drag_now = (mx, my);
        let in_view = input.mouse.is_some() && my < self.playfield_bottom();
        if (input.pad.active || self.pad_drag.is_some() || self.pad_wheel.is_open())
            && self.handle_pad_input(input, mx, my, in_view)
        {
            return;
        }

        for key in input.pressed.iter().copied() {
            // Enter opens the message prompt: LAN chat, and the retail
            // single-player cheat-code prompt.
            if matches!(key, Key::Enter | Key::NumPadEnter) {
                if self.net.is_some() || (self.replay.is_none() && !self.observing()) {
                    self.chat_active = true;
                    self.chat_input.clear();
                    self.play("ui_click");
                }
                continue;
            }
            // The retail keystrokes of StarCraft Help (`rez/help_txt.json`).
            if let Some(action) =
                galaxycraft::menu::game_keystroke(key_name(key), input.ctrl, input.alt, input.shift)
            {
                self.run_keystroke(action);
                continue;
            }
            // Retail card hotkeys first (the tooltip's leading letter): M
            // Move, S Stop, A Attack, B Build, ... on the card on screen.
            if !input.ctrl
                && !input.alt
                && let Some(letter) = key_letter(key).or_else(|| {
                    // Esc runs a card Cancel when nothing is pending (the
                    // Esc-only target card is handled below).
                    (key == Key::Escape && self.target_act.is_none() && self.build_mode.is_none())
                        .then_some('\x1b')
                })
                && let Some((_, card)) = self.current_card()
                && let Some(slot) = card
                    .iter()
                    .position(|e| e.is_some_and(|(b, _)| self.button_hotkey(&b) == Some(letter)))
            {
                self.activate_command(slot);
                continue;
            }
            match key {
                Key::F => {
                    self.fog = !self.fog;
                    self.status(if self.fog { "Fog on" } else { "Fog off" });
                }
                Key::F10 => {
                    // Retail F10 opens the Game Menu.
                    self.open_game_dialog(GameDialog::Menu);
                    self.play("ui_click");
                }
                Key::L => {
                    self.show_leaderboard = !self.show_leaderboard;
                }
                Key::Home => self.center_on_base(),
                Key::Space => {
                    // Retail: "Space Bar - Last Transmission" (`help_txt`);
                    // Ctrl-C / Alt-C centre on the unit.
                    if let Some((x, y)) = self
                        .world
                        .triggers
                        .as_ref()
                        .and_then(|t| t.last_transmission(self.local_player))
                    {
                        self.camera = (x - self.view.0 / 2, y - self.view.1 / 2);
                        self.clamp_camera();
                    }
                }
                Key::Tab => {
                    let workers: Vec<u32> = self
                        .world
                        .units
                        .iter()
                        .filter(|u| {
                            u.alive
                                && u.owner as usize == self.local_player
                                && self.data.unit(u.type_id).is_worker()
                                && matches!(u.order, Order::Idle | Order::Stop)
                        })
                        .map(|u| u.id)
                        .collect();
                    if let Some(&id) = workers.first() {
                        self.selected = vec![id];
                        if let Some(u) = self.world.unit(id) {
                            self.camera =
                                (u.x / 256 - self.view.0 / 2, u.y / 256 - self.view.1 / 2);
                            self.clamp_camera();
                        }
                        self.refresh_commands();
                        self.play_selected();
                    }
                }
                Key::Escape => {
                    if self.build_mode.is_some() || self.target_act.is_some() {
                        self.build_mode = None;
                        self.target_act = None;
                    } else if self.card_set.is_some() {
                        self.card_set = None;
                    } else {
                        self.open_game_dialog(GameDialog::Menu);
                        self.play("ui_click");
                    }
                }
                Key::Equal | Key::NumPadPlus => {
                    self.speed = (self.speed * 1.25).min(4.0);
                    self.status(format!("Speed {:.2}x", self.speed));
                }
                Key::Minus | Key::NumPadMinus => {
                    self.speed = (self.speed / 1.25).max(0.25);
                    self.status(format!("Speed {:.2}x", self.speed));
                }
                Key::Key1
                | Key::Key2
                | Key::Key3
                | Key::Key4
                | Key::Key5
                | Key::Key6
                | Key::Key7
                | Key::Key8
                | Key::Key9 => {
                    let n = match key {
                        Key::Key1 => 1,
                        Key::Key2 => 2,
                        Key::Key3 => 3,
                        Key::Key4 => 4,
                        Key::Key5 => 5,
                        Key::Key6 => 6,
                        Key::Key7 => 7,
                        Key::Key8 => 8,
                        _ => 9,
                    };
                    // Retail control groups: Ctrl+N assigns, Shift+N adds,
                    // N recalls (the card has letter hotkeys only).
                    if input.ctrl {
                        self.groups[n] = self.selected.iter().take(12).copied().collect();
                        self.status(format!("Group {n} set"));
                    } else if input.shift {
                        for id in self.selected.clone() {
                            if !self.groups[n].contains(&id) && self.groups[n].len() < 12 {
                                self.groups[n].push(id);
                            }
                        }
                    } else {
                        self.recall_group(n);
                    }
                }
                _ => {}
            }
        }

        // Camera scrolling.  Arrow keys and the right stick; A/S/H/P are
        // unit commands.  The Speed tab's Key Scroll slider scales the
        // keyboard rate around the retail default (0.5 -> 14 px).  The
        // rates are per 60 Hz frame: the window loop's frame time scales
        // them, keeping the fraction, so the camera glides at the same
        // speed whatever the frame rate (scripts step whole frames).
        let k = self.frame_dt.map_or(1.0, |dt| (dt * 60.0).clamp(0.0, 4.0));
        let key = 14.0 * (0.5 + self.game_options.key_scroll);
        let (mut dx, mut dy) = (input.pan.0 as f32, input.pan.1 as f32);
        dx += key * (input.right as i32 - input.left as i32) as f32;
        dy += key * (input.down as i32 - input.up as i32) as f32;
        // Mouse at a screen edge.  INFERENCE: the rate follows the Mouse
        // Scroll slider the way the Key Scroll one scales the keys.
        if let Some(dir) = self.edge_scroll {
            let step = 14.0 * (0.5 + self.game_options.mouse_scroll);
            let (ex, ey) = [
                (0, -1),
                (1, -1),
                (1, 0),
                (1, 1),
                (0, 1),
                (-1, 1),
                (-1, 0),
                (-1, -1),
            ][dir as usize % 8];
            dx += ex as f32 * step;
            dy += ey as f32 * step;
        }
        if dx != 0.0 || dy != 0.0 {
            let fx = self.scroll_frac.0 + dx * k;
            let fy = self.scroll_frac.1 + dy * k;
            self.camera.0 += fx.trunc() as i32;
            self.camera.1 += fy.trunc() as i32;
            self.scroll_frac = (fx.fract(), fy.fract());
        } else {
            self.scroll_frac = (0.0, 0.0);
        }
        if input.wheel != 0.0 {
            // Mouse Scroll Speed: 0.5 is the retail default of 48 px/notch.
            let step = 48.0 * (0.5 + self.game_options.mouse_scroll);
            self.camera.1 -= (input.wheel * step) as i32;
        }
        self.clamp_camera();

        // Mouse.
        if std::env::var("SC_VIEW_DEBUG").is_ok() {
            eprintln!(
                "mouse {} {} downL={} downR={} in_view={} start={:?}",
                mx, my, input.lmb, input.rmb, in_view, self.drag_start
            );
        }
        // HUD command-card clicks (the reusable `statbtndefault` slots).
        let lmb = input.lmb;
        if lmb && !self.lmb_down && !in_view {
            let slots = self.command_slots();
            let group_slot = if self.selected.len() > 1 {
                (0..self.selected.len().min(12)).find(|&n| self.group_slot_rect(n).contains(mx, my))
            } else {
                None
            };
            if let Some(index) = slots.iter().position(|r| r.contains(mx, my)) {
                self.activate_command(index);
            } else if let Some((transport, unit)) = self.cargo_slot_at(mx, my) {
                // Retail: a click on a cargo slot unloads that unit.
                if let Err(e) = self.world.unload_unit(transport, unit) {
                    self.show_sim_error(e);
                }
                self.play("ui_click");
            } else if let Some(n) = group_slot {
                // Retail group panel: a click keeps only that unit, a
                // Shift+click drops it from the selection.
                let id = self.selected[n];
                if input.shift {
                    self.selected.remove(n);
                } else {
                    self.selected = vec![id];
                }
                self.refresh_commands();
                self.play_selected();
            } else if let Some(rect) = self
                .console
                .as_ref()
                .and_then(|c| c.layout("stat_f10"))
                .and_then(|d| d.control(1))
                .map(|c| c.abs.scaled(self.ui_scale, self.ui_ox, self.ui_oy))
                && (rect.contains(mx, my) || self.menu_button_rect().contains(mx, my))
            {
                // The console MENU button (`stat_f10`) opens the Game Menu.
                self.open_game_dialog(GameDialog::Menu);
                self.play("ui_click");
            }
        }
        // Edge of the left button (the minimap terrain toggle fires once
        // per press, not every held frame).
        let lmb_pressed = lmb && !self.lmb_down;
        self.lmb_down = lmb;

        if input.lmb && in_view {
            if self.drag_start.is_none() {
                self.drag_start = Some((mx, my));
            }
        } else if let Some(start) = self.drag_start.take() {
            let dist = (mx - start.0).abs() + (my - start.1).abs();
            if self.target_act.is_some() && dist < 6 {
                self.execute_target(mx, my);
            } else if self.build_mode.is_some() && dist < 6 {
                self.place_building(mx, my);
            } else if dist < 6 {
                let clicked = self.unit_at(mx, my);
                let id = clicked.filter(|id| {
                    self.world
                        .unit(*id)
                        .is_some_and(|u| u.owner as usize == self.local_player)
                });
                // Retail: a plain click on an enemy, neutral or resource
                // unit selects it alone to show its stats (no command
                // card); Shift / Ctrl clicks only take the player's own.
                let foreign = clicked.filter(|_| id.is_none());
                if std::env::var("SC_VIEW_DEBUG").is_ok() {
                    eprintln!("select click at {} {} -> {:?}", mx, my, id);
                }
                if input.ctrl
                    && let Some(id) = id
                    && let Some(t) = self.world.unit(id).map(|u| u.type_id)
                {
                    // Retail Ctrl+click: every own unit of that type on the
                    // screen (12 at most, one for a building).
                    let (vw, vh) = self.view;
                    let limit = if self.data.unit(t).is_building() {
                        1
                    } else {
                        12
                    };
                    let picked: Vec<u32> = self
                        .world
                        .units
                        .iter()
                        .filter(|u| {
                            u.alive && u.owner as usize == self.local_player && u.type_id == t
                        })
                        .filter(|u| {
                            let (x, y) = self.screen_of(u);
                            x >= 0 && y >= 0 && x < vw && y < vh
                        })
                        .map(|u| u.id)
                        .take(limit)
                        .collect();
                    self.selected = picked;
                } else if input.shift {
                    if let Some(id) = id {
                        self.select_unit(id, true);
                    }
                } else if let Some(other) = foreign {
                    self.selected = vec![other];
                    self.refresh_commands();
                    self.play("ui_click");
                    self.drag_start = None;
                    self.rmb_down = input.rmb && in_view;
                    return;
                } else {
                    self.selected = id.into_iter().collect();
                }
                self.refresh_commands();
                self.play_selected();
            } else {
                self.select_box_region(start, (mx, my), input.shift);
            }
        }

        let rmb = input.rmb && in_view;
        if rmb && !self.rmb_down && (self.target_act.is_some() || self.build_mode.is_some()) {
            // Right click cancels a pending target or placement.
            self.target_act = None;
            self.build_mode = None;
            self.rmb_down = rmb;
            return;
        }
        if rmb && !self.rmb_down {
            let attack = self.attack_mode;
            let patrol = self.patrol_mode;
            self.attack_mode = false;
            self.patrol_mode = false;
            self.command_at(mx, my, attack, patrol, input.shift);
        }
        self.rmb_down = rmb;

        // Minimap click: the strip above the radar toggles the terrain layer
        // (retail's "Show Terrain in Minimap" button), the radar itself moves
        // the camera.
        let (rx, ry, rw, rh) = self.minimap_rect();
        let toggle_h = (rw as f32 * 0.09).round() as i32;
        if input.lmb
            && lmb_pressed
            && mx >= rx
            && mx < rx + (rw as f32 * 0.45) as i32
            && my >= ry - toggle_h
            && my < ry
        {
            self.minimap_terrain = !self.minimap_terrain;
            self.status(if self.minimap_terrain {
                "Terrain in minimap: on"
            } else {
                "Terrain in minimap: off"
            });
            self.play("ui_click");
            self.drag_start = None;
        } else if mx >= rx && mx < rx + rw && my >= ry && my < ry + rh {
            let (mw, mh) = self.tileset.minimap_size;
            let tx = (mx - rx) as i64 * mw as i64 / rw as i64;
            let ty = (my - ry) as i64 * mh as i64 / rh as i64;
            let world = (
                tx as i32 * TILE_PX + TILE_PX / 2,
                ty as i32 * TILE_PX + TILE_PX / 2,
            );
            let rmb_pressed = input.rmb && !self.rmb_minimap;
            self.rmb_minimap = input.rmb;
            if lmb_pressed && self.target_act.is_some() {
                // A pending card target can be given on the minimap.
                self.drag_start = None;
                self.execute_target_world(world, None);
            } else if rmb_pressed {
                // Retail: a right click on the minimap is a smart command to
                // that spot.
                self.command_world(world, None, false, false, input.shift);
            } else if input.lmb {
                self.drag_start = None;
                self.camera = (
                    tx as i32 * TILE_PX - self.view.0 / 2,
                    ty as i32 * TILE_PX - self.view.1 / 2,
                );
                self.clamp_camera();
            }
        } else {
            self.rmb_minimap = input.rmb;
        }
    }

    /// SC1 selection rules: a building can only be selected alone; mobile
    /// units cap at twelve.  `additive` is Shift / LT and toggles the unit.
    pub(super) fn select_unit(&mut self, id: u32, additive: bool) {
        if additive {
            if let Some(pos) = self.selected.iter().position(|&x| x == id) {
                self.selected.remove(pos);
                return;
            }
            if self.is_building_id(id) {
                self.selected = vec![id];
                return;
            }
            if self.selected.iter().any(|&x| self.is_building_id(x)) {
                self.selected.clear();
            }
            if self.selected.len() < 12 {
                self.selected.push(id);
            }
        } else {
            self.selected = vec![id];
        }
    }

    pub(super) fn is_building_id(&self, id: u32) -> bool {
        self.world
            .unit(id)
            .is_some_and(|u| self.data.unit(u.type_id).is_building())
    }

    /// Box selection in surface pixels: own units only, at most twelve, and
    /// buildings only when the box holds no mobile unit.
    pub(super) fn select_box_region(&mut self, start: (i32, i32), end: (i32, i32), additive: bool) {
        let (ax, ay) = self.to_logical(start.0, start.1);
        let (bx, by) = self.to_logical(end.0, end.1);
        let (x0, y0) = (ax.min(bx), ay.min(by));
        let (x1, y1) = (ax.max(bx), ay.max(by));
        let mut inside: Vec<(u32, bool)> = self
            .world
            .units
            .iter()
            .enumerate()
            .filter(|(i, u)| {
                u.alive
                    && u.owner as usize == self.local_player
                    && !self.world.inside_gas_building(*i)
            })
            .filter(|(_, u)| {
                let (ux, uy) = self.screen_of(u);
                ux >= x0 && ux <= x1 && uy >= y0 && uy <= y1
            })
            .map(|(_, u)| (u.id, self.data.unit(u.type_id).is_building()))
            .collect();
        // Left edge first, like retail's x-sorted unit finder.
        inside.sort_by_key(|(id, _)| {
            self.world.unit(*id).map_or((0, 0), |u| {
                let left = self.data.unit(u.type_id).dimensions.left as i32;
                (u.x / 256 - left, u.y / 256)
            })
        });
        let any_unit = inside.iter().any(|(_, b)| !b);
        let limit = if any_unit { 12 } else { 1 };
        let picked: Vec<u32> = inside
            .iter()
            .filter(|(_, b)| !any_unit || !b)
            .map(|(id, _)| *id)
            .take(limit)
            .collect();
        if additive {
            for id in picked {
                if !self.selected.contains(&id) {
                    self.select_unit(id, true);
                }
            }
        } else if !picked.is_empty() {
            self.selected = picked;
        }
        self.refresh_commands();
        self.play_selected();
    }

    /// Recall a control group, enforcing the SC1 twelve-unit / one-building
    /// limit and dropping dead members.
    pub(super) fn recall_group(&mut self, n: usize) {
        let group: Vec<u32> = self.groups[n]
            .iter()
            .copied()
            .filter(|id| self.world.unit(*id).is_some_and(|u| u.alive))
            .collect();
        if group.is_empty() {
            self.status(format!("Group {n} is empty"));
            return;
        }
        let selection = normalize_selection(group.iter().map(|&id| (id, self.is_building_id(id))));
        self.selected = selection;
        self.refresh_commands();
        self.play_selected();
        self.status(format!("Group {n}"));
    }

    pub(super) fn centre_on_group(&mut self, n: usize) {
        if let Some(u) = self.groups[n]
            .iter()
            .filter_map(|id| self.world.unit(*id))
            .find(|u| u.alive)
        {
            self.jump_camera_to(u.x / 256, u.y / 256);
            self.status(format!("Group {n}"));
        }
    }

    pub(super) fn jump_camera_to(&mut self, x: i32, y: i32) {
        self.camera = (x - self.view.0 / 2, y - self.view.1 / 2);
        self.clamp_camera();
    }

    /// D-pad cameras: a saved position, or a sensible live fallback for the
    /// base / expansion / front slots before the player saves one.
    pub(super) fn camera_slot_target(&self, slot: usize) -> Option<(i32, i32)> {
        if let Some(pos) = self.map_locations.get(slot).copied().flatten() {
            // Saved slots hold the top-left camera value (keyboard F2-F4
            // convention); the pad path centres on that view.
            return Some((pos.0 + self.view.0 / 2, pos.1 + self.view.1 / 2));
        }
        let base = self
            .world
            .units
            .iter()
            .find(|u| {
                u.alive
                    && u.owner as usize == self.local_player
                    && matches!(
                        u.type_id,
                        uid::COMMAND_CENTER | uid::NEXUS | uid::HATCHERY | uid::LAIR | uid::HIVE
                    )
            })
            .or_else(|| {
                self.world
                    .units
                    .iter()
                    .find(|u| u.alive && u.owner as usize == self.local_player)
            })
            .map(|u| (u.x / 256, u.y / 256));
        match slot {
            0 => base,
            1 => {
                // Nearest resource node outside the main base's mineral line.
                let base = base?;
                self.world
                    .units
                    .iter()
                    .filter(|u| u.alive && u.is_resource())
                    .map(|u| (u.x / 256, u.y / 256))
                    .filter(|&(x, y)| (x - base.0).pow(2) + (y - base.1).pow(2) > 12 * 12)
                    .min_by_key(|&(x, y)| (x - base.0).pow(2) + (y - base.1).pow(2))
                    .or(Some(base))
            }
            _ => {
                let front = self
                    .selected
                    .iter()
                    .filter_map(|id| self.world.unit(*id))
                    .find(|u| {
                        u.alive
                            && !self.data.unit(u.type_id).is_worker()
                            && !self.data.unit(u.type_id).is_building()
                    })
                    .or_else(|| {
                        self.world.units.iter().find(|u| {
                            u.alive
                                && u.owner as usize == self.local_player
                                && !self.data.unit(u.type_id).is_worker()
                                && !self.data.unit(u.type_id).is_building()
                        })
                    })
                    .map(|u| (u.x / 256, u.y / 256));
                front.or(base)
            }
        }
    }

    pub(super) fn save_camera_slot(&mut self, slot: usize) {
        // Store the same top-left camera value as Shift+F2..F4, so the pad
        // and the keyboard share the saved location without redefining the
        // game's hotkeys.
        self.map_locations[slot] = Some(self.camera);
        self.status(format!("Camera {} saved", slot + 1));
        self.play("ui_click");
    }

    pub(super) fn jump_to_last_alert(&mut self) {
        if let Some((x, y)) = self.last_alert_pos {
            self.jump_camera_to(x, y);
            self.status("Last alert");
            return;
        }
        if let Some((x, y)) = self
            .world
            .triggers
            .as_ref()
            .and_then(|t| t.last_transmission(self.local_player))
        {
            self.jump_camera_to(x, y);
            self.status("Last transmission");
        } else {
            self.status("No alert yet");
        }
    }

    /// Sample local-unit hit points once per input frame and remember where
    /// the most recent damage landed.
    pub(super) fn track_last_alert(&mut self) {
        let mut next = Vec::with_capacity(self.world.units.len());
        for u in &self.world.units {
            if u.owner as usize != self.local_player {
                continue;
            }
            if let Some((_, old)) = self.last_health.iter().find(|(id, _)| *id == u.id)
                && (!u.alive || u.hp < *old)
            {
                self.last_alert_pos = Some((u.x / 256, u.y / 256));
            }
            if u.alive {
                next.push((u.id, u.hp));
            }
        }
        self.last_health = next;
    }

    pub(super) fn jump_camera_to_minimap(&mut self, mx: i32, my: i32) {
        let (rx, ry, rw, rh) = self.minimap_rect();
        if rw <= 0 || rh <= 0 {
            return;
        }
        let (mw, mh) = self.tileset.minimap_size;
        let tx = (mx - rx) as i64 * mw as i64 / rw as i64;
        let ty = (my - ry) as i64 * mh as i64 / rh as i64;
        self.jump_camera_to(
            tx as i32 * TILE_PX + TILE_PX / 2,
            ty as i32 * TILE_PX + TILE_PX / 2,
        );
    }

    /// A released without dragging: own unit selects (double tap = every
    /// same type on screen), empty ground clears the selection.  LT toggles
    /// the unit in or out of the selection.
    pub(super) fn pad_click_select(&mut self, mx: i32, my: i32, in_view: bool, additive: bool) {
        if !in_view {
            return;
        }
        if let Some(id) = self.unit_at(mx, my) {
            let Some(unit) = self.world.unit(id).cloned() else {
                return;
            };
            let own = unit.owner as usize == self.local_player;
            if own {
                let type_id = unit.type_id;
                let now_ms = self.pad_started.elapsed().as_millis() as u64;
                let double = !additive && self.pad_tap.tap(now_ms, type_id as u32);
                if double {
                    self.select_same_type_on_screen(type_id);
                    self.status(format!("All {} selected", self.data.unit_name(type_id)));
                } else {
                    self.select_unit(id, additive);
                }
                self.refresh_commands();
                self.play_selected();
            } else {
                // Enemies, resources and neutral units are inspectable from
                // the controller: select the single unit so the status panel
                // shows its name, HP, shields and resource amount.  It gets
                // no command card because `refresh_commands` rejects
                // non-local owners.
                self.selected = vec![id];
                self.pad_tap.reset();
                self.refresh_commands();
                let name = self.data.unit_name(unit.type_id).to_string();
                if unit.is_resource() {
                    self.status(format!("{name}: {}", unit.resource_amount));
                } else {
                    self.status(format!("Enemy {name}"));
                }
                self.play("ui_click");
            }
            return;
        }
        if !additive && !self.selected.is_empty() {
            self.selected.clear();
            self.pad_tap.reset();
            self.refresh_commands();
            self.status("Selection cleared");
        }
    }

    /// A on the HUD: command-card slot or the MENU button.
    pub(super) fn pad_primary(&mut self, _input: &GameInput, mx: i32, my: i32, _in_view: bool) {
        if let Some(index) = self.command_slots().iter().position(|r| r.contains(mx, my)) {
            self.activate_command(index);
            return;
        }
        if self.menu_button_rect().contains(mx, my) {
            self.open_game_dialog(GameDialog::Menu);
            self.play("ui_click");
        }
    }

    /// Ctrl+click's same-type selection, shared by A double tap.
    pub(super) fn select_same_type_on_screen(&mut self, type_id: u16) {
        let (vw, vh) = self.view;
        let limit = if self.data.unit(type_id).is_building() {
            1
        } else {
            12
        };
        self.selected = self
            .world
            .units
            .iter()
            .filter(|u| u.alive && u.owner as usize == self.local_player && u.type_id == type_id)
            .filter(|u| {
                let (x, y) = self.screen_of(u);
                x >= 0 && y >= 0 && x < vw && y < vh
            })
            .map(|u| u.id)
            .take(limit)
            .collect();
    }

    /// B: back out one layer, then clear the selection, then open the menu.
    pub(super) fn pad_cancel(&mut self) {
        if self.target_act.is_some() || self.build_mode.is_some() {
            self.target_act = None;
            self.build_mode = None;
            self.status("Order cancelled");
            return;
        }
        if self.card_set.is_some() {
            self.card_set = None;
            self.status("Command page: base");
            return;
        }
        if self.dequeue_last_production() {
            return;
        }
        if !self.selected.is_empty() {
            self.selected.clear();
            self.pad_tap.reset();
            self.refresh_commands();
            self.status("Selection cleared");
            return;
        }
        self.open_game_dialog(GameDialog::Menu);
        self.play("ui_click");
    }

    /// Y: cycle an idle worker, or an idle army unit when no worker is idle
    /// (or when the current selection is a non-worker).  Mirrors Tab's
    /// centre-on-the-unit behaviour.
    pub(super) fn cycle_idle_unit(&mut self) {
        let workers = self.idle_units(true);
        let army = self.idle_units(false);
        let prefer_army = self
            .first_selected()
            .is_some_and(|u| !self.data.unit(u.type_id).is_worker());
        let (pool, kind) = if prefer_army && !army.is_empty() {
            (army, "army")
        } else if !workers.is_empty() {
            (workers, "worker")
        } else {
            (army, "army")
        };
        if pool.is_empty() {
            self.status("No idle units");
            return;
        }
        let current = self.selected.first().copied();
        let next = current
            .and_then(|id| pool.iter().position(|&u| u == id))
            .map_or(0, |i| (i + 1) % pool.len());
        let id = pool[next];
        self.selected = vec![id];
        self.pad_tap.reset();
        if let Some(u) = self.world.unit(id) {
            self.camera = (u.x / 256 - self.view.0 / 2, u.y / 256 - self.view.1 / 2);
            self.clamp_camera();
        }
        self.refresh_commands();
        self.play_selected();
        self.status(format!("Idle {kind} {}/{}", next + 1, pool.len()));
    }

    pub(super) fn idle_units(&self, workers: bool) -> Vec<u32> {
        self.world
            .units
            .iter()
            .filter(|u| {
                u.alive
                    && u.owner as usize == self.local_player
                    && matches!(u.order, Order::Idle | Order::Stop)
                    && if workers {
                        self.data.unit(u.type_id).is_worker()
                    } else {
                        !self.data.unit(u.type_id).is_worker()
                            && !self.data.unit(u.type_id).is_building()
                            && !u.is_resource()
                    }
            })
            .map(|u| u.id)
            .collect()
    }
}
