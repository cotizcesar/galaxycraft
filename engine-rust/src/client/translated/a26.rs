#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent26.rs
// ===========================================================================
// Translation batch 26.  Only this file is edited by the batch.
//
// Coverage: the `decompiled_uncited` rows of `0x005aa000..=0x005abfff`.
//
// The window belongs to the classic client's briefing / dialog layer.  Most
// of its ledger rows are *not* function entries: retail keeps inline
// `switch` jump tables in this region, so the recorded addresses land either
// inside an instruction or inside the table itself.  Every translation below
// names the exact address, states what the bytes at that address do and
// which fields of the dialog object the arm writes.  That object is not a
// `CUnit`, so the written values are returned to the caller instead of being
// added to `World`/`SimUnit`.
//
// Recovered dialog-object layout used by the comments: `+0x4e` state byte,
// `+0x58`/`+0x5a` u16 position pair, `+0x5c` live position dword, `+0x74`
// cue argument, `+0xc4`/`+0xc8` saved position, `+0xd3` flag byte and
// `+0xd4` intrusive-list head.
//
// Three rows are pure jump-table data and are deliberately skipped:
// `FUN_005ab7ef`, `FUN_005ab809` and `FUN_005ab8fc`.  The dwords there are
// `0x005a71ff`, `0x005a721c`, `0x005a7236`, ... targets of the
// `0x005a6xxx..=0x005a83xx` handlers, so there is no behaviour to translate.
//
// Retail's failure branches hand control to helpers (`FUN_0058b640`,
// `FUN_0058b650`, `FUN_005b4cd0`) and to the shared epilogue `0x005b8040`; the
// translations report those exits through [`A26Exit`] instead of
// re-implementing the helpers.

// ---------------------------------------------------------------------------
// Shared arithmetic arms
// ---------------------------------------------------------------------------

/// Selector `0` subtracts, `1` adds, `2` XORs.  Any other selector leaves the
/// accumulator untouched, which is what the recovered `jne` chains do when
/// they fall through both `cmp` tests.
fn a26_apply(selector: u32, acc: u32, operand: u32) -> u32 {
    match selector {
        0 => acc.wrapping_sub(operand),
        1 => acc.wrapping_add(operand),
        2 => acc ^ operand,
        _ => acc,
    }
}

/// The mirrored form used by the `+h` step of the recovered chains: selector
/// `0` adds and `1` subtracts.
fn a26_apply_plus(selector: u32, acc: u32, operand: u32) -> u32 {
    match selector {
        0 => acc.wrapping_add(operand),
        1 => acc.wrapping_sub(operand),
        2 => acc ^ operand,
        _ => acc,
    }
}

/// The `+y` seed of the three side chains: selector `0` keeps the value, `1`
/// negates it, `2` keeps it (retail computes `0 ^ y`, which is `y`) and every
/// other selector yields `0`.
fn a26_signed(selector: u32, value: u32) -> u32 {
    match selector {
        0 | 2 => value,
        1 => value.wrapping_neg(),
        _ => 0,
    }
}

impl SimUnit {
    /// `0x005aa074` (`FUN_005aa074`, ledger size 28): subtract arm.
    ///
    /// ```asm
    /// 005aa074  sub eax, edi
    /// 005aa076  jmp 005aa08b
    /// 005aa078  cmp ecx, 2      ; alternate entry, not this one
    /// 005aa07d  xor eax, edi
    /// ```
    ///
    /// The arm is entered when the selector is `0`, so the only operation it
    /// performs is the wrapping 32-bit subtraction `acc - operand`; the
    /// following block spills the result to `/Od` temporaries and keeps it.
    pub fn a26_arith_sub(acc: u32, operand: u32) -> u32 {
        acc.wrapping_sub(operand)
    }

