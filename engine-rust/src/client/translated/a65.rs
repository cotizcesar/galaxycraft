#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/translated/agent65.rs
// ===========================================================================
// Translation batch 65.  Only this file is edited by the batch.
//
// Batch 65 owns `0x00614000..=0x00615fff`.  The ledger lists 42
// `decompiled_uncited` rows in that window; 17 are translated below and 25
// are skipped.  The skip reasons are recorded in each section comment and
// summarised at the end of this file.
//
// The window is the implementation of the retail
// `KeyboardAndMouseInputDriver`.  `0x006141b9` installs
// `KeyboardAndMouseInputDriver::vftable` and initialises the driver, the
// `0x006142xx..0x006145xx` rows are its vtable forwarders, and the
// `0x006146xx..0x006151xx` rows build and dispatch the input events.
// Recovered member offsets used by the bodies below:
//
// | offset        | meaning                                                    |
// |---------------|------------------------------------------------------------|
// | `+0x000`      | vtable                                                     |
// | `+0x104`      | byte set while replaying queued key states                 |
// | `+0x105`      | byte: queued key-up events still need flushing             |
// | `+0x106`      | byte set when a replay was aborted by the event sink        |
// | `+0x108`      | dword: number of queued key-up events                      |
// | `+0x10c`      | byte: drag rectangle active                                |
// | `+0x10e..+0x115` | two packed `(i16 x, i16 y)` drag points                 |
// | `+0x118`      | dword parameter copied by `0x006147f0`                     |
// | `+0x11c`      | second dword parameter copied by `0x006147f0`              |
// | `+0x120`      | notification counter incremented by `FUN_00615098`           |
// | `+0x124`/`+0x128`/`+0x12c` | three dwords returned by `0x00614840`         |
// | `+0x130`/`+0x134` | `0xffffffff` sentinels written by `0x006141b9`          |
//
// Input events are 0x1c-byte records.  The routines below write kind at
// `+0x10`, a zeroed dword at `+0x12`, and the source/payload dword at
// `+0x18`; the qword at `+0x08` carries the key and modifier bits.  The
// shipped send helper at `FUN_009736b0` is not translated (it is outside this
// window), so the helpers here return the events in send order instead of
// invoking it.

// ---------------------------------------------------------------------------
// Recovered value shapes
// ---------------------------------------------------------------------------

/// One 0x1c-byte input event built by `0x006146c0`, `0x00614730`,
/// `0x00614980` and `0x00614c90`.
///
/// The retail record stores the same fields at these byte offsets:
/// `+0x00`/`+0x04` zeroed dwords, `+0x08` the qword state word, `+0x10` the
/// kind, `+0x12` a zeroed dword and `+0x18` the source/payload dword.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct A65Event {
    /// `+0x00` and `+0x04`: always zero in the recovered bodies.
    pub reserved: [u32; 2],
    /// `+0x08`: key/modifier state for key events, or the event bits for
    /// the `0x18` summary event.
    pub state: u64,
    /// `+0x10`: event kind.
    pub kind: u16,
    /// `+0x12`: always zero in the recovered bodies.
    pub zero12: u32,
    /// `+0x18`: source or payload dword.
    pub payload: u32,
}

/// Build one event with the recovered zeroed fields.
pub const fn a65_event(kind: u16, state: u64, payload: u32) -> A65Event {
    A65Event {
        reserved: [0; 2],
        state,
        kind,
        zero12: 0,
        payload,
    }
}

/// The recovered input-driver object.  Only the members touched by this
/// window are modelled; the full retail object is larger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A65InputDriver {
    /// `+0x000`: `KeyboardAndMouseInputDriver::vftable`.
    pub vtable: usize,
    /// `+0x104`: replay-in-progress byte.
    pub byte104: u8,
    /// `+0x105`: queued key-up flush byte.
    pub byte105: u8,
    /// `+0x106`: replay-aborted byte.
    pub byte106: u8,
    /// `+0x108`: number of queued key-up events.
    pub pending108: u32,
    /// `+0x10c`: drag-active byte.
    pub byte10c: u8,
    /// `+0x10e`: first drag point x.
    pub x1_10e: i16,
    /// `+0x110`: first drag point y.
    pub y1_110: i16,
    /// `+0x112`: second drag point x.
    pub x2_112: i16,
    /// `+0x114`: second drag point y.
    pub y2_114: i16,
    /// `+0x118`: first drag parameter.
    pub p1_118: u32,
    /// `+0x11c`: second drag parameter.
    pub p2_11c: u32,
    /// `+0x120`: notification counter.
    pub count_120: u32,
    /// `+0x124`: first value returned by `0x00614840`.
    pub q_124: u32,
    /// `+0x128`: second value returned by `0x00614840`.
    pub r_128: u32,
    /// `+0x12c`: third value returned by `0x00614840`.
    pub s_12c: u32,
    /// `+0x130`: `0xffffffff` sentinel.
    pub t_130: u32,
    /// `+0x134`: `0xffffffff` sentinel.
    pub u_134: u32,
}

