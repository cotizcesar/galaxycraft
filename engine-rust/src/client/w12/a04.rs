#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave12/agent04.rs
// ===========================================================================
// Wave 12 translation batch 04; temporary staging file.
//
// Rows translated: the `decompiled_uncited` entries in
// `0x00658000..=0x00659fff` whose recovered slice has an observable,
// side-effect-free core.  The window is the client-side **selection /
// overlay-record** layer: `CUnit` records are 0x34 bytes, an overlay is a
// 0x34-byte record reached through `CUnit+0x134`, and per-player overlay
// lists are intrusive doubly-linked lists with a free list threaded
// through the owning player object.
//
// Every entry point below takes the owning state explicitly (list head,
// free-list head, record table, callbacks for the retail lookups) so the
// recovered arithmetic can be tested without adding fields to
// [`crate::sim::World`] or [`crate::sim::SimUnit`].  Calls into routines
// that were *not* translated here (sprite creation, position lookup,
// rectangle test, the unit-type flag table) are represented by closures
// or by the returned slot, and are named only by role in the comments.
//
// Skipped rows and why:
//
// * `FUN_00658030`, `FUN_00658160`, `FUN_00658240`, `FUN_00658360`: target
//   re-selection, the selected-unit detach loop and the stack-array/random
//   candidate picker; all dispatch through untranslated selection,
//   predicate and unit-creation routines.
// * `FUN_006585a7` and `FUN_00658b0d..FUN_00658c58` (including `FUN_00658bd2`):
//   mid-function fragments of larger routines (stack-cookie/SEH copies),
//   not standalone bodies.
// * `FUN_006586a0`: destructor guard, only frees `*this` when non-null.
// * `FUN_006586c6`: second copy of the list-move body at `0x006586c0`.
// * `FUN_00658720`, `FUN_00658a70`, `FUN_00658aa2`, `FUN_00658abb`,
//   `FUN_00658c6b`, `FUN_00658d88`..`FUN_00658e3f`: compiler-generated
//   exception/`__security_check_cookie` unwind fragments.
// * `FUN_0065897f`: pool-list registration whose only work is copying six
//   pointer words into the fixed client tables.
// * `FUN_00658f10`: guard that forwards to two untranslated client routines.
// * `FUN_006591a9`, `FUN_00659270`, `FUN_006592aa`, `FUN_00659481`,
//   `FUN_00659590`, `FUN_00659645`, `FUN_00659840`: partial bodies of the
//   list/overlay allocator and pool reset whose remaining stores are
//   supplied by untranslated callees.
// * `FUN_0065991e`..`FUN_00659a23`, `FUN_00659ad9`, `FUN_00659dd8`: MSVC
//   checked-iterator fragments (`field < 0x80` selector plus
//   `_invalid_parameter`), including the out-of-line continuation of the
//   candidate loop, with no observable result of its own.
// * `FUN_00659b10`, `FUN_00659d10`, `FUN_00659d51`, `FUN_00659e61`,
//   `FUN_00659ea9`, `FUN_00659ee4`: one-line throw/abort pads and
//   checked-iterator guards.  `0x00659e20` and `0x00659f20` (whose cached
//   bodies are throw fragments) are translated as the slot scorers below
//   from the shipped image, where their eight-record loops are intact.

/// Base of the fixed overlay-record pool (`0x00fd7ba0` in the retail
/// image); records are 0x20 bytes apart.
pub const W12A04_POOL_BASE: u32 = 0x00fd_7ba0;

/// Stride between overlay-pool records.
pub const W12A04_POOL_STRIDE: u32 = 0x20;

/// Selected bit in a record's `+8` flag byte.
pub const W12A04_SELECTED: u8 = 0x10;

