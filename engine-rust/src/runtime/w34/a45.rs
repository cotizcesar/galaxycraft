#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent45.rs
// ===========================================================================
// Wave 34 agent 45: small range-table helpers (runtime.rs).

/// `0x00b4ee90`: number of 32-byte records between `begin` and `end` byte
/// offsets (`(end - begin) >> 4 >> 1`).  The validation call is not modelled.
pub fn w34a45_record_count(begin: i32, end: i32) -> u32 {
    ((end.wrapping_sub(begin) >> 4) as u32) >> 1
}

/// `0x00b4eeb0`: reads record `index` (stride 0x20) and returns the dwords
/// at offsets 0 and 4, and at offsets 0x10 and 0x14.
pub fn w34a45_read_record_pairs(rows: &[u8], index: usize) -> Option<([u32; 2], [u32; 2])> {
    let base = index.checked_mul(0x20)?;
    let rd = |o: usize| -> Option<u32> {
        let s = rows.get(base.checked_add(o)?..base.checked_add(o + 4)?)?;
        Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    };
    Some(([rd(0)?, rd(4)?], [rd(0x10)?, rd(0x14)?]))
}

/// `0x00b4eef0`: walks `(lo, hi, count, _)` records (stride 16 bytes).
/// `a` and `b` are 64-bit keys.  For the first record whose key is not
/// below `a`... returns false when the key exceeds `a` and either the
/// accumulated count is positive or the key is below `b`; true otherwise.
pub fn w34a45_range_check(recs: &[[u32; 4]], a: u64, b: u64) -> bool {
    let mut acc: i32 = 0;
    for r in recs {
        let key = ((r[1] as u64) << 32) | r[0] as u64;
        if a < key {
            return !(acc > 0 || key < b);
        }
        acc = acc.wrapping_add(r[2] as i32);
    }
    true
}

#[cfg(test)]
mod tests_wave34_agent45 {
    use super::*;
    #[test]
    fn count() {
        assert_eq!(w34a45_record_count(0, 64), 2);
    }
    #[test]
    fn pairs() {
        let mut v = vec![0u8; 64];
        v[32] = 1;
        v[36] = 2;
        v[48] = 3;
        v[52] = 4;
        assert_eq!(w34a45_read_record_pairs(&v, 1), Some(([1, 2], [3, 4])));
        assert_eq!(w34a45_read_record_pairs(&v, 2), None);
    }
    #[test]
    fn range() {
        let r = [[5, 0, 1, 0], [20, 0, 1, 0]];
        assert!(w34a45_range_check(&r, 100, 0));
        assert!(!w34a45_range_check(&r, 10, 0));
        assert!(w34a45_range_check(&r, 4, 0));
    }
}
