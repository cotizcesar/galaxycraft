#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent09.rs
// ===========================================================================
// Wave 29 translation batch 09: blz filebuf field layout helpers.

/// Model of the `blz::basic_filebuf<char>` fields touched by the routines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct W29a09Filebuf {
    /// Fields `[1..=6]` (get/put area pointers).
    pub area: [u32; 6],
    /// Field 7: opened file handle.
    pub file: u32,
    /// Byte at +0x20: file owned flag.
    pub file_owned: bool,
    /// Field 9: buffer pointer (+0x24).
    pub buf: u32,
    /// Byte at +0x2d: buffer owned flag.
    pub buf_owned: bool,
    /// Field 10: buffer size (+0x28).
    pub buf_size: u32,
}

/// `0x00ac6ad0`: filebuf constructor.  Clears the area and file fields; if the
/// close/probe call returns other than -1, frees an owned buffer through
/// `free_buf` and installs a fresh owned buffer of size `0x1000`.
pub fn w29a09_filebuf_new(probe: i32, mut free_buf: impl FnMut(u32)) -> W29a09Filebuf {
    let mut f = W29a09Filebuf::default();
    if probe != -1 {
        if f.buf_owned {
            free_buf(f.buf);
        }
        f.buf = 0;
        f.buf_owned = true;
        f.buf_size = 0x1000;
        f.area = [0; 6];
    }
    f
}

/// `0x00ac6bb0`: filebuf destructor.  If a file is open, resets the get area
/// (when set), releases the file via `close_file` only if owned, then frees an
/// owned buffer with `free_buf`.
pub fn w29a09_filebuf_drop(
    f: &mut W29a09Filebuf,
    mut close_file: impl FnMut(u32),
    mut free_buf: impl FnMut(u32),
) {
    if f.file != 0 {
        if f.area[0] != 0 {
            f.area[0] = 0;
            f.area[1] = 0;
            f.area[2] = 0;
        }
        if f.file_owned {
            close_file(f.file);
        }
        f.file = 0;
        f.file_owned = false;
    }
    if f.buf_owned {
        free_buf(f.buf);
    }
}

/// `0x00ac6c50`: frees the pointer at offset `+0x84` when non-null.
pub fn w29a09_free_field_84(ptr: u32, mut free: impl FnMut(u32)) {
    if ptr != 0 {
        free(ptr);
    }
}

#[cfg(test)]
mod tests_wave29_agent09 {
    use super::*;

    #[test]
    fn new_sets_buffer() {
        let f = w29a09_filebuf_new(0, |_| panic!());
        assert_eq!((f.buf_size, f.buf_owned), (0x1000, true));
        let g = w29a09_filebuf_new(-1, |_| panic!());
        assert_eq!(g, W29a09Filebuf::default());
    }

    #[test]
    fn drop_frees_owned() {
        let mut f = W29a09Filebuf {
            file: 5,
            file_owned: true,
            area: [1, 2, 3, 0, 0, 0],
            buf: 9,
            buf_owned: true,
            buf_size: 1,
        };
        let (mut c, mut b) = (0, 0);
        w29a09_filebuf_drop(&mut f, |h| c = h, |p| b = p);
        assert_eq!((c, b, f.file, f.area[0]), (5, 9, 0, 0));
    }

    #[test]
    fn free_84() {
        let mut n = 0;
        w29a09_free_field_84(0, |_| n += 1);
        w29a09_free_field_84(7, |_| n += 1);
        assert_eq!(n, 1);
    }
}
