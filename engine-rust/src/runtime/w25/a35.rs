#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent35.rs
// ===========================================================================
// Wave 25 agent 35: container/dispatch helpers (runtime.rs target).

/// `0x00866d40` (hash part): finalizer-style mix of a key and bucket
/// selection `mix & (bucket_count - 1)`.  Multipliers are the signed
/// constants `-0x7a143595` and `-0x3d4d51cb` from the retail code.
pub fn w25a35_hash_bucket(key: u32, bucket_count: u32) -> u32 {
    let mut h = key;
    h = (h >> 16 ^ h).wrapping_mul(0x85EB_CA6B);
    h = (h >> 13 ^ h).wrapping_mul(0xC2B2_AE35);
    (h >> 16 ^ h) & bucket_count.wrapping_sub(1)
}

/// `0x00866d40`: calls the owner's virtual slot `+0x34` with the key, then
/// (when the bucket table exists) returns the selected bucket index.
pub fn w25a35_lookup_bucket(
    key: u32,
    table_present: bool,
    bucket_count: u32,
    mut owner_slot34: impl FnMut(u32),
) -> Option<u32> {
    owner_slot34(key);
    if table_present {
        Some(w25a35_hash_bucket(key, bucket_count))
    } else {
        None
    }
}

/// `0x00866c20`: post-increments a 64-bit id counter (lo/hi pair with
/// carry), forwards `(lo, hi, arg)` to the owner's virtual slot `+0x30`
/// and returns the id that was handed out.
pub fn w25a35_next_id(
    counter: &mut (u32, u32),
    arg: u32,
    mut owner_slot30: impl FnMut(u32, u32, u32),
) -> (u32, u32) {
    let old = *counter;
    let (lo, carry) = counter.0.overflowing_add(1);
    counter.0 = lo;
    counter.1 = counter.1.wrapping_add(carry as u32);
    owner_slot30(old.0, old.1, arg);
    old
}

/// `0x00866020`: forwards `(a, b)` to the owner's virtual slot `+0x2c`.
pub fn w25a35_forward_slot2c<R>(a: u32, b: u32, mut owner_slot2c: impl FnMut(u32, u32) -> R) -> R {
    owner_slot2c(a, b)
}

#[cfg(test)]
mod tests_wave25_agent35 {
    use super::*;

    #[test]
    fn hash_bucket_in_range() {
        assert_eq!(w25a35_hash_bucket(0, 16), 0);
        for k in 0..100u32 {
            assert!(w25a35_hash_bucket(k, 8) < 8);
        }
        assert_eq!(w25a35_hash_bucket(1, 1), 0);
    }

    #[test]
    fn lookup_calls_slot() {
        let mut seen = 0;
        assert_eq!(w25a35_lookup_bucket(5, false, 8, |k| seen = k), None);
        assert_eq!(seen, 5);
        assert!(w25a35_lookup_bucket(5, true, 8, |_| {}).is_some());
    }

    #[test]
    fn next_id_carries() {
        let mut c = (u32::MAX, 0);
        let mut got = None;
        let r = w25a35_next_id(&mut c, 9, |l, h, a| got = Some((l, h, a)));
        assert_eq!(r, (u32::MAX, 0));
        assert_eq!(c, (0, 1));
        assert_eq!(got, Some((u32::MAX, 0, 9)));
    }

    #[test]
    fn forward() {
        assert_eq!(w25a35_forward_slot2c(1, 2, |a, b| a + b), 3);
    }
}
