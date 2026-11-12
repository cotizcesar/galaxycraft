#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent19.rs
// ===========================================================================
// Wave 32 agent 19 translations.

/// `0x009651f0`: returns bit 3 of the byte (`(b >> 3) & 1`).
pub fn w32a19_bit3(b: u8) -> u8 {
    (b >> 3) & 1
}

/// `0x009731c0`: 64-bit FNV-style hash of `key` (sign-extended, xor the
/// basis `0x84222325cbf29ce4`) then each byte of `data` (xor, multiply by
/// prime `0x100000001b3`, wrapping); the result is pushed onto `out`.
/// Always returns 1.
pub fn w32a19_hash_push(key: i32, data: &[u8], out: &mut Vec<u64>) -> u32 {
    let basis: u64 = 0x8422_2325_cbf2_9ce4;
    let prime: u64 = 0x0000_0100_0000_01b3;
    let mut h = ((((key >> 31) as u32 as u64) << 32) | (key as u32 as u64)) ^ basis;
    h = h.wrapping_mul(prime);
    for &b in data {
        h = (h ^ b as u64).wrapping_mul(prime);
    }
    out.push(h);
    1
}

/// State touched by `0x0097d090`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W32a19State {
    pub table: Vec<u32>,
    pub byte_a: u8,
    pub val_a: i32,
    pub byte_b: u8,
    pub val_b: i32,
    pub dirty: u32,
}

/// `0x0097d090`: updates one of two (byte,value) slots from `table[idx]`
/// when either differs; sets the dirty flag. `which == 0` selects slot A.
pub fn w32a19_update_slot(s: &mut W32a19State, idx: usize, value: i32, which: i32) {
    let t = s.table[idx];
    let (b, v) = if which == 0 {
        (&mut s.byte_a, &mut s.val_a)
    } else {
        (&mut s.byte_b, &mut s.val_b)
    };
    if t != *b as u32 || value != *v {
        *b = t as u8;
        *v = value;
        s.dirty = 1;
    }
}

#[cfg(test)]
mod tests_wave32_agent19 {
    use super::*;
    #[test]
    fn bit3() {
        assert_eq!(w32a19_bit3(8), 1);
        assert_eq!(w32a19_bit3(0xf7), 0);
    }
    #[test]
    fn hash() {
        let mut v = vec![];
        assert_eq!(w32a19_hash_push(0, b"ab", &mut v), 1);
        assert_eq!(v.len(), 1);
        let mut w = vec![];
        w32a19_hash_push(0, b"ab", &mut w);
        assert_eq!(v, w);
    }
    #[test]
    fn slot() {
        let mut s = W32a19State {
            table: vec![5],
            ..Default::default()
        };
        w32a19_update_slot(&mut s, 0, 3, 0);
        assert_eq!((s.byte_a, s.val_a, s.dirty), (5, 3, 1));
        s.dirty = 0;
        w32a19_update_slot(&mut s, 0, 3, 0);
        assert_eq!(s.dirty, 0);
        w32a19_update_slot(&mut s, 0, 3, 1);
        assert_eq!((s.byte_b, s.dirty), (5, 1));
    }
}
