#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave15/agent01.rs
// ===========================================================================
// Wave 15 translation batch 01.
//
// Skipped (call-only wrappers, noreturn stubs or unresolved callee chains):
// FUN_00535520 (exception constructor), FUN_00596cf0, FUN_005c7cc0,
// FUN_005d5810, FUN_00635340, FUN_00635360, FUN_00635380 (pure thunks to
// the same callee), FUN_006384a0, FUN_00638940 (noreturn forwarders),
// FUN_006387c0, FUN_006388e0 (call sequences only).

/// Stride of one per-player record in the 0xfaa1f8 block.
pub const W15A01_PLAYER_STRIDE: usize = 0x4e8;

/// One step emitted by [`w15a01_dispatch_command`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W15a01Step {
    /// Accumulate the offset (and `hi`) into the player's position triple.
    AddOffset { code: u32, hi: u32 },
    /// Call `FUN_00635dd0(player, code)`.
    Prepare { code: u32 },
    /// Call `FUN_00634570(player, 1, code, count, arg)`.
    Emit { code: u32, count: i8 },
}

/// `0x006346b0`: maps a command byte to the sequence of prepare/emit steps
/// (0x2c/0x3e add an offset then emit 0x2b; 0x44 -> 0x43 twice; 0x3f -> 0x3d
/// twice; 0x67 -> 0x26 once).  `count` is `param_3 - 1`; `hi` is the
/// high byte of the packed argument word (the Ghidra view aliases it).
pub fn w15a01_dispatch_command(cmd: u32, hi: u32, param_3: i8) -> Vec<W15a01Step> {
    let count = param_3.wrapping_sub(1);
    match cmd {
        0x2c | 0x3e => vec![
            W15a01Step::AddOffset { code: 0x2b, hi },
            W15a01Step::Emit { code: 0x2b, count },
        ],
        0x44 => vec![
            W15a01Step::Prepare { code: 0x43 },
            W15a01Step::Emit { code: 0x43, count },
            W15a01Step::Prepare { code: 0x43 },
            W15a01Step::Emit { code: 0x43, count },
        ],
        0x3f => vec![
            W15a01Step::Prepare { code: 0x3d },
            W15a01Step::Emit { code: 0x3d, count },
            W15a01Step::Prepare { code: 0x3d },
            W15a01Step::Emit { code: 0x3d, count },
        ],
        0x67 => vec![
            W15a01Step::Prepare { code: 0x26 },
            W15a01Step::Emit { code: 0x26, count },
        ],
        _ => Vec::new(),
    }
}

/// `0x00634b60`: argument guard; the inner call at 634520 only runs when
/// the player index is below 8 and the second index below 0xe4.
pub fn w15a01_guard_634b60(player: u32, index: u32) -> bool {
    player < 8 && index < 0xe4
}

/// `0x00634b90`: same guard as the sibling 634b60 (player < 8, index < 0xe4).
pub fn w15a01_guard_634b90(player: u32, index: u32) -> bool {
    player < 8 && index < 0xe4
}

/// `0x006353a0`: same guard again (player < 8, index < 0xe4).
pub fn w15a01_guard_6353a0(player: u32, index: u32) -> bool {
    player < 8 && index < 0xe4
}

/// `0x00634f10`: updates a player record.  Returns false (no-op) when
/// `param_6 == 0` and flag 0x200 is set.  Otherwise clears bit 0 when
/// `param_5 == 0`, sets 0x40, adds `offset(param_3)` to x/y and the high
/// byte of `param_6` to z.  The caller then runs the emit/dispatch calls.
pub fn w15a01_apply_move(
    flags: &mut u16,
    pos: &mut [i32; 3],
    param_3: u32,
    param_5: i32,
    param_6: u32,
    offset: impl Fn(u32) -> (i32, i32),
) -> bool {
    if param_6 == 0 && (*flags & 0x200) != 0 {
        return false;
    }
    if param_5 == 0 {
        *flags &= 0xfffe;
    }
    *flags |= 0x40;
    let (dx, dy) = offset(param_3);
    pos[0] = pos[0].wrapping_add(dx);
    pos[1] = pos[1].wrapping_add(dy);
    pos[2] = pos[2].wrapping_add((param_6 >> 24) as i32);
    true
}

/// `0x00635060`: returns the mode word (3, 7 or 4) to pass on, or `None`
/// when the call is suppressed (recent stamp within 30 ticks, different
/// owner, and code != 0x8c).  `table_entry` is the 0xf8fa08 flag word.
pub fn w15a01_select_mode(
    stamp: u32,
    now: u32,
    owner_matches: bool,
    code: i32,
    table_entry: u32,
) -> Option<u32> {
    if stamp != 0 && now.wrapping_sub(stamp) < 0x1e && !owner_matches && code != 0x8c {
        return None;
    }
    if table_entry & 8 == 0 {
        Some(((table_entry & 2) * 2) | 3)
    } else {
        Some(4)
    }
}

