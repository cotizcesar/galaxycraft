#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent25.rs
// ===========================================================================
// Wave 24 agent 25: MSVC red-black tree (`std::map<u16, _>`) helpers.
//
// Node layout (patch 1.23.10): `+0` parent, `+4` left, `+8` right, `+0xc`
// color byte, `+0x10` u16 key, value at `+0x14`.  The header shares the
// first three words (root, leftmost, rightmost); `+0x10` is the count.

/// Pointer-like link: null, the header sentinel, or a node arena index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W24a25Link {
    Nil,
    Head,
    Node(usize),
}

/// Which child slot of a parent receives an insertion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W24a25Slot {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub struct W24a25Node {
    pub parent: W24a25Link,
    pub left: W24a25Link,
    pub right: W24a25Link,
    pub color: u8,
    pub key: u16,
}

/// Tree header (root, leftmost, rightmost, aux word, element count).
#[derive(Clone, Debug)]
pub struct W24a25Head {
    pub root: W24a25Link,
    pub left: W24a25Link,
    pub right: W24a25Link,
    pub aux: u32,
    pub count: u32,
}

impl W24a25Head {
    pub fn new() -> Self {
        Self {
            root: W24a25Link::Nil,
            left: W24a25Link::Head,
            right: W24a25Link::Head,
            aux: 0,
            count: 0,
        }
    }
}

impl Default for W24a25Head {
    fn default() -> Self {
        Self::new()
    }
}

/// `0x00777c60`: when the flag byte at `this+4` is set, calls the release
/// routine on the handle that `this` points at.
pub fn w24a25_release_if_set(flag: u8, handle: u32, mut release: impl FnMut(u32)) {
    if flag != 0 {
        release(handle);
    }
}

/// `0x00777d80`: move-assign of the tree header.  Clears `this` (calling
/// `erase_all` when it still has a root), swaps the four header words with
/// `other`, takes over `other`'s count and zeroes it, then repoints the
/// sentinel links.  Returns the new root's parent fix-up target when the
/// moved tree is non-empty (`true` = root parent must become `this`).
pub fn w24a25_header_swap(
    this: &mut W24a25Head,
    other: &mut W24a25Head,
    mut erase_all: impl FnMut(W24a25Link),
) -> bool {
    if this.root != W24a25Link::Nil {
        erase_all(this.root);
    }
    this.count = 0;
    this.root = W24a25Link::Nil;
    this.left = W24a25Link::Head;
    this.right = W24a25Link::Head;
    std::mem::swap(&mut this.root, &mut other.root);
    std::mem::swap(&mut this.left, &mut other.left);
    std::mem::swap(&mut this.right, &mut other.right);
    std::mem::swap(&mut this.aux, &mut other.aux);
    this.count = other.count;
    other.count = 0;
    if this.count == 0 {
        this.left = W24a25Link::Head;
        this.right = W24a25Link::Head;
        other.left = W24a25Link::Head;
        other.right = W24a25Link::Head;
        false
    } else {
        other.left = W24a25Link::Head;
        other.right = W24a25Link::Head;
        true
    }
}

fn w24a25_key(head: &W24a25Head, nodes: &[W24a25Node], l: W24a25Link) -> u16 {
    match l {
        W24a25Link::Head => head.count as u16,
        W24a25Link::Node(i) => nodes[i].key,
        W24a25Link::Nil => 0,
    }
}

fn w24a25_child(nodes: &[W24a25Node], l: W24a25Link, s: W24a25Slot) -> W24a25Link {
    match l {
        W24a25Link::Node(i) => match s {
            W24a25Slot::Left => nodes[i].left,
            W24a25Slot::Right => nodes[i].right,
        },
        _ => W24a25Link::Nil,
    }
}

/// `0x0077a8d0`: locate the insertion point for `key`.  Returns the node
/// reached and `Some(slot)` when the key is absent (insert there) or `None`
/// when it already exists.  `prev` models the iterator decrement
/// (`0x59_7c70`).
pub fn w24a25_find_insert_point(
    head: &W24a25Head,
    nodes: &[W24a25Node],
    key: u16,
    prev: impl Fn(W24a25Link) -> W24a25Link,
) -> (W24a25Link, Option<W24a25Slot>) {
    let mut cur = W24a25Link::Head;
    let mut less = true;
    let mut next = head.root;
    while next != W24a25Link::Nil {
        cur = next;
        less = key < w24a25_key(head, nodes, cur);
        next = w24a25_child(
            nodes,
            cur,
            if less {
                W24a25Slot::Left
            } else {
                W24a25Slot::Right
            },
        );
    }
    if !less {
        return if w24a25_key(head, nodes, cur) < key {
            (cur, Some(W24a25Slot::Right))
        } else {
            (cur, None)
        };
    }
    if cur != head.left {
        let p = prev(cur);
        if key <= w24a25_key(head, nodes, p) {
            return (p, None);
        }
    }
    (cur, Some(W24a25Slot::Left))
}

/// `0x00777e10`: map lookup wrapper.  Returns the node whose value (at node
/// `+0x14`) is wanted, or `None` when the key is absent (the original then
/// takes the insertion path, not shown in the body).
pub fn w24a25_lookup(
    head: &W24a25Head,
    nodes: &[W24a25Node],
    key: u16,
    prev: impl Fn(W24a25Link) -> W24a25Link,
) -> Option<W24a25Link> {
    match w24a25_find_insert_point(head, nodes, key, prev) {
        (n, None) => Some(n),
        _ => None,
    }
}

