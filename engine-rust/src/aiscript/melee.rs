//! Retail skirmish computers: each runs its race's Brood War melee script
//! from `aiscript.bin` (`TMCx`, `ZMCx`, `PMCx`) on the town manager.

use super::CampaignAi;
use crate::sim::{Order, Race, SimUnit, World};

/// The melee script of a race (Brood War: the `*MCx` "expansion custom
/// level" entries).
pub fn melee_script_id(race: Race) -> Option<[u8; 4]> {
    match race {
        Race::Terran => Some(*b"TMCx"),
        Race::Zerg => Some(*b"ZMCx"),
        Race::Protoss => Some(*b"PMCx"),
        Race::Neutral => None,
    }
}

/// Whether a running script is one of the melee scripts.
pub fn is_melee_script(id: &[u8; 4]) -> bool {
    matches!(
        id,
        b"TMCx" | b"ZMCx" | b"PMCx" | b"TMCu" | b"ZMCu" | b"PMCu"
    )
}

/// A melee computer for `player`, by its race.
pub fn melee_ai(world: &World, player: u8) -> Option<CampaignAi> {
    let id = melee_script_id(world.players.get(player as usize)?.race)?;
    let script = world.data.campaign_ai_script(&id)?;
    let mut ai = CampaignAi::new(player, script);
    // The engine's roster army (`maintain_army`) is not retail; a melee
    // script spends only on what it asks for.
    ai.default_build = false;
    Some(ai)
}

/// Command Center, Nexus, Hatchery and its morphs.
pub(super) fn is_town_hall(t: u16) -> bool {
    use crate::ids::unit as uid;
    matches!(
        t,
        uid::COMMAND_CENTER | uid::NEXUS | uid::HATCHERY | uid::LAIR | uid::HIVE
    )
}

/// How far (pixels) a town hall claims a resource area.
const TOWN_RADIUS: i32 = 12 * 32;

/// A worker on a construction errand (never reassigned to gas).
pub(super) fn is_building_order(order: &Order) -> bool {
    matches!(order, Order::Build { .. } | Order::Construct { .. })
}

/// Parent building of a Terran add-on.
fn addon_parent(addon: u16) -> Option<u16> {
    use crate::ids::unit as uid;
    Some(match addon {
        uid::COMSAT_STATION | uid::NUCLEAR_SILO => uid::COMMAND_CENTER,
        uid::MACHINE_SHOP => uid::FACTORY,
        uid::CONTROL_TOWER => uid::STARPORT,
        uid::COVERT_OPS | uid::PHYSICS_LAB => uid::SCIENCE_FACILITY,
        _ => return None,
    })
}

impl CampaignAi {
    /// Producer types to keep idle (no new training on one without its
    /// add-on) while this thread, its parent or a child asks for an add-on
    /// it lacks: an add-on starts only on an idle building, and the
    /// threads' unit requests would otherwise keep it busy forever.
    pub(super) fn reserve_addon_parents(&mut self, world: &World) {
        // The root starts afresh; a child keeps what its parent set this tick.
        if !self.child {
            self.reserved.clear();
        }
        let mut own = self.pending_addon_parents(world);
        for t in &self.subthreads {
            own.extend(t.pending_addon_parents(world));
        }
        own.extend(self.reserved.iter().copied());
        own.sort_unstable();
        own.dedup();
        self.reserved = own;
        for t in &mut self.subthreads {
            t.reserved = self.reserved.clone();
        }
    }

    fn pending_addon_parents(&self, world: &World) -> Vec<u16> {
        self.build_requests
            .iter()
            .filter(|&&(unit, qty)| self.count(world, unit) < qty as i32)
            .filter_map(|&(unit, _)| addon_parent(unit))
            .collect()
    }

    /// A finished producer of `unit` with an empty queue: the engine's
    /// default army only fills idle producers, so the script's own `train`
    /// requests are never starved.
    pub(super) fn has_idle_producer(&self, world: &World, unit: u16) -> bool {
        let Some(info) = crate::tech::build_info(unit) else {
            return false;
        };
        world.units.iter().any(|u| {
            u.alive
                && u.completed
                && u.owner == self.player
                && u.type_id == info.producer
                && u.production.is_empty()
        })
    }

    /// Opcode 5 `expand <n> <block>`: start the block as a town of its own
    /// on the free resource area nearest the main base.  INFERENCE: retail
    /// picks the area from its region data; nearest free mineral field here.
    pub(super) fn spawn_expansion(&mut self, world: &World, towns: u8, block: u16) {
        let taken: Vec<(i32, i32)> = self
            .subthreads
            .iter()
            .filter_map(|t| t.expansion_site)
            .collect();
        // `expand <n>`: a new town only while the player has fewer than n.
        if 1 + taken.len() >= towns as usize || self.subthreads.len() >= 16 {
            return;
        }
        let Some(site) = self.free_resource_area(world, &taken) else {
            return;
        };
        let mut child = CampaignAi::at_offset(self.player, &self.script, block as usize);
        child.campaign = self.campaign;
        child.default_build = self.default_build;
        child.expansion_site = Some(site);
        child.child = true;
        // The town takes the gas on its site.  INFERENCE: the retail town
        // manager builds it; the expansion block does not ask for it.
        let gas = match world.players[self.player as usize].race {
            crate::sim::Race::Terran => crate::ids::unit::REFINERY,
            crate::sim::Race::Protoss => crate::ids::unit::ASSIMILATOR,
            _ => crate::ids::unit::EXTRACTOR,
        };
        if world.units.iter().any(|u| {
            u.alive
                && u.type_id == crate::ids::unit::VESPENE_GEYSER
                && (u.x / 256 - site.0).abs() <= TOWN_RADIUS
                && (u.y / 256 - site.1).abs() <= TOWN_RADIUS
        }) {
            child.build_requests.push((gas, 1));
        }
        self.subthreads.push(Box::new(child));
    }

