#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent30.rs
// ===========================================================================
// Wave 31 agent 30: small dispatch / counter helpers (queue, `runtime.rs`).

/// `0x006b0680`: resolves a key through a lookup closure; when the found
/// entry's index field is not `-1` its value is returned, otherwise the
/// fallback closure decides.
pub fn w31a30_lookup_or_fallback(
    lookup: impl FnOnce() -> Option<(i32, u32)>,
    fallback: impl FnOnce() -> Option<u32>,
) -> Option<u32> {
    let (idx, value) = lookup()?;
    if idx != -1 { Some(value) } else { fallback() }
}

/// `0x006b2020`: seek-style update of a cursor (mode 0 = set, 1 = add,
/// -1 = saturating subtract).  Returns the new cursor.
pub fn w31a30_seek_cursor(cursor: u32, amount: u32, mode: i32) -> u32 {
    match mode {
        0 => amount,
        1 => cursor.wrapping_add(amount),
        -1 => {
            if amount < cursor {
                cursor - amount
            } else {
                0
            }
        }
        _ => cursor,
    }
}

/// `0x006b25e0`: applies a set/add/saturating-sub on one of two counter
/// slots (selector 0 -> slot at +0x18, selector 4 -> slot at +0x1c).  Returns
/// `false` when the selector is neither, so the caller forwards to the
/// parent handler.
pub fn w31a30_update_counter_slot(
    slots: &mut [u32; 2],
    selector: i32,
    amount: u32,
    mode: i32,
) -> bool {
    let i = match selector {
        0 => 0,
        4 => 1,
        _ => return false,
    };
    slots[i] = w31a30_seek_cursor(slots[i], amount, mode);
    true
}

/// `0x006b2750`: ASCII digits `'0'..='8'` are accepted directly (`Some(true)`);
/// anything else is forwarded to the parent handler (`None`).
pub fn w31a30_digit_gate(selector: i32) -> Option<bool> {
    if (selector.wrapping_sub(0x30) as u32) < 9 {
        Some(true)
    } else {
        None
    }
}

/// `0x006b2780`: splits a flat index into (row, col) using a stride of
/// `0xda`; forwards `row * cols + col` when in range, `None` otherwise.
pub fn w31a30_row_col_index(index: u32, rows: u32, cols: u32) -> Option<u32> {
    let row = index / 0xda;
    let col = index % 0xda;
    if row < rows && col < cols {
        Some(cols * row + col)
    } else {
        None
    }
}

/// `0x006b27e0`: indices below `limit` are forwarded (`None`); others are
/// accepted (`Some(true)`).
pub fn w31a30_below_limit_forward(index: u32, limit: u32) -> Option<bool> {
    if index < limit { None } else { Some(true) }
}

/// `0x006b2890`: runs a prepare step, then the forwarded handler.
pub fn w31a30_prepare_then_forward<R>(prepare: impl FnOnce(), forward: impl FnOnce() -> R) -> R {
    prepare();
    forward()
}

/// `0x006b28f0`: like the previous but the prepare step receives the
/// field at +0x18.
pub fn w31a30_prepare_field_then_forward<R>(
    field: u32,
    prepare: impl FnOnce(u32),
    forward: impl FnOnce() -> R,
) -> R {
    prepare(field);
    forward()
}

/// `0x006b2920`: prepare step runs only when the field at +0x18 is non-zero.
pub fn w31a30_prepare_if_nonzero_then_forward<R>(
    field: u32,
    prepare: impl FnOnce(),
    forward: impl FnOnce() -> R,
) -> R {
    if field != 0 {
        prepare();
    }
    forward()
}

/// `0x006bcdb0`: builds the 8-byte notification packet (tag 0, bytes at
/// +8, +9, +0x94 and the dword at +0xa0).  `None` when no sink is set.
pub fn w31a30_build_notify_packet(
    has_sink: bool,
    b8: u8,
    b9: u8,
    b94: u8,
    d_a0: u32,
) -> Option<[u8; 8]> {
    if !has_sink {
        return None;
    }
    let d = d_a0.to_le_bytes();
    Some([0, b8, b9, b94, d[0], d[1], d[2], d[3]])
}

