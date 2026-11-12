#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent80.rs
// ===========================================================================
// Translation batch 80.  Only this file is edited by the batch.
//
// Batch 80 owns `0x00632000..=0x00633fff`.  The ledger lists 45
// `decompiled_uncited` rows in that window; 23 are translated below and the
// 22 rows that are not are listed at the end with their reason.
//
// Most of the window is MSVC `__finally` / catch funclets from one MFC glue
// dialog.  The funclet bodies address their state through the caller's
// `EBP`/`EDI` and end either in `FUN_00c86b6e` (the SEH dispatcher) or in
// `FUN_00538810` (the dialog's string setter).  Where the row contains only
// that frame plumbing it is skipped; where it contains a self-contained
// selection, arithmetic or table walk, that part is translated as a value
// type plus free functions.  No `sim` record gains a field.
//
// Recovered state:
//
// | owner                    | words                                                       |
// |--------------------------|-------------------------------------------------------------|
// | glue value record        | `+0x0c` id, `+0x10` intro, `+0x14` title, `+0x18` enabled, `+0x1c` two dwords, `+0x24` zoom |
// | glue cache               | same six slots plus a dirty byte (`EBP-0x2a9`)               |
// | priority heap            | `+0x00` base, `+0x08` count (`DAT_00faa1f0`), entry `{key, value}` |
// | drained order            | `DAT_00faa1e4` value array, `DAT_00faa1f4` upper index       |
// | report marshaller        | `+0x00` vftable, `+0x04` payload                              |
//
// Address to function:
//
// * `0x00632080` [`a80_vector_range`]
// * `0x0063225b` [`a80_fixed20_from_i32`]
// * `0x006322b1` [`a80_field_0x20_to_f64`]
// * `0x006322e9` [`a80_glue_refresh_all`]
// * `0x00632324` [`a80_glue_refresh_title`]
// * `0x006323ec` [`a80_glue_refresh_enabled`]
// * `0x00632401` [`a80_glue_refresh_enabled_after_begin`]
// * `0x006324d1` [`a80_glue_refresh_enabled_unconditional`]
// * `0x006324e6` [`a80_glue_refresh_enabled_unconditional_after_begin`]
// * `0x006325b0` [`a80_glue_refresh_zoom`]
// * `0x006325c5` [`a80_glue_refresh_zoom_after_begin`]
// * `0x00632690` [`a80_glue_commit`]
// * `0x006326a5` [`a80_glue_commit_after_begin`]
// * `0x00633050` [`a80_frame_accounting`]
// * `0x006335b0` [`a80_shutdown_gate`]
// * `0x00633620` [`a80_string_grow`]
// * `0x006339a0` [`A80SchedulerHeap::a80_reset`]
// * `0x006339c0` [`A80SchedulerHeap::a80_drain`]
// * `0x00633ac0` [`A80SchedulerHeap::a80_push`]
// * `0x00633b80` [`A80SchedulerHeap::a80_take_sorted`]
// * `0x00633ba0` [`A80ReportLogMarshaller::a80_new`]
// * `0x00633ca0` [`a80_renderer_name_source`]
// * `0x00633df0` [`a80_record_clone`]
//
// The translatable glue rows all belong to one refresh routine.  The shipped
// code compares the live record (`EDI`) with a cached copy on the stack,
// reports the changed controls through `FUN_00538810` and rewrites the cache.
// The `strlen` loop that precedes every report is the compiler's inline
// `strlen`; it is reproduced by [`a80_literal_len`] and the literal lengths.

// ---------------------------------------------------------------------------
// 0x00632080 - empty vector range check
// ---------------------------------------------------------------------------

/// `0x00632080`: the "vector must be empty" assertion.
///
/// ```c
/// if (*(int *)(ecx + 0x50) !=
///     *(int *)(ecx + 0x50) + *(int *)(ecx + 0x54) * 0x38)
///     FUN_00c86731(...);       // throw
/// FUN_00c86b6e();              // terminate
/// ```
///
/// The two `+0x50` words are the `begin` pointer and `+0x54` the element
/// count; the stride is `0x38`.  The comparison is performed on 32-bit
/// addresses, so the arithmetic is reproduced with wrapping `u32` values.
/// A non-empty range takes the throw path, an empty one the terminate path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A80VectorRange {
    /// `begin + count * 0x38`, with 32-bit wrapping.
    pub end: u32,
    /// `end != begin`, i.e. the range is not empty.
    pub nonempty: bool,
}

/// See [`A80VectorRange`].
pub fn a80_vector_range(begin: u32, count: i32) -> A80VectorRange {
    let offset = count.wrapping_mul(0x38) as u32;
    let end = begin.wrapping_add(offset);
    A80VectorRange {
        end,
        nonempty: end != begin,
    }
}

// ---------------------------------------------------------------------------
// 0x0063225b / 0x006322b1 - glue value conversions
// ---------------------------------------------------------------------------

/// The sign-indexed table at `0x00d33180`: `[0.0, 4294967296.0]`.
///
/// `4294967296.0` is `2^32`; the table is the compiler's branch-free
/// unsigned-int-to-double conversion.  The index is `-(value >> 31)`, so a
/// negative value adds `2^32` to its signed widening.
pub const A80_SIGN_BIAS: [f64; 2] = [0.0, 4_294_967_296.0];

/// `2^-20`, the multiplier used by `0x0063225b` (`9.536743e-07`).
pub const A80_FIXED20_SCALE: f64 = 9.536_743_164_062_5e-7;

/// `0x0063225b`: convert the `i32` at the record's `+0x1c` to a `Q12.20`
/// double.
///
/// ```c
/// (double)((float)((double)value +
///                  DAT_00d33180[-(value >> 31)]) * 9.536743e-07)
/// ```
///
/// The `+2^32` table entry is selected by the sign bit, the widened value is
/// narrowed to `f32`, and the power-of-two multiply moves the binary point
/// twenty places left.  Only the conversion is in the row; the call target
/// `FUN_00610640` that stores the result is not.
pub fn a80_fixed20_from_i32(value: i32) -> f64 {
    let bias = A80_SIGN_BIAS[((value >> 31) & 1) as usize];
    (((value as f64) + bias) as f32) as f64 * A80_FIXED20_SCALE
}

/// `0x006322b1`: widen the `f32` at the record's `+0x20` to a `double`.
///
/// `FUN_00610640(EBP - 0x38, (double)*(float *)(EDI + 0x20))` is the entire
/// row.  The bits are taken as an argument so the conversion stays exact,
/// including infinities and NaNs.
pub fn a80_field_0x20_to_f64(bits: u32) -> f64 {
    f32::from_bits(bits) as f64
}

// ---------------------------------------------------------------------------
// Glue label tables and refresh plans: 0x006322e9 .. 0x006326a5
// ---------------------------------------------------------------------------

/// Entry count of the `PTR_s_GLUE_TITLE_00d3bdf8` table.
pub const A80_GLUE_TITLE_COUNT: u32 = 0x23;

/// Entry count of the `PTR_s_GAME_INTRO_00f05628` table.
pub const A80_GAME_INTRO_COUNT: u32 = 0x0c;

/// The `"Enabled"` literal selected at `+0x18 != 0`.
pub const A80_ENABLED: &str = "Enabled";

