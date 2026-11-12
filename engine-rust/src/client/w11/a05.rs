#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave11/agent05.rs
// ===========================================================================
// Wave 11 translation batch 05: client/runtime helpers in
// `0x00650000..=0x00651fff`.
//
// The ledger lists 48 rows in this window: 47 `FUN_*` bodies plus the
// one-byte `__break` marker at `FUN_00651b6e`, all marked
// `decompiled_uncited`.  Twelve rows are translated below; the remaining
// rows are listed under [Skipped rows](#skipped-rows) with the reason each
// one is not modelled.
//
// The block is client/UI/runtime code: command-card slot tables, the
// per-owner unit-row arrays, the per-player reachability grid, target-list
// flags, selection bounds and the unit display-state setter.  Nothing here
// touches [`crate::sim::World`] or [`crate::sim::SimUnit`], so every
// translation is a free function over values or slices.  Calls whose target
// leaves the window enter as caller-supplied values, slices or closures; no
// helper outside this file is assumed to exist.
//
// ## Recovered layouts
//
// | location | layout |
// |----------|--------|
// | `0x00fd2d0c` | eight `dword` unit-row base pointers (`0x20` bytes), released by `0x00651410` |
// | `0x00fd2d38` | per-player reachability byte grid, `0x9c4` cells per player (`0x006516d0`) |
// | unit row | `0x34` bytes; `+0x1c`/`+0x20` select the reachability class (`0x006516d0`) |
// | target-list node | `+0x00` next, `+0x0c` unit id (`0x00651450`, `0x00651930`) |
// | display record | `+0` id, `+2` target, `+4` owner byte, `+5` state, `+6` anim, `+8` flags, `+9` substate, `+12..+42` extent words (`0x006519f0`) |
// | selection bounds | `+0` x, `+4` owner byte, `+0x10..+0x16` four extents (`0x00650600`) |
//
// ## Skipped rows
//
// * `FUN_00650093` - array copy whose source and destination pointers live in
//   `unaff_EBP` slots and whose final dispatch is the non-returning
//   `FUN_005b4db0`; no routine-level arguments survive.
// * `FUN_006501cd` - SEH destructor tail storing through `unaff_EBX` /
//   `unaff_EDI` and ending in the non-returning `FUN_005b4db0`.
// * `FUN_00650227`, `FUN_0065043d` - catch funclets: restore `ExceptionList`
//   and return 0/void; no arithmetic to translate.
// * `FUN_00650240` - stack-cookie/SEH frame plus a four-byte helper call whose
//   error path is non-returning; no stable signature.
// * `FUN_00650293`, `FUN_006502ac` - null/list-clear guards whose alternative
//   path is a non-returning runtime call; the only result is the constant 0.
// * `FUN_006502f9`, `FUN_00650350` - record constructors addressed through
//   `unaff_EBX`/`unaff_EDI` and absolute literals; the object base is not a
//   recovered argument.
// * `FUN_00650660` - allocation plus `memset` of `count * 0x34` bytes; only
//   allocator behavior, no value semantics.
// * `FUN_006506a0`, `FUN_00650773` - nearest-candidate scan: the `count == 1`
//   fast path returns the first candidate while the general path returns the
//   loop index, and the point metric plus modifier helpers are opaque, so
//   the return contract is inconsistent.
// * `FUN_00650952`, `FUN_00650cc0`, `FUN_00650d55`, `FUN_00650f2c` - command-slot
//   edits whose array pointer and count live in `unaff_EBX`/`unaff_ESI` and
//   whose outer walk uses `FUN_005b4bb0`, `FUN_005a3180` and
//   `FUN_005b6370`; only the inner "replace id with `0xe4`" loop is
//   self-contained.
// * `FUN_006509f0` - both branches call the same helper and then the
//   non-returning `FUN_0058b650`; no observable difference.
// * `FUN_00650c8f` - `in_AL` gate plus a no-argument `FUN_00695c10`; the
//   lookup's argument registers are not recovered.
// * `FUN_00650ee0` - unit-group validation; rewrites the transport list at
//   `+0x18`/`+0x30` and reinserts children through `FUN_00654420`, and only
//   part of the record layout is recovered.
// * `FUN_00650f1f`, `FUN_0065134c`, `FUN_00651382`, `FUN_0065139c`,
//   `FUN_006513d4`, `FUN_006513e2` - one-register predicates (`AX` /
//   `unaff_ESI`) whose failure branch is non-returning; the compared
//   register is not a recovered argument.
// * `FUN_00651320`, `FUN_006514d0` - condition then non-returning error call;
//   the only return value is the constant 0.
// * `FUN_006515b0`, `FUN_006517b0`, `FUN_006517e0` - coordinate wrappers around
//   `FUN_006515e0`; adding them would cite a helper this batch did not
//   implement.
// * `FUN_006515e0` - neighbour-list cache; the recovered loop reads the head
//   entry after the empty-list branch and the cache reset is a non-returning
//   `memset`, so the recovered control flow cannot be made terminating.
// * `FUN_00651b70`, `FUN_00651b91` - duplicate target-acquisition variants over
//   `unaff_EDI` unit records and the tile flag table at
//   `0x00449fc + DAT_011a6478`; the list walk and one path are
//   non-returning.
// * `FUN_00651e10` - unconditional non-returning `FUN_005b4cd0`.
// * `FUN_00651e90` - random player-slot pick over eight `FUN_005b0fb0` slots;
//   the relation tests (`FUN_0059c800`/`FUN_0059c8a0`) are opaque and the
//   empty case is non-returning.
// * `FUN_00651b6e` - `__break`, a one-byte data marker, not a function.

// ---------------------------------------------------------------------------
// 0x00650600 - grow a selection bounds record by one unit footprint
// ---------------------------------------------------------------------------

/// Selection bounds record written by `0x00650600`.
///
/// The routine touches six `short` positions: word 0 is the unit x coordinate
/// compared against the unit's own x, word 2 supplies the owner byte, and
/// words 8..=11 are the four values grown by the unit footprint.  Words
/// 1 and 3..=7 are never read or written and are not modelled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a05SelectionBounds {
    /// `+0x00`: candidate x coordinate (the routine compares it with the
    /// unit's `FUN_0075d7b0` result).
    pub word0: i16,
    /// `+0x04` low byte: candidate owner (`CUnit+0x4c`).
    pub owner: u8,
    /// `param_2[8]` (`+0x10`): first extent word, grown by the unit height.
    pub word8: i16,
    /// `param_2[9]` (`+0x12`): grown by the unit width.
    pub word9: i16,
    /// `param_2[10]` (`+0x14`): grown by the unit height.
    pub word10: i16,
    /// `param_2[11]` (`+0x16`): grown by the unit width.
    pub word11: i16,
}