impl A65InputDriver {
    /// `0x006141b9` (`FUN_006141b9`, 206 bytes): the driver constructor.
    ///
    /// The body installs the vtable at `+0x00`, zeroes the word at `+0x104`,
    /// the bytes at `+0x106` and `+0x10c`, the dword at `+0x108`, the qword
    /// at `+0x10e` (both packed drag points), and the dwords at
    /// `+0x118..+0x12c`, then writes `-1` to `+0x130` and `+0x134`.  The
    /// remaining stores install the object in `DAT_00fa94ac` and release the
    /// previous singleton; [`A65DriverRegistry`] reproduces that part.
    pub fn a65_construct(vtable: usize) -> Self {
        A65InputDriver {
            vtable,
            byte104: 0,
            byte105: 0,
            byte106: 0,
            pending108: 0,
            byte10c: 0,
            x1_10e: 0,
            y1_110: 0,
            x2_112: 0,
            y2_114: 0,
            p1_118: 0,
            p2_11c: 0,
            count_120: 0,
            q_124: 0,
            r_128: 0,
            s_12c: 0,
            t_130: 0xffff_ffff,
            u_134: 0xffff_ffff,
        }
    }

    /// `0x00614330` (`FUN_00614330`, 3 bytes): empty vtable slot.
    pub fn a65_noop_14330(&self) {}

    /// `0x006143b0` (`FUN_006143b0`, 3 bytes): empty vtable slot.
    pub fn a65_noop_143b0(&self) {}

    /// `0x00614620` (`FUN_00614620`, 66 bytes) applied to this driver.
    ///
    /// The three virtual queries at `+0x40` are called with keys `0x10`,
    /// `0x11` and `0x12`; the caller supplies their results.
    pub fn a65_modifier_mask(&self, key10: bool, key11: bool, key12: bool) -> u32 {
        a65_modifier_mask(key10, key11, key12)
    }

    /// `0x00614730` (`FUN_00614730`, 161 bytes): clear the drag flag and
    /// flush the queued key-up events.
    ///
    /// `+0x10c` is cleared unconditionally.  When `enabled` is non-zero and
    /// `+0x105` is set, the body sends `+0x108` events of kind `0x0f` with
    /// the `0xa00008` state word and `payload`, then clears `+0x105` and
    /// `+0x108`.  The returned vector is the send sequence.
    pub fn a65_flush_pending_keyups(&mut self, enabled: bool, payload: u32) -> Vec<A65Event> {
        self.byte10c = 0;
        let mut events = Vec::new();
        if enabled && self.byte105 != 0 {
            for _ in 0..self.pending108 {
                events.push(a65_event(0x000f, 0x00a0_0008, payload));
            }
            self.byte105 = 0;
            self.pending108 = 0;
        }
        events
    }

    /// `0x006147f0` (`FUN_006147f0`, 69 bytes): start a drag rectangle.
    ///
    /// The old `+0x10c` value is returned through the carry of the byte
    /// test: the body saves it, stores `1`, copies the two packed points
    /// into `+0x10e`/`+0x112` and the two dwords into `+0x118`/`+0x11c`, and
    /// only when the old byte was zero falls into `0x006151b0` to redraw.
    /// The boolean result is that redraw request.
    pub fn a65_begin_drag(
        &mut self,
        arg1: u32,
        arg2: u32,
        start: (i16, i16),
        end: (i16, i16),
    ) -> bool {
        let was_active = self.byte10c;
        self.byte10c = 1;
        self.x1_10e = start.0;
        self.y1_110 = start.1;
        self.x2_112 = end.0;
        self.y2_114 = end.1;
        self.p1_118 = arg1;
        self.p2_11c = arg2;
        was_active == 0
    }

    /// `0x00614840` (`FUN_00614840`, 40 bytes): copy `+0x124`, `+0x128`
    /// and `+0x12c` to the three output pointers.
    pub fn a65_drag_query(&self) -> (u32, u32, u32) {
        (self.q_124, self.r_128, self.s_12c)
    }

