#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent34.rs
// ===========================================================================
// Translation batch 34.  Only this file is edited by the batch.
//
// The `decompiled_uncited` rows in `0x005d6000..=0x005d7fff` are one
// obfuscated block that Ghidra split at arbitrary boundaries.  Most of the
// larger bodies share the same shape: a large `rdtsc` / integrity-cookie /
// anti-debug preamble followed by a jump into the retail unit-list
// maintenance pass.  The pass is the part that touches simulation state, so
// that is what the methods below translate:
//
// * walk the removed-unit list (records that are no longer alive) and discard
//   the transient order state they still retain;
// * walk the visible list and, for cloaked records, clear the `CUnit+0x96`
//   bookkeeping byte and advance the `CUnit+0x91` 30-tick refresh timer;
// * walk the sprite-hidden list and clear `CUnit+0x96` on cloaked records;
// * when the recovered temporary-list gate is open, traverse the temporary
//   units (Scanner Sweep `0x21`, Map Revealer `0x65`).
//
// Retail frees removed records and their sprites; [`World::units`] is
// append-only, so a removed record is instead left in place with its
// transient state cleared.  The preambles also terminate through a noreturn
// abort helper when an integrity check fails; those branches are translated
// as `None` so the caller can distinguish "the retail path would have
// halted" from a successful pass.
//
// Several rows in the range are not simulation code at all: 8-byte jump
// thunks, `halt_baddata()` stubs, MSVC thread-safe-static accessors and
// `std::vector`-of-28-byte-record insert helpers.  They are listed as
// skipped in the batch report and are not given invented methods here.

/// Unit ids kept on the retail temporary-unit list.
const A34_TEMPORARY_TYPES: [u16; 2] = [0x21, 0x65];

/// Reload values of the two list-animation counters in `0x005d60a0`.
/// The first cycles `0..=7` every 150 passes, the second `0..=0x1d` every
/// 300 passes.  The counters themselves live in globals and are not part of
/// [`World`]; the method returns the reload values so the caller can keep the
/// cadence.
pub const A34_LIST_PERIODS: (u16, u16) = (0x96, 300);

/// Observable result of one maintenance pass.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct A34Cleanup {
    /// Removed-list records visited.  Because the Rust model keeps records,
    /// the transient fields are only present on the first visit.
    pub released_dead: u32,
    /// Visible-list records traversed by the cloak/status pass.
    pub visible: u32,
    /// Visible cloaked records whose `CUnit+0x91` timer wrapped `0 -> 0x1e`.
    pub wraps: u32,
    /// Hidden-list records whose cloak state cleared `CUnit+0x96`.
    pub hidden: u32,
    /// Temporary-list records traversed when the recovered gate is open.
    pub temporary: u32,
}

/// A live unit whose sprite is not drawn belongs to the sprite-hidden list.
fn a34_sprite_hidden(unit: &SimUnit) -> bool {
    unit.in_transport.is_some() || unit.in_bunker
}

/// Live Scanner Sweeps and Map Revealers are the temporary-list records.
fn a34_is_temporary(unit: &SimUnit) -> bool {
    unit.alive && A34_TEMPORARY_TYPES.contains(&unit.type_id)
}

/// Advance the 30-tick `CUnit+0x91` refresh timer of one cloaked record.
/// Returns `true` when the timer wrapped and the retail path would have
/// emitted the refresh cue.
fn a34_tick_cloak_timer(unit: &mut SimUnit) -> bool {
    unit.field_096 = 0;
    if unit.field_091 == 0 {
        unit.field_091 = 0x1e;
        true
    } else {
        unit.field_091 -= 1;
        false
    }
}

/// Discard the transient order state retained by a removed record.  Retail
/// releases the record's sprite/associated object and unlinks the order
/// nodes; the typed queues and pointers below are the model's equivalent.
fn a34_release_dead_state(unit: &mut SimUnit) {
    unit.target = None;
    unit.queue.clear();
    unit.production.clear();
    unit.path.clear();
    unit.path_goal = None;
    unit.resume_order = None;
}

