#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent11.rs
// ===========================================================================
// Wave 26 agent 11: little-endian 32-bit-word big-integer vector helpers
// (record `{len, ptr, cap, offset}`, stride 0x10) from the runtime image.

/// Writes `val` at word `idx`, growing with zero fill (the checked-write
/// idiom that precedes every store in the retail bodies).
pub fn w26a11_set_word(out: &mut Vec<u32>, idx: usize, val: u32) {
    if out.len() <= idx {
        out.resize(idx + 1, 0);
    }
    out[idx] = val;
}

fn w26a11_get(v: &[u32], i: usize) -> u32 {
    v.get(i).copied().unwrap_or(0)
}

/// `0x0092e6f0`: sets the vector to the single word `1`.
pub fn w26a11_set_one() -> Vec<u32> {
    vec![1]
}

/// `0x0092e210`: multiplies `src` by the 64-bit scalar `lo + (hi << 32)`
/// (`param_3`, `param_4`), emitting words until the carry is exhausted.
pub fn w26a11_mul_scalar(src: &[u32], lo: u32, hi: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut carry: u32 = 0;
    let mut i = 0usize;
    loop {
        if carry == 0 && i >= src.len() {
            return out;
        }
        let w = w26a11_get(src, i);
        let v = (lo as u64)
            .wrapping_mul(w as u64)
            .wrapping_add(((hi.wrapping_mul(w)) as u64) << 32 | carry as u64);
        carry = (v >> 32) as u32;
        w26a11_set_word(&mut out, i, v as u32);
        i += 1;
    }
}

/// `0x0092e770`: shift left by `bits`; result has `len + 1 + bits / 32` words.
pub fn w26a11_shl(src: &[u32], bits: u32) -> Vec<u32> {
    let ws = (bits >> 5) as usize;
    let s = bits & 0x1f;
    let total = src.len() + 1 + ws;
    let mut out = vec![0u32; total];
    for i in (0..total).rev() {
        let mut v = 0u32;
        if i >= ws {
            v = w26a11_get(src, i - ws) << s;
            if i > ws && s != 0 {
                v = v.wrapping_add(w26a11_get(src, i - ws - 1) >> (32 - s));
            }
        }
        out[i] = v;
    }
    out
}

/// `0x0092ec00`: word-wise `a - b` with a signed running borrow; result has
/// `max(len a, len b)` words.
pub fn w26a11_sub(a: &[u32], b: &[u32]) -> Vec<u32> {
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n);
    let mut lo: u32 = 0;
    let mut hi: u32 = 0;
    for i in 0..n {
        let x = w26a11_get(a, i);
        let y = w26a11_get(b, i);
        let d = x.wrapping_sub(y);
        let t = lo.wrapping_add(d);
        let new_lo = hi
            .wrapping_sub((x < y) as u32)
            .wrapping_add(lo.checked_add(d).is_none() as u32);
        out.push(t);
        lo = new_lo;
        hi = if lo > 0x7fff_ffff { 0xffff_ffff } else { 0 };
    }
    out
}

/// `0x0092dfe0`: schoolbook product `a * b`; `a.len() + b.len()` words when
/// both are non-empty.
pub fn w26a11_mul(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    for (i, &av) in a.iter().enumerate() {
        let mut carry: u32 = 0;
        let mut j = 0usize;
        for (jj, &bv) in b.iter().enumerate() {
            let prod = bv as u64 * av as u64;
            let o = w26a11_get(&out, i + jj);
            let lo_sum = (prod as u32).wrapping_add(o);
            let c1 = (prod as u32).checked_add(o).is_none() as u32;
            let nv = carry.wrapping_add(lo_sum);
            let c2 = carry.checked_add(lo_sum).is_none() as u32;
            w26a11_set_word(&mut out, i + jj, nv);
            carry = ((prod >> 32) as u32).wrapping_add(c1).wrapping_add(c2);
            j = jj + 1;
        }
        w26a11_set_word(&mut out, i + j, carry);
    }
    out
}

