#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave16/agent02.rs
// ===========================================================================
// Wave 16 translation batch 02 (client.rs window, queue mode).
//
// Pure helpers; unresolved callees are closure parameters.

/// `0x005348f0` (table half): lazily builds the 44-entry u16 lookup table
/// (retail data `0x00f8_dc68`) from the -1 terminated group list.  `list` is the
/// u16 array whose element 0 is a header; groups start at index 1 and each
/// ends in `0xffff`.  The table value for the first element of a group is
/// `start_index + 1`; entries still `-1` afterwards become 0.
pub fn w16a02_build_group_table(table: &mut [i16; 44], list: &[u16]) {
    let at = |i: usize| list.get(i).copied().unwrap_or(0xffff);
    let mut idx = 1usize;
    if at(1) != 0xffff {
        let mut cur = at(1) as usize;
        loop {
            if let Some(e) = table.get_mut(cur) {
                *e = (idx + 1) as i16;
            }
            // skip to the group terminator (first test uses `cur`, never -1)
            idx += 1;
            while at(idx) != 0xffff {
                idx += 1;
            }
            let next = at(idx + 1);
            cur = next as usize;
            idx += 1;
            if next == 0xffff {
                break;
            }
        }
    }
    for e in table.iter_mut() {
        if *e == -1 {
            *e = 0;
        }
    }
}

/// `0x005348f0`: validation gate for an order/build request on `id`.
/// Returns `Ok(table_value)` when every check passes and the caller should
/// dispatch (the original calls the 5-arg dispatcher with it), or
/// `Err(code)` with the error code the original passes to its set-last-error
/// helper.  Checks run in retail order; callees are closures.
#[allow(clippy::too_many_arguments)]
pub fn w16a02_check_request(
    id: u16,
    player: u32,
    owner: u8,
    table: &mut [i16; 44],
    list: &[u16],
    flags: u32,
    mut busy_a: impl FnMut() -> bool,
    mut busy_b: impl FnMut() -> bool,
    mut req_a: impl FnMut(u32, u16) -> bool,
    mut blocked: impl FnMut(u16, u32) -> bool,
    mut req_b: impl FnMut(u32, u16) -> bool,
) -> Result<u16, u8> {
    if id > 0x2b {
        return Err(0x11);
    }
    if table[id as usize] == -1 {
        w16a02_build_group_table(table, list);
    }
    if player != owner as u32 {
        return Err(1);
    }
    if !busy_a() {
        return Err(0x14);
    }
    if busy_b() {
        return Err(10);
    }
    if !req_a(player, id) && flags & 0x1000 == 0 {
        return Err(2);
    }
    if blocked(id, player) {
        return Err(0xc);
    }
    if !req_b(player, id) && flags & 0x1000 == 0 {
        let v = table[id as usize];
        if v != 0 {
            return Ok(v as u16);
        }
        return Err(0x17);
    }
    Err(0xd)
}

/// `0x0058b490`: releases the object held at `this+0xc` and clears the slot
/// when the object's field `+0x20` is zero afterwards.  `release` models
/// the callee; `field20` reads the object's `+0x20` field.
pub fn w16a02_release_held(
    slot: &mut u32,
    mut release: impl FnMut(),
    field20: impl Fn(u32) -> u32,
) {
    if *slot != 0 {
        release();
        if field20(*slot) == 0 {
            *slot = 0;
        }
    }
}

/// `0x0058e040`: stores `id` into the first free (0xe4) slot of the 5-entry
/// u16 array at `this+0x98`, scanning from `current` (`this+0xa4`) with
/// wrap-around.  `try_assign(player, id, is_current, 1)` models the gate
/// callee; `id_flag(id)` reads the first byte of the 4-byte record in the
/// retail id table; `on_unflagged(player)` runs when bit 0 is clear.
pub fn w16a02_assign_free_slot(
    slots: &mut [u16; 5],
    current: u8,
    player: u8,
    id: u16,
    mut try_assign: impl FnMut(u8, u16, bool, u32) -> bool,
    id_flag: impl Fn(u16) -> u8,
    mut on_unflagged: impl FnMut(u8),
) -> bool {
    let mut left = 5;
    let mut s = current as usize;
    loop {
        left -= 1;
        if s >= 5 {
            s = 0;
        }
        if slots[s] == 0xe4 {
            break;
        }
        s += 1;
        if left == 0 {
            return false;
        }
    }
    if s == 5 {
        return false;
    }
    if !try_assign(player, id, s == current as usize, 1) {
        return false;
    }
    slots[s] = id;
    if id_flag(id) & 1 == 0 {
        on_unflagged(player);
    }
    true
}

