#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent13.rs
// ===========================================================================
// Wave 28 agent 13: EBML-style reader helpers (vector accessors, vint and
// big-endian unsigned readers).

/// Reader callback: `(pos_lo, pos_hi, len, out) -> status` (negative = error,
/// positive = short read, zero = ok).
pub type W28a13Reader<'a> = dyn FnMut(u32, i32, i32, &mut u8) -> i32 + 'a;

/// `0x00a6e530`: returns the first element of the pointer vector (stride 4,
/// count at `+0x38`).  If empty, calls `refill`; a negative status is
/// returned with out = 0.  Still empty afterwards gives `(0, 0)`.
pub fn w28a13_first_element(
    items: &mut Vec<u32>,
    mut refill: impl FnMut(&mut Vec<u32>) -> i32,
) -> (i32, u32) {
    if items.is_empty() {
        let r = refill(items);
        if r < 0 {
            return (r, 0);
        }
        if items.is_empty() {
            return (0, 0);
        }
    }
    (0, items[0])
}

/// `0x00a6e790`: returns element `cur_index + 1`; when out of range calls
/// `refill` (negative -> error with out 0, positive -> `(0, 0)`).  A still
/// out-of-range index after a zero status yields 0 (retail reads raw memory).
pub fn w28a13_next_element(
    items: &mut Vec<u32>,
    cur_index: u32,
    mut refill: impl FnMut(&mut Vec<u32>) -> i32,
) -> (i32, u32) {
    let idx = cur_index.wrapping_add(1);
    if items.len() as u32 <= idx {
        let r = refill(items);
        if r < 0 {
            return (r, 0);
        }
        if r > 0 {
            return (0, 0);
        }
    }
    (0, items.get(idx as usize).copied().unwrap_or(0))
}

/// `0x00a6eb30`: base (the i16 at `+0x18`, sign-extended) plus an optional
/// 64-bit offset; `-1` on a missing/negative offset or signed overflow.
/// `offset` is `None` when the argument is absent (returns the base).
pub fn w28a13_offset_sum(
    base: i16,
    has_arg: bool,
    mut lookup: impl FnMut() -> Result<i64, i32>,
) -> i64 {
    let base = base as i64;
    if !has_arg {
        return base;
    }
    let off = lookup().unwrap_or(-1);
    if off >= 0 {
        base.checked_add(off).unwrap_or(-1)
    } else {
        -1
    }
}

/// `0x00a6f380`: resets an 14-word state block: pairs (1,0), (1,0), (4,0),
/// (8,0) at words 0..8, words 10..14 = (1,0,1,0).  Returns true when word 8
/// (an owned pointer) was non-zero, i.e. retail frees it (never returns).
pub fn w28a13_reset_sizes(w: &mut [u32; 14]) -> bool {
    w[0] = 1;
    w[1] = 0;
    w[2] = 1;
    w[3] = 0;
    w[4] = 4;
    w[5] = 0;
    w[6] = 8;
    w[7] = 0;
    let owned = w[8] != 0;
    w[10] = 1;
    w[11] = 0;
    w[12] = 1;
    w[13] = 0;
    owned
}

/// `0x00a76170`: reads an EBML variable-length integer at `(pos_lo, pos_hi)`.
/// Returns the value (marker bit stripped) and stores the byte length in
/// `len`.  `-2` for bad args/zero first byte, `-3` for a short read, or the
/// reader's negative status.
pub fn w28a13_read_vint(
    read: &mut W28a13Reader,
    mut pos_lo: u32,
    mut pos_hi: i32,
    len: &mut i32,
) -> i64 {
    if pos_hi < 0 {
        return -2;
    }
    *len = 1;
    let mut b = 0u8;
    let r = read(pos_lo, pos_hi, 1, &mut b);
    if r < 0 {
        return r as i64;
    }
    if r > 0 {
        return -3;
    }
    if b == 0 {
        return -2;
    }
    let mut mask = 0x80u8;
    let mut n = *len;
    if (b as i8) >= 0 {
        loop {
            mask >>= 1;
            n = *len + 1;
            *len = n;
            if b & mask != 0 {
                break;
            }
        }
    }
    let mut hi = 0u32;
    let mut lo = !(mask as u32) & b as u32;
    let mut i = 1;
    pos_hi += (0xffff_fffe < pos_lo) as i32;
    if n > 1 {
        loop {
            let np = pos_lo.wrapping_add(1);
            let mut nb = 0u8;
            let r = read(np, pos_hi, 1, &mut nb);
            if r < 0 {
                *len = 1;
                return r as i64;
            }
            if r > 0 {
                *len = 1;
                return -3;
            }
            hi = hi << 8 | lo >> 24;
            lo = lo << 8 | nb as u32;
            pos_hi += (0xffff_fffe < np) as i32;
            i += 1;
            pos_lo = np;
            if i >= *len {
                break;
            }
        }
    }
    ((hi as u64) << 32 | lo as u64) as i64
}

