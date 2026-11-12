#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave13/agent03.rs
// ===========================================================================
// Wave 13 translation batch 03: player-command dispatch, region queries and
// per-player counter/relation updates recovered from
// `0x00660000..=0x00661fff`.
//
// This window sits on the client/AI boundary. A command record is passed to
// several routines that either dispatch an AI-script handler opcode
// (`0x0e..=0x1a`) or update one of the per-player counter tables through the
// obfuscated image base (`DAT_00f063cc ^ 0x10fae560`). The translations below
// keep only the value-level decisions: the table bases and record strides are
// documented as recovered layout, while the caller supplies the records.
//
// ## Recovered layout
//
// | offset | field |
// |--------|-------|
// | `+0x00` | 1-based player/owner slot (`u8`) |
// | `+0x04` | source word (`u32`) |
// | `+0x10` | player selector or AI-script opcode (`u32`) |
// | `+0x14` | region/script id or bitfield value (`u32`) |
// | `+0x18` | target word (`u16`) |
// | `+0x1a` | command id used by `0x006605d0` (`u8`) |
// | `+0x1b` | mode/repeat byte (`u8`) |
//
// Per-player area records live at `0xeb70 + i * 0x14`: `u16` words at
// `+0x00`, `+0x04`, `+0x08`, `+0x0c` and a marker word at `+0x12`. Region
// ids are 1-based for the query rows (`0xeb70 + (id - 1) * 0x14`), and the
// region rectangle is read as four `i32` at `+0x00..+0x0c`, so its centre is
// `(left + right) / 2, (top + bottom) / 2`.
//
// The relation matrix has `0xc`-byte rows based at `0x131b * 0xc` (`0xe544`).
// The command matrix has `0xc`-dword rows based at `0xb274` and accepts
// `0xe4` command rows. The pending-command record starts at `0xe61c`.
//
// Every routine below returns a source-level decision. The batch/query rows
// return the constant `1` (or fall into a non-returning consistency trap);
// the counter dispatchers `0x00661450`, `0x00661620`, `0x0066175a` and
// `0x00661990` return zero on every recovered path. Each routine documents
// which convention it models.
//
// ## Skipped rows
//
// * `FUN_00660234` - register-only `*in_EAX += (char)in_EAX` fragment; no
//   recoverable arguments.
// * `FUN_00660313` - `unaff_EBP` twin of `0x00660310`; the body is the same
//   routine viewed through the caller frame, so it is not translated twice.
// * `FUN_00660aab` - inline fragment over `in_*`/`unaff_*` registers and flags;
//   only the rectangle-clamp tail is visible.
// * `FUN_00660d7b` - `*(int *)(in_EAX + 1) += 1` increment fragment.
// * `FUN_00660d9e` - `in_EAX`/`unaff_ESI`/`unaff_BL` byte-add fragment.
// * `FUN_00660e5d` - `unaff_EBX`/`in_AL` fragment ending in a non-returning
//   runtime trap.
// * `FUN_006616dd` - register-only inline copy of the selector-`1` path of
//   `0x00661620`; its arguments arrive in `EAX`/`ECX`/`EDX`/`SI`.
// * `FUN_006619b9` - `in_EAX`/`unaff_ESI` stack-arithmetic fragment.
// * `FUN_00661db0` - callback whose body is a recursion through four
//   unrecovered calls plus non-returning tails.
// * `FUN_00661f10` - callback reduced to two opaque calls and a non-returning
//   tail; no value survives.
// * `FUN_00661f7d` - `in_EAX` indexes a global byte table at `DAT_00f92388`, then
//   the body calls opaque helpers and traps.

/// Number of per-player slots accepted by the counter routines (`7 < player`
/// is the exclusive guard used by `0x00661450` and `0x00661620`).
pub const W13A03_PLAYER_COUNT: usize = 8;

/// Stride of one per-player area record.
pub const W13A03_AREA_STRIDE: u32 = 0x14;

/// Base offset of the per-player area-record table.
pub const W13A03_AREA_BASE: u32 = 0xeb70;

/// Offset of the relation-matrix chunk; multiplied by the `0xc` row stride it
/// gives `0xe544`.
pub const W13A03_RELATION_CHUNK: u32 = 0x131b;

/// Byte stride of one relation-matrix row.
pub const W13A03_RELATION_STRIDE: usize = 0xc;

/// Number of relation-matrix rows and columns.
pub const W13A03_RELATION_SLOTS: usize = 0xc;

/// Image offset of the pending-command record (`0xe61c..=0xe627`).
pub const W13A03_PENDING_COMMAND_BASE: u32 = 0xe61c;

/// Image address returned by `0x00661410`.
pub const W13A03_COMMAND_TABLE_BASE: usize = 0x00fdf900;

/// Number of dword rows in the `0xb274` command matrix.
pub const W13A03_COMMAND_MATRIX_ROWS: usize = 0xe4;

/// Dwords per command-matrix row.
pub const W13A03_COMMAND_MATRIX_STRIDE: usize = 0xc;

/// One command record consumed by the dispatch rows in this window.
///
/// Only the fields touched by the translated routines are modelled. The
/// record is the first argument of `0x00660070`, `0x00660280`, `0x00660310`,
/// `0x00660440`, `0x00660580`, `0x006605d0`, `0x00660b90` and `0x00660d38`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03CommandRequest {
    /// `+0x00`: 1-based player/owner slot. Zero means "no player".
    pub player: u8,
    /// `+0x04`: source word copied into the pending command by
    /// `0x006605d0`.
    pub source: u32,
    /// `+0x10`: player selector (`< 8`) or AI-script handler opcode
    /// (`0x0e..=0x1a`).
    pub command: u32,
    /// `+0x14`: region/script id, bitfield value or counter amount.
    pub value: u32,
    /// `+0x18`: target word.
    pub target: u16,
    /// `+0x1a`: command byte inspected by `0x006605d0`.
    pub action: u8,
    /// `+0x1b`: mode/repeat byte.
    pub mode: u8,
}

/// One per-player area record (`u16` view of the `0xeb70` table).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03PlayerArea {
    /// `+0x00`: first extent word.
    pub left: u16,
    /// `+0x04`: second extent word.
    pub top: u16,
    /// `+0x08`: third extent word.
    pub right: u16,
    /// `+0x0c`: fourth extent word.
    pub bottom: u16,
    /// `+0x12`: marker/filter word.
    pub marker: u16,
}

/// One region rectangle (`i32` view used by the 1-based region queries).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03RegionRect {
    /// `+0x00`: left edge.
    pub left: i32,
    /// `+0x04`: top edge.
    pub top: i32,
    /// `+0x08`: right edge.
    pub right: i32,
    /// `+0x0c`: bottom edge.
    pub bottom: i32,
}

/// AI-script handler selected by an opcode in `0x0e..=0x1a`.
///
/// The variant documentation names the retail callee used by the dispatch
/// rows. The shared `0x12..=0x15` arm also serves `0x13`, `0x14` and `0x15`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03ScriptHandler {
    /// Opcode `0x0e`: `FUN_00664b60`.
    Script0e,
    /// Opcode `0x0f`: `FUN_00664700`.
    Script0f,
    /// Opcode `0x10`: `FUN_006650a0`.
    Script10,
    /// Opcode `0x11`: `FUN_00664610`.
    Script11,
    /// Opcodes `0x12..=0x15`: `FUN_00664ce0`.
    Script12To15,
    /// Opcode `0x1a`: `FUN_00664f50`.
    Script1a,
}

/// Source-level outcome shared by the dispatch rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03Outcome<T> {
    /// The row tail-calls the named AI-script handler.
    Delegate(W13a03ScriptHandler),
    /// The row updated a caller-owned table and reports the applied case.
    Applied(T),
    /// `0x00661450`'s `DAT_01165f58 != 0` branch: the legacy hook
    /// `FUN_006ae540` is called with the original arguments.
    LegacyHook,
    /// The row performed no table update and returned zero.
    Ignored,
}

