#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent07.rs
// ===========================================================================
// Wave 32 translation batch 07.
//
// Intrusive tagged-pointer list walks and a 2-byte command builder.

/// Resolves a tagged list link: odd or zero values terminate the list.
fn w32a07_link(v: u32) -> u32 {
    if v & 1 == 0 && v != 0 { v } else { 0 }
}

/// `0x007217d0`: walks the list from `head`; node `+4` is the next link and
/// the key is at `+8`.  Returns 0 when the list ends, 1 as soon as
/// `cmp(node)` reports a zero result (the original breaks on a zero compare).
pub fn w32a07_list_any_match(
    head: u32,
    next: &dyn Fn(u32) -> u32,
    cmp: &dyn Fn(u32) -> i32,
) -> u32 {
    let mut node = w32a07_link(head);
    while node != 0 {
        if cmp(node + 8) == 0 {
            return 1;
        }
        node = w32a07_link(next(node + 4));
    }
    0
}

/// `0x0073b010`: builds the 16-bit command word `(arg << 8) | 0x55`, with
/// bit 15 set when `flag` is non-zero; the caller sends it as 2 bytes.
pub fn w32a07_build_cmd55(arg: u8, flag: u8) -> [u8; 2] {
    let mut w = ((arg as u16) << 8) | 0x55;
    if flag != 0 {
        w |= 0x8000;
    }
    w.to_le_bytes()
}

/// `0x0076d1f0`: walks the list from `head` (node `+4` = next link, key at
/// `+0xc`); returns the first node whose `cmp(key)` is zero, else 0.
pub fn w32a07_list_find(head: u32, next: &dyn Fn(u32) -> u32, cmp: &dyn Fn(u32) -> i32) -> u32 {
    let mut node = w32a07_link(head);
    while node != 0 {
        let n = w32a07_link(next(node + 4));
        if cmp(node + 0xc) == 0 {
            return node;
        }
        node = n;
    }
    0
}

#[cfg(test)]
mod tests_wave32_agent07 {
    use super::*;

    #[test]
    fn cmd55() {
        assert_eq!(w32a07_build_cmd55(0x12, 0), [0x55, 0x12]);
        assert_eq!(w32a07_build_cmd55(0x12, 1), [0x55, 0x92]);
    }

    #[test]
    fn lists() {
        let next = |a: u32| match a {
            0x14 => 0x20,
            0x24 => 1,
            _ => 1,
        };
        assert_eq!(w32a07_list_find(0x10, &next, &|k| (k != 0x2c) as i32), 0x20);
        assert_eq!(w32a07_list_find(0x10, &next, &|_| 1), 0);
        assert_eq!(
            w32a07_list_any_match(0x10, &next, &|k| (k != 0x28) as i32),
            1
        );
        assert_eq!(w32a07_list_any_match(1, &next, &|_| 0), 0);
    }
}
