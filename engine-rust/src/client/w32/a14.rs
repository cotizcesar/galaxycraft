#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent14.rs
// ===========================================================================
// Wave 32 agent 14: three small UI helpers whose shared callee (the
// redraw/invalidate routine) is modelled as a returned `bool`.

/// `0x008c5840`: clears the dirty byte (offset `0x1e1` of the inner object)
/// and reports whether the caller must run the refresh routine.
pub fn w32a14_clear_dirty_flag(flag: &mut u8) -> bool {
    if *flag != 0 {
        *flag = 0;
        true
    } else {
        false
    }
}

/// `0x008c6380`: when `enable` is non-zero stores the pair at offsets `0xb4`
/// and `0xb8`; returns true when the caller must refresh and then call the
/// follow-up routine with argument 1.
pub fn w32a14_set_pair_if_enabled(slot: &mut [u32; 2], a: u32, b: u32, enable: u8) -> bool {
    if enable != 0 {
        slot[0] = a;
        slot[1] = b;
        true
    } else {
        false
    }
}

/// `0x008c6a20`: rects are `[left, top, right, bottom]` i16; returns true
/// (refresh needed) when width or height differ.
pub fn w32a14_rect_size_differs(a: &[i16; 4], b: &[i16; 4]) -> bool {
    (a[2] as i32 - a[0] as i32 != b[2] as i32 - b[0] as i32)
        || (a[3] as i32 - a[1] as i32 != b[3] as i32 - b[1] as i32)
}

#[cfg(test)]
mod tests_wave32_agent14 {
    use super::*;

    #[test]
    fn dirty_flag() {
        let mut f = 1u8;
        assert!(w32a14_clear_dirty_flag(&mut f));
        assert_eq!(f, 0);
        assert!(!w32a14_clear_dirty_flag(&mut f));
    }

    #[test]
    fn pair() {
        let mut s = [0u32; 2];
        assert!(!w32a14_set_pair_if_enabled(&mut s, 1, 2, 0));
        assert_eq!(s, [0, 0]);
        assert!(w32a14_set_pair_if_enabled(&mut s, 1, 2, 1));
        assert_eq!(s, [1, 2]);
    }

    #[test]
    fn rects() {
        assert!(!w32a14_rect_size_differs(&[0, 0, 4, 4], &[1, 1, 5, 5]));
        assert!(w32a14_rect_size_differs(&[0, 0, 4, 4], &[0, 0, 4, 5]));
    }
}
