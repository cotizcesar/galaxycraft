#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave16/agent06.rs
// ===========================================================================
// Wave 16 translation batch 06.
//
// Skipped (reason):
// - FUN_00597430: STL map insert-unique helper (tree walk), pure container glue.
// - FUN_005a28d6, FUN_005a5ce0, FUN_005ac3a0, FUN_005aff90, FUN_005b74a0,
//   FUN_005bad60, FUN_005956d0: noreturn tail or SEH funclet chain.
// - FUN_005b16f0, FUN_005b1700: bare call to unresolved helper with globals.
// - FUN_005b3040: vector resize, ends in noreturn memmove/alloc calls.

/// 0x0058deb0: ring of five u16 values at +0x98 starting at index byte +0xa4;
/// returns `CONCAT22(high16(idx/5), ring[idx%5])` with C signed division.
pub fn w16a06_ring5_lookup(base: u8, ring: &[u16; 5], offset: i32) -> u32 {
    let idx = (base as i32).wrapping_add(offset);
    let q = idx / 5;
    let r = (idx % 5) as usize; // may be negative in C; caller keeps offset >= 0
    (((q as u32) >> 16) << 16) | ring[r] as u32
}

/// 0x00590390: if `target` exists and the helper accepts, pick one of two
/// follow-up actions.  `target_ptr_fc` is the pointer at +0xfc (0 = none),
/// `target_is_self` is `ptr_fc == target`, `f8/fa` the shorts at +0xf8/+0xfa,
/// `same_owner` compares byte +0x4c of this and `ptr_fc`.
/// Returns Some((kind, a, b)): kind 0x31 -> FUN_005b7dd0(0x31, ptr), kind 6 ->
/// FUN_005b7d80(6, f8, fa).
pub fn w16a06_target_action(
    target_present: bool,
    gate: bool,
    ptr_fc: i32,
    target_ptr: i32,
    f8: i16,
    fa: i16,
    same_owner: bool,
) -> Option<(u8, i32, i32)> {
    if !target_present || !gate || ptr_fc == target_ptr || f8 == 0 {
        return None;
    }
    if ptr_fc != 0 && same_owner {
        return Some((0x31, ptr_fc, 0));
    }
    Some((6, f8 as i32, fa as i32))
}

/// 0x00591aa0: walk two linked lists (count at +200 / head +0xc0, count at
/// +0xc9 / head +0xc4, next link at node+0xc4) and return true when the
/// predicate accepts any node.
pub fn w16a06_any_in_lists<N: Copy>(
    count1: u8,
    head1: N,
    count2: u8,
    head2: N,
    mut next: impl FnMut(N) -> N,
    mut pred: impl FnMut(N) -> bool,
) -> bool {
    let (mut c, mut n) = (count1, head1);
    while c != 0 {
        c = c.wrapping_sub(1);
        if pred(n) {
            return true;
        }
        n = next(n);
    }
    let (mut c, mut n) = (count2, head2);
    while c != 0 {
        c = c.wrapping_sub(1);
        if pred(n) {
            return true;
        }
        n = next(n);
    }
    false
}

/// 0x0059f030: dispatch on flag bit 2 of byte +0x62.  Returns true when the
/// "flagged" path (FUN_0059f090 style) is taken; the closures receive
/// (byte +0x60, byte +0x21, dword +0x64) and, for the other path, also byte +0x4c.
pub fn w16a06_dispatch_by_flag(
    flags62: u8,
    b60: u8,
    b21: u8,
    d64: u32,
    b4c: u8,
    mut flagged: impl FnMut(u8, u8, u32),
    mut other: impl FnMut(u8, u8, u32, u8),
) -> bool {
    if flags62 & 2 != 0 {
        flagged(b60, b21, d64);
        true
    } else {
        other(b60, b21, d64, b4c);
        false
    }
}

/// Output of [`w16a06_recompute_rate`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W16a06Rate {
    pub field_60: u32,
    pub field_aa: u16,
}

/// 0x005b3980: recompute (+0x60, +0xaa) for the unit type id at +0x64.
/// `active` is the byte flag table entry, `table_val` the u16 from the
/// parallel table, `flags_dc` the dword at +0xdc, `divisor` = FUN_0058df30(id).
/// Returns None when nothing is written.
pub fn w16a06_recompute_rate(
    active: bool,
    flags_dc: u32,
    table_val: u16,
    divisor: u32,
) -> Option<W16a06Rate> {
    if !active || flags_dc & 2 == 0 {
        return None;
    }
    let f60 = ((table_val as u64 * 0x00cc_cccc_cd00_u64) >> 0x23) as u32;
    let mut aa = 1u16;
    let num = (table_val as u32).wrapping_mul(0x100).wrapping_sub(f60);
    if let Some(v) = num
        .checked_div(divisor)
        .map(|v| v as u16)
        .filter(|&v| v != 0)
    {
        aa = v;
    }
    Some(W16a06Rate {
        field_60: f60,
        field_aa: aa,
    })
}

/// 0x005b61f0: true unless the helper result is zero and the id at +100 is not 0xbc.
pub fn w16a06_is_special(helper_result: bool, id: u16) -> bool {
    helper_result || id == 0xbc
}

/// 0x005b6480: mask the flag byte at (ptr at +0xc)+0xc with `mask`.
pub fn w16a06_masked_flag(flag_byte: u8, mask: u8) -> u8 {
    flag_byte & mask
}

/// Result of [`w16a06_clamped_click`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W16a06Click {
    pub x: i16,
    pub y: i16,
    pub issued: bool,
}

