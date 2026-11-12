//! `impl App`: commands (moved out of `sc_view.rs` by scripts/split_impl.py).

use super::*;

impl App {
    /// Unit under a surface-pixel point (converted to logical game px,
    /// the space of [`Self::screen_of`]).
    pub(super) fn unit_at(&self, px: i32, py: i32) -> Option<u32> {
        let (sx, sy) = self.to_logical(px, py);
        let mut best: Option<(i32, u32)> = None;
        for (i, u) in self.world.units.iter().enumerate() {
            if !u.alive || self.world.inside_gas_building(i) {
                continue;
            }
            // Cannot click an enemy the fog (or an undetected cloak) hides.
            if u.owner as usize != self.local_player
                && (self.fog_on() || u.status.cloaked || u.status.burrowed)
                && !self.world.can_see_unit(self.local_player as u8, i)
            {
                continue;
            }
            let (ux, uy) = self.screen_of(u);
            let ty = self.data.unit(u.type_id);
            // units.dat box: left/up/right/down are distances from the
            // centre (the old `(right - left) / 2` was ~0 for symmetric
            // units, so only a 10 px circle was clickable and a building
            // could not be clicked away from its exact centre).  Small units
            // keep at least a 10 px reach.
            let d = &ty.dimensions;
            let (l, t, r, b) = (
                (d.left as i32).max(10),
                (d.top as i32).max(10),
                (d.right as i32).max(10),
                (d.bottom as i32).max(10),
            );
            let dx = sx - ux;
            let dy = sy - uy;
            if dx >= -l && dx <= r && dy >= -t && dy <= b {
                let score = dy.abs() - uy / 1000;
                if best.is_none_or(|(b, _)| score < b) {
                    best = Some((score, u.id));
                }
            }
        }
        if best.is_none() && std::env::var("SC_VIEW_DEBUG").is_ok() {
            eprintln!("unit_at({}, {}) miss; camera={:?}", sx, sy, self.camera);
            for u in self
                .world
                .units
                .iter()
                .filter(|u| u.alive && u.owner as usize == self.local_player)
            {
                if u.is_resource() {
                    continue;
                }
                let (ux, uy) = self.screen_of(u);
                eprintln!(
                    "   {} {} at screen {} {} hp {}",
                    u.id,
                    self.data.unit_name(u.type_id),
                    ux,
                    uy,
                    u.hp
                );
            }
        }
        best.map(|(_, id)| id)
    }

    pub(super) fn selected_units(&self) -> Vec<&SimUnit> {
        self.selected
            .iter()
            .filter_map(|id| self.world.unit(*id))
            .filter(|u| u.alive)
            .collect()
    }

    pub(super) fn first_selected(&self) -> Option<&SimUnit> {
        self.selected_units().into_iter().next()
    }

    pub(super) fn refresh_commands(&mut self) {
        let owner = self.first_selected().map(|u| u.id);
        if owner != self.card_owner {
            self.card_owner = owner;
            self.card_set = None;
            self.target_act = None;
            self.build_mode = None;
            self.pad_wheel.close();
            self.pad_wheel_pages.clear();
        }
        self.command_list.clear();
        self.build_list.clear();
        let Some(u) = self.first_selected() else {
            return;
        };
        if u.owner as usize != self.local_player {
            return;
        }
        let type_id = u.type_id;
        let is_worker = self.data.unit(type_id).is_worker();
        let is_building = self.data.unit(type_id).is_building() || type_id == uid::LARVA;
        if is_worker {
            for t in 0..228u16 {
                if let Some(info) = tree::build_info(t)
                    && info.producer == type_id
                {
                    self.build_list.push(t);
                }
            }
            self.command_list = self.build_list.clone();
        } else if is_building {
            for t in 0..228u16 {
                if let Some(info) = tree::build_info(t)
                    && info.producer == type_id
                {
                    self.command_list.push(t);
                }
            }
        }
    }

    /// The nine card buttons of the race dialog (`statbtn{t,z,p}`), right
    /// anchored like retail.
    pub(super) fn command_slots(&self) -> Vec<UiRect> {
        let dialog = self.card_dialog();
        (0..9)
            .map(|i| {
                let fallback = UiRect::new(496 + 9 + (i % 3) * 46, 354 + 4 + (i / 3) * 40, 36, 34);
                self.hud_control(dialog, i + 1, fallback)
            })
            .collect()
    }