/// One recovered 0x34-byte client record.
///
/// Only the fields touched by this batch are modelled: the intrusive
/// `next`/`prev` links, the back-link/owner word at `+0x0c`, the type word
/// at `+0x10`, the current positions at `+0x12`/`+0x14`, the home
/// positions at `+0x16`/`+0x18` and the frame timer at `+0x1c`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12A04Entry {
    /// `+0x00` next list node.
    pub next: Option<usize>,
    /// `+0x04` previous list node.
    pub prev: Option<usize>,
    /// `+0x0c` owning unit, `None` for an unowned record.
    pub owner: Option<usize>,
    /// `+0x10` record type word.
    pub type_id: u16,
    /// `+0x12` first position word.
    pub x: i16,
    /// `+0x14` second position word.
    pub y: i16,
    /// `+0x16` home first word.
    pub home_x: i16,
    /// `+0x18` home second word.
    pub home_y: i16,
    /// `+0x1c` frame timer; zero means "not armed".
    pub timer: i32,
}

/// One recovered overlay record reached through `CUnit+0x134`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12A04Overlay {
    /// `+0x08` overlay kind byte.
    pub kind: u8,
    /// `+0x09` saturating attempt counter.
    pub counter: u8,
    /// `+0x0c` owning unit back-link.
    pub owner: Option<usize>,
    /// `+0x12` current first target word.
    pub x: i16,
    /// `+0x14` current second target word.
    pub y: i16,
    /// `+0x16` home first word copied on attach.
    pub home_x: i16,
    /// `+0x18` home second word copied on attach.
    pub home_y: i16,
}

/// One 0x24-byte slot record used by the recovered candidate scoring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12A04SlotRecord {
    /// `+0x08` slot state byte.
    pub state: u8,
    /// `+0x0a` slot index byte compared with the candidate number.
    pub slot: u8,
}

// ===========================================================================
// `0x00658080`: first selected slot used by the follow overlay
// ===========================================================================

/// `0x00658080`: report the first selection slot past slot 0 that carries
/// the selected bit in its `+8` flag byte.
///
/// Retail reads the selection count and the per-player record array, then
/// walks slots `1 .. count` (slot 0 is never considered) and, on the first
/// `flags & 0x10`, creates the `0x7c` x `0x50` follow overlay for
/// `param_1`.  This translation returns the 1-based slot; creation stays
/// with the caller.  Fewer than two records, or no flagged slot, yields
/// `None`.
pub fn w12a04_first_flagged_selection_slot(flags: &[u8]) -> Option<usize> {
    if flags.len() <= 1 {
        return None;
    }
    (1..flags.len()).find(|&slot| flags[slot] & W12A04_SELECTED != 0)
}

/// `0x006580d0`: take the first selected slot past slot 0.
///
/// Same walk as [`w12a04_first_flagged_selection_slot`]; retail then
/// clears the selected bit (`+8 &= ~0x10`) and removes the slot from the
/// selection.  Returns the slot, or `None` when the walk reaches the
/// selection count as well.
pub fn w12a04_take_flagged_selection_slot(flags: &mut [u8]) -> Option<usize> {
    if flags.len() <= 1 {
        return None;
    }
    let slot = (1..flags.len()).find(|&slot| flags[slot] & W12A04_SELECTED != 0)?;
    flags[slot] &= !W12A04_SELECTED;
    Some(slot)
}

// ===========================================================================
// Overlay/target predicates
// ===========================================================================

/// `0x00658130`: may the current target keep its overlay?
///
/// Retail reads the unit status word at `+0xdc` bit 8 and the order/overlay
/// kind byte at `+0x4d`.  A set status bit with a kind other than `0x6a`
/// or `0x96` rejects the target; every other combination is accepted.
pub fn w12a04_overlay_target_permitted(status_bit8: bool, kind_byte: u8) -> bool {
    if status_bit8 && kind_byte != 0x6a && kind_byte != 0x96 {
        return false;
    }
    true
}

/// `0x00658670`: is the unit carrying a kind-4 overlay that owns it?
///
/// Retail checks that `CUnit+0x134` is non-null, that the overlay kind byte
/// is `4`, that the overlay back-link at `+0x0c` is the unit itself, and
/// that the unit-type flag word's bit 0 is clear.  `type_flag0` carries
/// that last lookup.
pub fn w12a04_overlay_owns_unit(
    overlay_kind: Option<u8>,
    owner_is_unit: bool,
    type_flag0: bool,
) -> bool {
    overlay_kind == Some(4) && owner_is_unit && !type_flag0
}

