#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave18/agent01.rs
// ===========================================================================
// Wave 18 translation batch 01.  Pure reference helpers for the client
// string/hash-table glue and a few small tables.

/// `0x00539640`: equality of two (ptr,len) byte strings.  Compares the common
/// prefix (`min(len)` bytes) and, when equal, the lengths.
pub fn w18a01_string_eq(a: &[u8], b: &[u8]) -> bool {
    let n = a.len().min(b.len());
    a[..n] == b[..n] && a.len() == b.len()
}

/// Result of `0x00538950` (`resize(n)` on a `{ptr,len,cap|flag}` string/vector).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W18a01ResizePlan {
    /// `new_len == len`: nothing happens.
    Noop,
    /// `new_len < len`: the tail is erased (memmove of the remaining tail).
    Shrink,
    /// Capacity is enough: zero-fill `[len, new_len)`.
    GrowInPlace,
    /// Reallocate with this new capacity (low 31 bits), then zero-fill.
    GrowRealloc(u32),
}

/// `0x00538950`: plans a resize.  `cap_field` is the third dword; bit 31 is
/// a flag and the low 31 bits are the capacity.  New capacity on overflow is
/// `max(new_len, cap + cap/2)` (the `>> 1 & 0x3fffffff` term).
pub fn w18a01_resize_plan(len: u32, cap_field: u32, new_len: u32) -> W18a01ResizePlan {
    if new_len < len {
        W18a01ResizePlan::Shrink
    } else if len < new_len {
        let cap = cap_field & 0x7fff_ffff;
        if cap < new_len {
            let grown = ((cap_field >> 1) & 0x3fff_ffff).wrapping_add(cap);
            W18a01ResizePlan::GrowRealloc(grown.max(new_len))
        } else {
            W18a01ResizePlan::GrowInPlace
        }
    } else {
        W18a01ResizePlan::Noop
    }
}

/// Bucket count target shared by `0x0053cdd0` and `0x0053e930`.
/// `size` is the element count, `needed` the callee result (`FUN_00c86e10`,
/// unresolved: supplied by the caller), `load_factor` the f32 at offset 12,
/// `hint` the requested minimum.  Starts at 8 and doubles until `>= want`.
pub fn w18a01_bucket_target(size: u32, needed: u32, load_factor: f32, hint: u32) -> u32 {
    let lhs = (size as f64) as f32;
    let rhs = (needed as f64) as f32 * load_factor;
    let mut want = needed.wrapping_add(1);
    if lhs <= rhs {
        want = needed;
    }
    if want < hint {
        want = hint;
    }
    let mut b: u32 = 8;
    if 8 < want {
        while b < want {
            b = b.wrapping_mul(2);
        }
    }
    b
}

/// `0x0053cdd0`: true when the table already has the right bucket count and a
/// bucket array (no rehash); false means the rehash path is taken.
pub fn w18a01_hash_reserve_noop_a(
    cur_buckets: u32,
    has_table: bool,
    size: u32,
    needed: u32,
    load_factor: f32,
    hint: u32,
) -> bool {
    w18a01_bucket_target(size, needed, load_factor, hint) == cur_buckets && has_table
}

/// `0x0053e930`: identical body to the one above (second template instance).
pub fn w18a01_hash_reserve_noop_b(
    cur_buckets: u32,
    has_table: bool,
    size: u32,
    needed: u32,
    load_factor: f32,
    hint: u32,
) -> bool {
    w18a01_hash_reserve_noop_a(cur_buckets, has_table, size, needed, load_factor, hint)
}

/// Move-assign of a SSO-flagged string, `0x0053cd10`.  `dst`/`src` are
/// `[ptr, len, cap|flag]`; bit 31 of cap set means "inline/borrowed" storage.
/// Returns the copied bytes when the source is inline (copy into dst
/// storage), or `None` after stealing a heap buffer (src zeroed, flag kept).
pub fn w18a01_string_move(
    dst: &mut [u32; 3],
    src: &mut [u32; 3],
    src_bytes: &[u8],
) -> Option<Vec<u8>> {
    if (src[2] as i32) >= 0 {
        dst[0] = src[0];
        dst[1] = src[1];
        dst[2] = src[2] & 0x7fff_ffff;
        src[2] &= 0x8000_0000;
        src[0] = 0;
        src[1] = 0;
        return None;
    }
    dst[1] = 0;
    let n = src[1];
    if n < (dst[2] & 0x7fff_ffff) && (dst[2] as i32) < 0 {
        dst[1] = n;
        return Some(src_bytes[..n as usize].to_vec());
    }
    None
}

