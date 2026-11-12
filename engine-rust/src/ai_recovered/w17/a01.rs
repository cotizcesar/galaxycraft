#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave17/agent01.rs
// ===========================================================================
// Wave 17 translation batch 01.
//
// Window: queue, module `ai_recovered.rs`.  Pure helpers; unresolved
// callees and globals are caller-supplied parameters.

/// `0x00695df0`: classifies a 16-bit id by its flag byte (flag table
/// `DAT_00f91fe8`, one byte per id; the caller passes `flags[id]`).  Bit 0
/// gives `hi << 8`, bit 1 gives `hi << 8 | 1`, otherwise
/// `((hi << 8 | (bit2 == 0)) * 2) + 2`.
pub fn w17a01_flag_class(id: u16, flags: &[u8]) -> u32 {
    let b = flags.get(id as usize).copied().unwrap_or(0);
    let hi = (id >> 8) as u32;
    if b & 1 != 0 {
        return hi << 8;
    }
    if b & 2 != 0 {
        return (hi << 8) | 1;
    }
    (((hi << 8) | ((b & 4) == 0) as u32).wrapping_mul(2)).wrapping_add(2)
}

/// `0x00695930`: effective value for `(id, player)`.  `flags` is the
/// `DAT_00f91fe8` table; `table` is the decoded global block (offsets
/// 0x3054/0x3084 for flag 1, 0x30e4/0x3114 for flag 2, 0x3174/0x31a4 for
/// flag 4, stride 4 per `param_2 & 0xff`).  `base` is `FUN_00695a70`
/// evaluated through `base_fn`; `override_flag` is `DAT_01165d00 & 0x20000`.
pub fn w17a01_effective_value(
    id: u16,
    param_2: u32,
    flags: &[u8],
    table: &[u8],
    override_flag: bool,
    mut base_fn: impl FnMut(u32, u32) -> u32,
) -> i32 {
    let rd = |off: usize| -> u32 {
        table
            .get(off..off + 4)
            .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
            .unwrap_or(0)
    };
    let b = flags.get(id as usize).copied().unwrap_or(0);
    let slot = ((param_2 & 0xff) * 4) as usize;
    let (mut v, sub) = if b & 1 != 0 {
        (None, Some((0x3054, 0x3084)))
    } else if b & 2 != 0 {
        (None, Some((0x30e4, 0x3114)))
    } else if b & 4 != 0 {
        (None, Some((0x3174, 0x31a4)))
    } else {
        (Some(0i32), None)
    };
    let mut sub_v = 0i32;
    if let Some((cap_off, sub_off)) = sub {
        let cap = rd(cap_off + slot);
        let a = base_fn(id as u32, param_2);
        v = Some(if a < cap {
            base_fn(id as u32, param_2)
        } else {
            cap
        } as i32);
        sub_v = rd(sub_off + slot) as i32;
    }
    let mut out = v.unwrap_or(0);
    if override_flag {
        out = base_fn(id as u32, param_2) as i32;
    }
    out.wrapping_sub(sub_v)
}

/// `0x006ae4e0`: records a "SC" tag marker.  `state` is the 8-byte record
/// at `DAT_01165f54` (byte 0 flag, dword at +4); `active` is
/// `DAT_01165f58 != 0`.  `src` is the source block: dword at 0, tag bytes
/// at 0x1e/0x1f.
pub fn w17a01_set_sc_marker(active: bool, state: Option<&mut [u8]>, src: &[u8]) {
    if !active {
        return;
    }
    let Some(st) = state else { return };
    if st.len() < 8 || src.len() < 0x20 {
        return;
    }
    let is_sc = src[0x1e] == b'S' && src[0x1f] == b'C';
    st[0] = is_sc as u8;
    if is_sc {
        st[4..8].copy_from_slice(&src[0..4]);
    }
}

/// `0x006ae520`: clears the marker flag byte when the record exists and
/// `active` (`DAT_01165f58 != 0`).
pub fn w17a01_clear_marker(active: bool, state: Option<&mut [u8]>) {
    if active
        && let Some(st) = state
        && !st.is_empty()
    {
        st[0] = 0;
    }
}

