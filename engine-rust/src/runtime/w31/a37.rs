#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent37.rs
// ===========================================================================
// Wave 31 agent 37: hash-chain lookup helpers (runtime.rs).
//
// Skipped: 7cdec0 (STL vector glue), 7d4f80 (unrecovered jump table),
// 7d8c90 (string move glue), and the Battle.net property-name
// throwers 7d8cf0, 7d8ed0, 7d9020, 7d90d0, 7d9180, 7d9230, 7d92e0, 7d93e0,
// 7d94e0, 7d95c0, 7d9b10, 7d9bb0 (bnet / non-returning).

/// Hash-chain node: next link (index into the node slice) and 64-bit key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W31a37Node {
    pub next: Option<usize>,
    pub key: (u32, u32),
}

/// `0x007d1940`: walks a chain from `first` and returns the predecessor of
/// the node whose key equals `key` (`None` stands for the head slot itself),
/// or the last node of the chain when nothing matches.
pub fn w31a37_find_prev(
    nodes: &[W31a37Node],
    first: Option<usize>,
    key: (u32, u32),
) -> Option<usize> {
    let mut cur = first?;
    let mut prev: Option<usize> = None;
    loop {
        let n = &nodes[cur];
        if n.key == key {
            return prev;
        }
        match n.next {
            None => return Some(cur),
            Some(nx) => {
                prev = Some(cur);
                cur = nx;
            }
        }
    }
}

/// Result of [`w31a37_find`]: matching node, bucket position, end position
/// (positions are in bucket units; the original scales them by 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W31a37Found {
    pub node: Option<usize>,
    pub pos: u32,
    pub end: u32,
}

/// `0x007d19d0`: finalizer-style hash of the low key word, bucket lookup,
/// chain walk comparing both key words.  `size` is the bucket count (a power
/// of two); an empty bucket array yields (none, size, size).
pub fn w31a37_find(
    size: u32,
    buckets: &[Option<usize>],
    nodes: &[W31a37Node],
    key: (u32, u32),
) -> W31a37Found {
    if buckets.is_empty() {
        return W31a37Found {
            node: None,
            pos: size,
            end: size,
        };
    }
    let mut h = (key.0 >> 16 ^ key.0).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    h = (h >> 16 ^ h) & size.wrapping_sub(1);
    let mut node = None;
    let mut cur = buckets[h as usize];
    while let Some(i) = cur {
        if nodes[i].key == key {
            node = Some(i);
            break;
        }
        cur = nodes[i].next;
    }
    W31a37Found {
        node,
        pos: if node.is_some() { h } else { size },
        end: size,
    }
}

#[cfg(test)]
mod tests_wave31_agent37 {
    use super::*;

    fn chain() -> Vec<W31a37Node> {
        vec![
            W31a37Node {
                next: Some(1),
                key: (1, 2),
            },
            W31a37Node {
                next: Some(2),
                key: (3, 4),
            },
            W31a37Node {
                next: None,
                key: (5, 6),
            },
        ]
    }

    #[test]
    fn prev_lookup() {
        let n = chain();
        assert_eq!(w31a37_find_prev(&n, None, (1, 2)), None);
        assert_eq!(w31a37_find_prev(&n, Some(0), (1, 2)), None);
        assert_eq!(w31a37_find_prev(&n, Some(0), (3, 4)), Some(0));
        assert_eq!(w31a37_find_prev(&n, Some(0), (9, 9)), Some(2));
    }

    #[test]
    fn hash_lookup() {
        let mut n = chain();
        let key = (7u32, 8u32);
        n[1].key = key;
        let mut h = (key.0 >> 16 ^ key.0).wrapping_mul(0x85eb_ca6b);
        h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
        h = (h >> 16 ^ h) & 7;
        let mut b = vec![None; 8];
        b[h as usize] = Some(0);
        let f = w31a37_find(8, &b, &n, key);
        assert_eq!(
            f,
            W31a37Found {
                node: Some(1),
                pos: h,
                end: 8
            }
        );
        let f = w31a37_find(8, &b, &n, (99, 1));
        assert_eq!(f.node, None);
        assert_eq!(f.pos, 8);
        assert_eq!(
            w31a37_find(4, &[], &n, key),
            W31a37Found {
                node: None,
                pos: 4,
                end: 4
            }
        );
    }
}
