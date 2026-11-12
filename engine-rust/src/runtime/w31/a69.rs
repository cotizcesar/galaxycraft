#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent69.rs
// ===========================================================================
// Wave 31 agent 69: renderer-side helpers (pure reference models).

/// `0x00a98320`: address of the 0x20-byte record `index` in one of two
/// tables (`0x0136bc98` when `second` is non-zero, else `0x0136bcf8`).
pub fn w31a69_record_addr(index: i32, second: i32) -> u32 {
    let base: u32 = if second == 0 {
        0x0136_bcf8
    } else {
        0x0136_bc98
    };
    base.wrapping_add((index as u32).wrapping_mul(0x20))
}

/// `0x00a9ae80` (first half): primitive mode for a draw kind
/// (0/2/4 -> 5, 3 -> 1, everything else 4).
pub fn w31a69_primitive_mode(kind: u32) -> u32 {
    match kind {
        0 | 2 | 4 => 5,
        3 => 1,
        _ => 4,
    }
}

/// `0x00a9ae80` (second half): element count is `total / stride`; a zero
/// stride draws nothing.
pub fn w31a69_element_count(total: u32, stride: u32) -> Option<u32> {
    total.checked_div(stride)
}

/// `0x00a9d790`: fills `[origin, min(a, b), 0]` in the output triple.
pub fn w31a69_fill_extent(out: &mut [u32; 3], origin: u32, a: u32, b: u32) {
    out[2] = 0;
    out[0] = origin;
    out[1] = a.min(b);
}

/// `0x00a8df30`: output dimension; `explicit` wins, otherwise
/// `(int)((float)(v + table[v < 0]) * scale)`.  `table` is the two-entry
/// double array at the unsigned-fixup global.
pub fn w31a69_scaled_dim(explicit: Option<i32>, v: i32, table: [f64; 2], scale: f32) -> i32 {
    match explicit {
        Some(x) => x,
        None => ((v as f64 + table[(v < 0) as usize]) as f32 * scale) as i32,
    }
}

/// `0x00a958a0`: true when slot `slot` (< 0x100, stride 0xd8) is live and
/// its stored key equals the non-zero `key`.
pub fn w31a69_slot_matches(slot: i32, key: i32, stored_key: impl Fn(usize) -> i32) -> bool {
    key != 0 && (0..0x100).contains(&slot) && stored_key(slot as usize) == key
}

/// Ops issued by `0x00a9cb00`.
#[derive(Debug, PartialEq, Eq)]
pub enum W31a69Op {
    None,
    /// disable(0xbe2) then call (0x302, arg)
    DisableThen(u32),
    /// enable(0xbe2)
    Enable,
}

/// `0x00a9cb00`: state-change decision; no op when unchanged.
pub fn w31a69_set_mode(current: i16, new: i16) -> W31a69Op {
    if new == current {
        return W31a69Op::None;
    }
    match new {
        0 => W31a69Op::DisableThen(0x303),
        1 => W31a69Op::Enable,
        2 => W31a69Op::DisableThen(1),
        _ => W31a69Op::None,
    }
}

#[cfg(test)]
mod tests_wave31_agent69 {
    use super::*;

    #[test]
    fn small_helpers() {
        assert_eq!(w31a69_record_addr(2, 0), 0x0136_bcf8 + 0x40);
        assert_eq!(w31a69_record_addr(1, 1), 0x0136_bc98 + 0x20);
        assert_eq!(w31a69_primitive_mode(3), 1);
        assert_eq!(w31a69_primitive_mode(4), 5);
        assert_eq!(w31a69_primitive_mode(9), 4);
        assert_eq!(w31a69_element_count(12, 4), Some(3));
        assert_eq!(w31a69_element_count(12, 0), None);
        let mut o = [9; 3];
        w31a69_fill_extent(&mut o, 7, 5, 3);
        assert_eq!(o, [7, 3, 0]);
    }

    #[test]
    fn dims_and_slots() {
        let t = [0.0, 4294967296.0];
        assert_eq!(w31a69_scaled_dim(Some(5), 100, t, 2.0), 5);
        assert_eq!(w31a69_scaled_dim(None, 100, t, 0.5), 50);
        assert!(w31a69_slot_matches(3, 8, |_| 8));
        assert!(!w31a69_slot_matches(0x100, 8, |_| 8));
        assert!(!w31a69_slot_matches(-1, 8, |_| 8));
        assert!(!w31a69_slot_matches(3, 0, |_| 0));
    }

    #[test]
    fn mode() {
        assert_eq!(w31a69_set_mode(1, 1), W31a69Op::None);
        assert_eq!(w31a69_set_mode(1, 0), W31a69Op::DisableThen(0x303));
        assert_eq!(w31a69_set_mode(0, 1), W31a69Op::Enable);
        assert_eq!(w31a69_set_mode(0, 2), W31a69Op::DisableThen(1));
        assert_eq!(w31a69_set_mode(0, 5), W31a69Op::None);
    }
}