/// `0x00650600` (`FUN_00650600`): expand one selection bounds record when the
/// unit is the record's origin and belongs to the record's owner.
///
/// ```c
/// if (FUN_005b5680() == 0
///     && record[2].lo == unit[+0x4c]
///     && record[0] == FUN_0075d7b0(unit)) {
///     record[0x0b] += FUN_0064fa60(unit);   // width
///     record[9]    += FUN_0064fa60(unit);
///     record[0x0a] += FUN_0064fa70(unit);   // height
///     record[8]    += FUN_0064fa70(unit);
/// }
/// return 0;
/// ```
///
/// The `FUN_005b5680` gate is caller-supplied as `suppressed` (retail expands
/// only when it returns zero).  The function itself returns 0 unconditionally;
/// the model returns whether the record changed so the caller can observe the
/// gate without re-reading the record.  Additions wrap at 16 bits like the
/// original `short` stores.
pub fn w11a05_expand_selection_bounds(
    bounds: &mut W11a05SelectionBounds,
    unit_x: i16,
    unit_owner: u8,
    unit_width: i16,
    unit_height: i16,
    suppressed: bool,
) -> bool {
    if suppressed || bounds.owner != unit_owner || bounds.word0 != unit_x {
        return false;
    }
    bounds.word11 = bounds.word11.wrapping_add(unit_width);
    bounds.word9 = bounds.word9.wrapping_add(unit_width);
    bounds.word10 = bounds.word10.wrapping_add(unit_height);
    bounds.word8 = bounds.word8.wrapping_add(unit_height);
    true
}

// ---------------------------------------------------------------------------
// 0x00650820 - min-biased extent arithmetic
// ---------------------------------------------------------------------------

/// `0x00650820` (`FUN_00650820`, 43 bytes): the two-lookup extent formula.
///
/// ```c
/// first  = FUN_00695c10(0x7d, index);
/// second = FUN_00695c10(0x00, index);
/// extra  = min(second, (u32)(first << 2));
/// return second + extra;
/// ```
///
/// Both lookups are caller-supplied; the routine itself only performs the
/// shift, the unsigned minimum and the unsigned add.  Because `extra` is at
/// most `second`, the result lies in `second..=2 * second` (modulo `u32`
/// wraparound), which is the whole observable arithmetic of the row.
pub fn w11a05_min_biased_extent(width_at_7d: i32, width_at_0: u32) -> u32 {
    let scaled = (width_at_7d as u32) << 2;
    let extra = width_at_0.min(scaled);
    width_at_0.wrapping_add(extra)
}

// ---------------------------------------------------------------------------
// 0x00650870 - decode a command slot table
// ---------------------------------------------------------------------------

/// `0x00650870` (`FUN_00650870`, 136 bytes): copy the nonzero command ids in a
/// per-unit-type table, decrementing each by one and stopping at the first
/// zero word.
///
/// ```c
/// i = 0;
/// for (p = table; *p != 0; p++) out[i++] = *p - 1;
/// return i;
/// ```
///
/// The routine's tail walks the caller's unit list (`FUN_005a3180`,
/// `FUN_005b6370`, `FUN_005b4bb0`) but its only effect is to return the same
/// count on the empty-list path and to reach `FUN_005b4cd0` otherwise; that
/// tail is not modelled because it cannot change `out` or the returned count.
pub fn w11a05_decode_command_slots(table: &[u16]) -> Vec<u32> {
    let mut out = Vec::new();
    for &value in table {
        if value == 0 {
            break;
        }
        out.push(u32::from(value - 1));
    }
    out
}

// ---------------------------------------------------------------------------
// 0x00650c60 - command-id lookup guard
// ---------------------------------------------------------------------------

/// `0x00650c60` (`FUN_00650c60`, 55 bytes): command-id availability guard.
///
/// ```c
/// if (FUN_00695c10(0x45, id & 0xff) == 0
///     && FUN_00695c10(0x0b, id & 0xff) == 0) {
///     FUN_00773180(id, 0x18);   // non-returning
/// }
/// return 1;
/// ```
///
/// The two lookups are caller-supplied booleans.  `Some(())` is the normal
/// return; `None` is the non-returning `FUN_00773180` path, which the original
/// never returns from.  The same guard is the reason the first branch of
/// [`w11a05_target_acceptable`] is dead in retail.
pub fn w11a05_command_id_known(lookup_0x45: bool, lookup_0x0b: bool) -> Option<()> {
    if lookup_0x45 || lookup_0x0b {
        Some(())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// 0x0065113b - random ready-unit spot
// ---------------------------------------------------------------------------

/// State codes accepted by the candidate scan in `0x0065113b`.
pub const W11A05_SPOT_STATE_A: u8 = 4;
/// Second accepted state code in `0x0065113b`.
pub const W11A05_SPOT_STATE_B: u8 = 5;

/// Flag mask tested by the candidate scan in `0x0065113b` (`& 0x41`).
pub const W11A05_SPOT_FLAG_MASK: u8 = 0x41;

/// Jitter mask applied to each random draw in `0x0065113b` (`& 0x1ff`).
pub const W11A05_SPOT_JITTER_MASK: u32 = 0x1ff;

/// Jitter bias subtracted from each masked draw in `0x0065113b`.
pub const W11A05_SPOT_JITTER_BIAS: i32 = 0x100;

/// Half extent of the occupancy rectangle tested by `0x0065113b` (`0x60`).
pub const W11A05_SPOT_RECT_HALF: i32 = 0x60;

/// One unit row read by the candidate scan in `0x0065113b`.
///
/// The scan reads the state byte at `-3` from its 0x34-byte row pointer and
/// the flag byte at `+0`; those relative positions are presented as named
/// fields here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W11a05SpotCandidate {
    /// State byte at row offset `-3` (the `4`/`5` test).
    pub state: u8,
    /// Flag byte at row offset `+0` (`& 0x41`).
    pub flags: u8,
    /// Position returned by `FUN_0075d630` for the row.
    pub x: i32,
    /// Second position word returned by `FUN_0075d630`.
    pub y: i32,
}

/// The three `FUN_0073f3b0` draws used by `0x0065113b`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W11a05SpotRolls {
    /// `rand(0x2c)`, reduced modulo the eligible-candidate count.
    pub pick: u32,
    /// `rand(0x2d)`, masked with `0x1ff` and biased by `-0x100`.
    pub jitter_x: u32,
    /// `rand(0x2e)`, masked with `0x1ff` and biased by `-0x100`.
    pub jitter_y: u32,
}

