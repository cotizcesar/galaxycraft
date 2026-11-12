#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent75.rs
// ===========================================================================
// Translation batch 75.  Only this file is edited by the batch.
//
// Batch 75 owns `0x00628000..=0x00629fff`.  The ledger lists 53 rows in that
// window: 52 `decompiled_uncited` and one 2-byte thunk (`FUN_00628c18`).  31 of
// the 52 rows are translated below; the other 21 are listed at the end of the
// file with their reason.
//
// The window is a screen-fade controller and the object plumbing around it.
// The controller is a single block of globals; every writer in the window
// touches one of them, and the recoverable bodies give the following roles:
//
// | address      | role in the recovered bodies                                |
// |--------------|-------------------------------------------------------------|
// | `0x00f05030` | `float` intensity, clamped to `[0, 1]` by every writer      |
// | `0x00f050dc` | `u32` current mode word                                    |
// | `0x00f050e0` | `0x1c`-byte fade record handed to `FUN_0067a1d0`/`FUN_0067b300` |
// | `0x00f050f0` | handle word tested by `FUN_0067acd0`                          |
// | `0x00fa94c8` | table address returned by `0x006289e0`                     |
// | `0x00faa0f4` | init guard word compared with the TLS epoch                |
// | `0x00faa178` | byte that gates the `0.75` multiplier                      |
// | `0x00faa179` | byte "active"                                              |
// | `0x00faa17a` | byte returned by `0x00628ce0`                              |
// | `0x00faa17c` | `float` fade ramp, stepped by `0.05`                        |
//
// The controller calls five helpers outside this batch: `FUN_0067a1d0`
// (build the fade record), `FUN_0067b300` (start it), `FUN_0067bb30` (stop every
// animation), `FUN_0067bd50(1, x)` (apply the overlay factor) and
// `FUN_0067acd0` (test a handle).  None of them is translated here, so the
// values they would supply are parameters (`playing`, `starts`) and the value
// they would receive is recorded on the model (`overlay_alpha`).  No `sim`
// record gains a field.
//
// Two conventions are used by the call records below:
//
// * a [`A75Call`] records a shipped call by its target address and the
//   literal arguments the row spells out; arguments that come from a frame
//   pointer or from an out-of-batch return value are not recovered and are
//   left out (`literal_count` says how many are present);
// * "direct import call" means the shipped `(*(code *)0x02xxxxxx)(...)` form,
//   i.e. a call through the pointer stored at that address, not a call to the
//   address itself.
//
// Address to function:
//
// * `0x006287f0` [`a75_option_value_dispatch`]
// * `0x00628830` [`a75_function_object_vtable`]
// * `0x00628840` [`A75ScreenFade::a75_reset_mode_19`]
// * `0x00628870` [`a75_reserve_record`] / [`a75_grow_capacity_word`]
// * `0x006289a8` [`a75_default_allocator_slot`]
// * `0x006289e0` [`a75_animation_table`]
// * `0x00628b90` [`a75_error_record`]
// * `0x00628c25` [`a75_catch_record`]
// * `0x00628cd0` [`a75_lambda_type_descriptor`]
// * `0x00628ce0` [`a75_pending_flag`]
// * `0x006291c7` [`A75ScreenFade::a75_on_mode_changed`]
// * `0x00629378` [`A75ScreenFade::a75_stop_handle_mode_1b`]
// * `0x006293d6` [`A75ScreenFade::a75_stop_handle_mode_1`]
// * `0x006293eb` [`A75ScreenFade::a75_vcall_stop_handle_mode_1`]
// * `0x00629450` [`A75ScreenFade::a75_apply_mode`]
// * `0x00629571` [`A75ScreenFade::a75_tick`]
// * `0x00629670` [`A75ScreenFade::a75_reset_and_apply`]
// * `0x00629800` [`a75_is_active`]
// * `0x00629880` [`A75ScreenFade::a75_apply_current_mode`]
// * `0x006298a5` [`A75ScreenFade::a75_apply_current_mode_alt`]
// * `0x00629980` [`A75ScreenFade::a75_set_intensity`]
// * `0x006299e0` [`A75ScreenFade::a75_stop`]
// * `0x006299f0` [`A75ScreenFade::a75_begin`]
// * `0x00629ab0` [`A75ScreenFade::a75_set_mode`] / [`a75_mode_probe`]
// * `0x00629b86` [`A75ScreenFade::a75_set_mode_continuation`]
// * `0x00629bf0` [`A75ScreenFade::a75_set_mode_after_vcall`]
// * `0x00629d40` [`A75ScreenFade::a75_stop_and_clear_active`]
// * `0x00629d60` [`A75ScreenFade::a75_clear_reduced`]
// * `0x00629de0` [`a75_state_holder_init`]
// * `0x00629e00` [`a75_diagnostic_log_request`]
// * `0x00629f90` [`a75_sort_plan`] / [`a75_median_of_three`] /
//   [`a75_choose_pivot`]

// ---------------------------------------------------------------------------
// Controller globals and the modelled state word block
// ---------------------------------------------------------------------------

/// `0x00f05030`: `float` intensity.
pub const A75_GLOBAL_INTENSITY: u32 = 0x00f0_5030;

/// `0x00f050dc`: current mode word.
pub const A75_GLOBAL_MODE: u32 = 0x00f0_50dc;

/// `0x00f050e0`: fade record consumed by `FUN_0067a1d0`/`FUN_0067b300`.
pub const A75_GLOBAL_FADE_RECORD: u32 = 0x00f0_50e0;

/// `0x00f050f0`: handle word tested by `FUN_0067acd0`.
pub const A75_GLOBAL_ANIMATION_HANDLE: u32 = 0x00f0_50f0;

/// `0x00fa94c8`: table address returned by `0x006289e0`.
pub const A75_ANIMATION_TABLE: u32 = 0x00fa_94c8;

/// `0x00faa0f4`: init guard word compared with the TLS epoch.
pub const A75_ANIMATION_TABLE_GUARD: u32 = 0x00fa_a0f4;

/// `0x00faa178`: byte that gates the `0.75` multiplier.
pub const A75_GLOBAL_REDUCED: u32 = 0x00fa_a178;

/// `0x00faa179`: byte "active".
pub const A75_GLOBAL_ACTIVE: u32 = 0x00fa_a179;

/// `0x00faa17a`: byte returned by `0x00628ce0`.
pub const A75_GLOBAL_PENDING: u32 = 0x00fa_a17a;

/// `0x00faa17c`: `float` fade ramp.
pub const A75_GLOBAL_RAMP: u32 = 0x00fa_a17c;

/// The `0.05` the ramp advances by in `0x00629571`.
pub const A75_RAMP_STEP: f32 = 0.05;

/// The factor applied while `0x00faa178` is set.
pub const A75_REDUCED_FACTOR: f32 = 0.75;

/// `0x00f063cc ^ 0x10fae560` plus this offset supplies the level word that
/// `0x006291c7` range-checks against `0x1f`.
pub const A75_LEVEL_TABLE_OFFSET: u32 = 0x154;

/// Highest level `0x006291c7` accepts from that table word.
pub const A75_LEVEL_MAX: u16 = 0x1f;

/// The address the level word is read from.
///
/// The row has `*(ushort *)((0x00f063cc ^ 0x10fae560) + 0x154)`; the xor is
/// the image-base obfuscation, so the base is a runtime value.
pub fn a75_level_word_address(obfuscated_base: u32) -> u32 {
    obfuscated_base.wrapping_add(A75_LEVEL_TABLE_OFFSET)
}

/// The three modes `0x006291c7` is allowed to adopt directly.
pub const A75_ADOPTABLE_MODES: [i32; 3] = [4, 8, 0x0c];

/// `0x006291c7` compares the descriptor dword at `+0x1c` with this value.
pub const A75_DESCRIPTOR_FADING: i32 = 2;

// ---------------------------------------------------------------------------
// Call and import records
// ---------------------------------------------------------------------------

/// `0x025505bf`: direct import call with the layer index.
pub const A75_IMPORT_SET_LAYER: u32 = 0x0255_05bf;

/// `0x02550c8c`: direct import call with `(3, 0x14, callback)`.
pub const A75_IMPORT_REGISTER_CALLBACK: u32 = 0x0255_0c8c;

/// `0x0249094d`: direct import call with no arguments.
pub const A75_IMPORT_CLEAR_FLAG: u32 = 0x0249_094d;

/// `0x02490440`: direct import call that receives a frame buffer and returns
/// a dword.
pub const A75_IMPORT_QUERY: u32 = 0x0249_0440;

/// `0x02490ed5`: the direct import call made by `FUN_0093f6c0`.
pub const A75_IMPORT_TIMER: u32 = 0x0249_0ed5;

/// `0x00629550`: the callback address registered by `0x00629450`.
pub const A75_CALLBACK_00629550: u32 = 0x0062_9550;

/// `FUN_00627750`: applies a built mode descriptor (`0x006291c7`).
pub const A75_APPLY_DESCRIPTOR: u32 = 0x0062_7750;

/// `FUN_0053a930`: flushes the applied descriptor (`0x006291c7`).
pub const A75_FLUSH_DESCRIPTOR: u32 = 0x0053_a930;

/// `FUN_00538810`: capacity-growth entry, forwarded to by `0x00628870`.
pub const A75_GROW_CAPACITY: u32 = 0x0053_8810;

/// `FUN_00713200`: handle release called with `(handle, -1, 0)`.
pub const A75_RELEASE_HANDLE: u32 = 0x0071_3200;

/// One shipped call: the target address plus the literal arguments the row
/// spells out.
///
/// Frame-relative and return-value arguments are not recovered and are left
/// out; `literal_count` is the number of valid entries in `literals`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A75Call {
    /// Callee address (an import-table address for the `0x02xxxxxx` form).
    pub target: u32,
    /// Up to three literal arguments, in argument order.
    pub literals: [u32; 3],
    /// How many entries of `literals` are present.
    pub literal_count: u8,
}

impl A75Call {
    /// Record a call with the given literal arguments.
    pub fn new(target: u32, literals: &[u32]) -> Self {
        let mut out = [0u32; 3];
        for (slot, value) in out.iter_mut().zip(literals.iter()) {
            *slot = *value;
        }
        A75Call {
            target,
            literals: out,
            literal_count: literals.len().min(3) as u8,
        }
    }

    /// The literal arguments actually present.
    pub fn args(&self) -> &[u32] {
        &self.literals[..self.literal_count as usize]
    }
}

// ---------------------------------------------------------------------------
// The fade-record shape: 0x006299f0
// ---------------------------------------------------------------------------

/// The four recovered words of the `0x00f050e0` fade record.
///
/// `FUN_0067a1d0(&0x00f050e0, id, 1, duration)` supplies `id`, `layer` (`1` in
/// every call in the window) and `duration`; `FUN_0067b300(&0x00f050e0, 1.0f, 0)`
/// supplies `target = 1.0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A75FadeRecord {
    /// Dword taken from the row's `*param_1` by `0x006299f0`.
    pub id: u32,
    /// Layer literal: `1`.
    pub layer: u32,
    /// Duration argument of `0x006299f0`.
    pub duration: f32,
    /// Target reached by the fade: `1.0`.
    pub target: f32,
}

/// A fade request: the record plus the out-of-batch start result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A75FadeRequest {
    /// The record handed to `FUN_0067a1d0`.
    pub record: A75FadeRecord,
    /// What `FUN_0067b300` returned; this is stored straight into `0x00faa179`.
    ///
    /// The shipped helper returns `1` only when the animation slot could be
    /// created, which needs the table at `0x00fa94c8` to be live and the slot
    /// handle to match; that check is outside this batch, so it is an input.
    pub starts: bool,
}

