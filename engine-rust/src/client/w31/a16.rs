#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent16.rs
// ===========================================================================
// Wave 31 agent 16: small UI input handlers (mouse/event glue).

/// `0x008c68a0`: runs `action` and returns true unless `blocked` (the result
/// of the 9651f0 check) is set, in which case it returns false.
pub fn w31a16_try_action(blocked: bool, action: impl FnOnce()) -> bool {
    if blocked {
        return false;
    }
    action();
    true
}

/// `0x008c68e0`: invokes the callee and always reports handled (true).
pub fn w31a16_always_handled(action: impl FnOnce()) -> bool {
    action();
    true
}

/// Local-space mouse position helper shared by 0x008c6920 and 0x008c69a0:
/// `(p - origin) - inner` per axis.
pub fn w31a16_local_pos(p: (i32, i32), origin: (i32, i32), inner: (i32, i32)) -> (i32, i32) {
    (
        p.0.wrapping_sub(origin.0).wrapping_sub(inner.0),
        p.1.wrapping_sub(origin.1).wrapping_sub(inner.1),
    )
}

/// `0x008c6920`: when `hit`, forwards the local position to `sink`; always
/// returns 1.
pub fn w31a16_mouse_move(
    hit: bool,
    p: (i32, i32),
    origin: (i32, i32),
    inner: (i32, i32),
    sink: impl FnOnce(i32, i32),
) -> u32 {
    if hit {
        let (x, y) = w31a16_local_pos(p, origin, inner);
        sink(x, y);
    }
    1
}

/// `0x008c69a0`: like `w31a16_mouse_move` but also forwards `param * 100`.
pub fn w31a16_mouse_scroll(
    hit: bool,
    p: (i32, i32),
    origin: (i32, i32),
    inner: (i32, i32),
    param: i32,
    sink: impl FnOnce(i32, i32, i32),
) -> u32 {
    if hit {
        let (x, y) = w31a16_local_pos(p, origin, inner);
        sink(x, y, param.wrapping_mul(100));
    }
    1
}

/// `0x008c6c30`: when `ready`, runs `update`; otherwise sets the flag byte
/// at object offset `0x1e1`.
pub fn w31a16_update_or_flag(ready: bool, flag_1e1: &mut u8, update: impl FnOnce()) {
    if ready {
        update();
    } else {
        *flag_1e1 = 1;
    }
}

/// `0x008db680`: takes the owned object at `+0xf48` (dropping it) and
/// resets the id at `+0xf4c` to all-ones after releasing `old_id`
/// (low 16 bits). Returns the released id.
pub fn w31a16_release_slot<T>(obj: &mut Option<T>, id: &mut u32) -> u16 {
    drop(obj.take());
    let released = *id as u16;
    *id = 0xffff_ffff;
    released
}

/// `0x0090c6c0`: calls the two handlers in order.
pub fn w31a16_call_both(first: impl FnOnce(), second: impl FnOnce()) {
    first();
    second();
}

/// `0x0090c850`: if the suppress flag at `+0x1c0` is set, clears it and
/// returns 0 (the secondary handler is skipped); otherwise runs both
/// handlers and returns the second result.
pub fn w31a16_suppressible_handler(
    flag_1c0: &mut u8,
    first: impl FnOnce(),
    second: impl FnOnce() -> u32,
) -> u32 {
    if *flag_1c0 != 0 {
        *flag_1c0 = 0;
        return 0;
    }
    first();
    second()
}

#[cfg(test)]
mod tests_wave31_agent16 {
    use super::*;

    #[test]
    fn actions() {
        let mut n = 0;
        assert!(!w31a16_try_action(true, || n += 1));
        assert_eq!(n, 0);
        assert!(w31a16_try_action(false, || n += 1));
        assert!(w31a16_always_handled(|| n += 1));
        assert_eq!(n, 2);
    }

    #[test]
    fn mouse() {
        let mut got = None;
        assert_eq!(
            w31a16_mouse_move(true, (10, 20), (1, 2), (3, 4), |x, y| got = Some((x, y))),
            1
        );
        assert_eq!(got, Some((6, 14)));
        let mut got3 = None;
        w31a16_mouse_scroll(true, (10, 20), (1, 2), (3, 4), 3, |x, y, z| {
            got3 = Some((x, y, z))
        });
        assert_eq!(got3, Some((6, 14, 300)));
        assert_eq!(
            w31a16_mouse_move(false, (0, 0), (0, 0), (0, 0), |_, _| panic!()),
            1
        );
    }

    #[test]
    fn flags_and_slots() {
        let mut f = 0u8;
        w31a16_update_or_flag(false, &mut f, || panic!());
        assert_eq!(f, 1);
        w31a16_update_or_flag(true, &mut f, || {});
        let mut o = Some(5);
        let mut id = 0x1_0007;
        assert_eq!(w31a16_release_slot(&mut o, &mut id), 7);
        assert!(o.is_none());
        assert_eq!(id, 0xffff_ffff);
        let mut s = 1u8;
        assert_eq!(w31a16_suppressible_handler(&mut s, || panic!(), || 9), 0);
        assert_eq!(s, 0);
        assert_eq!(w31a16_suppressible_handler(&mut s, || {}, || 9), 9);
        let mut order = vec![];
        let o2 = std::cell::RefCell::new(&mut order);
        w31a16_call_both(|| o2.borrow_mut().push(1), || o2.borrow_mut().push(2));
        assert_eq!(order, vec![1, 2]);
    }
}
