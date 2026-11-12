#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent46.rs
// ===========================================================================
// Wave 33 agent 46: printf-style format handler fragments (CRT glue).

/// `0x00c99c39`: if the probe `ready` (callee at `0xc9_9cba`, argument 0)
/// succeeds, updates the counter at ctx+0x18: incremented when
/// `check(flag_byte)` is true, else set to -1.  Returns whether `ready` passed.
pub fn w33a46_step_counter(
    counter: &mut i32,
    flag_byte: u8,
    ready: impl FnOnce(u8) -> bool,
    check: impl FnOnce(u8) -> bool,
) -> bool {
    if !ready(0) {
        return false;
    }
    if check(flag_byte) {
        *counter = counter.wrapping_add(1);
    } else {
        *counter = -1;
    }
    true
}

/// Result of the `%s` handler: text pointer, length and wide flag.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W33a46Str {
    pub text: u32,
    pub len: u32,
    pub wide: bool,
}

/// `0x00c9a966`: string argument handler.  A missing argument (or one with
/// a null data pointer) yields `"(null)"` with length 6; otherwise length is
/// the u16 header, halved when `is_wide` (callee `0xc9_8db5`) holds.
/// `null_ptr` is the address standing for the "(null)" literal.
pub fn w33a46_string_arg(arg: Option<(u16, u32)>, null_ptr: u32, is_wide: bool) -> W33a46Str {
    match arg {
        Some((hdr, data)) if data != 0 => W33a46Str {
            text: data,
            len: if is_wide {
                (hdr >> 1) as u32
            } else {
                hdr as u32
            },
            wide: is_wide,
        },
        _ => W33a46Str {
            text: null_ptr,
            len: 6,
            wide: false,
        },
    }
}

/// `0x00c9ace6`: single-character argument handler.  Narrow mode stores the
/// low byte into `buf[0]` with length 1; wide mode converts via `convert`
/// (callees `0xc9_8c85` / `0xca_ed87`) and returns (len, error flag).
/// Returns `(len, error)`.
pub fn w33a46_char_arg(
    buf: &mut [u8],
    value: u16,
    is_wide: bool,
    convert: impl FnOnce(&mut [u8], u16) -> Option<u32>,
) -> (u32, bool) {
    if !is_wide {
        if let Some(b) = buf.first_mut() {
            *b = value as u8;
        }
        (1, false)
    } else {
        match convert(buf, value) {
            Some(n) => (n, false),
            None => (0, true),
        }
    }
}

#[cfg(test)]
mod tests_wave33_agent46 {
    use super::*;

    #[test]
    fn counter() {
        let mut c = 5;
        assert!(w33a46_step_counter(&mut c, 1, |_| true, |_| true));
        assert_eq!(c, 6);
        assert!(w33a46_step_counter(&mut c, 1, |_| true, |_| false));
        assert_eq!(c, -1);
        assert!(!w33a46_step_counter(&mut c, 1, |_| false, |_| true));
    }

    #[test]
    fn string() {
        assert_eq!(w33a46_string_arg(None, 9, true).len, 6);
        assert_eq!(w33a46_string_arg(Some((10, 4)), 9, true).len, 5);
        assert_eq!(w33a46_string_arg(Some((10, 4)), 9, false).len, 10);
        assert_eq!(w33a46_string_arg(Some((10, 0)), 9, false).text, 9);
    }

    #[test]
    fn chr() {
        let mut b = [0u8; 4];
        assert_eq!(
            w33a46_char_arg(&mut b, 0x41, false, |_, _| None),
            (1, false)
        );
        assert_eq!(b[0], 0x41);
        assert_eq!(
            w33a46_char_arg(&mut b, 0x41, true, |_, _| Some(2)),
            (2, false)
        );
        assert_eq!(w33a46_char_arg(&mut b, 0x41, true, |_, _| None), (0, true));
    }
}
