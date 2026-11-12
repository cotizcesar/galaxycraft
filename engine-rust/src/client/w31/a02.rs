#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent02.rs
// ===========================================================================
// Wave 31 batch 02 (client.rs helpers): hash-table iteration, substring
// search and small packers.

/// Iterator cursor of the chained hash table (`cur`, `bucket`, `end`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W31a02Cursor {
    /// Current node index (0 = none).
    pub node: u32,
    /// Current bucket slot index.
    pub bucket: usize,
    /// One past the last bucket slot.
    pub end: usize,
}

/// `0x0053b260`: advance the cursor: follow the node's next link; if none,
/// scan forward through `buckets` for the next non-empty slot.
/// `next_of` yields a node's next link.
pub fn w31a02_cursor_advance(c: &mut W31a02Cursor, buckets: &[u32], next_of: impl Fn(u32) -> u32) {
    if c.node != 0 {
        c.node = next_of(c.node);
        if c.node != 0 {
            return;
        }
    }
    c.bucket += 1;
    while c.bucket < c.end {
        c.node = buckets.get(c.bucket).copied().unwrap_or(0);
        if c.node != 0 {
            return;
        }
        c.bucket += 1;
    }
}

/// `0x0053e8a0`: hash-table find; key hash is a 32-bit finalizer
/// (murmur3-style, multipliers `0x85EBCA6B`, `0xC2B2AE35`) masked by
/// `bucket_count - 1`.  Returns `(node, bucket_slot, end_slot)`; a missing
/// key yields `node == 0` and `bucket_slot == bucket_count`.
/// `bucket_count` must be a power of two; `buckets` holds node ids,
/// `next_of`/`key_of` read a node.
pub fn w31a02_hash_find(
    bucket_count: u32,
    buckets: Option<&[u32]>,
    key: u32,
    next_of: impl Fn(u32) -> u32,
    key_of: impl Fn(u32) -> u32,
) -> (u32, u32, u32) {
    let Some(b) = buckets else {
        return (0, bucket_count << 2, bucket_count << 2);
    };
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85EB_CA6B);
    h = (h >> 13 ^ h).wrapping_mul(0xC2B2_AE35);
    h = (h >> 16 ^ h) & bucket_count.wrapping_sub(1);
    let mut node = b.get(h as usize).copied().unwrap_or(0);
    while node != 0 && key_of(node) != key {
        node = next_of(node);
    }
    let slot = if node != 0 { h } else { bucket_count };
    (node, slot, bucket_count)
}

/// `0x00549a60`: forward substring search of `needle` in `hay[..hay_len]`
/// starting at `start`; returns the match index or `u32::MAX`.  An empty
/// needle returns `start` when `start <= hay_len`.
pub fn w31a02_find_bytes(hay: &[u8], hay_len: u32, needle: &[u8], start: u32) -> u32 {
    let n = needle.len() as u32;
    if start <= hay_len {
        if n == 0 {
            return start;
        }
        let mut end = start + n;
        let mut s = start;
        while end <= hay_len {
            let from = s as usize;
            if hay.get(from..from + n as usize) == Some(needle) {
                return s;
            }
            end += 1;
            s += 1;
        }
    }
    u32::MAX
}

/// `0x0057b8b0`: packed key of a record: when `kind < 5` defers to
/// `fallback`; else `((kind << 18 | flags & 0x10) << 9) | id`.
pub fn w31a02_pack_key(kind: u8, flags: u8, id: u16, fallback: impl Fn() -> u32) -> u32 {
    if kind < 5 {
        return fallback();
    }
    ((u32::from(kind) << 18 | u32::from(flags & 0x10)) << 9) | u32::from(id)
}

/// `0x00595c20`: push-back of an element given by reference `idx` into the
/// vector `v` (aliasing-safe): skipped when `idx == end_marker`; if the
/// source lies inside the vector, its value is copied before growth.
/// `src_in_vec` is the element index when the source is inside `v`.
pub fn w31a02_push_back_alias(
    v: &mut Vec<u32>,
    is_end: bool,
    src_in_vec: Option<usize>,
    value: u32,
) {
    if is_end {
        return;
    }
    let v_copy = match src_in_vec {
        Some(i) if i < v.len() => v[i],
        _ => value,
    };
    v.push(v_copy);
}

/// `0x00597560`: begin() of a bucket table: when the table exists, the
/// first node is `first`; if zero, advance via `advance`.  Returns
/// `(node, table_ptr, end)`; with no table returns `(0, n << 2, n << 2)`.
pub fn w31a02_table_begin(
    bucket_count: u32,
    table: Option<u32>,
    first: u32,
    advance: impl FnOnce() -> u32,
) -> (u32, u32, u32) {
    match table {
        Some(t) => {
            let node = if first == 0 { advance() } else { first };
            (node, t, t.wrapping_add(bucket_count))
        }
        None => (0, bucket_count << 2, bucket_count << 2),
    }
}

