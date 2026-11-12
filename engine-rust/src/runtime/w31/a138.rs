#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent138.rs
// ===========================================================================
// Wave 31 agent 138: CRT allocator wrappers and errno getters.

/// errno value for out of memory (`ENOMEM`).
pub const W31A138_ENOMEM: u32 = 0xc;
/// errno value for invalid argument (`EINVAL`).
pub const W31A138_EINVAL: u32 = 0x16;
/// Largest request the CRT heap accepts.
pub const W31A138_MAX_ALLOC: u32 = 0xffff_ffe0;

/// `0x00caea7e`: malloc.  Requests above 0xffffffe0 fail with ENOMEM; zero
/// is promoted to 1; the heap call is retried while the new-handler says so.
/// Returns `(ptr, errno)`; ptr is 0 on failure.
pub fn w31a138_malloc(
    size: u32,
    mut heap_alloc: impl FnMut(u32) -> u32,
    mut retry: impl FnMut(u32) -> bool,
) -> (u32, u32) {
    if size <= W31A138_MAX_ALLOC {
        let n = if size == 0 { 1 } else { size };
        loop {
            let p = heap_alloc(n);
            if p != 0 {
                return (p, 0);
            }
            if !retry(n) {
                break;
            }
        }
    }
    (0, W31A138_ENOMEM)
}

/// `0x00cae9a9`: calloc.  Fails when `count * size` would exceed the limit;
/// zero total is promoted to 1; heap call is zero-filled (flag 8).
pub fn w31a138_calloc(
    count: u32,
    size: u32,
    mut heap_alloc_zero: impl FnMut(u32) -> u32,
    mut retry: impl FnMut(u32) -> bool,
) -> (u32, u32) {
    if count == 0 || size <= W31A138_MAX_ALLOC / count {
        let total = count.wrapping_mul(size);
        let n = if total == 0 { 1 } else { total };
        loop {
            let p = heap_alloc_zero(n);
            if p != 0 {
                return (p, 0);
            }
            if !retry(n) {
                break;
            }
        }
    }
    (0, W31A138_ENOMEM)
}

/// `0x00cae11e`: free.  Null is ignored; on heap failure returns the errno
/// to store.
pub fn w31a138_free(ptr: u32, heap_free: impl FnOnce(u32) -> bool, last_error: u32) -> Option<u32> {
    if ptr != 0 && !heap_free(ptr) {
        return Some(last_error);
    }
    None
}

/// `0x00cae158`: realloc.  Null pointer behaves as malloc; zero size frees
/// and returns 0; oversize fails with ENOMEM.  Returns `(ptr, errno)`.
pub fn w31a138_realloc(
    ptr: u32,
    size: u32,
    mut heap_realloc: impl FnMut(u32, u32) -> u32,
    mut retry: impl FnMut(u32) -> bool,
    heap_alloc: impl FnMut(u32) -> u32,
    heap_free: impl FnOnce(u32) -> bool,
    last_error: u32,
) -> (u32, u32) {
    if ptr == 0 {
        return w31a138_malloc(size, heap_alloc, retry);
    }
    if size == 0 {
        return (0, w31a138_free(ptr, heap_free, last_error).unwrap_or(0));
    }
    if size <= W31A138_MAX_ALLOC {
        loop {
            let p = heap_realloc(ptr, size);
            if p != 0 {
                return (p, 0);
            }
            if !retry(size) {
                break;
            }
        }
    }
    (0, W31A138_ENOMEM)
}

fn w31a138_getter(out: Option<&mut u32>, value: u32) -> u32 {
    match out {
        None => W31A138_EINVAL,
        Some(o) => {
            *o = value;
            0
        }
    }
}

/// `0x00cafacb`: stores global DAT_0140fe50 into `out`; null gives EINVAL.
pub fn w31a138_get_global_fe50(out: Option<&mut u32>, value: u32) -> u32 {
    w31a138_getter(out, value)
}

/// `0x00cafaf7`: stores global DAT_0140fe54 into `out`; null gives EINVAL.
pub fn w31a138_get_global_fe54(out: Option<&mut u32>, value: u32) -> u32 {
    w31a138_getter(out, value)
}

/// `0x00cafb23`: stores global DAT_0140fe4c into `out`; null gives EINVAL.
pub fn w31a138_get_global_fe4c(out: Option<&mut u32>, value: u32) -> u32 {
    w31a138_getter(out, value)
}

#[cfg(test)]
mod tests_wave31_agent138 {
    use super::*;

    #[test]
    fn malloc_cases() {
        assert_eq!(w31a138_malloc(0xffff_ffe1, |_| 5, |_| false), (0, 0xc));
        let mut seen = 0;
        assert_eq!(
            w31a138_malloc(
                0,
                |n| {
                    seen = n;
                    9
                },
                |_| false
            ),
            (9, 0)
        );
        assert_eq!(seen, 1);
        let mut tries = 0;
        let r = w31a138_malloc(
            4,
            |_| {
                tries += 1;
                if tries == 3 { 7 } else { 0 }
            },
            |_| true,
        );
        assert_eq!(r, (7, 0));
        assert_eq!(w31a138_malloc(4, |_| 0, |_| false), (0, 0xc));
    }

    #[test]
    fn calloc_cases() {
        assert_eq!(w31a138_calloc(0x10000, 0x10000, |_| 1, |_| false), (0, 0xc));
        let mut seen = 0;
        assert_eq!(
            w31a138_calloc(
                0,
                5,
                |n| {
                    seen = n;
                    3
                },
                |_| false
            ),
            (3, 0)
        );
        assert_eq!(seen, 1);
        assert_eq!(w31a138_calloc(4, 8, |n| n, |_| false), (32, 0));
    }

    #[test]
    fn free_realloc_cases() {
        assert_eq!(w31a138_free(0, |_| false, 5), None);
        assert_eq!(w31a138_free(1, |_| false, 5), Some(5));
        assert_eq!(w31a138_free(1, |_| true, 5), None);
        let r = w31a138_realloc(0, 8, |_, _| 0, |_| false, |_| 11, |_| true, 0);
        assert_eq!(r, (11, 0));
        let r = w31a138_realloc(2, 0, |_, _| 0, |_| false, |_| 0, |_| true, 0);
        assert_eq!(r, (0, 0));
        let r = w31a138_realloc(2, 0xffff_fff0, |_, _| 1, |_| false, |_| 0, |_| true, 0);
        assert_eq!(r, (0, 0xc));
        let r = w31a138_realloc(2, 16, |p, n| p + n, |_| false, |_| 0, |_| true, 0);
        assert_eq!(r, (18, 0));
    }

    #[test]
    fn getters() {
        let mut v = 0;
        assert_eq!(w31a138_get_global_fe50(None, 1), 0x16);
        assert_eq!(w31a138_get_global_fe50(Some(&mut v), 1), 0);
        assert_eq!(v, 1);
        assert_eq!(w31a138_get_global_fe54(Some(&mut v), 2), 0);
        assert_eq!(v, 2);
        assert_eq!(w31a138_get_global_fe4c(Some(&mut v), 3), 0);
        assert_eq!(v, 3);
        assert_eq!(w31a138_get_global_fe4c(None, 3), 0x16);
    }
}
