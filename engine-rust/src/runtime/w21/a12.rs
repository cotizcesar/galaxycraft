#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent12.rs
// ===========================================================================
// Wave 21 agent 12: MSVC `std::map` red-black tree iterator helper.

/// Tree node as laid out by the MSVC STL: words `[parent, left, right, color]`.
/// `usize::MAX` is the null pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W21a12Node {
    pub parent: usize,
    pub left: usize,
    pub right: usize,
    pub color: u8,
}

pub const W21A12_NIL: usize = usize::MAX;

/// `0x00597c70`: red-black tree iterator decrement (`--it`).  A red header
/// node whose grandparent is itself yields its right child (rightmost);
/// otherwise take the rightmost of the left subtree, or climb while the
/// node is a left child.
pub fn w21a12_tree_prev(nodes: &[W21a12Node], it: usize) -> usize {
    let n = nodes[it];
    if n.color == 0 && nodes[n.parent].parent == it {
        return n.right;
    }
    let mut cur = n.left;
    if cur == W21A12_NIL {
        let mut p = n.parent;
        let mut child = it;
        while child == nodes[p].left {
            child = p;
            p = nodes[p].parent;
        }
        return p;
    }
    while nodes[cur].right != W21A12_NIL {
        cur = nodes[cur].right;
    }
    cur
}

#[cfg(test)]
mod tests_wave21_agent12 {
    use super::*;

    fn nd(parent: usize, left: usize, right: usize, color: u8) -> W21a12Node {
        W21a12Node {
            parent,
            left,
            right,
            color,
        }
    }

    #[test]
    fn prev_walks_tree() {
        // 0 = header(red): parent=root(1), left=min(2), right=max(3)
        // 1 = root(black): left 2, right 3 ; 2 = leaf left ; 3 = leaf right
        let t = vec![
            nd(1, 2, 3, 0),
            nd(0, 2, 3, 1),
            nd(1, W21A12_NIL, W21A12_NIL, 0),
            nd(1, W21A12_NIL, W21A12_NIL, 0),
        ];
        assert_eq!(w21a12_tree_prev(&t, 0), 3); // end() -> rightmost
        assert_eq!(w21a12_tree_prev(&t, 1), 2); // root -> max of left
        assert_eq!(w21a12_tree_prev(&t, 3), 1); // right leaf -> parent
    }
}