    /// `0x00614980` (`FUN_00614980`, 755 bytes): replay queued key states
    /// and emit the `0x18` summary event.
    ///
    /// The decompiled body uses `FUN_005614b0()` as the active predicate and
    /// `FUN_007a7160()` to expand the low 30 bits of `event` into the key
    /// array.  The expansion is supplied here as `keys`; the event bits are:
    ///
    /// * bits 0..29 select the key list (the literal `8` skips the whole
    ///   replay and skips the summary event);
    /// * bit 30 (`0x40000000`) becomes bit 24 of each key event state and
    ///   bit 40 of the summary state;
    /// * bit 31 is dropped from the summary state by the `0x3fffffff` mask.
    ///
    /// Each key event has kind `0x0f`, state
    /// `key | modifiers << 16 | (bit30 ? 0x01000000 : 0)`, and payload
    /// `payload`.  If the sink aborts after `abort_after` sent events, the
    /// body emits one `0x0f`/`0xa00008` teardown event per event already
    /// sent.  The summary event has kind `0x18`, state
    /// `(event & 0x3fffffff) | modifiers << 32 | (bit30 ? 0x10000000000 : 0)`
    /// and is emitted when `!third_flag || active`.  The decompiler's
    /// non-returning tail is represented by returning the event vector.
    pub fn a65_replay_key_states(
        &mut self,
        event: u64,
        keys: &[i8],
        payload: u32,
        active: bool,
        third_flag: bool,
        modifier_keys: [bool; 3],
        abort_after: Option<usize>,
    ) -> Vec<A65Event> {
        let mut events = Vec::new();
        let modifiers = a65_modifier_mask(modifier_keys[0], modifier_keys[1], modifier_keys[2]);
        let link_bit = (event & 0x4000_0000) != 0;

        if active && (event & 0x3fff_ffff) == 8 {
            return events;
        }

        if active {
            self.byte104 = 1;
            self.byte106 = 0;
            let mut completed = 0usize;
            for (index, &key) in keys.iter().enumerate() {
                let mut state = u64::from(key as i16 as u16);
                state |= u64::from(modifiers) << 16;
                if link_bit {
                    state |= 0x0100_0000;
                }
                events.push(a65_event(0x000f, state, payload));
                if abort_after == Some(index + 1) {
                    self.byte106 = 1;
                    break;
                }
                completed += 1;
            }
            if self.byte106 != 0 {
                for _ in 0..completed {
                    events.push(a65_event(0x000f, 0x00a0_0008, payload));
                }
            }
            self.byte104 = 0;
        }

        if !third_flag || active {
            let mut state = event & 0x3fff_ffff;
            state |= u64::from(modifiers) << 32;
            if link_bit {
                state |= 0x0100_0000_0000;
            }
            events.push(a65_event(0x0018, state, payload));
        }
        events
    }

    /// `0x006151b0` (`FUN_006151b0`, 180 bytes): build the drag rectangle
    /// and redraw it.
    ///
    /// The body first checks `+0x10c`, reads the two packed points from
    /// `+0x10e..+0x115`, maps both through `FUN_007b2f10` and calls the
    /// rectangle draw helper `FUN_00940bf0` with
    /// `(x1, y1, x2 - x1, y2 - y1)`.  `to_screen` models the two
    /// `FUN_007b2f10` calls; `None` means the drag flag was clear.
    pub fn a65_drag_rectangle<F>(&self, to_screen: F) -> Option<(i32, i32, i32, i32)>
    where
        F: Fn(i32, i32) -> (i32, i32),
    {
        if self.byte10c == 0 {
            return None;
        }
        let first = to_screen(i32::from(self.x1_10e), i32::from(self.y1_110));
        let second = to_screen(i32::from(self.x2_112), i32::from(self.y2_114));
        Some((
            first.0,
            first.1,
            second.0.wrapping_sub(first.0),
            second.1.wrapping_sub(first.1),
        ))
    }
}

/// `DAT_00fa94ac`: the process-wide input-driver singleton installed by
/// `0x006141b9`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct A65DriverRegistry {
    /// The installed driver, if any.
    pub active: Option<A65InputDriver>,
}

impl A65DriverRegistry {
    /// `0x006142a0` (`FUN_006142a0`, 6 bytes): `return DAT_00fa94ac`.
    pub fn a65_active_driver(&self) -> Option<&A65InputDriver> {
        self.active.as_ref()
    }

    /// The install half of `0x006141b9`: replace `DAT_00fa94ac`, returning
    /// the previously installed object so the caller can release it.
    pub fn a65_install(&mut self, driver: A65InputDriver) -> Option<A65InputDriver> {
        self.active.replace(driver)
    }
}

