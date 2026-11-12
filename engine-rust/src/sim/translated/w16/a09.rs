#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave16/agent09.rs
// ===========================================================================
// Wave 16 translation batch 09 (staging).
//
// Window: queue: 20 addresses, module `sim.rs`.
//
// Skipped (reason):
// - FUN_007285a0: SEH message loop, unresolved dispatch.
// - FUN_007288e0: SEH message loop, unresolved dispatch.
// - FUN_007289e0: SEH message loop, unresolved dispatch.
// - FUN_0072b450: SEH message loop, unresolved dispatch.
// - FUN_00753f80: ends in noreturn call.
// - FUN_00767d50: noreturn, unresolved callees.
// - FUN_0076c240: noreturn paths, unresolved callees.
// - FUN_0076c5a0: noreturn call chain.

/// Number of player slots scanned by the 0x11b_1fa8 table loops (12 dwords).
pub const W16A09_SLOT_COUNT: usize = 12;

/// 0x007287f0: iterate a list; for each node where `pred` holds run `action`.
pub fn w16a09_iterate_and_dispatch<T: Copy>(
    nodes: &[T],
    mut pred: impl FnMut(T) -> bool,
    mut action: impl FnMut(T),
) {
    for &n in nodes {
        if pred(n) {
            action(n);
        }
    }
}

/// 0x0072ac30: guarded handler; returns true when the action ran (the
/// caller then runs the follow-up call). `owner` is byte at +0x4c of the
/// node, `local_player` the global compared against it.
pub fn w16a09_guarded_action_a(
    has_node: bool,
    next_is_null: bool,
    cond_a: bool,
    cond_b: bool,
    owner: u8,
    local_player: u8,
) -> bool {
    has_node && next_is_null && cond_a && cond_b && owner == local_player
}

/// 0x0072ae10: same shape as the sibling guarded handler (different cond_a).
pub fn w16a09_guarded_action_b(
    has_node: bool,
    next_is_null: bool,
    cond_a: bool,
    cond_b: bool,
    owner: u8,
    local_player: u8,
) -> bool {
    w16a09_guarded_action_a(has_node, next_is_null, cond_a, cond_b, owner, local_player)
}

/// 0x0074d140: scan the 12 slots; returns 1 when an occupied slot's
/// check (on `word << 8`) is zero, else 0 (the error report is the caller's
/// job).  `words` is the u16 table at 0x00f8_dc10 indexed by `idx`.
pub fn w16a09_slot_scan_ok(
    words: &[u16],
    idx: u8,
    slots: &[i32; W16A09_SLOT_COUNT],
    mut check: impl FnMut(u32) -> i32,
) -> u32 {
    let w = words[idx as usize] as u32;
    for &s in slots {
        if s != 0 && check(w << 8) == 0 {
            return 1;
        }
    }
    0
}

/// 0x00752310: sets the flag byte when both predicates hold for `id`.
pub fn w16a09_set_flag_if(
    id: u8,
    pred_a: impl Fn(u8) -> bool,
    pred_b: impl Fn(u8) -> bool,
    flag: &mut u8,
) {
    if pred_a(id) && pred_b(id) {
        *flag = 1;
    }
}

/// 0x0075dab0: compare the u16 at offset 0x449fe + idx*0x40 in the
/// table `base` for two indices (already masked to 16 bits).
pub fn w16a09_table_entries_equal(base: &[u8], idx_a: u32, idx_b: u32) -> bool {
    let rd = |i: u32| {
        let o = (i & 0xffff) as usize * 0x40 + 0x449fe;
        i16::from_le_bytes([base[o], base[o + 1]])
    };
    rd(idx_a) == rd(idx_b)
}

/// 0x0076be20: returns true when `has_entry` is false, else defers to
/// `compare`.
pub fn w16a09_entry_or_compare(has_entry: bool, compare: impl FnOnce() -> bool) -> bool {
    if !has_entry {
        return true;
    }
    compare()
}

