#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent05.rs
// ===========================================================================
// Wave 34 translation batch 05.

/// `0x004bb260`: returns 1 when bit 25 (0x2000000, SSE) of the CPUID leaf-1
/// feature word is set, else 0.
pub fn w34a05_cpu_has_sse(feature_word: u32) -> u32 {
    ((feature_word & 0x0200_0000) != 0) as u32
}

/// `0x00cae905`: CRT `_fpclass`-style classifier of a double given as low and
/// high dwords.  `nan_kind` is the result of the NaN/Inf probe (callee
/// FUN_00cae75b): 1 -> 0x200, 2 -> 4, 3 -> 2, other -> 1.
pub fn w34a05_fpclass(low: u32, high: u32, nan_kind: i32) -> i32 {
    if (high >> 16) & 0x7ff0 == 0x7ff0 {
        return match nan_kind {
            1 => 0x200,
            2 => 4,
            3 => 2,
            _ => 1,
        };
    }
    let neg = (high & 0x8000_0000) != 0;
    let sign_mask = if neg { 0xffff_ffffu32 } else { 0 };
    if high & 0x7ff0_0000 == 0 && ((high & 0xfffff) != 0 || low != 0) {
        return ((sign_mask & 0xffff_ff90) as i32).wrapping_add(0x80);
    }
    if (high & 0x7fff_ffff) == 0 && low == 0 {
        return ((sign_mask & 0xffff_ffe0) as i32).wrapping_add(0x40);
    }
    ((sign_mask & 0xffff_ff08) as i32).wrapping_add(0x100)
}

#[cfg(test)]
mod tests_wave34_agent05 {
    use super::*;

    #[test]
    fn sse_bit() {
        assert_eq!(w34a05_cpu_has_sse(0x0200_0000), 1);
        assert_eq!(w34a05_cpu_has_sse(!0x0200_0000), 0);
    }

    #[test]
    fn fpclass_cases() {
        assert_eq!(w34a05_fpclass(0, 0x7ff0_0000, 1), 0x200);
        assert_eq!(w34a05_fpclass(0, 0x7ff0_0000, 2), 4);
        assert_eq!(w34a05_fpclass(0, 0x7ff0_0000, 3), 2);
        assert_eq!(w34a05_fpclass(0, 0x7ff0_0000, 0), 1);
        assert_eq!(w34a05_fpclass(1, 0, 0), 0x80);
        assert_eq!(w34a05_fpclass(1, 0x8000_0000, 0), 0x10);
        assert_eq!(w34a05_fpclass(0, 0, 0), 0x40);
        assert_eq!(w34a05_fpclass(0, 0x8000_0000, 0), 0x20);
        assert_eq!(w34a05_fpclass(0, 0x3ff0_0000, 0), 0x100);
        assert_eq!(w34a05_fpclass(0, 0xbff0_0000, 0), 8);
    }
}
