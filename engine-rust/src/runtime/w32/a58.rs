#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent58.rs
// ===========================================================================
// Wave 32 translation batch 58.
//
// Skipped: FUN_008a7060 (pure chain of unmodelled calls).

/// `0x008a7d40`: walks a circular intrusive list (sentinel at object offset
/// 0x800, first link at 0x804, next link at node+4) looking for a node whose
/// short at +0x24 equals the object's short at 0x6f8; calls `visit` with the
/// node payload offset (node+8) on the first match.  `links` maps a node
/// offset to `(next, key)`.
pub fn w32a58_visit_matching_node(
    first: usize,
    sentinel: usize,
    wanted: i16,
    links: &dyn Fn(usize) -> (usize, i16),
    mut visit: impl FnMut(usize),
) -> bool {
    let mut node = first;
    while node != sentinel {
        let (next, key) = links(node);
        if key == wanted {
            visit(node + 8);
            return true;
        }
        node = next;
    }
    false
}

/// `0x008c5130`: when the predicate (the event-type lookup) returns zero for
/// `param`, forwards `param` to the fallback handler.  Returns whether the
/// fallback ran.
pub fn w32a58_forward_if_unmapped(
    param: u32,
    lookup: impl FnOnce(u32) -> u32,
    fallback: impl FnOnce(u32),
) -> bool {
    if lookup(param) == 0 {
        fallback(param);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests_wave32_agent58 {
    use super::*;

    #[test]
    fn finds_node() {
        // nodes at 0x10 -> 0x20 -> sentinel 0x800
        let links = |n: usize| match n {
            0x10 => (0x20, 1),
            _ => (0x800, 7),
        };
        let mut got = None;
        assert!(w32a58_visit_matching_node(0x10, 0x800, 7, &links, |p| {
            got = Some(p)
        }));
        assert_eq!(got, Some(0x28));
        assert!(!w32a58_visit_matching_node(0x10, 0x800, 9, &links, |_| {}));
        assert!(!w32a58_visit_matching_node(0x800, 0x800, 1, &links, |_| {}));
    }

    #[test]
    fn forwards() {
        let mut seen = 0;
        assert!(w32a58_forward_if_unmapped(5, |_| 0, |p| seen = p));
        assert_eq!(seen, 5);
        assert!(!w32a58_forward_if_unmapped(5, |_| 1, |_| panic!()));
    }
}
