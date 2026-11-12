#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent01.rs
// ===========================================================================
// Wave 24 translation batch 01: small client helpers (message builders,
// effect-slot table walkers, vision-slot checks).

/// Number of entries in the 0x1c-byte effect slot table.
pub const W24A01_SLOT_COUNT: usize = 8;

/// `0x0072c500`: builds the two-byte net message `[0x21, value]` that the
/// original passes to the send helper with length 2.
pub fn w24a01_build_msg_21(value: u8) -> [u8; 2] {
    [0x21, value]
}

/// `0x0072cd60`: builds the one-byte net message `[0x36]` (length 1).
pub fn w24a01_build_msg_36() -> [u8; 1] {
    [0x36]
}

/// `0x00728200`: for command ids 1, 4, 8 and 0x19 the handler invokes the
/// stored callback once; for 2 and 0x17 it also invokes it (and returns).
/// Returns `true` when the callback was invoked, `false` for other ids.
pub fn w24a01_dispatch_cb(id: u32, mut cb: impl FnMut()) -> bool {
    match id {
        1 | 2 | 4 | 8 | 0x17 | 0x19 => {
            cb();
            true
        }
        _ => false,
    }
}

/// `0x0077de60`: returns whether player `index` may use the slot table.
/// `player_flag` is the byte at `index*0xb0 + 0x80` of the player table; a
/// zero flag gives `false`.  Otherwise every slot whose handle satisfies
/// `is_valid` and whose owner field (dword 3 of the 0x1c-byte record) equals
/// `index` makes the result `false`; if no slot matches the result is `true`.
pub fn w24a01_player_slot_free(
    index: u32,
    player_flag: u8,
    slots: &[(u32, u32); W24A01_SLOT_COUNT],
    is_valid: impl Fn(u32) -> bool,
) -> bool {
    if player_flag == 0 {
        return false;
    }
    !slots
        .iter()
        .any(|&(handle, owner)| is_valid(handle) && owner == index)
}

/// `0x0077f770`: calls the pre-step then `stop(handle)` for each of the
/// eight slot handles in order.
pub fn w24a01_stop_all_slots(
    handles: &[u32; W24A01_SLOT_COUNT],
    mut pre: impl FnMut(),
    mut stop: impl FnMut(u32),
) {
    pre();
    for &h in handles {
        stop(h);
    }
}

/// `0x0077e620`: same as the stop-all walk followed by a final `post` call.
pub fn w24a01_stop_all_slots_then(
    handles: &[u32; W24A01_SLOT_COUNT],
    pre: impl FnMut(),
    stop: impl FnMut(u32),
    mut post: impl FnMut(),
) {
    w24a01_stop_all_slots(handles, pre, stop);
    post();
}

/// `0x007ad300`: given a record with `active` (dword at +0x54) and `mode`
/// (dword at +0x88), returns `Some((count, mode))` where
/// `count = 4 - (mode != 1)` when `check(i)` holds for every i in
/// `0..count`; `None` when inactive or any check fails.  The caller's base
/// plus 0x8c is the returned pointer.
pub fn w24a01_probe_channels(
    active: u32,
    mode: u32,
    mut check: impl FnMut(u32) -> bool,
) -> Option<(u32, u32)> {
    if active == 0 {
        return None;
    }
    let count = 4 - u32::from(mode != 1);
    for i in 0..count {
        if !check(i) {
            return None;
        }
    }
    Some((count, mode))
}

/// `0x00729120`: coordinate bounds guard.  `x`/`y` are the u16 values at
/// +1/+3 of the record and must be below `width`/`height`; then, when
/// `is_observer` is set, returns `(x, y, own_player, true)`; otherwise when
/// `cell_visible` is nonzero returns `(x, y, 0x11 + (local != own), false)`.
#[allow(clippy::too_many_arguments)]
pub fn w24a01_marker_args(
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    is_observer: bool,
    own_player: u8,
    local_player: u32,
    own_player_id: u32,
    cell_visible: bool,
) -> Option<(i32, i32, u8, bool)> {
    if x >= width || y >= height {
        return None;
    }
    if is_observer {
        return Some((i32::from(x as i16), i32::from(y as i16), own_player, true));
    }
    if cell_visible {
        let kind = 0x11 + u8::from(local_player != own_player_id);
        return Some((i32::from(x as i16), i32::from(y as i16), kind, false));
    }
    None
}

#[cfg(test)]
mod tests_wave24_agent01 {
    use super::*;

    #[test]
    fn msgs() {
        assert_eq!(w24a01_build_msg_21(7), [0x21, 7]);
        assert_eq!(w24a01_build_msg_36(), [0x36]);
    }

    #[test]
    fn dispatch() {
        let mut n = 0;
        for id in [1, 2, 4, 8, 0x17, 0x19] {
            assert!(w24a01_dispatch_cb(id, || n += 1));
        }
        assert!(!w24a01_dispatch_cb(3, || n += 1));
        assert_eq!(n, 6);
    }

    #[test]
    fn slot_free() {
        let slots = [
            (1, 0),
            (2, 3),
            (0, 5),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
        ];
        let valid = |h: u32| h != 0;
        assert!(!w24a01_player_slot_free(3, 0, &slots, valid));
        assert!(!w24a01_player_slot_free(3, 1, &slots, valid));
        assert!(w24a01_player_slot_free(5, 1, &slots, valid));
    }

    #[test]
    fn stop_all() {
        let h = [1, 2, 3, 4, 5, 6, 7, 8];
        let log = std::cell::RefCell::new(Vec::new());
        w24a01_stop_all_slots_then(
            &h,
            || log.borrow_mut().push(0),
            |x| log.borrow_mut().push(x),
            || log.borrow_mut().push(99),
        );
        let log = log.into_inner();
        assert_eq!(log.len(), 10);
        assert_eq!(log[1], 1);
        assert_eq!(log[9], 99);
    }

    #[test]
    fn probe() {
        assert_eq!(w24a01_probe_channels(0, 1, |_| true), None);
        assert_eq!(w24a01_probe_channels(1, 1, |_| true), Some((4, 1)));
        assert_eq!(w24a01_probe_channels(1, 2, |_| true), Some((3, 2)));
        assert_eq!(w24a01_probe_channels(1, 2, |i| i < 2), None);
    }

    #[test]
    fn marker() {
        assert_eq!(
            w24a01_marker_args(10, 10, 10, 20, true, 1, 0, 0, true),
            None
        );
        assert_eq!(
            w24a01_marker_args(3, 4, 10, 20, true, 2, 0, 0, false),
            Some((3, 4, 2, true))
        );
        assert_eq!(
            w24a01_marker_args(3, 4, 10, 20, false, 2, 1, 0, true),
            Some((3, 4, 0x12, false))
        );
        assert_eq!(
            w24a01_marker_args(3, 4, 10, 20, false, 2, 1, 0, false),
            None
        );
    }
}
