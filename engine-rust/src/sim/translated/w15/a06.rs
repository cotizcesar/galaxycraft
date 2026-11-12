#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave15/agent06.rs
// ===========================================================================
// Wave 15 translation batch 06.
//
// Small pure helpers over a raw object record (`this` = `&[u8]`/`&mut [u8]`)
// plus closure-driven control flow for the routines whose callees are not
// translated yet.

fn w15a06_rd_u16(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}

fn w15a06_rd_u32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// `0x005af990`: init sequence for `n`: runs the three sub-steps in order,
/// the last one (arg `n * 2`) twice.  Callees are caller-supplied.
pub fn w15a06_init_sequence(
    n: i32,
    mut step_a: impl FnMut(i32),
    mut step_b: impl FnMut(i32),
    mut step_c: impl FnMut(i32),
) {
    step_a(n);
    step_b(n);
    step_c(n.wrapping_mul(2));
    step_c(n.wrapping_mul(2));
}

/// `0x005b2cb0`: applies `apply(param)` to the record, then walks the chain
/// through the pointer at `+0x70` (`next`), re-applying while `stop(cur)` is
/// false and the link is non-null.  `next(cur)` yields the link at `+0x70`
/// (0 = none).  Returns the number of applications.
pub fn w15a06_apply_chain(
    start: u32,
    param: u32,
    mut apply: impl FnMut(u32, u32),
    next: impl Fn(u32) -> u32,
    stop: impl Fn() -> bool,
) -> u32 {
    let mut n = 1;
    apply(start, param);
    let mut cur = start;
    let mut link = next(cur);
    while link != 0 {
        if stop() {
            break;
        }
        cur = link;
        apply(cur, param);
        n += 1;
        link = next(cur);
    }
    n
}

/// `0x005b67e0`: stores the 16-bit value at offset `+0xd0` of the record.
pub fn w15a06_set_word_d0(rec: &mut [u8], value: u16) {
    rec[0xd0..0xd2].copy_from_slice(&value.to_le_bytes());
}

/// `0x005b6990`: forwards the two 16-bit fields at `+0x58` and `+0x5a` of
/// the record to `f`.
pub fn w15a06_forward_xy<R>(rec: &[u8], f: impl FnOnce(u16, u16) -> R) -> R {
    f(w15a06_rd_u16(rec, 0x58), w15a06_rd_u16(rec, 0x5a))
}

/// `0x005b69a0`: forwards the 32-bit field at `+0x5c` of the record to `f`.
pub fn w15a06_forward_dword_5c<R>(rec: &[u8], f: impl FnOnce(u32) -> R) -> R {
    f(w15a06_rd_u32(rec, 0x5c))
}

/// `0x005b6e60`: requirement check for kind `kind` (`'#'` or `'P'`), target
/// id `id`.  Returns `(ok, error_message_id)`; unknown kinds pass.
/// `race()` is the low byte of the race query, `special(id)` the id test.
pub fn w15a06_requirement_check(
    kind: u8,
    id: u16,
    race: impl Fn() -> u8,
    special: impl Fn(u16) -> bool,
) -> (bool, Option<u16>) {
    if kind == b'#' {
        if id.wrapping_sub(0x6a) > 0x15 {
            return (false, Some(0x374));
        }
        (true, None)
    } else if kind == b'P' {
        let mut msg = (race() as u16).wrapping_add(0x37d);
        if id != 0xbc {
            if special(id) {
                return (true, None);
            }
            msg = 0x37c;
            match race() {
                0 if id == 0x95 => return (true, None),
                1 if id == 0x6e => return (true, None),
                2 if id == 0x9d => return (true, None),
                _ => {}
            }
        }
        (false, Some(msg))
    } else {
        (true, None)
    }
}

/// `0x005bd3b0`: sets bit 0 of the byte at `+0x18` of the linked record
/// (given), if any.
pub fn w15a06_set_link_flag(link: Option<&mut [u8]>) {
    if let Some(l) = link {
        l[0x18] |= 1;
    }
}

