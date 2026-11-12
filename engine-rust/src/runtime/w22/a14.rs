#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent14.rs
// ===========================================================================
// Wave 22 agent 14: small guard predicates and the TilePage constructor.

/// `0x0065bf20`: TilePage constructor field init.  Dword layout of the
/// object: [0]=vftable (not modelled), [1..=3]=0, [8..=0xd]=0, [0xf]=2,
/// byte at dword 0x10 = 0.  The call at dword 4.. (sub-constructor) is
/// supplied by the caller through `init_sub`.
pub fn w22a14_tilepage_init(obj: &mut [u32; 0x11], init_sub: impl FnOnce(&mut [u32; 0x11])) {
    obj[1] = 0;
    obj[2] = 0;
    obj[3] = 0;
    init_sub(obj);
    for i in 8..=0xd {
        obj[i] = 0;
    }
    obj[0xf] = 2;
    obj[0x10] = (obj[0x10] & !0xff) as u32;
}

/// `0x0065f7a0`: returns `(result, must_abort)`; the retail code calls a
/// non-returning handler when `flag_a == 0 && flag_b != 0`, else returns 1.
pub fn w22a14_guard_flags(flag_a: u8, flag_b: u8) -> (u32, bool) {
    if flag_a == 0 && flag_b != 0 {
        (0, true)
    } else {
        (1, false)
    }
}

/// `0x00658e14`: true when the table entry at `idx & 0xffff` has bit 1
/// clear (the retail code then calls a non-returning error routine).
pub fn w22a14_entry_bit1_clear(table: &[u8], idx: u32) -> bool {
    table
        .get((idx & 0xffff) as usize)
        .is_some_and(|v| v & 2 == 0)
}

/// `0x00667040`: true when both arguments are non-zero (the non-returning
/// path); otherwise the retail code calls the error routine with code 0x57.
pub fn w22a14_both_nonzero(a: i32, b: i32) -> (bool, u32) {
    if a != 0 && b != 0 {
        (true, 0)
    } else {
        (false, 0x57)
    }
}

/// `0x006586a0`: true when the pointer-sized member is non-zero (then the
/// non-returning release call with code 0x7d04 is made).
pub fn w22a14_member_set(member: i32) -> Option<u32> {
    if member != 0 { Some(0x7d04) } else { None }
}

/// `0x00658f10`: true when the probe result is non-zero (non-returning
/// call follows).
pub fn w22a14_probe_nonzero(probe: i32) -> bool {
    probe != 0
}

#[cfg(test)]
mod tests_wave22_agent14 {
    use super::*;

    #[test]
    fn tilepage() {
        let mut o = [0xffff_ffffu32; 0x11];
        w22a14_tilepage_init(&mut o, |o| o[4] = 7);
        assert_eq!(o[1..4], [0, 0, 0]);
        assert_eq!(o[4], 7);
        assert_eq!(o[8..=0xd], [0; 6]);
        assert_eq!(o[0xf], 2);
        assert_eq!(o[0x10], 0xffff_ff00);
    }

    #[test]
    fn guards() {
        assert_eq!(w22a14_guard_flags(0, 1), (0, true));
        assert_eq!(w22a14_guard_flags(1, 1), (1, false));
        assert_eq!(w22a14_guard_flags(0, 0), (1, false));
        assert!(w22a14_entry_bit1_clear(&[0, 2], 0));
        assert!(!w22a14_entry_bit1_clear(&[0, 2], 1));
        assert!(!w22a14_entry_bit1_clear(&[0], 0x10000 + 5));
        assert_eq!(w22a14_both_nonzero(1, 1), (true, 0));
        assert_eq!(w22a14_both_nonzero(1, 0), (false, 0x57));
        assert_eq!(w22a14_member_set(0), None);
        assert_eq!(w22a14_member_set(3), Some(0x7d04));
        assert!(w22a14_probe_nonzero(1) && !w22a14_probe_nonzero(0));
    }
}
