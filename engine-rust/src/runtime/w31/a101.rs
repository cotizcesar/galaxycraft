#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent101.rs
// ===========================================================================
// Wave 31 translation batch 101: vendor GPU capability tier helpers.

/// Result of the capability probe shared by the tier helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W31a101Probe {
    pub tier: u8,
    pub flag_0x147: u8,
}

/// Picks the tier (7..=1) from the probe answers, in priority order:
/// feature 0x22, 0x15, the 0x10008 query, 0xc, 0xe, then 1.
pub fn w31a101_tier(f22: bool, f15: bool, q10008: bool, f0c: bool, f0e: bool, f01: bool) -> u8 {
    if f22 {
        7
    } else if f15 {
        6
    } else if q10008 {
        5
    } else if f0c {
        4
    } else if f0e {
        3
    } else if f01 {
        2
    } else {
        1
    }
}

/// `0x00c274e0`: stores the probed tier at +0x7c and sets bit 0x8000 of the
/// flags word at +0x14c.  Returns `(tier, new_flags)`.
pub fn w31a101_probe_tier_a(
    f22: bool,
    f15: bool,
    q: bool,
    f0c: bool,
    f0e: bool,
    f01: bool,
    flags: u32,
) -> (u8, u32) {
    (w31a101_tier(f22, f15, q, f0c, f0e, f01), flags | 0x8000)
}

/// `0x00c27600`: same probe as the sibling, then remaps byte +0x147: when it
/// is 2 and tier > 4, tiers 5..=6 set it to 1 and tier 7 keeps it at 2.
pub fn w31a101_probe_tier_b(
    f22: bool,
    f15: bool,
    q: bool,
    f0c: bool,
    f0e: bool,
    f01: bool,
    byte_0x147: u8,
) -> W31a101Probe {
    let tier = w31a101_tier(f22, f15, q, f0c, f0e, f01);
    let mut b = byte_0x147;
    if b == 2 && tier > 4 {
        if tier < 7 {
            b = 1;
        } else {
            b = 2;
        }
    }
    W31a101Probe {
        tier,
        flag_0x147: b,
    }
}

/// Growth result for the 0x14-byte element vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W31a101Push {
    Pushed,
    /// Needs reallocation; carries the updated capacity word.
    Grow(u32),
}

/// `0x00c29760`: appends a zeroed 5-dword element.  The capacity word keeps
/// the capacity in its low 31 bits; when full, it grows by max(1, cap/2).
pub fn w31a101_push_zero(items: &mut Vec<[u32; 5]>, cap_word: u32) -> W31a101Push {
    let cap = cap_word & 0x7fff_ffff;
    if items.len() as u32 == cap && cap < cap.wrapping_add(1) {
        let half = (cap_word >> 1) & 0x3fff_ffff;
        let add = if cap < cap + half { half } else { 1 };
        return W31a101Push::Grow(((cap + add) ^ cap_word) & 0x7fff_ffff ^ cap_word);
    }
    items.push([0; 5]);
    W31a101Push::Pushed
}

#[cfg(test)]
mod tests_wave31_agent101 {
    use super::*;

    #[test]
    fn tier_priority() {
        assert_eq!(w31a101_tier(true, true, true, true, true, true), 7);
        assert_eq!(w31a101_tier(false, false, true, true, false, false), 5);
        assert_eq!(w31a101_tier(false, false, false, false, false, true), 2);
        assert_eq!(w31a101_tier(false, false, false, false, false, false), 1);
    }

    #[test]
    fn probe_a_sets_flag() {
        assert_eq!(
            w31a101_probe_tier_a(false, true, false, false, false, false, 1),
            (6, 0x8001)
        );
    }

    #[test]
    fn probe_b_remap() {
        let p = w31a101_probe_tier_b(false, true, false, false, false, false, 2);
        assert_eq!(
            p,
            W31a101Probe {
                tier: 6,
                flag_0x147: 1
            }
        );
        let p = w31a101_probe_tier_b(true, false, false, false, false, false, 2);
        assert_eq!(p.flag_0x147, 2);
        let p = w31a101_probe_tier_b(false, false, false, true, false, false, 2);
        assert_eq!(p.flag_0x147, 2);
    }

    #[test]
    fn push_zero_paths() {
        let mut v = vec![[1; 5]];
        assert_eq!(w31a101_push_zero(&mut v, 2), W31a101Push::Pushed);
        assert_eq!(v[1], [0; 5]);
        assert_eq!(w31a101_push_zero(&mut v, 2), W31a101Push::Grow(3));
        assert_eq!(v.len(), 2);
    }
}