/// `0x006591b0`: write the target words into a kind-1 overlay.
///
/// Retail only stores when `CUnit+0x134` is non-null and the overlay kind
/// byte is `1`; both 16-bit words are written unconditionally.
pub fn w12a04_store_overlay_anchor(
    overlay: Option<&mut W12A04Overlay>,
    first: i16,
    second: i16,
) -> bool {
    match overlay {
        Some(overlay) if overlay.kind == 1 => {
            overlay.x = first;
            overlay.y = second;
            true
        }
        _ => false,
    }
}

// ===========================================================================
// Intrusive overlay lists
// ===========================================================================

/// `0x006586c0`: unlink `node` from its list and push it onto the free
/// list.
///
/// Retail fixes up the head when `node` is the head, stitches the
/// neighbours, clears the node's previous link, links it in front of the
/// owner's free-list head (`+0x7d00`) and updates the new head's previous
/// pointer.  `entries` is the shared record table, `list_head` the
/// per-player live head and `free_head` the owner's free-list head.
/// Returns `false` when `node` is out of range.
pub fn w12a04_list_move_to_free_head(
    entries: &mut [W12A04Entry],
    list_head: &mut Option<usize>,
    free_head: &mut Option<usize>,
    node: usize,
) -> bool {
    if node >= entries.len() {
        return false;
    }
    let next = entries[node].next;
    let prev = entries[node].prev;

    if *list_head == Some(node) {
        *list_head = next;
    }
    if let Some(prev) = prev {
        if let Some(prev) = entries.get_mut(prev) {
            prev.next = next;
        }
    }
    if let Some(next) = next {
        if let Some(next) = entries.get_mut(next) {
            next.prev = prev;
        }
    }

    entries[node].prev = None;
    entries[node].next = *free_head;
    if let Some(free) = *free_head {
        if let Some(free) = entries.get_mut(free) {
            free.prev = Some(node);
        }
    }
    *free_head = Some(node);
    true
}

/// `0x00659120`: move every unowned record of one player's list to that
/// player's free list.
///
/// Retail walks the live list and, for every record whose `+0x0c` word is
/// zero, performs the same unlink/push as
/// [`w12a04_list_move_to_free_head`], restarting from the (possibly new)
/// head after each removal.  Returns the number of records reclaimed.
pub fn w12a04_sweep_unowned(
    entries: &mut [W12A04Entry],
    list_head: &mut Option<usize>,
    free_head: &mut Option<usize>,
) -> usize {
    let mut reclaimed = 0usize;
    let mut cursor = *list_head;
    while let Some(node) = cursor {
        let Some(entry) = entries.get(node) else {
            break;
        };
        let next = entry.next;
        if entry.owner.is_none() {
            w12a04_list_move_to_free_head(entries, list_head, free_head, node);
            reclaimed += 1;
        }
        cursor = next;
    }
    reclaimed
}

/// `0x006594e0`: attach an overlay record to its unit.
///
/// Retail copies the home words at `+0x16`/`+0x18` into the target words at
/// `+0x12`/`+0x14`, stores the unit at `+0x0c` and writes the overlay back
/// into `CUnit+0x134`.  The last store belongs to the caller; the owner
/// index is returned so it can perform it.
pub fn w12a04_attach_overlay(overlay: &mut W12A04Overlay, owner: usize) -> usize {
    overlay.x = overlay.home_x;
    overlay.y = overlay.home_y;
    overlay.owner = Some(owner);
    owner
}

/// `0x006596e0`: consume the pending target of a kind-1 overlay.
///
/// Retail bails out unless the overlay exists and its kind byte is `1`.
/// Otherwise it clears the owner word, increments the `+0x09` counter while
/// it is below 100, and reports success.
pub fn w12a04_consume_overlay_target(overlay: Option<&mut W12A04Overlay>) -> bool {
    match overlay {
        Some(overlay) if overlay.kind == 1 => {
            overlay.owner = None;
            if overlay.counter < 100 {
                overlay.counter += 1;
            }
            true
        }
        _ => false,
    }
}

// ===========================================================================
// Overlay-list queries
// ===========================================================================

