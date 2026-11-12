#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave12/agent02.rs
// ===========================================================================
// Wave 12 translation batch 02: client/UI/runtime helpers in
// `0x00654000..=0x00655fff`.
//
// The ledger lists 43 rows in this window: 40 `decompiled_uncited` bodies,
// two already-translated routines (`FUN_00655140`, `FUN_00655dd0`) and the
// `FUN_00655ab0` thunk.  Fifteen of the 40 uncited rows are translated below;
// the other 25 are listed under [Skipped rows](#skipped-rows) with the reason
// each one is not modelled.
//
// The block is client/UI/runtime code: the per-player `0x00fd2d0c` row table,
// its placement predicates, the membership chain a row keeps for its units,
// the region tile queries and the placement-class grid.  Nothing here touches
// [`crate::sim::World`] or [`crate::sim::SimUnit`], so every translation is a
// free function over values or slices.  Calls whose target leaves the window
// enter as caller-supplied values or closures; no helper outside this file is
// assumed to exist.
//
// ## Recovered layouts
//
// | location | layout |
// |----------|--------|
// | `0x00fd2d0c` | eight `dword` per-player row base pointers (`0x20` bytes) |
// | row | `0x34` bytes: `+0x00` region, `+0x02` related row, `+0x04` owner, `+0x05` state, `+0x06` step, `+0x08` flags, `+0x09` substate, `+0x0a` member count, `+0x0c`/`+0x0e` extent words, `+0x1c`/`+0x20` placement markers, `+0x28` target word, `+0x2c` member pool object, `+0x30` member chain head |
// | member node | `+0x00` next, `+0x04` previous, `+0x08` kind (`4` = member), `+0x0c` unit, `+0x10` row |
// | member pool | free chain head at `pool+0x4e20` (the `20000` of `0x00654420`) |
// | unit | `+0x0c` sprite, `+0x4c` owner, `+0x4d` visibility/order byte, `+0x64` type id, `+0x134` member node, `+0x138`/`+0x13a` footprint words, `+0xdc` status bits (bit 2 airborne, bit 5 building gate) |
// | region record | `0x40` bytes: `+0x02` neighbour count, `+0x08` neighbour list, `+0x0c`/`+0x10` pixel origin words |
//
// ## Skipped rows
//
// * `FUN_00654050` - order-`0x9f` retarget: the row re-selection is translatable
//   but both tails walk the owner's unit list into the non-returning
//   `FUN_005b4cd0`, so the routine has no return value to model.
// * `FUN_0065411c`, `FUN_00654d4c`, `FUN_00654e4a`, `FUN_00654ec7`, `FUN_00654f14`,
//   `FUN_00654fae`, `FUN_00654fbc`, `FUN_00654fe6`, `FUN_00655c1c` - jump-table
//   fragments: every argument (including the `AX` switch tag) and every
//   local lives in `unaff_EBX`/`unaff_EBP`/`unaff_ESI`, and the surviving
//   body is a tail call into `FUN_005b7d00`, `FUN_005b7e30`, `FUN_005b4cd0` or
//   `FUN_0058b650`; no routine-level contract survives.
// * `FUN_00654206` - adjusts a stale counter through a base pointer held in
//   `unaff_EBX` and then takes the non-returning `FUN_0058b640`; it has no
//   arguments, no return value and no recovered base.
// * `FUN_006542f7`, `FUN_0065431d` - one-line dispatchers that forward the
//   unrecovered `in_EAX`/`unaff_ESI` words to `FUN_005b7d80` with a fixed id
//   (`0xa3` and `0x92`); the payload is not a recovered argument, so only the
//   constant id could be modelled.
// * `FUN_00654598` - a three-byte `ret`; an empty body, not a routine.
// * `FUN_00654670` - unit merge: address arithmetic and two opaque target
//   acquisitions (`FUN_00652250`, `FUN_006520e0`) whose record kind decides
//   between three teardown paths, one of which is non-returning.
// * `FUN_00654895` - duplicate tail of the `0x00654838` loop (same
//   `FUN_005b5b90` filter, same `max(+0x138, +0x13a)` accumulation); translated
//   once through [`w12a02_rect_extent_sum`].
// * `FUN_006548d3` - 17 bytes that only call the list destructor
//   `FUN_005a5770`; no value semantics.
// * `FUN_006548f0` - unconditional non-returning `FUN_00748d40(unit+0x4c)`.
// * `FUN_00654917`, `FUN_00654a55` - the same 300-byte order-branch fragment; the
//   acting unit, the command byte and the stack frame all come from
//   `unaff_EBX`/`unaff_EBP`, and only one branch per command is reachable.
// * `FUN_00655050` - stores `0` to `0x00fd2d34` and then takes the
//   non-returning `FUN_0058b650`; no arguments and no return.
// * `FUN_006551a0` - runtime initialiser: a stack buffer, two `thiscall`
//   initialisers and the non-returning startup helper `FUN_00c8a080`.
// * `FUN_00655306`, `0x0065534c` share one loop; the loop itself is translated
//   as [`w12a02_expand_placement_classes`] (cited at `0x0065534c`) and the
//   surrounding commit (`FUN_00658240` placement passes, `FUN_00635f80` validity
//   list) is client-only and not modelled.  `FUN_00655306` is not cited because
//   its remaining body is that same loop plus the commit.
// * `FUN_006558c0` - validates an owner's unit list and then calls the
//   non-returning `FUN_005b4cd0`; the only value is the constant `0` of the
//   empty-list path.
// * `FUN_00655ac0` - building placement sweep: iterates rows, decodes command
//   tables and performs the placement passes; it ends in
//   `FUN_00748d40`/`FUN_00c86b6e` and the per-row helpers are the untranslated
//   `FUN_00634f10`/`FUN_00634e70` writers.
// * `0x00655e10` is partially translated: the bucket pick and the placement
//   assertion are modelled, the coordinate write and the trailing
//   non-returning dispatch are caller-side.

// ---------------------------------------------------------------------------
// Shared row view
// ---------------------------------------------------------------------------

/// Size of one row of the per-player table at `0x00fd2d0c` (`0x34` bytes).
pub const W12A02_ROW_BYTES: usize = 0x34;

/// Flag bit `0x20` of the row flags byte (`+0x08`): the row is claimed and no
/// longer counts as clear (`0x006543b0`, `0x00654770`, `0x006547b0`).
pub const W12A02_ROW_CLAIMED_FLAG: u8 = 0x20;

/// Flag bit written by `0x00655e10` when it hands out a placement spot
/// (`row+0x08 |= 4`).
pub const W12A02_ROW_SPOT_TAKEN_FLAG: u8 = 4;

/// Node kind that `0x00654420` writes to `node+0x08`.
pub const W12A02_NODE_KIND_MEMBER: u8 = 4;

