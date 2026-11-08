//! A small deterministic computer opponent.
//!
//! This is not the retail campaign AI (which is driven by `aiscript.bin` and
//! `bwscript.bin`).  It is a rules-level opponent for exercising the engine:
//! it keeps workers mining, maintains supply, follows a short build order,
//! adds production when its bank grows, defends its base, gathers its army
//! and attacks in waves that grow over the game.  Every decision is
//! deterministic (it reads only the world), so every LAN peer runs the same
//! computers.

use crate::ids::{tech, unit as uid, upgrade};
use crate::sim::{Order, Race, SimError, World};

#[derive(Clone, Debug)]
pub struct SimpleAi {
    pub player: u8,
    pub base: Option<(i32, i32)>,
    pub next_think: u64,
    pub next_attack: u64,
    pub last_build: u64,
    /// Attack waves launched so far (each needs a bigger army).
    pub waves: u32,
    /// Tick of the last order sent to answer an attack on the base.
    pub last_defend: u64,
}

/// Radius around the base (px) where enemies make the army come home.
const DEFEND_RADIUS: i32 = 640;
/// Production buildings the computer grows to (Barracks / Gateways /
/// Hatcheries) once its bank allows it.
const MAX_PRODUCTION: usize = 3;
/// Army it keeps building towards.
const MAX_ARMY: usize = 40;

/// Combat units a wave needs: 6, then 10, 14, ... up to 24 (a trickle of
/// single units was what the old timer sent).
pub fn wave_size(waves: u32) -> usize {
    (6 + 4 * waves as usize).min(24)
}

/// One tick of every computer player, in slot order: each attacks an enemy
/// town hall (`prefer`'s when it is an enemy, see
/// [`World::enemy_town_hall`]).  Shared by the local game loop and the LAN
/// lockstep so every peer runs the same computers identically; `prefer`
/// must therefore be the same on every peer in a LAN game.
pub fn run_computers(world: &mut World, ais: &mut [SimpleAi], prefer: u8) {
    for ai in ais.iter_mut() {
        let enemy_base = world.enemy_town_hall(ai.player, prefer);
        ai.update(world, enemy_base);
    }
}

impl SimpleAi {
    pub fn new(player: u8) -> Self {
        Self {
            player,
            base: None,
            next_think: 0,
            next_attack: 1200,
            last_build: 0,
            waves: 0,
            last_defend: 0,
        }
    }

    pub fn update(&mut self, world: &mut World, enemy_base: Option<(i32, i32)>) {
        if world.tick < self.next_think {
            return;
        }
        self.next_think = world.tick + 12;
        let owner = self.player;
        if !world.players[owner as usize].defeated && world.unit_ids(owner).is_empty() {
            world.players[owner as usize].defeated = true;
            return;
        }
        self.find_base(world);
        self.keep_workers_mining(world);
        self.manage_supply(world);
        self.build_order(world);
        self.add_production(world);
        self.train_army(world);
        if self.defend(world) {
            return;
        }
        if world.tick >= self.next_attack {
            if self.army(world).len() >= wave_size(self.waves) {
                self.next_attack = world.tick + 1200;
                self.waves += 1;
                self.attack(world, enemy_base);
            } else {
                // Not enough yet: gather at the front of the base and look
                // again in ten seconds.
                self.next_attack = world.tick + 240;
                self.gather(world, enemy_base);
            }
        }
    }

    /// Own combat units.
    fn army(&self, world: &World) -> Vec<u32> {
        world
            .units
            .iter()
            .filter(|u| u.alive && u.owner == self.player && is_combat(u.type_id))
            .map(|u| u.id)
            .collect()
    }

    /// An enemy unit inside [`DEFEND_RADIUS`] of the base brings the whole
    /// army there (at most every 4 s, so the orders are not re-issued every
    /// think).  Returns true when the base is under attack.
    fn defend(&mut self, world: &mut World) -> bool {
        let Some((bx, by)) = self.base else {
            return false;
        };
        let owner = self.player;
        let threat = world
            .units
            .iter()
            .filter(|u| {
                u.alive
                    && world.is_enemy(owner, u.owner)
                    && !crate::sim::is_resource_type(u.type_id)
                    && (u.x / 256 - bx).pow(2) + (u.y / 256 - by).pow(2) <= DEFEND_RADIUS.pow(2)
            })
            .map(|u| (u.x / 256, u.y / 256))
            .next();
        let Some(pos) = threat else {
            return false;
        };
        if world.tick >= self.last_defend + 96 {
            self.last_defend = world.tick;
            let army = self.army(world);
            world.command_attack_move(&army, pos);
        }
        true
    }

