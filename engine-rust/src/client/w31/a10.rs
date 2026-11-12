#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent10.rs
// ===========================================================================
// Wave 31 agent 10 translations (client glue helpers).

/// `0x007cbbb0`: forwards three args to the base initialiser (with a
/// trailing zero) and then sets the byte flag at `this+0x8c`.
pub fn w31a10_init_with_flag(
    flag_byte: &mut u8,
    args: (u32, u32, u32),
    base: impl FnOnce(u32, u32, u32, u32),
) {
    base(args.0, args.1, args.2, 0);
    *flag_byte = 1;
}

/// `0x007e7460`: calls the base setter with `(param, 2)` and sets the byte
/// flag at `this+0x30`.
pub fn w31a10_set_with_mode2(flag_byte: &mut u8, param: u32, base: impl FnOnce(u32, u32)) {
    base(param, 2);
    *flag_byte = 1;
}

/// `0x007f26c0`: when the predicate is false, runs the registration call,
/// marks byte `this+0x238` and stamps `this+0x248` with the running
/// counter, which is then incremented.
pub fn w31a10_register_once(
    already: bool,
    marked: &mut u8,
    stamp: &mut i32,
    counter: &mut i32,
    register: impl FnOnce(),
) {
    if !already {
        register();
        *marked = 1;
        *stamp = *counter;
        *counter = counter.wrapping_add(1);
    }
}

/// `0x007d6e60`: when `*flag != 0` sets state (`+0x24`) to 2, releases the
/// buffer handle at `+0x20` (two release calls, then zeroes it) and runs the
/// follow-up; otherwise runs the alternate path.  Returns true when the
/// flagged branch was taken.
pub fn w31a10_toggle_state(
    flag: u8,
    state: &mut u32,
    handle: &mut u32,
    release_prep: impl FnOnce(),
    release: impl FnOnce(u32),
    on_set: impl FnOnce(),
    on_clear: impl FnOnce(),
) -> bool {
    if flag == 0 {
        on_clear();
        return false;
    }
    *state = 2;
    if *handle != 0 {
        release_prep();
        release(*handle);
        *handle = 0;
    }
    on_set();
    true
}

#[cfg(test)]
mod tests_wave31_agent10 {
    use super::*;

    #[test]
    fn init_and_set() {
        let mut f = 0u8;
        let mut seen = (0, 0, 0, 9);
        w31a10_init_with_flag(&mut f, (1, 2, 3), |a, b, c, d| seen = (a, b, c, d));
        assert_eq!((f, seen), (1, (1, 2, 3, 0)));
        let mut g = 0u8;
        let mut m = (0, 0);
        w31a10_set_with_mode2(&mut g, 7, |a, b| m = (a, b));
        assert_eq!((g, m), (1, (7, 2)));
    }

    #[test]
    fn register() {
        let (mut m, mut s, mut c, mut n) = (0u8, 0, 5, 0);
        w31a10_register_once(true, &mut m, &mut s, &mut c, || n += 1);
        assert_eq!((m, s, c, n), (0, 0, 5, 0));
        w31a10_register_once(false, &mut m, &mut s, &mut c, || n += 1);
        assert_eq!((m, s, c, n), (1, 5, 6, 1));
    }

    #[test]
    fn toggle() {
        let (mut st, mut h) = (0, 4);
        let cell = std::cell::Cell::new(0);
        let r = w31a10_toggle_state(
            1,
            &mut st,
            &mut h,
            || cell.set(cell.get() + 1),
            |x| cell.set(cell.get() + x),
            || cell.set(cell.get() + 100),
            || cell.set(0),
        );
        assert!(r);
        assert_eq!((st, h, cell.get()), (2, 0, 105));
        let r = w31a10_toggle_state(0, &mut st, &mut h, || {}, |_| {}, || {}, || cell.set(0));
        assert!(!r);
        assert_eq!(cell.get(), 0);
    }
}
