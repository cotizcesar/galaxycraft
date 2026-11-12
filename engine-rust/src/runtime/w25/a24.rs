#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent24.rs
// ===========================================================================
/// `0x00818080`: walks a circular intrusive list (sentinel `head`, `next` at
/// node offset 4) and returns the payload address (`node + 8`) of the first
/// node whose key (callee FUN_00553f90 modelled by `key_of`) equals `wanted`,
/// or `None` (0) when the walk returns to the sentinel.
pub fn w25a24_list_find(
    next: &dyn Fn(usize) -> usize,
    head: usize,
    wanted: i32,
    key_of: &dyn Fn(usize) -> i32,
) -> Option<usize> {
    let mut node = next(head);
    while node != head && key_of(node) != wanted {
        node = next(node);
    }
    if node == head { None } else { Some(node + 8) }
}

#[cfg(test)]
mod tests_wave25_agent24 {
    use super::*;

    #[test]
    fn find_in_ring() {
        // ring 0 -> 16 -> 32 -> 0
        let next = |n: usize| match n {
            0 => 16,
            16 => 32,
            _ => 0,
        };
        let key = |n: usize| n as i32 * 10;
        assert_eq!(w25a24_list_find(&next, 0, 320, &key), Some(40));
        assert_eq!(w25a24_list_find(&next, 0, 5, &key), None);
    }

    #[test]
    fn empty_ring() {
        let next = |_n: usize| 0;
        assert_eq!(w25a24_list_find(&next, 0, 0, &|_| 0), None);
    }
}
