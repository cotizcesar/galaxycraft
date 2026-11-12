#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent15.rs
// ===========================================================================
// Wave 28 agent 15: tinyxml2 pool/printer/StrPair helpers and a stream buffer.

/// Stream buffer cursor (fields at `+0x2c`, `+0x34`, `+0x38`, `+0x3c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W28a15Buf {
    pub consumed: i32,
    pub pos: i32,
    pub end: i32,
    pub cap_end: i32,
}

/// `0x00a79420`: bytes copied out of the buffer: `min(want, end - pos)`.
pub fn w28a15_readable_len(b: &W28a15Buf, want: u32) -> u32 {
    let avail = b.end.wrapping_sub(b.pos) as u32;
    want.min(avail)
}

/// `0x00a795e0`: compacts the buffer. Adds `end - pos` to `consumed` and
/// returns `(src_offset = end, dst_offset = pos, len = cap_end - end)` for
/// the memmove the original performs.
pub fn w28a15_compact(b: &mut W28a15Buf) -> (i32, i32, i32) {
    let end = b.end;
    b.consumed = b.consumed.wrapping_add(end.wrapping_sub(b.pos));
    (end, b.pos, b.cap_end.wrapping_sub(end))
}

/// `0x00a7a240`: true when the heap pointer differs from the inline buffer
/// (i.e. the original would free it).
pub fn w28a15_needs_free_buffer(ptr: u32, inline_ptr: u32) -> bool {
    ptr != inline_ptr
}

/// tinyxml2 StrPair `{flags, start, end}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W28a15StrPair {
    pub flags: u32,
    pub start: u32,
    pub end: u32,
}

/// `0x00a7a270`: StrPair reset. Returns the pointer to free when the
/// `0x200` (needs-delete) flag is set, then zeroes all three fields.
pub fn w28a15_strpair_reset(p: &mut W28a15StrPair) -> Option<u32> {
    let free = if p.flags & 0x200 != 0 {
        Some(p.start)
    } else {
        None
    };
    *p = W28a15StrPair::default();
    free
}

/// `0x00a7a5c0`: XMLAttribute destructor body: resets the value StrPair
/// (`+0x10`) then the name StrPair (`+0x04`); returns pointers to free in
/// that order.
pub fn w28a15_attribute_release(
    name: &mut W28a15StrPair,
    value: &mut W28a15StrPair,
) -> [Option<u32>; 2] {
    let v = w28a15_strpair_reset(value);
    let n = w28a15_strpair_reset(name);
    [v, n]
}

/// XMLPrinter state set up by the constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W28a15Printer {
    pub first_element: bool,
    pub depth: i32,
    pub text_depth: i32,
    pub fp: u32,
    pub compact: bool,
    pub elem_just_opened: bool,
    pub process_entities: bool,
    pub entity_flag: [bool; 64],
    pub restricted_entity_flag: [bool; 64],
    pub buffer_cap: i32,
    pub buffer_len: i32,
    pub stack_cap: i32,
    pub stack_len: i32,
    pub mem_size: u32,
}

/// `0x00a7a0d0`: XMLPrinter constructor `(fp, compact, depth)`.
/// Pushes one NUL into the buffer (len 1), with entity flag tables zeroed
/// then `\n`,`\r`... entries as written (`+0x70` = 1, `+0x74..` = 0x101).
pub fn w28a15_printer_new(fp: u32, compact: bool, depth: u32) -> W28a15Printer {
    W28a15Printer {
        first_element: true,
        depth: depth as i32,
        text_depth: -1,
        fp,
        compact,
        elem_just_opened: false,
        process_entities: true,
        entity_flag: [false; 64],
        restricted_entity_flag: [false; 64],
        buffer_cap: 0x14,
        buffer_len: 1,
        stack_cap: 10,
        stack_len: 0,
        mem_size: 0,
    }
}

/// MemPool allocation state (`+0x38` free head, `+0x3c` current, `+0x40`
/// nallocs, `+0x44` max, `+0x48` untracked).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W28a15Pool {
    pub free_head: Option<u32>,
    pub current: i32,
    pub nallocs: i32,
    pub max: i32,
    pub untracked: i32,
}

fn w28a15_pool_alloc(p: &mut W28a15Pool, next_of: impl FnOnce(u32) -> Option<u32>) -> Option<u32> {
    let item = p.free_head?;
    p.free_head = next_of(item);
    p.current += 1;
    if p.max < p.current {
        p.max = p.current;
    }
    p.nallocs += 1;
    p.untracked += 1;
    Some(item)
}

/// `0x00a7a9a0`: MemPoolT alloc (item size 0xff0 block). `None` when the
/// free list is empty (original asserts after allocating a new block).
pub fn w28a15_pool_alloc_a(
    p: &mut W28a15Pool,
    next_of: impl FnOnce(u32) -> Option<u32>,
) -> Option<u32> {
    w28a15_pool_alloc(p, next_of)
}

/// `0x00a7ab70`: same allocation step for the 0xfd8 block pool.
pub fn w28a15_pool_alloc_b(
    p: &mut W28a15Pool,
    next_of: impl FnOnce(u32) -> Option<u32>,
) -> Option<u32> {
    w28a15_pool_alloc(p, next_of)
}

#[cfg(test)]
mod tests_wave28_agent15 {
    use super::*;

    #[test]
    fn buffer_helpers() {
        let mut b = W28a15Buf {
            consumed: 5,
            pos: 10,
            end: 14,
            cap_end: 20,
        };
        assert_eq!(w28a15_readable_len(&b, 100), 4);
        assert_eq!(w28a15_readable_len(&b, 2), 2);
        assert_eq!(w28a15_compact(&mut b), (14, 10, 6));
        assert_eq!(b.consumed, 9);
        assert!(w28a15_needs_free_buffer(1, 2));
        assert!(!w28a15_needs_free_buffer(2, 2));
    }

    #[test]
    fn strpair() {
        let mut p = W28a15StrPair {
            flags: 0x200,
            start: 7,
            end: 9,
        };
        assert_eq!(w28a15_strpair_reset(&mut p), Some(7));
        assert_eq!(p, W28a15StrPair::default());
        let mut q = W28a15StrPair {
            flags: 1,
            start: 3,
            end: 4,
        };
        assert_eq!(w28a15_strpair_reset(&mut q), None);
        let mut n = W28a15StrPair {
            flags: 0x200,
            start: 1,
            end: 0,
        };
        let mut v = W28a15StrPair {
            flags: 0x200,
            start: 2,
            end: 0,
        };
        assert_eq!(w28a15_attribute_release(&mut n, &mut v), [Some(2), Some(1)]);
    }

    #[test]
    fn printer() {
        let p = w28a15_printer_new(5, true, 3);
        assert_eq!(p.text_depth, -1);
        assert_eq!((p.buffer_cap, p.buffer_len, p.stack_cap), (0x14, 1, 10));
        assert!(p.first_element && p.process_entities && p.compact);
    }

    #[test]
    fn pool() {
        let mut p = W28a15Pool {
            free_head: Some(1),
            ..Default::default()
        };
        assert_eq!(w28a15_pool_alloc_a(&mut p, |_| None), Some(1));
        assert_eq!((p.current, p.max, p.nallocs, p.untracked), (1, 1, 1, 1));
        assert_eq!(w28a15_pool_alloc_b(&mut p, |_| None), None);
    }
}