    /// Activate command-card slot `index` (0..8).  This is shared by the
    /// number keys and mouse clicks on the retail button rectangles.
    /// Card on screen: the retail button set of the first selected own unit
    /// (or the build / target card a button switched to).
    pub(super) fn current_card(&self) -> Option<(SimUnit, Card)> {
        let u = self.first_selected()?.clone();
        if u.owner as usize != self.local_player {
            return None;
        }
        // While a target or a building spot is pending, retail shows the
        // Esc-only card (229).
        let set = if self.target_act.is_some() || self.build_mode.is_some() {
            Some(TARGET_CARD)
        } else if self.data.unit(u.type_id).is_building() && !u.completed && u.morph_to.is_none() {
            // A structure under construction shows only Cancel (set 231,
            // whose single button is the 0x74d440 construction cancel).
            Some(CONSTRUCTION_CARD)
        } else {
            self.card_set
        };
        let card = galaxycraft::buttons::card_of_set(&self.world, &u, set);
        Some((u, card))
    }

    /// Hotkey of a button: the first character of its `stat_txt` tooltip
    /// (retail stores "m<ctrl>Move", "bBuild Barracks", ...).
    pub(super) fn button_hotkey(&self, b: &galaxycraft::button_sets::Button) -> Option<char> {
        let text = self.data.strings.get(b.enabled_string.checked_sub(1)?)?;
        let c = text.chars().next()?;
        // `\x1b` = Esc (the Cancel buttons).
        (c.is_ascii_alphanumeric() || c == '\x1b').then(|| c.to_ascii_uppercase())
    }

    /// Selected own units that take movement orders (not grounded
    /// buildings: an order overwrote a Command Center's training).
    pub(super) fn selected_mobile(&self) -> Vec<u32> {
        self.selected
            .iter()
            .copied()
            .filter(|id| {
                self.world.unit(*id).is_some_and(|u| {
                    u.owner as usize == self.local_player
                        && (!self.data.unit(u.type_id).is_building() || u.lifted)
                })
            })
            .collect()
    }

    /// Selected own transports and Bunkers (the units whose card has the
    /// Load / Unload actions).
    pub(super) fn selected_transports(&self) -> Vec<u32> {
        self.selected
            .iter()
            .copied()
            .filter(|id| {
                self.world.unit(*id).is_some_and(|u| {
                    u.owner as usize == self.local_player
                        && u.alive
                        && u.completed
                        && self.data.unit(u.type_id).space_provided > 0
                })
            })
            .collect()
    }

    pub(super) fn selected_own_buildings(&self) -> Vec<u32> {
        self.selected
            .iter()
            .copied()
            .filter(|id| {
                self.world.unit(*id).is_some_and(|u| {
                    u.owner as usize == self.local_player && self.data.unit(u.type_id).is_building()
                })
            })
            .collect()
    }