/// `0x0070fe00`: returns true when `param_2 == 0x500` and the callee
/// (`FUN_00984770` with the value of `FUN_00661410`) returns non-zero.
pub fn w17a01_check_500(param_2: i32, mut probe: impl FnMut() -> bool) -> bool {
    param_2 == 0x500 && probe()
}

/// `0x00715a40`: calls the copy helper `FUN_00c671d0` and returns its
/// first argument unchanged.
pub fn w17a01_pass_through(param_1: u32, call: impl FnOnce(u32)) -> u32 {
    call(param_1);
    param_1
}

/// `0x0076a310`: byte at `0x011ac91d + index * 0x30`; `rows` starts at
/// that address.
pub fn w17a01_row_byte(rows: &[u8], index: usize) -> u8 {
    rows.get(index * 0x30).copied().unwrap_or(0)
}

/// `0x007aaf50`: sets the global flag `DAT_011b8700` to 1.
pub fn w17a01_set_flag(flag: &mut u32) {
    *flag = 1;
}

/// `0x0094fec0`: stores `v` into the byte global `DAT_011cacb5`.
pub fn w17a01_store_byte(slot: &mut u8, v: u8) {
    *slot = v;
}

#[cfg(test)]
mod tests_wave17_agent01 {
    use super::*;

    #[test]
    fn flag_class() {
        let mut f = vec![0u8; 0x300];
        f[0x201] = 1;
        f[0x202] = 2;
        f[0x203] = 4;
        assert_eq!(w17a01_flag_class(0x201, &f), 0x200);
        assert_eq!(w17a01_flag_class(0x202, &f), 0x201);
        assert_eq!(w17a01_flag_class(0x203, &f), (0x200 * 2 + 2));
        assert_eq!(w17a01_flag_class(0x200, &f), (0x200 | 1) * 2 + 2);
    }

    #[test]
    fn effective() {
        let mut t = vec![0u8; 0x3200];
        t[0x3054 + 8..0x3054 + 12].copy_from_slice(&10u32.to_le_bytes());
        t[0x3084 + 8..0x3084 + 12].copy_from_slice(&3u32.to_le_bytes());
        let mut f = vec![0u8; 4];
        f[1] = 1;
        assert_eq!(w17a01_effective_value(1, 2, &f, &t, false, |_, _| 5), 2);
        assert_eq!(w17a01_effective_value(1, 2, &f, &t, false, |_, _| 50), 7);
        assert_eq!(w17a01_effective_value(1, 2, &f, &t, true, |_, _| 50), 47);
        assert_eq!(w17a01_effective_value(0, 2, &f, &t, false, |_, _| 50), 0);
    }

    #[test]
    fn marker() {
        let mut st = [9u8; 8];
        let mut src = [0u8; 0x20];
        src[..4].copy_from_slice(&7u32.to_le_bytes());
        src[0x1e] = b'S';
        src[0x1f] = b'C';
        w17a01_set_sc_marker(true, Some(&mut st), &src);
        assert_eq!(st[0], 1);
        assert_eq!(st[4], 7);
        src[0x1f] = b'X';
        w17a01_set_sc_marker(true, Some(&mut st), &src);
        assert_eq!(st[0], 0);
        w17a01_set_sc_marker(false, Some(&mut st), &src);
        st[0] = 5;
        w17a01_clear_marker(true, Some(&mut st));
        assert_eq!(st[0], 0);
    }

    #[test]
    fn small() {
        assert!(w17a01_check_500(0x500, || true));
        assert!(!w17a01_check_500(0x501, || true));
        assert_eq!(w17a01_pass_through(7, |_| {}), 7);
        let mut r = vec![0u8; 0x60];
        r[0x30] = 4;
        assert_eq!(w17a01_row_byte(&r, 1), 4);
        let mut fl = 0;
        w17a01_set_flag(&mut fl);
        assert_eq!(fl, 1);
        let mut b = 0;
        w17a01_store_byte(&mut b, 3);
        assert_eq!(b, 3);
    }
}
