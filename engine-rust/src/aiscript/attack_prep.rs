//! `attack_prepare`: build the attack party and gather it towards the target.

use super::*;

impl CampaignAi {
    /// After `attack_prepare` (opcode 13) the party named by `attack_add`
    /// is trained when short and walks out towards the target, so
    /// `attack_do` finds it close.  Retail evidence (Desperate Alliance,
    /// live client 2026-11-13, player idle): the script waits 750, adds two
    /// Zerglings, prepares, waits 500 and attacks; the Zerglings were
    /// already at the player's ramp at frame ~1170 (before 750 + 500) and
    /// hit at ~1280.  INFERENCE: the gathering point (three quarters of the
    /// way from the AI's town to the target) and the 24-tick cadence.
    pub(super) fn gather_attack(&mut self, world: &mut World, enemy_base: Option<(i32, i32)>) {
        if !self.attack_prepared
            || self.attack_group.is_empty()
            || world.tick < self.attack_finish
            || !(world.tick + self.player as u64).is_multiple_of(24)
        {
            return;
        }
        let mut want: HashMap<u16, i32> = HashMap::new();
        for t in &self.attack_group {
            *want.entry(*t).or_default() += 1;
        }
        let mut types: Vec<(u16, i32)> = want.into_iter().collect();
        types.sort();
        for (t, n) in types {
            if self.count(world, t) + self.training_count(world, t) < n {
                let _ = self.try_train(world, t);
            }
        }
        let Some(staging) = self.staging(world, enemy_base) else {
            return;
        };
        let idle: Vec<u32> = self
            .party(world, staging)
            .into_iter()
            .filter(|&id| {
                world
                    .unit(id)
                    .is_some_and(|u| matches!(u.order, crate::sim::Order::Idle))
            })
            .collect();
        world.command_attack_move(&idle, staging);
    }

    /// Gathering point: three quarters of the way from the town to the
    /// target (INFERENCE).
    fn staging(&self, world: &World, enemy_base: Option<(i32, i32)>) -> Option<(i32, i32)> {
        let (bx, by) = self.base(world)?;
        let (tx, ty) = enemy_base.or_else(|| {
            self.nearest_enemy_unit(world)
                .map(|(x, y)| (x / 256, y / 256))
        })?;
        Some((bx + (tx - bx) * 3 / 4, by + (ty - by) * 3 / 4))
    }

    /// The attack party: per `attack_add` type, the fighters nearest the
    /// gathering point (so the ones already sent stay in it).
    fn party(&self, world: &World, staging: (i32, i32)) -> Vec<u32> {
        let mut want: HashMap<u16, usize> = HashMap::new();
        for t in &self.attack_group {
            *want.entry(*t).or_default() += 1;
        }
        let me = self.player;
        let mut near: Vec<(i64, u16, u32)> = world
            .units
            .iter()
            .filter(|u| u.alive && u.completed && u.owner == me && want.contains_key(&u.type_id))
            .map(|u| {
                let (dx, dy) = (
                    (u.x / 256 - staging.0) as i64,
                    (u.y / 256 - staging.1) as i64,
                );
                (dx * dx + dy * dy, u.type_id, u.id)
            })
            .collect();
        near.sort();
        near.into_iter()
            .filter(|(_, t, _)| match want.get_mut(t) {
                Some(n) if *n > 0 => {
                    *n -= 1;
                    true
                }
                _ => false,
            })
            .map(|(_, _, id)| id)
            .collect()
    }

    /// Who `attack_do` sends: the gathered party after `attack_prepare`,
    /// else the town's fighters (`military_ids`).
    pub(super) fn attackers(&self, world: &World, enemy_base: Option<(i32, i32)>) -> Vec<u32> {
        match self.staging(world, enemy_base) {
            Some(s) if self.attack_prepared && !self.attack_group.is_empty() => {
                self.party(world, s)
            }
            _ => self.military_ids(world),
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
    fn attack_prepare_walks_only_the_party_out() {
        let mut world = World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        );
        world.players[0].race = Race::Zerg;
        world.spawn_px(0, uid::HATCHERY, 320, 320).unwrap();
        for k in 0..6 {
            world.spawn_px(0, uid::ZERGLING, 420 + k * 20, 420).unwrap();
        }
        // attack_add 2 Zergling; attack_prepare; wait 500; stop.
        let script = AiScript {
            id: *b"Prep",
            label: 1,
            flags: 0,
            code: vec![12, 2, 37, 0, 13, 3, 0xF4, 0x01, 36],
            external: false,
        };
        let mut ai = CampaignAi::new(0, &script);
        for _ in 0..300 {
            ai.update(&mut world, Some((1800, 1800)));
            world.step();
        }
        let out = world
            .units
            .iter()
            .filter(|u| u.alive && u.type_id == uid::ZERGLING && u.x / 256 > 800)
            .count();
        assert_eq!(
            out, 2,
            "the two-Zergling party gathers, the rest stays home"
        );
    }
}
