#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent04.rs
// ===========================================================================
// Wave 23 translation batch 04: small client-side predicates and guards.

/// `0x006cd9a0`: negated result of the callee (`result == 0`).
pub fn w23a04_not_char(callee_result: u8) -> bool {
    callee_result == 0
}

/// Outcome of the budget tick in `0x006cda20`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W23a04Tick {
    /// Budget still positive: returns 1.
    Continue,
    /// Budget exhausted, no pending request: budget restored, returns 0.
    Idle,
    /// Budget exhausted and the recheck flag was clear: flag set, then a
    /// non-returning call.
    SetFlagThenHalt,
    /// A pending request exists (flag already set) -> non-returning call.
    HaltFlagSet,
    /// A pending request exists (flag clear) -> flag cleared, then
    /// non-returning call.
    HaltFlagClear,
}

/// `0x006cda20`: subtracts 1000 from `budget`, zeroes `*out`, and decides
/// what to do once the budget drops below 1.  `pending` is the result of the
/// pending-request probe (non-zero means pending); `flag` is the byte global.
pub fn w23a04_budget_tick(
    out: &mut u32,
    budget: &mut i32,
    flag: &mut bool,
    pending: impl FnOnce() -> bool,
) -> W23a04Tick {
    *out = 0;
    *budget = budget.wrapping_sub(1000);
    if *budget >= 1 {
        return W23a04Tick::Continue;
    }
    if pending() {
        return if *flag {
            W23a04Tick::HaltFlagSet
        } else {
            *flag = false;
            W23a04Tick::HaltFlagClear
        };
    }
    *budget = budget.wrapping_add(1000);
    if !*flag {
        *flag = true;
        return W23a04Tick::SetFlagThenHalt;
    }
    W23a04Tick::Idle
}

/// `0x006cdc20`: true when `counter >= 1`, otherwise the fallback probe.
pub fn w23a04_counter_or_probe(counter: i32, probe: impl FnOnce() -> bool) -> bool {
    counter >= 1 || probe()
}

/// `0x0071ef40`: dispatch on the 16-bit kind at offset 0x10 of the record.
/// Kinds 0,1,3..=9,0xf,0x18 call the helper and return 1; every other kind
/// returns the helper's result.
pub fn w23a04_kind_dispatch(kind: u16, call: impl FnOnce() -> u32) -> u32 {
    let r = call();
    match kind {
        0 | 1 | 3..=9 | 0xf | 0x18 => 1,
        _ => r,
    }
}

/// `0x007208c0`: when both coordinate getters are not -1, transforms the
/// point `(x, y)` (x from the first getter, y from the second, both as i16)
/// and returns it; the caller then issues the two set calls.  Returns
/// `(x, y, transformed)`.
pub fn w23a04_cursor_point(
    first: i32,
    second: i32,
    transform: impl FnOnce(i32, i32) -> (u32, u32),
) -> Option<(i16, i16, (u32, u32))> {
    if first == -1 || second == -1 {
        return None;
    }
    let y = second as i16;
    let x = first as i16;
    Some((x, y, transform(x as i32, y as i32)))
}

/// `0x00724e00`: guard chain; true when the final lookup should be run.
/// `slot` is the byte at offset 1 of the packet, `max_slot` the limit
/// getter, `taken(slot)` the occupancy probe and `accept` the final check.
pub fn w23a04_slot_guard(
    have_table: bool,
    state: u8,
    host: bool,
    slot: u8,
    max_slot: u8,
    taken: impl FnOnce(u8) -> bool,
    accept: impl FnOnce() -> bool,
) -> bool {
    have_table && state > 3 && host && slot != 0 && slot <= max_slot && !taken(slot) && accept()
}

#[cfg(test)]
mod tests_wave23_agent04 {
    use super::*;

    #[test]
    fn not_char() {
        assert!(w23a04_not_char(0));
        assert!(!w23a04_not_char(1));
    }

    #[test]
    fn tick() {
        let (mut o, mut b, mut f) = (9, 2500, false);
        assert_eq!(
            w23a04_budget_tick(&mut o, &mut b, &mut f, || false),
            W23a04Tick::Continue
        );
        assert_eq!((o, b), (0, 1500));
        let (mut b, mut f) = (1000, false);
        assert_eq!(
            w23a04_budget_tick(&mut o, &mut b, &mut f, || false),
            W23a04Tick::SetFlagThenHalt
        );
        assert_eq!((b, f), (1000, true));
        assert_eq!(
            w23a04_budget_tick(&mut o, &mut b, &mut f, || false),
            W23a04Tick::Idle
        );
        assert_eq!(
            w23a04_budget_tick(&mut o, &mut b, &mut f, || true),
            W23a04Tick::HaltFlagSet
        );
        let mut f = true;
        let mut b = 0;
        let _ = &mut f;
        f = false;
        assert_eq!(
            w23a04_budget_tick(&mut o, &mut b, &mut f, || true),
            W23a04Tick::HaltFlagClear
        );
    }

    #[test]
    fn counter() {
        assert!(w23a04_counter_or_probe(1, || false));
        assert!(!w23a04_counter_or_probe(0, || false));
        assert!(w23a04_counter_or_probe(-3, || true));
    }

    #[test]
    fn kind() {
        for k in [0u16, 1, 3, 4, 9, 0xf, 0x18] {
            assert_eq!(w23a04_kind_dispatch(k, || 7), 1);
        }
        for k in [2u16, 0xa, 0x10, 0x19] {
            assert_eq!(w23a04_kind_dispatch(k, || 7), 7);
        }
    }

    #[test]
    fn cursor() {
        assert!(w23a04_cursor_point(-1, 2, |_, _| (0, 0)).is_none());
        assert!(w23a04_cursor_point(1, -1, |_, _| (0, 0)).is_none());
        assert_eq!(
            w23a04_cursor_point(3, 4, |x, y| (x as u32, y as u32)),
            Some((3, 4, (3, 4)))
        );
    }

    #[test]
    fn guard() {
        let g = |s, m, t| w23a04_slot_guard(true, 4, true, s, m, |_| t, || true);
        assert!(g(1, 1, false));
        assert!(!g(0, 1, false));
        assert!(!g(2, 1, false));
        assert!(!g(1, 1, true));
        assert!(!w23a04_slot_guard(true, 3, true, 1, 1, |_| false, || true));
    }
}
