#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent13.rs
// ===========================================================================
// Wave 21 agent 13: small predicates and dispatch glue (runtime.rs target).

/// `0x005b3a20`: unit-flag gate.  Returns true when flags bit 1 (`+0xdc`)
/// is clear, the first query is non-zero and the second query is not 3.
pub fn w21a13_flag_gate(flags_dc: u32, query_a: u8, query_b: u8) -> bool {
    (flags_dc & 2) == 0 && query_a != 0 && query_b != 3
}

/// `0x005b4a70`: reads the u16 at `+8` of the record and returns it with the
/// low byte replaced by `table[idx]` (table at the `f8d1de` array).
pub fn w21a13_table_lookup_low(idx: u16, table: &[u8]) -> Option<u16> {
    let b = *table.get(idx as usize)?;
    Some((idx & 0xff00) | b as u16)
}

/// `0x005b5eb0`: unit eligibility predicate.  `type_flags` is the per-type
/// flag table (`f8fa08`, 0xe4 entries); the other args are unit fields at
/// `+0x64` (type), `+0xdc`, `+0x117`, `+0x119`, `+0x124`.
pub fn w21a13_unit_eligible(
    unit_type: u16,
    type_flags: &[u32],
    flags_dc: u32,
    b117: u8,
    b119: u8,
    b124: u8,
) -> bool {
    let tf = type_flags.get(unit_type as usize).copied().unwrap_or(0);
    let t = unit_type as u32;
    (unit_type > 0xe3 || (tf & 1) == 0)
        && (tf & 0x800) == 0
        && (flags_dc & 0x400) == 0
        && b117 == 0
        && b119 == 0
        && b124 == 0
        && unit_type.wrapping_sub(0xcb) > 10
        && ![0x0d, 0x24, 0x59, 0x5a, 0x5f, 0x5e, 0x5d, 0x60, 0x69].contains(&t)
        && unit_type != 0xca
}

/// `0x005b5f90`: if `precheck` passes, dispatches on type-flag bit 8 to
/// `path_a` (set) or `path_b` (clear).  Returns whether precheck passed.
pub fn w21a13_dispatch_by_flag(
    precheck: bool,
    type_flags: u32,
    path_a: impl FnOnce(),
    path_b: impl FnOnce(),
) -> bool {
    if !precheck {
        return false;
    }
    if type_flags & 0x100 != 0 {
        path_a();
    } else {
        path_b();
    }
    true
}

/// `0x005b6b80`: when `enable` is set and `query` is false, calls `set(1)`.
/// Always returns 1.
pub fn w21a13_ensure_flag(enable: bool, query: impl FnOnce() -> bool, set: impl FnOnce()) -> u32 {
    if enable && !query() {
        set();
    }
    1
}

/// `0x005b6bf0`: when `enable` is set calls `action`; always returns 1.
pub fn w21a13_conditional_action(enable: bool, action: impl FnOnce()) -> u32 {
    if enable {
        action();
    }
    1
}

/// `0x005b7320`: mode check.  `session` is the optional (active, kind,
/// value) triple read from the session record (`kind == 5`, `value` at +4).
/// Returns true for value < 0x1fd9, `param == 0` for value == 0x1fd9;
/// otherwise true only when `param` is 0 or 0x97 (-0x69 as i8).
pub fn w21a13_mode_check(session: Option<(bool, u8, u32)>, param: i8) -> bool {
    if let Some((flag, kind, value)) = session {
        if !flag && kind == 5 && value <= 0x1fd9 {
            return value < 0x1fd9 || param == 0;
        }
    }
    param == 0 || param == -0x69
}

/// `0x005b6dd0`: combines the mode check (same logic as `0x005b7320`) with
/// the unit byte at `+0x84`.  When the mode check holds the result is
/// `byte84 < 8 && cond_a()`; otherwise `!cond_b() && byte84 < 8`.
pub fn w21a13_mode_unit_check(
    session: Option<(bool, u8, u32)>,
    param: i8,
    byte84: u8,
    cond_a: impl FnOnce() -> bool,
    cond_b: impl FnOnce() -> bool,
) -> bool {
    if w21a13_mode_check(session, param) {
        byte84 < 8 && cond_a()
    } else {
        !cond_b() && byte84 < 8
    }
}

/// `0x005b7ca0`: temporarily swaps the global at `f95b40` with the object's
/// `+0x70` field around `call(param, 1)`, then stores the object pointer.
pub fn w21a13_with_swapped_global(
    global: &mut u32,
    obj: u32,
    field_70: u32,
    call: impl FnOnce(u32, u32),
    param: u32,
) {
    *global = field_70;
    call(param, 1);
    *global = obj;
}

/// `0x005b3ebd`: runs three steps in order, the first with `table[idx]`
/// (table at `f8f2c8`); returns None if `idx` is out of range.
pub fn w21a13_three_step(
    table: &[u32],
    idx: usize,
    step1: impl FnOnce(u32),
    step2: impl FnOnce(),
    step3: impl FnOnce(),
) -> Option<()> {
    step1(*table.get(idx)?);
    step2();
    step3();
    Some(())
}