impl World {
    /// `0x005d60a0` (`FUN_005d60a0`, size 156): visible-list flag sweep.
    ///
    /// The routine first runs two global list-animation counters.  When a
    /// counter expires it walks the visible list incrementing a shared frame
    /// counter through `0..=7` (reload 150 passes) or `0..=0x1d` (reload 300
    /// passes) and reloads the counter.  The visible-list walk then clears the
    /// removal/status flag stored in `CUnit+0xdc` bit 15 (read back by the
    /// caller at `0x005d614f`).  The recovered guard only skips an air record
    /// whose bit is already clear, which is unobservable, so the sweep clears
    /// every visible record.
    ///
    /// INFERENCE: the `0x69` probe and its noreturn abort branch have no
    /// simulation effect and are not modelled.  Returns
    /// [`A34_LIST_PERIODS`].
    pub fn a34_visible_flag_15_sweep(&mut self) -> (u16, u16) {
        for unit in self.units.iter_mut() {
            if !unit.alive || a34_sprite_hidden(unit) {
                continue;
            }
            unit.clear_flag_15();
        }
        A34_LIST_PERIODS
    }

    /// `0x005d614f` (`FUN_005d614f`, size 123): cloak refresh on the visible
    /// list.
    ///
    /// For every visible record with a live cloak state (the recovered
    /// `CUnit+0xdc` bits 8..9) the routine clears the `CUnit+0x96` byte and
    /// advances the `CUnit+0x91` timer: zero reloads to `0x1e` and triggers
    /// the refresh cue, any other value decrements.  Uncloaked visible
    /// records and sprite-hidden records are not touched by this pass.
    ///
    /// Returns `(cloaked records advanced, wraps)`, where a wrap is the count
    /// of retail refresh cues.
    pub fn a34_visible_cloak_refresh(&mut self) -> (u32, u32) {
        let mut advanced = 0;
        let mut wraps = 0;
        for unit in self.units.iter_mut() {
            if !unit.alive || a34_sprite_hidden(unit) {
                continue;
            }
            if unit.cloak_state() == 0 {
                continue;
            }
            advanced += 1;
            if a34_tick_cloak_timer(unit) {
                wraps += 1;
            }
        }
        (advanced, wraps)
    }

    /// `0x005d61cb` (`FUN_005d61cb`, size 373): guarded entry into the shared
    /// unit-list cleanup.
    ///
    /// The entry rejects two inputs before reaching the maintenance block:
    /// the low 16 bits equal to `0x69`, and a nonzero result from the
    /// recovered record-state probe.  Both retail paths are noreturn abort
    /// calls, translated here as `None`.  A clean entry performs
    /// [`World::a34_cleanup_all_lists`] with the temporary-list gate open.
    pub fn a34_cleanup_with_type_guard(
        &mut self,
        input: u16,
        state_probe: bool,
    ) -> Option<A34Cleanup> {
        if input == 0x69 || state_probe {
            return None;
        }
        Some(self.a34_cleanup_all_lists(true))
    }

    /// `0x005d644d` (`FUN_005d644d`, size 1450): full unit-list maintenance.
    ///
    /// The 1.4 kB body is almost entirely the obfuscated integrity preamble;
    /// it finally jumps into the shared block, which is what this method
    /// translates:
    ///
    /// 1. removed list: release the sprite/associated-object slot of every
    ///    dead record and either return it to the pool or run the per-record
    ///    status update;
    /// 2. visible list: overlay/status pass, then the cloaked-record refresh
    ///    from [`World::a34_visible_cloak_refresh`];
    /// 3. temporary list (only when the recovered global gate is open): the
    ///    status-overlay gate for Scanner Sweeps and Map Revealers;
    /// 4. hidden list: clear `CUnit+0x96` on cloaked records.
    ///
    /// Record removal itself is not modelled (append-only storage), so the
    /// released record stays with cleared order state.  `temporary_units`
    /// reports the traversed temporary records.
    pub fn a34_cleanup_all_lists(&mut self, temporary_units_enabled: bool) -> A34Cleanup {
        let mut pass = A34Cleanup::default();

        for unit in self.units.iter_mut() {
            if unit.alive {
                continue;
            }
            pass.released_dead += 1;
            a34_release_dead_state(unit);
        }

        for unit in self.units.iter_mut() {
            if !unit.alive {
                continue;
            }
            if a34_sprite_hidden(unit) {
                if unit.cloak_state() != 0 {
                    unit.field_096 = 0;
                    pass.hidden += 1;
                }
                continue;
            }
            pass.visible += 1;
            if unit.cloak_state() != 0 && a34_tick_cloak_timer(unit) {
                pass.wraps += 1;
            }
        }

        if temporary_units_enabled {
            pass.temporary = self
                .units
                .iter()
                .filter(|unit| a34_is_temporary(unit))
                .count() as u32;
        }

        pass
    }

