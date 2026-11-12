#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent133.rs
// ===========================================================================
// Wave 32 translation batch 133: CRT printf conversion dispatch/prefix.

/// Conversion kind selected by the printf-style formatter dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W32a133Conv {
    /// Integer via the radix helper: (radix, uppercase/hex-flag arg).
    Int {
        radix: u32,
        arg: u32,
        set_signed: bool,
    },
    Float,
    Char,
    WideChar,
    Str,
    Pointer,
    CountN,
    OctalO,
}

/// Shared body of `0x00c9a1ec` and `0x00c9a420`: maps the conversion
/// character (offset `0x31` of the state) to the helper it dispatches to.
/// `None` means the routine returns 0 (unsupported conversion).  `d`/`i`
/// also set flag bit `0x10` (signed) before calling the integer helper.
pub fn w32a133_dispatch(c: u8) -> Option<W32a133Conv> {
    let int = |radix, arg, set_signed| {
        Some(W32a133Conv::Int {
            radix,
            arg,
            set_signed,
        })
    };
    match c {
        b'd' | b'i' => int(10, 0, true),
        b'u' => int(10, 0, false),
        b'X' => int(0x10, 1, false),
        b'x' => int(0x10, 0, false),
        b'A' | b'E' | b'F' | b'G' | b'a' | b'e' | b'f' | b'g' => Some(W32a133Conv::Float),
        b'C' | b'c' => Some(W32a133Conv::Char),
        b'S' | b's' => Some(W32a133Conv::Str),
        b'Z' => Some(W32a133Conv::WideChar),
        b'p' => Some(W32a133Conv::Pointer),
        b'n' => Some(W32a133Conv::CountN),
        b'o' => Some(W32a133Conv::OctalO),
        _ => None,
    }
}

/// `0x00c9a1ec`: builds the sign / `0x` prefix after dispatch.  Returns
/// the prefix bytes and length, plus the padding count
/// `width - prefix_arg - len` (`width` at +0x24, `prefix_arg` at +0x38).
/// Flag bits: 0 plus, 1 space, 4 signed, 5 alt-form, 6 negative.
/// `0x00c9a420` is the byte-identical twin (wide-char build).
pub fn w32a133_prefix(flags: u32, conv: u8, width: i32, prefix_arg: i32) -> ([u8; 4], usize, i32) {
    let mut buf = [0u8; 4];
    let mut n = 0usize;
    if flags >> 4 & 1 != 0 {
        if flags >> 6 & 1 != 0 {
            buf[0] = b'-';
            n = 1;
        } else if flags & 1 != 0 {
            buf[0] = b'+';
            n = 1;
        } else if flags >> 1 & 1 != 0 {
            buf[0] = b' ';
            n = 1;
        }
    }
    let hex = matches!(conv, b'x' | b'X') && flags >> 5 & 1 != 0;
    if hex || matches!(conv, b'a' | b'A') {
        buf[n] = b'0';
        buf[n + 1] = if matches!(conv, b'X' | b'A') {
            b'X'
        } else {
            b'x'
        };
        n += 2;
    }
    (buf, n, width - prefix_arg - n as i32)
}

/// `0x00c9923d`: move-assign of a handle slot.  The old value in `dst` is
/// released through `free`, then `dst` takes `*src` and `src` is zeroed.
pub fn w32a133_move_assign(dst: &mut u32, src: &mut u32, mut free: impl FnMut(u32)) {
    free(*dst);
    *dst = 0;
    *dst = *src;
    *src = 0;
}

/// `0x00c9a420` wrapper sharing the dispatch/prefix logic above.
pub fn w32a133_prefix_wide(
    flags: u32,
    conv: u8,
    width: i32,
    prefix_arg: i32,
) -> ([u8; 4], usize, i32) {
    w32a133_prefix(flags, conv, width, prefix_arg)
}

#[cfg(test)]
mod tests_wave32_agent133 {
    use super::*;

    #[test]
    fn dispatch() {
        assert_eq!(
            w32a133_dispatch(b'x'),
            Some(W32a133Conv::Int {
                radix: 16,
                arg: 0,
                set_signed: false
            })
        );
        assert_eq!(w32a133_dispatch(b'Y'), None);
        assert_eq!(w32a133_dispatch(b'h'), None);
        assert_eq!(w32a133_dispatch(b'p'), Some(W32a133Conv::Pointer));
    }

    #[test]
    fn prefix() {
        let (b, n, pad) = w32a133_prefix(0x10 | 0x40, b'd', 10, 0);
        assert_eq!((&b[..n], pad), (&b"-"[..], 9));
        let (b, n, _) = w32a133_prefix(0x20, b'X', 0, 0);
        assert_eq!(&b[..n], b"0X");
        let (b, n, _) = w32a133_prefix(0x12, b'd', 0, 0);
        assert_eq!(&b[..n], b" ");
    }

    #[test]
    fn move_assign() {
        let (mut a, mut b, mut freed) = (5, 9, 0);
        w32a133_move_assign(&mut a, &mut b, |v| freed = v);
        assert_eq!((a, b, freed), (9, 0, 5));
    }
}