    /// `0x005aa0a3` (`FUN_005aa0a3`, ledger size 32): three-way dispatch.
    ///
    /// ```asm
    /// 005aa0a3  mov  [ebp-0xb0], eax
    /// 005aa0ab  mov  eax, [ebp-0xb0]
    /// 005aa0b1  test ecx, ecx
    /// 005aa0b3  jne  005aa0b7
    /// 005aa0b5  sub  eax, edi
    /// 005aa0b7  cmp  ecx, 1
    /// 005aa0ba  jne  005aa0c0
    /// 005aa0bc  add  eax, edi
    /// 005aa0c0  cmp  ecx, 2          ; selector 2 continues into 0x005aa0c3
    /// ```
    ///
    /// The address is a jump-table target inside the window's `/Od` expression
    /// evaluator.  `ecx` is the selector, `eax` the accumulator and `edi` the
    /// operand.  Selector `0` subtracts, `1` adds and `2` XORs the operand;
    /// anything else leaves the accumulator unchanged.
    pub fn a26_arith_apply(selector: u32, acc: u32, operand: u32) -> u32 {
        a26_apply(selector, acc, operand)
    }

    /// `0x005aa0c3` (`FUN_005aa0c3`, ledger size 291): the XOR arm plus two
    /// more dispatches and the table load that follows them.
    ///
    /// ```asm
    /// 005aa0c3  jne 005aa0c7
    /// 005aa0c5  xor eax, edi          ; first selector == 2
    /// ...
    /// 005aa103  test ebx, ebx ; jne ; sub eax, esi
    /// 005aa10b  cmp  ebx, 1   ; jne ; add eax, esi
    /// 005aa114  cmp  ebx, 2   ; jne ; xor eax, esi
    /// ...
    /// 005aa151  mov  eax, [ebp+0x14] ; test ; sub/add/xor eax, edx
    /// 005aa1a8  mov  ecx, eax
    /// 005aa1ae  mov  eax, [ebp-0x298]
    /// 005aa1b4  mov  eax, [ecx+eax]   ; table[index]
    /// ```
    ///
    /// This label only carries the XOR half of the first dispatch (the
    /// subtract arm is the previous entry), so the first selector is applied
    /// as `if selector == 2 { acc ^= first }`.  The next two dispatches use
    /// the full three-way form and the result indexes the dword table at
    /// `[ebp-0x298]`; the table contents belong to the caller.
    pub fn a26_arith_index(
        first_selector: u32,
        acc: u32,
        first: u32,
        second_selector: u32,
        second: u32,
        third_selector: u32,
        third: u32,
    ) -> u32 {
        let mut value = if first_selector == 2 {
            acc ^ first
        } else {
            acc
        };
        value = a26_apply(second_selector, value, second);
        a26_apply(third_selector, value, third)
    }

    /// `0x005ab70c` (`FUN_005ab70c`, ledger size 81): the folded arithmetic
    /// of a control check whose tail is `int 0x29`.
    ///
    /// ```asm
    /// 005ab70c  jne 005ab712 ; add eax, edi ; jmp 005ab719
    /// 005ab712  cmp edx, 2   ; jne ; xor eax, edi
    /// 005ab719  mov [ebp-0x100], eax
    /// 005ab725  test ebx, ebx ; jne ; sub eax, esi
    /// 005ab72d  cmp  ebx, 1   ; jne ; add eax, esi
    /// 005ab736  cmp  ebx, 2   ; jne ; xor eax, esi
    /// 005ab749  int  0x29                       ; __fastfail
    /// 005ab74b  mov  ecx, [ebp-0x10]            ; SEH epilogue
    /// ```
    ///
    /// Two selector/operand pairs are folded into one value and the path then
    /// executes `int 0x29` (`__fastfail`), so retail never returns.  The
    /// helper returns the folded value so the recovered arithmetic stays
    /// observable; callers must treat the path as terminal.
    ///
    /// The first dispatch's default arm is the `add` at `0x005ab70e`, hence
    /// the mirrored `+`/`-` table for the first selector.
    pub fn a26_guard_fold(
        first_selector: u32,
        acc: u32,
        first: u32,
        second_selector: u32,
        second: u32,
    ) -> u32 {
        let value = a26_apply_plus(first_selector, acc, first);
        a26_apply(second_selector, value, second)
    }
}

// ---------------------------------------------------------------------------
// 0x005aae8d - bounding block of the dialog object
// ---------------------------------------------------------------------------

