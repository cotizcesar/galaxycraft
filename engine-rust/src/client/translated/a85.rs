#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent85.rs
// ===========================================================================
// Translation batch 85.  Only this file is edited by the batch.
//
// Batch 85 owns `0x0063c000..=0x0063dfff`.  The incoming ledger lists 23 rows
// in that window: 16 `decompiled_uncited`, three rows already marked
// `translated` that no source cited, one `thunk` and three
// `decompile_failed`.
//
// Twelve of the sixteen uncited rows are translated below.  The other four
// (`FUN_0063d744`, `FUN_0063dd7d`, `FUN_0063deb2`, `FUN_0063dfc9`) are the flattened
// anti-tamper dispatcher: the decompiler followed a poisoned jump table, so
// their "bodies" read uninitialised stack slots, splice x86 opcodes into C
// expressions and end in `halt_baddata()`.  No statement in them is an
// independently recoverable behaviour, and they are listed with that reason
// at the end.  The three rows the decompiler never produced
// (`FUN_0063ce60`, `FUN_0063cf10`, `FUN_0063d19d`) sit in the same contiguous
// protection block.
//
// The window is mostly protection glue rather than simulation: `rdtsc` stamps
// feeding an attention trap, CFG-guarded indirect calls, a 300/6/7/37-frame
// heartbeat, a per-player 12-byte table write driven by a unit-rectangle
// query, and three small unit predicates.  Because the rows are wrappers,
// the reproducible part is the argument, the ordering and the arithmetic;
// every call into the rest of the image is recorded as an address and never
// given behaviour that the batch does not have.
//
// Recovered state (all offsets in bytes):
//
// | owner            | words                                                                    |
// |------------------|--------------------------------------------------------------------------|
// | marker timer     | `DAT_00fc86f0` sweep countdown (300), `DAT_00fc86f4` sub-count (6), `DAT_00fc86f8` refresh request |
// | frame heartbeat  | `DAT_00fc86ec` countdown (60 at init; reloads to 7 or 37)                 |
// | player rows      | 12-byte rows at `(DAT_00f063cc ^ 0x10fae560) + (player + 0x131b) * 0xc`   |
// | query key        | four `u16` sampled from source words `0, 2, 4, 6` into an 8-byte rect     |
// | unit marker      | `CUnit+0xdc` bit 7; the 300/6 sweep clears it through `FUN_005b0d70`'s list |
// | cleaned cell     | `+0x134` dword, zeroed by `0x0063ca41`                                    |
//
// Address to function:
//
// * `0x0063c0c0` [`World::a85_unit_marker_flag`]
// * `0x0063c110` [`a85_attach_plan`] (ledger said `translated`, nothing cites it)
// * `0x0063c190` [`a85_boot_plan`]
// * `0x0063c1b0` [`a85_owner_slot_gate`] (ledger said `translated`, nothing cites it)
// * `0x0063c9f0` [`A85GameTimers::a85_reinit`]
// * `0x0063ca41` [`A85Cell::a85_cleanup`]
// * `0x0063cc12` [`a85_assert_idle_scan`]
// * `0x0063cc50` [`A85MarkerTimer::a85_request_refresh`]
// * `0x0063cc60` [`A85MarkerTimer::a85_request_refresh_if_building`]
// * `0x0063cc90` [`a85_thunk_target`] (jump thunk)
// * `0x0063cca0` [`a85_cell_release_plan`]
// * `0x0063ccb0` [`A85GameTimers::a85_frame_update`]
// * `0x0063cd70` [`a85_chain_gate`]
// * `0x0063cdd0` [`A85AllianceTable::a85_mark_queried_owners`]
// * `0x0063cef4` [`a85_rdtsc_trap`]
//
// Every row here is reached from the anti-tamper glue: the callers check the
// game's integrity and the callees either report through `FUN_00923df0` /
// `FUN_00748d40` or abort through `FUN_00c87420` / `FUN_005b4cd0`.  That is
// why several rows have no simulation-visible effect and are modelled as a
// value plus a call plan instead of a `World` mutation.

// ---------------------------------------------------------------------------
// Shared call-plan types
// ---------------------------------------------------------------------------

/// One recorded call of a wrapper row: the callee and the extra scalar
/// argument the row passes after the implicit object pointer.
///
/// The rows in this window are `__thiscall` wrappers, so the decompiler does
/// not show the object argument.  `None` means the object pointer is the only
/// argument the row supplies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A85Call {
    /// Call target VA.
    pub target: u32,
    /// Extra scalar argument, when the row supplies one.
    pub arg: Option<u32>,
}

/// One CFG-guarded indirect call of `0x0063c190`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A85GuardedCall {
    /// `_guard_check_icall` runs before the call.
    pub cfg_check: bool,
    /// Call target VA.
    pub target: u32,
}

// ---------------------------------------------------------------------------
// 0x0063c0c0 - CUnit+0xdc bit 7 marker
// ---------------------------------------------------------------------------

/// `CUnit+0xdc` bit read by `0x0063c0c0` and cleared by `0x0063ccb0`.
pub const A85_MARKER_BIT: u32 = 7;

/// `1 << 7`.
pub const A85_MARKER_MASK: u32 = 1 << A85_MARKER_BIT;

/// Mask the sweep applies: `0xffffff7f` (`and 0xffffff7f` at `0x0063ccb0`).
pub const A85_MARKER_CLEAR_MASK: u32 = 0xffff_ff7f;

/// Extract `CUnit+0xdc` bit 7 exactly as `0x0063c0c0` does.
pub fn a85_marker_bit(flags_0xdc: u32) -> u32 {
    (flags_0xdc >> A85_MARKER_BIT) & 1
}

/// Apply the sweep's `and 0xffffff7f` to a `CUnit+0xdc` word.
pub fn a85_clear_marker_bit(flags_0xdc: u32) -> u32 {
    flags_0xdc & A85_MARKER_CLEAR_MASK
}

impl World {
    /// `0x0063c0c0`: the `CUnit+0xdc` bit-7 marker of one unit.
    ///
    /// ```c
    /// iVar1 = func_0x005b3b60();
    /// if (iVar1 != 0) {
    ///   iVar1 = FUN_005b5c70();              // dynamic air bit
    ///   if (iVar1 == 0) {
    ///     iVar1 = FUN_005a47e0(param_2);     // visibility probe
    ///     if (iVar1 == 0) {
    ///       return *(uint *)(param_2 + 0xdc) >> 7 & 1;
    ///     }
    ///   }
    /// }
    /// return 0;
    /// ```
    ///
    /// `func_0x005b3b60` is a runtime-resolved predicate that is not in the
    /// function inventory, so its result is the `gate` argument.  The air bit
    /// is the recovered `FUN_005b5c70` ([`World::is_air_unit`]).  The probe at
    /// `FUN_005a47e0` is out of the batch and is only tested for zero, so its
    /// result is an argument too.  `param_1` is unused.
    ///
    /// The sweep at `0x0063ccb0` clears this same bit, so the pair is one
    /// marker: the sweep sets a deadline and `0x0063c0c0` reads whether the
    /// deadline is still armed for a grounded, visible unit.
    pub fn a85_unit_marker_flag(
        &self,
        i: usize,
        gate: bool,
        visibility_probe: u32,
        flags_0xdc: u32,
    ) -> u32 {
        if i >= self.units.len() {
            return 0;
        }
        if !gate || self.is_air_unit(i) || visibility_probe != 0 {
            return 0;
        }
        a85_marker_bit(flags_0xdc)
    }
}

// ---------------------------------------------------------------------------
// 0x0063c110 - attach one unit, or assert the worker flag
// ---------------------------------------------------------------------------

/// `0x0063c110`: filter call applied to the unit before the worker test.
pub const A85_ATTACH_FILTER: u32 = 0x0063_4ac0;

