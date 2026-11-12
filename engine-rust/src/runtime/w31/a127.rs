#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent127.rs
// ===========================================================================
// Wave 31 translation batch 127: CRT checked-string helpers.

/// EINVAL as stored in the CRT errno slot.
pub const W31A127_EINVAL: u32 = 0x16;

/// `0x00c97faf`: steps a byte cursor back by one; when `expected` is
/// non-zero and the byte now under the cursor differs, errno becomes EINVAL.
pub fn w31a127_unget_byte(cursor: &mut usize, buf: &[u8], expected: u8, errno: &mut u32) {
    *cursor = cursor.wrapping_sub(1);
    if expected != 0 && buf.get(*cursor).copied() != Some(expected) {
        *errno = W31A127_EINVAL;
    }
}

/// `0x00c97fd8`: steps a cursor back by two bytes; when `expected` is
/// non-zero and the little-endian word there differs, errno becomes EINVAL.
pub fn w31a127_unget_word(cursor: &mut usize, buf: &[u8], expected: u16, errno: &mut u32) {
    *cursor = cursor.wrapping_sub(2);
    let got = buf
        .get(*cursor..cursor.wrapping_add(2))
        .map(|b| u16::from_le_bytes([b[0], b[1]]));
    if expected != 0 && got != Some(expected) {
        *errno = W31A127_EINVAL;
    }
}

/// `0x00c98006`: returns true when the cursor value is non-zero, otherwise
/// sets errno to EINVAL and returns false.
pub fn w31a127_check_nonzero(value: usize, errno: &mut u32) -> bool {
    if value == 0 {
        *errno = W31A127_EINVAL;
        return false;
    }
    true
}

/// `0x00c98021`: bounds-checked copy (memcpy_s shape).  Returns 0 on
/// success or EINVAL; on a bad source or too-small destination the
/// destination is zero-filled.
pub fn w31a127_copy_checked(
    dst: Option<&mut [u8]>,
    src: Option<&[u8]>,
    count: usize,
    errno: &mut u32,
) -> u32 {
    if count == 0 {
        return 0;
    }
    let Some(dst) = dst else {
        *errno = W31A127_EINVAL;
        return W31A127_EINVAL;
    };
    if let Some(src) = src {
        if count <= dst.len() && count <= src.len() {
            dst[..count].copy_from_slice(&src[..count]);
            return 0;
        }
    }
    dst.fill(0);
    *errno = W31A127_EINVAL;
    W31A127_EINVAL
}

#[cfg(test)]
mod tests_wave31_agent127 {
    use super::*;

    #[test]
    fn unget() {
        let mut e = 0;
        let mut c = 3;
        w31a127_unget_byte(&mut c, &[0, 0, 7, 0], 7, &mut e);
        assert_eq!((c, e), (2, 0));
        w31a127_unget_byte(&mut c, &[0, 0, 7, 0], 9, &mut e);
        assert_eq!(e, 0x16);
        let mut c = 4;
        let mut e = 0;
        w31a127_unget_word(&mut c, &[0, 0, 1, 2], 0x0201, &mut e);
        assert_eq!((c, e), (2, 0));
    }

    #[test]
    fn copy() {
        let mut e = 0;
        let mut d = [9u8; 4];
        assert_eq!(
            w31a127_copy_checked(Some(&mut d), Some(&[1, 2]), 2, &mut e),
            0
        );
        assert_eq!(d, [1, 2, 9, 9]);
        assert_eq!(
            w31a127_copy_checked(Some(&mut d), Some(&[1; 8]), 8, &mut e),
            0x16
        );
        assert_eq!(d, [0; 4]);
        assert!(!w31a127_check_nonzero(0, &mut e));
    }
}
