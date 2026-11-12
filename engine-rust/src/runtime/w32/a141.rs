#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent141.rs
// ===========================================================================
// Wave 32 agent 141 translations.

/// `0x00c9fb11`: classifies a decoder state record.  `mode` is the field at
/// +0x28, `arg` the field at +0x20, `flag` the byte at +0x24.  Modes 7 and
/// 9 (and 2..=6) defer to the caller-supplied helpers; modes 0, 1 and 8
/// return `flag != 0` plus one; negative or unknown modes return 0.
pub fn w32a141_state_kind(
    mode: i32,
    arg: u32,
    flag: u8,
    mut helper_a: impl FnMut(u32) -> i32,
    mut helper_b: impl FnMut(u32) -> i32,
) -> i32 {
    if mode < 0 {
        return 0;
    }
    if mode > 1 {
        if mode > 6 {
            if mode == 7 {
                return helper_a(arg);
            }
            if mode == 8 {
                return (flag != 0) as i32 + 1;
            }
            if mode != 9 {
                return 0;
            }
        }
        return helper_b(arg);
    }
    (flag != 0) as i32 + 1
}

#[cfg(test)]
mod tests_wave32_agent141 {
    use super::*;

    #[test]
    fn state_kind() {
        let a = |x: u32| x as i32 + 100;
        let b = |x: u32| x as i32 + 200;
        assert_eq!(w32a141_state_kind(-1, 5, 1, a, b), 0);
        assert_eq!(w32a141_state_kind(0, 5, 0, a, b), 1);
        assert_eq!(w32a141_state_kind(1, 5, 1, a, b), 2);
        assert_eq!(w32a141_state_kind(2, 5, 1, a, b), 205);
        assert_eq!(w32a141_state_kind(6, 5, 1, a, b), 205);
        assert_eq!(w32a141_state_kind(7, 5, 1, a, b), 105);
        assert_eq!(w32a141_state_kind(8, 5, 1, a, b), 2);
        assert_eq!(w32a141_state_kind(9, 5, 1, a, b), 205);
        assert_eq!(w32a141_state_kind(10, 5, 1, a, b), 0);
    }
}
