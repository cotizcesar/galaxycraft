#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent46.rs
// ===========================================================================
// Wave 25 agent 46: fmt-style writer and hash-map helpers (runtime.rs).

/// `0x00894be0`: resets a circular list head (`next = prev = self`,
/// `size = 0`).  Returns `true` when the old `prev` link already pointed at
/// the head (the clean case); retail otherwise aborts into the unwinder.
pub fn w25a46_list_head_reset(head: &mut [u32; 3], self_id: u32) -> bool {
    let old = head[1];
    head[1] = self_id;
    head[0] = self_id;
    head[2] = 0;
    old == self_id
}

/// Outcome of the char-format spec check in `0x00894d70`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W25a46CharSpec {
    /// No spec: the raw character is appended directly.
    Raw,
    /// Spec has type `'\0'` or `'c'` and valid flags: raw character.
    Char,
    /// Spec has type `'\0'`/`'c'` but a numeric flag is set: error
    /// "invalid format specifier for char".
    InvalidForChar,
    /// Other type: formatted as an integer; value negated and sign `'-'`
    /// for negatives, otherwise sign from the sign mode (bits 4..2 of
    /// flags `<< 0x19 >> 0x1d`: 2 -> '+', other non-0/1 -> ' ').
    Numeric,
}

/// `0x00894d70`: classifies how a `char` argument is written.  `ty` is the
/// spec byte at +8, `flags` the dword at +0xc, `flags2` the byte at +0x10.
pub fn w25a46_classify_char_spec(spec: Option<(u8, u32, u8)>) -> W25a46CharSpec {
    match spec {
        None => W25a46CharSpec::Raw,
        Some((ty, flags, flags2)) => {
            if ty != 0 && ty != b'c' {
                W25a46CharSpec::Numeric
            } else if (flags & 0xf) == 4 || (flags & 0x70) != 0 || (flags2 & 1) != 0 {
                W25a46CharSpec::InvalidForChar
            } else {
                W25a46CharSpec::Char
            }
        }
    }
}

/// `0x00894d70` numeric branch: sign byte for a negative-or-signed char.
/// Returns `(abs_value, Some(sign))`; `sign_mode` is `(flags << 25) >> 29`
/// arithmetic.
pub fn w25a46_char_sign(value: i8, flags: u32) -> (i32, Option<u8>) {
    let v = value as i32;
    if v < 0 {
        return (-v, Some(b'-'));
    }
    let mode = ((flags << 0x19) as i32) >> 0x1d;
    if mode != 0 && mode != 1 {
        (v, Some(if mode == 2 { b'+' } else { b' ' }))
    } else {
        (v, None)
    }
}

/// `0x00894f20`: writes a string.  `spec` is `(type, precision)`; a spec
/// type other than `'\0'`/`'s'` is an error ("invalid type specifier").
/// Precision (when non-negative and smaller) truncates.  Without a spec
/// the whole string is appended to `out`.
pub fn w25a46_write_str(
    out: &mut Vec<u8>,
    spec: Option<(u8, i32)>,
    s: &[u8],
) -> Result<usize, &'static str> {
    match spec {
        None => {
            out.extend_from_slice(s);
            Ok(s.len())
        }
        Some((ty, prec)) => {
            let mut n = s.len();
            let err = ty != 0 && ty != b's';
            if prec >= 0 && (prec as usize) < n {
                n = prec as usize;
            }
            out.extend_from_slice(&s[..n]);
            if err {
                Err("invalid type specifier")
            } else {
                Ok(n)
            }
        }
    }
}

/// `0x00896df0`: writes `"true"`/`"false"`, honouring an optional
/// precision (negative means unlimited) like the string writer.
pub fn w25a46_write_bool(out: &mut Vec<u8>, precision: Option<i32>, value: bool) -> usize {
    let s: &[u8] = if value { b"true" } else { b"false" };
    let mut n = s.len();
    if let Some(p) = precision
        && p >= 0
        && (p as usize) < n
    {
        n = p as usize;
    }
    out.extend_from_slice(&s[..n]);
    n
}

