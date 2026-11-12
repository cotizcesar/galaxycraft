//! `impl World`: lifting and landing Terran buildings, and their add-ons
//! (OpenBW `order_LiftingOff`, `order_BuildingLand`,
//! `building_abandon_addon`, `find_connecting_addon`, `unit_can_build`).

use super::*;

impl World {
    /// One tick of a lift-off or landing: move 1 px (up or down) until the
    /// building has travelled [`crate::unit_anim::LIFT_HEIGHT`] px; true
    /// once the phase is over (the move done and the iscript's `sigorder`
    /// reached).
    pub(super) fn advance_lift_phase(&mut self, i: usize) -> bool {
        let Some((landing, t)) = self.units[i].lift_phase else {
            return true;
        };
        if (t as i32) < crate::unit_anim::LIFT_HEIGHT {
            self.units[i].y += if landing { fp(1) } else { -fp(1) };
        }
        let t = t.saturating_add(1);
        let total = crate::unit_anim::lift_phase_ticks(&self.data, self.units[i].type_id, landing);
        if t as u32 >= total {
            self.units[i].lift_phase = None;
            true
        } else {
            self.units[i].lift_phase = Some((landing, t));
            false
        }
    }

    /// Whether the lifted building `i` may set down centred on `pos`.
    pub(super) fn can_land_at(&self, i: usize, pos: (i32, i32)) -> bool {
        self.landing_blocked(i, pos).is_ok()
    }

    pub(super) fn landing_blocked(&self, i: usize, pos: (i32, i32)) -> Result<(), SimError> {
        let building_id = self.units[i].id;
        let ty = self.data.unit(self.units[i].type_id);
        // Tiles, as retail: the placement box, not the sprite's dimensions
        // (a Machine Shop's box reaches 39 px left, into its Factory's
        // last column).
        let rect = placement_rect(ty, pos);
        if rect.left < 0
            || rect.top < 0
            || !self
                .terrain
                .footprint_buildable(rect.left, rect.top, rect.right, rect.bottom)
        {
            return Err(SimError::InvalidTarget);
        }
        let blocked = self.units.iter().any(|u| {
            if !u.alive || u.id == building_id || !u.completed || u.lifted {
                return false;
            }
            let ut = self.data.unit(u.type_id);
            if !(ut.is_building() || is_resource_type(u.type_id)) {
                return false;
            }
            let ux = u.x / fp(1);
            let uy = u.y / fp(1);
            let other = placement_rect(ut, (ux, uy));
            // Touching edges are fine; overlapping tiles are not.
            rect.left < other.right
                && other.left < rect.right
                && rect.top < other.bottom
                && other.top < rect.bottom
        });
        if blocked {
            return Err(SimError::InvalidTarget);
        }
        Ok(())
    }

