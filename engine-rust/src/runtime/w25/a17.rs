#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent17.rs
// ===========================================================================
// Wave 25 agent 17: MSVC `std::vector` sizing helpers (pure models).

/// Outcome of the resize planning shared by the vector resize routines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W25a17ResizePlan {
    /// Shrink: the end pointer moves to `begin + n` elements (no-op when equal).
    Shrink(u32),
    /// Fits in capacity: append `n` fill elements at the end.
    Append(u32),
    /// Reallocate with this new capacity (elements), then fill/copy.
    Grow(u32),
    /// Request exceeds the element limit (length error path).
    TooLong,
}

/// Shared growth policy: capacity grows by 1.5x unless `wanted` is larger.
fn w25a17_plan(len: u32, cap: u32, wanted: u32, max: u32) -> W25a17ResizePlan {
    if wanted <= cap {
        if wanted <= len {
            return W25a17ResizePlan::Shrink(wanted);
        }
        return W25a17ResizePlan::Append(wanted - len);
    }
    if wanted > max {
        return W25a17ResizePlan::TooLong;
    }
    let mut new_cap = wanted;
    if cap <= max - (cap >> 1) {
        new_cap = (cap >> 1) + cap;
        if new_cap < wanted {
            new_cap = wanted;
        }
    }
    W25a17ResizePlan::Grow(new_cap)
}

/// `0x007f39d0`: resize plan of an 8-byte-element vector (zero fill),
/// element limit `0x1fffffff`.  Pointers are begin/end/cap byte offsets.
pub fn w25a17_resize8_plan(begin: u32, end: u32, cap: u32, wanted: u32) -> W25a17ResizePlan {
    w25a17_plan(
        end.wrapping_sub(begin) >> 3,
        cap.wrapping_sub(begin) >> 3,
        wanted,
        0x1fff_ffff,
    )
}

/// `0x007f3ca0`: same plan as the zero-filling 8-byte vector resize, via the
/// other allocator (element limit `0x1fffffff`).
pub fn w25a17_resize8_plan_alt(begin: u32, end: u32, cap: u32, wanted: u32) -> W25a17ResizePlan {
    w25a17_plan(
        end.wrapping_sub(begin) >> 3,
        cap.wrapping_sub(begin) >> 3,
        wanted,
        0x1fff_ffff,
    )
}

/// `0x007f3b60`: resize of a 4-byte-element vector with fill value
/// (limit `0x3fffffff`); returns the plan.
pub fn w25a17_resize4_plan(begin: u32, end: u32, cap: u32, wanted: u32) -> W25a17ResizePlan {
    w25a17_plan(
        end.wrapping_sub(begin) >> 2,
        cap.wrapping_sub(begin) >> 2,
        wanted,
        0x3fff_ffff,
    )
}

/// Apply a plan to a `Vec<u32>` using `fill` (the semantics of `0x007f3b60`).
pub fn w25a17_resize4_apply(v: &mut Vec<u32>, wanted: usize, fill: u32) -> bool {
    let cap = v.capacity().min(u32::MAX as usize) as u32;
    let plan = w25a17_plan(v.len() as u32, cap, wanted as u32, 0x3fff_ffff);
    match plan {
        W25a17ResizePlan::TooLong => false,
        _ => {
            v.resize(wanted, fill);
            true
        }
    }
}

/// Copy-construct sizing: returns `Some(capacity_elems)` to allocate, `None`
/// for an empty source, `Err` when `n >= limit`.
fn w25a17_copy_size(bytes: u32, shift: u32, limit: u32) -> Result<Option<u32>, ()> {
    let n = bytes >> shift;
    if n == 0 {
        return Ok(None);
    }
    if n < limit { Ok(Some(n)) } else { Err(()) }
}

/// `0x007f3950`: byte-range copy constructor sizing (`end - begin` bytes,
/// limit `0x80000000`).
pub fn w25a17_copy_bytes_size(begin: u32, end: u32) -> Result<Option<u32>, ()> {
    w25a17_copy_size(end.wrapping_sub(begin), 0, 0x8000_0000)
}

/// `0x007f4630`: copy constructor sizing of a 4-byte-element vector
/// (limit `0x40000000`).
pub fn w25a17_copy4_size(begin: u32, end: u32) -> Result<Option<u32>, ()> {
    w25a17_copy_size(end.wrapping_sub(begin), 2, 0x4000_0000)
}

/// `0x007f4730`: copy constructor sizing of an 8-byte-element vector
/// (limit `0x20000000`).
pub fn w25a17_copy8_size(begin: u32, end: u32) -> Result<Option<u32>, ()> {
    w25a17_copy_size(end.wrapping_sub(begin), 3, 0x2000_0000)
}

/// Result of the MSVC big-block deallocation size logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W25a17Free {
    /// Pointer actually released.
    pub ptr: u32,
    /// Size passed to the deallocator.
    pub size: u32,
    /// Alignment-header sanity check failed (fatal path).
    pub corrupt: bool,
}

/// Shared deallocate model: blocks over `0xfff` bytes carry a back pointer
/// at `ptr - 4`, size gains `0x23`, and the header must lie within 0x23 bytes.
fn w25a17_dealloc(ptr: u32, bytes: u32, header: impl Fn(u32) -> u32) -> W25a17Free {
    if bytes > 0xfff {
        let real = header(ptr);
        let corrupt = ptr.wrapping_sub(real).wrapping_sub(4) > 0x1f;
        W25a17Free {
            ptr: real,
            size: bytes + 0x23,
            corrupt,
        }
    } else {
        W25a17Free {
            ptr,
            size: bytes,
            corrupt: false,
        }
    }
}