    /// Idle combat units wait together in front of the base, toward the
    /// enemy, so a wave leaves as one group.
    fn gather(&self, world: &mut World, enemy_base: Option<(i32, i32)>) {
        let (Some((bx, by)), Some((ex, ey))) = (self.base, enemy_base) else {
            return;
        };
        let rally = (bx + (ex - bx) / 6, by + (ey - by) / 6);
        let idle: Vec<u32> = world
            .units
            .iter()
            .filter(|u| {
                u.alive
                    && u.owner == self.player
                    && is_combat(u.type_id)
                    && matches!(u.order, Order::Idle | Order::Stop)
            })
            .map(|u| u.id)
            .collect();
        for id in idle {
            let _ = world.issue(id, Order::Move { pos: rally });
        }
    }

    /// With a bank of 350+ minerals after the first production building,
    /// add more of them (up to [`MAX_PRODUCTION`]).
    fn add_production(&mut self, world: &mut World) {
        let owner = self.player;
        if world.players[owner as usize].minerals < 350 {
            return;
        }
        let producer = match world.players[owner as usize].race {
            Race::Terran => uid::BARRACKS,
            Race::Protoss => uid::GATEWAY,
            Race::Zerg => uid::HATCHERY,
            Race::Neutral => return,
        };
        let have = self.count(world, producer)
            + if producer == uid::HATCHERY {
                self.count(world, uid::LAIR) + self.count(world, uid::HIVE)
            } else {
                0
            };
        if (1..MAX_PRODUCTION).contains(&have) {
            self.try_build(world, producer);
        }
    }

    fn find_base(&mut self, world: &World) {
        if self.base.is_some() {
            return;
        }
        self.base = world
            .units
            .iter()
            .filter(|u| {
                u.alive
                    && u.owner == self.player
                    && u.completed
                    && matches!(
                        u.type_id,
                        uid::COMMAND_CENTER | uid::NEXUS | uid::HATCHERY | uid::LAIR | uid::HIVE
                    )
            })
            .map(|u| (u.x / 256, u.y / 256))
            .next();
    }

    fn keep_workers_mining(&self, world: &mut World) {
        let gas_building = world
            .units
            .iter()
            .find(|u| {
                u.alive
                    && u.owner == self.player
                    && u.completed
                    && matches!(u.type_id, uid::REFINERY | uid::ASSIMILATOR | uid::EXTRACTOR)
                    && u.resource_amount > 0
            })
            .map(|u| u.id);
        let gas_workers = gas_building
            .map(|g| {
                world
                    .units
                    .iter()
                    .filter(|u| {
                        u.alive
                            && u.owner == self.player
                            && is_worker(u.type_id)
                            && (u.carry_kind == crate::sim::CarryKind::Gas
                                || u.resource_target == Some(g)
                                || matches!(u.order, Order::Gather { resource } if resource == g))
                    })
                    .count()
            })
            .unwrap_or(0);
        // Candidates ordered by how little they would disrupt: idle first,
        // then workers that are not carrying anything, then loaded workers.
        let mut candidates: Vec<(bool, i32, u32)> = world
            .units
            .iter()
            .filter(|u| u.alive && u.owner == self.player && is_worker(u.type_id))
            .filter(|u| {
                // Never steal a worker already assigned to gas.
                let ordered_to_gas = gas_building
                    .map(|g| matches!(u.order, Order::Gather { resource } if resource == g))
                    .unwrap_or(false);
                !ordered_to_gas
                    && gas_building.is_none_or(|g| u.resource_target != Some(g))
                    && u.carry_kind != crate::sim::CarryKind::Gas
            })
            .map(|u| (u.order == Order::Idle, u.carry, u.id))
            .collect();
        candidates.sort_by_key(|&(idle, carry, id)| (!idle, carry, id));
        if gas_workers < 3
            && let Some(g) = gas_building
        {
            for (_, _, worker) in candidates.iter().take(3 - gas_workers) {
                let _ = world.issue(*worker, Order::Gather { resource: g });
            }
        }
        for (idle, _, worker) in candidates {
            if !idle {
                break;
            }
            if world.units[world.index_of(worker).unwrap()].order != Order::Idle {
                continue;
            }
            let Some(widx) = world.index_of(worker) else {
                continue;
            };
            let (wx, wy) = (world.units[widx].x, world.units[widx].y);
            let resource = world
                .units
                .iter()
                .filter(|u| u.alive && is_resource(u.type_id) && u.resource_amount > 0)
                .min_by_key(|u| {
                    let dx = (u.x - wx) as i64;
                    let dy = (u.y - wy) as i64;
                    dx * dx + dy * dy
                })
                .map(|u| u.id);
            if let Some(r) = resource {
                let _ = world.issue(worker, Order::Gather { resource: r });
            }
        }
    }

