#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent36.rs
// ===========================================================================
// Wave 32 agent 36: CRT float-truncate helper and case-insensitive compare.

/// `0x00c93a21`: x87 extended-mantissa truncation helper.  `m` holds the
/// words `m[0]` (low) and `m[1]` (high, bit 15 sign, bits 7..14 exponent,
/// low 7 bits mantissa).  `masks` is the 16-entry mask table and `index`
/// the word-index table from the retail image (supplied by the caller).
/// Returns 1 or 2 for the all-ones exponent case, 0 when nothing is cut,
/// -1 when bits were cleared (-1 plus one when the dropped bits were zero).
pub fn w32a36_truncate_mantissa(
    m: &mut [u16],
    shift: i32,
    masks: &[u16; 16],
    index: &[i32],
) -> i32 {
    let hi = m[1];
    let exp = ((hi >> 7) & 0xff) as u32;
    if exp == 0xff {
        return if (hi & 0x7f) == 0 && m[0] == 0 { 1 } else { 2 };
    }
    if (hi & 0x7fff) != 0 || m[0] != 0 {
        let v = (0x96u32.wrapping_sub(exp)).wrapping_sub(shift as u32);
        let s = v as u16 as i16;
        if s > 0 {
            if s > 0x17 {
                m[0] = 0;
                m[1] = hi & 0x8000;
                return -1;
            }
            let slot = index[(s >> 4) as usize] as usize;
            let mut cut = masks[(v & 0xf) as usize] & m[slot];
            m[slot] ^= cut;
            if (s >> 4) > 0 {
                cut |= m[0];
                m[0] = 0;
            }
            return i32::from(cut == 0) - 1;
        }
    }
    0
}

/// `0x00c9c490`: ASCII case-insensitive compare of two NUL-terminated
/// byte strings; returns the difference of the first differing lowered bytes.
pub fn w32a36_stricmp(a: &[u8], b: &[u8]) -> i32 {
    let low = |c: u32| {
        if c.wrapping_sub(0x41) < 0x1a {
            c + 0x20
        } else {
            c
        }
    };
    let mut i = 0;
    loop {
        let x = low(*a.get(i).unwrap_or(&0) as u32);
        let y = low(*b.get(i).unwrap_or(&0) as u32);
        i += 1;
        if x == 0 || x != y {
            return x.wrapping_sub(y) as i32;
        }
    }
}

#[cfg(test)]
mod tests_wave32_agent36 {
    use super::*;

    #[test]
    fn stricmp_basic() {
        assert_eq!(w32a36_stricmp(b"AbC\0", b"aBc\0"), 0);
        assert!(w32a36_stricmp(b"a\0", b"b\0") < 0);
        assert!(w32a36_stricmp(b"abc\0", b"ab\0") > 0);
    }

    #[test]
    fn truncate_cases() {
        let masks = [0xffffu16; 16];
        let idx = [1, 0];
        let mut m = [0u16, 0x7f80];
        assert_eq!(w32a36_truncate_mantissa(&mut m, 0, &masks, &idx), 1);
        let mut m = [5u16, 0x0080];
        assert_eq!(w32a36_truncate_mantissa(&mut m, 0x60, &masks, &idx), -1);
        assert_eq!(m, [0, 0]);
    }
}