/// The 0x1c-byte object initialised by `0x006145d0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A65Init1c {
    /// `+0x00`: zeroed.
    pub d00: u32,
    /// `+0x04`: zeroed.
    pub d04: u32,
    /// `+0x08`: zeroed.
    pub d08: u32,
    /// `+0x0c`: zeroed.
    pub d0c: u32,
    /// `+0x10`: zeroed.
    pub d10: u32,
    /// `+0x14`: zeroed word.
    pub w14: u16,
    /// `+0x18`: the incoming dword.
    pub d18: u32,
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

/// `0x006140c0` (`FUN_006140c0`, 30 bytes): check the delete-object flag.
///
/// Bit 0 zero returns normally (scalar `operator delete`); bit 0 set routes
/// to the array form at `FUN_00923e90`.  Returns whether the array form is
/// selected.
pub const fn a65_delete_uses_array(flags: u8) -> bool {
    (flags & 1) != 0
}

/// `0x00614470` (`FUN_00614470`, 32 bytes): select the exception code.
///
/// A non-zero argument selects `0xc`; zero selects `0x138`.  Both paths call
/// the non-returning raise helper at `FUN_00923df0` with that code.
pub const fn a65_raise_code(value_nonzero: bool) -> u32 {
    if value_nonzero {
        0x0000_000c
    } else {
        0x0000_0138
    }
}

/// `0x006145d0` (`FUN_006145d0`, 45 bytes): initialise the 0x1c-byte object.
///
/// The body writes `0` to the dwords at `+0x00`, `+0x04`, `+0x08`, `+0x0c`
/// and `+0x10`, a zero word at `+0x14`, and the incoming dword at `+0x18`.
pub const fn a65_init_1c(value: u32) -> A65Init1c {
    A65Init1c {
        d00: 0,
        d04: 0,
        d08: 0,
        d0c: 0,
        d10: 0,
        w14: 0,
        d18: value,
    }
}

/// `0x00614620` (`FUN_00614620`, 66 bytes): build the modifier mask.
///
/// The body calls the virtual query at `+0x40` with keys `0x10`, `0x11` and
/// `0x12`.  Key `0x10` *assigns* `3` (it is not an OR of bit 1), key `0x11`
/// ORs `4`, and key `0x12` ORs `8`.
pub const fn a65_modifier_mask(key10: bool, key11: bool, key12: bool) -> u32 {
    let mut mask = 0;
    if key10 {
        mask = 3;
    }
    if key11 {
        mask |= 4;
    }
    if key12 {
        mask |= 8;
    }
    mask
}

/// `0x006146c0` (`FUN_006146c0`, 102 bytes): build a modifier event.
///
/// The guard call at `0x00614da0` precedes the branch.  Bit 4 is tested
/// first and selects kind `0x16`; otherwise bit 2 selects kind `0x15`.
/// Neither bit produces no event.
pub fn a65_modifier_event(flags: u32, payload: u32) -> Option<A65Event> {
    if (flags & 4) != 0 {
        Some(a65_event(0x0016, 0, payload))
    } else if (flags & 2) != 0 {
        Some(a65_event(0x0015, 0, payload))
    } else {
        None
    }
}

/// `0x006148b0` (`FUN_006148b0`, 34 bytes): the key-resync table.
///
/// Returns true exactly for `0x10`, `0x11`, `0x12`, `0x5b`, `0x5c` and
/// `0xa0..=0xa5`.  The caller at `FUN_006170d0` uses it to refresh the
/// modifier bytes after those keys are released.
pub const fn a65_key_needs_resync(key: u32) -> bool {
    matches!(key, 0x10 | 0x11 | 0x12 | 0x5b | 0x5c | 0xa0..=0xa5)
}

/// `0x00614da0` (`FUN_00614da0`, 55 bytes): conditional scratch clear.
///
/// The body checks `FUN_005614b0()`; a non-zero result zeroes a 1024-byte
/// stack buffer before both continuations reach decompiler-marked
/// non-returning trampolines (`FUN_00c8a080` and `FUN_00c86b6e`).  Only that
/// branch is recoverable, so it returns the zeroed buffer (`Some`) when the
/// predicate holds and `None` otherwise.
pub fn a65_clear_scratch_if_active(active: bool) -> Option<[u8; 0x400]> {
    if active { Some([0; 0x400]) } else { None }
}

/// Result of `0x00614c90`: the device forward plus any event the row emits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum A65MouseDispatch {
    /// A call through the driver vtable slot `+0x34` with `(index, payload)`,
    /// optionally followed by the `+0x38` refresh call, and optionally one
    /// event.
    DeviceCall {
        /// First argument to slot `+0x34`.
        index: u32,
        /// Second argument to slot `+0x34`.
        payload: u32,
        /// Whether slot `+0x38` is invoked afterwards.
        refresh: bool,
        /// Event emitted after the device call, if any.
        event: Option<A65Event>,
    },
    /// No device call; the listed events are emitted.
    Events(Vec<A65Event>),
    /// The body calls `0x006151b0` to redraw the drag rectangle.
    RedrawDrag,
    /// Selectors with no recovered work.
    Nothing,
}

