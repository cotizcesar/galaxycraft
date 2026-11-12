#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave12/agent03.rs
// ===========================================================================
// Wave 12 translation batch 03.
//
// Client/UI runtime helpers recovered from `0x00656000..=0x00657fff`.
// Rows that walked engine-global tables are reduced to the pure arithmetic
// and predicates they applied to caller-owned values; the global table
// plumbing stays in the engine and is not duplicated here.

/// Stride of one per-owner slot record, used by the rows in this batch that
/// index a slot table row.
pub const W12A03_SLOT_RECORD_STRIDE: usize = 0x34;

/// Exclusive sentinel of the per-frame unit scan cursor (`0x9c4`, 2500).
pub const W12A03_SCAN_SLOT_LIMIT: u16 = 0x9c4;

/// Number of per-owner rows the scan cursor cycles through.
pub const W12A03_SCAN_OWNER_ROWS: u8 = 8;

/// Slots advanced per call before the scan cursor returns to its caller
/// (`0x14d`, 333).
pub const W12A03_SCAN_STEP_BUDGET: u32 = 0x14d;

/// Result of [`w12a03_arm_slot`] for a slot whose state byte is zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W12a03ArmOutcome {
    /// State byte at `+5` after the row initialises the slot (always `4`).
    pub state: u8,
    /// Flags byte at `+8` after the row's `|=` steps.
    pub flags: u8,
    /// `u16` timer at `+0xc` when the row overwrites it.
    pub timer: Option<u16>,
}

/// `0x00656540` (`FUN_00656540`, 100 bytes): arm a freshly selected slot.
///
/// The row returns immediately unless the state byte at `+5` is zero.
/// Otherwise it runs the shared slot constructor with state `4`, sets bit 0
/// of the flags byte at `+8`, and then:
///
/// * when the explicit flag is set **or** the owner passes the row's
///   player-flag test, it also sets bit 6 (`0x40`);
/// * when the explicit flag is set **and** the slot is not the row's special
///   unit type, it writes `1000` to the `u16` timer at `+0xc`.
///
/// `player_flag` is the result of the player-flag test and `special_type`
/// the result of the special unit-type test; both are engine queries outside
/// this batch. The constructor's counter reinitialisation is likewise engine
/// state, so only the state byte it receives is reported here.
pub fn w12a03_arm_slot(
    state: u8,
    flags: u8,
    explicit: bool,
    player_flag: bool,
    special_type: bool,
) -> Option<W12a03ArmOutcome> {
    if state != 0 {
        return None;
    }
    let mut flags = flags | 0x01;
    let mut timer = None;
    if explicit || player_flag {
        flags |= 0x40;
        if explicit && !special_type {
            timer = Some(1000);
        }
    }
    Some(W12a03ArmOutcome {
        state: 4,
        flags,
        timer,
    })
}

/// `0x00656730` (`FUN_00656730`, 129 bytes): consume a list while every item
/// satisfies a predicate.
///
/// The row fetches the first item, reports `0` when the list is empty, and
/// otherwise consumes items while the predicate keeps returning nonzero. The
/// pure shape is reported here: `true` means the list was exhausted, while
/// `false` means an item failed the predicate (the row then takes its abort
/// path).
pub fn w12a03_drain_matching<T>(
    items: impl IntoIterator<Item = T>,
    mut matches: impl FnMut(&T) -> bool,
) -> bool {
    for item in items {
        if !matches(&item) {
            return false;
        }
    }
    true
}

/// `0x006568a5` (`FUN_006568a5`, 47 bytes): first-candidate filter used by
/// the slot scans.
///
/// A candidate kind byte at `+5` is accepted when it is `0`, `5`, or the
/// wanted kind supplied by the caller. On a match the row returns the
/// caller's fallback value; otherwise it walks on to the next candidate.
pub fn w12a03_scan_kind_matches(kind: u8, wanted: u8) -> bool {
    kind == 0 || kind == 5 || kind == wanted
}

/// `0x00656923` (`FUN_00656923`, 65 bytes): the fixed-`4` twin of the
/// first-candidate filter.
///
/// The row pins the wanted kind at `4`, so a kind byte at `+5` is accepted
/// when it is `0`, `5`, or `4`.
pub fn w12a03_scan_kind_matches_state4(kind: u8) -> bool {
    matches!(kind, 0 | 4 | 5)
}

