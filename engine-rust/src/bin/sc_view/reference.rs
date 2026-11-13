//! `impl App`: reference (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    /// `--game-script` state line: tick, resources, supply, selection,
    /// command card, build mode, own unit counts by type and the status.
    pub(super) fn game_state_line(&self) -> String {
        let me = self.local_player as u8;
        let p = &self.world.players[self.view_player()];
        let names = |ids: &[u16]| {
            ids.iter()
                .map(|&t| self.data.unit_name(t).to_string())
                .collect::<Vec<_>>()
                .join("+")
        };
        let selected: Vec<String> = self
            .selected
            .iter()
            .filter_map(|id| self.world.unit(*id))
            .map(|u| self.data.unit_name(u.type_id).to_string())
            .collect();
        let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
        for u in self.world.units.iter().filter(|u| u.alive && u.owner == me) {
            *counts
                .entry(self.data.unit_name(u.type_id).to_string())
                .or_default() += 1;
        }
        let counts: Vec<String> = counts.iter().map(|(k, v)| format!("{k}={v}")).collect();
        let first = self
            .first_selected()
            .map(|u| {
                format!(
                    "{}@({},{}) order={:?} carry={} prod={:?} done={} remaining={} winner={:?}",
                    self.data.unit_name(u.type_id),
                    u.x / 256,
                    u.y / 256,
                    u.order,
                    u.carry,
                    u.production,
                    u.completed,
                    u.build_remaining,
                    self.world.winner
                )
            })
            .unwrap_or_default();
        let target = self
            .first_selected()
            .and_then(|u| match u.order {
                Order::Gather { resource } | Order::AttackUnit { target: resource } => {
                    self.world.unit(resource).map(|t| {
                        format!(
                            " target={}@({},{}) dist={}",
                            self.data.unit_name(t.type_id),
                            t.x / 256,
                            t.y / 256,
                            ((t.x - u.x).abs() + (t.y - u.y).abs()) / 256
                        )
                    })
                }
                _ => None,
            })
            .unwrap_or_default();
        format!(
            "camera={:?} card={:?} target={:?} first=[{first}{target}] ",
            self.camera, self.card_set, self.target_act
        ) + &format!(
            "tick={} minerals={} gas={} supply={}/{} selected=[{}] commands=[{}] build={} units=[{}] status={:?} result={:?} players=[{}]",
            self.world.tick,
            p.minerals,
            p.gas,
            self.world.supply_used(me),
            self.world.supply_cap(me),
            selected.join("+"),
            names(&self.command_list),
            self.build_mode
                .map(|t| self.data.unit_name(t).to_string())
                .unwrap_or_default(),
            counts.join(" "),
            self.status,
            self.game_result,
            (0..12u8)
                .filter_map(|p| {
                    let n = self
                        .world
                        .units
                        .iter()
                        .filter(|u| u.alive && u.owner == p && !u.is_resource())
                        .count();
                    (n > 0).then(|| format!("p{p}:{n}"))
                })
                .collect::<Vec<_>>()
                .join(" ")
        ) + &format!(
            " dialog={} elapsed={}",
            self.end_dialog && self.end_dialog_hold == 0 && self.game_result.is_some(),
            self.elapsed_ticks()
        ) + &format!(
            " ais=[{}]",
            self.campaign_ais
                .iter()
                .map(|a| format!(
                    "p{}:{}@{}{}",
                    a.player,
                    String::from_utf8_lossy(&a.id),
                    a.pc,
                    if a.finished { "!" } else { "" }
                ))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }

    /// Prepare a headless in-game capture: leave the menu, size the logical
    /// view, reveal vision around the player's units and centre the camera.
    pub(super) fn prepare_ingame_shot(&mut self, fb: &Framebuffer) {
        // The references were taken with the radar's terrain layer switched
        // off (their tooltip is hovering that very button), so the headless
        // captures use the same state to stay comparable.
        self.minimap_terrain = false;
        self.menu = false;
        self.intro = false;
        self.briefing = false;
        self.update_view(fb);
        if !self.spawn_reference_scene(fb) {
            self.select_reference_selection();
            self.world.update_vision();
            self.centre_on_local_unit();
            if let Some(cam) = Self::cam_override() {
                self.camera = cam;
            }
        }
    }

    /// `GC_CAM=x,y`: capture calibration that pins the camera, so scene and
    /// no-scene captures can be compared over the same terrain.
    pub(super) fn cam_override() -> Option<(i32, i32)> {
        let v: Vec<i32> = std::env::var("GC_CAM")
            .ok()?
            .split(',')
            .filter_map(|t| t.trim().parse().ok())
            .collect();
        (v.len() == 2).then(|| (v[0], v[1]))
    }

    /// Reconstruct the reference frame: place the units the screenshot shows
    /// at the screen positions they occupy there, with the camera fixed so the
    /// layout lands where the reference has it. Returns false when no scene has
    /// been reconstructed for this capture (the map's own state is used then).
    pub(super) fn spawn_reference_scene(&mut self, fb: &Framebuffer) -> bool {
        // Opt-in (`GC_SCENE=1`): the reconstructed scene is faithful — the
        // supply-depot grid lands on the reference's own grid — but the SD art
        // we own differs from the retail HD art pixel by pixel, which costs
        // more on the measured error than the empty terrain it replaces.
        if std::env::var("GC_SCENE").is_err() {
            return false;
        }
        let Some(scene) = galaxycraft::scene::reference_scene(&self.shot_name) else {
            return false;
        };
        let (cx, _, _, _) = Self::content_rect(fb);
        self.camera = scene.camera;
        let owner = self.local_player;
        let mut wired = false;
        for su in scene.units {
            let per_world = galaxycraft::scene::SCREEN_PER_WORLD;
            let (wx, wy) = galaxycraft::scene::world_of(scene.camera, cx, su.screen, per_world);
            if let Err(e) = self.world.spawn_px(owner, su.type_id, wx, wy) {
                eprintln!("scene: type {} at {wx},{wy}: {e}", su.type_id);
            } else {
                wired = true;
            }
        }
        let _ = wired;
        self.world.update_vision();
        self.select_reference_selection();
        true
    }

    /// Centre the camera on the first local unit, like the retail client does
    /// when a mission starts.
    /// Retail start camera of a melee game: the start location sits half a
    /// view from the left and 200 game px from the top, clamped to the map
    /// (measured on 134905 / 135020 / 135051, (8)The Hunters).
    pub(super) fn start_camera(&mut self) {
        let Some((x, y)) = self.scenario.start_location_of(self.local_player as u8) else {
            return self.centre_on_local_unit();
        };
        self.camera = (x - self.view.0 / 2, y - START_CAMERA_TOP);
        self.clamp_camera();
    }

    /// Reference in-game captures: the start of a melee game on
    /// (8)The Hunters as slot `slot` (the start locations of the user's
    /// three autosaved games, 2026-10-27), with the town hall selected for
    /// Zerg/Protoss and nothing selected for Terran, as on the captures.
    pub(super) fn hunters_start(
        &mut self,
        fb: &Framebuffer,
        slot: usize,
        race: Race,
        nudge: (i32, i32),
    ) {
        // Colours the three games gave the player, measured on the team-mask
        // pixels of 135051 / 134905 (teal, brown) and swept on 135020 (orange).
        self.local_team_color = Some(match race {
            Race::Protoss => Rgb::new(40, 150, 125),
            Race::Zerg => Rgb::new(111, 73, 59),
            // Terran: orange, by sweep over the Command Center region
            // (error 19.1 for the earlier dark grey, 18.6 for orange).
            _ => Rgb::new(220, 120, 20),
        });
        if let Some(i) = self
            .maps
            .iter()
            .position(|m| m.name.to_lowercase().contains("the hunters") && m.players == 8)
        {
            self.map_index = i;
        }
        self.melee_slot = Some(slot);
        self.menu_race = race;
        self.races = None;
        self.load_map(self.map_index);
        self.minimap_terrain = false;
        self.menu = false;
        self.intro = false;
        self.briefing = false;
        self.update_view(fb);
        self.world.update_vision();
        self.selected.clear();
        if race != Race::Terran {
            self.select_reference_selection();
        }
        self.refresh_commands();
        self.start_camera();
        // The player nudged the camera before capturing: measured by
        // aligning the terrain (error 1.96-2.35 after the shift).
        self.camera.0 += nudge.0;
        self.camera.1 += nudge.1;
        self.unit_poses.clear();
        for &(type_id, x, y, facing) in hunters_poses(race) {
            // Nearest still-unassigned own unit of that type.
            let pick = self
                .world
                .units
                .iter()
                .filter(|u| {
                    u.alive
                        && u.owner as usize == self.local_player
                        && u.type_id == type_id
                        && !self.unit_poses.contains_key(&u.id)
                })
                .min_by_key(|u| (u.x / 256 - x).pow(2) + (u.y / 256 - y).pow(2))
                .map(|u| u.id);
            if let Some(id) = pick {
                self.unit_poses.insert(id, (x, y, facing));
            }
        }
        // Geyser smoke of each game's first capture (see `hunters_smoke`).
        self.set_geyser_smoke(hunters_smoke(race));
        self.creep_variants.clear();
        if race == Race::Zerg && slot == 1 {
            self.creep_variants = parse_tile_grid(HUNTERS_ZERG_CREEP.0, HUNTERS_ZERG_CREEP.1);
        }
        self.clamp_camera();
    }

    /// Show `smoke` (`image, frame, LO dx, LO dy` in game px) over the
    /// geyser nearest the local start location, or no smoke.
    pub(super) fn set_geyser_smoke(&mut self, smoke: Option<(u16, usize, f32, f32)>) {
        let start = self
            .scenario
            .start_location_of(self.local_player as u8)
            .unwrap_or((0, 0));
        self.geyser_smoke = smoke.and_then(|(image, frame, dx, dy)| {
            self.world
                .units
                .iter()
                .filter(|u| u.alive && u.type_id == galaxycraft::ids::unit::VESPENE_GEYSER)
                .min_by_key(|u| (u.x / 256 - start.0).pow(2) + (u.y / 256 - start.1).pow(2))
                .map(|g| (g.id, image, frame, dx, dy))
        });
    }

    pub(super) fn centre_on_local_unit(&mut self) {
        if let Some(u) = self
            .world
            .units
            .iter()
            .find(|u| u.alive && u.owner as usize == self.local_player && !u.is_resource())
        {
            let (x, y) = (u.x / 256, u.y / 256);
            self.camera = (x - self.view.0 / 2, y - self.view.1 / 2);
            self.clamp_camera();
        }
    }

    /// Reproduce the selection the reference capture shows: two units for the
    /// Terran screenshots, the main building for the Zerg and Protoss ones.
    pub(super) fn select_reference_selection(&mut self) {
        let building = match self.console_race {
            Race::Zerg => Some([
                galaxycraft::ids::unit::HATCHERY,
                galaxycraft::ids::unit::LAIR,
                galaxycraft::ids::unit::HIVE,
            ]),
            Race::Protoss => Some([galaxycraft::ids::unit::NEXUS; 3]),
            _ => None,
        };
        if let Some(ids) = building
            && let Some(u) = self.world.units.iter().find(|u| {
                u.alive && u.owner as usize == self.local_player && ids.contains(&u.type_id)
            })
        {
            self.selected = vec![u.id];
            self.refresh_commands();
            return;
        }
        self.select_some_units();
    }

    pub(super) fn select_some_units(&mut self) {
        // Prefer units that actually ship a painted portrait so the capture
        // exercises the real art instead of the wireframe fallback.
        let preferred = [
            galaxycraft::ids::unit::MARINE,
            galaxycraft::ids::unit::GHOST,
            galaxycraft::ids::unit::ZEALOT,
            galaxycraft::ids::unit::ZERGLING,
        ];
        let mut chosen: Vec<u32> = Vec::new();
        for want in preferred {
            if let Some(u) = self
                .world
                .units
                .iter()
                .find(|u| u.alive && u.owner as usize == self.local_player && u.type_id == want)
            {
                chosen.push(u.id);
            }
        }
        for u in self
            .world
            .units
            .iter()
            .filter(|u| u.alive && u.owner as usize == self.local_player)
        {
            if chosen.len() >= 2 {
                break;
            }
            if !chosen.contains(&u.id) && chosen.len() < 2 {
                chosen.push(u.id);
            }
        }
        self.selected = chosen;
        self.refresh_commands();
    }

    #[allow(unreachable_code)]
    /// Start the selected campaign mission of `episode` (Enter or the Ok
    /// button); the screen-change cue is the retail `swishin` sound.
    /// Campaign state of the profile on the reference captures: Terran
    /// played up to `8. The Big Push` (131132), everything else fresh, so
    /// Zerg/Protoss and Brood War Terran/Zerg are locked (130757/130804/
    /// 130832/130840 hover captions).
    pub(super) fn set_reference_progress(&mut self) {
        let mut p = galaxycraft::campaign_progress::Progress::default();
        p.next[galaxycraft::campaign_progress::Campaign::Terran.index()] = 9;
        self.set_progress(p);
    }
}