    /// `0x005d6515` (`FUN_005d6515`, size 2): indirect tail transfer.
    ///
    /// The recovered body is `jmp [esi]`: it forwards control to the first
    /// pointer of the caller-supplied dispatch record without saving state.
    /// The Rust equivalent is to invoke the supplied closure and return its
    /// value directly.
    pub fn a34_tail_dispatch<T>(target: impl FnOnce() -> T) -> T {
        target()
    }

    /// `0x005d6938` (`FUN_005d6938`, size 11): unguarded entry into the shared
    /// cleanup block.
    ///
    /// Eleven bytes cannot hold a body of their own; the decompiled text is
    /// the shared maintenance block reached by this entry.  It therefore
    /// performs the same pass as [`World::a34_cleanup_all_lists`] with the
    /// temporary-list gate open and no preconditions.
    pub fn a34_cleanup_plain_entry(&mut self) -> A34Cleanup {
        self.a34_cleanup_all_lists(true)
    }

    /// `0x005d6943` (`FUN_005d6943`, size 146): unwind-validated entry into
    /// the shared cleanup block.
    ///
    /// The preamble walks a batch of released slots in four-byte steps and
    /// terminates through the noreturn abort helper when a slot aliases one
    /// of the live temporaries.  The batch is the caller's `released` slice;
    /// INFERENCE: a repeated entry stands in for the alias (double release)
    /// and yields `None`.  A valid batch runs the full pass with the
    /// temporary-list gate open.
    pub fn a34_cleanup_unwind_entry(&mut self, released: &[u32]) -> Option<A34Cleanup> {
        for (i, slot) in released.iter().enumerate() {
            if released[i + 1..].contains(slot) {
                return None;
            }
        }
        Some(self.a34_cleanup_all_lists(true))
    }

    /// `0x005d69d8` (`FUN_005d69d8`, size 49): timing-probe value.
    ///
    /// The routine folds the incoming value with two xor keys and one bias
    /// (`x ^ 0x128d ^ 0x252 + 0xe9b`), reads the TSC and passes the folded
    /// value to the noreturn abort helper.  Only the arithmetic is
    /// representable; the TSC read and the abort are anti-debug scaffolding.
    pub fn a34_timing_probe_mix(value: u32) -> u32 {
        (value ^ 0x128d ^ 0x252).wrapping_add(0xe9b)
    }

    /// `0x005d6a09` (`FUN_005d6a09`, size 494): iterator-checked entry into
    /// the shared cleanup block.
    ///
    /// Before the maintenance pass the routine walks its drained-slot buffer
    /// and aborts when an entry aliases an active temporary.  `released` are
    /// the drained slots and `active` the temporaries that must stay live;
    /// an overlap yields `None`.  A disjoint pair runs the full pass with the
    /// temporary-list gate open.
    pub fn a34_cleanup_checked_entry(
        &mut self,
        released: &[u32],
        active: &[u32],
    ) -> Option<A34Cleanup> {
        if released.iter().any(|slot| active.contains(slot)) {
            return None;
        }
        Some(self.a34_cleanup_all_lists(true))
    }

    /// `0x005d6ca9` (`FUN_005d6ca9`, size 471): integrity-gated entry into
    /// the shared cleanup block.
    ///
    /// The preamble builds and tests the recovered integrity cookie; a
    /// mismatch takes the noreturn abort branch.  `integrity_ok` carries the
    /// verdict.  A matched cookie runs the full pass with the temporary-list
    /// gate open.
    pub fn a34_cleanup_integrity_entry(&mut self, integrity_ok: bool) -> Option<A34Cleanup> {
        if !integrity_ok {
            return None;
        }
        Some(self.a34_cleanup_all_lists(true))
    }

    /// `0x005d79b9` (`FUN_005d79b9`, size 8): mask the incoming value with
    /// the constant recovered from the body (`0xc6bd6cae`).  The eight-byte
    /// routine consists only of the `and` and the return.
    pub fn a34_masked_value(value: u32) -> u32 {
        value & 0xc6bd_6cae
    }

