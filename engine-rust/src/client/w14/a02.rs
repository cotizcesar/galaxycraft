#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave14/agent02.rs
// ===========================================================================
// Wave 14 translation batch 02 (staging).
//
// Window: `00672000..00673fff`.
//
// The window is the tail of one 32-bit `__thiscall` input/text control: a
// record queue plus the pointer, hot-key and source-string entry points that
// feed it.  Ghidra passes the object in `ECX`, so the recovered layout is the
// `this`-relative offsets the bodies read:
//
// | offset | layout |
// |--------|--------|
// | `+0xe8` | `i32` state: `1` is the queue-drain state, `2` the empty-source state |
// | `+0xec` | `u8` dispatch enable; every event gate tests it |
// | `+0xf0` | block-pointer array of the record ring (not modelled) |
// | `+0xf4` | `u32` block count, a power of two |
// | `+0xf8` | `u32` monotonic ring cursor |
// | `+0xfc` | `i32` queued record count |
// | `+0x100` | `u8` accept-any-key override |
//
// Ring records are `0x1c` bytes and eight records share one block:
// `block = (block_count - 1) & (cursor >> 3)`, `slot = cursor & 7`,
// `record = blocks[block] + slot * 0x1c`.  The write side of the same ring
// (`FUN_00631120`) computes the identical indices, which corroborates the
// geometry recovered from the two drain bodies.
//
// Only the events and the source dispatch are modelled; the client sinks
// (`FUN_008062f0`, `FUN_00806490`, `FUN_00806620`, `FUN_00866d20`), the
// allocator (`FUN_0093ec70`) and the queue storage itself are left out.  The
// helpers take and return plain values so they can be audited without the
// shipped object graph.
//
// ## Skipped rows
//
// * `FUN_00672160`, `FUN_00672220` - identical 18-byte thunks that forward
//   `this` to `FUN_007e1fd0` and never return; no other argument and no
//   recovered contract.
// * `FUN_00672270` - three-byte row that opens an SEH registration
//   (`ExceptionList` link and a cookie slot) and then calls `FUN_007e1fd0`;
//   it is a prologue fragment with no recoverable arguments.
// * `FUN_00672273` - mid-function SEH fragment: stores the frame cookie from
//   `unaff_EBP`, relinks `ExceptionList` and calls `FUN_007e1fd0`; every
//   operand is `unaff_EBP`.
// * `FUN_0067269b` - mid-function block of the local-path branch; it writes
//   `0x2f` through `unaff_EBP`/`unaff_EDI` slots and ends in the
//   non-returning `FUN_00538810`/`FUN_00c86b6e` pair with no arguments.
// * `FUN_0067271c` - virtual call through the unrecovered object in `EAX`
//   (`[eax + 0x10]`) followed by the SEH epilogue; no arguments, no result.
// * `FUN_00672770` - ledger row `decompile_failed`.
// * `FUN_00672d51` - dominated by `halt_baddata`, overlapping instructions and
//   an unrecovered jump table; the surviving blocks are `unaff_*` pointer
//   arithmetic.
// * `FUN_006730ab` - bare two-call terminate stub (`FUN_0053f940`,
//   `FUN_0053a640`), both non-returning, with no arguments or state.
// * `FUN_00673228` - five-byte `ret`.
// * `FUN_006732b0` - ledger row `decompile_failed`.
// * `FUN_00673bdd` - fragment that reads `unaff_EBX`/`unaff_EBP` plus a flags
//   register (`in_ZF`) and stores `[ebp + 8] = [ebx + 0x108]`; no recovered
//   arguments.
// * `FUN_00673cb5` - six-byte bare `ret`.
// * `FUN_00673cc0` - SEH prologue that calls `FUN_0066f490` with the frame
//   cookie and never returns; no arguments.
//
// The two drain rows `0x006736f0` and `0x006736fc` are the same routine at
// two entry points.  Their bodies are dominated by bad instruction data after
// the first few blocks, so only the recovered drain gate, the ring geometry
// and the pop side effects are translated here; the validator call, the
// per-record `+8` teardown and the sink call are not modelled.  The source
// router `0x00672590` is likewise translated for its recovered branch
// structure only: the local-file branch copies the text, runs an inlined
// capacity guard that reduces to `reserve(len)` and then dispatches through
// `FUN_00672770`, none of which is modelled.

// ---------------------------------------------------------------------------
// Recovered constants
// ---------------------------------------------------------------------------

