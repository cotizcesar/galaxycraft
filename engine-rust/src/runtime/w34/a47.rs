#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent47.rs
// ===========================================================================
// Wave 34 translation batch 47: block-compressed texture size helpers.

/// `0x00c12990`: true when `format` is 4 or 5.
pub fn w34a47_is_format_4_or_5(format: u8) -> bool {
    format.wrapping_sub(4) < 2
}

/// `0x00c12ec0`: size in bytes for `count` rows given a format code; the
/// divisor groups come from the format switch (4, 5, 6, 8, 10, 12, else 1).
pub fn w34a47_rows_size(format: u8, mult: i32, extent: u32) -> i32 {
    let d: u32 = match format {
        0x39..=0x52 | 0x5f | 0x60 | 0x6d..=0x74 => 4,
        0x53 | 0x54 | 0x56 | 0x59 | 0x61 | 0x62 | 0x64 | 0x67 => 5,
        0x55 | 0x57 | 0x5a | 0x63 | 0x65 | 0x68 => 6,
        0x58 | 0x5b | 0x66 | 0x69 => 8,
        0x5c | 0x5d | 0x6a | 0x6b => 10,
        0x5e | 0x6c => 12,
        _ => 1,
    };
    let add = if d == 1 { 0 } else { d - 1 };
    ((extent.wrapping_add(add)) / d).wrapping_mul(mult as u32) as i32
}

/// `0x00c12fd0`: block-rounded size; `table` models the per-format int
/// table (indexed by format), `extent` is the dimension to round up.
pub fn w34a47_block_size(format: u8, extent: i32, table: &[i32]) -> i32 {
    let d: u32 = match format {
        0x39..=0x51 | 0x5f | 0x6f | 0x70 | 0x73 | 0x74 => 4,
        0x52 | 0x53 | 0x60 | 0x61 => 5,
        0x54 | 0x55 | 0x62 | 0x63 => 6,
        0x56..=0x58 | 0x64 | 0x65 | 0x66 | 0x6d | 0x6e | 0x71 | 0x72 => 8,
        0x59..=0x5c | 0x67..=0x6a => 10,
        0x5d | 0x5e | 0x6b | 0x6c => 12,
        _ => 1,
    };
    let entry = table.get(format as usize).copied().unwrap_or(0);
    ((extent.wrapping_sub(1) as u32).wrapping_add(d) / d).wrapping_mul(entry as u32) as i32
}

#[cfg(test)]
mod tests_wave34_agent47 {
    use super::*;

    #[test]
    fn flag() {
        assert!(w34a47_is_format_4_or_5(4) && w34a47_is_format_4_or_5(5));
        assert!(!w34a47_is_format_4_or_5(3) && !w34a47_is_format_4_or_5(6));
    }

    #[test]
    fn rows() {
        assert_eq!(w34a47_rows_size(0x39, 2, 5), 4);
        assert_eq!(w34a47_rows_size(0x5e, 3, 13), 6);
        assert_eq!(w34a47_rows_size(0, 3, 7), 21);
    }

    #[test]
    fn block() {
        let mut t = vec![0; 256];
        t[0x39] = 8;
        t[1] = 3;
        assert_eq!(w34a47_block_size(0x39, 5, &t), 16);
        assert_eq!(w34a47_block_size(1, 5, &t), 15);
    }
}
