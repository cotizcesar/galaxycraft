#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/translated/agent27.rs
// ===========================================================================
// Translation batch 27.  Only this file is edited by the batch.
//
// Eleven `decompiled_uncited` rows in `0x005ac000..=0x005adfff` are
// translated here:
//
// * `0x005ac009`, `0x005ac06d` - the "smart interaction" target pickers of
//   the classic client (a candidate found by the near-point unit query is
//   adopted as the unit's `+0x5c` target, otherwise the reference state byte
//   `+0x4e` is set to 3 and a cue is played).
// * `0x005ac0fd` - an otherwise empty hook whose only instruction calls the
//   unresolved runtime helper `0x005b8040`.
// * `0x005ac28e` - the null-selector dialog cue (`0x51`/`0x55`) and its
//   routing to the selection slot `+0xc8`.
// * `0x005ac350` - the owner-list scan that returns the first unit whose
//   target (`+0x5c`) is the receiver.
// * `0x005ac531` - the overlay refresh: clear `CUnit+0xe0`, pick frame 8/9
//   and write it through the overlay setter (`+0xe0`/`+0xcf`).
// * `0x005ad558`, `0x005ad5ff` - the container-entry tail (busy byte, the
//   `+0xce`/`+0xd0` containment link and the 0x53/0x57/0x58 cues).
// * `0x005ad780` - unlinking a unit from a container's `+0xd4` cargo list.
// * `0x005ad8a0`, `0x005ad950` - the two `CUnit+0x64` type-id getters
//   (one gated on the `CUnit+0xdc` bit-0 test).
//
// `FUN_005ac3a0` (a SEH/terminate stub) and `FUN_005ac750` (recovered body is
// truncated at `halt_baddata`) are skipped: their decompiled bodies contain
// no modeled state.
//
// The container helpers use the fields the simulation already keeps:
// [`SimUnit::cargo`] for the `+0xd4`/`+0xd8` list and
// [`SimUnit::in_transport`] for the `+0xce`/`+0xd0` containment link.  The
// per-type byte at `CUnit+0x4d` (filled from the `units.dat` byte column at
// `0x00f8f1e0` by the unit constructor and rewritten by the order-state
// helpers) has no counterpart in [`SimUnit`], so the routines that test it
// take it as a parameter, the same way batch 15 takes the unmodeled
// `orders.dat` rows from its caller.

// ---------------------------------------------------------------------------
// 0x005ac009 - interaction target picker with the overlay-dependent cue
// ---------------------------------------------------------------------------

impl SimUnit {
    /// `0x005ac009` (`FUN_005ac009`, 134 bytes).
    ///
    /// ```c
    /// iVar1 = FUN_005c69f0(point, 0);          // near-point unit query
    /// if ((iVar1 == 0) || (iVar1 == excluded)) {
    ///   *(u8 *)(this + 0x4e) = 3;              // reference state byte
    ///   if (FUN_005b5630() != 0) {             // CUnit+0xe0 bit 1
    ///     FUN_005b7d00(0x5a);                  // cue 0x5a, CUnit+0x93 |= 1
    ///     return;
    ///   }
    ///   FUN_0056c820(this->x, this->y);
    ///   FUN_005b7250(0x56, excluded);
    /// } else {
    ///   *(u32 *)(this + 0xc8) = iVar1;
    ///   *(u32 *)(this + 0x5c) = iVar1;
    /// }
    /// ```
    ///
    /// The near-point query and the `+0xc8` selection slot are not part of
    /// the Rust model, so the resolved candidate is supplied by the caller
    /// (exactly like the recovered click rows in batch 15).  The adoption is
    /// the `+0x5c` target write, [`SimUnit::set_target`]; the fallback layers
    /// return the cue the caller has to play, and the `0x5a` path also sets
    /// the `CUnit+0x93` bit 0 through [`SimUnit::flags_93`].
    ///
    /// Returns `None` when a candidate was adopted, otherwise `Some(cue)`.
    pub fn a27_choose_interaction_target(
        &mut self,
        candidate: Option<u32>,
        excluded: u32,
    ) -> Option<u8> {
        match candidate {
            Some(id) if id != excluded && id != 0 => {
                self.set_target(id);
                None
            }
            _ => {
                if self.overlay_bit1() {
                    self.flags_93 |= 1;
                    Some(0x5a)
                } else {
                    Some(0x56)
                }
            }
        }
    }

