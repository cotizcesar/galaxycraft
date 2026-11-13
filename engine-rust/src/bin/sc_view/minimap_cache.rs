//! The HD minimap's terrain + fog layer, cached (`App::minimap_cache`).

use super::*;

impl App {
    /// Per-frame HD bookkeeping: open the sprite bake budget and, once per
    /// map and tile size, decode all of the map's terrain megatiles, so a
    /// scroll into unseen ground does not decode them on the spot (5 ms
    /// extra a frame at 1440p).
    pub(super) fn begin_hd_frame(&mut self, scale: f32) {
        let moving = LAST_CAMERA.with(|c| c.replace(self.camera) != self.camera);
        galaxycraft::hd_sprites::begin_frame(moving);
        let Some(t) = &self.hd_terrain else {
            return;
        };
        let size = (32.0 * scale).round().max(1.0) as usize;
        let tiles = &self.scenario.tiles;
        WARM.with(|w| {
            let key = (size, tiles.as_ptr() as usize, tiles.len());
            let (mut next, warmed) = w.get();
            if warmed != key {
                next = 0;
            }
            // Everything on the map's first HD frame (the game is starting):
            // spread over later frames it pushed busy ones past 16.7 ms.
            while next < tiles.len() {
                let mega = self.tileset.megatile(tiles[next]) as usize;
                let _ = t.tile(mega, size);
                next += 1;
            }
            w.set((next, key));
        });
    }

    /// Paint the HD minimap terrain under the smooth fog into `(rx, ry, rw,
    /// rh)` (134905: the terrain renderer at minimap scale, 3 px per tile at
    /// 1440p, creep included, so the visible area fades out like the
    /// playfield's).  `false` without HD terrain (the tile minimap is used).
    pub(super) fn prepaint_minimap(
        &mut self,
        fb: &mut Framebuffer,
        (rx, ry, rw, rh): (i32, i32, i32, i32),
    ) -> bool {
        if self.hd_terrain.is_none() {
            return false;
        }
        let creep = self.creep_lookup();
        let width = self.scenario.width;
        let creep_now: Vec<bool> = self.world.creep_grid().iter().map(|&c| c > 0).collect();
        let variants = self.creep_variants.len();
        let Some(t) = &self.hd_terrain else {
            return false;
        };
        // The fog is recomposed every `MINIMAP_REFRESH_TICKS` (1.6 ms
        // at 1080p); the terrain under it (41 ms) only when the creep
        // changes: redrawing both every 8 ticks hitched scrolling.
        let tick = self.world.tick;
        let fog = self.fog_on();
        let fresh = self.minimap_cache.as_ref().is_some_and(|c| {
            (c.size, c.fog) == ((rw, rh), fog)
                && tick >= c.at
                && tick - c.at < MINIMAP_REFRESH_TICKS
        });
        if !fresh {
            let (mw, _mh) = self.tileset.minimap_size;
            let s = rw as f32 / (mw.max(1) as f32 * 32.0);
            let terrain = match self.minimap_cache.take() {
                Some(mut c) if c.size == (rw, rh) && c.variants == variants => {
                    // Only the tiles whose creep changed (and their
                    // edges) are drawn again.
                    if let Some(b) = changed_tiles(&c.creep, &creep_now, width) {
                        redraw_tiles(
                            t,
                            &self.tileset,
                            &self.scenario,
                            &creep,
                            &mut c.terrain,
                            mw,
                            b,
                        );
                    }
                    c.terrain
                }
                _ => {
                    let mut tmp = Framebuffer::new(rw.max(1) as usize, rh.max(1) as usize);
                    t.draw(
                        &mut tmp,
                        &self.tileset,
                        &self.scenario,
                        (0, 0),
                        (0, 0, rw, rh),
                        s,
                        &creep,
                        None,
                    );
                    tmp
                }
            };
            let mut frame = Framebuffer::new(rw.max(1) as usize, rh.max(1) as usize);
            frame.pixels.copy_from_slice(&terrain.pixels);
            if fog {
                galaxycraft::render::draw_fog_smooth(
                    &mut frame,
                    &self.scenario,
                    &self.world,
                    self.local_player as u8,
                    (0, 0),
                    (0, 0, rw, rh),
                    s,
                );
            }
            self.minimap_cache = Some(MinimapCache {
                size: (rw, rh),
                at: tick,
                fog,
                frame,
                creep: creep_now,
                variants,
                terrain,
            });
        }
        if let Some(c) = &self.minimap_cache {
            for yy in 0..rh {
                for xx in 0..rw {
                    fb.put(rx + xx, ry + yy, c.frame.get(xx, yy));
                }
            }
        }
        true
    }
}

