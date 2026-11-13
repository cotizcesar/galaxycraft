//! `impl World`: starting alliances of a use-map-settings scenario.

use super::*;

/// `OWNR` value of a "Rescue Passive" player.
pub const OWNR_RESCUE_PASSIVE: u8 = 3;
/// Ticks between two rescue checks (retail order 160 handler resets its
/// timer at `+0x54` to 15).
pub const RESCUE_INTERVAL: u64 = 15;
/// Centre distance at which a rescuable unit is rescued.  Retail Wasteland
/// recording (2026-11-05, `Screencast_20261105_074011`): Raynor changes
/// hands at 15.7 s with the nearest Marine ~55 px away, not at ~65-75 px
/// half a second earlier; 2 tiles is the INFERENCE that fits.
pub const RESCUE_RANGE_PX: i32 = 64;

impl World {
    /// Retail use-map-settings alliances: computer players with a start
    /// location are allied with each other whatever their force (a
    /// trigger has to make them enemies), and the members of a force with
    /// the "allies" property (`FORC` flag 2) are allied.  Every other pair
    /// starts as enemies.  Sources: the StarEdit FAQ and Blizzard's forum
    /// answer on map-editor AI (computers "are allies regardless of which
    /// Force they're in"); Hammer Falls places two computer players' units
    /// side by side around the Ion Cannon with no alliance trigger.
    pub(super) fn apply_ums_alliances(&mut self, scenario: &crate::map::Scenario) {
        let n = self.players.len().min(8);
        let mut team: Vec<usize> = (0..n).collect();
        fn root(team: &mut [usize], mut i: usize) -> usize {
            while team[i] != i {
                team[i] = team[team[i]];
                i = team[i];
            }
            i
        }
        let union = |team: &mut Vec<usize>, a: usize, b: usize| {
            let (ra, rb) = (root(team, a), root(team, b));
            if ra != rb {
                team[ra.max(rb)] = ra.min(rb);
            }
        };
        let occupied = |p: usize| matches!(scenario.owners.get(p), Some(5 | 6));
        let computers: Vec<usize> = (0..n)
            .filter(|&p| {
                scenario.owners.get(p) == Some(&5) && scenario.start_location_of(p as u8).is_some()
            })
            .collect();
        for w in computers.windows(2) {
            union(&mut team, w[0], w[1]);
        }
        for f in 0..4 {
            if scenario.force_flags[f] & 2 == 0 {
                continue;
            }
            let members: Vec<usize> = (0..n)
                .filter(|&p| occupied(p) && scenario.forces[p] as usize == f)
                .collect();
            for w in members.windows(2) {
                union(&mut team, w[0], w[1]);
            }
        }
        for p in 0..n {
            self.players[p].team = root(&mut team, p) as u8;
        }
    }
}
