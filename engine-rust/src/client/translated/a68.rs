#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent68.rs
// ===========================================================================
// Translation batch 68.  Only this file is edited by the batch.
//
// Batch 68 owns `0x0061a000..=0x0061bfff`.  The ledger lists 67 rows in that
// window (66 `decompiled_uncited` plus the `FUN_0061a440` thunk); 30 rows are
// translated below and the other 37 are skipped for the reasons recorded in
// [Skipped rows](#skipped-rows).
//
// The window is the Remastered settings layer: the option-bit dispatcher
// (`0x0061a140`..=`0x0061a1e3`), the 120-frame timeline emitter
// (`0x0061a290`), the per-variant apply helper (`0x0061a880`..=
// `0x0061a9b5`), the `std::string` assignment sites that name the registry
// keys (`0x0061aa90`..=`0x0061b470`), the shutdown call list (`0x0061b170`),
// and the small storage helpers at the end of the window.  Nothing here
// reads or writes simulation state, so the translations are free functions
// over local value types.  Calls whose targets live outside the window enter
// as caller-supplied booleans or values; no helper outside this file is
// assumed to exist.
//
// Recovered layouts:
//
// | offset | meaning |
// |--------|---------|
// | `+0x00`/`+0x04` | option-event prefix words, always zero (`0x0061a140`) |
// | `+0x08` | option-event zero qword |
// | `+0x10` | option-event label word, the `FUN_009736b0` dispatch index |
// | `+0x12` | option-event packed query, `(second << 16) | first` |
// | `+0x18` | option-event payload word (`param_4` of `0x0061a140`) |
// | `+0x10`/`+0x14` | value-range low/high dwords (`0x0061a3f4`) |
// | `+0x18`..`+0x20` | value-range `-1`, `0x7fffffff`, zero byte |
// | `+0x78` | variant storage word, bit 0 tags the indirect wrapper |
// | `+0x60` | variant apply result byte |
// | `+0x130`/`+0x134` | caller-record dword pair (`0x0061a370`) |
// | `+0x04` | chain child link; `+0x18`/`+0x34` heap-tagged words (`0x0061b6b0`) |
// | `+0x00`/`+0x04`/`+0x08` | `std::list` self/self/size header (`0x0061bd50`) |
//
// The five capacity-only rows (`0x0061aa90`, `0x0061ab70`, `0x0061acb0`,
// `0x0061adf0`, `0x0061b090`) share the shipped growth policy at
// `FUN_00538810`:
//
// ```text
// if ((cap & 0x7fffffff) < request) {
//     grown = (cap >> 1 & 0x3fffffff) + (cap & 0x7fffffff);
//     if (grown < request) grown = request;
//     cap = ((cap ^ grown) & 0x7fffffff) ^ cap;
// }
// ```
//
// [`a68_grown_capacity`] reproduces it bit for bit; those five rows differ
// only in the immediate request (25, 14, 12, 16, 8) and are translated as
// [`a68_reserve_site`] over [`A68_RESERVE_SITES`].  Batch 60 independently
// recovered the same policy for `FUN_00538810` as `a60_grown_capacity`, which
// is a useful cross-check on that arithmetic.
//
// ## Skipped rows
//
// * `FUN_0061a04a` - the prologue is misaligned (`this + 0x458bf845` is not
//   a field) and the only self-contained branch,
//   `(byte)x == x ^ 0x80d`, is unsatisfiable (the xor always sets a bit above
//   bit 7), so the two constant chains are dead.  Everything else is
//   `unaff_EBP` caller-frame slots and one out-of-window report call.
// * `FUN_0061a349` - the recovered shape is `do { ... } while (true)` around
//   an inner loop that only descends while the total is positive.  For a
//   start below `-0x168` the outer loop re-adds `0x78` and re-emits the same
//   negative value forever, so a faithful translation would not terminate.
//   The terminating part is the seek form of `0x0061a290`'s stepper.
// * `FUN_0061a390`, `FUN_0061a450`, `FUN_0061a480`, `FUN_0061a4b0`,
//   `FUN_0061a4c0` - one-call trampolines (`FUN_00944f60`, `FUN_007e2680`,
//   `FUN_007e1fd0`); the call target and the caller's registers are all the
//   body contains.
// * `FUN_0061a3b0` - an SEH frame followed by the non-returning
//   `FUN_007e25b0(0, 0x32)`.
// * `FUN_0061a82d`, `FUN_0061a85a`, `FUN_0061ab39`, `FUN_0061ad41`,
//   `FUN_0061ad59`, `FUN_0061b8c2`, `FUN_0061b8ed`, `FUN_0061b9bb`,
//   `FUN_0061b9e8`, `FUN_0061ba10`, `FUN_0061ba65`, `FUN_0061ba87`,
//   `FUN_0061baa9` - MSVC exception landing pads.  Each starts with a
//   virtual `+0x10` call and then either stores a frame state constant or
//   rethrows through `FUN_00c86b6e`; the operands are `unaff_EBP` slots owned
//   by the caller, and no routine-level arithmetic is recoverable.
// * `FUN_0061a440` - the ledger marks this row `thunk`.
// * `FUN_0061a54f` - overlapping instruction decode (`in(in_DX)` is not an
//   instruction) plus a stack-cookie store into an unaff frame.
// * `FUN_0061a749` - `bad instruction data`, overlapping instruction ranges,
//   `rdtsc`, and `halt_baddata`; there is no coherent body to translate.
// * `FUN_0061b165` - one byte, `halt_baddata`.
// * `FUN_0061b23c`, `FUN_0061b275`, `FUN_0061b39c`, `FUN_0061b3d5`,
//   `FUN_0061b4fc`, `FUN_0061b535` - the catch funclets of the three
//   `useKrVariant` / `originalCampaign` / `originalUnitVO` registry writes.
//   They forward to `0x0061a880` and the out-of-window registry helpers and
//   read their arguments from `unaff_EBP`/`unaff_EDI`; the corresponding
//   key-assignment arithmetic is translated from `0x0061af30`..=`0x0061b470`.
// * `FUN_0061b5e0` - the owner destructor.  Its body is a TLS one-time-init
//   guard plus calls at `FUN_00679db0`, `FUN_00984a80` and the `+0x10` flag
//   pointer; the only local transitions (`+0x14 = 0`, byte at `+0x10` = 0)
//   are reached only when the caller's object is non-null, and the
//   decompiler recovers that object only as `unaff_ECX`.
// * `FUN_0061bb00`, `FUN_0061bb10` - config validation.  The bodies compare
//   a constructed string against data globals, then throw; the only local
//   arithmetic is `+1` on a helper result, and the compared literals are
//   addressed only through `&DAT_00d32a20` / `&DAT_00d396cc` /
//   `&DAT_00d3a2bc`, whose bytes are not in the decompilation.
// * `FUN_0061bd40` - returns the address of
//   `<lambda_69d86eae3b477236d04d62d2a6c1a7ae>::RTTI_Type_Descriptor`; no
//   numeric value survives.
// * `FUN_0061bf0b`, `FUN_0061bf69` - malformed prologues (stores through
//   `unaff_EBX - 0x12b73` / `in_ECX + 0x44a8902`) and `bad instruction`
//   overlaps around the same lambda-registration funclet.

// ---------------------------------------------------------------------------
// 0x0061a140 / 0x0061a1aa / 0x0061a1e3 - option-bit dispatcher
// ---------------------------------------------------------------------------

/// Label words copied into the option-event record at `+0x10` by
/// `0x0061a140`, `0x0061a1aa` and `0x0061a1e3`, indexed by slot.
///
/// The three values are the literals stored at `ebp-0x20`, `ebp-0x18` and
/// `ebp-0x10`, and they are also the `FUN_009736b0` dispatch indices read back
/// from the event's `+0x10` word.
pub const A68_OPTION_LABELS: [u16; 3] = [5, 8, 0x0b];

/// Fallback handlers copied to `ebp-0x1c`, `ebp-0x14`, `ebp-0xc`, indexed by
/// slot.  The three globals sit `0xc` bytes apart (`0x01176da4`, `0x01176db0`,
/// `0x01176dbc`); each is called with the event record when its slot has no
/// `FUN_009736b0` handler.
pub const A68_OPTION_HANDLERS: [u32; 3] = [0x0117_6da4, 0x0117_6db0, 0x0117_6dbc];