/// `0x005b5980`: true unless the first-slot id equals 0xe4 and both
/// follow-up probes fail.
pub fn w16a02_slot_ready(
    first_slot_id: i16,
    mut probe_a: impl FnMut() -> bool,
    mut probe_b: impl FnMut() -> bool,
) -> bool {
    if first_slot_id == 0xe4 && !probe_a() && !probe_b() {
        return false;
    }
    true
}

/// State written by [`w16a02_begin_tracking`] (retail globals).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W16a02Tracking {
    /// First argument (stored at the `..38e4` global).
    pub value: u32,
    /// Second argument (stored at the `..38e8` global).
    pub flag: i32,
    /// 0x100, or 0x200 when `flag != 0`.
    pub mode: u32,
    /// `(flag_low16 << 16) | value_high16` handed to the registration callee
    /// along with `value`, `mode` and a handler.
    pub packed: u32,
}

/// `0x005c69f0`: if any of the three probes (ids 0xb0..=0xb2, group 0xb)
/// succeeds, records `(value, flag)` and returns the state to register.
pub fn w16a02_begin_tracking(
    value: u32,
    flag: i32,
    mut probe: impl FnMut(u32, u32) -> bool,
) -> Option<W16a02Tracking> {
    if !(probe(0xb0, 0xb) || probe(0xb1, 0xb) || probe(0xb2, 0xb)) {
        return None;
    }
    let mode = if flag != 0 { 0x200 } else { 0x100 };
    Some(W16a02Tracking {
        value,
        flag,
        mode,
        packed: ((flag as u32) << 16) | ((value >> 16) & 0xffff),
    })
}

/// `0x005c7890`: stores `flag` and returns the dialog flag word to
/// register (0x120 when `flag != 0`, else 0x800 after `on_zero(player)`).
pub fn w16a02_dialog_flags(flag: i32, player: u8, mut on_zero: impl FnMut(u8)) -> (i32, u32) {
    if flag != 0 {
        return (flag, 0x120);
    }
    on_zero(player);
    (flag, 0x800)
}

/// Record stride of the 0x4e8-byte per-player table at `0x00fa_a210`.
pub const W16A02_PLAYER_STRIDE: usize = 0x4e8;

/// `0x00635c00`: reads the u32 at record offset +4 of player `idx` (base
/// `0x00fa_a210`).  `rows` starts at the base.
pub fn w16a02_player_u32(rows: &[u8], idx: usize) -> Option<u32> {
    let o = idx * W16A02_PLAYER_STRIDE + 4;
    Some(u32::from_le_bytes(rows.get(o..o + 4)?.try_into().ok()?))
}

/// `0x00635c20`: reads the u16 at record offset +2 of player `idx` (base
/// `0x00fa_a210`).
pub fn w16a02_player_u16(rows: &[u8], idx: usize) -> Option<u16> {
    let o = idx * W16A02_PLAYER_STRIDE + 2;
    Some(u16::from_le_bytes(rows.get(o..o + 2)?.try_into().ok()?))
}

/// Stride of the 0x30-byte table at `0x011a_c900` (ids 1..=0xf9).
pub const W16A02_REC30: usize = 0x30;

/// `0x0076a2e0`: byte at +0x1c of record `id` (`rows` starts at the base),
/// 0 for id 0 or >= 0xfa.
pub fn w16a02_rec_byte(rows: &[u8], id: u32) -> u8 {
    if id == 0 || id >= 0xfa {
        return 0;
    }
    rows.get(id as usize * W16A02_REC30 + 0x1c)
        .copied()
        .unwrap_or(0)
}

