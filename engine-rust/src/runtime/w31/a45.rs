#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent45.rs
// ===========================================================================
// Wave 31 agent 45: small runtime helpers.

/// `0x008866e0`: stores the result of the supplied getter at offset
/// `0x158` of the object (modelled as a field).
pub fn w31a45_store_at_158(field_158: &mut u32, getter: impl FnOnce() -> u32) {
    *field_158 = getter();
}

/// Result triple of the bucket lookup `0x008867d0`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W31a45Lookup {
    pub found: u32,
    pub bucket_addr: u32,
    pub end_addr: u32,
}

/// `0x008867d0`: hash-bucket lookup.  `count` is the bucket count (power
/// of two), `buckets` the base address (0 means empty table).  `hash` is
/// the key hash; `probe(addr)` returns the node value stored for the
/// bucket at `addr` (0 when absent).
pub fn w31a45_bucket_lookup(
    count: u32,
    buckets: u32,
    hash: u32,
    probe: impl FnOnce(u32) -> u32,
) -> W31a45Lookup {
    if buckets == 0 {
        return W31a45Lookup {
            found: 0,
            bucket_addr: count << 2,
            end_addr: count << 2,
        };
    }
    let bucket = buckets.wrapping_add((count.wrapping_sub(1) & hash).wrapping_mul(4));
    let end = buckets.wrapping_add(count.wrapping_mul(4));
    let found = probe(bucket);
    W31a45Lookup {
        found,
        bucket_addr: if found != 0 { bucket } else { end },
        end_addr: end,
    }
}

/// `0x0088cd30`: initialises an empty circular list header at address
/// `self_addr`: head and tail point at itself, size zero.
pub fn w31a45_list_init(self_addr: u32) -> [u32; 3] {
    [self_addr, self_addr, 0]
}

/// Outcome of the type-specifier check in `0x00894e90`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W31a45Spec {
    Plain,
    Pointer,
    Invalid,
}

/// `0x00894e90`: classifies the format spec; `None` or kind 0 / `s`
/// selects the plain path, `p` the pointer path, anything else is the
/// "invalid type specifier" error.
pub fn w31a45_classify_spec(kind: Option<u8>) -> W31a45Spec {
    match kind {
        None | Some(0) | Some(b's') => W31a45Spec::Plain,
        Some(b'p') => W31a45Spec::Pointer,
        Some(_) => W31a45Spec::Invalid,
    }
}

#[cfg(test)]
mod tests_wave31_agent45 {
    use super::*;

    #[test]
    fn store_and_init() {
        let mut f = 0;
        w31a45_store_at_158(&mut f, || 7);
        assert_eq!(f, 7);
        assert_eq!(w31a45_list_init(0x100), [0x100, 0x100, 0]);
    }

    #[test]
    fn lookup() {
        let r = w31a45_bucket_lookup(8, 0, 5, |_| 1);
        assert_eq!((r.found, r.bucket_addr, r.end_addr), (0, 32, 32));
        let r = w31a45_bucket_lookup(8, 1000, 13, |a| if a == 1020 { 9 } else { 0 });
        assert_eq!(
            r,
            W31a45Lookup {
                found: 9,
                bucket_addr: 1020,
                end_addr: 1032
            }
        );
        let r = w31a45_bucket_lookup(8, 1000, 13, |_| 0);
        assert_eq!(r.bucket_addr, 1032);
    }

    #[test]
    fn spec() {
        assert_eq!(w31a45_classify_spec(None), W31a45Spec::Plain);
        assert_eq!(w31a45_classify_spec(Some(b's')), W31a45Spec::Plain);
        assert_eq!(w31a45_classify_spec(Some(b'p')), W31a45Spec::Pointer);
        assert_eq!(w31a45_classify_spec(Some(b'x')), W31a45Spec::Invalid);
    }
}