/// One row of the per-player table behind `0x00fd2d0c`.
///
/// Only the fields read or written by this window are modelled; the remaining
/// bytes of the `0x34`-byte row are padding.  Byte offsets are the shipped
/// ones, and [`w12a02_row_from_bytes`] decodes them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12a02Row {
    /// `+0x00`: region index of the row's anchor (the input of the
    /// region-origin lookup used by the coordinate readers).
    pub region: u16,
    /// `+0x02`: the related row that `FUN_00654670` addresses.
    pub related_row: u16,
    /// `+0x04`: owner slot.
    pub owner: u8,
    /// `+0x05`: state code.  `0` is empty, `3` is the code the placement
    /// predicates reject and the code `0x00654420` re-targets, `4`/`5` are the
    /// two placement states and `8`/`9` complete the eligible set.
    pub state: u8,
    /// `+0x06`: step/animation byte.
    pub step: u8,
    /// `+0x08`: flags; bit `0x20` is [`W12A02_ROW_CLAIMED_FLAG`] and bit `4`
    /// is [`W12A02_ROW_SPOT_TAKEN_FLAG`].
    pub flags: u8,
    /// `+0x09`: substate byte.
    pub substate: u8,
    /// `+0x0a`: number of members linked into the row's chain.
    pub count: u16,
    /// `+0x0c`: first accumulated extent word.
    pub extent_a: u16,
    /// `+0x0e`: second accumulated extent word.
    pub extent_b: u16,
    /// `+0x1c`: first placement marker; non-zero means "not clear".
    pub marker_1c: u32,
    /// `+0x20`: second placement marker; non-zero means "not clear".
    pub marker_20: u32,
    /// `+0x28`: target word tested before the order-`0x9f` dispatch of
    /// `FUN_00654050`.
    pub target_28: u32,
}

fn w12a02_read_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn w12a02_read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Decode one `0x34`-byte row into [`W12a02Row`].
///
/// The offsets are the ones every routine in this window uses; the helper
/// exists so the byte-level layout can be asserted in a test instead of only
/// being described in prose.
pub fn w12a02_row_from_bytes(bytes: &[u8; W12A02_ROW_BYTES]) -> W12a02Row {
    W12a02Row {
        region: w12a02_read_u16(bytes, 0x00),
        related_row: w12a02_read_u16(bytes, 0x02),
        owner: bytes[0x04],
        state: bytes[0x05],
        step: bytes[0x06],
        flags: bytes[0x08],
        substate: bytes[0x09],
        count: w12a02_read_u16(bytes, 0x0a),
        extent_a: w12a02_read_u16(bytes, 0x0c),
        extent_b: w12a02_read_u16(bytes, 0x0e),
        marker_1c: w12a02_read_u32(bytes, 0x1c),
        marker_20: w12a02_read_u32(bytes, 0x20),
        target_28: w12a02_read_u32(bytes, 0x28),
    }
}

// ---------------------------------------------------------------------------
// 0x00654212 - keep the closer candidate of a scan
// ---------------------------------------------------------------------------

/// One candidate of the scan that `0x00654212` folds: the value the caller is
/// currently holding plus the metric the scan compares.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12a02Candidate {
    /// The value stored for the best candidate so far.
    pub id: u32,
    /// Metric of the best candidate; `u32::MAX` is the "nothing seen yet"
    /// seed the callers of the scan use.
    pub distance: u32,
}

impl W12a02Candidate {
    /// The seed of a scan that has not seen a candidate yet.
    pub const NONE: Self = Self {
        id: 0,
        distance: u32::MAX,
    };
}

/// `0x00654212` (`FUN_00654212`, 73 bytes): the fold step of a nearest-
/// candidate scan.  The routine is the body of the scan loop, so its candidate
/// and threshold arrive in `unaff_EBP` slots.
///
/// ```c
/// uVar1 = FUN_00604060(...);          // octagonal distance of this candidate
/// if (uVar1 < threshold) {
///     best      = candidate;
///     threshold = uVar1;
/// }
/// ```
///
/// `FUN_00604060` is the shipped octagonal distance helper, so the comparison
/// is a plain unsigned `<`: the **first** candidate wins every tie.  The
/// recovered loop tail (walking the owner's unit list into the non-returning
/// `FUN_005b4cd0`) does not touch either slot and is not modelled.
///
/// Returns whether the stored candidate changed.
pub fn w12a02_keep_closer_candidate(best: &mut W12a02Candidate, id: u32, distance: u32) -> bool {
    if distance < best.distance {
        best.id = id;
        best.distance = distance;
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// 0x00654770 / 0x006543b0 / 0x006547b0 - placement predicates
// ---------------------------------------------------------------------------

/// The shared "this row is still clear" test of `0x00654770` and
/// `0x006543b0`.
///
/// ```c
/// state != 3 && *(u32 *)(row + 0x1c) == 0 && *(u32 *)(row + 0x20) == 0
///     && (flags & 0x20) == 0
/// ```
fn w12a02_row_clear(row: &W12a02Row) -> bool {
    row.state != 3
        && row.marker_1c == 0
        && row.marker_20 == 0
        && row.flags & W12A02_ROW_CLAIMED_FLAG == 0
}

/// `0x00654770` (`FUN_00654770`, 52 bytes): the clear test for one indexed row
/// of a player's table.
///
/// ```c
/// row = base[player] + index * 0x34;
/// return state != 3 && row[0x1c] == 0 && row[0x20] == 0 && (flags & 0x20) == 0;
/// ```
///
/// `rows` is the player's table.  An index outside the table answers `false`
/// (retail reads past the array; the model refuses to call that clear).
pub fn w12a02_row_index_is_clear(rows: &[W12a02Row], index: usize) -> bool {
    rows.get(index).map(w12a02_row_clear).unwrap_or(false)
}

/// `0x006543b0` (`FUN_006543b0`, 99 bytes): true when every row of a region's
/// tile list is clear.
///
/// ```c
/// count = FUN_0075d6e0(region, &tiles);
/// for (i = 0; i < count; i++) {
///     row = base[player] + tiles[i] * 0x34;
///     if (state == 3 || row[0x1c] != 0 || row[0x20] != 0 || (flags & 0x20) != 0)
///         return 0;
/// }
/// return 1;
/// ```
///
/// `tiles` is the list the caller obtained for the region, in list order.  An
/// empty list answers `true`, exactly like the shipped `count == 0` path.  A
/// tile outside `rows` answers `false` (see [`w12a02_row_index_is_clear`]).
pub fn w12a02_region_tiles_clear(rows: &[W12a02Row], tiles: &[u16]) -> bool {
    for &tile in tiles {
        match rows.get(tile as usize) {
            Some(row) => {
                if !w12a02_row_clear(row) {
                    return false;
                }
            }
            None => return false,
        }
    }
    true
}

/// `0x006547b0` (`FUN_006547b0`, 85 bytes): true when a live unit's own row is
/// clear.
///
/// ```c
/// if (unit == 0) return 0;
/// if (FUN_005a3180() == 0) {                 // 0 = the unit is active
///     row = base[unit[0x4c]] + (FUN_0075d7b0(unit) & 0xffff) * 0x34;
///     if (state != 3 && row[0x1c] == 0 && row[0x20] == 0 && (flags & 0x20) == 0)
///         return 1;
/// }
/// return 0;
/// ```
///
/// `FUN_005a3180` is the shipped liveness gate (non-zero means "gone": no
/// sprite at `+0x0c` or a zero byte at `+0x4d`), so `unit_active` is
/// `FUN_005a3180() == 0`.  `region` is the `FUN_0075d7b0(unit)` index; the
/// table itself is selected by the unit's `+0x4c` owner byte, which is why
/// `rows` is the owning player's table.
pub fn w12a02_unit_row_is_clear(rows: &[W12a02Row], region: u16, unit_active: bool) -> bool {
    if !unit_active {
        return false;
    }
    w12a02_row_index_is_clear(rows, region as usize)
}

// ---------------------------------------------------------------------------
// 0x00654420 - link a unit into a row's membership chain
// ---------------------------------------------------------------------------

/// One node of the membership chain that `0x00654420` builds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12a02MemberNode {
    /// `+0x00`: next node.
    pub next: Option<usize>,
    /// `+0x04`: previous node.
    pub prev: Option<usize>,
    /// `+0x08`: kind byte; [`W12A02_NODE_KIND_MEMBER`] once linked.
    pub kind: u8,
    /// `+0x0c`: the unit the node belongs to (stored into `CUnit+0x134`).
    pub unit: u32,
    /// `+0x10`: the row the node was linked into.
    pub row: usize,
}

/// Head/count pair a row keeps for its membership chain (`+0x30` and `+0x0a`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12a02MemberList {
    /// `+0x30`: first node of the chain.
    pub head: Option<usize>,
    /// `+0x0a`: `u16` member count.
    pub count: u16,
}

