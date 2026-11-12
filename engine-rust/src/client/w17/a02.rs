#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave17/agent02.rs
// ===========================================================================
// Wave 17 batch 02: client-side helpers (trigger selection, cost checks, table accessors).

/// Trigger file chosen by `w17a02_trigger_file_for_mode` (pure part of `0x006e89f0`).
#[allow(dead_code)]
pub fn w17a02_trigger_file_for_mode(game_type: u8, value: u16) -> Option<&'static str> {
    match game_type {
        1 | 7 => Some("triggers\\Melee.trg"),
        5 => Some("triggers\\SuddenDeath.trg"),
        3 => match value {
            0x9c4 => Some("triggers\\Greed2500.trg"),
            5000 => Some("triggers\\Greed5000.trg"),
            0x1d4c => Some("triggers\\Greed7500.trg"),
            10000 => Some("triggers\\Greed10000.trg"),
            _ => None,
        },
        6 => match value {
            0xf => Some("triggers\\Slaughter15.trg"),
            0x1e => Some("triggers\\Slaughter30.trg"),
            0x2d => Some("triggers\\Slaughter45.trg"),
            0x3c => Some("triggers\\Slaughter60.trg"),
            _ => None,
        },
        4 => Some("triggers\\CTF.trg"),
        _ => None,
    }
}

/// `0x006e89f0`: loads the game-mode trigger file.  `mode_flag`/`flags` are
/// the bytes at `011a0ed3` and (`ecd`,`ed1`,`ed7`); `game_type` = byte `ecd`,
/// `value` = word `ec9`.  Callees are closures: `reset_slots` (006e8cc0),
/// `clear_player(i)` for i in 0..8 (00635430), `load(name)` (006e80b0),
/// CTF helpers `ctf_ok` (0065a7d0), `ctf_count` (0065a0e0), `ctf_has(i)`
/// (0065a610 || 0065a520), `ctf_id(i)` (0065a100), `ctf_apply(id)` (006e81f0).
/// Returns `None` where the original calls the non-returning abort (00748d40).
#[allow(clippy::too_many_arguments)]
pub fn w17a02_load_game_triggers(
    mode_flag: u8,
    any_flag: bool,
    game_type: u8,
    value: u16,
    mut reset_slots: impl FnMut(),
    mut clear_player: impl FnMut(u32),
    mut load: impl FnMut(&str) -> u32,
    ctf_ok: impl Fn() -> bool,
    ctf_count: impl Fn() -> u8,
    ctf_has: impl Fn(u32) -> bool,
    ctf_id: impl Fn(u32) -> u8,
    mut ctf_apply: impl FnMut(u8),
) -> Option<u32> {
    if (mode_flag == 1 || mode_flag == 3) && any_flag {
        reset_slots();
    } else {
        for i in 0..8 {
            clear_player(i);
        }
    }
    if game_type == 4 {
        if !ctf_ok() {
            return None;
        }
        let mut i = ctf_count() as u32;
        while i != 0 {
            if ctf_has(i) {
                ctf_apply(ctf_id(i));
            }
            i -= 1;
        }
    }
    match w17a02_trigger_file_for_mode(game_type, value) {
        Some(name) => Some(load(name)),
        None if matches!(game_type, 3 | 6) => Some(0),
        None => Some(1),
    }
}

/// `0x0075d6e0`: record at `idx * 0x40` in the unit-slot table; writes the
/// u32 at `+0x44a08` to `out` and returns `u8@+0x44a03 - i8@+0x44a1d`.
pub fn w17a02_slot_span(table: &[u8], idx: u16, out: &mut u32) -> i32 {
    let b = idx as usize * 0x40;
    *out = u32::from_le_bytes(table[b + 0x44a08..b + 0x44a0c].try_into().unwrap());
    table[b + 0x44a03] as i32 - table[b + 0x44a1d] as i8 as i32
}

/// `0x0075d710`: first signed 16-bit value of the table.
pub fn w17a02_table_first_i16(table: &[u8]) -> i32 {
    i16::from_le_bytes([table[0], table[1]]) as i32
}

/// `0x0075d940`: true when the i16 at `idx*0x40 + 0x449fc` equals `0x1ffd`.
pub fn w17a02_slot_is_marker(table: &[u8], idx: u16) -> bool {
    let o = idx as usize * 0x40 + 0x449fc;
    i16::from_le_bytes([table[o], table[o + 1]]) == 0x1ffd
}