/// `0x00614c90` (`FUN_00614c90`, 29 bytes): mouse/pointer dispatch switch.
///
/// `selector` is the first argument; `_arg2` is present in the body but
/// never read.  `modifiers` is the third argument and `payload` the fourth.
///
/// * `0`: slot `+0x34(0, payload)` then slot `+0x38()`.
/// * `1`: event kind `0x14`.
/// * `2`: for bit 4 call `0x006146c0(modifiers & !2, payload)`, then for
///   bit 2 call `0x006146c0(modifiers & !4, payload)`.
/// * `3`: `0x006151b0` drag redraw.
/// * `4`, `5`: nothing.
/// * `6`: slot `+0x34(1, payload)` then event kind `0x17`.
/// * `7..=10`: slot `+0x34(0, payload)`.
/// * anything else: nothing.
pub fn a65_mouse_dispatch(
    selector: u32,
    _arg2: u32,
    modifiers: u32,
    payload: u32,
) -> A65MouseDispatch {
    match selector {
        0 => A65MouseDispatch::DeviceCall {
            index: 0,
            payload,
            refresh: true,
            event: None,
        },
        1 => A65MouseDispatch::Events(vec![a65_event(0x0014, 0, payload)]),
        2 => {
            let mut events = Vec::new();
            if (modifiers & 4) != 0 {
                if let Some(event) = a65_modifier_event(modifiers & !2, payload) {
                    events.push(event);
                }
            }
            if (modifiers & 2) != 0 {
                if let Some(event) = a65_modifier_event(modifiers & !4, payload) {
                    events.push(event);
                }
            }
            A65MouseDispatch::Events(events)
        }
        3 => A65MouseDispatch::RedrawDrag,
        4 | 5 => A65MouseDispatch::Nothing,
        6 => A65MouseDispatch::DeviceCall {
            index: 1,
            payload,
            refresh: false,
            event: Some(a65_event(0x0017, 0, payload)),
        },
        7..=10 => A65MouseDispatch::DeviceCall {
            index: 0,
            payload,
            refresh: false,
            event: None,
        },
        _ => A65MouseDispatch::Nothing,
    }
}

// ---------------------------------------------------------------------------
// Skipped rows
// ---------------------------------------------------------------------------
//
// The following 21 rows are one-tail vtable forwarders.  Each body is just
// `(**(code **)(*(int *)*in_ECX + slot))(...)`, so the only recoverable
// behaviour is the slot number; the target method is outside this window and
// there is no arithmetic, table or string result to reproduce:
//
//   FUN_006140f0 slot +0x38   FUN_006142b0 slot +0x3c
//   FUN_006142c0 slot +0x4c   FUN_006142e0 slot +0x54
//   FUN_006142f0 slot +0x44   FUN_00614310 slot +0x34
//   FUN_00614320 slot +0x30   FUN_00614340 slot +0x2c
//   FUN_00614350 slot +0x24   FUN_00614360 slot +0x28
//   FUN_00614370 slot +0x40   FUN_00614380 slot +0x0c
//   FUN_006143c0 slot +0x20   FUN_006143d0 slot +0x04
//   FUN_00614400 slot +0x08   FUN_00614430 slot +0x14
//   FUN_00614440 slot +0x18   FUN_00614450 slot +0x10
//   FUN_00614460 slot +0x1c   FUN_00614590 slot +0x50
//   FUN_006145b0 slot +0x48
//
// Four further rows are not translatable:
//
// * `FUN_00614100` (44 bytes) is an SEH prolog whose only continuation is the
//   non-returning `FUN_00923df0(4, cookie)`; the decompiler recovered no
//   observable result.
// * `FUN_00615098` (261 bytes) starts with overlapping/bad instruction data and
//   computes its pointers from unaffecting registers; only the tail
//   (count `0x25` events, increment `+0x120`, set `+0x105`, clear `+0x108`)
//   is legible, so the row is left untranslated rather than guessed.
// * `FUN_00615e23` (186 bytes) and `FUN_00615ee3` (1093 bytes) are both flagged
//   `Control flow encountered bad instruction data` with overlapping
//   instructions and `halt_baddata()` paths.  The latter contains a legible
//   binary-heap sift-up fragment, but the surrounding address/data flow is
//   corrupt; neither body can be attributed reliably to this window's
//   driver, so both are skipped.

#[cfg(test)]
mod tests_agent65 {
    use super::*;