    /// Run command-card slot `index` (0..8): mouse clicks and hotkeys.
    pub(super) fn activate_command(&mut self, index: usize) {
        // The button shows pressed for a moment (user report 2026-11-07).
        self.card_pressed = Some((index, Instant::now()));
        use galaxycraft::buttons::{Act, ButtonState};
        let Some((u, card)) = self.current_card() else {
            return;
        };
        let Some((b, state)) = card.get(index).copied().flatten() else {
            return;
        };
        if state == ButtonState::Disabled {
            // Requirements unmet: retail shows the disabled tooltip only.
            return;
        }
        self.actions = self.actions.saturating_add(1);
        let var = b.act_var;
        match Act::of(b.act) {
            act @ (Act::TargetTech
            | Act::Move
            | Act::Attack
            | Act::Patrol
            | Act::Repair
            | Act::Gather
            | Act::Rally
            | Act::Land
            | Act::Nuke
            | Act::Heal
            | Act::Infest
            | Act::RechargeShields
            | Act::PlaceBeacon) => {
                self.target_act = Some((act, var));
            }
            Act::Stop => {
                for id in self.selected_mobile() {
                    self.send_order(id, Order::Stop);
                }
                self.play_ack();
            }
            Act::Hold => {
                for id in self.selected_mobile() {
                    self.send_order(id, Order::HoldPosition);
                }
                self.play_ack();
            }
            Act::ReturnCargo => {
                for id in self.selected_mobile() {
                    self.send_order(id, Order::ReturnCargo { depot: 0 });
                }
                self.play_ack();
            }
            Act::Card => {
                self.card_set = (var != 228).then_some(var);
            }
            Act::Build | Act::DroneBuild => {
                self.build_mode = Some(var);
            }
            Act::Train => self.train(var),
            Act::MorphUnit => self.morph_unit_command(var),
            Act::MorphBuilding => {
                for id in self.selected_own_buildings() {
                    if self.net.is_some() {
                        self.queue_act(galaxycraft::replay::RecAct::MorphBuilding {
                            source: id,
                            unit: var,
                        });
                    } else if let Err(e) = self.world.morph_building(id, var) {
                        self.show_sim_error(e);
                    }
                }
            }
            Act::SpawnLarva => {
                for id in self.selected_own_buildings() {
                    if self.net.is_some() {
                        self.queue_act(galaxycraft::replay::RecAct::SpawnLarva { hatchery: id });
                    } else if let Err(e) = self.world.spawn_larva(id) {
                        self.show_sim_error(e);
                    }
                }
            }
            Act::TrainHangar => {
                for id in self.selected_mobile() {
                    if self.net.is_some() {
                        self.queue_act(galaxycraft::replay::RecAct::Hangar { unit: id });
                    } else if let Err(e) = self.world.build_hangar_unit(id) {
                        self.show_sim_error(e);
                    }
                }
            }
            Act::Cloak | Act::Decloak => {
                let cloaked = matches!(Act::of(b.act), Act::Cloak);
                for id in self.selected_mobile() {
                    if self.net.is_some() {
                        self.queue_act(galaxycraft::replay::RecAct::Cloak { unit: id, cloaked });
                    } else if let Err(e) = self.world.set_cloak(id, cloaked) {
                        self.show_sim_error(e);
                    }
                }
            }
            Act::StopOrMerge => {
                if var as u8 == galaxycraft::ids::tech::ARCHON_WARP {
                    self.merge_templars(false);
                } else if var as u8 == galaxycraft::ids::tech::DARK_ARCHON_MELD {
                    self.merge_templars(true);
                } else {
                    for id in self.selected_mobile() {
                        self.send_order(id, Order::Stop);
                    }
                    self.play_ack();
                }
            }
            Act::MergeArchon => self.merge_templars(false),
            Act::MergeDarkArchon => self.merge_templars(true),
            Act::Load => self.load_transports(),
            Act::Unload => {
                // A Bunker empties at once; a transport asks where (click
                // the transport itself to unload on the spot), as retail.
                let flying = self.selected_transports().into_iter().any(|id| {
                    self.world
                        .unit(id)
                        .is_some_and(|u| !self.data.unit(u.type_id).is_building())
                });
                if flying {
                    self.target_act = Some((Act::Unload, var));
                } else {
                    for id in self.selected_transports() {
                        self.send_order(id, Order::Unload);
                    }
                    self.play_ack();
                }
            }
            Act::Addon => {
                if let Err(e) = self.world.build_addon(u.id, var) {
                    self.show_sim_error(e);
                }
            }
            Act::StimPacks => {
                for id in self.selected_mobile() {
                    // In a network game the cast goes through the lockstep
                    // queue like every other act.
                    if self.net.is_some() {
                        self.queue_act(galaxycraft::replay::RecAct::Cast {
                            caster: id,
                            spell: galaxycraft::sim::Spell::StimPacks,
                            target: galaxycraft::sim::SpellTarget::None,
                        });
                    } else {
                        let _ = self.world.cast(
                            id,
                            galaxycraft::sim::Spell::StimPacks,
                            galaxycraft::sim::SpellTarget::None,
                        );
                    }
                }
            }
            Act::Research => {
                if let Err(e) = self.world.research(u.id, var as u8) {
                    self.show_sim_error(e);
                }
            }
            Act::Upgrade => {
                if let Err(e) = self.world.upgrade(u.id, var as u8) {
                    self.show_sim_error(e);
                }
            }
            Act::LiftOff => {
                for id in self.selected_own_buildings() {
                    self.send_order(id, Order::Lift);
                }
            }
            Act::CancelConstruction => {
                if let Err(e) = self.world.cancel_construction(u.id) {
                    self.show_sim_error(e);
                }
            }
            Act::CancelLast => {
                if let Err(e) = self.world.cancel_production(u.id) {
                    self.show_sim_error(e);
                }
            }
            Act::CancelAddon | Act::CancelMorph => {
                // The shipped rows use several condition/action pairs for
                // "cancel"; try the state-specific cancel that applies.
                for id in self.selected.clone() {
                    if self.world.unit(id).is_some_and(|u| {
                        u.owner as usize == self.local_player
                            && self.data.unit(u.type_id).is_building()
                    }) {
                        self.cancel_building(id);
                    }
                }
            }
            Act::CancelTarget => {
                self.target_act = None;
                self.build_mode = None;
            }
            Act::Siege => {
                for id in self.selected_mobile() {
                    self.send_order(id, Order::Siege);
                }
            }
            Act::Unsiege => {
                for id in self.selected_mobile() {
                    self.send_order(id, Order::Unsiege);
                }
            }
            Act::Burrow => {
                for id in self.selected_mobile() {
                    self.send_order(id, Order::Burrow);
                }
            }
            Act::Unburrow => {
                for id in self.selected_mobile() {
                    self.send_order(id, Order::Unburrow);
                }
            }
            other => {
                self.status(format!("{other:?}: not implemented yet"));
            }
        }
        self.refresh_commands();
    }

