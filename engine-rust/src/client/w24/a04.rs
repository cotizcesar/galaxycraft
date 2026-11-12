#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent04.rs
// ===========================================================================
// Wave 24 translation batch 04: client option/state helpers.
//
// Skipped (bnet/SEH glue/non-returning chains): FUN_00773220, FUN_00774dd0,
// FUN_0077ac50, FUN_0077ae90, FUN_0077b0d0, FUN_0077b310, FUN_0077b550,
// FUN_0077b790, FUN_0077dec0, FUN_0077e600, FUN_0077f7a0.

/// `0x007730e0`: byte lookup in a table blob.  `a` selects the row; ids below
/// `0x18` use stride `0x18` at offset `0xdd34`, others stride `0x14` at
/// `0xff48`.
pub fn w24a04_table_byte(blob: &[u8], a: i32, b: u16) -> Option<u8> {
    let idx = if b < 0x18 {
        (b as i64) + (a as i64) * 0x18 + 0xdd34
    } else {
        (b as i64) + (a as i64) * 0x14 + 0xff48
    };
    usize::try_from(idx).ok().and_then(|i| blob.get(i).copied())
}

/// State block written by `0x00775910`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W24a04Clamp {
    pub active: u32,
    pub x: u32,
    pub y: u32,
    pub max_x: i32,
    pub max_y: i32,
}

/// `0x00775910`: when the blocking check is false, latch the 8-aligned
/// origin and unlimited maxima; the previous callback is invoked first
/// (caller-supplied `call_prev`).  `reset_scale` models the call that sets
/// scale 1.0 (`0x3f800000`).
pub fn w24a04_set_clamp(
    st: &mut W24a04Clamp,
    x: u32,
    y: u32,
    blocked: bool,
    call_prev: &mut dyn FnMut(),
    reset_scale: &mut dyn FnMut(),
) {
    call_prev();
    if !blocked {
        reset_scale();
        st.x = x & 0xffff_fff8;
        st.active = 1;
        st.y = y & 0xffff_fff8;
        st.max_x = 0x7fff_ffff;
        st.max_y = 0x7fff_ffff;
    }
}

/// `0x00775d70`: returns `Some(player)` when the follow-up (non-returning)
/// call is made: check passes, player differs from the local one and `< 8`.
pub fn w24a04_player_notify(player: i32, check: bool, local_player: i32) -> Option<i32> {
    (check && player != local_player && player < 8).then_some(player)
}

/// `0x00776b50`: stores `unit` (nonzero id) into slot `slot + player * 12`.
/// Returns `false` on rejection.  For `slot > 0` the unit must pass
/// `selectable` and its byte at +0x4c must equal `local_byte`.  The second
/// closure models the post-store refresh when `player_ok` is true.
#[allow(clippy::too_many_arguments)]
pub fn w24a04_assign_group_slot(
    slots: &mut [i32],
    unit: i32,
    player: i32,
    slot: i32,
    rejected: bool,
    selectable: bool,
    unit_byte_4c: u8,
    local_byte: u8,
    player_ok: bool,
    refresh: &mut dyn FnMut(),
) -> bool {
    if unit == 0 || player > 7 || slot > 0xb || rejected {
        return false;
    }
    if slot > 0 && (!selectable || unit_byte_4c != local_byte) {
        return false;
    }
    let idx = slot as i64 + player as i64 * 12;
    let Some(cell) = usize::try_from(idx).ok().and_then(|i| slots.get_mut(i)) else {
        return false;
    };
    *cell = unit;
    if player_ok {
        refresh();
    }
    true
}

/// `0x007767b0`: walks the 12 slots of `player` (< 8), calling `release` for
/// each nonzero entry and clearing it, stopping at the first zero.  When
/// `player_ok` is false the whole row is zeroed instead (0x30 bytes).
pub fn w24a04_clear_group_row(
    slots: &mut [i32],
    player: i32,
    player_ok: bool,
    release: &mut dyn FnMut(i32),
) {
    if !(0..8).contains(&player) {
        return;
    }
    let base = player as usize * 12;
    let Some(row) = slots.get_mut(base..base + 12) else {
        return;
    };
    if !player_ok {
        row.fill(0);
        return;
    }
    for cell in row.iter_mut() {
        if *cell == 0 {
            return;
        }
        release(*cell);
        *cell = 0;
    }
}

/// `0x0077a970`: constructor initialising a 0x1f-word object.  `ctor_sub`
/// models the member constructor call; the string pointer word (`0x11`)
/// stores the word index of the inline buffer (`0x14`).
pub fn w24a04_init_object(obj: &mut [u32; 0x1f], first: u32, ctor_sub: &mut dyn FnMut()) {
    obj[0] = 2;
    obj[1] = first;
    obj[8] = 2;
    ctor_sub();
    obj[0x12] = 0;
    obj[0x11] = 0x14;
    obj[0x13] = 0x8000_000f;
    obj[0x14] &= !0xff;
    obj[0x18] = (obj[0x18] & 0xff00_0000) | 0x100;
    obj[0x19] = 8;
    obj[0x1b] = 0;
    obj[0x1a] = 0;
    obj[0x1c] = 0x3f80_0000;
    obj[0x1d] = 1;
    obj[0x1e] = 1;
}

/// Volumes computed by `0x0077e0e0`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct W24a04Volumes {
    pub music: f32,
    pub sfx: f32,
    pub voice: f32,
}

