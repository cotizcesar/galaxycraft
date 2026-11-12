#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent103.rs
// ===========================================================================
// Wave 32 agent 103 translations.

/// `0x00b364f0`: file tell.  `handle` is the field at offset 0x28; when it is
/// not -1 the seek callee (called with offset 0, origin 1) is queried and its
/// result returned unless it is -1; otherwise 0.
pub fn w32a103_tell(handle: i32, mut seek: impl FnMut(i32, i32, i32, i32) -> i32) -> i64 {
    if handle != -1 {
        let r = seek(handle, 0, 0, 1);
        if r != -1 {
            return r as i64;
        }
    }
    0
}

/// `0x00b40d90`: runs the cleanup callee, then clears the dword at offset 4.
pub fn w32a103_reset(field4: &mut u32, mut cleanup: impl FnMut()) {
    cleanup();
    *field4 = 0;
}

/// `0x00b4efb0`: forwards four params plus the constant `0xc0000000` to the
/// first callee, then runs the second callee.
pub fn w32a103_open_generic_write(
    params: [u32; 4],
    mut first: impl FnMut(u32, u32, u32, u32, u32),
    mut second: impl FnMut(),
) {
    first(params[0], params[1], params[2], params[3], 0xc000_0000);
    second();
}

#[cfg(test)]
mod tests_wave32_agent103 {
    use super::*;

    #[test]
    fn tell() {
        assert_eq!(w32a103_tell(-1, |_, _, _, _| 5), 0);
        assert_eq!(w32a103_tell(3, |h, a, b, c| h + a + b + c), 4);
        assert_eq!(w32a103_tell(3, |_, _, _, _| -1), 0);
    }

    #[test]
    fn reset() {
        let (mut f, mut n) = (9u32, 0);
        w32a103_reset(&mut f, || n += 1);
        assert_eq!((f, n), (0, 1));
    }

    #[test]
    fn open() {
        let mut got = None;
        let mut done = false;
        w32a103_open_generic_write(
            [1, 2, 3, 4],
            |a, b, c, d, e| got = Some((a, b, c, d, e)),
            || done = true,
        );
        assert_eq!(got, Some((1, 2, 3, 4, 0xc000_0000)));
        assert!(done);
    }
}