/// `0x00659530`: does a live record of `type_id` have a resolvable target?
///
/// Retail walks the per-player list, keeps records whose `+0x0c` word is
/// zero and whose type word at `+0x10` equals `type_id`, converts the home
/// words at `+0x16`/`+0x18` through the retail position lookup and asks the
/// retail target lookup whether the result is a unit.  `target_present`
/// stands in for that lookup pair and receives the candidate record.
pub fn w12a04_list_has_live_type(
    entries: &[W12A04Entry],
    head: Option<usize>,
    type_id: u16,
    target_present: impl Fn(&W12A04Entry) -> bool,
) -> bool {
    let mut cursor = head;
    while let Some(node) = cursor {
        let Some(entry) = entries.get(node) else {
            break;
        };
        if entry.owner.is_none() && entry.type_id == type_id && target_present(entry) {
            return true;
        }
        cursor = entry.next;
    }
    false
}

/// `0x00659770`: re-arm matching timers of one player's overlay list.
///
/// Retail visits unowned records with a non-zero `+0x1c` timer, maps the
/// type word `0x1e` to `5`, and sets the timer to `1` when the mapped type
/// equals the requested type.  Returns the number of records re-armed.
pub fn w12a04_reset_timers_for_type(
    entries: &mut [W12A04Entry],
    head: Option<usize>,
    type_id: u16,
) -> usize {
    let mut rearmed = 0usize;
    let mut cursor = head;
    while let Some(node) = cursor {
        let next = match entries.get(node) {
            Some(entry) => entry.next,
            None => break,
        };
        if let Some(entry) = entries.get_mut(node) {
            if entry.owner.is_none() && entry.timer != 0 {
                let mapped = if entry.type_id == 0x1e {
                    5
                } else {
                    entry.type_id
                };
                if mapped == type_id {
                    entry.timer = 1;
                    rearmed += 1;
                }
            }
        }
        cursor = next;
    }
    rearmed
}

/// `0x006597c0`: is any record's position within the recovered rectangle?
///
/// Retail walks the per-player list and calls the retail rectangle test as
/// `test(entry.x, first, entry.y, second)`, returning success for the first
/// result at or below `0x120` (the recovered arm or unit footprint).  The
/// test itself is not translated here, so it is a callback; the argument
/// order and the inclusive threshold are reproduced exactly.
pub fn w12a04_any_entry_within(
    entries: &[W12A04Entry],
    head: Option<usize>,
    first: i16,
    second: i16,
    mut range_test: impl FnMut(i16, i16, i16, i16) -> u32,
) -> bool {
    let mut cursor = head;
    while let Some(node) = cursor {
        let Some(entry) = entries.get(node) else {
            break;
        };
        if range_test(entry.x, first, entry.y, second) <= 0x120 {
            return true;
        }
        cursor = entry.next;
    }
    false
}

// ===========================================================================
// Overlay-record pool arithmetic
// ===========================================================================

/// `0x006594c0`: convert an overlay-pool pointer to its 1-based slot.
///
/// Retail returns zero for the null pointer and otherwise
/// `((pointer - 0x00fd7ba0) >> 5) + 1`.  The subtraction and shift are
/// 32-bit operations, so the wrapping form is kept for pointers that are
/// not pool members.
pub fn w12a04_pool_slot_index(pointer: u32) -> u32 {
    if pointer == 0 {
        0
    } else {
        (pointer.wrapping_sub(W12A04_POOL_BASE) >> 5).wrapping_add(1)
    }
}

// ===========================================================================
// Candidate scoring and game-mode remap
// ===========================================================================

/// `0x00659e20`: score one candidate slot.
///
/// The cached decompile for this row is a one-byte throw fragment; the
/// eight-record loop is recovered intact from the shipped image.
/// Retail walks the eight 0x24-byte slot records from slot 8 down to 1 and
/// counts the records whose `+0x0a` byte equals the candidate number and
/// whose `+0x08` state byte is `6` or `0x0c`.  The direction of the walk
/// does not affect the count; the provided slice should hold the recovered
/// eight records.
pub fn w12a04_slot_score(records: &[W12A04SlotRecord], slot: u8) -> u8 {
    records
        .iter()
        .filter(|record| record.slot == slot && (record.state == 6 || record.state == 0x0c))
        .count() as u8
}

