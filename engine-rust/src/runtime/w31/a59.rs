#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent59.rs
// ===========================================================================
// Wave 31 agent 59: JSON reader helpers (flag accessors, literal dispatch,
// UTF-8 validation copy).

/// `0x009645b0`: bit 7 of the flags word at offset 0x34.
pub fn w31a59_flag_bit7(flags: u32) -> u32 {
    (flags >> 7) & 1
}

/// `0x009645c0`: bit 8 of the flags word at offset 0x34.
pub fn w31a59_flag_bit8(flags: u32) -> u32 {
    (flags >> 8) & 1
}

/// `0x00964e30`: sets or clears bit 1 of the flags word at offset 0x34.
pub fn w31a59_set_flag_bit1(flags: u32, on: bool) -> u32 {
    if on { flags | 2 } else { flags & !2 }
}

/// Result of the value dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W31a59Value {
    /// `"` string value.
    String,
    /// Default branch (number).
    Number,
    /// `[` array.
    Array,
    /// `{` object.
    Object,
    /// `null` literal (a zeroed 16-byte value is pushed).
    Null,
    /// `true` / `false` literal accepted by the handler.
    Bool(bool),
    /// Parse error `(code, offset from start)`.
    Error(u32, usize),
}

/// `0x00968740`: JSON value dispatcher on the first byte.  `buf` is the
/// input, `pos` the cursor (updated like the original), `handler` models the
/// bool callee at `FUN_0096d310` (true = accepted).  Error 3 is a
/// literal mismatch, 0x10 a handler refusal.
pub fn w31a59_parse_value(
    buf: &[u8],
    pos: &mut usize,
    mut handler: impl FnMut(bool) -> bool,
) -> W31a59Value {
    let at = |i: usize| buf.get(i).copied().unwrap_or(0);
    let start = *pos;
    let first = at(start);
    let (word, val): (&[u8], Option<bool>) = match first {
        b'"' => return W31a59Value::String,
        b'[' => return W31a59Value::Array,
        b'{' => return W31a59Value::Object,
        b'f' => (b"false", Some(false)),
        b't' => (b"true", Some(true)),
        b'n' => (b"null", None),
        _ => return W31a59Value::Number,
    };
    for (i, &ch) in word.iter().enumerate().skip(1) {
        *pos = start + i;
        if at(*pos) != ch {
            return W31a59Value::Error(3, *pos);
        }
    }
    match val {
        None => {
            *pos = start + 4;
            W31a59Value::Null
        }
        Some(v) => {
            *pos = start + word.len();
            if handler(v) {
                W31a59Value::Bool(v)
            } else {
                W31a59Value::Error(0x10, *pos)
            }
        }
    }
}

/// `0x00969030`: validates one UTF-8 sequence, copying each consumed byte
/// to `out`.  `table` is the 256-byte class table (DAT_00d7a858); `next`
/// reads the next input byte.
pub fn w31a59_copy_utf8(
    table: &[u8; 256],
    out: &mut Vec<u8>,
    mut next: impl FnMut() -> u8,
) -> bool {
    let mut get = |out: &mut Vec<u8>| {
        let b = next();
        out.push(b);
        b
    };
    let b = get(out);
    if b < 0x80 {
        return true;
    }
    let t = |x: u8| table[x as usize];
    match t(b).wrapping_sub(2) {
        0 => {
            let c = get(out);
            t(c) & 0x70 != 0
        }
        1 => {
            let c1 = get(out);
            let c2 = get(out);
            t(c2) & 0x70 != 0 && t(c1) & 0x70 != 0
        }
        2 => {
            let c1 = get(out);
            let c2 = get(out);
            t(c2) & 0x70 != 0 && t(c1) & 0x50 != 0
        }
        3 => {
            let c1 = get(out);
            let c2 = get(out);
            let c3 = get(out);
            t(c3) & 0x70 != 0 && t(c2) & 0x70 != 0 && (t(c1) >> 4) & 1 != 0
        }
        4 => {
            let c1 = get(out);
            let c2 = get(out);
            let c3 = get(out);
            t(c1) & 0x70 != 0 && t(c2) & 0x70 != 0 && t(c3) & 0x70 != 0
        }
        8 => {
            let c1 = get(out);
            let c2 = get(out);
            t(c2) & 0x70 != 0 && (t(c1) >> 5) & 1 != 0
        }
        9 => {
            let c1 = get(out);
            let c2 = get(out);
            let c3 = get(out);
            t(c1) & 0x60 != 0 && t(c2) & 0x70 != 0 && t(c3) & 0x70 != 0
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests_wave31_agent59 {
    use super::*;

    #[test]
    fn flags() {
        assert_eq!(w31a59_flag_bit7(0x80), 1);
        assert_eq!(w31a59_flag_bit7(0x100), 0);
        assert_eq!(w31a59_flag_bit8(0x100), 1);
        assert_eq!(w31a59_set_flag_bit1(0, true), 2);
        assert_eq!(w31a59_set_flag_bit1(7, false), 5);
    }

    #[test]
    fn literals() {
        let mut p = 0;
        assert_eq!(
            w31a59_parse_value(b"true,", &mut p, |_| true),
            W31a59Value::Bool(true)
        );
        assert_eq!(p, 4);
        let mut p = 0;
        assert_eq!(
            w31a59_parse_value(b"false", &mut p, |_| false),
            W31a59Value::Error(0x10, 5)
        );
        let mut p = 0;
        assert_eq!(
            w31a59_parse_value(b"nul!", &mut p, |_| true),
            W31a59Value::Error(3, 3)
        );
        let mut p = 0;
        assert_eq!(
            w31a59_parse_value(b"null", &mut p, |_| true),
            W31a59Value::Null
        );
        let mut p = 0;
        assert_eq!(
            w31a59_parse_value(b"[1]", &mut p, |_| true),
            W31a59Value::Array
        );
        assert_eq!(
            w31a59_parse_value(b"7", &mut 0, |_| true),
            W31a59Value::Number
        );
    }

    #[test]
    fn utf8() {
        let mut table = [0u8; 256];
        for b in 0x80..0xc0 {
            table[b] = 0x70;
        }
        table[0xc3] = 2;
        table[0xe2] = 3;
        let mut out = Vec::new();
        let src = [0xc3u8, 0xa9];
        let mut i = 0;
        let ok = w31a59_copy_utf8(&table, &mut out, || {
            i += 1;
            src[i - 1]
        });
        assert!(ok);
        assert_eq!(out, src);
        let mut out = Vec::new();
        assert!(w31a59_copy_utf8(&table, &mut out, || b'a'));
        let mut out = Vec::new();
        assert!(!w31a59_copy_utf8(&table, &mut out, || 0x80));
    }
}
