#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent30.rs
// ===========================================================================
// Wave 25 agent 30: shared-ptr release, string-keyed tree lookup, list unlink.

/// Action the shared-pointer release at `0x00841650` asks of its caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W25a30ReleaseAction {
    /// Use count was still above one afterwards: nothing to do.
    None,
    /// Use count hit zero: the object was disposed (vtable slot 8) but weak
    /// references remain.
    Dispose,
    /// Use and weak counts both hit zero: dispose, then destroy (slot 0xc).
    DisposeAndDestroy,
}

/// `0x00841650`: release of a `blz` shared-pointer control block.  The use
/// count (offset 4) is atomically decremented; when the old value was 1 the
/// object is disposed and the weak count (offset 8) is decremented, and when
/// that old value was 1 the block is destroyed.  Returns the new
/// `(use, weak)` counts and the action.
pub fn w25a30_shared_release(use_count: i32, weak_count: i32) -> (i32, i32, W25a30ReleaseAction) {
    let new_use = use_count.wrapping_sub(1);
    if use_count != 1 {
        return (new_use, weak_count, W25a30ReleaseAction::None);
    }
    let new_weak = weak_count.wrapping_sub(1);
    if weak_count == 1 {
        (new_use, new_weak, W25a30ReleaseAction::DisposeAndDestroy)
    } else {
        (new_use, new_weak, W25a30ReleaseAction::Dispose)
    }
}

/// `0x008404a0`: validates a two-state value; 1 yields `Ok(0)`, anything
/// else (including 0) is the fatal-error path.
pub fn w25a30_expect_one(value: i32) -> Result<u32, ()> {
    if value == 1 { Ok(0) } else { Err(()) }
}

/// One node of the string-keyed tree walked by `0x00841f30`.  Retail layout:
/// left at +4, right at +8, key pointer at +0x10, key length at +0x14,
/// value at +0x2c.
#[derive(Debug, Clone)]
pub struct W25a30Node {
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub key: Vec<u8>,
    pub value: u32,
}

fn w25a30_cmp(a: &[u8], b: &[u8]) -> i32 {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return a[i] as i32 - b[i] as i32;
        }
    }
    (a.len() as i32).wrapping_sub(b.len() as i32)
}

/// `0x00841f30`: lower-bound lookup in a string-keyed tree (`root` is the
/// node under the header).  Descends right when the node key is below
/// `key`, otherwise records the node and goes left; the candidate is
/// accepted when `key >= candidate key`.  `None` is the retail
/// out-of-range throw.
pub fn w25a30_tree_lookup(nodes: &[W25a30Node], root: Option<usize>, key: &[u8]) -> Option<u32> {
    let mut cur = root?;
    let mut cand: Option<usize> = None;
    loop {
        let n = &nodes[cur];
        let c = w25a30_cmp(&n.key, key);
        let next = if c < 0 {
            n.right
        } else {
            cand = Some(cur);
            n.left
        };
        match next {
            Some(i) => cur = i,
            None => break,
        }
    }
    let c = nodes[cand?].clone();
    if w25a30_cmp(key, &c.key) >= 0 {
        Some(c.value)
    } else {
        None
    }
}

/// `0x00843980`: unlinks a node from a circular doubly linked list
/// (`next` at +0, `prev` at +4): `next.prev = prev; prev.next = next`.
/// The rest of the retail body is member destructors.
pub fn w25a30_list_unlink(next: &mut [usize], prev: &mut [usize], node: usize) {
    let n = next[node];
    let p = prev[node];
    prev[n] = p;
    next[p] = n;
}

#[cfg(test)]
mod tests_wave25_agent30 {
    use super::*;

    #[test]
    fn release_counts() {
        assert_eq!(
            w25a30_shared_release(2, 1),
            (1, 1, W25a30ReleaseAction::None)
        );
        assert_eq!(
            w25a30_shared_release(1, 2),
            (0, 1, W25a30ReleaseAction::Dispose)
        );
        assert_eq!(
            w25a30_shared_release(1, 1),
            (0, 0, W25a30ReleaseAction::DisposeAndDestroy)
        );
    }

    #[test]
    fn expect_one() {
        assert_eq!(w25a30_expect_one(1), Ok(0));
        assert!(w25a30_expect_one(0).is_err());
        assert!(w25a30_expect_one(2).is_err());
    }

    #[test]
    fn lookup() {
        let mk = |k: &str, l, r, v| W25a30Node {
            left: l,
            right: r,
            key: k.as_bytes().to_vec(),
            value: v,
        };
        let nodes = vec![
            mk("b", Some(1), Some(2), 20),
            mk("a", None, None, 10),
            mk("c", None, None, 30),
        ];
        assert_eq!(w25a30_tree_lookup(&nodes, Some(0), b"a"), Some(10));
        assert_eq!(w25a30_tree_lookup(&nodes, Some(0), b"c"), Some(30));
        assert_eq!(w25a30_tree_lookup(&nodes, Some(0), b"bb"), None);
        assert_eq!(w25a30_tree_lookup(&nodes, None, b"a"), None);
    }

    #[test]
    fn unlink() {
        let mut next = vec![1, 2, 0];
        let mut prev = vec![2, 0, 1];
        w25a30_list_unlink(&mut next, &mut prev, 1);
        assert_eq!(next, vec![2, 2, 0]);
        assert_eq!(prev, vec![2, 0, 0]);
    }
}