    /// `0x006140c0`: only bit 0 selects the array-delete form.
    #[test]
    fn delete_flag_selects_the_array_form() {
        assert!(!a65_delete_uses_array(0));
        assert!(a65_delete_uses_array(1));
        assert!(!a65_delete_uses_array(0xfe));
        assert!(a65_delete_uses_array(0xff));
    }

    /// `0x006141b9` and `0x006142a0`: constructor state and singleton.
    #[test]
    fn constructor_and_registry_match_the_recovered_state() {
        let driver = A65InputDriver::a65_construct(0xdead_beef);
        assert_eq!(driver.vtable, 0xdead_beef);
        assert_eq!(
            (
                driver.byte104,
                driver.byte105,
                driver.byte106,
                driver.pending108,
                driver.byte10c
            ),
            (0, 0, 0, 0, 0)
        );
        assert_eq!(
            (driver.x1_10e, driver.y1_110, driver.x2_112, driver.y2_114),
            (0, 0, 0, 0)
        );
        assert_eq!(
            (
                driver.p1_118,
                driver.p2_11c,
                driver.count_120,
                driver.q_124,
                driver.r_128,
                driver.s_12c
            ),
            (0, 0, 0, 0, 0, 0)
        );
        assert_eq!((driver.t_130, driver.u_134), (0xffff_ffff, 0xffff_ffff));

        let mut registry = A65DriverRegistry::default();
        assert!(registry.a65_active_driver().is_none());
        let previous = registry.a65_install(driver.clone());
        assert!(previous.is_none());
        assert_eq!(registry.a65_active_driver(), Some(&driver));

        let mut replacement = driver.clone();
        replacement.vtable = 0x1234;
        let previous = registry.a65_install(replacement.clone()).unwrap();
        assert_eq!(previous, driver);
        assert_eq!(registry.a65_active_driver(), Some(&replacement));
    }

    /// `0x00614330` / `0x006143b0`: both slots are empty.
    #[test]
    fn empty_vtable_slots_leave_the_driver_unchanged() {
        let driver = A65InputDriver::a65_construct(1);
        let before = driver.clone();
        driver.a65_noop_14330();
        driver.a65_noop_143b0();
        assert_eq!(driver, before);
    }

    /// `0x00614470`: the two exception codes.
    #[test]
    fn raise_code_matches_the_two_branches() {
        assert_eq!(a65_raise_code(true), 0x0c);
        assert_eq!(a65_raise_code(false), 0x138);
    }

    /// `0x006145d0`: every field except `+0x18` is zero.
    #[test]
    fn init_1c_zeroes_everything_but_the_last_dword() {
        assert_eq!(
            a65_init_1c(0),
            A65Init1c {
                d00: 0,
                d04: 0,
                d08: 0,
                d0c: 0,
                d10: 0,
                w14: 0,
                d18: 0,
            }
        );
        let init = a65_init_1c(0x1122_3344);
        assert_eq!(init.d18, 0x1122_3344);
        assert_eq!(
            (init.d00, init.d04, init.d08, init.d0c, init.d10, init.w14),
            (0, 0, 0, 0, 0, 0)
        );
    }

    /// `0x00614620`: key `0x10` assigns 3 instead of setting bit 1.
    #[test]
    fn modifier_mask_assigns_three_for_key_10() {
        assert_eq!(a65_modifier_mask(false, false, false), 0);
        assert_eq!(a65_modifier_mask(true, false, false), 3);
        assert_eq!(a65_modifier_mask(false, true, false), 4);
        assert_eq!(a65_modifier_mask(false, false, true), 8);
        assert_eq!(a65_modifier_mask(true, true, true), 3 | 4 | 8);
        assert_eq!(a65_modifier_mask(true, false, true), 3 | 8);

        let driver = A65InputDriver::a65_construct(0);
        assert_eq!(driver.a65_modifier_mask(true, true, false), 7);
    }

    /// `0x006148b0`: the exact key-resync set.
    #[test]
    fn key_resync_table_is_exact() {
        for key in [
            0x10, 0x11, 0x12, 0x5b, 0x5c, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5,
        ] {
            assert!(a65_key_needs_resync(key), "0x{key:02x} should resync");
        }
        for key in [0x0f, 0x13, 0x5a, 0x5d, 0x9f, 0xa6] {
            assert!(!a65_key_needs_resync(key), "0x{key:02x} should not");
        }
    }

