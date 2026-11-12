#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave16/agent03.rs
// ===========================================================================
// Wave 16 translation batch 03.
//
// Pure helpers; unresolved callees are closure/bool parameters.
//
// Skipped (reason): FUN_005359c0 noreturn, FUN_00538a60 noreturn,
// FUN_00539110 noreturn alloc, FUN_0055baf0 bare call chain, FUN_00567a80
// flag plus call, FUN_00567bc0 flag plus call, FUN_00568f30 noreturn,
// FUN_00588e7e callee-only list setup, FUN_0059cf66 noreturn,
// FUN_005a5c50 noreturn, FUN_005a5ec0 bare call chain, FUN_005c5770
// noreturn, FUN_006560f0 noreturn callee, FUN_00658160 callee-driven list
// walk, FUN_00677db0 STL tree erase, FUN_006960f0 noreturn.

/// Outcome of the tail decision in 0x005d95c0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W16a03Action {
    /// Nothing happens.
    None,
    /// Calls the 0x57_ccd0 path.
    Release,
    /// Calls the 0x57_8d90 path with the looked-up byte.
    Apply(u8),
}

/// 0x00566430: per-race/slot statistics accumulator.
///
/// `counts` and `totals` hold 12 rows of 4 columns (flat, 48 entries).
/// Column 1 is also mirrored into column 0 of the same row.  `value` is
/// treated as unsigned for the float totals (table at 0x00d3_3180 adds
/// 2^32 for negative inputs).  Out-of-range or disabled input is ignored.
pub fn w16a03_stat_accumulate(
    counts: &mut [i32; 48],
    totals: &mut [f32; 48],
    enabled: bool,
    row: u32,
    col: i32,
    value: i32,
) {
    if !(row < 12 && col < 4 && enabled) || col < 0 {
        return;
    }
    let idx = col as usize + row as usize * 4;
    let base = row as usize * 4;
    counts[idx] = counts[idx].wrapping_add(value);
    if col == 1 {
        counts[base] = counts[base].wrapping_add(value);
    }
    let v = value as u32 as f64;
    totals[idx] = (totals[idx] as f64 + v) as f32;
    if col == 1 {
        totals[base] = (totals[base] as f64 + v) as f32;
    }
}

/// 0x0057d6c0: true when the byte at +10 of the owner record is 10 (`'\n'`),
/// in which case the 0x57_49b0 path runs.
pub fn w16a03_is_newline_state(byte10: u8) -> bool {
    byte10 == b'\n'
}

/// 0x005d95c0: decrement the counter at +0xf and choose the follow-up.
///
/// `blocked_a`/`blocked_b` are the results of the two guard callees,
/// `flags_c` = byte +0xc, `flags_e` = byte +0xe, `mask` = low byte of
/// the 0x0119_f7c0 global, `looked_up` = the byte read from the player table.
pub fn w16a03_counter_step(
    blocked_a: bool,
    blocked_b: bool,
    counter: &mut u8,
    flags_c: u8,
    flags_e: u8,
    mask: u8,
    looked_up: u8,
) -> W16a03Action {
    if blocked_a || blocked_b || *counter == 0 {
        return W16a03Action::None;
    }
    *counter -= 1;
    if flags_c & mask == 0 {
        *counter = 0;
    }
    let c = *counter;
    if c & 8 == 0 && (c != 0 || flags_e & 8 == 0) {
        W16a03Action::Release
    } else {
        W16a03Action::Apply(looked_up)
    }
}

