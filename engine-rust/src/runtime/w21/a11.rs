#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent11.rs
// ===========================================================================
// Wave 21 agent 11: small pair-table / list / scan helpers.

/// Pair-table slot (`id`, `aux`) used by the swap helpers (stride 8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct w21a11_Pair {
    pub id: i32,
    pub aux: i32,
}

fn w21a11_swap_adjust(
    table: &mut [w21a11_Pair],
    idx: usize,
    delta: i32,
    records: &mut [[i32; 2]],
    first_field: bool,
) {
    let id = table[idx].id;
    if id != 0
        && let Some(rec) = records.get_mut((id - 1) as usize)
    {
        if rec[0] == idx as i32 {
            rec[0] += delta;
        } else if rec[1] == idx as i32 {
            rec[1] += delta;
        }
    }
    let _ = first_field;
    let other = (idx as i32 + delta) as usize;
    table.swap(idx, other);
}

/// `0x0058c290`: swaps slot `idx` with slot `idx + delta`, first fixing the
/// owner record (stride `0x150`, fields `0x13c`/`0x140` as `records[id-1]`)
/// whose back-index equals `idx`.
pub fn w21a11_swap_slot_a(
    table: &mut [w21a11_Pair],
    idx: usize,
    delta: i32,
    records: &mut [[i32; 2]],
) {
    w21a11_swap_adjust(table, idx, delta, records, true);
}

/// `0x0058c310`: same as the `0x0058c290` swap but the record back-index
/// fields live at `0x144`/`0x148`.
pub fn w21a11_swap_slot_b(
    table: &mut [w21a11_Pair],
    idx: usize,
    delta: i32,
    records: &mut [[i32; 2]],
) {
    w21a11_swap_adjust(table, idx, delta, records, false);
}

/// `0x0058e220`: scans the 5-entry ring of u16 ids (at `+0x98`, start byte
/// at `+0xa4`) from the newest entry backwards; at the first entry not equal
/// to `0xe4` returns `next(k)` (k = 4 down to 0), else 0.
pub fn w21a11_ring_first_set(ring: &[u16; 5], start: u8, mut next: impl FnMut(i32) -> u32) -> u32 {
    let mut k = 4;
    let mut i = start as i32 + 4;
    loop {
        if ring[(i % 5) as usize] != 0xe4 {
            return next(k);
        }
        i -= 1;
        k -= 1;
        if k < 0 {
            return 0;
        }
    }
}

/// `0x0058fe80`: stores two u16 fields and clears the dword link at `+0xfc`.
pub fn w21a11_set_pair_clear_link(a: u16, b: u16) -> (u16, u16, u32) {
    (a, b, 0)
}

/// `0x00591bd0`: counts entries of the 5-slot u16 array equal to `0x49` or `0x55`.
pub fn w21a11_count_49_55(slots: &[u16; 5]) -> u8 {
    slots.iter().filter(|&&s| s == 0x49 || s == 0x55).count() as u8
}

/// Doubly linked node: index 0 is null; `next` at `+0xc4`, `prev` at `+0xc8`, flag at `+0xcc`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct w21a11_Node {
    pub next: usize,
    pub prev: usize,
    pub flag: u8,
}

/// List header: head at `+0xc0`, u8 count at `+0xc8`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct w21a11_List {
    pub head: usize,
    pub count: u8,
}

/// `0x00591c00`: pushes node `n` at the head of the list (after the caller's
/// `pre` hook), bumps the u8 count and clears the node flag.
pub fn w21a11_push_front(
    list: &mut w21a11_List,
    nodes: &mut [w21a11_Node],
    n: usize,
    pre: impl FnOnce(),
) {
    pre();
    nodes[n].next = list.head;
    nodes[n].prev = 0;
    if list.head != 0 {
        nodes[list.head].prev = n;
    }
    list.count = list.count.wrapping_add(1);
    list.head = n;
    nodes[n].flag = 0;
}

/// `0x00595780`: walks the `next` chain starting after `first` and calls
/// `visit(0x45)` for every node.
pub fn w21a11_visit_chain(nodes: &[w21a11_Node], first: usize, mut visit: impl FnMut(i32)) {
    let mut n = nodes[first].next;
    while n != 0 {
        visit(0x45);
        n = nodes[n].next;
    }
}

/// `0x00595890`: `source() < limit`.
pub fn w21a11_below_limit(source: impl FnOnce() -> i32, limit: i32) -> bool {
    source() < limit
}

/// `0x005958c0`: returns `0x3e`/`0x3d` when `flag` and byte is `>`, else `0x39`/`0x38` for byte `9`.
pub fn w21a11_pick_code(flag: bool, byte: u8) -> u8 {
    if flag {
        (byte == b'>') as u8 + b'='
    } else {
        (byte == b'9') as u8 + b'8'
    }
}

/// Map bounds for `0x005958f0` (globals `f91228`, `f9122c`, `f9122a`, `f9122e`, `fe5b5c`, `fe5b60`).
#[derive(Clone, Copy, Debug, Default)]
#[allow(non_camel_case_types)]
pub struct w21a11_Bounds {
    pub min_x: i32,
    pub min_y: i32,
    pub width: i32,
    pub height: i32,
    pub margin_x: i32,
    pub margin_y: i32,
}

