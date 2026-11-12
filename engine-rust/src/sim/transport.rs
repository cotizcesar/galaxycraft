//! `impl World`: transport (moved out of `mod.rs` by scripts/split_impl.py).

use super::*;

impl World {
    /// OpenBW `order_PickupTransport`: fly to a unit that wants to board
    /// until the boxes touch; done when it is aboard or gave up.
    pub(super) fn order_pickup_transport(&mut self, i: usize, unit: u32) {
        let waiting = self.index_of(unit).filter(|&p| {
            self.units[p].alive
                && self.units[p].in_transport.is_none()
                && self.units[p].order
                    == Order::EnterTransport {
                        transport: self.units[i].id,
                    }
        });
        let Some(p) = waiting else {
            self.units[i].path.clear();
            self.units[i].path_goal = None;
            self.order_done(i);
            return;
        };
        if self.box_gap_px(i, p) > 1 {
            let (px, py) = (self.units[p].x / fp(1), self.units[p].y / fp(1));
            self.move_towards(i, (px, py), fp(1));
        } else {
            self.units[i].path.clear();
            self.units[i].path_goal = None;
        }
    }

    pub(super) fn order_enter_transport(&mut self, i: usize, transport: u32) {
        let Some(tidx) = self.index_of(transport) else {
            self.order_done(i);
            return;
        };
        if !self.units[tidx].alive
            || !self.units[tidx].completed
            || tidx == i
            || self.units[tidx].owner != self.units[i].owner
            || self.data.unit(self.units[i].type_id).is_building()
        {
            self.order_done(i);
            return;
        }
        // A transport that can move comes to fetch its passenger (OpenBW
        // `order_EnterTransport` queues `PickupTransport` on it) and the
        // two board when their boxes touch (1 px).  A Bunker stays put and
        // takes units the same way.
        let mobile = !self.data.unit(self.units[tidx].type_id).is_building();
        if mobile && matches!(self.units[tidx].order, Order::Idle) {
            self.units[tidx].order = Order::PickupTransport {
                unit: self.units[i].id,
            };
            self.units[tidx].order_row = 94;
        }
        // OpenBW `order_EnterTransport`: board when the boxes are within
        // 1 px.
        let gap = self.box_gap_px(i, tidx);
        if gap > 1 {
            let (tx, ty) = (self.units[tidx].x, self.units[tidx].y);
            if !mobile && gap <= 8 {
                // The path grid ends a few pixels short of a building's box:
                // close in straight towards its nearest point.
                let ty_ = self.data.unit(self.units[tidx].type_id);
                let r = unit_box_rect(ty_, (tx / fp(1), ty / fp(1)));
                let (ux, uy) = (self.units[i].x, self.units[i].y);
                let near = (
                    (ux / fp(1)).clamp(r.left, r.right) * fp(1),
                    (uy / fp(1)).clamp(r.top, r.bottom) * fp(1),
                );
                let (dx, dy) = ((near.0 - ux) as i64, (near.1 - uy) as i64);
                let len = crate::sim::isqrt(dx * dx + dy * dy).max(1);
                let step = ((gap - 1).min(4) * fp(1)) as i64;
                self.units[i].x += (dx * step / len) as i32;
                self.units[i].y += (dy * step / len) as i32;
                self.units[i].path.clear();
                self.units[i].path_goal = None;
                return;
            }
            let stop = if mobile { fp(1) } else { fp(24) };
            self.move_towards(i, (tx / fp(1), ty / fp(1)), stop);
            return;
        }
        let target_type = self.units[tidx].type_id;
        let owner = self.units[i].owner;

        // Nydus Canal: the unit travels to the paired canal instead of being
        // stored.  Order 45 (Enter Nydus Canal) uses this path.
        if is_nydus_canal_type(target_type) {
            let paired = self.units[tidx]
                .nydus_exit
                .and_then(|id| self.index_of(id))
                .filter(|&e| self.units[e].alive && self.units[e].completed);
            let exit = paired.or_else(|| {
                self.units.iter().position(|u| {
                    u.alive
                        && u.completed
                        && u.owner == owner
                        && is_nydus_canal_type(u.type_id)
                        && u.id != transport
                })
            });
            if let Some(e) = exit {
                let (ex, ey) = self.exit_position(e);
                self.units[i].x = ex;
                self.units[i].y = ey;
                self.units[i].path.clear();
                self.units[i].path_goal = None;
                self.order_done(i);
            } else {
                self.order_done(i);
            }
            return;
        }

        let moving_id = self.units[i].id;
        if target_type == uid::BUNKER {
            // Four infantry slots, no vehicles or workers.
            if !is_bunker_infantry(self.units[i].type_id) || self.units[tidx].cargo.len() >= 4 {
                self.order_done(i);
                return;
            }
        } else {
            let capacity = self.data.unit(target_type).space_provided as i32;
            // Translation batch 33 (`0x005d5ea0`): cargo space already used.
            let used = self.a33_cargo_used_space(transport);
            let need = self.data.unit(self.units[i].type_id).space_required as i32;
            if capacity - used < need {
                self.order_done(i);
                return;
            }
        }
        self.units[tidx].cargo.push(moving_id);
        self.units[i].mark_in_transport(transport);
        self.units[i].set_in_bunker(target_type == uid::BUNKER);
        self.units[i].alive = false; // hidden/loaded
        if target_type == uid::BUNKER {
            self.units[i].x = self.units[tidx].x;
            self.units[i].y = self.units[tidx].y;
        }
        self.units[i].order = Order::Idle;
    }

