#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent01.rs
// ===========================================================================
// Wave 30 agent 01: VFS manifest reader field accessors (pure helpers).

/// Failure of a manifest bounds check (the retail code raises a logged error).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W30a01Err {
    /// Offset outside the manifest window.
    OutOfRange,
    /// Unterminated or malformed field.
    Corrupt,
}

/// File-table entry kind (`0x00af35f0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W30a01Entry {
    /// Kind 1 with the record count (byte value `1..=0xe0`).
    Records(u32),
    /// Marker `0xff`.
    Kind2,
    /// Marker `0xfe`.
    Kind3,
    /// Marker `0xfd`.
    Kind4,
}

/// `0x00af35f0`: classifies the entry byte at `off` inside the window
/// `data[base..base+len]`.  Markers `0xff/0xfe/0xfd` map to kinds 2/3/4;
/// values above `0xe0` are invalid; otherwise the entry needs
/// `stride * count + 1` bytes after `off` (`stride` is the result of the
/// unrecovered helper, passed in).
pub fn w30a01_classify_entry(
    data: &[u8],
    base: usize,
    len: u32,
    off: u32,
    stride: u32,
) -> Result<W30a01Entry, W30a01Err> {
    if off >= len {
        return Err(W30a01Err::OutOfRange);
    }
    let b = *data.get(base + off as usize).ok_or(W30a01Err::OutOfRange)? as u32;
    match b {
        0xff => Ok(W30a01Entry::Kind2),
        0xfe => Ok(W30a01Entry::Kind3),
        0xfd => Ok(W30a01Entry::Kind4),
        _ if b > 0xe0 => Err(W30a01Err::Corrupt),
        _ => {
            if len < stride.wrapping_mul(b).wrapping_add(1).wrapping_add(off) {
                Err(W30a01Err::OutOfRange)
            } else {
                Ok(W30a01Entry::Records(b))
            }
        }
    }
}

/// One record: 8 key bytes (two big-endian dwords) plus a size field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W30a01Record {
    /// Low dword: bytes 1..=4 big-endian.
    pub lo: u32,
    /// High dword: bytes 5..=8 big-endian.
    pub hi: u32,
    /// Size field, width chosen by `size_max`.
    pub size: u32,
}

/// `0x00af3fe0`: reads record `idx` of the entry at `off`.  The size field
/// is 1, 2, 3 or 4 bytes (big-endian) when `size_max` is `< 0x100`,
/// `< 0x10000`, `< 0x1000000` or larger.  `stride` is the per-record
/// stride (unrecovered helper result).
pub fn w30a01_read_record(
    data: &[u8],
    base: usize,
    len: u32,
    size_max: u32,
    off: u32,
    idx: u32,
    stride: u32,
) -> Result<W30a01Record, W30a01Err> {
    if len.wrapping_add(base as u32) <= (base as u32).wrapping_add(off) {
        return Err(W30a01Err::OutOfRange);
    }
    let count = *data.get(base + off as usize).ok_or(W30a01Err::OutOfRange)? as u32;
    if count > 0xe0 || count <= idx {
        return Err(W30a01Err::Corrupt);
    }
    if len < stride.wrapping_mul(count).wrapping_add(1).wrapping_add(off) {
        return Err(W30a01Err::OutOfRange);
    }
    let p = base + (off as usize) + (idx as usize) * (stride as usize);
    let g = |i: usize| data.get(p + i).copied().ok_or(W30a01Err::OutOfRange);
    let be = |n: usize| -> Result<u32, W30a01Err> {
        let mut v = 0u32;
        for i in 0..n {
            v = (v << 8) | g(9 + i)? as u32;
        }
        Ok(v)
    };
    let lo = u32::from_be_bytes([g(1)?, g(2)?, g(3)?, g(4)?]);
    let hi = u32::from_be_bytes([g(5)?, g(6)?, g(7)?, g(8)?]);
    let size = match size_max {
        0..=0xff => be(1)?,
        0x100..=0xffff => be(2)?,
        0x1_0000..=0xff_ffff => be(3)?,
        _ => be(4)?,
    };
    Ok(W30a01Record { lo, hi, size })
}