/// `0x0063c110`: link call made only when the worker flag is set.
pub const A85_ATTACH_LINK: u32 = 0x0063_8f10;

/// `0x0063c110`'s failure path, the shared assertion `FUN_005b4cd0`.
pub const A85_ASSERT_NORETURN: u32 = 0x005b_4cd0;

/// What `0x0063c110` does to one unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A85AttachPlan {
    /// `FUN_00634ac0(param_2)` when the runtime gate is open.
    pub filter: Option<u32>,
    /// `FUN_00638f10(param_1, param_2)` when the unit is a worker.
    pub link: Option<u32>,
    /// The row reaches `FUN_005b4cd0` (does not return).
    pub abort: bool,
}

/// `0x0063c110`: attach a unit when the runtime gate is open.
///
/// ```c
/// iVar1 = func_0x005b3b60();
/// if (iVar1 == 0) return;
/// FUN_00634ac0(param_2);
/// iVar1 = FUN_005b64c0();                 // units.dat worker flag
/// if (iVar1 != 0) { FUN_00638f10(param_1, param_2); return; }
/// FUN_005b4cd0();                         // noreturn
/// ```
///
/// The gate is the same unresolved predicate as in `0x0063c0c0`; the worker
/// flag is the recovered `FUN_005b64c0` (`units.dat` bit `0x8` for ids below
/// `0xe4`).  The ledger marks the row `translated` but no current source cites
/// it, so it is restored here.
pub fn a85_attach_plan(gate: bool, worker: bool) -> A85AttachPlan {
    if !gate {
        return A85AttachPlan {
            filter: None,
            link: None,
            abort: false,
        };
    }
    if worker {
        A85AttachPlan {
            filter: Some(A85_ATTACH_FILTER),
            link: Some(A85_ATTACH_LINK),
            abort: false,
        }
    } else {
        A85AttachPlan {
            filter: Some(A85_ATTACH_FILTER),
            link: None,
            abort: true,
        }
    }
}

// ---------------------------------------------------------------------------
// 0x0063c190 - two CFG-guarded calls
// ---------------------------------------------------------------------------

/// `0x0063c190`: the two guarded calls, in order.
///
/// The second call site is the five-byte jump thunk at `FUN_00655ab0`, whose
/// target is `FUN_00651410`; the decompiler prints the target's name on the
/// call.  The thunk address is kept because it is the address the row jumps
/// to.
pub const A85_BOOT_CALLS: [A85GuardedCall; 2] = [
    A85GuardedCall {
        cfg_check: true,
        target: 0x0064_05c0,
    },
    A85GuardedCall {
        cfg_check: true,
        target: 0x0065_5ab0,
    },
];

/// `0x0063c190`: the thunk behind the second call.
pub const A85_BOOT_THUNK_TARGET: u32 = 0x0065_1410;

/// `0x0063c190`: the window's two-call boot step.
///
/// ```c
/// guard_check_icall();
/// FUN_006405c0();
/// guard_check_icall();
/// thunk_FUN_00651410();
/// ```
///
/// Both callees are reached through the compiler's control-flow-guard check;
/// the second through `thunk_FUN_00651410` (`FUN_00655ab0`, a five-byte jump to
/// `FUN_00651410`).  The order is the whole recoverable content of the row.
pub fn a85_boot_plan() -> [A85GuardedCall; 2] {
    A85_BOOT_CALLS
}

// ---------------------------------------------------------------------------
// 0x0063c1b0 - owner-slot gate
// ---------------------------------------------------------------------------

/// `0x0063c1b0`: per-owner slot probe (`DAT_00faa425[owner * 0x4e8] == 0`).
pub const A85_SLOT_PROBE: u32 = 0x0063_5990;

/// `0x0063c1b0`: call made on the abort path before the assertion.
///
/// Ghidra prints it as `func_0x00648d95`, the same unresolved-target marker it
/// uses for `func_0x005b3b60`; the address is neither a function entry in the
/// inventory nor a recovered body.
pub const A85_SLOT_ABORT_CALL: u32 = 0x0064_8d95;

/// `0x0063c1b0`: stride of one owner record in the player table
/// (`FUN_00635990` indexes `(&DAT_00faa425)[owner * 0x4e8]`).
pub const A85_OWNER_RECORD_STRIDE: u32 = 0x4e8;

/// `0x0063c1b0`: address of the probed slot byte for `owner`.
pub fn a85_owner_slot_address(owner: u8) -> u32 {
    (0x00fa_a425u32).wrapping_add(u32::from(owner).wrapping_mul(A85_OWNER_RECORD_STRIDE))
}

/// Result of `0x0063c1b0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A85SlotGate {
    /// The row falls through and returns 0.
    ReturnZero {
        /// `FUN_00635990` ran before the return.
        probe_called: bool,
    },
    /// The row runs `func_0x00648d95` and then `FUN_005b4cd0` (noreturn).
    Abort {
        /// `FUN_00635990` ran before the abort.
        probe_called: bool,
    },
}

/// `0x0063c1b0`: gate a unit by its owner's slot byte.
///
/// ```c
/// if ((param_2 == 0) &&
///     (iVar1 = FUN_00635990(*(undefined1 *)(param_1 + 0x4c)), iVar1 == 0)) {
///   return 0;
/// }
/// func_0x00648d95();
/// FUN_005b4cd0();                         // noreturn
/// ```
///
/// `FUN_00635990` is a pure byte probe: it returns
/// `(&DAT_00faa425)[owner * 0x4e8] == 0`, i.e. the owner's 0x4e8-byte record
/// has a zero at `+1`.  The `&&` short-circuits, so the probe only runs when
/// `param_2` is zero.  The row returns zero only for `param_2 == 0` with the
/// slot byte *non-zero*; every other input aborts.
///
/// The ledger marks the row `translated` but no current source cites it, so it
/// is restored here.
pub fn a85_owner_slot_gate(second_arg_zero: bool, slot_byte_zero: bool) -> A85SlotGate {
    if !second_arg_zero {
        return A85SlotGate::Abort {
            probe_called: false,
        };
    }
    if slot_byte_zero {
        A85SlotGate::Abort { probe_called: true }
    } else {
        A85SlotGate::ReturnZero { probe_called: true }
    }
}

// ---------------------------------------------------------------------------
// 0x0063c9f0 / 0x0063ccb0 - heartbeat timers
// ---------------------------------------------------------------------------

/// `0x0063ccb0`: marker sweep period (`DAT_00fc86f0 = 300`).
pub const A85_MARKER_RELOAD: i32 = 300;

/// `0x0063ccb0`: marker sub-counter reload (`DAT_00fc86f4 = 6`).
pub const A85_MARKER_SUB_RELOAD: i32 = 6;

/// `0x0063c9f0`: initial frame heartbeat countdown (`DAT_00fc86ec = 0x3c`).
pub const A85_INIT_FRAME_COUNTDOWN: i32 = 0x3c;

/// `0x0063c9f0`: initial marker sweep countdown (`DAT_00fc86f0 = 0xf`).
pub const A85_INIT_MARKER_FRAMES: i32 = 0xf;

/// `0x0063c9f0`: initial marker sub-counter (`DAT_00fc86f4 = 6`).
pub const A85_INIT_MARKER_SUB: i32 = A85_MARKER_SUB_RELOAD;

/// `0x0063c9f0`: the refresh request after init (`DAT_00fc86f8 = 0`).
pub const A85_INIT_REFRESH: i32 = 0;

/// `0x0063ccb0`: value that selects the 7-frame heartbeat reload
/// (`0x1194 = 4500`; below it the reload is 37 - 0x1e = 7).
pub const A85_SPEED_BOUNDARY: u32 = 0x1194;

