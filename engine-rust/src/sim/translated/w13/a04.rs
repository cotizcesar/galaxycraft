#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave13/agent04.rs
// ===========================================================================
// Wave 13 translation batch 04 (staging).
//
// Window: `0x00662000..0x00663fff`, the client-side **order / shared-amount
// accounting** layer of StarCraft 1.23.10.
//
// The recovered routines in this window take a *player selector* (a direct
// slot `0..=0xc`, the local player `0xd`, or one of the relation iterators
// `0xe..=0x1a`), a per-order field word, and an amount.  The outer selector
// is expanded by the group iterators in the next window; this batch keeps
// the inner table arithmetic, the field routing and the target predicates.
//
// Recovered layout (all offsets relative to the de-obfuscated image base
// `DAT_00f063cc ^ 0x10fae560`):
//
// | offset | shape | meaning |
// |---|---|---|
// | `+0x2d84` | 8 x u32 | field `0xe6` slot values |
// | `+0x2ed4` | 8 x u32 | field `0xe7` slot values |
// | `+0x2ff4` | 8 x u32 | field `0xe8` slot values |
// | `+0xb274` | `0xe4` x `0xc` u32 | order amounts; row = field, column = player |
// | `+0xeb70` | `0x14`-byte records | target areas: 4 words + link word at `+0x12` |
// | `+0xf90fe0` | type x 8 bytes | footprint back/front words |
// | `+0xf90ad0` | type x 4 bytes | half-extent words |
// | `+0xf91fe8` | type byte | target-class bits `0x08` / `0x10` / `0x20` |
// | `+0xf92388` | type byte | attached-child count |
// | `+0xfe5b5c` / `+0xfe5b60` | u16 | map width / height |
//
// Ghidra split several of these bodies at calls it wrongly marked
// *does not return* (`FUN_005b4cd0` is in fact the `CUnit+0x64` getter).
// The cached pseudocode is therefore a prefix for `FUN_006620a0`,
// `FUN_00662330`, `FUN_00662d10` and the predicates at `FUN_00663c80`,
// `FUN_006639b0` and `FUN_00663cb0`; the translations below follow the
// shipped instructions for the full body where that was needed.
//
// Skipped rows (misaligned entries, mid-function fragments and the two
// jump-table fragments whose space base Ghidra could not recover):
// `FUN_00662031`, `FUN_006627ae`, `FUN_006627b0`, `FUN_0066280d`,
// `FUN_00662bec`, `FUN_006639d8`, `FUN_00663adf`, `FUN_00663e6f`.
//
// Every routine below is a detached reference helper: nothing is wired into
// `World`, `SimUnit`, the renderer or the client, and no save/replay state
// is introduced.

// ---------------------------------------------------------------------------
// Recovered table shape
// ---------------------------------------------------------------------------

/// De-obfuscation key for the recovered client table base
/// (`DAT_00f063cc ^ 0x10fae560`).
pub const W13A04_IMAGE_KEY: u32 = 0x10fa_e560;

/// Recovered image offset of the field-`0xe6` eight-slot table.
pub const W13A04_SLOT_0E6: u32 = 0x2d84;

/// Recovered image offset of the field-`0xe7` eight-slot table.
pub const W13A04_SLOT_0E7: u32 = 0x2ed4;

/// Recovered image offset of the field-`0xe8` eight-slot table.
pub const W13A04_SLOT_0E8: u32 = 0x2ff4;

/// Recovered image offset of the order-amount matrix.
pub const W13A04_ORDER_TABLE: u32 = 0xb274;

/// Column stride of the order-amount matrix, in 32-bit words.
pub const W13A04_ORDER_STRIDE: usize = 0xc;

/// Largest order field word stored in the matrix (`0xe3` inclusive).
pub const W13A04_ORDER_FIELDS: usize = 0xe4;

/// Recovered image offset of the `0x14`-byte target-area records.
pub const W13A04_TARGET_AREAS: u32 = 0xeb70;

/// Raw word count of one target-area record.
pub const W13A04_TARGET_AREA_WORDS: usize = 0x14 / 2;

// ---------------------------------------------------------------------------
// Shared-amount dispatch (`0x00662990` set, `0x00663290` subtract)
// ---------------------------------------------------------------------------

/// Route taken by the outer player-selector switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13A04SelectorRoute {
    /// Selectors `0..=0xc` are used as a player slot directly.
    Player(u8),
    /// `0xd`: resolve to the local player before storing.
    LocalPlayer,
    /// `0xe`: relation iterator `FUN_00664b60` (relation byte `0`).
    RelationZero,
    /// `0xf`: relation iterator `FUN_00664700` (relation byte `2`).
    RelationTwo,
    /// `0x10`: relation iterator `FUN_006650a0` (relation byte `1`).
    RelationOne,
    /// `0x11`: group-slot iterator `FUN_00664610` (slots `0..0xc`).
    GroupSlots,
    /// `0x12..=0x15`: team iterator `FUN_00664ce0`, id = selector - `0x11`.
    Team(u8),
    /// `0x16..=0x19`: the retail routine returns `0` without storing.
    None,
    /// `0x1a`: relation iterator `FUN_00664f50` (not relation `2`).
    RelationNotTwo,
    /// Any other selector: no route.
    OutOfRange,
}

/// Result of a shared-amount store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13A04StoreOutcome {
    /// The value was written to the recovered table.
    Stored,
    /// The selector/field failed a range check; retail returned `0`.
    Ignored,
    /// `DAT_01165f58` is set, so retail delegates to `FUN_006ae540` with the
    /// given fourth argument (`0` for a set, `-1` for a subtract).
    Delegated(i32),
}

/// One recovered amount table.
///
/// `orders` is the `0xe4` x `0xc` word matrix at image offset `+0xb274`;
/// the retail index is `field * 0xc + player`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W13A04Amounts {
    /// Field `0xe6` slots, image offset `+0x2d84`.
    pub slot_0e6: [u32; 8],
    /// Field `0xe7` slots, image offset `+0x2ed4`.
    pub slot_0e7: [u32; 8],
    /// Field `0xe8` slots, image offset `+0x2ff4`.
    pub slot_0e8: [u32; 8],
    /// Order-amount matrix, `W13A04_ORDER_FIELDS * W13A04_ORDER_STRIDE` words.
    pub orders: Vec<u32>,
}

impl W13A04Amounts {
    /// A zeroed table with the retail matrix shape.
    pub fn new() -> Self {
        Self {
            slot_0e6: [0; 8],
            slot_0e7: [0; 8],
            slot_0e8: [0; 8],
            orders: vec![0; W13A04_ORDER_FIELDS * W13A04_ORDER_STRIDE],
        }
    }

    fn slot_mut(&mut self, field: u16) -> Option<&mut [u32; 8]> {
        match field {
            0x00e6 => Some(&mut self.slot_0e6),
            0x00e7 => Some(&mut self.slot_0e7),
            0x00e8 => Some(&mut self.slot_0e8),
            _ => None,
        }
    }

    /// Read one order-matrix word, or `None` when the field/player pair is
    /// outside the recovered matrix.
    pub fn order(&self, field: u16, player: u16) -> Option<u32> {
        if player >= 8 || field as usize >= W13A04_ORDER_FIELDS {
            return None;
        }
        let index = field as usize * W13A04_ORDER_STRIDE + player as usize;
        self.orders.get(index).copied()
    }
}

impl Default for W13A04Amounts {
    fn default() -> Self {
        Self::new()
    }
}