impl A75FadeRequest {
    /// A request with the window's `layer = 1`, `target = 1.0` literals.
    pub fn new(id: u32, duration: f32, starts: bool) -> Self {
        A75FadeRequest {
            record: A75FadeRecord {
                id,
                layer: 1,
                duration,
                target: 1.0,
            },
            starts,
        }
    }
}

/// Outcome of `0x006299f0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A75Begin {
    /// The record the row handed to `FUN_0067a1d0`.
    pub record: A75FadeRecord,
    /// `0x00faa179` after the row.
    pub active: bool,
    /// True when `FUN_0067b300` failed and the row stopped everything again.
    pub stopped_after_failure: bool,
}

// ---------------------------------------------------------------------------
// The controller state
// ---------------------------------------------------------------------------

/// The seven words the window's controller keeps between calls.
///
/// Six fields are the globals of the module table; `overlay_alpha` records the
/// value the shipped rows hand to `FUN_0067bd50(1, x)`, which is the only
/// observable effect of the overlay helper.  The struct is a value type and
/// the methods are the rows.  `a75_new` is the model's neutral
/// state: mode `0` is what `0x00629ab0`'s `param_1 != 0` guard reads as "off",
/// and `1.0` is the value every recovered clamp falls back to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A75ScreenFade {
    /// `0x00f050dc`.
    pub mode: u32,
    /// `0x00f05030`.
    pub intensity: f32,
    /// `0x00faa17c`.
    pub ramp: f32,
    /// `0x00faa179`.
    pub active: bool,
    /// `0x00faa17a`.
    pub pending: bool,
    /// `0x00faa178`.
    pub reduced: bool,
    /// The value handed to `FUN_0067bd50(1, x)` in the shipped bodies.
    pub overlay_alpha: f32,
}

/// `0xffffffff`: the "uninitialised" value of the static-init guard.
pub const A75_INIT_UNSET: i32 = -1;

/// `0x00628840`: `FUN_0067bb30(); 0x00faa179 = 0; 0x00faa17a = 0;
/// 0x00629ab0(0x19)`.
///
/// The mode is set through the same tail as [`A75ScreenFade::a75_set_mode`],
/// so the probe/record inputs are passed through.  `FUN_0067bb30` is entered
/// twice on this path (once here, once inside the set-mode tail); only the
/// first one is observable in this model, because it carries no argument.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A75ResetOutcome {
    /// What `FUN_0067b300` returned when the mode tail started a fade.
    pub began: bool,
    /// `0x00faa179` after the row.
    pub active: bool,
}

impl A75ScreenFade {
    /// The neutral state described on the struct.
    pub fn a75_new() -> Self {
        A75ScreenFade {
            mode: 0,
            intensity: 1.0,
            ramp: 1.0,
            active: false,
            pending: false,
            reduced: false,
            overlay_alpha: 1.0,
        }
    }

    /// The multiplier the shipped rows read out of `0x00faa178`:
    /// `intensity * 0.75` when the byte is set, otherwise `intensity`.
    pub fn a75_effective_intensity(&self) -> f32 {
        if self.reduced {
            self.intensity * A75_REDUCED_FACTOR
        } else {
            self.intensity
        }
    }

    /// `0x006299e0`: `FUN_0067bb30();` — stop every animation.
    ///
    /// The helper is outside this batch and holds no argument; the row itself
    /// changes no word of the state block, so it is a no-op here.
    pub fn a75_stop(&mut self) {}

    /// `0x00629670`: reset the ramp, stop, damp and re-apply.
    ///
    /// ```text
    /// if (FUN_0067acd0(0x00f050f0)) {          // handle still playing
    ///     0x00faa179 = 0;
    ///     0x00faa17c = 1.0;
    ///     FUN_0093f6c0();                      // *(0x02490ed5)()
    /// }
    /// FUN_0067bb30();
    /// 0x00faa179 = 0;
    /// if (0x00faa178) 0x00f05030 *= 0.75;    // permanent damping
    /// clamp 0x00f05030 into [0, 1] with the 1.0 default;
    /// FUN_006289e0();
    /// FUN_0067bd50(1, 0x00f05030 * (0x00faa178 ? 0.75 : 1));
    /// ```
    ///
    /// Note the damping writes back to the stored intensity, so repeated calls
    /// with `0x00faa178` set multiply again; that is what the row does.
    pub fn a75_reset_and_apply(&mut self, playing: bool) -> A75Reset {
        let mut timer_call = false;
        if playing {
            self.active = false;
            self.ramp = 1.0;
            timer_call = true;
        }
        self.a75_stop();
        self.active = false;
        if self.reduced {
            self.intensity *= A75_REDUCED_FACTOR;
        }
        self.intensity = a75_clamp_unit_ceiling(self.intensity);
        let overlay = self.a75_effective_intensity();
        self.overlay_alpha = overlay;
        A75Reset {
            playing,
            timer_call,
            intensity: self.intensity,
            overlay_alpha: overlay,
        }
    }

    /// `0x00629980`: replace the intensity from an argument and apply it.
    ///
    /// ```text
    /// 0x00f05030 = (param <= 1.0) ? param : 1.0;
    /// if (0x00f05030 <= 0.0) 0x00f05030 = 0.0;
    /// FUN_006289e0();
    /// FUN_0067bd50(1, 0x00f05030 * (0x00faa178 ? 0.75 : 1));
    /// ```
    pub fn a75_set_intensity(&mut self, value: f32) -> f32 {
        self.intensity = a75_clamp_unit_ceiling(value);
        let overlay = self.a75_effective_intensity();
        self.overlay_alpha = overlay;
        overlay
    }

    /// `0x006299f0`: clear the reduced byte, clamp, apply and start the fade.
    ///
    /// The clamp reuses the stored intensity (the row has no float argument),
    /// then `0x00faa178` is cleared *before* the overlay value is derived, so
    /// the `0.75` factor cannot apply on this path.
    pub fn a75_begin(&mut self, request: A75FadeRequest) -> A75Begin {
        self.intensity = a75_clamp_unit_ceiling(self.intensity);
        self.reduced = false;
        self.overlay_alpha = self.a75_effective_intensity();
        self.active = request.starts;
        let mut stopped_after_failure = false;
        if !self.active {
            self.a75_stop();
            self.active = false;
            stopped_after_failure = true;
        }
        A75Begin {
            record: request.record,
            active: self.active,
            stopped_after_failure,
        }
    }

    /// `0x00629d60`: leave the reduced state, clamping and re-applying.
    ///
    /// The whole body is under `if (0x00faa178 != 0)`.  Unlike
    /// [`Self::a75_reset_and_apply`] there is no `* 0.75` store here: the byte
    /// is cleared first, so the clamp and the applied value are the plain
    /// intensity.
    pub fn a75_clear_reduced(&mut self) -> bool {
        if !self.reduced {
            return false;
        }
        let clamped = a75_clamp_unit_ceiling(self.intensity);
        self.reduced = false;
        self.intensity = clamped;
        self.overlay_alpha = self.a75_effective_intensity();
        true
    }

    /// `0x00629d40`: `FUN_0067bb30(); 0x00faa179 = 0;`.
    pub fn a75_stop_and_clear_active(&mut self) {
        self.a75_stop();
        self.active = false;
    }

    /// `0x00628840`: stop, clear both bytes and set mode `0x19`.
    pub fn a75_reset_mode_19(
        &mut self,
        table_live: bool,
        probe: &A75ModeProbe,
        request: A75FadeRequest,
    ) -> A75SetMode {
        self.active = false;
        self.pending = false;
        self.a75_set_mode(0x19, table_live, probe, request)
    }
}

/// Outcome of `0x00629670`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A75Reset {
    /// The `FUN_0067acd0` argument.
    pub playing: bool,
    /// True when the `0x02490ed5` import was called.
    pub timer_call: bool,
    /// `0x00f05030` after the damping and clamp.
    pub intensity: f32,
    /// The value handed to `FUN_0067bd50(1, x)`.
    pub overlay_alpha: f32,
}

/// The clamp every writer of `0x00f05030` performs.
///
/// The order matters: the source rows all read
///
/// ```text
/// v = 1.0;
/// if (x <= 1.0) v = x;
/// if (v <= 0.0) v = 0.0;
/// ```
///
/// so `NaN` becomes `1.0` (both comparisons are false, leaving the default),
/// `-0.0` becomes `+0.0`, and `+inf` becomes `1.0`.
fn a75_clamp_unit_ceiling(value: f32) -> f32 {
    let mut out = 1.0f32;
    if value <= 1.0 {
        out = value;
    }
    if out <= 0.0 {
        out = 0.0;
    }
    out
}

// ---------------------------------------------------------------------------
// 0x00629571: the ramp tick
// ---------------------------------------------------------------------------

/// Outcome of `0x00629571`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A75Tick {
    /// The value handed to `FUN_0067bd50(1, x)` on the first apply.
    pub alpha: f32,
    /// The `in_XMM0_Da <= 0x00faa17c` branch was taken.
    pub completed: bool,
    /// The second argument of the `0x025505bf` import, when it ran.
    pub callback_arg: Option<u32>,
}

impl A75ScreenFade {
    /// `0x00629571`: advance the ramp and re-apply the overlay.
    ///
    /// ```text
    /// 0x00faa17c += 0.05;
    /// scale = 0x00faa17c;
    /// if (target <= 0x00faa17c) {
    ///     0x00faa17c = 1.0;
    ///     completed = 1;
    ///     scale = target;
    /// }
    /// factor = 0x00f05030 * (0x00faa178 ? 0.75 : 1);
    /// alpha = target;
    /// if (scale * factor <= target) alpha = scale * factor;
    /// if (alpha <= 0.0) alpha = 0.0;
    /// FUN_006289e0();
    /// FUN_0067bd50(1, alpha);
    /// if (!completed) return;
    /// (*(0x025505bf))(FUN_007b2020(), param_3);
    /// if (0x00faa178) 0x00f05030 *= 0.75;
    /// clamp 0x00f05030 with the 1.0 default;
    /// FUN_006289e0();
    /// FUN_0067bd50(1, 0x00f05030 * (0x00faa178 ? 0.75 : 1));
    /// ```
    ///
    /// `callback_arg` is the dword the row reads from its caller's stack
    /// (`[ebp + 0x10]`); the handle from `FUN_007b2020` is not recovered.
    pub fn a75_tick(&mut self, target: f32, callback_arg: u32) -> A75Tick {
        self.ramp += A75_RAMP_STEP;
        let mut completed = false;
        let mut scale = self.ramp;
        if target <= self.ramp {
            self.ramp = 1.0;
            completed = true;
            scale = target;
        }
        let factor = self.a75_effective_intensity();
        let mut alpha = target;
        if scale * factor <= target {
            alpha = scale * factor;
        }
        if alpha <= 0.0 {
            alpha = 0.0;
        }
        self.overlay_alpha = alpha;
        let mut callback_arg_out = None;
        if completed {
            callback_arg_out = Some(callback_arg);
            if self.reduced {
                self.intensity *= A75_REDUCED_FACTOR;
            }
            self.intensity = a75_clamp_unit_ceiling(self.intensity);
            self.overlay_alpha = self.a75_effective_intensity();
        }
        A75Tick {
            alpha,
            completed,
            callback_arg: callback_arg_out,
        }
    }
}

// ---------------------------------------------------------------------------
// 0x00629ab0: the mode probe and the set-mode tail
// ---------------------------------------------------------------------------