    fn manage_supply(&mut self, world: &mut World) {
        let owner = self.player;
        let cap = world.supply_cap(owner);
        let used = world.supply_used(owner);
        if cap - used > 6 {
            return;
        }
        let race = world.players[owner as usize].race;
        let supply_type = match race {
            Race::Terran => Some(uid::SUPPLY_DEPOT),
            Race::Protoss => Some(uid::PYLON),
            Race::Zerg => Some(uid::OVERLORD),
            Race::Neutral => None,
        };
        if let Some(supply_type) = supply_type {
            // An Overlord in the making is an Egg (INFERENCE: any egg counts,
            // so a drone egg can delay the next Overlord by one morph).
            let pending = world.units.iter().any(|u| {
                u.alive
                    && u.owner == owner
                    && ((u.type_id == supply_type && !u.completed)
                        || (supply_type == uid::OVERLORD && u.type_id == uid::EGG))
            });
            if pending {
                return;
            }
        }
        match race {
            Race::Terran => {
                self.try_build(world, uid::SUPPLY_DEPOT);
            }
            Race::Protoss => {
                self.try_build(world, uid::PYLON);
            }
            Race::Zerg => {
                // Overlords hatch from larvae.  `try_train` asked the
                // Hatchery, which took the 100 minerals into a queue it
                // can never produce, so a Zerg computer stayed supply
                // blocked all game (`sc_match`, 2026-11-06).
                let _ = self.try_morph(world, uid::OVERLORD);
            }
            Race::Neutral => {}
        }
    }

    fn build_order(&mut self, world: &mut World) {
        let owner = self.player;
        let race = world.players[owner as usize].race;
        let workers = world
            .units
            .iter()
            .filter(|u| u.alive && u.owner == owner && is_worker(u.type_id))
            .count();
        let want_workers = if world.tick < 2400 { 12 } else { 18 };
        if workers < want_workers {
            let worker_type = match race {
                Race::Terran => uid::SCV,
                Race::Protoss => uid::PROBE,
                Race::Zerg => uid::DRONE,
                Race::Neutral => return,
            };
            if race == Race::Zerg {
                let _ = self.try_morph(world, worker_type);
            } else {
                let _ = self.try_train(world, worker_type);
            }
            return;
        }

        match race {
            // Production before gas: a gas building that cannot be placed
            // (no geyser in reach) used to block the whole order, so a
            // Protoss computer never built a Gateway and banked 10,000
            // minerals (`sc_match` on Boxer, 2026-11-06).
            Race::Terran => {
                if !self.has(world, uid::BARRACKS) {
                    self.try_build(world, uid::BARRACKS);
                } else if !self.has(world, uid::REFINERY) && self.try_gas(world, uid::REFINERY) {
                } else if !self.has(world, uid::ACADEMY) {
                    self.try_build(world, uid::ACADEMY);
                } else if !self.has(world, uid::FACTORY) {
                    self.try_build(world, uid::FACTORY);
                } else if !self.has(world, uid::ENGINEERING_BAY) {
                    self.try_build(world, uid::ENGINEERING_BAY);
                } else if !self.has(world, uid::STARPORT) {
                    self.try_build(world, uid::STARPORT);
                }
            }
            Race::Protoss => {
                if self.count(world, uid::PYLON) < 2 {
                    self.try_build(world, uid::PYLON);
                } else if !self.has(world, uid::GATEWAY) {
                    self.try_build(world, uid::GATEWAY);
                } else if !self.has(world, uid::ASSIMILATOR)
                    && self.try_gas(world, uid::ASSIMILATOR)
                {
                } else if !self.has(world, uid::CYBERNETICS_CORE) {
                    self.try_build(world, uid::CYBERNETICS_CORE);
                } else if !self.has(world, uid::FORGE) {
                    self.try_build(world, uid::FORGE);
                } else if !self.has(world, uid::STARGATE) {
                    self.try_build(world, uid::STARGATE);
                }
            }
            Race::Zerg => {
                // The Spawning Pool comes first: the Extractor needs creep
                // at the geyser, which may only arrive after the creep
                // maturity grid has spread.
                if !self.has(world, uid::SPAWNING_POOL) {
                    self.try_build(world, uid::SPAWNING_POOL);
                } else if !self.has(world, uid::EXTRACTOR) && self.try_gas(world, uid::EXTRACTOR) {
                } else if !self.has(world, uid::HYDRALISK_DEN) {
                    self.try_build(world, uid::HYDRALISK_DEN);
                } else if !self.has(world, uid::LAIR) {
                    self.try_build(world, uid::LAIR);
                }
            }
            Race::Neutral => {}
        }
    }