    /// A targeted spell from the card (`Act::TargetTech`, tech `act_var`):
    /// the first selected own unit that can cast it does (retail sends one
    /// caster), at the clicked unit when there is one, else at the point.
    pub(super) fn cast_targeted(&mut self, tech: u16, pos: (i32, i32), target: Option<u32>) {
        use galaxycraft::sim::{Spell, SpellTarget};
        let Some(spell) = Spell::from_tech(tech as u8) else {
            self.status(format!("tech {tech}: no spell"));
            return;
        };
        // Point spells (Spider Mines, Scanner Sweep, ...) take the clicked
        // spot; unit spells the clicked unit.
        let point_spell = matches!(
            spell,
            Spell::SpiderMines
                | Spell::ScannerSweep
                | Spell::DarkSwarm
                | Spell::PsionicStorm
                | Spell::Recall
                | Spell::StasisField
                | Spell::DisruptionWeb
                | Spell::Plague
                | Spell::Ensnare
                | Spell::EmpShockwave
                | Spell::Maelstrom
        );
        let t = match target {
            Some(id) if !point_spell => SpellTarget::Unit(id),
            _ => SpellTarget::Point(pos.0, pos.1),
        };
        // One caster: the first selected own unit whose card has it (an
        // `Order::Cast` walks it into range; the order is the logged act).
        let caster = self.selected.iter().copied().find(|id| {
            self.world.unit(*id).is_some_and(|u| {
                u.owner as usize == self.local_player
                    && galaxycraft::buttons::card(&self.world, u)
                        .iter()
                        .flatten()
                        .any(|(b, _)| {
                            galaxycraft::buttons::Act::of(b.act)
                                == galaxycraft::buttons::Act::TargetTech
                                && b.act_var == tech
                        })
            })
        });
        match caster {
            Some(id) => {
                self.send_order(id, Order::Cast { spell, target: t });
                self.play_ack();
            }
            None => self.status("No unit can cast that"),
        }
    }

    /// Left click in the world while a card button waits for a target.
    pub(super) fn execute_target(&mut self, px: i32, py: i32) {
        let (lx, ly) = self.to_logical(px, py);
        let pos = self.world_of(lx, ly);
        let target = self.unit_at(px, py);
        self.execute_target_world(pos, target);
    }

