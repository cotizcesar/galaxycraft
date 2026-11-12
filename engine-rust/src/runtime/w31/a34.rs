#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent34.rs
// ===========================================================================
// Wave 31 batch 34: small runtime helpers.

/// `0x007486f0`: parses a comma-terminated hex field.  `None` input stores
/// `default` and returns `None`; an empty field stores `default` and returns
/// the offset after the comma; otherwise digits (case-insensitive) are
/// accumulated until a comma (returns offset after it).  Any non-hex byte,
/// or the end of the slice (NUL), returns `None` with the partial value.
pub fn w31a34_parse_hex_field(out: &mut i32, s: Option<&[u8]>, default: i32) -> Option<usize> {
    let s = match s {
        None => {
            *out = default;
            return None;
        }
        Some(s) => s,
    };
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    if at(0) == b',' {
        *out = default;
        return Some(1);
    }
    *out = 0;
    let mut i = 0usize;
    loop {
        let c = at(i);
        if c == b',' {
            return Some(i + 1);
        }
        i += 1;
        let c = c.to_ascii_lowercase();
        let mut d = c.wrapping_sub(0x30);
        if d > 9 {
            if c.wrapping_sub(b'a') > 5 {
                return None;
            }
            d = c.wrapping_sub(b'a') + 10;
        }
        *out = out.wrapping_mul(16).wrapping_add(d as i32);
    }
}

/// `0x0073cbd0`: true when the feature flag is off, otherwise true when the
/// lowest active slot among 8 equals `current`.  Inactive slots leave the
/// minimum at 8.
pub fn w31a34_is_lowest_active_slot(
    enabled: bool,
    current: u32,
    active: impl Fn(u32) -> bool,
) -> bool {
    if !enabled {
        return true;
    }
    let mut lowest = 8u32;
    for i in 0..8u32 {
        if active(i) && i < lowest {
            lowest = i;
        }
    }
    lowest == current
}

/// `0x00759090`: returns 1 unless the global is set and bit 2 of the flags
/// word is clear.
pub fn w31a34_flag_bit2_or_unset(global: u32, flags: u32) -> u32 {
    if global != 0 && flags & 4 == 0 { 0 } else { 1 }
}

/// `0x0075a690`: copies the two dwords of `src` into the pair of globals.
pub fn w31a34_store_pair(src: [u32; 2], dst: &mut [u32; 2]) {
    dst[0] = src[0];
    dst[1] = src[1];
}

/// `0x00758d40`: indexed dword table lookup.
pub fn w31a34_table_lookup(table: &[u32], index: usize) -> Option<u32> {
    table.get(index).copied()
}

/// `0x0074f0c0`: true when the queried handle status equals 1.
pub fn w31a34_status_is_one(handle: u32, query: impl Fn(u32) -> i32) -> bool {
    query(handle) == 1
}

/// `0x0074fb80`: true when the handle query returns zero.
pub fn w31a34_query_is_zero(handle: u32, query: impl Fn(u32) -> u8) -> bool {
    query(handle) == 0
}

#[cfg(test)]
mod tests_wave31_agent34 {
    use super::*;

    #[test]
    fn hex_field() {
        let mut v = -1;
        assert_eq!(w31a34_parse_hex_field(&mut v, None, 7), None);
        assert_eq!(v, 7);
        assert_eq!(w31a34_parse_hex_field(&mut v, Some(b",x"), 9), Some(1));
        assert_eq!(v, 9);
        assert_eq!(w31a34_parse_hex_field(&mut v, Some(b"1Fa,z"), 0), Some(4));
        assert_eq!(v, 0x1fa);
        assert_eq!(w31a34_parse_hex_field(&mut v, Some(b"1g,"), 0), None);
        assert_eq!(w31a34_parse_hex_field(&mut v, Some(b"12"), 0), None);
    }

    #[test]
    fn lowest_slot() {
        assert!(w31a34_is_lowest_active_slot(false, 5, |_| false));
        assert!(w31a34_is_lowest_active_slot(true, 2, |i| i == 2 || i == 5));
        assert!(!w31a34_is_lowest_active_slot(true, 5, |i| i == 2 || i == 5));
        assert!(w31a34_is_lowest_active_slot(true, 8, |_| false));
    }

    #[test]
    fn small_helpers() {
        assert_eq!(w31a34_flag_bit2_or_unset(0, 0), 1);
        assert_eq!(w31a34_flag_bit2_or_unset(1, 0), 0);
        assert_eq!(w31a34_flag_bit2_or_unset(1, 4), 1);
        let mut d = [0; 2];
        w31a34_store_pair([3, 4], &mut d);
        assert_eq!(d, [3, 4]);
        assert_eq!(w31a34_table_lookup(&[1, 2], 1), Some(2));
        assert_eq!(w31a34_table_lookup(&[1, 2], 2), None);
        assert!(w31a34_status_is_one(0, |_| 1));
        assert!(!w31a34_status_is_one(0, |_| 2));
        assert!(w31a34_query_is_zero(0, |_| 0));
        assert!(!w31a34_query_is_zero(0, |_| 1));
    }
}