/// `0x00662990`: classify the outer player selector.
///
/// Retail computes `selector - 0xd` and dispatches through a 14-entry jump
/// table, so `0..=0xc` fall through to the direct-player path, `0xd` is
/// replaced by the local player, and `0xe..=0x1a` fan out over the relation
/// iterators.  Values above `0x1a` have no route.
pub fn w13a04_selector_route(selector: u32) -> W13A04SelectorRoute {
    match selector {
        0..=0xc => W13A04SelectorRoute::Player(selector as u8),
        0xd => W13A04SelectorRoute::LocalPlayer,
        0xe => W13A04SelectorRoute::RelationZero,
        0xf => W13A04SelectorRoute::RelationTwo,
        0x10 => W13A04SelectorRoute::RelationOne,
        0x11 => W13A04SelectorRoute::GroupSlots,
        0x12..=0x15 => W13A04SelectorRoute::Team((selector - 0x11) as u8),
        0x16..=0x19 => W13A04SelectorRoute::None,
        0x1a => W13A04SelectorRoute::RelationNotTwo,
        _ => W13A04SelectorRoute::OutOfRange,
    }
}

/// `0x00662990`: store one shared amount for a single player.
///
/// The outer selector has already been expanded by the caller (that is what
/// the relation iterators in the next window do).  The inner switch maps
/// fields `0xe6`/`0xe7`/`0xe8` onto the eight-slot tables and every field
/// below `0xe4` onto the order matrix at `(field * 0xc + player)`.  Field
/// `0xe5` is a no-op.  A non-zero `DAT_01165f58` takes the delegation path
/// before any range check.
pub fn w13a04_set_amount(
    amounts: &mut W13A04Amounts,
    player: u16,
    field: u16,
    value: u32,
    delegated_table: bool,
) -> W13A04StoreOutcome {
    if field == 0x00e5 {
        return W13A04StoreOutcome::Ignored;
    }
    if let Some(slots) = amounts.slot_mut(field) {
        if player >= 8 {
            return W13A04StoreOutcome::Ignored;
        }
        slots[player as usize] = value;
        return W13A04StoreOutcome::Stored;
    }
    if delegated_table {
        return W13A04StoreOutcome::Delegated(0);
    }
    if player >= 8 || field as usize >= W13A04_ORDER_FIELDS {
        return W13A04StoreOutcome::Ignored;
    }
    let index = field as usize * W13A04_ORDER_STRIDE + player as usize;
    amounts.orders[index] = value;
    W13A04StoreOutcome::Stored
}

/// `0x00663290`: subtract one shared amount for a single player.
///
/// This is the drain-side twin of [`w13a04_set_amount`]: fields `0xe6`,
/// `0xe7` and `0xe8` subtract from the eight-slot tables and the remaining
/// fields below `0xe4` subtract from the order matrix, clamping at zero
/// (`current <= amount` stores `0`).  Field `0xe5` is a no-op.  The
/// delegation path runs before the range checks in this routine and passes
/// `-1` as the fourth argument.
pub fn w13a04_subtract_amount(
    amounts: &mut W13A04Amounts,
    player: u16,
    field: u16,
    amount: u32,
    delegated_table: bool,
) -> W13A04StoreOutcome {
    if field == 0x00e5 {
        return W13A04StoreOutcome::Ignored;
    }
    if let Some(slots) = amounts.slot_mut(field) {
        if player >= 8 {
            return W13A04StoreOutcome::Ignored;
        }
        slots[player as usize] = w13a04_sub_clamped(slots[player as usize], amount);
        return W13A04StoreOutcome::Stored;
    }
    if delegated_table {
        return W13A04StoreOutcome::Delegated(-1);
    }
    if field as usize >= W13A04_ORDER_FIELDS || player >= 8 {
        return W13A04StoreOutcome::Ignored;
    }
    let index = field as usize * W13A04_ORDER_STRIDE + player as usize;
    amounts.orders[index] = w13a04_sub_clamped(amounts.orders[index], amount);
    W13A04StoreOutcome::Stored
}

/// Saturating subtraction used by both drain paths.
fn w13a04_sub_clamped(current: u32, amount: u32) -> u32 {
    current.saturating_sub(amount)
}

// ---------------------------------------------------------------------------
// Percentage scaling (`0x00662b60`, `0x00663240`, `0x00662cc4`)
// ---------------------------------------------------------------------------

/// `0x00662b60`: `base * percent / 100` in the retail 32-bit arithmetic.
///
/// The shipped body multiplies with `imul`, then divides by `100` with the
/// `0x51eb851f` reciprocal and a 5-bit shift, i.e. a 32-bit wrapping product
/// followed by an unsigned division.
pub fn w13a04_percent_of(base: u32, percent: u32) -> u32 {
    base.wrapping_mul(percent) / 100
}

/// `0x00662b60`: guarded percentage update.
///
/// Retail rejects a null unit (reporting error `0x57` through
/// `FUN_0092d5e0`) and requires `FUN_005b6b20()` to be non-zero; only then
/// does it store `[base * percent / 100]` with `FUN_005b6760` and report
/// success.  `unit_present` and `ready` model those two gates.
pub fn w13a04_scale_percentage(
    unit_present: bool,
    ready: bool,
    base: u32,
    percent: u32,
) -> Option<u32> {
    if !unit_present || !ready {
        return None;
    }
    Some(w13a04_percent_of(base, percent))
}

/// `0x00663240`: guarded percentage update through the second accessor pair.
///
/// The arithmetic is identical to [`w13a04_percent_of`]; retail gates it on
/// a non-null unit and `FUN_005b68a0()`, then calls `FUN_005b6710` with the
/// scaled value.
pub fn w13a04_scale_percentage_active(
    unit_present: bool,
    active: bool,
    base: u32,
    percent: u32,
) -> Option<u32> {
    if !unit_present || !active {
        return None;
    }
    Some(w13a04_percent_of(base, percent))
}

/// `0x00662cc4`: percentage with a minimum of one.
///
/// Ghidra's entry point is the continuation of the guarded routine entered
/// at the un-inventoried `FUN_00662ca2`, but the cached body carries the
/// complete arithmetic: scale the current value by the percentage and store
/// `max(1, scaled)`.  The optional follow-up store reads `FUN_005cc810()`
/// when `FUN_005b5e20()` is set.
pub fn w13a04_percent_at_least_one(base: u32, percent: u32) -> u32 {
    let scaled = w13a04_percent_of(base, percent);
    if scaled == 0 { 1 } else { scaled }
}

// ---------------------------------------------------------------------------
// Value application (`0x00662fc0`)
// ---------------------------------------------------------------------------

/// One accepted value update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13A04ValueUpdate {
    /// The word passed to the retail setter `FUN_005b67e0`.
    pub value: u32,
    /// Whether the follow-up `FUN_005ae170` refresh runs
    /// (`FUN_005ad950() != 0` in the shipped body).
    pub refresh: bool,
}

/// `0x00662fc0`: store a value when the object accepts it.
///
/// Retail reports error `0x57` for a null object, requires
/// `FUN_005b61d0()`, stores `param_2` with `FUN_005b67e0`, runs the optional
/// `FUN_005ae170` refresh, and reports success.  `None` covers both the
/// null-object and the not-accepted paths (both return `0`).
pub fn w13a04_apply_value(
    unit_present: bool,
    accepted: bool,
    value: u32,
    refresh: bool,
) -> Option<W13A04ValueUpdate> {
    if !unit_present || !accepted {
        return None;
    }
    Some(W13A04ValueUpdate { value, refresh })
}

// ---------------------------------------------------------------------------
// Ownership transfer (`0x00662d10`, continuation at `0x00662d65`)
// ---------------------------------------------------------------------------

