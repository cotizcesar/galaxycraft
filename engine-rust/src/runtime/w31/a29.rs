#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent29.rs
// ===========================================================================
/// `0x006b0510`: splits `value` into a 0xda-sized bucket and remainder.
/// When the bucket index is below `limit_a` and the remainder is below
/// `limit_b`, the result comes from `fallback` (`FUN_006a7a80`); otherwise
/// the low byte of the selected base is cleared.  When the bucket index is
/// below `limit_a`, the base is `limit_b`, else the bucket start.
pub fn w31a29_bucket_lookup(
    value: u32,
    limit_a: u32,
    limit_b: u32,
    fallback: impl FnOnce() -> u32,
) -> u32 {
    let bucket = value / 0xda;
    let mut base = bucket * 0xda;
    if bucket < limit_a {
        base = limit_b;
        if value % 0xda < limit_b {
            return fallback();
        }
    }
    base & 0xffff_ff00
}

#[cfg(test)]
mod tests_wave31_agent29 {
    use super::*;

    #[test]
    fn bucket_lookup() {
        assert_eq!(w31a29_bucket_lookup(5, 1, 10, || 77), 77);
        assert_eq!(w31a29_bucket_lookup(15, 1, 10, || 77), 0);
        assert_eq!(
            w31a29_bucket_lookup(0xda * 3 + 1, 1, 10, || 77),
            0xda * 3 & 0xffff_ff00
        );
        assert_eq!(w31a29_bucket_lookup(0xda * 3 + 20, 5, 0x1234, || 77), 77);
    }
}