/// `0x0065113b`: the candidate filter.
///
/// ```c
/// (state == 4 || state == 5) && (flags & 0x41) != 0
/// ```
pub fn w11a05_spot_candidate_eligible(state: u8, flags: u8) -> bool {
    (state == W11A05_SPOT_STATE_A || state == W11A05_SPOT_STATE_B)
        && flags & W11A05_SPOT_FLAG_MASK != 0
}

/// The clamp applied to each jittered coordinate by `0x0065113b`.
fn w11a05_clamp_axis(value: i32, extent: i32) -> i32 {
    if value < 0 {
        0
    } else if value >= extent {
        (extent - 1).max(0)
    } else {
        value
    }
}

/// `0x0065113b` (`FUN_0065113b`, 402 bytes): choose a random eligible unit and
/// return a jittered, clamped position near it when the position gate is open,
/// the position is clear, and no unit occupies the surrounding `0x60` rectangle.
///
/// The body scans the caller's unit array, keeps the rows satisfying
/// [`w11a05_spot_candidate_eligible`], picks `rand(0x2c) % count`, adds
/// `(rand(0x2d) & 0x1ff) - 0x100` to x and `(rand(0x2e) & 0x1ff) - 0x100` to y,
/// clamps both axes to the map extent, and then:
///
/// ```c
/// if (FUN_0075dab0() != 0 && FUN_0075d970(x, y) == 0) {
///     list = FUN_005a57f0({x - 0x60, y - 0x60, x + 0x60, y + 0x60});
///     if (list[0] == 0) { *out_x = x; *out_y = y; return; }
/// }
/// FUN_005b4cd0();   // non-returning
/// ```
///
/// The three random draws from [`W11a05SpotRolls`] are caller-supplied so the
/// choice is deterministic in tests.  `position_gate` is
/// `FUN_0075dab0() != 0`, `position_clear` is `FUN_0075d970(x, y) == 0`, and
/// `units_in_rect` is the length of the `FUN_005a57f0` rectangle list.  `None`
/// models every non-returning path: no eligible unit, a closed gate, a blocked
/// position or a non-empty rectangle.
///
/// The decompiler prints the scan as `while (false)`, but the body increments
/// the row index and loops while it is below the count, so the recovered form
/// is the `for`-style filter used here.
pub fn w11a05_random_ready_unit_spot(
    units: &[W11a05SpotCandidate],
    rolls: W11a05SpotRolls,
    map_width: i32,
    map_height: i32,
    position_gate: bool,
    position_clear: bool,
    units_in_rect: impl Fn(i32, i32, i32, i32) -> usize,
) -> Option<(i32, i32)> {
    let eligible: Vec<&W11a05SpotCandidate> = units
        .iter()
        .filter(|unit| w11a05_spot_candidate_eligible(unit.state, unit.flags))
        .collect();
    if eligible.is_empty() {
        return None;
    }
    let index = (rolls.pick % eligible.len() as u32) as usize;
    let chosen = eligible[index];
    let jittered_x =
        chosen.x + ((rolls.jitter_x & W11A05_SPOT_JITTER_MASK) as i32 - W11A05_SPOT_JITTER_BIAS);
    let jittered_y =
        chosen.y + ((rolls.jitter_y & W11A05_SPOT_JITTER_MASK) as i32 - W11A05_SPOT_JITTER_BIAS);
    let x = w11a05_clamp_axis(jittered_x, map_width);
    let y = w11a05_clamp_axis(jittered_y, map_height);
    if !position_gate || !position_clear {
        return None;
    }
    let occupied = units_in_rect(
        x - W11A05_SPOT_RECT_HALF,
        y - W11A05_SPOT_RECT_HALF,
        x + W11A05_SPOT_RECT_HALF,
        y + W11A05_SPOT_RECT_HALF,
    );
    if occupied != 0 {
        return None;
    }
    Some((x, y))
}

// ---------------------------------------------------------------------------
// 0x00651410 - release the eight unit-row slots
// ---------------------------------------------------------------------------

/// Slot count of the `0x00fd2d0c` table (`0x20` bytes / 4), released by
/// `0x00651410`.
pub const W11A05_UNIT_ROW_SLOTS: usize = 8;

/// `0x00651410` (`FUN_00651410`, 48 bytes): release every non-null pointer in
/// the eight-slot `0x00fd2d0c` unit-row table and zero each slot.
///
/// ```c
/// for (i = 0; i < 0x20; i += 4) {
///     if (table[i / 4] != 0) FUN_00924220(table[i / 4], ...);
///     table[i / 4] = 0;
/// }
/// ```
///
/// The model returns the released handles in slot order so the caller can
/// observe what was freed; the table is left with eight `None` entries.
pub fn w11a05_release_unit_row_slots(
    slots: &mut [Option<u32>; W11A05_UNIT_ROW_SLOTS],
) -> [u32; W11A05_UNIT_ROW_SLOTS] {
    let mut released = [0u32; W11A05_UNIT_ROW_SLOTS];
    for (index, slot) in slots.iter_mut().enumerate() {
        if let Some(handle) = slot.take() {
            released[index] = handle;
        }
    }
    released
}

// ---------------------------------------------------------------------------
// 0x00651450 - target-list flags
// ---------------------------------------------------------------------------

/// The relation byte compared against `-0x7e` by `0x00651450`.
pub const W11A05_TARGET_RELATION: i8 = -0x7e;

/// One unit read by `0x00651450`.
///
/// `relation_a` and `relation_b` are the results of the two opaque per-unit
/// helpers (`FUN_005a2e90` and `FUN_005a2d90`); `primary` is true when
/// `FUN_005b5c70` returns zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W11a05TargetUnit {
    /// Result of `FUN_005a2e90`, compared with [`W11A05_TARGET_RELATION`].
    pub relation_a: i8,
    /// Result of `FUN_005a2d90`, compared with [`W11A05_TARGET_RELATION`].
    pub relation_b: i8,
    /// `FUN_005b5c70() == 0`.
    pub primary: bool,
}

/// The three dwords written by `0x00651450`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a05TargetFlags {
    /// First output: set when any unit is primary.
    pub has_primary: bool,
    /// Second output: remains set when every unit's `relation_a` is `-0x7e`.
    pub all_relation_a: bool,
    /// Third output: remains set when every unit's `relation_b` is `-0x7e`.
    pub all_relation_b: bool,
}

