//! `impl World`: orders (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

impl World {
    pub fn issue(&mut self, unit_id: u32, new_order: Order) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive {
            return Err(SimError::BadUnit);
        }
        // A tank in the middle of its siege / unsiege animation takes no
        // orders (retail `status_flag_order_not_interruptible`).
        if self.is_transforming(i) {
            return Err(SimError::ProducerBusy);
        }
        // A new order drops whatever the old one was counting (a Yamato
        // charge, an unload in progress).
        self.units[i].order_wait = 0;
        self.cancel_nuke_for(unit_id);
        match &new_order {
            Order::AttackUnit { target } => {
                if self.unit(*target).is_none() {
                    return Err(SimError::InvalidTarget);
                }
            }
            Order::Gather { resource } => {
                let target = self.unit(*resource).ok_or(SimError::InvalidTarget)?;
                if !target.is_resource() && !is_gas_building(target.type_id) {
                    return Err(SimError::InvalidTarget);
                }
            }
            _ => {}
        }
        if matches!(new_order, Order::AttackUnit { .. }) && self.units[i].type_id == uid::CARRIER {
            // Carrier Attack row 20 starts with a 60-tick launch phase
            // (`CUnit+0x55 = 0x3c`); the Pulse Cannon cycle follows it.
            self.units[i].special_cooldown = 0x3c;
        }
        self.units[i].order = new_order;
        self.units[i].order_row = self.units[i].order.order_id();
        self.units[i].resume_order = None;
        self.units[i].target = None;
        self.units[i].path.clear();
        self.units[i].path_goal = None;
        if !matches!(self.units[i].order, Order::Idle | Order::HoldPosition) {
            self.units[i].queue.clear();
        }
        // Record successful commands for save games / replays / LAN sync.
        if self.recorder.is_some() {
            let tick = self.tick;
            let logged = self.units[i].order.clone();
            if let Some(r) = self.recorder.as_mut() {
                r.cmds.push(crate::replay::RecCmd {
                    tick,
                    act: crate::replay::RecAct::Issue {
                        unit: unit_id,
                        order: logged,
                    },
                });
            }
        }
        Ok(())
    }

    /// Issue any of the 189 `orders.dat` rows by id.  This is the front
    /// door to the recovered handler table: every row with a direct case in
    /// `FUN_7a178340` calls its own transcribed Rust routine in
    /// [`crate::handlers`], and rows resolved by an alias, a production timer
    /// or the engine fall back to the typed state machine.
    pub fn issue_order_id(
        &mut self,
        unit_id: u32,
        order_id: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        crate::handlers::dispatch_row(self, unit_id, order_id, target, pos)
    }

    /// Apply a row through the typed order state machine.  This is the body
    /// called by the transcribed handler routines and by rows that are
    /// resolved before the recovered switch.
    pub(crate) fn apply_order_row_typed(
        &mut self,
        unit_id: u32,
        order_id: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        let result = self.apply_order_row_typed_inner(unit_id, order_id, target, pos);
        if result.is_ok()
            && let Some(i) = self.index_of(unit_id)
        {
            self.units[i].order_row = order_id;
        }
        result
    }

    pub(super) fn apply_order_row_typed_inner(
        &mut self,
        unit_id: u32,
        order_id: u8,
        target: Option<u32>,
        pos: Option<(i32, i32)>,
    ) -> Result<(), SimError> {
        // Internal-but-player-visible rows with a concrete API.
        match order_id {
            // Critters / Junk Yard Dog wander around their current point.
            154 | 155 | 178 => {
                let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
                let a = (self.units[i].x / fp(1), self.units[i].y / fp(1));
                let dx = (self.world_rand() % 128) as i32 - 64;
                let dy = (self.world_rand() % 128) as i32 - 64;
                return self.issue(
                    unit_id,
                    Order::Patrol {
                        a,
                        b: (a.0 + dx, a.1 + dy),
                        forward: true,
                    },
                );
            }
            // AI harass uses attack-move; guard/tower/passive rows hold.
            157 => {
                let p = pos.ok_or(SimError::Unsupported)?;
                return self.issue(unit_id, Order::AttackMove { pos: p });
            }
            156 | 159 | 160 | 161 | 163 | 166 | 61 => {
                return self.issue(unit_id, Order::HoldPosition);
            }
            // Computer Return heads for the owner's town hall.
            162 => {
                let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
                let owner = self.units[i].owner;
                let home = self
                    .units
                    .iter()
                    .find(|u| {
                        u.alive
                            && u.completed
                            && u.owner == owner
                            && matches!(
                                u.type_id,
                                uid::COMMAND_CENTER
                                    | uid::NEXUS
                                    | uid::HATCHERY
                                    | uid::LAIR
                                    | uid::HIVE
                            )
                    })
                    .map(|u| (u.x / fp(1), u.y / fp(1)));
                return match home {
                    Some(p) => self.issue(unit_id, Order::Move { pos: p }),
                    None => self.issue(unit_id, Order::Stop),
                };
            }
            // Self-destructing rows (Scourge-style campaign units).
            164 | 174 => {
                let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
                let hp = self.units[i].hp;
                self.damage_unit(unit_id, hp)?;
                return Ok(());
            }
            // Reaver/Scarab combat rows.
            56 | 58 | 59 | 60 | 64 | 65 => {
                if let Some(t) = target {
                    return self.issue(unit_id, Order::AttackUnit { target: t });
                }
                let p = pos.ok_or(SimError::Unsupported)?;
                return self.issue(unit_id, Order::AttackMove { pos: p });
            }
            18 | 19 | 131 => {
                let p = pos.ok_or(SimError::Unsupported)?;
                self.place_spider_mine(unit_id, p)?;
                return Ok(());
            }
            49 | 62 => {
                self.build_hangar_unit(unit_id)?;
                return Ok(());
            }
            51 | 52 | 55 => return self.issue(unit_id, Order::Stop),
            68 => {
                let p = pos.ok_or(SimError::Unsupported)?;
                let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
                self.units[i].rally = Some(p);
                return Ok(());
            }
            104 | 105 => {
                return self.cast(unit_id, Spell::ArchonWarp, SpellTarget::None);
            }
            183 => return self.cast(unit_id, Spell::DarkArchonMeld, SpellTarget::None),
            138 => {
                let target = pos
                    .map(|(x, y)| SpellTarget::Point(x, y))
                    .unwrap_or(SpellTarget::None);
                return self.cast(unit_id, Spell::ScannerSweep, target);
            }
            _ => {}
        }
        // Rows whose behaviour is a direct ability call rather than a typed
        // order state.  These are the shipped cast/cloak/shield rows.
        if let Some(spell) = row_spell(order_id) {
            let target = match target {
                Some(t) => SpellTarget::Unit(t),
                None => match pos {
                    Some((x, y)) => SpellTarget::Point(x, y),
                    None => SpellTarget::None,
                },
            };
            // Parameter rows must fail loudly when the replay/AI omitted
            // their target instead of falling through to a no-op cast.
            if spell.takes_unit_target() && !matches!(target, SpellTarget::Unit(_)) {
                return Err(SimError::Unsupported);
            }
            return self.cast(unit_id, spell, target);
        }
        match order_id {
            57 | 107 | 108 | 136 => return self.set_cloak(unit_id, true),
            109 => return self.set_cloak(unit_id, false),
            66 | 67 => {
                let t = target.ok_or(SimError::Unsupported)?;
                self.shield_battery(unit_id, t)?;
                return Ok(());
            }
            124 | 127 => {
                let p = pos.ok_or(SimError::Unsupported)?;
                return self.launch_nuke(unit_id, p);
            }
            126 => {
                let t = target.ok_or(SimError::Unsupported)?;
                let u = self.unit(t).ok_or(SimError::InvalidTarget)?;
                let p = (u.x / fp(1), u.y / fp(1));
                return self.launch_nuke(unit_id, p);
            }
            _ => {}
        }
        // Terran lift/land rows (69-73) have their own typed states.
        match order_id {
            71..=73 => return self.issue(unit_id, Order::Lift),
            69 | 70 => {
                let p = pos.ok_or(SimError::Unsupported)?;
                return self.issue(unit_id, Order::Land { pos: p });
            }
            _ => {}
        }
        let order = match order_class(order_id) {
            OrderClass::Idle => Order::Stop,
            OrderClass::Move => Order::Move {
                pos: pos.ok_or(SimError::Unsupported)?,
            },
            OrderClass::Attack => match target {
                Some(t) => Order::AttackUnit { target: t },
                None => Order::AttackMove {
                    pos: pos.ok_or(SimError::Unsupported)?,
                },
            },
            OrderClass::AttackMove => Order::AttackMove {
                pos: pos.ok_or(SimError::Unsupported)?,
            },
            OrderClass::Hold => Order::HoldPosition,
            OrderClass::Patrol => {
                let a = pos.ok_or(SimError::Unsupported)?;
                Order::Patrol {
                    a,
                    b: a,
                    forward: true,
                }
            }
            OrderClass::Harvest => Order::Gather {
                resource: target.ok_or(SimError::Unsupported)?,
            },
            OrderClass::ReturnCargo => Order::ReturnCargo {
                depot: target.ok_or(SimError::Unsupported)?,
            },
            OrderClass::Build => return Err(SimError::Unsupported),
            OrderClass::Train => return Err(SimError::Unsupported),
            OrderClass::Research => return Err(SimError::Unsupported),
            OrderClass::Upgrade => return Err(SimError::Unsupported),
            OrderClass::Cast => return Err(SimError::Unsupported),
            OrderClass::Transport => Order::EnterTransport {
                transport: target.ok_or(SimError::Unsupported)?,
            },
            OrderClass::Unload => Order::Unload,
            OrderClass::Burrow => Order::Burrow,
            OrderClass::Siege => Order::Siege,
            OrderClass::Morph => return Err(SimError::Unsupported),
            OrderClass::Special => Order::Stop,
        };
        self.issue(unit_id, order)
    }

    pub fn stop(&mut self, unit_id: u32) {
        if let Some(i) = self.index_of(unit_id) {
            self.units[i].order = Order::Stop;
            self.units[i].resume_order = None;
            self.units[i].queue.clear();
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            self.units[i].target = None;
        }
    }

    pub fn queue_order(&mut self, unit_id: u32, o: Order) -> Result<(), SimError> {
        let i = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        // `orders.dat` uses two independent bits here: `can_be_queued`
        // decides replace-vs-queue, while a non-`-1` highlight decides
        // whether the order consumes one of the eight queue slots.
        let row = self.data.order(o.order_id());
        let can_queue = row.map(|r| r.can_be_queued).unwrap_or(false);
        let uses_slot = row.map(|r| r.highlight != -1).unwrap_or(false);
        if can_queue && uses_slot && self.units[i].queue.len() >= MAX_ORDER_QUEUE {
            return Err(SimError::QueueFull);
        }
        if self.units[i].order == Order::Idle || !can_queue {
            self.units[i].order_row = o.order_id();
            self.units[i].order = o;
            if !can_queue {
                self.units[i].queue.clear();
            }
        } else {
            self.units[i].queue.push_back(o);
        }
        Ok(())
    }

    /// Recorded mutator (`RecAct`): append a waypoint order without
    /// replacing the current one.  Used by Shift/LT right-click.
    pub fn queue_order_recorded(&mut self, unit_id: u32, o: Order) -> Result<(), SimError> {
        self.queue_order(unit_id, o.clone())?;
        if let Some(r) = self.recorder.as_mut() {
            r.cmds.push(crate::replay::RecCmd {
                tick: self.tick,
                act: crate::replay::RecAct::QueueOrder {
                    unit: unit_id,
                    order: o,
                },
            });
        }
        Ok(())
    }

    /// Recorded mutator (`RecAct`): saves, replays and LAN replay it.
    pub fn set_rally(&mut self, building_id: u32, pos: (i32, i32)) -> Result<(), SimError> {
        self.set_rally_unrecorded(building_id, pos)?;
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::Rally {
                    building: building_id,
                    pos,
                },
            );
        }
        Ok(())
    }

    pub(super) fn set_rally_unrecorded(
        &mut self,
        building_id: u32,
        pos: (i32, i32),
    ) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        // `0x005b6030`: only production/town-hall structures own a rally.
        if !is_production_building(self.units[i].type_id) {
            return Err(SimError::MissingRequirements);
        }
        self.units[i].rally = Some(pos);
        Ok(())
    }

    pub(super) fn execute_order(&mut self, i: usize, order: Order, think: bool) {
        if self.units[i].status.stasis_ticks > 0
            || self.units[i].status.lockdown_ticks > 0
            || self.units[i].status.maelstrom_ticks > 0
            || (self.units[i].status.burrowed && !matches!(order, Order::Unburrow))
        {
            return;
        }
        match order {
            Order::Idle => self.order_idle(i),
            Order::Stop => {
                self.units[i].target = None;
                self.units[i].path.clear();
                self.units[i].path_goal = None;
                self.units[i].order = Order::Idle;
            }
            Order::Move { pos } => {
                if self.move_towards(i, pos, fp(4)) {
                    self.order_done(i);
                }
            }
            Order::AttackMove { pos } => {
                // Retail `HealMove`: a Medic on attack-move stops for a
                // patient; `medic_healing` walks it there and heals.
                if self.units[i].type_id == uid::MEDIC
                    && self.units[i].energy_fixed() >= MEDIC_HEAL_COST_PER_TICK
                    && self.medic_pick_target(i, &[]).is_some()
                {
                    return;
                }
                if think && let Some(target) = self.acquire_target(i) {
                    self.units[i].resume_order = Some(Order::AttackMove { pos });
                    if self.units[i].type_id == uid::CARRIER {
                        self.units[i].special_cooldown = 0x3c;
                    }
                    self.units[i].order = Order::AttackUnit { target };
                    self.attack_target(i, target);
                    return;
                }
                if self.move_towards(i, pos, fp(4)) {
                    self.order_done(i);
                }
            }
            Order::AttackUnit { target } => {
                if think {
                    self.attack_target(i, target);
                } else {
                    self.advance_attack(i, target);
                }
            }
            Order::HoldPosition => {
                if let Some(target) = self.acquire_target(i) {
                    self.attack_target_no_move(i, target);
                }
            }
            Order::Patrol { a, b, forward } => {
                if let Some(target) = self.acquire_target(i) {
                    self.attack_target(i, target);
                    return;
                }
                let dest = if forward { b } else { a };
                if self.move_towards(i, dest, fp(4)) {
                    let next = Order::Patrol {
                        a,
                        b,
                        forward: !forward,
                    };
                    self.units[i].order = next;
                }
            }
            Order::Gather { resource } => self.order_gather(i, resource),
            Order::ReturnCargo { depot } => self.order_return(i, depot),
            Order::Build { unit_type, pos } => {
                if think {
                    self.order_build(i, unit_type, pos);
                } else {
                    // The walk to the site advances every tick; only the
                    // placement decision runs on the 8-tick order cadence
                    // (the SCV crawled at 1/8 of its speed, Wasteland).
                    let pos_fp = (pos.0 * fp(1), pos.1 * fp(1));
                    if self.distance_to_point(i, pos_fp) > fp(64) {
                        self.move_towards(i, pos, fp(32));
                    }
                }
            }
            Order::Repair { target } => self.order_repair(i, target),
            Order::Construct { building } => self.order_construct(i, building),
            Order::Train { .. } | Order::Research { .. } | Order::Upgrade { .. } => {
                // Handled by update_production.
            }
            Order::Cast { spell, target } => {
                // Yamato Gun charging: the shot leaves when the charge
                // overlay (image 543, iscript 314) reaches `castspell`.
                if spell == Spell::YamatoGun && self.units[i].order_wait > 0 {
                    self.units[i].order_wait -= 1;
                    if self.units[i].order_wait == 0 {
                        let id = self.units[i].id;
                        let _ = self.cast_unrecorded(id, spell, target);
                        self.order_done(i);
                    }
                    return;
                }
                if think {
                    self.order_cast(i, spell, target);
                } else {
                    // The walk into casting range advances every tick (it
                    // ran at 1/8 speed on the order cadence, like the build
                    // walk).
                    let range = self
                        .spell_range_px(spell)
                        .unwrap_or_else(|| spell_range(spell));
                    let pos = match target {
                        SpellTarget::Unit(id) => self.unit(id).map(|u| (u.x, u.y)),
                        SpellTarget::Point(x, y) => Some((x * fp(1), y * fp(1))),
                        SpellTarget::None => None,
                    };
                    if let Some(p) = pos
                        && self.distance_to_point(i, p) > range
                    {
                        self.move_towards(i, (p.0 / fp(1), p.1 / fp(1)), range / fp(1));
                    }
                }
            }
            Order::EnterTransport { transport } => self.order_enter_transport(i, transport),
            Order::PickupTransport { unit } => self.order_pickup_transport(i, unit),
            Order::Unload => self.order_unload(i),
            // OpenBW `order_MoveUnload`: within 16 px of the point the
            // transport starts unloading.
            Order::MoveUnload { pos } => {
                if self.distance_to_point(i, (pos.0 * fp(1), pos.1 * fp(1))) <= fp(16) {
                    self.units[i].path.clear();
                    self.units[i].path_goal = None;
                    self.units[i].order = Order::Unload;
                    self.units[i].order_wait = 0;
                } else {
                    self.move_towards(i, pos, fp(4));
                }
            }
            Order::Burrow => {
                if think {
                    self.order_burrow(i, true);
                }
            }
            Order::Unburrow => {
                if think {
                    self.order_burrow(i, false);
                }
            }
            // The transformation counts frames, so it runs every tick.
            Order::Siege => self.order_siege(i, true),
            Order::Unsiege => self.order_siege(i, false),
            Order::Lift => {
                let id = self.units[i].id;
                let _ = self.lift_building(id);
            }
            Order::Land { pos } => {
                // Fly to the grid-snapped spot (the preview's), then land
                // (Desperate Alliance recording: the Barracks flies over and
                // sets down in the green box).
                let id = self.units[i].id;
                let ty = self.units[i].type_id;
                let ((cx, cy), _, tiles) = self.placement_tiles(ty, pos, Some(id));
                if !tiles.iter().all(|t| *t) {
                    self.order_done(i);
                } else if self.distance_to_point(i, (cx * fp(1), cy * fp(1))) > fp(2) {
                    self.move_towards(i, (cx, cy), fp(1));
                } else if self.land_building(id, (cx, cy)).is_err() {
                    self.order_done(i);
                }
            }
            Order::Die => {}
        }
    }

    pub(super) fn order_done(&mut self, i: usize) {
        if self.units[i].anim.as_ref().is_some_and(|a| {
            matches!(
                a.anim as usize,
                ANIM_GND_ATTACK_INIT
                    | ANIM_AIR_ATTACK_INIT
                    | ANIM_GND_ATTACK_REPEAT
                    | ANIM_AIR_ATTACK_REPEAT
                    | ANIM_CAST_SPELL
            )
        }) {
            self.units[i].anim = None;
        }
        if let Some(resume) = self.units[i].resume_order.take() {
            // Finishing the temporary attack returns the unit to its
            // attack-move destination instead of dropping to Idle.
            self.units[i].order = resume;
            self.units[i].target = None;
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            return;
        }
        self.units[i].target = None;
        self.units[i].path.clear();
        self.units[i].path_goal = None;
        // Translation batch 06 (`0x005b7c20`): replace the finished order
        // with `Idle` (then promote the next queued node) and refresh the
        // `orders.dat` row.
        self.units[i].a06_pop_order_head(&self.data);
    }

    pub(super) fn order_idle(&mut self, i: usize) {
        let ty = self.data.unit(self.units[i].type_id).clone();
        if ty.is_building() {
            // Defensive buildings (Missile Turret, Photon Cannon, Sunken and
            // Spore Colony) acquire and fire while stationary; economic
            // buildings have no weapon and stay idle.
            if let Some(target) = self.acquire_target(i) {
                self.attack_target_no_move(i, target);
            }
            return;
        }
        if self.units[i].type_id == uid::SCV
            || self.units[i].type_id == uid::PROBE
            || self.units[i].type_id == uid::DRONE
            // The Spider Mine runs its own state machine
            // (`update_specials`), not the generic acquire-and-attack.
            || self.units[i].type_id == uid::SPIDER_MINE
        {
            return;
        }
        if let Some(target) = self.acquire_target(i) {
            self.attack_target(i, target);
        }
    }

    /// Apply a simple AI order: move/attack-move.
    pub fn command_move(&mut self, unit_ids: &[u32], pos: (i32, i32)) {
        for &id in unit_ids {
            let _ = self.issue(id, Order::Move { pos });
        }
    }

    pub fn command_attack_move(&mut self, unit_ids: &[u32], pos: (i32, i32)) {
        for &id in unit_ids {
            let _ = self.issue(id, Order::AttackMove { pos });
        }
    }
}
