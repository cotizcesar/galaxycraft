#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent01.rs
// ===========================================================================
// Wave 22 translation batch 01: JSON string scanner and hash-bucket helper.

/// Result of [`w22a01_parse_json_string`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W22a01StringResult {
    /// Position of the cursor after the scan (index into the input).
    pub pos: usize,
    /// Parse error code (0 = ok), stored at parser `+0x18`.
    pub error: u32,
    /// Error offset, stored at parser `+0x1c`.
    pub error_offset: usize,
    /// Decoded bytes without the trailing NUL terminator.
    pub bytes: Vec<u8>,
}

fn w22a01_hex4(src: &[u8], pos: &mut usize) -> Option<u32> {
    let mut v = 0u32;
    for _ in 0..4 {
        let c = src.get(*pos).copied().unwrap_or(0);
        let d = match c {
            b'0'..=b'9' => c - b'0',
            b'A'..=b'F' => c - b'A' + 10,
            b'a'..=b'f' => c - b'a' + 10,
            _ => return None,
        };
        v = v * 16 + d as u32;
        *pos += 1;
    }
    Some(v)
}

fn w22a01_push_utf8(out: &mut Vec<u8>, cp: u32) {
    match char::from_u32(cp) {
        Some(ch) => {
            let mut b = [0u8; 4];
            out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
        }
        None => out.push(b'?'),
    }
}

/// `0x0061d100`: scans a JSON string body.  `src[start]` is the opening
/// quote.  `escape_table` is the 256-entry simple-escape table (retail
/// table `d3ad78`; zero means "not a simple escape").  Handles `\uXXXX`
/// with surrogate pairs.  Error codes: 8 bad hex, 9 bad surrogate, 10
/// unknown escape, 11 unterminated, 12 control char, 0x10 when
/// `validate` (the retail validator `61ef80`) rejects the decoded text.
pub fn w22a01_parse_json_string(
    src: &[u8],
    start: usize,
    escape_table: &[u8; 256],
    validate: impl Fn(&[u8]) -> bool,
) -> W22a01StringResult {
    let mut out: Vec<u8> = Vec::new();
    let mut pos = start + 1;
    let mut error = 0u32;
    let mut err_off = 0usize;
    let at = |p: usize| src.get(p).copied().unwrap_or(0);
    loop {
        let c = at(pos);
        if c == b'\\' {
            let esc_off = pos;
            let e = at(pos + 1);
            let t = escape_table[e as usize];
            if t != 0 {
                out.push(t);
                pos += 2;
                continue;
            }
            if e != b'u' {
                error = 10;
                err_off = esc_off;
                pos += 1;
                break;
            }
            pos += 2;
            let Some(cp) = w22a01_hex4(src, &mut pos) else {
                error = 8;
                err_off = esc_off;
                break;
            };
            if cp.wrapping_sub(0xd800) >= 0x400 {
                w22a01_push_utf8(&mut out, cp);
                continue;
            }
            if at(pos) == b'\\' && at(pos + 1) == b'u' {
                pos += 2;
                match w22a01_hex4(src, &mut pos) {
                    None => {
                        error = 8;
                        err_off = esc_off;
                    }
                    Some(lo) if lo.wrapping_sub(0xdc00) < 0x400 => {
                        let full = (((cp - 0xd800) * 0x400) | (lo - 0xdc00)) + 0x10000;
                        w22a01_push_utf8(&mut out, full);
                        continue;
                    }
                    Some(_) => {
                        error = 9;
                        err_off = esc_off;
                    }
                }
            } else {
                error = 9;
                err_off = esc_off;
            }
            break;
        }
        if c == b'"' {
            pos += 1;
            break;
        }
        if c < 0x20 {
            error = if c == 0 { 0xb } else { 0xc };
            err_off = pos;
            break;
        }
        out.push(c);
        pos += 1;
    }
    if error == 0 && !validate(&out) {
        error = 0x10;
        err_off = pos;
    }
    W22a01StringResult {
        pos,
        error,
        error_offset: err_off,
        bytes: out,
    }
}

/// `0x0060aaf0`: bucket index of the 32-bit hash map (finalizer mix of the
/// key, masked with `capacity - 1`) and chain search.  `nodes[i] =
/// (next, key)` with `next` an index; `buckets[b]` is the chain head.
/// Returns the node index holding `key`, or `None` (retail then aborts).
pub fn w22a01_hash_bucket_find(
    capacity: u32,
    buckets: &[Option<usize>],
    nodes: &[(Option<usize>, u32)],
    key: u32,
) -> Option<usize> {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    h = (h >> 16 ^ h) & capacity.wrapping_sub(1);
    let mut cur = *buckets.get(h as usize)?;
    while let Some(i) = cur {
        let (next, k) = *nodes.get(i)?;
        if k == key {
            return Some(i);
        }
        cur = next;
    }
    None
}

#[cfg(test)]
mod tests_wave22_agent01 {
    use super::*;

    fn table() -> [u8; 256] {
        let mut t = [0u8; 256];
        t[b'n' as usize] = b'\n';
        t[b'"' as usize] = b'"';
        t[b'\\' as usize] = b'\\';
        t
    }

    fn run(s: &str) -> W22a01StringResult {
        w22a01_parse_json_string(s.as_bytes(), 0, &table(), |_| true)
    }

    #[test]
    fn json_string_basic_and_escapes() {
        let b = '\\';
        let r = run(&format!("\"a{b}nb{b}u00e9\""));
        assert_eq!(r.error, 0);
        assert_eq!(r.bytes, [b'a', b'\n', b'b', 0xc3, 0xa9]);
        assert_eq!(r.pos, 12);
    }

    #[test]
    fn json_string_surrogates_and_errors() {
        let b = '\\';
        let r = run(&format!("\"{b}ud83d{b}ude00\""));
        assert_eq!(r.bytes, [0xf0, 0x9f, 0x98, 0x80]);
        let r = run(&format!("\"{b}ud83dx\""));
        assert_eq!((r.error, r.error_offset), (9, 1));
        assert_eq!(run(&format!("\"{b}q\"")).error, 10);
        assert_eq!(run(&format!("\"{b}u12g4\"")).error, 8);
        let r = run("\"ab");
        assert_eq!((r.error, r.error_offset), (0xb, 3));
        assert_eq!(run("\"a\x01\"").error, 0xc);
        let r = w22a01_parse_json_string(b"\"a\"", 0, &table(), |_| false);
        assert_eq!(r.error, 0x10);
    }

    #[test]
    fn hash_bucket_lookup() {
        let key = 0x1234_5678u32;
        let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
        h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
        let b = ((h >> 16 ^ h) & 7) as usize;
        let mut buckets = vec![None; 8];
        buckets[b] = Some(0);
        let nodes = [(Some(1), 99), (None, key)];
        assert_eq!(w22a01_hash_bucket_find(8, &buckets, &nodes, key), Some(1));
        assert_eq!(w22a01_hash_bucket_find(8, &buckets, &nodes, 5), None);
    }
}
