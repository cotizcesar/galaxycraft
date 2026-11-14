//! `build`, `upgrade` / `tech`, `defensebuild_*`: queued town requests.
//!
//! `build N type` is a lasting request: the town builds or trains until it
//! has N (counting those in production and workers on their way to the
//! site) and rebuilds what is lost.  Before, a `build` the town could not
//! afford at that very tick was forgotten, and a later `wait_build` for it
//! waited forever.  `defensebuild_*` held the script until the defender
//! could be trained (Biting the Bullet's two towns, the Protoss tutorial
//! and Brood War Protoss 3 never got past it with the player idle); it is a
//! request the town keeps filled now.  INFERENCE: the rebuild of lost units and buildings
//! (retail campaign towns rebuild; the exact rule is not read).
//!
//! Retail campaign scripts list a dozen upgrades in a row (The Hammer
//! Falls' T12N / T12P, Norad II's Ter6) and then wait for their first
//! attack; the script thread does not stop at each one.  The request goes
//! to the town, which starts it once a building of the right type is free
//! and the money is there, and `wait_upgrades` waits until every queued
//! request has started.  Before, each `upgrade` held the script until the
//! research could start, so a busy Engineering Bay held the thread through
//! the previous research and The Hammer Falls never attacked.  INFERENCE:
//! the 24-tick retry cadence and the order (first queued, first tried).

use super::*;

/// One queued research request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Research {
    /// `upgrade <level> <id>`: raise the upgrade to `level`.
    Upgrade { id: u8, level: u8 },
    /// `tech <id>`.
    Tech { id: u8 },
}

impl CampaignAi {
    /// Opcode 6 (`build`): keep the largest count asked for each type and
    /// try at once.
    pub(super) fn request_build(&mut self, world: &mut World, ins: &Instruction) {
        let (qty, unit) = (ins.byte(0).unwrap_or(1), ins.word(1).unwrap_or(0));
        match self.build_requests.iter_mut().find(|(t, _)| *t == unit) {
            Some(r) => r.1 = r.1.max(qty),
            None => self.build_requests.push((unit, qty)),
        }
        self.fill_build_request(world, unit, qty);
    }

    fn fill_build_request(&self, world: &mut World, unit: u16, qty: u8) {
        let me = self.player;
        let en_route = world
            .units
            .iter()
            .filter(|u| {
                u.alive
                    && u.owner == me
                    && matches!(u.order, crate::sim::Order::Build { unit_type, .. } if unit_type == unit)
            })
            .count() as i32;
        if self.count(world, unit) + self.training_count(world, unit) + en_route >= qty as i32 {
            return;
        }
        if world.data.unit(unit).is_building() {
            let _ = self.try_build(world, unit);
        } else {
            let _ = self.try_train_or_morph(world, unit);
        }
    }

    /// Opcodes 7 (`upgrade`) and 8 (`tech`): queue the request, try it at
    /// once, never block.
    pub(super) fn request_research(&mut self, world: &mut World, ins: &Instruction) {
        let r = if ins.opcode == 7 {
            Research::Upgrade {
                id: ins.word(1).unwrap_or(0) as u8,
                level: ins.byte(0).unwrap_or(1),
            }
        } else {
            Research::Tech {
                id: ins.word(0).unwrap_or(0) as u8,
            }
        };
        if !self.pending_research.contains(&r) && !self.research_done(world, r) {
            self.pending_research.push(r);
        }
        self.try_pending_research(world);
    }

    /// Every 24 ticks: start what can be started (also in child threads).
    pub(super) fn start_pending_research(&mut self, world: &mut World) {
        if (world.tick + self.player as u64).is_multiple_of(24) {
            self.try_pending_research(world);
            for (unit, qty) in self.build_requests.clone() {
                self.fill_build_request(world, unit, qty);
            }
            // `defensebuild_*`: keep the defenders asked for.
            for (qty, unit) in self.defense_requests.into_iter().flatten() {
                if self.count(world, unit) + self.training_count(world, unit) < qty as i32 {
                    let _ = self.try_train(world, unit);
                }
            }
        }
        for t in &mut self.subthreads {
            t.start_pending_research(world);
        }
    }

