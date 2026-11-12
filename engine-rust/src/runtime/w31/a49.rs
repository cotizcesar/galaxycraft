#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent49.rs
// ===========================================================================
// Wave 31 agent 49 reference helpers.

/// Result of the string-keyed descent: the caller's original slot or a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W31a49Found {
    /// The original `param_1` was returned (empty tree or exact match on the first hop).
    Origin,
    /// Index of the returned node.
    Node(usize),
}

/// One tree node: left child index and the key bytes (`+8` pointer, `+0xc` length).
#[derive(Debug, Clone)]
pub struct W31a49Node<'a> {
    pub left: Option<usize>,
    pub key: &'a [u8],
}

/// `0x008b9fb0`: walks left children comparing the wanted key (memcmp over the
/// shorter length, then length difference).  Returns the last node visited
/// before a match or before running out of children; a match returns the
/// previously tracked node (`Origin` on the first hop).
pub fn w31a49_find_left(root: Option<usize>, nodes: &[W31a49Node], wanted: &[u8]) -> W31a49Found {
    let Some(mut cur) = root else {
        return W31a49Found::Origin;
    };
    let mut prev = W31a49Found::Origin;
    loop {
        let node = &nodes[cur];
        let n = wanted.len().min(node.key.len());
        let mut diff = wanted[..n] != node.key[..n];
        if !diff {
            diff = wanted.len() != node.key.len();
        }
        if !diff {
            return prev;
        }
        prev = W31a49Found::Node(cur);
        match node.left {
            Some(l) => cur = l,
            None => return prev,
        }
    }
}

/// `0x008ba9a0`: forwards both arguments to the inner object's callee.
pub fn w31a49_forward2<R>(inner: u32, a: u32, b: u32, mut f: impl FnMut(u32, u32, u32) -> R) -> R {
    f(inner, a, b)
}

/// `0x008ba9c0`: stores both values at `+0x5c`/`+0x60`, then forwards them.
pub fn w31a49_store_and_forward(
    fields: &mut [u32; 2],
    inner: u32,
    a: u32,
    b: u32,
    mut f: impl FnMut(u32, u32, u32),
) {
    fields[0] = a;
    fields[1] = b;
    f(inner, a, b);
}

/// `0x008b0ad0`: feeds the result of the first callee into the second.
pub fn w31a49_chain(p: u32, first: impl FnOnce(u32) -> u32, second: impl FnOnce(u32)) {
    second(first(p));
}

#[cfg(test)]
mod tests_wave31_agent49 {
    use super::*;

    #[test]
    fn find_left() {
        let nodes = [
            W31a49Node {
                left: Some(1),
                key: b"mm",
            },
            W31a49Node {
                left: None,
                key: b"aa",
            },
        ];
        assert_eq!(w31a49_find_left(None, &nodes, b"x"), W31a49Found::Origin);
        assert_eq!(
            w31a49_find_left(Some(0), &nodes, b"mm"),
            W31a49Found::Origin
        );
        assert_eq!(
            w31a49_find_left(Some(0), &nodes, b"aa"),
            W31a49Found::Node(0)
        );
        assert_eq!(
            w31a49_find_left(Some(0), &nodes, b"zz"),
            W31a49Found::Node(1)
        );
    }

    #[test]
    fn forwards() {
        assert_eq!(w31a49_forward2(1, 2, 3, |a, b, c| a + b + c), 6);
        let mut f = [0, 0];
        let mut seen = vec![];
        w31a49_store_and_forward(&mut f, 9, 4, 5, |a, b, c| seen.push((a, b, c)));
        assert_eq!(f, [4, 5]);
        assert_eq!(seen, vec![(9, 4, 5)]);
        let mut out = 0;
        w31a49_chain(2, |x| x * 3, |y| out = y);
        assert_eq!(out, 6);
    }
}
