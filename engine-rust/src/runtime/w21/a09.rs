#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent09.rs
// ===========================================================================
// Wave 21 translation batch 09.
//
// Window: queue: 20 addresses, module `runtime.rs`.

/// `0x0057b9f0`: walks the child list of a node.  When flag `0x40` of the
/// byte at `+0xe` is clear, each child whose kind is in `5..=7` calls
/// `on_kind(kind - 3)`.  Otherwise `pre()` runs first and every child with
/// kind 0, 2 or 4 calls `on_kind(3)`.
pub fn w21a09_notify_children(
    flags: u8,
    kinds: &[u8],
    pre: &mut dyn FnMut(),
    on_kind: &mut dyn FnMut(u8),
) {
    if flags & 0x40 == 0 {
        for &k in kinds {
            if k.wrapping_sub(5) < 3 {
                on_kind(k.wrapping_sub(3));
            }
        }
    } else {
        pre();
        for &k in kinds {
            if k == 0 || k == 2 || k == 4 {
                on_kind(3);
            }
        }
    }
}

/// `0x0057cc80`: if any of flag bits `0x06` is set in the byte at `+0xe`,
/// clears them and returns `refresh()`; otherwise returns 0.
pub fn w21a09_take_dirty_bits(flags: &mut u8, refresh: &mut dyn FnMut() -> u8) -> u8 {
    if *flags & 6 == 0 {
        return 0;
    }
    *flags &= 0xf9;
    refresh()
}

/// `0x0057d600`: for each child (kind at `+10`, value at `+0x30`) whose kind
/// is 0 or 0x10, calls `on_kind(0x10)` and stores `value` in the child.
pub fn w21a09_set_children_value(
    children: &mut [(u8, u32)],
    value: u8,
    on_kind: &mut dyn FnMut(u8),
) {
    for c in children.iter_mut() {
        if c.0 == 0 || c.0 == 0x10 {
            on_kind(0x10);
            c.1 = value as u32;
        }
    }
}

/// `0x0057e7b0`: when the current child's kind (`+10`) is `\n` (10), clears
/// bit `0x4` of its `u16` flags at `+0xc`.
pub fn w21a09_clear_flag4_if_newline(kind: u8, flags: &mut u16) {
    if kind == 10 {
        *flags &= 0xfffb;
    }
}

/// `0x0057ea70`: resizes a vector of 0x28-byte elements.  Shrinking just
/// lowers the length; growing appends `make()` elements (capacity growth
/// and construction are the allocator's job in the retail build).
pub fn w21a09_resize_vec<T>(v: &mut Vec<T>, n: usize, make: &mut dyn FnMut() -> T) {
    if n < v.len() {
        v.truncate(n);
    } else {
        while v.len() < n {
            v.push(make());
        }
    }
}

/// `0x0057f140`: inserts `count` 8-byte slots at index `pos` of a vector
/// of `[u32; 2]`, shifting the tail up.  New slots are zeroed (the caller
/// fills them).  Returns `(begin, end)` indices of the opened gap.
pub fn w21a09_insert_gap(v: &mut Vec<[u32; 2]>, pos: usize, count: usize) -> (usize, usize) {
    let pos = pos.min(v.len());
    for _ in 0..count {
        v.insert(pos, [0, 0]);
    }
    (pos, pos + count)
}

/// `0x00587f10`: initialises fields of a text/line record: `+0x4d = a`,
/// `+0x58 = b`, `+0x5a = c`, `+0x5c = 0`, `+0x4e = 0`, `+0x50 = 0xe4`.
pub fn w21a09_init_line_fields(rec: &mut [u8], a: u8, b: u16, c: u16) {
    rec[0x4d] = a;
    rec[0x58..0x5a].copy_from_slice(&b.to_le_bytes());
    rec[0x5a..0x5c].copy_from_slice(&c.to_le_bytes());
    rec[0x5c..0x60].copy_from_slice(&0u32.to_le_bytes());
    rec[0x4e] = 0;
    rec[0x50..0x52].copy_from_slice(&0xe4u16.to_le_bytes());
}