/// `0x00748d60`: true for network mode values `0x80..=0x83`.
pub fn w17a02_is_net_mode(mode: u32) -> bool {
    mode.wrapping_sub(0x80) < 4
}

/// Callees of `w17a02_check_cost`.
pub struct W17a02CostEnv<'a> {
    pub cost_ref: &'a dyn Fn(u32) -> u8,          // 00695df0
    pub supply_have: &'a dyn Fn(u8, u32) -> i32,  // 00695ae0
    pub other_cost: &'a dyn Fn(u32, u32) -> i32,  // 00695930
    pub limit: &'a dyn Fn(u32, u32) -> i32,       // 00695a70
    pub msg_a: &'a dyn Fn(u8) -> u32,             // 00645330
    pub msg_b: &'a dyn Fn(u8) -> u32,             // 006456e0
    pub report: &'a mut dyn FnMut(u32, u32, u16), // 006e2d40
}

/// `0x0067ced0`: checks whether `player` may afford record `id`.  `costs`
/// is the byte table `00f921b8`, `flags` the dword table `00f8fa08`,
/// `cur` the per-player scratch `00fe5ab8`, `adj` the dword at `00f921de`
/// subtracted for id `0x67`.  Reports via message ids `0x34f`/`0x34c`.
#[allow(clippy::too_many_arguments)]
pub fn w17a02_check_cost(
    player: u32,
    id: u32,
    show: bool,
    costs: &[u8],
    flags: &[u32],
    cur: &mut [u32],
    adj: u32,
    env: &mut W17a02CostEnv,
) -> bool {
    let p = (player & 0xff) as usize;
    let i = (id & 0xffff) as usize;
    let mut v = costs[i] as u32;
    cur[p] = v;
    let fl = flags[i];
    if fl & 0x400 != 0 {
        v = v.wrapping_mul(2);
        cur[p] = v;
    }
    if v == 0 || fl & 0x0100_0000 != 0 {
        return true;
    }
    if id as u16 == 0x67 {
        cur[p] = v.wrapping_sub(adj);
    }
    let b = (env.cost_ref)(id);
    let have = (env.supply_have)(b, player);
    let c1 = cur[p] as i32;
    let other = (env.other_cost)(id, player);
    let c2 = cur[p] as i32;
    let lim = (env.limit)(id, player);
    let (msg, code) = if lim < have.wrapping_add(c1) {
        if !show {
            return false;
        }
        ((env.msg_a)(b), 0x34fu16)
    } else {
        if other.wrapping_sub(c2) >= 0 {
            return true;
        }
        if !show {
            return false;
        }
        ((env.msg_b)(b), 0x34cu16)
    };
    (env.report)(p as u32, msg, (b as u16).wrapping_add(code));
    false
}

/// `0x0064f580`: inflates the rect `(x0,y0,x1,y1)` by `0x120` on each side
/// (pure part) and dispatches.  `get_rect` is 0075d7b0 (returns id, fills
/// rect), `prep` 0075d600, `rect_key` 005a57f0, `emit` 0064f1f0
/// (kind byte from `obj+0x4c`).
pub fn w17a02_inflate_rect(r: &mut [i16; 4]) {
    r[3] = r[3].wrapping_add(0x120);
    r[2] = r[2].wrapping_add(0x120);
    r[1] = r[1].wrapping_sub(0x120);
    r[0] = r[0].wrapping_sub(0x120);
}

/// `0x0064f580`: see `w17a02_inflate_rect`.
pub fn w17a02_emit_inflated(
    kind: u8,
    arg: u32,
    mut get_rect: impl FnMut(&mut [i16; 4]) -> u16,
    mut prep: impl FnMut(u16),
    rect_key: impl Fn(&[i16; 4]) -> u32,
    mut emit: impl FnMut(u8, u16, u32, u32),
) {
    let mut r = [0i16; 4];
    let id = get_rect(&mut r);
    prep(id);
    w17a02_inflate_rect(&mut r);
    let key = rect_key(&r);
    let mut dummy = [0i16; 4];
    let id2 = get_rect(&mut dummy);
    emit(kind, id2, key, arg);
}

