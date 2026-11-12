#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent48.rs
// ===========================================================================
// Wave 33 agent 48: CRT stream helpers over caller-supplied callees.

/// `0x00c9e50a`: re-initialises a buffer record: calls the reset helper,
/// then the allocator with `count * 2`.  On success (0) stores `count` at
/// +0xc and flag 1 at +0x14; otherwise zeroes +0xc and the flag.
pub fn w33a48_init_buffer(
    count: i32,
    reset: impl FnOnce(),
    alloc: impl FnOnce(i32) -> i32,
    size_field: &mut i32,
    flag: &mut u8,
) -> i32 {
    reset();
    let r = alloc(count.wrapping_mul(2));
    if r == 0 {
        *flag = 1;
        *size_field = count;
        0
    } else {
        *size_field = 0;
        *flag = 0;
        r
    }
}

/// `0x00c9f7b8`: reads units until one is accepted by the predicate
/// (returns 0 for accept); `u32::MAX` signals end of input.
pub fn w33a48_read_until_accepted(
    mut next: impl FnMut() -> u32,
    mut reject: impl FnMut(u32, i32) -> i32,
) -> u32 {
    loop {
        let v = next();
        if v == 0xffff_ffff {
            return 0xffff_ffff;
        }
        if reject(v & 0xff, 8) == 0 {
            return v;
        }
    }
}

/// `0x00c9fdbc`: dispatches on the stream mode: mode 1 runs `prep` first
/// then `mode1`, mode 2 uses `mode2`, anything else returns 0.
pub fn w33a48_dispatch(
    param: i32,
    prep: impl FnOnce(),
    mode: impl FnOnce() -> i32,
    mode1: impl FnOnce(i32) -> u32,
    mode2: impl FnOnce(i32) -> u32,
) -> u32 {
    if param == 1 {
        prep();
    }
    let m = mode();
    if m == 1 {
        mode1(param)
    } else if m == 2 {
        mode2(param)
    } else {
        0
    }
}

#[cfg(test)]
mod tests_wave33_agent48 {
    use super::*;

    #[test]
    fn init_ok_and_fail() {
        let (mut s, mut f) = (9, 9u8);
        assert_eq!(
            w33a48_init_buffer(
                4,
                || {},
                |n| {
                    assert_eq!(n, 8);
                    0
                },
                &mut s,
                &mut f
            ),
            0
        );
        assert_eq!((s, f), (4, 1));
        assert_eq!(w33a48_init_buffer(4, || {}, |_| 5, &mut s, &mut f), 5);
        assert_eq!((s, f), (0, 0));
    }

    #[test]
    fn read_loop() {
        let mut it = vec![0x141u32, 0x42].into_iter();
        let r = w33a48_read_until_accepted(|| it.next().unwrap(), |c, _| (c == 0x41) as i32);
        assert_eq!(r, 0x42);
        assert_eq!(w33a48_read_until_accepted(|| u32::MAX, |_, _| 0), u32::MAX);
    }

    #[test]
    fn dispatch_modes() {
        assert_eq!(
            w33a48_dispatch(1, || {}, || 1, |p| p as u32 + 10, |_| 0),
            11
        );
        assert_eq!(
            w33a48_dispatch(3, || {}, || 2, |_| 0, |p| p as u32 + 20),
            23
        );
        assert_eq!(w33a48_dispatch(3, || {}, || 7, |_| 1, |_| 2), 0);
    }
}