    /// `0x006146c0`: bit 4 wins over bit 2, and neither bit means no event.
    #[test]
    fn modifier_event_prioritises_bit_four() {
        assert_eq!(a65_modifier_event(0, 9), None);
        assert_eq!(a65_modifier_event(1, 9), None);
        assert_eq!(a65_modifier_event(2, 9), Some(a65_event(0x15, 0, 9)));
        assert_eq!(a65_modifier_event(4, 9), Some(a65_event(0x16, 0, 9)));
        assert_eq!(a65_modifier_event(6, 9), Some(a65_event(0x16, 0, 9)));
        assert_eq!(a65_modifier_event(8, 9), None);
    }

    /// `0x00614730`: flush count events, clear the drag byte, and leave the
    /// queue alone when the enable byte is clear.
    #[test]
    fn flush_pending_keyups_reproduces_the_queue_drain() {
        let mut driver = A65InputDriver::a65_construct(0);
        driver.byte105 = 1;
        driver.pending108 = 3;
        driver.byte10c = 1;
        let events = driver.a65_flush_pending_keyups(true, 0x77);
        assert_eq!(events.len(), 3);
        for event in &events {
            assert_eq!(*event, a65_event(0x0f, 0xa00008, 0x77));
        }
        assert_eq!(driver.byte105, 0);
        assert_eq!(driver.pending108, 0);
        assert_eq!(driver.byte10c, 0);

        let mut disabled = A65InputDriver::a65_construct(0);
        disabled.byte105 = 1;
        disabled.pending108 = 2;
        disabled.byte10c = 1;
        assert!(disabled.a65_flush_pending_keyups(false, 0x77).is_empty());
        assert_eq!(disabled.byte105, 1);
        assert_eq!(disabled.pending108, 2);
        assert_eq!(disabled.byte10c, 0);
    }

    /// `0x006147f0`, `0x00614840`, `0x006151b0`: drag start, query and
    /// rectangle arithmetic.
    #[test]
    fn drag_helpers_reproduce_the_points_and_rectangle() {
        let mut driver = A65InputDriver::a65_construct(0);
        assert_eq!(
            driver.a65_begin_drag(0x1122_3344, 0x5566_7788, (10, 20), (40, 70)),
            true
        );
        assert_eq!(driver.byte10c, 1);
        assert_eq!(
            (driver.x1_10e, driver.y1_110, driver.x2_112, driver.y2_114),
            (10, 20, 40, 70)
        );
        assert_eq!((driver.p1_118, driver.p2_11c), (0x1122_3344, 0x5566_7788));
        assert_eq!(driver.a65_begin_drag(1, 2, (0, 0), (1, 1)), false);

        driver.q_124 = 11;
        driver.r_128 = 22;
        driver.s_12c = 300;
        assert_eq!(driver.a65_drag_query(), (11, 22, 300));

        let rect = driver.a65_drag_rectangle(|x, y| (x * 2, y * 2));
        assert_eq!(rect, Some((0, 0, 2, 2)));

        driver.x1_10e = 10;
        driver.y1_110 = 20;
        driver.x2_112 = 40;
        driver.y2_114 = 70;
        assert_eq!(
            driver.a65_drag_rectangle(|x, y| (x * 2, y * 2)),
            Some((20, 40, 60, 100))
        );

        driver.byte10c = 0;
        assert_eq!(driver.a65_drag_rectangle(|x, y| (x, y)), None);
    }

    /// `0x00614c90`: every switch arm, including the bit-4-before-bit-2
    /// modifier order.
    #[test]
    fn mouse_dispatch_covers_every_selector() {
        let payload = 9;
        assert_eq!(
            a65_mouse_dispatch(0, 0, 0, payload),
            A65MouseDispatch::DeviceCall {
                index: 0,
                payload,
                refresh: true,
                event: None
            }
        );
        assert_eq!(
            a65_mouse_dispatch(1, 0, 0, payload),
            A65MouseDispatch::Events(vec![a65_event(0x14, 0, payload)])
        );
        assert_eq!(
            a65_mouse_dispatch(2, 0, 6, payload),
            A65MouseDispatch::Events(vec![
                a65_event(0x16, 0, payload),
                a65_event(0x15, 0, payload),
            ])
        );
        assert_eq!(
            a65_mouse_dispatch(2, 0, 0, payload),
            A65MouseDispatch::Events(Vec::new())
        );
        assert_eq!(
            a65_mouse_dispatch(3, 0, 0, payload),
            A65MouseDispatch::RedrawDrag
        );
        assert_eq!(
            a65_mouse_dispatch(4, 0, 0, payload),
            A65MouseDispatch::Nothing
        );
        assert_eq!(
            a65_mouse_dispatch(5, 0, 0, payload),
            A65MouseDispatch::Nothing
        );
        assert_eq!(
            a65_mouse_dispatch(6, 0, 0, payload),
            A65MouseDispatch::DeviceCall {
                index: 1,
                payload,
                refresh: false,
                event: Some(a65_event(0x17, 0, payload))
            }
        );
        assert_eq!(
            a65_mouse_dispatch(7, 0, 0, payload),
            A65MouseDispatch::DeviceCall {
                index: 0,
                payload,
                refresh: false,
                event: None
            }
        );
        assert_eq!(
            a65_mouse_dispatch(10, 0, 0, payload),
            A65MouseDispatch::DeviceCall {
                index: 0,
                payload,
                refresh: false,
                event: None
            }
        );
        assert_eq!(
            a65_mouse_dispatch(11, 0, 0, payload),
            A65MouseDispatch::Nothing
        );
        assert_eq!(
            a65_mouse_dispatch(0xffff_ffff, 0, 0, payload),
            A65MouseDispatch::Nothing
        );
    }

