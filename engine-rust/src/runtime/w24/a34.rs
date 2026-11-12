#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent34.rs
// ===========================================================================
// Wave 24 agent 34: node pool bucket/hash insert and index helpers.

/// Node of the 0x14-byte pool (links are pool indices, `None` = null).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W24a34Node {
    pub next: Option<usize>,
    pub prev: Option<usize>,
    pub hash_next: Option<usize>,
    pub hash_prev: Option<usize>,
    pub x: u8,
    pub y: u8,
    pub bucket: u8,
}

/// Bucket lists (9 heads + counts) and 1024-entry position hash.
#[derive(Clone, Debug)]
pub struct W24a34Lists {
    pub heads: [Option<usize>; 9],
    pub counts: [u32; 9],
    pub hash: Vec<Option<usize>>,
}

impl Default for W24a34Lists {
    fn default() -> Self {
        Self {
            heads: [None; 9],
            counts: [0; 9],
            hash: vec![None; 0x400],
        }
    }
}

/// `0x0079eee0`: clears the nine bucket heads and the 0x1000-byte hash table.
/// (Counters are not touched by the original.)
pub fn w24a34_reset_lists(l: &mut W24a34Lists) {
    l.heads = [None; 9];
    for h in l.hash.iter_mut() {
        *h = None;
    }
}

/// `0x0079e880`: pushes node `cur` at `(x, y)` onto its bucket list and the
/// position hash `(y * 0x11 + x) & 0x3ff`, then sets flag `0x10000000` in the
/// map grid cell `y * stride + x`.  `enabled` is the global gate (returns
/// false if clear), `unlink` is the callee that detaches the node first and
/// `bucket_of` the bucket classifier.
#[allow(clippy::too_many_arguments)]
pub fn w24a34_insert_node(
    l: &mut W24a34Lists,
    nodes: &mut [W24a34Node],
    cur: usize,
    x: i32,
    y: i32,
    enabled: bool,
    stride: u16,
    grid: &mut [u32],
    mut unlink: impl FnMut(&mut W24a34Lists, &mut [W24a34Node], usize),
    bucket_of: impl Fn(i32, i32) -> u8,
) -> bool {
    if !enabled {
        return false;
    }
    unlink(l, nodes, cur);
    let b = bucket_of(x, y);
    nodes[cur].x = x as u8;
    nodes[cur].y = y as u8;
    nodes[cur].bucket = b;
    let bi = b as usize;
    l.counts[bi] += 1;
    let old = l.heads[bi];
    nodes[cur].next = old;
    nodes[cur].prev = None;
    if let Some(o) = old {
        nodes[o].prev = Some(cur);
    }
    l.heads[bi] = Some(cur);
    let hi = ((nodes[cur].y as usize) * 0x11 + nodes[cur].x as usize) & 0x3ff;
    let old = l.hash[hi];
    nodes[cur].hash_next = old;
    nodes[cur].hash_prev = None;
    if let Some(o) = old {
        nodes[o].hash_prev = Some(cur);
    }
    l.hash[hi] = Some(cur);
    let gi = (stride as i32 * y + x) as usize;
    grid[gi] |= 0x1000_0000;
    true
}

/// `0x0079f800`: the pointer-to-index conversion used by the save routine:
/// a node address `p` (pool base `0x011b25a0`, stride 0x14) becomes a 1-based
/// index, and null becomes 0.
pub fn w24a34_node_index(p: u32) -> i32 {
    if p == 0 {
        0
    } else {
        (p.wrapping_sub(0x011b_25a0) as i32) / 0x14 + 1
    }
}

/// `0x007a01e0`: wrap of the cycling counter.  `wide` selects the 0x66 range
/// (flag byte nonzero), else 0x50.  Past the end it wraps to 1, at zero it
/// goes to the last value.
pub fn w24a34_cycle_counter(cur: i32, delta: i32, wide: bool) -> i32 {
    let v = cur.wrapping_add(delta);
    let limit = if wide { 0x67 } else { 0x50 };
    if v < limit {
        if v == 0 { limit - 1 } else { v }
    } else {
        1
    }
}

#[cfg(test)]
mod tests_wave24_agent34 {
    use super::*;

    #[test]
    fn reset_and_insert() {
        let mut l = W24a34Lists::default();
        let mut nodes = vec![W24a34Node::default(); 3];
        let mut grid = vec![0u32; 100];
        let ok = w24a34_insert_node(
            &mut l,
            &mut nodes,
            0,
            3,
            2,
            true,
            10,
            &mut grid,
            |_, _, _| {},
            |_, _| 4,
        );
        assert!(ok);
        w24a34_insert_node(
            &mut l,
            &mut nodes,
            1,
            5,
            5,
            true,
            10,
            &mut grid,
            |_, _, _| {},
            |_, _| 4,
        );
        assert_eq!(l.heads[4], Some(1));
        assert_eq!(nodes[1].next, Some(0));
        assert_eq!(nodes[0].prev, Some(1));
        assert_eq!(l.counts[4], 2);
        assert_eq!(l.hash[(2 * 0x11 + 3) & 0x3ff], Some(0));
        assert_eq!(grid[23], 0x1000_0000);
        assert!(!w24a34_insert_node(
            &mut l,
            &mut nodes,
            2,
            0,
            0,
            false,
            10,
            &mut grid,
            |_, _, _| {},
            |_, _| 0
        ));
        w24a34_reset_lists(&mut l);
        assert!(l.heads.iter().all(|h| h.is_none()) && l.hash.iter().all(|h| h.is_none()));
        assert_eq!(l.counts[4], 2);
    }

    #[test]
    fn index_and_cycle() {
        assert_eq!(w24a34_node_index(0), 0);
        assert_eq!(w24a34_node_index(0x011b_25a0), 1);
        assert_eq!(w24a34_node_index(0x011b_25a0 + 0x28), 3);
        assert_eq!(w24a34_cycle_counter(0x4f, 1, false), 1);
        assert_eq!(w24a34_cycle_counter(1, -1, false), 0x4f);
        assert_eq!(w24a34_cycle_counter(1, -1, true), 0x66);
        assert_eq!(w24a34_cycle_counter(0x66, 1, true), 1);
        assert_eq!(w24a34_cycle_counter(0x66, 0, true), 0x66);
    }
}
