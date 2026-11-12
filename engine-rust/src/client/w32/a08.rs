#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent08.rs
// ===========================================================================
// Wave 32 translation batch 08.
//
// Skipped: FUN_007ae350 (SEH frame, ends in a non-returning call).

fn w32a08_u16(t: &[u8], o: usize) -> u16 {
    match (t.get(o), t.get(o + 1)) {
        (Some(a), Some(b)) => u16::from_le_bytes([*a, *b]),
        _ => 0,
    }
}

fn w32a08_u8(t: &[u8], o: usize) -> u8 {
    t.get(o).copied().unwrap_or(0)
}

/// `0x007a3880`: resolve a numbered entry (optionally with a trailing letter
/// suffix) inside a table of 8-byte records.  `parse` is the strtol-like
/// callee returning `(value, rest_of_text)`; `lower` is the tolower callee.
/// Returns the `u16` at record offset 2, or `None`.
pub fn w32a08_lookup_numbered(
    text: &[u8],
    table: &[u8],
    skip: i32,
    parse: impl Fn(&[u8]) -> (i32, &[u8]),
    lower: impl Fn(u8) -> i32,
) -> Option<u32> {
    let (value, rest) = parse(text);
    if value - skip < 0 {
        return None;
    }
    let mut n = value - skip;
    let mut p = 0usize;
    let mut cnt = w32a08_u16(table, p + 2);
    loop {
        if cnt == 0 {
            return None;
        }
        let mut next = n;
        if w32a08_u16(table, p + 4) == 0 && w32a08_u8(table, p + 7) == 0 {
            next = n.wrapping_sub(1);
            if n == 0 {
                break;
            }
        }
        cnt = w32a08_u16(table, p + 10);
        n = next;
        p += 8;
    }
    if let Some(&c) = rest.first() {
        if c != 0 {
            let mut k = lower(c) - 0x61;
            while k != 0 {
                k -= 1;
                if w32a08_u16(table, p + 2) == 0 {
                    return None;
                }
                let flag = w32a08_u8(table, p + 15);
                p += 8;
                if flag == 0 {
                    return None;
                }
            }
            if w32a08_u16(table, p + 2) == 0 {
                return None;
            }
        }
    }
    Some(w32a08_u16(table, p + 2) as u32)
}

/// `0x007a76b0`: true when the UTF-8 sequence at the start of `s` decodes
/// (via `decode(seq)`, the callee returning 0xfffd on error) to a valid
/// scalar.  Invalid lead bytes yield false.
pub fn w32a08_utf8_valid(s: &[u8], decode: impl Fn(&[u8]) -> u32) -> bool {
    let b = match s.first() {
        Some(&b) => b,
        None => return false,
    };
    let len = if b < 0x80 {
        1
    } else if (b & 0xe0) == 0xc0 && b > 0xc1 {
        2
    } else if (b & 0xf0) == 0xe0 {
        3
    } else if (b & 0xf8) == 0xf0 && b <= 0xf4 {
        4
    } else {
        return false;
    };
    decode(&s[..len.min(s.len())]) != 0xfffd
}

#[cfg(test)]
mod tests_wave32_agent08 {
    use super::*;

    #[test]
    fn utf8() {
        let d = |s: &[u8]| {
            std::str::from_utf8(s)
                .ok()
                .and_then(|x| x.chars().next())
                .map_or(0xfffd, |c| c as u32)
        };
        assert!(w32a08_utf8_valid(b"a", d));
        assert!(w32a08_utf8_valid("é".as_bytes(), d));
        assert!(!w32a08_utf8_valid(&[0xc0, 0x80], d));
        assert!(!w32a08_utf8_valid(&[0xf5, 0, 0, 0], d));
    }

    #[test]
    fn lookup() {
        // records: [_, cnt, a, b, _, _, flag?]
        let mut t = vec![0u8; 8 * 4];
        t[2] = 1;
        t[10] = 1;
        t[18] = 1;
        t[26] = 7;
        fn parse(s: &[u8]) -> (i32, &[u8]) {
            (s[0] as i32 - b'0' as i32, &s[1..])
        }
        let low = |c: u8| c.to_ascii_lowercase() as i32;
        assert_eq!(w32a08_lookup_numbered(b"0", &t, 0, parse, low), Some(1));
        assert_eq!(w32a08_lookup_numbered(b"2", &t, 3, parse, low), None);
    }
}
