#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent19.rs
// ===========================================================================
// Wave 22 agent 19 (runtime container helpers).

/// `0x006907f0`: initialises a circular list sentinel node: both links point
/// to the node itself (`node[0] = node[1] = self_id`) and the size slot
/// (`node[2]`) is zeroed.  The trailing allocation call is not modelled.
pub fn w22a19_list_sentinel_init(node: &mut [u32; 3], self_id: u32) {
    node[0] = self_id;
    node[1] = self_id;
    node[2] = 0;
}

/// `0x00695640`: push_back of a 12-byte element into a vector laid out as
/// `{ptr, len, cap_flags}` (capacity is `cap_flags & 0x7fffffff`).  `src`
/// is the byte offset of the argument relative to the buffer start divided
/// by 12 (computed by the original so an element aliasing the buffer is
/// copied from its pre-growth slot).  Returns `false` when the vector is
/// full (the original takes the reallocation path, not modelled here).
pub fn w22a19_vec12_push_back(
    data: &mut Vec<[u32; 3]>,
    len: &mut u32,
    cap_flags: u32,
    src: u32,
    value: [u32; 3],
) -> bool {
    let cap = cap_flags & 0x7fff_ffff;
    if *len == cap {
        return false;
    }
    let item = if src < *len {
        data[src as usize]
    } else {
        value
    };
    let at = *len as usize;
    if data.len() <= at {
        data.resize(at + 1, [0; 3]);
    }
    data[at] = item;
    *len += 1;
    true
}

/// Growth size used on the full path of `0x00695640`: half of the current
/// capacity (bits 1..30 of `cap_flags`), at least 1.
pub fn w22a19_vec12_grow_step(cap_flags: u32) -> u32 {
    let _cap = cap_flags & 0x7fff_ffff;
    let half = (cap_flags >> 1) & 0x3fff_ffff;
    if half > 0 { half } else { 1 }
}

#[cfg(test)]
mod tests_wave22_agent19 {
    use super::*;

    #[test]
    fn sentinel() {
        let mut n = [9, 9, 9];
        w22a19_list_sentinel_init(&mut n, 0x10);
        assert_eq!(n, [0x10, 0x10, 0]);
    }

    #[test]
    fn push_back_paths() {
        let mut d = vec![[1, 2, 3], [4, 5, 6]];
        let mut len = 2;
        assert!(!w22a19_vec12_push_back(&mut d, &mut len, 2, 99, [0; 3]));
        d.reserve(4);
        assert!(w22a19_vec12_push_back(&mut d, &mut len, 4, 1, [7, 7, 7]));
        assert_eq!(d[2], [4, 5, 6]);
        assert!(w22a19_vec12_push_back(&mut d, &mut len, 4, 50, [7, 8, 9]));
        assert_eq!(d[3], [7, 8, 9]);
        assert_eq!(len, 4);
    }

    #[test]
    fn grow_step() {
        assert_eq!(w22a19_vec12_grow_step(0), 1);
        assert_eq!(w22a19_vec12_grow_step(8), 4);
    }
}