    fn train_army(&mut self, world: &mut World) {
        let owner = self.player;
        let race = world.players[owner as usize].race;
        let army = world
            .units
            .iter()
            .filter(|u| u.alive && u.owner == owner && is_combat(u.type_id))
            .count();
        if army >= MAX_ARMY {
            return;
        }
        // Keep a little bank for buildings.
        if world.players[owner as usize].minerals < 150 {
            return;
        }
        // A mixed army: the gas unit when it can be paid for, the basic one
        // otherwise (and always one basic unit per think from the other
        // production buildings).
        match race {
            Race::Terran => {
                if self.has(world, uid::FACTORY) {
                    let _ = self.try_train(world, uid::SIEGE_TANK);
                }
                let _ = self.try_train(world, uid::MARINE);
            }
            Race::Protoss => {
                if self.has(world, uid::CYBERNETICS_CORE) {
                    let _ = self.try_train(world, uid::DRAGOON);
                }
                let _ = self.try_train(world, uid::ZEALOT);
            }
            Race::Zerg => {
                if self.has(world, uid::HYDRALISK_DEN) {
                    if self.try_morph(world, uid::HYDRALISK).is_err() {
                        let _ = self.try_morph(world, uid::ZERGLING);
                    }
                } else if self.has(world, uid::SPAWNING_POOL) {
                    let _ = self.try_morph(world, uid::ZERGLING);
                }
            }
            Race::Neutral => {}
        }
        let _ = tech::STIM_PACKS;
        let _ = upgrade::TERRAN_INFANTRY_WEAPONS;
    }

    fn attack(&self, world: &mut World, enemy_base: Option<(i32, i32)>) {
        let owner = self.player;
        let army: Vec<u32> = world
            .units
            .iter()
            .filter(|u| u.alive && u.owner == owner && is_combat(u.type_id))
            .map(|u| u.id)
            .collect();
        if army.is_empty() {
            return;
        }
        let target = enemy_base
            .or_else(|| {
                world
                    .units
                    .iter()
                    .filter(|u| {
                        u.alive
                            && u.completed
                            && world.is_enemy(owner, u.owner)
                            && !crate::sim::is_resource_type(u.type_id)
                    })
                    .map(|u| (u.x / 256, u.y / 256))
                    .next()
            })
            .unwrap_or((64, 64));
        world.command_attack_move(&army, target);
    }

    fn has(&self, world: &World, type_id: u16) -> bool {
        self.count(world, type_id) > 0
    }

    fn count(&self, world: &World, type_id: u16) -> usize {
        world
            .units
            .iter()
            .filter(|u| u.alive && u.owner == self.player && u.type_id == type_id)
            .count()
    }

    fn try_train(&mut self, world: &mut World, unit_type: u16) -> Result<(), SimError> {
        let owner = self.player;
        let producer = crate::tech::build_info(unit_type)
            .map(|i| i.producer)
            .ok_or(SimError::MissingRequirements)?;
        // The least busy producer (the unit in training plus its queue),
        // never more than two deep: units spread over every Barracks /
        // Gateway instead of piling on one.
        let load =
            |u: &crate::sim::SimUnit| u.production.len() + usize::from(u.order != Order::Idle);
        let building = world
            .units
            .iter()
            .filter(|u| {
                u.alive && u.owner == owner && u.completed && u.type_id == producer && load(u) < 2
            })
            .min_by_key(|u| (load(u), u.id))
            .map(|u| u.id)
            .ok_or(SimError::MissingRequirements)?;
        world.train(building, unit_type)
    }

    fn try_morph(&mut self, world: &mut World, unit_type: u16) -> Result<(), SimError> {
        let owner = self.player;
        let larva = world
            .units
            .iter()
            .filter(|u| u.alive && u.owner == owner && u.type_id == uid::LARVA)
            .map(|u| u.id)
            .next()
            .ok_or(SimError::MissingRequirements)?;
        world.morph(larva, unit_type)
    }

