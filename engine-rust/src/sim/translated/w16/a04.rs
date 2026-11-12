#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave16/agent04.rs
// ===========================================================================
// Wave 16 translation batch 04.
//
// Skipped (reason in each line):
// FUN_00569190 global list plus callee chain; FUN_00569540 bare call chain;
// FUN_005697c0 ends in noreturn call; FUN_0056c970 noreturn thunk;
// FUN_0056cb40 ends in noreturn call; FUN_0056cbf0 single callee call;
// FUN_0056cc90 noreturn call chain; FUN_00571580 global list bookkeeping;
// FUN_00571610 bare call pair; FUN_00571640 bare call pair;
// FUN_00571d60 noreturn call chain; FUN_00571dd0 bare call chain;
// FUN_005729c0 bare call chain.

/// Sprite-like record fields touched by the init/position helpers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W16a04Obj {
    pub x10: i16,
    pub y12: i16,
    pub f14: u32,
    pub f18: u32,
    pub flags20: u8,
    pub f21: u8,
    pub f22: u8,
    pub f23: u8,
    pub f24: u16,
    pub f27: u8,
    pub x28: i16,
    pub y2a: i16,
    pub f2c: i32,
    pub f30: i32,
    pub f34: u32,
    pub f3c: u32,
    pub f48: u16,
}

/// Per-type lookup tables used by `w16a04_init_obj`.
pub struct W16a04Tables<'a> {
    pub t148: &'a [u32],
    pub t490: &'a [u16],
    pub t980: &'a [u8],
    pub tb30: &'a [u8],
}

fn pack_xy(x: i16, y: i16) -> u32 {
    (x as u16 as u32) | ((y as u16 as u32) << 16)
}

/// Binary search over a monotone table, signed result: 0x00569120.
/// `table` needs at least 0x40 entries.
pub fn w16a04_signed_table_search(value: i32, table: &[u32]) -> i32 {
    let abs = (value ^ (value >> 31)).wrapping_sub(value >> 31) as u32;
    let mut lo: u32 = 0;
    let mut hi: u32 = 0x40;
    let mut mid: u32 = 0x20;
    loop {
        let mut next_lo = mid + 1;
        if abs <= table[mid as usize] {
            hi = mid;
            next_lo = lo;
        }
        lo = next_lo;
        mid = (hi + lo) >> 1;
        if hi == lo {
            break;
        }
    }
    if value >= 0 {
        mid as i32
    } else {
        -(mid as i32)
    }
}

/// Init of an object slot; `post(x, y)` is the placement callee
/// (0x58_8e10, true = success, then 0x57_d5d0 side effect): 0x00569b10.
#[allow(clippy::too_many_arguments)]
pub fn w16a04_init_obj(
    o: &mut W16a04Obj,
    kind: u16,
    x: i16,
    y: i16,
    fill: u8,
    t: &W16a04Tables,
    mut place: impl FnMut(u16, i16, i16) -> bool,
    placed_ok: impl FnOnce(u8),
) -> bool {
    let k = kind as usize;
    o.f24 = kind;
    o.flags20 = 0;
    o.f3c = 0;
    o.f34 = t.t148[k];
    o.f48 = t.t490[k];
    o.f22 = t.t980[k];
    o.f27 = t.tb30[k];
    o.f2c = (x as i32) << 8;
    o.x28 = x;
    o.y2a = y;
    o.f30 = (y as i32) << 8;
    if x != o.x10 || y != o.y12 {
        o.x10 = x;
        o.y12 = y;
        o.f14 = 0;
        o.flags20 = 1;
        o.f18 = pack_xy(o.x10, o.y12);
    }
    o.f21 = fill;
    o.f23 = fill;
    if !place(kind, x, y) {
        return false;
    }
    placed_ok(o.f21);
    true
}

/// Clamp a heading byte toward a target by at most 0x20: 0x00569300.
pub fn w16a04_heading_clamp(cur: i8, target: i8) -> i8 {
    let d = target.wrapping_sub(cur);
    if d < -0x7f {
        cur.wrapping_sub(0x60)
    } else if d < -0x20 {
        cur.wrapping_add(0x20).wrapping_add(d)
    } else if d > 0x20 {
        target.wrapping_sub(0x20)
    } else {
        cur
    }
}

/// Move object to a new position, marking it dirty: 0x0056c930.
pub fn w16a04_set_pos(o: &mut W16a04Obj, x: i32, y: i32) {
    if x != o.x10 as i32 || y != o.y12 as i32 {
        o.x10 = x as i16;
        o.y12 = y as i16;
        o.flags20 = (o.flags20 & 0xfb) | 1;
        o.f14 = 0;
        o.f18 = pack_xy(o.x10, o.y12);
    }
}

/// Update cached (dx, dy) offsets from a frame-table byte pair; returns the
/// new (flags, dx, dy) with bit 0 set on change: 0x00573450.
pub fn w16a04_frame_offset(
    flags_c: u16,
    cur: (i8, i8),
    frame_flags: u8,
    pair: (i8, i8),
) -> (u16, i8, i8) {
    let mut dx = pair.0;
    if frame_flags & 2 != 0 {
        dx = dx.wrapping_neg();
    }
    let dy = pair.1;
    if cur.0 != dx || cur.1 != dy {
        (flags_c | 1, dx, dy)
    } else {
        (flags_c, cur.0, cur.1)
    }
}

