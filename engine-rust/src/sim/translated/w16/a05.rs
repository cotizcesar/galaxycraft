#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave16/agent05.rs
// ===========================================================================
// Wave 16 translation batch 05.
//
// Pure helpers over the retail 0x28-stride record table and its linked
// child lists.  Callees not yet translated are closure parameters.
//
// Skipped (FUN_ names, no `0x`):
// FUN_00578b10, FUN_00578c70, FUN_00578cb0, FUN_00578d20, FUN_00578dc0,
// FUN_00578e50, FUN_00588350: long callee chains, no standalone logic.
// FUN_00589350, FUN_0058b470: bare call thunks.
// FUN_0058b915: ends in a noreturn call.

/// Child list node: `next` at +4, `kind` (u16) at +8, `flags` (u16) at +0xc.
/// Links are indices into a caller-owned arena; `None` is the null pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W16a05Node {
    pub next: Option<usize>,
    pub kind: u16,
    pub flags: u16,
}

/// Record of the 0x28-stride table: flags byte at +0xe, child list head at +0x24.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W16a05Rec {
    pub flags: u8,
    pub head: Option<usize>,
}

/// 0x0057b880: pointer to 1-based index in the 0x28-stride table (0 for null).
pub fn w16a05_rec_index(ptr: i32, table_base: i32) -> i32 {
    if ptr == 0 {
        return 0;
    }
    ptr.wrapping_sub(table_base) / 0x28 + 1
}

/// 0x00588db0: pointer to 1-based index in a 16-byte-stride table (0 for null).
pub fn w16a05_index16(ptr: i32, table_base: i32) -> i32 {
    if ptr == 0 {
        return 0;
    }
    (ptr.wrapping_sub(table_base) >> 4) + 1
}

/// 0x0057d5a0: store byte at +10; if it changed, set bit 0 of the flags of
/// every node in the child list.  Returns the new byte at +10 via `field`.
pub fn w16a05_set_field_mark_children(
    field: &mut u8,
    value: u8,
    head: Option<usize>,
    nodes: &mut [W16a05Node],
) {
    if *field != value {
        *field = value;
        let mut cur = head;
        while let Some(i) = cur {
            nodes[i].flags |= 1;
            cur = nodes[i].next;
        }
    }
}

/// 0x0057d5d0: call `f(node_index)` for every node of the child list
/// (callee is the unresolved per-node routine).
pub fn w16a05_for_each_child(head: Option<usize>, nodes: &[W16a05Node], mut f: impl FnMut(usize)) {
    let mut cur = head;
    while let Some(i) = cur {
        f(i);
        cur = nodes[i].next;
    }
}

/// 0x0057dd70: find the first child with kind in 0x188..=0x194, else the
/// first with kind in 0x3be..=0x3bf; if it has flag 0x40, clear it and set 1.
pub fn w16a05_refresh_marked_child(head: Option<usize>, nodes: &mut [W16a05Node]) {
    let find = |lo: u16, hi: u16, nodes: &[W16a05Node]| {
        let mut cur = head;
        while let Some(i) = cur {
            if nodes[i].kind >= lo && nodes[i].kind <= hi {
                return Some(i);
            }
            cur = nodes[i].next;
        }
        None
    };
    let hit = find(0x188, 0x194, nodes).or_else(|| find(0x3be, 0x3bf, nodes));
    if let Some(i) = hit
        && nodes[i].flags & 0x40 != 0
    {
        nodes[i].flags = (nodes[i].flags & 0xffbf) | 1;
    }
}

/// 0x00587eb0: reset a state block (`buf` at least 0x60 bytes): byte +0x4d =
/// `value`, dwords +0x58/+0x5c = 0, byte +0x4e = 0, word +0x50 = 0xe4.
pub fn w16a05_reset_state(buf: &mut [u8], value: u8) {
    buf[0x4d] = value;
    buf[0x58..0x5c].fill(0);
    buf[0x4e] = 0;
    buf[0x5c..0x60].fill(0);
    buf[0x50..0x52].copy_from_slice(&0xe4u16.to_le_bytes());
}