    /// Build the gas building when a geyser near the base takes it; `false`
    /// (the order moves on) when there is none to build on.
    fn try_gas(&mut self, world: &mut World, type_id: u16) -> bool {
        let Some(base) = self.base else {
            return false;
        };
        if world.find_build_spot(type_id, base).is_none() {
            return false;
        }
        self.try_build(world, type_id);
        true
    }

    fn try_build(&mut self, world: &mut World, type_id: u16) {
        if world.tick < self.last_build + 24 {
            return;
        }
        let owner = self.player;
        let Some(base) = self.base else { return };
        // Only build what the tree allows.
        if !world.has_requirements(owner, type_id) {
            return;
        }
        if !world.can_afford(owner, type_id) {
            return;
        }
        // Zerg building upgrades are morphs of an existing structure, not a
        // new worker-built structure.
        if let Some(source) = world.morph_source_unit(owner, type_id) {
            if world.morph_building(source, type_id).is_ok() {
                self.last_build = world.tick;
            }
            return;
        }
        let Some(spot) = world.find_build_spot(type_id, base) else {
            return;
        };
        // Prefer an idle worker, but pull one off the mineral line when the
        // base has no free worker.  This mirrors the retail "worker leaves
        // the patch to build" behaviour.
        let mut candidates: Vec<(bool, i32, u32)> = world
            .units
            .iter()
            .filter(|u| u.alive && u.owner == owner && is_worker(u.type_id))
            .map(|u| (u.order == Order::Idle, u.carry, u.id))
            .collect();
        candidates.sort_by_key(|&(idle, carry, _)| (!idle, carry));
        let worker = candidates.first().map(|&(_, _, id)| id);
        let Some(worker) = worker else { return };
        if world
            .issue(
                worker,
                Order::Build {
                    unit_type: type_id,
                    pos: spot,
                },
            )
            .is_ok()
        {
            self.last_build = world.tick;
        }
    }
}

fn is_worker(type_id: u16) -> bool {
    matches!(type_id, uid::SCV | uid::PROBE | uid::DRONE)
}

fn is_resource(type_id: u16) -> bool {
    matches!(
        type_id,
        uid::MINERAL_FIELD | uid::MINERAL_FIELD_2 | uid::MINERAL_FIELD_3
    )
}

