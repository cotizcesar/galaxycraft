#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent111.rs
// ===========================================================================
// Wave 31 agent 111: small recovered helpers from the pm_gles glue region.

/// `0x00c54790`: capacity growth. The low 31 bits of `flags` are the current
/// capacity; if it is below `wanted`, the new capacity is the larger of
/// `cap + cap/2` (computed as `(flags >> 1 & 0x3fffffff) + cap`) and `wanted`.
/// The top bit of `flags` is preserved. Returns the updated flags word, or
/// `None` when no growth is needed.
pub fn w31a111_grow_capacity(flags: u32, wanted: u32) -> Option<u32> {
    let cap = flags & 0x7fff_ffff;
    if cap >= wanted {
        return None;
    }
    let mut new_cap = ((flags >> 1) & 0x3fff_ffff).wrapping_add(cap);
    if new_cap < wanted {
        new_cap = wanted;
    }
    Some(((flags ^ new_cap) & 0x7fff_ffff) ^ flags)
}

/// `0x00c54970`: clears a 0x208-byte record: two leading dwords set to zero
/// and the following 0x200 bytes zero-filled.
pub fn w31a111_clear_table(rec: &mut [u8; 0x208]) {
    rec.fill(0);
}

#[cfg(test)]
mod tests_wave31_agent111 {
    use super::*;

    #[test]
    fn grow_capacity() {
        assert_eq!(w31a111_grow_capacity(16, 10), None);
        assert_eq!(w31a111_grow_capacity(16, 20), Some(24));
        assert_eq!(w31a111_grow_capacity(16, 100), Some(100));
        assert_eq!(
            w31a111_grow_capacity(0x8000_0000 | 16, 20),
            Some(0x8000_0000 | 24)
        );
    }

    #[test]
    fn clear_table() {
        let mut r = [0xffu8; 0x208];
        w31a111_clear_table(&mut r);
        assert!(r.iter().all(|&b| b == 0));
    }
}
