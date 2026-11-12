#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent15.rs
// ===========================================================================
// Wave 32 agent 15.

/// `0x008c6cd0`: stores two dwords at object offsets `0x1d8` and `0x1dc`,
/// then runs the follow-up refresh callback (FUN_008c6c30).
pub fn w32a15_set_pair_and_refresh(
    obj: &mut [u8],
    a: u32,
    b: u32,
    refresh: &mut dyn FnMut(&mut [u8]),
) -> bool {
    if obj.len() < 0x1e0 {
        return false;
    }
    obj[0x1d8..0x1dc].copy_from_slice(&a.to_le_bytes());
    obj[0x1dc..0x1e0].copy_from_slice(&b.to_le_bytes());
    refresh(obj);
    true
}

#[cfg(test)]
mod tests_wave32_agent15 {
    use super::*;

    #[test]
    fn sets_fields_and_refreshes() {
        let mut o = vec![0u8; 0x200];
        let mut n = 0;
        assert!(w32a15_set_pair_and_refresh(&mut o, 1, 2, &mut |_| n += 1));
        assert_eq!(n, 1);
        assert_eq!(o[0x1d8], 1);
        assert_eq!(o[0x1dc], 2);
        assert!(!w32a15_set_pair_and_refresh(&mut [0; 4], 1, 2, &mut |_| {}));
    }
}
