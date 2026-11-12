#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent118.rs
// ===========================================================================
// Wave 32 agent 118.

/// `0x00c5d590`: stores `new_mask` into the cached mask (`+0x188`) and, for
/// every bit that differs from the old value (lowest bit first), invokes
/// `notify(bit_index, new_state)`.  Returns the changed-bit set.
pub fn w32a118_apply_mask_diff(
    cached: &mut u32,
    new_mask: u32,
    mut notify: impl FnMut(u32, bool),
) -> u32 {
    let diff = *cached ^ new_mask;
    if diff == 0 {
        return 0;
    }
    *cached = new_mask;
    let mut rest = diff;
    while rest != 0 {
        let i = rest.trailing_zeros();
        let bit = 1u32 << i;
        rest &= !bit;
        notify(i, new_mask & bit != 0);
    }
    diff
}

#[cfg(test)]
mod tests_wave32_agent118 {
    use super::*;

    #[test]
    fn diff_notifies_low_to_high() {
        let mut c = 0b0110;
        let mut seen = Vec::new();
        let d = w32a118_apply_mask_diff(&mut c, 0b1010, |i, s| seen.push((i, s)));
        assert_eq!(d, 0b1100);
        assert_eq!(c, 0b1010);
        assert_eq!(seen, vec![(2, false), (3, true)]);
        assert_eq!(w32a118_apply_mask_diff(&mut c, 0b1010, |_, _| panic!()), 0);
    }
}