/// Selectors used by the four `0x005aae8d` chains.
///
/// The chains are entered with selectors in registers (`esi` for `+0x20`/`w`,
/// `ebx` for `+0x28`/`h` and the `[ebp+0x14]` argument for the `y` pair), so
/// the translation exposes them instead of guessing the shipped values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A26FieldOps {
    /// Selector applied to the `+0x20` operand.
    pub w: u32,
    /// Selector applied to the `+0x28` operand.
    pub h: u32,
    /// Selector applied to the `+0x14` operand.
    pub y: u32,
}

impl SimUnit {
    /// `0x005aae8d` (`FUN_005aae8d`, ledger size 1041): the four recovered
    /// field chains of the dialog object at `[ebp-0x104]`.
    ///
    /// The first chain (`0x005aae8d`..`0x005ab00c`) writes `+0xa0`:
    ///
    /// ```asm
    /// 005aae8d  sub  eax, edi              ; apply(w, x, w)
    /// 005aaec9  add  eax, ecx              ; + h   (ebx == 0)
    /// 005aaf0b  sub  eax, ecx              ; - h   (ebx == 0)
    /// 005aaf5f  sub  eax, edi              ; - w   (esi == 0)
    /// 005aafb6  sub  eax, edx              ; - y   ([ebp+0x14] == 0)
    /// 005ab00c  mov  [eax+0xa0], ecx
    /// ```
    ///
    /// Three further chains write `+0x9c` (store at `0x005ab1ea`), `+0xb0`
    /// (`0x005ab3c8`) and `+0xb8`; each starts with
    /// `a26_signed([ebp+0x14], y)` and then repeats `+w`, `+h`, `-h`, `-w`,
    /// `-y`.  The three chains are instruction-for-instruction identical, so
    /// they always produce the same value; the return array keeps the store
    /// order `[+0x9c, +0xa0, +0xb0, +0xb8]` anyway.
    ///
    /// A consequence worth recording: every step of a side chain is undone by
    /// a later step with the same selector (including the `^` selector, which
    /// cancels itself), so `+0x9c`, `+0xb0` and `+0xb8` are always zero and
    /// the routine effectively clears them and writes `x - 2w - y` into
    /// `+0xa0`.  The chains are transcribed instead of hard-coded so the
    /// recovered shape stays reviewable.
    pub fn a26_rect_fields(x: i32, y: i32, w: i32, h: i32, ops: A26FieldOps) -> [i32; 4] {
        let (x, y, w, h) = (x as u32, y as u32, w as u32, h as u32);

        let mut top = a26_apply(ops.w, x, w);
        top = a26_apply_plus(ops.h, top, h);
        top = a26_apply(ops.h, top, h);
        top = a26_apply(ops.w, top, w);
        top = a26_apply(ops.y, top, y);

        let mut side = a26_signed(ops.y, y);
        side = a26_apply_plus(ops.w, side, w);
        side = a26_apply_plus(ops.h, side, h);
        side = a26_apply(ops.h, side, h);
        side = a26_apply(ops.w, side, w);
        side = a26_apply(ops.y, side, y);

        [side as i32, top as i32, side as i32, side as i32]
    }
}

// ---------------------------------------------------------------------------
// Dialog / briefing object transitions
// ---------------------------------------------------------------------------

/// Argument a recovered dialog cue is queued with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A26CueArg {
    /// The `+0x74` cue slot (`FUN_005b74f0(id, arg)`); the zero case is the
    /// `FUN_005b6ca0(id, 0)` variant.
    Slot(u32),
    /// The live `+0x58`/`+0x5c` pair (`FUN_005b72e0`).
    LivePosition,
    /// The linked dialog handle (`FUN_005b7250`).
    Handle,
    /// Queued with the id alone (`FUN_005b7d00`).
    NoArgument,
}

/// How a recovered dialog tick leaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A26Exit {
    /// Falls out through the routine's own `ret`.
    Return,
    /// Tail-jumps to the shared `0x005b8040` epilogue.
    Finish,
    /// Runs the `FUN_005d9480` disposal and then the shared epilogue.
    Dispose,
}

