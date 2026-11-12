#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent77.rs
// ===========================================================================
/// `0x00b53540`: linear search over a table of 16-byte records whose first
/// word points at a NUL-terminated string; returns the index of the first
/// record whose string equals `wanted` (byte compare up to the NUL).
pub fn w31a77_find_record_by_name(names: &[&[u8]], wanted: &[u8]) -> Option<usize> {
    let cut = |s: &[u8]| s.iter().position(|&b| b == 0).unwrap_or(s.len());
    let w = &wanted[..cut(wanted)];
    names.iter().position(|n| &n[..cut(n)] == w)
}

/// `0x00b54030`: bool to the literal `"true"` / `"false"`.
pub fn w31a77_bool_str(v: bool) -> &'static str {
    if v { "true" } else { "false" }
}

/// `0x00b547a0`: sets or clears bits `[start, end)` of an MSB-first bitset
/// (bit `i` is `0x80 >> (i & 7)` of byte `i >> 3`).
pub fn w31a77_bitset_fill(bits: &mut [u8], start: u32, end: u32, value: bool) {
    for i in start..end {
        let m = 0x80u8 >> (i & 7);
        let b = &mut bits[(i >> 3) as usize];
        if value {
            *b |= m;
        } else {
            *b &= !m;
        }
    }
}

/// `0x00b548c0`: first index in `[start, end)` whose bit equals `value`,
/// or `end` when there is none.
pub fn w31a77_bitset_find(bits: &[u8], start: u32, end: u32, value: bool) -> u32 {
    (start..end)
        .find(|&i| (bits[(i >> 3) as usize] & (0x80u8 >> (i & 7)) != 0) == value)
        .unwrap_or(end)
}

/// `0x00b549e0`: true when every bit in `[start, end)` equals `value`
/// (an empty range is true).
pub fn w31a77_bitset_all(bits: &[u8], start: u32, end: u32, value: bool) -> bool {
    w31a77_bitset_find(bits, start, end, !value) == end
}

/// `0x00b545c0`: formats a broken-down UTC time as `YYYY-MM-DDTHH:MM:SSZ`
/// (`compact` drops the `-` and `:` separators).  `tm` is
/// `[sec, min, hour, mday, mon0, year-1900]`.
pub fn w31a77_format_iso8601(tm: [i32; 6], compact: bool) -> String {
    let [sec, min, hour, mday, mon, yr] = tm;
    let y = yr + 0x76c;
    let sep = |c: char| {
        if compact {
            String::new()
        } else {
            c.to_string()
        }
    };
    format!(
        "{:04}{}{:02}{}{:02}T{:02}{}{:02}{}{:02}Z",
        y % 10000,
        sep('-'),
        mon + 1,
        sep('-'),
        mday,
        hour,
        sep(':'),
        min,
        sep(':'),
        sec
    )
}

#[cfg(test)]
mod tests_wave31_agent77 {
    use super::*;

    #[test]
    fn find_and_bool() {
        let n: [&[u8]; 2] = [b"ab\0", b"cd"];
        assert_eq!(w31a77_find_record_by_name(&n, b"cd\0x"), Some(1));
        assert_eq!(w31a77_find_record_by_name(&n, b"ab"), Some(0));
        assert_eq!(w31a77_find_record_by_name(&n, b"zz"), None);
        assert_eq!(w31a77_bool_str(true), "true");
        assert_eq!(w31a77_bool_str(false), "false");
    }

    #[test]
    fn bitsets() {
        let mut b = [0u8; 4];
        w31a77_bitset_fill(&mut b, 3, 21, true);
        assert_eq!(b, [0x1f, 0xff, 0xf8, 0]);
        assert_eq!(w31a77_bitset_find(&b, 0, 32, true), 3);
        assert_eq!(w31a77_bitset_find(&b, 3, 32, false), 21);
        assert_eq!(w31a77_bitset_find(&b, 21, 24, true), 24);
        assert!(w31a77_bitset_all(&b, 3, 21, true));
        assert!(!w31a77_bitset_all(&b, 2, 21, true));
        assert!(w31a77_bitset_all(&b, 5, 5, false));
        w31a77_bitset_fill(&mut b, 0, 32, false);
        assert_eq!(b, [0; 4]);
    }

    #[test]
    fn iso() {
        let tm = [5, 4, 3, 2, 0, 124];
        assert_eq!(w31a77_format_iso8601(tm, false), "2024-01-02T03:04:05Z");
        assert_eq!(w31a77_format_iso8601(tm, true), "20240102T030405Z");
    }
}
