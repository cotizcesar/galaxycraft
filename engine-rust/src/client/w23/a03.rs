#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent03.rs
// ===========================================================================
// Wave 23 translation batch 03.
//
// Queue window, module `client.rs`.  Most of the assignment is SEH/CRT
// glue, path-string building, non-returning call chains or bnet; only the
// small routines with visible logic are translated.

/// Record touched by `0x00715f10` (only the two fields it writes).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct W23a03Record {
    /// Field at offset `0x3e`.
    pub word_3e: u16,
    /// Flags at offset `0x30`.
    pub flags_30: u32,
}

/// `0x00709680`: true when the cached global equals `param`.
pub fn w23a03_matches_cached(cached: u32, param: u32) -> bool {
    cached == param
}

/// `0x00715f10`: looks up a record (callee with selector `-2`); when found,
/// copies `table[index]` into the `0x3e` word and ORs `flags` into `0x30`.
/// Returns whether a record was found.
pub fn w23a03_set_record_flags(
    record: Option<&mut W23a03Record>,
    table: &[u16],
    index: u16,
    flags: u32,
) -> bool {
    match record {
        Some(r) => {
            r.word_3e = table.get(index as usize).copied().unwrap_or(0);
            r.flags_30 |= flags;
            true
        }
        None => false,
    }
}

/// `0x00716de0`: classifies each map tile and reports `(x, y, a, b)` through
/// `emit` in row-major order.  `flags` holds `width * height` tile words;
/// `mask_a`/`mask_b` are the two visibility masks, `shared` is the
/// replay/observer switch, `mode` the second switch and `vis` the packed
/// player mask.  `fallback` stands for the callee evaluated only when
/// `shared` is zero.  Nothing happens when `enabled` is false.
#[allow(clippy::too_many_arguments)]
pub fn w23a03_classify_tiles(
    enabled: bool,
    width: usize,
    height: usize,
    flags: &[u32],
    mask_a: u32,
    mask_b: u32,
    shared: u32,
    mode: u32,
    vis: u32,
    mut fallback: impl FnMut() -> bool,
    mut emit: impl FnMut(usize, usize, bool, bool),
) {
    if !enabled {
        return;
    }
    for y in 0..height {
        for x in 0..width {
            let t = flags.get(y * width + x).copied().unwrap_or(0);
            let mut a = t & mask_a == 0;
            let mut b = t & mask_b == 0;
            if shared != 0 || fallback() {
                if mode == 0 {
                    if !t & ((vis & 0xff) << 8) != 0 {
                        if (vis as u8) & (!t as u8) == 0 {
                            a = false;
                            b = true;
                        } else {
                            a = true;
                            b = false;
                        }
                    } else {
                        a = false;
                        b = false;
                    }
                } else {
                    a = true;
                    b = false;
                }
            }
            emit(x, y, a, b);
        }
    }
}

#[cfg(test)]
mod tests_wave23_agent03 {
    use super::*;

    #[test]
    fn cached_match() {
        assert!(w23a03_matches_cached(5, 5));
        assert!(!w23a03_matches_cached(5, 6));
    }

    #[test]
    fn record_flags() {
        let mut r = W23a03Record::default();
        assert!(w23a03_set_record_flags(Some(&mut r), &[1, 2, 3], 2, 0x10));
        assert!(w23a03_set_record_flags(Some(&mut r), &[1, 2, 3], 1, 0x01));
        assert_eq!(
            r,
            W23a03Record {
                word_3e: 2,
                flags_30: 0x11
            }
        );
        assert!(!w23a03_set_record_flags(None, &[1], 0, 1));
    }

    fn run(t: u32, shared: u32, mode: u32, vis: u32, fb: bool) -> (bool, bool) {
        let mut out = (false, false);
        w23a03_classify_tiles(
            true,
            1,
            1,
            &[t],
            1,
            2,
            shared,
            mode,
            vis,
            || fb,
            |_, _, a, b| out = (a, b),
        );
        out
    }

    #[test]
    fn classify() {
        assert_eq!(run(0, 0, 0, 0, false), (true, true));
        assert_eq!(run(1, 0, 0, 0, false), (false, true));
        assert_eq!(run(0, 1, 1, 0, false), (true, false));
        assert_eq!(run(0, 0, 0, 0x01, true), (true, false));
        assert_eq!(run(0, 1, 0, 0x01, false), (true, false));
        assert_eq!(run(0, 1, 0, 0x100, false), (false, false));
        assert_eq!(run(0x100, 1, 0, 0x101, false), (false, false));
        assert_eq!(run(0x1ff, 1, 0, 0x101, false), (false, false));
        assert_eq!(run(0xff, 1, 0, 0xff, false), (false, true));
    }

    #[test]
    fn disabled_and_order() {
        let mut n = 0;
        w23a03_classify_tiles(
            false,
            2,
            2,
            &[0; 4],
            0,
            0,
            1,
            1,
            0,
            || true,
            |_, _, _, _| n += 1,
        );
        assert_eq!(n, 0);
        let mut v = vec![];
        w23a03_classify_tiles(
            true,
            2,
            2,
            &[0; 4],
            0,
            0,
            1,
            1,
            0,
            || true,
            |x, y, _, _| v.push((x, y)),
        );
        assert_eq!(v, vec![(0, 0), (1, 0), (0, 1), (1, 1)]);
    }
}