/// One recovered dialog-tick outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A26Dialog {
    /// Value written back to the `+0x4e` state byte, when the arm writes one.
    pub state: Option<u8>,
    /// `+0x58`/`+0x5c` copied into the saved position `+0xc4`/`+0xc8`.
    pub latch: bool,
    /// Cue queued before the state work (`0x97`), with its argument.
    pub preroll: Option<(u16, A26CueArg)>,
    /// Cue queued by the state arm, with its argument.
    pub cue: Option<(u16, A26CueArg)>,
    /// How the routine leaves.
    pub exit: A26Exit,
}

impl A26Dialog {
    /// No write, no cue, plain `ret`.
    pub const IDLE: Self = Self {
        state: None,
        latch: false,
        preroll: None,
        cue: None,
        exit: A26Exit::Return,
    };
}

/// Selector for the two clamped entry points of the dialog tail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A26DialogDispatch {
    /// The id is above the entry's limit: retail restores the saved position
    /// from `+0xc4`/`+0xc8` and aborts when both halves are zero.
    RestorePosition,
    /// State `0`: latch the live position and advance to state `1`.
    Latch,
    /// State `1`: run the engagement checks and move to state `3`.
    Engage,
    /// State `2`: the already-engaged arm.
    Engaged,
    /// Any other state byte: the dispatcher returns without a write.
    Idle,
}

impl World {
    /// `0x005aba57` (`FUN_005aba57`, ledger size 217): one tick of the
    /// briefing/transmission dialog.
    ///
    /// The recorded address is one byte inside the `push 0x97` immediate of
    /// the routine body (`0x005aba58` is the call target); the routine's
    /// prologue is not a ledger row.  The recovered body is:
    ///
    /// ```asm
    /// 005aba20  push ebx/esi/edi ; mov esi, ecx ; mov edi, [esi+0x5c]
    /// 005aba28  call FUN_005b58c0 ; jne 005abb1f        ; forced cleanup
    /// 005aba37  call FUN_005bb950 ; jne 005aba6b        ; skip the pre-roll
    /// 005aba44  call FUN_005bd4a0(this, 1)
    /// 005aba49  eax = [esi+0x74]
    /// 005aba58  call FUN_005b74f0(0x97, eax) / 005b6ca0(0x97, 0)
    /// 005aba6b  al = [esi+0x4e]
    /// 005aba70  je  005abad2                          ; state 0
    /// 005aba74  cmp al, 3 ; jne ret                   ; other states
    /// ...       state 3: FUN_005bb740, FUN_005ad8f0, FUN_005b5620
    /// 005abad2  state 0: FUN_005ad980 -> cue 0x55 + x/y
    ///                     else FUN_005d8f30 -> latch + state 3
    /// 005abb1f  call FUN_005d9480 ; jmp 0x005b8040      ; disposal
    /// ```
    ///
    /// `forced_cleanup` is the `FUN_005b58c0 != 0` result, `preroll_suppressed`
    /// the `FUN_005bb950 != 0` result (the `0x97` cue is skipped, together with
    /// the `FUN_005bd4a0(this, 1)` call), `cue_slot` the `+0x74` value,
    /// `acquire_ok` the `FUN_005ad980` result, `view_ok` the `FUN_005d8f30`
    /// result, `engage` the `FUN_005bb740` value (`0` returns immediately, `2`
    /// resets the state), `target_ok` the `FUN_005ad8f0` result and `overlay`
    /// the `FUN_005b5620` result.
    #[allow(clippy::too_many_arguments)]
    pub fn a26_transmission_tick(
        state: u8,
        has_link: bool,
        forced_cleanup: bool,
        preroll_suppressed: bool,
        cue_slot: u32,
        acquire_ok: bool,
        view_ok: bool,
        engage: u8,
        target_ok: bool,
        overlay: bool,
    ) -> A26Dialog {
        let mut out = A26Dialog::IDLE;
        if forced_cleanup {
            out.exit = A26Exit::Dispose;
            return out;
        }
        if !preroll_suppressed {
            out.preroll = Some((0x97, A26CueArg::Slot(cue_slot)));
        }
        match state {
            0 => {
                if !has_link {
                    out.exit = A26Exit::Dispose;
                    return out;
                }
                if acquire_ok {
                    out.cue = Some((0x55, A26CueArg::LivePosition));
                    out.exit = A26Exit::Finish;
                    return out;
                }
                if view_ok {
                    out.state = Some(3);
                    out.latch = true;
                }
            }
            3 => {
                if engage == 0 {
                    return out;
                }
                if !has_link || !target_ok {
                    out.exit = A26Exit::Dispose;
                    return out;
                }
                if engage == 2 {
                    out.state = Some(0);
                    return out;
                }
                if overlay {
                    out.cue = Some((0x54, A26CueArg::NoArgument));
                } else {
                    out.cue = Some((0x52, A26CueArg::Handle));
                    out.exit = A26Exit::Finish;
                }
            }
            _ => {}
        }
        out
    }

