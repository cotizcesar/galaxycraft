#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent121.rs
// ===========================================================================
// Wave 31 translation batch 121.
//
// Only the bzip2 fallback quicksort is real logic in this window; the rest
// is NVIDIA/driver-probe glue, CRT thunks or non-returning chains.

/// `0x00c7e9f0`: bzip2 `fallbackQSort3`.  Sorts `fmap[lo..=hi]` ascending by
/// `eclass[fmap[i]]` using a 3-way quicksort with an LCG pivot picker
/// (`r = r * 0x1dc5 + 1 & 0x7fff`) and a gap-4/gap-1 insertion sort for
/// ranges of at most 10 elements.  Returns `Err(0x3ec)` when the explicit
/// stack would exceed 0x62 entries (the retail panic code).
pub fn w31a121_fallback_qsort3(
    fmap: &mut [i32],
    eclass: &[u32],
    lo: i32,
    hi: i32,
) -> Result<(), u32> {
    let mut stack_lo = [0i32; 100];
    let mut stack_hi = [0i32; 100];
    stack_lo[0] = lo;
    stack_hi[0] = hi;
    let mut sp: usize = 1;
    let mut r: u32 = 0;
    let key = |fmap: &[i32], i: i32| eclass[fmap[i as usize] as usize];
    loop {
        if sp > 0x62 {
            return Err(0x3ec);
        }
        let lo = stack_lo[sp - 1];
        let hi = stack_hi[sp - 1];
        if hi - lo < 10 {
            if lo != hi {
                if hi - lo > 3 {
                    let mut i = hi - 4;
                    while i >= lo {
                        let v = fmap[i as usize];
                        let ev = eclass[v as usize];
                        let mut j = i + 4;
                        while j <= hi && key(fmap, j) < ev {
                            fmap[(j - 4) as usize] = fmap[j as usize];
                            j += 4;
                        }
                        fmap[(j - 4) as usize] = v;
                        i -= 1;
                    }
                }
                let mut i = hi - 1;
                while i >= lo {
                    let v = fmap[i as usize];
                    let ev = eclass[v as usize];
                    let mut j = i + 1;
                    while j <= hi && key(fmap, j) < ev {
                        fmap[(j - 1) as usize] = fmap[j as usize];
                        j += 1;
                    }
                    fmap[(j - 1) as usize] = v;
                    i -= 1;
                }
            }
            sp -= 1;
            if sp < 1 {
                return Ok(());
            }
            continue;
        }
        r = (r.wrapping_mul(0x1dc5).wrapping_add(1)) & 0x7fff;
        let pivot_idx = match r % 3 {
            0 => lo,
            1 => (lo + hi) >> 1,
            _ => hi,
        };
        let med = eclass[fmap[pivot_idx as usize] as usize];
        let (mut un_lo, mut lt_lo) = (lo, lo);
        let (mut un_hi, mut gt_hi) = (hi, hi);
        loop {
            while un_lo <= un_hi {
                let n = key(fmap, un_lo).wrapping_sub(med) as i32;
                if n == 0 {
                    fmap.swap(un_lo as usize, lt_lo as usize);
                    lt_lo += 1;
                    un_lo += 1;
                } else if n > 0 {
                    break;
                } else {
                    un_lo += 1;
                }
            }
            while un_lo <= un_hi {
                let n = key(fmap, un_hi).wrapping_sub(med) as i32;
                if n == 0 {
                    fmap.swap(un_hi as usize, gt_hi as usize);
                    gt_hi -= 1;
                    un_hi -= 1;
                } else if n < 0 {
                    break;
                } else {
                    un_hi -= 1;
                }
            }
            if un_lo > un_hi {
                break;
            }
            fmap.swap(un_lo as usize, un_hi as usize);
            un_lo += 1;
            un_hi -= 1;
        }
        if gt_hi < lt_lo {
            sp -= 1;
            if sp < 1 {
                return Ok(());
            }
            continue;
        }
        let n = (lt_lo - lo).min(un_lo - lt_lo);
        for k in 0..n {
            fmap.swap((lo + k) as usize, (un_lo - n + k) as usize);
        }
        let m = (hi - gt_hi).min(gt_hi - un_hi);
        for k in 0..m {
            fmap.swap((un_lo + k) as usize, (hi - m + 1 + k) as usize);
        }
        let n_hi = lo + un_lo - lt_lo - 1;
        let m_lo = hi - (gt_hi - un_hi) + 1;
        let top = sp - 1;
        if hi - m_lo < n_hi - lo {
            stack_lo[top] = m_lo;
            stack_hi[top] = hi;
            stack_lo[sp] = lo;
            stack_hi[sp] = n_hi;
        } else {
            stack_lo[top] = lo;
            stack_hi[top] = n_hi;
            stack_lo[sp] = m_lo;
            stack_hi[sp] = hi;
        }
        sp += 1;
    }
}

#[cfg(test)]
mod tests_wave31_agent121 {
    use super::*;

    #[test]
    fn sorts_by_key() {
        let n = 500usize;
        let mut s = 12345u32;
        let eclass: Vec<u32> = (0..n)
            .map(|_| {
                s = s.wrapping_mul(1103515245).wrapping_add(12345);
                (s >> 16) % 40
            })
            .collect();
        let mut fmap: Vec<i32> = (0..n as i32).collect();
        w31a121_fallback_qsort3(&mut fmap, &eclass, 0, n as i32 - 1).unwrap();
        for w in fmap.windows(2) {
            assert!(eclass[w[0] as usize] <= eclass[w[1] as usize]);
        }
        let mut sorted = fmap.clone();
        sorted.sort();
        assert_eq!(sorted, (0..n as i32).collect::<Vec<_>>());
    }

    #[test]
    fn small_and_single() {
        let eclass = [5u32, 3, 9, 1, 7, 2];
        let mut fmap = vec![0, 1, 2, 3, 4, 5];
        w31a121_fallback_qsort3(&mut fmap, &eclass, 0, 5).unwrap();
        assert_eq!(fmap, vec![3, 5, 1, 0, 4, 2]);
        let mut one = vec![0];
        w31a121_fallback_qsort3(&mut one, &[1], 0, 0).unwrap();
    }
}
