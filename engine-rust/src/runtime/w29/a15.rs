#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave29/agent15.rs
// ===========================================================================
// Wave 29 agent 15: length-prefixed key ordering and heap helpers.

/// `0x00ada860`: strict "less than" on length-prefixed byte keys
/// (`key[0]` is the length, bytes follow).  Compares the common prefix
/// bytewise; on a tie the shorter key is smaller.  `this` is the ECX operand.
pub fn w29a15_key_less(this: &[u8], other: &[u8]) -> bool {
    let la = this.first().copied().unwrap_or(0) as usize;
    let lb = other.first().copied().unwrap_or(0) as usize;
    let n = la.min(lb);
    for i in 1..=n {
        let a = this.get(i).copied().unwrap_or(0);
        let b = other.get(i).copied().unwrap_or(0);
        if a != b {
            return a < b;
        }
    }
    la < lb
}

/// `0x00ada170`: MSVC `_Adjust_heap` for 0x28-byte elements: sift the hole
/// at `hole` down to a leaf choosing the larger child under `less`, then sift
/// `value` back up.  `len` is the heap length.
pub fn w29a15_adjust_heap<T: Clone>(
    heap: &mut [T],
    hole: usize,
    len: usize,
    value: T,
    less: &mut dyn FnMut(&T, &T) -> bool,
) {
    let top = hole;
    let mut hole = hole;
    let mut child = hole * 2 + 2;
    while child < len {
        if less(&heap[child], &heap[child - 1]) {
            child -= 1;
        }
        heap[hole] = heap[child].clone();
        hole = child;
        child = hole * 2 + 2;
    }
    if child == len {
        heap[hole] = heap[child - 1].clone();
        hole = child - 1;
    }
    while top < hole {
        let parent = (hole - 1) >> 1;
        if !less(&heap[parent], &value) {
            break;
        }
        heap[hole] = heap[parent].clone();
        hole = parent;
    }
    heap[hole] = value;
}

/// `0x00ada280`: `make_heap` over a slice of 0x28-byte elements; fewer than
/// two elements is a no-op.
pub fn w29a15_make_heap<T: Clone>(heap: &mut [T], less: &mut dyn FnMut(&T, &T) -> bool) {
    let len = heap.len();
    if len < 2 {
        return;
    }
    let mut hole = (len >> 1) - 1;
    loop {
        let value = heap[hole].clone();
        w29a15_adjust_heap(heap, hole, len, value, less);
        if hole == 0 {
            return;
        }
        hole -= 1;
    }
}

/// `0x00adb110`: zero the two-word output, run the lookup `f` (the
/// `FUN_00adbfb0` core) and fold result codes 0, 6 and 0xd to success.
pub fn w29a15_lookup_fold(out: &mut [u32; 2], f: &mut dyn FnMut(&mut [u32; 2]) -> i32) -> i32 {
    *out = [0, 0];
    let r = f(out);
    if r == 0 || r == 0xd || r == 6 { 0 } else { r }
}

#[cfg(test)]
mod tests_wave29_agent15 {
    use super::*;

    #[test]
    fn key_less() {
        assert!(w29a15_key_less(&[2, 1, 2], &[2, 1, 3]));
        assert!(!w29a15_key_less(&[2, 1, 3], &[2, 1, 2]));
        assert!(!w29a15_key_less(&[2, 1, 2], &[2, 1, 2]));
        assert!(w29a15_key_less(&[1, 5], &[2, 5, 0]));
        assert!(!w29a15_key_less(&[2, 5, 0], &[1, 5]));
    }

    #[test]
    fn heap() {
        let mut v = vec![3u32, 9, 1, 7, 5, 8, 2];
        w29a15_make_heap(&mut v, &mut |a, b| a < b);
        assert_eq!(v[0], 9);
        for i in 1..v.len() {
            assert!(v[(i - 1) / 2] >= v[i]);
        }
        let mut one = vec![4u32];
        w29a15_make_heap(&mut one, &mut |a, b| a < b);
        assert_eq!(one, vec![4]);
    }

    #[test]
    fn adjust() {
        let mut v = vec![0u32, 9, 8, 3, 4, 1, 2];
        w29a15_adjust_heap(&mut v, 0, 7, 5, &mut |a, b| a < b);
        assert_eq!(v[0], 9);
        for i in 1..v.len() {
            assert!(v[(i - 1) / 2] >= v[i]);
        }
    }

    #[test]
    fn fold() {
        let mut o = [9, 9];
        assert_eq!(w29a15_lookup_fold(&mut o, &mut |_| 6), 0);
        assert_eq!(o, [0, 0]);
        assert_eq!(w29a15_lookup_fold(&mut o, &mut |_| 0xd), 0);
        assert_eq!(w29a15_lookup_fold(&mut o, &mut |_| 5), 5);
    }
}
