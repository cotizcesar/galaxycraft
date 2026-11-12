#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent02.rs
// ===========================================================================
// Wave 24 batch 02: small client-side helpers.

/// `0x0073f0e0`: swaps a global value, returning the previous one.
pub fn w24a02_swap_global(slot: &mut u32, new_value: u32) -> u32 {
    std::mem::replace(slot, new_value)
}

/// `0x007494e0`: true when the flag is set, the game is in the
/// 0x80..0x83 mode range (or `active_session != 0`), and `blocker == 0`.
pub fn w24a02_session_ready(flag: u8, active_session: u32, mode: u32, blocker: u32) -> bool {
    if flag != 0 && (active_session != 0 || mode.wrapping_sub(0x80) < 4) {
        return blocker == 0;
    }
    false
}

/// `0x00744740`: returns 8 when no slot is found, else the mapped value;
/// `None` when the session is absent (non-returning path in retail).
pub fn w24a02_slot_or_none(
    has_session: bool,
    find: impl Fn() -> u8,
    map: impl Fn(u8) -> u32,
) -> Option<u32> {
    if !has_session {
        return None;
    }
    let c = find();
    Some(if c == 0 { 8 } else { map(c) })
}

/// `0x00744d20`: like the slot lookup, then indexes an 8-entry byte table;
/// values above 7 yield 0.  `None` when the session is absent.
pub fn w24a02_slot_table_value(
    has_session: bool,
    find: impl Fn() -> u8,
    map: impl Fn(u8) -> u8,
    table: &[u8; 8],
) -> Option<u8> {
    if !has_session {
        return None;
    }
    let c = find();
    let v = if c == 0 { 8 } else { map(c) };
    Some(if v > 7 { 0 } else { table[v as usize] })
}

/// `0x0073bb30`: for `slot <= 7` runs the first callback, then the second
/// when the flag callback reports true; `None` when `slot > 7`
/// (non-returning in retail).
pub fn w24a02_apply_slot(
    slot: u32,
    first: impl FnOnce(u32),
    flag: impl FnOnce() -> bool,
    second: impl FnOnce(),
) -> Option<()> {
    if slot > 7 {
        return None;
    }
    first(slot);
    if flag() {
        second();
    }
    Some(())
}

#[cfg(test)]
mod tests_wave24_agent02 {
    use super::*;

    #[test]
    fn swap_and_ready() {
        let mut g = 5;
        assert_eq!(w24a02_swap_global(&mut g, 9), 5);
        assert_eq!(g, 9);
        assert!(w24a02_session_ready(1, 0, 0x83, 0));
        assert!(!w24a02_session_ready(1, 0, 0x84, 0));
        assert!(!w24a02_session_ready(1, 1, 0, 1));
        assert!(!w24a02_session_ready(0, 1, 0x80, 0));
    }

    #[test]
    fn slot_lookups() {
        assert_eq!(w24a02_slot_or_none(false, || 1, |c| c as u32), None);
        assert_eq!(w24a02_slot_or_none(true, || 0, |c| c as u32), Some(8));
        assert_eq!(w24a02_slot_or_none(true, || 3, |c| c as u32 + 1), Some(4));
        let t = [10, 11, 12, 13, 14, 15, 16, 17];
        assert_eq!(w24a02_slot_table_value(true, || 0, |c| c, &t), Some(0));
        assert_eq!(w24a02_slot_table_value(true, || 2, |c| c, &t), Some(12));
        assert_eq!(w24a02_slot_table_value(true, || 2, |c| c + 6, &t), Some(0));
        assert_eq!(w24a02_slot_table_value(false, || 2, |c| c, &t), None);
    }

    #[test]
    fn apply_slot() {
        let mut log = vec![];
        assert!(w24a02_apply_slot(8, |_| log.push(1), || true, || {}).is_none());
        assert!(log.is_empty());
        let n = std::cell::Cell::new(0);
        assert!(
            w24a02_apply_slot(7, |s| n.set(n.get() + s), || true, || n.set(n.get() + 100))
                .is_some()
        );
        assert_eq!(n.get(), 107);
    }
}
