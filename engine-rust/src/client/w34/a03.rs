#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent03.rs
// ===========================================================================
// Wave 34 translation batch 03.
//
// The shared callee (FUN_009a16c0) is not modelled, so it is supplied by
// the caller as a closure.

/// `0x009c4e00`: forwards `param_2` to the callee with a constant 0 flag.
pub fn w34a03_forward_zero(param_2: u32, callee: &mut dyn FnMut(u32, i32) -> i32) -> i32 {
    callee(param_2, 0)
}

/// `0x009cce80`: maps mode 0/1/2 to the same value and calls the callee;
/// any other mode returns `0x80070057`.  A nonzero callee result returns 1.
/// On success the optional out pair receives (`out_value`, 0) and 0 is
/// returned.
pub fn w34a03_set_mode(
    param_2: u32,
    mode: i32,
    out: Option<&mut [i32; 2]>,
    out_value: i32,
    callee: &mut dyn FnMut(u32, i32) -> i32,
) -> u32 {
    if !(0..=2).contains(&mode) {
        return 0x8007_0057;
    }
    if callee(param_2, mode) != 0 {
        return 1;
    }
    if let Some(o) = out {
        o[0] = out_value;
        o[1] = 0;
    }
    0
}

/// `0x009cdf20`: when bit 0 of the flags at +0x18c is set, forwards the
/// signed byte `param_4` to the callee; otherwise returns `0xffffffff`.
pub fn w34a03_forward_if_flag(
    flags: u32,
    param_2: u32,
    param_4: i8,
    callee: &mut dyn FnMut(u32, i32) -> u32,
) -> u32 {
    if flags & 1 != 0 {
        callee(param_2, param_4 as i32)
    } else {
        0xffff_ffff
    }
}

#[cfg(test)]
mod tests_wave34_agent03 {
    use super::*;

    #[test]
    fn forward_zero() {
        assert_eq!(w34a03_forward_zero(7, &mut |a, b| (a as i32) + b), 7);
    }

    #[test]
    fn set_mode_cases() {
        let mut o = [9, 9];
        assert_eq!(w34a03_set_mode(1, 3, None, 0, &mut |_, _| 0), 0x8007_0057);
        assert_eq!(w34a03_set_mode(1, 1, Some(&mut o), 5, &mut |_, _| 1), 1);
        assert_eq!(o, [9, 9]);
        assert_eq!(
            w34a03_set_mode(1, 2, Some(&mut o), 5, &mut |_, m| (m - 2) as i32),
            0
        );
        assert_eq!(o, [5, 0]);
    }

    #[test]
    fn flag_gate() {
        assert_eq!(w34a03_forward_if_flag(0, 1, 2, &mut |_, _| 0), 0xffff_ffff);
        assert_eq!(
            w34a03_forward_if_flag(1, 1, -1, &mut |_, c| c as u32),
            0xffff_ffff
        );
        assert_eq!(w34a03_forward_if_flag(3, 1, 4, &mut |_, c| c as u32), 4);
    }
}
