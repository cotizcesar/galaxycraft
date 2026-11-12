#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent20.rs
// ===========================================================================
// Wave 29 translation batch 20: small-buffer sequence helpers and a
// key/brace-list header scanner.

fn w29a20_is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | 0x0b | b'\r' | 0x0c | b'\n')
}

/// Error marker for the paths where the retail code calls a non-returning
/// allocator/throw helper.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W29a20Throw;

/// `0x00aedfb0`: scans `name : { a , b }` in a NUL-terminated buffer.
/// Returns `Ok(true)` when only a name is present (count untouched),
/// `Ok(false)` otherwise; `count` receives the number of top-level items.
/// The name start index is returned through `name_start`.
pub fn w29a20_parse_header(
    buf: &mut [u8],
    start: usize,
    name_start: &mut usize,
    count: &mut u32,
) -> Result<bool, W29a20Throw> {
    let at = |b: &[u8], i: usize| b.get(i).copied().unwrap_or(0);
    let mut p = start;
    while w29a20_is_space(at(buf, p)) {
        p += 1;
    }
    *name_start = p;
    let mut end = p;
    loop {
        let c = at(buf, end);
        if c == 0 || c == b':' || w29a20_is_space(c) {
            break;
        }
        end += 1;
    }
    let mut q = end;
    while w29a20_is_space(at(buf, q)) {
        q += 1;
    }
    let c = at(buf, q);
    if c == 0 {
        return Ok(true);
    }
    if c != b':' {
        return Ok(false);
    }
    buf[end] = 0;
    q += 1;
    while w29a20_is_space(at(buf, q)) {
        q += 1;
    }
    if at(buf, q) != b'{' {
        *count = 1;
        return Err(W29a20Throw);
    }
    q += 1;
    while w29a20_is_space(at(buf, q)) {
        q += 1;
    }
    let mut depth = 1i32;
    *count = 1;
    if at(buf, q) != 0 {
        loop {
            let c = at(buf, q);
            if !w29a20_is_space(c) {
                if depth == 0 {
                    return Ok(false);
                }
                match c {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    b',' if depth == 1 => *count += 1,
                    _ => {}
                }
            }
            q += 1;
            if at(buf, q) == 0 {
                break;
            }
        }
        // Balanced list: retail allocates `count * 4` bytes here (Ghidra
        // marks the allocator noreturn; the result is not visible).
    }
    Ok(false)
}

/// Small-buffer pair of sequences: up to 8 dwords inline (`n` <= 8) else
/// heap; up to 0x40 bytes inline (`m` <= 0x40) else heap.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W29a20Seq {
    pub inline_a: [u32; 8],
    pub heap_a: Option<Vec<u32>>,
    pub n: u16,
    pub inline_b: [u32; 16],
    pub heap_b: Option<Vec<u32>>,
    pub m: u16,
}

/// `0x00aeffa0`: releases both heap buffers and zeroes both counts.
pub fn w29a20_seq_reset(s: &mut W29a20Seq) {
    s.heap_a = None;
    s.n = 0;
    s.heap_b = None;
    s.m = 0;
}

/// `0x00aeec40`: move-assign; inline contents are copied when the source
/// count fits (n < 9, m < 0x41), otherwise the heap buffer is stolen.
pub fn w29a20_seq_move_assign(dst: &mut W29a20Seq, src: &mut W29a20Seq) {
    dst.n = src.n;
    if src.n < 9 {
        dst.inline_a = src.inline_a;
        dst.heap_a = None;
    } else {
        dst.heap_a = src.heap_a.take();
    }
    dst.m = src.m;
    if src.m < 0x41 {
        dst.inline_b = src.inline_b;
        dst.heap_b = None;
    } else {
        dst.heap_b = src.heap_b.take();
    }
}

/// `0x00aeed10`: copy-assign (deep-copies heap buffers, then counts).
pub fn w29a20_seq_copy_assign(dst: &mut W29a20Seq, src: &W29a20Seq) {
    dst.inline_a = src.inline_a;
    dst.heap_a = src.heap_a.clone();
    dst.inline_b = src.inline_b;
    dst.heap_b = src.heap_b.clone();
    dst.n = src.n;
    dst.m = src.m;
}

/// `0x00aeec00`: stores length and flag (offsets 0x80/0x84).
pub fn w29a20_set_len_flag(rec: &mut (u32, u8), len: u32, flag: u8) {
    *rec = (len, flag);
}

/// `0x00aeebc0`: like `0x00aeec00` with the length taken from a C string.
pub fn w29a20_set_len_flag_cstr(rec: &mut (u32, u8), s: &[u8], flag: u8) {
    let len = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    *rec = (len as u32, flag);
}

#[cfg(test)]
mod tests_wave29_agent20 {
    use super::*;

    fn parse(s: &str) -> (Result<bool, W29a20Throw>, u32, String) {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        let (mut ns, mut c) = (0, 0);
        let r = w29a20_parse_header(&mut b, 0, &mut ns, &mut c);
        let end = b[ns..].iter().position(|&x| x == 0).unwrap() + ns;
        (r, c, String::from_utf8_lossy(&b[ns..end]).into())
    }

    #[test]
    fn header() {
        assert_eq!(parse("  abc  "), (Ok(true), 0, "abc  ".into()));
        assert_eq!(parse("abc : { 1, {2,3}, 4 }"), (Ok(false), 3, "abc".into()));
        assert_eq!(parse("abc : x").0, Err(W29a20Throw));
    }

    #[test]
    fn seq() {
        let mut a = W29a20Seq {
            n: 9,
            heap_a: Some(vec![1; 9]),
            m: 3,
            inline_b: [7; 16],
            ..Default::default()
        };
        let mut b = W29a20Seq::default();
        w29a20_seq_move_assign(&mut b, &mut a);
        assert!(a.heap_a.is_none() && b.heap_a.is_some() && b.inline_b[0] == 7);
        let mut c = W29a20Seq::default();
        w29a20_seq_copy_assign(&mut c, &b);
        assert_eq!(c, b);
        w29a20_seq_reset(&mut c);
        assert_eq!((c.n, c.m, c.heap_a.is_none()), (0, 0, true));
    }

    #[test]
    fn len_flag() {
        let mut r = (0, 0);
        w29a20_set_len_flag(&mut r, 5, 1);
        assert_eq!(r, (5, 1));
        w29a20_set_len_flag_cstr(&mut r, b"abc\0def", 2);
        assert_eq!(r, (3, 2));
    }
}
