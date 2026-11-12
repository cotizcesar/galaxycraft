//! `impl World`: creep power (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

impl World {
    /// Recompute the creep mask from completed Zerg creep sources.  Called
    /// lazily; sources are Hatcheries/Lairs/Hives and the three colony
    /// buildings.  Creep covers walkable tiles within three tiles of the
    /// building footprint.
    pub(super) fn rebuild_creep(&mut self) {
        if !self.creep_dirty {
            return;
        }
        let w = self.terrain.width as i32;
        let h = self.terrain.height as i32;
        self.creep_supported.iter_mut().for_each(|s| *s = false);
        let mut seed = vec![false; self.creep.len()];
        // Tiles under buildings and resources never take creep (retail
        // `flag_occupied`); the source's own footprint is seeded anyway.
        let mut occupied = vec![false; self.creep.len()];
        let footprint = |data: &GameData, u: &SimUnit| {
            let ty = data.unit(u.type_id);
            let (pw, ph) = (ty.placement.x as i32 / 32, ty.placement.y as i32 / 32);
            let (x0, y0) = (u.x / fp(1) / 32 - pw / 2, u.y / fp(1) / 32 - ph / 2);
            (x0, y0, pw, ph)
        };
        for u in &self.units {
            if !u.alive || !(self.data.unit(u.type_id).is_building() || is_resource_type(u.type_id))
            {
                continue;
            }
            let (x0, y0, pw, ph) = footprint(&self.data, u);
            for ty in y0.max(0)..(y0 + ph).min(h) {
                for tx in x0.max(0)..(x0 + pw).min(w) {
                    occupied[(ty * w + tx) as usize] = true;
                }
            }
        }
        for u in &self.units {
            if !u.alive || !u.completed || !is_creep_source(u.type_id) {
                continue;
            }
            // Retail `spread_creep_completely` (OpenBW): seed the footprint,
            // then grow 8-connected over tiles that can have creep, are not
            // occupied and lie in the ellipse 25 dx^2 + 64 dy^2 <= 25 * 320^2
            // (tile centre to building position) inside pos +- (320, 200).
            let (cx, cy) = (u.x / fp(1), u.y / fp(1));
            let (x0, y0, pw, ph) = footprint(&self.data, u);
            let mut stack = Vec::new();
            for ty in y0.max(0)..(y0 + ph).min(h) {
                for tx in x0.max(0)..(x0 + pw).min(w) {
                    let i = (ty * w + tx) as usize;
                    seed[i] = true;
                    if self.terrain.tile_can_have_creep(tx, ty) && !self.creep_supported[i] {
                        self.creep_supported[i] = true;
                        stack.push((tx, ty));
                    }
                }
            }
            let bx0 = ((cx - CREEP_RX).max(0)) / 32;
            let by0 = ((cy - CREEP_RY).max(0)) / 32;
            let bx1 = ((cx + CREEP_RX) / 32).min(w - 1);
            let by1 = ((cy + CREEP_RY) / 32).min(h - 1);
            let in_area = |tx: i32, ty: i32| {
                if tx < bx0 || ty < by0 || tx > bx1 || ty > by1 {
                    return false;
                }
                let dx = (tx * 32 + 16 - cx) as i64;
                let dy = (ty * 32 + 16 - cy) as i64;
                dx * dx * 25 + dy * dy * 64 <= (CREEP_RX as i64).pow(2) * 25
            };
            while let Some((tx, ty)) = stack.pop() {
                for (dx, dy) in [
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                    (-1, 0),
                    (1, 0),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                ] {
                    let (nx, ny) = (tx + dx, ty + dy);
                    if !in_area(nx, ny) {
                        continue;
                    }
                    let i = (ny * w + nx) as usize;
                    if self.creep_supported[i]
                        || occupied[i]
                        || !self.terrain.tile_can_have_creep(nx, ny)
                    {
                        continue;
                    }
                    self.creep_supported[i] = true;
                    stack.push((nx, ny));
                }
            }
        }
        for (i, &is_seed) in seed.iter().enumerate() {
            if self.creep_supported[i] {
                if is_seed && self.creep_strength[i] == 0 {
                    self.creep_strength[i] = 3;
                }
            } else {
                // A source died: the tile recedes one step per rebuild.
                self.creep_strength[i] = self.creep_strength[i].saturating_sub(1);
            }
            self.creep[i] = self.creep_strength[i] > 0;
        }
        self.creep_dirty = false;
    }

