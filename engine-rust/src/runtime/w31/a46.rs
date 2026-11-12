#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent46.rs
// ===========================================================================
// Wave 31 translation batch 46: a few list/forwarding helpers from the
// 0x0089xxxx range.  Unresolved callees are closures.

/// Sentinel list header: `[0]` first link, `[1]` last link, `[2]` count.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct W31a46ListHdr {
    pub first: u32,
    pub last: u32,
    pub count: u32,
}

/// `0x0089aa10`: move-assign a sentinel list from `src` into `dst`.
/// `dst_addr`/`src_addr` are the headers' own addresses (the empty-list
/// sentinel value).  Links are swapped, the count is moved, and when the
/// moved list is non-empty the boundary nodes' back references are
/// rewritten through `write` (`addr, value`): `*last = dst` at the old
/// last-link and `*(first + 4) = dst`.  `src` is left empty.
pub fn w31a46_list_move_assign(
    dst: &mut W31a46ListHdr,
    dst_addr: u32,
    src: &mut W31a46ListHdr,
    src_addr: u32,
    mut write: impl FnMut(u32, u32),
) {
    let (old_first, old_last) = (dst.first, dst.last);
    dst.first = src.first;
    dst.last = src.last;
    src.first = old_first;
    src.last = old_last;
    dst.count = src.count;
    src.count = 0;
    if dst.count == 0 {
        dst.last = dst_addr;
        dst.first = dst_addr;
    } else {
        write(dst.last, dst_addr);
        write(dst.first.wrapping_add(4), dst_addr);
    }
    src.last = src_addr;
    src.first = src_addr;
}

/// `0x008975b0`: calls `reset(inner, 0)` then stores the sentinel
/// `0xfffffffe` into the field at `+0xb4`.
pub fn w31a46_reset_inner(inner: u32, field_b4: &mut u32, mut reset: impl FnMut(u32, u32)) {
    reset(inner, 0);
    *field_b4 = 0xffff_fffe;
}

/// `0x00897780`: forwards `arg` with the inner handle (`+4`).
pub fn w31a46_forward_a(inner: u32, arg: u32, mut f: impl FnMut(u32, u32)) {
    f(inner, arg);
}

/// `0x00897c70`: forwards `arg` with the inner handle (`+4`).
pub fn w31a46_forward_b(inner: u32, arg: u32, mut f: impl FnMut(u32, u32)) {
    f(inner, arg);
}

/// `0x00895040`: when `index` is -1 invoke vtable slot `0xc4` with no
/// argument, otherwise invoke slot `0xd4` with the looked-up value.
pub fn w31a46_dispatch(
    index: i32,
    lookup: impl FnOnce(i32) -> u32,
    mut slot_c4: impl FnMut(),
    mut slot_d4: impl FnMut(u32),
) {
    if index != -1 {
        slot_d4(lookup(index));
    } else {
        slot_c4();
    }
}

#[cfg(test)]
mod tests_wave31_agent46 {
    use super::*;

    #[test]
    fn move_empty() {
        let mut d = W31a46ListHdr {
            first: 1,
            last: 2,
            count: 5,
        };
        let mut s = W31a46ListHdr::default();
        w31a46_list_move_assign(&mut d, 100, &mut s, 200, |_, _| panic!());
        assert_eq!(
            d,
            W31a46ListHdr {
                first: 100,
                last: 100,
                count: 0
            }
        );
        assert_eq!(
            s,
            W31a46ListHdr {
                first: 200,
                last: 200,
                count: 0
            }
        );
    }

    #[test]
    fn move_nonempty() {
        let mut d = W31a46ListHdr::default();
        let mut s = W31a46ListHdr {
            first: 10,
            last: 20,
            count: 2,
        };
        let mut w = vec![];
        w31a46_list_move_assign(&mut d, 100, &mut s, 200, |a, v| w.push((a, v)));
        assert_eq!(w, vec![(20, 100), (14, 100)]);
        assert_eq!(d.count, 2);
        assert_eq!(s.first, 200);
    }

    #[test]
    fn forwards() {
        let mut f = 0;
        w31a46_reset_inner(7, &mut f, |i, a| assert_eq!((i, a), (7, 0)));
        assert_eq!(f, 0xffff_fffe);
        let mut got = vec![];
        w31a46_forward_a(1, 2, |a, b| got.push((a, b)));
        w31a46_forward_b(3, 4, |a, b| got.push((a, b)));
        assert_eq!(got, vec![(1, 2), (3, 4)]);
        let r = std::cell::Cell::new(0);
        w31a46_dispatch(-1, |_| 0, || r.set(1), |_| r.set(2));
        assert_eq!(r.get(), 1);
        w31a46_dispatch(3, |i| i as u32 + 1, || r.set(1), |v| r.set(v));
        assert_eq!(r.get(), 4);
    }
}