/// `0x00654420` (`FUN_00654420`, 187 bytes), row selection.
///
/// ```c
/// row = param_1;
/// if (row[5] == 3 && param_3 == 0) {
///     row = base[param_2[0x4c]] + (FUN_0075d7b0(param_2) & 0xffff) * 0x34;
/// }
/// ```
///
/// A structure row (`state == 3`) is replaced by the unit's own row **only when
/// the third argument is zero**.  The two call sites in this window pass `1`,
/// so the replacement is the cold path; it is modelled separately because it
/// changes which row receives the node.
pub fn w12a02_select_attachment_row(
    row_state: u8,
    flag_set: bool,
    unit_row: usize,
    row: usize,
) -> usize {
    if row_state == 3 && !flag_set {
        unit_row
    } else {
        row
    }
}

/// `0x00654420` (`FUN_00654420`, 187 bytes), chain splice.
///
/// ```c
/// node = pool->top;                       // *(pool + 0x4e20)
/// if (node == 0) return;
/// pool->top = node->next;
/// if (node->next) node->next->prev = 0;
/// node->prev = 0;
/// node->next = row->head;                 // row + 0x30
/// if (row->head) row->head->prev = node;
/// row->head = node;
/// node->kind = 4;
/// node->unit = unit;
/// node->row  = row;
/// if (FUN_005b5c70(unit) == 0) row->count++;   // ground units only
/// unit->node = node;                      // CUnit + 0x134
/// ```
///
/// The node is popped from the pool's free chain and pushed onto the front of
/// the row's chain.  `unit_airborne` is the `FUN_005b5c70` answer (bit 2 of
/// `CUnit+0xdc`): airborne units join the chain without touching the count.
/// Returns the node index, or `None` when the pool is empty or the chain
/// indices do not fit `nodes`.
pub fn w12a02_attach_member(
    nodes: &mut [W12a02MemberNode],
    free: &mut Option<usize>,
    list: &mut W12a02MemberList,
    unit: u32,
    row: usize,
    unit_airborne: bool,
) -> Option<usize> {
    let index = (*free)?;
    if index >= nodes.len() {
        return None;
    }
    let next_free = nodes[index].next;
    if let Some(next) = next_free
        && next < nodes.len()
    {
        nodes[next].prev = None;
    }
    *free = next_free;
    nodes[index].prev = None;
    nodes[index].next = list.head;
    if let Some(head) = list.head
        && head < nodes.len()
    {
        nodes[head].prev = Some(index);
    }
    list.head = Some(index);
    nodes[index].kind = W12A02_NODE_KIND_MEMBER;
    nodes[index].unit = unit;
    nodes[index].row = row;
    if !unit_airborne {
        list.count = list.count.wrapping_add(1);
    }
    Some(index)
}

// ---------------------------------------------------------------------------
// 0x00654520 / 0x00654560 / 0x006545e0 - member node payload accessors
// ---------------------------------------------------------------------------

/// The member node as the three accessors of this section read it:
/// `CUnit+0x134` plus the node and payload fields they touch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12a02NodeView {
    /// `node+0x08`: kind byte; `4` is the member kind.
    pub kind: u8,
    /// `node+0x0c`: the unit that owns the node.
    pub unit: u32,
    /// `node+0x10` payload state byte (`payload+0x05`).
    pub payload_state: u8,
    /// `payload+0x1c`: first placement marker.
    pub payload_first: u32,
    /// `payload+0x20`: second placement marker.
    pub payload_second: u32,
}

/// `0x00654520` (`FUN_00654520`, 59 bytes): the `+0x1c` marker of the row the
/// unit's member node points at.
///
/// ```c
/// node = unit->node;                                  // CUnit + 0x134
/// if (node != 0 && node->kind == 4 && node->unit == unit) {
///     if (FUN_005b5820() == 0)                        // not a building
///         return *(u32 *)(node->payload + 0x1c);
/// }
/// return 0;
/// ```
///
/// `unit_is_building` is the `FUN_005b5820` answer (`units.dat` building flag
/// `0x1` behind `CUnit+0x64`); buildings answer `0`.
pub fn w12a02_node_payload_first(
    node: Option<&W12a02NodeView>,
    unit: u32,
    unit_is_building: bool,
) -> u32 {
    match node {
        Some(node)
            if node.kind == W12A02_NODE_KIND_MEMBER && node.unit == unit && !unit_is_building =>
        {
            node.payload_first
        }
        _ => 0,
    }
}

/// `0x00654560` (`FUN_00654560`, 56 bytes): the `+0x20` marker, with the same
/// node, owner and building gates as [`w12a02_node_payload_first`].
pub fn w12a02_node_payload_second(
    node: Option<&W12a02NodeView>,
    unit: u32,
    unit_is_building: bool,
) -> u32 {
    match node {
        Some(node)
            if node.kind == W12A02_NODE_KIND_MEMBER && node.unit == unit && !unit_is_building =>
        {
            node.payload_second
        }
        _ => 0,
    }
}

/// `0x006545e0` (`FUN_006545e0`, 39 bytes): true when the unit has a member
/// node whose payload row is still empty.
///
/// ```c
/// node = unit->node;
/// if (node != 0 && node->kind == 4)
///     return *(char *)(node->payload + 5) == 0;
/// return false;
/// ```
///
/// Unlike the two marker accessors this one has **no** owner and **no**
/// building gate.
pub fn w12a02_node_payload_idle(node: Option<&W12a02NodeView>) -> bool {
    match node {
        Some(node) => node.kind == W12A02_NODE_KIND_MEMBER && node.payload_state == 0,
        None => false,
    }
}

// ---------------------------------------------------------------------------
// 0x00654838 - footprint sum over a rectangle query
// ---------------------------------------------------------------------------

/// Half extent of the query rectangle built by `0x00654838` (`0x400` world
/// units on every side of the region origin).
pub const W12A02_SCAN_HALF: i32 = 0x400;

/// A world rectangle, in the word order the spatial query expects:
/// `(left, top, right, bottom)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12a02Rect {
    /// `+0`: left edge.
    pub left: i16,
    /// `+2`: top edge.
    pub top: i16,
    /// `+4`: right edge.
    pub right: i16,
    /// `+6`: bottom edge.
    pub bottom: i16,
}

