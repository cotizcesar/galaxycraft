#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent12.rs
// ===========================================================================
// Wave 22 agent 12: small runtime helpers.

/// Stride of the per-player/slot record table read by `0x00635fa0`.
pub const W22A12_SLOT_STRIDE: usize = 0x4e8;
/// Offset of the u16 field inside a slot record (table base + 0x416 vs record base).
pub const W22A12_SLOT_FIELD_OFFSET: usize = 0x416;

/// `0x00635fa0`: returns the u16 at `table[index * 0x4e8 + 0x416]` (little endian),
/// or `None` when out of range.
pub fn w22a12_slot_u16(table: &[u8], index: usize) -> Option<u16> {
    let off = index
        .checked_mul(W22A12_SLOT_STRIDE)?
        .checked_add(W22A12_SLOT_FIELD_OFFSET)?;
    let b = table.get(off..off.checked_add(2)?)?;
    Some(u16::from_le_bytes([b[0], b[1]]))
}

/// `0x0063a703`: calls the caller-supplied helper and returns 1.
pub fn w22a12_call_then_true(mut helper: impl FnMut()) -> u32 {
    helper();
    1
}

/// Empty MSVC small string header: `[data_ptr, len, cap, inline bytes...]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W22a12EmptyString {
    pub data_ptr: u32,
    pub len: u32,
    pub cap: u32,
    pub inline0: u8,
}

/// `0x00649cc0` and `0x0064c130`: initialise an empty small string located at
/// address `self_addr`: pointer to its inline buffer (`self_addr + 12`), length 0,
/// capacity `0x8000000f`, first inline byte 0.  The trailing non-returning
/// call in the retail body is not modelled.
pub fn w22a12_init_empty_string(self_addr: u32) -> W22a12EmptyString {
    W22a12EmptyString {
        data_ptr: self_addr.wrapping_add(12),
        len: 0,
        cap: 0x8000_000f,
        inline0: 0,
    }
}

#[cfg(test)]
mod tests_wave22_agent12 {
    use super::*;

    #[test]
    fn slot_u16() {
        let mut t = vec![0u8; 2 * W22A12_SLOT_STRIDE];
        let o = W22A12_SLOT_STRIDE + W22A12_SLOT_FIELD_OFFSET;
        t[o] = 0x34;
        t[o + 1] = 0x12;
        assert_eq!(w22a12_slot_u16(&t, 1), Some(0x1234));
        assert_eq!(w22a12_slot_u16(&t, 2), None);
        assert_eq!(w22a12_slot_u16(&t, usize::MAX), None);
    }

    #[test]
    fn call_true() {
        let mut n = 0;
        assert_eq!(w22a12_call_then_true(|| n += 1), 1);
        assert_eq!(n, 1);
    }

    #[test]
    fn empty_string() {
        let s = w22a12_init_empty_string(0x100);
        assert_eq!(
            (s.data_ptr, s.len, s.cap, s.inline0),
            (0x10c, 0, 0x8000_000f, 0)
        );
    }
}