/// Inputs of the pre-switch probe inside `0x00629ab0`.
///
/// The shipped block first builds a descriptor with `FUN_00628a80()` and then
/// queries five helpers, all outside this batch.  The descriptor supplies the
/// `selector` (`+0x08`) and the `cleanup` flag (`+0x04`); the helpers supply
/// the rest.  Their recovered use is:
///
/// ```text
/// if (FUN_0092ac60()) {                       // session
///     if (FUN_0092a2e0()) {                   // display
///         hit = selector == 0 ? FUN_0092a380() : FUN_0092a130();
///         if (hit) {
///             p = FUN_00c8a430();
///             if (p) p[1] = 0;
///             (*(0x0249094d))();
///             result = (*(0x02490440))(frame_buffer);
///         }
///     }
///     if (descriptor[+0x04]) FUN_009298b0();
/// }
/// if (result == 4 && FUN_00709680()) FUN_00538810(0x14);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A75ModeProbe {
    /// `FUN_0092ac60() != 0`.
    pub session: bool,
    /// `FUN_0092a2e0() != 0`.
    pub display: bool,
    /// Descriptor `+0x08`.
    pub selector: u32,
    /// `FUN_0092a380() != 0`, read when `selector == 0`.
    pub primary: bool,
    /// `FUN_0092a130() != 0`, read when `selector != 0`.
    pub secondary: bool,
    /// Descriptor `+0x04`; drives the `FUN_009298b0()` cleanup call.
    pub cleanup: bool,
    /// `FUN_00c8a430()` pointer; its byte at `+1` is cleared when present.
    pub clear_object: Option<u32>,
    /// Return of the `0x02490440` import.
    pub import_result: u32,
    /// `FUN_00709680() != 0`.
    pub gated: bool,
}

/// Outcome of [`a75_mode_probe`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A75ProbeOutcome {
    /// The two import calls, in shipped order.
    pub calls: Vec<A75Call>,
    /// Address of the byte cleared through the `FUN_00c8a430` pointer.
    pub cleared_byte_at: Option<u32>,
    /// Return of the `0x02490440` import, when it ran.
    pub query_result: Option<u32>,
    /// `FUN_009298b0()` was called.
    pub cleanup: bool,
    /// `FUN_00538810(0x14)` was requested.
    pub record_request: Option<u32>,
}

/// `0x00629ab0`'s probe block, extracted so the branch order is testable.
pub fn a75_mode_probe(input: &A75ModeProbe) -> A75ProbeOutcome {
    let mut out = A75ProbeOutcome::default();
    if !input.session {
        return out;
    }
    if input.display {
        let hit = if input.selector == 0 {
            input.primary
        } else {
            input.secondary
        };
        if hit {
            out.cleared_byte_at = input.clear_object.map(|p| p.wrapping_add(1));
            out.calls.push(A75Call::new(A75_IMPORT_CLEAR_FLAG, &[]));
            out.calls.push(A75Call::new(A75_IMPORT_QUERY, &[]));
            out.query_result = Some(input.import_result);
        }
    }
    if input.cleanup {
        out.cleanup = true;
    }
    if out.query_result == Some(4) && input.gated {
        out.record_request = Some(0x14);
    }
    out
}

/// The unconditional part of the `0x00629ab0` probe.
///
/// `0x00629b86` re-enters the probe after its `session`/`display` gates have
/// already passed, so its head is just
///
/// ```text
/// p = FUN_00c8a430(); if (p) p[1] = 0;
/// (*(0x0249094d))();
/// result = (*(0x02490440))(frame_buffer);
/// if (descriptor[+0x04]) FUN_009298b0();
/// if (result == 4 && FUN_00709680()) FUN_00538810(0x14);
/// ```
///
/// with no gating on `session` or `display`.
pub fn a75_mode_probe_tail(input: &A75ModeProbe) -> A75ProbeOutcome {
    let mut out = A75ProbeOutcome {
        cleared_byte_at: input.clear_object.map(|p| p.wrapping_add(1)),
        query_result: Some(input.import_result),
        cleanup: input.cleanup,
        ..A75ProbeOutcome::default()
    };
    out.calls.push(A75Call::new(A75_IMPORT_CLEAR_FLAG, &[]));
    out.calls.push(A75Call::new(A75_IMPORT_QUERY, &[]));
    if input.import_result == 4 && input.gated {
        out.record_request = Some(0x14);
    }
    out
}

/// Outcome of the set-mode tail.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct A75SetMode {
    /// The `param_1 == 0x00f050dc` guard skipped the whole body.
    pub skipped: bool,
    /// The probe block's outcome (empty when `param_1 == 0`).
    pub probe: A75ProbeOutcome,
    /// `FUN_0067bb30()` ran.
    pub stopped: bool,
    /// The begin tail ran and `FUN_0067b300` succeeded.
    pub began: bool,
    /// `0x00faa179` after the row.
    pub active_after: bool,
    /// The record handed to `FUN_0067a1d0`, when the begin tail ran.
    pub record: Option<A75FadeRecord>,
}

impl A75ScreenFade {
    /// `0x00629ab0`: switch the mode.
    ///
    /// ```text
    /// if (FUN_0067acc0() && param_1 == 0x00f050dc) return;   // table live + same
    /// if (param_1 != 0) {
    ///     FUN_00628a80();          // descriptor for param_1
    ///     <the probe block>
    /// }
    /// 0x00f050dc = param_1;
    /// FUN_0067bb30();
    /// 0x00faa179 = 0;
    /// if (param_1 != 0) {
    ///     FUN_00628a80();
    ///     FUN_006299f0();
    /// }
    /// ```
    ///
    /// `table_live` is `FUN_0067acc0()` (the animation table's `+0x08` word is
    /// non-zero); it is an input for the same reason as
    /// [`A75FadeRequest::starts`].
    pub fn a75_set_mode(
        &mut self,
        mode: u32,
        table_live: bool,
        probe: &A75ModeProbe,
        request: A75FadeRequest,
    ) -> A75SetMode {
        let mut out = A75SetMode::default();
        if table_live && mode == self.mode {
            out.skipped = true;
            out.active_after = self.active;
            return out;
        }
        if mode != 0 {
            out.probe = a75_mode_probe(probe);
        }
        self.mode = mode;
        self.a75_stop();
        out.stopped = true;
        self.active = false;
        if mode != 0 {
            let began = self.a75_begin(request);
            out.began = began.active;
            out.record = Some(began.record);
        }
        out.active_after = self.active;
        out
    }
}

/// A mode adoption plus the set-mode call that follows it (`0x006291c7`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct A75ModeChange {
    /// `0x00faa179` was clear, so the row returned immediately.
    pub inactive: bool,
    /// The descriptor dword at `+0x1c` was not `2`.
    pub kind_mismatch: bool,
    /// `FUN_0067acd0(0x00f050f0)` reported a live handle.
    pub played: bool,
    /// The mode was adopted from the candidate before the tail ran.
    pub adopted: bool,
    /// Calls made by the adopt branch (`FUN_00627750`, `FUN_0053a930`).
    pub adopt_calls: Vec<A75Call>,
    /// The set-mode tail that always closes the row (`0x00629ab0`).
    pub set_mode: A75SetMode,
    /// The tail was entered at all.
    pub set_mode_called: bool,
}

impl A75ScreenFade {
    /// `0x006291c7`: react to a mode-change notification.
    ///
    /// ```text
    /// if (!0x00faa179) return;
    /// descriptor = FUN_00628a80(0x00f050dc);
    /// if (descriptor[+0x1c] != 2) return;
    /// if (FUN_0067acd0(0x00f050f0)) return;
    /// level = *(u16 *)((0x00f063cc ^ 0x10fae560) + 0x154);
    /// candidate = candidate_word;
    /// if (level != 0 && level <= 0x1f && (candidate == 4 || 8 || 0xc)) {
    ///     0x00f050dc = candidate;
    ///     FUN_00627750(FUN_00628a80(candidate));
    ///     FUN_0053a930();
    /// }
    /// FUN_00629ab0(candidate);
    /// ```
    ///
    /// Because the adopt branch stores the candidate in `0x00f050dc` first,
    /// the closing `0x00629ab0(candidate)` is normally skipped by its own
    /// `param_1 == 0x00f050dc` guard — see the `adopted_and_skipped` test.
    /// The level word lives behind the obfuscated pointer, so it is a
    /// parameter; only its `1..=0x1f` range check is recovered here.
    pub fn a75_on_mode_changed(
        &mut self,
        descriptor_kind: i32,
        level: u16,
        candidate: i32,
        playing: bool,
        table_live: bool,
        probe: &A75ModeProbe,
        request: A75FadeRequest,
    ) -> A75ModeChange {
        let mut out = A75ModeChange::default();
        if !self.active {
            out.inactive = true;
            return out;
        }
        if descriptor_kind != A75_DESCRIPTOR_FADING {
            out.kind_mismatch = true;
            return out;
        }
        if playing {
            out.played = true;
            return out;
        }
        if level != 0 && level <= A75_LEVEL_MAX && A75_ADOPTABLE_MODES.contains(&candidate) {
            self.mode = candidate as u32;
            out.adopted = true;
            out.adopt_calls
                .push(A75Call::new(A75_APPLY_DESCRIPTOR, &[]));
            out.adopt_calls
                .push(A75Call::new(A75_FLUSH_DESCRIPTOR, &[]));
        }
        out.set_mode = self.a75_set_mode(candidate as u32, table_live, probe, request);
        out.set_mode_called = true;
        out
    }
}

// ---------------------------------------------------------------------------
// 0x00629450 / 0x00629880 / 0x006298a5: the apply-mode wrappers
// ---------------------------------------------------------------------------

/// Outcome of the `0x00629450` family.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct A75ModeApply {
    /// `FUN_0067acd0(0x00f050f0)` was live, so the row did nothing.
    pub skipped: bool,
    /// `0x00faa17c` was zeroed before the tail.
    pub ramp_zeroed: bool,
    /// The set-mode tail that ran.
    pub set_mode: Option<A75SetMode>,
    /// Calls made after the tail.
    pub calls: Vec<A75Call>,
}

impl A75ScreenFade {
    fn a75_apply_mode_inner(
        &mut self,
        mode: u32,
        playing: bool,
        fade_in: bool,
        talk_available: bool,
        table_live: bool,
        probe: &A75ModeProbe,
        request: A75FadeRequest,
    ) -> A75ModeApply {
        let mut out = A75ModeApply::default();
        if playing {
            out.skipped = true;
            return out;
        }
        if !fade_in {
            out.set_mode = Some(self.a75_set_mode(mode, table_live, probe, request));
            return out;
        }
        self.ramp = 0.0;
        self.overlay_alpha = 0.0;
        out.ramp_zeroed = true;
        out.set_mode = Some(self.a75_set_mode(mode, table_live, probe, request));
        if talk_available {
            out.calls.push(A75Call::new(A75_IMPORT_SET_LAYER, &[3]));
            out.calls.push(A75Call::new(
                A75_IMPORT_REGISTER_CALLBACK,
                &[3, 0x14, A75_CALLBACK_00629550],
            ));
        }
        out
    }