/// The satellite updates planned by the ownership-transfer continuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13A04OwnerPlan {
    /// The resolved player slot written to `CUnit+0x4c`.
    pub player: u8,
    /// `FUN_005ad8a0()` and `FUN_005ac350()` both non-zero: the linked
    /// object also receives the player.
    pub assign_linked: bool,
    /// `FUN_005d5c80() != 0`: install the `FUN_00662e60` position callback.
    pub position_events: bool,
    /// `FUN_005b5890() != 0`: install the `FUN_00662e60` group callback.
    pub group_events: bool,
    /// `FUN_005b5810() != 0`: the driver block runs at all.
    pub driver: bool,
    /// `FUN_005b4980() != 0`: update the driver object.
    pub rider: bool,
    /// `FUN_005d2160() != 0`: update the `FUN_0058dee0()` cargo object.
    pub cargo: bool,
    /// `FUN_005b5f60()` and `FUN_005b4ba0()` both non-zero: update the
    /// related object.
    pub related: bool,
}

/// Outcome of the ownership guard chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13A04OwnerChange {
    /// The null-unit or out-of-range path, which reports error `0x57` and
    /// returns `0`.
    Invalid,
    /// One of the four blocking predicates or the same-owner check fired.
    Blocked,
    /// The player was accepted and the continuation runs.
    Applied(W13A04OwnerPlan),
}

/// `0x00662d10`: resolve the `0xd` selector to the local player.
///
/// Retail runs this substitution before the range check, so a selector of
/// `0xd` is allowed even though `0xd` is outside the player range.
pub fn w13a04_resolve_player_selector(selector: u16, local_player: u8) -> u16 {
    if selector == 0xd {
        local_player as u16
    } else {
        selector
    }
}

/// `0x00662d10`: the guard chain of the ownership-transfer routine.
///
/// The shipped body rejects a player slot `>= 0xc` and a null unit with
/// error `0x57`, then requires all four predicates (`FUN_005b5d00`,
/// `FUN_005ad9c0`, `FUN_005b5be0`, `FUN_005b57b0`) to be zero and the
/// current owner byte at `CUnit+0x4c` to differ from the new player.  The
/// four predicate results and the continuation inputs are supplied by the
/// caller so the state stays outside this helper.
#[allow(clippy::too_many_arguments)]
pub fn w13a04_plan_owner_change(
    unit_present: bool,
    player: u16,
    current_owner: u8,
    blocked_by: [bool; 4],
    linked_object: bool,
    position_events: bool,
    group_events: bool,
    driver: bool,
    rider: bool,
    cargo: bool,
    related: bool,
) -> W13A04OwnerChange {
    if !unit_present || player >= 0xc {
        return W13A04OwnerChange::Invalid;
    }
    if blocked_by.into_iter().any(|blocked| blocked) || current_owner as u16 == player {
        return W13A04OwnerChange::Blocked;
    }
    W13A04OwnerChange::Applied(w13a04_owner_change_propagation(
        player as u8,
        linked_object,
        position_events,
        group_events,
        driver,
        rider,
        cargo,
        related,
    ))
}

/// `0x00662d65`: the propagation half of the ownership-transfer routine.
///
/// Ghidra split this block off at a call it marked not-returning, but the
/// code is the continuation of the routine entered at `0x00662d10`.  The
/// unit itself always receives the new player (`FUN_005b5400`); the linked
/// object is updated only when both `FUN_005ad8a0()` and `FUN_005ac350()`
/// returned non-zero, and the callback/child updates are gated exactly as
/// recorded in [`W13A04OwnerPlan`].  The routine still reports success when
/// the driver gate is closed.
#[allow(clippy::too_many_arguments)]
pub fn w13a04_owner_change_propagation(
    player: u8,
    linked_object: bool,
    position_events: bool,
    group_events: bool,
    driver: bool,
    rider: bool,
    cargo: bool,
    related: bool,
) -> W13A04OwnerPlan {
    W13A04OwnerPlan {
        player,
        assign_linked: linked_object,
        position_events,
        group_events,
        driver,
        rider: driver && rider,
        cargo: driver && cargo,
        related: driver && related,
    }
}

// ---------------------------------------------------------------------------
// Callback forwarder (`0x00662e60`)
// ---------------------------------------------------------------------------

/// `0x00662e60`: forward the player word to the owner's setter.
///
/// Retail pushes `param_2`, calls `FUN_005b5400` on `param_1`, and returns
/// `0`.  The setter is supplied by the caller; the returned value is the
/// callback result, not the setter's.
pub fn w13a04_forward_player<T>(value: T, apply: impl FnOnce(T)) -> i32 {
    apply(value);
    0
}

// ---------------------------------------------------------------------------
// Order acceptance and overlay anchor (`0x006620a0`)
// ---------------------------------------------------------------------------

/// Requirement attached to a target-kind word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13A04KindRequirement {
    /// Word `0x11b`: no type-flag test.
    Always,
    /// Words `0x11c..=0x11e`: one of the class bits `0x08`, `0x10`, `0x20`.
    Flag(u8),
    /// Every other word: the unit type must equal the word.
    ExactType,
}

/// `0x006620a0`: map the order mode word at record `+0x14` to the action
/// byte.
///
/// The shipped body starts with `0x17` and replaces it with `0x06` for mode
/// `0`, `0x98` for mode `1` and `0x0e` for mode `2`; every other value keeps
/// `0x17`.
pub fn w13a04_order_mode_action(mode: u32) -> u8 {
    match mode {
        0 => 0x06,
        1 => 0x98,
        2 => 0x0e,
        _ => 0x17,
    }
}

/// `0x006620a0`: map the target-kind word to its flag requirement.
///
/// The 4-entry jump table maps `0x11b` straight to the acceptance label and
/// `0x11c`, `0x11d`, `0x11e` to tests of the type-flag bits `0x08`, `0x10`
/// and `0x20`; the default path compares the word with the unit type.
pub fn w13a04_kind_requirement(kind: u32) -> W13A04KindRequirement {
    match kind {
        0x11b => W13A04KindRequirement::Always,
        0x11c => W13A04KindRequirement::Flag(0x08),
        0x11d => W13A04KindRequirement::Flag(0x10),
        0x11e => W13A04KindRequirement::Flag(0x20),
        _ => W13A04KindRequirement::ExactType,
    }
}

/// `0x006620a0`: test a target-kind word against the unit.
///
/// `type_flags` is the byte at `0xf91fe8[unit_type]`; `unit_type` is the
/// `CUnit+0x64` word.  The jump-table cases accept only when the requested
/// class bit is set, while the default case requires exact type equality.
pub fn w13a04_target_kind_accepted(kind: u32, unit_type: u16, type_flags: u8) -> bool {
    match w13a04_kind_requirement(kind) {
        W13A04KindRequirement::Always => true,
        W13A04KindRequirement::Flag(mask) => type_flags & mask != 0,
        W13A04KindRequirement::ExactType => unit_type as u32 == kind,
    }
}

/// `0x006620a0`: order bytes that reject the candidate outright.
///
/// The shipped body tests the order/overlay byte at `CUnit+0x4d` for `0x47`
/// and `0x48` after `FUN_005b5820()` succeeds.
pub fn w13a04_order_byte_special(order_byte: u8) -> bool {
    order_byte == 0x47 || order_byte == 0x48
}

/// `0x006620a0`: order types rejected by the same routine.
///
/// The unit-type word is compared with `0x0e` (`FUN_005b4cd0` getter) before
/// the target-kind switch.
pub fn w13a04_order_type_excluded(unit_type: u16) -> bool {
    unit_type == 0x0e
}

