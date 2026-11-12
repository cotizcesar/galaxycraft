#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent111.rs
// ===========================================================================
/// `0x00c12d10`: size class for opcode `op` (default 1).
pub fn w32a111_size_a(op: u8) -> u32 {
    match op {
        0x39..=0x52 | 0x5f | 0x60 | 0x6d..=0x74 => 4,
        0x53 | 0x54 | 0x56 | 0x59 | 0x61 | 0x62 | 0x64 | 0x67 => 5,
        0x55 | 0x57 | 0x5a | 0x63 | 0x65 | 0x68 => 6,
        0x58 | 0x5b | 0x66 | 0x69 => 8,
        0x5c | 0x5d | 0x6a | 0x6b => 10,
        0x5e | 0x6c => 12,
        _ => 1,
    }
}

/// `0x00c12dc0`: size class for opcode `op` (default 1).
pub fn w32a111_size_b(op: u8) -> u32 {
    match op {
        0x39..=0x51 | 0x5f | 0x6f | 0x70 | 0x73 | 0x74 => 4,
        0x52 | 0x53 | 0x60 | 0x61 => 5,
        0x54 | 0x55 | 0x62 | 0x63 => 6,
        0x56 | 0x57 | 0x58 | 0x64 | 0x65 | 0x66 | 0x6d | 0x6e | 0x71 | 0x72 => 8,
        0x59 | 0x5a | 0x5b | 0x5c | 0x67 | 0x68 | 0x69 | 0x6a => 10,
        0x5d | 0x5e | 0x6b | 0x6c => 12,
        _ => 1,
    }
}

#[cfg(test)]
mod tests_wave32_agent111 {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(w32a111_size_a(0x38), 1);
        assert_eq!(w32a111_size_a(0x39), 4);
        assert_eq!(w32a111_size_a(0x52), 4);
        assert_eq!(w32a111_size_a(0x5e), 12);
        assert_eq!(w32a111_size_a(0x74), 4);
        assert_eq!(w32a111_size_a(0x75), 1);
        assert_eq!(w32a111_size_b(0x52), 5);
        assert_eq!(w32a111_size_b(0x72), 8);
        assert_eq!(w32a111_size_b(0x6c), 12);
        assert_eq!(w32a111_size_b(0x38), 1);
    }
}