/// `0x006bd0c0`: validates the current record: when present and its two
/// id bytes match, it is kept; otherwise `Err(())` asks for a re-lookup.
pub fn w31a30_check_current(cur: Option<(u8, u8)>, a: u8, b: u8) -> Result<bool, ()> {
    match cur {
        Some((x, y)) if x == a && y == b => Ok(true),
        Some(_) => Err(()),
        None => Ok(false),
    }
}

/// `0x006be310`: builds the 3-byte packet `[6, b8, b9]`; `None` without a sink.
pub fn w31a30_build_small_packet(has_sink: bool, b8: u8, b9: u8) -> Option<[u8; 3]> {
    if has_sink { Some([6, b8, b9]) } else { None }
}

/// `0x006c53d0`: table slot select; indices 1..=4 map to stride `0x74`
/// byte offsets, else `None`.
pub fn w31a30_table_offset(index: i32) -> Option<usize> {
    if index != 0 && index < 5 {
        Some(index as usize * 0x74)
    } else {
        None
    }
}

#[cfg(test)]
mod tests_wave31_agent30 {
    use super::*;

    #[test]
    fn lookup_and_seek() {
        assert_eq!(
            w31a30_lookup_or_fallback(|| Some((3, 9)), || Some(1)),
            Some(9)
        );
        assert_eq!(
            w31a30_lookup_or_fallback(|| Some((-1, 9)), || Some(1)),
            Some(1)
        );
        assert_eq!(w31a30_lookup_or_fallback(|| None, || Some(1)), None);
        assert_eq!(w31a30_seek_cursor(5, 7, 0), 7);
        assert_eq!(w31a30_seek_cursor(5, 7, 1), 12);
        assert_eq!(w31a30_seek_cursor(5, 7, -1), 0);
        assert_eq!(w31a30_seek_cursor(9, 7, -1), 2);
    }

    #[test]
    fn counters_and_gates() {
        let mut s = [1, 2];
        assert!(w31a30_update_counter_slot(&mut s, 4, 3, 1));
        assert_eq!(s, [1, 5]);
        assert!(!w31a30_update_counter_slot(&mut s, 1, 3, 1));
        assert_eq!(w31a30_digit_gate(0x38), Some(true));
        assert_eq!(w31a30_digit_gate(0x39), None);
        assert_eq!(w31a30_row_col_index(0xda + 3, 2, 10), Some(13));
        assert_eq!(w31a30_row_col_index(0xda * 2, 2, 10), None);
        assert_eq!(w31a30_below_limit_forward(3, 3), Some(true));
        assert_eq!(w31a30_below_limit_forward(2, 3), None);
    }

    #[test]
    fn forwarding_and_packets() {
        assert_eq!(w31a30_prepare_then_forward(|| {}, || 4), 4);
        let mut n = 0;
        w31a30_prepare_field_then_forward(7, |f| n += f, || ());
        w31a30_prepare_if_nonzero_then_forward(0, || n += 100, || ());
        assert_eq!(n, 7);
        assert_eq!(
            w31a30_build_notify_packet(true, 1, 2, 3, 0x04030201),
            Some([0, 1, 2, 3, 1, 2, 3, 4])
        );
        assert_eq!(w31a30_build_notify_packet(false, 1, 2, 3, 0), None);
        assert_eq!(w31a30_check_current(Some((1, 2)), 1, 2), Ok(true));
        assert_eq!(w31a30_check_current(Some((1, 3)), 1, 2), Err(()));
        assert_eq!(w31a30_check_current(None, 1, 2), Ok(false));
        assert_eq!(w31a30_build_small_packet(true, 8, 9), Some([6, 8, 9]));
        assert_eq!(w31a30_table_offset(4), Some(0x1d0));
        assert_eq!(w31a30_table_offset(5), None);
        assert_eq!(w31a30_table_offset(0), None);
    }
}