    /// `0x00629450`: apply a mode argument, fading in first when the
    /// descriptor asks for it.
    ///
    /// ```text
    /// if (FUN_0067acd0(0x00f050f0)) return;
    /// d = FUN_00628a80(param_1);
    /// if (d[+0x20] == 0) { FUN_00629ab0(param_1); return; }
    /// 0x00faa17c = 0;
    /// FUN_006289e0(); FUN_0067bd50(1, 0);
    /// FUN_00629ab0(param_1);
    /// if (!FUN_0077ea40()) return;
    /// h = FUN_007b2020(); (*(0x025505bf))(h, 3);
    /// h = FUN_007b2020(); (*(0x02550c8c))(h, 3, 0x14, &0x00629550);
    /// ```
    ///
    /// `fade_in` is the descriptor byte at `+0x20`, `talk_available` is
    /// `FUN_0077ea40`; the descriptor itself is built by the corrupt
    /// `FUN_00628cf0` row and is not reproduced.
    pub fn a75_apply_mode(
        &mut self,
        mode: u32,
        playing: bool,
        fade_in: bool,
        talk_available: bool,
        table_live: bool,
        probe: &A75ModeProbe,
        request: A75FadeRequest,
    ) -> A75ModeApply {
        self.a75_apply_mode_inner(
            mode,
            playing,
            fade_in,
            talk_available,
            table_live,
            probe,
            request,
        )
    }

    /// `0x00629880`: the same row with the mode taken from `0x00f050dc`.
    ///
    /// The only difference from `0x00629450` is the argument source
    /// (`uVar3 = 0x00f050dc` before the guard); the body is otherwise the same
    /// text, including the `+0x20` descriptor test and the two imports.
    pub fn a75_apply_current_mode(
        &mut self,
        playing: bool,
        fade_in: bool,
        talk_available: bool,
        table_live: bool,
        probe: &A75ModeProbe,
        request: A75FadeRequest,
    ) -> A75ModeApply {
        let mode = self.mode;
        self.a75_apply_mode_inner(
            mode,
            playing,
            fade_in,
            talk_available,
            table_live,
            probe,
            request,
        )
    }

    /// `0x006298a5`: the third copy of the same body.
    ///
    /// This row reads `0x00f050dc` into `uVar3` before the guard, evaluates
    /// the descriptor byte into a local, and then follows the identical
    /// branch.  It is exposed separately so the ledger address has its own
    /// citation; the behaviour is that of `0x00629880`.
    pub fn a75_apply_current_mode_alt(
        &mut self,
        playing: bool,
        fade_in: bool,
        talk_available: bool,
        table_live: bool,
        probe: &A75ModeProbe,
        request: A75FadeRequest,
    ) -> A75ModeApply {
        self.a75_apply_current_mode(playing, fade_in, talk_available, table_live, probe, request)
    }
}

// ---------------------------------------------------------------------------
// 0x00629378 / 0x006293d6 / 0x006293eb: release a handle, then switch mode
// ---------------------------------------------------------------------------

/// One `FUN_00713200` release call.
///
/// Every row in this family spells `(handle, -1, 0)`; `-1` selects the
/// whole-handle release branch of that helper.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A75HandleStop {
    /// Frame slot passed as `param_1`.
    pub handle: u32,
    /// `-1`.
    pub slot: i32,
    /// `0`.
    pub arg: u32,
}

/// Outcome of the three release-then-mode rows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct A75ModeCommit {
    /// Vtable slot entered before the release, when the row has one (`0x10`).
    pub virtual_slot: Option<u32>,
    /// The `FUN_00713200` call.
    pub handle_stop: A75HandleStop,
    /// The `0x00629ab0` tail.
    pub set_mode: A75SetMode,
}

impl A75ScreenFade {
    fn a75_commit_after_handle_stop(
        &mut self,
        mode: u32,
        handle: u32,
        virtual_slot: Option<u32>,
        table_live: bool,
        probe: &A75ModeProbe,
        request: A75FadeRequest,
    ) -> A75ModeCommit {
        A75ModeCommit {
            virtual_slot,
            handle_stop: A75HandleStop {
                handle,
                slot: -1,
                arg: 0,
            },
            set_mode: self.a75_set_mode(mode, table_live, probe, request),
        }
    }

    /// `0x00629378`: virtual slot, release, then `0x00629ab0(0x1b)`.
    ///
    /// ```text
    /// (*(*in_EAX + 0x10))();
    /// FUN_00713200(handle, -1, 0);
    /// FUN_00629ab0(0x1b);
    /// ```
    ///
    /// The remaining stores in the row write the function's try-level byte
    /// (`[ebp - 4]`) and the catch-state checks, not controller state.
    pub fn a75_stop_handle_mode_1b(
        &mut self,
        handle: u32,
        table_live: bool,
        probe: &A75ModeProbe,
        request: A75FadeRequest,
    ) -> A75ModeCommit {
        self.a75_commit_after_handle_stop(0x1b, handle, Some(0x10), table_live, probe, request)
    }

    /// `0x006293d6`: release, then `0x00629ab0(1)`.
    ///
    /// This copy has no leading virtual call; its first statement is the
    /// try-level store.
    pub fn a75_stop_handle_mode_1(
        &mut self,
        handle: u32,
        table_live: bool,
        probe: &A75ModeProbe,
        request: A75FadeRequest,
    ) -> A75ModeCommit {
        self.a75_commit_after_handle_stop(1, handle, None, table_live, probe, request)
    }

    /// `0x006293eb`: virtual slot, release, then `0x00629ab0(1)`.
    pub fn a75_vcall_stop_handle_mode_1(
        &mut self,
        handle: u32,
        table_live: bool,
        probe: &A75ModeProbe,
        request: A75FadeRequest,
    ) -> A75ModeCommit {
        self.a75_commit_after_handle_stop(1, handle, Some(0x10), table_live, probe, request)
    }
}

// ---------------------------------------------------------------------------
// 0x00629b86 / 0x00629bf0: set-mode continuations
// ---------------------------------------------------------------------------

impl A75ScreenFade {
    /// `0x00629b86`: the set-mode tail reached through the catch handler.
    ///
    /// This row begins *after* `0x00629ab0`'s `param_1 == 0x00f050dc` guard,
    /// so it has no guard of its own.  Its head is the unconditional part of
    /// the probe (see [`a75_mode_probe_tail`]), then it writes
    /// `*(bool *)(frame - 0x18c) = (*(i32 *)(frame - 0x128) == 1)` before the
    /// begin tail.  Relative to the `FUN_00628a80` descriptor at `frame -
    /// 0x198`, that is descriptor `+0x0c` set from descriptor `+0x70 == 1`.
    /// `descriptor_word_70` is that dword; the resulting byte is stored in
    /// `0x00faa17a`, the only controller word in the window that no other row
    /// writes.
    pub fn a75_set_mode_continuation(
        &mut self,
        mode: u32,
        descriptor_word_70: i32,
        probe: &A75ModeProbe,
        request: A75FadeRequest,
    ) -> A75SetMode {
        let mut out = A75SetMode::default();
        out.probe = a75_mode_probe_tail(probe);
        self.mode = mode;
        self.a75_stop();
        out.stopped = true;
        self.active = false;
        if mode != 0 {
            self.pending = descriptor_word_70 == 1;
            let began = self.a75_begin(request);
            out.began = began.active;
            out.record = Some(began.record);
        }
        out.active_after = self.active;
        out
    }

    /// `0x00629bf0`: the same tail after the virtual slot `0x10` call.
    ///
    /// ```text
    /// (*(*in_EAX + 0x10))();
    /// if (frame[-0x194] == 4 && FUN_00709680()) FUN_00538810(0x14);
    /// 0x00f050dc = mode;
    /// FUN_0067bb30();
    /// 0x00faa179 = 0;
    /// if (mode != 0) {
    ///     0x00faa17a = descriptor[+0x0c] = (descriptor[+0x70] == 1);
    ///     FUN_006299f0();
    /// }
    /// ```
    ///
    /// Like `0x00629b86` the row starts after the set-mode guard.  It carries
    /// no probe block of its own: the `frame[-0x194]` slot is the query result
    /// the earlier half stored, so the only recovered work before the mode
    /// store is the `0x14` record request, reported through
    /// [`A75SetMode::probe`] without replaying the earlier import calls.
    pub fn a75_set_mode_after_vcall(
        &mut self,
        mode: u32,
        descriptor_word_70: i32,
        query_result: u32,
        gated: bool,
        request: A75FadeRequest,
    ) -> A75SetMode {
        let mut out = A75SetMode::default();
        if query_result == 4 && gated {
            out.probe.record_request = Some(0x14);
        }
        self.mode = mode;
        self.a75_stop();
        out.stopped = true;
        self.active = false;
        if mode != 0 {
            self.pending = descriptor_word_70 == 1;
            let began = self.a75_begin(request);
            out.began = began.active;
            out.record = Some(began.record);
        }
        out.active_after = self.active;
        out
    }
}

// ---------------------------------------------------------------------------
// 0x006287f0: the tagged option-value receiver
// ---------------------------------------------------------------------------

/// Byte offset of the option field read by `0x006287f0`.
pub const A75_OPTION_FIELD_OFFSET: u32 = 0x74;

/// Vtable slot `0x006287f0` invokes, as a byte offset.
pub const A75_OPTION_SLOT_OFFSET: u32 = 0x04;

/// The address of the word `0x006287f0` reads, from the record base.
///
/// The shipped row computes `iVar1 + 0x74`; `base` is `iVar1`.
pub fn a75_option_field_address(base: u32) -> u32 {
    base.wrapping_add(A75_OPTION_FIELD_OFFSET)
}

/// The receiver cell selected by the tagged field word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A75Receiver {
    /// Bit 0 clear: the field at `address` is itself the vtable pointer cell.
    Inline { cell: u32 },
    /// Bit 0 set: `word & !1` is the cell that holds the vtable pointer.
    Tagged { cell: u32 },
}

impl A75Receiver {
    /// The address the shipped row dereferences for the vtable pointer.
    pub fn cell(&self) -> u32 {
        match *self {
            A75Receiver::Inline { cell } | A75Receiver::Tagged { cell } => cell,
        }
    }
}

/// `0x006287f0`: decode the receiver, then call `*(*cell + 4)`.
///
/// ```text
/// p = <the value forwarded from 0x00628870>;
/// if (p == 0) return 0;
/// cell = p + 0x74;
/// if (*(u8 *)cell & 1) cell = *(u32 *)cell & 0xfffffffe;
/// return (*(*cell + 4))();
/// ```
///
/// `present` is the `p != 0` test: the value itself comes from `0x00628870`,
/// which forwards to the capacity entry `FUN_00538810`; that entry's recovered
/// body declares no return value, so `p` is not reproducible and only the
/// bit arithmetic is translated.  `read_vtable` and `invoke` stand in for the
/// two dereferences.
pub fn a75_option_value_dispatch<R, I>(
    present: bool,
    field_address: u32,
    field_word: u32,
    mut read_vtable: R,
    mut invoke: I,
) -> Option<u32>
where
    R: FnMut(u32) -> u32,
    I: FnMut(u32) -> u32,
{
    if !present {
        return None;
    }
    let receiver = if field_word & 1 != 0 {
        A75Receiver::Tagged {
            cell: field_word & 0xffff_fffe,
        }
    } else {
        A75Receiver::Inline {
            cell: field_address,
        }
    };
    let vtable = read_vtable(receiver.cell());
    Some(invoke(vtable.wrapping_add(A75_OPTION_SLOT_OFFSET)))
}

/// `0x00628830`: install the copyable-function-object vtable.
///
/// The 16-byte body is `*param_1 = &vftable`, where the vftable is the one
/// named in the decompiled row.  The address itself is a relocation and is not
/// present in the row, so the symbol name is the recoverable value.
pub const A75_FUNCTION_OBJECT_VTABLE: &str = "bcCopyableFunctionObjectImpl<<lambda_60402e77127a80e062266a5881ca2e16>&,bool,Option_const&,OptionValue_const&>::vftable";

/// The symbol installed by `0x00628830`.
pub fn a75_function_object_vtable() -> &'static str {
    A75_FUNCTION_OBJECT_VTABLE
}