/// `0x0077e0e0`: each volume is `value / 100` when its enable flag is set,
/// else 0.  Inputs are `(enabled, value)` pairs for music, sfx, voice.
pub fn w24a04_volumes(music: (bool, i32), sfx: (bool, i32), voice: (bool, i32)) -> W24a04Volumes {
    let f = |(on, v): (bool, i32)| if on { v as f32 / 100.0 } else { 0.0 };
    W24a04Volumes {
        music: f(music),
        sfx: f(sfx),
        voice: f(voice),
    }
}

/// `0x0077e5c0`: little-endian u32 at `index * 0xb0 + 0x88` of a table.
pub fn w24a04_record_field(table: &[u8], index: i32) -> Option<u32> {
    let off = usize::try_from(index as i64 * 0xb0 + 0x88).ok()?;
    let b = table.get(off..off + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// `0x0077ea40`: true when option A and B are enabled and C's value is nonzero.
pub fn w24a04_all_enabled(a_on: bool, b_on: bool, c_value: i32) -> bool {
    a_on && b_on && c_value != 0
}

#[cfg(test)]
mod tests_wave24_agent04 {
    use super::*;

    #[test]
    fn table_and_record() {
        let mut blob = vec![0u8; 0x10000 + 0x200];
        blob[0xdd34 + 0x18 + 3] = 7;
        blob[0xff48 + 0x14 + 0x18] = 9;
        assert_eq!(w24a04_table_byte(&blob, 1, 3), Some(7));
        assert_eq!(w24a04_table_byte(&blob, 1, 0x18), Some(9));
        assert_eq!(w24a04_table_byte(&[], 0, 0), None);
        let mut t = vec![0u8; 0x88 + 0xb0 + 4];
        t[0x88 + 0xb0..0x88 + 0xb0 + 4].copy_from_slice(&5u32.to_le_bytes());
        assert_eq!(w24a04_record_field(&t, 1), Some(5));
        assert_eq!(w24a04_record_field(&t, 9), None);
    }

    #[test]
    fn clamp_and_notify() {
        let mut st = W24a04Clamp::default();
        let (mut p, mut r) = (0, 0);
        w24a04_set_clamp(&mut st, 0x1f, 0x2f, false, &mut || p += 1, &mut || r += 1);
        assert_eq!((st.x, st.y, st.active, p, r), (0x18, 0x28, 1, 1, 1));
        assert_eq!(st.max_x, 0x7fff_ffff);
        let mut st2 = W24a04Clamp::default();
        w24a04_set_clamp(&mut st2, 9, 9, true, &mut || {}, &mut || {});
        assert_eq!(st2, W24a04Clamp::default());
        assert_eq!(w24a04_player_notify(3, true, 1), Some(3));
        assert_eq!(w24a04_player_notify(8, true, 1), None);
        assert_eq!(w24a04_player_notify(1, true, 1), None);
        assert_eq!(w24a04_player_notify(2, false, 1), None);
    }

    #[test]
    fn group_slots() {
        let mut s = [0i32; 96];
        let mut n = 0;
        assert!(w24a04_assign_group_slot(
            &mut s,
            5,
            2,
            0,
            false,
            false,
            0,
            0,
            true,
            &mut || n += 1
        ));
        assert_eq!(s[24], 5);
        assert_eq!(n, 1);
        assert!(!w24a04_assign_group_slot(
            &mut s,
            5,
            2,
            1,
            false,
            true,
            1,
            0,
            true,
            &mut || {}
        ));
        assert!(!w24a04_assign_group_slot(
            &mut s,
            5,
            8,
            0,
            false,
            true,
            0,
            0,
            true,
            &mut || {}
        ));
        assert!(!w24a04_assign_group_slot(
            &mut s,
            5,
            2,
            12,
            false,
            true,
            0,
            0,
            true,
            &mut || {}
        ));
        assert!(!w24a04_assign_group_slot(
            &mut s,
            0,
            2,
            0,
            false,
            true,
            0,
            0,
            true,
            &mut || {}
        ));
        s[25] = 6;
        s[27] = 8;
        let mut rel = vec![];
        w24a04_clear_group_row(&mut s, 2, true, &mut |u| rel.push(u));
        assert_eq!(rel, vec![5, 6]);
        assert_eq!(s[27], 8);
        w24a04_clear_group_row(&mut s, 2, false, &mut |_| {});
        assert_eq!(s[27], 0);
    }

    #[test]
    fn init_and_volumes() {
        let mut o = [0xffff_ffffu32; 0x1f];
        let mut called = false;
        w24a04_init_object(&mut o, 77, &mut || called = true);
        assert!(called);
        assert_eq!((o[0], o[1], o[8], o[0x13]), (2, 77, 2, 0x8000_000f));
        assert_eq!(o[0x18], 0xff00_0100);
        assert_eq!(o[0x1c], 0x3f80_0000);
        let v = w24a04_volumes((true, 50), (false, 80), (true, 100));
        assert_eq!(
            v,
            W24a04Volumes {
                music: 0.5,
                sfx: 0.0,
                voice: 1.0
            }
        );
        assert!(w24a04_all_enabled(true, true, 1));
        assert!(!w24a04_all_enabled(true, true, 0));
        assert!(!w24a04_all_enabled(false, true, 1));
    }
}
