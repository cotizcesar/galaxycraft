#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent51.rs
// ===========================================================================
// Wave 33 agent 51: small path/stream helpers (merge target `runtime.rs`).

/// Record filled by `0x00ca52d2` (layout from the pseudocode offsets).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W33a51Record {
    /// Offset 0x00: `local_8 - 1`.
    pub first: i32,
    /// Offset 0x06: value returned by the 0x10-selector query.
    pub field6: u16,
    /// Offset 0x08: always set to 1.
    pub field8: u16,
    /// Offset 0x10: `local_8 - 1`.
    pub last: i32,
    /// Offsets 0x18, 0x20, 0x28: the same 64-bit handle.
    pub handles: [u64; 3],
}

/// `0x00ca52d2`: initialises a record from a stream handle.  `ready`
/// models the check callee (false -> error code 2 reported, returns false),
/// `query16` the 0x10-selector query, `count` the length probe (None on
/// failure) and `alloc` the 0x7bc allocator call.
pub fn w33a51_init_record(
    rec: &mut W33a51Record,
    ready: impl FnOnce() -> bool,
    report_error: impl FnOnce(u32),
    query16: impl FnOnce() -> u16,
    count: impl FnOnce() -> Option<i32>,
    alloc: impl FnOnce() -> u64,
) -> bool {
    if !ready() {
        report_error(2);
        return false;
    }
    rec.field6 = query16();
    rec.field8 = 1;
    let Some(n) = count() else {
        return false;
    };
    rec.last = n.wrapping_sub(1);
    rec.first = n.wrapping_sub(1);
    let h = alloc();
    rec.handles = [h; 3];
    true
}

/// `0x00ca57c3`: validates a UTF-16 path.  Skips a `X:` drive prefix; true
/// when the remainder is empty, or when `is_sep(first)` holds and the
/// following unit is zero.  `None` input yields false.
pub fn w33a51_path_ok(path: Option<&[u16]>, is_sep: impl Fn(u16) -> bool) -> bool {
    let Some(p) = path else { return false };
    let at = |i: usize| p.get(i).copied().unwrap_or(0);
    let mut start = 0;
    let c = at(0);
    if ((0x41..0x5b).contains(&c) || (0x61..0x7b).contains(&c)) && at(1) == 0x3a {
        start = 2;
    }
    let c = at(start);
    c == 0 || (is_sep(c) && at(start + 1) == 0)
}

#[cfg(test)]
mod tests_wave33_agent51 {
    use super::*;

    #[test]
    fn path() {
        let sep = |c: u16| c == 0x5c;
        assert!(!w33a51_path_ok(None, sep));
        assert!(w33a51_path_ok(Some(&[0]), sep));
        assert!(w33a51_path_ok(Some(&[0x43, 0x3a, 0]), sep));
        assert!(w33a51_path_ok(Some(&[0x43, 0x3a, 0x5c, 0]), sep));
        assert!(!w33a51_path_ok(Some(&[0x43, 0x3a, 0x5c, 0x61]), sep));
        assert!(!w33a51_path_ok(Some(&[0x61, 0x62]), sep));
    }

    #[test]
    fn record() {
        let mut r = W33a51Record::default();
        let mut e = 0;
        assert!(!w33a51_init_record(
            &mut r,
            || false,
            |c| e = c,
            || 0,
            || None,
            || 0
        ));
        assert_eq!(e, 2);
        assert!(w33a51_init_record(
            &mut r,
            || true,
            |_| {},
            || 7,
            || Some(5),
            || 9
        ));
        assert_eq!((r.first, r.last, r.field6, r.field8), (4, 4, 7, 1));
        assert_eq!(r.handles, [9; 3]);
    }
}
