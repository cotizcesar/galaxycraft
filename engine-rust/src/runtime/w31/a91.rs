#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent91.rs
// ===========================================================================
// Wave 31 agent 91: logger/number-formatting helpers from the runtime block.

/// `0x00b8edd0`: parses a run of ASCII decimal digits at the start of
/// `text` into a wrapping 64-bit value.  Returns `(value, digits_consumed)`;
/// parsing stops at the first non-digit (or the end of the slice).
pub fn w31a91_parse_decimal_u64(text: &[u8]) -> (u64, usize) {
    let mut value: u64 = 0;
    let mut used = 0usize;
    while let Some(&c) = text.get(used) {
        if c.wrapping_sub(b'0') > 9 {
            break;
        }
        value = value.wrapping_mul(10).wrapping_add(u64::from(c - b'0'));
        used += 1;
    }
    (value, used)
}

/// `0x00b8f390`: picks the symbolic label passed to the value formatter.
/// Zero uses the caller-supplied `zero_label`, `-1` (as i32) is
/// `"UINT_MAX"`, every other value has no label.  Returns `(value, label)`.
pub fn w31a91_value_label(value: i32, zero_label: &str) -> (i32, Option<&str>) {
    if value == 0 {
        (0, Some(zero_label))
    } else if value == -1 {
        (-1, Some("UINT_MAX"))
    } else {
        (value, None)
    }
}

/// `0x00b8e850`: builds the log-line prefix for a severity level and an
/// optional subsystem.  Each piece is `(text, length)` where the length is
/// the count the original appends (it includes the NUL for the fixed
/// strings, and excludes it for the subsystem name).  Levels above 5 emit
/// no level piece.
pub fn w31a91_level_prefix<'a>(level: u32, subsystem: Option<&'a str>) -> Vec<(&'a str, usize)> {
    let mut out: Vec<(&'a str, usize)> = Vec::new();
    match level {
        0 => out.push(("level=NOISE", 0xc)),
        1 => out.push(("level=DEBUG", 0xc)),
        2 => out.push(("level=INFO", 0xb)),
        3 => out.push(("level=WARNING", 0xe)),
        4 => out.push(("level=ERROR", 0xc)),
        5 => out.push(("level=FATAL", 0xc)),
        _ => {}
    }
    if let Some(name) = subsystem {
        out.push(("|subsystem=", 0xc));
        out.push((name, name.len()));
    }
    out
}

/// Bounded log buffer used by `0x00b8f640`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W31a91LogBuf {
    /// Optional running total of the parent (`*(owner + 0x10)`).
    pub parent_total: Option<usize>,
    /// Pending NUL-free text still to flush (`in_ECX[1]`).
    pub pending: Option<Vec<u8>>,
    /// Backing storage; its length is the capacity (`in_ECX[3]`).
    pub data: Vec<u8>,
    /// Bytes used (`in_ECX[4]`).
    pub len: usize,
}

/// `0x00b8f640`: flushes the pending text into the bounded buffer
/// (truncating silently at capacity), clears the pending slot, writes a
/// terminating NUL (into the last byte when full), then adds the used
/// length to the parent's running total.
pub fn w31a91_flush_log_buf(b: &mut W31a91LogBuf) {
    let cap = b.data.len();
    if let Some(p) = b.pending.take() {
        let mut pos = b.len;
        for &c in p.iter().take_while(|&&c| c != 0) {
            if pos < cap {
                b.data[pos] = c;
                pos += 1;
            }
        }
        b.len = pos;
    }
    if b.len < cap {
        b.data[b.len] = 0;
    } else if cap != 0 {
        b.data[cap - 1] = 0;
    }
    if let Some(t) = b.parent_total.as_mut() {
        *t += b.len;
    }
}

#[cfg(test)]
mod tests_wave31_agent91 {
    use super::*;

    #[test]
    fn parse_decimal() {
        assert_eq!(w31a91_parse_decimal_u64(b"1234x"), (1234, 4));
        assert_eq!(w31a91_parse_decimal_u64(b"x"), (0, 0));
        assert_eq!(
            w31a91_parse_decimal_u64(b"18446744073709551615"),
            (u64::MAX, 20)
        );
    }

    #[test]
    fn labels() {
        assert_eq!(w31a91_value_label(0, "zero"), (0, Some("zero")));
        assert_eq!(w31a91_value_label(-1, "zero"), (-1, Some("UINT_MAX")));
        assert_eq!(w31a91_value_label(5, "zero"), (5, None));
    }

    #[test]
    fn prefix() {
        let p = w31a91_level_prefix(3, Some("net"));
        assert_eq!(p[0], ("level=WARNING", 0xe));
        assert_eq!(p[1], ("|subsystem=", 0xc));
        assert_eq!(p[2], ("net", 3));
        assert!(w31a91_level_prefix(9, None).is_empty());
    }

    #[test]
    fn flush() {
        let mut b = W31a91LogBuf {
            parent_total: Some(1),
            pending: Some(b"abcdef".to_vec()),
            data: vec![9; 4],
            len: 1,
        };
        w31a91_flush_log_buf(&mut b);
        assert_eq!(b.len, 4);
        assert_eq!(&b.data, &[9, b'b' - 1, b'c' - 1, 0]);
        assert_eq!(b.parent_total, Some(5));
        assert!(b.pending.is_none());
    }
}
