#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent60.rs
// ===========================================================================
// Wave 31 agent 60: small vector-push helpers, percent-escape and a hash step.

/// `0x00972350`: pushes a 3-word record `[v, v, 0xffffffff]` and bumps the
/// counter.
pub fn w31a60_push_triple(rows: &mut Vec<u32>, count: &mut u32, v: u32) {
    rows.extend_from_slice(&[v, v, 0xffff_ffff]);
    *count += 1;
}

/// `0x009723a0`: pushes a 4-word record `[a, b, 0xffffffff, c]` and bumps the
/// counter.
pub fn w31a60_push_quad(rows: &mut Vec<u32>, count: &mut u32, a: u32, b: u32, c: u32) {
    rows.extend_from_slice(&[a, b, 0xffff_ffff, c]);
    *count += 1;
}

/// `0x00972840`: pushes `[ffffffff x3, v]` on `rows`, then pushes the old
/// counter value three times plus once more (3-word record) on `refs`.
pub fn w31a60_push_ref(rows: &mut Vec<u32>, count: &mut u32, refs: &mut Vec<u32>, v: u32) {
    rows.extend_from_slice(&[0xffff_ffff, 0xffff_ffff, 0xffff_ffff, v]);
    let old = *count;
    *count += 1;
    refs.extend_from_slice(&[old, old, old]);
}

/// `0x009728d0`: appends `%XX` (uppercase hex) for a byte.
pub fn w31a60_percent_escape(out: &mut Vec<u8>, b: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push(b'%');
    out.push(HEX[(b >> 4) as usize]);
    out.push(HEX[(b & 0xf) as usize]);
}

/// `0x00973260`: hashes a seed and byte run with the 64-bit multiply step
/// (multiplier `0x1000001b3`), pushing the result; returns 1.
pub fn w31a60_hash_push(out: &mut Vec<u64>, seed: u32, data: &[u8]) -> u32 {
    let mul = (0x100u64 << 32) | 0x1b3;
    let lo = seed ^ 0xcbf2_9ce4;
    let hi = ((seed as i32) >> 31) as u32 ^ 0x8422_2325;
    let mut h = (((hi as u64) << 32) | lo as u64).wrapping_mul(mul);
    for &b in data {
        h = ((h ^ b as u64) & 0xffff_ffff | (h & !0xffff_ffff)).wrapping_mul(mul);
    }
    out.push(h);
    1
}

/// `0x00979cf0`: reads the `idx`-th dword of a table.
pub fn w31a60_table_get(table: &[u32], idx: u8) -> Option<u32> {
    table.get(idx as usize).copied()
}

/// `0x00979d10`: the byte at +0x62 when the flag at +0x60 is set, else 0xff.
pub fn w31a60_flag_byte(flag: u8, value: u8) -> u8 {
    if flag != 0 { value } else { 0xff }
}

#[cfg(test)]
mod tests_wave31_agent60 {
    use super::*;

    #[test]
    fn pushes() {
        let (mut r, mut n) = (vec![], 0);
        w31a60_push_triple(&mut r, &mut n, 7);
        assert_eq!(r, [7, 7, 0xffff_ffff]);
        w31a60_push_quad(&mut r, &mut n, 1, 2, 3);
        assert_eq!(n, 2);
        assert_eq!(r[3..], [1, 2, 0xffff_ffff, 3]);
        let mut refs = vec![];
        w31a60_push_ref(&mut r, &mut n, &mut refs, 9);
        assert_eq!(refs, [2, 2, 2]);
        assert_eq!(n, 3);
    }

    #[test]
    fn escape_and_misc() {
        let mut o = vec![];
        w31a60_percent_escape(&mut o, 0xab);
        assert_eq!(o, b"%AB");
        assert_eq!(w31a60_table_get(&[1, 2], 1), Some(2));
        assert_eq!(w31a60_table_get(&[1, 2], 2), None);
        assert_eq!(w31a60_flag_byte(0, 5), 0xff);
        assert_eq!(w31a60_flag_byte(1, 5), 5);
    }

    #[test]
    fn hash() {
        let mut o = vec![];
        assert_eq!(w31a60_hash_push(&mut o, 1, b"ab"), 1);
        let mut p = vec![];
        w31a60_hash_push(&mut p, 1, b"ac");
        assert_ne!(o[0], p[0]);
    }
}
