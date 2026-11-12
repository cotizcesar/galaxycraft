#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave15/agent08.rs
// ===========================================================================
// Wave 15 translation batch 08.
//
// Window: queue: 20 addresses, module `sim.rs`.
// Callee-heavy bodies are modelled with caller-supplied closures.
//
// Skipped (no `0x` prefix on purpose):
// - FUN_0062c560, FUN_00664580, FUN_006558c0, FUN_006509f0: only forward to
//   non-returning callees or loop into one; no observable result.
// - FUN_0064f1f0, FUN_00650cc0, FUN_00651b70, FUN_00651e90, FUN_00652b60,
//   FUN_00654050, FUN_00661db0: dominated by unresolved callees and
//   non-returning exits, body not recoverable.
// - FUN_0065b2b0: large opcode interpreter over many unresolved predicates.
// - FUN_006a39c0: giant `this`-cached field switch over unresolved callees.

/// `0x006354b0`: true when the order has a target (`+0x134 != 0`), the
/// owning player's slot type (`DAT + player * 0x4e8`) is 2 (computer) and
/// `count(select(player), player, 1)` exceeds 11.
pub fn w15a08_has_target_and_big_count(
    target_ptr: u32,
    player: u8,
    slot_type: u8,
    select: impl Fn(u32) -> u16,
    count: impl Fn(u16, u32, u32) -> u32,
) -> bool {
    if target_ptr == 0 || slot_type != 2 {
        return false;
    }
    let p = u32::from(player);
    count(select(p), p, 1) > 0xb
}

/// `0x00637b70`: when `a[player] <= b[player]`, scans the player's list and
/// returns true at the first node that passes `skip == false`, `check` and
/// `confirm`; false otherwise (also when the counters are out of range).
pub fn w15a08_scan_list_confirm<T: Copy>(
    a: &[u32],
    b: &[u32],
    player: usize,
    list: &[T],
    skip: impl Fn(T) -> bool,
    check: impl Fn(T) -> bool,
    confirm: impl Fn(T) -> bool,
) -> bool {
    let (Some(&x), Some(&y)) = (a.get(player), b.get(player)) else {
        return false;
    };
    if x > y {
        return false;
    }
    list.iter().any(|&n| !skip(n) && check(n) && confirm(n))
}

/// `0x006517e0`: packs `(param_2, param_3)` via `pack`, reads the unit's id
/// via `unit_id`, then forwards `(owner, id, packed)` to `sink`.
pub fn w15a08_forward_owner_id_packed<R>(
    owner: u8,
    a: u32,
    b: u32,
    pack: impl Fn(u32, u32) -> u16,
    unit_id: impl Fn() -> u16,
    sink: impl FnOnce(u8, u16, u16) -> R,
) -> R {
    let packed = pack(a, b);
    let id = unit_id();
    sink(owner, id, packed)
}

/// `0x00687660`: tests a 32-px-cell flag grid.  The cell word at
/// `(y >> 5) * stride + (x >> 5)` (signed rounding toward zero) must hit
/// `want` and miss `forbid`.  Out of range cells return false.
pub fn w15a08_cell_flags_match(
    grid: &[u32],
    stride: u16,
    x: i16,
    y: i16,
    want: u32,
    forbid: u32,
) -> bool {
    let cx = i32::from(x).wrapping_add((i32::from(x) >> 31) & 0x1f) >> 5;
    let cy = i32::from(y).wrapping_add((i32::from(y) >> 31) & 0x1f) >> 5;
    let idx = i64::from(stride) * i64::from(cy) + i64::from(cx);
    let Some(&w) = usize::try_from(idx).ok().and_then(|i| grid.get(i)) else {
        return false;
    };
    want & w != 0 && forbid & w == 0
}

/// `0x00695e40`: thin wrapper that calls the target with `0xffffffff` as
/// its first argument and the given second one.
pub fn w15a08_call_with_minus_one<R>(arg: u32, f: impl FnOnce(u32, u32) -> R) -> R {
    f(0xffff_ffff, arg)
}