/// `0x0063ccb0`: offset of the selector word inside the image base
/// (`*(u32 *)((DAT_00f063cc ^ 0x10fae560) + 0xe608)`).
pub const A85_SPEED_WORD_OFFSET: u32 = 0xe608;

/// `0x0063ccb0`: address of the selector word.
pub fn a85_speed_word_address(base: u32) -> u32 {
    base.wrapping_add(A85_SPEED_WORD_OFFSET)
}

/// `0x0063ccb0`: heartbeat reload when `*(u32 *)(base + 0xe608) < 0x1194`.
pub const A85_FRAME_RELOAD_SHORT: i32 = 7;

/// `0x0063ccb0`: heartbeat reload otherwise (`0x25`).
pub const A85_FRAME_RELOAD_LONG: i32 = 0x25;

/// `0x0063ccb0`: calls made on every tick, before the timer block.
pub const A85_FRAME_TICK_CALLS: [u32; 3] = [0x0064_08b0, 0x0065_7730, 0x0076_9fc0];

/// `0x0063ccb0`: calls made when the heartbeat expires.
pub const A85_FRAME_EXPIRY_CALLS: [u32; 3] = [0x0063_59d0, 0x0063_9ed0, 0x0065_2b60];

/// Head of the visible-unit list walked by the marker sweep (`FUN_005b0d70`).
pub const A85_SWEEP_LIST_HEAD: u32 = 0x005b_0d70;

/// `0x0063c9f0`: the five calls the init row makes, in order.
pub const A85_INIT_CALLS: [A85Call; 5] = [
    A85Call {
        target: 0x0063_56b0,
        arg: None,
    },
    A85Call {
        target: 0x0065_9590,
        arg: None,
    },
    A85Call {
        target: 0x0065_6bc0,
        arg: Some(0),
    },
    A85Call {
        target: 0x0064_0620,
        arg: None,
    },
    A85Call {
        target: 0x0063_9990,
        arg: None,
    },
];

/// The marker timer words: `DAT_00fc86f0`, `DAT_00fc86f4` and `DAT_00fc86f8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A85MarkerTimer {
    /// `DAT_00fc86f0`: frame countdown to the next sweep check.
    pub frames: i32,
    /// `DAT_00fc86f4`: sweeps-left counter, reloaded to 6.
    pub sub: i32,
    /// `DAT_00fc86f8`: refresh request; any non-zero value forces a sweep.
    pub refresh: i32,
}

impl A85MarkerTimer {
    /// `0x0063cc50`: `DAT_00fc86f8 = 1`.
    pub fn a85_request_refresh(&mut self) {
        self.refresh = 1;
    }

    /// `0x0063cc60`: `if (FUN_005b5820()) DAT_00fc86f8 = 1;`
    ///
    /// `FUN_005b5820` is the recovered `units.dat` building flag for ids below
    /// `0xe4` ([`World::unit_is_building_flagged`]), so the predicate is an
    /// argument and the row only writes the request.
    pub fn a85_request_refresh_if_building(&mut self, is_building: bool) {
        if is_building {
            self.refresh = 1;
        }
    }

    /// `0x0063ccb0`, first block: advance the marker timer and report whether
    /// the sweep runs.
    ///
    /// ```c
    /// if (DAT_00fc86f0 == 0) {
    ///   DAT_00fc86f0 = 300;
    ///   if ((DAT_00fc86f4 == 0) ||
    ///       (DAT_00fc86f4 = DAT_00fc86f4 - 1, DAT_00fc86f8 != 0)) {
    ///     DAT_00fc86f4 = 6;
    ///     DAT_00fc86f8 = 0;
    ///     for (u = FUN_005b0d70(); u; u = *(int *)(u + 4))
    ///       *(uint *)(u + 0xdc) &= 0xffffff7f;
    ///   }
    /// } else {
    ///   DAT_00fc86f0 = DAT_00fc86f0 + -1;
    /// }
    /// ```
    ///
    /// The comma operator matters: when `sub` is non-zero it is decremented
    /// *before* the refresh byte is tested, and a set request is consumed on
    /// the same tick without waiting for the sub-counter to reach zero.  The
    /// sweep itself is returned as a flag because the visible list is not part
    /// of this module's state; [`a85_clear_marker_bit`] is the exact store.
    pub fn a85_tick(&mut self) -> bool {
        if self.frames == 0 {
            self.frames = A85_MARKER_RELOAD;
            if self.sub == 0 || {
                self.sub -= 1;
                self.refresh != 0
            } {
                self.sub = A85_MARKER_SUB_RELOAD;
                self.refresh = 0;
                return true;
            }
            false
        } else {
            self.frames -= 1;
            false
        }
    }
}

/// The frame heartbeat word: `DAT_00fc86ec`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A85FrameTimer {
    /// `DAT_00fc86ec`: countdown tested once per update.
    pub countdown: i32,
}

impl A85FrameTimer {
    /// `0x0063ccb0`, second block: decrement, and reload when the *previous*
    /// value was zero.
    ///
    /// ```c
    /// iVar1 = DAT_00fc86ec;
    /// DAT_00fc86ec = DAT_00fc86ec + -1;
    /// if (iVar1 != 0) return;
    /// DAT_00fc86ec = (-(uint)(*(uint *)(B + 0xe608) < 0x1194) & 0xffffffe2) + 0x25;
    /// FUN_006359d0(); FUN_00639ed0(); FUN_00652b60();
    /// ```
    ///
    /// `Some(reload)` means the three expiry calls run.
    pub fn a85_tick(&mut self, word_at_e608: u32) -> Option<i32> {
        let previous = self.countdown;
        self.countdown = self.countdown.wrapping_sub(1);
        if previous != 0 {
            return None;
        }
        let reload = a85_frame_timer_reload(word_at_e608);
        self.countdown = reload;
        Some(reload)
    }
}

/// `0x0063ccb0`: the heartbeat reload, as the shipped mask arithmetic.
///
/// ```text
/// (-(u32)(word < 0x1194) & 0xffffffe2) + 0x25
/// ```
///
/// The two results are exactly 7 and 37.
pub fn a85_frame_timer_reload(word_at_e608: u32) -> i32 {
    let mask = if word_at_e608 < A85_SPEED_BOUNDARY {
        0xffff_ffe2u32
    } else {
        0
    };
    mask.wrapping_add(A85_FRAME_RELOAD_LONG as u32) as i32
}

/// The two heartbeat globals of `0x0063c9f0` / `0x0063ccb0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A85GameTimers {
    /// `DAT_00fc86f0`/`f4`/`f8`.
    pub marker: A85MarkerTimer,
    /// `DAT_00fc86ec`.
    pub frame: A85FrameTimer,
}

/// What one `0x0063ccb0` update reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A85FrameUpdate {
    /// The marker sweep runs and clears bit 7 of every visible unit.
    pub sweep: bool,
    /// The heartbeat reloaded to this value; the three expiry calls run.
    pub heartbeat_reload: Option<i32>,
}

impl A85GameTimers {
    /// `0x0063c9f0`: (re)initialise the heartbeat.
    ///
    /// ```c
    /// DAT_00fc86f8 = 0;
    /// FUN_006356b0();
    /// FUN_00659590();
    /// FUN_00656bc0(0);
    /// FUN_00640620();
    /// FUN_00639990();
    /// DAT_00fc86ec = 0x3c;
    /// DAT_00fc86f0 = 0xf;
    /// DAT_00fc86f4 = 6;
    /// ```
    ///
    /// The refresh byte is cleared *before* the five calls; the three timer
    /// constants are stored after them.  The calls are returned so the order
    /// stays observable; `FUN_00656bc0` receives the only argument (`0`, the
    /// clear value of the 0x00fd2d38 table fill).
    pub fn a85_reinit(&mut self) -> [A85Call; 5] {
        self.marker.refresh = A85_INIT_REFRESH;
        let calls = A85_INIT_CALLS;
        self.frame.countdown = A85_INIT_FRAME_COUNTDOWN;
        self.marker.frames = A85_INIT_MARKER_FRAMES;
        self.marker.sub = A85_INIT_MARKER_SUB;
        calls
    }

