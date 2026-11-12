#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent74.rs
// ===========================================================================
// Wave 31 translation batch 74: file-open mode mapping, small record
// initialisers and a sorted-pointer merge.

/// `0x00b459b0` (mode table): maps the open mode `mode` to
/// `(desired_access, creation_disposition, seek_to_end)`.  Modes 6 and 7
/// are append variants of 0 and 1.  Unknown modes abort in retail (`None`).
pub fn w31a74_open_mode(mode: i32) -> Option<(u32, u32, bool)> {
    Some(match mode {
        0 | 6 => (0x4000_0000, 4, mode == 6),
        1 | 7 => (0xc000_0000, 4, mode == 7),
        2 => (0x8000_0000, 3, false),
        3 => (0xc000_0000, 3, false),
        4 => (0x4000_0000, 2, false),
        5 => (0xc000_0000, 2, false),
        _ => return None,
    })
}

/// `0x00b459b0` (flags): file attribute/flag word built from the open
/// flags `flags`: base `0x80`, bit 1 shifted to `0x2000_0000`, bit 2 adds
/// `0x1000_0000`, otherwise bit 3 adds `0x0800_0000`.
pub fn w31a74_open_flags(flags: u32) -> u32 {
    let hi = (flags & 2) << 28;
    if flags & 4 != 0 {
        hi | 0x1000_0080
    } else if flags & 8 != 0 {
        hi | 0x0800_0080
    } else {
        hi | 0x80
    }
}

/// `0x00b445f0`: initialises a record: `[id, 0x150, 0x100 zero bytes]`.
pub fn w31a74_init_record(id: u32) -> (u32, u32, [u8; 0x100]) {
    (id, 0x150, [0u8; 0x100])
}

/// `0x00b468e0`: zero-fills a `0x30` byte block.
pub fn w31a74_zero_block(block: &mut [u8; 0x30]) {
    block.fill(0);
}

/// `0x00b47180`: true when the container's data pointer `data_ptr` differs
/// from its inline buffer (`inline_ptr`, at word offset 3), i.e. the
/// storage was heap allocated and must be freed.
pub fn w31a74_uses_heap(data_ptr: usize, inline_ptr: usize) -> bool {
    data_ptr != inline_ptr
}

/// `0x00b46f80`: true when the pointer range holds more than one 4-byte
/// element (`(end - begin) >> 2 > 1`), which triggers the cleanup path.
pub fn w31a74_has_multiple(begin: i32, end: i32) -> bool {
    1 < (end.wrapping_sub(begin) >> 2)
}

/// `0x00b46ce0`: merges two ascending runs of 8-byte keys.  `a` and `b`
/// are the two runs; at each step the `b` key is taken when it sorts
/// strictly before the `a` key, otherwise the `a` key.  When either run is
/// exhausted the remaining `a` entries are appended (the `b` tail is left
/// to the caller, as in retail).
pub fn w31a74_merge_keys(a: &[[u8; 8]], b: &[[u8; 8]]) -> Vec<[u8; 8]> {
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    if !a.is_empty() && !b.is_empty() {
        while i < a.len() && j < b.len() {
            if b[j] < a[i] {
                out.push(b[j]);
                j += 1;
            } else {
                out.push(a[i]);
                i += 1;
            }
        }
    }
    out.extend_from_slice(&a[i..]);
    out
}

#[cfg(test)]
mod tests_wave31_agent74 {
    use super::*;

    #[test]
    fn open_mode_and_flags() {
        assert_eq!(w31a74_open_mode(6), Some((0x4000_0000, 4, true)));
        assert_eq!(w31a74_open_mode(7), Some((0xc000_0000, 4, true)));
        assert_eq!(w31a74_open_mode(2), Some((0x8000_0000, 3, false)));
        assert_eq!(w31a74_open_mode(5), Some((0xc000_0000, 2, false)));
        assert_eq!(w31a74_open_mode(8), None);
        assert_eq!(w31a74_open_flags(0), 0x80);
        assert_eq!(w31a74_open_flags(2), 0x2000_0080);
        assert_eq!(w31a74_open_flags(4), 0x1000_0080);
        assert_eq!(w31a74_open_flags(8), 0x0800_0080);
    }

    #[test]
    fn small_helpers() {
        let (id, sz, z) = w31a74_init_record(9);
        assert_eq!((id, sz, z.len()), (9, 0x150, 0x100));
        let mut b = [1u8; 0x30];
        w31a74_zero_block(&mut b);
        assert!(b.iter().all(|&x| x == 0));
        assert!(w31a74_uses_heap(1, 2));
        assert!(!w31a74_uses_heap(2, 2));
        assert!(!w31a74_has_multiple(0, 4));
        assert!(w31a74_has_multiple(0, 8));
    }

    #[test]
    fn merge() {
        let a = [[1u8; 8], [5; 8]];
        let b = [[3u8; 8]];
        assert_eq!(w31a74_merge_keys(&a, &b), vec![[1; 8], [3; 8], [5; 8]]);
        assert_eq!(w31a74_merge_keys(&a, &[]), vec![[1; 8], [5; 8]]);
    }
}
