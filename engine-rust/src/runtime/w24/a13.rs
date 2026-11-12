#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent13.rs
// ===========================================================================
// Wave 24 agent 13: small buffer/record helpers (runtime.rs target).

/// `0x007415b0`: resets the global record counter to zero and clears the
/// first `0x400` bytes of the record buffer.
pub fn w24a13_reset_record_buffer(counter: &mut u32, buf: &mut [u8]) {
    *counter = 0;
    let n = buf.len().min(0x400);
    buf[..n].fill(0);
}

/// `0x00741610`: capacity growth for a string-like header whose word at +8
/// packs a flag in bit 31 and the capacity in bits 0..30.  When the capacity
/// is below `wanted`, the new capacity is `max(cap + cap/2, wanted)` (the
/// decompiled `(w >> 1 & 0x3fffffff) + cap`) and the new word keeps bit 31.
/// Returns the updated word when growth is required.
pub fn w24a13_grow_capacity_word(word: u32, wanted: u32) -> Option<u32> {
    let cap = word & 0x7fff_ffff;
    if cap >= wanted {
        return None;
    }
    let mut new_cap = ((word >> 1) & 0x3fff_ffff).wrapping_add(cap);
    if new_cap < wanted {
        new_cap = wanted;
    }
    Some((word & 0x8000_0000) | (new_cap & 0x7fff_ffff))
}

/// Layout of the record writer initialised by `0x007416f0` (dword fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W24a13Writer {
    pub active: u32,
    pub buf_handle: u32,
    pub len: u32,
    pub capacity: u32,
    pub mark: u32,
    pub last_tag: u32,
    pub reserved: u32,
}

/// `0x007416f0`: initialises the writer: capacity 50000, buffer from the
/// allocator `alloc(size)`, length 0, mark 0, tag `0xffffffff`, active = 1.
pub fn w24a13_init_writer(alloc: impl FnOnce(u32) -> u32) -> W24a13Writer {
    W24a13Writer {
        active: 1,
        buf_handle: alloc(50000),
        len: 0,
        capacity: 50000,
        mark: 0,
        last_tag: 0xffff_ffff,
        reserved: 0,
    }
}

/// `0x007419f0`: true when the buffer must grow (`capacity <= len + 0x200`);
/// the new size is `capacity + 10000`.
pub fn w24a13_writer_grow_size(len: u32, capacity: u32) -> Option<u32> {
    if capacity <= len.wrapping_add(0x200) {
        Some(capacity.wrapping_add(10000))
    } else {
        None
    }
}

/// `0x007420b0`: clears the dword at offset 4 of the active record header.
pub fn w24a13_clear_header_flag(header: &mut [u32; 2]) {
    header[1] = 0;
}

#[cfg(test)]
mod tests_wave24_agent13 {
    use super::*;

    #[test]
    fn reset_clears() {
        let mut c = 5;
        let mut b = vec![1u8; 0x500];
        w24a13_reset_record_buffer(&mut c, &mut b);
        assert_eq!(c, 0);
        assert!(b[..0x400].iter().all(|&x| x == 0));
        assert_eq!(b[0x400], 1);
    }

    #[test]
    fn grow_word() {
        assert_eq!(w24a13_grow_capacity_word(100, 100), None);
        assert_eq!(w24a13_grow_capacity_word(100, 120), Some(150));
        assert_eq!(
            w24a13_grow_capacity_word(0x8000_0000 | 100, 500),
            Some(0x8000_0000 | 500)
        );
    }

    #[test]
    fn init_and_grow() {
        let w = w24a13_init_writer(|n| n + 1);
        assert_eq!(
            (w.active, w.buf_handle, w.capacity, w.last_tag),
            (1, 50001, 50000, u32::MAX)
        );
        assert_eq!(w24a13_writer_grow_size(0, 0x200), Some(10512));
        assert_eq!(w24a13_writer_grow_size(0, 0x201), None);
        let mut h = [1, 2];
        w24a13_clear_header_flag(&mut h);
        assert_eq!(h, [1, 0]);
    }
}
