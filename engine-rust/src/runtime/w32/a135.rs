#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent135.rs
// ===========================================================================
// Wave 32 translation batch 135: printf-state integer-prepare wrappers.

/// Format state fields touched by the wrappers (`this+0x20`, `+0x28`, `+0x2c`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct W32a135State {
    pub flags: u32,
    pub field28: u32,
    pub field2c: u32,
}

/// `0x00c9b12c`: if flag bit 5 is set, sets bit 7 (`0x80`); then the caller
/// invokes the narrow prepare helper with `(8, 0)`, returned here.
pub fn w32a135_narrow_prepare_args(s: &mut W32a135State) -> (u32, u32) {
    if (s.flags >> 5) & 1 != 0 {
        s.flags |= 0x80;
    }
    (8, 0)
}

/// `0x00c9b14b`: same as the narrow variant but for the wide prepare helper
/// (args `(8, 0)`).
pub fn w32a135_wide_prepare_args(s: &mut W32a135State) -> (u32, u32) {
    if (s.flags >> 5) & 1 != 0 {
        s.flags |= 0x80;
    }
    (8, 0)
}

/// `0x00c9b16a`: stores `+0x28 = 8`, `+0x2c = 10`; narrow prepare args `(0x10, 1)`.
pub fn w32a135_pointer_prepare_args(s: &mut W32a135State) -> (u32, u32) {
    s.field28 = 8;
    s.field2c = 10;
    (0x10, 1)
}

#[cfg(test)]
mod tests_wave32_agent135 {
    use super::*;

    #[test]
    fn wrappers() {
        let mut s = W32a135State {
            flags: 0x20,
            ..Default::default()
        };
        assert_eq!(w32a135_narrow_prepare_args(&mut s), (8, 0));
        assert_eq!(s.flags, 0xa0);
        let mut t = W32a135State::default();
        assert_eq!(w32a135_wide_prepare_args(&mut t), (8, 0));
        assert_eq!(t.flags, 0);
        assert_eq!(w32a135_pointer_prepare_args(&mut t), (0x10, 1));
        assert_eq!((t.field28, t.field2c), (8, 10));
    }
}