/// `0x00769b00`: capped value for record `id`: `b = byte(+0x1c)`,
/// `t = u32(+0x20) / 500`; when `limit <= t` returns `min(t, 2*b)`, else `b`.
/// `limit` is the result of the retail helper called with `param_2`.
pub fn w16a02_rec_capped(rows: &[u8], id: u32, limit: u32) -> u32 {
    if id == 0 || id > 0xf9 {
        return 0;
    }
    let base = id as usize * W16A02_REC30;
    let b = rows.get(base + 0x1c).copied().unwrap_or(0) as u32;
    let t = rows
        .get(base + 0x20..base + 0x24)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
        .unwrap_or(0)
        / 500;
    if limit <= t {
        return t.min(b * 2);
    }
    b
}

/// `0x00769f20`: 1-based index of the point nearest to the reference.
/// `human` is the result of the first callee: when false the reference is
/// `slot_pos` and the count is 8, otherwise `pos` and `count`.  `points`
/// holds `(x, y)` i16 pairs at the 0x30 stride; `dist(x1, x2, y1, y2)`
/// models the distance callee.  Ties pick the later index; 0 if none.
pub fn w16a02_nearest_point(
    human: bool,
    pos: (i32, i32),
    slot_pos: (i32, i32),
    count: u32,
    points: &[(i16, i16)],
    mut dist: impl FnMut(i32, i32, i32, i32) -> u32,
) -> u32 {
    let (pos, count) = if human { (pos, count) } else { (slot_pos, 8) };
    let mut best = 0;
    let mut best_d = 99_999_999u32;
    for i in 1..=count {
        let (px, py) = points.get(i as usize - 1).copied().unwrap_or((0, 0));
        let d = dist(pos.0, px as i32, pos.1, py as i32);
        if d <= best_d {
            best = i;
            best_d = d;
        }
    }
    best
}

/// `0x0076a190`: rebuilds the de-duplicated u16 list (retail array
/// `0x011a_f800`) from up to 8 candidate slots.  For each slot `i` whose
/// `row[i] == 0` and `lookup(i, x, y)` is non-zero, converts it with `conv`
/// and tries `k = 2..7`: `query(conv, k)` yields `(n, ids)`; the first `k`
/// with `0 < n <= limit` appends the ids not already present, then stops.
pub fn w16a02_collect_ids(
    row: &[u8; 8],
    pos: (i32, i32),
    limit: u32,
    mut lookup: impl FnMut(u32, i32, i32) -> i32,
    mut conv: impl FnMut(i32) -> u16,
    mut query: impl FnMut(u16, u32) -> (u32, [i16; 4]),
) -> Vec<i16> {
    let mut out: Vec<i16> = Vec::new();
    for i in 0..8u32 {
        if row[i as usize] != 0 {
            continue;
        }
        let r = lookup(i, pos.0, pos.1);
        if r == 0 {
            continue;
        }
        let c = conv(r);
        for k in 2..7 {
            let (n, ids) = query(c, k);
            if n != 0 && n <= limit {
                for &id in ids.iter().take(n as usize) {
                    if !out.contains(&id) {
                        out.push(id);
                    }
                }
                break;
            }
        }
    }
    out
}

// Skipped (no translation):
// FUN_00534c20 - ends in noreturn dispatcher call
// FUN_005c7850 - bare call chain
// FUN_00633320 - noreturn exception funclet
// FUN_00638ba0 - bare noreturn call
// FUN_00711c20 - noreturn file-loading glue
// FUN_00711e60 - noreturn file-loading glue
// FUN_007127a0 - noreturn file-loading glue
// FUN_00c6be00 - printf glue, noreturn end

#[cfg(test)]
mod tests_wave16_agent02 {
    use super::*;

    #[test]
    fn table_build() {
        let mut t = [-1i16; 44];
        // header, group {3,4,-1}, group {7,-1}, end
        let list = [0u16, 3, 4, 0xffff, 7, 0xffff, 0xffff];
        w16a02_build_group_table(&mut t, &list);
        assert_eq!(t[3], 2);
        assert_eq!(t[7], 5);
        assert_eq!(t[0], 0);
    }