/// `0x00656b00` (`FUN_00656b00`, 61 bytes): test whether a container's
/// current payload kind is accepted.
///
/// The row first requires the sprite/container pointer to be present and its
/// state byte at `+8` to be `4`. It then reads the carried slot's kind byte
/// at `+5` and accepts:
///
/// * `1` and `2` unconditionally;
/// * `8` and `9` only when the caller's second argument is zero.
///
/// `container_state` is the byte at `+8`; `passive_allowed` is `true` when
/// the second argument is zero.
pub fn w12a03_container_accepts_kind(container_state: u8, kind: u8, passive_allowed: bool) -> bool {
    container_state == 4
        && (kind == 1 || (passive_allowed && (kind == 8 || kind == 9)) || kind == 2)
}

/// `0x00656b40` (`FUN_00656b40`, 23 bytes): scan the `+0x3c` flag bytes of
/// one owner row.
///
/// The count comes from the engine's slot count. Starting at position `1`,
/// the row inspects the flag byte of records `0..count-2` and breaks on the
/// first `byte & 2`; it reports whether the position reached the count.
/// Consequently:
///
/// * an empty row reports `false`;
/// * a one-record row reports `true` without reading the record;
/// * a flagged byte before the last record reports `false`;
/// * a flag only on the last record is never inspected and reports `true`.
///
/// `flags` holds the `+0x3c` byte of each record in row order.
pub fn w12a03_slot_flag_scan(flags: &[u8]) -> bool {
    let count = flags.len();
    let mut position = 1usize;
    if count > 1 {
        for flag in &flags[..count - 1] {
            if flag & 0x02 != 0 {
                break;
            }
            position += 1;
        }
    }
    position == count
}

/// `0x00656e4b` (`FUN_00656e4b`, 161 bytes): drain the two `u16` counters at
/// `+0xe` and `+0xc` of one slot.
///
/// The row reduces each counter with saturating subtraction:
///
/// ```c
/// uVar2 = *(ushort *)(slot + 0xe);
/// if (uVar5 < uVar2) *(ushort *)(slot + 0xe) = uVar2 - (short)uVar5;
/// else *(ushort *)(slot + 0xe) = 0;
/// /* then the same with +0xc and the second amount */
/// ```
///
/// The two amounts come from external lookups. The returned pair is
/// `(counter at +0xe, counter at +0xc)`.
pub fn w12a03_drain_counters(
    field_e: u16,
    field_c: u16,
    amount_e: u16,
    amount_c: u16,
) -> (u16, u16) {
    (
        field_e.saturating_sub(amount_e),
        field_c.saturating_sub(amount_c),
    )
}

/// `0x00656fb0` (`FUN_00656fb0`, 38 bytes): address of a slot record.
///
/// The row selects the owner row with the byte at `+0x4c` of the unit and
/// combines it with the slot index:
///
/// ```c
/// return (slot & 0xffff) * 0x34 + row_base;
/// ```
///
/// `row_base` is the base address of the selected owner's slot array (read
/// from the engine's owner table) and `slot` is the zero-extended unit slot
/// index returned by the engine's handle lookup.
pub fn w12a03_slot_record_offset(row_base: usize, slot: u16) -> usize {
    row_base + usize::from(slot) * W12A03_SLOT_RECORD_STRIDE
}

/// `0x00657105` (`FUN_00657105`, 65 bytes): range guard for a recovered
/// character code.
///
/// ```c
/// uVar1 = ...;
/// if (0x80 < uVar1) { ... }
/// return 1;
/// ```
///
/// The row traps when the value exceeds `0x80`; `true` means the value is in
/// the accepted `0..=0x80` range.
pub fn w12a03_char_code_in_range(code: u32) -> bool {
    code <= 0x80
}

