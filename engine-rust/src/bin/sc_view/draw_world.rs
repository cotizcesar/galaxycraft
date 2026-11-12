//! `impl App`: draw world (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    /// Input readout: pointer position, button states and the last key. Shown
    /// with `--debug-input` so a player can confirm the client is receiving
    /// events on their session.
    pub(super) fn draw_input_debug(&self, fb: &mut Framebuffer) {
        let (cx, _cy, _cw, _ch) = Self::content_rect(fb);
        let lines = [
            format!("pointer: {:?}", self.pointer),
            format!("LMB: {}   RMB: {}", self.mouse_was_down, self.rmb_down),
            format!("last key: {}", self.last_key.clone().unwrap_or_default()),
            format!(
                "mode: {}  overlay: {}",
                if self.menu {
                    "menu"
                } else if self.briefing {
                    "briefing"
                } else if self.paused {
                    "paused"
                } else {
                    "game"
                },
                self.overlay
            ),
            format!(
                "hover: {}  fb: {}x{}",
                self.profile_sel, fb.width, fb.height
            ),
        ];
        let w = 340;
        let h = 12 + lines.len() as i32 * 12;
        let (x, y) = (cx + 12, 12);
        for yy in y..y + h {
            for xx in x..x + w {
                let c = fb.get(xx, yy);
                fb.put(xx, yy, c.blend(Rgb::new(0, 0, 0), 200));
            }
        }
        fb.rect_outline(x, y, w, h, Rgb::new(90, 200, 120));
        for (i, line) in lines.iter().enumerate() {
            fb.text(
                x + 6,
                y + 6 + i as i32 * 12,
                line,
                Rgb::new(150, 255, 170),
                1,
            );
        }
    }

    /// Software pointer: the compositor does not always show the system cursor
    /// for a borderless window, so the client paints its own arrow.
    /// Hidden `ShowFPS` setting: the retail executable stores `ShowFPS` in
    /// its option map.  The overlay is debug chrome, so it is not in the
    /// Options tabs.
    pub(super) fn draw_fps(&mut self, fb: &mut Framebuffer) {
        if !self.game_options.show_fps {
            return;
        }
        let text = fps_label(self.fps);
        self.glue_text_face(
            fb,
            20.0,
            40.0,
            &text,
            HUD_SMALL_PX,
            Rgb::new(255, 255, 0),
            None,
            GlueFace::Hud,
        );
    }

    /// Hidden `ShowTurnRate` setting registered by the retail config routine
    /// at `0x006e2653`; shows the selected flingy's `flingy.dat` turn rate.
    pub(super) fn draw_turn_rate(&mut self, fb: &mut Framebuffer) {
        if !self.game_options.show_turn_rate {
            return;
        }
        let rate = self
            .selected
            .first()
            .and_then(|id| self.world.unit(*id))
            .and_then(|u| {
                let unit = self.data.unit(u.type_id);
                self.data.flingy(unit.flingy).map(|f| f.turn_rate)
            });
        let text = turn_rate_label(rate);
        self.glue_text_face(
            fb,
            20.0,
            if self.game_options.show_fps {
                60.0
            } else {
                40.0
            },
            &text,
            HUD_SMALL_PX,
            Rgb::new(255, 255, 0),
            None,
            GlueFace::Hud,
        );
    }

    pub(super) fn draw_cursor(&self, fb: &mut Framebuffer) {
        // The command wheel is modal: no pointer while it is open.
        if self.pad_wheel.is_open() {
            return;
        }
        let target = self.pointer.and_then(|(mx, my)| {
            // Only the live playfield transforms the pointer; menus, briefing
            // screens, dialogs and the command console keep the arrow.
            if self.menu
                || self.intro
                || self.briefing
                || self.overlay != 0
                || self.game_dialog.is_some()
                || my >= self.playfield_bottom()
            {
                return None;
            }
            let u = self.world.unit(self.unit_at(mx, my)?)?;
            Some(if u.owner as usize == self.local_player {
                CursorTarget::Own
            } else if self.world.hostile(self.local_player as u8, u.owner) {
                CursorTarget::Enemy
            } else {
                CursorTarget::Neutral
            })
        });
        draw_virtual_cursor(
            fb,
            self.pointer,
            self.cursor_art.as_ref(),
            target,
            self.cursor_started.elapsed(),
        );
    }

    /// Facing byte the renderer should draw.
    /// Turret of a Siege Tank or Goliath (`units.dat` subunit): its image
    /// and heading (`SimUnit::turret_facing`).  Drawn on top of the base at
    /// the same position.
    pub(super) fn turret_pose(&self, u: &galaxycraft::sim::SimUnit) -> Option<(u16, u8)> {
        let ty = self.data.unit(u.type_id);
        self.data.turret(ty)?;
        let image = galaxycraft::unit_anim::unit_image(&self.data, ty.turret)?;
        // The simulation turns the turret at its own rate.
        Some((image, u.turret_facing))
    }

    /// Frame of the sieged tank's base (iscript 92): frames 0..=5 unfold
    /// over the deploy animation (five frames of 5, then it holds 5) and
    /// fold back at the end of the pack-up one (38 frames of hold first).
    /// Frame of the sieged cannon (iscript 93) while the tank deploys or
    /// packs up: it waits folded (17), then unfolds 34 / 51 / 68 / 85 three
    /// frames apart and rests at 0; packing up runs 85 down to 17.  `None`
    /// outside those animations.
    pub(super) fn siege_turret_frame(&self, u: &galaxycraft::sim::SimUnit) -> Option<u16> {
        use galaxycraft::sim::{SIEGE_FRAMES, UNSIEGE_FRAMES};
        if u.type_id != galaxycraft::ids::unit::SIEGE_MODE || u.order_wait == 0 {
            return None;
        }
        let wait = u.order_wait as i32;
        Some(match u.order {
            Order::Siege => {
                let e = SIEGE_FRAMES as i32 - (wait - 1);
                match e {
                    e if e < 42 => 17,
                    e if e < 54 => (34 + 17 * ((e - 42) / 3)) as u16,
                    _ => 0,
                }
            }
            Order::Unsiege => {
                let e = UNSIEGE_FRAMES as i32 - (wait - 1);
                (85 - 17 * (e / 3)).max(17) as u16
            }
            _ => return None,
        })
    }

    pub(super) fn siege_base_frame(&self, u: &galaxycraft::sim::SimUnit) -> usize {
        use galaxycraft::sim::{SIEGE_FRAMES, UNSIEGE_FRAMES};
        let wait = u.order_wait as i32;
        match u.order {
            Order::Siege if wait > 0 => {
                let elapsed = SIEGE_FRAMES as i32 - (wait - 1);
                ((elapsed - 1) / 5).clamp(0, 5) as usize
            }
            Order::Unsiege if wait > 0 => {
                let elapsed = UNSIEGE_FRAMES as i32 - (wait - 1);
                if elapsed < 38 {
                    5
                } else {
                    (4 - (elapsed - 38) / 5).clamp(0, 5) as usize
                }
            }
            _ => 5,
        }
    }

    pub(super) fn rendered_facing(&self, u: &galaxycraft::sim::SimUnit) -> u8 {
        self.smooth_facing
            .get(&u.id)
            .map_or(u.facing, |f| (f.round() as i32).rem_euclid(256) as u8)
    }

    /// Draw the current frame.  Every state (movies, menus, briefings,
    /// the game and its dialogs) lives in the 16:9 content box: the retail
    /// client is black outside it on all 56 reference captures.
    pub(super) fn draw(&mut self, fb: &mut Framebuffer) {
        self.draw_frame(fb);
        self.draw_fps(fb);
        self.draw_turn_rate(fb);
        if self.end_dialog
            && self.end_dialog_hold == 0
            && self.game_result.is_some()
            && !self.menu
            && !self.intro
        {
            self.draw_end_dialog(fb);
        }
        let (cx, _, cw, _) = Self::content_rect(fb);
        mask_pillarbox(fb, cx, cw);
    }

    pub(super) fn draw_frame(&mut self, fb: &mut Framebuffer) {
        self.update_view(fb);

        if self.intro {
            self.draw_intro(fb);
            return;
        }
        if self.menu {
            self.draw_menu(fb);
            return;
        }
        if self.briefing {
            draw_in_box(self, fb, Self::draw_briefing);
            return;
        }
        self.update_smooth_facings();
        self.tileset
            .update_color_cycle(self.world.tick, self.game_options.game_color_cycling);
        fb.clear(Rgb::new(0, 0, 0));
        // Walking / attack poses, bullets and explosions follow the ticks
        // the world ran since the last frame.
        {
            // Building fire / blood positions come from each image's damage
            // LO file in the install (`unit\\<race>\\<name>.lof`).
            let _ = self.image_table();
            let install = self.install.as_ref();
            let data = self.data.clone();
            let names = self.image_names.as_deref().unwrap_or(&[]);
            let mut lo = |image: u16| -> Vec<(u8, i8, i8)> {
                let Some(install) = install else {
                    return Vec::new();
                };
                let index = data.image(image).map_or(0, |i| i.damage_filename_index) as usize;
                if index == 0 {
                    return Vec::new();
                }
                names
                    .get(index - 1)
                    .and_then(|n| install.read(&format!("unit/{}", n.replace('\\', "/"))))
                    .and_then(|b| galaxycraft::unit_anim::parse_lo_slots(&b))
                    .unwrap_or_default()
            };
            self.animator.advance_with(&self.world, &mut lo);
        }
        self.play_world_sounds();
        let fb_full = fb;
        // The world is rendered into a logical 1x framebuffer and then blitted
        // scaled, so sprites keep their proportion against the HUD at any
        // resolution (720p..4K) and ultrawide only widens the view.
        // HD path (retail at >= 2x): the terrain is drawn at screen
        // resolution from the 4x art over the whole content height (the
        // console has see-through gaps), the 1x layer only carries creep and
        // units, and the fog is applied at screen resolution afterwards.
        let scale = self.ui_scale.max(0.5);
        let hd =
            self.game_options.video_hd && self.hd_terrain.is_some() && scale >= HD_WORLD_MIN_SCALE;
        let (vw, vh) = self.view;
        let vh = if hd {
            (Self::content_rect(fb_full).3 as f32 / scale).ceil() as i32
        } else {
            vh
        };
        let mut world_fb = Framebuffer::new(vw.max(1) as usize, vh.max(1) as usize);
        world_fb.clear(if hd { WORLD_KEY } else { Rgb::new(0, 0, 0) });
        let fb = &mut world_fb;
        let cam = self.camera;
        if !hd {
            self.tileset.draw_terrain(fb, &self.scenario, cam);
        }
        // Creep sits on the terrain and under the fog (retail tiles and
        // edge frames; the HD path draws it with the terrain below).
        if !hd {
            let creep = self.creep_lookup();
            self.tileset.draw_creep(fb, &self.scenario, cam, &creep);
        }
        if self.fog_on() && !hd {
            self.tileset.draw_fog(
                fb,
                &self.scenario,
                cam,
                &self.world,
                self.local_player as u8,
            );
        }

        let mut order: Vec<usize> = (0..self.world.units.len())
            .filter(|&i| self.world.units[i].alive && !self.world.inside_gas_building(i))
            .collect(); // Elevation first, then y (retail sprite order); an SCV welding a
        // building is drawn in front of it.
        order.sort_by_key(|&i| {
            let u = &self.world.units[i];
            let y = match u.order {
                Order::Construct { building } => {
                    self.world.unit(building).map_or(u.y, |b| u.y.max(b.y + 1))
                }
                _ => u.y,
            };
            (self.data.unit(u.type_id).elevation, y)
        });
        // SD: a Terran building under construction shows its scaffold
        // (`units.dat` construction image) and then its AlmostBuilt frame,
        // at the same progress marks as the HD path.
        let mut building_stage: std::collections::HashMap<
            usize,
            (std::rc::Rc<galaxycraft::render::Grp>, usize),
        > = std::collections::HashMap::new();
        // SD turrets: (turret GRP, frame, mirrored) per unit index.
        let mut sd_turrets: std::collections::HashMap<
            usize,
            (std::rc::Rc<galaxycraft::render::Grp>, usize, bool),
        > = std::collections::HashMap::new();
        if !hd {
            for &i in &order {
                // The sieged tank's base is its own image, unfolding.
                if self.world.units[i].type_id == galaxycraft::ids::unit::SIEGE_MODE {
                    let frame = self.siege_base_frame(&self.world.units[i]);
                    if let Some(image) = galaxycraft::unit_anim::unit_image(
                        &self.data,
                        galaxycraft::ids::unit::SIEGE_MODE,
                    ) && let Some(grp) = self.sd_image_grp(image)
                        && grp.frame(frame).is_some()
                    {
                        building_stage.insert(i, (grp, frame));
                    }
                }
                if let Some((timage, tfacing)) = self.turret_pose(&self.world.units[i])
                    && let Some(grp) = self.sd_image_grp(timage)
                {
                    let raw = self
                        .siege_turret_frame(&self.world.units[i])
                        .or_else(|| self.animator.turret_frame(self.world.units[i].id))
                        .unwrap_or(0);
                    let frame = galaxycraft::render::sd_frame_index(
                        grp.frames.len(),
                        tfacing,
                        Some(raw),
                        false,
                        0,
                    );
                    let flip = galaxycraft::render::sd_frame_flip(grp.frames.len(), tfacing);
                    sd_turrets.insert(i, (grp, frame, flip));
                }
                if building_stage.contains_key(&i) {
                    continue;
                }
                let (type_id, stage) = {
                    let u = &self.world.units[i];
                    (
                        u.type_id,
                        terran_construction_stage(u, self.data.unit(u.type_id)),
                    )
                };
                let pick = match stage {
                    Some(ConstructionStage::Scaffold(n))
                        if self.data.unit(type_id).construction_image == 0 =>
                    {
                        galaxycraft::unit_anim::unit_image(&self.data, type_id).map(|img| {
                            (
                                img,
                                self.first_frame_of(img, CONSTRUCTION_STAGE_ANIMS[n.min(2)])
                                    .unwrap_or(0),
                            )
                        })
                    }
                    Some(ConstructionStage::Scaffold(n)) => {
                        Some((self.data.unit(type_id).construction_image, n))
                    }
                    Some(ConstructionStage::AlmostBuilt) => {
                        galaxycraft::unit_anim::unit_image(&self.data, type_id).map(|img| {
                            (
                                img,
                                self.first_frame_of(img, ALMOST_BUILT_ANIM).unwrap_or(0),
                            )
                        })
                    }
                    None => None,
                };
                if let Some((image, frame)) = pick
                    && let Some(grp) = self.sd_image_grp(image)
                    && grp.frame(frame).is_some()
                {
                    building_stage.insert(i, (grp, frame));
                } else if pick.is_none()
                    && !self.sprites.has_unit(type_id)
                    && let Some(image) = galaxycraft::unit_anim::unit_image(&self.data, type_id)
                    && let Some(grp) = self.sd_image_grp(image)
                {
                    // A unit the bundled SD sheets do not cover (the
                    // nuclear missile): its GRP straight from the install.
                    let (id, facing) = {
                        let u = &self.world.units[i];
                        (u.id, u.facing)
                    };
                    let frame = galaxycraft::render::sd_frame_index(
                        grp.frames.len(),
                        facing,
                        self.animator.unit_frame(id),
                        false,
                        0,
                    );
                    if grp.frame(frame).is_some() {
                        building_stage.insert(i, (grp, frame));
                    }
                }
            }
        }
        let mut hd_units = Vec::new();
        for i in order {
            let u = &self.world.units[i];
            // Enemy units the fog hides; cloaked or burrowed ones without a
            // detector are hidden even with the fog switched off.
            if u.owner as usize != self.local_player
                && (self.fog_on() || u.status.cloaked || u.status.burrowed)
                && !self.world.can_see_unit(self.local_player as u8, i)
            {
                continue;
            }
            if hd && self.hd_unit_image(u.type_id).is_some() {
                hd_units.push(i);
                continue;
            }
            let (sx, sy) = self.screen_of(u);
            if sx < -160 || sy < -160 || sx > fb.width + 160 || sy > self.view.1 + 160 {
                continue;
            }
            let selected = self.selected.contains(&u.id);
            let pose = (!self.data.unit(u.type_id).is_building())
                .then(|| self.animator.unit_frame(u.id))
                .flatten();
            let facing = self.rendered_facing(u);
            // Status images behind the unit (Defensive Matrix back shell).
            for (image, frame) in self.animator.status_underlays(u.id) {
                let remap = self.data.image(image).map_or(0, |im| im.color_shift);
                if let (Some(grp), Some(table)) =
                    (self.sd_image_grp(image), self.remap_table(remap))
                {
                    galaxycraft::render::draw_grp_remapped(
                        fb,
                        &grp,
                        frame as usize,
                        (sx, sy),
                        false,
                        &table,
                    );
                }
            }
            let u = &self.world.units[i];
            // A cloaked unit we can see is drawn faded.
            let cloak = u
                .status
                .cloaked
                .then(|| fb.save_rect(sx - 48, sy - 48, 96, 96));
            if let Some((grp, frame)) = building_stage.get(&i) {
                galaxycraft::render::draw_grp_posed(
                    fb,
                    grp,
                    *frame,
                    self.sprites.palette(),
                    (sx, sy),
                    false,
                    galaxycraft::render::GrpInk::Solid(Some(galaxycraft::render::player_rgb(
                        &self.world,
                        u.owner,
                    ))),
                );
            } else if !self.sprites.draw_unit_turned(
                fb,
                u,
                // Idle hover bob of flyers (`setvertpos`).
                (sx, sy + self.animator.unit_vert(u.id)),
                self.world.tick,
                pose,
                facing,
            ) {
                draw_unit_box(fb, u, &self.data, (sx, sy), selected);
            } else if selected {
                let ty = self.data.unit(u.type_id);
                let r = ((ty.dimensions.right - ty.dimensions.left) as i32 / 2).max(12) + 4;
                fb.circle_outline(sx, sy, r, Rgb::new(80, 255, 80));
            }
            if let Some((grp, frame, flip)) = sd_turrets.get(&i) {
                galaxycraft::render::draw_grp_posed(
                    fb,
                    grp,
                    *frame,
                    self.sprites.palette(),
                    (sx, sy),
                    *flip,
                    galaxycraft::render::GrpInk::Solid(Some(galaxycraft::render::player_rgb(
                        &self.world,
                        u.owner,
                    ))),
                );
            }
            if let Some(saved) = &cloak {
                fb.fade_rect(saved, galaxycraft::render::CLOAK_KEEP);
            }
            // Neutral resources are stored under player 0: no bar on them
            // unless selected.
            let neutral = u.is_resource() || u.type_id == galaxycraft::ids::unit::VESPENE_GEYSER;
            if selected || (u.owner as usize == self.local_player && !neutral) {
                self.sprites.draw_health(fb, u, &self.data, (sx, sy));
            }
        }

        let ctf_clip = (0, 0, world_fb.width, self.view.1);
        self.draw_ctf_flags(&mut world_fb, ctf_clip, 1.0, false);

        if !hd {
            self.draw_animated_images_sd(&mut world_fb);
        }

        // Blit the logical frame scaled to the window, then draw the HUD in
        // screen space on top.
        let prof = std::env::var_os("GC_PROFILE").is_some();
        let mut marks: Vec<(&str, Instant)> = vec![("world1x", Instant::now())];
        if hd {
            let (cx, cy, cw, ch) = Self::content_rect(fb_full);
            let creep = self.creep_lookup();
            if let Some(t) = &self.hd_terrain {
                t.draw(
                    fb_full,
                    &self.tileset,
                    &self.scenario,
                    cam,
                    (cx, cy, cw, ch),
                    scale,
                    &creep,
                    self.game_options
                        .game_color_cycling
                        .then_some(self.world.tick / 3),
                );
            }
            marks.push(("terrain", Instant::now()));
            blit_scaled_keyed(&world_fb, fb_full, scale, self.content_x0);
            marks.push(("blit", Instant::now()));
            let env = if self.game_options.video_lighting {
                self.hd_light_env(&hd_units, (cx, cy, cw, ch), scale)
            } else {
                galaxycraft::hd_lights::LightEnv::new(
                    (cx as f32, cy as f32, cw as f32, ch as f32),
                    scale,
                )
            };
            marks.push(("lights", Instant::now()));
            for &i in &hd_units {
                // A cloaked unit we can see is drawn faded.
                let u = &self.world.units[i];
                let cloak = u.status.cloaked.then(|| {
                    let sx = cx + ((u.x / 256 - self.camera.0) as f32 * scale) as i32;
                    let sy = cy + ((u.y / 256 - self.camera.1) as f32 * scale) as i32;
                    let half = (56.0 * scale) as i32;
                    fb_full.save_rect(sx - half, sy - half, half * 2, half * 2)
                });
                self.draw_unit_hd(fb_full, i, (cx, cy, cw, ch), scale, &env);
                if let Some(saved) = &cloak {
                    fb_full.fade_rect(saved, galaxycraft::render::CLOAK_KEEP);
                }
            }
            self.draw_ctf_flags(fb_full, (cx, cy, cw, ch), scale, true);
            self.draw_animated_images(fb_full, (cx, cy, cw, ch), scale, &env);
            self.draw_order_marker(fb_full, (cx, cy, cw, ch), scale);
            self.draw_placement_preview(fb_full, (cx, cy, cw, ch), scale);
            marks.push(("units", Instant::now()));
            if prof {
                let t = Instant::now();
                let n = hd_units
                    .iter()
                    .filter(|&&i| self.hd_unit_parts(i, (cx, cy, cw, ch), scale).is_some())
                    .count();
                eprintln!(
                    "hd units {} (with parts {n}), parts cost {:.1} ms",
                    hd_units.len(),
                    t.elapsed().as_secs_f64() * 1000.0
                );
            }
            if self.fog_on() {
                galaxycraft::render::draw_fog_smooth(
                    fb_full,
                    &self.scenario,
                    &self.world,
                    self.local_player as u8,
                    cam,
                    (cx, cy, cw, ch),
                    scale,
                );
            }
        } else {
            blit_scaled_at(
                &world_fb,
                fb_full,
                scale,
                self.content_x0,
                self.game_options.video_sd_filter,
            );
        }
        // The view is `vw` game px wide; retail scales it to fill the box.
        let content = Self::content_rect(fb_full);
        let drawn = (vw as f32 * scale).round() as i32;
        marks.push(("fog", Instant::now()));
        galaxycraft::render::stretch_columns(fb_full, content, drawn);
        if hd && self.game_options.video_environment {
            self.draw_environment_effects(fb_full, content);
        }
        marks.push(("stretch", Instant::now()));
        // The selection box is in window pixels (where the drag started and
        // where the pointer is), so it goes on the screen frame after the
        // world: on the 1x layer it was scaled away and, in HD, covered.
        if let Some(rect) = drag_box(self.drag_start, self.drag_now) {
            let t = (scale / 2.0).round().max(1.0) as i32;
            for k in 0..t {
                fb_full.rect_outline(
                    rect.0 + k,
                    rect.1 + k,
                    (rect.2 - 2 * k).max(1),
                    (rect.3 - 2 * k).max(1),
                    Rgb::new(16, 252, 24),
                );
            }
        }
        let fb = fb_full;
        self.draw_hud(fb);
        marks.push(("hud", Instant::now()));
        self.draw_overlay(fb);
        marks.push(("overlay", Instant::now()));
        self.draw_pad_prompts(fb);
        if prof {
            let parts: Vec<String> = marks
                .windows(2)
                .map(|w| format!("{} {:.1}", w[1].0, (w[1].1 - w[0].1).as_secs_f64() * 1000.0))
                .collect();
            let (baked, lit_px) = galaxycraft::hd_sprites::take_draw_stats();
            eprintln!(
                "game draw: {} | sprites unlit {baked}, lit {lit_px}",
                parts.join(" | ")
            );
        }
    }

    /// MTXM entry each creep tile draws with (recorded capture variants,
    /// else the session picker), `None` off creep: shared by the playfield
    /// and the terrain minimap.
    pub(super) fn creep_lookup(&mut self) -> impl Fn(i32, i32) -> Option<u16> + 'static + use<> {
        let width = self.scenario.width;
        self.creep_tiles
            .update(&mut self.crt_rand, self.world.creep_grid(), width);
        let tiles = self.creep_tiles.clone();
        let variants = self.creep_variants.clone();
        move |tx: i32, ty: i32| {
            let picked = tiles.get(tx, ty)?;
            Some(match variants.get(&(tx, ty)) {
                Some(&v) => (1 << 4) | v as u16,
                None => picked,
            })
        }
    }

    /// HD image of a unit's main sprite (`unit -> flingy -> sprite -> image`).
    pub(super) fn hd_unit_image(
        &self,
        type_id: u16,
    ) -> Option<std::rc::Rc<galaxycraft::hd_sprites::HdImage>> {
        let ty = self.data.unit(type_id);
        let flingy = self.data.flingy(ty.flingy)?;
        let image = self.data.sprite(flingy.sprite)?.image;
        let install = self.install.as_ref();
        let skin = unit_skin_id(type_id, &self.game_options);
        self.hd_sprites
            .image_skin(self.data.hd_anim_id(image), skin, |path| {
                install?.read(path)
            })
    }

    /// Building placement preview (Boot Camp recording 169-170 s): the
    /// building's image half blended at the grid-snapped spot under the
    /// pointer, and its placement box tinted green (x0.62, x1.6, x0.62
    /// over the ground, measured) or red where it cannot be built
    /// (INFERENCE: the mirrored tint).
    pub(super) fn draw_placement_preview(
        &mut self,
        fb: &mut Framebuffer,
        content: (i32, i32, i32, i32),
        s: f32,
    ) {
        // Building placement, or a lifted building's Land target.
        let landing = match self.target_act {
            Some((galaxycraft::buttons::Act::Land, _)) => self
                .selected
                .iter()
                .filter_map(|id| self.world.unit(*id))
                .find(|u| u.lifted)
                .map(|u| (u.type_id, u.id)),
            _ => None,
        };
        let Some(t) = self.build_mode.or(landing.map(|l| l.0)) else {
            return;
        };
        let (mx, my) = self.drag_now;
        if my >= self.playfield_bottom() || mx < content.0 || mx >= content.0 + content.2 {
            return;
        }
        let (lx, ly) = self.to_logical(mx, my);
        let cursor = self.world_of(lx, ly);
        let (centre, (pw, ph), valid) = self.world.placement_preview(t, cursor);
        // Placing a Protoss building that needs power: every own Pylon's
        // psi field shows in blue (the area `World::has_power_px` uses).
        let local = self.local_player as u8;
        if self.world.players.get(local as usize).map(|p| p.race) == Some(Race::Protoss)
            && self.data.unit(t).is_building()
            && !matches!(
                t,
                galaxycraft::ids::unit::NEXUS
                    | galaxycraft::ids::unit::PYLON
                    | galaxycraft::ids::unit::ASSIMILATOR
            )
        {
            let span = |k: i32| {
                let lo = match k.cmp(&0) {
                    std::cmp::Ordering::Greater => 32 * k,
                    std::cmp::Ordering::Equal => -31,
                    std::cmp::Ordering::Less => 32 * k - 31,
                };
                let hi = if k >= 0 { 32 * k + 31 } else { 32 * k };
                (lo, hi)
            };
            let pylons: Vec<(i32, i32)> = self
                .world
                .units
                .iter()
                .filter(|u| {
                    u.alive
                        && u.completed
                        && u.owner == local
                        && u.type_id == galaxycraft::ids::unit::PYLON
                })
                .map(|u| (u.x / 256, u.y / 256))
                .collect();
            let (cx0, cy0, cw, ch) = content;
            for (px, py) in pylons {
                for (row, cols) in galaxycraft::sim::PSI_FIELD.iter().enumerate() {
                    let (y0, y1) = span(row as i32 - 5);
                    for (col, on) in cols.iter().enumerate() {
                        if !on {
                            continue;
                        }
                        let (x0, x1) = span(col as i32 - 8);
                        let sx0 = cx0 as f32 + (px + x0 - self.camera.0) as f32 * s;
                        let sy0 = cy0 as f32 + (py + y0 - self.camera.1) as f32 * s;
                        let sx1 = cx0 as f32 + (px + x1 + 1 - self.camera.0) as f32 * s;
                        let sy1 = cy0 as f32 + (py + y1 + 1 - self.camera.1) as f32 * s;
                        let (ix0, iy0) = ((sx0 as i32).max(cx0), (sy0 as i32).max(cy0));
                        let (ix1, iy1) = ((sx1 as i32).min(cx0 + cw), (sy1 as i32).min(cy0 + ch));
                        for yy in iy0..iy1 {
                            for xx in ix0..ix1 {
                                let c = fb.get(xx, yy);
                                fb.put(xx, yy, c.blend(Rgb::new(40, 110, 255), 60));
                            }
                        }
                    }
                }
            }
        }
        // Per-tile green / red (gas buildings keep one colour: they need a
        // geyser, not free tiles).
        let tiles = (!galaxycraft::sim::is_gas_building(t)).then(|| {
            self.world
                .placement_tiles(t, cursor, landing.map(|l| l.1))
                .2
        });
        let (tw, th) = ((pw / 32).max(1), (ph / 32).max(1));
        let (cx, cy, cw, ch) = content;
        let sx = cx as f32 + (centre.0 - self.camera.0) as f32 * s;
        let sy = cy as f32 + (centre.1 - self.camera.1) as f32 * s;
        let x0 = (sx - pw as f32 * s / 2.0).round() as i32;
        let y0 = (sy - ph as f32 * s / 2.0).round() as i32;
        let (w, h) = (
            (pw as f32 * s).round() as i32,
            (ph as f32 * s).round() as i32,
        );
        // Ghost: draw the building, then average with what was under it.
        let ty = self.data.unit(t).clone();
        let install = self.install.as_ref();
        let image = self
            .data
            .flingy(ty.flingy)
            .and_then(|f| self.data.sprite(f.sprite))
            .map(|sp| sp.image);
        if let Some(img) = image.and_then(|id| {
            self.hd_sprites
                .image(self.data.hd_anim_id(id), |path| install?.read(path))
        }) {
            let (gx0, gy0, gx1, gy1) = (
                (x0 - w).max(cx),
                (y0 - h).max(cy),
                (x0 + 2 * w).min(cx + cw),
                (y0 + 2 * h).min(cy + ch),
            );
            let before: Vec<u32> = (gy0..gy1)
                .flat_map(|y| (gx0..gx1).map(move |x| (x, y)))
                .map(|(x, y)| fb.get(x, y).0)
                .collect();
            galaxycraft::hd_sprites::draw_frame(
                fb,
                &img,
                0,
                sx,
                sy,
                s,
                false,
                None,
                (cx, cy, cx + cw, cy + ch),
            );
            let mut k = 0;
            for y in gy0..gy1 {
                for x in gx0..gx1 {
                    let (a, b) = (before[k], fb.get(x, y).0);
                    k += 1;
                    if a != b {
                        fb.put(x, y, Rgb(a).blend(Rgb(b), 128));
                    }
                }
            }
        }
        for y in y0.max(cy)..(y0 + h).min(cy + ch) {
            for x in x0.max(cx)..(x0 + w).min(cx + cw) {
                let free = tiles.as_ref().map_or(valid, |tl| {
                    let i = (((y - y0) * th / h.max(1)).min(th - 1) * tw
                        + ((x - x0) * tw / w.max(1)).min(tw - 1))
                        as usize;
                    tl.get(i).copied().unwrap_or(valid)
                });
                let (fr, fg, fb_) = if free {
                    (0.62, 1.6, 0.62)
                } else {
                    (1.6, 0.62, 0.62)
                };
                let p = fb.get(x, y).0;
                let ch = |v: u32, f: f32| ((v as f32 * f).round() as u32).min(255);
                let out = ch((p >> 16) & 0xff, fr) << 16
                    | ch((p >> 8) & 0xff, fg) << 8
                    | ch(p & 0xff, fb_);
                fb.put(x, y, Rgb(out));
            }
        }
    }

    /// Overlays one image's animation `anim` attaches (the Terran scaffold's
    /// shadow, `TBldSml` Init -> image 331).
    pub(super) fn hd_image_layers(&self, image: u16, anim: usize) -> Vec<(u16, i8, i8, bool)> {
        let Some(script) = self.data.image(image).map(|i| i.iscript_id as u16) else {
            return Vec::new();
        };
        let Some(mut st) = self.data.iscript.start_animation(script, anim, 0) else {
            return Vec::new();
        };
        let mut ctx = galaxycraft::iscript::AnimContext::default();
        let mut out = Vec::new();
        for _ in 0..4 {
            for e in self.data.iscript.tick_anim(&mut st, &mut ctx) {
                if let galaxycraft::iscript::AnimEvent::Overlay {
                    id, x, y, below, ..
                } = e
                {
                    out.push((id, x, y, below));
                }
            }
        }
        out
    }

    /// First `playfram` of an image's animation (the AlmostBuilt frame).
    pub(super) fn first_frame_of(&self, image: u16, anim: usize) -> Option<usize> {
        let script = self.data.image(image)?.iscript_id as u16;
        let mut st = self.data.iscript.start_animation(script, anim, 0)?;
        let mut ctx = galaxycraft::iscript::AnimContext::default();
        for _ in 0..4 {
            for e in self.data.iscript.tick_anim(&mut st, &mut ctx) {
                if let galaxycraft::iscript::AnimEvent::Frame { raw, .. } = e {
                    return Some(raw as usize);
                }
            }
        }
        None
    }

    /// Overlays the unit's Init animation attaches (`imgul` shadows below,
    /// `imgol` glows above), from the retail iscript: image id, offset and
    /// whether it is drawn under the main image.
    /// A finished building also gets the overlays of its `Built`
    /// animation (iscript entry 16): the Nexus glow `pneGlow` (image 181),
    /// whose point light the retail client draws (2026-11-04 capture).
    pub(super) fn hd_unit_layers(&self, type_id: u16, built: bool) -> Vec<(u16, i8, i8, bool)> {
        const INIT: usize = 0;
        const BUILT: usize = 16;
        let ty = self.data.unit(type_id);
        let Some(script) = self.data.unit_iscript_id(ty) else {
            return Vec::new();
        };
        let mut out: Vec<(u16, i8, i8, bool)> = Vec::new();
        let anims: &[usize] = if built { &[INIT, BUILT] } else { &[INIT] };
        for &anim in anims {
            let Some(mut st) = self.data.iscript.start_animation(script, anim, 0) else {
                continue;
            };
            let mut ctx = galaxycraft::iscript::AnimContext::default();
            for _ in 0..4 {
                for e in self.data.iscript.tick_anim(&mut st, &mut ctx) {
                    if let galaxycraft::iscript::AnimEvent::Overlay {
                        id, x, y, below, ..
                    } = e
                        && !out.iter().any(|o| o.0 == id)
                    {
                        out.push((id, x, y, below));
                    }
                }
                if st.ended {
                    break;
                }
            }
        }
        out
    }

    /// Screen placement of a unit's HD art: the main image, the overlays its
    /// Init animation attaches and the geyser smoke.  Shared by drawing and
    /// by the point-light pass, so lights sit exactly on what is drawn.
    pub(super) fn hd_unit_parts(
        &self,
        i: usize,
        content: (i32, i32, i32, i32),
        s: f32,
    ) -> Option<HdParts> {
        let u = &self.world.units[i];
        let img = self.hd_unit_image(u.type_id)?;
        // Recorded capture pose (render only), else the unit's own state and
        // the Video option's render-only smoothed facing.
        let (ux, uy, facing) = self.unit_poses.get(&u.id).copied().unwrap_or((
            u.x / 256,
            u.y / 256,
            self.rendered_facing(u),
        ));
        let directional = img.header.frames.len() >= 17;
        let (frame, flip) = if directional {
            let dir = ((facing as i32 + 4) / 8).rem_euclid(32);
            if dir <= 16 {
                (dir as usize, false)
            } else {
                ((32 - dir) as usize, true)
            }
        } else {
            (0, false)
        };
        let (cx, cy, _, _) = content;
        let sx = cx as f32 + (ux - self.camera.0) as f32 * s;
        let sy = cy as f32 + (uy - self.camera.1) as f32 * s;
        let ty = self.data.unit(u.type_id);
        let mut image_id = self
            .data
            .flingy(ty.flingy)
            .and_then(|f| self.data.sprite(f.sprite))
            .map(|sp| sp.image)?;
        let install = self.install.as_ref();
        let fetch = |id: u16| {
            self.hd_sprites
                .image(self.data.hd_anim_id(id), |path| install?.read(path))
        };
        let (mut img, mut frame) = (img, frame);
        if directional
            && !ty.is_building()
            && let Some(raw) = self.animator.unit_frame(u.id)
        {
            // The walking / attacking pose from the unit's iscript.
            let posed = raw as usize + frame;
            if posed < img.header.frames.len() {
                frame = posed;
            }
        }
        if u.type_id == galaxycraft::ids::unit::SIEGE_MODE {
            let f = self.siege_base_frame(u);
            if f < img.header.frames.len() {
                frame = f;
            }
        }
        let mut layer_list = self.hd_unit_layers(u.type_id, u.is_completed_building(&self.data));
        if let Some(stage) = terran_construction_stage(u, ty) {
            match stage {
                ConstructionStage::Scaffold(n) if ty.construction_image == 0 => {
                    frame = self
                        .first_frame_of(image_id, CONSTRUCTION_STAGE_ANIMS[n.min(2)])
                        .unwrap_or(frame);
                }
                ConstructionStage::Scaffold(n) => {
                    if let Some(si) = fetch(ty.construction_image) {
                        image_id = ty.construction_image;
                        img = si;
                        frame = n;
                        layer_list = self.hd_image_layers(ty.construction_image, 0);
                    }
                }
                ConstructionStage::AlmostBuilt => {
                    frame = self
                        .first_frame_of(image_id, ALMOST_BUILT_ANIM)
                        .unwrap_or(0);
                }
            }
        }
        // Zerg: a building forming from a drone shows the animated
        // construction image; a morph (Hatchery -> Lair, Creep Colony ->
        // Sunken / Spore, Spire -> Greater Spire) keeps the source building
        // until it completes.
        if !u.completed && ty.is_building() {
            if let Some((ci, cf)) = self.animator.construction_pose(u.id) {
                if let Some(cimg) = fetch(ci) {
                    let n = cimg.header.frames.len().max(1);
                    image_id = ci;
                    img = cimg;
                    frame = (cf as usize).min(n - 1);
                    layer_list = Vec::new();
                }
            } else if let Some(src) = galaxycraft::sim::building_morph_source(u.type_id)
                && let Some(src_image) = galaxycraft::unit_anim::unit_image(&self.data, src)
                && let Some(simg) = fetch(src_image)
            {
                image_id = src_image;
                img = simg;
                frame = 0;
                layer_list = self.hd_unit_layers(src, true);
            }
        }
        let layers = layer_list
            .into_iter()
            .filter_map(|(id, ox, oy, below)| {
                let li = fetch(id)?;
                let lf = frame.min(li.header.frames.len().saturating_sub(1));
                Some(HdPart {
                    image_id: id,
                    img: li,
                    frame: lf,
                    x: sx + ox as f32 * s,
                    y: sy + oy as f32 * s,
                    below,
                })
            })
            .collect();
        // Geyser smoke (`creategasoverlays`): image 430 + overlay at the
        // geyser's LO offset, above it.
        let smoke =
            self.geyser_smoke
                .filter(|g| g.0 == u.id)
                .and_then(|(_, image, sframe, dx, dy)| {
                    Some(HdPart {
                        image_id: image,
                        img: fetch(image)?,
                        frame: sframe,
                        x: sx + dx * s,
                        y: sy + dy * s,
                        below: false,
                    })
                });
        Some(HdParts {
            main: HdPart {
                image_id,
                img,
                frame,
                x: sx,
                y: sy,
                below: false,
            },
            flip,
            layers,
            smoke,
        })
    }

    /// Point lights (`anim/main.lit`) of every visible unit's lit parts,
    /// placed on screen for the deferred-blit term.
    pub(super) fn hd_light_env(
        &self,
        units: &[usize],
        content: (i32, i32, i32, i32),
        s: f32,
    ) -> galaxycraft::hd_lights::LightEnv {
        let (cx, cy, cw, ch) = content;
        let mut env =
            galaxycraft::hd_lights::LightEnv::new((cx as f32, cy as f32, cw as f32, ch as f32), s);
        let Some(table) = self.hd_lights.as_ref() else {
            return env;
        };
        for &i in units {
            let Some(parts) = self.hd_unit_parts(i, content, s) else {
                continue;
            };
            // Drawn like the sprites: main image and overlays mirror with the
            // unit, the smoke never does.
            let lit = std::iter::once((&parts.main, parts.flip))
                .chain(
                    parts
                        .layers
                        .iter()
                        .filter(|p| !p.below)
                        .map(|p| (p, parts.flip)),
                )
                .chain(parts.smoke.iter().map(|p| (p, false)));
            for (p, flip) in lit {
                let Some(rec) = table.get(p.image_id, p.frame) else {
                    continue;
                };
                let Some(f) = p.img.header.frames.get(p.frame) else {
                    continue;
                };
                env.lights.push(galaxycraft::hd_lights::place(
                    rec,
                    f.x,
                    f.y,
                    p.img.header.frame_w,
                    p.img.header.frame_h,
                    p.x,
                    p.y,
                    s,
                    flip,
                ));
            }
        }
        env
    }

    /// Draw one unit from its HD sprite at screen resolution.  Directional
    /// images store 17 facings (north..south through east); the western ones
    /// are the mirrored frames, as in the SD GRPs.
    /// The SD view's counterpart of [`Self::draw_animated_images`]: the same
    /// poses drawn at 1x from the images' SD GRPs.
    pub(super) fn draw_animated_images_sd(&mut self, fb: &mut Framebuffer) {
        let local = self.local_player as u8;
        let poses: Vec<(u16, i32, i32, u8, u16)> = self
            .animator
            .damage_overlays(&self.world)
            .into_iter()
            .map(|o| (o.image, o.x, o.y, 0u8, o.frame))
            .chain(
                self.animator
                    .bullets()
                    .into_iter()
                    .map(|b| (b.image, b.x, b.y, b.facing, b.frame)),
            )
            .chain(
                self.animator
                    .carry_overlays(&self.world)
                    .into_iter()
                    .map(|b| (b.image, b.x, b.y, b.facing, b.frame)),
            )
            .chain(
                self.animator
                    .status_overlays(&self.world)
                    .into_iter()
                    .map(|b| (b.image, b.x, b.y, b.facing, b.frame)),
            )
            .chain(
                self.animator
                    .effects()
                    .map(|e| (e.image, e.x, e.y, e.facing, e.frame)),
            )
            .collect();
        for (image, x, y, facing, raw) in poses {
            if self.fog_on() && !self.world.is_visible_to(local, x, y) {
                continue;
            }
            let Some(grp) = self.sd_image_grp(image) else {
                continue;
            };
            let index =
                galaxycraft::render::sd_frame_index(grp.frames.len(), facing, Some(raw), false, 0);
            let index = if grp.frame(index).is_some() {
                index
            } else {
                raw as usize
            };
            // `images.dat` draw function 9 (remapped glows: fire, heal,
            // lockdown, smoke): the pixel values are strengths of a tinted
            // light, not palette colours.  INFERENCE: tints approximate
            // the `ofire` / `gfire` / `bfire` / `bexpl` remap tables.
            let glow = self
                .data
                .image(image)
                .filter(|i| i.modifier == 9)
                .map(|i| i.color_shift);
            // The retail way: through the tileset's remap table.
            if let Some(remap) = glow
                && let Some(table) = self.remap_table(remap)
            {
                galaxycraft::render::draw_grp_remapped(
                    fb,
                    &grp,
                    index,
                    (x - self.camera.0, y - self.camera.1),
                    galaxycraft::render::sd_frame_flip(grp.frames.len(), facing),
                    &table,
                );
                continue;
            }
            // Without the install's table: a tinted light.
            let ink = match glow {
                Some(shift) => galaxycraft::render::GrpInk::Glow(match shift {
                    2 => Rgb::new(90, 255, 90),
                    3 | 4 => Rgb::new(90, 170, 255),
                    _ => Rgb::new(255, 150, 40),
                }),
                None => galaxycraft::render::GrpInk::Solid(None),
            };
            galaxycraft::render::draw_grp_posed(
                fb,
                &grp,
                index,
                self.sprites.palette(),
                (x - self.camera.0, y - self.camera.1),
                galaxycraft::render::sd_frame_flip(grp.frames.len(), facing),
                ink,
            );
        }
        // Engine glows of moving units (the SCV's thruster, image 249):
        // the glow's own direction frame, alternating its two frame sets
        // (iscript 291: `engset 0`, `engset 1`, one tick each).
        let glows: Vec<(u16, i32, i32, u8)> = self
            .world
            .units
            .iter()
            .enumerate()
            .filter(|(i, u)| {
                u.alive
                    && !self.world.inside_gas_building(*i)
                    && (!self.fog_on()
                        || u.owner as usize == self.local_player
                        || self.world.can_see_unit(local, *i))
            })
            .filter_map(|(_, u)| {
                let glow = self.animator.unit_glow(u.id)?;
                Some((glow, u.x / 256, u.y / 256, self.rendered_facing(u)))
            })
            .collect();
        let set = (self.world.tick % 2) as u16 * 17;
        for (image, x, y, facing) in glows {
            let Some(grp) = self.sd_image_grp(image) else {
                continue;
            };
            let index =
                galaxycraft::render::sd_frame_index(grp.frames.len(), facing, Some(set), false, 0);
            let remap = self.data.image(image).map_or(1, |i| i.color_shift);
            if let Some(table) = self.remap_table(remap) {
                galaxycraft::render::draw_grp_remapped(
                    fb,
                    &grp,
                    index,
                    (x - self.camera.0, y - self.camera.1),
                    galaxycraft::render::sd_frame_flip(grp.frames.len(), facing),
                    &table,
                );
                continue;
            }
            galaxycraft::render::draw_grp_posed(
                fb,
                &grp,
                index,
                self.sprites.palette(),
                (x - self.camera.0, y - self.camera.1),
                galaxycraft::render::sd_frame_flip(grp.frames.len(), facing),
                galaxycraft::render::GrpInk::Glow(Rgb::new(255, 150, 40)),
            );
        }
    }

    /// The remap table an image's `images.dat` remapping names, for the
    /// current tileset.
    pub(super) fn remap_table(
        &mut self,
        remap: u8,
    ) -> Option<std::rc::Rc<galaxycraft::render::RemapTable>> {
        let name = match remap {
            1 => "ofire",
            2 => "gfire",
            3 => "bfire",
            4 => "bexpl",
            _ => return None,
        };
        let era = self.scenario.era;
        if let Some(t) = self.remap_tables.get(&(era, remap)) {
            return t.clone();
        }
        let path = format!("tileset/{}/{name}.pcx", galaxycraft::render::era_name(era));
        let table = self
            .install
            .as_ref()
            .and_then(|i| i.read(&path))
            .and_then(|b| galaxycraft::render::RemapTable::parse(&b))
            .map(std::rc::Rc::new);
        self.remap_tables.insert((era, remap), table.clone());
        table
    }

    /// `arr/images.tbl` (index n = entry n - 1), read once from the install.
    pub(super) fn image_table(&mut self) -> &[String] {
        if self.image_names.is_none() {
            self.image_names = Some(
                self.install
                    .as_ref()
                    .and_then(|i| i.read("arr/images.tbl"))
                    .and_then(|b| galaxycraft::tables::StringTable::parse(&b).ok())
                    .map(|t| (0..t.len() as u16).map(|i| t.text(i).to_string()).collect())
                    .unwrap_or_default(),
            );
        }
        self.image_names.as_deref().unwrap_or(&[])
    }

    /// SD GRP of an image (`images.dat` GRP name under `unit\\`), cached.
    pub(super) fn sd_image_grp(
        &mut self,
        image: u16,
    ) -> Option<std::rc::Rc<galaxycraft::render::Grp>> {
        if let Some(g) = self.sd_image_grps.get(&image) {
            return g.clone();
        }
        let index = self.data.image(image).map_or(0, |i| i.grp_filename_index) as usize;
        let name = index
            .checked_sub(1)
            .and_then(|i| self.image_table().get(i).cloned());
        let grp = name
            .and_then(|n| {
                self.install
                    .as_ref()?
                    .read(&format!("unit/{}", n.replace('\\', "/")))
            })
            .and_then(|b| galaxycraft::render::Grp::parse(&b))
            .map(std::rc::Rc::new);
        self.sd_image_grps.insert(image, grp.clone());
        grp
    }

    /// Bullets in flight and death / impact effects from the animator,
    /// drawn at screen resolution over the units (the fog covers them).
    pub(super) fn draw_animated_images(
        &self,
        fb: &mut Framebuffer,
        content: (i32, i32, i32, i32),
        s: f32,
        env: &galaxycraft::hd_lights::LightEnv,
    ) {
        let (cx, cy, cw, ch) = content;
        let clip = (cx, cy, cx + cw, cy + ch);
        let install = self.install.as_ref();
        let poses = self
            .animator
            .damage_overlays(&self.world)
            .into_iter()
            .map(|o| (o.image, o.x, o.y, 0u8, o.frame))
            .chain(
                self.animator
                    .bullets()
                    .into_iter()
                    .map(|b| (b.image, b.x, b.y, b.facing, b.frame)),
            )
            .chain(
                self.animator
                    .carry_overlays(&self.world)
                    .into_iter()
                    .map(|b| (b.image, b.x, b.y, b.facing, b.frame)),
            )
            .chain(
                self.animator
                    .status_overlays(&self.world)
                    .into_iter()
                    .map(|b| (b.image, b.x, b.y, b.facing, b.frame)),
            )
            .chain(
                self.animator
                    .effects()
                    .map(|e| (e.image, e.x, e.y, e.facing, e.frame)),
            );
        let local = self.local_player as u8;
        for (image, x, y, facing, raw) in poses {
            // Nothing burns or flies where the player cannot see.
            if self.fog_on() && !self.world.is_visible_to(local, x, y) {
                continue;
            }
            let Some(img) = self
                .hd_sprites
                .image(self.data.hd_anim_id(image), |path| install?.read(path))
            else {
                continue;
            };
            let n = img.header.frames.len();
            let (frame, flip) = if n >= 17 {
                let dir = ((facing as i32 + 4) / 8).rem_euclid(32);
                let (d, flip) = if dir <= 16 {
                    (dir as usize, false)
                } else {
                    ((32 - dir) as usize, true)
                };
                let posed = raw as usize + d;
                if posed < n {
                    (posed, flip)
                } else {
                    (raw as usize, false)
                }
            } else {
                (raw as usize, false)
            };
            if frame >= n {
                continue;
            }
            let sx = cx as f32 + (x - self.camera.0) as f32 * s;
            let sy = cy as f32 + (y - self.camera.1) as f32 * s;
            galaxycraft::hd_sprites::draw_frame_lit(
                fb, &img, frame, sx, sy, s, flip, None, clip, env,
            );
        }
    }

    /// Green circle on the ground where a move order was given, blinking
    /// like the target flash.  INFERENCE: retail's cursor-marker sprite is
    /// approximated with the 32 px selection circle.
    pub(super) fn draw_order_marker(
        &mut self,
        fb: &mut Framebuffer,
        clip: (i32, i32, i32, i32),
        scale: f32,
    ) {
        let Some(((wx, wy), t)) = self.order_marker else {
            return;
        };
        let age = t.elapsed().as_secs_f32();
        if age > ORDER_FLASH_SECS {
            self.order_marker = None;
            return;
        }
        if !order_flash_on(age) {
            return;
        }
        let install = self.install.as_ref();
        let Some(img) = self.hd_sprites.image(561 + 1, |path| install?.read(path)) else {
            return;
        };
        let s = scale;
        let sx = clip.0 as f32 + (wx - self.camera.0) as f32 * scale;
        let sy = clip.1 as f32 + (wy - self.camera.1) as f32 * scale;
        galaxycraft::hd_sprites::draw_flat(fb, &img, 0, sx, sy, s, HUD_SELECT_GREEN, clip);
    }

    /// Capture the Flag marker: a small pole and coloured pennant over the
    /// beacon/worker.  `content`/`scale` are used in the HD pass; the SD pass
    /// draws in logical game pixels.
    pub(super) fn draw_ctf_flags(
        &self,
        fb: &mut Framebuffer,
        content: (i32, i32, i32, i32),
        scale: f32,
        hd: bool,
    ) {
        let Some(ctf) = self.world.ctf.as_ref() else {
            return;
        };
        for flag in &ctf.flags {
            if !flag.released {
                continue;
            }
            let (wx, wy) = flag.pos;
            let (sx, sy) = if hd {
                (
                    content.0 as f32 + (wx - self.camera.0) as f32 * scale,
                    content.1 as f32 + (wy - self.camera.1) as f32 * scale,
                )
            } else {
                ((wx - self.camera.0) as f32, (wy - self.camera.1) as f32)
            };
            let (x, y) = (sx as i32, sy as i32);
            let (cx, cy, cw, ch) = content;
            if x < cx - 32 || y < cy - 32 || x > cx + cw + 32 || y > cy + ch + 32 {
                continue;
            }
            let color = galaxycraft::render::player_rgb(&self.world, flag.owner);
            let s = if hd { scale.max(1.0) } else { 1.0 };
            let pole = (2.0 * s).max(1.0) as i32;
            let height = (24.0 * s) as i32;
            fb.rect(x, y - height, pole, height, Rgb::new(30, 30, 30));
            fb.rect(
                x + pole,
                y - height,
                (14.0 * s).max(4.0) as i32,
                (9.0 * s).max(3.0) as i32,
                color,
            );
        }
    }

    pub(super) fn draw_unit_hd(
        &self,
        fb: &mut Framebuffer,
        i: usize,
        content: (i32, i32, i32, i32),
        s: f32,
        env: &galaxycraft::hd_lights::LightEnv,
    ) {
        let u = &self.world.units[i];
        let Some(parts) = self.hd_unit_parts(i, content, s) else {
            return;
        };
        let (img, frame, flip, sx, sy) = (
            parts.main.img.clone(),
            parts.main.frame,
            parts.flip,
            parts.main.x,
            parts.main.y,
        );
        let (cx, cy, cw, ch) = content;
        let team = match self.local_team_color {
            Some(c) if u.owner as usize == self.local_player => c,
            _ => galaxycraft::render::player_rgb(&self.world, u.owner),
        };
        let clip = (cx, cy, cx + cw, cy + ch);
        let install = self.install.as_ref();
        let layer = |below: bool, fb: &mut Framebuffer| {
            for p in parts.layers.iter().filter(|l| l.below == below) {
                if below {
                    if !self.game_options.video_shadow_stacking {
                        continue;
                    }
                    galaxycraft::hd_sprites::draw_shadow(
                        fb,
                        &p.img,
                        p.frame,
                        p.x,
                        p.y,
                        s,
                        flip,
                        HD_SHADOW_ALPHA,
                        clip,
                    );
                } else {
                    galaxycraft::hd_sprites::draw_frame_lit(
                        fb, &p.img, p.frame, p.x, p.y, s, flip, None, clip, env,
                    );
                }
            }
        };
        // Any selected unit shows its circle and health bar: green for the
        // player's, red for an enemy's, yellow for allied / neutral ones.
        let flashing = self
            .order_flash
            .is_some_and(|(id, t)| id == u.id && order_flash_on(t.elapsed().as_secs_f32()));
        let selected = self.selected.contains(&u.id) || flashing;
        let circle_colour = circle_colour_for(
            u.owner as usize == self.local_player,
            u.is_resource() || u.owner >= 11,
            self.world.is_enemy(self.local_player as u8, u.owner),
        );
        let sprite = self
            .data
            .flingy(self.data.unit(u.type_id).flingy)
            .and_then(|f| self.data.sprite(f.sprite))
            .cloned();
        // Selection circle (image 561 + size) under the unit, at vpos.
        let circle = sprite.as_ref().filter(|_| selected).and_then(|sp| {
            let img = self
                .hd_sprites
                .image(561 + sp.selection_circle as u16, |path| install?.read(path))?;
            Some((img, sp.selection_circle_vpos as f32, sp.health_bar_size))
        });
        if let Some((ci, vpos, _)) = &circle {
            galaxycraft::hd_sprites::draw_flat(
                fb,
                ci,
                0,
                sx,
                sy + vpos * s,
                s,
                circle_colour,
                clip,
            );
        }
        let ty = self.data.unit(u.type_id);
        let protoss_building = ty.is_building()
            && self.world.players.get(u.owner as usize).map(|p| p.race) == Some(Race::Protoss);
        if protoss_building && !u.completed {
            // Warping in: a blue, shimmering silhouette that firms up as it
            // completes.  INFERENCE: retail draws a warp texture through
            // the building's shape; the timing of the shimmer is ours.
            let progress = 1.0 - u.build_remaining as f32 / u.build_total.max(1) as f32;
            let shimmer = ((self.world.tick as f32) * 0.25).sin() * 0.08;
            galaxycraft::hd_sprites::draw_tinted(
                fb,
                &img,
                frame,
                sx,
                sy,
                s,
                flip,
                (0.35 + 0.45 * progress + shimmer).clamp(0.2, 0.9),
                Rgb::new(70, 120, 255),
                clip,
            );
            return;
        }
        layer(true, fb);
        // Idle hover bob of flyers (`setvertpos`), on the main image only.
        let bob_y = sy + self.animator.unit_vert(u.id) as f32 * s;
        // Status images behind the unit (Defensive Matrix back shell).
        for (image, sframe) in self.animator.status_underlays(u.id) {
            if let Some(si) = self
                .hd_sprites
                .image(self.data.hd_anim_id(image), |path| install?.read(path))
                && (sframe as usize) < si.header.frames.len()
            {
                galaxycraft::hd_sprites::draw_frame_lit(
                    fb,
                    &si,
                    sframe as usize,
                    sx,
                    sy,
                    s,
                    false,
                    None,
                    clip,
                    env,
                );
            }
        }
        galaxycraft::hd_sprites::draw_frame_lit(
            fb,
            &img,
            frame,
            sx,
            bob_y,
            s,
            flip,
            Some(team),
            clip,
            env,
        );
        layer(false, fb);
        // The turret (tank barrel, Goliath torso) on top, pointing at the
        // target.
        if let Some((timage, tfacing)) = self.turret_pose(u) {
            let skin = unit_skin_id(u.type_id, &self.game_options);
            if let Some(ti) =
                self.hd_sprites
                    .image_skin(self.data.hd_anim_id(timage), skin, |path| {
                        install?.read(path)
                    })
            {
                let n = ti.header.frames.len();
                let raw = self
                    .siege_turret_frame(u)
                    .or_else(|| self.animator.turret_frame(u.id))
                    .unwrap_or(0) as usize;
                let (tframe, tflip) = if n >= 17 {
                    let dir = ((tfacing as i32 + 4) / 8).rem_euclid(32);
                    let (d, flip) = if dir <= 16 {
                        (dir as usize, false)
                    } else {
                        ((32 - dir) as usize, true)
                    };
                    (if raw + d < n { raw + d } else { d }, flip)
                } else {
                    (0, false)
                };
                if tframe < n {
                    galaxycraft::hd_sprites::draw_frame_lit(
                        fb,
                        &ti,
                        tframe,
                        sx,
                        sy,
                        s,
                        tflip,
                        Some(team),
                        clip,
                        env,
                    );
                }
            }
        }
        // Engine glow on top of a moving unit (the SCV's thruster, image
        // 249), on the unit's own frame.
        if let Some(glow) = self.animator.unit_glow(u.id) {
            let install = self.install.as_ref();
            if let Some(gi) = self.hd_sprites.image(glow, |path| install?.read(path)) {
                // `tscGlow` carries four flicker frames per 17-direction
                // block (68 frames against the SCV's 51): keep the unit's
                // direction and cycle the flicker.  INFERENCE: two ticks
                // per flicker frame.
                let flicker = (self.world.tick / 2 % 4) as usize;
                let gframe = frame % 17 + 17 * flicker;
                galaxycraft::hd_sprites::draw_frame(fb, &gi, gframe, sx, sy, s, flip, None, clip);
            }
        }
        if protoss_building && u.completed && !self.world.building_powered(i) {
            // No power: the building goes dark (retail dims and stops it).
            galaxycraft::hd_sprites::draw_shadow(fb, &img, frame, sx, sy, s, flip, 0.55, clip);
        }
        if let Some(p) = &parts.smoke {
            galaxycraft::hd_sprites::draw_frame_lit(
                fb, &p.img, p.frame, p.x, p.y, s, false, None, clip, env,
            );
        }
        // The order flash shows the circle only, no health bar.
        let circle = circle.filter(|_| self.selected.contains(&u.id));
        if let Some((ci, vpos, bar)) = circle {
            // Retail `draw_health_bars`: centred `vpos + circle_h / 2 + 8`
            // below the sprite.
            // OpenBW uses the circle's frame-0 height (not the GRP box).
            let circle_h = ci
                .header
                .frames
                .first()
                .map_or(0, |f| (f.h / ci.header.scale.max(1) as u16) as i32);
            let ty = self.data.unit(u.type_id);
            let max_hp = (ty.hit_points_raw as i32 / 256).max(1);
            let hp_pct = (u.hp / 256) * 100 / max_hp;
            let shield_pct = ty
                .has_shields
                .then(|| (u.shields / 256) * 100 / (ty.shield_points as i32).max(1));
            let gy = u.y / 256 + vpos as i32 + circle_h / 2 + 8;
            draw_health_bar(
                fb,
                (sx, cy as f32 + (gy - self.camera.1) as f32 * s),
                bar as i32,
                hp_pct,
                shield_pct,
                s,
                clip,
            );
        }
    }

    /// Environment Effects overlay: deterministic era-coloured particles
    /// from the shipped `hd2/effect/particle.dds`, positioned by the mission
    /// tick so a capture or replay always draws the same field.
    pub(super) fn draw_environment_effects(
        &mut self,
        fb: &mut Framebuffer,
        content: (i32, i32, i32, i32),
    ) {
        if !self.env_particle_loaded {
            self.env_particle_loaded = true;
            self.env_particle = self
                .install
                .as_ref()
                .and_then(|i| i.read("hd2/effect/particle.dds"))
                .and_then(|b| galaxycraft::dds::parse(&b));
        }
        let Some(particle) = self.env_particle.clone() else {
            return;
        };
        let (cx, cy, cw, ch) = content;
        if cw <= 0 || ch <= 0 {
            return;
        }
        let spec = env_spec(self.scenario.era);
        let t = self.world.tick as f32 + self.accumulator.clamp(0.0, 1.0);
        let w = cw as f32;
        let h = ch as f32;
        let unit = (ch as f32 / 1440.0).max(0.35);
        // About one particle per 11k screen px, bounded so a 4K ultrawide
        // window stays cheap and a small window still shows weather.
        let count = (((cw as i64 * ch as i64) / 11_000).clamp(24, 150)) as usize;
        let era = self.scenario.era.min(7) as u32;
        for i in 0..count {
            let seed = i as u32 * 7 + era * 131;
            let h1 = env_hash(seed + 1);
            let h2 = env_hash(seed + 2);
            let h3 = env_hash(seed + 3);
            let h4 = env_hash(seed + 4);
            let frac = |v: u32| v as f32 / u32::MAX as f32;
            let speed = spec.fall * (0.6 + frac(h4) * 0.8);
            let drift = spec.drift * (0.6 + frac(h3) * 0.8);
            let mut x = (frac(h1) * w + t * drift).rem_euclid(w)
                + (t * 0.11 + frac(h4) * std::f32::consts::TAU).sin() * spec.sway;
            let y = (frac(h2) * h + t * speed).rem_euclid(h);
            x = x.rem_euclid(w);
            let size = (spec.size * unit * (0.7 + frac(h3) * 0.6)).round().max(1.0) as i32;
            let alpha = (spec.alpha as f32 * (0.5 + frac(h2) * 0.5)).round() as u8;
            draw_env_particle(
                fb,
                &particle,
                cx + x.round() as i32,
                cy + y.round() as i32,
                size,
                spec.colour,
                alpha,
            );
        }
    }
}
