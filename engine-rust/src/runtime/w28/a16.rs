#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent16.rs
// ===========================================================================
// Wave 28 agent 16: TinyXML-style pool / string-scan helpers.

/// Pool header fields of the 0xA7xxxx allocator family (offsets 0x34..0x48).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct W28a16Pool {
    pub block_count: i32,
    pub free_head: Option<usize>,
    pub in_use: i32,
    pub allocs_total: i32,
    pub peak: i32,
    pub allocs_counter: i32,
}

/// `0x00a7ad40`: pops the free-list head (`+0x38`); `next_of` reads the link
/// stored in the node.  Empty list is a non-returning error: `None`.
/// Updates in-use (`+0x3c`), peak (`+0x44`) and the counters at `+0x40`/`+0x48`.
pub fn w28a16_pool_alloc_a(
    p: &mut W28a16Pool,
    next_of: impl Fn(usize) -> Option<usize>,
) -> Option<usize> {
    w28a16_pool_pop(p, next_of)
}

/// `0x00a7af00`: same pop as the sibling pool allocator (different node size
/// in the error path only).
pub fn w28a16_pool_alloc_b(
    p: &mut W28a16Pool,
    next_of: impl Fn(usize) -> Option<usize>,
) -> Option<usize> {
    w28a16_pool_pop(p, next_of)
}

fn w28a16_pool_pop(p: &mut W28a16Pool, next_of: impl Fn(usize) -> Option<usize>) -> Option<usize> {
    let head = p.free_head?;
    p.free_head = next_of(head);
    p.in_use += 1;
    if p.peak < p.in_use {
        p.peak = p.in_use;
    }
    p.allocs_total += 1;
    p.allocs_counter += 1;
    Some(head)
}

fn w28a16_pool_release(p: &mut W28a16Pool) -> Option<usize> {
    if p.block_count != 0 {
        p.block_count -= 1;
        return Some(p.block_count as usize);
    }
    p.free_head = None;
    p.in_use = 0;
    p.allocs_total = 0;
    p.peak = 0;
    p.allocs_counter = 0;
    None
}

/// `0x00a7b130`: pool teardown step.  While blocks remain it decrements the
/// block count and returns the block index to free; once empty it zeroes the
/// stats and returns `None`.
pub fn w28a16_pool_release_a(p: &mut W28a16Pool) -> Option<usize> {
    w28a16_pool_release(p)
}

/// `0x00a7b190`: same teardown step for the second pool type.
pub fn w28a16_pool_release_b(p: &mut W28a16Pool) -> Option<usize> {
    w28a16_pool_release(p)
}

/// `0x00a7b1f0`: same teardown step for the third pool type.
pub fn w28a16_pool_release_c(p: &mut W28a16Pool) -> Option<usize> {
    w28a16_pool_release(p)
}

/// `0x00a7cd30`: pushes onto a capped pointer vector (`len` at 0x30, cap at
/// 0x2c); returns `false` where retail grows (non-returning realloc path).
pub fn w28a16_vec_push(buf: &mut [u32], len: &mut usize, value: u32) -> bool {
    if buf.len() < *len + 1 {
        return false;
    }
    buf[*len] = value;
    *len += 1;
    true
}

/// `0x00a7cdb0`: reserves `n` bytes in the text buffer; returns the offset of
/// the reserved region (old length) or `None` when capacity is exceeded.
pub fn w28a16_buf_reserve(cap: usize, len: &mut usize, n: usize) -> Option<usize> {
    let old = *len;
    if cap < old + n {
        return None;
    }
    *len = old + n;
    Some(old)
}

/// `0x00a7c970`: scans an XML name.  First byte: ASCII letter, ':' or '_'
/// (bytes >= 0x80 accepted); then letters, digits, ':', '_', '.', '-' or high
/// bytes.  Returns the end offset, `None` if no name starts at 0.
pub fn w28a16_scan_name(s: &[u8]) -> Option<usize> {
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let first = at(0);
    if first == 0
        || (first < 0x80 && !first.is_ascii_alphabetic() && first != b':' && first != b'_')
    {
        return None;
    }
    let mut i = 1;
    loop {
        let c = at(i);
        let ok = c != 0
            && (c >= 0x80
                || c.is_ascii_alphabetic()
                || c.is_ascii_digit()
                || c == b':'
                || c == b'_'
                || c == b'.'
                || c == b'-');
        if !ok {
            return Some(i);
        }
        i += 1;
    }
}

/// `0x00a7ca40`: finds `needle` in NUL-terminated `text`, counting `\n`
/// bytes skipped into `lines`.  Returns `(match_start, end_after_match)`.
pub fn w28a16_find_counting_lines(
    text: &[u8],
    needle: &[u8],
    lines: &mut i32,
) -> Option<(usize, usize)> {
    let first = needle.first().copied().unwrap_or(0);
    let mut i = 0;
    while i < text.len() && text[i] != 0 {
        if text[i] == first && text[i..].starts_with(needle) {
            return Some((i, i + needle.len()));
        }
        if text[i] == b'\n' {
            *lines += 1;
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests_wave28_agent16 {
    use super::*;

    #[test]
    fn pool_alloc_and_release() {
        let mut p = W28a16Pool {
            block_count: 1,
            free_head: Some(3),
            ..Default::default()
        };
        let n = w28a16_pool_alloc_a(&mut p, |i| if i == 3 { Some(5) } else { None });
        assert_eq!(n, Some(3));
        assert_eq!(
            (p.in_use, p.peak, p.allocs_total, p.allocs_counter),
            (1, 1, 1, 1)
        );
        assert_eq!(p.free_head, Some(5));
        p.free_head = None;
        assert_eq!(w28a16_pool_alloc_b(&mut p, |_| None), None);
        assert_eq!(w28a16_pool_release_a(&mut p), Some(0));
        assert_eq!(w28a16_pool_release_b(&mut p), None);
        assert_eq!(p, W28a16Pool::default());
        assert_eq!(w28a16_pool_release_c(&mut p), None);
    }

    #[test]
    fn vec_and_buf() {
        let mut b = [0u32; 2];
        let mut l = 0;
        assert!(w28a16_vec_push(&mut b, &mut l, 7));
        assert!(w28a16_vec_push(&mut b, &mut l, 8));
        assert!(!w28a16_vec_push(&mut b, &mut l, 9));
        assert_eq!(b, [7, 8]);
        let mut len = 2;
        assert_eq!(w28a16_buf_reserve(4, &mut len, 2), Some(2));
        assert_eq!(w28a16_buf_reserve(4, &mut len, 1), None);
        assert_eq!(len, 4);
    }

    #[test]
    fn name_scan() {
        assert_eq!(w28a16_scan_name(b"a:b-c.d=1\0"), Some(7));
        assert_eq!(w28a16_scan_name(b"1a"), None);
        assert_eq!(w28a16_scan_name(b""), None);
        assert_eq!(w28a16_scan_name(b"_x y"), Some(2));
    }

    #[test]
    fn find_lines() {
        let mut n = 0;
        assert_eq!(
            w28a16_find_counting_lines(b"a\nb\n-->x\0", b"-->", &mut n),
            Some((4, 7))
        );
        assert_eq!(n, 2);
        assert_eq!(w28a16_find_counting_lines(b"abc\0", b"z", &mut n), None);
    }
}
