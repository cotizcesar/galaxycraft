#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent62.rs
// ===========================================================================
// Translation batch 62.  Only this file is edited by the batch.
//
// Batch 62 owns `0x0060e000..=0x0060ffff`.  The ledger lists 20 rows in that
// window: three `decompile_failed` fragments of `FUN_0060de60`, twelve
// `decompiled_uncited` rows with coherent bodies, and five
// `decompiled_uncited` rows that are mid-instruction pieces of the two
// obfuscated blobs at `FUN_0060ed26` and `FUN_0060f4dd`.  The twelve coherent
// rows are translated below; the skip reasons are recorded in the notes at
// the end of this file.
//
// # Recovered module
//
// The window is the tail of a screen-resolution / gamma-ramp controller.  Its
// state block is a run of globals at `0x00f04ac0..0x00f04b08`:
//
// | global | type | meaning |
// | --- | --- | --- |
// | `0x00f04ac0` | `int` | screen extent (width) |
// | `0x00f04ac4` | `int` | `level * scale`, rounded |
// | `0x00f04ac8` | `int` | level (height-like extent) |
// | `0x00f04acc` | `f32` | `0x3f4c49ba` = 0.79799998 |
// | `0x00f04ad0` | `f32` | current gamma value |
// | `0x00f04ad4` | `f32` | ramp start |
// | `0x00f04ad8` | `f32` | ramp end |
// | `0x00f04adc` | `u8` | ramp active |
// | `0x00f04ae0` | `f32` | linear progress, 0..=1 |
// | `0x00f04ae8` | `u64` | timestamp of the last applied step |
// | `0x00f04af0` | `int` | placement mode (1, 2 or 3) |
// | `0x00f04af4` | `u8` | use the curve-rate ramp |
// | `0x00f04af8` | `int` | curve-rate speed |
// | `0x00f04afc` | `f32` | pending x offset (mode 2) |
// | `0x00f04b00` | `f32` | pending y offset (mode 2) |
// | `0x00f04b08` | `f32` | curve-rate gain |
//
// `0x0060c9c5`-style rounding: every `FUN_00c9c5ae` call in this window is the
// CRT `roundf`, which rounds halfway cases away from zero.  It was identified
// by executing it out of the shipped image: `2.5 -> 3.0`, `-2.5 -> -3.0`,
// `2.4 -> 2.0`, `-0.4 -> -0.0`.
//
// Two helper routines are outside the window and stay external, so their
// results enter this batch as explicit values:
//
// * `FUN_0060c0e0` maps a raw ramp value to the stored gamma value; it is the
//   `curved` parameter below (the two call sites are `0x0060f900` and
//   `0x0060fba0`).
// * `FUN_00c94230` maps linear progress to the eased fraction; it is the
//   `eased` parameter of [`A62Gamma::a62_tick`].
//
// No crate type is extended and no shared module is touched.

// ---------------------------------------------------------------------------
// Constants and float helpers
// ---------------------------------------------------------------------------

/// `0x00f04acc`: `movl $0x3f4c49ba`, the level scale used by `FUN_0060d940`
/// and mode 2 of `0x0060fc9a`.
pub const A62_LEVEL_SCALE: f32 = f32::from_bits(0x3f4c_49ba);

/// `0x00f04adc` time gate: steps of `0.005` seconds or less are ignored and
/// do not advance the stored timestamp (`0xd399d8`).
pub const A62_STEP_FLOOR: f64 = 0.005;

/// Upper clamp applied to the elapsed time of one step (`0xd399e0`).  The
/// stored double is the exact double value of `0.1f32`
/// (`0x3FB99999A0000000`), not the double literal `0.1`.
pub const A62_STEP_CEIL: f64 = 0.1f32 as f64;

/// Linear progress gained per second (`0xd399f0`, `8.0f`).
pub const A62_PROGRESS_PER_SECOND: f32 = 8.0;

/// Completion threshold (`0xd399cc`, `2^-22`).
pub const A62_EPSILON: f32 = f32::from_bits(0x3480_0000);

/// `0xd399f8`: nanoseconds per second used by `0x0060f900`.
pub const A62_NANOS_PER_SECOND: f64 = 1.0e9;

/// The CRT `roundf` used throughout the window: halfway cases away from zero.
#[inline]
pub fn a62_round_f32(value: f32) -> f32 {
    value.round()
}

/// `cvttss2si`: truncate toward zero, with the x86 invalid-result sentinel
/// (`INT_MIN`) for NaN and out-of-range inputs.  Rust's `as i32` saturates
/// instead, so the shipped conversion is reproduced explicitly.
#[inline]
pub fn a62_trunc_i32(value: f32) -> i32 {
    if value.is_nan() || value >= 2_147_483_648.0 || value < -2_147_483_648.0 {
        i32::MIN
    } else {
        value as i32
    }
}

/// `(int)roundf(value)`, the shape used by the shipped rounding sites.
#[inline]
pub fn a62_round_i32(value: f32) -> i32 {
    a62_trunc_i32(a62_round_f32(value))
}

/// `0x0060f900`: elapsed seconds between two `edx:eax` timestamps.
///
/// The shipped subtraction is 64-bit wrapping, then `FUN_00c87120` converts the
/// signed 64-bit result to `double`; the division by `1.0e9` follows.
pub fn a62_step_seconds(now: u64, last: u64) -> f64 {
    let delta = now.wrapping_sub(last);
    (delta as i64) as f64 / A62_NANOS_PER_SECOND
}