/// `0x00659f20`: tie-break weight for one candidate slot.
///
/// Same eight-record walk as [`w12a04_slot_score`], counting records whose
/// `+0x0a` byte equals the candidate number and whose `+0x08` state byte is
/// `2`.  The candidate search keeps the *smaller* weight on a score tie.
pub fn w12a04_slot_tiebreak(records: &[W12A04SlotRecord], slot: u8) -> u8 {
    records
        .iter()
        .filter(|record| record.slot == slot && record.state == 2)
        .count() as u8
}

/// `0x00659a50`: pick the best candidate in `1 ..= limit`.
///
/// Retail computes the candidate cap as `2` in one client state and as the
/// current game-mode byte otherwise, then scans candidates in ascending
/// order.  A strictly larger score wins; on an equal score the candidate
/// with the smaller tie-break weight replaces the current best, and an
/// equal weight keeps the earlier candidate.  Candidate `0` is the "none"
/// result and is returned when the cap is below 1.
pub fn w12a04_best_candidate(
    limit: u8,
    score: impl Fn(u8) -> u8,
    tiebreak: impl Fn(u8) -> u8,
) -> u8 {
    let mut best = 0u8;
    let mut best_score = 0u8;
    let mut candidate = 1u16;
    while candidate <= u16::from(limit) {
        let current = candidate as u8;
        if score(current) == best_score && best != 0 && tiebreak(current) < tiebreak(best) {
            best = current;
            best_score = score(best);
        }
        if best_score < score(current) {
            best = current;
            best_score = score(best);
        }
        candidate += 1;
    }
    best
}

/// `0x00659bc0`: remap the client game mode for the option dialog.
///
/// Retail maps mode `2` to `4`, keeps `3`, maps `4` to `2`, and raises the
/// invalid-options error for every other value (the error text is
/// [`w12a04_invalid_options_message`]).  `None` is that error path.
pub fn w12a04_remap_game_mode(mode: u8) -> Option<u8> {
    match mode {
        2 => Some(4),
        3 => Some(3),
        4 => Some(2),
        _ => None,
    }
}

/// `0x00659c94`: the invalid-options error text.
///
/// The recovered slice copies `"strERROR_INVALID_OPTIONS"` into the error
/// object and stores its length as `0x18` (24) bytes; the literal below is
/// exactly that many bytes.
pub fn w12a04_invalid_options_message() -> &'static str {
    "strERROR_INVALID_OPTIONS"
}

#[cfg(test)]
mod tests_wave12_agent04 {
    use super::*;

    fn entry(next: Option<usize>, prev: Option<usize>, owner: Option<usize>) -> W12A04Entry {
        W12A04Entry {
            next,
            prev,
            owner,
            ..W12A04Entry::default()
        }
    }

    /// Three linked records: 0 <-> 1 <-> 2.
    fn three_node_list() -> (Vec<W12A04Entry>, Option<usize>, Option<usize>) {
        let entries = vec![
            entry(Some(1), None, Some(10)),
            entry(Some(2), Some(0), Some(11)),
            entry(None, Some(1), Some(12)),
        ];
        (entries, Some(0), None)
    }

    #[test]
    fn first_flagged_slot_ignores_slot_zero_and_short_selections() {
        assert_eq!(w12a04_first_flagged_selection_slot(&[]), None);
        assert_eq!(w12a04_first_flagged_selection_slot(&[0x10]), None);
        assert_eq!(
            w12a04_first_flagged_selection_slot(&[0x10, 0x00, 0x00]),
            None
        );
        assert_eq!(
            w12a04_first_flagged_selection_slot(&[0x00, 0x10, 0x10]),
            Some(1)
        );
        assert_eq!(
            w12a04_first_flagged_selection_slot(&[0x00, 0x00, 0x10]),
            Some(2)
        );
    }

