#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent51.rs
// ===========================================================================
// Wave 25 agent 51: small string/vector/hash helpers (runtime.rs target).

/// `0x008ae770` and `0x008b4450`: length of a string whose storage is the
/// heap pointer (`heap`) unless flag bit `0x1000` at offset `0xe` is set, in
/// which case the bytes are inline.  Counts bytes up to the first NUL.
pub fn w25a51_string_len(flags: u16, inline: &[u8], heap: &[u8]) -> usize {
    let s = if flags & 0x1000 == 0 { heap } else { inline };
    s.iter().position(|&b| b == 0).unwrap_or(s.len())
}

/// `0x008af560`: capacity/size selector: heap field when bit `0x1000` of
/// the flags is clear, otherwise `0xd - tail_byte` (inline remaining).
pub fn w25a51_string_size(flags: u16, heap_field: i32, tail_byte: i8) -> i32 {
    if flags & 0x1000 == 0 {
        heap_field
    } else {
        0xd - tail_byte as i32
    }
}

/// `0x008ba070`: byte offset of the bucket slot for `hash` in a table of
/// `count` buckets (power of two): `(hash & (count - 1)) * 4`.
pub fn w25a51_bucket_offset(hash: u32, count: u32) -> u32 {
    (hash & count.wrapping_sub(1)).wrapping_mul(4)
}

/// Outcome of the 0x14-stride push_back planning.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W25a51Push {
    /// Vector is full; carries the new capacity/flag word to store at +0x58.
    Grow(u32),
    /// Room available; element source index (`None` = value is external).
    Append(Option<usize>),
}

/// `0x008ba8b0`: push_back planning for a vector with 0x14-byte elements.
/// `addr_off` is the pushed value's byte offset from the base (it may alias
/// an existing element); `size` is the count, `cap_word` the capacity word
/// whose low 31 bits are the capacity.
pub fn w25a51_push_plan(addr_off: u32, size: u32, cap_word: u32) -> W25a51Push {
    let cap = cap_word & 0x7fff_ffff;
    if size == cap && cap < cap.wrapping_add(1) {
        let extra = (cap_word >> 1) & 0x3fff_ffff;
        let grow = if extra >= 1 { extra } else { 1 };
        let new = (grow.wrapping_add(cap) ^ cap_word) & 0x7fff_ffff ^ cap_word;
        return W25a51Push::Grow(new);
    }
    let idx = addr_off / 0x14;
    W25a51Push::Append(if idx < size { Some(idx as usize) } else { None })
}

/// `0x008bafd0`: true when all three guarded counters (at +0x4c, +0x30,
/// +0x14) are non-negative (otherwise the original aborts).
pub fn w25a51_counters_ok(a: i32, b: i32, c: i32) -> bool {
    a >= 0 && b >= 0 && c >= 0
}

#[cfg(test)]
mod tests_wave25_agent51 {
    use super::*;

    #[test]
    fn string_len_and_size() {
        assert_eq!(w25a51_string_len(0, b"zz\0", b"abc\0"), 3);
        assert_eq!(w25a51_string_len(0x1000, b"zz\0", b"abc\0"), 2);
        assert_eq!(w25a51_string_len(0, b"", b""), 0);
        assert_eq!(w25a51_string_size(0, 7, 3), 7);
        assert_eq!(w25a51_string_size(0x1000, 7, 3), 10);
    }

    #[test]
    fn bucket() {
        assert_eq!(w25a51_bucket_offset(0x13, 8), 12);
    }

    #[test]
    fn push() {
        assert_eq!(w25a51_push_plan(0, 4, 4), W25a51Push::Grow(6));
        assert_eq!(w25a51_push_plan(0x28, 3, 8), W25a51Push::Append(Some(2)));
        assert_eq!(w25a51_push_plan(0x100, 3, 8), W25a51Push::Append(None));
    }

    #[test]
    fn counters() {
        assert!(w25a51_counters_ok(0, 1, 2));
        assert!(!w25a51_counters_ok(0, -1, 2));
    }
}
