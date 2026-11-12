#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave15/agent07.rs
// ===========================================================================
// Wave 15 translation batch 07.
//
// Callees not yet translated are passed as closures.
//
// Skipped (callee-dependent, fragments or CRT glue):
// - FUN_005c4ed0, FUN_005c4f70: linked-list surgery on retail memory layout.
// - FUN_005c5c10, FUN_005c5e05: `unaff_*` fragments.
// - FUN_005c6010, FUN_005c8b40, FUN_005cea40: dozens of untranslated callees and
//   encrypted-table reads; no testable pure logic.
// - FUN_005c8fc0: TLS static-init guard.
// - FUN_005ccaa0, FUN_005d4451, FUN_0062c000, FUN_0062c350: pure call chains.

/// Translation of 0x005c1c90: rectangle-fits-in-map check then two probes.
///
/// `p1`/`p2` is the centre, `w`/`h` the footprint, `map_w`/`map_h` the
/// 16-bit map limits (DAT_00fe5b5c / DAT_00fe5b60).
#[allow(clippy::too_many_arguments)]
pub fn w15a07_footprint_fits(
    p1: u32,
    p2: u32,
    w: i32,
    h: i32,
    map_w: u16,
    map_h: u16,
    mut probe_a: impl FnMut(u32, u32) -> bool,
    mut set_rect: impl FnMut(i32, i32, i32, i32),
    mut probe_b: impl FnMut(u32, u32) -> bool,
) -> bool {
    let x = (p1 as i16).wrapping_sub((w / 2) as i16);
    let y = (p2 as i16).wrapping_sub((h / 2) as i16);
    #[allow(clippy::collapsible_if)]
    if x > -1 && (x as i32 + w) < map_w as i32 && y > -1 && (y as i32 + h) < map_h as i32 {
        if probe_a(p1, p2) {
            set_rect(
                (w - 1) / 2,
                (h - 1) / 2,
                (w - ((w / 2) & 0xffff)) - 1,
                (h - ((h / 2) & 0xffff)) - 1,
            );
            if probe_b(p1, p2) {
                return true;
            }
        }
    }
    false
}

/// Translation of 0x005c7550: if the unit has a target (`target_ptr != 0`) and
/// no blocking condition, a value >= 99 with a secondary value < 0x4c triggers
/// the action.  Returns the constant 0 like retail.
pub fn w15a07_conditional_action(
    short_value: i16,
    cond_a: bool,
    target_ptr: u32,
    cond_b: bool,
    value_a: i32,
    value_b: i32,
    mut action: impl FnMut(),
) -> u32 {
    if short_value == 0 && !cond_a && target_ptr != 0 && !cond_b && value_a > 99 && value_b < 0x4c {
        action();
    }
    0
}

/// Translation of 0x005c80a0: validated command; optionally fires a side effect.
#[allow(clippy::too_many_arguments)]
pub fn w15a07_validated_command(
    pre_ok: bool,
    blocked: bool,
    check_22: bool,
    check_2c80: bool,
    dat_fa3900: i32,
    owner_ok: bool,
    flag_64c0: bool,
    mut side_effect: impl FnMut(),
) -> u32 {
    if pre_ok && !blocked && check_22 && check_2c80 {
        if dat_fa3900 == 0 && owner_ok && !flag_64c0 {
            side_effect();
        }
        return 1;
    }
    0
}

/// Translation of 0x005c89d0: command 0x53 validation; on success records the
/// pending command (id 0x53, value 0) and calls the follow-up.  Returns 0.
/// `owner_ptr` is the `+0x5c` field, `owner_player` its byte at `+0x4c`.
/// Returns `(ret, Option<(dat_fa38f4, dat_fa38f8)>)`.
pub fn w15a07_command_53(
    pre_ok: bool,
    check_53: bool,
    blocked_5820: bool,
    owner_ptr: u32,
    owner_player: u8,
    owner_flag_5b50: bool,
    mut follow_up: impl FnMut(),
) -> (u32, Option<(u32, u32)>) {
    if pre_ok && check_53 && !blocked_5820 && owner_ptr != 0 && owner_player < 8 && !owner_flag_5b50
    {
        follow_up();
        return (0, Some((0x53, 0)));
    }
    (0, None)
}

/// Translation of 0x005cbb60.  Returns the `FUN_005cf3b0` argument when the
/// 0x6d follow-up fires.
pub fn w15a07_threshold_fire(
    cond_5940: bool,
    cond_5c90: bool,
    dat_1165d00: u32,
    dat_f8dc24: u16,
    field_a2: u16,
    mut call_4cd0: impl FnMut(),
) -> Option<u32> {
    if cond_5940 {
        return None;
    }
    if !cond_5c90 {
        call_4cd0();
    }
    if dat_1165d00 & 0x800 != 0 {
        return Some(((dat_f8dc24 & 0xff) as u32) << 8);
    }
    let v = ((dat_f8dc24 as u32) << 8) as u16;
    if v <= field_a2 {
        return Some(v as u32);
    }
    None
}