    /// [`Self::execute_target`] at a world position (minimap clicks).
    pub(super) fn execute_target_world(&mut self, pos: (i32, i32), target: Option<u32>) {
        use galaxycraft::buttons::Act;
        let Some((act, var)) = self.target_act.take() else {
            return;
        };
        let mobile = self.selected_mobile();
        if act == Act::TargetTech {
            self.cast_targeted(var, pos, target);
            self.refresh_commands();
            return;
        }
        match act {
            Act::Move => match target {
                Some(t) => {
                    let p = self
                        .world
                        .unit(t)
                        .map(|u| (u.x / 256, u.y / 256))
                        .unwrap_or(pos);
                    self.world.command_move(&mobile, p);
                }
                None => self.world.command_move(&mobile, pos),
            },
            Act::Attack => match target {
                Some(t) => {
                    for id in mobile {
                        self.send_order(id, Order::AttackUnit { target: t });
                    }
                }
                None => self.world.command_attack_move(&mobile, pos),
            },
            Act::Patrol => {
                for id in mobile {
                    if let Some(u) = self.world.unit(id) {
                        let a = (u.x / 256, u.y / 256);
                        self.send_order(
                            id,
                            Order::Patrol {
                                a,
                                b: pos,
                                forward: true,
                            },
                        );
                    }
                }
            }
            Act::Gather => {
                if let Some(t) = target {
                    for id in mobile {
                        self.send_order(id, Order::Gather { resource: t });
                    }
                }
            }
            Act::Repair => {
                if let Some(t) = target {
                    for id in mobile {
                        self.send_order(id, Order::Repair { target: t });
                    }
                }
            }
            Act::Rally => {
                for id in self.selected_own_buildings() {
                    let _ = self.world.set_rally(id, pos);
                }
            }
            Act::Land => {
                for id in self.selected_own_buildings() {
                    self.send_order(id, Order::Land { pos });
                }
            }
            Act::Nuke => {
                let ghosts: Vec<u32> = self
                    .selected_mobile()
                    .into_iter()
                    .filter(|id| {
                        self.world.unit(*id).is_some_and(|u| {
                            u.type_id == galaxycraft::ids::unit::GHOST
                                && u.owner as usize == self.local_player
                        })
                    })
                    .collect();
                for id in ghosts {
                    if self.net.is_some() {
                        self.queue_act(galaxycraft::replay::RecAct::Nuke { ghost: id, pos });
                    } else if let Err(e) = self.world.launch_nuke(id, pos) {
                        self.show_sim_error(e);
                    }
                }
            }
            Act::Unload => {
                for id in self.selected_transports() {
                    let Some(u) = self.world.unit(id) else {
                        continue;
                    };
                    let here = target == Some(id)
                        || self.data.unit(u.type_id).is_building()
                        || ((u.x / 256 - pos.0).abs() <= 16 && (u.y / 256 - pos.1).abs() <= 16);
                    let order = if here {
                        Order::Unload
                    } else {
                        Order::MoveUnload { pos }
                    };
                    self.send_order(id, order);
                }
                self.play_ack();
            }
            Act::Heal => {
                if let Some(t) = target {
                    for id in self.selected_mobile() {
                        if self
                            .world
                            .unit(id)
                            .is_some_and(|u| u.type_id == galaxycraft::ids::unit::MEDIC)
                        {
                            self.send_order(
                                id,
                                Order::Cast {
                                    spell: galaxycraft::sim::Spell::Healing,
                                    target: galaxycraft::sim::SpellTarget::Unit(t),
                                },
                            );
                        }
                    }
                }
            }
            Act::Infest => {
                if let Some(t) = target {
                    for id in self.selected_mobile() {
                        if self
                            .world
                            .unit(id)
                            .is_some_and(|u| u.type_id == galaxycraft::ids::unit::QUEEN)
                        {
                            self.send_order(
                                id,
                                Order::Cast {
                                    spell: galaxycraft::sim::Spell::Infestation,
                                    target: galaxycraft::sim::SpellTarget::Unit(t),
                                },
                            );
                        }
                    }
                }
            }
            Act::PlaceBeacon => {
                // Capture the Flag: move the selected own beacon once
                // during the first ten minutes.
                if self.net.is_some() {
                    self.queue_act(galaxycraft::replay::RecAct::Beacon {
                        player: self.local_player as u8,
                        pos,
                    });
                } else if let Err(e) = self.world.move_beacon(self.local_player as u8, pos) {
                    self.show_sim_error(e);
                }
            }
            Act::RechargeShields => {
                if let Some(t) = target {
                    let batteries: Vec<u32> = self
                        .selected_own_buildings()
                        .into_iter()
                        .filter(|id| {
                            self.world.unit(*id).is_some_and(|u| {
                                u.type_id == galaxycraft::ids::unit::SHIELD_BATTERY
                            })
                        })
                        .collect();
                    for id in batteries {
                        if self.net.is_some() {
                            self.queue_act(galaxycraft::replay::RecAct::ShieldBattery {
                                battery: id,
                                target: t,
                            });
                        } else if let Err(e) = self.world.shield_battery(id, t) {
                            self.show_sim_error(e);
                        }
                    }
                }
            }
            _ => {}
        }
        self.play_ack();
        self.refresh_commands();
    }