    /// `0x005abf29` (`FUN_005abf29`, ledger size 17): lowest accepted dialog
    /// id.
    ///
    /// ```asm
    /// 005abf29  mov  ecx, 0xb0
    /// 005abf2e  cmp  ax, cx
    /// 005abf31  ja   005abf49        ; saved-position restore arm
    /// 005abf33  mov  ecx, edi
    /// 005abf35  call FUN_005b4cd0      ; precondition, then fall into 0x005abf3a
    /// ```
    ///
    /// Ids at or below `0xb0` run the `FUN_005b4cd0` precondition and then the
    /// state dispatch; larger ids take the restore arm.  The sibling entry at
    /// `0x005abf3a` uses the wider `0xb2` bound, see
    /// [`World::a26_dialog_dispatch`].
    pub fn a26_dialog_arg_fits_low(arg: u16) -> bool {
        arg <= 0xb0
    }

    /// `0x005abf3a` (`FUN_005abf3a`, ledger size 56): second clamp plus the
    /// three-case state dispatch.
    ///
    /// ```asm
    /// 005abf3a  mov  ecx, 0xb2
    /// 005abf40  cmp  ax, cx
    /// 005abf43  ja   005abf49        ; restore saved position
    /// 005abf44  mov  al, [esi+0x4e]
    /// 005abf47  jmp  005abfb7        ; switch (state)
    /// 005abfb7  movzx eax, al ; sub eax, 0 ; je 005ac10b   ; state 0
    /// 005abfc4  sub  eax, 1   ; je 005abfd5                ; state 1
    /// 005abfc9  sub  eax, 2   ; je 005ac047                ; state 2
    /// 005abfce  pop/pop/pop ; ret                           ; other states
    /// ```
    ///
    /// The two clamped entries share this body; only the bound and the
    /// `FUN_005b4cd0` precondition differ.
    pub fn a26_dialog_dispatch(arg: u16, state: u8) -> A26DialogDispatch {
        if arg > 0xb2 {
            return A26DialogDispatch::RestorePosition;
        }
        match state {
            0 => A26DialogDispatch::Latch,
            1 => A26DialogDispatch::Engage,
            2 => A26DialogDispatch::Engaged,
            _ => A26DialogDispatch::Idle,
        }
    }

    /// `0x005abf6f` (`FUN_005abf6f`, ledger size 57): operand selection for
    /// the lookup at `FUN_005c6970`.
    ///
    /// ```asm
    /// 005abf6f  mov  [ebp-2], ax          ; the u16 argument is parked
    /// 005abf73  mov  eax, [ebp-4]         ; this entry uses the local dword
    /// 005abf76  jmp  005abf86
    /// 005abf78  mov  eax, [esi+0xc4]      ; alternate entry
    /// 005abf7e  xor  ecx, ecx
    /// 005abf80  mov  [esi+0xc4], ecx      ; clears the saved position
    /// 005abf86  push eax ; mov ecx, esi ; call FUN_005c6970
    /// 005abf8e  mov  edi, eax ; test ; jne abort   ; error -> +0x5c, abort
    /// ```
    ///
    /// Returns the operand handed to `FUN_005c6970` and whether the alternate
    /// arm cleared `+0xc4`.  `stored` is `None` for this entry's own arm and
    /// `Some(value)` for the alternate one.
    pub fn a26_dialog_lookup_operand(local: u32, stored: Option<u32>) -> (u32, bool) {
        match stored {
            Some(value) => (value, true),
            None => (local, false),
        }
    }

