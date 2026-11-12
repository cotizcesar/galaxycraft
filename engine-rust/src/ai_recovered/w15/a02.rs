#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave15/agent02.rs
// ===========================================================================
// Wave 15 translation batch 02.
//
// Small pure helpers: a per-player table lookup, a unit-record cost table
// reader, and two callback-driven predicates.  Skipped rows (written as
// `FUN_00xxxxxx`) are listed at the bottom of this header:
//
// - FUN_00638b00, FUN_00638c00, FUN_0063a3c0: tail-call a non-returning stub
// - FUN_0063a9a0: only non-returning calls, no recoverable result
// - FUN_006520e0, FUN_00652250, FUN_00653190: end in non-returning calls
// - FUN_006523f0, FUN_00655ac0, FUN_006551a0, FUN_006565b0: depend on opaque
//   globals / many unmodelled callees, control flow does not return normally
// - FUN_0065ee50, FUN_00713200: string/exception glue ending in noreturn
// - FUN_00687d90: file writer with noreturn error paths
// - FUN_0073f120: single global store (DAT_011a2434), no state to model

/// `0x00695be0`: reads the u32 at index `col + row * 12` of the per-player
/// table that starts at offset `0x3234` of the global state block.  `table`
/// is that table viewed as u32 words.  Returns `None` when out of range.
pub fn w15a02_player_table_value(table: &[u32], row: u16, col: u8) -> Option<u32> {
    let idx = usize::from(col) + usize::from(row) * 12;
    table.get(idx).copied()
}

/// One 0x30-byte record of the table based at `DAT_011ac918`; only the
/// fields read by the helpers below are kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct W15a02Record {
    /// Signed word at record offset `0x0`.
    pub word0: i16,
    /// Byte at record offset `0x4`.
    pub byte4: u8,
    /// Byte at record offset `0x5`.
    pub byte5: u8,
    /// Dword at record offset `0x8`.
    pub dword8: u32,
}

/// `0x0076a4d0`: returns the signed word at the start of record `id`
/// (valid ids are `1..0xfa`), else 0.
pub fn w15a02_record_word(rec: &W15a02Record, id: u32) -> i32 {
    if id != 0 && id < 0xfa {
        i32::from(rec.word0)
    } else {
        0
    }
}

/// `0x00769aa0`: cost estimate for record `id` (valid `1..0xfa`, else 0).
/// `limit` is the result of the callee queried with the second argument.
/// With `c = dword8 / 500`: if `limit <= c` the base is `min(c, byte4 * 2)`,
/// otherwise it is `byte4`; the result adds `byte5 * 3`.
pub fn w15a02_record_cost(rec: &W15a02Record, id: u32, limit: u32) -> i32 {
    if id == 0 || id >= 0xfa {
        return 0;
    }
    let c = rec.dword8 / 500;
    let a = u32::from(rec.byte4);
    let base = if limit <= c { c.min(a * 2) } else { a };
    (base + u32::from(rec.byte5) * 3) as i32
}

/// `0x007693f0`: guarded call.  When `guarded` and `is_blocked()` reports
/// the player blocked, returns 0.  Otherwise calls `inner(arg)` where `arg`
/// is `0x28` if `kind as i16 == 0x7c`, else `param6`; if the inner result is
/// 0 and `guarded`, `on_fail()` runs.  Returns the inner result.
pub fn w15a02_guarded_call(
    kind: u32,
    param6: u32,
    guarded: bool,
    mut is_blocked: impl FnMut() -> bool,
    mut on_fail: impl FnMut(),
    mut inner: impl FnMut(u32) -> i32,
) -> i32 {
    if guarded && is_blocked() {
        return 0;
    }
    let arg = if kind as u16 == 0x7c { 0x28 } else { param6 };
    let r = inner(arg);
    if r == 0 && guarded {
        on_fail();
    }
    r
}

/// `0x00657a00`: walks a zero-terminated list of unit owner bytes (the byte
/// at unit offset `0x4c`) and invokes `visit` for every entry whose owner
/// equals `player`.  Returns the number of visits.
pub fn w15a02_visit_owned(owners: &[u8], player: u32, mut visit: impl FnMut(usize)) -> usize {
    let mut n = 0;
    for (i, &o) in owners.iter().enumerate() {
        if u32::from(o) == player {
            visit(i);
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests_wave15_agent02 {
    use super::*;

    #[test]
    fn table_lookup() {
        let t: Vec<u32> = (0..36).collect();
        assert_eq!(w15a02_player_table_value(&t, 2, 5), Some(29));
        assert_eq!(w15a02_player_table_value(&t, 3, 0), None);
    }

    #[test]
    fn record_helpers() {
        let r = W15a02Record {
            word0: -7,
            byte4: 10,
            byte5: 2,
            dword8: 5000,
        };
        assert_eq!(w15a02_record_word(&r, 0), 0);
        assert_eq!(w15a02_record_word(&r, 0xfa), 0);
        assert_eq!(w15a02_record_word(&r, 1), -7);
        // c = 10, limit <= c -> min(10, 20) + 6
        assert_eq!(w15a02_record_cost(&r, 1, 10), 16);
        assert_eq!(w15a02_record_cost(&r, 1, 11), 16);
        let r2 = W15a02Record {
            byte4: 3,
            dword8: 5000,
            byte5: 0,
            ..r
        };
        assert_eq!(w15a02_record_cost(&r2, 1, 5), 6);
        assert_eq!(w15a02_record_cost(&r2, 1, 50), 3);
        assert_eq!(w15a02_record_cost(&r2, 0, 5), 0);
    }

    #[test]
    fn guarded() {
        let mut failed = 0;
        let mut seen = 0;
        let r = w15a02_guarded_call(
            0x7c,
            9,
            true,
            || false,
            || failed += 1,
            |a| {
                seen = a;
                0
            },
        );
        assert_eq!((r, seen, failed), (0, 0x28, 1));
        let r = w15a02_guarded_call(1, 9, true, || true, || {}, |_| 5);
        assert_eq!(r, 0);
        let r = w15a02_guarded_call(1, 9, false, || true, || {}, |a| a as i32);
        assert_eq!(r, 9);
    }

    #[test]
    fn visit() {
        let mut hits = vec![];
        let n = w15a02_visit_owned(&[1, 2, 1], 1, |i| hits.push(i));
        assert_eq!((n, hits), (2, vec![0, 2]));
    }
}
