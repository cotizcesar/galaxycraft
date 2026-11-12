#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent11.rs
// ===========================================================================
// Wave 27 translation batch 11: dialog-control helpers.

/// Minimal view of a dialog control (offsets from the retail record).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W27a11Ctl {
    /// +0x06
    pub left: i16,
    /// +0x0a
    pub right: i16,
    /// +0x3a: index among siblings
    pub index: i16,
    /// +0x3c: control type
    pub kind: i16,
    /// +0x3e
    pub sub: i16,
    /// +0x50: checked/selected state
    pub state: u32,
    /// +0x63
    pub height: u8,
    /// +0x6a
    pub min: i16,
}

/// Dialog event (`+0` code, `+4` value, `+0x10` kind).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W27a11Event {
    pub code: i32,
    pub value: i32,
    pub kind: i16,
}

/// `0x00978f30`: when the control check passes, runs the reset callback,
/// snaps `left` to `min`, sets `right = height + min`; returns true when
/// the global at 011ce23c must be cleared.
pub fn w27a11_reset_extent(c: &mut W27a11Ctl, check: bool, mut reset: impl FnMut()) -> bool {
    if !check {
        return false;
    }
    reset();
    c.left = c.min;
    c.right = (c.height as i16).wrapping_add(c.min);
    true
}

/// `0x00979cd0`: reads the u32 slot `idx` of a table.
pub fn w27a11_slot(table: &[u32], idx: u8) -> Option<u32> {
    table.get(idx as usize).copied()
}

/// `0x0097a1d0`: forwards the byte at +0x62 to the callee.
pub fn w27a11_forward_byte62(byte62: u8, mut f: impl FnMut(u8)) {
    f(byte62)
}

/// `0x0097c270` (table part): squares table, index `i + 255` holds `i * i`
/// for `i` in `-255..=255` (the retail loop fills 0..=255 at +0 and the
/// vector loop starts at -255).
pub fn w27a11_square_table() -> [i32; 511] {
    let mut t = [0i32; 511];
    for (k, v) in t.iter_mut().enumerate() {
        let i = k as i32 - 255;
        *v = i * i;
    }
    t
}

fn w27a11_clamp_right(c: &mut W27a11Ctl, pad: u8) {
    let want = (c.left as i32 + pad as i32) as i16;
    if c.right <= want {
        c.right = want;
    }
}

/// `0x0097d150`: toggle-style handler for kind 0xe events.  Returns true
/// when handled; false means fall back to the default handler.
pub fn w27a11_toggle_event(c: &mut W27a11Ctl, e: &W27a11Event, pad: u8) -> bool {
    if e.kind == 0xe {
        match e.code {
            2 => {
                c.state ^= 1;
                return true;
            }
            10 => {
                c.sub = 0xb;
                w27a11_clamp_right(c, pad);
            }
            0xb => {
                c.state = (e.value != 0) as u32;
                return true;
            }
            _ => {}
        }
    }
    false
}

/// `0x0097d1e0`: for kind 0xe code 2 on a control with index -3 or -2,
/// returns that index (to store in the global) and handles the event.
pub fn w27a11_index_event(c: &W27a11Ctl, e: &W27a11Event) -> Option<i16> {
    if e.kind == 0xe && e.code == 2 && (c.index == -3 || c.index == -2) {
        Some(c.index)
    } else {
        None
    }
}

/// `0x0097d230`: radio-style handler; selects through `select` on code 2 or
/// code 0xb with nonzero value.  Returns true when handled.
pub fn w27a11_radio_event(
    c: &mut W27a11Ctl,
    e: &W27a11Event,
    pad: u8,
    mut select: impl FnMut(),
) -> bool {
    if e.kind == 0xe {
        match e.code {
            2 => {
                select();
                return true;
            }
            10 => w27a11_clamp_right(c, pad),
            0xb => {
                if e.value == 0 {
                    return true;
                }
                select();
                return true;
            }
            _ => {}
        }
    }
    false
}

/// Radio group member: type, sibling index and state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W27a11Node {
    pub kind: i16,
    pub index: i16,
    pub state: u32,
}

