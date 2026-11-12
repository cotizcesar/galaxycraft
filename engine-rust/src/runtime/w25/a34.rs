#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent34.rs
// ===========================================================================
// Wave 25 agent 34: container glue from the 0x0085xxxx block.

/// `0x00859650`: destroys an owned-pointer array: for each of the `len`
/// slots that is non-null (`Some`), the object's virtual destructor is
/// invoked with flag 1 (delete).  Returns the number of destructor calls.
/// The retail loop count is `(len * 4 + 3) >> 2`, i.e. `len`.
pub fn w25a34_destroy_ptr_array<T>(slots: &mut [Option<T>], mut dtor: impl FnMut(T)) -> usize {
    let mut calls = 0;
    for slot in slots.iter_mut() {
        if let Some(obj) = slot.take() {
            dtor(obj);
            calls += 1;
        }
    }
    calls
}

/// Circular list head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W25a34ListHead {
    pub prev: u32,
    pub next: u32,
    pub size: u32,
}

/// `0x0085cfc0`: resets a circular list head at address `this` so that both
/// links point at itself and the size is zero.  Returns the previous `next`
/// link; the retail code walks and frees the nodes when it differs from
/// `this`.
pub fn w25a34_list_reset(head: &mut W25a34ListHead, this: u32) -> u32 {
    let old_next = head.next;
    head.next = this;
    head.prev = this;
    head.size = 0;
    old_next
}

#[cfg(test)]
mod tests_wave25_agent34 {
    use super::*;

    #[test]
    fn destroy_skips_null() {
        let mut v = vec![Some(1), None, Some(3)];
        let mut seen = vec![];
        let n = w25a34_destroy_ptr_array(&mut v, |x| seen.push(x));
        assert_eq!((n, seen), (2, vec![1, 3]));
        assert!(v.iter().all(Option::is_none));
    }

    #[test]
    fn list_reset_self_links() {
        let mut h = W25a34ListHead {
            prev: 5,
            next: 9,
            size: 4,
        };
        assert_eq!(w25a34_list_reset(&mut h, 0x100), 9);
        assert_eq!(
            h,
            W25a34ListHead {
                prev: 0x100,
                next: 0x100,
                size: 0
            }
        );
    }
}