/// Animation state request. Returns `Some((new_state, f12))` when the state
/// changes (caller then resets 0x17/0x14 to 0 and calls the 0x57_34f0
/// refresh). `lookup(state)` reads the u16 at table+8+state*2: 0x005748f0.
pub fn w16a04_request_anim(
    req: u8,
    cur: u8,
    flags_c: u8,
    lookup: impl Fn(u8) -> u16,
) -> Option<(u8, u16)> {
    if req == 1 && cur == 1 {
        return None;
    }
    if flags_c & 0x10 == 0 && req != 0 && req != 1 {
        return None;
    }
    if req == cur && (req == 0xb || req == 0x13) {
        return None;
    }
    let mut s = req;
    if req == 5 {
        if cur != 5 && cur != 2 {
            s = 2;
        }
    } else if req == 6 && cur != 6 && cur != 3 {
        s = 3;
    }
    Some((s, lookup(s)))
}

/// Doubly linked node for `w16a04_list_insert_after`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W16a04Node {
    pub next: Option<usize>,
    pub prev: Option<usize>,
}

/// Insert `new` after `this`; updates `tail` if `this` was the tail:
/// 0x005771e0.
pub fn w16a04_list_insert_after(
    nodes: &mut [W16a04Node],
    this: usize,
    new: usize,
    tail: &mut Option<usize>,
) {
    if *tail == Some(this) {
        *tail = Some(new);
    }
    nodes[new].next = nodes[this].next;
    nodes[new].prev = Some(this);
    if let Some(n) = nodes[this].next {
        nodes[n].prev = Some(new);
    }
    nodes[this].next = Some(new);
}

#[cfg(test)]
mod tests_wave16_agent04 {
    use super::*;

    #[test]
    fn search() {
        let t: Vec<u32> = (0..0x41).map(|i| i * 10).collect();
        assert_eq!(w16a04_signed_table_search(35, &t), 4);
        assert_eq!(w16a04_signed_table_search(-35, &t), -4);
        assert_eq!(w16a04_signed_table_search(0, &t), 0);
    }

    #[test]
    fn heading() {
        assert_eq!(w16a04_heading_clamp(0, 10), 0);
        assert_eq!(w16a04_heading_clamp(0, 50), 18);
        assert_eq!(w16a04_heading_clamp(0, -50), -18);
        assert_eq!(w16a04_heading_clamp(0, -128), -0x60);
    }

    #[test]
    fn pos_and_init() {
        let mut o = W16a04Obj::default();
        w16a04_set_pos(&mut o, 3, 4);
        assert_eq!(o.flags20, 1);
        assert_eq!(o.f18, 3 | (4 << 16));
        let t = W16a04Tables {
            t148: &[7, 8],
            t490: &[1, 2],
            t980: &[3, 4],
            tb30: &[5, 6],
        };
        let mut seen = None;
        assert!(w16a04_init_obj(
            &mut o,
            1,
            9,
            2,
            5,
            &t,
            |_, _, _| true,
            |f| seen = Some(f)
        ));
        assert_eq!((o.f34, o.f48, o.f22, o.f27, o.f2c), (8, 2, 4, 6, 9 << 8));
        assert_eq!(o.flags20, 1);
        assert_eq!(seen, Some(5));
        assert!(!w16a04_init_obj(
            &mut o,
            0,
            9,
            2,
            0,
            &t,
            |_, _, _| false,
            |_| {}
        ));
    }

    #[test]
    fn frame_and_anim() {
        assert_eq!(w16a04_frame_offset(0, (1, 2), 2, (1, 2)), (1, -1, 2));
        assert_eq!(w16a04_frame_offset(0, (1, 2), 0, (1, 2)), (0, 1, 2));
        assert_eq!(w16a04_request_anim(1, 1, 0, |s| s as u16), None);
        assert_eq!(
            w16a04_request_anim(5, 0, 0x10, |s| s as u16 * 2),
            Some((2, 4))
        );
        assert_eq!(w16a04_request_anim(6, 6, 0x10, |s| s as u16), Some((6, 6)));
        assert_eq!(w16a04_request_anim(0xb, 0xb, 0x10, |s| s as u16), None);
        assert_eq!(w16a04_request_anim(3, 0, 0, |s| s as u16), None);
    }

    #[test]
    fn list() {
        let mut n = vec![W16a04Node::default(); 3];
        let mut tail = Some(0);
        w16a04_list_insert_after(&mut n, 0, 1, &mut tail);
        w16a04_list_insert_after(&mut n, 0, 2, &mut tail);
        assert_eq!(tail, Some(1));
        assert_eq!(n[0].next, Some(2));
        assert_eq!(n[2].next, Some(1));
        assert_eq!(n[1].prev, Some(2));
    }
}