    /// `0x005abfad` (`FUN_005abfad`, ledger size 127): the reset arm.
    ///
    /// ```asm
    /// 005abfad  mov  [esi+0x5a], ax
    /// 005abfb1  xor  al, al
    /// 005abfb3  mov  [esi+0x4e], 0        ; state = 0
    /// 005abfb7  movzx eax, al
    /// 005abfba  sub  eax, 0
    /// 005abfbd  je   005ac10b             ; always case 0
    /// 005ac10b  push [esi+0x5c] ; call FUN_005d8f30
    /// 005ac117  je   005abfcf             ; no write, ret
    /// 005ac11d  mov  [esi+0xc4], [esi+0x58]
    /// 005ac12a  mov  [esi+0xc8], [esi+0x5c]
    /// 005ac130  mov  [esi+0x4e], 1
    /// ```
    ///
    /// The u16 argument is stored in the `+0x5a` half of the position pair
    /// before the state is zeroed.  `view_ok` is the `FUN_005d8f30` result; a
    /// hit latches the live position and leaves state `1`, a miss leaves the
    /// zeroed state untouched.
    pub fn a26_dialog_reset(view_ok: bool) -> A26Dialog {
        let mut out = A26Dialog::IDLE;
        out.state = Some(0);
        if view_ok {
            out.state = Some(1);
            out.latch = true;
        }
        out
    }
}

#[cfg(test)]
mod tests_agent26 {
    use super::*;
    use crate::data::GameData;
    use crate::ids::unit as uid;
    use crate::map::Terrain;
    use std::sync::Arc;