/// `0x0061a140`: the guard accepts option ids 1 and 2 (`id < 3`) and 4.
/// Id 3 reaches the mask formula but is rejected before the event is built.
pub fn a68_option_is_valid(id: u32) -> bool {
    id != 0 && (id < 3 || id == 4)
}

/// `0x0061a140`: the flag mask of an option id.
///
/// ```c
/// mask  = (id & 1) * 2;
/// if (id & 4) mask |= 8;
/// if (id & 2) mask |= 0x20;
/// ```
///
/// The valid ids produce `1 -> 0x02`, `2 -> 0x20`, `4 -> 0x08`; those are
/// exactly the three bits of the `0x2a` group tested by the guard.
pub fn a68_option_mask(id: u32) -> u16 {
    let mut mask = ((id & 1) as u16) * 2;
    if id & 4 != 0 {
        mask |= 8;
    }
    if id & 2 != 0 {
        mask |= 0x20;
    }
    mask
}

/// `0x0061a140` / `0x0061a1aa` / `0x0061a1e3`: the raw slot expression.
///
/// ```c
/// slot = 0;
/// if (id != 1) slot = (id != 4) + 1;   // 1 for id 4, 2 otherwise
/// ```
pub fn a68_option_slot_raw(id: u32) -> usize {
    if id == 1 {
        0
    } else if id == 4 {
        1
    } else {
        2
    }
}

/// [`a68_option_slot_raw`] behind the `0x0061a140` validity guard.
pub fn a68_option_slot(id: u32) -> Option<usize> {
    if a68_option_is_valid(id) {
        Some(a68_option_slot_raw(id))
    } else {
        None
    }
}

/// `0x0061a140`: the label word for a slot.
pub fn a68_option_label(slot: usize) -> Option<u16> {
    A68_OPTION_LABELS.get(slot).copied()
}

/// `0x0061a140`: the fallback handler for a slot.
pub fn a68_option_handler(slot: usize) -> Option<u32> {
    A68_OPTION_HANDLERS.get(slot).copied()
}

/// The record built by the three dispatcher rows before `FUN_009736b0`.
///
/// The shipped record is 0x1c bytes: `+0x00`, `+0x04` and the qword at `+0x08`
/// are zeroed, the label word occupies `+0x10`, the packed query word sits at
/// the unaligned `+0x12`, and the payload is at `+0x18`.  The two bytes
/// between the query and the payload (`+0x16`/`+0x17`) are never written and
/// are not modelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68OptionEvent {
    /// `+0x00`, zeroed by the body.
    pub word0: u32,
    /// `+0x04`, zeroed by the body.
    pub word4: u32,
    /// `+0x08`, zeroed by the body.
    pub qword8: u64,
    /// `+0x10`: `A68_OPTION_LABELS[slot]` (or the caller's first label).
    pub label: u16,
    /// `+0x12`: `(query_high << 16) | query_low`.
    pub query: u32,
    /// `+0x18`: the `param_4` dword of the dispatcher.
    pub payload: u32,
}

/// `0x0061a140`: pack the two 16-bit results of `FUN_006bb610` and
/// `FUN_006bb620` into the event's `+0x12` dword.
///
/// The body writes the first result into the low word and the second into the
/// high word (`CONCAT22` twice).
pub fn a68_option_query(low: u16, high: u16) -> u32 {
    ((high as u32) << 16) | low as u32
}

/// Build the zeroed event record shared by `0x0061a140`, `0x0061a1aa` and
/// `0x0061a1e3`.
pub fn a68_option_event_record(
    label: u16,
    query_low: u16,
    query_high: u16,
    payload: u32,
) -> A68OptionEvent {
    A68OptionEvent {
        word0: 0,
        word4: 0,
        qword8: 0,
        label,
        query: a68_option_query(query_low, query_high),
        payload,
    }
}

/// Dispatch selected by `0x0061a140`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68OptionDispatch {
    /// Slot `0`/`1`/`2` of the label and handler tables.
    pub slot: usize,
    /// `A68_OPTION_LABELS[slot]`, also stored in the event.
    pub label: u16,
    /// `A68_OPTION_HANDLERS[slot]`.
    pub handler: u32,
    /// The record passed to `FUN_009736b0` and then to the handler.
    pub event: A68OptionEvent,
}

/// Result of `0x0061a140`: the possibly-cleared flag word plus the dispatch
/// that runs when the guard accepts the id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68OptionApply {
    /// `DAT_0119f5ec` after the body: the low word of the flag word.
    pub flags: u16,
    /// `None` when the id is invalid or another option bit of `0x2a` is set.
    pub dispatch: Option<A68OptionDispatch>,
}

/// `0x0061a140` (`FUN_0061a140`, 105 bytes): close one option.
///
/// ```c
/// if (id != 0 && (id < 3 || id == 4)) {
///   mask = (id & 1) * 2 | (id & 4 ? 8 : 0) | (id & 2 ? 0x20 : 0);
///   if ((~mask & DAT_0119f5ec & 0x2a) == 0) {
///     slot  = id == 1 ? 0 : (id == 4 ? 1 : 2);
///     label = {5, 8, 0xb}[slot];
///     event = { 0, 0, 0, label, (second << 16) | first, param_4 };
///     DAT_0119f5ec = ~mask & DAT_0119f5ec;
///     if (FUN_009736b0(&event) == 0 && handler[slot]) handler[slot](&event);
///   }
/// }
/// ```
///
/// The `FUN_009736b0` return value gates the fallback handler; the model returns
/// the dispatch and leaves that gate to the caller, because the routine's own
/// record is what the gate receives.  The flag word only loses its own bit,
/// so a hidden option leaves `flags` untouched.
pub fn a68_option_apply(
    flags: u16,
    id: u32,
    query_low: u16,
    query_high: u16,
    payload: u32,
) -> A68OptionApply {
    if !a68_option_is_valid(id) {
        return A68OptionApply {
            flags,
            dispatch: None,
        };
    }
    let mask = a68_option_mask(id);
    if (!mask & flags & 0x2a) != 0 {
        return A68OptionApply {
            flags,
            dispatch: None,
        };
    }
    let slot = a68_option_slot_raw(id);
    let label = A68_OPTION_LABELS[slot];
    let handler = A68_OPTION_HANDLERS[slot];
    let event = a68_option_event_record(label, query_low, query_high, payload);
    A68OptionApply {
        flags: !mask & flags,
        dispatch: Some(A68OptionDispatch {
            slot,
            label,
            handler,
            event,
        }),
    }
}

/// Result of the two dispatcher tail rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68OptionTail {
    /// The record built by the fragment.
    pub event: A68OptionEvent,
    /// Slot used to index the label and handler tables.
    pub slot: usize,
    /// `0x0061a1aa` writes the low word of its incoming flags dword to
    /// `DAT_0119f5ec`; `0x0061a1e3` leaves the global alone.
    pub flags: Option<u16>,
}

/// `0x0061a1aa` (`FUN_0061a1aa`, 57 bytes): the event-building tail of the
/// option dispatcher when slot 0's label comes from the incoming `eax` word.
///
/// ```c
/// labels   = { (short)eax, 8, 0xb };
/// handlers = { DAT_01176da4, DAT_01176db0, DAT_01176dbc };
/// DAT_0119f5ec = (undefined2)ebx;         // low word only
/// slot = 0; if (id != 1) slot = (id != 4) + 1;
/// event = { 0, 0, 0, labels[slot], (second << 16) | first, param_4 };
/// ```
///
/// The guard that precedes this block lives in `0x0061a140`; the fragment
/// builds the event unconditionally.
pub fn a68_option_event_with_first_label(
    id: u32,
    first_label: u16,
    flags_word: u32,
    query_low: u16,
    query_high: u16,
    payload: u32,
) -> A68OptionTail {
    let slot = a68_option_slot_raw(id);
    let labels = [first_label, A68_OPTION_LABELS[1], A68_OPTION_LABELS[2]];
    let label = labels[slot];
    A68OptionTail {
        event: a68_option_event_record(label, query_low, query_high, payload),
        slot,
        flags: Some(flags_word as u16),
    }
}

