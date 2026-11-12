//! Turret images of the Siege Tank and Goliath and the sieged tank's
//! deploy frames.

use super::*;

impl App {
    /// Facing byte the renderer should draw.
    /// Turret of a Siege Tank or Goliath (`units.dat` subunit): its image
    /// and heading (`SimUnit::turret_facing`).  Drawn on top of the base at
    /// the same position.
    pub(super) fn turret_pose(&self, u: &galaxycraft::sim::SimUnit) -> Option<(u16, u8)> {
        let ty = self.data.unit(u.type_id);
        self.data.turret(ty)?;
        let image = galaxycraft::unit_anim::unit_image(&self.data, ty.turret)?;
        // The simulation turns the turret at its own rate.
        Some((image, u.turret_facing))
    }

    /// Frame of the sieged tank's base (iscript 92): frames 0..=5 unfold
    /// over the deploy animation (five frames of 5, then it holds 5) and
    /// fold back at the end of the pack-up one (38 frames of hold first).
    /// Frame of the sieged cannon (iscript 93) while the tank deploys or
    /// packs up: it waits folded (17), then unfolds 34 / 51 / 68 / 85 three
    /// frames apart and rests at 0; packing up runs 85 down to 17.  `None`
    /// outside those animations.
    pub(super) fn siege_turret_frame(&self, u: &galaxycraft::sim::SimUnit) -> Option<u16> {
        use galaxycraft::sim::{SIEGE_FRAMES, UNSIEGE_FRAMES};
        if u.type_id != galaxycraft::ids::unit::SIEGE_MODE || u.order_wait == 0 {
            return None;
        }
        let wait = u.order_wait as i32;
        Some(match u.order {
            Order::Siege => {
                let e = SIEGE_FRAMES as i32 - (wait - 1);
                match e {
                    e if e < 42 => 17,
                    e if e < 54 => (34 + 17 * ((e - 42) / 3)) as u16,
                    _ => 0,
                }
            }
            Order::Unsiege => {
                let e = UNSIEGE_FRAMES as i32 - (wait - 1);
                (85 - 17 * (e / 3)).max(17) as u16
            }
            _ => return None,
        })
    }

    pub(super) fn siege_base_frame(&self, u: &galaxycraft::sim::SimUnit) -> usize {
        use galaxycraft::sim::{SIEGE_FRAMES, UNSIEGE_FRAMES};
        let wait = u.order_wait as i32;
        match u.order {
            Order::Siege if wait > 0 => {
                let elapsed = SIEGE_FRAMES as i32 - (wait - 1);
                ((elapsed - 1) / 5).clamp(0, 5) as usize
            }
            Order::Unsiege if wait > 0 => {
                let elapsed = UNSIEGE_FRAMES as i32 - (wait - 1);
                if elapsed < 38 {
                    5
                } else {
                    (4 - (elapsed - 38) / 5).clamp(0, 5) as usize
                }
            }
            _ => 5,
        }
    }
}