/// Query/side-effect hooks used by `w15a06_choose_state`.
pub trait W15a06StateProbe {
    /// `FUN_00588d70` result.
    fn small_value(&mut self) -> u8;
    /// `FUN_005b6370`.
    fn q_6370(&mut self) -> bool;
    /// `FUN_005a3150`.
    fn q_3150(&mut self) -> bool;
    /// `FUN_005b5680`.
    fn q_5680(&mut self) -> bool;
    /// `FUN_005b5d40`.
    fn q_5d40(&mut self) -> bool;
    /// `FUN_005b5840`.
    fn q_5840(&mut self) -> bool;
    /// `FUN_005b3a90`.
    fn q_3a90(&mut self) -> bool;
    /// `func_0x5b_3a70`.
    fn q_3a70(&mut self) -> bool;
    /// `FUN_005bb740`.
    fn q_b740(&mut self) -> bool;
    /// Side effects `FUN_005bd3c0`, `FUN_00569540`, `FUN_005bb7c0` in order.
    fn finish_stop(&mut self);
}

/// `0x005bce90`: picks the next state byte (`+0x97`) for the record.
/// `flags_dc` is the dword at `+0xdc`, `flags_20` the byte at `+0x20`,
/// `b105` the byte at `+0x105` (updated in place), `d108`/`d10c` (`+0x108`,
/// `+0x10c`) are cleared.  Returns the new state (the function always
/// returns 1 after storing it).
pub fn w15a06_choose_state(
    flags_dc: u32,
    flags_20: u8,
    b105: &mut u8,
    d108_10c: &mut [u32; 2],
    p: &mut impl W15a06StateProbe,
) -> u8 {
    *d108_10c = [0, 0];
    let v = p.small_value();
    *b105 = ((v < 0xc) as u8) | (*b105 & 0xfc);
    if !p.q_6370() && flags_dc & 0x2000 != 0 {
        return 1;
    }
    if p.q_3150() {
        return 2;
    }
    if p.q_5680() {
        return 4;
    }
    if p.q_5d40() {
        if flags_20 & 2 == 0 && p.q_b740() {
            return 6;
        }
        p.finish_stop();
        return 6;
    }
    if p.q_5840() {
        return 2;
    }
    if !p.q_3a90() && !p.q_3a70() {
        if *b105 & 1 != 0 {
            if flags_20 & 2 == 0 && p.q_b740() {
                return 2;
            }
            return 0xe;
        }
        return 2;
    }
    if !p.q_3a90() {
        return if p.q_6370() { 3 } else { 5 };
    }
    if *b105 & 1 != 0 { 9 } else { 7 }
}

/// Shared rectangle test of `0x005c04d0` and `0x005c0670`: with the
/// position `(x, y)` (signed 16-bit), extents `left`, `top`, `right`,
/// `bottom` and map size `(w, h)`, true when the box is inside the map.
pub fn w15a06_box_inside(pos: (i16, i16), ext: (i16, i16, i16, i16), map: (u16, u16)) -> bool {
    let (x, y) = (pos.0 as i32, pos.1 as i32);
    let (left, top, right, bottom) = (ext.0 as i32, ext.1 as i32, ext.2 as i32, ext.3 as i32);
    x - left >= 0 && y - top >= 0 && right + x < map.0 as i32 && bottom + y < map.1 as i32
}

/// `0x005c04d0`: `None` when the first probe (`FUN_0076c5a0`) is non-zero
/// returns `Some(1)`; when the box is inside the map the finisher runs and
/// `Some(0)` is returned; otherwise the original does not return (`None`).
pub fn w15a06_check_box_a(probe_ok: bool, inside: bool, finish: impl FnOnce()) -> Option<u32> {
    if !probe_ok {
        return Some(1);
    }
    if inside {
        finish();
        return Some(0);
    }
    None
}

