#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent21.rs
// ===========================================================================
// Wave 25 agent 21: MSVC STL allocator / hash-table / tagged-pointer glue.

/// `0x00802c10`: allocation request size for `count` 8-byte elements.
/// Returns `None` when nothing is allocated (zero elements); big blocks
/// (>= 0x1000 bytes) carry a `0x23` alignment header, saturating at u32::MAX.
pub fn w25a21_alloc_request_size8(count: u32) -> Option<u32> {
    let mut bytes = count.wrapping_mul(8);
    if count < 0x2000_0000 {
        if bytes < 0x1000 {
            return if bytes != 0 { Some(bytes) } else { None };
        }
    } else {
        bytes = 0xffff_ffff;
    }
    let big = bytes.wrapping_add(0x23);
    Some(if big <= bytes { 0xffff_ffff } else { big })
}

/// `0x00802d20`: deallocation of `count` 8-byte elements at `ptr`; returns
/// `(block, size)` or `Err(())` when the big-block header check fails.
pub fn w25a21_dealloc_block8(ptr: u32, header: u32, count: u32) -> Result<(u32, u32), ()> {
    let mut size = count.wrapping_mul(8);
    let mut block = ptr;
    if size > 0xfff {
        block = header;
        size = size.wrapping_add(0x23);
        if ptr.wrapping_sub(block).wrapping_sub(4) > 0x1f {
            return Err(());
        }
    }
    Ok((block, size))
}

/// `0x00803460`: deallocation of an SSO string buffer with capacity `cap`;
/// `None` when the string is inline (`cap < 0x10`), else `(block, size)`.
pub fn w25a21_string_free_block(ptr: u32, header: u32, cap: u32) -> Option<Result<(u32, u32), ()>> {
    if cap < 0x10 {
        return None;
    }
    let mut size = cap + 1;
    let mut block = ptr;
    if size > 0xfff {
        block = header;
        size = cap + 0x24;
        if ptr.wrapping_sub(block).wrapping_sub(4) > 0x1f {
            return Some(Err(()));
        }
    }
    Some(Ok((block, size)))
}

/// `0x00802d60`: resolves a tagged pointer cell: bit 0 set means the real
/// pointer is `v & !1`; otherwise the cell itself.  `cell_addr` is returned
/// for the untagged case.  `None` means null (nothing to release).
pub fn w25a21_resolve_tagged(v: u32, cell_addr: u32) -> Option<u32> {
    let p = if v & 1 != 0 {
        v & 0xffff_fffe
    } else {
        cell_addr
    };
    if p != 0 { Some(p) } else { None }
}

/// `0x008037e0`: resolves the two tagged cells (at +0x34 and +0x20) of a
/// node; returns the pointer to release for each, in that order.
pub fn w25a21_node_tagged_pair(node: u32, cell34: u32, cell20: u32) -> (Option<u32>, Option<u32>) {
    (
        w25a21_resolve_tagged(cell34, node.wrapping_add(0x34)),
        w25a21_resolve_tagged(cell20, node.wrapping_add(0x20)),
    )
}

/// `0x008038d0` (hash step): 32-bit finalizer mixing used by the hash table.
pub fn w25a21_hash_mix(key: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    h >> 16 ^ h
}

/// `0x008038d0`: bucket-chain search.  `buckets` is the bucket array
/// (`None` table = empty => `None`), `nodes[i] = (next, key_lo, key_hi)`
/// with indices+1 as links (0 = end).  Returns the index of the node
/// matching `(lo, hi)`.
pub fn w25a21_hash_find(
    bucket_count: u32,
    buckets: &[u32],
    nodes: &[(u32, u32, u32)],
    lo: u32,
    hi: u32,
) -> Option<usize> {
    if buckets.is_empty() {
        return None;
    }
    let b = (w25a21_hash_mix(lo) & bucket_count.wrapping_sub(1)) as usize;
    let mut link = *buckets.get(b)?;
    while link != 0 {
        let n = nodes.get(link as usize - 1)?;
        if n.1 == lo && n.2 == hi {
            return Some(link as usize - 1);
        }
        link = n.0;
    }
    None
}

/// `0x00803b30`: after resetting the intrusive list sentinel at +0x10
/// (returns the previous head link), looks the 64-bit key up in the table
/// and swaps the owned pointer at node +0x10; returns `(new_slot, old)`.
pub fn w25a21_swap_owned(slot: &mut u32, new: &mut u32) -> u32 {
    let incoming = *new;
    *new = 0;
    let old = *slot;
    *slot = incoming;
    old
}

/// `0x00802c80`: resets a circular list head `[next, prev, count]` to the
/// empty state; returns the previous `prev` link.
pub fn w25a21_list_reset(head: &mut [u32; 3], self_addr: u32) -> u32 {
    let old = head[1];
    head[1] = self_addr;
    head[0] = self_addr;
    head[2] = 0;
    old
}

