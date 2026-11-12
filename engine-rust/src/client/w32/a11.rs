#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent11.rs
// ===========================================================================
// Wave 32 translation batch 11.
//
// Skipped: FUN_00897b10 (forwards to an unmodelled callee).

/// `0x0087bc60`: inclusive span of a rectangle-like record reached through
/// the object's pointer at +4: `i16@+10 - i16@+6 + 1`.
pub fn w32a11_inclusive_span(inner: &[u8]) -> Option<i32> {
    let a = i16::from_le_bytes(inner.get(10..12)?.try_into().ok()?) as i32;
    let b = i16::from_le_bytes(inner.get(6..8)?.try_into().ok()?) as i32;
    Some(a - b + 1)
}

/// `0x0087bc70`: reads the two signed 16-bit fields at +4 and +6 of the
/// inner record into a pair of 32-bit values.
pub fn w32a11_read_origin(inner: &[u8]) -> Option<[i32; 2]> {
    let a = i16::from_le_bytes(inner.get(4..6)?.try_into().ok()?) as i32;
    let b = i16::from_le_bytes(inner.get(6..8)?.try_into().ok()?) as i32;
    Some([a, b])
}

#[cfg(test)]
mod tests_wave32_agent11 {
    use super::*;

    #[test]
    fn span() {
        let mut r = [0u8; 12];
        r[6..8].copy_from_slice(&5i16.to_le_bytes());
        r[10..12].copy_from_slice(&9i16.to_le_bytes());
        assert_eq!(w32a11_inclusive_span(&r), Some(5));
        assert_eq!(w32a11_inclusive_span(&r[..8]), None);
    }

    #[test]
    fn origin() {
        let mut r = [0u8; 8];
        r[4..6].copy_from_slice(&(-3i16).to_le_bytes());
        r[6..8].copy_from_slice(&7i16.to_le_bytes());
        assert_eq!(w32a11_read_origin(&r), Some([-3, 7]));
    }
}
