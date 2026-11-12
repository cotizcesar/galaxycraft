#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent19.rs
// ===========================================================================
/// `0x007f6590`: compares a caller-supplied current value (result of the
/// unresolved getter) with the field at offset 4 of the receiver.
pub fn w25a19_field4_matches(current: i32, receiver_field4: i32) -> bool {
    current == receiver_field4
}

/// `0x007f7ba0`: bucket index for a 64-bit key in a power-of-two hash table
/// (`bucket_count` is the field at +0x20); the key is folded to 32 bits first.
pub fn w25a19_hash_bucket(key: u64, bucket_count: u32) -> u32 {
    let mut h = ((key as u32) >> 16 ^ key as u32).wrapping_mul(0x85ebca6bu32);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2ae35u32);
    (h >> 16 ^ h) & bucket_count.wrapping_sub(1)
}

#[cfg(test)]
mod tests_wave25_agent19 {
    use super::*;

    #[test]
    fn field_match() {
        assert!(w25a19_field4_matches(5, 5));
        assert!(!w25a19_field4_matches(5, 6));
    }

    #[test]
    fn bucket_in_range() {
        assert_eq!(w25a19_hash_bucket(0, 16), 0);
        for k in 0..100u64 {
            assert!(w25a19_hash_bucket(k * 7919, 64) < 64);
        }
    }
}