/// `0x0060aa00`: lazily cached timestamp; on first use queries `query`
/// (returning the raw value, or `None`) and scales via `scale`; the result
/// (0 on failure) is cached.  Returns the cached `(lo, hi)` pair.
pub fn w31a02_cached_time(
    cache: &mut Option<u64>,
    query: impl FnOnce() -> Option<u64>,
    scale: impl FnOnce(u64) -> u64,
) -> (u32, u32) {
    let v = *cache.get_or_insert_with(|| query().map(scale).unwrap_or(0));
    (v as u32, (v >> 32) as u32)
}

/// `0x0053f840`: initialise a stream-like record `[a, b, c, 0, 0, 0x20000]`;
/// slot 3 is replaced by `probe()` when it returns `Some`.
pub fn w31a02_init_record_a(
    a: u32,
    b: u32,
    c: u32,
    probe: impl FnOnce() -> Option<u32>,
) -> [u32; 6] {
    let mut r = [a, b, c, 0, 0, 0x20000];
    if let Some(p) = probe() {
        r[3] = p;
    }
    r
}

/// `0x0053f8a0`: like the sibling initialiser but layout `[0, a, b, 0, 0,
/// 0x20000]`.
pub fn w31a02_init_record_b(a: u32, b: u32, probe: impl FnOnce() -> Option<u32>) -> [u32; 6] {
    let mut r = [0, a, b, 0, 0, 0x20000];
    if let Some(p) = probe() {
        r[3] = p;
    }
    r
}

#[cfg(test)]
mod tests_wave31_agent02 {
    use super::*;

    #[test]
    fn cursor() {
        let buckets = [0, 0, 7, 0, 9];
        let mut c = W31a02Cursor {
            node: 5,
            bucket: 0,
            end: 5,
        };
        w31a02_cursor_advance(&mut c, &buckets, |n| if n == 5 { 0 } else { 0 });
        assert_eq!((c.node, c.bucket), (7, 2));
        w31a02_cursor_advance(&mut c, &buckets, |_| 0);
        assert_eq!((c.node, c.bucket), (9, 4));
        w31a02_cursor_advance(&mut c, &buckets, |_| 0);
        assert_eq!((c.node, c.bucket), (0, 5));
    }

    #[test]
    fn find() {
        assert_eq!(w31a02_hash_find(4, None, 1, |_| 0, |_| 0), (0, 16, 16));
        let b = [1u32, 0, 0, 0];
        let r = w31a02_hash_find(4, Some(&b), 0, |_| 0, |_| 7);
        assert_eq!(r.0, 0);
        assert_eq!(r.1, 4);
        let h = w31a02_hash_find(4, Some(&[1, 1, 1, 1]), 3, |_| 0, |_| 3);
        assert_eq!(h.0, 1);
        assert!(h.1 < 4);
    }

    #[test]
    fn bytes() {
        assert_eq!(w31a02_find_bytes(b"abcabc", 6, b"ca", 0), 2);
        assert_eq!(w31a02_find_bytes(b"abcabc", 6, b"zz", 0), u32::MAX);
        assert_eq!(w31a02_find_bytes(b"abc", 3, b"", 2), 2);
        assert_eq!(w31a02_find_bytes(b"abc", 3, b"", 4), u32::MAX);
        assert_eq!(w31a02_find_bytes(b"abcabc", 6, b"abc", 1), 3);
    }

    #[test]
    fn pack() {
        assert_eq!(w31a02_pack_key(4, 0, 1, || 99), 99);
        assert_eq!(
            w31a02_pack_key(5, 0x10, 3, || 0),
            ((5 << 18 | 0x10) << 9) | 3
        );
    }

    #[test]
    fn push() {
        let mut v = vec![1, 2];
        w31a02_push_back_alias(&mut v, true, None, 5);
        assert_eq!(v.len(), 2);
        w31a02_push_back_alias(&mut v, false, Some(1), 5);
        assert_eq!(v, [1, 2, 2]);
        w31a02_push_back_alias(&mut v, false, None, 8);
        assert_eq!(v[3], 8);
    }

    #[test]
    fn begin_and_time() {
        assert_eq!(w31a02_table_begin(2, None, 0, || 1), (0, 8, 8));
        assert_eq!(w31a02_table_begin(2, Some(10), 0, || 4), (4, 10, 12));
        let mut c = None;
        assert_eq!(
            w31a02_cached_time(&mut c, || Some(3), |x| x * 1_000_000_000),
            (3_000_000_000, 0)
        );
        assert_eq!(
            w31a02_cached_time(&mut c, || None, |x| x),
            (3_000_000_000, 0)
        );
        let mut d = None;
        assert_eq!(w31a02_cached_time(&mut d, || None, |x| x), (0, 0));
    }

    #[test]
    fn records() {
        assert_eq!(
            w31a02_init_record_a(1, 2, 3, || Some(9)),
            [1, 2, 3, 9, 0, 0x20000]
        );
        assert_eq!(
            w31a02_init_record_b(1, 2, || None),
            [0, 1, 2, 0, 0, 0x20000]
        );
    }
}
