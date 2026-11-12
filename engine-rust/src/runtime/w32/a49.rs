#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent49.rs
// ===========================================================================
// Wave 32 agent 49: red-black tree in-order successor.

/// Null link marker for [`w32a49_tree_next`].
pub const W32A49_NULL: usize = usize::MAX;

/// `0x007e4760`: in-order successor of `node` in a tree whose nodes are
/// `[parent, left, right]` index triples (std::map iterator increment).
/// With a right child, descends to the leftmost node of the right subtree;
/// otherwise climbs while the node is its parent's right child, and keeps
/// the parent unless the node's right link already equals it (header case).
pub fn w32a49_tree_next(nodes: &[[usize; 3]], node: usize) -> usize {
    let right = nodes[node][2];
    if right != W32A49_NULL {
        let mut cur = right;
        while nodes[cur][1] != W32A49_NULL {
            cur = nodes[cur][1];
        }
        return cur;
    }
    let mut cur = node;
    let mut parent = nodes[cur][0];
    while cur == nodes[parent][2] {
        cur = parent;
        parent = nodes[cur][0];
    }
    if nodes[cur][2] != parent {
        cur = parent;
    }
    cur
}

#[cfg(test)]
mod tests_wave32_agent49 {
    use super::*;
    const N: usize = W32A49_NULL;

    #[test]
    fn successor() {
        // 0 = header (root at left? use right link = root), 1 root, 2 left, 3 right
        let nodes = [[N, N, 1], [0, 2, 3], [1, N, N], [1, N, N]];
        let mut nodes = nodes.to_vec();
        nodes[0] = [1, 1, 1];
        assert_eq!(w32a49_tree_next(&nodes, 2), 1);
        assert_eq!(w32a49_tree_next(&nodes, 1), 3);
        assert_eq!(w32a49_tree_next(&nodes, 3), 0);
    }
}
