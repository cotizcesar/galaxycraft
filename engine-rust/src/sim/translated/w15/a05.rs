#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave15/agent05.rs
// ===========================================================================
// Wave 15 translation batch 05.
//
// Window: queue: 20 addresses, module `sim.rs`.
// Skipped (no `0x` prefix, see per-row notes):
// FUN_0058b8c0 (ends in a non-returning call, callees unknown),
// FUN_0058ca90, FUN_0058cb60, FUN_0058db70, FUN_00595c80, FUN_0059c800,
// FUN_0059c832, FUN_0059c9d0, FUN_0059cbe0, FUN_005a30c0: all contain
// non-returning calls (FUN_005b4cd0 and friends) whose semantics are not
// recoverable from the pseudocode, so branches would be invented.

/// `0x005853b0`: copies the `0x4c..0x5c` block of one record into another
/// (4 bytes, two u16, two u32 laid out contiguously).  The trailing call on
/// the source's `+0x5c` pointer is a non-returning tail call and is not
/// modelled.  Returns `false` when either slice is too short.
pub fn w15a05_copy_record_block(src: &[u8], dst: &mut [u8]) -> bool {
    if src.len() < 0x5c || dst.len() < 0x5c {
        return false;
    }
    dst[0x4c..0x5c].copy_from_slice(&src[0x4c..0x5c]);
    true
}

/// `0x00588d90`: address of the 1-based 16-byte entry `index` in a table
/// starting at `base`; index 0 yields 0.
pub fn w15a05_entry_address(base: i32, index: i32) -> i32 {
    if index == 0 {
        return 0;
    }
    base.wrapping_sub(0x10)
        .wrapping_add(index.wrapping_mul(0x10))
}

/// `0x00588fd0`: true when the i32 at offset `0xc` of the record is zero.
pub fn w15a05_field_c_is_zero(rec: &[u8]) -> bool {
    rec.get(0xc..0x10)
        .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]) == 0)
        .unwrap_or(false)
}

/// `0x0058b820`: binary search over `count` 8-byte rows whose i32 key lives
/// at row offset 4 (`keys[i]`).  Compares `key*2` against
/// `value*2 + (if flag == 0 { 1 } else { -1 })`; returns the first row whose
/// doubled key is not greater than the probe (a lower/upper bound), or 0
/// when `count` is zero.
pub fn w15a05_bound_search(keys: &[i32], count: u32, value: i32, flag: i32) -> u32 {
    if count == 0 {
        return 0;
    }
    let probe = (u32::from(flag == 0) * 2)
        .wrapping_sub(1)
        .wrapping_add((value as u32).wrapping_mul(2)) as i32;
    let mut lo = 0u32;
    let mut hi = count;
    let mut sum = count;
    loop {
        let half = sum >> 1;
        let key = keys.get(half as usize).copied().unwrap_or(i32::MAX);
        if probe < key.wrapping_mul(2) {
            sum = half + lo;
            hi = half;
        } else {
            lo = half + 1;
            sum = half + 1 + hi;
        }
        if lo >= hi {
            return hi;
        }
    }
}

fn w15a05_tile_rect(x: i16, y: i16, width: i16, height: i16) -> (i32, i32, i32, i32) {
    let div32 = |v: i32| (v + ((v >> 31) & 0x1f)) >> 5;
    let w = div32(i32::from(width));
    let h = div32(i32::from(height));
    let left = i32::from(div32(i32::from(x)) as i16) - w / 2;
    let top = i32::from(div32(i32::from(y)) as i16) - h / 2;
    (left, top, w, h)
}

/// `0x0058c0b0`: converts a pixel position and size (callee results of the
/// two size getters) to a tile rectangle `(left, top, w, h)` centred on the
/// position (division by 32 rounding toward zero).  The original passes the
/// rect plus the constant flag `0x8000000` to the draw helper.
pub fn w15a05_tile_rect_a(x: i16, y: i16, width: i16, height: i16) -> (i32, i32, i32, i32, u32) {
    let (l, t, w, h) = w15a05_tile_rect(x, y, width, height);
    (l, t, w, h, 0x8000000)
}

