#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent08.rs
// ===========================================================================
// Wave 26 translation batch 08.

/// `0x0091f090`: resets a circular list header `[next, prev, count]` so both
/// links point at the header itself and the count is zero.  Returns the old
/// `prev` link had differed from the header (the original then frees the
/// nodes), i.e. `true` when there were nodes to release.
pub fn w26a08_list_header_reset(hdr: &mut [u32; 3], self_addr: u32) -> bool {
    let old = hdr[1];
    hdr[1] = self_addr;
    hdr[0] = self_addr;
    hdr[2] = 0;
    old != self_addr
}

/// `0x00920840`: the five `(id: u16, value: u32)` pairs passed to the
/// option-table installer (count 5).
pub fn w26a08_option_pairs() -> [(u16, u32); 5] {
    [(1, 0), (2, 2), (3, 3), (4, 2), (5, 0)]
}

/// Deque header used by `0x00921ec0`: block map length, start offset, size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W26a08Deque {
    pub map_size: u32,
    pub off: u32,
    pub size: u32,
}

/// `0x00921ec0`: deque push_back slot lookup (8 elements per block, element
/// stride `0x88`).  Returns `(block_index, byte_offset_in_block)` and bumps
/// `size`; `None` when the map must grow first (or the block is missing,
/// as `block_present` reports).
pub fn w26a08_deque_push_slot(
    d: &mut W26a08Deque,
    block_present: impl Fn(u32) -> bool,
) -> Option<(u32, u32)> {
    if (d.off.wrapping_add(d.size) & 7) == 0 && d.map_size <= (d.size.wrapping_add(8) >> 3) {
        return None;
    }
    let pos = d.off.wrapping_add(d.size);
    d.size = d.size.wrapping_add(1);
    let blk = d.map_size.wrapping_sub(1) & (pos >> 3);
    if !block_present(blk) {
        return None;
    }
    Some((blk, (pos & 7) * 0x88))
}

/// `0x00923dd0`: `std::bad_alloc` constructor; returns the stored message.
pub fn w26a08_bad_alloc_message() -> &'static str {
    "bad allocation"
}

#[cfg(test)]
mod tests_wave26_agent08 {
    use super::*;

    #[test]
    fn list_reset() {
        let mut h = [1, 2, 3];
        assert!(w26a08_list_header_reset(&mut h, 9));
        assert_eq!(h, [9, 9, 0]);
        assert!(!w26a08_list_header_reset(&mut h, 9));
    }

    #[test]
    fn pairs_and_msg() {
        assert_eq!(w26a08_option_pairs()[3], (4, 2));
        assert_eq!(w26a08_bad_alloc_message(), "bad allocation");
    }

    #[test]
    fn deque_slot() {
        let mut d = W26a08Deque {
            map_size: 4,
            off: 0,
            size: 3,
        };
        assert_eq!(
            w26a08_deque_push_slot(&mut d, |_| true),
            Some((0, 3 * 0x88))
        );
        assert_eq!(d.size, 4);
        let mut d = W26a08Deque {
            map_size: 1,
            off: 0,
            size: 8,
        };
        assert_eq!(w26a08_deque_push_slot(&mut d, |_| true), None);
        let mut d = W26a08Deque {
            map_size: 4,
            off: 0,
            size: 3,
        };
        assert_eq!(w26a08_deque_push_slot(&mut d, |_| false), None);
    }
}
