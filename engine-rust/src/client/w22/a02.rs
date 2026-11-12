#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent02.rs
// ===========================================================================
// Wave 22 translation batch 02: client-side introsort and a 8-slot table.

/// Heap sift used by the sort below (same shape as the adjust-heap helper
/// of the introsort): sifts `value` into the heap rooted at `top` of the
/// first `len` elements of `v`.
fn w22a02_adjust_heap(v: &mut [u32], top: isize, len: isize, value: u32) {
    let mut hole = top;
    let mut child;
    loop {
        child = hole * 2;
        let right = child + 2;
        if len <= right {
            // `right` is out of range; fall through with `child` = left - 1.
            if right == len {
                v[hole as usize] = v[(right - 1) as usize];
                hole = child + 1;
            }
            break;
        }
        child += 1;
        if v[(right - 1) as usize] <= v[right as usize] {
            child = right;
        }
        v[hole as usize] = v[child as usize];
        hole = child;
    }
    if hole > top {
        loop {
            let parent = (hole - 1) >> 1;
            let pv = v[parent as usize];
            if value <= pv {
                break;
            }
            v[hole as usize] = pv;
            hole = parent;
            if top >= parent {
                break;
            }
        }
    }
    v[hole as usize] = value;
}

/// `0x00631430`: introsort over `v[begin..end]` (u32 keys).  Ranges of up
/// to 32 elements, or an exhausted `depth`, fall back to insertion sort
/// (<= 32) or heap sort; otherwise a three-way partition around the pivot
/// chosen by `median(v, begin, end)` (the unresolved callee returns the
/// pivot index) and the left part recurses with `depth / 2`.
pub fn w22a02_introsort(
    v: &mut [u32],
    begin: usize,
    end: usize,
    depth: i32,
    median: &dyn Fn(&[u32], usize, usize) -> usize,
) {
    let mut b = begin;
    let mut depth = depth;
    loop {
        let n = end - b;
        if !(n > 32 && depth >= 1) {
            if n < 33 {
                for i in b + 1..end {
                    let key = v[i];
                    let mut j = i;
                    while j != b && key < v[j - 1] {
                        v[j] = v[j - 1];
                        j -= 1;
                    }
                    v[j] = key;
                }
            } else {
                let cnt = n as isize;
                if cnt > 1 {
                    let s = &mut v[b..end];
                    let mut h = (cnt >> 1) - 1;
                    let val = s[h as usize];
                    w22a02_adjust_heap(s, h, cnt, val);
                    while h != 0 {
                        let val = s[(h - 1) as usize];
                        h -= 1;
                        w22a02_adjust_heap(s, h, cnt, val);
                    }
                    let mut e = cnt;
                    while e > 1 {
                        let val = s[(e - 1) as usize];
                        e -= 1;
                        s[e as usize] = s[0];
                        w22a02_adjust_heap(s, 0, e, val);
                    }
                }
            }
            return;
        }
        let start = b;
        let last = end - 1;
        let m = median(v, b, end);
        v.swap(m, last);
        let mut hi_eq = last;
        let mut lo_eq = b;
        let mut i = b;
        let mut j = last;
        let pivot;
        let old_i;
        loop {
            let p = v[last];
            while v[i] < p {
                i += 1;
            }
            loop {
                j -= 1;
                if v[j] <= p || j == b {
                    break;
                }
            }
            let vi = v[i];
            if j <= i {
                pivot = p;
                old_i = vi;
                break;
            }
            v[i] = v[j];
            v[j] = vi;
            let mut p2 = v[last];
            if v[i] == p2 {
                v.swap(lo_eq, i);
                p2 = v[last];
                lo_eq += 1;
            }
            let vj = v[j];
            if vj == p2 {
                hi_eq -= 1;
                v[j] = v[hi_eq];
                v[hi_eq] = vj;
            }
            i += 1;
        }
        v[i] = pivot;
        v[last] = old_i;
        let mut lo = b;
        let mut k = i;
        loop {
            k -= 1;
            if lo_eq <= lo + 1 {
                break;
            }
            v.swap(lo, k);
            lo += 1;
        }
        let mut f = i;
        let mut r = last - 1;
        loop {
            f += 1;
            if hi_eq >= r {
                break;
            }
            v.swap(f, r);
            r -= 1;
        }
        depth >>= 1;
        w22a02_introsort(v, start, k + 1, depth, median);
        b = f;
    }
}

/// `0x00635940`: reads byte 0 of record `index` in the 8-slot table whose
/// base is the global at faa424; record stride is `0x4e8`.  `table`
/// starts at that global.  `None` when out of range.
pub fn w22a02_slot_flag(table: &[u8], index: usize) -> Option<u8> {
    table.get(index.checked_mul(0x4e8)?).copied()
}

/// `0x006356b0`: clears the `0x2740`-byte (8 x `0x4e8`) slot table at the
/// global faa1f8 to zero.
pub fn w22a02_clear_slot_table(table: &mut [u8]) {
    let n = table.len().min(0x2740);
    table[..n].fill(0);
}

#[cfg(test)]
mod tests_wave22_agent02 {
    use super::*;

    fn mid(_: &[u32], b: usize, e: usize) -> usize {
        b + (e - b) / 2
    }

    fn sorted_check(mut v: Vec<u32>, depth: i32) {
        let mut want = v.clone();
        want.sort();
        let n = v.len();
        w22a02_introsort(&mut v, 0, n, depth, &mid);
        assert_eq!(v, want);
    }

    #[test]
    fn introsort_sorts() {
        let mut x = 12345u32;
        let mut rnd = |m: u32| {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            (x >> 8) % m
        };
        for &(n, m) in &[
            (0usize, 5u32),
            (1, 5),
            (10, 5),
            (33, 1000),
            (200, 7),
            (500, 100000),
        ] {
            let v: Vec<u32> = (0..n).map(|_| rnd(m)).collect();
            sorted_check(v.clone(), 20);
            sorted_check(v, 0);
        }
        sorted_check((0..300).collect(), 20);
        sorted_check((0..300).rev().collect(), 20);
        sorted_check(vec![4; 100], 20);
    }

    #[test]
    fn slot_table() {
        let mut t = vec![0u8; 0x2740];
        t[0x4e8 * 3] = 9;
        assert_eq!(w22a02_slot_flag(&t, 3), Some(9));
        assert_eq!(w22a02_slot_flag(&t, 8), None);
        w22a02_clear_slot_table(&mut t);
        assert_eq!(w22a02_slot_flag(&t, 3), Some(0));
    }
}