fn is_combat(type_id: u16) -> bool {
    matches!(
        type_id,
        uid::MARINE
            | uid::FIREBAT
            | uid::GHOST
            | uid::VULTURE
            | uid::SIEGE_TANK
            | uid::SIEGE_MODE
            | uid::GOLIATH
            | uid::WRAITH
            | uid::BATTLECRUISER
            | uid::ZEALOT
            | uid::DRAGOON
            | uid::HIGH_TEMPLAR
            | uid::DARK_TEMPLAR
            | uid::ARCHON
            | uid::DARK_ARCHON
            | uid::SCOUT
            | uid::CARRIER
            | uid::REAVER
            | uid::ZERGLING
            | uid::HYDRALISK
            | uid::LURKER
            | uid::ULTRALISK
            | uid::MUTALISK
            | uid::GUARDIAN
            | uid::DEVOURER
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::map::Terrain;
    use crate::sim::World;
    use std::sync::Arc;

    #[test]
    fn ai_builds_and_trains_over_time() {
        let mut world = World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        );
        world.players[0].minerals = 2000;
        world.players[0].gas = 1000;
        world.players[1].minerals = 2000;
        let cc = world
            .spawn(0, uid::COMMAND_CENTER, 16 * 32, 16 * 32)
            .unwrap();
        let enemy = world
            .spawn(1, uid::COMMAND_CENTER, 48 * 32, 48 * 32)
            .unwrap();
        let mut ai = SimpleAi::new(0);
        for _ in 0..6000 {
            ai.update(&mut world, Some((48, 48)));
            world.step();
        }
        assert!(world.players[0].minerals < 2000);
        assert!(
            world
                .units
                .iter()
                .any(|u| u.alive && u.owner == 0 && is_worker(u.type_id) && u.type_id == uid::SCV)
        );
        let _ = (cc, enemy);
    }

    fn duel() -> (World, SimpleAi) {
        let mut world = World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        );
        world.players[0].minerals = 50;
        world
            .spawn(0, uid::COMMAND_CENTER, 16 * 32, 16 * 32)
            .unwrap();
        world
            .spawn(1, uid::COMMAND_CENTER, 48 * 32, 48 * 32)
            .unwrap();
        let mut ai = SimpleAi::new(0);
        ai.find_base(&world);
        (world, ai)
    }

    fn attack_moving(world: &World) -> usize {
        world
            .units
            .iter()
            .filter(|u| u.alive && u.owner == 0 && matches!(u.order, Order::AttackMove { .. }))
            .count()
    }

    #[test]
    fn waves_wait_for_an_army_and_grow() {
        // User report 2026-11-06: the AI was weak; it sent whatever it had
        // every 50 s, one unit at a time.
        assert_eq!(wave_size(0), 6);
        assert_eq!(wave_size(1), 10);
        assert_eq!(wave_size(10), 24);
        let (mut world, mut ai) = duel();
        for k in 0..3 {
            world
                .spawn(0, uid::MARINE, 20 * 32 + k * 20, 20 * 32)
                .unwrap();
        }
        world.tick = 1200;
        ai.update(&mut world, Some((48 * 32, 48 * 32)));
        assert_eq!(attack_moving(&world), 0, "three marines wait");
        assert_eq!(ai.waves, 0);
        for k in 0..3 {
            world
                .spawn(0, uid::MARINE, 20 * 32 + k * 20, 21 * 32)
                .unwrap();
        }
        world.tick = ai.next_attack;
        ai.next_think = 0;
        ai.update(&mut world, Some((48 * 32, 48 * 32)));
        assert_eq!(attack_moving(&world), 6, "six go together");
        assert_eq!(ai.waves, 1);
    }

    #[test]
    fn the_army_comes_home_when_the_base_is_attacked() {
        let (mut world, mut ai) = duel();
        let marine = world.spawn(0, uid::MARINE, 30 * 32, 30 * 32).unwrap();
        world.tick = 100;
        ai.update(&mut world, Some((48 * 32, 48 * 32)));
        assert_eq!(attack_moving(&world), 0, "nothing to defend");
        let raider = world.spawn(1, uid::ZERGLING, 18 * 32, 16 * 32).unwrap();
        world.tick = 200;
        ai.next_think = 0;
        ai.update(&mut world, Some((48 * 32, 48 * 32)));
        let (rx, ry) = (world.units[raider].x / 256, world.units[raider].y / 256);
        assert_eq!(
            world.units[marine].order,
            Order::AttackMove { pos: (rx, ry) },
            "the marine answers the raid"
        );
    }

    #[test]
    fn a_supply_blocked_zerg_morphs_an_overlord_from_a_larva() {
        let mut world = World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        );
        world.players[0].race = Race::Zerg;
        world.players[0].minerals = 100;
        let hatch = world.spawn(0, uid::HATCHERY, 16 * 32, 16 * 32).unwrap();
        world.units[hatch].completed = true;
        let larva = world.spawn(0, uid::LARVA, 16 * 32, 18 * 32).unwrap();
        for k in 0..9 {
            world
                .spawn(0, uid::DRONE, 14 * 32 + k * 16, 20 * 32)
                .unwrap();
        }
        let mut ai = SimpleAi::new(0);
        ai.find_base(&world);
        ai.manage_supply(&mut world);
        assert_eq!(world.players[0].minerals, 0, "paid for the Overlord");
        assert_eq!(world.units[larva].type_id, uid::EGG, "from the larva");
        assert!(
            world.units[hatch].production.is_empty(),
            "not a Hatchery queue"
        );
    }

    #[test]
    fn units_spread_over_every_production_building() {
        let (mut world, mut ai) = duel();
        world.players[0].minerals = 1000;
        let a = world.spawn(0, uid::BARRACKS, 20 * 32, 12 * 32).unwrap();
        let b = world.spawn(0, uid::BARRACKS, 24 * 32, 12 * 32).unwrap();
        for i in [a, b] {
            world.units[i].completed = true;
        }
        for _ in 0..4 {
            assert!(ai.try_train(&mut world, uid::MARINE).is_ok());
        }
        // Each Barracks: one Marine in training and one queued.
        for i in [a, b] {
            assert_ne!(world.units[i].order, Order::Idle);
            assert_eq!(world.units[i].production.len(), 1);
        }
        assert!(
            ai.try_train(&mut world, uid::MARINE).is_err(),
            "no deep queues"
        );
    }
}