/// `0x006e3b60`: if no replay flag (`flag == 0`) and `unit_player ==
/// local_player`, picks a player `p` and, when `check(p)` (006e41c0 with
/// args 1,100) passes, calls `fire(value, p)` with `value = i16@(p*0xb0+0x5c)
/// + (now/1000)*1000` (`now` from 0077e5c0).  `table` = base `00fc9b74`.
#[allow(clippy::too_many_arguments)]
pub fn w17a02_timed_event(
    flag: u32,
    unit_player: u8,
    local_player: u8,
    table: &[u8],
    pick: impl Fn() -> i32,
    check: impl Fn(i32) -> bool,
    now: impl Fn(i32) -> u32,
    mut fire: impl FnMut(i32, i32),
) {
    if flag != 0 || unit_player != local_player {
        return;
    }
    let p = pick();
    if check(p) {
        let o = p as usize * 0xb0 + 0x5c;
        let base = i16::from_le_bytes([table[o], table[o + 1]]) as i32;
        let v = base.wrapping_add(((now(p) / 1000) * 1000) as i32);
        fire(v, p);
    }
}

#[allow(clippy::too_many_arguments)]
fn w17a02_net_cmd(
    player_byte: u8,
    code: u8,
    first: impl FnOnce() -> i32,
    second: impl FnOnce() -> i32,
    try_cmd: impl FnOnce(u8, i32) -> i32,
    confirm: impl FnOnce(u8) -> bool,
    mut notify: impl FnMut(u8),
    mut finish: impl FnMut(),
    other: impl FnOnce(),
) {
    let h = first();
    if h != 0 && second() == 0 {
        if try_cmd(player_byte, h) == 1 {
            if confirm(player_byte) {
                notify(code);
                finish();
            }
        } else {
            other();
        }
    }
}

/// `0x00729c30`: network command variant with notify code `0x4b`.
/// Callees: first 00776a90, second 00776aa0, try 005348f0, confirm
/// 005da550, notify 005b7d00, finish 00793360, other 00728200(65b900()).
#[allow(clippy::too_many_arguments)]
pub fn w17a02_net_cmd_4b(
    player_byte: u8,
    first: impl FnOnce() -> i32,
    second: impl FnOnce() -> i32,
    try_cmd: impl FnOnce(u8, i32) -> i32,
    confirm: impl FnOnce(u8) -> bool,
    notify: impl FnMut(u8),
    finish: impl FnMut(),
    other: impl FnOnce(),
) {
    w17a02_net_cmd(
        player_byte,
        0x4b,
        first,
        second,
        try_cmd,
        confirm,
        notify,
        finish,
        other,
    );
}

/// `0x0072b570`: same shape as the `4b` variant, notify code `0x4c`
/// (try 00534c20, confirm 005da5d0).
#[allow(clippy::too_many_arguments)]
pub fn w17a02_net_cmd_4c(
    player_byte: u8,
    first: impl FnOnce() -> i32,
    second: impl FnOnce() -> i32,
    try_cmd: impl FnOnce(u8, i32) -> i32,
    confirm: impl FnOnce(u8) -> bool,
    notify: impl FnMut(u8),
    finish: impl FnMut(),
    other: impl FnOnce(),
) {
    w17a02_net_cmd(
        player_byte,
        0x4c,
        first,
        second,
        try_cmd,
        confirm,
        notify,
        finish,
        other,
    );
}

#[cfg(test)]
mod tests_wave17_agent02 {
    use super::*;

    #[test]
    fn trigger_names() {
        assert_eq!(
            w17a02_trigger_file_for_mode(1, 0),
            Some("triggers\\Melee.trg")
        );
        assert_eq!(
            w17a02_trigger_file_for_mode(3, 0x1d4c),
            Some("triggers\\Greed7500.trg")
        );
        assert_eq!(w17a02_trigger_file_for_mode(3, 7), None);
        assert_eq!(
            w17a02_trigger_file_for_mode(6, 0x2d),
            Some("triggers\\Slaughter45.trg")
        );
    }

