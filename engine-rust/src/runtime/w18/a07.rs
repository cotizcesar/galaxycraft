#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave18/agent07.rs
// ===========================================================================
// Wave 18 translation batch 07: 12-byte vector header helpers and MSVC
// `std::string` / list sentinel idioms from the runtime container area.

/// Error for paths where retail calls a non-returning (allocator/throw) routine.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W18a07Unrecovered {
    /// Growth needs a reallocation; carries the new capacity written back.
    Grow(u32),
    /// Source uses the heap-flag representation; callee not recovered.
    HeapSource,
}

/// Vector of 12-byte elements with `{begin, size, cap|flag}` header.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct W18a07Vec12 {
    pub data: Vec<[u32; 3]>,
    /// Header word 2: low 31 bits capacity, bit 31 flag (preserved).
    pub cap_flags: u32,
}

/// `0x00538f90`: resize to `n` elements.  Shrinking erases the tail;
/// growing within capacity zero-fills; growing beyond capacity updates the
/// capacity to `max(cap + cap/2, n)` (flag bit kept) then calls an
/// unrecovered allocator, reported as `Err(Grow(newcap))`.
pub fn w18a07_vec12_resize(v: &mut W18a07Vec12, n: usize) -> Result<(), W18a07Unrecovered> {
    let len = v.data.len();
    if n < len {
        v.data.truncate(n);
    } else if len < n {
        let cap = v.cap_flags & 0x7fff_ffff;
        if (cap as usize) < n {
            let mut new_cap = (v.cap_flags >> 1 & 0x3fff_ffff).wrapping_add(cap);
            if (new_cap as usize) < n {
                new_cap = n as u32;
            }
            v.cap_flags = (v.cap_flags ^ new_cap) & 0x7fff_ffff ^ v.cap_flags;
            return Err(W18a07Unrecovered::Grow(new_cap));
        }
        v.data.resize(n, [0; 3]);
    }
    Ok(())
}

/// `0x0053a6b0`: move-construct a `{ptr,size,cap|flag}` header.  Only the
/// non-flagged source (bit 31 clear) is recovered: destination takes the
/// fields, source keeps only its flag bit and is zeroed.
pub fn w18a07_move_header(src: &mut [u32; 3]) -> Result<[u32; 3], W18a07Unrecovered> {
    if (src[2] as i32) < 0 {
        return Err(W18a07Unrecovered::HeapSource);
    }
    let dst = [src[0], src[1], src[2] & 0x7fff_ffff];
    src[2] &= 0x8000_0000;
    src[0] = 0;
    src[1] = 0;
    Ok(dst)
}

/// `0x0053ab00`: reset a list head `[next, prev, size]` to the empty
/// sentinel (`self` links, size 0); returns the previous `prev` link, which
/// retail then walks to destroy nodes (unrecovered).
pub fn w18a07_list_reset_head(head: &mut [u32; 3], self_addr: u32) -> u32 {
    let old = head[1];
    head[1] = self_addr;
    head[0] = self_addr;
    head[2] = 0;
    old
}

/// `0x0053ae10`: same sentinel reset as the sibling above; returns whether
/// any node chain was left to destroy.
pub fn w18a07_list_reset_head_dirty(head: &mut [u32; 3], self_addr: u32) -> bool {
    w18a07_list_reset_head(head, self_addr) != self_addr
}

/// `0x0053b6a0`: bit 0 of the flag byte selects the free path.
pub fn w18a07_needs_free(flag: u8) -> bool {
    flag & 1 != 0
}

/// `0x00539c80`: scan 0x1c-byte records for the first whose word at +8 is
/// non-negative (retail then runs unrecovered cleanup); `None` if the range
/// is exhausted.  `words` holds that word per record.
pub fn w18a07_first_nonneg_record(words: &[i32]) -> Option<usize> {
    words.iter().position(|&w| -1 < w)
}

#[cfg(test)]
mod tests_wave18_agent07 {
    use super::*;

    #[test]
    fn resize_paths() {
        let mut v = W18a07Vec12 {
            data: vec![[1, 2, 3]; 3],
            cap_flags: 4,
        };
        assert_eq!(w18a07_vec12_resize(&mut v, 2), Ok(()));
        assert_eq!(v.data.len(), 2);
        assert_eq!(w18a07_vec12_resize(&mut v, 4), Ok(()));
        assert_eq!(v.data[3], [0; 3]);
        assert_eq!(
            w18a07_vec12_resize(&mut v, 5),
            Err(W18a07Unrecovered::Grow(6))
        );
        assert_eq!(v.cap_flags, 6);
        v.cap_flags = 0x8000_0004;
        assert_eq!(
            w18a07_vec12_resize(&mut v, 20),
            Err(W18a07Unrecovered::Grow(20))
        );
        assert_eq!(v.cap_flags, 0x8000_0014);
    }

    #[test]
    fn move_header_cases() {
        let mut s = [10, 5, 8];
        assert_eq!(w18a07_move_header(&mut s), Ok([10, 5, 8]));
        assert_eq!(s, [0, 0, 0]);
        let mut h = [1, 2, 0x8000_0003];
        assert_eq!(
            w18a07_move_header(&mut h),
            Err(W18a07Unrecovered::HeapSource)
        );
    }

    #[test]
    fn list_and_flags() {
        let mut h = [9, 9, 9];
        assert_eq!(w18a07_list_reset_head(&mut h, 0x100), 9);
        assert_eq!(h, [0x100, 0x100, 0]);
        assert!(!w18a07_list_reset_head_dirty(&mut h, 0x100));
        assert!(w18a07_list_reset_head_dirty(&mut [0, 7, 0], 0x100));
        assert!(w18a07_needs_free(3) && !w18a07_needs_free(2));
        assert_eq!(w18a07_first_nonneg_record(&[-1, -5, 0, 3]), Some(2));
        assert_eq!(w18a07_first_nonneg_record(&[-1]), None);
    }
}