/// `0x00654838` (`FUN_00654838`, 93 bytes), rectangle construction.
///
/// ```c
/// FUN_0075d630(region, &x, &y);
/// rect = { (short)(x - 0x400), (short)(y - 0x400),
///          (short)(x + 0x400), (short)(y + 0x400) };
/// ```
///
/// The four edges are stored as 16-bit words, so the additions wrap; the model
/// keeps that (a map coordinate near `i16::MAX` produces a negative right
/// edge, just like the shipped store).
pub fn w12a02_scan_rect(x: i32, y: i32) -> W12a02Rect {
    let half = W12A02_SCAN_HALF as i16;
    let x = x as i16;
    let y = y as i16;
    W12a02Rect {
        left: x.wrapping_sub(half),
        top: y.wrapping_sub(half),
        right: x.wrapping_add(half),
        bottom: y.wrapping_add(half),
    }
}

/// One unit of the `0x00654838` rectangle query: the two footprint words at
/// `CUnit+0x138` and `CUnit+0x13a`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12a02ExtentUnit {
    /// `+0x138`: first footprint word.
    pub extent_a: u16,
    /// `+0x13a`: second footprint word.
    pub extent_b: u16,
}

impl W12a02ExtentUnit {
    /// The per-unit contribution: `max(extent_a, extent_b)`.
    pub fn contribution(&self) -> u32 {
        u32::from(self.extent_a.max(self.extent_b))
    }
}

/// `0x00654838` (`FUN_00654838`, 93 bytes), accumulation loop.
///
/// ```c
/// sum = *out;                                  // the caller's accumulator
/// for (unit in list) {
///     if (FUN_005b5b90(player, unit) == 0) continue;   // owner filter
///     w = unit[0x138];
///     if (w <= unit[0x13a]) w = unit[0x13a];
///     sum += w;                                // 32-bit wrapping add
/// }
/// *out = sum;                                  // FUN_005a5770 releases the list
/// return sum;
/// ```
///
/// The `<=` form is exactly `max`: when the two words are equal either one is
/// taken.  `keep` is the `FUN_005b5b90` owner filter; `units` is the spatial
/// query result in list order and `initial` the accumulator's incoming value.
pub fn w12a02_rect_extent_sum(
    units: &[W12a02ExtentUnit],
    initial: u32,
    keep: impl Fn(&W12a02ExtentUnit) -> bool,
) -> u32 {
    let mut sum = initial;
    for unit in units {
        if !keep(unit) {
            continue;
        }
        sum = sum.wrapping_add(unit.contribution());
    }
    sum
}

// ---------------------------------------------------------------------------
// 0x00655560 - "any row in an active state" scan
// ---------------------------------------------------------------------------

/// `0x00655560` (`FUN_00655560`, 69 bytes): true when any row of the table
/// other than row `0` holds state `1`, `2`, `8` or `9`.
///
/// ```c
/// count = FUN_0075d710();                      // row count
/// for (i = 1; i < count; i++) {
///     switch (*(u8 *)(base[player] + i * 0x34 + 5)) {
///     case 1: case 2: case 8: case 9: return 1;
///     }
/// }
/// return 0;
/// ```
///
/// Retail starts at `+0x39`, which is row `1` plus the state byte `+0x05`, so
/// row `0` is never inspected.  A table of zero or one row therefore answers
/// `false`.
pub fn w12a02_any_row_state_active(rows: &[W12a02Row]) -> bool {
    rows.iter()
        .skip(1)
        .any(|row| matches!(row.state, 1 | 2 | 8 | 9))
}

// ---------------------------------------------------------------------------
// 0x006555d0 / 0x00655660 - rectangle selection dispatch
// ---------------------------------------------------------------------------

/// Event id that both selection routines dispatch for every selected unit
/// (`FUN_005b7dd0(0x5c, unit)`).
pub const W12A02_EVENT_UNIT_CLICK: u32 = 0x5c;

/// One unit of the rectangle query behind `0x006555d0` and `0x00655660`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W12a02SelectUnit {
    /// `CUnit+0x4c`: owner slot compared with the routine's player byte.
    pub owner: u8,
    /// First per-unit gate: `FUN_005c7c20(unit) != 0` in `0x006555d0` and
    /// `FUN_006c20c0(0x00fe5b54, ..., unit) != 0` in `0x00655660`.
    pub gate_a: bool,
    /// Second per-unit gate, `0x006555d0` only: `FUN_005d4a80(unit) != 0`.
    pub gate_b: bool,
}

fn w12a02_rect_owner_filter(
    owner: u8,
    units: &[W12a02SelectUnit],
    gate: impl Fn(&W12a02SelectUnit) -> bool,
) -> Vec<usize> {
    let mut selected = Vec::new();
    for (index, unit) in units.iter().enumerate() {
        if unit.owner == owner && gate(unit) {
            selected.push(index);
        }
    }
    selected
}

/// `0x006555d0` (`FUN_006555d0`, 141 bytes): dispatch the click event for
/// every unit of a rectangle whose owner matches and which passes both gates.
///
/// ```c
/// rect = { word[0], word[2], word[4], word[6] };
/// for (unit in FUN_005a57f0(&rect)) {
///     if (unit[0x4c] == player && FUN_005c7c20(unit) && FUN_005d4a80(unit))
///         FUN_005b7dd0(0x5c, unit);
/// }
/// FUN_005a5770(list);
/// ```
///
/// The rectangle is built from the four words of the caller's record; the
/// spatial query itself is caller-supplied, so `units` is the query result in
/// list order.  Returns the indices that receive [`W12A02_EVENT_UNIT_CLICK`]
/// (or nothing when the rectangle is empty).
pub fn w12a02_rect_owner_two_gate(owner: u8, units: &[W12a02SelectUnit]) -> Vec<usize> {
    w12a02_rect_owner_filter(owner, units, |unit| unit.gate_a && unit.gate_b)
}

/// `0x00655660` (`FUN_00655660`, 138 bytes): the single-gate variant of
/// [`w12a02_rect_owner_two_gate`].
///
/// ```c
/// for (unit in FUN_005a57f0(&rect)) {
///     if (unit[0x4c] == player && FUN_006c20c0(&DAT_00fe5b54, &LAB_006512e0, unit))
///         FUN_005b7dd0(0x5c, unit);
/// }
/// ```
///
/// The registry lookup behind `FUN_006c20c0` is a per-unit client-side
/// membership test; it enters as `gate_a`, so only one gate is consulted.
pub fn w12a02_rect_owner_registry(owner: u8, units: &[W12a02SelectUnit]) -> Vec<usize> {
    w12a02_rect_owner_filter(owner, units, |unit| unit.gate_a)
}

// ---------------------------------------------------------------------------
// 0x00655990 - pick a spot inside the unit's region
// ---------------------------------------------------------------------------

/// The "no candidate seen" sentinel of `0x00655990`; it is also one past the
/// largest valid row index, so it cannot collide with a picked tile.
pub const W12A02_REGION_SENTINEL: u16 = 0x9c4;

/// The order id dispatched with the picked coordinates
/// (`FUN_005b7d80(0x70, x, y)`).
pub const W12A02_ORDER_MOVE: u32 = 0x70;

/// The tile marker that makes `0x00655990` hand over to the wider search
/// instead of issuing the move order (the region word is compared with
/// `0x1ffd`).
pub const W12A02_TILE_MARKER: i16 = 0x1ffd;