/// `this + 0xe8`: queue-drain state.  Both drain rows only run when it is `1`.
pub const W14A02_STATE_DRAINABLE: i32 = 1;

/// `this + 0xe8`: state stored by the source router when the text is empty.
pub const W14A02_STATE_EMPTY_SOURCE: i32 = 2;

/// Offset of the `i32` state word (`0x00672590`, `0x006736f0`,
/// `0x006736fc`, `0x00673bb0`).
pub const W14A02_OFFSET_STATE: usize = 0xe8;

/// Offset of the `u8` dispatch enable byte (`0x00672120`, `0x00672140`,
/// `0x00672370`, `0x006723d0`, `0x00672400`, `0x00672460`).
pub const W14A02_OFFSET_ENABLED: usize = 0xec;

/// Offset of the `u32` block count of the record ring (`0x006736f0`,
/// `0x006736fc`).
pub const W14A02_OFFSET_QUEUE_BLOCK_COUNT: usize = 0xf4;

/// Offset of the `u32` ring cursor (`0x006736f0`, `0x006736fc`).
pub const W14A02_OFFSET_QUEUE_CURSOR: usize = 0xf8;

/// Offset of the `i32` queued record count (`0x006736f0`, `0x006736fc`).
pub const W14A02_OFFSET_QUEUE_COUNT: usize = 0xfc;

/// Offset of the `u8` accept-any-key byte (`0x006720f0`, `0x00673bc0`,
/// `0x00673c40`).
pub const W14A02_OFFSET_ACCEPT_ANY_KEY: usize = 0x100;

/// Bytes per queued record: the stride between ring slots (`0x006736f0`,
/// `0x006736fc`).
pub const W14A02_RING_RECORD_BYTES: usize = 0x1c;

/// Records per ring block (`0x006736f0`, `0x006736fc`).
pub const W14A02_RING_SLOTS_PER_BLOCK: u32 = 8;

/// Highest key code the filter accepts without the `+0x100` override
/// (`0x00673bc0`).
pub const W14A02_VK_MAX_CONTROL: u32 = 0x0c;

/// First key code above the always-accepted control range; the gap between
/// this and `W14A02_VK_MAX_CONTROL` is rejected without the override
/// (`0x00673bc0`).
pub const W14A02_VK_MIN_EXTENDED: u32 = 0x0e;

/// Last key code of the always-accepted extended range (`0x00673bc0`).
pub const W14A02_VK_MAX_EXTENDED: u32 = 0x1f;

/// Key codes the hot-key switch answers for indices `0..=5` (`0x00672490`):
/// `A`, `X`, `C`, `V`, `Z`, `Z`.
pub const W14A02_HOTKEY_VK: [u8; 6] = [0x41, 0x58, 0x43, 0x56, 0x5a, 0x5a];

/// Scheme bytes tested first by the source router (`0x00672590`).
pub const W14A02_HTTP_PREFIX: &[u8] = b"http://";

/// Scheme bytes tested second by the source router (`0x00672590`).
pub const W14A02_HTTPS_PREFIX: &[u8] = b"https://";

/// Byte searched for in the local-file branch (`0x00672590`): the backslash
/// passed to `FUN_00674460`.
pub const W14A02_PATH_SEPARATOR: u8 = 0x5c;

// ---------------------------------------------------------------------------
// Recovered object
// ---------------------------------------------------------------------------

/// The `this`-relative words the translated routines read or write.
///
/// The pointer fields of the shipped object (`+0xf0` block array and the
/// `+0xe0`/`+0xe4` sink words) are deliberately absent: they are not
/// representable as plain values and none of the modelled routines own them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a02EditState {
    /// `+0xe8`: `1` drains the queue, `2` marks an empty source.
    pub state: i32,
    /// `+0xec`: event dispatch enable.
    pub enabled: bool,
    /// `+0x100`: accept every key code (skip the filter and the apply gate).
    pub accept_any_key: bool,
    /// `+0xf4`: number of allocated ring blocks.
    pub queue_block_count: u32,
    /// `+0xf8`: monotonic ring cursor.
    pub queue_cursor: u32,
    /// `+0xfc`: queued record count.
    pub queue_count: i32,
}

/// The dispatch record the pointer handlers build before calling their sink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a02Dispatch {
    /// First payload word.
    pub first: u32,
    /// Second payload word.
    pub second: u32,
    /// Third payload word: the remapped button for the two button
    /// dispatchers, the pass-through word for `0x00672460`, and `0` when the
    /// routine has no third word (`0x006723d0`).
    pub code: u32,
    /// The literal `1`/`0` selector of the two button dispatchers; `None` for
    /// the payload dispatchers, which pass no selector.
    pub selector: Option<u32>,
    /// Modifier mask built by `FUN_00671d40` at the call site.
    pub modifiers: u16,
}