    /// Lift a Terran building off the ground.  Add-ons are left behind and
    /// destroyed, as in retail.
    pub fn lift_building(&mut self, building_id: u32) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive || !self.units[i].completed {
            return Err(SimError::MissingRequirements);
        }
        if !can_lift_building(self.units[i].type_id) {
            return Err(SimError::InvalidTarget);
        }
        if self.units[i].lifted {
            return Err(SimError::ProducerBusy);
        }
        // The add-on stays on the ground, abandoned (OpenBW
        // `building_abandon_addon`); one still under construction is
        // cancelled (`make_unit_neutral`).
        if let Some(addon_id) = self.units[i].addon
            && let Some(a) = self.index_of(addon_id)
        {
            if self.units[a].completed {
                self.units[a].addon_attached = false;
            } else {
                let _ = self.cancel_addon_unrecorded(building_id);
            }
        }
        self.units[i].addon = None;
        self.units[i].lifted = true;
        self.units[i].order = Order::Idle;
        self.units[i].path.clear();
        self.units[i].path_goal = None;
        Ok(())
    }

    /// Land a lifted building at `pos` (pixels).  The footprint must be
    /// buildable and free of other structures/units.
    pub fn land_building(&mut self, building_id: u32, pos: (i32, i32)) -> Result<(), SimError> {
        let i = self.index_of(building_id).ok_or(SimError::BadUnit)?;
        if !self.units[i].alive || !self.units[i].lifted {
            return Err(SimError::InvalidTarget);
        }
        self.landing_blocked(i, pos)?;
        self.units[i].x = pos.0 * fp(1);
        self.units[i].y = pos.1 * fp(1);
        self.units[i].lifted = false;
        self.units[i].lift_phase = None;
        self.units[i].order = Order::Idle;
        self.units[i].path.clear();
        self.units[i].path_goal = None;
        // Landing exactly where an abandoned add-on of the right type
        // expects its parent re-attaches it, whoever built it: it becomes
        // the lander's (OpenBW `find_connecting_addon` compares the tiles
        // of the two placement boxes, `connect_addon` gives it away).
        let parent_type = self.units[i].type_id;
        let found = self.units.iter().position(|u| {
            if !(u.alive && u.is_detached_addon() && addon_parent(u.type_id) == Some(parent_type)) {
                return false;
            }
            let ((left, top), (aw, ah)) = self.addon_box(i, u.type_id);
            let ul = u.x / fp(1) - aw / 2;
            let ut = u.y / fp(1) - ah / 2;
            ul.div_euclid(32) == left.div_euclid(32) && ut.div_euclid(32) == top.div_euclid(32)
        });
        if let Some(a) = found {
            self.units[a].addon_attached = true;
            self.units[a].owner = self.units[i].owner;
            let addon_id = self.units[a].id;
            self.units[i].a14_set_addon_link(addon_id);
        }
        Ok(())
    }

    /// Placement box (top-left, size in px) of an `addon_type` built by
    /// the building at index `parent`.
    pub fn addon_box(&self, parent: usize, addon_type: u16) -> ((i32, i32), (i32, i32)) {
        let pt = self.data.unit(self.units[parent].type_id);
        let at = self.data.unit(addon_type);
        let (pw, ph) = (pt.placement.x as i32, pt.placement.y as i32);
        let (aw, ah) = (at.placement.x as i32, at.placement.y as i32);
        let left = self.units[parent].x / fp(1) - pw / 2 + at.addon_position.x as i32;
        let top = self.units[parent].y / fp(1) - ph / 2 + at.addon_position.y as i32;
        ((left, top), (aw, ah))
    }

    /// Retail rules that depend on the producing building itself (OpenBW
    /// `unit_can_build`): nothing trains while the building constructs its
    /// add-on, the Siege Tank needs a Machine Shop on *this* Factory, and
    /// the Dropship, Science Vessel, Valkyrie and Battlecruiser a Control
    /// Tower on *this* Starport.
    pub fn producer_can_make(&self, producer: usize, unit_type: u16) -> bool {
        let Some(p) = self.units.get(producer) else {
            return false;
        };
        let addon = p.addon.and_then(|id| self.unit(id));
        if addon.is_some_and(|a| !a.completed) {
            return false;
        }
        let needed = match unit_type {
            uid::SIEGE_TANK => uid::MACHINE_SHOP,
            uid::DROPSHIP | uid::SCIENCE_VESSEL | uid::VALKYRIE | uid::BATTLECRUISER => {
                uid::CONTROL_TOWER
            }
            _ => return true,
        };
        if self.players[p.owner as usize].cheats.build_anything {
            return true;
        }
        addon.is_some_and(|a| a.type_id == needed && a.addon_attached)
    }
}

impl SimUnit {
    /// A finished add-on with no building attached.  Retail gives it to
    /// the neutral player (OpenBW `building_abandon_addon`): it trains,
    /// researches and upgrades nothing until a building lands next to it.
    pub fn is_detached_addon(&self) -> bool {
        self.completed && !self.addon_attached && addon_parent(self.type_id).is_some()
    }
}

/// The tiles a building occupies: its `units.dat` placement box around
/// `centre` (right / bottom exclusive).
pub(super) fn placement_rect(ty: &UnitType, centre: (i32, i32)) -> Rect {
    let (w, h) = (ty.placement.x.max(1) as i32, ty.placement.y.max(1) as i32);
    let (l, t) = (centre.0 - w / 2, centre.1 - h / 2);
    Rect::new(l, t, l + w, t + h)
}

/// Terran structures that can lift off in the retail game.
pub(super) fn can_lift_building(type_id: u16) -> bool {
    matches!(
        type_id,
        uid::COMMAND_CENTER
            | uid::BARRACKS
            | uid::FACTORY
            | uid::STARPORT
            | uid::SCIENCE_FACILITY
            | uid::ENGINEERING_BAY
            | uid::INFESTED_COMMAND_CENTER
    )
}

/// Parent building for each Terran add-on.
pub(super) fn addon_parent(addon: u16) -> Option<u16> {
    Some(match addon {
        uid::COMSAT_STATION | uid::NUCLEAR_SILO => uid::COMMAND_CENTER,
        uid::MACHINE_SHOP => uid::FACTORY,
        uid::CONTROL_TOWER => uid::STARPORT,
        uid::COVERT_OPS | uid::PHYSICS_LAB => uid::SCIENCE_FACILITY,
        _ => return None,
    })
}