    /// Unload one unit from a Bunker / transport (retail: a click on its
    /// cargo slot).  Recorded (`RecAct::UnloadOne`, save tag 11).
    pub fn unload_unit(&mut self, transport_id: u32, unit_id: u32) -> Result<(), SimError> {
        let i = self.index_of(transport_id).ok_or(SimError::BadUnit)?;
        // Translation batch 27 (`0x005ad780`): detach the cargo node and
        // clear the unit's containment back-link.
        if !self.a27_remove_cargo(transport_id, unit_id) {
            return Err(SimError::InvalidTarget);
        }
        let (x, y) = self.exit_position(i);
        if let Some(j) = self.index_of(unit_id) {
            self.units[j].alive = true;
            self.units[j].clear_transport_link();
            self.units[j].set_in_bunker(false);
            self.units[j].target = None;
            self.units[j].order_wait = 0;
            self.units[j].x = x;
            self.units[j].y = y;
        }
        let tick = self.tick;
        if let Some(r) = self.recorder.as_mut() {
            r.record_econ(
                tick,
                crate::replay::RecAct::UnloadOne {
                    transport: transport_id,
                    unit: unit_id,
                },
            );
        }
        Ok(())
    }

    /// Unload (OpenBW `order_Unload` / `unit_unload`).  A Bunker empties at
    /// once.  A transport lets one unit out every 15 frames; each one
    /// lands with its weapon on cooldown, so a drop does not fire the
    /// instant it touches the ground.
    pub(super) fn order_unload(&mut self, i: usize) {
        let building = self.data.unit(self.units[i].type_id).is_building();
        if !building && self.units[i].order_wait > 0 {
            self.units[i].order_wait -= 1;
            return;
        }
        let (cx, cy) = (self.units[i].x, self.units[i].y);
        let count = if building {
            self.units[i].cargo.len()
        } else {
            1.min(self.units[i].cargo.len())
        };
        for _ in 0..count {
            let id = self.units[i].cargo.remove(0);
            let Some(j) = self.index_of(id) else {
                continue;
            };
            // OpenBW `unit_unload`: the nearest free spot to the container
            // (`find_unit_placement`), not one shared exit point.
            let (x, y) = self
                .free_spot_near(j, (cx, cy))
                .unwrap_or_else(|| self.exit_position(i));
            self.units[j].alive = true;
            self.units[j].clear_transport_link();
            self.units[j].set_in_bunker(false);
            self.units[j].x = x;
            self.units[j].y = y;
            if !building {
                let ty = self.data.unit(self.units[j].type_id).clone();
                let cooldown = [self.data.ground_weapon(&ty), self.data.air_weapon(&ty)]
                    .into_iter()
                    .flatten()
                    .map(|w| self.modified_cooldown(j, w))
                    .max()
                    .unwrap_or(0);
                self.units[j].cooldown = self.units[j].cooldown.max(cooldown);
            }
        }
        if building || self.units[i].cargo.is_empty() {
            self.units[i].order_wait = 0;
            self.order_done(i);
        } else {
            self.units[i].order_wait = TRANSPORT_UNLOAD_FRAMES;
        }
    }
}
