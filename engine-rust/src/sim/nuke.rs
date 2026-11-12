//! `impl World`: nuke (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

impl World {
    /// Launch a nuclear strike from a Ghost.  Requires a completed Nuclear
    /// Silo and pays the 200/200 cost.  The missile is a travelling projectile
    /// with the retail nuclear hit type; impact damage is
    /// `max(500, 2/3 * max_hp_plus_shields)` over the large splash radius.
    /// Arm a completed, attached Nuclear Silo (`0x5d1e80`).  The missile
    /// costs its `units.dat` 200/200 when armed, and the silo holds exactly
    /// one until a Ghost launches it.
    pub fn arm_nuke(&mut self, silo_id: u32) -> Result<(), SimError> {
        let i = self.index_of(silo_id).ok_or(SimError::BadUnit)?;
        if self.units[i].type_id != uid::NUCLEAR_SILO
            || !self.units[i].alive
            || !self.units[i].completed
            || !self.units[i].addon_attached
        {
            return Err(SimError::MissingRequirements);
        }
        if self.units[i].nuke_ready {
            return Ok(());
        }
        let owner = self.units[i].owner as usize;
        let p = &mut self.players[owner];
        if p.minerals < 200 {
            return Err(SimError::NotEnoughMinerals);
        }
        if p.gas < 200 {
            return Err(SimError::NotEnoughGas);
        }
        p.pay_minerals(200);
        p.pay_gas(200);
        self.units[i].nuke_ready = true;
        Ok(())
    }

