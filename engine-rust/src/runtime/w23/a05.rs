#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent05.rs
// ===========================================================================
/// `0x006dfa60`: returns the product of a per-index lookup (called with
/// index 6) and the 32-bit field at offset `0x14c` of the unmasked global
/// block.  The callee and the block are caller-supplied; arithmetic wraps
/// like the original 32-bit `imul`.
pub fn w23a05_scaled_field(lookup6: i32, block: &[u8]) -> Option<i32> {
    let b = block.get(0x14c..0x150)?;
    let f = i32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    Some(f.wrapping_mul(lookup6))
}

#[cfg(test)]
mod tests_wave23_agent05 {
    use super::*;

    #[test]
    fn scaled_field_multiplies_and_bounds() {
        let mut blk = vec![0u8; 0x150];
        blk[0x14c..0x150].copy_from_slice(&7i32.to_le_bytes());
        assert_eq!(w23a05_scaled_field(6, &blk), Some(42));
        assert_eq!(w23a05_scaled_field(6, &blk[..0x14f]), None);
        blk[0x14c..0x150].copy_from_slice(&i32::MAX.to_le_bytes());
        assert_eq!(w23a05_scaled_field(2, &blk), Some(-2));
    }
}