/// `0x00628870`: forward the record's length word as a capacity request.
///
/// ```text
/// FUN_00538810(*(u32 *)(param_1 + 4));
/// ```
///
/// The callee is the capacity-growth entry whose policy batches 60, 68 and 77
/// recovered independently (see [`a75_grow_capacity_word`]).  `param_1` is the
/// record whose `+0x04` word is the requested size; `capacity_word` is the
/// `+0x08` word of the string the callee grows (bit 31 is the ownership/SSO
/// flag, the low 31 bits the capacity).
pub fn a75_reserve_record(record: &[u32; 3], capacity_word: u32) -> u32 {
    a75_grow_capacity_word(capacity_word, record[1])
}

/// The exact capacity arithmetic of `FUN_00538810`.
///
/// ```text
/// low = capacity_word & 0x7fffffff;
/// if (low < requested) {
///     grown = (capacity_word >> 1 & 0x3fffffff) + low;
///     if (grown < requested) grown = requested;
///     capacity_word = ((capacity_word ^ grown) & 0x7fffffff) ^ capacity_word;
/// }
/// ```
///
/// The final expression keeps bit 31 and stores `grown` in the low 31 bits.
pub fn a75_grow_capacity_word(capacity_word: u32, requested: u32) -> u32 {
    let low = capacity_word & 0x7fff_ffff;
    if low >= requested {
        return capacity_word;
    }
    let mut grown = (capacity_word >> 1 & 0x3fff_ffff).wrapping_add(low);
    if grown < requested {
        grown = requested;
    }
    ((capacity_word ^ grown) & 0x7fff_ffff) ^ capacity_word
}

/// `0x006289a8`: the two-instruction forwarder to the allocator accessor.
///
/// The row is exactly `FUN_0093ec70();` and nothing else.  That callee is the
/// TLS-guarded accessor for `PTR_DAT_00f38738`; it is outside this batch, so
/// its bootstrap is not modelled and the recovered value is the pointer it
/// returns.
pub const A75_ALLOCATOR_SLOT: u32 = 0x00f3_8738;

/// The pointer `0x006289a8` leaves in `EAX`.
pub fn a75_default_allocator_slot() -> u32 {
    A75_ALLOCATOR_SLOT
}

// ---------------------------------------------------------------------------
// 0x006289e0: the animation-table accessor
// ---------------------------------------------------------------------------

/// The static-init state behind `0x00faa0f4`.
///
/// The row is the standard thread-safe static pattern:
///
/// ```text
/// if (tls_epoch < 0x00faa0f4) {          // FUN_00c87327
///     FUN_00c87327(&0x00faa0f4);
///     if (0x00faa0f4 == -1) {
///         FUN_00679bc0();                  // build the table
///         FUN_00c86b59(&LAB_00d0e780);
///         FUN_00c872dd(&0x00faa0f4);
///     }
/// }
/// return 0x00fa94c8;
/// ```
///
/// Only the guard comparison and the `-1` test are arithmetic that survives
/// decompilation; the header/footer bodies are not recovered, so `guard_epoch`
/// is set to the observing epoch when the initializer runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A75AnimationTableInit {
    /// `0x00faa0f4`.
    pub guard_epoch: i32,
    /// `FUN_00679bc0` has run.
    pub initialized: bool,
}

impl A75AnimationTableInit {
    /// An uninitialised guard (`0xffffffff`).
    pub fn a75_new() -> Self {
        A75AnimationTableInit {
            guard_epoch: A75_INIT_UNSET,
            initialized: false,
        }
    }
}

/// Outcome of `0x006289e0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A75TableAccess {
    /// The `tls_epoch < guard` test entered the header.
    pub header_entered: bool,
    /// The `guard == -1` test ran the table initializer.
    pub initialized_now: bool,
    /// The address left in `EAX`.
    pub table: u32,
}

/// `0x006289e0`: lazily reach `0x00fa94c8` through the static-init guard.
pub fn a75_animation_table(state: &mut A75AnimationTableInit, thread_epoch: i32) -> A75TableAccess {
    let mut out = A75TableAccess {
        header_entered: false,
        initialized_now: false,
        table: A75_ANIMATION_TABLE,
    };
    if thread_epoch < state.guard_epoch {
        out.header_entered = true;
        if state.guard_epoch == A75_INIT_UNSET {
            state.initialized = true;
            state.guard_epoch = thread_epoch;
            out.initialized_now = true;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 0x00628b90 / 0x00628c25: the 0x24-byte record image
// ---------------------------------------------------------------------------

/// Dword count covering `+0x00 ..= +0x27` of the record `0x00628b90` builds.
pub const A75_ERROR_RECORD_WORDS: usize = 10;

/// The capacity request `0x00628b90` ends with: `FUN_00538810(0x24)`.
pub const A75_ERROR_RECORD_REQUEST: u32 = 0x24;

/// The `0x24`-flavoured record image of `0x00628b90`.
///
/// The row's stores, in address order:
///
/// ```text
/// +0x00 = this + 0x0c          (inline buffer pointer)
/// +0x04 = 0                    (length)
/// +0x08 = 0x8000000f           (capacity 15, bit 31 set)
/// +0x0c = 0                    (terminator)
/// +0x1c = 1
/// +0x20 = 0                    (byte)
/// +0x24 = 0                    (dword)
/// ```
///
/// and then `FUN_00538810(0x24)`.  `base` is the address of the record.  The
/// words and bytes the row does not store (`+0x0d..+0x0f`, `+0x10..+0x1b`) are
/// zero in the returned image; only the listed offsets are recovered stores.
pub fn a75_error_record(base: u32) -> [u32; A75_ERROR_RECORD_WORDS] {
    let mut words = [0u32; A75_ERROR_RECORD_WORDS];
    words[0] = base.wrapping_add(0x0c);
    words[2] = 0x8000_000f;
    words[7] = 1;
    words
}

/// The catch-handler record of `0x00628c25`.
///
/// ```text
/// *(u8 *)(*esi + 0x24) = 0;
/// esi[1] = 0x24;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A75CatchRecord {
    /// `*esi`: the object address.
    pub object: u32,
    /// The byte the handler clears: `object + 0x24`.
    pub cleared_byte_at: u32,
    /// The size word written at `esi[1]`.
    pub size: u32,
}

/// `0x00628c25`: the catch handler's record fix-up.
pub fn a75_catch_record(object: u32) -> A75CatchRecord {
    A75CatchRecord {
        object,
        cleared_byte_at: object.wrapping_add(0x24),
        size: 0x24,
    }
}

/// `0x00628cd0`: the lambda's RTTI type descriptor.
///
/// The six-byte body returns the address of the symbol named here; the
/// address is a relocation, so the name is the recoverable value.
pub const A75_LAMBDA_TYPE_DESCRIPTOR: &str =
    "<lambda_60402e77127a80e062266a5881ca2e16>::RTTI_Type_Descriptor";

/// The symbol returned by `0x00628cd0`.
pub fn a75_lambda_type_descriptor() -> &'static str {
    A75_LAMBDA_TYPE_DESCRIPTOR
}

// ---------------------------------------------------------------------------
// Flag readers: 0x00629800 and 0x00628ce0
// ---------------------------------------------------------------------------

/// `0x00629800`: `return 0x00faa179;`.
pub fn a75_is_active(state: &A75ScreenFade) -> bool {
    state.active
}

/// `0x00628ce0`: `return 0x00faa17a;`.
pub fn a75_pending_flag(state: &A75ScreenFade) -> bool {
    state.pending
}

// ---------------------------------------------------------------------------
// 0x00629de0: the state-holder constructor
// ---------------------------------------------------------------------------

/// `0x00faa18c`: destination of `0x00629de0`.
pub const A75_STATE_HOLDER: u32 = 0x00fa_a18c;

/// `0x00d3b968`: source descriptor of `0x00629de0`.
pub const A75_STATE_SOURCE: u32 = 0x00d3_b968;

/// The five arguments `0x00629de0` builds.
///
/// ```text
/// FUN_00630df0(&0x00faa18c, &0x00d3b968, param_2, param_3, param_4);
/// ```
///
/// The row's first parameter is not referenced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A75StateInit {
    /// `&0x00faa18c`.
    pub dest: u32,
    /// `&0x00d3b968`.
    pub source: u32,
    /// `param_2..param_4`, in order.
    pub args: [u32; 3],
}

/// `0x00629de0`: forward three parameters into the holder constructor.
pub fn a75_state_holder_init(param_2: u32, param_3: u32, param_4: u32) -> A75StateInit {
    A75StateInit {
        dest: A75_STATE_HOLDER,
        source: A75_STATE_SOURCE,
        args: [param_2, param_3, param_4],
    }
}

// ---------------------------------------------------------------------------
// 0x00629e00: the diagnostic-log request
// ---------------------------------------------------------------------------

/// The literal name `0x00629e00` passes to `FUN_0053a640`.
pub const A75_DIAGNOSTIC_LOG_NAME: &str = "SCR-NGDP-DiagnosticLog.txt";

/// The frame buffer `0x00629e00` passes as the second argument
/// (`local_a1[141]`).
pub const A75_DIAGNOSTIC_LOG_BUFFER_BYTES: usize = 141;

/// `0x00f4e578`: the stack-cookie word the row xors its frame into.
pub const A75_SECURITY_COOKIE: u32 = 0x00f4_e578;

/// `0x00629e00`: `FUN_0053a640("SCR-NGDP-DiagnosticLog.txt", buf, cookie)`.
///
/// The first argument is the literal; the second is a 141-byte frame buffer
/// whose address is not recoverable.  Returns the pair for citation.
pub fn a75_diagnostic_log_request() -> (&'static str, usize) {
    (A75_DIAGNOSTIC_LOG_NAME, A75_DIAGNOSTIC_LOG_BUFFER_BYTES)
}

// ---------------------------------------------------------------------------
// 0x00629f90: the sort plan and median-of-three selection
// ---------------------------------------------------------------------------

/// Element stride of the array sorted by `0x00629f90`: `0x68` bytes.
pub const A75_SORT_ELEMENT_BYTES: usize = 0x68;

/// The row's small-range limit in bytes (`0xd68` = 33 elements), so arrays of
/// at most 32 elements go to the insertion-sort callee.
pub const A75_SORT_SMALL_LIMIT_BYTES: usize = 0xd68;

/// Two elements per `0xd0` bytes, used by the middle-candidate computation.
pub const A75_SORT_MIDDLE_DIVISOR: usize = 0xd0;

/// The three candidate index triples `0x00629f90` builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A75SortAreas {
    /// `first, first + step, first + 2 * step`.
    pub first: [usize; 3],
    /// `middle - step, middle, middle + step`.
    pub middle: [usize; 3],
    /// `last - 2 * step, last - step, last`.
    pub last: [usize; 3],
}

/// What `0x00629f90` does with the range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A75SortPlan {
    /// `bytes < 0xd68`: `FUN_0062a8d0` insertion-sorts in place.  `call` is
    /// false when `first >= last`, where the row skips the call.
    Insertion { call: bool },
    /// `bytes >= 0xd68` and the recursion budget is exhausted:
    /// `FUN_0062b040` then `FUN_0062b2f0`.
    DepthExhausted,
    /// Median-of-nine pivot selection, then `FUN_0062b8f0` partitions.
    QuickSort {
        /// `(n / 8)`, the stride of each candidate triple.
        step: usize,
        /// Element index of the middle candidate (`(bytes / 0xd0)`).
        middle: usize,
        /// The three triples.
        areas: A75SortAreas,
    },
}

