#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent10.rs
// ===========================================================================
// Wave 21 batch 10: small record/array helpers around the 0x0058xxxx area.

/// Four link fields of a 0x150-byte record: offsets 0x13c, 0x140, 0x144, 0x148.
pub type W21a10Links = [i32; 4];

/// `0x00588de0`: true when the object's pointer field at +0xc is non-null.
pub fn w21a10_has_data(field_c: u32) -> bool {
    field_c != 0
}

/// `0x0058b410`: stores `value` into the dword at +8.
pub fn w21a10_set_field8(field8: &mut u32, value: u32) {
    *field8 = value;
}

/// `0x0058b440`: writes `a + b` (wrapping byte add) to the byte at +0xd of the
/// record that the +0xc pointer references.
pub fn w21a10_set_sum_byte(record: &mut [u8], a: u8, b: u8) {
    record[0xd] = a.wrapping_add(b);
}

/// `0x0058b1b0`: runs the 0x0058b200 check on two table pairs; true only if
/// both succeed (second check skipped when the first fails).
pub fn w21a10_check_both(mut check: impl FnMut(usize) -> bool) -> bool {
    check(0) && check(1)
}

/// Shared walk of the four fixups.  `table` holds 8-byte entries whose first
/// dword is a 1-based record id (0 = none); `records[id - 1]` is the record
/// (stride 0x150).  Entries with `idx` in `(from, to]` (ascending) or
/// `[from, to)` (descending) have the matching link slot (`slots.0`, else
/// `slots.1`) moved by `delta`.
fn w21a10_fix(
    table: &[[i32; 2]],
    records: &mut [W21a10Links],
    range: impl Iterator<Item = i32>,
    slots: (usize, usize),
    delta: i32,
) {
    for idx in range {
        let id = table[idx as usize][0];
        if id <= 0 {
            continue;
        }
        let r = &mut records[id as usize - 1];
        if r[slots.0] == idx {
            r[slots.0] += delta;
        } else if r[slots.1] == idx {
            r[slots.1] += delta;
        }
    }
}

/// `0x0058bd80`: removal fixup; for entries `from+1..=to` decrements the
/// 0x13c/0x140 link equal to the index, then moves entries `from+1..=to`
/// down one slot (memmove of `(to-from)*8` bytes).
pub fn w21a10_remove_shift_a(
    table: &mut [[i32; 2]],
    records: &mut [W21a10Links],
    from: i32,
    to: i32,
) {
    if to <= from {
        return;
    }
    w21a10_fix(table, records, (from + 1)..=to, (0, 1), -1);
    let (f, t) = (from as usize, to as usize);
    table.copy_within(f + 1..=t, f);
}

/// `0x0058be10`: as `0x0058bd80` but for links at 0x144/0x148.
pub fn w21a10_remove_shift_b(
    table: &mut [[i32; 2]],
    records: &mut [W21a10Links],
    from: i32,
    to: i32,
) {
    if to <= from {
        return;
    }
    w21a10_fix(table, records, (from + 1)..=to, (2, 3), -1);
    let (f, t) = (from as usize, to as usize);
    table.copy_within(f + 1..=t, f);
}

/// `0x0058bea0`: insert fixup; for indices `to-1` down to `from` increments
/// the 0x13c/0x140 link equal to the index.  The trailing move is a
/// non-returning call with no visible arguments and is left to the caller.
pub fn w21a10_insert_fixup_a(table: &[[i32; 2]], records: &mut [W21a10Links], from: i32, to: i32) {
    if to <= from {
        return;
    }
    w21a10_fix(table, records, (from..to).rev(), (0, 1), 1);
}

/// `0x0058bf30`: as `0x0058bea0` but for links at 0x144/0x148.
pub fn w21a10_insert_fixup_b(table: &[[i32; 2]], records: &mut [W21a10Links], from: i32, to: i32) {
    if to <= from {
        return;
    }
    w21a10_fix(table, records, (from..to).rev(), (2, 3), 1);
}

#[cfg(test)]
mod tests_wave21_agent10 {
    use super::*;

    #[test]
    fn tiny() {
        assert!(!w21a10_has_data(0));
        assert!(w21a10_has_data(4));
        let mut f = 0;
        w21a10_set_field8(&mut f, 9);
        assert_eq!(f, 9);
        let mut r = [0u8; 16];
        w21a10_set_sum_byte(&mut r, 250, 10);
        assert_eq!(r[0xd], 4);
        assert!(w21a10_check_both(|_| true));
        assert!(!w21a10_check_both(|i| i == 0));
        let mut n = 0;
        assert!(!w21a10_check_both(|_| {
            n += 1;
            false
        }));
        assert_eq!(n, 1);
    }

    #[test]
    fn remove_shift() {
        let mut t = [[1, 0], [1, 0], [2, 0], [0, 0]];
        let mut r = [[1, 2, 0, 0], [2, 9, 2, 3]];
        w21a10_remove_shift_a(&mut t, &mut r, 0, 2);
        assert_eq!(r[0][0], 0);
        assert_eq!(r[0][1], 2);
        assert_eq!(r[1][0], 1);
        assert_eq!(t, [[1, 0], [2, 0], [2, 0], [0, 0]]);
        w21a10_remove_shift_b(&mut t, &mut r, 0, 2);
        assert_eq!(r[1][2], 1);
        assert_eq!(r[1][3], 3);
        w21a10_remove_shift_a(&mut t, &mut r, 2, 2);
    }

    #[test]
    fn insert_fixup() {
        let t = [[1, 0], [2, 0], [0, 0]];
        let mut r = [[0, 1, 0, 0], [1, 5, 1, 1]];
        w21a10_insert_fixup_a(&t, &mut r, 0, 2);
        assert_eq!(r[0], [1, 1, 0, 0]);
        assert_eq!(r[1][0], 2);
        w21a10_insert_fixup_b(&t, &mut r, 0, 2);
        assert_eq!(r[1][2], 2);
        w21a10_insert_fixup_b(&t, &mut r, 1, 1);
    }
}
