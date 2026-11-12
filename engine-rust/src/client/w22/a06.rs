#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent06.rs
// ===========================================================================
// Wave 22 translation batch 06 (staging).

/// `0x00687f30` (chunk planning only): the stream writer splits `size`
/// bytes into 0x2000-byte chunks; returns `(chunk_count, header_bytes,
/// chunk_lengths)` where the header is `size + (chunks + 2) * 4`.
/// Returns `None` for `size == 0` (the routine returns 0 immediately).
pub fn w22a06_plan_stream_chunks(size: u32) -> Option<(u32, u32, Vec<u32>)> {
    if size == 0 {
        return None;
    }
    let mut chunks = size >> 13;
    if size != chunks << 13 {
        chunks += 1;
    }
    let header = size.wrapping_add((chunks + 2).wrapping_mul(4));
    let mut lens = Vec::with_capacity(chunks as usize);
    for i in 0..chunks {
        lens.push((size - i * 0x2000).min(0x2000));
    }
    Some((chunks, header, lens))
}

/// Integer hash shared by `0x00689080` and `0x00689da0`: a two-round
/// multiply/xor-shift mixer (constants 0x85ebca6b, 0xc2b2ae35).
pub fn w22a06_hash_u32(key: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    h >> 16 ^ h
}

/// Unique-key hash set with bucket chains (max load factor 1.0).
#[derive(Debug, Clone, PartialEq)]
pub struct W22a06Set {
    pub bucket_count: u32,
    pub buckets: Vec<Vec<u32>>,
    pub size: u32,
    pub max_load: f32,
}

impl W22a06Set {
    fn insert_unique(&mut self, key: u32) {
        if self.buckets.is_empty() {
            return;
        }
        let idx = (w22a06_hash_u32(key) & (self.bucket_count - 1)) as usize;
        if !self.buckets[idx].contains(&key) {
            self.buckets[idx].push(key);
            self.size += 1;
        }
    }
}

/// `0x00689080`: constructs the set with `bucket_count` (power of two)
/// buckets, size 0, load factor 1.0, then inserts each key of `keys`
/// (12-byte source stride in retail; key is the first dword) skipping
/// duplicates.
pub fn w22a06_set_build(keys: &[u32], bucket_count: u32) -> W22a06Set {
    let mut s = W22a06Set {
        bucket_count,
        buckets: vec![Vec::new(); bucket_count as usize],
        size: 0,
        max_load: 1.0,
    };
    for &k in keys {
        s.insert_unique(k);
    }
    s
}

/// `0x00689da0`: assignment; no-op on self. Clears every bucket and the
/// size, then inserts each source key in bucket order, skipping duplicates.
pub fn w22a06_set_assign(dst: &mut W22a06Set, src: &W22a06Set) {
    for b in dst.buckets.iter_mut() {
        b.clear();
    }
    dst.size = 0;
    for b in &src.buckets {
        for &k in b {
            dst.insert_unique(k);
        }
    }
}

/// `0x00691a10`: picks an entry of the seven-slot (stride 0x70) name table
/// at 0xf06cc8: first entry whose name equals `wanted`; else the last entry
/// named "Default"; else entry 0.
pub fn w22a06_select_entry(names: &[&str], wanted: &str) -> usize {
    let mut default = None;
    for (i, n) in names.iter().take(7).enumerate() {
        if *n == wanted {
            return i;
        }
        if *n == "Default" {
            default = Some(i);
        }
    }
    default.unwrap_or(0)
}

#[cfg(test)]
mod tests_wave22_agent06 {
    use super::*;

    #[test]
    fn chunks() {
        assert_eq!(w22a06_plan_stream_chunks(0), None);
        let (c, h, l) = w22a06_plan_stream_chunks(0x2001).unwrap();
        assert_eq!((c, h, l), (2, 0x2001 + 16, vec![0x2000, 1]));
        assert_eq!(w22a06_plan_stream_chunks(0x2000).unwrap().0, 1);
    }

    #[test]
    fn set_build_and_assign() {
        assert_eq!(w22a06_hash_u32(0), 0);
        let s = w22a06_set_build(&[1, 2, 2, 3], 8);
        assert_eq!(s.size, 3);
        assert_eq!(s.max_load, 1.0);
        let mut d = w22a06_set_build(&[9, 10], 4);
        w22a06_set_assign(&mut d, &s);
        assert_eq!(d.size, 3);
        let before = d.clone();
        let same = d.clone();
        w22a06_set_assign(&mut d, &same);
        assert_eq!(d.size, before.size);
    }

    #[test]
    fn select() {
        let n = ["A", "Default", "B", "Default"];
        assert_eq!(w22a06_select_entry(&n, "B"), 2);
        assert_eq!(w22a06_select_entry(&n, "X"), 3);
        assert_eq!(w22a06_select_entry(&["A"], "X"), 0);
    }
}
