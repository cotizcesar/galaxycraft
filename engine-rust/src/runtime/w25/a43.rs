#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent43.rs
// ===========================================================================
// Wave 25 agent 43: small container-iteration helpers (runtime.rs target).

/// Element count of a 4-byte-element range `[begin, begin + len*4)`:
/// `((end - begin) + 3) >> 2`, or 0 when the end wraps below `begin`.
fn w25a43_range_count(begin: u32, len: u32) -> u32 {
    let end = begin.wrapping_add(len.wrapping_mul(4));
    if end < begin {
        0
    } else {
        end.wrapping_sub(begin).wrapping_add(3) >> 2
    }
}

/// `0x00881190`: runs the per-element callback once per element of the
/// range and returns the last result with the low byte cleared (0 when
/// the range is empty).
pub fn w25a43_apply_last_masked(begin: u32, len: u32, mut f: impl FnMut() -> u32) -> u32 {
    let mut last = 0;
    for _ in 0..w25a43_range_count(begin, len) {
        last = f();
    }
    last & 0xffff_ff00
}

/// `0x00881240`: same loop as the sibling at `881190` with a different
/// per-element callee; returns the last result with the low byte cleared.
pub fn w25a43_apply_last_masked_b(begin: u32, len: u32, f: impl FnMut() -> u32) -> u32 {
    w25a43_apply_last_masked(begin, len, f)
}

/// `0x00886400`: runs the callback once per element of the range.
pub fn w25a43_apply_each(begin: u32, len: u32, mut f: impl FnMut()) {
    for _ in 0..w25a43_range_count(begin, len) {
        f();
    }
}

/// `0x00886790`: runs the callback for each of the `count & 0x3fffffff`
/// elements of a vector (0 if `base + count*4` wraps below `base`).
pub fn w25a43_for_each_element(base: u32, count: u32, mut f: impl FnMut()) {
    let mut n = count & 0x3fff_ffff;
    if count.wrapping_mul(4).wrapping_add(base) < base {
        n = 0;
    }
    for _ in 0..n {
        f();
    }
}

/// `0x00886700`: clears a pointer-slot array and resets its used count.
/// A non-null slot diverges in the retail code; returned as `Err(index)`.
pub fn w25a43_clear_slots(slots: Option<&mut [u32]>, used: &mut u32) -> Result<(), usize> {
    if let Some(s) = slots {
        for (i, v) in s.iter_mut().enumerate() {
            if *v != 0 {
                return Err(i);
            }
            *v = 0;
        }
    }
    *used = 0;
    Ok(())
}

#[cfg(test)]
mod tests_wave25_agent43 {
    use super::*;

    #[test]
    fn counts_and_loops() {
        let mut n = 0;
        w25a43_apply_each(0x1000, 5, || n += 1);
        assert_eq!(n, 5);
        let mut k = 0;
        assert_eq!(
            w25a43_apply_last_masked(0x1000, 3, || {
                k += 1;
                0x1234 + k
            }),
            0x1200
        );
        assert_eq!(w25a43_apply_last_masked_b(0x1000, 0, || 7), 0);
        assert_eq!(w25a43_apply_last_masked_b(0xffff_fff0, 8, || 7), 0);
    }

    #[test]
    fn elements_and_clear() {
        let mut n = 0;
        w25a43_for_each_element(0x100, 4, || n += 1);
        assert_eq!(n, 4);
        n = 0;
        w25a43_for_each_element(0xffff_fff0, 8, || n += 1);
        assert_eq!(n, 0);
        let mut used = 3;
        let mut a = [0u32; 3];
        assert_eq!(w25a43_clear_slots(Some(&mut a), &mut used), Ok(()));
        assert_eq!(used, 0);
        let mut b = [0, 5, 0];
        used = 3;
        assert_eq!(w25a43_clear_slots(Some(&mut b), &mut used), Err(1));
        assert_eq!(used, 3);
    }
}
