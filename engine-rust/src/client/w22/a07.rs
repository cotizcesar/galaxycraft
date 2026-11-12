#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent07.rs
// ===========================================================================
// Wave 22 translation batch 07.

/// `0x00695b50`: min of two per-slot u32 tables selected by `kind`
/// (0 -> 0x3054/0x30b4, 1 -> 0x30e4/0x3144, 2 -> 0x3174/0x31d4, stride 4,
/// relative to the unmasked state base).  Other kinds return 0.
pub fn w22a07_min_pair(base: &[u8], kind: u8, slot: u8) -> u32 {
    let (a, b) = match kind {
        0 => (0x3054usize, 0x30b4usize),
        1 => (0x30e4, 0x3144),
        2 => (0x3174, 0x31d4),
        _ => return 0,
    };
    let rd = |o: usize| {
        let p = o + slot as usize * 4;
        u32::from_le_bytes([base[p], base[p + 1], base[p + 2], base[p + 3]])
    };
    rd(a).min(rd(b))
}

/// `0x0069a6e0`: true when the (opaque) check at 0x74_8d60 is zero, the
/// per-player flag byte (state + 0xe544 + player*0xc + unit byte at +0x4c)
/// is zero and global DAT_011a4580 is zero.
pub fn w22a07_flag_clear(check_nonzero: bool, flag_byte: u8, global: u32) -> bool {
    !check_nonzero && flag_byte == 0 && global == 0
}

/// Murmur-style finalizer used by the hash lookup at `0x006a9b00`.
pub fn w22a07_hash(key: u32) -> u32 {
    let h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    let h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    h >> 16 ^ h
}

/// `0x006a9b00`: bucket-chain lookup.  `buckets.len()` is a power of two
/// (empty table is the original's error path -> `None`); each bucket lists
/// chain keys in order.  Returns (bucket, position) of the first match.
pub fn w22a07_hash_find(buckets: &[Vec<u32>], key: u32) -> Option<(usize, usize)> {
    if buckets.is_empty() {
        return None;
    }
    let b = (w22a07_hash(key) as usize) & (buckets.len() - 1);
    buckets[b].iter().position(|&k| k == key).map(|p| (b, p))
}

/// `0x006bb610`: first dword of the record returned by the helper.
pub fn w22a07_rec_first(rec: &[u32]) -> u32 {
    rec[0]
}

/// `0x006bb620`: second dword of the record returned by the helper.
pub fn w22a07_rec_second(rec: &[u32]) -> u32 {
    rec[1]
}

/// `0x006bbcd0`: forwards its argument to the byte-returning helper.
pub fn w22a07_forward_byte(arg: u32, f: impl Fn(u32) -> u8) -> u8 {
    f(arg)
}

/// `0x006c06e0`: clears the u16 mask, and unless the 0x74_8d60 check is
/// set scans the 8 flag bytes (stride 1, loop counter step 0x24 below
/// 0x120) returning the first nonzero index (the original tail-calls
/// 0x74_8d40 with it).
pub fn w22a07_first_flagged(mask: &mut u16, check_nonzero: bool, flags: &[u8]) -> Option<u8> {
    *mask = 0;
    if check_nonzero {
        return None;
    }
    flags.iter().take(8).position(|&f| f != 0).map(|i| i as u8)
}

#[cfg(test)]
mod tests_wave22_agent07 {
    use super::*;

    #[test]
    fn min_pair() {
        let mut b = vec![0u8; 0x3300];
        b[0x3054 + 8..0x3054 + 12].copy_from_slice(&5u32.to_le_bytes());
        b[0x30b4 + 8..0x30b4 + 12].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(w22a07_min_pair(&b, 0, 2), 3);
        assert_eq!(w22a07_min_pair(&b, 1, 2), 0);
        assert_eq!(w22a07_min_pair(&b, 3, 2), 0);
    }

    #[test]
    fn flag() {
        assert!(w22a07_flag_clear(false, 0, 0));
        assert!(!w22a07_flag_clear(true, 0, 0));
        assert!(!w22a07_flag_clear(false, 1, 0));
        assert!(!w22a07_flag_clear(false, 0, 1));
    }

    #[test]
    fn hash_lookup() {
        assert_eq!(w22a07_hash(0), 0);
        let mut t = vec![Vec::new(); 8];
        let b = w22a07_hash(77) as usize & 7;
        t[b].push(1000);
        t[b].push(77);
        assert_eq!(w22a07_hash_find(&t, 77), Some((b, 1)));
        assert_eq!(w22a07_hash_find(&t, 78), None);
        assert_eq!(w22a07_hash_find(&[], 1), None);
    }

    #[test]
    fn small() {
        assert_eq!(w22a07_rec_first(&[4, 9]), 4);
        assert_eq!(w22a07_rec_second(&[4, 9]), 9);
        assert_eq!(w22a07_forward_byte(3, |x| x as u8 + 1), 4);
        let mut m = 7u16;
        assert_eq!(w22a07_first_flagged(&mut m, false, &[0, 0, 2, 0]), Some(2));
        assert_eq!(m, 0);
        assert_eq!(w22a07_first_flagged(&mut m, true, &[1]), None);
    }
}
