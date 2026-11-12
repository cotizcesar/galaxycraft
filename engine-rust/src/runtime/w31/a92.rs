#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent92.rs
// ===========================================================================
// Wave 31 translation batch 92: MD5 finalisation, errno wrapper and a
// bounded 8-byte-pair push table.

/// MD5 context as used by the finaliser: bit count and the four state words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W31a92Md5Ctx {
    /// Total number of bits fed so far (low word first in the retail layout).
    pub bit_count: u64,
    /// A, B, C, D state words.
    pub state: [u32; 4],
}

/// MD5 initial state words.
pub const W31A92_MD5_INIT: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];

/// `0x00b8fbb0`: one-shot MD5 over `data`.  Starts from the standard IV with a
/// zero bit count, feeds `data` through `update`, then pads with the table
/// `0x80,0,0..` for `((-(lo >> 3) - 9) & 0x3f) + 1` bytes, feeds the 8-byte
/// little-endian bit count captured before padding, and returns the state.
pub fn w31a92_md5_finish(
    data: &[u8],
    mut update: impl FnMut(&mut W31a92Md5Ctx, &[u8]),
) -> [u32; 4] {
    let mut ctx = W31a92Md5Ctx {
        bit_count: 0,
        state: W31A92_MD5_INIT,
    };
    update(&mut ctx, data);
    let lo = ctx.bit_count as u32;
    let hi = (ctx.bit_count >> 32) as u32;
    let pad_len = ((0u32.wrapping_sub(lo >> 3).wrapping_sub(9)) & 0x3f) + 1;
    let mut pad = vec![0u8; pad_len as usize];
    pad[0] = 0x80;
    update(&mut ctx, &pad);
    let mut tail = [0u8; 8];
    tail[..4].copy_from_slice(&lo.to_le_bytes());
    tail[4..].copy_from_slice(&hi.to_le_bytes());
    update(&mut ctx, &tail);
    ctx.state
}

/// `0x00b90920`: runs the wrapped call, stores its returned code into errno
/// and returns `-1` if errno is now non-zero, else `0`.
pub fn w31a92_errno_result(call: impl FnOnce() -> u32, errno: &mut u32) -> i32 {
    *errno = call();
    -((*errno != 0) as i32)
}

/// `0x00b8fa60`: appends the pair `(a, b)` to the 0x200-entry table (8-byte
/// rows) when not full; returns whether it was stored.
pub fn w31a92_push_pair(table: &mut Vec<(u32, u32)>, a: u32, b: u32) -> bool {
    if table.len() == 0x200 {
        return false;
    }
    table.push((a, b));
    true
}

#[cfg(test)]
mod tests_wave31_agent92 {
    use super::*;

    #[test]
    fn md5_padding_lengths() {
        for n in [0usize, 1, 55, 56, 63, 64, 100] {
            let data = vec![7u8; n];
            let mut fed = Vec::new();
            w31a92_md5_finish(&data, |c, b| {
                c.bit_count += (b.len() as u64) * 8;
                fed.push(b.len());
            });
            assert_eq!(fed[0], n);
            assert_eq!(fed[2], 8);
            assert_eq!((n + fed[1] + 8) % 64, 0);
        }
    }

    #[test]
    fn errno_and_push() {
        let mut e = 5;
        assert_eq!(w31a92_errno_result(|| 0, &mut e), 0);
        assert_eq!(e, 0);
        assert_eq!(w31a92_errno_result(|| 9, &mut e), -1);
        let mut t = Vec::new();
        for i in 0..0x200 {
            assert!(w31a92_push_pair(&mut t, i, i));
        }
        assert!(!w31a92_push_pair(&mut t, 1, 1));
    }
}