/// The `"Disabled"` literal selected at `+0x18 == 0`.
pub const A80_DISABLED: &str = "Disabled";

/// The `"4x"` literal selected when the `+0x24` zoom word is `4`.
pub const A80_ZOOM_4X: &str = "4x";

/// The `"2x"` literal selected for every other zoom word.
pub const A80_ZOOM_2X: &str = "2x";

/// The `"Unknown"` fallback used past both table bounds.
pub const A80_UNKNOWN: &str = "Unknown";

/// A label produced by one of the glue rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A80Label {
    /// The in-range index into the row's string-pointer table.  The table
    /// literals themselves are not part of the decompilation, so only the
    /// index and the bound test are reproduced.
    TableEntry(u32),
    /// The `"Unknown"` literal used when the index is out of range.
    Unknown,
}

/// `0x006322e9`/`0x00632324`/`0x006323ec`/`0x00632401` title selection:
/// `(uint)index < 0x23` selects `PTR_s_GLUE_TITLE[index]`, otherwise the
/// `"Unknown"` literal.
pub fn a80_glue_title_label(index: u32) -> A80Label {
    if index < A80_GLUE_TITLE_COUNT {
        A80Label::TableEntry(index)
    } else {
        A80Label::Unknown
    }
}

/// `0x006322e9`/`0x00632324` intro selection:
/// `(uint)index < 0x0c` selects `PTR_s_GAME_INTRO[index]`, otherwise the
/// `"Unknown"` literal.
pub fn a80_game_intro_label(index: u32) -> A80Label {
    if index < A80_GAME_INTRO_COUNT {
        A80Label::TableEntry(index)
    } else {
        A80Label::Unknown
    }
}

/// `0x006322e9`/`0x00632324`/`0x006323ec`/`0x00632401` enabled selection:
/// `"Enabled"` when the byte at `+0x18` is non-zero, `"Disabled"` otherwise.
pub fn a80_enabled_label(enabled: bool) -> &'static str {
    if enabled { A80_ENABLED } else { A80_DISABLED }
}

/// Zoom selection: `"4x"` when `+0x24 == 4`, `"2x"` for every other value
/// (including zero and values above four).
pub fn a80_zoom_label(zoom: u32) -> &'static str {
    if zoom == 4 { A80_ZOOM_4X } else { A80_ZOOM_2X }
}

/// The inline `strlen` result used by every report call.
///
/// The shipped loop is `do { c = *p++; } while (c != 0); len = p - (s + 1);`,
/// i.e. `strlen` including the empty-string case.
pub fn a80_literal_len(text: &str) -> usize {
    text.as_bytes()
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(text.len())
}

/// The live glue record (`EDI` in the rows).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A80GlueState {
    /// `+0x0c`.
    pub field_c: i32,
    /// `+0x10`.
    pub field_10: u32,
    /// `+0x14`.
    pub field_14: u32,
    /// `+0x18`, one byte in the shipped layout.
    pub enabled: bool,
    /// `+0x1c`, copied as one `qword`.
    pub field_1c: u64,
    /// `+0x24`.
    pub zoom: u32,
}

/// The cached copy kept on the handler's stack (`EBP-0x2d8 .. EBP-0x2b4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A80GlueCache {
    /// Cached `+0x0c`.
    pub field_c: i32,
    /// Cached `+0x10`.
    pub field_10: u32,
    /// Cached `+0x14`.
    pub field_14: u32,
    /// Cached `+0x18`.
    pub enabled: bool,
    /// Cached `+0x1c`.
    pub field_1c: u64,
    /// Cached `+0x24`.
    pub zoom: u32,
    /// The dirty byte at `EBP-0x2a9`; the shipped constructor starts it set.
    pub dirty: bool,
}

impl A80GlueCache {
    /// `0x006322e9`..`0x006326a5`: the fresh handler cache.  The decompilation
    /// only exposes stores, but the first `0x006322e9` guard tests the byte
    /// before any store, so a dirty initial value is the only useful model.
    pub fn a80_new() -> Self {
        A80GlueCache {
            dirty: true,
            ..A80GlueCache::default()
        }
    }

    /// The snapshot written by every `Commit` step: all six record slots and
    /// `dirty = 0`.
    pub fn a80_store_state(&mut self, state: &A80GlueState) {
        self.field_c = state.field_c;
        self.field_10 = state.field_10;
        self.field_14 = state.field_14;
        self.enabled = state.enabled;
        self.field_1c = state.field_1c;
        self.zoom = state.zoom;
        self.dirty = false;
    }
}

/// One observable action of a glue refresh row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A80GlueStep {
    /// `FUN_00610730` stores the `+0x0c` value into the edited control.
    Value(i32),
    /// `FUN_00538810("Enabled" | "Disabled")`.
    Enabled(&'static str),
    /// `FUN_00538810("4x" | "2x")`.
    Zoom(&'static str),
    /// `FUN_00538810(PTR_s_GLUE_TITLE[index] | "Unknown")`.
    Title(A80Label),
    /// `FUN_00538810(PTR_s_GAME_INTRO[index] | "Unknown")`.
    Intro(A80Label),
    /// `FUN_00633320` followed by the cache snapshot; terminal in the row.
    Commit,
}

/// Result of one refresh row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A80GlueRefresh {
    /// `1` when the row starts with `(*(*this + 0x10))()`, otherwise `0`.
    pub vtable_calls: u8,
    /// The report/commit sequence, in shipped order.
    pub steps: Vec<A80GlueStep>,
    /// The cache after the row (only a `Commit` changes it).
    pub cache: A80GlueCache,
}

impl A80GlueRefresh {
    fn new(cache: &A80GlueCache, vtable_calls: u8) -> Self {
        A80GlueRefresh {
            vtable_calls,
            steps: Vec::new(),
            cache: *cache,
        }
    }
}

/// `0x006322e9`: full refresh of the edited glue value.
///
/// ```c
/// if (dirty || record.id != cache.id) { FUN_00610730(record.id); return; }
/// if (record.intro == cache.intro) {
///     if (record.title == cache.title) {
///         if (record.enabled != cache.enabled) report(enabled_label);
///         if (record.zoom != cache.zoom)       report(zoom_label);
///         FUN_00633320(...); cache = record; dirty = 0; return;
///     }
///     report(title < 0x23 ? table[title] : "Unknown");
/// }
/// report(intro < 0x0c ? table[intro] : "Unknown");
/// ```
pub fn a80_glue_refresh_all(state: &A80GlueState, cache: &mut A80GlueCache) -> A80GlueRefresh {
    let mut out = A80GlueRefresh::new(cache, 0);
    if cache.dirty || state.field_c != cache.field_c {
        out.steps.push(A80GlueStep::Value(state.field_c));
        return out;
    }
    if state.field_10 == cache.field_10 {
        if state.field_14 == cache.field_14 {
            if state.enabled != cache.enabled {
                out.steps
                    .push(A80GlueStep::Enabled(a80_enabled_label(state.enabled)));
            }
            if state.zoom != cache.zoom {
                out.steps
                    .push(A80GlueStep::Zoom(a80_zoom_label(state.zoom)));
            }
            out.steps.push(A80GlueStep::Commit);
            cache.a80_store_state(state);
            out.cache = *cache;
            return out;
        }
        out.steps
            .push(A80GlueStep::Title(a80_glue_title_label(state.field_14)));
    }
    out.steps
        .push(A80GlueStep::Intro(a80_game_intro_label(state.field_10)));
    out.cache = *cache;
    out
}