/// Action selected for one iteration of the `0x00660070` batch loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03BatchAction {
    /// The actor slot is below `8`; retail calls `FUN_005ada00` and
    /// `FUN_005b1da0` with the target word and the centre of the actor's area
    /// record, then optionally `FUN_00712ce0` when the value at `+0x14` is in
    /// `1..=0x40`.
    Placement {
        /// Resolved player slot (`0x0d` is replaced by the current player).
        player: u32,
    },
    /// The opcode names a dedicated AI-script handler.
    Script(W13a03ScriptHandler),
    /// Opcodes `0x16..=0x19`, `0x08..=0x0c` and values above `0x1a` do
    /// nothing.
    None,
}

/// One plan for the `0x00660070` batch loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a03BatchPlan {
    /// Number of iterations: `+0x1b`, or `1` when that byte is zero.
    pub iterations: u8,
    /// Work performed on each iteration for a stable command record.
    pub action: W13a03BatchAction,
}

/// Post-batch handler selected by `0x00660280` from the `+0x1b` byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03Followup {
    /// Mode `7` selects `FUN_00662990`.
    Mode7,
    /// Mode `8` selects `0x00661450`, the player-command counter dispatch.
    Mode8,
    /// Mode `9` selects `FUN_00663290`.
    Mode9,
}

/// Pending-command record written by `0x006605d0` at `0xe61c`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03PendingCommand {
    /// `+0x00` (`0xe61c`): command id.
    pub command: u8,
    /// `+0x01` (`0xe61d`): zero-based player slot, only written by commands
    /// `0x12` and `0x22`.
    pub slot: u8,
    /// `+0x02` (`0xe61e`): target word.
    pub target: u16,
    /// `+0x04` (`0xe620`): value word.
    pub value: u32,
    /// `+0x08` (`0xe624`): source word.
    pub source: u32,
}

/// Region/script dispatch data computed by `0x00660580`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a03ScriptRegionRequest {
    /// Script id passed as the first argument of `FUN_00640940` (record `+0x14`).
    pub script_id: u32,
    /// Image offset of the per-player area record passed as the third
    /// argument.
    pub area_offset: u32,
}

/// Query record built by `0x00660310` for the callback `FUN_006620a0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03AreaQuery {
    /// Extent words copied from the resolved player area.
    pub area: W13a03PlayerArea,
    /// Command word from record `+0x10`.
    pub command: u32,
    /// Target word from record `+0x18`, zero-extended.
    pub target: u32,
    /// Marker word copied from the player area.
    pub marker: u16,
    /// Centre of the resolved region, `(left + right) / 2`.
    pub center_x: i32,
    /// Centre of the resolved region, `(top + bottom) / 2`.
    pub center_y: i32,
    /// Zero-extended mode byte from record `+0x1b`.
    pub mode: u32,
}

/// Query record built by `0x00660b90` for the callback `FUN_006625e0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03ScriptQuery {
    /// Extent words copied from the resolved player area.
    pub area: W13a03PlayerArea,
    /// Command word from record `+0x10`.
    pub command: u32,
    /// Target word from record `+0x18`.
    pub target: u16,
    /// Marker word copied from the player area.
    pub marker: u16,
    /// Last byte of the callback record: the low byte of `value - 1`, i.e. the
    /// zero-based region slot.
    pub region_slot: u8,
    /// Value written to the global replay limit `DAT_00fdfe0c`: the mode byte
    /// from record `+0x1b`, or `999999` when that byte is zero.
    pub replay_limit: u32,
}

/// Query record built by `0x00661d10` for the callback `FUN_00661db0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03RegionQuery {
    /// Extent words copied from the resolved player area.
    pub area: W13a03PlayerArea,
    /// Command word passed as the second argument.
    pub command: u32,
    /// Target word passed as the first argument.
    pub target: u16,
    /// Marker word copied from the player area.
    pub marker: u16,
    /// Fixed mode word (`1`).
    pub mode: u32,
}

/// One of the three `0xe6`/`0xe7`/`0xe8` counter arrays updated by
/// `0x00661450`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03TypeCounter {
    /// Selector `0xe6`, array `0x2d84`.
    At0x2d84,
    /// Selector `0xe7`, array `0x2ed4`.
    At0x2ed4,
    /// Selector `0xe8`, array `0x2ff4`.
    At0x2ff4,
}

/// Applied case of `0x00661450`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03CommandUpdate {
    /// One of the three type counters was incremented.
    TypeCounter(W13a03TypeCounter),
    /// The `0xb274` command matrix cell was incremented.
    Matrix,
}

/// Counter arrays updated by `0x00661450`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03CommandCounters {
    /// `0x2d84 + player * 4`: selector `0xe6` counts.
    pub at_0x2d84: [i32; W13A03_PLAYER_COUNT],
    /// `0x2ed4 + player * 4`: selector `0xe7` counts.
    pub at_0x2ed4: [i32; W13A03_PLAYER_COUNT],
    /// `0x2ff4 + player * 4`: selector `0xe8` counts.
    pub at_0x2ff4: [i32; W13A03_PLAYER_COUNT],
}

/// The `0xb274` command matrix: `0xe4` rows of `0xc` dwords.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13a03CommandMatrix {
    /// `0xb274 + (player + command * 0xc) * 4`.
    pub rows: [[i32; W13A03_COMMAND_MATRIX_STRIDE]; W13A03_COMMAND_MATRIX_ROWS],
}

impl Default for W13a03CommandMatrix {
    fn default() -> Self {
        Self {
            rows: [[0; W13A03_COMMAND_MATRIX_STRIDE]; W13A03_COMMAND_MATRIX_ROWS],
        }
    }
}

/// Applied selector of `0x00661620`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03SelectionSet {
    /// Selector `0`: `0x00` and `0x90` were incremented.
    Selector0,
    /// Selector `1`: `0x30` and `0x60` were incremented.
    Selector1,
    /// Selector `2`: all four arrays were incremented.
    Selector2,
}

/// Counter arrays updated by `0x00661620`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03SelectionCounters {
    /// `0x00 + player * 4`.
    pub at_0x00: [i32; W13A03_PLAYER_COUNT],
    /// `0x30 + player * 4`.
    pub at_0x30: [i32; W13A03_PLAYER_COUNT],
    /// `0x60 + player * 4`.
    pub at_0x60: [i32; W13A03_PLAYER_COUNT],
    /// `0x90 + player * 4`.
    pub at_0x90: [i32; W13A03_PLAYER_COUNT],
}

/// Applied selector of `0x0066175a`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03RatioSet {
    /// Selector `0`: a quarter of the amount reached all four quadrant
    /// arrays (`0x2de4`, `0x2e14`, `0x2f34`, `0x2f64`).
    QuarterAll,
    /// Selector `1`: the whole amount reached `0x2de4`.
    WholeA,
    /// Selector `2`: the whole amount reached `0x2f34`.
    WholeC,
    /// Selector `3`: half the amount reached `0x2de4` and `0x2f34`.
    HalfAC,
    /// Selector `4`: the whole amount reached `0x2e14`.
    WholeB,
    /// Selector `5`: the whole amount reached `0x2f64`.
    WholeD,
    /// Selector `6`: half the amount reached `0x2e14` and `0x2f64`.
    HalfBD,
    /// Selector `7`: the whole amount reached `0x3204`.
    WholeE,
}

/// Counter arrays updated by `0x0066175a`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03RatioCounters {
    /// `0x2de4 + player * 4`.
    pub at_0x2de4: [i32; W13A03_PLAYER_COUNT],
    /// `0x2e14 + player * 4`.
    pub at_0x2e14: [i32; W13A03_PLAYER_COUNT],
    /// `0x2f34 + player * 4`.
    pub at_0x2f34: [i32; W13A03_PLAYER_COUNT],
    /// `0x2f64 + player * 4`.
    pub at_0x2f64: [i32; W13A03_PLAYER_COUNT],
    /// `0x3204 + player * 4`.
    pub at_0x3204: [i32; W13A03_PLAYER_COUNT],
}

