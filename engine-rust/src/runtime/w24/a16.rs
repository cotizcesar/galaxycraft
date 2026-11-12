#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent16.rs
// ===========================================================================
// Wave 24 agent 16 translations (runtime glue).

/// `0x0074a410`: splits a horizontal strip into three drawn rects
/// `[left, top, right, bottom]`.  `cap` is the end-cap width (callee
/// `a89780`), `width` the strip width (`*(u16*)(param_1+0xc)`), `origin` the
/// `(x, y)` pair from `param_6`.  The closure receives `(index, rect)`.
pub fn w24a16_draw_three_part(
    cap: i32,
    width: u16,
    param_2: i32,
    index: i32,
    rows: i32,
    origin: (i16, i16),
    mut draw: impl FnMut(i32, [i32; 4]),
) {
    let left = origin.0 as i32;
    let top = param_2 + origin.1 as i32;
    let bottom = cap * rows + top;
    let mid_l = left + cap;
    draw(index, [left, top, mid_l, bottom]);
    let mid_r = mid_l + (width as i32 - cap);
    draw(index + 1, [mid_l, top, mid_r, bottom]);
    draw(index + 2, [mid_r, top, mid_r + cap, bottom]);
}

/// Record built by `0x0074cca0` (0x14 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W24a16Record {
    pub a: u32,
    pub b: u32,
    pub d: u32,
    pub c: u32,
    pub kind: u8,
}

/// `0x0074cca0`: fills a fresh record from two globals and two getters
/// (`c` comes from the first getter at +0xc, `d` from the second at +8);
/// byte at +0x12 is 5.
pub fn w24a16_make_record(g0: u32, g1: u32, getter_a: u32, getter_b: u32) -> W24a16Record {
    W24a16Record {
        a: g0,
        b: g1,
        c: getter_a,
        d: getter_b,
        kind: 5,
    }
}

/// `0x0074cae0`: if the flag is set, calls `release` and clears it.
pub fn w24a16_release_if_set(flag: &mut u32, release: impl FnOnce()) {
    if *flag != 0 {
        release();
        *flag = 0;
    }
}

/// `0x0074ce30`: forwards `arg` only when the global object exists.
pub fn w24a16_forward_if_present(present: bool, arg: u32, mut forward: impl FnMut(u32)) {
    if present {
        forward(arg);
    }
}

/// `0x0074c960`: tears down the global object.  `inner` is field +4 and
/// `flag` the byte at +0x160 of the object; `shutdown` runs when both are
/// non-zero, then the slot is cleared and `destroy(1)` is called.
pub fn w24a16_teardown(
    slot: &mut Option<(u32, u8)>,
    shutdown: impl FnOnce(),
    destroy: impl FnOnce(i32),
) {
    if let Some((inner, flag)) = *slot
        && inner != 0
        && flag != 0
    {
        shutdown();
    }
    if slot.take().is_some() {
        destroy(1);
    }
}

#[cfg(test)]
mod tests_wave24_agent16 {
    use super::*;

    #[test]
    fn three_part() {
        let mut v = Vec::new();
        w24a16_draw_three_part(4, 20, 10, 7, 2, (3, 5), |i, r| v.push((i, r)));
        assert_eq!(
            v,
            vec![
                (7, [3, 15, 7, 23]),
                (8, [7, 15, 23, 23]),
                (9, [23, 15, 27, 23])
            ]
        );
    }

    #[test]
    fn record() {
        let r = w24a16_make_record(1, 2, 3, 4);
        assert_eq!((r.a, r.b, r.c, r.d, r.kind), (1, 2, 3, 4, 5));
    }

    #[test]
    fn release_and_forward() {
        let mut f = 1;
        let mut n = 0;
        w24a16_release_if_set(&mut f, || n += 1);
        w24a16_release_if_set(&mut f, || n += 1);
        assert_eq!((f, n), (0, 1));
        let mut got = vec![];
        w24a16_forward_if_present(false, 1, |x| got.push(x));
        w24a16_forward_if_present(true, 2, |x| got.push(x));
        assert_eq!(got, vec![2]);
    }

    #[test]
    fn teardown() {
        let mut s = Some((1, 1));
        let (mut sd, mut d) = (0, 0);
        w24a16_teardown(&mut s, || sd += 1, |_| d += 1);
        assert_eq!((s, sd, d), (None, 1, 1));
        let mut s = Some((0, 1));
        w24a16_teardown(&mut s, || sd += 1, |_| d += 1);
        assert_eq!((sd, d), (1, 2));
        w24a16_teardown(&mut None, || sd += 1, |_| d += 1);
        assert_eq!((sd, d), (1, 2));
    }
}
