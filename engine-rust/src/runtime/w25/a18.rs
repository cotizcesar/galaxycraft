#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent18.rs
// ===========================================================================
// Wave 25 agent 18: MSVC STL vector-assign planning and node destructors.

/// Outcome of the vector `assign(first, last)` planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W25a18AssignPlan {
    /// Source fits in current storage and `n <= len`: copy `n` elements.
    CopyAll(usize),
    /// Fits in capacity but `n > len`: copy `len` elements then construct the tail.
    CopyThenConstruct { copy: usize, total: usize },
    /// Reallocate to `new_cap` elements.
    Realloc { new_cap: usize },
    /// Element count above the container maximum (length_error).
    LengthError,
}

fn w25a18_plan(n: usize, len: usize, cap: usize, max: usize) -> W25a18AssignPlan {
    if n <= cap {
        if n <= len {
            return W25a18AssignPlan::CopyAll(n);
        }
        return W25a18AssignPlan::CopyThenConstruct {
            copy: len,
            total: n,
        };
    }
    if n > max {
        return W25a18AssignPlan::LengthError;
    }
    let half = cap >> 1;
    let mut new_cap = n;
    if cap <= max - half {
        new_cap = half + cap;
        if new_cap < n {
            new_cap = n;
        }
    }
    W25a18AssignPlan::Realloc { new_cap }
}

/// `0x007f4eb0`: assign planner for a vector of 4-byte elements (max
/// `0x3fffffff`, growth 1.5x clamped up to the requested count).
pub fn w25a18_vec4_assign_plan(n: usize, len: usize, cap: usize) -> W25a18AssignPlan {
    w25a18_plan(n, len, cap, 0x3fff_ffff)
}

/// `0x007f4d60`: same planner for the 8-byte-element vector at offset 0x14
/// (max `0x1fffffff`).
pub fn w25a18_vec8_assign_plan(n: usize, len: usize, cap: usize) -> W25a18AssignPlan {
    w25a18_plan(n, len, cap, 0x1fff_ffff)
}

/// `0x007f4cb0`: big-allocation unwrap before free.  Returns `None` for an
/// empty vector, `Err(())` when the padding check fails, else
/// `(block_ptr, byte_size)`; `read_prev` reads the word before `begin`.
pub fn w25a18_vec_free_args(
    begin: u32,
    cap_end: u32,
    read_prev: impl Fn(u32) -> u32,
) -> Option<Result<(u32, u32), ()>> {
    if begin == 0 {
        return None;
    }
    let mut size = cap_end.wrapping_sub(begin) & 0xffff_fff8;
    let mut ptr = begin;
    if size > 0xfff {
        ptr = read_prev(begin.wrapping_sub(4));
        size = size.wrapping_add(0x23);
        if begin.wrapping_sub(ptr).wrapping_sub(4) > 0x1f {
            return Some(Err(()));
        }
    }
    Some(Ok((ptr, size)))
}

/// Scalar-deleting-destructor flag test shared by the trivial node dtors
/// `0x007f5ff0`, `0x007f6090`, `0x007f60c0`, `0x007f61f0`, `0x007f6220`,
/// `0x007f62d0`, `0x007f6300`: returns true when bit 0 requests delete.
pub fn w25a18_dtor_should_delete(flag: u8) -> bool {
    flag & 1 != 0
}

/// `0x007f61a0`: `_Node_assert` destructor; walks the chain from `head`
/// (next link at dword 3), clearing each link and deleting each node.
/// Returns the deletion order.
pub fn w25a18_node_assert_dtor(head: Option<usize>, next: &mut [Option<usize>]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut cur = head;
    while let Some(i) = cur {
        cur = next[i].take();
        out.push(i);
    }
    out
}

/// `0x007f6250`: `_Node_if` destructor.  `outer` is the chain at dword 6
/// starting at `first`; for each outer node the inner chain (dword 3 links)
/// is deleted until it reaches `stop` (dword 5) or ends.  The outer node
/// itself is the first inner node.  Returns deletion order.
pub fn w25a18_node_if_dtor(
    first: Option<usize>,
    stop: Option<usize>,
    next3: &mut [Option<usize>],
    next6: &mut [Option<usize>],
) -> Vec<usize> {
    let mut out = Vec::new();
    let mut outer = first;
    while let Some(o) = outer {
        outer = next6[o].take();
        let mut inner = Some(o);
        while let Some(i) = inner {
            if Some(i) == stop {
                break;
            }
            inner = next3[i].take();
            out.push(i);
        }
    }
    out
}

#[cfg(test)]
mod tests_wave25_agent18 {
    use super::*;

    #[test]
    fn plans() {
        assert_eq!(
            w25a18_vec4_assign_plan(3, 5, 8),
            W25a18AssignPlan::CopyAll(3)
        );
        assert_eq!(
            w25a18_vec4_assign_plan(6, 5, 8),
            W25a18AssignPlan::CopyThenConstruct { copy: 5, total: 6 }
        );
        assert_eq!(
            w25a18_vec4_assign_plan(9, 5, 8),
            W25a18AssignPlan::Realloc { new_cap: 12 }
        );
        assert_eq!(
            w25a18_vec4_assign_plan(30, 5, 8),
            W25a18AssignPlan::Realloc { new_cap: 30 }
        );
        assert_eq!(
            w25a18_vec4_assign_plan(0x4000_0000, 0, 0),
            W25a18AssignPlan::LengthError
        );
    }

    #[test]
    fn plans8() {
        assert_eq!(
            w25a18_vec8_assign_plan(0x2000_0000, 0, 0),
            W25a18AssignPlan::LengthError
        );
        assert_eq!(
            w25a18_vec8_assign_plan(4, 0, 2),
            W25a18AssignPlan::Realloc { new_cap: 4 }
        );
    }

    #[test]
    fn free_args() {
        assert_eq!(w25a18_vec_free_args(0, 0, |_| 0), None);
        assert_eq!(
            w25a18_vec_free_args(0x1000, 0x1040, |_| 0),
            Some(Ok((0x1000, 0x40)))
        );
        assert_eq!(
            w25a18_vec_free_args(0x2000, 0x3000, |_| 0x1fe0),
            Some(Ok((0x1fe0, 0x1023)))
        );
        assert_eq!(
            w25a18_vec_free_args(0x2000, 0x3000, |_| 0x1000),
            Some(Err(()))
        );
    }

    #[test]
    fn dtor_flag() {
        assert!(w25a18_dtor_should_delete(1));
        assert!(!w25a18_dtor_should_delete(0));
        assert!(!w25a18_dtor_should_delete(2));
    }

    #[test]
    fn assert_chain() {
        let mut next = vec![Some(2), None, Some(1)];
        assert_eq!(w25a18_node_assert_dtor(Some(0), &mut next), vec![0, 2, 1]);
        assert!(next.iter().all(|n| n.is_none()));
    }

    #[test]
    fn if_chain() {
        let mut n3 = vec![Some(1), None, None];
        let mut n6 = vec![Some(2), None, None];
        let out = w25a18_node_if_dtor(Some(0), None, &mut n3, &mut n6);
        assert_eq!(out, vec![0, 1, 2]);
        let mut n3 = vec![Some(1), None];
        let mut n6 = vec![None, None];
        assert_eq!(
            w25a18_node_if_dtor(Some(0), Some(1), &mut n3, &mut n6),
            vec![0]
        );
    }
}
