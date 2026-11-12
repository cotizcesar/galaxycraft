#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent14.rs
// ===========================================================================
// Wave 29 translation batch 14 (staging).

/// Cached stream window of the file-backed reader (offsets from patch 1.23.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W29a14Window {
    /// Offset `0x44`: number of valid bytes in the buffer.
    pub len: u32,
    /// Offsets `0x68`/`0x6c`: stream position of the first buffered byte.
    pub pos: u64,
}

/// `0x00ad8170`: moves the buffered window to stream position `new_pos`.
/// When the new position lies strictly inside the buffered range the tail is
/// shifted down with `memmove(shift_from, shift_len)` (offset, count); in all
/// cases where the position differs the cached position is replaced and the
/// valid length reset to 0.
pub fn w29a14_seek_window(
    w: &mut W29a14Window,
    new_pos: u64,
    mut memmove: impl FnMut(usize, usize),
) {
    if w.pos == new_pos {
        return;
    }
    let end = w.pos + u64::from(w.len);
    if w.pos < new_pos && new_pos < end {
        let delta = (new_pos - w.pos) as usize;
        memmove(delta, w.len as usize - delta);
    }
    w.pos = new_pos;
    w.len = 0;
}

/// `0x00ad81f0`: post-processes an I/O result code.  Code 5 becomes 6 unless
/// flag `0x400` is set; every other code passes through unchanged.
pub fn w29a14_map_result(code: i32, flags: u32) -> i32 {
    if code == 5 && flags & 0x400 == 0 {
        6
    } else {
        code
    }
}

#[cfg(test)]
mod tests_wave29_agent14 {
    use super::*;

    #[test]
    fn seek_window() {
        let mut w = W29a14Window { len: 10, pos: 100 };
        let mut calls = vec![];
        w29a14_seek_window(&mut w, 100, |a, b| calls.push((a, b)));
        assert!(calls.is_empty() && w.len == 10);
        w29a14_seek_window(&mut w, 104, |a, b| calls.push((a, b)));
        assert_eq!(calls, vec![(4, 6)]);
        assert_eq!(w, W29a14Window { len: 0, pos: 104 });
        let mut w = W29a14Window { len: 10, pos: 100 };
        w29a14_seek_window(&mut w, 110, |_, _| panic!());
        assert_eq!(w.pos, 110);
    }

    #[test]
    fn map_result() {
        assert_eq!(w29a14_map_result(5, 0), 6);
        assert_eq!(w29a14_map_result(5, 0x400), 5);
        assert_eq!(w29a14_map_result(6, 0), 6);
        assert_eq!(w29a14_map_result(0x16, 0), 0x16);
        assert_eq!(w29a14_map_result(0, 0), 0);
    }
}
