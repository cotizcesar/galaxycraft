//! `impl World`: movement (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

/// A goal this close is approached in a straight line once the route ends.
const DIRECT_APPROACH_PX: i32 = 96;
/// Routes longer than this are shared within a tick (see `World::path_cache`).
const LONG_ROUTE_PX: i32 = 512;
/// `(start x / 128, start y / 128, goal x, goal y, half width, half height)`
/// of a long ground route: units of one size leaving the same 128 px square
/// for the same goal share it within a tick (an attack wave asked for a dozen
/// ~8 ms cross-map searches on one frame).
pub(super) type PathKey = (i32, i32, i32, i32, i32, i32);

/// Long routes searched this tick and how many were started (`World::routes`,
/// reset every tick).
#[derive(Default)]
pub(super) struct RouteCache {
    map: std::collections::HashMap<PathKey, Option<Vec<(i32, i32)>>>,
    searches: u32,
}
/// New long route searches allowed per tick.
const LONG_SEARCHES_PER_TICK: u32 = 6;

impl World {
    /// OpenBW `find_unit_placement`: the spot nearest to `center` (1/256
    /// px), within 128 px, where unit `j`'s box is on walkable ground and
    /// overlaps no other ground unit or building.  Searched on a 4 px grid,
    /// ring by ring.
    pub(super) fn free_spot_near(&self, j: usize, center: (i32, i32)) -> Option<(i32, i32)> {
        let ty = self.data.unit(self.units[j].type_id);
        let (cx, cy) = (center.0 / fp(1), center.1 / fp(1));
        let fits = |x: i32, y: i32| {
            let r = ty.dimensions.rect_at((x, y));
            if r.0 < 0 || r.1 < 0 {
                return false;
            }
            let corners = [(r.0, r.1), (r.2, r.1), (r.0, r.3), (r.2, r.3), (x, y)];
            if !corners
                .iter()
                .all(|&(px, py)| self.terrain.walkable_px(px, py))
            {
                return false;
            }
            !self.units.iter().enumerate().any(|(k, o)| {
                k != j && o.alive && o.in_transport.is_none() && !self.is_air_unit(k) && {
                    let b = self
                        .data
                        .unit(o.type_id)
                        .dimensions
                        .rect_at((o.x / fp(1), o.y / fp(1)));
                    r.0 <= b.2 && b.0 <= r.2 && r.1 <= b.3 && b.1 <= r.3
                }
            })
        };
        for ring in 0..=32 {
            let d = ring * 4;
            let mut best: Option<(i64, (i32, i32))> = None;
            for k in -ring..=ring {
                for (dx, dy) in [(k * 4, -d), (k * 4, d), (-d, k * 4), (d, k * 4)] {
                    let (x, y) = (cx + dx, cy + dy);
                    if fits(x, y) {
                        let dist = (dx as i64).pow(2) + (dy as i64).pow(2);
                        if best.is_none_or(|(bd, _)| dist < bd) {
                            best = Some((dist, (x, y)));
                        }
                    }
                }
            }
            if let Some((_, (x, y))) = best {
                return Some((x * fp(1), y * fp(1)));
            }
        }
        None
    }

    pub(super) fn exit_position(&self, i: usize) -> (i32, i32) {
        let u = &self.units[i];
        let ty = self.data.unit(u.type_id);
        let w = ty.dimensions.right.max(0) as i32 * fp(1);
        let h = ty.dimensions.bottom.max(0) as i32 * fp(1);
        (u.x + w / 2 + fp(8), u.y + h / 2 + fp(8))
    }

    // ------------------------------------------------------------------
    // Movement
    // ------------------------------------------------------------------

    pub(super) fn build_obstacles(&mut self) {
        self.obstacles.clear();
        for u in &self.units {
            if !u.alive || !u.completed {
                continue;
            }
            let ty = self.data.unit(u.type_id);
            if is_door_type(u.type_id) {
                if u.doodad_enabled.unwrap_or(false) {
                    self.obstacles
                        .push(unit_box_rect(ty, (u.x / fp(1), u.y / fp(1))));
                }
                continue;
            }
            if !u.lifted && (is_resource_type(u.type_id) || ty.is_building()) {
                let (l, t, r, b) = ty.dimensions.extents();
                if l + r > 0 || t + b > 0 {
                    let rect = unit_box_rect(ty, (u.x / fp(1), u.y / fp(1)));
                    self.obstacles.push(rect);
                }
            }
        }
    }