    #[test]
    fn load_triggers_paths() {
        let mut cleared = 0;
        let mut loaded = String::new();
        let r = w17a02_load_game_triggers(
            0,
            false,
            3,
            5000,
            || {},
            |_| cleared += 1,
            |n| {
                loaded = n.into();
                9
            },
            || true,
            || 0,
            |_| false,
            |_| 0,
            |_| {},
        );
        assert_eq!(
            (r, cleared, loaded.as_str()),
            (Some(9), 8, "triggers\\Greed5000.trg")
        );
        let r = w17a02_load_game_triggers(
            1,
            true,
            2,
            0,
            || {},
            |_| {},
            |_| 9,
            || true,
            || 0,
            |_| false,
            |_| 0,
            |_| {},
        );
        assert_eq!(r, Some(1));
        let r = w17a02_load_game_triggers(
            1,
            true,
            6,
            1,
            || {},
            |_| {},
            |_| 9,
            || true,
            || 0,
            |_| false,
            |_| 0,
            |_| {},
        );
        assert_eq!(r, Some(0));
        let mut ids = vec![];
        let r = w17a02_load_game_triggers(
            0,
            false,
            4,
            0,
            || {},
            |_| {},
            |_| 2,
            || true,
            || 2,
            |i| i == 2,
            |i| i as u8 + 10,
            |id| ids.push(id),
        );
        assert_eq!((r, ids), (Some(2), vec![12]));
        let r = w17a02_load_game_triggers(
            0,
            false,
            4,
            0,
            || {},
            |_| {},
            |_| 2,
            || false,
            || 0,
            |_| false,
            |_| 0,
            |_| {},
        );
        assert_eq!(r, None);
    }

    #[test]
    fn slot_accessors() {
        let mut t = vec![0u8; 0x44a40 + 0x40];
        t[0x40 + 0x44a08..0x40 + 0x44a0c].copy_from_slice(&0xdeadbeefu32.to_le_bytes());
        t[0x40 + 0x44a03] = 10;
        t[0x40 + 0x44a1d] = 0xfe;
        t[0x40 + 0x449fc..0x40 + 0x449fe].copy_from_slice(&0x1ffdu16.to_le_bytes());
        t[0] = 0xfe;
        t[1] = 0xff;
        let mut o = 0;
        assert_eq!(w17a02_slot_span(&t, 1, &mut o), 12);
        assert_eq!(o, 0xdeadbeef);
        assert!(w17a02_slot_is_marker(&t, 1));
        assert!(!w17a02_slot_is_marker(&t, 0));
        assert_eq!(w17a02_table_first_i16(&t), -2);
    }

    #[test]
    fn net_mode() {
        assert!(!w17a02_is_net_mode(0x7f));
        assert!(w17a02_is_net_mode(0x80));
        assert!(w17a02_is_net_mode(0x83));
        assert!(!w17a02_is_net_mode(0x84));
    }

    #[test]
    fn cost_check() {
        let mut costs = vec![0u8; 0x70];
        let mut flags = vec![0u32; 0x70];
        costs[5] = 10;
        costs[0x67] = 10;
        costs[6] = 4;
        flags[6] = 0x400;
        flags[7] = 0;
        costs[7] = 3;
        flags[8] = 0x0100_0000;
        costs[8] = 3;
        let mut cur = vec![0u32; 8];
        let mut log: Vec<(u32, u32, u16)> = vec![];
        let mut rep = |a: u32, b: u32, c: u16| log.push((a, b, c));
        let cr = |_| 2u8;
        let sh = |_: u8, _| 5;
        let oc = |_, _| 100;
        let lim = |_, _| 100;
        let ma = |b: u8| b as u32 + 500;
        let mb = |b: u8| b as u32 + 600;
        let mut env = W17a02CostEnv {
            cost_ref: &cr,
            supply_have: &sh,
            other_cost: &oc,
            limit: &lim,
            msg_a: &ma,
            msg_b: &mb,
            report: &mut rep,
        };
        assert!(w17a02_check_cost(
            1, 0, true, &costs, &flags, &mut cur, 0, &mut env
        ));
        assert!(w17a02_check_cost(
            1, 8, true, &costs, &flags, &mut cur, 0, &mut env
        ));
        assert!(w17a02_check_cost(
            1, 6, true, &costs, &flags, &mut cur, 0, &mut env
        ));
        assert_eq!(cur[1], 8);
        assert!(w17a02_check_cost(
            1, 0x67, true, &costs, &flags, &mut cur, 4, &mut env
        ));
        assert_eq!(cur[1], 6);
        let lim2 = |_, _| 10;
        let mut env2 = W17a02CostEnv {
            cost_ref: &cr,
            supply_have: &sh,
            other_cost: &oc,
            limit: &lim2,
            msg_a: &ma,
            msg_b: &mb,
            report: &mut |a, b, c| log.push((a, b, c)),
        };
        assert!(!w17a02_check_cost(
            1, 5, false, &costs, &flags, &mut cur, 0, &mut env2
        ));
        assert!(!w17a02_check_cost(
            1, 5, true, &costs, &flags, &mut cur, 0, &mut env2
        ));
        assert_eq!(log, vec![(1, 502, 2 + 0x34f)]);
        let oc2 = |_, _| 5;
        let mut env3 = W17a02CostEnv {
            cost_ref: &cr,
            supply_have: &sh,
            other_cost: &oc2,
            limit: &lim,
            msg_a: &ma,
            msg_b: &mb,
            report: &mut |a, b, c| log.push((a, b, c)),
        };
        assert!(!w17a02_check_cost(
            1, 5, true, &costs, &flags, &mut cur, 0, &mut env3
        ));
        assert_eq!(log[1], (1, 602, 2 + 0x34c));
    }