/// `0x0092e9a0`: square of `a` (cross terms doubled), `2 * len` words.
pub fn w26a11_square(a: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    for i in 0..a.len() {
        let mut carry: u64 = 0;
        for j in 0..=i {
            let prod = a[j] as u64 * a[i] as u64;
            let o = w26a11_get(&out, i + j) as u64;
            let sum = prod + o;
            let add = if j < i {
                prod.wrapping_add(carry)
            } else {
                carry
            };
            let total = sum.wrapping_add(add);
            let ov = (total < sum) as u64;
            carry = (total >> 32) | (ov << 32);
            w26a11_set_word(&mut out, i + j, total as u32);
        }
        w26a11_set_word(&mut out, 2 * i + 1, carry as u32);
    }
    out
}

/// `0x0092e620`: capacity growth for a word vector.  `add` extra words are
/// requested over `len`; `grow` selects the policy hook (`policy()` gives
/// the retail suggested size, never below the request).  Returns the new
/// capacity, or `None` if no reallocation is needed.
pub fn w26a11_reserve_words(
    len: u32,
    cap: u32,
    add: u32,
    grow: bool,
    policy: impl Fn() -> u32,
) -> Option<u32> {
    let need = add.wrapping_add(len);
    if cap >= need {
        return None;
    }
    let mut n = need;
    if grow {
        n = policy();
        if n < need {
            n = need;
        }
    }
    if n == len { None } else { Some(n) }
}

/// `0x0092e560`: the byte-vector twin of the growth rule above (same policy,
/// element size 1 instead of 4).
pub fn w26a11_reserve_bytes(
    len: u32,
    cap: u32,
    add: u32,
    grow: bool,
    policy: impl Fn() -> u32,
) -> Option<u32> {
    w26a11_reserve_words(len, cap, add, grow, policy)
}

#[cfg(test)]
mod tests_wave26_agent11 {
    use super::*;

    #[test]
    fn one_and_scalar() {
        assert_eq!(w26a11_set_one(), vec![1]);
        assert_eq!(
            w26a11_mul_scalar(&[0xffff_ffff], 2, 0),
            vec![0xffff_fffe, 1]
        );
        assert_eq!(w26a11_mul_scalar(&[], 5, 0), Vec::<u32>::new());
        assert_eq!(w26a11_mul_scalar(&[3], 0, 1), vec![0, 3]);
    }

    #[test]
    fn shift() {
        assert_eq!(w26a11_shl(&[1], 4), vec![16, 0]);
        assert_eq!(w26a11_shl(&[0x8000_0000], 1), vec![0, 1]);
        assert_eq!(w26a11_shl(&[1], 32), vec![0, 1, 0]);
    }

    #[test]
    fn subtract() {
        assert_eq!(w26a11_sub(&[5], &[3]), vec![2]);
        assert_eq!(w26a11_sub(&[0, 1], &[1]), vec![0xffff_ffff, 0]);
        assert_eq!(w26a11_sub(&[1], &[2]), vec![0xffff_ffff]);
    }

    #[test]
    fn multiply_and_square() {
        assert_eq!(
            w26a11_mul(&[0xffff_ffff], &[0xffff_ffff]),
            vec![1, 0xffff_fffe]
        );
        assert_eq!(w26a11_mul(&[2, 1], &[3]), vec![6, 3, 0]);
        assert_eq!(w26a11_square(&[0xffff_ffff]), vec![1, 0xffff_fffe]);
        let a = [0x1234_5678, 0x9abc_def0];
        assert_eq!(w26a11_square(&a), w26a11_mul(&a, &a));
        assert!(w26a11_mul(&[], &[1]).is_empty());
    }

    #[test]
    fn reserve() {
        assert_eq!(w26a11_reserve_words(2, 8, 3, false, || 0), None);
        assert_eq!(w26a11_reserve_words(2, 4, 3, false, || 0), Some(5));
        assert_eq!(w26a11_reserve_words(2, 4, 3, true, || 16), Some(16));
        assert_eq!(w26a11_reserve_words(2, 4, 3, true, || 1), Some(5));
        assert_eq!(w26a11_reserve_bytes(2, 4, 3, false, || 0), Some(5));
    }
}