/// `0x0060f900`: the curve-rate denominator for the current gamma value.
///
/// The shipped branch is `1.0 <= value`; `comiss` + `jbe` is also true for an
/// unordered comparison, so NaN takes the first branch as well:
///
/// ```text
/// value >= 1.0:  2.0 - (value - 1.0) * 1.75
/// value <  1.0:  t = 1.0 - (value - 0.25) / 0.75;  t + t + 2.0
/// ```
pub fn a62_curve_denominator(value: f32) -> f32 {
    // `comiss` + `jbe` takes the first branch for NaN as well, so the test is
    // `!(1.0 > value)` rather than `1.0 <= value`.
    if !(1.0 > value) {
        2.0 - (value - 1.0) * 1.75
    } else {
        let t = 1.0 - (value - 0.25) / 0.75;
        t + t + 2.0
    }
}

/// `0x0060f900`: linear progress after one step.
///
/// `progress + 8 * dt`, clamped into `0..=1`.  The upper clamp is `minss`
/// (`1.0 < p ? 1.0 : p`, so NaN survives) followed by the `p > 0.0` guard
/// that turns NaN and negative values into `0.0`.
pub fn a62_ramp_progress(progress: f32, dt: f32) -> f32 {
    let raised = progress + dt * A62_PROGRESS_PER_SECOND;
    let capped = if 1.0 < raised { 1.0 } else { raised };
    if capped > 0.0 { capped } else { 0.0 }
}

/// `0x0060f900`: linear ramp output for an eased fraction.
pub fn a62_linear_value(start: f32, end: f32, eased: f32) -> f32 {
    start + (end - start) * eased
}

/// `0x0060f900` / `0x0060fa46`: the overshoot predicate.
///
/// The shipped tests, in order, are `end > start && value > end`, then
/// `start > end && end > value`.  Any unordered comparison falls through to
/// the non-overshoot path, which plain `>` reproduces.
pub fn a62_overshoot(start: f32, end: f32, value: f32) -> bool {
    (end > start && value > end) || (start > end && end > value)
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// The controller state block at `0x00f04ac0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A62Gamma {
    /// `0x00f04ac0`.
    pub screen: i32,
    /// `0x00f04ac4`.
    pub screen_scaled: i32,
    /// `0x00f04ac8`.
    pub level: i32,
    /// `0x00f04acc`.
    pub scale: f32,
    /// `0x00f04ad0`.
    pub value: f32,
    /// `0x00f04ad4`.
    pub start: f32,
    /// `0x00f04ad8`.
    pub end: f32,
    /// `0x00f04adc`.
    pub active: bool,
    /// `0x00f04ae0`.
    pub progress: f32,
    /// `0x00f04ae8` (`0x00f04aec` is the high half).
    pub last_time: u64,
    /// `0x00f04af0`.
    pub mode: i32,
    /// `0x00f04af4`.
    pub curved: bool,
    /// `0x00f04af8`, consumed as a signed integer by `cvtdq2pd`.
    pub speed: i32,
    /// `0x00f04afc`.
    pub pending_x: f32,
    /// `0x00f04b00`.
    pub pending_y: f32,
    /// `0x00f04b08`.
    pub gain: f32,
}

/// Result of the resolution commit shared by `0x0060f880` and `0x0060fba0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A62Commit {
    /// The shipped guard `screen == screen && scaled == scaled && level == level`
    /// was false, so the update branch ran.
    pub changed: bool,
    pub screen: i32,
    pub screen_scaled: i32,
    pub level: i32,
}

/// Result of the resolution/scaling math of `0x0060fba0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct A62Apply {
    /// `0x00f04ad0` after the call.
    pub value: f32,
    pub screen: i32,
    pub screen_scaled: i32,
    pub level: i32,
    /// The `(u16 << 5)` ceiling clamped the computed count.
    pub clamped: bool,
}

/// What the caller of `0x0060f900` must do next.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum A62Tick {
    /// Inactive, or the elapsed time did not exceed `0.005` s.
    Idle,
    /// Call `0x0060fba0` with this parameter.
    Apply(f32),
    /// Call `0x0060fba0` with this parameter, then the fade-completion reset.
    ApplyAndFinish(f32),
}

/// Decision returned by the `0x0060fa46` tail of the curve ramp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A62FadeTail {
    ApplyOnly,
    ApplyAndFinish,
}

/// What `0x0060fb50` does with the display/resume queries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A62ResumeAction {
    /// The shipped routine falls through without an action.
    Idle,
    /// `FUN_00751800(0, 0, 1)`.
    Resume,
    /// `FUN_00784d50(0xe4)`.
    Initialize,
    /// Zero `0x011b24b8` and tail-jump to the shutdown routine.
    Shutdown,
}

/// Screen metrics read by the placement modes:
/// `0x00fe5b88` (width) and `0x00fe5b8c` (height).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A62Display {
    pub width: u32,
    pub height: u32,
}

/// Query results consumed by mode 2 of `0x0060fc9a`.
///
/// The shipped order is `FUN_006bb610` (client width), `FUN_007b42b0` (screen
/// width), `FUN_006bb620` (client height) and `FUN_007b3e10` (screen height).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A62PlaceHelper {
    pub client_width: i32,
    pub screen_width: i32,
    pub client_height: i32,
    pub screen_height: i32,
}

impl A62Gamma {
    /// The all-zero state the raw globals start in.
    pub const fn a62_new() -> Self {
        A62Gamma {
            screen: 0,
            screen_scaled: 0,
            level: 0,
            scale: 0.0,
            value: 0.0,
            start: 0.0,
            end: 0.0,
            active: false,
            progress: 0.0,
            last_time: 0,
            mode: 0,
            curved: false,
            speed: 0,
            pending_x: 0.0,
            pending_y: 0.0,
            gain: 0.0,
        }
    }

