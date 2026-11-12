#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent09.rs
// ===========================================================================
// Wave 22 agent 09: small wrappers recovered from the 0x60xxxx range.

/// Argument block built by the wrappers `0x00608520` and `0x00608630`
/// before calling the shared logger-style helper (flags word is always 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W22a09Args {
    pub words: [u32; 5],
}

/// Result of the wrapper: string length (without NUL), selector constant,
/// flags (always 0) and the copied argument block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W22a09Call {
    pub len: usize,
    pub selector: u32,
    pub flags: u32,
    pub args: W22a09Args,
}

/// `0x00608520`: measures the C string, copies two words from `a`, one
/// from `b` and one from `c` (slots 0,1,2,4 of the block) and calls the
/// helper with selector `0x23d`.
pub fn w22a09_wrap_23d(text: &[u8], a: [u32; 2], b: u32, c: u32) -> W22a09Call {
    let len = text.iter().position(|&x| x == 0).unwrap_or(text.len());
    W22a09Call {
        len,
        selector: 0x23d,
        flags: 0,
        args: W22a09Args {
            words: [a[0], a[1], b, 0, c],
        },
    }
}

/// `0x00608630`: like the previous wrapper with selector `0x3d` and a
/// three-word block (two words from `a`, one from `b`).
pub fn w22a09_wrap_3d(text: &[u8], a: [u32; 2], b: u32) -> W22a09Call {
    let len = text.iter().position(|&x| x == 0).unwrap_or(text.len());
    W22a09Call {
        len,
        selector: 0x3d,
        flags: 0,
        args: W22a09Args {
            words: [a[0], a[1], b, 0, 0],
        },
    }
}

/// State written by `0x00603f50`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W22a09Globals {
    pub g_3964: u32,
    pub g_3960: u32,
    pub w_b51e: u32,
    pub w_b522: u32,
}

/// `0x00603f50`: stores both parameters, clears the word at offset
/// `b51e`, stores the value returned by the timer helper at `b522`.
pub fn w22a09_set_globals(p1: u32, p2: u32, helper_ret: u32) -> W22a09Globals {
    W22a09Globals {
        g_3964: p1,
        g_3960: p2,
        w_b51e: 0,
        w_b522: helper_ret,
    }
}

#[cfg(test)]
mod tests_wave22_agent09 {
    use super::*;

    #[test]
    fn wrappers() {
        let c = w22a09_wrap_23d(b"abc\0def", [1, 2], 3, 4);
        assert_eq!((c.len, c.selector, c.flags), (3, 0x23d, 0));
        assert_eq!(c.args.words, [1, 2, 3, 0, 4]);
        let d = w22a09_wrap_3d(b"", [5, 6], 7);
        assert_eq!((d.len, d.selector), (0, 0x3d));
        assert_eq!(d.args.words[..3], [5, 6, 7]);
    }

    #[test]
    fn globals() {
        let g = w22a09_set_globals(1, 2, 9);
        assert_eq!((g.g_3964, g.g_3960, g.w_b51e, g.w_b522), (1, 2, 0, 9));
    }
}