/// `0x006ae540`: slot `(base + idx * 12) * 4`; when `try_fn` fails the
/// fallback `on_fail(slot, byte)` runs and 0 is returned, else 1.
pub fn w15a08_try_slot(
    base: i32,
    idx: u16,
    try_fn: impl FnOnce(i32) -> bool,
    on_fail: impl FnOnce(i32),
) -> bool {
    let slot = base
        .wrapping_add(i32::from(idx) * 0xc)
        .wrapping_mul(4)
        .wrapping_add(0x58_a364);
    if try_fn(slot) {
        true
    } else {
        on_fail(slot);
        false
    }
}

/// `0x006c2670`: walks a zero-terminated pointer list calling `pred`; the
/// first element accepted is returned.  With no predicate the first element
/// (or 0 for an empty list) is returned.  Zero entries terminate the walk.
pub fn w15a08_find_first(list: &[u32], pred: Option<&dyn Fn(u32) -> bool>) -> u32 {
    let first = list.first().copied().unwrap_or(0);
    let Some(p) = pred else { return first };
    for &e in list.iter().take_while(|&&e| e != 0) {
        if p(e) {
            return e;
        }
    }
    0
}

#[cfg(test)]
mod tests_wave15_agent08 {
    use super::*;

    #[test]
    fn big_count() {
        let sel = |p: u32| p as u16;
        assert!(w15a08_has_target_and_big_count(1, 3, 2, sel, |_, _, _| 12));
        assert!(!w15a08_has_target_and_big_count(1, 3, 2, sel, |_, _, _| 11));
        assert!(!w15a08_has_target_and_big_count(0, 3, 2, sel, |_, _, _| 99));
        assert!(!w15a08_has_target_and_big_count(1, 3, 1, sel, |_, _, _| 99));
    }

    #[test]
    fn scan() {
        let l = [1u32, 2, 3];
        let t = |_| true;
        assert!(w15a08_scan_list_confirm(
            &[1],
            &[1],
            0,
            &l,
            |n| n == 1,
            t,
            |n| n == 2
        ));
        assert!(!w15a08_scan_list_confirm(
            &[2],
            &[1],
            0,
            &l,
            |_| false,
            t,
            t
        ));
        assert!(!w15a08_scan_list_confirm(
            &[1],
            &[1],
            5,
            &l,
            |_| false,
            t,
            t
        ));
        assert!(!w15a08_scan_list_confirm(&[1], &[1], 0, &l, |_| true, t, t));
    }

    #[test]
    fn forward() {
        let r = w15a08_forward_owner_id_packed(
            4,
            1,
            2,
            |a, b| (a * 10 + b) as u16,
            || 77,
            |o, i, p| (o, i, p),
        );
        assert_eq!(r, (4, 77, 12));
    }

    #[test]
    fn cells() {
        let mut g = vec![0u32; 16];
        g[4 + 1] = 0b11;
        assert!(w15a08_cell_flags_match(&g, 4, 40, 40, 1, 4));
        assert!(!w15a08_cell_flags_match(&g, 4, 40, 40, 1, 2));
        assert!(!w15a08_cell_flags_match(&g, 4, 0, 0, 1, 0));
        assert!(!w15a08_cell_flags_match(&g, 4, -40, 0, 1, 0));
        assert!(!w15a08_cell_flags_match(&g, 4, 4000, 4000, 1, 0));
    }

    #[test]
    fn wrapper_and_slot() {
        assert_eq!(w15a08_call_with_minus_one(5, |a, b| (a, b)), (u32::MAX, 5));
        let mut failed = None;
        assert!(!w15a08_try_slot(1, 2, |_| false, |s| failed = Some(s)));
        assert_eq!(failed, Some((1 + 24) * 4 + 0x58_a364));
        assert!(w15a08_try_slot(1, 2, |_| true, |_| panic!()));
    }

    #[test]
    fn find() {
        let l = [5u32, 6, 7, 0, 9];
        assert_eq!(w15a08_find_first(&l, None), 5);
        assert_eq!(w15a08_find_first(&l, Some(&|e| e == 7)), 7);
        assert_eq!(w15a08_find_first(&l, Some(&|e| e == 9)), 0);
        assert_eq!(w15a08_find_first(&[], None), 0);
    }
}