    /// `0x0060f870` (`FUN_0060f870`, 6 bytes): `return *(int *)0x00f04ac0`.
    pub fn a62_screen(&self) -> i32 {
        self.screen
    }

    /// `0x0060f8f0` (`FUN_0060f8f0`, 7 bytes): `return *(float *)0x00f04ad0`
    /// (returned on the x87 stack as `float10`).
    pub fn a62_value(&self) -> f32 {
        self.value
    }

    /// State portion of the call at `0x0060f8e7` / `0x0060fc8d`
    /// (`FUN_0060d940`, just below this batch): recompute
    /// `scaled = (int)roundf(level * scale)` and commit the triple only when
    /// it differs from the stored one.  The positive-value guards are part of
    /// the shipped branch.
    ///
    /// The three refresh calls the shipped routine makes afterwards
    /// (`FUN_00714650`, `FUN_00681cc0`, `FUN_0060d7b0`) are outside this window and
    /// have no state effect on this block, so they are not reproduced.
    pub fn a62_commit_resolution(&mut self, screen: i32, level: i32) -> A62Commit {
        let scaled = a62_round_i32(level as f32 * self.scale);
        let changed =
            !(screen == self.screen && scaled == self.screen_scaled && level == self.level);
        if changed {
            if screen > 0 {
                self.screen = screen;
            }
            if level > 0 {
                self.level = level;
                self.screen_scaled = scaled;
            }
        }
        A62Commit {
            changed,
            screen: self.screen,
            screen_scaled: self.screen_scaled,
            level: self.level,
        }
    }

    /// `0x0060f880` (`FUN_0060f880`, 65 bytes): one-time initialisation plus
    /// the level-scale commit.
    ///
    /// With `already_initialized == false` the body stores
    /// `screen = helper_a`, `screen_scaled = 0`, `level = helper_b`,
    /// sets `value`, `start` and `end` to `1.0`, and `mode` to `2`.  With the
    /// flag set it reloads the stored `screen`/`level` and ignores both helper
    /// values.  Both paths then write `scale = 0.79799998` and commit
    /// `(screen, level)`.
    ///
    /// The ledger's decompilation marks `FUN_007b3e10` as noreturn; the shipped
    /// epilogue at `0x0060f8ef` is a plain `ret`, and the value it returns is
    /// stored in `0x00f04ac8`.
    pub fn a62_initialize(
        &mut self,
        already_initialized: bool,
        screen: i32,
        level: i32,
    ) -> A62Commit {
        if !already_initialized {
            self.screen = screen;
            self.screen_scaled = 0;
            self.level = level;
            self.value = 1.0;
            self.start = 1.0;
            self.end = 1.0;
            self.mode = 2;
        }
        self.scale = A62_LEVEL_SCALE;
        self.a62_commit_resolution(self.screen, self.level)
    }

    /// `0x0060ff50` (`FUN_0060ff50`, 91 bytes): begin a curve-rate ramp.
    ///
    /// The shipped body writes, in this order: `speed = arg0`, `curved = 1`,
    /// `last_time = FUN_0093f7b0()`, `active = 1`, `end = curve(arg1)`,
    /// `start = value`, `mode = arg2`.  The shipped routine evaluates
    /// `curve(arg1)` before storing the end value, so `curved_target` is that
    /// already evaluated result and the raw `arg1` is not needed here.
    pub fn a62_start_fade(&mut self, speed: i32, curved_target: f32, mode: i32, now: u64) {
        self.speed = speed;
        self.curved = true;
        self.last_time = now;
        self.active = true;
        self.end = curved_target;
        self.start = self.value;
        self.mode = mode;
    }

    /// `0x0060fba0` (`FUN_0060fba0`, 250 bytes): store the curved value and
    /// derive the `(screen, level)` pair from it.
    ///
    /// The shipped arithmetic is
    ///
    /// ```text
    /// value = curve(param)                       // `curved` argument
    /// f     = roundf(extent * (1.0 / value))     // reciprocal, not divide
    /// max   = (u16 at ((0x00f063cc ^ 0x10fae560) + 0xe4)) << 5
    /// if f > max { value = extent / max; f = max }
    /// x       = f / aspect                       // FUN_007b3dd0 = width/height
    /// screen  = (int)x                           // cvttss2si
    /// level   = (int)roundf(x)
    /// commit(screen, level)
    /// ```
    ///
    /// `curved` is the output of the external `FUN_0060c0e0` applied to the raw
    /// ramp parameter, which is therefore not a separate argument here.
    /// `aspect` is the `FUN_007b3dd0` result (screen width / screen height) and
    /// `extent` is the
    /// `FUN_007b42b0` result, called twice by the shipped clamp path for the same
    /// value.  The dead `roundf` call at `0x0060fc62` (its result is overwritten
    /// by the following `flds`) is not reproduced.
    ///
    /// The tail at `0x0060fc9a` is a separate ledger row and is translated as
    /// [`A62Gamma::a62_place`]; the caller passes the pre-call
    /// `screen`/`screen_scaled` to it.
    pub fn a62_apply(&mut self, curved: f32, aspect: f32, extent: i32, max_units: u16) -> A62Apply {
        self.value = curved;
        // The shipped code forms `1.0f / value` first and multiplies the
        // extent by it, which differs from a direct division for values such
        // as 1.2f where the product rounds to an exact half.
        let mut f = a62_round_f32(extent as f32 * (1.0 / curved));
        let max = ((max_units as u32) << 5) as f32;
        let mut clamped = false;
        if f > max {
            self.value = extent as f32 / max;
            f = max;
            clamped = true;
        }
        let x = f / aspect;
        let screen = a62_trunc_i32(x);
        let level = a62_round_i32(x);
        let commit = self.a62_commit_resolution(screen, level);
        A62Apply {
            value: self.value,
            screen: commit.screen,
            screen_scaled: commit.screen_scaled,
            level: commit.level,
            clamped,
        }
    }