/// `0x00657730` (`FUN_00657730`, 14 bytes): advance the throttled per-frame
/// unit scan cursor.
///
/// Each call advances the slot cursor by at most [`W12A03_SCAN_STEP_BUDGET`]
/// steps. When the cursor reaches [`W12A03_SCAN_SLOT_LIMIT`] the row adds one
/// to the owner index, resets the slot cursor to `1`, and stops; the owner
/// index is an `0..7` ring, so the next value after `7` is `0`:
///
/// ```text
/// repeat up to 0x14d times:
///     slot += 1
///     if slot == 0x9c4:
///         owner += 1
///         if owner == 8: owner = 0
///         slot = 1
///         stop
/// ```
///
/// The returned flag is `true` when the owner ring wrapped back to `0`; the
/// row uses that wrap to clear its per-slot caches for every owner row. A
/// budget of zero leaves the cursor untouched.
///
/// The row's pre-step bounds comparison only feeds a player-flag predicate
/// and does not alter the cursor. `owner` is the recovered `0..7` ring index.
pub fn w12a03_advance_unit_scan(owner: u8, slot: u16, budget: u32) -> (u8, u16, bool) {
    let mut owner = owner;
    let mut slot = slot;
    for _ in 0..budget {
        slot = slot.wrapping_add(1);
        if slot == W12A03_SCAN_SLOT_LIMIT {
            owner = owner.wrapping_add(1);
            if owner == W12A03_SCAN_OWNER_ROWS {
                owner = 0;
            }
            return (owner, 1, owner == 0);
        }
    }
    (owner, slot, false)
}

/// `0x00657888` (`FUN_00657888`, 236 bytes): add an amount to one of the two
/// `i16` counters of a slot.
///
/// ```c
/// if (flag == 0) { *(short *)(slot + 0x1a) += amount; }
/// else           { *(short *)(slot + 0x18) += amount; }
/// ```
///
/// `flag` is a unit flag read by the row and `amount` is the caller's
/// `short`. The pair returned is `(counter at +0x18, counter at +0x1a)`,
/// using two's-complement wrap as the `short` addition does.
pub fn w12a03_accumulate_slot_value(
    primary: i16,
    secondary: i16,
    amount: i16,
    use_primary: bool,
) -> (i16, i16) {
    if use_primary {
        (primary.wrapping_add(amount), secondary)
    } else {
        (primary, secondary.wrapping_add(amount))
    }
}

/// `0x00657a50` (`FUN_00657a50`, 209 bytes): centre of a selection box.
///
/// The row copies the four `i32` box edges and draws a `0x98` sprite at
///
/// ```c
/// x = (right - left) / 2 + left;
/// y = (bottom - top) / 2 + top;
/// ```
///
/// C `int` division truncates toward zero, as Rust `i32` division does.
/// `wrapping_*` keeps the recovered 32-bit arithmetic defined at the edges of
/// the coordinate domain.
pub fn w12a03_selection_center(min: i32, max: i32) -> i32 {
    max.wrapping_sub(min) / 2 + min
}

/// `0x00657a50` (`FUN_00657a50`, 209 bytes), key extraction: pack the four
/// even-indexed words.
///
/// The row builds its eight-byte lookup key from every other `u16` of the
/// caller's sixteen-byte structure:
///
/// ```c
/// local[0] = words[0];
/// local[1] = words[2];
/// local[2] = words[4];
/// local[3] = words[6];
/// ```
pub fn w12a03_selection_box_words(words: &[u16; 8]) -> [u16; 4] {
    [words[0], words[2], words[4], words[6]]
}

/// One node of the attachment chain walked by `0x00657bcb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W12a03Attachment {
    /// Whether the node's linked slot passes the row's liveness test.
    pub live: bool,
    /// Kind byte at `+0x4d` of the linked slot.
    pub kind: u8,
    /// Slot index of the unit referenced at `+0x24` of the linked slot.
    pub source_slot: u16,
}

/// Cursor decision at the end of `0x00657bcb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W12a03CursorAction {
    /// The chosen slot is the unit's own slot: centre the camera.
    Center,
    /// The chosen slot differs: move the cursor to it.
    MoveTo(u16),
}

/// `0x00657bcb` (`FUN_00657bcb`, 45 bytes): cursor target chosen from the
/// attachment chain.
///
/// The row keeps the unit's current slot index as its default, then walks the
/// linked attachment list rooted at the unit. Each node carries a linked slot
/// at `+0xc`; the first node whose linked slot fails the row's liveness test,
/// or whose kind byte at `+0x4d` is `0x6a` (`'j'`) or `0x96`, supplies the
/// replacement index (the slot index of the unit referenced at `+0x24` of
/// the linked slot). If the walk ends without a candidate, the default
/// remains.
///
/// The row then compares the chosen index with the unit's own slot: equal
/// means the camera centres on the unit, otherwise the cursor moves to the
/// chosen slot.
pub fn w12a03_attachment_cursor(
    nodes: &[W12a03Attachment],
    current_slot: u16,
    default_slot: u16,
) -> W12a03CursorAction {
    let target = nodes
        .iter()
        .find(|node| !node.live || node.kind == 0x6a || node.kind == 0x96)
        .map(|node| node.source_slot)
        .unwrap_or(default_slot);
    if target == current_slot {
        W12a03CursorAction::Center
    } else {
        W12a03CursorAction::MoveTo(target)
    }
}

