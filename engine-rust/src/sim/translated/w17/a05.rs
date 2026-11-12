#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave17/agent05.rs
// ===========================================================================
// Wave 17 translation batch 05: sprite-like node flag/shade helpers,
// child-list walkers and a chained hash-table lookup.

/// Node fields touched by the translated routines (offsets from the retail record).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W17a05Obj {
    /// +0x08 id.
    pub id: u16,
    /// +0x0a kind.
    pub kind: u8,
    /// +0x0b shade step.
    pub shade: u8,
    /// +0x0c flags.
    pub flags: u16,
    /// +0x0e flag byte.
    pub flags_e: u8,
    /// +0x0f byte.
    pub byte_f: u8,
    /// +0x18 base value.
    pub base: u16,
    /// +0x1a current value.
    pub cur: u16,
    /// +0x30 marker.
    pub marker: u16,
}

fn w17a05_apply_shade(o: &mut W17a05Obj, flip: bool, step: u8) {
    o.shade = step;
    if o.kind == 0x11 {
        o.marker = 0x230;
    }
    o.flags = (u16::from(flip)) << 1 | (o.flags & 0xfffd) | 1;
    let v = (u32::from(o.base) + u32::from(step)) as u16;
    if o.cur != v {
        o.cur = v;
    }
}

/// `0x005713d0`: updates shade from a 0..=0x20 angle step; `on_dirty` models
/// the call made when the flag byte has its sign bit set.
pub fn w17a05_set_shade(o: &mut W17a05Obj, p: u8, on_dirty: impl FnOnce(&mut W17a05Obj)) {
    if o.flags & 8 == 0 {
        return;
    }
    let step = if p < 0x11 { p } else { 0x20 - p };
    let flip = p > 0x10;
    if ((o.flags >> 1) & 1 == 1) != flip || o.shade != step {
        w17a05_apply_shade(o, flip, step);
        if (o.flags as u8) & 0x80 != 0 {
            on_dirty(o);
        }
    }
}

/// `0x00571460`: sets the flip bit (bit 1) when it differs from `p`.
pub fn w17a05_set_flip(o: &mut W17a05Obj, p: u8, on_dirty: impl FnOnce(&mut W17a05Obj)) {
    if ((o.flags as u8) >> 1) & 1 != p {
        if o.kind == 0x11 {
            o.marker = 0x230;
        }
        o.flags = (u16::from(p & 1) * 2) | (o.flags & 0xfffd) | 1;
        if (o.flags as u8) & 0x80 != 0 {
            on_dirty(o);
        }
    }
}

/// `0x005747e0`: like the shade setter but takes a 0..=0xff heading
/// (`(p + 4) >> 3`) and runs `on_dirty` first when the sign bit is set.
pub fn w17a05_set_shade_heading(o: &mut W17a05Obj, p: u8, on_dirty: impl FnOnce(&mut W17a05Obj)) {
    if (o.flags as u8) & 0x80 != 0 {
        on_dirty(o);
    }
    if o.flags & 8 == 0 {
        return;
    }
    let u3 = (u32::from(p) + 4) >> 3;
    let b1 = u3 as u8;
    let step = if b1 < 0x11 { u3 as u8 } else { 0x20 - b1 };
    let flip = b1 > 0x10;
    if o.shade != step || ((o.flags >> 1) & 1 == 1) != flip {
        w17a05_apply_shade(o, flip, step);
    }
}

/// `0x00572750`: loads kind from `kind_tbl[id]`, bit 4 of flags from
/// `bit_tbl[id] & 1`, then calls the three unresolved helpers in order
/// (arg is `ptr_tbl[id]`, then 0).
pub fn w17a05_init_from_tables(
    o: &mut W17a05Obj,
    kind_tbl: &[u8],
    bit_tbl: &[u8],
    ptr_tbl: &[u32],
    mut call_a: impl FnMut(u32),
    mut call_b: impl FnMut(u32),
    mut call_c: impl FnMut(),
) {
    let i = usize::from(o.id);
    o.kind = kind_tbl[i];
    if o.kind == 0x11 {
        o.marker = 0x230;
    }
    o.flags |= 1;
    o.flags = u16::from(bit_tbl[i] & 1) << 4 | (o.flags & 0xffef);
    call_a(ptr_tbl[i]);
    call_b(0);
    call_c();
}

