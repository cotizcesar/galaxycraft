#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent14.rs
// ===========================================================================
// Wave 27 agent 14: small object-state helpers (byte-buffer object model).

/// `0x00995d10`: reads field `+0xac` of the sub-object at `+0x1c` of the
/// object referenced from slot `+0x4c`.  Returns `(status, value)`:
/// `0x25` for a missing out parameter, `0x24` (value 0) when the slot is
/// empty, otherwise `0` with the value (0 when the sub-object is absent).
pub fn w27a14_get_sub_field(
    want_out: bool,
    slot_present: bool,
    sub_field_ac: Option<u32>,
) -> (u32, u32) {
    if !want_out {
        return (0x25, 0);
    }
    if !slot_present {
        return (0x24, 0);
    }
    (0, sub_field_ac.unwrap_or(0))
}

/// `0x0099ee50`: returns `0x25` when `handle` is zero, otherwise invokes the
/// external release routine and returns 0.
pub fn w27a14_call_if_nonzero_a(handle: u32, mut f: impl FnMut(u32)) -> u32 {
    if handle == 0 {
        return 0x25;
    }
    f(handle);
    0
}

/// `0x0099eeb0`: same shape as the sibling routine, with a different
/// external callee; `0x25` for a zero handle, else call and return 0.
pub fn w27a14_call_if_nonzero_b(handle: u32, mut f: impl FnMut(u32)) -> u32 {
    if handle == 0 {
        return 0x25;
    }
    f(handle);
    0
}

/// `0x009a0e40`: resets an object in a byte buffer (`obj` must be at least
/// `0x1c4` bytes): stores `param_3` at `+0x164`, `param_1` at `+0x190`,
/// `param_2` at `+0x14`/`+0x18`/`+0x1c`, clears the listed dword fields,
/// the byte at `+0x2c` and the `0x120` bytes at `+0x2d`.
pub fn w27a14_reset_object(obj: &mut [u8], param_1: u32, param_2: u32, param_3: u32) {
    let mut put = |off: usize, v: u32| obj[off..off + 4].copy_from_slice(&v.to_le_bytes());
    put(0x164, param_3);
    put(0x190, param_1);
    put(0x14, 0);
    put(0x18, param_2);
    put(0x14, param_2);
    put(0x1c, param_2);
    for off in [
        0x184, 0x24, 0x20, 0x178, 0x1bc, 0x1c0, 0x15c, 0x16c, 0x170, 0x174, 0x168, 0x158, 0x160,
        0x1b8, 0x180, 0x150, 0x154, 0x188, 0x17c,
    ] {
        put(off, 0);
    }
    obj[0x2c] = 0;
    obj[0x2d..0x2d + 0x120].fill(0);
}

/// `0x00998310`: validity check before the (non-returning) clear.  Returns
/// true only when the `+0x4c` slot exists, `param_2` is non-zero,
/// `param_3 <= max_56c`, `0 <= index < count_570`, and `index` is allowed
/// by the mode (`+0xe8`): 1 -> {0}; 2 -> {0,1}; 3 -> {0,1,4,5};
/// 4 -> {0,1,2,4,5}; 5/7/8 -> {0..=5}; 6/9 -> index <= 7; other modes
/// accept any index.
pub fn w27a14_slot_index_valid(
    slot_present: bool,
    param_2: i32,
    param_3: i32,
    max_56c: i32,
    count_570: i32,
    index: i32,
    mode: i32,
) -> bool {
    if !slot_present || param_2 == 0 || max_56c < param_3 {
        return false;
    }
    if !(0 <= index && index < count_570) {
        return false;
    }
    match mode {
        1 => index == 0,
        2 => matches!(index, 0 | 1),
        3 => matches!(index, 0 | 1 | 4 | 5),
        4 => matches!(index, 0 | 1 | 2 | 4 | 5),
        5 | 7 | 8 => (0..=5).contains(&index),
        6 | 9 => index <= 7,
        _ => true,
    }
}

/// `0x009957a0`: adjusts an optional float out value.  When `is_primary`
/// (`param_1` equals the base pointer plus `0x5eb4`) the value is 1.0;
/// otherwise it comes from `probe`, and a value <= 0 is left unchanged.
/// Otherwise the value is scaled by `1.0 - blend()`.
pub fn w27a14_scale_factor(
    out: Option<&mut f32>,
    is_primary: bool,
    probe: impl FnOnce() -> f32,
    blend: impl FnOnce() -> f32,
) {
    let Some(out) = out else { return };
    if is_primary {
        *out = 1.0;
    } else {
        *out = probe();
        if *out <= 0.0 {
            return;
        }
    }
    *out *= 1.0 - blend();
}

