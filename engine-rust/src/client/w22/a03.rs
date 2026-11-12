#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent03.rs
// ===========================================================================
// Wave 22 translation batch 03.
//
// Records are `0x4e8` bytes wide; the caller passes a byte slice that starts
// at address `0x00faa000` (the DAT base is an assumption of this helper).

/// Record stride of the table at `0x00faa000`.
pub const W22A03_STRIDE: usize = 0x4e8;
/// Offset of the byte read by the getter (`DAT_00faa41a`).
pub const W22A03_OFF_FLAG_A: usize = 0x41a;
/// Offset of the byte set by the setter (`DAT_00faa426`).
pub const W22A03_OFF_FLAG_B: usize = 0x426;
/// Offset of the record sub-block address (`DAT_00faa428`).
pub const W22A03_OFF_SUB: usize = 0x428;

/// `0x00635bc0`: sets the flag byte at +0x426 of record `index` to 1.
/// Returns false when out of range.
pub fn w22a03_set_flag(table: &mut [u8], index: usize) -> bool {
    match table.get_mut(index * W22A03_STRIDE + W22A03_OFF_FLAG_B) {
        Some(b) => {
            *b = 1;
            true
        }
        None => false,
    }
}

/// `0x00635f40`: returns the byte at +0x41a of record `index`.
pub fn w22a03_get_flag(table: &[u8], index: usize) -> Option<u8> {
    table
        .get(index * W22A03_STRIDE + W22A03_OFF_FLAG_A)
        .copied()
}

/// `0x00635f80`: returns the byte offset of the +0x428 sub-block of record
/// `index` inside the table.
pub fn w22a03_sub_offset(index: usize) -> usize {
    index * W22A03_STRIDE + W22A03_OFF_SUB
}

/// `0x00650c8f`: true only when the AL flag is non-zero and the callee
/// (`probe`) returns non-zero.
pub fn w22a03_flag_and_probe(flag: u8, probe: impl FnOnce() -> i32) -> bool {
    flag != 0 && probe() != 0
}

#[cfg(test)]
mod tests_wave22_agent03 {
    use super::*;

    #[test]
    fn flags() {
        let mut t = vec![0u8; W22A03_STRIDE * 3];
        assert!(w22a03_set_flag(&mut t, 2));
        assert_eq!(t[2 * 0x4e8 + 0x426], 1);
        assert!(!w22a03_set_flag(&mut t, 3));
        t[0x4e8 + 0x41a] = 7;
        assert_eq!(w22a03_get_flag(&t, 1), Some(7));
        assert_eq!(w22a03_get_flag(&t, 3), None);
    }

    #[test]
    fn sub_offset() {
        assert_eq!(w22a03_sub_offset(0), 0x428);
        assert_eq!(w22a03_sub_offset(2), 0x9d0 + 0x428);
    }

    #[test]
    fn probe() {
        assert!(!w22a03_flag_and_probe(0, || 1));
        assert!(!w22a03_flag_and_probe(1, || 0));
        assert!(w22a03_flag_and_probe(1, || 5));
    }
}