/// 0x007ab9a0 (partial): the 8-slot countdown loop and the periodic
/// tickers.  Returns true when the 30-tick refresh counter fired.
///
/// `slot_active`/`slot_left` model the 8 byte flags and u32 timers
/// (0xffff_ffff means infinite).  `refresh` is the counter that is reset
/// to 0x1e, `fast` the 0xf counter, `half` the 0x2d counter.  `redraw`
/// is raised when a slot expires.  `fast_fired`/`half_fired` report the
/// other tickers (callers run their callees).
#[allow(clippy::too_many_arguments)]
pub fn w16a03_timer_tick(
    dt: u32,
    slot_active: &mut [bool; 8],
    slot_left: &mut [u32; 8],
    refresh: &mut i32,
    fast: &mut i32,
    half: &mut i32,
    fast_fired: &mut bool,
    half_fired: &mut bool,
) -> bool {
    *half_fired = false;
    let h = *half == 0;
    *half = half.wrapping_sub(1);
    if h {
        *half = 0x2d;
        *half_fired = true;
    }
    let mut redraw = false;
    for i in 0..8 {
        if slot_active[i] && slot_left[i] != u32::MAX {
            if dt < slot_left[i] {
                slot_left[i] -= dt;
            } else {
                slot_active[i] = false;
                slot_left[i] = 0;
                redraw = true;
            }
        }
    }
    if redraw {
        *refresh = 1;
    }
    let f = *fast == 0;
    *fast = fast.wrapping_sub(1);
    *fast_fired = f;
    if f {
        *fast = 0xf;
    }
    let r = *refresh == 0;
    *refresh = refresh.wrapping_sub(1);
    if r {
        *refresh = 0x1e;
    }
    r
}

#[cfg(test)]
mod tests_wave16_agent03 {
    use super::*;

    #[test]
    fn stat_mirrors_col1() {
        let mut c = [0; 48];
        let mut t = [0.0; 48];
        w16a03_stat_accumulate(&mut c, &mut t, true, 2, 1, 5);
        assert_eq!((c[9], c[8]), (5, 5));
        assert_eq!((t[9], t[8]), (5.0, 5.0));
        w16a03_stat_accumulate(&mut c, &mut t, true, 12, 1, 5);
        w16a03_stat_accumulate(&mut c, &mut t, false, 1, 2, 5);
        assert_eq!(c.iter().sum::<i32>(), 10);
        w16a03_stat_accumulate(&mut c, &mut t, true, 0, 2, -1);
        assert_eq!(c[2], -1);
        assert_eq!(t[2], 4294967295.0f32);
    }

    #[test]
    fn newline() {
        assert!(w16a03_is_newline_state(10));
        assert!(!w16a03_is_newline_state(9));
    }

    #[test]
    fn counter() {
        let mut n = 0u8;
        assert_eq!(
            w16a03_counter_step(false, false, &mut n, 1, 0, 1, 7),
            W16a03Action::None
        );
        let mut n = 2u8;
        assert_eq!(
            w16a03_counter_step(false, false, &mut n, 1, 0, 1, 7),
            W16a03Action::Release
        );
        assert_eq!(n, 1);
        let mut n = 2u8;
        w16a03_counter_step(false, false, &mut n, 0, 0, 1, 7);
        assert_eq!(n, 0);
        let mut n = 9u8;
        assert_eq!(
            w16a03_counter_step(false, false, &mut n, 1, 0, 1, 7),
            W16a03Action::Apply(7)
        );
        let mut n = 1u8;
        assert_eq!(
            w16a03_counter_step(false, false, &mut n, 1, 8, 1, 7),
            W16a03Action::Apply(7)
        );
        assert_eq!(
            w16a03_counter_step(true, false, &mut n, 1, 8, 1, 7),
            W16a03Action::None
        );
    }

    #[test]
    fn ticks() {
        let mut a = [false; 8];
        let mut l = [0u32; 8];
        a[0] = true;
        l[0] = 3;
        a[1] = true;
        l[1] = u32::MAX;
        let (mut r, mut f, mut h) = (5, 1, 1);
        let (mut ff, mut hf) = (false, false);
        let fired = w16a03_timer_tick(5, &mut a, &mut l, &mut r, &mut f, &mut h, &mut ff, &mut hf);
        assert!(!a[0] && a[1]);
        assert!(!fired);
        assert_eq!((r, hf, ff), (0, false, false));
    }
}
