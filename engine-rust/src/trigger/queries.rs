//! Read-only queries on the trigger state: locations and which triggers
//! have run (the latter for tools: `sc_solve`).

use super::TriggerSystem;

impl TriggerSystem {
    /// Centre (world px) of location `location` (1-based, as actions name
    /// it); `None` for "Anywhere" (64), 0 or an unknown index.
    pub fn location_center(&self, location: u32) -> Option<(i32, i32)> {
        if location == 0 || location == 64 {
            return None;
        }
        let l = self.locations.get(location as usize - 1)?;
        Some(((l.0 + l.2) / 2, (l.1 + l.3) / 2))
    }

    /// The location of `player`'s last Transmission.
    pub fn last_transmission(&self, player: usize) -> Option<(i32, i32)> {
        self.transmission_locations
            .iter()
            .rev()
            .filter(|(p, _)| *p == player)
            .find_map(|(_, l)| self.location_center(*l))
    }

    /// Whether a unit at `(x_px, y_px)` (fixed point) is inside `location`
    /// (0 and 64 are "Anywhere").
    pub(super) fn unit_in_location(&self, x_px: i32, y_px: i32, location: u32) -> bool {
        if location == 0 || location == 64 {
            return true;
        }
        let Some(l) = self.locations.get(location as usize - 1) else {
            return false;
        };
        let x = x_px / 256;
        let y = y_px / 256;
        x >= l.0 && x < l.2 && y >= l.1 && y < l.3
    }

    /// Whether trigger `ti` has fired for some player and is not preserved,
    /// so it will not run again.
    pub fn has_fired_for_good(&self, ti: usize) -> bool {
        self.state.fired.iter().enumerate().any(|(p, f)| {
            f.get(ti).copied().unwrap_or(false)
                && !self
                    .preserved
                    .get(p)
                    .and_then(|v| v.get(ti))
                    .copied()
                    .unwrap_or(false)
                && self
                    .triggers
                    .get(ti)
                    .is_some_and(|t| t.execution_flags & 4 == 0)
        })
    }
}