/// `0x005c0670`: `None` when the probe `FUN_005bb790` is non-zero (the
/// original does not return); otherwise `Some(0)` when the box is inside
/// the map and `Some(1)` when not.
pub fn w15a06_check_box_b(probe_blocked: bool, inside: bool) -> Option<u32> {
    if probe_blocked {
        return None;
    }
    Some(if inside { 0 } else { 1 })
}

// Skipped (reason in brackets):
// FUN_005a4ae0 [noreturn thunk]
// FUN_005b2262 [noreturn tail of an unmodelled frame]
// FUN_005b2289 [noreturn tail of an unmodelled frame]
// FUN_005b5400 [dozens of unmodelled callees, noreturn tail]
// FUN_005b6f10 [switch on unmodelled callees with noreturn arms]
// FUN_005ba190 [SEH, string formatting and unmodelled dispatch]
// FUN_005bae20 [recursive and unmodelled callees]
// FUN_005bba50 [36-state machine over many unmodelled callees]
// FUN_005bcff0 [unrecovered in-register stack args]
// FUN_005c1940 [globals tables and noreturn tail]

#[cfg(test)]
mod tests_wave15_agent06 {
    use super::*;

    #[test]
    fn init_sequence_order() {
        let mut log = Vec::new();
        let l = std::cell::RefCell::new(&mut log);
        w15a06_init_sequence(
            3,
            |a| l.borrow_mut().push(('a', a)),
            |a| l.borrow_mut().push(('b', a)),
            |a| l.borrow_mut().push(('c', a)),
        );
        assert_eq!(log, vec![('a', 3), ('b', 3), ('c', 6), ('c', 6)]);
    }

    #[test]
    fn chain_walk() {
        let mut seen = vec![];
        let n = w15a06_apply_chain(
            1,
            9,
            |c, p| seen.push((c, p)),
            |c| if c < 3 { c + 1 } else { 0 },
            || false,
        );
        assert_eq!(n, 3);
        assert_eq!(seen, vec![(1, 9), (2, 9), (3, 9)]);
        let n = w15a06_apply_chain(1, 0, |_, _| {}, |_| 2, || true);
        assert_eq!(n, 1);
    }

    #[test]
    fn field_helpers() {
        let mut r = vec![0u8; 0x110];
        w15a06_set_word_d0(&mut r, 0x1234);
        assert_eq!(&r[0xd0..0xd2], &[0x34, 0x12]);
        r[0x58] = 1;
        r[0x5b] = 2;
        r[0x5c] = 7;
        assert_eq!(w15a06_forward_xy(&r, |a, b| (a, b)), (1, 0x200));
        assert_eq!(w15a06_forward_dword_5c(&r, |a| a), 7);
        w15a06_set_link_flag(Some(&mut r));
        assert_eq!(r[0x18], 1);
        w15a06_set_link_flag(None);
    }

    #[test]
    fn requirements() {
        assert_eq!(
            w15a06_requirement_check(b'#', 0x6a, || 0, |_| false),
            (true, None)
        );
        assert_eq!(
            w15a06_requirement_check(b'#', 0x6a + 0x16, || 0, |_| false),
            (false, Some(0x374))
        );
        assert_eq!(
            w15a06_requirement_check(b'#', 0, || 0, |_| false),
            (false, Some(0x374))
        );
        assert_eq!(
            w15a06_requirement_check(b'x', 0, || 0, |_| false),
            (true, None)
        );
        assert_eq!(
            w15a06_requirement_check(b'P', 0xbc, || 2, |_| true),
            (false, Some(0x37f))
        );
        assert_eq!(
            w15a06_requirement_check(b'P', 5, || 0, |_| true),
            (true, None)
        );
        assert_eq!(
            w15a06_requirement_check(b'P', 0x95, || 0, |_| false),
            (true, None)
        );
        assert_eq!(
            w15a06_requirement_check(b'P', 0x6e, || 1, |_| false),
            (true, None)
        );
        assert_eq!(
            w15a06_requirement_check(b'P', 0x9d, || 2, |_| false),
            (true, None)
        );
        assert_eq!(
            w15a06_requirement_check(b'P', 0x9d, || 0, |_| false),
            (false, Some(0x37c))
        );
    }

