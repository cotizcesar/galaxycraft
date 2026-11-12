#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent01.rs
// ===========================================================================
// Wave 27 translation batch 01: small UI-control and handle helpers.

/// Control record fields touched by `0x0097a020` (offsets noted).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W27a01Ctl {
    /// +0x30 flags (bit 31 = busy/lock).
    pub flags: u32,
    /// +0x3c kind (0xd = scroll bar variant).
    pub kind: i16,
    /// +0x06 low edge.
    pub lo: i16,
    /// +0x0a high edge.
    pub hi: i16,
    /// +0x50 non-zero child pointer.
    pub child: u32,
    /// +0x60.
    pub b60: u8,
    /// +0x62.
    pub b62: u8,
    /// +0x63 stride.
    pub stride: u8,
    /// +0x64 row count.
    pub rows: u8,
    /// +0x65.
    pub b65: u8,
    /// +0x66.
    pub b66: u8,
    /// +0x67.
    pub b67: u8,
    /// +0x68.
    pub b68: u8,
}

/// `0x0097a020`: recomputes row count and clamps for a control.  `delta`
/// is the register-passed adjustment (`local_8`), `parent_hi` the parent's
/// +0x0a edge; `on_kind_d` is FUN_009650f0 and `on_child` FUN_0097a450.
pub fn w27a01_relayout(
    c: &mut W27a01Ctl,
    delta: i16,
    parent_hi: i16,
    mut on_kind_d: impl FnMut(&mut W27a01Ctl),
    mut on_child: impl FnMut(&mut W27a01Ctl),
) {
    if (c.flags as i32) < 0 {
        return;
    }
    if c.kind == 0xd {
        if (parent_hi as i32) < c.hi as i32 + delta as i32 {
            c.lo = c.lo.wrapping_sub(delta);
            c.b66 |= 1;
        } else {
            c.b66 = 0;
            c.hi = delta.wrapping_add(c.hi);
        }
        on_kind_d(c);
    }
    let stride = c.stride as i32;
    if stride == 0 {
        return;
    }
    let span = c.hi.wrapping_sub(c.lo).wrapping_add(1) as i32;
    let q = (span / stride) as u32;
    let prod = (c.stride as u16).wrapping_mul(q as u16);
    c.rows = q as u8;
    let mut size = prod as u32;
    if (prod as i16) as i32 <= stride {
        size = stride as u32;
    }
    c.hi = (size as i16).wrapping_add(c.lo);
    let b60 = c.b60;
    let b3;
    if b60 == 0 {
        b3 = 0xff;
        c.b62 = 0xff;
    } else {
        let mut v = c.b62;
        if b60 <= v && v != 0xff {
            v = b60 - 1;
            c.b62 = v;
        }
        b3 = v;
    }
    c.b68 = b3;
    let diff = b60 as i32 - (q & 0xff) as i32;
    let b4 = if diff >= 0 { diff as u8 } else { 0 };
    c.b65 = b4;
    if b4 < c.b67 {
        c.b67 = b4;
    }
    c.flags |= 0x8000_0000;
    if c.child != 0 {
        on_child(c);
    }
    if c.kind == 0xd {
        if c.b66 & 1 == 0 {
            c.hi = c.hi.wrapping_sub(delta);
        } else {
            c.lo = c.lo.wrapping_add(delta);
        }
    }
    c.flags &= 0x7fff_ffff;
}

/// `0x0097a340`: sets or releases the busy bit; releasing re-runs relayout.
pub fn w27a01_set_lock(c: &mut W27a01Ctl, release: bool, relayout: impl FnOnce(&mut W27a01Ctl)) {
    if !release {
        c.flags |= 0x8000_0000;
    } else if (c.flags as i32) < 0 {
        c.flags &= 0x7fff_ffff;
        relayout(c);
    }
}

/// `0x009738c0`: top element of a stack (last of `data`), or 0 when empty.
pub fn w27a01_stack_top(data: &[u32]) -> u32 {
    data.last().copied().unwrap_or(0)
}