/// `0x00629f90`: choose the range plan and the nine candidate indexes.
///
/// The legible head of the row is
///
/// ```text
/// bytes = last - first;
/// if (bytes < 0xd68) {
///     if (first < last) FUN_0062a8d0(first, last, predicate);
///     return;
/// }
/// if (depth < 1) {
///     FUN_0062b040(first, last, predicate);
///     FUN_0062b2f0(first, last, predicate & 0xff);
///     return;
/// }
/// last_elem = last - 0x68;
/// middle    = first + (bytes / 0xd0) * 0x1a dwords;   // = bytes / 0xd0 elements
/// n         = (last_elem - first) / 0x68 + 1;
/// step      = (n + ((i32)n >> 31 & 7)) >> 3;          // = n / 8 for n >= 0
/// ...
/// ```
///
/// `bytes / 0xd0` is the element index of the middle candidate because one
/// `0x1a`-dword step is exactly one `0x68`-byte element.  The row then forms
/// the three triples below and reduces each with the same median-of-three
/// code as [`a75_median_of_three`].
pub fn a75_sort_plan(first: u32, last: u32, depth: i32) -> A75SortPlan {
    let bytes = last.wrapping_sub(first) as i32;
    if bytes < A75_SORT_SMALL_LIMIT_BYTES as i32 {
        return A75SortPlan::Insertion { call: first < last };
    }
    if depth < 1 {
        return A75SortPlan::DepthExhausted;
    }
    let element = A75_SORT_ELEMENT_BYTES as i32;
    let n_signed = (bytes - element) / element + 1;
    let step = ((n_signed + ((n_signed >> 31) & 7)) >> 3) as usize;
    let n = n_signed as usize;
    let middle = (bytes as usize) / A75_SORT_MIDDLE_DIVISOR;
    A75SortPlan::QuickSort {
        step,
        middle,
        areas: A75SortAreas {
            first: [0, step, 2 * step],
            middle: [middle - step, middle, middle + step],
            last: [n - 1 - 2 * step, n - 1 - step, n - 1],
        },
    }
}

/// `FUN_0062bd60`'s median-of-three selection, reproduced branch for branch.
///
/// With `less(a, b)` the comparator the row calls, the shipped branch order is
///
/// ```text
/// sel = mid;
/// if (!less(lo, mid)) {
///     if (less(lo, hi)) sel = lo;
///     else if (less(mid, hi)) sel = hi;
///     else sel = mid;
/// } else if (less(mid, hi)) {
///     sel = mid;
/// } else if (less(lo, hi)) {
///     sel = hi;
/// } else {
///     sel = lo;
/// }
/// ```
///
/// which returns the index of the median of the three elements.
pub fn a75_median_of_three<F>(less: &mut F, lo: usize, mid: usize, hi: usize) -> usize
where
    F: FnMut(usize, usize) -> bool,
{
    if !less(lo, mid) {
        if less(lo, hi) {
            lo
        } else if less(mid, hi) {
            hi
        } else {
            mid
        }
    } else if less(mid, hi) {
        mid
    } else if less(lo, hi) {
        hi
    } else {
        lo
    }
}

/// `0x00629f90`: reduce the three candidate triples to one pivot index.
///
/// The row calls the same median-of-three code four times: once per triple,
/// then once over the three results.  Order matters only for comparators with
/// side effects; it is the shipped order.
pub fn a75_choose_pivot<F>(less: &mut F, areas: &A75SortAreas) -> usize
where
    F: FnMut(usize, usize) -> bool,
{
    let first = a75_median_of_three(less, areas.first[0], areas.first[1], areas.first[2]);
    let middle = a75_median_of_three(less, areas.middle[0], areas.middle[1], areas.middle[2]);
    let last = a75_median_of_three(less, areas.last[0], areas.last[1], areas.last[2]);
    a75_median_of_three(less, first, middle, last)
}

// ---------------------------------------------------------------------------
// Skipped rows
// ---------------------------------------------------------------------------
//
// The following 21 `decompiled_uncited` rows in `0x00628000..=0x00629fff` are
// not translated:
//
// * `FUN_006281d5` (379 bytes) and `FUN_006283c7` (755 bytes) are both flagged
//   `Control flow encountered bad instruction data`: the first carries 250
//   `Removing unreachable block` warnings, the second 78, and both contain
//   overlapping instructions and `halt_baddata()` paths.  The second derives
//   every pointer from `unaff_EBP`/`unaff_ESI` arithmetic such as
//   `unaff_EBP + 0x1d1b4ee9`; its only legible tail is a list unlink/append
//   sequence whose control flow is corrupt.
// * `0x00628870` is translated above, but `FUN_00628a80` (99 bytes) is not: it
//   builds an object with the corrupt `FUN_00628cf0` row and forwards
//   `*(object + 8)` to `FUN_00538810`, so neither the object nor the forwarded
//   value is recoverable.
// * `FUN_006288b6` (14 bytes) is an unwind fragment: it stores
//   `[ebp - 0x2c] = ebp - 0x20` and tail-calls `FUN_00538810` with an
//   `unaff_EDI` argument.  It has no entry state of its own.
// * `FUN_00628bf4` (47 bytes) has overlapping instructions
//   (`0x00628c10`/`0x00628c0e`) and its only legible content is a
//   `lock xadd`-style decrement at `unaff_EBX + 0x3b246a0e` followed by two
//   non-returning paths; the counter address comes from an unrecovered
//   register.
// * `FUN_00628cf0` (313 bytes) is flagged bad-instruction data with 148
//   `Removing unreachable block` warnings.  It contains a legible insertion-sort
//   inner loop (`while (value < prev[-1]) prev[0] = prev[-1]`) but every
//   bound comes from garbage registers, and the real insertion sort of this
//   window is the `FUN_0062a8d0` callee named by `0x00629f90`.
// * `FUN_00628eb3` (145 bytes), `FUN_00628f46` (21 bytes) and `FUN_00628f5b`
//   (89 bytes) repeat the same corrupt block: all three read
//   `unaff_EBP + 0xb9bdad...` slots, jump through `FUN_00a8f9d0`/
//   `FUN_00a8fa00`, and end in `halt_baddata()`.
// * `FUN_00629059` (3 bytes) and `FUN_006290f2` (2 bytes) are single
//   `halt_baddata()` rows: the decompiler recovered no instruction at all.
// * `FUN_00629172`, `FUN_0062942b`, `FUN_00629d11` and `FUN_00629f6d` are catch
//   funclets whose whole body is `(**(code **)(*in_EAX + 0x10))();` plus the
//   `ExceptionList` restore and `FUN_00c86b6e()` (rethrow).  The only
//   recoverable constant is the vtable slot `0x10`, which the translated
//   `0x00629378` / `0x006293eb` rows already record.
// * `FUN_0062922d` (107 bytes) is the adopt fragment of `0x006291c7` emitted as
//   a separate row.  Its head is decompiler junk
//   (`(byte *)(ebp - 0x7d + eax)` and an `or` of `in_DH`), and the mode store
//   itself is garbled to `0x00f050dc = in_EAX | 0x75`, so the adopted value is
//   not recoverable; the legible tail is the same three calls that
//   [`A75ScreenFade::a75_on_mode_changed`] records.
// * `FUN_006293b2` (36 bytes) is an unwind funclet: it clears a byte at
//   `vtable + 0xb`, stores `0xb`, calls `FUN_0090da70` and enters
//   `FUN_0053ae90`.  The addresses are frame-relative.
// * `FUN_00629ebe`, `FUN_00629edd`, `FUN_00629efc` and `FUN_00629f1b` (31, 31, 31 and
//   82 bytes) are four unwind cascades over the same frame: each clears
//   try-level markers at `[ebp - 0x24]`, `[ebp - 0x40]`, `[ebp - 0x5c]` and
//   `[ebp - 0x78]`, calls `FUN_00630b10(ebp - 0x80, 0)` and `FUN_00b8e660`, then
//   rethrows at `0x00629dd0`.  No controller word survives.
// * `FUN_00628c18` is the 2-byte thunk `thunk_FUN_00628c25`, so it is a ledger
//   `thunk` row rather than a `decompiled_uncited` one.

#[cfg(test)]
mod tests_agent75 {
    use super::*;

    fn state() -> A75ScreenFade {
        A75ScreenFade::a75_new()
    }

    fn probe_none() -> A75ModeProbe {
        A75ModeProbe::default()
    }

    fn request() -> A75FadeRequest {
        A75FadeRequest::new(0x11, 0.5, true)
    }

    // -- 0x00629571 -------------------------------------------------------

    #[test]
    fn tick_steps_the_ramp_by_five_percent() {
        let mut s = state();
        s.ramp = 0.0;
        s.intensity = 1.0;
        let tick = s.a75_tick(1.0, 7);
        assert!(!tick.completed);
        assert_eq!(tick.alpha, A75_RAMP_STEP);
        assert_eq!(s.ramp, A75_RAMP_STEP);
        assert_eq!(tick.callback_arg, None);
        assert_eq!(s.overlay_alpha, A75_RAMP_STEP);
    }

    #[test]
    fn tick_completes_and_reapplies_the_intensity() {
        let mut s = state();
        s.ramp = 0.0;
        s.intensity = 0.5;
        let tick = s.a75_tick(0.05, 9);
        assert!(tick.completed);
        assert_eq!(s.ramp, 1.0);
        assert_eq!(tick.callback_arg, Some(9));
        // Completion snaps the ramp to 1.0 but `scale` stays at the target,
        // and the product 0.05 * 0.5 is still below the target.
        assert_eq!(tick.alpha, 0.05 * 0.5);
        // The second apply uses the clamped intensity.
        assert_eq!(s.overlay_alpha, 0.5);
    }

    #[test]
    fn tick_applies_the_reduced_multiplier() {
        let mut s = state();
        s.ramp = 0.0;
        s.reduced = true;
        s.intensity = 0.4;
        let tick = s.a75_tick(1.0, 0);
        assert_eq!(tick.alpha, A75_RAMP_STEP * 0.4 * 0.75);
        assert_eq!(s.overlay_alpha, A75_RAMP_STEP * 0.4 * 0.75);
    }

    #[test]
    fn tick_nan_target_keeps_nan_alpha() {
        let mut s = state();
        s.ramp = 0.0;
        let tick = s.a75_tick(f32::NAN, 0);
        assert!(!tick.completed);
        assert!(tick.alpha.is_nan());
        assert!(s.overlay_alpha.is_nan());
    }

    #[test]
    fn tick_floors_a_negative_alpha_at_zero() {
        let mut s = state();
        s.ramp = 0.0;
        s.intensity = 1.0;
        let tick = s.a75_tick(-1.0, 0);
        assert!(tick.completed);
        assert_eq!(tick.alpha, 0.0);
        assert_eq!(tick.alpha.to_bits(), 0);
    }

    // -- intensity clamps ------------------------------------------------

    #[test]
    fn intensity_clamp_uses_the_ceiling_first() {
        let mut s = state();
        assert_eq!(s.a75_set_intensity(1.5), 1.0);
        assert_eq!(s.a75_set_intensity(f32::NAN), 1.0);
        assert_eq!(s.a75_set_intensity(f32::INFINITY), 1.0);
        assert_eq!(s.a75_set_intensity(-2.0), 0.0);
        assert_eq!(s.a75_set_intensity(-0.0).to_bits(), 0);
        assert_eq!(s.a75_set_intensity(0.25), 0.25);
    }

    #[test]
    fn set_intensity_applies_the_reduced_multiplier() {
        let mut s = state();
        s.reduced = true;
        assert_eq!(s.a75_set_intensity(0.8), 0.6);
        assert_eq!(s.overlay_alpha, 0.6);
        assert_eq!(s.intensity, 0.8);
    }

    // -- 0x006299f0 -------------------------------------------------------