/// `0x006620a0`: apply the overlay anchor for action kinds `6` and `0x0e`.
///
/// Retail only writes the two words into `CUnit+0x134` when the overlay is
/// present, its kind byte at `+0x8` is `1`, and the action byte is `0x06`
/// or `0x0e`.  The record words at `+0xc` and `+0x10` become the overlay
/// words at `+0x12` and `+0x14`.
pub fn w13a04_overlay_anchor(
    action_kind: u8,
    overlay_kind: Option<u8>,
    x: u16,
    y: u16,
) -> Option<(u16, u16)> {
    if overlay_kind == Some(1) && (action_kind == 0x06 || action_kind == 0x0e) {
        Some((x, y))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Placement arithmetic (`0x00662330`)
// ---------------------------------------------------------------------------

/// The six footprint words read from the per-type tables by `0x00662330`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13A04Footprint {
    /// `+0xf90fe0`: distance behind the origin on the first axis.
    pub back_x: i16,
    /// `+0xf90fe4`: distance ahead of the origin on the first axis.
    pub front_x: i16,
    /// `+0xf90fe2`: distance behind the origin on the second axis.
    pub back_y: i16,
    /// `+0xf90fe6`: distance ahead of the origin on the second axis.
    pub front_y: i16,
    /// `+0xf90ad0`: half extent on the first axis.
    pub half_x: i16,
    /// `+0xf90ad2`: half extent on the second axis.
    pub half_y: i16,
}

/// The recovered placement result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13A04Placement {
    /// Centred first coordinate, aligned to 32 pixels.
    pub x: u16,
    /// Centred second coordinate, aligned to 32 pixels.
    pub y: u16,
    /// First-axis tile delta passed to `FUN_00687240`.
    pub tile_dx: i32,
    /// Second-axis tile delta passed to `FUN_00687240`.
    pub tile_dy: i32,
}

/// `0x00662330`: fit one axis inside the map bounds.
///
/// The 16-bit comparison uses the table word at `back` and the word at
/// `front`: a negative `coord - back` shifts the origin forward to `back`
/// (`coord -= coord - back`), and `coord + front >= limit` shifts it back to
/// `limit - front - 1`.  The operations wrap in 16 bits in the shipped body.
pub fn w13a04_fit_origin(coord: u16, back: i16, front: i16, limit: u16) -> u16 {
    let behind = coord.wrapping_sub(back as u16) as i16;
    if behind < 0 {
        return coord.wrapping_sub(behind as u16);
    }
    let ahead = coord.wrapping_add(front as u16) as i16;
    if ahead as i32 >= limit as i32 {
        return (limit as i32 - ahead as i32 - 1 + coord as i32) as u16;
    }
    coord
}

/// `0x00662330`: centre one axis on a 32-pixel grid.
///
/// Retail subtracts the half extent, clamps into `0..limit-1`, clears the
/// low five bits (`and 0xffffffe0`) and adds the half extent back.  The
/// result is the anchored coordinate handed to the area effect.
pub fn w13a04_center_coordinate(coord: u16, half: i16, limit: u16) -> u16 {
    let mut value = coord as i32 - half as i32;
    if value < 0 {
        value = 0;
    } else if value >= limit as i32 {
        value = limit as i32 - 1;
    }
    value &= !0x1f;
    (value + half as i32) as u16
}

/// `0x00662330`: tile delta between a centred coordinate and a half extent.
///
/// The shipped body computes `(centred + sign) >> 5` for each operand, which
/// is a truncating division by 32, then subtracts the two tile values.
pub fn w13a04_centered_tile(centered: i32, half: i32) -> i32 {
    centered / 32 - half / 32
}

/// `0x00662330`: recover the full placement plan from the footprint words.
///
/// This is the pure front half of the routine; the shipped tail then calls
/// the terrain check `FUN_00687240`, the position setters and the effect
/// bookkeeping, none of which are modelled here.
pub fn w13a04_placement(
    footprint: W13A04Footprint,
    x: u16,
    y: u16,
    map_width: u16,
    map_height: u16,
) -> W13A04Placement {
    let fitted_x = w13a04_fit_origin(x, footprint.back_x, footprint.front_x, map_width);
    let fitted_y = w13a04_fit_origin(y, footprint.back_y, footprint.front_y, map_height);
    let centered_x = w13a04_center_coordinate(fitted_x, footprint.half_x, map_width);
    let centered_y = w13a04_center_coordinate(fitted_y, footprint.half_y, map_height);
    W13A04Placement {
        x: centered_x,
        y: centered_y,
        tile_dx: w13a04_centered_tile(centered_x as i32, footprint.half_x as i32),
        tile_dy: w13a04_centered_tile(centered_y as i32, footprint.half_y as i32),
    }
}

// ---------------------------------------------------------------------------
// Recursive effect walk (`0x00662800`)
// ---------------------------------------------------------------------------

/// The 16-byte effect context read by the walk at `0x00662800`.
///
/// Retail copies the first 16 bytes into a local, clears the `+0xc` word,
/// and reuses that copy for every recursive call.  The action code pointer
/// at `+0` is represented by the [`W13A04EffectTarget`] implementation; the
/// remaining words are data only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13A04EffectRecord {
    /// `+0x4`: first argument passed to the action callback.
    pub arg: u32,
    /// `+0x8`: second argument passed to the action callback.
    pub target: u16,
    /// `+0xa`: unit-link word tested by the head guard.
    pub link: u16,
    /// `+0xc`: non-zero arms the head guard for the first application.
    pub link_state: u32,
}

/// Caller-supplied view of one candidate unit for the effect walk.
///
/// The retail routine drives `CUnit` methods whose bodies live outside this
/// window; the trait keeps the recovered control flow testable without
/// adding state to the solver.
pub trait W13A04EffectTarget {
    /// `FUN_00664580(unit, link) != 0`.
    fn has_link(&self, link: u16) -> bool;
    /// Number of attached slots (`0xf92388[unit_type]`).
    fn attached_count(&self) -> usize;
    /// `FUN_005d5c30` / `FUN_005d5be0`: the attached child, when present.
    fn attached_child(&mut self, index: usize) -> Option<&mut Self>;
    /// `FUN_005b64c0` / `FUN_005b4be0`: the transport/child link.
    fn transport_child(&mut self) -> Option<&mut Self>;
    /// `FUN_005a3180`: the unit is already gone.
    fn is_gone(&self) -> bool;
    /// The record's action code pointer, called with `(arg, target)`.
    fn apply_action(&mut self, arg: u32, target: u16) -> bool;
    /// The recovered post-action cleanup branch.
    fn finish(&mut self);
}

