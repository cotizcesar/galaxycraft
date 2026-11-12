#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent39.rs
// ===========================================================================
// Wave 25 agent 39 batch (small container / bounds helpers).

/// `0x00873420`: length of the NUL-terminated string stored at offset
/// `0x0b` of the record (0 when empty); `rec` is the record bytes.
pub fn w25a39_name_len(rec: &[u8]) -> usize {
    rec.get(0xb..)
        .map(|s| s.iter().position(|&b| b == 0).unwrap_or(s.len()))
        .unwrap_or(0)
}

/// `0x008738f0`: byte-sized index check; values above 7 take the
/// failure path (`Err(value)`), otherwise the call proceeds and returns 1.
pub fn w25a39_check_index_u8(v: u32) -> Result<u32, u32> {
    let b = v & 0xff;
    if b > 7 { Err(b) } else { Ok(1) }
}

/// `0x00873330`: returns 1; the failure path is taken only when the
/// global flag is zero (`Err(())`).
pub fn w25a39_flag_gate(flag: u32) -> Result<u32, ()> {
    if flag == 0 { Err(()) } else { Ok(1) }
}

/// `0x00872c10`: scans 0x28-byte records in `[0, count)` and returns the
/// first whose i32 at offset 8 is non-negative.
pub fn w25a39_first_nonneg(rows: &[u8], count: usize) -> Option<usize> {
    (0..count).find(|&i| {
        rows.get(i * 0x28 + 8..i * 0x28 + 12)
            .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]) >= 0)
            .unwrap_or(false)
    })
}

/// `0x00875c80`: grow policy of the 0x28-stride vector. Header word keeps
/// the top bit; low 31 bits are capacity. Returns the new word when
/// `len == capacity`, else `None`.
pub fn w25a39_grow_word(len: u32, word: u32) -> Option<u32> {
    let cap = word & 0x7fff_ffff;
    if len != cap || cap == cap.wrapping_add(1) || cap > cap.wrapping_add(1) {
        return None;
    }
    let step = (word >> 1) & 0x3fff_ffff;
    let add = if cap.wrapping_add(1) <= cap.wrapping_add(step) {
        step
    } else {
        1
    };
    let newcap = add.wrapping_add(cap);
    Some(((newcap ^ word) & 0x7fff_ffff) ^ word)
}

/// `0x00872e50`: copies the first five dwords of a record.
pub fn w25a39_copy5(dst: &mut [u32; 5], src: &[u32; 5]) {
    *dst = *src;
}

#[cfg(test)]
mod tests_wave25_agent39 {
    use super::*;
    #[test]
    fn name_len() {
        let mut r = [0u8; 0x20];
        assert_eq!(w25a39_name_len(&r), 0);
        r[0xb..0xe].copy_from_slice(b"abc");
        assert_eq!(w25a39_name_len(&r), 3);
        assert_eq!(w25a39_name_len(&[1, 2]), 0);
    }
    #[test]
    fn index_and_gate() {
        assert_eq!(w25a39_check_index_u8(7), Ok(1));
        assert_eq!(w25a39_check_index_u8(0x108), Err(8));
        assert_eq!(w25a39_flag_gate(0), Err(()));
        assert_eq!(w25a39_flag_gate(1), Ok(1));
    }
    #[test]
    fn scan() {
        let mut v = vec![0u8; 0x28 * 3];
        v[8..12].copy_from_slice(&(-1i32).to_le_bytes());
        v[0x28 + 8..0x28 + 12].copy_from_slice(&(-5i32).to_le_bytes());
        assert_eq!(w25a39_first_nonneg(&v, 3), Some(2));
        assert_eq!(w25a39_first_nonneg(&v, 2), None);
    }
    #[test]
    fn grow() {
        assert_eq!(w25a39_grow_word(3, 4), None);
        assert_eq!(w25a39_grow_word(4, 4), Some(6));
        assert_eq!(w25a39_grow_word(8, 0x8000_0008), Some(0x8000_000c));
    }
    #[test]
    fn copy() {
        let mut d = [0; 5];
        w25a39_copy5(&mut d, &[1, 2, 3, 4, 5]);
        assert_eq!(d, [1, 2, 3, 4, 5]);
    }
}
