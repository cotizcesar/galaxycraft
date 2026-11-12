#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave34/agent55.rs
// ===========================================================================
// Wave 34 agent 55 translations.

/// `0x00c9fe04`: thiscall helper.  Calls the inner lookup with
/// `this + 8` and the dword at `this + 0x68`, feeds its result to the
/// follow-up routine and always returns 1.
pub fn w34a55_lookup_then_apply(
    this_plus_8: u32,
    field_68: u32,
    lookup: impl FnOnce(u32, u32) -> u32,
    apply: impl FnOnce(u32),
) -> u32 {
    apply(lookup(this_plus_8, field_68));
    1
}

/// `0x00ca207a`: address of record `index` in the global table at
/// `0x00f4e670`; the record stride is `0x38` bytes.
pub fn w34a55_table_entry_address(index: i32) -> u32 {
    0x00f4_e670u32.wrapping_add((index as u32).wrapping_mul(0x38))
}

#[cfg(test)]
mod tests_wave34_agent55 {
    use super::*;

    #[test]
    fn lookup_then_apply_passes_result() {
        let mut seen = 0;
        let r = w34a55_lookup_then_apply(0x18, 7, |a, b| a + b, |v| seen = v);
        assert_eq!(r, 1);
        assert_eq!(seen, 0x1f);
    }

    #[test]
    fn table_entry_stride() {
        assert_eq!(w34a55_table_entry_address(0), 0x00f4_e670);
        assert_eq!(w34a55_table_entry_address(2), 0x00f4_e670 + 0x70);
    }
}