/// `0x00657dd0` (`FUN_00657dd0`, 337 bytes): carried-kind validation after
/// the rollback path.
///
/// After detaching, the row re-reads the current sprite and requires its
/// state byte at `+8` to still be `4` and the carried slot's kind byte at
/// `+5` to be `1`, `2`, `8` or `9`; anything else takes the row's abort
/// path. This is that membership test.
pub fn w12a03_carried_kind_set(kind: u8) -> bool {
    matches!(kind, 1 | 2 | 8 | 9)
}

/// `0x00657fd0` (`FUN_00657fd0`, 32 bytes): mutual-exclusion guard for two
/// selection sources.
///
/// ```c
/// if ((param_2 != 0) && (param_1 != 0)) { ... }
/// return 0;
/// ```
///
/// The row's only failure is both arguments being present; `true` means the
/// pair is legal.
pub fn w12a03_exclusive_selection_ok(first: bool, second: bool) -> bool {
    !(first && second)
}

/// `0x00657fea` (`FUN_00657fea`, 63 bytes): asset hotkey gate.
///
/// The row returns unless the key code is `0x49` (`'I'`). It then fetches an
/// object from the key's queue and requires either of two lookups to yield a
/// target before it re-reads the queue and acts. `has_queue`, `has_target`
/// and `has_alternate` model those three nonzero results; `true` means the
/// row proceeds.
pub fn w12a03_asset_hotkey_accepted(
    key: u16,
    has_queue: bool,
    has_target: bool,
    has_alternate: bool,
) -> bool {
    key == 0x49 && has_queue && (has_target || has_alternate)
}

#[cfg(test)]
mod tests_wave12_agent03 {
    use super::*;

    #[test]
    fn arm_slot_ignores_nonzero_state() {
        assert_eq!(w12a03_arm_slot(1, 0, true, true, false), None);
        assert_eq!(w12a03_arm_slot(7, 0xff, true, true, false), None);
    }

    #[test]
    fn arm_slot_without_explicit_or_player_flag() {
        assert_eq!(
            w12a03_arm_slot(0, 0x80, false, false, false),
            Some(W12a03ArmOutcome {
                state: 4,
                flags: 0x81,
                timer: None,
            })
        );
    }

    #[test]
    fn arm_slot_sets_selection_bit_for_explicit_choice() {
        assert_eq!(
            w12a03_arm_slot(0, 0x02, true, false, false),
            Some(W12a03ArmOutcome {
                state: 4,
                flags: 0x43,
                timer: Some(1000),
            })
        );
    }

    #[test]
    fn arm_slot_special_type_skips_timer() {
        assert_eq!(
            w12a03_arm_slot(0, 0, true, false, true),
            Some(W12a03ArmOutcome {
                state: 4,
                flags: 0x41,
                timer: None,
            })
        );
        assert_eq!(
            w12a03_arm_slot(0, 0, false, true, true),
            Some(W12a03ArmOutcome {
                state: 4,
                flags: 0x41,
                timer: None,
            })
        );
    }

    #[test]
    fn drain_matching_reports_exhaustion_and_failure() {
        let values = [2u8, 4, 6];
        assert!(w12a03_drain_matching(values, |value| *value % 2 == 0));
        assert!(!w12a03_drain_matching(values, |value| *value < 5));
        let empty: [u8; 0] = [];
        assert!(w12a03_drain_matching(empty, |_| false));
    }

    #[test]
    fn drain_matching_stops_at_first_failure() {
        let mut seen = 0u32;
        let values = [1u8, 2, 3];
        let matched = w12a03_drain_matching(values, |value| {
            seen += 1;
            *value != 2
        });
        assert!(!matched);
        assert_eq!(seen, 2);
    }

    #[test]
    fn scan_kind_matches_accepts_open_zero_five_and_wanted() {
        for kind in 0u8..=10 {
            assert_eq!(w12a03_scan_kind_matches(kind, 7), matches!(kind, 0 | 5 | 7));
        }
    }

    #[test]
    fn scan_kind_matches_state4_is_the_fixed_wanted_case() {
        for kind in 0u8..=10 {
            assert_eq!(
                w12a03_scan_kind_matches_state4(kind),
                matches!(kind, 0 | 4 | 5)
            );
        }
    }