/// Offset of the mapped value inside a node (`0x00777e10`).
pub const W24A25_VALUE_OFFSET: usize = 0x14;

/// `0x0077a850`: link node `new` below `parent` in `slot`, bump the count,
/// maintain leftmost/rightmost, and call `rebalance` (`0x59_7290`).
pub fn w24a25_tree_link(
    head: &mut W24a25Head,
    nodes: &mut [W24a25Node],
    parent: W24a25Link,
    slot: W24a25Slot,
    new: usize,
    mut rebalance: impl FnMut(usize),
) {
    head.count += 1;
    nodes[new].parent = parent;
    nodes[new].left = W24a25Link::Nil;
    nodes[new].right = W24a25Link::Nil;
    nodes[new].color = 0;
    let n = W24a25Link::Node(new);
    match parent {
        W24a25Link::Head => {
            head.root = n;
            head.left = n;
            head.right = n;
        }
        W24a25Link::Node(p) => {
            match slot {
                W24a25Slot::Left => nodes[p].left = n,
                W24a25Slot::Right => nodes[p].right = n,
            }
            if parent == head.left && slot == W24a25Slot::Left {
                head.left = n;
            }
            if parent == head.right && slot == W24a25Slot::Right {
                head.right = n;
            }
        }
        W24a25Link::Nil => {}
    }
    rebalance(new);
}

/// `0x0077c0d0`: return value of the intrusive-node release helper: the
/// tagged link word `tagged` (taken from `param+4`, or from `this+4` when
/// the node pointer is null) when it is even and non-zero, else 0.
pub fn w24a25_untagged_link(tagged: u32) -> u32 {
    if tagged & 1 == 0 && tagged != 0 {
        tagged
    } else {
        0
    }
}

#[cfg(test)]
mod tests_wave24_agent25 {
    use super::*;

    fn node(key: u16) -> W24a25Node {
        W24a25Node {
            parent: W24a25Link::Nil,
            left: W24a25Link::Nil,
            right: W24a25Link::Nil,
            color: 9,
            key,
        }
    }

    #[test]
    fn release_flag() {
        let mut got = vec![];
        w24a25_release_if_set(0, 5, |h| got.push(h));
        w24a25_release_if_set(1, 7, |h| got.push(h));
        assert_eq!(got, vec![7]);
    }

    #[test]
    fn link_and_find() {
        let mut head = W24a25Head::new();
        let mut nodes = vec![node(10), node(5), node(20)];
        let none = |_| W24a25Link::Nil;
        let (p, s) = w24a25_find_insert_point(&head, &nodes, 10, none);
        assert_eq!((p, s), (W24a25Link::Head, Some(W24a25Slot::Left)));
        w24a25_tree_link(&mut head, &mut nodes, p, s.unwrap(), 0, |_| {});
        assert_eq!(head.count, 1);
        assert_eq!(head.root, W24a25Link::Node(0));
        let (p, s) = w24a25_find_insert_point(&head, &nodes, 5, none);
        assert_eq!(s, Some(W24a25Slot::Left));
        w24a25_tree_link(&mut head, &mut nodes, p, s.unwrap(), 1, |_| {});
        assert_eq!(head.left, W24a25Link::Node(1));
        let (p, s) = w24a25_find_insert_point(&head, &nodes, 20, none);
        assert_eq!(s, Some(W24a25Slot::Right));
        w24a25_tree_link(&mut head, &mut nodes, p, s.unwrap(), 2, |_| {});
        assert_eq!(head.right, W24a25Link::Node(2));
        assert_eq!(nodes[2].color, 0);
        assert_eq!(
            w24a25_lookup(&head, &nodes, 20, none),
            Some(W24a25Link::Node(2))
        );
        assert_eq!(
            w24a25_lookup(&head, &nodes, 7, |_| W24a25Link::Node(1)),
            None
        );
        assert_eq!(
            w24a25_find_insert_point(&head, &nodes, 5, |_| W24a25Link::Nil).1,
            None
        );
        assert_eq!(W24A25_VALUE_OFFSET, 0x14);
    }

    #[test]
    fn swap_header() {
        let mut a = W24a25Head::new();
        let mut b = W24a25Head::new();
        b.root = W24a25Link::Node(3);
        b.left = W24a25Link::Node(3);
        b.right = W24a25Link::Node(3);
        b.count = 4;
        let mut erased = 0;
        assert!(w24a25_header_swap(&mut a, &mut b, |_| erased += 1));
        assert_eq!((erased, a.count, b.count), (0, 4, 0));
        assert_eq!(a.root, W24a25Link::Node(3));
        assert_eq!(b.root, W24a25Link::Nil);
        assert_eq!(b.left, W24a25Link::Head);
        assert!(!w24a25_header_swap(&mut a, &mut b, |_| erased += 1));
        assert_eq!(erased, 1);
        assert_eq!(a.count, 0);
    }

    #[test]
    fn untagged() {
        assert_eq!(w24a25_untagged_link(0), 0);
        assert_eq!(w24a25_untagged_link(3), 0);
        assert_eq!(w24a25_untagged_link(8), 8);
    }
}
