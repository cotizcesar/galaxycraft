#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent20.rs
// ===========================================================================
// Wave 24 agent 20: slot-state helpers and dialog tile selection.

/// `0x00759c10`: marks slot `slot` as 1 (pending) unless it is already 2.
pub fn w24a20_slot_mark_pending(slots: &mut [u32], slot: u8) {
    let s = &mut slots[slot as usize];
    if *s != 2 {
        *s = 1;
    }
}

/// `0x00759c30`: marks slot `slot` as 2 (active).
pub fn w24a20_slot_mark_active(slots: &mut [u32], slot: u8) {
    slots[slot as usize] = 2;
}

/// `0x00759c50`: clears slot `slot` to 0.
pub fn w24a20_slot_clear(slots: &mut [u32], slot: u8) {
    slots[slot as usize] = 0;
}

/// `0x00759c70`: sweeps the 8 slots; active (2) slots clear their bit in
/// the three caller-supplied masks, pending (1) slots reset to 0.
pub fn w24a20_slot_sweep(
    slots: &mut [u32; 8],
    mask_a: &mut u32,
    mask_b: &mut u32,
    mask_c: &mut u32,
) {
    for i in 0..8u32 {
        match slots[i as usize] {
            2 => {
                let m = !(1u32 << i);
                *mask_a &= m;
                *mask_b &= m;
                *mask_c &= m;
            }
            1 => slots[i as usize] = 0,
            _ => {}
        }
    }
}

/// `0x0075a090`: zeroes the 8-dword block.
pub fn w24a20_zero8(block: &mut [u32; 8]) {
    *block = [0; 8];
}

/// `0x0075b560`: dialog tile selector; returns `(clamped_param1, table_index)`
/// where the index is `(param2 != 0 ? 3 : 0) + clamped` (clamped = param1 if < 3 else 0).
pub fn w24a20_tile_select_a(param1: i32, param2: i32) -> (i32, usize) {
    let c = if param1 < 3 { param1 } else { 0 };
    let base = if param2 != 0 { 3 } else { 0 };
    (c, (base + c) as usize)
}

/// `0x0075b9e0`: same selection as the sibling at 0x0075b560.
pub fn w24a20_tile_select_b(param1: i32, param2: i32) -> (i32, usize) {
    w24a20_tile_select_a(param1, param2)
}

#[cfg(test)]
mod tests_wave24_agent20 {
    use super::*;

    #[test]
    fn slots() {
        let mut s = [0u32; 8];
        w24a20_slot_mark_pending(&mut s, 1);
        assert_eq!(s[1], 1);
        w24a20_slot_mark_active(&mut s, 1);
        w24a20_slot_mark_pending(&mut s, 1);
        assert_eq!(s[1], 2);
        w24a20_slot_mark_active(&mut s, 3);
        w24a20_slot_mark_pending(&mut s, 5);
        let (mut a, mut b, mut c) = (0xff, 0xf0, 0xffff_ffff);
        w24a20_slot_sweep(&mut s, &mut a, &mut b, &mut c);
        assert_eq!((a, b, c), (0xf5, 0xf0, !(1 << 1 | 1 << 3)));
        assert_eq!(s[5], 0);
        assert_eq!(s[3], 2);
        w24a20_slot_clear(&mut s, 3);
        assert_eq!(s[3], 0);
    }

    #[test]
    fn zero_and_tiles() {
        let mut b = [7u32; 8];
        w24a20_zero8(&mut b);
        assert_eq!(b, [0; 8]);
        assert_eq!(w24a20_tile_select_a(2, 1), (2, 5));
        assert_eq!(w24a20_tile_select_a(3, 1), (0, 3));
        assert_eq!(w24a20_tile_select_b(1, 0), (1, 1));
    }
}