/// One ring slot, as addressed by the queue drain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a02RingSlot {
    /// Index into the `+0xf0` block-pointer array.
    pub block: usize,
    /// Slot inside the block, `0..8`.
    pub slot: usize,
    /// Byte offset of the record inside its block; `slot * 0x1c`.
    pub record_offset: usize,
}

/// Outcome of the hot-key table (`0x00672490`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W14a02Hotkey {
    /// Index outside `0..=5`; the shipped routine answers `0` without
    /// applying a key.
    Unbound,
    /// Index mapped and the `+0x100` byte set; the routine applies `vk` and
    /// answers `1`.
    Applied(u8),
    /// Index mapped but `+0x100` clear; the routine reaches the
    /// non-returning `FUN_00c66860`.
    Aborted(u8),
}

/// Recovered branch of the source router (`0x00672590`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W14a02Route {
    /// Zero-length source: the routine stores `2` at `this + 0xe8` and
    /// returns without dispatching.
    Empty,
    /// Scheme matched; the recovered body dispatched through
    /// `FUN_006732b0` for both schemes.
    Url {
        /// Length the compare was clamped to: `7` or `8`.
        prefix_len: usize,
    },
    /// Every other source: the body copies the text, looks up the first
    /// backslash with `FUN_00674460` and dispatches through `FUN_00672770`.
    LocalFile {
        /// Index of the first `0x5c` byte, or `None` when there is none.
        first_backslash: Option<usize>,
    },
}

// ---------------------------------------------------------------------------
// 0x006720f0 - set the accept-any-key override
// ---------------------------------------------------------------------------

/// `0x006720f0`: writes `1` to `this + 0x100` and returns `1`.
///
/// The shipped routine is a `__thiscall` with no stack arguments.  The
/// override is what lets the key filter (`0x00673bc0`) and the key apply
/// (`0x00673c40`) complete for rejected codes.
pub fn w14a02_accept_any_key(state: &mut W14a02EditState) -> u8 {
    state.accept_any_key = true;
    1
}

// ---------------------------------------------------------------------------
// 0x00672120 / 0x00672140 - enabled key entry points
// ---------------------------------------------------------------------------

/// `0x00672120`: when the `+0xec` enable byte is set, run the key filter
/// (`0x00673bc0`) over `key`, then return `1`.
///
/// `Some(1)` is the constant result of the shipped routine; `None` means the
/// filter took its non-returning abort path (`FUN_00c66860`), so the
/// `return 1` was never reached.  A disabled object returns `1` without
/// touching the filter, exactly like the shipped early exit.
pub fn w14a02_filter_key(state: &W14a02EditState, key: u32) -> Option<u8> {
    if !state.enabled {
        return Some(1);
    }
    if w14a02_key_allowed(key, state.accept_any_key) {
        Some(1)
    } else {
        None
    }
}

/// `0x00672140`: when the `+0xec` enable byte is set, apply `key` through
/// `FUN_00944d40`, then return `1`.
///
/// The shipped call is `FUN_00673c40(key, 0)`; the second (zero) argument is
/// discarded by the callee, so only the key and the `+0x100` gate survive.
/// `Some(1)` is the constant result; `None` means the apply gate aborted.
pub fn w14a02_apply_key(state: &W14a02EditState, key: u32) -> Option<u8> {
    if !state.enabled {
        return Some(1);
    }
    w14a02_key_apply(state.accept_any_key, key).map(|_| 1)
}

// ---------------------------------------------------------------------------
// 0x00672370 / 0x006723d0 / 0x00672400 / 0x00672460 - pointer dispatchers
// ---------------------------------------------------------------------------

/// `0x00672370`, `0x00672400`: button-code remap shared by the two button
/// dispatchers.
///
/// Button `2` becomes `3`, button `3` becomes `2`, and every other code
/// becomes the literal `1`.
pub fn w14a02_pointer_code(code: u32) -> u32 {
    match code {
        2 => 3,
        3 => 2,
        _ => 1,
    }
}

