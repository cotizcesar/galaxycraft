#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave18/agent05.rs
// ===========================================================================
// Wave 18 translation batch 05: stream-read helpers and small constructors.

/// Layout of the string-holder object initialised by `0x005352b0` and
/// `0x005353a0` (dword indices into the object).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W18a05StringHolder {
    /// dwords 0..=2 cleared.
    pub head: [u32; 3],
    /// byte at +0x0c cleared.
    pub flag: u8,
    /// dword 5: pointer to inline buffer (object offset 0x20).
    pub data_offset: u32,
    /// dword 6: current length.
    pub len: u32,
    /// dword 7: capacity marker `0x8000000f` (inline SSO capacity 15).
    pub cap: u32,
    /// byte at +0x20: inline buffer first byte, cleared.
    pub inline0: u8,
}

fn w18a05_holder_init() -> W18a05StringHolder {
    W18a05StringHolder {
        head: [0; 3],
        flag: 0,
        data_offset: 0x20,
        len: 0,
        cap: 0x8000_000f,
        inline0: 0,
    }
}

/// `0x005353a0`: constructs an empty string holder (the trailing assign of
/// length 0 is a non-returning call and not modelled).
pub fn w18a05_holder_new_empty() -> W18a05StringHolder {
    w18a05_holder_init()
}

/// `0x005352b0`: constructs a string holder from a NUL-terminated string;
/// returns the holder and the length handed to the assign call
/// (0 when the first byte is NUL).
pub fn w18a05_holder_new_from_cstr(s: &[u8]) -> (W18a05StringHolder, usize) {
    let n = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    (w18a05_holder_init(), n)
}

/// `0x00534f30`: copy length for a C string, clamped to 0x103 bytes.
pub fn w18a05_clamped_len(s: &[u8]) -> usize {
    let n = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    n.min(0x103)
}

/// `0x00535250`: zeroes a two-dword record.
pub fn w18a05_zero_pair(rec: &mut [u32; 2]) {
    *rec = [0, 0];
}

/// `0x00535110`: reads `count` dwords through `read(offset, len) -> bytes`
/// (0 = EOF), looping until the total is a multiple of 4.  Returns the dword
/// count, or the "Premature end of stream" error.
pub fn w18a05_read_dwords(
    count: usize,
    mut read: impl FnMut(usize, usize) -> usize,
) -> Result<usize, &'static str> {
    let want = count * 4;
    let mut got = read(0, want);
    while got & 3 != 0 {
        let n = read(got, want.wrapping_sub(got));
        if n == 0 {
            return Err("Premature end of stream");
        }
        got += n;
    }
    Ok(got >> 2)
}

/// `0x005351c0`: reads one 12-byte record; loops until the byte count is a
/// multiple of 12, then requires at least 12 bytes.
pub fn w18a05_read_record12(
    mut read: impl FnMut(usize, usize) -> usize,
) -> Result<(), &'static str> {
    let mut got = read(0, 12);
    while got != (got / 12) * 12 {
        let n = read(got, 12 - got.min(12));
        if n == 0 {
            return Err("Premature end of stream");
        }
        got += n;
    }
    if got > 11 {
        Ok(())
    } else {
        Err("Premature end of stream")
    }
}

/// `0x00535000`: scans 12-byte elements for the first whose dword at +8 is
/// non-negative; `None` when the range is exhausted.
pub fn w18a05_first_nonneg_at8(rows: &[u8]) -> Option<usize> {
    rows.as_chunks::<12>()
        .0
        .iter()
        .position(|r| i32::from_le_bytes([r[8], r[9], r[10], r[11]]) >= 0)
}

/// Scalar deleting-destructor epilogue shared by `0x00535c30`, `0x00535c60`,
/// `0x00535c90`, `0x00535cf0`, `0x00535d50` and `0x00535d80`: the object is
/// freed only when bit 0 of the flag is set.
pub fn w18a05_should_free(flag: u8) -> bool {
    flag & 1 != 0
}

#[cfg(test)]
mod tests_wave18_agent05 {
    use super::*;

    #[test]
    fn holder_and_len() {
        let h = w18a05_holder_new_empty();
        assert_eq!(h.cap, 0x8000_000f);
        assert_eq!(h.data_offset, 0x20);
        assert_eq!(w18a05_holder_new_from_cstr(b"abc\0").1, 3);
        assert_eq!(w18a05_holder_new_from_cstr(b"\0").1, 0);
        assert_eq!(w18a05_clamped_len(&vec![b'a'; 400]), 0x103);
        assert_eq!(w18a05_clamped_len(b"hi\0xx"), 2);
        let mut p = [5, 6];
        w18a05_zero_pair(&mut p);
        assert_eq!(p, [0, 0]);
    }

    #[test]
    fn reads() {
        assert_eq!(w18a05_read_dwords(2, |_, l| l), Ok(2));
        let mut calls = vec![6usize, 2];
        assert_eq!(w18a05_read_dwords(2, |_, _| calls.remove(0)), Ok(2));
        let mut c = vec![5usize, 0];
        assert!(w18a05_read_dwords(2, |_, _| c.remove(0)).is_err());
        assert_eq!(w18a05_read_record12(|_, l| l), Ok(()));
        let mut c = vec![5usize, 7];
        assert_eq!(w18a05_read_record12(|_, _| c.remove(0)), Ok(()));
        let mut c = vec![5usize, 0];
        assert!(w18a05_read_record12(|_, _| c.remove(0)).is_err());
    }

    #[test]
    fn scan_and_free() {
        let mut rows = vec![0u8; 24];
        rows[8..12].copy_from_slice(&(-1i32).to_le_bytes());
        assert_eq!(w18a05_first_nonneg_at8(&rows), Some(1));
        rows[20..24].copy_from_slice(&(-5i32).to_le_bytes());
        rows[8..12].copy_from_slice(&(-1i32).to_le_bytes());
        assert_eq!(w18a05_first_nonneg_at8(&rows), None);
        assert!(w18a05_should_free(1) && w18a05_should_free(3));
        assert!(!w18a05_should_free(0) && !w18a05_should_free(2));
    }
}
