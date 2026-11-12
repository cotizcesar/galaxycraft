#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent73.rs
// ===========================================================================
// Wave 31 translation batch 73: runtime helpers (range clamp, ordering, vector move-push).

/// `0x00b30cd0`: clamps a (offset, length) request against the object's
/// size `limit` (lo/hi words at +0x48/+0x4c).  Offset above the limit is
/// pulled back to the limit; a length that overshoots the limit (or is all
/// ones) becomes `limit - offset`.  A full-range request returns
/// `(0, 0, fallback)` without calling `convert`; otherwise `convert(off, len)`
/// supplies the result.
pub fn w31a73_clamp_range(
    limit: u64,
    offset: u64,
    length: u64,
    fallback: (u32, u32),
    convert: impl FnOnce(u64, u64) -> (u64, (u32, u32)),
) -> (u64, (u32, u32)) {
    let off = offset.min(limit);
    let end = off.wrapping_add(length);
    let len = if limit < end || length == u64::MAX {
        limit.wrapping_sub(off)
    } else {
        length
    };
    if off == 0 && len == limit {
        return (0, fallback);
    }
    convert(off, len)
}

/// `0x00b38900`: strict "less than" over records whose key is a
/// length-prefixed byte string (len at +0x18, bytes from +0x19) followed by a
/// 64-bit tiebreak (+0x30 lo, +0x34 hi).  Names compare bytewise, shorter
/// prefix first; equal names fall back to the 64-bit value.
pub fn w31a73_record_less(a_name: &[u8], a_tie: u64, b_name: &[u8], b_tie: u64) -> bool {
    match a_name.cmp(b_name) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => a_tie < b_tie,
    }
}

/// 48-byte element used by `0x00b3ae40` (words 3 is padding and not copied).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W31a73Elem {
    pub w: [u32; 12],
    pub flag: u8,
}

/// Where the pushed element comes from.
pub enum W31a73Src<'a> {
    /// Index of an element already inside the vector.
    Inside(usize),
    /// An outside element.
    Outside(&'a mut W31a73Elem),
}

/// `0x00b3ae40`: move-push of a 0x30-byte element.  Copies words
/// 0,1,2,4..9, the flag byte and word 11, then zeroes word 11 of the source
/// (ownership transfer).  An index inside the vector is resolved after growth.
pub fn w31a73_push_moved(v: &mut Vec<W31a73Elem>, src: W31a73Src<'_>) {
    fn mv(dst: &mut W31a73Elem, s: &mut W31a73Elem) {
        for i in [0usize, 1, 2, 4, 5, 6, 7, 8, 9, 11] {
            dst.w[i] = s.w[i];
        }
        dst.flag = s.flag;
        s.w[11] = 0;
    }
    let mut dst = W31a73Elem::default();
    match src {
        W31a73Src::Inside(i) if i < v.len() => mv(&mut dst, &mut v[i]),
        W31a73Src::Inside(_) => return,
        W31a73Src::Outside(s) => mv(&mut dst, s),
    }
    v.push(dst);
}

/// `0x00b3c1c0`: forwards to `callee(a, b, c, rec[0], rec[8], rec[10])` when
/// the record exists and its word 8 is non-zero; otherwise returns 1.
pub fn w31a73_forward_record(
    rec: Option<&[u32; 11]>,
    a: u32,
    b: u32,
    c: u32,
    callee: impl FnOnce(u32, u32, u32, u32, u32, u32) -> u32,
) -> u32 {
    match rec {
        Some(r) if r[8] != 0 => callee(a, b, c, r[0], r[8], r[10]),
        _ => 1,
    }
}

/// `0x00b2e0b0`: wrapper calling the six-argument worker with the last two
/// arguments zero.
pub fn w31a73_call_with_zero_tail<R>(
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    worker: impl FnOnce(u32, u32, u32, u32, u32, u32) -> R,
) -> R {
    worker(a, b, c, d, 0, 0)
}

#[cfg(test)]
mod tests_wave31_agent73 {
    use super::*;

    #[test]
    fn clamp() {
        let r = w31a73_clamp_range(100, 0, 100, (7, 8), |_, _| panic!());
        assert_eq!(r, (0, (7, 8)));
        let r = w31a73_clamp_range(100, 200, 5, (0, 0), |o, l| (o << 8 | l, (1, 2)));
        assert_eq!(r.0, 100 << 8);
        let r = w31a73_clamp_range(100, 10, u64::MAX, (0, 0), |o, l| (o + l, (0, 0)));
        assert_eq!(r.0, 100);
        let r = w31a73_clamp_range(100, 10, 20, (0, 0), |o, l| (o * 1000 + l, (0, 0)));
        assert_eq!(r.0, 10020);
    }

    #[test]
    fn less() {
        assert!(w31a73_record_less(b"abc", 9, b"abd", 0));
        assert!(!w31a73_record_less(b"abd", 0, b"abc", 9));
        assert!(w31a73_record_less(b"ab", 9, b"abc", 0));
        assert!(w31a73_record_less(b"x", 1, b"x", 2));
        assert!(!w31a73_record_less(b"x", 2, b"x", 2));
    }

    #[test]
    fn push() {
        let mut v = vec![W31a73Elem::default()];
        v[0].w[11] = 5;
        v[0].w[0] = 3;
        w31a73_push_moved(&mut v, W31a73Src::Inside(0));
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].w[11], 5);
        assert_eq!(v[0].w[11], 0);
        let mut e = W31a73Elem::default();
        e.w[11] = 9;
        w31a73_push_moved(&mut v, W31a73Src::Outside(&mut e));
        assert_eq!(v[2].w[11], 9);
        assert_eq!(e.w[11], 0);
    }

    #[test]
    fn forward() {
        assert_eq!(
            w31a73_forward_record(None, 1, 2, 3, |_, _, _, _, _, _| 9),
            1
        );
        let mut r = [0u32; 11];
        assert_eq!(
            w31a73_forward_record(Some(&r), 1, 2, 3, |_, _, _, _, _, _| 9),
            1
        );
        r[8] = 4;
        r[10] = 6;
        assert_eq!(
            w31a73_forward_record(Some(&r), 1, 2, 3, |a, _, _, _, e, f| a + e + f),
            11
        );
        assert_eq!(
            w31a73_call_with_zero_tail(1, 2, 3, 4, |_, _, _, _, e, f| e + f),
            0
        );
    }
}
