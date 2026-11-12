#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent46.rs
// ===========================================================================
// Wave 32 agent 46.

/// `0x006ae9a0`: maps a pointer into the 0x150-byte record table to a
/// derived value.  `ptr` is the value read through `this+0xc`; zero or a
/// pointer outside `[table_base, table_base + count * 0x150)` yields 0.
/// Otherwise returns `(ptr - table_base) / 0x150 * stride + base`, where
/// `stride`/`base` come from the record at `param_1 + 0x10c` (+0x20 / +0x18).
pub fn w32a46_record_index_value(
    ptr: u32,
    table_base: u32,
    count: i32,
    stride: i32,
    base: i32,
) -> i32 {
    if ptr == 0 {
        return 0;
    }
    let end = (count as u32).wrapping_mul(0x150).wrapping_add(table_base);
    if table_base <= ptr && ptr < end {
        let idx = (ptr.wrapping_sub(table_base) as i32) / 0x150;
        return idx.wrapping_mul(stride).wrapping_add(base);
    }
    0
}

#[cfg(test)]
mod tests_wave32_agent46 {
    use super::*;

    #[test]
    fn maps_index() {
        assert_eq!(w32a46_record_index_value(0, 0x1000, 4, 8, 3), 0);
        assert_eq!(
            w32a46_record_index_value(0x1000 + 0x150 * 2 + 5, 0x1000, 4, 8, 3),
            19
        );
        assert_eq!(
            w32a46_record_index_value(0x1000 + 0x150 * 4, 0x1000, 4, 8, 3),
            0
        );
        assert_eq!(w32a46_record_index_value(0xfff, 0x1000, 4, 8, 3), 0);
    }
}
