#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave17/agent08.rs
// ===========================================================================
// Wave 17 batch 08 (sim.rs window): small record/list helpers.

/// `0x0075d600`: copies the two dwords at record offset `0x44a14` of the
/// `0x40`-byte-stride table into `out`.  `base` is the table bytes starting
/// at the global block pointer.  Returns `None` when out of range.
pub fn w17a08_read_pair_0x40(base: &[u8], index: u16) -> Option<[u32; 2]> {
    let o = index as usize * 0x40 + 0x44a14;
    let a = base.get(o..o + 4)?;
    let b = base.get(o + 4..o + 8)?;
    Some([
        u32::from_le_bytes(a.try_into().ok()?),
        u32::from_le_bytes(b.try_into().ok()?),
    ])
}

/// `0x00760170`: sets rectangle fields of a view record (`rec` is at least
/// `0x38` bytes; i16 fields at 0x20,0x22,0x24,0x26,0x30,0x32,0x34,0x36).
pub fn w17a08_set_view_bounds(rec: &mut [u8], p1: i16, p2: i16, p3: u16, p4: u16) {
    let mut put = |off: usize, v: u16| rec[off..off + 2].copy_from_slice(&v.to_le_bytes());
    put(0x20, p2 as u16);
    put(0x22, p3.wrapping_neg());
    put(0x32, !p3);
    put(0x26, p1 as u16);
    put(0x24, p4.wrapping_neg());
    put(0x34, !p4);
    put(0x30, p2.wrapping_add(1) as u16);
    put(0x36, p1.wrapping_add(1) as u16);
}

/// `0x0076bd90`: pushes node `node_addr` onto an intrusive list whose
/// nodes are `0x80` bytes apart from `base`.  Returns
/// `(field0, field1, new_head)`: field1 is the 1-based node index, field0 the
/// previous head's 1-based index (0 when the list was empty).
pub fn w17a08_list_push_front(node_addr: i32, base: i32, head: Option<i32>) -> (i32, i32, i32) {
    let f1 = ((node_addr.wrapping_sub(base)) >> 7) + 1;
    let f0 = match head {
        None => 0,
        Some(h) => ((h.wrapping_sub(base)) >> 7) + 1,
    };
    (f0, f1, node_addr)
}

/// `0x0076bdd0`: with the byte at record offset `0x19` (`None` when the
/// record lookup failed): returns 1 when it is zero, otherwise decrements it
/// and returns 0.  A missing record returns 0.
pub fn w17a08_tick_counter(counter: Option<&mut u8>) -> u32 {
    match counter {
        None => 0,
        Some(c) if *c == 0 => 1,
        Some(c) => {
            *c -= 1;
            0
        }
    }
}

/// `0x0076be00`: stores `value` at record offset `0x19` when the record exists.
pub fn w17a08_set_counter(counter: Option<&mut u8>, value: u8) {
    if let Some(c) = counter {
        *c = value;
    }
}

/// `0x00748cf0`: when `test` (a predicate callee) holds, runs `act(1)`.
pub fn w17a08_conditional_act(test: bool, mut act: impl FnMut(u32)) {
    if test {
        act(1);
    }
}

/// `0x00775c80`: walks the 12 slots (stops at the first zero slot, returning
/// true); returns false as soon as `ok(slot)` is false.
pub fn w17a08_all_slots_ok(slots: &[u32; 12], mut ok: impl FnMut(u32) -> bool) -> bool {
    for &s in slots {
        if s == 0 {
            return true;
        }
        if !ok(s) {
            return false;
        }
    }
    true
}

/// `0x00775cb0`: ordering predicate.  `c1..c4` are the four sequential
/// boolean callee results, `k1`/`k2` the two byte keys.
pub fn w17a08_order_less(p: i32, c1: bool, c2: bool, c3: bool, c4: bool, k1: u8, k2: u8) -> bool {
    if p == 0 {
        return true;
    }
    if c1 && !c2 {
        return true;
    }
    if !c3 && c4 {
        return false;
    }
    k1 < k2
}