/// `0x00662800`: apply an effect context to a unit and its attachments.
///
/// The recovered order is: the head guard (`link_state != 0` together with
/// `FUN_00664580`), a copy of the context with `link_state` cleared, the
/// attached-child loop, the transport child, the liveness check
/// (`FUN_005a3180`), the record's action callback, the post-action cleanup,
/// and finally one decrement of the pending-effect counter unless the
/// process-global nesting flag (`DAT_00fdfe08`) is set.  Because the flag is
/// global and the routine clears it after every recursive call, a nested
/// node that itself recursed also reaches its tail with the flag clear; the
/// shared `nested` flag reproduces that exactly.  Returns whether the
/// counter reached zero.
pub fn w13a04_apply_effect<T: W13A04EffectTarget>(
    target: &mut T,
    record: &W13A04EffectRecord,
    nested: &mut bool,
    counter: &mut u32,
) -> bool {
    if record.link_state != 0 && target.has_link(record.link) {
        return false;
    }
    let mut child_record = *record;
    child_record.link_state = 0;
    let attached = target.attached_count();
    for index in 0..attached {
        if let Some(child) = target.attached_child(index) {
            *nested = true;
            w13a04_apply_effect(child, &child_record, nested, counter);
            *nested = false;
        }
    }
    if let Some(child) = target.transport_child() {
        *nested = true;
        w13a04_apply_effect(child, &child_record, nested, counter);
        *nested = false;
    }
    if target.is_gone() {
        return false;
    }
    if !target.apply_action(record.arg, record.target) {
        return false;
    }
    target.finish();
    if !*nested {
        *counter = counter.wrapping_sub(1);
    }
    *counter == 0
}

// ---------------------------------------------------------------------------
// Target predicates (`0x00663960`, `0x006639b0`, `0x00663c80`, `0x00663cb0`)
// ---------------------------------------------------------------------------

/// `0x00663960`: the unit belongs to the requested player.
///
/// Retail loads the owner byte at `CUnit+0x4c`, zero-extends it and compares
/// it with the first argument (`sete`).
pub fn w13a04_same_owner(owner: u8, player: u32) -> bool {
    owner as u32 == player
}

/// `0x006639b0`: owner match plus type-flag bit `0x20`.
///
/// The cached body stops at the not-returning annotation, but the shipped
/// instructions test `0xf91fe8[type] & 0x20` and return `1` only when both
/// the owner matches and the bit is set.
pub fn w13a04_owner_has_flag_20(owner: u8, player: u32, type_flags: u8) -> bool {
    w13a04_same_owner(owner, player) && type_flags & 0x20 != 0
}

/// `0x00663c80`: owner match plus type-flag bit `0x08`.
///
/// Same shape as [`w13a04_owner_has_flag_20`] with the mask from the
/// `0x663960`-family predicate table.
pub fn w13a04_owner_has_flag_08(owner: u8, player: u32, type_flags: u8) -> bool {
    w13a04_same_owner(owner, player) && type_flags & 0x08 != 0
}

/// `0x00663cb0`: owner match plus exact unit-type match.
///
/// Retail compares the zero-extended `CUnit+0x64` word with the second
/// argument after the owner byte matches.
pub fn w13a04_owner_and_type(owner: u8, player: u32, unit_type: u16, wanted: u32) -> bool {
    w13a04_same_owner(owner, player) && unit_type as u32 == wanted
}

// ---------------------------------------------------------------------------
// Target-area effect dispatch (`0x006639e0`)
// ---------------------------------------------------------------------------

/// Predicate selected by the field word of the target-area effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13A04TargetPredicate {
    /// Field `0xe5`: owner match only.
    Owner,
    /// Field `0xe6`: owner match plus flag `0x08`.
    Flag08,
    /// Field `0xe7`: owner match plus flag `0x10`.
    Flag10,
    /// Field `0xe8`: owner match plus flag `0x20`.
    Flag20,
    /// Any other field: owner match plus exact unit type.
    ExactType,
}

/// One `0x14`-byte target-area record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13A04TargetArea {
    /// First corner word pair at `+0x0` / `+0x2`.
    pub first: (u16, u16),
    /// Second corner word pair at `+0x4` / `+0x6`.
    pub second: (u16, u16),
    /// Word at `+0x12`, copied into the effect context at `+0xa`.
    pub link: u16,
}

/// `0x006639e0`: map the field word to the predicate applied to each
/// candidate unit.
///
/// The 5-entry jump table selects the owner-only predicate for `0xe5`, then
/// the three class-flag predicates for `0xe6`, `0xe7` and `0xe8`, and the
/// exact-type predicate for every other word.
pub fn w13a04_target_predicate(field: u16) -> W13A04TargetPredicate {
    match field {
        0x00e5 => W13A04TargetPredicate::Owner,
        0x00e6 => W13A04TargetPredicate::Flag08,
        0x00e7 => W13A04TargetPredicate::Flag10,
        0x00e8 => W13A04TargetPredicate::Flag20,
        _ => W13A04TargetPredicate::ExactType,
    }
}

/// `0x006639e0`: evaluate the selected predicate for one candidate unit.
pub fn w13a04_predicate_matches(
    predicate: W13A04TargetPredicate,
    owner: u8,
    player: u32,
    unit_type: u16,
    type_flags: u8,
    wanted: u32,
) -> bool {
    match predicate {
        W13A04TargetPredicate::Owner => w13a04_same_owner(owner, player),
        W13A04TargetPredicate::Flag08 => w13a04_owner_has_flag_08(owner, player, type_flags),
        W13A04TargetPredicate::Flag10 => w13a04_same_owner(owner, player) && type_flags & 0x10 != 0,
        W13A04TargetPredicate::Flag20 => w13a04_owner_has_flag_20(owner, player, type_flags),
        W13A04TargetPredicate::ExactType => w13a04_owner_and_type(owner, player, unit_type, wanted),
    }
}

/// `0x006639e0`: read one target-area record from the table at `+0xeb70`.
///
/// Each record is five 32-bit words (`0x14` bytes); only words `0..3` and
/// the link word at `+0x12` are used by the effect builder.  Returns `None`
/// when `index` is outside the supplied table.
pub fn w13a04_target_area(table: &[u16], index: u16) -> Option<W13A04TargetArea> {
    let base = index as usize * W13A04_TARGET_AREA_WORDS;
    let record = table.get(base..base + W13A04_TARGET_AREA_WORDS)?;
    Some(W13A04TargetArea {
        first: (record[0], record[1]),
        second: (record[2], record[3]),
        link: record[9],
    })
}

/// `0x006639e0`: whether the index takes the scheduled-table path.
///
/// Retail uses the signed compare `index >= 0xff` to choose the wide
/// (unit-list iteration) path, so every value below `0xff` takes the
/// target-area table path.
pub fn w13a04_target_index_is_table(index: i32) -> bool {
    index < 0xff
}

// ---------------------------------------------------------------------------
// Threshold comparisons (`0x00663d30`, `0x00663e8b`)
// ---------------------------------------------------------------------------

/// `0x00663d30`: compare a freshly looked-up value with a record threshold.
///
/// The mode byte at `+0xe` selects the recovered relation:
/// `0` = `threshold <= computed`, `1` = `computed <= threshold`,
/// `0xa` = equality; every other mode is false.  The shipped body gets the
/// computed value from `FUN_00665ad0(handle, word, mode != 1)`; that lookup
/// is supplied by the caller.
pub fn w13a04_compare_threshold(computed: u32, threshold: u32, mode: u8) -> bool {
    match mode {
        0 => threshold <= computed,
        1 => computed <= threshold,
        0x0a => computed == threshold,
        _ => false,
    }
}

