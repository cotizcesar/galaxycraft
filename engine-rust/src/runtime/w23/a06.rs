#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent06.rs
// ===========================================================================
// Wave 23 translation batch 06: small runtime/record helpers.

/// `0x006e28e0`: push-back of a `u16` into a Microsoft-style vector
/// (`size` at +4, capacity word at +8 with the top bit as a flag).  Returns
/// `true` if pushed; `false` when the vector is full (the retail code then
/// tail-calls the grow routine, which is not modelled).
pub fn w23a06_vec_u16_push(data: &mut [u16], size: &mut u32, cap_word: u32, value: u16) -> bool {
    let cap = cap_word & 0x7fff_ffff;
    if *size == cap && cap < cap.wrapping_add(1) {
        return false;
    }
    match data.get_mut(*size as usize) {
        Some(slot) => {
            *slot = value;
            *size += 1;
            true
        }
        None => false,
    }
}

/// `0x006e2c40`: true when the global flag is clear and the record byte at
/// +0x4c equals the current player id byte.
pub fn w23a06_owner_matches(flag: u32, record_byte_4c: u8, current: u8) -> bool {
    flag == 0 && record_byte_4c == current
}

/// `0x006e3e70`: rounds the value returned by the callee down to a multiple
/// of 1000.
pub fn w23a06_round_down_1000(value: u32) -> u32 {
    (value / 1000) * 1000
}

/// `0x006e3f10`: byte at +0x57 of the 0xb0-stride record, zero-extended
/// (upper bytes come from the table pointer in retail; modelled as 0).
pub fn w23a06_record_byte_57(table: &[u8], index: usize) -> Option<u32> {
    table.get(index * 0xb0 + 0x57).map(|&b| u32::from(b))
}

/// `0x006e3f30`: whether the byte at +0x56 of the 0xb0-stride record is
/// non-zero.
pub fn w23a06_record_flag_56(table: &[u8], index: usize) -> Option<bool> {
    table.get(index * 0xb0 + 0x56).map(|&b| b != 0)
}

/// `0x006e5be0`: reads the first dword through a pointer.
pub fn w23a06_first_dword(p: &[u32]) -> Option<u32> {
    p.first().copied()
}

/// `0x006e5c00`: builds a flag word from a message header (`kind` at +0,
/// `arg` at +1, `word` at +2); `online` ORs in 0x80.
pub fn w23a06_message_flags(kind: u8, arg: u8, word: u16, online: bool) -> u32 {
    let mut v = 0u32;
    if kind == 9 {
        if word == 2 {
            v = 0x10;
        }
    } else if kind == 0x80 && arg != 0 {
        let hi = u32::from(arg) << 24;
        v = hi | 0x80_0020;
        if word == 2 {
            v = hi | 0x80_0030;
        }
    }
    if online {
        v |= 0x80;
    }
    v
}

#[cfg(test)]
mod tests_wave23_agent06 {
    use super::*;

    #[test]
    fn push() {
        let mut d = [0u16; 2];
        let mut n = 0;
        assert!(w23a06_vec_u16_push(&mut d, &mut n, 2, 7));
        assert_eq!((d[0], n), (7, 1));
        n = 2;
        assert!(!w23a06_vec_u16_push(&mut d, &mut n, 2, 1));
    }

    #[test]
    fn small_predicates() {
        assert!(w23a06_owner_matches(0, 3, 3));
        assert!(!w23a06_owner_matches(1, 3, 3));
        assert_eq!(w23a06_round_down_1000(2999), 2000);
        let mut t = vec![0u8; 0xb0 * 2];
        t[0xb0 + 0x57] = 9;
        t[0xb0 + 0x56] = 1;
        assert_eq!(w23a06_record_byte_57(&t, 1), Some(9));
        assert_eq!(w23a06_record_flag_56(&t, 1), Some(true));
        assert_eq!(w23a06_record_flag_56(&t, 0), Some(false));
        assert_eq!(w23a06_record_flag_56(&t, 5), None);
        assert_eq!(w23a06_first_dword(&[4]), Some(4));
    }

    #[test]
    fn flags() {
        assert_eq!(w23a06_message_flags(9, 0, 2, false), 0x10);
        assert_eq!(w23a06_message_flags(9, 0, 1, true), 0x80);
        assert_eq!(w23a06_message_flags(0x80, 2, 2, false), 0x0280_0030);
        assert_eq!(w23a06_message_flags(0x80, 2, 0, false), 0x0280_0020);
        assert_eq!(w23a06_message_flags(0x80, 0, 2, false), 0);
    }
}
