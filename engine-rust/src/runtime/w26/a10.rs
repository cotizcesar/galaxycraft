#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent10.rs
// ===========================================================================
// Wave 26 agent 10: small string/array helpers from the 0x0092xxxx block.

/// `0x0092d2e0`: builds the CRLF-normalised log line.  Starts with "\r\n",
/// inserts `\r` before every `\n` not already preceded by `\r`, and appends
/// "\r\n" unless the text already ends in `\n`.
pub fn w26a10_crlf_line(text: &[u8]) -> Vec<u8> {
    let mut out = vec![b'\r', b'\n'];
    let mut prev = 0u8;
    let mut last = 0u8;
    for &c in text {
        if c == b'\n' && prev != b'\r' {
            out.push(b'\r');
        }
        out.push(c);
        prev = c;
        last = c;
    }
    if text.is_empty() || last != b'\n' {
        out.extend_from_slice(b"\r\n");
    }
    out
}

/// `0x0092d4d0`: debug-memory label.  Only produced when the cached
/// "Internal/Debug Memory" flag is set; `"%s (%s)"` when `extra` is
/// non-empty, otherwise just `name`; truncated to 255 bytes (buffer 0x100).
pub fn w26a10_debug_memory_label(flag: bool, name: &str, extra: Option<&str>) -> Option<String> {
    if !flag {
        return None;
    }
    let mut s = match extra {
        Some(e) if !e.is_empty() => format!("{name} ({e})"),
        _ => name.to_string(),
    };
    s.truncate(255);
    Some(s)
}

/// `0x0092c340`: string-table id chosen by `kind_zero` (param_1 == 0) and
/// the result of the probe `FUN_0094a880` (`probe_nonzero`):
/// probe 0 -> 9 / 10, probe nonzero -> 12 / 13.
pub fn w26a10_message_id(kind_zero: bool, probe_nonzero: bool) -> u32 {
    match (probe_nonzero, kind_zero) {
        (false, true) => 9,
        (false, false) => 10,
        (true, true) => 12,
        (true, false) => 13,
    }
}

/// `0x0092d720`: index of element `base + idx` in a growable u32 array,
/// zero-extending the array to `base + idx + 1` entries when the index is
/// at or beyond the current length.
pub fn w26a10_array_slot(v: &mut Vec<u32>, base: usize, idx: usize) -> usize {
    let u = base + idx;
    if v.len() <= u {
        v.resize(u + 1, 0);
    }
    u
}

#[cfg(test)]
mod tests_wave26_agent10 {
    use super::*;

    #[test]
    fn crlf() {
        assert_eq!(w26a10_crlf_line(b"a\nb"), b"\r\na\r\nb\r\n");
        assert_eq!(w26a10_crlf_line(b"a\r\n"), b"\r\na\r\n");
        assert_eq!(w26a10_crlf_line(b""), b"\r\n\r\n");
        assert_eq!(w26a10_crlf_line(b"x\n"), b"\r\nx\r\n");
    }

    #[test]
    fn label() {
        assert_eq!(w26a10_debug_memory_label(false, "a", None), None);
        assert_eq!(
            w26a10_debug_memory_label(true, "a", Some("b")).unwrap(),
            "a (b)"
        );
        assert_eq!(w26a10_debug_memory_label(true, "a", Some("")).unwrap(), "a");
        assert_eq!(
            w26a10_debug_memory_label(true, &"z".repeat(300), None)
                .unwrap()
                .len(),
            255
        );
    }

    #[test]
    fn ids() {
        assert_eq!(w26a10_message_id(true, false), 9);
        assert_eq!(w26a10_message_id(false, false), 10);
        assert_eq!(w26a10_message_id(true, true), 12);
        assert_eq!(w26a10_message_id(false, true), 13);
    }

    #[test]
    fn slot() {
        let mut v = vec![1, 2];
        assert_eq!(w26a10_array_slot(&mut v, 1, 0), 1);
        assert_eq!(v.len(), 2);
        assert_eq!(w26a10_array_slot(&mut v, 1, 3), 4);
        assert_eq!(v, vec![1, 2, 0, 0, 0]);
    }
}