/// `0x00655990` (`FUN_00655990`, 233 bytes): choose the tile of the unit's
/// region that the move order should target.
///
/// ```c
/// own = FUN_0075d7b0(unit) & 0xffff;
/// preferred = empty = other = 0x9c4;
/// for (tile in FUN_0075d6b0(own)) {
///     switch (row(tile)[5]) {
///     case 0:                 empty     = tile; break;
///     case 4: case 5: case 8: case 9:  preferred = tile; break;
///     default:                other     = tile; break;
///     }
/// }
/// chosen = preferred != 0x9c4 ? preferred
///        : empty     != 0x9c4 ? empty
///        : other     != 0x9c4 ? other
///        : own;
/// ```
///
/// Every category keeps the **last** match in list order.  When the region list
/// is empty (or the routine's first branch is taken) the answer is the unit's
/// own region index, which is also the fall-through of the shipped
/// `local_10 != sentinel` chain.
///
/// Retail then tests [`W12A02_TILE_MARKER`] on the chosen tile: a match re-runs
/// the wider search, otherwise `FUN_005b7d80` dispatches
/// [`W12A02_ORDER_MOVE`] at the coordinates of `FUN_0075d630(tile)`.  Both of
/// those are caller-side.
pub fn w12a02_region_spot_tile(rows: &[W12a02Row], region: u16, tiles: &[u16]) -> u16 {
    let mut preferred = W12A02_REGION_SENTINEL;
    let mut empty = W12A02_REGION_SENTINEL;
    let mut other = W12A02_REGION_SENTINEL;
    for &tile in tiles {
        let Some(row) = rows.get(tile as usize) else {
            continue;
        };
        match row.state {
            0 => empty = tile,
            4 | 5 | 8 | 9 => preferred = tile,
            _ => other = tile,
        }
    }
    if preferred != W12A02_REGION_SENTINEL {
        return preferred;
    }
    if empty != W12A02_REGION_SENTINEL {
        return empty;
    }
    if other != W12A02_REGION_SENTINEL {
        return other;
    }
    region
}

// ---------------------------------------------------------------------------
// 0x0065534c - placement-class grid expansion
// ---------------------------------------------------------------------------

/// Cell count of one placement-class grid (`0x9c4`, the same window the
/// reachability grid uses).
pub const W12A02_GRID_CELLS: usize = 0x9c4;