    #[test]
    fn container_accepts_kind_base_states() {
        assert!(w12a03_container_accepts_kind(4, 1, false));
        assert!(w12a03_container_accepts_kind(4, 2, false));
        assert!(!w12a03_container_accepts_kind(4, 0, true));
        assert!(!w12a03_container_accepts_kind(4, 3, true));
        assert!(!w12a03_container_accepts_kind(3, 1, true));
        assert!(!w12a03_container_accepts_kind(5, 2, true));
    }

    #[test]
    fn container_accepts_kind_passive_gate() {
        assert!(!w12a03_container_accepts_kind(4, 8, false));
        assert!(!w12a03_container_accepts_kind(4, 9, false));
        assert!(w12a03_container_accepts_kind(4, 8, true));
        assert!(w12a03_container_accepts_kind(4, 9, true));
        assert!(!w12a03_container_accepts_kind(4, 10, true));
    }

    #[test]
    fn slot_flag_scan_short_rows() {
        assert!(!w12a03_slot_flag_scan(&[]));
        assert!(w12a03_slot_flag_scan(&[0x00]));
        // A one-record row is never inspected.
        assert!(w12a03_slot_flag_scan(&[0x02]));
    }

    #[test]
    fn slot_flag_scan_ignores_last_record() {
        assert!(w12a03_slot_flag_scan(&[0x00, 0x02]));
        assert!(w12a03_slot_flag_scan(&[0x00, 0x00, 0x02]));
    }

    #[test]
    fn slot_flag_scan_detects_early_flag() {
        assert!(!w12a03_slot_flag_scan(&[0x02, 0x00]));
        assert!(!w12a03_slot_flag_scan(&[0x00, 0x02, 0x00]));
        assert!(!w12a03_slot_flag_scan(&[0xfe, 0x00, 0x00]));
        // Other bits do not stop the scan.
        assert!(w12a03_slot_flag_scan(&[0xfd & !0x02, 0x00]));
    }

    #[test]
    fn drain_counters_saturates_independently() {
        assert_eq!(w12a03_drain_counters(10, 20, 4, 30), (6, 0));
        assert_eq!(w12a03_drain_counters(0, 0, 1, 1), (0, 0));
        assert_eq!(w12a03_drain_counters(5, 5, 5, 5), (0, 0));
        assert_eq!(w12a03_drain_counters(u16::MAX, 1, 1, 0), (u16::MAX - 1, 1));
    }

    #[test]
    fn slot_record_offset_steps_by_stride() {
        assert_eq!(w12a03_slot_record_offset(0x1000, 0), 0x1000);
        assert_eq!(w12a03_slot_record_offset(0x1000, 1), 0x1000 + 0x34);
        assert_eq!(w12a03_slot_record_offset(0x2000, 3), 0x2000 + 3 * 0x34);
        assert_eq!(
            w12a03_slot_record_offset(0, u16::MAX),
            usize::from(u16::MAX) * W12A03_SLOT_RECORD_STRIDE
        );
    }

    #[test]
    fn char_code_range_accepts_through_0x80() {
        for code in [0, 0x41, 0x7f, 0x80] {
            assert!(w12a03_char_code_in_range(code));
        }
        for code in [0x81, 0x100, u32::MAX] {
            assert!(!w12a03_char_code_in_range(code));
        }
    }

    #[test]
    fn advance_scan_steps_with_budget() {
        assert_eq!(w12a03_advance_unit_scan(0, 10, 3), (0, 13, false));
        assert_eq!(w12a03_advance_unit_scan(0, 0, 0), (0, 0, false));
        assert_eq!(
            w12a03_advance_unit_scan(0, 0, W12A03_SCAN_STEP_BUDGET),
            (0, W12A03_SCAN_STEP_BUDGET as u16, false)
        );
    }

    #[test]
    fn advance_scan_wraps_slot_to_next_owner() {
        assert_eq!(
            w12a03_advance_unit_scan(0, W12A03_SCAN_SLOT_LIMIT - 1, 1),
            (1, 1, false)
        );
        assert_eq!(
            w12a03_advance_unit_scan(3, W12A03_SCAN_SLOT_LIMIT - 2, 2),
            (4, 1, false)
        );
    }