    /// `0x005ac06d` (`FUN_005ac06d`, 52 bytes).
    ///
    /// ```c
    /// iVar1 = FUN_005c6970(point);             // query with the other filter
    /// if (iVar1 == 0) { func_0x005b8040(); return; }
    /// *(u32 *)(this + 0xc8) = iVar1;
    /// *(u32 *)(this + 0x5c) = iVar1;
    /// ```
    ///
    /// The candidate comes from the second near-point query, which is again
    /// outside the data model.  Returns true when the target (`+0x5c`) was
    /// adopted.
    pub fn a27_adopt_interaction_target(&mut self, candidate: Option<u32>) -> bool {
        match candidate {
            Some(id) if id != 0 => {
                self.set_target(id);
                true
            }
            _ => false,
        }
    }

    /// `0x005ac0fd` (`FUN_005ac0fd`, 14 bytes).
    ///
    /// ```c
    /// func_0x005b8040();
    /// return;
    /// ```
    ///
    /// The body consists of the single call to the unresolved runtime helper
    /// `0x005b8040` and then returns.  No `CUnit` slot is read or written, so
    /// the translation is an inert hook (the same shape as the no-op vtable
    /// slot translated in batch 02).
    pub fn a27_command_flush_hook(&self) {}
}

// ---------------------------------------------------------------------------
// 0x005ac28e - dialog cue for the null/non-null selector
// ---------------------------------------------------------------------------

/// Destination of the cue selected by `0x005ac28e`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A27CueRoute {
    /// The selection slot was zero, so the cue is played on the global
    /// channel.
    Global(u8),
    /// The selection slot was set, so the cue is forwarded to that unit.
    ToSelection(u8, u32),
}

/// `0x005ac28e` (`FUN_005ac28e`, 17 bytes).
///
/// ```c
/// iVar2 = (-(in_EAX != 0) & 0xfffffffc) + 0x55;
/// if (*(int *)(this + 0xc8) == 0) { FUN_005b71a0(iVar2); }
/// else                            { FUN_005b7250(iVar2, *(int *)(this + 0xc8)); }
/// ```
///
/// The pointer test turns into a `-4` bias, so a present selector selects the
/// cue `0x51` and a null selector the cue `0x55`.  The strict negation keeps
/// the two cues four apart, matching the block layout the neighboring dialog
/// sound selector uses.
pub fn a27_null_selector_cue(selector_present: bool) -> u8 {
    if selector_present { 0x51 } else { 0x55 }
}

/// `0x005ac28e` (`FUN_005ac28e`, 17 bytes): pick the cue and its destination.
///
/// `selection` is the `+0xc8` slot; `None` is the decompiled `== 0` test.
/// The trailing overlay clear and the final unresolved calls of the
/// decompiled body touch no modeled field, so they are not reproduced here.
pub fn a27_route_null_selector_cue(selector_present: bool, selection: Option<u32>) -> A27CueRoute {
    let cue = a27_null_selector_cue(selector_present);
    match selection {
        Some(id) if id != 0 => A27CueRoute::ToSelection(cue, id),
        _ => A27CueRoute::Global(cue),
    }
}

// ---------------------------------------------------------------------------
// 0x005ac350 - first owned unit whose target is the receiver
// ---------------------------------------------------------------------------

impl World {
    /// `0x005ac350` (`FUN_005ac350`, 79 bytes).
    ///
    /// ```c
    /// u = first_unit_of_player(this->owner);        // 0x005b0fb0(player)
    /// while (u != 0) {
    ///   if (u->byte_4d == 0x53 && u->is_worker() && sprite_bit5(u) &&
    ///       u->target_5c == this) break;
    ///   u = u->next_in_player_list();               // u->+0x6c
    /// }
    /// return u;
    /// ```
    ///
    /// `FUN_005b64c0` is the `units.dat` worker bit (`UnitType::is_worker`)
    /// and `FUN_005b5d40` reads `CSprite+0xe` bit 5, which the model keeps as
    /// [`crate::sim::Status::cloaked`].  The per-type byte at `CUnit+0x4d` is
    /// not modeled, so the caller supplies it; the retail list order is the
    /// creation order of [`World::units`], and dead units have already been
    /// removed from the retail list, hence the `alive` filter.
    pub fn a27_find_worker_targeting(
        &self,
        target_id: u32,
        byte_4d: impl Fn(&SimUnit) -> u8,
    ) -> Option<u32> {
        let target = self.unit(target_id)?;
        self.units
            .iter()
            .find(|u| {
                u.alive
                    && u.owner == target.owner
                    && self.data.unit(u.type_id).is_worker()
                    && u.status.cloaked
                    && byte_4d(u) == 0x53
                    && u.target == Some(target_id)
            })
            .map(|u| u.id)
    }
}

