#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent125.rs
// ===========================================================================
// Wave 32 agent 125: CRT lock-table slot registration.
// Cites 0x00c84b64 below.

/// `0x00c84b64`: registers `obj` in an 8-entry slot table (slot 0 unused).
/// Scans from slot 1 for an empty slot or one already holding `obj`; if none
/// matches the scan ends with index 8 (original quirk: the table has 8
/// entries, so the caller must size `slots` and `counts` to at least 9).
/// Increments the per-slot use counter, stores `obj`, and returns the index,
/// which the original also writes to `*(obj + 8)`.  The lock/unlock pair
/// (callees 2 / unlock) is supplied by the caller around this call.
pub fn w32a125_register_slot(slots: &mut [u32], counts: &mut [u8], obj: u32) -> u32 {
    let mut idx: u32 = 1;
    loop {
        let cur = slots[idx as usize];
        if cur == 0 || cur == obj {
            break;
        }
        idx += 1;
        if idx >= 8 {
            break;
        }
    }
    counts[idx as usize] = counts[idx as usize].wrapping_add(1);
    slots[idx as usize] = obj;
    idx
}

#[cfg(test)]
mod tests_wave32_agent125 {
    use super::*;

    #[test]
    fn registers_first_empty_and_reuses() {
        let mut s = [0u32; 9];
        let mut c = [0u8; 9];
        assert_eq!(w32a125_register_slot(&mut s, &mut c, 10), 1);
        assert_eq!(w32a125_register_slot(&mut s, &mut c, 20), 2);
        assert_eq!(w32a125_register_slot(&mut s, &mut c, 10), 1);
        assert_eq!(c[1], 2);
        assert_eq!(s[2], 20);
    }

    #[test]
    fn full_table_ends_at_eight() {
        let mut s = [0u32; 9];
        let mut c = [0u8; 9];
        for i in 1..8 {
            s[i] = 100 + i as u32;
        }
        assert_eq!(w32a125_register_slot(&mut s, &mut c, 5), 8);
    }
}
