#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent47.rs
// ===========================================================================
// Wave 25 agent 47 (runtime helpers).

/// Hash-table chain node used by `0x00897b30` (key at +8, value pair at +0x10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W25a47Node {
    pub next: Option<usize>,
    pub key: u32,
    pub value: [u32; 2],
}

/// Murmur3-style finaliser plus bucket mask used by `0x00897b30`.
pub fn w25a47_bucket_index(key: u32, bucket_count: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    (h >> 16 ^ h) & bucket_count.wrapping_sub(1)
}

/// `0x00897b30`: looks the (post-incremented) counter key up in the bucket
/// chain and stores the 8-byte value into the node.  Returns the updated
/// node index, or `None` when the table is empty or the key is missing
/// (the retail code aborts in both cases).
pub fn w25a47_update_entry(
    buckets: &[Option<usize>],
    nodes: &mut [W25a47Node],
    key: u32,
    value: [u32; 2],
) -> Option<usize> {
    if buckets.is_empty() {
        return None;
    }
    let idx = w25a47_bucket_index(key, buckets.len() as u32) as usize;
    let mut cur = *buckets.get(idx)?;
    while let Some(n) = cur {
        if nodes[n].key == key {
            nodes[n].value = value;
            return Some(n);
        }
        cur = nodes[n].next;
    }
    None
}

/// `0x00899b00`: resets a circular list sentinel (`next`/`prev` to itself,
/// size 0).  Returns true when the old `prev` link was not the sentinel,
/// i.e. nodes had to be released.
pub fn w25a47_reset_sentinel(links: &mut [u32; 3], self_id: u32) -> bool {
    let old = links[1];
    links[1] = self_id;
    links[0] = self_id;
    links[2] = 0;
    old != self_id
}

/// `0x00898710`: scalar deleting destructor wrapper; runs the destructor and
/// reports whether the memory must be freed (bit 0 of the flags).
pub fn w25a47_deleting_dtor(flags: u8, destroy: impl FnOnce()) -> bool {
    destroy();
    flags & 1 != 0
}

/// `0x0089ac90`: same scalar deleting destructor shape as above.
pub fn w25a47_deleting_dtor_b(flags: u8, destroy: impl FnOnce()) -> bool {
    destroy();
    flags & 1 != 0
}

#[cfg(test)]
mod tests_wave25_agent47 {
    use super::*;

    #[test]
    fn update_entry() {
        let key = 5u32;
        let i = w25a47_bucket_index(key, 8) as usize;
        let mut buckets = vec![None; 8];
        buckets[i] = Some(0);
        let mut nodes = vec![W25a47Node {
            next: None,
            key,
            value: [0, 0],
        }];
        assert_eq!(
            w25a47_update_entry(&buckets, &mut nodes, key, [1, 2]),
            Some(0)
        );
        assert_eq!(nodes[0].value, [1, 2]);
        assert_eq!(w25a47_update_entry(&buckets, &mut nodes, 6, [3, 4]), None);
        assert_eq!(w25a47_update_entry(&[], &mut nodes, 5, [0, 0]), None);
    }

    #[test]
    fn sentinel_and_dtor() {
        let mut l = [9, 9, 9];
        assert!(w25a47_reset_sentinel(&mut l, 7));
        assert_eq!(l, [7, 7, 0]);
        assert!(!w25a47_reset_sentinel(&mut l, 7));
        let mut n = 0;
        assert!(w25a47_deleting_dtor(1, || n += 1));
        assert!(!w25a47_deleting_dtor_b(2, || n += 1));
        assert_eq!(n, 2);
    }
}