// ---------------------------------------------------------------------------
// 0x005ac531 - overlay refresh: clear, bump the frame, set again
// ---------------------------------------------------------------------------

impl SimUnit {
    /// `0x005ac531` (`FUN_005ac531`, 150 bytes).
    ///
    /// ```c
    /// state = 8;                                   // +0xe0 companion frame
    /// if (result_of_vtable_slot_0x10 != &DAT_00d07d80) {
    ///   if (FUN_005b5620() != 0 || FUN_005b5630() != 0) {  // +0xe0 bits 0/1
    ///     FUN_005b3650();                          // drop the overlay sprite
    ///     *(u8 *)(this + 0xe0) = 0;                // clear the overlay bits
    ///   }
    ///   if (FUN_006354b0(this) != 0) state = 9;    // AI-state bump
    ///   FUN_005b31c0(image, state, mode);          // set overlay + 0xcf
    /// }
    /// ```
    ///
    /// The overlay is cleared before it is set again, exactly like the
    /// retail order.  `FUN_005b31c0` is [`SimUnit::set_overlay_state`], so the
    /// method also writes the companion byte `+0xcf`.  The AI gate argument
    /// stands in for `FUN_006354b0`, which reads the player's campaign-AI
    /// state record and is not part of the simulation model.
    ///
    /// Returns the frame byte that was written to `+0xcf`.
    pub fn a27_refresh_overlay_frame(&mut self, ai_frame_bump: bool, mode: bool) -> u8 {
        if self.overlay_bit0() || self.overlay_bit1() {
            self.overlay_flags = 0;
        }
        let frame = if ai_frame_bump { 9 } else { 8 };
        self.set_overlay_state(mode, frame);
        frame
    }
}

// ---------------------------------------------------------------------------
// 0x005ad558 / 0x005ad5ff - enter a single-slot container
// ---------------------------------------------------------------------------

/// The two cues produced by the container-entry tail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A27ContainerEntry {
    /// Cue played on the global channel before the main cue, when the entering
    /// unit's `CUnit+0x4d` byte is not `0x52` (`FUN_005b71a0(0x58)`).
    pub pre_cue: Option<u8>,
    /// Cue played on the container (`FUN_005b7250`).
    pub cue: u8,
}

/// The cue selection of the `0x005ad558` / `0x005ad5ff` tail.
///
/// ```c
/// if (*(char *)(unit + 0x4d) == 'R') cue = 0x53;   // 'R' = 0x52
/// else { FUN_005b71a0(0x58); cue = 0x57; }
/// ```
///
/// `byte_4d` is the per-type byte the unit constructor fills from the
/// `units.dat` column at `0x00f8f1e0`; it has no `SimUnit` counterpart.
pub fn a27_container_entry_cues(byte_4d: u8) -> A27ContainerEntry {
    if byte_4d == 0x52 {
        A27ContainerEntry {
            pre_cue: None,
            cue: 0x53,
        }
    } else {
        A27ContainerEntry {
            pre_cue: Some(0x58),
            cue: 0x57,
        }
    }
}

impl World {
    /// Shared tail of `0x005ad558` and `0x005ad5ff`: one-slot container
    /// entry.
    ///
    /// ```c
    /// if (*(u8 *)(carrier + 0xd3) == 0) {
    ///   *(u8 *)(carrier + 0xd3) = 1;               // container busy
    ///   *(u8 *)(unit + 0xce) = 1;                  // containment flag
    ///   *(u32 *)(unit + 0xd0) = carrier;           // containment link
    ///   play(a27_container_entry_cues(unit->byte_4d));
    ///   return 1;
    /// }
    /// return 0;
    /// ```
    ///
    /// The retail `+0xd3` busy byte and the `+0xce`/`+0xd0` link are modeled
    /// by [`SimUnit::cargo`] and [`SimUnit::in_transport`].  The routine only
    /// ever accepts one occupant (the busy byte is *set*, never incremented),
    /// which is the single-slot behavior reproduced with "cargo must be
    /// empty".
    fn a27_enter_container_body(
        &mut self,
        unit_id: u32,
        carrier_id: u32,
        byte_4d: u8,
    ) -> Result<Option<A27ContainerEntry>, SimError> {
        let carrier = self.index_of(carrier_id).ok_or(SimError::BadUnit)?;
        let unit = self.index_of(unit_id).ok_or(SimError::BadUnit)?;
        if carrier == unit || !self.units[carrier].cargo.is_empty() {
            return Ok(None);
        }
        let entry = a27_container_entry_cues(byte_4d);
        self.units[carrier].cargo.push(unit_id);
        self.units[unit].in_transport = Some(carrier_id);
        Ok(Some(entry))
    }

