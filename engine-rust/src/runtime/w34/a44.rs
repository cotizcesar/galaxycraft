#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent44.rs
// ===========================================================================
// Wave 34 translation batch 44.

/// `0x009f1bc0`: canonical-Huffman symbol decode.  `peek(n)` models
/// FUN_009f0200 (returns n bits, or negative when unavailable) and
/// `consume(n)` models FUN_009f0180.  Fields: `lookup` (+0x20), `lookup_bits`
/// (+0x24), `max_bits` (+0x28), `limit` (+8), `codes` (+0x14), `lens` (+0x1c).
/// Returns the symbol or `u32::MAX`.
#[allow(clippy::too_many_arguments)]
pub fn w34a44_huff_decode(
    lookup: &[u32],
    lookup_bits: i32,
    max_bits: i32,
    limit: i32,
    codes: &[u32],
    lens: &[i8],
    peek: &mut dyn FnMut(i32) -> i32,
    consume: &mut dyn FnMut(i32),
) -> u32 {
    let mut max = max_bits;
    let (mut lo, mut hi): (u32, i32);
    let first = peek(lookup_bits);
    if first < 0 {
        lo = 0;
        hi = limit;
    } else {
        let e = lookup[first as usize];
        if (e as i32) >= 0 {
            consume(lens[(e - 1) as usize] as i32);
            return e.wrapping_sub(1);
        }
        lo = ((e as i32) >> 15 & 0x7fff) as u32;
        hi = limit.wrapping_sub((e & 0x7fff) as i32);
    }
    let mut v = peek(max);
    loop {
        if v >= 0 || max < 2 {
            if v < 0 {
                return u32::MAX;
            }
            let mut x = v as u32;
            x = x.rotate_left(16);
            x = ((x >> 8) ^ (x << 8)) & 0x00ff_00ff ^ (x << 8);
            x = ((x >> 4) ^ (x << 4)) & 0x0f0f_0f0f ^ (x << 4);
            x = ((x >> 2) ^ x.wrapping_mul(4)) & 0x3333_3333 ^ x.wrapping_mul(4);
            x = ((x >> 1) ^ x.wrapping_mul(2)) & 0x5555_5555 ^ x.wrapping_mul(2);
            let mut d = hi.wrapping_sub(lo as i32);
            while d > 1 {
                let half = (d >> 1) as u32;
                let c = codes[(half + lo) as usize];
                if x < c {
                    hi = hi.wrapping_sub(half as i32);
                } else {
                    lo = lo.wrapping_add(half);
                }
                d = hi.wrapping_sub(lo as i32);
            }
            let l = lens[lo as usize] as i32;
            if max < l {
                consume(max);
                return u32::MAX;
            }
            consume(l);
            return lo;
        }
        max -= 1;
        v = peek(max);
    }
}

/// `0x00ac04e0`: counts the leading hex digits (`0-9`, `a-f`, `A-F`) of
/// `text[..len]`; `check(text, n)` models FUN_00b8ef50 and `conv(text, n)`
/// models FUN_00b8eeb0.  When the check passes and `1 <= n <= 0x2e` (even
/// rounding) the converted byte is returned in the second slot.
pub fn w34a44_hex_prefix(
    text: &[u8],
    len: u32,
    check: &dyn Fn(&[u8], u32) -> bool,
    conv: &dyn Fn(&[u8], u32) -> u8,
) -> (u32, Option<u8>) {
    let mut n = 0u32;
    if len != 0 {
        loop {
            let c = text[n as usize];
            if !(c.is_ascii_digit() || (b'a'..=b'f').contains(&c) || (b'A'..=b'F').contains(&c)) {
                break;
            }
            n += 1;
            if n >= len {
                break;
            }
        }
    }
    if check(text, n) && n != 0 && (n & !1) < 0x2f {
        return (n, Some(conv(text, n)));
    }
    (0, None)
}

/// `0x00b0bb60`: walks 28-byte (7 x u32) records, accumulating the first
/// word into `sum_a` and the second into `sum_b` (64-bit), stopping before
/// `sum_b` would reach `limit + 1`.  Returns `(count, sum_a, sum_b)`.
pub fn w34a44_accumulate(
    limit_lo: u32,
    limit_hi: u32,
    recs: &[u32],
    count: u32,
) -> (u32, u64, u64) {
    let (mut a, mut b) = (0u64, 0u64);
    let mut n = 0u32;
    if (count > 1 || recs.first().copied().unwrap_or(0) != 0) && count != 0 {
        let hi_lim = limit_hi.wrapping_add((limit_lo == 0xffff_ffff) as u32);
        let lo_lim = limit_lo.wrapping_add(1);
        loop {
            let r = &recs[n as usize * 7..];
            let nb = b.wrapping_add(r[1] as u64);
            let (nlo, nhi) = (nb as u32, (nb >> 32) as u32);
            if hi_lim < nhi || (hi_lim <= nhi && lo_lim <= nlo) {
                return (n, a, b);
            }
            b = nb;
            a = a.wrapping_add(r[0] as u64);
            n += 1;
            if n >= count {
                break;
            }
        }
    }
    (n, a, b)
}

#[cfg(test)]
mod tests_wave34_agent44 {
    use super::*;

    #[test]
    fn hex_prefix() {
        let t = b"1aFz";
        assert_eq!(
            w34a44_hex_prefix(t, 4, &|_, _| true, &|_, n| n as u8),
            (3, Some(3))
        );
        assert_eq!(w34a44_hex_prefix(t, 4, &|_, _| false, &|_, _| 0), (0, None));
    }

    #[test]
    fn accumulate() {
        let recs = [1, 10, 0, 0, 0, 0, 0, 2, 20, 0, 0, 0, 0, 0];
        assert_eq!(w34a44_accumulate(100, 0, &recs, 2), (2, 3, 30));
        assert_eq!(w34a44_accumulate(15, 0, &recs, 2), (1, 1, 10));
    }

    #[test]
    fn huff_direct() {
        let lookup = [3u32, 0];
        let lens = [0i8, 0, 2];
        let mut used = 0;
        let r = w34a44_huff_decode(&lookup, 1, 4, 3, &[], &lens, &mut |_| 0, &mut |n| used = n);
        assert_eq!((r, used), (2, 2));
    }
}
