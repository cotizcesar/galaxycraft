#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent41.rs
// ===========================================================================
// Wave 33 agent 41: CRT ctype table lookup and 4-char literal matcher.

/// `0x00c936d8`: ctype table lookup.  Valid indices are -1..=0x100 (the
/// unsigned test `index + 1 < 0x101`); the table holds 16-bit class masks.
/// Returns `table[index] & mask`, or 0 when out of range.
pub fn w33a41_ctype_mask(table: &[u16], index: i32, mask: u16) -> u16 {
    if (index.wrapping_add(1) as u32) < 0x101 {
        let i = index as isize;
        // Index -1 reads the element before the table base; model it as 0.
        if i < 0 {
            return 0;
        }
        return table.get(i as usize).copied().unwrap_or(0) & mask;
    }
    0
}

/// `0x00c95411`: matches up to four characters of a stream against two
/// 4-byte tables (upper/lower case spellings).  `current` is the character
/// slot (`*param_1`), `next` supplies the following stream character.
/// Returns true when all four positions matched.
pub fn w33a41_match_four(
    current: &mut u8,
    upper: &[u8; 4],
    lower: &[u8; 4],
    mut next: impl FnMut() -> u8,
) -> bool {
    let mut i = 0usize;
    while *current == upper[i] || *current == lower[i] {
        *current = next();
        i += 1;
        if i == 4 {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests_wave33_agent41 {
    use super::*;

    #[test]
    fn mask_range() {
        let t = [0x0Fu16; 0x101];
        assert_eq!(w33a41_ctype_mask(&t, 5, 0x3), 3);
        assert_eq!(w33a41_ctype_mask(&t, 0x101, 0x3), 0);
        assert_eq!(w33a41_ctype_mask(&t, -2, 0x3), 0);
        assert_eq!(w33a41_ctype_mask(&t, -1, 0x3), 0);
    }

    #[test]
    fn match_four() {
        let mut it = b"RUE!".iter().copied();
        let mut c = b'T';
        assert!(w33a41_match_four(&mut c, b"TRUE", b"true", || it
            .next()
            .unwrap()));
        let mut c = b'x';
        assert!(!w33a41_match_four(&mut c, b"TRUE", b"true", || 0));
    }
}
