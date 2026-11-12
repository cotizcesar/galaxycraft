#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent05.rs
// ===========================================================================
// Wave 32 translation batch 05.

/// `0x006ae870`: locates `param_3` inside a bucketed offset table.  `base`
/// is `this+4`, `modulus` the divisor `**(this+0x18)`, `offsets` the table
/// at `(*(this+0x18))+0xc`, `flags` is `this+0x1c`.  `lookup` models the hash
/// find `0x53_e8a0` and returns the 4-byte-or-larger value's first dword.
/// Returns `[bucket_start, found_offset(-1 if none), extra]`; `extra` is
/// `Some` only when the lookup supplied a value.  Result is always 1 in the
/// original.
pub fn w32a05_locate_offset(
    base: i32,
    modulus: u32,
    offsets: &[i32],
    flags: u32,
    param_3: u32,
    want_lookup: bool,
    lookup: impl Fn(i32) -> Option<i32>,
) -> (i32, i32, Option<i32>) {
    let start = base
        .wrapping_sub((param_3 % modulus) as i32)
        .wrapping_add(param_3 as i32);
    let target = base.wrapping_add(param_3 as i32);
    for &o in offsets {
        if start.wrapping_add(o) == target {
            let mut extra = None;
            if o != -1 && want_lookup && (flags >> 1) & 1 == 0 {
                extra = lookup(target);
            }
            return (start, o, extra);
        }
    }
    (start, -1, None)
}

/// `0x006b01c0`: looks up `key` through the hash find `0x53_e8a0`; when the
/// entry exists and its length field exceeds 3, returns its first dword.
pub fn w32a05_lookup_first_dword(entry: Option<(&[u8], u32)>) -> Option<u32> {
    let (data, len) = entry?;
    if len > 3 && data.len() >= 4 {
        Some(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
    } else {
        None
    }
}

/// `0x006e01f0`: clamps `v` to at most 0x18, raises anything below 5 to 4
/// (note: values 0..4 map to 4), and returns the byte passed on to
/// `0x72_e710`.
pub fn w32a05_clamp_level(v: i32) -> u8 {
    let c = v.min(0x18);
    if c < 5 { 4 } else { c as u8 }
}

#[cfg(test)]
mod tests_wave32_agent05 {
    use super::*;

    #[test]
    fn locate() {
        let r = w32a05_locate_offset(100, 8, &[3, 5], 0, 11, true, |t| Some(t * 2));
        // start = 100 - 3 + 11 = 108; target = 111; 108+3 == 111
        assert_eq!(r, (108, 3, Some(222)));
        let r = w32a05_locate_offset(100, 8, &[1], 0, 11, true, |_| None);
        assert_eq!(r, (108, -1, None));
        let r = w32a05_locate_offset(100, 8, &[3], 2, 11, true, |_| Some(1));
        assert_eq!(r.2, None);
    }

    #[test]
    fn first_dword() {
        assert_eq!(w32a05_lookup_first_dword(Some((&[1, 0, 0, 0], 4))), Some(1));
        assert_eq!(w32a05_lookup_first_dword(Some((&[1, 0, 0, 0], 3))), None);
        assert_eq!(w32a05_lookup_first_dword(None), None);
    }

    #[test]
    fn clamp() {
        assert_eq!(w32a05_clamp_level(2), 4);
        assert_eq!(w32a05_clamp_level(10), 10);
        assert_eq!(w32a05_clamp_level(99), 0x18);
    }
}
