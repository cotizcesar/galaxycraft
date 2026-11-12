#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent01.rs
// ===========================================================================
// Wave 21 agent 01: client UI helper predicates and list walkers.

/// Minimal view of a linked-list node used by the walkers below: the state
/// byte at node offset `+10` and the two payload bytes at `+0x30`/`+0x31`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W21a01Node {
    pub state: u8,
    pub b30: u8,
    pub b31: u8,
}

/// `0x00570fa0`: returns `base + 4 + offset` (wrapping, 32-bit).
pub fn w21a01_offset_plus_4(base: i32, offset: i32) -> i32 {
    base.wrapping_add(4).wrapping_add(offset)
}

/// `0x00572310`: returns `0x2f3`, plus `0xf4` when the probe is non-zero.
pub fn w21a01_select_constant(probe_nonzero: bool) -> i32 {
    (if probe_nonzero { 0xf4 } else { 0 }) + 0x2f3
}

/// `0x00578ab0`: advances the 2-bit field (bits 1..=2) of the flag byte
/// at `+0xe` unless it already equals `mode - 1`; when the new field is
/// below 3 and `check()` is false the field bits are cleared (`& 0xf9`).
pub fn w21a01_advance_mode(flags: &mut u8, mode: u8, mut check: impl FnMut() -> bool) {
    if ((*flags >> 1) & 3) as u32 != (mode as u32).wrapping_sub(1) {
        let b = *flags;
        let n = (((b as u32 + 2) ^ b as u32) & 6 ^ b as u32) as u8;
        *flags = n;
        if (n & 6) < 3 && !check() {
            *flags &= 0xf9;
        }
    }
}

/// `0x0057b630`: after `pre()`, kind 1 / 2 call `on_node(i, 2 or 4)` for
/// every node with state 0 and store `p2`/`p3`; kind 3 calls `on_node(i, 3)`
/// for nodes in state 0, 2 or 4.
pub fn w21a01_update_nodes(
    nodes: &mut [W21a01Node],
    kind: u8,
    p2: u8,
    p3: u8,
    mut pre: impl FnMut(),
    mut on_node: impl FnMut(usize, u8),
) {
    pre();
    for (i, n) in nodes.iter_mut().enumerate() {
        match kind {
            1 | 2 if n.state == 0 => {
                on_node(i, if kind == 1 { 2 } else { 4 });
                n.b30 = p2;
                n.b31 = p3;
            }
            3 if matches!(n.state, 0 | 2 | 4) => on_node(i, 3),
            _ => {}
        }
    }
}

/// `0x0057b820`: keeps the high byte of `id` and replaces the low byte with
/// `table[id]` (0 if out of range).
pub fn w21a01_remap_low_byte(id: u16, table: &[u8]) -> u16 {
    (id & 0xff00) | table.get(id as usize).copied().unwrap_or(0) as u16
}

/// `0x0057c570`: finds the first `(kind, byte)` item with
/// `0x23a < kind < 0x245`; calls `on_found()` and returns its byte, else 0.
pub fn w21a01_find_kind_byte(items: &[(u16, u8)], mut on_found: impl FnMut()) -> u8 {
    for &(kind, b) in items {
        if 0x23a < kind && kind < 0x245 {
            on_found();
            return b;
        }
    }
    0
}

/// `0x0057d190`: when flag bit 3 is clear stores `value` at `+0xb`, sets
/// bit 3, calls `refresh()`, and sets bit 0 if clear and `check()` is true.
pub fn w21a01_set_value(
    flags: &mut u8,
    slot: &mut u8,
    value: u8,
    mut refresh: impl FnMut(),
    mut check: impl FnMut() -> bool,
) {
    if *flags & 8 != 0 {
        return;
    }
    *slot = value;
    *flags |= 8;
    refresh();
    if *flags & 1 == 0 && check() {
        *flags |= 1;
    }
}

/// `0x0057dca0`: with flag bit 6 clear, states 2..=4 call `on_node(i, state+3)`
/// and state 0 calls `on_node(i, 6)`; with it set, states 2..=7 call
/// `on_node(i, 0)`.
pub fn w21a01_dispatch_states(flags: u8, nodes: &[W21a01Node], mut on_node: impl FnMut(usize, u8)) {
    for (i, n) in nodes.iter().enumerate() {
        let s = n.state;
        if flags & 0x40 == 0 {
            if s.wrapping_sub(2) < 3 {
                on_node(i, s.wrapping_add(3));
            } else if s == 0 {
                on_node(i, 6);
            }
        } else if s.wrapping_sub(2) < 6 {
            on_node(i, 0);
        }
    }
}

/// Globals read by `0x0058b730` (area walkability test).
pub struct W21a01AreaCtx<'a> {
    pub flag_4580: u32,
    pub row_width: u16,
    pub tiles: &'a [u32],
    pub mask_7c0: u32,
    pub mask_4578: u8,
    pub flag_457c: u32,
}

