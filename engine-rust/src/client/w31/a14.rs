#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent14.rs
// ===========================================================================
// Wave 31 translation batch 14.

/// Event code (field at offset 0xe of the message) that yields a true flag.
pub const W31A14_EVENT_FLAG_CODE: i16 = 10;

fn w31a14_flag(event_code: i16) -> bool {
    event_code == W31A14_EVENT_FLAG_CODE
}

/// `0x008ae6c0`: forwards `event_code == 10` to the sink.
pub fn w31a14_ae6c0_forward_flag(event_code: i16, mut sink: impl FnMut(bool)) {
    sink(w31a14_flag(event_code));
}

/// `0x008ae990`: teardown sequence.  `handle` is the field at index 0xfa
/// (-1 = none); `release` is called with it when valid; `is_overridden`
/// is the virtual at +0xa0; `owned` is the field 0xfb (taken and dropped).
pub fn w31a14_ae990_teardown<T>(
    handle: i32,
    mut release: impl FnMut(i32),
    is_overridden: bool,
    mut default_cleanup: impl FnMut(),
    owned: &mut Option<T>,
    mut tail: impl FnMut(),
) {
    if handle != -1 {
        release(handle);
    }
    if !is_overridden {
        default_cleanup();
    }
    drop(owned.take());
    tail();
}

/// `0x008a78c0`: returns the refresh step list.  Each node yields
/// `(a, b, word)`; `resolve` maps `(a, b)` to a handle, `push` receives
/// `(handle, word)`.  Returns true when `count > 0` (draw path), false
/// otherwise (clear path).
pub fn w31a14_a78c0_collect<N: Copy>(
    nodes: &[(N, N, u16)],
    mut resolve: impl FnMut(N, N) -> u32,
    mut push: impl FnMut(u32, [u32; 2]),
    count: i32,
) -> bool {
    for &(a, b, w) in nodes {
        let h = resolve(a, b);
        push(h, [w as u32, 0]);
    }
    count > 0
}

/// `0x008ae690`: flag computed, then action, then sink; flag is `event_code == 10`.
pub fn w31a14_8ae690_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    let flag = w31a14_flag(event_code);
    action();
    sink(flag);
}

/// `0x008ae6e0`: action, then sink; flag is `event_code == 10`.
pub fn w31a14_8ae6e0_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    action();
    sink(w31a14_flag(event_code));
}

/// `0x008ae710`: action, then sink; flag is `event_code == 10`.
pub fn w31a14_8ae710_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    action();
    sink(w31a14_flag(event_code));
}

/// `0x008ae740`: action, then sink; flag is `event_code == 10`.
pub fn w31a14_8ae740_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    action();
    sink(w31a14_flag(event_code));
}

/// `0x008aede0`: flag computed, then action, then sink; flag is `event_code == 10`.
pub fn w31a14_8aede0_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    let flag = w31a14_flag(event_code);
    action();
    sink(flag);
}

/// `0x008af100`: flag computed, then action, then sink; flag is `event_code == 10`.
pub fn w31a14_8af100_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    let flag = w31a14_flag(event_code);
    action();
    sink(flag);
}

/// `0x008af160`: action, then sink; flag is `event_code == 10`.
pub fn w31a14_8af160_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    action();
    sink(w31a14_flag(event_code));
}

/// `0x008af1d0`: action, then sink; flag is `event_code == 10`.
pub fn w31a14_8af1d0_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    action();
    sink(w31a14_flag(event_code));
}

/// `0x008af6c0`: flag computed, then action, then sink; flag is `event_code == 10`.
pub fn w31a14_8af6c0_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    let flag = w31a14_flag(event_code);
    action();
    sink(flag);
}

/// `0x008af6f0`: flag computed, then action, then sink; flag is `event_code == 10`.
pub fn w31a14_8af6f0_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    let flag = w31a14_flag(event_code);
    action();
    sink(flag);
}

/// `0x008af7a0`: action, then sink; flag is `event_code == 10`.
pub fn w31a14_8af7a0_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    action();
    sink(w31a14_flag(event_code));
}

/// `0x008afa60`: action, then sink; flag is `event_code == 10`.
pub fn w31a14_8afa60_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    action();
    sink(w31a14_flag(event_code));
}

/// `0x008afac0`: flag computed, then action, then sink; flag is `event_code == 10`.
pub fn w31a14_8afac0_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    let flag = w31a14_flag(event_code);
    action();
    sink(flag);
}

/// `0x008b0510`: flag computed, then action, then sink; flag is `event_code == 10`.
pub fn w31a14_8b0510_dispatch(
    event_code: i16,
    mut action: impl FnMut(),
    mut sink: impl FnMut(bool),
) {
    let flag = w31a14_flag(event_code);
    action();
    sink(flag);
}

#[cfg(test)]
mod tests_wave31_agent14 {
    use super::*;

    #[test]
    fn flag_dispatch() {
        let log = std::cell::RefCell::new(Vec::new());
        w31a14_8ae690_dispatch(
            10,
            || log.borrow_mut().push("a"),
            |f| log.borrow_mut().push(if f { "t" } else { "f" }),
        );
        assert_eq!(*log.borrow(), ["a", "t"]);
        let mut got = None;
        w31a14_8b0510_dispatch(9, || {}, |f| got = Some(f));
        assert_eq!(got, Some(false));
        w31a14_ae6c0_forward_flag(10, |f| assert!(f));
    }

    #[test]
    fn teardown_and_collect() {
        let mut o = Some(1);
        let mut rel = vec![];
        let mut flag = false;
        w31a14_ae990_teardown(-1, |h| rel.push(h), false, || flag = true, &mut o, || {});
        assert!(flag && rel.is_empty());
        assert!(o.is_none());
        let mut pushed = vec![];
        let r = w31a14_a78c0_collect(
            &[(1u8, 2u8, 7)],
            |a, b| (a + b) as u32,
            |h, w| pushed.push((h, w)),
            1,
        );
        assert!(r);
        assert_eq!(pushed, [(3, [7, 0])]);
    }
}