    /// `0x0063ccb0`: one full heartbeat update.
    ///
    /// ```c
    /// FUN_006408b0(); FUN_00657730(); FUN_00769fc0();
    /// ... marker timer block ...
    /// ... frame heartbeat block ...
    /// ```
    ///
    /// The three unconditional calls run before either timer block; the three
    /// expiry calls (`A85_FRAME_EXPIRY_CALLS`) run only when the heartbeat
    /// fires.  `word_at_e608` is the dword at
    /// `(DAT_00f063cc ^ 0x10fae560) + 0xe608` that selects the 7/37 reload.
    pub fn a85_frame_update(&mut self, word_at_e608: u32) -> A85FrameUpdate {
        let sweep = self.marker.a85_tick();
        let heartbeat_reload = self.frame.a85_tick(word_at_e608);
        A85FrameUpdate {
            sweep,
            heartbeat_reload,
        }
    }
}

/// The state `0x0063c9f0` leaves behind, starting from a zeroed image.
pub fn a85_initial_timers() -> A85GameTimers {
    let mut timers = A85GameTimers::default();
    timers.a85_reinit();
    timers
}

// ---------------------------------------------------------------------------
// 0x0063ca41 - cell cleanup
// ---------------------------------------------------------------------------

/// `0x0063ca41`: dword offset zeroed on the cleaned cell.
pub const A85_CELL_CLEARED_OFFSET: u32 = 0x134;

/// `0x0063ca41`: the four calls it makes, with the object pointer implied.
pub const A85_CELL_CALLS: [u32; 4] = [0x0065_6d90, 0x0063_9d00, 0x0065_96e0, 0x0063_5760];

/// An object with the one field `0x0063ca41` clears.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A85Cell {
    /// `+0x134`, zeroed by the cleanup.
    pub field_134: u32,
}

impl A85Cell {
    /// `0x0063ca41`: tear one cell down.
    ///
    /// ```c
    /// FUN_00656d90(param_2, param_3);
    /// FUN_00639d00(param_2, 0);
    /// FUN_006596e0(param_2);
    /// FUN_00635760(param_2);
    /// *(undefined4 *)(param_2 + 0x134) = 0;
    /// ```
    ///
    /// `param_1` is unused.  `param_3` is forwarded to the first call and the
    /// second call receives a literal zero.  The one-byte row `FUN_0063ca40`
    /// prints the same body with the object in `param_1`, but a one-byte row
    /// cannot contain those statements, so that entry is an alias of this
    /// cleanup and is not translated separately.
    pub fn a85_cleanup(&mut self, param_3: u32) -> [A85Call; 4] {
        self.field_134 = 0;
        [
            A85Call {
                target: A85_CELL_CALLS[0],
                arg: Some(param_3),
            },
            A85Call {
                target: A85_CELL_CALLS[1],
                arg: Some(0),
            },
            A85Call {
                target: A85_CELL_CALLS[2],
                arg: None,
            },
            A85Call {
                target: A85_CELL_CALLS[3],
                arg: None,
            },
        ]
    }
}

// ---------------------------------------------------------------------------
// 0x0063cc12 - assert that an owner's unit list is idle
// ---------------------------------------------------------------------------

/// `0x0063cc12`: the tag `AX` must carry on entry.
pub const A85_IDLE_SCAN_TAG: u16 = 0x4a;

/// `0x0063cc12`: prologue call before the list walk (`FUN_00658030`).
pub const A85_IDLE_SCAN_PROLOGUE: u32 = 0x0065_8030;

/// `0x0063cc12`: next record in the owner's list (`CUnit+0x6c`, `FUN_005b4bb0`).
pub const A85_LIST_NEXT: u32 = 0x005b_4bb0;

/// `0x0063cc12`: `FUN_005a3180`, "unit has no sprite or its order id is zero".
pub const A85_IDLE_OR_HIDDEN: u32 = 0x005a_3180;

/// Result of `0x0063cc12`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A85IdleScan {
    /// `AX != 0x4a`: the entry assertion fires.
    TagMismatch,
    /// The owner's list was empty.
    Empty,
    /// Every record was idle/hidden.
    AllIdle,
    /// One record was active: the trailing assertion fires.
    VisibleUnit,
}

/// `0x0063cc12`: walk an owner's unit list, asserting every record is idle.
///
/// ```c
/// if (in_AX != 0x4a) FUN_005b4cd0();
/// FUN_00658030();
/// do {
///   iVar1 = FUN_005b4bb0();      // next record, 0 ends the list
///   if (iVar1 == 0) return;
///   iVar1 = FUN_005a3180();      // 0 = active
/// } while (iVar1 != 0);
/// FUN_005b4cd0();              // noreturn
/// ```
///
/// The decompiler lost the object argument of the `thiscall` helpers, so the
/// per-record result is the `hidden` slice in list order.  The loop's `ECX`
/// reload between `FUN_005b4bb0` and `FUN_005a3180` is also lost, so the exact
/// record each iteration tests (head, next, or tail) is not recoverable; the
/// modelled reading is that every record is visited in list order and any
/// active one selects the trailing assertion.  Empty and all-idle are the two
/// bounded outcomes.
pub fn a85_assert_idle_scan(tag: u16, hidden: &[bool]) -> A85IdleScan {
    if tag != A85_IDLE_SCAN_TAG {
        return A85IdleScan::TagMismatch;
    }
    if hidden.is_empty() {
        return A85IdleScan::Empty;
    }
    if hidden.iter().all(|&record| record) {
        A85IdleScan::AllIdle
    } else {
        A85IdleScan::VisibleUnit
    }
}

// ---------------------------------------------------------------------------
// 0x0063cca0 - table fill and release
// ---------------------------------------------------------------------------

/// `0x0063cca0`: the two calls, in order; `FUN_00656bc0` gets the set value.
pub const A85_CELL_RELEASE_CALLS: [A85Call; 2] = [
    A85Call {
        target: 0x0065_6bc0,
        arg: Some(1),
    },
    A85Call {
        target: 0x0076_b120,
        arg: None,
    },
];

/// `0x0063cca0`: mark the 0x00fd2d38 table and release the renderer.
///
/// ```c
/// FUN_00656bc0(1);
/// FUN_0076b120();
/// ```
///
/// `FUN_00656bc0`'s decompiled body fills `0x00fd2d38..=0x00fd7b57` (20000
/// bytes) through `FUN_00c8a080`; the argument is the fill value the two
/// callers pass (`0` at `0x0063c9f0`, `1` here), so the row is recorded with
/// its argument rather than a behaviour the callee's decompilation does not
/// show.
pub fn a85_cell_release_plan() -> [A85Call; 2] {
    A85_CELL_RELEASE_CALLS
}

// ---------------------------------------------------------------------------
// 0x0063cd70 - six-check integrity chain
// ---------------------------------------------------------------------------

/// `0x0063cd70`: the six checks, in order, all called with `param_1`.
pub const A85_CHAIN_CHECKS: [u32; 6] = [
    0x0065_0240,
    0x0065_8a70,
    0x0063_7080,
    0x0063_43c0,
    0x0064_0320,
    0x0076_7d30,
];

