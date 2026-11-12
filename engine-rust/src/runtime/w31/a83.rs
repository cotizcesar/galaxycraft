#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent83.rs
// ===========================================================================
// Wave 31 agent 83: container growth helpers (runtime.rs).

/// `0x00b6b1a0`: capacity the container at `ECX` grows to when it needs one
/// more slot: starts at 8 (or the current size field at +4 when non-zero) and
/// doubles until it exceeds `size + 1`.  The caller then reallocates.
pub fn w31a83_next_pow2_capacity(size: u32) -> u32 {
    let mut cap = if size != 0 { size } else { 8 };
    while cap < size.wrapping_add(1) {
        cap = cap.wrapping_mul(2);
    }
    cap
}

/// Result of [`w31a83_deque_push_front_slot`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W31a83DequeSlot {
    pub grow_map: bool,
    pub start: u32,
    pub size: u32,
    pub block: u32,
    pub offset: u32,
}

/// `0x00b6b7e0`: deque (16 u32 per block) push-front slot selection.  Fields
/// are `map_len` (+4), `start` (+8) and `size` (+0xc).  When the start sits
/// on a block boundary and the map is too small, the map must grow first
/// (`grow_map`).  Then `size` increments, `start` decrements, and the slot is
/// block `(map_len-1) & (start>>4)`, element `start & 0xf` (the retail code
/// aborts if that block is unallocated; the caller checks).
pub fn w31a83_deque_push_front_slot(map_len: u32, start: u32, size: u32) -> W31a83DequeSlot {
    let grow_map = (start & 0xf) == 0 && map_len <= size.wrapping_add(0x10) >> 4;
    let start = start.wrapping_sub(1);
    W31a83DequeSlot {
        grow_map,
        start,
        size: size.wrapping_add(1),
        block: map_len.wrapping_sub(1) & (start >> 4),
        offset: start & 0xf,
    }
}

/// `0x00b6d8b0`: capacity growth for appending one 16-byte record.  `cap_field`
/// is the dword at +8 whose low 31 bits are the capacity.  Returns
/// `Some(new_field)` (growth by half, minimum 1; high bit preserved) when
/// `len == capacity`, otherwise `None` (room available).
pub fn w31a83_vec16_grow_field(len: u32, cap_field: u32) -> Option<u32> {
    let cap = cap_field & 0x7fff_ffff;
    if len == cap && cap < cap.wrapping_add(1) {
        let half = (cap_field >> 1) & 0x3fff_ffff;
        let add = if cap.wrapping_add(1) <= cap.wrapping_add(half) {
            half
        } else {
            1
        };
        let new_cap = cap.wrapping_add(add);
        Some(((new_cap ^ cap_field) & 0x7fff_ffff) ^ cap_field)
    } else {
        None
    }
}

/// `0x00b6d8b0`: writes a 16-byte record `[a, b, c0, c1]` (dword order
/// `p0=a, p1=b, p2=c0, p3=c1`) at index `len`.
pub fn w31a83_vec16_store(rec: &mut [u32; 4], a: u32, b: u32, c: [u32; 2]) {
    rec[0] = a;
    rec[1] = b;
    rec[2] = c[0];
    rec[3] = c[1];
}

#[cfg(test)]
mod tests_wave31_agent83 {
    use super::*;

    #[test]
    fn capacity() {
        assert_eq!(w31a83_next_pow2_capacity(0), 8);
        assert_eq!(w31a83_next_pow2_capacity(5), 10);
        assert_eq!(w31a83_next_pow2_capacity(8), 8 * 2);
    }

    #[test]
    fn deque() {
        let s = w31a83_deque_push_front_slot(4, 0x10, 0);
        assert!(!s.grow_map);
        assert_eq!((s.start, s.size, s.block, s.offset), (0xf, 1, 0, 0xf));
        assert!(w31a83_deque_push_front_slot(1, 0, 0).grow_map);
    }

    #[test]
    fn vec16() {
        assert_eq!(w31a83_vec16_grow_field(3, 4), None);
        assert_eq!(w31a83_vec16_grow_field(4, 4), Some(6));
        assert_eq!(w31a83_vec16_grow_field(8, 0x8000_0008), Some(0x8000_000c));
        let mut r = [0; 4];
        w31a83_vec16_store(&mut r, 1, 2, [3, 4]);
        assert_eq!(r, [1, 2, 3, 4]);
    }
}