/// The HD minimap: the fog-covered frame shown (redone every
/// `MINIMAP_REFRESH_TICKS`) and the terrain under it, which only changes
/// where creep comes or goes (`creep`: the tiles with creep it was drawn
/// with; `variants`: the count of fixed creep tiles).
pub(super) struct MinimapCache {
    size: (i32, i32),
    at: u64,
    fog: bool,
    frame: Framebuffer,
    creep: Vec<bool>,
    variants: usize,
    terrain: Framebuffer,
}

/// Bounding box `(x0, y0, x1, y1)` (tiles, exclusive end) of the tiles whose
/// creep differs, grown by one tile for the creep edges they change.
fn changed_tiles(old: &[bool], new: &[bool], width: usize) -> Option<(i32, i32, i32, i32)> {
    if old.len() != new.len() || width == 0 {
        let h = (new.len() / width.max(1)) as i32;
        return Some((0, 0, width as i32, h));
    }
    let mut b: Option<(i32, i32, i32, i32)> = None;
    for (i, _) in old.iter().zip(new).enumerate().filter(|(_, (a, b))| a != b) {
        let (x, y) = ((i % width) as i32, (i / width) as i32);
        b = Some(match b {
            None => (x, y, x + 1, y + 1),
            Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1)),
        });
    }
    let h = (new.len() / width) as i32;
    b.map(|(x0, y0, x1, y1)| {
        (
            (x0 - 1).max(0),
            (y0 - 1).max(0),
            (x1 + 1).min(width as i32),
            (y1 + 1).min(h),
        )
    })
}

/// Redraw the minimap terrain over tiles `b` only.  The patch is rendered
/// with a 2-tile margin (creep edge frames spill over their tile) into a
/// scratch frame whose origin sits on a whole minimap pixel, so every tile
/// lands on the same pixels as in a full redraw; then the tiles of `b` are
/// copied in.
fn redraw_tiles(
    t: &galaxycraft::render::HdTerrain,
    tileset: &Tileset,
    scenario: &galaxycraft::map::Scenario,
    creep: &dyn Fn(i32, i32) -> Option<u16>,
    terrain: &mut Framebuffer,
    mw: usize,
    b: (i32, i32, i32, i32),
) {
    let rw = terrain.width;
    // Tiles per whole-pixel step: tx * rw / mw is an integer for tx % q == 0.
    let q = (mw / gcd(mw, rw as usize).max(1)).max(1) as i32;
    let (mw_i, mh) = (mw as i32, scenario.height as i32);
    let x0 = ((b.0 - 2).max(0) / q) * q;
    let y0 = ((b.1 - 2).max(0) / q) * q;
    let x1 = (b.2 + 2).min(mw_i);
    let y1 = (b.3 + 2).min(mh);
    let px = |tile: i32| tile * rw / mw_i;
    let s = rw as f32 / (mw.max(1) as f32 * 32.0);
    let (ox, oy) = (px(x0), px(y0));
    let (w, h) = (px(x1) - ox + 1, px(y1) - oy + 1);
    let mut patch = Framebuffer::new(w.max(1) as usize, h.max(1) as usize);
    t.draw(
        &mut patch,
        tileset,
        scenario,
        (x0 * 32, y0 * 32),
        (0, 0, w, h),
        s,
        creep,
        None,
    );
    let (cx0, cy0, cx1, cy1) = (
        px(b.0),
        px(b.1),
        px(b.2).min(rw),
        px(b.3).min(terrain.height),
    );
    for y in cy0..cy1 {
        for x in cx0..cx1 {
            terrain.put(x, y, patch.get(x - ox, y - oy));
        }
    }
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[cfg(test)]
mod minimap_cache_tests {
    use super::changed_tiles;

    #[test]
    fn creep_changes_give_their_box_grown_by_one_tile() {
        let old = vec![false; 64];
        assert_eq!(changed_tiles(&old, &old, 8), None);
        let mut new = old.clone();
        new[3 * 8 + 4] = true;
        new[5 * 8 + 6] = true;
        assert_eq!(changed_tiles(&old, &new, 8), Some((3, 2, 8, 7)));
        new[0] = true;
        assert_eq!(changed_tiles(&old, &new, 8), Some((0, 0, 8, 7)));
    }
}

thread_local! {
    /// Camera of the previous HD frame (a moving view defers sprite bakes).
    static LAST_CAMERA: std::cell::Cell<(i32, i32)> = const { std::cell::Cell::new((i32::MIN, 0)) };
    /// Terrain pre-decode cursor: next map tile, for `(tile size, map)`.
    static WARM: std::cell::Cell<(usize, (usize, usize, usize))> =
        const { std::cell::Cell::new((0, (0, 0, 0))) };
}
