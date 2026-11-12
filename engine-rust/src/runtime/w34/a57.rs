#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent57.rs
// ===========================================================================
// Wave 34 translation batch 57.
//
// Two CRT helpers that release a handle unless it equals a global sentinel;
// the sentinel and the release routine are caller-supplied.
// FUN_00ca69a8 is skipped (locked stream write with unmodelled callees).

/// `0x00caab9a`: if `*handle` differs from the sentinel global (the one at
/// the first of the two adjacent slots), calls the release routine on it.
pub fn w34a57_release_unless_sentinel_a(handle: &i32, sentinel: i32, release: &mut dyn FnMut(i32)) {
    if *handle != sentinel {
        release(*handle);
    }
}

/// `0x00caabb5`: same as the neighbouring helper but compared against the
/// second sentinel global.
pub fn w34a57_release_unless_sentinel_b(handle: &i32, sentinel: i32, release: &mut dyn FnMut(i32)) {
    if *handle != sentinel {
        release(*handle);
    }
}

#[cfg(test)]
mod tests_wave34_agent57 {
    use super::*;

    #[test]
    fn releases_only_when_different() {
        let mut got = vec![];
        w34a57_release_unless_sentinel_a(&5, 5, &mut |h| got.push(h));
        assert!(got.is_empty());
        w34a57_release_unless_sentinel_b(&6, 5, &mut |h| got.push(h));
        assert_eq!(got, vec![6]);
    }
}
