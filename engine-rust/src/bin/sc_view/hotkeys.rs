//! `impl App`: hotkeys (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    /// Apply a rebind and run CEF's per-unit collision rules.
    pub(super) fn rebind_hotkey(&mut self, action: u16, text: char, code: u16) {
        let (unbound, error) =
            hotkey_data::apply_rebind(&mut self.hotkey_bindings, action, text, code);
        self.hotkey_unbound = unbound;
        self.hotkey_binding_error = error;
    }

    pub(super) fn refresh_hotkey_errors(&mut self) {
        let hk = hotkey_data::HotkeyData::get();
        self.hotkey_unbound.clear();
        for unit in hk.all_units() {
            let unbound = unit.cards.iter().flat_map(|c| c.actions.iter()).any(|a| {
                self.hotkey_bindings
                    .get(a.id as usize)
                    .copied()
                    .flatten()
                    .is_none()
            });
            if unbound {
                self.hotkey_unbound.push(unit.id);
            }
        }
        self.hotkey_binding_error = !self.hotkey_unbound.is_empty();
    }

    pub(super) fn restore_hotkey_defaults(&mut self) {
        self.hotkey_bindings = hotkey_data::HotkeyData::get().default_bindings();
        self.hotkey_unbound.clear();
        self.hotkey_binding_error = false;
    }

    /// Retail Hotkeys panel (references 131015/131026/131037/131049).
    /// Geometry is expressed in the reference's 2560x1440 content pixels:
    /// plate 354..2202 x 117..1256, title tab and race tabs raised above the
    /// plate top, unit grids 96 px cells with a 9 px gutter, command cards
    /// (3 x 3, 96 px cells) and the Larva rebind dialog.
    pub(super) fn draw_hotkeys(&mut self, fb: &mut Framebuffer) {
        self.hotkey_cells.clear();
        self.hotkey_action_cells.clear();
        // The retail CEF window keeps the space painting and the build
        // footer but replaces the main-menu entries (reference 131015 has no
        // "View Intro" behind the plate).
        self.draw_menu_backdrop_plain(fb);
        let race = match &self.menu_screen {
            MenuScreen::Hotkeys { race } => *race,
            _ => HotkeyRace::Terran,
        };
        let unbound_units: Vec<u16> = self.hotkey_unbound.clone();
        let (cx, _cy, cw, ch) = Self::content_rect(fb);
        let (_scale, _uox, _uoy) = self.ui_metrics(fb);
        let dx = |p: f32| (cw as f32 * p / 2560.0).round() as i32;
        let dy = |p: f32| (ch as f32 * p / 1440.0).round() as i32;
        let sx = |p: f32| cx + dx(p);
        let sy = |p: f32| dy(p);
        let fpx = |p: f32| p * ch as f32 / 1440.0;
        let white = Rgb::new(235, 235, 235);
        let dim = CSS_LAVENDER;
        let green = CSS_GREEN;
        let border = dx(3.0);
        let inner = dx(6.0);

        // Main plate: 3 px #331f1f, 3 px gap, 3 px #6c0000.
        let panel = (sx(354.0), sy(117.0), dx(1849.0), dy(1140.0));
        css_panel3_rounded(fb, panel, 230, border, inner, dx(12.0));

        // Title tab, open at the bottom and overlapping the plate border by
        // 12 px: the 90% fill reaches row 128 and dims both panel bands,
        // while the inner #6c0000 band stops at row 125 (reference 131015).
        let title_tab = (sx(354.0), sy(42.0), dx(424.0), dy(75.0));
        top_rounded_fill(
            fb,
            title_tab.0,
            title_tab.1,
            title_tab.2,
            title_tab.3 + dy(12.0),
            dx(12.0) as f32,
            230,
        );
        tab_border_rounded(fb, title_tab, border, inner, dx(12.0), dy(12.0), CSS_OUTER);
        tab_border_rounded(
            fb,
            (
                title_tab.0 + inner,
                title_tab.1 + inner,
                title_tab.2 - 2 * inner,
                title_tab.3,
            ),
            border,
            inner,
            (dx(12.0) - inner).max(1),
            dy(3.0),
            CSS_INNER,
        );
        let hotkeys_title = self.cef_text("hotkeyPane_title", "Hotkeys");
        if let Some(font) = self.ui_font_hover.as_mut() {
            font.draw_centered_stretched(
                fb,
                title_tab.0 + title_tab.2 / 2,
                sy(83.0),
                &hotkeys_title,
                fpx(42.0),
                0.3,
                1.0,
                1.02,
                white,
                None,
                1.0,
            );
        } else if let Some(font) = self.ui_font.as_mut() {
            font.draw_centered_stretched(
                fb,
                title_tab.0 + title_tab.2 / 2,
                sy(83.0),
                &hotkeys_title,
                fpx(42.0),
                0.3,
                1.0,
                1.02,
                white,
                None,
                1.0,
            );
        }

        // Race tabs: outer 261 x 75, separated by 18 px.
        for (i, r) in HotkeyRace::all().iter().enumerate() {
            let rect = (sx(811.0 + 279.0 * i as f32), sy(42.0), dx(261.0), dy(75.0));
            let selected = *r == race;
            let extra = if selected { dy(9.0) } else { 0 };
            top_rounded_fill(
                fb,
                rect.0,
                rect.1,
                rect.2,
                rect.3 + extra,
                dx(9.0) as f32,
                230,
            );
            tab_border_rounded(fb, rect, border, inner, dx(9.0), extra, CSS_OUTER);
            tab_border_rounded(
                fb,
                (rect.0 + inner, rect.1 + inner, rect.2 - 2 * inner, rect.3),
                border,
                inner,
                (dx(9.0) - inner).max(1),
                if selected { 0 } else { -inner - 3 },
                CSS_INNER,
            );
            let color = if selected || self.hotkey_tab_hover == Some(*r) {
                CSS_GREEN_HI
            } else {
                green
            };
            if let Some(font) = self.ui_font_hover.as_mut() {
                font.draw_centered(
                    fb,
                    rect.0 + rect.2 / 2,
                    sy(71.0),
                    r.name(),
                    fpx(27.0),
                    0.0,
                    color,
                    None,
                );
            } else if let Some(font) = self.ui_font.as_mut() {
                font.draw_centered(
                    fb,
                    rect.0 + rect.2 / 2,
                    sy(71.0),
                    r.name(),
                    fpx(27.0),
                    0.0,
                    color,
                    None,
                );
            }
        }

        // Section headings ("Units" / "Buildings") use the wide face.
        let section_labels = [
            self.cef_text("glue_hotkeysPanel_units_label", "Units"),
            self.cef_text("glue_hotkeysPanel_buildings_label", "Buildings"),
        ];
        for (label, y) in section_labels.iter().zip([187.0f32, 661.0]) {
            if let Some(font) = self.ui_font_hover.as_mut() {
                font.draw_top_stretched(
                    fb,
                    sx(423.0),
                    sy(y),
                    label,
                    fpx(33.0),
                    0.0,
                    1.0,
                    1.04,
                    white,
                    None,
                    1.0,
                );
            } else if let Some(font) = self.ui_font.as_mut() {
                font.draw_top_stretched(
                    fb,
                    sx(423.0),
                    sy(y),
                    label,
                    fpx(33.0),
                    0.0,
                    1.0,
                    1.04,
                    white,
                    None,
                    1.0,
                );
            }
        }

        // Exact lists from the shipped `hotkeypanel.js` (`e.Units`),
        // grouped by the four retail sections.
        let hk = hotkey_data::HotkeyData::get();
        let sub_labels = [
            self.cef_text("glue_hotkeysPanel_groudUnits_label", "Ground Units"),
            self.cef_text("glue_hotkeysPanel_airUnits_label", "Air Units"),
            self.cef_text("glue_hotkeysPanel_basicBuildings_label", "Basic"),
            self.cef_text("glue_hotkeysPanel_advancedBuildings_label", "Advanced"),
        ];
        for (label, (x, y)) in sub_labels.iter().zip([
            (423.0f32, 256.0f32),
            (768.0, 256.0),
            (423.0, 730.0),
            (768.0, 730.0),
        ]) {
            if let Some(font) = self.ui_font.as_mut() {
                font.draw_top(fb, sx(x), sy(y), label, fpx(27.0), 0.0, dim, None);
            } else {
                fb.text(sx(x), sy(y), label, dim, (dy(18.0) / 8).max(1));
            }
        }

        let cell = dx(96.0);
        let step = dx(105.0);
        for (kind, side, cell_y) in [
            ("ground", 0usize, 309.0f32),
            ("air", 1, 309.0),
            ("basic-building", 0, 783.0),
            ("advanced-building", 1, 783.0),
        ] {
            let gx = sx(if side == 0 { 423.0 } else { 768.0 });
            let gy = sy(cell_y);
            for (i, unit) in hk.units(race, Some(kind)).iter().enumerate() {
                let id = unit.id;
                let bx = gx + (i as i32 % 3) * step;
                let by = gy + (i as i32 / 3) * step;
                fb.rect(bx, by, cell, cell, Rgb::new(0, 0, 0));
                if let Some(icon) = self.hotkey_icon(&unit.icon, cell, cell) {
                    blit_rgba_src(&icon, fb, bx, by, cell, cell);
                } else if let Some(console) = &self.console {
                    console.draw_wireframe_gray(fb, id, (bx, by, cell, cell));
                }
                // CEF box-shadow subpixel: the four hairlines render at
                // different coverages (measured 42/48/29/42 on the refs).
                fb.rect(bx, by, cell, 1, Rgb::new(42, 42, 42));
                fb.rect(bx, by, 1, cell, Rgb::new(48, 48, 48));
                fb.rect(bx + cell - 1, by, 1, cell, Rgb::new(29, 29, 29));
                fb.rect(bx, by + cell - 1, cell, 1, Rgb::new(42, 42, 42));
                if self.hotkey_sel == Some(id) {
                    for t in 0..border {
                        fb.rect_outline(bx + t, by + t, cell - 2 * t, cell - 2 * t, CSS_GREEN_HI);
                    }
                } else if self.hotkey_hover == Some(id) {
                    for t in 0..border {
                        fb.rect_outline(bx + t, by + t, cell - 2 * t, cell - 2 * t, CSS_GREEN);
                    }
                }
                if unbound_units.contains(&id) {
                    draw_unbound_badge(fb, bx, by, dx(1.0), dy(1.0));
                    if let Some(font) = self.ui_font.as_mut() {
                        font.draw_top(
                            fb,
                            bx + dx(13.0),
                            by + dy(10.0),
                            "?",
                            fpx(19.0),
                            0.0,
                            Rgb::new(0, 0, 0),
                            None,
                        );
                    }
                }
                self.hotkey_cells.push((id, (bx, by, cell, cell)));
            }
        }

        // Right column: 3 px #1e1e1e divider, command cards per unit.
        fb.rect(
            sx(1113.0),
            sy(126.0),
            dx(3.0),
            dy(1122.0),
            Rgb::new(30, 30, 30),
        );
        let hotkey_unit = self
            .hotkey_sel
            .or_else(|| self.first_selected().map(|u| u.type_id));
        if let Some(id) = hotkey_unit {
            let race_name = race.name();
            let raw_name = self.data.unit_name(id).to_string();
            let name = raw_name
                .strip_prefix(race_name)
                .map(str::trim)
                .unwrap_or(&raw_name)
                .to_string();
            let unit_label = format!("{race_name} {name}");
            let selected_prefix = self.cef_text("glue_hotkeysPanel_selected_prefix", "Selected:");
            if let Some(font) = self.ui_font_hover.as_mut() {
                font.draw_top_stretched(
                    fb,
                    sx(1163.0),
                    sy(187.0),
                    &selected_prefix,
                    fpx(33.0),
                    0.0,
                    1.0,
                    1.10,
                    Rgb::new(221, 221, 221),
                    None,
                    0.9,
                );
                let selected_w = font.measure(&selected_prefix, fpx(33.0), 0.0);
                font.draw_top_stretched(
                    fb,
                    sx(1163.0) + selected_w + dx(15.0),
                    sy(187.0),
                    &unit_label,
                    fpx(33.0),
                    0.0,
                    1.0,
                    1.10,
                    Rgb::new(245, 245, 245),
                    None,
                    0.9,
                );
            } else if let Some(font) = self.ui_font.as_mut() {
                font.draw_top_stretched(
                    fb,
                    sx(1163.0),
                    sy(187.0),
                    &selected_prefix,
                    fpx(33.0),
                    0.0,
                    1.0,
                    1.10,
                    Rgb::new(221, 221, 221),
                    None,
                    0.9,
                );
                let selected_w = font.measure(&selected_prefix, fpx(33.0), 0.0);
                font.draw_top_stretched(
                    fb,
                    sx(1163.0) + selected_w + dx(15.0),
                    sy(187.0),
                    &unit_label,
                    fpx(33.0),
                    0.0,
                    1.0,
                    1.10,
                    Rgb::new(245, 245, 245),
                    None,
                    0.9,
                );
            }
            let hk = hotkey_data::HotkeyData::get();
            let cards = hk.unit(id).map(|u| u.cards.as_slice()).unwrap_or(&[]);
            let card_x = [1164.0f32, 1504.0, 1843.0];
            let grid_y = sy(304.0);
            let chip = dx(30.0);
            let chip_off = dx(3.0);
            let wrap_px = 3 * step + dy(30.0);
            // A fourth card (Zerg Drone `generalBurrowed`) wraps to a second
            // row and pushes the CEF instruction line down.
            let hint_y = if cards.len() > 3 { 1000.0 } else { 639.0 };
            let card_titles: Vec<String> = cards
                .iter()
                .map(|card| {
                    self.action_card_label(hotkey_data::HotkeyData::card_title(&card.label))
                })
                .collect();
            if let Some(font) = self.ui_font.as_mut() {
                for (ci, _) in cards.iter().enumerate() {
                    let col = ci % 3;
                    let block = ci / 3;
                    font.draw_top(
                        fb,
                        sx(card_x[col]),
                        sy(255.0) + block as i32 * wrap_px,
                        &card_titles[ci],
                        fpx(27.6),
                        0.0,
                        dim,
                        None,
                    );
                }
            }
            for (ci, card) in cards.iter().enumerate() {
                // The third actions panel sits one pixel further left in
                // the CEF flex layout than its heading's glyph origin.
                let col = ci % 3;
                let block = ci / 3;
                let x = sx(card_x[col] + if col == 2 { 1.0 } else { 0.0 });
                let gy = grid_y + block as i32 * wrap_px;
                let mut occupied = [[false; 3]; 3];
                for a in &card.actions {
                    let cx = (a.x - 1) as i32;
                    let cy = (a.y - 1) as i32;
                    if !(0..3).contains(&cx) || !(0..3).contains(&cy) {
                        continue;
                    }
                    occupied[cy as usize][cx as usize] = true;
                    let bx = x + cx * step;
                    let by = gy + cy * step;
                    fb.rect(bx, by, cell, cell, Rgb::new(0, 0, 0));
                    // The shipped panel special-cases Probe Build/Advanced to
                    // the warp-gate art (278/279 -> 272/273).
                    let icon_name = if id == 64 && a.id == 678 {
                        "cmdicons.272.png".to_string()
                    } else if id == 64 && a.id == 679 {
                        "cmdicons.273.png".to_string()
                    } else {
                        a.icon.clone()
                    };
                    if let Some(icon) = self.hotkey_icon(&icon_name, cell, cell) {
                        blit_tinted_rgba(
                            &icon,
                            fb,
                            bx,
                            by,
                            cell,
                            cell,
                            Rgb::new(245, 245, 0),
                            false,
                        );
                    } else if let Some(console) = &self.console {
                        console.draw_command_icon_ink(fb, a.id as usize, (bx, by, cell, cell));
                    }
                    let key = self.hotkey_bindings.get(a.id as usize).copied().flatten();
                    if key.is_none() {
                        // CEF `.action.bindingError:after` red 20 % wash.
                        fill_alpha_rect(fb, bx, by, cell, cell, 51);
                    }
                    if self.hotkey_action == Some(a.id) {
                        for t in 0..border {
                            fb.rect_outline(
                                bx + t,
                                by + t,
                                cell - 2 * t,
                                cell - 2 * t,
                                CSS_GREEN_HI,
                            );
                        }
                    } else {
                        // .action hairlines: 25 top, 22 left, 42 right/bottom.
                        fb.rect(bx, by, cell, 1, Rgb::new(25, 25, 25));
                        fb.rect(bx, by, 1, cell, Rgb::new(25, 25, 25));
                        fb.rect(bx + cell - 1, by, 1, cell, Rgb::new(42, 42, 42));
                        fb.rect(bx, by + cell - 1, cell, 1, Rgb::new(42, 42, 42));
                    }
                    if let Some(key) = key {
                        fill_alpha_rect(fb, bx + chip_off, by + chip_off, chip, chip, 153);
                        let key_label = key.text.to_ascii_uppercase().to_string();
                        if let Some(font) = self.ui_font.as_mut() {
                            font.draw_centered_stretched(
                                fb,
                                bx + chip_off + chip / 2,
                                by + chip_off + dy(7.0),
                                &key_label,
                                fpx(std::env::var("GC_KEY_PX")
                                    .ok()
                                    .and_then(|v| v.parse().ok())
                                    .unwrap_or(20.5)),
                                0.0,
                                1.0,
                                std::env::var("GC_KEY_YS")
                                    .ok()
                                    .and_then(|v| v.parse().ok())
                                    .unwrap_or(1.07),
                                white,
                                None,
                                1.0,
                            );
                        }
                    }
                    self.hotkey_action_cells.push((a.id, (bx, by, cell, cell)));
                }
                for (row, used) in occupied.iter().enumerate() {
                    for (col, used) in used.iter().enumerate() {
                        if !used {
                            let bx = x + col as i32 * step;
                            let by = gy + row as i32 * step;
                            fb.rect(bx, by, cell, cell, Rgb::new(35, 35, 35));
                        }
                    }
                }
            }
            let rebind = self.hotkey_action.and_then(|aid| {
                let action = hk.action(aid)?;
                Some((
                    action.label.clone(),
                    action.default_text.to_ascii_uppercase(),
                    action.icon.clone(),
                    hk.shares(aid),
                    self.hotkey_bindings.get(aid as usize).copied().flatten(),
                ))
            });
            let command_hint = self.cef_text(
                "glue_hotkeysPanel_commandSelection_instructions",
                "Click on a command to change its hotkey",
            );
            if rebind.is_none()
                && let Some(font) = self.ui_font.as_mut()
            {
                let phase: f32 = std::env::var("GC_HINT_PHASE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.2);
                font.draw_centered_weighted_phase(
                    fb,
                    sx(1654.0),
                    phase,
                    sy(hint_y),
                    &command_hint,
                    fpx(27.6),
                    0.0,
                    dim,
                    None,
                    1.0,
                );
            }
            let press_hint = self.cef_text(
                "glue_hotkeysPanel_hotkey_instructions",
                "Press the desired key",
            );
            let unbound_hint = self.cef_text(
                "glue_hotkeysPanel_unboundedInstructions",
                "Some commands have no key assigned",
            );
            let default_prefix = self.cef_text("glue_hotkeysPanel_binding_default", "Default:");
            if let Some((label, def, icon_name, shares, current)) = rebind {
                if let Some(font) = self.ui_font.as_mut() {
                    if !self.hotkey_binding_error {
                        font.draw_centered(
                            fb,
                            sx(1655.0),
                            sy(1021.0),
                            &press_hint,
                            fpx(25.7),
                            0.0,
                            Rgb::new(221, 221, 221),
                            None,
                        );
                    } else {
                        // CEF `.bindingErrorInstructions` (#d60e00, 8 px).
                        font.draw_centered_weighted(
                            fb,
                            sx(1655.0),
                            sy(1021.0),
                            &unbound_hint,
                            fpx(25.7),
                            0.0,
                            Rgb::new(214, 14, 0),
                            None,
                            1.0,
                        );
                    }
                }
                // Rebind panel (reference 131049): gray plate from x1113 to
                // the plate edge, icon at 1161, label at 1289.
                let pr = (sx(1113.0), sy(1053.0), dx(1080.0), dy(195.0));
                // 3 px border + 3 px black gap + #1c1c1c body, as measured.
                fb.rect(pr.0, pr.1, pr.2, pr.3, Rgb::new(0, 0, 0));
                fb.rect(
                    sx(1119.0),
                    sy(1059.0),
                    dx(1071.0),
                    dy(182.0),
                    Rgb::new(28, 28, 28),
                );
                fb.rect(pr.0, pr.1, pr.2, border, Rgb::new(30, 30, 30));
                fb.rect(pr.0, pr.1, border, pr.3, Rgb::new(30, 30, 30));
                fb.rect(
                    pr.0,
                    pr.1 + pr.3 - border,
                    pr.2,
                    border,
                    Rgb::new(30, 30, 30),
                );
                let ibx = sx(1164.0);
                let iby = sy(1098.0);
                fb.rect(ibx, iby, cell, cell, Rgb::new(0, 0, 0));
                if let Some(icon) = self.hotkey_icon(&icon_name, cell, cell) {
                    blit_tinted_rgba(
                        &icon,
                        fb,
                        ibx,
                        iby,
                        cell,
                        cell,
                        Rgb::new(245, 245, 0),
                        false,
                    );
                }
                fb.rect_outline(ibx, iby, cell, cell, Rgb::new(45, 45, 45));
                let mut label_lines = label.lines();
                let first_line = label_lines.next().unwrap_or("");
                let second_line = label_lines.next();
                if let Some(font) = self.ui_font_hover.as_mut() {
                    font.draw_top_stretched(
                        fb,
                        sx(1287.0),
                        sy(1104.0),
                        first_line,
                        fpx(27.0),
                        0.0,
                        1.0,
                        std::env::var("GC_MORPH_YS")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(1.0),
                        Rgb::new(238, 238, 238),
                        None,
                        1.0,
                    );
                }
                if let Some(second_line) = second_line
                    && let Some(font) = self.ui_font.as_mut()
                {
                    font.draw_top(
                        fb,
                        sx(1287.0),
                        sy(1128.0),
                        second_line,
                        fpx(22.0),
                        0.0,
                        Rgb::new(190, 190, 190),
                        None,
                    );
                }
                if let Some(font) = self.ui_font_hover.as_mut() {
                    if shares > 1
                        && let Some(font) = self.ui_font.as_mut()
                    {
                        font.draw_top(
                            fb,
                            sx(1287.0),
                            sy(if second_line.is_some() {
                                1158.0
                            } else {
                                1136.0
                            }),
                            &format!("{shares} units share this command"),
                            fpx(27.0),
                            0.0,
                            dim,
                            None,
                        );
                    }
                    let def_top = if second_line.is_some() {
                        1190.0
                    } else if shares > 1 {
                        1172.0
                    } else {
                        1143.0
                    };
                    font.draw_top_stretched(
                        fb,
                        sx(1287.0),
                        sy(def_top),
                        &format!("{default_prefix} {def}"),
                        fpx(27.0),
                        0.0,
                        1.0,
                        std::env::var("GC_DEF_YS")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0.96),
                        Rgb::new(150, 150, 155),
                        None,
                        1.0,
                    );
                }
                let kb = (sx(1966.0), sy(1098.0), dx(150.0), dy(64.0));
                fb.rect(kb.0, kb.1, kb.2, kb.3, Rgb::new(35, 35, 35));
                for t in 0..border {
                    fb.rect_outline(
                        kb.0 + t,
                        kb.1 + t,
                        kb.2 - 2 * t,
                        kb.3 - 2 * t,
                        Rgb::new(128, 128, 128),
                    );
                }
                let shown = current
                    .map(|k| k.text.to_ascii_uppercase().to_string())
                    .unwrap_or_default();
                if let Some(font) = self.ui_font_hover.as_mut() {
                    font.draw_centered(
                        fb,
                        kb.0 + kb.2 / 2 - dx(1.0),
                        sy(1118.0),
                        &shown,
                        fpx(33.0),
                        0.0,
                        Rgb::new(240, 240, 240),
                        None,
                    );
                }
            }
        } else {
            let unit_hint = self.cef_text(
                "glue_hotkeysPanel_unitSelection_instructions",
                "Click on a unit to view its commands",
            );
            if let Some(font) = self.ui_font.as_mut() {
                font.draw_centered(
                    fb,
                    sx(1653.0),
                    sy(440.0),
                    &unit_hint,
                    fpx(27.0),
                    0.0,
                    dim,
                    None,
                );
            } else {
                let hw = unit_hint.chars().count() as i32 * 8 * (dy(24.0) / 8).max(1);
                fb.text(
                    sx(1653.0) - hw / 2,
                    sy(440.0),
                    &unit_hint,
                    dim,
                    (dy(24.0) / 8).max(1),
                );
            }
        }

        // Bottom buttons: outer #331f1f group + #6c0000 button border.
        let restore_label = self.cef_text(
            "glue_hotkeysPanel_restoreToDefault_button",
            "Restore to Default",
        );
        let save_label = self.cef_text("glue_hotkeysPanel_save_button", "Save");
        let cancel_label = self.cef_text("glue_hotkeysPanel_cancel_button", "Cancel");
        for (label, gx0, gy0, gw, gh, rw, rh) in [
            (
                restore_label.as_str(),
                354.0f32,
                1302.0f32,
                390.0f32,
                84.0f32,
                372.0f32,
                66.0f32,
            ),
            (
                save_label.as_str(),
                1453.0,
                1302.0,
                378.0,
                84.0,
                360.0,
                66.0,
            ),
            (
                cancel_label.as_str(),
                1825.0,
                1302.0,
                378.0,
                84.0,
                360.0,
                66.0,
            ),
        ] {
            let group = (sx(gx0), sy(gy0), dx(gw), dy(gh));
            // Retail button group: 5 CSS px radius with a 1.5 CSS px brown
            // band; the inner button uses 3 CSS px and a 1 CSS px red band.
            rounded_fill_alpha(fb, group.0, group.1, group.2, group.3, dx(15.0) as f32, 205);
            rounded_band(
                fb,
                group.0,
                group.1,
                group.2,
                group.3,
                dx(15.0) as f32,
                0,
                dx(4.0),
                CSS_OUTER,
            );
            let btn = (group.0 + dx(9.0), group.1 + dx(9.0), dx(rw), dy(rh));
            rounded_fill_alpha(fb, btn.0, btn.1, btn.2, btn.3, dx(9.0) as f32, 205);
            rounded_band(
                fb,
                btn.0,
                btn.1,
                btn.2,
                btn.3,
                dx(9.0) as f32,
                0,
                border,
                CSS_INNER,
            );
            if let Some(font) = self.ui_font_hover.as_mut() {
                font.draw_centered_stretched(
                    fb,
                    btn.0 + btn.2 / 2,
                    btn.1 + dy(21.0),
                    label,
                    fpx(30.0),
                    0.0,
                    1.0,
                    1.10,
                    green,
                    None,
                    1.0,
                );
            } else if let Some(font) = self.ui_font.as_mut() {
                font.draw_centered_stretched(
                    fb,
                    btn.0 + btn.2 / 2,
                    btn.1 + dy(21.0),
                    label,
                    fpx(30.0),
                    0.0,
                    1.0,
                    1.10,
                    green,
                    None,
                    1.0,
                );
            } else {
                let lw = label.chars().count() as i32 * 8 * (dy(30.0) / 8).max(1);
                fb.text(
                    btn.0 + (btn.2 - lw) / 2,
                    btn.1 + dy(18.0),
                    label,
                    green,
                    (dy(30.0) / 8).max(1),
                );
            }
        }

        // Restore-defaults confirmation (reference 120018): the CEF page and
        // the in-game buttons behind the modal dim to 40 %, the dialog itself
        // stays opaque black.
        if self.hotkey_reset {
            // The CEF modal overlay covers the whole hotkey container,
            // including the bottom button strip and the space between it and
            // the panel (all samples give a 0.40 factor).
            fill_alpha_rect(fb, sx(354.0), sy(42.0), dx(1849.0), dy(1398.0), 153);
            let reset_question = self.cef_text(
                "glue_hotkeysPanel_restoreDefaultsQuestion",
                "Restore defaults?",
            );
            self.draw_confirm_dialog(fb, &reset_question, (735.0, 486.0, 1087.0, 468.0), false);
        }
    }
}