    /// `0x005ad558` (`FUN_005ad558`, 167 bytes): enter a one-slot container.
    ///
    /// The decompiled body runs a large effect call (`FUN_005a6990` with the
    /// nine panel arguments) and a `rdtsc`-guarded loop over the local frame
    /// before reaching the shared tail; neither touches `CUnit` state that
    /// the model owns.  The tail is the container check and the cue selection
    /// documented on [`Self::a27_enter_container_body`].
    ///
    /// `byte_4d` is the entering unit's `CUnit+0x4d` byte (see
    /// [`a27_container_entry_cues`]).  Returns `Ok(None)` when the container
    /// already has an occupant, which is the decompiled `return 0`.
    pub fn a27_enter_container(
        &mut self,
        unit_id: u32,
        carrier_id: u32,
        byte_4d: u8,
    ) -> Result<Option<A27ContainerEntry>, SimError> {
        self.a27_enter_container_body(unit_id, carrier_id, byte_4d)
    }

    /// `0x005ad5ff` (`FUN_005ad5ff`, 163 bytes): the second entry point for
    /// the same container tail.
    ///
    /// The prefix walks a local pointer list (`EBP-0x2c..EBP-0x7c`) that the
    /// decompiled frame does not name as a game object; once that walk ends it
    /// falls into the identical busy-byte/link/cue block.  The resolved unit
    /// and container are therefore the arguments, exactly as in
    /// [`Self::a27_enter_container`]; the observable result is the same.
    pub fn a27_enter_container_scanned(
        &mut self,
        unit_id: u32,
        carrier_id: u32,
        byte_4d: u8,
    ) -> Result<Option<A27ContainerEntry>, SimError> {
        self.a27_enter_container_body(unit_id, carrier_id, byte_4d)
    }

    /// `0x005ad780` (`FUN_005ad780`, 150 bytes).
    ///
    /// ```c
    /// if (unit->byte_ce != 0) { unload_path(carrier); return; }
    /// node = carrier->cargo_head_0xd4;
    /// while (node != 0) {
    ///   next = node->next_0xd8;
    ///   if (node == unit) {
    ///     if (node->prev_0xd4 == 0) carrier->cargo_head_0xd4 = next;
    ///     else node->prev_0xd4->next_0xd8 = next;
    ///     if (next != 0) next->prev_0xd4 = node->prev_0xd4;
    ///     node->prev_0xd4 = node->next_0xd8 = node->container_0xd0 = 0;
    ///     return;
    ///   }
    ///   node = next;
    /// }
    /// ```
    ///
    /// The doubly linked list is [`SimUnit::cargo`], the containment flag and
    /// back link are [`SimUnit::in_transport`].  The in-container branch
    /// delegates to the retail unload path in the reference build; the Rust
    /// body performs the equivalent detach from that container.
    ///
    /// Returns true when the unit was linked to a container (either branch).
    pub fn a27_remove_cargo(&mut self, carrier_id: u32, unit_id: u32) -> bool {
        if let Some(container) = self.unit(unit_id).and_then(|u| u.in_transport) {
            if let Some(container_unit) = self.unit_mut(container)
                && let Some(pos) = container_unit.cargo.iter().position(|&id| id == unit_id)
            {
                container_unit.cargo.remove(pos);
            }
            if let Some(unit) = self.unit_mut(unit_id) {
                unit.in_transport = None;
            }
            return true;
        }

        let Some(carrier) = self.unit_mut(carrier_id) else {
            return false;
        };
        let Some(pos) = carrier.cargo.iter().position(|&id| id == unit_id) else {
            return false;
        };
        carrier.cargo.remove(pos);
        if let Some(unit) = self.unit_mut(unit_id) {
            unit.in_transport = None;
        }
        true
    }
}

