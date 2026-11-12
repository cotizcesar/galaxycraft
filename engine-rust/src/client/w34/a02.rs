#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent02.rs
// ===========================================================================
// Wave 34 translation batch 02.

/// `0x00960460`: reads four bytes and combines them big-endian
/// (`b0<<24 | b1<<16 | b2<<8 | b3`).  Returns `None` if fewer than four bytes.
pub fn w34a02_read_be_u32(p: &[u8]) -> Option<u32> {
    let b = p.get(..4)?;
    Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

#[cfg(test)]
mod tests_wave34_agent02 {
    use super::*;

    #[test]
    fn be_u32() {
        assert_eq!(w34a02_read_be_u32(&[1, 2, 3, 4, 9]), Some(0x0102_0304));
        assert_eq!(w34a02_read_be_u32(&[1, 2, 3]), None);
    }
}