    /// Half extents of a ground unit's collision box in 1/256 pixels.
    pub(super) fn collision_extents(&self, i: usize) -> Option<(i32, i32)> {
        let u = &self.units[i];
        if !u.alive || u.in_transport.is_some() || u.status.burrowed {
            return None;
        }
        if self.is_gathering(i) {
            return None;
        }
        let ty = self.data.unit(u.type_id);
        if ty.is_building() || ty.is_flyer() || is_door_type(u.type_id) {
            return None;
        }
        // `units.dat` dimensions are in pixels.  Every stock ground unit has
        // a non-zero box; the floor keeps tiny doodads from collapsing.
        let hw = ty.dimensions.right.max(ty.dimensions.left.max(0)) as i32;
        let hh = ty.dimensions.bottom.max(ty.dimensions.top.max(0)) as i32;
        Some(((hw.max(5)) * fp(1), (hh.max(5)) * fp(1)))
    }

    /// Air units do not collide, but they do not stack either (OpenBW
    /// `apply_repulse_field`).  The map is cut into 48 px cells; a flyer
    /// that shares its cell with another drifts half a pixel per frame
    /// along its repulse direction, faster the more crowded the cell when
    /// it is standing still, with a small per-unit bias (the adjust table)
    /// so a stack fans out instead of sliding as one.  Buildings,
    /// Interceptors and units that cannot be interrupted are left alone.
    pub(super) fn apply_air_repulse(&mut self) {
        const ADJUST: [i32; 32] = [
            -5, -5, -5, 5, 5, -5, 5, 5, 5, -5, 5, 5, -5, -5, -5, 5, -5, 5, 5, -5, -5, -5, 5, 5, -5,
            5, 5, -5, 5, 5, -5, -5,
        ];
        let movers: Vec<usize> = (0..self.units.len())
            .filter(|&i| {
                let u = &self.units[i];
                u.alive
                    && u.in_transport.is_none()
                    && u.type_id != uid::INTERCEPTOR
                    && u.type_id != uid::NUCLEAR_MISSILE
                    && !self.data.unit(u.type_id).is_building()
                    && self.is_air_unit(i)
            })
            .collect();
        if movers.len() < 2 {
            return;
        }
        let cell = |u: &SimUnit| (u.x / fp(48), u.y / fp(48));
        let mut field: HashMap<(i32, i32), i32> = HashMap::new();
        for &i in &movers {
            *field.entry(cell(&self.units[i])).or_default() += 1;
        }
        let (map_w, map_h) = (
            self.terrain.width as i32 * 32,
            self.terrain.height as i32 * 32,
        );
        for i in movers {
            let strength = field[&cell(&self.units[i])].min(255);
            if self.units[i].repulse_flags & 0xf0 == 0 {
                let v = self.world_rand();
                self.units[i].repulse_dir = (v & 0xff) as u8;
                let row = ((v >> 8) & 0xf0) as u8;
                self.units[i].repulse_flags = if row == 0 { 0xf0 } else { row };
            }
            if strength <= 1 {
                self.units[i].repulse_flags &= !7;
                continue;
            }
            if self.is_transforming(i)
                || self.units[i].status.lockdown_ticks > 0
                || self.units[i].status.stasis_ticks > 0
                || self.units[i].status.maelstrom_ticks > 0
            {
                continue;
            }
            let still = self.units[i].path.is_empty();
            if self.units[i].repulse_flags & 7 != 0 {
                self.units[i].repulse_flags -= 1;
            } else {
                self.units[i].repulse_flags |= 7;
                // `update_repulse_direction`: while moving, keep the drift
                // to the side of the travel direction; near a map edge,
                // turn it back inwards.
                let mut dir = self.units[i].repulse_dir;
                if let Some(&(wx, wy)) = self.units[i].path.front() {
                    let target = direction_byte(wx - self.units[i].x, wy - self.units[i].y) as i32;
                    let index = (dir as i32 - target).rem_euclid(256);
                    let side = index % 32 + if index < 128 { 32 } else { -64 };
                    dir = (target + side).rem_euclid(256) as u8;
                }
                let (x, y) = (self.units[i].x / fp(1), self.units[i].y / fp(1));
                // Directions 1..=127 point east, 129..=255 west; 65..=191
                // south, the rest north.
                let east = (1..128).contains(&dir);
                let south = (65..192).contains(&dir);
                if (x < 48 && !east && dir != 0 && dir != 128) || (x > map_w - 48 && east) {
                    dir = 0u8.wrapping_sub(dir);
                }
                if (y < 48 && !south) || (y > map_h - 48 && south) {
                    dir = 128u8.wrapping_sub(dir);
                }
                self.units[i].repulse_dir = dir;
            }
            let (vx, vy) = direction_vector(self.units[i].repulse_dir as i32);
            let (mut rx, mut ry) = (vx / 2, vy / 2);
            if still {
                rx = rx * (strength / 4 + 1) / 2;
                ry = ry * (strength / 4 + 1) / 2;
            }
            let row = (self.units[i].repulse_flags >> 4) as usize;
            let (ax, ay) = (ADJUST[row * 2], ADJUST[row * 2 + 1]);
            // Pull a strong component back by the bias and push a weak one
            // further (OpenBW's four-way compare, folded).
            let bias = |v: i32, a: i32| match v {
                0 => 0,
                v if v > 0 => {
                    if v > a * 4 {
                        v - a
                    } else {
                        v + a
                    }
                }
                v => {
                    if v < -a * 4 {
                        v + a
                    } else {
                        v - a
                    }
                }
            };
            rx = bias(rx, ax);
            ry = bias(ry, ay);
            let nx = (self.units[i].x + rx).clamp(fp(8), fp(map_w - 8));
            let ny = (self.units[i].y + ry).clamp(fp(8), fp(map_h - 8));
            self.units[i].x = nx;
            self.units[i].y = ny;
        }
    }

