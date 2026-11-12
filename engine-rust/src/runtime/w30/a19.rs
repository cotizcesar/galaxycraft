#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent19.rs
// ===========================================================================
// Wave 30 agent 19: small runtime helpers.

/// `0x00b3a8f0`: copies two 16-byte blocks at offsets `0x70` and `0x80` of
/// the object (taken under its lock at `+0xa0`) into a 32-byte output.
pub fn w30a19_copy_locked_pair(obj: &[u8]) -> Option<[u8; 32]> {
    let a = obj.get(0x70..0x90)?;
    let mut out = [0u8; 32];
    out.copy_from_slice(a);
    Some(out)
}

/// `0x00b3ac30`: scans 16-byte records (four i32 words) for one whose
/// words 0 and 1 equal `(key_a, key_b)` and returns its word 2 (the owner
/// pointer); `None` when absent.
pub fn w30a19_find_record_owner(records: &[[i32; 4]], key_a: i32, key_b: i32) -> Option<i32> {
    records
        .iter()
        .find(|r| r[0] == key_a && r[1] == key_b)
        .map(|r| r[2])
}

/// `0x00b3af20`: capacity growth.  The low 31 bits of `word` hold the
/// capacity; when below `wanted` the new capacity is the larger of
/// `cap + cap/2` and `wanted`, and the top bit is preserved.  Returns the
/// updated word, or `None` when no growth is needed.
pub fn w30a19_grow_capacity_word(word: u32, wanted: u32) -> Option<u32> {
    let cap = word & 0x7fff_ffff;
    if cap >= wanted {
        return None;
    }
    let mut new_cap = ((word >> 1) & 0x3fff_ffff) + cap;
    if new_cap < wanted {
        new_cap = wanted;
    }
    Some(((word ^ new_cap) & 0x7fff_ffff) ^ word)
}

/// `0x00b3dfd0`: derives the 64-bit low-bits mask `(1 << bits) - 1`
/// (wrapping; `bits >= 64` yields all ones) as `(lo, hi)` words.
pub fn w30a19_bit_mask_words(bits: u8) -> (u32, u32) {
    let b = bits as u32;
    let mut hi_v = 0u32;
    let mut lo_v = 1u32 << (b & 31);
    if b > 31 {
        hi_v = lo_v;
        lo_v = 0;
    }
    if b > 63 {
        hi_v = lo_v;
    }
    (
        lo_v.wrapping_sub(1),
        hi_v.wrapping_sub(1).wrapping_add((lo_v != 0) as u32),
    )
}

#[cfg(test)]
mod tests_wave30_agent19 {
    use super::*;

    #[test]
    fn copy_pair() {
        let mut o = vec![0u8; 0xa0];
        o[0x70] = 1;
        o[0x8f] = 9;
        let r = w30a19_copy_locked_pair(&o).unwrap();
        assert_eq!((r[0], r[31]), (1, 9));
        assert!(w30a19_copy_locked_pair(&o[..0x80]).is_none());
    }

    #[test]
    fn find_owner() {
        let r = [[1, 2, 3, 0], [4, 5, 6, 0]];
        assert_eq!(w30a19_find_record_owner(&r, 4, 5), Some(6));
        assert_eq!(w30a19_find_record_owner(&r, 4, 6), None);
    }

    #[test]
    fn grow() {
        assert_eq!(w30a19_grow_capacity_word(10, 8), None);
        assert_eq!(w30a19_grow_capacity_word(10, 12), Some(15));
        assert_eq!(w30a19_grow_capacity_word(10, 100), Some(100));
        assert_eq!(
            w30a19_grow_capacity_word(0x8000_0004, 100),
            Some(0x8000_0064)
        );
    }

    #[test]
    fn mask() {
        assert_eq!(w30a19_bit_mask_words(0), (0, 0));
        assert_eq!(w30a19_bit_mask_words(8), (0xff, 0));
        assert_eq!(w30a19_bit_mask_words(32), (0xffff_ffff, 0));
        assert_eq!(w30a19_bit_mask_words(40), (0xffff_ffff, 0xff));
    }
}