    #[test]
    fn request_checks() {
        let mut t = [0i16; 44];
        t[5] = 9;
        let r = |id, owner, f| {
            let mut tt = t;
            w16a02_check_request(
                id,
                1,
                owner,
                &mut tt,
                &[],
                f,
                || true,
                || false,
                |_, _| true,
                |_, _| false,
                |_, _| false,
            )
        };
        assert_eq!(r(0x30, 1, 0), Err(0x11));
        assert_eq!(r(5, 2, 0), Err(1));
        assert_eq!(r(5, 1, 0), Ok(9));
        assert_eq!(r(6, 1, 0), Err(0x17));
    }

    #[test]
    fn slots() {
        let mut s = [1, 0xe4, 0xe4, 2, 3];
        let mut called = 0;
        assert!(w16a02_assign_free_slot(
            &mut s,
            2,
            0,
            7,
            |_, _, cur, _| cur,
            |_| 0,
            |_| called += 1
        ));
        assert_eq!(s[2], 7);
        assert_eq!(called, 1);
        let mut full = [1; 5];
        assert!(!w16a02_assign_free_slot(
            &mut full,
            0,
            0,
            7,
            |_, _, _, _| true,
            |_| 1,
            |_| {}
        ));
    }

    #[test]
    fn misc() {
        let mut slot = 5;
        w16a02_release_held(&mut slot, || {}, |_| 0);
        assert_eq!(slot, 0);
        assert!(!w16a02_slot_ready(0xe4, || false, || false));
        assert!(w16a02_slot_ready(1, || false, || false));
        let t = w16a02_begin_tracking(0x1234_5678, 1, |a, _| a == 0xb2).unwrap();
        assert_eq!((t.mode, t.packed), (0x200, 0x0001_1234));
        assert!(w16a02_begin_tracking(1, 0, |_, _| false).is_none());
        assert_eq!(w16a02_dialog_flags(0, 3, |_| {}), (0, 0x800));
        assert_eq!(w16a02_dialog_flags(2, 3, |_| {}), (2, 0x120));
    }

    #[test]
    fn tables() {
        let mut rows = vec![0u8; 0x30 * 4];
        rows[0x30 + 0x1c] = 10;
        rows[0x30 + 0x20..0x30 + 0x24].copy_from_slice(&5000u32.to_le_bytes());
        assert_eq!(w16a02_rec_byte(&rows, 1), 10);
        assert_eq!(w16a02_rec_byte(&rows, 0), 0);
        assert_eq!(w16a02_rec_capped(&rows, 1, 5), 10); // t=10, min(10,20)
        assert_eq!(w16a02_rec_capped(&rows, 1, 50), 10);
        let mut p = vec![0u8; W16A02_PLAYER_STRIDE * 2];
        p[W16A02_PLAYER_STRIDE + 4] = 7;
        p[W16A02_PLAYER_STRIDE + 2] = 9;
        assert_eq!(w16a02_player_u32(&p, 1), Some(7));
        assert_eq!(w16a02_player_u16(&p, 1), Some(9));
        assert_eq!(w16a02_player_u16(&p, 2), None);
    }

    #[test]
    fn nearest_and_collect() {
        let pts = [(0, 0), (10, 10), (3, 3)];
        let d = |x1: i32, x2: i32, y1: i32, y2: i32| ((x1 - x2).abs() + (y1 - y2).abs()) as u32;
        assert_eq!(w16a02_nearest_point(true, (4, 4), (0, 0), 3, &pts, d), 3);
        let row = [0, 1, 0, 0, 0, 0, 0, 0];
        let ids = w16a02_collect_ids(
            &row,
            (0, 0),
            3,
            |i, _, _| if i < 3 { 1 } else { 0 },
            |r| r as u16,
            |_, k| {
                if k == 3 {
                    (2, [5, 6, 0, 0])
                } else {
                    (0, [0; 4])
                }
            },
        );
        assert_eq!(ids, vec![5, 6]);
    }
}
