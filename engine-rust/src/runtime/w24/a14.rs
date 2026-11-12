#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent14.rs
// ===========================================================================
// Wave 24 agent 14 batch.

/// `0x007448f0`: returns the byte at `index` of the 8-entry table at
/// DAT_011a30a8, or 0 when `index` is 8 or more.
pub fn w24a14_table_byte(table: &[u8; 8], index: u8) -> u8 {
    if index > 7 {
        return 0;
    }
    table[index as usize]
}

/// `0x00742df0`: record status byte (at +8) dispatch.  Status 0x0c leaves
/// the status unchanged; status 9 is rewritten to 2; in both cases the
/// record's first dword is handed to the (non-returning) callee.  Returns
/// `Some(first_dword)` when the callee is invoked, else `None`.
pub fn w24a14_status_dispatch(first: u32, status: &mut u8) -> Option<u32> {
    if *status == 0x0c {
        return Some(first);
    }
    if *status == 0x09 {
        *status = 2;
        return Some(first);
    }
    None
}

#[cfg(test)]
mod tests_wave24_agent14 {
    use super::*;

    #[test]
    fn table_byte_bounds() {
        let t = [1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(w24a14_table_byte(&t, 0), 1);
        assert_eq!(w24a14_table_byte(&t, 7), 8);
        assert_eq!(w24a14_table_byte(&t, 8), 0);
    }

    #[test]
    fn status_dispatch() {
        let mut s = 0x0c;
        assert_eq!(w24a14_status_dispatch(5, &mut s), Some(5));
        assert_eq!(s, 0x0c);
        s = 9;
        assert_eq!(w24a14_status_dispatch(6, &mut s), Some(6));
        assert_eq!(s, 2);
        assert_eq!(w24a14_status_dispatch(7, &mut s), None);
    }
}