/// `0x008041b0`: index of `target` among `count` 8-byte entries (first word
/// is the pointer); returns `count` when absent.  Also the tail length in
/// bytes that gets shifted down (`(count - idx - 1) * 8`).
pub fn w25a21_find_entry(entries: &[(u32, u32)], target: u32) -> (usize, usize) {
    let idx = entries
        .iter()
        .position(|e| e.0 == target)
        .unwrap_or(entries.len());
    (idx, entries.len().saturating_sub(idx + 1) * 8)
}

/// `0x00804010`: scalar-deleting destructor flag: true when bit 0 of the
/// flags requests the memory be freed after destruction.
pub fn w25a21_deleting_dtor_frees(flags: u8) -> bool {
    flags & 1 != 0
}

/// `0x00804080`: fixed allocation size of this constructor's object.
pub fn w25a21_alloc_size_0x148() -> u32 {
    0x148
}

/// `0x00804120`: fixed allocation size of this constructor's object.
pub fn w25a21_alloc_size_0xf0() -> u32 {
    0xf0
}

/// `0x00801f90`: fixed allocation size of this constructor's object.
pub fn w25a21_alloc_size_0x18() -> u32 {
    0x18
}

/// `0x00803210`: length of a node's name: `name_ptr` zero means the inline
/// buffer is used; returns the NUL-terminated length (excluding NUL).
pub fn w25a21_node_name_len(name: &[u8]) -> usize {
    name.iter().position(|&c| c == 0).unwrap_or(name.len())
}

#[cfg(test)]
mod tests_wave25_agent21 {
    use super::*;

    #[test]
    fn alloc_sizes() {
        assert_eq!(w25a21_alloc_request_size8(0), None);
        assert_eq!(w25a21_alloc_request_size8(1), Some(8));
        assert_eq!(w25a21_alloc_request_size8(0x200), Some(0x1023));
        assert_eq!(w25a21_alloc_request_size8(0x2000_0000), Some(0xffff_ffff));
        assert_eq!(w25a21_alloc_size_0x148(), 0x148);
        assert_eq!(w25a21_alloc_size_0xf0(), 0xf0);
        assert_eq!(w25a21_alloc_size_0x18(), 0x18);
    }

    #[test]
    fn dealloc() {
        assert_eq!(w25a21_dealloc_block8(100, 0, 4), Ok((100, 32)));
        assert_eq!(w25a21_dealloc_block8(104, 100, 0x200), Ok((100, 0x1023)));
        assert_eq!(w25a21_dealloc_block8(200, 100, 0x200), Err(()));
        assert_eq!(w25a21_string_free_block(5, 0, 0xf), None);
        assert_eq!(w25a21_string_free_block(5, 0, 0x10), Some(Ok((5, 0x11))));
        assert_eq!(
            w25a21_string_free_block(104, 100, 0xfff),
            Some(Ok((100, 0x1023)))
        );
    }

    #[test]
    fn tagged() {
        assert_eq!(w25a21_resolve_tagged(1, 50), None);
        assert_eq!(w25a21_resolve_tagged(0x1001, 50), Some(0x1000));
        assert_eq!(w25a21_resolve_tagged(0, 50), Some(50));
        assert_eq!(
            w25a21_node_tagged_pair(0x100, 0, 0x201),
            (Some(0x134), Some(0x200))
        );
    }

    #[test]
    fn hash_and_list() {
        assert_eq!(w25a21_hash_mix(0), 0);
        let h = w25a21_hash_mix(7) & 3;
        let mut buckets = [0u32; 4];
        buckets[h as usize] = 1;
        let nodes = [(2u32, 9u32, 9u32), (0, 7, 8)];
        buckets[h as usize] = 1;
        // chain: node0 (key 9,9) -> node1 (7,8)
        assert_eq!(w25a21_hash_find(4, &buckets, &nodes, 7, 8), Some(1));
        assert_eq!(w25a21_hash_find(4, &buckets, &nodes, 7, 9), None);
        assert_eq!(w25a21_hash_find(4, &[], &nodes, 7, 8), None);
        let (mut slot, mut new) = (5u32, 9u32);
        assert_eq!(w25a21_swap_owned(&mut slot, &mut new), 5);
        assert_eq!((slot, new), (9, 0));
        let mut head = [1, 2, 3];
        assert_eq!(w25a21_list_reset(&mut head, 77), 2);
        assert_eq!(head, [77, 77, 0]);
    }

    #[test]
    fn misc() {
        let e = [(1, 0), (2, 0), (3, 0)];
        assert_eq!(w25a21_find_entry(&e, 2), (1, 8));
        assert_eq!(w25a21_find_entry(&e, 9), (3, 0));
        assert!(w25a21_deleting_dtor_frees(1));
        assert!(!w25a21_deleting_dtor_frees(2));
        assert_eq!(w25a21_node_name_len(b"abc\0d"), 3);
    }
}
