#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent21.rs
// ===========================================================================
// Wave 22 agent 21: image-resampler ring/cache helpers and menu flag glue.

/// `0x0069a8c0`: sets the flag at `DAT_01165f09` to 1, then runs the refresh.
pub fn w22a21_set_flag_and_refresh(flag: &mut bool, refresh: impl FnOnce()) {
    *flag = true;
    refresh();
}

/// `0x0069bd50`: clears the flag at `DAT_01165f09`, then runs the refresh.
pub fn w22a21_clear_flag_and_refresh(flag: &mut bool, refresh: impl FnOnce()) {
    *flag = false;
    refresh();
}

/// `0x0069e4e0`: records a new value (skipped when unchanged and `force` is
/// zero); when the derived key changes, calls `on_key_change`, then `refresh`.
/// Returns whether anything was updated.
pub fn w22a21_update_value(
    value: &mut i32,
    key: &mut i32,
    new_value: i32,
    force: u8,
    derive_key: impl FnOnce() -> i32,
    on_key_change: impl FnOnce(),
    refresh: impl FnOnce(),
) -> bool {
    if *value == new_value && force == 0 {
        return false;
    }
    *value = new_value;
    let k = derive_key();
    if *key != k {
        *key = k;
        on_key_change();
    }
    refresh();
    true
}

/// `0x0069e6b0`: the 19-entry table must be non-zero everywhere except index
/// 10 (which is skipped) for the flag to become true; `enable == 0` clears it.
pub fn w22a21_all_slots_filled(enable: u8, table: &[u8; 19]) -> bool {
    if enable == 0 {
        return false;
    }
    (0..19).all(|i| i == 10 || table[i] != 0)
}

/// `0x0069e760`: growth step of a flagged vector.  `cap_word` keeps the
/// capacity in its low 31 bits; returns the new word when a grow is needed
/// (new capacity = max(requested, cap + cap/2)), or `None`.
pub fn w22a21_grow_capacity(cap_word: u32, requested: u32) -> Option<u32> {
    if (cap_word & 0x7fff_ffff) >= requested {
        return None;
    }
    let mut new_cap = ((cap_word >> 1) & 0x3fff_ffff).wrapping_add(cap_word & 0x7fff_ffff);
    if new_cap < requested {
        new_cap = requested;
    }
    Some(((cap_word ^ new_cap) & 0x7fff_ffff) ^ cap_word)
}

/// Ring-buffer cursor of the resampler (offsets 0xa4/0xa8/0xac/0xb0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W22a21Ring {
    /// 0xa4: slot count.
    pub slots: i32,
    /// 0xa8: first buffered row.
    pub first_row: i32,
    /// 0xac: last buffered row.
    pub last_row: i32,
    /// 0xb0: slot of the first row, negative when empty.
    pub head_slot: i32,
}

/// `0x0069e840`: appends `row` as the last row and returns the slot index
/// whose `row_bytes` bytes the original then zeroes.
pub fn w22a21_ring_push_row(ring: &mut W22a21Ring, row: i32) -> i32 {
    ring.last_row = row;
    if ring.head_slot < 0 {
        ring.head_slot = 0;
        ring.first_row = row;
        0
    } else {
        ((ring.head_slot - ring.first_row) + row) % ring.slots
    }
}

/// `0x006a01a0`: drains ring rows below `target` through `process(row, slot)`
/// (only for rows inside `0..height`).  `buf_base` is returned when nothing
/// was drained, matching the original's return value.
pub fn w22a21_ring_drain(
    ring: &mut W22a21Ring,
    height: i32,
    target: i32,
    buf_base: i32,
    mut process: impl FnMut(i32, i32),
) -> i32 {
    let mut ret = buf_base;
    if ring.head_slot >= 0 && ring.first_row < target {
        let mut slot = ring.head_slot;
        loop {
            let row = ring.first_row;
            if (0..height).contains(&row) {
                process(row, slot);
            }
            ret = ring.first_row;
            if ret == ring.last_row {
                break;
            }
            ring.first_row = ret + 1;
            let next = ring.head_slot + 1;
            slot = next % ring.slots;
            ring.head_slot = slot;
            if target <= ring.first_row {
                return next / ring.slots;
            }
        }
        ring.head_slot = -1;
        ring.first_row = 0;
        ring.last_row = 0;
    }
    ret
}

/// Inputs of the buffer-size computation at `0x0069f340`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct W22a21Dims {
    pub off4: i32,
    pub off8: i32,
    pub off14: i32,
    pub off18: i32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub kind_x: i32,
    pub kind_y: i32,
    pub channels: i32,
}

/// Derived sizes written back by `0x0069f340`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W22a21Sizes {
    pub w98: i32,
    pub h9c: i32,
    pub ring_slots: i32,
    pub bc: i32,
    pub c0: i32,
    pub c4: i32,
    pub c8: i32,
    pub cc: i32,
    pub d0: i32,
    pub d4: i32,
    pub d8: i32,
    pub total: i32,
}

