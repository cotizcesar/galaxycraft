#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent19.rs
// ===========================================================================
// Wave 24 translation batch 19: tiny container/struct initialisers.

/// `0x00756450`: zeroes eight consecutive dwords (a 32-byte block).
pub fn w24a19_zero_8_dwords(block: &mut [u32; 8]) {
    for d in block.iter_mut() {
        *d = 0;
    }
}

/// `0x007557a0`: initialises a list sentinel node: `prev` and `next` point
/// at the node itself (`self_addr`) and the element count is zero.
/// Returns `[prev, next, size]`.
pub fn w24a19_init_list_sentinel(self_addr: u32) -> [u32; 3] {
    [self_addr, self_addr, 0]
}

/// Layout written by `0x00758200` (24 bytes; dword 0 is a state flag).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W24a19Record {
    pub state: u32,
    pub value: u32,
    pub field2: u32,
    pub field3: u32,
    pub field4: u32,
    pub byte20: u8,
    pub tail21: u32,
}

/// `0x00758200`: resets the record: state = 1, fields 2..4 and the trailing
/// byte/dword (offsets 0x14, 0x15) cleared, and `value` taken from offset
/// 0xc of the object returned by the caller-supplied lookup.
pub fn w24a19_init_record(lookup_word_c: impl FnOnce() -> u32) -> W24a19Record {
    W24a19Record {
        state: 1,
        value: lookup_word_c(),
        ..Default::default()
    }
}

/// `0x00758a40`: releases an owned handle through `free` when non-zero and
/// always clears it to zero.
pub fn w24a19_release_handle(handle: &mut u32, free: impl FnOnce(u32)) {
    if *handle != 0 {
        free(*handle);
    }
    *handle = 0;
}

#[cfg(test)]
mod tests_wave24_agent19 {
    use super::*;

    #[test]
    fn zero_block() {
        let mut b = [7u32; 8];
        w24a19_zero_8_dwords(&mut b);
        assert_eq!(b, [0; 8]);
    }

    #[test]
    fn sentinel() {
        assert_eq!(w24a19_init_list_sentinel(0x1000), [0x1000, 0x1000, 0]);
    }

    #[test]
    fn record() {
        let r = w24a19_init_record(|| 9);
        assert_eq!(
            (r.state, r.value, r.field4, r.byte20, r.tail21),
            (1, 9, 0, 0, 0)
        );
    }

    #[test]
    fn release() {
        let mut h = 5;
        let mut freed = 0;
        w24a19_release_handle(&mut h, |x| freed = x);
        assert_eq!((h, freed), (0, 5));
        w24a19_release_handle(&mut h, |_| panic!());
    }
}