/// `0x00963ca0`: after looking an entry up, clears the "selected" bit (bit 2)
/// of the old selection when not found, otherwise activates an entry whose
/// bit 2 is clear.  `flags` are the +0x34 words, `selected` the +0x60 link.
pub fn w27a01_update_selection(
    flags: &mut [u32],
    selected: &mut Option<usize>,
    found: Option<usize>,
    mut activate: impl FnMut(usize),
) {
    match found {
        None => {
            if let Some(s) = selected.take() {
                flags[s] &= !4;
            }
        }
        Some(i) => {
            if (flags[i] >> 2) & 1 == 0 {
                activate(i);
            }
        }
    }
}

/// `0x00999640`: resolves a packed handle (bits 28..31 table, 16..27 slot,
/// 0..15 generation) to a slot index.  `table(idx)` is FUN_00990450 and yields
/// `Ok((slot_count, Some(ids)))`; `ids[slot]` is the dword at +0x8c.
pub fn w27a01_resolve_handle(
    handle: u32,
    table: impl Fn(u32) -> Result<(usize, Option<Vec<u32>>), i32>,
    out: Option<&mut Option<usize>>,
) -> i32 {
    let slot = ((handle >> 16) & 0xfff) as usize;
    let gen_ = handle & 0xffff;
    let Some(out) = out else { return 0x25 };
    *out = None;
    if gen_ != 0
        && let Ok((count, ids)) = table(handle >> 28)
    {
        let Some(ids) = ids else { return 0x4f };
        if slot < count {
            let id = ids.get(slot).copied().unwrap_or(0);
            if gen_ == 0xffff || id == handle {
                *out = Some(slot);
                return 0;
            }
            if 1 < (id & 0xffff).wrapping_sub(gen_) {
                return 0xb;
            }
        }
    }
    0x24
}

#[cfg(test)]
mod tests_wave27_agent01 {
    use super::*;

    #[test]
    fn stack_top() {
        assert_eq!(w27a01_stack_top(&[]), 0);
        assert_eq!(w27a01_stack_top(&[1, 2, 9]), 9);
    }

    #[test]
    fn selection() {
        let mut f = [4u32, 0];
        let mut sel = Some(0);
        w27a01_update_selection(&mut f, &mut sel, None, |_| panic!());
        assert_eq!((f[0], sel), (0, None));
        let mut hit = None;
        w27a01_update_selection(&mut f, &mut sel, Some(1), |i| hit = Some(i));
        assert_eq!(hit, Some(1));
        f[1] = 4;
        w27a01_update_selection(&mut f, &mut sel, Some(1), |_| panic!());
    }

    #[test]
    fn lock_and_relayout() {
        let mut c = W27a01Ctl::default();
        w27a01_set_lock(&mut c, false, |_| panic!());
        assert_eq!(c.flags, 0x8000_0000);
        let mut called = false;
        w27a01_set_lock(&mut c, true, |_| called = true);
        assert!(called && c.flags == 0);
        c = W27a01Ctl {
            stride: 10,
            lo: 0,
            hi: 34,
            b60: 5,
            b62: 9,
            b67: 9,
            ..Default::default()
        };
        w27a01_relayout(&mut c, 0, 0, |_| {}, |_| {});
        assert_eq!(
            (c.rows, c.hi, c.b62, c.b68, c.b65, c.b67),
            (3, 30, 4, 4, 2, 2)
        );
        let mut l = W27a01Ctl {
            flags: 0x8000_0000,
            ..Default::default()
        };
        w27a01_relayout(&mut l, 0, 0, |_| {}, |_| {});
        assert_eq!(l.flags, 0x8000_0000);
    }

    #[test]
    fn handle() {
        let h = 0x1005_0003u32;
        let t = |_| Ok((8usize, Some(vec![0, 0, 0, 0, 0, 0x1005_0003, 0, 0])));
        let mut o = None;
        assert_eq!(w27a01_resolve_handle(h, t, Some(&mut o)), 0);
        assert_eq!(o, Some(5));
        assert_eq!(w27a01_resolve_handle(h, t, None), 0x25);
        assert_eq!(w27a01_resolve_handle(h & !0xffff, t, Some(&mut o)), 0x24);
        let t2 = |_| Ok((8usize, None));
        assert_eq!(w27a01_resolve_handle(h, t2, Some(&mut o)), 0x4f);
        let t3 = |_| Ok((8usize, Some(vec![0, 0, 0, 0, 0, 0x1005_0009, 0, 0])));
        assert_eq!(w27a01_resolve_handle(h, t3, Some(&mut o)), 0xb);
    }
}