/// `0x008977a0`: hash-bucket lookup.  `hash` comes from the key hasher;
/// it is mixed with the murmur3 finaliser (multipliers 85ebca6b, c2b2ae35
/// as the negated constants), masked by `bucket_count - 1`, and the chain
/// is walked comparing stored hashes.  `buckets[i]` is the first node
/// index; `nodes[i] = (next, hash, value)`.  Returns the value or `None`.
pub fn w25a46_hash_lookup(
    hash: u32,
    buckets: &[Option<usize>],
    nodes: &[(Option<usize>, u32, u64)],
) -> Option<u64> {
    if buckets.is_empty() {
        return None;
    }
    let mut h = (hash >> 16 ^ hash).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    h = (h >> 16 ^ h) & (buckets.len() as u32).wrapping_sub(1);
    let mut cur = buckets[h as usize];
    while let Some(i) = cur {
        let n = nodes.get(i)?;
        if n.1 == hash {
            return Some(n.2);
        }
        cur = n.0;
    }
    None
}

/// `0x00897890`: returns 1 when both lookups (`first`, `second`) are both zero,
/// `flag` holds and the kind is one of 0, 1, 0xf, 0x18; otherwise the
/// first lookup's value.
pub fn w25a46_kind_check(first: i32, second: i32, flag: bool, kind: u16) -> i32 {
    if first == 0 && second == 0 && flag && matches!(kind, 0 | 1 | 0xf | 0x18) {
        return 1;
    }
    first
}

#[cfg(test)]
mod tests_wave25_agent46 {
    use super::*;

    #[test]
    fn list_reset() {
        let mut h = [9, 9, 9];
        assert!(!w25a46_list_head_reset(&mut h, 5));
        assert_eq!(h, [5, 5, 0]);
        assert!(w25a46_list_head_reset(&mut h, 5));
    }

    #[test]
    fn char_spec() {
        assert_eq!(w25a46_classify_char_spec(None), W25a46CharSpec::Raw);
        assert_eq!(
            w25a46_classify_char_spec(Some((b'c', 0, 0))),
            W25a46CharSpec::Char
        );
        assert_eq!(
            w25a46_classify_char_spec(Some((0, 4, 0))),
            W25a46CharSpec::InvalidForChar
        );
        assert_eq!(
            w25a46_classify_char_spec(Some((0, 0x10, 0))),
            W25a46CharSpec::InvalidForChar
        );
        assert_eq!(
            w25a46_classify_char_spec(Some((0, 0, 1))),
            W25a46CharSpec::InvalidForChar
        );
        assert_eq!(
            w25a46_classify_char_spec(Some((b'd', 0, 0))),
            W25a46CharSpec::Numeric
        );
        assert_eq!(w25a46_char_sign(-5, 0), (5, Some(b'-')));
        assert_eq!(w25a46_char_sign(5, 0), (5, None));
        assert_eq!(w25a46_char_sign(5, 2 << 4), (5, Some(b'+')));
        assert_eq!(w25a46_char_sign(5, 3 << 4), (5, Some(b' ')));
    }

    #[test]
    fn strings() {
        let mut o = Vec::new();
        assert_eq!(w25a46_write_str(&mut o, None, b"abc"), Ok(3));
        assert_eq!(w25a46_write_str(&mut o, Some((b's', 2)), b"xyz"), Ok(2));
        assert_eq!(o, b"abcxy");
        assert!(w25a46_write_str(&mut o, Some((b'd', -1)), b"q").is_err());
        let mut b = Vec::new();
        assert_eq!(w25a46_write_bool(&mut b, None, true), 4);
        assert_eq!(w25a46_write_bool(&mut b, Some(3), false), 3);
        assert_eq!(b, b"truefal");
    }

    #[test]
    fn lookup() {
        let h = 0x1234u32;
        let mut m = (h >> 16 ^ h).wrapping_mul(0x85eb_ca6b);
        m = (m >> 13 ^ m).wrapping_mul(0xc2b2_ae35);
        m = (m >> 16 ^ m) & 3;
        let mut buckets = vec![None; 4];
        buckets[m as usize] = Some(0);
        let nodes = vec![(Some(1), 7, 1), (None, h, 42)];
        assert_eq!(w25a46_hash_lookup(h, &buckets, &nodes), Some(42));
        assert_eq!(w25a46_hash_lookup(h, &[], &nodes), None);
    }

    #[test]
    fn kind() {
        assert_eq!(w25a46_kind_check(0, 0, true, 0x18), 1);
        assert_eq!(w25a46_kind_check(0, 0, true, 2), 0);
        assert_eq!(w25a46_kind_check(3, 0, true, 0), 3);
    }
}