/// 0x00588e00: set bit 0x20 in the flags byte (+0xe) of the owner record and
/// call the unresolved follow-up with argument 0.
pub fn w16a05_flag_owner(owner_flags: &mut u8, mut follow_up: impl FnMut(u32)) {
    *owner_flags |= 0x20;
    follow_up(0);
}

/// 0x0057cd20: walk all records; flag 8 -> `on_flag8(i)`, else flag 1 ->
/// `on_flag1(i)`.  Then if flags & 6, clear them and, if any child has kind
/// in 0x23b..=0x244, call `on_child(i)` once.
pub fn w16a05_update_records(
    recs: &mut [W16a05Rec],
    nodes: &[W16a05Node],
    mut on_flag8: impl FnMut(usize),
    mut on_flag1: impl FnMut(usize),
    mut on_child: impl FnMut(usize),
) {
    for (i, r) in recs.iter_mut().enumerate() {
        if r.flags & 8 != 0 {
            on_flag8(i);
        } else if r.flags & 1 != 0 {
            on_flag1(i);
        }
        if r.flags & 6 != 0 {
            r.flags &= 0xf9;
            let mut cur = r.head;
            while let Some(n) = cur {
                if nodes[n].kind > 0x23a && nodes[n].kind < 0x245 {
                    on_child(i);
                    break;
                }
                cur = nodes[n].next;
            }
        }
    }
}

/// 0x005888b0: copy `src[2]`, remap `src[3]` through `remap_3`, and when
/// `convert` is set remap `src[0..2]` with `remap_a` (table `0` or `tbl_a`)
/// or `remap_b` (table `tbl_b`).
#[allow(clippy::too_many_arguments)]
pub fn w16a05_remap_pair(
    src: &[u32; 4],
    dst: &mut [u32; 4],
    table: i32,
    tbl_a: i32,
    tbl_b: i32,
    convert: bool,
    remap_3: impl Fn(u32) -> u32,
    remap_a: impl Fn(u32) -> u32,
    remap_b: impl Fn(u32) -> u32,
) {
    dst[2] = src[2];
    dst[3] = remap_3(src[3]);
    if convert {
        if table == 0 || table == tbl_a {
            dst[0] = remap_a(src[0]);
            dst[1] = remap_a(src[1]);
        } else if table == tbl_b {
            dst[0] = remap_b(src[0]);
            dst[1] = remap_b(src[1]);
        }
    }
}

/// Retail free/active pool node for 0x00577260: byte +8, word +10, dwords +0xc/+0x10.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W16a05Pool {
    pub b8: u8,
    pub w10: u16,
    pub d12: u32,
    pub d16: u32,
}

/// 0x00577260: take the free-list head (None -> returns false), fill its
/// fields, move it to the active list via `relink`, bump `count`.
pub fn w16a05_pool_alloc(
    free_head: Option<&mut W16a05Pool>,
    p1: u8,
    p2: u32,
    p3: u32,
    p4: u16,
    count: &mut u32,
    mut relink: impl FnMut(),
) -> bool {
    let Some(n) = free_head else {
        return false;
    };
    n.d12 = p2;
    n.b8 = p1;
    n.d16 = p3;
    n.w10 = p4;
    relink();
    *count = count.wrapping_add(1);
    true
}

#[cfg(test)]
mod tests_wave16_agent05 {
    use super::*;

    fn chain(kinds: &[u16]) -> Vec<W16a05Node> {
        (0..kinds.len())
            .map(|i| W16a05Node {
                next: (i + 1 < kinds.len()).then_some(i + 1),
                kind: kinds[i],
                flags: 0,
            })
            .collect()
    }

    #[test]
    fn indices() {
        assert_eq!(w16a05_rec_index(0, 100), 0);
        assert_eq!(w16a05_rec_index(100 + 0x50, 100), 3);
        assert_eq!(w16a05_index16(0, 64), 0);
        assert_eq!(w16a05_index16(64 + 0x30, 64), 4);
    }