/// 0x00780730: true when some occupied slot has neither predicate set.
pub fn w16a09_any_slot_neither(
    slots: &[i32; W16A09_SLOT_COUNT],
    mut p1: impl FnMut() -> bool,
    mut p2: impl FnMut() -> bool,
) -> bool {
    for &s in slots {
        if s != 0 && !p1() && !p2() {
            return true;
        }
    }
    false
}

/// 0x00780770: true when some occupied slot has either predicate set.
pub fn w16a09_any_slot_either(
    slots: &[i32; W16A09_SLOT_COUNT],
    mut p1: impl FnMut() -> bool,
    mut p2: impl FnMut() -> bool,
) -> bool {
    for &s in slots {
        if s != 0 && (p1() || p2()) {
            return true;
        }
    }
    false
}

fn w16a09_bit_offset(bit: u8, row: i32) -> usize {
    (bit >> 3) as usize + (row.wrapping_add(0x2ae0) as usize).wrapping_mul(6)
}

/// 0x00781500: clear bit `bit` in the 6-byte row `row + 0x2ae0` of `bits`.
pub fn w16a09_clear_row_bit(bits: &mut [u8], bit: u8, row: i32) {
    bits[w16a09_bit_offset(bit, row)] &= !(1u8 << (bit & 7));
}

/// 0x00781600: set bit `bit` in the 6-byte row `row + 0x2ae0` of `bits`.
pub fn w16a09_set_row_bit(bits: &mut [u8], bit: u8, row: i32) {
    bits[w16a09_bit_offset(bit, row)] |= 1u8 << (bit & 7);
}

/// 0x007505d0: returns Some(new handle) when the spawn succeeded; the
/// caller then runs the follow-ups. `lookup` yields a position, `spawn`
/// creates the object.
pub fn w16a09_spawn_with_lookup<P, H>(
    lookup: impl FnOnce() -> Option<P>,
    spawn: impl FnOnce(P) -> Option<H>,
) -> Option<H> {
    let p = lookup()?;
    spawn(p)
}

#[cfg(test)]
mod tests_wave16_agent09 {
    use super::*;

    #[test]
    fn bits() {
        let mut b = vec![0u8; 0x2ae0 * 6 + 64];
        w16a09_set_row_bit(&mut b, 9, -0x2ae0 + 1);
        assert_eq!(b[7], 2);
        w16a09_clear_row_bit(&mut b, 9, -0x2ae0 + 1);
        assert_eq!(b[7], 0);
    }

    #[test]
    fn slots() {
        let mut s = [0; W16A09_SLOT_COUNT];
        assert!(!w16a09_any_slot_either(&s, || true, || true));
        s[3] = 1;
        assert!(w16a09_any_slot_either(&s, || false, || true));
        assert!(w16a09_any_slot_neither(&s, || false, || false));
        assert!(!w16a09_any_slot_neither(&s, || true, || false));
        assert_eq!(w16a09_slot_scan_ok(&[5], 0, &s, |v| (v != 0x500) as i32), 1);
        assert_eq!(w16a09_slot_scan_ok(&[5], 0, &s, |_| 1), 0);
    }

    #[test]
    fn misc() {
        let mut base = vec![0u8; 0x449fe + 0x40 * 3];
        base[0x449fe + 0x40] = 7;
        base[0x449fe + 0x80] = 7;
        assert!(w16a09_table_entries_equal(&base, 1, 2));
        assert!(!w16a09_table_entries_equal(&base, 0, 2));
        assert!(w16a09_entry_or_compare(false, || false));
        assert!(!w16a09_entry_or_compare(true, || false));
        let mut f = 0;
        w16a09_set_flag_if(1, |_| true, |_| true, &mut f);
        assert_eq!(f, 1);
        assert!(w16a09_guarded_action_b(true, true, true, true, 2, 2));
        assert!(!w16a09_guarded_action_a(true, true, true, true, 1, 2));
        assert_eq!(
            w16a09_spawn_with_lookup(|| Some(1), |p| Some(p + 1)),
            Some(2)
        );
        let mut n = 0;
        w16a09_iterate_and_dispatch(&[1, 2, 3], |x| x > 1, |_| n += 1);
        assert_eq!(n, 2);
    }
}