/// Result of the relation-matrix update in `0x00661990`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13a03RelationWrite {
    /// Relation byte written to the actor's row: `0`, `1` or `2`.
    pub relation: u8,
    /// Whether the non-zero relation was propagated across the actor's row.
    pub propagated: bool,
}

/// `0x00660070` (`FUN_00660070`, 295 bytes): plan one execution of the
/// per-player command batch.
///
/// The row returns the constant `1` and does nothing when the record's `+0x00`
/// player byte is zero; `None` reports that case. Otherwise it saves the dword
/// returned by `FUN_0073f110`, writes `0x75bcd15` through `FUN_0073f120` (the
/// same global), restores the saved value at the end, and loops `+0x1b` times,
/// defaulting that byte to `1` when it is zero.
///
/// Each iteration re-reads `+0x10` and decides:
///
/// * `0x0d` substitutes the current-player global (`FUN_007a9d00`) and then
///   takes the placement path when the substitute is below `8`;
/// * `0x00..=0x07` take the placement path directly;
/// * `0x0e`, `0x0f`, `0x10`, `0x11`, `0x12..=0x15` and `0x1a` delegate to the
///   AI-script handlers named by [`W13a03ScriptHandler`];
/// * `0x16..=0x19` and every other value do nothing.
///
/// The placement path itself depends on a unit lookup (`FUN_005ada00` /
/// `FUN_005b1da0`) and a callback (`FUN_00712ce0`); only the dispatch decision
/// is reported here.
pub fn w13a03_batch_plan(
    record_player: u8,
    mode: u8,
    command: u32,
    current_player: u32,
) -> Option<W13a03BatchPlan> {
    if record_player == 0 {
        return None;
    }
    let iterations = if mode == 0 { 1 } else { mode };
    let action = if command == 0x0d {
        if current_player < W13A03_PLAYER_COUNT as u32 {
            W13a03BatchAction::Placement {
                player: current_player,
            }
        } else {
            W13a03BatchAction::None
        }
    } else if command < W13A03_PLAYER_COUNT as u32 {
        W13a03BatchAction::Placement { player: command }
    } else {
        match w13a03_script_handler(command) {
            Some(handler) => W13a03BatchAction::Script(handler),
            None => W13a03BatchAction::None,
        }
    };
    Some(W13a03BatchPlan { iterations, action })
}

/// Shared opcode-to-handler mapping used by the dispatch rows.
fn w13a03_script_handler(opcode: u32) -> Option<W13a03ScriptHandler> {
    match opcode {
        0x0e => Some(W13a03ScriptHandler::Script0e),
        0x0f => Some(W13a03ScriptHandler::Script0f),
        0x10 => Some(W13a03ScriptHandler::Script10),
        0x11 => Some(W13a03ScriptHandler::Script11),
        0x12..=0x15 => Some(W13a03ScriptHandler::Script12To15),
        0x1a => Some(W13a03ScriptHandler::Script1a),
        _ => None,
    }
}

/// `0x00660280` (`FUN_00660280`, 143 bytes): post-batch follow-up dispatch.
///
/// The row runs the lifecycle call `FUN_006ae4e0`, reads the mode byte at
/// `+0x1b`, and selects:
///
/// * `7` -> `FUN_00662990`;
/// * `8` -> `0x00661450` (player-command counter dispatch);
/// * `9` -> `FUN_00663290`.
///
/// All paths then call `FUN_006ae520` and return `1`. Modes outside `7..=9`
/// still run the lifecycle calls but hit no handler.
pub fn w13a03_batch_followup(mode: u8) -> Option<W13a03Followup> {
    match mode {
        7 => Some(W13a03Followup::Mode7),
        8 => Some(W13a03Followup::Mode8),
        9 => Some(W13a03Followup::Mode9),
        _ => None,
    }
}

/// `0x00660310` (`FUN_00660310`, 3-byte entry / shared body): build the
/// area-query record passed to `FUN_006c2670` with callback `FUN_006620a0`.
///
/// The row requires a non-zero player byte at `+0x00` and a non-zero region id
/// at `+0x14`. It copies the player area record at
/// `0xeb70 + (player - 1) * 0x14` (four `u16` extent words plus the `+0x12`
/// marker), takes the region rectangle at `0xeb70 + (region - 1) * 0x14`
/// (four `i32` edges), and computes the rectangle centre:
///
/// ```c
/// x = (region.right + region.left) / 2;
/// y = (region.bottom + region.top) / 2;
/// ```
///
/// The query also carries the command word at `+0x10`, the zero-extended
/// target at `+0x18` and the mode byte at `+0x1b`. The callback itself is not
/// modelled.
pub fn w13a03_area_query(
    areas: &[W13a03PlayerArea],
    regions: &[W13a03RegionRect],
    request: &W13a03CommandRequest,
) -> Option<W13a03AreaQuery> {
    if request.player == 0 || request.value == 0 {
        return None;
    }
    let area = *areas.get(usize::from(request.player - 1))?;
    let region_index = usize::try_from((request.value as i32).checked_sub(1)?).ok()?;
    let region = *regions.get(region_index)?;
    Some(W13a03AreaQuery {
        area,
        command: request.command,
        target: u32::from(request.target),
        marker: area.marker,
        center_x: region.left.wrapping_add(region.right) / 2,
        center_y: region.top.wrapping_add(region.bottom) / 2,
        mode: u32::from(request.mode),
    })
}

/// `0x00660440` (`FUN_00660440`, 173 bytes): update one bit of a per-player
/// dword array based at `0xeb50`.
///
/// The value at `+0x14` must be below `0x100`. Word index is `value >> 5`,
/// bit index is `value & 0x1f`, and the mode byte at `+0x1b` selects:
///
/// * `4`: set the bit;
/// * `5`: clear the bit;
/// * `6`: toggle the bit;
/// * `0x0b`: clear the bit when `probe_ok`, otherwise set it. `probe_ok` is
///   the result of `FUN_0073f3b0(0x39) >= 0`;
/// * anything else: no change.
///
/// Returns `true` when a word was touched. The retail row always returns `1`.
pub fn w13a03_update_player_bit(
    words: &mut [u32; W13A03_PLAYER_COUNT],
    value: u32,
    mode: u8,
    probe_ok: bool,
) -> bool {
    if value >= 0x100 {
        return false;
    }
    let word = (value >> 5) as usize;
    let bit = value & 0x1f;
    let mask = 1u32 << bit;
    match mode {
        0x04 => words[word] |= mask,
        0x05 => words[word] &= !mask,
        0x06 => words[word] ^= mask,
        0x0b => {
            if probe_ok {
                words[word] &= !mask;
            } else {
                words[word] |= mask;
            }
        }
        _ => return false,
    }
    true
}

/// `0x006604c8` (`FUN_006604c8`, 8 bytes): constant success result.
///
/// The whole body is `return 1;`. It is the default handler installed for a
/// command-card row that performs no work.
pub fn w13a03_default_success_a() -> u32 {
    1
}

/// `0x00660580` (`FUN_00660580`, 72 bytes): script-dispatch request.
///
/// With a non-zero player byte at `+0x00`, the row reads the script id at
/// `+0x14` and computes the third argument of `FUN_00640940` as
/// `(player - 1) * 5 * 4 + 0x3adc * 4`, i.e.
/// `0xeb70 + (player - 1) * 0x14`. The second argument is the current-player
/// global (`FUN_007a9d00`), which is not part of this value helper.
pub fn w13a03_script_region_request(
    player: u8,
    script_id: u32,
) -> Option<W13a03ScriptRegionRequest> {
    if player == 0 {
        return None;
    }
    Some(W13a03ScriptRegionRequest {
        script_id,
        area_offset: W13A03_AREA_BASE + u32::from(player - 1) * W13A03_AREA_STRIDE,
    })
}