/// `0x0069f340`: computes the scratch buffer sizes; `support(kind, scale)`
/// and `taps(kind, scale)` stand for the two filter helpers.
pub fn w22a21_buffer_sizes(
    d: &W22a21Dims,
    support: impl Fn(i32, f32) -> i32,
    taps: impl Fn(i32, f32) -> i32,
) -> W22a21Sizes {
    let sx = support(d.kind_x, d.scale_x);
    let sy = support(d.kind_y, d.scale_y);
    let w98 = if d.scale_x <= 1.0 {
        d.off4 + (support(d.kind_x, d.scale_x) / 2) * 2
    } else {
        d.off14
    };
    let h9c = if d.scale_y <= 1.0 {
        d.off8 + (support(d.kind_y, d.scale_y) / 2) * 2
    } else {
        d.off18
    };
    let ring_slots = sy + 1;
    let bc = w98 * 8;
    let c0 = taps(d.kind_x, d.scale_x) * w98 * 4;
    let c4 = h9c << 3;
    let c8 = taps(d.kind_y, d.scale_y) * h9c * 4;
    let mut d0 = d.channels * d.off14 * 4;
    let cc = (d.off4 + (sx / 2) * 2) * d.channels * 4;
    let d4 = ring_slots * d.channels * d.off14 * 4;
    let mut d8 = d0;
    if d.scale_y <= 1.0 {
        d8 = 0;
    } else {
        d0 = 0;
    }
    let total = d4 + d8 + cc + c4 + c0 + c8 + bc;
    W22a21Sizes {
        w98,
        h9c,
        ring_slots,
        bc,
        c0,
        c4,
        c8,
        cc,
        d0,
        d4,
        d8,
        total,
    }
}

#[cfg(test)]
mod tests_wave22_agent21 {
    use super::*;

    #[test]
    fn flags() {
        let mut f = false;
        let mut n = 0;
        w22a21_set_flag_and_refresh(&mut f, || n += 1);
        assert!(f);
        w22a21_clear_flag_and_refresh(&mut f, || n += 1);
        assert!(!f && n == 2);
    }

    #[test]
    fn update_value() {
        let (mut v, mut k) = (5, 1);
        assert!(!w22a21_update_value(
            &mut v,
            &mut k,
            5,
            0,
            || 9,
            || {},
            || {}
        ));
        let log = std::cell::Cell::new(0);
        assert!(w22a21_update_value(
            &mut v,
            &mut k,
            5,
            1,
            || 9,
            || log.set(log.get() + 1),
            || log.set(log.get() + 10)
        ));
        assert_eq!((k, log.get()), (9, 11));
    }

    #[test]
    fn slots() {
        let mut t = [1u8; 19];
        t[10] = 0;
        assert!(w22a21_all_slots_filled(1, &t));
        assert!(!w22a21_all_slots_filled(0, &t));
        t[3] = 0;
        assert!(!w22a21_all_slots_filled(1, &t));
    }

    #[test]
    fn grow() {
        assert_eq!(w22a21_grow_capacity(10, 10), None);
        assert_eq!(w22a21_grow_capacity(10, 11), Some(15));
        assert_eq!(w22a21_grow_capacity(0x8000_0004, 100), Some(0x8000_0064));
    }

    #[test]
    fn ring() {
        let mut r = W22a21Ring {
            slots: 4,
            first_row: 0,
            last_row: 0,
            head_slot: -1,
        };
        assert_eq!(w22a21_ring_push_row(&mut r, 7), 0);
        assert_eq!(r.first_row, 7);
        assert_eq!(w22a21_ring_push_row(&mut r, 9), 2);
        let mut seen = vec![];
        let ret = w22a21_ring_drain(&mut r, 100, 8, 55, |a, b| seen.push((a, b)));
        assert_eq!(seen, vec![(7, 0)]);
        assert_eq!(ret, 0);
        assert_eq!(r.first_row, 8);
        let mut r2 = W22a21Ring {
            slots: 4,
            first_row: 0,
            last_row: 1,
            head_slot: 0,
        };
        assert_eq!(w22a21_ring_drain(&mut r2, 5, 10, 0, |_, _| {}), 1);
        assert_eq!(r2.head_slot, -1);
        assert_eq!(w22a21_ring_drain(&mut r2, 5, 10, 55, |_, _| {}), 55);
    }

    #[test]
    fn sizes() {
        let d = W22a21Dims {
            off4: 10,
            off8: 20,
            off14: 16,
            off18: 32,
            scale_x: 0.5,
            scale_y: 2.0,
            kind_x: 1,
            kind_y: 2,
            channels: 4,
        };
        let s = w22a21_buffer_sizes(&d, |_, _| 4, |_, _| 3);
        assert_eq!((s.w98, s.h9c, s.ring_slots), (14, 32, 5));
        assert_eq!((s.d0, s.d8), (0, 256));
        assert_eq!(s.cc, 14 * 4 * 4);
        assert_eq!(s.total, s.d4 + s.d8 + s.cc + s.c4 + s.c0 + s.c8 + s.bc);
    }
}
