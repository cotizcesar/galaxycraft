#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent52.rs
// ===========================================================================
// Wave 31 agent 52: small runtime helpers.

/// `0x008eb7a0`: clears a one-slot indirect flag and its companion word.
/// Widths of the globals are not visible in the pseudocode; modelled as u32.
pub fn w31a52_clear_pair(flag: &mut u32, companion: &mut u32) {
    *flag = 0;
    *companion = 0;
}

/// `0x008f4b30`: replaces a global object slot.  The previous object (if
/// any) is destroyed with flag 1; the new object (if any) has its field at
/// offset 0x138 zeroed and two reset callbacks run.
pub fn w31a52_replace_slot<T>(
    slot: &mut Option<T>,
    new: Option<T>,
    destroy_old: impl FnOnce(T, u32),
    mut clear_field_138: impl FnMut(&mut T),
    mut reset: impl FnMut(),
) {
    let old = std::mem::replace(slot, new);
    if let Some(o) = old {
        destroy_old(o, 1);
    }
    if let Some(n) = slot.as_mut() {
        clear_field_138(n);
        reset();
        reset();
    }
}

/// `0x008f7890`: when `busy` is false, visits every 0xb8-byte element of
/// the list (count `n`) and then invokes the vtable slot 0x78 with 1;
/// finally forwards the two context words.  Returns the forwarded pair.
pub fn w31a52_visit_elements(
    n: usize,
    busy: bool,
    mut visit: impl FnMut(usize),
    mut vtable_78: impl FnMut(u32),
    ctx: (u32, u32),
) -> (u32, u32) {
    if !busy {
        for i in 0..n {
            visit(i);
        }
        vtable_78(1);
    }
    ctx
}

#[cfg(test)]
mod tests_wave31_agent52 {
    use super::*;

    #[test]
    fn clear_pair_zeroes() {
        let (mut a, mut b) = (5, 9);
        w31a52_clear_pair(&mut a, &mut b);
        assert_eq!((a, b), (0, 0));
    }

    #[test]
    fn replace_slot_runs_callbacks() {
        let mut slot = Some(1u32);
        let mut destroyed = vec![];
        let mut resets = 0;
        w31a52_replace_slot(
            &mut slot,
            Some(2),
            |o, f| destroyed.push((o, f)),
            |n| *n = 0,
            || resets += 1,
        );
        assert_eq!(destroyed, vec![(1, 1)]);
        assert_eq!(slot, Some(0));
        assert_eq!(resets, 2);
        w31a52_replace_slot(
            &mut slot,
            None,
            |o, f| destroyed.push((o, f)),
            |_| panic!(),
            || panic!(),
        );
        assert_eq!(destroyed.len(), 2);
    }

    #[test]
    fn visit_elements_gated() {
        let mut seen = vec![];
        let mut v = vec![];
        assert_eq!(
            w31a52_visit_elements(3, false, |i| seen.push(i), |x| v.push(x), (7, 8)),
            (7, 8)
        );
        assert_eq!(seen, vec![0, 1, 2]);
        assert_eq!(v, vec![1]);
        seen.clear();
        w31a52_visit_elements(3, true, |i| seen.push(i), |_| panic!(), (0, 0));
        assert!(seen.is_empty());
    }
}
