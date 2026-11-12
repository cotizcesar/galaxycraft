#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave30/agent13.rs
// ===========================================================================
// Wave 30 agent 13: tact container helpers (reference translations).

/// `0x00b223a0`: bounds check for appending `len` bytes to a 64 KiB staging
/// buffer whose 64-bit fill counter lives at `+0x20018/+0x2001c`.  Returns
/// true (and copies) when `fill + len <= 0x10000` with no 64-bit carry.
pub fn w30a13_buffer_append(buf: &mut [u8], fill: &mut u64, src: &[u8]) -> bool {
    let len = src.len() as u64;
    match fill.checked_add(len) {
        Some(end) if end <= 0x10000 && (end as usize) <= buf.len() => {
            buf[*fill as usize..end as usize].copy_from_slice(src);
            *fill = end;
            true
        }
        _ => false,
    }
}

/// `0x00b24160`: small-vector reserve policy.  The capacity word keeps a
/// flag in bit 31; when the requested count exceeds the capacity the new
/// capacity is `max(cap + cap/2, wanted)`.  Returns the updated word, or
/// `None` when no growth is needed.
pub fn w30a13_reserve_policy(word: u32, wanted: u32) -> Option<u32> {
    let cap = word & 0x7fff_ffff;
    if cap >= wanted {
        return None;
    }
    let grown = ((word >> 1) & 0x3fff_ffff) + cap;
    let new_cap = grown.max(wanted);
    Some(((word ^ new_cap) & 0x7fff_ffff) ^ word)
}

/// `0x00b24010`: hash-table bucket count policy.  `count` is the stored
/// element count, `cur` the value returned by the sizing helper, `load` the
/// max load factor.  Result is the power of two (min 8) that holds the
/// request, and whether the table already has that size (no rehash).
pub fn w30a13_bucket_target(
    count: i32,
    cur: u32,
    load: f32,
    request: u32,
    buckets: u32,
    has_table: bool,
) -> (u32, bool) {
    let cnt = count as u32 as f32;
    let mut want = cur.wrapping_add(1);
    if cnt <= (cur as f32) * load {
        want = cur;
    }
    if want < request {
        want = request;
    }
    let mut size = 8u32;
    if want > 8 {
        while size < want {
            size = size.wrapping_mul(2);
        }
    }
    (size, size == buckets && has_table)
}

/// `0x00b22d80`: linear search of 16-byte records `(lo, hi, extra, ptr)`
/// for a 64-bit key; on a hit the first three words are copied out.
pub fn w30a13_find_record(rows: &[[u32; 4]], lo: u32, hi: u32) -> Option<[u32; 3]> {
    rows.iter()
        .find(|r| r[0] == lo && r[1] == hi)
        .map(|r| [r[0], r[1], r[2]])
}

#[cfg(test)]
mod tests_wave30_agent13 {
    use super::*;

    #[test]
    fn append() {
        let mut b = vec![0u8; 0x10000];
        let mut f = 0xfffe;
        assert!(!w30a13_buffer_append(&mut b, &mut f, &[1, 2, 3]));
        assert!(w30a13_buffer_append(&mut b, &mut f, &[1, 2]));
        assert_eq!(f, 0x10000);
        assert_eq!(b[0xffff], 2);
    }

    #[test]
    fn reserve() {
        assert_eq!(w30a13_reserve_policy(10, 5), None);
        assert_eq!(w30a13_reserve_policy(10, 11), Some(15));
        assert_eq!(w30a13_reserve_policy(10, 40), Some(40));
        assert_eq!(w30a13_reserve_policy(0x8000_0004, 100), Some(0x8000_0064));
    }

    #[test]
    fn buckets() {
        assert_eq!(w30a13_bucket_target(1, 4, 1.0, 0, 8, true), (8, true));
        assert_eq!(w30a13_bucket_target(9, 8, 1.0, 0, 8, true), (16, false));
        assert_eq!(w30a13_bucket_target(1, 4, 1.0, 100, 8, false), (128, false));
    }

    #[test]
    fn find() {
        let rows = [[1, 2, 3, 0], [4, 5, 6, 0]];
        assert_eq!(w30a13_find_record(&rows, 4, 5), Some([4, 5, 6]));
        assert_eq!(w30a13_find_record(&rows, 4, 6), None);
    }
}