/// `0x00a766d0`: reads a big-endian unsigned integer of `size` bytes (1..=8,
/// `size_hi` must be 0) at `(pos_lo, pos_hi)`.  Returns `Err(status)`
/// (`-2` bad args, or the reader status) or `Ok(value)`.
pub fn w28a13_read_uint_be(
    read: &mut W28a13Reader,
    pos_lo: u32,
    pos_hi: i32,
    size: u32,
    size_hi: i32,
) -> Result<u64, i32> {
    if pos_hi < 0 || size_hi != 0 || size == 0 || size > 8 {
        return Err(-2);
    }
    let mut b = 0u8;
    let r = read(pos_lo, pos_hi, 1, &mut b);
    if r < 0 {
        return Err(r);
    }
    let mut v = b as u64;
    let mut p = pos_lo.wrapping_add(1);
    let mut h = pos_hi + (0xffff_fffe < pos_lo) as i32;
    let mut i = 1;
    while i < size {
        let r = read(p, h, 1, &mut b);
        if r < 0 {
            return Err(r);
        }
        v = v << 8 | b as u64;
        h += (0xffff_fffe < p) as i32;
        p = p.wrapping_add(1);
        i += 1;
    }
    Ok(v)
}

#[cfg(test)]
mod tests_wave28_agent13 {
    use super::*;

    fn rd(data: Vec<u8>) -> impl FnMut(u32, i32, i32, &mut u8) -> i32 {
        move |lo, _hi, _n, out| match data.get(lo as usize) {
            Some(b) => {
                *out = *b;
                0
            }
            None => -4,
        }
    }

    #[test]
    fn element_accessors() {
        let mut v = vec![];
        assert_eq!(w28a13_first_element(&mut v, |_| -5), (-5, 0));
        assert_eq!(
            w28a13_first_element(&mut v, |x| {
                x.push(9);
                0
            }),
            (0, 9)
        );
        let mut v = vec![1, 2];
        assert_eq!(w28a13_next_element(&mut v, 0, |_| -1), (0, 2));
        assert_eq!(w28a13_next_element(&mut v, 1, |_| -1), (-1, 0));
        assert_eq!(w28a13_next_element(&mut v, 1, |_| 1), (0, 0));
    }

    #[test]
    fn offset_sum() {
        assert_eq!(w28a13_offset_sum(-3, false, || Ok(1)), -3);
        assert_eq!(w28a13_offset_sum(5, true, || Ok(10)), 15);
        assert_eq!(w28a13_offset_sum(5, true, || Err(-1)), -1);
        assert_eq!(w28a13_offset_sum(5, true, || Ok(i64::MAX)), -1);
    }

    #[test]
    fn reset() {
        let mut w = [7u32; 14];
        assert!(w28a13_reset_sizes(&mut w));
        assert_eq!(&w[..8], &[1, 0, 1, 0, 4, 0, 8, 0]);
        assert_eq!(&w[10..], &[1, 0, 1, 0]);
        w[8] = 0;
        assert!(!w28a13_reset_sizes(&mut w));
    }

    #[test]
    fn vint() {
        let mut len = 0;
        assert_eq!(w28a13_read_vint(&mut rd(vec![0x81]), 0, 0, &mut len), 1);
        assert_eq!(len, 1);
        assert_eq!(
            w28a13_read_vint(&mut rd(vec![0x41, 0x02]), 0, 0, &mut len),
            0x102
        );
        assert_eq!(len, 2);
        assert_eq!(w28a13_read_vint(&mut rd(vec![0x00]), 0, 0, &mut len), -2);
        assert_eq!(w28a13_read_vint(&mut rd(vec![0x41]), 0, 0, &mut len), -4);
        assert_eq!(w28a13_read_vint(&mut |_, _, _, _| 1, 0, 0, &mut len), -3);
        assert_eq!(w28a13_read_vint(&mut rd(vec![0x81]), 0, -1, &mut len), -2);
    }

    #[test]
    fn uint_be() {
        assert_eq!(
            w28a13_read_uint_be(&mut rd(vec![1, 2, 3]), 0, 0, 3, 0),
            Ok(0x010203)
        );
        assert_eq!(w28a13_read_uint_be(&mut rd(vec![7]), 0, 0, 1, 0), Ok(7));
        assert_eq!(w28a13_read_uint_be(&mut rd(vec![7]), 0, 0, 9, 0), Err(-2));
        assert_eq!(w28a13_read_uint_be(&mut rd(vec![7]), 0, 0, 0, 0), Err(-2));
        assert_eq!(w28a13_read_uint_be(&mut rd(vec![7]), 0, 0, 2, 0), Err(-4));
    }
}