    /// Right click in the world (retail smart command): enemies are
    /// attacked, workers gather from minerals and refineries, other targets
    /// and empty ground are moved to.  Grounded buildings take no orders;
    /// producing buildings set their rally point instead.
    pub(super) fn command_at(
        &mut self,
        sx: i32,
        sy: i32,
        attack_move: bool,
        patrol: bool,
        queue: bool,
    ) {
        let (lx, ly) = self.to_logical(sx, sy);
        let pos = self.world_of(lx, ly);
        let target = self.unit_at(sx, sy);
        self.command_world(pos, target, attack_move, patrol, queue);
    }

    /// [`Self::command_at`] at a world position (also the minimap's right
    /// click, which has no unit target).
    pub(super) fn command_world(
        &mut self,
        (wx, wy): (i32, i32),
        target: Option<u32>,
        attack_move: bool,
        patrol: bool,
        queue: bool,
    ) {
        if self.selected.is_empty() {
            return;
        }
        self.actions = self.actions.saturating_add(1);
        let me = self.local_player as u8;
        let mobile = self.selected_mobile();
        for id in self.selected_own_buildings() {
            if self.world.unit(id).is_some_and(|u| !u.lifted) {
                let _ = self.world.set_rally(id, (wx, wy));
            }
        }
        if mobile.is_empty() {
            return;
        }
        let target = target.and_then(|id| self.world.unit(id)).map(|t| {
            (
                t.id,
                t.owner,
                t.is_resource() || galaxycraft::sim::is_gas_building(t.type_id),
                (t.x / 256, t.y / 256),
            )
        });
        // Retail order feedback: the target's circle blinks (yellow on a
        // mineral field, user report 2026-11-07), a ground order marks the
        // spot.
        match target {
            Some((tid, ..)) => {
                self.order_flash = Some((tid, Instant::now()));
                self.order_marker = None;
            }
            None => {
                self.order_marker = Some(((wx, wy), Instant::now()));
                self.order_flash = None;
            }
        }
        if let Some((tid, towner, resource, tpos)) = target {
            if towner != me && self.world.hostile(me, towner) && !resource {
                for id in mobile {
                    self.send_order_or_queue(id, Order::AttackUnit { target: tid }, queue);
                }
                self.play_ack();
                return;
            }
            // Retail smart command on an own Bunker / transport with room:
            // the units that fit go in (Desperate Alliance recording 272 s:
            // four Marines inside a Bunker).
            if towner == me
                && let Some(t) = self.world.unit(tid).cloned()
                && t.completed
                && self.data.unit(t.type_id).space_provided > 0
            {
                let riders: Vec<u32> = mobile
                    .iter()
                    .copied()
                    .filter(|id| *id != tid)
                    .filter(|id| {
                        self.world.unit(*id).is_some_and(|u| {
                            let ty = self.data.unit(u.type_id);
                            ty.space_required > 0 && ty.space_required < 8 && !ty.is_building()
                        })
                    })
                    .collect();
                if !riders.is_empty() {
                    for id in riders {
                        self.send_order_or_queue(
                            id,
                            Order::EnterTransport { transport: tid },
                            queue,
                        );
                    }
                    self.play_ack();
                    return;
                }
            }
            // An SCV resumes the construction of an own unfinished building.
            if towner == me
                && self
                    .world
                    .unit(tid)
                    .is_some_and(|t| !t.completed && t.builder != 0)
            {
                let scvs: Vec<u32> = mobile
                    .iter()
                    .copied()
                    .filter(|id| {
                        self.world
                            .unit(*id)
                            .is_some_and(|u| u.type_id == galaxycraft::ids::unit::SCV)
                    })
                    .collect();
                if let Some(&scv) = scvs.first() {
                    self.send_order_or_queue(scv, Order::Construct { building: tid }, queue);
                    self.play_ack();
                    return;
                }
            }
            if resource {
                let (workers, others): (Vec<u32>, Vec<u32>) = mobile.iter().partition(|id| {
                    self.world
                        .unit(**id)
                        .is_some_and(|u| self.data.unit(u.type_id).is_worker())
                });
                for id in workers {
                    self.send_order_or_queue(id, Order::Gather { resource: tid }, queue);
                }
                if !others.is_empty() {
                    if queue {
                        for id in others {
                            self.send_order_or_queue(id, Order::Move { pos: tpos }, true);
                        }
                    } else {
                        self.world.command_move(&others, tpos);
                    }
                }
                self.play_ack();
                return;
            }
            if queue {
                for id in mobile {
                    self.send_order_or_queue(id, Order::Move { pos: tpos }, true);
                }
            } else {
                self.world.command_move(&mobile, tpos);
            }
            self.play_ack();
            return;
        }
        if patrol {
            for id in mobile {
                if let Some(u) = self.world.unit(id) {
                    let a = (u.x / 256, u.y / 256);
                    self.send_order_or_queue(
                        id,
                        Order::Patrol {
                            a,
                            b: (wx, wy),
                            forward: true,
                        },
                        queue,
                    );
                }
            }
        } else if attack_move {
            if queue {
                for id in mobile {
                    self.send_order_or_queue(id, Order::AttackMove { pos: (wx, wy) }, true);
                }
            } else {
                self.world.command_attack_move(&mobile, (wx, wy));
            }
        } else if queue {
            for id in mobile {
                self.send_order_or_queue(id, Order::Move { pos: (wx, wy) }, true);
            }
        } else {
            self.world.command_move(&mobile, (wx, wy));
        }
        self.refresh_commands();
        self.play_ack();
    }