/// `0x005749b0`: stores byte +0xf, setting dirty bit 0 on change.
pub fn w17a05_set_byte_f(o: &mut W17a05Obj, v: u8) {
    if o.byte_f != v {
        o.flags |= 1;
        o.byte_f = v;
    }
}

/// `0x00578d90`: when bit 0 of +0xe is clear and the helper returns non-zero, sets it.
pub fn w17a05_latch_flag(o: &mut W17a05Obj, helper: impl FnOnce() -> i32) {
    if o.flags_e & 1 == 0 && helper() != 0 {
        o.flags_e |= 1;
    }
}

/// `0x005794a0`: needs bit 3 of +0xe.  Scans `list_a` kinds for the first
/// kind 0x0b (calling `on_kind_b`), then, if bit 0 of +0xe is set, returns the
/// byte (+0x30) of the first `list_b` entry `(id, byte)` with id in 0x231..=0x23a
/// after calling `on_found`.  Otherwise 0.
pub fn w17a05_find_marker_byte(
    flags_e: u8,
    list_a: &[u8],
    list_b: &[(u16, u8)],
    mut on_kind_b: impl FnMut(),
    mut on_found: impl FnMut(),
) -> u8 {
    if flags_e & 8 == 0 {
        return 0;
    }
    if list_a.contains(&0x0b) {
        on_kind_b();
    }
    if flags_e & 1 != 0 {
        for &(id, b) in list_b {
            if 0x230 < id && id < 0x23b {
                on_found();
                return b;
            }
        }
    }
    0
}

/// `0x0057d680`: calls `f(p1)` once per child when `force != 0` or the sign bit of +0xe is clear.
pub fn w17a05_for_children(
    children: usize,
    flags_e: u8,
    p1: u32,
    force: i32,
    mut f: impl FnMut(u32),
) {
    for _ in 0..children {
        if force != 0 || (flags_e as i8) >= 0 {
            f(p1);
        }
    }
}

/// `0x0057e700` (selector part): the value passed to each child, or `None`
/// when the routine returns early / does nothing.
pub fn w17a05_child_value(kind: u8, id: u16) -> Option<u32> {
    match kind {
        2 | 5 => Some(8),
        3 | 6 => Some(9),
        7 => Some(0xc),
        0xd => (id == 0x1e9).then_some(0xc),
        0xf => match id {
            0xef | 0x8c | 0xc2 => Some(8),
            _ => None,
        },
        _ => None,
    }
}

/// `0x0057e700`: calls `f(value)` for every child using [`w17a05_child_value`].
pub fn w17a05_for_children_by_kind(kind: u8, id: u16, children: usize, mut f: impl FnMut(u32)) {
    if let Some(v) = w17a05_child_value(kind, id) {
        for _ in 0..children {
            f(v);
        }
    }
}

/// Hash finaliser shared by `0x0057ee40` and `0x0057f030`.
pub fn w17a05_mix(h: u32) -> u32 {
    let mut x = (h >> 16 ^ h).wrapping_mul(0x85eb_ca6b);
    x = (x >> 13 ^ x).wrapping_mul(0xc2b2_ae35);
    x >> 16 ^ x
}

/// One chained hash node: `(next, hash, value)`.
pub type W17a05Node = (Option<usize>, u32, u32);

fn w17a05_find(buckets: &[Option<usize>], nodes: &[W17a05Node], hash: u32) -> Option<usize> {
    if buckets.is_empty() {
        return None;
    }
    let b = (w17a05_mix(hash) & (buckets.len() as u32).wrapping_sub(1)) as usize;
    let mut cur = buckets[b];
    while let Some(i) = cur {
        if nodes[i].1 == hash {
            return Some(i);
        }
        cur = nodes[i].0;
    }
    None
}

/// `0x0057ee40`: looks up `hash` (already passed through the key hasher) in a
/// power-of-two bucket array; returns the node value or 0.
pub fn w17a05_hash_get(buckets: &[Option<usize>], nodes: &[W17a05Node], hash: u32) -> u32 {
    w17a05_find(buckets, nodes, hash).map_or(0, |i| nodes[i].2)
}