/// `0x00672370` (selector `1`) and `0x00672400` (selector `0`): build the
/// pointer event and hand it to `FUN_008062f0`.
///
/// The two rows are identical apart from the selector literal: both remap the
/// third argument through [`w14a02_pointer_code`], copy the first two words
/// into a two-word payload, and append the modifier mask.  `None` means the
/// `+0xec` gate was closed and nothing was dispatched.
pub fn w14a02_pointer_dispatch(
    state: &W14a02EditState,
    first: u32,
    second: u32,
    code: u32,
    selector: u32,
    modifiers: u16,
) -> Option<W14a02Dispatch> {
    if !state.enabled {
        return None;
    }
    Some(W14a02Dispatch {
        first,
        second,
        code: w14a02_pointer_code(code),
        selector: Some(selector),
        modifiers,
    })
}

/// `0x006723d0`: dispatch a two-word payload plus the modifier mask.
///
/// The shipped body passes the address of its first stack word to
/// `FUN_00806490`, which reads two payload words and the modifier mask; the
/// two words are modelled directly.  `None` means the `+0xec` gate was
/// closed.
pub fn w14a02_payload_dispatch(
    state: &W14a02EditState,
    first: u32,
    second: u32,
    modifiers: u16,
) -> Option<W14a02Dispatch> {
    if !state.enabled {
        return None;
    }
    Some(W14a02Dispatch {
        first,
        second,
        code: 0,
        selector: None,
        modifiers,
    })
}

/// `0x00672460`: dispatch a two-word payload, a third word and the modifier
/// mask to `FUN_00806620`.
///
/// The third argument is forwarded unchanged (`param_3`) and the two payload
/// words travel as one two-word record.  `None` means the `+0xec` gate was
/// closed.
pub fn w14a02_payload_code_dispatch(
    state: &W14a02EditState,
    first: u32,
    second: u32,
    code: u32,
    modifiers: u16,
) -> Option<W14a02Dispatch> {
    if !state.enabled {
        return None;
    }
    Some(W14a02Dispatch {
        first,
        second,
        code,
        selector: None,
        modifiers,
    })
}

// ---------------------------------------------------------------------------
// 0x00672490 - hot-key table
// ---------------------------------------------------------------------------

/// `0x00672490`: map a hot-key index to a virtual-key code and apply it.
///
/// The switch answers `0x41`, `0x58`, `0x43`, `0x56`, `0x5a`, `0x5a` for
/// indices `0..=5` (index `4` falls through into case `5`) and `0` for every
/// other index.  The mapped code is applied through `FUN_00944d40`; when the
/// `+0x100` byte is clear the routine then reaches the non-returning
/// `FUN_00c66860`, which the `Aborted` variant records.
pub fn w14a02_hotkey(index: u32, accept_any_key: bool) -> W14a02Hotkey {
    let Some(&vk) = usize::try_from(index)
        .ok()
        .and_then(|i| W14A02_HOTKEY_VK.get(i))
    else {
        return W14a02Hotkey::Unbound;
    };
    if accept_any_key {
        W14a02Hotkey::Applied(vk)
    } else {
        W14a02Hotkey::Aborted(vk)
    }
}

// ---------------------------------------------------------------------------
// 0x00672590 - source router
// ---------------------------------------------------------------------------

/// `0x00672590`: classify the source string handed to the object.
///
/// The recovered body:
///
/// 1. stores `2` at `this + 0xe8` and returns when the length word is zero;
/// 2. compares the text with `http://` and then `https://` (both through
///    `FUN_0067d9b0`, which clamps the compare to the shorter length and
///    requires equal lengths, so this is a prefix test) and dispatches
///    through `FUN_006732b0`;
/// 3. otherwise copies the text, looks up the first `0x5c` with
///    `FUN_00674460`, and dispatches through `FUN_00672770`.
///
/// Only the decision is modelled.  The local-file copy, the inlined capacity
/// guard (which reduces to `reserve(len)`) and both sinks are not.
pub fn w14a02_source_route(text: &[u8]) -> W14a02Route {
    if text.is_empty() {
        return W14a02Route::Empty;
    }
    if text.starts_with(W14A02_HTTP_PREFIX) {
        return W14a02Route::Url {
            prefix_len: W14A02_HTTP_PREFIX.len(),
        };
    }
    if text.starts_with(W14A02_HTTPS_PREFIX) {
        return W14a02Route::Url {
            prefix_len: W14A02_HTTPS_PREFIX.len(),
        };
    }
    W14a02Route::LocalFile {
        first_backslash: text.iter().position(|&b| b == W14A02_PATH_SEPARATOR),
    }
}

