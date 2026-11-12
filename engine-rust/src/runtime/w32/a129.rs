#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent129.rs
// ===========================================================================
// Wave 32 agent 129: CRT float-scan helpers for "INF"/"INFINITY" and "NAN(...)".
// Callees FUN_00c97faf (unget) and FUN_00c95719 (rewind) are modelled on the
// cursor.  FUN_00c94e65 is skipped: it depends on opaque locale/state structs.

/// Character cursor over a byte stream (models the `char *` cell plus current char).
#[derive(Debug, Clone)]
pub struct W32a129Cursor<'a> {
    pub buf: &'a [u8],
    pub pos: usize,
    pub cur: u8,
}

impl<'a> W32a129Cursor<'a> {
    fn advance(&mut self) {
        let c = self.buf.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        self.cur = c;
    }
    /// FUN_00c97faf equivalent: step the read pointer back by one.
    fn unget(&mut self) {
        self.pos = self.pos.saturating_sub(1);
    }
}

fn w32a129_match_ci(c: &mut W32a129Cursor, upper: &[u8], lower: &[u8]) -> bool {
    let mut i = 0;
    while i < upper.len() && (c.cur == upper[i] || c.cur == lower[i]) {
        c.advance();
        i += 1;
        if i == upper.len() {
            return true;
        }
    }
    false
}

/// `0x00c9525c`: scans "INF" then optionally "INITY" (case-insensitive via
/// the two 3-byte tables).  Returns 3 on INF/INFINITY, 7 on mismatch
/// (cursor rewound to `start`, current char cleared).
pub fn w32a129_scan_inf(
    c: &mut W32a129Cursor,
    upper3: &[u8; 3],
    lower3: &[u8; 3],
    start: usize,
) -> u32 {
    if !w32a129_match_ci(c, upper3, lower3) {
        c.unget();
        c.cur = 0;
        c.pos = start;
        return 7;
    }
    c.unget();
    let saved = c.pos;
    c.advance();
    if w32a129_match_ci(c, b"INITY", b"inity") {
        c.unget();
        return 3;
    }
    c.unget();
    c.cur = 0;
    c.pos = saved;
    3
}

/// `0x00c95300`: scans "NAN" then optional "(...)".  Returns 7 on a bad
/// prefix, 5 / 6 for the two special payload spellings, 4 for plain NAN
/// (also the result of a malformed payload, after rewinding to `start`).
pub fn w32a129_scan_nan(
    c: &mut W32a129Cursor,
    upper3: &[u8; 3],
    lower3: &[u8; 3],
    payload5: (&[u8; 5], &[u8; 5]),
    payload4: (&[u8; 4], &[u8; 4]),
    start: usize,
) -> u32 {
    let rewind = |c: &mut W32a129Cursor| {
        c.unget();
        c.cur = 0;
        c.pos = start;
    };
    if !w32a129_match_ci(c, upper3, lower3) {
        rewind(c);
        return 7;
    }
    c.unget();
    c.advance();
    if c.cur != b'(' {
        rewind(c);
        return 4;
    }
    c.advance();
    if w32a129_match_ci(c, payload5.0, payload5.1) {
        c.unget();
        return 5;
    }
    if w32a129_match_ci(c, payload4.0, payload4.1) {
        c.unget();
        return 6;
    }
    while c.cur != b')' {
        let ok = c.cur != 0 && (c.cur.is_ascii_alphanumeric() || c.cur == b'_');
        if !ok {
            rewind(c);
            return 4;
        }
        c.advance();
    }
    4
}

#[cfg(test)]
mod tests_wave32_agent129 {
    use super::*;

    #[test]
    fn inf_variants() {
        let mut c = W32a129Cursor {
            buf: b"nfinity!",
            pos: 0,
            cur: b'i',
        };
        assert_eq!(w32a129_scan_inf(&mut c, b"INF", b"inf", 0), 3);
        let mut c = W32a129Cursor {
            buf: b"xyz",
            pos: 1,
            cur: b'q',
        };
        assert_eq!(w32a129_scan_inf(&mut c, b"INF", b"inf", 0), 7);
        assert_eq!((c.cur, c.pos), (0, 0));
    }

    #[test]
    fn nan_variants() {
        let p5 = (b"SNAN)", b"snan)");
        let p4 = (b"IND)", b"ind)");
        let mut c = W32a129Cursor {
            buf: b"AN(ab_1)",
            pos: 0,
            cur: b'N',
        };
        assert_eq!(w32a129_scan_nan(&mut c, b"NAN", b"nan", p5, p4, 0), 4);
        let mut c = W32a129Cursor {
            buf: b"AN(ind)",
            pos: 0,
            cur: b'n',
        };
        assert_eq!(w32a129_scan_nan(&mut c, b"NAN", b"nan", p5, p4, 0), 6);
        let mut c = W32a129Cursor {
            buf: b"xx",
            pos: 0,
            cur: b'x',
        };
        assert_eq!(w32a129_scan_nan(&mut c, b"NAN", b"nan", p5, p4, 0), 7);
    }
}