    #[test]
    fn take_flagged_slot_clears_the_bit_and_advances() {
        let mut flags = [0x00, 0x10, 0x11, 0x00];
        assert_eq!(w12a04_take_flagged_selection_slot(&mut flags), Some(1));
        assert_eq!(flags, [0x00, 0x00, 0x11, 0x00]);
        assert_eq!(w12a04_take_flagged_selection_slot(&mut flags), Some(2));
        assert_eq!(flags, [0x00, 0x00, 0x01, 0x00]);
        assert_eq!(w12a04_take_flagged_selection_slot(&mut flags), None);
        assert_eq!(w12a04_take_flagged_selection_slot(&mut [0x10]), None);
    }

    #[test]
    fn overlay_target_gate_needs_the_status_bit_and_a_foreign_kind() {
        assert!(w12a04_overlay_target_permitted(false, 0x00));
        assert!(w12a04_overlay_target_permitted(true, 0x6a));
        assert!(w12a04_overlay_target_permitted(true, 0x96));
        assert!(!w12a04_overlay_target_permitted(true, 0x01));
    }

    #[test]
    fn kind_four_overlay_must_own_the_unit_and_clear_the_type_flag() {
        assert!(w12a04_overlay_owns_unit(Some(4), true, false));
        assert!(!w12a04_overlay_owns_unit(None, true, false));
        assert!(!w12a04_overlay_owns_unit(Some(4), false, false));
        assert!(!w12a04_overlay_owns_unit(Some(4), true, true));
        assert!(!w12a04_overlay_owns_unit(Some(1), true, false));
    }

    #[test]
    fn store_anchor_only_writes_kind_one_overlays() {
        let mut overlay = W12A04Overlay {
            kind: 1,
            x: 1,
            y: 2,
            ..W12A04Overlay::default()
        };
        assert!(w12a04_store_overlay_anchor(Some(&mut overlay), -7, 300));
        assert_eq!((overlay.x, overlay.y), (-7, 300));

        let mut other = W12A04Overlay {
            kind: 2,
            x: 1,
            y: 2,
            ..W12A04Overlay::default()
        };
        assert!(!w12a04_store_overlay_anchor(Some(&mut other), 9, 9));
        assert_eq!((other.x, other.y), (1, 2));
        assert!(!w12a04_store_overlay_anchor(None, 3, 4));
    }

    #[test]
    fn moving_a_middle_node_stitches_both_neighbours() {
        let (mut entries, mut head, mut free) = three_node_list();
        assert!(w12a04_list_move_to_free_head(
            &mut entries,
            &mut head,
            &mut free,
            1
        ));
        assert_eq!(head, Some(0));
        assert_eq!(free, Some(1));
        assert_eq!(entries[0].next, Some(2));
        assert_eq!(entries[2].prev, Some(0));
        assert_eq!(entries[1].prev, None);
        assert_eq!(entries[1].next, None);
    }

    #[test]
    fn moving_the_head_and_reusing_the_free_list_keeps_links_consistent() {
        let (mut entries, mut head, mut free) = three_node_list();
        assert!(w12a04_list_move_to_free_head(
            &mut entries,
            &mut head,
            &mut free,
            0
        ));
        assert_eq!(head, Some(1));
        assert_eq!(entries[1].prev, None);
        assert_eq!(free, Some(0));
        assert_eq!(entries[0].next, None);

        assert!(w12a04_list_move_to_free_head(
            &mut entries,
            &mut head,
            &mut free,
            2
        ));
        assert_eq!(head, Some(1));
        assert_eq!(free, Some(2));
        assert_eq!(entries[2].next, Some(0));
        assert_eq!(entries[0].prev, Some(2));
        assert!(!w12a04_list_move_to_free_head(
            &mut entries,
            &mut head,
            &mut free,
            99
        ));
    }

    #[test]
    fn sweep_reclaims_only_unowned_records() {
        let mut entries = vec![
            entry(Some(1), None, Some(10)),
            entry(Some(2), Some(0), None),
            entry(Some(3), Some(1), None),
            entry(None, Some(2), Some(13)),
        ];
        let mut head = Some(0);
        let mut free = None;

        assert_eq!(w12a04_sweep_unowned(&mut entries, &mut head, &mut free), 2);
        assert_eq!(head, Some(0));
        assert_eq!(entries[0].next, Some(3));
        assert_eq!(entries[3].prev, Some(0));
        assert_eq!(free, Some(2));
        assert_eq!(entries[2].next, Some(1));
        assert_eq!(entries[1].next, None);

        assert_eq!(w12a04_sweep_unowned(&mut entries, &mut head, &mut free), 0);
    }