    pub fn launch_nuke(&mut self, ghost_id: u32, pos: (i32, i32)) -> Result<(), SimError> {
        self.launch_nuke_unrecorded(ghost_id, pos)?;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                self.tick,
                crate::replay::RecAct::Nuke {
                    ghost: ghost_id,
                    pos,
                },
            );
        }
        Ok(())
    }

    pub(super) fn launch_nuke_unrecorded(
        &mut self,
        ghost_id: u32,
        pos: (i32, i32),
    ) -> Result<(), SimError> {
        let i = self.index_of(ghost_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive || !is_ghost_type(self.units[i].type_id) {
            return Err(SimError::Unsupported);
        }
        if self.nuke_strikes.iter().any(|s| s.ghost == ghost_id) {
            return Err(SimError::ProducerBusy);
        }
        let owner = self.units[i].owner;
        let is_silo = |u: &SimUnit| {
            u.alive
                && u.completed
                && u.owner == owner
                && u.type_id == uid::NUCLEAR_SILO
                && u.addon_attached
        };
        // A silo that already holds a missile first.
        let Some(silo) = self
            .units
            .iter()
            .position(|u| is_silo(u) && u.nuke_ready)
            .or_else(|| self.units.iter().position(is_silo))
        else {
            return Err(SimError::MissingRequirements);
        };
        let silo_id = self.units[silo].id;
        if !self.units[silo].nuke_ready {
            // The missile has to be built in the silo first (200 / 200,
            // build time 1500).  A computer player whose script asks for a
            // strike without one starts building it and strikes on a later
            // request, like a player would.
            if self.players[owner as usize].is_computer {
                let _ = self.train(silo_id, uid::NUCLEAR_MISSILE);
            }
            return Err(SimError::MissingRequirements);
        }
        self.units[silo].nuke_ready = false;
        // `0x005b4950`: the launched missile links back to its silo
        // (`CUnit+0xd0`), and the silo records the live missile.
        self.nuke_strikes.push(NukeStrike {
            owner,
            ghost: ghost_id,
            silo: silo_id,
            target: (pos.0 * fp(1), pos.1 * fp(1)),
            timer: 90,
            dot: true,
            warned: false,
            missile: 0,
        });
        self.units[i].cooldown = 5;
        Ok(())
    }

    pub(super) fn advance_nuke_strikes(&mut self) {
        // The Ghost paints the target from within its sight range: until it
        // is that close it walks towards the point and the countdown waits.
        // OpenBW `order_CastNuclearStrike`: centre distance to the point
        // greater than `unit_sight_range` keeps the Ghost walking.
        let mut waiting = Vec::new();
        for k in 0..self.nuke_strikes.len() {
            let (ghost, target) = (self.nuke_strikes[k].ghost, self.nuke_strikes[k].target);
            let Some(g) = self.index_of(ghost) else {
                continue;
            };
            let range = self.unit_sight_range(g) * 32 * fp(1);
            if self.distance_to_point(g, target) > range {
                self.move_towards(g, (target.0 / fp(1), target.1 / fp(1)), fp(8));
                waiting.push(ghost);
            } else {
                self.units[g].path.clear();
                self.units[g].path_goal = None;
            }
        }
        // The countdown has started: the missile leaves its silo and climbs
        // off the top of the map (OpenBW `order_NukeLaunch`: 90 frames,
        // warning at 45).  It is invincible and only seen, never fought.
        for k in 0..self.nuke_strikes.len() {
            let st = &self.nuke_strikes[k];
            if st.missile != 0 || waiting.contains(&st.ghost) {
                continue;
            }
            let (owner, silo, ghost) = (st.owner, st.silo, st.ghost);
            let Some(si) = self.index_of(silo) else {
                continue;
            };
            let (sx, sy) = (self.units[si].x / fp(1), self.units[si].y / fp(1));
            if let Ok(m) = self.spawn_px(owner as usize, uid::NUCLEAR_MISSILE, sx, sy) {
                self.units[m].invincible = true;
                self.units[m].connected_unit = Some(ghost);
                self.units[m].order = Order::Idle;
                self.units[m].facing = 0;
                self.units[m].speed = 0;
                self.nuke_strikes[k].missile = self.units[m].id;
            }
        }
        let mut launched = Vec::new();
        for strike in &mut self.nuke_strikes {
            if waiting.contains(&strike.ghost) {
                continue;
            }
            strike.timer -= 1;
            if strike.timer <= 45 && !strike.warned {
                strike.warned = true;
                self.nuke_warnings
                    .push((strike.target.0 / fp(1), strike.target.1 / fp(1)));
            }
            // The missile is out of sight for its WarpIn animation
            // (iscript 131: `wait 125, wait 125, sigorder 2`) before it
            // reappears above the target.
            if strike.timer == -NUKE_WARP_FRAMES {
                launched.push((strike.owner, strike.ghost, strike.target));
            }
        }
        for (owner, ghost, target) in launched {
            self.spawn_nuke_missile(owner, ghost, target);
        }
        // The strike stays on the books until the bomb has landed.
        let falling = |w: &World, ghost: u32| {
            w.projectiles
                .iter()
                .find(|p| p.weapon == wid::NUCLEAR_STRIKE && p.source == ghost)
                .map(|p| (p.x, p.y))
        };
        let live: Vec<bool> = self
            .nuke_strikes
            .iter()
            .map(|s| s.timer > -NUKE_WARP_FRAMES || falling(self, s.ghost).is_some())
            .collect();
        let mut keep = live.iter();
        self.nuke_strikes.retain(|_| *keep.next().unwrap_or(&false));
        // The missile unit: climbing during the countdown, hidden while it
        // is off the map, then the same unit 320 px above the target,
        // heading down (OpenBW `order_NukeLaunch` states 2 to 5), carried
        // by the nuclear bullet that deals the damage.
        for k in 0..self.nuke_strikes.len() {
            let (timer, ghost, missile) = {
                let s = &self.nuke_strikes[k];
                (s.timer, s.ghost, s.missile)
            };
            let Some(m) = self.index_of(missile) else {
                continue;
            };
            if timer > 0 {
                // Climbing straight up from rest with the missile's flingy
                // acceleration (33/256 px per frame per frame, top speed
                // 8533/256).
                let v = (self.units[m].speed + NUKE_ACCELERATION).min(NUKE_TOP_SPEED);
                self.units[m].speed = v;
                self.units[m].y -= v;
                continue;
            }
            // The falling bomb accelerates the same way.
            if let Some(p) = self
                .projectiles
                .iter_mut()
                .find(|p| p.weapon == wid::NUCLEAR_STRIKE && p.source == ghost)
            {
                p.speed = (p.speed + NUKE_ACCELERATION).min(NUKE_TOP_SPEED);
            }
            match falling(self, ghost) {
                Some((x, y)) => {
                    self.units[m].alive = true;
                    self.units[m].x = x;
                    self.units[m].y = y;
                    self.units[m].facing = 128;
                    self.units[m].order = Order::Idle;
                    self.units[m].path.clear();
                    self.units[m].path_goal = None;
                }
                None => self.units[m].alive = false,
            }
        }
        // A missile whose strike landed or was called off leaves the map.
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if u.alive
                && u.type_id == uid::NUCLEAR_MISSILE
                && u.invincible
                && !self.nuke_strikes.iter().any(|s| s.missile == u.id)
            {
                self.units[i].alive = false;
                self.units[i].order = Order::Die;
            }
        }
    }

    pub(super) fn spawn_nuke_missile(&mut self, owner: u8, ghost: u32, target: (i32, i32)) {
        let target_unit = self
            .units
            .iter()
            .position(|u| {
                u.alive
                    && u.owner != owner
                    && (u.x - target.0).abs() < fp(4)
                    && (u.y - target.1).abs() < fp(4)
            })
            .map(|i| self.units[i].id);
        self.projectiles.push(Projectile {
            id: self.next_projectile_id,
            owner,
            source: ghost,
            weapon: wid::NUCLEAR_STRIKE,
            behavior: behavior::FLY,
            hit_type: hit::NUCLEAR,
            state: BulletState::Fly,
            x: target.0,
            y: target.1 - 320 * fp(1),
            target: target_unit,
            target_pos: target,
            // From rest; `advance_nuke_strikes` accelerates it each frame,
            // so the 320 px take about 70 frames.
            speed: NUKE_ACCELERATION,
            remaining_time: 300,
            bounces_left: 0,
            hit_flags: 0,
            previous_target: None,
            last_dot_tick: self.tick as i64,
        });
        self.next_projectile_id = self.next_projectile_id.wrapping_add(1).max(1);
    }

    /// `0x005b4950`: record the launched missile as linked to its silo
    /// (`CUnit+0xd0`, with `CUnit+0xd4 = 1`).
    pub fn link_nuke_missile(&mut self, silo_id: u32) -> bool {
        self.units
            .iter()
            .any(|u| u.id == silo_id && u.alive && u.type_id == uid::NUCLEAR_SILO)
    }

    /// `0x005b5a30`: true when the silo has no linked missile
    /// (`CUnit+0xd0` is zero).
    pub fn has_no_linked_missile(&self, silo_id: u32) -> bool {
        !self.nuke_strikes.iter().any(|s| s.silo == silo_id)
    }
}