    /// Spread/recede creep one maturity step.  Runs every
    /// `CREEP_GROW_INTERVAL` logical ticks.
    pub(super) fn grow_creep(&mut self) {
        self.rebuild_creep();
        let w = self.terrain.width as i32;
        let h = self.terrain.height as i32;
        let mut next = self.creep_strength.clone();
        for tile_y in 0..h {
            for tile_x in 0..w {
                let i = (tile_y * w + tile_x) as usize;
                if !self.creep_supported[i] {
                    next[i] = self.creep_strength[i].saturating_sub(1);
                    continue;
                }
                if self.creep_strength[i] >= 3 {
                    continue;
                }
                let mut adjacent = false;
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = tile_x + dx;
                    let ny = tile_y + dy;
                    if nx >= 0
                        && ny >= 0
                        && nx < w
                        && ny < h
                        && self.creep_strength[(ny * w + nx) as usize] > 0
                    {
                        adjacent = true;
                        break;
                    }
                }
                if adjacent {
                    next[i] = (self.creep_strength[i] + 1).min(3);
                }
            }
        }
        self.creep_strength = next;
        for i in 0..self.creep.len() {
            self.creep[i] = self.creep_strength[i] > 0;
        }
    }

    pub fn has_creep_tile(&mut self, tx: i32, ty: i32) -> bool {
        self.rebuild_creep();
        let w = self.terrain.width as i32;
        let h = self.terrain.height as i32;
        if tx < 0 || ty < 0 || tx >= w || ty >= h {
            return false;
        }
        self.creep[(ty * w + tx) as usize]
    }

    /// Per-tile creep maturity (`0..=3`) for renderers.  Rebuilds the mask
    /// first so a source that appeared or died this tick is reflected without
    /// waiting for the next `CREEP_GROW_INTERVAL` boundary.
    /// Spread every source's creep to its full extent at once: a melee
    /// game starts with the start Hatchery's creep already grown (134905).
    pub fn mature_creep(&mut self) {
        self.creep_dirty = true;
        self.rebuild_creep();
        for i in 0..self.creep.len() {
            if self.creep_supported[i] {
                self.creep_strength[i] = 3;
                self.creep[i] = true;
            }
        }
    }

    pub fn creep_grid(&mut self) -> &[u8] {
        self.rebuild_creep();
        &self.creep_strength
    }

    /// Creep maturity at one tile, or zero outside the map.
    pub fn creep_strength_at(&mut self, tx: i32, ty: i32) -> u8 {
        self.rebuild_creep();
        let w = self.terrain.width as i32;
        let h = self.terrain.height as i32;
        if tx < 0 || ty < 0 || tx >= w || ty >= h {
            return 0;
        }
        self.creep_strength[(ty * w + tx) as usize]
    }

    pub fn has_creep_px(&mut self, x: i32, y: i32) -> bool {
        self.has_creep_tile(x.div_euclid(32), y.div_euclid(32))
    }

    /// Protoss power field: a point is powered by one of `owner`'s
    /// completed Pylons when it falls in that Pylon's psi field
    /// ([`psi_field_covers`]).  It was a 12 x 12 tile square that any
    /// player's Pylon could provide.
    pub fn has_power_px(&self, owner: u8, x: i32, y: i32) -> bool {
        self.units.iter().any(|u| {
            u.alive
                && u.completed
                && u.owner == owner
                && is_pylon_type(u.type_id)
                && psi_field_covers(x - u.x / fp(1), y - u.y / fp(1))
        })
    }

    /// A Protoss building functions only inside a power field.  Nexus and
    /// Pylon are exempt (they are the field providers themselves).
    pub fn building_powered(&self, i: usize) -> bool {
        let u = &self.units[i];
        if !is_protoss_building(u.type_id) || matches!(u.type_id, uid::NEXUS | uid::PYLON) {
            return true;
        }
        self.has_power_px(u.owner, u.x / fp(1), u.y / fp(1))
    }
}
