#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent112.rs
// ===========================================================================
// Wave 32 batch 112.

/// `0x00c12eb0`: looks up a 32-bit entry in the dword table at
/// `DAT_00f493e8`, indexed by a byte.  The table is caller-supplied.
pub fn w32a112_table_lookup(table: &[u32], index: u8) -> Option<u32> {
    table.get(index as usize).copied()
}

/// `0x00c222c0`: forwards to the helper with `(base + 0xc, base + 2)`
/// (pointer-sized offsets in the original; computed here as address
/// arithmetic).  Returns the two argument values passed on.
pub fn w32a112_forward_args(base: u32) -> (u32, u32) {
    (base.wrapping_add(0xc), base.wrapping_add(2))
}

#[cfg(test)]
mod tests_wave32_agent112 {
    use super::*;

    #[test]
    fn lookup() {
        let t = [10u32, 20, 30];
        assert_eq!(w32a112_table_lookup(&t, 1), Some(20));
        assert_eq!(w32a112_table_lookup(&t, 9), None);
    }

    #[test]
    fn forward() {
        assert_eq!(w32a112_forward_args(0x100), (0x10c, 0x102));
    }
}
