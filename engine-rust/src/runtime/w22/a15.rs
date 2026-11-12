#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent15.rs
// ===========================================================================
// Wave 22 batch 15 (runtime glue).
// Addresses cited below are the only ones implemented.

/// Outcome of the `0x006670e0` argument check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W22a15Check {
    /// Both arguments non-zero: the retail code calls a non-returning helper.
    Abort,
    /// Otherwise the function calls the error setter with this code (0x57).
    Error(u32),
}

/// `0x006670e0`: when both arguments are non-zero the original aborts
/// (non-returning call); otherwise it reports error code `0x57`.
pub fn w22a15_check_args(a: i32, b: i32) -> W22a15Check {
    if a != 0 && b != 0 {
        W22a15Check::Abort
    } else {
        W22a15Check::Error(0x57)
    }
}

/// `0x00669580`: initialises a circular list header at word address
/// `self_addr`: next = self, prev = self, size = 0.  Returns the three words.
/// `a != b` in the original triggers a non-returning helper; reported as
/// `Err(())`.
pub fn w22a15_list_head_init(self_addr: u32, a: i32, b: i32) -> Result<[u32; 3], ()> {
    if a != b {
        return Err(());
    }
    Ok([self_addr, self_addr, 0])
}

/// `0x0066f490`: constructor image: 14 dwords of the object with
/// `[0..4]=0`, halfword at byte 0xe = 3, `[11]=0x400`.
pub fn w22a15_route_ctor_init() -> [u32; 14] {
    let mut w = [0u32; 14];
    // Halfword at offset 0xe is the high half of dword 3 on little endian.
    w[3] = 3 << 16;
    w[11] = 0x400;
    w
}

#[cfg(test)]
mod tests_wave22_agent15 {
    use super::*;

    #[test]
    fn check_args() {
        assert_eq!(w22a15_check_args(1, 1), W22a15Check::Abort);
        assert_eq!(w22a15_check_args(0, 1), W22a15Check::Error(0x57));
        assert_eq!(w22a15_check_args(1, 0), W22a15Check::Error(0x57));
    }

    #[test]
    fn list_head() {
        assert_eq!(w22a15_list_head_init(0x100, 2, 2), Ok([0x100, 0x100, 0]));
        assert!(w22a15_list_head_init(0x100, 1, 2).is_err());
    }

    #[test]
    fn ctor() {
        let w = w22a15_route_ctor_init();
        assert_eq!(w[3], 0x0003_0000);
        assert_eq!(w[11], 0x400);
        assert_eq!(w[0], 0);
    }
}
