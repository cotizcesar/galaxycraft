#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent44.rs
// ===========================================================================
// Wave 31 agent 44: one real routine; the rest of the queue is SEH/no-return glue.

/// Marker value stored in each filled slot (address of the static default
/// record in the retail image, `DAT_0087fb80`).
pub const W31A44_SLOT_MARKER: u32 = 0x0087_fb80;

/// `0x00881120`: fills slots of a pointer table.  `len_field` is the packed
/// count word at `+0x14c` (low 30 bits are the count); `base` is the pointer
/// at `+0x148`.  If `len_field * 4 + base` wraps in 32 bits the count is zero.
/// For every iteration `next_index` (the callee at 0x87_b500) yields a slot
/// index; `slots[idx]` (table at `+0x44`) is set to the marker and
/// `high_water` (`+0x154`, signed) becomes `max(high_water, idx - 1)`.
/// Returns `tail() & 0xffffff00` (the callee at 0x87_f850).
pub fn w31a44_fill_slots(
    len_field: u32,
    base: u32,
    slots: &mut [u32],
    high_water: &mut i32,
    mut next_index: impl FnMut() -> i32,
    tail: impl FnOnce() -> u32,
) -> u32 {
    let mut count = len_field & 0x3fff_ffff;
    if len_field.wrapping_mul(4).checked_add(base).is_none() {
        count = 0;
    }
    for _ in 0..count {
        let idx = next_index();
        if idx >= 0 && (idx as usize) < slots.len() {
            slots[idx as usize] = W31A44_SLOT_MARKER;
        }
        if *high_water <= idx.wrapping_sub(1) {
            *high_water = idx.wrapping_sub(1);
        }
    }
    tail() & 0xffff_ff00
}

#[cfg(test)]
mod tests_wave31_agent44 {
    use super::*;

    #[test]
    fn fills_and_tracks_high_water() {
        let mut slots = [0u32; 8];
        let mut hw = 0;
        let mut seq = [3, 5].into_iter();
        let r = w31a44_fill_slots(
            2,
            0x1000,
            &mut slots,
            &mut hw,
            || seq.next().unwrap(),
            || 0x1ff,
        );
        assert_eq!(r, 0x100);
        assert_eq!(slots[3], W31A44_SLOT_MARKER);
        assert_eq!(slots[5], W31A44_SLOT_MARKER);
        assert_eq!(hw, 4);
    }

    #[test]
    fn overflow_means_zero_count() {
        let mut slots = [0u32; 2];
        let mut hw = 0;
        let r = w31a44_fill_slots(0x3fff_ffff, 0xffff_fff0, &mut slots, &mut hw, || 1, || 0xab);
        assert_eq!(r, 0);
        assert_eq!(slots, [0, 0]);
    }
}
