#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent29.rs
// ===========================================================================
// Wave 22 agent 29: small container / header helpers (runtime.rs window).

/// `0x006c1d50`: pre-step of a handle release; the type-0 init hook runs only
/// when the record kind (`+0x10`, 16 bit) is `0xe` and its first word is 0.
pub fn w22a29_needs_init_hook(kind: i16, first_word: i32) -> bool {
    kind == 0xe && first_word == 0
}

/// `0x006c1d80`: hash-table bucket-count target.  `size` is the element
/// count, `current` the bucket-derived count (callee result), `load` the max
/// load factor, `want` the caller minimum.  Result is a power of two >= 8.
pub fn w22a29_bucket_target(size: u32, current: u32, load: f32, want: u32) -> u32 {
    let mut n = current.wrapping_add(1);
    if (size as f64) as f32 <= (current as f64) as f32 * load {
        n = current;
    }
    if n < want {
        n = want;
    }
    let mut cap = 8u32;
    if n > 8 {
        while cap < n {
            cap = cap.wrapping_mul(2);
        }
    }
    cap
}

/// `0x006c26e0`: collects indices `i` of a `0x3c`-stride record table for which
/// `selectable(i)` holds, the record's word at `+0x24` is nonzero and
/// `probe(i)` succeeds.  `records` is the table bytes.
pub fn w22a29_collect_indices(
    records: &[u8],
    count: usize,
    mut selectable: impl FnMut(usize) -> bool,
    mut probe: impl FnMut(usize) -> bool,
) -> Vec<u32> {
    let mut out = Vec::new();
    for i in 0..count {
        if !selectable(i) {
            continue;
        }
        let off = i * 0x3c + 0x24;
        let nonzero = records.get(off..off + 4).is_some_and(|b| b != [0, 0, 0, 0]);
        if nonzero && probe(i) {
            out.push(i as u32);
        }
    }
    out
}

/// `0x006c3ff0`: true when the 8-byte header read succeeded, read some bytes,
/// and its magic is `0x494c4f46` ("FOLI").
pub fn w22a29_header_magic_ok(read_ok: bool, bytes_read: u32, header: &[u8; 8]) -> bool {
    read_ok
        && bytes_read != 0
        && u32::from_le_bytes([header[0], header[1], header[2], header[3]]) == 0x494c4f46
}

/// `0x006c43b0`: builds the 8-byte header written before a record
/// (magic `0x494c4f46`, version 1).
pub fn w22a29_build_header() -> [u8; 8] {
    let mut h = [0u8; 8];
    h[..4].copy_from_slice(&0x494c4f46u32.to_le_bytes());
    h[4..].copy_from_slice(&1u32.to_le_bytes());
    h
}

/// `0x006c4790`: resizes a vector of 16-byte elements to `new_len`.  Shrinking
/// drops the tail; growing zero-fills (the retail call to the element ctor is
/// reported through `on_new`).
pub fn w22a29_resize16(v: &mut Vec<[u32; 4]>, new_len: usize, mut on_new: impl FnMut(usize)) {
    if new_len < v.len() {
        v.truncate(new_len);
    } else {
        for i in v.len()..new_len {
            v.push([0; 4]);
            on_new(i);
        }
    }
}

/// `0x006c4910`: fires only for a slot kind in `1..=4`.
pub fn w22a29_kind_in_1_to_4(kind: u8) -> bool {
    kind.wrapping_sub(1) < 4
}

/// `0x006c4ac0`: same `1..=4` kind test as the sibling predicate.
pub fn w22a29_kind_in_1_to_4_b(kind: u8) -> bool {
    kind.wrapping_sub(1) < 4
}

/// `0x006c4d80`: stores the 8-byte value then increments each of its 8 bytes
/// (wrapping), returning the stored array.
pub fn w22a29_store_and_bump(value: [u8; 8]) -> [u8; 8] {
    let mut out = value;
    for b in out.iter_mut() {
        *b = b.wrapping_add(1);
    }
    out
}

/// `0x006c5860`: scans 12 slots (stride 36 bytes, id at +0, kind byte at +6
/// relative to the id) and returns the ids with `id != -1 && kind < 4`.
pub fn w22a29_active_slots(ids: &[i32; 12], kinds: &[u8; 12]) -> Vec<i32> {
    ids.iter()
        .zip(kinds)
        .filter(|(id, k)| **id != -1 && **k < 4)
        .map(|(id, _)| *id)
        .collect()
}

#[cfg(test)]
mod tests_wave22_agent29 {
    use super::*;

    #[test]
    fn hook_and_kinds() {
        assert!(w22a29_needs_init_hook(0xe, 0));
        assert!(!w22a29_needs_init_hook(0xe, 1));
        assert!(!w22a29_kind_in_1_to_4(0));
        assert!(w22a29_kind_in_1_to_4(4));
        assert!(!w22a29_kind_in_1_to_4_b(5));
        assert!(w22a29_kind_in_1_to_4_b(1));
    }

    #[test]
    fn buckets() {
        assert_eq!(w22a29_bucket_target(0, 0, 1.0, 0), 8);
        assert_eq!(w22a29_bucket_target(100, 10, 1.0, 0), 16);
        assert_eq!(w22a29_bucket_target(5, 10, 1.0, 40), 64);
    }

    #[test]
    fn collect() {
        let mut rec = vec![0u8; 0x3c * 3];
        rec[0x24] = 1;
        rec[0x3c * 2 + 0x24] = 1;
        let v = w22a29_collect_indices(&rec, 3, |i| i != 2, |_| true);
        assert_eq!(v, vec![0]);
    }

    #[test]
    fn header() {
        let h = w22a29_build_header();
        assert!(w22a29_header_magic_ok(true, 8, &h));
        assert!(!w22a29_header_magic_ok(true, 0, &h));
        assert!(!w22a29_header_magic_ok(false, 8, &h));
        assert_eq!(h[4], 1);
    }

    #[test]
    fn resize_bump_slots() {
        let mut v = vec![[1; 4]; 2];
        let mut n = 0;
        w22a29_resize16(&mut v, 4, |_| n += 1);
        assert_eq!((v.len(), n, v[3]), (4, 2, [0; 4]));
        w22a29_resize16(&mut v, 1, |_| {});
        assert_eq!(v.len(), 1);
        assert_eq!(w22a29_store_and_bump([255, 0, 1, 2, 3, 4, 5, 6])[0], 0);
        let mut ids = [-1; 12];
        ids[3] = 7;
        ids[4] = 9;
        let mut k = [0u8; 12];
        k[4] = 4;
        assert_eq!(w22a29_active_slots(&ids, &k), vec![7]);
    }
}
