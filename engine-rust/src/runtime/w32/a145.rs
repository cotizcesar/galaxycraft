#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent145.rs
// ===========================================================================
// Wave 32 agent 145: CRT stream flush.

/// CRT `FILE` model: `ptr`, `base`, `cnt`, `flag` (words 0, 1, 2, 3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W32a145File {
    pub ptr: i32,
    pub base: i32,
    pub cnt: i32,
    pub flag: u32,
}

/// `0x00ca4e05`: CRT `_flush`.  When the stream is write-mode (flag & 3 == 2)
/// and buffered (flag & 0xc0 != 0), rewinds the pointer to the base, clears the
/// count and writes the pending bytes through `write`; a short write sets the
/// error bit 0x10 and returns -1, a full write clears the 0x02 bit when bit 0x04
/// is set.  Returns 0 otherwise.
pub fn w32a145_flush(f: &mut W32a145File, write: &mut dyn FnMut(i32, i32) -> i32) -> i32 {
    if (f.flag & 3) == 2 && (f.flag & 0xc0) != 0 {
        let base = f.base;
        let pending = f.ptr.wrapping_sub(base);
        f.ptr = base;
        f.cnt = 0;
        if pending > 0 {
            if write(base, pending) != pending {
                f.flag |= 0x10;
                return -1;
            }
            if (f.flag >> 2) & 1 != 0 {
                f.flag &= 0xffff_fffd;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests_wave32_agent145 {
    use super::*;

    #[test]
    fn flush_ok_and_short() {
        let mut f = W32a145File {
            ptr: 110,
            base: 100,
            cnt: 5,
            flag: 0x4e,
        };
        assert_eq!(w32a145_flush(&mut f, &mut |_, n| n), 0);
        assert_eq!((f.ptr, f.cnt, f.flag), (100, 0, 0x4c));
        let mut g = W32a145File {
            ptr: 110,
            base: 100,
            cnt: 5,
            flag: 0x42,
        };
        assert_eq!(w32a145_flush(&mut g, &mut |_, _| 3), -1);
        assert_eq!(g.flag & 0x10, 0x10);
        let mut h = W32a145File {
            ptr: 1,
            base: 0,
            cnt: 1,
            flag: 1,
        };
        assert_eq!(w32a145_flush(&mut h, &mut |_, _| 0), 0);
        assert_eq!(h.cnt, 1);
    }
}
