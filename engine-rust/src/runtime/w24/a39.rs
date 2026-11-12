#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent39.rs
// ===========================================================================
// Wave 24 agent 39: viewport width and deque slot helpers.

/// `0x007b39b0` (core): logical viewport width and its offset from 640.
/// Aspect ratio `w/h` is clamped to `[4/3, 16/9]`; when `fixed_640` is false
/// the width is `(int)(aspect * 480)`, otherwise `0x280`.  Returns
/// `(logical_width, logical_width - 0x280)`, or `None` when `w*h == 0`.
pub fn w24a39_logical_width(w: i32, h: i32, fixed_640: bool) -> Option<(i32, i32)> {
    if w.wrapping_mul(h) == 0 {
        return None;
    }
    let mut a = 1.7777778f32;
    let r = w as f32 / h as f32;
    if r <= a {
        a = r;
    }
    if a <= 1.3333334 {
        a = 1.3333334;
    }
    let lw = if fixed_640 { 0x280 } else { (a * 480.0) as i32 };
    Some((lw, lw - 0x280))
}

/// Outcome of [`w24a39_deque_push_slot`].
#[derive(Debug, PartialEq, Eq)]
pub enum W24a39Slot {
    /// `(block index, element index within the 16-entry block)`.
    Slot(usize, usize),
    /// Map is full: the retail code grows the map (not modelled).
    NeedGrow,
    /// Block pointer is null: the retail code allocates a block.
    NeedBlock,
}

/// `0x007b5f60`: deque back-slot lookup.  `map` holds block ids (0 = null),
/// `offset`/`size` are the deque fields; blocks hold 16 entries.  `size` is
/// incremented on success.
pub fn w24a39_deque_push_slot(map: &[u32], offset: u32, size: &mut u32) -> W24a39Slot {
    let map_size = map.len() as u32;
    if (offset.wrapping_add(*size) & 0xf) == 0 && map_size <= (size.wrapping_add(0x10) >> 4) {
        return W24a39Slot::NeedGrow;
    }
    let idx = offset.wrapping_add(*size);
    *size += 1;
    let block = (map_size.wrapping_sub(1) & (idx >> 4)) as usize;
    if map[block] == 0 {
        return W24a39Slot::NeedBlock;
    }
    W24a39Slot::Slot(block, (idx & 0xf) as usize)
}

/// `0x007b6630`: true when the state field is 1 or 4 (the retail code then
/// calls the state handler).
pub fn w24a39_state_is_active(state: i32) -> bool {
    state == 1 || state == 4
}

#[cfg(test)]
mod tests_wave24_agent39 {
    use super::*;

    #[test]
    fn logical_width() {
        assert_eq!(w24a39_logical_width(0, 5, false), None);
        assert_eq!(w24a39_logical_width(1920, 1080, false), Some((853, 213)));
        assert_eq!(w24a39_logical_width(1000, 1000, false), Some((640, 0)));
        assert_eq!(w24a39_logical_width(3000, 1000, false), Some((853, 213)));
        assert_eq!(w24a39_logical_width(1920, 1080, true), Some((640, 0)));
    }

    #[test]
    fn deque_slot() {
        let mut size = 3;
        let map = [0, 7];
        assert_eq!(
            w24a39_deque_push_slot(&map, 0, &mut size),
            W24a39Slot::NeedBlock
        );
        assert_eq!(size, 4);
        let mut size = 3;
        assert_eq!(
            w24a39_deque_push_slot(&[5, 7], 0, &mut size),
            W24a39Slot::Slot(0, 3)
        );
        let mut size = 16;
        assert_eq!(
            w24a39_deque_push_slot(&[5, 7, 1, 1], 0, &mut size),
            W24a39Slot::Slot(1, 0)
        );
        let mut size = 16;
        assert_eq!(
            w24a39_deque_push_slot(&[5], 0, &mut size),
            W24a39Slot::NeedGrow
        );
        assert_eq!(size, 16);
    }

    #[test]
    fn state() {
        assert!(w24a39_state_is_active(1) && w24a39_state_is_active(4));
        assert!(!w24a39_state_is_active(0) && !w24a39_state_is_active(2));
    }
}