    #[test]
    fn inflate_and_emit() {
        let mut r = [0, 0, 10, 10];
        w17a02_inflate_rect(&mut r);
        assert_eq!(r, [-0x120, -0x120, 10 + 0x120, 10 + 0x120]);
        let mut out = None;
        let mut prepped = 0;
        w17a02_emit_inflated(
            3,
            77,
            |r| {
                *r = [0, 0, 1, 1];
                5
            },
            |id| prepped = id,
            |r| r[2] as u32,
            |k, id, key, a| out = Some((k, id, key, a)),
        );
        assert_eq!(prepped, 5);
        assert_eq!(out, Some((3, 5, 0x121, 77)));
    }

    #[test]
    fn timed_event_fires() {
        let mut t = vec![0u8; 0xb0 * 2 + 0x60];
        let o = 0xb0 + 0x5c;
        t[o..o + 2].copy_from_slice(&7i16.to_le_bytes());
        let mut got = None;
        w17a02_timed_event(
            0,
            3,
            3,
            &t,
            || 1,
            |_| true,
            |_| 12_345,
            |v, p| got = Some((v, p)),
        );
        assert_eq!(got, Some((12_007, 1)));
        got = None;
        w17a02_timed_event(
            1,
            3,
            3,
            &t,
            || 1,
            |_| true,
            |_| 0,
            |v, p| got = Some((v, p)),
        );
        assert_eq!(got, None);
        w17a02_timed_event(
            0,
            3,
            4,
            &t,
            || 1,
            |_| true,
            |_| 0,
            |v, p| got = Some((v, p)),
        );
        assert_eq!(got, None);
        w17a02_timed_event(
            0,
            3,
            3,
            &t,
            || 1,
            |_| false,
            |_| 0,
            |v, p| got = Some((v, p)),
        );
        assert_eq!(got, None);
    }

    #[test]
    fn net_cmds() {
        let mut notes = vec![];
        let mut fin = 0;
        w17a02_net_cmd_4b(
            2,
            || 9,
            || 0,
            |_, _| 1,
            |_| true,
            |c| notes.push(c),
            || fin += 1,
            || {},
        );
        w17a02_net_cmd_4c(
            2,
            || 9,
            || 0,
            |_, _| 1,
            |_| true,
            |c| notes.push(c),
            || fin += 1,
            || {},
        );
        assert_eq!((notes.clone(), fin), (vec![0x4b, 0x4c], 2));
        let mut other = 0;
        w17a02_net_cmd_4b(
            2,
            || 9,
            || 0,
            |_, _| 0,
            |_| true,
            |_| {},
            || {},
            || other += 1,
        );
        w17a02_net_cmd_4c(
            2,
            || 9,
            || 1,
            |_, _| 0,
            |_| true,
            |_| {},
            || {},
            || other += 1,
        );
        w17a02_net_cmd_4c(
            2,
            || 0,
            || 0,
            |_, _| 0,
            |_| true,
            |_| {},
            || {},
            || other += 1,
        );
        assert_eq!(other, 1);
    }
}
