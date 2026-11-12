#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent06.rs
// ===========================================================================
// Wave 24 agent 06 (client.rs target).  Skipped (SEH/noreturn glue, no
// recoverable logic): FUN_007a7160, FUN_007a7850, FUN_007ae420, FUN_007ae660,
// FUN_007ae8a0, FUN_007af190, FUN_007b0b40, FUN_007b5e30, FUN_007c2be0,
// FUN_007c88d0, FUN_007c97d0, FUN_007cb760, FUN_007cb950, FUN_007cc3b0,
// FUN_007cea50, FUN_007d0c50, FUN_007d67c0.

/// Result of `0x007a8770`: the packed cursor cell (x, y) and the flag byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W24a06CellSelect {
    pub x: i16,
    pub y: i16,
    pub flag: bool,
}

/// `0x007a8770`: cell-selection handler.  `blocked(x, y)` models the first
/// callee (non-zero means ignore), `accepts(x, y)` the vtable slot 0x14 call
/// (non-zero means ignore).  When both pass, the cell is stored (x in the low
/// half, y in the high half) and the flag is set only for kind 6.
pub fn w24a06_cell_select(
    x: i16,
    y: i16,
    kind: i16,
    blocked: impl Fn(i32, i32) -> bool,
    accepts: impl Fn(i32, i32) -> bool,
) -> Option<W24a06CellSelect> {
    if blocked(x as i32, y as i32) || accepts(x as i32, y as i32) {
        return None;
    }
    Some(W24a06CellSelect {
        x,
        y,
        flag: kind == 6,
    })
}

/// `0x007a9000`: resets the 8 list sentinels (12-byte nodes: next, prev,
/// count) to self-linked/empty.  Returns the index of the first node whose old
/// prev link was not itself (the original then hits a non-returning path), or
/// `None` if all were fine.  Node links are node indices.
pub fn w24a06_reset_sentinels(nodes: &mut [[u32; 3]; 8]) -> Option<usize> {
    for i in 0..8 {
        let old_prev = nodes[i][1];
        nodes[i] = [i as u32, i as u32, 0];
        if old_prev != i as u32 {
            return Some(i);
        }
    }
    None
}

/// Layout produced by `0x007ca4b0` (16-byte small-string header).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W24a06StrHeader {
    pub len: u32,
    pub heap_alloc: Option<u32>,
    pub tag: u16,
    pub remaining_byte: u8,
}

/// `0x007ca4b0`: initialises a string header for `len` bytes.  Under 14 the
/// text is inline (tag 0x1c05, byte at +0xd = 13 - len); otherwise tag 0xc05,
/// length stored and `len + 1` bytes are allocated.
pub fn w24a06_str_header_init(len: u32) -> W24a06StrHeader {
    if len < 0xe {
        W24a06StrHeader {
            len: 0,
            heap_alloc: None,
            tag: 0x1c05,
            remaining_byte: (13u32.wrapping_sub(len)) as u8,
        }
    } else {
        W24a06StrHeader {
            len,
            heap_alloc: Some(len + 1),
            tag: 0xc05,
            remaining_byte: 0,
        }
    }
}

#[cfg(test)]
mod tests_wave24_agent06 {
    use super::*;

    #[test]
    fn cell_select() {
        let r = w24a06_cell_select(3, 4, 6, |_, _| false, |_, _| false).unwrap();
        assert!(r.flag && r.x == 3 && r.y == 4);
        assert!(
            !w24a06_cell_select(1, 2, 5, |_, _| false, |_, _| false)
                .unwrap()
                .flag
        );
        assert!(w24a06_cell_select(1, 2, 5, |_, _| true, |_, _| false).is_none());
        assert!(w24a06_cell_select(1, 2, 5, |_, _| false, |_, _| true).is_none());
    }

    #[test]
    fn sentinels() {
        let mut n = [[0u32; 3]; 8];
        for (i, e) in n.iter_mut().enumerate() {
            *e = [9, i as u32, 5];
        }
        assert_eq!(w24a06_reset_sentinels(&mut n), None);
        assert_eq!(n[7], [7, 7, 0]);
        let mut m = [[0u32; 3]; 8];
        assert_eq!(w24a06_reset_sentinels(&mut m), Some(1));
    }

    #[test]
    fn str_header() {
        let a = w24a06_str_header_init(13);
        assert_eq!((a.tag, a.remaining_byte, a.heap_alloc), (0x1c05, 0, None));
        assert_eq!(w24a06_str_header_init(0).remaining_byte, 13);
        let b = w24a06_str_header_init(14);
        assert_eq!((b.tag, b.len, b.heap_alloc), (0xc05, 14, Some(15)));
    }
}