    fn open_world() -> World {
        World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(64, 64),
            2,
        )
    }

    fn spawn(world: &mut World, x: i32, y: i32) -> usize {
        world.spawn_px(0, uid::MARINE, x, y).unwrap()
    }

    #[test]
    fn arith_sub_is_wrapping_subtraction() {
        let mut w = open_world();
        let i = spawn(&mut w, 20, 10);
        let unit = &w.units[i];
        assert_eq!(
            SimUnit::a26_arith_sub(unit.x as u32, 3),
            (unit.x as u32) - 3
        );
        assert_eq!(SimUnit::a26_arith_sub(10, 3), 7);
        assert_eq!(SimUnit::a26_arith_sub(0, 1), u32::MAX);
    }

    #[test]
    fn arith_apply_selects_sub_add_xor() {
        let w = open_world();
        assert_eq!(w.units.len(), 0);
        assert_eq!(SimUnit::a26_arith_apply(0, 10, 4), 6);
        assert_eq!(SimUnit::a26_arith_apply(1, 10, 4), 14);
        assert_eq!(SimUnit::a26_arith_apply(2, 0b1010, 0b0110), 0b1100);
        // Selectors outside 0..=2 fall through both compares.
        assert_eq!(SimUnit::a26_arith_apply(3, 10, 4), 10);
    }

    #[test]
    fn arith_index_folds_three_selectors() {
        let mut w = open_world();
        let i = spawn(&mut w, 32, 32);
        let base = w.units[i].x as u32;
        assert_eq!(
            SimUnit::a26_arith_index(0, base, 0x08, 0, 4, 1, 2),
            base - 4 + 2
        );
        // The label itself only XORs on the first selector's `2` arm.
        assert_eq!(
            SimUnit::a26_arith_index(2, 0xff, 0x0f, 1, 1, 2, 1),
            ((0xff ^ 0x0f) + 1) ^ 1
        );
        assert_eq!(SimUnit::a26_arith_index(1, 0xff, 0x0f, 0, 0, 0, 0), 0xff);
    }

    #[test]
    fn rect_fields_reproduce_the_recovered_chains() {
        let mut w = open_world();
        let i = spawn(&mut w, 320, 288);
        let unit = &w.units[i];
        let (x, y) = (unit.x, unit.y);
        let (wd, ht) = (8, 4);

        let f = SimUnit::a26_rect_fields(x, y, wd, ht, A26FieldOps::default());
        // +0xa0 = x - w + h - h - w - y.
        assert_eq!(f[1], x - 2 * wd - y);
        // The three side chains cancel for every selector.
        assert_eq!((f[0], f[2], f[3]), (0, 0, 0));

        for ops in [
            A26FieldOps { w: 1, h: 0, y: 0 },
            A26FieldOps { w: 0, h: 1, y: 0 },
            A26FieldOps { w: 0, h: 0, y: 1 },
            A26FieldOps { w: 2, h: 2, y: 2 },
            A26FieldOps { w: 1, h: 1, y: 1 },
        ] {
            let f = SimUnit::a26_rect_fields(x, y, wd, ht, ops);
            assert_eq!((f[0], f[2], f[3]), (0, 0, 0), "{ops:?}");
        }

        // With all three selectors flipped both `w` steps add, so the
        // `+0xa0` chain becomes `x + 2w + y`.
        let flipped = SimUnit::a26_rect_fields(x, y, wd, ht, A26FieldOps { w: 1, h: 1, y: 1 });
        assert_eq!(flipped[1], x + 2 * wd + y);
    }

    #[test]
    fn guard_fold_matches_the_fastfail_arithmetic() {
        let mut w = open_world();
        let i = spawn(&mut w, 64, 64);
        assert!(w.units[i].alive);
        // (5 + 3) - 2, the first dispatch's default arm is `add`.
        assert_eq!(SimUnit::a26_guard_fold(0, 5, 3, 0, 2), 6);
        // (5 - 3) + 2.
        assert_eq!(SimUnit::a26_guard_fold(1, 5, 3, 1, 2), 4);
        // (0b1100 ^ 0b1010) ^ 0b0011.
        assert_eq!(
            SimUnit::a26_guard_fold(2, 0b1100, 0b1010, 2, 0b0011),
            0b0110 ^ 0b0011
        );
        // Unused selectors leave the accumulator alone.
        assert_eq!(SimUnit::a26_guard_fold(7, 9, 4, 7, 4), 9);
    }

    #[test]
    fn transmission_tick_state_zero_arms() {
        let mut w = open_world();
        let i = spawn(&mut w, 320, 288);
        assert_eq!(w.units[i].y, 288 * 256);

        // The forced cleanup at 0x005abb1f wins over everything.
        let out =
            World::a26_transmission_tick(0, true, true, false, 0x11, true, true, 1, true, false);
        assert_eq!(out.exit, A26Exit::Dispose);
        assert_eq!(out.preroll, None);

        // No linked dialog: state 0 disposes.
        let out =
            World::a26_transmission_tick(0, false, false, true, 0, false, false, 0, false, false);
        assert_eq!(out.exit, A26Exit::Dispose);

        // Acquire path queues 0x55 with the live position and finishes.
        let out =
            World::a26_transmission_tick(0, true, false, true, 0, true, false, 0, false, false);
        assert_eq!(out.cue, Some((0x55, A26CueArg::LivePosition)));
        assert_eq!(out.exit, A26Exit::Finish);
        assert_eq!(out.state, None);

        // View predicate latches the position and advances to state 3.
        let out =
            World::a26_transmission_tick(0, true, false, true, 0, false, true, 0, false, false);
        assert_eq!(out.state, Some(3));
        assert!(out.latch);
        assert_eq!(out.exit, A26Exit::Return);

        // Neither predicate: plain return.
        let out =
            World::a26_transmission_tick(0, true, false, true, 0, false, false, 0, false, false);
        assert_eq!(out, A26Dialog::IDLE);

        // The 0x97 pre-roll rides in front of the state work.
        let out = World::a26_transmission_tick(
            0, true, false, false, 0x2a, false, false, 0, false, false,
        );
        assert_eq!(out.preroll, Some((0x97, A26CueArg::Slot(0x2a))));
    }

    #[test]
    fn transmission_tick_state_three_arms() {
        let w = open_world();
        assert_eq!(w.players.len(), 2);

        // engage == 0 returns before the link checks.
        let out =
            World::a26_transmission_tick(3, false, false, true, 0, false, false, 0, false, false);
        assert_eq!(out, A26Dialog::IDLE);

        // Missing link or failed target check disposes.
        let out =
            World::a26_transmission_tick(3, false, false, true, 0, false, false, 1, true, false);
        assert_eq!(out.exit, A26Exit::Dispose);
        let out =
            World::a26_transmission_tick(3, true, false, true, 0, false, false, 1, false, false);
        assert_eq!(out.exit, A26Exit::Dispose);

        // engage == 2 falls back to state 0.
        let out =
            World::a26_transmission_tick(3, true, false, true, 0, false, false, 2, true, false);
        assert_eq!(out.state, Some(0));
        assert_eq!(out.exit, A26Exit::Return);

        // FUN_005b5620 selects the overlay cue.
        let out =
            World::a26_transmission_tick(3, true, false, true, 0, false, false, 1, true, true);
        assert_eq!(out.cue, Some((0x54, A26CueArg::NoArgument)));
        assert_eq!(out.exit, A26Exit::Return);

        let out =
            World::a26_transmission_tick(3, true, false, true, 0, false, false, 1, true, false);
        assert_eq!(out.cue, Some((0x52, A26CueArg::Handle)));
        assert_eq!(out.exit, A26Exit::Finish);

        // Other states fall through the al == 0 / al == 3 tests.
        let out = World::a26_transmission_tick(7, true, false, true, 0, true, true, 2, true, true);
        assert_eq!(out, A26Dialog::IDLE);
    }

    #[test]
    fn dialog_arg_bound_is_zero_b0() {
        let mut w = open_world();
        let i = spawn(&mut w, 32, 32);
        assert!(w.units[i].alive);
        assert!(World::a26_dialog_arg_fits_low(0));
        assert!(World::a26_dialog_arg_fits_low(0xb0));
        assert!(!World::a26_dialog_arg_fits_low(0xb1));
        assert!(!World::a26_dialog_arg_fits_low(0xffff));
    }

    #[test]
    fn dialog_dispatch_uses_the_b2_bound_and_state() {
        let w = open_world();
        assert_eq!(w.units.len(), 0);
        assert_eq!(
            World::a26_dialog_dispatch(0xb2, 0),
            A26DialogDispatch::Latch
        );
        assert_eq!(
            World::a26_dialog_dispatch(0xb2, 1),
            A26DialogDispatch::Engage
        );
        assert_eq!(
            World::a26_dialog_dispatch(0xb2, 2),
            A26DialogDispatch::Engaged
        );
        assert_eq!(World::a26_dialog_dispatch(0xb2, 3), A26DialogDispatch::Idle);
        // One past the second bound restores the saved position.
        assert_eq!(
            World::a26_dialog_dispatch(0xb3, 0),
            A26DialogDispatch::RestorePosition
        );
        assert_eq!(
            World::a26_dialog_dispatch(0x0200, 2),
            A26DialogDispatch::RestorePosition
        );
    }

    #[test]
    fn dialog_lookup_operand_prefers_local_and_clears_stored() {
        let mut w = open_world();
        let i = spawn(&mut w, 32, 32);
        let id = w.units[i].id;
        assert_eq!(World::a26_dialog_lookup_operand(id, None), (id, false));
        assert_eq!(
            World::a26_dialog_lookup_operand(id, Some(0x99)),
            (0x99, true)
        );
    }

    #[test]
    fn dialog_reset_reaches_case_zero() {
        let mut w = open_world();
        let i = spawn(&mut w, 32, 32);
        assert_eq!(w.units[i].owner, 0);

        let hit = World::a26_dialog_reset(true);
        assert_eq!(hit.state, Some(1));
        assert!(hit.latch);
        assert_eq!(hit.exit, A26Exit::Return);

        let miss = World::a26_dialog_reset(false);
        assert_eq!(miss.state, Some(0));
        assert!(!miss.latch);
        assert_eq!(miss.exit, A26Exit::Return);
    }
}