/// `0x00587f50`: stores `a` at `+0x4d` and `ptr` at `+0x5c`; a non-null
/// `ptr` hands off to `adopt()` (retail does not return), otherwise
/// `+0x58`/`+0x5a` = 0, `+0x4e` = 0, `+0x50` = 0xe4.  Returns whether the
/// zero-defaults path ran.
pub fn w21a09_init_line_ptr(rec: &mut [u8], a: u8, ptr: u32, adopt: &mut dyn FnMut()) -> bool {
    rec[0x4d] = a;
    rec[0x5c..0x60].copy_from_slice(&ptr.to_le_bytes());
    if ptr != 0 {
        adopt();
        return false;
    }
    rec[0x58..0x5c].copy_from_slice(&[0; 4]);
    rec[0x4e] = 0;
    rec[0x50..0x52].copy_from_slice(&0xe4u16.to_le_bytes());
    true
}

/// `0x00588480`: calls `release` on each of its two arguments in order.
pub fn w21a09_release_pair(a: u32, b: u32, release: &mut dyn FnMut(u32)) {
    release(a);
    release(b);
}

#[cfg(test)]
mod tests_wave21_agent09 {
    use super::*;

    #[test]
    fn notify_children_both_modes() {
        let mut got = vec![];
        let mut pre = 0;
        w21a09_notify_children(0, &[4, 5, 6, 7, 8], &mut || pre += 1, &mut |k| got.push(k));
        assert_eq!(got, vec![2, 3, 4]);
        assert_eq!(pre, 0);
        got.clear();
        w21a09_notify_children(0x40, &[0, 1, 2, 3, 4], &mut || pre += 1, &mut |k| {
            got.push(k)
        });
        assert_eq!(got, vec![3, 3, 3]);
        assert_eq!(pre, 1);
    }

    #[test]
    fn dirty_bits() {
        let mut f = 0xffu8;
        assert_eq!(w21a09_take_dirty_bits(&mut f, &mut || 9), 9);
        assert_eq!(f, 0xf9);
        let mut f = 0x01u8;
        assert_eq!(w21a09_take_dirty_bits(&mut f, &mut || 9), 0);
        assert_eq!(f, 1);
    }

    #[test]
    fn children_value() {
        let mut c = [(0u8, 1u32), (3, 2), (0x10, 3)];
        let mut n = 0;
        w21a09_set_children_value(&mut c, 7, &mut |k| {
            assert_eq!(k, 0x10);
            n += 1
        });
        assert_eq!(n, 2);
        assert_eq!(c, [(0, 7), (3, 2), (0x10, 7)]);
    }

    #[test]
    fn newline_flag() {
        let mut f = 0xffffu16;
        w21a09_clear_flag4_if_newline(10, &mut f);
        assert_eq!(f, 0xfffb);
        let mut f = 0xffffu16;
        w21a09_clear_flag4_if_newline(9, &mut f);
        assert_eq!(f, 0xffff);
    }

    #[test]
    fn resize_and_gap() {
        let mut v = vec![1, 2, 3];
        w21a09_resize_vec(&mut v, 5, &mut || 0);
        assert_eq!(v, [1, 2, 3, 0, 0]);
        w21a09_resize_vec(&mut v, 2, &mut || 0);
        assert_eq!(v, [1, 2]);
        let mut g = vec![[1, 1], [2, 2]];
        assert_eq!(w21a09_insert_gap(&mut g, 1, 2), (1, 3));
        assert_eq!(g, [[1, 1], [0, 0], [0, 0], [2, 2]]);
    }

    #[test]
    fn line_fields() {
        let mut r = [0xffu8; 0x60];
        w21a09_init_line_fields(&mut r, 3, 0x1234, 0x5678);
        assert_eq!(r[0x4d], 3);
        assert_eq!(&r[0x58..0x5c], &[0x34, 0x12, 0x78, 0x56]);
        assert_eq!(&r[0x5c..0x60], &[0; 4]);
        assert_eq!((r[0x4e], r[0x50], r[0x51]), (0, 0xe4, 0));
        let mut called = 0;
        assert!(w21a09_init_line_ptr(&mut r, 1, 0, &mut || called += 1));
        assert_eq!(&r[0x58..0x5c], &[0; 4]);
        assert!(!w21a09_init_line_ptr(&mut r, 2, 5, &mut || called += 1));
        assert_eq!(called, 1);
        assert_eq!(r[0x5c], 5);
    }

    #[test]
    fn release_pair_order() {
        let mut v = vec![];
        w21a09_release_pair(1, 2, &mut |x| v.push(x));
        assert_eq!(v, [1, 2]);
    }
}
