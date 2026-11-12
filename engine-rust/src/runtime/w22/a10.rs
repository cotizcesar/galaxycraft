#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent10.rs
// ===========================================================================
// Wave 22 translation batch 10.

/// `0x006238e0`: calls the helper with both arguments and returns the
/// first argument unchanged.
pub fn w22a10_call_return_first<T: Copy, U>(first: T, second: U, helper: impl FnOnce(T, U)) -> T {
    helper(first, second);
    first
}

/// Result of a small-vector tidy: whether the heap block must be released.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct W22a10Tidy {
    /// Element range `[0, len)` that was handed to the destructor, if any.
    pub destroyed: Option<usize>,
    /// True when the capacity field is non-negative (heap block owned).
    pub free_heap: bool,
}

/// `0x0061d820`: if the `active` flag is set, destroys `len` elements
/// (stride `0x1c`) and, when `cap >= 0`, requests the heap free; always
/// clears the flag.
pub fn w22a10_tidy_stride1c(
    active: &mut bool,
    len: u32,
    cap: i32,
    destroy: impl FnOnce(usize),
) -> W22a10Tidy {
    let mut out = W22a10Tidy {
        destroyed: None,
        free_heap: false,
    };
    if *active {
        let n = len as usize;
        destroy(n);
        out.destroyed = Some(n);
        out.free_heap = cap >= 0;
    }
    *active = false;
    out
}

/// `0x00625250`: destroys `len` elements (stride `0xc`) and reports whether
/// the heap block must be freed (`cap >= 0`).
pub fn w22a10_tidy_stride0c(len: u32, cap: i32, destroy: impl FnOnce(usize)) -> W22a10Tidy {
    let n = len as usize;
    destroy(n);
    W22a10Tidy {
        destroyed: Some(n),
        free_heap: cap >= 0,
    }
}

#[cfg(test)]
mod tests_wave22_agent10 {
    use super::*;

    #[test]
    fn first_arg_returned() {
        let mut seen = (0, 0);
        assert_eq!(w22a10_call_return_first(5, 9, |a, b| seen = (a, b)), 5);
        assert_eq!(seen, (5, 9));
    }

    #[test]
    fn stride1c() {
        let mut a = false;
        let r = w22a10_tidy_stride1c(&mut a, 3, 0, |_| panic!());
        assert_eq!(r.destroyed, None);
        let mut a = true;
        let mut got = 0;
        let r = w22a10_tidy_stride1c(&mut a, 3, -1, |n| got = n);
        assert_eq!((got, r.free_heap, a), (3, false, false));
        let mut a = true;
        assert!(w22a10_tidy_stride1c(&mut a, 0, 0, |_| {}).free_heap);
    }

    #[test]
    fn stride0c() {
        assert_eq!(
            w22a10_tidy_stride0c(2, 0, |_| {}),
            W22a10Tidy {
                destroyed: Some(2),
                free_heap: true
            }
        );
        assert!(!w22a10_tidy_stride0c(2, -5, |_| {}).free_heap);
    }
}