/// `0x0058c390`: identical rectangle computation to the one above, handed
/// to the second draw helper with the same flag `0x8000000`.
pub fn w15a05_tile_rect_b(x: i16, y: i16, width: i16, height: i16) -> (i32, i32, i32, i32, u32) {
    let (l, t, w, h) = w15a05_tile_rect(x, y, width, height);
    (l, t, w, h, 0x8000000)
}

/// `0x0058dee0`: selects a target value from a record.  Byte `0x4d` in
/// {0x1e, 0x21, 0x22} or `is_special` true gives the u32 at `0x5c`;
/// otherwise, if byte `0xa6` is 0x26/0x3f or `other_check` is true, the u32
/// at `0xec`, else 0.
pub fn w15a05_select_target(
    rec: &[u8],
    is_special: impl FnOnce() -> bool,
    other_check: impl FnOnce() -> bool,
) -> u32 {
    let rd = |o: usize| {
        rec.get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .unwrap_or(0)
    };
    let kind = rec.get(0x4d).copied().unwrap_or(0);
    if kind != 0x1e && kind != 0x22 && kind != 0x21 && !is_special() {
        let b = rec.get(0xa6).copied().unwrap_or(0);
        if b != 0x26 && b != 0x3f && !other_check() {
            return 0;
        }
        return rd(0xec);
    }
    rd(0x5c)
}

/// `0x0058e1c0`: walks five u16 slots at `0x98` (rotated by the byte at
/// `0xa4`) from index 4 down to 0, calling `release(i)` for each slot not
/// equal to `0xe4`; then resets slot 2 (`0xa0`) to `0xe4` and the byte at
/// `0xa4` to 0.  The two string-pointer stores at `0x98/0x9c` are
/// host-pointer writes and are not modelled.
pub fn w15a05_reset_slots(rec: &mut [u8], mut release: impl FnMut(i32)) {
    if rec.len() < 0xa5 {
        return;
    }
    let mut i: i32 = 4;
    while i >= 0 {
        let idx = (i32::from(rec[0xa4]) + i) % 5;
        let o = 0x98 + idx as usize * 2;
        if u16::from_le_bytes([rec[o], rec[o + 1]]) != 0xe4 {
            release(i);
        }
        i -= 1;
    }
    rec[0xa0..0xa2].copy_from_slice(&0xe4u16.to_le_bytes());
    rec[0xa4] = 0;
}

/// `0x00591ba0`: returns 0x49 when the first predicate holds, else 0x55
/// when the second holds, else 0.
pub fn w15a05_pick_code(first: impl FnOnce() -> bool, second: impl FnOnce() -> bool) -> u32 {
    if first() {
        0x49
    } else if second() {
        0x55
    } else {
        0
    }
}

/// `0x0059f090`: conditional side-effect dispatcher.  Requires the u32 at
/// `0x8` non-zero and `blocked()` false.  With `has_target`, calls
/// `notify_target(kind != 0x22)` and, when `kind != 0x22` and the owner
/// bytes at `0x4c` differ (`owner`, `target_owner`), `owner_changed()`.
/// Then, if `check()` holds, the u32 at `0x60` exceeds 0xff and
/// `flag_table[kind]` is non-zero, calls `final_action()`.
#[allow(clippy::too_many_arguments)]
pub fn w15a05_dispatch(
    rec: &[u8],
    kind: u8,
    has_target: bool,
    target_owner: u8,
    flag_table: &[u8],
    blocked: impl FnOnce() -> bool,
    mut notify_target: impl FnMut(bool),
    mut owner_changed: impl FnMut(),
    check: impl FnOnce() -> bool,
    mut final_action: impl FnMut(),
) {
    let rd = |o: usize| {
        rec.get(o..o + 4)
            .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .unwrap_or(0)
    };
    if rd(8) == 0 || blocked() {
        return;
    }
    if has_target {
        notify_target(kind != 0x22);
        if kind != 0x22 && rec.get(0x4c).copied().unwrap_or(0) != target_owner {
            owner_changed();
        }
    }
    if check() && rd(0x60) > 0xff && flag_table.get(kind as usize).copied().unwrap_or(0) != 0 {
        final_action();
    }
}