/// `0x0061a1e3` (`FUN_0061a1e3`, 153 bytes): the event-building tail when the
/// caller already owns the 8-byte-stride label table at `ebp-0x1c`.
///
/// ```c
/// *(ebp - 8) = DAT_01176dbc;              // only slot 2's handler is written here
/// slot = 0; if (id != 1) slot = (id != 4) + 1;
/// event = { 0, 0, 0, labels[slot], (second << 16) | first, param_4 };
/// ```
///
/// Slots 0 and 1 of the handler table, and all three labels, belong to the
/// caller frame; the labels enter as `labels`.
pub fn a68_option_event_from_labels(
    id: u32,
    labels: [u16; 3],
    query_low: u16,
    query_high: u16,
    payload: u32,
) -> A68OptionTail {
    let slot = a68_option_slot_raw(id);
    A68OptionTail {
        event: a68_option_event_record(labels[slot], query_low, query_high, payload),
        slot,
        flags: None,
    }
}

// ---------------------------------------------------------------------------
// 0x0061a290 - 120-frame timeline emitter
// ---------------------------------------------------------------------------

/// Signed magnitude exactly as `0x0061a290` computes it:
/// `(v ^ (v >> 31)) - (v >> 31)` with 32-bit wrapping.
pub fn a68_abs_i32(value: i32) -> i32 {
    (value ^ (value >> 31)).wrapping_sub(value >> 31)
}

/// Result of one `0x0061a290` step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A68TimelineStep {
    /// `DAT_00fa94c0` after the step.  This is in `-0x77..=0x77` for every
    /// input except the wrapping `i32::MIN` magnitude, which stays negative
    /// and stops the loop immediately.
    pub total: i32,
    /// Event ids emitted by the step, in order.
    pub ids: Vec<u16>,
}

/// `0x0061a290` (`FUN_0061a290`, 185 bytes): add a delta to the 120-frame
/// accumulator and emit one event per crossed interval.
///
/// ```c
/// DAT_00fa94c0 += delta;
/// while (0x77 < abs(DAT_00fa94c0)) {
///     emit((DAT_00fa94c0 < 1) + 0x11);      // 0x11 forward, 0x12 backward
///     DAT_00fa94c0 = DAT_00fa94c0 < 1
///                  ? DAT_00fa94c0 + 0x78
///                  : DAT_00fa94c0 - 0x78;
/// }
/// ```
///
/// The accumulator is pulled toward zero by `0x78` per emission, so the
/// result is the remainder below 120 frames in magnitude: `1000` becomes
/// `40` after eight `0x11` events, `-1000` becomes `-40` after eight `0x12`
/// events.  The wrapping magnitude of `i32::MIN` is still negative, so that
/// input emits nothing and is returned unchanged.
///
/// Each emission builds the same 0x1c-byte record as `0x0061a140` with the id
/// in the `+0x10` word, passes it to `FUN_009736b0`, and calls
/// `DAT_01176dd4` for the `0x11` case or `DAT_01176dd8` for `0x12`; only the
/// id and the new total are returned here.
pub fn a68_timeline_advance(total: i32, delta: i32) -> A68TimelineStep {
    let mut total = total.wrapping_add(delta);
    let mut ids = Vec::new();
    while 0x77 < a68_abs_i32(total) {
        ids.push(if total < 1 { 0x12 } else { 0x11 });
        total = if total < 1 {
            total.wrapping_add(0x78)
        } else {
            total.wrapping_sub(0x78)
        };
    }
    A68TimelineStep { total, ids }
}

// ---------------------------------------------------------------------------
// 0x0061a370 / 0x0061a3f4 - caller-record stores
// ---------------------------------------------------------------------------

/// The `+0x130` / `+0x134` dword pair written by `0x0061a370`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A68TailPair {
    pub at_130: u32,
    pub at_134: u32,
}

/// `0x0061a370` (`FUN_0061a370`, 25 bytes): store two dwords at `+0x130` and
/// `+0x134`.
///
/// ```c
/// *(undefined4 *)(this + 0x130) = param_1;
/// *(undefined4 *)(this + 0x134) = param_2;
/// ```
pub fn a68_store_tail_pair(record: &mut A68TailPair, first: u32, second: u32) {
    record.at_130 = first;
    record.at_134 = second;
}

/// The value-range header initialised by `0x0061a3f4`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68ValueRange {
    /// `+0x10`, from `eax`.
    pub low: u32,
    /// `+0x14`, from `edx`.
    pub high: u32,
    /// `+0x18`, always `-1`.
    pub minus_one: i32,
    /// `+0x1c`, always `0x7fffffff`.
    pub max: i32,
    /// `+0x20`, always zero.
    pub flag: u8,
}

/// `0x0061a3f4` (`FUN_0061a3f4`, 29 bytes): initialise the value range.
///
/// ```c
/// *(this + 0x10) = eax;  *(this + 0x14) = edx;
/// *(this + 0x18) = -1;   *(this + 0x1c) = 0x7fffffff;
/// *(byte *)(this + 0x20) = 0;
/// ```
///
/// The body ends in the out-of-window `FUN_00c66860` tail; only the stores are
/// reproduced.
pub fn a68_range_init(low: u32, high: u32) -> A68ValueRange {
    A68ValueRange {
        low,
        high,
        minus_one: -1,
        max: 0x7fff_ffff,
        flag: 0,
    }
}

// ---------------------------------------------------------------------------
// 0x0061a4f0 - mode-3 abort gate
// ---------------------------------------------------------------------------

/// `0x0061a4f0` (`FUN_0061a4f0`, 34 bytes): the fatal-report gate.
///
/// ```c
/// if (FUN_00623320(this) == 3) {
///     if (FUN_00709680() != 0) FUN_00c66860();   // fatal report
/// }
/// ```
///
/// Both helper results enter as parameters; the routine contributes only the
/// `== 3` comparison and the `&&`.
pub fn a68_mode3_abort(mode: i32, report: bool) -> bool {
    mode == 3 && report
}

// ---------------------------------------------------------------------------
// 0x0061a575 - 64-bit delta divided into hours
// ---------------------------------------------------------------------------

/// `0x0061a575`: the 64-bit divisor is `0x0000034630b8a000`, which is exactly
/// 3,600,000,000,000 - one hour in nanoseconds.
pub const A68_NS_PER_HOUR: i64 = 3_600_000_000_000;

/// Quotient and remainder of the signed 64-bit division in `0x0061a575`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68Division {
    /// `edx:eax` after the shipped `__alldvrm`.
    pub quotient: i64,
    /// `ecx:ebx` after the shipped `__alldvrm`.
    pub remainder: i64,
}

/// `0x0061a575` (`FUN_0061a575`, 59 bytes): subtract a 64-bit base and divide
/// by one hour in nanoseconds.
///
/// ```c
/// lo = eax - *(u32 *)(this + 0x10);
/// hi = (edx - *(i32 *)(this + 0x14)) - (eax < *(u32 *)(this + 0x10));
/// FUN_00c87550(lo, hi, 0x30b8a000, 0x346);   // signed 64-bit division
/// ```
///
/// The borrow in the high word is exactly a wrapping 64-bit subtraction, and
/// the divisor pair `(0x30b8a000, 0x346)` is `3_600_000_000_000`.
pub fn a68_delta_div(lo: u32, hi: u32, base_lo: u32, base_hi: u32) -> A68Division {
    let value = (((hi as u64) << 32) | lo as u64) as i64;
    let base = (((base_hi as u64) << 32) | base_lo as u64) as i64;
    let delta = value.wrapping_sub(base);
    A68Division {
        quotient: delta / A68_NS_PER_HOUR,
        remainder: delta % A68_NS_PER_HOUR,
    }
}

// ---------------------------------------------------------------------------
// 0x0061a880 / 0x0061a9a0 / 0x0061a9b5 - variant apply helper
// ---------------------------------------------------------------------------

