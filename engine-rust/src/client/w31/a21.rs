#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent21.rs
// ===========================================================================
// Wave 31 batch 21: hash/arithmetic helpers from the CRT/library region.
//
// Skipped (no 0x prefix): FUN_00c54850 (shrink branch ends in a
// non-returning call), FUN_00c66f80 (MD5 update loop ending in a
// non-returning call), FUN_00c6c360, FUN_00c6c400 (SEH string ctor glue),
// FUN_00c86b59 (wrapper over unresolved callee), FUN_00c87120,
// FUN_00c87160 (empty), FUN_00c872dd (TLS/lock glue), FUN_00c8a430 (SSE2
// strchr, opaque flag), FUN_00c9b19a (printf glue over unknown state).

/// `0x00c62800`: MurmurHash3 x86_32 finalised with the length (seed `h`).
pub fn w31a21_murmur3_32(data: &[u8], seed: u32) -> u32 {
    let len = data.len() as u32;
    let mut h = seed;
    let blocks = data.len() / 4;
    for i in 0..blocks {
        let mut k = u32::from_le_bytes([
            data[i * 4],
            data[i * 4 + 1],
            data[i * 4 + 2],
            data[i * 4 + 3],
        ]);
        k = k
            .wrapping_mul(0xcc9e2d51)
            .rotate_left(15)
            .wrapping_mul(0x1b873593);
        h ^= k;
        h = h.rotate_left(13).wrapping_mul(5).wrapping_add(0xe6546b64);
    }
    let tail = &data[blocks * 4..];
    if !tail.is_empty() {
        let mut k = 0u32;
        for (i, b) in tail.iter().enumerate() {
            k ^= (*b as u32) << (8 * i);
        }
        k = k
            .wrapping_mul(0xcc9e2d51)
            .rotate_left(15)
            .wrapping_mul(0x1b873593);
        h ^= k;
    }
    h ^= len;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^ (h >> 16)
}

/// `0x00c663d0`: true when the first word of the object is non-zero.
pub fn w31a21_is_set(first_word: i32) -> bool {
    first_word != 0
}

/// `0x00c86d80`: 64-bit multiply built from 32-bit halves (low 64 bits).
pub fn w31a21_mul64(a: u64, b: u64) -> u64 {
    a.wrapping_mul(b)
}

/// `0x00c874e0`: unsigned 64-bit division (`__aulldiv`); divisor must be
/// non-zero (the original traps).
pub fn w31a21_udiv64(n: u64, d: u64) -> Option<u64> {
    n.checked_div(d)
}

/// `0x00c93789`: ASCII tolower for the C locale branch (`A..Z` -> `a..z`).
pub fn w31a21_tolower_c(c: i32) -> i32 {
    if (c.wrapping_sub(0x41) as u32) < 0x1a {
        c + 0x20
    } else {
        c
    }
}

/// `0x00c66df0`: MD5 block transform.  `x` is the 16-word block, `state`
/// the (a, b, c, d) words; `index`, `shift`, `k` are the retail tables
/// (word order, rotate counts, additive constants, 64 entries each).
pub fn w31a21_md5_transform(
    state: &mut [u32; 4],
    x: &[u32; 16],
    index: &[u8; 64],
    shift: &[u8; 64],
    k: &[u32; 64],
) {
    let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);
    for i in 0..64 {
        let f = match i / 16 {
            0 => ((c ^ d) & b) ^ d,
            1 => ((b ^ c) & d) ^ c,
            2 => b ^ c ^ d,
            _ => (!d | b) ^ c,
        };
        let t = f
            .wrapping_add(x[index[i] as usize])
            .wrapping_add(k[i])
            .wrapping_add(a);
        let nb = t.rotate_left((shift[i] & 0x1f) as u32).wrapping_add(b);
        a = d;
        d = c;
        c = b;
        b = nb;
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
}

/// `0x00c94110`: `ceil` for doubles (SSE4 fast path semantics: negative
/// results in (-1, 0) give -0.0, NaN/inf/large values pass through).
pub fn w31a21_ceil(x: f64) -> f64 {
    let bits = x.to_bits();
    let hi = (bits >> 32) as u32;
    let e = hi >> 20;
    let sh = 0x433u32.wrapping_sub(e & 0x7ff);
    if e & 0x800 == 0 {
        if e < 0x3ff {
            return if x > 0.0 { 1.0 } else { 0.0 };
        }
        if e < 0x433 {
            let t = f64::from_bits((bits >> sh) << sh);
            return if t < x { t + 1.0 } else { t };
        }
    } else {
        if e < 0xbff {
            return -0.0;
        }
        if e < 0xc33 {
            return f64::from_bits((bits >> sh) << sh);
        }
    }
    x
}

#[cfg(test)]
mod tests_wave31_agent21 {
    use super::*;

    #[test]
    fn small_helpers() {
        assert!(!w31a21_is_set(0) && w31a21_is_set(-1));
        assert_eq!(
            w31a21_mul64(0x1_0000_0001, 0x1_0000_0003),
            0x1_0000_0001u64.wrapping_mul(0x1_0000_0003)
        );
        assert_eq!(w31a21_udiv64(1 << 40, 3), Some((1u64 << 40) / 3));
        assert_eq!(w31a21_udiv64(5, 0), None);
        assert_eq!(w31a21_tolower_c(0x41), 0x61);
        assert_eq!(w31a21_tolower_c(0x5a), 0x7a);
        assert_eq!(w31a21_tolower_c(0x5b), 0x5b);
        assert_eq!(w31a21_tolower_c(0x40), 0x40);
    }

    #[test]
    fn murmur_known() {
        assert_eq!(w31a21_murmur3_32(b"", 0), 0);
        assert_eq!(w31a21_murmur3_32(b"", 1), 0x514e28b7);
        assert_eq!(w31a21_murmur3_32(b"hello", 0), 0x248bfa47);
    }

    #[test]
    fn ceil_matches_std() {
        for v in [
            0.5,
            1.0,
            1.2,
            2.9999,
            -0.5,
            -1.0,
            -1.5,
            -2.7,
            1e300,
            -1e300,
            4503599627370496.5,
            0.0,
            1e-300,
            -1e-300,
        ] {
            let r = w31a21_ceil(v);
            let e = v.ceil();
            assert!(
                r == e && r.is_sign_negative() == e.is_sign_negative(),
                "{v}: {r} vs {e}"
            );
        }
        assert!(w31a21_ceil(f64::NAN).is_nan());
        assert_eq!(w31a21_ceil(f64::INFINITY), f64::INFINITY);
    }

    #[test]
    fn md5_abc() {
        let mut index = [0u8; 64];
        let mut shift = [0u8; 64];
        let mut k = [0u32; 64];
        let s = [
            [7, 12, 17, 22],
            [5, 9, 14, 20],
            [4, 11, 16, 23],
            [6, 10, 15, 21],
        ];
        for i in 0..64 {
            index[i] = match i / 16 {
                0 => i,
                1 => (5 * i + 1) % 16,
                2 => (3 * i + 5) % 16,
                _ => (7 * i) % 16,
            } as u8;
            shift[i] = s[i / 16][i % 4];
            k[i] = ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32;
        }
        let mut x = [0u32; 16];
        x[0] = u32::from_le_bytes([b'a', b'b', b'c', 0x80]);
        x[14] = 24;
        let mut st = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
        w31a21_md5_transform(&mut st, &x, &index, &shift, &k);
        let mut out = Vec::new();
        for w in st {
            out.extend_from_slice(&w.to_le_bytes());
        }
        let hex: String = out.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "900150983cd24fb0d6963f7d28e17f72");
    }
}