/// Translation of 0x005ccb40: byte flags at fixed offsets select the handlers
/// to run, in order.  Returns the handler addresses invoked.
pub fn w15a07_flagged_handlers(this: &[u8]) -> Vec<u32> {
    const TABLE: [(usize, u32); 8] = [
        (0x114, 0x5d_26a0),
        (0x117, 0x5d_27a0),
        (0x124, 0x5d_27f0),
        (0x118, 0x5d_2720),
        (0x116, 0x5d_c680),
        (0x119, 0x5b_b5e0),
        (0x11a, 0x5d_c5e0),
        (0x126, 0x5d_c630),
    ];
    TABLE
        .iter()
        .filter(|(off, _)| this.get(*off).copied().unwrap_or(0) != 0)
        .map(|&(_, f)| f)
        .collect()
}

/// Translation of 0x005d46c0: call `f` with the lookup result when non-null.
pub fn w15a07_call_if_found(lookup: u32, mut f: impl FnMut(u32)) {
    if lookup != 0 {
        f(lookup);
    }
}

/// Translation of 0x005dcae0: same-player check between two units.
pub fn w15a07_same_player_ok(
    a_ok: bool,
    b_ok: bool,
    c_ok: bool,
    this_player: u8,
    other_player: u8,
    blocked: bool,
    flag_5250: bool,
) -> u32 {
    (a_ok && b_ok && c_ok && this_player == other_player && !blocked && !flag_5250) as u32
}

#[cfg(test)]
mod tests_wave15_agent07 {
    use super::*;

    #[test]
    fn footprint() {
        let mut rect = None;
        let ok = w15a07_footprint_fits(
            10,
            10,
            4,
            3,
            64,
            64,
            |_, _| true,
            |a, b, c, d| rect = Some((a, b, c, d)),
            |_, _| true,
        );
        assert!(ok);
        assert_eq!(rect, Some((1, 1, 1, 1)));
        assert!(!w15a07_footprint_fits(
            1,
            10,
            4,
            3,
            64,
            64,
            |_, _| true,
            |_, _, _, _| {},
            |_, _| true
        ));
        assert!(!w15a07_footprint_fits(
            10,
            10,
            4,
            3,
            64,
            64,
            |_, _| false,
            |_, _, _, _| {},
            |_, _| true
        ));
    }

    #[test]
    fn cond_action() {
        let mut n = 0;
        assert_eq!(
            w15a07_conditional_action(0, false, 1, false, 100, 0x4b, || n += 1),
            0
        );
        assert_eq!(n, 1);
        w15a07_conditional_action(0, false, 1, false, 99, 0x4b, || n += 1);
        w15a07_conditional_action(0, false, 1, false, 100, 0x4c, || n += 1);
        assert_eq!(n, 1);
    }

    #[test]
    fn commands() {
        let mut n = 0;
        assert_eq!(
            w15a07_validated_command(true, false, true, true, 0, true, false, || n += 1),
            1
        );
        assert_eq!(n, 1);
        assert_eq!(
            w15a07_validated_command(false, false, true, true, 0, true, false, || n += 1),
            0
        );
        assert_eq!(
            w15a07_command_53(true, true, false, 5, 7, false, || n += 1),
            (0, Some((0x53, 0)))
        );
        assert_eq!(
            w15a07_command_53(true, true, false, 5, 8, false, || n += 1),
            (0, None)
        );
        assert_eq!(n, 2);
    }

    #[test]
    fn threshold() {
        assert_eq!(w15a07_threshold_fire(true, true, 0, 0, 0, || {}), None);
        assert_eq!(
            w15a07_threshold_fire(false, true, 0x800, 0x1234, 0, || {}),
            Some(0x3400)
        );
        assert_eq!(
            w15a07_threshold_fire(false, true, 0, 2, 0x200, || {}),
            Some(0x200)
        );
        assert_eq!(w15a07_threshold_fire(false, true, 0, 3, 0x200, || {}), None);
        let mut n = 0;
        w15a07_threshold_fire(false, false, 0, 3, 0, || n += 1);
        assert_eq!(n, 1);
    }

    #[test]
    fn handlers() {
        let mut b = vec![0u8; 0x130];
        b[0x114] = 1;
        b[0x126] = 2;
        assert_eq!(w15a07_flagged_handlers(&b), vec![0x5d_26a0, 0x5d_c630]);
        assert!(w15a07_flagged_handlers(&[]).is_empty());
    }

    #[test]
    fn misc() {
        let mut got = 0;
        w15a07_call_if_found(0, |v| got = v);
        assert_eq!(got, 0);
        w15a07_call_if_found(9, |v| got = v);
        assert_eq!(got, 9);
        assert_eq!(
            w15a07_same_player_ok(true, true, true, 1, 1, false, false),
            1
        );
        assert_eq!(
            w15a07_same_player_ok(true, true, true, 1, 2, false, false),
            0
        );
    }
}