/// The stores made by the `0x0061a880` constructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68VariantInit {
    /// `+0x00`: written `0` and then `1`; the final value is `1`.
    pub word0: u32,
    /// `+0x04`: the `*param_1` byte.
    pub byte4: u8,
    /// `+0x20`, always `1`.
    pub word20: u32,
    /// `+0x24`, always `0`.
    pub word24: u32,
    /// The index read back from `+0x00` for the `UNK_0061a980` jump table.
    /// The table contents are not part of the decompilation.
    pub jump_slot: u32,
}

/// `0x0061a880` (`FUN_0061a880`, 95 bytes): initialise a variant record.
///
/// ```c
/// *this = 0;
/// *(byte *)(this + 1) = *param_1;   // byte at +0x04
/// *this = 1;                        // dword at +0x00
/// this[8] = 1;                      // dword at +0x20
/// this[9] = 0;                      // dword at +0x24
/// (*(&UNK_0061a980 + *this * 4))(); // indirect jump with index 1
/// ```
pub fn a68_variant_init(byte4: u8) -> A68VariantInit {
    A68VariantInit {
        word0: 1,
        byte4,
        word20: 1,
        word24: 0,
        jump_slot: 1,
    }
}

/// Result of the variant apply helper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68VariantApply {
    /// The byte returned in `al`: `0` when the indirect check failed, else `1`.
    pub result: u8,
    /// `+0x60` when the body writes it; `None` leaves the byte alone.
    pub byte60: Option<u8>,
    /// `0x0061a9a0` only: `*param_1`, copied into the scope-guard local at
    /// `ebp-0x2c`; `None` for `0x0061a9b5`.
    pub guard_byte: Option<u8>,
}

fn a68_variant_apply_core(
    slot_word: u32,
    virtual_ok: bool,
    shared_ok: bool,
    keep: bool,
    guard_byte: Option<u8>,
) -> A68VariantApply {
    let ptr = if slot_word & 1 != 0 {
        slot_word & !1
    } else {
        slot_word
    };
    if ptr != 0 && !virtual_ok {
        return A68VariantApply {
            result: 0,
            byte60: Some(1),
            guard_byte,
        };
    }
    if !shared_ok {
        return A68VariantApply {
            result: 1,
            byte60: Some(u8::from(keep)),
            guard_byte,
        };
    }
    A68VariantApply {
        result: 1,
        byte60: None,
        guard_byte,
    }
}

/// `0x0061a9b5` (`FUN_0061a9b5`, 213 bytes): apply one variant.
///
/// ```c
/// word = *(u32 *)(this + 0x78);
/// ptr  = (word & 1) ? (word & ~1) : word;
/// if (ptr != 0 && !vtable_slot_1(ptr)) {
///     clear(+0x24);                       // FUN_0053b060
///     *(byte *)(this + 0x60) = 1;
///     commit();                           // FUN_0053cac0
///     return 0;
/// }
/// if (!shared_ok()) {                     // FUN_0053b1e0
///     *(byte *)(this + 0x60) = keep;
///     if (!keep) reset();                 // FUN_006304a0
/// }
/// commit();                               // FUN_0053cac0
/// return 1;
/// ```
///
/// The three calls outside the window enter as `virtual_ok`, `shared_ok` and
/// `keep`; the routine's own contribution is the bit-0 pointer test, the
/// `+0x60` stores and the return byte.
pub fn a68_variant_apply(
    slot_word: u32,
    virtual_ok: bool,
    shared_ok: bool,
    keep: bool,
) -> A68VariantApply {
    a68_variant_apply_core(slot_word, virtual_ok, shared_ok, keep, None)
}

/// `0x0061a9a0` (`FUN_0061a9a0`, 21 bytes recovered): the same apply helper
/// when the caller passes a byte pointer.
///
/// ```c
/// guard = { 1, *param_1 };       // ebp-0x30 / ebp-0x2c
/// ... same body as 0x0061a9b5, with keep = param_2 ...
/// ```
///
/// The guard byte is only copied into the `FUN_0053af40` scope guard, so the
/// model returns it unchanged in [`A68VariantApply::guard_byte`].
pub fn a68_variant_apply_ptr(
    slot_word: u32,
    guard_byte: u8,
    virtual_ok: bool,
    shared_ok: bool,
    keep: bool,
) -> A68VariantApply {
    a68_variant_apply_core(slot_word, virtual_ok, shared_ok, keep, Some(guard_byte))
}

// ---------------------------------------------------------------------------
// FUN_00538810 growth policy and the string-assignment sites
// ---------------------------------------------------------------------------

/// Low 31-bit capacity mask of the shipped `std::string` header.
pub const A68_STRING_CAP_MASK: u32 = 0x7fff_ffff;

/// Half-capacity mask used by the growth addend.
pub const A68_STRING_GROW_MASK: u32 = 0x3fff_ffff;

/// `FUN_00538810`: grow a string header's capacity to at least `request`.
///
/// ```c
/// cap = *(u32 *)(this + 8);
/// if ((cap & 0x7fffffff) < request) {
///     grown = (cap >> 1 & 0x3fffffff) + (cap & 0x7fffffff);
///     if (grown < request) grown = request;
///     *(u32 *)(this + 8) = ((cap ^ grown) & 0x7fffffff) ^ cap;
/// }
/// ```
///
/// The top bit of `cap` is the inline-buffer tag and survives unchanged; only
/// the low 31 bits are replaced.
pub fn a68_grown_capacity(cap: u32, request: u32) -> u32 {
    let low = cap & A68_STRING_CAP_MASK;
    if low < request {
        let mut grown = (cap >> 1 & A68_STRING_GROW_MASK).wrapping_add(low);
        if grown < request {
            grown = request;
        }
        ((cap ^ grown) & A68_STRING_CAP_MASK) ^ cap
    } else {
        cap
    }
}

/// The capacity-only rows of the window: address -> immediate request passed
/// to `FUN_00538810`.
///
/// * `0x0061aa90` - 0x19 (25)
/// * `0x0061ab70` - 0x0e (14)
/// * `0x0061acb0` - 0x0c (12)
/// * `0x0061adf0` - 0x10 (16)
/// * `0x0061b090` - 0x08 (8)
pub const A68_RESERVE_SITES: [(u32, u32); 5] = [
    (0x0061_aa90, 0x19),
    (0x0061_ab70, 0x0e),
    (0x0061_acb0, 0x0c),
    (0x0061_adf0, 0x10),
    (0x0061_b090, 0x08),
];

/// The request recorded for a capacity-only site.
pub fn a68_site_request(site: u32) -> Option<u32> {
    A68_RESERVE_SITES
        .iter()
        .find(|(address, _)| *address == site)
        .map(|(_, request)| *request)
}

/// `0x0061aa90`, `0x0061ab70`, `0x0061acb0`, `0x0061adf0`, `0x0061b090`:
/// apply the shipped capacity policy with the site's immediate request.
///
/// Each body is a `FUN_00538810(immediate)` call preceded by an SEH frame; the
/// decompiler truncates the copy that follows, so the recoverable arithmetic
/// is exactly the capacity update.  `0x0061b090` additionally fetches the
/// `FUN_0062fd60` singleton (`&DAT_00f05618`) before the reserve, which is not
/// part of the returned value.
pub fn a68_reserve_site(site: u32, cap: u32) -> Option<u32> {
    a68_site_request(site).map(|request| a68_grown_capacity(cap, request))
}

/// `0x0061af30` / `0x0061af61`: the registry key literal.
pub const A68_CINEMATIC_SUBTITLES: &[u8] = b"cinematicSubtitlesEnabled";
/// `0x0061b1b0`: the registry key literal.
pub const A68_USE_KR_VARIANT: &[u8] = b"useKrVariant";
/// `0x0061b310`: the registry key literal.
pub const A68_ORIGINAL_CAMPAIGN: &[u8] = b"originalCampaign";
/// `0x0061b470`: the registry key literal.
pub const A68_ORIGINAL_UNIT_VO: &[u8] = b"originalUnitVO";

