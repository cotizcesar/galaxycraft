#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent39.rs
// ===========================================================================
// Wave 32 agent 39 translations.

/// One node of the singly linked list walked by `0x0056ff90`: the next link
/// (index) and the node's key bytes (pointer at +4, length at +8).
pub struct W32a39Node<'a> {
    pub next: Option<usize>,
    pub key: &'a [u8],
}

/// `0x0056ff90`: walks a singly linked list from `head` comparing each node
/// key with `needle` (memcmp over the shorter length, then length
/// difference).  Returns `None` when the result is the list head pointer
/// itself (empty list, or match on the first node), otherwise `Some(i)`: the
/// node visited before the match, or the last node when nothing matched.
pub fn w32a39_find_prev_node(
    head: Option<usize>,
    nodes: &[W32a39Node<'_>],
    needle: &[u8],
) -> Option<usize> {
    let mut cur = head?;
    let mut result: Option<usize> = None;
    loop {
        let node = &nodes[cur];
        let n = node.key.len().min(needle.len());
        let mut c = match needle[..n].cmp(&node.key[..n]) {
            std::cmp::Ordering::Equal => 0,
            _ => 1,
        };
        if c == 0 {
            c = (needle.len() as u32).wrapping_sub(node.key.len() as u32);
        }
        if c == 0 {
            return result;
        }
        result = Some(cur);
        match node.next {
            None => return Some(cur),
            Some(nx) => cur = nx,
        }
    }
}

#[cfg(test)]
mod tests_wave32_agent39 {
    use super::*;

    #[test]
    fn walk() {
        let nodes = [
            W32a39Node {
                next: Some(1),
                key: b"a",
            },
            W32a39Node {
                next: Some(2),
                key: b"bb",
            },
            W32a39Node {
                next: None,
                key: b"c",
            },
        ];
        assert_eq!(w32a39_find_prev_node(None, &nodes, b"a"), None);
        assert_eq!(w32a39_find_prev_node(Some(0), &nodes, b"a"), None);
        assert_eq!(w32a39_find_prev_node(Some(0), &nodes, b"bb"), Some(0));
        assert_eq!(w32a39_find_prev_node(Some(0), &nodes, b"c"), Some(1));
        assert_eq!(w32a39_find_prev_node(Some(0), &nodes, b"zz"), Some(2));
    }
}
