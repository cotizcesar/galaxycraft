#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent87.rs
// ===========================================================================
// Wave 32 agent 87: linked-list lookups by name (bounded compare).

/// `0x00a7b5c0`: walks a singly linked list (head from `this+0x38`, next at
/// `+0x20`) and returns the first node whose name equals `wanted`, or whose
/// bounded compare (`max = 0x7fffffff`) returns 0.  `None` is the null node.
pub fn w32a87_find_node_by_name<N: Copy, K: Copy + PartialEq>(
    head: Option<N>,
    next: impl Fn(N) -> Option<N>,
    name_of: impl Fn(N) -> K,
    compare: impl Fn(K, K, i32) -> i32,
    wanted: K,
) -> Option<N> {
    let mut cur = head;
    while let Some(node) = cur {
        let name = name_of(node);
        if name == wanted || compare(name, wanted, 0x7fff_ffff) == 0 {
            return Some(node);
        }
        cur = next(node);
    }
    None
}

/// `0x00a7b610`: walks a list (head from `this+0x1c`, next at `+0x28`); each
/// entry yields an object via its first vtable slot.  An absent object is
/// skipped; with `wanted == 0` the first object is returned; otherwise the
/// object's name (0 when its slot `+0x18` reports non-zero, else the
/// default name) is matched by equality or a bounded compare returning 0.
pub fn w32a87_find_object_by_name<E: Copy, O: Copy>(
    head: Option<E>,
    next: impl Fn(E) -> Option<E>,
    object_of: impl Fn(E) -> Option<O>,
    has_override: impl Fn(O) -> bool,
    default_name: impl Fn() -> i32,
    compare: impl Fn(i32, i32, i32) -> i32,
    wanted: i32,
) -> Option<O> {
    let mut cur = head;
    while let Some(entry) = cur {
        if let Some(obj) = object_of(entry) {
            if wanted == 0 {
                return Some(obj);
            }
            let name = if has_override(obj) { 0 } else { default_name() };
            if name == wanted || compare(name, wanted, 0x7fff_ffff) == 0 {
                return Some(obj);
            }
        }
        cur = next(entry);
    }
    None
}

#[cfg(test)]
mod tests_wave32_agent87 {
    #[test]
    fn find_node() {
        let next = |n: usize| if n < 3 { Some(n + 1) } else { None };
        let name = |n: usize| n as i32 * 10;
        let cmp = |a: i32, b: i32, _m: i32| (a - b).abs();
        assert_eq!(
            super::w32a87_find_node_by_name(Some(1), next, name, cmp, 20),
            Some(2)
        );
        assert_eq!(
            super::w32a87_find_node_by_name(Some(1), next, name, cmp, 99),
            None
        );
        assert_eq!(
            super::w32a87_find_node_by_name(None, next, name, cmp, 10),
            None
        );
    }

    #[test]
    fn find_object() {
        let next = |n: usize| if n < 3 { Some(n + 1) } else { None };
        let obj = |n: usize| if n == 1 { None } else { Some(n) };
        let f = |wanted| {
            super::w32a87_find_object_by_name(
                Some(1),
                next,
                obj,
                |o| o == 3,
                || 7,
                |a, b, _| a - b,
                wanted,
            )
        };
        assert_eq!(f(0), Some(2));
        assert_eq!(f(7), Some(2));
        assert_eq!(f(5), None);
    }
}
