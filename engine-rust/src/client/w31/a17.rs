#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent17.rs
// ===========================================================================
// Wave 31 agent 17 translations (client helpers).

/// `0x00932140`: maps `mode` (1 -> 9, 2 -> 1, else -1), calls `inner(*cursor,
/// mode_code)`.  On success (`true`) the cursor is updated from the inner
/// result and the out value is cleared; on failure with `soft_fail == 0`
/// returns 1; otherwise the cursor is reset to `fallback`, out cleared.
/// Returns `(status, cursor, out)`.
pub fn w31a17_try_advance(
    cursor: u32,
    fallback: u32,
    mode: i32,
    soft_fail: i32,
    inner: impl FnOnce(u32, u32) -> Option<u32>,
) -> (u32, u32, u32) {
    let code = match mode {
        1 => 9,
        2 => 1,
        _ => 0xffff_ffff,
    };
    match inner(cursor, code) {
        Some(new_cursor) => (0, new_cursor, 0),
        None if soft_fail == 0 => (1, cursor, 0),
        None => (0, fallback, 0),
    }
}

/// `0x00940aa0`: forwards to the generic dispatcher with selector 8.
pub fn w31a17_dispatch8<R>(a: u32, b: u32, f: impl FnOnce(u32, u32, u32) -> R) -> R {
    f(8, a, b)
}

/// `0x00944e70`: bit 15 of the state returned by `query` for the slot
/// mapped from `kind` (1->1, 2->4, 4->2, 8->5, 0x10->6); other kinds give 0.
pub fn w31a17_key_state_bit(kind: u32, query: impl FnOnce(u32) -> u32) -> u32 {
    let slot = match kind {
        1 => 1,
        2 => 4,
        4 => 2,
        8 => 5,
        0x10 => 6,
        _ => return 0,
    };
    (query(slot) >> 15) & 1
}

/// `0x009546d0`: sets bit 0 of the flag byte of 0x14-byte record `idx`.
pub fn w31a17_set_record_flag(flags: &mut [u8], idx: u8) {
    flags[idx as usize * 0x14] |= 1;
}

/// State block shifted by `0x00958370`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W31a17Pair {
    pub quad: [u32; 4],
    pub tail: u32,
    pub prev_quad: [u32; 4],
    pub prev_tail: u32,
}

/// `0x00958370`: pushes current values into the "previous" slots and stores
/// the new quad and tail.
pub fn w31a17_push_state(s: &mut W31a17Pair, quad: [u32; 4], tail: u32) {
    s.prev_tail = s.tail;
    s.prev_quad = s.quad;
    s.tail = tail;
    s.quad = quad;
}

/// `0x00959280`: maps `kind` (1->0x30, 2->0x21, 4->0x25, 8->0x40, else
/// 0x10), and returns true when the call result is not 2.
pub fn w31a17_dialog_not_two(kind: u16, call: impl FnOnce(u32) -> i32) -> bool {
    let flags = match kind {
        1 => 0x30,
        2 => 0x21,
        4 => 0x25,
        8 => 0x40,
        _ => 0x10,
    };
    call(flags) != 2
}

#[cfg(test)]
mod tests_wave31_agent17 {
    use super::*;

    #[test]
    fn advance() {
        assert_eq!(
            w31a17_try_advance(5, 9, 1, 0, |c, m| {
                assert_eq!(m, 9);
                Some(c + 1)
            }),
            (0, 6, 0)
        );
        assert_eq!(
            w31a17_try_advance(5, 9, 3, 0, |_, m| {
                assert_eq!(m, u32::MAX);
                None
            }),
            (1, 5, 0)
        );
        assert_eq!(
            w31a17_try_advance(5, 9, 2, 1, |_, m| {
                assert_eq!(m, 1);
                None
            }),
            (0, 9, 0)
        );
    }

    #[test]
    fn small() {
        assert_eq!(w31a17_dispatch8(1, 2, |a, b, c| a + b + c), 11);
        assert_eq!(
            w31a17_key_state_bit(8, |s| if s == 5 { 0x8000 } else { 0 }),
            1
        );
        assert_eq!(w31a17_key_state_bit(3, |_| 0xffff), 0);
        let mut f = vec![0u8; 0x14 * 3];
        w31a17_set_record_flag(&mut f, 2);
        assert_eq!(f[0x28], 1);
        assert_eq!(
            w31a17_dialog_not_two(4, |x| {
                assert_eq!(x, 0x25);
                2
            }),
            false
        );
        assert!(w31a17_dialog_not_two(0x20, |x| {
            assert_eq!(x, 0x10);
            1
        }));
        let mut s = W31a17Pair::default();
        w31a17_push_state(&mut s, [1, 2, 3, 4], 5);
        w31a17_push_state(&mut s, [6, 7, 8, 9], 10);
        assert_eq!((s.prev_quad, s.prev_tail, s.tail), ([1, 2, 3, 4], 5, 10));
    }
}
