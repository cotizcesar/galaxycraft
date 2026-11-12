#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent08.rs
// ===========================================================================
// Wave 21 translation batch 08: small container / list helpers.

/// `0x005775c0`: load-factor style threshold.  Returns
/// `(count - (int)((count + bias) * 0.1)) <= limit` (unsigned compare).
/// `bias_table` is the two-entry double table in the image: index 0 for
/// `count >= 0`, index 1 for negative counts (`-(count >> 31)`).
pub fn w21a08_below_threshold(count: i32, limit: u32, bias_table: [f64; 2]) -> bool {
    let idx = if count < 0 { 1 } else { 0 };
    let scaled = ((count as f64 + bias_table[idx]) * 0.1) as i32;
    (count.wrapping_sub(scaled) as u32) <= limit
}

/// `0x005777f0`: resize a vector of 20-byte (`[u32; 5]`) records to
/// `new_len`.  Shrinking drops the tail; growing zero-fills.  Returns the
/// new capacity word (low 31 bits = capacity, top bit preserved) when the
/// retail code would reallocate, otherwise `None`.
pub fn w21a08_resize_records(
    rows: &mut Vec<[u32; 5]>,
    cap_word: u32,
    new_len: usize,
) -> Option<u32> {
    let len = rows.len();
    if new_len < len {
        rows.truncate(new_len);
        return None;
    }
    if new_len == len {
        return None;
    }
    let cap = cap_word & 0x7fff_ffff;
    let mut out = None;
    if (cap as usize) < new_len {
        let mut grown = (cap_word >> 1 & 0x3fff_ffff).wrapping_add(cap);
        if (grown as usize) < new_len {
            grown = new_len as u32;
        }
        out = Some(((cap_word ^ grown) & 0x7fff_ffff) ^ cap_word);
    }
    rows.resize(new_len, [0; 5]);
    out
}

/// `0x005789a0` (hash part): bucket index of a `u16` key in a table of
/// `bucket_count` (power of two) buckets.
pub fn w21a08_hash_u16(key: u16, bucket_count: u32) -> u32 {
    let mut h = (key as u32).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    (h >> 16 ^ h) & bucket_count.wrapping_sub(1)
}

/// `0x005789a0`: chained hash lookup.  `buckets[i]` is the first node of a
/// chain, `nodes[n]` is `(next, key)`.  Returns the node index holding
/// `key`, or `None` (retail then allocates and inserts a node).
pub fn w21a08_hash_find(
    buckets: &[Option<usize>],
    nodes: &[(Option<usize>, u16)],
    key: u16,
) -> Option<usize> {
    let b = w21a08_hash_u16(key, buckets.len() as u32) as usize;
    let mut cur = *buckets.get(b)?;
    while let Some(n) = cur {
        let (next, k) = *nodes.get(n)?;
        if k == key {
            return Some(n);
        }
        cur = next;
    }
    None
}

/// `0x0057b800`: reads the two bytes at offsets `0x30` and `0x31` of the
/// record referenced by the object (`*(this + 0x1c)`).
pub fn w21a08_read_pair(record: &[u8]) -> Option<(u8, u8)> {
    Some((*record.get(0x30)?, *record.get(0x31)?))
}

/// `0x0057b8f0`: union of the rectangles `[left, top, right, bottom]` of
/// every list node whose flag byte has bit `0x20`.  Returns `None` when no
/// node qualifies (the output is left untouched in retail).
pub fn w21a08_union_flagged(items: &[(u8, [i32; 4])]) -> Option<[i32; 4]> {
    let mut it = items.iter().filter(|(f, _)| f & 0x20 != 0);
    let mut r = it.next()?.1;
    for (_, c) in it {
        r[0] = r[0].min(c[0]);
        r[2] = r[2].max(c[2]);
        r[1] = r[1].min(c[1]);
        r[3] = r[3].max(c[3]);
    }
    Some(r)
}

/// `0x00579130`: walks the node list and invokes `f` for every node whose
/// kind byte (offset 10) is 0, 4 or 7.
pub fn w21a08_for_kinds_0_4_7(kinds: &[u8], mut f: impl FnMut(usize)) {
    for (i, &k) in kinds.iter().enumerate() {
        if k == 0 || k == 4 || k == 7 {
            f(i);
        }
    }
}

#[cfg(test)]
mod tests_wave21_agent08 {
    use super::*;

    #[test]
    fn threshold() {
        assert!(w21a08_below_threshold(100, 90, [0.0, 0.0]));
        assert!(!w21a08_below_threshold(100, 89, [0.0, 0.0]));
    }

    #[test]
    fn resize() {
        let mut v = vec![[1; 5]; 2];
        assert_eq!(w21a08_resize_records(&mut v, 4, 3), None);
        assert_eq!(v[2], [0; 5]);
        assert_eq!(w21a08_resize_records(&mut v, 3, 10), Some(10));
        assert_eq!(v.len(), 10);
        assert_eq!(w21a08_resize_records(&mut v, 10, 1), None);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn hash() {
        assert_eq!(w21a08_hash_u16(0, 16), 0);
        let b = w21a08_hash_u16(7, 8) as usize;
        let mut buckets = vec![None; 8];
        buckets[b] = Some(0);
        let nodes = [(Some(1), 3u16), (None, 7u16)];
        assert_eq!(w21a08_hash_find(&buckets, &nodes, 7), Some(1));
        assert_eq!(w21a08_hash_find(&buckets, &nodes, 9), None);
    }

    #[test]
    fn misc() {
        let mut rec = [0u8; 0x32];
        rec[0x30] = 5;
        rec[0x31] = 6;
        assert_eq!(w21a08_read_pair(&rec), Some((5, 6)));
        assert_eq!(w21a08_read_pair(&rec[..0x31]), None);
        let items = [
            (0, [0, 0, 99, 99]),
            (0x20, [5, 6, 7, 8]),
            (0x20, [1, 9, 6, 10]),
        ];
        assert_eq!(w21a08_union_flagged(&items), Some([1, 6, 7, 10]));
        assert_eq!(w21a08_union_flagged(&items[..1]), None);
        let mut hit = vec![];
        w21a08_for_kinds_0_4_7(&[0, 1, 4, 7, 8], |i| hit.push(i));
        assert_eq!(hit, [0, 2, 3]);
    }
}