/// `0x00af3330`: length of the NUL-terminated string at `off` inside the
/// window `data[base..base+len]`.
pub fn w30a01_cstr_len(data: &[u8], base: usize, len: u32, off: u32) -> Result<u32, W30a01Err> {
    if off >= len {
        return Err(W30a01Err::OutOfRange);
    }
    let mut i = off;
    while i < len {
        match data.get(base + i as usize) {
            Some(0) => break,
            Some(_) => i += 1,
            None => return Err(W30a01Err::OutOfRange),
        }
    }
    if i == len {
        return Err(W30a01Err::Corrupt);
    }
    Ok(i - off)
}

/// `0x00af3820`: validates a `0xfe`-tagged length-prefixed block at `off`
/// and returns `length_byte + 1`.
pub fn w30a01_tagged_len(data: &[u8], base: usize, len: u32, off: u32) -> Result<u32, W30a01Err> {
    let end = len as u64 + base as u64;
    let p = off as u64 + base as u64;
    if end < p + 2 {
        return Err(W30a01Err::OutOfRange);
    }
    if data.get(p as usize) != Some(&0xfe) {
        return Err(W30a01Err::Corrupt);
    }
    let n = *data.get(p as usize + 1).ok_or(W30a01Err::OutOfRange)? as u64;
    if end < n + 3 + p {
        return Err(W30a01Err::OutOfRange);
    }
    Ok(n as u32 + 1)
}

/// `0x00af43d0`: succeeds when `off + 4` fits in `len`.
pub fn w30a01_check_u32_fits(len: u32, off: u32) -> Result<(), W30a01Err> {
    if len < off.wrapping_add(4) {
        Err(W30a01Err::OutOfRange)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests_wave30_agent01 {
    use super::*;

    #[test]
    fn classify() {
        let d = [0xff, 0xfe, 0xfd, 0xe1, 2, 0, 0, 0];
        assert_eq!(
            w30a01_classify_entry(&d, 0, 8, 0, 9),
            Ok(W30a01Entry::Kind2)
        );
        assert_eq!(
            w30a01_classify_entry(&d, 0, 8, 1, 9),
            Ok(W30a01Entry::Kind3)
        );
        assert_eq!(
            w30a01_classify_entry(&d, 0, 8, 2, 9),
            Ok(W30a01Entry::Kind4)
        );
        assert_eq!(
            w30a01_classify_entry(&d, 0, 8, 3, 9),
            Err(W30a01Err::Corrupt)
        );
        assert_eq!(
            w30a01_classify_entry(&d, 0, 8, 8, 1),
            Err(W30a01Err::OutOfRange)
        );
        assert_eq!(
            w30a01_classify_entry(&d, 0, 8, 4, 1),
            Ok(W30a01Entry::Records(2))
        );
        assert_eq!(
            w30a01_classify_entry(&d, 0, 8, 4, 4),
            Err(W30a01Err::OutOfRange)
        );
    }

    #[test]
    fn record() {
        let d = [1u8, 1, 2, 3, 4, 5, 6, 7, 8, 0x12, 0x34];
        let r = w30a01_read_record(&d, 0, 11, 0x200, 0, 0, 10).unwrap();
        assert_eq!((r.lo, r.hi, r.size), (0x01020304, 0x05060708, 0x1234));
        let r = w30a01_read_record(&d, 0, 11, 0x20, 0, 0, 10).unwrap();
        assert_eq!(r.size, 0x12);
        assert_eq!(
            w30a01_read_record(&d, 0, 11, 0x20, 0, 1, 10),
            Err(W30a01Err::Corrupt)
        );
    }

    #[test]
    fn strings_and_bounds() {
        let d = *b"ab\0cd";
        assert_eq!(w30a01_cstr_len(&d, 0, 5, 0), Ok(2));
        assert_eq!(w30a01_cstr_len(&d, 0, 5, 3), Err(W30a01Err::Corrupt));
        assert_eq!(w30a01_cstr_len(&d, 0, 5, 5), Err(W30a01Err::OutOfRange));
        let t = [0xfe, 3, 0, 0, 0, 0];
        assert_eq!(w30a01_tagged_len(&t, 0, 6, 0), Ok(4));
        assert_eq!(w30a01_tagged_len(&t, 0, 5, 0), Err(W30a01Err::OutOfRange));
        assert_eq!(
            w30a01_tagged_len(&[1, 0, 0], 0, 3, 0),
            Err(W30a01Err::Corrupt)
        );
        assert!(w30a01_check_u32_fits(8, 4).is_ok());
        assert!(w30a01_check_u32_fits(7, 4).is_err());
    }
}