/// `0x0058b730`: scans the `w` x `h` rectangle at (`x`,`y`) from the
/// bottom-right corner; returns false at the first blocked cell.  Tile index
/// is `row_width * row + col`; out-of-range tiles read as 0.  `alt()` is the
/// probe made when `flag_4580 == 0`.  Assumes the mask at global `4578` is a
/// byte (the pseudocode casts the tile to `byte`).
pub fn w21a01_area_clear(
    ctx: &W21a01AreaCtx,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
    mut alt: impl FnMut() -> bool,
) -> bool {
    let mut row = y as i32 - 1 + h as i32;
    if (y as i32) > row {
        return true;
    }
    let right = x as i32 - 1 + w as i32;
    loop {
        let mut col = right;
        while x as i32 <= col {
            let idx = ctx.row_width as i64 * row as i64 + col as i64;
            let tile = usize::try_from(idx)
                .ok()
                .and_then(|i| ctx.tiles.get(i))
                .copied()
                .unwrap_or(0);
            if ctx.flag_4580 == 0 && !alt() {
                if tile & ctx.mask_7c0 == 0 {
                    return false;
                }
            } else {
                if ctx.mask_4578 & !(tile as u8) != 0 {
                    return false;
                }
                if ctx.flag_457c != 0 {
                    return false;
                }
            }
            col -= 1;
        }
        let more = (y as i32) < row;
        row -= 1;
        if !more {
            break;
        }
    }
    true
}

#[cfg(test)]
mod tests_wave21_agent01 {
    use super::*;

    #[test]
    fn small_helpers() {
        assert_eq!(w21a01_offset_plus_4(10, 3), 17);
        assert_eq!(w21a01_select_constant(false), 0x2f3);
        assert_eq!(w21a01_select_constant(true), 0x3e7);
        assert_eq!(w21a01_remap_low_byte(0x1202, &[0, 0, 0x77]), 0x1200);
        assert_eq!(w21a01_remap_low_byte(0x0002, &[0, 0, 0x77]), 0x0077);
    }

    #[test]
    fn advance_mode() {
        let mut f = 0u8;
        w21a01_advance_mode(&mut f, 1, || true);
        assert_eq!(f, 0); // field 0 == mode-1: unchanged
        w21a01_advance_mode(&mut f, 3, || true);
        assert_eq!(f, 2);
        let mut f = 0x80u8;
        w21a01_advance_mode(&mut f, 3, || false);
        assert_eq!(f, 0x80);
        let mut f = 0x04u8; // field 2 -> next 6 (not <3), no check
        w21a01_advance_mode(&mut f, 1, || false);
        assert_eq!(f, 0x06);
    }

    #[test]
    fn update_nodes() {
        let mut n = [
            W21a01Node::default(),
            W21a01Node {
                state: 1,
                ..Default::default()
            },
        ];
        let mut calls = vec![];
        let mut pre = vec![];
        w21a01_update_nodes(
            &mut n,
            2,
            5,
            6,
            || pre.push(99),
            |i, a| calls.push(i as u8 * 10 + a),
        );
        assert_eq!(pre, vec![99]);
        assert_eq!(calls, vec![4]);
        assert_eq!((n[0].b30, n[0].b31, n[1].b30), (5, 6, 0));
        let mut n = [
            W21a01Node {
                state: 4,
                ..Default::default()
            },
            W21a01Node {
                state: 3,
                ..Default::default()
            },
        ];
        let mut c = vec![];
        w21a01_update_nodes(&mut n, 3, 0, 0, || {}, |i, a| c.push((i, a)));
        assert_eq!(c, vec![(0, 3)]);
    }

    #[test]
    fn find_kind() {
        let mut hit = 0;
        assert_eq!(
            w21a01_find_kind_byte(&[(0x23a, 1), (0x23b, 9)], || hit += 1),
            9
        );
        assert_eq!(hit, 1);
        assert_eq!(w21a01_find_kind_byte(&[(0x245, 1)], || hit += 1), 0);
        assert_eq!(hit, 1);
    }

    #[test]
    fn set_value() {
        let (mut f, mut s) = (0u8, 0u8);
        w21a01_set_value(&mut f, &mut s, 7, || {}, || true);
        assert_eq!((f, s), (9, 7));
        w21a01_set_value(&mut f, &mut s, 8, || {}, || true);
        assert_eq!(s, 7);
    }

    #[test]
    fn dispatch() {
        let nodes: Vec<_> = [0u8, 2, 4, 5, 7, 8]
            .iter()
            .map(|&state| W21a01Node {
                state,
                ..Default::default()
            })
            .collect();
        let mut c = vec![];
        w21a01_dispatch_states(0, &nodes, |i, a| c.push((i, a)));
        assert_eq!(c, vec![(0, 6), (1, 5), (2, 7)]);
        let mut c = vec![];
        w21a01_dispatch_states(0x40, &nodes, |i, a| c.push((i, a)));
        assert_eq!(c, vec![(1, 0), (2, 0), (3, 0), (4, 0)]);
    }

    #[test]
    fn area_clear() {
        let mut tiles = vec![1u32; 16];
        let mk = |t: &'static [u32]| W21a01AreaCtx {
            flag_4580: 0,
            row_width: 4,
            tiles: t,
            mask_7c0: 1,
            mask_4578: 0,
            flag_457c: 0,
        };
        let t: &'static [u32] = Box::leak(tiles.clone().into_boxed_slice());
        assert!(w21a01_area_clear(&mk(t), 1, 1, 2, 2, || false));
        tiles[5] = 0;
        let t: &'static [u32] = Box::leak(tiles.into_boxed_slice());
        assert!(!w21a01_area_clear(&mk(t), 1, 1, 2, 2, || false));
        assert!(w21a01_area_clear(&mk(t), 1, 1, 0, 0, || false));
        let mut c = mk(t);
        c.flag_457c = 1;
        assert!(!w21a01_area_clear(&c, 2, 2, 1, 1, || true));
    }
}
