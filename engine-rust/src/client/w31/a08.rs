#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent08.rs
// ===========================================================================
// Wave 31 translation batch 08.

/// `0x00775750`: scroll direction from keys or edge state.  Returns `(x, y)`
/// each in -1/0/1.  `edge` is `Some((cur_x, cur_y, ref_x, ref_y))` when the
/// edge-scroll flag is set; otherwise `key(code)` queries pressed keys.
pub fn w31a08_scroll_direction(
    edge: Option<(i32, i32, i32, i32)>,
    key: &dyn Fn(u32) -> bool,
) -> (i32, i32) {
    let (mut x, mut y) = (0i32, 0i32);
    if let Some((cx, cy, rx, ry)) = edge {
        if cx < rx {
            x = -1;
        } else if rx < cx {
            x = 1;
        }
        if ry > cy {
            y = -1;
        } else if cy > ry {
            y = 1;
        }
        return (x, y);
    }
    if key(99) || key(0x22) {
        return (1, 1);
    }
    if key(0x61) || key(0x23) {
        return (-1, 1);
    }
    if key(0x67) || key(0x24) {
        return (-1, -1);
    }
    if key(0x69) || key(0x21) {
        return (1, -1);
    }
    if key(0x28) || key(0x62) {
        y = 1;
    } else if key(0x26) || key(0x68) {
        y = -1;
    }
    if key(0x27) || key(0x66) {
        x = 1;
    } else if key(0x25) || key(100) {
        x = -1;
    }
    (x, y)
}

/// `0x0075fe10`: copies a 0x30-byte rect/state block into a 0x180 offset
/// shadow area and then mirrors the shadow fields into the live fields.
/// Both slices are little-endian u32 words.
pub fn w31a08_copy_state(dst: &mut [u32], src: &[u32]) {
    let rd16 = |w: &[u32], byte: usize| -> u16 { (w[byte / 4] >> ((byte % 4) * 8)) as u16 };
    let wr16 = |w: &mut [u32], byte: usize, v: u16| {
        let sh = (byte % 4) * 8;
        let i = byte / 4;
        w[i] = (w[i] & !(0xffffu32 << sh)) | ((v as u32) << sh);
    };
    dst[0x5d..0x5d + 4].copy_from_slice(&src[..4]);
    for off in [0x10usize, 0x12, 0x14, 0x16, 0x28, 0x2a, 0x2c, 0x2e] {
        let v = rd16(src, off);
        let d = match off {
            0x10 => 0x184,
            0x12 => 0x186,
            0x14 => 0x188,
            0x16 => 0x18a,
            o => o,
        };
        wr16(dst, d, v);
    }
    for i in 0..4 {
        dst[i] = dst[0x5d + i];
    }
    for (a, b) in [
        (0x10usize, 0x184usize),
        (0x12, 0x186),
        (0x14, 0x188),
        (0x16, 0x18a),
    ] {
        let v = rd16(dst, b);
        wr16(dst, a, v);
    }
}

/// `0x0077e780`: sets the flag byte at offset 0x80 of each 0xb0-byte record.
pub fn w31a08_mark_records(table: &mut [u8], count: i32) {
    for i in 0..count.max(0) as usize {
        table[i * 0xb0 + 0x80] = 1;
    }
}

/// `0x007564c0`: with `active`, releases the first handle and, when the
/// guard word is zero, reports a reset of the third.  Returns
/// `(release_first, reset_third)`.
pub fn w31a08_release_guard(active: bool, guard: i32) -> (bool, bool) {
    (active, active && guard == 0)
}

/// `0x007564f0`: releases the first handle only when `active`.
pub fn w31a08_release_if_active(active: bool) -> bool {
    active
}

/// `0x00785e30`: pre-dispatch for message type 0xe.  Returns the action:
/// 1 = call the init helper (sub-code 0), 2 = two refresh calls (sub-code 1),
/// 0 = none; the handler table index is the ushort at +0x3c.
pub fn w31a08_pre_dispatch(msg_type: i16, sub: i32) -> u8 {
    if msg_type != 0xe {
        return 0;
    }
    match sub {
        0 => 1,
        1 => 2,
        _ => 0,
    }
}

/// `0x00785f00`: classification of message types before default handling.
/// Returns `Some(0)` for types 0, 1 and 0xf (handled, no default), else
/// `None`.
pub fn w31a08_msg_early_out(msg_type: i16) -> Option<u32> {
    match msg_type {
        0 | 1 | 0xf => Some(0),
        _ => None,
    }
}

#[cfg(test)]
mod tests_wave31_agent08 {
    use super::*;

    #[test]
    fn scroll() {
        assert_eq!(
            w31a08_scroll_direction(Some((1, 5, 3, 2)), &|_| false),
            (-1, 1)
        );
        assert_eq!(
            w31a08_scroll_direction(Some((3, 2, 3, 2)), &|_| false),
            (0, 0)
        );
        assert_eq!(w31a08_scroll_direction(None, &|k| k == 99), (1, 1));
        assert_eq!(w31a08_scroll_direction(None, &|k| k == 0x23), (-1, 1));
        assert_eq!(w31a08_scroll_direction(None, &|k| k == 0x69), (1, -1));
        assert_eq!(
            w31a08_scroll_direction(None, &|k| k == 0x28 || k == 0x25),
            (-1, 1)
        );
        assert_eq!(w31a08_scroll_direction(None, &|_| false), (0, 0));
    }

    #[test]
    fn copy() {
        let mut src = [0u32; 12];
        src[0] = 7;
        src[4] = 0x0002_0001;
        src[5] = 0x0004_0003;
        let mut dst = vec![0u32; 0x70];
        w31a08_copy_state(&mut dst, &src);
        assert_eq!(dst[0], 7);
        assert_eq!(dst[0x5d], 7);
        assert_eq!(dst[4], 0x0002_0001);
        assert_eq!(dst[0x61], 0x0002_0001);
        assert_eq!(dst[5], 0x0004_0003);
    }

    #[test]
    fn misc() {
        let mut t = vec![0u8; 0xb0 * 3];
        w31a08_mark_records(&mut t, 2);
        assert_eq!((t[0x80], t[0x130], t[0x1e0]), (1, 1, 0));
        assert_eq!(w31a08_release_guard(true, 0), (true, true));
        assert_eq!(w31a08_release_guard(true, 1), (true, false));
        assert!(!w31a08_release_if_active(false));
        assert_eq!(w31a08_pre_dispatch(0xe, 1), 2);
        assert_eq!(w31a08_pre_dispatch(3, 0), 0);
        assert_eq!(w31a08_msg_early_out(0xf), Some(0));
        assert_eq!(w31a08_msg_early_out(3), None);
    }
}
