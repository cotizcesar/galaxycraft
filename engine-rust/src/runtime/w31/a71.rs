#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent71.rs
// ===========================================================================
// Wave 31 batch 71: small stream/error-code helpers.

/// `0x00ad8280`: normalises an error code before it is stored in an
/// 8-word status record.  Code 0 is kept; 5, 6 and 0x16 are collapsed to 6
/// unless flag `0x400` is set; any other nonzero code first calls
/// `report_unknown` (FUN_00ad1ae0).  Returns the stored code.
pub fn w31a71_normalize_status(code: i32, flags: u32, mut report_unknown: impl FnMut()) -> i32 {
    let mut c = code;
    if c != 0 {
        match c {
            5 | 6 | 0x16 => {
                if c != 5 {
                    // 6 and 0x16 fall to the final store unchanged.
                } else if flags & 0x400 == 0 {
                    c = 6;
                }
            }
            _ => report_unknown(),
        }
    }
    c
}

/// `0x00af0790`: returns `false` only when the counter at `+0x6a` is
/// nonzero while the one at `+0x68` is zero.
pub fn w31a71_counters_ok(at_68: u16, at_6a: u16) -> bool {
    !(at_6a != 0 && at_68 == 0)
}

/// `0x00ae7340`: computes the inclusive end offset `base + len - 1` of a
/// 64-bit range, `None` when `len` is zero (the callee is not invoked).
pub fn w31a71_range_last(base: u64, len: u64) -> Option<u64> {
    if len == 0 {
        None
    } else {
        Some(base.wrapping_add(len).wrapping_sub(1))
    }
}

/// `0x00ac84e0`: reads a size byte from `table[idx]`.  Unless flag bit 2 of
/// `flags` is set and `failed()` (FUN_00ac1cc0) is true, it grows via
/// `grow` (FUN_00ac05d0) when `byte*2 <= limit`, then returns the re-read
/// byte doubled.  Returns 0 when `failed()` is true with bit 2 clear.
pub fn w31a71_doubled_size(
    flags: u8,
    mut failed: impl FnMut() -> bool,
    table: &mut [u8],
    idx: usize,
    limit: u32,
    mut grow: impl FnMut(&mut [u8]),
) -> u32 {
    if flags & 4 == 0 && failed() {
        return 0;
    }
    if (table[idx] as u32) * 2 <= limit {
        grow(table);
    }
    (table[idx] as u32) * 2
}

/// `0x00af1450`: initialises a 16-word record to zero, except words 0xc
/// and 0xd which take the two global defaults; word 1 is a single byte.
pub fn w31a71_init_record(global_c: u32, global_d: u32) -> [u32; 16] {
    let mut r = [0u32; 16];
    r[0xc] = global_c;
    r[0xd] = global_d;
    r
}

#[cfg(test)]
mod tests_wave31_agent71 {
    use super::*;

    #[test]
    fn normalize() {
        let mut n = 0;
        assert_eq!(w31a71_normalize_status(0, 0, || n += 1), 0);
        assert_eq!(w31a71_normalize_status(5, 0, || n += 1), 6);
        assert_eq!(w31a71_normalize_status(5, 0x400, || n += 1), 5);
        assert_eq!(w31a71_normalize_status(0x16, 0, || n += 1), 0x16);
        assert_eq!(n, 0);
        assert_eq!(w31a71_normalize_status(9, 0, || n += 1), 9);
        assert_eq!(n, 1);
    }

    #[test]
    fn counters() {
        assert!(!w31a71_counters_ok(0, 1));
        assert!(w31a71_counters_ok(1, 1));
        assert!(w31a71_counters_ok(0, 0));
    }

    #[test]
    fn range() {
        assert_eq!(w31a71_range_last(10, 0), None);
        assert_eq!(w31a71_range_last(0xffff_ffff, 2), Some(0x1_0000_0000));
    }

    #[test]
    fn doubled() {
        let mut t = [3u8];
        assert_eq!(
            w31a71_doubled_size(4, || true, &mut t, 0, 10, |t| t[0] = 9),
            18
        );
        let mut t = [3u8];
        assert_eq!(w31a71_doubled_size(0, || true, &mut t, 0, 10, |_| {}), 0);
        let mut t = [3u8];
        assert_eq!(w31a71_doubled_size(0, || false, &mut t, 0, 5, |_| {}), 6);
    }

    #[test]
    fn init() {
        let r = w31a71_init_record(7, 8);
        assert_eq!((r[0], r[0xc], r[0xd], r[0xf]), (0, 7, 8, 0));
    }
}