/// `0x0057f030`: same probe; the retail code calls non-returning helpers when
/// the table is empty or the key missing, modelled as `false`.
pub fn w17a05_hash_require(buckets: &[Option<usize>], nodes: &[W17a05Node], hash: u32) -> bool {
    w17a05_find(buckets, nodes, hash).is_some()
}

/// `0x00588d10`: element pointer `base - 0x10 + idx * 0x10`, 0 for index 0.
pub fn w17a05_elem_addr(base: u32, idx: u32) -> u32 {
    if idx == 0 {
        0
    } else {
        base.wrapping_sub(0x10).wrapping_add(idx.wrapping_mul(0x10))
    }
}

/// `0x00588e10`: true when the forwarded call returns non-zero.
pub fn w17a05_nonzero(r: i32) -> bool {
    r != 0
}

/// `0x0058b370`: copies `src[2]`, hashes `src[3]`, and when `convert` is set
/// remaps `src[1]` with `conv_a` (param_3 is 0 or `table_a`) or `conv_b`
/// (param_3 is `table_b`).  The first converter call per branch (of `src[0]`)
/// is made for its side effects and its result is discarded.
#[allow(clippy::too_many_arguments)]
pub fn w17a05_copy_record(
    src: &[u32; 4],
    dst: &mut [u32; 4],
    sel: u32,
    table_a: u32,
    table_b: u32,
    convert: bool,
    hash: impl Fn(u32) -> u32,
    mut conv_a: impl FnMut(u32) -> u32,
    mut conv_b: impl FnMut(u32) -> u32,
) {
    dst[2] = src[2];
    dst[3] = hash(src[3]);
    if convert {
        if sel == 0 || sel == table_a {
            conv_a(src[0]);
            dst[1] = conv_a(src[1]);
        } else if sel == table_b {
            conv_b(src[0]);
            dst[1] = conv_b(src[1]);
        }
    }
}

#[cfg(test)]
mod tests_wave17_agent05 {
    use super::*;

    fn obj(flags: u16) -> W17a05Obj {
        W17a05Obj {
            flags,
            base: 100,
            kind: 0x11,
            ..Default::default()
        }
    }

    #[test]
    fn shade_setters() {
        let mut o = obj(0);
        w17a05_set_shade(&mut o, 5, |_| panic!());
        assert_eq!(o.shade, 0);
        let mut o = obj(8);
        let mut dirty = false;
        w17a05_set_shade(&mut o, 0x18, |_| dirty = true);
        assert_eq!((o.shade, o.cur, o.marker), (8, 108, 0x230));
        assert_eq!(o.flags, 8 | 2 | 1);
        assert!(!dirty);
        let mut o = obj(8 | 0x80);
        w17a05_set_shade(&mut o, 3, |x| x.byte_f = 9);
        assert_eq!(o.byte_f, 9);
        let mut o = obj(8);
        w17a05_set_shade_heading(&mut o, 0xff, |_| {});
        assert_eq!(o.shade, 0); // (255+4)>>3 = 32 -> 0x20-0x20
        assert_eq!(o.flags & 2, 2);
        w17a05_set_shade_heading(&mut o, 0x40, |_| {});
        assert_eq!((o.shade, o.flags & 2), (8, 0));
    }

    #[test]
    fn flip_and_byte() {
        let mut o = obj(0);
        w17a05_set_flip(&mut o, 1, |_| {});
        assert_eq!(o.flags, 3);
        w17a05_set_flip(&mut o, 2, |_| {}); // 2 != bit -> forces update, p&1 = 0
        assert_eq!(o.flags, 1);
        w17a05_set_byte_f(&mut o, 0);
        assert_eq!(o.flags, 1);
        o.flags = 0;
        w17a05_set_byte_f(&mut o, 4);
        assert_eq!((o.flags, o.byte_f), (1, 4));
    }

