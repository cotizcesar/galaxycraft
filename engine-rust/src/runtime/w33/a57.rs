#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent57.rs
// ===========================================================================
// Wave 33 translation batch 57.
//
// Skipped: FUN_00cad1ce (ends in non-returning call), FUN_00cad660
// (depends on unmodelled CRT callees).

/// 0x00cacff7: frees the owned blocks of a CRT per-thread locale/data
/// record. Slot 0 is only freed when it is not the static default.
/// Returns the slots that were released, in call order.
pub fn w33a57_free_thread_data(
    slots: &[u32],
    default_ptr: u32,
    free: &mut dyn FnMut(u32),
) -> Vec<u32> {
    let mut freed = Vec::new();
    let mut f = |p: u32| {
        free(p);
        freed.push(p);
    };
    if let Some(&s0) = slots.first() {
        if s0 != default_ptr {
            f(s0);
        }
    }
    for i in [0xf, 0xc, 0xd, 0xe, 10, 0xb, 0x10, 0x11, 0xd8] {
        f(slots.get(i).copied().unwrap_or(0));
    }
    freed
}

#[cfg(test)]
mod tests_wave33_agent57 {
    use super::*;

    #[test]
    fn frees_in_order_and_skips_default() {
        let mut slots = vec![0u32; 0xd9];
        slots[0] = 0x100;
        slots[0xf] = 1;
        slots[0xd8] = 9;
        let mut seen = vec![];
        let r = w33a57_free_thread_data(&slots, 0x100, &mut |p| seen.push(p));
        assert_eq!(r, seen);
        assert_eq!(r.len(), 9);
        assert_eq!(r[0], 1);
        assert_eq!(r[8], 9);
        let r2 = w33a57_free_thread_data(&slots, 0x200, &mut |_| {});
        assert_eq!(r2.len(), 10);
        assert_eq!(r2[0], 0x100);
    }
}