/// `0x00651450` (`FUN_00651450`, 119 bytes): fold the target record's
/// `+0x30` unit list into three output flags.
///
/// ```c
/// out0 = 0; out1 = 1; out2 = 1;
/// for (node = *(record + 0x30); node; node = *node) {
///     if (FUN_005a2e90() != -0x7e) out1 = 0;
///     if (FUN_005a2d90() != -0x7e) out2 = 0;
///     if (FUN_005b5c70() == 0)      out0 = 1;
/// }
/// ```
///
/// An empty list therefore yields `(false, true, true)`.  The linked-list walk
/// is presented as a slice in list order.
pub fn w11a05_target_list_flags(units: &[W11a05TargetUnit]) -> W11a05TargetFlags {
    let mut flags = W11a05TargetFlags {
        has_primary: false,
        all_relation_a: true,
        all_relation_b: true,
    };
    for unit in units {
        if unit.relation_a != W11A05_TARGET_RELATION {
            flags.all_relation_a = false;
        }
        if unit.relation_b != W11A05_TARGET_RELATION {
            flags.all_relation_b = false;
        }
        if unit.primary {
            flags.has_primary = true;
        }
    }
    flags
}

// ---------------------------------------------------------------------------
// 0x00651590 - state 8/9 predicate
// ---------------------------------------------------------------------------

/// `0x00651590` (`FUN_00651590`, 28 bytes): true only for the two state codes
/// `8` and `9`.
///
/// The body compares the byte at record offset `+5` (the same state slot
/// written by [`w11a05_apply_display_state`]) with `8` and `9`.
pub fn w11a05_state_is_eight_or_nine(state: u8) -> bool {
    state == 8 || state == 9
}

// ---------------------------------------------------------------------------
// 0x006516d0 - per-player reachability flood fill
// ---------------------------------------------------------------------------

/// Cell count of one player's `0x00fd2d38` reachability grid (`0x9c4`),
/// indexed by `0x006516d0`.
pub const W11A05_REGION_STRIDE: usize = 0x9c4;

/// `0x006516d0` (`FUN_006516d0`, 205 bytes): flood the per-player unit-index
/// grid from `start` and report whether `target` was reached.
///
/// The grid is indexed by unit slot inside one player's `0x9c4`-cell window
/// (`region[base + slot]`).  The routine writes:
///
/// * `1` at the start cell;
/// * for every neighbour of a cell that still holds `1`: `2` when the unit row
///   has a nonzero dword at `+0x1c` or `+0x20`, otherwise `1`;
///
/// and repeats full sweeps until a sweep discovers no new cell.  Cells holding
/// `2` are terminal: they are never expanded.  The final answer is
/// `region[base + target] == 2`, which is why a blocked neighbour can be a
/// positive result while units behind it stay unreachable.
///
/// `neighbors[slot]` is the `FUN_0075d6b0` list for that unit slot and
/// `blocked[slot]` is the `+0x1c`/`+0x20` classification.  Slots outside
/// `0..count` are ignored defensively; the retail body trusts the lists.
pub fn w11a05_region_flood_reach(
    region: &mut [u8],
    base: usize,
    start: usize,
    target: usize,
    count: usize,
    neighbors: &[Vec<u16>],
    blocked: &[bool],
) -> bool {
    assert!(
        base + count <= region.len(),
        "reachability window does not fit the grid"
    );
    assert!(start < count, "start slot outside the reachability window");
    assert!(
        target < count,
        "target slot outside the reachability window"
    );
    if count < 2 {
        return false;
    }
    region[base + start] = 1;
    loop {
        let mut discovered = false;
        for slot in 1..count {
            if region[base + slot] != 1 {
                continue;
            }
            region[base + slot] = 2;
            let Some(list) = neighbors.get(slot) else {
                continue;
            };
            for &neighbour in list {
                let index = neighbour as usize;
                if index >= count || region[base + index] != 0 {
                    continue;
                }
                let is_blocked = blocked.get(index).copied().unwrap_or(false);
                region[base + index] = if is_blocked { 2 } else { 1 };
                discovered = true;
            }
        }
        if !discovered {
            break;
        }
    }
    region[base + target] == 2
}

// ---------------------------------------------------------------------------
// 0x00651820 - target acceptance predicate
// ---------------------------------------------------------------------------