    /// `0x0060fc9a` (`FUN_0060fc9a`, 311 bytes): placement mode dispatch.
    ///
    /// Ghidra starts this row at the `cmp $1, %eax` branch target inside
    /// `0x0060fba0`, so the mode arrives in `EAX` rather than in ECX.  The
    /// three modes use the pre-apply `(screen, screen_scaled)` pair, the screen
    /// metrics, and mode 2's pending sub-pixel offsets.  Return value is the
    /// `(x, y)` pair passed to `FUN_00681ad0`, or `None` for mode 2's early
    /// return and for unknown modes.
    pub fn a62_place(
        &mut self,
        mode: i32,
        old_screen: i32,
        old_scaled: i32,
        display: A62Display,
        helper: A62PlaceHelper,
    ) -> Option<(i32, i32)> {
        match mode {
            1 => {
                let dx = (self.screen - old_screen) as f32 * 0.5;
                let dy = (self.screen_scaled - old_scaled) as f32 * 0.5;
                let x = a62_trunc_i32(display.width as f32 - a62_round_f32(dx));
                let y = a62_trunc_i32(display.height as f32 - a62_round_f32(dy));
                Some((x, y))
            }
            2 => {
                let ratio_x = helper.client_width as f32 / helper.screen_width as f32;
                let ratio_y =
                    helper.client_height as f32 / (helper.screen_height as f32 * A62_LEVEL_SCALE);
                let dx = (self.screen - old_screen) as f32 * ratio_x + self.pending_x;
                let dy = (self.screen_scaled - old_scaled) as f32 * ratio_y + self.pending_y;
                if dx.abs() < 1.0 && dy.abs() < 1.0 {
                    self.pending_x = dx;
                    self.pending_y = dy;
                    return None;
                }
                let mut x = display.width as i32;
                let mut y = display.height as i32;
                if dx.abs() > 1.0 {
                    let step = a62_round_f32(dx);
                    x -= a62_trunc_i32(step);
                    self.pending_x = dx - step;
                } else {
                    self.pending_x = dx;
                }
                if dy.abs() > 1.0 {
                    let step = a62_round_f32(dy);
                    y -= a62_trunc_i32(step);
                    self.pending_y = dy - step;
                } else {
                    self.pending_y = dy;
                }
                Some((x, y))
            }
            3 => Some((
                display.width as i32 - (self.screen - old_screen) / 2,
                display.height as i32,
            )),
            _ => None,
        }
    }

    /// `0x0060f900` (`FUN_0060f900`, 457 bytes): advance the active ramp.
    ///
    /// Steps whose elapsed time is `<= 0.005` s are ignored without touching
    /// `last_time`.  Longer steps are clamped to `0.1` s and stored.
    ///
    /// * Linear mode (`curved == false`) advances `progress` by
    ///   `8 * dt`, clamps it into `0..=1`, and produces
    ///   `start + (end - start) * eased`; it finishes when `progress >= 1`.
    /// * Curve mode updates
    ///   `value += (float)((speed * dt) * gain) / denominator(value)`, then asks
    ///   the external curve for the new value.  Overshoot (see
    ///   [`a62_overshoot`]) applies the end value and finishes; otherwise the
    ///   curved value is applied and the fade finishes when the caller's
    ///   `previous` value is within `2^-22` of it.
    ///
    /// `curve` stands for the external `FUN_0060c0e0`.  `previous` is the scratch
    /// slot reloaded at `0x0060fa81`: the shipped function reads an
    /// uninitialised local there (Ghidra's `in_stack_0000e87b`), so the value is
    /// supplied explicitly instead of being read from an undefined stack slot.
    /// `eased` is the `FUN_00c94230` output for the linear path.
    pub fn a62_tick<F>(&mut self, now: u64, previous: f32, curve: F, eased: f32) -> A62Tick
    where
        F: Fn(f32) -> f32,
    {
        if !self.active {
            return A62Tick::Idle;
        }
        let mut dt = a62_step_seconds(now, self.last_time);
        if !(dt > A62_STEP_FLOOR) {
            return A62Tick::Idle;
        }
        if dt > A62_STEP_CEIL {
            dt = A62_STEP_CEIL;
        }
        self.last_time = now;
        let dt = dt as f32;

        if !self.curved {
            self.progress = a62_ramp_progress(self.progress, dt);
            let value = a62_linear_value(self.start, self.end, eased);
            if self.progress >= 1.0 {
                return A62Tick::ApplyAndFinish(value);
            }
            return A62Tick::Apply(value);
        }

        let step = ((self.speed as f64) * (dt as f64) * (self.gain as f64)) as f32;
        let current = self.value;
        let value = current + step / a62_curve_denominator(current);
        self.value = value;
        let curved = curve(value);
        if self.active && a62_overshoot(self.start, self.end, curved) {
            return A62Tick::ApplyAndFinish(self.end);
        }
        if (previous - curved).abs() < A62_EPSILON {
            return A62Tick::ApplyAndFinish(curved);
        }
        A62Tick::Apply(curved)
    }
}

/// `0x0060fa46` (`FUN_0060fa46`, 121 bytes): tail of the curve ramp's
/// overshoot test.
///
/// The row starts on the `ja` of the `value > end` test, so the decompiler
/// renders the incoming flags as `in_CF`/`in_ZF`.  With `start`, `end` and
/// `value` the branch is exactly [`a62_overshoot`]: overshoot applies the end
/// value and finishes, everything else applies the curved value and only
/// finishes when the epsilon test also passes.
pub fn a62_fade_tail(start: f32, end: f32, value: f32) -> A62FadeTail {
    if a62_overshoot(start, end, value) {
        A62FadeTail::ApplyAndFinish
    } else {
        A62FadeTail::ApplyOnly
    }
}

