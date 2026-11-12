#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent43.rs
// ===========================================================================
// Wave 24 translation batch 43: scalar-deleting destructor wrappers and a sender lookup.

/// Shared body of the MSVC scalar deleting destructor: run the destructor, then
/// release the storage when bit 0 of `flags` is set.  Returns whether it freed.
pub fn w24a43_scalar_delete(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    dtor();
    if flags & 1 != 0 {
        free();
        true
    } else {
        false
    }
}

/// `0x007c4d10`: scalar deleting destructor; calls the destructor and frees when `flags & 1`.
pub fn w24a43_sdd_007c4d10(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    w24a43_scalar_delete(flags, dtor, free)
}

/// `0x007c4d70`: scalar deleting destructor; calls the destructor and frees when `flags & 1`.
pub fn w24a43_sdd_007c4d70(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    w24a43_scalar_delete(flags, dtor, free)
}

/// `0x007c4da0`: scalar deleting destructor; calls the destructor and frees when `flags & 1`.
pub fn w24a43_sdd_007c4da0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    w24a43_scalar_delete(flags, dtor, free)
}

/// `0x007c4dd0`: scalar deleting destructor; calls the destructor and frees when `flags & 1`.
pub fn w24a43_sdd_007c4dd0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    w24a43_scalar_delete(flags, dtor, free)
}

/// `0x007c4e90`: scalar deleting destructor; calls the destructor and frees when `flags & 1`.
pub fn w24a43_sdd_007c4e90(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    w24a43_scalar_delete(flags, dtor, free)
}

/// `0x007c4ec0`: scalar deleting destructor; calls the destructor and frees when `flags & 1`.
pub fn w24a43_sdd_007c4ec0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    w24a43_scalar_delete(flags, dtor, free)
}

/// `0x007c4ef0`: scalar deleting destructor; calls the destructor and frees when `flags & 1`.
pub fn w24a43_sdd_007c4ef0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    w24a43_scalar_delete(flags, dtor, free)
}

/// `0x007c5080`: scalar deleting destructor; calls the destructor and frees when `flags & 1`.
pub fn w24a43_sdd_007c5080(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    w24a43_scalar_delete(flags, dtor, free)
}

/// `0x007c50b0`: scalar deleting destructor; calls the destructor and frees when `flags & 1`.
pub fn w24a43_sdd_007c50b0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    w24a43_scalar_delete(flags, dtor, free)
}

/// `0x007c50e0`: scalar deleting destructor; calls the destructor and frees when `flags & 1`.
pub fn w24a43_sdd_007c50e0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    w24a43_scalar_delete(flags, dtor, free)
}

/// `0x007c5110`: scalar deleting destructor; calls the destructor and frees when `flags & 1`.
pub fn w24a43_sdd_007c5110(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) -> bool {
    w24a43_scalar_delete(flags, dtor, free)
}

/// `0x007c5bf0`: clears a vector of 0x1c-byte elements: destroys each element
/// then sets the count to zero.  Returns the number destroyed.
pub fn w24a43_clear_vec(len: &mut usize, mut destroy: impl FnMut(usize)) -> usize {
    let n = *len;
    for i in 0..n {
        destroy(i);
    }
    *len = 0;
    n
}

/// `0x007c6360`: searches `senders` (4-byte slots) for the first slot whose
/// category (via `category_of`) equals `wanted`.  If found and the slot is
/// non-zero the sender is returned; if found but zero, or not found, the
/// original raises "Failed to find sender for category {0}".
pub fn w24a43_find_sender(
    senders: &[i32],
    wanted: i32,
    category_of: impl Fn(usize) -> i32,
) -> Result<i32, String> {
    for (i, &s) in senders.iter().enumerate() {
        if category_of(i) == wanted {
            if s != 0 {
                return Ok(s);
            }
            break;
        }
    }
    Err(format!("Failed to find sender for category {wanted}"))
}

#[cfg(test)]
mod tests_wave24_agent43 {
    use super::*;

    #[test]
    fn sdd_all() {
        let fs: [fn(u8, &mut u32, &mut u32) -> bool; 11] = [
            |f, d, r| w24a43_sdd_007c4d10(f, || *d += 1, || *r += 1),
            |f, d, r| w24a43_sdd_007c4d70(f, || *d += 1, || *r += 1),
            |f, d, r| w24a43_sdd_007c4da0(f, || *d += 1, || *r += 1),
            |f, d, r| w24a43_sdd_007c4dd0(f, || *d += 1, || *r += 1),
            |f, d, r| w24a43_sdd_007c4e90(f, || *d += 1, || *r += 1),
            |f, d, r| w24a43_sdd_007c4ec0(f, || *d += 1, || *r += 1),
            |f, d, r| w24a43_sdd_007c4ef0(f, || *d += 1, || *r += 1),
            |f, d, r| w24a43_sdd_007c5080(f, || *d += 1, || *r += 1),
            |f, d, r| w24a43_sdd_007c50b0(f, || *d += 1, || *r += 1),
            |f, d, r| w24a43_sdd_007c50e0(f, || *d += 1, || *r += 1),
            |f, d, r| w24a43_sdd_007c5110(f, || *d += 1, || *r += 1),
        ];
        for f in fs {
            let (mut d, mut r) = (0, 0);
            assert!(!f(0, &mut d, &mut r));
            assert_eq!((d, r), (1, 0));
            assert!(f(3, &mut d, &mut r));
            assert_eq!((d, r), (2, 1));
        }
    }

    #[test]
    fn clear_vec() {
        let mut n = 3;
        let mut seen = vec![];
        assert_eq!(w24a43_clear_vec(&mut n, |i| seen.push(i)), 3);
        assert_eq!((n, seen), (0, vec![0, 1, 2]));
    }

    #[test]
    fn find_sender() {
        let s = [0, 7, 9];
        let cat = |i: usize| i as i32 + 10;
        assert_eq!(w24a43_find_sender(&s, 11, cat), Ok(7));
        assert!(w24a43_find_sender(&s, 10, cat).is_err());
        assert_eq!(
            w24a43_find_sender(&s, 99, cat).unwrap_err(),
            "Failed to find sender for category 99"
        );
    }
}
