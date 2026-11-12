#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent01.rs
// ===========================================================================
/// `0x006df820` (counter part): the frame-tick countdown.  A zero counter
/// reloads to 100, then decrements; returns `(new_counter, fired)` where
/// `fired` is true when the counter reached zero (the original then runs
/// two follow-up calls).
pub fn w23a01_tick_countdown(counter: u32) -> (u32, bool) {
    let c = if counter == 0 { 100 } else { counter };
    let c = c.wrapping_sub(1);
    (c, c == 0)
}

/// `0x006e5350`: bucket hash used by the id hash-table.  `bucket_count`
/// must be a power of two.
pub fn w23a01_hash_bucket(key: u32, bucket_count: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    (h >> 16 ^ h) & bucket_count.wrapping_sub(1)
}

/// `0x006e5350`: lookup in a chained table.  Nodes are `(next, key)`
/// indices into `nodes`; `buckets[i]` is the head node index.  Returns the
/// node index (payload lives at node offset 8) or `None` (original aborts).
pub fn w23a01_hash_find(
    buckets: &[Option<usize>],
    nodes: &[(Option<usize>, u32)],
    key: u32,
) -> Option<usize> {
    if buckets.is_empty() {
        return None;
    }
    let b = w23a01_hash_bucket(key, buckets.len() as u32) as usize;
    let mut cur = *buckets.get(b)?;
    while let Some(i) = cur {
        let (next, k) = *nodes.get(i)?;
        if k == key {
            return Some(i);
        }
        cur = next;
    }
    None
}

/// `0x006e7cb0`: true when the mode byte equals 3.
pub fn w23a01_mode_is_3(mode: u8) -> bool {
    mode == 3
}

/// `0x006e7dd0`: true when the mode byte equals 6.
pub fn w23a01_mode_is_6(mode: u8) -> bool {
    mode == 6
}

/// `0x006e7df0`: true when the state byte equals 0x0f.
pub fn w23a01_state_is_0f(state: u8) -> bool {
    state == 0x0f
}

/// `0x006e7d60`: true when bytes at +0x75, +0x79, +0x7f of the record are
/// all zero.  `rec` is the caller-chosen record (default global if `None`
/// in the original).
pub fn w23a01_flags_clear(rec: &[u8]) -> bool {
    rec.get(0x75) == Some(&0) && rec.get(0x79) == Some(&0) && rec.get(0x7f) == Some(&0)
}

/// `0x006e80b0`: validates a loaded buffer made of `0x960`-byte records;
/// `check` is the per-record validator (stops at first failure).  Returns
/// false when the load failed (`None`), else true.  Second value is the
/// number of records accepted before stopping.
pub fn w23a01_validate_records(
    buf: Option<&[u8]>,
    mut check: impl FnMut(&[u8]) -> bool,
) -> (bool, usize) {
    let Some(buf) = buf else { return (false, 0) };
    let mut n = 0;
    for rec in buf.chunks_exact(0x960) {
        if !check(rec) {
            break;
        }
        n += 1;
    }
    (true, n)
}

#[cfg(test)]
mod tests_wave23_agent01 {
    use super::*;

    #[test]
    fn countdown() {
        assert_eq!(w23a01_tick_countdown(0), (99, false));
        assert_eq!(w23a01_tick_countdown(1), (0, true));
        assert_eq!(w23a01_tick_countdown(5), (4, false));
    }

    #[test]
    fn hash_lookup() {
        assert!(w23a01_hash_bucket(0x1234_5678, 16) < 16);
        let b = w23a01_hash_bucket(7, 4) as usize;
        let mut buckets = vec![None; 4];
        buckets[b] = Some(0);
        let nodes = [(Some(1), 3u32), (None, 7)];
        assert_eq!(w23a01_hash_find(&buckets, &nodes, 7), Some(1));
        assert_eq!(w23a01_hash_find(&buckets, &nodes, 9), None);
        assert_eq!(w23a01_hash_find(&[], &nodes, 7), None);
    }

    #[test]
    fn predicates() {
        assert!(w23a01_mode_is_3(3) && !w23a01_mode_is_3(6));
        assert!(w23a01_mode_is_6(6) && !w23a01_mode_is_6(3));
        assert!(w23a01_state_is_0f(0x0f) && !w23a01_state_is_0f(0));
        let mut r = [0u8; 0x80];
        assert!(w23a01_flags_clear(&r));
        r[0x79] = 1;
        assert!(!w23a01_flags_clear(&r));
        assert!(!w23a01_flags_clear(&[0; 4]));
    }

    #[test]
    fn records() {
        assert_eq!(w23a01_validate_records(None, |_| true), (false, 0));
        let buf = vec![0u8; 0x960 * 3 + 5];
        assert_eq!(w23a01_validate_records(Some(&buf), |_| true), (true, 3));
        let mut k = 0;
        assert_eq!(
            w23a01_validate_records(Some(&buf), |_| {
                k += 1;
                k < 2
            }),
            (true, 1)
        );
    }
}
