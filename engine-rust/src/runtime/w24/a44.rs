#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent44.rs
// ===========================================================================
// Wave 24 batch 44: small container helpers.

/// `0x007c8f30`: calls `f` once per element of a dword vector whose header
/// holds `begin` (+0xc) and `size_flags` (+0x10).  The count is the low 30
/// bits of the size; it is zero when `begin + size * 4` wraps a u32.
pub fn w24a44_for_each_dword(begin: u32, size_flags: u32, mut f: impl FnMut(u32)) -> u32 {
    let mut count = size_flags & 0x3fff_ffff;
    if begin.wrapping_add(size_flags.wrapping_mul(4)) < begin {
        count = 0;
    }
    for i in 0..count {
        f(i);
    }
    count
}

/// `0x007c93f0`: releases an owned object slot.  When the slot is set it is
/// cleared and the object's destructor (`drop_fn`) runs; returns whether a
/// slot was released.
pub fn w24a44_release_slot<T>(slot: &mut Option<T>, mut drop_fn: impl FnMut(T)) -> bool {
    match slot.take() {
        Some(obj) => {
            drop_fn(obj);
            true
        }
        None => false,
    }
}

/// Result of one `w24a44_append_range` step.
#[derive(Debug, PartialEq, Eq)]
pub enum W24a44Append {
    /// All source elements were appended.
    Done,
    /// The buffer is full; caller must reallocate to `new_cap` (the flags
    /// word has already been updated).
    Grow { new_cap: u32 },
}

/// `0x007c96a0`: appends `src` dwords to a vector (`data`, `size`, `flags`
/// where capacity is `flags & 0x7fffffff`).  When full and the capacity
/// can still grow, the new capacity is `cap + max(1, (flags >> 1) & 0x3fffffff)`
/// (1 when that sum would overflow) and the flags are updated.
pub fn w24a44_append_range(data: &mut Vec<u32>, flags: &mut u32, src: &[u32]) -> W24a44Append {
    for &v in src {
        let cap = *flags & 0x7fff_ffff;
        if data.len() as u32 == cap && cap.wrapping_add(1) > cap {
            let step = (*flags >> 1) & 0x3fff_ffff;
            let add = if cap.wrapping_add(1) <= cap.wrapping_add(step) {
                step
            } else {
                1
            };
            let new_cap = cap.wrapping_add(add);
            *flags ^= (new_cap ^ *flags) & 0x7fff_ffff;
            return W24a44Append::Grow { new_cap };
        }
        data.push(v);
    }
    W24a44Append::Done
}

#[cfg(test)]
mod tests_wave24_agent44 {
    use super::*;

    #[test]
    fn for_each_count() {
        let mut n = vec![];
        assert_eq!(w24a44_for_each_dword(0x1000, 3, |i| n.push(i)), 3);
        assert_eq!(n, vec![0, 1, 2]);
        assert_eq!(w24a44_for_each_dword(0xffff_fff0, 8, |_| {}), 0);
        assert_eq!(w24a44_for_each_dword(0, 0xc000_0002, |_| {}), 2);
    }

    #[test]
    fn release() {
        let mut s = Some(5);
        let mut got = 0;
        assert!(w24a44_release_slot(&mut s, |v| got = v));
        assert_eq!((s, got), (None, 5));
        assert!(!w24a44_release_slot(&mut s, |_| panic!()));
    }

    #[test]
    fn append() {
        let mut d = vec![];
        let mut fl = 2u32;
        assert_eq!(
            w24a44_append_range(&mut d, &mut fl, &[1, 2]),
            W24a44Append::Done
        );
        assert_eq!(
            w24a44_append_range(&mut d, &mut fl, &[3]),
            W24a44Append::Grow { new_cap: 3 }
        );
        assert_eq!(fl & 0x7fff_ffff, 3);
        assert_eq!(d, vec![1, 2]);
    }
}