/// `0x006605d0` (`FUN_006605d0`, 200 bytes): write the pending-command record.
///
/// The preamble call `FUN_007aaf50` is engine state and is not modelled; the
/// row then branches on the command byte at `+0x1a`:
///
/// * `0x12` / `0x22`: a zero player byte at `+0x00` aborts; otherwise the
///   zero-based slot `player - 1` is stored at `0xe61d`, and the common tail
///   runs;
/// * `0x28`: `0xe61c = 0x28`, `0xe620 = value`, `0xe61e = 2`, then return;
/// * any other command: the common tail runs.
///
/// The common tail writes `0xe61c = command`, `0xe61e = target`,
/// `0xe620 = value` and `0xe624 = source` (record `+0x04`).
///
/// Returns `false` only for the aborted `0x12`/`0x22` zero-player case. The
/// retail row returns `1` on every path.
pub fn w13a03_write_pending_command(
    pending: &mut W13a03PendingCommand,
    request: &W13a03CommandRequest,
) -> bool {
    match request.action {
        0x12 | 0x22 => {
            if request.player == 0 {
                return false;
            }
            pending.slot = request.player - 1;
        }
        0x28 => {
            pending.command = 0x28;
            pending.target = 2;
            pending.value = request.value;
            return true;
        }
        _ => {}
    }
    pending.command = request.action;
    pending.target = request.target;
    pending.value = request.value;
    pending.source = request.source;
    true
}

/// `0x00660820` (`FUN_00660820`, 7 bytes): constant success result.
///
/// The whole body is `return 1;`; it is a second no-work handler in the same
/// dispatch family as `0x006604c8`.
pub fn w13a03_default_success_b() -> u32 {
    1
}

/// `0x0066098d` (`FUN_0066098d`, 27 bytes): current-player gate.
///
/// The row compares the current-player global (`FUN_007a9d00`) with
/// `DAT_00f2acf4` and calls `FUN_006e4f30` when they are equal, then returns
/// `1`. `true` reports that the conditional call is taken.
pub fn w13a03_current_player_gate(current: u32, expected: u32) -> bool {
    current == expected
}

/// `0x00660b90` (`FUN_00660b90`, 209 bytes): build the script query passed to
/// `FUN_006c2670` with callback `FUN_006625e0`.
///
/// The row requires a non-zero player byte at `+0x00` and a non-zero word at
/// `+0x14`. It copies the player area record at
/// `0xeb70 + (player - 1) * 0x14`. The callback record carries the command
/// word at `+0x10`, the target word at `+0x18`, the area marker, and the low
/// byte of `value - 1` as its last byte (the zero-based region slot). The
/// mode byte at `+0x1b` is written to the global replay limit
/// `DAT_00fdfe0c`, or `999999` when that byte is zero. After the callback the
/// row calls `FUN_00664810` and falls into the non-returning consistency trap
/// `FUN_00c86b6e`.
pub fn w13a03_script_query(
    areas: &[W13a03PlayerArea],
    request: &W13a03CommandRequest,
) -> Option<W13a03ScriptQuery> {
    if request.player == 0 || request.value == 0 {
        return None;
    }
    let area = *areas.get(usize::from(request.player - 1))?;
    let replay_limit = if request.mode == 0 {
        999_999
    } else {
        u32::from(request.mode)
    };
    Some(W13a03ScriptQuery {
        area,
        command: request.command,
        target: request.target,
        marker: area.marker,
        region_slot: (request.value as u8).wrapping_sub(1),
        replay_limit,
    })
}

/// `0x00660d38` (`FUN_00660d38`, 63 bytes): target-command dispatch.
///
/// After masking a global with an unrecovered call result, the row returns
/// immediately unless the player byte at `+0x00` is non-zero and the region
/// query `0x00661d10(target at +0x18, command at +0x10, player - 1)` succeeds.
/// It then switches on the mode byte at `+0x1b`:
///
/// * `0` and `6`: call `FUN_005b69b0`;
/// * `4`: call `FUN_005b4880`;
/// * `5`: call `FUN_005b3bf0`;
/// * anything else: no handler.
///
/// The global-mask update and the region-query guard are not modelled; this
/// helper reports only the recovered mode dispatch.
pub fn w13a03_target_mode_handler(mode: u8) -> Option<W13a03TargetHandler> {
    match mode {
        0 | 6 => Some(W13a03TargetHandler::Mode0Or6),
        4 => Some(W13a03TargetHandler::Mode4),
        5 => Some(W13a03TargetHandler::Mode5),
        _ => None,
    }
}

/// Handler selected by [`w13a03_target_mode_handler`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13a03TargetHandler {
    /// Mode `0`/`6`: `FUN_005b69b0`.
    Mode0Or6,
    /// Mode `4`: `FUN_005b4880`.
    Mode4,
    /// Mode `5`: `FUN_005b3bf0`.
    Mode5,
}

/// `0x00661410` (`FUN_00661410`, 6 bytes): command-table base address.
///
/// The row is a constant address getter for `0x00fdf900`; it reads no memory.
pub fn w13a03_command_table_base() -> usize {
    W13A03_COMMAND_TABLE_BASE
}

/// `0x00661450` (`FUN_00661450`, 129 bytes): apply one player-command effect.
///
/// The first argument is a player selector with two special forms: `0x0d`
/// substitutes the current player, and `0x0e..=0x1a` name AI-script handlers.
/// `0x16..=0x19` return immediately. The remaining values fall through to the
/// low 16 bits of the second argument:
///
/// * `0xe5`: no update;
/// * `0xe6` / `0xe7` / `0xe8`: when the player is below `8`, add the amount to
///   `0x2d84` / `0x2ed4` / `0x2ff4` respectively;
/// * otherwise, when the legacy global `DAT_01165f58` is non-zero, call
///   `FUN_006ae540` and stop; when the player is below `8` and the command is at
///   most `0xe3`, add the amount to
///   `0xb274 + (player + command * 0xc) * 4`.
///
/// Every recovered path returns zero in retail; [`W13a03Outcome`] reports the
/// selected action instead.
pub fn w13a03_apply_player_counter(
    player: u32,
    selector: u16,
    amount: i32,
    current_player: u32,
    legacy_hook: bool,
    counters: &mut W13a03CommandCounters,
    matrix: &mut W13a03CommandMatrix,
) -> W13a03Outcome<W13a03CommandUpdate> {
    // The retail switch inspects the original selector; only the fall-through
    // uses the substituted current player, so do not re-dispatch it.
    if player != 0x0d && (0x0e..=0x1a).contains(&player) {
        return match w13a03_script_handler(player) {
            Some(handler) => W13a03Outcome::Delegate(handler),
            None => W13a03Outcome::Ignored,
        };
    }
    let player = if player == 0x0d {
        current_player
    } else {
        player
    };
    let player_index = usize::try_from(player).ok();
    match selector {
        0xe5 => return W13a03Outcome::Ignored,
        0xe6 | 0xe7 | 0xe8 => {
            let Some(index) = player_index.filter(|&i| i < W13A03_PLAYER_COUNT) else {
                return W13a03Outcome::Ignored;
            };
            let (array, kind) = match selector {
                0xe6 => (&mut counters.at_0x2d84, W13a03TypeCounter::At0x2d84),
                0xe7 => (&mut counters.at_0x2ed4, W13a03TypeCounter::At0x2ed4),
                _ => (&mut counters.at_0x2ff4, W13a03TypeCounter::At0x2ff4),
            };
            array[index] = array[index].wrapping_add(amount);
            return W13a03Outcome::Applied(W13a03CommandUpdate::TypeCounter(kind));
        }
        _ => {}
    }
    if legacy_hook {
        return W13a03Outcome::LegacyHook;
    }
    let Some(index) = player_index.filter(|&i| i < W13A03_PLAYER_COUNT) else {
        return W13a03Outcome::Ignored;
    };
    if selector > 0xe3 {
        return W13a03Outcome::Ignored;
    }
    let cell = &mut matrix.rows[usize::from(selector)][index];
    *cell = cell.wrapping_add(amount);
    W13a03Outcome::Applied(W13a03CommandUpdate::Matrix)
}