// ---------------------------------------------------------------------------
// 0x005ad8a0 / 0x005ad950 - CUnit+0x64 type-id getters
// ---------------------------------------------------------------------------

impl SimUnit {
    /// `0x005ad8a0` (`FUN_005ad8a0`, 23 bytes).
    ///
    /// ```c
    /// if ((*(u32 *)(this + 0xdc) & 1) != 0) return *(u16 *)(this + 0x64);
    /// return 0;
    /// ```
    ///
    /// `CUnit+0x64` is the `units.dat` type id: `FUN_005b4cd0` returns that
    /// word and the flag/HP getters index the `units.dat` columns with it.
    /// The `CUnit+0xdc` bit-0 test is not one of the flag bits the Rust model
    /// keeps, so it is an argument.  Both call sites treat a nonzero result as
    /// "a unit of a known type is selected", which matches the decompiled
    /// `return 0` on the not-flagged arm.
    pub fn a27_flagged_type_id(&self, flag_bit0: bool) -> u16 {
        if flag_bit0 { self.type_id } else { 0 }
    }

    /// `0x005ad950` (`FUN_005ad950`, 8 bytes).
    ///
    /// The whole body is `return *(u16 *)(this + 0x64)`, the same getter as
    /// `FUN_005b4cd0`; `CUnit+0x64` is the `units.dat` type id, which the
    /// simulation stores as [`SimUnit::type_id`].
    pub fn a27_type_id_word(&self) -> u16 {
        self.type_id
    }
}

