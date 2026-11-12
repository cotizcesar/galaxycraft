#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent09.rs
// ===========================================================================
// Wave 26 agent 09: Storm-style crypt table, block decrypt and cached read helpers.

/// `0x00924440`: builds the 0x500-entry crypt table (five rows of 256
/// words) with the LCG `x * 0x7d + 3 mod 0x2aaaab`, seed `0x100001`.
pub fn w26a09_build_crypt_table() -> [u32; 0x500] {
    let step = |x: u32| x.wrapping_mul(0x7d).wrapping_add(3) % 0x2aaaab;
    let mut t = [0u32; 0x500];
    let mut s: u32 = 0x100001;
    for i in 0..0x100 {
        for row in 0..5 {
            let a = step(s);
            let b = step(a);
            t[row * 0x100 + i] = (a << 16) | (b & 0xffff);
            s = b;
        }
    }
    t
}

/// `0x009258b0`: decrypts `len_bytes / 4` words in place with `key`,
/// using row 4 of the crypt table.
pub fn w26a09_decrypt_block(table: &[u32; 0x500], data: &mut [u32], len_bytes: u32, key: u32) {
    let mut acc: u32 = 0xeeeeeeeeu32; // -0x11111112
    let mut k = key;
    for w in data.iter_mut().take((len_bytes >> 2) as usize) {
        acc = acc.wrapping_add(table[0x400 + (k & 0xff) as usize]);
        *w ^= acc.wrapping_add(k);
        acc = acc.wrapping_mul(0x21).wrapping_add(*w).wrapping_add(3);
        k = (k >> 11) | (!k).wrapping_mul(0x200000).wrapping_add(0x11111111);
    }
}

/// `0x00923fa0`: allocation retry loop.  `alloc` returns nonzero on
/// success; on failure `new_handler` is asked to run a handler (returns
/// false when none is installed, ending the loop with 0).
pub fn w26a09_alloc_retry(
    mut alloc: impl FnMut() -> u32,
    mut new_handler: impl FnMut() -> bool,
) -> u32 {
    loop {
        let r = alloc();
        if r != 0 {
            return r;
        }
        if !new_handler() {
            return 0;
        }
    }
}

/// `0x00924020`: bounded-format result fixup.  `formatted_len` is the raw
/// length from the formatter (negative becomes `u32::MAX`).  With a
/// finite `size`, an overflowing result truncates to `size - 1` and
/// NUL-terminates `buf`.  `size == 0` returns 0; `0x7fffffff` is unbounded.
pub fn w26a09_bounded_format_result(buf: &mut [u8], size: u32, formatted_len: i32) -> u32 {
    if size == 0 {
        return 0;
    }
    let mut n = if formatted_len < 0 {
        u32::MAX
    } else {
        formatted_len as u32
    };
    if size != 0x7fffffff && size <= n {
        n = size - 1;
        if let Some(b) = buf.get_mut(n as usize) {
            *b = 0;
        }
    }
    n
}

/// Cached sequential-read state of a file object (offsets from `0x00926ad0`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct W26a09ReadCache {
    /// +0x208 offset of the cached read.
    pub cached_off: u32,
    /// +0x20c result of the cached read.
    pub cached_val: u32,
    /// +0x12c file size.
    pub size: u32,
    /// +0x130 flags.
    pub flags: u32,
}

/// `0x00926ad0`: returns the cached result if `off` matches, otherwise
/// clamps the chunk (`chunk` = parent+0x268) to the file size unless
/// flags & 0xff00 is set, performs the read and caches it.
pub fn w26a09_cached_read(
    c: &mut W26a09ReadCache,
    off: u32,
    chunk: u32,
    mut read: impl FnMut(u32, u32) -> u32,
) -> u32 {
    if c.cached_off == off {
        return c.cached_val;
    }
    let mut len = chunk;
    if c.size < len.wrapping_add(off) && (c.flags & 0xff00) == 0 {
        len = c.size.wrapping_sub(off);
    }
    c.cached_off = off;
    c.cached_val = read(off, len);
    c.cached_val
}

