#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent22.rs
// ===========================================================================
// Wave 31 agent 22 translations (queue, client.rs target).

/// `0x00688230`: reads a block through `read(&mut capacity, a, b, 1)` with
/// an initial capacity of `0x2000`.  Returns `(capacity, handle)` on
/// success (status 0), else `None` (the retail returns 0 and leaves the
/// out-handle untouched).
pub fn w31a22_read_block(
    handle: u32,
    a: u32,
    b: u32,
    read: impl FnOnce(u32, &mut u32, u32, u32, u32) -> i32,
) -> Option<(u32, u32)> {
    let mut cap = 0x2000u32;
    if read(handle, &mut cap, a, b, 1) != 0 {
        return None;
    }
    Some((cap, handle))
}

/// `0x007a35c0`: when neither guard flag is set, runs the first step and,
/// if the record at `record` has a non-zero u16 at offset 2, runs the
/// second step on the payload at offset 8.  Returns whether step one ran.
pub fn w31a22_update_record(
    flag_a: u8,
    flag_b: u32,
    record: Option<&[u8]>,
    step1: &mut dyn FnMut(),
    step2: &mut dyn FnMut(&[u8]),
) -> bool {
    if flag_a != 0 || flag_b != 0 {
        return false;
    }
    step1();
    if let Some(r) = record
        && r.len() >= 8
        && u16::from_le_bytes([r[2], r[3]]) != 0
    {
        step2(&r[8..]);
    }
    true
}

/// `0x00a89520`: scales a rect (x0,y0,x1,y1) by dividing x components by
/// `sx` and y components by `sy`.
pub fn w31a22_scale_rect(x0: f32, y0: f32, x1: f32, y1: f32, sx: f32, sy: f32) -> [f32; 4] {
    [x0 / sx, y0 / sy, x1 / sx, y1 / sy]
}

/// `0x00c9c4cb`: CRT dispatch.  When the mode global is non-zero calls
/// `alt`; otherwise valid (both non-zero) args call `main`; invalid args
/// yield `0x7fffffff` (EINVAL = 0x16 is set by the caller).
pub fn w31a22_crt_dispatch(
    mode: u32,
    p1: u32,
    p2: u32,
    alt: impl FnOnce() -> u32,
    main: impl FnOnce() -> u32,
) -> Result<u32, u32> {
    if mode != 0 {
        return Ok(alt());
    }
    if p1 != 0 && p2 != 0 {
        return Ok(main());
    }
    Err(0x16)
}

/// `0x00c9c5ae`: rounds `x` away from zero by 1.0 unless the classifier
/// (`class(x, 1)`) returns 1 or 2, or `class(x, 0)` returns 0.
pub fn w31a22_round_away(x: f32, class: impl Fn(f32, i32) -> i16) -> f32 {
    let c = class(x, 1);
    if !(1..=2).contains(&c) && class(x, 0) != 0 {
        if x.to_bits() & 0x8000_0000 == 0 {
            x + 1.0
        } else {
            x - 1.0
        }
    } else {
        x
    }
}

#[cfg(test)]
mod tests_wave31_agent22 {
    use super::*;

    #[test]
    fn read_block() {
        assert_eq!(
            w31a22_read_block(7, 1, 2, |_, c, _, _, _| {
                assert_eq!(*c, 0x2000);
                *c = 5;
                0
            }),
            Some((5, 7))
        );
        assert_eq!(w31a22_read_block(7, 1, 2, |_, _, _, _, _| 1), None);
    }

    #[test]
    fn update_record() {
        let rec = [0, 0, 1, 0, 0, 0, 0, 0, 9];
        let mut n = 0;
        let mut got = 0;
        assert!(w31a22_update_record(
            0,
            0,
            Some(&rec),
            &mut || n += 1,
            &mut |p| got = p[0]
        ));
        assert_eq!((n, got), (1, 9));
        assert!(!w31a22_update_record(
            1,
            0,
            Some(&rec),
            &mut || {},
            &mut |_| {}
        ));
    }

    #[test]
    fn scale() {
        assert_eq!(
            w31a22_scale_rect(2.0, 4.0, 6.0, 8.0, 2.0, 4.0),
            [1.0, 1.0, 3.0, 2.0]
        );
    }

    #[test]
    fn dispatch_and_round() {
        assert_eq!(w31a22_crt_dispatch(1, 0, 0, || 3, || 4), Ok(3));
        assert_eq!(w31a22_crt_dispatch(0, 1, 1, || 3, || 4), Ok(4));
        assert_eq!(w31a22_crt_dispatch(0, 0, 1, || 3, || 4), Err(0x16));
        assert_eq!(
            w31a22_round_away(1.5, |_, m| if m == 1 { 0 } else { 3 }),
            2.5
        );
        assert_eq!(
            w31a22_round_away(-1.5, |_, m| if m == 1 { 0 } else { 3 }),
            -2.5
        );
        assert_eq!(
            w31a22_round_away(1.5, |_, m| if m == 1 { 1 } else { 3 }),
            1.5
        );
    }
}