/// `0x00695a70`: per-slot table select.  `flags` is the byte at
/// `DAT[slot]`; bit 0, 1, 2 choose one of three dword
/// tables (`0x30_b4`, `0x31_44`, `0x31_d4` into the game struct).  `tables`
/// holds the three arrays, indexed by `idx`.
pub fn w18a01_slot_table_lookup(flags: u8, idx: u8, tables: &[&[u32]; 3]) -> u32 {
    for (bit, t) in tables.iter().enumerate() {
        if flags & (1 << bit) != 0 {
            return t[idx as usize];
        }
    }
    0
}

/// Byte offsets of the three tables used by `0x00695a70`.
pub const W18A01_TABLE_OFFSETS: [u32; 3] = [0x30_b4, 0x31_44, 0x31_d4];

/// `0x00c671d0`: MD5 init + padding block.  Returns the initial state and
/// the 56-byte block `0x80, 0...` that is passed to the compress helper.
pub fn w18a01_md5_init() -> ([u32; 4], [u8; 56]) {
    let mut blk = [0u8; 56];
    blk[0] = 0x80;
    ([0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476], blk)
}

#[cfg(test)]
mod tests_wave18_agent01 {
    use super::*;

    #[test]
    fn eq() {
        assert!(w18a01_string_eq(b"abcde", b"abcde"));
        assert!(!w18a01_string_eq(b"abcde", b"abcdf"));
        assert!(!w18a01_string_eq(b"abc", b"abcd"));
        assert!(w18a01_string_eq(b"", b""));
    }

    #[test]
    fn resize() {
        assert_eq!(w18a01_resize_plan(5, 16, 3), W18a01ResizePlan::Shrink);
        assert_eq!(w18a01_resize_plan(5, 16, 5), W18a01ResizePlan::Noop);
        assert_eq!(w18a01_resize_plan(5, 16, 16), W18a01ResizePlan::GrowInPlace);
        assert_eq!(
            w18a01_resize_plan(5, 16, 17),
            W18a01ResizePlan::GrowRealloc(24)
        );
        assert_eq!(
            w18a01_resize_plan(5, 16, 100),
            W18a01ResizePlan::GrowRealloc(100)
        );
    }

    #[test]
    fn buckets() {
        assert_eq!(w18a01_bucket_target(1, 3, 1.0, 0), 8);
        assert_eq!(w18a01_bucket_target(100, 50, 1.0, 0), 64);
        assert_eq!(w18a01_bucket_target(1, 3, 1.0, 20), 32);
        assert!(w18a01_hash_reserve_noop_a(8, true, 1, 3, 1.0, 0));
        assert!(!w18a01_hash_reserve_noop_b(8, false, 1, 3, 1.0, 0));
        assert!(!w18a01_hash_reserve_noop_b(16, true, 1, 3, 1.0, 0));
    }

    #[test]
    fn mv() {
        let mut d = [9, 9, 0];
        let mut s = [0x1000, 4, 8];
        assert!(w18a01_string_move(&mut d, &mut s, b"").is_none());
        assert_eq!(d, [0x1000, 4, 8]);
        assert_eq!(s, [0, 0, 0]);
        let mut d = [1, 7, 0x8000_0010];
        let mut s = [0, 3, 0x8000_0000];
        assert_eq!(
            w18a01_string_move(&mut d, &mut s, b"xyz"),
            Some(b"xyz".to_vec())
        );
        assert_eq!(d[1], 3);
    }

    #[test]
    fn lookup_md5() {
        let a = [1, 2, 3];
        let b = [4, 5, 6];
        let c = [7, 8, 9];
        let t = [&a[..], &b[..], &c[..]];
        assert_eq!(w18a01_slot_table_lookup(1, 2, &t), 3);
        assert_eq!(w18a01_slot_table_lookup(6, 1, &t), 5);
        assert_eq!(w18a01_slot_table_lookup(4, 0, &t), 7);
        assert_eq!(w18a01_slot_table_lookup(0, 0, &t), 0);
        let (s, b) = w18a01_md5_init();
        assert_eq!(s[0], 0x6745_2301);
        assert_eq!(b[0], 0x80);
        assert_eq!(b[55], 0);
    }
}
