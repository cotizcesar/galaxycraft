#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave27/agent08.rs
// ===========================================================================
// Wave 27 translation batch 08: rapidjson schema-validator helpers.

/// Result of the schema-stack push helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W27a08Push {
    /// New stack top offset.
    pub new_top: u32,
    /// Tag stored at `top - 2`.
    pub tag: u16,
    /// Bytes copied into a fresh allocation (0 means three slots are zeroed).
    pub copy_len: u32,
}

fn w27a08_push(top: u32, count: u32, stride: u32, tag: u16) -> W27a08Push {
    let len = count.wrapping_mul(stride);
    W27a08Push {
        new_top: top.wrapping_sub(len),
        tag,
        copy_len: len,
    }
}

/// `0x0096e360`: pushes `count` 16-byte records, frame tag 4.
pub fn w27a08_push_tag4(top: u32, count: u32) -> W27a08Push {
    w27a08_push(top, count, 0x10, 4)
}

/// `0x0096e550`: pushes `count` 32-byte records, frame tag 3.
pub fn w27a08_push_tag3(top: u32, count: u32) -> W27a08Push {
    w27a08_push(top, count, 0x20, 3)
}

/// `0x0096db80`: new 0x18-byte stack descriptor `[allocator, 0, 0, 0, 0, 0x100]`;
/// `None` when the allocator handle is null (bad-alloc path).
pub fn w27a08_new_stack(allocator: u32) -> Option<[u32; 6]> {
    if allocator == 0 {
        return None;
    }
    Some([allocator, 0, 0, 0, 0, 0x100])
}

/// `0x0096dfc0`: initial field values of the 0x54-byte schema validator
/// (21 dwords; vftable slots 0 and 1 are passed in).  `None` on null allocator.
pub fn w27a08_init_validator(
    vft0: u32,
    vft1: u32,
    allocator: u32,
    doc: u32,
    param: u32,
    stack_buf: u32,
) -> Option<[u32; 21]> {
    if allocator == 0 {
        return None;
    }
    let mut v = [0u32; 21];
    v[0] = vft0;
    v[1] = vft1;
    v[2] = doc;
    v[3] = param;
    v[4] = allocator;
    v[6] = allocator;
    v[0xb] = 0x400;
    v[0xc] = allocator;
    v[0x11] = 0x100;
    v[0x12] = stack_buf;
    v[0x13] = stack_buf;
    v[0x14] = 1; // low byte flag
    Some(v)
}

/// `0x0096d9b0`: range `(start_byte, len_bytes)` of 16-byte records copied to
/// the end of the buffer: `n = total - last`, start `last * 16`.
pub fn w27a08_tail_copy_range(total: i32, last: i32) -> (i64, i64) {
    let n = total.wrapping_sub(last);
    (last as i64 * 16, n as i64 * 16)
}

/// `0x0096da90`: clones a (12-byte entry table + data blob) pair, reserving
/// `extra_entries` entries and `extra_bytes` bytes.  Entry word 0 is an
/// offset into the data blob and is rebased by `dst_data - src_data`; entry
/// word 1 is the entry's data size.  Returns `(entries, data, total_alloc)`.
/// `None` when the allocator is null.
pub fn w27a08_clone_table(
    has_alloc: bool,
    src_entries: &[[u32; 3]],
    src_data: &[u8],
    src_data_base: u32,
    dst_data_base: u32,
    extra_entries: u32,
    extra_bytes: u32,
) -> Option<(Vec<[u32; 3]>, Vec<u8>, u32)> {
    if !has_alloc {
        return None;
    }
    let count = src_entries.len() as u32;
    let sizes = src_entries.iter().fold(count, |a, e| a.wrapping_add(e[1]));
    let n_entries = extra_entries.wrapping_add(count);
    let total = sizes
        .wrapping_add(n_entries.wrapping_mul(12))
        .wrapping_add(extra_bytes);
    let delta = dst_data_base.wrapping_sub(src_data_base);
    let entries = src_entries
        .iter()
        .map(|e| [e[0].wrapping_add(delta), e[1], e[2]])
        .collect();
    let take = (sizes as usize).min(src_data.len());
    Some((entries, src_data[..take].to_vec(), total))
}

#[cfg(test)]
mod tests_wave27_agent08 {
    use super::*;

    #[test]
    fn push_helpers() {
        let p = w27a08_push_tag4(0x100, 2);
        assert_eq!((p.new_top, p.tag, p.copy_len), (0xe0, 4, 0x20));
        let q = w27a08_push_tag3(0x100, 0);
        assert_eq!((q.new_top, q.tag, q.copy_len), (0x100, 3, 0));
        assert_eq!(w27a08_push_tag3(0x100, 1).new_top, 0xe0);
    }

    #[test]
    fn stack_and_validator() {
        assert_eq!(w27a08_new_stack(0), None);
        assert_eq!(w27a08_new_stack(7), Some([7, 0, 0, 0, 0, 0x100]));
        assert!(w27a08_init_validator(1, 2, 0, 3, 4, 5).is_none());
        let v = w27a08_init_validator(1, 2, 9, 3, 4, 5).unwrap();
        assert_eq!(
            (v[2], v[3], v[4], v[6], v[0xb], v[0xc], v[0x11]),
            (3, 4, 9, 9, 0x400, 9, 0x100)
        );
        assert_eq!((v[0x12], v[0x13], v[0x14]), (5, 5, 1));
    }

    #[test]
    fn tail_copy() {
        assert_eq!(w27a08_tail_copy_range(10, 4), (64, 96));
        assert_eq!(w27a08_tail_copy_range(4, 4), (64, 0));
    }

    #[test]
    fn clone_table() {
        assert!(w27a08_clone_table(false, &[], &[], 0, 0, 0, 0).is_none());
        let e = [[100, 3, 0], [103, 2, 0]];
        let (ents, data, total) =
            w27a08_clone_table(true, &e, &[1, 2, 3, 4, 5, 6, 7, 8], 100, 500, 1, 4).unwrap();
        assert_eq!(ents[0][0], 500);
        assert_eq!(ents[1][0], 503);
        assert_eq!(data.len(), 7);
        assert_eq!(total, 7 + 3 * 12 + 4);
    }
}