    pub(super) fn train(&mut self, unit_type: u16) {
        let mut trained = false;
        for id in self.selected.clone() {
            if let Some(u) = self.world.unit(id)
                && u.owner as usize == self.local_player
                && self.data.unit(u.type_id).is_building()
            {
                if self.net.is_some() {
                    if self.send_train(id, unit_type) {
                        trained = true;
                    }
                } else {
                    match self.world.train(id, unit_type) {
                        Ok(()) => trained = true,
                        Err(e) => self.show_sim_error(e),
                    }
                }
            }
        }
        if trained {
            self.actions = self.actions.saturating_add(1);
            self.play("ui_click");
            self.status(format!("Training {}", self.data.unit_name(unit_type)));
        }
    }

    /// The Morph Unit button: larva → unit, Hydralisk → Lurker,
    /// Mutalisk → Guardian/Devourer.  (Those are the only morph rows with
    /// this action in the shipped button table.)
    pub(super) fn morph_unit_command(&mut self, unit_type: u16) {
        for id in self.selected.clone() {
            let Some(u) = self.world.unit(id).cloned() else {
                continue;
            };
            if u.owner as usize != self.local_player || !u.alive || u.in_transport.is_some() {
                continue;
            }
            if u.type_id == galaxycraft::ids::unit::LARVA {
                if self.net.is_some() {
                    self.queue_act(galaxycraft::replay::RecAct::Morph {
                        larva: id,
                        unit: unit_type,
                    });
                } else if let Err(e) = self.world.morph(id, unit_type) {
                    self.show_sim_error(e);
                }
            } else if matches!(
                u.type_id,
                galaxycraft::ids::unit::HYDRALISK | galaxycraft::ids::unit::MUTALISK
            ) {
                if self.net.is_some() {
                    self.queue_act(galaxycraft::replay::RecAct::MorphUnit {
                        unit: id,
                        unit_type,
                    });
                } else if let Err(e) = self.world.morph_unit(id, unit_type) {
                    self.show_sim_error(e);
                }
            }
        }
    }

    /// Archon Warp / Dark Archon Meld: one cast merges the pair (the
    /// simulation removes the two templars and spawns the archon).
    pub(super) fn merge_templars(&mut self, dark: bool) {
        use galaxycraft::ids::unit as uid;
        use galaxycraft::sim::{Spell, SpellTarget};
        let wanted = if dark {
            uid::DARK_TEMPLAR
        } else {
            uid::HIGH_TEMPLAR
        };
        let selected: Vec<u32> = self
            .selected_mobile()
            .into_iter()
            .filter(|id| self.world.unit(*id).is_some_and(|u| u.type_id == wanted))
            .collect();
        if selected.len() < 2 {
            self.status("Select 2 or more Templars");
            return;
        }
        let spell = if dark {
            Spell::DarkArchonMeld
        } else {
            Spell::ArchonWarp
        };
        self.send_order(
            selected[0],
            Order::Cast {
                spell,
                target: SpellTarget::None,
            },
        );
        self.play_ack();
    }

