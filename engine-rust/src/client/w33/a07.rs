#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent07.rs
// ===========================================================================
// Wave 33 translation batch 07: small setter helpers of a library context.

/// `0x00992070`: stores `value` into the shared global slot (+0xc) and into
/// the context field at +0x5758, returning 0.
pub fn w33a07_set_shared_value(global_slot: &mut u32, ctx_field: &mut u32, value: u32) -> u32 {
    *global_slot = value;
    *ctx_field = value;
    0
}

/// `0x00992090`: validates and stores a size pair.  Returns 0x20 when the
/// context is already initialised (`init` flag at +0x15 non-zero), 0x25 when
/// `unit == 0` or `count <= 1`, otherwise writes `unit` (+0x584) and
/// `unit * count` (+0x588, wrapping) and returns 0.
pub fn w33a07_set_size_pair(
    init: u8,
    f584: &mut i32,
    f588: &mut i32,
    unit: i32,
    count: i32,
) -> u32 {
    if init != 0 {
        return 0x20;
    }
    if unit != 0 && 1 < count {
        *f584 = unit;
        *f588 = unit.wrapping_mul(count);
        return 0;
    }
    0x25
}

#[cfg(test)]
mod tests_wave33_agent07 {
    use super::*;

    #[test]
    fn shared_value() {
        let (mut g, mut c) = (0, 0);
        assert_eq!(w33a07_set_shared_value(&mut g, &mut c, 7), 0);
        assert_eq!((g, c), (7, 7));
    }

    #[test]
    fn size_pair() {
        let (mut a, mut b) = (0, 0);
        assert_eq!(w33a07_set_size_pair(1, &mut a, &mut b, 4, 4), 0x20);
        assert_eq!(w33a07_set_size_pair(0, &mut a, &mut b, 0, 4), 0x25);
        assert_eq!(w33a07_set_size_pair(0, &mut a, &mut b, 4, 1), 0x25);
        assert_eq!(w33a07_set_size_pair(0, &mut a, &mut b, 4, 3), 0);
        assert_eq!((a, b), (4, 12));
    }
}
