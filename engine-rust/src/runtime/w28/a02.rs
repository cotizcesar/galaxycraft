#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent02.rs
// ===========================================================================
// Wave 28 translation batch 02: small libvorbis/ogg buffer helpers.
//
// Implements 0x009f2520, 0x009f3530, 0x009f74a0, 0x009f03e0, 0x009f8bb0, 0x009f1e70.

/// Ogg byte buffer header used by `0x009f2520` (fields at +0,+4,+8,+0xc).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct W28a02OggBuf {
    pub storage: Vec<u8>,
    pub capacity: i32,
    pub fill: i32,
    pub returned: i32,
}

/// `0x009f2520`: reserve `need` bytes in an ogg buffer.  Returns the write
/// offset (`fill`) or `None` on failure.  `alloc_ok` models the allocator;
/// `on_fail` is the cleanup call made when it fails.
pub fn w28a02_oggpack_reserve(
    b: &mut W28a02OggBuf,
    need: i32,
    alloc_ok: bool,
    on_fail: &mut dyn FnMut(&mut W28a02OggBuf),
) -> Option<usize> {
    if b.capacity < 0 {
        return None;
    }
    if b.returned != 0 {
        b.fill -= b.returned;
        if b.fill > 0 {
            let r = b.returned as usize;
            let n = b.fill as usize;
            b.storage.copy_within(r..r + n, 0);
        }
        b.returned = 0;
    }
    if b.capacity - b.fill < need {
        let new_cap = need + 0x1000 + b.fill;
        if !alloc_ok {
            b.storage = Vec::new();
            on_fail(b);
            return None;
        }
        b.storage.resize(new_cap as usize, 0);
        b.capacity = new_cap;
    }
    Some(b.fill as usize)
}

/// `0x009f3530`: returns true (1) when the field at +0x1c is zero; otherwise
/// the original clears the 0x68-byte record.
pub fn w28a02_field_1c_is_zero(field_1c: u32) -> bool {
    field_1c == 0
}

/// `0x009f74a0`: true when the clear branch (0x70-byte memset) is taken:
/// `f2 > 0 && f0 >= 0x40 && f0 <= f1` of the block at +0x1c.
pub fn w28a02_block_clear_condition(present: bool, f0: i32, f1: i32, f2: i32) -> bool {
    present && f2 > 0 && f0 > 0x3f && f0 <= f1
}

/// `0x009f03e0`: free the five pointers at words 4..=8 and zero the 0x2c-byte
/// record (11 words).
pub fn w28a02_free_five_and_clear(rec: &mut [u32; 11], free: &mut dyn FnMut(u32)) {
    for &w in &rec[4..9] {
        if w != 0 {
            free(w);
        }
    }
    *rec = [0; 11];
}

/// `0x009f8bb0`: same shape as above for a 0x38-byte record (14 words),
/// freeing words 4..=8.
pub fn w28a02_free_five_and_clear_38(rec: &mut [u32; 14], free: &mut dyn FnMut(u32)) {
    for &w in &rec[4..9] {
        if w != 0 {
            free(w);
        }
    }
    *rec = [0; 14];
}

/// `0x009f1e70`: free words 0, 4, 5 of a 0x168-byte record (90 words) and zero
/// it; returns 0 always (including for a missing record).
pub fn w28a02_free_three_and_clear(rec: Option<&mut [u32; 90]>, free: &mut dyn FnMut(u32)) -> i32 {
    if let Some(r) = rec {
        for i in [0usize, 4, 5] {
            if r[i] != 0 {
                free(r[i]);
            }
        }
        *r = [0; 90];
    }
    0
}

#[cfg(test)]
mod tests_wave28_agent02 {
    use super::*;

    #[test]
    fn reserve() {
        let mut b = W28a02OggBuf::default();
        assert_eq!(
            w28a02_oggpack_reserve(&mut b, 10, true, &mut |_| {}),
            Some(0)
        );
        assert_eq!(b.capacity, 10 + 0x1000);
        b.fill = 5;
        b.returned = 2;
        b.storage[2..5].copy_from_slice(&[7, 8, 9]);
        assert_eq!(
            w28a02_oggpack_reserve(&mut b, 1, true, &mut |_| {}),
            Some(3)
        );
        assert_eq!(&b.storage[..3], &[7, 8, 9]);
        b.capacity = -1;
        assert_eq!(w28a02_oggpack_reserve(&mut b, 1, true, &mut |_| {}), None);
        let mut c = W28a02OggBuf::default();
        let mut called = false;
        assert_eq!(
            w28a02_oggpack_reserve(&mut c, 1, false, &mut |_| called = true),
            None
        );
        assert!(called);
    }

    #[test]
    fn predicates() {
        assert!(w28a02_field_1c_is_zero(0));
        assert!(!w28a02_field_1c_is_zero(1));
        assert!(w28a02_block_clear_condition(true, 0x40, 0x40, 1));
        assert!(!w28a02_block_clear_condition(true, 0x3f, 0x40, 1));
        assert!(!w28a02_block_clear_condition(true, 0x41, 0x40, 1));
        assert!(!w28a02_block_clear_condition(false, 0x40, 0x40, 1));
        assert!(!w28a02_block_clear_condition(true, 0x40, 0x40, 0));
    }

    #[test]
    fn frees() {
        let mut r = [1u32; 11];
        r[6] = 0;
        let mut f = vec![];
        w28a02_free_five_and_clear(&mut r, &mut |p| f.push(p));
        assert_eq!(f.len(), 4);
        assert_eq!(r, [0; 11]);
        let mut r = [2u32; 14];
        let mut n = 0;
        w28a02_free_five_and_clear_38(&mut r, &mut |_| n += 1);
        assert_eq!((n, r), (5, [0; 14]));
        let mut r = [3u32; 90];
        let mut n = 0;
        assert_eq!(
            w28a02_free_three_and_clear(Some(&mut r), &mut |_| n += 1),
            0
        );
        assert_eq!((n, r[0]), (3, 0));
        assert_eq!(w28a02_free_three_and_clear(None, &mut |_| n += 1), 0);
    }
}