/// `0x00632324`: refresh after the value edit, guarded by the dirty byte and
/// the intro word.
///
/// ```c
/// if (dirty == 0 && record.intro == cache.intro) {
///     if (record.title == cache.title) { ... enabled/zoom/commit ... return; }
///     report(title_label);
/// }
/// report(intro_label);
/// ```
pub fn a80_glue_refresh_title(state: &A80GlueState, cache: &mut A80GlueCache) -> A80GlueRefresh {
    let mut out = A80GlueRefresh::new(cache, 0);
    if !cache.dirty && state.field_10 == cache.field_10 {
        if state.field_14 == cache.field_14 {
            if state.enabled != cache.enabled {
                out.steps
                    .push(A80GlueStep::Enabled(a80_enabled_label(state.enabled)));
            }
            if state.zoom != cache.zoom {
                out.steps
                    .push(A80GlueStep::Zoom(a80_zoom_label(state.zoom)));
            }
            out.steps.push(A80GlueStep::Commit);
            cache.a80_store_state(state);
            out.cache = *cache;
            return out;
        }
        out.steps
            .push(A80GlueStep::Title(a80_glue_title_label(state.field_14)));
    }
    out.steps
        .push(A80GlueStep::Intro(a80_game_intro_label(state.field_10)));
    out.cache = *cache;
    out
}

fn a80_enabled_zoom_steps(state: &A80GlueState, cache: &A80GlueCache, out: &mut A80GlueRefresh) {
    if state.enabled != cache.enabled {
        out.steps
            .push(A80GlueStep::Enabled(a80_enabled_label(state.enabled)));
    }
    if state.zoom != cache.zoom {
        out.steps
            .push(A80GlueStep::Zoom(a80_zoom_label(state.zoom)));
    }
}

/// `0x006323ec`: finish the enabled/zoom edit and then refresh the title.
///
/// ```c
/// if (dirty == 0 && record.title == cache.title) {
///     ... enabled/zoom reports ...
///     FUN_00633320(...); cache = record; dirty = 0; return;
/// }
/// report(title < 0x23 ? table[title] : "Unknown");
/// ```
pub fn a80_glue_refresh_enabled(state: &A80GlueState, cache: &mut A80GlueCache) -> A80GlueRefresh {
    a80_glue_refresh_enabled_impl(state, cache, 0)
}

/// `0x00632401`: [`a80_glue_refresh_enabled`] preceded by the virtual call
/// `(*(*this + 0x10))()`.
pub fn a80_glue_refresh_enabled_after_begin(
    state: &A80GlueState,
    cache: &mut A80GlueCache,
) -> A80GlueRefresh {
    a80_glue_refresh_enabled_impl(state, cache, 1)
}

fn a80_glue_refresh_enabled_impl(
    state: &A80GlueState,
    cache: &mut A80GlueCache,
    vtable_calls: u8,
) -> A80GlueRefresh {
    let mut out = A80GlueRefresh::new(cache, vtable_calls);
    if !cache.dirty && state.field_14 == cache.field_14 {
        a80_enabled_zoom_steps(state, cache, &mut out);
        out.steps.push(A80GlueStep::Commit);
        cache.a80_store_state(state);
        out.cache = *cache;
        return out;
    }
    out.steps
        .push(A80GlueStep::Title(a80_glue_title_label(state.field_14)));
    out.cache = *cache;
    out
}

/// `0x006324d1`: enabled/zoom refresh without the title/intro comparisons.
///
/// ```c
/// if (dirty || record.enabled != cache.enabled) report(enabled_label);
/// if (record.zoom != cache.zoom)               report(zoom_label);
/// FUN_00633320(...); cache = record; dirty = 0;
/// ```
pub fn a80_glue_refresh_enabled_unconditional(
    state: &A80GlueState,
    cache: &mut A80GlueCache,
) -> A80GlueRefresh {
    a80_glue_refresh_enabled_unconditional_impl(state, cache, 0)
}

/// `0x006324e6`: [`a80_glue_refresh_enabled_unconditional`] preceded by the
/// virtual call `(*(*this + 0x10))()`.
pub fn a80_glue_refresh_enabled_unconditional_after_begin(
    state: &A80GlueState,
    cache: &mut A80GlueCache,
) -> A80GlueRefresh {
    a80_glue_refresh_enabled_unconditional_impl(state, cache, 1)
}

fn a80_glue_refresh_enabled_unconditional_impl(
    state: &A80GlueState,
    cache: &mut A80GlueCache,
    vtable_calls: u8,
) -> A80GlueRefresh {
    let mut out = A80GlueRefresh::new(cache, vtable_calls);
    if cache.dirty || state.enabled != cache.enabled {
        out.steps
            .push(A80GlueStep::Enabled(a80_enabled_label(state.enabled)));
    }
    if state.zoom != cache.zoom {
        out.steps
            .push(A80GlueStep::Zoom(a80_zoom_label(state.zoom)));
    }
    out.steps.push(A80GlueStep::Commit);
    cache.a80_store_state(state);
    out.cache = *cache;
    out
}

/// `0x006325b0`: finish the zoom edit.
///
/// ```c
/// if (dirty == 0 && record.zoom == cache.zoom) {
///     FUN_00633320(...); cache = record; dirty = 0; return;
/// }
/// report(record.zoom == 4 ? "4x" : "2x");
/// ```
pub fn a80_glue_refresh_zoom(state: &A80GlueState, cache: &mut A80GlueCache) -> A80GlueRefresh {
    a80_glue_refresh_zoom_impl(state, cache, 0)
}

/// `0x006325c5`: [`a80_glue_refresh_zoom`] preceded by the virtual call
/// `(*(*this + 0x10))()`.
pub fn a80_glue_refresh_zoom_after_begin(
    state: &A80GlueState,
    cache: &mut A80GlueCache,
) -> A80GlueRefresh {
    a80_glue_refresh_zoom_impl(state, cache, 1)
}

fn a80_glue_refresh_zoom_impl(
    state: &A80GlueState,
    cache: &mut A80GlueCache,
    vtable_calls: u8,
) -> A80GlueRefresh {
    let mut out = A80GlueRefresh::new(cache, vtable_calls);
    if !cache.dirty && state.zoom == cache.zoom {
        out.steps.push(A80GlueStep::Commit);
        cache.a80_store_state(state);
    } else {
        out.steps
            .push(A80GlueStep::Zoom(a80_zoom_label(state.zoom)));
    }
    out.cache = *cache;
    out
}

/// `0x00632690`: commit the six cached record slots.
///
/// The row first tests the high bit of a stack string's tag word
/// (`if ((tag & 0x80000000) == 0) FUN_0093ec70();`), then calls
/// `FUN_00633320`, copies `+0x0c/+0x10/+0x14/+0x18/+0x1c/+0x24` into the
/// cache and clears the dirty byte.
pub fn a80_glue_commit(state: &A80GlueState, cache: &mut A80GlueCache) -> A80GlueRefresh {
    a80_glue_commit_impl(state, cache, 0)
}