/// `0x0063cd70`: return 1 only when all six checks report success.
///
/// ```c
/// cVar1 = FUN_00650240(param_1); if (cVar1 == '\0') return 0;
/// cVar1 = FUN_00658a70(param_1); if (cVar1 == '\0') return 0;
/// cVar1 = FUN_00637080(param_1); if (cVar1 == '\0') return 0;
/// cVar1 = FUN_006343c0(param_1); if (cVar1 == '\0') return 0;
/// cVar1 = FUN_00640320(param_1); if (cVar1 == '\0') return 0;
/// iVar2 = FUN_00767d30(param_1); return iVar2 != 0;
/// ```
///
/// The first five are byte tests and the last is the 32-bit form; all six
/// receive the same word.  The callees are integrity probes from the same
/// dispatcher and are only referenced by address.
pub fn a85_chain_gate(passed: [bool; 6]) -> u32 {
    if passed.iter().all(|&check| check) {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// 0x0063cdd0 - player-row write over a unit rectangle
// ---------------------------------------------------------------------------

/// `0x0063cdd0`: the row base added to the player index.
pub const A85_PLAYER_ROW_BASE: u32 = 0x131b;

/// `0x0063cdd0`: the 12-byte row stride.
pub const A85_PLAYER_ROW_STRIDE: u32 = 0xc;

/// `0x0063cdd0`: `0x131b * 0xc`, the row area's offset from the image base
/// (`0xe544`).
pub const A85_PLAYER_TABLE_OFFSET: u32 = A85_PLAYER_ROW_BASE * A85_PLAYER_ROW_STRIDE;

/// `0x0063cdd0`: `DAT_00f063cc ^ 0x10fae560`, the recovered image base.
pub const A85_IMAGE_BASE: u32 = 0x00f0_63cc ^ 0x10fa_e560;

/// `0x0063cdd0`: players the table models (owners are bytes).
pub const A85_PLAYER_ROWS: usize = 8;

/// `0x0063cdd0`: bytes in the eight modelled rows.
pub const A85_PLAYER_TABLE_BYTES: usize = A85_PLAYER_ROWS * A85_PLAYER_ROW_STRIDE as usize;

/// `0x0063cdd0`: the source words sampled by the rectangle key.
pub const A85_RECT_KEY_WORDS: [usize; 4] = [0, 2, 4, 6];

/// `0x0063cdd0`: the row offset for `player`, `(player + 0x131b) * 0xc`.
pub fn a85_player_row_offset(player: u32) -> u32 {
    player
        .wrapping_add(A85_PLAYER_ROW_BASE)
        .wrapping_mul(A85_PLAYER_ROW_STRIDE)
}

/// `0x0063cdd0`: address of the byte the row writes for `player`/`owner`.
///
/// ```c
/// *(undefined1 *)(owner + (player + 0x131b) * 0xc + base) = status;
/// ```
///
/// The product is `0xe544 + player * 0xc`, which the sibling probe at
/// `FUN_0063dfc9` also uses as the table base.  All arithmetic is 32-bit
/// wrapping like the shipped address computation.
pub fn a85_player_row_address(base: u32, player: u32, owner: u8) -> u32 {
    let row = a85_player_row_offset(player);
    base.wrapping_add(row).wrapping_add(u32::from(owner))
}

/// `0x0063cdd0`: pack the rectangle key out of the source's low words.
///
/// The row copies `param_2[0]`, `param_2[2]`, `param_2[4]` and `param_2[6]`
/// into four consecutive `undefined2` locals, i.e. it samples the low `u16` of
/// four dwords into an 8-byte rectangle.  The 8-word view below is exactly the
/// `u16 *` view of four 32-bit coordinates.
pub fn a85_query_rect_key(source_words: &[u16; 8]) -> [u16; 4] {
    let mut key = [0u16; 4];
    for (slot, &index) in A85_RECT_KEY_WORDS.iter().enumerate() {
        key[slot] = source_words[index];
    }
    key
}

/// `0x0063cdd0`: the same packing from four 32-bit coordinates, keeping the
/// low `u16` of each (`(short)` truncation, not saturation).
pub fn a85_rect_key_from_coords(coords: [i32; 4]) -> [u16; 4] {
    coords.map(|coord| coord as u16)
}

/// The eight 12-byte player rows at `A85_PLAYER_TABLE_OFFSET`.
///
/// The sibling probe at `FUN_0063dfc9` compares a row byte with `2` while
/// accumulating a per-unit `u16` (`CUnit+0x13a`) into allied totals, which is
/// the only recovered reading of the value; the row written here is the same
/// table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A85AllianceTable {
    /// Raw row bytes; index `player * 0xc + owner`.
    pub bytes: Vec<u8>,
}

impl A85AllianceTable {
    /// An all-zero eight-row table.
    pub fn a85_new() -> Self {
        A85AllianceTable {
            bytes: vec![0; A85_PLAYER_TABLE_BYTES],
        }
    }

    /// `0x0063cdd0`: write `status` for every queried owner except the actor.
    ///
    /// ```c
    /// iVar2 = *cursor;
    /// while (iVar2 != 0) {
    ///   if (*(byte *)(iVar2 + 0x4c) != player)
    ///     *(byte *)(owner + (player + 0x131b) * 0xc + base) = status;
    ///   iVar2 = cursor[1]; cursor++;
    /// }
    /// ```
    ///
    /// `queried_owners` is the `CUnit+0x4c` byte of each record in the
    /// rectangle query's list, in list order; `FUN_005a57f0` (the query) and
    /// `FUN_005a5770` (the cursor release) are out of the batch.  The method
    /// returns every byte address written, in write order, including a repeat
    /// write when two records share an owner.  The vector holds the eight
    /// modelled rows (`0xe544..=0xe5a3`); a player index beyond them extends
    /// the vector instead of wrapping into unrelated memory, which retail never
    /// does because the caller passes 0..=7.
    pub fn a85_mark_queried_owners(
        &mut self,
        player: u32,
        status: u8,
        queried_owners: &[u8],
    ) -> Vec<u32> {
        let mut writes = Vec::new();
        for &owner in queried_owners {
            if u32::from(owner) == player {
                continue;
            }
            let address = a85_player_row_address(A85_IMAGE_BASE, player, owner);
            let index = player
                .wrapping_mul(A85_PLAYER_ROW_STRIDE)
                .wrapping_add(u32::from(owner)) as usize;
            if self.bytes.len() <= index {
                self.bytes.resize(index + 1, 0);
            }
            self.bytes[index] = status;
            writes.push(address);
        }
        writes
    }
}

// ---------------------------------------------------------------------------
// 0x0063cef4 - rdtsc trap payload
// ---------------------------------------------------------------------------

/// `0x0063cef4`: the non-returning report target.
pub const A85_TRAP_REPORT: u32 = 0x00c8_7420;

/// The four arguments `0x0063cef4` hands to `FUN_00c87420`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A85TrapPayload {
    /// Low dword of `rdtsc`.
    pub tsc_low: u32,
    /// High dword of `rdtsc`.
    pub tsc_high: u32,
    /// The value reloaded from `EBP-8` (`in_EAX`).
    pub value: i32,
    /// `value >> 31`, the sign extension the row computes.
    pub sign: i32,
}

/// `0x0063cef4`: capture a timestamp and the trapped value.
///
/// ```c
/// *(undefined4 *)(unaff_EBP + -8) = in_EAX;
/// *(undefined4 *)(unaff_EBP + -8) = *(undefined4 *)(unaff_EBP + -8);
/// uVar1 = rdtsc();
/// FUN_00c87420((int)uVar1, (int)((ulonglong)uVar1 >> 0x20),
///              *(int *)(unaff_EBP + -8), *(int *)(unaff_EBP + -8) >> 0x1f);
/// ```
///
/// The self-assignment is the compiler's store of the value it later reloads;
/// it changes nothing.  `FUN_00c87420` does not return, so this is a payload
/// value rather than a control-flow model.
pub fn a85_rdtsc_trap(value: i32, tsc: u64) -> A85TrapPayload {
    A85TrapPayload {
        tsc_low: tsc as u32,
        tsc_high: (tsc >> 32) as u32,
        value,
        sign: value >> 31,
    }
}

