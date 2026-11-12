#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent47.rs
// ===========================================================================
// Wave 32 agent 47 translations.

/// `0x00715ef0`: stores the 16-bit table entry `table[idx]` at offset `0x3e`
/// of the record and ORs `flags` into the 32-bit field at offset `0x30`.
/// Returns false when the record is too short or the index is out of range.
pub fn w32a47_set_entry_and_flags(rec: &mut [u8], table: &[u16], idx: u16, flags: u32) -> bool {
    if rec.len() < 0x42 {
        return false;
    }
    let Some(&v) = table.get(idx as usize) else {
        return false;
    };
    rec[0x3e..0x40].copy_from_slice(&v.to_le_bytes());
    let cur = u32::from_le_bytes([rec[0x30], rec[0x31], rec[0x32], rec[0x33]]);
    rec[0x30..0x34].copy_from_slice(&(cur | flags).to_le_bytes());
    true
}

#[cfg(test)]
mod tests_wave32_agent47 {
    #[test]
    fn sets_fields() {
        let mut r = [0u8; 0x48];
        r[0x30] = 1;
        assert!(super::w32a47_set_entry_and_flags(
            &mut r,
            &[7, 0x1234],
            1,
            4
        ));
        assert_eq!(r[0x30], 5);
        assert_eq!(&r[0x3e..0x40], &[0x34, 0x12]);
        assert!(!super::w32a47_set_entry_and_flags(&mut r, &[7], 3, 0));
    }
}