/// `0x00661620` (`FUN_00661620`, 50 bytes): apply a selection counter effect.
///
/// The dispatch header is the same as [`w13a03_apply_player_counter`]
/// (`0x0d` substitutes the current player; `0x0e..=0x1a` delegate;
/// `0x16..=0x19` stop). The low 16 bits of the second argument then select:
///
/// * `0`: add the amount to `0x00` and `0x90`;
/// * `1`: add the amount to `0x30` and `0x60`;
/// * `2`: add the amount to all four arrays;
/// * other values: no update.
///
/// The header also requires a player below `8`. Retail returns zero on every
/// path.
pub fn w13a03_apply_selection_counter(
    player: u32,
    selector: u16,
    amount: i32,
    current_player: u32,
    counters: &mut W13a03SelectionCounters,
) -> W13a03Outcome<W13a03SelectionSet> {
    // The retail switch inspects the original selector; only the fall-through
    // uses the substituted current player, so do not re-dispatch it.
    if player != 0x0d && (0x0e..=0x1a).contains(&player) {
        return match w13a03_script_handler(player) {
            Some(handler) => W13a03Outcome::Delegate(handler),
            None => W13a03Outcome::Ignored,
        };
    }
    let player = if player == 0x0d {
        current_player
    } else {
        player
    };
    let Some(index) = usize::try_from(player)
        .ok()
        .filter(|&i| i < W13A03_PLAYER_COUNT)
    else {
        return W13a03Outcome::Ignored;
    };
    match selector {
        0 => {
            counters.at_0x00[index] = counters.at_0x00[index].wrapping_add(amount);
            counters.at_0x90[index] = counters.at_0x90[index].wrapping_add(amount);
            W13a03Outcome::Applied(W13a03SelectionSet::Selector0)
        }
        1 => {
            counters.at_0x30[index] = counters.at_0x30[index].wrapping_add(amount);
            counters.at_0x60[index] = counters.at_0x60[index].wrapping_add(amount);
            W13a03Outcome::Applied(W13a03SelectionSet::Selector1)
        }
        2 => {
            counters.at_0x00[index] = counters.at_0x00[index].wrapping_add(amount);
            counters.at_0x90[index] = counters.at_0x90[index].wrapping_add(amount);
            counters.at_0x30[index] = counters.at_0x30[index].wrapping_add(amount);
            counters.at_0x60[index] = counters.at_0x60[index].wrapping_add(amount);
            W13a03Outcome::Applied(W13a03SelectionSet::Selector2)
        }
        _ => W13a03Outcome::Ignored,
    }
}

/// `0x0066175a` (`FUN_0066175a`, 64 bytes): apply a ratio counter effect.
///
/// The dispatch header matches the other player-command rows. For a player
/// below `8`, the low 16 bits of the second argument select:
///
/// | selector | effect |
/// |----------|--------|
/// | `0` | quarter `(amount + 3) >> 2` to `0x2de4`, `0x2f34`, `0x2e14`, `0x2f64` |
/// | `1` | whole amount to `0x2de4` |
/// | `2` | whole amount to `0x2f34` |
/// | `3` | half `(amount + 1) >> 1` to `0x2de4` and `0x2f34` |
/// | `4` | whole amount to `0x2e14` |
/// | `5` | whole amount to `0x2f64` |
/// | `6` | half `(amount + 1) >> 1` to `0x2e14` and `0x2f64` |
/// | `7` | whole amount to `0x3204` |
///
/// The quarter/half sums use unsigned 32-bit arithmetic in retail. Retail
/// returns zero on every path.
pub fn w13a03_apply_ratio_counter(
    player: u32,
    selector: u16,
    amount: i32,
    current_player: u32,
    counters: &mut W13a03RatioCounters,
) -> W13a03Outcome<W13a03RatioSet> {
    // The retail switch inspects the original selector; only the fall-through
    // uses the substituted current player, so do not re-dispatch it.
    if player != 0x0d && (0x0e..=0x1a).contains(&player) {
        return match w13a03_script_handler(player) {
            Some(handler) => W13a03Outcome::Delegate(handler),
            None => W13a03Outcome::Ignored,
        };
    }
    let player = if player == 0x0d {
        current_player
    } else {
        player
    };
    let Some(index) = usize::try_from(player)
        .ok()
        .filter(|&i| i < W13A03_PLAYER_COUNT)
    else {
        return W13a03Outcome::Ignored;
    };
    let add = |array: &mut [i32; W13A03_PLAYER_COUNT], value: u32| {
        array[index] = array[index].wrapping_add(value as i32);
    };
    let quarter = (amount as u32).wrapping_add(3) >> 2;
    let half = (amount as u32).wrapping_add(1) >> 1;
    let set = match selector {
        0 => {
            add(&mut counters.at_0x2de4, quarter);
            add(&mut counters.at_0x2f34, quarter);
            add(&mut counters.at_0x2e14, quarter);
            add(&mut counters.at_0x2f64, quarter);
            W13a03RatioSet::QuarterAll
        }
        1 => {
            add(&mut counters.at_0x2de4, amount as u32);
            W13a03RatioSet::WholeA
        }
        2 => {
            add(&mut counters.at_0x2f34, amount as u32);
            W13a03RatioSet::WholeC
        }
        3 => {
            add(&mut counters.at_0x2de4, half);
            add(&mut counters.at_0x2f34, half);
            W13a03RatioSet::HalfAC
        }
        4 => {
            add(&mut counters.at_0x2e14, amount as u32);
            W13a03RatioSet::WholeB
        }
        5 => {
            add(&mut counters.at_0x2f64, amount as u32);
            W13a03RatioSet::WholeD
        }
        6 => {
            add(&mut counters.at_0x2e14, half);
            add(&mut counters.at_0x2f64, half);
            W13a03RatioSet::HalfBD
        }
        7 => {
            add(&mut counters.at_0x3204, amount as u32);
            W13a03RatioSet::WholeE
        }
        _ => return W13a03Outcome::Ignored,
    };
    W13a03Outcome::Applied(set)
}

