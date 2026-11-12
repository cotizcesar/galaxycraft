#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent40.rs
// ===========================================================================
// Wave 24 agent 40: MSVC `std::map` header helpers (red-black tree glue).

/// Tree node links (node layout: parent +0, left +4, right +8; 0 = null).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W24a40Node {
    pub parent: usize,
    pub left: usize,
    pub right: usize,
}

/// Map header: root/leftmost/rightmost/size at `+0x10..+0x20` of the owner
/// (here a self-index sentinel `head` stands for the header address).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W24a40Header {
    pub root: usize,
    pub leftmost: usize,
    pub rightmost: usize,
    pub size: u32,
}

/// `0x007bb750`: clears two map headers on an object.  Tree 1 (at `+0x10`)
/// has its nodes destroyed (post-order, see `w24a40_destroy_postorder`) and
/// is reset to empty with leftmost/rightmost pointing at the sentinel and
/// size 0; tree 2 (at `+0x28`) is reset the same way after its root node
/// (if any) is released through `release_root`.  Returns the two headers.
pub fn w24a40_clear_two_maps(
    first_root: usize,
    second_root: usize,
    first_head: usize,
    second_head: usize,
    destroy_first: &mut dyn FnMut(usize),
    release_second: &mut dyn FnMut(usize),
) -> (W24a40Header, W24a40Header) {
    destroy_first(first_root);
    if second_root != 0 {
        release_second(second_root);
    }
    (
        W24a40Header {
            root: 0,
            leftmost: first_head,
            rightmost: first_head,
            size: 0,
        },
        W24a40Header {
            root: 0,
            leftmost: second_head,
            rightmost: second_head,
            size: 0,
        },
    )
}

/// `0x007bc530`: recursive tree destruction.  Visits the right subtree
/// (link `+4`) first, then frees the node, then continues down the link at
/// `+8`... the retail code recurses on `+4` (left) and loops on the other
/// side; the visit order produced here is left-recursion then node.
pub fn w24a40_destroy_postorder(nodes: &[W24a40Node], root: usize, freed: &mut Vec<usize>) {
    let mut cur = root;
    while cur != 0 {
        let n = nodes[cur];
        w24a40_destroy_postorder(nodes, n.left, freed);
        freed.push(cur);
        cur = n.right;
    }
}

#[cfg(test)]
mod tests_wave24_agent40 {
    use super::*;

    #[test]
    fn destroy_visits_left_then_node_then_right() {
        // 1 root, 2 left, 3 right
        let nodes = [
            W24a40Node::default(),
            W24a40Node {
                parent: 0,
                left: 2,
                right: 3,
            },
            W24a40Node {
                parent: 1,
                left: 0,
                right: 0,
            },
            W24a40Node {
                parent: 1,
                left: 0,
                right: 0,
            },
        ];
        let mut v = Vec::new();
        w24a40_destroy_postorder(&nodes, 1, &mut v);
        assert_eq!(v, vec![2, 1, 3]);
        let mut e = Vec::new();
        w24a40_destroy_postorder(&nodes, 0, &mut e);
        assert!(e.is_empty());
    }

    #[test]
    fn clear_resets_headers() {
        let mut a = 0;
        let mut b = 0;
        let (h1, h2) = w24a40_clear_two_maps(5, 7, 0x10, 0x28, &mut |_| a += 1, &mut |_| b += 1);
        assert_eq!((a, b), (1, 1));
        assert_eq!(
            h1,
            W24a40Header {
                root: 0,
                leftmost: 0x10,
                rightmost: 0x10,
                size: 0
            }
        );
        assert_eq!(h2.leftmost, 0x28);
        let (_, _) = w24a40_clear_two_maps(5, 0, 1, 2, &mut |_| {}, &mut |_| b += 10);
        assert_eq!(b, 1);
    }
}