/// 0x005baa60: clamp (x, y) into the map-view rectangle then try to issue
/// command 0x69; `lo_x`/`lo_y` = DAT mins, `view_w/h` = view extents minus
/// offsets (`hi_x = w - off_x`, `hi_y = h - off_y`).
/// The clamp keeps C semantics: values below the minimum snap to the minimum,
/// values at or above the maximum snap to max-1 (x) / max-0x21 (y, max already -0x20).
#[allow(clippy::too_many_arguments)]
pub fn w16a06_clamped_click(
    x: i16,
    y: i16,
    min_x: i16,
    min_y: i16,
    w: u16,
    off_x: i16,
    h: u16,
    off_y: i16,
    issue: impl FnOnce(i16, i16) -> bool,
) -> W16a06Click {
    let mut cx = min_x;
    if min_x <= x {
        cx = x;
        if (w as i32 - off_x as i32) <= x as i32 {
            cx = (w as i16).wrapping_sub(off_x).wrapping_sub(1);
        }
    }
    let mut cy = min_y;
    if min_y <= y {
        cy = y;
        if (h as i32 - off_y as i32 - 0x20) <= y as i32 {
            cy = (h as i16).wrapping_sub(off_y).wrapping_sub(0x21);
        }
    }
    let issued = issue(cx, cy);
    W16a06Click {
        x: cx,
        y: cy,
        issued,
    }
}

/// 0x005bb7c0: if the helper accepts, scroll by (cur - prev) and flag a
/// redraw when either coordinate changed 32-pixel cell.  Returns
/// (result, redraw_flag_set).
pub fn w16a06_scroll_step(
    accepted: bool,
    cur: (i16, i16),
    prev: (i16, i16),
    mut scroll: impl FnMut(i32, i32),
) -> (bool, bool) {
    if !accepted {
        return (false, false);
    }
    scroll(cur.0 as i32 - prev.0 as i32, cur.1 as i32 - prev.1 as i32);
    let cell = |v: i16| -> i32 {
        let v = v as i32;
        (((v >> 31) & 0x1f) + v) >> 5
    };
    let redraw = cell(prev.0) != cell(cur.0) || cell(prev.1) != cell(cur.1);
    (true, redraw)
}

#[cfg(test)]
mod tests_wave16_agent06 {
    use super::*;

    #[test]
    fn ring() {
        let r = [1, 2, 3, 4, 5];
        assert_eq!(w16a06_ring5_lookup(3, &r, 4), 3);
        assert_eq!(w16a06_ring5_lookup(0, &r, 2), 3);
    }

    #[test]
    fn target() {
        assert_eq!(
            w16a06_target_action(true, true, 5, 6, 3, 4, true),
            Some((0x31, 5, 0))
        );
        assert_eq!(
            w16a06_target_action(true, true, 5, 6, 3, 4, false),
            Some((6, 3, 4))
        );
        assert_eq!(w16a06_target_action(true, true, 6, 6, 3, 4, true), None);
        assert_eq!(w16a06_target_action(true, true, 5, 6, 0, 4, true), None);
    }

    #[test]
    fn lists() {
        let next = |n: u32| n + 1;
        assert!(w16a06_any_in_lists(2, 0u32, 2, 10, next, |n| n == 11));
        assert!(!w16a06_any_in_lists(2, 0u32, 1, 10, next, |n| n == 11));
    }

    #[test]
    fn dispatch() {
        let mut log = vec![];
        assert!(w16a06_dispatch_by_flag(
            2,
            1,
            2,
            3,
            4,
            |a, _, _| log.push(a),
            |_, _, _, _| {}
        ));
        assert!(!w16a06_dispatch_by_flag(
            0,
            1,
            2,
            3,
            4,
            |_, _, _| {},
            |_, _, _, d| log.push(d)
        ));
        assert_eq!(log, vec![1, 4]);
    }

    #[test]
    fn rate() {
        assert_eq!(w16a06_recompute_rate(false, 2, 100, 1), None);
        assert_eq!(w16a06_recompute_rate(true, 0, 100, 1), None);
        let r = w16a06_recompute_rate(true, 2, 100, 0).unwrap();
        assert_eq!(r.field_aa, 1);
        assert_eq!(r.field_60, ((100u64 * 0x00cc_cccc_cd00) >> 0x23) as u32);
        let r = w16a06_recompute_rate(true, 2, 100, 2).unwrap();
        assert_eq!(r.field_aa, ((25600 - r.field_60) / 2) as u16);
    }

    #[test]
    fn small() {
        assert!(w16a06_is_special(false, 0xbc));
        assert!(!w16a06_is_special(false, 1));
        assert_eq!(w16a06_masked_flag(0xf0, 0x3c), 0x30);
    }

    #[test]
    fn click() {
        let r = w16a06_clamped_click(-5, 1000, 0, 0, 100, 0, 100, 0, |_, _| true);
        assert_eq!((r.x, r.y, r.issued), (0, 100 - 0x21, true));
        let r = w16a06_clamped_click(100, 10, 0, 0, 100, 0, 100, 0, |_, _| false);
        assert_eq!((r.x, r.y, r.issued), (99, 10, false));
    }

    #[test]
    fn scroll() {
        let mut d = (0, 0);
        assert_eq!(
            w16a06_scroll_step(true, (40, 5), (30, 5), |a, b| d = (a, b)),
            (true, true)
        );
        assert_eq!(d, (10, 0));
        assert_eq!(
            w16a06_scroll_step(true, (33, 5), (32, 5), |_, _| {}),
            (true, false)
        );
        assert_eq!(
            w16a06_scroll_step(false, (0, 0), (0, 0), |_, _| {}),
            (false, false)
        );
    }
}