/// `0x006326a5`: [`a80_glue_commit`] preceded by the virtual call
/// `(*(*this + 0x10))()`.
pub fn a80_glue_commit_after_begin(
    state: &A80GlueState,
    cache: &mut A80GlueCache,
) -> A80GlueRefresh {
    a80_glue_commit_impl(state, cache, 1)
}

fn a80_glue_commit_impl(
    state: &A80GlueState,
    cache: &mut A80GlueCache,
    vtable_calls: u8,
) -> A80GlueRefresh {
    let mut out = A80GlueRefresh::new(cache, vtable_calls);
    out.steps.push(A80GlueStep::Commit);
    cache.a80_store_state(state);
    out.cache = *cache;
    out
}

// ---------------------------------------------------------------------------
// 0x00633050 - frame accounting before the runtime call
// ---------------------------------------------------------------------------

/// `0x00633050`: bump the two frame counters and enter `FUN_00c66860`.
///
/// ```c
/// *(int *)(this + 0x48) += 1;
/// *(int *)(this + 0x4c) += param_2;
/// FUN_00c66860();             // no return
/// ```
///
/// Both increments are 32-bit wrapping additions.  `FUN_00c66860` is outside
/// the batch and takes no recovered argument, so it is reported as
/// `terminates` rather than implemented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A80FrameAccounting {
    /// `+0x48` after the increment.
    pub frames: i32,
    /// `+0x4c` after adding `delta`.
    pub bytes: i32,
    /// Always `true`; the row ends in the out-of-batch runtime call.
    pub terminates: bool,
}

/// See [`A80FrameAccounting`].
pub fn a80_frame_accounting(frames: i32, bytes: i32, delta: i32) -> A80FrameAccounting {
    A80FrameAccounting {
        frames: frames.wrapping_add(1),
        bytes: bytes.wrapping_add(delta),
        terminates: true,
    }
}

// ---------------------------------------------------------------------------
// 0x006335b0 - shutdown gate
// ---------------------------------------------------------------------------

/// `0x006335b0`: the deferred-shutdown gate.
///
/// ```c
/// if (DAT_00f05658 != 0 && (FUN_00631bc0(local_14), DAT_00f05658 != 0)
///     && DAT_00f05660 != 0) {
///     DAT_00f05658 = 0;
///     i = FUN_00631e00();
///     if (i != -1) { FUN_00632080(i); FUN_0093fb70(i); }
/// }
/// ```
///
/// Only the gate and the flag clear are in the row; the two out-of-batch
/// calls are not reproduced.  `flag_after_probe` is the second read of
/// `DAT_00f05658` after `FUN_00631bc0` has run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A80ShutdownGate {
    /// The three-way conjunction.
    pub attempt: bool,
    /// The flag is cleared exactly when `attempt` is true.
    pub clear_flag: bool,
}

/// See [`A80ShutdownGate`].
pub fn a80_shutdown_gate(
    flag_before: bool,
    flag_after_probe: bool,
    target: u32,
) -> A80ShutdownGate {
    let attempt = flag_before && flag_after_probe && target != 0;
    A80ShutdownGate {
        attempt,
        clear_flag: attempt,
    }
}

// ---------------------------------------------------------------------------
// 0x00633620 - string capacity growth
// ---------------------------------------------------------------------------

/// `0x00633620`: the `std::string` growth decision.
///
/// ```c
/// raw = *(u32 *)(this + 8);
/// if ((raw & 0x7fffffff) < requested) {
///     capacity = (raw >> 1 & 0x3fffffff) + (raw & 0x7fffffff);
///     if (capacity < requested) capacity = requested;
///     *(u32 *)(this + 8) = (raw ^ capacity) & 0x7fffffff ^ raw;
///     FUN_0093ec70();          // reallocate
/// }
/// ```
///
/// The low 31 bits are the live capacity/flag word and bit 31 is preserved by
/// the store.  The growth target is `max(requested, capacity + capacity / 2)`,
/// with the top two bits masked out of the halved term.  The reallocation
/// call is not reproduced; `None` means the shipped guard did nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A80StringGrow {
    /// The raw word before the update.
    pub previous_raw: u32,
    /// The computed low-31-bit capacity.
    pub capacity: u32,
    /// The raw word stored back at `+0x8`.
    pub raw: u32,
}

/// See [`A80StringGrow`].
pub fn a80_string_grow(current_raw: u32, requested: u32) -> Option<A80StringGrow> {
    let live = current_raw & 0x7fff_ffff;
    if live >= requested {
        return None;
    }
    let mut capacity = (current_raw >> 1 & 0x3fff_ffff) + live;
    if capacity < requested {
        capacity = requested;
    }
    Some(A80StringGrow {
        previous_raw: current_raw,
        capacity,
        raw: (current_raw ^ capacity) & 0x7fff_ffff ^ current_raw,
    })
}

// ---------------------------------------------------------------------------
// 0x006339a0 / 0x006339c0 / 0x00633ac0 / 0x00633b80 - scheduler heap
// ---------------------------------------------------------------------------

/// One `{key, value}` pair of the scheduler heap (`DAT_00faa1d8`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct A80HeapEntry {
    /// Key word at `+0x00`.
    pub key: u32,
    /// Value word at `+0x04`.
    pub value: u32,
}

/// `0x00633b80` read position when the heap has never been drained.
pub const A80_SORTED_NONE: u32 = 0;

/// The globals behind `0x006339a0`, `0x006339c0`, `0x00633ac0` and
/// `0x00633b80`.
///
/// `entries` is indexed from one; index 0 is the unused sentinel.  Drained
/// values are kept in `sorted`, the array the shipped code writes through
/// `DAT_00faa1e4`.  `sorted_next` is `DAT_00faa1f4`, the exclusive upper
/// bound used by `0x00633b80`.
///
/// Entries above `count` are deliberately retained: the shipped sift-down
/// reads `heap[2 * i + 1]` without bounding it by `count`, so stale slots are
/// observable.  Reads beyond the vector model the unused tail of the
/// allocation and return `{0, 0}`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct A80SchedulerHeap {
    entries: Vec<A80HeapEntry>,
    count: u32,
    sorted: Vec<u32>,
    sorted_next: u32,
}

impl A80SchedulerHeap {
    /// An empty heap with only the sentinel slot allocated.
    pub fn a80_new() -> Self {
        A80SchedulerHeap {
            entries: vec![A80HeapEntry::default()],
            ..A80SchedulerHeap::default()
        }
    }

    /// `+0x08` of the header (`DAT_00faa1f0`), i.e. the number of live pairs.
    pub fn a80_len(&self) -> u32 {
        self.count
    }

    fn a80_entry(&self, index: u32) -> A80HeapEntry {
        self.entries
            .get(index as usize)
            .copied()
            .unwrap_or_default()
    }

    fn a80_reserve(&mut self, index: u32) {
        while self.entries.len() <= index as usize {
            self.entries.push(A80HeapEntry::default());
        }
    }