    #[test]
    fn attach_copies_home_words_and_sets_the_owner() {
        let mut overlay = W12A04Overlay {
            kind: 4,
            x: 1,
            y: 2,
            home_x: -40,
            home_y: 75,
            ..W12A04Overlay::default()
        };
        assert_eq!(w12a04_attach_overlay(&mut overlay, 7), 7);
        assert_eq!((overlay.x, overlay.y), (-40, 75));
        assert_eq!(overlay.owner, Some(7));
    }

    #[test]
    fn consume_target_clears_owner_and_saturates_the_counter() {
        let mut overlay = W12A04Overlay {
            kind: 1,
            counter: 99,
            owner: Some(3),
            ..W12A04Overlay::default()
        };
        assert!(w12a04_consume_overlay_target(Some(&mut overlay)));
        assert_eq!(overlay.owner, None);
        assert_eq!(overlay.counter, 100);

        assert!(w12a04_consume_overlay_target(Some(&mut overlay)));
        assert_eq!(overlay.counter, 100);

        let mut other = W12A04Overlay {
            kind: 2,
            counter: 1,
            ..W12A04Overlay::default()
        };
        assert!(!w12a04_consume_overlay_target(Some(&mut other)));
        assert_eq!(other.counter, 1);
        assert!(!w12a04_consume_overlay_target(None));
    }

    #[test]
    fn live_type_scan_requires_ownerless_type_match_and_a_target() {
        let mut entries = vec![W12A04Entry::default(), W12A04Entry::default()];
        entries[0].next = Some(1);
        entries[0].type_id = 0x20;
        entries[0].owner = Some(5);
        entries[1].type_id = 0x21;
        let head = Some(0);

        assert!(!w12a04_list_has_live_type(&entries, head, 0x20, |_| true));
        assert!(!w12a04_list_has_live_type(&entries, head, 0x22, |_| true));
        assert!(!w12a04_list_has_live_type(&entries, head, 0x21, |_| false));

        entries[1].type_id = 0x20;
        assert!(w12a04_list_has_live_type(&entries, head, 0x20, |entry| {
            entry.x == 0 && entry.y == 0 && entry.home_x == 0 && entry.home_y == 0
        }));
        assert!(!w12a04_list_has_live_type(&entries, None, 0x20, |_| true));
    }

    #[test]
    fn reset_timers_normalises_type_thirty_and_skips_owned_records() {
        let mut entries = vec![
            W12A04Entry {
                next: Some(1),
                type_id: 0x1e,
                timer: 40,
                ..W12A04Entry::default()
            },
            W12A04Entry {
                next: Some(2),
                type_id: 5,
                timer: 40,
                owner: Some(1),
                ..W12A04Entry::default()
            },
            W12A04Entry {
                next: Some(3),
                type_id: 5,
                timer: 40,
                ..W12A04Entry::default()
            },
            W12A04Entry {
                type_id: 5,
                timer: 0,
                ..W12A04Entry::default()
            },
            W12A04Entry {
                type_id: 6,
                timer: 40,
                ..W12A04Entry::default()
            },
        ];

        assert_eq!(w12a04_reset_timers_for_type(&mut entries, Some(0), 5), 2);
        assert_eq!(entries[0].timer, 1);
        assert_eq!(entries[1].timer, 40);
        assert_eq!(entries[2].timer, 1);
        assert_eq!(entries[3].timer, 0);
        assert_eq!(entries[4].timer, 40);
    }

