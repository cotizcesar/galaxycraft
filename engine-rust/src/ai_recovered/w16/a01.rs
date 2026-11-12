#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave16/agent01.rs
// ===========================================================================
// Wave 16 translation batch 01.
//
// Pure helpers; unresolved callees are closure parameters.

/// Resource-style triple used by the per-player accumulators (+0x1f8/+0x1fc/+0x200 of
/// the 0x4e8-byte player record).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct W16a01Tally {
    pub a: i32,
    pub b: i32,
    pub c: u32,
}

/// Slot block of a record: five u16 slots at +0x98 and a start index byte at +0xa4.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct W16a01SlotBlock {
    pub slots: [u16; 5],
    pub start: u8,
}

/// Wrapped search for slot value `needle` beginning at `start` (index >= 5 wraps to 0).
fn w16a01_find_slot(b: &W16a01SlotBlock, needle: u16) -> Option<usize> {
    let mut idx = b.start as usize;
    for _ in 0..5 {
        if idx >= 5 {
            idx = 0;
        }
        if b.slots[idx] == needle {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

/// 0x0058e0e0: replace the first free (0xe4) slot, scanning from the start index.
/// `commit(slot_is_start)` is the unresolved reservation call (returns false on
/// failure); `flag_bit0` is bit 0 of the per-kind table entry for `value`;
/// `on_no_flag` / `on_done` are the trailing callees.
pub fn w16a01_replace_empty_slot(
    b: &mut W16a01SlotBlock,
    value: u16,
    flag_bit0: bool,
    mut commit: impl FnMut(bool) -> bool,
    mut on_no_flag: impl FnMut(),
    mut on_done: impl FnMut(),
) -> bool {
    let Some(idx) = w16a01_find_slot(b, 0xe4) else {
        return false;
    };
    if !commit(idx == b.start as usize) {
        return false;
    }
    b.slots[idx] = value;
    if !flag_bit0 {
        on_no_flag();
    }
    on_done();
    true
}

/// 0x005b6680: store a u16 at +0xc4.
pub fn w16a01_set_c4(field: &mut u16, v: u16) {
    *field = v;
}

/// 0x005b7c70: set byte +0xa6 and clear +0xe8, +0xec (u32) and +0xe2 (u8).
pub fn w16a01_set_a6_reset(a6: &mut u8, e8: &mut u32, ec: &mut u32, e2: &mut u8, v: u8) {
    *a6 = v;
    *e8 = 0;
    *ec = 0;
    *e2 = 0;
}

/// 0x005baa30: true when byte +0x4d is 0x1f, `pred1()` holds, +0x5c is non-zero and
/// `pred2()` is false.
pub fn w16a01_state1f_ready(
    b4d: u8,
    f5c: u32,
    pred1: impl FnOnce() -> bool,
    pred2: impl FnOnce() -> bool,
) -> bool {
    b4d == 0x1f && pred1() && f5c != 0 && !pred2()
}

/// 0x006358d0: bit 4 of the player flags word, 0 when player >= 8.
pub fn w16a01_player_flag4(player: u32, flags: u16) -> u16 {
    if player > 7 { 0 } else { (flags >> 4) & 1 }
}

/// 0x00635eb0: inverted bit 2 of the player flags word.
pub fn w16a01_player_not_flag2(flags: u16) -> u16 {
    !(flags >> 2) & 1
}

/// 0x00634eb0: add the (a, b, c) triple for `kind` into the tally. Returns the
/// arguments for the follow-up call (event code 8).
pub fn w16a01_tally_add(t: &mut W16a01Tally, triple: (i32, i32, u8)) -> u32 {
    t.a = t.a.wrapping_add(triple.0);
    t.b = t.b.wrapping_add(triple.1);
    t.c = t.c.wrapping_add(triple.2 as u32);
    8
}

/// 0x00634fc0: rate-limited tally. `last` is +0x41c of the player record, `now` the
/// frame counter, `kind` the u16 at +0x10 of the subject. Returns the effective kind
/// (0x1e maps to 5) when applied; the caller then issues events with code 2 and
/// weight 0x32. `None` when still inside the 0x1d-frame window.
pub fn w16a01_tally_rate_limited(
    t: &mut W16a01Tally,
    last: u32,
    now: u32,
    kind: u16,
    triple_of: impl FnOnce(u16) -> (i32, i32, u8),
) -> Option<u16> {
    if last != 0 && now.wrapping_sub(last) <= 0x1d {
        return None;
    }
    let k = if kind == 0x1e { 5 } else { kind };
    w16a01_tally_add(t, triple_of(k));
    Some(k)
}

/// 0x005c7a20: mask chosen for the lookup; 0x120 when `alt`, else 0x400 doubled to
/// 0x800 when `flag` (callee result) is set.
pub fn w16a01_mask_c7a20(alt: bool, flag: bool) -> u32 {
    if alt {
        0x120
    } else {
        (if flag { 0x400 } else { 0 }) + 0x400
    }
}

/// 0x005c79a3: mask 0x800 or 0x1000 depending on `flag`.
pub fn w16a01_mask_c79a3(flag: bool) -> u32 {
    (if flag { 0x800 } else { 0 }) + 0x800
}

/// 0x005c79d0: base mask is 0x120 when `alt`, else 0x800. `lookup(mask, which)` is
/// the unresolved search (which 0 = first handler, 1 = fallback); the fallback runs
/// only when `!alt` and the first returned 0.
pub fn w16a01_c79d0(alt: bool, mut lookup: impl FnMut(u32, u8) -> i32) {
    let mask = if alt { 0x120 } else { 0x800 };
    if lookup(mask, 0) == 0 && !alt {
        lookup(mask, 1);
    }
}

/// 0x0053bfd0: argument shuffle (p1, p3, p2, p3) for the wrapped call.
pub fn w16a01_args_bfd0(p1: u32, p2: u32, p3: u32) -> (u32, u32, u32, u32) {
    (p1, p3, p2, p3)
}

/// 0x006574c5: run `act` only when `pred()` is zero.
pub fn w16a01_unless(pred: impl FnOnce() -> i32, act: impl FnOnce()) {
    if pred() == 0 {
        act();
    }
}

// FUN_0054b6e0: noreturn call
// FUN_005525a0: SEH + noreturn path
// FUN_00597290: red-black tree erase fixup, STL, skipped for size/cost
// FUN_005c7870: bare call
// FUN_00636960: noreturn call
// FUN_00657260: opaque callees, stack-cookie noreturn
// FUN_00658240: noreturn, opaque callees

#[cfg(test)]
mod tests_wave16_agent01 {
    use super::*;

    #[test]
    fn slot_replace() {
        let mut b = W16a01SlotBlock {
            slots: [1, 0xe4, 2, 0xe4, 3],
            start: 2,
        };
        let n = std::cell::Cell::new(0);
        assert!(w16a01_replace_empty_slot(
            &mut b,
            9,
            false,
            |s| !s,
            || n.set(n.get() + 1),
            || n.set(n.get() + 10)
        ));
        assert_eq!(b.slots, [1, 0xe4, 2, 9, 3]);
        assert_eq!(n.get(), 11);
        let mut full = W16a01SlotBlock {
            slots: [1; 5],
            start: 0,
        };
        assert!(!w16a01_replace_empty_slot(
            &mut full,
            9,
            true,
            |_| true,
            || {},
            || {}
        ));
        let mut r = W16a01SlotBlock {
            slots: [0xe4, 1, 1, 1, 1],
            start: 0,
        };
        assert!(!w16a01_replace_empty_slot(
            &mut r,
            9,
            true,
            |s| !s,
            || {},
            || {}
        ));
        assert_eq!(r.slots[0], 0xe4);
    }

    #[test]
    fn flags() {
        assert_eq!(w16a01_player_flag4(8, 0xff), 0);
        assert_eq!(w16a01_player_flag4(3, 0x10), 1);
        assert_eq!(w16a01_player_not_flag2(0b100), 0);
        assert_eq!(w16a01_player_not_flag2(0), 1);
    }

    #[test]
    fn tally() {
        let mut t = W16a01Tally::default();
        assert_eq!(
            w16a01_tally_rate_limited(&mut t, 100, 110, 5, |_| (1, 2, 3)),
            None
        );
        let k = w16a01_tally_rate_limited(&mut t, 100, 200, 0x1e, |k| (k as i32, 2, 3));
        assert_eq!(k, Some(5));
        assert_eq!(t, W16a01Tally { a: 5, b: 2, c: 3 });
        assert_eq!(w16a01_tally_add(&mut t, (1, 1, 1)), 8);
    }

    #[test]
    fn misc() {
        assert!(w16a01_state1f_ready(0x1f, 1, || true, || false));
        assert!(!w16a01_state1f_ready(0x1e, 1, || true, || false));
        assert_eq!(w16a01_mask_c7a20(true, true), 0x120);
        assert_eq!(w16a01_mask_c7a20(false, true), 0x800);
        assert_eq!(w16a01_mask_c7a20(false, false), 0x400);
        assert_eq!(w16a01_mask_c79a3(true), 0x1000);
        let mut calls = vec![];
        w16a01_c79d0(false, |m, w| {
            calls.push((m, w));
            0
        });
        assert_eq!(calls, vec![(0x800, 0), (0x800, 1)]);
        assert_eq!(w16a01_args_bfd0(1, 2, 3), (1, 3, 2, 3));
        let (mut a, mut e8, mut ec, mut e2) = (0, 5, 6, 7);
        w16a01_set_a6_reset(&mut a, &mut e8, &mut ec, &mut e2, 4);
        assert_eq!((a, e8, ec, e2), (4, 0, 0, 0));
        let mut f = 0;
        w16a01_set_c4(&mut f, 7);
        assert_eq!(f, 7);
        let mut hit = false;
        w16a01_unless(|| 0, || hit = true);
        assert!(hit);
    }
}
