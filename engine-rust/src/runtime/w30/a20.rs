#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent20.rs
// ===========================================================================
// Wave 30 agent 20: recoverable guard logic from the 0xb3e..0xb43 container /
// MultiProcessIndex glue.  Everything else in the queue is SEH/vtable glue.

/// `0x00b3e430`: shifts `offset` right by `shift` and validates the span
/// index: it must be below `0x3ff` and survive the optional round-trip check.
pub fn w30a20_span_index(offset: u64, shift: u8, roundtrip_ok: Option<bool>) -> Option<u32> {
    let v = (offset >> (shift & 63)) as u32;
    let same = roundtrip_ok.unwrap_or(true);
    if v < 0x3ff && same { Some(v) } else { None }
}

/// `0x00b3f360`: the buffer must hold at least `count` headers of `0x1e` bytes.
pub fn w30a20_headers_fit(buf_len: u32, count: u8) -> bool {
    buf_len >= u32::from(count) * 0x1e
}

/// `0x00b43050`: clamps the filled prefix length of a 9-byte key slot and
/// returns the number of trailing bytes to zero (`9 - min(len, 9)`).
pub fn w30a20_zero_tail_len(len: u32) -> u32 {
    9 - len.min(9)
}

/// `0x00b41c20`: maps a block kind to its table offset (4 -> `0x150`,
/// 5 -> `0x154`); other kinds yield `None` (the function returns 0).
pub fn w30a20_kind_offset(kind: i32) -> Option<u32> {
    match kind {
        4 => Some(0x150),
        5 => Some(0x154),
        _ => None,
    }
}

/// `0x00b410a0`: recursive-owner bookkeeping: when no owner is set the
/// caller becomes owner with depth 1, otherwise the depth is incremented.
pub fn w30a20_acquire_owner(owner: &mut u32, depth: &mut u32, caller: u32) {
    if *owner == 0 {
        *owner = caller;
        *depth = 1;
    } else {
        *depth += 1;
    }
}

/// `0x00b427f0`: true when the NUL-terminated name plus 7 bytes still fits
/// the 255-byte stack buffer (length incl. NUL + 7 - 1 < 0x100 check).
pub fn w30a20_name_fits(name: &[u8]) -> bool {
    let len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
    len + 7 < 0x100
}

#[cfg(test)]
mod tests_wave30_agent20 {
    use super::*;

    #[test]
    fn span_index() {
        assert_eq!(w30a20_span_index(0x1000, 4, None), Some(0x100));
        assert_eq!(w30a20_span_index(0x3ff, 0, None), None);
        assert_eq!(w30a20_span_index(0x3fe, 0, Some(false)), None);
        assert_eq!(w30a20_span_index(0x3fe, 0, Some(true)), Some(0x3fe));
    }

    #[test]
    fn headers() {
        assert!(w30a20_headers_fit(0x1e * 3, 3));
        assert!(!w30a20_headers_fit(0x1e * 3 - 1, 3));
    }

    #[test]
    fn zero_tail() {
        assert_eq!(w30a20_zero_tail_len(0), 9);
        assert_eq!(w30a20_zero_tail_len(20), 0);
    }

    #[test]
    fn kinds() {
        assert_eq!(w30a20_kind_offset(4), Some(0x150));
        assert_eq!(w30a20_kind_offset(5), Some(0x154));
        assert_eq!(w30a20_kind_offset(6), None);
    }

    #[test]
    fn owner() {
        let (mut o, mut d) = (0, 0);
        w30a20_acquire_owner(&mut o, &mut d, 7);
        w30a20_acquire_owner(&mut o, &mut d, 7);
        assert_eq!((o, d), (7, 2));
    }

    #[test]
    fn name() {
        assert!(w30a20_name_fits(b"abc\0"));
        assert!(!w30a20_name_fits(&[b'a'; 249]));
        assert!(w30a20_name_fits(&[b'a'; 248]));
    }
}
