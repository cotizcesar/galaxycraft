#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent59.rs
// ===========================================================================
// Wave 34 agent 59: CRT character-class helpers.

/// `0x00caea06`: CRT `_isctype`-style lookup.  `0xffff` yields 0; values
/// below 0x100 index the 16-bit ctype table and mask with `mask`; larger
/// values call `wide(ch)` (the FUN_00cb7d3b wide lookup) and mask its result
/// when it succeeds.
pub fn w34a59_isctype(ch: u16, mask: u16, table: &[u16], wide: impl Fn(u16) -> Option<u16>) -> u16 {
    if ch == 0xffff {
        return 0;
    }
    if ch < 0x100 {
        return table.get(ch as usize).copied().unwrap_or(0) & mask;
    }
    match wide(ch) {
        Some(v) => v & mask,
        None => 0,
    }
}

/// `0x00caed87`: forwards its four arguments to the five-argument worker
/// (FUN_00caec2a) with a trailing 0.
pub fn w34a59_forward4<R>(
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    inner: impl FnOnce(u32, u32, u32, u32, u32) -> R,
) -> R {
    inner(a, b, c, d, 0)
}

/// `0x00cb16d9`: no-argument wrapper that just calls FUN_00cb1471.
pub fn w34a59_call_inner<R>(inner: impl FnOnce() -> R) -> R {
    inner()
}

#[cfg(test)]
mod tests_wave34_agent59 {
    use super::*;

    #[test]
    fn isctype() {
        let t = [0x0003u16; 256];
        assert_eq!(w34a59_isctype(0xffff, 0xff, &t, |_| Some(1)), 0);
        assert_eq!(w34a59_isctype(5, 0x1, &t, |_| None), 1);
        assert_eq!(w34a59_isctype(0x200, 0x2, &t, |_| Some(3)), 2);
        assert_eq!(w34a59_isctype(0x200, 0x2, &t, |_| None), 0);
    }

    #[test]
    fn wrappers() {
        assert_eq!(
            w34a59_forward4(1, 2, 3, 4, |a, b, c, d, e| a + b + c + d + e),
            10
        );
        assert_eq!(w34a59_call_inner(|| 7), 7);
    }
}
