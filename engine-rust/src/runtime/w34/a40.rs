#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent40.rs
// ===========================================================================
// Wave 34 translation batch 40.

/// `0x0093c410`: wide-char path basename.  Applies `wcsrchr` for `/`, then
/// `\`, then `:` in turn, each time restarting from one wide char past the
/// last match.  Returns the start index of the result within `path`.
pub fn w34a40_path_basename(path: &[u16]) -> usize {
    fn rchr(s: &[u16], from: usize, c: u16) -> Option<usize> {
        let end = s.iter().position(|&x| x == 0).unwrap_or(s.len());
        if from > end {
            return None;
        }
        s[from..end].iter().rposition(|&x| x == c).map(|i| i + from)
    }
    let mut start = 0usize;
    for c in [0x2f_u16, 0x5c, 0x3a] {
        if let Some(i) = rchr(path, start, c) {
            start = i + 1;
        }
    }
    start
}

/// `0x0087ca50`: builds the 8-byte argument block passed to the follow-up
/// call.  `obj` is the record at `this+4` (>= 12 bytes).  When the u16 at
/// `+0x3c` is 0xe, the u16 at `+0x64` becomes `[+0x60] + delta` (obj must
/// then be >= 0x66 bytes).  Returns `(first_dword, second_dword)`: the dword at
/// `+4`, and a dword whose low half is `delta + low16(dword@+4)` and whose
/// high half is the high half of the dword at `+8`.
pub fn w34a40_offset_block(obj: &mut [u8], delta: i16) -> (u32, u32) {
    let rd16 = |o: &[u8], i: usize| u16::from_le_bytes([o[i], o[i + 1]]);
    if rd16(obj, 0x3c) == 0xe {
        let v = (rd16(obj, 0x60) as i16).wrapping_add(delta) as u16;
        obj[0x64..0x66].copy_from_slice(&v.to_le_bytes());
    }
    let d4 = u32::from_le_bytes([obj[4], obj[5], obj[6], obj[7]]);
    let hi = rd16(obj, 10);
    let lo = (delta as u16).wrapping_add(d4 as u16);
    (d4, (hi as u32) << 16 | lo as u32)
}

#[cfg(test)]
mod tests_wave34_agent40 {
    use super::*;
    #[test]
    fn basename() {
        let s: Vec<u16> = "a/b\\c:d".encode_utf16().chain([0]).collect();
        assert_eq!(w34a40_path_basename(&s), 6);
        let s: Vec<u16> = "abc".encode_utf16().chain([0]).collect();
        assert_eq!(w34a40_path_basename(&s), 0);
    }
    #[test]
    fn block() {
        let mut o = vec![0u8; 0x70];
        o[0x3c] = 0xe;
        o[0x60] = 5;
        o[4] = 1;
        o[10] = 7;
        let (a, b) = w34a40_offset_block(&mut o, 2);
        assert_eq!(a, 1);
        assert_eq!(b, 0x0007_0003);
        assert_eq!(o[0x64], 7);
    }
}
