#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent45.rs
// ===========================================================================
// Wave 24 agent 45: small accessor helpers (object at `this`, byte slices).

/// `0x007cc690`: runs `action(arg)` only when both cached ids match.
pub fn w24a45_call_if_match(
    cur_a: i32,
    want_a: i32,
    cur_b: i32,
    want_b: i32,
    arg: u32,
    mut action: impl FnMut(u32),
) -> bool {
    if cur_a == want_a && cur_b == want_b {
        action(arg);
        return true;
    }
    false
}

fn w24a45_u32(obj: &[u8], off: usize) -> u32 {
    obj.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .unwrap_or(0)
}

/// `0x007ce8f0`: copies the two dwords at offsets `0x50` and `0x54`.
pub fn w24a45_get_pair_50(obj: &[u8]) -> [u32; 2] {
    [w24a45_u32(obj, 0x50), w24a45_u32(obj, 0x54)]
}

/// `0x007ce980`: returns the dword at offset `0x60`.
pub fn w24a45_get_field_60(obj: &[u8]) -> u32 {
    w24a45_u32(obj, 0x60)
}

/// `0x007ce960`: low byte of `lookup(field_0x60)` combined with the word at
/// offset `0x64` in the high half.
pub fn w24a45_pack_key(obj: &[u8], lookup: impl Fn(u32) -> u32) -> u32 {
    let hi = obj
        .get(0x64..0x66)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .unwrap_or(0) as u32;
    (lookup(w24a45_u32(obj, 0x60)) & 0xff) | (hi << 16)
}

/// `0x007cf3b0`: true when the callee result is non-zero.
pub fn w24a45_nonzero(v: i32) -> bool {
    v != 0
}

#[cfg(test)]
mod tests_wave24_agent45 {
    use super::*;

    #[test]
    fn match_call() {
        let mut got = 0;
        assert!(w24a45_call_if_match(1, 1, 2, 2, 7, |a| got = a));
        assert_eq!(got, 7);
        assert!(!w24a45_call_if_match(1, 1, 2, 3, 9, |a| got = a));
        assert_eq!(got, 7);
    }

    #[test]
    fn accessors() {
        let mut o = vec![0u8; 0x70];
        o[0x50..0x54].copy_from_slice(&5u32.to_le_bytes());
        o[0x54..0x58].copy_from_slice(&6u32.to_le_bytes());
        o[0x60..0x64].copy_from_slice(&0x1234u32.to_le_bytes());
        o[0x64..0x66].copy_from_slice(&0xabcdu16.to_le_bytes());
        assert_eq!(w24a45_get_pair_50(&o), [5, 6]);
        assert_eq!(w24a45_get_field_60(&o), 0x1234);
        assert_eq!(w24a45_pack_key(&o, |v| v + 0x100), 0xabcd_0034);
        assert_eq!(w24a45_get_field_60(&[]), 0);
        assert!(w24a45_nonzero(-1) && !w24a45_nonzero(0));
    }
}