/// `0x00661990` (`FUN_00661990`, 81 bytes): write one relation-matrix cell.
///
/// The row runs only for a first argument below `0x1b`. `0x0d`, `0x16..=0x19`
/// do nothing; `0x0e..=0x15` and `0x1a` delegate to the AI-script handlers.
/// Every other value is a zero-based relation slot:
///
/// * the target slot equal to the current player is skipped;
/// * the relation byte is `1` for selector `1`, `2` for selector `2`, and `0`
///   otherwise;
/// * the byte is written to `matrix[current][slot]`;
/// * when the relation is non-zero and `matrix[current][current]` differs,
///   every non-zero cell in the current player's row is overwritten with the
///   relation.
///
/// The matrix rows are `0xc` bytes based at `0xe544`. The row is passed as a
/// flat byte slice because the `+0xc` slot aliases the next row in retail.
/// The retail body re-reads the current-player global for the self-relation
/// test and for every column of the propagation loop; this helper assumes the
/// value is stable for the duration of the call. The row returns zero on every
/// path.
pub fn w13a03_apply_relation(
    player: u32,
    selector: u16,
    current_player: u32,
    cells: &mut [u8],
) -> W13a03Outcome<W13a03RelationWrite> {
    if player >= 0x1b {
        return W13a03Outcome::Ignored;
    }
    match player {
        0x0d | 0x16..=0x19 => return W13a03Outcome::Ignored,
        0x0e..=0x15 | 0x1a => {
            if let Some(handler) = w13a03_script_handler(player) {
                return W13a03Outcome::Delegate(handler);
            }
            return W13a03Outcome::Ignored;
        }
        _ => {}
    }
    if player == current_player {
        return W13a03Outcome::Ignored;
    }
    let actor = usize::try_from(current_player).ok();
    let target = usize::try_from(player).ok();
    let (Some(actor), Some(target)) = (actor, target) else {
        return W13a03Outcome::Ignored;
    };
    if actor >= W13A03_RELATION_SLOTS || cells.len() < W13A03_RELATION_SLOTS * W13A03_RELATION_SLOTS
    {
        return W13a03Outcome::Ignored;
    }
    let row = actor * W13A03_RELATION_STRIDE;
    let cell = row + target;
    if cell >= cells.len() {
        return W13a03Outcome::Ignored;
    }
    let relation = match selector {
        1 => 1u8,
        2 => 2u8,
        _ => 0u8,
    };
    cells[cell] = relation;
    let mut propagated = false;
    if relation != 0 && cells[row + actor] != relation {
        for column in 0..W13A03_RELATION_SLOTS {
            if cells[row + column] != 0 {
                cells[row + column] = relation;
            }
        }
        propagated = true;
    }
    W13a03Outcome::Applied(W13a03RelationWrite {
        relation,
        propagated,
    })
}

/// `0x00661b40` (`FUN_00661b40`, 40 bytes): text-driven time budget.
///
/// A null pointer returns `0`. Otherwise the row measures the C string length,
/// multiplies by `0x32`, and clamps values below `0xfa1` up to `4000`:
///
/// ```c
/// uVar3 = strlen(text) * 50;
/// if (uVar3 < 4001) uVar3 = 4000;
/// ```
pub fn w13a03_text_budget(text: Option<&[u8]>) -> u32 {
    let Some(bytes) = text else {
        return 0;
    };
    let length = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    let scaled = (length as u32).wrapping_mul(0x32);
    if scaled < 0x0fa1 { 4000 } else { scaled }
}

/// `0x00661d10` (`FUN_00661d10`, 154 bytes): build the region query passed to
/// `FUN_006c2670` with callback `FUN_00661db0`.
///
/// `player_index` is already zero-based: the call site `0x00660d38` passes
/// `record.player - 1`. The row copies the area record at
/// `0xeb70 + player_index * 0x14`, carries the caller's target word and command
/// word, and uses the fixed mode byte `1`. It then falls into the
/// non-returning consistency trap `FUN_00c86b6e`.
pub fn w13a03_region_query(
    areas: &[W13a03PlayerArea],
    player_index: u8,
    target: u16,
    command: u32,
) -> Option<W13a03RegionQuery> {
    let area = *areas.get(usize::from(player_index))?;
    Some(W13a03RegionQuery {
        area,
        command,
        target,
        marker: area.marker,
        mode: 1,
    })
}

#[cfg(test)]
mod tests_wave13_agent03 {
    use super::*;

    fn sample_area(marker: u16) -> W13a03PlayerArea {
        W13a03PlayerArea {
            left: 0x10,
            top: 0x20,
            right: 0x30,
            bottom: 0x40,
            marker,
        }
    }

    fn sample_request(player: u8, command: u32, value: u32) -> W13a03CommandRequest {
        W13a03CommandRequest {
            player,
            source: 0x1122_3344,
            command,
            value,
            target: 0x5566,
            action: 0x04,
            mode: 0,
        }
    }

