#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent66.rs
// ===========================================================================
// Wave 32 agent 66 translations.

/// `0x00955c30`: maps a key to a byte.  When the feature flag is off it
/// fails; when `count < 2` the key's low byte is returned directly;
/// otherwise the 8-byte records (`u32` key, byte at +4) are searched and the
/// matching record's byte is returned.
pub fn w32a66_lookup_byte(flag: bool, key: u32, count: u32, table: &[[u8; 8]]) -> Option<u8> {
    if !flag {
        return None;
    }
    if count < 2 {
        return Some(key as u8);
    }
    table
        .iter()
        .take(0xfb)
        .find(|r| u32::from_le_bytes([r[0], r[1], r[2], r[3]]) == key)
        .map(|r| r[4])
}

#[cfg(test)]
mod tests_wave32_agent66 {
    use super::*;

    #[test]
    fn lookup() {
        let t = [[7, 0, 0, 0, 9, 0, 0, 0]];
        assert_eq!(w32a66_lookup_byte(false, 7, 5, &t), None);
        assert_eq!(w32a66_lookup_byte(true, 0x1ff, 1, &t), Some(0xff));
        assert_eq!(w32a66_lookup_byte(true, 7, 5, &t), Some(9));
        assert_eq!(w32a66_lookup_byte(true, 8, 5, &t), None);
    }
}