/// `0x0060fb50` (`FUN_0060fb50`, 67 bytes): decide what the resume path does.
///
/// The shipped body, in order:
///
/// 1. if `FUN_0079cc40()` is true, zero `0x011b24b8` and tail-jump to the
///    shutdown routine;
/// 2. else if `FUN_007517d0()` is non-zero, call `FUN_00751800(0, 0, 1)`;
/// 3. else if `0x011a4580 == 0` and `FUN_00748d60() == 0`, call
///    `FUN_00784d50(0xe4)`.
///
/// `runtime_ready` is the `FUN_00748d60() == 0` result.
pub fn a62_resume_action(
    display_gone: bool,
    resume_request: i32,
    state_flag: i32,
    runtime_ready: bool,
) -> A62ResumeAction {
    if display_gone {
        return A62ResumeAction::Shutdown;
    }
    if resume_request != 0 {
        return A62ResumeAction::Resume;
    }
    if state_flag == 0 && runtime_ready {
        return A62ResumeAction::Initialize;
    }
    A62ResumeAction::Idle
}

// ---------------------------------------------------------------------------
// Small reference-counted owner used by three rows
// ---------------------------------------------------------------------------

/// The `this` object of `0x0060ffb0`, `0x0060ffd0` and `0x0060ffe0`: a count
/// at `+0x00`, a payload at `+0x08` and a signed flag at `+0x10`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A62RefCount {
    /// `+0x00`.
    pub count: i32,
    /// `+0x08`.
    pub payload: i32,
    /// `+0x10`.
    pub flag10: i32,
}

impl A62RefCount {
    /// `0x0060ffd0` (`FUN_0060ffd0`, 9 bytes): `this->count = 0; return this`.
    ///
    /// Returns `self` to mirror the shipped `mov %ecx, %eax`.
    pub fn a62_clear(&mut self) -> &mut Self {
        self.count = 0;
        self
    }

    /// `0x0060ffb0` (`FUN_0060ffb0`, 27 bytes, `ret 4`): clear the count and
    /// forward one argument to `FUN_00610410` (next batch).
    ///
    /// The forwarded request is returned so the call stays observable.
    pub fn a62_clear_and_forward(&mut self, request: u32) -> u32 {
        self.count = 0;
        request
    }

