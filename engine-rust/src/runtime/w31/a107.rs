#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent107.rs
// ===========================================================================
// Wave 31 agent 107: pm_dx12 flag/extent helpers (pure parts only).

/// `0x00c43850` (flag step): maps a 3-bit option word to the driver flags:
/// bit0 kept, bit1 -> 4, bit2 -> 8.
pub fn w31a107_map_flags(v: u32) -> u32 {
    let mut r = v & 1;
    if v & 2 != 0 {
        r |= 4;
    }
    if v & 4 != 0 {
        r |= 8;
    }
    r
}

/// `0x00c443c0` (extent step): rounds a width/height up to the block size
/// when the format flag `0x10` is set, otherwise returns the extent as is.
pub fn w31a107_align_extent(extent: u32, block: u32, compressed: bool) -> u32 {
    if compressed && block != 0 {
        extent.wrapping_add(block).wrapping_sub(1) & block.wrapping_neg()
    } else {
        extent
    }
}

/// `0x00c44530` (offset step): `((fmt >> 28) + 1) * a + b`, wrapping.
pub fn w31a107_subresource_offset(fmt: u32, a: u32, b: u32) -> u32 {
    ((fmt >> 28).wrapping_add(1))
        .wrapping_mul(a)
        .wrapping_add(b)
}

#[cfg(test)]
mod tests_wave31_agent107 {
    use super::*;

    #[test]
    fn flags() {
        assert_eq!(w31a107_map_flags(7), 13);
        assert_eq!(w31a107_map_flags(2), 4);
        assert_eq!(w31a107_map_flags(0), 0);
    }

    #[test]
    fn align() {
        assert_eq!(w31a107_align_extent(5, 4, true), 8);
        assert_eq!(w31a107_align_extent(5, 4, false), 5);
        assert_eq!(w31a107_align_extent(8, 4, true), 8);
    }

    #[test]
    fn offset() {
        assert_eq!(w31a107_subresource_offset(0x2000_0000, 3, 1), 10);
    }
}
