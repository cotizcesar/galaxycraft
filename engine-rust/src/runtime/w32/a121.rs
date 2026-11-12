#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent121.rs
// ===========================================================================
// Wave 32 agent 121: two member-forwarding helpers.

/// `0x00c6d090`: member forwarder.  Reads the dword at `this + 0xc` and
/// calls the callee (unresolved, supplied by the caller) with that value
/// and the two stack arguments.
pub fn w32a121_forward_field_0c<R>(
    this: &[u8],
    a: u32,
    b: u32,
    callee: impl FnOnce(u32, u32, u32) -> R,
) -> Option<R> {
    let f = u32::from_le_bytes(this.get(0xc..0x10)?.try_into().ok()?);
    Some(callee(f, a, b))
}

/// `0x00c6d100`: member forwarder.  Reads the dword at `this + 8` and
/// calls the callee (unresolved, supplied by the caller) with that value
/// and the two stack arguments.
pub fn w32a121_forward_field_08<R>(
    this: &[u8],
    a: u32,
    b: u32,
    callee: impl FnOnce(u32, u32, u32) -> R,
) -> Option<R> {
    let f = u32::from_le_bytes(this.get(8..12)?.try_into().ok()?);
    Some(callee(f, a, b))
}

#[cfg(test)]
mod tests_wave32_agent121 {
    use super::*;

    #[test]
    fn forwarders_read_expected_field() {
        let mut obj = [0u8; 16];
        obj[8..12].copy_from_slice(&0x1111u32.to_le_bytes());
        obj[12..16].copy_from_slice(&0x2222u32.to_le_bytes());
        assert_eq!(
            w32a121_forward_field_0c(&obj, 1, 2, |f, a, b| (f, a, b)),
            Some((0x2222, 1, 2))
        );
        assert_eq!(
            w32a121_forward_field_08(&obj, 3, 4, |f, a, b| (f, a, b)),
            Some((0x1111, 3, 4))
        );
        assert_eq!(
            w32a121_forward_field_0c(&obj[..15], 0, 0, |f, _, _| f),
            None
        );
    }
}