    #[test]
    fn tables_and_latch() {
        let mut o = W17a05Obj {
            id: 1,
            flags: 0x10,
            ..Default::default()
        };
        let log = std::cell::RefCell::new(vec![]);
        w17a05_init_from_tables(
            &mut o,
            &[0, 0x11],
            &[0, 0],
            &[7, 9],
            |a| log.borrow_mut().push(a),
            |b| log.borrow_mut().push(b + 100),
            || {},
        );
        assert_eq!((o.kind, o.marker, o.flags), (0x11, 0x230, 1));
        assert_eq!(*log.borrow(), vec![9, 100]);
        let mut o = W17a05Obj::default();
        w17a05_latch_flag(&mut o, || 0);
        assert_eq!(o.flags_e, 0);
        w17a05_latch_flag(&mut o, || 3);
        assert_eq!(o.flags_e, 1);
        w17a05_latch_flag(&mut o, || panic!());
    }

    #[test]
    fn children() {
        assert_eq!(w17a05_find_marker_byte(0, &[0xb], &[], || {}, || {}), 0);
        let n = std::cell::Cell::new(0);
        let r = w17a05_find_marker_byte(
            9,
            &[1, 0xb],
            &[(0x230, 1), (0x231, 7), (0x23a, 8)],
            || n.set(n.get() + 1),
            || n.set(n.get() + 10),
        );
        assert_eq!((r, n.get()), (7, 11));
        assert_eq!(
            w17a05_find_marker_byte(8, &[], &[(0x231, 7)], || {}, || {}),
            0
        );
        let mut c = 0;
        w17a05_for_children(3, 0x80, 1, 0, |_| c += 1);
        assert_eq!(c, 0);
        w17a05_for_children(3, 0x80, 1, 1, |_| c += 1);
        assert_eq!(c, 3);
        assert_eq!(w17a05_child_value(2, 0), Some(8));
        assert_eq!(w17a05_child_value(6, 0), Some(9));
        assert_eq!(w17a05_child_value(0xd, 0x1e9), Some(0xc));
        assert_eq!(w17a05_child_value(0xd, 1), None);
        assert_eq!(w17a05_child_value(0xf, 0xc2), Some(8));
        assert_eq!(w17a05_child_value(0xf, 1), None);
        assert_eq!(w17a05_child_value(1, 0), None);
        let mut v = vec![];
        w17a05_for_children_by_kind(7, 0, 2, |x| v.push(x));
        assert_eq!(v, vec![0xc, 0xc]);
    }

    #[test]
    fn hash_and_misc() {
        let h = 12345u32;
        let b = (w17a05_mix(h) & 3) as usize;
        let mut buckets = vec![None; 4];
        let nodes = vec![(Some(1), 999, 1), (None, h, 77)];
        buckets[b] = Some(0);
        // node 0 may share the bucket; chain to node 1 either way
        let nodes = if buckets[b] == Some(0) {
            nodes
        } else {
            unreachable!()
        };
        assert_eq!(w17a05_hash_get(&buckets, &nodes, h), 77);
        assert_eq!(w17a05_hash_get(&buckets, &nodes, 5), 0);
        assert_eq!(w17a05_hash_get(&[], &nodes, h), 0);
        assert!(w17a05_hash_require(&buckets, &nodes, h));
        assert!(!w17a05_hash_require(&buckets, &nodes, 6));
        assert_eq!(w17a05_mix(0), 0);
        assert_eq!(w17a05_elem_addr(0x1000, 0), 0);
        assert_eq!(w17a05_elem_addr(0x1000, 2), 0x1010);
        assert!(w17a05_nonzero(-1) && !w17a05_nonzero(0));
        let mut d = [0; 4];
        w17a05_copy_record(
            &[1, 2, 3, 4],
            &mut d,
            0,
            5,
            6,
            true,
            |x| x + 1,
            |x| x * 10,
            |x| x + 1,
        );
        assert_eq!(d, [0, 20, 3, 5]);
        w17a05_copy_record(
            &[1, 2, 3, 4],
            &mut d,
            6,
            5,
            6,
            true,
            |x| x,
            |x| x,
            |x| x + 1,
        );
        assert_eq!(d[1], 3);
        w17a05_copy_record(
            &[1, 2, 3, 4],
            &mut d,
            9,
            5,
            6,
            true,
            |x| x,
            |x| x,
            |x| x + 50,
        );
        assert_eq!(d[1], 3);
    }
}