    /// `0x005d7e01` (`FUN_005d7e01`, size 195): temporary-list teardown.
    ///
    /// The teardown tail walks the temporary-unit list and runs the
    /// temporary-record release routine on each entry.  INFERENCE: the record
    /// is freed in retail; since [`World::units`] never removes rows, the
    /// equivalent is to retire the record (`alive = false`) and clear its
    /// order state.  The returned count is the number of records retired by
    /// this call, so a second call reports zero.
    pub fn a34_release_temporary_units(&mut self) -> u32 {
        let mut released = 0;
        for unit in self.units.iter_mut() {
            if !a34_is_temporary(unit) {
                continue;
            }
            unit.alive = false;
            a34_release_dead_state(unit);
            unit.field_091 = 0;
            unit.field_096 = 0;
            released += 1;
        }
        released
    }
}

#[cfg(test)]
mod tests_agent34 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use crate::sim::{Order, World};
    use std::sync::Arc;

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, type_id: u16, x: i32) -> usize {
        world.spawn_px(0, type_id, x, 320).unwrap()
    }

    #[test]
    fn flag_15_sweep_clears_visible_records_only() {
        let mut w = open_world();
        let marine = spawn(&mut w, uid::MARINE, 320);
        let wraith = spawn(&mut w, uid::WRAITH, 400);
        let hidden = spawn(&mut w, uid::MARINE, 480);

        for i in [marine, wraith, hidden] {
            w.units[i].flag_15 = true;
        }
        w.units[hidden].in_bunker = true;

        assert_eq!(w.a34_visible_flag_15_sweep(), A34_LIST_PERIODS);
        assert!(!w.units[marine].flag_15);
        // An air record whose bit is already clear is skipped by retail; a
        // set bit is cleared like any other visible record.
        assert!(!w.units[wraith].flag_15);
        assert!(w.units[hidden].flag_15);
    }

    #[test]
    fn visible_cloak_refresh_ticks_and_wraps() {
        let mut w = open_world();
        let cloaked = spawn(&mut w, uid::GHOST, 320);
        let plain = spawn(&mut w, uid::MARINE, 400);
        let hidden = spawn(&mut w, uid::GHOST, 480);

        w.units[cloaked].status.cloaked = true;
        w.units[cloaked].field_096 = 7;
        w.units[hidden].in_bunker = true;
        w.units[hidden].status.cloaked = true;
        w.units[hidden].field_096 = 9;
        w.units[hidden].field_091 = 5;

        assert_eq!(w.a34_visible_cloak_refresh(), (1, 1));
        assert_eq!(w.units[cloaked].field_091, 0x1e);
        assert_eq!(w.units[cloaked].field_096, 0);

        assert_eq!(w.a34_visible_cloak_refresh(), (1, 0));
        assert_eq!(w.units[cloaked].field_091, 0x1d);

        // Uncloaked and hidden records are outside this pass.
        assert_eq!(w.units[plain].field_091, 0);
        assert_eq!(w.units[hidden].field_091, 5);
        assert_eq!(w.units[hidden].field_096, 9);
    }

    #[test]
    fn type_guard_rejects_dialog_input_and_probe() {
        let mut w = open_world();
        spawn(&mut w, uid::MARINE, 320);

        assert_eq!(w.a34_cleanup_with_type_guard(0x69, false), None);
        assert_eq!(w.a34_cleanup_with_type_guard(0, true), None);
        assert!(w.a34_cleanup_with_type_guard(0, false).is_some());
    }

    #[test]
    fn full_cleanup_releases_dead_state() {
        let mut w = open_world();
        let dead = spawn(&mut w, uid::MARINE, 320);
        let live = spawn(&mut w, uid::MARINE, 400);

        w.units[dead].alive = false;
        w.units[dead].target = Some(999);
        w.units[dead].queue.push_back(Order::Stop);
        w.units[dead].production.push_back(Order::Stop);
        w.units[dead].path.push_back((1, 1));
        w.units[dead].path_goal = Some((1, 1));
        w.units[dead].resume_order = Some(Order::Stop);

        let pass = w.a34_cleanup_all_lists(false);
        assert_eq!(pass.released_dead, 1);
        assert_eq!(pass.visible, 1);
        assert_eq!(pass.temporary, 0);

        assert!(w.units[dead].target.is_none());
        assert!(w.units[dead].queue.is_empty());
        assert!(w.units[dead].production.is_empty());
        assert!(w.units[dead].path.is_empty());
        assert!(w.units[dead].path_goal.is_none());
        assert!(w.units[dead].resume_order.is_none());
        // The live record is untouched.
        assert_eq!(w.units[live].target, None);
    }

    #[test]
    fn full_cleanup_counts_temporary_units_only_when_gated_open() {
        let mut w = open_world();
        spawn(&mut w, uid::SCANNER_SWEEP, 320);
        spawn(&mut w, uid::MARINE, 400);

        assert_eq!(w.a34_cleanup_all_lists(false).temporary, 0);
        assert_eq!(w.a34_cleanup_all_lists(true).temporary, 1);
        // The overlay gate does not retire the record.
        assert!(w.units[0].alive);
    }

    #[test]
    fn tail_dispatch_forwards_the_result_once() {
        let mut calls = 0;
        let value = World::a34_tail_dispatch(|| {
            calls += 1;
            0x5d
        });
        assert_eq!(value, 0x5d);
        assert_eq!(calls, 1);
    }

    #[test]
    fn plain_entry_matches_the_full_pass() {
        let mut a = open_world();
        let mut b = open_world();
        spawn(&mut a, uid::MARINE, 320);
        spawn(&mut b, uid::MARINE, 320);
        a.units[0].status.cloaked = true;
        b.units[0].status.cloaked = true;

        assert_eq!(a.a34_cleanup_plain_entry(), b.a34_cleanup_all_lists(true));
    }

    #[test]
    fn unwind_entry_rejects_a_double_release() {
        let mut w = open_world();
        assert_eq!(w.a34_cleanup_unwind_entry(&[1, 2, 2]), None);
        assert!(w.a34_cleanup_unwind_entry(&[1, 2]).is_some());
        assert!(w.a34_cleanup_unwind_entry(&[]).is_some());
    }

    #[test]
    fn timing_probe_mix_uses_the_recovered_keys() {
        assert_eq!(World::a34_timing_probe_mix(0), 0x1f7a);
        assert_eq!(World::a34_timing_probe_mix(0xffff_ffff), 0xffff_fdbb);
        assert_eq!(
            World::a34_timing_probe_mix(0x1234_5678),
            (0x1234_5678_u32 ^ 0x128d ^ 0x252).wrapping_add(0xe9b),
        );
    }

    #[test]
    fn checked_entry_rejects_drained_active_overlap() {
        let mut w = open_world();
        assert_eq!(w.a34_cleanup_checked_entry(&[7, 8], &[8, 9]), None);
        assert!(w.a34_cleanup_checked_entry(&[7, 8], &[9, 10]).is_some());
        assert!(w.a34_cleanup_checked_entry(&[], &[9]).is_some());
    }

    #[test]
    fn integrity_entry_gates_on_the_cookie_verdict() {
        let mut w = open_world();
        assert_eq!(w.a34_cleanup_integrity_entry(false), None);
        assert!(w.a34_cleanup_integrity_entry(true).is_some());
    }

    #[test]
    fn masked_value_keeps_only_the_retail_bits() {
        const MASK: u32 = 0xc6bd_6cae;
        assert_eq!(World::a34_masked_value(0), 0);
        assert_eq!(World::a34_masked_value(0xffff_ffff), MASK);
        assert_eq!(World::a34_masked_value(MASK), MASK);
        assert_eq!(World::a34_masked_value(!MASK), 0);
    }

    #[test]
    fn temporary_teardown_retires_scanner_sweeps_once() {
        let mut w = open_world();
        let sweep = spawn(&mut w, uid::SCANNER_SWEEP, 320);
        let marine = spawn(&mut w, uid::MARINE, 400);
        w.units[sweep].target = Some(marine as u32 + 1);
        w.units[sweep].field_091 = 12;

        assert_eq!(w.a34_release_temporary_units(), 1);
        assert!(!w.units[sweep].alive);
        assert!(w.units[sweep].target.is_none());
        assert_eq!(w.units[sweep].field_091, 0);
        assert!(w.units[marine].alive);

        assert_eq!(w.a34_release_temporary_units(), 0);
    }
}