// ---------------------------------------------------------------------------
// 0x00673bb0 / 0x00673bc0 / 0x00673c40 - state and key predicates
// ---------------------------------------------------------------------------

/// `0x00673bb0`: `this + 0xe8 != 2`, the state predicate the control exposes
/// to its caller.
pub fn w14a02_state_present(state: i32) -> bool {
    state != W14A02_STATE_EMPTY_SOURCE
}

/// `0x00673bc0`: whether the key filter accepts `key`.
///
/// The shipped routine aborts through the non-returning `FUN_00c66860` when
/// `key > 0x0c` and (`key < 0x0e` or `key > 0x1f`) and the `+0x100` override
/// byte is clear.  `true` here means the routine returns normally.
pub fn w14a02_key_allowed(key: u32, accept_any_key: bool) -> bool {
    if accept_any_key {
        return true;
    }
    !(key > W14A02_VK_MAX_CONTROL
        && !(W14A02_VK_MIN_EXTENDED..=W14A02_VK_MAX_EXTENDED).contains(&key))
}

/// `0x00673c40`: apply `key` and report whether the routine completed.
///
/// The body forwards `key` to `FUN_00944d40` unchanged and then aborts through
/// the non-returning `FUN_00c66860` unless the `+0x100` byte is set, so the
/// applied key is echoed back only when the gate is open.
pub fn w14a02_key_apply(accept_any_key: bool, key: u32) -> Option<u32> {
    if accept_any_key { Some(key) } else { None }
}

// ---------------------------------------------------------------------------
// 0x006736f0 / 0x006736fc - queue drain geometry
// ---------------------------------------------------------------------------

/// `0x006736f0`, `0x006736fc`: address of the ring slot a cursor selects.
///
/// The recovered arithmetic is `(block_count - 1) & (cursor >> 3)` for the
/// block index and `cursor & 7` for the slot.  The subtraction wraps in the
/// shipped code, so a zero block count yields `cursor >> 3`; that is
/// preserved with `wrapping_sub`.
pub fn w14a02_ring_slot(block_count: u32, cursor: u32) -> (usize, usize) {
    let block = block_count.wrapping_sub(1) & (cursor >> 3);
    let slot = cursor & (W14A02_RING_SLOTS_PER_BLOCK - 1);
    (block as usize, slot as usize)
}

/// `0x006736f0`, `0x006736fc`: pop one record from the queue ring.
///
/// Both drain bodies start with `if (this + 0xe8 != 1) goto done;` and then
/// loop while `this + 0xfc != 0`, computing the slot with
/// [`w14a02_ring_slot`] and closing each iteration with `--count` and
/// `++cursor`.  This helper performs one such iteration and reports the slot.
/// The shipped loop also validates the record through `FUN_007a73c0`, tears
/// down the record's `+8` field and calls `FUN_00866d20`; none of that is
/// modelled.
///
/// `None` means the drain gate was closed or the queue was empty.  A negative
/// count also answers `None`; the shipped `!= 0` test would loop on a
/// corrupted count, which the safe model does not reproduce.
pub fn w14a02_queue_pop(state: &mut W14a02EditState) -> Option<W14a02RingSlot> {
    if state.state != W14A02_STATE_DRAINABLE || state.queue_count <= 0 {
        return None;
    }
    let (block, slot) = w14a02_ring_slot(state.queue_block_count, state.queue_cursor);
    state.queue_count -= 1;
    state.queue_cursor = state.queue_cursor.wrapping_add(1);
    Some(W14a02RingSlot {
        block,
        slot,
        record_offset: slot * W14A02_RING_RECORD_BYTES,
    })
}

// ---------------------------------------------------------------------------
// 0x00673f60 - enqueue validation gate
// ---------------------------------------------------------------------------