/// `0x00635d90`: sets (`on != 0`) or clears bit 0x400 of the player flags.
pub fn w15a01_set_flag_400(flags: u16, on: bool) -> u16 {
    if on { flags | 0x400 } else { flags & 0xfbff }
}

/// `0x00635ed0`: zeroes the 0x80-byte block at record offset 0x18 for
/// `player < 8`; returns whether it ran.  `block` is the whole table.
pub fn w15a01_clear_player_block(block: &mut [u8], player: u32) -> bool {
    if player >= 8 {
        return false;
    }
    let start = player as usize * W15A01_PLAYER_STRIDE + 0x18;
    match block.get_mut(start..start + 0x80) {
        Some(s) => {
            s.fill(0);
            true
        }
        None => false,
    }
}

/// `0x00590400`: returns whether the unit matches param (`wanted`).  `unit`
/// is the raw record; `c1`/`c2` are results of the helper calls at
/// 5baa30 / 5d2160.  Returns `None` where the original calls a noreturn
/// stub (helper 5b64c0 false).
pub fn w15a01_unit_matches(
    unit: &[u8],
    wanted: u16,
    helper_ok: bool,
    c1: bool,
    c2: bool,
) -> Option<bool> {
    if !helper_ok || unit.len() < 0xf0 {
        return None;
    }
    let kind = unit[0x4d];
    let rd32 = |o: usize| u32::from_le_bytes([unit[o], unit[o + 1], unit[o + 2], unit[o + 3]]);
    let blocked = if matches!(kind, 0x1e | 0x22 | 0x21) || c1 {
        rd32(0x5c) != 0
    } else if unit[0xa6] == b'&' || unit[0xa6] == b'?' || c2 {
        rd32(0xec) != 0
    } else {
        false
    };
    if blocked {
        return Some(false);
    }
    let o = 0x98 + (unit[0xa4] as usize % 5) * 2;
    let v = u16::from_le_bytes([unit[o], unit[o + 1]]);
    Some(v == wanted && matches!(kind, 0x1e | 0x1f | 0x46 | 0x1a | 0x19))
}

#[cfg(test)]
mod tests_wave15_agent01 {
    use super::*;

    #[test]
    fn dispatch() {
        assert_eq!(w15a01_dispatch_command(0x44, 0, 5).len(), 4);
        assert_eq!(
            w15a01_dispatch_command(0x67, 0, 1),
            vec![
                W15a01Step::Prepare { code: 0x26 },
                W15a01Step::Emit {
                    code: 0x26,
                    count: 0
                }
            ]
        );
        assert_eq!(
            w15a01_dispatch_command(0x2c, 5, 3)[0],
            W15a01Step::AddOffset { code: 0x2b, hi: 5 }
        );
        assert!(w15a01_dispatch_command(1, 0, 1).is_empty());
    }

    #[test]
    fn guards() {
        assert!(w15a01_guard_634b60(7, 0xe3));
        assert!(!w15a01_guard_634b90(8, 0));
        assert!(!w15a01_guard_6353a0(0, 0xe4));
    }

    #[test]
    fn apply_move() {
        let mut f = 0x201u16;
        let mut p = [1, 2, 3];
        assert!(!w15a01_apply_move(&mut f, &mut p, 0, 0, 0, |_| (1, 1)));
        assert!(w15a01_apply_move(&mut f, &mut p, 0, 0, 0x0200_0000, |_| (
            10, 20
        )));
        assert_eq!(f, 0x240);
        assert_eq!(p, [11, 22, 5]);
    }

    #[test]
    fn mode() {
        assert_eq!(w15a01_select_mode(0, 100, false, 1, 0), Some(3));
        assert_eq!(w15a01_select_mode(0, 100, false, 1, 2), Some(7));
        assert_eq!(w15a01_select_mode(0, 100, false, 1, 8), Some(4));
        assert_eq!(w15a01_select_mode(90, 100, false, 1, 0), None);
        assert_eq!(w15a01_select_mode(90, 100, false, 0x8c, 0), Some(3));
    }

    #[test]
    fn flags_and_clear() {
        assert_eq!(w15a01_set_flag_400(0, true), 0x400);
        assert_eq!(w15a01_set_flag_400(0xffff, false), 0xfbff);
        let mut b = vec![0xffu8; 8 * W15A01_PLAYER_STRIDE];
        assert!(w15a01_clear_player_block(&mut b, 1));
        assert_eq!(b[W15A01_PLAYER_STRIDE + 0x18], 0);
        assert_eq!(b[W15A01_PLAYER_STRIDE + 0x98], 0xff);
        assert!(!w15a01_clear_player_block(&mut b, 8));
    }

    #[test]
    fn unit_match() {
        let mut u = vec![0u8; 0x100];
        u[0x4d] = 0x1e;
        u[0x98] = 7;
        assert_eq!(w15a01_unit_matches(&u, 7, true, false, false), Some(true));
        u[0x5c] = 1;
        assert_eq!(w15a01_unit_matches(&u, 7, true, false, false), Some(false));
        assert_eq!(w15a01_unit_matches(&u, 7, false, false, false), None);
    }
}