/// `0x0065534c` (`FUN_0065534c`, 522 bytes): expand the placement classes of a
/// grid until a full pass changes nothing.
///
/// Each pass works on a snapshot of the grid so that one pass only reads the
/// previous pass's classes:
///
/// ```c
/// do {
///     changed = false;
///     snapshot = grid;
///     for (i = 0; i < 0x9c4; i++)
///         if (snapshot[i] > 4)
///             for (n in neighbours(i)) {
///                 if (grid[n] == 1) { grid[n] = 2; changed = true; }
///                 else if (grid[n] == 0) { grid[n] = 6; changed = true; }
///             }
///     if (!changed) break;
///     snapshot = grid;
///     for (i = 0; i < 0x9c4; i++)
///         if (snapshot[i] == 2) {
///             grid[i] = 3;
///             for (n in neighbours(i)) if (grid[n] == 0) grid[n] = 4;
///         }
/// } while (true);
/// ```
///
/// Classes are only ever rewritten along `0 -> 6`, `1 -> 2`, `2 -> 3` and
/// `0 -> 4`, so the loop is monotone and terminates.  The routine that follows
/// the loop in retail (`FUN_00658240` placement passes, `FUN_00635f80` validity
/// list, `FUN_00635bc0` commit) is client-side and is not modelled; `FUN_00655306`
/// is the same loop reached from a different entry.
///
/// `neighbours[i]` is the neighbour list of cell `i` (`FUN_0075d6b0`), as a
/// slice; entries outside the grid are ignored defensively.
pub fn w12a02_expand_placement_classes(
    grid: &mut [u8; W12A02_GRID_CELLS],
    neighbours: &[Vec<u16>],
) {
    loop {
        let mut snapshot = *grid;
        let mut changed = false;
        for index in 0..W12A02_GRID_CELLS {
            if snapshot[index] <= 4 {
                continue;
            }
            let Some(list) = neighbours.get(index) else {
                continue;
            };
            for &neighbour in list {
                let cell = neighbour as usize;
                if cell >= W12A02_GRID_CELLS {
                    continue;
                }
                if grid[cell] == 1 {
                    grid[cell] = 2;
                    changed = true;
                } else if grid[cell] == 0 {
                    grid[cell] = 6;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
        snapshot = *grid;
        for index in 0..W12A02_GRID_CELLS {
            if snapshot[index] != 2 {
                continue;
            }
            grid[index] = 3;
            let Some(list) = neighbours.get(index) else {
                continue;
            };
            for &neighbour in list {
                let cell = neighbour as usize;
                if cell < W12A02_GRID_CELLS && grid[cell] == 0 {
                    grid[cell] = 4;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 0x00655e10 - bucket pick of a blocked placement row
// ---------------------------------------------------------------------------

/// Candidate capacity of each of the four buckets in `0x00655e10` (`0x10`).
pub const W12A02_BUCKET_CAP: usize = 0x10;

/// Random-draw id used to pick inside a bucket (`FUN_0073f3b0(0x37)`).
pub const W12A02_BUCKET_ROLL: u32 = 0x37;

/// `0x00655e10` (`FUN_00655e10`, 653 bytes): pick a placement row near the
/// unit out of four priority buckets and mark it taken.
///
/// ```c
/// own = FUN_0075d7b0(unit) & 0xffff;
/// for (tile in FUN_0075d6e0(own)) {
///     row = base[player] + tile * 0x34;
///     if (row[5] != 4 && row[5] != 5) continue;
///     if (row[8] & 1 == 0) push(row[8] & 4 ? b1 : b0, tile);
///     else                 push(row[8] & 4 ? b3 : b2, tile);
/// }
/// chosen = b2 non-empty ? b2[rand % n]
///        : b3 non-empty ? b3[rand % n]
///        : b0 non-empty ? b0[rand % n]
///        : b1 non-empty ? b1[rand % n]
///        : own;
/// FUN_0075d630(chosen, &x, &y);       // coordinates for the caller
/// chosen_row[8] |= 4;
/// ```
///
/// Each bucket drops candidates once it holds [`W12A02_BUCKET_CAP`] entries.
/// The bucket priority is `(bit0 set, bit2 clear)`, `(bit0 set, bit2 set)`,
/// `(bit0 clear, bit2 clear)`, `(bit0 clear, bit2 set)`.
///
/// Retail then checks the chosen row's state: if it is not `4`/`5` it scans
/// every row from index `1` for a placement row and takes the non-returning
/// error path when it finds one (see [`w12a02_any_placement_row`]).  The model
/// marks the chosen row and returns its tile; the coordinate read and the
/// order dispatch stay with the caller.
pub fn w12a02_pick_blocked_spot(
    rows: &mut [W12a02Row],
    tiles: &[u16],
    roll: u32,
    own_region: u16,
) -> u16 {
    let mut buckets: [Vec<u16>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for &tile in tiles {
        let Some(row) = rows.get(tile as usize) else {
            continue;
        };
        if row.state != 4 && row.state != 5 {
            continue;
        }
        let bucket = match (row.flags & 1 != 0, row.flags & 4 != 0) {
            (false, false) => 0,
            (false, true) => 1,
            (true, false) => 2,
            (true, true) => 3,
        };
        if buckets[bucket].len() < W12A02_BUCKET_CAP {
            buckets[bucket].push(tile);
        }
    }
    let mut chosen = own_region;
    for bucket in [2usize, 3, 0, 1] {
        let candidates = &buckets[bucket];
        if candidates.is_empty() {
            continue;
        }
        chosen = candidates[(roll % candidates.len() as u32) as usize];
        break;
    }
    if let Some(row) = rows.get_mut(chosen as usize) {
        row.flags |= W12A02_ROW_SPOT_TAKEN_FLAG;
    }
    chosen
}

/// `0x00655e10` (`FUN_00655e10`, 653 bytes), defensive scan: true when any row
/// other than row `0` is in the placement state `4` or `5`.
///
/// ```c
/// count = FUN_0075d710();
/// for (i = 1; i < count; i++)
///     if (row(i)[5] == 4 || row(i)[5] == 5) error();
/// ```
///
/// The shipped code runs this only when the bucket pick fell back to the
/// unit's own region; it is an assertion that no placement row exists anywhere,
/// not a placement test.
pub fn w12a02_any_placement_row(rows: &[W12a02Row]) -> bool {
    rows.iter()
        .skip(1)
        .any(|row| row.state == 4 || row.state == 5)
}

#[cfg(test)]
mod tests_wave12_agent02 {
    use super::*;

    fn row(state: u8, flags: u8) -> W12a02Row {
        W12a02Row {
            state,
            flags,
            ..W12a02Row::default()
        }
    }

    fn node(kind: u8, unit: u32) -> W12a02NodeView {
        W12a02NodeView {
            kind,
            unit,
            payload_state: 0,
            payload_first: 0x11,
            payload_second: 0x22,
        }
    }

    #[test]
    fn row_bytes_decode_at_the_recovered_offsets() {
        let mut bytes = [0u8; W12A02_ROW_BYTES];
        bytes[0x00] = 0x34;
        bytes[0x01] = 0x12;
        bytes[0x02] = 0x78;
        bytes[0x03] = 0x56;
        bytes[0x04] = 2;
        bytes[0x05] = 5;
        bytes[0x06] = 7;
        bytes[0x08] = 0x21;
        bytes[0x09] = 9;
        bytes[0x0a] = 3;
        bytes[0x0c] = 0x10;
        bytes[0x0e] = 0x20;
        bytes[0x1c] = 0xaa;
        bytes[0x20] = 0xbb;
        bytes[0x28] = 0xcc;
        let decoded = w12a02_row_from_bytes(&bytes);
        assert_eq!(decoded.region, 0x1234);
        assert_eq!(decoded.related_row, 0x5678);
        assert_eq!(decoded.owner, 2);
        assert_eq!(decoded.state, 5);
        assert_eq!(decoded.step, 7);
        assert_eq!(decoded.flags, 0x21);
        assert_eq!(decoded.substate, 9);
        assert_eq!(decoded.count, 3);
        assert_eq!(decoded.extent_a, 0x10);
        assert_eq!(decoded.extent_b, 0x20);
        assert_eq!(decoded.marker_1c, 0xaa);
        assert_eq!(decoded.marker_20, 0xbb);
        assert_eq!(decoded.target_28, 0xcc);
        assert_eq!(W12A02_ROW_BYTES, 0x34);
    }

    #[test]
    fn keep_closer_candidate_keeps_the_first_of_equal_metrics() {
        let mut best = W12a02Candidate::NONE;
        assert_eq!(best.distance, u32::MAX);
        assert!(w12a02_keep_closer_candidate(&mut best, 7, 100));
        assert_eq!(
            best,
            W12a02Candidate {
                id: 7,
                distance: 100
            }
        );
        assert!(!w12a02_keep_closer_candidate(&mut best, 9, 100));
        assert_eq!(best.id, 7);
        assert!(w12a02_keep_closer_candidate(&mut best, 8, 99));
        assert_eq!(
            best,
            W12a02Candidate {
                id: 8,
                distance: 99
            }
        );
        // Only a strictly smaller metric wins, and the seed is the maximum.
        assert!(!w12a02_keep_closer_candidate(&mut best, 1, 500));
        assert_eq!(best.id, 8);
    }

    #[test]
    fn row_index_is_clear_tests_state_markers_and_the_claimed_flag() {
        let clear = W12a02Row::default();
        let mut rows = vec![clear; 4];
        assert!(w12a02_row_index_is_clear(&rows, 0));
        assert!(!w12a02_row_index_is_clear(&rows, 4));

        rows[1] = row(3, 0);
        assert!(!w12a02_row_index_is_clear(&rows, 1));

        rows[1] = W12a02Row {
            marker_1c: 1,
            ..W12a02Row::default()
        };
        assert!(!w12a02_row_index_is_clear(&rows, 1));

        rows[1] = W12a02Row {
            marker_20: 1,
            ..W12a02Row::default()
        };
        assert!(!w12a02_row_index_is_clear(&rows, 1));

        rows[1] = row(0, W12A02_ROW_CLAIMED_FLAG);
        assert!(!w12a02_row_index_is_clear(&rows, 1));

        // The claimed flag is the only flag bit the predicate reads.
        rows[1] = row(0, 0xdf);
        assert!(w12a02_row_index_is_clear(&rows, 1));
        assert_eq!(W12A02_ROW_CLAIMED_FLAG, 0x20);
    }

    #[test]
    fn region_tiles_clear_accepts_an_empty_list_and_rejects_one_busy_tile() {
        let rows = vec![W12a02Row::default(); 6];
        assert!(w12a02_region_tiles_clear(&rows, &[]));
        assert!(w12a02_region_tiles_clear(&rows, &[1, 2, 3]));
        // Out of range tiles are not proven clear.
        assert!(!w12a02_region_tiles_clear(&rows, &[1, 6]));

        let mut busy = rows.clone();
        busy[3] = row(3, 0);
        assert!(!w12a02_region_tiles_clear(&busy, &[1, 2, 3, 4]));
        assert!(w12a02_region_tiles_clear(&busy, &[1, 2, 4]));
    }

    #[test]
    fn unit_row_is_clear_needs_an_active_unit_and_its_clear_row() {
        let mut rows = vec![W12a02Row::default(); 8];
        assert!(w12a02_unit_row_is_clear(&rows, 5, true));
        assert!(!w12a02_unit_row_is_clear(&rows, 5, false));
        assert!(!w12a02_unit_row_is_clear(&rows, 8, true));

        rows[5] = row(3, 0);
        assert!(!w12a02_unit_row_is_clear(&rows, 5, true));

        rows[5] = W12a02Row {
            marker_20: 4,
            ..W12a02Row::default()
        };
        assert!(!w12a02_unit_row_is_clear(&rows, 5, true));
    }

    #[test]
    fn select_attachment_row_retargets_a_structure_only_when_the_flag_is_clear() {
        // state 3, flag clear -> the unit's own row.
        assert_eq!(w12a02_select_attachment_row(3, false, 9, 4), 9);
        // state 3, flag set -> the received row.
        assert_eq!(w12a02_select_attachment_row(3, true, 9, 4), 4);
        // Any other state keeps the received row in both cases.
        assert_eq!(w12a02_select_attachment_row(4, false, 9, 4), 4);
        assert_eq!(w12a02_select_attachment_row(0, true, 9, 4), 4);
    }

    #[test]
    fn attach_member_pops_the_pool_and_pushes_onto_the_row_chain() {
        let mut nodes = [
            W12a02MemberNode {
                next: Some(1),
                ..W12a02MemberNode::default()
            },
            W12a02MemberNode::default(),
        ];
        let mut free = Some(0);
        let mut list = W12a02MemberList::default();

        assert_eq!(
            w12a02_attach_member(&mut nodes, &mut free, &mut list, 42, 7, false),
            Some(0)
        );
        assert_eq!(free, Some(1));
        assert_eq!(list.head, Some(0));
        assert_eq!(list.count, 1);
        assert_eq!(nodes[0].kind, W12A02_NODE_KIND_MEMBER);
        assert_eq!(nodes[0].unit, 42);
        assert_eq!(nodes[0].row, 7);
        assert_eq!(nodes[0].next, None);
        assert_eq!(nodes[0].prev, None);

        assert_eq!(
            w12a02_attach_member(&mut nodes, &mut free, &mut list, 43, 8, true),
            Some(1)
        );
        assert_eq!(free, None);
        assert_eq!(list.head, Some(1));
        // Airborne units join without touching the count.
        assert_eq!(list.count, 1);
        assert_eq!(nodes[1].next, Some(0));
        assert_eq!(nodes[0].prev, Some(1));

        assert_eq!(
            w12a02_attach_member(&mut nodes, &mut free, &mut list, 44, 9, false),
            None
        );
        assert_eq!(list.count, 1);
    }

    #[test]
    fn node_payload_accessors_gate_on_kind_owner_and_building_flag() {
        let view = node(W12A02_NODE_KIND_MEMBER, 5);
        assert_eq!(w12a02_node_payload_first(Some(&view), 5, false), 0x11);
        assert_eq!(w12a02_node_payload_second(Some(&view), 5, false), 0x22);
        // Wrong owner, wrong kind, building unit and missing node all answer 0.
        assert_eq!(w12a02_node_payload_first(Some(&view), 6, false), 0);
        assert_eq!(w12a02_node_payload_first(Some(&node(1, 5)), 5, false), 0);
        assert_eq!(w12a02_node_payload_first(Some(&view), 5, true), 0);
        assert_eq!(w12a02_node_payload_second(Some(&view), 5, true), 0);
        assert_eq!(w12a02_node_payload_first(None, 5, false), 0);
        assert_eq!(w12a02_node_payload_second(None, 5, false), 0);
    }

    #[test]
    fn node_payload_idle_has_no_owner_or_building_gate() {
        let mut view = node(W12A02_NODE_KIND_MEMBER, 5);
        assert!(w12a02_node_payload_idle(Some(&view)));
        // An empty payload row is idle even for another owner.
        assert!(w12a02_node_payload_idle(Some(&view)));
        view.payload_state = 1;
        assert!(!w12a02_node_payload_idle(Some(&view)));
        view.payload_state = 0;
        view.kind = 1;
        assert!(!w12a02_node_payload_idle(Some(&view)));
        assert!(!w12a02_node_payload_idle(None));
    }

    #[test]
    fn scan_rect_expands_the_region_origin_by_0x400_with_word_wrap() {
        let rect = w12a02_scan_rect(1000, 2000);
        assert_eq!(
            rect,
            W12a02Rect {
                left: -24,
                top: 976,
                right: 2024,
                bottom: 3024
            }
        );
        assert_eq!(W12A02_SCAN_HALF, 0x400);

        let wrapped = w12a02_scan_rect(i32::from(i16::MAX), 0);
        assert_eq!(wrapped.left, i16::MAX - 0x400);
        assert_eq!(wrapped.right, i16::MIN + 0x3ff);
        assert_eq!(wrapped.top, -0x400);
        assert_eq!(wrapped.bottom, 0x400);
    }

    #[test]
    fn rect_extent_sum_adds_the_larger_footprint_word() {
        let units = [
            W12a02ExtentUnit {
                extent_a: 3,
                extent_b: 5,
            },
            W12a02ExtentUnit {
                extent_a: 7,
                extent_b: 2,
            },
            W12a02ExtentUnit {
                extent_a: 4,
                extent_b: 4,
            },
        ];
        assert_eq!(w12a02_rect_extent_sum(&units, 0, |_| true), 16);
        assert_eq!(w12a02_rect_extent_sum(&units, 1, |_| true), 17);
        assert_eq!(
            w12a02_rect_extent_sum(&units, 0, |unit| unit.extent_a < 7),
            9
        );
        assert_eq!(
            w12a02_rect_extent_sum(&[], 0xffff_ffff, |_| true),
            0xffff_ffff
        );

        // The 32-bit accumulator wraps.
        let single = [W12a02ExtentUnit {
            extent_a: 4,
            extent_b: 1,
        }];
        assert_eq!(w12a02_rect_extent_sum(&single, 0xffff_fffe, |_| true), 2);
    }

    #[test]
    fn any_row_state_active_skips_row_zero_and_accepts_four_states() {
        let mut rows = vec![W12a02Row::default(); 3];
        assert!(!w12a02_any_row_state_active(&rows));
        assert!(!w12a02_any_row_state_active(&[]));
        assert!(!w12a02_any_row_state_active(&rows[..1]));

        // Row 0 is never inspected.
        rows[0] = row(1, 0);
        assert!(!w12a02_any_row_state_active(&rows));

        for state in [1u8, 2, 8, 9] {
            rows[1] = row(state, 0);
            assert!(w12a02_any_row_state_active(&rows), "state {state}");
        }
        for state in [3u8, 4, 5, 6, 7] {
            rows[1] = row(state, 0);
            assert!(!w12a02_any_row_state_active(&rows), "state {state}");
        }
    }

    #[test]
    fn rect_owner_two_gate_requires_owner_and_both_gates() {
        let units = [
            W12a02SelectUnit {
                owner: 1,
                gate_a: true,
                gate_b: true,
            },
            W12a02SelectUnit {
                owner: 2,
                gate_a: true,
                gate_b: true,
            },
            W12a02SelectUnit {
                owner: 1,
                gate_a: true,
                gate_b: false,
            },
            W12a02SelectUnit {
                owner: 1,
                gate_a: false,
                gate_b: true,
            },
            W12a02SelectUnit {
                owner: 1,
                gate_a: true,
                gate_b: true,
            },
        ];
        assert_eq!(w12a02_rect_owner_two_gate(1, &units), vec![0, 4]);
        assert!(w12a02_rect_owner_two_gate(3, &units).is_empty());
        assert_eq!(W12A02_EVENT_UNIT_CLICK, 0x5c);
    }

    #[test]
    fn rect_owner_registry_uses_only_the_registry_gate() {
        let units = [
            W12a02SelectUnit {
                owner: 0,
                gate_a: false,
                gate_b: true,
            },
            W12a02SelectUnit {
                owner: 0,
                gate_a: true,
                gate_b: false,
            },
            W12a02SelectUnit {
                owner: 1,
                gate_a: true,
                gate_b: false,
            },
        ];
        assert_eq!(w12a02_rect_owner_registry(0, &units), vec![1]);
        assert_eq!(w12a02_rect_owner_registry(1, &units), vec![2]);
    }

    #[test]
    fn region_spot_tile_prefers_the_last_match_of_each_category() {
        let mut rows = vec![W12a02Row::default(); 12];
        rows[7] = row(2, 0); // "other"
        rows[8] = row(0, 0); // empty
        rows[9] = row(5, 0); // preferred
        rows[10] = row(0, 0); // empty, later
        rows[11] = row(9, 0); // preferred, later

        // Both a preferred tile and two empty tiles exist.
        assert_eq!(w12a02_region_spot_tile(&rows, 3, &[7, 8, 9, 10, 11]), 11);
        // No preferred tile: the last empty one wins.
        assert_eq!(w12a02_region_spot_tile(&rows, 3, &[7, 8, 10]), 10);
        // Only "other" states: the last of them wins.
        assert_eq!(w12a02_region_spot_tile(&rows, 3, &[7, 7]), 7);
        // Empty list and unknown tiles fall back to the unit's own region.
        assert_eq!(w12a02_region_spot_tile(&rows, 3, &[]), 3);
        assert_eq!(w12a02_region_spot_tile(&rows, 3, &[42]), 3);
        assert_eq!(W12A02_REGION_SENTINEL, 0x9c4);
        assert_eq!(W12A02_ORDER_MOVE, 0x70);
        assert_eq!(W12A02_TILE_MARKER, 0x1ffd);
    }

    #[test]
    fn region_spot_tile_accepts_all_four_preferred_states() {
        let mut rows = vec![W12a02Row::default(); 4];
        for state in [4u8, 5, 8, 9] {
            rows[2] = row(state, 0);
            assert_eq!(w12a02_region_spot_tile(&rows, 0, &[2]), 2, "state {state}");
        }
        rows[2] = row(3, 0);
        assert_eq!(w12a02_region_spot_tile(&rows, 0, &[2]), 2);
        rows[2] = row(6, 0);
        assert_eq!(w12a02_region_spot_tile(&rows, 0, &[2]), 2);
    }

    #[test]
    fn expand_placement_classes_spreads_from_the_passable_class() {
        let mut grid = [0u8; W12A02_GRID_CELLS];
        let mut neighbours = vec![Vec::new(); W12A02_GRID_CELLS];
        grid[0] = 5;
        neighbours[0] = vec![1, 2];
        neighbours[1] = vec![10];

        w12a02_expand_placement_classes(&mut grid, &neighbours);

        assert_eq!(grid[0], 5);
        // Class 0 next to a class-5 cell becomes 6.
        assert_eq!(grid[1], 6);
        assert_eq!(grid[2], 6);
        assert_eq!(grid[10], 6);
    }

    #[test]
    fn expand_placement_classes_upgrades_class_two_to_three_and_marks_neighbours() {
        let mut grid = [0u8; W12A02_GRID_CELLS];
        let mut neighbours = vec![Vec::new(); W12A02_GRID_CELLS];
        grid[0] = 5;
        grid[1] = 1;
        neighbours[0] = vec![1];
        neighbours[1] = vec![2];

        w12a02_expand_placement_classes(&mut grid, &neighbours);

        assert_eq!(grid[0], 5);
        // 1 -> 2 in the first pass, 2 -> 3 in the second.
        assert_eq!(grid[1], 3);
        // The neighbour of the upgraded cell is marked with class 4.
        assert_eq!(grid[2], 4);
    }

    #[test]
    fn expand_placement_classes_is_a_no_op_on_a_stable_grid() {
        let mut grid = [0u8; W12A02_GRID_CELLS];
        let neighbours = vec![Vec::new(); W12A02_GRID_CELLS];
        grid[3] = 3;
        grid[4] = 4;
        grid[5] = 6;
        let before = grid;
        w12a02_expand_placement_classes(&mut grid, &neighbours);
        assert_eq!(grid, before);

        // A class-6 cell with a class-1 neighbour also settles: 1 -> 2, then
        // 2 -> 3, and nothing is left to expand.
        let mut grid = [0u8; W12A02_GRID_CELLS];
        let mut neighbours = vec![Vec::new(); W12A02_GRID_CELLS];
        grid[0] = 6;
        neighbours[0] = vec![1];
        let mut second = grid;
        second[1] = 1;
        w12a02_expand_placement_classes(&mut second, &neighbours);
        assert_eq!(second[1], 3);
        assert_eq!(second[0], 6);
    }

    #[test]
    fn pick_blocked_spot_follows_the_bucket_priority() {
        let mut rows = vec![W12a02Row::default(); 8];
        rows[1] = row(4, 0); // bit0 clear, bit2 clear -> bucket 0
        rows[2] = row(5, 4); // bit0 clear, bit2 set   -> bucket 1
        rows[3] = row(4, 1); // bit0 set,   bit2 clear -> bucket 2
        rows[4] = row(5, 5); // bit0 set,   bit2 set   -> bucket 3
        rows[5] = row(0, 1); // not a placement row
        let tiles = [1u16, 2, 3, 4, 5];

        // Bucket 2 wins first.
        assert_eq!(w12a02_pick_blocked_spot(&mut rows, &tiles, 0, 9), 3);
        assert_eq!(rows[3].flags, 1 | W12A02_ROW_SPOT_TAKEN_FLAG);

        // Remove bucket 2: bucket 3 wins.
        rows[3].state = 0;
        assert_eq!(w12a02_pick_blocked_spot(&mut rows, &tiles, 0, 9), 4);

        // Remove bucket 3: bucket 0 wins.
        rows[4].state = 0;
        assert_eq!(w12a02_pick_blocked_spot(&mut rows, &tiles, 0, 9), 1);

        // Remove bucket 0: bucket 1 wins.
        rows[1].state = 0;
        assert_eq!(w12a02_pick_blocked_spot(&mut rows, &tiles, 0, 9), 2);

        // Nothing left: the unit's own region is returned and marked.
        rows[2].state = 0;
        assert_eq!(w12a02_pick_blocked_spot(&mut rows, &tiles, 0, 6), 6);
        assert_eq!(rows[6].flags, W12A02_ROW_SPOT_TAKEN_FLAG);
    }

    #[test]
    fn pick_blocked_spot_caps_each_bucket_at_sixteen_and_uses_the_roll() {
        // Seventeen candidates in one bucket: only the first sixteen are kept.
        let build = || {
            let mut rows = vec![W12a02Row::default(); 0x14];
            for index in 1..=0x11 {
                rows[index] = row(4, 1);
            }
            rows
        };
        let tiles: Vec<u16> = (1..=0x11).collect();
        assert_eq!(W12A02_BUCKET_CAP, 0x10);

        // A roll of 16 wraps to the first kept candidate; a seventeenth
        // candidate would have been picked instead.
        let mut rows = build();
        assert_eq!(w12a02_pick_blocked_spot(&mut rows, &tiles, 16, 0), 1);
        // The roll selects inside the kept window.
        let mut rows = build();
        assert_eq!(w12a02_pick_blocked_spot(&mut rows, &tiles, 15, 0), 16);
        assert_eq!(W12A02_BUCKET_ROLL, 0x37);
    }

    #[test]
    fn any_placement_row_skips_row_zero() {
        let mut rows = vec![W12a02Row::default(); 4];
        assert!(!w12a02_any_placement_row(&rows));
        rows[0] = row(4, 0);
        assert!(!w12a02_any_placement_row(&rows));
        rows[3] = row(5, 0);
        assert!(w12a02_any_placement_row(&rows));
        rows[3] = row(3, 0);
        assert!(!w12a02_any_placement_row(&rows));
    }
}