// ---------------------------------------------------------------------------
// 0x0063cc90 - jump thunk
// ---------------------------------------------------------------------------

/// `0x0063cc90`: the tail-jump target (`thunk_FUN_0064f940`).
pub const A85_THUNK_TARGET: u32 = 0x0064_f940;

/// `0x0063cc90`: the five-byte `jmp` to `FUN_0064f940`.
pub fn a85_thunk_target() -> u32 {
    A85_THUNK_TARGET
}

// ---------------------------------------------------------------------------
// Rows not translated
// ---------------------------------------------------------------------------

// * `FUN_0063ca40` (size 1, ledger `translated`): the decompiler printed the
//   `0x0063ca41` cleanup body under this address, with the object in
//   `param_1` instead of `param_2`.  A one-byte row cannot contain that body,
//   so the entry is an alias of the adjacent cleanup rather than an
//   independent routine; only `0x0063ca41` is translated.
//
// * `FUN_0063d744` (1554 bytes), `FUN_0063dd7d` (147), `FUN_0063deb2` (274),
//   `FUN_0063dfc9` (635): the flattened anti-tamper dispatcher.  The decompiler
//   flagged "Control flow encountered bad instruction data", "Instruction
//   overlaps instruction" and "Unable to track spacebase fully"; the bodies
//   read `unaff_EBP`/`unaff_ESI` without assignment, contain `halt_baddata()`,
//   `swi(3)` and `out()` on uninitialised values, and splice x86 opcodes
//   (`0xb9, 0xffffba, ...`) into C expressions.  The only high-level-looking
//   fragments are reached through the bad jump table and cannot be tied to the
//   row's entry path; no statement is translated.
//
// * `FUN_0063ce60` (143), `FUN_0063cf10` (1053), `FUN_0063d19d` (642): the
//   decompiler never produced a body (timeout, "process: timeout"), and their
//   byte dumps begin with the same poisoned padding (`70 6c / 8a ed / 71 04`
//   inside a valid prologue).  There is no recovered C to translate.
//
// Counts for the window: 23 ledger rows = 16 uncited + 3 stale `translated`
// + 1 thunk + 3 failed.  Translated here: 15 addresses (12 uncited, 2 stale
// `translated`, 1 thunk); 8 rows are left alone for the reasons above, 3 of
// them because no decompilation exists.

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent85 {
    use super::*;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn world() -> World {
        World::new(
            Arc::new(crate::data::GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    // -- 0x0063c0c0 ---------------------------------------------------------

    /// `0x0063c0c0`: the return is bit 7 only, and the two guards and the
    /// probe each force zero.
    #[test]
    fn marker_flag_is_bit_seven_behind_three_guards() {
        let mut w = world();
        let marine = w.spawn(0, uid::MARINE, 10, 10).unwrap();
        let scout = w.spawn(0, uid::SCOUT, 20, 10).unwrap();

        assert_eq!(a85_marker_bit(0x0000_0000), 0);
        assert_eq!(a85_marker_bit(0x0000_0080), 1);
        assert_eq!(a85_marker_bit(0xffff_ff7f), 0);
        assert_eq!(a85_marker_bit(0xffff_ffff), 1);

        assert_eq!(w.a85_unit_marker_flag(marine, true, 0, 0x80), 1);
        assert_eq!(w.a85_unit_marker_flag(marine, false, 0, 0x80), 0);
        assert_eq!(w.a85_unit_marker_flag(marine, true, 1, 0x80), 0);
        assert_eq!(w.a85_unit_marker_flag(marine, true, 0, 0x40), 0);
        assert_eq!(w.a85_unit_marker_flag(marine, true, 0, 0x180), 1);
        // `FUN_005b5c70`: a flyer never reports the marker.
        assert_eq!(w.a85_unit_marker_flag(scout, true, 0, 0x80), 0);
        // Out of range is not a unit: the shipped pointer would be invalid.
        assert_eq!(w.a85_unit_marker_flag(999, true, 0, 0x80), 0);
    }

    /// `0x0063ccb0`: the sweep clears exactly `0x80`, and nothing else.
    #[test]
    fn sweep_mask_clears_only_bit_seven() {
        assert_eq!(a85_clear_marker_bit(0xffff_ffff), 0xffff_ff7f);
        assert_eq!(a85_clear_marker_bit(0x0000_0080), 0);
        assert_eq!(a85_clear_marker_bit(0x0000_0100), 0x0000_0100);
        assert_eq!(A85_MARKER_CLEAR_MASK, 0xffff_ff7f);
        assert_eq!(A85_MARKER_MASK, 0x80);
    }

    // -- 0x0063c110 ---------------------------------------------------------

    /// `0x0063c110`: the runtime gate short-circuits before any call, the
    /// worker flag selects the link call, and a non-worker asserts.
    #[test]
    fn attach_plan_follows_the_gate_and_worker_flag() {
        let closed = a85_attach_plan(false, true);
        assert_eq!(closed.filter, None);
        assert_eq!(closed.link, None);
        assert!(!closed.abort);

        let attached = a85_attach_plan(true, true);
        assert_eq!(attached.filter, Some(0x0063_4ac0));
        assert_eq!(attached.link, Some(0x0063_8f10));
        assert!(!attached.abort);

        let rejected = a85_attach_plan(true, false);
        assert_eq!(rejected.filter, Some(0x0063_4ac0));
        assert_eq!(rejected.link, None);
        assert!(rejected.abort);
    }

    // -- 0x0063c190 ---------------------------------------------------------

    /// `0x0063c190`: two CFG-checked calls in the recovered order.
    #[test]
    fn boot_plan_guards_both_calls() {
        let plan = a85_boot_plan();
        assert_eq!(plan.len(), 2);
        assert!(plan[0].cfg_check && plan[1].cfg_check);
        assert_eq!(plan[0].target, 0x0064_05c0);
        assert_eq!(plan[1].target, 0x0065_5ab0);
        assert_eq!(A85_BOOT_THUNK_TARGET, 0x0065_1410);
    }

    // -- 0x0063c1b0 ---------------------------------------------------------

    /// `0x0063c1b0`: only `param_2 == 0` with a non-zero slot byte returns.
    #[test]
    fn owner_slot_gate_returns_only_for_the_probed_case() {
        assert_eq!(
            a85_owner_slot_gate(true, false),
            A85SlotGate::ReturnZero { probe_called: true }
        );
        assert_eq!(
            a85_owner_slot_gate(true, true),
            A85SlotGate::Abort { probe_called: true }
        );
        // The `&&` short-circuit skips the probe when param_2 is non-zero.
        assert_eq!(
            a85_owner_slot_gate(false, false),
            A85SlotGate::Abort {
                probe_called: false
            }
        );
        assert_eq!(A85_SLOT_PROBE, 0x0063_5990);
        assert_eq!(A85_SLOT_ABORT_CALL, 0x0064_8d95);
        assert_eq!(A85_OWNER_RECORD_STRIDE, 0x4e8);
        assert_eq!(a85_owner_slot_address(0), 0x00fa_a425);
        assert_eq!(a85_owner_slot_address(1), 0x00fa_a425 + 0x4e8);
        assert_eq!(a85_owner_slot_address(7), 0x00fa_a425 + 7 * 0x4e8);
    }

    // -- 0x0063c9f0 ---------------------------------------------------------

    /// `0x0063c9f0`: refresh cleared before the calls; 60/15/6 after them.
    #[test]
    fn reinit_stores_sixty_fifteen_and_six() {
        let mut timers = A85GameTimers {
            marker: A85MarkerTimer {
                frames: 111,
                sub: 222,
                refresh: 333,
            },
            frame: A85FrameTimer { countdown: 444 },
        };
        let calls = timers.a85_reinit();
        assert_eq!(timers.marker.refresh, 0);
        assert_eq!(timers.frame.countdown, 0x3c);
        assert_eq!(timers.marker.frames, 0xf);
        assert_eq!(timers.marker.sub, 6);
        assert_eq!(
            calls.map(|call| call.target),
            [
                0x0063_56b0,
                0x0065_9590,
                0x0065_6bc0,
                0x0064_0620,
                0x0063_9990
            ]
        );
        assert_eq!(calls[2].arg, Some(0));
        assert_eq!(calls[0].arg, None);
        assert_eq!(calls[4].arg, None);

        let fresh = a85_initial_timers();
        assert_eq!(fresh, timers);
    }

    // -- 0x0063ca41 ---------------------------------------------------------

    /// `0x0063ca41`: the +0x134 dword is zeroed and the four calls carry the
    /// recovered arguments.
    #[test]
    fn cell_cleanup_zeroes_0x134_and_keeps_call_order() {
        let mut cell = A85Cell {
            field_134: 0xdead_beef,
        };
        let calls = cell.a85_cleanup(0x1234_5678);
        assert_eq!(cell.field_134, 0);
        assert_eq!(A85_CELL_CLEARED_OFFSET, 0x134);
        assert_eq!(calls[0].target, 0x0065_6d90);
        assert_eq!(calls[0].arg, Some(0x1234_5678));
        assert_eq!(calls[1].target, 0x0063_9d00);
        assert_eq!(calls[1].arg, Some(0));
        assert_eq!(calls[2].target, 0x0065_96e0);
        assert_eq!(calls[2].arg, None);
        assert_eq!(calls[3].target, 0x0063_5760);
        assert_eq!(calls[3].arg, None);
    }

    // -- 0x0063cc12 ---------------------------------------------------------

    /// `0x0063cc12`: the entry tag and the first active record select the two
    /// assertion paths.
    #[test]
    fn idle_scan_asserts_on_tag_and_on_the_first_active_unit() {
        assert_eq!(a85_assert_idle_scan(0x49, &[]), A85IdleScan::TagMismatch);
        assert_eq!(a85_assert_idle_scan(0x4a, &[]), A85IdleScan::Empty);
        assert_eq!(
            a85_assert_idle_scan(0x4a, &[true, true, true]),
            A85IdleScan::AllIdle
        );
        assert_eq!(
            a85_assert_idle_scan(0x4a, &[true, false, true]),
            A85IdleScan::VisibleUnit
        );
        assert_eq!(A85_IDLE_SCAN_TAG, 0x4a);
        assert_eq!(A85_IDLE_SCAN_PROLOGUE, 0x0065_8030);
        assert_eq!(A85_LIST_NEXT, 0x005b_4bb0);
        assert_eq!(A85_IDLE_OR_HIDDEN, 0x005a_3180);
    }

    // -- 0x0063cc50 / 0x0063cc60 -------------------------------------------

    /// `0x0063cc50`: unconditional request; `0x0063cc60`: request only for a
    /// building.  Neither touches the counters.
    #[test]
    fn refresh_requests_set_only_the_request_byte() {
        let mut timer = A85MarkerTimer {
            frames: 7,
            sub: 3,
            refresh: 0,
        };
        timer.a85_request_refresh();
        assert_eq!(timer.refresh, 1);
        assert_eq!(timer.frames, 7);
        assert_eq!(timer.sub, 3);

        let mut building = A85MarkerTimer::default();
        building.a85_request_refresh_if_building(true);
        assert_eq!(building.refresh, 1);

        let mut other = A85MarkerTimer {
            refresh: 9,
            ..Default::default()
        };
        other.a85_request_refresh_if_building(false);
        assert_eq!(other.refresh, 9);
    }

    // -- 0x0063cca0 ---------------------------------------------------------

    /// `0x0063cca0`: the fill gets 1 and the release follows.
    #[test]
    fn cell_release_fills_with_one_then_releases() {
        let plan = a85_cell_release_plan();
        assert_eq!(plan[0].target, 0x0065_6bc0);
        assert_eq!(plan[0].arg, Some(1));
        assert_eq!(plan[1].target, 0x0076_b120);
        assert_eq!(plan[1].arg, None);
    }

    // -- 0x0063ccb0 ---------------------------------------------------------

    /// `0x0063ccb0`: the sweep countdown reloads to 300 and only checks the
    /// sub-counter when it reaches zero.
    #[test]
    fn marker_timer_decrements_then_reloads_to_300() {
        let mut timer = A85MarkerTimer {
            frames: 2,
            sub: 6,
            refresh: 0,
        };
        assert!(!timer.a85_tick());
        assert_eq!(timer.frames, 1);
        assert_eq!(timer.sub, 6);
        assert!(!timer.a85_tick());
        assert_eq!(timer.frames, 0);
        assert_eq!(timer.sub, 6);
        // Reaching zero reloads and consumes one sub-count.
        assert!(!timer.a85_tick());
        assert_eq!(timer.frames, 300);
        assert_eq!(timer.sub, 5);
        assert_eq!(A85_MARKER_RELOAD, 300);
    }

    /// `0x0063ccb0`: a zero sub-counter sweeps immediately and reloads it.
    #[test]
    fn marker_timer_sweeps_when_the_sub_counter_is_zero() {
        let mut timer = A85MarkerTimer {
            frames: 0,
            sub: 0,
            refresh: 0,
        };
        assert!(timer.a85_tick());
        assert_eq!(timer.frames, 300);
        assert_eq!(timer.sub, 6);
        assert_eq!(timer.refresh, 0);
    }

    /// `0x0063ccb0`: the refresh request is tested *after* the sub-counter is
    /// decremented, so the decrement is observable on the same tick.
    #[test]
    fn marker_timer_consumes_refresh_after_decrementing() {
        let mut timer = A85MarkerTimer {
            frames: 0,
            sub: 5,
            refresh: 1,
        };
        assert!(timer.a85_tick());
        assert_eq!(timer.sub, 6);
        assert_eq!(timer.refresh, 0);
        // A set request never waits for the sub-counter to reach zero.
        let mut long = A85MarkerTimer {
            frames: 0,
            sub: 5,
            refresh: 1,
        };
        assert!(long.a85_tick());
        assert_eq!(long.frames, 300);
    }

    /// `0x0063ccb0`: the heartbeat tests the previous value, and the reload is
    /// 7 below 0x1194 and 37 at or above it.
    #[test]
    fn frame_timer_uses_the_previous_value_and_the_mask_reload() {
        assert_eq!(a85_frame_timer_reload(0), 7);
        assert_eq!(a85_frame_timer_reload(0x1193), 7);
        assert_eq!(a85_frame_timer_reload(0x1194), 37);
        assert_eq!(a85_frame_timer_reload(u32::MAX), 37);
        assert_eq!(A85_FRAME_RELOAD_SHORT, 7);
        assert_eq!(A85_FRAME_RELOAD_LONG, 0x25);
        assert_eq!(A85_SPEED_BOUNDARY, 0x1194);
        assert_eq!(
            a85_speed_word_address(A85_IMAGE_BASE),
            A85_IMAGE_BASE + 0xe608
        );

        let mut timer = A85FrameTimer { countdown: 0x3c };
        assert_eq!(timer.a85_tick(0), None);
        assert_eq!(timer.countdown, 0x3b);

        let mut expired = A85FrameTimer { countdown: 0 };
        assert_eq!(expired.a85_tick(0x1193), Some(7));
        assert_eq!(expired.countdown, 7);

        let mut expired_high = A85FrameTimer { countdown: 0 };
        assert_eq!(expired_high.a85_tick(0x1194), Some(37));
        assert_eq!(expired_high.countdown, 37);
    }

    /// `0x0063ccb0`: one update advances both globals independently; the
    /// expiry list is the recovered three calls.
    #[test]
    fn frame_update_runs_both_timers() {
        let mut timers = a85_initial_timers();
        let update = timers.a85_frame_update(0);
        assert!(!update.sweep);
        assert_eq!(update.heartbeat_reload, None);
        assert_eq!(timers.frame.countdown, 0x3b);

        let mut expired = A85GameTimers {
            marker: A85MarkerTimer {
                frames: 0,
                sub: 0,
                refresh: 0,
            },
            frame: A85FrameTimer { countdown: 0 },
        };
        let update = expired.a85_frame_update(0x2000);
        assert!(update.sweep);
        assert_eq!(update.heartbeat_reload, Some(37));
        assert_eq!(
            A85_FRAME_EXPIRY_CALLS,
            [0x0063_59d0, 0x0063_9ed0, 0x0065_2b60]
        );
        assert_eq!(
            A85_FRAME_TICK_CALLS,
            [0x0064_08b0, 0x0065_7730, 0x0076_9fc0]
        );
        assert_eq!(A85_SWEEP_LIST_HEAD, 0x005b_0d70);
    }

    // -- 0x0063cd70 ---------------------------------------------------------

    /// `0x0063cd70`: all six checks must pass; one failure returns zero.
    #[test]
    fn chain_gate_needs_all_six_checks() {
        assert_eq!(a85_chain_gate([true; 6]), 1);
        for failed in 0..6 {
            let mut passed = [true; 6];
            passed[failed] = false;
            assert_eq!(a85_chain_gate(passed), 0, "check {failed}");
        }
        assert_eq!(
            A85_CHAIN_CHECKS,
            [
                0x0065_0240,
                0x0065_8a70,
                0x0063_7080,
                0x0063_43c0,
                0x0064_0320,
                0x0076_7d30
            ]
        );
    }

    // -- 0x0063cdd0 ---------------------------------------------------------

    /// `0x0063cdd0`: the row address is `0xe544 + player * 0xc + owner`, and
    /// the sibling's `(player + 0x131b) * 0xc` form is the same value.
    #[test]
    fn player_row_addresses_match_the_0xe544_table() {
        assert_eq!(A85_PLAYER_TABLE_OFFSET, 0xe544);
        assert_eq!(A85_IMAGE_BASE, 0x100a_86ac);
        assert_eq!(a85_player_row_offset(0), 0xe544);
        assert_eq!(a85_player_row_offset(1), 0xe544 + 0xc);
        assert_eq!(a85_player_row_offset(7), 0xe544 + 7 * 0xc);
        assert_eq!(
            a85_player_row_address(A85_IMAGE_BASE, 0, 0),
            A85_IMAGE_BASE + 0xe544
        );
        assert_eq!(
            a85_player_row_address(A85_IMAGE_BASE, 1, 0),
            A85_IMAGE_BASE + 0xe544 + 0xc
        );
        assert_eq!(
            a85_player_row_address(A85_IMAGE_BASE, 7, 7),
            A85_IMAGE_BASE + 0xe544 + 7 * 0xc + 7
        );
        assert_eq!(
            a85_player_row_address(A85_IMAGE_BASE, 3, 5),
            A85_IMAGE_BASE + (3 + 0x131b) * 0xc + 5
        );
        assert_eq!(A85_PLAYER_TABLE_BYTES, 8 * 0xc);
    }

    /// `0x0063cdd0`: the key samples words 0/2/4/6, so each dword coordinate
    /// contributes only its low 16 bits.
    #[test]
    fn query_key_samples_every_other_word() {
        let words = [
            0x0001, 0x0000, 0xfffe, 0xffff, 0x2345, 0x0001, 0x8000, 0x00ff,
        ];
        assert_eq!(a85_query_rect_key(&words), [0x0001, 0xfffe, 0x2345, 0x8000]);
        assert_eq!(A85_RECT_KEY_WORDS, [0, 2, 4, 6]);

        assert_eq!(
            a85_rect_key_from_coords([1, -1, 0x1_2345, -0x8000]),
            [0x0001, 0xffff, 0x2345, 0x8000]
        );
        // The dword view and the u16 view agree.
        let coords = [0x0001_0001, -2, 0x0002_0002, 0x7fff_ffff];
        let dwords = [
            coords[0] as u32 as u16,
            (coords[0] as u32 >> 16) as u16,
            coords[1] as u32 as u16,
            (coords[1] as u32 >> 16) as u16,
            coords[2] as u32 as u16,
            (coords[2] as u32 >> 16) as u16,
            coords[3] as u32 as u16,
            (coords[3] as u32 >> 16) as u16,
        ];
        assert_eq!(
            a85_query_rect_key(&dwords),
            a85_rect_key_from_coords(coords)
        );
    }

    /// `0x0063cdd0`: the acting player's own records are skipped, every other
    /// owner is written, duplicates rewrite the same byte and the write
    /// addresses come back in list order.
    #[test]
    fn alliance_mark_skips_self_and_writes_each_other_owner() {
        let mut table = A85AllianceTable::a85_new();
        let writes = table.a85_mark_queried_owners(0, 2, &[0, 1, 1, 2]);
        assert_eq!(
            writes,
            [
                a85_player_row_address(A85_IMAGE_BASE, 0, 1),
                a85_player_row_address(A85_IMAGE_BASE, 0, 1),
                a85_player_row_address(A85_IMAGE_BASE, 0, 2),
            ]
        );
        let index = |owner: usize| owner; // row 0 starts at the table offset
        assert_eq!(table.bytes[index(0)], 0);
        assert_eq!(table.bytes[index(1)], 2);
        assert_eq!(table.bytes[index(2)], 2);
        assert_eq!(table.bytes[index(3)], 0);

        let mut later = A85AllianceTable::a85_new();
        let writes = later.a85_mark_queried_owners(3, 2, &[3, 7]);
        assert_eq!(writes, [a85_player_row_address(A85_IMAGE_BASE, 3, 7)]);
        assert_eq!(later.bytes[3 * 0xc + 3], 0);
        assert_eq!(later.bytes[3 * 0xc + 7], 2);
        assert_eq!(later.bytes[4 * 0xc], 0);
    }

    // -- 0x0063cef4 ---------------------------------------------------------

    /// `0x0063cef4`: the timestamp splits into two dwords and the value keeps
    /// its exact sign-extension argument.
    #[test]
    fn rdtsc_trap_splits_the_stamp_and_signs_the_value() {
        let payload = a85_rdtsc_trap(-5, 0x1122_3344_5566_7788);
        assert_eq!(payload.tsc_low, 0x5566_7788);
        assert_eq!(payload.tsc_high, 0x1122_3344);
        assert_eq!(payload.value, -5);
        assert_eq!(payload.sign, -1);

        assert_eq!(a85_rdtsc_trap(0, 0).sign, 0);
        assert_eq!(a85_rdtsc_trap(7, 0).sign, 0);
        assert_eq!(a85_rdtsc_trap(i32::MIN, 0).sign, -1);
        assert_eq!(A85_TRAP_REPORT, 0x00c8_7420);
    }

    // -- 0x0063cc90 ---------------------------------------------------------

    /// `0x0063cc90`: the thunk's only content is its target.
    #[test]
    fn thunk_targets_fun_0064f940() {
        assert_eq!(a85_thunk_target(), 0x0064_f940);
        assert_eq!(A85_THUNK_TARGET, 0x0064_f940);
    }
}
