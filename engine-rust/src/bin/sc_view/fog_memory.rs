//! Fog of war memory: enemy buildings the local player has scouted stay
//! drawn where they were seen, darkened, and on the minimap after the
//! scout leaves (`App::seen_buildings`).

use super::*;

impl App {
    /// Remember every enemy building in sight this frame.
    pub(super) fn remember_seen_buildings(&mut self) {
        let local = self.local_player as u8;
        for (i, u) in self.world.units.iter().enumerate() {
            if u.alive
                && u.owner != local
                && self.data.unit(u.type_id).is_building()
                && self.world.can_see_unit(local, i)
            {
                self.seen_buildings.insert(u.id);
            }
        }
    }

    /// Minimap blips of scouted buildings the fog hides now.
    pub(super) fn remembered_minimap_dots(&self) -> Vec<galaxycraft::render::MinimapDot> {
        if !self.fog_on() {
            return Vec::new();
        }
        let local = self.local_player as u8;
        self.world
            .units
            .iter()
            .enumerate()
            .filter(|(i, u)| {
                u.alive
                    && self.seen_buildings.contains(&u.id)
                    && !self.world.can_see_unit(local, *i)
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
}