    /// Turrets (Siege Tank, Goliath) turn towards the unit's target, or
    /// back along the hull without one, by their flingy turn rate per
    /// frame (17 for the Goliath's guns, 13 for the tank cannons).  A
    /// freshly spawned or just-sieged turret starts along the hull.
    pub(super) fn turn_turrets(&mut self) {
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if !u.alive {
                continue;
            }
            let ty = self.data.unit(u.type_id);
            let Some(turret) = self.data.turret(ty) else {
                continue;
            };
            let rate = self
                .data
                .flingy(turret.flingy)
                .map_or(255, |f| f.turn_rate as i32)
                .max(1);
            // At rest: along the hull, or the sieged cannon's own rest
            // heading once it has unfolded.
            let rest = if u.type_id == uid::SIEGE_MODE
                && !(matches!(u.order, Order::Siege) && u.order_wait > 0)
            {
                SIEGED_TURRET_HEADING
            } else {
                u.facing as i32
            };
            let want = u
                .target
                .and_then(|t| self.index_of(t))
                .filter(|&t| self.units[t].alive)
                .map_or(rest, |t| {
                    direction_byte(self.units[t].x - u.x, self.units[t].y - u.y) as i32
                });
            let cur = u.turret_facing as i32;
            let mut diff = (want - cur).rem_euclid(256);
            if diff > 128 {
                diff -= 256;
            }
            let step = diff.clamp(-rate, rate);
            self.units[i].turret_facing = (cur + step).rem_euclid(256) as u8;
        }
    }

    pub(super) fn try_shift(&mut self, i: usize, dx: i32, dy: i32) -> bool {
        if dx == 0 && dy == 0 {
            return true;
        }
        let nx = self.units[i].x + dx;
        let ny = self.units[i].y + dy;
        if self.terrain.walkable_px(nx / fp(1), ny / fp(1)) {
            self.units[i].x = nx;
            self.units[i].y = ny;
            true
        } else {
            false
        }
    }

    /// Push overlapping ground units apart.  Retail units cannot share the
    /// same ground position; ignoring this made armies collapse into a single
    /// point.  The separation is the minimum translation along the smaller
    /// overlap axis, split between both bodies, with a few relaxation passes.
    pub(super) fn resolve_collisions(&mut self) {
        let mut movable: Vec<usize> = (0..self.units.len())
            .filter(|&i| self.collision_extents(i).is_some())
            .collect();
        if movable.len() < 2 {
            return;
        }
        movable.sort_unstable();
        let cell = 32 * fp(1);
        for _pass in 0..2 {
            let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
            for &i in &movable {
                let key = (
                    self.units[i].x.div_euclid(cell),
                    self.units[i].y.div_euclid(cell),
                );
                grid.entry(key).or_default().push(i);
            }
            let mut pairs: Vec<(usize, usize)> = Vec::new();
            for (&(cx, cy), list) in &grid {
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        let Some(other) = grid.get(&(cx + dx, cy + dy)) else {
                            continue;
                        };
                        for &i in list {
                            for &j in other {
                                if i < j {
                                    pairs.push((i, j));
                                }
                            }
                        }
                    }
                }
            }
            pairs.sort_unstable();
            pairs.dedup();
            for (i, j) in pairs {
                let (Some((hwi, hhi)), Some((hwj, hhj))) =
                    (self.collision_extents(i), self.collision_extents(j))
                else {
                    continue;
                };
                let dx = self.units[j].x - self.units[i].x;
                let dy = self.units[j].y - self.units[i].y;
                let ox = hwi + hwj - dx.abs();
                let oy = hhi + hhj - dy.abs();
                if ox <= 0 || oy <= 0 {
                    continue;
                }
                if ox < oy {
                    let sign = if dx >= 0 { 1 } else { -1 };
                    let half = (ox / 2).max(1) * sign;
                    if !self.try_shift(i, -half, 0) {
                        let _ = self.try_shift(j, half * 2, 0);
                    } else {
                        let _ = self.try_shift(j, half, 0);
                    }
                } else {
                    let sign = if dy >= 0 { 1 } else { -1 };
                    let half = (oy / 2).max(1) * sign;
                    if !self.try_shift(i, 0, -half) {
                        let _ = self.try_shift(j, 0, half * 2);
                    } else {
                        let _ = self.try_shift(j, 0, half);
                    }
                }
            }
        }
    }

    /// Move toward `goal` in pixel coordinates.  Returns true when the unit
    /// has arrived.
    pub(super) fn move_towards(&mut self, i: usize, goal: (i32, i32), stop_distance: i32) -> bool {
        let ty = self.data.unit(self.units[i].type_id).clone();
        if ty.is_building() && !self.units[i].status.sieged && !self.units[i].lifted {
            return true;
        }
        if !ty.is_flyer() && !self.units[i].lifted {
            self.ensure_walking_animation(i);
        }
        let goal_fp = (goal.0.saturating_mul(fp(1)), goal.1.saturating_mul(fp(1)));
        if self.units[i].path_goal != Some(goal_fp) {
            self.build_obstacles();
            let speed = self.speed_of(i);
            let half_w = ty.dimensions.right.max(0).max(ty.dimensions.left.max(0)) as i32;
            let half_h = ty.dimensions.bottom.max(0).max(ty.dimensions.top.max(0)) as i32;
            let req = PathRequest {
                start: (self.units[i].x / fp(1), self.units[i].y / fp(1)),
                goal: (goal.0, goal.1),
                half_width: half_w.max(4),
                half_height: half_h.max(4),
                flying: ty.is_flyer() || self.units[i].lifted || speed <= 0,
            };
            let (dx, dy) = (req.goal.0 - req.start.0, req.goal.1 - req.start.1);
            let key = (dx.abs().max(dy.abs()) > LONG_ROUTE_PX && !req.flying).then_some((
                req.start.0.div_euclid(128),
                req.start.1.div_euclid(128),
                req.goal.0,
                req.goal.1,
                req.half_width,
                req.half_height,
            ));
            let cached = key.and_then(|k| self.routes.map.get(&k).cloned());
            if key.is_some() && cached.is_none() && self.routes.searches >= LONG_SEARCHES_PER_TICK {
                // Too many new long routes this frame: this unit asks again
                // next frame (INFERENCE: spreading searches over frames, so
                // an attack wave does not stall one frame for ~100 ms).
                return false;
            }
            let path = if speed <= 0 {
                None
            } else if let Some(cached) = cached {
                cached
            } else {
                if key.is_some() {
                    self.routes.searches += 1;
                }
                let found = find_path_with_regions(
                    &self.terrain,
                    &self.obstacles,
                    &req,
                    Some(&self.regions),
                )
                .map(|p| p.waypoints);
                if let Some(k) = key {
                    self.routes.map.insert(k, found.clone());
                }
                found
            };
            self.units[i].path.clear();
            self.units[i].path_goal = Some(goal_fp);
            if let Some(p) = path {
                for w in p {
                    // find_path works in pixels; convert to fp.
                    self.units[i].path.push_back((w.0 * fp(1), w.1 * fp(1)));
                }
                if self.units[i].path.is_empty() {
                    self.units[i].path.push_back(goal_fp);
                }
            } else {
                // No route: fall back to a direct move.  The retail engine
                // reports "unreachable" and keeps the unit in place; a direct
                // fallback keeps the simulation usable on partially walkable
                // custom maps without hiding the failure from the caller.
                self.units[i].path.push_back(goal_fp);
            }
        }
        let top = self.speed_of(i);
        if top <= 0 {
            return true;
        }
        // Flingy-controlled units (SCV, Vulture, Drone, Overlord, lifted
        // buildings; flingy.dat move control 0) accelerate by `acceleration`
        // per frame up to their top speed and brake inside `halt_distance`
        // of the goal, as retail.  FACT (Boot Camp recording, 3 SCVs on gas):
        // a delivery every 1.97 s against our 1.80 s with SCVs at full speed
        // from a standstill.  Iscript-moved walkers (Marines, Zerglings)
        // keep their animation speeds.
        let speed = match self.data.flingy(ty.flingy) {
            Some(f)
                if f.movement_type == crate::data::MovementType::Flingy && f.acceleration > 0 =>
            {
                let acc = self.modified_rate(i, f.acceleration as i32);
                let cur = if self.units[i].moved_tick + 1 == self.tick {
                    self.units[i].speed
                } else {
                    0
                };
                // Brake when the stopping distance at the current speed,
                // cur^2 / (2 acc) in 1/256 px, reaches the goal: at top
                // speed that is exactly flingy.dat's `halt_distance` (SCV
                // 1280^2 / 134 = 12227).
                let stopping = (cur as i64 * cur as i64 / (2 * acc as i64)) as i32;
                let near = self.distance_to_point(i, goal_fp) <= stopping.max(acc);
                let next = if near {
                    (cur - acc).max(acc)
                } else {
                    (cur + acc).min(top)
                };
                self.units[i].speed = next;
                self.units[i].moved_tick = self.tick;
                next
            }
            _ => top,
        };
        // Flingy units turn at flingy.dat `turn_rate` (256 = a full turn)
        // per frame and do not advance until they face their waypoint
        // within one step (INFERENCE from BW's flingy model; FACT: Boot Camp
        // gas with instant turning ran 1.82 s per delivery against the
        // recording's 1.97 s, two reversals per trip).
        let turn_rate = self
            .data
            .flingy(ty.flingy)
            .filter(|f| f.movement_type == crate::data::MovementType::Flingy && f.turn_rate > 0)
            .map(|f| f.turn_rate as i32)
            .map(|r| self.modified_rate(i, r));
        if let (Some(rate), Some(&(wx, wy))) = (turn_rate, self.units[i].path.front()) {
            let (dx, dy) = (wx - self.units[i].x, wy - self.units[i].y);
            if dx != 0 || dy != 0 {
                let want = direction_byte(dx, dy) as i32;
                let cur = self.units[i].facing as i32;
                let diff = (want - cur + 128).rem_euclid(256) - 128;
                if diff != 0 {
                    if diff.abs() > rate {
                        self.units[i].facing = (cur + rate * diff.signum()).rem_euclid(256) as u8;
                        self.units[i].speed = 0;
                        return false;
                    }
                    // Within one turn step: face the waypoint exactly before
                    // advancing, or the sprite keeps a small permanent skew.
                    self.units[i].facing = want as u8;
                }
            }
        }
        let mut remaining = speed;
        // Intermediate waypoints are reached within twice the unit's half
        // size, and given up after `PATH_STALL_TICKS` ticks without getting
        // closer: a group shares the same path corners, and requiring each
        // unit to stand on one froze the whole group once the others
        // occupied it (Wasteland: nine units jammed for 1,500+ ticks).
        let reach = fp((2 * half_extent_px(&ty)).max(16));
        if let Some(&(wx, wy)) = self.units[i].path.front() {
            let d = self.distance_to_point(i, (wx, wy));
            if d >= self.units[i].path_last_dist {
                self.units[i].path_stall = self.units[i].path_stall.saturating_add(1);
            } else {
                self.units[i].path_stall = 0;
            }
            self.units[i].path_last_dist = d;
            if self.units[i].path_stall >= PATH_STALL_TICKS {
                self.units[i].path_stall = 0;
                self.units[i].path_last_dist = i32::MAX;
                if self.units[i].path.len() > 1 {
                    self.units[i].path.pop_front();
                } else if self.distance_to_point(i, goal_fp) <= fp(ARRIVAL_CROWD_PX) {
                    // The goal is taken by the rest of the group: stop here,
                    // as retail units around a crowded destination do.
                    self.units[i].path.clear();
                    self.finish_move_animation(i);
                    return true;
                }
            }
        }
        while remaining > 0 {
            if self.units[i].path.len() > 1
                && let Some(&(wx, wy)) = self.units[i].path.front()
                && self.distance_to_point(i, (wx, wy)) <= reach
            {
                self.units[i].path.pop_front();
                self.units[i].path_last_dist = i32::MAX;
                continue;
            }
            let Some(&(wx, wy)) = self.units[i].path.front() else {
                let left = self.distance_to_point(i, goal_fp);
                if left > stop_distance && left <= fp(DIRECT_APPROACH_PX) {
                    // The route ended short of a goal inside an obstacle (a
                    // worker's mineral field): close the last stretch in a
                    // straight line, collisions keep it outside.  Replanning
                    // every frame ran a failing search per waiting worker
                    // and made an 8-player game run at 12 frames per second.
                    self.units[i].path.push_back(goal_fp);
                    continue;
                }
                if left > stop_distance {
                    // Replan next tick instead of freezing.
                    self.units[i].path_goal = None;
                }
                let done = self.distance_to_point(i, goal_fp) <= stop_distance;
                if done {
                    self.finish_move_animation(i);
                }
                return done;
            };
            let dx = wx - self.units[i].x;
            let dy = wy - self.units[i].y;
            let dist = isqrt((dx as i64) * (dx as i64) + (dy as i64) * (dy as i64)) as i32;
            if dist <= remaining {
                self.units[i].x = wx;
                self.units[i].y = wy;
                self.units[i].path.pop_front();
                remaining -= dist.max(0);
            } else if dist > 0 {
                self.units[i].x += (dx as i64 * remaining as i64 / dist as i64) as i32;
                self.units[i].y += (dy as i64 * remaining as i64 / dist as i64) as i32;
                self.units[i].facing = direction_byte(dx, dy);
                remaining = 0;
            } else {
                self.units[i].path.pop_front();
            }
        }
        self.distance_to_point(i, goal_fp) <= stop_distance && self.units[i].path.is_empty()
    }

    pub(super) fn finish_move_animation(&mut self, i: usize) {
        if self.units[i]
            .anim
            .as_ref()
            .is_some_and(|a| a.anim as usize == ANIM_WALKING)
        {
            self.start_animation(i, ANIM_WALKING_TO_IDLE);
        }
    }

    pub(super) fn distance_to_point(&self, i: usize, p: (i32, i32)) -> i32 {
        let dx = (self.units[i].x - p.0) as i64;
        let dy = (self.units[i].y - p.1) as i64;
        isqrt(dx * dx + dy * dy) as i32
    }

    /// Speed clamp recovered from `FUN_005b6900`: a positive status count
    /// applies the +50% speed (minimum 0x355) and three special hero types
    /// are pinned at 0x6ab; a negative count halves it; both cancel.
    #[cfg(test)]
    pub(super) fn modified_speed(type_id: u16, base: i32, stim: bool, ensnare: bool) -> i32 {
        let mods = i32::from(stim) - i32::from(ensnare);
        Self::modified_speed_for_mods(type_id, base, mods)
    }

    /// The clamp itself, driven by the recovered status-modifier count
    /// (translation batch 04, `0x005b49a0`: Stim +1, `CUnit+0xdc` bit 29
    /// +1, Ensnare -1).
    pub(super) fn modified_speed_for_mods(type_id: u16, base: i32, mods: i32) -> i32 {
        if mods > 0 {
            if matches!(type_id, 0x46 | 0x50 | 0x58) {
                return 0x6ab;
            }
            (base + base / 2).max(0x355)
        } else if mods < 0 {
            base / 2
        } else {
            base
        }
    }

    pub(super) fn speed_of(&self, i: usize) -> i32 {
        let u = &self.units[i];
        let ty = self.data.unit(u.type_id);
        if u.status.lockdown_ticks > 0 || u.status.stasis_ticks > 0 {
            return 0;
        }
        if u.status.sieged || u.status.burrowed {
            return 0;
        }
        let mut speed = if let Some(anim) = &u.anim {
            if anim.fling_speed > 0 {
                anim.fling_speed
            } else if anim.anim as usize == ANIM_WALKING && anim.move_speed > 0 {
                anim.move_speed
            } else {
                let cycle = self.data.unit_walk_cycle(ty);
                if cycle.is_empty() {
                    self.data.unit_speed(ty)
                } else {
                    cycle[(u.anim_tick as usize) % cycle.len()]
                }
            }
        } else {
            let cycle = self.data.unit_walk_cycle(ty);
            if cycle.is_empty() {
                self.data.unit_speed(ty)
            } else {
                cycle[(u.anim_tick as usize) % cycle.len()]
            }
        };
        speed = Self::modified_speed_for_mods(u.type_id, speed, self.movement_mods(i));
        // A lifted Terran building flies at 1 px per tick: OpenBW
        // `order_LiftingOff` sets `flingy_top_speed` to 1 and only restores
        // it for non-buildings.
        if u.lifted {
            speed = 256;
        }
        speed
    }

    /// OpenBW `update_unit_speed_upgrades`: whether this unit moves with
    /// its speed upgrade (Ion Thrusters for the Vulture, Metabolic Boost
    /// for the Zergling, ...) or is one of the units that always have it.
    pub fn has_speed_upgrade(&self, i: usize) -> bool {
        let u = &self.units[i];
        let id = match u.type_id {
            // 19 = Jim Raynor (Vulture).
            uid::VULTURE | 19 => upgrade::ION_THRUSTERS,
            uid::OVERLORD => upgrade::PNEUMATIZED_CARAPACE,
            uid::ZERGLING => upgrade::METABOLIC_BOOST,
            uid::HYDRALISK => upgrade::MUSCULAR_AUGMENTS,
            uid::ZEALOT => upgrade::LEG_ENHANCEMENTS,
            uid::SCOUT => upgrade::GRAVITIC_THRUSTERS,
            uid::SHUTTLE => upgrade::GRAVITIC_DRIVE,
            uid::OBSERVER => upgrade::GRAVITIC_BOOSTERS,
            uid::ULTRALISK => upgrade::ANABOLIC_SYNTHESIS,
            // Hunter Killer 53, Yggdrasill 57, Fenix (Zealot) 77, Mojo 80,
            // Artanis 88 and every Lurker.
            53 | 57 | 77 | 80 | 88 | uid::LURKER => return true,
            _ => return false,
        };
        self.players
            .get(u.owner as usize)
            .is_some_and(|p| p.upgrade_level(id) > 0)
    }

    /// Movement modifier count (OpenBW `get_modified_unit_speed`): +1 for
    /// Stim, +1 for a speed upgrade, -1 for Ensnare.  Positive: speed x1.5
    /// (at least 3 1/3 px per frame), acceleration and turn rate x2;
    /// negative: speed / 2, acceleration and turn rate - 1/4.
    pub fn movement_mods(&self, i: usize) -> i32 {
        let u = &self.units[i];
        i32::from(u.status.stim_ticks > 0) + i32::from(u.flag_29 || self.has_speed_upgrade(i))
            - i32::from(u.status.ensnare_ticks > 0)
    }

    /// Flingy acceleration or turn rate under the movement modifiers.
    pub(super) fn modified_rate(&self, i: usize, base: i32) -> i32 {
        match self.movement_mods(i) {
            m if m > 0 => base * 2,
            m if m < 0 => base - base / 4,
            _ => base,
        }
    }
}