    #[test]
    fn begin_clears_reduced_and_starts() {
        let mut s = state();
        s.reduced = true;
        s.intensity = 2.0;
        let began = s.a75_begin(A75FadeRequest::new(0x42, 1.5, true));
        assert_eq!(s.intensity, 1.0);
        assert!(!s.reduced);
        assert_eq!(s.overlay_alpha, 1.0);
        assert!(s.active);
        assert!(!began.stopped_after_failure);
        assert_eq!(
            began.record,
            A75FadeRecord {
                id: 0x42,
                layer: 1,
                duration: 1.5,
                target: 1.0
            }
        );
    }

    #[test]
    fn begin_failure_stops_and_clears_active() {
        let mut s = state();
        s.active = true;
        let began = s.a75_begin(A75FadeRequest::new(1, 0.0, false));
        assert!(!began.active);
        assert!(began.stopped_after_failure);
        assert!(!s.active);
    }

    // -- 0x00629670 / 0x00629d60 -----------------------------------------

    #[test]
    fn reset_and_apply_damps_once_per_call() {
        let mut s = state();
        s.reduced = true;
        s.intensity = 1.0;
        let reset = s.a75_reset_and_apply(false);
        assert!(!reset.timer_call);
        assert_eq!(reset.intensity, 0.75);
        assert_eq!(reset.overlay_alpha, 0.75 * 0.75);
        // The damping writes back: a second call multiplies again.
        let again = s.a75_reset_and_apply(false);
        assert_eq!(again.intensity, 0.5625);
    }

    #[test]
    fn reset_and_apply_playing_branch_resets_the_ramp() {
        let mut s = state();
        s.active = true;
        s.ramp = 0.3;
        let reset = s.a75_reset_and_apply(true);
        assert!(reset.timer_call);
        assert!(!s.active);
        assert_eq!(s.ramp, 1.0);
    }

    #[test]
    fn clear_reduced_only_acts_when_set() {
        let mut s = state();
        assert!(!s.a75_clear_reduced());
        s.reduced = true;
        s.intensity = 3.0;
        assert!(s.a75_clear_reduced());
        assert_eq!(s.intensity, 1.0);
        assert_eq!(s.overlay_alpha, 1.0);
        assert!(!s.reduced);
    }

    #[test]
    fn stop_and_clear_active_only_clears_the_flag() {
        let mut s = state();
        s.active = true;
        s.pending = true;
        s.a75_stop_and_clear_active();
        assert!(!s.active);
        assert!(s.pending);
    }

    // -- 0x00629ab0 probe -------------------------------------------------

    #[test]
    fn probe_calls_the_imports_and_requests_the_record() {
        let input = A75ModeProbe {
            session: true,
            display: true,
            selector: 0,
            primary: true,
            secondary: false,
            cleanup: true,
            clear_object: Some(0x5000),
            import_result: 4,
            gated: true,
        };
        let out = a75_mode_probe(&input);
        assert_eq!(out.cleared_byte_at, Some(0x5001));
        assert_eq!(out.query_result, Some(4));
        assert!(out.cleanup);
        assert_eq!(out.record_request, Some(0x14));
        assert_eq!(
            out.calls,
            vec![
                A75Call::new(A75_IMPORT_CLEAR_FLAG, &[]),
                A75Call::new(A75_IMPORT_QUERY, &[]),
            ]
        );
    }

    #[test]
    fn probe_uses_the_secondary_query_when_the_selector_is_set() {
        let input = A75ModeProbe {
            session: true,
            display: true,
            selector: 2,
            primary: false,
            secondary: true,
            import_result: 3,
            ..A75ModeProbe::default()
        };
        let out = a75_mode_probe(&input);
        assert_eq!(out.cleared_byte_at, None);
        assert_eq!(out.query_result, Some(3));
        // A result other than 4 never requests the record, even when gated.
        let gated = A75ModeProbe {
            gated: true,
            ..input
        };
        assert_eq!(a75_mode_probe(&gated).record_request, None);
    }

    #[test]
    fn probe_short_circuits_without_a_session() {
        let input = A75ModeProbe {
            display: true,
            primary: true,
            cleanup: true,
            ..A75ModeProbe::default()
        };
        let out = a75_mode_probe(&input);
        assert!(out.calls.is_empty());
        assert!(out.cleared_byte_at.is_none());
        assert!(!out.cleanup);
    }

    // -- 0x00629ab0 / 0x00628840 -----------------------------------------

    #[test]
    fn set_mode_guard_skips_the_whole_body() {
        let mut s = state();
        s.mode = 8;
        let out = s.a75_set_mode(8, true, &probe_none(), request());
        assert!(out.skipped);
        assert!(!out.stopped);
        assert!(!out.began);
    }

    #[test]
    fn set_mode_zero_stops_without_beginning() {
        let mut s = state();
        s.mode = 4;
        s.active = true;
        let out = s.a75_set_mode(0, true, &probe_none(), request());
        assert!(!out.skipped);
        assert!(out.stopped);
        assert!(!out.began);
        assert_eq!(s.mode, 0);
        assert!(!s.active);
        assert!(out.record.is_none());
        // A zero mode never runs the probe.
        assert!(out.probe.calls.is_empty());
    }

    #[test]
    fn set_mode_tail_starts_the_fade() {
        let mut s = state();
        s.mode = 1;
        let out = s.a75_set_mode(4, true, &probe_none(), request());
        assert_eq!(s.mode, 4);
        assert!(out.stopped);
        assert!(out.began);
        assert!(out.active_after);
        assert_eq!(out.record.unwrap().layer, 1);
    }

    #[test]
    fn reset_mode_19_clears_both_bytes() {
        let mut s = state();
        s.active = true;
        s.pending = true;
        let out = s.a75_reset_mode_19(false, &probe_none(), request());
        assert_eq!(s.mode, 0x19);
        assert!(!s.pending);
        assert!(out.began);
    }

    #[test]
    fn set_mode_continuation_stores_the_pending_byte() {
        let mut s = state();
        let out = s.a75_set_mode_continuation(2, 1, &probe_none(), request());
        assert!(s.pending);
        assert!(out.began);
        let mut s = state();
        s.pending = true;
        let _ = s.a75_set_mode_continuation(2, 0, &probe_none(), request());
        assert!(!s.pending);
    }

    #[test]
    fn set_mode_after_vcall_commits_the_tail_without_a_probe() {
        let mut s = state();
        let out = s.a75_set_mode_after_vcall(3, 1, 0, false, request());
        assert_eq!(s.mode, 3);
        assert!(s.pending);
        assert!(out.stopped);
        assert!(out.began);
        assert!(out.probe.calls.is_empty());
    }

    #[test]
    fn set_mode_after_vcall_reports_the_stored_query_result() {
        let mut a = state();
        let out = a.a75_set_mode_after_vcall(3, 0, 4, true, request());
        assert_eq!(out.probe.record_request, Some(0x14));
        // The first half's import calls are not replayed by this row.
        assert!(out.probe.calls.is_empty());
        let mut b = state();
        let out = b.a75_set_mode_after_vcall(3, 0, 4, false, request());
        assert_eq!(out.probe.record_request, None);
    }

    // -- 0x00629378 / 0x006293d6 / 0x006293eb -----------------------------

    #[test]
    fn handle_release_rows_record_the_exact_sequence() {
        let mut s = state();
        let out = s.a75_stop_handle_mode_1b(0x7000, false, &probe_none(), request());
        assert_eq!(out.virtual_slot, Some(0x10));
        assert_eq!(
            out.handle_stop,
            A75HandleStop {
                handle: 0x7000,
                slot: -1,
                arg: 0
            }
        );
        assert_eq!(s.mode, 0x1b);

        let mut s = state();
        let out = s.a75_stop_handle_mode_1(0x7001, false, &probe_none(), request());
        assert_eq!(out.virtual_slot, None);
        assert_eq!(out.handle_stop.slot, -1);
        assert_eq!(s.mode, 1);

        let mut s = state();
        let out = s.a75_vcall_stop_handle_mode_1(0x7002, false, &probe_none(), request());
        assert_eq!(out.virtual_slot, Some(0x10));
        assert_eq!(s.mode, 1);
    }

    // -- 0x00629450 / 0x00629880 / 0x006298a5 ----------------------------

    #[test]
    fn apply_mode_is_a_noop_while_the_handle_plays() {
        let mut s = state();
        s.mode = 4;
        let out = s.a75_apply_mode(8, true, true, true, false, &probe_none(), request());
        assert!(out.skipped);
        assert_eq!(s.mode, 4);
        assert!(out.calls.is_empty());
        assert!(out.set_mode.is_none());
    }

    #[test]
    fn apply_mode_without_fade_in_only_sets_the_mode() {
        let mut s = state();
        s.ramp = 0.4;
        let out = s.a75_apply_mode(4, false, false, true, false, &probe_none(), request());
        assert!(!out.ramp_zeroed);
        assert_eq!(s.ramp, 0.4);
        assert_eq!(s.mode, 4);
        assert!(out.calls.is_empty());
    }

    #[test]
    fn apply_mode_fade_in_zeros_the_ramp_and_registers_the_callback() {
        let mut s = state();
        s.ramp = 0.4;
        s.overlay_alpha = 0.9;
        let out = s.a75_apply_mode(4, false, true, true, false, &probe_none(), request());
        assert!(out.ramp_zeroed);
        assert_eq!(s.ramp, 0.0);
        assert_eq!(
            out.calls,
            vec![
                A75Call::new(A75_IMPORT_SET_LAYER, &[3]),
                A75Call::new(
                    A75_IMPORT_REGISTER_CALLBACK,
                    &[3, 0x14, A75_CALLBACK_00629550]
                ),
            ]
        );
        assert_eq!(out.calls[1].args(), &[3, 0x14, A75_CALLBACK_00629550]);
        assert_eq!(s.mode, 4);
    }

    #[test]
    fn apply_mode_fade_in_without_talk_skips_the_imports() {
        let mut s = state();
        let out = s.a75_apply_mode(8, false, true, false, false, &probe_none(), request());
        assert!(out.ramp_zeroed);
        assert!(out.calls.is_empty());
    }

    #[test]
    fn apply_current_mode_and_alt_read_the_stored_mode() {
        let mut a = state();
        a.mode = 8;
        let out_a = a.a75_apply_current_mode(false, false, false, true, &probe_none(), request());
        assert_eq!(a.mode, 8);
        assert!(out_a.set_mode.as_ref().unwrap().skipped);

        let mut b = state();
        b.mode = 8;
        let out_b =
            b.a75_apply_current_mode_alt(false, false, false, true, &probe_none(), request());
        assert_eq!(out_a, out_b);
        assert_eq!(a, b);
    }

    // -- 0x006291c7 -------------------------------------------------------

    #[test]
    fn mode_change_requires_active_and_kind_two() {
        let mut s = state();
        let out = s.a75_on_mode_changed(2, 0x10, 4, false, true, &probe_none(), request());
        assert!(out.inactive);
        assert!(!out.set_mode_called);

        s.active = true;
        let out = s.a75_on_mode_changed(3, 0x10, 4, false, true, &probe_none(), request());
        assert!(out.kind_mismatch);
        assert!(!out.set_mode_called);

        let out = s.a75_on_mode_changed(2, 0x10, 4, true, true, &probe_none(), request());
        assert!(out.played);
        assert!(!out.set_mode_called);
    }

