#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent10.rs
// ===========================================================================
// Wave 24 agent 10: 16-byte record introsort helpers and lobby flag glue.

/// One 16-byte sort record; the sort key is the dword at offset 8 (index 2).
pub type W24a10Rec = [u32; 4];

/// `0x00731110`: derives the packed lobby-flag word from the global flag
/// word `flags`: bit 0 is `flags >> 4 & 1`, bits 8..32 are `flags >> 12`.
pub fn w24a10_flag_word(flags: u32) -> u32 {
    let hi = (flags >> 12) & 0x00ff_ffff;
    let lo = (flags >> 4) & 0xff;
    ((hi << 8) | lo) & 0xffff_ff01
}

/// `0x00731140`: clears the lobby flag word and the "game created" flag,
/// then runs the unresolved reset callee.
pub fn w24a10_reset_flags(flags: &mut u32, created: &mut u32, reset: impl FnOnce()) {
    *flags = 0;
    *created = 0;
    reset();
}

/// `0x00732500`: copy-assigns a vector of 16-byte records (self-assignment
/// is a no-op in the retail code and impossible here).
pub fn w24a10_assign_records(dst: &mut Vec<W24a10Rec>, src: &[W24a10Rec]) {
    dst.clear();
    dst.extend_from_slice(src);
}

/// `0x007322e0`: heap sift (adjust-heap) over 16-byte records keyed on dword
/// 2.  Moves the hole from `hole` down to a leaf, then back up, and stores
/// `val` there.  `len` is the heap length.
pub fn w24a10_adjust_heap(a: &mut [W24a10Rec], hole: i32, len: i32, val: W24a10Rec) {
    let mut i = hole;
    let mut child;
    loop {
        child = i * 2;
        let right = child + 2;
        if len <= right {
            if right == len {
                a[i as usize] = a[(right - 1) as usize];
                i = child + 1;
            }
            break;
        }
        let mut pick = child + 1;
        if a[(right - 1) as usize][2] <= a[right as usize][2] {
            pick = right;
        }
        a[i as usize] = a[pick as usize];
        i = pick;
    }
    while hole < i {
        let parent = (i - 1) >> 1;
        if a[parent as usize][2] < val[2] {
            a[i as usize] = a[parent as usize];
            i = parent;
        } else {
            break;
        }
    }
    a[i as usize] = val;
}

fn w24a10_med3(a: &[W24a10Rec], x: usize, y: usize, z: usize) -> usize {
    let (kx, ky, kz) = (a[x][2], a[y][2], a[z][2]);
    if kx < ky {
        if ky < kz {
            return y;
        }
        let r = x;
        if kx < kz { z } else { r }
    } else {
        if kx < kz {
            return x;
        }
        if ky < kz { z } else { y }
    }
}

/// `0x007321c0`: median-of-medians (ninther) pivot choice over
/// `a[first..last]`; returns the chosen index.
pub fn w24a10_pick_pivot(a: &[W24a10Rec], first: usize, last: usize) -> usize {
    let n = last - first;
    let step = n / 8;
    let mid = first + n / 2;
    let m1 = w24a10_med3(a, first, first + step, first + 2 * step);
    // middle triple (literal min/max form)
    let (lo, hi) = (mid - step, mid + step);
    let (mut imin, mut imax) = (mid, lo);
    let (mut kmin, mut kmax) = (a[mid][2], a[lo][2]);
    if a[lo][2] < a[mid][2] {
        imin = lo;
        imax = mid;
        kmin = a[lo][2];
        kmax = a[mid][2];
    }
    let m2 = if a[hi][2] <= kmax {
        if kmin < a[hi][2] { hi } else { imin }
    } else {
        imax
    };
    let l1 = last - 1;
    let m3 = w24a10_med3(a, l1 - 2 * step, l1 - step, l1);
    let (k1, k2, k3) = (a[m1][2], a[m2][2], a[m3][2]);
    if k1 < k2 {
        if k2 < k3 {
            return m2;
        }
        if k1 < k3 {
            return m3;
        }
        m1
    } else if k3 <= k1 {
        if k3 <= k2 { m2 } else { m3 }
    } else {
        m1
    }
}