/// `0x00673f60`: whether a text argument passes the validator that guards the
/// queue insert.
///
/// The body builds a temporary from `param_1`, calls the UTF-8 validator
/// `FUN_007a73c0`, and on acceptance calls the ring insert `FUN_00631120`
/// before the non-returning copy/teardown.  The validator walks a
/// NUL-terminated string and accepts `0..=0x7f` and well-formed multi-byte
/// UTF-8, so the helper validates the prefix before the first NUL and treats
/// the rest of the slice as unreachable.  The temporary and the insert are
/// not modelled.
pub fn w14a02_text_accepted(text: &[u8]) -> bool {
    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    core::str::from_utf8(&text[..end]).is_ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_wave14_agent02 {
    use super::*;

    /// `0x006720f0`
    #[test]
    fn accept_any_key_flag_is_set_and_one_returned() {
        let mut state = W14a02EditState::default();
        assert!(!state.accept_any_key);
        assert_eq!(w14a02_accept_any_key(&mut state), 1);
        assert!(state.accept_any_key);
        // Idempotent: the shipped body stores the constant each time.
        assert_eq!(w14a02_accept_any_key(&mut state), 1);
        assert!(state.accept_any_key);
        assert_eq!(W14A02_OFFSET_ACCEPT_ANY_KEY, 0x100);
    }

    /// `0x00672120`
    #[test]
    fn filter_key_is_gated_by_enable_and_the_key_filter() {
        let mut state = W14a02EditState::default();
        // Disabled: constant 1, the filter is never consulted.
        assert_eq!(w14a02_filter_key(&state, 0x41), Some(1));

        state.enabled = true;
        assert_eq!(w14a02_filter_key(&state, 0x09), Some(1));
        assert_eq!(w14a02_filter_key(&state, 0x41), None);
        assert_eq!(w14a02_filter_key(&state, 0x0d), None);

        state.accept_any_key = true;
        assert_eq!(w14a02_filter_key(&state, 0x41), Some(1));
        assert_eq!(w14a02_filter_key(&state, 0x0d), Some(1));
        assert_eq!(W14A02_OFFSET_ENABLED, 0xec);
    }

    /// `0x00672140`
    #[test]
    fn apply_key_is_gated_by_enable_then_the_override_byte() {
        let mut state = W14a02EditState::default();
        assert_eq!(w14a02_apply_key(&state, 0x41), Some(1));

        state.enabled = true;
        assert_eq!(w14a02_apply_key(&state, 0x41), None);

        state.accept_any_key = true;
        assert_eq!(w14a02_apply_key(&state, 0x41), Some(1));
    }

    /// `0x00672370`, `0x00672400`
    #[test]
    fn pointer_dispatch_remaps_buttons_and_keeps_the_selector() {
        let mut state = W14a02EditState::default();
        assert_eq!(w14a02_pointer_dispatch(&state, 10, 20, 2, 1, 0x0005), None);

        state.enabled = true;
        let pressed = w14a02_pointer_dispatch(&state, 10, 20, 2, 1, 0x0005).unwrap();
        assert_eq!(pressed.first, 10);
        assert_eq!(pressed.second, 20);
        assert_eq!(pressed.code, 3);
        assert_eq!(pressed.selector, Some(1));
        assert_eq!(pressed.modifiers, 0x0005);

        let released = w14a02_pointer_dispatch(&state, 11, 21, 3, 0, 0).unwrap();
        assert_eq!(released.code, 2);
        assert_eq!(released.selector, Some(0));
    }

    /// `0x00672370`, `0x00672400`: the remap table itself.
    #[test]
    fn pointer_code_table_swaps_two_and_three_and_defaults_to_one() {
        assert_eq!(w14a02_pointer_code(0), 1);
        assert_eq!(w14a02_pointer_code(1), 1);
        assert_eq!(w14a02_pointer_code(2), 3);
        assert_eq!(w14a02_pointer_code(3), 2);
        assert_eq!(w14a02_pointer_code(4), 1);
        assert_eq!(w14a02_pointer_code(u32::MAX), 1);
    }

    /// `0x006723d0`
    #[test]
    fn payload_dispatch_carries_two_words_and_no_selector() {
        let state = W14a02EditState {
            enabled: true,
            ..W14a02EditState::default()
        };
        let event = w14a02_payload_dispatch(&state, 7, 8, 0x0040).unwrap();
        assert_eq!(event.first, 7);
        assert_eq!(event.second, 8);
        assert_eq!(event.code, 0);
        assert_eq!(event.selector, None);
        assert_eq!(event.modifiers, 0x0040);

        let closed = W14a02EditState::default();
        assert_eq!(w14a02_payload_dispatch(&closed, 7, 8, 0), None);
    }

    /// `0x00672460`
    #[test]
    fn payload_code_dispatch_forwards_the_third_word() {
        let state = W14a02EditState {
            enabled: true,
            ..W14a02EditState::default()
        };
        let event = w14a02_payload_code_dispatch(&state, 1, 2, 0x1234, 0x0003).unwrap();
        assert_eq!(event.first, 1);
        assert_eq!(event.second, 2);
        assert_eq!(event.code, 0x1234);
        assert_eq!(event.selector, None);
        assert_eq!(event.modifiers, 0x0003);

        let closed = W14a02EditState::default();
        assert_eq!(w14a02_payload_code_dispatch(&closed, 1, 2, 3, 0), None);
    }

    /// `0x00672490`
    #[test]
    fn hotkey_table_matches_the_recovered_switch() {
        let expected = [0x41u8, 0x58, 0x43, 0x56, 0x5a, 0x5a];
        for (index, &vk) in expected.iter().enumerate() {
            assert_eq!(
                w14a02_hotkey(index as u32, true),
                W14a02Hotkey::Applied(vk),
                "index {index}"
            );
            assert_eq!(
                w14a02_hotkey(index as u32, false),
                W14a02Hotkey::Aborted(vk),
                "index {index}"
            );
        }
        // Index 4 falls through into case 5 in the shipped switch.
        assert_eq!(w14a02_hotkey(4, true), w14a02_hotkey(5, true));
        // Anything outside the switch answers 0 without applying a key,
        // whatever the override byte says.
        assert_eq!(w14a02_hotkey(6, true), W14a02Hotkey::Unbound);
        assert_eq!(w14a02_hotkey(6, false), W14a02Hotkey::Unbound);
        assert_eq!(w14a02_hotkey(u32::MAX, true), W14a02Hotkey::Unbound);
        assert_eq!(W14A02_HOTKEY_VK, expected);
    }

    /// `0x00672590`
    #[test]
    fn source_route_splits_empty_schemes_and_local_files() {
        assert_eq!(w14a02_source_route(b""), W14a02Route::Empty);
        assert_eq!(
            w14a02_source_route(b"http://example"),
            W14a02Route::Url { prefix_len: 7 }
        );
        assert_eq!(
            w14a02_source_route(b"https://example"),
            W14a02Route::Url { prefix_len: 8 }
        );
        // A bare "http:/" is not the seven-byte scheme.
        assert_eq!(
            w14a02_source_route(b"http:/x"),
            W14a02Route::LocalFile {
                first_backslash: None
            }
        );
        assert_eq!(
            w14a02_source_route(b"C:\\dir\\file.txt"),
            W14a02Route::LocalFile {
                first_backslash: Some(2)
            }
        );
        assert_eq!(
            w14a02_source_route(b"no separator"),
            W14a02Route::LocalFile {
                first_backslash: None
            }
        );
        assert_eq!(
            w14a02_source_route(b"\\leading"),
            W14a02Route::LocalFile {
                first_backslash: Some(0)
            }
        );
        assert_eq!(
            w14a02_source_route(b"trailing\\"),
            W14a02Route::LocalFile {
                first_backslash: Some(8)
            }
        );
    }

    /// `0x00673bb0`
    #[test]
    fn state_present_is_false_only_for_the_empty_source_state() {
        assert!(w14a02_state_present(0));
        assert!(w14a02_state_present(W14A02_STATE_DRAINABLE));
        assert!(!w14a02_state_present(W14A02_STATE_EMPTY_SOURCE));
        assert!(w14a02_state_present(3));
        assert!(w14a02_state_present(-1));
        assert_eq!(W14A02_STATE_EMPTY_SOURCE, 2);
        assert_eq!(W14A02_OFFSET_STATE, 0xe8);
    }

    /// `0x00673bc0`
    #[test]
    fn key_filter_rejects_enter_and_printable_codes_without_override() {
        for key in 0..=W14A02_VK_MAX_CONTROL {
            assert!(w14a02_key_allowed(key, false), "control {key:#x}");
        }
        // 0x0d sits in the gap above 0x0c and below 0x0e.
        assert!(!w14a02_key_allowed(0x0d, false));
        for key in W14A02_VK_MIN_EXTENDED..=W14A02_VK_MAX_EXTENDED {
            assert!(w14a02_key_allowed(key, false), "extended {key:#x}");
        }
        assert!(!w14a02_key_allowed(0x20, false));
        assert!(!w14a02_key_allowed(0x41, false));
        assert!(!w14a02_key_allowed(u32::MAX, false));

        // The override opens every code, including the two rejected ranges.
        for key in [0x0d, 0x20, 0x41, u32::MAX] {
            assert!(w14a02_key_allowed(key, true), "override {key:#x}");
        }
        assert_eq!(W14A02_VK_MAX_CONTROL + 1, W14A02_VK_MIN_EXTENDED - 1);
    }

    /// `0x00673c40`
    #[test]
    fn key_apply_echoes_the_key_only_when_the_override_is_set() {
        assert_eq!(w14a02_key_apply(true, 0x41), Some(0x41));
        assert_eq!(w14a02_key_apply(true, 0), Some(0));
        assert_eq!(w14a02_key_apply(false, 0x41), None);
        assert_eq!(w14a02_key_apply(false, u32::MAX), None);
    }

    /// `0x006736f0`, `0x006736fc`
    #[test]
    fn ring_slot_wraps_through_the_block_array() {
        // One block: every cursor lands in block 0, slots cycle 0..7.
        for cursor in 0..8u32 {
            assert_eq!(w14a02_ring_slot(1, cursor), (0, cursor as usize));
        }
        // Two blocks: the block index alternates with cursor >> 3.
        assert_eq!(w14a02_ring_slot(2, 0), (0, 0));
        assert_eq!(w14a02_ring_slot(2, 7), (0, 7));
        assert_eq!(w14a02_ring_slot(2, 8), (1, 0));
        assert_eq!(w14a02_ring_slot(2, 15), (1, 7));
        assert_eq!(w14a02_ring_slot(2, 16), (0, 0));
        assert_eq!(w14a02_ring_slot(2, 24), (1, 0));
        // A zero block count wraps the subtraction in the shipped code.
        assert_eq!(w14a02_ring_slot(0, 16), (2, 0));
        assert_eq!(W14A02_RING_SLOTS_PER_BLOCK, 8);
        assert_eq!(W14A02_RING_RECORD_BYTES, 0x1c);
    }

    /// `0x006736f0`, `0x006736fc`
    #[test]
    fn queue_pop_advances_the_cursor_and_decrements_the_count() {
        let mut state = W14a02EditState {
            state: W14A02_STATE_DRAINABLE,
            queue_block_count: 1,
            queue_cursor: 7,
            queue_count: 2,
            ..W14a02EditState::default()
        };
        let slot = w14a02_queue_pop(&mut state).unwrap();
        assert_eq!(
            slot,
            W14a02RingSlot {
                block: 0,
                slot: 7,
                record_offset: 7 * 0x1c
            }
        );
        assert_eq!(state.queue_count, 1);
        assert_eq!(state.queue_cursor, 8);

        // Second pop wraps into block (1 - 1) & (8 >> 3) = 0, slot 0.
        let slot = w14a02_queue_pop(&mut state).unwrap();
        assert_eq!(slot.block, 0);
        assert_eq!(slot.slot, 0);
        assert_eq!(state.queue_count, 0);
        assert_eq!(state.queue_cursor, 9);
        assert_eq!(w14a02_queue_pop(&mut state), None);
        assert_eq!(W14A02_OFFSET_QUEUE_BLOCK_COUNT, 0xf4);
        assert_eq!(W14A02_OFFSET_QUEUE_CURSOR, 0xf8);
        assert_eq!(W14A02_OFFSET_QUEUE_COUNT, 0xfc);

        // The drain gate refuses every other state.
        let mut idle = W14a02EditState {
            state: W14A02_STATE_EMPTY_SOURCE,
            queue_count: 4,
            queue_cursor: 4,
            ..W14a02EditState::default()
        };
        assert_eq!(w14a02_queue_pop(&mut idle), None);
        assert_eq!(idle.queue_count, 4);
        assert_eq!(idle.queue_cursor, 4);

        // A negative count is refused by the safe model.
        let mut corrupt = W14a02EditState {
            state: W14A02_STATE_DRAINABLE,
            queue_count: -1,
            queue_cursor: 0,
            ..W14a02EditState::default()
        };
        assert_eq!(w14a02_queue_pop(&mut corrupt), None);
    }

    /// `0x00673f60`
    #[test]
    fn text_gate_validates_utf8_up_to_the_first_nul() {
        assert!(w14a02_text_accepted(b""));
        assert!(w14a02_text_accepted(b"plain ascii"));
        assert!(w14a02_text_accepted("héllo".as_bytes()));
        assert!(w14a02_text_accepted("日本語".as_bytes()));
        // The shipped validator stops at the terminator, so a bad tail after
        // an embedded NUL is never examined.
        assert!(w14a02_text_accepted(b"ok\0\xff"));
        // Without the NUL the same bytes are rejected.
        assert!(!w14a02_text_accepted(b"\xff"));
        assert!(!w14a02_text_accepted(b"a\xc3"));
        // Overlong and surrogate encodings are rejected.
        assert!(!w14a02_text_accepted(b"\xc0\xaf"));
        assert!(!w14a02_text_accepted(b"\xed\xa0\x80"));
    }
}