    #[test]
    fn mode_change_adopts_an_allowed_candidate_and_skips_the_tail() {
        let mut s = state();
        s.active = true;
        s.mode = 1;
        let out = s.a75_on_mode_changed(2, 0x10, 8, false, true, &probe_none(), request());
        assert!(out.adopted);
        assert_eq!(s.mode, 8);
        assert!(out.set_mode_called);
        assert!(out.set_mode.skipped);
        assert_eq!(
            out.adopt_calls,
            vec![
                A75Call::new(A75_APPLY_DESCRIPTOR, &[]),
                A75Call::new(A75_FLUSH_DESCRIPTOR, &[]),
            ]
        );
        // The adopt branch leaves the active flag alone.
        assert!(s.active);
    }

    #[test]
    fn mode_change_rejects_a_level_outside_one_to_0x1f() {
        for level in [0u16, 0x20] {
            let mut s = state();
            s.active = true;
            s.mode = 1;
            let out = s.a75_on_mode_changed(2, level, 4, false, true, &probe_none(), request());
            assert!(!out.adopted);
            assert!(!out.set_mode.skipped);
            assert_eq!(s.mode, 4);
            assert!(s.active);
        }
    }

    #[test]
    fn mode_change_rejects_a_candidate_outside_the_three_modes() {
        for candidate in [0i32, 1, 5, 0x0b, 0x0d] {
            let mut s = state();
            s.active = true;
            s.mode = 1;
            let out = s.a75_on_mode_changed(2, 1, candidate, false, true, &probe_none(), request());
            assert!(!out.adopted);
            assert!(out.adopt_calls.is_empty());
            assert_eq!(s.mode, candidate as u32);
        }
    }

    #[test]
    fn mode_change_accepts_every_adoptable_mode() {
        for candidate in A75_ADOPTABLE_MODES {
            let mut s = state();
            s.active = true;
            let out =
                s.a75_on_mode_changed(2, 0x1f, candidate, false, true, &probe_none(), request());
            assert!(out.adopted);
            assert_eq!(s.mode, candidate as u32);
        }
    }

    // -- 0x006287f0 -------------------------------------------------------

    #[test]
    fn option_dispatch_untags_the_field() {
        let mut invoked = None;
        let value = a75_option_value_dispatch(
            true,
            0x2000,
            0x3001,
            |cell| {
                assert_eq!(cell, 0x3000);
                0x7000
            },
            |slot| {
                invoked = Some(slot);
                0x1234
            },
        );
        assert_eq!(value, Some(0x1234));
        assert_eq!(invoked, Some(0x7004));
    }

    #[test]
    fn option_field_and_level_word_offsets() {
        assert_eq!(a75_option_field_address(0x1000), 0x1074);
        assert_eq!(A75_OPTION_FIELD_OFFSET, 0x74);
        assert_eq!(a75_level_word_address(0), 0x154);
        assert_eq!(a75_level_word_address(0xf000_0000), 0xf000_0154);
    }

    #[test]
    fn option_dispatch_inline_cell_is_the_field_address() {
        let value = a75_option_value_dispatch(
            true,
            0x2000,
            0x7000,
            |cell| {
                assert_eq!(cell, 0x2000);
                0x9000
            },
            |slot| {
                assert_eq!(slot, 0x9004);
                7
            },
        );
        assert_eq!(value, Some(7));
    }

    #[test]
    fn option_dispatch_returns_none_when_absent() {
        let value =
            a75_option_value_dispatch(false, 0, 0, |_| panic!("no read"), |_| panic!("no call"));
        assert_eq!(value, None);
    }

    // -- 0x00628830 / 0x00628870 / 0x006289a8 / 0x00628cd0 ----------------

    #[test]
    fn vtable_and_rtti_names_are_exact() {
        assert_eq!(a75_function_object_vtable(), A75_FUNCTION_OBJECT_VTABLE);
        assert!(A75_FUNCTION_OBJECT_VTABLE.starts_with(
            "bcCopyableFunctionObjectImpl<<lambda_60402e77127a80e062266a5881ca2e16>&,bool"
        ));
        assert!(A75_FUNCTION_OBJECT_VTABLE.ends_with(">::vftable"));
        assert_eq!(a75_lambda_type_descriptor(), A75_LAMBDA_TYPE_DESCRIPTOR);
        assert!(A75_LAMBDA_TYPE_DESCRIPTOR.ends_with(">::RTTI_Type_Descriptor"));
    }

    #[test]
    fn reserve_record_uses_the_length_word() {
        let record = [0u32, 20, 0];
        let grown = a75_reserve_record(&record, 0x8000_000f);
        assert_eq!(grown, 0x8000_0016);
        // 15 >= 10: the word is returned unchanged.
        assert_eq!(a75_reserve_record(&[0, 10, 0], 0x8000_000f), 0x8000_000f);
        // A request above the 1.5x growth wins.
        assert_eq!(a75_reserve_record(&[0, 100, 0], 0x8000_000f), 0x8000_0064);
    }

    #[test]
    fn allocator_slot_is_the_recovered_pointer() {
        assert_eq!(a75_default_allocator_slot(), 0x00f3_8738);
    }

    // -- 0x006289e0 -------------------------------------------------------

    #[test]
    fn animation_table_initialises_once_for_an_older_epoch() {
        let mut init = A75AnimationTableInit::a75_new();
        let first = a75_animation_table(&mut init, -2);
        assert!(first.header_entered);
        assert!(first.initialized_now);
        assert_eq!(first.table, A75_ANIMATION_TABLE);
        assert!(init.initialized);
        let second = a75_animation_table(&mut init, -2);
        assert!(!second.initialized_now);
    }

    #[test]
    fn animation_table_skips_the_guard_for_a_newer_epoch() {
        let mut init = A75AnimationTableInit::a75_new();
        let out = a75_animation_table(&mut init, 5);
        assert!(!out.header_entered);
        assert!(!out.initialized_now);
        assert_eq!(out.table, A75_ANIMATION_TABLE);
        assert!(!init.initialized);
    }

    // -- 0x00628b90 / 0x00628c25 -----------------------------------------

    #[test]
    fn error_record_image_is_exact() {
        let words = a75_error_record(0x1000);
        assert_eq!(words.len(), A75_ERROR_RECORD_WORDS);
        assert_eq!(words[0], 0x100c);
        assert_eq!(words[1], 0);
        assert_eq!(words[2], 0x8000_000f);
        assert_eq!(words[3], 0);
        assert_eq!(&words[4..7], &[0, 0, 0]);
        assert_eq!(words[7], 1);
        assert_eq!(words[8], 0);
        assert_eq!(words[9], 0);
        assert_eq!(A75_ERROR_RECORD_REQUEST, 0x24);
    }

    #[test]
    fn catch_record_clears_the_object_byte() {
        let record = a75_catch_record(0x4000);
        assert_eq!(record.object, 0x4000);
        assert_eq!(record.cleared_byte_at, 0x4024);
        assert_eq!(record.size, 0x24);
    }

    // -- flag readers and holder ------------------------------------------

    #[test]
    fn flag_readers_report_the_state_bytes() {
        let mut s = state();
        assert!(!a75_is_active(&s));
        assert!(!a75_pending_flag(&s));
        s.active = true;
        s.pending = true;
        assert!(a75_is_active(&s));
        assert!(a75_pending_flag(&s));
    }

    #[test]
    fn state_holder_carries_the_five_arguments() {
        let init = a75_state_holder_init(1, 2, 3);
        assert_eq!(init.dest, A75_STATE_HOLDER);
        assert_eq!(init.source, A75_STATE_SOURCE);
        assert_eq!(init.dest, 0x00fa_a18c);
        assert_eq!(init.source, 0x00d3_b968);
        assert_eq!(init.args, [1, 2, 3]);
    }

    // -- 0x00629e00 -------------------------------------------------------

    #[test]
    fn diagnostic_log_name_is_exact() {
        assert_eq!(
            a75_diagnostic_log_request(),
            ("SCR-NGDP-DiagnosticLog.txt", 141)
        );
        assert_eq!(A75_DIAGNOSTIC_LOG_NAME.len(), 26);
        assert_eq!(A75_SECURITY_COOKIE, 0x00f4_e578);
    }

    // -- 0x00629f90 -------------------------------------------------------

    #[test]
    fn sort_plan_uses_insertion_below_the_threshold() {
        let bytes = 32 * A75_SORT_ELEMENT_BYTES as u32;
        assert_eq!(
            a75_sort_plan(0x1000, 0x1000 + bytes, 5),
            A75SortPlan::Insertion { call: true }
        );
        assert_eq!(
            a75_sort_plan(0x1000, 0x1000, 5),
            A75SortPlan::Insertion { call: false }
        );
    }

    #[test]
    fn sort_plan_falls_back_when_the_depth_is_exhausted() {
        let bytes = 33 * A75_SORT_ELEMENT_BYTES as u32;
        assert_eq!(
            a75_sort_plan(0x1000, 0x1000 + bytes, 0),
            A75SortPlan::DepthExhausted
        );
    }

    #[test]
    fn sort_plan_builds_the_nine_candidates() {
        let n = 33usize;
        let bytes = (n * A75_SORT_ELEMENT_BYTES) as u32;
        let plan = a75_sort_plan(0x1000, 0x1000 + bytes, 4);
        match plan {
            A75SortPlan::QuickSort {
                step,
                middle,
                areas,
            } => {
                assert_eq!(step, n / 8);
                assert_eq!(middle, n / 2);
                assert_eq!(areas.first, [0, 4, 8]);
                assert_eq!(areas.middle, [12, 16, 20]);
                assert_eq!(areas.last, [24, 28, 32]);
            }
            other => panic!("expected a quicksort plan, got {other:?}"),
        }
    }

    #[test]
    fn sort_plan_matches_the_byte_arithmetic_for_odd_counts() {
        // 67 elements: 67 / 2 truncates to 33 for the middle candidate and
        // 67 / 8 to 8 for the step.
        let n = 67usize;
        let bytes = (n * A75_SORT_ELEMENT_BYTES) as u32;
        match a75_sort_plan(0, bytes, 3) {
            A75SortPlan::QuickSort {
                step,
                middle,
                areas,
            } => {
                assert_eq!(step, 8);
                assert_eq!(middle, 33);
                assert_eq!(areas.middle, [25, 33, 41]);
                assert_eq!(areas.last, [50, 58, 66]);
            }
            other => panic!("expected a quicksort plan, got {other:?}"),
        }
    }

    #[test]
    fn median_of_three_selects_the_middle_value_for_all_orders() {
        let values = [10u32, 20, 30];
        let permutations = [
            [0usize, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        for order in permutations {
            let mut less = |a: usize, b: usize| values[a] < values[b];
            let winner = a75_median_of_three(&mut less, order[0], order[1], order[2]);
            assert_eq!(
                values[winner], 20,
                "order {order:?} selected index {winner}"
            );
        }
    }

    #[test]
    fn choose_pivot_reduces_the_three_medians() {
        let values: Vec<u32> = (0..64).collect();
        let areas = A75SortAreas {
            first: [0, 8, 16],
            middle: [24, 32, 40],
            last: [47, 55, 63],
        };
        let mut less = |a: usize, b: usize| values[a] < values[b];
        assert_eq!(a75_choose_pivot(&mut less, &areas), 32);
    }

    #[test]
    fn choose_pivot_uses_the_plan_indexes() {
        let values: Vec<u32> = (0..33).map(|i| 33 - i).collect();
        let plan = a75_sort_plan(0, 33 * A75_SORT_ELEMENT_BYTES as u32, 2);
        let A75SortPlan::QuickSort { areas, .. } = plan else {
            panic!("expected a quicksort plan");
        };
        let mut less = |a: usize, b: usize| values[a] < values[b];
        let pivot = a75_choose_pivot(&mut less, &areas);
        // Descending data: every triple median is its middle index.
        assert_eq!(pivot, 16);
    }
}
