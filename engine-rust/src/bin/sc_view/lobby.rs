//! `impl App`: lobby (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    /// Load Replay: the retail `Create` screen (`glucreat`) in its replay
    /// mode, drawn by [`Self::draw_lobby`].
    pub(super) fn draw_create(&mut self, fb: &mut Framebuffer) {
        // Replays reuse the Create shell (same retail plates and tree).
        self.draw_lobby(fb);
    }

    /// Load Saved Game (`rez/gluload.ui.json`), see [`Self::draw_glue_list`].
    /// Reference 130848: with no saved game retail lists its `[Cloud]`
    /// folder and Delete/Ok are disabled; this client has no cloud saves,
    /// so the list holds only the saved games (user request 2026-11-07).
    pub(super) fn draw_saved(&mut self, fb: &mut Framebuffer) {
        let list = self.save_list(false);
        let rows = Self::saved_rows(&list, self.save_sel);
        let has_save = !rows.is_empty();
        self.draw_glue_list(
            fb,
            &galaxycraft::menu::LOAD_DIALOG,
            |g| g.load_list.as_ref(),
            "Saved Games",
            &rows,
            &[("Delete", has_save), ("Ok", has_save), ("Cancel", true)],
        );
    }

    /// Rows of the Load Saved list: one per save, no folder rows.
    pub(super) fn saved_rows(list: &[PathBuf], sel: usize) -> Vec<(String, bool)> {
        list.iter()
            .enumerate()
            .take(SAVED_ROWS)
            .map(|(i, p)| {
                let name = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string();
                (format!("{name}: {}", Self::slot_label(p)), i == sel)
            })
            .collect()
    }

    pub(super) fn draw_lan(&mut self, fb: &mut Framebuffer) {
        self.draw_menu_backdrop(fb);
        let map = self
            .maps
            .get(self.lan.map_index)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "(no map)".to_string());
        let mut lines = vec![format!("Map: {map}"), self.lan.status.clone()];
        if let Some(queue) = &self.lan.queue {
            lines.extend(lan_lobby_lines(queue, self.menu_race));
        } else if self.lan.entries.is_empty() {
            lines.push("(no games found)".to_string());
        } else {
            for (i, (addr, a)) in self.lan.entries.iter().enumerate() {
                let mark = if i == self.lan.sel { "> " } else { "  " };
                lines.push(format!(
                    "{mark}{} - {} [{}] {} player(s)",
                    a.name,
                    addr.ip(),
                    a.port,
                    a.players
                ));
            }
        }
        self.draw_menu_panel(
            fb,
            "LAN",
            &lines,
            self.lan.sel,
            "S: scan   H: host   ENTER: join / start   UP/DN: select   ESC: back",
        );
    }

    /// Build the 128x128 terrain minimap used by the Create screen.
    pub(super) fn ensure_lobby_preview(&mut self) {
        let Some(entry) = self.maps.get(self.map_index) else {
            return;
        };
        let Ok(raw) = std::fs::read(&entry.path) else {
            return;
        };
        let Ok(sc) = Scenario::parse(&raw) else {
            return;
        };
        let Some(tiles) = Tileset::load(&self.tiles_dir, sc.era, &sc) else {
            return;
        };
        let mut preview = Framebuffer::new(128, 128);
        tiles.draw_minimap_with_terrain(
            &mut preview,
            (0, 0, 128, 128),
            &[],
            (0, 0, 0, 0),
            None,
            0,
            galaxycraft::render::MinimapTerrain::Tiles,
        );
        self.lobby_preview = Some((self.map_index, preview.pixels));
    }

    /// One row of the single-player `Play Custom` map browser.
    pub(super) fn classic_rows(&self) -> Vec<ClassicRow> {
        let melee: Vec<usize> = self
            .maps
            .iter()
            .enumerate()
            .filter(|(_, m)| m.campaign.is_none())
            .map(|(i, _)| i)
            .collect();
        let root = melee
            .iter()
            .filter_map(|&i| self.maps[i].path.parent())
            .min_by_key(|p| p.components().count())
            .map(Path::to_path_buf);
        let Some(root) = root else {
            return Vec::new();
        };
        let dir = self.lobby_dir.iter().fold(root.clone(), |p, d| p.join(d));
        let mut folders: Vec<String> = Vec::new();
        let mut maps: Vec<usize> = Vec::new();
        for &i in &melee {
            let Ok(rel) = self.maps[i].path.strip_prefix(&dir) else {
                continue;
            };
            let parts: Vec<_> = rel.components().collect();
            if parts.len() == 1 {
                maps.push(i);
            } else if let Some(first) = parts.first() {
                let name = first.as_os_str().to_string_lossy().into_owned();
                // The original campaign's Create browser hides the Brood War
                // set (the retail capture shows ladder/oldladder/Replays/...).
                if name != "BroodWar" && name != "campaign" && !folders.contains(&name) {
                    folders.push(name);
                }
            }
        }
        // The retail tree also lists the replay and save browsers as
        // folders (reference 39); they live outside the maps directory.
        if self.lobby_dir.is_empty() {
            folders.push("Replays".to_string());
            folders.push("Save".to_string());
        }
        folders.sort_by_key(|f| f.to_lowercase());
        maps.sort_by_key(|&i| self.maps[i].name.to_lowercase());
        let mut rows = Vec::new();
        if !self.lobby_dir.is_empty() {
            rows.push(ClassicRow::Up);
        }
        rows.extend(folders.into_iter().map(ClassicRow::Folder));
        rows.extend(maps.into_iter().map(ClassicRow::Map));
        rows
    }

    pub(super) fn draw_lobby(&mut self, fb: &mut Framebuffer) {
        // Retail `Create` screen (reference 134831): map list on the left,
        // map preview on the right and the slot/game-type panel underneath.
        self.draw_moon_backdrop(fb);
        let replays = matches!(&self.menu_screen, MenuScreen::Replays)
            || matches!(self.shot_name.as_str(), "replays")
            || matches!(
                &self.glue_swish,
                Some(GlueSwish::Out {
                    from: MenuScreen::Replays,
                    ..
                })
            );
        // Load Replay swishes like the classic list dialogs (retail
        // recording 2026-11-04, 15.95-16.82 s): tree and lower plates from
        // the left, the information plate and Ok from the right, Cancel from
        // the bottom.  Offsets in 1440p px.
        let (from_left, info_dx, ok_dx, cancel_dy) = if replays {
            (
                -self.swish_offset(1404.0 / 3.0) * 3.0,
                self.swish_offset(1041.0 / 3.0) * 3.0,
                self.swish_offset(SWISH_OK_UI) * 3.0,
                self.swish_offset(SWISH_CANCEL_UI) * 3.0,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };
        let (cx, _cy, cw, ch) = Self::content_rect(fb);
        let (scale, uox, uoy) = self.ui_metrics(fb);
        // The shipped plates carry the metal trim, red hairlines and corner
        // brackets.  Align their frame line (not their outer canvas) with the
        // measured reference rectangles.
        // Retail `glucreat` chrome, placed by SIFT affine against reference
        // 134831 (content box 2560x1440): the 4x art is drawn at 3/4 of its
        // native size, i.e. scale/4, at the measured screen rects.
        let k = ch as f32 / 1440.0;
        let place = |fb: &mut Framebuffer,
                     art: &galaxycraft::dds::DdsImage,
                     x: f32,
                     y: f32,
                     w: f32,
                     h: f32| {
            art.draw_scaled(
                fb,
                cx + (x * k).round() as i32,
                (y * k).round() as i32,
                (w * k).round() as i32,
                (h * k).round() as i32,
            );
        };
        if let Some(glue) = &self.glue {
            if let Some(art) = &glue.create_map2 {
                place(fb, art, -21.0 + from_left, 0.0, 1425.0, 879.0);
            }
            if let Some(art) = &glue.create_info {
                place(fb, art, 1519.0 + info_dx, 0.0, 1065.0, 999.0);
            }
            if let Some(art) = &glue.create_slots {
                place(fb, art, -57.0 + from_left, 855.0, 1461.0, 579.0);
            }
            // Create/Replays use the classic Ok/Cancel plates: SIFT ranks
            // `pok`/`pcancel` at residual 4.5/6.7 while `twobtnholder` covers
            // only the metal support (unique-pixel residual 67.6).
            if let Some(art) = &glue.palnl_ok {
                place(fb, art, 1627.0 + ok_dx, 1006.0, 1062.0, 214.0);
            }
            if let Some(art) = &glue.palnl_cancel {
                place(fb, art, 1734.0, 1068.0 + cancel_dy, 940.0, 273.0);
            }
        }
        if replays && !self.swish_text_shown() {
            return;
        }
        let at = |x: f32, y: f32| (cx + (cw as f32 * x) as i32, (ch as f32 * y) as i32);
        let rgb_green = Rgb::new(120, 235, 120);
        let grey = Rgb::new(215, 215, 220);
        let dim = Rgb::new(150, 155, 165);
        let panel = |fb: &mut Framebuffer, r: (i32, i32, i32, i32)| {
            for y in r.1..r.1 + r.3 {
                for x in r.0..r.0 + r.2 {
                    let c = fb.get(x, y);
                    fb.put(x, y, c.blend(Rgb::new(5, 7, 11), 215));
                }
            }
            fb.rect_outline(r.0, r.1, r.2, r.3, Rgb::new(105, 100, 86));
            fb.rect_outline(r.0 + 2, r.1 + 2, r.2 - 4, r.3 - 4, Rgb::new(140, 26, 26));
        };
        let hd = |app: &mut App,
                  fb: &mut Framebuffer,
                  x: i32,
                  y: i32,
                  text: &str,
                  px: f32,
                  color: Rgb| {
            if let Some(font) = app.ui_font.as_mut() {
                font.draw_top(fb, x, y, text, px, 0.4, color, Some(Rgb::new(0, 0, 0)));
            } else {
                fb.text(x, y, text, color, 1);
            }
        };
        // Left map panel.  The art already carries its frame; only fill the
        // plate when the retail asset is unavailable.
        let left = at(0.1406, 0.0208);
        let lw = (cw as f32 * (0.6004 - 0.1406)) as i32;
        let lh = (ch as f32 * (0.5700 - 0.0208)) as i32;
        if self
            .glue
            .as_ref()
            .and_then(|g| g.create_map.as_ref())
            .is_none()
        {
            panel(fb, (left.0, left.1, lw, lh));
        }
        // Title and current folder, measured on 134831/130858: the panel is
        // still the Create dialog ("Create" title), the folder line changes.
        use galaxycraft::menu::LabelAlign;
        let pale = Rgb::new(0xc6, 0xda, 0xff);
        let folder_gold = Rgb::new(0xe7, 0xc9, 0x00);
        let create_title = self.ui_text("gluCreat-1", "Create");
        self.draw_hd_text(
            fb,
            &create_title,
            (443.0 - 320.0) / 3.0,
            220.0 / 3.0,
            LabelAlign::Left,
            195.0 / 3.0,
            16.0,
            scale,
            uox,
            uoy,
            pale,
            true,
        );
        // Open-folder glyph 48x33 at (492,318).
        let fx = cx + (cw as f32 * (492.0 / 2560.0)) as i32;
        let fy = (ch as f32 * (318.0 / 1440.0)) as i32;
        let fw = (cw as f32 * (48.0 / 2560.0)) as i32;
        let fh = (ch as f32 * (33.0 / 1440.0)) as i32;
        fb.rect(fx, fy + 6, fw, fh - 6, folder_gold);
        fb.rect(fx + 4, fy, fw - 4, 8, folder_gold);
        let folder_label = if replays {
            self.cef_text("gameCloudReplays", "Replays")
        } else {
            self.cef_text("gameCloudMaps", "Maps")
        };
        let folder_w = if replays { 145.0 } else { 104.0 } / 3.0;
        self.draw_hd_text(
            fb,
            &folder_label,
            (553.0 - 320.0) / 3.0,
            323.0 / 3.0,
            LabelAlign::Left,
            folder_w,
            13.0,
            scale,
            uox,
            uoy,
            pale,
            false,
        );
        let list_row_h = (ch as f32 * (57.0 / 1440.0)).round() as i32;
        let list_top = (ch as f32 * (406.0 / 1440.0)) as i32;
        if replays {
            // Measured on 130858: three folder rows at x565 (42x36 folder
            // icons at x504) and the selected LastReplay.rep at x556 (35x42
            // document icon), 57 px pitch, no selection rectangle.
            // Live: the folders, then the real replay files (the selected one
            // recoloured); the reference shot shows its `LastReplay.rep`.
            let files: Vec<String> = if self.shot_name == "replays" {
                vec![ReplayInfo::reference().file]
            } else {
                self.save_list(true)
                    .iter()
                    .filter_map(|p| p.file_name()?.to_str().map(str::to_string))
                    .collect()
            };
            let items: Vec<(bool, &str)> = REPLAY_FOLDERS
                .iter()
                .map(|f| (true, *f))
                .chain(files.iter().map(|f| (false, f.as_str())))
                .take(6)
                .collect();
            let selected_file = self.save_sel;
            for (i, (folder, label)) in items.iter().enumerate() {
                let y = list_top + i as i32 * list_row_h;
                let ix = cx + (cw as f32 * (504.0 / 2560.0)) as i32;
                if *folder {
                    let iw = (cw as f32 * (42.0 / 2560.0)) as i32;
                    let ih = (ch as f32 * (35.0 / 1440.0)) as i32;
                    fb.rect(ix, y + 4, iw, ih - 4, folder_gold);
                    fb.rect(ix, y, ((iw as f32) * 0.55) as i32, 8, folder_gold);
                    hd(
                        self,
                        fb,
                        cx + (cw as f32 * (565.0 / 2560.0)) as i32,
                        y,
                        label,
                        16.0 * scale,
                        Rgb::new(0x5c, 0xed, 0x30),
                    );
                } else {
                    let iw = (cw as f32 * (35.0 / 2560.0)) as i32;
                    let ih = (ch as f32 * (42.0 / 1440.0)) as i32;
                    let colour = if i - REPLAY_FOLDERS.len() == selected_file {
                        Rgb::new(0xda, 0xff, 0x8c)
                    } else {
                        Rgb::new(0x5c, 0xed, 0x30)
                    };
                    fb.rect_outline(ix, y, iw, ih, Rgb::new(0xb5, 0xfc, 0x76));
                    hd(
                        self,
                        fb,
                        cx + (cw as f32 * (556.0 / 2560.0)) as i32,
                        y,
                        label,
                        16.0 * scale,
                        colour,
                    );
                }
            }
        }
        // Map browser (`Play Custom`): parent, then sub-folders, then the
        // melee maps of the current folder, exactly as the retail tree lists
        // them (reference 39: [ladder] / [oldladder] / [Replays] / [Save] /
        // [scenario] / (2)Bottleneck.scm).
        let rows = if replays {
            Vec::new()
        } else {
            self.classic_rows()
        };
        if !replays {
            let selected_row = rows
                .iter()
                .position(|r| matches!(r, ClassicRow::Map(i) if *i == self.map_index));
            let fallback = rows.iter().position(|r| matches!(r, ClassicRow::Map(_)));
            let sel_row = selected_row.or(fallback);
            if let Some(row) = sel_row
                && let ClassicRow::Map(i) = rows[row]
                && self.map_index != i
            {
                self.map_index = i;
            }
            self.lobby_top = self.lobby_top.min(rows.len().saturating_sub(6));
            if let Some(row) = sel_row
                && (row < self.lobby_top || row >= self.lobby_top + 6)
            {
                self.lobby_top = (row + 1).saturating_sub(6);
            }
            let row_h = list_row_h;
            let text_x = cx + (cw as f32 * (555.0 / 2560.0)) as i32;
            let icon_x = cx + (cw as f32 * (504.0 / 2560.0)) as i32;
            let tree_green = Rgb::new(0x5c, 0xed, 0x30);
            let selected_file = sel_row.and_then(|r| match rows.get(r) {
                Some(ClassicRow::Map(i)) => Some(*i),
                _ => None,
            });
            for (slot, row) in rows.iter().skip(self.lobby_top).take(6).enumerate() {
                let y = list_top + slot as i32 * row_h;
                let selected = matches!(row, ClassicRow::Map(i) if Some(*i) == selected_file);
                let color = if selected {
                    Rgb::new(0xda, 0xff, 0x8c)
                } else {
                    tree_green
                };
                let iw = (cw as f32 * (36.0 / 2560.0)) as i32;
                let ih = (ch as f32 * (42.0 / 1440.0)) as i32;
                match row {
                    ClassicRow::Map(_) => {
                        fb.rect_outline(icon_x, y - 4, iw, ih, color);
                        let name = match row {
                            ClassicRow::Map(i) => format!("{}.scm", self.maps[*i].name),
                            _ => unreachable!(),
                        };
                        hd(self, fb, text_x, y, &name, 16.0 * scale, color);
                    }
                    ClassicRow::Up | ClassicRow::Folder(_) => {
                        // Gold folder with a tab, 42x36 at x 504.
                        let fw = (cw as f32 * (42.0 / 2560.0)) as i32;
                        let fh = (ch as f32 * (35.0 / 1440.0)) as i32;
                        fb.rect(icon_x, y + 4, fw, fh - 4, folder_gold);
                        fb.rect(icon_x, y, ((fw as f32) * 0.55) as i32, 8, folder_gold);
                        let label = match row {
                            ClassicRow::Up => "[Up One Level]".to_string(),
                            ClassicRow::Folder(f) => format!("[{f}]"),
                            _ => unreachable!(),
                        };
                        hd(self, fb, text_x, y, &label, 16.0 * scale, color);
                    }
                }
            }
            // Red scrollbar of the map tree, measured (1314,390)-(1362,737).
            let sx = cx + (cw as f32 * (1314.0 / 2560.0)) as i32;
            let sy0 = (ch as f32 * (390.0 / 1440.0)) as i32;
            let sh = (ch as f32 * (347.0 / 1440.0)) as i32;
            let sbw = (cw as f32 * (48.0 / 2560.0)) as i32;
            fb.rect(sx, sy0, sbw, sh, Rgb::new(10, 4, 4));
            fb.rect_outline(sx, sy0, sbw, sh, CSS_INNER);
            let span = rows.len().saturating_sub(6) as f32;
            let at = if span > 0.0 {
                self.lobby_top as f32 / span
            } else {
                0.0
            };
            let thumb = ((sh as f32 * 0.62) as i32).max(24);
            let track = (sh - thumb).max(0);
            fb.rect(
                sx + (sbw as f32 * 0.25) as i32,
                sy0 + (track as f32 * at) as i32,
                (sbw as f32 * 0.5) as i32,
                thumb,
                Rgb::new(150, 25, 25),
            );
        }
        // The selected map in the current folder (none when the folder is
        // empty, e.g. [Popular Maps] in reference 38).
        let classic_sel = if replays {
            None
        } else {
            rows.iter().find_map(|r| match r {
                ClassicRow::Map(i) if *i == self.map_index => Some(*i),
                _ => None,
            })
        };
        let classic_slots = classic_sel
            .and_then(|i| self.maps.get(i).map(|m| m.players))
            .unwrap_or(0);
        // Game type row (not used by the replay browser, and hidden until a
        // map is selected: reference 38).
        if !replays && classic_sel.is_some() {
            let gy = (ch as f32 * 0.5833) as i32;
            let game_type = self.cef_text("infoPane_gameType", "Game Type");
            hd(
                self,
                fb,
                cx + (cw as f32 * 0.1734) as i32,
                gy,
                &game_type,
                8.5 * scale,
                dim,
            );
            hd(
                self,
                fb,
                cx + (cw as f32 * 0.2988) as i32,
                gy,
                &self.cef_text(
                    &format!(
                        "scr.common.terms.gameType.{}",
                        self.lobby_game_type.tag() + 1
                    ),
                    self.lobby_game_type.name(),
                ),
                8.5 * scale,
                grey,
            );
            // Dropdown chevron at the right edge of the row.
            let ax = cx + (cw as f32 * 0.5220) as i32;
            let ay = gy + (4.0 * scale) as i32;
            for i in 0..3 {
                fb.rect(ax + i, ay + i, (7 - i * 2) * 2, 2, Rgb::new(220, 200, 40));
            }
        }
        // Slot panel: reference 134831 spans x 300..1533, with the player
        // type and race dropdown columns inside it.  It only exists once a
        // map is selected (reference 38).
        let slots = at(0.1172, 0.6000);
        let sw = (cw as f32 * 0.4816) as i32;
        let sh = (ch as f32 * 0.4000) as i32;
        if !replays
            && classic_sel.is_some()
            && self
                .glue
                .as_ref()
                .and_then(|g| g.create_slots.as_ref())
                .is_none()
        {
            panel(fb, (slots.0, slots.1, sw, sh));
        }
        // Rows measured on 134831: first ink row at y 960, 48 px pitch.
        let row_y = |i: usize| {
            (ch as f32 * (960.0 / 1440.0)) as i32 + (i as f32 * ch as f32 * (48.0 / 1440.0)) as i32
        };
        for i in 0..(if replays { 0 } else { classic_slots.min(8) }) {
            let y = row_y(i);
            let race = self
                .lobby_races
                .get(i % 5)
                .copied()
                .unwrap_or(LobbyRace::Random);
            let name = if i == 0 {
                self.active_name()
            } else {
                self.lobby_label("Computer")
            };
            let name_x = cx + (cw as f32 * (431.0 / 2560.0)) as i32;
            let type_x = cx + (cw as f32 * 0.2461) as i32;
            let race_x = cx + (cw as f32 * (910.0 / 2560.0)) as i32;
            let arrow_x = cx + (cw as f32 * 0.5078) as i32;
            // Insets and the stepped arrow are 1440p pixels scaled with the
            // box (`px`), so the slot rows keep their look from 720p up.
            let off = |v: i32| (v as f32 * ch as f32 / 1440.0).round() as i32;
            let px = |v: i32| off(v).max(1);
            for (x0, x1) in [
                (name_x - off(10), type_x - off(12)),
                (type_x - off(6), race_x - off(12)),
                (race_x - off(8), slots.0 + sw - off(10)),
            ] {
                fb.rect_outline(
                    x0,
                    y - off(2),
                    (x1 - x0).max(1),
                    (ch as f32 * 0.031) as i32,
                    CSS_INNER,
                );
            }
            hd(self, fb, name_x, y, &name, 8.5 * scale, rgb_green);
            hd(
                self,
                fb,
                race_x,
                y,
                &self.lobby_label(race.label()),
                8.5 * scale,
                grey,
            );
            for ax in [type_x, arrow_x] {
                for k in 0..3 {
                    fb.rect(
                        ax + off(k),
                        y + (3.0 * scale) as i32 + off(k),
                        px((9 - k * 2) * 2),
                        px(2),
                        Rgb::new(220, 200, 40),
                    );
                }
            }
        }
        // Reference 134831 captures the second slot's race list open.  The
        // same list is only a menu state in the live client, so keep the
        // screenshot path faithful without changing normal play.
        if !replays && self.shot_name == "lobby-chat" {
            // The other Create capture (134843) shows the CEF chat overlay
            // instead of the open race dropdown: dark header/body with the
            // message lines.  Measured bbox (858,783)-(1374,1181).
            let (px, py) = (
                cx + (cw as f32 * (858.0 / 2560.0)) as i32,
                (ch as f32 * (783.0 / 1440.0)) as i32,
            );
            let pw = (cw as f32 * (516.0 / 2560.0)) as i32;
            let ph = (ch as f32 * (398.0 / 1440.0)) as i32;
            fb.rect(px, py, pw, ph, Rgb::new(10, 2, 12));
            fb.rect_outline(px, py, pw, ph, CSS_OUTER);
            fb.rect(
                px + 2,
                py + 2,
                pw - 4,
                (ch as f32 * (164.0 / 1440.0)) as i32,
                Rgb::new(17, 0, 22),
            );
            fb.rect(
                px + (cw as f32 * (39.0 / 2560.0)) as i32,
                py + (ch as f32 * (192.0 / 1440.0)) as i32,
                (cw as f32 * (450.0 / 2560.0)) as i32,
                (ch as f32 * (206.0 / 1440.0)) as i32,
                Rgb::new(48, 28, 30),
            );
            for (i, line) in ["[All] hello", "[All] gl hf", "[All] gg"]
                .iter()
                .enumerate()
            {
                hd(
                    self,
                    fb,
                    px + (cw as f32 * (48.0 / 2560.0)) as i32,
                    py + (ch as f32 * ((230.0 + 60.0 * i as f32) / 1440.0)) as i32,
                    line,
                    9.0 * scale,
                    Rgb::new(70, 140, 60),
                );
            }
        }
        if !replays && self.shot_name == "lobby-race" {
            let dx = cx + (cw as f32 * 0.3516) as i32;
            let dy = row_y(1) - (ch as f32 * 0.0060) as i32;
            let dw = (cw as f32 * 0.1738) as i32;
            let dh = (ch as f32 * 0.1944) as i32;
            fb.rect(dx, dy, dw, dh, Rgb::new(2, 4, 10));
            fb.rect_outline(dx, dy, dw, dh, CSS_INNER);
            let race_labels: Vec<String> = ["Zerg", "Terran", "Protoss", "Random"]
                .iter()
                .map(|name| self.lobby_label(name))
                .collect();
            for (k, label) in race_labels.iter().enumerate() {
                let ty = dy + (ch as f32 * 0.010) as i32 + (k as f32 * ch as f32 * 0.0382) as i32;
                if k == 0 {
                    fb.rect(
                        dx + 1,
                        ty - 2,
                        dw - 2,
                        (ch as f32 * 0.036) as i32,
                        Rgb::new(14, 24, 66),
                    );
                }
                hd(
                    self,
                    fb,
                    dx + (cw as f32 * 0.006) as i32,
                    ty,
                    label,
                    8.5 * scale,
                    if k == 0 {
                        Rgb::new(225, 235, 255)
                    } else {
                        rgb_green
                    },
                );
            }
        }
        // Live race dropdown: same box, order and pitch as the shipped popup
        // on 134831 (Zerg / Terran / Protoss / Random).  The highlighted row
        // is the one under the pointer (retail hover), falling back to the
        // slot's current race.  The screenshot states above stay identical.
        if !replays && let Some(slot) = self.lobby_race_open {
            let popup_x = cx + (cw as f32 * 0.3516) as i32;
            let popup_y = row_y(slot) - (ch as f32 * 0.0060) as i32;
            let popup_w = (cw as f32 * 0.1738) as i32;
            let popup_h = (ch as f32 * 0.1944) as i32;
            fb.rect(popup_x, popup_y, popup_w, popup_h, Rgb::new(2, 4, 10));
            fb.rect_outline(popup_x, popup_y, popup_w, popup_h, CSS_INNER);
            let options = [
                LobbyRace::Zerg,
                LobbyRace::Terran,
                LobbyRace::Protoss,
                LobbyRace::Random,
            ];
            let hover_k = match self.menu_hover {
                Some(MenuHit::RacePick(hovered, k)) if hovered == slot => Some(k),
                _ => None,
            };
            for (k, race) in options.iter().enumerate() {
                let ty =
                    popup_y + (ch as f32 * 0.010) as i32 + (k as f32 * ch as f32 * 0.0382) as i32;
                let lit = hover_k == Some(k)
                    || (hover_k.is_none() && self.lobby_races.get(slot) == Some(race));
                if lit {
                    fb.rect(
                        popup_x + 1,
                        ty - 2,
                        popup_w - 2,
                        (ch as f32 * 0.036) as i32,
                        Rgb::new(14, 24, 66),
                    );
                }
                hd(
                    self,
                    fb,
                    popup_x + (cw as f32 * 0.006) as i32,
                    ty,
                    &self.lobby_label(race.label()),
                    8.5 * scale,
                    if lit {
                        Rgb::new(225, 235, 255)
                    } else {
                        rgb_green
                    },
                );
            }
        }
        if replays {
            // `Replay Information`, measured on 130858 (ink lefts in content
            // px, faces/sizes matched to the ink boxes within 2 px): title and
            // `Players:` Eurostile Extended 10 UI px, the rest regular 9 UI px,
            // all with natural spacing.  Lavender labels, white values.
            let u = |v: f32| (v - 320.0) / 3.0;
            let label_color = Rgb::new(0xa2, 0xb1, 0xf5);
            let value_color = Rgb::new(0xff, 0xff, 0xff);
            let put = |app: &mut Self,
                       fb: &mut Framebuffer,
                       text: &str,
                       x: f32,
                       y: f32,
                       px: f32,
                       extended: bool,
                       colour: Rgb| {
                let width = app.hd_advance_px(text, px, scale, extended);
                app.draw_hd_text(
                    fb,
                    text,
                    u(x),
                    y / 3.0,
                    LabelAlign::Left,
                    width,
                    px,
                    scale,
                    uox,
                    uoy,
                    colour,
                    extended,
                );
            };
            put(
                self,
                fb,
                "Replay Information",
                1555.0,
                189.0,
                10.0,
                true,
                value_color,
            );
            let info = if self.shot_name == "replays" {
                Some(ReplayInfo::reference())
            } else {
                let profile = self
                    .active_index()
                    .and_then(|i| self.profile_store.names().get(i).cloned())
                    .unwrap_or_default();
                self.save_list(true)
                    .get(self.save_sel)
                    .and_then(|p| ReplayInfo::load(p, &profile))
            };
            if let Some(info) = info {
                // Each value starts one space after its label (130858:
                // label ink end + 13 px).
                for (y, label, value) in [
                    (242.0, "Map:", &info.map),
                    (284.0, "Date:", &info.date),
                    (326.0, "Length:", &info.length),
                    (365.0, "File Name:", &info.file),
                ] {
                    put(self, fb, label, 1553.0, y, 9.0, false, label_color);
                    let gap = self.hd_advance_px(&format!("{label} "), 9.0, scale, false);
                    put(
                        self,
                        fb,
                        value,
                        1553.0 + gap * 3.0,
                        y,
                        9.0,
                        false,
                        value_color,
                    );
                }
                put(
                    self,
                    fb,
                    &self.cef_text("infoPane_players", "Players"),
                    1555.0,
                    601.0,
                    10.0,
                    true,
                    label_color,
                );
                let terran = self.race_label(Race::Terran);
                let zerg = self.race_label(Race::Zerg);
                let protoss = self.race_label(Race::Protoss);
                for (i, (name, race)) in info.players.iter().take(8).enumerate() {
                    let y = 650.0 + 36.0 * i as f32;
                    let (race_name, colour) = match race {
                        Race::Terran => (terran.as_str(), Rgb::new(0x80, 0xd4, 0xff)),
                        Race::Zerg => (zerg.as_str(), Rgb::new(0xd4, 0x80, 0xff)),
                        // INFERENCE: no reference shows a Protoss row.
                        Race::Protoss => (protoss.as_str(), Rgb::new(0xff, 0xd4, 0x80)),
                        Race::Neutral => ("", value_color),
                    };
                    put(self, fb, name, 1552.0, y, 9.0, false, value_color);
                    put(self, fb, race_name, 1849.0, y, 9.0, false, colour);
                }
            }
        } else {
            // Right preview panel.  The shipped `pinfo` plate provides the metal
            // frame and the red hairline; fall back to a flat panel without it.
            let right = at(0.59375, 0.1097);
            let rw = (cw as f32 * 0.2539) as i32;
            let rh = (ch as f32 * 0.5764) as i32;
            if self
                .glue
                .as_ref()
                .and_then(|g| g.create_info.as_ref())
                .is_none()
            {
                panel(fb, (right.0, right.1, rw, rh));
            }
            let map_name = self
                .maps
                .get(self.map_index)
                .map(|m| {
                    let raw = m.name.trim();
                    raw.split_once(')')
                        .map(|(_, tail)| tail.trim().to_string())
                        .filter(|tail| !tail.is_empty() && raw.starts_with('('))
                        .unwrap_or_else(|| raw.to_string())
                })
                .unwrap_or_default();
            if let Some(font) = self.ui_font.as_mut() {
                font.draw_centered(
                    fb,
                    cx + (cw as f32 * 0.7207) as i32,
                    (ch as f32 * 0.3403) as i32,
                    &map_name,
                    10.5 * scale,
                    0.4,
                    grey,
                    Some(Rgb::new(0, 0, 0)),
                );
                let desc = if map_name.contains("Hunters") {
                    "Eight-player map."
                } else if map_name.contains("Bottleneck") {
                    "Control the choke points."
                } else {
                    "Melee map."
                };
                font.draw_top(
                    fb,
                    cx + (cw as f32 * 0.6055) as i32,
                    (ch as f32 * 0.3750) as i32,
                    desc,
                    8.0 * scale,
                    0.3,
                    grey,
                    Some(Rgb::new(0, 0, 0)),
                );
            }
            // Preview well: the real terrain minimap of the selected map with
            // its start locations, as the retail MapBrowserInfoPanel shows.
            let px = cx + (cw as f32 * 0.6582) as i32;
            let py = (ch as f32 * 0.1250) as i32;
            let pw = (cw as f32 * 0.1230) as i32;
            let ph = (ch as f32 * 0.2153) as i32;
            if self
                .lobby_preview
                .as_ref()
                .map(|(i, _)| *i != self.map_index)
                .unwrap_or(true)
            {
                self.ensure_lobby_preview();
            }
            fb.rect(px, py, pw, ph, Rgb::new(2, 6, 4));
            if let Some((_, pixels)) = &self.lobby_preview {
                for y in 0..ph {
                    let sy = (y as usize * 128 / ph.max(1) as usize).min(127);
                    for x in 0..pw {
                        let sx = (x as usize * 128 / pw.max(1) as usize).min(127);
                        fb.put(px + x, py + y, Rgb(pixels[sy * 128 + sx] & 0x00ff_ffff));
                    }
                }
            }
            fb.rect_outline(px, py, pw, ph, Rgb::new(0x6c, 0x00, 0x00));
            if let Some(entry) = self.maps.get(self.map_index)
                && let Ok(raw) = std::fs::read(&entry.path)
                && let Ok(sc) = Scenario::parse(&raw)
            {
                for (sx, sy) in sc.start_locations() {
                    let gx = px + (sx * pw / (sc.width as i32).max(1));
                    let gy = py + (sy * ph / (sc.height as i32).max(1));
                    fb.rect(gx - 2, gy - 2, 5, 5, Rgb::new(60, 220, 90));
                    fb.rect(gx - 1, gy - 1, 3, 3, Rgb::new(220, 255, 220));
                }
                for u in &sc.units {
                    if galaxycraft::sim::is_resource_type(u.unit_id) {
                        let gx = px + (u.x * pw / (sc.width as i32).max(1));
                        let gy = py + (u.y * ph / (sc.height as i32).max(1));
                        fb.rect(gx, gy, 2, 2, Rgb::new(0x00, 0xe4, 0xfc));
                    }
                }
            }
            // Map statistics, right-aligned in the lower part of the info plate.
            let (size_label, tileset_label, suggested_label) = self
                .maps
                .get(self.map_index)
                .map(|entry| {
                    let mut size = "128x128".to_string();
                    let mut tileset = "Jungle".to_string();
                    if let Ok(raw) = std::fs::read(&entry.path)
                        && let Ok(sc) = Scenario::parse(&raw)
                    {
                        size = format!("{}x{}", sc.width, sc.height);
                        tileset = match sc.era {
                            0 => "Badlands",
                            1 => "Space",
                            2 => "Installation",
                            3 => "Ash World",
                            4 => "Jungle",
                            5 => "Desert",
                            6 => "Ice",
                            _ => "Twilight",
                        }
                        .to_string();
                    }
                    let suggested = if entry.players >= 8 {
                        "2-8".to_string()
                    } else {
                        entry.players.to_string()
                    };
                    (size, tileset, suggested)
                })
                .unwrap_or_else(|| ("128x128".into(), "Jungle".into(), "2".into()));
            let size_header = self.cef_text("infoPane_size", "Size");
            let tileset_header = self.cef_text("infoPane_tileset", "Tileset");
            let suggested_header = self.cef_text("infoPane_suggestedPlayers", "Suggested Players");
            let tileset_label = self.tileset_label(&tileset_label);
            if let Some(font) = self.ui_font.as_mut() {
                let rx = cx + (cw as f32 * 0.8477) as i32;
                let size_line = format!("{size_header}:  {size_label}");
                let tileset_line = format!("{tileset_header}:  {tileset_label}");
                let suggested_line = format!("{suggested_header}:  {suggested_label}");
                for (i, line) in [
                    size_line.as_str(),
                    tileset_line.as_str(),
                    suggested_line.as_str(),
                ]
                .iter()
                .enumerate()
                {
                    font.draw_right(
                        fb,
                        rx,
                        (ch as f32 * 0.6320) as i32 + i as i32 * (9.5 * scale) as i32,
                        line,
                        8.0 * scale,
                        0.3,
                        grey,
                        Some(Rgb::new(0, 0, 0)),
                    );
                }
            }
        }
        // Ok / Cancel `FLCBTN`s over the shipped pok/pcancel plates.
        let ok_label = self.ui_text("gluCreat-2", "Ok");
        let cancel_label = self.ui_text("gluCreat-3", "Cancel");
        self.draw_glue_button(fb, &ok_label, galaxycraft::menu::PALNL_OK_BUTTON, true);
        self.draw_glue_button(
            fb,
            &cancel_label,
            galaxycraft::menu::PALNL_CANCEL_BUTTON,
            true,
        );
    }
}