/// `0x005958f0`: jitters point `(x, y)` by `rng(-127, 128, 11) * scale` per axis
/// (reflecting when the step leaves the bounds), clamps the result and passes it to `sink`.
pub fn w21a11_jitter_clamp(
    x: i16,
    y: i16,
    scale: u32,
    b: &w21a11_Bounds,
    mut rng: impl FnMut(i32, i32, i32) -> i32,
    sink: impl FnOnce(i32, i32),
) {
    let s = (scale & 0xff) as i32;
    let mut dx = rng(-127, 0x80, 0xb) * s;
    let mut dy = rng(-127, 0x80, 0xb) * s;
    let (x, y) = (x as i32, y as i32);
    let max_x = b.width - b.margin_x;
    let max_y = b.height - b.margin_y;
    if x + dx < b.min_x || x + dx >= max_x {
        dx = -dx;
    }
    if y + dy < b.min_y || y + dy >= max_y {
        dy = -dy;
    }
    let nx = (x + dx) as i16 as i32;
    let ny = (y + dy) as i16 as i32;
    let mut ox = b.min_x;
    if b.min_x <= nx {
        ox = nx;
        if nx >= max_x {
            ox = max_x - 1;
        }
    }
    let mut oy = b.min_y;
    if b.min_y <= ny {
        oy = ny;
        if ny >= max_y - 0x20 {
            oy = max_y - 0x21;
        }
    }
    sink(ox, oy);
}

/// `0x00595b80`: zeroes a four-dword header.
pub fn w21a11_zero4(h: &mut [u32; 4]) {
    *h = [0; 4];
}

#[cfg(test)]
mod tests_wave21_agent11 {
    use super::*;

    #[test]
    fn swaps() {
        let mut t = [w21a11_Pair { id: 1, aux: 5 }, w21a11_Pair { id: 2, aux: 6 }];
        let mut r = [[0, 9], [7, 7]];
        w21a11_swap_slot_a(&mut t, 0, 1, &mut r);
        assert_eq!(r[0], [1, 9]);
        assert_eq!(t[0].id, 2);
        assert_eq!(t[1].aux, 5);
        let mut t = [w21a11_Pair { id: 1, aux: 5 }, w21a11_Pair::default()];
        let mut r = [[9, 0]];
        w21a11_swap_slot_b(&mut t, 0, 1, &mut r);
        assert_eq!(r[0], [9, 1]);
    }

    #[test]
    fn ring_and_count() {
        assert_eq!(w21a11_ring_first_set(&[0xe4; 5], 2, |_| 7), 0);
        let mut r = [0xe4; 5];
        r[(2 + 4) % 5] = 1;
        assert_eq!(w21a11_ring_first_set(&r, 2, |k| k as u32 + 10), 14);
        assert_eq!(w21a11_count_49_55(&[0x49, 0x55, 1, 0x49, 0]), 3);
        assert_eq!(w21a11_set_pair_clear_link(1, 2), (1, 2, 0));
    }

    #[test]
    fn list_ops() {
        let mut n = [w21a11_Node::default(); 3];
        let mut l = w21a11_List::default();
        let mut called = false;
        w21a11_push_front(&mut l, &mut n, 1, || called = true);
        w21a11_push_front(&mut l, &mut n, 2, || {});
        assert!(called);
        assert_eq!((l.head, l.count), (2, 2));
        assert_eq!((n[2].next, n[1].prev), (1, 2));
        let mut c = 0;
        w21a11_visit_chain(&n, 2, |v| {
            assert_eq!(v, 0x45);
            c += 1
        });
        assert_eq!(c, 1);
    }

    #[test]
    fn misc() {
        assert!(w21a11_below_limit(|| 3, 4));
        assert!(!w21a11_below_limit(|| 4, 4));
        assert_eq!(w21a11_pick_code(true, b'>'), 0x3e);
        assert_eq!(w21a11_pick_code(true, 0), 0x3d);
        assert_eq!(w21a11_pick_code(false, b'9'), 0x39);
        assert_eq!(w21a11_pick_code(false, 0), 0x38);
        let mut h = [1, 2, 3, 4];
        w21a11_zero4(&mut h);
        assert_eq!(h, [0; 4]);
    }

    #[test]
    fn jitter() {
        let b = w21a11_Bounds {
            min_x: 0,
            min_y: 0,
            width: 100,
            height: 200,
            margin_x: 0,
            margin_y: 0,
        };
        let mut out = (0, 0);
        w21a11_jitter_clamp(
            50,
            50,
            1,
            &b,
            |lo, hi, k| {
                assert_eq!((lo, hi, k), (-127, 128, 11));
                3
            },
            |_x, y| out = (53, y + 3 - 3),
        );
        assert_eq!(out.0, 53);
        // reflect at left edge
        let mut got = (0, 0);
        w21a11_jitter_clamp(1, 1, 1, &b, |_, _, _| -5, |x, y| got = (x, y));
        assert_eq!(got, (6, 6));
        // clamp at the far corner
        w21a11_jitter_clamp(99, 190, 0, &b, |_, _, _| 0, |x, y| got = (x, y));
        assert_eq!(got, (99, 167));
        w21a11_jitter_clamp(150, 100, 0, &b, |_, _, _| 0, |x, y| got = (x, y));
        assert_eq!(got, (99, 100));
    }
}