/// `0x005afec0`: init sequence; runs `pre`, and if `fail()` is true the
/// routine aborts (returns None), else runs `post` and returns 1.
pub fn w21a13_init_sequence(
    pre: impl FnOnce(),
    fail: impl FnOnce() -> bool,
    post: impl FnOnce(),
) -> Option<u32> {
    pre();
    if fail() {
        return None;
    }
    post();
    Some(1)
}

/// `0x005b2f00`: forwards the pair (`a36b4`, `a3704`) to the copy helper.
pub fn w21a13_forward_pair_a<T>(f: impl FnOnce(u32, u32) -> T) -> T {
    f(0x00fa_36b4, 0x00fa_3704)
}

/// `0x005b2f10`: forwards the pair (`a36a4`, `a36fc`) to the copy helper.
pub fn w21a13_forward_pair_b<T>(f: impl FnOnce(u32, u32) -> T) -> T {
    f(0x00fa_36a4, 0x00fa_36fc)
}

#[cfg(test)]
mod tests_wave21_agent13 {
    use super::*;

    #[test]
    fn gate_and_lookup() {
        assert!(w21a13_flag_gate(0, 1, 2));
        assert!(!w21a13_flag_gate(2, 1, 2));
        assert!(!w21a13_flag_gate(0, 0, 2));
        assert!(!w21a13_flag_gate(0, 1, 3));
        assert_eq!(w21a13_table_lookup_low(2, &[0, 0, 7]), Some(7));
        assert_eq!(w21a13_table_lookup_low(5, &[0]), None);
    }

    #[test]
    fn eligible() {
        let tf = vec![0u32; 0xe4];
        assert!(w21a13_unit_eligible(0x10, &tf, 0, 0, 0, 0));
        assert!(!w21a13_unit_eligible(0x0d, &tf, 0, 0, 0, 0));
        assert!(!w21a13_unit_eligible(0xcb, &tf, 0, 0, 0, 0));
        assert!(w21a13_unit_eligible(0xd6, &tf, 0, 0, 0, 0));
        assert!(!w21a13_unit_eligible(0x10, &tf, 0x400, 0, 0, 0));
        assert!(!w21a13_unit_eligible(0xca, &tf, 0, 0, 0, 0));
        let mut tf2 = tf.clone();
        tf2[0x10] = 1;
        assert!(!w21a13_unit_eligible(0x10, &tf2, 0, 0, 0, 0));
        tf2[0x10] = 0x800;
        assert!(!w21a13_unit_eligible(0x10, &tf2, 0, 0, 0, 0));
    }

    #[test]
    fn dispatch_and_actions() {
        let (mut a, mut b) = (0, 0);
        assert!(!w21a13_dispatch_by_flag(false, 0x100, || a += 1, || b += 1));
        assert!(w21a13_dispatch_by_flag(true, 0x100, || a += 1, || b += 1));
        assert!(w21a13_dispatch_by_flag(true, 0, || a += 1, || b += 1));
        assert_eq!((a, b), (1, 1));
        let mut n = 0;
        assert_eq!(w21a13_ensure_flag(true, || false, || n += 1), 1);
        assert_eq!(w21a13_ensure_flag(true, || true, || n += 1), 1);
        assert_eq!(w21a13_ensure_flag(false, || false, || n += 1), 1);
        assert_eq!(n, 1);
        assert_eq!(w21a13_conditional_action(true, || n += 1), 1);
        assert_eq!(w21a13_conditional_action(false, || n += 1), 1);
        assert_eq!(n, 2);
    }

    #[test]
    fn mode_checks() {
        assert!(w21a13_mode_check(None, 0));
        assert!(w21a13_mode_check(None, -0x69));
        assert!(!w21a13_mode_check(None, 1));
        assert!(w21a13_mode_check(Some((false, 5, 0x1fd8)), 9));
        assert!(w21a13_mode_check(Some((false, 5, 0x1fd9)), 0));
        assert!(!w21a13_mode_check(Some((false, 5, 0x1fd9)), -0x69));
        assert!(!w21a13_mode_check(Some((false, 5, 0x1fda)), 1));
        assert!(!w21a13_mode_check(Some((true, 5, 0x10)), 1));
        assert!(w21a13_mode_unit_check(None, 0, 7, || true, || true));
        assert!(!w21a13_mode_unit_check(None, 0, 8, || true, || true));
        assert!(w21a13_mode_unit_check(None, 1, 7, || false, || false));
        assert!(!w21a13_mode_unit_check(None, 1, 7, || false, || true));
    }

    #[test]
    fn glue() {
        let mut g = 9;
        let mut seen = (0, 0);
        w21a13_with_swapped_global(&mut g, 4, 3, |p, f| seen = (p, f), 77);
        assert_eq!((g, seen), (4, (77, 1)));
        let mut order = vec![];
        assert_eq!(w21a13_three_step(&[5], 1, |_| {}, || {}, || {}), None);
        w21a13_three_step(&[5], 0, |v| order.push(v), || {}, || {}).unwrap();
        assert_eq!(order, vec![5]);
        assert_eq!(w21a13_init_sequence(|| {}, || false, || {}), Some(1));
        assert_eq!(w21a13_init_sequence(|| {}, || true, || {}), None);
        assert_eq!(w21a13_forward_pair_a(|a, b| (a, b)), (0xfa36b4, 0xfa3704));
        assert_eq!(w21a13_forward_pair_b(|a, b| (a, b)), (0xfa36a4, 0xfa36fc));
    }
}