/// `0x0097d410`: walks backwards over kind-3 siblings from `start`, returns
/// the first member of the group.  `find(i)` is FUN_009642f0 returning the
/// node position with sibling index `i`.
pub fn w27a11_group_head(
    nodes: &[W27a11Node],
    start: usize,
    mut find: impl FnMut(i16) -> Option<usize>,
) -> usize {
    let mut head = start;
    let mut cur = find(nodes[start].index - 1);
    while let Some(i) = cur {
        if nodes[i].kind != 3 {
            break;
        }
        cur = find(nodes[i].index - 1);
        head = i;
    }
    head
}

/// `0x0097d460`: clears every member of the group and selects `start`.
pub fn w27a11_group_select(
    nodes: &mut [W27a11Node],
    start: usize,
    mut find: impl FnMut(i16) -> Option<usize>,
) {
    let head = w27a11_group_head(nodes, start, &mut find);
    nodes[head].state = 0;
    let mut cur = find(nodes[head].index + 1);
    while let Some(i) = cur {
        if nodes[i].kind != 3 {
            break;
        }
        nodes[i].state = 0;
        cur = find(nodes[i].index + 1);
    }
    nodes[start].state = 1;
}

#[cfg(test)]
mod tests_wave27_agent11 {
    use super::*;

    #[test]
    fn extent_and_slots() {
        let mut c = W27a11Ctl {
            min: 5,
            height: 20,
            left: 1,
            right: 2,
            ..Default::default()
        };
        let mut n = 0;
        assert!(!w27a11_reset_extent(&mut c, false, || n += 1));
        assert!(w27a11_reset_extent(&mut c, true, || n += 1));
        assert_eq!((c.left, c.right, n), (5, 25, 1));
        assert_eq!(w27a11_slot(&[1, 2], 1), Some(2));
        assert_eq!(w27a11_slot(&[1, 2], 2), None);
        let mut got = 0;
        w27a11_forward_byte62(7, |b| got = b);
        assert_eq!(got, 7);
    }

    #[test]
    fn squares() {
        let t = w27a11_square_table();
        assert_eq!(t[0], 65025);
        assert_eq!(t[255], 0);
        assert_eq!(t[510], 65025);
    }

    #[test]
    fn events() {
        let mut c = W27a11Ctl {
            left: 10,
            right: 5,
            index: -3,
            ..Default::default()
        };
        let ev = |code, value, kind| W27a11Event { code, value, kind };
        assert!(w27a11_toggle_event(&mut c, &ev(2, 0, 0xe), 4));
        assert_eq!(c.state, 1);
        assert!(w27a11_toggle_event(&mut c, &ev(0xb, 0, 0xe), 4));
        assert_eq!(c.state, 0);
        assert!(!w27a11_toggle_event(&mut c, &ev(10, 0, 0xe), 4));
        assert_eq!((c.sub, c.right), (0xb, 14));
        assert!(!w27a11_toggle_event(&mut c, &ev(2, 0, 1), 4));
        assert_eq!(w27a11_index_event(&c, &ev(2, 0, 0xe)), Some(-3));
        assert_eq!(w27a11_index_event(&c, &ev(3, 0, 0xe)), None);
        let mut n = 0;
        assert!(w27a11_radio_event(&mut c, &ev(0xb, 0, 0xe), 0, || n += 1));
        assert_eq!(n, 0);
        assert!(w27a11_radio_event(&mut c, &ev(0xb, 1, 0xe), 0, || n += 1));
        assert!(w27a11_radio_event(&mut c, &ev(2, 0, 0xe), 0, || n += 1));
        assert_eq!(n, 2);
        assert!(!w27a11_radio_event(&mut c, &ev(10, 0, 0xe), 0, || n += 1));
    }

    #[test]
    fn groups() {
        let mk = |kind, index| W27a11Node {
            kind,
            index,
            state: 1,
        };
        let mut nodes = vec![mk(2, 0), mk(3, 1), mk(3, 2), mk(3, 3), mk(2, 4)];
        let find = |nodes: Vec<i16>| move |i: i16| nodes.iter().position(|&x| x == i);
        let idx: Vec<i16> = nodes.iter().map(|n| n.index).collect();
        // node 0 (kind 2) is not part of the group: head of node 3 is node 1.
        assert_eq!(w27a11_group_head(&nodes, 3, find(idx.clone())), 1);
        w27a11_group_select(&mut nodes, 2, find(idx));
        let s: Vec<u32> = nodes.iter().map(|n| n.state).collect();
        assert_eq!(s, vec![1, 0, 1, 0, 1]);
    }
}