    #[test]
    fn batch_plan_matches_0x00660070() {
        // A zero player byte aborts before the loop.
        assert_eq!(w13a03_batch_plan(0, 0, 0, 0), None);

        let zero_mode = w13a03_batch_plan(1, 0, 0, 0).unwrap();
        assert_eq!(zero_mode.iterations, 1);
        assert_eq!(zero_mode.action, W13a03BatchAction::Placement { player: 0 });

        assert_eq!(w13a03_batch_plan(1, 9, 5, 0).unwrap().iterations, 9);

        // 0x0d is replaced by the current player.
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x0d, 3).unwrap().action,
            W13a03BatchAction::Placement { player: 3 }
        );
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x0d, 8).unwrap().action,
            W13a03BatchAction::None
        );

        assert_eq!(
            w13a03_batch_plan(1, 1, 0x0e, 0).unwrap().action,
            W13a03BatchAction::Script(W13a03ScriptHandler::Script0e)
        );
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x12, 0).unwrap().action,
            W13a03BatchAction::Script(W13a03ScriptHandler::Script12To15)
        );
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x1a, 0).unwrap().action,
            W13a03BatchAction::Script(W13a03ScriptHandler::Script1a)
        );
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x16, 0).unwrap().action,
            W13a03BatchAction::None
        );
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x0c, 0).unwrap().action,
            W13a03BatchAction::None
        );
        assert_eq!(
            w13a03_batch_plan(1, 1, 0x1b, 0).unwrap().action,
            W13a03BatchAction::None
        );
    }

    #[test]
    fn batch_followup_matches_0x00660280() {
        assert_eq!(w13a03_batch_followup(7), Some(W13a03Followup::Mode7));
        assert_eq!(w13a03_batch_followup(8), Some(W13a03Followup::Mode8));
        assert_eq!(w13a03_batch_followup(9), Some(W13a03Followup::Mode9));
        for mode in [0, 6, 10, 0xff] {
            assert_eq!(w13a03_batch_followup(mode), None);
        }
    }

    #[test]
    fn area_query_matches_0x00660310() {
        let areas = [sample_area(7); 8];
        let regions = [
            W13a03RegionRect {
                left: 10,
                top: 20,
                right: 30,
                bottom: 40,
            },
            W13a03RegionRect {
                left: -9,
                top: -3,
                right: 5,
                bottom: 9,
            },
        ];
        let request = sample_request(2, 0x33, 2);

        let query = w13a03_area_query(&areas, &regions, &request).unwrap();
        assert_eq!(query.area, sample_area(7));
        assert_eq!(query.command, 0x33);
        assert_eq!(query.target, 0x5566);
        assert_eq!(query.marker, 7);
        assert_eq!(query.mode, 0);
        // Region 2 is regions[1]; C division truncates toward zero.
        assert_eq!(query.center_x, (-9 + 5) / 2);
        assert_eq!(query.center_y, (-3 + 9) / 2);

        assert_eq!(
            w13a03_area_query(&areas, &regions, &sample_request(0, 0, 1)),
            None
        );
        assert_eq!(
            w13a03_area_query(&areas, &regions, &sample_request(1, 0, 0)),
            None
        );
        assert_eq!(
            w13a03_area_query(&areas, &regions, &sample_request(1, 0, 3)),
            None
        );
        assert_eq!(
            w13a03_area_query(&areas, &regions, &sample_request(9, 0, 1)),
            None
        );
    }

    #[test]
    fn player_bit_update_matches_0x00660440() {
        let mut words = [0u32; 8];
        assert!(w13a03_update_player_bit(&mut words, 0x25, 4, false));
        assert_eq!(words[1], 1 << 5);
        assert!(w13a03_update_player_bit(&mut words, 0x25, 6, false));
        assert_eq!(words[1], 0);
        assert!(w13a03_update_player_bit(&mut words, 0xdf, 4, false));
        assert_eq!(words[6], 1 << 0x1f);
        assert!(w13a03_update_player_bit(&mut words, 0xdf, 5, false));
        assert_eq!(words[6], 0);

        // Mode 0x0b clears only when the probe succeeds.
        assert!(w13a03_update_player_bit(&mut words, 0, 4, false));
        assert!(w13a03_update_player_bit(&mut words, 0, 0x0b, true));
        assert_eq!(words[0], 0);
        assert!(w13a03_update_player_bit(&mut words, 0, 0x0b, false));
        assert_eq!(words[0], 1);

        // Out-of-range value and unknown mode leave the array alone.
        let before = words;
        assert!(!w13a03_update_player_bit(&mut words, 0x100, 4, false));
        assert!(!w13a03_update_player_bit(&mut words, 1, 3, false));
        assert_eq!(words, before);
    }

    #[test]
    fn success_stubs_match_0x006604c8_and_0x00660820() {
        assert_eq!(w13a03_default_success_a(), 1);
        assert_eq!(w13a03_default_success_b(), 1);
    }

    #[test]
    fn script_region_request_matches_0x00660580() {
        assert_eq!(w13a03_script_region_request(0, 7), None);
        let first = w13a03_script_region_request(1, 0x1234).unwrap();
        assert_eq!(first.script_id, 0x1234);
        assert_eq!(first.area_offset, 0xeb70);
        let last = w13a03_script_region_request(8, 9).unwrap();
        assert_eq!(last.area_offset, 0xeb70 + 7 * 0x14);
    }

    #[test]
    fn pending_command_matches_0x006605d0() {
        assert_eq!(W13A03_PENDING_COMMAND_BASE, 0xe61c);
        let mut pending = W13a03PendingCommand::default();
        let mut request = sample_request(3, 0, 0x99);
        request.action = 0x12;
        assert!(w13a03_write_pending_command(&mut pending, &request));
        assert_eq!(pending.slot, 2);
        assert_eq!(pending.command, 0x12);
        assert_eq!(pending.target, 0x5566);
        assert_eq!(pending.value, 0x99);
        assert_eq!(pending.source, 0x1122_3344);

        // The 0x12/0x22 path aborts for a zero player.
        let mut untouched = W13a03PendingCommand {
            command: 0xaa,
            slot: 0xbb,
            target: 0xcccc,
            value: 0xdddd,
            source: 0xeeee,
        };
        request.action = 0x22;
        request.player = 0;
        assert!(!w13a03_write_pending_command(&mut untouched, &request));
        assert_eq!(untouched.command, 0xaa);

        // Command 0x28 fixes the target at 2 and leaves the source alone.
        let mut fixed = W13a03PendingCommand::default();
        request.action = 0x28;
        request.player = 1;
        request.target = 0x7777;
        request.source = 0x8888;
        assert!(w13a03_write_pending_command(&mut fixed, &request));
        assert_eq!(fixed.command, 0x28);
        assert_eq!(fixed.target, 2);
        assert_eq!(fixed.value, 0x99);
        assert_eq!(fixed.source, 0);
        assert_eq!(fixed.slot, 0);

        // Any other command runs the common tail.
        let mut common = W13a03PendingCommand::default();
        request.action = 0x05;
        assert!(w13a03_write_pending_command(&mut common, &request));
        assert_eq!(common.command, 0x05);
        assert_eq!(common.target, 0x7777);
        assert_eq!(common.source, 0x8888);
    }

    #[test]
    fn current_player_gate_matches_0x0066098d() {
        assert!(w13a03_current_player_gate(3, 3));
        assert!(!w13a03_current_player_gate(3, 4));
        assert!(w13a03_current_player_gate(0, 0));
    }

    #[test]
    fn script_query_matches_0x00660b90() {
        let areas = [sample_area(0xbeef); 8];
        let request = sample_request(1, 0x77, 1);
        let query = w13a03_script_query(&areas, &request).unwrap();
        assert_eq!(query.command, 0x77);
        assert_eq!(query.target, 0x5566);
        assert_eq!(query.marker, 0xbeef);
        assert_eq!(query.region_slot, 0);
        assert_eq!(query.replay_limit, 999_999);

        // The callback's last byte is the low byte of value - 1; the mode byte
        // only feeds the global replay limit.
        let mut mode_request = request;
        mode_request.mode = 5;
        mode_request.value = 3;
        let mode_query = w13a03_script_query(&areas, &mode_request).unwrap();
        assert_eq!(mode_query.region_slot, 2);
        assert_eq!(mode_query.replay_limit, 5);
        assert_eq!(mode_query.marker, 0xbeef);
        assert_eq!(w13a03_script_query(&areas, &sample_request(0, 0, 1)), None);
        assert_eq!(w13a03_script_query(&areas, &sample_request(1, 0, 0)), None);
        assert_eq!(w13a03_script_query(&areas, &sample_request(9, 0, 1)), None);
    }

    #[test]
    fn target_mode_dispatch_matches_0x00660d38() {
        assert_eq!(
            w13a03_target_mode_handler(0),
            Some(W13a03TargetHandler::Mode0Or6)
        );
        assert_eq!(
            w13a03_target_mode_handler(6),
            Some(W13a03TargetHandler::Mode0Or6)
        );
        assert_eq!(
            w13a03_target_mode_handler(4),
            Some(W13a03TargetHandler::Mode4)
        );
        assert_eq!(
            w13a03_target_mode_handler(5),
            Some(W13a03TargetHandler::Mode5)
        );
        for mode in [1, 2, 3, 7, 0xff] {
            assert_eq!(w13a03_target_mode_handler(mode), None);
        }
    }

    #[test]
    fn command_table_base_matches_0x00661410() {
        assert_eq!(w13a03_command_table_base(), 0x00fdf900);
    }

    #[test]
    fn player_counter_matches_0x00661450() {
        let mut counters = W13a03CommandCounters::default();
        let mut matrix = W13a03CommandMatrix::default();

        assert_eq!(
            w13a03_apply_player_counter(2, 0xe6, 5, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Applied(W13a03CommandUpdate::TypeCounter(
                W13a03TypeCounter::At0x2d84
            ))
        );
        assert_eq!(counters.at_0x2d84[2], 5);
        assert_eq!(
            w13a03_apply_player_counter(2, 0xe7, -1, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Applied(W13a03CommandUpdate::TypeCounter(
                W13a03TypeCounter::At0x2ed4
            ))
        );
        assert_eq!(counters.at_0x2ed4[2], -1);
        assert_eq!(
            w13a03_apply_player_counter(2, 0xe8, 7, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Applied(W13a03CommandUpdate::TypeCounter(
                W13a03TypeCounter::At0x2ff4
            ))
        );
        assert_eq!(counters.at_0x2ff4[2], 7);

        // 0xe5 is ignored.
        assert_eq!(
            w13a03_apply_player_counter(2, 0xe5, 9, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Ignored
        );
        // A player outside 0..7 is ignored for the type counters.
        assert_eq!(
            w13a03_apply_player_counter(8, 0xe6, 9, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Ignored
        );

        // 0x0d resolves to the current player and reaches the command matrix.
        assert_eq!(
            w13a03_apply_player_counter(0x0d, 0x10, 3, 4, false, &mut counters, &mut matrix),
            W13a03Outcome::Applied(W13a03CommandUpdate::Matrix)
        );
        assert_eq!(matrix.rows[0x10][4], 3);
        // The matrix rejects commands above 0xe3.
        assert_eq!(
            w13a03_apply_player_counter(1, 0xe4, 3, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Ignored
        );
        // The legacy hook wins before the matrix write.
        assert_eq!(
            w13a03_apply_player_counter(1, 0x10, 3, 0, true, &mut counters, &mut matrix),
            W13a03Outcome::LegacyHook
        );
        // Delegated opcodes.
        assert_eq!(
            w13a03_apply_player_counter(0x0f, 0, 0, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Delegate(W13a03ScriptHandler::Script0f)
        );
        assert_eq!(
            w13a03_apply_player_counter(0x16, 0, 0, 0, false, &mut counters, &mut matrix),
            W13a03Outcome::Ignored
        );
    }

    #[test]
    fn selection_counter_matches_0x00661620() {
        let mut counters = W13a03SelectionCounters::default();
        assert_eq!(
            w13a03_apply_selection_counter(1, 0, 4, 0, &mut counters),
            W13a03Outcome::Applied(W13a03SelectionSet::Selector0)
        );
        assert_eq!(counters.at_0x00[1], 4);
        assert_eq!(counters.at_0x90[1], 4);
        assert_eq!(counters.at_0x30[1], 0);
        assert_eq!(counters.at_0x60[1], 0);

        assert_eq!(
            w13a03_apply_selection_counter(1, 1, 6, 0, &mut counters),
            W13a03Outcome::Applied(W13a03SelectionSet::Selector1)
        );
        assert_eq!(counters.at_0x30[1], 6);
        assert_eq!(counters.at_0x60[1], 6);
        assert_eq!(counters.at_0x00[1], 4);

        assert_eq!(
            w13a03_apply_selection_counter(1, 2, 2, 0, &mut counters),
            W13a03Outcome::Applied(W13a03SelectionSet::Selector2)
        );
        assert_eq!(counters.at_0x00[1], 6);
        assert_eq!(counters.at_0x90[1], 6);
        assert_eq!(counters.at_0x30[1], 8);
        assert_eq!(counters.at_0x60[1], 8);

        assert_eq!(
            w13a03_apply_selection_counter(1, 3, 2, 0, &mut counters),
            W13a03Outcome::Ignored
        );
        assert_eq!(
            w13a03_apply_selection_counter(8, 0, 2, 0, &mut counters),
            W13a03Outcome::Ignored
        );
        assert_eq!(
            w13a03_apply_selection_counter(0x0d, 0, 2, 2, &mut counters),
            W13a03Outcome::Applied(W13a03SelectionSet::Selector0)
        );
        assert_eq!(counters.at_0x00[2], 2);
        assert_eq!(
            w13a03_apply_selection_counter(0x12, 0, 2, 0, &mut counters),
            W13a03Outcome::Delegate(W13a03ScriptHandler::Script12To15)
        );
    }

    #[test]
    fn ratio_counter_matches_0x0066175a() {
        let mut counters = W13a03RatioCounters::default();
        assert_eq!(
            w13a03_apply_ratio_counter(3, 0, 7, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::QuarterAll)
        );
        assert_eq!(counters.at_0x2de4[3], 2);
        assert_eq!(counters.at_0x2e14[3], 2);
        assert_eq!(counters.at_0x2f34[3], 2);
        assert_eq!(counters.at_0x2f64[3], 2);
        assert_eq!(counters.at_0x3204[3], 0);

        assert_eq!(
            w13a03_apply_ratio_counter(4, 1, 5, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::WholeA)
        );
        assert_eq!(counters.at_0x2de4[4], 5);
        assert_eq!(
            w13a03_apply_ratio_counter(4, 2, 5, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::WholeC)
        );
        assert_eq!(counters.at_0x2f34[4], 5);

        assert_eq!(
            w13a03_apply_ratio_counter(5, 3, 5, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::HalfAC)
        );
        assert_eq!(counters.at_0x2de4[5], 3);
        assert_eq!(counters.at_0x2f34[5], 3);
        assert_eq!(counters.at_0x2e14[5], 0);

        assert_eq!(
            w13a03_apply_ratio_counter(6, 4, 9, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::WholeB)
        );
        assert_eq!(counters.at_0x2e14[6], 9);
        assert_eq!(
            w13a03_apply_ratio_counter(6, 5, 9, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::WholeD)
        );
        assert_eq!(counters.at_0x2f64[6], 9);

        assert_eq!(
            w13a03_apply_ratio_counter(7, 6, 9, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::HalfBD)
        );
        assert_eq!(counters.at_0x2e14[7], 5);
        assert_eq!(counters.at_0x2f64[7], 5);
        assert_eq!(
            w13a03_apply_ratio_counter(7, 7, 9, 0, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::WholeE)
        );
        assert_eq!(counters.at_0x3204[7], 9);

        assert_eq!(
            w13a03_apply_ratio_counter(1, 8, 9, 0, &mut counters),
            W13a03Outcome::Ignored
        );
        assert_eq!(
            w13a03_apply_ratio_counter(8, 1, 9, 0, &mut counters),
            W13a03Outcome::Ignored
        );
        assert_eq!(
            w13a03_apply_ratio_counter(0x0d, 1, 9, 2, &mut counters),
            W13a03Outcome::Applied(W13a03RatioSet::WholeA)
        );
        assert_eq!(counters.at_0x2de4[2], 9);
    }

    #[test]
    fn relation_update_matches_0x00661990() {
        assert_eq!(
            W13A03_RELATION_CHUNK * W13A03_RELATION_STRIDE as u32,
            0xe544
        );
        let mut cells = [0u8; W13A03_RELATION_SLOTS * W13A03_RELATION_SLOTS];
        assert_eq!(
            w13a03_apply_relation(1, 1, 0, &mut cells),
            W13a03Outcome::Applied(W13a03RelationWrite {
                relation: 1,
                propagated: true,
            })
        );
        assert_eq!(cells[1], 1);

        // Self-relation differs, so every non-zero cell of the actor's row is
        // overwritten with the new relation.
        cells[0] = 1;
        assert_eq!(
            w13a03_apply_relation(2, 2, 0, &mut cells),
            W13a03Outcome::Applied(W13a03RelationWrite {
                relation: 2,
                propagated: true,
            })
        );
        assert_eq!(cells[0], 2);
        assert_eq!(cells[1], 2);
        assert_eq!(cells[2], 2);
        assert_eq!(cells[3], 0);

        // Selector 0 writes a zero relation and does not propagate.
        assert_eq!(
            w13a03_apply_relation(3, 0, 0, &mut cells),
            W13a03Outcome::Applied(W13a03RelationWrite {
                relation: 0,
                propagated: false,
            })
        );
        assert_eq!(cells[3], 0);

        // Same-slot writes are skipped.
        let before = cells;
        assert_eq!(
            w13a03_apply_relation(0, 1, 0, &mut cells),
            W13a03Outcome::Ignored
        );
        assert_eq!(cells, before);

        // Opcodes delegate; 0x16..=0x19 and values above 0x1a are inert.
        assert_eq!(
            w13a03_apply_relation(0x10, 0, 0, &mut cells),
            W13a03Outcome::Delegate(W13a03ScriptHandler::Script10)
        );
        assert_eq!(
            w13a03_apply_relation(0x16, 0, 0, &mut cells),
            W13a03Outcome::Ignored
        );
        assert_eq!(
            w13a03_apply_relation(0x1b, 0, 0, &mut cells),
            W13a03Outcome::Ignored
        );
        // A short matrix is rejected instead of indexing past the end.
        let mut short = [0u8; 8];
        assert_eq!(
            w13a03_apply_relation(1, 1, 0, &mut short),
            W13a03Outcome::Ignored
        );
    }

    #[test]
    fn text_budget_matches_0x00661b40() {
        assert_eq!(w13a03_text_budget(None), 0);
        assert_eq!(w13a03_text_budget(Some(b"")), 4000);
        assert_eq!(w13a03_text_budget(Some(&[b'a'; 80])), 4000);
        assert_eq!(w13a03_text_budget(Some(&[b'a'; 81])), 4050);
        assert_eq!(w13a03_text_budget(Some(b"abc\0ignored")), 4000);
        assert_eq!(w13a03_text_budget(Some(&[b'a'; 200])), 10_000);
    }

    #[test]
    fn region_query_matches_0x00661d10() {
        let areas = [sample_area(0x1234), sample_area(0x5678)];
        let query = w13a03_region_query(&areas, 1, 0x9abc, 0xdef0).unwrap();
        assert_eq!(query.area, sample_area(0x5678));
        assert_eq!(query.target, 0x9abc);
        assert_eq!(query.command, 0xdef0);
        assert_eq!(query.marker, 0x5678);
        assert_eq!(query.mode, 1);
        assert_eq!(w13a03_region_query(&areas, 2, 0, 0), None);
    }
}
