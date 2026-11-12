#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent07.rs
// ===========================================================================
// Wave 24 translation batch 07: small client/save-file helpers.

/// `0x00726220`: lazily allocates the one-byte global buffer (when `buf` is
/// empty) and stores the constant `0x3c` in it.
pub fn w24a07_init_flag_byte(buf: &mut Vec<u8>) {
    if buf.is_empty() {
        buf.push(0);
    }
    buf[0] = 0x3c;
}

/// `0x00726ad0`: membership check of `wanted` against a table of eight bytes
/// plus a packed word of four bytes.  Returns `true` when the byte is present
/// (the retail function returns 0 and only aborts when it is absent and
/// `fatal_flag` is zero; a non-zero `fatal_flag` always aborts).
/// `Ok(())` means retail returns 0; `Err(())` stands for the abort path.
pub fn w24a07_validate_member(
    table8: &[u8; 8],
    packed: u32,
    wanted: u8,
    fatal_flag: i32,
) -> Result<(), ()> {
    if fatal_flag != 0 {
        return Err(());
    }
    let in_table = table8.contains(&wanted);
    let in_packed = packed.to_le_bytes().contains(&wanted);
    if in_table || in_packed {
        Ok(())
    } else {
        Err(())
    }
}

/// `0x00772c80`: validates a 32-bit header word read from a save stream.
/// Returns the high half when bit 16 is set and the high half lies in
/// `0x69..=0x6d`, otherwise `None`.
pub fn w24a07_header_word_kind(word: u32) -> Option<u32> {
    let hi = word >> 16;
    if hi & 1 != 0 && hi.wrapping_sub(0x69) < 5 {
        Some(hi)
    } else {
        None
    }
}

/// `0x00741b30`: skips an optional section of a save file.  Reads a u32 flag;
/// when non-zero reads two more u32s (the second is the entry count), then for
/// each entry reads a u32 length and seeks forward by it (relative).  Returns
/// the new position, or `None` ("File corrupt") on a short read or negative seek.
pub fn w24a07_skip_section(data: &[u8], mut pos: usize) -> Option<usize> {
    fn rd(data: &[u8], pos: &mut usize) -> Option<u32> {
        let b = data.get(*pos..pos.checked_add(4)?)?;
        *pos += 4;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    if rd(data, &mut pos)? == 0 {
        return Some(pos);
    }
    rd(data, &mut pos)?;
    let n = rd(data, &mut pos)?;
    for _ in 0..n {
        let v = rd(data, &mut pos)? as i32 as i64;
        let np = pos as i64 + v;
        if np < 0 {
            return None;
        }
        pos = np as usize;
    }
    Some(pos)
}

#[cfg(test)]
mod tests_wave24_agent07 {
    use super::*;

    #[test]
    fn flag_byte() {
        let mut b = Vec::new();
        w24a07_init_flag_byte(&mut b);
        assert_eq!(b, vec![0x3c]);
        b[0] = 1;
        w24a07_init_flag_byte(&mut b);
        assert_eq!(b[0], 0x3c);
    }

    #[test]
    fn member() {
        let t = [1, 2, 3, 4, 5, 6, 7, 8];
        assert!(w24a07_validate_member(&t, 0x0b0a0908, 9, 0).is_ok());
        assert!(w24a07_validate_member(&t, 0x0b0a0908, 3, 0).is_ok());
        assert!(w24a07_validate_member(&t, 0, 99, 0).is_err());
        assert!(w24a07_validate_member(&t, 0, 3, 1).is_err());
    }

    #[test]
    fn header() {
        assert_eq!(w24a07_header_word_kind(0x0069_0000), Some(0x69));
        assert_eq!(w24a07_header_word_kind(0x006a_0000), None);
        assert_eq!(w24a07_header_word_kind(0x006d_0000), Some(0x6d));
        assert_eq!(w24a07_header_word_kind(0x0001_0000), None);
    }

    #[test]
    fn skip() {
        let mut d = vec![];
        for v in [1u32, 0, 2, 4, 0, 0] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        assert_eq!(w24a07_skip_section(&d, 0), Some(24));
        assert_eq!(w24a07_skip_section(&0u32.to_le_bytes(), 0), Some(4));
        assert_eq!(w24a07_skip_section(&d[..10], 0), None);
    }
}