    fn free_resource_area(&self, world: &World, taken: &[(i32, i32)]) -> Option<(i32, i32)> {
        let (bx, by) = self.base(world)?;
        let near = |a: (i32, i32), b: (i32, i32)| {
            (a.0 - b.0).abs() <= TOWN_RADIUS && (a.1 - b.1).abs() <= TOWN_RADIUS
        };
        let halls: Vec<(i32, i32)> = world
            .units
            .iter()
            .filter(|u| u.alive && is_town_hall(u.type_id))
            .map(|u| (u.x / 256, u.y / 256))
            .collect();
        world
            .units
            .iter()
            .filter(|u| {
                u.alive
                    && matches!(u.type_id, 176..=178)
                    && u.resource_amount > 0
                    && !halls
                        .iter()
                        .chain(taken)
                        .any(|&h| near(h, (u.x / 256, u.y / 256)))
            })
            .map(|u| (u.x / 256, u.y / 256))
            .min_by_key(|&(x, y)| ((x - bx) as i64).pow(2) + ((y - by) as i64).pow(2))
    }

    /// The player's units of `unit`'s kind (any town hall stage, or that
    /// type) on a site, finished or not.
    pub(super) fn owned_near(&self, world: &World, site: (i32, i32), unit: u16) -> i32 {
        let hall = is_town_hall(unit);
        world
            .units
            .iter()
            .filter(|u| {
                u.alive
                    && u.owner == self.player
                    && (u.type_id == unit || hall && is_town_hall(u.type_id))
                    && (u.x / 256 - site.0).abs() <= TOWN_RADIUS
                    && (u.y / 256 - site.1).abs() <= TOWN_RADIUS
            })
            .count() as i32
    }

    /// Three workers on every finished refinery of the town (the retail
    /// town manager staffs each gas building, not only the first one).
    pub(super) fn staff_gas(&self, world: &mut World, worker_type: u16) {
        let me = self.player;
        let gas: Vec<u32> = world
            .units
            .iter()
            .filter(|u| {
                u.alive
                    && u.owner == me
                    && u.completed
                    && crate::sim::is_gas_building(u.type_id)
                    && u.resource_amount > 0
            })
            .map(|u| u.id)
            .collect();
        let on = |u: &SimUnit, g: u32| {
            u.resource_target == Some(g)
                || matches!(u.order, Order::Gather { resource } if resource == g)
        };
        let mut free: Vec<(bool, u32)> = world
            .units
            .iter()
            .filter(|u| u.alive && u.owner == me && u.type_id == worker_type && u.completed)
            .filter(|u| {
                u.carry_kind != crate::sim::CarryKind::Gas
                    && !is_building_order(&u.order)
                    && !gas.iter().any(|&g| on(u, g))
            })
            .map(|u| (u.order != Order::Idle, u.id))
            .collect();
        free.sort_unstable();
        // Minerals first: at most a third of the workers on gas.
        // INFERENCE (retail keeps a gas/mineral split in its town manager).
        let total = world
            .units
            .iter()
            .filter(|u| u.alive && u.owner == me && u.type_id == worker_type)
            .count();
        let on_gas = world
            .units
            .iter()
            .filter(|u| u.alive && u.owner == me && u.type_id == worker_type)
            .filter(|u| gas.iter().any(|&g| on(u, g)))
            .count();
        let mut budget = (total / 3).saturating_sub(on_gas);
        let mut free = free.into_iter();
        for g in gas {
            let staffed = world
                .units
                .iter()
                .filter(|u| u.alive && u.owner == me && u.type_id == worker_type && on(u, g))
                .count();
            for _ in staffed..3 {
                if budget == 0 {
                    return;
                }
                budget -= 1;
                let Some((_, id)) = free.next() else {
                    return;
                };
                let _ = world.issue(id, Order::Gather { resource: g });
            }
        }
    }

    /// The worker that takes a construction order: an idle one first,
    /// otherwise a gatherer with empty hands, as the retail town takes a
    /// mining worker (melee towns have no idle workers).
    pub(super) fn builder<'a>(&self, world: &'a World, worker_type: u16) -> Option<&'a SimUnit> {
        let free = |u: &&SimUnit, gathering: bool| {
            u.alive
                && u.owner == self.player
                && u.type_id == worker_type
                && u.completed
                && u.carry_kind == crate::sim::CarryKind::None
                && match u.order {
                    Order::Idle => true,
                    Order::Gather { .. } => gathering,
                    _ => false,
                }
        };
        let units = &world.units;
        units
            .iter()
            .find(|u| free(u, false))
            .or_else(|| units.iter().find(|u| free(u, true)))
    }
}