    #[derive(Default)]
    struct P {
        v: u8,
        q: [bool; 8],
        fin: u32,
    }
    impl W15a06StateProbe for P {
        fn small_value(&mut self) -> u8 {
            self.v
        }
        fn q_6370(&mut self) -> bool {
            self.q[0]
        }
        fn q_3150(&mut self) -> bool {
            self.q[1]
        }
        fn q_5680(&mut self) -> bool {
            self.q[2]
        }
        fn q_5d40(&mut self) -> bool {
            self.q[3]
        }
        fn q_5840(&mut self) -> bool {
            self.q[4]
        }
        fn q_3a90(&mut self) -> bool {
            self.q[5]
        }
        fn q_3a70(&mut self) -> bool {
            self.q[6]
        }
        fn q_b740(&mut self) -> bool {
            self.q[7]
        }
        fn finish_stop(&mut self) {
            self.fin += 1;
        }
    }

    #[test]
    fn choose_state() {
        let mut b = 0xffu8;
        let mut d = [5, 6];
        let mut p = P {
            v: 3,
            ..Default::default()
        };
        // nothing set: falls through to the b105 bit 0 branch -> 0xe
        assert_eq!(w15a06_choose_state(0, 0, &mut b, &mut d, &mut p), 0xe);
        assert_eq!(b, 0xfd);
        assert_eq!(d, [0, 0]);
        let mut p = P {
            v: 0xc,
            ..Default::default()
        };
        let mut b = 0xffu8;
        assert_eq!(w15a06_choose_state(0, 0, &mut b, &mut d, &mut p), 2);
        assert_eq!(b, 0xfc);
        let mut p = P::default();
        assert_eq!(w15a06_choose_state(0x2000, 0, &mut b, &mut d, &mut p), 1);
        p.q[0] = true;
        assert_ne!(w15a06_choose_state(0x2000, 0, &mut b, &mut d, &mut p), 1);
        let mut p = P::default();
        p.q[3] = true;
        assert_eq!(w15a06_choose_state(0, 0, &mut b, &mut d, &mut p), 6);
        assert_eq!(p.fin, 1);
        let mut p = P::default();
        p.q[5] = true;
        let mut b = 1u8;
        assert_eq!(w15a06_choose_state(0, 0, &mut b, &mut d, &mut p), 9);
        let mut p = P {
            v: 0xc,
            ..Default::default()
        };
        p.q[5] = true;
        let mut b = 0u8;
        assert_eq!(w15a06_choose_state(0, 0, &mut b, &mut d, &mut p), 7);
        let mut p = P::default();
        p.q[6] = true;
        assert_eq!(w15a06_choose_state(0, 0, &mut b, &mut d, &mut p), 5);
        p.q[0] = true;
        assert_eq!(w15a06_choose_state(0, 0, &mut b, &mut d, &mut p), 3);
    }

    #[test]
    fn box_checks() {
        assert!(w15a06_box_inside((10, 10), (5, 5, 5, 5), (100, 100)));
        assert!(!w15a06_box_inside((2, 10), (5, 5, 5, 5), (100, 100)));
        assert!(!w15a06_box_inside((95, 10), (5, 5, 5, 5), (100, 100)));
        assert!(!w15a06_box_inside((10, 95), (5, 5, 5, 5), (100, 100)));
        assert_eq!(w15a06_check_box_a(false, false, || {}), Some(1));
        let mut hit = false;
        assert_eq!(w15a06_check_box_a(true, true, || hit = true), Some(0));
        assert!(hit);
        assert_eq!(w15a06_check_box_a(true, false, || {}), None);
        assert_eq!(w15a06_check_box_b(true, true), None);
        assert_eq!(w15a06_check_box_b(false, true), Some(0));
        assert_eq!(w15a06_check_box_b(false, false), Some(1));
    }
}
