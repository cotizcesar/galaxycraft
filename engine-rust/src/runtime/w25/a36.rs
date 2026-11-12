#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent36.rs
// ===========================================================================
/// `0x0086a5d0`: copies the two dwords of `src` into the object's fields at
/// offsets `0xc` and `0x10`.
pub fn w25a36_set_pair(obj: &mut [u8], src: [u32; 2]) {
    obj[0xc..0x10].copy_from_slice(&src[0].to_le_bytes());
    obj[0x10..0x14].copy_from_slice(&src[1].to_le_bytes());
}

/// `0x0086ae90`: stores a byte at offset `0x54` of the object.
pub fn w25a36_set_byte_54(obj: &mut [u8], v: u8) {
    obj[0x54] = v;
}

/// `0x00869c60`: if the flag byte at offset `0x18` is set, clears it and
/// returns true (the retail code then tail-calls a release helper, which the
/// caller must perform); otherwise returns false.
pub fn w25a36_clear_flag_18(obj: &mut [u8]) -> bool {
    if obj[0x18] != 0 {
        obj[0x18] = 0;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests_wave25_agent36 {
    use super::*;

    #[test]
    fn pair_and_byte() {
        let mut o = [0u8; 0x60];
        w25a36_set_pair(&mut o, [1, 0x01020304]);
        assert_eq!(o[0xc], 1);
        assert_eq!(&o[0x10..0x14], &[4, 3, 2, 1]);
        w25a36_set_byte_54(&mut o, 9);
        assert_eq!(o[0x54], 9);
    }

    #[test]
    fn clear_flag() {
        let mut o = [0u8; 0x20];
        assert!(!w25a36_clear_flag_18(&mut o));
        o[0x18] = 1;
        assert!(w25a36_clear_flag_18(&mut o));
        assert_eq!(o[0x18], 0);
    }
}