    fn try_pending_research(&mut self, world: &mut World) {
        let pending = std::mem::take(&mut self.pending_research);
        for r in pending {
            let started = self.research_done(world, r)
                || match r {
                    Research::Upgrade { id, .. } => self.try_upgrade(world, id),
                    Research::Tech { id } => self.try_tech(world, id),
                };
            if !started {
                self.pending_research.push(r);
            }
        }
    }

    fn research_done(&self, world: &World, r: Research) -> bool {
        let p = &world.players[self.player as usize];
        match r {
            Research::Upgrade { id, level } => p.upgrade_level(id) >= level,
            Research::Tech { id } => p.has_tech(id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::Race;
    use std::sync::Arc;

    #[test]
    fn a_build_the_town_cannot_afford_yet_is_kept() {
        let mut world = World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        );
        world.players[0].race = Race::Terran;
        world.players[0].minerals = 0;
        world.spawn_px(0, uid::COMMAND_CENTER, 320, 320).unwrap();
        world.spawn_px(0, uid::SCV, 420, 420).unwrap();
        // build 1 Barracks; wait_build 1 Barracks; stop.
        let script = AiScript {
            id: *b"Bld1",
            label: 1,
            flags: 0,
            code: vec![6, 1, 111, 0, 80, 9, 1, 111, 0, 36],
            external: false,
        };
        let mut ai = CampaignAi::new(0, &script);
        for _ in 0..48 {
            ai.update(&mut world, None);
            world.step();
        }
        world.players[0].minerals = 500;
        for _ in 0..3000 {
            ai.update(&mut world, None);
            world.step();
        }
        assert!(ai.finished, "the Barracks got built once the money came");
    }

    #[test]
    fn defensebuild_does_not_hold_the_script() {
        let mut world = World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        );
        world.players[0].race = Race::Terran;
        world.players[0].minerals = 0;
        world.spawn_px(0, uid::COMMAND_CENTER, 320, 320).unwrap();
        // defensebuild_gg 1 Marine with no Barracks; stop.
        let script = AiScript {
            id: *b"Def1",
            label: 1,
            flags: 0,
            code: vec![19, 1, 0, 0, 36],
            external: false,
        };
        let mut ai = CampaignAi::new(0, &script);
        ai.update(&mut world, None);
        assert!(ai.finished, "the script ran past the defence request");
        assert_eq!(ai.defense_requests[0], Some((1, uid::MARINE)));
    }

    #[test]
    fn upgrades_queue_and_the_script_goes_on() {
        let mut world = World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        );
        world.players[0].race = Race::Terran;
        world.players[0].minerals = 5000;
        world.players[0].gas = 5000;
        world.spawn_px(0, uid::COMMAND_CENTER, 320, 320).unwrap();
        world.spawn_px(0, uid::ENGINEERING_BAY, 600, 320).unwrap();
        // upgrade 1 Infantry Weapons (7); upgrade 1 Infantry Armor (0):
        // one Engineering Bay, so the second waits; stop.
        let script = AiScript {
            id: *b"Rsch",
            label: 1,
            flags: 0,
            code: vec![7, 1, 7, 0, 30, 7, 1, 0, 0, 30, 36],
            external: false,
        };
        let mut ai = CampaignAi::new(0, &script);
        ai.update(&mut world, None);
        assert!(ai.finished, "the script ran past both upgrades");
        assert_eq!(
            ai.pending_research.len(),
            1,
            "the busy bay leaves one queued"
        );
    }
}

#[cfg(test)]
mod nuke_tests {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use std::sync::Arc;

    #[test]
    fn create_nuke_arms_one_silo() {
        // Emperor's Fall (Ground Zero): TB2A calls create_nuke so that NuHe's
        // nuke_location can launch at ~18 s; retail `0x6421ff` arms the
        // first of the player's silos that has no missile.
        let mut world = World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(32, 32),
            2,
        );
        let a = world.spawn_px(0, uid::NUCLEAR_SILO, 200, 200).unwrap();
        let b = world.spawn_px(0, uid::NUCLEAR_SILO, 400, 200).unwrap();
        let script = AiScript {
            id: *b"Nuk2",
            label: 0,
            flags: 0,
            code: vec![107, 36],
            external: false,
        };
        let mut ai = CampaignAi::new(0, &script);
        ai.update(&mut world, None);
        assert_eq!(
            [world.units[a].nuke_ready, world.units[b].nuke_ready],
            [true, false]
        );
    }
}