    /// The transport / Bunker Load button: nearby own units board the
    /// selected transport.  Retail loads the units around the transport,
    /// not a clicked target.
    pub(super) fn load_transports(&mut self) {
        let transports = self.selected_transports();
        for transport in transports {
            let Some(t) = self.world.unit(transport).cloned() else {
                continue;
            };
            let ty = self.data.unit(t.type_id);
            let used: i32 = t
                .cargo
                .iter()
                .filter_map(|id| self.world.unit(*id))
                .map(|u| self.data.unit(u.type_id).space_required as i32)
                .sum();
            let mut free = ty.space_provided as i32 - used;
            if free <= 0 {
                continue;
            }
            let riders: Vec<u32> = self
                .world
                .units
                .iter()
                .filter(|u| {
                    u.alive
                        && u.owner as usize == self.local_player
                        && u.id != transport
                        && !u.in_transport.is_some()
                        && !self.data.unit(u.type_id).is_building()
                        && self.data.unit(u.type_id).space_required > 0
                        && (u.x - t.x).abs() < 8 * 256
                        && (u.y - t.y).abs() < 8 * 256
                })
                .map(|u| u.id)
                .collect();
            for id in riders {
                let need = self
                    .world
                    .unit(id)
                    .map(|u| self.data.unit(u.type_id).space_required as i32)
                    .unwrap_or(0);
                if need > free {
                    continue;
                }
                self.send_order(id, Order::EnterTransport { transport });
                free -= need;
                if free <= 0 {
                    break;
                }
            }
        }
        self.play_ack();
    }

    /// Cancel whatever the selected building is currently doing: a morph, an
    /// add-on, a construction site or a production queue (the shipped
    /// Cancel rows share one card slot).
    pub(super) fn cancel_building(&mut self, id: u32) {
        if self.world.cancel_morph(id).is_ok()
            || self.world.cancel_addon(id).is_ok()
            || self.world.cancel_construction(id).is_ok()
        {
            return;
        }
        if let Err(e) = self.world.cancel_production(id) {
            self.show_sim_error(e);
        }
    }

    pub(super) fn place_building(&mut self, sx: i32, sy: i32) {
        let Some(unit_type) = self.build_mode else {
            return;
        };
        // Mouse is in window pixels; the world view is logical (scaled).
        let (lx, ly) = self.to_logical(sx, sy);
        let (wx, wy) = self.world_of(lx, ly);
        let Some(spot) = self.world.placement_spot(unit_type, (wx, wy)) else {
            self.show_game_error(STAT_CANT_BUILD_THERE);
            return;
        };
        let mut placed = false;
        for id in self.selected.clone() {
            if self
                .world
                .unit(id)
                .is_some_and(|u| self.data.unit(u.type_id).is_worker())
            {
                match self.world.issue(
                    id,
                    Order::Build {
                        unit_type,
                        pos: spot,
                    },
                ) {
                    Ok(()) => placed = true,
                    Err(e) => self.show_sim_error(e),
                }
            }
        }
        if placed {
            self.actions = self.actions.saturating_add(1);
            self.play("ui_click");
            self.build_mode = None;
            // Retail returns to the unit's own card once the spot is given.
            self.card_set = None;
        }
    }

    /// B with a production building selected: refund and drop the newest
    /// queued item (the current item keeps building).
    pub(super) fn dequeue_last_production(&mut self) -> bool {
        let mut removed = false;
        for id in self.selected_own_buildings() {
            let queued = self
                .world
                .unit(id)
                .is_some_and(|u| !u.production.is_empty());
            if !queued {
                continue;
            }
            if self.net.is_some() {
                self.queue_act(galaxycraft::replay::RecAct::Dequeue { building: id });
                removed = true;
            } else {
                match self.world.dequeue_production(id) {
                    Ok(true) => removed = true,
                    Ok(false) => {}
                    Err(e) => self.show_sim_error(e),
                }
            }
        }
        if removed {
            self.status("Removed the last queued item");
            self.play("ui_click");
            self.refresh_commands();
        }
        removed
    }
}