    /// `0x00614980`: key replay, bit-30 packing, and the summary event.
    #[test]
    fn replay_key_states_packs_keys_and_summary() {
        let mut driver = A65InputDriver::a65_construct(0);
        let events = driver.a65_replay_key_states(
            0x4000_0011,
            &[-1, 0x41],
            0x33,
            true,
            false,
            [true, true, false],
            None,
        );
        assert_eq!(events.len(), 3);
        assert_eq!(events[0], a65_event(0x0f, 0x0107_ffff, 0x33));
        assert_eq!(events[1], a65_event(0x0f, 0x0107_0041, 0x33));
        assert_eq!(events[2], a65_event(0x18, 0x0107_0000_0011, 0x33));
        assert_eq!(driver.byte104, 0);
        assert_eq!(driver.byte106, 0);
    }

    /// `0x00614980`: an aborted replay emits one teardown per sent event.
    #[test]
    fn replay_abort_emits_teardown_events() {
        let mut driver = A65InputDriver::a65_construct(0);
        let events = driver.a65_replay_key_states(
            0,
            &[1, 2, 3],
            0x44,
            true,
            true,
            [false, false, false],
            Some(2),
        );
        assert_eq!(events.len(), 4);
        assert_eq!(events[0], a65_event(0x0f, 1, 0x44));
        assert_eq!(events[1], a65_event(0x0f, 2, 0x44));
        assert_eq!(events[2], a65_event(0x0f, 0xa00008, 0x44));
        assert_eq!(events[3], a65_event(0x18, 0, 0x44));
        assert_eq!(driver.byte104, 0);
        assert_eq!(driver.byte106, 1);

        // The index only advances after a successful send, so aborting on the
        // very first key leaves nothing to tear down.
        let first_abort = driver.a65_replay_key_states(
            0,
            &[7, 8],
            0x44,
            true,
            false,
            [false, false, false],
            Some(1),
        );
        assert_eq!(
            first_abort,
            vec![a65_event(0x0f, 7, 0x44), a65_event(0x18, 0, 0x44),]
        );
    }

    /// `0x00614980`: inactive skips the replay, the literal key `8` skips
    /// everything, and the summary gate is `!third_flag || active`.
    #[test]
    fn replay_gates_match_the_recovered_branches() {
        let mut driver = A65InputDriver::a65_construct(0);
        let inactive = driver.a65_replay_key_states(
            0x4000_0002,
            &[1],
            0x55,
            false,
            false,
            [false, false, false],
            None,
        );
        assert_eq!(inactive, vec![a65_event(0x18, 0x0100_0000_0002, 0x55)]);
        assert_eq!(driver.byte104, 0);

        let gated = driver.a65_replay_key_states(
            0x4000_0002,
            &[1],
            0x55,
            false,
            true,
            [false, false, false],
            None,
        );
        assert!(gated.is_empty());

        let literal_eight = driver.a65_replay_key_states(
            8,
            &[1, 2],
            0x55,
            true,
            false,
            [false, false, false],
            None,
        );
        assert!(literal_eight.is_empty());

        let active_no_keys =
            driver.a65_replay_key_states(0x10, &[], 0x55, true, false, [false, true, false], None);
        assert_eq!(
            active_no_keys,
            vec![a65_event(0x18, 0x0000_0004_0000_0010, 0x55)]
        );
    }

    /// `0x00614da0`: the recoverable branch is the conditional zeroed
    /// 1024-byte scratch buffer.
    #[test]
    fn scratch_guard_returns_a_clear_buffer_only_when_active() {
        assert_eq!(a65_clear_scratch_if_active(false), None);
        let buffer = a65_clear_scratch_if_active(true).unwrap();
        assert_eq!(buffer.len(), 0x400);
        assert!(buffer.iter().all(|byte| *byte == 0));
    }
}