/// `0x00776aa0`: advances the slot cursor.  While the current slot of the
/// 12-slot row (`row`, selected by caller) is non-zero and `gate()` holds,
/// runs `per_item(slot, 0..8)` then `after(slot)`.  Increments the cursor
/// (an `i32` < 12) and returns the last slot value; returns 0 when the
/// cursor is already >= 12.
pub fn w17a08_advance_slots(
    cursor: &mut i32,
    row: &[i32; 12],
    mut gate: impl FnMut() -> bool,
    mut per_item: impl FnMut(i32, i32),
    mut after: impl FnMut(i32),
) -> i32 {
    if *cursor >= 12 {
        return 0;
    }
    let mut slot;
    loop {
        slot = row[*cursor as usize];
        if slot == 0 || !gate() {
            break;
        }
        for i in 0..8 {
            per_item(slot, i);
        }
        after(slot);
    }
    *cursor += 1;
    slot
}

/// `0x00776a90`: resets the cursor to 0 then runs [`w17a08_advance_slots`].
pub fn w17a08_reset_and_advance(
    cursor: &mut i32,
    row: &[i32; 12],
    gate: impl FnMut() -> bool,
    per_item: impl FnMut(i32, i32),
    after: impl FnMut(i32),
) -> i32 {
    *cursor = 0;
    w17a08_advance_slots(cursor, row, gate, per_item, after)
}

#[cfg(test)]
mod tests_wave17_agent08 {
    use super::*;

    #[test]
    fn pair_and_bounds() {
        let mut b = vec![0u8; 0x44a14 + 0x40 + 8];
        let o = 0x44a14 + 0x40;
        b[o..o + 4].copy_from_slice(&7u32.to_le_bytes());
        b[o + 4..o + 8].copy_from_slice(&9u32.to_le_bytes());
        assert_eq!(w17a08_read_pair_0x40(&b, 1), Some([7, 9]));
        assert_eq!(w17a08_read_pair_0x40(&b, 5), None);
        let mut r = [0u8; 0x38];
        w17a08_set_view_bounds(&mut r, 10, 20, 3, 4);
        let g = |o: usize| i16::from_le_bytes([r[o], r[o + 1]]);
        assert_eq!((g(0x20), g(0x22), g(0x32), g(0x26)), (20, -3, -4, 10));
        assert_eq!((g(0x24), g(0x34), g(0x30), g(0x36)), (-4, -5, 21, 11));
    }

    #[test]
    fn list_and_counter() {
        assert_eq!(w17a08_list_push_front(0x1100, 0x1000, None), (0, 3, 0x1100));
        assert_eq!(
            w17a08_list_push_front(0x1100, 0x1000, Some(0x1000)),
            (1, 3, 0x1100)
        );
        let mut c = 2u8;
        assert_eq!(w17a08_tick_counter(Some(&mut c)), 0);
        assert_eq!(c, 1);
        c = 0;
        assert_eq!(w17a08_tick_counter(Some(&mut c)), 1);
        assert_eq!(w17a08_tick_counter(None), 0);
        w17a08_set_counter(Some(&mut c), 5);
        assert_eq!(c, 5);
        w17a08_set_counter(None, 1);
    }

    #[test]
    fn predicates() {
        let mut n = 0;
        w17a08_conditional_act(true, |v| n += v);
        w17a08_conditional_act(false, |v| n += v);
        assert_eq!(n, 1);
        let mut s = [0u32; 12];
        s[0] = 1;
        s[1] = 2;
        assert!(w17a08_all_slots_ok(&s, |_| true));
        assert!(!w17a08_all_slots_ok(&s, |x| x != 2));
        assert!(w17a08_order_less(0, false, false, false, false, 9, 1));
        assert!(w17a08_order_less(1, true, false, false, false, 9, 1));
        assert!(!w17a08_order_less(1, false, false, false, true, 1, 9));
        assert!(w17a08_order_less(1, false, false, false, false, 1, 2));
        assert!(!w17a08_order_less(1, false, false, false, false, 2, 2));
    }

    #[test]
    fn advance() {
        let mut row = [0i32; 12];
        row[0] = 5;
        let mut cur = 3;
        let mut gates = 1;
        let mut items = 0;
        let r = w17a08_reset_and_advance(
            &mut cur,
            &row,
            || {
                gates -= 1;
                gates >= 0
            },
            |_, _| items += 1,
            |_| {},
        );
        assert_eq!((r, cur, items), (5, 1, 8));
        cur = 12;
        assert_eq!(
            w17a08_advance_slots(&mut cur, &row, || true, |_, _| {}, |_| {}),
            0
        );
    }
}
