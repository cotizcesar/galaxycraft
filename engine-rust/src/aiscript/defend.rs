//! The campaign AI's defence response.

use super::*;

impl CampaignAi {
    /// `defenseuse_gg/ag/ga/aa` (opcodes 23-26) name the units sent against
    /// an enemy attacking the player; the slot is chosen by the layers of
    /// the attacked unit and the attacker (g ground, a air).  INFERENCE on
    /// the cadence (every 24 ticks) and on sending only idle defenders:
    /// retail's defence manager is in the unreadable part of the corpus.
    pub(super) fn defend(&mut self, world: &mut World) {
        if self.defense_units.iter().all(Option::is_none)
            || !(world.tick + self.player as u64).is_multiple_of(24)
        {
            return;
        }
        let me = self.player;
        let attack = world.units.iter().enumerate().find_map(|(ai, a)| {
            let victim = a.target.and_then(|t| world.index_of(t))?;
            let v = &world.units[victim];
            (a.alive && v.alive && v.owner == me && world.hostile(a.owner, me))
                .then_some((ai, victim))
        });
        let Some((attacker, victim)) = attack else {
            return;
        };
        let slot = world.is_air_unit(victim) as usize + 2 * world.is_air_unit(attacker) as usize;
        let Some((_, unit)) = self.defense_units[slot] else {
            return;
        };
        let target = world.units[attacker].id;
        let defenders: Vec<u32> = world
            .units
            .iter()
            .filter(|u| {
                u.alive
                    && u.owner == me
                    && u.type_id == unit
                    && matches!(u.order, crate::sim::Order::Idle)
            })
            .map(|u| u.id)
            .collect();
        for id in defenders {
            let _ = world.issue(id, crate::sim::Order::AttackUnit { target });
        }
    }
}
