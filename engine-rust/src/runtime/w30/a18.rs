#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent18.rs
// ===========================================================================
// Wave 30 agent 18: tact memory-file helpers and open-flag mapping.

/// `0x00b35f10`: maps the caller's open mode to the native open flags: low
/// two bits 0/1 -> `0x8000`, 2 -> `0x8001`, 3 -> `0x8002`; bit 2 adds `0x100`.
pub fn w30a18_open_flags(mode: u32) -> u32 {
    let base = match mode & 3 {
        2 => 0x8001,
        3 => 0x8002,
        _ => 0x8000,
    };
    if mode & 4 != 0 { base | 0x100 } else { base }
}

/// `0x00b36a40`: memory-file read.  `buf` is the backing store, `pos` the
/// cursor (+0x10) and `len` the valid length (+0xc).  `want` is clamped to
/// the remaining bytes (0 at or past the end); returns the bytes to copy.
pub fn w30a18_mem_read<'a>(buf: &'a [u8], pos: usize, len: usize, want: &mut usize) -> &'a [u8] {
    if len <= pos {
        *want = 0;
        return &[];
    }
    let avail = len - pos;
    if avail < *want {
        *want = avail;
    }
    &buf[pos..pos + *want]
}

/// Outcome of [`w30a18_mem_write_check`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W30a18WriteStatus {
    /// Status 2: file is read-only (flag +0x14).
    ReadOnly,
    /// Status 0: nothing to write.
    Empty,
    /// Data fits inside the current length; copy at the cursor, status continues.
    Fits,
    /// Status 0xd: would need to grow but growth is forbidden (flag +0x15).
    Fixed,
    /// Buffer must be grown to at least this end offset before copying.
    Grow(usize),
}

/// `0x00b36b30`: memory-file write pre-check.
pub fn w30a18_mem_write_check(
    read_only: bool,
    no_grow: bool,
    pos: usize,
    len: usize,
    count: usize,
) -> W30a18WriteStatus {
    if read_only {
        return W30a18WriteStatus::ReadOnly;
    }
    if count == 0 {
        return W30a18WriteStatus::Empty;
    }
    let end = pos.wrapping_add(count);
    if end < len {
        return W30a18WriteStatus::Fits;
    }
    if no_grow {
        return W30a18WriteStatus::Fixed;
    }
    W30a18WriteStatus::Grow(end)
}

/// `0x00b38d20`: validates through `check`, then reports whether the state
/// byte (+0x90 of the inner object) equals 1.
pub fn w30a18_state_is_one(check: bool, state: u8) -> bool {
    check && state == 1
}

#[cfg(test)]
mod tests_wave30_agent18 {
    use super::*;

    #[test]
    fn open_flags() {
        assert_eq!(w30a18_open_flags(0), 0x8000);
        assert_eq!(w30a18_open_flags(1), 0x8000);
        assert_eq!(w30a18_open_flags(2), 0x8001);
        assert_eq!(w30a18_open_flags(7), 0x8102);
    }

    #[test]
    fn read_clamps() {
        let b = [1u8, 2, 3, 4];
        let mut n = 10;
        assert_eq!(w30a18_mem_read(&b, 1, 4, &mut n), &[2, 3, 4]);
        assert_eq!(n, 3);
        let mut n = 2;
        assert_eq!(w30a18_mem_read(&b, 0, 4, &mut n), &[1, 2]);
        let mut n = 2;
        assert!(w30a18_mem_read(&b, 4, 4, &mut n).is_empty());
        assert_eq!(n, 0);
    }

    #[test]
    fn write_check() {
        use W30a18WriteStatus::*;
        assert_eq!(w30a18_mem_write_check(true, false, 0, 8, 1), ReadOnly);
        assert_eq!(w30a18_mem_write_check(false, false, 0, 8, 0), Empty);
        assert_eq!(w30a18_mem_write_check(false, false, 0, 8, 7), Fits);
        assert_eq!(w30a18_mem_write_check(false, true, 0, 8, 8), Fixed);
        assert_eq!(w30a18_mem_write_check(false, false, 4, 8, 8), Grow(12));
    }

    #[test]
    fn state() {
        assert!(w30a18_state_is_one(true, 1));
        assert!(!w30a18_state_is_one(true, 0));
        assert!(!w30a18_state_is_one(false, 1));
    }
}