/// `0x00927a90`: reads `count` bytes at 64-bit offset `(lo, hi)` through
/// `read` (returns bytes read, or `None` on failure).  Returns
/// `(ok, bytes_read, end_offset)`; `None` when the reader failed.
pub fn w26a09_read_checked(
    lo: u32,
    hi: u32,
    count: u32,
    mut read: impl FnMut(u32, u32, u32) -> Option<u32>,
) -> Option<(bool, u32, u64)> {
    let got = read(lo, hi, count)?;
    let end = (((hi as u64) << 32) | lo as u64).wrapping_add(got as u64);
    Some((got == count, got, end))
}

#[cfg(test)]
mod tests_wave26_agent09 {
    use super::*;

    #[test]
    fn crypt_table_known_values() {
        let t = w26a09_build_crypt_table();
        let step = |x: u32| x.wrapping_mul(0x7d).wrapping_add(3) % 0x2aaaab;
        let a = step(0x100001);
        let b = step(a);
        assert_eq!(t[0], (a << 16) | (b & 0xffff));
        assert_eq!(t[0], 0x55c636e2);
        assert_eq!(t[0x100], 1996014001);
        let c = step(b);
        let d = step(c);
        assert_eq!(t[0x100], (c << 16) | (d & 0xffff));
        assert_eq!(t.len(), 0x500);
    }

    #[test]
    fn decrypt_changes_and_is_len_bounded() {
        let t = w26a09_build_crypt_table();
        let mut d = [0u32; 4];
        w26a09_decrypt_block(&t, &mut d, 8, 0x1234);
        assert_ne!(d[0], 0);
        assert_ne!(d[1], 0);
        assert_eq!(d[2], 0);
        let mut e = [0u32; 2];
        w26a09_decrypt_block(&t, &mut e, 8, 0x1234);
        assert_eq!(e, [d[0], d[1]]);
    }

    #[test]
    fn retry_loop() {
        let mut n = 0;
        assert_eq!(
            w26a09_alloc_retry(
                || {
                    n += 1;
                    if n == 3 { 9 } else { 0 }
                },
                || true
            ),
            9
        );
        assert_eq!(w26a09_alloc_retry(|| 0, || false), 0);
    }

    #[test]
    fn bounded_format() {
        let mut b = [b'x'; 8];
        assert_eq!(w26a09_bounded_format_result(&mut b, 0, 3), 0);
        assert_eq!(w26a09_bounded_format_result(&mut b, 4, 10), 3);
        assert_eq!(b[3], 0);
        assert_eq!(w26a09_bounded_format_result(&mut b, 0x7fffffff, 10), 10);
        assert_eq!(w26a09_bounded_format_result(&mut b, 4, -1), 3);
    }

    #[test]
    fn cached_read() {
        let mut c = W26a09ReadCache {
            cached_off: 0,
            cached_val: 5,
            size: 100,
            flags: 0,
        };
        let mut calls = 0;
        assert_eq!(w26a09_cached_read(&mut c, 0, 64, |_, _| 1), 5);
        let r = w26a09_cached_read(&mut c, 90, 64, |o, l| {
            calls += 1;
            assert_eq!((o, l), (90, 10));
            l
        });
        assert_eq!((r, calls), (10, 1));
        assert_eq!(w26a09_cached_read(&mut c, 90, 64, |_, _| 0), 10);
        c.flags = 0x100;
        w26a09_cached_read(&mut c, 95, 64, |_, l| {
            assert_eq!(l, 64);
            0
        });
    }

    #[test]
    fn read_checked() {
        assert_eq!(
            w26a09_read_checked(0xffff_fff0, 0, 0x20, |_, _, c| Some(c)),
            Some((true, 0x20, 0x1_0000_0010))
        );
        assert_eq!(
            w26a09_read_checked(0, 0, 8, |_, _, _| Some(4)),
            Some((false, 4, 4))
        );
        assert_eq!(w26a09_read_checked(0, 0, 8, |_, _, _| None), None);
    }
}