    #[test]
    fn set_field() {
        let mut n = chain(&[1, 2]);
        let mut f = 5u8;
        w16a05_set_field_mark_children(&mut f, 5, Some(0), &mut n);
        assert_eq!(n[0].flags, 0);
        w16a05_set_field_mark_children(&mut f, 6, Some(0), &mut n);
        assert_eq!((f, n[0].flags, n[1].flags), (6, 1, 1));
    }

    #[test]
    fn each_child() {
        let n = chain(&[1, 2, 3]);
        let mut v = vec![];
        w16a05_for_each_child(Some(0), &n, |i| v.push(i));
        assert_eq!(v, [0, 1, 2]);
    }

    #[test]
    fn refresh() {
        let mut n = chain(&[0x3be, 0x190]);
        n[0].flags = 0x40;
        n[1].flags = 0x40;
        w16a05_refresh_marked_child(Some(0), &mut n);
        assert_eq!((n[0].flags, n[1].flags), (0x40, 1));
        let mut n = chain(&[0x10, 0x3bf]);
        n[1].flags = 0x40;
        w16a05_refresh_marked_child(Some(0), &mut n);
        assert_eq!(n[1].flags, 1);
        let mut n = chain(&[0x10]);
        w16a05_refresh_marked_child(Some(0), &mut n);
        assert_eq!(n[0].flags, 0);
    }

    #[test]
    fn reset() {
        let mut b = [0xffu8; 0x60];
        w16a05_reset_state(&mut b, 7);
        assert_eq!((b[0x4d], b[0x4e], b[0x50], b[0x51]), (7, 0, 0xe4, 0));
        assert_eq!(&b[0x58..0x60], &[0; 8]);
    }

    #[test]
    fn owner() {
        let mut f = 1u8;
        let mut a = 9;
        w16a05_flag_owner(&mut f, |x| a = x);
        assert_eq!((f, a), (0x21, 0));
    }

    #[test]
    fn records() {
        let nodes = chain(&[0x23b]);
        let mut recs = [
            W16a05Rec {
                flags: 8,
                head: None,
            },
            W16a05Rec {
                flags: 1,
                head: None,
            },
            W16a05Rec {
                flags: 6,
                head: Some(0),
            },
            W16a05Rec {
                flags: 2,
                head: None,
            },
        ];
        let (mut a, mut b, mut c) = (vec![], vec![], vec![]);
        w16a05_update_records(
            &mut recs,
            &nodes,
            |i| a.push(i),
            |i| b.push(i),
            |i| c.push(i),
        );
        assert_eq!((a, b, c), (vec![0], vec![1], vec![2]));
        assert_eq!(recs[2].flags, 0);
        assert_eq!(recs[3].flags, 0);
    }

    #[test]
    fn remap() {
        let src = [1, 2, 3, 4];
        let mut d = [0; 4];
        w16a05_remap_pair(
            &src,
            &mut d,
            0,
            10,
            20,
            true,
            |x| x + 100,
            |x| x + 1000,
            |x| x + 2000,
        );
        assert_eq!(d, [1001, 1002, 3, 104]);
        w16a05_remap_pair(
            &src,
            &mut d,
            20,
            10,
            20,
            true,
            |x| x,
            |x| x + 1000,
            |x| x + 2000,
        );
        assert_eq!(d, [2001, 2002, 3, 4]);
        let mut d = [9; 4];
        w16a05_remap_pair(&src, &mut d, 5, 10, 20, true, |x| x, |x| x, |x| x);
        assert_eq!(d, [9, 9, 3, 4]);
    }

    #[test]
    fn pool() {
        let mut c = 0;
        let mut n = 0;
        assert!(!w16a05_pool_alloc(None, 1, 2, 3, 4, &mut c, || n += 1));
        let mut p = W16a05Pool::default();
        assert!(w16a05_pool_alloc(Some(&mut p), 1, 2, 3, 4, &mut c, || n += 1));
        assert_eq!((p.b8, p.d12, p.d16, p.w10, c, n), (1, 2, 3, 4, 1, 1));
    }
}