/// `0x00731eb0`: introsort over `a[first..last]` by dword 2 (ascending).
/// Quicksort with three-way-ish partition while `depth > 0` and more than
/// 32 records remain, insertion sort for <= 32, heap sort once the depth
/// budget is exhausted.
pub fn w24a10_intro_sort(a: &mut [W24a10Rec], first: usize, last: usize, depth: i32) {
    let mut first = first;
    let mut depth = depth;
    loop {
        let n = last - first;
        if n <= 32 || depth < 1 {
            if n <= 32 {
                for i in first + 1..last {
                    let v = a[i];
                    let mut j = i;
                    while j != first && v[2] < a[j - 1][2] {
                        a[j] = a[j - 1];
                        j -= 1;
                    }
                    a[j] = v;
                }
            } else {
                let mut count = n as i32;
                let base = &mut a[first..last];
                let mut h = count / 2 - 1;
                loop {
                    let v = base[h as usize];
                    w24a10_adjust_heap(base, h, count, v);
                    if h == 0 {
                        break;
                    }
                    h -= 1;
                }
                let mut end = n;
                while count > 1 {
                    let v = base[end - 1];
                    base[end - 1] = base[0];
                    count = (end - 1) as i32;
                    w24a10_adjust_heap(base, 0, count, v);
                    end -= 1;
                }
            }
            return;
        }
        let p = w24a10_pick_pivot(a, first, last);
        a.swap(p, last - 1);
        let pk = a[last - 1][2];
        let mut lo_eq = first;
        let mut hi_eq = last - 1;
        let mut i = first;
        let mut j = last - 1;
        loop {
            while a[i][2] < pk {
                i += 1;
            }
            loop {
                j -= 1;
                if a[j][2] <= pk || j == first {
                    break;
                }
            }
            if j <= i {
                break;
            }
            a.swap(i, j);
            if a[i][2] == pk {
                a.swap(i, lo_eq);
                lo_eq += 1;
            }
            if a[j][2] == pk {
                hi_eq -= 1;
                a.swap(j, hi_eq);
            }
            i += 1;
        }
        a.swap(i, last - 1);
        let mut left = first;
        let mut right = i;
        let mut left_next;
        loop {
            left_next = left + 1;
            right -= 1;
            if lo_eq <= left_next {
                break;
            }
            a.swap(left, right);
            left = left_next;
        }
        let mut k = last - 2;
        let mut pi = i;
        loop {
            let nxt = pi + 1;
            if hi_eq < k {
                a.swap(nxt, k);
                pi = nxt;
                k -= 1;
            } else {
                pi = nxt - 1;
                break;
            }
        }
        depth >>= 1;
        w24a10_intro_sort(a, first, right + 1, depth);
        first = pi + 1;
    }
}

#[cfg(test)]
mod tests_wave24_agent10 {
    use super::*;

    fn mkrecs(n: usize, seed: u32, modulo: u32) -> Vec<W24a10Rec> {
        let mut s = seed;
        (0..n)
            .map(|i| {
                s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                [i as u32, 7, (s >> 8) % modulo, 9]
            })
            .collect()
    }

    fn check(mut v: Vec<W24a10Rec>, depth: i32) {
        let mut exp: Vec<u32> = v.iter().map(|r| r[0]).collect();
        exp.sort();
        let n = v.len();
        w24a10_intro_sort(&mut v, 0, n, depth);
        assert!(v.windows(2).all(|w| w[0][2] <= w[1][2]));
        let mut got: Vec<u32> = v.iter().map(|r| r[0]).collect();
        got.sort();
        assert_eq!(got, exp);
    }

    #[test]
    fn flags() {
        assert_eq!(w24a10_flag_word(0), 0);
        assert_eq!(w24a10_flag_word(0x10), 1);
        assert_eq!(w24a10_flag_word(0x1000), 0x100);
        let (mut f, mut c) = (5, 6);
        let mut ran = false;
        w24a10_reset_flags(&mut f, &mut c, || ran = true);
        assert_eq!((f, c, ran), (0, 0, true));
    }

    #[test]
    fn assign() {
        let mut d = vec![[1, 2, 3, 4]; 3];
        w24a10_assign_records(&mut d, &[[9, 9, 9, 9]]);
        assert_eq!(d, vec![[9, 9, 9, 9]]);
    }

    #[test]
    fn heap_and_pivot() {
        let mut v = mkrecs(50, 3, 1000);
        let n = v.len() as i32;
        let mut cnt = n;
        for h in (0..n / 2).rev() {
            let x = v[h as usize];
            w24a10_adjust_heap(&mut v, h, cnt, x);
        }
        assert!((1..v.len()).all(|i| v[(i - 1) / 2][2] >= v[i][2]));
        cnt = 0;
        let _ = cnt;
        let p = w24a10_pick_pivot(&v, 0, v.len());
        assert!(p < v.len());
    }

    #[test]
    fn sorts() {
        for &(n, m) in &[
            (0, 5),
            (1, 5),
            (10, 4),
            (32, 100),
            (33, 100),
            (500, 1000),
            (700, 3),
        ] {
            check(mkrecs(n, n as u32 + 1, m), 40);
            check(mkrecs(n, n as u32 + 9, m), 0);
        }
    }
}