#[cfg(test)]
mod tests_agent27 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, owner: usize, type_id: u16) -> (u32, usize) {
        let index = world.spawn(owner, type_id, 512, 512).unwrap();
        (world.units[index].id, index)
    }

    #[test]
    fn choose_interaction_target_adopts_a_foreign_candidate() {
        // 0x005ac009: `candidate == 0 || candidate == excluded` is false.
        let mut w = world();
        let (this, i) = spawn(&mut w, 0, uid::MARINE);
        let (other, _) = spawn(&mut w, 0, uid::SCV);
        assert_eq!(w.units[i].target, None);
        assert_eq!(
            w.units[i].a27_choose_interaction_target(Some(other), this),
            None
        );
        assert_eq!(w.units[i].target, Some(other));
    }

    #[test]
    fn choose_interaction_target_falls_back_to_the_cue() {
        // 0x005ac009: the `candidate == 0 || candidate == excluded` arm sets
        // the state byte, then tests CUnit+0xe0 bit 1 for the 0x5a cue.
        let mut w = world();
        let (this, i) = spawn(&mut w, 0, uid::MARINE);

        w.units[i].overlay_flags = 0;
        w.units[i].flags_93 = 0;
        assert_eq!(
            w.units[i].a27_choose_interaction_target(None, this),
            Some(0x56)
        );
        assert_eq!(w.units[i].flags_93, 0, "0x56 does not set CUnit+0x93");

        // Candidate equal to the excluded unit takes the same arm.
        assert_eq!(
            w.units[i].a27_choose_interaction_target(Some(this), this),
            Some(0x56)
        );
        assert_eq!(w.units[i].target, None);

        // CUnit+0xe0 bit 1 selects cue 0x5a and sets CUnit+0x93 bit 0.
        w.units[i].overlay_flags = 2;
        assert_eq!(
            w.units[i].a27_choose_interaction_target(None, this),
            Some(0x5a)
        );
        assert_eq!(w.units[i].flags_93 & 1, 1);
        w.units[i].overlay_flags = 1; // bit 0 only
        assert_eq!(
            w.units[i].a27_choose_interaction_target(None, this),
            Some(0x56)
        );
    }

    #[test]
    fn adopt_interaction_target_matches_the_decompiled_branch() {
        // 0x005ac06d: a zero query result returns through the flush hook,
        // anything else stores the candidate at +0xc8 and +0x5c.
        let mut w = world();
        let (_this, i) = spawn(&mut w, 0, uid::MARINE);
        let (other, _) = spawn(&mut w, 1, uid::SCV);
        assert!(!w.units[i].a27_adopt_interaction_target(None));
        assert_eq!(w.units[i].target, None);
        assert!(w.units[i].a27_adopt_interaction_target(Some(other)));
        assert_eq!(w.units[i].target, Some(other));
    }

    #[test]
    fn command_flush_hook_touches_no_state() {
        // 0x005ac0fd: the only call is the unresolved helper 0x005b8040.
        let mut w = world();
        let (other, _) = spawn(&mut w, 0, uid::SCV);
        let (_, i) = spawn(&mut w, 0, uid::MARINE);
        w.units[i].set_target(other);
        let before = w.units[i].clone();
        w.units[i].a27_command_flush_hook();
        assert_eq!(w.units[i].target, before.target);
        assert_eq!(w.units[i].order, before.order);
        assert_eq!(w.units[i].flags_93, before.flags_93);
    }

    #[test]
    fn null_selector_cue_uses_the_null_bit() {
        // 0x005ac28e: (-(ptr != 0) & 0xfffffffc) + 0x55.
        assert_eq!(a27_null_selector_cue(true), 0x51);
        assert_eq!(a27_null_selector_cue(false), 0x55);
    }

    #[test]
    fn null_selector_cue_routes_to_the_selection() {
        // 0x005ac28e: the +0xc8 slot decides between FUN_005b71a0 and
        // FUN_005b7250.
        assert_eq!(
            a27_route_null_selector_cue(true, None),
            A27CueRoute::Global(0x51)
        );
        assert_eq!(
            a27_route_null_selector_cue(false, Some(0)),
            A27CueRoute::Global(0x55)
        );
        assert_eq!(
            a27_route_null_selector_cue(false, Some(0x2a)),
            A27CueRoute::ToSelection(0x55, 0x2a)
        );
    }

    #[test]
    fn find_worker_targeting_applies_every_gate() {
        // 0x005ac350: byte +0x4d == 0x53, units.dat worker bit, CSprite+0xe
        // bit 5, and target +0x5c == receiver, all on the same owner.
        let mut w = world();
        let (target, _) = spawn(&mut w, 0, uid::MARINE);
        let (candidate, c) = spawn(&mut w, 0, uid::SCV);
        w.units[c].target = Some(target);
        w.units[c].status.cloaked = true;

        assert_eq!(
            w.a27_find_worker_targeting(target, |_| 0x53),
            Some(candidate)
        );
        // The per-type byte gate.
        assert_eq!(w.a27_find_worker_targeting(target, |_| 0x52), None);
        // The worker gate on the candidate.
        let (marine, m) = spawn(&mut w, 0, uid::MARINE);
        w.units[m].target = Some(target);
        w.units[m].status.cloaked = true;
        w.units[c].status.cloaked = false;
        assert_eq!(w.a27_find_worker_targeting(target, |_| 0x53), None);
        // The sprite bit gate.
        w.units[m].status.cloaked = false;
        w.units[c].status.cloaked = true;
        assert_eq!(
            w.a27_find_worker_targeting(target, |_| 0x53),
            Some(candidate)
        );
        // The target gate and the owner gate.
        w.units[c].target = Some(marine);
        assert_eq!(w.a27_find_worker_targeting(target, |_| 0x53), None);
        w.units[c].target = Some(target);
        w.units[c].owner = 1;
        assert_eq!(w.a27_find_worker_targeting(target, |_| 0x53), None);
    }

    #[test]
    fn refresh_overlay_frame_clears_then_sets() {
        // 0x005ac531: bits 0/1 are cleared, then the frame goes through
        // FUN_005b31c0 with the AI-dependent 8/9 step.
        let mut w = world();
        let (_, i) = spawn(&mut w, 0, uid::MARINE);

        w.units[i].overlay_flags = 3;
        w.units[i].field_0cf = 0x77;
        assert_eq!(w.units[i].a27_refresh_overlay_frame(false, false), 8);
        assert_eq!(w.units[i].overlay_flags, 1);
        assert_eq!(w.units[i].field_0cf, 8);

        w.units[i].overlay_flags = 0;
        assert_eq!(w.units[i].a27_refresh_overlay_frame(true, true), 9);
        assert_eq!(w.units[i].overlay_flags, 2);
        assert_eq!(w.units[i].field_0cf, 9);

        // No bit set: the clear is skipped but the set still happens.
        w.units[i].overlay_flags = 0;
        assert_eq!(w.units[i].a27_refresh_overlay_frame(false, true), 8);
        assert_eq!(w.units[i].overlay_flags, 2);
    }

    #[test]
    fn container_entry_cues_follow_the_byte() {
        // 0x005ad558 / 0x005ad5ff tail.
        assert_eq!(
            a27_container_entry_cues(0x52),
            A27ContainerEntry {
                pre_cue: None,
                cue: 0x53
            }
        );
        assert_eq!(
            a27_container_entry_cues(0x53),
            A27ContainerEntry {
                pre_cue: Some(0x58),
                cue: 0x57
            }
        );
    }

    #[test]
    fn enter_container_links_the_first_occupant_only() {
        // 0x005ad558: `+0xd3 == 0` sets the busy byte, the containment flag
        // and the back link.
        let mut w = world();
        let (carrier, c) = spawn(&mut w, 0, uid::DROPSHIP);
        let (first, f) = spawn(&mut w, 0, uid::MARINE);
        let (second, s) = spawn(&mut w, 0, uid::MARINE);

        let entry = w.a27_enter_container(first, carrier, 0x52).unwrap();
        assert_eq!(entry.unwrap().cue, 0x53);
        assert_eq!(w.units[c].cargo, vec![first]);
        assert_eq!(w.units[f].in_transport, Some(carrier));

        // The container is busy: `return 0`.
        assert_eq!(w.a27_enter_container(second, carrier, 0x52).unwrap(), None);
        assert_eq!(w.units[c].cargo, vec![first]);
        assert_eq!(w.units[s].in_transport, None);

        // Unknown ids are rejected rather than unwrapped.
        assert!(matches!(
            w.a27_enter_container(0xffff_ff00, carrier, 0x52),
            Err(SimError::BadUnit)
        ));
    }

    #[test]
    fn scanned_enter_container_matches_the_shared_tail() {
        // 0x005ad5ff: the same busy-byte/link/cue block after the local list
        // walk.
        let mut w = world();
        let (carrier, c) = spawn(&mut w, 0, uid::DROPSHIP);
        let (unit, u) = spawn(&mut w, 0, uid::MARINE);
        let entry = w
            .a27_enter_container_scanned(unit, carrier, 0x53)
            .unwrap()
            .unwrap();
        assert_eq!(entry.pre_cue, Some(0x58));
        assert_eq!(entry.cue, 0x57);
        assert_eq!(w.units[c].cargo, vec![unit]);
        assert_eq!(w.units[u].in_transport, Some(carrier));
    }

    #[test]
    fn remove_cargo_unlinks_the_list() {
        // 0x005ad780: the second branch walks the carrier's +0xd4 list.
        let mut w = world();
        let (carrier, c) = spawn(&mut w, 0, uid::DROPSHIP);
        let (unit, u) = spawn(&mut w, 0, uid::MARINE);
        w.units[c].cargo = vec![unit];
        w.units[u].in_transport = None;
        assert!(w.a27_remove_cargo(carrier, unit));
        assert!(w.units[c].cargo.is_empty());
        assert_eq!(w.units[u].in_transport, None);
        assert!(!w.a27_remove_cargo(carrier, unit));
    }

    #[test]
    fn remove_cargo_uses_the_container_link() {
        // 0x005ad780: a unit whose +0xce flag is set goes through the unload
        // branch and is detached from the container it records at +0xd0.
        let mut w = world();
        let (carrier, c) = spawn(&mut w, 0, uid::DROPSHIP);
        let (unit, u) = spawn(&mut w, 0, uid::MARINE);
        w.units[c].cargo = vec![unit];
        w.units[u].in_transport = Some(carrier);
        // The first argument is irrelevant once the unit is in a container.
        assert!(w.a27_remove_cargo(0xdead_beef, unit));
        assert!(w.units[c].cargo.is_empty());
        assert_eq!(w.units[u].in_transport, None);
    }

    #[test]
    fn flagged_type_id_reports_the_type_only_when_flagged() {
        // 0x005ad8a0: CUnit+0xdc bit 0 gates the CUnit+0x64 read.
        let mut w = world();
        let (_, i) = spawn(&mut w, 0, uid::MARINE);
        assert_eq!(w.units[i].a27_flagged_type_id(true), uid::MARINE);
        assert_eq!(w.units[i].a27_flagged_type_id(false), 0);
    }

    #[test]
    fn type_id_word_matches_the_spawned_type() {
        // 0x005ad950: the bare CUnit+0x64 getter.
        let mut w = world();
        let (_, i) = spawn(&mut w, 0, uid::HYDRALISK);
        assert_eq!(w.units[i].a27_type_id_word(), uid::HYDRALISK);
        assert_eq!(w.units[i].a27_type_id_word(), w.units[i].type_id);
    }
}