/// `0x00663e8b`: same threshold relation using the cached-value getter.
///
/// This routine reads the record at `+4` (handle), `+8` (threshold) and
/// `+0xe` (mode) and calls `FUN_00664db0` for the computed value.  The
/// relation and the default-false mode are identical to
/// [`w13a04_compare_threshold`].
pub fn w13a04_compare_threshold_cached(cached_value: u32, threshold: u32, mode: u8) -> bool {
    w13a04_compare_threshold(cached_value, threshold, mode)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_wave13_agent04 {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn selector_routes_cover_every_branch() {
        assert_eq!(w13a04_selector_route(0), W13A04SelectorRoute::Player(0));
        assert_eq!(w13a04_selector_route(0xc), W13A04SelectorRoute::Player(0xc));
        assert_eq!(w13a04_selector_route(0xd), W13A04SelectorRoute::LocalPlayer);
        assert_eq!(
            w13a04_selector_route(0xe),
            W13A04SelectorRoute::RelationZero
        );
        assert_eq!(w13a04_selector_route(0xf), W13A04SelectorRoute::RelationTwo);
        assert_eq!(
            w13a04_selector_route(0x10),
            W13A04SelectorRoute::RelationOne
        );
        assert_eq!(w13a04_selector_route(0x11), W13A04SelectorRoute::GroupSlots);
        assert_eq!(w13a04_selector_route(0x12), W13A04SelectorRoute::Team(1));
        assert_eq!(w13a04_selector_route(0x15), W13A04SelectorRoute::Team(4));
        assert_eq!(w13a04_selector_route(0x16), W13A04SelectorRoute::None);
        assert_eq!(w13a04_selector_route(0x19), W13A04SelectorRoute::None);
        assert_eq!(
            w13a04_selector_route(0x1a),
            W13A04SelectorRoute::RelationNotTwo
        );
        assert_eq!(w13a04_selector_route(0x1b), W13A04SelectorRoute::OutOfRange);
    }

    #[test]
    fn set_amount_writes_slots_and_matrix() {
        let mut amounts = W13A04Amounts::new();
        assert_eq!(
            w13a04_set_amount(&mut amounts, 3, 0x00e6, 17, false),
            W13A04StoreOutcome::Stored
        );
        assert_eq!(amounts.slot_0e6[3], 17);
        assert_eq!(
            w13a04_set_amount(&mut amounts, 7, 0x00e7, 23, false),
            W13A04StoreOutcome::Stored
        );
        assert_eq!(amounts.slot_0e7[7], 23);
        assert_eq!(
            w13a04_set_amount(&mut amounts, 0, 0x00e8, 29, false),
            W13A04StoreOutcome::Stored
        );
        assert_eq!(amounts.slot_0e8[0], 29);
        assert_eq!(
            w13a04_set_amount(&mut amounts, 2, 0x00e3, 41, false),
            W13A04StoreOutcome::Stored
        );
        assert_eq!(amounts.order(0x00e3, 2), Some(41));
    }

    #[test]
    fn set_amount_boundaries_and_delegation() {
        let mut amounts = W13A04Amounts::new();
        // Player 8 is the first rejected slot for all special fields.
        assert_eq!(
            w13a04_set_amount(&mut amounts, 8, 0x00e6, 1, false),
            W13A04StoreOutcome::Ignored
        );
        // Field 0xe5 is a no-op, 0xe4 is the first rejected order field.
        assert_eq!(
            w13a04_set_amount(&mut amounts, 0, 0x00e5, 1, false),
            W13A04StoreOutcome::Ignored
        );
        assert_eq!(
            w13a04_set_amount(&mut amounts, 0, 0x00e4, 1, false),
            W13A04StoreOutcome::Ignored
        );
        // The delegation flag is consulted before the matrix range checks.
        assert_eq!(
            w13a04_set_amount(&mut amounts, 0xff, 0x00e4, 1, true),
            W13A04StoreOutcome::Delegated(0)
        );
        assert_eq!(
            w13a04_set_amount(&mut amounts, 0, 0x00e6, 1, true),
            W13A04StoreOutcome::Stored
        );
    }

    #[test]
    fn subtract_amount_clamps_and_delegates() {
        let mut amounts = W13A04Amounts::new();
        amounts.slot_0e6[1] = 100;
        amounts.orders[0x0010 * W13A04_ORDER_STRIDE + 1] = 30;
        assert_eq!(
            w13a04_subtract_amount(&mut amounts, 1, 0x00e6, 30, false),
            W13A04StoreOutcome::Stored
        );
        assert_eq!(amounts.slot_0e6[1], 70);
        assert_eq!(
            w13a04_subtract_amount(&mut amounts, 1, 0x0010, 30, false),
            W13A04StoreOutcome::Stored
        );
        assert_eq!(amounts.order(0x0010, 1), Some(0));
        assert_eq!(
            w13a04_subtract_amount(&mut amounts, 1, 0x00e6, 1000, false),
            W13A04StoreOutcome::Stored
        );
        assert_eq!(amounts.slot_0e6[1], 0);
        assert_eq!(
            w13a04_subtract_amount(&mut amounts, 0, 0x00e4, 1, true),
            W13A04StoreOutcome::Delegated(-1)
        );
        assert_eq!(
            w13a04_subtract_amount(&mut amounts, 8, 0x00e8, 1, false),
            W13A04StoreOutcome::Ignored
        );
        assert_eq!(
            w13a04_subtract_amount(&mut amounts, 0, 0x00e5, 1, false),
            W13A04StoreOutcome::Ignored
        );
    }

    #[test]
    fn percentage_scaling_matches_the_reciprocal_divide() {
        assert_eq!(w13a04_percent_of(2000, 50), 1000);
        assert_eq!(w13a04_percent_of(1, 99), 0);
        assert_eq!(w13a04_percent_of(0, 100), 0);
        // 32-bit wrapping product: 0xffff_ffff * 100 mod 2^32 = 0xffff_ff9c.
        assert_eq!(w13a04_percent_of(0xffff_ffff, 100), 42_949_671);
        assert_eq!(w13a04_scale_percentage(true, true, 2000, 50), Some(1000));
        assert_eq!(w13a04_scale_percentage(false, true, 2000, 50), None);
        assert_eq!(w13a04_scale_percentage(true, false, 2000, 50), None);
        assert_eq!(
            w13a04_scale_percentage_active(true, true, 2000, 25),
            Some(500)
        );
        assert_eq!(w13a04_scale_percentage_active(true, false, 2000, 25), None);
        assert_eq!(w13a04_scale_percentage_active(false, true, 2000, 25), None);
        assert_eq!(w13a04_percent_at_least_one(10, 1), 1);
        assert_eq!(w13a04_percent_at_least_one(200, 50), 100);
    }

    #[test]
    fn value_application_reports_refresh() {
        assert_eq!(
            w13a04_apply_value(true, true, 0x1234, true),
            Some(W13A04ValueUpdate {
                value: 0x1234,
                refresh: true
            })
        );
        assert_eq!(
            w13a04_apply_value(true, true, 0x1234, false),
            Some(W13A04ValueUpdate {
                value: 0x1234,
                refresh: false
            })
        );
        assert_eq!(w13a04_apply_value(false, true, 1, true), None);
        assert_eq!(w13a04_apply_value(true, false, 1, true), None);
    }

    #[test]
    fn owner_change_guards_and_propagation() {
        assert_eq!(w13a04_resolve_player_selector(0xd, 4), 4);
        assert_eq!(w13a04_resolve_player_selector(6, 4), 6);

        assert_eq!(
            w13a04_plan_owner_change(
                false, 1, 0, [false; 4], false, false, false, false, false, false, false
            ),
            W13A04OwnerChange::Invalid
        );
        assert_eq!(
            w13a04_plan_owner_change(
                true, 0xc, 0, [false; 4], false, false, false, false, false, false, false
            ),
            W13A04OwnerChange::Invalid
        );
        assert_eq!(
            w13a04_plan_owner_change(
                true, 1, 1, [false; 4], false, false, false, false, false, false, false
            ),
            W13A04OwnerChange::Blocked
        );
        for index in 0..4 {
            let mut blocked = [false; 4];
            blocked[index] = true;
            assert_eq!(
                w13a04_plan_owner_change(
                    true, 1, 0, blocked, false, false, false, false, false, false, false
                ),
                W13A04OwnerChange::Blocked
            );
        }
        let plan = w13a04_owner_change_propagation(3, true, true, false, true, true, false, true);
        assert_eq!(
            plan,
            W13A04OwnerPlan {
                player: 3,
                assign_linked: true,
                position_events: true,
                group_events: false,
                driver: true,
                rider: true,
                cargo: false,
                related: true,
            }
        );
        let plan = w13a04_owner_change_propagation(2, false, false, true, false, true, true, true);
        assert_eq!(
            plan,
            W13A04OwnerPlan {
                player: 2,
                assign_linked: false,
                position_events: false,
                group_events: true,
                driver: false,
                rider: false,
                cargo: false,
                related: false,
            }
        );
    }

    #[test]
    fn forwarder_returns_zero_after_applying() {
        let seen = Cell::new(0u32);
        let result = w13a04_forward_player(0x42, |value| seen.set(value));
        assert_eq!(result, 0);
        assert_eq!(seen.get(), 0x42);
    }

    #[test]
    fn order_mode_and_kind_requirements() {
        assert_eq!(w13a04_order_mode_action(0), 0x06);
        assert_eq!(w13a04_order_mode_action(1), 0x98);
        assert_eq!(w13a04_order_mode_action(2), 0x0e);
        assert_eq!(w13a04_order_mode_action(3), 0x17);
        assert_eq!(w13a04_order_mode_action(0xffff_ffff), 0x17);

        assert_eq!(
            w13a04_kind_requirement(0x11b),
            W13A04KindRequirement::Always
        );
        assert_eq!(
            w13a04_kind_requirement(0x11c),
            W13A04KindRequirement::Flag(0x08)
        );
        assert_eq!(
            w13a04_kind_requirement(0x11d),
            W13A04KindRequirement::Flag(0x10)
        );
        assert_eq!(
            w13a04_kind_requirement(0x11e),
            W13A04KindRequirement::Flag(0x20)
        );
        assert_eq!(
            w13a04_kind_requirement(0x11f),
            W13A04KindRequirement::ExactType
        );

        assert!(w13a04_target_kind_accepted(0x11b, 0, 0));
        assert!(!w13a04_target_kind_accepted(0x11c, 0, 0x04));
        assert!(w13a04_target_kind_accepted(0x11c, 0, 0x08));
        assert!(w13a04_target_kind_accepted(0x11d, 0, 0x10));
        assert!(w13a04_target_kind_accepted(0x11e, 0, 0x20));
        assert!(!w13a04_target_kind_accepted(0x200, 0x1ff, 0xff));
        assert!(w13a04_target_kind_accepted(0x200, 0x200, 0));
        assert!(w13a04_order_byte_special(0x47));
        assert!(w13a04_order_byte_special(0x48));
        assert!(!w13a04_order_byte_special(0x46));
        assert!(w13a04_order_type_excluded(0x0e));
        assert!(!w13a04_order_type_excluded(0x0d));
    }

    #[test]
    fn overlay_anchor_requires_kind_one_and_action_6_or_e() {
        assert_eq!(
            w13a04_overlay_anchor(0x06, Some(1), 0x10, 0x20),
            Some((0x10, 0x20))
        );
        assert_eq!(
            w13a04_overlay_anchor(0x0e, Some(1), 0x10, 0x20),
            Some((0x10, 0x20))
        );
        assert_eq!(w13a04_overlay_anchor(0x98, Some(1), 0x10, 0x20), None);
        assert_eq!(w13a04_overlay_anchor(0x17, Some(1), 0x10, 0x20), None);
        assert_eq!(w13a04_overlay_anchor(0x06, Some(2), 0x10, 0x20), None);
        assert_eq!(w13a04_overlay_anchor(0x06, None, 0x10, 0x20), None);
    }

    #[test]
    fn placement_fits_and_centres_on_the_grid() {
        let footprint = W13A04Footprint {
            back_x: 8,
            front_x: 8,
            back_y: 12,
            front_y: 12,
            half_x: 8,
            half_y: 8,
        };
        // Left/top clamp pushes the origin to the back extent.
        assert_eq!(w13a04_fit_origin(2, 8, 8, 64), 8);
        assert_eq!(w13a04_fit_origin(2, 12, 12, 64), 12);
        // Right/bottom clamp: limit - front - 1.
        assert_eq!(w13a04_fit_origin(60, 8, 8, 64), 55);
        assert_eq!(w13a04_fit_origin(60, 12, 12, 64), 51);
        // Interior coordinates are untouched.
        assert_eq!(w13a04_fit_origin(30, 8, 8, 64), 30);

        assert_eq!(w13a04_center_coordinate(8, 8, 64), 8);
        assert_eq!(w13a04_center_coordinate(55, 8, 64), 40);
        assert_eq!(w13a04_center_coordinate(0, 8, 64), 8);
        assert_eq!(w13a04_center_coordinate(64, 8, 64), 40);

        assert_eq!(w13a04_centered_tile(40, 8), 1);
        assert_eq!(w13a04_centered_tile(8, 8), 0);
        assert_eq!(w13a04_centered_tile(-1, 8), 0);
        assert_eq!(w13a04_centered_tile(-33, 8), -1);
        assert_eq!(w13a04_centered_tile(-64, 32), -3);

        let plan = w13a04_placement(footprint, 2, 60, 64, 64);
        assert_eq!(plan.x, 8);
        assert_eq!(plan.y, w13a04_center_coordinate(51, 8, 64));
        assert_eq!(plan.tile_dx, 0);
        assert_eq!(plan.tile_dy, w13a04_centered_tile(plan.y as i32, 8));
    }

    #[test]
    fn owner_predicates_cover_both_bits_and_type_match() {
        assert!(w13a04_same_owner(3, 3));
        assert!(!w13a04_same_owner(3, 4));
        assert!(w13a04_owner_has_flag_20(3, 3, 0x20));
        assert!(!w13a04_owner_has_flag_20(3, 3, 0x08));
        assert!(!w13a04_owner_has_flag_20(4, 3, 0x20));
        assert!(w13a04_owner_has_flag_08(3, 3, 0x08));
        assert!(!w13a04_owner_has_flag_08(3, 3, 0x10));
        assert!(w13a04_owner_and_type(3, 3, 0x21, 0x21));
        assert!(!w13a04_owner_and_type(3, 3, 0x21, 0x22));
        assert!(!w13a04_owner_and_type(4, 3, 0x21, 0x21));
    }

    #[test]
    fn target_predicates_and_area_records() {
        assert_eq!(
            w13a04_target_predicate(0x00e5),
            W13A04TargetPredicate::Owner
        );
        assert_eq!(
            w13a04_target_predicate(0x00e6),
            W13A04TargetPredicate::Flag08
        );
        assert_eq!(
            w13a04_target_predicate(0x00e7),
            W13A04TargetPredicate::Flag10
        );
        assert_eq!(
            w13a04_target_predicate(0x00e8),
            W13A04TargetPredicate::Flag20
        );
        assert_eq!(
            w13a04_target_predicate(0x0100),
            W13A04TargetPredicate::ExactType
        );
        assert!(w13a04_predicate_matches(
            W13A04TargetPredicate::Owner,
            2,
            2,
            0,
            0,
            0
        ));
        assert!(w13a04_predicate_matches(
            W13A04TargetPredicate::Flag08,
            2,
            2,
            0,
            0x08,
            0
        ));
        assert!(w13a04_predicate_matches(
            W13A04TargetPredicate::Flag10,
            2,
            2,
            0,
            0x10,
            0
        ));
        assert!(w13a04_predicate_matches(
            W13A04TargetPredicate::Flag20,
            2,
            2,
            0,
            0x20,
            0
        ));
        assert!(w13a04_predicate_matches(
            W13A04TargetPredicate::ExactType,
            2,
            2,
            0x55,
            0,
            0x55
        ));
        assert!(!w13a04_predicate_matches(
            W13A04TargetPredicate::ExactType,
            2,
            2,
            0x55,
            0,
            0x56
        ));

        let mut table = vec![0u16; W13A04_TARGET_AREA_WORDS * 2];
        table[..W13A04_TARGET_AREA_WORDS]
            .copy_from_slice(&[0x11, 0x22, 0x33, 0x44, 0, 0, 0, 0, 0, 0x55]);
        assert_eq!(
            w13a04_target_area(&table, 0),
            Some(W13A04TargetArea {
                first: (0x11, 0x22),
                second: (0x33, 0x44),
                link: 0x55,
            })
        );
        assert_eq!(
            w13a04_target_area(&table, 1),
            Some(W13A04TargetArea::default())
        );
        assert_eq!(w13a04_target_area(&table, 2), None);
        assert!(w13a04_target_index_is_table(0));
        assert!(w13a04_target_index_is_table(0xfe));
        assert!(w13a04_target_index_is_table(-1));
        assert!(!w13a04_target_index_is_table(0xff));
    }

    #[test]
    fn recovered_layout_constants() {
        assert_eq!(W13A04_IMAGE_KEY, 0x10fa_e560);
        assert_eq!(W13A04_SLOT_0E6, 0x2d84);
        assert_eq!(W13A04_SLOT_0E7, 0x2ed4);
        assert_eq!(W13A04_SLOT_0E8, 0x2ff4);
        assert_eq!(W13A04_ORDER_TABLE, 0xb274);
        assert_eq!(W13A04_TARGET_AREAS, 0xeb70);
        assert_eq!(W13A04_TARGET_AREA_WORDS, 10);
        assert_eq!(
            W13A04_ORDER_FIELDS * W13A04_ORDER_STRIDE,
            W13A04Amounts::new().orders.len()
        );
    }

    #[test]
    fn threshold_modes_match_the_recovered_relations() {
        // Mode 0: threshold <= computed.
        assert!(w13a04_compare_threshold(10, 10, 0));
        assert!(w13a04_compare_threshold(11, 10, 0));
        assert!(!w13a04_compare_threshold(9, 10, 0));
        // Mode 1: computed <= threshold.
        assert!(w13a04_compare_threshold(10, 10, 1));
        assert!(w13a04_compare_threshold(9, 10, 1));
        assert!(!w13a04_compare_threshold(11, 10, 1));
        // Mode 10: equality.
        assert!(w13a04_compare_threshold(10, 10, 0x0a));
        assert!(!w13a04_compare_threshold(11, 10, 0x0a));
        // Every other mode is false.
        assert!(!w13a04_compare_threshold(10, 10, 2));
        assert!(!w13a04_compare_threshold(10, 10, 0xff));
        // The cached variant is the same relation.
        assert!(w13a04_compare_threshold_cached(10, 10, 0x0a));
        assert!(!w13a04_compare_threshold_cached(9, 10, 0));
    }

    /// Small tree used by the effect-walk test.
    struct MockEffectNode {
        linked: bool,
        attached: Vec<Option<MockEffectNode>>,
        transport: Option<Box<MockEffectNode>>,
        gone: bool,
        applied: Vec<(u32, u16)>,
        finished: bool,
    }

    impl MockEffectNode {
        fn leaf() -> Self {
            Self {
                linked: false,
                attached: Vec::new(),
                transport: None,
                gone: false,
                applied: Vec::new(),
                finished: false,
            }
        }

        fn with_attached(attached: Vec<Option<MockEffectNode>>) -> Self {
            let mut node = Self::leaf();
            node.attached = attached;
            node
        }
    }

    impl W13A04EffectTarget for MockEffectNode {
        fn has_link(&self, _link: u16) -> bool {
            self.linked
        }

        fn attached_count(&self) -> usize {
            self.attached.len()
        }

        fn attached_child(&mut self, index: usize) -> Option<&mut Self> {
            self.attached.get_mut(index).and_then(Option::as_mut)
        }

        fn transport_child(&mut self) -> Option<&mut Self> {
            self.transport.as_deref_mut()
        }

        fn is_gone(&self) -> bool {
            self.gone
        }

        fn apply_action(&mut self, arg: u32, target: u16) -> bool {
            self.applied.push((arg, target));
            true
        }

        fn finish(&mut self) {
            self.finished = true;
        }
    }

    #[test]
    fn recursive_effect_walk_applies_to_attachments_and_transport() {
        let mut top = MockEffectNode::with_attached(vec![
            Some(MockEffectNode::leaf()),
            None,
            Some(MockEffectNode::leaf()),
        ]);
        top.transport = Some(Box::new(MockEffectNode::leaf()));
        let record = W13A04EffectRecord {
            arg: 7,
            target: 0x33,
            link: 0,
            link_state: 0,
        };
        let mut counter = 1;
        let mut nested = false;
        assert!(w13a04_apply_effect(
            &mut top,
            &record,
            &mut nested,
            &mut counter
        ));
        assert_eq!(counter, 0);
        assert_eq!(top.applied, vec![(7, 0x33)]);
        assert!(top.finished);
        for index in [0usize, 2] {
            let child = top.attached[index].as_ref().unwrap();
            assert_eq!(child.applied, vec![(7, 0x33)]);
            assert!(child.finished);
        }
        let transport = top.transport.as_ref().unwrap();
        assert_eq!(transport.applied, vec![(7, 0x33)]);

        // A nested leaf must not decrement the counter a second time.
        let mut child = MockEffectNode::leaf();
        let mut counter = 1;
        let mut nested = true;
        assert!(!w13a04_apply_effect(
            &mut child,
            &record,
            &mut nested,
            &mut counter
        ));
        assert_eq!(counter, 1);

        // A nested node that itself recursed clears the global flag, so its
        // own tail decrements the counter again (the retail quirk).
        let inner = MockEffectNode::with_attached(vec![Some(MockEffectNode::leaf())]);
        let mut outer = MockEffectNode::with_attached(vec![Some(inner)]);
        let mut counter = 2;
        let mut nested = false;
        assert!(w13a04_apply_effect(
            &mut outer,
            &record,
            &mut nested,
            &mut counter
        ));
        assert_eq!(counter, 0);
    }

    #[test]
    fn recursive_effect_walk_head_guard_and_gone_unit() {
        let mut linked = MockEffectNode::leaf();
        linked.linked = true;
        let record = W13A04EffectRecord {
            arg: 1,
            target: 2,
            link: 3,
            link_state: 1,
        };
        let mut counter = 5;
        let mut nested = false;
        assert!(!w13a04_apply_effect(
            &mut linked,
            &record,
            &mut nested,
            &mut counter
        ));
        assert_eq!(counter, 5);
        assert!(linked.applied.is_empty());

        let mut gone = MockEffectNode::leaf();
        gone.gone = true;
        let record = W13A04EffectRecord {
            link_state: 0,
            ..record
        };
        assert!(!w13a04_apply_effect(
            &mut gone,
            &record,
            &mut nested,
            &mut counter
        ));
        assert_eq!(counter, 5);
        assert!(gone.applied.is_empty());
        assert!(!gone.finished);
    }
}