#[cfg(test)]
mod tests_wave15_agent05 {
    use super::*;

    #[test]
    fn copy_block() {
        let mut s = vec![0u8; 0x60];
        for (i, b) in s.iter_mut().enumerate() {
            *b = i as u8;
        }
        let mut d = vec![0u8; 0x60];
        assert!(w15a05_copy_record_block(&s, &mut d));
        assert_eq!(d[0x4c], 0x4c);
        assert_eq!(d[0x5b], 0x5b);
        assert_eq!(d[0x5c], 0);
        assert!(!w15a05_copy_record_block(&s[..8], &mut d));
    }

    #[test]
    fn entry_addr_and_zero() {
        assert_eq!(w15a05_entry_address(1000, 0), 0);
        assert_eq!(w15a05_entry_address(1000, 1), 1000);
        assert_eq!(w15a05_entry_address(1000, 3), 1032);
        let mut r = [1u8; 0x10];
        assert!(!w15a05_field_c_is_zero(&r));
        r[0xc..0x10].fill(0);
        assert!(w15a05_field_c_is_zero(&r));
    }

    #[test]
    fn search() {
        let keys = [1, 3, 5, 7];
        assert_eq!(w15a05_bound_search(&keys, 0, 3, 0), 0);
        // flag==0: probe = 2*v+1 ; flag!=0: 2*v-1
        assert_eq!(w15a05_bound_search(&keys, 4, 3, 0), 2);
        assert_eq!(w15a05_bound_search(&keys, 4, 3, 1), 1);
        assert_eq!(w15a05_bound_search(&keys, 4, 100, 1), 4);
        assert_eq!(w15a05_bound_search(&keys, 4, -5, 0), 0);
    }

    #[test]
    fn rects() {
        let r = w15a05_tile_rect_a(320, 160, 64, 96);
        assert_eq!(r, (10 - 1, 5 - 1, 2, 3, 0x8000000));
        assert_eq!(w15a05_tile_rect_b(-33, 0, 32, 32).0, -1);
    }

    #[test]
    fn select() {
        let mut r = vec![0u8; 0x100];
        r[0x5c] = 7;
        r[0xec] = 9;
        r[0x4d] = 0x1e;
        assert_eq!(w15a05_select_target(&r, || false, || false), 7);
        r[0x4d] = 0;
        assert_eq!(w15a05_select_target(&r, || false, || false), 0);
        r[0xa6] = 0x3f;
        assert_eq!(w15a05_select_target(&r, || false, || false), 9);
        assert_eq!(w15a05_select_target(&r, || true, || false), 7);
    }

    #[test]
    fn reset() {
        let mut r = vec![0u8; 0xa8];
        for i in 0..5 {
            r[0x98 + i * 2..0x9a + i * 2].copy_from_slice(&0xe4u16.to_le_bytes());
        }
        r[0x98 + 2 * 3] = 1;
        r[0xa4] = 2;
        let mut seen = vec![];
        w15a05_reset_slots(&mut r, |i| seen.push(i));
        // (2 + i) % 5 == 3 -> i == 1
        assert_eq!(seen, vec![1]);
        assert_eq!(r[0xa4], 0);
        assert_eq!(&r[0xa0..0xa2], &0xe4u16.to_le_bytes());
    }

    #[test]
    fn codes_and_dispatch() {
        assert_eq!(w15a05_pick_code(|| true, || true), 0x49);
        assert_eq!(w15a05_pick_code(|| false, || true), 0x55);
        assert_eq!(w15a05_pick_code(|| false, || false), 0);
        let mut r = vec![0u8; 0x70];
        r[8] = 1;
        r[0x4c] = 1;
        r[0x61] = 1;
        let table = [0u8, 1, 1];
        let l = std::cell::RefCell::new(Vec::<&str>::new());
        w15a05_dispatch(
            &r,
            1,
            true,
            2,
            &table,
            || false,
            |b| l.borrow_mut().push(if b { "n1" } else { "n0" }),
            || l.borrow_mut().push("own"),
            || true,
            || l.borrow_mut().push("fin"),
        );
        assert_eq!(*l.borrow(), vec!["n1", "own", "fin"]);
    }
}