/// `0x0099da90`: indexed query through an optional callback (`+0xb4`).
/// `callback(index)` returns `(status, value, 32-byte buffer)`.  Requires
/// `0 <= index < count`; on status 0 stores `value` and copies
/// `min(len, 0x10)` bytes of the buffer into `buf`.  Returns true when the
/// callback succeeded.
#[allow(clippy::type_complexity)]
pub fn w27a14_indexed_query(
    callback: Option<&mut dyn FnMut(i32) -> (i32, u32, [u8; 32])>,
    count: i32,
    index: i32,
    out_value: Option<&mut u32>,
    buf: Option<&mut [u8]>,
    len: i32,
) -> bool {
    let Some(cb) = callback else { return false };
    if !(0 <= index && index < count) {
        return false;
    }
    let (status, value, data) = cb(index);
    if status != 0 {
        return false;
    }
    if let Some(v) = out_value {
        *v = value;
    }
    if let Some(b) = buf {
        let n = len.clamp(0, 0x10) as usize;
        let n = n.min(b.len());
        b[..n].copy_from_slice(&data[..n]);
    }
    true
}

#[cfg(test)]
mod tests_wave27_agent14 {
    use super::*;

    #[test]
    fn sub_field() {
        assert_eq!(w27a14_get_sub_field(false, true, Some(5)), (0x25, 0));
        assert_eq!(w27a14_get_sub_field(true, false, Some(5)), (0x24, 0));
        assert_eq!(w27a14_get_sub_field(true, true, Some(5)), (0, 5));
        assert_eq!(w27a14_get_sub_field(true, true, None), (0, 0));
    }

    #[test]
    fn nonzero_calls() {
        let mut n = 0;
        assert_eq!(w27a14_call_if_nonzero_a(0, |_| n += 1), 0x25);
        assert_eq!(w27a14_call_if_nonzero_a(7, |h| n += h), 0);
        assert_eq!(w27a14_call_if_nonzero_b(0, |_| n += 100), 0x25);
        assert_eq!(w27a14_call_if_nonzero_b(1, |h| n += h), 0);
        assert_eq!(n, 8);
    }

    #[test]
    fn reset() {
        let mut o = vec![0xffu8; 0x1c4];
        w27a14_reset_object(&mut o, 1, 2, 3);
        let g = |off: usize| u32::from_le_bytes(o[off..off + 4].try_into().unwrap());
        assert_eq!(
            (g(0x164), g(0x190), g(0x14), g(0x18), g(0x1c)),
            (3, 1, 2, 2, 2)
        );
        assert_eq!(g(0x178), 0);
        assert_eq!(o[0x2c], 0);
        assert!(o[0x2d..0x14d].iter().all(|&b| b == 0));
        assert_eq!(o[0x14d], 0xff);
    }

    #[test]
    fn index_valid() {
        let v = |i, m| w27a14_slot_index_valid(true, 1, 1, 4, 10, i, m);
        assert!(v(0, 1) && !v(1, 1));
        assert!(v(1, 2) && !v(2, 2));
        assert!(v(5, 3) && !v(2, 3));
        assert!(v(2, 4) && !v(3, 4));
        assert!(v(3, 5) && !v(6, 5));
        assert!(v(7, 6) && !v(8, 6));
        assert!(v(9, 0));
        assert!(!v(10, 0) && !v(-1, 0));
        assert!(!w27a14_slot_index_valid(false, 1, 1, 4, 10, 0, 0));
        assert!(!w27a14_slot_index_valid(true, 0, 1, 4, 10, 0, 0));
        assert!(!w27a14_slot_index_valid(true, 1, 5, 4, 10, 0, 0));
    }

    #[test]
    fn scale() {
        let mut f = 9.0;
        w27a14_scale_factor(Some(&mut f), true, || 0.0, || 0.25);
        assert_eq!(f, 0.75);
        w27a14_scale_factor(Some(&mut f), false, || 0.0, || 0.25);
        assert_eq!(f, 0.0);
        w27a14_scale_factor(Some(&mut f), false, || 2.0, || 0.5);
        assert_eq!(f, 1.0);
        w27a14_scale_factor(None, true, || 0.0, || 0.0);
    }

    #[test]
    fn query() {
        let mut cb = |i: i32| (if i == 3 { 1 } else { 0 }, 42u32, [7u8; 32]);
        let mut v = 0;
        let mut b = [0u8; 20];
        assert!(w27a14_indexed_query(
            Some(&mut cb),
            5,
            1,
            Some(&mut v),
            Some(&mut b),
            20
        ));
        assert_eq!(v, 42);
        assert_eq!(b.iter().filter(|&&x| x == 7).count(), 16);
        assert!(!w27a14_indexed_query(Some(&mut cb), 5, 3, None, None, 0));
        assert!(!w27a14_indexed_query(Some(&mut cb), 5, 5, None, None, 0));
        assert!(!w27a14_indexed_query(None, 5, 0, None, None, 0));
    }
}