/// `0x007f48d0` and `0x007f49a0`: free a 4-byte-element buffer
/// (`bytes = (end - begin) & !3`); `None` when `begin == 0`.
pub fn w25a17_free4(begin: u32, end: u32, header: impl Fn(u32) -> u32) -> Option<W25a17Free> {
    if begin == 0 {
        return None;
    }
    Some(w25a17_dealloc(begin, end.wrapping_sub(begin) & !3, header))
}

/// `0x007f4930`: free an 8-byte-element buffer (`bytes & !7`); `None` when the
/// buffer is null (the caller then runs the two fallback releases).
pub fn w25a17_free8(begin: u32, end: u32, header: impl Fn(u32) -> u32) -> Option<W25a17Free> {
    if begin == 0 {
        return None;
    }
    Some(w25a17_dealloc(begin, end.wrapping_sub(begin) & !7, header))
}

/// `0x007f4c60`: free a byte buffer (`bytes = cap - begin`, no mask).
pub fn w25a17_free_bytes(begin: u32, cap: u32, header: impl Fn(u32) -> u32) -> Option<W25a17Free> {
    if begin == 0 {
        return None;
    }
    Some(w25a17_dealloc(begin, cap.wrapping_sub(begin), header))
}

/// `0x007f4a00`: chooses which buffer to free: the 8-byte-element one at
/// `+0x14` (mask `!7`, end at `+0x1c`) when present, otherwise the 4-byte one
/// at `+4` (mask `!3`, end at `+0xc`).  Arguments are (begin, end) pairs.
pub fn w25a17_free_either(
    a: (u32, u32),
    b: (u32, u32),
    header: impl Fn(u32) -> u32,
) -> Option<W25a17Free> {
    if a.0 != 0 {
        return w25a17_free8(a.0, a.1, header);
    }
    w25a17_free4(b.0, b.1, header)
}

#[cfg(test)]
mod tests_wave25_agent17 {
    use super::*;

    #[test]
    fn resize_plans() {
        // 8-byte: len 2, cap 4
        assert_eq!(
            w25a17_resize8_plan(0, 16, 32, 1),
            W25a17ResizePlan::Shrink(1)
        );
        assert_eq!(
            w25a17_resize8_plan(0, 16, 32, 3),
            W25a17ResizePlan::Append(1)
        );
        assert_eq!(
            w25a17_resize8_plan(0, 16, 32, 10),
            W25a17ResizePlan::Grow(10)
        );
        assert_eq!(w25a17_resize8_plan(0, 16, 32, 5), W25a17ResizePlan::Grow(6));
        assert_eq!(
            w25a17_resize8_plan(0, 16, 32, 0x2000_0000),
            W25a17ResizePlan::TooLong
        );
        assert_eq!(
            w25a17_resize8_plan_alt(0, 16, 32, 5),
            w25a17_resize8_plan(0, 16, 32, 5)
        );
        assert_eq!(
            w25a17_resize4_plan(0, 8, 16, 0x4000_0000),
            W25a17ResizePlan::TooLong
        );
        assert_eq!(w25a17_resize4_plan(0, 8, 16, 5), W25a17ResizePlan::Grow(6));
    }

    #[test]
    fn resize4_apply() {
        let mut v = vec![1u32, 2];
        assert!(w25a17_resize4_apply(&mut v, 4, 9));
        assert_eq!(v, [1, 2, 9, 9]);
        assert!(w25a17_resize4_apply(&mut v, 1, 0));
        assert_eq!(v, [1]);
        assert!(!w25a17_resize4_apply(&mut v, 0x4000_0000, 0));
    }

    #[test]
    fn copy_sizes() {
        assert_eq!(w25a17_copy_bytes_size(5, 5), Ok(None));
        assert_eq!(w25a17_copy_bytes_size(0, 10), Ok(Some(10)));
        assert!(w25a17_copy_bytes_size(0, 0x8000_0000).is_err());
        assert_eq!(w25a17_copy4_size(0, 12), Ok(Some(3)));
        assert_eq!(w25a17_copy4_size(0, 0xffff_fffc), Ok(Some(0x3fff_ffff)));
        assert_eq!(w25a17_copy8_size(0, 24), Ok(Some(3)));
        assert_eq!(w25a17_copy8_size(0, 4), Ok(None));
    }

    #[test]
    fn frees() {
        let hdr = |p: u32| p - 8;
        assert_eq!(w25a17_free4(0, 100, hdr), None);
        let f = w25a17_free4(100, 100 + 0xfff, hdr).unwrap();
        assert_eq!((f.ptr, f.size, f.corrupt), (100, 0xffc, false));
        let f = w25a17_free4(100, 100 + 0x1004, hdr).unwrap();
        assert_eq!((f.ptr, f.size, f.corrupt), (92, 0x1004 + 0x23, false));
        let f = w25a17_free4(100, 100 + 0x1004, |p| p - 0x40).unwrap();
        assert!(f.corrupt);
        let f = w25a17_free8(100, 100 + 0xfff, hdr).unwrap();
        assert_eq!(f.size, 0xff8);
        let f = w25a17_free_bytes(100, 100 + 0x1000, hdr).unwrap();
        assert_eq!(f.size, 0x1023);
        assert_eq!(w25a17_free_bytes(100, 110, hdr).unwrap().size, 10);
        let e = w25a17_free_either((0, 0), (200, 208), hdr).unwrap();
        assert_eq!(e.size, 8);
        let e = w25a17_free_either((200, 224), (300, 304), hdr).unwrap();
        assert_eq!(e.size, 24);
        assert_eq!(w25a17_free_either((0, 0), (0, 0), hdr), None);
    }
}