    #[test]
    fn advance_scan_full_cycle_flags_reset() {
        assert_eq!(
            w12a03_advance_unit_scan(7, W12A03_SCAN_SLOT_LIMIT - 1, 1),
            (0, 1, true)
        );
        // Advancing from another owner does not report the reset.
        assert_eq!(
            w12a03_advance_unit_scan(1, W12A03_SCAN_SLOT_LIMIT - 1, 1),
            (2, 1, false)
        );
    }

    #[test]
    fn accumulate_slot_value_branches() {
        assert_eq!(w12a03_accumulate_slot_value(10, 20, 5, true), (15, 20));
        assert_eq!(w12a03_accumulate_slot_value(10, 20, 5, false), (10, 25));
        assert_eq!(w12a03_accumulate_slot_value(0, 0, -7, false), (0, -7));
    }

    #[test]
    fn accumulate_slot_value_wraps() {
        assert_eq!(
            w12a03_accumulate_slot_value(i16::MAX, 0, 1, true),
            (i16::MIN, 0)
        );
        assert_eq!(
            w12a03_accumulate_slot_value(0, i16::MIN, -1, false),
            (0, i16::MAX)
        );
    }

    #[test]
    fn selection_center_truncates_toward_zero() {
        assert_eq!(w12a03_selection_center(10, 20), 15);
        assert_eq!(w12a03_selection_center(10, 25), 17);
        assert_eq!(w12a03_selection_center(-10, 10), 0);
        assert_eq!(w12a03_selection_center(-10, -7), -9);
        assert_eq!(w12a03_selection_center(5, 5), 5);
    }

    #[test]
    fn selection_box_words_take_even_indices() {
        let words = [1u16, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(w12a03_selection_box_words(&words), [1, 3, 5, 7]);
    }

    #[test]
    fn attachment_cursor_picks_first_failing_node() {
        let nodes = [
            W12a03Attachment {
                live: true,
                kind: 0,
                source_slot: 11,
            },
            W12a03Attachment {
                live: false,
                kind: 0,
                source_slot: 22,
            },
            W12a03Attachment {
                live: true,
                kind: 0x6a,
                source_slot: 33,
            },
        ];
        assert_eq!(
            w12a03_attachment_cursor(&nodes, 11, 99),
            W12a03CursorAction::MoveTo(22)
        );
        assert_eq!(
            w12a03_attachment_cursor(&nodes, 22, 99),
            W12a03CursorAction::Center
        );
    }

    #[test]
    fn attachment_cursor_accepts_special_kinds_and_default() {
        let nodes = [
            W12a03Attachment {
                live: true,
                kind: 0x01,
                source_slot: 11,
            },
            W12a03Attachment {
                live: true,
                kind: 0x96,
                source_slot: 44,
            },
        ];
        assert_eq!(
            w12a03_attachment_cursor(&nodes, 11, 99),
            W12a03CursorAction::MoveTo(44)
        );
        let no_match = [W12a03Attachment {
            live: true,
            kind: 0x01,
            source_slot: 11,
        }];
        assert_eq!(
            w12a03_attachment_cursor(&no_match, 11, 99),
            W12a03CursorAction::MoveTo(99)
        );
        assert_eq!(
            w12a03_attachment_cursor(&[], 11, 11),
            W12a03CursorAction::Center
        );
        assert_eq!(
            w12a03_attachment_cursor(&[], 11, 99),
            W12a03CursorAction::MoveTo(99)
        );
    }

    #[test]
    fn carried_kind_set_is_1_2_8_9() {
        for kind in 0u8..=10 {
            assert_eq!(w12a03_carried_kind_set(kind), matches!(kind, 1 | 2 | 8 | 9));
        }
        assert!(!w12a03_carried_kind_set(0xff));
    }

    #[test]
    fn exclusive_selection_allows_only_one_source() {
        assert!(w12a03_exclusive_selection_ok(false, false));
        assert!(w12a03_exclusive_selection_ok(true, false));
        assert!(w12a03_exclusive_selection_ok(false, true));
        assert!(!w12a03_exclusive_selection_ok(true, true));
    }

    #[test]
    fn asset_hotkey_requires_i_and_a_target() {
        assert!(w12a03_asset_hotkey_accepted(0x49, true, true, false));
        assert!(w12a03_asset_hotkey_accepted(0x49, true, false, true));
        assert!(!w12a03_asset_hotkey_accepted(0x48, true, true, true));
        assert!(!w12a03_asset_hotkey_accepted(0x49, false, true, true));
        assert!(!w12a03_asset_hotkey_accepted(0x49, true, false, false));
    }
}