    /// `0x006339a0`: clear the heap count and the `0x100`-byte slot table.
    ///
    /// ```c
    /// DAT_00faa1f0 = 0;
    /// memset(&DAT_00fa2af0, 0, 0x100);
    /// ```
    ///
    /// The one-byte slot table is returned instead of being stored: it is a
    /// separate global and its index computation in `0x00633ac0` depends on
    /// the out-of-batch `FUN_0057ca80`.
    pub fn a80_reset(&mut self) -> [u8; 0x100] {
        self.count = 0;
        [0u8; 0x100]
    }

    /// `0x006339c0`: drain every pair into the sorted array and return the
    /// last value written.
    ///
    /// Each iteration writes the current root value to
    /// `DAT_00faa1e4[index++]`, moves the last pair to the root, and sifts it
    /// down against the larger child (`heap[left].key < heap[right].key`
    /// chooses the right child; the walk stops when the child key is `<=` the
    /// moved key).  The result is a descending-key sequence; the returned
    /// value is its last element, i.e. the minimum-key value.  The shipped
    /// function returns the bare `0` sentinel for an empty heap without
    /// touching the sorted array, which `None` models here.
    pub fn a80_drain(&mut self) -> Option<u32> {
        if self.count == 0 {
            return None;
        }
        let n = self.count;
        self.sorted.clear();
        self.sorted_next = n - 1;
        let mut old_count = n;
        loop {
            self.sorted.push(self.a80_entry(1).value);
            let last = self.a80_entry(old_count);
            self.count = old_count - 1;
            self.a80_reserve(1);
            self.entries[1] = last;
            let key = last.key;
            let value = last.value;
            let half = self.count >> 1;
            let mut index = 1u32;
            if half != 0 {
                loop {
                    let mut child = index.wrapping_mul(2);
                    if child < self.count
                        && self.a80_entry(child).key < self.a80_entry(child + 1).key
                    {
                        child += 1;
                    }
                    if self.a80_entry(child).key <= key {
                        break;
                    }
                    self.a80_reserve(index);
                    self.a80_reserve(child);
                    self.entries[index as usize] = self.entries[child as usize];
                    index = child;
                    if child > half {
                        break;
                    }
                }
            }
            self.a80_reserve(index);
            self.entries[index as usize] = A80HeapEntry { key, value };
            old_count = self.count;
            if old_count == 0 {
                break;
            }
        }
        self.sorted.last().copied()
    }

    /// `0x00633ac0`: insert one pair and sift it up.
    ///
    /// ```c
    /// ++count;
    /// heap[count].value = value;
    /// heap[count].key   = FUN_0057b8b0();   // key source, out of batch
    /// i = count;
    /// if (1 < count) {
    ///     while (key > heap[i >> 1].key) { heap[i] = heap[i >> 1]; i >>= 1; }
    ///     heap[i] = { key, value };
    /// }
    /// ```
    ///
    /// `FUN_0057b8b0` is not in the batch, so the key is an argument.  The
    /// shipped return value is either `*(u16 *)(param_1 + 8)` or a byte in
    /// `DAT_00fa2af0` toggled with `DAT_0119f7c0`; both depend on out-of-batch
    /// helpers and are not reproduced.  The inserted index is returned.
    pub fn a80_push(&mut self, key: u32, value: u32) -> u32 {
        self.count = self.count.wrapping_add(1);
        let index = self.count;
        self.a80_reserve(index);
        self.entries[index as usize] = A80HeapEntry { key, value };
        if index > 1 {
            let mut slot = index;
            loop {
                let parent = slot >> 1;
                self.a80_reserve(parent);
                if key <= self.entries[parent as usize].key {
                    break;
                }
                self.entries[slot as usize] = self.entries[parent as usize];
                slot = parent;
                if parent <= 1 {
                    break;
                }
            }
            self.entries[slot as usize] = A80HeapEntry { key, value };
        }
        index
    }

    /// `0x00633b80`: take the next value out of the sorted array.
    ///
    /// ```c
    /// if (DAT_00faa1f4 == 0) return 0;
    /// --DAT_00faa1f4;
    /// return DAT_00faa1e4[DAT_00faa1f4];
    /// ```
    ///
    /// The shipped sentinel is the bare `0`, so an exhausted heap and a value
    /// of zero are indistinguishable.
    pub fn a80_take_sorted(&mut self) -> u32 {
        if self.sorted_next == A80_SORTED_NONE {
            return 0;
        }
        self.sorted_next -= 1;
        self.sorted
            .get(self.sorted_next as usize)
            .copied()
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// 0x00633ba0 - report marshaller constructor
// ---------------------------------------------------------------------------

/// Vftable identity stored at `+0x00` by `0x00633ba0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A80ReportLogVftable {
    /// `ReportLogMarshaller::vftable`.
    ReportLogMarshaller,
}

/// The two-word object built by `0x00633ba0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A80ReportLogMarshaller {
    /// `+0x00`.
    pub vtable: A80ReportLogVftable,
    /// `+0x04`, the caller's word.
    pub field_4: u32,
}

impl A80ReportLogMarshaller {
    /// `0x00633ba0`: `this[1] = param; this[0] = ReportLogMarshaller::vftable;`.
    pub fn a80_new(field_4: u32) -> Self {
        A80ReportLogMarshaller {
            vtable: A80ReportLogVftable::ReportLogMarshaller,
            field_4,
        }
    }
}

// ---------------------------------------------------------------------------
// 0x00633ca0 - renderer name source
// ---------------------------------------------------------------------------

/// The literal address reported for `DAT_0135e34c == 2`; its bytes are not in
/// the decompilation.
pub const A80_RENDERER_LITERAL_CASE_2: u32 = 0x00d3_c448;

/// Which object `0x00633ca0` hands to `FUN_0053a640`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A80RendererName {
    /// Case `1`: the literal `"OpenGL"`.
    OpenGl,
    /// Case `2`: the literal at `0x00d3c448`, contents unrecovered.
    UnresolvedLiteral(u32),
    /// Cases `3..=6` and the default: the name returned by the active
    /// graphics backend's `+0x6c` virtual method.
    Backend,
}

/// `0x00633ca0`: select the renderer name string.
///
/// ```c
/// switch (DAT_0135e34c) {
/// case 1: report("OpenGL"); break;
/// case 2: report(&UNK_00d3c448); break;
/// case 3: case 4: case 5: case 6:
/// default: report((*(*DAT_0135e348 + 0x6c))()); break;
/// }
/// ```
pub fn a80_renderer_name_source(kind: u32) -> A80RendererName {
    match kind {
        1 => A80RendererName::OpenGl,
        2 => A80RendererName::UnresolvedLiteral(A80_RENDERER_LITERAL_CASE_2),
        3..=6 => A80RendererName::Backend,
        _ => A80RendererName::Backend,
    }
}

// ---------------------------------------------------------------------------
// 0x00633df0 - fixed-size record copy
// ---------------------------------------------------------------------------

/// Bytes of the record copied by `0x00633df0` (`param_2[0x139]` ends at
/// `+0x4e7`).
pub const A80_RECORD_BYTES: usize = 0x4e8;

/// Number of `0x3f`-element slots starting at `+0x18`.
pub const A80_RECORD_SLOTS: usize = 0x3f;

/// Offset of the first `+0x18` slot.
pub const A80_RECORD_SLOT_OFFSET: usize = 0x18;

/// Stride of one slot.
pub const A80_RECORD_SLOT_STRIDE: usize = 8;