/// The result of assigning a NUL-terminated literal into a string header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68LiteralAssign {
    /// `strlen(literal)`, the inline scan's result.
    pub len: u32,
    /// Capacity after the `FUN_00538810` request of `len`.
    pub cap: u32,
}

/// `0x0061af30`, `0x0061af61`, `0x0061b1b0`, `0x0061b310`, `0x0061b470`:
/// the inlined `std::string = literal` prologue.
///
/// The compiler emits `strlen` as the `do { c = *p++; } while (c)` loop whose
/// end pointer lands after the NUL, so the scanned length is
/// `end - (start + 1)`.  That length is passed to `FUN_00538810`.
pub fn a68_assign_literal(cap: u32, literal: &[u8]) -> A68LiteralAssign {
    let len = a68_cstr_len(literal);
    A68LiteralAssign {
        len,
        cap: a68_grown_capacity(cap, len),
    }
}

/// `0x0061af30` (`FUN_0061af30`, 46 bytes): reserve
/// `"cinematicSubtitlesEnabled"` (25 bytes).
///
/// The body first calls `0x0061b090` (the 8-byte reserve of the enclosing
/// key) and then scans the literal; only the literal's scan and growth are
/// returned.
pub fn a68_reserve_cinematic_subtitles(cap: u32) -> A68LiteralAssign {
    a68_assign_literal(cap, A68_CINEMATIC_SUBTITLES)
}

/// The `0x0061af61` result: the assignment plus the frame byte the fragment
/// writes at `ebp-0xb9`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68LiteralArm {
    pub assign: A68LiteralAssign,
    /// `*(undefined1 *)(ebp - 0xb9) = 1`.
    pub flag_b9: bool,
}

/// `0x0061af61` (`FUN_0061af61`, 77 bytes): arm the frame flag and reserve
/// `"cinematicSubtitlesEnabled"`.
///
/// ```c
/// *(byte *)(ebp - 0xb9) = 1;
/// len = strlen("cinematicSubtitlesEnabled");   // 25
/// FUN_00538810(len);
/// ```
pub fn a68_assign_cinematic_subtitles(cap: u32) -> A68LiteralArm {
    A68LiteralArm {
        assign: a68_assign_literal(cap, A68_CINEMATIC_SUBTITLES),
        flag_b9: true,
    }
}

/// `0x0061b1b0` (`FUN_0061b1b0`, 126 bytes): reserve
/// `"useKrVariant"` (12 bytes).
pub fn a68_reserve_use_kr_variant(cap: u32) -> A68LiteralAssign {
    a68_assign_literal(cap, A68_USE_KR_VARIANT)
}

/// `0x0061b310` (`FUN_0061b310`, 126 bytes): reserve
/// `"originalCampaign"` (16 bytes).
pub fn a68_reserve_original_campaign(cap: u32) -> A68LiteralAssign {
    a68_assign_literal(cap, A68_ORIGINAL_CAMPAIGN)
}

/// `0x0061b470` (`FUN_0061b470`, 126 bytes): reserve
/// `"originalUnitVO"` (14 bytes).
pub fn a68_reserve_original_unit_vo(cap: u32) -> A68LiteralAssign {
    a68_assign_literal(cap, A68_ORIGINAL_UNIT_VO)
}

// ---------------------------------------------------------------------------
// 0x0061b170 - shutdown call list
// ---------------------------------------------------------------------------

/// The nine calls of `0x0061b170`, in order.  The first takes `1`; the other
/// eight take no arguments.
pub const A68_SHUTDOWN_CALLS: [(u32, Option<u32>); 9] = [
    (0x0070_12c0, Some(1)),
    (0x0077_e780, None),
    (0x0064_eeb0, None),
    (0x0064_ef30, None),
    (0x0062_0890, None),
    (0x0062_0b40, None),
    (0x0056_e6f0, None),
    (0x0056_f4e0, None),
    (0x006e_8750, None),
];

/// `0x0061b170` (`FUN_0061b170`, 56 bytes): the shutdown sequence.
///
/// ```c
/// FUN_007012c0(1);  FUN_0077e780();  FUN_0064eeb0();  FUN_0064ef30();
/// FUN_00620890();   FUN_00620b40();  FUN_0056e6f0();  FUN_0056f4e0();
/// FUN_006e8750();
/// ```
pub fn a68_shutdown_sequence() -> [(u32, Option<u32>); 9] {
    A68_SHUTDOWN_CALLS
}

// ---------------------------------------------------------------------------
// 0x0061b5d0 - storage-word test
// ---------------------------------------------------------------------------

/// `0x0061b5d0` (`FUN_0061b5d0`, 7 bytes): `return *this != 0`.
pub fn a68_slot_is_nonzero(word: u32) -> bool {
    word != 0
}

// ---------------------------------------------------------------------------
// 0x0061b6b0 - chain teardown order
// ---------------------------------------------------------------------------

/// One node of the `0x0061b6b0` teardown chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68ChainNode {
    /// `+0x04`: the recursive link (zero means no child).
    pub child: Option<usize>,
    /// `+0x34`: a tag word; the teardown branch runs when it is non-negative.
    pub word34: i32,
    /// `+0x18`: a tag word; the teardown branch runs when it is non-negative.
    pub word18: i32,
}

/// One node visited by [`a68_chain_free_order`], after its child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68ChainStep {
    pub index: usize,
    /// `-1 < word34`: the body enters the `FUN_0093ec70` allocator branch.
    pub releases_34: bool,
    /// `-1 < word18`: the body enters the `FUN_0093ec70` allocator branch.
    pub releases_18: bool,
}

/// `0x0061b6b0` (`FUN_0061b6b0`, 137 bytes): post-order teardown of a chain.
///
/// ```c
/// if (!node) return;
/// FUN_0061b6b0(*(u32 *)(node + 4));       // child first
/// if (-1 < *(i32 *)(node + 0x34)) FUN_0093ec70();   // heap-tagged word
/// if (-1 < *(i32 *)(node + 0x18)) FUN_0093ec70();   // heap-tagged word
/// FUN_0093ec70();                                   // release the node
/// ```
///
/// The two conditional branches are the shipped heap-buffer tests (the top bit
/// clear means the buffer is not inline).  `FUN_0093ec70` is the allocator
/// accessor, and the decompiler truncates each branch immediately after it, so
/// the model reports which branches run instead of performing the release.
/// The visited bitmap only exists so a malformed cyclic input cannot recurse
/// forever; retail performs no such check.
pub fn a68_chain_free_order(nodes: &[A68ChainNode], head: usize) -> Vec<A68ChainStep> {
    fn walk(
        nodes: &[A68ChainNode],
        index: usize,
        visited: &mut [bool],
        steps: &mut Vec<A68ChainStep>,
    ) {
        if index >= nodes.len() || visited[index] {
            return;
        }
        visited[index] = true;
        if let Some(child) = nodes[index].child {
            walk(nodes, child, visited, steps);
        }
        let node = nodes[index];
        steps.push(A68ChainStep {
            index,
            releases_34: node.word34 >= 0,
            releases_18: node.word18 >= 0,
        });
    }

    let mut steps = Vec::new();
    let mut visited = vec![false; nodes.len()];
    walk(nodes, head, &mut visited, &mut steps);
    steps
}

// ---------------------------------------------------------------------------
// 0x0061b760 / 0x0061b7a0 - literal compare and length-field reserve
// ---------------------------------------------------------------------------

/// The NUL scan emitted inline by `0x0061b760` and by the literal assignment
/// sites (`0x0061af30` and friends).
///
/// ```c
/// if (*p == 0) n = 0;
/// else { q = p; do { c = *q++; } while (c); n = q - (p + 1); }
/// ```
///
/// A slice without a NUL yields its full length; retail assumes termination.
pub fn a68_cstr_len(bytes: &[u8]) -> u32 {
    let mut len = 0;
    while len < bytes.len() && bytes[len] != 0 {
        len += 1;
    }
    len as u32
}

