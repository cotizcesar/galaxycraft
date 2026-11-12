#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent77.rs
// ===========================================================================
// Wave 32 translation batch 77.
//
// CRT-style stream helpers: open, seek and bit-reader byte fill.

/// Stream state used by the `0x009e40d0` / `0x009e4260` helpers (`this`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct W32a77Stream {
    /// Underlying handle (0 = none).
    pub handle: i32,
    /// Current position.
    pub pos: u32,
    /// Memory-mode flag (byte at dword index 0x11).
    pub mem_mode: bool,
}

/// `0x009e40d0`: opens a stream through `open`; stores handle, clears
/// position, returns 0 on success or 0x17 when the open failed.
pub fn w32a77_open(s: &mut W32a77Stream, open: impl FnOnce() -> i32) -> u32 {
    let h = open();
    s.pos = 0;
    s.handle = h;
    if h != 0 { 0 } else { 0x17 }
}

/// `0x009e4260`: seek to `target`. 0x21 when no handle; in memory mode a
/// target before the current position is 0x25, otherwise `skip(delta)`
/// result; else `seek` returning 0 updates the position, failure is 0x14.
pub fn w32a77_seek(
    s: &mut W32a77Stream,
    target: u32,
    skip: impl FnOnce(u32) -> u32,
    seek: impl FnOnce(i32, u32) -> i32,
) -> u32 {
    if s.handle == 0 {
        return 0x21;
    }
    if s.mem_mode {
        if target < s.pos {
            return 0x25;
        }
        return skip(target - s.pos);
    }
    if seek(s.handle, target) == 0 {
        s.pos = target;
        0
    } else {
        0x14
    }
}

/// `0x009e4850`: fills `out` with bytes, each read as an 8-bit value.
pub fn w32a77_read_bytes(out: &mut [u8], mut read_bits: impl FnMut(u32) -> u8) {
    for b in out.iter_mut() {
        *b = read_bits(8);
    }
}

#[cfg(test)]
mod tests_wave32_agent77 {
    use super::*;

    #[test]
    fn open_codes() {
        let mut s = W32a77Stream {
            pos: 5,
            ..Default::default()
        };
        assert_eq!(w32a77_open(&mut s, || 0), 0x17);
        assert_eq!(s.pos, 0);
        assert_eq!(w32a77_open(&mut s, || 3), 0);
        assert_eq!(s.handle, 3);
    }

    #[test]
    fn seek_paths() {
        let mut s = W32a77Stream::default();
        assert_eq!(w32a77_seek(&mut s, 4, |_| 0, |_, _| 0), 0x21);
        s.handle = 1;
        assert_eq!(w32a77_seek(&mut s, 4, |_| 0, |_, _| 0), 0);
        assert_eq!(s.pos, 4);
        assert_eq!(w32a77_seek(&mut s, 9, |_| 0, |_, _| 1), 0x14);
        s.mem_mode = true;
        assert_eq!(w32a77_seek(&mut s, 2, |_| 0, |_, _| 0), 0x25);
        assert_eq!(w32a77_seek(&mut s, 6, |d| d + 10, |_, _| 0), 12);
    }

    #[test]
    fn read_bytes() {
        let mut o = [0u8; 3];
        let mut n = 0;
        w32a77_read_bytes(&mut o, |b| {
            assert_eq!(b, 8);
            n += 1;
            n
        });
        assert_eq!(o, [1, 2, 3]);
    }
}
