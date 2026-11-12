#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent05.rs
// ===========================================================================
// Wave 29 translation batch 05.
//
// Graphics-driver / STL glue; only two routines carry recoverable logic.

/// `0x00aa6ed0`: maps a PCI vendor id to the length of the vendor string
/// the original constructs (string bytes are not visible in the body; only
/// the length passed to the string builder is).  Unknown vendors use 7.
pub fn w29a05_vendor_name_len(vendor_id: u16) -> usize {
    match vendor_id {
        0x10de => 6,
        0x1002 => 3,
        0x106b => 5,
        0x1414 => 9,
        0x8086 => 5,
        _ => 7,
    }
}

/// `0x00aa90c0`: hash-table bucket count for a requested minimum.  `size`
/// is the element count (`in_ECX[2]`), `buckets` the current count,
/// `max_load` the load factor, `hint` the value from the unresolved sizing
/// call (`size + 1` is used when load is above the limit).  Returns the
/// power-of-two (>= 8) count and whether the table is already that size.
pub fn w29a05_bucket_count(
    size: i32,
    cur_buckets: u32,
    has_table: bool,
    max_load: f32,
    sizing: u32,
    wanted: u32,
) -> (u32, bool) {
    let mut n = sizing.wrapping_add(1);
    if (size as f64) as f32 <= (sizing as f64) as f32 * max_load {
        n = sizing;
    }
    if n < wanted {
        n = wanted;
    }
    let mut p = 8u32;
    if n > 8 {
        while p < n {
            p *= 2;
        }
    }
    (p, p == cur_buckets && has_table)
}

#[cfg(test)]
mod tests_wave29_agent05 {
    use super::*;

    #[test]
    fn vendor_lengths() {
        assert_eq!(w29a05_vendor_name_len(0x10de), 6);
        assert_eq!(w29a05_vendor_name_len(0x1002), 3);
        assert_eq!(w29a05_vendor_name_len(0x1414), 9);
        assert_eq!(w29a05_vendor_name_len(0x8086), 5);
        assert_eq!(w29a05_vendor_name_len(0x106b), 5);
        assert_eq!(w29a05_vendor_name_len(1), 7);
    }

    #[test]
    fn bucket_count() {
        assert_eq!(w29a05_bucket_count(1, 8, true, 1.0, 4, 0), (8, true));
        assert_eq!(w29a05_bucket_count(1, 8, false, 1.0, 4, 0), (8, false));
        assert_eq!(w29a05_bucket_count(100, 8, true, 1.0, 20, 0), (32, false));
        assert_eq!(w29a05_bucket_count(1, 8, true, 1.0, 4, 100), (128, false));
    }
}