/// `0x0061b760` (`FUN_0061b760`, 58 bytes): compare `literal`'s scanned
/// length against the stored range.
///
/// ```c
/// len = *literal ? strlen(literal) : 0;
/// return range_compare(stored_low, stored_high, 0, stored_high,
///                      literal, len) == 0;
/// ```
///
/// The range-compare entry point lives outside the window; the local part is
/// the scan and the length it feeds, so the model is an equality test over the
/// first `len` bytes.  A zero-length range is treated as equal (the usual
/// range-compare convention the `len = 0` shortcut reaches).
pub fn a68_literal_matches(stored: &[u8], literal: &[u8]) -> bool {
    let len = a68_cstr_len(literal) as usize;
    stored.len() >= len && stored[..len] == literal[..len]
}

/// `0x0061b7a0` (`FUN_0061b7a0`, 106 bytes): reserve the length field of the
/// source string object.
///
/// ```c
/// FUN_00538810(*(undefined4 *)(param_2 + 4));
/// ```
///
/// The same request shape is used by `0x0061bd50`.
pub fn a68_reserve_length_field(cap: u32, length: u32) -> u32 {
    a68_grown_capacity(cap, length)
}

// ---------------------------------------------------------------------------
// 0x0061bae0 / 0x0061bc20 - deleting-destructor and byte-add leaves
// ---------------------------------------------------------------------------

/// `0x0061bae0` (`FUN_0061bae0`, 27 bytes): the MSVC deleting-destructor
/// flag test.
///
/// ```c
/// if ((flags & 1) != 0) FUN_00923e90();   // delete this
/// ```
///
/// `FUN_00923e90` is the delete-if-non-null entry; the model returns whether it
/// runs.
pub fn a68_delete_flag(flags: u8) -> bool {
    flags & 1 != 0
}

/// `0x0061bc20` (`FUN_0061bc20`, 5 bytes): `return al + ch` on raw bytes.
///
/// ```c
/// char FUN_0061bc20(void) { char in_AL; char in_CH; return in_AL + in_CH; }
/// ```
///
/// The result is the 8-bit wrapping sum interpreted as the signed `char` the
/// decompiler declares.
pub fn a68_add_bytes(al: u8, ch: u8) -> i8 {
    al.wrapping_add(ch) as i8
}

// ---------------------------------------------------------------------------
// 0x0061bd10 / 0x0061bd50 - bcCopyableFunction copy and list header
// ---------------------------------------------------------------------------

/// The two vftables written by `0x0061bd10`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A68Vtable {
    /// `bcCopyableFunctionImpl<bool, OS_FILE_DATA64_const&>::vftable`, stored
    /// first and overwritten immediately.
    Base,
    /// `bcCopyableFunctionObjectImpl<<lambda_69d86eae3b477236d04d62d2a6c1a7ae>&,
    /// bool, OS_FILE_DATA64_const&>::vftable`, the final tag.
    Derived,
}

/// A copied `bcCopyableFunctionObjectImpl` record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68FunctorCopy {
    /// Final vftable tag at `+0x00`.
    pub vtable: A68Vtable,
    /// `+0x04..+0x0f`: the 8-byte member at `+0x04` followed by the dword at
    /// `+0x0c`, i.e. 12 contiguous bytes.
    pub payload: [u8; 12],
}

/// `0x0061bd10` (`FUN_0061bd10`, 38 bytes): copy a callable record.
///
/// ```c
/// *out = bcCopyableFunctionImpl::vftable;    // transient
/// *(u64 *)(out + 4) = *(u64 *)(this + 4);
/// out[3] = *(u32 *)(this + 0xc);
/// *out = bcCopyableFunctionObjectImpl::vftable;
/// ```
///
/// The base vftable store is overwritten before the routine returns, so only
/// the derived tag is observable.
pub fn a68_functor_copy(payload: [u8; 12]) -> A68FunctorCopy {
    A68FunctorCopy {
        vtable: A68Vtable::Derived,
        payload,
    }
}

/// The self-referential `std::list` header written by `0x0061bd50`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68ListHeader {
    /// `+0x00`: `this` (the sentinel's next pointer).
    pub next: u32,
    /// `+0x04`: `this` (the sentinel's previous pointer).
    pub prev: u32,
    /// `+0x08`: element count, zero.
    pub size: u32,
}

/// Result of `0x0061bd50`: the header plus the string capacity it grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A68ListInit {
    pub header: A68ListHeader,
    /// The `+0x08` capacity word after the reserve.
    pub cap: u32,
}

