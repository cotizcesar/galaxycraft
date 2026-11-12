#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent06.rs
// ===========================================================================
// Wave 32 agent 06 translations.

/// `0x006f96f0`: picks the argument forwarded to the follow-up routine:
/// `0x20` when the byte flag at offset `0x10350` of the state block is set,
/// `0x12` when it is zero.
pub fn w32a06_select_mode(flag: u8) -> u32 {
    if flag == 0 { 0x12 } else { 0x20 }
}

/// `0x0071e120`: converts the 64-bit counter (low, high dwords at `+0x10`
/// and `+0x14` of the record) to microsecond-style units by dividing by
/// 1,000,000; the result is stored at `+0x80` of the target.
pub fn w32a06_div_micro(lo: u32, hi: u32) -> u64 {
    let v = ((hi as u64) << 32) | lo as u64;
    v / 1_000_000
}

#[cfg(test)]
mod tests_wave32_agent06 {
    use super::*;

    #[test]
    fn mode() {
        assert_eq!(w32a06_select_mode(0), 0x12);
        assert_eq!(w32a06_select_mode(1), 0x20);
    }

    #[test]
    fn div() {
        assert_eq!(w32a06_div_micro(2_000_000, 0), 2);
        assert_eq!(w32a06_div_micro(0, 1), (1u64 << 32) / 1_000_000);
        assert_eq!(w32a06_div_micro(999_999, 0), 0);
    }
}