    /// `0x0060ffe0` (`FUN_0060ffe0`, 84 bytes): decrement-style release.
    ///
    /// The shipped code computes `n = count - 1`; when `n == 0` and
    /// `flag10 >= 0` it runs the diagnostic path (frame cookie `0xcbd380`,
    /// report through `FUN_0093ec70`, then a virtual call with the payload) and
    /// otherwise skips it.  In both cases `count` ends at zero.  `Some(payload)`
    /// reports the diagnostic path; the count write is always performed.
    ///
    /// Note the shipped `sub $1` result is only compared with zero, so the new
    /// count is not the decremented value.
    pub fn a62_release(&mut self) -> Option<i32> {
        let next = self.count.wrapping_sub(1);
        let report = if next == 0 && self.flag10 >= 0 {
            Some(self.payload)
        } else {
            None
        };
        self.count = 0;
        report
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_agent62 {
    use super::*;

    const MS: u64 = 1_000_000;

    fn gamma_at(value: f32) -> A62Gamma {
        let mut state = A62Gamma::a62_new();
        state.value = value;
        state.start = value;
        state.end = value;
        state.active = true;
        state
    }

    #[test]
    fn getters_read_the_state_block() {
        // 0x0060f870 and 0x0060f8f0.
        let mut state = A62Gamma::a62_new();
        assert_eq!(state.a62_screen(), 0);
        assert_eq!(state.a62_value().to_bits(), 0);

        state.screen = 0x1234;
        state.value = 1.25;
        assert_eq!(state.a62_screen(), 0x1234);
        assert_eq!(state.a62_value().to_bits(), 1.25f32.to_bits());
    }

    #[test]
    fn initialize_from_scratch_sets_defaults_and_commits() {
        // 0x0060f880: fresh path.
        let mut state = A62Gamma::a62_new();
        let commit = state.a62_initialize(false, 1000, 600);

        assert_eq!(state.screen, 1000);
        assert_eq!(state.level, 600);
        assert_eq!(state.screen_scaled, a62_round_i32(600.0 * A62_LEVEL_SCALE));
        assert_eq!(state.value.to_bits(), 1.0f32.to_bits());
        assert_eq!(state.start.to_bits(), 1.0f32.to_bits());
        assert_eq!(state.end.to_bits(), 1.0f32.to_bits());
        assert_eq!(state.mode, 2);
        assert_eq!(state.scale.to_bits(), A62_LEVEL_SCALE.to_bits());
        assert!(commit.changed);
        assert_eq!(commit.screen_scaled, state.screen_scaled);
    }

    #[test]
    fn initialize_second_call_ignores_the_new_extents() {
        // 0x0060f880: already-initialised path reloads the stored pair.
        let mut state = A62Gamma::a62_new();
        state.a62_initialize(false, 800, 600);
        state.value = 0.75;
        state.mode = 3;

        let commit = state.a62_initialize(true, 1920, 1080);
        assert_eq!(state.screen, 800);
        assert_eq!(state.level, 600);
        assert_eq!(state.value.to_bits(), 0.75f32.to_bits());
        assert_eq!(state.mode, 3);
        assert!(!commit.changed);
        assert_eq!(state.scale.to_bits(), A62_LEVEL_SCALE.to_bits());
    }

    #[test]
    fn commit_clamps_the_scaled_level_only_for_positive_levels() {
        let mut state = A62Gamma::a62_new();
        state.scale = A62_LEVEL_SCALE;

        // level = 0 leaves the stored scaled value untouched and is "changed"
        // because screen 0 is the same but level 0 differs?  It is equal, so
        // the guard short-circuits.
        let commit = state.a62_commit_resolution(0, 0);
        assert!(!commit.changed);
        assert_eq!(state.screen_scaled, 0);

        let commit = state.a62_commit_resolution(640, 480);
        assert!(commit.changed);
        assert_eq!(state.screen, 640);
        assert_eq!(state.level, 480);
        assert_eq!(state.screen_scaled, a62_round_i32(480.0 * A62_LEVEL_SCALE));

        // Non-positive extents are not stored.
        let commit = state.a62_commit_resolution(-1, -1);
        assert!(commit.changed);
        assert_eq!(state.screen, 640);
        assert_eq!(state.level, 480);
    }

    #[test]
    fn step_seconds_is_signed_64_bit_over_one_billion() {
        // 0x0060f900 time base.
        assert_eq!(a62_step_seconds(1_500 * MS, 500 * MS), 1.0);
        assert_eq!(a62_step_seconds(2_000 * MS, 1_000 * MS), 1.0);
        // now < last wraps the subtraction and remains negative.
        assert_eq!(a62_step_seconds(0, 1), -1.0e-9);
        // The high half participates: 1 << 32 ns is not truncated.
        assert_eq!(a62_step_seconds(1u64 << 32, 0), 4.294967296);
    }

    #[test]
    fn curve_denominator_switches_at_one() {
        // 0x0060f900 curve-rate branch.
        assert_eq!(a62_curve_denominator(1.0).to_bits(), 2.0f32.to_bits());
        assert_eq!(a62_curve_denominator(2.0).to_bits(), 0.25f32.to_bits());
        assert_eq!(a62_curve_denominator(0.25).to_bits(), 4.0f32.to_bits());
        // Below 1.0: t = 1 - (0 - 0.25)/0.75 = 1.3333; 2t + 2 = 4.6666.
        let expected = {
            let t = 1.0f32 - (0.0f32 - 0.25) / 0.75;
            t + t + 2.0
        };
        assert_eq!(a62_curve_denominator(0.0).to_bits(), expected.to_bits());
    }

    #[test]
    fn ramp_progress_accumulates_and_clamps() {
        // 0x0060f900 linear branch.
        assert_eq!(
            a62_ramp_progress(0.0, 0.1).to_bits(),
            (0.1f32 * 8.0).to_bits()
        );
        assert_eq!(a62_ramp_progress(0.9, 0.1), 1.0);
        assert_eq!(a62_ramp_progress(0.0, -1.0), 0.0);
        // `minss` keeps a NaN produced by the addition, then the sign guard
        // makes it zero.
        assert_eq!(a62_ramp_progress(f32::NAN, 0.0), 0.0);
    }

    #[test]
    fn linear_tick_from_0_8_to_1_0() {
        // 0x0060f900 linear mode.
        let mut state = A62Gamma::a62_new();
        state.active = true;
        state.curved = false;
        state.start = 0.5;
        state.end = 1.0;
        state.progress = 0.0;
        state.last_time = 0;

        let first = state.a62_tick(100 * MS, 0.0, |x| x, 0.5);
        assert_eq!(state.progress.to_bits(), (0.1f32 * 8.0).to_bits());
        assert_eq!(first, A62Tick::Apply(0.5f32 + (1.0f32 - 0.5) * 0.5));

        // Second step: progress clamps at 1.0 and the eased 0.75 is used.
        let second = state.a62_tick(200 * MS, 0.0, |x| x, 0.75);
        assert_eq!(state.progress, 1.0);
        assert_eq!(
            second,
            A62Tick::ApplyAndFinish(0.5f32 + (1.0f32 - 0.5) * 0.75)
        );
    }

    #[test]
    fn tick_ignores_short_and_inactive_steps() {
        // 0x0060f900 gate.
        let mut state = gamma_at(1.0);
        state.curved = true;
        state.last_time = 5;
        let idle = state.a62_tick(5 + 4 * MS, 0.0, |x| x, 0.0);
        assert_eq!(idle, A62Tick::Idle);
        assert_eq!(state.last_time, 5);

        state.active = false;
        let idle = state.a62_tick(5 + 100 * MS, 0.0, |x| x, 0.0);
        assert_eq!(idle, A62Tick::Idle);
        assert_eq!(state.last_time, 5);
    }

    #[test]
    fn tick_clamps_the_elapsed_time() {
        // 0x0060f900: one second becomes 0.1.
        let mut state = gamma_at(1.0);
        state.curved = true;
        state.speed = 1;
        state.gain = 1.0;
        state.last_time = 0;

        let tick = state.a62_tick(1_000 * MS, 0.0, |x| x, 0.0);
        let expected = 1.0f32 + ((0.1f32 as f64) as f32) / 2.0;
        assert_eq!(state.last_time, 1_000 * MS);
        assert_eq!(tick, A62Tick::Apply(expected));
        assert_eq!(state.value.to_bits(), expected.to_bits());
    }

    #[test]
    fn curve_tick_stops_within_epsilon() {
        // 0x0060f900 epsilon tail, also 0x0060fa46.
        let mut state = gamma_at(1.0);
        state.curved = true;
        state.speed = 1;
        state.gain = 1.0;
        state.last_time = 0;
        let expected = 1.05f32;

        let tick = state.a62_tick(100 * MS, expected, |x| x, 0.0);
        assert_eq!(state.value.to_bits(), expected.to_bits());
        assert_eq!(tick, A62Tick::ApplyAndFinish(expected));
        assert_eq!(a62_fade_tail(1.0, 2.0, 1.05), A62FadeTail::ApplyOnly);
        assert_eq!(a62_fade_tail(1.0, 2.0, 2.05), A62FadeTail::ApplyAndFinish);
    }

    #[test]
    fn curve_tick_detects_overshoot_and_applies_the_end_value() {
        // 0x0060f900 overshoot branch.
        let mut state = A62Gamma::a62_new();
        state.active = true;
        state.curved = true;
        state.start = 1.0;
        state.end = 0.5;
        state.value = 0.47;
        state.speed = 1;
        state.gain = 1.0;
        state.last_time = 0;

        let tick = state.a62_tick(100 * MS, 0.0, |x| x, 0.0);
        assert!(state.value < 0.5);
        assert_eq!(tick, A62Tick::ApplyAndFinish(0.5));
        assert!(a62_overshoot(1.0, 0.5, state.value));
        assert!(!a62_overshoot(0.5, 1.0, state.value));
    }

    #[test]
    fn overshoot_predicate_matches_the_two_shipped_tests() {
        // 0x0060f900 / 0x0060fa46 predicate.
        assert!(a62_overshoot(1.0, 2.0, 2.5));
        assert!(!a62_overshoot(1.0, 2.0, 2.0));
        assert!(!a62_overshoot(1.0, 2.0, 1.5));
        assert!(a62_overshoot(2.0, 1.0, 0.5));
        assert!(!a62_overshoot(2.0, 1.0, 1.0));
        assert!(!a62_overshoot(2.0, 1.0, 1.5));
        // Equal ends never overshoot.
        assert!(!a62_overshoot(1.0, 1.0, 2.0));
        assert!(!a62_overshoot(1.0, 1.0, 0.0));
    }

    #[test]
    fn start_fade_uses_the_curved_target_and_current_value() {
        // 0x0060ff50.
        let mut state = A62Gamma::a62_new();
        state.value = 0.25;
        state.a62_start_fade(3, 0.75, 2, 42);

        assert_eq!(state.speed, 3);
        assert!(state.curved);
        assert!(state.active);
        assert_eq!(state.last_time, 42);
        assert_eq!(state.end.to_bits(), 0.75f32.to_bits());
        assert_eq!(state.start.to_bits(), 0.25f32.to_bits());
        assert_eq!(state.mode, 2);
    }

    #[test]
    fn apply_rounds_screen_and_level_differently() {
        // 0x0060fba0: f = 500, aspect 3.0 -> 166.666..,
        // screen = trunc -> 166, level = round -> 167.
        let mut state = A62Gamma::a62_new();
        state.scale = A62_LEVEL_SCALE;
        let applied = state.a62_apply(2.0, 3.0, 1000, 0x20);

        assert!(!applied.clamped);
        assert_eq!(applied.value.to_bits(), 2.0f32.to_bits());
        assert_eq!(applied.screen, 166);
        assert_eq!(applied.level, 167);
        assert_eq!(
            applied.screen_scaled,
            a62_round_i32(167.0 * A62_LEVEL_SCALE)
        );
        assert_eq!(state.value.to_bits(), 2.0f32.to_bits());
    }

    #[test]
    fn apply_uses_the_reciprocal_multiply() {
        // 0x0060fba0: 9 * (1 / 1.2f) rounds to 7.5 and `roundf` gives 8,
        // while 9 / 1.2f gives 7.4999995 and rounds to 7.
        let mut state = A62Gamma::a62_new();
        state.scale = A62_LEVEL_SCALE;
        let applied = state.a62_apply(1.2, 1.0, 9, 0x20);
        assert_eq!(applied.screen, 8);
        assert_eq!(applied.level, 8);
    }

    #[test]
    fn apply_clamps_against_the_shifted_word_limit() {
        // 0x0060fba0 clamp: max = 0x20 << 5 = 1024.
        let mut state = A62Gamma::a62_new();
        state.scale = A62_LEVEL_SCALE;
        let applied = state.a62_apply(0.5, 1.0, 1000, 0x20);

        assert!(applied.clamped);
        assert_eq!(applied.value.to_bits(), (1000.0f32 / 1024.0).to_bits());
        // The clamp replaces the count with the ceiling itself, so the pair is
        // derived from 1024 / 1.0 rather than from 1000 / 1.0.
        assert_eq!(applied.screen, 1024);
        assert_eq!(applied.level, 1024);
        assert_eq!(
            applied.screen_scaled,
            a62_round_i32(1024.0 * A62_LEVEL_SCALE)
        );
    }

    #[test]
    fn place_mode_one_centres_half_the_delta() {
        // 0x0060fc9a mode 1.
        let mut state = A62Gamma::a62_new();
        state.screen = 1000;
        state.screen_scaled = 700;
        let display = A62Display {
            width: 1024,
            height: 768,
        };
        let helper = A62PlaceHelper {
            client_width: 0,
            screen_width: 0,
            client_height: 0,
            screen_height: 0,
        };
        let position = state.a62_place(1, 800, 600, display, helper);
        assert_eq!(position, Some((924, 718)));
    }

    #[test]
    fn place_mode_two_accumulates_sub_pixel_offsets() {
        // 0x0060fc9a mode 2.
        let mut state = A62Gamma::a62_new();
        state.screen = 1000;
        state.screen_scaled = 600;
        let display = A62Display {
            width: 1024,
            height: 768,
        };
        let helper = A62PlaceHelper {
            client_width: 500,
            screen_width: 1000,
            client_height: 300,
            screen_height: 500,
        };

        let position = state.a62_place(2, 800, 400, display, helper);
        assert_eq!(position, Some((924, 618)));
        assert_eq!(state.pending_x, 0.0);
        let ratio_y = 300.0f32 / (500.0f32 * A62_LEVEL_SCALE);
        let expected_pending = 200.0f32 * ratio_y - a62_round_f32(200.0f32 * ratio_y);
        assert_eq!(state.pending_y.to_bits(), expected_pending.to_bits());

        // A sub-pixel delta is only accumulated.
        state.screen = 800;
        state.screen_scaled = 400;
        state.pending_x = 0.25;
        state.pending_y = 0.0;
        state.screen_scaled = 400;
        let position = state.a62_place(2, 800, 400, display, helper);
        assert_eq!(position, None);
        assert_eq!(state.pending_x, 0.25);
        assert_eq!(state.pending_y, 0.0);
    }

    #[test]
    fn place_mode_three_uses_the_truncated_half_delta() {
        // 0x0060fc9a mode 3.
        let mut state = A62Gamma::a62_new();
        state.screen = 1001;
        let display = A62Display {
            width: 1024,
            height: 768,
        };
        let helper = A62PlaceHelper {
            client_width: 0,
            screen_width: 0,
            client_height: 0,
            screen_height: 0,
        };
        // (1001 - 800) / 2 = 100 (truncating).
        let position = state.a62_place(3, 800, 0, display, helper);
        assert_eq!(position, Some((924, 768)));
        // Unknown modes do nothing.
        assert_eq!(state.a62_place(7, 800, 0, display, helper), None);
    }

    #[test]
    fn resume_action_precedence() {
        // 0x0060fb50.
        assert_eq!(
            a62_resume_action(true, 1, 0, true),
            A62ResumeAction::Shutdown
        );
        assert_eq!(
            a62_resume_action(false, 1, 0, true),
            A62ResumeAction::Resume
        );
        assert_eq!(
            a62_resume_action(false, 0, 0, true),
            A62ResumeAction::Initialize
        );
        assert_eq!(a62_resume_action(false, 0, 1, true), A62ResumeAction::Idle);
        assert_eq!(a62_resume_action(false, 0, 0, false), A62ResumeAction::Idle);
    }

    #[test]
    fn refcount_clear_and_forward() {
        // 0x0060ffd0 and 0x0060ffb0.
        let mut owner = A62RefCount {
            count: 5,
            payload: 7,
            flag10: -1,
        };
        assert_eq!(owner.a62_clear().count, 0);
        owner.count = 9;
        assert_eq!(owner.a62_clear_and_forward(0xabc), 0xabc);
        assert_eq!(owner.count, 0);
    }

    #[test]
    fn refcount_release_reports_the_diagnostic_path() {
        // 0x0060ffe0.
        let mut owner = A62RefCount {
            count: 2,
            payload: 7,
            flag10: 0,
        };
        assert_eq!(owner.a62_release(), None);
        assert_eq!(owner.count, 0);

        let mut owner = A62RefCount {
            count: 1,
            payload: 7,
            flag10: 0,
        };
        assert_eq!(owner.a62_release(), Some(7));
        assert_eq!(owner.count, 0);

        // A negative flag suppresses the report but the count still clears.
        let mut owner = A62RefCount {
            count: 1,
            payload: 7,
            flag10: -1,
        };
        assert_eq!(owner.a62_release(), None);
        assert_eq!(owner.count, 0);
    }
}

// ---------------------------------------------------------------------------
// Batch skip notes
// ---------------------------------------------------------------------------
//
// The window contains 17 `decompiled_uncited` rows.  Twelve have coherent
// bodies and are translated above.  The other five are fragments of two
// obfuscated blobs and cannot be given a function contract from the shipped
// image:
//
// * FUN_0060ed26 (1198 bytes, `FUN_0060ed26..0x0060f1d4`): the decompiler output
//   addresses stack slots relative to an uninitialised `EBP`
//   (`0x9a6363xx`), interleaves `rdtsc` opaque predicates, overlapping
//   instructions and self-modifying jumps, and ends in
//   `WARNING: Bad instruction - Truncating control flow here`.  The
//   disassembly shows no prologue and no consistent return.  Not translated.
// * FUN_0060f144 (7 bytes): exactly `js $` (`0x78 0xfe`, a self-loop) followed
//   by the middle of the obfuscated stream; there is no prologue, argument
//   use or return.
// * FUN_0060f1e2 (33 bytes): starts mid-instruction (`inc edx; jo ...`); the
//   decompiler's "infinite loop" and `thunk_FUN_00538810` tail are artifacts
//   of the bad entry point.
// * FUN_0060f445 (9 bytes): starts on a `jp` that belongs to the body of
//   FUN_0060f4dd; the decompiler reports overlapping instructions at
//   `0x0060f779`/`0x0060f77b`.
// * FUN_0060f4dd (957 bytes): continuation of the same obfuscated blob; its
//   recovered body ends in an indirect call through a computed address and
//   `swi(0x29)` stack-cookie checks, so there is no usable entry/return
//   contract.
//
// Three further rows in the window (`FUN_0060e0dd`, `FUN_0060e117`, `FUN_0060e17b`)
// are `decompile_failed`; the cache only holds the failure banner, and their
// disassembly starts inside `FUN_0060de60` (`FUN_0060e0dd` begins in the middle of
// that routine's body, with its frame and loop counters already live).  They
// are outside the requested `decompiled_uncited` status.
