#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent25.rs
// ===========================================================================
// Wave 25 agent 25: tagged-slot reset and scalar-deleting-destructor helpers.

/// Offsets (from the owner base) of the six reset slots in `0x00820980`.
pub const W25A25_SLOT_OFFSETS: [usize; 6] = [0xb4, 200, 0xdc, 0xf0, 0x104, 0x118];

/// `0x00820980`: for each of the slots at `W25A25_SLOT_OFFSETS` (plus `300`),
/// resolves the tagged pointer (low bit set => masked pointer, else inline),
/// invokes the manager with 0 when non-null, and stores the empty marker 1.
/// Returns the number of slots whose manager was invoked.  A slot whose tag
/// bit is still set after the call is the non-returning error path in retail;
/// here it is reported via `Err(slot_index)`.
pub fn w25a25_reset_slots(
    slots: &mut [u32; 7],
    mut call_manager: impl FnMut(usize, u32) -> Option<u32>,
) -> Result<usize, usize> {
    let mut called = 0;
    for (i, slot) in slots.iter_mut().enumerate() {
        let target = if *slot & 1 != 0 { *slot & !1 } else { *slot };
        if target != 0 {
            let new = call_manager(i, target).unwrap_or(*slot);
            called += 1;
            if new & 1 != 0 {
                return Err(i);
            }
            *slot = 1;
        }
    }
    Ok(called)
}

/// `0x0081f260`: scalar deleting destructor; runs the destructor and reports
/// whether the memory must be freed (`flags & 1`).
pub fn w25a25_scalar_delete_a(flags: u32, dtor: impl FnOnce()) -> bool {
    dtor();
    flags & 1 != 0
}

/// `0x0081f290`: scalar deleting destructor twin of the previous helper.
pub fn w25a25_scalar_delete_b(flags: u32, dtor: impl FnOnce()) -> bool {
    dtor();
    flags & 1 != 0
}

#[cfg(test)]
mod tests_wave25_agent25 {
    use super::*;

    #[test]
    fn reset_slots_empties_nonnull() {
        let mut s = [1, 0x1001, 0, 0x2000, 1, 1, 0x3000];
        let mut calls = vec![];
        let r = w25a25_reset_slots(&mut s, |i, t| {
            calls.push((i, t));
            Some(0)
        });
        assert_eq!(r, Ok(3));
        assert_eq!(s, [1, 1, 0, 1, 1, 1, 1]);
        assert_eq!(calls[0], (1, 0x1000));
    }

    #[test]
    fn reset_slots_error_path() {
        let mut s = [0x1001, 0, 0, 0, 0, 0, 0];
        assert_eq!(w25a25_reset_slots(&mut s, |_, t| Some(t | 1)), Err(0));
    }

    #[test]
    fn scalar_delete_flags() {
        let mut n = 0;
        assert!(w25a25_scalar_delete_a(1, || n += 1));
        assert!(!w25a25_scalar_delete_b(0, || n += 1));
        assert_eq!(n, 2);
    }
}
