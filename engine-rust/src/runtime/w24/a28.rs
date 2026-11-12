#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent28.rs
// ===========================================================================
// Wave 24 agent 28 translations (dialog helpers).

fn w24a28_rd_i16(buf: &[u8], off: usize) -> i16 {
    i16::from_le_bytes([buf[off], buf[off + 1]])
}

fn w24a28_wr_i16(buf: &mut [u8], off: usize, v: i16) {
    buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

/// `0x0078f510`: shifts the dialog's x1 (+4) and x2 (+8) fields left by 100,
/// then snaps its first child (`child`, found by the caller via the dialog
/// child lookup) so that x2 = -10 and x1 moves by `-10 - old_x2`.
pub fn w24a28_shift_dialog_and_child(dialog: &mut [u8], child: &mut [u8]) {
    let a = w24a28_rd_i16(dialog, 4).wrapping_sub(100);
    w24a28_wr_i16(dialog, 4, a);
    let b = w24a28_rd_i16(dialog, 8).wrapping_sub(100);
    w24a28_wr_i16(dialog, 8, b);
    let old_x2 = w24a28_rd_i16(child, 8);
    w24a28_wr_i16(child, 8, -10);
    let x1 = w24a28_rd_i16(child, 4).wrapping_add((-10i16).wrapping_sub(old_x2));
    w24a28_wr_i16(child, 4, x1);
}

/// `0x00790170`: true when the feature flag is set, the first predicate
/// holds and the second does not.
pub fn w24a28_flag_and_not(flag: i32, first: bool, second: bool) -> bool {
    flag != 0 && first && !second
}

#[cfg(test)]
mod tests_wave24_agent28 {
    use super::*;

    #[test]
    fn shift_dialog() {
        let mut d = [0u8; 12];
        w24a28_wr_i16(&mut d, 4, 50);
        w24a28_wr_i16(&mut d, 8, 200);
        let mut c = [0u8; 12];
        w24a28_wr_i16(&mut c, 4, 20);
        w24a28_wr_i16(&mut c, 8, 30);
        w24a28_shift_dialog_and_child(&mut d, &mut c);
        assert_eq!(w24a28_rd_i16(&d, 4), -50);
        assert_eq!(w24a28_rd_i16(&d, 8), 100);
        assert_eq!(w24a28_rd_i16(&c, 8), -10);
        assert_eq!(w24a28_rd_i16(&c, 4), -20);
    }

    #[test]
    fn flag_and_not() {
        assert!(w24a28_flag_and_not(1, true, false));
        assert!(!w24a28_flag_and_not(0, true, false));
        assert!(!w24a28_flag_and_not(1, false, false));
        assert!(!w24a28_flag_and_not(1, true, true));
    }
}
