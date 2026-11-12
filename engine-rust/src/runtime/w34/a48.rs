#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent48.rs
// ===========================================================================
// Wave 34 agent 48: small dispatch/flag-combine helpers.

/// `0x00c1a670`: looks up a value through the virtual slot at `+0x38` using
/// the bytes at record offsets `+8` and `+0x20`, validates the record with the
/// check callee, and either dispatches the forwarded call (slot `+300` of the
/// owner object) or runs the failure path.  Returns `param_1` unchanged.
pub fn w34a48_validate_dispatch<L, C, D, F>(
    param_1: u32,
    rec: &[u8],
    lookup: L,
    check: C,
    dispatch: D,
    on_fail: F,
) -> u32
where
    L: FnOnce(u8, u8) -> u32,
    C: FnOnce(&[u8], u32) -> bool,
    D: FnOnce(),
    F: FnOnce(),
{
    let key = lookup(rec[8], rec[0x20]);
    if check(rec, key) {
        dispatch();
    } else {
        on_fail();
    }
    param_1
}

/// `0x00c20d20`: ANDs the byte results of two callees with bit 12 of
/// `a_flags` and bit 3 of `b_flags` (both shifted down, low byte used).
pub fn w34a48_flag_and_a(a_flags: u32, b_flags: u32, first: u8, second: u8) -> u8 {
    second & ((a_flags >> 12) as u8) & ((b_flags >> 3) as u8) & first
}

/// `0x00c20f50`: ANDs the byte results of two callees with bit 4 of
/// `a_flags` and bit 11 of `b_flags` (both shifted down, low byte used).
pub fn w34a48_flag_and_b(a_flags: u32, b_flags: u32, first: u8, second: u8) -> u8 {
    second & first & ((a_flags >> 4) as u8) & ((b_flags >> 11) as u8)
}

#[cfg(test)]
mod tests_wave34_agent48 {
    use super::*;

    #[test]
    fn dispatch_paths() {
        let mut rec = [0u8; 0x24];
        rec[8] = 3;
        rec[0x20] = 4;
        let hit = std::cell::Cell::new(0);
        let r = w34a48_validate_dispatch(
            9,
            &rec,
            |a, b| (a as u32) << 8 | b as u32,
            |_, k| k == 0x304,
            || hit.set(1),
            || hit.set(2),
        );
        assert_eq!((r, hit.get()), (9, 1));
        let r = w34a48_validate_dispatch(
            7,
            &rec,
            |_, _| 0,
            |_, _| false,
            || hit.set(1),
            || hit.set(2),
        );
        assert_eq!((r, hit.get()), (7, 2));
    }

    #[test]
    fn flags() {
        assert_eq!(w34a48_flag_and_a(1 << 12, 1 << 3, 0xff, 0xff), 1);
        assert_eq!(w34a48_flag_and_a(0, 1 << 3, 0xff, 0xff), 0);
        assert_eq!(w34a48_flag_and_b(1 << 4, 1 << 11, 0xff, 0xff), 1);
        assert_eq!(w34a48_flag_and_b(1 << 4, 0, 0xff, 0xff), 0);
    }
}