    #[test]
    fn range_scan_uses_the_retail_argument_order_and_inclusive_limit() {
        let entries = vec![
            W12A04Entry {
                next: Some(1),
                x: 10,
                y: 20,
                ..W12A04Entry::default()
            },
            W12A04Entry {
                x: 30,
                y: 40,
                ..W12A04Entry::default()
            },
        ];

        let mut seen = Vec::new();
        let hit = w12a04_any_entry_within(&entries, Some(0), 50, 60, |x1, y1, x2, y2| {
            seen.push((x1, y1, x2, y2));
            if x1 == 10 { 0x120 } else { 0x121 }
        });
        assert!(hit);
        assert_eq!(seen, vec![(10, 50, 20, 60)]);

        let hit = w12a04_any_entry_within(&entries, Some(0), 0, 0, |_, _, _, _| 0x121);
        assert!(!hit);

        let hit = w12a04_any_entry_within(&entries, Some(0), 0, 0, |x1, _, _, _| {
            if x1 == 30 { 0x120 } else { 0x121 }
        });
        assert!(hit);

        assert!(!w12a04_any_entry_within(
            &entries,
            None,
            0,
            0,
            |_, _, _, _| 0
        ));
    }

    #[test]
    fn best_candidate_prefers_score_then_smaller_tiebreak_then_earlier() {
        let scores = [0u8, 3, 3, 5, 5, 1];
        let breaks = [0u8, 9, 4, 8, 2, 0];
        let score = |i: u8| scores[i as usize];
        let tie = |i: u8| breaks[i as usize];

        // Candidate 5 owns the top score but loses the tie on weight to 4.
        assert_eq!(w12a04_best_candidate(5, score, tie), 4);

        // Equal weights keep the earlier candidate.
        let flat = |_: u8| 1u8;
        assert_eq!(w12a04_best_candidate(5, score, flat), 3);
        assert_eq!(w12a04_best_candidate(0, score, tie), 0);
    }

    #[test]
    fn candidate_scores_count_the_recovered_slot_states() {
        let mut records = [W12A04SlotRecord::default(); 8];
        records[0] = W12A04SlotRecord { state: 6, slot: 1 };
        records[1] = W12A04SlotRecord {
            state: 0x0c,
            slot: 1,
        };
        records[2] = W12A04SlotRecord { state: 2, slot: 1 };
        records[3] = W12A04SlotRecord { state: 6, slot: 2 };
        records[4] = W12A04SlotRecord {
            state: 0x0c,
            slot: 2,
        };
        records[5] = W12A04SlotRecord { state: 2, slot: 2 };
        records[6] = W12A04SlotRecord { state: 2, slot: 2 };
        records[7] = W12A04SlotRecord { state: 6, slot: 9 };

        assert_eq!(w12a04_slot_score(&records, 1), 2);
        assert_eq!(w12a04_slot_score(&records, 2), 2);
        assert_eq!(w12a04_slot_score(&records, 9), 1);
        assert_eq!(w12a04_slot_tiebreak(&records, 1), 1);
        assert_eq!(w12a04_slot_tiebreak(&records, 2), 2);
        assert_eq!(w12a04_slot_tiebreak(&records, 3), 0);
    }

    #[test]
    fn game_mode_remap_swaps_two_and_four() {
        assert_eq!(w12a04_remap_game_mode(2), Some(4));
        assert_eq!(w12a04_remap_game_mode(3), Some(3));
        assert_eq!(w12a04_remap_game_mode(4), Some(2));
        assert_eq!(w12a04_remap_game_mode(0), None);
        assert_eq!(w12a04_remap_game_mode(5), None);
    }

    #[test]
    fn invalid_options_message_matches_the_recovered_literal_length() {
        let message = w12a04_invalid_options_message();
        assert_eq!(message, "strERROR_INVALID_OPTIONS");
        assert_eq!(message.len(), 0x18);
    }

    #[test]
    fn pool_slot_index_maps_the_retail_pool_base() {
        assert_eq!(w12a04_pool_slot_index(0), 0);
        assert_eq!(w12a04_pool_slot_index(W12A04_POOL_BASE), 1);
        assert_eq!(
            w12a04_pool_slot_index(W12A04_POOL_BASE + W12A04_POOL_STRIDE),
            2
        );
        assert_eq!(w12a04_pool_slot_index(W12A04_POOL_BASE + 0x1e0), 16);
        // The low five bits are dropped by the shift, matching retail.
        assert_eq!(w12a04_pool_slot_index(W12A04_POOL_BASE + 0x21), 2);
    }
}
