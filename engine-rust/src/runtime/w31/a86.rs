#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent86.rs
// ===========================================================================
/// `0x00b799b0`: walks a circular intrusive list of nodes (`next` at +4,
/// payload pointer at +8) from `start` up to `end`, returning the first
/// node whose payload has `(+0x10, +0x14) == (key_lo, key_hi)`, or `end`.
/// Nodes are modelled as `(next_index, key_lo, key_hi)` triples.
pub fn w31a86_find_node_by_key(
    nodes: &[(usize, i32, i32)],
    start: usize,
    end: usize,
    key_lo: i32,
    key_hi: i32,
) -> usize {
    let mut cur = start;
    while cur != end {
        let Some(n) = nodes.get(cur) else { return end };
        if n.1 == key_lo && n.2 == key_hi {
            return cur;
        }
        cur = n.0;
    }
    end
}

#[cfg(test)]
mod tests_wave31_agent86 {
    use super::*;

    #[test]
    fn find_node() {
        let nodes = [(1, 0, 0), (2, 5, 6), (3, 7, 8), (3, 0, 0)];
        assert_eq!(w31a86_find_node_by_key(&nodes, 0, 3, 7, 8), 2);
        assert_eq!(w31a86_find_node_by_key(&nodes, 0, 3, 9, 9), 3);
        assert_eq!(w31a86_find_node_by_key(&nodes, 3, 3, 7, 8), 3);
    }
}