/// `0x0061bd50` (`FUN_0061bd50`, 153 bytes): initialise a list sentinel and
/// reserve the source string's length.
///
/// ```c
/// *(u32 *)(param_1 + 0) = param_1;
/// *(u32 *)(param_1 + 4) = param_1;
/// *(u32 *)(param_1 + 8) = 0;
/// FUN_00538810(*(u32 *)(param_3 + 4));   // reserve on the ECX string
/// ```
///
/// `param_1` is the sentinel address and the reserve target is the string
/// header in `ecx`, whose current capacity enters as `cap`.
pub fn a68_list_init(address: u32, cap: u32, source_len: u32) -> A68ListInit {
    A68ListInit {
        header: A68ListHeader {
            next: address,
            prev: address,
            size: 0,
        },
        cap: a68_grown_capacity(cap, source_len),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent68 {
    use super::*;

    /// `0x0061a140`: the three valid ids, their masks and their slots.
    #[test]
    fn option_ids_map_to_the_0x2a_group() {
        for id in [0u32, 3, 5, 0xffff_ffff] {
            assert!(!a68_option_is_valid(id), "id {id} must be rejected");
            assert_eq!(a68_option_slot(id), None);
        }
        for id in [1u32, 2, 4] {
            assert!(a68_option_is_valid(id));
        }
        assert_eq!(
            (a68_option_mask(1), a68_option_mask(2), a68_option_mask(4)),
            (0x02, 0x20, 0x08)
        );
        // The raw formula still computes 0x22 for the rejected id 3.
        assert_eq!(a68_option_mask(3), 0x22);
        assert_eq!(
            (a68_option_slot(1), a68_option_slot(4), a68_option_slot(2)),
            (Some(0), Some(1), Some(2))
        );
        assert_eq!(A68_OPTION_LABELS, [5, 8, 0x0b]);
        assert_eq!(A68_OPTION_HANDLERS, [0x0117_6da4, 0x0117_6db0, 0x0117_6dbc]);
        assert_eq!(a68_option_label(2), Some(0x0b));
        assert_eq!(a68_option_handler(1), Some(0x0117_6db0));
        assert_eq!(a68_option_label(3), None);
    }

    /// `0x0061a140`: the guard is "no other option bit of 0x2a is set", and a
    /// successful apply clears only the id's own bit.
    #[test]
    fn option_apply_tests_the_other_group_bits() {
        // No bits: id 1 is visible, its own bit is already clear.
        let open = a68_option_apply(0, 1, 0x1111, 0x2222, 0xdead_beef);
        assert_eq!(open.flags, 0);
        let dispatch = open.dispatch.expect("id 1 is visible from a clear word");
        assert_eq!((dispatch.slot, dispatch.label), (0, 5));
        assert_eq!(dispatch.handler, 0x0117_6da4);
        assert_eq!(dispatch.event.query, 0x2222_1111);
        assert_eq!(dispatch.event.payload, 0xdead_beef);
        assert_eq!((dispatch.event.word0, dispatch.event.word4), (0, 0));
        assert_eq!(dispatch.event.qword8, 0);

        // Own bit set, no other bit: still visible, and the bit is cleared.
        let own = a68_option_apply(0x02, 1, 0, 0, 0);
        assert_eq!(own.flags, 0);
        assert!(own.dispatch.is_some());

        // Another option's bit set: hidden, flags untouched.
        let hidden = a68_option_apply(0x08, 1, 0, 0, 0);
        assert_eq!(hidden.flags, 0x08);
        assert_eq!(hidden.dispatch, None);

        // The whole group set: hidden and untouched.
        let all = a68_option_apply(0x2a, 2, 0, 0, 0);
        assert_eq!(all.flags, 0x2a);
        assert_eq!(all.dispatch, None);

        // Bits outside 0x2a never block the dispatch.
        let outside = a68_option_apply(0x00d4, 4, 0, 0, 0);
        assert_eq!(outside.flags, 0x00d4);
        assert!(outside.dispatch.is_some());
        assert_eq!(outside.dispatch.unwrap().slot, 1);

        // Id 3 is rejected before the mask is used.
        let invalid = a68_option_apply(0, 3, 0, 0, 0);
        assert_eq!(invalid.dispatch, None);
        assert_eq!(invalid.flags, 0);
    }

    /// `0x0061a1aa`: the slot-0 label comes from the incoming word and the
    /// low word of the incoming flag dword replaces the global.
    #[test]
    fn option_event_with_first_label_uses_the_incoming_word() {
        let tail = a68_option_event_with_first_label(1, 0x21, 0x1234_5678, 1, 2, 3);
        assert_eq!(tail.slot, 0);
        assert_eq!(tail.event.label, 0x21);
        assert_eq!(tail.event.query, 0x0002_0001);
        assert_eq!(tail.flags, Some(0x5678));

        let slot_two = a68_option_event_with_first_label(2, 0x21, 0, 0, 0, 0);
        assert_eq!(slot_two.slot, 2);
        assert_eq!(slot_two.event.label, 0x0b);
    }

    /// `0x0061a1e3`: the caller-owned label table is indexed by the slot and
    /// the global flag word is left alone.
    #[test]
    fn option_event_from_labels_indexes_the_caller_table() {
        let labels = [7u16, 9, 0x0d];
        let base = a68_option_event_from_labels(1, labels, 0, 0, 0);
        let four = a68_option_event_from_labels(4, labels, 0, 0, 0);
        let two = a68_option_event_from_labels(2, labels, 0, 0, 0);
        assert_eq!((base.slot, base.event.label), (0, 7));
        assert_eq!((four.slot, four.event.label), (1, 9));
        assert_eq!((two.slot, two.event.label), (2, 0x0d));
        assert_eq!(base.flags, None);
    }

    /// `0x0061a290`: the accumulator is pulled toward zero by 0x78 per event.
    #[test]
    fn timeline_advance_emits_per_crossed_interval() {
        let forward = a68_timeline_advance(1000, 0);
        assert_eq!(forward.total, 40);
        assert_eq!(forward.ids, vec![0x11; 8]);

        let backward = a68_timeline_advance(-1000, 0);
        assert_eq!(backward.total, -40);
        assert_eq!(backward.ids, vec![0x12; 8]);

        assert_eq!(a68_timeline_advance(0, 120).ids, vec![0x11]);
        assert_eq!(a68_timeline_advance(0, 120).total, 0);
        assert_eq!(a68_timeline_advance(119, 0).ids, Vec::<u16>::new());
        assert_eq!(a68_timeline_advance(120, 0).total, 0);
        assert_eq!(a68_timeline_advance(-120, 0).ids, vec![0x12]);
        assert_eq!(a68_timeline_advance(0, 0).ids, Vec::<u16>::new());
        assert_eq!(a68_timeline_advance(i32::MIN, 0).ids, Vec::<u16>::new());
    }

    /// `0x0061a370`: two dwords at `+0x130` and `+0x134`.
    #[test]
    fn tail_pair_stores_both_words() {
        let mut record = A68TailPair::default();
        a68_store_tail_pair(&mut record, 0x1122_3344, 0x5566_7788);
        assert_eq!(record.at_130, 0x1122_3344);
        assert_eq!(record.at_134, 0x5566_7788);
    }

    /// `0x0061a3f4`: five field initialisers.
    #[test]
    fn range_init_writes_the_fixed_sentinels() {
        let range = a68_range_init(0xaaaa_bbbb, 0xcccc_dddd);
        assert_eq!(range.low, 0xaaaa_bbbb);
        assert_eq!(range.high, 0xcccc_dddd);
        assert_eq!(range.minus_one, -1);
        assert_eq!(range.max, i32::MAX);
        assert_eq!(range.flag, 0);
    }

    /// `0x0061a4f0`: only mode 3 with a set report flag aborts.
    #[test]
    fn mode3_abort_is_the_conjunction() {
        assert!(a68_mode3_abort(3, true));
        assert!(!a68_mode3_abort(3, false));
        assert!(!a68_mode3_abort(2, true));
        assert!(!a68_mode3_abort(4, true));
    }

    /// `0x0061a575`: the wrapping 64-bit subtract and the 3.6e12 divisor.
    #[test]
    fn delta_div_uses_the_hour_divisor() {
        assert_eq!(A68_NS_PER_HOUR, 3_600_000_000_000);
        assert_eq!(
            a68_delta_div(0, 0, 0, 0),
            A68Division {
                quotient: 0,
                remainder: 0
            }
        );
        assert_eq!(
            a68_delta_div(1000, 0, 0, 0),
            A68Division {
                quotient: 0,
                remainder: 1000
            }
        );
        // 0x0000034630b8a000 is exactly one hour.
        assert_eq!(
            a68_delta_div(0x30b8_a000, 0x346, 0, 0),
            A68Division {
                quotient: 1,
                remainder: 0
            }
        );
        assert_eq!(
            a68_delta_div(0, 0, 0x30b8_a000, 0x346),
            A68Division {
                quotient: -1,
                remainder: 0
            }
        );
        // The borrow makes the two's complement wrap resolve to +1.
        assert_eq!(
            a68_delta_div(0, 0, 0xffff_ffff, 0xffff_ffff),
            A68Division {
                quotient: 0,
                remainder: 1
            }
        );
        assert_eq!(
            a68_delta_div(0x1234_5678, 0x9abc_def0, 0x1111_1111, 0x2222_2222),
            A68Division {
                quotient: 2_414_018,
                remainder: 1_223_933_166_951
            }
        );
    }

    /// `0x0061a880`: the five constructor stores and the jump index.
    #[test]
    fn variant_init_writes_the_recovered_fields() {
        let init = a68_variant_init(0x5a);
        assert_eq!(init.word0, 1);
        assert_eq!(init.byte4, 0x5a);
        assert_eq!(init.word20, 1);
        assert_eq!(init.word24, 0);
        assert_eq!(init.jump_slot, 1);
    }

    /// `0x0061a9b5` / `0x0061a9a0`: the bit-0 pointer test, the two `+0x60`
    /// stores and the return byte.
    #[test]
    fn variant_apply_follows_the_three_branches() {
        // Null slot: the indirect check is skipped.
        let null = a68_variant_apply(0, false, true, false);
        assert_eq!((null.result, null.byte60), (1, None));

        // Null slot and no shared state: the keep flag lands at +0x60.
        let null_keep = a68_variant_apply(0, false, false, false);
        assert_eq!((null_keep.result, null_keep.byte60), (1, Some(0)));
        let null_keep_on = a68_variant_apply(0, false, false, true);
        assert_eq!((null_keep_on.result, null_keep_on.byte60), (1, Some(1)));

        // Non-null slot with a failing indirect check.
        let bad = a68_variant_apply(0x1234, false, true, false);
        assert_eq!((bad.result, bad.byte60), (0, Some(1)));

        // Non-null slot and a passing check: +0x60 is untouched.
        let good = a68_variant_apply(0x1234, true, true, false);
        assert_eq!((good.result, good.byte60), (1, None));

        // The odd word is masked before the check, and keep = true survives.
        let masked = a68_variant_apply(0x1235, true, false, true);
        assert_eq!((masked.result, masked.byte60), (1, Some(1)));

        // The pointer variant copies its byte into the scope guard.
        let ptr = a68_variant_apply_ptr(0, 0xa5, false, true, false);
        assert_eq!(ptr.guard_byte, Some(0xa5));
        assert_eq!(a68_variant_apply(0, false, true, false).guard_byte, None);
    }

    /// `FUN_00538810` growth policy and the five immediate-request sites.
    #[test]
    fn reserve_sites_apply_the_growth_policy() {
        // Empty inline header (0x8000000f), request 16: 7 + 15 = 22 = 0x16.
        assert_eq!(a68_grown_capacity(0x8000_000f, 16), 0x8000_0016);
        assert_eq!(a68_grown_capacity(0x8000_000f, 15), 0x8000_000f);
        assert_eq!(a68_grown_capacity(0x8000_000f, 100), 0x8000_0064);
        // The half-capacity addend is 7 even without the inline tag.
        assert_eq!(a68_grown_capacity(0x0000_000f, 16), 0x0000_0016);
        assert_eq!(a68_grown_capacity(0x0000_0020, 8), 0x0000_0020);

        assert_eq!(a68_site_request(0x0061_aa90), Some(25));
        assert_eq!(a68_site_request(0x0061_ab70), Some(14));
        assert_eq!(a68_site_request(0x0061_acb0), Some(12));
        assert_eq!(a68_site_request(0x0061_adf0), Some(16));
        assert_eq!(a68_site_request(0x0061_b090), Some(8));
        assert_eq!(a68_site_request(0x0061_0000), None);

        assert_eq!(
            a68_reserve_site(0x0061_aa90, 0x8000_000f),
            Some(0x8000_0019)
        );
        // A 15-byte inline capacity already covers the 14-, 12- and 8-byte
        // requests, so those sites leave the header alone.
        assert_eq!(
            a68_reserve_site(0x0061_ab70, 0x8000_000f),
            Some(0x8000_000f)
        );
        assert_eq!(
            a68_reserve_site(0x0061_acb0, 0x8000_000f),
            Some(0x8000_000f)
        );
        assert_eq!(
            a68_reserve_site(0x0061_adf0, 0x8000_000f),
            Some(0x8000_0016)
        );
        assert_eq!(
            a68_reserve_site(0x0061_b090, 0x8000_000f),
            Some(0x8000_000f)
        );
        assert_eq!(a68_reserve_site(0x0061_b091, 0x8000_000f), None);
    }

    /// `0x0061af30`..=`0x0061b470`: the four key literals and their lengths.
    #[test]
    fn literal_assignments_scan_the_key_names() {
        assert_eq!(a68_cstr_len(A68_CINEMATIC_SUBTITLES) as usize, 25);
        assert_eq!(a68_cstr_len(A68_USE_KR_VARIANT) as usize, 12);
        assert_eq!(a68_cstr_len(A68_ORIGINAL_CAMPAIGN) as usize, 16);
        assert_eq!(a68_cstr_len(A68_ORIGINAL_UNIT_VO) as usize, 14);
        assert_eq!(A68_CINEMATIC_SUBTITLES, b"cinematicSubtitlesEnabled");
        assert_eq!(A68_USE_KR_VARIANT, b"useKrVariant");
        assert_eq!(A68_ORIGINAL_CAMPAIGN, b"originalCampaign");
        assert_eq!(A68_ORIGINAL_UNIT_VO, b"originalUnitVO");

        assert_eq!(
            a68_reserve_cinematic_subtitles(0x8000_000f),
            A68LiteralAssign {
                len: 25,
                cap: 0x8000_0019
            }
        );
        assert_eq!(a68_reserve_use_kr_variant(0x8000_000f).cap, 0x8000_000f);
        assert_eq!(a68_reserve_original_campaign(0x8000_000f).cap, 0x8000_0016);
        assert_eq!(a68_reserve_original_unit_vo(0x8000_000f).cap, 0x8000_000f);
    }

    /// `0x0061af61`: the extra frame byte is set beside the 25-byte reserve.
    #[test]
    fn cinematic_assignment_arms_the_frame_byte() {
        let armed = a68_assign_cinematic_subtitles(0);
        assert_eq!(armed.assign.len, 25);
        assert_eq!(armed.assign.cap, 25);
        assert!(armed.flag_b9);
    }

    /// `0x0061b170`: the nine calls in order, only the first with an argument.
    #[test]
    fn shutdown_sequence_keeps_the_call_order() {
        assert_eq!(
            a68_shutdown_sequence(),
            [
                (0x0070_12c0, Some(1)),
                (0x0077_e780, None),
                (0x0064_eeb0, None),
                (0x0064_ef30, None),
                (0x0062_0890, None),
                (0x0062_0b40, None),
                (0x0056_e6f0, None),
                (0x0056_f4e0, None),
                (0x006e_8750, None),
            ]
        );
    }

    /// `0x0061b5d0`: the storage-word test.
    #[test]
    fn slot_test_is_a_zero_compare() {
        assert!(!a68_slot_is_nonzero(0));
        assert!(a68_slot_is_nonzero(1));
        assert!(a68_slot_is_nonzero(0x8000_0000));
    }

    /// `0x0061b6b0`: children are visited before their parent, and the two
    /// tag words choose the release branches.
    #[test]
    fn chain_free_order_is_post_order() {
        let nodes = [
            A68ChainNode {
                child: Some(1),
                word34: -1,
                word18: 5,
            },
            A68ChainNode {
                child: Some(2),
                word34: -1,
                word18: -1,
            },
            A68ChainNode {
                child: None,
                word34: 0,
                word18: 7,
            },
        ];
        let order = a68_chain_free_order(&nodes, 0);
        assert_eq!(
            order,
            vec![
                A68ChainStep {
                    index: 2,
                    releases_34: true,
                    releases_18: true
                },
                A68ChainStep {
                    index: 1,
                    releases_34: false,
                    releases_18: false
                },
                A68ChainStep {
                    index: 0,
                    releases_34: false,
                    releases_18: true
                },
            ]
        );
        // An empty head and an out-of-range head visit nothing.
        assert!(a68_chain_free_order(&nodes, 3).is_empty());
        assert!(a68_chain_free_order(&[], 0).is_empty());
    }

    /// `0x0061b760`: the NUL scan feeds a prefix equality test.
    #[test]
    fn literal_matches_scans_then_compares() {
        assert_eq!(a68_cstr_len(b"abc\0def"), 3);
        assert_eq!(a68_cstr_len(b""), 0);
        assert_eq!(a68_cstr_len(b"abc"), 3);

        assert!(a68_literal_matches(b"", b""));
        assert!(a68_literal_matches(b"abc", b""));
        assert!(a68_literal_matches(b"abc\0def", b"abc"));
        assert!(!a68_literal_matches(b"abd", b"abc"));
        assert!(!a68_literal_matches(b"ab", b"abc"));
        assert!(a68_literal_matches(
            b"cinematicSubtitlesEnabled\0",
            A68_CINEMATIC_SUBTITLES
        ));
    }

    /// `0x0061b7a0`: the length-field request is the same growth policy.
    #[test]
    fn reserve_length_field_uses_the_source_length() {
        assert_eq!(a68_reserve_length_field(0x8000_000f, 0), 0x8000_000f);
        assert_eq!(a68_reserve_length_field(0x8000_000f, 16), 0x8000_0016);
        assert_eq!(a68_reserve_length_field(0x8000_000f, 100), 0x8000_0064);
    }

    /// `0x0061bae0` / `0x0061bc20`: the two register-level leaves.
    #[test]
    fn destructor_flag_and_byte_add() {
        assert!(!a68_delete_flag(0));
        assert!(a68_delete_flag(1));
        assert!(!a68_delete_flag(2));
        assert!(a68_delete_flag(3));

        assert_eq!(a68_add_bytes(0x7f, 0x01), -128);
        assert_eq!(a68_add_bytes(0xff, 0x02), 1);
        assert_eq!(a68_add_bytes(0, 0), 0);
        assert_eq!(a68_add_bytes(0x10, 0xf0), 0);
    }

    /// `0x0061bd10`: the 12-byte payload and the final derived vftable.
    #[test]
    fn functor_copy_keeps_the_payload() {
        let payload = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let copy = a68_functor_copy(payload);
        assert_eq!(copy.payload, payload);
        assert_eq!(copy.vtable, A68Vtable::Derived);
    }

    /// `0x0061bd50`: the sentinel points at itself and the reserve uses the
    /// source length.
    #[test]
    fn list_init_writes_the_sentinel_and_reserves() {
        let init = a68_list_init(0x2000, 0x8000_000f, 16);
        assert_eq!(
            init.header,
            A68ListHeader {
                next: 0x2000,
                prev: 0x2000,
                size: 0
            }
        );
        assert_eq!(init.cap, 0x8000_0016);
        assert_eq!(a68_list_init(0x40, 0x8000_000f, 0).cap, 0x8000_000f);
    }
}
