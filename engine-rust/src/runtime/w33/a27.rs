#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent27.rs
// ===========================================================================
// Wave 33 agent 27: runtime helpers.

/// State for the byte-counting sink at `0x00aec790` (offsets 0x62, 0x66,
/// 0xc0 and 0xc4 of the owning object).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W33a27Sink {
    /// Byte at +0x66: sink accepts data.
    pub open: bool,
    /// Byte at +0x62: when set, the callback is skipped.
    pub skip_callback: bool,
    /// 64-bit running total (+0xc0 low, +0xc4 high).
    pub total: u64,
}

/// `0x00aec790`: accounts `len` bytes. A zero length succeeds; otherwise a
/// closed sink fails, the callback runs unless `skip_callback`, and the
/// 64-bit total (low word with carry into the high word) grows by `len`.
pub fn w33a27_account(sink: &mut W33a27Sink, len: u32, mut cb: impl FnMut(u32)) -> bool {
    if len != 0 {
        if !sink.open {
            return false;
        }
        if !sink.skip_callback {
            cb(len);
        }
        sink.total = sink.total.wrapping_add(len as u64) & 0xffff_ffff_ffff_ffff;
    }
    true
}

/// `0x00b09c80`: queries with kind 9 and a zeroed 64-bit out value; stores
/// the callee result in `out` and returns 2 when the 64-bit value equals
/// `expected` (high word zero), else 0.
pub fn w33a27_match_query(
    expected: u32,
    out: &mut u32,
    query: impl FnOnce(u32, &mut u64) -> u32,
) -> u32 {
    let mut v: u64 = 0;
    *out = query(9, &mut v);
    if v == expected as u64 { 2 } else { 0 }
}

#[cfg(test)]
mod tests_wave33_agent27 {
    use super::*;

    #[test]
    fn account_rules() {
        let mut s = W33a27Sink::default();
        assert!(w33a27_account(&mut s, 0, |_| panic!()));
        assert!(!w33a27_account(&mut s, 5, |_| {}));
        s.open = true;
        let mut n = 0;
        assert!(w33a27_account(&mut s, 0xffff_ffff, |l| n = l));
        assert!(w33a27_account(&mut s, 2, |_| {}));
        assert_eq!(n, 0xffff_ffff);
        assert_eq!(s.total, 0x1_0000_0001);
        s.skip_callback = true;
        assert!(w33a27_account(&mut s, 1, |_| panic!()));
    }

    #[test]
    fn match_query_rules() {
        let mut out = 0;
        let r = w33a27_match_query(7, &mut out, |k, v| {
            assert_eq!(k, 9);
            *v = 7;
            42
        });
        assert_eq!((r, out), (2, 42));
        assert_eq!(
            w33a27_match_query(7, &mut out, |_, v| {
                *v = 1 << 32 | 7;
                1
            }),
            0
        );
    }
}
