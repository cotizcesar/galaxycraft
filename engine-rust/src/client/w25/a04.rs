#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent04.rs
// ===========================================================================
// Wave 25 translation batch 04.
//
// Most of this queue is MSVC exception funclets and STL tree/hash walks
// that never return; only the pure arithmetic of three routines is kept.

/// Mixer used by the engine's integer hash (`std::hash<uint>` finalizer):
/// two xor-shift/multiply rounds and a final xor-shift.
pub fn w25a04_mix_hash(key: u32) -> u32 {
    let a = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    let b = (a >> 13 ^ a).wrapping_mul(0xc2b2_ae35);
    b >> 16 ^ b
}

/// Bucket slot for `key` in a table with `bucket_count` buckets (power of
/// two).  `None` when the table has no bucket storage (the retail code
/// aborts there).
pub fn w25a04_bucket_index(key: u32, bucket_count: u32) -> Option<u32> {
    if bucket_count == 0 {
        return None;
    }
    Some(w25a04_mix_hash(key) & bucket_count.wrapping_sub(1))
}

/// Shared body of two id-registration routines: post-increments the global
/// counter `*counter`, hashes the *new* value and requires it to be present
/// in `contains`; on success the three callbacks run in order and the old
/// counter value is returned.  `None` mirrors the abort path (the counter is
/// still advanced).
fn w25a04_register_id(
    counter: &mut u32,
    bucket_count: u32,
    contains: impl Fn(u32, u32) -> bool,
    mut steps: impl FnMut(),
) -> Option<u32> {
    let old = *counter;
    *counter = old.wrapping_add(1);
    let slot = w25a04_bucket_index(*counter, bucket_count)?;
    if !contains(slot, *counter) {
        return None;
    }
    steps();
    Some(old)
}

/// `0x0081ffb0`: allocates the next id from the global counter and checks
/// its presence in the hash table at object offsets `+0x94` (bucket count)
/// / `+0x98` (buckets); runs the three follow-up steps and returns the id.
pub fn w25a04_alloc_id_table_94(
    counter: &mut u32,
    bucket_count: u32,
    contains: impl Fn(u32, u32) -> bool,
    steps: impl FnMut(),
) -> Option<u32> {
    w25a04_register_id(counter, bucket_count, contains, steps)
}

/// `0x008200f0`: same as the routine at `0x0081ffb0` but for the table at
/// object offsets `+0xa4` (bucket count) / `+0xa8` (buckets).
pub fn w25a04_alloc_id_table_a4(
    counter: &mut u32,
    bucket_count: u32,
    contains: impl Fn(u32, u32) -> bool,
    steps: impl FnMut(),
) -> Option<u32> {
    w25a04_register_id(counter, bucket_count, contains, steps)
}

/// `0x00844060`: element index of `ptr` in a vector with base `base` and
/// element stride `0x38`, plus the capacity-growth update.  `meta` is the
/// packed size/capacity word (`bit31` flag, low 31 bits capacity).  Returns
/// `(index, new_meta)`; `new_meta` only differs when `len == capacity`,
/// growing by `max(1, capacity / 2)` while preserving the flag bit.
pub fn w25a04_vec38_index_grow(base: u32, ptr: u32, len: u32, meta: u32) -> (u32, u32) {
    let index = ptr.wrapping_sub(base) / 0x38;
    let cap = meta & 0x7fff_ffff;
    if len != cap || cap.wrapping_add(1) <= cap {
        return (index, meta);
    }
    let half = meta >> 1 & 0x3fff_ffff;
    let add = if half > 0 { half } else { 1 };
    let new_meta = (add.wrapping_add(cap) ^ meta) & 0x7fff_ffff ^ meta;
    (index, new_meta)
}

#[cfg(test)]
mod tests_wave25_agent04 {
    use super::*;

    #[test]
    fn hash_and_bucket() {
        assert_eq!(w25a04_mix_hash(0), 0);
        assert_eq!(w25a04_bucket_index(5, 0), None);
        assert!(w25a04_bucket_index(5, 16).unwrap() < 16);
    }

    #[test]
    fn alloc_ids() {
        let mut c = 7;
        let mut n = 0;
        assert_eq!(
            w25a04_alloc_id_table_94(&mut c, 8, |_, k| k == 8, || n += 1),
            Some(7)
        );
        assert_eq!((c, n), (8, 1));
        let mut c = 1;
        assert_eq!(
            w25a04_alloc_id_table_a4(&mut c, 8, |_, _| false, || {}),
            None
        );
        assert_eq!(c, 2);
        assert_eq!(
            w25a04_alloc_id_table_a4(&mut c, 0, |_, _| true, || {}),
            None
        );
    }

    #[test]
    fn vec_grow() {
        assert_eq!(w25a04_vec38_index_grow(0x1000, 0x1000 + 0x70, 2, 4), (2, 4));
        assert_eq!(w25a04_vec38_index_grow(0, 0x38, 4, 4), (1, 6));
        assert_eq!(w25a04_vec38_index_grow(0, 0, 0, 0), (0, 1));
        assert_eq!(w25a04_vec38_index_grow(0, 0, 4, 0x8000_0004).1, 0x8000_0006);
    }
}
