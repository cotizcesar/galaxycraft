#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent60.rs
// ===========================================================================
// Wave 33 agent 60: CRT stdio temporary-buffer helpers (`_stbuf`/`_ftbuf`).

/// Minimal model of the CRT `FILE` words used by the helpers (`ptr`,
/// `base`, `cnt`, `flag`, `bufsiz` at word indices 0, 1, 2, 3, 6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W33a60File {
    pub ptr: u32,
    pub base: u32,
    pub cnt: u32,
    pub flag: u32,
    pub bufsiz: u32,
    /// Address of the inline 2-byte buffer (`param_1 + 5` words).
    pub inline_buf: u32,
}

/// `0x00caf798`: true when the saved stream-state word equals the
/// reference word with bit 0 set.
pub fn w33a60_state_matches(state: u32, reference: u32) -> bool {
    state == (reference | 1)
}

/// `0x00caf7ad`: set up a temporary 0x1000-byte buffer for stdout/stderr
/// (`which` is 1 or 2 after the caller resolved the stream; `None` when the
/// stream is neither or not a tty).  `slot` is the cached buffer for that
/// stream, `alloc` allocates 0x1000 bytes (0 on failure).  Returns whether
/// the buffer was installed.
pub fn w33a60_stbuf(
    f: &mut W33a60File,
    is_tty: bool,
    which: Option<u8>,
    slot: &mut u32,
    counter: &mut u32,
    alloc: &mut dyn FnMut() -> u32,
) -> bool {
    if !is_tty || !matches!(which, Some(1) | Some(2)) {
        return false;
    }
    *counter = counter.wrapping_add(1);
    if f.flag & 0x4c0 != 0 {
        return false;
    }
    f.flag |= 0x282;
    if *slot == 0 {
        *slot = alloc();
        if *slot == 0 {
            f.cnt = 2;
            f.base = f.inline_buf;
            f.ptr = f.inline_buf;
            f.bufsiz = 2;
            return true;
        }
    }
    f.base = *slot;
    f.ptr = *slot;
    f.cnt = 0x1000;
    f.bufsiz = 0x1000;
    true
}

/// `0x00caf860`: release a temporary buffer: when `active` and flag bit 9
/// is set, flush via `flush`, clear flag bits 0x280 and zero the buffer
/// fields.
pub fn w33a60_ftbuf(active: bool, f: &mut W33a60File, flush: &mut dyn FnMut(&mut W33a60File)) {
    if active && (f.flag >> 9) & 1 != 0 {
        flush(f);
        f.flag &= 0xffff_fd7f;
        f.bufsiz = 0;
        f.base = 0;
        f.ptr = 0;
    }
}

#[cfg(test)]
mod tests_wave33_agent60 {
    use super::*;

    #[test]
    fn state() {
        assert!(w33a60_state_matches(5, 4));
        assert!(!w33a60_state_matches(4, 4));
    }

    #[test]
    fn stbuf_and_ftbuf() {
        let mut f = W33a60File {
            inline_buf: 99,
            ..Default::default()
        };
        let (mut slot, mut n) = (0u32, 0u32);
        assert!(w33a60_stbuf(
            &mut f,
            true,
            Some(1),
            &mut slot,
            &mut n,
            &mut || 0
        ));
        assert_eq!((f.bufsiz, f.base, f.flag), (2, 99, 0x282));
        let mut g = W33a60File::default();
        assert!(w33a60_stbuf(
            &mut g,
            true,
            Some(2),
            &mut slot,
            &mut n,
            &mut || 500
        ));
        assert_eq!((g.base, g.cnt, n), (500, 0x1000, 2));
        let mut h = W33a60File::default();
        assert!(!w33a60_stbuf(
            &mut h,
            true,
            None,
            &mut slot,
            &mut n,
            &mut || 1
        ));
        let mut flushed = false;
        w33a60_ftbuf(true, &mut g, &mut |_| flushed = true);
        assert!(flushed && g.flag == 2 && g.base == 0);
    }
}