/// `0x00651820` (`FUN_00651820`, 268 bytes): decide whether a clicked target
/// record is acceptable to the acting unit.
///
/// The routine first calls `FUN_00651450` on the target record (translated as
/// [`w11a05_target_list_flags`]).  The guarded block that follows is dead in
/// retail: it runs only when `FUN_00650c60` returns zero, but that guard
/// returns `1` or takes the non-returning `FUN_00773180` path (see
/// [`w11a05_command_id_known`]).  The live part is:
///
/// ```c
/// if ((!all_b && !all_a) || actor[0] == target[0]) return 0;
/// for (unit in units_at_target_owned_by_target) {
///     if ((!FUN_005b5c70() && all_b) || (FUN_005b5c70() && all_a)) return 0;
/// }
/// return 1;
/// ```
///
/// `target_units` is the target record's `+0x30` chain; `target_owner_units`
/// holds the `FUN_005b5c70` result for the `FUN_005a5790` location list after
/// the `FUN_005b5b90` owner filter, in list order.
pub fn w11a05_target_acceptable(
    actor_id: u16,
    target_id: u16,
    target_units: &[W11a05TargetUnit],
    target_owner_units: &[bool],
) -> bool {
    let flags = w11a05_target_list_flags(target_units);
    if (!flags.all_relation_a && !flags.all_relation_b) || actor_id == target_id {
        return false;
    }
    for &secondary in target_owner_units {
        if (!secondary && flags.all_relation_b) || (secondary && flags.all_relation_a) {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// 0x00651930 - nearest primary/secondary target
// ---------------------------------------------------------------------------

/// Initial distance sentinel in `0x00651930` (`99999999`).
pub const W11A05_TARGET_DISTANCE_LIMIT: u32 = 99_999_999;

/// One candidate in the `+0x30` list scan of `0x00651930`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W11a05TargetCandidate {
    /// Unit id stored at node `+0x0c`.
    pub id: u32,
    /// `FUN_00569ab0` distance to the candidate.
    pub distance: u32,
    /// `FUN_005b5c70() != 0`.
    pub secondary: bool,
}

/// `0x00651930` (`FUN_00651930`, 189 bytes): pick the closest primary target,
/// falling back to the closest secondary target.
///
/// ```c
/// if (FUN_00635720(record[+4]) == 0) return;
/// primary = 0; secondary = 0;
/// primary_d = LIMIT; secondary_d = LIMIT;
/// if (record[+0x24] != 0) {           // seed with the current target
///     if (!secondary(current)) { primary = current; primary_d = distance(current); }
///     else                     { secondary = current; secondary_d = distance(current); }
/// }
/// for (node in record[+0x30]) {
///     if (!secondary(node) && distance(node) < primary_d)   { primary = node; primary_d = d; }
///     if ( secondary(node) && distance(node) < secondary_d) { secondary = node; secondary_d = d; }
/// }
/// if (primary != 0) secondary = primary;
/// record[+0x24] = secondary;
/// ```
///
/// `enabled` is the `FUN_00635720` gate; when it is false the cached target is
/// left untouched and the model returns `None`.  When it is true the model
/// returns `Some(selected)`, where zero means no target.
pub fn w11a05_select_nearest_target(
    enabled: bool,
    current: Option<W11a05TargetCandidate>,
    candidates: &[W11a05TargetCandidate],
) -> Option<u32> {
    if !enabled {
        return None;
    }
    let mut primary = (0u32, W11A05_TARGET_DISTANCE_LIMIT);
    let mut secondary = (0u32, W11A05_TARGET_DISTANCE_LIMIT);
    if let Some(candidate) = current.filter(|candidate| candidate.id != 0) {
        if candidate.secondary {
            if candidate.distance < secondary.1 {
                secondary = (candidate.id, candidate.distance);
            }
        } else if candidate.distance < primary.1 {
            primary = (candidate.id, candidate.distance);
        }
    }
    for candidate in candidates {
        if candidate.secondary {
            if candidate.distance < secondary.1 {
                secondary = (candidate.id, candidate.distance);
            }
        } else if candidate.distance < primary.1 {
            primary = (candidate.id, candidate.distance);
        }
    }
    Some(if primary.0 != 0 {
        primary.0
    } else {
        secondary.0
    })
}

// ---------------------------------------------------------------------------
// 0x006519f0 - unit display-state setter
// ---------------------------------------------------------------------------

/// Display record written by `0x006519f0`.
///
/// The record is a `short` array.  `extent[i]` is `param_1[6 + i]`, i.e. the
/// sixteen words at `+12..+42`: the idle branch clears all of them and then
/// writes the two words at `+12`/`+14`.  The switch never touches these
/// extent words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W11a05DisplayRecord {
    /// `+0x00`: unit/type id; zero makes the whole routine a no-op.
    pub unit_id: u16,
    /// `+0x02`: cached secondary id cleared when entering a non-7 state from
    /// state 0.
    pub target_id: u16,
    /// `+0x04` low byte: owner/player index used by the caller-supplied
    /// lookups.
    pub owner: u8,
    /// `+0x05`: current state byte.
    pub state: u8,
    /// `+0x06`: animation/frame id written by the switch.
    pub anim: u8,
    /// `+0x08`: flag byte; bit `0x20` is cleared on every call and bit
    /// `0x40` is read by the state-0 branch.
    pub flags: u8,
    /// `+0x09`: substate byte cleared by state 7.
    pub substate: u8,
    /// `+0x0c..+0x2a`: sixteen extent words (`param_1[6..=0x15]`).
    pub extent: [u16; 16],
}

/// `0x006519f0` (`FUN_006519f0`, 166 bytes): apply one unit state transition.
///
/// The routine first resets the display extents when the old state is zero:
///
/// ```c
/// if (unit_id != 0) {
///     if (state == 0) {
///         extent = FUN_00635f40(owner);
///         if (FUN_00635720(owner) == 0) extent = 0;
///         param_1[6] = selected ? 0 : extent;
///         param_1[7] = selected ? extent : 0;
///         param_1[8..=0x15] = 0;
///         if (new_state != 7) param_1[1] = 0;
///     }
///     flags = param_1[4].byte;
///     param_1[4].byte = flags & 0xdf;
///     param_1[2].byte[1] = new_state;   // +5
///     switch (new_state) { ... }
/// }
/// ```
///
/// The switch writes the byte at `+6`:
///
/// | state | anim |
/// |-------|------|
/// | 0 | `10`, or `0x28` when the old flags had `0x40` and the owner is not a building |
/// | 1, 2 | `0x46` for a building, else `0xa0` |
/// | 3 | `10`, and flags become `old_flags & 0xde` |
/// | 4 | `0x28` |
/// | 5 | `0x32` |
/// | 6 | `0x6e` when the unit frame exists, else `0x64` |
/// | 7 | `0x6e` when the target frame exists, else the state-6 result; clears `+9` first |
/// | 8, 9 | `0x32` for a building, else `0xa0` |
/// | other | `10` |
///
/// `selected` is `FUN_0075d940(unit_id) != 0`, `owner_extent` is
/// `FUN_00635f40(owner)`, `is_building` is `FUN_00635720(owner) != 0`, and the
/// two `*_frame_valid` arguments are `FUN_00638d30(owner, id) != 0` for the
/// target and unit ids.  All of these leave the window and are supplied by the
/// caller.
pub fn w11a05_apply_display_state(
    record: &mut W11a05DisplayRecord,
    new_state: u8,
    selected: bool,
    owner_extent: u16,
    is_building: bool,
    target_frame_valid: bool,
    unit_frame_valid: bool,
) {
    if record.unit_id == 0 {
        return;
    }
    if record.state == 0 {
        let biased_extent = if is_building { owner_extent } else { 0 };
        record.extent[0] = if selected { 0 } else { biased_extent };
        record.extent[1] = if selected { biased_extent } else { 0 };
        for word in &mut record.extent[2..] {
            *word = 0;
        }
        if new_state != 7 {
            record.target_id = 0;
        }
    }
    let old_flags = record.flags;
    record.flags = old_flags & 0xdf;
    record.state = new_state;
    match new_state {
        0 => {
            record.anim = 10;
            if old_flags & 0x40 != 0 && !is_building {
                record.anim = 0x28;
            }
        }
        1 | 2 => {
            record.anim = if is_building {
                // Retail stores `0xa0 + 0xa6` in a byte, so the building
                // value is 0x46 after the 8-bit truncation.
                0xa0u8.wrapping_add(0xa6)
            } else {
                0xa0
            };
        }
        3 => {
            record.flags = old_flags & 0xde;
            record.anim = 10;
        }
        4 => record.anim = 0x28,
        5 => record.anim = 0x32,
        6 => {
            record.anim = if unit_frame_valid { 0x6e } else { 0x64 };
        }
        7 => {
            record.substate = 0;
            record.anim = if target_frame_valid || unit_frame_valid {
                0x6e
            } else {
                0x64
            };
        }
        8 | 9 => {
            record.anim = if is_building {
                // 0xa0 + 0x92 truncated to a byte.
                0xa0u8.wrapping_add(0x92)
            } else {
                0xa0
            };
        }
        _ => record.anim = 10,
    }
}

#[cfg(test)]
mod tests_wave11_agent05 {
    use super::*;
    use std::cell::Cell;

    // -- 0x00650600 --------------------------------------------------------

    #[test]
    fn selection_bounds_grow_by_the_footprint() {
        let mut bounds = W11a05SelectionBounds {
            word0: 12,
            owner: 3,
            word8: 1,
            word9: 2,
            word10: 3,
            word11: 4,
        };
        assert!(w11a05_expand_selection_bounds(
            &mut bounds,
            12,
            3,
            10,
            20,
            false
        ));
        assert_eq!(bounds.word8, 21);
        assert_eq!(bounds.word9, 12);
        assert_eq!(bounds.word10, 23);
        assert_eq!(bounds.word11, 14);
    }

    #[test]
    fn selection_bounds_reject_the_wrong_origin_owner_or_gate() {
        let original = W11a05SelectionBounds {
            word0: 12,
            owner: 3,
            word8: 1,
            word9: 2,
            word10: 3,
            word11: 4,
        };
        let mut bounds = original;
        assert!(!w11a05_expand_selection_bounds(
            &mut bounds,
            13,
            3,
            10,
            20,
            false
        ));
        assert_eq!(bounds, original);
        assert!(!w11a05_expand_selection_bounds(
            &mut bounds,
            12,
            4,
            10,
            20,
            false
        ));
        assert_eq!(bounds, original);
        assert!(!w11a05_expand_selection_bounds(
            &mut bounds,
            12,
            3,
            10,
            20,
            true
        ));
        assert_eq!(bounds, original);
    }

    #[test]
    fn selection_bounds_additions_wrap_at_sixteen_bits() {
        let mut bounds = W11a05SelectionBounds {
            word0: 0,
            owner: 1,
            word8: i16::MAX,
            word9: i16::MAX,
            word10: i16::MIN,
            word11: i16::MIN,
        };
        assert!(w11a05_expand_selection_bounds(
            &mut bounds,
            0,
            1,
            1,
            -1,
            false
        ));
        assert_eq!(bounds.word8, i16::MAX - 1);
        assert_eq!(bounds.word9, i16::MIN);
        assert_eq!(bounds.word10, i16::MAX);
        assert_eq!(bounds.word11, i16::MIN + 1);
    }

    // -- 0x00650820 --------------------------------------------------------

    #[test]
    fn min_biased_extent_uses_the_smaller_of_the_two_lookups() {
        assert_eq!(w11a05_min_biased_extent(0, 10), 10);
        assert_eq!(w11a05_min_biased_extent(2, 10), 18);
        assert_eq!(w11a05_min_biased_extent(3, 10), 20);
        assert_eq!(w11a05_min_biased_extent(100, 7), 14);
        assert_eq!(w11a05_min_biased_extent(-1, 4), 8);
        assert_eq!(w11a05_min_biased_extent(-1, 0), 0);
    }

    // -- 0x00650870 --------------------------------------------------------

    #[test]
    fn command_slots_are_decremented_and_stop_at_zero() {
        assert_eq!(w11a05_decode_command_slots(&[]), Vec::<u32>::new());
        assert_eq!(w11a05_decode_command_slots(&[1, 2, 3]), vec![0, 1, 2]);
        assert_eq!(
            w11a05_decode_command_slots(&[1, 0xffff, 0, 7]),
            vec![0, 65534]
        );
        assert_eq!(w11a05_decode_command_slots(&[0, 9]), Vec::<u32>::new());
    }

    // -- 0x00650c60 --------------------------------------------------------

    #[test]
    fn command_id_guard_accepts_either_lookup() {
        assert_eq!(w11a05_command_id_known(true, false), Some(()));
        assert_eq!(w11a05_command_id_known(false, true), Some(()));
        assert_eq!(w11a05_command_id_known(true, true), Some(()));
        assert_eq!(w11a05_command_id_known(false, false), None);
    }

    // -- 0x0065113b --------------------------------------------------------

    fn spot(state: u8, flags: u8, x: i32, y: i32) -> W11a05SpotCandidate {
        W11a05SpotCandidate { state, flags, x, y }
    }

    #[test]
    fn spot_candidate_filter_requires_state_four_or_five_and_a_flag() {
        assert!(w11a05_spot_candidate_eligible(4, 0x01));
        assert!(w11a05_spot_candidate_eligible(5, 0x40));
        assert!(!w11a05_spot_candidate_eligible(4, 0x02));
        assert!(!w11a05_spot_candidate_eligible(5, 0x00));
        assert!(!w11a05_spot_candidate_eligible(6, 0x41));
        assert!(!w11a05_spot_candidate_eligible(3, 0x41));
    }

    #[test]
    fn random_spot_selects_an_eligible_unit_and_jitters_it() {
        let units = [
            spot(4, 0x01, 10, 20),
            spot(5, 0x40, 100, 200),
            spot(6, 0x41, 50, 50),
        ];
        let seen = Cell::new((0, 0, 0, 0));
        let result = w11a05_random_ready_unit_spot(
            &units,
            W11a05SpotRolls {
                pick: 1,
                jitter_x: 0x100,
                jitter_y: 0x100,
            },
            1000,
            1000,
            true,
            true,
            |left, top, right, bottom| {
                seen.set((left, top, right, bottom));
                0
            },
        );
        assert_eq!(result, Some((100, 200)));
        assert_eq!(seen.get(), (100 - 0x60, 200 - 0x60, 100 + 0x60, 200 + 0x60));
    }

    #[test]
    fn random_spot_clamps_to_the_map_and_rejects_blocked_positions() {
        let units = [spot(4, 0x01, 10, 20)];
        assert_eq!(
            w11a05_random_ready_unit_spot(
                &units,
                W11a05SpotRolls {
                    pick: 0,
                    jitter_x: 0,
                    jitter_y: 0x1ff,
                },
                100,
                100,
                true,
                true,
                |_, _, _, _| 0
            ),
            Some((0, 99))
        );
        assert_eq!(
            w11a05_random_ready_unit_spot(
                &units,
                W11a05SpotRolls {
                    pick: 0,
                    jitter_x: 0x100,
                    jitter_y: 0x100,
                },
                100,
                100,
                false,
                true,
                |_, _, _, _| 0
            ),
            None
        );
        assert_eq!(
            w11a05_random_ready_unit_spot(
                &units,
                W11a05SpotRolls {
                    pick: 0,
                    jitter_x: 0x100,
                    jitter_y: 0x100,
                },
                100,
                100,
                true,
                false,
                |_, _, _, _| 0
            ),
            None
        );
        assert_eq!(
            w11a05_random_ready_unit_spot(
                &units,
                W11a05SpotRolls {
                    pick: 0,
                    jitter_x: 0x100,
                    jitter_y: 0x100,
                },
                100,
                100,
                true,
                true,
                |_, _, _, _| 1
            ),
            None
        );
    }

    #[test]
    fn random_spot_has_no_result_without_an_eligible_unit() {
        let units = [spot(3, 0x41, 1, 2), spot(7, 0x41, 3, 4)];
        assert_eq!(
            w11a05_random_ready_unit_spot(
                &units,
                W11a05SpotRolls {
                    pick: 0,
                    jitter_x: 0,
                    jitter_y: 0,
                },
                10,
                10,
                true,
                true,
                |_, _, _, _| 0
            ),
            None
        );
    }

    // -- 0x00651410 --------------------------------------------------------

    #[test]
    fn unit_row_slots_are_released_and_cleared() {
        let mut slots = [Some(11), None, Some(33), None, None, Some(55), None, None];
        let released = w11a05_release_unit_row_slots(&mut slots);
        assert_eq!(released, [11, 0, 33, 0, 0, 55, 0, 0]);
        assert!(slots.iter().all(Option::is_none));
        assert_eq!(
            w11a05_release_unit_row_slots(&mut slots),
            [0; W11A05_UNIT_ROW_SLOTS]
        );
    }

    // -- 0x00651450 --------------------------------------------------------

    fn target_unit(relation_a: i8, relation_b: i8, primary: bool) -> W11a05TargetUnit {
        W11a05TargetUnit {
            relation_a,
            relation_b,
            primary,
        }
    }

    #[test]
    fn target_flags_fold_the_unit_list() {
        let units = [
            target_unit(W11A05_TARGET_RELATION, W11A05_TARGET_RELATION, false),
            target_unit(W11A05_TARGET_RELATION, 0, true),
        ];
        assert_eq!(
            w11a05_target_list_flags(&units),
            W11a05TargetFlags {
                has_primary: true,
                all_relation_a: true,
                all_relation_b: false,
            }
        );
        assert_eq!(
            w11a05_target_list_flags(&[]),
            W11a05TargetFlags {
                has_primary: false,
                all_relation_a: true,
                all_relation_b: true,
            }
        );
        assert!(!w11a05_target_list_flags(&[target_unit(0, 0, false)]).has_primary);
    }

    // -- 0x00651590 --------------------------------------------------------

    #[test]
    fn state_predicate_accepts_only_eight_and_nine() {
        assert!(w11a05_state_is_eight_or_nine(8));
        assert!(w11a05_state_is_eight_or_nine(9));
        for state in [0u8, 1, 7, 10, 0xff] {
            assert!(!w11a05_state_is_eight_or_nine(state));
        }
    }

    // -- 0x006516d0 --------------------------------------------------------

    #[test]
    fn flood_reaches_through_open_cells_and_stops_at_blocked_ones() {
        let mut region = [0u8; 8];
        // Prefix cells outside the window must not be touched.
        region[0] = 9;
        region[1] = 9;
        let neighbors = vec![Vec::new(), vec![2], vec![3], Vec::new()];
        let blocked = [false, false, false, false];
        assert!(w11a05_region_flood_reach(
            &mut region,
            4,
            1,
            3,
            4,
            &neighbors,
            &blocked
        ));
        assert_eq!(&region[0..2], &[9, 9]);
        assert_eq!(region[4 + 1], 2);
        assert_eq!(region[4 + 2], 2);
        assert_eq!(region[4 + 3], 2);
    }

    #[test]
    fn flood_does_not_expand_a_blocked_cell() {
        let mut region = [0u8; 8];
        let neighbors = vec![Vec::new(), vec![2], vec![3], Vec::new()];
        let blocked = [false, false, true, false];
        assert!(!w11a05_region_flood_reach(
            &mut region,
            4,
            1,
            3,
            4,
            &neighbors,
            &blocked
        ));
        // The blocked neighbour itself is the target, so it is accepted.
        let mut direct = [0u8; 8];
        assert!(w11a05_region_flood_reach(
            &mut direct,
            4,
            1,
            2,
            4,
            &neighbors,
            &blocked
        ));
    }

    #[test]
    fn flood_handles_start_equals_target_and_small_windows() {
        let neighbors = vec![Vec::new(), Vec::new()];
        let mut region = [0u8; 4];
        assert!(w11a05_region_flood_reach(
            &mut region,
            0,
            1,
            1,
            2,
            &neighbors,
            &[false, false]
        ));
        let mut single = [0u8; 2];
        assert!(!w11a05_region_flood_reach(
            &mut single,
            0,
            0,
            0,
            1,
            &neighbors,
            &[false, false]
        ));
    }

    // -- 0x00651820 --------------------------------------------------------

    #[test]
    fn target_acceptance_rejects_empty_flags_and_self_targets() {
        assert!(w11a05_target_acceptable(1, 2, &[], &[]));
        assert!(!w11a05_target_acceptable(
            1,
            2,
            &[target_unit(0, 0, false)],
            &[]
        ));
        assert!(!w11a05_target_acceptable(1, 1, &[], &[]));
    }

    #[test]
    fn target_acceptance_matches_the_owner_unit_classes() {
        let all_a = [target_unit(W11A05_TARGET_RELATION, 0, false)];
        let all_b = [target_unit(0, W11A05_TARGET_RELATION, false)];
        // all_a set: a secondary owner unit is rejected, a primary one is not.
        assert!(!w11a05_target_acceptable(1, 2, &all_a, &[true]));
        assert!(w11a05_target_acceptable(1, 2, &all_a, &[false]));
        // all_b set: a primary owner unit is rejected, a secondary one is not.
        assert!(!w11a05_target_acceptable(1, 2, &all_b, &[false]));
        assert!(w11a05_target_acceptable(1, 2, &all_b, &[true]));
    }

    // -- 0x00651930 --------------------------------------------------------

    fn candidate(id: u32, distance: u32, secondary: bool) -> W11a05TargetCandidate {
        W11a05TargetCandidate {
            id,
            distance,
            secondary,
        }
    }

    #[test]
    fn nearest_target_prefers_the_closest_primary() {
        let current = candidate(10, 50, false);
        let candidates = [
            candidate(20, 100, false),
            candidate(30, 20, false),
            candidate(40, 5, true),
        ];
        assert_eq!(
            w11a05_select_nearest_target(true, Some(current), &candidates),
            Some(30)
        );
        // The current primary keeps its slot on a distance tie.
        assert_eq!(
            w11a05_select_nearest_target(
                true,
                Some(candidate(10, 10, false)),
                &[candidate(20, 10, false)]
            ),
            Some(10)
        );
    }

    #[test]
    fn nearest_target_falls_back_to_the_closest_secondary() {
        let candidates = [candidate(40, 9, true), candidate(50, 5, true)];
        assert_eq!(
            w11a05_select_nearest_target(true, None, &candidates),
            Some(50)
        );
        assert_eq!(
            w11a05_select_nearest_target(
                true,
                Some(candidate(60, 2, true)),
                &[candidate(40, 9, true)]
            ),
            Some(60)
        );
        // A primary candidate always wins over the secondary fallback.
        assert_eq!(
            w11a05_select_nearest_target(
                true,
                Some(candidate(60, 2, true)),
                &[candidate(70, 100, false)]
            ),
            Some(70)
        );
    }

    #[test]
    fn nearest_target_honours_the_gate_and_the_distance_limit() {
        assert_eq!(w11a05_select_nearest_target(false, None, &[]), None);
        assert_eq!(w11a05_select_nearest_target(true, None, &[]), Some(0));
        assert_eq!(
            w11a05_select_nearest_target(
                true,
                None,
                &[candidate(80, W11A05_TARGET_DISTANCE_LIMIT, false)]
            ),
            Some(0)
        );
    }

    // -- 0x006519f0 --------------------------------------------------------

    fn display_record() -> W11a05DisplayRecord {
        W11a05DisplayRecord {
            unit_id: 7,
            target_id: 99,
            owner: 2,
            state: 0,
            anim: 0,
            flags: 0x20,
            substate: 5,
            extent: [0xaaaa; 16],
        }
    }

    #[test]
    fn display_state_zero_resets_the_extents_from_idle() {
        let mut record = display_record();
        w11a05_apply_display_state(&mut record, 0, false, 7, true, false, false);
        assert_eq!(record.extent[0], 7);
        assert_eq!(record.extent[1], 0);
        assert!(record.extent[2..].iter().all(|&word| word == 0));
        assert_eq!(record.target_id, 0);
        assert_eq!(record.anim, 10);
        assert_eq!(record.state, 0);
        assert_eq!(record.flags, 0x00);
        assert_eq!(record.substate, 5);

        let mut selected = display_record();
        w11a05_apply_display_state(&mut selected, 0, true, 7, true, false, false);
        assert_eq!(selected.extent[0], 0);
        assert_eq!(selected.extent[1], 7);
    }

    #[test]
    fn display_state_zero_does_not_reset_extents_from_another_state() {
        let mut record = display_record();
        record.state = 3;
        record.extent = [0x1111; 16];
        w11a05_apply_display_state(&mut record, 0, false, 7, true, false, false);
        assert_eq!(record.extent, [0x1111; 16]);
        assert_eq!(record.target_id, 99);
    }

    #[test]
    fn display_state_zero_uses_the_old_flag_and_building_gate() {
        let mut record = display_record();
        record.flags = 0x60;
        w11a05_apply_display_state(&mut record, 0, false, 0, false, false, false);
        assert_eq!(record.anim, 0x28);
        assert_eq!(record.flags, 0x40);

        let mut building = display_record();
        building.flags = 0x60;
        w11a05_apply_display_state(&mut building, 0, false, 0, true, false, false);
        assert_eq!(building.anim, 10);
    }

    #[test]
    fn display_states_one_and_two_use_the_building_frame() {
        for state in [1u8, 2] {
            let mut building = display_record();
            building.state = 4;
            w11a05_apply_display_state(&mut building, state, false, 0, true, false, false);
            assert_eq!(building.anim, 0x46);
            let mut unit = display_record();
            unit.state = 4;
            w11a05_apply_display_state(&mut unit, state, false, 0, false, false, false);
            assert_eq!(unit.anim, 0xa0);
        }
    }

    #[test]
    fn display_state_three_clears_the_extra_flag_bit() {
        let mut record = display_record();
        record.state = 4;
        record.flags = 0x61;
        w11a05_apply_display_state(&mut record, 3, false, 0, true, false, false);
        assert_eq!(record.flags, 0x40);
        assert_eq!(record.anim, 10);
    }

    #[test]
    fn display_state_seven_prefers_the_target_frame() {
        let mut record = display_record();
        record.state = 4;
        record.substate = 9;
        w11a05_apply_display_state(&mut record, 7, false, 0, true, true, false);
        assert_eq!(record.substate, 0);
        assert_eq!(record.anim, 0x6e);
        assert_eq!(record.target_id, 99);

        let mut unit_frame = display_record();
        unit_frame.state = 4;
        w11a05_apply_display_state(&mut unit_frame, 7, false, 0, true, false, true);
        assert_eq!(unit_frame.anim, 0x6e);

        let mut missing = display_record();
        missing.state = 4;
        w11a05_apply_display_state(&mut missing, 7, false, 0, true, false, false);
        assert_eq!(missing.anim, 0x64);
    }

    #[test]
    fn display_state_six_uses_the_unit_frame() {
        let mut present = display_record();
        present.state = 4;
        w11a05_apply_display_state(&mut present, 6, false, 0, true, false, true);
        assert_eq!(present.anim, 0x6e);
        let mut absent = display_record();
        absent.state = 4;
        w11a05_apply_display_state(&mut absent, 6, false, 0, true, false, false);
        assert_eq!(absent.anim, 0x64);
    }

    #[test]
    fn display_states_five_and_nine_and_eight_write_their_frames() {
        let mut five = display_record();
        five.state = 4;
        w11a05_apply_display_state(&mut five, 5, false, 0, false, false, false);
        assert_eq!(five.anim, 0x32);

        for state in [8u8, 9] {
            let mut building = display_record();
            building.state = 4;
            w11a05_apply_display_state(&mut building, state, false, 0, true, false, false);
            assert_eq!(building.anim, 0x32);
            let mut unit = display_record();
            unit.state = 4;
            w11a05_apply_display_state(&mut unit, state, false, 0, false, false, false);
            assert_eq!(unit.anim, 0xa0);
        }
    }

    #[test]
    fn display_state_unknown_values_and_null_units_are_safe() {
        let mut record = display_record();
        record.state = 4;
        w11a05_apply_display_state(&mut record, 42, false, 0, true, false, false);
        assert_eq!(record.anim, 10);

        let original = W11a05DisplayRecord::default();
        let mut null_unit = original;
        w11a05_apply_display_state(&mut null_unit, 5, true, 9, true, true, true);
        assert_eq!(null_unit, original);
    }
}
