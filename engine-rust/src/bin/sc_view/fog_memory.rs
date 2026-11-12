//! Fog of war memory: enemy buildings the local player has scouted stay
//! drawn as they were last seen, darkened, and on the minimap, until the
//! area is seen again; one destroyed out of sight stays as a ghost until
//! then (`App::fog_mem`).

use super::*;

/// Last-seen snapshots of scouted enemy buildings, and the units swapped
/// out for them while a frame is drawn.
#[derive(Default)]
pub(super) struct FogMemory {
    pub(super) seen: std::collections::HashMap<u32, galaxycraft::sim::SimUnit>,
    stash: (Vec<(usize, galaxycraft::sim::SimUnit)>, usize),
}

impl App {
    /// Snapshot every enemy building in sight; forget the ones whose spot
    /// is in sight again but which are gone or no longer seen there.
    pub(super) fn remember_seen_buildings(&mut self) {
        let local = self.local_player as u8;
        for (i, u) in self.world.units.iter().enumerate() {
            if u.alive
                && u.owner != local
                && self.data.unit(u.type_id).is_building()
                && self.world.can_see_unit(local, i)
            {
                self.fog_mem.seen.insert(u.id, u.clone());
            }
        }
        let world = &self.world;
        self.fog_mem.seen.retain(|id, snap| {
            let live = world.index_of(*id).filter(|&i| world.units[i].alive);
            let seen_now = live.is_some_and(|i| world.can_see_unit(local, i));
            let still_there =
                live.is_some_and(|i| (world.units[i].x, world.units[i].y) == (snap.x, snap.y));
            let spot_seen = world.is_visible_to(local, snap.x / 256, snap.y / 256);
            // Forgotten once its spot is in sight and it is gone from there.
            seen_now || still_there || !spot_seen
        });
    }

    /// For this frame's drawing: a remembered building out of sight shows
    /// its snapshot, and one destroyed out of sight comes back as a ghost.
    pub(super) fn fog_swap_in(&mut self) {
        self.remember_seen_buildings();
        if !self.fog_on() {
            return;
        }
        let local = self.local_player as u8;
        let mut swapped = Vec::new();
        let mut ghosts = Vec::new();
        for (id, snap) in &self.fog_mem.seen {
            match self
                .world
                .index_of(*id)
                .filter(|&i| self.world.units[i].alive)
            {
                Some(i) if !self.world.can_see_unit(local, i) => swapped.push((i, snap.clone())),
                Some(_) => {}
                None => {
                    let mut g = snap.clone();
                    g.alive = true;
                    ghosts.push(g);
                }
            }
        }
        for (i, snap) in &mut swapped {
            std::mem::swap(&mut self.world.units[*i], snap);
        }
        self.fog_mem.stash = (swapped, ghosts.len());
        self.world.units.extend(ghosts);
    }

    /// Put the live units back after drawing.
    pub(super) fn fog_swap_out(&mut self) {
        let (swapped, ghosts) = std::mem::take(&mut self.fog_mem.stash);
        let n = self.world.units.len() - ghosts;
        self.world.units.truncate(n);
        for (i, live) in swapped {
            self.world.units[i] = live;
        }
    }

    /// Minimap blips of remembered buildings out of sight.
    pub(super) fn remembered_minimap_dots(&self) -> Vec<galaxycraft::render::MinimapDot> {
        if !self.fog_on() {
            return Vec::new();
        }
        let local = self.local_player as u8;
        self.fog_mem
            .seen
            .iter()
            .filter(|(id, _)| {
                !self
                    .world
                    .index_of(**id)
                    .is_some_and(|i| self.world.units[i].alive && self.world.can_see_unit(local, i))
            })
            .map(|(_, u)| {
                let ty = self.data.unit(u.type_id);
                galaxycraft::render::MinimapDot {
                    x: u.x / 256,
                    y: u.y / 256,
                    owner: u.owner,
                    placement: (ty.placement.x as i32, ty.placement.y as i32),
                    building: true,
                    resource: false,
                    color: galaxycraft::render::player_rgb(&self.world, u.owner),
                }
            })
            .collect()
    }

    /// `edge:x:y` script step: the pointer rests on a screen edge of the
    /// focused window for one frame (edge scroll and its cursor).
    pub(super) fn script_edge(&mut self, x: i32, y: i32, size: (i32, i32)) -> Option<(i32, i32)> {
        let p = surface_point(self.present_offset, x, y);
        self.edge_scroll = edge_scroll_dir(p, size);
        Some(p)
    }
}