/// Bytes copied from the start of each slot; the remaining four are zeroed.
pub const A80_RECORD_SLOT_COPY: usize = 4;

/// The four bytes the shipped copy never touches (`+0x4d4..+0x4d8`), left
/// zero by [`a80_record_clone`].
pub const A80_RECORD_GAP: std::ops::Range<usize> = 0x4d4..0x4d8;

/// `0x00633df0`: copy one `0x4e8`-byte record.
///
/// The body the decompiler emits is mechanical:
///
/// * dwords `0..6` (`+0x00..+0x18`) are copied;
/// * `0x3f` slots of eight bytes follow, each copying four source bytes and
///   zeroing the next four;
/// * `+0x210..+0x4d4` is copied contiguously (the byte/word/dword stores and
///   the 64-word and eight 20-word loops are one range);
/// * `+0x4d8..+0x4e8` (four dwords) is copied;
/// * `+0x4d4..+0x4d8` is neither read nor written.
pub fn a80_record_clone(src: &[u8; A80_RECORD_BYTES]) -> [u8; A80_RECORD_BYTES] {
    let mut dst = [0u8; A80_RECORD_BYTES];
    dst[..A80_RECORD_SLOT_OFFSET].copy_from_slice(&src[..A80_RECORD_SLOT_OFFSET]);
    for slot in 0..A80_RECORD_SLOTS {
        let offset = A80_RECORD_SLOT_OFFSET + slot * A80_RECORD_SLOT_STRIDE;
        dst[offset..offset + A80_RECORD_SLOT_COPY]
            .copy_from_slice(&src[offset..offset + A80_RECORD_SLOT_COPY]);
    }
    dst[0x210..A80_RECORD_GAP.start].copy_from_slice(&src[0x210..A80_RECORD_GAP.start]);
    dst[A80_RECORD_GAP.end..].copy_from_slice(&src[A80_RECORD_GAP.end..]);
    dst
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent80 {
    use super::*;

    // -- 0x00632080 ---------------------------------------------------------

    #[test]
    fn vector_range_uses_the_0x38_stride() {
        let range = a80_vector_range(0x1000, 3);
        assert_eq!(range.end, 0x1000 + 3 * 0x38);
        assert!(range.nonempty);

        let empty = a80_vector_range(0x2000, 0);
        assert_eq!(empty.end, 0x2000);
        assert!(!empty.nonempty);

        // The comparison is on 32-bit addresses.
        let wrap = a80_vector_range(0xffff_ffc8, 1);
        assert_eq!(wrap.end, 0);
        assert!(wrap.nonempty);
    }

    // -- 0x0063225b / 0x006322b1 -------------------------------------------

    #[test]
    fn fixed20_conversion_matches_the_sign_bias() {
        assert_eq!(a80_fixed20_from_i32(0), 0.0);
        assert_eq!(a80_fixed20_from_i32(1 << 20), 1.0);
        assert_eq!(a80_fixed20_from_i32(3 << 20), 3.0);
        // Negative words add 2^32 before the f32 narrowing.
        assert_eq!(a80_fixed20_from_i32(-1), 4096.0);
        assert_eq!(a80_fixed20_from_i32(i32::MIN), 2048.0);
        assert_eq!(a80_fixed20_from_i32(i32::MAX), 2048.0);
    }

    #[test]
    fn field_0x20_widens_exactly() {
        assert_eq!(a80_field_0x20_to_f64(1.5f32.to_bits()), 1.5f64);
        assert_eq!(a80_field_0x20_to_f64((-2.25f32).to_bits()), -2.25f64);
        assert_eq!(a80_field_0x20_to_f64(0x7f80_0000), f64::INFINITY);
    }

    // -- label helpers ------------------------------------------------------

    #[test]
    fn label_bounds_and_literals() {
        assert_eq!(a80_glue_title_label(0), A80Label::TableEntry(0));
        assert_eq!(a80_glue_title_label(0x22), A80Label::TableEntry(0x22));
        assert_eq!(a80_glue_title_label(0x23), A80Label::Unknown);

        assert_eq!(a80_game_intro_label(0xb), A80Label::TableEntry(0xb));
        assert_eq!(a80_game_intro_label(0xc), A80Label::Unknown);

        assert_eq!(a80_enabled_label(true), "Enabled");
        assert_eq!(a80_enabled_label(false), "Disabled");
        assert_eq!(a80_zoom_label(4), "4x");
        assert_eq!(a80_zoom_label(2), "2x");
        assert_eq!(a80_zoom_label(0), "2x");
        assert_eq!(a80_zoom_label(5), "2x");

        assert_eq!(a80_literal_len("Enabled"), 7);
        assert_eq!(a80_literal_len("Disabled"), 8);
        assert_eq!(a80_literal_len("Unknown"), 7);
        assert_eq!(a80_literal_len(""), 0);
    }

    // -- 0x006322e9 ---------------------------------------------------------

    fn state() -> A80GlueState {
        A80GlueState {
            field_c: 0x11,
            field_10: 2,
            field_14: 3,
            enabled: true,
            field_1c: 0x1122_3344_5566_7788,
            zoom: 2,
        }
    }

    fn cache_equal(state: &A80GlueState) -> A80GlueCache {
        let mut cache = A80GlueCache::a80_new();
        cache.a80_store_state(state);
        cache
    }

    #[test]
    fn full_refresh_reports_the_edited_value_when_dirty() {
        let state = state();
        let mut cache = A80GlueCache::a80_new();
        let plan = a80_glue_refresh_all(&state, &mut cache);
        assert_eq!(plan.vtable_calls, 0);
        assert_eq!(plan.steps, vec![A80GlueStep::Value(0x11)]);
        assert!(cache.dirty);
    }

    #[test]
    fn full_refresh_reports_changed_enabled_zoom_then_commits() {
        let state = state();
        let mut cache = cache_equal(&state);
        let mut changed = state;
        changed.enabled = false;
        changed.zoom = 4;

        let plan = a80_glue_refresh_all(&changed, &mut cache);
        assert_eq!(
            plan.steps,
            vec![
                A80GlueStep::Enabled("Disabled"),
                A80GlueStep::Zoom("4x"),
                A80GlueStep::Commit,
            ]
        );
        assert_eq!(plan.cache, cache_equal(&changed));
        assert!(!cache.dirty);
        assert_eq!(cache.field_1c, changed.field_1c);
    }

    #[test]
    fn full_refresh_reports_title_then_intro() {
        let state = state();
        let mut cache = cache_equal(&state);
        let mut changed = state;
        changed.field_14 = 0x22;

        let plan = a80_glue_refresh_all(&changed, &mut cache);
        assert_eq!(
            plan.steps,
            vec![
                A80GlueStep::Title(A80Label::TableEntry(0x22)),
                A80GlueStep::Intro(A80Label::TableEntry(2)),
            ]
        );
        // Title-only changes do not commit in this row.
        assert_eq!(cache.field_14, state.field_14);
    }

    #[test]
    fn full_refresh_reports_intro_when_it_changed() {
        let state = state();
        let mut cache = cache_equal(&state);
        let mut changed = state;
        changed.field_10 = 0xc;

        let plan = a80_glue_refresh_all(&changed, &mut cache);
        assert_eq!(plan.steps, vec![A80GlueStep::Intro(A80Label::Unknown)]);
    }

    // -- 0x00632324 ---------------------------------------------------------

    #[test]
    fn title_refresh_with_a_dirty_cache_reports_intro_only() {
        let state = state();
        let mut cache = A80GlueCache::a80_new();
        let plan = a80_glue_refresh_title(&state, &mut cache);
        assert_eq!(
            plan.steps,
            vec![A80GlueStep::Intro(A80Label::TableEntry(2))]
        );
    }

    #[test]
    fn title_refresh_matches_the_guarded_paths() {
        let state = state();
        let mut cache = cache_equal(&state);

        let mut changed = state;
        changed.field_14 = 0x30;
        let plan = a80_glue_refresh_title(&changed, &mut cache);
        assert_eq!(
            plan.steps,
            vec![
                A80GlueStep::Title(A80Label::Unknown),
                A80GlueStep::Intro(A80Label::TableEntry(2)),
            ]
        );

        let mut changed = state;
        changed.enabled = false;
        changed.zoom = 4;
        let plan = a80_glue_refresh_title(&changed, &mut cache);
        assert_eq!(
            plan.steps,
            vec![
                A80GlueStep::Enabled("Disabled"),
                A80GlueStep::Zoom("4x"),
                A80GlueStep::Commit,
            ]
        );
    }

    // -- 0x006323ec / 0x00632401 -------------------------------------------

    #[test]
    fn enabled_refresh_commits_only_when_the_title_matches() {
        let state = state();
        let mut cache = cache_equal(&state);
        let mut changed = state;
        changed.enabled = false;

        let plan = a80_glue_refresh_enabled(&changed, &mut cache);
        assert_eq!(plan.vtable_calls, 0);
        assert_eq!(
            plan.steps,
            vec![A80GlueStep::Enabled("Disabled"), A80GlueStep::Commit]
        );
        assert!(!cache.dirty);

        let mut cache = cache_equal(&state);
        let mut changed = state;
        changed.field_14 = 0;
        changed.zoom = 4;
        let plan = a80_glue_refresh_enabled_after_begin(&changed, &mut cache);
        assert_eq!(plan.vtable_calls, 1);
        assert_eq!(
            plan.steps,
            vec![A80GlueStep::Title(A80Label::TableEntry(0))]
        );
        assert_eq!(cache.field_14, state.field_14);
    }

    // -- 0x006324d1 / 0x006324e6 -------------------------------------------

    #[test]
    fn unconditional_enabled_refresh_reports_before_committing() {
        let state = state();
        let mut cache = cache_equal(&state);
        // Dirty alone is enough to report enabled, even when it is unchanged.
        cache.dirty = true;
        let plan = a80_glue_refresh_enabled_unconditional(&state, &mut cache);
        assert_eq!(
            plan.steps,
            vec![A80GlueStep::Enabled("Enabled"), A80GlueStep::Commit]
        );
        assert!(!cache.dirty);

        let mut cache = cache_equal(&state);
        let changed = A80GlueState {
            enabled: false,
            zoom: 4,
            ..state
        };
        let plan = a80_glue_refresh_enabled_unconditional_after_begin(&changed, &mut cache);
        assert_eq!(plan.vtable_calls, 1);
        assert_eq!(
            plan.steps,
            vec![
                A80GlueStep::Enabled("Disabled"),
                A80GlueStep::Zoom("4x"),
                A80GlueStep::Commit,
            ]
        );
    }

    // -- 0x006325b0 / 0x006325c5 -------------------------------------------

    #[test]
    fn zoom_refresh_commits_only_for_an_unchanged_zoom() {
        let state = state();
        let mut cache = cache_equal(&state);
        let plan = a80_glue_refresh_zoom(&state, &mut cache);
        assert_eq!(plan.steps, vec![A80GlueStep::Commit]);

        let mut cache = cache_equal(&state);
        let changed = A80GlueState { zoom: 4, ..state };
        let plan = a80_glue_refresh_zoom_after_begin(&changed, &mut cache);
        assert_eq!(plan.vtable_calls, 1);
        assert_eq!(plan.steps, vec![A80GlueStep::Zoom("4x")]);

        let mut cache = cache_equal(&state);
        cache.dirty = true;
        let plan = a80_glue_refresh_zoom(&state, &mut cache);
        assert_eq!(plan.steps, vec![A80GlueStep::Zoom("2x")]);
    }

    // -- 0x00632690 / 0x006326a5 -------------------------------------------

    #[test]
    fn commit_rows_copy_every_cached_slot() {
        let state = A80GlueState {
            field_c: -7,
            field_10: 0xb,
            field_14: 0x21,
            enabled: false,
            field_1c: 0xdead_beef_0123_4567,
            zoom: 4,
        };
        let mut cache = A80GlueCache::a80_new();
        let plan = a80_glue_commit(&state, &mut cache);
        assert_eq!(plan.vtable_calls, 0);
        assert_eq!(plan.steps, vec![A80GlueStep::Commit]);
        assert_eq!(plan.cache, cache_equal(&state));
        assert!(!cache.dirty);

        let mut cache = A80GlueCache::a80_new();
        let plan = a80_glue_commit_after_begin(&state, &mut cache);
        assert_eq!(plan.vtable_calls, 1);
        assert_eq!(plan.steps, vec![A80GlueStep::Commit]);
        assert_eq!(cache.field_c, -7);
        assert_eq!(cache.field_1c, 0xdead_beef_0123_4567);
    }

    // -- 0x00633050 ---------------------------------------------------------

    #[test]
    fn frame_accounting_wraps_both_counters() {
        let out = a80_frame_accounting(4, 0x8000_0000u32 as i32, 0x9000_0000u32 as i32);
        assert_eq!(out.frames, 5);
        assert_eq!(out.bytes, 0x1000_0000u32 as i32);
        assert!(out.terminates);
    }

    // -- 0x006335b0 ---------------------------------------------------------

    #[test]
    fn shutdown_gate_needs_all_three_terms() {
        assert!(a80_shutdown_gate(true, true, 0x1234).attempt);
        assert!(a80_shutdown_gate(true, true, 0x1234).clear_flag);
        assert!(!a80_shutdown_gate(false, true, 0x1234).attempt);
        assert!(!a80_shutdown_gate(true, false, 0x1234).attempt);
        assert!(!a80_shutdown_gate(true, true, 0).attempt);
        assert!(!a80_shutdown_gate(true, true, 0).clear_flag);
    }

    // -- 0x00633620 ---------------------------------------------------------

    #[test]
    fn string_growth_matches_the_raw_word_arithmetic() {
        assert_eq!(a80_string_grow(10, 10), None);
        assert_eq!(a80_string_grow(10, 9), None);

        let grow = a80_string_grow(10, 11).unwrap();
        assert_eq!(grow.capacity, 15);
        assert_eq!(grow.raw, 15);

        // Bit 31 is preserved; only the low 31 bits become the capacity.
        let grow = a80_string_grow(0x8000_000a, 11).unwrap();
        assert_eq!(grow.capacity, 15);
        assert_eq!(grow.raw, 0x8000_000f);

        // A request above the 1.5x target wins.
        let grow = a80_string_grow(0x100, 0x200).unwrap();
        assert_eq!(grow.capacity, 0x200);
        assert_eq!(grow.raw, 0x200);
    }

    // -- 0x006339a0 / 0x006339c0 / 0x00633ac0 / 0x00633b80 -----------------

    #[test]
    fn empty_heap_has_no_drain_and_a_zero_sentinel() {
        let mut heap = A80SchedulerHeap::a80_new();
        assert_eq!(heap.a80_len(), 0);
        assert_eq!(heap.a80_drain(), None);
        assert_eq!(heap.a80_take_sorted(), 0);
    }

    #[test]
    fn heap_push_drain_and_take_sorted_follow_the_shipped_sift() {
        let mut heap = A80SchedulerHeap::a80_new();
        assert_eq!(heap.a80_push(3, 30), 1);
        assert_eq!(heap.a80_push(1, 10), 2);
        assert_eq!(heap.a80_push(4, 40), 3);
        assert_eq!(heap.a80_push(2, 20), 4);
        assert_eq!(heap.a80_push(5, 50), 5);
        assert_eq!(heap.a80_len(), 5);

        // Full drain returns the minimum-key value; the sorted array is
        // descending and the take walks it from the end.
        assert_eq!(heap.a80_drain(), Some(10));
        assert_eq!(heap.a80_len(), 0);
        // The drain returned the smallest value; the take index starts at
        // count-1, so the walk continues with the second-smallest.
        assert_eq!(heap.a80_take_sorted(), 20);
        assert_eq!(heap.a80_take_sorted(), 30);
        assert_eq!(heap.a80_take_sorted(), 40);
        assert_eq!(heap.a80_take_sorted(), 50);
        assert_eq!(heap.a80_take_sorted(), 0);
    }

    #[test]
    fn reset_clears_the_count_and_returns_a_zeroed_slot_table() {
        let mut heap = A80SchedulerHeap::a80_new();
        heap.a80_push(1, 2);
        let slots = heap.a80_reset();
        assert_eq!(heap.a80_len(), 0);
        assert_eq!(slots, [0u8; 0x100]);
    }

    // -- 0x00633ba0 ---------------------------------------------------------

    #[test]
    fn marshaller_stores_the_vftable_then_the_payload() {
        let object = A80ReportLogMarshaller::a80_new(0x55aa);
        assert_eq!(object.vtable, A80ReportLogVftable::ReportLogMarshaller);
        assert_eq!(object.field_4, 0x55aa);
    }

    // -- 0x00633ca0 ---------------------------------------------------------

    #[test]
    fn renderer_name_switch() {
        assert_eq!(a80_renderer_name_source(1), A80RendererName::OpenGl);
        assert_eq!(
            a80_renderer_name_source(2),
            A80RendererName::UnresolvedLiteral(0x00d3_c448)
        );
        for kind in [0, 3, 4, 5, 6, 7] {
            assert_eq!(a80_renderer_name_source(kind), A80RendererName::Backend);
        }
    }

    // -- 0x00633df0 ---------------------------------------------------------

    #[test]
    fn record_copy_zeroes_slot_halves_and_skips_the_gap() {
        let src = [0xa5u8; A80_RECORD_BYTES];
        let dst = a80_record_clone(&src);

        assert_eq!(
            &dst[..A80_RECORD_SLOT_OFFSET],
            &src[..A80_RECORD_SLOT_OFFSET]
        );
        for slot in 0..A80_RECORD_SLOTS {
            let offset = A80_RECORD_SLOT_OFFSET + slot * A80_RECORD_SLOT_STRIDE;
            assert_eq!(&dst[offset..offset + 4], &[0xa5; 4]);
            assert_eq!(&dst[offset + 4..offset + 8], &[0; 4]);
        }
        assert_eq!(&dst[0x210..0x4d4], &src[0x210..0x4d4]);
        assert_eq!(&dst[0x4d4..0x4d8], &[0; 4]);
        assert_eq!(&dst[0x4d8..], &src[0x4d8..]);
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// The 22 rows not translated above, grouped by why:
//
// * `FUN_006323ad` (`FUN_006323ad`): continuation of the inline string edit started by
//   `0x006322e9`; every store is at a caller-frame offset (`EBP-0x2c`,
//   `EBP-0x28`) and the tail is `FUN_00610680` -> `FUN_00610040` ->
//   `FUN_0060ffe0`.  Nothing in the row survives without that frame.
// * `FUN_0063270b` (`FUN_0063270b`): one virtual call plus the `FUN_00c867a5` registration; no
//   recovered state.
// * `FUN_00632780` (`FUN_00632780`): `ExceptionList` setup followed by `FUN_00c86731`; frame
//   plumbing only.
// * `FUN_006327d2` (`FUN_006327d2`): begins mid-instruction (`*in_EAX = *in_EAX + (char)in_EAX`,
//   string-literal byte arithmetic) and continues into a stack string store;
//   the decompiler lost the instruction boundary.
// * `FUN_00632ad0` (`FUN_00632ad0`), `FUN_00632ae5` (`FUN_00632ae5`): catch funclets that clear a local
//   `std::string` (tag `0x8000000f`) and report code `9`; the string object
//   lives in the caller's frame.
// * `FUN_00632e08` (`FUN_00632e08`), `FUN_00632e1d` (`FUN_00632e1d`): the same pair with report code `10`.
// * `FUN_00632eaa` (`FUN_00632eaa`), `FUN_00632ebf` (`FUN_00632ebf`): the same pair with report code `8`.
// * `FUN_00632ff1` (`FUN_00632ff1`): rethrow funclet (`in_ZF`, `FUN_00633320`,
//   `FUN_00c867a5`); no self-contained behaviour.
// * `FUN_00633035` (`FUN_00633035`): restores `ExceptionList` and enters `FUN_00c86b6e`.
// * `FUN_00633320` (`FUN_00633320`): its only visible checks are the `0x82`-dword range and
//   `*param_2 == 0`; the actual scan is the out-of-batch `FUN_00610830`, and
//   the tail is SEH registration.  No arithmetic beyond the already reported
//   `0x82`/zero tests is recoverable.
// * `FUN_006336d0` (`FUN_006336d0`): indirect jump through `DAT_00633908 + param_2 * 4`; the
//   target table is not in the row.
// * `FUN_00633970` (`FUN_00633970`): two calls into out-of-batch allocation helpers.
// * `FUN_00633bc0` (`FUN_00633bc0`): eight-byte tail call to `FUN_00a89650`.
// * `FUN_00633bd0` (`FUN_00633bd0`): varargs staging plus `FUN_00c6c400` and `FUN_00c6de50`
//   with an unrecovered format literal at `0x00d3c19c`.
// * `FUN_00633bf2` (`FUN_00633bf2`): empty infinite loop (`do { } while (true)`).
// * `FUN_00633c60` (`FUN_00633c60`): external `FUN_00c6de50` with the literal at
//   `0x00d384e4`.
// * `FUN_00633c80` (`FUN_00633c80`): virtual call through `*in_ECX`; no arguments or state.
// * `FUN_00633c9d` (`FUN_00633c9d`): `int3`; no behaviour.
// * `FUN_00633d52` (`FUN_00633d52`): mid-instruction string-byte arithmetic followed by the
//   SEH dispatcher; no recoverable behaviour.
